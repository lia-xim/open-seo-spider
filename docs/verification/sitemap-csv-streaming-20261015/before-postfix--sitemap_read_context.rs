//! One preparation for bounded inventory windows and callback-bound exports.
//! The actual Connection/TX and all private proofs live until the callback ends.
use super::*;
use crate::sitemap_coverage::verified_read::{CoverageReadContext, project_read_with_control};
use crate::sitemap_ledger::verified_read::{VerifiedRawRead, verify_raw_with_control};
use crate::sitemap_membership_analysis::{
    AnalysisReadInput, VerifiedAnalysisRead, prepare_window_analysis_with_control,
};
use crate::sitemap_read_control::SitemapReadControl;
use crate::sitemap_source_store::verified_read::verify_source_with_control;
use anyhow::ensure;
use serde::Serialize;

/// Owned context from one actual verified read, not a constructor for a proof.
/// No rows, complete catalog, source-record list or task inventory is retained.
#[derive(Debug, Clone, Serialize)]
pub struct SitemapVerifiedReadHeader {
    pub crawl_config_sha256: Option<String>,
    pub generation: Option<u64>,
    pub ledger: Option<crate::SitemapLedgerSummary>,
    pub inherited: Option<crate::SitemapInheritedBasis>,
    pub coverage: crate::SitemapCoverage,
    pub coverage_inventory_sha256: Option<String>,
    pub sitemap_source_measurement_summary: Option<crate::SitemapSourceStoreSummary>,
    pub sitemap_source_measurement_context: String,
    pub analysis_context: crate::SitemapAnalysisContext,
    pub reader_engine: String,
    pub catalog_state_sha256: Option<String>,
    pub materializing_selected_compatibility: bool,
}

// These references cannot leave the caller-owned transaction. The public
// stored-row session exposes neither these proof fields nor SQL access.
pub(super) struct PreparedSitemapRead<'read> {
    pub(super) header: SitemapVerifiedReadHeader,
    pub(super) raw: Option<&'read VerifiedRawRead<'read>>,
    pub(super) analysis: &'read VerifiedAnalysisRead<'read, 'read>,
    pub(super) coverage: &'read crate::SitemapCoverage,
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

/// One actual same-TX preparation, then a borrowing callback. The callback's
/// result is returned only after proofs have ended and the read TX has closed.
/// Existing selected/import/policy materializations remain coarse checkpoints;
/// this does not claim a hard maximum cancellation latency or measured RSS.
pub(super) fn with_prepared_sitemap_read<T>(
    store: &CrawlStore,
    expected: Option<&SitemapReadWindowExpectedContext>,
    control: &SitemapReadControl,
    action: impl FnOnce(PreparedSitemapRead<'_>) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    control.checkpoint()?;
    if let Some(expected) = expected {
        expected.validate()?;
    }
    let mut connection = store
        .connection
        .lock()
        .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
    control.checkpoint()?;
    let _interrupt_guard = control.bind_connection(&connection)?;
    let result = (|| {
        let transaction = connection.transaction()?;
        super::sitemap_raw_read::preflight_context_metadata(&transaction)?;
        control.checkpoint()?;
        let saved_config = saved_config_bytes(&transaction)?;
        control.checkpoint()?;
        let crawl_config_sha256 = saved_config.as_ref().map(|(_, sha)| sha.clone());
        let selected = matches!(load_run_plan(&transaction)?, RunPlan::SelectedRecrawl(_));
        control.checkpoint()?;
        if selected {
            // Both exact parent-presence/plan/retained-row guards remain. An
            // inherited flag or low-level None token never replaces them.
            load_sitemap_catalog_verified(&transaction)?;
            control.checkpoint()?;
            load_sitemap_source_measurements_verified(&transaction)?;
            control.checkpoint()?;
        }
        let raw = verify_raw_with_control(&transaction, control)?;
        let source = verify_source_with_control(&transaction, raw.as_ref(), control)?;
        let generation = metadata::<CrawlRunState>(&transaction, "run_state")?
            .map(|run| run.generation)
            .filter(|generation| *generation > 0);
        let discovery_initialized =
            metadata_text(&transaction, "discovery_initialized")?.as_deref() == Some("true");
        let coverage = project_read_with_control(
            &transaction,
            raw.as_ref(),
            CoverageReadContext {
                saved_config_bytes: saved_config.as_ref().map(|(bytes, _)| bytes.as_slice()),
                saved_config_sha256: crawl_config_sha256.as_deref(),
                generation,
                discovery_initialized,
            },
            control,
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
        let analysis: VerifiedAnalysisRead<'_, '_> =
            prepare_window_analysis_with_control(&transaction, &input, control)?;
        control.checkpoint()?;
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
        let header = SitemapVerifiedReadHeader {
            crawl_config_sha256,
            generation,
            ledger: raw.as_ref().map(|raw| raw.summary().clone()),
            inherited: raw.as_ref().and_then(|raw| raw.inherited().cloned()),
            coverage: coverage.coverage.clone(),
            coverage_inventory_sha256: coverage.inventory_sha256.clone(),
            sitemap_source_measurement_summary: source
                .as_ref()
                .map(|source| source.summary().clone()),
            sitemap_source_measurement_context: source_context,
            analysis_context: analysis.context().clone(),
            reader_engine: SITEMAP_READ_WINDOW_ENGINE.into(),
            catalog_state_sha256: state,
            materializing_selected_compatibility: selected
                || coverage.materializing_selected_compatibility
                || source
                    .as_ref()
                    .is_some_and(|source| source.materializing_selected_compatibility()),
        };
        let value = action(PreparedSitemapRead {
            header,
            raw: raw.as_ref(),
            analysis: &analysis,
            coverage: &coverage.coverage,
        })?;
        control.checkpoint()?;
        drop(analysis);
        drop(source);
        drop(raw);
        transaction.commit()?;
        control.checkpoint()?;
        Ok(value)
    })();
    // Interrupt may make sqlite_step return SQLITE_INTERRUPT before a Rust
    // loop checkpoint. Only an actual requested stop gets the typed stop error.
    control.checkpoint()?;
    result
}
