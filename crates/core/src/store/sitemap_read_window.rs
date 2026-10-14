//! Same-TX rich sitemap window. Raw/source tails and analysis context are
//! verified before a continuation is selected. No fabricated partial catalog.
use super::sitemap_read_context::{SitemapVerifiedReadHeader, with_prepared_sitemap_read};
use super::*;
use crate::sitemap_ledger::verified_read::{
    RawWindowByteBudget, SitemapLedgerFilter, SitemapLedgerRowKind,
};
use crate::sitemap_membership_analysis::project_prepared_window;
use crate::sitemap_read_control::SitemapReadControl;
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
    pub(super) fn validate(&self) -> anyhow::Result<()> {
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
    pub(super) fn matches_header(&self, read: &SitemapVerifiedReadHeader) -> bool {
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
    /// Complete verification and Expected comparison precede continuation
    /// selection. Shared preparation is also used by callback-bound exports.
    pub fn sitemap_read_window_with_membership_analysis(
        &self,
        request: &crate::SitemapMembershipAnalysisRequest,
        expected: Option<&SitemapReadWindowExpectedContext>,
    ) -> anyhow::Result<SitemapReadWindow> {
        request.validate()?;
        with_prepared_sitemap_read(self, expected, &SitemapReadControl::default(), |prepared| {
            if expected.is_some_and(|expected| !expected.matches_header(&prepared.header)) {
                return Err(SitemapReadWindowError::StaleContext.into());
            }
            let header = prepared.header;
            let rows = header.ledger.as_ref().map(|_| Vec::new());
            let mut read = SitemapReadWindow {
                crawl_config_sha256: header.crawl_config_sha256,
                generation: header.generation,
                ledger: header.ledger,
                inherited: header.inherited,
                rows,
                next_after_sequence: None,
                has_more: false,
                coverage: header.coverage,
                coverage_inventory_sha256: header.coverage_inventory_sha256,
                sitemap_source_measurement_summary: header.sitemap_source_measurement_summary,
                sitemap_source_measurement_context: header.sitemap_source_measurement_context,
                membership_analysis: None,
                reader_engine: header.reader_engine,
                catalog_state_sha256: header.catalog_state_sha256,
                materializing_selected_compatibility: header.materializing_selected_compatibility,
            };
            let filter = raw_filter(request);
            if let Some(after) = request.after_sequence {
                let matching = match prepared.raw {
                    Some(raw) => raw.continuation_matches(&filter, after)?,
                    None => false,
                };
                if !matching {
                    return Err(SitemapReadWindowError::InvalidContinuation.into());
                }
            }
            if let Some(raw) = prepared.raw
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
                prepared.analysis,
                prepared.raw,
                prepared.coverage,
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
            Ok(read)
        })
    }
}

#[cfg(test)]
#[path = "sitemap_read_window_tests.rs"]
mod tests;
