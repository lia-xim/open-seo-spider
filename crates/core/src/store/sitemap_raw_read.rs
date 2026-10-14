//! Bounded raw rows and verified source summary from one read transaction.
//!
//! This is deliberately a separate raw reader. Coverage, saved-page analysis
//! and their richer App cursors retain their existing APIs and proof boundary.
use super::*;
use crate::sitemap_ledger::verified_read::{SitemapLedgerFilter, SitemapLedgerRowKind, verify_raw};
use crate::sitemap_source_store::verified_read::verify_source;
use anyhow::ensure;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

pub const SITEMAP_RAW_READ_ENGINE: &str = "sitemap-verified-raw-window-v1";
const MAX_WINDOW_JSON_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapRawRowKind {
    #[default]
    All,
    Root,
    Document,
    Entry,
    Location,
}

/// Exact original source/member identities. Continuations must carry the state
/// returned by this raw reader, which has a separate profile from App cursors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SitemapRawReadRequest {
    pub kind: SitemapRawRowKind,
    pub source_observation_id: Option<String>,
    pub membership_url: Option<String>,
    pub after_sequence: Option<u64>,
    pub limit: usize,
    pub expected_state_sha256: Option<String>,
}
impl Default for SitemapRawReadRequest {
    fn default() -> Self {
        Self {
            kind: SitemapRawRowKind::All,
            source_observation_id: None,
            membership_url: None,
            after_sequence: None,
            limit: 100,
            expected_state_sha256: None,
        }
    }
}
impl SitemapRawReadRequest {
    fn filter(&self) -> SitemapLedgerFilter {
        SitemapLedgerFilter {
            kind: match self.kind {
                SitemapRawRowKind::All => SitemapLedgerRowKind::All,
                SitemapRawRowKind::Root => SitemapLedgerRowKind::Root,
                SitemapRawRowKind::Document => SitemapLedgerRowKind::Document,
                SitemapRawRowKind::Entry => SitemapLedgerRowKind::Entry,
                SitemapRawRowKind::Location => SitemapLedgerRowKind::Location,
            },
            source_observation_id: self.source_observation_id.clone(),
            membership_url: self.membership_url.clone(),
        }
    }
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            (1..=1000).contains(&self.limit),
            "Raw sitemap limit must be 1..=1000"
        );
        self.filter().validate()?;
        ensure!(
            self.after_sequence != Some(0),
            "Raw sitemap continuation must be nonzero"
        );
        ensure!(
            self.after_sequence.is_none() || self.expected_state_sha256.is_some(),
            "Raw sitemap continuation requires verified state"
        );
        if let Some(state) = &self.expected_state_sha256 {
            ensure!(lowercase_sha(state), "Invalid raw sitemap state SHA-256");
        }
        Ok(())
    }
}

/// Verified raw/source context only. A missing ledger is unavailable history,
/// while an available ledger with no rows is a known empty raw catalog.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SitemapRawReadContext {
    pub schema_version: u32,
    pub engine: String,
    pub scope: String,
    pub crawl_config_sha256: Option<String>,
    pub generation: Option<u64>,
    pub ledger: Option<crate::SitemapLedgerSummary>,
    pub inherited: Option<crate::SitemapInheritedBasis>,
    pub source_summary: Option<crate::SitemapSourceStoreSummary>,
    pub source_context_sha256: String,
    /// The existing selected-plan/parent validators still materialize bounded
    /// parent data. True must never be described as a fully streaming read.
    pub materializing_selected_compatibility: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SitemapRawReadWindow {
    pub context: SitemapRawReadContext,
    pub state_sha256: String,
    pub rows: Vec<crate::SitemapLedgerRow>,
    pub next_after_sequence: Option<u64>,
    pub has_more: bool,
}

#[derive(Serialize)]
struct FilterBinding<'a> {
    kind: SitemapRawRowKind,
    source_observation_id: &'a Option<String>,
    membership_url: &'a Option<String>,
}
fn state_sha(
    request: &SitemapRawReadRequest,
    context: &SitemapRawReadContext,
) -> anyhow::Result<String> {
    // Not self-referential. Page position/size do not alter the captured state;
    // changing the filter always invalidates a continuation under this profile.
    Ok(crate::sitemap_ledger::digest(serde_json::to_vec(&(
        SITEMAP_RAW_READ_ENGINE,
        1_u32,
        FilterBinding {
            kind: request.kind,
            source_observation_id: &request.source_observation_id,
            membership_url: &request.membership_url,
        },
        context,
    ))?))
}
fn lowercase_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn saved_config_sha(connection: &Connection) -> anyhow::Result<Option<String>> {
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
        "Saved raw-read config exceeds bounded input"
    );
    let bytes: Vec<u8> = connection.query_row(
        "SELECT CAST(value AS BLOB) FROM metadata WHERE key='config'",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        i64::try_from(bytes.len())? == length,
        "Saved raw-read config byte length differs"
    );
    let config: CrawlConfig = serde_json::from_slice(&bytes)?;
    verify_sitemap_config_context(connection, &config)?;
    Ok(Some(crate::sitemap_ledger::digest(&bytes)))
}

pub(super) fn preflight_context_metadata(connection: &Connection) -> anyhow::Result<()> {
    // Existing plan validators remain authoritative after this additive input
    // bound. In particular a small parent does not bound a mutable plan value.
    for (key, cap) in [
        ("run_state", 65_536_i64),
        ("operation_kind", 65_536_i64),
        ("discovery_initialized", 65_536_i64),
        ("discovery_complete", 65_536_i64),
        ("run_plan", 268_435_456_i64),
    ] {
        let length: Option<i64> = connection
            .query_row(
                "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key=?1",
                [key],
                |row| row.get(0),
            )
            .optional()?;
        ensure!(
            length.is_none_or(|length| (0..=cap).contains(&length)),
            "Raw sitemap context metadata {key} exceeds bounded input"
        );
    }
    Ok(())
}

struct OutputSize(usize);
impl Write for OutputSize {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let size = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("Raw window size overflow"))?;
        if size > MAX_WINDOW_JSON_BYTES {
            return Err(io::Error::other("Raw sitemap window JSON budget exceeded"));
        }
        self.0 = size;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl CrawlStore {
    /// Verify the complete raw ledger and source sidecar before returning a
    /// bounded window. No schema migration or website request is performed.
    /// Ordinary reads retain compact document counters and one raw/source row;
    /// config validation and selected-parent compatibility have separate costs.
    /// This method does not project coverage or terminal saved-page analysis.
    pub fn sitemap_raw_window(
        &self,
        request: &SitemapRawReadRequest,
    ) -> anyhow::Result<SitemapRawReadWindow> {
        request.validate()?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        preflight_context_metadata(&transaction)?;
        let crawl_config_sha256 = saved_config_sha(&transaction)?;
        let plan = load_run_plan(&transaction)?;
        let selected = matches!(plan, RunPlan::SelectedRecrawl(_));
        if selected {
            // Preserve full immutable-parent reconstruction and both presence
            // guards until their separate streaming replacement is proved.
            load_sitemap_catalog_verified(&transaction)?;
            load_sitemap_source_measurements_verified(&transaction)?;
        }
        let raw = verify_raw(&transaction)?;
        let source = verify_source(&transaction, raw.as_ref())?;
        if let Some(source) = &source {
            ensure!(
                std::ptr::eq(source.connection(), &*transaction),
                "Source read transaction differs"
            );
        }
        let source_context_sha256 = source
            .as_ref()
            .map(|value| value.context().to_owned())
            .unwrap_or(crate::sitemap_source_store::snapshot_context(None)?);
        let context = SitemapRawReadContext {
            schema_version: 1,
            engine: SITEMAP_RAW_READ_ENGINE.into(),
            scope: "raw_rows_and_source_measurements_only".into(),
            crawl_config_sha256,
            generation: metadata::<CrawlRunState>(&transaction, "run_state")?
                .map(|run| run.generation)
                .filter(|generation| *generation > 0),
            ledger: raw.as_ref().map(|value| value.summary().clone()),
            inherited: raw.as_ref().and_then(|value| value.inherited().cloned()),
            source_summary: source.as_ref().map(|value| value.summary().clone()),
            source_context_sha256,
            materializing_selected_compatibility: selected
                || source
                    .as_ref()
                    .is_some_and(|value| value.materializing_selected_compatibility()),
        };
        let state_sha256 = state_sha(request, &context)?;
        if let Some(expected) = &request.expected_state_sha256 {
            ensure!(
                expected == &state_sha256,
                "Raw sitemap state stale: restart pagination"
            );
        }
        let (rows, next_after_sequence, has_more) = match raw.as_ref() {
            Some(raw) => {
                let page = raw.window(&request.filter(), request.after_sequence, request.limit)?;
                (page.rows, page.next_after_sequence, page.has_more)
            }
            None => {
                ensure!(
                    request.after_sequence.is_none(),
                    "Unavailable raw history has no continuation"
                );
                (Vec::new(), None, false)
            }
        };
        let window = SitemapRawReadWindow {
            context,
            state_sha256,
            rows,
            next_after_sequence,
            has_more,
        };
        serde_json::to_writer(&mut OutputSize(0), &window)?;
        drop(source);
        drop(raw);
        transaction.commit()?;
        Ok(window)
    }
}

#[cfg(test)]
#[path = "sitemap_raw_read_tests.rs"]
mod tests;
