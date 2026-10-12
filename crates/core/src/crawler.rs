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
    policy: Option<Arc<crate::CompiledCrawlPolicy>>,
    policy_limits: crate::LiveCrawlPolicyLimits,
    capture: Option<Arc<crate::CompiledContentCaptureConfig>>,
    #[cfg(test)]
    pdf_parse_hook: Option<PdfParseHook>,
}

impl Crawler {
    pub fn new(mut config: CrawlConfig) -> anyhow::Result<Self> {
        config.prepare_input_lists()?;
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
        let capture = config
            .content_capture
            .as_ref()
            .map(crate::ContentCaptureConfig::compile)
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
            policy,
            policy_limits,
            capture,
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
        same_scope(url, &self.seeds, self.config.include_subdomains)
            || self.config.check_external
            || self
                .config
                .resource_discovery
                .as_ref()
                .is_some_and(|config| {
                    config.permits_resource_origin(url, &self.seeds, self.config.include_subdomains)
                })
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
        if self.policy.is_some() && self.store.is_none() {
            bail!(
                "Native crawl policy requires explicit SQLite storage for durable admission and ledger evidence"
            );
        }
        if self.config.resource_discovery.is_some() && self.store.is_none() {
            bail!(
                "Native resource discovery requires explicit SQLite storage for transport, source and grant evidence"
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
        if let (Some(policy), Some(store), Some(generation)) =
            (&self.policy, &self.store, generation)
            && selected.is_none()
        {
            let mut rows = self.initial_policy_rows(policy);
            store.initialize_policy(policy, &mut rows, &self.policy_limits, generation)?;
        }
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
                            admission: target.policy_admission.clone(),
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
        for seed in self
            .config
            .seed_urls
            .iter()
            .filter(|_| selected.is_none() && self.policy.is_none())
        {
            let entry = QueueEntry {
                url: seed.clone(),
                depth: 0,
                discovered_from: None,
                admission: None,
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
        if let (Some(config), Some(store), Some(generation)) =
            (&self.config.resource_discovery, &self.store, generation)
        {
            store.initialize_resources(
                config,
                &self
                    .config
                    .resource_discovery_limits
                    .clone()
                    .unwrap_or_default(),
                generation,
            )?;
        }
        let mut warnings = discovery.warnings;
        if selected.is_some() {
            warnings.push("Selected recrawl: unselected observations are inherited unchanged; pending selected historical rows are excluded. This is mixed-age coverage, not a fresh full-site crawl.".into());
        }
        let context = Arc::new(FetchContext {
            store: self.store.clone(),
            capture: self.capture.clone(),
            generation,
            config: self.config.clone(),
            seeds: self.seeds.clone(),
            client: self.client.clone(),
            headers: self.headers.clone(),
            robots: self.robots.clone(),
            rate: self.rate.clone(),
            pdf_slots: self.pdf_slots.clone(),
            policy: self.policy.clone(),
            #[cfg(test)]
            pdf_parse_hook: self.pdf_parse_hook.clone(),
        });
        let mut tasks: FuturesUnordered<BoxFuture<'static, anyhow::Result<PageRecord>>> =
            FuturesUnordered::new();
        let mut completed_transports = self
            .store
            .as_ref()
            .map(|store| store.resource_ledger_summary())
            .transpose()?
            .flatten()
            .map(|summary| summary.completed_transports);
        let mut limited = false;
        loop {
            if control.request() == CrawlRequest::CancelRequested {
                break;
            }
            // Expansion is a durable metadata-only unit. No second HTTP is
            // issued when an independent late role augments a captured URL.
            if control.request() == CrawlRequest::Running
                && let Some(store) = &self.store
                && self.config.resource_discovery.is_some()
                && let Some(mut page) = store.resource_expansion()?
            {
                let (mut rows, mut policy_rows) = self.resource_rows(
                    &mut page,
                    selected.is_some(),
                    generation.expect("stored generation"),
                )?;
                let discovered = store.complete_resources(
                    &page,
                    &mut rows,
                    &mut policy_rows,
                    generation.expect("stored generation"),
                    true,
                )?;
                queue.extend(discovered);
                continue;
            }
            while control.request() == CrawlRequest::Running
                && tasks.len() < self.config.concurrency
                && !queue.is_empty()
            {
                let at_limit = match &operation {
                    Some(operation) => operation
                        .max_attempts
                        .is_some_and(|limit| operation.admitted_this_run >= limit),
                    None => self.config.max_pages.is_some_and(|limit| {
                        completed_transports.unwrap_or(pages.len()) + tasks.len() >= limit
                    }),
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
                tasks.push(
                    async move {
                        let (mut page, text) = fetch.document(entry).await;
                        if let Some(config) = &fetch.config.resource_discovery {
                            let decode = page
                                .resource_response
                                .as_ref()
                                .map_or(crate::ContentCaptureDecodeStatus::Unknown, |facts| {
                                    facts.decode_status
                                });
                            page.resource_source = Some(crate::resource_capture::inventory(
                                &page,
                                text.as_deref(),
                                decode,
                                &fetch.seeds,
                                fetch.config.include_subdomains,
                                config,
                                &fetch
                                    .config
                                    .resource_discovery_limits
                                    .clone()
                                    .unwrap_or_default(),
                            )?);
                        }
                        Ok(page)
                    }
                    .boxed(),
                );
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
            let mut page = tokio::select! {
                page = tasks.next() => page.expect("nonempty tasks")?,
                _ = control.changed() => continue,
            };
            let mut discoveries = Vec::new();
            if self.config.resource_discovery.is_none()
                && self.policy.is_none()
                && selected.is_none()
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
                            admission: None,
                        });
                    }
                }
            }
            let retained = page
                .resource_permission
                .as_ref()
                .is_none_or(|permission| permission.store);
            if self.config.resource_discovery.is_some() {
                let store = self.store.as_ref().expect("resource storage required");
                let (mut rows, mut policy_rows) = self.resource_rows(
                    &mut page,
                    selected.is_some(),
                    generation.expect("stored generation"),
                )?;
                discoveries = store.complete_resources(
                    &page,
                    &mut rows,
                    &mut policy_rows,
                    generation.expect("stored generation"),
                    false,
                )?;
                if let Some(count) = &mut completed_transports {
                    *count += 1;
                }
                if let Some(ledger) = store.resource_ledger_summary()? {
                    emit(
                        &events,
                        CrawlEvent::ResourceProgress {
                            ledger: Box::new(ledger),
                        },
                    );
                }
            } else if let (Some(policy), Some(store)) = (&self.policy, &self.store) {
                let mut rows = self.page_policy_rows(&mut page, policy, selected.is_some())?;
                discoveries = store.complete_with_policy(
                    &page,
                    &mut rows,
                    generation.expect("stored generation"),
                    &self.policy_limits,
                )?;
                for entry in &discoveries {
                    seen.insert(entry.url.clone());
                }
                if let Some(ledger) = store.policy_ledger_summary()? {
                    emit(
                        &events,
                        CrawlEvent::PolicyProgress {
                            ledger: Box::new(ledger),
                        },
                    );
                }
            } else if let Some(store) = &self.store {
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
            if retained {
                emit(
                    &events,
                    CrawlEvent::Page {
                        page: Box::new(page.clone()),
                    },
                );
                pages.push(page);
            }
        }
        let pending = queue.len() + tasks.len();
        drop(tasks); // In-flight requests remain pending in SQLite and can be resumed.
        if self.config.resource_discovery.is_some()
            && let Some(store) = &self.store
        {
            pages = store.pages()?;
        }
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
            resource_ledger: self
                .store
                .as_ref()
                .map(|store| store.resource_ledger_summary())
                .transpose()?
                .flatten(),
            policy_ledger: self
                .store
                .as_ref()
                .map(|store| store.policy_ledger_summary())
                .transpose()?
                .flatten(),
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

    fn resource_rows(
        &self,
        page: &mut PageRecord,
        selected: bool,
        generation: u64,
    ) -> anyhow::Result<(
        Vec<crate::StoredResourceOccurrence>,
        Vec<crate::PolicyOccurrence>,
    )> {
        let config = self
            .config
            .resource_discovery
            .as_ref()
            .context("Resource configuration missing")?;
        let permission = page
            .resource_permission
            .as_ref()
            .context("Resource permission missing")?;
        permission.validate()?;
        let batch = page
            .resource_source
            .as_ref()
            .context("Resource source inventory missing")?;
        let evaluation = crate::resource_policy::digest(format!(
            "{}:{}",
            batch.source_revision,
            permission.revision()?
        ));
        let redirects_traverse = page.policy.as_ref().is_none_or(|policy| {
            policy
                .redirect_decisions
                .iter()
                .all(|row| row.decision.traverse)
        });
        let mut rows = Vec::new();
        let mut policy_rows = page
            .policy
            .as_ref()
            .map(|observation| observation.redirect_decisions.clone())
            .unwrap_or_default();
        if page
            .policy
            .as_ref()
            .is_some_and(|observation| observation.source_revision.starts_with("resource:"))
        {
            policy_rows.clear();
        }
        for occurrence in &batch.occurrences {
            let mut policy_row = self.policy.as_ref().map(|policy| {
                crate::policy_ledger::decide(
                    policy,
                    &format!("resource:{evaluation}"),
                    occurrence.source_ordinal,
                    if occurrence.reference_kind.is_navigation() {
                        crate::CandidateSourceKind::Anchor
                    } else {
                        crate::CandidateSourceKind::Resource
                    },
                    occurrence
                        .decoded_value
                        .clone()
                        .unwrap_or_else(|| occurrence.raw_value.clone()),
                    Some(
                        page.document_base_url
                            .as_deref()
                            .unwrap_or(&page.final_url)
                            .into(),
                    ),
                    Some(page.url.clone()),
                    if occurrence.classification_basis == "resolved_http_link_header_target" {
                        crate::PolicyInputBasis::ResolvedLegacyFallback
                    } else {
                        crate::PolicyInputBasis::ObservedAttribute
                    },
                    crate::PolicyTaskRole::PageDocument,
                )
            });
            let mut candidate = occurrence.clone();
            if let Some(row) = &policy_row {
                candidate.resolved_url = row.decision.fetch_candidate_url.clone();
            }
            let target = candidate
                .resolved_url
                .as_deref()
                .and_then(|url| normalize_url(url, None));
            let scope = target.as_ref().is_some_and(|url| {
                if occurrence.reference_kind.is_navigation() {
                    same_scope(url, &self.seeds, self.config.include_subdomains)
                        || self.config.check_external
                } else {
                    config.permits_resource_origin(url, &self.seeds, self.config.include_subdomains)
                }
            });
            let source_can_expand = match batch.source_kind {
                crate::ResourceSourceKind::NativeCss => permission.can_expand_css,
                crate::ResourceSourceKind::PdfAnnotation
                    if occurrence.reference_kind == crate::ResourceReferenceKind::PdfAnnotation =>
                {
                    permission.can_expand_pdf
                }
                _ if occurrence.reference_kind.is_navigation() => permission.can_expand_navigation,
                _ => permission.can_expand_resources,
            } && redirects_traverse;
            let mut decision = config.decide(
                &candidate,
                crate::resource_policy::ResourcePermission {
                    source_complete: occurrence.source_complete,
                    source_can_expand,
                    selected,
                    navigation_enabled: self.config.crawl_links,
                    depth_allowed: self.config.max_depth.is_none_or(|limit| page.depth < limit),
                    scope_allowed: if occurrence.reference_kind.is_navigation()
                        && self.policy.is_some()
                    {
                        policy_row.as_ref().is_some_and(|row| {
                            row.decision.decision != crate::CrawlPolicyDecisionKind::Deny
                        })
                    } else {
                        scope
                    },
                    filters_allowed: candidate
                        .resolved_url
                        .as_deref()
                        .is_some_and(|url| self.filters.allows(url)),
                    policy_check: policy_row.as_ref().is_none_or(|row| {
                        row.decision.decision != crate::CrawlPolicyDecisionKind::Deny
                    }),
                    policy_traverse: policy_row.as_ref().is_none_or(|row| row.decision.traverse),
                    document_nofollow: page
                        .effective_robots
                        .iter()
                        .any(|directive| directive == "nofollow" || directive == "none"),
                },
            )?;
            if let Some(row) = &mut policy_row {
                if let Some(grant) = &mut decision.grant {
                    grant.policy_sha256 = Some(row.decision.policy_sha256.clone());
                    grant.policy_occurrence_id = Some(row.decision.occurrence_id.clone());
                } else {
                    row.task_role = crate::PolicyTaskRole::ObservationOnly;
                    row.admission_reason = format!("resource:{}", decision.reason);
                }
            }
            rows.push(crate::StoredResourceOccurrence {
                sequence: 0,
                generation,
                basis: crate::ResourceObservationBasis::Current,
                evaluation_revision: evaluation.clone(),
                occurrence: occurrence.clone(),
                decision,
            });
            if let Some(row) = policy_row {
                policy_rows.push(row);
            }
        }
        if let Some(observation) = &mut page.policy {
            observation.source_revision = format!("resource:{evaluation}");
            observation.candidates_complete = batch.complete;
            observation.candidate_count = batch.occurrences.len();
        }
        if let Some(facts) = &mut page.resource_response {
            facts.retained_response = permission.store;
        }
        Ok((rows, policy_rows))
    }

    fn initial_policy_rows(
        &self,
        policy: &crate::CompiledCrawlPolicy,
    ) -> Vec<crate::PolicyOccurrence> {
        use crate::{
            CandidateSourceKind as Kind, PolicyInputBasis as Basis, PolicyTaskRole as Role,
        };
        let mut rows = Vec::new();
        let mut covered = HashSet::new();
        for (index, input) in self.config.input_imports.iter().enumerate() {
            let revision = format!(
                "initial:{}:{index}:{}",
                policy.policy_sha256(),
                input.input_sha256
            );
            for row in &input.rows {
                let (kind, role) = match row.candidate_kind {
                    crate::ListCandidateKind::PageUrl => (Kind::ListPage, Role::PageDocument),
                    crate::ListCandidateKind::SitemapDocument => {
                        (Kind::SitemapDocument, Role::SitemapDocument)
                    }
                };
                if let Some(url) = &row.normalized_url {
                    covered.insert((role.key(), url.clone()));
                }
                let mut occurrence = crate::policy_ledger::decide(
                    policy,
                    &revision,
                    row.source_index,
                    kind,
                    row.raw_value.clone(),
                    None,
                    None,
                    Basis::DeclaredInput,
                    role,
                );
                if row.error.is_some() || row.normalized_url.is_none() {
                    occurrence.admission_reason = "input_row_rejected".into();
                    occurrence.input_error = row
                        .error
                        .clone()
                        .or_else(|| Some("Importer supplied no accepted normalized URL".into()));
                }
                rows.push(occurrence);
            }
        }
        for (role, kind, urls) in [
            (Role::PageDocument, Kind::Seed, &self.config.seed_urls),
            (
                Role::SitemapDocument,
                Kind::SitemapDocument,
                &self.config.sitemap_urls,
            ),
        ] {
            let revision = format!("initial:{}:explicit:{}", policy.policy_sha256(), role.key());
            for (index, url) in urls.iter().enumerate() {
                if !covered.contains(&(role.key(), url.clone())) {
                    rows.push(crate::policy_ledger::decide(
                        policy,
                        &revision,
                        index,
                        kind,
                        url.clone(),
                        None,
                        None,
                        Basis::DeclaredInput,
                        role,
                    ));
                }
            }
        }
        rows
    }

    fn page_policy_rows(
        &self,
        page: &mut PageRecord,
        policy: &crate::CompiledCrawlPolicy,
        selected: bool,
    ) -> anyhow::Result<Vec<crate::PolicyOccurrence>> {
        use crate::{
            CandidateSourceKind as Kind, PolicyInputBasis as Basis, PolicyTaskRole as Role,
        };
        let revision = crate::policy_ledger::source_revision(page)?;
        let complete = page.final_status == Some(200)
            && page.error.is_none()
            && !page.truncated
            && !page.blocked_by_robots
            && page.pdf.as_ref().is_none_or(|pdf| pdf.annotations_complete);
        let observation = page
            .policy
            .as_mut()
            .context("Native policy task produced no admission evidence")?;
        observation.source_revision = revision.clone();
        observation.candidates_complete = complete;
        let expand = observation.effective_can_expand && complete;
        let mut rows = observation.redirect_decisions.clone();
        let mut candidates = Vec::new();
        for link in &page.links {
            if link.kind == "pdf_annotation" && page.pdf.is_some() {
                continue;
            }
            let kind = if link.kind == "anchor" {
                Kind::Anchor
            } else if link.kind == "pdf_annotation" {
                Kind::PdfAnnotation
            } else {
                Kind::Resource
            };
            let (raw, basis) = link
                .raw_href
                .as_ref()
                .map(|value| (value.clone(), Basis::ObservedAttribute))
                .unwrap_or_else(|| (link.url.clone(), Basis::ResolvedLegacyFallback));
            candidates.push((kind, raw, basis));
        }
        if let Some(pdf) = &page.pdf {
            candidates.extend(pdf.annotation_links.iter().map(|link| {
                (
                    Kind::PdfAnnotation,
                    link.raw_uri.clone(),
                    Basis::PdfAnnotationUri,
                )
            }));
        }
        let candidate_count = candidates.len();
        for (index, (kind, raw, basis)) in candidates.into_iter().enumerate() {
            let mut row = crate::policy_ledger::decide(
                policy,
                &revision,
                index,
                kind,
                raw,
                page.document_base_url
                    .clone()
                    .or_else(|| Some(page.document_url().into())),
                Some(page.url.clone()),
                basis,
                Role::PageDocument,
            );
            if row.admission_reason == "pending_admission" {
                row.admission_reason = if selected {
                    "selected_no_expansion"
                } else if !self.config.crawl_links {
                    "crawl_links_disabled"
                } else if self
                    .config
                    .max_depth
                    .is_some_and(|limit| page.depth >= limit)
                {
                    "depth_limit"
                } else if !complete {
                    "source_incomplete"
                } else if !expand {
                    "source_check_only"
                } else {
                    "pending_admission"
                }
                .into();
            }
            rows.push(row);
        }
        // Relationships keep their actual graph identity and remain observation-only.
        for (index, relationship) in page.link_relationships.iter().enumerate() {
            let mut row = crate::policy_ledger::decide(
                policy,
                &revision,
                candidate_count + index,
                Kind::Relationship,
                relationship.url.clone(),
                None,
                Some(page.url.clone()),
                Basis::ResolvedLegacyFallback,
                Role::ObservationOnly,
            );
            if row.admission_reason == "pending_admission" {
                row.admission_reason = "observation_only".into();
            }
            rows.push(row);
        }
        page.policy
            .as_mut()
            .expect("observed policy")
            .candidate_count = rows.len();
        Ok(rows)
    }

    fn discovery_seeds(&self) -> anyhow::Result<Vec<DiscoveryEntry>> {
        if self.policy.is_some() {
            let mut origins = HashSet::new();
            let mut entries = Vec::new();
            if self.config.crawl_links
                && self.config.discover_sitemaps
                && self.config.obey_robots
                && let Some(store) = &self.store
            {
                for task in store.pending()? {
                    if let Some(url) = normalize_url(&task.url, None)
                        && origins.insert(origin(&url))
                    {
                        entries.push(DiscoveryEntry {
                            kind: DiscoveryKind::Robots,
                            url: format!("{}/", origin(&url)),
                        });
                    }
                }
            }
            return Ok(entries);
        }
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
        Ok(entries)
    }

    async fn run_discovery(
        &self,
        control: &CrawlControl,
        events: &Option<UnboundedSender<CrawlEvent>>,
        page_seen: &mut HashSet<String>,
        initial: (usize, usize),
        generation: Option<u64>,
    ) -> anyhow::Result<DiscoveryProgress> {
        let seeds = self.discovery_seeds()?;
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
            store: self.store.clone(),
            capture: self.capture.clone(),
            generation,
            config: self.config.clone(),
            seeds: self.seeds.clone(),
            client: self.client.clone(),
            headers: self.headers.clone(),
            robots: self.robots.clone(),
            rate: self.rate.clone(),
            pdf_slots: self.pdf_slots.clone(),
            policy: self.policy.clone(),
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
                output = self.discover_unit(&entry, &context) => output?,
                _ = control.cancelled() => break,
            };
            if self.policy.is_some()
                && let Some(store) = &self.store
            {
                let revision = output
                    .policy_revision
                    .as_deref()
                    .context("Policy discovery produced no source revision")?;
                (output.pages, output.children) = store.complete_discovery_with_policy(
                    &entry,
                    &mut output.policy_rows,
                    (revision, output.policy_complete),
                    &output.warnings,
                    generation.expect("stored generation"),
                    &self.policy_limits,
                )?;
                for page in &output.pages {
                    page_seen.insert(page.url.clone());
                }
                for child in &output.children {
                    discovered.insert(child.clone());
                }
                if let Some(ledger) = store.policy_ledger_summary()? {
                    emit(
                        events,
                        CrawlEvent::PolicyProgress {
                            ledger: Box::new(ledger),
                        },
                    );
                }
            } else {
                output.pages.retain(|page| {
                    self.filters.allows(&page.url) && page_seen.insert(page.url.clone())
                });
                output
                    .children
                    .retain(|child| discovered.insert(child.clone()));
            }
            if self.policy.is_none()
                && let Some(store) = &self.store
            {
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
    ) -> anyhow::Result<DiscoveryOutput> {
        let mut output = DiscoveryOutput {
            policy_revision: Some(format!("protocol:{}:{}", entry.kind.as_str(), entry.url)),
            ..Default::default()
        };
        let Some(parsed) = normalize_url(&entry.url, None) else {
            output
                .warnings
                .push(format!("Invalid sitemap URL {}", entry.url));
            return Ok(output);
        };
        if self.policy.is_none()
            && !same_scope(&parsed, &self.seeds, self.config.include_subdomains)
        {
            output.warnings.push(format!(
                "Sitemap outside configured scope skipped: {}",
                entry.url
            ));
            return Ok(output);
        }
        if entry.kind == DiscoveryKind::Robots {
            if let Some(policy) = &self.policy {
                let robots = self.robots.get(&parsed).await;
                output.policy_complete = robots.warning.is_none();
                let revision = output
                    .policy_revision
                    .as_deref()
                    .expect("protocol revision");
                for (index, url) in robots.sitemaps.iter().enumerate() {
                    output.policy_rows.push(crate::policy_ledger::decide(
                        policy,
                        revision,
                        index,
                        crate::CandidateSourceKind::SitemapDocument,
                        url.clone(),
                        None,
                        Some(entry.url.clone()),
                        crate::PolicyInputBasis::DeclaredInput,
                        crate::PolicyTaskRole::SitemapDocument,
                    ));
                }
                return Ok(output);
            }
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
            return Ok(output);
        }
        let (page, body) = context
            .document(QueueEntry {
                url: parsed.to_string(),
                depth: 0,
                discovered_from: None,
                admission: if self.policy.is_some() {
                    self.store
                        .as_ref()
                        .context("Native policy discovery requires durable storage")?
                        .task_policy(crate::PolicyTaskRole::SitemapDocument, &entry.url)?
                } else {
                    None
                },
            })
            .await;
        if let Some(observation) = &page.policy {
            output
                .policy_rows
                .extend(observation.redirect_decisions.clone());
        }
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
            return Ok(output);
        }
        let body = body.unwrap_or_default();
        output.policy_revision = Some(crate::policy_ledger::digest(format!(
            "sitemap:{}:{body}",
            entry.url
        )));
        match parse_sitemap(&body) {
            Ok(sitemap) => {
                output.policy_complete = true;
                for (index, loc) in sitemap.locations.into_iter().enumerate() {
                    if let Some(policy) = &self.policy {
                        let role = if sitemap.is_index {
                            crate::PolicyTaskRole::SitemapDocument
                        } else {
                            crate::PolicyTaskRole::PageDocument
                        };
                        let kind = if sitemap.is_index {
                            crate::CandidateSourceKind::SitemapDocument
                        } else {
                            crate::CandidateSourceKind::SitemapLocation
                        };
                        let mut row = crate::policy_ledger::decide(
                            policy,
                            output.policy_revision.as_deref().expect("XML revision"),
                            index,
                            kind,
                            loc,
                            None,
                            Some(entry.url.clone()),
                            crate::PolicyInputBasis::ParserObservedXml,
                            role,
                        );
                        let xml_allowed = page.policy.as_ref().is_some_and(|observation| {
                            observation.admission.can_discover_xml
                                && observation
                                    .redirect_decisions
                                    .iter()
                                    .all(|row| row.decision.folder_internal)
                        });
                        if !xml_allowed && row.admission_reason == "pending_admission" {
                            row.admission_reason = "source_check_only".into();
                        }
                        output.policy_rows.push(row);
                        continue;
                    }
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
                            admission: None,
                        });
                    }
                }
            }
            Err(error) => output
                .warnings
                .push(format!("Invalid sitemap XML at {}: {error}", entry.url)),
        }
        Ok(output)
    }
}

#[derive(Default)]
struct DiscoveryOutput {
    pages: Vec<QueueEntry>,
    children: Vec<DiscoveryEntry>,
    warnings: Vec<String>,
    policy_rows: Vec<crate::PolicyOccurrence>,
    policy_revision: Option<String>,
    policy_complete: bool,
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
    store: Option<Arc<CrawlStore>>,
    capture: Option<Arc<crate::CompiledContentCaptureConfig>>,
    generation: Option<u64>,
    config: CrawlConfig,
    seeds: Vec<Url>,
    client: Client,
    headers: HeaderMap,
    robots: Arc<RobotsCache>,
    rate: Arc<RateLimiter>,
    pdf_slots: Arc<Semaphore>,
    policy: Option<Arc<crate::CompiledCrawlPolicy>>,
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
        if self.config.resource_discovery.is_some()
            && !entry.admission.as_ref().is_some_and(|admission| {
                admission.task_role == crate::PolicyTaskRole::SitemapDocument
            })
        {
            page.resource_permission = self
                .store
                .as_ref()
                .and_then(|store| store.resource_permission(&entry.url).ok())
                .flatten();
            if page.resource_permission.is_none() {
                page.error =
                    Some("Durable resource permission missing; request was not made".into());
                if let Some(capture) = &self.capture {
                    crate::content_binding::unavailable_sources(
                        &mut page,
                        capture,
                        self.generation,
                        "resource_admission_missing",
                    );
                }
                return (page, None);
            }
        }
        if let Some(policy) = &self.policy {
            let Some(admission) = entry.admission else {
                page.error = Some("Native policy admission missing; request was not made".into());
                if let Some(capture) = &self.capture {
                    crate::content_binding::unavailable_sources(
                        &mut page,
                        capture,
                        self.generation,
                        "native_admission_missing",
                    );
                }
                return (page, None);
            };
            if let Err(error) =
                admission.validate(policy.policy_sha256(), &page.url, admission.task_role)
            {
                page.error = Some(error.to_string());
                if let Some(capture) = &self.capture {
                    crate::content_binding::unavailable_sources(
                        &mut page,
                        capture,
                        self.generation,
                        "native_admission_invalid",
                    );
                }
                return (page, None);
            }
            let can_expand =
                page.resource_permission
                    .as_ref()
                    .map_or(admission.can_expand, |permission| {
                        permission.can_expand_navigation
                            || permission.can_expand_resources
                            || permission.can_expand_css
                            || permission.can_expand_pdf
                    });
            page.policy = Some(crate::PagePolicyObservation {
                admission,
                effective_can_expand: can_expand,
                redirect_decisions: Vec::new(),
                source_revision: String::new(),
                candidates_complete: false,
                candidate_count: 0,
            });
        }
        let Some(mut current) = normalize_url(&entry.url, None) else {
            page.error = Some("Invalid HTTP(S) URL".into());
            set_indexability(&mut page);
            if let Some(capture) = &self.capture {
                crate::content_binding::unavailable_sources(
                    &mut page,
                    capture,
                    self.generation,
                    "invalid_request_url",
                );
            }
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
                // Observe literal Location before parser/loop/limit stops. A
                // sitemap protocol response has no retained PageRecord later.
                let redirect_decision =
                    if let (Some(policy), Some(location)) = (&self.policy, location) {
                        let observation = page.policy.as_mut().expect("validated policy admission");
                        let revision = format!("redirect:{}", observation.admission.occurrence_id);
                        let mut decision = crate::policy_ledger::decide(
                            policy,
                            &revision,
                            page.redirect_chain.len() + 1,
                            crate::CandidateSourceKind::Redirect,
                            location.into(),
                            Some(current.to_string()),
                            Some(page.url.clone()),
                            crate::PolicyInputBasis::ObservedRedirectLocation,
                            crate::PolicyTaskRole::ObservationOnly,
                        );
                        observation.effective_can_expand &= decision.decision.traverse;
                        decision.admission_reason =
                            if decision.decision.decision == crate::CrawlPolicyDecisionKind::Deny {
                                "redirect_denied"
                            } else {
                                "redirect_followed"
                            }
                            .into();
                        let index = observation.redirect_decisions.len();
                        observation.redirect_decisions.push(decision);
                        Some(index)
                    } else {
                        None
                    };
                let Some(target) =
                    location.and_then(|location| normalize_url(location, Some(&current)))
                else {
                    if let Some(index) = redirect_decision {
                        page.policy
                            .as_mut()
                            .expect("observed policy")
                            .redirect_decisions[index]
                            .admission_reason = "redirect_invalid".into();
                    }
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
                    if let Some(index) = redirect_decision {
                        page.policy
                            .as_mut()
                            .expect("observed policy")
                            .redirect_decisions[index]
                            .admission_reason = "redirect_stopped_loop".into();
                    }
                    page.error = Some("Redirect loop detected".into());
                    page.final_status = None;
                    break;
                }
                if page.redirect_chain.len() > self.config.max_redirects {
                    if let Some(index) = redirect_decision {
                        page.policy
                            .as_mut()
                            .expect("observed policy")
                            .redirect_decisions[index]
                            .admission_reason = "redirect_stopped_limit".into();
                    }
                    page.error = Some(format!(
                        "Redirect limit ({}) exceeded",
                        self.config.max_redirects
                    ));
                    page.final_status = None;
                    break;
                }
                if redirect_decision.is_some_and(|index| {
                    page.policy
                        .as_ref()
                        .expect("observed policy")
                        .redirect_decisions[index]
                        .decision
                        .decision
                        == crate::CrawlPolicyDecisionKind::Deny
                }) {
                    page.warnings.push(
                        "Redirect target denied by native crawl policy; no target request was made"
                            .into(),
                    );
                    break;
                }
                if self.policy.is_none()
                    && !same_scope(&target, &self.seeds, self.config.include_subdomains)
                    && !self.config.check_external
                    && !self
                        .config
                        .resource_discovery
                        .as_ref()
                        .is_some_and(|config| {
                            config.permits_resource_origin(
                                &target,
                                &self.seeds,
                                self.config.include_subdomains,
                            )
                        })
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
        let resource_entity = self
            .config
            .resource_discovery
            .as_ref()
            .filter(|_| {
                page.final_status
                    .is_some_and(|status| !matches!(status, 301 | 302 | 303 | 307 | 308))
                    && !page.blocked_by_robots
            })
            .map(|_| crate::resource_policy::digest(&body));
        let entity = self
            .capture
            .as_ref()
            .filter(|capture| {
                capture
                    .config()
                    .sources
                    .contains(&crate::ContentCaptureSourceKind::RawHttpHtml)
            })
            .and_then(|_| {
                page.final_status
                    .filter(|status| !matches!(status, 301 | 302 | 303 | 307 | 308))
            })
            .map(|_| {
                crate::content_binding::entity_bytes(
                    &body,
                    page.error.is_none() && !page.truncated && !page.blocked_by_robots,
                )
            });
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
                        raw_href: Some(link.raw_uri.clone()),
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
        let mut decode_status = crate::ContentCaptureDecodeStatus::Unknown;
        let mut decode_encoding = None;
        let text = if !pdf
            && page.image_metadata.is_none()
            && textual
            && (!body.is_empty() || page.is_html())
        {
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
            let declared_encoding =
                charset.and_then(|label| encoding_rs::Encoding::for_label(label.as_bytes()));
            let encoding = declared_encoding.unwrap_or(encoding_rs::UTF_8);
            let (decoded, used_encoding, errors) = encoding.decode(&body);
            decode_status = if errors {
                crate::ContentCaptureDecodeStatus::Lossy
            } else if charset.is_some() && declared_encoding.is_none() {
                crate::ContentCaptureDecodeStatus::Unknown
            } else {
                crate::ContentCaptureDecodeStatus::Exact
            };
            decode_encoding = Some(used_encoding.name().into());
            if errors {
                page.warnings.push(
                    "Response text contains invalid bytes for the declared character encoding"
                        .into(),
                );
            }
            if self.config.resource_discovery.is_some() && media_type == "text/css" {
                let (decoded, status, encoding) =
                    crate::resource_capture::decode_css(&body, &page.content_type);
                decode_status = status;
                decode_encoding = Some(encoding);
                Some(decoded)
            } else {
                Some(decoded.into_owned())
            }
        } else {
            None
        };
        if let Some(capture) = &self.capture {
            let unavailable = if page.blocked_by_robots {
                Some("blocked_by_robots")
            } else if entity.is_none() {
                Some("native_response_not_observed")
            } else if !page.is_html() {
                Some("native_non_html")
            } else {
                None
            };
            let complete = page.final_status == Some(200)
                && page.error.is_none()
                && !page.truncated
                && !page.blocked_by_robots;
            let document_url = entity.as_ref().map(|_| page.final_url.clone());
            crate::content_binding::observe(
                &mut page,
                capture,
                self.generation,
                crate::content_binding::SourceObservation {
                    source_kind: crate::ContentCaptureSourceKind::RawHttpHtml,
                    html: if unavailable.is_none() {
                        text.as_deref()
                    } else {
                        None
                    },
                    source_complete: complete && unavailable.is_none(),
                    decode_status,
                    decode_encoding: decode_encoding.clone(),
                    incomplete_reason: if !complete && unavailable.is_none() {
                        Some("native_response_incomplete_or_non200".into())
                    } else {
                        None
                    },
                    unavailable_reason: unavailable.map(str::to_owned),
                    document_url,
                    rendered_at: None,
                    entity,
                },
            );
            let reason = if self.config.renderer.is_none() {
                "renderer_not_configured"
            } else {
                "native_response_not_render_eligible"
            };
            crate::content_binding::observe(
                &mut page,
                capture,
                self.generation,
                crate::content_binding::SourceObservation {
                    source_kind: crate::ContentCaptureSourceKind::RenderedDomHtml,
                    html: None,
                    source_complete: false,
                    decode_status: crate::ContentCaptureDecodeStatus::Unknown,
                    decode_encoding: None,
                    incomplete_reason: None,
                    unavailable_reason: Some(reason.into()),
                    document_url: None,
                    rendered_at: None,
                    entity: None,
                },
            );
        }
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
            crate::renderer::render_page(
                &mut page,
                &self.config,
                &self.seeds,
                self.capture.as_deref(),
                self.generation,
            )
            .await;
        }
        crate::metadata::evaluate_robots(&mut page);
        crate::metadata::sync_relationship_links(
            &mut page,
            &self.seeds,
            self.config.include_subdomains,
        );
        set_indexability(&mut page);
        if self.config.resource_discovery.is_some() && page.resource_permission.is_some() {
            page.resource_response = Some(crate::ResourceResponseFacts {
                engine: crate::RESOURCE_DISCOVERY_ENGINE.into(),
                observed_generation: self.generation.unwrap_or(1),
                retained_response: page
                    .resource_permission
                    .as_ref()
                    .is_some_and(|permission| permission.store),
                entity_body_retained: false,
                captured_entity_sha256: resource_entity,
                captured_entity_bytes: page.size_bytes,
                captured_entity_complete: page.final_status.is_some()
                    && !matches!(page.final_status, Some(301 | 302 | 303 | 307 | 308))
                    && page.error.is_none()
                    && !page.truncated
                    && !page.blocked_by_robots,
                decoded_source_sha256: text.as_ref().map(crate::resource_policy::digest),
                decode_status,
                decode_encoding,
                metadata_projection_complete: true,
                unretained_fields: vec!["entity_body".into()],
            });
        }
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

    #[test]
    fn pdf_policy_candidates_keep_raw_uri_and_rejected_actions_without_false_completeness() {
        let crawler = Crawler::new(CrawlConfig {
            seed_urls: vec!["https://example.org/document.pdf".into()],
            crawl_policy: Some(crate::CrawlPolicyConfig {
                scope: crate::CrawlScopeConfig {
                    roots: vec![crate::CrawlScopeRoot {
                        origin: "https://example.org".into(),
                        subdomains: false,
                        folders: vec!["/".into()],
                        exact_paths: vec![],
                    }],
                    ..Default::default()
                },
                ..Default::default()
            }),
            ..Default::default()
        })
        .unwrap();
        let policy = crawler.policy.as_ref().unwrap();
        let initial = crate::policy_ledger::decide(
            policy,
            "initial-pdf",
            0,
            crate::CandidateSourceKind::Seed,
            "https://example.org/document.pdf".into(),
            None,
            None,
            crate::PolicyInputBasis::DeclaredInput,
            crate::PolicyTaskRole::PageDocument,
        );
        let admission = crate::policy_ledger::admission(&initial).unwrap();
        let mut page = PageRecord {
            url: admission.fetch_url.clone(),
            final_url: admission.fetch_url.clone(),
            final_status: Some(200),
            content_type: "application/pdf".into(),
            links: vec![crate::LinkRecord {
                url: "https://example.org/next#part".into(),
                kind: "pdf_annotation".into(),
                ..Default::default()
            }],
            pdf: Some(crate::PdfRecord {
                annotations_complete: true,
                status: crate::ResourceAnalysisStatus::Complete,
                annotation_links: vec![
                    crate::PdfAnnotationLink {
                        page: 1,
                        raw_uri: "next#part".into(),
                        url: Some("https://example.org/next".into()),
                    },
                    crate::PdfAnnotationLink {
                        page: 1,
                        raw_uri: "#local".into(),
                        url: Some("https://example.org/document.pdf".into()),
                    },
                    crate::PdfAnnotationLink {
                        page: 2,
                        raw_uri: "ftp://example.org/unfetchable".into(),
                        url: None,
                    },
                ],
                ..Default::default()
            }),
            policy: Some(crate::PagePolicyObservation {
                admission,
                effective_can_expand: true,
                redirect_decisions: vec![],
                source_revision: String::new(),
                candidates_complete: false,
                candidate_count: 0,
            }),
            ..Default::default()
        };
        let rows = crawler.page_policy_rows(&mut page, policy, false).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|row| row.input_basis
            == crate::PolicyInputBasis::PdfAnnotationUri
            && row.raw_value_observed));
        assert_eq!(rows[0].decision.raw_value, "next#part");
        assert_eq!(
            rows[0].decision.resolved_discovered_url.as_deref(),
            Some("https://example.org/next#part")
        );
        assert_eq!(rows[0].admission_reason, "pending_admission");
        assert_eq!(rows[2].admission_reason, "policy_denied");
        assert!(page.policy.as_ref().unwrap().candidates_complete);
        page.pdf.as_mut().unwrap().annotations_complete = false;
        let rows = crawler.page_policy_rows(&mut page, policy, false).unwrap();
        assert!(!page.policy.as_ref().unwrap().candidates_complete);
        assert_eq!(rows[0].admission_reason, "source_incomplete");
        assert_eq!(rows[2].admission_reason, "policy_denied");
    }

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
