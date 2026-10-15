//! Offline CSV migration workflow shared by API and explicit local-file CLI.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use open_seo_spider_core::{
    ComparisonCallerBinding, CrawlReport, MigrationCsvOptions, MigrationReport, MigrationRow,
    MigrationVerdict, TaskOwner, evaluate_migration_csv,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{self, Write},
    path::{Path, PathBuf},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

pub(crate) const REQUEST_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const API_CSV_BYTES: usize = 1024 * 1024;
pub(crate) const CLI_CSV_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const REPORT_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const JSON_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const CSV_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const OPTIONS_BYTES: usize = 64 * 1024;
pub(crate) type Result<T> = std::result::Result<T, Failure>;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Failure {
    pub code: String,
    pub message: String,
    pub website_requests: usize,
}
pub(crate) fn fail(code: &str, message: impl ToString) -> Failure {
    Failure {
        code: code.into(),
        message: message.to_string().chars().take(512).collect(),
        website_requests: 0,
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub csv: String,
    pub options: MigrationCsvOptions,
    #[serde(default)]
    pub expected_generation: Option<u64>,
    #[serde(default)]
    pub expected_snapshot_token: Option<String>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub outcome: Option<MigrationVerdict>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}
fn default_limit() -> usize {
    100
}
impl Request {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.csv.len() > API_CSV_BYTES {
            return Err(fail(
                "csv_byte_budget",
                "API CSV is limited to 1MiB of decoded UTF-8; JSON request limit remains 8MiB",
            ));
        }
        self.options
            .import_limits
            .validate()
            .map_err(|e| fail("invalid_options", e))?;
        self.options
            .observation_limits
            .validate()
            .map_err(|e| fail("invalid_options", e))?;
        if self.limit == 0 || self.limit > 200 {
            return Err(fail("invalid_page_limit", "Page limit must be 1..=200"));
        }
        if self.cursor.as_ref().is_some_and(|s| s.len() > 2048)
            || self
                .expected_snapshot_token
                .as_ref()
                .is_some_and(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err(fail(
                "invalid_continuation",
                "Invalid bounded cursor or snapshot token",
            ));
        }
        digest(&self.options, OPTIONS_BYTES)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Snapshot {
    pub job_id: Option<Uuid>,
    pub app_generation: Option<u64>,
    pub status: String,
    pub typed_report_sha256: String,
    pub typed_report_bytes: usize,
    pub job_config_sha256: String,
    pub report_config_sha256: String,
    pub hash_profile: &'static str,
    pub caller_binding: ComparisonCallerBinding,
    pub sql_attested: bool,
    pub complete_website_asserted: bool,
}
impl Snapshot {
    pub(crate) fn from_report(
        report: &CrawlReport,
        id: Option<Uuid>,
        generation: Option<u64>,
        status: String,
        job_config_sha256: String,
        caller_binding: ComparisonCallerBinding,
    ) -> Result<Self> {
        let (typed_report_sha256, typed_report_bytes) = digest(report, REPORT_BYTES)?;
        Ok(Self {
            job_id: id,
            app_generation: generation,
            status,
            typed_report_sha256,
            typed_report_bytes,
            job_config_sha256,
            report_config_sha256: digest(&report.config, REPORT_BYTES)?.0,
            hash_profile: "app_migration_typed_crawl_report_compact_serde_json_v1",
            caller_binding,
            sql_attested: false,
            complete_website_asserted: false,
        })
    }
}
struct HashWriter {
    hash: Sha256,
    count: usize,
    max: usize,
}
impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let n = self
            .count
            .checked_add(bytes.len())
            .filter(|n| *n <= self.max)
            .ok_or_else(|| io::Error::other("Serialized migration byte budget exceeded"))?;
        self.hash.update(bytes);
        self.count = n;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(crate) fn digest(value: &impl Serialize, max: usize) -> Result<(String, usize)> {
    let mut writer = HashWriter {
        hash: Sha256::new(),
        count: 0,
        max,
    };
    serde_json::to_writer(&mut writer, value).map_err(|e| fail("serialized_byte_budget", e))?;
    Ok((format!("{:x}", writer.hash.finalize()), writer.count))
}
pub(crate) fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>> {
    digest(value, JSON_BYTES)?;
    serde_json::to_vec(value).map_err(|e| fail("json_serialization", e))
}

pub(crate) struct Evaluation {
    pub snapshot: Snapshot,
    pub token: String,
    pub report: MigrationReport,
}
pub(crate) fn evaluate(
    csv: &[u8],
    options: &MigrationCsvOptions,
    report: CrawlReport,
    snapshot: Snapshot,
) -> Result<Evaluation> {
    let result = evaluate_migration_csv(csv, options, &report.pages)
        .map_err(|e| fail("invalid_mapping_or_budget", e))?;
    let token = digest(
        &(
            "app_migration_binding_v1",
            &snapshot,
            &result.input_sha256,
            options,
        ),
        OPTIONS_BYTES,
    )?
    .0;
    Ok(Evaluation {
        snapshot,
        token,
        report: result,
    })
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: u8,
    token: String,
    outcome: Option<MigrationVerdict>,
    limit: usize,
    offset: usize,
}
#[derive(Serialize)]
pub(crate) struct Page<'a> {
    pub engine: &'static str,
    pub website_requests: usize,
    pub snapshot: &'a Snapshot,
    pub snapshot_token: &'a str,
    pub csv_sha256: &'a str,
    pub options: &'a MigrationCsvOptions,
    pub summary: &'a BTreeMap<MigrationVerdict, usize>,
    pub total_rows: usize,
    pub filtered_rows: usize,
    pub offset: usize,
    pub rows: Vec<&'a MigrationRow>,
    pub next_cursor: Option<String>,
    pub scope: &'static str,
}
impl Evaluation {
    pub(crate) fn check_token(&self, request: &Request, required: bool) -> Result<()> {
        if required && request.expected_snapshot_token.is_none() {
            return Err(fail(
                "snapshot_token_required",
                "Export requires the preview snapshot token",
            ));
        }
        if request
            .expected_snapshot_token
            .as_ref()
            .is_some_and(|s| s != &self.token)
        {
            return Err(fail(
                "stale_snapshot",
                "Saved report, generation, CSV or options changed; run a new preview",
            ));
        }
        Ok(())
    }
    pub(crate) fn page(&self, request: &Request) -> Result<Page<'_>> {
        self.check_token(request, false)?;
        let offset = if let Some(raw) = &request.cursor {
            let bytes = URL_SAFE_NO_PAD
                .decode(raw)
                .map_err(|e| fail("invalid_cursor", e))?;
            let c: Cursor =
                serde_json::from_slice(&bytes).map_err(|e| fail("invalid_cursor", e))?;
            if c.version != 1
                || c.token != self.token
                || c.outcome != request.outcome
                || c.limit != request.limit
            {
                return Err(fail(
                    "stale_cursor",
                    "Snapshot, CSV, options or filter/page limit differs from cursor",
                ));
            }
            c.offset
        } else {
            0
        };
        let filtered: Vec<_> = self
            .report
            .rows
            .iter()
            .filter(|r| request.outcome.is_none_or(|v| r.verdict == v))
            .collect();
        if offset > filtered.len() {
            return Err(fail(
                "invalid_cursor",
                "Cursor offset exceeds complete filtered result",
            ));
        }
        let rows = filtered
            .iter()
            .skip(offset)
            .take(request.limit)
            .copied()
            .collect::<Vec<_>>();
        let end = offset + rows.len();
        let next_cursor = if end < filtered.len() {
            Some(
                URL_SAFE_NO_PAD.encode(
                    serde_json::to_vec(&Cursor {
                        version: 1,
                        token: self.token.clone(),
                        outcome: request.outcome,
                        limit: request.limit,
                        offset: end,
                    })
                    .map_err(|e| fail("cursor_serialization", e))?,
                ),
            )
        } else {
            None
        };
        Ok(Page {
            engine: self.report.engine,
            website_requests: 0,
            snapshot: &self.snapshot,
            snapshot_token: &self.token,
            csv_sha256: &self.report.input_sha256,
            options: &self.report.options,
            summary: &self.report.summary,
            total_rows: self.report.rows.len(),
            filtered_rows: filtered.len(),
            offset,
            rows,
            next_cursor,
            scope: "complete_mapping_evaluated_against_selected_saved_observations_no_network_or_website_completeness_attestation",
        })
    }
    pub(crate) fn export_json(&self) -> Result<Vec<u8>> {
        json_bytes(
            &serde_json::json!({"snapshot":self.snapshot,"snapshot_token":self.token,"website_requests":0,"report":self.report,"scope":"complete_csv_mapping_all_outcomes_not_filtered"}),
        )
    }
    pub(crate) fn export_csv(&self) -> Result<Vec<u8>> {
        let mut out = BoundedBytes {
            bytes: Vec::new(),
            max: CSV_BYTES,
        };
        csv_record(
            &mut out,
            &[
                "source_index",
                "source_row",
                "source_line",
                "old_original",
                "expected_original",
                "old_url",
                "expected_url",
                "observed_final",
                "final_status",
                "outcome",
                "reason",
                "duplicate_of_row",
                "conflicting_expectations",
                "observation_count",
                "redirect_chain_json",
                "csv_sha256",
                "snapshot_token",
                "typed_report_sha256",
                "website_requests",
            ]
            .map(str::to_owned),
        )?;
        for row in &self.report.rows {
            let o = row.observation.as_ref();
            csv_record(
                &mut out,
                &[
                    row.source_index.to_string(),
                    row.source_row.to_string(),
                    row.source_line.map(|v| v.to_string()).unwrap_or_default(),
                    row.old_value.clone(),
                    row.expected_value.clone(),
                    row.old_url.clone().unwrap_or_default(),
                    row.expected_url.clone().unwrap_or_default(),
                    o.map(|o| o.raw_final_url.clone()).unwrap_or_default(),
                    o.and_then(|o| o.final_status)
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    serde_json::to_value(row.verdict)
                        .map_err(|e| fail("csv_serialization", e))?
                        .as_str()
                        .unwrap_or("")
                        .into(),
                    row.reason.clone(),
                    row.duplicate_of_row
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    row.conflicting_expectations.to_string(),
                    row.observation_count.to_string(),
                    serde_json::to_string(&o.map(|o| &o.redirect_chain))
                        .map_err(|e| fail("csv_serialization", e))?,
                    self.report.input_sha256.clone(),
                    self.token.clone(),
                    self.snapshot.typed_report_sha256.clone(),
                    "0".into(),
                ],
            )?;
        }
        Ok(out.bytes)
    }
}
struct BoundedBytes {
    bytes: Vec<u8>,
    max: usize,
}
impl Write for BoundedBytes {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let n = self
            .bytes
            .len()
            .checked_add(b.len())
            .filter(|n| *n <= self.max)
            .ok_or_else(|| io::Error::other("CSV output exceeds 64MiB"))?;
        self.bytes.extend_from_slice(b);
        debug_assert_eq!(n, self.bytes.len());
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn csv_record(writer: &mut impl Write, fields: &[String]) -> Result<()> {
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            writer.write_all(b",").map_err(|e| fail("csv_write", e))?;
        }
        let text = open_seo_spider_core::csv_text(field);
        writer
            .write_all(b"\"")
            .and_then(|_| writer.write_all(text.replace('"', "\"\"").as_bytes()))
            .and_then(|_| writer.write_all(b"\""))
            .map_err(|e| fail("csv_write", e))?;
    }
    writer.write_all(b"\r\n").map_err(|e| fail("csv_write", e))
}

async fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>> {
    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|e| fail("input_read", e))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(fail("input_not_regular", "Inputs must be ordinary files"));
    }
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|e| fail("input_read", e))?;
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| fail("input_read", e))?;
    if bytes.len() > max {
        return Err(fail(
            "input_byte_budget",
            format!("Input exceeds {max} bytes"),
        ));
    }
    Ok(bytes)
}
async fn destination(path: &Path) -> Result<PathBuf> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(_) => return Err(fail("output_exists", "Output must be a new file")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(fail("output_check", e)),
    };
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| fail("invalid_output", "Output needs a filename"))?;
    let path = tokio::fs::canonicalize(parent)
        .await
        .map_err(|e| fail("invalid_output", e))?
        .join(name);
    #[cfg(windows)]
    {
        Ok(PathBuf::from(
            path.as_os_str().to_string_lossy().to_lowercase(),
        ))
    }
    #[cfg(not(windows))]
    {
        Ok(path)
    }
}
pub(crate) async fn run_cli(
    mapping: PathBuf,
    report_path: PathBuf,
    options_path: PathBuf,
    output: PathBuf,
    csv_output: Option<PathBuf>,
) -> Result<()> {
    let destination_json = destination(&output).await?;
    if let Some(path) = &csv_output
        && destination(path).await? == destination_json
    {
        return Err(fail(
            "duplicate_output",
            "JSON and CSV destinations must differ",
        ));
    }
    let csv = read_bounded(&mapping, CLI_CSV_BYTES).await?;
    let options: MigrationCsvOptions =
        serde_json::from_slice(&read_bounded(&options_path, OPTIONS_BYTES).await?)
            .map_err(|e| fail("invalid_options", e))?;
    let bytes = read_bounded(&report_path, REPORT_BYTES).await?;
    let report: CrawlReport =
        serde_json::from_slice(&bytes).map_err(|e| fail("invalid_saved_report", e))?;
    drop(bytes);
    let want_csv = csv_output.is_some();
    let owner = TaskOwner::new("offline-migration-cli");
    let result = owner
        .run_blocking(move || {
            let binding = ComparisonCallerBinding {
                origin: Some("cli_declared_saved_json_file".into()),
                basis: Some("typed_local_saved_report_not_sql_attested".into()),
                active_checkpoint: Some(false),
                ..Default::default()
            };
            let snapshot = Snapshot::from_report(
                &report,
                None,
                None,
                "declared_saved_file".into(),
                digest(&report.config, REPORT_BYTES)?.0,
                binding,
            )?;
            let evaluation = evaluate(&csv, &options, report, snapshot)?;
            let json = evaluation.export_json()?;
            let csv = if want_csv {
                Some(evaluation.export_csv()?)
            } else {
                None
            };
            Ok::<_, Failure>((json, csv))
        })
        .await
        .map_err(|e| fail("migration_worker", e));
    let drained = owner
        .shutdown_and_drain()
        .await
        .map_err(|e| fail("migration_worker_drain", e));
    let (json, csv) = result??;
    drained?;
    // All parse/evaluation/serialization budgets pass before either atomic reservation.
    // Two files are not an FS transaction; IO failure can leave newly owned partial outputs.
    let mut json_file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .await
        .map_err(|e| fail("output_reservation", e))?;
    let mut csv_file = if let Some(path) = csv_output {
        Some(
            tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .await
                .map_err(|e| fail("output_reservation", e))?,
        )
    } else {
        None
    };
    json_file
        .write_all(&json)
        .await
        .map_err(|e| fail("output_write", e))?;
    json_file
        .sync_all()
        .await
        .map_err(|e| fail("output_sync", e))?;
    if let (Some(file), Some(bytes)) = (csv_file.as_mut(), csv.as_ref()) {
        file.write_all(bytes)
            .await
            .map_err(|e| fail("output_write", e))?;
        file.sync_all().await.map_err(|e| fail("output_sync", e))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "migration_app_tests.rs"]
mod tests;
