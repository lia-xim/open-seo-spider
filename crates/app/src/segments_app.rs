//! One saved segment service for HTTP, explicit offline CLI and the MCP HTTP adapter.
use anyhow::{Result, ensure};
use open_seo_spider_core::{
    CrawlReport,
    segments::{self, Projection, Rules, Selection},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum View {
    #[default]
    Pages,
    Issues,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Cursor {
    pub offset: usize,
    pub view: View,
    pub snapshot_token: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Request {
    pub rules: Rules,
    pub selection: Selection,
    pub view: View,
    pub limit: usize,
    pub cursor: Option<Cursor>,
    pub expected_snapshot_token: Option<String>,
    pub expected_generation: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting_mode: Option<segments::ReportingMode>,
}
impl Default for Request {
    fn default() -> Self {
        Self {
            rules: Rules::default(),
            selection: Selection::All,
            view: View::Pages,
            limit: 100,
            cursor: None,
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
            "Segment page limit must be1..200"
        );
        segments::validate_rules(&self.rules)?;
        for token in self
            .expected_snapshot_token
            .iter()
            .chain(self.cursor.iter().map(|c| &c.snapshot_token))
        {
            ensure!(
                token.len() == 64
                    && token
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "Invalid snapshot token"
            );
        }
        if let Some(c) = &self.cursor {
            ensure!(
                c.view == self.view && c.offset > 0,
                "Cursor view/offset mismatch"
            );
            ensure!(
                self.expected_snapshot_token.as_ref() == Some(&c.snapshot_token),
                "Continuation requires the original snapshot token"
            );
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Context {
    pub engine: &'static str,
    pub job_id: Option<String>,
    pub app_generation: Option<u64>,
    pub origin: String,
    pub typed_report_sha256: String,
    pub rules_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting_view: Option<segments::ReportingIdentity>,
    pub snapshot_token: String,
    pub sql_attested: bool,
    pub report_generation_attested: bool,
}
struct HashWriter {
    hash: Sha256,
    bytes: usize,
}
impl Write for HashWriter {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(b.len())
            .filter(|n| *n <= segments::REPORT_BYTES)
            .ok_or_else(|| {
                io::Error::other("Saved report exceeds segment serialization ceiling")
            })?;
        self.hash.update(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(crate) fn report_hash(report: &CrawlReport) -> Result<String> {
    ensure!(
        report.pages.len() <= segments::MAX_PAGES && report.issues.len() <= segments::MAX_ISSUES,
        "Saved segment row ceiling exceeded"
    );
    let mut w = HashWriter {
        hash: Sha256::new(),
        bytes: 0,
    };
    serde_json::to_writer(&mut w, report)?;
    Ok(format!("{:x}", w.hash.finalize()))
}
pub(crate) fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) struct Evaluation {
    pub context: Context,
    pub projection: Projection,
}
pub(crate) fn evaluate(
    report: &CrawlReport,
    request: &Request,
    job: Option<String>,
    generation: Option<u64>,
    origin: &str,
) -> Result<Evaluation> {
    request.validate()?;
    ensure!(
        request.expected_generation.is_none() || request.expected_generation == generation,
        "Expected generation conflicts with this snapshot"
    );
    let typed_report_sha256 = report_hash(report)?;
    let rules_sha256 = sha(&segments::json_bytes(&request.rules)?);
    let reporting_view = segments::reporting_identity(&report.config, request.reporting_mode)?;
    // Raw snapshot authority is unchanged; separate reporting identity binds every cursor/export.
    // Page/issue view remains separately checked by Cursor.
    let token = if let Some(identity) = &reporting_view {
        sha(&segments::json_bytes(&(
            segments::ENGINE,
            &job,
            generation,
            origin,
            &typed_report_sha256,
            &rules_sha256,
            &request.selection,
            identity,
        ))?)
    } else {
        // Exact legacy token formula for inactive default requests.
        sha(&segments::json_bytes(&(
            segments::ENGINE,
            &job,
            generation,
            origin,
            &typed_report_sha256,
            &rules_sha256,
            &request.selection,
        ))?)
    };
    ensure!(
        request
            .expected_snapshot_token
            .as_ref()
            .is_none_or(|t| *t == token),
        "Snapshot/rules/selection/reporting view changed; preview again"
    );
    let projection = segments::project_with_reporting(
        report,
        &request.rules,
        &request.selection,
        request.reporting_mode,
    )?;
    ensure!(
        projection.reporting.as_ref().map(|value| &value.identity) == reporting_view.as_ref(),
        "Reporting identity mismatch"
    );
    Ok(Evaluation {
        context: Context {
            engine: segments::ENGINE,
            job_id: job,
            app_generation: generation,
            origin: origin.into(),
            typed_report_sha256,
            rules_sha256,
            reporting_view,
            snapshot_token: token,
            sql_attested: false,
            report_generation_attested: false,
        },
        projection,
    })
}
pub(crate) fn page(e: &Evaluation, request: &Request) -> Result<Vec<u8>> {
    let total = match request.view {
        View::Pages => e.projection.pages.len(),
        View::Issues => e.projection.issues.len(),
    };
    let offset = request.cursor.as_ref().map_or(0, |c| c.offset);
    ensure!(offset <= total, "Cursor offset exceeds selected result");
    let end = offset.saturating_add(request.limit).min(total);
    let rows = match request.view {
        View::Pages => serde_json::to_value(&e.projection.pages[offset..end])?,
        View::Issues => serde_json::to_value(&e.projection.issues[offset..end])?,
    };
    let next = (end < total).then(|| Cursor {
        offset: end,
        view: request.view,
        snapshot_token: e.context.snapshot_token.clone(),
    });
    let mut output = serde_json::json!({"context":e.context,"depth_policy":e.projection.depth_policy,"counts":e.projection.counts,"charts":e.projection.charts,"summary":{"total_pages":e.projection.total_pages,"selected_pages":e.projection.selected_pages,"selected_issues":e.projection.issues.len(),"multiple_matches":e.projection.multiple_matches,"unassigned_pages":e.projection.unassigned_pages,"unknown_pages":e.projection.unknown_pages,"orphan_issues":e.projection.orphan_issues},"view":request.view,"total":total,"rows":rows,"next":next});
    if let Some(reporting) = &e.projection.reporting {
        output["reporting"] = serde_json::to_value(reporting)?;
    }
    segments::json_bytes(&output)
}
pub(crate) fn full_json(e: &Evaluation) -> Result<Vec<u8>> {
    segments::json_bytes(&serde_json::json!({"context":e.context,"projection":e.projection}))
}
pub(crate) fn full_csv(e: &Evaluation) -> Result<Vec<u8>> {
    segments::csv_bytes(&e.projection, &e.context)
}
async fn bounded(path: &Path, cap: usize) -> Result<Vec<u8>> {
    let f = tokio::fs::File::open(path).await?;
    ensure!(
        f.metadata().await?.is_file() && f.metadata().await?.len() <= u64::try_from(cap)?,
        "Input is not a bounded regular JSON file"
    );
    let mut b = Vec::new();
    f.take(u64::try_from(cap)?.saturating_add(1))
        .read_to_end(&mut b)
        .await?;
    ensure!(b.len() <= cap, "Input grew beyond byte ceiling");
    Ok(b)
}
pub(crate) async fn run_cli(
    report: PathBuf,
    request: PathBuf,
    output: PathBuf,
    csv: bool,
) -> Result<()> {
    // create_new never replaces either input or an existing destination, including aliases.
    let options = bounded(&request, segments::REQUEST_BYTES).await?;
    let request: Request = serde_json::from_slice(&options)?;
    request.validate()?;
    ensure!(
        request.cursor.is_none(),
        "Full CLI export does not accept a page cursor"
    );
    // Unsupported predicate fields are refused before even opening/reading the saved report.
    let bytes = bounded(&report, segments::REPORT_BYTES).await?;
    let result = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let report: CrawlReport = serde_json::from_slice(&bytes)?;
        let e = evaluate(
            &report,
            &request,
            None,
            None,
            "declared_offline_report_file",
        )?;
        if csv { full_csv(&e) } else { full_json(&e) }
    })
    .await??;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .await?;
    file.write_all(&result).await?;
    file.sync_all().await?;
    Ok(())
}
#[cfg(test)]
#[path = "segments_app_tests.rs"]
mod tests;
