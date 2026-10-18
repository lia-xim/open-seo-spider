//! Shared declaration validation before HTTP clients, runtime caches or stores.
use crate::{
    CrawlConfig, normalize_url,
    urls::{UrlFilters, origin},
};
use anyhow::{Context, bail};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::sync::Arc;
use url::Url;

pub(crate) struct PreparedCrawlConfig {
    pub config: CrawlConfig,
    pub seeds: Vec<Url>,
    pub headers: HeaderMap,
    pub filters: UrlFilters,
    pub automatically_captured_sitemap_inputs: bool,
    pub policy: Option<Arc<crate::CompiledCrawlPolicy>>,
    pub policy_limits: crate::LiveCrawlPolicyLimits,
    pub capture: Option<Arc<crate::CompiledContentCaptureConfig>>,
    pub custom_search: Option<Arc<crate::CompiledCustomSearchConfig>>,
    pub cdn_classification: Option<Arc<crate::cdn_classification::CompiledCdnClassification>>,
    pub cdn_directory_reporting:
        Option<Arc<crate::cdn_directory_reporting::CompiledCdnDirectoryReporting>>,
}

pub(crate) fn prepare(
    mut config: CrawlConfig,
    fresh_input: bool,
    require_targets: bool,
) -> anyhow::Result<PreparedCrawlConfig> {
    let automatically_captured_sitemap_inputs =
        fresh_input && config.explicit_sitemap_inputs.is_none();
    // Snapshot before merge, but defer the provisional fresh budget until
    // with_store / with_selected_recrawl has identified the actual mode.
    let original_sitemap_inputs =
        automatically_captured_sitemap_inputs.then(|| config.sitemap_urls.clone());
    if let Some(tls) = &config.tls {
        tls.validate()?;
        if !tls.is_inactive() && config.renderer.is_some() {
            bail!(
                "Crawl-scoped trust/HSTS is not yet supported by the external renderer; disable rendering or TLS options"
            );
        }
    }
    if config
        .tls
        .as_ref()
        .is_some_and(crate::TlsConfig::is_inactive)
    {
        config.tls = None;
    }
    config.prepare_input_lists()?;
    config.validate_crawl_quotas()?;
    if let Some(profile) = &config.directive_reporting {
        profile.validate()?;
    }
    if automatically_captured_sitemap_inputs {
        config.explicit_sitemap_inputs = original_sitemap_inputs;
    }
    if let Some(resource) = &config.resource_discovery {
        resource.validate()?;
        config
            .resource_discovery_limits
            .clone()
            .unwrap_or_default()
            .validate()?;
        if config.renderer.is_some() {
            bail!(
                "Native resource discovery cannot be combined with browser rendering: browser resource enforcement is not implemented"
            );
        }
    } else if config.resource_discovery_limits.is_some() {
        bail!("Resource discovery limits require resource discovery configuration");
    }
    let cdn_classification = config
        .cdn_classification
        .as_ref()
        .map(crate::CdnClassificationConfig::compile)
        .transpose()?
        .map(Arc::new);
    let cdn_directory_reporting = config
        .cdn_directory_reporting
        .as_ref()
        .map(crate::CdnDirectoryReportingConfig::compile)
        .transpose()?
        .map(Arc::new);
    let capture = config
        .content_capture
        .as_ref()
        .map(crate::ContentCaptureConfig::compile)
        .transpose()?
        .map(Arc::new);
    let custom_search = config
        .custom_search
        .as_ref()
        .map(crate::CustomSearchConfig::compile)
        .transpose()?
        .map(Arc::new);
    let policy_limits = config.live_policy_limits.clone().unwrap_or_default();
    policy_limits.validate()?;
    if config.crawl_policy.is_none() && config.live_policy_limits.is_some() {
        bail!("Live policy limits require a configured crawl policy");
    }
    if config.crawl_policy.is_some() && config.renderer.is_some() {
        bail!(
            "Native crawl policy cannot be combined with browser rendering: browser request enforcement is not implemented"
        );
    }
    if config.crawl_policy.is_some()
        && (!config.include_patterns.is_empty() || !config.exclude_patterns.is_empty())
    {
        bail!("Use policy include/exclude rules rather than ambiguous legacy URL filters");
    }
    let policy = config
        .crawl_policy
        .as_ref()
        .map(crate::CompiledCrawlPolicy::new)
        .transpose()?
        .map(Arc::new);
    if let Some(policy) = &policy {
        config.crawl_policy = Some(policy.config().clone());
    }
    if require_targets && config.seed_urls.is_empty() && config.sitemap_urls.is_empty() {
        bail!("At least one seed URL or sitemap URL is required");
    }
    if config.concurrency == 0 || config.concurrency > 1024 {
        bail!("Concurrency must be between 1 and 1024");
    }
    if config.request_timeout_secs == 0 || config.max_body_bytes == 0 {
        bail!("Timeout and response-body limit must be positive");
    }
    // Validate the saved source-only work profile before any client/store I/O.
    // Page response limits and retained membership budgets remain independent.
    config.validate_crawl_admission_limits()?;
    config.sitemap_source_parser_limits()?;
    config.sitemap_source_body_limit()?;
    config.sitemap_membership_ledger_limits()?;
    if let Some(archive) = &config.sitemap_source_archive {
        archive.validate()?;
    }
    config.indexability_agent = config.indexability_agent.trim().to_ascii_lowercase();
    if config.indexability_agent.is_empty()
        || config.indexability_agent.len() > 128
        || !config
            .indexability_agent
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'*' | b'.'))
    {
        bail!("Indexability agent must be a bot token with 1 to 128 ASCII characters");
    }
    if let Some(renderer) = &config.renderer {
        crate::renderer::validate_renderer(renderer, config.request_timeout_secs)?;
    }
    crate::extract::validate_custom_extractions(&config.custom_extractions)?;
    config.audit_thresholds.validate()?;
    config.pdf_analysis.validate()?;
    crate::dom_facts::validate_position_rules(&config.link_position_rules)?;
    let filters = UrlFilters::new(&config.include_patterns, &config.exclude_patterns)?;
    if config.max_pages == Some(0) {
        config.max_pages = None;
    }
    if config.max_depth == Some(0) {
        config.max_depth = None;
    }
    let mut seeds = Vec::new();
    for seed in &config.seed_urls {
        if seed.chars().any(char::is_control) {
            bail!("Seed URLs must not contain raw control characters");
        }
        seeds.push(
            normalize_url(seed, None)
                .with_context(|| format!("Invalid HTTP(S) seed URL: {seed}"))?,
        );
    }
    config.seed_urls = seeds.iter().map(Url::to_string).collect();
    let sitemap_only_scope = seeds.is_empty();
    let mut sitemap_urls = Vec::new();
    for sitemap in &config.sitemap_urls {
        let url = normalize_url(sitemap, None)
            .with_context(|| format!("Invalid HTTP(S) sitemap URL: {sitemap}"))?;
        if sitemap_only_scope {
            seeds.push(Url::parse(&format!("{}/", origin(&url)))?);
        }
        sitemap_urls.push(url.to_string());
    }
    config.sitemap_urls = sitemap_urls;
    let mut headers = HeaderMap::new();
    for (name, value) in &config.custom_headers {
        headers
            .try_insert(
                HeaderName::from_bytes(name.as_bytes())
                    .with_context(|| format!("Invalid header name {name}"))?,
                HeaderValue::from_str(value)
                    .with_context(|| format!("Invalid header value for {name}"))?,
            )
            .context("Configured headers exceed native HTTP header-map capacity")?;
    }

    HeaderValue::from_str(&config.user_agent).context("Invalid User-Agent header")?;
    Ok(PreparedCrawlConfig {
        config,
        seeds,
        headers,
        filters,
        automatically_captured_sitemap_inputs,
        policy,
        policy_limits,
        capture,
        custom_search,
        cdn_classification,
        cdn_directory_reporting,
    })
}
