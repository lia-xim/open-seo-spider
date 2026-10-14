//! Compact second-pass counts, separate from retained membership/admission facts.
use super::models::{
    SitemapDocumentKind, SitemapFailure, SitemapNamespaceProfile, SitemapParserLimits,
};
use serde::{Deserialize, Serialize};

pub const SITEMAP_COUNT_XML_ENGINE: &str = "open-seo-spider/sitemap-count-only-xml-v1";
pub const SITEMAP_COUNT_LOC_PROFILE: &str = "absolute-http-xml-loc-v1";

/// Scanner work limits do not contain cumulative retained-row/text limits.
/// Defaults preserve 8 MiB/500,000 events. XML/event hard ceilings permit the
/// explicit large-source work profile; depth/field limits remain unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SitemapCountLimits {
    pub max_xml_bytes: usize,
    pub max_xml_events: usize,
    pub max_xml_depth: usize,
    pub max_field_utf8_bytes: usize,
    pub allow_legacy_unqualified: bool,
}
impl Default for SitemapCountLimits {
    fn default() -> Self {
        let limits = SitemapParserLimits::default();
        Self {
            max_xml_bytes: limits.max_xml_bytes,
            max_xml_events: limits.max_xml_events,
            max_xml_depth: limits.max_xml_depth,
            max_field_utf8_bytes: limits.max_field_utf8_bytes,
            allow_legacy_unqualified: limits.allow_legacy_unqualified,
        }
    }
}
impl SitemapCountLimits {
    pub fn validate(&self) -> Result<(), SitemapFailure> {
        self.parser_limits().validate()
    }
    pub(super) fn parser_limits(&self) -> SitemapParserLimits {
        SitemapParserLimits {
            max_xml_bytes: self.max_xml_bytes,
            max_xml_events: self.max_xml_events,
            max_xml_depth: self.max_xml_depth,
            max_field_utf8_bytes: self.max_field_utf8_bytes,
            allow_legacy_unqualified: self.allow_legacy_unqualified,
            ..SitemapParserLimits::default()
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapXmlCountFacts {
    /// Actual recognized immediate entry opening events, not URL count.
    pub immediate_entry_starts: u64,
    pub completed_immediate_entries: u64,
    /// Actual immediate document-family Loc opening events; never synthetic missing rows.
    pub started_document_namespace_loc_elements: u64,
    /// Accepted actual immediate Loc closes, including Locs in an unfinished entry.
    pub actual_document_namespace_loc_occurrences: u64,
    /// Accepted entry closes with exactly one completed eligible original Loc.
    pub protocol_eligible_completed_entries: u64,
    /// These three completed-entry classifications are disjoint.
    pub missing_location_entries: u64,
    pub multiple_location_entries: u64,
    pub invalid_single_location_entries: u64,
    /// Completed Loc diagnostics; a bad Loc in a multi-Loc entry still appears here.
    pub invalid_location_occurrences: u64,
    pub empty_location_occurrences: u64,
    pub over_character_limit_location_occurrences: u64,
    pub credential_location_occurrences: u64,
    pub malformed_percent_location_occurrences: u64,
    pub fragment_location_occurrences: u64,
    pub invalid_absolute_http_location_occurrences: u64,
    pub xml_space_trimmed_location_occurrences: u64,
    /// Extension names are recognized only as ignored lexical names, never protocol entries.
    pub ignored_nonfamily_entry_elements: u64,
    pub ignored_nonfamily_loc_elements: u64,
    /// A recognized-family element at a wrong parent still stops the shared recognizer.
    pub misplaced_family_elements: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapCountObservation {
    pub schema_version: u32,
    pub engine: String,
    pub syntax_profile: String,
    pub loc_lexical_profile: String,
    pub loc_whitespace_profile: String,
    /// Strictly fewer than 2048 Unicode scalar values after XML-S edge trimming.
    pub loc_character_limit_exclusive: u64,
    pub effective_limits: SitemapCountLimits,
    /// Facts about the complete supplied bounded input, not successfully traversed bytes.
    pub input_xml_bytes: usize,
    pub input_xml_sha256: String,
    pub input_xml_sha_profile: String,
    pub kind: Option<SitemapDocumentKind>,
    pub namespace_profile: Option<SitemapNamespaceProfile>,
    pub events_observed: usize,
    /// Complete shared grammar traversal, allowing recorded missing/multi/invalid Locs.
    /// This is not full XSD validity, source-integrity, membership or freshness proof.
    pub xml_scan_complete: bool,
    /// Counts are observed prefixes when this is false; input length/hash remain full-input facts.
    pub failure: Option<SitemapFailure>,
    pub counts: SitemapXmlCountFacts,
}
