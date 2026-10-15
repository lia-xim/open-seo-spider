//! Full stored sitemap finding selections. This is an independent local
//! predicate contract, not a claim of manufacturer predicate equivalence.
use crate::{
    ResourceAnalysisStatus, SitemapCoverageTask, SitemapSavedPageFinding,
    SitemapSourceCommitAssociation, SitemapSourceDocument, SitemapSourceProtocolAssessment,
    SitemapSourceStoreOmission, SitemapSourceStoredMeasurement, SitemapVerifiedReadHeader,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

pub(crate) mod predicate;
pub(crate) mod source;

pub const SITEMAP_FINDINGS_ENGINE: &str = "open-seo-spider/full-stored-sitemap-findings-v1";
pub const MAX_SITEMAP_FINDINGS_PAGE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SITEMAP_FINDINGS_SELECTION_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_SITEMAP_FINDINGS_CANDIDATES_PER_PAGE: usize = 5_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SitemapFindingFilter {
    #[default]
    #[serde(rename = "urls-in-sitemap")]
    UrlsInSitemap,
    #[serde(rename = "urls-not-in-sitemap")]
    UrlsNotInSitemap,
    #[serde(rename = "orphan-urls")]
    OrphanUrls,
    #[serde(rename = "non-indexable-urls-in-sitemap")]
    NonIndexableUrlsInSitemap,
    #[serde(rename = "urls-in-multiple-sitemaps")]
    UrlsInMultipleSitemaps,
    #[serde(rename = "xml-sitemap-with-over-50k-urls")]
    XmlSitemapWithOver50kUrls,
    #[serde(rename = "xml-sitemap-over-50mb")]
    XmlSitemapOver50mb,
}
impl SitemapFindingFilter {
    pub fn domain(self) -> SitemapFindingDomain {
        match self {
            Self::XmlSitemapWithOver50kUrls | Self::XmlSitemapOver50mb => {
                SitemapFindingDomain::SourceAttempt
            }
            _ => SitemapFindingDomain::SavedPage,
        }
    }
    pub fn granular_id(self) -> &'static str {
        match self {
            Self::UrlsInSitemap => "filter.sitemaps-urls-in-sitemap",
            Self::UrlsNotInSitemap => "filter.sitemaps-urls-not-in-sitemap",
            Self::OrphanUrls => "filter.sitemaps-orphan-urls",
            Self::NonIndexableUrlsInSitemap => "filter.sitemaps-non-indexable-urls-in-sitemap",
            Self::UrlsInMultipleSitemaps => "filter.sitemaps-urls-in-multiple-sitemaps",
            Self::XmlSitemapWithOver50kUrls => "filter.sitemaps-xml-sitemap-with-over-50k-urls",
            Self::XmlSitemapOver50mb => "filter.sitemaps-xml-sitemap-over-50mb",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SitemapFindingDomain {
    SavedPage,
    SourceAttempt,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SitemapFindingVerdict {
    Match,
    NotMatched,
    Unknown,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SitemapFindingAssessmentSelection {
    #[default]
    Match,
    NotMatched,
    Unknown,
    All,
}
impl SitemapFindingAssessmentSelection {
    pub fn accepts(self, verdict: SitemapFindingVerdict) -> bool {
        matches!(self, Self::All)
            || matches!(
                (self, verdict),
                (Self::Match, SitemapFindingVerdict::Match)
                    | (Self::NotMatched, SitemapFindingVerdict::NotMatched)
                    | (Self::Unknown, SitemapFindingVerdict::Unknown)
            )
    }
}

/// Exact selectors apply only to their own result domain. Search is a literal,
/// case-sensitive substring of the original requested/task identity, not aliases.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SitemapFindingSelection {
    pub filter: SitemapFindingFilter,
    pub assessment: SitemapFindingAssessmentSelection,
    pub requested_url: Option<String>,
    pub source_task_url: Option<String>,
    pub search: Option<String>,
}
impl SitemapFindingSelection {
    pub fn validate(&self) -> anyhow::Result<()> {
        let domain = self.filter.domain();
        if (domain == SitemapFindingDomain::SavedPage && self.source_task_url.is_some())
            || (domain == SitemapFindingDomain::SourceAttempt && self.requested_url.is_some())
        {
            return Err(SitemapFindingsError::InvalidSelection("selector domain differs").into());
        }
        for (value, max) in [
            (&self.requested_url, 65_536),
            // Actual legacy discovery tasks have an aggregate 32MiB limit,
            // not the measurement identity's separate 8KiB limit.
            (&self.source_task_url, 32 * 1024 * 1024),
            (&self.search, 1_024),
        ] {
            if value
                .as_ref()
                .is_some_and(|v| v.is_empty() || v.len() > max)
            {
                return Err(SitemapFindingsError::InvalidSelection("selector text budget").into());
            }
        }
        Ok(())
    }
    pub(crate) fn accepts_identity(&self, original: &str) -> bool {
        self.requested_url
            .as_deref()
            .or(self.source_task_url.as_deref())
            .is_none_or(|expected| expected == original)
            && self
                .search
                .as_deref()
                .is_none_or(|term| original.contains(term))
    }
}

/// Positions belong to this findings role. Saved ordinals are actual zero-based
/// report positions; measurement/document sequences and task ordinals start at1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SitemapFindingPosition {
    SavedPage { ordinal: u64 },
    SourceMeasurement { sequence: u64 },
    LegacySourceDocument { sequence: u64 },
    UnmeasuredSourceTask { ordinal: u64 },
}
impl SitemapFindingPosition {
    pub fn domain(&self) -> SitemapFindingDomain {
        match self {
            Self::SavedPage { .. } => SitemapFindingDomain::SavedPage,
            _ => SitemapFindingDomain::SourceAttempt,
        }
    }
    pub fn validate(&self, domain: SitemapFindingDomain) -> anyhow::Result<()> {
        let (value, zero_allowed) = match self {
            Self::SavedPage { ordinal } => (*ordinal, true),
            Self::SourceMeasurement { sequence } | Self::LegacySourceDocument { sequence } => {
                (*sequence, false)
            }
            Self::UnmeasuredSourceTask { ordinal } => (*ordinal, false),
        };
        if self.domain() != domain || (!zero_allowed && value == 0) || i64::try_from(value).is_err()
        {
            return Err(SitemapFindingsError::InvalidContinuation.into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SitemapFindingsRequest {
    pub selection: SitemapFindingSelection,
    pub limit: usize,
    pub candidate_limit: usize,
    pub after: Option<SitemapFindingPosition>,
    pub expected_context_sha256: Option<String>,
}
impl Default for SitemapFindingsRequest {
    fn default() -> Self {
        Self {
            selection: Default::default(),
            limit: 100,
            candidate_limit: MAX_SITEMAP_FINDINGS_CANDIDATES_PER_PAGE,
            after: None,
            expected_context_sha256: None,
        }
    }
}
impl SitemapFindingsRequest {
    pub fn validate(&self) -> anyhow::Result<()> {
        self.selection.validate()?;
        if !(1..=200).contains(&self.limit)
            || !(1..=MAX_SITEMAP_FINDINGS_CANDIDATES_PER_PAGE).contains(&self.candidate_limit)
        {
            return Err(SitemapFindingsError::InvalidSelection("page or work-window limit").into());
        }
        if let Some(after) = &self.after {
            after.validate(self.selection.filter.domain())?;
            if self.expected_context_sha256.is_none() {
                return Err(SitemapFindingsError::InvalidContinuation.into());
            }
        }
        if self
            .expected_context_sha256
            .as_deref()
            .is_some_and(|v| !valid_hash(v))
        {
            return Err(SitemapFindingsError::InvalidContinuation.into());
        }
        Ok(())
    }
}
pub(crate) fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Additional actual saved fields used only by the new predicate. The legacy
/// inventory/target DTO and its serialized bytes are not changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SitemapFindingPageCandidate {
    pub finding: SitemapSavedPageFinding,
    pub content_type: String,
    pub robots_warning_field_observed: bool,
    pub robots_warning: Option<String>,
    pub pdf_field_observed: bool,
    pub pdf_record_present: bool,
    pub pdf_status: Option<ResourceAnalysisStatus>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SitemapSourceFindingEvidence {
    Measured {
        record: Box<SitemapSourceStoredMeasurement>,
        protocol: SitemapSourceProtocolAssessment,
        association: Option<Box<SitemapSourceCommitAssociation>>,
        committed_document: Option<Box<SitemapSourceDocument>>,
    },
    LegacyDocument {
        document: Box<SitemapSourceDocument>,
    },
    UnmeasuredTask {
        task: SitemapCoverageTask,
    },
}
impl SitemapSourceFindingEvidence {
    pub fn original_task_url(&self) -> &str {
        match self {
            Self::Measured { record, .. } => &record.measurement.payload.binding.requested_task_url,
            Self::LegacyDocument { document } => &document.task_url,
            Self::UnmeasuredTask { task } => &task.task_url,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SitemapFindingReason {
    ListedOriginal,
    NotListedCurrentDeclaredSources,
    NotListedHistoricalDeclaredSources,
    MembershipUnknown,
    ObservedMultipleOriginalSources,
    CompleteCohortAtMostOneOriginalSource,
    SourceCohortIncomplete,
    NoObservedNonselfAnchors,
    ObservedNonselfAnchors,
    CapturedGraphOnly,
    OriginalRobotsDisallow,
    RobotsPolicyUnavailable,
    RobotsEvidenceUnobserved,
    OriginalRedirect,
    OriginalNon200,
    ObservedNonIndexability,
    IndexabilityReasonsUnavailable,
    CompleteIndexableObservation,
    PdfAnalysisIncomplete,
    PdfAnalysisUnobserved,
    TargetFetchFailed,
    TargetObservationIncomplete,
    TargetNoResponse,
    TargetPending,
    TargetUnobserved,
    RenderedNavigationChanged,
    RenderedResponseNon200,
    OriginalLocationUnavailable,
    SourceObservedProtocolExcess,
    SourceWithinProtocolLimit,
    SourceProtocolUnknown,
    MeasurementUnavailable,
    PendingSourceTask,
    DoneSourceWithoutDocument,
    LegacyDocumentWithoutMeasurement,
    CatalogUnavailable,
    SavedPageAnalysisUnavailable,
    SourceHistoryUnknown,
    SourceCaptureOmission,
    SourceTaskInventoryUnavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SitemapFindingAssessment {
    pub verdict: SitemapFindingVerdict,
    pub reasons: Vec<SitemapFindingReason>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "domain", rename_all = "snake_case")]
pub enum SitemapFindingEvidence {
    SavedPage {
        page: Box<SitemapFindingPageCandidate>,
    },
    SourceAttempt {
        source: Box<SitemapSourceFindingEvidence>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SitemapFindingRow {
    pub position: SitemapFindingPosition,
    pub assessment: SitemapFindingAssessment,
    pub evidence: SitemapFindingEvidence,
}

/// Frontier facts concern the actual current connection, even for inherited
/// source captures. Missing history and a global omission are not fake attempts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SitemapSourceFindingsScope {
    pub current_task_inventory_available: bool,
    pub current_task_inventory_sha256: Option<String>,
    pub current_frontier_task_count: Option<u64>,
    pub measurement_history_available: bool,
    pub unknown_history: bool,
    pub omission: Option<SitemapSourceStoreOmission>,
    pub inherited: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SitemapFindingsAvailability {
    Available,
    AvailableWithUnknownHistory,
    Unavailable,
}
#[derive(Debug, Clone, Serialize)]
pub struct SitemapFindingsContext {
    pub engine: String,
    pub selection_sha256: String,
    pub context_sha256: String,
    pub availability: SitemapFindingsAvailability,
    pub reasons: Vec<SitemapFindingReason>,
    pub basis: SitemapVerifiedReadHeader,
    pub source_scope: Option<SitemapSourceFindingsScope>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SitemapFindingsStop {
    Exhausted,
    MoreMatches,
    CandidateBudget,
    Unavailable,
}
#[derive(Debug, Clone, Serialize)]
pub struct SitemapFindingsScan {
    pub evaluated_candidates: u64,
    pub exhausted: bool,
    pub stop: SitemapFindingsStop,
    /// None at a work-window stop: remaining candidates may contain matches.
    pub more_matches: Option<bool>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SitemapFindingsContinuation {
    pub context_sha256: String,
    pub after: SitemapFindingPosition,
    pub reason: SitemapFindingsStop,
}
#[derive(Debug, Clone, Serialize)]
pub struct SitemapFindingsPage {
    pub context: SitemapFindingsContext,
    /// None is unavailable. Some([]) is an evaluated stored selection; its
    /// scan/context still distinguish incomplete work and unknown source history.
    pub rows: Option<Vec<SitemapFindingRow>>,
    pub scan: SitemapFindingsScan,
    pub continuation: Option<SitemapFindingsContinuation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SitemapFindingsError {
    InvalidSelection(&'static str),
    InvalidContinuation,
    StaleContext,
    ResponseByteBudget,
    SelectionByteBudget,
    Unavailable,
}
impl fmt::Display for SitemapFindingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSelection(reason) => {
                write!(f, "Invalid sitemap finding selection: {reason}")
            }
            Self::InvalidContinuation => f.write_str("Invalid sitemap finding continuation"),
            Self::StaleContext => f.write_str("Stale sitemap finding context"),
            Self::ResponseByteBudget => {
                f.write_str("Sitemap finding response byte budget exceeded")
            }
            Self::SelectionByteBudget => {
                f.write_str("Sitemap finding selection byte budget exceeded")
            }
            Self::Unavailable => f.write_str("Sitemap finding evidence unavailable"),
        }
    }
}
impl Error for SitemapFindingsError {}
