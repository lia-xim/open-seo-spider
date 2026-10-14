use crate::{
    ControlledCrawlResult, CrawlConfig, CrawlControl, CrawlEvent, CrawlOutcome, CrawlReport,
    CrawlRequest, CrawlStore, CrawlSummary, PageRecord, PreparedSelectedRecrawl,
    RecrawlResumeOverride, RecrawlTargetObservation, RecrawlTargetState, RedirectHop, RunPlan,
    Severity, audit_pages_with_thresholds, normalize_url,
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
    // Ephemeral construction fact; never persisted as original source evidence.
    automatically_captured_sitemap_inputs: bool,
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
    /// Construct a new crawl from declarations that have not yet been merged.
    pub fn new(config: CrawlConfig) -> anyhow::Result<Self> {
        Self::new_with_input_capture(config, true)
    }

    /// Validate/normalize a saved or inherited configuration while preserving
    /// the declared original-input Option exactly, including legacy None.
    pub fn from_saved_config(config: CrawlConfig) -> anyhow::Result<Self> {
        Self::new_with_input_capture(config, false)
    }

    fn new_with_input_capture(mut config: CrawlConfig, fresh_input: bool) -> anyhow::Result<Self> {
        let automatically_captured_sitemap_inputs =
            fresh_input && config.explicit_sitemap_inputs.is_none();
        // Snapshot before merge, but defer the provisional fresh budget until
        // with_store / with_selected_recrawl has identified the actual mode.
        let original_sitemap_inputs =
            automatically_captured_sitemap_inputs.then(|| config.sitemap_urls.clone());
        config.prepare_input_lists()?;
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
        // Validate the saved source-only work profile before any client/store I/O.
        // Page response limits and retained membership budgets remain independent.
        config.sitemap_source_parser_limits()?;
        config.sitemap_source_body_limit()?;
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
            automatically_captured_sitemap_inputs,
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
        let store = Arc::new(CrawlStore::open(path)?);
        if self.automatically_captured_sitemap_inputs
            && let Some(saved) = store.config()?
        {
            // A stored config identifies resume even with an empty frontier.
            // Only undo this constructor's provisional fresh interpretation.
            // Caller-supplied Some is left intact for Store::prepare to compare.
            self.config.explicit_sitemap_inputs = saved.explicit_sitemap_inputs;
            self.automatically_captured_sitemap_inputs = false;
        }
        self.config.validate_input_provenance_budget()?;
        self.store = Some(store);
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
        if self.automatically_captured_sitemap_inputs {
            self.config.explicit_sitemap_inputs = prepared.config.explicit_sitemap_inputs.clone();
            self.automatically_captured_sitemap_inputs = false;
        }
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
        self.config.validate_input_provenance_budget()?;
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
        if selected.is_none()
            && let Some(generation) = generation
        {
            self.initialize_original_sitemaps(generation)?;
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
            let mut retained = page
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
                if self
                    .config
                    .resource_discovery
                    .as_ref()
                    .is_some_and(|resource| resource.contract.is_some())
                {
                    let facts = page
                        .resource_response
                        .as_ref()
                        .context("Strict response facts missing")?;
                    retained =
                        facts.retained_response || facts.selected_materialization_reason.is_some();
                }
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
        let sitemap_snapshot = self
            .store
            .as_ref()
            .map(|store| store.sitemap_read_snapshot())
            .transpose()?;
        let sitemap_catalog = match &sitemap_snapshot {
            Some(snapshot) => snapshot.catalog.clone(),
            None => match memory_base {
                Some(prepared) => {
                    // Parent validation binds the exact bytes, typed plan and config;
                    // inheritance retains original identities without current discovery.
                    prepared.validate()?;
                    prepared
                        .base_report
                        .sitemap_catalog
                        .as_ref()
                        .map(|catalog| {
                            crate::sitemap_ledger::project_inherited_catalog(
                                catalog,
                                &prepared.plan.parent.snapshot_sha256,
                            )
                        })
                        .transpose()?
                }
                None => None,
            },
        };
        let sitemap_coverage_inventory = match &sitemap_snapshot {
            Some(snapshot) => snapshot.coverage_inventory.clone(),
            None => match memory_base {
                Some(prepared) => prepared
                    .base_report
                    .sitemap_coverage_inventory
                    .clone()
                    .map(|inventory| {
                        inventory.bind_inherited_parent(
                            prepared
                                .base_report
                                .sitemap_catalog
                                .as_ref()
                                .context("Coverage parent catalog missing")?,
                            &prepared.base_snapshot_bytes,
                        )
                    })
                    .transpose()?,
                None => None,
            },
        };
        let sitemap_source_measurements = match &sitemap_snapshot {
            Some(snapshot) => snapshot.sitemap_source_measurements.clone(),
            None => memory_base
                .and_then(|prepared| {
                    prepared
                        .base_report
                        .sitemap_source_measurements
                        .as_ref()
                        .map(|snapshot| {
                            crate::sitemap_source_store::project_inherited_snapshot(
                                snapshot,
                                &prepared.plan.parent.snapshot_sha256,
                            )
                        })
                })
                .transpose()?,
        };
        let report = CrawlReport {
            sitemap_catalog,
            sitemap_coverage_inventory,
            sitemap_source_measurements,
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
        crate::recrawl::validate_portable_report_size(&report)?;
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
        // A late independent role can reuse known source metadata, but it must
        // satisfy the same actual redirect chain; earlier observed grants stay
        // immutable in ResourceResponseFacts rather than being rewritten.
        let eligible_grants = if config.contract.is_some() {
            Some(config.eligible_authority(
                page,
                permission,
                &self.seeds,
                self.config.include_subdomains,
                self.config.check_external,
                false,
            )?)
        } else {
            None
        };
        let retained_response = if config.contract.is_some() {
            let eligible = config.eligible_authority(
                page,
                permission,
                &self.seeds,
                self.config.include_subdomains,
                self.config.check_external,
                true,
            )?;
            permission
                .authorizing_grants
                .iter()
                .any(|grant| grant.store && eligible.contains(&grant.occurrence_id))
        } else {
            permission.store
        };
        let expands = |predicate: fn(&crate::ResourceGrant) -> bool, legacy: bool| {
            eligible_grants.as_ref().map_or(legacy, |eligible| {
                permission.authorizing_grants.iter().any(|grant| {
                    eligible.contains(grant.occurrence_id.as_str()) && predicate(grant)
                })
            })
        };
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
                crate::ResourceSourceKind::NativeCss => {
                    expands(|grant| grant.can_expand_css, permission.can_expand_css)
                }
                crate::ResourceSourceKind::PdfAnnotation
                    if occurrence.reference_kind == crate::ResourceReferenceKind::PdfAnnotation =>
                {
                    expands(|grant| grant.can_expand_pdf, permission.can_expand_pdf)
                }
                _ if occurrence.reference_kind.is_navigation() => expands(
                    |grant| grant.can_expand_navigation,
                    permission.can_expand_navigation,
                ),
                _ => expands(
                    |grant| grant.can_expand_resources,
                    permission.can_expand_resources,
                ),
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
                    target_internal: target
                        .as_ref()
                        .map(|url| same_scope(url, &self.seeds, self.config.include_subdomains)),
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
            facts.retained_response = retained_response;
        }
        if selected && !retained_response && config.contract.is_some() {
            *page = crate::resource_capture::private_projection(page, &self.config, generation)?;
            page.resource_response
                .as_mut()
                .context("Selected projection facts missing")?
                .selected_materialization_reason = Some("selected_scope_limited_projection".into());
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
                self.config
                    .explicit_sitemap_inputs
                    .as_ref()
                    .unwrap_or(&self.config.sitemap_urls),
            ),
        ] {
            let revision = format!("initial:{}:explicit:{}", policy.policy_sha256(), role.key());
            for (index, url) in urls.iter().enumerate() {
                if (role == Role::SitemapDocument && self.config.explicit_sitemap_inputs.is_some())
                    || !covered.contains(&(role.key(), url.clone()))
                {
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

    fn initialize_original_sitemaps(&self, generation: u64) -> anyhow::Result<()> {
        let Some(store) = &self.store else {
            return Ok(());
        };
        let entries = self.discovery_seeds()?;
        if let Some(catalog) = store.sitemap_catalog()? {
            return store.initialize_sitemap_discovery(
                &entries,
                generation,
                &crate::store::SitemapCatalogInit {
                    config_sha256: catalog.summary.config_sha256,
                    limits: catalog.summary.limits,
                    roots: vec![],
                    legacy_history_unknown: catalog.summary.legacy_history_unknown,
                },
            );
        }
        // Legacy saved input absence is unknown. It cannot be reconstructed
        // from normalized or merged task URLs, even when those URLs are empty.
        if self.config.explicit_sitemap_inputs.is_none() {
            return Ok(());
        }
        let config_sha256 = store.saved_config_bytes_sha256()?;
        let original = crate::sitemap_initial_roots::initial_roots(
            self.config.explicit_sitemap_inputs.as_deref(),
            &self.config.input_imports,
            &config_sha256,
            self.policy.as_ref().map(|policy| policy.policy_sha256()),
        )?
        .context("Known initial sitemap provenance unexpectedly absent")?;
        let initial = crate::store::SitemapCatalogInit {
            config_sha256,
            limits: crate::SitemapLedgerLimits::default(),
            roots: original.iter().map(|item| item.root.clone()).collect(),
            legacy_history_unknown: false,
        };
        let sitemap_tasks = entries
            .iter()
            .filter(|entry| entry.kind == DiscoveryKind::Sitemap)
            .map(|entry| entry.url.clone())
            .collect::<Vec<_>>();
        store.initialize_sitemap_discovery_with(
            &entries,
            generation,
            &initial,
            |transaction, roots| {
                let mut items = original;
                anyhow::ensure!(
                    roots.len() == items.len()
                        && roots
                            .iter()
                            .zip(&items)
                            .all(|(root, item)| root == &item.root),
                    "Initializer changed original sitemap root coordinates"
                );
                if self.policy.is_some() {
                    crate::sitemap_initial_roots::finalize_policy_roots(
                        transaction,
                        &mut items,
                        generation,
                    )?;
                } else {
                    crate::sitemap_initial_roots::finalize_ordinary_roots(
                        transaction,
                        &mut items,
                        &sitemap_tasks,
                    )?;
                }
                *roots = items.into_iter().map(|item| item.root).collect();
                Ok(())
            },
        )
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
        // Initial-root/bootstrap owner enables typed mode by committing an honest
        // catalog first. No catalog remains qualified legacy behavior, not empty proof.
        // Actual saved config bytes/current generation/catalog in one read-TX.
        // Catalog.summary.config_sha256 remains the historical initial basis.
        let (sitemap_catalog, capture_basis) =
            sitemap_capture::discovery_basis(self.store.as_deref(), generation)?;
        let observed_discovery = sitemap_catalog.is_some();
        let mut progress = DiscoveryProgress::default();
        let (mut queue, mut discovered, mut completed_sitemaps) = if let Some(store) = &self.store {
            if let Some(catalog) = &sitemap_catalog {
                store.initialize_sitemap_discovery(
                    &seeds,
                    generation.context("Stored discovery needs generation")?,
                    &crate::store::SitemapCatalogInit {
                        config_sha256: catalog.summary.config_sha256.clone(),
                        limits: catalog.summary.limits.clone(),
                        roots: vec![],
                        legacy_history_unknown: catalog.summary.legacy_history_unknown,
                    },
                )?;
            } else {
                store.initialize_discovery(&seeds)?;
            }
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
                output = async {
                    if observed_discovery { self.discover_unit_observed_bound(&entry, &context, &capture_basis).await }
                    else { self.discover_unit(&entry, &context).await }
                } => output?,
                _ = control.cancelled() => break,
            };
            // A is committed before any ordinary/policy finalizer or membership
            // write. B failure must leave this immutable capture visible/unjoined.
            let source_persistence = if observed_discovery && entry.kind == DiscoveryKind::Sitemap {
                let capture = output
                    .source_capture
                    .as_ref()
                    .context("Observed XML source lost actual capture")?;
                let document = &output
                    .sitemap_batch
                    .as_ref()
                    .context("Source capture lost original batch")?
                    .document;
                let store = self.store.as_ref().context("Capture A requires Store")?;
                let generation = generation.context("Capture A requires stored generation")?;
                match &capture.measurement {
                    Ok(measurement) => {
                        SourceCapturePersistence::Receipt(store.record_sitemap_source_capture(
                            &entry,
                            document,
                            measurement,
                            generation,
                        )?)
                    }
                    Err(failure) => {
                        let reason =
                            capture_unavailable_reason(failure, document).with_context(|| {
                                format!("Source capture cannot be durably bound: {failure:?}")
                            })?;
                        match &capture_basis {
                            sitemap_capture::RunBasis::Verified { config_sha256, .. } => {
                                SourceCapturePersistence::Receipt(
                                    store.record_unavailable_sitemap_source_capture(
                                        &entry,
                                        document,
                                        config_sha256,
                                        generation,
                                    )?,
                                )
                            }
                            sitemap_capture::RunBasis::Unavailable(
                                sitemap_capture::CaptureFailure::NoDurableContext
                                | sitemap_capture::CaptureFailure::MissingCurrentConfig
                                | sitemap_capture::CaptureFailure::DirectCallerUnverified,
                            ) => SourceCapturePersistence::NoMeasurement(reason),
                            _ => anyhow::bail!("Capture context failed a durable guard"),
                        }
                    }
                }
            } else {
                SourceCapturePersistence::NotApplicable
            };
            let source_receipt = source_persistence.receipt();
            if self.policy.is_some()
                && let Some(store) = &self.store
            {
                if observed_discovery {
                    let batch = output
                        .sitemap_batch
                        .take()
                        .context("Observed policy source lost whole batch")?;
                    let revision = batch.document.source_revision.clone();
                    let complete = batch.document.complete();
                    (output.pages, output.children) = if let Some(receipt) = source_receipt {
                        store.complete_discovery_with_policy_measured(
                            &entry,
                            &mut output.policy_rows,
                            (&revision, complete),
                            &output.warnings,
                            generation.context("Stored generation absent")?,
                            &self.policy_limits,
                            batch,
                            receipt,
                            finalize_protocol_policy_roots,
                        )?
                    } else {
                        store.complete_discovery_with_policy_observed(
                            &entry,
                            &mut output.policy_rows,
                            (&revision, complete),
                            &output.warnings,
                            generation.context("Stored generation absent")?,
                            &self.policy_limits,
                            batch,
                            finalize_protocol_policy_roots,
                        )?
                    };
                } else {
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
                }
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
            } else if observed_discovery {
                self.finalize_ordinary_protocol(&mut output, page_seen, &discovered)?;
                let batch = output
                    .sitemap_batch
                    .take()
                    .context("Observed ordinary source lost whole batch")?;
                let store = self
                    .store
                    .as_ref()
                    .context("Observed source has no Store")?;
                if let Some(receipt) = source_receipt {
                    store.complete_discovery_measured(
                        &entry,
                        &output.pages,
                        &output.children,
                        &output.warnings,
                        generation.context("Stored generation absent")?,
                        batch,
                        receipt,
                    )?;
                } else {
                    store.complete_discovery_observed(
                        &entry,
                        &output.pages,
                        &output.children,
                        &output.warnings,
                        generation.context("Stored generation absent")?,
                        batch,
                    )?;
                }
                // No global dedupe state leaks from a rolled-back source transaction.
                for page in &output.pages {
                    page_seen.insert(page.url.clone());
                }
                for child in &output.children {
                    discovered.insert(child.clone());
                }
            } else {
                output.pages.retain(|page| {
                    self.filters.allows(&page.url) && page_seen.insert(page.url.clone())
                });
                output
                    .children
                    .retain(|child| discovered.insert(child.clone()));
            }
            if !observed_discovery
                && self.policy.is_none()
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

    async fn discover_unit_observed(
        &self,
        entry: &DiscoveryEntry,
        context: &FetchContext,
    ) -> anyhow::Result<DiscoveryOutput> {
        // Compatibility/direct callers have no verified invocation config basis.
        self.discover_unit_observed_bound(
            entry,
            context,
            &sitemap_capture::RunBasis::Unavailable(
                sitemap_capture::CaptureFailure::DirectCallerUnverified,
            ),
        )
        .await
    }

    async fn discover_unit_observed_bound(
        &self,
        entry: &DiscoveryEntry,
        context: &FetchContext,
        capture_basis: &sitemap_capture::RunBasis,
    ) -> anyhow::Result<DiscoveryOutput> {
        use crate::sitemap_ledger::{SitemapSourceKind as K, SitemapTransportFacts};
        let generation = context
            .generation
            .context("Typed sitemap source needs durable generation")?;
        let revision = format!("protocol:{}:{}", entry.kind.as_str(), entry.url);
        let parsed = normalize_url(&entry.url, None);
        let preflight_failure = match parsed.as_ref() {
            None => Some("sitemap_invalid_task_url"),
            Some(url)
                if self.policy.is_none()
                    && !self.config.check_external
                    && !same_scope(url, &self.seeds, self.config.include_subdomains) =>
            {
                Some("sitemap_outside_scope")
            }
            Some(_) => None,
        };
        if let Some(code) = preflight_failure {
            let batch = match entry.kind {
                DiscoveryKind::Sitemap => crate::sitemap_discovery::xml_batch(
                    &entry.url,
                    generation,
                    &revision,
                    SitemapTransportFacts {
                        request_started: false,
                        requested_url: None,
                        final_url: None,
                        initial_status: None,
                        final_status: None,
                        blocked_by_robots: false,
                        body_complete: false,
                        post_http_decoding_bytes: 0,
                        post_http_decoding_sha256: None,
                        retained_post_http_prefix_sha256: None,
                        failure_code: Some(code.into()),
                    },
                    None,
                    None,
                    Some(code.into()),
                )?,
                DiscoveryKind::Robots => {
                    let requested = parsed
                        .as_ref()
                        .map(|url| format!("{}/robots.txt", origin(url)))
                        .unwrap_or_else(|| entry.url.clone());
                    let source = crate::RobotsSitemapSource::unavailable(
                        requested,
                        false,
                        None,
                        None,
                        code.into(),
                    );
                    // Invalid origin task has no valid request identity. Do not fabricate one.
                    let mut batch = crate::sitemap_discovery::robots_batch(
                        &entry.url, generation, &revision, &source,
                    )?;
                    if parsed.is_none() {
                        batch
                            .document
                            .transport
                            .as_mut()
                            .expect("robots transport")
                            .requested_url = None;
                    }
                    batch
                }
            };
            let capture = if batch.document.kind == K::SitemapXml {
                Some(
                    sitemap_capture::SourceCapture::not_invoked(
                        sitemap_capture::ReceiveProgress::new(
                            FetchPurpose::SitemapProtocol.body_limit(&context.config),
                        ),
                        crate::SitemapSourceMeasurementStop::RequestNotStarted,
                    )
                    .with_limits(context.config.sitemap_source_parser_limits()?)
                    .into_outcome(&batch.document, capture_basis),
                )
            } else {
                None
            };
            let mut output = discovery_from_batch(batch, vec![format!("{code}: {}", entry.url)]);
            output.source_capture = capture;
            return Ok(output);
        }
        let parsed = parsed.expect("checked valid task");
        if entry.kind == DiscoveryKind::Robots {
            let robots = self.robots.get(&parsed).await;
            let unavailable;
            let source = if let Some(source) = &robots.sitemap_source {
                source
            } else {
                unavailable = crate::RobotsSitemapSource::unavailable(
                    format!("{}/robots.txt", origin(&parsed)),
                    false,
                    None,
                    None,
                    "robots_source_observation_unavailable".into(),
                );
                &unavailable
            };
            // Revision uses actual retained source facts, not warning absence or lenient URLs.
            let revision = crate::sitemap_ledger::digest(serde_json::to_vec(&(
                "robots-source-v1",
                &entry.url,
                &source.requested_url,
                &source.final_url,
                source.status,
                &source.retained_prefix_sha256,
                source.retained_post_http_decoding_bytes,
                source.body_complete,
                source.complete,
                &source.failure,
            ))?);
            let batch =
                crate::sitemap_discovery::robots_batch(&entry.url, generation, &revision, source)?;
            let mut warnings = Vec::new();
            if let Some(warning) = &robots.warning {
                warnings.push(warning.clone());
            }
            if !batch.document.complete() {
                warnings.push(format!(
                    "Robots sitemap declarations incomplete: {}",
                    entry.url
                ));
            }
            let mut output = discovery_from_batch(batch, warnings);
            self.protocol_policy_rows(&mut output, false)?;
            return Ok(output);
        }
        let (page, mut protocol) = context
            .sitemap_protocol(QueueEntry {
                url: entry.url.clone(),
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
        let revision = protocol
            .entity_decode
            .as_ref()
            .map(|decoded| {
                crate::sitemap_ledger::digest(
                    serde_json::to_vec(&(
                        "sitemap-decoded-xml-source-v1",
                        &entry.url,
                        &decoded.decoded_xml_sha256,
                    ))
                    .expect("fixed typed source tuple"),
                )
            })
            .unwrap_or_else(|| {
                crate::sitemap_ledger::digest(
                    serde_json::to_vec(&(
                        "sitemap-source-failure-v1",
                        &entry.url,
                        protocol.request_started,
                        &protocol.requested_url,
                        &protocol.final_url,
                        protocol.final_status,
                        protocol.retained_post_http_entity_bytes,
                        &protocol.post_http_entity_sha256,
                        &protocol.failure,
                    ))
                    .expect("fixed typed source tuple"),
                )
            });
        let transport = SitemapTransportFacts {
            request_started: protocol.request_started,
            requested_url: protocol.requested_url.clone(),
            // Base PageRecord may hold an unfetched redirect target when status
            // is unavailable. Do not relabel that proposed target a final response.
            final_url: protocol.final_status.and(protocol.final_url.clone()),
            initial_status: protocol.initial_status,
            final_status: protocol.final_status,
            blocked_by_robots: protocol.blocked_by_robots,
            body_complete: protocol.body_complete,
            post_http_decoding_bytes: u64::try_from(protocol.retained_post_http_entity_bytes)?,
            post_http_decoding_sha256: protocol.post_http_entity_sha256.clone(),
            // Incomplete entity prefixes are not hashed by the current private capture.
            // Copy only the actual complete hash; never invent a prefix digest.
            retained_post_http_prefix_sha256: protocol.post_http_entity_sha256.clone(),
            failure_code: protocol
                .failure
                .as_ref()
                .map(|failure| failure.code.clone()),
        };
        let batch = crate::sitemap_discovery::xml_batch(
            &entry.url,
            generation,
            &revision,
            transport,
            protocol.entity_decode.as_ref(),
            protocol.parser.take(),
            protocol
                .failure
                .as_ref()
                .map(|failure| failure.code.clone()),
        )?;
        let mut warnings = Vec::new();
        if !batch.document.complete() {
            warnings.push(format!(
                "Sitemap could not be read completely: {}: {}",
                entry.url,
                protocol
                    .failure
                    .as_ref()
                    .map(|failure| failure.code.clone())
                    .or(page.error.clone())
                    .unwrap_or_else(|| format!("status {:?}", page.final_status))
            ));
        }
        let xml_allowed = self.policy.is_none()
            || page.policy.as_ref().is_some_and(|observed| {
                observed.admission.can_discover_xml
                    && observed
                        .redirect_decisions
                        .iter()
                        .all(|row| row.decision.folder_internal)
            });
        let capture = protocol
            .source_capture
            .clone()
            .into_outcome(&batch.document, capture_basis);
        let mut output = discovery_from_batch(batch, warnings);
        output.source_capture = Some(capture);
        if let Some(observed) = &page.policy {
            anyhow::ensure!(
                observed.redirect_decisions.iter().all(|row| row.task_role
                    == crate::PolicyTaskRole::ObservationOnly
                    && row.scheduled_fetch_url.is_none()),
                "Protocol redirect observation must not schedule another task"
            );
            output
                .policy_rows
                .extend(observed.redirect_decisions.clone());
        }
        output.protocol_observation = Some(protocol);
        self.protocol_policy_rows(&mut output, xml_allowed)?;
        anyhow::ensure!(
            output
                .sitemap_batch
                .as_ref()
                .is_some_and(|batch| batch.document.kind == K::SitemapXml),
            "Typed XML source lost its whole batch"
        );
        Ok(output)
    }

    fn protocol_policy_rows(
        &self,
        output: &mut DiscoveryOutput,
        xml_allowed: bool,
    ) -> anyhow::Result<()> {
        use crate::sitemap_ledger::SitemapSourceKind as K;
        use crate::{CandidateSourceKind as C, PolicyInputBasis as B, PolicyTaskRole as R};
        let Some(policy) = &self.policy else {
            return Ok(());
        };
        let batch = output
            .sitemap_batch
            .as_ref()
            .context("Typed discovery has no source batch")?;
        if !batch.document.complete() {
            return Ok(());
        }
        match batch.document.kind {
            K::RobotsDirectives => {
                for root in &batch.roots {
                    if root.normalized_document_url.is_none() {
                        continue;
                    }
                    output.policy_rows.push(crate::policy_ledger::decide(
                        policy,
                        &batch.document.source_revision,
                        usize::try_from(root.source_ordinal)?,
                        C::SitemapDocument,
                        root.raw_value.clone(),
                        None,
                        Some(batch.document.task_url.clone()),
                        B::DeclaredInput,
                        R::SitemapDocument,
                    ));
                }
            }
            K::SitemapXml => {
                for location in &batch.locations {
                    if location.observed.loc_ordinal.is_none() {
                        continue;
                    }
                    let (kind, role) = match location.observed.role {
                        crate::SitemapLocationRole::PageLocation => {
                            (C::SitemapLocation, R::PageDocument)
                        }
                        crate::SitemapLocationRole::ChildSitemap => {
                            (C::SitemapDocument, R::SitemapDocument)
                        }
                    };
                    let mut row = crate::policy_ledger::decide(
                        policy,
                        &batch.document.source_revision,
                        location.observed.ordinal,
                        kind,
                        location
                            .observed
                            .decoded_value
                            .clone()
                            .context("Observed original Loc has no decoded value")?,
                        None,
                        Some(batch.document.task_url.clone()),
                        B::ParserObservedXml,
                        role,
                    );
                    if location.observed.validation_error.is_some()
                        || location.observed.normalized_fetch_url.is_none()
                    {
                        // Preserve the policy evaluation even when the strict XML
                        // parser rejected an otherwise leniently normalized URL.
                        // The original parser rejection prevents any admission.
                        row.input_error =
                            location.observed.validation_error.clone().or_else(|| {
                                Some("Parser supplied no accepted normalized URL".into())
                            });
                        if row.admission_reason == "pending_admission" {
                            row.admission_reason = "input_row_rejected".into();
                        }
                    } else if !xml_allowed && row.admission_reason == "pending_admission" {
                        row.admission_reason = "source_check_only".into();
                    }
                    output.policy_rows.push(row);
                }
            }
        }
        Ok(())
    }

    /// Every final ordinary decision is assigned to its ORIGINAL occurrence.
    /// Stage only new unit targets; global sets change only after Store commit.
    fn finalize_ordinary_protocol(
        &self,
        output: &mut DiscoveryOutput,
        page_seen: &HashSet<String>,
        discovered: &HashSet<DiscoveryEntry>,
    ) -> anyhow::Result<()> {
        use crate::sitemap_ledger::SitemapAdmissionDecision as D;
        let batch = output
            .sitemap_batch
            .as_mut()
            .context("Typed discovery has no batch")?;
        if !batch.document.complete() {
            return Ok(());
        }
        output.pages.clear();
        output.children.clear();
        // check_external permits checking a source, not recursively expanding
        // another origin. Preserve ordinary sitemap target scope and require
        // both original task and actual final response to be internal sources.
        let source_internal = normalize_url(&batch.document.task_url, None)
            .is_some_and(|url| same_scope(&url, &self.seeds, self.config.include_subdomains))
            && batch
                .document
                .transport
                .as_ref()
                .and_then(|transport| transport.final_url.as_deref())
                .and_then(|url| normalize_url(url, None))
                .is_some_and(|url| same_scope(&url, &self.seeds, self.config.include_subdomains));
        if !source_internal {
            for root in &mut batch.roots {
                if root.normalized_document_url.is_some() {
                    root.admission.decision = D::SourceCheckOnly;
                }
            }
            for location in &mut batch.locations {
                if location.observed.loc_ordinal.is_some()
                    && location.observed.validation_error.is_none()
                    && location.observed.normalized_fetch_url.is_some()
                {
                    location.admission.decision = D::SourceCheckOnly;
                }
            }
            return Ok(());
        }
        let mut new_pages = HashSet::new();
        let mut new_children = HashSet::new();
        for root in &mut batch.roots {
            let Some(url) = &root.normalized_document_url else {
                continue;
            };
            let parsed = normalize_url(url, None).context("Strict root normalized URL invalid")?;
            if !same_scope(&parsed, &self.seeds, self.config.include_subdomains) {
                root.admission.decision = D::OutsideScope;
                continue;
            }
            let child = DiscoveryEntry {
                kind: DiscoveryKind::Sitemap,
                url: url.clone(),
            };
            root.admission.decision =
                if !discovered.contains(&child) && new_children.insert(child.clone()) {
                    output.children.push(child);
                    D::Scheduled
                } else {
                    D::Coalesced
                };
            root.admission.admitted_fetch_url = Some(url.clone());
        }
        for location in &mut batch.locations {
            if location.observed.loc_ordinal.is_none()
                || location.observed.validation_error.is_some()
            {
                continue;
            }
            let Some(url) = &location.observed.normalized_fetch_url else {
                continue;
            };
            let parsed = normalize_url(url, None).context("Strict Loc normalized URL invalid")?;
            if !same_scope(&parsed, &self.seeds, self.config.include_subdomains) {
                location.admission.decision = D::OutsideScope;
                continue;
            }
            match location.observed.role {
                crate::SitemapLocationRole::PageLocation => {
                    if !self.filters.allows(url) {
                        location.admission.decision = D::FilterDenied;
                        continue;
                    }
                    location.admission.decision =
                        if !page_seen.contains(url) && new_pages.insert(url.clone()) {
                            output.pages.push(QueueEntry {
                                url: url.clone(),
                                depth: 0,
                                discovered_from: Some(batch.document.task_url.clone()),
                                admission: None,
                            });
                            D::Scheduled
                        } else {
                            D::Coalesced
                        };
                    location.admission.admitted_fetch_url = Some(url.clone());
                }
                crate::SitemapLocationRole::ChildSitemap => {
                    let child = DiscoveryEntry {
                        kind: DiscoveryKind::Sitemap,
                        url: url.clone(),
                    };
                    location.admission.decision =
                        if !discovered.contains(&child) && new_children.insert(child.clone()) {
                            output.children.push(child);
                            D::Scheduled
                        } else {
                            D::Coalesced
                        };
                    location.admission.admitted_fetch_url = Some(url.clone());
                    location.referenced_sitemap_task = Some(url.clone());
                }
            }
        }
        Ok(())
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
        let (page, protocol) = context
            .sitemap_protocol(QueueEntry {
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
        // Retain complete transport/decoder facts and an incomplete typed XML
        // prefix even when the unchanged compatibility admission path rejects it.
        output.protocol_observation = Some(protocol);
        let protocol = output
            .protocol_observation
            .as_ref()
            .expect("protocol observation");
        if page.error.is_some()
            || page.final_status != Some(200)
            || page.truncated
            || protocol.failure.is_some()
            || !protocol
                .parser
                .as_ref()
                .is_some_and(|parser| parser.complete)
        {
            output.warnings.push(format!(
                "Sitemap could not be read completely: {}: {}",
                entry.url,
                protocol
                    .failure
                    .as_ref()
                    .map(|failure| format!("{}: {}", failure.code, failure.message))
                    .or(page.error)
                    .unwrap_or_else(|| format!("status {:?}", page.final_status)),
            ));
            return Ok(output);
        }
        let Some(body) = protocol
            .entity_decode
            .as_ref()
            .map(|decoded| decoded.xml.as_slice())
        else {
            return Ok(output);
        };
        // Typed parser already verified UTF8; this exact slice only binds the
        // existing source revision. No second XML parse or charset guessing.
        let body = match std::str::from_utf8(body) {
            Ok(body) => body,
            Err(_) => {
                output
                    .warnings
                    .push(format!("Invalid sitemap UTF8 at {}", entry.url));
                return Ok(output);
            }
        };
        let parser = protocol.parser.as_ref().expect("complete typed parser");
        let is_index = parser.kind == Some(crate::SitemapDocumentKind::SitemapIndex);
        output.policy_revision = Some(crate::policy_ledger::digest(format!(
            "sitemap:{}:{body}",
            entry.url
        )));
        output.policy_complete = true;
        // Exactly the legacy parse_sitemap projection: decoded values, Unicode
        // trim, drop blank/missing, compact enumeration. This is NOT the original
        // typed Loc ordinal/raw-value contract required by the future ledger.
        for (index, loc) in parser
            .locations
            .iter()
            .filter_map(|row| row.decoded_value.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .enumerate()
        {
            let loc = loc.to_owned();
            if let Some(policy) = &self.policy {
                let role = if is_index {
                    crate::PolicyTaskRole::SitemapDocument
                } else {
                    crate::PolicyTaskRole::PageDocument
                };
                let kind = if is_index {
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
            if is_index {
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
        Ok(output)
    }
}

/// Optional measurement profiling must not convert honest bounded source
/// failures into a different crawl admission rule. NoMeasurement has no receipt
/// and creates no commit association; absence remains unknown to durable reads.
enum SourceCapturePersistence {
    NotApplicable,
    NoMeasurement(SourceCaptureUnavailable),
    Receipt(crate::sitemap_source_store::SourceMeasurementReceipt),
}
impl SourceCapturePersistence {
    fn receipt(&self) -> Option<&crate::sitemap_source_store::SourceMeasurementReceipt> {
        match self {
            Self::Receipt(receipt) => Some(receipt),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceCaptureUnavailable {
    Context,
    IdentityBytes,
    RecordBytes,
    ReceiveCounter,
}
fn capture_unavailable_reason(
    failure: &sitemap_capture::CaptureFailure,
    document: &crate::SitemapSourceDocument,
) -> Option<SourceCaptureUnavailable> {
    use crate::SitemapSourceMeasurementFailureCode as F;
    use sitemap_capture::CaptureFailure as C;
    match failure {
        C::NoDurableContext | C::MissingCurrentConfig | C::DirectCallerUnverified => {
            Some(SourceCaptureUnavailable::Context)
        }
        C::ReceiveCounterUnavailable => Some(SourceCaptureUnavailable::ReceiveCounter),
        C::Measurement(failure) if failure.code == F::JsonBytesLimit => {
            Some(SourceCaptureUnavailable::RecordBytes)
        }
        C::Measurement(failure) if failure.code == F::InvalidIdentity => {
            let cap = crate::SitemapSourceMeasurementLimits::default().max_identity_utf8_bytes;
            let exceeds = document.task_url.len() > cap
                || document.source_revision.len() > cap
                || document.transport.as_ref().is_some_and(|transport| {
                    [&transport.requested_url, &transport.final_url]
                        .into_iter()
                        .flatten()
                        .any(|value| value.len() > cap)
                });
            exceeds.then_some(SourceCaptureUnavailable::IdentityBytes)
        }
        // Invalid bindings, identities without actual size excess, unknown
        // helper profiles, config/generation mismatch and SQL errors are errors.
        _ => None,
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
    // Work integration seam: full ledger must consume this whole unit atomically.
    protocol_observation: Option<SitemapProtocolObservation>,
    sitemap_batch: Option<crate::SitemapDiscoveryBatch>,
    // In-flight captured evidence, never a persisted receipt or source admission.
    source_capture: Option<sitemap_capture::CaptureOutcome>,
}

#[derive(Default)]
struct DiscoveryProgress {
    pages: Vec<QueueEntry>,
    warnings: Vec<String>,
    pending: usize,
    limited: bool,
}

fn discovery_from_batch(
    batch: crate::SitemapDiscoveryBatch,
    warnings: Vec<String>,
) -> DiscoveryOutput {
    DiscoveryOutput {
        policy_revision: Some(batch.document.source_revision.clone()),
        policy_complete: batch.document.complete(),
        sitemap_batch: Some(batch),
        warnings,
        ..Default::default()
    }
}

fn finalize_protocol_policy_roots(
    transaction: &rusqlite::Transaction<'_>,
    batch: &mut crate::SitemapDiscoveryBatch,
    rows: &[crate::PolicyOccurrence],
) -> anyhow::Result<()> {
    use crate::sitemap_ledger::SitemapSourceKind as K;
    if batch.document.kind != K::RobotsDirectives {
        return Ok(());
    }
    if !batch.document.complete() {
        anyhow::ensure!(
            rows.iter()
                .all(|row| row.input_basis != crate::PolicyInputBasis::DeclaredInput),
            "Incomplete robots source has scheduling inputs"
        );
        return Ok(());
    }
    let observed: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.input_basis == crate::PolicyInputBasis::DeclaredInput
                && row.source_revision == batch.document.source_revision
        })
        .collect();
    let mut bound = HashSet::new();
    for root in &mut batch.roots {
        if root.normalized_document_url.is_none() {
            continue;
        }
        let ordinal = usize::try_from(root.source_ordinal)?;
        let matching: Vec<_> = observed
            .iter()
            .filter(|row| row.source_ordinal == ordinal)
            .collect();
        anyhow::ensure!(
            matching.len() == 1,
            "Missing/duplicate final robots directive policy decision"
        );
        let row = *matching[0];
        crate::sitemap_discovery::finalize_root_policy(
            transaction,
            root,
            row,
            &crate::sitemap_discovery::RootPolicyContext {
                source_revision: &batch.document.source_revision,
                source_ordinal: ordinal,
                generation: batch.document.generation,
                source_url: Some(&batch.document.task_url),
            },
        )?;
        anyhow::ensure!(
            bound.insert(ordinal),
            "Duplicate final robots directive ordinal"
        );
    }
    anyhow::ensure!(
        bound.len() == observed.len(),
        "Unexpected robots policy rows not represented by valid raw directives"
    );
    Ok(())
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

// WORK draft: insert before FetchContext in crawler.rs. Private implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FetchPurpose {
    PageDocument,
    SitemapProtocol,
}

impl FetchPurpose {
    fn policy_role(self) -> crate::PolicyTaskRole {
        match self {
            Self::PageDocument => crate::PolicyTaskRole::PageDocument,
            Self::SitemapProtocol => crate::PolicyTaskRole::SitemapDocument,
        }
    }

    fn requires_page_resource_permission(self) -> bool {
        self == Self::PageDocument
    }

    fn body_limit(self, config: &CrawlConfig) -> usize {
        match self {
            Self::PageDocument => config.max_body_bytes,
            Self::SitemapProtocol => config
                .sitemap_source_body_limit()
                .expect("source processing profile was validated before fetch"),
        }
    }
}

enum FetchPayload {
    PageText(String),
    SitemapProtocol(Box<SitemapProtocolObservation>),
}

/// Private staging value, not a report or persistence contract. Serialize allows
/// Root's proposed ledger adapter to project the retained bounded facts explicitly.
#[derive(Debug, serde::Serialize)]
struct SitemapProtocolObservation {
    request_started: bool,
    /// Oversized context is unavailable rather than a fabricated truncated URL.
    requested_url: Option<String>,
    final_url: Option<String>,
    initial_status: Option<u16>,
    final_status: Option<u16>,
    blocked_by_robots: bool,
    truncated: bool,
    redirect_count: usize,
    body_complete: bool,
    /// Retained bytes AFTER reqwest Content-Decoding, never compressed wire bytes.
    /// On a truncated body this is a prefix count, not the full response size.
    retained_post_http_entity_bytes: usize,
    /// Only a complete actual entity is hashed; corrupt file-gzip can still have
    /// a complete transport entity with a failing entity decoder.
    post_http_entity_sha256: Option<String>,
    entity_decode: Option<crate::SitemapEntityDecode>,
    /// A late XML failure retains its typed bounded prefix and failure together.
    parser: Option<crate::SitemapParseObservation>,
    failure: Option<crate::SitemapFailure>,
    // Compact private capture is not part of the existing legacy serialization.
    #[serde(skip_serializing)]
    source_capture: sitemap_capture::SourceCapture,
}

impl SitemapProtocolObservation {
    fn capture(page: &PageRecord, request_started: bool, body: &[u8]) -> Self {
        // Compatibility supplied-slice caller. The actual fetch passes the
        // independently counted response.chunk progress to the configured capture.
        let mut progress = sitemap_capture::ReceiveProgress::new(
            crate::SitemapParserLimits::default().max_entity_bytes,
        );
        if request_started {
            progress.begin_body();
            progress.returned(body.len());
        }
        Self::capture_with_progress(page, request_started, body, progress)
    }

    fn capture_with_progress(
        page: &PageRecord,
        request_started: bool,
        body: &[u8],
        progress: sitemap_capture::ReceiveProgress,
    ) -> Self {
        Self::capture_with_limits(
            page,
            request_started,
            body,
            progress,
            crate::SitemapParserLimits::default(),
        )
    }

    fn capture_for_config(
        page: &PageRecord,
        request_started: bool,
        body: &[u8],
        progress: sitemap_capture::ReceiveProgress,
        config: &CrawlConfig,
    ) -> Self {
        let limits = config
            .sitemap_source_parser_limits()
            .expect("source processing profile was validated before fetch");
        if config.sitemap_source_processing.is_none() {
            Self::capture_with_progress(page, request_started, body, progress)
        } else {
            Self::capture_with_limits(page, request_started, body, progress, limits)
        }
    }

    fn capture_with_limits(
        page: &PageRecord,
        request_started: bool,
        body: &[u8],
        progress: sitemap_capture::ReceiveProgress,
        limits: crate::SitemapParserLimits,
    ) -> Self {
        let body_complete = page.final_status.is_some()
            && !matches!(page.final_status, Some(301 | 302 | 303 | 307 | 308))
            && page.error.is_none()
            && !page.truncated
            && !page.blocked_by_robots;
        let mut observed = Self {
            request_started,
            requested_url: (page.url.len() <= limits.max_field_utf8_bytes)
                .then(|| page.url.clone()),
            final_url: (page.final_url.len() <= limits.max_field_utf8_bytes)
                .then(|| page.final_url.clone()),
            initial_status: page.status,
            final_status: page.final_status,
            blocked_by_robots: page.blocked_by_robots,
            truncated: page.truncated,
            redirect_count: page.redirect_chain.len(),
            body_complete,
            retained_post_http_entity_bytes: body.len(),
            post_http_entity_sha256: body_complete.then(|| crate::resource_policy::digest(body)),
            entity_decode: None,
            parser: None,
            failure: None,
            source_capture: sitemap_capture::SourceCapture::not_invoked(
                progress,
                crate::SitemapSourceMeasurementStop::ContextUnavailable,
            )
            .with_limits(limits.clone()),
        };
        if request_started && observed.source_capture.received.received_bytes.is_some() {
            observed.source_capture.retained_prefix_sha256 = Some(
                observed
                    .post_http_entity_sha256
                    .clone()
                    .unwrap_or_else(|| crate::resource_policy::digest(body)),
            );
        }
        if observed.requested_url.is_none() || observed.final_url.is_none() {
            observed.failure = Some(crate::SitemapFailure::new(
                "sitemap_transport_context_byte_budget",
                "Exact protocol URL context exceeds the bounded field profile",
                None,
            ));
            return observed;
        }
        if !body_complete || page.final_status != Some(200) {
            use crate::SitemapSourceMeasurementStop as S;
            let stop = if page.blocked_by_robots {
                S::RobotsDenied
            } else if !request_started {
                S::RequestNotStarted
            } else if page.truncated {
                S::EntityByteLimit
            } else if !body_complete {
                S::TransportIncomplete
            } else {
                S::HttpStatusNot200
            };
            observed.source_capture.unobserved_stop = stop;
            observed.source_capture.count = sitemap_capture::CountCapture::NotInvoked(stop);
            let (code, message) = if page.blocked_by_robots {
                (
                    "sitemap_robots_denied",
                    "Sitemap request was blocked by robots",
                )
            } else if page.truncated {
                (
                    "sitemap_transport_body_byte_budget",
                    "Sitemap entity did not fit the post-HTTP byte cap",
                )
            } else if !request_started {
                (
                    "sitemap_request_not_started",
                    "Sitemap request did not pass native admission",
                )
            } else if page.error.is_some() {
                (
                    "sitemap_transport_incomplete",
                    "Sitemap transport did not complete",
                )
            } else {
                (
                    "sitemap_http_non200",
                    "Sitemap did not end with a complete HTTP 200 body",
                )
            };
            observed.failure = Some(crate::SitemapFailure::new(code, message, None));
            return observed;
        }
        let outcome = crate::decode_sitemap_entity_observed(body, &limits);
        observed.source_capture.decoder = Some(outcome.observation);
        match outcome.result {
            Ok(entity) => {
                // Same successful decoded Vec; independent of legacy retained
                // parser outcome. Count failures never affect legacy failure/ID.
                observed.source_capture.count = sitemap_capture::CountCapture::Observed(
                    crate::count_sitemap_xml(&entity.xml, &observed.source_capture.count_limits()),
                );
                let context = crate::SitemapParserContext {
                    requested_document_url: Some(page.url.clone()),
                    final_document_url: Some(page.final_url.clone()),
                    // A coalesced task may have several declaring parents. The
                    // occurrence ledger must bind every edge; do not invent one.
                    originating_occurrence_id: None,
                };
                match crate::observe_sitemap_xml(&entity.xml, context, &limits) {
                    Ok(parser) => {
                        observed.failure = parser.failure.clone();
                        observed.parser = Some(parser);
                    }
                    Err(failure) => observed.failure = Some(failure),
                }
                observed.entity_decode = Some(entity);
            }
            Err(failure) => {
                observed.source_capture.count = sitemap_capture::CountCapture::NotInvoked(
                    sitemap_capture::decode_stop(&failure.code)
                        .unwrap_or(crate::SitemapSourceMeasurementStop::ContextUnavailable),
                );
                observed.failure = Some(failure);
            }
        }
        observed
    }
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
        let (page, payload) = self.fetch(entry, FetchPurpose::PageDocument).await;
        match payload {
            Some(FetchPayload::PageText(text)) => (page, Some(text)),
            None => (page, None),
            Some(FetchPayload::SitemapProtocol(_)) => unreachable!("private page fetch purpose"),
        }
    }

    async fn sitemap_protocol(
        &self,
        entry: QueueEntry,
    ) -> (PageRecord, SitemapProtocolObservation) {
        let (page, payload) = self.fetch(entry, FetchPurpose::SitemapProtocol).await;
        let observed = match payload {
            Some(FetchPayload::SitemapProtocol(observed)) => *observed,
            // Existing early returns precede any send(). Keep an explicit
            // failure envelope even when policy/resource/URL admission failed.
            None => SitemapProtocolObservation::capture_for_config(
                &page,
                false,
                &[],
                sitemap_capture::ReceiveProgress::new(
                    FetchPurpose::SitemapProtocol.body_limit(&self.config),
                ),
                &self.config,
            ),
            Some(FetchPayload::PageText(_)) => unreachable!("private sitemap fetch purpose"),
        };
        (page, observed)
    }

    async fn fetch(
        &self,
        entry: QueueEntry,
        purpose: FetchPurpose,
    ) -> (PageRecord, Option<FetchPayload>) {
        let mut page = PageRecord {
            url: entry.url.clone(),
            final_url: entry.url.clone(),
            depth: entry.depth,
            discovered_from: entry.discovered_from,
            fetched_at: Utc::now().to_rfc3339(),
            indexability_agent: self.config.indexability_agent.clone(),
            ..Default::default()
        };
        // Purpose comes only from private document/protocol wrappers. A supplied
        // policy role cannot select a different transport privilege.
        if entry
            .admission
            .as_ref()
            .is_some_and(|admission| admission.task_role != purpose.policy_role())
        {
            page.error = Some(
                "Native admission role differs from fetch purpose; request was not made".into(),
            );
            if let Some(capture) = &self.capture {
                crate::content_binding::unavailable_sources(
                    &mut page,
                    capture,
                    self.generation,
                    "native_admission_role_mismatch",
                );
            }
            return (page, None);
        }
        if self.config.resource_discovery.is_some() && purpose.requires_page_resource_permission() {
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
                admission.validate(policy.policy_sha256(), &page.url, purpose.policy_role())
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
        let mut request_started = false;
        let body_limit = purpose.body_limit(&self.config);
        let mut sitemap_received = sitemap_capture::ReceiveProgress::new(body_limit);
        let mut resource_redirects: Vec<crate::ResourceRedirectDecision> = Vec::new();
        loop {
            if self.config.obey_robots {
                let policy = self.robots.get(&current).await;
                let allowed = policy.allowed(&self.config.user_agent, current.as_str());
                if let Some(hop) = resource_redirects.last_mut()
                    && hop.target_url.as_deref() == Some(current.as_str())
                {
                    hop.robots_allowed = Some(allowed);
                    if !allowed {
                        hop.allowed = false;
                        hop.reason = "redirect_robots_denied".into();
                    }
                }
                if !allowed {
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
            if let Some(hop) = resource_redirects.last_mut()
                && hop.target_url.as_deref() == Some(current.as_str())
            {
                hop.request_started = true;
            }
            request_started = true;
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
                let target = location.and_then(|location| normalize_url(location, Some(&current)));
                let strict_redirect = if let (Some(resource), Some(permission), Some(raw)) = (
                    &self.config.resource_discovery,
                    &page.resource_permission,
                    location,
                ) && resource.contract.is_some()
                {
                    let policy = redirect_decision.map(|index| {
                        let decision = &page
                            .policy
                            .as_ref()
                            .expect("observed policy")
                            .redirect_decisions[index]
                            .decision;
                        (
                            decision.decision != crate::CrawlPolicyDecisionKind::Deny,
                            decision.traverse,
                        )
                    });
                    match resource.redirect_decision(
                        permission,
                        &current,
                        raw,
                        target.as_ref(),
                        &self.seeds,
                        self.config.include_subdomains,
                        self.config.check_external,
                        policy,
                        &resource_redirects,
                    ) {
                        Ok(decision) => {
                            let index = resource_redirects.len();
                            resource_redirects.push(decision);
                            Some(index)
                        }
                        Err(error) => {
                            page.error =
                                Some(format!("Invalid resource redirect admission: {error}"));
                            break;
                        }
                    }
                } else {
                    None
                };
                let Some(target) = target else {
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
                    if let Some(index) = strict_redirect {
                        resource_redirects[index].allowed = false;
                        resource_redirects[index].reason = "redirect_stopped_loop".into();
                    }
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
                    if let Some(index) = strict_redirect {
                        resource_redirects[index].allowed = false;
                        resource_redirects[index].reason = "redirect_stopped_limit".into();
                    }
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
                if strict_redirect.is_some_and(|index| !resource_redirects[index].allowed) {
                    page.warnings.push("Redirect target denied by immutable resource role/scope; no target request was made".into());
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
                if strict_redirect.is_none()
                    && self.policy.is_none()
                    && !same_scope(&target, &self.seeds, self.config.include_subdomains)
                    && !self.config.check_external
                    && !(purpose.requires_page_resource_permission()
                        && self
                            .config
                            .resource_discovery
                            .as_ref()
                            .is_some_and(|config| {
                                config.permits_resource_origin(
                                    &target,
                                    &self.seeds,
                                    self.config.include_subdomains,
                                )
                            }))
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
            if purpose == FetchPurpose::SitemapProtocol {
                sitemap_received.begin_body();
            }
            loop {
                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        if purpose == FetchPurpose::SitemapProtocol {
                            sitemap_received.returned(chunk.len());
                        }
                        let remaining = body_limit - body.len();
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
        if purpose == FetchPurpose::SitemapProtocol {
            // response.chunk() already performed reqwest HTTP Content-Decoding.
            // File-gzip magic is inspected exactly once here, before page sniffing.
            let observed = SitemapProtocolObservation::capture_for_config(
                &page,
                request_started,
                &body,
                sitemap_received,
                &self.config,
            );
            return (
                page,
                Some(FetchPayload::SitemapProtocol(Box::new(observed))),
            );
        }
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
                contract_engine: self
                    .config
                    .resource_discovery
                    .as_ref()
                    .and_then(|config| config.contract.as_ref())
                    .map(|contract| contract.engine.clone()),
                redirect_decisions: resource_redirects,
                selected_materialization_reason: None,
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
        (page, text.map(FetchPayload::PageText))
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

#[cfg(test)]
#[path = "crawler/sitemap_protocol_tests.rs"]
mod sitemap_protocol_tests;

#[cfg(test)]
#[path = "crawler/discovery_ledger_tests.rs"]
mod discovery_ledger_tests;

#[cfg(test)]
#[path = "crawler/bootstrap_tests.rs"]
mod bootstrap_ledger_tests;

#[path = "crawler/sitemap_capture.rs"]
mod sitemap_capture;

#[cfg(test)]
#[path = "crawler/sitemap_capture_tests.rs"]
mod sitemap_capture_tests;

/// Stored-observation Store-test support, not a native HTTP assertion. Reuses
/// the actual collector/decoder/scanner rather than a forged measurement DTO.
#[cfg(test)]
pub(crate) fn test_sitemap_measurement_from_stored_transport(
    document: &crate::SitemapSourceDocument,
    body: &[u8],
    config_sha256: String,
    body_limit_bytes: usize,
) -> crate::SitemapSourceMeasurement {
    let page = PageRecord {
        url: document.task_url.clone(),
        final_url: document.task_url.clone(),
        status: Some(200),
        final_status: Some(200),
        ..Default::default()
    };
    let mut progress = sitemap_capture::ReceiveProgress::new(body_limit_bytes);
    progress.begin_body();
    progress.returned(body.len());
    SitemapProtocolObservation::capture_with_progress(&page, true, body, progress)
        .source_capture
        .into_outcome(
            document,
            &sitemap_capture::RunBasis::Verified {
                config_sha256,
                generation: document.generation,
            },
        )
        .measurement
        .expect("actual helper/scanner stored-observation binding")
}
