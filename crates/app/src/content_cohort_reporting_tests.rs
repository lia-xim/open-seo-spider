//! Source-authored reporting regressions. Executed only later by Root, not by author.
use super::*;
use open_seo_spider_core::{CrawlConfig, CrawlSummary, DirectiveReportingConfig, PageRecord};
fn profile(mask: u8) -> Option<DirectiveReportingConfig> {
    Some(DirectiveReportingConfig {
        version: 1,
        respect_noindex: mask & 1 != 0,
        respect_canonical: mask & 2 != 0,
        respect_next_prev: mask & 4 != 0,
    })
}
fn report() -> CrawlReport {
    let pages = ["plain", "noindex", "canonical", "prev", "next-only"]
        .into_iter()
        .map(|name| {
            let url = format!("https://fixture.test/{name}");
            let mut page = PageRecord {
                url: url.clone(),
                final_url: url,
                indexable: true,
                ..Default::default()
            };
            match name {
                "noindex" => page.meta_robots = vec!["noindex".into()],
                "canonical" => page.canonical = vec!["https://fixture.test/plain".into()],
                "prev" => page.pagination_prev = vec!["https://fixture.test/plain".into()],
                "next-only" => page.pagination_next = vec!["https://fixture.test/plain".into()],
                _ => {}
            }
            page
        })
        .collect();
    CrawlReport {
        config: CrawlConfig::default(),
        pages,
        issues: vec![],
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
fn request(report: &CrawlReport) -> SavedContentRequest {
    serde_json::from_value(
        json!({"urls":report.pages.iter().map(|p|p.url.clone()).collect::<Vec<_>>()}),
    )
    .unwrap()
}
fn analyze(report: &CrawlReport, request: &SavedContentRequest) -> Value {
    let c = snapshot_context(report, Some(7), "declared_imported_report").unwrap();
    let (e, i) = prepare_saved_analysis(report, &c, request).unwrap();
    finish_saved_analysis(e, i).unwrap()
}
#[test]
fn content_reporting_all_eight_saved_profiles_filter_before_selection_and_pair_corpus_keep_raw_ordinals()
 {
    for mask in 0..8 {
        let mut r = report();
        r.config.directive_reporting = profile(mask);
        let before = serde_json::to_vec(&r).unwrap();
        let c = snapshot_context(&r, Some(7), "declared_imported_report").unwrap();
        let expected = (0..5)
            .filter(|n| match n {
                1 => mask & 1 == 0,
                2 => mask & 2 == 0,
                3 => mask & 4 == 0,
                _ => true,
            })
            .collect::<Vec<_>>();
        let inventory = inventory(&r, &c, default_source(), 0, 200).unwrap();
        assert_eq!(
            inventory["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["ordinal"].as_u64().unwrap() as usize)
                .collect::<Vec<_>>(),
            expected
        );
        let result = analyze(&r, &request(&r));
        let n = expected.len();
        assert_eq!(result["included_count"], n);
        assert_eq!(result["analysis"]["documents"].as_array().unwrap().len(), n);
        assert_eq!(
            result["analysis"]["pairs"].as_array().unwrap().len(),
            n * (n - 1) / 2
        );
        assert_eq!(
            result["documents"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["row"]["ordinal"].as_u64().unwrap() as usize)
                .collect::<Vec<_>>(),
            expected
        );
        for excluded in result["excluded"].as_array().unwrap() {
            assert_eq!(excluded["reason"], "excluded_reporting");
            assert_eq!(excluded["row"]["status"], "excluded_reporting");
            assert_eq!(excluded["row"]["capture_status"], "missing");
        }
        if mask == 0 {
            assert!(result.get("reporting").is_none());
        } else {
            assert_eq!(result["reporting"]["captured_page_observations"], 5);
            assert_eq!(result["excluded_reporting_count"], 5 - n);
        }
        assert_eq!(serde_json::to_vec(&r).unwrap(), before);
    }
}
#[test]
fn content_reporting_all_hidden_is_explicit_known_exclusion_unknown_url_still_refuses() {
    let mut r = report();
    r.pages.retain(|p| p.url.ends_with("/noindex"));
    r.config.directive_reporting = profile(1);
    let c = snapshot_context(&r, Some(7), "declared_imported_report").unwrap();
    let result = analyze(&r, &request(&r));
    assert_eq!(result["status"], "empty_filtered_cohort");
    assert!(result["analysis"].is_null());
    assert_eq!(result["excluded_reporting_count"], 1);
    assert_eq!(result["included_count"], 0);
    let document = saved_document(&r, &c, default_source(), &r.pages[0].url).unwrap();
    assert_eq!(document["status"], "excluded_reporting");
    assert!(document["capture"].is_null());
    assert_eq!(document["row"]["requested_url"], r.pages[0].url);
    let mut q = request(&r);
    q.urls.push("https://fixture.test/unknown".into());
    assert!(
        prepare_saved_analysis(&r, &c, &q)
            .unwrap_err()
            .to_string()
            .contains("not in the saved report")
    );
    assert!(saved_document(&r, &c, default_source(), "https://fixture.test/unknown").is_err());
}
#[test]
fn content_reporting_inactive_absent_null_and_all_false_keep_legacy_json_bytes() {
    let mut r = report();
    let q = request(&r);
    let baseline = analyze(&r, &q);
    let c = snapshot_context(&r, Some(7), "declared_imported_report").unwrap();
    let original_inventory = inventory(&r, &c, default_source(), 0, 2).unwrap();
    let original_document = saved_document(&r, &c, default_source(), &r.pages[0].url).unwrap();
    let old_inventory = json!({"engine":SAVED_CONTENT_ENGINE,"snapshot":c,"source_kind":default_source(),"after_ordinal":0,"limit":2,"total_saved_pages":5,
        "rows":[document_summary(&r,0,default_source()).unwrap(),document_summary(&r,1,default_source()).unwrap()],"next_after_ordinal":2,"has_more":true,"text_included":false,"website_requests":0,"site_wide_coverage":false});
    assert_eq!(
        serde_json::to_vec(&original_inventory).unwrap(),
        serde_json::to_vec(&old_inventory).unwrap()
    );
    for profile in [None, profile(0)] {
        r.config.directive_reporting = profile;
        let mut wire = serde_json::to_value(&q).unwrap();
        wire["reporting_mode"] = Value::Null;
        let q: SavedContentRequest = serde_json::from_value(wire).unwrap();
        assert_eq!(
            serde_json::to_vec(&analyze(&r, &q)).unwrap(),
            serde_json::to_vec(&baseline).unwrap()
        );
        let c = snapshot_context(&r, Some(7), "declared_imported_report").unwrap();
        assert_eq!(
            inventory(&r, &c, default_source(), 0, 2).unwrap(),
            original_inventory
        );
        assert_eq!(
            saved_document(&r, &c, default_source(), &r.pages[0].url).unwrap(),
            original_document
        );
        assert!(
            serde_json::to_value(&q)
                .unwrap()
                .get("reporting_mode")
                .is_none()
        );
    }
}
#[test]
fn content_reporting_equal_counts_still_bind_mode_profile_and_analysis_identity_separately() {
    let mut r = report();
    r.pages.truncate(1);
    r.config.directive_reporting = profile(1);
    let mut q = request(&r);
    q.reporting_mode = Some(ReportingMode::Raw);
    let raw = analyze(&r, &q);
    q.reporting_mode = Some(ReportingMode::Reported);
    let visible = analyze(&r, &q);
    assert_eq!(raw["included_count"], visible["included_count"]);
    assert_eq!(raw["snapshot"]["sha256"], visible["snapshot"]["sha256"]);
    assert_ne!(
        raw["reporting"]["identity_sha256"],
        visible["reporting"]["identity_sha256"]
    );
    assert_ne!(
        raw["analysis"]["input_sha256"],
        visible["analysis"]["input_sha256"]
    );
    assert_eq!(raw["selection_sha256"], visible["selection_sha256"]);
    let first = visible["reporting"]["identity_sha256"].clone();
    r.config.directive_reporting = profile(2);
    assert_ne!(analyze(&r, &q)["reporting"]["identity_sha256"], first);
}
#[test]
fn content_reporting_generation_raw_snapshot_and_view_guards_refuse_stale_all_frontdoors() {
    let mut r = report();
    r.config.directive_reporting = profile(1);
    let c = snapshot_context(&r, Some(7), "declared_imported_report").unwrap();
    let v = inventory(&r, &c, default_source(), 0, 200).unwrap();
    let options = ReportingOptions {
        expected_generation: Some(7),
        expected_snapshot_sha256: Some(c.sha256.clone()),
        expected_reporting_identity: Some(
            v["reporting"]["identity_sha256"].as_str().unwrap().into(),
        ),
        ..Default::default()
    };
    for changed in 0..4 {
        let mut stale = options.clone();
        match changed {
            0 => stale.expected_generation = Some(6),
            1 => stale.expected_snapshot_sha256 = Some("a".repeat(64)),
            2 => stale.mode = Some(ReportingMode::Raw),
            _ => stale.expected_reporting_identity = Some("b".repeat(64)),
        }
        assert!(inventory_with_reporting(&r, &c, default_source(), 0, 200, &stale).is_err());
        assert!(
            saved_document_with_reporting(&r, &c, default_source(), &r.pages[0].url, &stale)
                .is_err()
        );
        let mut q = request(&r);
        q.reporting_mode = stale.mode;
        q.expected_generation = stale.expected_generation;
        q.expected_snapshot_sha256 = stale.expected_snapshot_sha256;
        q.expected_reporting_identity = stale.expected_reporting_identity;
        assert!(prepare_saved_analysis(&r, &c, &q).is_err());
    }
}
#[test]
fn content_reporting_raw_ordinal_paging_can_be_empty_without_faking_complete_report() {
    let mut r = report();
    r.config.directive_reporting = profile(7);
    let c = snapshot_context(&r, None, "declared_imported_report").unwrap();
    let page = inventory(&r, &c, default_source(), 1, 2).unwrap();
    assert_eq!(page["rows"], json!([]));
    assert_eq!(page["has_more"], true);
    assert_eq!(page["next_after_ordinal"], 3);
    assert_eq!(page["total_saved_pages"], 5);
    assert_eq!(page["reporting"]["eligible_page_observations"], 2);
}
#[test]
fn content_reporting_modes_strict_explicit128_limit_and_duplicate_known_identity_are_preserved() {
    let mut r = report();
    r.config.directive_reporting = profile(7);
    let c = snapshot_context(&r, None, "declared_imported_report").unwrap();
    for bad in [json!("auto"), json!("RAW"), json!(true), json!({})] {
        assert!(
            serde_json::from_value::<SavedContentRequest>(
                json!({"urls":[r.pages[0].url],"reporting_mode":bad})
            )
            .is_err()
        );
    }
    let mut q = request(&r);
    q.urls = vec![r.pages[0].url.clone(); 129];
    assert!(prepare_saved_analysis(&r, &c, &q).is_err());
    q = request(&r);
    q.urls.push(q.urls[0].clone());
    assert!(prepare_saved_analysis(&r, &c, &q).is_err());
    r.pages.push(r.pages[0].clone());
    assert!(prepare_saved_analysis(&r, &c, &request(&r)).is_err());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn content_reporting_real_capture_saved_reopen_no_hidden_text_analysis_leak_or_binding_repair()
 {
    use axum::{Router, routing::get};
    use open_seo_spider_core::{CrawlControl, Crawler};
    let html = "<html lang='en'><head><meta name='robots' content='noindex'></head><body>secretcapturedword repeated secretcapturedword</body></html>";
    let site = Router::new().route(
        "/",
        get(move || async move { ([("content-type", "text/html")], html) }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, site)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let crawler = Crawler::new(CrawlConfig {
        seed_urls: vec![url],
        content_capture: Some(Default::default()),
        crawl_links: false,
        obey_robots: false,
        discover_sitemaps: false,
        request_timeout_secs: 5,
        ..Default::default()
    })
    .unwrap();
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        crawler.run_controlled(None, CrawlControl::new()),
    )
    .await;
    drop(crawler);
    let _ = stop.send(());
    server.await.unwrap();
    let mut r = outcome.unwrap().unwrap().report;
    r.config.directive_reporting = profile(1);
    let bytes = serde_json::to_vec(&r).unwrap();
    let reopened: CrawlReport = serde_json::from_slice(&bytes).unwrap();
    let q = request(&reopened);
    let hidden = analyze(&reopened, &q);
    assert_eq!(hidden["status"], "empty_filtered_cohort");
    assert_eq!(hidden["excluded"][0]["row"]["capture_status"], "complete");
    assert!(
        !serde_json::to_string(&hidden)
            .unwrap()
            .contains("secretcapturedword")
    );
    let c = snapshot_context(&reopened, Some(7), "declared_imported_report").unwrap();
    let hidden_document =
        saved_document(&reopened, &c, default_source(), &reopened.pages[0].url).unwrap();
    assert_eq!(hidden_document["status"], "excluded_reporting");
    assert!(hidden_document["capture"].is_null());
    assert!(
        !serde_json::to_string(&hidden_document)
            .unwrap()
            .contains("secretcapturedword")
    );
    let options = ReportingOptions {
        mode: Some(ReportingMode::Raw),
        ..Default::default()
    };
    let raw = saved_document_with_reporting(
        &reopened,
        &c,
        default_source(),
        &reopened.pages[0].url,
        &options,
    )
    .unwrap();
    assert!(
        raw["capture"]["result"]["text"]
            .as_str()
            .unwrap()
            .contains("secretcapturedword")
    );
    assert_eq!(
        raw["capture"]["source_revision"],
        hidden_document["row"]["source_revision"]
    );
    assert_eq!(serde_json::to_vec(&reopened).unwrap(), bytes);
    r.pages[0].content_captures.as_mut().unwrap()[0]
        .result
        .as_mut()
        .unwrap()
        .text = Some("tampered".into());
    let c = snapshot_context(&r, Some(7), "declared_imported_report").unwrap();
    assert!(prepare_saved_analysis(&r, &c, &request(&r)).is_err());
}
