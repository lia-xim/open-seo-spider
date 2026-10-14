//! Same-TX rich sitemap window. Raw/source tails and analysis context are
//! verified before a continuation is selected. No fabricated partial catalog.
use super::*;
use crate::sitemap_coverage::verified_read::{CoverageReadContext, project_read};
use crate::sitemap_ledger::verified_read::{
    RawWindowByteBudget, SitemapLedgerFilter, SitemapLedgerRowKind, verify_raw,
};
use crate::sitemap_membership_analysis::{
    AnalysisReadInput, VerifiedAnalysisRead, prepare_window_analysis, project_prepared_window,
};
use crate::sitemap_source_store::verified_read::verify_source;
use anyhow::ensure;
use serde::Serialize;
use std::io::{self, Write};

pub const SITEMAP_READ_WINDOW_ENGINE: &str = "open-seo-spider/sitemap-rich-sql-window-v1";
// This is a Rust DTO, with extra verified source/header fields. App retains its
// independent exact8MiB complete-response cap; this bound must not preempt an
// otherwise valid8MiB App response merely because the DTO has extra metadata.
const MAX_RICH_WINDOW_JSON_BYTES: usize = 16 * 1024 * 1024;

/// Pure hash input, not a proof constructor or authorization capability.
/// Callers must obtain every field from the actual verified same-TX reader.
pub struct SitemapReadState<'a> {
    pub summary: &'a crate::SitemapLedgerSummary,
    pub inherited: Option<&'a crate::SitemapInheritedBasis>,
    pub coverage_inventory_sha256: Option<&'a str>,
    pub coverage: &'a crate::SitemapCoverage,
    pub source_measurement_context: &'a str,
    pub analysis_context: Option<&'a crate::SitemapAnalysisContext>,
}

/// Shared by compact inventory and the existing full buffered CSV reader.
/// The explicit new engine makes valid old-profile cursors become stale.
pub fn sitemap_read_state_sha256(state: &SitemapReadState<'_>) -> anyhow::Result<String> {
    Ok(crate::sitemap_ledger::digest(serde_json::to_vec(&(
        SITEMAP_READ_WINDOW_ENGINE,
        state.summary,
        state.inherited,
        state.coverage_inventory_sha256,
        state.coverage,
        state.source_measurement_context,
        crate::SITEMAP_LASTMOD_ENGINE,
        crate::SITEMAP_MEMBERSHIP_ANALYSIS_ENGINE,
        state.analysis_context,
    ))?))
}

/// App checks its job UUID, filter and token roles before calling Core. Core
/// checks all remaining snapshot fields before reading a continuation window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitemapReadWindowExpectedContext {
    pub generation: u64,
    pub crawl_config_sha256: String,
    pub catalog_config_sha256: String,
    pub catalog_revision: u64,
    pub catalog_rolling_sha256: String,
    pub catalog_state_sha256: String,
    pub inherited_basis: Option<crate::SitemapInheritedBasis>,
}
impl SitemapReadWindowExpectedContext {
    fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.generation > 0,
            "Expected sitemap generation must be positive"
        );
        for value in [
            &self.crawl_config_sha256,
            &self.catalog_config_sha256,
            &self.catalog_rolling_sha256,
            &self.catalog_state_sha256,
        ] {
            ensure!(
                value.len() == 64
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
                "Invalid expected sitemap SHA-256"
            );
        }
        Ok(())
    }
    fn matches(&self, read: &SitemapReadWindow) -> bool {
        read.generation == Some(self.generation)
            && read.crawl_config_sha256.as_deref() == Some(self.crawl_config_sha256.as_str())
            && read.catalog_state_sha256.as_deref() == Some(self.catalog_state_sha256.as_str())
            && read.inherited == self.inherited_basis
            && read.ledger.as_ref().is_some_and(|summary| {
                summary.config_sha256 == self.catalog_config_sha256
                    && summary.revision == self.catalog_revision
                    && summary.rolling_sha256 == self.catalog_rolling_sha256
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SitemapReadWindowError {
    StaleContext,
    InvalidContinuation,
    ResponseByteBudget,
}
impl std::fmt::Display for SitemapReadWindowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::StaleContext => "Sitemap read context stale: restart pagination",
            Self::InvalidContinuation => "Continuation sequence is not a matching retained row",
            Self::ResponseByteBudget => "Sitemap read window response byte budget exceeded",
        })
    }
}
impl std::error::Error for SitemapReadWindowError {}

#[derive(Debug, Clone, Serialize)]
pub struct SitemapReadWindow {
    pub crawl_config_sha256: Option<String>,
    pub generation: Option<u64>,
    pub ledger: Option<crate::SitemapLedgerSummary>,
    pub inherited: Option<crate::SitemapInheritedBasis>,
    /// Only the requested bounded window; never a replacement portable catalog.
    /// None denotes unavailable history, Some([]) may also be a context-only read.
    pub rows: Option<Vec<crate::SitemapLedgerRow>>,
    pub next_after_sequence: Option<u64>,
    pub has_more: bool,
    pub coverage: crate::SitemapCoverage,
    pub coverage_inventory_sha256: Option<String>,
    pub sitemap_source_measurement_summary: Option<crate::SitemapSourceStoreSummary>,
    pub sitemap_source_measurement_context: String,
    pub membership_analysis: Option<crate::SitemapMembershipAnalysis>,
    pub reader_engine: String,
    pub catalog_state_sha256: Option<String>,
    pub materializing_selected_compatibility: bool,
}

fn saved_config_bytes(connection: &Connection) -> anyhow::Result<Option<(Vec<u8>, String)>> {
    let length: Option<i64> = connection
        .query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key='config'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let Some(length) = length else {
        return Ok(None);
    };
    ensure!(
        (1..=67_108_864).contains(&length),
        "Saved rich sitemap config exceeds bounded input"
    );
    let bytes: Vec<u8> = connection.query_row(
        "SELECT CAST(value AS BLOB) FROM metadata WHERE key='config'",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        i64::try_from(bytes.len())? == length,
        "Saved rich sitemap config length differs"
    );
    let config: CrawlConfig = serde_json::from_slice(&bytes)?;
    verify_sitemap_config_context(connection, &config)?;
    let sha = crate::sitemap_ledger::digest(&bytes);
    Ok(Some((bytes, sha)))
}
fn raw_filter(request: &crate::SitemapMembershipAnalysisRequest) -> SitemapLedgerFilter {
    use crate::SitemapAnalysisRowKind as K;
    SitemapLedgerFilter {
        kind: match request.kind {
            K::All => SitemapLedgerRowKind::All,
            K::Root => SitemapLedgerRowKind::Root,
            K::Document => SitemapLedgerRowKind::Document,
            K::Entry => SitemapLedgerRowKind::Entry,
            K::Location => SitemapLedgerRowKind::Location,
        },
        source_observation_id: request.source_observation_id.clone(),
        membership_url: request.membership_url.clone(),
    }
}
struct WindowSize(usize);
impl Write for WindowSize {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let size = self
            .0
            .checked_add(bytes.len())
            .filter(|size| *size <= MAX_RICH_WINDOW_JSON_BYTES)
            .ok_or_else(|| io::Error::other(SitemapReadWindowError::ResponseByteBudget))?;
        self.0 = size;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl CrawlStore {
    /// Complete-tail verification, coverage and terminal-analysis context use
    /// one actual read transaction. Expected context is checked before raw-page
    /// position; typed errors preserve stale409 / invalid400 / budget413 in App.
    /// Ordinary raw/source/report payloads are streamed. Selected-parent paths
    /// retain explicit materializing compatibility until independently replaced.
    pub fn sitemap_read_window_with_membership_analysis(
        &self,
        request: &crate::SitemapMembershipAnalysisRequest,
        expected: Option<&SitemapReadWindowExpectedContext>,
    ) -> anyhow::Result<SitemapReadWindow> {
        request.validate()?;
        if let Some(expected) = expected {
            expected.validate()?;
        }
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        super::sitemap_raw_read::preflight_context_metadata(&transaction)?;
        let saved_config = saved_config_bytes(&transaction)?;
        let crawl_config_sha256 = saved_config.as_ref().map(|(_, sha)| sha.clone());
        let selected = matches!(load_run_plan(&transaction)?, RunPlan::SelectedRecrawl(_));
        if selected {
            // Keep both exact parent presence/plan/retained-row guards. Neither
            // a low-level None token nor a parent hash substitutes for these.
            load_sitemap_catalog_verified(&transaction)?;
            load_sitemap_source_measurements_verified(&transaction)?;
        }
        let raw = verify_raw(&transaction)?;
        let source = verify_source(&transaction, raw.as_ref())?;
        let generation = metadata::<CrawlRunState>(&transaction, "run_state")?
            .map(|run| run.generation)
            .filter(|generation| *generation > 0);
        let discovery_initialized =
            metadata_text(&transaction, "discovery_initialized")?.as_deref() == Some("true");
        let coverage = project_read(
            &transaction,
            raw.as_ref(),
            CoverageReadContext {
                saved_config_bytes: saved_config.as_ref().map(|(bytes, _)| bytes.as_slice()),
                saved_config_sha256: crawl_config_sha256.as_deref(),
                generation,
                discovery_initialized,
            },
        )?;
        let source_context = match source.as_ref() {
            Some(source) => source.context().to_owned(),
            None => crate::sitemap_source_store::snapshot_context(None)?,
        };
        let input = AnalysisReadInput {
            raw: raw.as_ref(),
            source: source.as_ref(),
            generation,
            crawl_config_sha256: crawl_config_sha256.as_deref(),
            coverage: &coverage.coverage,
        };
        let analysis: VerifiedAnalysisRead<'_, '_> = prepare_window_analysis(&transaction, &input)?;
        let state = raw
            .as_ref()
            .map(|raw| {
                sitemap_read_state_sha256(&SitemapReadState {
                    summary: raw.summary(),
                    inherited: raw.inherited(),
                    coverage_inventory_sha256: coverage.inventory_sha256.as_deref(),
                    coverage: &coverage.coverage,
                    source_measurement_context: &source_context,
                    analysis_context: Some(analysis.context()),
                })
            })
            .transpose()?;
        let mut read = SitemapReadWindow {
            crawl_config_sha256: crawl_config_sha256.clone(),
            generation,
            ledger: raw.as_ref().map(|raw| raw.summary().clone()),
            inherited: raw.as_ref().and_then(|raw| raw.inherited().cloned()),
            rows: raw.as_ref().map(|_| Vec::new()),
            next_after_sequence: None,
            has_more: false,
            coverage: coverage.coverage.clone(),
            coverage_inventory_sha256: coverage.inventory_sha256.clone(),
            sitemap_source_measurement_summary: source
                .as_ref()
                .map(|source| source.summary().clone()),
            sitemap_source_measurement_context: source_context,
            membership_analysis: None,
            reader_engine: SITEMAP_READ_WINDOW_ENGINE.into(),
            catalog_state_sha256: state,
            materializing_selected_compatibility: selected
                || coverage.materializing_selected_compatibility
                || source
                    .as_ref()
                    .is_some_and(|source| source.materializing_selected_compatibility()),
        };
        if expected.is_some_and(|expected| !expected.matches(&read)) {
            return Err(SitemapReadWindowError::StaleContext.into());
        }
        let filter = raw_filter(request);
        if let Some(after) = request.after_sequence {
            let matching = match raw.as_ref() {
                Some(raw) => raw.continuation_matches(&filter, after)?,
                None => false,
            };
            if !matching {
                return Err(SitemapReadWindowError::InvalidContinuation.into());
            }
        }
        if let Some(raw) = raw.as_ref()
            && request.row_limit > 0
        {
            let page = raw
                .window(&filter, request.after_sequence, request.row_limit)
                .map_err(|error| {
                    if error.is::<RawWindowByteBudget>() {
                        SitemapReadWindowError::ResponseByteBudget.into()
                    } else {
                        error
                    }
                })?;
            read.rows = Some(page.rows);
            read.next_after_sequence = page.next_after_sequence;
            read.has_more = page.has_more;
        }
        read.membership_analysis = Some(project_prepared_window(
            &analysis,
            raw.as_ref(),
            &coverage.coverage,
            read.rows.as_deref().unwrap_or(&[]),
            request,
        )?);
        serde_json::to_writer(&mut WindowSize(0), &read).map_err(|error| -> anyhow::Error {
            if error.is_io() {
                SitemapReadWindowError::ResponseByteBudget.into()
            } else {
                error.into()
            }
        })?;
        drop(analysis);
        drop(source);
        drop(raw);
        transaction.commit()?;
        Ok(read)
    }
}

#[cfg(test)]
#[path = "sitemap_read_window_tests.rs"]
mod tests;
