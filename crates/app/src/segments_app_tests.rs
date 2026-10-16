use super::*;
use open_seo_spider_core::{
    AuditIssue, CrawlConfig, CrawlSummary, PageRecord, Severity,
    segments::{Condition, Definition, Field, Join, Operator},
};
fn report() -> CrawlReport {
    CrawlReport {
        config: CrawlConfig::default(),
        pages: (0..301)
            .map(|i| PageRecord {
                url: format!("https://x.test/{i}"),
                final_url: format!("https://x.test/{i}"),
                status: Some(200),
                depth: i % 3,
                ..Default::default()
            })
            .collect(),
        issues: vec![AuditIssue {
            code: "x".into(),
            severity: Severity::Warning,
            url: "https://x.test/300".into(),
            message: "last issue".into(),
            related_urls: vec![],
        }],
        summary: CrawlSummary::default(),
        warnings: vec![],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
        sitemap_source_measurements: None,
    }
}
#[test]
fn segments_service_cursor_binds_generation_rules_selection_view_and_report_bytes() {
    let r = report();
    let mut q = Request {
        limit: 200,
        ..Default::default()
    };
    let first = evaluate(&r, &q, Some("job".into()), Some(7), "declared").unwrap();
    let page1: serde_json::Value = serde_json::from_slice(&page(&first, &q).unwrap()).unwrap();
    assert_eq!(page1["rows"].as_array().unwrap().len(), 200);
    assert_eq!(page1["rows"][199]["page_ordinal"], 199);
    q.cursor = Some(serde_json::from_value(page1["next"].clone()).unwrap());
    q.expected_snapshot_token = Some(first.context.snapshot_token.clone());
    q.expected_generation = Some(7);
    let e = evaluate(&r, &q, Some("job".into()), Some(7), "declared").unwrap();
    let last: serde_json::Value = serde_json::from_slice(&page(&e, &q).unwrap()).unwrap();
    assert_eq!(last["rows"].as_array().unwrap().len(), 101);
    assert_eq!(last["rows"][100]["page_ordinal"], 300);
    assert!(last["next"].is_null());
    assert!(evaluate(&r, &q, Some("job".into()), Some(8), "declared").is_err());
    let mut changed = r.clone();
    changed.issues[0].message.push('!');
    assert!(evaluate(&changed, &q, Some("job".into()), Some(7), "declared").is_err());
    let mut altered = q.clone();
    altered.selection = Selection::Unassigned;
    assert!(evaluate(&r, &altered, Some("job".into()), Some(7), "declared").is_err());
    altered = q.clone();
    altered.rules.definitions.push(Definition {
        id: "new".into(),
        name: "new".into(),
        join: Join::All,
        conditions: vec![Condition {
            field: Field::RequestedUrl,
            operator: Operator::Contains,
            value: "/new".into(),
            case_insensitive: false,
        }],
    });
    assert!(evaluate(&r, &altered, Some("job".into()), Some(7), "declared").is_err());
    altered = q;
    altered.view = View::Issues;
    assert!(altered.validate().is_err());
}
#[test]
fn segments_service_full_exports_include_tail_issues_and_counts_without_page_truncation() {
    let e = evaluate(&report(), &Request::default(), None, None, "offline").unwrap();
    let full: serde_json::Value = serde_json::from_slice(&full_json(&e).unwrap()).unwrap();
    assert_eq!(full["projection"]["pages"].as_array().unwrap().len(), 301);
    assert_eq!(full["projection"]["pages"][300]["page_ordinal"], 300);
    assert_eq!(full["projection"]["issues"][0]["message"], "last issue");
    let csv = String::from_utf8(full_csv(&e).unwrap()).unwrap();
    assert_eq!(csv.lines().filter(|s| s.starts_with("page,")).count(), 301);
    assert_eq!(csv.lines().filter(|s| s.starts_with("issue,")).count(), 1);
    assert!(csv.contains(&e.context.snapshot_token));
    assert!(csv.contains("https://x.test/300"));
    assert_eq!(full["context"]["sql_attested"], false);
    assert!(
        serde_json::from_value::<Request>(serde_json::json!({"endpoint":"http://evil"})).is_err()
    );
    assert!(
        Request {
            limit: 201,
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        Request {
            cursor: Some(Cursor {
                offset: 1,
                view: View::Pages,
                snapshot_token: "0".repeat(64)
            }),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}
#[tokio::test]
async fn segments_cli_cold_report_export_and_no_overwrite_inputs_or_existing_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("report.json");
    let rules = dir.path().join("request.json");
    let output = dir.path().join("out.json");
    let report_bytes = serde_json::to_vec(&report()).unwrap();
    let rules_bytes = b"{}";
    tokio::fs::write(&report_path, &report_bytes).await.unwrap();
    tokio::fs::write(&rules, rules_bytes).await.unwrap();
    run_cli(report_path.clone(), rules.clone(), output.clone(), false)
        .await
        .unwrap();
    let first = tokio::fs::read(&output).await.unwrap();
    let value: serde_json::Value = serde_json::from_slice(&first).unwrap();
    assert_eq!(value["projection"]["selected_pages"], 301);
    assert_eq!(value["context"]["origin"], "declared_offline_report_file");
    assert!(value["context"]["app_generation"].is_null());
    assert!(
        run_cli(
            report_path.clone(),
            rules.clone(),
            report_path.clone(),
            false
        )
        .await
        .is_err()
    );
    assert!(
        run_cli(report_path.clone(), rules.clone(), rules.clone(), false)
            .await
            .is_err()
    );
    assert!(
        run_cli(report_path.clone(), rules.clone(), output.clone(), false)
            .await
            .is_err()
    );
    assert_eq!(tokio::fs::read(&report_path).await.unwrap(), report_bytes);
    assert_eq!(tokio::fs::read(&rules).await.unwrap(), rules_bytes);
    assert_eq!(tokio::fs::read(&output).await.unwrap(), first);
    let cold = dir.path().join("cold.json");
    run_cli(report_path, rules, cold.clone(), false)
        .await
        .unwrap();
    assert_eq!(tokio::fs::read(cold).await.unwrap(), first);
}

#[tokio::test]
async fn segments_cli_depth_predicate_refuses_before_opening_a_report_or_output() {
    let dir = tempfile::tempdir().unwrap();
    let request = dir.path().join("unsupported-depth.json");
    let absent_report = dir.path().join("report-must-not-be-opened.json");
    let output = dir.path().join("must-not-exist.json");
    let bytes = serde_json::to_vec(&serde_json::json!({"rules":{"version":1,"definitions":[{"id":"depth0","name":"Depth0","conditions":[{"field":"depth","operator":"equals","value":"0"}]}]}})).unwrap();
    tokio::fs::write(&request, &bytes).await.unwrap();
    let error = run_cli(
        absent_report.clone(),
        request.clone(),
        output.clone(),
        false,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("unknown variant"));
    assert!(error.to_string().contains("depth"));
    assert!(!absent_report.exists());
    assert!(!output.exists());
    assert_eq!(tokio::fs::read(&request).await.unwrap(), bytes);
}
