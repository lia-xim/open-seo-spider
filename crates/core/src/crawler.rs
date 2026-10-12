use crate::{
    CrawlConfig, CrawlEvent, CrawlReport, CrawlStore, CrawlSummary, PageRecord, RedirectHop,
    Severity, audit_pages_with_thresholds, normalize_url, parse_sitemap,
};
use crate::{
    audit::set_indexability,
    robots::{RateLimiter, RobotsCache},
    store::QueueEntry,
    urls::{UrlFilters, origin, same_scope},
};
use anyhow::{Context, bail};
use chrono::Utc;
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, mpsc::UnboundedSender};
use url::Url;

pub struct Crawler {
    config: CrawlConfig,
    seeds: Vec<Url>,
    client: Client,
    headers: HeaderMap,
    filters: UrlFilters,
    robots: Arc<RobotsCache>,
    rate: Arc<RateLimiter>,
    store: Option<Arc<CrawlStore>>,
    pdf_slots: Arc<Semaphore>,
}

impl Crawler {
    pub fn new(mut config: CrawlConfig) -> anyhow::Result<Self> {
        if config.seed_urls.is_empty() && config.sitemap_urls.is_empty() {
            bail!("At least one seed URL or sitemap URL is required");
        }
        if config.concurrency == 0 || config.concurrency > 1024 {
            bail!("Concurrency must be between 1 and 1024");
        }
        if config.request_timeout_secs == 0 || config.max_body_bytes == 0 {
            bail!("Timeout and response-body limit must be positive");
        }
        config.indexability_agent = config.indexability_agent.trim().to_ascii_lowercase();
        if config.indexability_agent.is_empty()
            || config.indexability_agent.len() > 128
            || !config.indexability_agent.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'*' | b'.')
            })
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
            headers.insert(
                HeaderName::from_bytes(name.as_bytes())
                    .with_context(|| format!("Invalid header name {name}"))?,
                HeaderValue::from_str(value)
                    .with_context(|| format!("Invalid header value for {name}"))?,
            );
        }
        let client = Client::builder()
            .user_agent(&config.user_agent)
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let robots_client = Client::builder()
            .user_agent(&config.user_agent)
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()?;
        let robots = Arc::new(RobotsCache::new(robots_client));
        let rate = Arc::new(RateLimiter::new(config.delay_ms));
        Ok(Self {
            config,
            seeds,
            client,
            headers,
            filters,
            robots,
            rate,
            store: None,
            pdf_slots: Arc::new(Semaphore::new(2)),
        })
    }

    pub fn with_store(mut self, path: impl AsRef<Path>) -> anyhow::Result<Self> {
        self.store = Some(Arc::new(CrawlStore::open(path)?));
        Ok(self)
    }

    pub async fn run(
        &self,
        events: Option<UnboundedSender<CrawlEvent>>,
        cancel: Arc<AtomicBool>,
    ) -> anyhow::Result<CrawlReport> {
        let started_at = Utc::now().to_rfc3339();
        let started = Instant::now();
        let resumed = match &self.store {
            Some(store) => store.prepare(&self.config)?,
            None => false,
        };
        let mut pages = match &self.store {
            Some(store) => store.pages()?,
            None => Vec::new(),
        };
        let mut queue: VecDeque<QueueEntry> = match &self.store {
            Some(store) => store.pending()?.into(),
            None => VecDeque::new(),
        };
        let mut seen: HashSet<String> = pages
            .iter()
            .map(|page| page.url.clone())
            .chain(queue.iter().map(|entry| entry.url.clone()))
            .collect();
        let mut warnings = Vec::new();
        if resumed {
            for page in &pages {
                emit(
                    &events,
                    CrawlEvent::Page {
                        page: Box::new(page.clone()),
                    },
                );
            }
        }
        // Always ensure seeds exist, including after an interruption immediately
        // after the configuration was first saved.
        for seed in &self.config.seed_urls {
            let entry = QueueEntry {
                url: seed.clone(),
                depth: 0,
                discovered_from: None,
            };
            if seen.insert(entry.url.clone()) {
                if let Some(store) = &self.store {
                    store.enqueue(&entry)?;
                }
                queue.push_back(entry);
            }
        }
        // Sitemap discovery is skipped in list mode so an explicit URL list stays exact.
        let discovery_complete = match &self.store {
            Some(store) => store.discovery_complete()?,
            None => false,
        };
        if !discovery_complete && (self.config.crawl_links || !self.config.sitemap_urls.is_empty())
        {
            let discovery = async {
                let sitemap_urls = self.sitemap_seeds(&cancel).await;
                self.ingest_sitemaps(sitemap_urls, &cancel).await
            };
            let (entries, sitemap_warnings) = tokio::select! {
                result = discovery => result,
                _ = wait_for_cancel(&cancel) => (Vec::new(),Vec::new()),
            };
            for warning in sitemap_warnings {
                emit(
                    &events,
                    CrawlEvent::Warning {
                        message: warning.clone(),
                    },
                );
                warnings.push(warning);
            }
            let entries: Vec<_> = entries
                .into_iter()
                .filter(|entry| self.filters.allows(&entry.url) && seen.insert(entry.url.clone()))
                .collect();
            if let Some(store) = &self.store {
                store.save_discovery(&entries, !cancel.load(Ordering::Relaxed))?;
            }
            for entry in entries {
                queue.push_back(entry);
            }
        }
        let context = Arc::new(FetchContext {
            config: self.config.clone(),
            seeds: self.seeds.clone(),
            client: self.client.clone(),
            headers: self.headers.clone(),
            robots: self.robots.clone(),
            rate: self.rate.clone(),
            pdf_slots: self.pdf_slots.clone(),
        });
        let mut tasks: FuturesUnordered<BoxFuture<'static, PageRecord>> = FuturesUnordered::new();
        let mut limited = false;
        loop {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            while tasks.len() < self.config.concurrency && !queue.is_empty() {
                if self
                    .config
                    .max_pages
                    .is_some_and(|limit| pages.len() + tasks.len() >= limit)
                {
                    limited = true;
                    break;
                }
                let entry = queue.pop_front().expect("nonempty queue");
                let fetch = context.clone();
                tasks.push(async move { fetch.document(entry).await.0 }.boxed());
            }
            emit(
                &events,
                CrawlEvent::Progress {
                    completed: pages.len(),
                    queued: queue.len(),
                    in_flight: tasks.len(),
                },
            );
            if tasks.is_empty() {
                break;
            }
            let page = tokio::select! {
                page = tasks.next() => page.expect("nonempty tasks"),
                _ = tokio::time::sleep(Duration::from_millis(100)) => continue,
            };
            let mut discoveries = Vec::new();
            if self.config.crawl_links
                && page.internal
                && !page.blocked_by_robots
                && self.config.max_depth.is_none_or(|limit| page.depth < limit)
            {
                for link in &page.links {
                    // Nofollow describes search-engine semantics; an audit may still inspect it.
                    if (link.internal || self.config.check_external)
                        && self.filters.allows(&link.url)
                        && seen.insert(link.url.clone())
                    {
                        discoveries.push(QueueEntry {
                            url: link.url.clone(),
                            depth: page.depth + 1,
                            discovered_from: Some(page.url.clone()),
                        });
                    }
                }
            }
            if let Some(store) = &self.store {
                store.complete(&page, &discoveries)?;
            }
            queue.extend(discoveries);
            emit(
                &events,
                CrawlEvent::Page {
                    page: Box::new(page.clone()),
                },
            );
            pages.push(page);
        }
        let pending = queue.len() + tasks.len();
        drop(tasks); // In-flight requests remain pending in SQLite and can be resumed.
        let mut issues = audit_pages_with_thresholds(&pages, &self.config.audit_thresholds);
        issues.extend(crate::audit_dom_metadata(&pages));
        let mut status_counts = BTreeMap::new();
        for page in &pages {
            let key = if page.blocked_by_robots {
                "robots".into()
            } else {
                page.status
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "failed".into())
            };
            *status_counts.entry(key).or_insert(0) += 1;
        }
        let summary = CrawlSummary {
            started_at,
            finished_at: Utc::now().to_rfc3339(),
            elapsed_ms: started.elapsed().as_millis() as u64,
            completed: pages.len(),
            discovered: seen.len(),
            pending,
            html_pages: pages.iter().filter(|p| p.is_html()).count(),
            indexable_pages: pages.iter().filter(|p| p.internal && p.indexable).count(),
            errors: issues
                .iter()
                .filter(|i| i.severity == Severity::Error)
                .count(),
            warnings: issues
                .iter()
                .filter(|i| i.severity == Severity::Warning)
                .count(),
            cancelled: cancel.load(Ordering::Relaxed),
            limited: limited && pending > 0,
            status_counts,
        };
        let report = CrawlReport {
            config: self.config.clone(),
            pages,
            issues,
            summary: summary.clone(),
            warnings,
        };
        if let Some(store) = &self.store {
            store.save_report(&report)?;
        }
        emit(&events, CrawlEvent::Finished { summary });
        Ok(report)
    }

    async fn sitemap_seeds(&self, cancel: &AtomicBool) -> Vec<String> {
        let mut sitemaps = self.config.sitemap_urls.clone();
        if self.config.crawl_links && self.config.discover_sitemaps && self.config.obey_robots {
            let mut origins = HashSet::new();
            for seed in &self.seeds {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                if origins.insert(origin(seed)) {
                    sitemaps.extend(self.robots.get(seed).await.sitemaps.clone());
                }
            }
        }
        sitemaps
    }

    async fn ingest_sitemaps(
        &self,
        seeds: Vec<String>,
        cancel: &AtomicBool,
    ) -> (Vec<QueueEntry>, Vec<String>) {
        let mut queue: VecDeque<String> = seeds.into();
        let mut seen = HashSet::new();
        let mut entries = Vec::new();
        let mut warnings = Vec::new();
        let context = FetchContext {
            config: self.config.clone(),
            seeds: self.seeds.clone(),
            client: self.client.clone(),
            headers: self.headers.clone(),
            robots: self.robots.clone(),
            rate: self.rate.clone(),
            pdf_slots: self.pdf_slots.clone(),
        };
        while let Some(url) = queue.pop_front() {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            if !seen.insert(url.clone()) {
                continue;
            }
            if seen.len() > self.config.max_sitemap_documents {
                warnings.push(format!(
                    "Sitemap document resource limit ({}) reached",
                    self.config.max_sitemap_documents
                ));
                break;
            }
            let Some(parsed) = normalize_url(&url, None) else {
                warnings.push(format!("Invalid sitemap URL {url}"));
                continue;
            };
            if !same_scope(&parsed, &self.seeds, self.config.include_subdomains) {
                warnings.push(format!("Sitemap outside configured scope skipped: {url}"));
                continue;
            }
            let (page, body) = context
                .document(QueueEntry {
                    url: url.clone(),
                    depth: 0,
                    discovered_from: None,
                })
                .await;
            if page.error.is_some()
                || page.final_status != Some(200)
                || page.truncated
                || body.is_none()
            {
                warnings.push(format!(
                    "Sitemap could not be read completely: {url}: {}",
                    page.error
                        .unwrap_or_else(|| format!("status {:?}", page.final_status))
                ));
                continue;
            }
            match parse_sitemap(&body.unwrap_or_default()) {
                Ok(sitemap) => {
                    for loc in sitemap.locations {
                        let Some(parsed) = normalize_url(&loc, None) else {
                            continue;
                        };
                        if !same_scope(&parsed, &self.seeds, self.config.include_subdomains) {
                            continue;
                        }
                        if sitemap.is_index {
                            queue.push_back(parsed.to_string());
                        } else {
                            entries.push(QueueEntry {
                                url: parsed.to_string(),
                                depth: 0,
                                discovered_from: Some(url.clone()),
                            });
                        }
                    }
                }
                Err(error) => warnings.push(format!("Invalid sitemap XML at {url}: {error}")),
            }
        }
        (entries, warnings)
    }
}

fn emit(events: &Option<UnboundedSender<CrawlEvent>>, event: CrawlEvent) {
    if let Some(events) = events {
        let _ = events.send(event);
    }
}

async fn wait_for_cancel(cancel: &AtomicBool) {
    while !cancel.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

struct FetchContext {
    config: CrawlConfig,
    seeds: Vec<Url>,
    client: Client,
    headers: HeaderMap,
    robots: Arc<RobotsCache>,
    rate: Arc<RateLimiter>,
    pdf_slots: Arc<Semaphore>,
}

impl FetchContext {
    async fn document(&self, entry: QueueEntry) -> (PageRecord, Option<String>) {
        let mut page = PageRecord {
            url: entry.url.clone(),
            final_url: entry.url.clone(),
            depth: entry.depth,
            discovered_from: entry.discovered_from,
            fetched_at: Utc::now().to_rfc3339(),
            indexability_agent: self.config.indexability_agent.clone(),
            ..Default::default()
        };
        let Some(mut current) = normalize_url(&entry.url, None) else {
            page.error = Some("Invalid HTTP(S) URL".into());
            set_indexability(&mut page);
            return (page, None);
        };
        page.internal = same_scope(&current, &self.seeds, self.config.include_subdomains);
        let mut visited = HashSet::new();
        visited.insert(current.to_string());
        let mut body = Vec::new();
        loop {
            if self.config.obey_robots {
                let policy = self.robots.get(&current).await;
                if !policy.allowed(&self.config.user_agent, current.as_str()) {
                    page.blocked_by_robots = true;
                    page.robots_warning = policy.warning.clone();
                    break;
                }
            }
            self.rate.wait(&current).await;
            let started = Instant::now();
            let request = self.client.get(current.clone());
            // Credential-bearing headers belong to explicitly configured origins,
            // even when external-link checking or subdomain discovery is enabled.
            let request = if self
                .seeds
                .iter()
                .any(|seed| origin(seed) == origin(&current))
            {
                request.headers(self.headers.clone())
            } else {
                request
            };
            let response = request.send().await;
            let mut response = match response {
                Ok(response) => response,
                Err(error) => {
                    page.response_ms += started.elapsed().as_millis() as u64;
                    page.error = Some(error.to_string());
                    break;
                }
            };
            let status = response.status().as_u16();
            page.status.get_or_insert(status);
            page.final_status = Some(status);
            page.final_url = current.to_string();
            page.content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("")
                .to_owned();
            page.headers.clear();
            for (name, value) in response.headers() {
                if let Ok(value) = value.to_str() {
                    let value = if name == reqwest::header::SET_COOKIE {
                        crate::metadata::redact_set_cookie(value)
                    } else {
                        value.to_owned()
                    };
                    page.headers
                        .entry(name.to_string())
                        .and_modify(|existing| {
                            existing.push_str(", ");
                            existing.push_str(&value);
                        })
                        .or_insert(value);
                }
            }
            page.header_link_values = response
                .headers()
                .get_all("link")
                .iter()
                .filter_map(|value| value.to_str().ok().map(str::to_owned))
                .collect();
            page.x_robots_tag = response
                .headers()
                .get_all("x-robots-tag")
                .iter()
                .filter_map(|v| v.to_str().ok().map(str::to_owned))
                .collect();
            if matches!(status, 301 | 302 | 303 | 307 | 308) {
                page.response_ms += started.elapsed().as_millis() as u64;
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok());
                let Some(target) =
                    location.and_then(|location| normalize_url(location, Some(&current)))
                else {
                    page.error = Some("Redirect has no valid HTTP(S) Location".into());
                    break;
                };
                page.redirect_chain.push(RedirectHop {
                    url: current.to_string(),
                    status,
                    target: target.to_string(),
                });
                page.final_url = target.to_string();
                page.final_status = None;
                if !visited.insert(target.to_string()) {
                    page.error = Some("Redirect loop detected".into());
                    page.final_status = None;
                    break;
                }
                if page.redirect_chain.len() > self.config.max_redirects {
                    page.error = Some(format!(
                        "Redirect limit ({}) exceeded",
                        self.config.max_redirects
                    ));
                    page.final_status = None;
                    break;
                }
                if !same_scope(&target, &self.seeds, self.config.include_subdomains)
                    && !self.config.check_external
                {
                    page.warnings.push(
                        "Redirect target outside crawl scope was recorded without fetching it"
                            .into(),
                    );
                    page.final_status = None;
                    break;
                }
                current = target;
                continue;
            }
            crate::metadata::extract_http_metadata(&mut page);
            loop {
                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        let remaining = self.config.max_body_bytes - body.len();
                        body.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                        if chunk.len() > remaining {
                            page.truncated = true;
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        page.error = Some(format!("Response body read failed: {error}"));
                        break;
                    }
                }
            }
            page.response_ms += started.elapsed().as_millis() as u64;
            break;
        }
        page.size_bytes = body.len();
        let pdf = crate::pdf::is_pdf(&body, &page.content_type);
        if pdf {
            let record = if page.error.is_some() || page.truncated || page.final_status != Some(200)
            {
                crate::pdf::incomplete(
                    "PDF body was not captured as a complete successful HTTP 200 response",
                )
            } else {
                let config = self.config.pdf_analysis.clone();
                let url = current.clone();
                let pdf_body = std::mem::take(&mut body);
                let permit = self
                    .pdf_slots
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("PDF semaphore is never closed");
                match tokio::task::spawn_blocking(move || {
                    let _permit = permit;
                    crate::pdf::analyze(&pdf_body, &url, &config)
                })
                .await
                {
                    Ok(record) => record,
                    Err(error) => {
                        crate::pdf::incomplete(&format!("Native PDF analysis task failed: {error}"))
                    }
                }
            };
            for link in &record.annotation_links {
                if let Some(url) = link.url.as_deref().and_then(|url| normalize_url(url, None)) {
                    page.links.push(crate::LinkRecord {
                        url: url.to_string(),
                        text: format!("PDF URI annotation (page {})", link.page),
                        internal: same_scope(&url, &self.seeds, self.config.include_subdomains),
                        kind: "pdf_annotation".into(),
                        ..Default::default()
                    });
                }
            }
            if record.status != crate::ResourceAnalysisStatus::Complete
                && record.status != crate::ResourceAnalysisStatus::Disabled
            {
                page.warnings.push(format!("PDF analysis is {:?}; consult PDF errors and limits before drawing absence conclusions", record.status));
            }
            page.pdf = Some(record);
        } else {
            page.image_metadata = crate::image_metadata::analyze(
                &body,
                &page.content_type,
                page.error.is_none() && !page.truncated && page.final_status == Some(200),
            );
        }
        let media_type = page.media_type().to_ascii_lowercase();
        let textual = page.is_html()
            || media_type.starts_with("text/")
            || matches!(media_type.as_str(), "application/xml" | "application/json")
            || media_type.ends_with("+xml")
            || media_type.ends_with("+json")
            || media_type.is_empty();
        let text = if !pdf && page.image_metadata.is_none() && textual && !body.is_empty() {
            let charset = page
                .content_type
                .split(';')
                .skip(1)
                .find_map(|part| {
                    part.split_once('=')
                        .filter(|(name, _)| name.trim().eq_ignore_ascii_case("charset"))
                        .map(|(_, value)| value.trim())
                })
                .map(|name| name.trim_matches(['\'', '"']));
            let encoding = charset
                .and_then(|label| encoding_rs::Encoding::for_label(label.as_bytes()))
                .unwrap_or(encoding_rs::UTF_8);
            let (decoded, _, errors) = encoding.decode(&body);
            if errors {
                page.warnings.push(
                    "Response text contains invalid bytes for the declared character encoding"
                        .into(),
                );
            }
            Some(decoded.into_owned())
        } else {
            None
        };
        if page.is_html()
            && let Some(text) = &text
        {
            crate::extract::extract_html_in_source(
                &mut page,
                text,
                &self.seeds,
                self.config.include_subdomains,
                &self.config.link_position_rules,
                "raw_html",
            );
            crate::extract::apply_custom_extractions(
                &mut page,
                text,
                &self.config.custom_extractions,
                "raw_html",
            );
        }
        if page.is_html()
            && page.final_status == Some(200)
            && page.error.is_none()
            && !page.truncated
            && !page.blocked_by_robots
        {
            crate::renderer::render_page(&mut page, &self.config, &self.seeds).await;
        }
        crate::metadata::evaluate_robots(&mut page);
        crate::metadata::sync_relationship_links(
            &mut page,
            &self.seeds,
            self.config.include_subdomains,
        );
        set_indexability(&mut page);
        if let Some(pdf) = &page.pdf
            && !matches!(
                pdf.status,
                crate::ResourceAnalysisStatus::Complete | crate::ResourceAnalysisStatus::Disabled
            )
        {
            page.indexable = false;
            page.indexability_reasons
                .push("PDF analysis incomplete; document indexability not established".into());
        }
        (page, text)
    }
}
