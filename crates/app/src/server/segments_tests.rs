use super::*;
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
                indexable: true,
                internal: true,
                fetched_at: "2026-10-15T00:00:00Z".into(),
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
        quota_increases: None,
        policy_ledger: None,
        resource_ledger: None,
    }
}
fn state(j: Job) -> AppState {
    AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Arc::new(RwLock::new(HashMap::from([(
            j.id,
            Arc::new(RwLock::new(j)),
        )]))),
        data_dir: Arc::new(PathBuf::from("unused-directory-readonly-fixture")),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle: Default::default(),
    }
}
async fn value(r: Response) -> Value {
    serde_json::from_slice(
        &axum::body::to_bytes(r.into_body(), core::OUTPUT_BYTES)
            .await
            .unwrap(),
    )
    .unwrap()
}
fn refused<T>(r: SResult<T>) -> (StatusCode, Failure) {
    match r {
        Err((s, Json(e))) => (s, e),
        Ok(_) => panic!("Expected refusal"),
    }
}
#[tokio::test]
async fn segments_saved_api_rejects_active_and_stale_generation_and_preserves_declared_authority() {
    let id = Uuid::new_v4();
    let state = state(job(id, "running", "https://x.test/a"));
    let (status, error) =
        refused(preview(State(state.clone()), Path(id), Ok(Json(Request::default()))).await);
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error.code, "unstable_snapshot");
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|_| panic!("expected saved crawl job"));
    source.write().await.status = "completed".into();
    let before = serde_json::to_vec(source.read().await.report.as_ref().unwrap()).unwrap();
    let first = value(
        preview(State(state.clone()), Path(id), Ok(Json(Request::default())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(first["summary"]["selected_pages"], 1);
    assert_eq!(first["context"]["sql_attested"], false);
    assert_eq!(first["context"]["report_generation_attested"], false);
    assert_eq!(
        first["context"]["origin"],
        "app_declared_imported_or_detached_report"
    );
    let request = Request {
        expected_generation: Some(7),
        expected_snapshot_token: Some(first["context"]["snapshot_token"].as_str().unwrap().into()),
        ..Default::default()
    };
    let exported = value(
        json(State(state.clone()), Path(id), Ok(Json(request.clone())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        exported["projection"]["pages"][0]["url"],
        "https://x.test/a"
    );
    assert_eq!(
        serde_json::to_vec(source.read().await.report.as_ref().unwrap()).unwrap(),
        before
    );
    source.write().await.generation = 8;
    assert_eq!(
        refused(preview(State(state.clone()), Path(id), Ok(Json(request))).await).0,
        StatusCode::CONFLICT
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
#[tokio::test]
async fn segments_saved_api_full_export_requires_token_and_detects_report_tail_change() {
    let id = Uuid::new_v4();
    let state = state(job(id, "completed", "https://x.test/a"));
    assert_eq!(
        refused(csv(State(state.clone()), Path(id), Ok(Json(Request::default()))).await).0,
        StatusCode::BAD_REQUEST
    );
    let first = value(
        preview(State(state.clone()), Path(id), Ok(Json(Request::default())))
            .await
            .unwrap(),
    )
    .await;
    let q = Request {
        expected_snapshot_token: Some(first["context"]["snapshot_token"].as_str().unwrap().into()),
        ..Default::default()
    };
    let response = csv(State(state.clone()), Path(id), Ok(Json(q.clone())))
        .await
        .unwrap();
    let expected = response.headers()["x-content-sha256"]
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = axum::body::to_bytes(response.into_body(), core::OUTPUT_BYTES)
        .await
        .unwrap();
    assert_eq!(crate::segments_app::sha(&bytes), expected);
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|_| panic!("expected saved crawl job"));
    source
        .write()
        .await
        .report
        .as_mut()
        .unwrap()
        .warnings
        .push("changed after first page".into());
    assert_eq!(
        refused(json(State(state.clone()), Path(id), Ok(Json(q))).await).0,
        StatusCode::CONFLICT
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}

fn reporting_job(id: Uuid) -> Job {
    let mut j = job(id, "completed", "https://x.test/visible");
    let report = j.report.as_mut().unwrap();
    report.config.directive_reporting = Some(open_seo_spider_core::DirectiveReportingConfig {
        respect_noindex: true,
        ..Default::default()
    });
    let mut hidden = report.pages[0].clone();
    hidden.url = "https://x.test/hidden".into();
    hidden.final_url = hidden.url.clone();
    hidden.meta_robots = vec!["noindex".into()];
    report.pages.push(hidden);
    j
}

#[test]
fn segments_borrowed_authority_binding_preserves_owned_comparison_facts_and_refusals() {
    let id = Uuid::new_v4();
    let mut j = job(id, "completed", "https://x.test/authority");
    for frontier in [false, true] {
        j.frontier_available = frontier;
        for generation in [0, 7] {
            j.generation = generation;
            let borrowed = comparison::stable_saved_report_binding(
                &j,
                ComparisonSide::Before,
                Some(generation),
            )
            .unwrap();
            let owned =
                comparison::stable_saved_report(&j, ComparisonSide::Before, Some(generation))
                    .unwrap();
            assert_eq!(
                serde_json::to_value(borrowed).unwrap(),
                serde_json::to_value(owned.binding).unwrap()
            );
            assert_eq!(
                serde_json::to_vec(&owned.report).unwrap(),
                serde_json::to_vec(j.report.as_ref().unwrap()).unwrap()
            );
        }
    }
    j.checkpoint = Some(CrawlCheckpoint {
        generation: 7,
        revision: 3,
        outcome: CrawlOutcome::Completed,
        created_at: "captured".into(),
        report_sha256: "declared-report".into(),
        config_sha256: "declared-config".into(),
        completed: 1,
        pending: 0,
        discovery_pending: 0,
        discovery_complete: true,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_ledger: None,
        sitemap_source_measurements: None,
    });
    let binding =
        comparison::stable_saved_report_binding(&j, ComparisonSide::Before, Some(7)).unwrap();
    assert_eq!(
        binding.origin.as_deref(),
        Some("app_declared_job_report_with_cached_checkpoint")
    );
    let basis: Value = serde_json::from_str(binding.basis.as_ref().unwrap()).unwrap();
    assert_eq!(basis["checkpoint_revision"], 3);
    assert_eq!(basis["sql_attested"], false);
    for (status, expected) in [("running", Some(7)), ("completed", Some(8))] {
        j.status = status.into();
        let first = comparison::stable_saved_report_binding(&j, ComparisonSide::Before, expected)
            .err()
            .unwrap();
        let second = comparison::stable_saved_report(&j, ComparisonSide::Before, expected)
            .err()
            .unwrap();
        assert_eq!(first.0, second.0);
        assert_eq!(
            serde_json::to_value(first.1.0).unwrap(),
            serde_json::to_value(second.1.0).unwrap()
        );
    }
}
#[tokio::test]
async fn segments_saved_api_profile_preview_json_csv_share_view_bound_token_and_raw_authority() {
    let id = Uuid::new_v4();
    let state = state(reporting_job(id));
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|_| panic!("fixture"));
    let before = serde_json::to_vec(source.read().await.report.as_ref().unwrap()).unwrap();
    let first = value(
        preview(State(state.clone()), Path(id), Ok(Json(Request::default())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(first["summary"]["selected_pages"], 1);
    assert_eq!(first["reporting"]["captured_page_observations"], 2);
    assert_eq!(first["context"]["reporting_view"]["mode"], "reported");
    let q = Request {
        expected_snapshot_token: Some(first["context"]["snapshot_token"].as_str().unwrap().into()),
        ..Default::default()
    };
    let full = value(
        json(State(state.clone()), Path(id), Ok(Json(q.clone())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(full["projection"]["reporting"], first["reporting"]);
    assert_eq!(
        full["context"]["typed_report_sha256"],
        first["context"]["typed_report_sha256"]
    );
    let out = csv(State(state.clone()), Path(id), Ok(Json(q.clone())))
        .await
        .unwrap();
    let expected = out.headers()["x-content-sha256"]
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = axum::body::to_bytes(out.into_body(), core::OUTPUT_BYTES)
        .await
        .unwrap();
    assert_eq!(service::sha(&bytes), expected);
    let mut crossed = q;
    crossed.reporting_mode = Some(core::ReportingMode::Raw);
    assert_eq!(
        refused(preview(State(state.clone()), Path(id), Ok(Json(crossed.clone()))).await).0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        refused(json(State(state.clone()), Path(id), Ok(Json(crossed.clone()))).await).0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        refused(csv(State(state.clone()), Path(id), Ok(Json(crossed))).await).0,
        StatusCode::CONFLICT
    );
    let raw = value(
        preview(
            State(state.clone()),
            Path(id),
            Ok(Json(Request {
                reporting_mode: Some(core::ReportingMode::Raw),
                ..Default::default()
            })),
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(raw["summary"]["selected_pages"], 2);
    assert_eq!(
        raw["context"]["typed_report_sha256"],
        first["context"]["typed_report_sha256"]
    );
    assert_eq!(
        serde_json::to_vec(source.read().await.report.as_ref().unwrap()).unwrap(),
        before
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
#[tokio::test]
async fn segments_saved_api_same_job_profile_change_refuses_prior_preview_and_exports() {
    let id = Uuid::new_v4();
    let state = state(reporting_job(id));
    let first = value(
        preview(State(state.clone()), Path(id), Ok(Json(Request::default())))
            .await
            .unwrap(),
    )
    .await;
    let q = Request {
        expected_snapshot_token: Some(first["context"]["snapshot_token"].as_str().unwrap().into()),
        ..Default::default()
    };
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|_| panic!("fixture"));
    source
        .write()
        .await
        .report
        .as_mut()
        .unwrap()
        .config
        .directive_reporting
        .as_mut()
        .unwrap()
        .respect_canonical = true;
    assert_eq!(
        refused(preview(State(state.clone()), Path(id), Ok(Json(q.clone()))).await).0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        refused(json(State(state.clone()), Path(id), Ok(Json(q.clone()))).await).0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        refused(csv(State(state.clone()), Path(id), Ok(Json(q))).await).0,
        StatusCode::CONFLICT
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
