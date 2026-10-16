use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CrawlConfig {
    /// Crawl-scoped trust and optional learned HSTS. Inactive values preserve legacy bytes.
    #[serde(default, skip_serializing_if = "crate::TlsConfig::inactive_option")]
    pub tls: Option<crate::TlsConfig>,
    pub seed_urls: Vec<String>,
    /// Declared original input occurrences; network deduplication is separate.
    pub input_imports: Vec<crate::ListImportResult>,
    pub sitemap_urls: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explicit_sitemap_inputs: Option<Vec<String>>,
    pub discover_sitemaps: bool,
    pub max_sitemap_documents: usize,
    /// Explicit source-only work budgets. None preserves the legacy page-body
    /// clamp and parser defaults; retained membership/catalog budgets are separate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sitemap_source_processing: Option<crate::SitemapSourceProcessingConfig>,
    /// Explicit durable membership-catalog capacity, separate from source work and wire limits.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_sitemap_membership_limits"
    )]
    pub sitemap_membership_limits: Option<crate::SitemapLedgerLimits>,
    /// Opt-in local original source bytes, independent of XML work/member/wire limits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sitemap_source_archive: Option<crate::SitemapSourceArchiveConfig>,
    /// Optional static native PageDocument URL limits; None preserves legacy bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crawl_admission_limits: Option<crate::CrawlAdmissionLimits>,
    /// Durable distinct native target quotas. Absence preserves all legacy bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crawl_quotas: Option<crate::CrawlQuotaConfig>,
    pub crawl_links: bool,
    pub include_subdomains: bool,
    /// Declared reporting classification only; never grants fetch, traversal or credentials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdn_classification: Option<crate::CdnClassificationConfig>,
    /// Own literal-directory reporting only; never network or credential authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdn_directory_reporting: Option<crate::CdnDirectoryReportingConfig>,
    pub concurrency: usize,
    pub request_timeout_secs: u64,
    /// None means no product-imposed URL limit.
    pub max_pages: Option<usize>,
    pub max_depth: Option<usize>,
    /// Minimum interval between request starts for the same origin.
    pub delay_ms: u64,
    /// Reporting visibility only; never a request, frontier or outlink permission.
    #[serde(
        default,
        skip_serializing_if = "crate::DirectiveReportingConfig::is_inactive"
    )]
    pub directive_reporting: Option<crate::DirectiveReportingConfig>,
    pub obey_robots: bool,
    pub check_external: bool,
    pub max_body_bytes: usize,
    pub max_redirects: usize,
    pub user_agent: String,
    /// Bot token used to evaluate page-level indexing directives; independent of HTTP UA.
    pub indexability_agent: String,
    pub custom_headers: BTreeMap<String, String>,
    pub renderer: Option<RendererConfig>,
    pub custom_extractions: Vec<CustomExtractionConfig>,
    pub audit_thresholds: AuditThresholds,
    pub pdf_analysis: PdfAnalysisConfig,
    /// Fragment checks require captured target DOMs and are an explicit audit opt-in.
    pub validate_fragments: bool,
    /// First matching configured selector wins; matches the link or its ancestors.
    pub link_position_rules: Vec<LinkPositionRule>,
    /// Full normalized URL regex filters for discovery; explicit seeds are exempt.
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crawl_policy: Option<crate::CrawlPolicyConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_policy_limits: Option<crate::LiveCrawlPolicyLimits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_capture: Option<crate::ContentCaptureConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_discovery: Option<crate::ResourceDiscoveryConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_discovery_limits: Option<crate::ResourceDiscoveryLimits>,
}

// Keep the new opt-in object strict in every Serde format; outer null remains absence.
fn deserialize_sitemap_membership_limits<'de, D>(
    deserializer: D,
) -> Result<Option<crate::SitemapLedgerLimits>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct OptionalLimits;
    impl<'de> serde::de::Visitor<'de> for OptionalLimits {
        type Value = Option<crate::SitemapLedgerLimits>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("an optional object containing sitemap membership limits")
        }
        fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_some<T: serde::Deserializer<'de>>(self, d: T) -> Result<Self::Value, T::Error> {
            struct ObjectLimits;
            impl<'de> serde::de::Visitor<'de> for ObjectLimits {
                type Value = crate::SitemapLedgerLimits;
                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("an object containing sitemap membership limits")
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    map: A,
                ) -> Result<Self::Value, A::Error> {
                    crate::SitemapLedgerLimits::deserialize(
                        serde::de::value::MapAccessDeserializer::new(map),
                    )
                }
            }
            d.deserialize_map(ObjectLimits).map(Some)
        }
    }
    deserializer.deserialize_option(OptionalLimits)
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            seed_urls: Vec::new(),
            input_imports: Vec::new(),
            sitemap_urls: Vec::new(),
            explicit_sitemap_inputs: None,
            discover_sitemaps: true,
            max_sitemap_documents: 1000,
            sitemap_source_processing: None,
            sitemap_membership_limits: None,
            sitemap_source_archive: None,
            crawl_admission_limits: None,
            crawl_quotas: None,
            crawl_links: true,
            include_subdomains: false,
            cdn_classification: None,
            cdn_directory_reporting: None,
            concurrency: 8,
            request_timeout_secs: 30,
            max_pages: None,
            max_depth: None,
            delay_ms: 100,
            tls: None,
            directive_reporting: None,
            obey_robots: true,
            check_external: false,
            max_body_bytes: 5 * 1024 * 1024,
            max_redirects: 10,
            user_agent: "OpenSEOSpider/0.1 (+local SEO audit)".into(),
            indexability_agent: "googlebot".into(),
            custom_headers: BTreeMap::new(),
            renderer: None,
            custom_extractions: Vec::new(),
            audit_thresholds: AuditThresholds::default(),
            pdf_analysis: PdfAnalysisConfig::default(),
            validate_fragments: false,
            link_position_rules: Vec::new(),
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            crawl_policy: None,
            live_policy_limits: None,
            content_capture: None,
            resource_discovery: None,
            resource_discovery_limits: None,
        }
    }
}

impl CrawlConfig {
    /// Validate before creating a store or sending website requests. Browser
    /// navigation/subrequests have no interception contract in this native slice.
    pub fn validate_crawl_quotas(&self) -> anyhow::Result<()> {
        if let Some(quotas) = &self.crawl_quotas {
            quotas.validate()?;
            anyhow::ensure!(
                self.renderer.is_none(),
                "crawl_quotas cannot be combined with a renderer"
            );
            if let Some(resource) = &self.resource_discovery {
                anyhow::ensure!(
                    resource
                        .contract
                        .as_ref()
                        .is_some_and(|contract| contract.engine == crate::RESOURCE_CONTRACT_ENGINE),
                    "crawl_quotas with resource discovery requires resource-contract-v2"
                );
                anyhow::ensure!(
                    !self.include_subdomains,
                    "crawl_quotas with resource discovery does not support include_subdomains"
                );
                anyhow::ensure!(
                    !self.check_external,
                    "crawl_quotas with resource discovery does not support check_external"
                );
                anyhow::ensure!(
                    resource.resource_scope.external_origins.is_empty(),
                    "crawl_quotas with resource discovery does not support external resource origins"
                );
            }
        }
        Ok(())
    }

    pub fn validate_crawl_admission_limits(&self) -> anyhow::Result<()> {
        if let Some(limits) = &self.crawl_admission_limits {
            limits.validate()?;
            anyhow::ensure!(
                self.renderer.is_none(),
                "crawl_admission_limits cannot be combined with a renderer; browser request limits are not implemented"
            );
        }
        Ok(())
    }

    /// Fresh catalog capacity only. Resume/inherited discovery keeps its durable catalog limits.
    pub fn sitemap_membership_ledger_limits(&self) -> anyhow::Result<crate::SitemapLedgerLimits> {
        let limits = self.sitemap_membership_limits.clone().unwrap_or_default();
        limits.validate()?;
        Ok(limits)
    }

    /// Effective XML/decode work bounds, validated without repairing supplied
    /// values. Retained entry/location/text limits keep their existing defaults.
    pub fn sitemap_source_parser_limits(&self) -> anyhow::Result<crate::SitemapParserLimits> {
        match &self.sitemap_source_processing {
            Some(profile) => profile
                .parser_limits()
                .map_err(|error| anyhow::anyhow!("{}: {}", error.code, error.message)),
            None => Ok(crate::SitemapParserLimits::default()),
        }
    }

    /// Source-only opt-in is independent of page max_body_bytes. Historical
    /// configurations keep the previous min(page body, default entity) behavior.
    pub fn sitemap_source_body_limit(&self) -> anyhow::Result<usize> {
        let parser = self.sitemap_source_parser_limits()?;
        match &self.sitemap_source_processing {
            Some(profile) => Ok(profile.max_post_http_entity_bytes),
            None => {
                anyhow::ensure!(self.max_body_bytes > 0, "Body byte budget must be positive");
                Ok(self.max_body_bytes.min(parser.max_entity_bytes))
            }
        }
    }

    /// Call only at a fresh-input boundary, before the first import merge.
    /// Repeated preparation must not replace an already declared original vector.
    pub fn capture_explicit_sitemap_inputs(&mut self) {
        if self.explicit_sitemap_inputs.is_none() {
            self.explicit_sitemap_inputs = Some(self.sitemap_urls.clone());
        }
    }

    /// Prepare input provenance and discovery URLs without inventing original
    /// explicit sitemap declarations for unknown saved configurations.
    /// Fresh callers must capture before this method's first import merge.
    pub fn prepare_input_lists(&mut self) -> anyhow::Result<()> {
        use crate::{ListCandidateKind, ListImportFormat, ListImportLimits, ListImportOptions};
        if self.input_imports.is_empty() && !self.seed_urls.is_empty() {
            self.input_imports.push(crate::import_url_list(
                self.seed_urls.join("\n").as_bytes(),
                &ListImportOptions {
                    format: ListImportFormat::Txt,
                    limits: ListImportLimits::default(),
                },
            )?);
        }
        anyhow::ensure!(
            self.input_imports.len() <= 32,
            "At most 32 input lists may be attached to a crawl"
        );
        let mut total_bytes = 0usize;
        let mut seeds: std::collections::HashSet<String> = self
            .seed_urls
            .iter()
            .map(|seed| {
                crate::normalize_url(seed, None)
                    .map(|url| url.to_string())
                    .unwrap_or_else(|| seed.clone())
            })
            .collect();
        let mut sitemaps: std::collections::HashSet<String> =
            self.sitemap_urls.iter().cloned().collect();
        for input in &self.input_imports {
            crate::validate_import_result(input)?;
            total_bytes = total_bytes.saturating_add(serde_json::to_vec(input)?.len());
            anyhow::ensure!(
                total_bytes <= 32 * 1024 * 1024,
                "Combined input provenance exceeds 32 MiB"
            );
            for row in &input.rows {
                let Some(url) = row.normalized_url.as_ref().filter(|_| row.error.is_none()) else {
                    continue;
                };
                match row.candidate_kind {
                    ListCandidateKind::PageUrl if seeds.insert(url.clone()) => {
                        self.seed_urls.push(url.clone())
                    }
                    ListCandidateKind::SitemapDocument if sitemaps.insert(url.clone()) => {
                        self.sitemap_urls.push(url.clone())
                    }
                    _ => {}
                }
            }
        }
        self.validate_input_provenance_budget()?;
        Ok(())
    }

    /// Final budget check for retained original inputs; never drops raw values.
    /// A constructor's provisional fresh capture is checked after store/parent
    /// binding, or before a memory-only run can issue its first request.
    pub fn validate_input_provenance_budget(&self) -> anyhow::Result<()> {
        let mut total_bytes = self
            .explicit_sitemap_inputs
            .as_ref()
            .map(serde_json::to_vec)
            .transpose()?
            .map_or(0, |bytes| bytes.len());
        anyhow::ensure!(
            total_bytes <= 32 * 1024 * 1024,
            "Combined input provenance exceeds 32 MiB"
        );
        for input in &self.input_imports {
            total_bytes = total_bytes.saturating_add(serde_json::to_vec(input)?.len());
            anyhow::ensure!(
                total_bytes <= 32 * 1024 * 1024,
                "Combined input provenance exceeds 32 MiB"
            );
        }
        Ok(())
    }
}

/// Bounds for the independent native PDF parser, separate from crawl URL limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PdfAnalysisConfig {
    pub enabled: bool,
    pub max_pages: usize,
    pub max_objects: usize,
    /// Per-stream and combined page-content decompression limit.
    pub max_decompressed_bytes: usize,
    pub max_text_bytes: usize,
    pub max_text_bytes_per_page: usize,
    pub max_annotations: usize,
}

impl Default for PdfAnalysisConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_pages: 1000,
            max_objects: 100_000,
            max_decompressed_bytes: 8 * 1024 * 1024,
            max_text_bytes: 4 * 1024 * 1024,
            max_text_bytes_per_page: 1024 * 1024,
            max_annotations: 10_000,
        }
    }
}

impl PdfAnalysisConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        for (name, value, ceiling) in [
            ("max_pages", self.max_pages, 100_000),
            ("max_objects", self.max_objects, 1_000_000),
            (
                "max_decompressed_bytes",
                self.max_decompressed_bytes,
                128 * 1024 * 1024,
            ),
            ("max_text_bytes", self.max_text_bytes, 64 * 1024 * 1024),
            (
                "max_text_bytes_per_page",
                self.max_text_bytes_per_page,
                64 * 1024 * 1024,
            ),
            ("max_annotations", self.max_annotations, 1_000_000),
        ] {
            if value == 0 || value > ceiling {
                anyhow::bail!("PDF {name} must be between 1 and {ceiling}");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceAnalysisStatus {
    Complete,
    Partial,
    #[default]
    Error,
    Encrypted,
    Disabled,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PdfAnnotationLink {
    pub page: u32,
    pub raw_uri: String,
    /// Only normalized HTTP(S) URI actions have a discovery URL.
    pub url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PdfPageText {
    pub page: u32,
    pub chars: Option<usize>,
    pub word_count: Option<usize>,
    pub preview: Option<String>,
    pub truncated: bool,
    pub error: Option<String>,
}

/// Text means decoded page text operators, not OCR, visual layout or full conformance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PdfRecord {
    pub engine: String,
    pub status: ResourceAnalysisStatus,
    pub version: Option<String>,
    pub page_count: Option<usize>,
    pub declared_page_count: Option<usize>,
    pub encrypted: Option<bool>,
    pub info_metadata: BTreeMap<String, String>,
    pub text_pages_processed: usize,
    pub text_chars: Option<usize>,
    pub word_count: Option<usize>,
    pub text_hash: Option<String>,
    pub text_preview: Option<String>,
    pub text_complete: bool,
    pub text_pages: Vec<PdfPageText>,
    pub annotation_links: Vec<PdfAnnotationLink>,
    pub annotations_complete: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub parse_error: Option<String>,
    pub limits_reached: Vec<String>,
}

/// Actual fetched image resource headers; unrelated to HTML img attributes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageResourceMetadata {
    pub engine: String,
    pub status: ResourceAnalysisStatus,
    pub format: Option<String>,
    /// Raster pixels, or explicit absolute SVG lengths converted to CSS pixels.
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub dimension_unit: Option<String>,
    pub encoded_bytes: usize,
    pub body_complete: bool,
    pub svg_view_box: Option<[f64; 4]>,
    pub warnings: Vec<String>,
    pub parse_error: Option<String>,
}

/// Original heuristic thresholds; text lengths use Unicode scalar values.
/// Pixel checks use an explicitly recorded local font profile.
/// None disables an individual check; these are not search engine limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditThresholds {
    pub min_title_chars: Option<usize>,
    pub max_title_chars: Option<usize>,
    pub min_description_chars: Option<usize>,
    pub max_description_chars: Option<usize>,
    pub max_h1_chars: Option<usize>,
    pub max_h2_chars: Option<usize>,
    pub max_alt_chars: Option<usize>,
    pub max_url_chars: Option<usize>,
    pub min_word_count: Option<usize>,
    pub max_internal_outlinks: Option<usize>,
    pub max_external_outlinks: Option<usize>,
    pub min_title_pixels: Option<usize>,
    pub max_title_pixels: Option<usize>,
    pub min_description_pixels: Option<usize>,
    pub max_description_pixels: Option<usize>,
}

impl Default for AuditThresholds {
    fn default() -> Self {
        Self {
            min_title_chars: Some(30),
            max_title_chars: Some(60),
            min_description_chars: Some(70),
            max_description_chars: Some(155),
            max_h1_chars: Some(70),
            max_h2_chars: Some(70),
            max_alt_chars: Some(100),
            max_url_chars: Some(115),
            min_word_count: Some(200),
            max_internal_outlinks: Some(1000),
            max_external_outlinks: Some(10),
            min_title_pixels: Some(200),
            max_title_pixels: Some(561),
            min_description_pixels: Some(400),
            max_description_pixels: Some(985),
        }
    }
}

impl AuditThresholds {
    pub fn validate(&self) -> anyhow::Result<()> {
        if [
            self.min_title_chars,
            self.max_title_chars,
            self.min_description_chars,
            self.max_description_chars,
            self.max_h1_chars,
            self.max_h2_chars,
            self.max_alt_chars,
            self.max_url_chars,
            self.min_word_count,
            self.max_internal_outlinks,
            self.max_external_outlinks,
            self.min_title_pixels,
            self.max_title_pixels,
            self.min_description_pixels,
            self.max_description_pixels,
        ]
        .into_iter()
        .flatten()
        .any(|v| v > 1_000_000)
        {
            anyhow::bail!("Audit thresholds must not exceed 1000000");
        }
        for (name, min, max) in [
            ("title", self.min_title_chars, self.max_title_chars),
            (
                "description",
                self.min_description_chars,
                self.max_description_chars,
            ),
            ("title pixels", self.min_title_pixels, self.max_title_pixels),
            (
                "description pixels",
                self.min_description_pixels,
                self.max_description_pixels,
            ),
        ] {
            if let (Some(min), Some(max)) = (min, max)
                && min > max
            {
                anyhow::bail!("Minimum {name} length exceeds maximum length");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RendererConfig {
    /// The browser worker is a separate process bound to the local machine.
    pub endpoint: String,
    pub wait_ms: u64,
    pub accessibility: bool,
    pub screenshot: bool,
    pub custom_javascript: Vec<CustomJavaScriptConfig>,
    pub collect_cookies: bool,
    pub max_response_bytes: usize,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:8790/render".into(),
            wait_ms: 500,
            accessibility: false,
            screenshot: false,
            custom_javascript: Vec::new(),
            collect_cookies: false,
            max_response_bytes: 32 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomJavaScriptConfig {
    pub name: String,
    pub script: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RobotsDirective {
    pub source: String,
    pub agent: Option<String>,
    pub raw: String,
    pub directives: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkRelationship {
    pub url: String,
    pub rel: Vec<String>,
    pub source: String,
    pub context_url: String,
    pub applies_to_document: bool,
    pub hreflang: Vec<String>,
    pub media: Option<String>,
    pub type_hint: Option<String>,
    pub title: Option<String>,
    pub parameters: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RawMetadata {
    pub titles: Vec<String>,
    pub meta_descriptions: Vec<String>,
    pub title_pixel_widths: Vec<Option<u32>>,
    pub description_pixel_widths: Vec<Option<u32>>,
    pub text_metrics_engine: Option<String>,
    pub h1: Vec<String>,
    pub h2: Vec<String>,
    pub canonical: Vec<String>,
    pub html_canonical: Vec<String>,
    pub header_canonical: Vec<String>,
    pub meta_robots: Vec<String>,
    pub robots_directives: Vec<RobotsDirective>,
    pub effective_robots: Vec<String>,
    pub indexability_agent: String,
    pub hreflang: Vec<HreflangRecord>,
    pub html_hreflang: Vec<HreflangRecord>,
    pub header_hreflang: Vec<HreflangRecord>,
    pub document_base_url: Option<String>,
    pub base_href: Option<String>,
    pub link_relationships: Vec<LinkRelationship>,
    pub pagination_next: Vec<String>,
    pub pagination_prev: Vec<String>,
    pub amphtml: Vec<String>,
    pub word_count: usize,
    pub content_hash: Option<String>,
    pub links: Vec<LinkRecord>,
    pub custom_extractions: Vec<CustomExtraction>,
    pub dom_metadata: Option<DomMetadata>,
}

impl From<&PageRecord> for RawMetadata {
    fn from(page: &PageRecord) -> Self {
        Self {
            titles: page.titles.clone(),
            meta_descriptions: page.meta_descriptions.clone(),
            title_pixel_widths: page.title_pixel_widths.clone(),
            description_pixel_widths: page.description_pixel_widths.clone(),
            text_metrics_engine: page.text_metrics_engine.clone(),
            h1: page.h1.clone(),
            h2: page.h2.clone(),
            canonical: page.canonical.clone(),
            html_canonical: page.html_canonical.clone(),
            header_canonical: page.header_canonical.clone(),
            meta_robots: page.meta_robots.clone(),
            robots_directives: page.robots_directives.clone(),
            effective_robots: page.effective_robots.clone(),
            indexability_agent: page.indexability_agent.clone(),
            hreflang: page.hreflang.clone(),
            html_hreflang: page.html_hreflang.clone(),
            header_hreflang: page.header_hreflang.clone(),
            document_base_url: page.document_base_url.clone(),
            base_href: page.base_href.clone(),
            link_relationships: page.link_relationships.clone(),
            pagination_next: page.pagination_next.clone(),
            pagination_prev: page.pagination_prev.clone(),
            amphtml: page.amphtml.clone(),
            word_count: page.word_count,
            content_hash: page.content_hash.clone(),
            links: page.links.clone(),
            custom_extractions: page.custom_extractions.clone(),
            dom_metadata: page.dom_metadata.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkRecord {
    pub url: String,
    pub text: String,
    pub rel: String,
    pub internal: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdn_classification: Option<crate::CdnClassificationObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdn_directory_reporting: Option<crate::CdnDirectoryReportingObservation>,
    pub nofollow: bool,
    pub kind: String,
    pub raw_href: Option<String>,
    /// Resolved HTTP(S) address including its original fragment; url is the fetch identity.
    pub resolved_href: Option<String>,
    pub fragment: Option<String>,
    pub dom_location: Option<DomLocation>,
    pub position: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkPositionRule {
    pub name: String,
    pub selector: String,
}

/// Location in the repaired HTML5 tree, not an original byte offset or rendered coordinate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DomLocation {
    pub index: usize,
    pub path: Option<String>,
    pub path_truncated: bool,
    pub semantic_ancestors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HeadingRecord {
    pub tag_level: u8,
    pub text: String,
    pub raw_text: String,
    /// Independent DOM-order text profile with inline img alt; None is unobserved/legacy.
    pub text_with_image_alt: Option<String>,
    pub image_alt: Vec<Option<String>>,
    pub aria_level: Option<String>,
    pub heading_offset: Option<String>,
    pub hidden_attribute: bool,
    pub dom_location: DomLocation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MetaKeywordRecord {
    pub content: Option<String>,
    pub normalized: String,
    pub chars: Option<usize>,
    pub dom_location: DomLocation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FragmentTarget {
    /// id or legacy_anchor_name; only a[name] supplies the latter.
    pub kind: String,
    pub value: String,
    pub dom_location: DomLocation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NavigationCandidate {
    pub tag: String,
    pub raw_href: Option<String>,
    pub resolved_href: Option<String>,
    pub potential_url: Option<String>,
    pub scheme: Option<String>,
    pub reason: String,
    pub onclick: Option<String>,
    pub router_link: Option<String>,
    pub text: String,
    pub internal: Option<bool>,
    pub dom_location: DomLocation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageCandidate {
    pub source: String,
    pub raw_url: String,
    pub url: Option<String>,
    pub descriptors: Vec<String>,
    pub width: Option<u64>,
    pub density: Option<f64>,
    pub height: Option<u64>,
    pub media: Option<String>,
    pub type_hint: Option<String>,
    pub sizes: Option<String>,
    /// A syntactically accepted candidate associated with this img; selection is unknown.
    pub eligible: bool,
    pub error: Option<String>,
    pub dom_location: DomLocation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ResponsiveImageRecord {
    pub src: Option<String>,
    pub srcset: Option<String>,
    pub sizes: Option<String>,
    pub alt: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub loading: Option<String>,
    pub candidates: Vec<ImageCandidate>,
    pub dom_location: DomLocation,
}

/// Some(empty) establishes an observed empty snapshot; None is legacy/unavailable.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DomMetadata {
    pub engine: String,
    pub source: String,
    /// Identifies the independent normalization applied to text_with_image_alt.
    pub heading_text_engine: Option<String>,
    /// Observation collection completeness; does not prove HTTP/render success or future JS state.
    pub complete: bool,
    pub limits_reached: Vec<String>,
    pub headings: Vec<HeadingRecord>,
    pub meta_keywords: Vec<MetaKeywordRecord>,
    pub fragment_targets: Vec<FragmentTarget>,
    pub navigation_candidates: Vec<NavigationCandidate>,
    pub responsive_images: Vec<ResponsiveImageRecord>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FragmentStatus {
    MatchedId,
    MatchedName,
    Top,
    Missing,
    #[default]
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FragmentObservation {
    pub source_url: String,
    pub source_document_url: String,
    pub target_url: String,
    pub resolved_href: String,
    pub fragment: String,
    pub decoded_fragment: Option<String>,
    pub source_dom_location: Option<DomLocation>,
    pub target_document_url: Option<String>,
    pub status: FragmentStatus,
    pub reason: String,
    pub matched_target: Option<FragmentTarget>,
    pub target_complete: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageRecord {
    pub url: String,
    pub alt: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub loading: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HreflangRecord {
    pub language: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RedirectHop {
    pub url: String,
    pub status: u16,
    pub target: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomExtraction {
    pub name: String,
    pub selector: String,
    pub attribute: Option<String>,
    pub values: Vec<String>,
    pub source: String,
    pub method: ExtractionMethod,
    pub output: ExtractionOutput,
    pub engine: String,
    pub result_type: String,
    /// Extractable values before the configured result bound; None means unknown.
    pub matched_count: Option<usize>,
    pub truncated: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomExtractionConfig {
    pub name: String,
    /// CSS selector, XPath 1.0 expression, or Rust regex pattern.
    pub selector: String,
    #[serde(default)]
    pub attribute: Option<String>,
    pub method: ExtractionMethod,
    /// Auto preserves historical text/attribute/scalar behavior.
    pub output: ExtractionOutput,
    /// Numeric regex capture group; None selects the entire match.
    pub regex_group: Option<usize>,
    /// Supported flags: i, m, s, x, U. Inline Rust regex flags also work.
    pub regex_flags: String,
    /// XPath expression prefix to namespace URI bindings. HTML names are unnamespaced.
    pub namespaces: BTreeMap<String, String>,
    /// Per-definition value bound; excess matches retain count/truncation evidence.
    pub max_results: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtractionMethod {
    #[default]
    Css,
    XPath,
    Regex,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionOutput {
    #[default]
    Auto,
    Text,
    InnerHtml,
    OuterHtml,
    FunctionValue,
}

impl Default for CustomExtractionConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            selector: String::new(),
            attribute: None,
            method: ExtractionMethod::Css,
            output: ExtractionOutput::Auto,
            regex_group: None,
            regex_flags: String::new(),
            namespaces: BTreeMap::new(),
            max_results: 10000,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PageRecord {
    pub url: String,
    pub final_url: String,
    pub depth: usize,
    pub discovered_from: Option<String>,
    pub internal: bool,
    pub fetched_at: String,
    /// None is unobserved/legacy; configured sources have explicit result/error observations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_captures: Option<Vec<crate::PageContentCapture>>,
    /// Native static dispatch evidence; absent on legacy/unconfigured pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crawl_admission: Option<crate::CrawlAdmissionObservation>,
    /// First actual network HTTP response; HSTS synthetic 307 stays in tls, never here.
    pub status: Option<u16>,
    pub final_status: Option<u16>,
    pub content_type: String,
    pub response_ms: u64,
    pub size_bytes: usize,
    pub truncated: bool,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub blocked_by_robots: bool,
    pub robots_warning: Option<String>,
    pub redirect_chain: Vec<RedirectHop>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls: Option<crate::PageTlsObservation>,
    pub headers: BTreeMap<String, String>,
    pub titles: Vec<String>,
    pub meta_descriptions: Vec<String>,
    pub title_pixel_widths: Vec<Option<u32>>,
    pub description_pixel_widths: Vec<Option<u32>>,
    pub text_metrics_engine: Option<String>,
    pub h1: Vec<String>,
    pub h2: Vec<String>,
    pub canonical: Vec<String>,
    pub html_canonical: Vec<String>,
    pub header_canonical: Vec<String>,
    pub meta_robots: Vec<String>,
    pub x_robots_tag: Vec<String>,
    pub robots_directives: Vec<RobotsDirective>,
    pub effective_robots: Vec<String>,
    pub indexability_agent: String,
    pub header_link_values: Vec<String>,
    pub hreflang: Vec<HreflangRecord>,
    pub html_hreflang: Vec<HreflangRecord>,
    pub header_hreflang: Vec<HreflangRecord>,
    pub document_base_url: Option<String>,
    pub base_href: Option<String>,
    pub link_relationships: Vec<LinkRelationship>,
    pub pagination_next: Vec<String>,
    pub pagination_prev: Vec<String>,
    pub amphtml: Vec<String>,
    pub language: Option<String>,
    pub viewport: Option<String>,
    pub images: Vec<ImageRecord>,
    pub links: Vec<LinkRecord>,
    pub word_count: usize,
    pub content_hash: Option<String>,
    pub structured_data: Vec<serde_json::Value>,
    pub invalid_structured_data: usize,
    /// Native crawling does not execute JavaScript. This is the count of script tags.
    pub script_count: usize,
    pub indexable: bool,
    pub indexability_reasons: Vec<String>,
    pub raw_metadata: Option<RawMetadata>,
    /// Requested URL classification, never inferred from a redirect or a CDN provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdn_classification: Option<crate::CdnClassificationObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdn_directory_reporting: Option<crate::CdnDirectoryReportingObservation>,
    /// Bounded native HTTP head-source facts, independent of repaired/rendered metadata.
    /// Legacy absence is unknown; no observation is reconstructed on report load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html_head_observation: Option<crate::HtmlHeadObservation>,
    pub rendered: bool,
    pub renderer_error: Option<String>,
    pub rendered_url: Option<String>,
    pub rendered_status: Option<u16>,
    pub rendered_headers: BTreeMap<String, String>,
    pub rendered_at: Option<String>,
    pub browser_version: Option<String>,
    pub rendered_viewport: Option<serde_json::Value>,
    pub render_wait_ms: Option<u64>,
    pub network_events_dropped: usize,
    pub console_errors: Vec<serde_json::Value>,
    pub network_requests: Vec<serde_json::Value>,
    pub accessibility: Option<serde_json::Value>,
    pub screenshot_base64: Option<String>,
    pub custom_extractions: Vec<CustomExtraction>,
    pub custom_javascript_results: Vec<serde_json::Value>,
    pub browser_cookies: Vec<serde_json::Value>,
    pub pdf: Option<PdfRecord>,
    pub image_metadata: Option<ImageResourceMetadata>,
    pub dom_metadata: Option<DomMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<crate::PagePolicyObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_source: Option<crate::ResourceSourceBatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_permission: Option<crate::ResourceTaskPermission>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_response: Option<crate::ResourceResponseFacts>,
}

impl PageRecord {
    pub fn document_url(&self) -> &str {
        if self.rendered && self.renderer_error.is_none() {
            self.rendered_url.as_deref().unwrap_or(&self.final_url)
        } else {
            &self.final_url
        }
    }
    pub fn is_html(&self) -> bool {
        self.pdf.is_none()
            && self.image_metadata.is_none()
            && (self.media_type().eq_ignore_ascii_case("text/html")
                || self
                    .media_type()
                    .eq_ignore_ascii_case("application/xhtml+xml"))
    }
    pub fn media_type(&self) -> &str {
        self.content_type.split(';').next().unwrap_or("").trim()
    }
    pub fn is_indexable_document(&self) -> bool {
        self.is_html()
            || self.pdf.is_some()
            || self.media_type().eq_ignore_ascii_case("application/pdf")
            || self.media_type().eq_ignore_ascii_case("text/plain")
    }
    pub fn title(&self) -> &str {
        self.titles.first().map(String::as_str).unwrap_or("")
    }
    pub fn description(&self) -> &str {
        self.meta_descriptions
            .first()
            .map(String::as_str)
            .unwrap_or("")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Notice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditIssue {
    pub code: String,
    pub severity: Severity,
    pub url: String,
    pub message: String,
    pub related_urls: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CrawlSummary {
    pub started_at: String,
    pub finished_at: String,
    pub elapsed_ms: u64,
    pub completed: usize,
    pub discovered: usize,
    pub pending: usize,
    pub html_pages: usize,
    pub indexable_pages: usize,
    pub errors: usize,
    pub warnings: usize,
    pub cancelled: bool,
    pub limited: bool,
    pub status_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlReport {
    pub config: CrawlConfig,
    pub pages: Vec<PageRecord>,
    pub issues: Vec<AuditIssue>,
    pub summary: CrawlSummary,
    pub warnings: Vec<String>,
    /// Absent on ordinary and legacy crawls. Selected snapshots have mixed-age
    /// observations and exclude selected historical rows until fresh commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<crate::SelectedRecrawlOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_ledger: Option<crate::PolicyLedgerSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_ledger: Option<crate::ResourceLedgerSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sitemap_catalog: Option<crate::SitemapCatalog>,
    /// Exact captured source/task/config context, retained for historical selected inheritance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sitemap_coverage_inventory: Option<crate::SitemapCoverageInventory>,
    /// Immutable measured captures and exact durable membership commit links.
    /// Absence remains unavailable, including legacy reports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sitemap_source_measurements: Option<crate::SitemapSourceStoreSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrawlEvent {
    ResourceProgress {
        ledger: Box<crate::ResourceLedgerSummary>,
    },
    PolicyProgress {
        ledger: Box<crate::PolicyLedgerSummary>,
    },
    Operation {
        operation: Box<crate::SelectedRecrawlOperation>,
    },
    RecrawlProgress {
        target: Box<crate::RecrawlTargetObservation>,
        admitted_this_run: usize,
        committed_this_run: usize,
        refreshed_count: usize,
        pending_count: usize,
    },
    Page {
        page: Box<PageRecord>,
    },
    Progress {
        completed: usize,
        queued: usize,
        in_flight: usize,
    },
    Warning {
        message: String,
    },
    Finished {
        summary: CrawlSummary,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageChange {
    pub url: String,
    pub kind: String,
    pub fields: Vec<String>,
}
