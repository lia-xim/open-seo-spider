//! Separate saved-ledger scope. No entity parsing, crawling, anchor backfill or IO.
use crate::{
    CrawlConfig, CrawlRunState, ResourceLedgerSummary, ResourceObservationBasis,
    ResourceReadSnapshot, ResourceResponseFacts, ResourceTransportObservation,
    StoredResourceOccurrence,
};
use anyhow::{Context, Result, ensure};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
};

pub const RESOURCE_BROKEN_SOURCES_SCOPE: &str = "verified_resource_ledger_occurrences";
const ENGINE: &str = "verified_resource_broken_sources_v1";
const JSON_CAP: usize = 8 * 1024 * 1024;
const CSV_CAP: usize = 64 * 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceBrokenSourcesLimits {
    pub max_evaluation_rows: usize,
    pub max_transports: usize,
    pub max_column_utf8_bytes: usize,
    pub max_row_utf8_bytes: usize,
}
impl Default for ResourceBrokenSourcesLimits {
    fn default() -> Self {
        Self {
            max_evaluation_rows: 250_000,
            max_transports: 50_000,
            max_column_utf8_bytes: 256 * 1024 * 1024,
            max_row_utf8_bytes: 32 * 1024 * 1024,
        }
    }
}
impl ResourceBrokenSourcesLimits {
    fn validate(&self) -> Result<()> {
        for (name, value, upper) in [
            ("evaluation rows", self.max_evaluation_rows, 250_000),
            ("transports", self.max_transports, 50_000),
            (
                "table UTF8 bytes",
                self.max_column_utf8_bytes,
                256 * 1024 * 1024,
            ),
            ("row UTF8 bytes", self.max_row_utf8_bytes, 32 * 1024 * 1024),
        ] {
            ensure!(
                value > 0 && value <= upper,
                "Resource broken-sources {name} limit outside 1..={upper}"
            );
        }
        Ok(())
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceBrokenSourcesFilter {
    #[default]
    All,
    Failures,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceBrokenSourcesRequest {
    pub filter: ResourceBrokenSourcesFilter,
    pub limit: usize,
    pub after_sequence: Option<u64>,
    pub expected_snapshot_token: Option<String>,
    pub limits: ResourceBrokenSourcesLimits,
}
impl Default for ResourceBrokenSourcesRequest {
    fn default() -> Self {
        Self {
            filter: Default::default(),
            limit: 100,
            after_sequence: None,
            expected_snapshot_token: None,
            limits: Default::default(),
        }
    }
}
impl ResourceBrokenSourcesRequest {
    pub(crate) fn validate(&self) -> Result<()> {
        self.limits.validate()?;
        ensure!(
            (1..=200).contains(&self.limit),
            "Resource broken-sources page limit must be 1..=200"
        );
        ensure!(
            self.expected_snapshot_token
                .as_ref()
                .is_none_or(|v| v.len() == 64 && v.bytes().all(|c| c.is_ascii_hexdigit())),
            "Invalid resource broken-sources snapshot token"
        );
        ensure!(
            self.after_sequence.is_none() || self.expected_snapshot_token.is_some(),
            "Continuation needs a snapshot token"
        );
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceBrokenSourcesState {
    ObservedHttpFailure,
    ObservedNonFailure,
    NoGrant,
    NotFetched,
    InvalidReference,
    IncompleteSource,
    InheritedObservation,
    RobotsBlocked,
    StoredFetchError,
    UnobservedFinal,
    InvalidTransport,
    NonterminalFinalStatus,
}
impl ResourceBrokenSourcesFilter {
    fn includes(self, state: ResourceBrokenSourcesState) -> bool {
        match self {
            Self::All => true,
            Self::Failures => state == ResourceBrokenSourcesState::ObservedHttpFailure,
            Self::Unknown => !matches!(
                state,
                ResourceBrokenSourcesState::ObservedHttpFailure
                    | ResourceBrokenSourcesState::ObservedNonFailure
            ),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct ResourceBrokenSourcesTransport {
    pub requested_url: String,
    pub observed_generation: u64,
    pub retained_response: bool,
    pub payload_sha256: String,
    pub initial_status: Option<u16>,
    pub final_status: Option<u16>,
    pub final_url: String,
    pub fetched_at: String,
    pub error: Option<String>,
    pub blocked_by_robots: bool,
    pub redirect_chain: Vec<crate::RedirectHop>,
    pub response_facts: Option<ResourceResponseFacts>,
}
impl ResourceBrokenSourcesTransport {
    fn from_observation(o: ResourceTransportObservation, sha: String) -> Self {
        let p = o.projection;
        Self {
            requested_url: o.requested_url,
            observed_generation: o.observed_generation,
            retained_response: o.retained_response,
            payload_sha256: sha,
            initial_status: p.status,
            final_status: p.final_status,
            final_url: p.final_url,
            fetched_at: p.fetched_at,
            error: p.error,
            blocked_by_robots: p.blocked_by_robots,
            redirect_chain: p.redirect_chain,
            response_facts: p.resource_response,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct ResourceBrokenSourcesRow {
    /// A decision evaluation, not a newly discovered physical source occurrence.
    pub observation: StoredResourceOccurrence,
    pub source_transport: Option<ResourceBrokenSourcesTransport>,
    pub grant_fetch_url: Option<String>,
    pub target_transport: Option<ResourceBrokenSourcesTransport>,
    pub state: ResourceBrokenSourcesState,
    pub reason: &'static str,
    pub anchor_text: Option<String>,
    pub anchor_text_state: &'static str,
}
#[derive(Debug, Clone, Serialize)]
pub struct ResourceBrokenSourcesSnapshot {
    pub engine: &'static str,
    pub scope: &'static str,
    pub saved: ResourceReadSnapshot,
    pub ledger: ResourceLedgerSummary,
    pub transport_set_sha256: String,
    pub transport_rows: usize,
    pub transport_utf8_bytes: usize,
    pub evaluation_rows: usize,
    pub distinct_stored_occurrence_ids: usize,
    pub filter: ResourceBrokenSourcesFilter,
    pub page_limit: usize,
    pub limits: ResourceBrokenSourcesLimits,
    pub snapshot_token: String,
    pub website_requests: usize,
    pub current_website_freshness_attested: bool,
    pub source_kind_coverage_complete: bool,
    pub precision_gaps: Vec<&'static str>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ResourceBrokenSourcesPage {
    pub scope: &'static str,
    pub available: bool,
    pub unavailable_reason: Option<&'static str>,
    pub snapshot: Option<ResourceBrokenSourcesSnapshot>,
    pub rows: Vec<ResourceBrokenSourcesRow>,
    pub selected_evaluation_rows: usize,
    pub selected_distinct_occurrence_ids: usize,
    pub counts: BTreeMap<ResourceBrokenSourcesState, usize>,
    pub next_sequence: Option<u64>,
    pub has_more: bool,
    pub website_requests: usize,
}
#[derive(Debug)]
pub struct ResourceBrokenSourcesCsv {
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub snapshot: ResourceBrokenSourcesSnapshot,
    pub exported_evaluation_rows: usize,
    pub distinct_exported_occurrence_ids: usize,
}
pub(crate) enum Output {
    Page(ResourceBrokenSourcesPage),
    Csv(ResourceBrokenSourcesCsv),
}
#[derive(Clone, Copy)]
pub(crate) enum Mode {
    Page,
    Csv,
}

fn exists(c: &Connection, name: &str) -> Result<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [name],
        |r| r.get(0),
    )?)
}
fn metadata_text(c: &Connection, key: &str, cap: usize) -> Result<Option<String>> {
    let count: i64 = c.query_row("SELECT count(*) FROM metadata WHERE key=?1", [key], |r| {
        r.get(0)
    })?;
    ensure!(count <= 1, "Duplicate saved metadata key: {key}");
    if count == 0 {
        return Ok(None);
    }
    let (kind, bytes): (String, i64) = c.query_row(
        "SELECT typeof(value),length(CAST(value AS BLOB)) FROM metadata WHERE key=?1",
        [key],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    ensure!(
        kind == "text" && bytes >= 0 && usize::try_from(bytes)? <= cap.saturating_mul(2),
        "Saved {key} storage size/type exceeds resource-read admission"
    );
    let mut statement = c.prepare("SELECT value FROM metadata WHERE key=?1")?;
    let mut rows = statement.query([key])?;
    let row = rows
        .next()?
        .context("Saved metadata vanished inside read snapshot")?;
    let text = row.get_ref(0)?.as_str()?;
    ensure!(
        text.len() <= cap,
        "Saved {key} UTF8 exceeds resource-read admission"
    );
    Ok(Some(text.to_owned()))
}
// Preflight never changes the old verifier. Its fixed native/full-parent rules
// still decide consistency; these are extra limits for this new optional read.
// The BLOB guard is <=2x only for SQLite UTF16 storage. Actual borrowed UTF8 is
// checked independently before a Rust String copy. No total RSS claim is made.
pub(crate) fn preflight(
    c: &Connection,
    r: &ResourceBrokenSourcesRequest,
) -> Result<Option<CrawlConfig>> {
    r.validate()?;
    let cfg = metadata_text(c, "config", 8 * 1024 * 1024)?
        .map(|s| serde_json::from_str::<CrawlConfig>(&s))
        .transpose()?;
    for (key, cap) in [
        ("run_state", 64 * 1024),
        ("run_plan", 8 * 1024 * 1024),
        ("resource_ledger", 1024 * 1024),
        ("discovery_complete", 16),
        ("operation_kind", 256),
    ] {
        metadata_text(c, key, cap)?;
    }
    let required = [
        "resource_occurrences",
        "resource_grants",
        "resource_tasks",
        "resource_transports",
        "resource_expansions",
        "resource_sources",
    ];
    let mut found = 0;
    for name in required {
        found += usize::from(exists(c, name)?);
    }
    ensure!(
        found == 0 || found == required.len(),
        "Partial native resource schema is unavailable/inconsistent"
    );
    if found == 0 {
        ensure!(
            metadata_text(c, "resource_ledger", 1024 * 1024)?.is_none(),
            "Resource summary exists without native tables"
        );
        return Ok(cfg);
    }
    for (table, key, cap, rows) in [
        (
            "resource_occurrences",
            "occurrence_id",
            256,
            r.limits.max_evaluation_rows,
        ),
        (
            "resource_grants",
            "occurrence_id",
            256,
            r.limits.max_evaluation_rows,
        ),
        ("resource_tasks", "url", 16 * 1024, r.limits.max_transports),
        (
            "resource_transports",
            "url",
            16 * 1024,
            r.limits.max_transports,
        ),
        (
            "resource_expansions",
            "url",
            16 * 1024,
            r.limits.max_transports,
        ),
        ("resource_sources", "revision", 256, r.limits.max_transports),
    ] {
        column(c, table, key, cap, r.limits.max_column_utf8_bytes, rows)?;
    }
    for table in [
        "resource_occurrences",
        "resource_grants",
        "resource_tasks",
        "resource_transports",
    ] {
        column(
            c,
            table,
            "data",
            r.limits.max_row_utf8_bytes,
            r.limits.max_column_utf8_bytes,
            if table == "resource_occurrences" || table == "resource_grants" {
                r.limits.max_evaluation_rows
            } else {
                r.limits.max_transports
            },
        )?;
    }
    column(
        c,
        "resource_expansions",
        "processed_revision",
        256,
        r.limits.max_column_utf8_bytes,
        r.limits.max_transports,
    )?;
    column(
        c,
        "resource_occurrences",
        "evaluation",
        256,
        r.limits.max_column_utf8_bytes,
        r.limits.max_evaluation_rows,
    )?;
    column(
        c,
        "resource_transports",
        "payload_sha256",
        64,
        r.limits.max_column_utf8_bytes,
        r.limits.max_transports,
    )?;
    for (table, key) in [
        ("resource_grants", "occurrence_id"),
        ("resource_tasks", "url"),
        ("resource_transports", "url"),
        ("resource_expansions", "url"),
        ("resource_sources", "revision"),
    ] {
        let sql = format!("SELECT count(*)=count(DISTINCT {key} COLLATE BINARY) FROM {table}");
        ensure!(
            c.query_row(&sql, [], |r| r.get::<_, bool>(0))?,
            "Duplicate resource index identity: {table}"
        );
    }
    ensure!(!c.query_row("SELECT EXISTS(SELECT 1 FROM resource_occurrences GROUP BY occurrence_id COLLATE BINARY,evaluation COLLATE BINARY HAVING count(*)>1)",[],|r|r.get::<_,bool>(0))?,"Duplicate occurrence/evaluation identity");
    if exists(c, "recrawl_base")? {
        let count: i64 = c.query_row("SELECT count(*) FROM recrawl_base", [], |r| r.get(0))?;
        ensure!(count <= 1, "Multiple immutable parent snapshots");
        if count > 0 {
            let bytes: i64 = c.query_row(
                "SELECT length(snapshot) FROM recrawl_base WHERE id=1",
                [],
                |r| r.get(0),
            )?;
            ensure!(
                (0..=268435456).contains(&bytes),
                "Immutable parent exceeds existing256MiB guard"
            );
        }
    }
    if exists(c, "pages")? {
        column(
            c,
            "pages",
            "data",
            r.limits.max_row_utf8_bytes,
            r.limits.max_column_utf8_bytes,
            r.limits.max_transports,
        )?;
    }
    // Selected-parent verifier additionally reads these short identities. It
    // retains its own original SHA/generation/parent authority checks.
    for (table, col) in [
        ("recrawl_pages", "payload_sha256"),
        ("recrawl_targets", "url"),
    ] {
        if exists(c, table)? {
            column(
                c,
                table,
                col,
                if col == "url" { 16 * 1024 } else { 64 },
                r.limits.max_column_utf8_bytes,
                r.limits.max_transports,
            )?;
        }
    }
    let native = cfg.as_ref().is_some_and(|c| c.resource_discovery.is_some());
    if !native || metadata_text(c, "resource_ledger", 1024 * 1024)?.is_none() {
        for table in required {
            let count: i64 =
                c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?;
            ensure!(
                count == 0,
                "Resource history exists without initialized enabled native config/summary"
            );
        }
    }
    Ok(cfg)
}
fn column(
    c: &Connection,
    table: &str,
    col: &str,
    row_cap: usize,
    total_cap: usize,
    row_count_cap: usize,
) -> Result<()> {
    // All identifiers are fixed internal callsites, never caller strings.
    let sql = format!(
        "SELECT count(*),coalesce(max(length(CAST({col} AS BLOB))),0),coalesce(sum(length(CAST({col} AS BLOB))),0),coalesce(sum(typeof({col})!='text'),0) FROM {table}"
    );
    let (n, max, total, bad): (i64, i64, i64, i64) = c.query_row(&sql, [], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    })?;
    ensure!(
        n >= 0
            && usize::try_from(n)? <= row_count_cap
            && max >= 0
            && usize::try_from(max)? <= row_cap.saturating_mul(2)
            && total >= 0
            && usize::try_from(total)? <= total_cap.saturating_mul(2)
            && bad == 0,
        "Resource {table}.{col} row/type/storage byte budget exceeded"
    );
    let mut stmt = c.prepare(&format!("SELECT {col} FROM {table}"))?;
    let mut rows = stmt.query([])?;
    let mut utf8 = 0usize;
    while let Some(r) = rows.next()? {
        let s = r.get_ref(0)?.as_str()?;
        ensure!(
            s.len() <= row_cap,
            "Resource {table}.{col} UTF8 row budget exceeded"
        );
        utf8 = utf8
            .checked_add(s.len())
            .filter(|n| *n <= total_cap)
            .context("Resource UTF8 table budget exceeded")?;
    }
    Ok(())
}
pub(crate) fn verification_required(c: &Connection, cfg: &CrawlConfig) -> Result<bool> {
    Ok(cfg.resource_discovery.is_some()
        && exists(c, "resource_occurrences")?
        && metadata_text(c, "resource_ledger", 1024 * 1024)?.is_some())
}
fn framed(hash: &mut Sha256, value: &[u8]) {
    hash.update((value.len() as u64).to_le_bytes());
    hash.update(value)
}
fn hash_json(value: &impl Serialize, cap: usize) -> Result<(String, usize)> {
    let mut h = HashWriter {
        hash: Sha256::new(),
        bytes: 0,
        cap,
    };
    serde_json::to_writer(&mut h, value)?;
    Ok((format!("{:x}", h.hash.finalize()), h.bytes))
}
struct HashWriter {
    hash: Sha256,
    bytes: usize,
    cap: usize,
}
impl Write for HashWriter {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(b.len())
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| io::Error::other("Resource broken-sources serialization limit"))?;
        self.hash.update(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn text(r: &rusqlite::Row<'_>, index: usize, cap: usize) -> Result<String> {
    let s = r.get_ref(index)?.as_str()?;
    ensure!(
        s.len() <= cap,
        "Resource joined value exceeds UTF8 row admission"
    );
    Ok(s.to_owned())
}
fn transport(
    r: &rusqlite::Row<'_>,
    data: usize,
    sha: usize,
    cap: usize,
) -> Result<Option<ResourceBrokenSourcesTransport>> {
    if r.get_ref(data)?.data_type() == rusqlite::types::Type::Null {
        return Ok(None);
    }
    let raw = text(r, data, cap)?;
    let expected = text(r, sha, 64)?;
    ensure!(
        crate::resource_policy::digest(&raw) == expected,
        "Joined private transport SHA differs"
    );
    let o: ResourceTransportObservation = serde_json::from_str(&raw)?;
    ensure!(
        o.engine == crate::RESOURCE_DISCOVERY_ENGINE,
        "Joined transport engine changed"
    );
    Ok(Some(ResourceBrokenSourcesTransport::from_observation(
        o, expected,
    )))
}
fn coherent(t: &ResourceBrokenSourcesTransport) -> bool {
    let (Some(initial), Some(final_status)) = (t.initial_status, t.final_status) else {
        return false;
    };
    if !(100..=599).contains(&initial) || !(100..=599).contains(&final_status) {
        return false;
    }
    let Some(mut at) = crate::normalize_url(&t.requested_url, None) else {
        return false;
    };
    let Some(final_url) = crate::normalize_url(&t.final_url, None) else {
        return false;
    };
    let mut seen = BTreeSet::from([at.to_string()]);
    for (i, h) in t.redirect_chain.iter().enumerate() {
        if ![301, 302, 303, 307, 308].contains(&h.status)
            || crate::normalize_url(&h.url, None).as_ref() != Some(&at)
            || i == 0 && h.status != initial
        {
            return false;
        }
        let Some(next) = crate::normalize_url(&h.target, None) else {
            return false;
        };
        if !seen.insert(next.to_string()) {
            return false;
        }
        at = next;
    }
    at == final_url && (!t.redirect_chain.is_empty() || initial == final_status)
}
fn state(
    o: &StoredResourceOccurrence,
    target: Option<&ResourceBrokenSourcesTransport>,
) -> (ResourceBrokenSourcesState, &'static str) {
    use ResourceBrokenSourcesState as S;
    if o.occurrence.observation_error.is_some()
        || o.occurrence
            .resolved_url
            .as_deref()
            .is_none_or(|s| crate::normalize_url(s, None).is_none())
    {
        return (S::InvalidReference, "stored_reference_invalid_or_non_http");
    }
    if o.basis == ResourceObservationBasis::Inherited {
        return (
            S::InheritedObservation,
            "inherited_occurrence_not_a_fresh_source_observation",
        );
    }
    if !o.occurrence.source_complete {
        return (S::IncompleteSource, "source_unit_incomplete");
    }
    let Some(g) = &o.decision.grant else {
        return (S::NoGrant, "this_evaluation_has_no_fetch_grant");
    };
    if crate::normalize_url(&g.fetch_url, None).is_none() {
        return (S::InvalidReference, "invalid_granted_fetch_identity");
    }
    let Some(t) = target else {
        return (
            S::NotFetched,
            "no_saved_transport_for_this_grant_fetch_identity",
        );
    };
    if t.requested_url != g.fetch_url {
        return (S::InvalidTransport, "target_transport_does_not_match_grant");
    }
    if t.blocked_by_robots {
        return (S::RobotsBlocked, "stored_target_robots_denial");
    }
    if t.error.is_some() {
        return (S::StoredFetchError, "stored_target_fetch_error");
    }
    if t.final_status.is_none() {
        return (S::UnobservedFinal, "final_http_response_not_observed");
    }
    if !coherent(t) {
        return (
            S::InvalidTransport,
            "inconsistent_initial_final_or_redirect_chain",
        );
    }
    match t.final_status {
        Some(400..=599) => (S::ObservedHttpFailure, "saved_final_http_4xx_or_5xx"),
        Some(200..=299) => (S::ObservedNonFailure, "saved_final_http_2xx"),
        _ => (
            S::NonterminalFinalStatus,
            "saved_final_status_not_terminal_2xx_4xx_5xx",
        ),
    }
}
fn unavailable(reason: &'static str) -> ResourceBrokenSourcesPage {
    ResourceBrokenSourcesPage {
        scope: RESOURCE_BROKEN_SOURCES_SCOPE,
        available: false,
        unavailable_reason: Some(reason),
        snapshot: None,
        rows: vec![],
        selected_evaluation_rows: 0,
        selected_distinct_occurrence_ids: 0,
        counts: BTreeMap::new(),
        next_sequence: None,
        has_more: false,
        website_requests: 0,
    }
}

// Caller holds exactly one Store read transaction and has already invoked the
// unchanged saved configuration/selected-parent/resource verifier on this TX.
pub(crate) fn project(
    c: &Connection,
    cfg: Option<&CrawlConfig>,
    r: &ResourceBrokenSourcesRequest,
    mode: Mode,
) -> Result<Output> {
    let reason = if cfg.is_none() {
        Some("saved_config_unavailable")
    } else if cfg.is_some_and(|c| c.resource_discovery.is_none()) {
        Some("native_resource_contract_disabled_or_legacy")
    } else if !exists(c, "resource_occurrences")? {
        Some("native_resource_history_unavailable")
    } else {
        None
    };
    if let Some(reason) = reason {
        ensure!(
            r.expected_snapshot_token.is_none(),
            "Resource snapshot no longer available"
        );
        ensure!(
            matches!(mode, Mode::Page),
            "Native resource history is unavailable for CSV"
        );
        return Ok(Output::Page(unavailable(reason)));
    }
    let cfg = cfg.context("Saved config missing")?;
    let Some(ledger) = crate::resource_ledger::summary(c)? else {
        ensure!(
            r.expected_snapshot_token.is_none(),
            "Resource snapshot no longer initialized"
        );
        ensure!(
            matches!(mode, Mode::Page),
            "Native resource ledger is not initialized for CSV"
        );
        return Ok(Output::Page(unavailable(
            "native_resource_ledger_not_initialized",
        )));
    };
    let run = metadata_text(c, "run_state", 64 * 1024)?
        .map(|s| serde_json::from_str::<CrawlRunState>(&s))
        .transpose()?;
    let saved = ResourceReadSnapshot {
        crawl_config_sha256: hash_json(cfg, 8 * 1024 * 1024)?.0,
        generation: run.map(|v| v.generation),
        resource_contract_engine: cfg
            .resource_discovery
            .as_ref()
            .and_then(|r| r.contract.as_ref())
            .map(|c| c.engine.clone()),
    };
    let mut hash = Sha256::new();
    hash.update(b"resource_transport_set_raw_utf8_length_frames_v1\0");
    let mut stmt = c.prepare(
        "SELECT url,data,payload_sha256 FROM resource_transports ORDER BY url COLLATE BINARY",
    )?;
    let mut rows = stmt.query([])?;
    let mut transports = 0usize;
    let mut transport_bytes = 0usize;
    while let Some(row) = rows.next()? {
        for i in 0..3 {
            let s = row.get_ref(i)?.as_str()?;
            framed(&mut hash, s.as_bytes());
            transport_bytes = transport_bytes
                .checked_add(s.len())
                .context("Transport set bytes overflow")?;
        }
        transports += 1;
    }
    hash.update((transports as u64).to_le_bytes());
    let transport_set_sha256 = format!("{:x}", hash.finalize());
    drop(rows);
    drop(stmt);
    let physical: i64 = c.query_row(
        "SELECT count(DISTINCT occurrence_id COLLATE BINARY) FROM resource_occurrences",
        [],
        |r| r.get(0),
    )?;
    let mut snapshot = ResourceBrokenSourcesSnapshot {
        engine: ENGINE,
        scope: RESOURCE_BROKEN_SOURCES_SCOPE,
        saved,
        transport_set_sha256,
        transport_rows: transports,
        transport_utf8_bytes: transport_bytes,
        evaluation_rows: ledger.decision_rows,
        distinct_stored_occurrence_ids: usize::try_from(physical)?,
        ledger,
        filter: r.filter,
        page_limit: r.limit,
        limits: r.limits.clone(),
        snapshot_token: String::new(),
        website_requests: 0,
        current_website_freshness_attested: false,
        source_kind_coverage_complete: false,
        precision_gaps: vec![
            "anchor_text_not_captured",
            "dom_index_path_truncation_ancestors_not_captured",
            "css_at_rule_path_and_font_category_not_fully_retained",
            "utf8_spans_address_decoded_source_unit_not_wire_or_html_bytes",
            "originating_html_ancestor_graph_not_inferred",
            "legacy_and_inherited_capture_gaps_remain_unknown",
        ],
    };
    snapshot.snapshot_token = hash_json(&snapshot, 4 * 1024 * 1024)?.0;
    ensure!(
        r.expected_snapshot_token
            .as_ref()
            .is_none_or(|t| t == &snapshot.snapshot_token),
        "Stale resource broken-sources snapshot/config/generation/filter/transport set"
    );
    if matches!(mode, Mode::Csv) {
        ensure!(
            r.expected_snapshot_token.is_some() && r.after_sequence.is_none(),
            "Complete CSV needs the preview snapshot token and no continuation"
        );
    }
    ensure!(
        r.after_sequence
            .is_none_or(|v| v <= snapshot.evaluation_rows as u64),
        "Continuation exceeds saved decision rows"
    );
    let mut output = vec![];
    let mut page_row_bytes = 0usize;
    let mut more = false;
    let mut counts = BTreeMap::new();
    let mut selected = 0usize;
    let mut selected_ids = BTreeSet::new();
    let mut seen_rows = 0usize;
    let mut csv = Bytes {
        data: vec![],
        cap: CSV_CAP,
    };
    if matches!(mode, Mode::Csv) {
        csv_record(
            &mut csv,
            &[
                "scope",
                "sequence",
                "occurrence_id",
                "evaluation_revision",
                "generation",
                "basis",
                "source_requested_url",
                "source_document_url",
                "source_unit_revision",
                "source_ordinal",
                "source_complete",
                "raw_value",
                "decoded_value",
                "original_resolved_url",
                "grant_fetch_url",
                "reference_kind",
                "classification_basis",
                "dom_path",
                "utf8_unit_byte_start",
                "utf8_unit_byte_end",
                "attributes_json",
                "anchor_text",
                "anchor_text_state",
                "state",
                "reason",
                "target_initial_status",
                "target_final_status",
                "target_final_url",
                "target_retained_response",
                "source_transport_json",
                "target_transport_json",
                "complete_evaluation_json",
                "snapshot_token",
                "transport_set_sha256",
                "distinct_stored_occurrence_ids",
                "evaluation_rows",
                "website_requests",
            ]
            .map(str::to_owned),
        )?;
    }
    let mut stmt=c.prepare("SELECT o.data,t.data,t.payload_sha256,s.data,s.payload_sha256 FROM resource_occurrences o LEFT JOIN resource_transports t ON t.url COLLATE BINARY=json_extract(o.data,'$.decision.grant.fetch_url') COLLATE BINARY LEFT JOIN resource_transports s ON s.url COLLATE BINARY=json_extract(o.data,'$.occurrence.requested_source_url') COLLATE BINARY ORDER BY o.sequence")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let raw = text(row, 0, r.limits.max_row_utf8_bytes)?;
        let observation: StoredResourceOccurrence = serde_json::from_str(&raw)?;
        seen_rows += 1;
        ensure!(
            observation.sequence == seen_rows as u64,
            "Joined occurrence sequence changed or multiplied"
        );
        let target = transport(row, 1, 2, r.limits.max_row_utf8_bytes)?;
        let source = transport(row, 3, 4, r.limits.max_row_utf8_bytes)?;
        let (state, reason) = state(&observation, target.as_ref());
        *counts.entry(state).or_insert(0) += 1;
        if !r.filter.includes(state) {
            continue;
        }
        selected += 1;
        selected_ids.insert(observation.occurrence.occurrence_id.clone());
        let entry = ResourceBrokenSourcesRow {
            grant_fetch_url: observation
                .decision
                .grant
                .as_ref()
                .map(|g| g.fetch_url.clone()),
            observation,
            source_transport: source,
            target_transport: target,
            state,
            reason,
            anchor_text: None,
            anchor_text_state: "not_captured_in_resource_occurrence",
        };
        if matches!(mode, Mode::Csv) {
            write_row(&mut csv, &entry, &snapshot)?;
        } else if entry.observation.sequence > r.after_sequence.unwrap_or(0) {
            if output.len() < r.limit {
                let n = hash_json(&entry, JSON_CAP)?.1;
                page_row_bytes = page_row_bytes
                    .checked_add(n)
                    .filter(|n| *n <= JSON_CAP)
                    .context("Resource page JSON row budget exceeded; reduce page limit")?;
                output.push(entry)
            } else {
                more = true
            }
        }
    }
    ensure!(
        seen_rows == snapshot.evaluation_rows,
        "Joined evaluation count differs from verified rolling ledger"
    );
    drop(rows);
    drop(stmt);
    if matches!(mode, Mode::Csv) {
        let sha256 = crate::resource_policy::digest(&csv.data);
        return Ok(Output::Csv(ResourceBrokenSourcesCsv {
            bytes: csv.data,
            sha256,
            snapshot,
            exported_evaluation_rows: selected,
            distinct_exported_occurrence_ids: selected_ids.len(),
        }));
    }
    let next_sequence = if more {
        output.last().map(|r| r.observation.sequence)
    } else {
        None
    };
    let page = ResourceBrokenSourcesPage {
        scope: RESOURCE_BROKEN_SOURCES_SCOPE,
        available: true,
        unavailable_reason: None,
        snapshot: Some(snapshot),
        rows: output,
        selected_evaluation_rows: selected,
        selected_distinct_occurrence_ids: selected_ids.len(),
        counts,
        next_sequence,
        has_more: more,
        website_requests: 0,
    };
    hash_json(&page, JSON_CAP)?;
    Ok(Output::Page(page))
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
            .ok_or_else(|| io::Error::other("Resource broken-sources CSV exceeds64MiB"))?;
        self.data.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn json<T: Serialize + ?Sized>(v: &T) -> Result<String> {
    hash_json(&v, JSON_CAP)?;
    Ok(serde_json::to_string(v)?)
}
fn enum_text(v: &impl Serialize) -> Result<String> {
    let value = serde_json::to_value(v)?;
    Ok(value
        .as_str()
        .context("Expected serialized resource enum string")?
        .to_owned())
}
fn csv_record(out: &mut impl Write, values: &[String]) -> Result<()> {
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            out.write_all(b",")?;
        }
        out.write_all(b"\"")?;
        out.write_all(crate::csv_text(v).replace('"', "\"\"").as_bytes())?;
        out.write_all(b"\"")?;
    }
    out.write_all(b"\r\n")?;
    Ok(())
}
fn write_row(
    out: &mut impl Write,
    r: &ResourceBrokenSourcesRow,
    s: &ResourceBrokenSourcesSnapshot,
) -> Result<()> {
    let o = &r.observation;
    let x = &o.occurrence;
    let t = r.target_transport.as_ref();
    csv_record(
        out,
        &[
            RESOURCE_BROKEN_SOURCES_SCOPE.into(),
            o.sequence.to_string(),
            x.occurrence_id.clone(),
            o.evaluation_revision.clone(),
            o.generation.to_string(),
            enum_text(&o.basis)?,
            x.requested_source_url.clone(),
            x.source_document_url.clone(),
            x.source_unit_revision.clone(),
            x.source_ordinal.to_string(),
            x.source_complete.to_string(),
            x.raw_value.clone(),
            x.decoded_value.clone().unwrap_or_default(),
            x.resolved_url.clone().unwrap_or_default(),
            r.grant_fetch_url.clone().unwrap_or_default(),
            enum_text(&x.reference_kind)?,
            x.classification_basis.clone(),
            x.dom_location.clone().unwrap_or_default(),
            x.byte_start.map(|v| v.to_string()).unwrap_or_default(),
            x.byte_end.map(|v| v.to_string()).unwrap_or_default(),
            json(&x.attributes)?,
            String::new(),
            r.anchor_text_state.into(),
            enum_text(&r.state)?,
            r.reason.into(),
            t.and_then(|t| t.initial_status)
                .map(|v| v.to_string())
                .unwrap_or_default(),
            t.and_then(|t| t.final_status)
                .map(|v| v.to_string())
                .unwrap_or_default(),
            t.map(|t| t.final_url.clone()).unwrap_or_default(),
            t.map(|t| t.retained_response.to_string())
                .unwrap_or_default(),
            json(&r.source_transport)?,
            json(&r.target_transport)?,
            json(o)?,
            s.snapshot_token.clone(),
            s.transport_set_sha256.clone(),
            s.distinct_stored_occurrence_ids.to_string(),
            s.evaluation_rows.to_string(),
            "0".into(),
        ],
    )
}
#[cfg(test)]
#[path = "resource_broken_sources_tests.rs"]
mod tests;
