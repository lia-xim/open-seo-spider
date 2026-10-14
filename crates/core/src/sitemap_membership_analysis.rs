//! Read-only joins against original requested identities in a checkpoint-bound
//! saved report. Zero observed anchors never proves a website-wide orphan.
use crate::{
    CrawlCheckpoint, CrawlOutcome, CrawlRunState, SitemapAdmissionDecision, SitemapCoverageScope,
    SitemapLedgerRow, SitemapLocationRole, SitemapReadSnapshot,
};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

mod window;
use window::MembershipBasis;
pub(crate) use window::{
    AnalysisReadInput, VerifiedAnalysisRead, prepare_window_analysis, project_prepared_window,
};

pub const SITEMAP_MEMBERSHIP_ANALYSIS_ENGINE: &str =
    "open-seo-spider/sitemap-target-inlink-analysis-v1";
const MAX_REPORT_BYTES: i64 = 268_435_456;
const MAX_PAGES: i64 = 1_000_000;
const MAX_LINKS: i64 = 5_000_000;
const MAX_FIELD_BYTES: usize = 65_536;
const MAX_METADATA_BYTES: i64 = 1_048_576;
const SAMPLE_LIMIT: usize = 5;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapAnalysisRowKind {
    #[default]
    All,
    Root,
    Document,
    Entry,
    Location,
}

#[derive(Debug, Clone, Default)]
pub struct SitemapMembershipAnalysisRequest {
    pub kind: SitemapAnalysisRowKind,
    pub source_observation_id: Option<String>,
    pub membership_url: Option<String>,
    pub after_sequence: Option<u64>,
    /// Zero captures only the stable report context (e.g. raw CSV export).
    pub row_limit: usize,
    pub after_page_ordinal: Option<u64>,
    /// Independently paged saved requested URLs. Zero requests no page rows.
    pub page_limit: usize,
}
impl SitemapMembershipAnalysisRequest {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.row_limit <= 200 && self.page_limit <= 200,
            "Sitemap analysis page limit must be 0..200"
        );
        ensure!(
            self.after_sequence != Some(0),
            "Invalid sitemap analysis sequence"
        );
        if let Some(value) = &self.source_observation_id {
            ensure!(
                value.len() == 64
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "Invalid source identity"
            );
        }
        ensure!(
            self.after_page_ordinal.is_none_or(|v| v < MAX_PAGES as u64),
            "Invalid saved-page ordinal"
        );
        for value in [&self.membership_url].into_iter().flatten() {
            ensure!(canonical(value), "Expected exact original HTTP(S) identity");
        }
        Ok(())
    }
    fn matches(&self, row: &SitemapLedgerRow) -> bool {
        let (kind, source, member) = match row {
            SitemapLedgerRow::Root(r) => (
                SitemapAnalysisRowKind::Root,
                r.source_observation_id.as_deref(),
                r.admission.membership_url.as_deref(),
            ),
            SitemapLedgerRow::Document(r) => (
                SitemapAnalysisRowKind::Document,
                Some(r.observation_id.as_str()),
                None,
            ),
            SitemapLedgerRow::Entry(r) => (
                SitemapAnalysisRowKind::Entry,
                Some(r.observation_id.as_str()),
                None,
            ),
            SitemapLedgerRow::Location(r) => (
                SitemapAnalysisRowKind::Location,
                Some(r.observation_id.as_str()),
                r.admission.membership_url.as_deref(),
            ),
        };
        (self.kind == SitemapAnalysisRowKind::All || self.kind == kind)
            && self
                .source_observation_id
                .as_deref()
                .is_none_or(|v| Some(v) == source)
            && self
                .membership_url
                .as_deref()
                .is_none_or(|v| Some(v) == member)
            && self.after_sequence.is_none_or(|v| row.sequence() > v)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapAnalysisContext {
    pub engine: String,
    pub available: bool,
    pub reason: String,
    pub generation: Option<u64>,
    pub checkpoint_revision: Option<u64>,
    pub outcome: Option<CrawlOutcome>,
    pub report_sha256: Option<String>,
    pub report_bytes: Option<u64>,
    pub saved_page_count: Option<u64>,
    pub pending_page_count: Option<u64>,
    pub source_scope: SitemapCoverageScope,
    pub observation_scope: String,
    pub website_wide_orphan_inference: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapTargetState {
    BlockedByRobots,
    Redirected,
    Non200,
    NonIndexable,
    FetchFailed,
    IncompleteObservation,
    Indexable,
    NoResponse,
    Pending,
    NoStoredRequestedObservation,
    InvalidOrMissingOriginalLocation,
    RenderedNavigationChanged,
    RenderedNon200,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapTargetObservation {
    pub report_page_ordinal: u64,
    pub requested_url: String,
    pub final_url: String,
    pub status: Option<u16>,
    pub final_status: Option<u16>,
    pub indexable: bool,
    pub indexability_reasons: Vec<String>,
    pub blocked_by_robots: bool,
    pub error: Option<String>,
    pub truncated: bool,
    pub renderer_error: Option<String>,
    pub rendered: bool,
    pub rendered_url: Option<String>,
    /// Safe normalized browser fetch identity used ONLY to compare navigation;
    /// original membership joins never use it. Raw rendered_url is retained.
    pub rendered_fetch_url: Option<String>,
    pub rendered_status: Option<u16>,
    pub rendering_expected: bool,
    pub rendering_state: String,
    pub rendered_navigation_changed: bool,
    pub redirect_chain_present: bool,
    pub fetched_at: String,
    /// This is generation provenance, not an assertion of present freshness.
    pub age_scope: String,
    pub state: SitemapTargetState,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapObservedInlinks {
    pub anchor_occurrences: u64,
    pub nonself_anchor_occurrences: u64,
    pub internal_nonself_anchor_occurrences: u64,
    pub distinct_nonself_requested_sources: u64,
    pub samples: Vec<SitemapInlinkSample>,
    pub samples_have_more: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapInlinkSample {
    pub source_requested_url: String,
    pub target_requested_url: String,
    pub source_page_ordinal: u64,
    pub link_ordinal: u64,
    pub text: String,
    pub rel: String,
    pub internal: bool,
    pub nofollow: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapLocationFinding {
    pub sequence: u64,
    pub observation_id: String,
    pub occurrence_id: String,
    pub entry_ordinal: usize,
    pub original_membership_url: Option<String>,
    pub admitted_fetch_url: Option<String>,
    pub admission_decision: SitemapAdmissionDecision,
    pub original_state: SitemapTargetState,
    pub original_observation: Option<SitemapTargetObservation>,
    /// Separate alias evidence; never a replacement for the original identity.
    pub admitted_alias_observation: Option<SitemapTargetObservation>,
    pub admitted_alias_pending: bool,
    pub observed_original_inlinks: Option<SitemapObservedInlinks>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapSavedPageMembership {
    ListedOriginal,
    NotListedCurrentDeclaredSources,
    NotListedHistoricalDeclaredSources,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSavedPageFinding {
    pub observation: SitemapTargetObservation,
    pub membership: SitemapSavedPageMembership,
    pub original_membership_occurrences: u64,
    /// DISTINCT requested XML source task URLs, never redirected final URLs.
    pub original_source_document_count: u64,
    pub original_source_observation_count: u64,
    pub original_source_document_samples: Vec<SitemapSourceDocumentSample>,
    pub source_document_samples_have_more: bool,
    pub admitted_alias_occurrences: u64,
    pub observed_inlinks: SitemapObservedInlinks,
    /// Qualified candidate among observed report anchors, never universal orphan.
    pub listed_without_observed_nonself_inlinks: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceDocumentSample {
    pub observation_id: String,
    pub task_url: String,
    pub source_final_url: Option<String>,
}
#[derive(Default)]
struct MembershipCounts<'a> {
    occurrences: u64,
    aliases: u64,
    /// Borrow verified catalog identities; no clone of the full source catalog.
    sources: BTreeMap<&'a str, BTreeSet<&'a str>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapMembershipAnalysis {
    pub context: SitemapAnalysisContext,
    /// None means unavailable; Some([]) is an available empty visible selection.
    pub location_findings: Option<Vec<SitemapLocationFinding>>,
    pub saved_pages: Option<Vec<SitemapSavedPageFinding>>,
    pub saved_pages_have_more: bool,
    pub last_saved_page_url: Option<String>,
    pub last_saved_page_ordinal: Option<u64>,
}
fn canonical(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_FIELD_BYTES
        && !value.chars().any(char::is_control)
        && crate::normalize_url(value, None).is_some_and(|v| v.as_str() == value)
}
fn bounded_text(connection: &Connection, key: &str) -> Result<Option<String>> {
    let length: Option<i64> = connection
        .query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key=?1",
            [key],
            |row| row.get(0),
        )
        .optional()?;
    let Some(length) = length else {
        return Ok(None);
    };
    ensure!(
        (1..=MAX_METADATA_BYTES).contains(&length),
        "Sitemap analysis metadata input budget exceeded"
    );
    let value: String =
        connection.query_row("SELECT value FROM metadata WHERE key=?1", [key], |row| {
            row.get(0)
        })?;
    ensure!(
        i64::try_from(value.len())? == length,
        "Analysis metadata byte length differs"
    );
    Ok(Some(value))
}
fn meta<T: DeserializeOwned>(connection: &Connection, key: &str) -> Result<Option<T>> {
    bounded_text(connection, key)?
        .map(|v| serde_json::from_str(&v).map_err(Into::into))
        .transpose()
}
fn count(connection: &Connection, sql: &str) -> Result<u64> {
    Ok(u64::try_from(
        connection.query_row(sql, [], |r| r.get::<_, i64>(0))?,
    )?)
}
fn report_hash(connection: &Connection) -> Result<Option<(String, u64)>> {
    let length: Option<i64> = connection
        .query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key='report'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let Some(length) = length else {
        return Ok(None);
    };
    ensure!(
        (1..=MAX_REPORT_BYTES).contains(&length),
        "Sitemap analysis report input budget exceeded"
    );
    // Borrow SQLite's actual bounded value once: no Vec<PageRecord>, no full
    // Rust report clone, and no repeated CAST+substr of the full TEXT value.
    let (sha, actual): (String, usize) =
        connection.query_row("SELECT value FROM metadata WHERE key='report'", [], |row| {
            let bytes = row.get_ref(0)?.as_bytes()?;
            Ok((format!("{:x}", Sha256::digest(bytes)), bytes.len()))
        })?;
    ensure!(
        i64::try_from(actual)? == length,
        "Saved analysis report byte length differs"
    );
    Ok(Some((sha, u64::try_from(length)?)))
}

// Bound the serialized report projection before materializing it. This is a
// separate read envelope; the durable Core validator retains its own smaller
// record/logical-catalog budgets. Compare typed fields with the already verified
// same-TX Store snapshot rather than trusting a report-only summary.
fn validate_report_source_measurements(
    connection: &Connection,
    expected: Option<&crate::SitemapSourceStoreSnapshot>,
) -> Result<()> {
    let (kind, bytes): (Option<String>, Option<i64>) = connection.query_row(
        "SELECT json_type(value,'$.sitemap_source_measurements'),CASE WHEN json_type(value,'$.sitemap_source_measurements')='object' THEN length(CAST(json_extract(value,'$.sitemap_source_measurements') AS BLOB)) END FROM metadata WHERE key='report'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let Some(expected) = expected else {
        ensure!(
            kind.is_none() || kind.as_deref() == Some("null"),
            "Sitemap analysis saved source measurements have no verified SQL basis"
        );
        return Ok(());
    };
    ensure!(
        kind.as_deref() == Some("object") && bytes.is_some_and(|n| (1..=67_108_864).contains(&n)),
        "Sitemap analysis saved source measurement envelope differs or exceeds read budget"
    );
    let raw: Vec<u8> = connection.query_row(
        "SELECT CAST(json_extract(value,'$.sitemap_source_measurements') AS BLOB) FROM metadata WHERE key='report'",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        Some(i64::try_from(raw.len())?) == bytes,
        "Sitemap analysis source measurement byte length differs"
    );
    let saved: crate::SitemapSourceStoreSnapshot = serde_json::from_slice(&raw)
        .context("Sitemap analysis saved source measurement projection is invalid")?;
    ensure!(
        &saved == expected,
        "Sitemap analysis saved source measurements differ from verified SQL snapshot"
    );
    Ok(())
}

enum SourceBasis<'a> {
    Snapshot(Option<&'a crate::SitemapSourceStoreSnapshot>),
    Verified(Option<&'a crate::sitemap_source_store::verified_read::VerifiedSourceRead<'a>>),
}
impl SourceBasis<'_> {
    fn summary(&self) -> Option<&crate::SitemapSourceStoreSummary> {
        match self {
            Self::Snapshot(value) => value.map(|v| &v.summary),
            Self::Verified(value) => value.map(|v| v.summary()),
        }
    }
    fn verify_report(&self, connection: &Connection) -> Result<()> {
        match self {
            Self::Snapshot(value) => validate_report_source_measurements(connection, *value),
            Self::Verified(value) => {
                crate::sitemap_source_store::verified_report::verify_saved_report(
                    connection, *value,
                )
            }
        }
    }
}
struct ContextInput<'a> {
    generation: Option<u64>,
    config_sha: Option<&'a str>,
    coverage: &'a crate::SitemapCoverage,
    summary: Option<&'a crate::SitemapLedgerSummary>,
    source: SourceBasis<'a>,
}
struct PreparedContext {
    base: SitemapMembershipAnalysis,
    selected: bool,
    generation: Option<u64>,
}

/// Historical materialized reader remains an oracle for membership counting.
/// Target, report/checkpoint guards, renderer provenance and paging share code.
pub(crate) fn project(
    connection: &Connection,
    read: &SitemapReadSnapshot,
    request: &SitemapMembershipAnalysisRequest,
) -> Result<SitemapMembershipAnalysis> {
    request.validate()?;
    let catalog = read.catalog.as_ref().filter(|v| v.rows.is_some());
    let rows = catalog.and_then(|v| v.rows.as_deref()).unwrap_or_default();
    let visible = rows
        .iter()
        .filter(|r| request.matches(r))
        .take(request.row_limit)
        .filter_map(|r| match r {
            SitemapLedgerRow::Location(loc)
                if loc.observed.role == SitemapLocationRole::PageLocation =>
            {
                Some(loc.as_ref())
            }
            _ => None,
        })
        .collect();
    let prepared = prepare_context(
        connection,
        ContextInput {
            generation: read.generation,
            config_sha: read.crawl_config_sha256.as_deref(),
            coverage: &read.coverage,
            summary: catalog.map(|v| &v.summary),
            source: SourceBasis::Snapshot(read.sitemap_source_measurements.as_ref()),
        },
    )?;
    project_rows(
        connection,
        &prepared,
        &read.coverage,
        visible,
        MembershipBasis::Materialized(rows),
        request,
    )
}

fn prepare_context(connection: &Connection, input: ContextInput<'_>) -> Result<PreparedContext> {
    let ContextInput {
        generation,
        config_sha,
        coverage,
        summary,
        source,
    } = input;
    let mut selected = false;
    let mut report_generation = generation;

    let run: Option<CrawlRunState> = meta(connection, "run_state")?;
    let checkpoint: Option<CrawlCheckpoint> = meta(connection, "checkpoint")?;
    let report = report_hash(connection)?;
    let mut out = SitemapMembershipAnalysis {
        context: SitemapAnalysisContext {
            engine: SITEMAP_MEMBERSHIP_ANALYSIS_ENGINE.into(),
            available: false,
            reason: "terminal_saved_report_unavailable".into(),
            generation,
            checkpoint_revision: checkpoint.as_ref().map(|v| v.revision),
            outcome: run.as_ref().and_then(|v| v.outcome),
            report_sha256: report.as_ref().map(|v| v.0.clone()),
            report_bytes: report.as_ref().map(|v| v.1),
            saved_page_count: None,
            pending_page_count: None,
            source_scope: coverage.scope,
            observation_scope: "unavailable".into(),
            website_wide_orphan_inference: false,
        },
        location_findings: None,
        saved_pages: None,
        saved_pages_have_more: false,
        last_saved_page_url: None,
        last_saved_page_ordinal: None,
    };
    let Some(summary) = summary else {
        out.context.reason = "validated_source_catalog_unavailable".into();
        return Ok(PreparedContext {
            base: out,
            selected,
            generation: report_generation,
        });
    };
    let (Some(run), Some(checkpoint), Some((sha, _))) = (run, checkpoint, report) else {
        return Ok(PreparedContext {
            base: out,
            selected,
            generation: report_generation,
        });
    };
    if run.outcome.is_none() || run.finished_at.is_none() {
        out.context.reason = "live_crawl_target_analysis_unavailable".into();
        return Ok(PreparedContext {
            base: out,
            selected,
            generation: report_generation,
        });
    }
    let completed = count(connection, "SELECT count(*) FROM pages")?;
    let pending = count(connection, "SELECT count(*) FROM frontier WHERE done=0")?;
    let discovery_pending = count(
        connection,
        "SELECT count(*) FROM discovery_frontier WHERE done=0",
    )?;
    let discovery_complete =
        bounded_text(connection, "discovery_complete")?.as_deref() == Some("true");
    if run.generation != checkpoint.generation
        || Some(run.generation) != generation
        || run.outcome != Some(checkpoint.outcome)
        || Some(checkpoint.config_sha256.as_str()) != config_sha
        || checkpoint.sitemap_ledger.as_ref() != Some(summary)
        || u64::try_from(checkpoint.completed)? != completed
        || u64::try_from(checkpoint.pending)? != pending
        || u64::try_from(checkpoint.discovery_pending)? != discovery_pending
        || checkpoint.discovery_complete != discovery_complete
    {
        out.context.reason = "saved_checkpoint_context_mismatch".into();
        return Ok(PreparedContext {
            base: out,
            selected,
            generation: report_generation,
        });
    }
    if checkpoint.sitemap_source_measurements.as_ref() != source.summary() {
        out.context.reason = "saved_source_measurement_context_mismatch".into();
        return Ok(PreparedContext {
            base: out,
            selected,
            generation: report_generation,
        });
    }
    ensure!(
        sha == checkpoint.report_sha256,
        "Sitemap analysis saved report SHA differs from checkpoint"
    );
    let (valid,shape,pages,summary_completed,summary_pending): (bool,Option<String>,Option<i64>,Option<i64>,Option<i64>) = connection.query_row(
        "SELECT json_valid(value),json_type(value,'$.pages'),json_array_length(value,'$.pages'),json_extract(value,'$.summary.completed'),json_extract(value,'$.summary.pending') FROM metadata WHERE key='report'",[],
        |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
    ensure!(
        valid
            && shape.as_deref() == Some("array")
            && pages.is_some_and(|v| (0..=MAX_PAGES).contains(&v))
            && pages == Some(i64::try_from(completed)?)
            && summary_completed == pages
            && summary_pending == Some(i64::try_from(pending)?),
        "Sitemap analysis saved report envelope differs"
    );
    source.verify_report(connection)?;
    // A controlled writer has unique requested page identities. Refuse malformed
    // typed identity/anchor fields instead of silently dropping an unjoinable row.
    let (distinct,invalid,links): (i64,i64,i64) = connection.query_row(
        "SELECT count(DISTINCT json_extract(p.value,'$.url')),coalesce(sum(CASE WHEN json_type(p.value,'$.url')='text' AND json_type(p.value,'$.links')='array' THEN 0 ELSE 1 END),0),coalesce(sum(json_array_length(p.value,'$.links')),0) FROM metadata m,json_each(m.value,'$.pages') p WHERE m.key='report'",[],
        |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
    ensure!(
        Some(distinct) == pages && invalid == 0,
        "Sitemap analysis requested page identities differ"
    );
    ensure!(
        (0..=MAX_LINKS).contains(&links),
        "Sitemap analysis anchor input budget exceeded"
    );
    selected = connection
        .query_row(
            "SELECT json_type(value,'$.operation')='object' FROM metadata WHERE key='report'",
            [],
            |r| r.get::<_, Option<bool>>(0),
        )?
        .unwrap_or(false);
    out.context.available = true;
    out.context.reason = "validated_terminal_saved_report".into();
    out.context.saved_page_count = Some(completed);
    out.context.pending_page_count = Some(pending);
    out.context.observation_scope = if selected {
        "mixed_age_selected_saved_report"
    } else if checkpoint.outcome == CrawlOutcome::Completed {
        "completed_saved_report_observations"
    } else {
        "partial_terminal_saved_report_observations"
    }
    .into();
    report_generation = Some(run.generation);
    Ok(PreparedContext {
        base: out,
        selected,
        generation: report_generation,
    })
}

fn project_rows(
    connection: &Connection,
    prepared: &PreparedContext,
    coverage: &crate::SitemapCoverage,
    visible: Vec<&crate::StoredSitemapLocation>,
    membership: MembershipBasis<'_>,
    request: &SitemapMembershipAnalysisRequest,
) -> Result<SitemapMembershipAnalysis> {
    let mut out = prepared.base.clone();
    if !out.context.available {
        return Ok(out);
    }
    let selected = prepared.selected;
    let generation = prepared
        .generation
        .context("Available analysis has no generation")?;
    if request.row_limit == 0 && request.page_limit == 0 {
        out.location_findings = Some(vec![]);
        out.saved_pages = Some(vec![]);
        return Ok(out);
    }
    let mut wanted = BTreeSet::new();
    for loc in &visible {
        if loc.observed.validation_error.is_none()
            && loc.observed.loc_ordinal.is_some()
            && let Some(url) = &loc.observed.normalized_fetch_url
        {
            wanted.insert(url.clone());
        }
        if let Some(url) = &loc.admission.admitted_fetch_url {
            wanted.insert(url.clone());
        }
    }
    let mut page_window = read_page_window(connection, request, selected, generation)?;
    out.saved_pages_have_more = page_window.len() > request.page_limit;
    page_window.truncate(request.page_limit);
    for page in &page_window {
        wanted.insert(page.requested_url.clone());
    }
    let mut observations = read_target_pages(connection, &wanted, selected, generation)?;
    let inlinks = read_inlinks(connection, &wanted)?;
    let mut pending_urls = BTreeSet::new();
    for url in &wanted {
        let done: Option<i64> = connection
            .query_row("SELECT done FROM frontier WHERE url=?1", [url], |r| {
                r.get(0)
            })
            .optional()?;
        ensure!(
            done.is_none_or(|v| matches!(v, 0 | 1)),
            "Invalid target frontier state"
        );
        if done == Some(0) {
            pending_urls.insert(url.clone());
        }
    }
    out.location_findings = Some(
        visible
            .into_iter()
            .map(|loc| {
                let original = loc.observed.normalized_fetch_url.clone().filter(|_| {
                    loc.observed.validation_error.is_none() && loc.observed.loc_ordinal.is_some()
                });
                let original_observation =
                    original.as_ref().and_then(|u| observations.get(u)).cloned();
                let original_state = if original.is_none() {
                    SitemapTargetState::InvalidOrMissingOriginalLocation
                } else if let Some(page) = &original_observation {
                    page.state.clone()
                } else if original.as_ref().is_some_and(|u| pending_urls.contains(u)) {
                    SitemapTargetState::Pending
                } else {
                    SitemapTargetState::NoStoredRequestedObservation
                };
                let alias = loc
                    .admission
                    .admitted_fetch_url
                    .as_ref()
                    .filter(|u| Some(*u) != original.as_ref());
                SitemapLocationFinding {
                    sequence: loc.sequence,
                    observation_id: loc.observation_id.clone(),
                    occurrence_id: loc.occurrence_id.clone(),
                    entry_ordinal: loc.observed.entry_ordinal,
                    original_membership_url: original.clone(),
                    admitted_fetch_url: loc.admission.admitted_fetch_url.clone(),
                    admission_decision: loc.admission.decision.clone(),
                    original_state,
                    original_observation,
                    admitted_alias_observation: alias.and_then(|u| observations.get(u)).cloned(),
                    admitted_alias_pending: alias.is_some_and(|u| pending_urls.contains(u)),
                    observed_original_inlinks: original
                        .map(|u| inlinks.get(&u).cloned().unwrap_or_default()),
                }
            })
            .collect(),
    );
    let mut members = membership.counts(connection, &page_window)?;
    out.last_saved_page_url = page_window.last().map(|v| v.requested_url.clone());
    out.last_saved_page_ordinal = page_window.last().map(|v| v.report_page_ordinal);
    out.saved_pages = Some(
        page_window
            .into_iter()
            .map(|page| {
                let counts = members.remove(&page.requested_url).unwrap_or_default();
                let observed_inlinks = inlinks
                    .get(&page.requested_url)
                    .cloned()
                    .unwrap_or_default();
                let membership = if counts.occurrences > 0 {
                    SitemapSavedPageMembership::ListedOriginal
                } else if coverage.current_valid_original_location_negative_permitted {
                    SitemapSavedPageMembership::NotListedCurrentDeclaredSources
                } else if coverage.historical_valid_original_location_negative_permitted {
                    SitemapSavedPageMembership::NotListedHistoricalDeclaredSources
                } else {
                    SitemapSavedPageMembership::Unknown
                };
                observations.remove(&page.requested_url);
                let original_source_document_count = counts.document_count;
                let original_source_observation_count = counts.observation_count;
                let original_source_document_samples = counts.samples;
                SitemapSavedPageFinding {
                    listed_without_observed_nonself_inlinks: counts.occurrences > 0
                        && observed_inlinks.nonself_anchor_occurrences == 0,
                    observation: page,
                    membership,
                    original_membership_occurrences: counts.occurrences,
                    original_source_document_count,
                    original_source_observation_count,
                    source_document_samples_have_more: original_source_document_count
                        > original_source_document_samples.len() as u64,
                    original_source_document_samples,
                    admitted_alias_occurrences: counts.aliases,
                    observed_inlinks,
                }
            })
            .collect(),
    );
    Ok(out)
}

const PAGE_COLUMNS: &str = "json_extract(p.value,'$.url'),json_extract(p.value,'$.final_url'),json_extract(p.value,'$.status'),json_extract(p.value,'$.final_status'),json_extract(p.value,'$.indexable'),json_extract(p.value,'$.indexability_reasons'),json_extract(p.value,'$.blocked_by_robots'),json_extract(p.value,'$.error'),json_extract(p.value,'$.truncated'),json_extract(p.value,'$.renderer_error'),coalesce(json_array_length(p.value,'$.redirect_chain'),0)>0,json_extract(p.value,'$.fetched_at'),json_extract(m.value,'$.operation.targets'),CAST(p.key AS INTEGER),json_extract(p.value,'$.rendered'),json_extract(p.value,'$.rendered_url'),json_extract(p.value,'$.rendered_status'),coalesce(json_type(m.value,'$.config.renderer')='object',0),json_extract(p.value,'$.content_type'),json_extract(p.value,'$.pdf') IS NULL,json_extract(p.value,'$.image_metadata') IS NULL";
fn page_from_row(
    row: &rusqlite::Row<'_>,
    selected: bool,
    generation: u64,
) -> Result<SitemapTargetObservation> {
    let requested_url: String = row.get(0)?;
    let final_url: String = row.get(1)?;
    ensure!(
        canonical(&requested_url) && (final_url.is_empty() || canonical(&final_url)),
        "Invalid saved requested/final page identity"
    );
    let indexable = checked_bool(row.get(4)?)?;
    let reasons: String = row.get(5)?;
    ensure!(
        reasons.len() <= MAX_FIELD_BYTES,
        "Sitemap analysis page field input budget exceeded"
    );
    let indexability_reasons: Vec<String> = serde_json::from_str(&reasons)?;
    let error: Option<String> = row.get(7)?;
    let renderer_error: Option<String> = row.get(9)?;
    let fetched_at: String = row.get(11)?;
    let rendered_url: Option<String> = row.get(15)?;
    ensure!(
        rendered_url
            .as_ref()
            .is_none_or(|v| v.len() <= MAX_FIELD_BYTES && !v.chars().any(char::is_control)),
        "Sitemap analysis page field input budget exceeded"
    );
    let rendered_fetch_url = rendered_url
        .as_deref()
        .and_then(|v| crate::normalize_url(v, None))
        .map(|v| v.to_string());
    let content_type: String = row.get(18)?;
    ensure!(
        content_type.len() <= MAX_FIELD_BYTES,
        "Sitemap analysis page field input budget exceeded"
    );
    let media_type = content_type.split(';').next().unwrap_or("").trim();
    let html = (media_type.eq_ignore_ascii_case("text/html")
        || media_type.eq_ignore_ascii_case("application/xhtml+xml"))
        && row.get::<_, bool>(19)?
        && row.get::<_, bool>(20)?;
    let configured_renderer: bool = row.get(17)?;
    ensure!(
        error.as_ref().is_none_or(|v| v.len() <= MAX_FIELD_BYTES)
            && renderer_error
                .as_ref()
                .is_none_or(|v| v.len() <= MAX_FIELD_BYTES)
            && fetched_at.len() <= 1024,
        "Sitemap analysis page field input budget exceeded"
    );
    let mut page = SitemapTargetObservation {
        report_page_ordinal: u64::try_from(row.get::<_, i64>(13)?)?,
        requested_url,
        final_url,
        status: row.get(2)?,
        final_status: row.get(3)?,
        indexable,
        indexability_reasons,
        blocked_by_robots: checked_bool(row.get(6)?)?,
        error,
        truncated: checked_bool(row.get(8)?)?,
        renderer_error,
        rendered: checked_bool(row.get(14)?)?,
        rendered_url,
        rendered_fetch_url,
        rendered_status: row.get(16)?,
        rendering_expected: false,
        rendering_state: String::new(),
        rendered_navigation_changed: false,
        redirect_chain_present: row.get(10)?,
        fetched_at,
        age_scope: "captured_checkpoint_history".into(),
        state: SitemapTargetState::NoResponse,
    };
    if selected {
        // Query just the matching target's generation, not the full target list
        // or any parent PageRecord payload.
        let committed: Option<i64> = row.get(12)?;
        page.age_scope = match committed {
            Some(value) if u64::try_from(value)? == generation => "selected_current_generation",
            Some(_) => "selected_prior_generation",
            None => "inherited_parent_observation",
        }
        .into();
    }
    page.rendering_expected = configured_renderer
        && html
        && page.final_status == Some(200)
        && page.error.is_none()
        && !page.truncated
        && !page.blocked_by_robots;
    page.rendered_navigation_changed = page.rendered
        && page
            .rendered_fetch_url
            .as_ref()
            .is_some_and(|url| url != &page.final_url);
    page.rendering_state = if page.renderer_error.is_some() {
        "failed"
    } else if page.rendered && page.rendered_url.is_none() {
        "url_unobserved"
    } else if page.rendered && page.rendered_fetch_url.is_none() {
        "url_unusable"
    } else if page.rendered && page.rendered_status.is_none() {
        "response_unobserved"
    } else if page.rendered && page.rendered_status != Some(200) {
        "non200"
    } else if page.rendered_navigation_changed {
        "navigation_changed"
    } else if page.rendered {
        "observed_200"
    } else if page.rendering_expected {
        "required_but_unobserved"
    } else if configured_renderer {
        "not_applicable"
    } else {
        "not_configured"
    }
    .into();
    page.state = if page.blocked_by_robots {
        SitemapTargetState::BlockedByRobots
    } else if page.redirect_chain_present
        || page.status.is_some_and(|s| (300..400).contains(&s))
        || (!page.final_url.is_empty() && page.final_url != page.requested_url)
    {
        SitemapTargetState::Redirected
    } else if page.final_status.or(page.status).is_some_and(|s| s != 200) {
        SitemapTargetState::Non200
    } else if page.error.is_some() {
        SitemapTargetState::FetchFailed
    } else if page.truncated || page.renderer_error.is_some() {
        SitemapTargetState::IncompleteObservation
    } else if page.rendered_navigation_changed {
        SitemapTargetState::RenderedNavigationChanged
    } else if page.rendered && page.rendered_status.is_some_and(|s| s != 200) {
        SitemapTargetState::RenderedNon200
    } else if (page.rendered
        && (page.rendered_status.is_none() || page.rendered_fetch_url.is_none()))
        || (page.rendering_expected && !page.rendered)
    {
        SitemapTargetState::IncompleteObservation
    } else if page.final_status.or(page.status).is_none() {
        SitemapTargetState::NoResponse
    } else if !page.indexable {
        SitemapTargetState::NonIndexable
    } else {
        SitemapTargetState::Indexable
    };
    Ok(page)
}
fn checked_bool(value: i64) -> Result<bool> {
    ensure!(matches!(value, 0 | 1), "Invalid saved boolean");
    Ok(value == 1)
}
fn page_columns() -> String {
    PAGE_COLUMNS.replace("json_extract(m.value,'$.operation.targets')",
        "(SELECT json_extract(t.value,'$.committed_generation') FROM json_each(m.value,'$.operation.targets') t WHERE json_extract(t.value,'$.requested_url')=json_extract(p.value,'$.url'))")
}
fn read_page_window(
    connection: &Connection,
    request: &SitemapMembershipAnalysisRequest,
    selected: bool,
    generation: u64,
) -> Result<Vec<SitemapTargetObservation>> {
    if request.page_limit == 0 {
        return Ok(vec![]);
    }
    let sql = format!(
        "SELECT {} FROM metadata m,json_each(m.value,'$.pages') p WHERE m.key='report' AND (?1 IS NULL OR CAST(p.key AS INTEGER)>?1) ORDER BY CAST(p.key AS INTEGER) LIMIT ?2",
        page_columns()
    );
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query(params![
        request.after_page_ordinal.map(i64::try_from).transpose()?,
        i64::try_from(request.page_limit + 1)?
    ])?;
    let mut pages = vec![];
    while let Some(row) = rows.next()? {
        pages.push(page_from_row(row, selected, generation)?);
    }
    Ok(pages)
}
fn read_target_pages(
    connection: &Connection,
    wanted: &BTreeSet<String>,
    selected: bool,
    generation: u64,
) -> Result<BTreeMap<String, SitemapTargetObservation>> {
    let mut pages = BTreeMap::new();
    if wanted.is_empty() {
        return Ok(pages);
    }
    let keys = serde_json::to_string(wanted)?;
    let sql = format!(
        "SELECT {} FROM metadata m,json_each(m.value,'$.pages') p WHERE m.key='report' AND json_extract(p.value,'$.url') IN (SELECT value FROM json_each(?1))",
        page_columns()
    );
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query([keys])?;
    while let Some(row) = rows.next()? {
        let page = page_from_row(row, selected, generation)?;
        pages.insert(page.requested_url.clone(), page);
    }
    Ok(pages)
}
fn read_inlinks(
    connection: &Connection,
    wanted: &BTreeSet<String>,
) -> Result<BTreeMap<String, SitemapObservedInlinks>> {
    let mut out = BTreeMap::new();
    if wanted.is_empty() {
        return Ok(out);
    }
    let keys = serde_json::to_string(wanted)?;
    let from = "FROM metadata m,json_each(m.value,'$.pages') p,json_each(p.value,'$.links') l WHERE m.key='report' AND json_extract(l.value,'$.kind')='anchor' AND json_extract(l.value,'$.url') IN (SELECT value FROM json_each(?1))";
    let sql = format!(
        "SELECT json_extract(l.value,'$.url'),count(*),sum(json_extract(p.value,'$.url')<>json_extract(l.value,'$.url')),sum(json_extract(p.value,'$.url')<>json_extract(l.value,'$.url') AND json_extract(l.value,'$.internal')=1),count(DISTINCT CASE WHEN json_extract(p.value,'$.url')<>json_extract(l.value,'$.url') THEN json_extract(p.value,'$.url') END) {from} GROUP BY json_extract(l.value,'$.url')"
    );
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query([&keys])?;
    while let Some(row) = rows.next()? {
        let url: String = row.get(0)?;
        let mut value = SitemapObservedInlinks {
            anchor_occurrences: u64::try_from(row.get::<_, i64>(1)?)?,
            nonself_anchor_occurrences: u64::try_from(row.get::<_, i64>(2)?)?,
            internal_nonself_anchor_occurrences: u64::try_from(row.get::<_, i64>(3)?)?,
            distinct_nonself_requested_sources: u64::try_from(row.get::<_, i64>(4)?)?,
            ..Default::default()
        };
        value.samples_have_more = value.anchor_occurrences > u64::try_from(SAMPLE_LIMIT)?;
        out.insert(url, value);
    }
    let sql = format!(
        "SELECT source,target,page_ordinal,link_ordinal,text,rel,internal,nofollow FROM (SELECT json_extract(p.value,'$.url') source,json_extract(l.value,'$.url') target,CAST(p.key AS INTEGER) page_ordinal,CAST(l.key AS INTEGER) link_ordinal,json_extract(l.value,'$.text') text,json_extract(l.value,'$.rel') rel,json_extract(l.value,'$.internal') internal,json_extract(l.value,'$.nofollow') nofollow,row_number() OVER (PARTITION BY json_extract(l.value,'$.url') ORDER BY json_extract(p.value,'$.url') COLLATE BINARY,CAST(p.key AS INTEGER),CAST(l.key AS INTEGER)) rank {from}) WHERE rank<=?2 ORDER BY target,rank"
    );
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query(params![keys, i64::try_from(SAMPLE_LIMIT)?])?;
    while let Some(row) = rows.next()? {
        let sample = SitemapInlinkSample {
            source_requested_url: row.get(0)?,
            target_requested_url: row.get(1)?,
            source_page_ordinal: u64::try_from(row.get::<_, i64>(2)?)?,
            link_ordinal: u64::try_from(row.get::<_, i64>(3)?)?,
            text: row.get(4)?,
            rel: row.get(5)?,
            internal: checked_bool(row.get(6)?)?,
            nofollow: checked_bool(row.get(7)?)?,
        };
        ensure!(
            sample.text.len() <= MAX_FIELD_BYTES
                && sample.rel.len() <= MAX_FIELD_BYTES
                && canonical(&sample.source_requested_url),
            "Sitemap analysis anchor field input budget exceeded"
        );
        out.get_mut(&sample.target_requested_url)
            .context("Missing anchor count group")?
            .samples
            .push(sample);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "sitemap_membership_analysis_tests.rs"]
mod tests;
