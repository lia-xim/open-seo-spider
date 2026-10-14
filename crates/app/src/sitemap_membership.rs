//! Read-only sitemap projection after full Store validation. Inventory consumes
//! a bounded rich window; CSV visits all stored rows in the same verified TX.
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use open_seo_spider_core::{
    SITEMAP_LASTMOD_ENGINE, SITEMAP_READ_WINDOW_ENGINE, SitemapAnalysisRowKind, SitemapCatalog,
    SitemapCoverage, SitemapInheritedBasis, SitemapLastmodEntryAssessment, SitemapLastmodLimits,
    SitemapLedgerRow, SitemapLedgerSummary, SitemapMembershipAnalysis,
    SitemapMembershipAnalysisRequest, SitemapReadSnapshot, SitemapReadState, SitemapReadWindow,
    SitemapReadWindowExpectedContext, SitemapStoredRowsFilter, SitemapVerifiedReadHeader,
    assess_sitemap_lastmod_entry, project_sitemap_coverage, sitemap_read_state_sha256,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Write};
use uuid::Uuid;
pub const MAX_JSON_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_CSV_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_TOKEN_BYTES: usize = 4096;
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Failure {
    pub code: String,
    pub error: String,
    pub website_requests: u8,
}
impl Failure {
    pub fn new(code: &str, error: impl ToString) -> Self {
        Self {
            code: code.into(),
            error: error.to_string(),
            website_requests: 0,
        }
    }
    pub fn status(&self) -> u16 {
        match self.code.as_str() {
            "invalid_sitemap_query" | "invalid_sitemap_cursor" => 400,
            "stale_sitemap_cursor"
            | "sitemap_job_snapshot_conflict"
            | "sitemap_catalog_unavailable" => 409,
            "sitemap_response_byte_budget"
            | "sitemap_csv_byte_budget"
            | "sitemap_lastmod_assessment_budget" => 413,
            "sitemap_analysis_input_budget" => 413,
            "job_not_found" => 404,
            "sitemap_export_timeout" => 504,
            "sitemap_export_cancelled" => 503,
            _ => 500,
        }
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.error)
    }
}
impl std::error::Error for Failure {}
type Result<T> = std::result::Result<T, Failure>;
fn digest(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|error| Failure::new("sitemap_read_failure", error))
}
fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RowKind {
    #[default]
    All,
    Root,
    Document,
    Entry,
    Location,
}
impl RowKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Root => "root",
            Self::Document => "document",
            Self::Entry => "entry",
            Self::Location => "location",
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Filter {
    pub kind: RowKind,
    pub source_observation_id: Option<String>,
    pub membership_url: Option<String>,
}
impl Filter {
    pub fn validate(&self) -> Result<()> {
        if self
            .source_observation_id
            .as_ref()
            .is_some_and(|value| !is_hash(value))
        {
            return Err(Failure::new(
                "invalid_sitemap_query",
                "source_observation_id must be an exact lowercase SHA-256 ID",
            ));
        }
        if self.membership_url.as_ref().is_some_and(|value| {
            value.is_empty()
                || value.len() > 65536
                || value.chars().any(char::is_control)
                || open_seo_spider_core::normalize_url(value, None)
                    .is_none_or(|url| url.as_str() != value)
        }) {
            return Err(Failure::new(
                "invalid_sitemap_query",
                "membership_url must be an exact canonical HTTP(S) original identity",
            ));
        }
        Ok(())
    }
    fn matches(&self, row: &SitemapLedgerRow) -> bool {
        let (kind, source, member) = match row {
            SitemapLedgerRow::Root(row) => (
                RowKind::Root,
                row.source_observation_id.as_deref(),
                row.admission.membership_url.as_deref(),
            ),
            SitemapLedgerRow::Document(row) => {
                (RowKind::Document, Some(row.observation_id.as_str()), None)
            }
            SitemapLedgerRow::Entry(row) => {
                (RowKind::Entry, Some(row.observation_id.as_str()), None)
            }
            SitemapLedgerRow::Location(row) => (
                RowKind::Location,
                Some(row.observation_id.as_str()),
                row.admission.membership_url.as_deref(),
            ),
        };
        (self.kind == RowKind::All || self.kind == kind)
            && self
                .source_observation_id
                .as_deref()
                .is_none_or(|value| Some(value) == source)
            && self
                .membership_url
                .as_deref()
                .is_none_or(|value| Some(value) == member)
    }
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ReadQuery {
    pub kind: RowKind,
    pub source_observation_id: Option<String>,
    pub membership_url: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub page_limit: Option<usize>,
    pub page_cursor: Option<String>,
}
impl ReadQuery {
    pub fn filter(&self) -> Filter {
        Filter {
            kind: self.kind,
            source_observation_id: self.source_observation_id.clone(),
            membership_url: self.membership_url.clone(),
        }
    }
    pub fn validate(&self) -> Result<usize> {
        self.filter().validate()?;
        let limit = self.limit.unwrap_or(100);
        if !(1..=200).contains(&limit) {
            return Err(Failure::new(
                "invalid_sitemap_query",
                "limit must be 1..200",
            ));
        }
        if let Some(cursor) = &self.cursor {
            let parsed = decode(cursor)?;
            if parsed.payload.schema_version != 1
                || parsed.payload.after_sequence.is_none_or(|value| value == 0)
            {
                return Err(Failure::new(
                    "invalid_sitemap_cursor",
                    "Expected a nonzero continuation token",
                ));
            }
        }
        if self.page_limit.is_some_and(|value| value > 200) {
            return Err(Failure::new(
                "invalid_sitemap_query",
                "page_limit must be 0..200",
            ));
        }
        if let Some(cursor) = &self.page_cursor {
            if self.page_limit == Some(0) {
                return Err(Failure::new(
                    "invalid_sitemap_query",
                    "page_cursor requires a positive page_limit",
                ));
            }
            let parsed = decode(cursor)?;
            if parsed.payload.schema_version != 2 || parsed.payload.after_page_ordinal.is_none() {
                return Err(Failure::new(
                    "invalid_sitemap_cursor",
                    "Expected a saved-page continuation token",
                ));
            }
        }
        Ok(limit)
    }
    /// Parse both canonical token roles before the read-only Store is opened.
    /// Snapshot equality is checked only after the actual same-TX capture.
    pub fn analysis_request(&self) -> Result<SitemapMembershipAnalysisRequest> {
        let limit = self.validate()?;
        Ok(SitemapMembershipAnalysisRequest {
            kind: match self.kind {
                RowKind::All => SitemapAnalysisRowKind::All,
                RowKind::Root => SitemapAnalysisRowKind::Root,
                RowKind::Document => SitemapAnalysisRowKind::Document,
                RowKind::Entry => SitemapAnalysisRowKind::Entry,
                RowKind::Location => SitemapAnalysisRowKind::Location,
            },
            source_observation_id: self.source_observation_id.clone(),
            membership_url: self.membership_url.clone(),
            after_sequence: self
                .cursor
                .as_deref()
                .map(decode)
                .transpose()?
                .and_then(|v| v.payload.after_sequence),
            row_limit: limit,
            page_limit: self.page_limit.unwrap_or(100),
            after_page_ordinal: self
                .page_cursor
                .as_deref()
                .map(decode)
                .transpose()?
                .and_then(|v| v.payload.after_page_ordinal),
        })
    }

    /// Parse canonical roles and bind caller/job/filter before opening Store.
    /// Core compares the resulting expected context with its actual same-TX
    /// state before selecting either raw rows or saved-page continuations.
    pub fn expected_context(
        &self,
        job_id: Uuid,
    ) -> Result<Option<SitemapReadWindowExpectedContext>> {
        self.validate()?;
        let filter_sha256 = digest(encode(&self.filter())?);
        let mut expected: Option<Snapshot> = None;
        for value in [self.cursor.as_deref(), self.page_cursor.as_deref()]
            .into_iter()
            .flatten()
        {
            let parsed = decode(value)?;
            if parsed.payload.snapshot.job_id != job_id
                || parsed.payload.filter_sha256 != filter_sha256
                || expected
                    .as_ref()
                    .is_some_and(|snapshot| snapshot != &parsed.payload.snapshot)
            {
                return Err(Failure::new(
                    "stale_sitemap_cursor",
                    "Snapshot, job or filter changed; restart pagination",
                ));
            }
            expected = Some(parsed.payload.snapshot);
        }
        Ok(expected.map(|snapshot| SitemapReadWindowExpectedContext {
            generation: snapshot.crawl_generation,
            crawl_config_sha256: snapshot.crawl_config_sha256,
            catalog_config_sha256: snapshot.catalog_config_sha256,
            catalog_revision: snapshot.catalog_revision,
            catalog_rolling_sha256: snapshot.catalog_rolling_sha256,
            catalog_state_sha256: snapshot.catalog_state_sha256,
            inherited_basis: snapshot.inherited_basis,
        }))
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CsvQuery {
    #[serde(default)]
    pub kind: RowKind,
    pub source_observation_id: Option<String>,
    pub membership_url: Option<String>,
    pub snapshot: String,
}
impl CsvQuery {
    pub fn validate(&self) -> Result<()> {
        Filter {
            kind: self.kind,
            source_observation_id: self.source_observation_id.clone(),
            membership_url: self.membership_url.clone(),
        }
        .validate()?;
        let token = decode(&self.snapshot)?;
        if token.payload.schema_version != 1 || token.payload.after_sequence.is_some() {
            return Err(Failure::new(
                "invalid_sitemap_cursor",
                "Expected a full-export snapshot token",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Snapshot {
    pub job_id: Uuid,
    pub crawl_generation: u64,
    pub crawl_config_sha256: String,
    pub catalog_config_sha256: String,
    pub catalog_revision: u64,
    pub catalog_rolling_sha256: String,
    pub catalog_state_sha256: String,
    pub inherited_basis: Option<SitemapInheritedBasis>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Payload {
    schema_version: u32,
    snapshot: Snapshot,
    filter_sha256: String,
    after_sequence: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    after_page_ordinal: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Token {
    payload: Payload,
    checksum_sha256: String,
}
fn token(snapshot: &Snapshot, filter: &Filter, after_sequence: Option<u64>) -> Result<String> {
    let payload = Payload {
        schema_version: 1,
        snapshot: snapshot.clone(),
        filter_sha256: digest(encode(filter)?),
        after_sequence,
        after_page_ordinal: None,
    };
    let checksum_sha256 = digest(encode(&payload)?);
    let bytes = encode(&Token {
        payload,
        checksum_sha256,
    })?;
    let result = URL_SAFE_NO_PAD.encode(bytes);
    if result.len() > MAX_TOKEN_BYTES {
        return Err(Failure::new(
            "sitemap_read_failure",
            "Cursor profile exceeds its bound",
        ));
    }
    Ok(result)
}
fn decode(value: &str) -> Result<Token> {
    let bad = |message: &str| Failure::new("invalid_sitemap_cursor", message);
    if value.is_empty() || value.len() > MAX_TOKEN_BYTES {
        return Err(bad("Invalid token byte length"));
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| bad("Token encoding is invalid"))?;
    let parsed: Token =
        serde_json::from_slice(&bytes).map_err(|_| bad("Token shape/types are invalid"))?;
    let canonical = serde_json::to_vec(&parsed).map_err(|_| bad("Token cannot be serialized"))?;
    if URL_SAFE_NO_PAD.encode(canonical) != value
        || !matches!(parsed.payload.schema_version, 1 | 2)
        || (parsed.payload.schema_version == 1 && parsed.payload.after_page_ordinal.is_some())
        || (parsed.payload.schema_version == 2
            && (parsed.payload.after_sequence.is_some()
                || parsed
                    .payload
                    .after_page_ordinal
                    .is_none_or(|value| value >= 1_000_000)))
        || !is_hash(&parsed.checksum_sha256)
        || !is_hash(&parsed.payload.filter_sha256)
        || parsed.payload.snapshot.crawl_generation == 0
        || !is_hash(&parsed.payload.snapshot.crawl_config_sha256)
        || !is_hash(&parsed.payload.snapshot.catalog_config_sha256)
        || !is_hash(&parsed.payload.snapshot.catalog_rolling_sha256)
        || !is_hash(&parsed.payload.snapshot.catalog_state_sha256)
        || digest(
            serde_json::to_vec(&parsed.payload)
                .map_err(|_| bad("Token payload serialization failed"))?,
        ) != parsed.checksum_sha256
    {
        return Err(bad("Token integrity/profile is invalid"));
    }
    Ok(parsed)
}
#[cfg(test)]
fn bound_catalog(
    job_id: Uuid,
    generation: u64,
    config_sha: &str,
    read: Option<SitemapReadSnapshot>,
) -> Result<Option<(SitemapCatalog, Snapshot)>> {
    let Some(read) = read else {
        return Ok(None);
    };
    let Some(catalog) = read.catalog else {
        return Ok(None);
    };
    if generation == 0
        || read.generation != Some(generation)
        || read.crawl_config_sha256.as_deref() != Some(config_sha)
    {
        return Err(Failure::new(
            "sitemap_job_snapshot_conflict",
            "Actual SQL generation/config differs from the App job",
        ));
    }
    // Only the fully validated one-TX Core Store snapshot supplies this value.
    // The Core catalog validator is private; do not replace its full read gate
    // with a weaker App-only schema/hash check or accept a supplied report.
    if catalog.rows.is_none() {
        return Err(Failure::new(
            "sitemap_read_failure",
            "Validated catalog rows are unavailable",
        ));
    }
    let inventory_sha256 = read
        .coverage_inventory
        .as_ref()
        .map(|inventory| encode(inventory).map(digest))
        .transpose()?;
    let catalog_state_sha256 = sitemap_read_state_sha256(&SitemapReadState {
        summary: &catalog.summary,
        inherited: catalog.inherited.as_ref(),
        coverage_inventory_sha256: inventory_sha256.as_deref(),
        coverage: &read.coverage,
        source_measurement_context: &read.sitemap_source_measurement_context,
        analysis_context: read
            .membership_analysis
            .as_ref()
            .map(|analysis| &analysis.context),
    })
    .map_err(|error| Failure::new("sitemap_read_failure", error))?;
    let snapshot = Snapshot {
        job_id,
        crawl_generation: generation,
        crawl_config_sha256: config_sha.into(),
        catalog_config_sha256: catalog.summary.config_sha256.clone(),
        catalog_revision: catalog.summary.revision,
        catalog_rolling_sha256: catalog.summary.rolling_sha256.clone(),
        // Same explicit rich-reader profile as compact JSON. The optional
        // captured-inventory body hash preserves None and avoids token drift
        // between JSON windows and this buffered CSV compatibility reader.
        catalog_state_sha256,
        inherited_basis: catalog.inherited.clone(),
    };
    Ok(Some((catalog, snapshot)))
}
fn validate_token(
    value: &str,
    snapshot: &Snapshot,
    filter: &Filter,
    continuation: bool,
) -> Result<Option<u64>> {
    let parsed = decode(value)?;
    if parsed.payload.snapshot != *snapshot
        || parsed.payload.filter_sha256 != digest(encode(filter)?)
    {
        return Err(Failure::new(
            "stale_sitemap_cursor",
            "Snapshot or filter changed; restart pagination",
        ));
    }
    if parsed.payload.schema_version != 1
        || continuation != parsed.payload.after_sequence.is_some()
        || parsed.payload.after_sequence == Some(0)
    {
        return Err(Failure::new(
            "invalid_sitemap_cursor",
            "Wrong snapshot/continuation token role",
        ));
    }
    Ok(parsed.payload.after_sequence)
}
#[derive(Debug, Serialize)]
pub(crate) struct Coverage {
    pub declared_source_cohort_complete: bool,
    pub negative_membership: &'static str,
    pub fetch_status_join: &'static str,
    pub scope: &'static str,
    /// Separate computed source-graph evidence; the original stored summary
    /// and broad/legacy negative_membership fields are never rewritten.
    pub derived: SitemapCoverage,
}
#[derive(Debug, Serialize)]
pub(crate) struct Inventory {
    pub schema_version: u32,
    pub job_id: Uuid,
    pub catalog_available: bool,
    pub availability_reason: &'static str,
    pub summary: Option<SitemapLedgerSummary>,
    pub inherited: Option<SitemapInheritedBasis>,
    pub snapshot: Option<Snapshot>,
    pub snapshot_token: Option<String>,
    pub filter: Filter,
    pub rows: Option<Vec<SitemapLedgerRow>>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub website_requests: u8,
    pub snapshot_scope: &'static str,
    pub coverage: Coverage,
    pub lastmod_assessment_engine: &'static str,
    /// Only visible Entry rows are projected. None is unavailable history;
    /// an available page with no entries has Some([]), not missing evidence.
    pub lastmod_assessments: Option<Vec<SitemapLastmodEntryAssessment>>,
    pub membership_analysis: Option<SitemapMembershipAnalysis>,
    pub next_page_cursor: Option<String>,
}
#[cfg(test)]
pub(crate) fn inventory(
    job_id: Uuid,
    generation: u64,
    config_sha: &str,
    read: Option<SitemapReadSnapshot>,
    query: ReadQuery,
) -> Result<Inventory> {
    query.validate()?;
    let sqlite_available = read.is_some();
    let analysis = read
        .as_ref()
        .and_then(|value| value.membership_analysis.clone());
    let derived_coverage = if let Some(read) = &read {
        read.coverage.clone()
    } else {
        project_sitemap_coverage(None, None)
            .map_err(|error| Failure::new("sitemap_read_failure", error))?
    };
    let bound = bound_catalog(job_id, generation, config_sha, read)?.map(|(catalog, snapshot)| {
        (
            InventoryBasis {
                summary: catalog.summary.clone(),
                inherited: catalog.inherited.clone(),
                snapshot,
            },
            InventoryRows::Materialized(catalog),
        )
    });
    project_inventory(
        job_id,
        query,
        sqlite_available,
        analysis,
        derived_coverage,
        bound,
    )
}

/// Actual HTTP inventory path. It consumes only Core's bounded selected rows;
/// it never manufactures a truncated SitemapCatalog or source-store snapshot.
pub(crate) fn inventory_window(
    job_id: Uuid,
    generation: u64,
    config_sha: &str,
    read: Option<SitemapReadWindow>,
    query: ReadQuery,
) -> Result<Inventory> {
    let _ = query.expected_context(job_id)?;
    let sqlite_available = read.is_some();
    let derived_coverage = match &read {
        Some(read) => read.coverage.clone(),
        None => project_sitemap_coverage(None, None)
            .map_err(|error| Failure::new("sitemap_read_failure", error))?,
    };
    let mut analysis = None;
    let bound = if let Some(read) = read {
        if read.reader_engine != SITEMAP_READ_WINDOW_ENGINE {
            return Err(Failure::new(
                "sitemap_read_failure",
                "Unknown rich sitemap reader profile",
            ));
        }
        if let Some(summary) = read.ledger {
            if generation == 0
                || read.generation != Some(generation)
                || read.crawl_config_sha256.as_deref() != Some(config_sha)
            {
                return Err(Failure::new(
                    "sitemap_job_snapshot_conflict",
                    "Actual SQL generation/config differs from the App job",
                ));
            }
            let state = sitemap_read_state_sha256(&SitemapReadState {
                summary: &summary,
                inherited: read.inherited.as_ref(),
                coverage_inventory_sha256: read.coverage_inventory_sha256.as_deref(),
                coverage: &read.coverage,
                source_measurement_context: &read.sitemap_source_measurement_context,
                analysis_context: read
                    .membership_analysis
                    .as_ref()
                    .map(|value| &value.context),
            })
            .map_err(|error| Failure::new("sitemap_read_failure", error))?;
            if read.catalog_state_sha256.as_deref() != Some(state.as_str()) {
                return Err(Failure::new(
                    "sitemap_read_failure",
                    "Rich reader state binding differs",
                ));
            }
            let snapshot = Snapshot {
                job_id,
                crawl_generation: generation,
                crawl_config_sha256: config_sha.into(),
                catalog_config_sha256: summary.config_sha256.clone(),
                catalog_revision: summary.revision,
                catalog_rolling_sha256: summary.rolling_sha256.clone(),
                catalog_state_sha256: state,
                inherited_basis: read.inherited.clone(),
            };
            let rows = read.rows.ok_or_else(|| {
                Failure::new(
                    "sitemap_read_failure",
                    "Validated rich window rows are unavailable",
                )
            })?;
            analysis = read.membership_analysis;
            Some((
                InventoryBasis {
                    summary,
                    inherited: read.inherited,
                    snapshot,
                },
                InventoryRows::Window {
                    rows,
                    next: read.next_after_sequence,
                    has_more: read.has_more,
                },
            ))
        } else {
            if read.rows.is_some()
                || read.next_after_sequence.is_some()
                || read.has_more
                || read.catalog_state_sha256.is_some()
            {
                return Err(Failure::new(
                    "sitemap_read_failure",
                    "Unavailable rich catalog has retained window state",
                ));
            }
            analysis = read.membership_analysis;
            None
        }
    } else {
        None
    };
    project_inventory(
        job_id,
        query,
        sqlite_available,
        analysis,
        derived_coverage,
        bound,
    )
}

struct InventoryBasis {
    summary: SitemapLedgerSummary,
    inherited: Option<SitemapInheritedBasis>,
    snapshot: Snapshot,
}
enum InventoryRows {
    // Kept for the existing materializing oracle/tests; HTTP uses Window.
    #[cfg(test)]
    Materialized(SitemapCatalog),
    Window {
        rows: Vec<SitemapLedgerRow>,
        next: Option<u64>,
        has_more: bool,
    },
}
fn project_inventory(
    job_id: Uuid,
    query: ReadQuery,
    sqlite_available: bool,
    analysis: Option<SitemapMembershipAnalysis>,
    derived_coverage: SitemapCoverage,
    bound: Option<(InventoryBasis, InventoryRows)>,
) -> Result<Inventory> {
    let limit = query.validate()?;
    let filter = query.filter();
    let mut response = Inventory {
        schema_version: 1,
        job_id,
        catalog_available: false,
        availability_reason: if sqlite_available {
            "sitemap_catalog_unavailable"
        } else {
            "sqlite_history_unavailable"
        },
        summary: None,
        inherited: None,
        snapshot: None,
        snapshot_token: None,
        filter: filter.clone(),
        rows: None,
        next_cursor: None,
        has_more: false,
        website_requests: 0,
        snapshot_scope: "validated_sqlite_read_snapshot",
        coverage: Coverage {
            declared_source_cohort_complete: false,
            negative_membership: "unknown",
            fetch_status_join: "not_performed",
            scope: "retained_original_declarations_and_memberships",
            derived: derived_coverage,
        },
        lastmod_assessment_engine: SITEMAP_LASTMOD_ENGINE,
        lastmod_assessments: None,
        membership_analysis: analysis,
        next_page_cursor: None,
    };
    if let Some((basis, selection)) = bound {
        let snapshot = basis.snapshot;
        if let Some(value) = query.page_cursor.as_deref() {
            let parsed = decode(value)?;
            if parsed.payload.snapshot != snapshot
                || parsed.payload.filter_sha256 != digest(encode(&filter)?)
            {
                return Err(Failure::new(
                    "stale_sitemap_cursor",
                    "Saved report context or filter changed; restart page pagination",
                ));
            }
            if response
                .membership_analysis
                .as_ref()
                .is_none_or(|value| !value.context.available)
            {
                return Err(Failure::new(
                    "stale_sitemap_cursor",
                    "Saved target analysis is unavailable",
                ));
            }
            if parsed.payload.after_page_ordinal.is_some_and(|after| {
                response
                    .membership_analysis
                    .as_ref()
                    .and_then(|value| value.context.saved_page_count)
                    .is_none_or(|count| after >= count)
            }) {
                return Err(Failure::new(
                    "invalid_sitemap_cursor",
                    "Saved-page continuation ordinal is not in the captured report",
                ));
            }
        }
        let after = query
            .cursor
            .as_deref()
            .map(|value| validate_token(value, &snapshot, &filter, true))
            .transpose()?
            .flatten();
        let (rows, next, has_more) = match selection {
            #[cfg(test)]
            InventoryRows::Materialized(catalog) => {
                let all = catalog.rows.as_ref().expect("validated rows");
                if after.is_some_and(|after| {
                    !all.iter()
                        .any(|row| row.sequence() == after && filter.matches(row))
                }) {
                    return Err(Failure::new(
                        "invalid_sitemap_cursor",
                        "Continuation sequence is not a matching retained row",
                    ));
                }
                let mut matching = all.iter().filter(|row| {
                    filter.matches(row) && after.is_none_or(|after| row.sequence() > after)
                });
                let rows: Vec<_> = matching.by_ref().take(limit).cloned().collect();
                let has_more = matching.next().is_some();
                let next = has_more.then(|| rows.last().expect("positive page size").sequence());
                (rows, next, has_more)
            }
            InventoryRows::Window {
                rows,
                next,
                has_more,
            } => {
                if rows.len() > limit
                    || rows.iter().any(|row| {
                        !filter.matches(row) || after.is_some_and(|after| row.sequence() <= after)
                    })
                    || has_more != next.is_some()
                    || next.is_some_and(|next| rows.last().is_none_or(|row| row.sequence() != next))
                {
                    return Err(Failure::new(
                        "sitemap_read_failure",
                        "Core window differs from the requested bounded selection",
                    ));
                }
                (rows, next, has_more)
            }
        };
        let limits = SitemapLastmodLimits::default();
        let assessments = rows
            .iter()
            .filter_map(|row| match row {
                SitemapLedgerRow::Entry(entry) => Some(entry),
                _ => None,
            })
            .map(|entry| {
                assess_sitemap_lastmod_entry(entry, &limits)
                    .map_err(|error| Failure::new("sitemap_lastmod_assessment_budget", error))
            })
            .collect::<Result<Vec<_>>>()?;
        response.next_cursor = next
            .map(|next| token(&snapshot, &filter, Some(next)))
            .transpose()?;
        response.snapshot_token = Some(token(&snapshot, &filter, None)?);
        if let Some(analysis) = &response.membership_analysis
            && analysis.saved_pages_have_more
        {
            let after_page_ordinal = analysis.last_saved_page_ordinal.ok_or_else(|| {
                Failure::new(
                    "sitemap_read_failure",
                    "Missing saved-page continuation identity",
                )
            })?;
            response.next_page_cursor = Some(page_token(&snapshot, &filter, after_page_ordinal)?);
        }
        response.snapshot = Some(snapshot);
        response.coverage.declared_source_cohort_complete =
            basis.summary.declared_source_cohort_complete;
        response.summary = Some(basis.summary);
        response.inherited = basis.inherited;
        response.rows = Some(rows);
        response.lastmod_assessments = Some(assessments);
        response.has_more = has_more;
        response.catalog_available = true;
        response.availability_reason = "saved_validated_sqlite_catalog";
    } else if query.cursor.is_some() || query.page_cursor.is_some() {
        return Err(Failure::new(
            "stale_sitemap_cursor",
            "Catalog is unavailable; no continuation exists",
        ));
    }
    // Measure the complete actual response with a bounded serializer. Derived
    // assessment/raw copies must not allocate an unbounded JSON buffer before
    // rejecting the page. No successful prefix can escape on a cap failure.
    let mut measured = BoundedBytes {
        bytes: Vec::new(),
        max: MAX_JSON_BYTES,
    };
    serde_json::to_writer(&mut measured, &response).map_err(|error| {
        if error.is_io() {
            Failure::new(
                "sitemap_response_byte_budget",
                "JSON page exceeds 8 MiB; request a smaller limit",
            )
        } else {
            Failure::new("sitemap_read_failure", error)
        }
    })?;
    Ok(response)
}
fn page_token(snapshot: &Snapshot, filter: &Filter, after_page_ordinal: u64) -> Result<String> {
    let payload = Payload {
        schema_version: 2,
        snapshot: snapshot.clone(),
        filter_sha256: digest(encode(filter)?),
        after_sequence: None,
        after_page_ordinal: Some(after_page_ordinal),
    };
    let checksum_sha256 = digest(encode(&payload)?);
    let result = URL_SAFE_NO_PAD.encode(encode(&Token {
        payload,
        checksum_sha256,
    })?);
    if result.len() > MAX_TOKEN_BYTES {
        return Err(Failure::new(
            "sitemap_read_failure",
            "Saved-page cursor profile exceeds its bound",
        ));
    }
    Ok(result)
}
struct BoundedBytes {
    bytes: Vec<u8>,
    max: usize,
}
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|length| length > self.max)
        {
            return Err(io::Error::other("sitemap CSV byte budget"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn csv_record(writer: &mut impl Write, fields: &[String]) -> io::Result<()> {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            writer.write_all(b",")?;
        }
        writer.write_all(b"\"")?;
        for (index, piece) in field.split('"').enumerate() {
            if index > 0 {
                writer.write_all(b"\"\"")?;
            }
            writer.write_all(piece.as_bytes())?;
        }
        writer.write_all(b"\"")?;
    }
    writer.write_all(b"\r\n")
}
fn write_csv_fields(writer: &mut impl Write, fields: Vec<String>) -> Result<()> {
    csv_record(writer, &fields).map_err(|error| Failure::new("sitemap_csv_byte_budget", error))
}
// Shared by the current buffered export and the planned verified SQL visitor.
// Keep the column order and row encoding identical across output sinks.
pub(crate) fn write_csv_header(writer: &mut impl Write) -> Result<()> {
    write_csv_fields(
        writer,
        [
            "kind",
            "sequence",
            "observation_id",
            "occurrence_id",
            "source_ordinal",
            "raw_value",
            "membership_url",
            "admitted_fetch_url",
            "admission_decision",
            "typed_row_json",
        ]
        .map(str::to_owned)
        .to_vec(),
    )
}
pub(crate) fn write_csv_row(writer: &mut impl Write, row: &SitemapLedgerRow) -> Result<()> {
    let (kind, source, id, ordinal, raw, member, alias, decision) = match row {
        SitemapLedgerRow::Root(r) => (
            "root",
            r.source_observation_id.clone().unwrap_or_default(),
            r.declaration_id.clone(),
            r.source_ordinal.to_string(),
            r.raw_value.clone(),
            r.admission.membership_url.clone(),
            r.admission.admitted_fetch_url.clone(),
            Some(&r.admission.decision),
        ),
        SitemapLedgerRow::Document(r) => (
            "document",
            r.observation_id.clone(),
            r.observation_id.clone(),
            String::new(),
            String::new(),
            None,
            None,
            None,
        ),
        SitemapLedgerRow::Entry(r) => (
            "entry",
            r.observation_id.clone(),
            r.entry_id.clone(),
            r.observed.ordinal.to_string(),
            String::new(),
            None,
            None,
            None,
        ),
        SitemapLedgerRow::Location(r) => (
            "location",
            r.observation_id.clone(),
            r.occurrence_id.clone(),
            r.observed.ordinal.to_string(),
            r.observed.decoded_value.clone().unwrap_or_default(),
            r.admission.membership_url.clone(),
            r.admission.admitted_fetch_url.clone(),
            Some(&r.admission.decision),
        ),
    };
    let decision = decision
        .map(serde_json::to_value)
        .transpose()
        .map_err(|error| Failure::new("sitemap_read_failure", error))?
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default();
    let fields = vec![
        kind.into(),
        row.sequence().to_string(),
        source,
        id,
        ordinal,
        raw,
        member.unwrap_or_default(),
        alias.unwrap_or_default(),
        decision,
        String::from_utf8(encode(row)?)
            .map_err(|error| Failure::new("sitemap_read_failure", error))?,
    ];
    write_csv_fields(
        writer,
        fields
            .into_iter()
            .enumerate()
            .map(|(index, text)| {
                if matches!(index, 1 | 4) {
                    text
                } else {
                    open_seo_spider_core::csv_text(&text)
                }
            })
            .collect(),
    )
}
#[cfg(test)]
pub(crate) fn csv(
    job_id: Uuid,
    generation: u64,
    config_sha: &str,
    read: Option<SitemapReadSnapshot>,
    query: CsvQuery,
) -> Result<(Vec<u8>, Snapshot)> {
    query.validate()?;
    let filter = Filter {
        kind: query.kind,
        source_observation_id: query.source_observation_id,
        membership_url: query.membership_url,
    };
    filter.validate()?;
    let (catalog, snapshot) =
        bound_catalog(job_id, generation, config_sha, read)?.ok_or_else(|| {
            Failure::new(
                "sitemap_catalog_unavailable",
                "Saved SQLite sitemap catalog unavailable",
            )
        })?;
    validate_token(&query.snapshot, &snapshot, &filter, false)?;
    let mut out = BoundedBytes {
        bytes: Vec::new(),
        max: MAX_CSV_BYTES,
    };
    write_csv_header(&mut out)?;
    for row in catalog
        .rows
        .as_ref()
        .expect("validated rows")
        .iter()
        .filter(|row| filter.matches(row))
    {
        write_csv_row(&mut out, row)?;
    }
    Ok((out.bytes, snapshot))
}

#[cfg(test)]
#[path = "sitemap_membership_tests.rs"]
mod tests;

impl CsvQuery {
    pub(crate) fn stored_filter(&self) -> SitemapStoredRowsFilter {
        SitemapStoredRowsFilter {
            kind: match self.kind {
                RowKind::All => SitemapAnalysisRowKind::All,
                RowKind::Root => SitemapAnalysisRowKind::Root,
                RowKind::Document => SitemapAnalysisRowKind::Document,
                RowKind::Entry => SitemapAnalysisRowKind::Entry,
                RowKind::Location => SitemapAnalysisRowKind::Location,
            },
            source_observation_id: self.source_observation_id.clone(),
            membership_url: self.membership_url.clone(),
        }
    }
    fn csv_filter(&self) -> Filter {
        Filter {
            kind: self.kind,
            source_observation_id: self.source_observation_id.clone(),
            membership_url: self.membership_url.clone(),
        }
    }
    pub(crate) fn expected_context(
        &self,
        job_id: Uuid,
    ) -> Result<SitemapReadWindowExpectedContext> {
        self.validate()?;
        let parsed = decode(&self.snapshot)?;
        if parsed.payload.snapshot.job_id != job_id
            || parsed.payload.filter_sha256 != digest(encode(&self.csv_filter())?)
        {
            return Err(Failure::new(
                "stale_sitemap_cursor",
                "Snapshot, job or filter changed; restart export",
            ));
        }
        let snapshot = parsed.payload.snapshot;
        Ok(SitemapReadWindowExpectedContext {
            generation: snapshot.crawl_generation,
            crawl_config_sha256: snapshot.crawl_config_sha256,
            catalog_config_sha256: snapshot.catalog_config_sha256,
            catalog_revision: snapshot.catalog_revision,
            catalog_rolling_sha256: snapshot.catalog_rolling_sha256,
            catalog_state_sha256: snapshot.catalog_state_sha256,
            inherited_basis: snapshot.inherited_basis,
        })
    }
    /// The header is obtained only inside Core's actual opaque same-TX callback.
    pub(crate) fn bind_verified_header(
        &self,
        job_id: Uuid,
        generation: u64,
        config_sha: &str,
        header: &SitemapVerifiedReadHeader,
    ) -> Result<Snapshot> {
        let summary = header.ledger.as_ref().ok_or_else(|| {
            Failure::new(
                "sitemap_catalog_unavailable",
                "Saved SQLite sitemap catalog unavailable",
            )
        })?;
        self.expected_context(job_id)?;
        if generation == 0
            || header.generation != Some(generation)
            || header.crawl_config_sha256.as_deref() != Some(config_sha)
        {
            return Err(Failure::new(
                "sitemap_job_snapshot_conflict",
                "Actual SQL generation/config differs from the App job",
            ));
        }
        if header.reader_engine != SITEMAP_READ_WINDOW_ENGINE {
            return Err(Failure::new(
                "sitemap_read_failure",
                "Unknown rich sitemap reader profile",
            ));
        }
        let state = sitemap_read_state_sha256(&SitemapReadState {
            summary,
            inherited: header.inherited.as_ref(),
            coverage_inventory_sha256: header.coverage_inventory_sha256.as_deref(),
            coverage: &header.coverage,
            source_measurement_context: &header.sitemap_source_measurement_context,
            analysis_context: Some(&header.analysis_context),
        })
        .map_err(|error| Failure::new("sitemap_read_failure", error))?;
        if header.catalog_state_sha256.as_deref() != Some(state.as_str()) {
            return Err(Failure::new(
                "sitemap_read_failure",
                "Verified CSV state binding differs",
            ));
        }
        let snapshot = Snapshot {
            job_id,
            crawl_generation: generation,
            crawl_config_sha256: config_sha.into(),
            catalog_config_sha256: summary.config_sha256.clone(),
            catalog_revision: summary.revision,
            catalog_rolling_sha256: summary.rolling_sha256.clone(),
            catalog_state_sha256: state,
            inherited_basis: header.inherited.clone(),
        };
        validate_token(&self.snapshot, &snapshot, &self.csv_filter(), false)?;
        Ok(snapshot)
    }
}
