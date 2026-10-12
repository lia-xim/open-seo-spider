use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CrawlConfig {
    pub seed_urls: Vec<String>,
    /// Declared original input occurrences; network deduplication is separate.
    pub input_imports: Vec<crate::ListImportResult>,
    pub sitemap_urls: Vec<String>,
    pub discover_sitemaps: bool,
    pub max_sitemap_documents: usize,
    pub crawl_links: bool,
    pub include_subdomains: bool,
    pub concurrency: usize,
    pub request_timeout_secs: u64,
    /// None means no product-imposed URL limit.
    pub max_pages: Option<usize>,
    pub max_depth: Option<usize>,
    /// Minimum interval between request starts for the same origin.
    pub delay_ms: u64,
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
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            seed_urls: Vec::new(),
            input_imports: Vec::new(),
            sitemap_urls: Vec::new(),
            discover_sitemaps: true,
            max_sitemap_documents: 1000,
            crawl_links: true,
            include_subdomains: false,
            concurrency: 8,
            request_timeout_secs: 30,
            max_pages: None,
            max_depth: None,
            delay_ms: 100,
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
        }
    }
}

impl CrawlConfig {
    /// Prepare declared input provenance and discovery URLs without any network work.
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
    /// Status of the initially requested URL; final_status belongs to final_url.
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrawlEvent {
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
