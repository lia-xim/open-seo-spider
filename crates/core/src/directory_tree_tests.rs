use super::*;
fn page(url: &str, status: Option<u16>) -> PageRecord {
    PageRecord {
        url: url.into(),
        final_url: url.into(),
        status,
        final_status: status,
        fetched_at: "2026-10-15T00:00:00Z".into(),
        internal: true,
        indexable: true,
        ..Default::default()
    }
}
fn tree(pages: &[PageRecord]) -> DirectoryTree {
    build_directory_tree(pages, &Default::default()).unwrap()
}
fn node<'a>(t: &'a DirectoryTree, kind: DirectoryNodeKind, address: &str) -> &'a DirectoryNode {
    t.nodes
        .iter()
        .find(|n| n.kind == kind && n.address == address)
        .unwrap()
}
fn root(t: &DirectoryTree) -> &DirectoryNode {
    t.nodes.iter().find(|n| n.id == t.root_id).unwrap()
}

#[test]
fn directory_tree_synthetic_paths_never_become_fetched_url_observations() {
    let t = tree(&[page("https://x.test/a/b", Some(200))]);
    let group = node(&t, DirectoryNodeKind::Directory, "https://x.test/a/");
    assert!(group.synthetic_group);
    assert_eq!(group.counts.observations, 1);
    assert!(
        !t.nodes
            .iter()
            .any(|n| n.kind == DirectoryNodeKind::ObservedUrl && n.address == "https://x.test/a/")
    );
    let leaf = node(&t, DirectoryNodeKind::ObservedUrl, "https://x.test/a/b");
    assert!(!leaf.synthetic_group);
    assert_eq!(leaf.parent_id.as_deref(), Some(group.id.as_str()));
    assert_eq!(root(&t).counts.unique_urls, 1);
    assert_eq!(t.website_requests, 0);
}
#[test]
fn directory_tree_protocol_host_effective_port_ipv6_and_root_are_separate() {
    let t = tree(&[
        page("HTTPS://EXAMPLE.test:443/", Some(200)),
        page("http://example.test/", Some(200)),
        page("https://example.test:8443/", Some(404)),
        page("https://[::1]:8443/", Some(200)),
    ]);
    assert_eq!(
        node(&t, DirectoryNodeKind::Origin, "https://example.test")
            .counts
            .unique_urls,
        1
    );
    assert_eq!(
        node(&t, DirectoryNodeKind::Origin, "http://example.test")
            .counts
            .unique_urls,
        1
    );
    assert_eq!(
        node(&t, DirectoryNodeKind::Origin, "https://example.test:8443")
            .counts
            .status_4xx,
        1
    );
    assert_eq!(
        node(&t, DirectoryNodeKind::Origin, "https://[::1]:8443")
            .counts
            .unique_urls,
        1
    );
    assert_eq!(
        t.nodes
            .iter()
            .filter(|n| n.kind == DirectoryNodeKind::ObservedUrl)
            .count(),
        4
    );
    assert!(
        !t.nodes
            .iter()
            .any(|n| n.kind == DirectoryNodeKind::Directory)
    );
}
#[test]
fn directory_tree_encoded_slash_query_order_empty_query_and_fragment_identity() {
    let t = tree(&[
        page("https://x.test/a%2Fb/c?x=1&y=2#one", Some(200)),
        page("https://x.test/a%2Fb/c?x=1&y=2#two", Some(200)),
        page("https://x.test/a%2Fb/c?y=2&x=1", Some(200)),
        page("https://x.test/a/b/c", Some(200)),
        page("https://x.test/", Some(200)),
        page("https://x.test/?", Some(200)),
        page("https://x.test/%E6%9D%B1%E4%BA%AC/p", Some(200)),
    ]);
    assert_eq!(
        node(&t, DirectoryNodeKind::Directory, "https://x.test/a%2Fb/")
            .counts
            .unique_urls,
        2
    );
    assert_eq!(
        node(&t, DirectoryNodeKind::Directory, "https://x.test/a/")
            .counts
            .unique_urls,
        1
    );
    assert_eq!(
        node(
            &t,
            DirectoryNodeKind::ObservedUrl,
            "https://x.test/a%2Fb/c?x=1&y=2"
        )
        .counts
        .observations,
        2
    );
    assert!(
        t.nodes
            .iter()
            .any(|n| n.address == "https://x.test/?" && n.kind == DirectoryNodeKind::ObservedUrl)
    );
    assert!(
        t.nodes
            .iter()
            .any(|n| n.address == "https://x.test/%E6%9D%B1%E4%BA%AC/"
                && n.kind == DirectoryNodeKind::Directory)
    );
    assert_eq!(root(&t).counts.unique_urls, 6);
    assert_eq!(root(&t).counts.duplicate_observations, 1);
}
#[test]
fn directory_tree_trailing_slash_empty_segments_and_query_directory_leaf_stay_distinct() {
    let t = tree(&[
        page("https://x.test/a", Some(200)),
        page("https://x.test/a/", Some(200)),
        page("https://x.test/a/?q=1", Some(200)),
        page("https://x.test/a//b", Some(200)),
    ]);
    let group = node(&t, DirectoryNodeKind::Directory, "https://x.test/a/");
    assert_eq!(group.counts.unique_urls, 3);
    assert_ne!(
        group.id,
        node(&t, DirectoryNodeKind::ObservedUrl, "https://x.test/a/").id
    );
    assert_eq!(
        node(&t, DirectoryNodeKind::Directory, "https://x.test/a//").label,
        "(leeres Pfadsegment)"
    );
    assert_eq!(
        node(&t, DirectoryNodeKind::ObservedUrl, "https://x.test/a/?q=1")
            .parent_id
            .as_ref(),
        Some(&group.id)
    );
}
#[test]
fn directory_tree_duplicates_conflicting_observations_are_counted_without_latest_selection() {
    let a = page("https://x.test/a", Some(200));
    let mut b = page("https://x.test/a", Some(404));
    b.indexable = false;
    let first = tree(&[a.clone(), b.clone(), a.clone()]);
    let second = tree(&[a.clone(), a, b]);
    assert_eq!(first, second);
    let c = &root(&first).counts;
    assert_eq!(
        (
            c.observations,
            c.unique_urls,
            c.duplicate_observations,
            c.conflicting_urls
        ),
        (3, 1, 2, 1)
    );
    assert_eq!((c.status_2xx, c.status_4xx), (2, 1));
}
#[test]
fn directory_tree_initial_status_and_unknown_final_indexability_never_use_landing_200() {
    let mut redirect = page("https://x.test/old", Some(301));
    redirect.final_url = "https://x.test/new".into();
    redirect.final_status = Some(200);
    let mut missing = page("https://x.test/not-fetched", None);
    missing.blocked_by_robots = true;
    let mut error = page("https://x.test/failed", Some(200));
    error.error = Some("Body incomplete".into());
    let t = tree(&[redirect, missing, error]);
    let c = &root(&t).counts;
    assert_eq!((c.status_3xx, c.status_2xx, c.status_unobserved), (1, 1, 1));
    assert_eq!(c.indexability_unobserved, 3);
    assert_eq!(c.stored_indexable, 0);
    assert_eq!(c.stored_non_indexable, 0);
    assert_eq!((c.errors, c.robots_blocked), (1, 1));
    assert!(!t.nodes.iter().any(|n| n.address == "https://x.test/new"));
}
#[test]
fn directory_tree_invalid_unsupported_and_credentials_are_explicit_exclusions() {
    let t = tree(&[
        page("not a URL", None),
        page("mailto:me@x.test", None),
        page("https://user:secret@x.test/a", Some(200)),
    ]);
    assert_eq!(
        (
            t.input_observations,
            t.invalid_url_observations,
            t.unsupported_url_observations,
            t.credential_url_observations
        ),
        (3, 1, 1, 1)
    );
    assert_eq!(root(&t).counts.observations, 0);
    assert_eq!(t.nodes.len(), 1);
    assert!(
        !String::from_utf8(directory_tree_json(&t, DIRECTORY_TREE_JSON_BYTES).unwrap())
            .unwrap()
            .contains("secret")
    );
}
#[test]
fn directory_tree_real_node_text_url_segment_and_page_boundaries_refuse_without_partial_tree() {
    let pages = [page("https://x.test/a/b", Some(200))];
    let exact = tree(&pages);
    let mut limits = DirectoryTreeLimits {
        max_nodes: exact.nodes.len(),
        max_retained_text_bytes: exact.retained_text_bytes,
        ..Default::default()
    };
    assert_eq!(
        build_directory_tree(&pages, &limits).unwrap().nodes,
        exact.nodes
    );
    limits.max_nodes -= 1;
    assert!(build_directory_tree(&pages, &limits).is_err());
    limits.max_nodes = exact.nodes.len();
    limits.max_retained_text_bytes -= 1;
    assert!(build_directory_tree(&pages, &limits).is_err());
    limits = DirectoryTreeLimits {
        max_url_bytes: pages[0].url.len() - 1,
        ..Default::default()
    };
    assert!(build_directory_tree(&pages, &limits).is_err());
    limits = DirectoryTreeLimits {
        max_path_segments: 1,
        ..Default::default()
    };
    assert!(build_directory_tree(&pages, &limits).is_err());
    limits = DirectoryTreeLimits {
        max_pages: 1,
        ..Default::default()
    };
    assert!(build_directory_tree(&[pages[0].clone(), pages[0].clone()], &limits).is_err());
}
#[test]
fn directory_tree_csv_is_complete_formula_safe_and_exact_byte_caps_are_independent() {
    let t = tree(&[page("https://x.test/=SUM(1,2)", Some(200))]);
    let bytes = directory_tree_csv(&t, DIRECTORY_TREE_CSV_BYTES).unwrap();
    let mut reader = csv::Reader::from_reader(bytes.as_slice());
    let rows: Vec<_> = reader.records().map(|r| r.unwrap()).collect();
    assert_eq!(rows.len(), t.nodes.len());
    assert!(rows.iter().any(|r| r.get(4) == Some("'=SUM(1,2)")));
    assert_eq!(directory_tree_csv(&t, bytes.len()).unwrap(), bytes);
    assert!(directory_tree_csv(&t, bytes.len() - 1).is_err());
    let json = directory_tree_json(&t, DIRECTORY_TREE_JSON_BYTES).unwrap();
    assert_eq!(directory_tree_json(&t, json.len()).unwrap(), json);
    assert!(directory_tree_json(&t, json.len() - 1).is_err());
}

#[test]
fn directory_tree_rendered_document_identity_and_incomplete_observations_stay_unknown() {
    let direct = page("https://x.test/direct", Some(200));
    let mut nonindexable = page("https://x.test/noindex", Some(200));
    nonindexable.indexable = false;
    let mut rendered = page("https://x.test/rendered", Some(200));
    rendered.rendered = true;
    rendered.rendered_status = Some(200);
    rendered.rendered_url = Some("https://x.test/rendered#same-document".into());
    let mut jump = page("https://x.test/jump", Some(200));
    jump.rendered = true;
    jump.rendered_status = Some(200);
    jump.rendered_url = Some("https://x.test/other-dom".into());
    let mut renderer_error = page("https://x.test/render-error", Some(200));
    renderer_error.rendered = true;
    renderer_error.renderer_error = Some("Navigation failed".into());
    let mut truncated = page("https://x.test/truncated", Some(200));
    truncated.truncated = true;
    let mut unobserved_status = page("https://x.test/render-status-unknown", Some(200));
    unobserved_status.rendered = true;
    unobserved_status.rendered_url = Some(unobserved_status.url.clone());
    let mut failed_status = page("https://x.test/render-status-404", Some(200));
    failed_status.rendered = true;
    failed_status.rendered_url = Some(failed_status.url.clone());
    failed_status.rendered_status = Some(404);
    let mut invalid_document = page("https://x.test/render-url-invalid", Some(200));
    invalid_document.rendered = true;
    invalid_document.rendered_url = Some("not a URL".into());
    invalid_document.rendered_status = Some(200);
    let pages = [
        direct,
        nonindexable,
        rendered,
        jump,
        renderer_error,
        truncated,
        unobserved_status,
        failed_status,
        invalid_document,
    ];
    let t = tree(&pages);
    let c = &root(&t).counts;
    assert_eq!(
        (
            c.observations,
            c.status_2xx,
            c.stored_indexable,
            c.stored_non_indexable,
            c.indexability_unobserved
        ),
        (9, 9, 2, 1, 6)
    );
    for p in &pages[3..] {
        let c = &node(&t, DirectoryNodeKind::ObservedUrl, &p.url).counts;
        assert_eq!(
            (
                c.stored_indexable,
                c.stored_non_indexable,
                c.indexability_unobserved
            ),
            (0, 0, 1)
        );
    }
    assert!(
        !t.nodes
            .iter()
            .any(|n| n.address == "https://x.test/other-dom")
    );
    let mut oversized = page("https://x.test/short", Some(200));
    oversized.rendered = true;
    oversized.rendered_status = Some(200);
    oversized.rendered_url = Some(format!("https://x.test/{}", "a".repeat(65)));
    assert!(
        build_directory_tree(
            &[oversized],
            &DirectoryTreeLimits {
                max_url_bytes: 64,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn directory_tree_saved_nonstandard_http_codes_are_known_other_not_unobserved() {
    let statuses = [
        Some(100),
        Some(199),
        Some(200),
        Some(301),
        Some(404),
        Some(503),
        Some(600),
        Some(999),
        None,
        Some(0),
        Some(99),
        Some(1000),
    ];
    let pages: Vec<_> = statuses
        .into_iter()
        .enumerate()
        .map(|(i, status)| page(&format!("https://x.test/status/{i}"), status))
        .collect();
    let t = tree(&pages);
    let c = &root(&t).counts;
    assert_eq!(
        (
            c.observations,
            c.status_2xx,
            c.status_3xx,
            c.status_4xx,
            c.status_5xx,
            c.status_other,
            c.status_unobserved
        ),
        (12, 1, 1, 1, 1, 4, 4)
    );
    for p in &pages[6..8] {
        let c = &node(&t, DirectoryNodeKind::ObservedUrl, &p.url).counts;
        assert_eq!((c.status_other, c.status_unobserved), (1, 0));
    }
    for p in &pages[8..] {
        let c = &node(&t, DirectoryNodeKind::ObservedUrl, &p.url).counts;
        assert_eq!((c.status_other, c.status_unobserved), (0, 1));
    }
}

fn reporting_config(mask: u8) -> CrawlConfig {
    CrawlConfig {
        directive_reporting: Some(DirectiveReportingConfig {
            version: 1,
            respect_noindex: mask & 1 != 0,
            respect_canonical: mask & 2 != 0,
            respect_next_prev: mask & 4 != 0,
        }),
        ..Default::default()
    }
}
fn reporting_pages() -> Vec<PageRecord> {
    let plain = page("https://x.test/a/plain", Some(200));
    let mut noindex = page("https://x.test/hidden/noindex", Some(200));
    noindex.meta_robots = vec!["noindex".into()];
    let mut canonical = page("https://x.test/hidden/canonical", Some(200));
    canonical.canonical = vec![plain.url.clone()];
    let mut prev = page("https://x.test/hidden/prev", Some(200));
    prev.pagination_prev = vec![plain.url.clone()];
    let mut self_canonical_next = page("https://x.test/a/next", Some(200));
    self_canonical_next.canonical = vec![self_canonical_next.url.clone()];
    self_canonical_next.pagination_next = vec!["https://x.test/target-only".into()];
    self_canonical_next.links = vec![crate::LinkRecord {
        url: "https://x.test/target-only".into(),
        ..Default::default()
    }];
    vec![plain, noindex, canonical, prev, self_canonical_next]
}
#[test]
fn directory_reporting_eight_masks_filter_individual_observations_before_aggregation() {
    let pages = reporting_pages();
    let captured = serde_json::to_vec(&pages).unwrap();
    for mask in 0u8..8 {
        let view =
            build_directory_tree_view(&reporting_config(mask), &pages, None, &Default::default())
                .unwrap();
        let expected = 5 - mask.count_ones() as usize;
        assert_eq!(root(&view.tree).counts.observations, expected);
        assert_eq!(root(&view.tree).counts.unique_urls, expected);
        assert_eq!(view.tree.input_observations, expected);
        for (index, bit) in [(1, 1), (2, 2), (3, 4)] {
            assert_eq!(
                view.tree
                    .nodes
                    .iter()
                    .any(|n| n.kind == DirectoryNodeKind::ObservedUrl
                        && n.address == pages[index].url),
                mask & bit == 0
            );
        }
        assert!(
            !view
                .tree
                .nodes
                .iter()
                .any(|n| n.address == "https://x.test/target-only")
        );
        if mask != 0 {
            let counts = view.reporting.as_ref().unwrap();
            assert_eq!(counts.identity.mode, DirectoryTreeMode::Reported);
            assert_eq!(counts.captured_page_observations, 5);
            assert_eq!(counts.profile_visible_page_observations, expected);
            assert_eq!(
                counts.profile_excluded_page_observations,
                mask.count_ones() as usize
            );
        } else {
            assert!(view.reporting.is_none());
        }
        assert_eq!(serde_json::to_vec(&pages).unwrap(), captured);
    }
}
#[test]
fn directory_reporting_visible_duplicate_keeps_its_url_without_hidden_status_counts() {
    let mut hidden = page("https://x.test/a/item", Some(404));
    hidden.meta_robots = vec!["noindex".into()];
    let visible = page("https://x.test/a/item", Some(200));
    let pages = vec![hidden, visible.clone(), visible];
    let view =
        build_directory_tree_view(&reporting_config(1), &pages, None, &Default::default()).unwrap();
    let counts = &node(
        &view.tree,
        DirectoryNodeKind::Directory,
        "https://x.test/a/",
    )
    .counts;
    assert_eq!(
        (
            counts.observations,
            counts.unique_urls,
            counts.duplicate_observations,
            counts.conflicting_urls
        ),
        (2, 1, 1, 0)
    );
    assert_eq!((counts.status_2xx, counts.status_4xx), (2, 0));
    assert_eq!(root(&view.tree).counts, *counts);
    assert_eq!(
        view.reporting.unwrap().excluded_from_view_page_observations,
        1
    );
}
#[test]
fn directory_reporting_raw_mode_retains_all_observations_and_qualifies_profile_denominator() {
    let pages = reporting_pages();
    let config = reporting_config(7);
    let raw = build_directory_tree_view(
        &config,
        &pages,
        Some(DirectoryTreeMode::Raw),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(raw.tree, tree(&pages));
    let counts = raw.reporting.unwrap();
    assert_eq!(counts.identity.mode, DirectoryTreeMode::Raw);
    assert_eq!(
        (
            counts.captured_page_observations,
            counts.profile_visible_page_observations,
            counts.profile_excluded_page_observations
        ),
        (5, 2, 3)
    );
    assert_eq!(
        (
            counts.selected_page_observations,
            counts.excluded_from_view_page_observations
        ),
        (5, 0)
    );
    assert_eq!(counts.hierarchy_page_observations, 5);
}
#[test]
fn directory_reporting_invalid_selected_urls_and_all_hidden_rows_have_truthful_root_counts() {
    let mut pages = reporting_pages();
    pages.extend([
        page("invalid", None),
        page("mailto:a@x.test", None),
        page("https://user:secret@x.test/a", None),
    ]);
    let view =
        build_directory_tree_view(&reporting_config(7), &pages, None, &Default::default()).unwrap();
    let counts = view.reporting.as_ref().unwrap();
    assert_eq!(
        (
            counts.captured_page_observations,
            counts.profile_visible_page_observations,
            counts.profile_excluded_page_observations
        ),
        (8, 5, 3)
    );
    assert_eq!(
        (
            counts.selected_page_observations,
            counts.hierarchy_page_observations,
            counts.unrepresented_page_observations
        ),
        (5, 2, 3)
    );
    assert_eq!(root(&view.tree).counts.observations, 2);
    let mut hidden = page("https://x.test/hidden/only", Some(200));
    hidden.meta_robots = vec!["noindex".into()];
    let empty =
        build_directory_tree_view(&reporting_config(1), &[hidden], None, &Default::default())
            .unwrap();
    assert_eq!(empty.tree.nodes.len(), 1);
    assert_eq!(root(&empty.tree).counts.observations, 0);
    assert_eq!(
        empty
            .reporting
            .unwrap()
            .excluded_from_view_page_observations,
        1
    );
}
#[test]
fn directory_reporting_legacy_none_null_all_false_keep_tree_json_and_csv_bytes() {
    let pages = reporting_pages();
    let legacy = tree(&pages);
    let csv = directory_tree_csv(&legacy, DIRECTORY_TREE_CSV_BYTES).unwrap();
    let json = serde_json::to_vec(&legacy).unwrap();
    let original = serde_json::to_value(CrawlConfig::default()).unwrap();
    for profile in [
        None,
        Some(serde_json::Value::Null),
        Some(serde_json::to_value(DirectiveReportingConfig::default()).unwrap()),
    ] {
        let mut wire = original.clone();
        if let Some(profile) = profile {
            wire["directive_reporting"] = profile;
        }
        let config: CrawlConfig = serde_json::from_value(wire).unwrap();
        assert_eq!(serde_json::to_value(&config).unwrap(), original);
        let view = build_directory_tree_view(&config, &pages, None, &Default::default()).unwrap();
        assert!(view.reporting.is_none());
        assert_eq!(serde_json::to_vec(&view.tree).unwrap(), json);
        assert_eq!(
            directory_tree_csv_for_view(
                &view.tree,
                view.reporting.as_ref(),
                DIRECTORY_TREE_CSV_BYTES
            )
            .unwrap(),
            csv
        );
    }
}
#[test]
fn directory_reporting_view_identity_binds_mode_and_profile_even_when_rows_match() {
    let pages = vec![page("https://x.test/plain", Some(200))];
    let first =
        build_directory_tree_view(&reporting_config(1), &pages, None, &Default::default()).unwrap();
    let raw = build_directory_tree_view(
        &reporting_config(1),
        &pages,
        Some(DirectoryTreeMode::Raw),
        &Default::default(),
    )
    .unwrap();
    let second =
        build_directory_tree_view(&reporting_config(2), &pages, None, &Default::default()).unwrap();
    assert_eq!(first.tree, raw.tree);
    assert_eq!(first.tree, second.tree);
    assert_ne!(
        first.reporting.as_ref().unwrap().identity_sha256,
        raw.reporting.unwrap().identity_sha256
    );
    assert_ne!(
        first.reporting.unwrap().identity_sha256,
        second.reporting.unwrap().identity_sha256
    );
    assert!(serde_json::from_str::<DirectoryTreeMode>("\"latest\"").is_err());
}
#[test]
fn directory_reporting_csv_retains_complete_nodes_counts_formula_guard_and_byte_limits() {
    let mut pages = reporting_pages();
    pages.push(page("https://x.test/=SUM(1)", Some(200)));
    let view =
        build_directory_tree_view(&reporting_config(7), &pages, None, &Default::default()).unwrap();
    let bytes = directory_tree_csv_for_view(
        &view.tree,
        view.reporting.as_ref(),
        DIRECTORY_TREE_CSV_BYTES,
    )
    .unwrap();
    let mut reader = csv::Reader::from_reader(bytes.as_slice());
    let headers = reader.headers().unwrap().clone();
    assert_eq!(headers.len(), 33);
    let position = |label: &str| headers.iter().position(|v| v == label).unwrap();
    let rows: Vec<_> = reader.records().map(|row| row.unwrap()).collect();
    assert_eq!(rows.len(), view.tree.nodes.len());
    for row in &rows {
        assert_eq!(&row[position("view_mode")], "reported");
        assert_eq!(&row[position("captured_page_observations")], "6");
        assert_eq!(&row[position("selected_page_observations")], "3");
        assert_eq!(&row[position("profile_excluded_page_observations")], "3");
    }
    assert!(rows.iter().any(|r| &r[position("label")] == "'=SUM(1)"));
    assert_eq!(
        directory_tree_csv_for_view(&view.tree, view.reporting.as_ref(), bytes.len()).unwrap(),
        bytes
    );
    assert!(
        directory_tree_csv_for_view(&view.tree, view.reporting.as_ref(), bytes.len() - 1).is_err()
    );
    assert!(
        build_directory_tree_view(
            &reporting_config(7),
            &pages,
            None,
            &DirectoryTreeLimits {
                max_pages: 2,
                ..Default::default()
            }
        )
        .is_err()
    );
}
