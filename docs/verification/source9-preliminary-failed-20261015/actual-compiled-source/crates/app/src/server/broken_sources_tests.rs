use super::*;
use crate::broken_sources::Filter;
use axum::body::to_bytes;
use open_seo_spider_core::{CrawlSummary, LinkRecord, PageRecord};
fn job(id: Uuid, url: &str) -> Job {
    let mut source = PageRecord {
        url: format!("{url}/source"),
        final_url: format!("{url}/source"),
        status: Some(200),
        final_status: Some(200),
        fetched_at: "2026-10-15T12:00:05Z".into(),
        ..Default::default()
    };
    source.links = vec![
        LinkRecord {
            url: format!("{url}/broken"),
            text: "first, ä".into(),
            kind: "anchor".into(),
            ..Default::default()
        },
        LinkRecord {
            url: format!("{url}/broken"),
            text: "second".into(),
            kind: "anchor".into(),
            ..Default::default()
        },
    ];
    let target = PageRecord {
        url: format!("{url}/broken"),
        final_url: format!("{url}/broken"),
        status: Some(404),
        final_status: Some(404),
        fetched_at: "2026-10-15T12:00:06Z".into(),
        ..Default::default()
    };
    Job {
        id,
        config: Default::default(),
        status: "completed".into(),
        started_at: String::new(),
        finished_at: None,
        pages: vec![],
        issues: vec![],
        queued: 0,
        discovery_pending: 0,
        in_flight: 0,
        warnings: vec![],
        error: None,
        report: Some(CrawlReport {
            config: Default::default(),
            pages: vec![source, target],
            issues: vec![],
            summary: CrawlSummary {
                started_at: "2026-10-15T12:00:00Z".into(),
                finished_at: "2026-10-15T12:00:10Z".into(),
                ..Default::default()
            },
            warnings: vec![],
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
            sitemap_catalog: None,
            sitemap_coverage_inventory: None,
            sitemap_source_measurements: None,
        }),
        control: CrawlControl::new(),
        generation: 7,
        checkpoint: None,
        frontier_available: true,
        operation: None,
        recrawl_override: Default::default(),
        policy_ledger: None,
        resource_ledger: None,
    }
}
fn state(job: Job) -> AppState {
    AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Arc::new(RwLock::new(HashMap::from([(
            job.id,
            Arc::new(RwLock::new(job)),
        )]))),
        data_dir: Arc::new(PathBuf::from("unused-broken-sources-readonly-fixture")),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle: Default::default(),
    }
}
async fn value(response: Response) -> Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), projection::JSON_BYTES)
            .await
            .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn broken_sources_api_refuses_active_and_generation_changes_and_marks_imported_age_unknown() {
    let id = Uuid::new_v4();
    let mut source = job(id, "https://example.test");
    source.status = "running".into();
    let state = state(source);
    let request = Request {
        expected_generation: Some(7),
        ..Default::default()
    };
    let failure = preview(State(state.clone()), Path(id), Ok(Json(request.clone())))
        .await
        .err()
        .unwrap();
    assert_eq!(failure.0, StatusCode::CONFLICT);
    assert_eq!(failure.1.0.code, "unstable_snapshot");
    let job = get_job(&state, id)
        .await
        .unwrap_or_else(|(_, Json(e))| panic!("{}", e.error));
    {
        let mut g = job.write().await;
        g.status = "completed".into();
        g.generation = 8;
    }
    assert_eq!(
        preview(State(state.clone()), Path(id), Ok(Json(request.clone())))
            .await
            .err()
            .unwrap()
            .1
            .0
            .code,
        "generation_conflict"
    );
    {
        let mut g = job.write().await;
        g.generation = 7;
        g.frontier_available = false;
    }
    let p = value(
        preview(State(state.clone()), Path(id), Ok(Json(request)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(p["rows"][0]["verdict"], "unknown");
    assert_eq!(
        p["snapshot"]["caller_binding"]["origin"],
        "app_declared_imported_or_detached_report"
    );
    assert_eq!(p["snapshot"]["sql_attested"], false);
    assert_eq!(p["source_kind_coverage_complete"], false);
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
#[tokio::test]
async fn broken_sources_api_full_csv_keeps_duplicate_sources_and_exact_length_sha_without_website_requests()
 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let id = Uuid::new_v4();
    let state = state(job(id, &url));
    let mut request = Request {
        filter: Filter::Broken,
        limit: 1,
        expected_generation: Some(7),
        ..Default::default()
    };
    let p = value(
        preview(State(state.clone()), Path(id), Ok(Json(request.clone())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(p["rows"][0]["link"]["text"], "first, ä");
    assert_eq!(p["filtered_rows"], 2);
    assert_eq!(p["website_requests"], 0);
    request.expected_snapshot_token = Some(p["snapshot_token"].as_str().unwrap().into());
    request.cursor = Some(p["next_cursor"].as_str().unwrap().into());
    let second = value(
        preview(State(state.clone()), Path(id), Ok(Json(request.clone())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(second["rows"][0]["link"]["text"], "second");
    assert!(second["next_cursor"].is_null());
    assert_eq!(
        csv(State(state.clone()), Path(id), Ok(Json(request.clone())))
            .await
            .err()
            .unwrap()
            .1
            .0
            .code,
        "full_selection_export"
    );
    request.cursor = None;
    let response = csv(State(state.clone()), Path(id), Ok(Json(request.clone())))
        .await
        .unwrap();
    let length = response.headers()[header::CONTENT_LENGTH]
        .to_str()
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let sha = response.headers()["x-content-sha256"]
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = to_bytes(response.into_body(), projection::CSV_BYTES)
        .await
        .unwrap();
    assert_eq!(bytes.len(), length);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), sha);
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains("\"first, ä\""));
    assert!(text.contains("\"second\""));
    assert_eq!(text.matches("\"observed_http_failure\"").count(), 2);
    let job = get_job(&state, id)
        .await
        .unwrap_or_else(|(_, Json(e))| panic!("{}", e.error));
    job.write()
        .await
        .report
        .as_mut()
        .unwrap()
        .warnings
        .push("saved report changed".into());
    let failure = csv(State(state.clone()), Path(id), Ok(Json(request)))
        .await
        .err()
        .unwrap();
    assert_eq!(failure.0, StatusCode::CONFLICT);
    assert_eq!(failure.1.0.code, "stale_snapshot");
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
            .await
            .is_err(),
        "Offline preview/page/full export attempted website IO"
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
