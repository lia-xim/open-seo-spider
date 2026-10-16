use super::*;
use axum::body::to_bytes;
use open_seo_spider_core::{Crawler, ResourceDiscoveryConfig};
use std::sync::atomic::{AtomicUsize, Ordering};

fn state(dir: PathBuf, report: CrawlReport, generation: u64) -> (AppState, Uuid) {
    let id = Uuid::new_v4();
    let job = Job {
        id,
        config: report.config.clone(),
        status: "completed".into(),
        started_at: report.summary.started_at.clone(),
        finished_at: Some(report.summary.finished_at.clone()),
        pages: vec![],
        issues: vec![],
        queued: 0,
        discovery_pending: 0,
        in_flight: 0,
        warnings: vec![],
        error: None,
        resource_ledger: report.resource_ledger.clone(),
        report: Some(report),
        control: CrawlControl::new(),
        generation,
        checkpoint: None,
        frontier_available: true,
        operation: None,
        recrawl_override: Default::default(),
        quota_increases: None,
        policy_ledger: None,
    };
    (
        AppState {
            mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
            jobs: Arc::new(RwLock::new(HashMap::from([(
                id,
                Arc::new(RwLock::new(job)),
            )]))),
            data_dir: Arc::new(dir),
            exports: Default::default(),
            finding_reads: Default::default(),
            lifecycle: Default::default(),
        },
        id,
    )
}
fn response(result: ApiResult<Response>) -> Response {
    result.unwrap_or_else(|(status, Json(error))| panic!("{status}: {}", error.error))
}
async fn value(response: Response) -> Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), workflow::JSON_BYTES)
            .await
            .unwrap(),
    )
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resource_sources_api_real_saved_css_over200_full_csv_stale_and_cli_reopen_do_no_new_website_io()
 {
    let requests = Arc::new(AtomicUsize::new(0));
    let seen = requests.clone();
    let css = (0..205)
        .map(|n| format!(".n{n}{{background:url('/bad.png')}}\n"))
        .collect::<String>();
    let site=Router::new().route("/",get(move||{let seen=seen.clone();async move{seen.fetch_add(1,Ordering::SeqCst);([(header::CONTENT_TYPE,"text/html")],"<html><head><link rel='stylesheet' href='/source.css'></head><body>owned fixture</body></html>")}}));
    let seen = requests.clone();
    let site = site.route(
        "/source.css",
        get(move || {
            let seen = seen.clone();
            let css = css.clone();
            async move {
                seen.fetch_add(1, Ordering::SeqCst);
                ([(header::CONTENT_TYPE, "text/css")], css)
            }
        }),
    );
    let seen = requests.clone();
    let site = site.route(
        "/bad.png",
        get(move || {
            let seen = seen.clone();
            async move {
                seen.fetch_add(1, Ordering::SeqCst);
                (
                    StatusCode::NOT_FOUND,
                    [(header::CONTENT_TYPE, "image/png")],
                    "owned missing resource",
                )
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let website = tokio::spawn(async move {
        axum::serve(listener, site)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let captured = temp.path().join("captured.sqlite");
    let mut resources = ResourceDiscoveryConfig::default();
    resources.resources.images.store = false;
    let config = CrawlConfig {
        seed_urls: vec![origin],
        resource_discovery: Some(resources),
        crawl_links: false,
        discover_sitemaps: false,
        obey_robots: false,
        concurrency: 1,
        request_timeout_secs: 5,
        ..Default::default()
    };
    let crawler = Crawler::new(config).unwrap().with_store(&captured).unwrap();
    let report = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        crawler.run_controlled(None, CrawlControl::new()),
    )
    .await
    .unwrap()
    .unwrap()
    .report;
    drop(crawler);
    let database = CrawlStore::open_read_only(&captured).unwrap();
    let generation = database.checkpoint().unwrap().unwrap().generation;
    drop(database);
    let (state, id) = state(temp.path().to_path_buf(), report, generation);
    let database_path = temp.path().join(format!("{id}.sqlite"));
    // Move the completed original database through SQLite backup-independent
    // file rename only after the crawler and all Store connections are closed.
    tokio::fs::rename(&captured, &database_path).await.unwrap();
    let baseline = requests.load(Ordering::SeqCst);
    assert_eq!(
        baseline, 3,
        "Fixture must observe the actual HTML/CSS/missing-image crawl"
    );
    let mut request = Request {
        filter: open_seo_spider_core::ResourceBrokenSourcesFilter::Failures,
        limit: Some(200),
        expected_generation: Some(generation),
        ..Default::default()
    };
    let first = value(response(
        preview(State(state.clone()), Path(id), Ok(Json(request.clone()))).await,
    ))
    .await;
    assert_eq!(first["scope"], workflow::SCOPE);
    assert_eq!(first["page"]["rows"].as_array().unwrap().len(), 200);
    assert_eq!(first["page"]["selected_evaluation_rows"], 205);
    assert_eq!(first["page"]["website_requests"], 0);
    assert_eq!(
        first["page"]["snapshot"]["source_kind_coverage_complete"],
        false
    );
    let token = first["snapshot_token"].as_str().unwrap().to_owned();
    request.expected_snapshot_token = Some(token.clone());
    request.after_sequence = first["page"]["next_sequence"].as_u64();
    let second = value(response(
        preview(State(state.clone()), Path(id), Ok(Json(request.clone()))).await,
    ))
    .await;
    assert_eq!(second["page"]["rows"].as_array().unwrap().len(), 5);
    assert_eq!(second["page"]["has_more"], false);
    assert_eq!(second["snapshot_token"], token);
    assert_eq!(
        csv(State(state.clone()), Path(id), Ok(Json(request.clone())))
            .await
            .err()
            .unwrap()
            .0,
        StatusCode::BAD_REQUEST
    );
    request.after_sequence = None;
    let download = response(csv(State(state.clone()), Path(id), Ok(Json(request.clone()))).await);
    let len = download.headers()[header::CONTENT_LENGTH]
        .to_str()
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let digest = download.headers()["x-content-sha256"]
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = to_bytes(download.into_body(), workflow::CSV_BYTES)
        .await
        .unwrap();
    assert_eq!(bytes.len(), len);
    assert_eq!(workflow::bytes_sha(&bytes), digest);
    assert_eq!(
        std::str::from_utf8(&bytes)
            .unwrap()
            .matches("\"observed_http_failure\"")
            .count(),
        205
    );
    let mut limited = request.clone();
    limited.expected_snapshot_token = None;
    limited.limits.max_evaluation_rows = 1;
    assert_eq!(
        preview(State(state.clone()), Path(id), Ok(Json(limited)))
            .await
            .err()
            .unwrap()
            .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let output = temp.path().join("resource-page.json");
    let csv_path = temp.path().join("resource-full.csv");
    let options = temp.path().join("resource-request.json");
    tokio::fs::write(&options, br#"{"filter":"failures","limit":200}"#)
        .await
        .unwrap();
    workflow::run_cli(
        database_path.clone(),
        Some(options),
        output.clone(),
        Some(csv_path.clone()),
    )
    .await
    .unwrap();
    let cli: Value = serde_json::from_slice(&tokio::fs::read(output).await.unwrap()).unwrap();
    assert_eq!(cli["rows"].as_array().unwrap().len(), 200);
    assert_eq!(cli["selected_evaluation_rows"], 205);
    assert_eq!(tokio::fs::read(csv_path).await.unwrap(), bytes.as_ref());
    let job = get_job(&state, id)
        .await
        .unwrap_or_else(|(status, Json(error))| panic!("{status}: {}", error.error));
    job.write().await.generation += 1;
    assert_eq!(
        preview(State(state.clone()), Path(id), Ok(Json(request.clone())))
            .await
            .err()
            .unwrap()
            .0,
        StatusCode::CONFLICT
    );
    job.write().await.generation = generation;
    // A same-generation committed transport edit invalidates the complete set
    // digest, even though no new request is sent to the owned website.
    let connection = rusqlite::Connection::open(&database_path).unwrap();
    let url: String = connection
        .query_row(
            "SELECT url FROM resource_transports ORDER BY url LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let raw: String = connection
        .query_row(
            "SELECT data FROM resource_transports WHERE url=?1",
            [&url],
            |r| r.get(0),
        )
        .unwrap();
    let mut observation: Value = serde_json::from_str(&raw).unwrap();
    observation["projection"]["headers"]["x-offline-witness"] = json!("changed without HTTP");
    let raw = serde_json::to_string(&observation).unwrap();
    connection
        .execute(
            "UPDATE resource_transports SET data=?1,payload_sha256=?2 WHERE url=?3",
            rusqlite::params![raw, workflow::bytes_sha(raw.as_bytes()), url],
        )
        .unwrap();
    drop(connection);
    assert_eq!(
        preview(State(state.clone()), Path(id), Ok(Json(request.clone())))
            .await
            .err()
            .unwrap()
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        csv(State(state.clone()), Path(id), Ok(Json(request)))
            .await
            .err()
            .unwrap()
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        requests.load(Ordering::SeqCst),
        baseline,
        "Every saved read/preview/page/export/CLI operation must be offline"
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
    state
        .lifecycle
        .response_streams
        .shutdown_and_drain()
        .await
        .unwrap();
    stop.send(()).unwrap();
    website.await.unwrap();
}

#[tokio::test]
async fn resource_sources_api_unavailable_active_config_and_snapshot_generation_authority_are_explicit()
 {
    let temp = tempfile::tempdir().unwrap();
    let report = CrawlReport {
        config: Default::default(),
        pages: vec![],
        issues: vec![],
        summary: Default::default(),
        warnings: vec![],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
        sitemap_source_measurements: None,
    };
    let (state, id) = state(temp.path().to_path_buf(), report, 7);
    let page = value(response(
        preview(State(state.clone()), Path(id), Ok(Json(Request::default()))).await,
    ))
    .await;
    assert_eq!(page["page"]["available"], false);
    assert_eq!(
        page["page"]["unavailable_reason"],
        "native_resource_discovery_disabled"
    );
    assert!(page["snapshot_token"].is_null());
    let request = Request {
        expected_snapshot_token: Some(workflow::api_token(id, 7, &"a".repeat(64))),
        ..Default::default()
    };
    assert_eq!(
        preview(State(state.clone()), Path(id), Ok(Json(request)))
            .await
            .err()
            .unwrap()
            .0,
        StatusCode::CONFLICT
    );
    let job = get_job(&state, id)
        .await
        .unwrap_or_else(|(status, Json(error))| panic!("{status}: {}", error.error));
    job.write().await.status = "running".into();
    assert_eq!(
        preview(State(state.clone()), Path(id), Ok(Json(Request::default())))
            .await
            .err()
            .unwrap()
            .0,
        StatusCode::CONFLICT
    );
    job.write().await.status = "completed".into();
    let metadata = json!({"crawl_generation":7,"crawl_config_sha256":"wrong"});
    assert_eq!(
        check_job(&*job.read().await, &metadata, &Request::default())
            .err()
            .unwrap()
            .0,
        StatusCode::CONFLICT
    );
}
