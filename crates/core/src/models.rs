use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CrawlConfig {
    pub seed_urls: Vec<String>,
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
    pub custom_headers: BTreeMap<String, String>,
    pub renderer: Option<RendererConfig>,
    pub custom_extractions: Vec<CustomExtractionConfig>,
    /// Full normalized URL regex filters for discovery; explicit seeds are exempt.
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            seed_urls: Vec::new(),
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
            custom_headers: BTreeMap::new(),
            renderer: None,
            custom_extractions: Vec::new(),
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
        }
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
    pub max_response_bytes: usize,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:8790/render".into(),
            wait_ms: 500,
            accessibility: false,
            screenshot: false,
            max_response_bytes: 32 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RawMetadata {
    pub titles: Vec<String>,
    pub meta_descriptions: Vec<String>,
    pub h1: Vec<String>,
    pub h2: Vec<String>,
    pub canonical: Vec<String>,
    pub meta_robots: Vec<String>,
    pub hreflang: Vec<HreflangRecord>,
    pub word_count: usize,
    pub content_hash: Option<String>,
    pub links: Vec<LinkRecord>,
    pub custom_extractions: Vec<CustomExtraction>,
}

impl From<&PageRecord> for RawMetadata {
    fn from(page: &PageRecord) -> Self {
        Self {
            titles: page.titles.clone(),
            meta_descriptions: page.meta_descriptions.clone(),
            h1: page.h1.clone(),
            h2: page.h2.clone(),
            canonical: page.canonical.clone(),
            meta_robots: page.meta_robots.clone(),
            hreflang: page.hreflang.clone(),
            word_count: page.word_count,
            content_hash: page.content_hash.clone(),
            links: page.links.clone(),
            custom_extractions: page.custom_extractions.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LinkRecord {
    pub url: String,
    pub text: String,
    pub rel: String,
    pub internal: bool,
    pub nofollow: bool,
    pub kind: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomExtractionConfig {
    pub name: String,
    pub selector: String,
    #[serde(default)]
    pub attribute: Option<String>,
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
    pub h1: Vec<String>,
    pub h2: Vec<String>,
    pub canonical: Vec<String>,
    pub meta_robots: Vec<String>,
    pub x_robots_tag: Vec<String>,
    pub hreflang: Vec<HreflangRecord>,
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
}

impl PageRecord {
    pub fn is_html(&self) -> bool {
        self.content_type.contains("text/html")
            || self.content_type.contains("application/xhtml+xml")
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrawlEvent {
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
