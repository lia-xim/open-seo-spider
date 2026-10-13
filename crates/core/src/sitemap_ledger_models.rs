//! WORK-only candidate. Parser models are reused from current Core exports.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

pub const SITEMAP_LEDGER_ENGINE: &str = "open-seo-spider/sitemap-membership-ledger-v1";
pub const SITEMAP_LEDGER_HASH_PROFILE: &str = "sha256-tagged-canonical-rust-serde-rows-v1";
pub const MAX_SITEMAP_CATALOG_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SitemapLedgerLimits {
    pub max_catalog_json_bytes: usize,
    pub max_row_json_bytes: usize,
    pub max_rows: u64,
}
impl Default for SitemapLedgerLimits {
    fn default() -> Self {
        Self {
            max_catalog_json_bytes: MAX_SITEMAP_CATALOG_BYTES,
            max_row_json_bytes: 256 * 1024,
            max_rows: 1_000_000,
        }
    }
}
impl SitemapLedgerLimits {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.max_catalog_json_bytes > 0
                && self.max_catalog_json_bytes <= MAX_SITEMAP_CATALOG_BYTES,
            "Invalid sitemap catalog byte cap"
        );
        ensure!(
            self.max_row_json_bytes > 0
                && self.max_row_json_bytes <= 1024 * 1024
                && self.max_row_json_bytes <= self.max_catalog_json_bytes,
            "Invalid sitemap row byte cap"
        );
        ensure!(
            self.max_rows > 0 && self.max_rows <= 1_000_000,
            "Invalid sitemap row cap"
        );
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapSourceKind {
    RobotsDirectives,
    SitemapXml,
}
impl SitemapSourceKind {
    pub(crate) fn frontier_kind(self) -> &'static str {
        match self {
            Self::RobotsDirectives => "robots",
            Self::SitemapXml => "sitemap",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapRootKind {
    ExplicitConfig,
    ImportedSitemapRow,
    RobotsDirective,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapAdmissionDecision {
    Unbound,
    InvalidLocation,
    MissingLocation,
    IncompleteSource,
    OutsideScope,
    FilterDenied,
    SourceCheckOnly,
    PolicyDenied,
    Coalesced,
    Scheduled,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapAdmissionBinding {
    pub decision: SitemapAdmissionDecision,
    /// Original safe listed identity; never replaced by a policy fetch alias.
    pub membership_url: Option<String>,
    pub admitted_fetch_url: Option<String>,
    pub coalesced_policy_occurrence_id: Option<String>,
    pub policy_occurrence_id: Option<String>,
    pub policy_source_revision: Option<String>,
}
impl SitemapAdmissionBinding {
    pub fn observed(location: &crate::SitemapLocationOccurrence, complete: bool) -> Self {
        let decision = if !complete {
            SitemapAdmissionDecision::IncompleteSource
        } else if location.loc_ordinal.is_none() {
            SitemapAdmissionDecision::MissingLocation
        } else if location.validation_error.is_some() || location.normalized_fetch_url.is_none() {
            SitemapAdmissionDecision::InvalidLocation
        } else {
            SitemapAdmissionDecision::Unbound
        };
        Self {
            decision,
            membership_url: location.normalized_fetch_url.clone(),
            admitted_fetch_url: None,
            coalesced_policy_occurrence_id: None,
            policy_occurrence_id: None,
            policy_source_revision: None,
        }
    }
    /// Called ONLY after policy_ledger::commit has finalized scheduling/coalescing.
    pub fn from_final_policy(
        location: &crate::SitemapLocationOccurrence,
        row: &crate::PolicyOccurrence,
    ) -> Result<Self> {
        ensure!(
            location.loc_ordinal.is_some(),
            "Missing Loc is not a policy input"
        );
        ensure!(
            location.decoded_value.as_deref() == Some(row.decision.raw_value.as_str()),
            "Policy raw value differs from original XML Loc"
        );
        ensure!(
            location.ordinal == row.source_ordinal,
            "Policy/XML ordinal differs"
        );
        ensure!(
            row.input_basis == crate::PolicyInputBasis::ParserObservedXml,
            "Policy input is not an XML observation"
        );
        ensure!(
            row.raw_value_observed,
            "Policy XML raw value was not observed"
        );
        let role = match location.role {
            crate::SitemapLocationRole::PageLocation => crate::PolicyTaskRole::PageDocument,
            crate::SitemapLocationRole::ChildSitemap => crate::PolicyTaskRole::SitemapDocument,
        };
        ensure!(row.task_role == role, "Policy/XML task role differs");
        let decision = match row.admission_reason.as_str() {
            "pending_admission" => anyhow::bail!("Policy scheduling is not finalized"),
            "scheduled" => {
                ensure!(
                    row.decision.policy_complete,
                    "Incomplete policy cannot schedule XML Loc"
                );
                ensure!(
                    row.scheduled_fetch_url.is_some(),
                    "Scheduled policy URL missing"
                );
                SitemapAdmissionDecision::Scheduled
            }
            "coalesced" => {
                ensure!(
                    row.decision.policy_complete,
                    "Incomplete policy cannot coalesce XML Loc"
                );
                ensure!(
                    row.coalesced_task_occurrence_id.is_some(),
                    "Coalesced policy reference missing"
                );
                SitemapAdmissionDecision::Coalesced
            }
            "source_check_only" => SitemapAdmissionDecision::SourceCheckOnly,
            _ if location.validation_error.is_some() || location.normalized_fetch_url.is_none() => {
                SitemapAdmissionDecision::InvalidLocation
            }
            _ => SitemapAdmissionDecision::PolicyDenied,
        };
        let admitted = if matches!(decision, SitemapAdmissionDecision::Scheduled) {
            row.scheduled_fetch_url.clone()
        } else if matches!(decision, SitemapAdmissionDecision::Coalesced) {
            row.decision.fetch_candidate_url.clone()
        } else {
            None
        };
        Ok(Self {
            decision,
            membership_url: location.normalized_fetch_url.clone(),
            admitted_fetch_url: admitted,
            coalesced_policy_occurrence_id: row.coalesced_task_occurrence_id.clone(),
            policy_occurrence_id: Some(row.decision.occurrence_id.clone()),
            policy_source_revision: Some(row.source_revision.clone()),
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapRootDeclaration {
    pub sequence: u64,
    pub declaration_id: String,
    pub kind: SitemapRootKind,
    pub source_observation_id: Option<String>,
    pub source_ordinal: u64,
    pub raw_line: Option<String>,
    pub raw_value: String,
    pub normalized_document_url: Option<String>,
    pub robots_evidence: Option<SitemapRobotsDeclarationFacts>,
    pub admission: SitemapAdmissionBinding,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapRobotsDeclarationFacts {
    pub line_ordinal: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_complete: bool,
    pub utf8_lossy: bool,
    /// Existing base/comment projection; it is never the original declaration identity.
    pub legacy_candidate_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapTransportFacts {
    /// Recorded immediately before send(); never inferred from an error/status.
    pub request_started: bool,
    pub requested_url: Option<String>,
    pub final_url: Option<String>,
    pub initial_status: Option<u16>,
    pub final_status: Option<u16>,
    pub blocked_by_robots: bool,
    pub body_complete: bool,
    pub post_http_decoding_bytes: u64,
    pub post_http_decoding_sha256: Option<String>,
    pub retained_post_http_prefix_sha256: Option<String>,
    pub failure_code: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapDecodeFacts {
    pub encoding: crate::SitemapEntityEncoding,
    pub post_http_decode_entity_bytes: usize,
    pub decoded_xml_bytes: usize,
    pub decoded_xml_sha256: String,
}
impl From<&crate::SitemapEntityDecode> for SitemapDecodeFacts {
    fn from(value: &crate::SitemapEntityDecode) -> Self {
        Self {
            encoding: value.encoding,
            post_http_decode_entity_bytes: value.post_http_decode_entity_bytes,
            decoded_xml_bytes: value.decoded_xml_bytes,
            decoded_xml_sha256: value.decoded_xml_sha256.clone(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapParserFacts {
    pub schema_version: u32,
    pub engine: String,
    pub context_basis: String,
    pub context: crate::SitemapParserContext,
    pub xml_bytes: usize,
    pub xml_sha256: String,
    pub xml_sha_profile: String,
    pub namespace_profile: Option<crate::SitemapNamespaceProfile>,
    pub xml_name_profile: String,
    pub kind: Option<crate::SitemapDocumentKind>,
    pub events_observed: usize,
    pub retained_text_utf8_bytes: usize,
    pub entry_count: usize,
    pub location_count: usize,
    pub complete: bool,
    pub failure: Option<crate::SitemapFailure>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceDocument {
    pub sequence: u64,
    pub observation_id: String,
    pub kind: SitemapSourceKind,
    pub task_url: String,
    pub generation: u64,
    pub source_revision: String,
    pub transport: Option<SitemapTransportFacts>,
    pub entity_decode: Option<SitemapDecodeFacts>,
    pub parser: Option<SitemapParserFacts>,
    pub declaration_scan_complete: bool,
    /// Some(false) attests lossless robots body UTF-8; None means unobserved/not applicable.
    pub declaration_utf8_lossy: Option<bool>,
    /// Count of retained raw robots directives; None for XML. Prefix counts are still exact.
    pub declaration_count: Option<usize>,
    pub retained_rows_complete: bool,
    pub failure_code: Option<String>,
}
impl SitemapSourceDocument {
    /// Frontier identity remains unchanged. Robots tasks are keyed by the origin
    /// root while RobotsCache requests origin/robots.txt; XML uses its task URL.
    pub(crate) fn expected_requested_url(&self) -> Option<String> {
        match self.kind {
            SitemapSourceKind::SitemapXml => Some(self.task_url.clone()),
            SitemapSourceKind::RobotsDirectives => crate::normalize_url(&self.task_url, None)
                .map(|url| format!("{}/robots.txt", url.origin().ascii_serialization())),
        }
    }
    pub fn complete(&self) -> bool {
        self.failure_code.is_none()
            && self.retained_rows_complete
            && self.transport.as_ref().is_some_and(|value| {
                value.request_started
                    && value.body_complete
                    && value.requested_url.as_deref() == self.expected_requested_url().as_deref()
                    && value.requested_url.is_some()
                    && value.final_url.is_some()
                    && value.final_status == Some(200)
                    && !value.blocked_by_robots
                    && value.failure_code.is_none()
            })
            && match self.kind {
                SitemapSourceKind::RobotsDirectives => {
                    self.declaration_scan_complete && self.declaration_utf8_lossy == Some(false)
                }
                SitemapSourceKind::SitemapXml => {
                    self.parser
                        .as_ref()
                        .is_some_and(|value| value.complete && value.failure.is_none())
                        && self.entity_decode.is_some()
                }
            }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StoredSitemapEntry {
    pub sequence: u64,
    pub observation_id: String,
    pub entry_id: String,
    pub observed: crate::SitemapEntryObservation,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StoredSitemapLocation {
    pub sequence: u64,
    pub observation_id: String,
    pub occurrence_id: String,
    pub observed: crate::SitemapLocationOccurrence,
    pub admission: SitemapAdmissionBinding,
    pub referenced_sitemap_task: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "row", rename_all = "snake_case")]
pub enum SitemapLedgerRow {
    Root(SitemapRootDeclaration),
    Document(Box<SitemapSourceDocument>),
    Entry(StoredSitemapEntry),
    Location(StoredSitemapLocation),
}
impl SitemapLedgerRow {
    pub fn sequence(&self) -> u64 {
        match self {
            Self::Root(v) => v.sequence,
            Self::Document(v) => v.sequence,
            Self::Entry(v) => v.sequence,
            Self::Location(v) => v.sequence,
        }
    }
    pub(crate) fn set_sequence(&mut self, sequence: u64) {
        match self {
            Self::Root(v) => v.sequence = sequence,
            Self::Document(v) => v.sequence = sequence,
            Self::Entry(v) => v.sequence = sequence,
            Self::Location(v) => v.sequence = sequence,
        }
    }
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Root(_) => "root",
            Self::Document(_) => "document",
            Self::Entry(_) => "entry",
            Self::Location(_) => "location",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapInheritedBasis {
    pub parent_snapshot_sha256: String,
    pub parent_catalog_sha256: String,
    pub current_discovery_executed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapLedgerSummary {
    pub engine: String,
    pub hash_profile: String,
    pub config_sha256: String,
    pub limits: SitemapLedgerLimits,
    pub revision: u64,
    pub last_sequence: u64,
    pub root_rows: u64,
    pub document_rows: u64,
    pub entry_rows: u64,
    pub location_rows: u64,
    pub incomplete_documents: u64,
    pub catalog_json_bytes: u64,
    pub rolling_sha256: String,
    pub rows_available: bool,
    /// This draft NEVER asserts full recursive source-cohort coverage.
    pub declared_source_cohort_complete: bool,
    pub legacy_history_unknown: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapCatalog {
    pub schema_version: u32,
    pub summary: SitemapLedgerSummary,
    pub inherited: Option<SitemapInheritedBasis>,
    /// Some([]) is an observed initialized empty ledger; None is unavailable.
    pub rows: Option<Vec<SitemapLedgerRow>>,
}
#[derive(Debug, Clone)]
pub struct SitemapDiscoveryBatch {
    pub document: SitemapSourceDocument,
    pub roots: Vec<SitemapRootDeclaration>,
    pub entries: Vec<StoredSitemapEntry>,
    pub locations: Vec<StoredSitemapLocation>,
}
impl SitemapDiscoveryBatch {
    /// Keeps every parser entry/Loc, including invalid/missing/duplicate facts.
    /// Valid Locs are Unbound until the caller assigns final ordinary/policy decisions.
    pub fn from_parser(
        mut document: SitemapSourceDocument,
        observed: crate::SitemapParseObservation,
    ) -> Result<Self> {
        ensure!(
            document.kind == SitemapSourceKind::SitemapXml,
            "Wrong source kind for XML"
        );
        ensure!(document.parser.is_none(), "Parser facts already assigned");
        let document_id = document.observation_id.clone();
        document.parser = Some(SitemapParserFacts {
            schema_version: observed.schema_version,
            engine: observed.engine,
            context_basis: observed.context_basis,
            context: observed.context,
            xml_bytes: observed.xml_bytes,
            xml_sha256: observed.xml_sha256,
            xml_sha_profile: observed.xml_sha_profile,
            namespace_profile: observed.namespace_profile,
            xml_name_profile: observed.xml_name_profile,
            kind: observed.kind,
            events_observed: observed.events_observed,
            retained_text_utf8_bytes: observed.retained_text_utf8_bytes,
            entry_count: observed.entries.len(),
            location_count: observed.locations.len(),
            complete: observed.complete,
            failure: observed.failure,
        });
        let complete = document.complete();
        let entries = observed
            .entries
            .into_iter()
            .map(|observed| {
                let entry_id = super::identity(
                    "entry",
                    &document_id,
                    u64::try_from(observed.ordinal).context("Entry ordinal overflow")?,
                );
                Ok(StoredSitemapEntry {
                    sequence: 0,
                    observation_id: document_id.clone(),
                    entry_id,
                    observed,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let locations = observed
            .locations
            .into_iter()
            .map(|observed| {
                let occurrence_id = super::identity(
                    "location",
                    &document_id,
                    u64::try_from(observed.ordinal).context("Location ordinal overflow")?,
                );
                Ok(StoredSitemapLocation {
                    sequence: 0,
                    observation_id: document_id.clone(),
                    occurrence_id,
                    admission: SitemapAdmissionBinding::observed(&observed, complete),
                    observed,
                    referenced_sitemap_task: None,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            document,
            roots: vec![],
            entries,
            locations,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapLedgerPage {
    pub ledger: Option<SitemapLedgerSummary>,
    pub inherited: Option<SitemapInheritedBasis>,
    pub rows: Vec<SitemapLedgerRow>,
    pub next_after_sequence: Option<u64>,
    pub has_more: bool,
}
