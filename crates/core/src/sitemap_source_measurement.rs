//! Pure validation/projection of caller-supplied captured source measurements.
//! No scanner, HTTP, persistence, clock, membership admission, or authenticity claim.
//! Count/byte ceilings are not complete XSD or search-engine acceptance validation.
use crate::{
    SitemapDocumentKind, SitemapEntityEncoding, SitemapNamespaceProfile, SitemapSourceKind,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Write};

pub const SITEMAP_SOURCE_MEASUREMENT_ENGINE: &str = "open-seo-spider/sitemap-source-measurement-v1";
pub const SITEMAP_SOURCE_LIMIT_ENGINE: &str = "open-seo-spider/sitemap-source-limits-v1";
pub const SITEMAP_SOURCE_XML_PROFILE: &str = "ascii-qname-xml10-no-dtd-v1";
pub const SITEMAP_SOURCE_LOC_PROFILE: &str = "absolute-http-xml-loc-v1";
pub const SITEMAP_SOURCE_HTTP_PROFILE: &str = "reqwest-post-content-decoding-v1";
pub const SITEMAP_PROTOCOL_ENTRY_LIMIT: u64 = 50_000;
pub const SITEMAP_PROTOCOL_XML_BYTE_LIMIT: u64 = 52_428_800;
const MAX_RECORD_BYTES: usize = 16_384;
const MAX_IDENTITY_BYTES: usize = 8192;
// Compatible with future SQLite integer storage; arithmetic is still checked.
const MAX_SCALAR: u64 = i64::MAX as u64;
type Result<T> = std::result::Result<T, SitemapSourceMeasurementFailure>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SitemapSourceMeasurementLimits {
    pub max_record_json_bytes: usize,
    pub max_identity_utf8_bytes: usize,
}
impl Default for SitemapSourceMeasurementLimits {
    fn default() -> Self {
        Self {
            max_record_json_bytes: MAX_RECORD_BYTES,
            max_identity_utf8_bytes: MAX_IDENTITY_BYTES,
        }
    }
}
impl SitemapSourceMeasurementLimits {
    pub fn validate(&self) -> Result<()> {
        require(
            self.max_record_json_bytes > 0
                && self.max_record_json_bytes <= MAX_RECORD_BYTES
                && self.max_identity_utf8_bytes > 0
                && self.max_identity_utf8_bytes <= MAX_IDENTITY_BYTES,
            SitemapSourceMeasurementFailureCode::InvalidLimits,
        )
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapSourceMeasurementFailureCode {
    InvalidLimits,
    JsonBytesLimit,
    InvalidJson,
    UnrecognizedProfile,
    InvalidBinding,
    InvalidIdentity,
    ScalarOutOfRange,
    InvalidQuantity,
    InvalidTransport,
    InvalidDecode,
    InvalidScan,
    CounterInconsistency,
    ArithmeticOverflow,
    MeasurementIdMismatch,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceMeasurementFailure {
    pub code: SitemapSourceMeasurementFailureCode,
}
impl std::fmt::Display for SitemapSourceMeasurementFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sitemap source measurement {:?}", self.code)
    }
}
impl std::error::Error for SitemapSourceMeasurementFailure {}
fn error(code: SitemapSourceMeasurementFailureCode) -> SitemapSourceMeasurementFailure {
    SitemapSourceMeasurementFailure { code }
}
fn require(value: bool, code: SitemapSourceMeasurementFailureCode) -> Result<()> {
    if value { Ok(()) } else { Err(error(code)) }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapMeasuredPrecision {
    Exact,
    AtLeast,
    Unavailable,
}
/// Precision concerns the FULL named source quantity. AtLeast is a measured
/// prefix's lower bound; unavailable is never a known zero.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapMeasuredValue {
    pub value: Option<u64>,
    pub precision: SitemapMeasuredPrecision,
}
impl SitemapMeasuredValue {
    pub fn exact(value: u64) -> Self {
        Self {
            value: Some(value),
            precision: SitemapMeasuredPrecision::Exact,
        }
    }
    pub fn at_least(value: u64) -> Self {
        Self {
            value: Some(value),
            precision: SitemapMeasuredPrecision::AtLeast,
        }
    }
    pub fn unavailable() -> Self {
        Self {
            value: None,
            precision: SitemapMeasuredPrecision::Unavailable,
        }
    }
    fn validate(self) -> Result<()> {
        require(
            matches!(
                (self.precision, self.value),
                (SitemapMeasuredPrecision::Unavailable, None)
                    | (
                        SitemapMeasuredPrecision::Exact | SitemapMeasuredPrecision::AtLeast,
                        Some(_)
                    )
            ),
            SitemapSourceMeasurementFailureCode::InvalidQuantity,
        )?;
        if let Some(value) = self.value {
            scalar(value)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceMeasurementBinding {
    pub crawl_config_sha256: String,
    pub run_generation: u64,
    pub source_kind: SitemapSourceKind,
    /// Original requested task identity; never a final/rewritten alias.
    pub requested_task_url: String,
    /// Expected legacy ID only. It does not assert a committed Document exists.
    pub expected_source_observation_id: String,
    pub source_revision: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceTransportMeasurement {
    pub http_decode_profile: String,
    pub request_started: bool,
    pub requested_url: Option<String>,
    pub final_url: Option<String>,
    pub initial_status: Option<u16>,
    pub final_status: Option<u16>,
    pub blocked_by_robots: bool,
    pub body_complete: bool,
    /// Bytes actually returned after HTTP content decoding, including unretained
    /// received bytes. Not Content-Length, wire bytes, or a gzip ISIZE value.
    pub post_http_received_bytes: SitemapMeasuredValue,
    pub retained_post_http_prefix_bytes: u64,
    pub retained_post_http_prefix_sha256: Option<String>,
    pub complete_post_http_entity_sha256: Option<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapSourceDecodeIntegrity {
    Complete,
    Failed,
    Incomplete,
    NotObserved,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapSourceMeasurementStop {
    RequestNotStarted,
    RobotsDenied,
    HttpStatusNot200,
    TransportIncomplete,
    EntityByteLimit,
    DecodedByteLimit,
    InvalidGzip,
    TrailingGzipBytes,
    UnsupportedNestedGzip,
    InvalidUtf8,
    IllegalXmlCharacter,
    XmlSyntax,
    DtdDenied,
    EventLimit,
    DepthLimit,
    FieldLimit,
    ContextUnavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceDecodeMeasurement {
    pub encoding: Option<SitemapEntityEncoding>,
    /// Exact only at actual full-source decode EOF, including all file-gzip
    /// members/trailers/checksums. Failed/incomplete output is AtLeast.
    pub decoded_xml_bytes: SitemapMeasuredValue,
    pub decoded_prefix_sha256: Option<String>,
    pub decoded_complete_sha256: Option<String>,
    pub integrity: SitemapSourceDecodeIntegrity,
    pub stop_code: Option<SitemapSourceMeasurementStop>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapSourceScanStatus {
    CompleteProfile,
    Failed,
    NotObserved,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceEntryMeasurements {
    pub immediate_entry_starts: SitemapMeasuredValue,
    pub completed_immediate_entries: SitemapMeasuredValue,
    /// Actual closed immediate document-namespace loc children. Includes invalid
    /// URL values, excludes synthetic missing-loc diagnostic rows.
    pub actual_document_namespace_loc_occurrences: SitemapMeasuredValue,
    /// Completed entries with exactly one eligible original Loc under the
    /// recorded lexical profile. Same URL in distinct entries counts twice.
    pub protocol_eligible_completed_entries: SitemapMeasuredValue,
    /// Disjoint classifications of completed entries in the SAME scan prefix.
    pub missing_loc_entries: u64,
    pub multiple_loc_entries: u64,
    pub invalid_single_loc_entries: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceScanMeasurement {
    pub xml_profile: String,
    pub loc_lexical_profile: String,
    pub document_kind: Option<SitemapDocumentKind>,
    pub namespace_profile: Option<SitemapNamespaceProfile>,
    pub status: SitemapSourceScanStatus,
    pub stop_code: Option<SitemapSourceMeasurementStop>,
    pub events_observed: SitemapMeasuredValue,
    pub counts: SitemapSourceEntryMeasurements,
}
/// Recorded policies are metadata, not claims that retained bytes equal total
/// source bytes. Future bounded collectors can supply larger measured facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceCaptureBounds {
    pub max_retained_post_http_bytes: u64,
    pub max_retained_xml_bytes: u64,
    pub max_xml_events: u64,
    pub max_xml_depth: u64,
    pub max_field_utf8_bytes: u64,
    pub max_retained_entries: u64,
    pub max_retained_location_occurrences: u64,
    pub max_retained_text_utf8_bytes: u64,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapMembershipRetention {
    Complete,
    Incomplete,
    Unobserved,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceMeasurementPayload {
    pub schema_version: u32,
    pub engine: String,
    pub binding: SitemapSourceMeasurementBinding,
    pub transport: SitemapSourceTransportMeasurement,
    pub decode: SitemapSourceDecodeMeasurement,
    pub scan: SitemapSourceScanMeasurement,
    pub membership_retention: SitemapMembershipRetention,
    pub bounds: SitemapSourceCaptureBounds,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceMeasurement {
    pub measurement_id: String,
    pub payload: SitemapSourceMeasurementPayload,
}
impl SitemapSourceMeasurement {
    /// Seals a NEW caller-supplied observation; no field or URL is normalized,
    /// repaired or inferred. The only added value is its content address.
    pub fn new(
        payload: SitemapSourceMeasurementPayload,
        limits: &SitemapSourceMeasurementLimits,
    ) -> Result<Self> {
        limits.validate()?;
        validate_payload(&payload, limits)?;
        let mut observation = Self {
            measurement_id: "0".repeat(64),
            payload,
        };
        bounded_json_size(&observation, limits.max_record_json_bytes)?;
        observation.measurement_id = payload_id(&observation.payload)?;
        Ok(observation)
    }
}
fn scalar(value: u64) -> Result<()> {
    require(
        value <= MAX_SCALAR,
        SitemapSourceMeasurementFailureCode::ScalarOutOfRange,
    )
}
fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn optional_hash(value: &Option<String>) -> Result<()> {
    require(
        value.as_deref().is_none_or(hash),
        SitemapSourceMeasurementFailureCode::InvalidQuantity,
    )
}
fn empty_hash_if_zero(bytes: u64, value: &Option<String>) -> Result<()> {
    if bytes == 0 {
        let empty = format!("{:x}", Sha256::digest(b""));
        require(
            value.as_deref().is_none_or(|value| value == empty.as_str()),
            SitemapSourceMeasurementFailureCode::InvalidQuantity,
        )?;
    }
    Ok(())
}
fn bounded_identity(value: &str, limits: &SitemapSourceMeasurementLimits) -> Result<()> {
    // Legacy source IDs bind original bounded strings, not protocol-eligible
    // Loc lexemes. Preserve actual admitted identities, including percent forms
    // that the existing URL normalizer accepts. No canonicalization or repair.
    require(
        !value.is_empty() && value.len() <= limits.max_identity_utf8_bytes,
        SitemapSourceMeasurementFailureCode::InvalidIdentity,
    )
}
fn validate_payload(
    p: &SitemapSourceMeasurementPayload,
    limits: &SitemapSourceMeasurementLimits,
) -> Result<()> {
    use SitemapSourceMeasurementFailureCode as F;
    require(
        p.schema_version == 1
            && p.engine == SITEMAP_SOURCE_MEASUREMENT_ENGINE
            && p.transport.http_decode_profile == SITEMAP_SOURCE_HTTP_PROFILE
            && p.scan.xml_profile == SITEMAP_SOURCE_XML_PROFILE
            && p.scan.loc_lexical_profile == SITEMAP_SOURCE_LOC_PROFILE,
        F::UnrecognizedProfile,
    )?;
    let b = &p.binding;
    require(
        b.source_kind == SitemapSourceKind::SitemapXml
            && hash(&b.crawl_config_sha256)
            && hash(&b.expected_source_observation_id)
            && b.run_generation > 0,
        F::InvalidBinding,
    )?;
    scalar(b.run_generation)?;
    bounded_identity(&b.requested_task_url, limits)?;
    bounded_identity(&b.source_revision, limits)?;
    let expected = crate::sitemap_ledger::source_observation_id(
        b.source_kind,
        &b.requested_task_url,
        b.run_generation,
        &b.source_revision,
    )
    .map_err(|_| error(F::InvalidBinding))?;
    require(
        expected == b.expected_source_observation_id,
        F::InvalidBinding,
    )?;
    for value in [
        p.bounds.max_retained_post_http_bytes,
        p.bounds.max_retained_xml_bytes,
        p.bounds.max_xml_events,
        p.bounds.max_xml_depth,
        p.bounds.max_field_utf8_bytes,
        p.bounds.max_retained_entries,
        p.bounds.max_retained_location_occurrences,
        p.bounds.max_retained_text_utf8_bytes,
    ] {
        scalar(value)?;
        require(value > 0, F::InvalidLimits)?;
    }
    validate_transport(p, limits)?;
    validate_decode(p)?;
    validate_scan(p)?;
    Ok(())
}
fn validate_transport(
    p: &SitemapSourceMeasurementPayload,
    limits: &SitemapSourceMeasurementLimits,
) -> Result<()> {
    use SitemapMeasuredPrecision as P;
    use SitemapSourceMeasurementFailureCode as F;
    let t = &p.transport;
    t.post_http_received_bytes.validate()?;
    scalar(t.retained_post_http_prefix_bytes)?;
    require(
        t.retained_post_http_prefix_bytes <= p.bounds.max_retained_post_http_bytes,
        F::InvalidTransport,
    )?;
    for value in [&t.requested_url, &t.final_url].into_iter().flatten() {
        bounded_identity(value, limits)?;
    }
    require(
        t.requested_url
            .as_deref()
            .is_none_or(|url| url == p.binding.requested_task_url),
        F::InvalidBinding,
    )?;
    for status in [t.initial_status, t.final_status].into_iter().flatten() {
        require((100..=599).contains(&status), F::InvalidTransport)?;
    }
    require(
        t.final_url.is_some() == t.final_status.is_some(),
        F::InvalidTransport,
    )?;
    optional_hash(&t.retained_post_http_prefix_sha256)?;
    optional_hash(&t.complete_post_http_entity_sha256)?;
    empty_hash_if_zero(
        t.retained_post_http_prefix_bytes,
        &t.retained_post_http_prefix_sha256,
    )?;
    if let Some(received) = t.post_http_received_bytes.value {
        empty_hash_if_zero(received, &t.complete_post_http_entity_sha256)?;
    }
    if !t.request_started {
        require(
            !t.body_complete
                && t.initial_status.is_none()
                && t.final_status.is_none()
                && t.post_http_received_bytes.precision == P::Unavailable
                && t.retained_post_http_prefix_bytes == 0
                && t.retained_post_http_prefix_sha256.is_none()
                && t.complete_post_http_entity_sha256.is_none(),
            F::InvalidTransport,
        )?;
    }
    match t.post_http_received_bytes.value {
        Some(received) => require(
            received >= t.retained_post_http_prefix_bytes,
            F::InvalidTransport,
        )?,
        None => require(
            t.retained_post_http_prefix_bytes == 0 && t.retained_post_http_prefix_sha256.is_none(),
            F::InvalidTransport,
        )?,
    }
    if t.body_complete {
        require(
            t.request_started
                && !t.blocked_by_robots
                && t.final_status.is_some()
                && t.requested_url.is_some()
                && t.post_http_received_bytes.precision == P::Exact
                && t.complete_post_http_entity_sha256.is_some(),
            F::InvalidTransport,
        )?;
        if t.post_http_received_bytes.value == Some(t.retained_post_http_prefix_bytes)
            && t.retained_post_http_prefix_sha256.is_some()
        {
            require(
                t.retained_post_http_prefix_sha256 == t.complete_post_http_entity_sha256,
                F::InvalidTransport,
            )?;
        }
    } else {
        require(
            t.post_http_received_bytes.precision != P::Exact
                && t.complete_post_http_entity_sha256.is_none(),
            F::InvalidTransport,
        )?;
    }
    Ok(())
}
fn http_source_observed(t: &SitemapSourceTransportMeasurement) -> bool {
    t.request_started
        && t.requested_url.is_some()
        && t.final_url.is_some()
        && t.final_status == Some(200)
        && !t.blocked_by_robots
}
fn stop_matches_transport(
    stop: Option<SitemapSourceMeasurementStop>,
    t: &SitemapSourceTransportMeasurement,
) -> bool {
    use SitemapSourceMeasurementStop as S;
    match stop {
        Some(S::RequestNotStarted) => !t.request_started,
        Some(S::RobotsDenied) => t.blocked_by_robots,
        Some(S::HttpStatusNot200) => t.final_status.is_some_and(|status| status != 200),
        Some(S::TransportIncomplete) => !t.body_complete,
        _ => true,
    }
}
fn validate_decode(p: &SitemapSourceMeasurementPayload) -> Result<()> {
    use SitemapMeasuredPrecision as P;
    use SitemapSourceDecodeIntegrity as I;
    use SitemapSourceMeasurementFailureCode as F;
    let d = &p.decode;
    require(
        stop_matches_transport(d.stop_code, &p.transport),
        F::InvalidDecode,
    )?;
    d.decoded_xml_bytes.validate()?;
    optional_hash(&d.decoded_prefix_sha256)?;
    optional_hash(&d.decoded_complete_sha256)?;
    if let Some(bytes) = d.decoded_xml_bytes.value {
        empty_hash_if_zero(bytes, &d.decoded_prefix_sha256)?;
        empty_hash_if_zero(bytes, &d.decoded_complete_sha256)?;
    }
    // The identity XML-budget preflight sniffs its encoding before declining
    // to emit any XML bytes. Preserve that known encoding, not an invented
    // output extent. Unstarted/denied sources cannot satisfy this context.
    let known_identity_preflight = d.encoding == Some(SitemapEntityEncoding::Identity)
        && d.stop_code == Some(SitemapSourceMeasurementStop::DecodedByteLimit)
        && p.transport.request_started
        && !p.transport.blocked_by_robots
        && p.transport.post_http_received_bytes.value.is_some()
        && p.transport.retained_post_http_prefix_bytes > p.bounds.max_retained_xml_bytes;
    match d.integrity {
        I::NotObserved => require(
            (d.encoding.is_none() || known_identity_preflight)
                && d.decoded_xml_bytes.precision == P::Unavailable
                && d.decoded_prefix_sha256.is_none()
                && d.decoded_complete_sha256.is_none()
                && d.stop_code.is_some(),
            F::InvalidDecode,
        )?,
        I::Complete => require(
            http_source_observed(&p.transport)
                && p.transport.body_complete
                && d.encoding.is_some()
                && d.decoded_xml_bytes.precision == P::Exact
                && d.decoded_complete_sha256.is_some()
                && d.decoded_prefix_sha256 == d.decoded_complete_sha256
                && d.stop_code.is_none(),
            F::InvalidDecode,
        )?,
        I::Failed | I::Incomplete => require(
            p.transport.request_started
                && p.transport.post_http_received_bytes.value.is_some()
                && d.encoding.is_some()
                && d.decoded_xml_bytes.precision != P::Exact
                && d.decoded_complete_sha256.is_none()
                && d.stop_code.is_some(),
            F::InvalidDecode,
        )?,
    }
    use SitemapSourceMeasurementStop as Stop;
    let valid_stop = match d.integrity {
        I::Complete => d.stop_code.is_none(),
        I::NotObserved => matches!(
            d.stop_code,
            Some(
                Stop::RequestNotStarted
                    | Stop::RobotsDenied
                    | Stop::HttpStatusNot200
                    | Stop::TransportIncomplete
                    | Stop::EntityByteLimit
                    | Stop::DecodedByteLimit
                    | Stop::ContextUnavailable
            )
        ),
        I::Failed => matches!(
            d.stop_code,
            Some(Stop::InvalidGzip | Stop::TrailingGzipBytes | Stop::UnsupportedNestedGzip)
        ),
        I::Incomplete => {
            matches!(
                d.stop_code,
                Some(
                    Stop::EntityByteLimit
                        | Stop::DecodedByteLimit
                        | Stop::TransportIncomplete
                        | Stop::ContextUnavailable
                )
            ) || (d.stop_code == Some(Stop::InvalidGzip)
                && d.encoding == Some(SitemapEntityEncoding::GzipFileEntity))
        }
    };
    require(valid_stop, F::InvalidDecode)?;
    if d.decoded_xml_bytes.value.is_none() {
        require(d.decoded_prefix_sha256.is_none(), F::InvalidDecode)?;
    }
    // Identity cannot produce more XML bytes than actually delivered entity bytes.
    if d.encoding == Some(SitemapEntityEncoding::Identity) {
        if let (Some(xml), Some(entity)) = (
            d.decoded_xml_bytes.value,
            p.transport.post_http_received_bytes.value,
        ) {
            require(xml <= entity, F::InvalidDecode)?;
            if d.integrity == I::Complete {
                require(xml == entity, F::InvalidDecode)?;
            }
        }
    }
    Ok(())
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b)
        .ok_or_else(|| error(SitemapSourceMeasurementFailureCode::ArithmeticOverflow))
}
fn validate_scan(p: &SitemapSourceMeasurementPayload) -> Result<()> {
    use SitemapMeasuredPrecision as P;
    use SitemapSourceMeasurementFailureCode as F;
    use SitemapSourceScanStatus as S;
    let s = &p.scan;
    require(
        stop_matches_transport(s.stop_code, &p.transport),
        F::InvalidScan,
    )?;
    let c = &s.counts;
    s.events_observed.validate()?;
    let quantities = [
        c.immediate_entry_starts,
        c.completed_immediate_entries,
        c.actual_document_namespace_loc_occurrences,
        c.protocol_eligible_completed_entries,
    ];
    for value in quantities {
        value.validate()?;
    }
    for value in [
        c.missing_loc_entries,
        c.multiple_loc_entries,
        c.invalid_single_loc_entries,
    ] {
        scalar(value)?;
    }
    let diagnostics = add(
        add(c.missing_loc_entries, c.multiple_loc_entries)?,
        c.invalid_single_loc_entries,
    )?;
    match s.status {
        S::NotObserved => require(
            s.document_kind.is_none()
                && s.namespace_profile.is_none()
                && s.events_observed.precision == P::Unavailable
                && quantities.iter().all(|v| v.precision == P::Unavailable)
                && diagnostics == 0
                && s.stop_code.is_some(),
            F::InvalidScan,
        )?,
        S::CompleteProfile => require(
            p.decode.integrity == SitemapSourceDecodeIntegrity::Complete
                && s.document_kind.is_some()
                && s.namespace_profile.is_some()
                && s.stop_code.is_none()
                && s.events_observed.precision == P::Exact
                && quantities.iter().all(|v| v.precision == P::Exact),
            F::InvalidScan,
        )?,
        S::Failed => require(
            p.decode.integrity != SitemapSourceDecodeIntegrity::NotObserved
                && s.stop_code.is_some()
                && !matches!(
                    s.stop_code,
                    Some(
                        SitemapSourceMeasurementStop::RequestNotStarted
                            | SitemapSourceMeasurementStop::RobotsDenied
                            | SitemapSourceMeasurementStop::HttpStatusNot200
                    )
                )
                && s.events_observed.precision == P::AtLeast,
            F::InvalidScan,
        )?,
    }
    let known_root = s.document_kind.is_some() && s.namespace_profile.is_some();
    if s.status != S::NotObserved {
        require(p.decode.decoded_xml_bytes.value.is_some(), F::InvalidScan)?;
    }
    let event_budget_sentinel =
        s.status == S::Failed && s.stop_code == Some(SitemapSourceMeasurementStop::EventLimit);
    if s.stop_code == Some(SitemapSourceMeasurementStop::EventLimit) {
        require(event_budget_sentinel, F::InvalidScan)?;
    }
    if let Some(events) = s.events_observed.value {
        let event_cap = if event_budget_sentinel {
            add(p.bounds.max_xml_events, 1)?
        } else {
            p.bounds.max_xml_events
        };
        require(events <= event_cap, F::InvalidScan)?;
        if event_budget_sentinel {
            require(events == event_cap, F::InvalidScan)?;
        }
        require(
            events <= add(p.decode.decoded_xml_bytes.value.unwrap_or(0), 1)?,
            F::InvalidScan,
        )?;
        if known_root {
            require(events > 0, F::InvalidScan)?;
            require(
                p.decode
                    .decoded_xml_bytes
                    .value
                    .is_some_and(|bytes| bytes > 0),
                F::InvalidScan,
            )?;
        }
    }
    if !known_root {
        // An attempted scan can report its known zero accepted-prefix counters
        // before root/profile recognition. It cannot report positive scoped
        // protocol counts without that recognition, or infer a complete zero.
        let observed_zero_prefix = s.status == S::Failed
            && quantities
                .iter()
                .all(|v| *v == SitemapMeasuredValue::at_least(0));
        require(
            (quantities.iter().all(|v| v.precision == P::Unavailable) || observed_zero_prefix)
                && diagnostics == 0,
            F::InvalidScan,
        )?;
        return Ok(());
    }
    let precision = if s.status == S::CompleteProfile {
        P::Exact
    } else {
        P::AtLeast
    };
    require(
        quantities.iter().all(|v| v.precision == precision),
        F::InvalidScan,
    )?;
    let [starts, completed, locs, eligible] =
        quantities.map(|q| q.value.expect("validated available quantity"));
    require(
        eligible <= completed && completed <= starts && eligible <= locs,
        F::CounterInconsistency,
    )?;
    // The strict XML recognizer has at most one active immediate entry.
    require(starts - completed <= 1, F::CounterInconsistency)?;
    require(
        add(eligible, diagnostics)? == completed,
        F::CounterInconsistency,
    )?;
    let multi_locs = c
        .multiple_loc_entries
        .checked_mul(2)
        .ok_or_else(|| error(F::ArithmeticOverflow))?;
    require(
        add(add(eligible, c.invalid_single_loc_entries)?, multi_locs)? <= locs,
        F::CounterInconsistency,
    )?;
    if starts == completed && c.multiple_loc_entries == 0 {
        require(
            add(eligible, c.invalid_single_loc_entries)? == locs,
            F::CounterInconsistency,
        )?;
    }
    require(
        add(starts, locs)? <= s.events_observed.value.unwrap_or(0),
        F::CounterInconsistency,
    )?;
    if s.status == S::CompleteProfile {
        require(starts == completed, F::CounterInconsistency)?;
    }
    Ok(())
}
struct JsonCounter {
    count: usize,
    cap: usize,
}
impl Write for JsonCounter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .count
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("measurement length overflow"))?;
        if next > self.cap {
            return Err(io::Error::other("measurement JSON byte limit"));
        }
        self.count = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn bounded_json_size(value: &impl Serialize, cap: usize) -> Result<()> {
    serde_json::to_writer(JsonCounter { count: 0, cap }, value)
        .map_err(|_| error(SitemapSourceMeasurementFailureCode::JsonBytesLimit))
}
fn payload_id(payload: &SitemapSourceMeasurementPayload) -> Result<String> {
    let bytes = serde_json::to_vec(&("open-seo-spider/sitemap-source-measurement-id-v1", payload))
        .map_err(|_| error(SitemapSourceMeasurementFailureCode::InvalidJson))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
pub fn validate_sitemap_source_measurement(
    measurement: &SitemapSourceMeasurement,
    limits: &SitemapSourceMeasurementLimits,
) -> Result<()> {
    limits.validate()?;
    validate_payload(&measurement.payload, limits)?;
    require(
        hash(&measurement.measurement_id),
        SitemapSourceMeasurementFailureCode::MeasurementIdMismatch,
    )?;
    bounded_json_size(measurement, limits.max_record_json_bytes)?;
    require(
        payload_id(&measurement.payload)? == measurement.measurement_id,
        SitemapSourceMeasurementFailureCode::MeasurementIdMismatch,
    )
}
/// Bound the original JSON bytes BEFORE deserializing; no unknown fields,
/// version coercion, precision repair, URL rewriting or inferred zeroes.
pub fn parse_sitemap_source_measurement_json(
    input: &[u8],
    limits: &SitemapSourceMeasurementLimits,
) -> Result<SitemapSourceMeasurement> {
    limits.validate()?;
    require(
        input.len() <= limits.max_record_json_bytes,
        SitemapSourceMeasurementFailureCode::JsonBytesLimit,
    )?;
    let value: SitemapSourceMeasurement = serde_json::from_slice(input)
        .map_err(|_| error(SitemapSourceMeasurementFailureCode::InvalidJson))?;
    validate_sitemap_source_measurement(&value, limits)?;
    Ok(value)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapSourceLimitVerdict {
    WithinLimit,
    ObservedExceeded,
    Unknown,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapSourceLimitBasis {
    CompleteStructuralUpperBound,
    EligibleCompletedLowerBound,
    CompleteDecodedBytes,
    ObservedDecodedLowerBound,
    NoHttp200Source,
    UnrecognizedStandardSource,
    IncompleteStructuralScan,
    StructuralExcessWithoutEligibleExcess,
    IncompleteDecodedSource,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SitemapSourceLimitAssessment {
    pub threshold: u64,
    pub verdict: SitemapSourceLimitVerdict,
    pub basis: SitemapSourceLimitBasis,
    pub proof_quantity: SitemapMeasuredValue,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SitemapSourceProtocolAssessment {
    pub schema_version: u32,
    pub engine: &'static str,
    pub measurement_id: String,
    pub document_kind: Option<SitemapDocumentKind>,
    pub namespace_profile: Option<SitemapNamespaceProfile>,
    pub decode_integrity: SitemapSourceDecodeIntegrity,
    pub xml_scan_status: SitemapSourceScanStatus,
    pub membership_retention: SitemapMembershipRetention,
    /// Raw structural diagnostics; neither field alone proves excess URLs.
    pub observed_entry_starts_exceeded: bool,
    pub observed_completed_entry_slots_exceeded: bool,
    pub entry_limit: SitemapSourceLimitAssessment,
    pub decoded_xml_byte_limit: SitemapSourceLimitAssessment,
}
/// Derives threshold claims only. Caller-supplied facts are validated for internal
/// consistency, not authenticated as a real fetch. No complete-XSD/cohort claim.
pub fn assess_sitemap_source_protocol_limits(
    measurement: &SitemapSourceMeasurement,
    limits: &SitemapSourceMeasurementLimits,
) -> Result<SitemapSourceProtocolAssessment> {
    validate_sitemap_source_measurement(measurement, limits)?;
    use SitemapSourceLimitBasis as B;
    use SitemapSourceLimitVerdict as V;
    let p = &measurement.payload;
    let c = &p.scan.counts;
    let source = http_source_observed(&p.transport);
    let full_decode = source
        && p.transport.body_complete
        && p.decode.integrity == SitemapSourceDecodeIntegrity::Complete;
    let standard = p.scan.namespace_profile == Some(SitemapNamespaceProfile::Sitemap09)
        && p.scan.document_kind.is_some();
    let result = |threshold, verdict, basis, proof_quantity| SitemapSourceLimitAssessment {
        threshold,
        verdict,
        basis,
        proof_quantity,
    };
    let entry_limit = if !source {
        result(
            SITEMAP_PROTOCOL_ENTRY_LIMIT,
            V::Unknown,
            B::NoHttp200Source,
            SitemapMeasuredValue::unavailable(),
        )
    } else if !standard {
        result(
            SITEMAP_PROTOCOL_ENTRY_LIMIT,
            V::Unknown,
            B::UnrecognizedStandardSource,
            SitemapMeasuredValue::unavailable(),
        )
    } else if c
        .protocol_eligible_completed_entries
        .value
        .is_some_and(|n| n > SITEMAP_PROTOCOL_ENTRY_LIMIT)
    {
        result(
            SITEMAP_PROTOCOL_ENTRY_LIMIT,
            V::ObservedExceeded,
            B::EligibleCompletedLowerBound,
            c.protocol_eligible_completed_entries,
        )
    } else if full_decode && p.scan.status == SitemapSourceScanStatus::CompleteProfile {
        let structural = c.completed_immediate_entries;
        if structural
            .value
            .is_some_and(|n| n <= SITEMAP_PROTOCOL_ENTRY_LIMIT)
        {
            result(
                SITEMAP_PROTOCOL_ENTRY_LIMIT,
                V::WithinLimit,
                B::CompleteStructuralUpperBound,
                structural,
            )
        } else {
            result(
                SITEMAP_PROTOCOL_ENTRY_LIMIT,
                V::Unknown,
                B::StructuralExcessWithoutEligibleExcess,
                structural,
            )
        }
    } else {
        result(
            SITEMAP_PROTOCOL_ENTRY_LIMIT,
            V::Unknown,
            B::IncompleteStructuralScan,
            c.protocol_eligible_completed_entries,
        )
    };
    let xml = p.decode.decoded_xml_bytes;
    let decoded_xml_byte_limit = if !source {
        result(
            SITEMAP_PROTOCOL_XML_BYTE_LIMIT,
            V::Unknown,
            B::NoHttp200Source,
            SitemapMeasuredValue::unavailable(),
        )
    } else if xml
        .value
        .is_some_and(|n| n > SITEMAP_PROTOCOL_XML_BYTE_LIMIT)
    {
        result(
            SITEMAP_PROTOCOL_XML_BYTE_LIMIT,
            V::ObservedExceeded,
            B::ObservedDecodedLowerBound,
            xml,
        )
    } else if full_decode {
        result(
            SITEMAP_PROTOCOL_XML_BYTE_LIMIT,
            V::WithinLimit,
            B::CompleteDecodedBytes,
            xml,
        )
    } else {
        result(
            SITEMAP_PROTOCOL_XML_BYTE_LIMIT,
            V::Unknown,
            B::IncompleteDecodedSource,
            xml,
        )
    };
    Ok(SitemapSourceProtocolAssessment {
        schema_version: 1,
        engine: SITEMAP_SOURCE_LIMIT_ENGINE,
        measurement_id: measurement.measurement_id.clone(),
        document_kind: p.scan.document_kind,
        namespace_profile: p.scan.namespace_profile.clone(),
        decode_integrity: p.decode.integrity,
        xml_scan_status: p.scan.status,
        membership_retention: p.membership_retention,
        observed_entry_starts_exceeded: c
            .immediate_entry_starts
            .value
            .is_some_and(|n| n > SITEMAP_PROTOCOL_ENTRY_LIMIT),
        observed_completed_entry_slots_exceeded: c
            .completed_immediate_entries
            .value
            .is_some_and(|n| n > SITEMAP_PROTOCOL_ENTRY_LIMIT),
        entry_limit,
        decoded_xml_byte_limit,
    })
}

#[cfg(test)]
#[path = "sitemap_source_measurement_tests.rs"]
mod tests;
