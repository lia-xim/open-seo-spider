use axum::{
    Json, Router,
    extract::{Path, Query, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use open_seo_spider_core::{
    AuditIssue, ContentAnalysisInput, ContentAnalysisResult, CrawlCheckpoint, CrawlConfig,
    CrawlControl, CrawlEvent, CrawlOutcome, CrawlPolicyPreviewInput, CrawlReport, CrawlStore,
    Crawler, ListImportLimits, ListImportOptions, PageRecord, RecrawlResumeOverride,
    RecrawlSourceKind, RobotsSimulationInput, RunPlan, SelectedRecrawlOperation,
    SelectedRecrawlParent, SerpInputRow, analyze_content, compare_crawls,
    export_browser_cookies_csv, export_custom_extractions_csv, export_custom_javascript_csv,
    export_dom_observations_csv, export_fragments_csv, export_image_resources_csv,
    export_input_occurrences_csv, export_issues_csv, export_links_csv, export_pages_csv,
    export_pdf_csv, export_response_observations_csv, export_robots_simulation_csv,
    export_serp_csv, export_sitemap_xml, import_serp_csv, import_url_list, inspect_xlsx_list,
    prepare_selected_recrawl, preview_crawl_policy, preview_serp, response_observations,
    simulate_robots, validate_fragments,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;
mod broken_sources;
mod comparison;
mod content;
mod directory_tree;
mod lifecycle;
mod migration;
mod ngram_corpus;
mod policy;
mod resource_broken_sources;
pub(crate) mod resources;
mod response_stream;
mod sitemap_csv_stream;
mod sitemap_findings;
mod sitemap_findings_reads;
mod sitemap_generation;
mod sitemaps;
mod source_bodies;
mod source_body_stream;
mod transport;

#[cfg(test)]
#[path = "server/sitemap_processing_config_tests.rs"]
mod sitemap_processing_config_tests;

#[derive(Clone)]
struct AppState {
    mcp: crate::mcp::Bridge,
    jobs: Arc<RwLock<HashMap<Uuid, Arc<RwLock<Job>>>>>,
    data_dir: Arc<PathBuf>,
    exports: sitemap_csv_stream::ExportRegistry,
    finding_reads: sitemap_findings_reads::ReadRegistry,
    lifecycle: lifecycle::Lifecycle,
}
struct Job {
    id: Uuid,
    config: CrawlConfig,
    status: String,
    started_at: String,
    finished_at: Option<String>,
    pages: Vec<PageRecord>,
    issues: Vec<AuditIssue>,
    queued: usize,
    discovery_pending: usize,
    in_flight: usize,
    warnings: Vec<String>,
    error: Option<String>,
    report: Option<CrawlReport>,
    control: CrawlControl,
    generation: u64,
    checkpoint: Option<CrawlCheckpoint>,
    frontier_available: bool,
    operation: Option<SelectedRecrawlOperation>,
    recrawl_override: RecrawlResumeOverride,
    policy_ledger: Option<open_seo_spider_core::PolicyLedgerSummary>,
    resource_ledger: Option<open_seo_spider_core::ResourceLedgerSummary>,
}
#[derive(Serialize, Deserialize)]
struct JobManifest {
    id: Uuid,
    config: CrawlConfig,
    status: String,
    started_at: String,
    finished_at: Option<String>,
    #[serde(default)]
    generation: u64,
    #[serde(default)]
    checkpoint: Option<CrawlCheckpoint>,
}
impl Job {
    fn manifest(&self) -> JobManifest {
        JobManifest {
            id: self.id,
            config: self.config.clone(),
            status: self.status.clone(),
            started_at: self.started_at.clone(),
            finished_at: self.finished_at.clone(),
            generation: self.generation,
            checkpoint: self.checkpoint.clone(),
        }
    }
    fn status(&self) -> Value {
        let failed = self
            .pages
            .iter()
            .filter(|page| {
                page.error.is_some() || page.final_status.is_some_and(|status| status >= 400)
            })
            .count();
        let active = matches!(self.status.as_str(), "running" | "pausing" | "cancelling");
        json!({"id":self.id,"status":self.status,"seed_url":self.config.seed_urls.first(),"config":self.config,"started_at":self.started_at,"finished_at":self.finished_at,"generation":self.generation,"checkpoint":self.checkpoint,"operation":self.operation,"policy_ledger":self.policy_ledger,"resource_ledger":self.resource_ledger,"resume_available":self.frontier_available && !active,"recrawl_available":self.report.is_some() && !active,"parent_snapshot_available":self.operation.is_some() && self.frontier_available,"stats":{"discovered":self.pages.len()+self.queued+self.in_flight,"queued":self.queued,"discovery_pending":self.discovery_pending,"fetched":self.pages.len(),"failed":failed,"issues":self.issues.len(),"in_flight":self.in_flight},"warnings":self.warnings,"error":self.error,"summary":self.report.as_ref().map(|report| &report.summary)})
    }
}
#[derive(Serialize)]
struct ApiError {
    error: String,
}
type ApiResult<T> = Result<T, (StatusCode, Json<ApiError>)>;
fn api_error(code: StatusCode, message: impl ToString) -> (StatusCode, Json<ApiError>) {
    (
        code,
        Json(ApiError {
            error: message.to_string(),
        }),
    )
}

pub async fn serve(listen: &str, data_dir: PathBuf) -> anyhow::Result<()> {
    serve_with_mcp_files(listen, data_dir, None).await
}

pub async fn serve_with_mcp_files(
    listen: &str,
    data_dir: PathBuf,
    mcp_files_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    serve_with_mcp_files_ready(listen, data_dir, mcp_files_dir, None).await
}

pub(crate) async fn serve_with_mcp_files_ready(
    listen: &str,
    data_dir: PathBuf,
    mcp_files_dir: Option<PathBuf>,
    ready: Option<tokio::sync::oneshot::Sender<std::net::SocketAddr>>,
) -> anyhow::Result<()> {
    serve_with_mcp_files_ready_source(
        listen,
        data_dir,
        mcp_files_dir,
        ready,
        lifecycle::ShutdownSource::CtrlC,
    )
    .await
}

pub async fn serve_with_owned_stdin(
    listen: &str,
    data_dir: PathBuf,
    mcp_files_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    serve_with_mcp_files_ready_source(
        listen,
        data_dir,
        mcp_files_dir,
        None,
        lifecycle::ShutdownSource::OwnedStdin,
    )
    .await
}

async fn serve_with_mcp_files_ready_source(
    listen: &str,
    data_dir: PathBuf,
    mcp_files_dir: Option<PathBuf>,
    ready: Option<tokio::sync::oneshot::Sender<std::net::SocketAddr>>,
    source: lifecycle::ShutdownSource,
) -> anyhow::Result<()> {
    // Validate and retain the directory capability before either listening or
    // creating the application data directory. Port zero is replaced in place.
    let mut mcp = crate::mcp::Bridge::new_with_file_tools("http://127.0.0.1:0", mcp_files_dir)?;
    let listener = if mcp.file_tools_enabled() {
        let addresses: Vec<_> = tokio::net::lookup_host(listen).await?.collect();
        anyhow::ensure!(
            !addresses.is_empty() && addresses.iter().all(|address| address.ip().is_loopback()),
            "MCP file tools require an exclusively loopback listen address"
        );
        tokio::net::TcpListener::bind(addresses.as_slice()).await?
    } else {
        tokio::net::TcpListener::bind(listen).await?
    };
    let address = listener.local_addr()?;
    mcp.set_local_api_address(address)?;
    tokio::fs::create_dir_all(&data_dir).await?;
    let lifecycle = lifecycle::Lifecycle::default();
    mcp.set_task_owner(lifecycle.blocking.clone());
    let state = AppState {
        mcp,
        jobs: Arc::new(RwLock::new(HashMap::new())),
        data_dir: Arc::new(data_dir),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle,
    };
    load_reports(&state).await?;
    // Signal monitoring starts only after capability validation and recovery.
    let (shutdown_signal, stdin_monitor) = lifecycle::shutdown_source(source);
    let shutdown_state = state.clone();
    let drain_state = state.clone();
    let router = Router::new()
        .route(
            "/",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    include_str!("../../../web/index.html"),
                )
            }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../../../web/app.js"),
                )
            }),
        )
        .route(
            "/saved-comparison.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../../../web/saved-comparison.js"),
                )
            }),
        )
        .route(
            "/styles.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../../../web/styles.css"),
                )
            }),
        )
        .route(
            "/healthz",
            get(|| async {
                Json(
                    json!({"ok":true,"name":"Open SEO Spider","version":env!("CARGO_PKG_VERSION")}),
                )
            }),
        )
        .route("/api/capabilities", get(capabilities_with_mcp_files))
        .route(
            "/list-import.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../../../web/list-import.js"),
                )
            }),
        )
        .route("/api/lists/inspect", post(list_inspect))
        .route("/api/lists/import", post(list_import))
        .route("/api/policy/preview", post(policy_preview))
        .route("/api/content/analyze", post(content_analysis))
        .route("/api/css/inspect", post(resources::inspect_css))
        .route("/api/crawls/{id}/content", get(content::inventory))
        .route("/api/crawls/{id}/content/document", get(content::document))
        .route("/api/crawls/{id}/content/analyze", post(content::analyze))
        .route("/api/crawls/{id}/recrawl", post(recrawl))
        .route("/api/crawls/{id}/parent.json", get(export_recrawl_parent))
        .route(
            "/serp",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    include_str!("../../../web/serp.html"),
                )
            }),
        )
        .route(
            "/serp.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../../../web/serp.js"),
                )
            }),
        )
        .route("/api/serp/preview", post(serp_preview))
        .route("/api/serp/import", post(serp_import))
        .route("/api/serp/export.csv", post(serp_export))
        .route(
            "/robots",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    include_str!("../../../web/robots.html"),
                )
            }),
        )
        .route(
            "/robots.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../../../web/robots.js"),
                )
            }),
        )
        .route("/api/robots/simulate", post(robots_simulate))
        .route("/api/robots/export.csv", post(robots_export))
        .route("/mcp", post(mcp_http))
        .route("/api/crawls", get(list_crawls).post(create_crawl))
        .route("/api/crawls/import", post(import_report))
        .route("/api/crawls/{id}", get(crawl_status))
        .route("/api/crawls/{id}/pages", get(pages))
        .route("/api/crawls/{id}/issues", get(issues))
        .route("/api/crawls/{id}/analysis", get(advanced_analysis))
        .route("/api/crawls/{id}/policy", get(policy::inventory))
        .route(
            "/api/crawls/{id}/policy.json",
            get(policy::export_json).layer(middleware::from_fn(response_stream::require_http11)),
        )
        .route(
            "/api/crawls/{id}/policy.csv",
            get(policy::export_csv).layer(middleware::from_fn(response_stream::require_http11)),
        )
        .route("/api/crawls/{id}/sitemaps", get(sitemaps::inventory))
        .route(
            "/api/crawls/{id}/sitemaps/source-bodies",
            get(source_bodies::inventory),
        )
        .route(
            "/api/crawls/{id}/sitemaps/source-bodies/{body_id}",
            get(source_bodies::download).head(source_bodies::unsupported_head),
        )
        .merge(sitemap_findings::routes())
        .route(
            "/api/crawls/{id}/sitemaps/export.csv",
            get(sitemaps::export_csv),
        )
        .route("/api/crawls/{id}/resources", get(resources::inventory))
        .route(
            "/api/crawls/{id}/resources/transports",
            get(resources::transports),
        )
        .route(
            "/api/crawls/{id}/resources.json",
            get(resources::export_json).layer(middleware::from_fn(response_stream::require_http11)),
        )
        .route(
            "/api/crawls/{id}/resources.csv",
            get(resources::export_csv).layer(middleware::from_fn(response_stream::require_http11)),
        )
        .route("/api/crawls/{id}/responses", get(response_inventory))
        .route("/api/crawls/{id}/responses.csv", get(export_responses))
        .route("/api/crawls/{id}/fragments", get(fragment_inventory))
        .route("/api/crawls/{id}/fragments.csv", get(export_fragments))
        .route("/api/crawls/{id}/dom.csv", get(export_dom))
        .route("/api/crawls/{id}/inputs.csv", get(export_inputs))
        .route("/api/crawls/{id}/cancel", post(cancel_crawl))
        .route("/api/crawls/{id}/pause", post(pause_crawl))
        .route("/api/crawls/{id}/resume", post(resume_crawl))
        .route("/api/crawls/{id}/export.csv", get(export_csv))
        .route("/api/crawls/{id}/issues.csv", get(export_issue_csv))
        .route("/api/crawls/{id}/links.csv", get(export_link_csv))
        .route(
            "/api/crawls/{id}/javascript.csv",
            get(export_javascript_csv),
        )
        .route("/api/crawls/{id}/cookies.csv", get(export_cookie_csv))
        .route("/api/crawls/{id}/pdf.csv", get(export_pdf))
        .route("/api/crawls/{id}/images.csv", get(export_images))
        .route(
            "/api/crawls/{id}/extractions.csv",
            get(export_extraction_csv),
        )
        .route("/api/crawls/{id}/sitemap.xml", get(export_xml))
        .route(
            "/api/crawls/{id}/sitemap-bundle",
            post(sitemap_generation::bundle),
        )
        .route("/api/crawls/{id}/project.json", get(export_project))
        .route(
            "/resource-broken-sources",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    include_str!("../../../web/resource-broken-sources.html"),
                )
            }),
        )
        .route(
            "/resource-broken-sources.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../../../web/resource-broken-sources.js"),
                )
            }),
        )
        .route(
            "/resource-broken-sources.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../../../web/resource-broken-sources.css"),
                )
            }),
        )
        .route(
            "/api/crawls/{id}/resource-broken-sources/preview",
            post(resource_broken_sources::preview).layer(axum::extract::DefaultBodyLimit::max(
                crate::resource_broken_sources_app::REQUEST_BYTES,
            )),
        )
        .route(
            "/api/crawls/{id}/resource-broken-sources/csv",
            post(resource_broken_sources::csv)
                .layer(axum::extract::DefaultBodyLimit::max(
                    crate::resource_broken_sources_app::REQUEST_BYTES,
                ))
                .layer(middleware::from_fn(response_stream::require_http11)),
        )
        .route(
            "/broken-sources",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    include_str!("../../../web/broken-sources.html"),
                )
            }),
        )
        .route(
            "/broken-sources.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../../../web/broken-sources.js"),
                )
            }),
        )
        .route(
            "/broken-sources.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../../../web/broken-sources.css"),
                )
            }),
        )
        .route(
            "/api/crawls/{id}/broken-sources/preview",
            post(broken_sources::preview).layer(axum::extract::DefaultBodyLimit::max(
                crate::broken_sources::REQUEST_BYTES,
            )),
        )
        .route(
            "/api/crawls/{id}/broken-sources/csv",
            post(broken_sources::csv).layer(axum::extract::DefaultBodyLimit::max(
                crate::broken_sources::REQUEST_BYTES,
            )),
        )
        .route(
            "/directory-tree",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    include_str!("../../../web/directory-tree.html"),
                )
            }),
        )
        .route(
            "/directory-tree.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../../../web/directory-tree.js"),
                )
            }),
        )
        .route(
            "/directory-tree.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../../../web/directory-tree.css"),
                )
            }),
        )
        .route(
            "/api/crawls/{id}/directory-tree/preview",
            post(directory_tree::preview).layer(axum::extract::DefaultBodyLimit::max(
                directory_tree::REQUEST_BYTES,
            )),
        )
        .route(
            "/api/crawls/{id}/directory-tree/csv",
            post(directory_tree::csv).layer(axum::extract::DefaultBodyLimit::max(
                directory_tree::REQUEST_BYTES,
            )),
        )
        .route(
            "/saved-ngrams",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    include_str!("../../../web/saved-ngrams.html"),
                )
            }),
        )
        .route(
            "/saved-ngrams.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../../../web/saved-ngrams.js"),
                )
            }),
        )
        .route(
            "/saved-ngrams.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../../../web/saved-ngrams.css"),
                )
            }),
        )
        .route(
            "/api/crawls/{id}/ngrams/preview",
            post(ngram_corpus::preview).layer(axum::extract::DefaultBodyLimit::max(
                crate::ngram_app::REQUEST_BYTES,
            )),
        )
        .route(
            "/api/crawls/{id}/ngrams/csv",
            post(ngram_corpus::csv)
                .layer(axum::extract::DefaultBodyLimit::max(
                    crate::ngram_app::REQUEST_BYTES,
                ))
                .layer(middleware::from_fn(response_stream::require_http11)),
        )
        .route(
            "/migration",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    include_str!("../../../web/migration.html"),
                )
            }),
        )
        .route(
            "/migration.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
                    include_str!("../../../web/migration.js"),
                )
            }),
        )
        .route(
            "/migration.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                    include_str!("../../../web/migration.css"),
                )
            }),
        )
        .route(
            "/api/crawls/{id}/migration/preview",
            post(migration::preview).layer(axum::extract::DefaultBodyLimit::max(
                crate::migration_app::REQUEST_BYTES,
            )),
        )
        .route(
            "/api/crawls/{id}/migration/json",
            post(migration::json).layer(axum::extract::DefaultBodyLimit::max(
                crate::migration_app::REQUEST_BYTES,
            )),
        )
        .route(
            "/api/crawls/{id}/migration/csv",
            post(migration::csv).layer(axum::extract::DefaultBodyLimit::max(
                crate::migration_app::REQUEST_BYTES,
            )),
        )
        .route("/api/compare", get(compare))
        .route(
            "/api/compare/saved",
            post(comparison::compare_saved).layer(axum::extract::DefaultBodyLimit::max(
                crate::comparison_app::MAX_REQUEST_BYTES,
            )),
        )
        .layer(axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            lifecycle::request,
        ))
        .layer(middleware::from_fn(local_origin))
        .with_state(state.clone());
    let response_cancel = state.lifecycle.response_streams.clone();
    let body_cancel = state.lifecycle.source_bodies.clone();
    // Registration owns the listener before readiness. Even startup refusal
    // still reaches every existing drain and the retained stdin monitor below.
    let (serving, reason, drained) = match state.lifecycle.transports.start(
        listener,
        router,
        Arc::new(move || {
            response_cancel.cancel_all();
            body_cancel.cancel_all();
        }),
    ) {
        Ok(running) => {
            println!("Open SEO Spider: http://{}", address);
            if let Some(ready) = ready {
                let _ = ready.send(address);
            }
            let serving = running.wait();
            let draining = async move {
                let reason = tokio::select! {
                    reason = shutdown_signal => reason,
                    _ = shutdown_state.lifecycle.transports.wait_failure() => Err(anyhow::anyhow!("Owned incoming transport failed")),
                };
                // T0 closes every admission synchronously, including on error.
                lifecycle::close_admission(&shutdown_state);
                let drained = lifecycle::drain(&drain_state).await;
                (reason, drained)
            };
            let (serving, (reason, drained)) = tokio::join!(serving, draining);
            (serving, reason, drained)
        }
        Err(error) => {
            drop(ready);
            lifecycle::close_admission(&state);
            let drained = lifecycle::drain(&state).await;
            (Err(error), Ok("incoming_start_failure"), drained)
        }
    };
    // Owned stdin uses a runtime blocking read. Ctrl-C must not discard it.
    let monitor = match stdin_monitor {
        Some(monitor) => monitor
            .await
            .map_err(anyhow::Error::from)
            .and_then(|result| result.map_err(anyhow::Error::from)),
        None => Ok(()),
    };
    let mut failures = Vec::new();
    if let Err(error) = serving {
        failures.push(error.to_string());
    }
    if let Err(error) = drained {
        failures.push(error.to_string());
    }
    if let Err(error) = monitor {
        failures.push(error.to_string());
    }
    let cause = match reason {
        Ok(cause) => cause,
        Err(error) => {
            failures.push(error.to_string());
            "signal_failure"
        }
    };
    lifecycle::receipt(&state, cause, failures.is_empty());
    anyhow::ensure!(
        failures.is_empty(),
        "Owned server shutdown failed: {}",
        failures.join("; ")
    );
    Ok(())
}
async fn mcp_http(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<Value>,
) -> Response {
    if let Some(version) = headers
        .get("mcp-protocol-version")
        .and_then(|value| value.to_str().ok())
        && !crate::mcp::PROTOCOLS.contains(&version)
    {
        return api_error(StatusCode::BAD_REQUEST, "Unsupported MCP protocol version")
            .into_response();
    }
    match state.mcp.dispatch(request).await {
        Some(response) => Json(response).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

async fn local_origin(request: Request, next: Next) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let local = host
        .parse::<axum::http::uri::Authority>()
        .ok()
        .is_some_and(|authority| {
            let host = authority.host();
            let address = host
                .strip_prefix('[')
                .and_then(|value| value.strip_suffix(']'))
                .unwrap_or(host);
            host == "localhost"
                || address
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|address| address.is_loopback())
        });
    if !local {
        return api_error(StatusCode::FORBIDDEN, "Use a loopback hostname").into_response();
    }
    if let Some(origin) = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        && origin != format!("http://{host}")
        && origin != format!("https://{host}")
    {
        return api_error(
            StatusCode::FORBIDDEN,
            "Cross-origin requests are not allowed",
        )
        .into_response();
    }
    next.run(request).await
}

async fn capabilities() -> Json<Value> {
    Json(json!({
        "product":"Open SEO Spider",
        "crawl":{
            "http":true,"list":true,"sitemap":true,"robots":true,
            "agent_scoped_indexing_directives":true,"http_link_relationships":true,
            "redirects":true,"intermediate_response_inventory":true,
            "selected_recrawl":true,"selected_recrawl_profile":"new_child_exact_requested_urls_no_outward_discovery",
            "cancel":true,"pause":true,"pause_semantics":"stop_admission_then_drain_then_checkpoint","resume":true,"sqlite_only_recovery":true,"immutable_report_revisions":true,"url_limit":null,"url_regex_filters":true,
            "css_extraction":true,"xpath_1_0_extraction":true,"regex_extraction":true,
            "html_extraction_output":true,"configurable_audit_thresholds":true,
            "local_font_pixel_profile":true,"native_pdf_observations":true,
            "fetched_image_dimensions":true,
            "heading_dom_observations":true,"meta_keyword_observations":true,
            "link_dom_positions":true,"custom_css_link_positions":true,
            "responsive_image_candidates":true,"nonstandard_navigation_observations":true,
            "fragment_validation":{"opt_in":true,"requires_captured_target":true,"unknown_is_not_missing":true}
        },
        "offline_policy_preview":{"endpoint":"/api/policy/preview","website_requests":0,"applied_to_crawls":true},
        "native_crawl_policy":{"opt_in":true,"engine":open_seo_spider_core::LIVE_CRAWL_POLICY_ENGINE,"durable_sqlite_ledger":true,"paged_observations":true,"streamed_snapshot_exports":true,"renderer_supported":false,"scope_does_not_grant_credentials":true,"occurrence_limit":null},
        "offline_content_analysis":{"endpoint":"/api/content/analyze","website_requests":0,"similarity":"exact_token_shingle_jaccard","readability":"explicit_en_de_heuristic","full_reference_parity":false},
        "saved_content_analysis":{"endpoint":"/api/crawls/{id}/content/analyze","engine":crate::content_cohort::SAVED_CONTENT_ENGINE,"website_requests":0,"explicit_cohort":true,"max_documents":128,"site_wide_coverage":false},
        "saved_comparison":{"endpoint":"/api/compare/saved","engine":open_seo_spider_core::SAVED_COMPARISON_ENGINE,"website_requests":0,"exact_requested_identity":true,"explicit_mapping":true,"unknown_facts_preserved":true,"full_original_report_bundle":false,"sql_attested":false,"full_reference_parity":false},
        "native_content_capture":{"opt_in":true,"sources":["raw_http_html","rendered_dom_html"],"body_retention":false,"source_binding":"persisted_consistency","full_reference_parity":false},
        "native_resource_discovery":{"opt_in":true,"engine":open_seo_spider_core::RESOURCE_DISCOVERY_ENGINE,"semantic_contracts":[open_seo_spider_core::RESOURCE_DISCOVERY_ENGINE,open_seo_spider_core::RESOURCE_CONTRACT_ENGINE],"contract_selection":"explicit_optional_config","qualified_directives":["nofollow","ugc","sponsored"],"qualified_link_scope":"target_internal_external_including_document_nofollow","durable_original_reference_roles":true,"durable_sqlite_ledger":true,"paged_occurrences":true,"paged_transport_metadata":true,"streamed_snapshot_exports":true,"entity_body_retention":false,"renderer_supported":false,"full_reference_parity":false},
        "stored_sitemap_source_bodies":{"opt_in":true,"config_field":"sitemap_source_archive","storage":"utf8_sqlite_own_archive","metadata_byte_availability":"not_checked","download":"verified_private_spool_before_headers","max_binary_bytes":67108864,"json_page_limits_unchanged":true,"wire_bytes":false,"portable_bundle":false,"inherited_body_copy":false,"unresponsive_transport_drain":"not_proven"},
        "crawl_admission_limits":{"opt_in":true,"config_field":"crawl_admission_limits","version":1,"static_only":true,"folder_depth_zero_allowed":true,"query_tokens_zero_allowed":true,"fetch_url_bytes_min":1,"fetch_url_bytes_max":1048576,"page_document_and_redirects":true,"renderer_supported":false,"dynamic_quotas":false},
        "sitemap_membership_limits":{"opt_in":true,"config_field":"sitemap_membership_limits","default_catalog_json_bytes":open_seo_spider_core::DEFAULT_SITEMAP_CATALOG_BYTES,"max_catalog_json_bytes":open_seo_spider_core::MAX_SITEMAP_CATALOG_BYTES,"source_work_independent":true,"raw_xml_body_retention":false,"wire_limits_unchanged":true},
        "offline_serp":{"path":"/serp","website_requests":0,"csv_import":true,"row_limit":10000},
        "offline_robots":{"path":"/robots","website_requests":0,"engine":"crawler_policy","remote_rules_fetch":false},
        "offline_list_import":{"formats":["txt","csv","xml","xlsx"],"website_requests":0,"max_input_bytes":8388608,"max_rows_per_file":50000,"row_order":true,"duplicates":true,"xlsx_formulas":"reject_or_explicit_cached_string_only","provenance":"declared_client_input"},
        "optional_worker":{"javascript_rendering":true,"custom_javascript":true,
            "redacted_cookie_observations":true,"screenshots":true,"accessibility_axe":true,
            "endpoint":"http://127.0.0.1:8790/render","enabled_by_default":false},
        "mcp":{"stdio":true,"streamable_http":"/mcp"},
        "exports":["pages_csv","issues_csv","links_csv","extractions_csv","javascript_csv","cookies_csv","pdf_csv","image_resources_csv","responses_csv","fragments_csv","dom_csv","inputs_csv","sitemap_xml","sitemap_bundle_zip","project_json"],
        "unimplemented":["api_integrations","spelling_grammar","pdf_ocr_and_rendering","shared_authenticated_browser_sessions","embedding_semantic_search","scheduling","full_structured_data_validation"],
        "parity":"in progress; see docs/parity/granular-acceptance.json"
    }))
}

async fn capabilities_with_mcp_files(State(state): State<AppState>) -> Json<Value> {
    let Json(mut value) = capabilities().await;
    if let Some(capability) = state.mcp.file_tools_capability() {
        value["mcp"]["file_tools"] = capability;
    }
    Json(value)
}

async fn content_analysis(
    State(state): State<AppState>,
    Json(input): Json<ContentAnalysisInput>,
) -> ApiResult<Json<ContentAnalysisResult>> {
    state
        .lifecycle
        .blocking
        .run_blocking(move || analyze_content(&input))
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?
        .map(Json)
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))
}
async fn policy_preview(
    State(state): State<AppState>,
    Json(input): Json<CrawlPolicyPreviewInput>,
) -> ApiResult<Json<Value>> {
    let preview = state
        .lifecycle
        .blocking
        .run_blocking(move || preview_crawl_policy(&input))
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
    Ok(Json(serde_json::to_value(preview).map_err(|error| {
        api_error(StatusCode::INTERNAL_SERVER_ERROR, error)
    })?))
}
async fn list_crawls(State(state): State<AppState>) -> Json<Value> {
    let jobs: Vec<_> = state.jobs.read().await.values().cloned().collect();
    let mut results = Vec::new();
    for job in jobs {
        results.push(job.read().await.status());
    }
    results.sort_by(|left, right| {
        right["started_at"]
            .as_str()
            .cmp(&left["started_at"].as_str())
    });
    Json(json!({"crawls":results}))
}
#[cfg(test)]
#[path = "server/crawl_admission_limits_tests.rs"]
mod crawl_admission_limits_tests;

fn config_from_input(mut value: Value) -> anyhow::Result<CrawlConfig> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Configuration must be an object"))?;
    if !object.contains_key("seed_urls") {
        let mut seeds = object
            .get("urls")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if let Some(seed) = object
            .get("seed_url")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            seeds.insert(0, json!(seed));
        }
        object.insert("seed_urls".into(), Value::Array(seeds));
    }
    for (from, to) in [
        ("follow_subdomains", "include_subdomains"),
        ("timeout_seconds", "request_timeout_secs"),
    ] {
        if !object.contains_key(to)
            && let Some(value) = object.get(from).cloned()
        {
            object.insert(to.into(), value);
        }
    }
    if object.get("mode").and_then(Value::as_str) == Some("list") {
        object.insert("crawl_links".into(), json!(false));
    }
    for key in ["max_pages", "max_depth"] {
        if object.get(key).and_then(Value::as_u64) == Some(0) {
            object.insert(key.into(), Value::Null);
        }
    }
    let config: CrawlConfig = serde_json::from_value(value)?;
    // Fresh construction captures explicit raw sitemap declarations before merging imports.
    let crawler = Crawler::new(config)?;
    crawler
        .effective_config()
        .validate_input_provenance_budget()?;
    Ok(crawler.effective_config().clone())
}
async fn create_crawl(
    State(state): State<AppState>,
    Json(value): Json<Value>,
) -> ApiResult<Json<Value>> {
    lifecycle::start_operation(state.clone(), create_crawl_prepared(state, value)).await
}
async fn create_crawl_prepared(
    state: AppState,
    value: Value,
) -> ApiResult<lifecycle::PreparedLaunch> {
    let config =
        config_from_input(value).map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
    let id = Uuid::new_v4();
    let job = Arc::new(RwLock::new(Job {
        id,
        config,
        status: "running".into(),
        started_at: chrono::Utc::now().to_rfc3339(),
        finished_at: None,
        pages: Vec::new(),
        issues: Vec::new(),
        queued: 0,
        discovery_pending: 0,
        in_flight: 0,
        warnings: Vec::new(),
        error: None,
        report: None,
        control: CrawlControl::new(),
        generation: 1,
        checkpoint: None,
        frontier_available: false,
        operation: None,
        recrawl_override: RecrawlResumeOverride::default(),
        policy_ledger: None,
        resource_ledger: None,
    }));
    save_manifest(&state, &job)
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    state.jobs.write().await.insert(id, job.clone());
    Ok((Json(json!({"id":id,"status":"running"})), job))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecrawlRequest {
    urls: Vec<String>,
    #[serde(default)]
    max_attempts: Option<usize>,
}
async fn recrawl(
    State(state): State<AppState>,
    Path(parent_id): Path<Uuid>,
    Json(request): Json<RecrawlRequest>,
) -> ApiResult<Json<Value>> {
    lifecycle::start_operation(state.clone(), recrawl_prepared(state, parent_id, request)).await
}
async fn recrawl_prepared(
    state: AppState,
    parent_id: Uuid,
    request: RecrawlRequest,
) -> ApiResult<lifecycle::PreparedLaunch> {
    let parent = get_job(&state, parent_id).await?;
    let guard = parent.read().await;
    if matches!(guard.status.as_str(), "running" | "pausing" | "cancelling") {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Wait for a stable source checkpoint before selected recrawl",
        ));
    }
    let report = guard.report.as_ref().ok_or_else(|| {
        api_error(
            StatusCode::CONFLICT,
            "Selected recrawl needs a captured source report",
        )
    })?;
    let current = serde_json::to_vec(report)
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    let stored_path = state.data_dir.join(format!("{parent_id}.json"));
    let stored = match tokio::fs::metadata(&stored_path).await {
        Ok(metadata) if metadata.len() <= 256 * 1024 * 1024 => {
            tokio::fs::read(&stored_path).await.ok()
        }
        _ => None,
    };
    let unchanged = stored.as_ref().is_some_and(|bytes| {
        serde_json::from_slice::<CrawlReport>(bytes)
            .ok()
            .and_then(|report| serde_json::to_vec(&report).ok())
            .as_ref()
            == Some(&current)
    });
    let source_kind = if !unchanged {
        RecrawlSourceKind::CurrentProjection
    } else if guard.checkpoint.is_some() {
        RecrawlSourceKind::CapturedCheckpoint
    } else if guard.frontier_available {
        RecrawlSourceKind::StoredJson
    } else {
        RecrawlSourceKind::ImportedSnapshot
    };
    let reference = SelectedRecrawlParent {
        job_id: parent_id.to_string(),
        checkpoint_revision: guard
            .checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.revision),
        source_kind,
    };
    let bytes = if unchanged {
        stored.expect("checked source bytes")
    } else {
        current
    };
    drop(guard);
    let prepared = state
        .lifecycle
        .blocking
        .run_blocking(move || {
            prepare_selected_recrawl(reference, &bytes, &request.urls, request.max_attempts)
        })
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
    let id = Uuid::new_v4();
    let database = state.data_dir.join(format!("{id}.sqlite"));
    // Reserve this exact new UUID before any initialization/cleanup can touch it.
    let reservation = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&database)
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    drop(reservation);
    let config = prepared.config.clone();
    let initialization = state
        .lifecycle
        .blocking
        .run_blocking(move || -> anyhow::Result<_> {
            let store = CrawlStore::open(&database)?;
            store.initialize_selected_recrawl(&prepared)?;
            Ok((store.pages()?, store.recrawl_operation()?))
        })
        .await
        .map_err(|error| anyhow::anyhow!(error))
        .and_then(|result| result);
    let (pages, operation) = match initialization {
        Ok(value) => value,
        Err(error) => {
            remove_reserved_job_files(&state, id).await;
            return Err(api_error(StatusCode::INTERNAL_SERVER_ERROR, error));
        }
    };
    let queued = operation
        .as_ref()
        .map_or(0, |operation| operation.pending_count);
    let job = Arc::new(RwLock::new(Job {
        id, config, status:"running".into(), started_at:chrono::Utc::now().to_rfc3339(), finished_at:None,
        pages, issues:Vec::new(), queued, discovery_pending:0, in_flight:0,
        warnings:vec!["Selected recrawl retains unrelated source observations; current coverage combines inherited and newly captured results.".into()],
        error:None, report:None, control:CrawlControl::new(), generation:1, checkpoint:None,
        frontier_available:true, operation, recrawl_override:RecrawlResumeOverride::default(), policy_ledger:None, resource_ledger:None,
    }));
    if let Err(error) = save_manifest(&state, &job).await {
        remove_reserved_job_files(&state, id).await;
        return Err(api_error(StatusCode::INTERNAL_SERVER_ERROR, error));
    }
    state.jobs.write().await.insert(id, job.clone());
    Ok((
        Json(json!({"id":id,"status":"running","parent_id":parent_id})),
        job,
    ))
}
async fn remove_reserved_job_files(state: &AppState, id: Uuid) {
    for suffix in [
        ".sqlite",
        ".sqlite-wal",
        ".sqlite-shm",
        ".job.json",
        ".job.tmp",
    ] {
        let path = state.data_dir.join(format!("{id}{suffix}"));
        if let Err(error) = tokio::fs::remove_file(&path).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%id,%error,path=?path,"Could not remove failed reserved child initialization");
        }
    }
}
async fn export_recrawl_parent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    get_job(&state, id).await?;
    let database = state.data_dir.join(format!("{id}.sqlite"));
    let bytes = state
        .lifecycle
        .blocking
        .run_blocking(move || -> anyhow::Result<Vec<u8>> {
            let store = CrawlStore::open_read_only(database)?;
            if !matches!(store.run_plan()?, RunPlan::SelectedRecrawl(_)) {
                anyhow::bail!("This job has no immutable selected-recrawl parent snapshot");
            }
            store
                .parent_snapshot_bytes()?
                .ok_or_else(|| anyhow::anyhow!("Saved immutable parent snapshot is unavailable"))
        })
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?
        .map_err(|error| api_error(StatusCode::CONFLICT, error))?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=parent-snapshot.json",
            ),
        ],
        bytes,
    )
        .into_response())
}
async fn get_job(state: &AppState, id: Uuid) -> ApiResult<Arc<RwLock<Job>>> {
    state
        .jobs
        .read()
        .await
        .get(&id)
        .cloned()
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Crawl not found"))
}
async fn crawl_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let job = get_job(&state, id).await?;
    Ok(Json(job.read().await.status()))
}
#[derive(Default, Deserialize)]
struct PageQuery {
    offset: Option<usize>,
    limit: Option<usize>,
    q: Option<String>,
    internal: Option<bool>,
}
async fn pages(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<PageQuery>,
) -> ApiResult<Json<Value>> {
    let job = get_job(&state, id).await?;
    let guard = job.read().await;
    let q = query.q.unwrap_or_default().to_lowercase();
    let filtered: Vec<_> = guard
        .pages
        .iter()
        .filter(|page| {
            query
                .internal
                .is_none_or(|internal| page.internal == internal)
                && (q.is_empty()
                    || page.url.to_lowercase().contains(&q)
                    || page.title().to_lowercase().contains(&q))
        })
        .collect();
    let total = filtered.len();
    let limit = query.limit.unwrap_or(200).clamp(1, 10000);
    let offset = query.offset.unwrap_or(0);
    let results: Vec<_> = filtered
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|page| {
            let mut value = serde_json::to_value(page).unwrap_or(Value::Null);
            if let Some(object) = value.as_object_mut() {
                object.insert("title".into(), json!(page.title()));
                object.insert("meta_description".into(), json!(page.description()));
                object.insert("response_time_ms".into(), json!(page.response_ms));
            }
            value
        })
        .collect();
    Ok(Json(
        json!({"pages":results,"total":total,"offset":offset,"limit":limit}),
    ))
}
async fn issues(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Json<Value>> {
    let job = get_job(&state, id).await?;
    let guard = job.read().await;
    Ok(Json(
        json!({"issues":guard.issues,"total":guard.issues.len()}),
    ))
}
async fn cancel_crawl(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let job = get_job(&state, id).await?;
    let mut guard = job.write().await;
    if guard.status == "cancelling" {
        return Ok(Json(guard.status()));
    }
    if !matches!(guard.status.as_str(), "running" | "pausing") {
        return Err(api_error(StatusCode::CONFLICT, "Crawl is not running"));
    }
    if !guard.control.request_cancel() {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Crawl is finalizing its checkpoint",
        ));
    }
    guard.status = "cancelling".into();
    Ok(Json(guard.status()))
}
async fn pause_crawl(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let job = get_job(&state, id).await?;
    let mut guard = job.write().await;
    if matches!(guard.status.as_str(), "pausing" | "paused") {
        return Ok(Json(guard.status()));
    }
    if guard.status != "running" {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Only a running crawl can be paused",
        ));
    }
    if !guard.control.request_pause() {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Crawl is finalizing its checkpoint",
        ));
    }
    guard.status = "pausing".into();
    Ok(Json(guard.status()))
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ResumeOptions {
    /// Omitted retains the saved limit; an explicit zero removes this operator limit.
    max_pages: Option<usize>,
    max_attempts: Option<usize>,
    concurrency: Option<usize>,
    delay_ms: Option<u64>,
    request_timeout_secs: Option<u64>,
}
async fn resume_crawl(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    options: Option<Json<ResumeOptions>>,
) -> ApiResult<Json<Value>> {
    let options = options.map(|Json(options)| options).unwrap_or_default();
    lifecycle::start_operation(state.clone(), resume_crawl_prepared(state, id, options)).await
}
async fn resume_crawl_prepared(
    state: AppState,
    id: Uuid,
    options: ResumeOptions,
) -> ApiResult<lifecycle::PreparedLaunch> {
    let job = get_job(&state, id).await?;
    let database = state.data_dir.join(format!("{id}.sqlite"));
    if !database.exists() {
        return Err(api_error(
            StatusCode::CONFLICT,
            "No saved SQLite crawl frontier is available; start a new crawl",
        ));
    }
    {
        let mut guard = job.write().await;
        if matches!(guard.status.as_str(), "running" | "pausing" | "cancelling") {
            return Err(api_error(StatusCode::CONFLICT, "Crawl is already active"));
        }
        let inspection = CrawlStore::inspect(&database).map_err(|error| {
            api_error(
                StatusCode::CONFLICT,
                format!("Invalid saved frontier: {error}"),
            )
        })?;
        if inspection.config.is_none() {
            return Err(api_error(
                StatusCode::CONFLICT,
                "Saved frontier has no crawl configuration",
            ));
        }
        let selected = inspection.operation.is_some();
        if selected && options.max_pages.is_some() {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "Selected recrawl uses max_attempts; its inherited page limit is not changed",
            ));
        }
        if !selected && options.max_attempts.is_some() {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "max_attempts applies only to selected recrawl",
            ));
        }
        let generation = inspection
            .run_state
            .as_ref()
            .map_or(1, |run| run.generation.saturating_add(1));
        let mut config = inspection.config.clone().ok_or_else(|| {
            api_error(
                StatusCode::CONFLICT,
                "Saved frontier has no crawl configuration",
            )
        })?;
        if let Some(max_pages) = options.max_pages {
            config.max_pages = (max_pages > 0).then_some(max_pages);
        }
        if let Some(concurrency) = options.concurrency {
            config.concurrency = concurrency;
        }
        if let Some(delay_ms) = options.delay_ms {
            config.delay_ms = delay_ms;
        }
        if let Some(timeout) = options.request_timeout_secs {
            config.request_timeout_secs = timeout;
        }
        config
            .prepare_input_lists()
            .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
        Crawler::from_saved_config(config.clone())
            .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
        archive_current_report(&state, id)
            .await
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
        let mut manifest = guard.manifest();
        manifest.config = config.clone();
        manifest.status = "running".into();
        manifest.finished_at = None;
        manifest.generation = generation;
        manifest.checkpoint = None;
        write_json(state.data_dir.join(format!("{id}.job.json")), &manifest)
            .await
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
        guard.control = CrawlControl::new();
        guard.generation = generation;
        guard.config = config;
        guard.checkpoint = None;
        guard.status = "running".into();
        guard.finished_at = None;
        guard.error = None;
        guard.report = None;
        guard.operation = inspection.operation;
        guard.recrawl_override = RecrawlResumeOverride {
            max_attempts: options.max_attempts,
        };
    }
    Ok((Json(json!({"id":id,"status":"running"})), job))
}
async fn launch(state: AppState, job: Arc<RwLock<Job>>) {
    let (config, id, control, generation, recrawl_override) = {
        let guard = job.read().await;
        (
            guard.config.clone(),
            guard.id,
            guard.control.clone(),
            guard.generation,
            guard.recrawl_override.clone(),
        )
    };
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let event_job = job.clone();
    let event_task = tokio::spawn(async move {
        let mut page_indices: HashMap<String, usize> = event_job
            .read()
            .await
            .pages
            .iter()
            .enumerate()
            .map(|(index, page)| (page.url.clone(), index))
            .collect();
        let mut target_indices: HashMap<String, usize> = event_job
            .read()
            .await
            .operation
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
        while let Some(event) = receiver.recv().await {
            let mut guard = event_job.write().await;
            if guard.generation != generation {
                continue;
            }
            match event {
                CrawlEvent::Page { page } => {
                    if let Some(index) = page_indices.get(&page.url) {
                        guard.pages[*index] = *page;
                    } else {
                        page_indices.insert(page.url.clone(), guard.pages.len());
                        guard.pages.push(*page);
                    }
                }
                CrawlEvent::Progress {
                    queued, in_flight, ..
                } => {
                    guard.queued = queued;
                    guard.in_flight = in_flight;
                }
                CrawlEvent::Warning { message } => guard.warnings.push(message),
                CrawlEvent::PolicyProgress { ledger } => guard.policy_ledger = Some(*ledger),
                CrawlEvent::ResourceProgress { ledger } => guard.resource_ledger = Some(*ledger),
                CrawlEvent::Operation { operation } => {
                    target_indices = operation
                        .targets
                        .iter()
                        .enumerate()
                        .map(|(index, target)| (target.requested_url.clone(), index))
                        .collect();
                    guard.operation = Some(*operation);
                }
                CrawlEvent::RecrawlProgress {
                    target,
                    admitted_this_run,
                    committed_this_run,
                    refreshed_count,
                    pending_count,
                } => {
                    if let Some(operation) = &mut guard.operation {
                        if let Some(index) = target_indices.get(&target.requested_url) {
                            operation.targets[*index] = *target;
                        }
                        operation.admitted_this_run = admitted_this_run;
                        operation.committed_this_run = committed_this_run;
                        operation.refreshed_count = refreshed_count;
                        operation.pending_count = pending_count;
                    }
                }
                CrawlEvent::Finished { .. } => {}
            }
        }
    });
    let result = match Crawler::from_saved_config(config) {
        Ok(crawler) => match crawler.with_store(state.data_dir.join(format!("{id}.sqlite"))) {
            Ok(crawler) => {
                crawler
                    .with_recrawl_resume_override(recrawl_override)
                    .run_controlled(Some(sender.clone()), control)
                    .await
            }
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    };
    drop(sender);
    let event_failure = event_task
        .await
        .err()
        .map(|error| format!("Crawl event supervisor failed to join: {error}"));
    if let Some(message) = &event_failure {
        state.lifecycle.crawls.record_failure(message);
    }
    let mut guard = job.write().await;
    if guard.generation != generation {
        return;
    }
    guard.queued = 0;
    guard.in_flight = 0;
    guard.finished_at = Some(chrono::Utc::now().to_rfc3339());
    match result {
        Ok(result) => {
            guard.frontier_available = true;
            let mut report = result.report;
            crate::analysis::extend_report(&mut report);
            guard.status = outcome_status(result.outcome).into();
            guard.queued = result
                .checkpoint
                .as_ref()
                .map_or(0, |checkpoint| checkpoint.pending);
            guard.discovery_pending = result.discovery_pending;
            guard.checkpoint = result.checkpoint;
            guard.pages = report.pages.clone();
            guard.issues = report.issues.clone();
            guard.warnings = report.warnings.clone();
            guard.operation = report.operation.clone();
            guard.policy_ledger = report.policy_ledger.clone();
            guard.resource_ledger = report.resource_ledger.clone();
            guard.report = Some(report.clone());
            let path = state.data_dir.join(format!("{id}.json"));
            if let Err(error) = write_json(path, &report).await {
                guard.status = "failed".into();
                guard.error = Some(format!("Could not save report: {error}"));
                state
                    .lifecycle
                    .crawls
                    .record_failure(format!("Crawl {id} terminal persistence: {error}"));
            }
            if let Err(error) = save_revision(&state, id, generation, &report).await {
                guard.status = "failed".into();
                guard.error = Some(format!("Could not save immutable report revision: {error}"));
                state
                    .lifecycle
                    .crawls
                    .record_failure(format!("Crawl {id} terminal persistence: {error}"));
            }
        }
        Err(error) => {
            lifecycle::record_owned_crawl_failure(&state, &error);
            guard.status = "failed".into();
            guard.error = Some(error.to_string());
            guard.frontier_available =
                CrawlStore::inspect(state.data_dir.join(format!("{id}.sqlite")))
                    .is_ok_and(|inspection| inspection.config.is_some());
        }
    }
    if let Some(message) = event_failure {
        guard.status = "failed".into();
        guard.error = Some(message);
    }
    // Keep the write lock until the terminal checkpoint write is complete. A status
    // reader or immediate resume must not race this write to the same file.
    let manifest = guard.manifest();
    if let Err(error) = write_json(state.data_dir.join(format!("{id}.job.json")), &manifest).await {
        tracing::error!(%id,%error,"Could not save job state");
        state
            .lifecycle
            .crawls
            .record_failure(format!("Crawl {id} terminal persistence: {error}"));
        let message = format!("Could not save job state: {error}");
        guard.status = "failed".into();
        guard.warnings.push(message.clone());
        guard.error = Some(match guard.error.take() {
            Some(previous) => format!("{previous}; {message}"),
            None => message,
        });
    }
}
fn outcome_status(outcome: CrawlOutcome) -> &'static str {
    match outcome {
        CrawlOutcome::Completed => "completed",
        CrawlOutcome::Paused => "paused",
        CrawlOutcome::Cancelled => "cancelled",
        CrawlOutcome::Limited => "limited",
    }
}
async fn require_report(state: &AppState, id: Uuid) -> ApiResult<CrawlReport> {
    let job = get_job(state, id).await?;
    let guard = job.read().await;
    guard.report.clone().ok_or_else(|| {
        api_error(
            StatusCode::CONFLICT,
            "Wait for a completed, paused, limited or cancelled checkpoint before exporting",
        )
    })
}
async fn advanced_analysis(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let report = require_report(&state, id).await?;
    Ok(Json(crate::analysis::analyze_report(&report)))
}
fn download(body: String, kind: &str, filename: &str) -> Response {
    (
        [
            (header::CONTENT_TYPE, kind.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        body,
    )
        .into_response()
}
async fn export_csv(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Response> {
    let report = require_report(&state, id).await?;
    Ok(download(
        export_pages_csv(&report)
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
        "text/csv; charset=utf-8",
        "pages.csv",
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListInspectRequest {
    content_base64: String,
    #[serde(default)]
    limits: ListImportLimits,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListImportRequest {
    content_base64: String,
    options: ListImportOptions,
}
fn decode_list_input(encoded: &str, limits: &ListImportLimits) -> anyhow::Result<Vec<u8>> {
    limits.validate()?;
    anyhow::ensure!(
        encoded.len() <= limits.max_input_bytes.div_ceil(3) * 4,
        "Encoded input exceeds max_input_bytes"
    );
    let bytes = STANDARD.decode(encoded)?;
    anyhow::ensure!(
        bytes.len() <= limits.max_input_bytes,
        "Input exceeds max_input_bytes"
    );
    Ok(bytes)
}
async fn list_inspect(
    State(state): State<AppState>,
    Json(input): Json<ListInspectRequest>,
) -> ApiResult<Json<Value>> {
    let result = state
        .lifecycle
        .blocking
        .run_blocking(move || {
            let bytes = decode_list_input(&input.content_base64, &input.limits)?;
            inspect_xlsx_list(&bytes, &input.limits)
        })
        .await
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!(result)))
}
async fn list_import(
    State(state): State<AppState>,
    Json(input): Json<ListImportRequest>,
) -> ApiResult<Json<Value>> {
    let result = state
        .lifecycle
        .blocking
        .run_blocking(move || {
            let bytes = decode_list_input(&input.content_base64, &input.options.limits)?;
            import_url_list(&bytes, &input.options)
        })
        .await
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!(result)))
}
async fn export_inputs(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Response> {
    let report = require_report(&state, id).await?;
    Ok(download(
        export_input_occurrences_csv(&report)
            .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?,
        "text/csv; charset=utf-8",
        "inputs.csv",
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SerpRowsRequest {
    rows: Vec<SerpInputRow>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SerpCsvRequest {
    csv: String,
}
async fn serp_preview(
    Json(input): Json<SerpRowsRequest>,
) -> ApiResult<Json<open_seo_spider_core::SerpPreview>> {
    preview_serp(&input.rows)
        .map(Json)
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))
}
async fn serp_import(
    Json(input): Json<SerpCsvRequest>,
) -> ApiResult<Json<open_seo_spider_core::SerpPreview>> {
    let rows =
        import_serp_csv(&input.csv).map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
    preview_serp(&rows)
        .map(Json)
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))
}
async fn serp_export(Json(input): Json<SerpRowsRequest>) -> ApiResult<Response> {
    Ok(download(
        export_serp_csv(&input.rows).map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?,
        "text/csv; charset=utf-8",
        "serp.csv",
    ))
}
async fn robots_simulate(
    State(state): State<AppState>,
    Json(input): Json<RobotsSimulationInput>,
) -> ApiResult<Json<open_seo_spider_core::RobotsSimulation>> {
    state
        .lifecycle
        .blocking
        .run_blocking(move || simulate_robots(&input))
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?
        .map(Json)
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))
}
async fn robots_export(
    State(state): State<AppState>,
    Json(input): Json<RobotsSimulationInput>,
) -> ApiResult<Response> {
    let csv = state
        .lifecycle
        .blocking
        .run_blocking(move || export_robots_simulation_csv(&input))
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
    Ok(download(
        csv,
        "text/csv; charset=utf-8",
        "robots-simulation.csv",
    ))
}
async fn fragment_inventory(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let report = require_report(&state, id).await?;
    Ok(Json(
        json!({"observations":validate_fragments(&report),"enabled":report.config.validate_fragments,"scope":"declared_anchor_fragments_in_captured_documents","unknown_is_not_missing":true}),
    ))
}
async fn export_fragments(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let report = require_report(&state, id).await?;
    Ok(download(
        export_fragments_csv(&report)
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
        "text/csv; charset=utf-8",
        "fragments.csv",
    ))
}
async fn export_dom(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Response> {
    let report = require_report(&state, id).await?;
    Ok(download(
        export_dom_observations_csv(&report)
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
        "text/csv; charset=utf-8",
        "dom-observations.csv",
    ))
}
async fn export_pdf(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Response> {
    Ok(download(
        export_pdf_csv(&require_report(&state, id).await?)
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
        "text/csv; charset=utf-8",
        "pdf.csv",
    ))
}
async fn export_images(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Response> {
    Ok(download(
        export_image_resources_csv(&require_report(&state, id).await?)
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
        "text/csv; charset=utf-8",
        "images.csv",
    ))
}
async fn response_inventory(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let rows = response_observations(&require_report(&state, id).await?);
    Ok(Json(
        json!({"responses":rows,"scope":"captured_native_response_observations","unretained_redirect_headers_and_body":"unknown","document_records_are_separate":true}),
    ))
}
async fn export_responses(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    Ok(download(
        export_response_observations_csv(&require_report(&state, id).await?)
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
        "text/csv; charset=utf-8",
        "responses.csv",
    ))
}
async fn export_issue_csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let report = require_report(&state, id).await?;
    Ok(download(
        export_issues_csv(&report)
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
        "text/csv; charset=utf-8",
        "issues.csv",
    ))
}
async fn export_link_csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let report = require_report(&state, id).await?;
    Ok(download(
        export_links_csv(&report)
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
        "text/csv; charset=utf-8",
        "links.csv",
    ))
}
async fn export_extraction_csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let report = require_report(&state, id).await?;
    Ok(download(
        export_custom_extractions_csv(&report)
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
        "text/csv; charset=utf-8",
        "extractions.csv",
    ))
}
async fn export_xml(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Response> {
    let report = require_report(&state, id).await?;
    Ok(download(
        export_sitemap_xml(&report),
        "application/xml; charset=utf-8",
        "sitemap.xml",
    ))
}
async fn export_javascript_csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let report = require_report(&state, id).await?;
    Ok(download(
        export_custom_javascript_csv(&report)
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
        "text/csv; charset=utf-8",
        "javascript.csv",
    ))
}
async fn export_cookie_csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let report = require_report(&state, id).await?;
    Ok(download(
        export_browser_cookies_csv(&report)
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
        "text/csv; charset=utf-8",
        "cookies.csv",
    ))
}
async fn export_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let report = require_report(&state, id).await?;
    Ok(download(
        serde_json::to_string_pretty(&report)
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?,
        "application/json",
        "crawl.json",
    ))
}
async fn import_report(
    State(state): State<AppState>,
    Json(report): Json<CrawlReport>,
) -> ApiResult<Json<Value>> {
    let id = Uuid::new_v4();
    persist_import(&state, id, report).await?;
    Ok(Json(json!({"id":id,"status":"completed"})))
}
async fn persist_import(state: &AppState, id: Uuid, mut report: CrawlReport) -> ApiResult<()> {
    crate::analysis::extend_report(&mut report);
    write_json(state.data_dir.join(format!("{id}.json")), &report)
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    register_report(state, id, report).await;
    Ok(())
}
async fn register_report(state: &AppState, id: Uuid, report: CrawlReport) {
    let operation = report.operation.clone();
    let policy_ledger = report.policy_ledger.clone();
    let resource_ledger = report.resource_ledger.clone();
    let job = Job {
        id,
        config: report.config.clone(),
        status: if report.summary.cancelled {
            "cancelled"
        } else if report.summary.limited {
            "limited"
        } else {
            "completed"
        }
        .into(),
        started_at: report.summary.started_at.clone(),
        finished_at: Some(report.summary.finished_at.clone()),
        pages: report.pages.clone(),
        issues: report.issues.clone(),
        queued: report.summary.pending,
        discovery_pending: 0,
        in_flight: 0,
        warnings: report.warnings.clone(),
        error: None,
        report: Some(report),
        control: CrawlControl::new(),
        generation: 0,
        checkpoint: None,
        frontier_available: false,
        operation,
        recrawl_override: RecrawlResumeOverride::default(),
        policy_ledger,
        resource_ledger,
    };
    state
        .jobs
        .write()
        .await
        .insert(id, Arc::new(RwLock::new(job)));
}
async fn write_json<T: Serialize>(path: PathBuf, value: &T) -> anyhow::Result<()> {
    let temporary = path.with_extension("tmp");
    tokio::fs::write(&temporary, serde_json::to_vec(value)?).await?;
    tokio::fs::rename(temporary, path).await?;
    Ok(())
}
async fn archive_current_report(state: &AppState, id: Uuid) -> anyhow::Result<()> {
    let current = state.data_dir.join(format!("{id}.json"));
    let bytes = match tokio::fs::read(current).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let directory = state.data_dir.join("snapshots").join(id.to_string());
    tokio::fs::create_dir_all(&directory).await?;
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(format!("before-resume-{}.json", Uuid::new_v4())))
        .await?;
    file.write_all(&bytes).await?;
    file.sync_all().await?;
    Ok(())
}
async fn save_revision(
    state: &AppState,
    id: Uuid,
    generation: u64,
    report: &CrawlReport,
) -> anyhow::Result<()> {
    let directory = state.data_dir.join("snapshots").join(id.to_string());
    tokio::fs::create_dir_all(&directory).await?;
    let path = directory.join(format!("generation-{generation}.json"));
    let bytes = serde_json::to_vec(report)?;
    use tokio::io::AsyncWriteExt;
    match tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .await
    {
        Ok(mut file) => {
            file.write_all(&bytes).await?;
            file.sync_all().await?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            anyhow::ensure!(
                tokio::fs::read(path).await? == bytes,
                "Immutable report generation already contains different bytes"
            );
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
async fn save_manifest(state: &AppState, job: &Arc<RwLock<Job>>) -> anyhow::Result<()> {
    let guard = job.read().await;
    let manifest = guard.manifest();
    drop(guard);
    write_json(
        state.data_dir.join(format!("{}.job.json", manifest.id)),
        &manifest,
    )
    .await
}
async fn load_reports(state: &AppState) -> anyhow::Result<()> {
    let mut paths = Vec::new();
    let mut entries = tokio::fs::read_dir(state.data_dir.as_ref()).await?;
    while let Some(entry) = entries.next_entry().await? {
        paths.push(entry.path());
    }
    for path in &paths {
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = path
            .file_stem()
            .and_then(|value| value.to_str())
            .and_then(|value| Uuid::parse_str(value).ok())
        else {
            continue;
        };
        let report = match tokio::fs::read(path).await {
            Ok(bytes) => {
                tokio::task::spawn_blocking(move || serde_json::from_slice::<CrawlReport>(&bytes))
                    .await?
                    .ok()
            }
            Err(_) => None,
        };
        match report {
            Some(report) => {
                // Loading a historical snapshot is read-only. Importing a new
                // report is a separate operation that may derive and persist
                // current analysis; startup must not overwrite stored bytes.
                register_report(state, id, report).await;
            }
            None => tracing::warn!(path=?path,"Ignoring invalid saved crawl"),
        }
    }
    let mut manifests = HashMap::new();
    for path in &paths {
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".job.json"))
        {
            continue;
        }
        let bytes = tokio::fs::read(path).await?;
        let manifest: JobManifest =
            match tokio::task::spawn_blocking(move || serde_json::from_slice(&bytes)).await? {
                Ok(value) => value,
                Err(error) => {
                    tracing::warn!(path=?path,%error,"Ignoring invalid job manifest");
                    continue;
                }
            };
        manifests.insert(manifest.id, manifest);
    }
    for path in &paths {
        if path.extension().and_then(|value| value.to_str()) != Some("sqlite") {
            continue;
        }
        let Some(id) = path
            .file_stem()
            .and_then(|value| value.to_str())
            .and_then(|value| Uuid::parse_str(value).ok())
        else {
            continue;
        };
        let recovery_path = path.clone();
        let inspection =
            match tokio::task::spawn_blocking(move || CrawlStore::inspect(recovery_path)).await? {
                Ok(inspection) => inspection,
                Err(error) => {
                    tracing::warn!(path=?path,%error,"Ignoring invalid saved frontier");
                    continue;
                }
            };
        let Some(config) = inspection.config else {
            continue;
        };
        let manifest = manifests.remove(&id);
        // Completed legacy snapshots remain the source of their historical
        // representation. New checkpoints are reconciled against SQLite.
        if inspection.run_state.is_none()
            && state.jobs.read().await.contains_key(&id)
            && manifest.as_ref().is_none_or(|manifest| {
                matches!(
                    manifest.status.as_str(),
                    "completed" | "cancelled" | "limited"
                )
            })
        {
            if let Some(job) = state.jobs.read().await.get(&id).cloned() {
                job.write().await.frontier_available = true;
            }
            continue;
        }
        let recovery_path = path.clone();
        let checkpoint_valid = inspection.checkpoint_valid;
        // SQLite, typed snapshot parsing and derived analysis are synchronous
        // recovery work. Keep them off the async startup poll's Windows stack.
        let (report, pages) = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
            let store = CrawlStore::open_read_only(recovery_path)?;
            let mut report = if checkpoint_valid {
                store.load_report()?
            } else {
                None
            };
            if let Some(report) = &mut report {
                // In-memory projection only; never rewrite historical JSON.
                crate::analysis::extend_report(report);
            }
            let pages = if let Some(report) = &report {
                report.pages.clone()
            } else {
                store.pages()?
            };
            Ok((report, pages))
        })
        .await??;
        let status = if inspection.checkpoint_valid {
            inspection
                .checkpoint
                .as_ref()
                .map(|checkpoint| outcome_status(checkpoint.outcome))
                .unwrap_or("interrupted")
        } else {
            "interrupted"
        };
        let started_at = inspection
            .run_state
            .as_ref()
            .map(|run| run.started_at.clone())
            .or_else(|| {
                manifest
                    .as_ref()
                    .map(|manifest| manifest.started_at.clone())
            })
            .or_else(|| {
                report
                    .as_ref()
                    .map(|report| report.summary.started_at.clone())
            })
            .unwrap_or_default();
        let finished_at = inspection
            .run_state
            .as_ref()
            .and_then(|run| run.finished_at.clone());
        let generation = inspection.run_state.as_ref().map_or_else(
            || manifest.as_ref().map_or(0, |manifest| manifest.generation),
            |run| run.generation,
        );
        let issues = report
            .as_ref()
            .map(|report| report.issues.clone())
            .unwrap_or_default();
        let mut warnings = report
            .as_ref()
            .map(|report| report.warnings.clone())
            .unwrap_or_default();
        if status == "interrupted" {
            warnings.push("The previous process stopped before a terminal checkpoint. Resume uses committed SQLite work; uncommitted requests may repeat.".into());
        }
        let job = Job {
            id,
            config,
            status: status.into(),
            started_at,
            finished_at,
            pages,
            issues,
            queued: inspection.pending,
            discovery_pending: inspection.discovery_pending,
            in_flight: 0,
            warnings,
            error: None,
            report,
            control: CrawlControl::new(),
            generation,
            checkpoint: if inspection.checkpoint_valid {
                inspection.checkpoint
            } else {
                None
            },
            frontier_available: true,
            operation: inspection.operation,
            recrawl_override: RecrawlResumeOverride::default(),
            policy_ledger: inspection.policy_ledger,
            resource_ledger: inspection.resource_ledger,
        };
        state
            .jobs
            .write()
            .await
            .insert(id, Arc::new(RwLock::new(job)));
    }
    for (id, manifest) in manifests {
        if let Some(job) = state.jobs.read().await.get(&id).cloned() {
            if matches!(
                manifest.status.as_str(),
                "paused"
                    | "pausing"
                    | "running"
                    | "cancelling"
                    | "interrupted"
                    | "limited"
                    | "failed"
            ) {
                let mut guard = job.write().await;
                guard.status = "failed".into();
                guard.generation = manifest.generation;
                guard.checkpoint = None;
                guard.frontier_available = false;
                guard.error = Some(format!(
                    "Previous state '{}' has no valid SQLite frontier. The captured historical report remains available; start a new crawl.",
                    manifest.status
                ));
            }
            continue;
        }
        let job = Job {
            id,
            config: manifest.config,
            status: "failed".into(),
            started_at: manifest.started_at,
            finished_at: manifest.finished_at,
            pages: Vec::new(),
            issues: Vec::new(),
            queued: 0,
            discovery_pending: 0,
            in_flight: 0,
            warnings: Vec::new(),
            error: Some(
                "Saved job metadata has no valid SQLite frontier; start a new crawl.".into(),
            ),
            report: None,
            control: CrawlControl::new(),
            generation: manifest.generation,
            checkpoint: None,
            frontier_available: false,
            operation: None,
            recrawl_override: RecrawlResumeOverride::default(),
            policy_ledger: None,
            resource_ledger: None,
        };
        state
            .jobs
            .write()
            .await
            .insert(id, Arc::new(RwLock::new(job)));
    }
    Ok(())
}
#[derive(Deserialize)]
struct CompareQuery {
    before: Uuid,
    after: Uuid,
}
async fn compare(
    State(state): State<AppState>,
    Query(query): Query<CompareQuery>,
) -> ApiResult<Json<Value>> {
    let before = require_report(&state, query.before).await?;
    let after = require_report(&state, query.after).await?;
    Ok(Json(json!({"changes":compare_crawls(&before,&after)})))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uploaded_list_config_retains_occurrences_and_rejects_forged_targets() {
        let parsed = import_url_list(
            b"https://example.test/a#one\nnot a URL\nhttps://example.test/a#one\n",
            &ListImportOptions {
                format: open_seo_spider_core::ListImportFormat::Txt,
                limits: ListImportLimits::default(),
            },
        )
        .unwrap();
        let config = config_from_input(json!({"mode":"list","input_imports":[parsed]})).unwrap();
        assert_eq!(config.seed_urls, vec!["https://example.test/a"]);
        assert_eq!(config.input_imports[0].rows.len(), 3);
        assert_eq!(config.input_imports[0].rows[0].source_index, 1);
        assert!(config.input_imports[0].rows[1].error.is_some());
        let mut forged = config.input_imports[0].clone();
        forged.rows[0].normalized_url = Some("https://other.test/stolen".into());
        assert!(config_from_input(json!({"input_imports":[forged]})).is_err());
    }
    #[test]
    fn xml_index_entries_prepare_sitemap_discovery_without_page_seeds() {
        let parsed=import_url_list(br#"<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc>https://example.test/nested.xml</loc></sitemap></sitemapindex>"#, &ListImportOptions {format:open_seo_spider_core::ListImportFormat::Xml,limits:ListImportLimits::default()}).unwrap();
        let config = config_from_input(json!({"input_imports":[parsed],"mode":"list"})).unwrap();
        assert!(config.seed_urls.is_empty());
        assert_eq!(config.sitemap_urls, vec!["https://example.test/nested.xml"]);
    }
    #[test]
    fn encoded_upload_limits_are_checked_before_decode() {
        let limits = ListImportLimits {
            max_input_bytes: 2,
            ..Default::default()
        };
        assert_eq!(decode_list_input("YWI=", &limits).unwrap(), b"ab");
        assert!(decode_list_input("YWJj", &limits).is_err());
        assert!(decode_list_input("YWI=YWI=", &limits).is_err());
        assert!(decode_list_input("not base64!", &limits).is_err());
        assert!(
            decode_list_input(
                "",
                &ListImportLimits {
                    max_input_bytes: 0,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn direct_seed_raw_controls_cannot_create_ambiguous_import_targets() {
        assert!(config_from_input(json!({"seed_urls":["https://example.test/a\nb"]})).is_err());
    }
    #[test]
    fn ui_config_aliases_do_not_introduce_a_url_limit() {
        let config=config_from_input(json!({"seed_url":"https://example.com","mode":"list","max_pages":0,"max_depth":0,"follow_subdomains":true})).unwrap();
        assert_eq!(config.max_pages, None);
        assert_eq!(config.max_depth, None);
        assert!(!config.crawl_links);
        assert!(config.include_subdomains);
    }
}

// Append as a sibling cfg(test) module in crates/app/src/server.rs.
// AUTHORED ONLY; calls pure config DTO normalization, never launch/HTTP.
#[cfg(test)]
mod explicit_sitemap_dto_regressions {
    use super::*;

    #[test]
    fn create_dto_captures_original_explicit_duplicates_before_imported_sitemaps() {
        let imported = import_url_list(
            br#"<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc>https://example.test/imported.xml</loc></sitemap></sitemapindex>"#,
            &ListImportOptions {
                format: open_seo_spider_core::ListImportFormat::Xml,
                limits: ListImportLimits::default(),
            },
        ).unwrap();
        let original = vec![
            "HTTPS://EXAMPLE.TEST/explicit.xml#one".to_owned(),
            "HTTPS://EXAMPLE.TEST/explicit.xml#one".to_owned(),
        ];
        let config = config_from_input(json!({
            "seed_urls":["https://example.test/"],
            "sitemap_urls":original,
            "input_imports":[imported]
        }))
        .unwrap();
        assert_eq!(config.explicit_sitemap_inputs, Some(original));
        assert!(
            config
                .sitemap_urls
                .contains(&"https://example.test/imported.xml".to_owned())
        );
        let repeated = config_from_input(serde_json::to_value(&config).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(repeated).unwrap(),
            serde_json::to_value(config).unwrap()
        );
    }

    #[test]
    fn create_import_only_dto_is_known_empty_while_saved_validation_remains_unknown() {
        let imported = import_url_list(
            br#"<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc>https://example.test/imported.xml</loc></sitemap></sitemapindex>"#,
            &ListImportOptions {
                format: open_seo_spider_core::ListImportFormat::Xml,
                limits: ListImportLimits::default(),
            },
        ).unwrap();
        let config = config_from_input(json!({"input_imports":[imported]})).unwrap();
        assert_eq!(config.explicit_sitemap_inputs, Some(vec![]));
        let mut legacy = config;
        legacy.explicit_sitemap_inputs = None;
        let preserved = Crawler::from_saved_config(legacy).unwrap();
        assert_eq!(preserved.effective_config().explicit_sitemap_inputs, None);
    }
}
