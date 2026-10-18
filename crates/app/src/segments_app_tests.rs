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
    let page1: serde_json::Value = serde_json::from_slice(&page(&r, &first, &q).unwrap()).unwrap();
    assert_eq!(page1["rows"].as_array().unwrap().len(), 200);
    assert_eq!(page1["rows"][199]["page_ordinal"], 199);
    q.cursor = Some(serde_json::from_value(page1["next"].clone()).unwrap());
    q.expected_snapshot_token = Some(first.context.snapshot_token.clone());
    q.expected_generation = Some(7);
    let e = evaluate(&r, &q, Some("job".into()), Some(7), "declared").unwrap();
    let last: serde_json::Value = serde_json::from_slice(&page(&r, &e, &q).unwrap()).unwrap();
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
    let r = report();
    let e = evaluate(&r, &Request::default(), None, None, "offline").unwrap();
    let full: serde_json::Value = serde_json::from_slice(&full_json(&r, &e).unwrap()).unwrap();
    assert_eq!(full["projection"]["pages"].as_array().unwrap().len(), 301);
    assert_eq!(full["projection"]["pages"][300]["page_ordinal"], 300);
    assert_eq!(full["projection"]["issues"][0]["message"], "last issue");
    let csv = String::from_utf8(full_csv(&r, &e).unwrap()).unwrap();
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

fn reporting_report() -> CrawlReport {
    let mut r = report();
    r.config.directive_reporting = Some(open_seo_spider_core::DirectiveReportingConfig {
        respect_noindex: true,
        ..Default::default()
    });
    for i in [0, 200] {
        r.pages[i].meta_robots = vec!["noindex".into()];
    }
    r
}
#[test]
fn segments_service_reporting_mode_serde_is_strict_and_legacy_request_omits_new_field() {
    let baseline = serde_json::to_value(Request::default()).unwrap();
    assert!(baseline.get("reporting_mode").is_none());
    assert!(
        serde_json::from_value::<Request>(serde_json::json!({"reporting_mode":null}))
            .unwrap()
            .reporting_mode
            .is_none()
    );
    for mode in ["raw", "reported"] {
        let parsed: Request =
            serde_json::from_value(serde_json::json!({"reporting_mode":mode})).unwrap();
        assert_eq!(
            serde_json::to_value(parsed).unwrap()["reporting_mode"],
            mode
        );
    }
    for mode in [
        serde_json::json!("latest"),
        serde_json::json!(true),
        serde_json::json!({}),
    ] {
        assert!(
            serde_json::from_value::<Request>(serde_json::json!({"reporting_mode":mode})).is_err()
        );
    }
}
#[test]
fn segments_service_reporting_tokens_bind_mode_profile_and_raw_snapshot_without_relabelling() {
    let mut r = report();
    r.pages.truncate(1);
    r.config.directive_reporting = Some(open_seo_spider_core::DirectiveReportingConfig {
        respect_noindex: true,
        ..Default::default()
    });
    let before = serde_json::to_vec(&r).unwrap();
    let q = Request::default();
    let reported = evaluate(&r, &q, Some("job".into()), Some(7), "declared").unwrap();
    let raw_q = Request {
        reporting_mode: Some(segments::ReportingMode::Raw),
        ..Default::default()
    };
    let raw = evaluate(&r, &raw_q, Some("job".into()), Some(7), "declared").unwrap();
    assert_eq!(
        serde_json::to_value(&reported.projection.pages).unwrap(),
        serde_json::to_value(&raw.projection.pages).unwrap(),
        "Same rows still require distinct view authority"
    );
    assert_eq!(
        reported.context.typed_report_sha256,
        raw.context.typed_report_sha256
    );
    assert_eq!(
        reported.context.typed_report_sha256,
        report_hash(&r).unwrap()
    );
    assert_ne!(reported.context.snapshot_token, raw.context.snapshot_token);
    let stale = Request {
        reporting_mode: Some(segments::ReportingMode::Raw),
        expected_snapshot_token: Some(reported.context.snapshot_token.clone()),
        ..Default::default()
    };
    assert!(evaluate(&r, &stale, Some("job".into()), Some(7), "declared").is_err());
    let explicit = evaluate(
        &r,
        &Request {
            reporting_mode: Some(segments::ReportingMode::Reported),
            ..Default::default()
        },
        Some("job".into()),
        Some(7),
        "declared",
    )
    .unwrap();
    assert_eq!(
        explicit.context.snapshot_token,
        reported.context.snapshot_token
    );
    r.config
        .directive_reporting
        .as_mut()
        .unwrap()
        .respect_canonical = true;
    let changed = evaluate(&r, &q, Some("job".into()), Some(7), "declared").unwrap();
    assert_ne!(
        changed.context.reporting_view,
        reported.context.reporting_view
    );
    assert!(
        evaluate(
            &r,
            &Request {
                expected_snapshot_token: Some(reported.context.snapshot_token),
                ..Default::default()
            },
            Some("job".into()),
            Some(7),
            "declared"
        )
        .is_err()
    );
    r.config
        .directive_reporting
        .as_mut()
        .unwrap()
        .respect_canonical = false;
    assert_eq!(serde_json::to_vec(&r).unwrap(), before);
}
#[test]
fn segments_service_reported_paging_keeps_raw_ordinals_and_full_json_csv_population() {
    let r = reporting_report();
    let mut q = Request {
        limit: 200,
        ..Default::default()
    };
    let first = evaluate(&r, &q, Some("job".into()), Some(7), "declared").unwrap();
    let value: serde_json::Value = serde_json::from_slice(&page(&r, &first, &q).unwrap()).unwrap();
    assert_eq!(value["summary"]["total_pages"], 299);
    assert_eq!(value["reporting"]["captured_page_observations"], 301);
    assert_eq!(value["reporting"]["profile_excluded_page_observations"], 2);
    assert_eq!(value["rows"][0]["page_ordinal"], 1);
    assert_eq!(value["rows"][199]["page_ordinal"], 201);
    q.cursor = Some(serde_json::from_value(value["next"].clone()).unwrap());
    q.expected_snapshot_token = Some(first.context.snapshot_token.clone());
    let continuation = evaluate(&r, &q, Some("job".into()), Some(7), "declared").unwrap();
    let tail: serde_json::Value =
        serde_json::from_slice(&page(&r, &continuation, &q).unwrap()).unwrap();
    assert_eq!(tail["rows"].as_array().unwrap().len(), 99);
    assert_eq!(tail["rows"][98]["page_ordinal"], 300);
    let mut mismatched = q.clone();
    mismatched.reporting_mode = Some(segments::ReportingMode::Raw);
    assert!(evaluate(&r, &mismatched, Some("job".into()), Some(7), "declared").is_err());
    let full: serde_json::Value = serde_json::from_slice(&full_json(&r, &first).unwrap()).unwrap();
    assert_eq!(full["projection"]["pages"].as_array().unwrap().len(), 299);
    assert_eq!(full["projection"]["reporting"], value["reporting"]);
    let csv = String::from_utf8(full_csv(&r, &first).unwrap()).unwrap();
    assert_eq!(csv.lines().filter(|r| r.starts_with("page,")).count(), 299);
    assert_eq!(
        csv.lines().filter(|r| r.starts_with("reporting,")).count(),
        1
    );
    assert_eq!(csv.lines().filter(|r| r.starts_with("issue,")).count(), 1);
}
#[test]
fn segments_service_inactive_profile_keeps_old_token_formula_full_json_and_csv() {
    let mut r = report();
    let q = Request::default();
    let first = evaluate(&r, &q, None, None, "offline").unwrap();
    let expected = sha(&segments::json_bytes(&(
        segments::ENGINE,
        &Option::<String>::None,
        Option::<u64>::None,
        "offline",
        &first.context.typed_report_sha256,
        &first.context.rules_sha256,
        &q.selection,
    ))
    .unwrap());
    assert_eq!(first.context.snapshot_token, expected);
    let original_json = full_json(&r, &first).unwrap();
    let original_csv = full_csv(&r, &first).unwrap();
    r.config.directive_reporting = Some(open_seo_spider_core::DirectiveReportingConfig::default());
    let inactive = evaluate(&r, &q, None, None, "offline").unwrap();
    assert!(inactive.context.reporting_view.is_none() && inactive.projection.reporting.is_none());
    assert_eq!(full_json(&r, &inactive).unwrap(), original_json);
    assert_eq!(full_csv(&r, &inactive).unwrap(), original_csv);
    let explicit = evaluate(
        &r,
        &Request {
            reporting_mode: Some(segments::ReportingMode::Raw),
            ..Default::default()
        },
        None,
        None,
        "offline",
    )
    .unwrap();
    assert_ne!(
        explicit.context.snapshot_token,
        first.context.snapshot_token
    );
}
#[tokio::test]
async fn segments_cli_shared_reporting_modes_export_and_refuse_cross_view_token_without_raw_writes()
{
    let dir = tempfile::tempdir().unwrap();
    let report_path = dir.path().join("report.json");
    let r = reporting_report();
    let captured = serde_json::to_vec(&r).unwrap();
    tokio::fs::write(&report_path, &captured).await.unwrap();
    for (label, mode, expected) in [
        ("reported", None, 299),
        ("raw", Some(segments::ReportingMode::Raw), 301),
    ] {
        let request = Request {
            reporting_mode: mode,
            ..Default::default()
        };
        let request_path = dir.path().join(format!("{label}-request.json"));
        tokio::fs::write(&request_path, serde_json::to_vec(&request).unwrap())
            .await
            .unwrap();
        let out = dir.path().join(format!("{label}.json"));
        run_cli(
            report_path.clone(),
            request_path.clone(),
            out.clone(),
            false,
        )
        .await
        .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&tokio::fs::read(out).await.unwrap()).unwrap();
        assert_eq!(json["projection"]["selected_pages"], expected);
        let out = dir.path().join(format!("{label}.csv"));
        run_cli(report_path.clone(), request_path, out.clone(), true)
            .await
            .unwrap();
        let csv = String::from_utf8(tokio::fs::read(out).await.unwrap()).unwrap();
        assert_eq!(
            csv.lines().filter(|r| r.starts_with("page,")).count(),
            expected as usize
        );
    }
    let raw = evaluate(
        &r,
        &Request {
            reporting_mode: Some(segments::ReportingMode::Raw),
            ..Default::default()
        },
        None,
        None,
        "declared_offline_report_file",
    )
    .unwrap();
    let stale = Request {
        expected_snapshot_token: Some(raw.context.snapshot_token),
        ..Default::default()
    };
    let request_path = dir.path().join("stale-request.json");
    tokio::fs::write(&request_path, serde_json::to_vec(&stale).unwrap())
        .await
        .unwrap();
    let output = dir.path().join("must-not-exist.json");
    assert!(
        run_cli(report_path.clone(), request_path, output.clone(), false)
            .await
            .is_err()
    );
    assert!(!output.exists());
    assert_eq!(tokio::fs::read(report_path).await.unwrap(), captured);
}
