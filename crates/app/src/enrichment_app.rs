//! One owned API-only service for the API and explicit CLI. No CrawlConfig/PageRecord.
use anyhow::{Result, ensure};
use open_seo_spider_core::{
    ListImportOptions, TaskAdmission, TaskOwner,
    api_enrichment::{
        self as api, ApiCancellation, ApiCredentialProfile, ApiEnrichmentOptions,
        ApiEnrichmentPlan, ApiEnrichmentReport, ApiOutcome, PageSpeedHttp, PageSpeedTransport,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use uuid::Uuid;
const SNAPSHOT_BYTES: usize = api::API_REPORT_BYTES;
const JSON_BYTES: usize = 8 * 1024 * 1024;
const CSV_BYTES: usize = 64 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StartRequest {
    #[serde(default)]
    pub acknowledge_remote_fetch: bool,
    pub input_utf8: String,
    pub import: ListImportOptions,
    #[serde(default)]
    pub options: ApiEnrichmentOptions,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadRequest {
    #[serde(default)]
    pub expected_snapshot: Option<String>,
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default = "limit")]
    pub limit: usize,
    #[serde(default)]
    pub outcome: Option<ApiOutcome>,
}
fn limit() -> usize {
    100
}
impl Default for ReadRequest {
    fn default() -> Self {
        Self {
            expected_snapshot: None,
            after: None,
            limit: 100,
            outcome: None,
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Completion {
    version: u32,
    id: Uuid,
    generation: u64,
    plan_sha256: String,
    report_sha256: Option<String>,
    report_bytes: usize,
    status: String,
    failure: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: u32,
    id: Uuid,
    generation: u64,
    snapshot: String,
    outcome: Option<ApiOutcome>,
    offset: usize,
}
#[derive(Clone)]
pub(crate) struct Service {
    root: Arc<PathBuf>,
    active: Arc<Mutex<BTreeMap<Uuid, ApiCancellation>>>,
    owner: TaskOwner,
    admission: TaskAdmission,
    network_enabled: bool,
}
fn locked<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn ordinary(p: &Path) -> Result<()> {
    let mut p = std::path::absolute(p)?;
    loop {
        let m = fs::symlink_metadata(&p)?;
        ensure!(
            !m.file_type().is_symlink(),
            "API-only reparse paths are refused"
        );
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            ensure!(
                m.file_attributes() & 0x400 == 0,
                "API-only reparse paths are refused"
            );
        }
        let Some(parent) = p.parent() else { break };
        if parent == p {
            break;
        }
        p = parent.to_path_buf();
    }
    Ok(())
}
fn read(p: &Path, cap: usize) -> Result<Vec<u8>> {
    ordinary(p)?;
    let mut f = File::open(p)?;
    let n = f.metadata()?.len();
    ensure!(
        usize::try_from(n).ok().is_some_and(|n| n <= cap),
        "API-only file byte budget exceeded"
    );
    let mut b = Vec::new();
    Read::by_ref(&mut f)
        .take(u64::try_from(cap)?.saturating_add(1))
        .read_to_end(&mut b)?;
    ensure!(
        b.len() <= cap && u64::try_from(b.len())? == n,
        "API-only file changed or exceeded budget"
    );
    Ok(b)
}
fn create(p: &Path, b: &[u8]) -> Result<()> {
    ordinary(
        p.parent()
            .ok_or_else(|| anyhow::anyhow!("Missing parent"))?,
    )?;
    let mut f = OpenOptions::new().write(true).create_new(true).open(p)?;
    f.write_all(b)?;
    f.sync_all()?;
    Ok(())
}
fn decode<T: serde::de::DeserializeOwned>(p: &Path, cap: usize) -> Result<T> {
    Ok(serde_json::from_slice(&read(p, cap)?)?)
}
fn credential(options: &ApiEnrichmentOptions) -> Result<Option<String>> {
    match options.credential_profile {
        ApiCredentialProfile::Anonymous => Ok(None),
        ApiCredentialProfile::PageSpeedEnvironment => {
            let key = std::env::var("OPEN_SEO_PAGESPEED_API_KEY").map_err(|_| {
                anyhow::anyhow!("PageSpeed environment credential profile is unavailable")
            })?;
            ensure!(
                !key.is_empty() && key.len() <= 4096 && key.bytes().all(|b| b.is_ascii_graphic()),
                "Invalid PageSpeed credential profile value"
            );
            Ok(Some(key))
        }
    }
}
pub(crate) fn prepare(request: &StartRequest) -> Result<ApiEnrichmentPlan> {
    ensure!(
        !matches!(
            &request.import.format,
            open_seo_spider_core::ListImportFormat::Xlsx { .. }
        ),
        "JSON API accepts UTF-8 TXT, CSV or XML input; use explicit local CLI for XLSX"
    );
    api::prepare_api_enrichment(
        request.input_utf8.as_bytes(),
        &request.import,
        &request.options,
    )
}
pub(crate) fn preview(plan: &ApiEnrichmentPlan) -> serde_json::Value {
    serde_json::json!({"engine":plan.engine,"input_sha256":plan.input.input_sha256,"input_bytes":plan.input.input_bytes,"supplied_rows":plan.input.rows.len(),"accepted_occurrences":plan.input.rows.iter().filter(|r|r.normalized_url.is_some()).count(),"unique_provider_urls":plan.unique_urls.len(),"fingerprint":plan.fingerprint,"options":plan.options,"application_website_requests":0,"provider_may_fetch_submitted_site":true,"provider_notice":"Our application does not crawl supplied websites in this mode. Google PageSpeed may fetch the submitted website remotely.","credential_availability":"not_checked_until_start","execution_model":"one_retained_sequential_worker_bounded_calls_retries_and_deadline"})
}
impl Service {
    pub(crate) fn open(
        root: PathBuf,
        admission: TaskAdmission,
        network_enabled: bool,
    ) -> Result<Self> {
        ordinary(
            root.parent()
                .ok_or_else(|| anyhow::anyhow!("Missing API store parent"))?,
        )?;
        if root.exists() {
            ordinary(&root)?;
            ensure!(root.is_dir(), "API-only store must be a directory");
        }
        Ok(Self {
            root: Arc::new(root),
            active: Default::default(),
            owner: TaskOwner::with_admission("api_enrichment_operations", admission.clone()),
            admission,
            network_enabled,
        })
    }
    pub(crate) fn close(&self) {
        self.admission.close();
        for cancel in locked(&self.active).values() {
            cancel.cancel();
        }
    }
    pub(crate) async fn drain(&self) -> Result<()> {
        self.close();
        self.owner.shutdown_and_drain().await?;
        Ok(())
    }
    pub(crate) fn stats(&self) -> serde_json::Value {
        let s = self.owner.snapshot();
        serde_json::json!({"owner":s.owner,"closing":s.closing,"admitted":s.admitted,"started":s.started,"joined":s.joined,"retained":s.retained,"first_failure":s.first_failure,"active_jobs":locked(&self.active).len()})
    }
    fn directory(&self, id: Uuid) -> PathBuf {
        self.root.join(id.to_string())
    }
    pub(crate) async fn start(
        &self,
        plan: ApiEnrichmentPlan,
        input: Vec<u8>,
    ) -> Result<serde_json::Value> {
        ensure!(
            self.network_enabled,
            "API-only jobs require an explicitly loopback local API listener"
        );
        let key = credential(&plan.options)?;
        self.start_transport(plan, input, PageSpeedHttp::new()?, key)
            .await
    }
    async fn start_transport<T: PageSpeedTransport + 'static>(
        &self,
        plan: ApiEnrichmentPlan,
        input: Vec<u8>,
        transport: T,
        key: Option<String>,
    ) -> Result<serde_json::Value> {
        ensure!(
            input.len() <= api::API_INPUT_BYTES
                && input.len() == plan.input.input_bytes
                && api::api_sha(&input) == plan.input.input_sha256,
            "API-only original input bytes differ from declared import"
        );
        let actual = api::prepare_api_enrichment(&input, &plan.input.options, &plan.options)?;
        ensure!(
            api::bounded_api_json(&actual, SNAPSHOT_BYTES)?
                == api::bounded_api_json(&plan, SNAPSHOT_BYTES)?,
            "API-only plan must derive from the original input bytes"
        );
        ensure!(!self.admission.is_closed(), "API-only admission is closed");
        ensure!(
            self.owner.snapshot().first_failure.is_none(),
            "API-only owner previously failed"
        );
        self.owner.reap_completed().await;
        let mut active = locked(&self.active);
        ensure!(active.len() < 4, "Four API-only jobs are already active");
        if !self.root.exists() {
            ordinary(
                self.root
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("Missing API store parent"))?,
            )?;
            fs::create_dir(self.root.as_ref())?;
        }
        ordinary(&self.root)?;
        ensure!(self.root.is_dir(), "API-only store must be a directory");
        ensure!(
            fs::read_dir(self.root.as_ref())?.take(1001).count() < 1000,
            "API-only stored job count budget exceeded"
        );
        let id = Uuid::new_v4();
        let dir = self.directory(id);
        fs::create_dir(&dir)?;
        ordinary(&dir)?;
        let raw = api::bounded_api_json(&plan, SNAPSHOT_BYTES)?;
        create(&dir.join("input.bin"), &input)?;
        create(&dir.join("plan.json"), &raw)?;
        let plan_sha = api::api_sha(&raw);
        let cancel = ApiCancellation::default();
        active.insert(id, cancel.clone());
        drop(active);
        let service = self.clone();
        let fingerprint = plan.fingerprint.clone();
        let task = self.owner.spawn_async(async move {
            let result = api::run_api_enrichment(plan, &transport, key.as_deref(), cancel).await;
            let completion = match result {
                Ok(report) => match api::bounded_api_json(&report, SNAPSHOT_BYTES).and_then(|raw| {
                    create(&dir.join("report.json"), &raw)?;
                    Ok((raw, report.outcome))
                }) {
                    Ok((raw, status)) => Completion {
                        version: 1,
                        id,
                        generation: 1,
                        plan_sha256: plan_sha,
                        report_sha256: Some(api::api_sha(&raw)),
                        report_bytes: raw.len(),
                        status,
                        failure: None,
                    },
                    Err(_) => Completion {
                        version: 1,
                        id,
                        generation: 1,
                        plan_sha256: plan_sha,
                        report_sha256: None,
                        report_bytes: 0,
                        status: "failed".into(),
                        failure: Some("snapshot_write_failed".into()),
                    },
                },
                Err(_) => Completion {
                    version: 1,
                    id,
                    generation: 1,
                    plan_sha256: plan_sha,
                    report_sha256: None,
                    report_bytes: 0,
                    status: "failed".into(),
                    failure: Some("bounded_api_operation_failed".into()),
                },
            };
            let saved = api::bounded_api_json(&completion, 65536)
                .and_then(|raw| create(&dir.join("completion.json"), &raw));
            if saved.is_err() {
                service
                    .owner
                    .record_failure("API-only completion snapshot failed");
            }
            locked(&service.active).remove(&id);
        });
        match task {
            Ok(ticket) => drop(ticket),
            Err(e) => {
                locked(&self.active).remove(&id);
                return Err(e.into());
            }
        }
        Ok(
            serde_json::json!({"id":id,"generation":1,"status":"running","fingerprint":fingerprint,"application_website_requests":0,"provider_may_fetch_submitted_site":true}),
        )
    }
    #[cfg(test)]
    pub(crate) async fn start_mock<T: PageSpeedTransport + 'static>(
        &self,
        plan: ApiEnrichmentPlan,
        input: Vec<u8>,
        transport: T,
        key: Option<String>,
    ) -> Result<serde_json::Value> {
        self.start_transport(plan, input, transport, key).await
    }
    pub(crate) fn configuration(&self, id: Uuid) -> Result<Vec<u8>> {
        let dir = self.directory(id);
        ordinary(&dir)?;
        let plan: ApiEnrichmentPlan = decode(&dir.join("plan.json"), SNAPSHOT_BYTES)?;
        let input = read(&dir.join("input.bin"), api::API_INPUT_BYTES)?;
        let actual = api::prepare_api_enrichment(&input, &plan.input.options, &plan.options)?;
        ensure!(
            api::bounded_api_json(&actual, SNAPSHOT_BYTES)?
                == api::bounded_api_json(&plan, SNAPSHOT_BYTES)?,
            "Saved API-only input/configuration changed"
        );
        if dir.join("completion.json").exists() {
            let c: Completion = decode(&dir.join("completion.json"), 65536)?;
            ensure!(
                c.version == 1
                    && c.id == id
                    && c.generation == 1
                    && c.plan_sha256
                        == api::api_sha(&read(&dir.join("plan.json"), SNAPSHOT_BYTES)?),
                "Saved configuration differs from completion"
            );
        }
        let input_utf8 = String::from_utf8(input).map_err(|_| {
            anyhow::anyhow!(
                "Binary CLI input cannot be edited as UTF-8; the stored report remains readable"
            )
        })?;
        api::bounded_api_json(
            &StartRequest {
                acknowledge_remote_fetch: false,
                input_utf8,
                import: plan.input.options,
                options: plan.options,
            },
            SNAPSHOT_BYTES,
        )
    }
    pub(crate) fn cancel(&self, id: Uuid) -> Result<serde_json::Value> {
        ordinary(&self.directory(id))?;
        if let Some(cancel) = locked(&self.active).get(&id) {
            cancel.cancel();
            return Ok(
                serde_json::json!({"id":id,"status":"cancelling","application_website_requests":0}),
            );
        }
        self.status(id)
    }
    pub(crate) fn status(&self, id: Uuid) -> Result<serde_json::Value> {
        let dir = self.directory(id);
        ordinary(&dir)?;
        if locked(&self.active).contains_key(&id) {
            ensure!(
                self.owner.snapshot().first_failure.is_none(),
                "API-only owner failed"
            );
            return Ok(
                serde_json::json!({"id":id,"generation":1,"status":"running","application_website_requests":0}),
            );
        }
        if !dir.join("completion.json").exists() {
            return Ok(
                serde_json::json!({"id":id,"generation":1,"status":"interrupted","automatic_provider_resume":false,"application_website_requests":0}),
            );
        }
        let c: Completion = decode(&dir.join("completion.json"), 65536)?;
        ensure!(
            c.version == 1 && c.id == id && c.generation == 1,
            "Invalid API-only completion identity"
        );
        let plan = read(&dir.join("plan.json"), SNAPSHOT_BYTES)?;
        ensure!(
            api::api_sha(&plan) == c.plan_sha256,
            "API-only plan changed"
        );
        Ok(
            serde_json::json!({"id":id,"generation":1,"status":c.status,"snapshot":c.report_sha256,"report_bytes":c.report_bytes,"failure":c.failure,"application_website_requests":0,"automatic_provider_resume":false}),
        )
    }
    pub(crate) fn report(&self, id: Uuid) -> Result<(ApiEnrichmentReport, String)> {
        ensure!(
            !locked(&self.active).contains_key(&id),
            "Wait for a stable API-only snapshot"
        );
        let dir = self.directory(id);
        ordinary(&dir)?;
        let c: Completion = decode(&dir.join("completion.json"), 65536)?;
        ensure!(
            c.version == 1 && c.id == id && c.generation == 1,
            "Invalid API-only completion identity"
        );
        let hash = c
            .report_sha256
            .ok_or_else(|| anyhow::anyhow!("No completed API-only report"))?;
        let b = read(&dir.join("report.json"), SNAPSHOT_BYTES)?;
        ensure!(
            b.len() == c.report_bytes && api::api_sha(&b) == hash,
            "API-only report changed"
        );
        let plan = read(&dir.join("plan.json"), SNAPSHOT_BYTES)?;
        ensure!(
            api::api_sha(&plan) == c.plan_sha256,
            "API-only plan changed"
        );
        let report: ApiEnrichmentReport = serde_json::from_slice(&b)?;
        ensure!(
            api::bounded_api_json(&report.plan, SNAPSHOT_BYTES)? == plan,
            "API-only report plan differs from declared input snapshot"
        );
        api::validate_api_report(&report)?;
        Ok((report, hash))
    }
    pub(crate) fn page(&self, id: Uuid, request: &ReadRequest) -> Result<Vec<u8>> {
        ensure!(
            (1..=200).contains(&request.limit),
            "API-only page limit must be1..=200"
        );
        let (report, snapshot) = self.report(id)?;
        if let Some(expected) = &request.expected_snapshot {
            ensure!(expected == &snapshot, "Stale API-only snapshot");
        }
        let offset = if let Some(after) = &request.after {
            ensure!(
                request.expected_snapshot.is_some() && after.len() <= 2048,
                "Continuation requires an exact snapshot"
            );
            let bytes =
                base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, after)?;
            let c: Cursor = serde_json::from_slice(&bytes)?;
            ensure!(
                c.version == 1
                    && c.id == id
                    && c.generation == 1
                    && c.snapshot == snapshot
                    && c.outcome == request.outcome,
                "Stale or mismatched API-only cursor"
            );
            c.offset
        } else {
            0
        };
        let by = report
            .observations
            .iter()
            .map(|r| (&r.requested_url, r))
            .collect::<BTreeMap<_, _>>();
        let selected = report
            .plan
            .input
            .rows
            .iter()
            .filter(|row| {
                request.outcome.is_none_or(|wanted| {
                    row.normalized_url
                        .as_ref()
                        .and_then(|u| by.get(u))
                        .is_some_and(|r| r.outcome == wanted)
                })
            })
            .collect::<Vec<_>>();
        ensure!(
            offset <= selected.len(),
            "API-only cursor exceeds selected rows"
        );
        let end = offset.saturating_add(request.limit).min(selected.len());
        let rows=selected[offset..end].iter().map(|row|serde_json::json!({"input":row,"provider_observation":row.normalized_url.as_ref().and_then(|u|by.get(u))})).collect::<Vec<_>>();
        let after = if end < selected.len() {
            Some(base64::Engine::encode(
                &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                api::bounded_api_json(
                    &Cursor {
                        version: 1,
                        id,
                        generation: 1,
                        snapshot: snapshot.clone(),
                        outcome: request.outcome,
                        offset: end,
                    },
                    1024,
                )?,
            ))
        } else {
            None
        };
        api::bounded_api_json(
            &serde_json::json!({"id":id,"generation":1,"snapshot":snapshot,"status":report.outcome,"rows":rows,"total":selected.len(),"after":after,"provider_request_attempts":report.provider_request_attempts,"application_website_requests":0,"provider_may_fetch_submitted_site":true}),
            JSON_BYTES,
        )
    }
    pub(crate) fn csv(&self, id: Uuid, request: &ReadRequest) -> Result<Vec<u8>> {
        ensure!(
            request.after.is_none() && request.outcome.is_none(),
            "Full API-only CSV does not accept pagination or row filters"
        );
        let (report, hash) = self.report(id)?;
        ensure!(
            request.expected_snapshot.as_deref() == Some(hash.as_str()),
            "CSV requires the exact current snapshot"
        );
        let mut out = Limited {
            bytes: Vec::new(),
            cap: CSV_BYTES,
        };
        api::export_api_enrichment_csv(&report, &mut out)?;
        Ok(out.bytes)
    }
}
struct Limited {
    bytes: Vec<u8>,
    cap: usize,
}
impl Write for Limited {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(b.len())
            .is_none_or(|n| n > self.cap)
        {
            return Err(std::io::Error::other("API-only CSV byte budget exceeded"));
        }
        self.bytes.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub(crate) async fn run_cli(
    input: PathBuf,
    options: PathBuf,
    store: PathBuf,
    output: PathBuf,
    csv: Option<PathBuf>,
) -> Result<serde_json::Value> {
    run_cli_inner(
        input,
        options,
        store,
        output,
        csv,
        PageSpeedHttp::new()?,
        credential,
    )
    .await
}
async fn run_cli_inner<T: PageSpeedTransport + 'static>(
    input: PathBuf,
    options: PathBuf,
    store: PathBuf,
    output: PathBuf,
    csv: Option<PathBuf>,
    transport: T,
    resolve_key: impl FnOnce(&ApiEnrichmentOptions) -> Result<Option<String>>,
) -> Result<serde_json::Value> {
    // Reserve every output exclusively before a provider call. Failed output files are never deleted/replaced.
    ordinary(
        input
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Missing input parent"))?,
    )?;
    let bytes = read(&input, api::API_INPUT_BYTES)?;
    let request: StartRequest = decode(&options, 65536)?;
    ensure!(
        request.acknowledge_remote_fetch,
        "CLI options must acknowledge provider remote website fetch"
    );
    ensure!(
        request.input_utf8.is_empty(),
        "CLI options input_utf8 must be empty; the explicit input file supplies bytes"
    );
    let plan = api::prepare_api_enrichment(&bytes, &request.import, &request.options)?;
    for p in std::iter::once(&output).chain(csv.iter()) {
        ordinary(
            p.parent()
                .ok_or_else(|| anyhow::anyhow!("Missing output parent"))?,
        )?;
    }
    let mut output_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    let mut csv_file = csv
        .as_ref()
        .map(|p| OpenOptions::new().write(true).create_new(true).open(p))
        .transpose()?;
    let key = resolve_key(&plan.options)?;
    let service = Service::open(store, TaskAdmission::default(), true)?;
    let started = service.start_transport(plan, bytes, transport, key).await?;
    let id: Uuid = serde_json::from_value(started["id"].clone())?;
    loop {
        if service.owner.snapshot().first_failure.is_some()
            || !locked(&service.active).contains_key(&id)
        {
            break;
        }
        tokio::select! {_=tokio::signal::ctrl_c()=>{service.cancel(id)?;},_=tokio::time::sleep(std::time::Duration::from_millis(25))=>{}}
    }
    service.drain().await?;
    let (report, hash) = service.report(id)?;
    let json = api::bounded_api_json(&report, SNAPSHOT_BYTES)?;
    let csv_bytes = if csv_file.is_some() {
        Some(service.csv(
            id,
            &ReadRequest {
                expected_snapshot: Some(hash.clone()),
                ..Default::default()
            },
        )?)
    } else {
        None
    };
    output_file.write_all(&json)?;
    output_file.sync_all()?;
    if let (Some(f), Some(b)) = (&mut csv_file, csv_bytes) {
        f.write_all(&b)?;
        f.sync_all()?;
    }
    Ok(
        serde_json::json!({"id":id,"status":report.outcome,"snapshot":hash,"json_bytes":json.len(),"provider_request_attempts":report.provider_request_attempts,"application_website_requests":0,"provider_may_fetch_submitted_site":true,"owned_tasks":service.stats()}),
    )
}
#[cfg(test)]
#[path = "enrichment_app_tests.rs"]
pub(crate) mod tests;
