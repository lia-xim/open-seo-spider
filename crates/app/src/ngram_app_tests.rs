use super::*;
use open_seo_spider_core::{NgramCorpusDocument, PageRecord};
pub(super) fn legacy_report() -> CrawlReport {
    CrawlReport {
        config: Default::default(),
        pages: vec![PageRecord {
            url: "https://example.test/legacy".into(),
            final_url: "https://example.test/legacy".into(),
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
#[test]
fn saved_ngram_projection_token_binds_every_filter_limit_source_and_snapshot_with_complete_csv() {
    let docs = [NgramCorpusDocument {
        requested_url: "https://example.test/a",
        source_revision: Some("own-declared-fixture"),
        capture_config_sha256: None,
        text_sha256: None,
        text: Some("red red blue"),
        complete: true,
        basis: "declared_pure_projection_test",
        observed_generation: None,
    }];
    let report = aggregate_saved_ngrams(&docs, &Default::default()).unwrap();
    let report_source = legacy_report();
    let context = context(&report_source, Some("job".into()), Some(7), "declared").unwrap();
    let mut request = Request {
        limit: 1,
        ..Default::default()
    };
    let (_, snapshot) = preview(&report, context.clone(), &request).unwrap();
    request.expected_snapshot_token = Some(snapshot.snapshot_token.clone());
    request.after = Some(1);
    let (next, _) = preview(&report, context.clone(), &request).unwrap();
    let next: Value = serde_json::from_slice(&next).unwrap();
    assert_eq!(next["rows"].as_array().unwrap().len(), 1);
    assert!(csv(&report, context.clone(), &request).is_err());
    request.after = None;
    let (bytes, _) = csv(&report, context.clone(), &request).unwrap();
    assert_eq!(
        std::str::from_utf8(&bytes).unwrap().lines().count(),
        report.rows.len() + 1
    );
    for i in 0..5 {
        let mut changed = request.clone();
        match i {
            0 => changed.query = "red".into(),
            1 => changed.limit = 2,
            2 => changed.n = Some(1),
            3 => changed.source_kind = ContentCaptureSourceKind::RenderedDomHtml,
            _ => {
                changed.options.case_behavior = open_seo_spider_core::ContentCaseBehavior::Preserve
            }
        };
        assert!(preview(&report, context.clone(), &changed).is_err());
    }
    let mut changed = context;
    changed.typed_report_sha256 = "changed".into();
    assert!(csv(&report, changed, &request).is_err());
}
#[test]
fn saved_ngram_missing_and_pending_sources_remain_unknown_strict_options_and_json_caps_fail() {
    let mut report = legacy_report();
    let mut request = Request {
        view: View::Sources,
        ..Default::default()
    };
    let r = aggregate(&report, &request).unwrap();
    assert_eq!(r.source_states["missing_or_unavailable"], 1);
    assert!(r.rows.is_empty());
    report.operation=Some(serde_json::from_value(json!({"engine":open_seo_spider_core::SELECTED_RECRAWL_ENGINE,"coverage":open_seo_spider_core::SELECTED_RECRAWL_COVERAGE,"parent":{"job_id":"parent","checkpoint_revision":1,"source_kind":"stored_json","snapshot_sha256":"declared"},"plan_sha256":"declared","inherited_config_sha256":"declared","selection":[],"retained_count":0,"refreshed_count":0,"pending_count":1,"max_attempts":null,"admitted_this_run":0,"committed_this_run":0,"targets":[{"requested_url":report.pages[0].url,"parent_ordinal":0,"parent_page_sha256":"declared","state":"pending","admitted_attempts":0,"committed_generation":null}]})).unwrap());
    let r = aggregate(&report, &request).unwrap();
    assert_eq!(r.sources[0].basis, "pending_selected");
    assert!(!r.sources[0].counted);
    request.after = Some(0);
    assert!(request.validate().is_err());
    for value in [
        json!({"html":"invented"}),
        json!({"report":{}}),
        json!({"options":{"limits":{"unbounded":true}}}),
    ] {
        assert!(serde_json::from_value::<Request>(value).is_err());
    }
    assert!(bytes(&json!({"large":"x".repeat(JSON_BYTES)}), JSON_BYTES).is_err());
}
#[tokio::test]
async fn saved_ngram_cli_reopen_missing_source_never_overwrites_report_options_or_existing_output()
{
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("report.json");
    let options = dir.path().join("options.json");
    tokio::fs::write(&report, serde_json::to_vec(&legacy_report()).unwrap())
        .await
        .unwrap();
    tokio::fs::write(&options, b"{\"view\":\"sources\"}")
        .await
        .unwrap();
    let before = tokio::fs::read(&report).await.unwrap();
    for out in [report.clone(), options.clone()] {
        assert!(
            run_cli(report.clone(), Some(options.clone()), out, None)
                .await
                .is_err()
        );
    }
    let out = dir.path().join("page.json");
    let full = dir.path().join("sources.csv");
    run_cli(
        report.clone(),
        Some(options.clone()),
        out.clone(),
        Some(full.clone()),
    )
    .await
    .unwrap();
    let page: Value = serde_json::from_slice(&tokio::fs::read(&out).await.unwrap()).unwrap();
    assert_eq!(page["rows"][0]["source"]["state"], "missing_or_unavailable");
    assert_eq!(page["rows"][0]["source"]["counted"], false);
    assert_eq!(page["website_requests"], 0);
    assert!(
        std::str::from_utf8(&tokio::fs::read(full).await.unwrap())
            .unwrap()
            .contains("missing_or_unavailable")
    );
    assert!(
        run_cli(report.clone(), Some(options), out, None)
            .await
            .is_err()
    );
    assert_eq!(tokio::fs::read(report).await.unwrap(), before);
}

#[test]
fn saved_ngram_extended_filter_csv_and_snapshot_range_are_bound_and_strict() {
    let docs = [NgramCorpusDocument {
        requested_url: "https://example.test/six",
        source_revision: Some("declared-six"),
        capture_config_sha256: None,
        text_sha256: None,
        text: Some("a b c d e f a b c d e f"),
        complete: true,
        basis: "declared_pure_projection_test",
        observed_generation: None,
    }];
    let options = NgramCorpusOptions {
        max_n: 6,
        ..Default::default()
    };
    let report = aggregate_saved_ngrams(&docs, &options).unwrap();
    let mut request = Request {
        options,
        n: Some(6),
        view: View::Occurrences,
        exact_tokens: Some(["a", "b", "c", "d", "e", "f"].map(str::to_owned).to_vec()),
        ..Default::default()
    };
    assert!(request.validate().is_ok());
    let context = context(&legacy_report(), Some("job".into()), Some(7), "declared").unwrap();
    let (bytes, snapshot) = preview(&report, context.clone(), &request).unwrap();
    let page: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(page["max_n"], 6);
    assert_eq!(page["total_rows"], 1);
    assert_eq!(page["rows"][0]["occurrences"], 2);
    assert_eq!(page["rows"][0]["first_positions"], json!([1, 7]));
    assert_eq!(page["extended_windows"][2]["n"], 6);
    request.expected_snapshot_token = Some(snapshot.snapshot_token);
    let (bytes, _) = csv(&report, context.clone(), &request).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert_eq!(text.lines().count(), 2);
    assert!(text.contains("https://example.test/six"));
    assert!(text.contains("[1,7]"));
    let mut changed = request.clone();
    changed.options.max_n = 5;
    assert!(changed.validate().is_err());
    assert!(csv(&report, context.clone(), &changed).is_err());
    changed = request.clone();
    changed.n = Some(5);
    assert!(preview(&report, context.clone(), &changed).is_err());
    changed = request.clone();
    changed.exact_tokens.as_mut().unwrap().push("seven".into());
    assert!(changed.validate().is_err());
    changed = request;
    changed.expected_snapshot_token = None;
    changed.options.max_n = 7;
    assert!(changed.validate().is_err());
    let defaults: Request = serde_json::from_str("{}").unwrap();
    assert_eq!(defaults.options.max_n, 3);
    let old6: Request = serde_json::from_str(r#"{"n":6}"#).unwrap();
    assert!(old6.validate().is_err());
    let explicit: Request = serde_json::from_str(r#"{"options":{"max_n":6},"n":6}"#).unwrap();
    assert!(explicit.validate().is_ok());
}
