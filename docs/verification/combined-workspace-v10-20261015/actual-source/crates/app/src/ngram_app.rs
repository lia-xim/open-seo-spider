//! Saved-report adapter. Complete saved-row aggregation is independent of the
//! older explicitly selected128-source content/similarity workflow.
use anyhow::{Context as _, Result, ensure};
use open_seo_spider_core::{
    ContentCaptureSourceKind, ContentCaptureStatus, CrawlReport, NgramCorpusDocument,
    NgramCorpusOptions, NgramCorpusReport, TaskOwner, aggregate_saved_ngrams,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Write},
    path::PathBuf,
};
use tokio::io::AsyncWriteExt;
pub(crate) const REQUEST_BYTES: usize = 64 * 1024;
pub(crate) const REPORT_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const JSON_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const CSV_BYTES: usize = 64 * 1024 * 1024;
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum View {
    #[default]
    Totals,
    Occurrences,
    Sources,
}
fn default_source() -> ContentCaptureSourceKind {
    ContentCaptureSourceKind::RawHttpHtml
}
fn default_limit() -> usize {
    100
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Request {
    pub source_kind: ContentCaptureSourceKind,
    pub options: NgramCorpusOptions,
    pub view: View,
    pub n: Option<usize>,
    pub query: String,
    pub exact_tokens: Option<Vec<String>>,
    pub limit: usize,
    pub after: Option<usize>,
    pub expected_snapshot_token: Option<String>,
    pub expected_generation: Option<u64>,
}
impl Default for Request {
    fn default() -> Self {
        Self {
            source_kind: default_source(),
            options: Default::default(),
            view: Default::default(),
            n: None,
            query: String::new(),
            exact_tokens: None,
            limit: default_limit(),
            after: None,
            expected_snapshot_token: None,
            expected_generation: None,
        }
    }
}
impl Request {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            (1..=200).contains(&self.limit),
            "Ngram page limit must be1..=200"
        );
        self.options.validate()?;
        ensure!(
            self.n.is_none_or(|n| (1..=self.options.max_n).contains(&n)),
            "Selected ngram size must be1..=options.max_n (at most6)"
        );
        ensure!(self.query.len() <= 256, "Ngram query exceeds256 UTF8 bytes");
        ensure!(
            self.exact_tokens.as_ref().is_none_or(|t| !t.is_empty()
                && t.len() <= self.options.max_n
                && t.iter().all(|s| s.len() <= 4096)),
            "Invalid exact token selector"
        );
        ensure!(
            self.expected_snapshot_token
                .as_ref()
                .is_none_or(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())),
            "Invalid snapshot token"
        );
        ensure!(
            self.after.is_none() || self.expected_snapshot_token.is_some(),
            "Continuation requires the preview snapshot token"
        );
        self.options.limits.validate()?;
        Ok(())
    }
}
struct HashBudget {
    hash: Sha256,
    bytes: usize,
    cap: usize,
}
impl Write for HashBudget {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(b.len())
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| io::Error::other("Saved ngram serialization budget exceeded"))?;
        self.hash.update(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(crate) fn bounded_hash(value: &impl Serialize, cap: usize) -> Result<String> {
    let mut sink = HashBudget {
        hash: Sha256::new(),
        bytes: 0,
        cap,
    };
    serde_json::to_writer(&mut sink, value)?;
    Ok(format!("{:x}", sink.hash.finalize()))
}
pub(crate) fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn preclone(report: &CrawlReport) -> Result<()> {
    ensure!(
        report.pages.len() <= 50_000,
        "Saved report page limit exceeded"
    );
    bounded_hash(report, REPORT_BYTES)?;
    Ok(())
}
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Context {
    pub job_id: Option<String>,
    pub generation: Option<u64>,
    pub origin: String,
    pub typed_report_sha256: String,
    pub report_sha_profile: &'static str,
    pub sql_attested: bool,
    pub report_generation_attested: bool,
}
pub(crate) fn context(
    report: &CrawlReport,
    job_id: Option<String>,
    generation: Option<u64>,
    origin: &str,
) -> Result<Context> {
    Ok(Context {
        job_id,
        generation,
        origin: origin.into(),
        typed_report_sha256: bounded_hash(report, REPORT_BYTES)?,
        report_sha_profile: "typed-crawl-report-serde-json-v1",
        sql_attested: false,
        report_generation_attested: false,
    })
}
#[derive(Clone, Serialize)]
pub(crate) struct Snapshot {
    pub caller: Context,
    pub source_kind: ContentCaptureSourceKind,
    pub options_sha256: String,
    pub aggregate_sha256: String,
    pub snapshot_token: String,
}
pub(crate) fn aggregate(report: &CrawlReport, request: &Request) -> Result<NgramCorpusReport> {
    request.validate()?;
    ensure!(
        report.pages.len() <= request.options.limits.max_documents,
        "Saved document count exceeds requested limit"
    );
    let mut docs = Vec::with_capacity(report.pages.len());
    for page in &report.pages {
        let (basis, capture) =
            crate::content_cohort::captured_source(report, page, request.source_kind)?;
        let result = capture.and_then(|c| c.result.as_ref());
        docs.push(NgramCorpusDocument {
            requested_url: &page.url,
            source_revision: capture.map(|c| c.source_revision.as_str()),
            capture_config_sha256: capture.map(|c| c.config_sha256.as_str()),
            text_sha256: result.and_then(|r| r.text_sha256.as_deref()),
            text: result.and_then(|r| r.text.as_deref()),
            complete: result.is_some_and(|r| {
                matches!(
                    r.status,
                    ContentCaptureStatus::Complete | ContentCaptureStatus::Empty
                )
            }),
            basis,
            observed_generation: capture.and_then(|c| c.observed_generation),
        });
    }
    aggregate_saved_ngrams(&docs, &request.options)
}
fn snapshot(context: Context, request: &Request, report: &NgramCorpusReport) -> Result<Snapshot> {
    ensure!(
        request
            .expected_generation
            .is_none_or(|g| context.generation == Some(g)),
        "Saved generation changed or is unknown"
    );
    let options_sha256 = bounded_hash(
        &json!({"source_kind":request.source_kind,"options":request.options,"view":request.view,"n":request.n,"query":request.query,"exact_tokens":request.exact_tokens,"limit":request.limit}),
        REQUEST_BYTES,
    )?;
    let aggregate_sha256 = bounded_hash(report, CSV_BYTES)?;
    let snapshot_token = bounded_hash(
        &(&context, &options_sha256, &aggregate_sha256),
        REQUEST_BYTES,
    )?;
    ensure!(
        request
            .expected_snapshot_token
            .as_ref()
            .is_none_or(|t| t == &snapshot_token),
        "Stale saved ngram snapshot/options/filter/limit"
    );
    Ok(Snapshot {
        caller: context,
        source_kind: request.source_kind,
        options_sha256,
        aggregate_sha256,
        snapshot_token,
    })
}
fn includes(row: &open_seo_spider_core::NgramCorpusRow, request: &Request) -> bool {
    request.n.is_none_or(|n| row.n == n)
        && request
            .exact_tokens
            .as_ref()
            .is_none_or(|t| t == &row.tokens)
        && row
            .tokens
            .join(" ")
            .to_lowercase()
            .contains(&request.query.to_lowercase())
}
fn visit(
    report: &NgramCorpusReport,
    request: &Request,
    mut f: impl FnMut(Value) -> Result<()>,
) -> Result<usize> {
    let mut count = 0usize;
    match request.view {
        View::Sources => {
            for source in &report.sources {
                f(json!({"view":"sources","source":source}))?;
                count += 1;
            }
        }
        View::Totals | View::Occurrences => {
            for row in &report.rows {
                if !includes(row, request) {
                    continue;
                }
                if request.view == View::Totals {
                    f(
                        json!({"view":"totals","n":row.n,"tokens":row.tokens,"occurrences":row.occurrences,"document_frequency":row.document_frequency,"source_memberships_complete":true}),
                    )?;
                    count += 1;
                } else {
                    for membership in &row.sources {
                        let source = report
                            .sources
                            .get(membership.source_ordinal)
                            .context("Unknown aggregate source ordinal")?;
                        f(
                            json!({"view":"occurrences","n":row.n,"tokens":row.tokens,"occurrences":membership.occurrences,"source":source,"first_positions":membership.first_positions,"positions_sampled":membership.positions_sampled}),
                        )?;
                        count += 1;
                    }
                }
            }
        }
    }
    Ok(count)
}
struct Bytes {
    data: Vec<u8>,
    cap: usize,
}
impl Write for Bytes {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.data
            .len()
            .checked_add(b.len())
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| io::Error::other("Saved ngram output exceeds byte budget"))?;
        self.data.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn bytes(value: &impl Serialize, cap: usize) -> Result<Vec<u8>> {
    let mut sink = Bytes { data: vec![], cap };
    serde_json::to_writer(&mut sink, value)?;
    Ok(sink.data)
}
pub(crate) fn preview(
    report: &NgramCorpusReport,
    context: Context,
    request: &Request,
) -> Result<(Vec<u8>, Snapshot)> {
    let snapshot = snapshot(context, request, report)?;
    let after = request.after.unwrap_or(0);
    let mut index = 0usize;
    let mut rows = vec![];
    let total = visit(report, request, |row| {
        if index >= after && rows.len() < request.limit {
            rows.push(row);
        }
        index += 1;
        Ok(())
    })?;
    ensure!(after <= total, "Ngram continuation exceeds filtered rows");
    let end = after + rows.len();
    let value = json!({"engine":report.engine,"scope":"all_saved_requested_page_rows_selected_text","snapshot":snapshot,"source_kind":request.source_kind,"view":request.view,"rows":rows,"total_rows":total,"after":after,"next_after":(end<total).then_some(end),"has_more":end<total,"saved_documents":report.sources.len(),"source_states":report.source_states,"all_supplied_sources_complete":report.all_supplied_sources_complete,"candidate_windows":report.candidate_windows,"excluded_stopword_windows":report.excluded_stopword_windows,"included_windows":report.included_windows,"extended_windows":report.extended_windows,"max_n":report.options.max_n,"total_tokens":report.total_tokens,"total_text_bytes":report.total_text_bytes,"token_profile":report.token_profile,"lowercase_unicode_version":report.lowercase_unicode_version,"position_samples_per_source":request.options.limits.position_samples,"limitations":report.limitations,"website_requests":0,"whole_website_coverage_attested":false});
    Ok((bytes(&value, JSON_BYTES)?, snapshot))
}
fn csv_row(out: &mut impl Write, cells: &[String]) -> Result<()> {
    for (i, value) in cells.iter().enumerate() {
        if i > 0 {
            out.write_all(b",")?;
        }
        out.write_all(b"\"")?;
        if value
            .trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
            .chars()
            .next()
            .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@'))
        {
            out.write_all(b"'")?;
        }
        out.write_all(value.replace('"', "\"\"").as_bytes())?;
        out.write_all(b"\"")?;
    }
    out.write_all(b"\r\n")?;
    Ok(())
}
pub(crate) fn csv(
    report: &NgramCorpusReport,
    context: Context,
    request: &Request,
) -> Result<(Vec<u8>, Snapshot)> {
    ensure!(
        request.expected_snapshot_token.is_some() && request.after.is_none(),
        "Complete CSV requires the preview snapshot token and no continuation"
    );
    let snapshot = snapshot(context, request, report)?;
    let mut out = Bytes {
        data: vec![],
        cap: CSV_BYTES,
    };
    csv_row(
        &mut out,
        &[
            "snapshot_token",
            "view",
            "n",
            "tokens_json",
            "occurrences",
            "document_frequency",
            "source_ordinal",
            "requested_url",
            "source_revision",
            "capture_config_sha256",
            "text_sha256",
            "basis",
            "observed_generation",
            "source_state",
            "source_counted",
            "first_positions_json",
            "positions_sampled",
            "website_requests",
        ]
        .map(str::to_owned),
    )?;
    visit(report, request, |row| {
        let source = &row["source"];
        let cell = |v: &Value| match v {
            Value::Null => String::new(),
            Value::String(v) => v.clone(),
            v => v.to_string(),
        };
        csv_row(
            &mut out,
            &[
                snapshot.snapshot_token.clone(),
                cell(&row["view"]),
                cell(&row["n"]),
                row["tokens"].to_string(),
                cell(&row["occurrences"]),
                cell(&row["document_frequency"]),
                cell(&source["ordinal"]),
                cell(&source["requested_url"]),
                cell(&source["source_revision"]),
                cell(&source["capture_config_sha256"]),
                cell(&source["text_sha256"]),
                cell(&source["basis"]),
                cell(&source["observed_generation"]),
                cell(&source["state"]),
                cell(&source["counted"]),
                row["first_positions"].to_string(),
                cell(&row["positions_sampled"]),
                "0".into(),
            ],
        )
    })?;
    Ok((out.data, snapshot))
}
pub(crate) async fn run_cli(
    report_path: PathBuf,
    options_path: Option<PathBuf>,
    output: PathBuf,
    csv_output: Option<PathBuf>,
) -> Result<()> {
    let source = tokio::fs::canonicalize(&report_path).await?;
    let options_source = if let Some(p) = &options_path {
        Some(tokio::fs::canonicalize(p).await?)
    } else {
        None
    };
    let mut seen = std::collections::HashSet::new();
    for p in std::iter::once(&output).chain(csv_output.iter()) {
        let dest = crate::canonical_destination(p).await?;
        let identity = crate::destination_identity(&dest);
        ensure!(
            identity != crate::destination_identity(&source)
                && options_source
                    .as_ref()
                    .is_none_or(|p| identity != crate::destination_identity(p)),
            "Output cannot overwrite report/options inputs"
        );
        ensure!(
            seen.insert(identity) && !tokio::fs::try_exists(p).await?,
            "Every output must be a distinct new file"
        );
    }
    let raw = crate::read_bounded(&report_path, REPORT_BYTES).await?;
    let request: Request = if let Some(p) = options_path {
        serde_json::from_slice(&crate::read_bounded(&p, REQUEST_BYTES).await?)?
    } else {
        Default::default()
    };
    ensure!(
        csv_output.is_none() || request.after.is_none(),
        "Full CLI CSV cannot use a continuation"
    );
    let full = csv_output.is_some();
    let owner = TaskOwner::new("saved-ngram-cli");
    let result = owner
        .run_blocking(move || -> Result<(Vec<u8>, Option<Vec<u8>>)> {
            let report: CrawlReport = serde_json::from_slice(&raw)?;
            preclone(&report)?;
            let context = context(&report, None, None, "explicit_cli_report_file_app_declared")?;
            let result = aggregate(&report, &request)?;
            let (json, snapshot) = preview(&result, context.clone(), &request)?;
            let csv = if full {
                let mut export = request.clone();
                export.expected_snapshot_token = Some(snapshot.snapshot_token);
                Some(csv(&result, context, &export)?.0)
            } else {
                None
            };
            Ok((json, csv))
        })
        .await;
    let drained = owner.shutdown_and_drain().await;
    let (json, csv) = result??;
    drained?;
    let mut json_file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .await?;
    let mut csv_file = if let Some(p) = csv_output {
        Some(
            tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(p)
                .await?,
        )
    } else {
        None
    };
    json_file.write_all(&json).await?;
    json_file.sync_all().await?;
    if let (Some(file), Some(bytes)) = (csv_file.as_mut(), csv) {
        file.write_all(&bytes).await?;
        file.sync_all().await?;
    }
    Ok(())
}
#[cfg(test)]
#[path = "ngram_app_tests.rs"]
mod tests;
