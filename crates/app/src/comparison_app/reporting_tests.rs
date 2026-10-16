//! Source26 regressions: raw joining precedes side-specific presentation.
use super::*;
use open_seo_spider_core::{DirectiveReportingConfig, PageRecord, ReportingMode};
fn page(url: &str, extra: &str) -> PageRecord {
    let mut p = PageRecord {
        url: url.into(),
        final_url: url.into(),
        status: Some(200),
        final_status: Some(200),
        content_type: "text/html".into(),
        fetched_at: "2026-10-16T00:00:00Z".into(),
        ..Default::default()
    };
    let html = format!(
        "<html><head><title>Saved</title>{extra}</head><body><a href='/target'>Target</a></body></html>"
    );
    open_seo_spider_core::extract_html(&mut p, &html, &[reqwest::Url::parse(url).unwrap()], false);
    p
}
fn report(pages: Vec<PageRecord>, mask: u8) -> CrawlReport {
    CrawlReport {
        config: open_seo_spider_core::CrawlConfig {
            directive_reporting: (mask != 0).then_some(DirectiveReportingConfig {
                version: 1,
                respect_noindex: mask & 1 != 0,
                respect_canonical: mask & 2 != 0,
                respect_next_prev: mask & 4 != 0,
            }),
            ..Default::default()
        },
        pages,
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
fn raw(a: &CrawlReport, b: &CrawlReport) -> SavedComparisonResult {
    compare_saved_crawls(a, b, &Default::default(), None, None).unwrap()
}
fn view(
    a: &CrawlReport,
    b: &CrawlReport,
    mode: Option<ReportingMode>,
) -> reporting::ComparisonReporting {
    reporting::build(&raw(a, b), a, b, mode, None, None)
        .unwrap()
        .unwrap()
}
#[test]
fn reporting_inactive_none_null_and_allfalse_keep_exact_legacy_json_csv_and_hashes() {
    let a = report(vec![page("https://fixture.test/a", "")], 0);
    let before = raw(&a, &a);
    let old = SavedComparisonResponse::new(before.clone()).unwrap();
    let mut old_csv = vec![];
    write_csv(&before, &mut old_csv).unwrap();
    for value in [
        Value::Null,
        json!({"version":1,"respect_noindex":false,"respect_canonical":false,"respect_next_prev":false}),
    ] {
        let mut b = a.clone();
        b.config.directive_reporting = serde_json::from_value(value).unwrap();
        let result = raw(&b, &b);
        assert_eq!(
            serde_json::to_vec(&result).unwrap(),
            serde_json::to_vec(&before).unwrap()
        );
        let reporting = reporting::build(&result, &b, &b, None, None, None).unwrap();
        assert!(reporting.is_none());
        let response = SavedComparisonResponse::new(result)
            .unwrap()
            .with_reporting(reporting)
            .unwrap();
        assert_eq!(
            serde_json::to_vec(&response).unwrap(),
            serde_json::to_vec(&old).unwrap()
        );
        let mut csv = vec![];
        write_csv_projection_with_limit(&response.comparison, None, &mut csv, MAX_CSV_BYTES)
            .unwrap();
        assert_eq!(csv, old_csv);
    }
}
#[test]
fn reporting_all64_saved_profile_pairs_use_each_side_without_touching_raw_facts() {
    let htmls = [
        "",
        "<meta name='robots' content='noindex'>",
        "<link rel='canonical' href='https://fixture.test/other'>",
        "<link rel='prev' href='https://fixture.test/prev'>",
    ];
    for left in 0..8 {
        for right in 0..8 {
            let a = report(
                htmls
                    .iter()
                    .enumerate()
                    .map(|(i, h)| page(&format!("https://fixture.test/{i}"), h))
                    .collect(),
                left,
            );
            let b = report(a.pages.clone(), right);
            let result = raw(&a, &b);
            let exact = serde_json::to_vec(&result).unwrap();
            let automatic = reporting::build(&result, &a, &b, None, None, None).unwrap();
            if left == 0 && right == 0 {
                assert!(automatic.is_none());
            } else {
                assert_eq!(automatic.unwrap().identity.mode, ReportingMode::Reported);
            }
            for mode in [ReportingMode::Raw, ReportingMode::Reported] {
                let v = reporting::build(&result, &a, &b, Some(mode), None, None)
                    .unwrap()
                    .unwrap();
                for r in &v.rows {
                    let p = &result.pages[r.comparison_index];
                    let av = open_seo_spider_core::directive_page_visible(
                        &a.config,
                        &a.pages[p.before.as_ref().unwrap().ordinal],
                    );
                    let bv = open_seo_spider_core::directive_page_visible(
                        &b.config,
                        &b.pages[p.after.as_ref().unwrap().ordinal],
                    );
                    assert_eq!((r.before_visible, r.after_visible), (Some(av), Some(bv)));
                    assert_eq!(r.included, mode == ReportingMode::Raw || av || bv);
                }
                assert_eq!(serde_json::to_vec(&result).unwrap(), exact);
            }
        }
    }
}
#[test]
fn reporting_hidden_to_visible_and_reverse_never_manufacture_added_or_removed() {
    let hidden = report(
        vec![page(
            "https://fixture.test/a",
            "<meta name='robots' content='noindex'>",
        )],
        1,
    );
    let visible = report(vec![page("https://fixture.test/a", "")], 1);
    for (a, b) in [(&hidden, &visible), (&visible, &hidden)] {
        let result = raw(a, b);
        let v = view(a, b, None);
        assert_eq!(result.pages.len(), 1);
        assert!(v.rows[0].included);
        assert_eq!(result.counts.added, 0);
        assert_eq!(result.counts.removed, 0);
        assert_eq!(v.view_counts, result.counts);
        assert_eq!(
            result.pages[0].kind,
            open_seo_spider_core::ComparisonPageKind::Changed
        );
    }
}
#[test]
fn reporting_hidden_only_absent_membership_is_excluded_after_raw_classification() {
    let a = report(
        vec![page(
            "https://fixture.test/a",
            "<meta name='robots' content='noindex'>",
        )],
        1,
    );
    let b = report(vec![], 1);
    for (a, b) in [(&a, &b), (&b, &a)] {
        let result = raw(a, b);
        let v = view(a, b, Some(ReportingMode::Reported));
        assert_eq!(result.pages.len(), 1);
        assert!(!v.rows[0].included);
        assert_eq!(v.included_compared_rows, 0);
        assert_eq!(v.excluded_compared_rows, 1);
        assert_eq!(v.view_counts, Default::default());
    }
}
#[test]
fn reporting_raw_keeps_hidden_rows_and_full_target_fields() {
    let a = report(
        vec![page(
            "https://fixture.test/a",
            "<meta name='robots' content='noindex'>",
        )],
        1,
    );
    let result = raw(&a, &a);
    let v = view(&a, &a, Some(ReportingMode::Raw));
    assert_eq!(v.included_compared_rows, 1);
    assert_eq!(v.rows[0].before_visible, Some(false));
    assert!(
        result.pages[0]
            .fields
            .iter()
            .any(|f| f.field.contains("links"))
    );
    assert_eq!(v.raw_counts, v.view_counts);
}
#[test]
fn reporting_raw_duplicate_and_mapping_collision_refusals_cannot_be_hidden() {
    let p = page(
        "https://fixture.test/a",
        "<meta name='robots' content='noindex'>",
    );
    let a = report(vec![p.clone(), p], 1);
    let b = report(vec![], 1);
    let e = compare_saved_crawls(&a, &b, &Default::default(), None, None).unwrap_err();
    assert_eq!(e.code, "ambiguous_snapshot_identity");
    assert_eq!(e.collisions[0].occurrences.len(), 2);
    let a = report(
        vec![
            page("https://fixture.test/a", ""),
            page(
                "https://fixture.test/b",
                "<meta name='robots' content='noindex'>",
            ),
        ],
        1,
    );
    let mut config = SavedComparisonConfig::default();
    config
        .mappings
        .push(open_seo_spider_core::ComparisonUrlMapping {
            side: ComparisonSide::Before,
            from_origin: "https://fixture.test".into(),
            from_path_prefix: "/b".into(),
            to_origin: "https://fixture.test".into(),
            to_path_prefix: "/a".into(),
        });
    assert_eq!(
        compare_saved_crawls(&a, &b, &config, None, None)
            .unwrap_err()
            .code,
        "ambiguous_snapshot_identity"
    );
    // Projection is only reachable after these complete raw collision checks.
}
#[test]
fn reporting_equal_population_profiles_and_raw_report_changes_have_distinct_bindings() {
    let a = report(vec![page("https://fixture.test/a", "")], 1);
    let b = report(a.pages.clone(), 2);
    let one = view(&a, &a, Some(ReportingMode::Raw));
    let two = view(&a, &b, Some(ReportingMode::Raw));
    assert_eq!(one.included_compared_rows, two.included_compared_rows);
    assert_ne!(one.identity_sha256, two.identity_sha256);
    assert_eq!(
        reporting::build(
            &raw(&a, &b),
            &a,
            &b,
            Some(ReportingMode::Raw),
            Some(&one.identity_sha256),
            None
        )
        .unwrap_err()
        .error
        .code,
        "comparison_reporting_changed"
    );
    let mut changed = a.clone();
    changed.pages[0].fetched_at = "2026-10-17T00:00:00Z".into();
    let next = view(&a, &changed, Some(ReportingMode::Raw));
    assert_eq!(one.identity_sha256, next.identity_sha256);
    assert_ne!(one.view_sha256, next.view_sha256);
    assert!(
        reporting::build(
            &raw(&a, &changed),
            &a,
            &changed,
            Some(ReportingMode::Raw),
            None,
            Some(&one.view_sha256)
        )
        .is_err()
    );
    let config = SavedComparisonConfig {
        include_selected_text: true,
        ..Default::default()
    };
    let configured = compare_saved_crawls(&a, &a, &config, None, None).unwrap();
    let other = reporting::build(&configured, &a, &a, Some(ReportingMode::Raw), None, None)
        .unwrap()
        .unwrap();
    assert_eq!(one.identity_sha256, other.identity_sha256);
    assert_ne!(one.view_sha256, other.view_sha256);
}
#[test]
fn reporting_missing_auto_identity_invalid_profile_and_tampered_index_refuse() {
    let a = report(vec![page("https://fixture.test/a", "")], 0);
    let result = raw(&a, &a);
    assert!(reporting::build(&result, &a, &a, None, Some(&"a".repeat(64)), None).is_err());
    let mut bad = a.clone();
    bad.config.directive_reporting = Some(DirectiveReportingConfig {
        version: 2,
        ..Default::default()
    });
    assert_eq!(
        reporting::build(&result, &bad, &a, Some(ReportingMode::Raw), None, None)
            .unwrap_err()
            .error
            .code,
        "invalid_comparison_reporting_profile"
    );
    let original = view(&a, &a, Some(ReportingMode::Raw));
    for kind in 0..5 {
        let mut v = original.clone();
        match kind {
            0 => v.rows[0].comparison_index = 1,
            1 => v.rows[0].comparison_key.push('x'),
            2 => v.rows[0].included = false,
            3 => v.view_counts.added += 1,
            _ => v.identity_sha256 = "b".repeat(64),
        }
        assert!(v.validate(&result).is_err());
    }
}
#[test]
fn reporting_empty_view_csv_retains_typed_metadata_and_default_csv_has_no_new_columns() {
    let a = report(
        vec![page(
            "https://fixture.test/a",
            "<meta name='robots' content='noindex'>",
        )],
        1,
    );
    let result = raw(&a, &a);
    let v = view(&a, &a, None);
    let mut csv = vec![];
    write_csv_projection_with_limit(&result, Some(&v), &mut csv, MAX_CSV_BYTES).unwrap();
    let text = String::from_utf8(csv).unwrap();
    assert_eq!(text.matches("\r\n").count(), 2);
    assert!(text.contains("reporting_json"));
    assert!(text.contains("saved-comparison-reporting-v1"));
    assert!(
        !text.contains("https://fixture.test/a"),
        "Empty view CSV summary has no hidden row keys or raw field data"
    );
    let mut legacy = vec![];
    write_csv(&result, &mut legacy).unwrap();
    assert!(
        !String::from_utf8(legacy)
            .unwrap()
            .contains("reporting_json")
    );
}
#[test]
fn reporting_original_bundle_stays_complete_and_raw_side_metadata_is_immutable() {
    let a = report(
        vec![page(
            "https://fixture.test/a",
            "<meta name='robots' content='noindex'>",
        )],
        1,
    );
    let result = raw(&a, &a);
    let v = reporting::build(&result, &a, &a, None, None, None).unwrap();
    let response = SavedComparisonResponse::with_original_reports(result, a.clone(), a.clone())
        .unwrap()
        .with_reporting(v)
        .unwrap();
    response.validate_original_reports().unwrap();
    assert_eq!(
        response.reporting.as_ref().unwrap().included_compared_rows,
        0
    );
    assert_eq!(
        serde_json::to_vec(&response.original_reports.as_ref().unwrap().before.report).unwrap(),
        serde_json::to_vec(&a).unwrap()
    );
    assert_eq!(response.comparison.pages.len(), 1);
}
#[test]
fn reporting_request_is_strict_and_modes_and_guards_are_advertised() {
    let base = json!({"before":"00000000-0000-0000-0000-000000000001","after":"00000000-0000-0000-0000-000000000002"});
    for mode in [json!("auto"), json!(false), json!(0), json!({})] {
        let mut v = base.clone();
        v["reporting_mode"] = mode;
        assert!(serde_json::from_value::<SavedComparisonRequest>(v).is_err());
    }
    for mode in ["raw", "reported"] {
        let mut v = base.clone();
        v["reporting_mode"] = json!(mode);
        let parsed: SavedComparisonRequest = serde_json::from_value(v).unwrap();
        assert!(parsed.reporting_mode.is_some());
    }
    let schema = request_schema();
    for key in [
        "reporting_mode",
        "expected_reporting_identity",
        "expected_reporting_view",
    ] {
        assert!(schema["properties"].get(key).is_some());
    }
}
