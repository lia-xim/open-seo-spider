//! Pure, deterministic generation from captured report facts and explicit options.
//! No HTTP, current time, guessed content freshness, or writes to a caller path.
use crate::{
    CrawlReport, PageRecord, SitemapLastmodValueAssessment, SitemapLedgerRow, SitemapLocationRole,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Write},
};
use url::Url;

pub const SITEMAP_GENERATION_ENGINE: &str = "open-seo-spider/segmented-sitemap-generation-v1";
pub const SITEMAP_GENERATION_MAX_URLS: usize = 50_000;
pub const SITEMAP_GENERATION_MAX_XML_BYTES: usize = 52_428_800;
const MAX_TOTAL: usize = 256 * 1024 * 1024;
const MAX_PAGES: usize = 1_000_000;
const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
const NS: &str = "http://www.sitemaps.org/schemas/sitemap/0.9";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SitemapChangeFrequency {
    Always,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Never,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapProvidedLastmod {
    pub page_url: String,
    pub value: String,
    pub reference: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum SitemapGenerationLastmod {
    #[default]
    Omit,
    Provided {
        values: Vec<SitemapProvidedLastmod>,
    },
    CapturedSitemap,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SitemapGenerationOptions {
    pub public_base_url: String,
    pub max_urls_per_file: usize,
    pub max_urlset_uncompressed_bytes: usize,
    pub max_index_uncompressed_bytes: usize,
    pub max_total_output_bytes: usize,
    pub include_images: bool,
    pub allow_external_images: bool,
    pub include_hreflang: bool,
    pub lastmod: SitemapGenerationLastmod,
    pub changefreq: Option<SitemapChangeFrequency>,
    pub priority: Option<String>,
}
impl Default for SitemapGenerationOptions {
    fn default() -> Self {
        Self {
            public_base_url: String::new(),
            max_urls_per_file: 50_000,
            max_urlset_uncompressed_bytes: 52_428_800,
            max_index_uncompressed_bytes: 52_428_800,
            max_total_output_bytes: 64 * 1024 * 1024,
            include_images: false,
            allow_external_images: false,
            include_hreflang: false,
            lastmod: Default::default(),
            changefreq: None,
            priority: None,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SitemapGenerationDiagnosticCode {
    InvalidRequestedUrl,
    DuplicateRequestedUrl,
    WrongOrigin,
    OutsidePublicBasePath,
    NotInternal,
    Redirected,
    NotHttp200,
    NonIndexable,
    NonDocument,
    FailedOrIncomplete,
    RenderedObservationFailed,
    RenderedStatusNot200,
    RenderedNavigationChanged,
    RenderedObservationIncomplete,
    InvalidImageUrl,
    ExternalImageOmitted,
    DuplicateImageUrl,
    ImageVerificationNotProven,
    InvalidHreflangUrl,
    InvalidHreflangTag,
    HreflangSelfMissing,
    HreflangConflictingLanguage,
    HreflangDuplicate,
    HreflangReciprocityNotProven,
    CapturedLastmodUnavailable,
    CapturedLastmodInvalid,
    CapturedLastmodAmbiguous,
    ProvidedLastmodUnused,
    CapturedLastmodUnused,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapGenerationDiagnostic {
    pub code: SitemapGenerationDiagnosticCode,
    pub page_ordinal: Option<usize>,
    pub value: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum SitemapGeneratedLastmodSource {
    Provided {
        reference: String,
        original_value: String,
    },
    CapturedSitemap {
        observation_id: String,
        entry_id: String,
        field_ordinal: usize,
        original_value: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapGeneratedLastmod {
    pub value: String,
    pub source: SitemapGeneratedLastmodSource,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapGeneratedAlternate {
    pub language: String,
    pub url: String,
    pub source_ordinals: Vec<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapGeneratedMembership {
    pub url: String,
    pub page_ordinal: usize,
    pub segment: String,
    pub images: Vec<String>,
    pub hreflang: Vec<SitemapGeneratedAlternate>,
    pub lastmod: Option<SitemapGeneratedLastmod>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapGeneratedFileDescriptor {
    pub name: String,
    pub public_url: Option<String>,
    pub kind: String,
    pub url_count: usize,
    pub uncompressed_bytes: usize,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapGenerationManifest {
    pub schema_version: u8,
    pub engine: String,
    pub report_sha256: String,
    pub report_sha_profile: String,
    pub options: SitemapGenerationOptions,
    pub options_sha256: String,
    pub files: Vec<SitemapGeneratedFileDescriptor>,
    pub memberships: Vec<SitemapGeneratedMembership>,
    pub diagnostics: Vec<SitemapGenerationDiagnostic>,
    pub excluded_pages: usize,
    pub input_pages: usize,
    pub source_scope: String,
    pub negative_membership_claim: bool,
    pub hreflang_validation_profile: String,
    pub selected_recrawl: bool,
}
#[derive(Debug, Clone)]
pub struct SitemapGeneratedFile {
    pub descriptor: SitemapGeneratedFileDescriptor,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone)]
pub struct SitemapGeneratedBundle {
    pub manifest: SitemapGenerationManifest,
    pub files: Vec<SitemapGeneratedFile>,
    pub zip_bytes: Vec<u8>,
    pub zip_sha256: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapGenerationFailureCode {
    InvalidOptions,
    InvalidLastmod,
    InvalidSource,
    InputBudget,
    EntryByteLimit,
    TotalOutputBudget,
    IndexLimit,
    NoEligibleUrls,
    ZipFailure,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapGenerationFailure {
    pub code: SitemapGenerationFailureCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub diagnostic_counts: BTreeMap<SitemapGenerationDiagnosticCode, u64>,
}
impl std::fmt::Display for SitemapGenerationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}
impl std::error::Error for SitemapGenerationFailure {}
type Result<T> = std::result::Result<T, SitemapGenerationFailure>;
fn fail(
    code: SitemapGenerationFailureCode,
    message: impl Into<String>,
) -> SitemapGenerationFailure {
    SitemapGenerationFailure {
        code,
        message: message.into(),
        diagnostic_counts: BTreeMap::new(),
    }
}
fn sha(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
fn json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| {
        fail(
            SitemapGenerationFailureCode::InvalidSource,
            "Serialization failed",
        )
    })
}
struct ReportHashWriter {
    count: usize,
    hash: Sha256,
}
impl Write for ReportHashWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let count = self
            .count
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("Typed input byte overflow"))?;
        if count > MAX_TOTAL {
            return Err(std::io::Error::other("Typed input report exceeds budget"));
        }
        self.count = count;
        self.hash.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn report_digest(report: &CrawlReport) -> Result<String> {
    let mut writer = std::io::BufWriter::with_capacity(
        65536,
        ReportHashWriter {
            count: 0,
            hash: Sha256::new(),
        },
    );
    serde_json::to_writer(&mut writer, report).map_err(|_| {
        fail(
            SitemapGenerationFailureCode::InputBudget,
            "Typed input report exceeds bounded serialization",
        )
    })?;
    writer.flush().map_err(|_| {
        fail(
            SitemapGenerationFailureCode::InputBudget,
            "Typed input report exceeds bounded serialization",
        )
    })?;
    let hashed = writer.into_inner().map_err(|_| {
        fail(
            SitemapGenerationFailureCode::InputBudget,
            "Typed input hash flush failed",
        )
    })?;
    Ok(format!("{:x}", hashed.hash.finalize()))
}
/// RFC3986 percent-triplet syntax only. Keep encoded octets and hex case as
/// recorded; URL identity must not be rewritten by decoding or double escaping.
fn valid_percent_escapes(value: &str) -> bool {
    let bytes = value.as_bytes();
    for (offset, byte) in bytes.iter().enumerate() {
        if *byte == b'%'
            && !bytes
                .get(offset + 1..offset + 3)
                .is_some_and(|digits| digits.iter().all(u8::is_ascii_hexdigit))
        {
            return false;
        }
    }
    true
}
fn url(value: &str) -> Option<Url> {
    if value.chars().count() >= 2048
        || value.chars().any(char::is_control)
        || !valid_percent_escapes(value)
    {
        return None;
    }
    crate::normalize_url(value, None).filter(|parsed| parsed.as_str() == value)
}
/// A fragment identifies a client-side location, not another HTTP fetch target.
/// Validate the complete raw observation before discarding its fragment on a
/// local parsed copy; never trim controls, repair malformed URLs, or rewrite
/// the stored renderer fact. The emitted page URL still obeys `url` bounds.
fn rendered_http_fetch_identity(value: &str) -> Option<Url> {
    if value.chars().any(char::is_control) || !valid_percent_escapes(value) {
        return None;
    }
    let mut parsed = Url::parse(value).ok()?;
    if parsed.as_str() != value {
        return None;
    }
    parsed.set_fragment(None);
    url(parsed.as_str())
}
fn base(options: &SitemapGenerationOptions) -> Result<Url> {
    use SitemapGenerationFailureCode as F;
    if options.max_urls_per_file == 0
        || options.max_urls_per_file > SITEMAP_GENERATION_MAX_URLS
        || options.max_urlset_uncompressed_bytes == 0
        || options.max_urlset_uncompressed_bytes > SITEMAP_GENERATION_MAX_XML_BYTES
        || options.max_index_uncompressed_bytes == 0
        || options.max_index_uncompressed_bytes > SITEMAP_GENERATION_MAX_XML_BYTES
        || options.max_total_output_bytes == 0
        || options.max_total_output_bytes > MAX_TOTAL
        || (!options.include_images && options.allow_external_images)
    {
        return Err(fail(
            F::InvalidOptions,
            "Invalid protocol/output/options limits",
        ));
    }
    let base = url(&options.public_base_url).ok_or_else(|| {
        fail(
            F::InvalidOptions,
            "Public base must be a canonical HTTP(S) URL shorter than 2048 characters",
        )
    })?;
    if base.query().is_some()
        || base.fragment().is_some()
        || !base.path().ends_with('/')
        || base.path().split('/').any(|part| {
            let lower = part.to_ascii_lowercase();
            lower.contains("%2f")
                || lower.contains("%5c")
                || lower.contains("%2e")
                || part == "."
                || part == ".."
                || part.contains('\\')
        })
    {
        return Err(fail(
            F::InvalidOptions,
            "Public base must be a safe directory path without query, fragment, encoded separators or dot segments",
        ));
    }
    if let Some(priority) = &options.priority
        && !valid_priority(priority)
    {
        return Err(fail(
            F::InvalidOptions,
            "Priority must be an XML decimal from 0 to 1 within 4096 bytes",
        ));
    }
    // Verify fixed output names themselves fit the protocol URL character bound.
    if url(base
        .join("sitemap-50000.xml")
        .map_err(|_| fail(F::InvalidOptions, "Public base join failed"))?
        .as_str())
    .is_none()
    {
        return Err(fail(
            F::InvalidOptions,
            "Public segment URL exceeds protocol URL limit",
        ));
    }
    Ok(base)
}
fn valid_priority(value: &str) -> bool {
    if value.len() > 4096 {
        return false;
    }
    let value = value.trim_matches([' ', '\t', '\n', '\r']);
    let negative = value.starts_with('-');
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    let parts: Vec<_> = unsigned.split('.').collect();
    if parts.is_empty() || parts.len() > 2 {
        return false;
    }
    let integer = parts[0];
    let fraction = parts.get(1).copied().unwrap_or("");
    if integer.is_empty() && fraction.is_empty() {
        return false;
    }
    if !integer
        .bytes()
        .chain(fraction.bytes())
        .all(|b| b.is_ascii_digit())
    {
        return false;
    }
    let integer = integer.trim_start_matches('0');
    if !matches!(integer, "" | "1") {
        return false;
    }
    if integer == "1" && !fraction.bytes().all(|b| b == b'0') {
        return false;
    }
    !negative || (integer.is_empty() && fraction.bytes().all(|b| b == b'0'))
}
fn diagnostic(
    out: &mut Vec<SitemapGenerationDiagnostic>,
    code: SitemapGenerationDiagnosticCode,
    ordinal: Option<usize>,
    value: Option<&str>,
) {
    out.push(SitemapGenerationDiagnostic {
        code,
        page_ordinal: ordinal,
        value: value.map(str::to_owned),
    });
}
fn lexical_lastmod(value: &str) -> Result<String> {
    if value.len() > 4096 {
        return Err(fail(
            SitemapGenerationFailureCode::InvalidLastmod,
            "Lastmod value exceeds 4096 bytes",
        ));
    }
    let assessed =
        crate::assess_sitemap_lastmod_value(value, &Default::default()).map_err(|_| {
            fail(
                SitemapGenerationFailureCode::InvalidLastmod,
                "Lastmod lexical budget failed",
            )
        })?;
    if !matches!(assessed.xsd, SitemapLastmodValueAssessment::Valid { .. }) {
        return Err(fail(
            SitemapGenerationFailureCode::InvalidLastmod,
            "Lastmod must be an actual valid Sitemap XSD date/dateTime value",
        ));
    }
    Ok(assessed.evaluated_value)
}
/// Validate generation options without reading a report, making HTTP requests, or creating files.
pub fn validate_sitemap_generation_options(options: &SitemapGenerationOptions) -> Result<()> {
    base(options)?;
    if let SitemapGenerationLastmod::Provided { values } = &options.lastmod {
        provided_lastmods(values)?;
    }
    Ok(())
}
fn provided_lastmods(
    provided: &[SitemapProvidedLastmod],
) -> Result<BTreeMap<String, SitemapGeneratedLastmod>> {
    let mut values = BTreeMap::new();
    if provided.len() > MAX_PAGES {
        return Err(fail(
            SitemapGenerationFailureCode::InputBudget,
            "Too many provided lastmod values",
        ));
    }
    let mut bytes = 0usize;
    for item in provided {
        bytes = bytes
            .checked_add(item.page_url.len() + item.value.len() + item.reference.len())
            .ok_or_else(|| {
                fail(
                    SitemapGenerationFailureCode::InputBudget,
                    "Lastmod input overflow",
                )
            })?;
        if bytes > 32 * 1024 * 1024
            || url(&item.page_url).is_none()
            || item.reference.is_empty()
            || item.reference.len() > 1024
            || item.reference.chars().any(char::is_control)
        {
            return Err(fail(
                SitemapGenerationFailureCode::InvalidLastmod,
                "Lastmod needs canonical page identity and a bounded explicit source reference",
            ));
        }
        let value = lexical_lastmod(&item.value)?;
        if values
            .insert(
                item.page_url.clone(),
                SitemapGeneratedLastmod {
                    value,
                    source: SitemapGeneratedLastmodSource::Provided {
                        reference: item.reference.clone(),
                        original_value: item.value.clone(),
                    },
                },
            )
            .is_some()
        {
            return Err(fail(
                SitemapGenerationFailureCode::InvalidLastmod,
                "Duplicate provided lastmod identity",
            ));
        }
    }
    Ok(values)
}
fn lastmods(
    report: &CrawlReport,
    options: &SitemapGenerationOptions,
    diagnostics: &mut Vec<SitemapGenerationDiagnostic>,
) -> Result<BTreeMap<String, SitemapGeneratedLastmod>> {
    use SitemapGenerationDiagnosticCode as D;
    let mut values = BTreeMap::new();
    match &options.lastmod {
        SitemapGenerationLastmod::Omit => {}
        SitemapGenerationLastmod::Provided { values: provided } => {
            values = provided_lastmods(provided)?;
        }
        SitemapGenerationLastmod::CapturedSitemap => {
            let Some(catalog) = &report.sitemap_catalog else {
                diagnostic(diagnostics, D::CapturedLastmodUnavailable, None, None);
                return Ok(values);
            };
            crate::sitemap_ledger::validate_catalog(catalog).map_err(|_| {
                fail(
                    SitemapGenerationFailureCode::InvalidSource,
                    "Captured sitemap catalog is invalid",
                )
            })?;
            let Some(rows) = &catalog.rows else {
                diagnostic(diagnostics, D::CapturedLastmodUnavailable, None, None);
                return Ok(values);
            };
            let documents = rows
                .iter()
                .filter_map(|row| {
                    if let SitemapLedgerRow::Document(doc) = row {
                        Some((doc.observation_id.as_str(), doc.complete()))
                    } else {
                        None
                    }
                })
                .collect::<BTreeMap<_, _>>();
            let entries = rows
                .iter()
                .filter_map(|row| {
                    if let SitemapLedgerRow::Entry(entry) = row {
                        Some((
                            (entry.observation_id.as_str(), entry.observed.ordinal),
                            entry,
                        ))
                    } else {
                        None
                    }
                })
                .collect::<BTreeMap<_, _>>();
            // Identity ambiguity precedes date eligibility. Missing, invalid, incomplete
            // or multi-date declarations must not disappear and make a contrary valid
            // date appear uniquely sourced. Count original Page Loc occurrences only;
            // never join HTTP final URLs or redirect aliases.
            let mut occurrences = BTreeMap::<&str, usize>::new();
            for row in rows {
                if let SitemapLedgerRow::Location(location) = row
                    && location.observed.role == SitemapLocationRole::PageLocation
                    && let Some(identity) = location.observed.normalized_fetch_url.as_deref()
                {
                    *occurrences.entry(identity).or_default() += 1;
                }
            }
            let mut ambiguous = occurrences
                .into_iter()
                .filter(|(_, count)| *count > 1)
                .map(|(identity, _)| identity.to_owned())
                .collect::<BTreeSet<_>>();
            for row in rows {
                let SitemapLedgerRow::Location(location) = row else {
                    continue;
                };
                if location.observed.role != SitemapLocationRole::PageLocation {
                    continue;
                }
                let Some(page_url) = location.observed.normalized_fetch_url.as_ref() else {
                    continue;
                };
                let Some(entry) = entries.get(&(
                    location.observation_id.as_str(),
                    location.observed.entry_ordinal,
                )) else {
                    return Err(fail(
                        SitemapGenerationFailureCode::InvalidSource,
                        "Captured lastmod entry missing",
                    ));
                };
                if entry.observed.lastmod.is_empty() {
                    continue;
                }
                if entry.observed.lastmod.len() != 1
                    || entry.observed.loc_occurrence_ordinals.len() != 1
                    || location.observed.validation_error.is_some()
                    || location.observed.loc_ordinal.is_none()
                    || !entry.observed.complete
                    || documents.get(entry.observation_id.as_str()) != Some(&true)
                {
                    diagnostic(diagnostics, D::CapturedLastmodInvalid, None, Some(page_url));
                    continue;
                }
                let field = &entry.observed.lastmod[0];
                let Ok(value) = lexical_lastmod(&field.decoded_value) else {
                    diagnostic(diagnostics, D::CapturedLastmodInvalid, None, Some(page_url));
                    continue;
                };
                let item = SitemapGeneratedLastmod {
                    value,
                    source: SitemapGeneratedLastmodSource::CapturedSitemap {
                        observation_id: entry.observation_id.clone(),
                        entry_id: entry.entry_id.clone(),
                        field_ordinal: field.ordinal,
                        original_value: field.decoded_value.clone(),
                    },
                };
                if values.contains_key(page_url) {
                    ambiguous.insert(page_url.clone());
                } else {
                    values.insert(page_url.clone(), item);
                }
            }
            for identity in ambiguous {
                values.remove(&identity);
                diagnostic(
                    diagnostics,
                    D::CapturedLastmodAmbiguous,
                    None,
                    Some(&identity),
                );
            }
        }
    }
    Ok(values)
}
fn images(
    page: &PageRecord,
    base: &Url,
    options: &SitemapGenerationOptions,
    ordinal: usize,
    diagnostics: &mut Vec<SitemapGenerationDiagnostic>,
) -> Result<Vec<String>> {
    use SitemapGenerationDiagnosticCode as D;
    if !options.include_images {
        return Ok(vec![]);
    }
    if page.images.len() > 100_000 {
        return Err(fail(
            SitemapGenerationFailureCode::InputBudget,
            "Captured image input limit exceeded",
        ));
    }
    let mut values = BTreeSet::new();
    for image in &page.images {
        let Some(parsed) = url(&image.url) else {
            diagnostic(
                diagnostics,
                D::InvalidImageUrl,
                Some(ordinal),
                Some(&image.url),
            );
            continue;
        };
        if parsed.origin() != base.origin() && !options.allow_external_images {
            diagnostic(
                diagnostics,
                D::ExternalImageOmitted,
                Some(ordinal),
                Some(&image.url),
            );
            continue;
        }
        if parsed.origin() != base.origin() {
            diagnostic(
                diagnostics,
                D::ImageVerificationNotProven,
                Some(ordinal),
                Some(&image.url),
            );
        }
        if !values.insert(image.url.clone()) {
            diagnostic(
                diagnostics,
                D::DuplicateImageUrl,
                Some(ordinal),
                Some(&image.url),
            );
        }
    }
    if values.len() > 1000 {
        return Err(fail(
            SitemapGenerationFailureCode::EntryByteLimit,
            "Google image sitemap limit is 1000 unique images per page; no silent truncation",
        ));
    }
    Ok(values.into_iter().collect())
}
fn language_tag(value: &str) -> bool {
    if value.eq_ignore_ascii_case("x-default") {
        return true;
    }
    let parts: Vec<_> = value.split('-').collect();
    let alpha = |s: &str, n| s.len() == n && s.bytes().all(|b| b.is_ascii_alphabetic());
    match parts.as_slice() {
        [language] => alpha(language, 2),
        [language, region] => alpha(language, 2) && (alpha(region, 2) || alpha(region, 4)),
        [language, script, region] => alpha(language, 2) && alpha(script, 4) && alpha(region, 2),
        _ => false,
    }
}
fn alternates(
    page: &PageRecord,
    ordinal: usize,
    options: &SitemapGenerationOptions,
    diagnostics: &mut Vec<SitemapGenerationDiagnostic>,
) -> Result<Vec<SitemapGeneratedAlternate>> {
    use SitemapGenerationDiagnosticCode as D;
    if !options.include_hreflang {
        return Ok(vec![]);
    }
    if page.hreflang.len() > 1000 {
        return Err(fail(
            SitemapGenerationFailureCode::InputBudget,
            "Captured hreflang input limit exceeded",
        ));
    }
    let mut values: BTreeMap<String, SitemapGeneratedAlternate> = BTreeMap::new();
    let mut invalid = false;
    for (source_ordinal, item) in page.hreflang.iter().enumerate() {
        if !language_tag(&item.language) {
            diagnostic(
                diagnostics,
                D::InvalidHreflangTag,
                Some(ordinal),
                Some(&item.language),
            );
            invalid = true;
            continue;
        }
        if url(&item.url).is_none() {
            diagnostic(
                diagnostics,
                D::InvalidHreflangUrl,
                Some(ordinal),
                Some(&item.url),
            );
            invalid = true;
            continue;
        }
        let language = item.language.to_ascii_lowercase();
        if let Some(previous) = values.get_mut(&language) {
            if previous.url != item.url {
                diagnostic(
                    diagnostics,
                    D::HreflangConflictingLanguage,
                    Some(ordinal),
                    Some(&item.language),
                );
                invalid = true;
            } else {
                previous.source_ordinals.push(source_ordinal);
                diagnostic(
                    diagnostics,
                    D::HreflangDuplicate,
                    Some(ordinal),
                    Some(&item.url),
                );
            }
        } else {
            values.insert(
                language,
                SitemapGeneratedAlternate {
                    language: item.language.clone(),
                    url: item.url.clone(),
                    source_ordinals: vec![source_ordinal],
                },
            );
        }
    }
    if values.is_empty() {
        return Ok(vec![]);
    }
    if !values.values().any(|alternate| alternate.url == page.url) {
        diagnostic(
            diagnostics,
            D::HreflangSelfMissing,
            Some(ordinal),
            Some(&page.url),
        );
        invalid = true;
    }
    if invalid {
        return Ok(vec![]);
    }
    diagnostic(
        diagnostics,
        D::HreflangReciprocityNotProven,
        Some(ordinal),
        Some(&page.url),
    );
    Ok(values.into_values().collect())
}
fn row(m: &SitemapGeneratedMembership, options: &SitemapGenerationOptions) -> String {
    let escape = |s: &str| quick_xml::escape::escape(s).into_owned();
    let mut text = format!("  <url><loc>{}</loc>", escape(&m.url));
    if let Some(date) = &m.lastmod {
        text.push_str(&format!("<lastmod>{}</lastmod>", escape(&date.value)));
    }
    if let Some(changefreq) = &options.changefreq {
        text.push_str(&format!(
            "<changefreq>{}</changefreq>",
            serde_json::to_value(changefreq)
                .expect("fixed enum")
                .as_str()
                .expect("fixed enum string")
        ));
    }
    if let Some(priority) = &options.priority {
        text.push_str(&format!("<priority>{}</priority>", escape(priority)));
    }
    for image in &m.images {
        text.push_str(&format!(
            "<image:image><image:loc>{}</image:loc></image:image>",
            escape(image)
        ));
    }
    for alt in &m.hreflang {
        text.push_str(&format!(
            "<xhtml:link rel=\"alternate\" hreflang=\"{}\" href=\"{}\"/>",
            escape(&alt.language),
            escape(&alt.url)
        ));
    }
    text.push_str("</url>\n");
    text
}
fn descriptor(
    name: String,
    public_url: Option<String>,
    kind: &str,
    count: usize,
    bytes: &[u8],
) -> SitemapGeneratedFileDescriptor {
    SitemapGeneratedFileDescriptor {
        name,
        public_url,
        kind: kind.into(),
        url_count: count,
        uncompressed_bytes: bytes.len(),
        sha256: sha(bytes),
    }
}

/// Generate everything in memory, returning only after XML, manifests and ZIP
/// close successfully. No caller-visible partial files or target URL attempts.
pub fn generate_sitemap_bundle(
    report: &CrawlReport,
    options: &SitemapGenerationOptions,
) -> Result<SitemapGeneratedBundle> {
    use SitemapGenerationDiagnosticCode as D;
    use SitemapGenerationFailureCode as F;
    let public_base = base(options)?;
    if report.pages.len() > MAX_PAGES {
        return Err(fail(
            F::InputBudget,
            "Captured report exceeds page input limit",
        ));
    }
    let report_sha256 = report_digest(report)?;
    let mut diagnostics = Vec::new();
    let mut lastmod = lastmods(report, options, &mut diagnostics)?;
    let mut frequencies = BTreeMap::new();
    for page in &report.pages {
        *frequencies.entry(page.url.as_str()).or_insert(0usize) += 1;
    }
    let mut members = Vec::new();
    let mut excluded = 0;
    for (ordinal, page) in report.pages.iter().enumerate() {
        let mut reasons = Vec::new();
        let requested = url(&page.url);
        if requested.is_none() {
            reasons.push(D::InvalidRequestedUrl);
        }
        if frequencies[page.url.as_str()] != 1 {
            reasons.push(D::DuplicateRequestedUrl);
        }
        if !page.internal {
            reasons.push(D::NotInternal);
        }
        if page.status != Some(200) || page.final_status != Some(200) {
            reasons.push(D::NotHttp200);
        }
        if !page.redirect_chain.is_empty() || page.final_url != page.url {
            reasons.push(D::Redirected);
        }
        if !page.indexable {
            reasons.push(D::NonIndexable);
        }
        if !page.is_indexable_document() {
            reasons.push(D::NonDocument);
        }
        if page.error.is_some() || page.truncated || page.blocked_by_robots {
            reasons.push(D::FailedOrIncomplete);
        }
        if page.renderer_error.is_some() {
            reasons.push(D::RenderedObservationFailed);
        }
        if page.rendered {
            if page.rendered_status != Some(200) {
                reasons.push(D::RenderedStatusNot200);
            }
            let final_fetch = url(&page.final_url);
            match page
                .rendered_url
                .as_deref()
                .and_then(rendered_http_fetch_identity)
            {
                Some(rendered) if final_fetch.as_ref() != Some(&rendered) => {
                    reasons.push(D::RenderedNavigationChanged)
                }
                None => reasons.push(D::RenderedObservationIncomplete),
                _ => {}
            }
        } else if page.is_html() && report.config.renderer.is_some() {
            reasons.push(D::RenderedObservationIncomplete);
        }
        if let Some(requested) = requested {
            if requested.origin() != public_base.origin() {
                reasons.push(D::WrongOrigin);
            }
            if !requested.path().starts_with(public_base.path()) {
                reasons.push(D::OutsidePublicBasePath);
            }
        }
        if !reasons.is_empty() {
            excluded += 1;
            for reason in reasons {
                diagnostic(&mut diagnostics, reason, Some(ordinal), Some(&page.url));
            }
            continue;
        }
        members.push(SitemapGeneratedMembership {
            url: page.url.clone(),
            page_ordinal: ordinal,
            segment: String::new(),
            images: images(page, &public_base, options, ordinal, &mut diagnostics)?,
            hreflang: alternates(page, ordinal, options, &mut diagnostics)?,
            lastmod: lastmod.remove(&page.url),
        });
    }
    for unused in lastmod.keys() {
        diagnostic(
            &mut diagnostics,
            if matches!(options.lastmod, SitemapGenerationLastmod::Provided { .. }) {
                D::ProvidedLastmodUnused
            } else {
                D::CapturedLastmodUnused
            },
            None,
            Some(unused),
        );
    }
    if members.is_empty() {
        let mut error = fail(
            F::NoEligibleUrls,
            format!(
                "No eligible captured document URLs ({excluded} excluded pages, {} diagnostics)",
                diagnostics.len()
            ),
        );
        for item in diagnostics {
            *error.diagnostic_counts.entry(item.code).or_default() += 1;
        }
        return Err(error);
    }
    members.sort_by(|a, b| a.url.cmp(&b.url));
    let mut opening = format!("{XML}<urlset xmlns=\"{NS}\"");
    if options.include_images {
        opening.push_str(" xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\"");
    }
    if options.include_hreflang {
        opening.push_str(" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\"");
    }
    opening.push_str(">\n");
    let closing = "</urlset>\n";
    let overhead = opening.len() + closing.len();
    let mut files = Vec::new();
    let mut current = String::new();
    let mut count = 0;
    let mut total = 0usize;
    let mut segment = 1usize;
    let finish = |current: &mut String,
                  count: &mut usize,
                  segment: usize,
                  files: &mut Vec<SitemapGeneratedFile>,
                  total: &mut usize|
     -> Result<()> {
        if *count == 0 {
            return Ok(());
        }
        let bytes = format!("{opening}{current}{closing}").into_bytes();
        *total = total
            .checked_add(bytes.len())
            .ok_or_else(|| fail(F::TotalOutputBudget, "Output byte overflow"))?;
        if *total > options.max_total_output_bytes {
            return Err(fail(
                F::TotalOutputBudget,
                "XML output exceeds total budget",
            ));
        }
        let name = format!("sitemap-{segment:05}.xml");
        let public_url = public_base
            .join(&name)
            .map_err(|_| fail(F::InvalidOptions, "Segment URL join failed"))?
            .to_string();
        files.push(SitemapGeneratedFile {
            descriptor: descriptor(name, Some(public_url), "urlset", *count, &bytes),
            bytes,
        });
        current.clear();
        *count = 0;
        Ok(())
    };
    for member in &mut members {
        let text = row(member, options);
        if text.len() + overhead > options.max_urlset_uncompressed_bytes {
            return Err(fail(
                F::EntryByteLimit,
                "One URL with extensions exceeds uncompressed file byte limit",
            ));
        }
        if count == options.max_urls_per_file
            || current.len() + text.len() + overhead > options.max_urlset_uncompressed_bytes
        {
            finish(&mut current, &mut count, segment, &mut files, &mut total)?;
            segment += 1;
            if segment > 50_000 {
                return Err(fail(
                    F::IndexLimit,
                    "Sitemap index cannot list more than 50,000 segments",
                ));
            }
        }
        member.segment = format!("sitemap-{segment:05}.xml");
        current.push_str(&text);
        count += 1;
    }
    finish(&mut current, &mut count, segment, &mut files, &mut total)?;
    let mut index = format!("{XML}<sitemapindex xmlns=\"{NS}\">\n");
    for file in &files {
        index.push_str(&format!(
            "  <sitemap><loc>{}</loc></sitemap>\n",
            quick_xml::escape::escape(file.descriptor.public_url.as_deref().expect("segment URL"))
        ));
    }
    index.push_str("</sitemapindex>\n");
    if index.len() > options.max_index_uncompressed_bytes {
        return Err(fail(
            F::IndexLimit,
            "Sitemap index exceeds protocol byte limit",
        ));
    }
    let index_bytes = index.into_bytes();
    total = total
        .checked_add(index_bytes.len())
        .ok_or_else(|| fail(F::TotalOutputBudget, "Output overflow"))?;
    files.push(SitemapGeneratedFile {
        descriptor: descriptor(
            "sitemap-index.xml".into(),
            Some(
                public_base
                    .join("sitemap-index.xml")
                    .map_err(|_| fail(F::InvalidOptions, "Index URL join failed"))?
                    .to_string(),
            ),
            "sitemapindex",
            segment,
            &index_bytes,
        ),
        bytes: index_bytes,
    });
    let manifest=SitemapGenerationManifest{schema_version:1,engine:SITEMAP_GENERATION_ENGINE.into(),report_sha256,report_sha_profile:"typed-crawl-report-canonical-rust-serde-json-v1".into(),options:options.clone(),options_sha256:sha(json(options)?),files:files.iter().map(|file|file.descriptor.clone()).collect(),memberships:members,diagnostics,excluded_pages:excluded,input_pages:report.pages.len(),source_scope:if report.operation.is_some(){"mixed_age_selected_recrawl; stored unselected inherited observations may be included; no current freshness/full-site completeness inference"}else{"captured_static_report_documents; no current freshness/full-site completeness inference"}.into(),selected_recrawl:report.operation.is_some(),negative_membership_claim:false,hreflang_validation_profile:"HTTP(S) identity + language/script/region lexical grammar + captured self-link/conflict checks; ISO registry assignments and reciprocity are not attested".into()};
    let manifest_bytes = json(&manifest)?;
    total = total
        .checked_add(manifest_bytes.len())
        .ok_or_else(|| fail(F::TotalOutputBudget, "Manifest byte overflow"))?;
    if total > options.max_total_output_bytes {
        return Err(fail(
            F::TotalOutputBudget,
            "Complete XML plus exact membership manifest exceeds total budget",
        ));
    }
    files.push(SitemapGeneratedFile {
        descriptor: descriptor(
            "manifest.json".into(),
            None,
            "manifest",
            manifest.memberships.len(),
            &manifest_bytes,
        ),
        bytes: manifest_bytes,
    });
    let cursor = Cursor::new(Vec::with_capacity(total));
    let mut zip = zip::ZipWriter::new(cursor);
    let file_options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(
            zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
                .map_err(|_| fail(F::ZipFailure, "Fixed archive date failed"))?,
        )
        .unix_permissions(0o644);
    for file in &files {
        zip.start_file(&file.descriptor.name, file_options)
            .map_err(|_| fail(F::ZipFailure, "Archive entry failed"))?;
        zip.write_all(&file.bytes)
            .map_err(|_| fail(F::ZipFailure, "Archive write failed"))?;
    }
    let zip_bytes = zip
        .finish()
        .map_err(|_| fail(F::ZipFailure, "Archive close failed"))?
        .into_inner();
    if zip_bytes.len() > options.max_total_output_bytes {
        return Err(fail(
            F::TotalOutputBudget,
            "Complete ZIP including file headers exceeds output budget",
        ));
    }
    let zip_sha256 = sha(&zip_bytes);
    Ok(SitemapGeneratedBundle {
        manifest,
        files,
        zip_bytes,
        zip_sha256,
    })
}

#[cfg(test)]
#[path = "sitemap_generation_tests.rs"]
mod tests;
