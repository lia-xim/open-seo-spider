//! Explicit saved-text checks. The only outbound work is an acknowledged literal-loopback
//! LanguageTool request. No website client, crawl scheduler, renderer or SQLite access.
use crate::content_cohort::{self, ReportingMode, SavedContentRequest, SnapshotContext};
use anyhow::{Result, ensure};
use futures::StreamExt;
use open_seo_spider_core::{
    ContentAnalysisConfig, ContentCaptureSourceKind, CrawlReport, spelling,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::Duration,
};
pub const REQUEST_BYTES: usize = 128 * 1024;
pub const STORE_BYTES: usize = spelling::MAX_RESULT_BYTES;
const RUN_SECONDS: u64 = 90;
const RESPONSE_TOTAL: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub urls: Vec<String>,
    #[serde(default = "content_cohort::default_source")]
    pub source_kind: ContentCaptureSourceKind,
    #[serde(default)]
    pub reporting_mode: Option<ReportingMode>,
    #[serde(default)]
    pub expected_generation: Option<u64>,
    #[serde(default)]
    pub expected_snapshot_sha256: Option<String>,
    #[serde(default)]
    pub expected_reporting_identity: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub selection: Selection,
    pub config: spelling::Config,
    pub expected_dictionary_sha256: String,
}
impl Request {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.version == 1, "Unsupported spelling request version");
        ensure!(
            !self.selection.urls.is_empty() && self.selection.urls.len() <= spelling::MAX_DOCUMENTS,
            "Select1..=32 saved URLs"
        );
        ensure!(
            valid_hash(&self.expected_dictionary_sha256),
            "Dictionary revision required"
        );
        let mut urls = BTreeSet::new();
        ensure!(
            self.selection
                .urls
                .iter()
                .all(|u| !u.is_empty() && u.len() <= 16 * 1024 && urls.insert(u)),
            "Selected URLs must be bounded, exact and unique"
        );
        ensure!(
            serde_json::to_vec(self)?.len() <= REQUEST_BYTES,
            "Spelling request exceeds128KiB"
        );
        self.config.validate().map_err(anyhow::Error::msg)
    }
}
pub fn valid_hash(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderEvidence {
    pub endpoint: String,
    pub http_status: Option<u16>,
    pub body: String,
    pub non_utf8_base64: Option<String>,
    pub body_bytes: usize,
    pub body_sha256: String,
    pub retained_limit_bytes: usize,
    pub body_complete: bool,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub url: String,
    pub source: Value,
    pub status: String,
    pub reason: Option<String>,
    pub text: String,
    pub text_sha256: String,
    pub language: Option<spelling::LanguageDecision>,
    pub provider: Option<ProviderEvidence>,
    pub checked: Option<spelling::Checked>,
    pub accepted_count: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleSummary {
    pub rule_id: String,
    pub issues: usize,
    pub affected_urls: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub engine: String,
    pub run_id: uuid::Uuid,
    pub crawl_id: Option<uuid::Uuid>,
    pub created_at: String,
    pub request: Request,
    pub config_sha256: String,
    pub dictionary_sha256: String,
    pub dictionary: spelling::Dictionary,
    pub saved_capture: Value,
    pub website_requests: u64,
    pub engine_requests: usize,
    pub offset_unit: String,
    pub binding_current: bool,
    pub binding_qualification: String,
    pub complete: bool,
    pub accepted_count: Option<usize>,
    pub rules: Vec<RuleSummary>,
    pub documents: Vec<Document>,
    pub limitations: Vec<String>,
}
pub struct Prepared {
    request: Request,
    dictionary: spelling::Dictionary,
    envelope: Value,
    documents: Vec<open_seo_spider_core::ContentTextInput>,
}
pub fn prepare(
    report: &CrawlReport,
    context: &SnapshotContext,
    request: Request,
    dictionary: spelling::Dictionary,
) -> Result<Prepared> {
    request.validate()?;
    dictionary.validate().map_err(anyhow::Error::msg)?;
    ensure!(
        spelling::hash(&dictionary).map_err(anyhow::Error::msg)?
            == request.expected_dictionary_sha256,
        "Dictionary revision changed"
    );
    let selection = &request.selection;
    let saved = SavedContentRequest {
        urls: selection.urls.clone(),
        source_kind: selection.source_kind,
        config: ContentAnalysisConfig::default(),
        indexable_only: false,
        complete_only: false,
        reporting_mode: selection.reporting_mode,
        expected_generation: selection.expected_generation,
        expected_snapshot_sha256: selection.expected_snapshot_sha256.clone(),
        expected_reporting_identity: selection.expected_reporting_identity.clone(),
    };
    let (envelope, input) = content_cohort::prepare_saved_analysis(report, context, &saved)?;
    let documents = input.map(|v| v.documents).unwrap_or_default();
    ensure!(
        documents.len() <= spelling::MAX_DOCUMENTS,
        "Saved cohort exceeds32 documents"
    );
    Ok(Prepared {
        request,
        dictionary,
        envelope,
        documents,
    })
}
/// Exact request partition: both arrays retain request order and included IDs retain
/// original selection ordinals. Visible missing/incomplete captures stay included.
fn validate_partition(selection: &Selection, saved: &Value) -> Result<()> {
    let included = saved["documents"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing included provenance"))?;
    let excluded = saved["excluded"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing excluded provenance"))?;
    ensure!(
        saved["selected_count"].as_u64() == Some(selection.urls.len() as u64)
            && saved["included_count"].as_u64() == Some(included.len() as u64)
            && included.len().checked_add(excluded.len()) == Some(selection.urls.len()),
        "Selected/included/excluded accounting differs"
    );
    ensure!(
        saved["indexable_only"] == false
            && saved["complete_only"] == false
            && saved["site_wide_coverage"] == false,
        "Spelling may not discard visible missing/incomplete observations"
    );
    let mut reporting_exclusions = false;
    let mut captured = None;
    if let Some(view) = saved.get("reporting") {
        let identity = &view["identity"];
        ensure!(
            identity["engine"] == "open-seo-spider/saved-content-reporting-v1",
            "Unexpected saved reporting contract"
        );
        let mode: ReportingMode = serde_json::from_value(identity["mode"].clone())?;
        ensure!(
            view["identity_sha256"].as_str()
                == Some(&spelling::hash(identity).map_err(anyhow::Error::msg)?),
            "Saved reporting identity digest differs"
        );
        if let Some(expected) = &selection.expected_reporting_identity {
            ensure!(
                view["identity_sha256"] == *expected,
                "Requested reporting identity differs"
            );
        }
        let profile: Option<open_seo_spider_core::DirectiveReportingConfig> = identity
            .get("directive_reporting")
            .filter(|v| !v.is_null())
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()?;
        let active = profile.as_ref().is_some_and(|v| v.active());
        ensure!(
            profile.is_none() || active,
            "Inactive saved reporting profile must remain absent"
        );
        if let Some(requested) = selection.reporting_mode {
            ensure!(mode == requested, "Requested reporting mode differs");
        } else {
            ensure!(
                active && mode == ReportingMode::Reported,
                "Auto must use active saved Reported view, otherwise legacy absence"
            );
        }
        reporting_exclusions = active && mode == ReportingMode::Reported;
        let count = view["captured_page_observations"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing captured reporting count"))?;
        let visible = view["profile_visible_page_observations"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing visible reporting count"))?;
        let hidden = view["profile_excluded_page_observations"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing excluded reporting count"))?;
        ensure!(
            visible.checked_add(hidden) == Some(count)
                && view["eligible_page_observations"].as_u64()
                    == Some(if mode == ReportingMode::Raw {
                        count
                    } else {
                        visible
                    }),
            "Saved reporting counters differ"
        );
        ensure!(
            saved["excluded_reporting_count"].as_u64() == Some(excluded.len() as u64),
            "Selected reporting exclusion count differs"
        );
        captured = Some(count);
    } else {
        ensure!(
            selection.reporting_mode.is_none() && selection.expected_reporting_identity.is_none(),
            "Requested reporting view is missing"
        );
        ensure!(
            excluded.is_empty(),
            "Legacy/raw cohort may not invent reporting exclusions"
        );
        if let Some(count) = saved.get("excluded_reporting_count") {
            ensure!(count.as_u64() == Some(0), "Legacy exclusion count differs");
        }
    }
    let mut urls = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    for (entries, is_excluded) in [(included, false), (excluded, true)] {
        let mut previous = None;
        for entry in entries {
            let row = &entry["row"];
            let url = row["requested_url"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing partition URL"))?;
            let position = selection
                .urls
                .iter()
                .position(|v| v == url)
                .ok_or_else(|| anyhow::anyhow!("Unknown URL in saved partition"))?;
            ensure!(urls.insert(url), "Duplicate URL across saved partition");
            ensure!(
                previous.is_none_or(|last| last < position),
                "Saved partition does not preserve requested order"
            );
            previous = Some(position);
            let ordinal = row["ordinal"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing raw page ordinal"))?;
            ensure!(
                ordinals.insert(ordinal) && captured.is_none_or(|count| ordinal < count),
                "Duplicate/out-of-range raw page ordinal"
            );
            ensure!(
                row["source_kind"] == json!(selection.source_kind),
                "Partition capture source differs"
            );
            if is_excluded {
                ensure!(
                    reporting_exclusions
                        && entry["reason"] == "excluded_reporting"
                        && row["status"] == "excluded_reporting"
                        && row["reason"] == "excluded_reporting"
                        && row["capture_status"].is_string(),
                    "Only actual saved reported-view exclusions are allowed"
                );
                ensure!(
                    entry.get("analysis_document_id").is_none(),
                    "Excluded row masquerades as checked analysis ID"
                );
            } else {
                ensure!(
                    entry["analysis_document_id"] == format!("selection-{position}"),
                    "Analysis ID does not retain selection ordinal"
                );
                ensure!(
                    entry.get("reason").is_none() && row["status"] != "excluded_reporting",
                    "Included row has exclusion marker"
                );
            }
        }
    }
    ensure!(
        urls.len() == selection.urls.len()
            && selection.urls.iter().all(|u| urls.contains(u.as_str())),
        "Saved partition omits selected URLs"
    );
    Ok(())
}

async fn exchange(
    client: &reqwest::Client,
    endpoint: &str,
    text: &str,
    language: &str,
    remaining_bytes: usize,
    deadline: tokio::time::Instant,
) -> ProviderEvidence {
    let mut form = reqwest::Url::parse("http://127.0.0.1/").expect("static URL");
    form.query_pairs_mut()
        .append_pair("text", text)
        .append_pair("language", language);
    let body = form.query().unwrap_or_default().to_owned();
    let retained_limit_bytes = remaining_bytes.min(spelling::MAX_RESPONSE_BYTES);
    let mut evidence = ProviderEvidence {
        endpoint: endpoint.into(),
        http_status: None,
        body: String::new(),
        non_utf8_base64: None,
        body_bytes: 0,
        body_sha256: spelling::digest(&[]),
        retained_limit_bytes,
        body_complete: false,
        error: None,
    };
    let mut bytes = Vec::new();
    let result = tokio::time::timeout_at(deadline, async {
        ensure!(
            retained_limit_bytes > 0,
            "Aggregate provider response budget exhausted before dispatch"
        );
        let response = client
            .post(endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .body(body)
            .send()
            .await?;
        evidence.http_status = Some(response.status().as_u16());
        let status = response.status();
        let json_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| {
                v.split(';')
                    .next()
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"))
            });
        let encoding = response
            .headers()
            .get(reqwest::header::CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("identity");
        ensure!(
            encoding == "identity",
            "Encoded engine responses are not supported"
        );
        let expected = response.content_length();
        ensure!(
            expected.is_none_or(|v| v <= retained_limit_bytes as u64),
            "Engine declared body exceeds remaining retained response budget"
        );
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let available = retained_limit_bytes
                .checked_sub(bytes.len())
                .ok_or_else(|| anyhow::anyhow!("Retained response accounting overflow"))?;
            let take = chunk.len().min(available);
            bytes.extend_from_slice(&chunk[..take]);
            ensure!(
                take == chunk.len(),
                "Engine stream exceeds remaining retained response budget"
            );
        }
        evidence.body_complete = true;
        std::str::from_utf8(&bytes).map_err(|_| anyhow::anyhow!("Engine reply is not UTF8"))?;
        ensure!(
            expected.is_none_or(|v| v == bytes.len() as u64),
            "Engine length mismatch"
        );
        ensure!(status.is_success(), "LanguageTool HTTP{}", status.as_u16());
        ensure!(json_type, "Engine JSON media type required");
        Ok::<(), anyhow::Error>(())
    })
    .await;
    let result = result.unwrap_or_else(|_| Err(anyhow::anyhow!("Run deadline")));
    evidence.body_bytes = bytes.len();
    evidence.body_sha256 = spelling::digest(&bytes);
    match String::from_utf8(bytes.clone()) {
        Ok(text) => evidence.body = text,
        Err(_) => {
            use base64::Engine;
            evidence.non_utf8_base64 =
                Some(base64::engine::general_purpose::STANDARD.encode(&bytes));
        }
    }
    if let Err(error) = result {
        evidence.error = Some(error.to_string());
    }
    evidence
}
pub async fn run(prepared: Prepared) -> Result<Report> {
    let Prepared {
        request,
        dictionary,
        envelope,
        documents,
    } = prepared;
    request.validate()?;
    validate_partition(&request.selection, &envelope)?;
    let provenance = envelope["documents"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing prepared provenance"))?;
    ensure!(
        documents.len() == provenance.len(),
        "Prepared documents omit included provenance"
    );
    for (input, source) in documents.iter().zip(provenance) {
        ensure!(
            source["analysis_document_id"] == input.id
                && source["row"]["sample_complete"] == input.complete
                && source["row"]["html_language"].as_str() == input.language.as_deref(),
            "Prepared source identity/completeness/language differs"
        );
        if let Some(expected) = source["row"]["text_sha256"].as_str() {
            ensure!(
                input
                    .text
                    .as_ref()
                    .is_some_and(|text| spelling::digest(text.as_bytes()) == expected),
                "Prepared text differs from saved capture"
            );
        }
    }
    let config_sha256 = spelling::hash(&request.config).map_err(anyhow::Error::msg)?;
    let dictionary_sha256 = spelling::hash(&dictionary).map_err(anyhow::Error::msg)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .pool_max_idle_per_host(0)
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(12))
        .build()?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(RUN_SECONDS);
    let mut output = Vec::new();
    let mut engine_requests = 0;
    let mut response_bytes = 0usize;
    let mut first_software = None;
    let mut mixed = false;
    for input in documents {
        let source = envelope["documents"]
            .as_array()
            .and_then(|v| {
                v.iter()
                    .find(|v| v["analysis_document_id"].as_str() == Some(&input.id))
            })
            .map(|v| v["row"].clone())
            .ok_or_else(|| anyhow::anyhow!("Missing capture provenance"))?;
        let url = source["requested_url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing capture URL"))?
            .to_owned();
        let text = input.text.unwrap_or_default();
        let mut doc = Document {
            id: input.id,
            url,
            source,
            status: "unavailable".into(),
            reason: None,
            text_sha256: spelling::digest(text.as_bytes()),
            text,
            language: None,
            provider: None,
            checked: None,
            accepted_count: None,
        };
        if !request.config.enabled {
            doc.status = "disabled".into();
            doc.reason = Some("Checking is off; no engine request".into());
        } else if !input.complete {
            doc.status = "incomplete_capture".into();
            doc.reason = Some(
                input
                    .truncation_reason
                    .unwrap_or_else(|| "Saved capture incomplete".into()),
            );
        } else if doc.text.is_empty() {
            doc.status = "empty_saved_text".into();
            doc.reason = Some("No engine request or zero-error claim for empty text".into());
        } else if doc.text.len() > spelling::MAX_TEXT_BYTES
            || doc.text.encode_utf16().count() > spelling::MAX_TEXT_UTF16
        {
            doc.status = "text_bound".into();
            doc.reason = Some(
                "Whole saved text exceeds finite bound; text was not truncated or checked".into(),
            );
        } else if response_bytes >= RESPONSE_TOTAL || tokio::time::Instant::now() >= deadline {
            doc.status = "not_run_bound".into();
            doc.reason = Some("Run time/response budget exhausted".into());
        } else {
            match spelling::choose_language(&request.config, input.language.as_deref()) {
                Err(error) => {
                    doc.status = "language_unavailable".into();
                    doc.reason = Some(error);
                }
                Ok(language) => {
                    let endpoint = request
                        .config
                        .endpoint
                        .as_deref()
                        .ok_or_else(|| anyhow::anyhow!("Missing endpoint"))?;
                    engine_requests += 1;
                    let remaining = RESPONSE_TOTAL
                        .checked_sub(response_bytes)
                        .ok_or_else(|| anyhow::anyhow!("Aggregate response accounting overflow"))?;
                    let evidence = exchange(
                        &client,
                        endpoint,
                        &doc.text,
                        &language.requested,
                        remaining,
                        deadline,
                    )
                    .await;
                    response_bytes = response_bytes
                        .checked_add(evidence.body_bytes)
                        .ok_or_else(|| anyhow::anyhow!("Aggregate response accounting overflow"))?;
                    ensure!(
                        response_bytes <= RESPONSE_TOTAL,
                        "Exchange exceeded aggregate retained response budget"
                    );
                    let checked = if let Some(error) = &evidence.error {
                        Err(error.clone())
                    } else {
                        spelling::parse_reply(
                            evidence.body.as_bytes(),
                            &doc.text,
                            &language,
                            &request.config,
                            &dictionary,
                        )
                    };
                    match checked {
                        Err(error) => {
                            doc.status = "provider_error".into();
                            doc.reason = Some(error);
                        }
                        Ok(checked) => {
                            if first_software
                                .as_ref()
                                .is_some_and(|v| v != &checked.software)
                            {
                                mixed = true;
                            }
                            if first_software.is_none() {
                                first_software = Some(checked.software.clone());
                            }
                            doc.status = if checked.complete {
                                "complete"
                            } else {
                                "provider_incomplete"
                            }
                            .into();
                            doc.accepted_count = checked.accepted_count;
                            doc.checked = Some(checked);
                        }
                    }
                    doc.language = Some(language);
                    doc.provider = Some(evidence);
                }
            }
        }
        output.push(doc);
    }
    let complete = !output.is_empty() && !mixed && output.iter().all(|v| v.status == "complete");
    let accepted_count = complete.then(|| output.iter().filter_map(|v| v.accepted_count).sum());
    let mut grouped: BTreeMap<String, (usize, BTreeSet<String>)> = BTreeMap::new();
    for d in &output {
        if let Some(checked) = &d.checked {
            for issue in &checked.issues {
                if issue.disposition == "accepted" {
                    let item = grouped.entry(issue.rule_id.clone()).or_default();
                    item.0 += 1;
                    item.1.insert(d.url.clone());
                }
            }
        }
    }
    let mut rules: Vec<_> = grouped
        .into_iter()
        .map(|(rule_id, (issues, urls))| RuleSummary {
            rule_id,
            issues,
            affected_urls: urls.len(),
        })
        .collect();
    rules.sort_by(|a, b| {
        b.issues
            .cmp(&a.issues)
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });
    let report=Report {engine:spelling::ENGINE.into(),run_id:uuid::Uuid::new_v4(),crawl_id:None,created_at:chrono::Utc::now().to_rfc3339(),
        request,config_sha256,dictionary_sha256,dictionary,saved_capture:envelope,website_requests:0,engine_requests,
        offset_unit:"utf16_code_units".into(),binding_current:true,binding_qualification:"Current at analysis preparation; API revalidates before persistence. Offline imported snapshot is declared input.".into(),
        complete,accepted_count,rules,documents:output,
        limitations:vec!["Explicit saved HTML capture cohort; no website, PDF, title-only or metadata-only checks".into(),
            "Local LanguageTool basic rules; no cloud AI or automatic language detection accuracy claim".into(),
            "Non-misspelling LanguageTool issue types are retained under the grammar switch, including style/typography rules".into(),
            "Dictionary uses exact case/Unicode words and exact regional or explicit base-language scope".into(),
            if mixed {"Engine identity changed during run; aggregate count is unknown"} else {"Engine offset interpretation is UTF16; pinned engine fixture verification is a separate runtime gate"}.into()] };
    validate_report(&report, None)?;
    ensure!(
        serde_json::to_vec(&report)?.len() <= STORE_BYTES,
        "Spelling report exceeds8MiB; no partial report accepted"
    );
    Ok(report)
}
/// Recompute persisted/transport bindings, including raw provider bytes. A self-described
/// count or embedded request alone does not establish the requested projection.
pub fn validate_report(report: &Report, expected: Option<&Request>) -> Result<()> {
    ensure!(
        report.engine == spelling::ENGINE
            && report.website_requests == 0
            && report.offset_unit == "utf16_code_units",
        "Spelling result contract mismatch"
    );
    report.request.validate()?;
    report.dictionary.validate().map_err(anyhow::Error::msg)?;
    ensure!(
        report.config_sha256
            == spelling::hash(&report.request.config).map_err(anyhow::Error::msg)?
            && report.dictionary_sha256
                == spelling::hash(&report.dictionary).map_err(anyhow::Error::msg)?
            && report.dictionary_sha256 == report.request.expected_dictionary_sha256,
        "Spelling config/dictionary digest mismatch"
    );
    if let Some(expected) = expected {
        ensure!(
            serde_json::to_value(expected)? == serde_json::to_value(&report.request)?,
            "Spelling response differs from exact request"
        );
    }
    let s = &report.request.selection;
    let saved = &report.saved_capture;
    ensure!(
        saved["engine"] == content_cohort::SAVED_CONTENT_ENGINE
            && saved["snapshot"]["sha_profile"] == content_cohort::REPORT_HASH_PROFILE,
        "Saved content authority/profile mismatch"
    );
    ensure!(
        saved["requested_urls"] == json!(s.urls)
            && saved["source_kind"] == json!(s.source_kind)
            && saved["website_requests"] == 0,
        "Saved cohort differs from requested selection"
    );
    ensure!(
        saved["snapshot"]["sha256"].as_str().is_some_and(valid_hash),
        "Missing raw snapshot identity"
    );
    ensure!(
        s.expected_generation
            .is_none_or(|g| saved["snapshot"]["generation"] == g),
        "Saved generation mismatch"
    );
    ensure!(
        s.expected_snapshot_sha256
            .as_ref()
            .is_none_or(|h| saved["snapshot"]["sha256"] == *h),
        "Saved snapshot mismatch"
    );
    ensure!(
        s.expected_reporting_identity
            .as_ref()
            .is_none_or(|h| saved["reporting"]["identity_sha256"] == *h),
        "Saved reporting identity mismatch"
    );
    if let Some(mode) = s.reporting_mode {
        ensure!(
            saved["reporting"]["identity"]["mode"] == json!(mode),
            "Requested reporting mode missing or differs"
        );
    }
    validate_partition(s, saved)?;
    ensure!(
        report.documents.len() <= spelling::MAX_DOCUMENTS
            && report.engine_requests <= spelling::MAX_DOCUMENTS,
        "Spelling result document bound"
    );
    let mut ids = BTreeSet::new();
    let mut observed = 0;
    let mut total = 0;
    let mut software = None;
    let mut mixed = false;
    let mut retained_bytes = 0usize;
    let provenance = saved["documents"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing document provenance"))?;
    ensure!(
        provenance.len() == report.documents.len(),
        "Spelling document/provenance cardinality differs"
    );
    let mut grouped: BTreeMap<String, (usize, BTreeSet<String>)> = BTreeMap::new();
    for (d, provenance_row) in report.documents.iter().zip(provenance) {
        ensure!(
            ids.insert(&d.id)
                && s.urls.contains(&d.url)
                && d.source["requested_url"] == d.url
                && d.text_sha256 == spelling::digest(d.text.as_bytes()),
            "Spelling document identity/text mismatch"
        );
        ensure!(
            provenance_row["analysis_document_id"] == d.id && provenance_row["row"] == d.source,
            "Spelling ordered source provenance mismatch"
        );
        ensure!(
            d.source["source_kind"] == json!(s.source_kind),
            "Spelling document capture source differs"
        );
        if let Some(expected) = d.source["text_sha256"].as_str() {
            ensure!(
                expected == d.text_sha256,
                "Checked text differs from saved capture digest"
            );
        }
        if d.checked.is_some() {
            ensure!(
                d.source["sample_complete"] == true
                    && d.source["text_sha256"].as_str() == Some(&d.text_sha256),
                "Checked result lacks complete saved-text binding"
            );
        }
        if let Some(p) = &d.provider {
            ensure!(
                report.request.config.enabled
                    && report.request.config.acknowledge_local_text_transfer
                    && d.source["sample_complete"] == true
                    && !d.text.is_empty()
                    && d.text.len() <= spelling::MAX_TEXT_BYTES
                    && d.text.encode_utf16().count() <= spelling::MAX_TEXT_UTF16,
                "Provider attempt lacks enabled/acknowledged complete nonempty bounded saved text"
            );
            let remaining = RESPONSE_TOTAL
                .checked_sub(retained_bytes)
                .ok_or_else(|| anyhow::anyhow!("Retained response accounting overflow"))?;
            ensure!(
                remaining > 0
                    && p.retained_limit_bytes == remaining.min(spelling::MAX_RESPONSE_BYTES)
                    && p.body_bytes <= p.retained_limit_bytes,
                "Provider retained limit differs from remaining run budget"
            );
            retained_bytes = retained_bytes
                .checked_add(p.body_bytes)
                .ok_or_else(|| anyhow::anyhow!("Retained response accounting overflow"))?;
            ensure!(
                retained_bytes <= RESPONSE_TOTAL,
                "Aggregate retained response budget exceeded"
            );
            observed += 1;
            let raw = if let Some(encoded) = &p.non_utf8_base64 {
                use base64::Engine;
                ensure!(
                    p.body.is_empty() && p.error.is_some(),
                    "NonUTF8 provider body claims success"
                );
                base64::engine::general_purpose::STANDARD.decode(encoded)?
            } else {
                p.body.as_bytes().to_vec()
            };
            ensure!(
                raw.len() == p.body_bytes
                    && p.body_sha256 == spelling::digest(&raw)
                    && Some(&p.endpoint) == report.request.config.endpoint.as_ref(),
                "Spelling provider evidence mismatch"
            );
        }
        if let Some(c) = &d.checked {
            let p = d
                .provider
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Checked result has no provider"))?;
            let language = d
                .language
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Checked result has no language"))?;
            ensure!(
                p.body_complete
                    && p.error.is_none()
                    && p.http_status.is_some_and(|s| (200..300).contains(&s)),
                "Checked result has incomplete/failed HTTP evidence"
            );
            let expected_language = spelling::choose_language(
                &report.request.config,
                d.source["html_language"].as_str(),
            )
            .map_err(anyhow::Error::msg)?;
            ensure!(
                language == &expected_language,
                "Spelling language decision mismatch"
            );
            let actual = spelling::parse_reply(
                p.body.as_bytes(),
                &d.text,
                language,
                &report.request.config,
                &report.dictionary,
            )
            .map_err(anyhow::Error::msg)?;
            ensure!(
                &actual == c
                    && d.accepted_count == c.accepted_count
                    && d.status
                        == if c.complete {
                            "complete"
                        } else {
                            "provider_incomplete"
                        },
                "Spelling issue result mismatch"
            );
            if software.as_ref().is_some_and(|v| v != &c.software) {
                mixed = true;
            }
            if software.is_none() {
                software = Some(c.software.clone());
            }
            total += c.accepted_count.unwrap_or(0);
            for i in &c.issues {
                if i.disposition == "accepted" {
                    let row = grouped.entry(i.rule_id.clone()).or_default();
                    row.0 += 1;
                    row.1.insert(d.url.clone());
                }
            }
        } else {
            ensure!(
                d.accepted_count.is_none() && d.status != "complete",
                "Unchecked document claims count"
            );
        }
    }
    // Deadline cancellation can lack a response but still represents one dispatched
    // request; every attempt is represented by an explicit provider evidence row.
    ensure!(
        observed == report.engine_requests,
        "Engine request/evidence count mismatch"
    );
    let mut rules: Vec<_> = grouped
        .into_iter()
        .map(|(rule_id, (issues, urls))| RuleSummary {
            rule_id,
            issues,
            affected_urls: urls.len(),
        })
        .collect();
    rules.sort_by(|a, b| {
        b.issues
            .cmp(&a.issues)
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });
    ensure!(rules == report.rules, "Spelling rule aggregation mismatch");
    ensure!(
        !report.complete
            || (report.binding_current
                && !mixed
                && !report.documents.is_empty()
                && report.documents.iter().all(|d| d.status == "complete")
                && report.accepted_count == Some(total)),
        "Spelling complete/count claim invalid"
    );
    ensure!(
        report.complete || report.accepted_count.is_none(),
        "Incomplete result claims aggregate count"
    );
    Ok(())
}
pub fn csv(report: &Report) -> Result<String> {
    validate_report(report, None)?;
    let mut out = String::from(
        "run_id,binding_current,run_complete,run_accepted_count,url,status,reason,language,offset_utf16,length_utf16,matched_text,message,rule_id,issue_type,disposition,replacements,context,engine_version,text_sha256,snapshot_sha256,config_sha256,dictionary_sha256\r\n",
    );
    fn cell(v: &str) -> String {
        let guarded = if matches!(v.chars().next(), Some('=' | '+' | '-' | '@' | '\t' | '\r')) {
            format!("'{v}")
        } else {
            v.into()
        };
        format!("\"{}\"", guarded.replace('"', "\"\""))
    }
    for d in &report.documents {
        let issues = d
            .checked
            .as_ref()
            .map(|v| v.issues.as_slice())
            .unwrap_or(&[]);
        let rows: Vec<Option<&spelling::Issue>> = if issues.is_empty() {
            vec![None]
        } else {
            issues.iter().map(Some).collect()
        };
        for issue in rows {
            let language = d
                .language
                .as_ref()
                .map(|v| v.requested.as_str())
                .unwrap_or("");
            let version = d
                .checked
                .as_ref()
                .map(|v| v.software.version.as_str())
                .unwrap_or("");
            let mut cells = vec![
                report.run_id.to_string(),
                report.binding_current.to_string(),
                report.complete.to_string(),
                report
                    .accepted_count
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                d.url.clone(),
                d.status.clone(),
                d.reason.clone().unwrap_or_default(),
                language.into(),
            ];
            if let Some(i) = issue {
                cells.extend([
                    i.offset_utf16.to_string(),
                    i.length_utf16.to_string(),
                    i.matched_text.clone(),
                    i.message.clone(),
                    i.rule_id.clone(),
                    i.issue_type.clone(),
                    i.disposition.clone(),
                    serde_json::to_string(&i.replacements)?,
                    i.context.clone(),
                ]);
            } else {
                cells.extend(std::iter::repeat_n(String::new(), 9));
            }
            cells.extend([
                version.into(),
                d.text_sha256.clone(),
                report.saved_capture["snapshot"]["sha256"]
                    .as_str()
                    .unwrap_or("")
                    .into(),
                report.config_sha256.clone(),
                report.dictionary_sha256.clone(),
            ]);
            out.push_str(&cells.iter().map(|v| cell(v)).collect::<Vec<_>>().join(","));
            out.push_str("\r\n");
        }
    }
    ensure!(out.len() <= STORE_BYTES, "CSV exceeds8MiB");
    Ok(out)
}
// Explicit operations only. Refuse links/reparse parents and never accept a caller's store path.
pub fn ordinary(path: &Path) -> Result<()> {
    let mut current = Some(path);
    while let Some(p) = current {
        match std::fs::symlink_metadata(p) {
            Ok(m) => {
                ensure!(!m.file_type().is_symlink(), "Spelling path is a symlink");
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    ensure!(
                        m.file_attributes() & 0x400 == 0,
                        "Spelling path is a reparse point"
                    );
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        current = p.parent();
    }
    Ok(())
}
pub async fn read_json<T: serde::de::DeserializeOwned>(
    path: &Path,
    cap: usize,
) -> Result<Option<T>> {
    ordinary(path)?;
    if !tokio::fs::try_exists(path).await? {
        return Ok(None);
    }
    use tokio::io::AsyncReadExt;
    let file = tokio::fs::File::open(path).await?;
    ensure!(file.metadata().await?.is_file(), "Spelling file required");
    let mut bytes = Vec::new();
    file.take(cap as u64 + 1).read_to_end(&mut bytes).await?;
    ensure!(bytes.len() <= cap, "Spelling file exceeds bound");
    Ok(Some(serde_json::from_slice(&bytes)?))
}
pub async fn save_json<T: Serialize>(path: &Path, value: &T, replace: bool) -> Result<()> {
    ordinary(path)?;
    let bytes = serde_json::to_vec(value)?;
    ensure!(bytes.len() <= STORE_BYTES, "Spelling storage bound");
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Missing storage parent"))?;
    tokio::fs::create_dir_all(parent).await?;
    ordinary(parent)?;
    ensure!(
        replace || !tokio::fs::try_exists(path).await?,
        "Spelling result exists"
    );
    let temp: PathBuf = parent.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    use tokio::io::AsyncWriteExt;
    let mut f = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .await?;
    f.write_all(&bytes).await?;
    f.sync_all().await?;
    drop(f);
    // All application writers hold the shared spelling gate; external file edits are
    // not claimed as supported concurrent writers. Rename failure retains temp evidence.
    tokio::fs::rename(&temp, path).await?;
    Ok(())
}
pub fn dictionary_view(dictionary: &spelling::Dictionary) -> Result<Value> {
    dictionary.validate().map_err(anyhow::Error::msg)?;
    Ok(
        json!({"dictionary":dictionary,"sha256":spelling::hash(dictionary).map_err(anyhow::Error::msg)?,"scope":"application_dictionary","case_sensitive":true}),
    )
}
#[cfg(test)]
pub(crate) mod tests;
