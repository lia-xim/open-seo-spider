use crate::{
    ControlledCrawlResult, CrawlConfig, CrawlControl, CrawlEvent, CrawlOutcome, CrawlReport,
    CrawlRequest, CrawlStore, CrawlSummary, PageRecord, PreparedSelectedRecrawl,
    RecrawlResumeOverride, RecrawlTargetObservation, RecrawlTargetState, RedirectHop, RunPlan,
    Severity, audit_pages_with_thresholds, normalize_url, parse_sitemap,
};
use crate::{
    audit::set_indexability,
    robots::{RateLimiter, RobotsCache},
    store::{DiscoveryEntry, DiscoveryKind, QueueEntry},
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
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    path::Path,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, mpsc::UnboundedSender};
use url::Url;

#[cfg(test)]
type PdfParseHook = Arc<dyn Fn() + Send + Sync>;

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
    asserted_plan: Option<RunPlan>,
    selected_base: Option<PreparedSelectedRecrawl>,
    recrawl_override: RecrawlResumeOverride,
    #[cfg(test)]
    pdf_parse_hook: Option<PdfParseHook>,
}

impl Crawler {
    pub fn new(mut config: CrawlConfig) -> anyhow::Result<Self> {
        config.prepare_input_lists()?;
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
            asserted_plan: None,
            selected_base: None,
            recrawl_override: RecrawlResumeOverride::default(),
            #[cfg(test)]
            pdf_parse_hook: None,
        })
    }

    pub fn with_store(mut self, path: impl AsRef<Path>) -> anyhow::Result<Self> {
        self.store = Some(Arc::new(CrawlStore::open(path)?));
        Ok(self)
    }

    pub fn effective_config(&self) -> &CrawlConfig {
        &self.config
    }

    pub(crate) fn selected_target_allowed(&self, url: &Url) -> bool {
        same_scope(url, &self.seeds, self.config.include_subdomains) || self.config.check_external
    }

    /// An optional assertion. A saved selected store is always detected even
    /// when the caller does not supply its plan (SQLite-only resume).
    pub fn with_run_plan(mut self, plan: RunPlan) -> anyhow::Result<Self> {
        if let RunPlan::SelectedRecrawl(plan) = &plan
            && plan.engine != crate::SELECTED_RECRAWL_ENGINE
        {
            bail!("Unsupported selected-recrawl engine");
        }
        self.asserted_plan = Some(plan);
        Ok(self)
    }

    pub fn with_selected_recrawl(
        mut self,
        prepared: PreparedSelectedRecrawl,
    ) -> anyhow::Result<Self> {
        prepared.validate()?;
        crate::recrawl::validate_saved_plan(
            &prepared.plan,
            &prepared.base_snapshot_bytes,
            &self.config,
        )?;
        self.asserted_plan = Some(RunPlan::SelectedRecrawl(prepared.plan.clone()));
        self.selected_base = Some(prepared);
        Ok(self)
    }

    pub fn with_recrawl_resume_override(mut self, options: RecrawlResumeOverride) -> Self {
        self.recrawl_override = options;
        self
    }

    pub async fn run(
        &self,
        events: Option<UnboundedSender<CrawlEvent>>,
        cancel: Arc<AtomicBool>,
    ) -> anyhow::Result<CrawlReport> {
        Ok(self
            .run_controlled(events, CrawlControl::from_cancel(cancel))
            .await?
            .report)
    }

    /// Pause stops admission and awaits every already admitted document. Cancel
    /// keeps the compatibility abort behavior; aborted requests remain pending.
    pub async fn run_controlled(
        &self,
        events: Option<UnboundedSender<CrawlEvent>>,
        control: CrawlControl,
    ) -> anyhow::Result<ControlledCrawlResult> {
        if control.is_finished() {
            bail!(
                "A finished crawl controller cannot be reused; create one for the new invocation"
            );
        }
        if let (Some(store), Some(prepared)) = (&self.store, &self.selected_base) {
            match store.run_plan()? {
                RunPlan::FullCrawl => store.initialize_selected_recrawl(prepared)?,
                RunPlan::SelectedRecrawl(plan) if plan == prepared.plan => {}
                _ => bail!("Saved selected plan differs from requested plan"),
            }
        }
        let run_plan = match &self.store {
            Some(store) => store.run_plan()?,
            None => self.asserted_plan.clone().unwrap_or_default(),
        };
        if self
            .asserted_plan
            .as_ref()
            .is_some_and(|expected| expected != &run_plan)
        {
            bail!("Saved run plan differs from the caller's asserted plan");
        }
        let selected = match &run_plan {
            RunPlan::FullCrawl => None,
            RunPlan::SelectedRecrawl(plan) => Some(plan),
        };
        let memory_base = if self.store.is_none() && selected.is_some() {
            Some(
                self.selected_base
                    .as_ref()
                    .context("An in-memory selected run needs its immutable parent snapshot")?,
            )
        } else {
            None
        };
        if selected.is_none() && self.recrawl_override.max_attempts.is_some() {
            bail!("max_attempts applies only to selected recrawl");
        }
        let started_at = Utc::now().to_rfc3339();
        let started = Instant::now();
        let resumed = match &self.store {
            Some(store) => store.prepare(&self.config)?,
            None => false,
        };
        let generation = self
            .store
            .as_ref()
            .map(|store| store.begin_run(&started_at))
            .transpose()?;
        let max_attempts = if let Some(plan) = selected {
            match (&self.store, generation) {
                (Some(store), Some(generation)) => store
                    .start_recrawl_invocation(generation, self.recrawl_override.max_attempts)?,
                _ => self
                    .recrawl_override
                    .max_attempts
                    .map_or(plan.default_max_attempts, |value| {
                        (value > 0).then_some(value)
                    }),
            }
        } else {
            None
        };
        let mut pages = match &self.store {
            Some(store) => store.pages()?,
            None => memory_base
                .map(|prepared| {
                    let targets = prepared.plan.targets_set();
                    prepared
                        .base_report
                        .pages
                        .iter()
                        .filter(|page| !targets.contains(page.url.as_str()))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default(),
        };
        let mut queue: VecDeque<QueueEntry> = match &self.store {
            Some(store) => store.pending()?.into(),
            None => selected
                .map(|plan| {
                    plan.targets
                        .iter()
                        .map(|target| QueueEntry {
                            url: target.requested_url.clone(),
                            depth: target.depth,
                            discovered_from: target.discovered_from.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        };
        let mut operation = if let Some(plan) = selected {
            if let Some(store) = &self.store {
                store.recrawl_operation_for_runner()?
            } else {
                Some(
                    plan.operation(
                        pages.len(),
                        plan.targets
                            .iter()
                            .map(|target| RecrawlTargetObservation {
                                requested_url: target.requested_url.clone(),
                                parent_ordinal: target.parent_ordinal,
                                parent_page_sha256: target.parent_page_sha256.clone(),
                                state: RecrawlTargetState::Pending,
                                admitted_attempts: 0,
                                committed_generation: None,
                            })
                            .collect(),
                        max_attempts,
                        0,
                        0,
                    ),
                )
            }
        } else {
            None
        };
        let target_indices: HashMap<_, _> = operation
            .as_ref()
            .map(|operation| {
                operation
                    .targets
                    .iter()
                    .enumerate()
                    .map(|(index, target)| (target.requested_url.clone(), index))
                    .collect()
            })
            .unwrap_or_default();
        if let Some(operation) = &operation {
            emit(
                &events,
                CrawlEvent::Operation {
                    operation: Box::new(operation.clone()),
                },
            );
        }
        let mut seen: HashSet<String> = pages
            .iter()
            .map(|page| page.url.clone())
            .chain(queue.iter().map(|entry| entry.url.clone()))
            .collect();
        if resumed || selected.is_some() {
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
        for seed in self.config.seed_urls.iter().filter(|_| selected.is_none()) {
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
        let discovery = if selected.is_some() {
            DiscoveryProgress::default()
        } else {
            self.run_discovery(
                &control,
                &events,
                &mut seen,
                (pages.len(), queue.len()),
                generation,
            )
            .await?
        };
        queue.extend(discovery.pages);
        let mut warnings = discovery.warnings;
        if selected.is_some() {
            warnings.push("Selected recrawl: unselected observations are inherited unchanged; pending selected historical rows are excluded. This is mixed-age coverage, not a fresh full-site crawl.".into());
        }
        let context = Arc::new(FetchContext {
            config: self.config.clone(),
            seeds: self.seeds.clone(),
            client: self.client.clone(),
            headers: self.headers.clone(),
            robots: self.robots.clone(),
            rate: self.rate.clone(),
            pdf_slots: self.pdf_slots.clone(),
            #[cfg(test)]
            pdf_parse_hook: self.pdf_parse_hook.clone(),
        });
        let mut tasks: FuturesUnordered<BoxFuture<'static, PageRecord>> = FuturesUnordered::new();
        let mut limited = false;
        loop {
            if control.request() == CrawlRequest::CancelRequested {
                break;
            }
            while control.request() == CrawlRequest::Running
                && tasks.len() < self.config.concurrency
                && !queue.is_empty()
            {
                let at_limit = match &operation {
                    Some(operation) => operation
                        .max_attempts
                        .is_some_and(|limit| operation.admitted_this_run >= limit),
                    None => self
                        .config
                        .max_pages
                        .is_some_and(|limit| pages.len() + tasks.len() >= limit),
                };
                if at_limit {
                    limited = true;
                    break;
                }
                let entry = queue.pop_front().expect("nonempty queue");
                if let Some(operation) = &mut operation {
                    if let Some(store) = &self.store {
                        store.admit_selected(&entry.url, generation.expect("stored generation"))?;
                    }
                    let index = *target_indices
                        .get(&entry.url)
                        .context("Pending URL is not a selected target")?;
                    operation.admitted_this_run += 1;
                    operation.targets[index].admitted_attempts += 1;
                    emit_recrawl_progress(&events, operation, index);
                }
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
                _ = control.changed() => continue,
            };
            let mut discoveries = Vec::new();
            if selected.is_none()
                && self.config.crawl_links
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
                store.complete(
                    &page,
                    &discoveries,
                    generation.expect("stored run generation"),
                )?;
            }
            if let Some(operation) = &mut operation {
                let index = *target_indices
                    .get(&page.url)
                    .context("Result is not a selected target")?;
                operation.targets[index].state = RecrawlTargetState::Committed;
                operation.targets[index].committed_generation = Some(generation.unwrap_or(1));
                operation.committed_this_run += 1;
                operation.refreshed_count += 1;
                operation.pending_count -= 1;
                emit_recrawl_progress(&events, operation, index);
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
        if let Some(plan) = selected {
            let order: HashMap<_, _> = if let Some(store) = &self.store {
                // One immutable snapshot read at terminal publication, not per
                // page. Ordinary store ordering is already stable on resume.
                let bytes = store
                    .parent_snapshot_bytes()?
                    .context("Immutable parent missing")?;
                let base: CrawlReport = serde_json::from_slice(&bytes)?;
                base.pages
                    .iter()
                    .enumerate()
                    .map(|(index, page)| (page.url.clone(), index))
                    .collect()
            } else {
                memory_base
                    .expect("validated memory base")
                    .base_report
                    .pages
                    .iter()
                    .enumerate()
                    .map(|(index, page)| (page.url.clone(), index))
                    .collect()
            };
            pages.sort_by_key(|page| order.get(&page.url).copied().unwrap_or(usize::MAX));
            if operation
                .as_ref()
                .is_none_or(|op| op.plan_sha256 != plan.plan_sha256 || op.pending_count != pending)
            {
                bail!("Selected outcome does not match pending plan evidence");
            }
        }
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
        let final_request = control.finish();
        let is_limited = (limited && pending > 0) || discovery.limited;
        let outcome = match final_request {
            CrawlRequest::CancelRequested => CrawlOutcome::Cancelled,
            CrawlRequest::PauseRequested => CrawlOutcome::Paused,
            CrawlRequest::Running if is_limited => CrawlOutcome::Limited,
            CrawlRequest::Running => CrawlOutcome::Completed,
        };
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
            cancelled: outcome == CrawlOutcome::Cancelled,
            limited: is_limited,
            status_counts,
        };
        let report = CrawlReport {
            config: self.config.clone(),
            pages,
            issues,
            summary: summary.clone(),
            warnings,
            operation: operation.clone(),
        };
        let checkpoint = self
            .store
            .as_ref()
            .map(|store| {
                store.save_checkpoint(&report, outcome, generation.expect("stored run generation"))
            })
            .transpose()?;
        emit(&events, CrawlEvent::Finished { summary });
        if let Some(operation) = operation {
            emit(
                &events,
                CrawlEvent::Operation {
                    operation: Box::new(operation),
                },
            );
        }
        Ok(ControlledCrawlResult {
            report,
            outcome,
            checkpoint,
            discovery_pending: discovery.pending,
        })
    }

    fn discovery_seeds(&self) -> Vec<DiscoveryEntry> {
        let mut entries = Vec::new();
        if self.config.crawl_links && self.config.discover_sitemaps && self.config.obey_robots {
            let mut origins = HashSet::new();
            for seed in &self.seeds {
                if origins.insert(origin(seed)) {
                    entries.push(DiscoveryEntry {
                        kind: DiscoveryKind::Robots,
                        url: format!("{}/", origin(seed)),
                    });
                }
            }
        }
        entries.extend(self.config.sitemap_urls.iter().map(|url| DiscoveryEntry {
            kind: DiscoveryKind::Sitemap,
            url: url.clone(),
        }));
        entries
    }

    async fn run_discovery(
        &self,
        control: &CrawlControl,
        events: &Option<UnboundedSender<CrawlEvent>>,
        page_seen: &mut HashSet<String>,
        initial: (usize, usize),
        generation: Option<u64>,
    ) -> anyhow::Result<DiscoveryProgress> {
        let seeds = self.discovery_seeds();
        let mut progress = DiscoveryProgress::default();
        let (mut queue, mut discovered, mut completed_sitemaps) = if let Some(store) = &self.store {
            store.initialize_discovery(&seeds)?;
            progress.warnings = store.discovery_warnings()?;
            if store.discovery_complete()? {
                return Ok(progress);
            }
            (
                VecDeque::from(store.discovery_entries(true)?),
                store
                    .discovery_entries(false)?
                    .into_iter()
                    .collect::<HashSet<_>>(),
                store.completed_sitemaps()?,
            )
        } else {
            let mut discovered = HashSet::new();
            let queue = seeds
                .into_iter()
                .filter(|entry| discovered.insert(entry.clone()))
                .collect();
            (queue, discovered, 0)
        };
        let context = FetchContext {
            config: self.config.clone(),
            seeds: self.seeds.clone(),
            client: self.client.clone(),
            headers: self.headers.clone(),
            robots: self.robots.clone(),
            rate: self.rate.clone(),
            pdf_slots: self.pdf_slots.clone(),
            #[cfg(test)]
            pdf_parse_hook: self.pdf_parse_hook.clone(),
        };
        while let Some(entry) = queue.front().cloned() {
            if control.request() != CrawlRequest::Running {
                break;
            }
            if entry.kind == DiscoveryKind::Sitemap
                && completed_sitemaps >= self.config.max_sitemap_documents
            {
                let warning = format!(
                    "Sitemap document resource limit ({}) reached",
                    self.config.max_sitemap_documents
                );
                emit(
                    events,
                    CrawlEvent::Warning {
                        message: warning.clone(),
                    },
                );
                progress.warnings.push(warning);
                progress.limited = true;
                break;
            }
            emit(
                events,
                CrawlEvent::Progress {
                    completed: initial.0,
                    queued: initial.1 + progress.pages.len() + queue.len().saturating_sub(1),
                    in_flight: 1,
                },
            );
            // Pause deliberately does not drop this future. Cancel retains the
            // existing abort behavior and leaves this unit pending in storage.
            let mut output = tokio::select! {
                output = self.discover_unit(&entry, &context) => output,
                _ = control.cancelled() => break,
            };
            output.pages.retain(|page| {
                self.filters.allows(&page.url) && page_seen.insert(page.url.clone())
            });
            output
                .children
                .retain(|child| discovered.insert(child.clone()));
            if let Some(store) = &self.store {
                store.complete_discovery(
                    &entry,
                    &output.pages,
                    &output.children,
                    &output.warnings,
                    generation.expect("stored run generation"),
                )?;
            }
            queue.pop_front();
            queue.extend(output.children);
            if entry.kind == DiscoveryKind::Sitemap {
                completed_sitemaps += 1;
            }
            progress.pages.extend(output.pages);
            for warning in output.warnings {
                emit(
                    events,
                    CrawlEvent::Warning {
                        message: warning.clone(),
                    },
                );
                if !progress.warnings.contains(&warning) {
                    progress.warnings.push(warning);
                }
            }
        }
        progress.pending = queue.len();
        emit(
            events,
            CrawlEvent::Progress {
                completed: initial.0,
                queued: initial.1 + progress.pages.len() + progress.pending,
                in_flight: 0,
            },
        );
        Ok(progress)
    }

    async fn discover_unit(
        &self,
        entry: &DiscoveryEntry,
        context: &FetchContext,
    ) -> DiscoveryOutput {
        let mut output = DiscoveryOutput::default();
        let Some(parsed) = normalize_url(&entry.url, None) else {
            output
                .warnings
                .push(format!("Invalid sitemap URL {}", entry.url));
            return output;
        };
        if !same_scope(&parsed, &self.seeds, self.config.include_subdomains) {
            output.warnings.push(format!(
                "Sitemap outside configured scope skipped: {}",
                entry.url
            ));
            return output;
        }
        if entry.kind == DiscoveryKind::Robots {
            output
                .children
                .extend(
                    self.robots
                        .get(&parsed)
                        .await
                        .sitemaps
                        .iter()
                        .map(|url| DiscoveryEntry {
                            kind: DiscoveryKind::Sitemap,
                            url: url.clone(),
                        }),
                );
            return output;
        }
        let (page, body) = context
            .document(QueueEntry {
                url: parsed.to_string(),
                depth: 0,
                discovered_from: None,
            })
            .await;
        if page.error.is_some()
            || page.final_status != Some(200)
            || page.truncated
            || body.is_none()
        {
            output.warnings.push(format!(
                "Sitemap could not be read completely: {}: {}",
                entry.url,
                page.error
                    .unwrap_or_else(|| format!("status {:?}", page.final_status)),
            ));
            return output;
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
                        output.children.push(DiscoveryEntry {
                            kind: DiscoveryKind::Sitemap,
                            url: parsed.to_string(),
                        });
                    } else {
                        output.pages.push(QueueEntry {
                            url: parsed.to_string(),
                            depth: 0,
                            discovered_from: Some(entry.url.clone()),
                        });
                    }
                }
            }
            Err(error) => output
                .warnings
                .push(format!("Invalid sitemap XML at {}: {error}", entry.url)),
        }
        output
    }
}

#[derive(Default)]
struct DiscoveryOutput {
    pages: Vec<QueueEntry>,
    children: Vec<DiscoveryEntry>,
    warnings: Vec<String>,
}

#[derive(Default)]
struct DiscoveryProgress {
    pages: Vec<QueueEntry>,
    warnings: Vec<String>,
    pending: usize,
    limited: bool,
}

fn emit(events: &Option<UnboundedSender<CrawlEvent>>, event: CrawlEvent) {
    if let Some(events) = events {
        let _ = events.send(event);
    }
}

fn emit_recrawl_progress(
    events: &Option<UnboundedSender<CrawlEvent>>,
    operation: &crate::SelectedRecrawlOperation,
    index: usize,
) {
    emit(
        events,
        CrawlEvent::RecrawlProgress {
            target: Box::new(operation.targets[index].clone()),
            admitted_this_run: operation.admitted_this_run,
            committed_this_run: operation.committed_this_run,
            refreshed_count: operation.refreshed_count,
            pending_count: operation.pending_count,
        },
    );
}

struct FetchContext {
    config: CrawlConfig,
    seeds: Vec<Url>,
    client: Client,
    headers: HeaderMap,
    robots: Arc<RobotsCache>,
    rate: Arc<RateLimiter>,
    pdf_slots: Arc<Semaphore>,
    #[cfg(test)]
    pdf_parse_hook: Option<PdfParseHook>,
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
                #[cfg(test)]
                let parse_hook = self.pdf_parse_hook.clone();
                match tokio::task::spawn_blocking(move || {
                    let _permit = permit;
                    #[cfg(test)]
                    if let Some(hook) = parse_hook {
                        hook();
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;
    use std::sync::{Condvar, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct ParserRelease(Arc<(Mutex<bool>, Condvar)>);

    impl ParserRelease {
        fn release(&self) {
            *self.0.0.lock().unwrap() = true;
            self.0.1.notify_all();
        }
    }

    impl Drop for ParserRelease {
        fn drop(&mut self) {
            self.release();
        }
    }

    #[tokio::test]
    async fn pause_waits_for_the_started_blocking_pdf_parser_before_checkpointing() {
        let mut pdf = lopdf::Document::with_version("1.7");
        let pages_id = pdf.new_object_id();
        let page = pdf.add_object(lopdf::dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![0.into(),0.into(),612.into(),792.into()],
            "Resources" => lopdf::dictionary! {},
        });
        pdf.objects.insert(
            pages_id,
            lopdf::dictionary! {
                "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
            }
            .into(),
        );
        let catalog =
            pdf.add_object(lopdf::dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        pdf.trailer.set("Root", catalog);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/blank.pdf", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                if n == 0 {
                    return;
                }
                request.extend_from_slice(&buffer[..n]);
            }
            let header = format!(
                "HTTP/1.1 200 Fixture\r\nContent-Type: application/pdf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            );
            socket.write_all(header.as_bytes()).await.unwrap();
            socket.write_all(&bytes).await.unwrap();
        });
        let release = ParserRelease(Arc::new((Mutex::new(false), Condvar::new())));
        let barrier = release.0.clone();
        let (started_sender, started) = tokio::sync::oneshot::channel();
        let started_sender = Mutex::new(Some(started_sender));
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("parser.sqlite");
        let mut crawler = Crawler::new(CrawlConfig {
            seed_urls: vec![url],
            obey_robots: false,
            discover_sitemaps: false,
            crawl_links: false,
            concurrency: 1,
            request_timeout_secs: 5,
            ..Default::default()
        })
        .unwrap()
        .with_store(&path)
        .unwrap();
        crawler.pdf_parse_hook = Some(Arc::new(move || {
            started_sender
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .send(())
                .unwrap();
            let (mutex, condition) = &*barrier;
            let mut released = mutex.lock().unwrap();
            while !*released {
                released = condition.wait(released).unwrap();
            }
        }));
        let control = CrawlControl::new();
        let cloned = control.clone();
        let (sender, mut events) = tokio::sync::mpsc::unbounded_channel();
        let run = tokio::spawn(async move { crawler.run_controlled(Some(sender), cloned).await });
        tokio::time::timeout(Duration::from_secs(5), started)
            .await
            .unwrap()
            .unwrap();
        while events.try_recv().is_ok() {}
        control.request_pause();
        tokio::time::timeout(Duration::from_secs(3), async {
            while let Some(event) = events.recv().await {
                if matches!(event, CrawlEvent::Progress { in_flight: 1, .. }) {
                    return;
                }
            }
            panic!("parser drain progress absent");
        })
        .await
        .unwrap();
        assert!(!run.is_finished());
        let before = CrawlStore::inspect(&path).unwrap();
        assert!(!before.checkpoint_valid);
        assert_eq!(before.completed, 0);
        assert_eq!(before.pending, 1);
        release.release();
        let result = tokio::time::timeout(Duration::from_secs(5), run)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(result.outcome, CrawlOutcome::Paused);
        assert_eq!(result.report.pages.len(), 1);
        assert_eq!(
            result.report.pages[0].pdf.as_ref().unwrap().page_count,
            Some(1)
        );
        assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
        server.await.unwrap();
    }
}
