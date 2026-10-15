use super::*;
use axum::body::to_bytes;
use open_seo_spider_core::PageRecord;
fn job(id: Uuid, status: &str, url: &str) -> Job {
    Job {
        id,
        config: Default::default(),
        status: status.into(),
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
            pages: vec![PageRecord {
                url: url.into(),
                final_url: url.into(),
                status: Some(200),
                final_status: Some(200),
                ..Default::default()
            }],
            issues: vec![],
            summary: Default::default(),
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
        frontier_available: false,
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
        data_dir: Arc::new(PathBuf::from("unused-migration-readonly-fixture")),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle: Default::default(),
    }
}
fn request(url: &str) -> Request {
    Request {
        csv: format!("old,new\n{url},{url}\nhttps://x.test/missing,https://x.test/new\n"),
        options: Default::default(),
        expected_generation: Some(7),
        expected_snapshot_token: None,
        cursor: None,
        outcome: None,
        limit: 1,
    }
}
async fn value(response: Response) -> Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), workflow::JSON_BYTES)
            .await
            .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn migration_api_reuses_stable_readlock_authority_and_exposes_declared_import_scope() {
    let id = Uuid::new_v4();
    let url = "https://x.test/a";
    let state = state(job(id, "running", url));
    let (code, Json(e)) = preview(State(state.clone()), Path(id), Ok(Json(request(url))))
        .await
        .err()
        .unwrap();
    assert_eq!(code, StatusCode::CONFLICT);
    assert_eq!(e.code, "unstable_snapshot");
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|(_, Json(e))| panic!("{}", e.error));
    {
        let mut g = source.write().await;
        g.status = "completed".into();
        g.generation = 8;
    }
    assert_eq!(
        preview(State(state.clone()), Path(id), Ok(Json(request(url))))
            .await
            .err()
            .unwrap()
            .1
            .0
            .code,
        "generation_conflict"
    );
    {
        let mut g = source.write().await;
        g.generation = 0;
    }
    let mut r = request(url);
    r.expected_generation = Some(0);
    let page = value(
        preview(State(state.clone()), Path(id), Ok(Json(r.clone())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(page["snapshot"]["app_generation"], 0);
    assert_eq!(page["snapshot"]["sql_attested"], false);
    assert_eq!(
        page["snapshot"]["caller_binding"]["origin"],
        "app_declared_imported_or_detached_report"
    );
    source.write().await.report = None;
    assert_eq!(
        preview(State(state.clone()), Path(id), Ok(Json(r)))
            .await
            .err()
            .unwrap()
            .1
            .0
            .code,
        "saved_report_unavailable"
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
#[tokio::test]
async fn migration_api_pages_exports_and_stale_reports_do_not_contact_observed_website() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/saved", listener.local_addr().unwrap());
    let id = Uuid::new_v4();
    let state = state(job(id, "completed", &url));
    let mut r = request(&url);
    let page = value(
        preview(State(state.clone()), Path(id), Ok(Json(r.clone())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(page["rows"][0]["verdict"], "destination_match");
    assert_eq!(page["website_requests"], 0);
    r.expected_snapshot_token = Some(page["snapshot_token"].as_str().unwrap().into());
    r.cursor = Some(page["next_cursor"].as_str().unwrap().into());
    let second = value(
        preview(State(state.clone()), Path(id), Ok(Json(r.clone())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(second["rows"][0]["verdict"], "missing_observation");
    assert!(second["next_cursor"].is_null());
    r.cursor = None;
    let output = json(State(state.clone()), Path(id), Ok(Json(r.clone())))
        .await
        .unwrap();
    let length = output.headers()[header::CONTENT_LENGTH]
        .to_str()
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let declared = output.headers()["x-content-sha256"]
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = to_bytes(output.into_body(), workflow::JSON_BYTES)
        .await
        .unwrap();
    assert_eq!(bytes.len(), length);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), declared);
    let result: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result["report"]["rows"].as_array().unwrap().len(), 2);
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|(_, Json(e))| panic!("{}", e.error));
    source
        .write()
        .await
        .report
        .as_mut()
        .unwrap()
        .warnings
        .push("later typed report mutation".into());
    assert_eq!(
        json(State(state.clone()), Path(id), Ok(Json(r)))
            .await
            .err()
            .unwrap()
            .1
            .0
            .code,
        "stale_snapshot"
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
            .await
            .is_err(),
        "Read/preview/export contacted the observed website"
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
