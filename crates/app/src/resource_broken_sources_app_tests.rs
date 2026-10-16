use super::*;

#[test]
fn resource_source_reporting_request_http_core_mode_serialization_is_strict_without_uploading_visibility()
 {
    let id = Uuid::nil();
    for mode in [None, Some("raw"), Some("reported")] {
        let value = serde_json::json!({"reporting_mode":mode});
        let request: Request = serde_json::from_value(value).unwrap();
        assert_eq!(
            serde_json::to_value(request.core(id, 7).unwrap())
                .unwrap()
                .get("reporting_mode")
                .and_then(|v| v.as_str()),
            mode
        );
    }
    for bad in [
        serde_json::json!({"reporting_mode":"auto"}),
        serde_json::json!({"hidden_owners":["https://fixture.test/"]}),
        serde_json::json!({"raw_report":{}}),
    ] {
        assert!(serde_json::from_value::<Request>(bad).is_err());
    }
}
#[test]
fn resource_sources_request_rejects_paths_observations_unknown_fields_and_wrong_job_generation() {
    for input in [
        r#"{"path":"other.sqlite"}"#,
        r#"{"pages":[]}"#,
        r#"{"limits":{"unbounded":true}}"#,
    ] {
        assert!(serde_json::from_str::<Request>(input).is_err());
    }
    let id = Uuid::new_v4();
    let token = api_token(id, 7, &"a".repeat(64));
    let request = Request {
        limit: Some(200),
        after_sequence: Some(1),
        expected_generation: Some(7),
        expected_snapshot_token: Some(token.clone()),
        ..Default::default()
    };
    assert_eq!(
        request
            .core(id, 7)
            .unwrap()
            .expected_snapshot_token
            .as_deref(),
        Some("a".repeat(64).as_str())
    );
    assert!(request.core(Uuid::new_v4(), 7).is_err());
    assert!(request.core(id, 8).is_err());
    let mut bad = request.clone();
    bad.expected_snapshot_token = Some("x".repeat(257));
    assert!(bad.core(id, 7).is_err());
    bad = request.clone();
    bad.expected_snapshot_token = None;
    assert!(bad.core(id, 7).is_err());
    bad = request;
    bad.limit = Some(201);
    assert!(bad.core(id, 7).is_err());
}

#[test]
fn resource_sources_app_json_rejects_envelope_over_budget_and_retains_unknown_null() {
    let big = "x".repeat(JSON_BYTES);
    assert!(json_bytes(&serde_json::json!({"value":big})).is_err());
    let page = ResourceBrokenSourcesPage {
        scope: SCOPE,
        available: false,
        unavailable_reason: Some("missing_history"),
        snapshot: None,
        rows: vec![],
        selected_evaluation_rows: 0,
        selected_distinct_occurrence_ids: 0,
        reporting_counts: None,
        counts: Default::default(),
        next_sequence: None,
        has_more: false,
        website_requests: 0,
    };
    let bytes = page_bytes(Uuid::new_v4(), 1, page).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value["snapshot_token"].is_null());
    assert_eq!(value["page"]["available"], false);
    assert_eq!(value["website_requests"], 0);
}

#[tokio::test]
async fn resource_sources_cli_cannot_overwrite_database_request_sidecars_or_existing_outputs() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("input.sqlite");
    let database = CrawlStore::open(&db).unwrap();
    drop(database);
    let request = temp.path().join("request.json");
    tokio::fs::write(&request, b"{}").await.unwrap();
    let existing = temp.path().join("existing.json");
    tokio::fs::write(&existing, b"owned previous output")
        .await
        .unwrap();
    let db_before = tokio::fs::read(&db).await.unwrap();
    for output in [
        db.clone(),
        request.clone(),
        existing.clone(),
        PathBuf::from(format!("{}-wal", db.display())),
    ] {
        assert!(
            run_cli(db.clone(), Some(request.clone()), output, None)
                .await
                .is_err()
        );
    }
    assert_eq!(tokio::fs::read(&db).await.unwrap(), db_before);
    assert_eq!(tokio::fs::read(&request).await.unwrap(), b"{}");
    assert_eq!(
        tokio::fs::read(existing).await.unwrap(),
        b"owned previous output"
    );
    let output = temp.path().join("unavailable.json");
    run_cli(db, Some(request), output.clone(), None)
        .await
        .unwrap();
    let page: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(output).await.unwrap()).unwrap();
    assert_eq!(page["available"], false);
    assert!(page["snapshot"].is_null());
}
