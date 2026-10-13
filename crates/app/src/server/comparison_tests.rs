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
    };
    let (code, Json(error)) = stable_pair(&state, &request).await.err().unwrap();
    assert_eq!(code, StatusCode::CONFLICT);
    assert_eq!(error.context.unwrap().side, ComparisonSide::After);
}
