use serde::{Deserialize, Serialize};
pub const SITEMAP_XML_ENGINE: &str = "open-seo-spider/sitemap-xml-occurrences-v1";
pub const SITEMAP_XML_NS: &str = "http://www.sitemaps.org/schemas/sitemap/0.9";
pub const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
pub const XMLNS_NS: &str = "http://www.w3.org/2000/xmlns/";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SitemapParserLimits {
    /// Bytes supplied after HTTP Content-Decoding; never claimed to be wire bytes.
    pub max_entity_bytes: usize,
    pub max_xml_bytes: usize,
    pub max_xml_events: usize,
    pub max_xml_depth: usize,
    pub max_entries: usize,
    pub max_location_occurrences: usize,
    pub max_field_utf8_bytes: usize,
    /// Input-derived retained string bytes (context, raw XML, decoded fields,
    /// normalized URLs). Fixed metadata strings and JSON-envelope bytes are separate.
    pub max_retained_text_utf8_bytes: usize,
    pub allow_legacy_unqualified: bool,
}
impl Default for SitemapParserLimits {
    fn default() -> Self {
        Self {
            max_entity_bytes: 8 * 1024 * 1024,
            max_xml_bytes: 8 * 1024 * 1024,
            max_xml_events: 500_000,
            max_xml_depth: 64,
            max_entries: 50_000,
            max_location_occurrences: 100_000,
            max_field_utf8_bytes: 8192,
            max_retained_text_utf8_bytes: 32 * 1024 * 1024,
            allow_legacy_unqualified: true,
        }
    }
}
impl SitemapParserLimits {
    pub fn validate(&self) -> Result<(), SitemapFailure> {
        let cap = Self::default();
        for (name, value, maximum) in [
            (
                "max_entity_bytes",
                self.max_entity_bytes,
                cap.max_entity_bytes,
            ),
            ("max_xml_bytes", self.max_xml_bytes, cap.max_xml_bytes),
            ("max_xml_events", self.max_xml_events, cap.max_xml_events),
            ("max_xml_depth", self.max_xml_depth, cap.max_xml_depth),
            ("max_entries", self.max_entries, cap.max_entries),
            (
                "max_location_occurrences",
                self.max_location_occurrences,
                cap.max_location_occurrences,
            ),
            (
                "max_field_utf8_bytes",
                self.max_field_utf8_bytes,
                cap.max_field_utf8_bytes,
            ),
            (
                "max_retained_text_utf8_bytes",
                self.max_retained_text_utf8_bytes,
                cap.max_retained_text_utf8_bytes,
            ),
        ] {
            if value == 0 || value > maximum {
                return Err(SitemapFailure::new(
                    "sitemap_invalid_limits",
                    format!("{name} must be 1..={maximum}"),
                    None,
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapParserContext {
    /// Optional caller declarations. The parser neither fetches nor attests them.
    pub requested_document_url: Option<String>,
    pub final_document_url: Option<String>,
    pub originating_occurrence_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapNamespaceProfile {
    Sitemap09,
    LegacyUnqualified,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapDocumentKind {
    UrlSet,
    SitemapIndex,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapLocationRole {
    PageLocation,
    ChildSitemap,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapFailure {
    pub code: String,
    pub message: String,
    pub xml_byte_offset: Option<u64>,
}
impl SitemapFailure {
    pub(crate) fn new(code: &str, message: impl Into<String>, offset: Option<u64>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            xml_byte_offset: offset,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapFieldOccurrence {
    pub ordinal: usize,
    pub xml_byte_start: u64,
    pub xml_byte_end: u64,
    pub raw_element_xml: String,
    pub raw_inner_xml: String,
    pub decoded_value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapLocationOccurrence {
    pub ordinal: usize,
    pub entry_ordinal: usize,
    /// None means a missing-loc entry diagnostic, not an observed loc element.
    pub loc_ordinal: Option<usize>,
    pub role: SitemapLocationRole,
    pub xml_byte_start: u64,
    pub xml_byte_end: u64,
    pub raw_element_xml: Option<String>,
    pub raw_inner_xml: Option<String>,
    pub decoded_value: Option<String>,
    /// Separate ordinary safe-fetch identity; query order/case/trailing slash follow existing normalizer.
    pub normalized_fetch_url: Option<String>,
    pub validation_error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapEntryObservation {
    pub ordinal: usize,
    pub xml_byte_start: u64,
    pub xml_byte_end: u64,
    /// False on a retained prefix of an entry whose close was never accepted.
    pub complete: bool,
    pub loc_occurrence_ordinals: Vec<usize>,
    /// Raw XML facts only. No date parsing, freshness or validity assertion.
    pub lastmod: Vec<SitemapFieldOccurrence>,
    pub diagnostics: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapParseObservation {
    pub schema_version: u32,
    pub engine: String,
    pub context_basis: String,
    pub context: SitemapParserContext,
    pub xml_bytes: usize,
    pub xml_sha256: String,
    pub xml_sha_profile: String,
    pub namespace_profile: Option<SitemapNamespaceProfile>,
    pub xml_name_profile: String,
    pub kind: Option<SitemapDocumentKind>,
    pub events_observed: usize,
    pub retained_text_utf8_bytes: usize,
    /// Complete bounded syntax/profile inspection only; not discovery-frontier or source-cohort completeness.
    pub complete: bool,
    pub failure: Option<SitemapFailure>,
    pub entries: Vec<SitemapEntryObservation>,
    pub locations: Vec<SitemapLocationOccurrence>,
}
impl SitemapParseObservation {
    /// No partial-document values reach admission. Invalid URI rows remain stored facts.
    pub fn admission_locations(&self) -> impl Iterator<Item = &SitemapLocationOccurrence> {
        self.locations.iter().filter(|row| {
            self.complete
                && row.loc_ordinal.is_some()
                && row.validation_error.is_none()
                && row.normalized_fetch_url.is_some()
        })
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapEntityEncoding {
    Identity,
    GzipFileEntity,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapEntityDecode {
    pub encoding: SitemapEntityEncoding,
    pub post_http_decode_entity_bytes: usize,
    pub decoded_xml_bytes: usize,
    pub decoded_xml_sha256: String,
    /// Bytes are an internal parser input, not a retained report JSON integer array.
    #[serde(skip)]
    pub xml: Vec<u8>,
}
