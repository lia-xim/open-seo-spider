use axum::{
    Json, Router,
    extract::{Path, Query, Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use open_seo_spider_core::{
    AuditIssue, CrawlConfig, CrawlEvent, CrawlReport, CrawlStore, Crawler, PageRecord,
    compare_crawls, export_custom_extractions_csv, export_issues_csv, export_links_csv,
    export_pages_csv, export_sitemap_xml,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    mcp: crate::mcp::Bridge,
    jobs: Arc<RwLock<HashMap<Uuid, Arc<RwLock<Job>>>>>,
    data_dir: Arc<PathBuf>,
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
    in_flight: usize,
    warnings: Vec<String>,
    error: Option<String>,
    report: Option<CrawlReport>,
    cancel: Arc<AtomicBool>,
}
#[derive(Serialize, Deserialize)]
struct JobManifest {
    id: Uuid,
    config: CrawlConfig,
    status: String,
    started_at: String,
    finished_at: Option<String>,
}
impl Job {
    fn status(&self) -> Value {
        let failed = self
            .pages
            .iter()
            .filter(|page| {
                page.error.is_some() || page.final_status.is_some_and(|status| status >= 400)
            })
            .count();
        json!({"id":self.id,"status":self.status,"seed_url":self.config.seed_urls.first(),"config":self.config,"started_at":self.started_at,"finished_at":self.finished_at,"stats":{"discovered":self.pages.len()+self.queued+self.in_flight,"queued":self.queued,"fetched":self.pages.len(),"failed":failed,"issues":self.issues.len(),"in_flight":self.in_flight},"warnings":self.warnings,"error":self.error,"summary":self.report.as_ref().map(|report| &report.summary)})
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
    let listener = tokio::net::TcpListener::bind(listen).await?;
    let address = listener.local_addr()?;
    let mcp_host = if address.is_ipv6() {
        "[::1]"
    } else {
        "127.0.0.1"
    };
    tokio::fs::create_dir_all(&data_dir).await?;
    let state = AppState {
        mcp: crate::mcp::Bridge::new(&format!("http://{mcp_host}:{}", address.port()))?,
        jobs: Arc::new(RwLock::new(HashMap::new())),
        data_dir: Arc::new(data_dir),
    };
    load_reports(&state).await?;
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
        .route("/api/capabilities", get(capabilities))
        .route("/mcp", post(mcp_http))
        .route("/api/crawls", get(list_crawls).post(create_crawl))
        .route("/api/crawls/import", post(import_report))
        .route("/api/crawls/{id}", get(crawl_status))
        .route("/api/crawls/{id}/pages", get(pages))
        .route("/api/crawls/{id}/issues", get(issues))
        .route("/api/crawls/{id}/analysis", get(advanced_analysis))
        .route("/api/crawls/{id}/cancel", post(cancel_crawl))
        .route("/api/crawls/{id}/resume", post(resume_crawl))
        .route("/api/crawls/{id}/export.csv", get(export_csv))
        .route("/api/crawls/{id}/issues.csv", get(export_issue_csv))
        .route("/api/crawls/{id}/links.csv", get(export_link_csv))
        .route(
            "/api/crawls/{id}/extractions.csv",
            get(export_extraction_csv),
        )
        .route("/api/crawls/{id}/sitemap.xml", get(export_xml))
        .route("/api/crawls/{id}/project.json", get(export_project))
        .route("/api/compare", get(compare))
        .layer(axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024))
        .layer(middleware::from_fn(local_origin))
        .with_state(state);
    println!("Open SEO Spider: http://{}", listener.local_addr()?);
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
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
        .is_some_and(|authority| matches!(authority.host(), "localhost" | "127.0.0.1" | "[::1]"));
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
    Json(
        json!({"product":"Open SEO Spider","crawl":{"http":true,"list":true,"sitemap":true,"robots":true,"redirects":true,"cancel":true,"resume":true,"url_limit":null,"url_regex_filters":true,"css_extraction":true},"optional_worker":{"javascript_rendering":true,"screenshots":true,"accessibility_axe":true,"endpoint":"http://127.0.0.1:8790/render","enabled_by_default":false},"mcp":{"stdio":true,"streamable_http":"/mcp"},"exports":["pages_csv","issues_csv","links_csv","extractions_csv","sitemap_xml","project_json"],"unimplemented":["custom_javascript","xpath_extraction","api_integrations","spelling_grammar","pdf_analysis","embedding_semantic_search","scheduling","full_structured_data_validation"],"parity":"in progress; see docs/feature-parity.json"}),
    )
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
    Crawler::new(config.clone())?;
    Ok(config)
}
async fn create_crawl(
    State(state): State<AppState>,
    Json(value): Json<Value>,
) -> ApiResult<Json<Value>> {
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
        in_flight: 0,
        warnings: Vec::new(),
        error: None,
        report: None,
        cancel: Arc::new(AtomicBool::new(false)),
    }));
    save_manifest(&state, &job)
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    state.jobs.write().await.insert(id, job.clone());
    launch(state, job).await;
    Ok(Json(json!({"id":id,"status":"running"})))
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
    if guard.status != "running" {
        return Err(api_error(StatusCode::CONFLICT, "Crawl is not running"));
    }
    guard.cancel.store(true, Ordering::Relaxed);
    guard.status = "cancelling".into();
    Ok(Json(guard.status()))
}
async fn resume_crawl(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let job = get_job(&state, id).await?;
    if !state.data_dir.join(format!("{id}.sqlite")).exists()
        && !state.data_dir.join(format!("{id}.job.json")).exists()
    {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Imported snapshots have no saved crawl frontier; start a new crawl",
        ));
    }
    {
        let mut guard = job.write().await;
        if guard.status == "running" || guard.status == "cancelling" {
            return Err(api_error(StatusCode::CONFLICT, "Crawl is already active"));
        }
        let manifest = JobManifest {
            id,
            config: guard.config.clone(),
            status: "running".into(),
            started_at: guard.started_at.clone(),
            finished_at: None,
        };
        write_json(state.data_dir.join(format!("{id}.job.json")), &manifest)
            .await
            .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
        guard.cancel = Arc::new(AtomicBool::new(false));
        guard.status = "running".into();
        guard.finished_at = None;
        guard.error = None;
        guard.report = None;
    }
    launch(state, job).await;
    Ok(Json(json!({"id":id,"status":"running"})))
}
async fn launch(state: AppState, job: Arc<RwLock<Job>>) {
    tokio::spawn(async move {
        let (config, id, cancel) = {
            let guard = job.read().await;
            (guard.config.clone(), guard.id, guard.cancel.clone())
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
            while let Some(event) = receiver.recv().await {
                let mut guard = event_job.write().await;
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
                    CrawlEvent::Finished { .. } => {}
                }
            }
        });
        let result = match Crawler::new(config) {
            Ok(crawler) => match crawler.with_store(state.data_dir.join(format!("{id}.sqlite"))) {
                Ok(crawler) => crawler.run(Some(sender.clone()), cancel).await,
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };
        drop(sender);
        let _ = event_task.await;
        let mut guard = job.write().await;
        guard.queued = 0;
        guard.in_flight = 0;
        guard.finished_at = Some(chrono::Utc::now().to_rfc3339());
        match result {
            Ok(mut report) => {
                crate::analysis::extend_report(&mut report);
                guard.status = if report.summary.cancelled {
                    "cancelled"
                } else {
                    "completed"
                }
                .into();
                guard.pages = report.pages.clone();
                guard.issues = report.issues.clone();
                guard.warnings = report.warnings.clone();
                guard.report = Some(report.clone());
                let path = state.data_dir.join(format!("{id}.json"));
                if let Err(error) = write_json(path, &report).await {
                    guard.error = Some(format!("Could not save report: {error}"));
                }
            }
            Err(error) => {
                guard.status = "failed".into();
                guard.error = Some(error.to_string());
            }
        }
        drop(guard);
        if let Err(error) = save_manifest(&state, &job).await {
            tracing::error!(%id,%error,"Could not save job state");
        }
    });
}
async fn require_report(state: &AppState, id: Uuid) -> ApiResult<CrawlReport> {
    let job = get_job(state, id).await?;
    let guard = job.read().await;
    guard.report.clone().ok_or_else(|| {
        api_error(
            StatusCode::CONFLICT,
            "Wait for the crawl to finish or cancel it before exporting",
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
    let job = Job {
        id,
        config: report.config.clone(),
        status: if report.summary.cancelled {
            "cancelled"
        } else {
            "completed"
        }
        .into(),
        started_at: report.summary.started_at.clone(),
        finished_at: Some(report.summary.finished_at.clone()),
        pages: report.pages.clone(),
        issues: report.issues.clone(),
        queued: 0,
        in_flight: 0,
        warnings: report.warnings.clone(),
        error: None,
        report: Some(report.clone()),
        cancel: Arc::new(AtomicBool::new(false)),
    };
    write_json(state.data_dir.join(format!("{id}.json")), &report)
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    state
        .jobs
        .write()
        .await
        .insert(id, Arc::new(RwLock::new(job)));
    Ok(())
}
async fn write_json<T: Serialize>(path: PathBuf, value: &T) -> anyhow::Result<()> {
    let temporary = path.with_extension("tmp");
    tokio::fs::write(&temporary, serde_json::to_vec(value)?).await?;
    tokio::fs::rename(temporary, path).await?;
    Ok(())
}
async fn save_manifest(state: &AppState, job: &Arc<RwLock<Job>>) -> anyhow::Result<()> {
    let guard = job.read().await;
    let manifest = JobManifest {
        id: guard.id,
        config: guard.config.clone(),
        status: guard.status.clone(),
        started_at: guard.started_at.clone(),
        finished_at: guard.finished_at.clone(),
    };
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
        match tokio::fs::read(path)
            .await
            .ok()
            .and_then(|bytes| serde_json::from_slice::<CrawlReport>(&bytes).ok())
        {
            Some(report) => {
                persist_import(state, id, report)
                    .await
                    .map_err(|error| anyhow::anyhow!(error.1.0.error))?;
            }
            None => tracing::warn!(path=?path,"Ignoring invalid saved crawl"),
        }
    }
    for path in &paths {
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".job.json"))
        {
            continue;
        }
        let manifest: JobManifest = match serde_json::from_slice(&tokio::fs::read(path).await?) {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(path=?path,%error,"Ignoring invalid job manifest");
                continue;
            }
        };
        if state.jobs.read().await.contains_key(&manifest.id)
            && matches!(manifest.status.as_str(), "completed" | "cancelled")
        {
            continue;
        }
        let database = state.data_dir.join(format!("{}.sqlite", manifest.id));
        let (pages, queued) = if database.exists() {
            let store = CrawlStore::open(database)?;
            (store.pages()?, store.pending_count()?)
        } else {
            (Vec::new(), 0)
        };
        let job=Job{id:manifest.id,config:manifest.config,status:"interrupted".into(),started_at:manifest.started_at,finished_at:None,pages,issues:Vec::new(),queued,in_flight:0,warnings:vec!["The previous process stopped before this crawl finished. Resume uses the saved SQLite frontier.".into()],error:None,report:None,cancel:Arc::new(AtomicBool::new(false))};
        state
            .jobs
            .write()
            .await
            .insert(manifest.id, Arc::new(RwLock::new(job)));
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
    fn ui_config_aliases_do_not_introduce_a_url_limit() {
        let config=config_from_input(json!({"seed_url":"https://example.com","mode":"list","max_pages":0,"max_depth":0,"follow_subdomains":true})).unwrap();
        assert_eq!(config.max_pages, None);
        assert_eq!(config.max_depth, None);
        assert!(!config.crawl_links);
        assert!(config.include_subdomains);
    }
}
