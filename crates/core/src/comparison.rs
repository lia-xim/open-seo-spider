//! Draft B11: deterministic comparison of recorded report facts; no HTTP/SQL.
//! Join mappings affect identity only. Original observed values are never rewritten.
use crate::{
    ContentCaptureSourceKind, CrawlReport, PageContentCapture, PageContentCaptureStatus,
    PageRecord, RecrawlTargetState, SELECTED_RECRAWL_COVERAGE, SELECTED_RECRAWL_ENGINE,
    validate_page_content_captures,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Write};
use url::Url;

pub const SAVED_COMPARISON_ENGINE: &str = "open-seo-spider/saved-comparison-v1";
pub const COMPARISON_REPORT_HASH_PROFILE: &str = "typed-crawl-report-serde-json-v1";
pub const COMPARISON_IDENTITY_PROFILE: &str = "exact-recorded-requested-url-v1";
pub const COMPARISON_MAPPING_PROFILE: &str = "origin-segment-path-prefix-join-key-only-v1";
pub const COMPARISON_JSON_PROFILE: &str = "json-structural-equality-block-and-array-order-v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonSide {
    Before,
    After,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ComparisonUrlMapping {
    pub side: ComparisonSide,
    /// Canonical HTTP(S) origin, without trailing slash, credentials/query/fragment.
    pub from_origin: String,
    /// Absolute path, matching itself and descendants at a slash boundary.
    pub from_path_prefix: String,
    pub to_origin: String,
    pub to_path_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SavedComparisonLimits {
    pub max_pages_per_snapshot: usize,
    pub max_mappings: usize,
    pub max_url_bytes: usize,
    /// Sum of the exact typed serializations of both reports, before projection.
    pub max_input_json_bytes: usize,
    pub max_output_json_bytes: usize,
}
impl Default for SavedComparisonLimits {
    fn default() -> Self {
        Self {
            max_pages_per_snapshot: 100_000,
            max_mappings: 32,
            max_url_bytes: 16 * 1024,
            max_input_json_bytes: 256 * 1024 * 1024,
            max_output_json_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SavedComparisonConfig {
    pub engine: String,
    pub mappings: Vec<ComparisonUrlMapping>,
    /// Selected text is opt-in; hashes/byte counts are returned either way.
    pub include_selected_text: bool,
    pub limits: SavedComparisonLimits,
}
impl Default for SavedComparisonConfig {
    fn default() -> Self {
        Self {
            engine: SAVED_COMPARISON_ENGINE.into(),
            mappings: Vec::new(),
            include_selected_text: false,
            limits: SavedComparisonLimits::default(),
        }
    }
}

/// Optional caller declaration. Pure Core cannot attest SQL/native/imported origin.
/// A typed-report hash is checked; an exact-file-byte hash remains explicitly declared.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ComparisonCallerBinding {
    pub job_id: Option<String>,
    pub snapshot_sha256: Option<String>,
    pub sha_profile: Option<String>,
    pub generation: Option<u64>,
    pub origin: Option<String>,
    pub basis: Option<String>,
    pub active_checkpoint: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComparisonIdentityOccurrence {
    pub ordinal: usize,
    pub original_requested_url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComparisonCollision {
    pub side: ComparisonSide,
    pub comparison_key: String,
    pub occurrences: Vec<ComparisonIdentityOccurrence>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComparisonError {
    pub code: String,
    pub message: String,
    pub collisions: Vec<ComparisonCollision>,
}
impl fmt::Display for ComparisonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for ComparisonError {}
fn fail(code: &str, message: impl Into<String>) -> ComparisonError {
    ComparisonError {
        code: code.into(),
        message: message.into(),
        collisions: Vec::new(),
    }
}
type CResult<T> = Result<T, ComparisonError>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonFactState {
    Observed,
    ObservedEmpty,
    Partial,
    Missing,
    Unknown,
    Unavailable,
    Error,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonFact {
    pub state: ComparisonFactState,
    pub value: Option<Value>,
    pub profile: Option<String>,
    pub reasons: Vec<String>,
    /// Source revisions/timestamps/generations do not participate in value equality.
    pub provenance: Option<Value>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonFieldOutcome {
    Changed,
    Unchanged,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonField {
    pub field: String,
    pub before: ComparisonFact,
    pub after: ComparisonFact,
    pub outcome: ComparisonFieldOutcome,
    pub availability_changed: bool,
    /// False for profile mismatches, incomplete/equal prefixes or unavailable facts.
    pub comparable_complete_values: bool,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonPageKind {
    Added,
    Removed,
    Changed,
    Unchanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonPageReference {
    pub ordinal: usize,
    pub original_requested_url: String,
    pub document_url: String,
    /// These are source provenance, not compared content fields.
    pub fetched_at: String,
    pub rendered_at: Option<String>,
    pub basis: String,
    pub committed_selected_generation: Option<u64>,
    pub parent_snapshot_sha256: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparedSavedPage {
    pub comparison_key: String,
    pub kind: ComparisonPageKind,
    pub before: Option<ComparisonPageReference>,
    pub after: Option<ComparisonPageReference>,
    pub fields: Vec<ComparisonField>,
    /// Equality only for this field profile, never complete website equivalence.
    pub equality_proven_for_compared_fields: bool,
    pub unknown_fields: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComparisonCoverage {
    pub saved_page_count: usize,
    pub pending: usize,
    pub limited: bool,
    pub cancelled: bool,
    pub mixed_age_selected_recrawl: bool,
    pub operation_engine: Option<String>,
    pub operation_coverage: Option<String>,
    pub website_completeness_proven: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComparedSnapshot {
    pub typed_report_sha256: String,
    pub typed_report_sha_profile: String,
    pub typed_report_json_bytes: usize,
    pub caller_binding: Option<ComparisonCallerBinding>,
    pub caller_binding_attested_by_core: bool,
    pub coverage: ComparisonCoverage,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedComparisonCounts {
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub pages_with_unknown_fields: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedComparisonResult {
    pub schema_version: u32,
    pub engine: String,
    pub identity_profile: String,
    pub mapping_profile: String,
    pub structured_data_profile: String,
    pub config: SavedComparisonConfig,
    pub config_sha256: String,
    pub before_snapshot: ComparedSnapshot,
    pub after_snapshot: ComparedSnapshot,
    pub counts: SavedComparisonCounts,
    pub pages: Vec<ComparedSavedPage>,
    pub ignored_runtime_fields: Vec<String>,
    pub website_requests: usize,
    pub vendor_parity: String,
    pub limitations: Vec<String>,
}

struct HashWriter {
    digest: Sha256,
    bytes: usize,
    maximum: usize,
}
impl Write for HashWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .checked_add(buf.len())
            .filter(|next| *next <= self.maximum)
            .ok_or_else(|| io::Error::other("comparison serialized byte budget exceeded"))?;
        self.digest.update(buf);
        self.bytes = next;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn digest_serialized<T: Serialize>(value: &T, maximum: usize) -> CResult<(String, usize)> {
    let mut sink = HashWriter {
        digest: Sha256::new(),
        bytes: 0,
        maximum,
    };
    serde_json::to_writer(&mut sink, value)
        .map_err(|error| fail("serialized_byte_budget", error.to_string()))?;
    Ok((format!("{:x}", sink.digest.finalize()), sink.bytes))
}
fn value<T: Serialize>(input: T) -> CResult<Value> {
    serde_json::to_value(input)
        .map_err(|error| fail("invalid_serializable_fact", error.to_string()))
}

fn check_url(value: &str, maximum: usize) -> CResult<Url> {
    if value.is_empty()
        || value.len() > maximum
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(fail(
            "invalid_requested_identity",
            "URL must be nonempty, bounded and free of surrounding whitespace/control characters",
        ));
    }
    let parsed =
        Url::parse(value).map_err(|error| fail("invalid_requested_identity", error.to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(fail(
            "invalid_requested_identity",
            "Only absolute HTTP(S) URL identities without credentials are supported",
        ));
    }
    Ok(parsed)
}
fn check_origin(origin: &str, maximum: usize) -> CResult<()> {
    let parsed = check_url(origin, maximum)?;
    if parsed.origin().ascii_serialization() != origin
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(fail(
            "invalid_mapping_origin",
            "Mapping origins must use canonical scheme/host/port spelling without a trailing slash, path, query or fragment",
        ));
    }
    Ok(())
}
fn check_prefix(prefix: &str, origin: &str, maximum: usize) -> CResult<()> {
    if !prefix.starts_with('/')
        || (prefix != "/" && prefix.ends_with('/'))
        || prefix.len() > maximum
        || prefix.contains(['?', '#', '\\'])
        || prefix.chars().any(char::is_control)
    {
        return Err(fail(
            "invalid_mapping_path_prefix",
            "Use an absolute path without trailing slash (except /), query, fragment or backslash",
        ));
    }
    let parsed = Url::parse(&format!("{origin}{prefix}"))
        .map_err(|error| fail("invalid_mapping_path_prefix", error.to_string()))?;
    if parsed.path() != prefix {
        return Err(fail(
            "invalid_mapping_path_prefix",
            "Mapping path prefixes must not require URL parser normalization",
        ));
    }
    Ok(())
}
fn prefix_matches(path: &str, prefix: &str) -> bool {
    prefix == "/"
        || path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with('/'))
}
fn validate_config(config: &SavedComparisonConfig) -> CResult<()> {
    if config.engine != SAVED_COMPARISON_ENGINE {
        return Err(fail("unsupported_comparison_engine", &config.engine));
    }
    let ceiling = SavedComparisonLimits::default();
    macro_rules! bound { ($($field:ident),+) => { $(
        if config.limits.$field == 0 || config.limits.$field > ceiling.$field {
            return Err(fail("invalid_comparison_limit", stringify!($field)));
        }
    )+ }; }
    bound!(
        max_pages_per_snapshot,
        max_mappings,
        max_url_bytes,
        max_input_json_bytes,
        max_output_json_bytes
    );
    if config.mappings.len() > config.limits.max_mappings {
        return Err(fail("mapping_count_budget", "Too many URL mapping rules"));
    }
    for mapping in &config.mappings {
        check_origin(&mapping.from_origin, config.limits.max_url_bytes)?;
        check_origin(&mapping.to_origin, config.limits.max_url_bytes)?;
        check_prefix(
            &mapping.from_path_prefix,
            &mapping.from_origin,
            config.limits.max_url_bytes,
        )?;
        check_prefix(
            &mapping.to_path_prefix,
            &mapping.to_origin,
            config.limits.max_url_bytes,
        )?;
    }
    for (index, a) in config.mappings.iter().enumerate() {
        for b in config.mappings.iter().skip(index + 1) {
            if a.side == b.side
                && a.from_origin == b.from_origin
                && (prefix_matches(&a.from_path_prefix, &b.from_path_prefix)
                    || prefix_matches(&b.from_path_prefix, &a.from_path_prefix))
            {
                return Err(fail(
                    "ambiguous_mapping_rules",
                    "Overlapping origin/path rules on the same snapshot side are rejected before joining",
                ));
            }
        }
    }
    Ok(())
}

/// Preserve raw query, fragment and encoded path spelling while mapping explicit join keys.
fn raw_path_and_tail(url: &str) -> CResult<(String, &str)> {
    let scheme_end = url
        .find("://")
        .ok_or_else(|| fail("invalid_requested_identity", "Missing authority separator"))?
        + 3;
    let authority_tail = &url[scheme_end..];
    if authority_tail
        .split(['/', '?', '#'])
        .next()
        .is_none_or(str::is_empty)
    {
        return Err(fail(
            "ambiguous_mapping_identity",
            "Mapped URL has an empty raw authority; parser-normalized host/path cannot supply a join key",
        ));
    }
    let component_start = authority_tail
        .find(['/', '?', '#'])
        .map(|n| scheme_end + n)
        .unwrap_or(url.len());
    let remainder = &url[component_start..];
    let path_end = remainder.find(['?', '#']).unwrap_or(remainder.len());
    let path = &remainder[..path_end];
    Ok((
        if path.is_empty() {
            "/".into()
        } else {
            path.into()
        },
        &remainder[path_end..],
    ))
}
fn join_key(url: &str, side: ComparisonSide, config: &SavedComparisonConfig) -> CResult<String> {
    let parsed = check_url(url, config.limits.max_url_bytes)?;
    let origin = parsed.origin().ascii_serialization();
    let origin_has_mapping = config
        .mappings
        .iter()
        .any(|rule| rule.side == side && rule.from_origin == origin);
    if !origin_has_mapping {
        return Ok(url.to_owned());
    }
    // WHATWG special HTTP URLs treat a literal backslash as a path separator.
    // Raw identity stays byte-exact without mapping. A mapped ambiguous head
    // must be rejected rather than silently losing a requested path segment.
    if url
        .split(['?', '#'])
        .next()
        .is_some_and(|head| head.contains('\\'))
    {
        return Err(fail(
            "ambiguous_mapping_identity",
            "Literal backslash in a mapped HTTP(S) authority/path is ambiguous; no join performed",
        ));
    }
    let (path, tail) = raw_path_and_tail(url)?;
    let mapping = config.mappings.iter().find(|rule| {
        rule.side == side
            && rule.from_origin == origin
            && prefix_matches(&path, &rule.from_path_prefix)
    });
    let Some(rule) = mapping else {
        return Ok(url.to_owned());
    };
    // Remove only the matched prefix's boundary slash. Any further slash is
    // part of the recorded suffix and must survive (/deploy//a -> /site//a).
    let suffix = if rule.from_path_prefix == "/" {
        &path[1..]
    } else {
        path[rule.from_path_prefix.len()..]
            .strip_prefix('/')
            .unwrap_or("")
    };
    let target_path = if rule.to_path_prefix == "/" {
        format!("/{suffix}")
    } else if path == rule.from_path_prefix {
        rule.to_path_prefix.clone()
    } else {
        format!("{}/{suffix}", rule.to_path_prefix)
    };
    let key = format!("{}{target_path}{tail}", rule.to_origin);
    check_url(&key, config.limits.max_url_bytes)?;
    Ok(key)
}
fn index_snapshot(
    report: &CrawlReport,
    side: ComparisonSide,
    config: &SavedComparisonConfig,
) -> CResult<(BTreeMap<String, usize>, Vec<ComparisonCollision>)> {
    if report.pages.len() > config.limits.max_pages_per_snapshot {
        return Err(fail(
            "page_count_budget",
            "Snapshot page count exceeds the declared maximum",
        ));
    }
    let mut keys: BTreeMap<String, Vec<ComparisonIdentityOccurrence>> = BTreeMap::new();
    for (ordinal, page) in report.pages.iter().enumerate() {
        let key = join_key(&page.url, side, config)?;
        keys.entry(key)
            .or_default()
            .push(ComparisonIdentityOccurrence {
                ordinal,
                original_requested_url: page.url.clone(),
            });
    }
    let mut index = BTreeMap::new();
    let mut collisions = Vec::new();
    for (key, occurrences) in keys {
        if occurrences.len() > 1 {
            collisions.push(ComparisonCollision {
                side,
                comparison_key: key,
                occurrences,
            });
        } else {
            index.insert(key, occurrences[0].ordinal);
        }
    }
    Ok((index, collisions))
}

fn validate_binding(binding: Option<&ComparisonCallerBinding>, typed_sha: &str) -> CResult<()> {
    let Some(binding) = binding else {
        return Ok(());
    };
    if binding.generation == Some(0) {
        return Err(fail(
            "invalid_caller_binding",
            "Declared generation must be positive when present",
        ));
    }
    for label in [
        &binding.job_id,
        &binding.sha_profile,
        &binding.origin,
        &binding.basis,
    ]
    .into_iter()
    .flatten()
    {
        if label.is_empty() || label.len() > 256 || label.chars().any(char::is_control) {
            return Err(fail(
                "invalid_caller_binding",
                "Declared labels must have 1..=256 bytes without controls",
            ));
        }
    }
    if binding.snapshot_sha256.is_some() != binding.sha_profile.is_some() {
        return Err(fail(
            "invalid_caller_binding",
            "Declared snapshot hash and its representation profile must appear together",
        ));
    }
    if let Some(hash) = &binding.snapshot_sha256 {
        if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(fail(
                "invalid_caller_binding",
                "Declared snapshot SHA must have 64 hex characters",
            ));
        }
        if binding.sha_profile.as_deref() == Some(COMPARISON_REPORT_HASH_PROFILE)
            && !hash.eq_ignore_ascii_case(typed_sha)
        {
            return Err(fail(
                "typed_snapshot_hash_mismatch",
                "Declared typed-report SHA does not match the actual compared report",
            ));
        }
    }
    Ok(())
}
fn snapshot(
    report: &CrawlReport,
    binding: Option<&ComparisonCallerBinding>,
    sha: String,
    bytes: usize,
) -> ComparedSnapshot {
    ComparedSnapshot {
        typed_report_sha256: sha,
        typed_report_sha_profile: COMPARISON_REPORT_HASH_PROFILE.into(),
        typed_report_json_bytes: bytes,
        caller_binding: binding.cloned(),
        caller_binding_attested_by_core: false,
        coverage: ComparisonCoverage {
            saved_page_count: report.pages.len(),
            pending: report.summary.pending,
            limited: report.summary.limited,
            cancelled: report.summary.cancelled,
            mixed_age_selected_recrawl: report.operation.as_ref().is_some_and(|op| {
                op.engine == SELECTED_RECRAWL_ENGINE && op.coverage == SELECTED_RECRAWL_COVERAGE
            }),
            operation_engine: report.operation.as_ref().map(|op| op.engine.clone()),
            operation_coverage: report.operation.as_ref().map(|op| op.coverage.clone()),
            website_completeness_proven: false,
        },
    }
}

#[derive(Debug, Clone)]
struct PageBasis {
    label: String,
    selected_generation: Option<u64>,
    pending: bool,
    known: bool,
}
fn selected_target_index(
    report: &CrawlReport,
) -> CResult<BTreeMap<&str, &crate::RecrawlTargetObservation>> {
    let mut targets = BTreeMap::new();
    let Some(operation) = &report.operation else {
        return Ok(targets);
    };
    if operation.engine != SELECTED_RECRAWL_ENGINE
        || operation.coverage != SELECTED_RECRAWL_COVERAGE
    {
        return Ok(targets);
    }
    let pages: BTreeSet<_> = report.pages.iter().map(|page| page.url.as_str()).collect();
    for target in &operation.targets {
        if targets
            .insert(target.requested_url.as_str(), target)
            .is_some()
        {
            return Err(fail(
                "ambiguous_selected_target",
                "Selected operation repeats a requested target identity",
            ));
        }
        if !pages.contains(target.requested_url.as_str()) {
            return Err(fail(
                "selected_target_not_in_snapshot",
                "Selected target has no corresponding saved report row",
            ));
        }
        match target.state {
            RecrawlTargetState::Pending if target.committed_generation.is_some() => {
                return Err(fail(
                    "invalid_selected_target",
                    "Pending selected target declares a committed generation",
                ));
            }
            RecrawlTargetState::Committed
                if target.admitted_attempts == 0
                    || target
                        .committed_generation
                        .is_none_or(|generation| generation == 0) =>
            {
                return Err(fail(
                    "invalid_selected_target",
                    "Committed selected target needs positive admission and generation",
                ));
            }
            _ => {}
        }
    }
    Ok(targets)
}
fn page_basis(
    report: &CrawlReport,
    page: &PageRecord,
    targets: &BTreeMap<&str, &crate::RecrawlTargetObservation>,
) -> CResult<PageBasis> {
    let Some(operation) = &report.operation else {
        return Ok(PageBasis {
            label: "recorded_in_crawl".into(),
            selected_generation: None,
            pending: false,
            known: true,
        });
    };
    if operation.engine != SELECTED_RECRAWL_ENGINE
        || operation.coverage != SELECTED_RECRAWL_COVERAGE
    {
        return Ok(PageBasis {
            label: "unknown_operation".into(),
            selected_generation: None,
            pending: false,
            known: false,
        });
    }
    let Some(target) = targets.get(page.url.as_str()) else {
        return Ok(PageBasis {
            label: "inherited_parent".into(),
            selected_generation: None,
            pending: false,
            known: true,
        });
    };
    match target.state {
        RecrawlTargetState::Pending => {
            if target.committed_generation.is_some() {
                return Err(fail(
                    "invalid_selected_target",
                    "Pending selected target declares a committed generation",
                ));
            }
            Ok(PageBasis {
                label: "pending_selected_historical_parent".into(),
                selected_generation: None,
                pending: true,
                known: true,
            })
        }
        RecrawlTargetState::Committed => {
            if target.admitted_attempts == 0
                || target
                    .committed_generation
                    .is_none_or(|generation| generation == 0)
            {
                return Err(fail(
                    "invalid_selected_target",
                    "Committed selected target needs positive admission and generation",
                ));
            }
            Ok(PageBasis {
                label: "refreshed_selected".into(),
                selected_generation: target.committed_generation,
                pending: false,
                known: true,
            })
        }
    }
}
fn reference(report: &CrawlReport, ordinal: usize, basis: &PageBasis) -> ComparisonPageReference {
    let page = &report.pages[ordinal];
    ComparisonPageReference {
        ordinal,
        original_requested_url: page.url.clone(),
        document_url: page.document_url().into(),
        fetched_at: page.fetched_at.clone(),
        rendered_at: page.rendered_at.clone(),
        basis: basis.label.clone(),
        committed_selected_generation: basis.selected_generation,
        parent_snapshot_sha256: report
            .operation
            .as_ref()
            .map(|op| op.parent.snapshot_sha256.clone()),
    }
}

fn unknown(reason: &str) -> ComparisonFact {
    ComparisonFact {
        state: ComparisonFactState::Unknown,
        value: None,
        profile: None,
        reasons: vec![reason.into()],
        provenance: None,
    }
}
fn observed(data: Value) -> ComparisonFact {
    ComparisonFact {
        state: ComparisonFactState::Observed,
        value: Some(data),
        profile: None,
        reasons: Vec::new(),
        provenance: None,
    }
}
fn absent_page() -> ComparisonFact {
    ComparisonFact {
        state: ComparisonFactState::Missing,
        value: None,
        profile: None,
        reasons: vec!["not_in_this_saved_report_cohort".into()],
        provenance: None,
    }
}
fn nullable<T: Serialize>(data: &Option<T>, missing_reason: &str) -> CResult<ComparisonFact> {
    Ok(match data {
        Some(data) => observed(value(data)?),
        None => unknown(missing_reason),
    })
}
fn nonempty_string(data: &str, missing_reason: &str) -> ComparisonFact {
    if data.is_empty() {
        unknown(missing_reason)
    } else {
        observed(json!(data))
    }
}
fn is_empty_value(data: &Value) -> bool {
    matches!(data, Value::Array(rows) if rows.is_empty())
        || matches!(data, Value::String(text) if text.is_empty())
}
/// Empty typed Vec alone is not evidence of an observed empty legacy DOM.
fn html_fact(page: &PageRecord, data: Value) -> ComparisonFact {
    let Some(dom) = &page.dom_metadata else {
        if is_empty_value(&data) || data.is_null() {
            return unknown("html_source_completeness_not_recorded");
        }
        return ComparisonFact {
            state: ComparisonFactState::Partial,
            value: Some(data),
            profile: Some("legacy-declared-html-facts-v1".into()),
            reasons: vec!["html_source_completeness_not_recorded".into()],
            provenance: None,
        };
    };
    let known_profile = dom.engine == "independent_html5_dom_v1"
        && matches!(dom.source.as_str(), "raw_html" | "rendered_html");
    let partial = !known_profile
        || !dom.complete
        || page.truncated
        || page.error.is_some()
        || page.blocked_by_robots;
    let mut reasons = dom.limits_reached.clone();
    if !known_profile {
        reasons.push("unknown_dom_observation_profile".into());
    }
    if !dom.complete {
        reasons.push("dom_observation_incomplete".into());
    }
    if page.truncated {
        reasons.push("http_body_truncated".into());
    }
    if page.error.is_some() {
        reasons.push("source_response_error".into());
    }
    if page.blocked_by_robots {
        reasons.push("source_blocked_by_robots".into());
    }
    ComparisonFact {
        state: if partial {
            ComparisonFactState::Partial
        } else if is_empty_value(&data) {
            ComparisonFactState::ObservedEmpty
        } else {
            ComparisonFactState::Observed
        },
        value: Some(data),
        profile: Some(format!("{}:{}", dom.engine, dom.source)),
        reasons,
        provenance: Some(json!({"source":dom.source,"dom_complete":dom.complete,
            "document_url":page.document_url()})),
    }
}
fn capture_provenance(capture: &PageContentCapture, basis: &PageBasis) -> Value {
    json!({"source_kind":capture.source_kind,"requested_url":capture.requested_url,
        "source_document_url":capture.source_document_url,"fetched_at":capture.fetched_at,
        "rendered_at":capture.rendered_at,"source_revision":capture.source_revision,
        "observed_generation":capture.observed_generation,"basis":basis.label,
        "committed_selected_generation":basis.selected_generation})
}

fn declared_body_text_hash(page: &PageRecord) -> ComparisonFact {
    let Some(hash) = &page.content_hash else {
        return unknown("legacy_content_hash_not_observed");
    };
    let mut fact = html_fact(page, json!(hash));
    fact.profile = fact
        .profile
        .map(|profile| format!("declared-clean-body-text-sha256-v1:{profile}"));
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase())
    {
        fact.state = ComparisonFactState::Partial;
        fact.reasons
            .push("declared_body_text_hash_shape_invalid".into());
    }
    fact
}

struct EffectiveHeaderBasis<'a> {
    profile: &'static str,
    source: &'static str,
    document_url: Option<&'a str>,
    status: Option<u16>,
    inventory_recorded: bool,
}
fn effective_header_basis(page: &PageRecord) -> EffectiveHeaderBasis<'_> {
    // render_page retains native final_url/headers, while JavaScript navigation
    // rebinds effective header fields to rendered_url/rendered_headers.
    if page.rendered {
        if page.renderer_error.is_none()
            && let Some(rendered_url) = page.rendered_url.as_deref()
        {
            if rendered_url != page.final_url {
                return EffectiveHeaderBasis {
                    profile: "declared-rendered-header-inventory-v1",
                    source: "rendered_navigation_response",
                    document_url: Some(rendered_url),
                    status: page.rendered_status,
                    inventory_recorded: !page.rendered_headers.is_empty(),
                };
            }
        } else {
            return EffectiveHeaderBasis {
                profile: "unknown-effective-header-source-v1",
                source: "unknown_rendered_document_identity",
                document_url: None,
                status: None,
                inventory_recorded: false,
            };
        }
    }
    EffectiveHeaderBasis {
        profile: "declared-native-final-header-inventory-v1",
        source: "native_final_response",
        document_url: (!page.final_url.is_empty()).then_some(page.final_url.as_str()),
        status: page.final_status,
        inventory_recorded: !page.headers.is_empty(),
    }
}

fn effective_header_fact(basis: &EffectiveHeaderBasis<'_>, data: Value) -> ComparisonFact {
    let complete =
        basis.document_url.is_some() && basis.status.is_some() && basis.inventory_recorded;
    let mut fact = if complete {
        observed(data)
    } else if !is_empty_value(&data) {
        ComparisonFact {
            state: ComparisonFactState::Partial,
            value: Some(data),
            profile: None,
            reasons: vec!["effective_header_inventory_not_recorded".into()],
            provenance: None,
        }
    } else {
        unknown("effective_header_inventory_not_recorded")
    };
    fact.profile = Some(basis.profile.into());
    fact.provenance = Some(
        json!({"source":basis.source,"document_url":basis.document_url,
        "status":basis.status,"header_inventory_recorded":basis.inventory_recorded}),
    );
    fact
}
fn capture_facts(
    page: &PageRecord,
    basis: &PageBasis,
    source: ContentCaptureSourceKind,
    config: &SavedComparisonConfig,
) -> CResult<Vec<(String, ComparisonFact)>> {
    let rows = page.content_captures.as_deref().unwrap_or_default();
    let mut sources = rows.iter().filter(|capture| capture.source_kind == source);
    let capture = sources.next();
    if sources.next().is_some() {
        return Err(fail(
            "duplicate_saved_text_source",
            "Saved page repeats a captured source kind",
        ));
    }
    let label = match source {
        ContentCaptureSourceKind::RawHttpHtml => "raw_http_html",
        ContentCaptureSourceKind::RenderedDomHtml => "rendered_dom_html",
    };
    let names = [
        "profile",
        "selected_text",
        "html_representation",
        "entity_bytes",
    ];
    let Some(capture) = capture else {
        return Ok(names
            .into_iter()
            .map(|name| {
                let mut fact = unknown("saved_text_source_not_recorded");
                fact.state = ComparisonFactState::Missing;
                (format!("text_sources.{label}.{name}"), fact)
            })
            .collect());
    };
    if let Some(generation) = basis.selected_generation
        && capture.observed_generation != Some(generation)
    {
        return Err(fail(
            "selected_text_generation_mismatch",
            "Saved capture does not belong to the committed selected generation",
        ));
    }
    if basis.pending || !basis.known {
        return Ok(names
            .into_iter()
            .map(|name| {
                let mut fact = unknown(if basis.pending {
                    "selected_target_pending_historical_capture_not_current"
                } else {
                    "unknown_operation_capture_basis"
                });
                fact.provenance = Some(capture_provenance(capture, basis));
                (format!("text_sources.{label}.{name}"), fact)
            })
            .collect());
    }
    let provenance = Some(capture_provenance(capture, basis));
    let result = capture.result.as_ref();
    let profile_value = result.map(|result| json!({"engine":capture.engine,
        "capture_engine":result.engine,"selection_profile":result.selection_profile,
        "whitespace_profile":result.whitespace_profile,"parser_profile":result.parser_profile,
        "unicode_version":result.unicode_version,"html_hash_representation":result.html_hash_representation,
        "source_kind":source,"config":result.config,"config_sha256":capture.config_sha256}));
    let mut profile = profile_value
        .clone()
        .map(observed)
        .unwrap_or_else(|| unknown("capture_profile_unavailable"));
    profile.provenance = provenance.clone();
    let selection_identity = profile_value
        .as_ref()
        .map(|profile| {
            digest_serialized(profile, config.limits.max_input_json_bytes).map(|(hash, _)| hash)
        })
        .transpose()?;
    let state = match capture.status {
        PageContentCaptureStatus::Complete => ComparisonFactState::Observed,
        PageContentCaptureStatus::Empty => ComparisonFactState::ObservedEmpty,
        PageContentCaptureStatus::Incomplete => ComparisonFactState::Partial,
        PageContentCaptureStatus::Unavailable => ComparisonFactState::Unavailable,
        PageContentCaptureStatus::Error => ComparisonFactState::Error,
    };
    let mut reasons = result
        .map(|result| {
            result
                .reasons
                .iter()
                .map(|reason| format!("{:?}", reason.code))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(error) = &capture.failure {
        reasons.push(error.code.clone());
    }
    let selected_value = result.and_then(|result| {
        result.text_sha256.as_ref().map(|hash| {
            json!({"text_sha256":hash,"text_utf8_bytes":result.text.as_ref().map(String::len),
            "text":if config.include_selected_text { result.text.clone() } else { None },
            "text_included":config.include_selected_text,"html_language":result.html_language,
            "html_language_reason":result.html_language_reason})
        })
    });
    let selected = ComparisonFact {
        state,
        value: selected_value,
        profile: selection_identity,
        reasons: reasons.clone(),
        provenance: provenance.clone(),
    };
    let html = ComparisonFact { state: if result.is_none() {
            // observe() can retain a supplied prefix hash even when its bounded
            // selection failed. A hash alone never attests complete HTML.
            if capture.status == PageContentCaptureStatus::Error { ComparisonFactState::Error }
            else { ComparisonFactState::Unknown }
        } else if capture.source_html_utf8_sha256.is_none() {
            if state == ComparisonFactState::Error { state } else { ComparisonFactState::Unavailable }
        } else if result.is_some_and(|result| !result.source_complete) {
            ComparisonFactState::Partial
        } else { ComparisonFactState::Observed },
        value: capture.source_html_utf8_sha256.as_ref().map(|hash|
            json!({"html_utf8_sha256":hash,"html_utf8_bytes":capture.source_html_utf8_bytes})),
        profile: result.map(|result| result.html_hash_representation.clone()),
        reasons: reasons.clone(), provenance: provenance.clone() };
    let entity = ComparisonFact { state: if capture.encoded_body_sha256.is_none() {
            ComparisonFactState::Unavailable
        } else if capture.encoded_body_complete == Some(true) { ComparisonFactState::Observed }
        else { ComparisonFactState::Partial },
        value: capture.encoded_body_sha256.as_ref().map(|hash| json!({"encoded_body_sha256":hash,
            "encoded_body_bytes":capture.encoded_body_bytes,"decode_encoding":capture.decode_encoding})),
        profile: Some("reqwest-decoded-entity-bytes-v1".into()), reasons, provenance };
    Ok(names
        .into_iter()
        .zip([profile, selected, html, entity])
        .map(|(name, fact)| (format!("text_sources.{label}.{name}"), fact))
        .collect())
}

fn project_page(
    page: &PageRecord,
    basis: &PageBasis,
    config: &SavedComparisonConfig,
) -> CResult<BTreeMap<String, ComparisonFact>> {
    validate_page_content_captures(page)
        .map_err(|error| fail("invalid_saved_text_binding", error.to_string()))?;
    let mut facts = BTreeMap::new();
    macro_rules! declared { ($($field:ident),+) => { $(
        facts.insert(stringify!($field).into(), observed(value(&page.$field)?));
    )+ }; }
    declared!(
        error,
        blocked_by_robots,
        truncated,
        redirect_chain,
        rendered,
        renderer_error
    );
    // Empty serde-default flags/arrays do not establish a robots/indexability run.
    let evaluation_basis = !page.indexability_agent.is_empty()
        && (page.dom_metadata.is_some()
            || (page.final_status.is_some() && !page.content_type.is_empty()));
    // Metadata extraction evaluates robots but does not run set_indexability.
    // The latter records true or false with reasons; false without reasons can
    // therefore still be a serde default even with modern parsed DOM metadata.
    let indexability_basis =
        evaluation_basis && (page.indexable || !page.indexability_reasons.is_empty());
    let evaluation_partial = page.dom_metadata.as_ref().is_some_and(|dom| !dom.complete)
        || page.truncated
        || page.error.is_some();
    for (name, data) in [
        ("indexable", json!(page.indexable)),
        ("indexability_reasons", value(&page.indexability_reasons)?),
        ("effective_robots", value(&page.effective_robots)?),
    ] {
        let fact_basis = if name == "effective_robots" {
            evaluation_basis
        } else {
            indexability_basis
        };
        let mut fact = if fact_basis {
            observed(data)
        } else if evaluation_basis {
            unknown("indexability_false_without_evaluation_reasons")
        } else {
            unknown("indexability_evaluation_basis_not_recorded")
        };
        if fact_basis {
            fact.profile = Some(format!(
                "declared-indexability-agent:{}",
                page.indexability_agent
            ));
            if evaluation_partial {
                fact.state = ComparisonFactState::Partial;
                fact.reasons
                    .push("source_evaluation_may_be_incomplete".into());
            }
        }
        facts.insert(name.into(), fact);
    }
    facts.insert(
        "indexability_agent".into(),
        nonempty_string(&page.indexability_agent, "indexability_agent_not_recorded"),
    );
    facts.insert(
        "status".into(),
        nullable(&page.status, "initial_status_not_observed")?,
    );
    facts.insert(
        "final_status".into(),
        nullable(&page.final_status, "final_status_not_observed")?,
    );
    facts.insert(
        "final_url".into(),
        nonempty_string(&page.final_url, "final_document_url_not_observed"),
    );
    facts.insert(
        "content_type".into(),
        nonempty_string(&page.content_type, "content_type_not_observed"),
    );
    facts.insert(
        "rendered_url".into(),
        nullable(&page.rendered_url, "rendered_document_not_observed")?,
    );
    facts.insert(
        "rendered_status".into(),
        nullable(&page.rendered_status, "rendered_status_not_observed")?,
    );
    facts.insert("content_hash".into(), declared_body_text_hash(page));
    facts.insert("word_count".into(), html_fact(page, json!(page.word_count)));
    macro_rules! html { ($($field:ident),+) => { $(
        facts.insert(stringify!($field).into(), html_fact(page, value(&page.$field)?));
    )+ }; }
    html!(
        titles,
        meta_descriptions,
        h1,
        h2,
        canonical,
        html_canonical,
        meta_robots,
        html_hreflang,
        link_relationships,
        pagination_next,
        pagination_prev,
        amphtml,
        language,
        viewport,
        images,
        links,
        structured_data
    );
    let mut extraction = html_fact(page, value(&page.custom_extractions)?);
    if page
        .custom_extractions
        .iter()
        .any(|row| row.error.is_some())
    {
        extraction.state = ComparisonFactState::Error;
        extraction.reasons.push("custom_extraction_error".into());
    } else if page.custom_extractions.iter().any(|row| {
        row.truncated
            || row.matched_count.is_none()
            || row.engine.is_empty()
            || row.source.is_empty()
    }) {
        extraction.state = ComparisonFactState::Partial;
        extraction
            .reasons
            .push("custom_extraction_truncated_or_completeness_unknown".into());
    }
    facts.insert("custom_extractions".into(), extraction);
    // Effective/header declarations follow the corresponding native or navigated
    // response inventory, not a predecessor's unrelated headers or DOM-empty proof.
    let header_basis = effective_header_basis(page);
    for (name, data) in [
        ("header_canonical", value(&page.header_canonical)?),
        ("header_hreflang", value(&page.header_hreflang)?),
        ("x_robots_tag", value(&page.x_robots_tag)?),
        ("header_link_values", value(&page.header_link_values)?),
    ] {
        facts.insert(name.into(), effective_header_fact(&header_basis, data));
    }
    facts.insert(
        "invalid_structured_data".into(),
        html_fact(page, json!(page.invalid_structured_data)),
    );
    if page.invalid_structured_data > 0
        && let Some(fact) = facts.get_mut("structured_data")
    {
        fact.state = ComparisonFactState::Partial;
        fact.reasons.push("invalid_jsonld_blocks_not_parsed".into());
    }
    for source in [
        ContentCaptureSourceKind::RawHttpHtml,
        ContentCaptureSourceKind::RenderedDomHtml,
    ] {
        facts.extend(capture_facts(page, basis, source, config)?);
    }
    Ok(facts)
}

fn complete_fact(fact: &ComparisonFact) -> bool {
    matches!(
        fact.state,
        ComparisonFactState::Observed | ComparisonFactState::ObservedEmpty
    ) && fact.value.is_some()
}
fn compare_fact(field: String, before: ComparisonFact, after: ComparisonFact) -> ComparisonField {
    let availability_changed = before.state != after.state;
    let compatible_profile = before.profile == after.profile;
    let comparable_complete_values =
        complete_fact(&before) && complete_fact(&after) && compatible_profile;
    let both_values = before.value.is_some() && after.value.is_some();
    let (outcome, reason) = if !compatible_profile && both_values {
        (
            ComparisonFieldOutcome::Unknown,
            Some("different_observation_profiles".into()),
        )
    } else if availability_changed {
        (
            ComparisonFieldOutcome::Changed,
            Some("observation_availability_changed_not_content_equivalence".into()),
        )
    } else if both_values && before.value != after.value {
        (
            ComparisonFieldOutcome::Changed,
            (!comparable_complete_values)
                .then(|| "observed_partial_values_differ_complete_source_unknown".into()),
        )
    } else if comparable_complete_values {
        (ComparisonFieldOutcome::Unchanged, None)
    } else {
        (
            ComparisonFieldOutcome::Unknown,
            Some("complete_comparable_values_unavailable".into()),
        )
    };
    ComparisonField {
        field,
        before,
        after,
        outcome,
        availability_changed,
        comparable_complete_values,
        reason,
    }
}

/// Strict, bounded and pure. Failure returns no partial result and no truncated cohort.
pub fn compare_saved_crawls(
    before: &CrawlReport,
    after: &CrawlReport,
    config: &SavedComparisonConfig,
    before_binding: Option<&ComparisonCallerBinding>,
    after_binding: Option<&ComparisonCallerBinding>,
) -> CResult<SavedComparisonResult> {
    validate_config(config)?;
    let (before_sha, before_bytes) = digest_serialized(before, config.limits.max_input_json_bytes)?;
    let (after_sha, after_bytes) = digest_serialized(
        after,
        config
            .limits
            .max_input_json_bytes
            .checked_sub(before_bytes)
            .ok_or_else(|| fail("serialized_byte_budget", "Combined input budget exceeded"))?,
    )?;
    validate_binding(before_binding, &before_sha)?;
    validate_binding(after_binding, &after_sha)?;
    let (old, mut collisions) = index_snapshot(before, ComparisonSide::Before, config)?;
    let (new, more_collisions) = index_snapshot(after, ComparisonSide::After, config)?;
    collisions.extend(more_collisions);
    if !collisions.is_empty() {
        return Err(ComparisonError { code: "ambiguous_snapshot_identity".into(),
            message: "Duplicate original URLs or mapped keys collide within a snapshot; no join was performed".into(), collisions });
    }
    let old_targets = selected_target_index(before)?;
    let new_targets = selected_target_index(after)?;
    let keys: BTreeSet<_> = old.keys().chain(new.keys()).cloned().collect();
    let mut pages = Vec::with_capacity(keys.len());
    let mut counts = SavedComparisonCounts::default();
    for key in keys {
        let old_ordinal = old.get(&key).copied();
        let new_ordinal = new.get(&key).copied();
        let old_basis = old_ordinal
            .map(|ordinal| page_basis(before, &before.pages[ordinal], &old_targets))
            .transpose()?;
        let new_basis = new_ordinal
            .map(|ordinal| page_basis(after, &after.pages[ordinal], &new_targets))
            .transpose()?;
        let old_facts = old_ordinal
            .zip(old_basis.as_ref())
            .map(|(ordinal, basis)| project_page(&before.pages[ordinal], basis, config))
            .transpose()?
            .unwrap_or_default();
        let new_facts = new_ordinal
            .zip(new_basis.as_ref())
            .map(|(ordinal, basis)| project_page(&after.pages[ordinal], basis, config))
            .transpose()?
            .unwrap_or_default();
        let field_keys: BTreeSet<_> = old_facts.keys().chain(new_facts.keys()).cloned().collect();
        let fields: Vec<_> = field_keys
            .into_iter()
            .map(|field| {
                let a = old_facts.get(&field).cloned().unwrap_or_else(absent_page);
                let b = new_facts.get(&field).cloned().unwrap_or_else(absent_page);
                compare_fact(field, a, b)
            })
            .collect();
        let kind = match (old_ordinal, new_ordinal) {
            (None, Some(_)) => ComparisonPageKind::Added,
            (Some(_), None) => ComparisonPageKind::Removed,
            (Some(_), Some(_))
                if fields.iter().any(|field| {
                    field.outcome == ComparisonFieldOutcome::Changed || field.availability_changed
                }) =>
            {
                ComparisonPageKind::Changed
            }
            _ => ComparisonPageKind::Unchanged,
        };
        match kind {
            ComparisonPageKind::Added => counts.added += 1,
            ComparisonPageKind::Removed => counts.removed += 1,
            ComparisonPageKind::Changed => counts.changed += 1,
            ComparisonPageKind::Unchanged => counts.unchanged += 1,
        }
        let unknown_fields: Vec<_> = fields
            .iter()
            .filter(|field| {
                field.outcome == ComparisonFieldOutcome::Unknown
                    || !field.comparable_complete_values
            })
            .map(|field| field.field.clone())
            .collect();
        if !unknown_fields.is_empty() {
            counts.pages_with_unknown_fields += 1;
        }
        let fresh_basis = old_basis
            .as_ref()
            .is_some_and(|basis| basis.known && !basis.pending)
            && new_basis
                .as_ref()
                .is_some_and(|basis| basis.known && !basis.pending);
        let active_binding = [before_binding, after_binding]
            .into_iter()
            .flatten()
            .any(|binding| binding.active_checkpoint == Some(true));
        pages.push(ComparedSavedPage {
            comparison_key: key,
            kind,
            before: old_ordinal
                .zip(old_basis.as_ref())
                .map(|(ordinal, basis)| reference(before, ordinal, basis)),
            after: new_ordinal
                .zip(new_basis.as_ref())
                .map(|(ordinal, basis)| reference(after, ordinal, basis)),
            equality_proven_for_compared_fields: kind == ComparisonPageKind::Unchanged
                && unknown_fields.is_empty()
                && fresh_basis
                && !active_binding,
            fields,
            unknown_fields,
        });
    }
    let (config_sha256, _) = digest_serialized(config, config.limits.max_input_json_bytes)?;
    let result = SavedComparisonResult { schema_version: 1, engine: SAVED_COMPARISON_ENGINE.into(),
        identity_profile: COMPARISON_IDENTITY_PROFILE.into(), mapping_profile: COMPARISON_MAPPING_PROFILE.into(),
        structured_data_profile: COMPARISON_JSON_PROFILE.into(), config: config.clone(), config_sha256,
        before_snapshot: snapshot(before, before_binding, before_sha, before_bytes),
        after_snapshot: snapshot(after, after_binding, after_sha, after_bytes), counts, pages,
        ignored_runtime_fields: ["fetched_at", "rendered_at", "response_ms", "source_revision",
            "observed_generation", "summary.started_at", "summary.finished_at", "summary.elapsed_ms"]
            .into_iter().map(String::from).collect(), website_requests: 0, vendor_parity: "NOT_PROVEN".into(),
        limitations: vec![
            "Membership classes describe these saved report cohorts, not deletion/addition on the complete website.".into(),
            "Unchanged means no comparable recorded value difference; inspect unknown_fields before inferring equality.".into(),
            "Mapping changes requested-URL join keys only; original final URLs, graph targets and source facts remain unchanged.".into(),
            "JSON-LD comparison is structural JSON/block/array equality, not RDF graph equivalence or schema eligibility.".into(),
            "Link comparison retains recorded occurrence order and duplicates; no complete resource-ledger graph is inferred.".into(),
            "Pure Core checks typed-report/content-binding consistency; caller SQL generations/origin/exact-file hashes are declarations.".into(),
            "No full HTML/DOM artifact, screenshot, provider, cookie, PDF, accessibility, policy or resource-ledger delta is inferred.".into(),
        ] };
    digest_serialized(&result, config.limits.max_output_json_bytes)?;
    Ok(result)
}

#[cfg(test)]
#[path = "comparison_tests.rs"]
mod tests;
