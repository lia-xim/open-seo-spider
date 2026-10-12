use crate::urls::{origin, same_scope};
use crate::{CrawlConfig, PageRecord, RawMetadata, RendererConfig, normalize_url};
use anyhow::{Context, bail};
use reqwest::Client;
use serde::Deserialize;
use std::{collections::BTreeMap, time::Duration};
use url::Url;

#[derive(Deserialize)]
struct RenderedDocument {
    html: String,
    final_url: String,
    status: Option<u16>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    console_errors: Vec<serde_json::Value>,
    #[serde(default)]
    network_requests: Vec<serde_json::Value>,
    rendered_at: Option<String>,
    #[serde(default)]
    browser_version: Option<String>,
    #[serde(default)]
    viewport: Option<serde_json::Value>,
    #[serde(default)]
    render_wait_ms: Option<u64>,
    #[serde(default)]
    network_events_dropped: usize,
    accessibility: Option<serde_json::Value>,
    screenshot_base64: Option<String>,
    #[serde(default)]
    custom_javascript_results: Vec<serde_json::Value>,
    #[serde(default)]
    browser_cookies: Vec<serde_json::Value>,
}

pub(crate) fn validate_renderer(config: &RendererConfig, timeout_secs: u64) -> anyhow::Result<()> {
    let endpoint = normalize_url(&config.endpoint, None).context("Invalid renderer endpoint")?;
    if endpoint.scheme() != "http"
        || !matches!(
            endpoint.host_str(),
            Some("127.0.0.1" | "localhost" | "[::1]" | "::1")
        )
    {
        bail!("Renderer endpoint must be HTTP on the loopback interface");
    }
    if config.max_response_bytes == 0 {
        bail!("Renderer response limit must be positive");
    }
    if timeout_secs > 120 {
        bail!("Browser rendering requires a timeout of at most 120 seconds");
    }
    if config.wait_ms > 30_000 {
        bail!("Renderer wait must be at most 30000 ms");
    }
    if config.wait_ms >= timeout_secs.saturating_mul(1000) {
        bail!("Renderer wait must be less than the request timeout");
    }
    if config.custom_javascript.len() > 32 {
        bail!("At most 32 custom JavaScript definitions are supported");
    }
    let mut names = std::collections::HashSet::new();
    let mut total_script_bytes = 0usize;
    for definition in &config.custom_javascript {
        let name = definition.name.trim();
        if name.is_empty()
            || name.len() > 128
            || name.chars().any(char::is_control)
            || !names.insert(name)
        {
            bail!(
                "Custom JavaScript names must be unique, nonempty, contain no controls and fit within 128 UTF-8 bytes"
            );
        }
        if definition.script.trim().is_empty() || definition.script.len() > 65536 {
            bail!("Custom JavaScript scripts must contain 1 to 65536 UTF-8 bytes");
        }
        total_script_bytes += definition.script.len();
    }
    if total_script_bytes > 262144 {
        bail!("Combined custom JavaScript exceeds 262144 UTF-8 bytes");
    }
    Ok(())
}

pub(crate) async fn render_page(page: &mut PageRecord, config: &CrawlConfig, seeds: &[Url]) {
    let Some(renderer) = &config.renderer else {
        return;
    };
    page.raw_metadata = Some(RawMetadata::from(&*page));
    match request_render(page, config, renderer, seeds).await {
        Ok(document) => {
            let native_final_url = page.final_url.clone();
            page.final_url = document.final_url.clone();
            if page.final_url != native_final_url {
                // A JavaScript navigation establishes a different document;
                // its native predecessor's headers cannot govern that DOM.
                page.header_link_values =
                    document.headers.get("link").cloned().into_iter().collect();
                page.x_robots_tag = document
                    .headers
                    .get("x-robots-tag")
                    .cloned()
                    .into_iter()
                    .collect();
                crate::metadata::extract_http_metadata(page);
                for relationship in &mut page.link_relationships {
                    relationship.source = "rendered_http_header".into();
                }
                for rule in &mut page.robots_directives {
                    rule.source = "rendered_http_header".into();
                }
            }
            page.titles.clear();
            page.meta_descriptions.clear();
            page.h1.clear();
            page.h2.clear();
            page.canonical.clear();
            page.meta_robots.clear();
            page.hreflang.clear();
            page.images.clear();
            page.links.clear();
            page.structured_data.clear();
            page.invalid_structured_data = 0;
            page.word_count = 0;
            page.content_hash = None;
            page.script_count = 0;
            page.language = None;
            page.viewport = None;
            crate::extract::extract_html_in_source(
                page,
                &document.html,
                seeds,
                config.include_subdomains,
                &config.link_position_rules,
                "rendered_html",
            );
            crate::extract::apply_custom_extractions(
                page,
                &document.html,
                &config.custom_extractions,
                "rendered_html",
            );
            page.final_url = native_final_url;
            page.rendered = true;
            page.rendered_url = Some(document.final_url);
            page.rendered_status = document.status;
            page.rendered_headers = document.headers;
            page.rendered_at = document.rendered_at;
            page.browser_version = document.browser_version;
            page.rendered_viewport = document.viewport;
            page.render_wait_ms = document.render_wait_ms;
            page.network_events_dropped = document.network_events_dropped;
            if page.network_events_dropped > 0 {
                page.warnings.push(format!(
                    "Browser network capture is incomplete: {} event(s) exceeded the worker resource limit",
                    page.network_events_dropped
                ));
            }
            page.console_errors = document.console_errors;
            page.network_requests = document.network_requests;
            page.accessibility = document.accessibility;
            page.screenshot_base64 = document.screenshot_base64;
            page.custom_javascript_results = document.custom_javascript_results;
            page.browser_cookies = document.browser_cookies;
        }
        Err(error) => {
            let message = format!("Browser rendering failed: {error:#}");
            page.renderer_error = Some(message.clone());
            page.warnings.push(message);
        }
    }
}

async fn request_render(
    page: &PageRecord,
    config: &CrawlConfig,
    renderer: &RendererConfig,
    seeds: &[Url],
) -> anyhow::Result<RenderedDocument> {
    let url = normalize_url(&page.final_url, None).context("Invalid rendering target")?;
    let headers = if seeds.iter().any(|seed| origin(seed) == origin(&url)) {
        config.custom_headers.clone()
    } else {
        BTreeMap::new()
    };
    let body = serde_json::to_vec(
        &serde_json::json!({"url":page.final_url,"user_agent":config.user_agent,"timeout_ms":config.request_timeout_secs.saturating_mul(1000),"wait_ms":renderer.wait_ms,"custom_headers":headers,"accessibility":renderer.accessibility,"screenshot":renderer.screenshot,"custom_javascript":renderer.custom_javascript,"collect_cookies":renderer.collect_cookies}),
    )?;
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(
            Duration::from_secs(config.request_timeout_secs)
                .saturating_add(Duration::from_millis(renderer.wait_ms))
                .saturating_add(Duration::from_secs(30)),
        )
        .build()?;
    let mut response = client
        .post(&renderer.endpoint)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .context("Local renderer is unreachable or timed out")?;
    let status = response.status();
    let mut data = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if data.len().saturating_add(chunk.len()) > renderer.max_response_bytes {
            bail!("Renderer response exceeds configured resource limit");
        }
        data.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        let error: serde_json::Value =
            serde_json::from_slice(&data).unwrap_or(serde_json::Value::Null);
        let message = error
            .pointer("/error/message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("No renderer error detail");
        bail!("Renderer returned HTTP {status}: {message}");
    }
    let document: RenderedDocument =
        serde_json::from_slice(&data).context("Invalid renderer response contract")?;
    let final_url = normalize_url(&document.final_url, None)
        .context("Renderer returned an invalid final URL")?;
    if !same_scope(&final_url, seeds, config.include_subdomains) && !config.check_external {
        bail!("Rendered navigation ended outside the configured crawl scope");
    }
    if document.status != Some(200) {
        bail!(
            "Rendered navigation did not establish HTTP200 (reported {:?})",
            document.status
        );
    }
    Ok(document)
}
