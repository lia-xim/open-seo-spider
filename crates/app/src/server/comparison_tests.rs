//! Stable report, generation and actual App-to-Core binding controls.
use super::*;
fn report() -> CrawlReport {
    CrawlReport {
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
    }
}
fn job(status: &str) -> Job {
    Job {
        id: Uuid::nil(),
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
        report: Some(report()),
        control: CrawlControl::new(),
        generation: 2,
        checkpoint: None,
        frontier_available: false,
        operation: None,
        recrawl_override: Default::default(),
        policy_ledger: None,
        resource_ledger: None,
    }
}
#[test]
fn active_statuses_refuse_retained_report_with_structured_conflict() {
    for status in ["running", "pausing", "cancelling"] {
        let (code, Json(error)) = stable_saved_report(&job(status), ComparisonSide::Before, None)
            .err()
            .unwrap();
        assert_eq!(code, StatusCode::CONFLICT);
        assert_eq!(error.error.code, "unstable_snapshot");
        assert_eq!(error.context.unwrap().status, status);
    }
}
#[test]
fn missing_report_is_conflict_and_not_empty_cohort() {
    let mut job = job("completed");
    job.report = None;
    let (code, Json(error)) = stable_saved_report(&job, ComparisonSide::After, None)
        .err()
        .unwrap();
    assert_eq!(code, StatusCode::CONFLICT);
    assert_eq!(error.error.code, "saved_report_unavailable");
}
#[test]
fn mismatched_generation_retains_expected_observed_and_side() {
    let (code, Json(error)) = stable_saved_report(&job("paused"), ComparisonSide::After, Some(1))
        .err()
        .unwrap();
    assert_eq!(code, StatusCode::CONFLICT);
    assert_eq!(error.error.code, "generation_conflict");
    let context = error.context.unwrap();
    assert_eq!(context.expected_generation, Some(1));
    assert_eq!(context.observed_generation, 2);
    assert_eq!(context.side, ComparisonSide::After);
}
#[test]
fn imported_or_detached_report_is_declared_and_job_generation_not_sql_attested() {
    let mut source = job("completed");
    source.generation = 0;
    let saved = stable_saved_report(&source, ComparisonSide::Before, Some(0)).unwrap();
    assert_eq!(
        saved.binding.origin.as_deref(),
        Some("app_declared_imported_or_detached_report")
    );
    assert_eq!(saved.binding.generation, None);
    assert!(saved.binding.snapshot_sha256.is_none());
    let basis: Value = serde_json::from_str(saved.binding.basis.as_ref().unwrap()).unwrap();
    assert_eq!(basis["declaration"], "app_declared");
    assert_eq!(basis["sql_attested"], false);
    source
        .report
        .as_mut()
        .unwrap()
        .warnings
        .push("new later state".into());
    assert!(saved.report.warnings.is_empty());
}
#[test]
fn stable_native_and_imported_bindings_satisfy_actual_core_contract() {
    for generation in [2, 0] {
        let mut source = job("completed");
        source.generation = generation;
        let saved = stable_saved_report(&source, ComparisonSide::Before, Some(generation)).unwrap();
        let result = compare_saved_crawls(
            &saved.report,
            &saved.report,
            &Default::default(),
            Some(&saved.binding),
            Some(&saved.binding),
        );
        assert!(result.is_ok(), "generation {generation}: {result:?}");
    }
}
#[test]
fn stable_partial_terminal_statuses_remain_comparable_saved_cohorts() {
    for status in [
        "completed",
        "paused",
        "limited",
        "cancelled",
        "failed",
        "interrupted",
    ] {
        let saved = stable_saved_report(&job(status), ComparisonSide::Before, Some(2)).unwrap();
        assert_eq!(saved.binding.active_checkpoint, Some(false));
    }
}
#[tokio::test]
async fn same_job_pair_cannot_observe_two_different_expected_generations() {
    let source = job("completed");
    let id = source.id;
    let state = AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Arc::new(RwLock::new(HashMap::from([(
            id,
            Arc::new(RwLock::new(source)),
        )]))),
        data_dir: Arc::new(PathBuf::from("unused-comparison-test-directory")),
    };
    let request = SavedComparisonRequest {
        before: id,
        after: id,
        config: None,
        expected_before_generation: Some(2),
        expected_after_generation: Some(1),
        include_original_reports: false,
    };
    let (code, Json(error)) = stable_pair(&state, &request).await.err().unwrap();
    assert_eq!(code, StatusCode::CONFLICT);
    assert_eq!(error.context.unwrap().side, ComparisonSide::After);
}

fn bundle_state(sources: Vec<Job>) -> AppState {
    AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Arc::new(RwLock::new(
            sources
                .into_iter()
                .map(|source| (source.id, Arc::new(RwLock::new(source))))
                .collect(),
        )),
        data_dir: Arc::new(PathBuf::from("unused-original-report-bundle-directory")),
    }
}
fn bundle_request(before: Uuid, after: Uuid) -> SavedComparisonRequest {
    SavedComparisonRequest {
        before,
        after,
        config: None,
        expected_before_generation: Some(2),
        expected_after_generation: Some(2),
        include_original_reports: true,
    }
}
#[tokio::test]
async fn endpoint_default_omits_bundle_and_explicit_same_job_bundle_uses_one_revision() {
    let mut source = job("completed");
    source
        .report
        .as_mut()
        .unwrap()
        .warnings
        .push("retained once".into());
    let id = source.id;
    let state = bundle_state(vec![source]);
    let request = bundle_request(id, id);
    let Json(response) = compare_saved(State(state.clone()), Ok(Json(request.clone())))
        .await
        .unwrap();
    response.validate_original_reports().unwrap();
    let bundle = response.original_reports.as_ref().unwrap();
    assert_eq!(
        bundle.before.typed_report_sha256,
        bundle.after.typed_report_sha256
    );
    assert_eq!(bundle.before.report.warnings, ["retained once"]);
    assert_eq!(
        response
            .comparison
            .before_snapshot
            .caller_binding
            .as_ref()
            .unwrap()
            .generation,
        Some(2)
    );
    let mut default_request = request;
    default_request.include_original_reports = false;
    let Json(response) = compare_saved(State(state), Ok(Json(default_request)))
        .await
        .unwrap();
    let value = serde_json::to_value(response).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 2);
    assert!(value.get("original_reports").is_none());
}
#[tokio::test]
async fn bundle_uses_same_stable_pair_after_both_sources_resume_and_change() {
    let mut before = job("completed");
    before.id = Uuid::from_u128(2);
    before.report.as_mut().unwrap().warnings = vec!["before clone".into()];
    let mut after = job("completed");
    after.id = Uuid::from_u128(1);
    after.report.as_mut().unwrap().warnings = vec!["after clone".into()];
    let request = bundle_request(before.id, after.id);
    let state = bundle_state(vec![before, after]);
    let (cloned_before, cloned_after) = stable_pair(&state, &request).await.unwrap();
    let sources: Vec<_> = state.jobs.read().await.values().cloned().collect();
    for source in sources {
        let mut guard = source.write().await;
        guard.status = "running".into();
        guard.generation = 3;
        guard.report.as_mut().unwrap().warnings = vec!["later generation mutation".into()];
    }
    let response =
        compare_stable_reports(cloned_before, cloned_after, Default::default(), true).unwrap();
    response.validate_original_reports().unwrap();
    let bundle = response.original_reports.as_ref().unwrap();
    assert_eq!(bundle.before.report.warnings, ["before clone"]);
    assert_eq!(bundle.after.report.warnings, ["after clone"]);
    for snapshot in [
        &response.comparison.before_snapshot,
        &response.comparison.after_snapshot,
    ] {
        assert_eq!(
            snapshot.caller_binding.as_ref().unwrap().generation,
            Some(2)
        );
        assert!(!snapshot.caller_binding_attested_by_core);
    }
}
#[tokio::test]
async fn imported_detached_bundle_retains_values_without_inventing_sql_or_generation_attestation() {
    let mut source = job("completed");
    source.generation = 0;
    source.report.as_mut().unwrap().summary.elapsed_ms = 9_007_199_254_740_993;
    source.report.as_mut().unwrap().warnings = vec!["retained imported warning".into()];
    let id = source.id;
    let state = bundle_state(vec![source]);
    let mut request = bundle_request(id, id);
    request.expected_before_generation = Some(0);
    request.expected_after_generation = Some(0);
    let Json(response) = compare_saved(State(state), Ok(Json(request)))
        .await
        .unwrap();
    response.validate_original_reports().unwrap();
    let binding = response
        .comparison
        .before_snapshot
        .caller_binding
        .as_ref()
        .unwrap();
    assert!(binding.generation.is_none());
    assert!(binding.snapshot_sha256.is_none());
    assert_eq!(
        binding.origin.as_deref(),
        Some("app_declared_imported_or_detached_report")
    );
    let basis: Value = serde_json::from_str(binding.basis.as_ref().unwrap()).unwrap();
    assert_eq!(basis["sql_attested"], false);
    assert_eq!(basis["report_generation_attested"], false);
    let bundle = response.original_reports.as_ref().unwrap();
    assert_eq!(
        bundle.before.report.summary.elapsed_ms,
        9_007_199_254_740_993
    );
    assert!(
        String::from_utf8(serde_json::to_vec(&response).unwrap())
            .unwrap()
            .contains("9007199254740993")
    );
}
#[tokio::test]
async fn bundle_flag_does_not_bypass_active_status_or_expected_generation_conflicts() {
    for status in ["running", "pausing", "cancelling"] {
        let source = job(status);
        let id = source.id;
        let state = bundle_state(vec![source]);
        let (code, Json(error)) = compare_saved(State(state), Ok(Json(bundle_request(id, id))))
            .await
            .err()
            .unwrap();
        assert_eq!(code, StatusCode::CONFLICT);
        assert_eq!(error.error.code, "unstable_snapshot");
        assert_eq!(error.website_requests, 0);
    }
    let source = job("completed");
    let id = source.id;
    let state = bundle_state(vec![source]);
    let mut request = bundle_request(id, id);
    request.expected_after_generation = Some(1);
    let (code, Json(error)) = compare_saved(State(state), Ok(Json(request)))
        .await
        .err()
        .unwrap();
    assert_eq!(code, StatusCode::CONFLICT);
    assert_eq!(error.error.code, "generation_conflict");
    assert_eq!(error.context.unwrap().side, ComparisonSide::After);
}
