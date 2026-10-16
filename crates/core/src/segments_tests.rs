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
