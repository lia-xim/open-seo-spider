use super::*;
use open_seo_spider_core::{
    DomLocation, RecrawlParentReference, RecrawlSourceKind, RedirectHop, SelectedRecrawlOperation,
};
fn page(url: &str, status: u16) -> PageRecord {
    PageRecord {
        url: url.into(),
        final_url: url.into(),
        status: Some(status),
        final_status: Some(status),
        fetched_at: "2026-10-15T12:00:05Z".into(),
        ..Default::default()
    }
}
fn link(url: &str, text: &str) -> LinkRecord {
    LinkRecord {
        url: url.into(),
        text: text.into(),
        kind: "anchor".into(),
        ..Default::default()
    }
}
fn report(pages: Vec<PageRecord>) -> CrawlReport {
    CrawlReport {
        config: Default::default(),
        pages,
        issues: vec![],
        summary: open_seo_spider_core::CrawlSummary {
            started_at: "2026-10-15T12:00:00Z".into(),
            finished_at: "2026-10-15T12:00:10Z".into(),
            ..Default::default()
        },
        warnings: vec![],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
        sitemap_source_measurements: None,
    }
}
fn evaluated(report: CrawlReport, filter: Filter) -> Evaluation {
    let config = digest(&report.config, REPORT_BYTES).unwrap().0;
    let binding = ComparisonCallerBinding {
        origin: Some("app_declared_job_report_without_cached_checkpoint".into()),
        active_checkpoint: Some(false),
        ..Default::default()
    };
    let snapshot =
        Snapshot::new(&report, Uuid::nil(), 7, "completed".into(), config, binding).unwrap();
    evaluate(report, snapshot, filter).unwrap()
}
fn json(e: &Evaluation, r: &Request) -> serde_json::Value {
    serde_json::from_slice(&json_bytes(&e.page(r).unwrap()).unwrap()).unwrap()
}

fn reporting_profile(mask: u8) -> open_seo_spider_core::DirectiveReportingConfig {
    open_seo_spider_core::DirectiveReportingConfig {
        version: 1,
        respect_noindex: mask & 1 != 0,
        respect_canonical: mask & 2 != 0,
        respect_next_prev: mask & 4 != 0,
    }
}
fn reporting_report() -> CrawlReport {
    let mut pages = Vec::new();
    for name in ["plain", "noindex", "canonical", "prev", "next-only"] {
        let mut p = page(&format!("https://a.test/{name}"), 200);
        p.links = vec![link("https://a.test/target", "retained exact occurrence")];
        match name {
            "noindex" => p.meta_robots = vec!["noindex".into()],
            "canonical" => p.canonical = vec!["https://a.test/plain".into()],
            "prev" => p.pagination_prev = vec!["https://a.test/plain".into()],
            "next-only" => p.pagination_next = vec!["https://a.test/plain".into()],
            _ => {}
        }
        pages.push(p);
    }
    let mut target = page("https://a.test/target", 404);
    target.meta_robots = vec!["noindex".into()];
    pages.push(target);
    report(pages)
}
fn reporting_evaluation(
    report: CrawlReport,
    mode: Option<open_seo_spider_core::ReportingMode>,
) -> Evaluation {
    let snapshot = evaluated(report.clone(), Filter::All).snapshot;
    evaluate_with_reporting(report, snapshot, Filter::All, mode).unwrap()
}
#[test]
fn broken_source_reporting_eight_profiles_keep_hidden_target404_full_facts_raw_ordinals_and_raw_report()
 {
    for mask in 0..8 {
        let mut report = reporting_report();
        report.config.directive_reporting = Some(reporting_profile(mask));
        let raw = serde_json::to_vec(&report).unwrap();
        let e = reporting_evaluation(report, None);
        let q = Request {
            filter: Filter::All,
            ..Default::default()
        };
        let p = json(&e, &q);
        let expected = (0..5)
            .filter(|n| match n {
                1 => mask & 1 == 0,
                2 => mask & 2 == 0,
                3 => mask & 4 == 0,
                _ => true,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            p["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["source"]["page_ordinal"].as_u64().unwrap() as usize)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(p["total_retained_link_occurrences"], 5);
        for row in p["rows"].as_array().unwrap() {
            assert_eq!(row["verdict"], "observed_http_failure");
            assert_eq!(row["observation_count"], 1);
            assert_eq!(row["target_observations"][0]["final_status"], 404);
        }
        assert_eq!(serde_json::to_vec(&e.report).unwrap(), raw);
        if mask > 0 {
            assert_eq!(p["reporting"]["raw_summary"]["observed_http_failure"], 5);
            assert_eq!(p["reporting"]["eligible_link_occurrences"], expected.len());
        }
        let csv = String::from_utf8(e.export_csv().unwrap()).unwrap();
        assert_eq!(csv.lines().count(), expected.len() + 1);
    }
}
#[test]
fn broken_source_reporting_legacy_null_all_false_json_csv_token_exact_and_equalcount_view_tokens_differ()
 {
    let report = reporting_report();
    let baseline = reporting_evaluation(report.clone(), None);
    let q = Request {
        filter: Filter::All,
        ..Default::default()
    };
    let bytes = json_bytes(&baseline.page(&q).unwrap()).unwrap();
    let csv = baseline.export_csv().unwrap();
    let mut inactive = report.clone();
    inactive.config.directive_reporting = Some(reporting_profile(0));
    let e = reporting_evaluation(inactive, None);
    assert_eq!(json_bytes(&e.page(&q).unwrap()).unwrap(), bytes);
    assert_eq!(e.export_csv().unwrap(), csv);
    assert_eq!(e.token, baseline.token);
    let mut report = report;
    report
        .pages
        .retain(|p| p.url.ends_with("/plain") || p.url.ends_with("/target"));
    report.config.directive_reporting = Some(reporting_profile(1));
    let raw = reporting_evaluation(
        report.clone(),
        Some(open_seo_spider_core::ReportingMode::Raw),
    );
    let reported =
        reporting_evaluation(report, Some(open_seo_spider_core::ReportingMode::Reported));
    assert_eq!(raw.rows.len(), reported.rows.len());
    assert_eq!(
        raw.snapshot.typed_report_sha256,
        reported.snapshot.typed_report_sha256
    );
    assert_ne!(raw.token, reported.token);
    let wrong = Request {
        filter: Filter::All,
        reporting_mode: Some(open_seo_spider_core::ReportingMode::Reported),
        expected_snapshot_token: Some(raw.token.clone()),
        ..Default::default()
    };
    assert!(reported.page(&wrong).is_err());
    assert!(raw.page(&wrong).is_err());
    assert!(
        serde_json::from_value::<Request>(serde_json::json!({"reporting_mode":"auto"})).is_err()
    );
}
#[test]
fn broken_source_reporting_all_hidden_and_cursor_profile_edit_refuse_stale_without_classification_rewrite()
 {
    let mut report = reporting_report();
    for page in &mut report.pages {
        page.meta_robots = vec!["noindex".into()];
    }
    report.config.directive_reporting = Some(reporting_profile(1));
    let e = reporting_evaluation(report.clone(), None);
    let p = json(
        &e,
        &Request {
            filter: Filter::All,
            ..Default::default()
        },
    );
    assert_eq!(p["rows"], serde_json::json!([]));
    assert_eq!(p["reporting"]["excluded_link_occurrences"], 5);
    assert_eq!(p["reporting"]["raw_summary"]["observed_http_failure"], 5);
    let raw = reporting_evaluation(
        report.clone(),
        Some(open_seo_spider_core::ReportingMode::Raw),
    );
    let first = Request {
        filter: Filter::All,
        reporting_mode: Some(open_seo_spider_core::ReportingMode::Raw),
        limit: 1,
        ..Default::default()
    };
    let first_page = raw.page(&first).unwrap();
    let cursor = first_page.next_cursor.clone().unwrap();
    report.config.directive_reporting = Some(reporting_profile(2));
    let changed = reporting_evaluation(report, Some(open_seo_spider_core::ReportingMode::Raw));
    let stale = Request {
        cursor: Some(cursor),
        expected_snapshot_token: Some(raw.token.clone()),
        ..first
    };
    assert!(changed.page(&stale).is_err());
}

#[test]
fn broken_sources_preserves_repeated_occurrences_dom_absence_and_exact_requested_redirect_identity()
{
    let mut source = page("https://a.test/source", 200);
    let mut a = link("https://a.test/old?x=1", "first, \"ä\"\r\nanchor");
    a.rel = "nofollow sponsored".into();
    a.nofollow = true;
    a.raw_href = Some("/old?x=1#part".into());
    a.resolved_href = Some("https://a.test/old?x=1#part".into());
    a.fragment = Some("part".into());
    a.position = Some("custom footer".into());
    a.dom_location = Some(DomLocation {
        index: 12,
        path: Some("html>body>footer>a:nth-of-type(2)".into()),
        path_truncated: true,
        semantic_ancestors: vec!["footer".into()],
    });
    source.links = vec![
        a.clone(),
        a,
        link("https://a.test/old?x=1", "same URL, no DOM"),
        link("https://a.test/old?x=2", "different query"),
        link(
            "https://a.test/old?x=1#part",
            "fragment is not a stored fetch key",
        ),
    ];
    let mut second = page("https://a.test/second", 200);
    second.links = vec![link("https://a.test/old?x=1", "other page")];
    let mut target = page("https://a.test/old?x=1", 301);
    target.final_url = "https://a.test/final".into();
    target.final_status = Some(404);
    target.redirect_chain = vec![RedirectHop {
        url: target.url.clone(),
        status: 301,
        target: target.final_url.clone(),
    }];
    let e = evaluated(report(vec![source, second, target]), Filter::All);
    let p = json(
        &e,
        &Request {
            filter: Filter::All,
            ..Default::default()
        },
    );
    let rows = p["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 6);
    assert_eq!(p["scope"], SCOPE);
    assert_eq!(p["source_kind_coverage_complete"], false);
    assert_eq!(
        rows.iter()
            .filter(|r| r["verdict"] == "observed_http_failure")
            .count(),
        4
    );
    assert_eq!(rows[0]["link"]["text"], "first, \"ä\"\r\nanchor");
    assert_eq!(rows[0]["link"]["rel"], "nofollow sponsored");
    assert_eq!(rows[0]["link"]["dom_location"]["index"], 12);
    assert_eq!(rows[0]["link"]["dom_location"]["path_truncated"], true);
    assert_eq!(rows[1]["link_ordinal"], 1);
    assert!(rows[2]["link"]["dom_location"].is_null());
    assert_eq!(rows[3]["reason"], "no_requested_url_observation");
    assert_eq!(rows[4]["reason"], "no_requested_url_observation");
    assert_eq!(rows[5]["source"]["page_ordinal"], 1);
    assert_eq!(rows[0]["target_observations"][0]["initial_status"], 301);
    assert_eq!(rows[0]["target_observations"][0]["final_status"], 404);
    assert_eq!(
        rows[0]["target_observations"][0]["redirect_chain"][0]["target"],
        "https://a.test/final"
    );
}
#[test]
fn broken_sources_unknowns_keep_ambiguous_robots_errors_proposed_locations_old_and_incoherent_observations()
 {
    let mut source = page("https://a.test/s", 200);
    let mut pages = vec![];
    for name in [
        "duplicate",
        "robots",
        "error",
        "proposed",
        "old",
        "badchain",
        "healthy",
        "missing",
    ] {
        source
            .links
            .push(link(&format!("https://a.test/{name}"), name));
    }
    let duplicate = page("https://a.test/duplicate", 404);
    pages.push(duplicate.clone());
    let mut conflict = duplicate;
    conflict.final_status = Some(200);
    pages.push(conflict);
    let mut robots = page("https://a.test/robots", 404);
    robots.blocked_by_robots = true;
    pages.push(robots);
    let mut error = page("https://a.test/error", 503);
    error.error = Some("connection body ended".into());
    pages.push(error);
    let mut proposed = page("https://a.test/proposed", 302);
    proposed.final_url = "https://a.test/unfetched".into();
    proposed.final_status = None;
    pages.push(proposed);
    let mut old = page("https://a.test/old", 404);
    old.fetched_at = "2026-10-14T12:00:05Z".into();
    pages.push(old);
    let mut bad = page("https://a.test/badchain", 301);
    bad.final_url = "https://a.test/other".into();
    bad.final_status = Some(404);
    pages.push(bad);
    pages.push(page("https://a.test/healthy", 200));
    pages.insert(0, source);
    let e = evaluated(report(pages), Filter::All);
    let p = json(
        &e,
        &Request {
            filter: Filter::All,
            ..Default::default()
        },
    );
    let reasons = p["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["reason"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        reasons,
        vec![
            "ambiguous_requested_url_observations",
            "target_blocked_by_robots",
            "recorded_target_fetch_error",
            "target_final_response_unobserved",
            "target_age_unknown",
            "inconsistent_stored_response_or_redirect_chain",
            "unique_observed_final_http_2xx",
            "no_requested_url_observation"
        ]
    );
    assert_eq!(p["rows"][0]["observation_count"], 2);
    assert_eq!(
        p["rows"][0]["target_observations"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        p["rows"][4]["target_observations"][0]["age_unknown_reason"],
        "outside_saved_run_time_window"
    );
    assert_eq!(
        p["summary"]["observed_http_failure"],
        serde_json::Value::Null
    );
}
#[test]
fn broken_sources_selected_inherited_and_imported_age_cannot_turn_old_404_into_a_current_observed_failure()
 {
    let mut source = page("https://a.test/s", 200);
    source.links.push(link("https://a.test/t", "target"));
    let mut r = report(vec![source, page("https://a.test/t", 404)]);
    r.operation = Some(SelectedRecrawlOperation {
        engine: "selected".into(),
        coverage: "mixed-age".into(),
        parent: RecrawlParentReference {
            job_id: "parent".into(),
            checkpoint_revision: None,
            source_kind: RecrawlSourceKind::StoredJson,
            snapshot_sha256: "a".repeat(64),
        },
        plan_sha256: "b".repeat(64),
        inherited_config_sha256: "c".repeat(64),
        selection: vec![],
        retained_count: 1,
        refreshed_count: 1,
        pending_count: 0,
        max_attempts: None,
        admitted_this_run: 1,
        committed_this_run: 1,
        targets: vec![RecrawlTargetObservation {
            requested_url: "https://a.test/s".into(),
            parent_ordinal: 0,
            parent_page_sha256: "d".repeat(64),
            state: RecrawlTargetState::Committed,
            admitted_attempts: 1,
            committed_generation: Some(900),
        }],
    });
    let e = evaluated(r.clone(), Filter::All);
    let p = json(
        &e,
        &Request {
            filter: Filter::All,
            ..Default::default()
        },
    );
    assert_eq!(p["rows"][0]["verdict"], "unknown");
    assert_eq!(
        p["rows"][0]["target_observations"][0]["age_unknown_reason"],
        "inherited_selected_observation"
    );
    assert_eq!(
        p["rows"][0]["source"]["selected_observations"][0]["committed_generation"],
        900
    );
    assert!(
        p["rows"][0]["source"]["age_unknown_reason"].is_null(),
        "Core generation900 is not App generation7"
    );
    r.operation = None;
    let mut s = evaluated(r.clone(), Filter::All).snapshot;
    s.caller_binding.origin = Some("app_declared_imported_or_detached_report".into());
    let imported = evaluate(r, s, Filter::All).unwrap();
    let p = json(
        &imported,
        &Request {
            filter: Filter::All,
            ..Default::default()
        },
    );
    assert_eq!(p["rows"][0]["reason"], "source_age_unknown");
    assert_eq!(
        p["rows"][0]["source"]["age_unknown_reason"],
        "imported_or_detached_age_unknown"
    );
}
#[test]
fn broken_sources_csv_exports_every_filtered_occurrence_formula_safe_exact_utf8_and_refuses_overcap()
 {
    let mut source = page("https://a.test/s", 200);
    source.links = vec![
        link("https://a.test/t", " =SUM(1,2)\r\n\"ä\""),
        link("https://a.test/t", "second"),
    ];
    let e = evaluated(
        report(vec![source, page("https://a.test/t", 500)]),
        Filter::Broken,
    );
    let bytes = e.export_csv().unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.contains("\"' =SUM(1,2)\r\n\"\"ä\"\"\""));
    assert!(text.contains("\"second\""));
    assert!(text.contains("\"retained_link_json\""));
    assert!(text.ends_with("\"0\"\r\n"));
    assert_eq!(text.matches("\"observed_http_failure\"").count(), 2);
    let exact = e.export_csv_limit(bytes.len()).unwrap();
    assert_eq!(exact, bytes);
    let failure = e.export_csv_limit(bytes.len() - 1).err().unwrap();
    assert_eq!(failure.code, "csv_budget");
}
#[test]
fn broken_sources_cursor_and_export_require_exact_snapshot_filter_generation_and_page_limit() {
    let mut source = page("https://a.test/s", 200);
    source.links = vec![
        link("https://a.test/t", "one"),
        link("https://a.test/t", "two"),
    ];
    let r = report(vec![source, page("https://a.test/t", 404)]);
    let e = evaluated(r.clone(), Filter::Broken);
    let first = Request {
        filter: Filter::Broken,
        limit: 1,
        ..Default::default()
    };
    let p = e.page(&first).unwrap();
    let mut next = first.clone();
    next.cursor = p.next_cursor.clone();
    next.expected_snapshot_token = Some(e.token.clone());
    assert_eq!(e.page(&next).unwrap().rows[0].link_ordinal, 1);
    next.limit = 2;
    assert_eq!(e.page(&next).err().unwrap().code, "stale_cursor");
    next.limit = 1;
    next.filter = Filter::Unknown;
    assert_eq!(e.page(&next).err().unwrap().code, "stale_snapshot");
    assert_eq!(
        e.check_token(&first, true).err().unwrap().code,
        "snapshot_token_required"
    );
    let mut changed = e.snapshot.clone();
    changed.app_generation += 1;
    let changed = evaluate(r, changed, Filter::Broken).unwrap();
    assert_eq!(
        changed
            .page(&Request {
                filter: Filter::Broken,
                limit: 1,
                cursor: p.next_cursor,
                ..Default::default()
            })
            .err()
            .unwrap()
            .code,
        "stale_cursor"
    );
    let defaults: Request = serde_json::from_str("{}").unwrap();
    assert_eq!(defaults.limit, 100);
    assert_eq!(defaults.filter, Filter::Actionable);
    assert!(serde_json::from_str::<Request>("{\"caller_path\":\"C:/foreign\"}").is_err());
}
#[test]
fn broken_sources_caps_reject_rows_and_ambiguous_selected_join_expansion_without_truncation() {
    let mut source = page("https://a.test/s", 200);
    source.links = vec![link("https://a.test/t", ""); MAX_LINKS + 1];
    let too_many = report(vec![source]);
    assert_eq!(
        preflight(&too_many).err().unwrap().code,
        "link_count_budget"
    );
    let mut source = page("https://a.test/s", 200);
    source.links = vec![link("https://a.test/t", ""); 1001];
    let mut pages = vec![source];
    for _ in 0..500 {
        pages.push(page("https://a.test/t", 404))
    }
    let r = report(pages);
    let s = evaluated(report(vec![]), Filter::All).snapshot;
    assert_eq!(
        evaluate(r, s, Filter::All).err().unwrap().code,
        "join_fanout_budget"
    );
}
