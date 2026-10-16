use super::*;
use crate::{CrawlConfig, CrawlSummary, Severity};
fn report(pages: Vec<PageRecord>) -> CrawlReport {
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
fn p(url: &str, title: Option<&str>, status: Option<u16>) -> PageRecord {
    PageRecord {
        url: url.into(),
        final_url: url.into(),
        status,
        titles: title.map(|s| vec![s.into()]).unwrap_or_default(),
        content_type: "text/html; charset=utf-8".into(),
        ..Default::default()
    }
}
fn d(id: &str, field: Field, op: Operator, text: &str) -> Definition {
    Definition {
        id: id.into(),
        name: id.into(),
        join: Join::All,
        conditions: vec![Condition {
            field,
            operator: op,
            value: text.into(),
            case_insensitive: false,
        }],
    }
}
fn rules(definitions: Vec<Definition>) -> Rules {
    Rules {
        version: 1,
        definitions,
    }
}

#[test]
fn segments_order_overlap_duplicate_rows_and_missing_are_independent() {
    let r = report(vec![
        p("https://x/a", Some("A"), Some(200)),
        p("https://x/a", None, None),
        p("https://x/b", Some(""), Some(404)),
    ]);
    let c = rules(vec![
        d("url", Field::RequestedUrl, Operator::Contains, "/a"),
        d("title", Field::StoredTitle, Operator::Regex, "^A$"),
        d("empty", Field::StoredTitle, Operator::Equals, ""),
    ]);
    let a = project(&r, &c, &Selection::All).unwrap();
    assert_eq!(
        a.pages
            .iter()
            .map(|p| p.matched_indices.clone())
            .collect::<Vec<_>>(),
        vec![vec![0, 1], vec![0], vec![2]]
    );
    assert_eq!(a.pages[1].unknown_indices, vec![1, 2]);
    assert_eq!(
        (a.multiple_matches, a.unassigned_pages, a.unknown_pages),
        (1, 0, 0)
    );
    assert_eq!(
        a.counts
            .iter()
            .map(|c| (c.matched, c.unknown))
            .collect::<Vec<_>>(),
        vec![(2, 0), (1, 1), (1, 1)]
    );
    let mut swapped = c.clone();
    swapped.definitions.swap(0, 1);
    assert_eq!(
        project(&r, &swapped, &Selection::All).unwrap().pages[1].matched_indices,
        vec![1]
    );
}
#[test]
fn segments_kleene_all_any_and_disjoint_unknown_unassigned() {
    let r = report(vec![
        p("a", None, None),
        p("b", Some("x"), Some(200)),
        p("c", Some("y"), Some(404)),
    ]);
    let mut c = rules(vec![d("one", Field::StoredTitle, Operator::Equals, "x")]);
    c.definitions[0].conditions.push(Condition {
        field: Field::Status,
        operator: Operator::Equals,
        value: "404".into(),
        case_insensitive: false,
    });
    let all = project(&r, &c, &Selection::All).unwrap();
    assert_eq!(
        (
            all.counts[0].matched,
            all.counts[0].unknown,
            all.unassigned_pages,
            all.unknown_pages
        ),
        (0, 1, 2, 1)
    );
    c.definitions[0].join = Join::Any;
    assert_eq!(
        project(&r, &c, &Selection::Segment { id: "one".into() })
            .unwrap()
            .selected_pages,
        2
    );
    assert_eq!(
        project(&r, &c, &Selection::Unknown).unwrap().pages[0].url,
        "a"
    );
    assert!(
        project(&r, &c, &Selection::Unassigned)
            .unwrap()
            .pages
            .is_empty()
    );
}
#[test]
fn segments_unicode_literal_regex_and_percent_case_remain_explicit() {
    let r = report(vec![
        p("https://x/Ü/%2f", None, Some(600)),
        p("https://x/ü/%2F", None, Some(999)),
    ]);
    let mut c = rules(vec![
        d("literal", Field::RequestedUrl, Operator::Contains, "Ü/%2f"),
        d(
            "re",
            Field::RequestedUrl,
            Operator::Regex,
            r"/\p{L}/%2[Ff]$",
        ),
    ]);
    let a = project(&r, &c, &Selection::All).unwrap();
    assert_eq!(
        a.counts.iter().map(|v| v.matched).collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        a.charts.status,
        BTreeMap::from([("600".into(), 1), ("999".into(), 1)])
    );
    c.definitions[0].conditions[0].case_insensitive = true;
    assert_eq!(
        project(&r, &c, &Selection::All).unwrap().counts[0].matched,
        2
    );
}
#[test]
fn segments_title_authority_truncated_failed_renderer_and_legacy_unknown() {
    let mut rows = vec![
        p("a", None, None),
        p("b", Some("hit"), Some(200)),
        p("c", Some("hit"), Some(200)),
        p("d", Some("hit"), Some(200)),
        p("e", Some("hit"), Some(200)),
    ];
    rows[1].truncated = true;
    rows[2].renderer_error = Some("failed".into());
    rows[3].rendered = true;
    rows[3].rendered_status = Some(500);
    rows[3].rendered_url = Some("https://other/".into());
    rows[4].rendered = true;
    rows[4].rendered_status = Some(200);
    rows[4].rendered_url = Some("https://other/".into());
    let a = project(
        &report(rows),
        &rules(vec![d(
            "title",
            Field::StoredTitle,
            Operator::Equals,
            "hit",
        )]),
        &Selection::All,
    )
    .unwrap();
    assert_eq!((a.counts[0].matched, a.counts[0].unknown), (1, 4));
    assert_eq!(a.pages[4].document_url, "https://other/");
}
#[test]
fn segments_issue_url_union_charts_and_full_exports_share_one_cohort() {
    let mut r = report(vec![
        p("a", Some("yes"), Some(200)),
        p("a", None, Some(404)),
        p("b", Some("no"), Some(500)),
    ]);
    r.pages[0].depth = 3;
    for u in ["a", "b", "orphan"] {
        r.issues.push(AuditIssue {
            code: "x".into(),
            severity: Severity::Warning,
            url: u.into(),
            message: "m,\"\r\n雪".into(),
            related_urls: vec!["a".into()],
        });
    }
    let a = project(
        &r,
        &rules(vec![d("t", Field::StoredTitle, Operator::Equals, "yes")]),
        &Selection::Segment { id: "t".into() },
    )
    .unwrap();
    assert_eq!((a.pages.len(), a.issues.len(), a.orphan_issues), (1, 1, 1));
    assert_eq!(
        (
            a.counts[0].matched,
            a.counts[0].selected_matched,
            a.counts[0].selected_unknown
        ),
        (1, 1, 0)
    );
    assert_eq!(a.charts.depth, BTreeMap::from([(3, 1)]));
    let csv = csv_bytes(&a, &"fixture").unwrap();
    let rows = csv::Reader::from_reader(csv.as_slice())
        .records()
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        rows.iter().map(|r| &r[0]).collect::<Vec<_>>(),
        vec!["context", "summary", "page", "issue"]
    );
    let issue: AuditIssue = serde_json::from_str(&rows[3][5]).unwrap();
    assert_eq!(issue.message, r.issues[0].message);
    let json: serde_json::Value = serde_json::from_slice(&json_bytes(&a).unwrap()).unwrap();
    assert_eq!(json["pages"][0]["url"], "a");
    assert_eq!(json["issues"][0]["url"], "a");
}
#[test]
fn segments_rules_regex_work_and_row_ceilings_refuse_without_partial_success() {
    let r = report(vec![p("x", None, None)]);
    for text in ["(", "a{1000000}"] {
        assert!(
            project(
                &r,
                &rules(vec![d("a", Field::RequestedUrl, Operator::Regex, text)]),
                &Selection::All
            )
            .is_err()
        );
    }
    let duplicate = d("a", Field::RequestedUrl, Operator::Equals, "x");
    assert!(validate_rules(&rules(vec![duplicate.clone(), duplicate])).is_err());
    assert!(
        validate_rules(&Rules {
            version: 2,
            definitions: vec![]
        })
        .is_err()
    );
    assert!(
        validate_rules(&rules(
            (0..33)
                .map(|i| d(&i.to_string(), Field::RequestedUrl, Operator::Equals, "x"))
                .collect()
        ))
        .is_err()
    );
    assert!(
        project(
            &r,
            &Rules::default(),
            &Selection::Segment {
                id: "absent".into()
            }
        )
        .is_err()
    );
    let large = report((0..600).map(|_| p(&"a".repeat(4096), None, None)).collect());
    let c = rules(
        (0..32)
            .map(|i| d(&i.to_string(), Field::RequestedUrl, Operator::Contains, "a"))
            .collect(),
    );
    assert!(project(&large, &c, &Selection::All).is_err());
    let too_many = report(vec![PageRecord::default(); MAX_PAGES + 1]);
    assert!(project(&too_many, &Rules::default(), &Selection::All).is_err());
}
#[test]
fn segments_empty_rules_legacy_config_and_formula_safe_csv() {
    let r = report(vec![p("\t=HYPERLINK(\"evil\")", None, None)]);
    let before = serde_json::to_vec(&r).unwrap();
    let a = project(&r, &Rules::default(), &Selection::All).unwrap();
    assert_eq!((a.selected_pages, a.unassigned_pages), (1, 1));
    let csv = csv_bytes(&a, &"fixture").unwrap();
    let rows = csv::Reader::from_reader(csv.as_slice())
        .records()
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();
    assert_eq!(&rows[2][2], "'\t=HYPERLINK(\"evil\")");
    assert_eq!(before, serde_json::to_vec(&r).unwrap());
}
#[test]
fn segments_export_byte_boundary_and_utf8_rule_ceiling_are_exact() {
    let exact = "x".repeat(OUTPUT_BYTES - 2); // JSON quote pair occupies the remaining two bytes.
    assert_eq!(json_bytes(&exact).unwrap().len(), OUTPUT_BYTES);
    let over = format!("{exact}x");
    assert!(json_bytes(&over).is_err());
    let exact_pattern = "é".repeat(1024);
    assert!(
        validate_rules(&rules(vec![d(
            "a",
            Field::RequestedUrl,
            Operator::Contains,
            &exact_pattern
        )]))
        .is_ok()
    );
    assert!(
        validate_rules(&rules(vec![d(
            "a",
            Field::RequestedUrl,
            Operator::Contains,
            &format!("{exact_pattern}x")
        )]))
        .is_err()
    );
}

#[test]
fn segments_omitted_depth_is_not_a_supported_predicate_and_chart_default_is_declared() {
    let mut omitted =
        serde_json::to_value(report(vec![p("https://x/legacy", None, None)])).unwrap();
    omitted["pages"][0].as_object_mut().unwrap().remove("depth");
    let mut explicit_zero = omitted.clone();
    explicit_zero["pages"][0]
        .as_object_mut()
        .unwrap()
        .insert("depth".into(), serde_json::json!(0));
    let legacy: CrawlReport = serde_json::from_value(omitted).unwrap();
    let explicit: CrawlReport = serde_json::from_value(explicit_zero).unwrap();
    assert_eq!((legacy.pages[0].depth, explicit.pages[0].depth), (0, 0));
    let depth_request = serde_json::json!({"version":1,"definitions":[{"id":"depth0","name":"Depth0","conditions":[{"field":"depth","operator":"equals","value":"0"}]}]});
    assert!(serde_json::from_value::<Rules>(depth_request).is_err());
    for saved in [&legacy, &explicit] {
        let evaluated = project(saved, &Rules::default(), &Selection::All).unwrap();
        assert!(evaluated.counts.is_empty());
        assert_eq!(evaluated.charts.depth, BTreeMap::from([(0, 1)]));
        assert_eq!(evaluated.pages[0].depth, 0);
        assert_eq!(evaluated.depth_policy, DEPTH_POLICY);
        let json: serde_json::Value =
            serde_json::from_slice(&json_bytes(&evaluated).unwrap()).unwrap();
        assert_eq!(json["depth_policy"], DEPTH_POLICY);
        let csv = csv_bytes(&evaluated, &"declared legacy/import fixture").unwrap();
        let rows = csv::Reader::from_reader(csv.as_slice())
            .records()
            .map(|r| r.unwrap())
            .collect::<Vec<_>>();
        let summary: serde_json::Value = serde_json::from_str(&rows[1][5]).unwrap();
        assert_eq!(summary["depth_policy"], DEPTH_POLICY);
    }
}

fn profile(mask: u8) -> crate::DirectiveReportingConfig {
    crate::DirectiveReportingConfig {
        version: 1,
        respect_noindex: mask & 1 != 0,
        respect_canonical: mask & 2 != 0,
        respect_next_prev: mask & 4 != 0,
    }
}
fn reporting_report() -> CrawlReport {
    let mut r = report(vec![
        p("https://x/plain", Some("yes"), Some(200)),
        p("https://x/noindex", Some("yes"), Some(404)),
        p("https://x/canonical", Some("yes"), Some(200)),
        p("https://x/prev", Some("yes"), Some(200)),
        p("https://x/next", Some("yes"), Some(200)),
    ]);
    r.pages[1].meta_robots = vec!["noindex".into()];
    r.pages[2].canonical = vec!["https://x/target-only".into()];
    r.pages[3].pagination_prev = vec!["https://x/target-only".into()];
    r.pages[4].pagination_next = vec!["https://x/target-only".into()];
    r.pages[4].canonical = vec![r.pages[4].url.clone()];
    r.pages[4].links.push(crate::LinkRecord {
        url: "https://x/target-only".into(),
        ..Default::default()
    });
    r
}
#[test]
fn segments_reporting_eight_profiles_filter_before_rule_counts_charts_and_selection() {
    let mut r = reporting_report();
    let rules = rules(vec![
        d("all", Field::RequestedUrl, Operator::Contains, "https://x/"),
        d("yes", Field::StoredTitle, Operator::Equals, "yes"),
    ]);
    for mask in 0u8..8 {
        r.config.directive_reporting = Some(profile(mask));
        let before = serde_json::to_vec(&r).unwrap();
        let projected = project_with_reporting(&r, &rules, &Selection::All, None).unwrap();
        let expected = 5 - mask.count_ones() as usize;
        assert_eq!(
            (
                projected.total_pages,
                projected.selected_pages,
                projected.counts[0].matched,
                projected.counts[1].matched,
                projected.multiple_matches
            ),
            (expected, expected, expected, expected, expected)
        );
        assert_eq!(projected.charts.status.values().sum::<usize>(), expected);
        assert!(
            !projected
                .pages
                .iter()
                .any(|p| p.url == "https://x/target-only")
        );
        assert_eq!(serde_json::to_vec(&r).unwrap(), before);
        if mask == 0 {
            assert!(projected.reporting.is_none());
        } else {
            let metadata = projected.reporting.unwrap();
            assert_eq!(metadata.captured_page_observations, 5);
            assert_eq!(metadata.profile_visible_page_observations, expected);
            assert_eq!(
                metadata.profile_excluded_page_observations,
                mask.count_ones() as usize
            );
            assert_eq!(metadata.eligible_page_observations, expected);
        }
    }
}
#[test]
fn segments_reporting_mixed_duplicates_use_selected_visible_owner_and_keep_raw_ordinals() {
    let mut r = report(vec![
        p("https://x/mixed", Some("hidden"), Some(404)),
        p("https://x/mixed", Some("visible"), Some(200)),
        p("https://x/only-hidden", Some("hidden"), Some(200)),
        p("https://x/visible", Some("visible"), Some(200)),
    ]);
    for index in [0, 2] {
        r.pages[index].meta_robots = vec!["noindex".into()];
    }
    r.config.directive_reporting = Some(profile(1));
    for url in [
        "https://x/mixed",
        "https://x/only-hidden",
        "https://x/visible",
        "https://x/orphan",
    ] {
        r.issues.push(AuditIssue {
            code: "x".into(),
            severity: Severity::Warning,
            url: url.into(),
            message: "original".into(),
            related_urls: vec!["https://x/only-hidden".into()],
        });
    }
    let before = serde_json::to_vec(&r).unwrap();
    let all = project_with_reporting(&r, &Rules::default(), &Selection::All, None).unwrap();
    assert_eq!(
        all.pages.iter().map(|p| p.page_ordinal).collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert_eq!(
        all.issues
            .iter()
            .map(|i| i.url.as_str())
            .collect::<Vec<_>>(),
        vec!["https://x/mixed", "https://x/visible"]
    );
    assert_eq!(all.issues[0].related_urls, vec!["https://x/only-hidden"]);
    assert_eq!(
        all.orphan_issues, 1,
        "A known hidden URL never becomes an orphan"
    );
    let meta = all.reporting.unwrap();
    assert_eq!(
        (
            meta.captured_issues,
            meta.profile_visible_issues,
            meta.profile_excluded_issues,
            meta.selected_issues,
            meta.captured_orphan_issues
        ),
        (4, 3, 1, 2, 1)
    );
    let selected = project_with_reporting(
        &r,
        &rules(vec![d(
            "hidden",
            Field::StoredTitle,
            Operator::Equals,
            "hidden",
        )]),
        &Selection::Segment {
            id: "hidden".into(),
        },
        None,
    )
    .unwrap();
    assert!(
        selected.pages.is_empty() && selected.issues.is_empty(),
        "Hidden matching duplicate cannot select the visible duplicate's issue"
    );
    assert_eq!(selected.counts[0].matched, 0);
    assert_eq!(serde_json::to_vec(&r).unwrap(), before);
}
#[test]
fn segments_reporting_unknown_unassigned_and_overlap_cohorts_only_use_visible_rows() {
    let mut r = report(vec![
        p("hidden", None, None),
        p("unknown", None, None),
        p("unassigned", Some("no"), Some(404)),
        p("matched", Some("yes"), Some(200)),
    ]);
    r.pages[0].meta_robots = vec!["noindex".into()];
    r.config.directive_reporting = Some(profile(1));
    let rules = rules(vec![
        d("one", Field::StoredTitle, Operator::Equals, "yes"),
        d("two", Field::Status, Operator::Equals, "200"),
    ]);
    let unknown = project_with_reporting(&r, &rules, &Selection::Unknown, None).unwrap();
    assert_eq!(
        unknown
            .pages
            .iter()
            .map(|p| p.page_ordinal)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(
        (
            unknown.total_pages,
            unknown.unknown_pages,
            unknown.unassigned_pages,
            unknown.multiple_matches
        ),
        (3, 1, 1, 1)
    );
    let unassigned = project_with_reporting(&r, &rules, &Selection::Unassigned, None).unwrap();
    assert_eq!(unassigned.pages[0].page_ordinal, 2);
    let matched =
        project_with_reporting(&r, &rules, &Selection::Segment { id: "one".into() }, None).unwrap();
    assert_eq!(matched.pages[0].page_ordinal, 3);
    assert_eq!(
        matched
            .counts
            .iter()
            .map(|c| c.selected_matched)
            .collect::<Vec<_>>(),
        vec![1, 1]
    );
    assert_eq!(matched.charts.status, BTreeMap::from([("200".into(), 1)]));
}
#[test]
fn segments_reporting_explicit_raw_keeps_full_projection_but_distinct_profile_identity() {
    let mut r = reporting_report();
    r.config.directive_reporting = Some(profile(7));
    let raw = project_with_reporting(
        &r,
        &Rules::default(),
        &Selection::All,
        Some(ReportingMode::Raw),
    )
    .unwrap();
    assert_eq!(raw.pages.len(), 5);
    let metadata = raw.reporting.as_ref().unwrap();
    assert_eq!(metadata.identity.mode, ReportingMode::Raw);
    assert_eq!(
        (
            metadata.captured_page_observations,
            metadata.profile_visible_page_observations,
            metadata.profile_excluded_page_observations,
            metadata.eligible_page_observations,
            metadata.selected_page_observations
        ),
        (5, 2, 3, 5, 5)
    );
    let reported = project_with_reporting(&r, &Rules::default(), &Selection::All, None).unwrap();
    assert_ne!(
        metadata.identity_sha256,
        reported.reporting.unwrap().identity_sha256
    );
    let mut actual = serde_json::to_value(raw).unwrap();
    actual.as_object_mut().unwrap().remove("reporting");
    assert_eq!(
        actual,
        serde_json::to_value(project(&r, &Rules::default(), &Selection::All).unwrap()).unwrap()
    );
}
#[test]
fn segments_reporting_legacy_none_null_false_keep_fixed_empty_csv_contract_and_wire() {
    let original = serde_json::to_value(CrawlConfig::default()).unwrap();
    let legacy_summary = r#"{"charts":{"depth":{},"status":{}},"counts":[],"depth_policy":"declared_saved_depth_including_possible_legacy_or_import_default_zero_not_a_segment_predicate","engine":"saved_segments_v1","multiple_matches":0,"orphan_issues":0,"rules":{"definitions":[],"version":1},"selected_pages":0,"selection":{"kind":"all"},"total_pages":0,"unassigned_pages":0,"unknown_pages":0}"#;
    let golden = format!(
        "row_type,page_ordinal,url,segment_indices_json,unknown_indices_json,data_json\r\ncontext,,,,,\"\"\"fixture\"\"\"\r\nsummary,,,,,\"{}\"\r\n",
        legacy_summary.replace('"', "\"\"")
    );
    for wire in [
        original.clone(),
        {
            let mut v = original.clone();
            v["directive_reporting"] = serde_json::Value::Null;
            v
        },
        {
            let mut v = original.clone();
            v["directive_reporting"] = serde_json::to_value(profile(0)).unwrap();
            v
        },
    ] {
        let mut r = report(vec![]);
        r.config = serde_json::from_value(wire).unwrap();
        assert_eq!(serde_json::to_value(&r.config).unwrap(), original);
        let projected =
            project_with_reporting(&r, &Rules::default(), &Selection::All, None).unwrap();
        assert!(projected.reporting.is_none());
        assert!(
            serde_json::to_value(&projected)
                .unwrap()
                .get("reporting")
                .is_none()
        );
        assert_eq!(
            csv_bytes(&projected, &"fixture").unwrap(),
            golden.as_bytes()
        );
    }
}
#[test]
fn segments_reporting_csv_includes_exact_reporting_row_and_selected_original_issue_facts() {
    let mut r = reporting_report();
    r.config.directive_reporting = Some(profile(7));
    r.issues.push(AuditIssue {
        code: "x".into(),
        severity: Severity::Warning,
        url: r.pages[0].url.clone(),
        message: "m,\"\r\n雪".into(),
        related_urls: vec![r.pages[1].url.clone()],
    });
    let projected = project_with_reporting(&r, &Rules::default(), &Selection::All, None).unwrap();
    let bytes = csv_bytes(&projected, &"fixture").unwrap();
    let rows = csv::Reader::from_reader(bytes.as_slice())
        .records()
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        rows.iter().map(|r| &r[0]).collect::<Vec<_>>(),
        vec!["context", "summary", "reporting", "page", "page", "issue"]
    );
    let metadata: serde_json::Value = serde_json::from_str(&rows[2][5]).unwrap();
    assert_eq!(
        metadata,
        serde_json::to_value(projected.reporting.as_ref().unwrap()).unwrap()
    );
    assert_eq!(metadata["captured_page_observations"], 5);
    assert_eq!(metadata["eligible_page_observations"], 2);
    let issue: AuditIssue = serde_json::from_str(&rows[5][5]).unwrap();
    assert_eq!(
        serde_json::to_value(issue).unwrap(),
        serde_json::to_value(&r.issues[0]).unwrap()
    );
}
#[test]
fn segments_reporting_hidden_population_still_respects_raw_row_budget_and_profile_guards() {
    let mut r = report(vec![PageRecord::default(); MAX_PAGES + 1]);
    r.config.directive_reporting = Some(profile(1));
    for p in &mut r.pages {
        p.meta_robots = vec!["noindex".into()];
    }
    assert!(project_with_reporting(&r, &Rules::default(), &Selection::All, None).is_err());
    let mut invalid = reporting_report();
    invalid.config.directive_reporting = Some(crate::DirectiveReportingConfig {
        version: 2,
        ..profile(0)
    });
    assert!(project_with_reporting(&invalid, &Rules::default(), &Selection::All, None).is_err());
}
