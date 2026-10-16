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
/// Reporting is an App presentation boundary; core corpus membership indices stay dense.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReportingMode {
    Raw,
    Reported,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ReportingIdentity {
    pub engine: &'static str,
    pub mode: ReportingMode,
    pub directive_reporting: Option<open_seo_spider_core::DirectiveReportingConfig>,
}
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Reporting {
    pub identity: ReportingIdentity,
    pub identity_sha256: String,
    pub raw_page_ordinals_sha256: String,
    pub captured_page_observations: usize,
    pub profile_visible_page_observations: usize,
    pub profile_excluded_page_observations: usize,
    pub eligible_page_observations: usize,
    pub counted_source_observations: usize,
    pub uncounted_source_observations: usize,
    pub counts_mean: &'static str,
}
#[derive(Debug, Serialize)]
pub(crate) struct Aggregate {
    #[serde(flatten)]
    pub corpus: NgramCorpusReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting: Option<Reporting>,
    #[serde(skip)]
    raw_page_ordinals: Option<Vec<usize>>,
}
impl std::ops::Deref for Aggregate {
    type Target = NgramCorpusReport;
    fn deref(&self) -> &Self::Target {
        &self.corpus
    }
}
impl Aggregate {
    #[cfg(test)]
    fn legacy(corpus: NgramCorpusReport) -> Self {
        Self {
            corpus,
            reporting: None,
            raw_page_ordinals: None,
        }
    }
}
fn reporting_identity(
    profile: &Option<open_seo_spider_core::DirectiveReportingConfig>,
    requested: Option<ReportingMode>,
) -> Result<Option<ReportingIdentity>> {
    if let Some(profile) = profile {
        profile.validate()?;
    }
    let active = !open_seo_spider_core::DirectiveReportingConfig::is_inactive(profile);
    if !active && requested.is_none() {
        return Ok(None);
    }
    Ok(Some(ReportingIdentity {
        engine: "open-seo-spider/saved-ngram-reporting-v1",
        mode: requested.unwrap_or(if active {
            ReportingMode::Reported
        } else {
            ReportingMode::Raw
        }),
        directive_reporting: profile
            .clone()
            .filter(open_seo_spider_core::DirectiveReportingConfig::active),
    }))
}
fn selected_observations(
    report: &CrawlReport,
    request: &Request,
) -> Result<(Option<ReportingIdentity>, Vec<usize>, usize)> {
    // The raw serialization/page ceilings remain global, even when every row is hidden.
    preclone(report)?;
    let identity = reporting_identity(&report.config.directive_reporting, request.reporting_mode)?;
    let reported = identity
        .as_ref()
        .is_some_and(|v| v.mode == ReportingMode::Reported);
    let mut ordinals = Vec::new();
    let mut visible = 0;
    for (ordinal, page) in report.pages.iter().enumerate() {
        // Validate against the original complete report, including selected-recrawl
        // target/generation bindings. Never construct a filtered report or rewrite captures.
        if identity.is_some() || request.retained_metadata_channel.is_some() {
            open_seo_spider_core::validate_page_content_captures(page)?;
        }
        crate::content_cohort::captured_source(report, page, request.source_kind)?;
        let shown = open_seo_spider_core::directive_page_visible(&report.config, page);
        visible += usize::from(shown);
        if !reported || shown {
            ordinals.push(ordinal);
        }
    }
    ensure!(
        ordinals.len() <= request.options.limits.max_documents,
        "Saved document count exceeds requested limit"
    );
    Ok((identity, ordinals, visible))
}
fn finish_aggregate(
    corpus: NgramCorpusReport,
    report: &CrawlReport,
    identity: Option<ReportingIdentity>,
    ordinals: Vec<usize>,
    visible: usize,
) -> Result<Aggregate> {
    ensure!(
        corpus.sources.len() == ordinals.len(),
        "Ngram source observation mapping mismatch"
    );
    ensure!(
        corpus
            .sources
            .iter()
            .enumerate()
            .all(|(i, s)| s.ordinal == i && s.requested_url == report.pages[ordinals[i]].url),
        "Dense corpus source identity mismatch"
    );
    let reporting = identity.map(|identity| -> Result<Reporting> {
        let counted = corpus.sources.iter().filter(|s| s.counted).count();
        Ok(Reporting {
            identity_sha256: bounded_hash(&identity, REQUEST_BYTES)?, identity,
            raw_page_ordinals_sha256: bounded_hash(&ordinals, JSON_BYTES)?,
            captured_page_observations: report.pages.len(),
            profile_visible_page_observations: visible,
            profile_excluded_page_observations: report.pages.len() - visible,
            eligible_page_observations: ordinals.len(), counted_source_observations: counted,
            uncounted_source_observations: ordinals.len() - counted,
            counts_mean: "Captured/profile counts describe original page observations. Only eligible observations enter this corpus and its limits. Counted/uncounted are extraction states within that eligible population, not website coverage. source.ordinal and memberships remain dense corpus indices; source.raw_page_ordinal separately identifies the complete raw report observation. Multiple eligible duplicate URLs retain the existing explicit ambiguity refusal. Raw snapshot/crawl facts and capture bindings are unchanged.",
        })
    }).transpose()?;
    Ok(Aggregate {
        corpus,
        raw_page_ordinals: reporting.as_ref().map(|_| ordinals),
        reporting,
    })
}
fn source_value(
    report: &Aggregate,
    source: &open_seo_spider_core::NgramCorpusSource,
) -> Result<Value> {
    let mut value = serde_json::to_value(source)?;
    if let Some(ordinals) = &report.raw_page_ordinals {
        value["raw_page_ordinal"] = json!(
            ordinals
                .get(source.ordinal)
                .context("Missing raw source observation ordinal")?
        );
    }
    Ok(value)
}
fn csv_header(out: &mut impl Write, columns: &[String], snapshot: &Snapshot) -> Result<()> {
    let mut header = columns.to_vec();
    if snapshot.reporting.is_some() {
        header.extend(["reporting_json".into(), "raw_page_ordinal".into()]);
    }
    csv_row(out, &header)?;
    if let Some(reporting) = &snapshot.reporting {
        // A typed metadata row retains counts/profile even for a wholly hidden corpus.
        let mut row = vec![String::new(); columns.len()];
        row[0] = snapshot.snapshot_token.clone();
        row[1] = "reporting".into();
        row.extend([serde_json::to_string(reporting)?, String::new()]);
        csv_row(out, &row)?;
    }
    Ok(())
}
fn csv_record(
    out: &mut impl Write,
    columns: &[String],
    snapshot: &Snapshot,
    raw_ordinal: &Value,
) -> Result<()> {
    let mut row = columns.to_vec();
    if snapshot.reporting.is_some() {
        row.extend([
            String::new(),
            if raw_ordinal.is_null() {
                String::new()
            } else {
                raw_ordinal.to_string()
            },
        ]);
    }
    csv_row(out, &row)
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retained_metadata_channel: Option<open_seo_spider_core::NgramRetainedMetadataProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_text_channel: Option<open_seo_spider_core::NgramHtmlTextProfile>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting_mode: Option<ReportingMode>,
}
impl Default for Request {
    fn default() -> Self {
        Self {
            html_text_channel: None,
            retained_metadata_channel: None,
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
            reporting_mode: None,
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
        if let Some(profile) = self.html_text_channel {
            profile.validate()?;
        }
        if let Some(profile) = self.retained_metadata_channel {
            profile.validate()?;
        }
        ensure!(
            self.html_text_channel.is_none() || self.retained_metadata_channel.is_none(),
            "Choose exactly one retained-metadata or capture-bound HTML channel profile"
        );
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
    #[serde(skip)]
    directive_reporting: Option<open_seo_spider_core::DirectiveReportingConfig>,
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
        directive_reporting: report.config.directive_reporting.clone(),
    })
}
#[derive(Clone, Serialize)]
pub(crate) struct Snapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting: Option<Reporting>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retained_metadata_channel: Option<open_seo_spider_core::NgramRetainedMetadataProfile>,
    pub caller: Context,
    pub source_kind: ContentCaptureSourceKind,
    pub options_sha256: String,
    pub aggregate_sha256: String,
    pub snapshot_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_text_channel: Option<open_seo_spider_core::NgramHtmlTextProfile>,
}
pub(crate) fn aggregate(report: &CrawlReport, request: &Request) -> Result<Aggregate> {
    request.validate()?;
    let (identity, ordinals, visible) = selected_observations(report, request)?;
    let corpus = if let Some(profile) = request.retained_metadata_channel {
        channels::aggregate_metadata(report, request, profile, &ordinals)?
    } else if let Some(profile) = request.html_text_channel {
        channels::aggregate(report, request, profile, &ordinals)?
    } else {
        let mut docs = Vec::with_capacity(ordinals.len());
        for &ordinal in &ordinals {
            let page = &report.pages[ordinal];
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
        aggregate_saved_ngrams(&docs, &request.options)?
    };
    finish_aggregate(corpus, report, identity, ordinals, visible)
}
fn snapshot(context: Context, request: &Request, report: &Aggregate) -> Result<Snapshot> {
    request.validate()?;
    let identity = reporting_identity(&context.directive_reporting, request.reporting_mode)?;
    ensure!(
        report.reporting.as_ref().map(|v| &v.identity) == identity.as_ref(),
        "Stale saved ngram reporting view/profile"
    );
    ensure!(
        report.retained_metadata_profile == request.retained_metadata_channel,
        "Saved aggregate belongs to another retained metadata channel"
    );
    ensure!(
        report.channel_profile == request.html_text_channel,
        "Saved aggregate belongs to another HTML text channel"
    );
    if request.html_text_channel.is_some() || request.retained_metadata_channel.is_some() {
        ensure!(
            report.sources.iter().all(|s| s
                .channel
                .as_ref()
                .is_some_and(|c| c.source_kind == request.source_kind)),
            "Saved aggregate belongs to another raw/DOM source"
        );
    }
    ensure!(
        request
            .expected_generation
            .is_none_or(|g| context.generation == Some(g)),
        "Saved generation changed or is unknown"
    );
    let options_sha256 = if let Some(profile) = request.retained_metadata_channel {
        bounded_hash(
            &json!({"retained_metadata_channel":profile,"source_kind":request.source_kind,"options":request.options,"view":request.view,"n":request.n,"query":request.query,"exact_tokens":request.exact_tokens,"limit":request.limit}),
            REQUEST_BYTES,
        )?
    } else if let Some(profile) = request.html_text_channel {
        bounded_hash(
            &json!({"html_text_channel":profile,"source_kind":request.source_kind,"options":request.options,"view":request.view,"n":request.n,"query":request.query,"exact_tokens":request.exact_tokens,"limit":request.limit}),
            REQUEST_BYTES,
        )?
    } else {
        bounded_hash(
            &json!({"source_kind":request.source_kind,"options":request.options,"view":request.view,"n":request.n,"query":request.query,"exact_tokens":request.exact_tokens,"limit":request.limit}),
            REQUEST_BYTES,
        )?
    };
    let options_sha256 = if let Some(identity) = &identity {
        bounded_hash(&(&options_sha256, identity), REQUEST_BYTES)?
    } else {
        options_sha256
    };
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
        reporting: report.reporting.clone(),
        caller: context,
        source_kind: request.source_kind,
        options_sha256,
        aggregate_sha256,
        snapshot_token,
        html_text_channel: request.html_text_channel,
        retained_metadata_channel: request.retained_metadata_channel,
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
    report: &Aggregate,
    request: &Request,
    mut f: impl FnMut(Value) -> Result<()>,
) -> Result<usize> {
    let mut count = 0usize;
    match request.view {
        View::Sources => {
            for source in &report.sources {
                f(json!({"view":"sources","source":source_value(report, source)?}))?;
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
                            json!({"view":"occurrences","n":row.n,"tokens":row.tokens,"occurrences":membership.occurrences,"source":source_value(report, source)?,"first_positions":membership.first_positions,"positions_sampled":membership.positions_sampled}),
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
    report: &Aggregate,
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
    let mut value = json!({"engine":report.engine,"scope":"all_saved_requested_page_rows_selected_text","snapshot":snapshot,"source_kind":request.source_kind,"view":request.view,"rows":rows,"total_rows":total,"after":after,"next_after":(end<total).then_some(end),"has_more":end<total,"saved_documents":report.sources.len(),"source_states":report.source_states,"all_supplied_sources_complete":report.all_supplied_sources_complete,"candidate_windows":report.candidate_windows,"excluded_stopword_windows":report.excluded_stopword_windows,"included_windows":report.included_windows,"extended_windows":report.extended_windows,"max_n":report.options.max_n,"total_tokens":report.total_tokens,"total_text_bytes":report.total_text_bytes,"token_profile":report.token_profile,"lowercase_unicode_version":report.lowercase_unicode_version,"position_samples_per_source":request.options.limits.position_samples,"limitations":report.limitations,"website_requests":0,"whole_website_coverage_attested":false});
    if let Some(profile) = request.html_text_channel {
        value["scope"] = json!("all_saved_requested_page_rows_retained_html_elements_v1");
        value["html_text_channel"] = json!(profile);
    }
    if let Some(profile) = request.retained_metadata_channel {
        value["scope"] = json!("all_saved_requested_page_rows_retained_metadata_elements_v1");
        value["retained_metadata_channel"] = json!(profile);
    }
    if let Some(reporting) = &report.reporting {
        value["reporting"] = serde_json::to_value(reporting)?;
        if reporting.identity.mode == ReportingMode::Reported {
            let scope = value["scope"]
                .as_str()
                .context("Missing corpus scope")?
                .replacen("all_saved_", "reported_saved_", 1);
            value["scope"] = json!(scope);
        }
    }
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
    report: &Aggregate,
    context: Context,
    request: &Request,
) -> Result<(Vec<u8>, Snapshot)> {
    ensure!(
        request.expected_snapshot_token.is_some() && request.after.is_none(),
        "Complete CSV requires the preview snapshot token and no continuation"
    );
    let snapshot = snapshot(context, request, report)?;
    if request.retained_metadata_channel.is_some() {
        return channels::csv_metadata(report, request, snapshot);
    }
    if request.html_text_channel.is_some() {
        return channels::csv(report, request, snapshot);
    }
    let mut out = Bytes {
        data: vec![],
        cap: CSV_BYTES,
    };
    csv_header(
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
        &snapshot,
    )?;
    visit(report, request, |row| {
        let source = &row["source"];
        let cell = |v: &Value| match v {
            Value::Null => String::new(),
            Value::String(v) => v.clone(),
            v => v.to_string(),
        };
        csv_record(
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
            &snapshot,
            &source["raw_page_ordinal"],
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
#[path = "ngram_app/channel_tests.rs"]
mod channel_tests;
#[path = "ngram_app/channels.rs"]
mod channels;

#[cfg(test)]
#[path = "ngram_app_tests.rs"]
mod tests;
