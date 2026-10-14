//! Decode-local facts only. A supplied slice is not a complete HTTP representation.
use super::models::SitemapEntityEncoding;
use serde::{Deserialize, Serialize};

pub const SITEMAP_ENTITY_DECODE_OBSERVATION_ENGINE: &str =
    "open-seo-spider/sitemap-entity-decode-observation-v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapEntityDecodeIntegrity {
    Complete,
    Failed,
    Incomplete,
    NotObserved,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapEntityDecodeReadErrorKind {
    UnexpectedEof,
    InvalidInput,
    InvalidData,
    Other,
}

/// The two effective decode budgets, present only after ALL parser limits validate.
/// Parser counts, HTTP bounds and configured current defaults are separate domains.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapEntityDecodeEffectiveLimits {
    pub max_entity_bytes: u64,
    pub max_xml_bytes: u64,
}

/// Compact source-produced evidence; never reconstruct full XML size from ISIZE,
/// compressed input length, a ratio, a header or bytes written by an Err read.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapEntityDecodeObservation {
    pub schema_version: u8,
    pub engine: String,
    /// Exact length of this supplied slice; no wire/HTTP/body-completeness claim.
    pub supplied_post_http_entity_slice_bytes: u64,
    /// None means the legacy preflight stopped before the encoding sniff.
    pub encoding: Option<SitemapEntityEncoding>,
    pub effective_limits: Option<SitemapEntityDecodeEffectiveLimits>,
    /// Actual successful read outputs, including a real returned sentinel byte.
    /// Identity success counts the direct output slice; predecode stops are None.
    pub decoded_bytes_returned: Option<u64>,
    /// Hash of exactly decoded_bytes_returned bytes, not an unreturned Err buffer.
    /// Some(empty SHA) with Some(0) is distinct from unavailable None/None.
    pub decoded_prefix_sha256: Option<String>,
    /// Present only when the legacy decode Result succeeds. Never a transport hash.
    pub decoded_complete_sha256: Option<String>,
    /// Gzip only: an actual Ok(0) from a nonempty requested buffer was observed.
    pub decoder_eof_observed: Option<bool>,
    /// Gzip only: EOF plus all members/trailers/checksums and consumed input.
    /// Can be true for a valid outer gzip whose nested gzip content is rejected.
    pub all_gzip_members_validated: Option<bool>,
    pub integrity: SitemapEntityDecodeIntegrity,
    /// Exact bounded code from the returned legacy SitemapFailure, when present.
    pub stop_code: Option<String>,
    pub read_error_kind: Option<SitemapEntityDecodeReadErrorKind>,
}
