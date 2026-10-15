use super::*;
use open_seo_spider_core::{PageRecord, RedirectHop};
fn saved() -> CrawlReport {
    CrawlReport {
        config: Default::default(),
        pages: vec![PageRecord {
            url: "https://x.test/a".into(),
            final_url: "https://x.test/observed".into(),
            status: Some(301),
            final_status: Some(200),
            redirect_chain: vec![RedirectHop {
                url: "https://x.test/a".into(),
                status: 301,
                target: "https://x.test/observed".into(),
            }],
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
    }
}
fn request() -> Request {
    Request{csv:"old,new\nhttps://x.test/a,https://x.test/expected\nhttps://x.test/missing,https://x.test/expected\nhttps://x.test/a,https://x.test/expected\n".into(),options:Default::default(),expected_generation:Some(7),expected_snapshot_token:None,cursor:None,outcome:None,limit:1}
}
fn eval(r: &Request, generation: u64) -> Evaluation {
    let report = saved();
    let snapshot = Snapshot::from_report(
        &report,
        Some(Uuid::nil()),
        Some(generation),
        "completed".into(),
        digest(&report.config, OPTIONS_BYTES).unwrap().0,
        Default::default(),
    )
    .unwrap();
    evaluate(r.csv.as_bytes(), &r.options, report, snapshot).unwrap()
}

#[test]
fn migration_workflow_cursor_binds_actual_snapshot_csv_options_filter_and_limit() {
    let mut request = request();
    let evaluated = eval(&request, 7);
    let page = evaluated.page(&request).unwrap();
    assert_eq!(page.total_rows, 3);
    assert_eq!(page.rows[0].verdict, MigrationVerdict::DestinationMismatch);
    assert_eq!(page.summary[&MigrationVerdict::MissingObservation], 1);
    let cursor = page.next_cursor.unwrap();
    request.cursor = Some(cursor);
    request.expected_snapshot_token = Some(evaluated.token.clone());
    let page = evaluated.page(&request).unwrap();
    assert_eq!(page.offset, 1);
    assert_eq!(page.rows[0].verdict, MigrationVerdict::MissingObservation);
    let mut altered = request.clone();
    altered.limit = 2;
    assert_eq!(evaluated.page(&altered).err().unwrap().code, "stale_cursor");
    altered = request.clone();
    altered.outcome = Some(MigrationVerdict::DestinationMismatch);
    assert_eq!(evaluated.page(&altered).err().unwrap().code, "stale_cursor");
    altered = request.clone();
    altered
        .csv
        .push_str("https://x.test/new,https://x.test/new\n");
    assert_eq!(
        eval(&altered, 7)
            .check_token(&altered, false)
            .unwrap_err()
            .code,
        "stale_snapshot"
    );
    altered = request.clone();
    altered.options.start_row = 3;
    assert_eq!(
        eval(&altered, 7)
            .check_token(&altered, false)
            .unwrap_err()
            .code,
        "stale_snapshot"
    );
    assert_eq!(
        eval(&request, 8)
            .check_token(&request, false)
            .unwrap_err()
            .code,
        "stale_snapshot"
    );
    let mut all = request.clone();
    all.cursor = None;
    all.outcome = Some(MigrationVerdict::DestinationMismatch);
    all.limit = 200;
    let filtered = evaluated.page(&all).unwrap();
    assert_eq!(filtered.filtered_rows, 2);
    assert_eq!(filtered.rows[1].duplicate_of_row, Some(2));
    assert!(filtered.next_cursor.is_none());
}
#[test]
fn migration_workflow_complete_exports_preserve_unknown_and_formula_safe_originals() {
    let mut request = request();
    request.csv.push_str("=SUM(1),https://x.test/b\n");
    let evaluated = eval(&request, 7);
    assert_eq!(
        evaluated.check_token(&request, true).unwrap_err().code,
        "snapshot_token_required"
    );
    request.expected_snapshot_token = Some(evaluated.token.clone());
    evaluated.check_token(&request, true).unwrap();
    let json = evaluated.export_json().unwrap();
    let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(value["website_requests"], 0);
    assert_eq!(value["report"]["rows"].as_array().unwrap().len(), 4);
    assert_eq!(value["report"]["rows"][3]["old_value"], "=SUM(1)");
    assert_eq!(value["report"]["rows"][3]["verdict"], "invalid_mapping");
    assert_eq!(value["snapshot"]["sql_attested"], false);
    let csv = String::from_utf8(evaluated.export_csv().unwrap()).unwrap();
    assert!(csv.contains("\"'=SUM(1)\""));
    assert!(csv.contains("destination_mismatch"));
    assert!(csv.contains("missing_observation"));
    assert!(csv.contains(&evaluated.token));
    assert_eq!(evaluated.report.website_requests, 0);
}
#[test]
fn migration_workflow_strict_request_and_independent_budgets_reject_before_evaluation() {
    let mut value = serde_json::to_value(request()).unwrap();
    value["report_path"] = "C:/foreign.sqlite".into();
    assert!(serde_json::from_value::<Request>(value).is_err());
    let mut r = request();
    r.csv = "x".repeat(API_CSV_BYTES + 1);
    assert_eq!(r.validate().unwrap_err().code, "csv_byte_budget");
    r = request();
    r.limit = 201;
    assert_eq!(r.validate().unwrap_err().code, "invalid_page_limit");
    r = request();
    r.options.observation_limits.max_pages = 50_001;
    assert_eq!(r.validate().unwrap_err().code, "invalid_options");
    let mut bytes = BoundedBytes {
        bytes: vec![],
        max: 4,
    };
    bytes.write_all(b"1234").unwrap();
    assert!(bytes.write_all(b"5").is_err());
    assert_eq!(bytes.bytes, b"1234");
    assert!(digest(&"abcdef", 3).is_err());
}
#[tokio::test]
async fn migration_cli_new_outputs_keep_mismatch_success_and_never_overwrite_inputs() {
    let d = tempfile::tempdir().unwrap();
    let mapping = d.path().join("map.csv");
    let report = d.path().join("report.json");
    let options = d.path().join("options.json");
    let json = d.path().join("migration.json");
    let csv = d.path().join("migration.csv");
    let input = request().csv.into_bytes();
    tokio::fs::write(&mapping, &input).await.unwrap();
    tokio::fs::write(&report, serde_json::to_vec(&saved()).unwrap())
        .await
        .unwrap();
    tokio::fs::write(&options, b"{}").await.unwrap();
    run_cli(
        mapping.clone(),
        report.clone(),
        options.clone(),
        json.clone(),
        Some(csv.clone()),
    )
    .await
    .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&tokio::fs::read(&json).await.unwrap()).unwrap();
    assert_eq!(
        value["report"]["rows"][0]["verdict"],
        "destination_mismatch"
    );
    assert_eq!(value["website_requests"], 0);
    assert_eq!(tokio::fs::read(&mapping).await.unwrap(), input);
    let before = tokio::fs::read(&json).await.unwrap();
    assert_eq!(
        run_cli(
            mapping.clone(),
            report.clone(),
            options.clone(),
            json.clone(),
            Some(csv.clone())
        )
        .await
        .unwrap_err()
        .code,
        "output_exists"
    );
    assert_eq!(tokio::fs::read(&json).await.unwrap(), before);
    let same = d.path().join("same.json");
    assert_eq!(
        run_cli(mapping, report, options, same.clone(), Some(same.clone()))
            .await
            .unwrap_err()
            .code,
        "duplicate_output"
    );
    assert!(!same.exists());
}
