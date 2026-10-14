//! Directed unit contract cases for the versioned saved-comparison Core contract.
use super::*;
use crate::{
    ContentCaptureConfig, ContentCaptureDecodeStatus, CrawlConfig, CrawlSummary,
    RecrawlParentReference, RecrawlSourceKind, RecrawlTargetObservation, SelectedRecrawlOperation,
};

fn report(pages: Vec<PageRecord>) -> CrawlReport {
    CrawlReport {
        config: CrawlConfig::default(),
        pages,
        issues: Vec::new(),
        summary: CrawlSummary::default(),
        warnings: Vec::new(),
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
        sitemap_source_measurements: None,
    }
}
fn html_page(url: &str, html: &str) -> PageRecord {
    let mut page = PageRecord {
        url: url.into(),
        final_url: url.into(),
        fetched_at: "2026-10-13T01:00:00Z".into(),
        status: Some(200),
        final_status: Some(200),
        content_type: "text/html; charset=UTF-8".into(),
        size_bytes: html.len(),
        internal: true,
        ..Default::default()
    };
    crate::extract_html(&mut page, html, &[Url::parse(url).unwrap()], false);
    page
}
fn add_capture(
    page: &mut PageRecord,
    html: &str,
    config: &ContentCaptureConfig,
    complete: bool,
    generation: u64,
) {
    let document_url = page.final_url.clone();
    crate::content_binding::observe(
        page,
        &config.compile().unwrap(),
        Some(generation),
        crate::content_binding::SourceObservation {
            source_kind: ContentCaptureSourceKind::RawHttpHtml,
            html: Some(html),
            source_complete: complete,
            decode_status: ContentCaptureDecodeStatus::Exact,
            decode_encoding: Some("UTF-8".into()),
            incomplete_reason: (!complete).then(|| "fixture_truncated_source".into()),
            unavailable_reason: None,
            document_url: Some(document_url),
            rendered_at: None,
            entity: Some(crate::content_binding::entity_bytes(
                html.as_bytes(),
                complete,
            )),
        },
    );
    validate_page_content_captures(page).unwrap();
}
fn sample(url: &str, word: &str) -> (PageRecord, String) {
    let html = format!(
        "<html lang=en><head><title>Valid title</title></head><body><main>{word}</main></body></html>"
    );
    let mut page = html_page(url, &html);
    add_capture(&mut page, &html, &ContentCaptureConfig::default(), true, 1);
    (page, html)
}
fn compare(before: &CrawlReport, after: &CrawlReport) -> SavedComparisonResult {
    compare_saved_crawls(before, after, &SavedComparisonConfig::default(), None, None).unwrap()
}
fn field<'a>(result: &'a SavedComparisonResult, name: &str) -> &'a ComparisonField {
    result.pages[0]
        .fields
        .iter()
        .find(|field| field.field == name)
        .unwrap()
}
fn mapping(from_path: &str, to_path: &str) -> ComparisonUrlMapping {
    ComparisonUrlMapping {
        side: ComparisonSide::Before,
        from_origin: "https://staging.test".into(),
        from_path_prefix: from_path.into(),
        to_origin: "https://production.test".into(),
        to_path_prefix: to_path.into(),
    }
}
fn operation(targets: Vec<RecrawlTargetObservation>) -> SelectedRecrawlOperation {
    SelectedRecrawlOperation {
        engine: SELECTED_RECRAWL_ENGINE.into(),
        coverage: SELECTED_RECRAWL_COVERAGE.into(),
        parent: RecrawlParentReference {
            job_id: "parent-job".into(),
            checkpoint_revision: Some(1),
            source_kind: RecrawlSourceKind::CapturedCheckpoint,
            snapshot_sha256: "a".repeat(64),
        },
        plan_sha256: "b".repeat(64),
        inherited_config_sha256: "c".repeat(64),
        selection: Vec::new(),
        retained_count: 1,
        refreshed_count: 0,
        pending_count: targets.len(),
        max_attempts: None,
        admitted_this_run: 0,
        committed_this_run: 0,
        targets,
    }
}
fn target(
    url: &str,
    state: RecrawlTargetState,
    generation: Option<u64>,
) -> RecrawlTargetObservation {
    RecrawlTargetObservation {
        requested_url: url.into(),
        parent_ordinal: 0,
        parent_page_sha256: "d".repeat(64),
        state,
        admitted_attempts: if generation.is_some() { 1 } else { 0 },
        committed_generation: generation,
    }
}

#[test]
fn all_four_saved_membership_classes_with_old_new_values() {
    let a = html_page(
        "https://fixture.test/changed",
        "<title>Old exact title</title><body><h1>Heading</h1></body>",
    );
    let b = html_page(
        "https://fixture.test/changed",
        "<title>New exact title</title><body><h1>Heading</h1></body>",
    );
    let same = html_page(
        "https://fixture.test/same",
        "<title>Same</title><body>unchanged</body>",
    );
    let before = report(vec![
        same.clone(),
        a,
        html_page("https://fixture.test/removed", "<body>old</body>"),
    ]);
    let after = report(vec![
        html_page("https://fixture.test/added", "<body>new</body>"),
        b,
        same,
    ]);
    let result = compare(&before, &after);
    assert_eq!(
        (
            result.counts.added,
            result.counts.removed,
            result.counts.changed,
            result.counts.unchanged
        ),
        (1, 1, 1, 1)
    );
    let changed = result
        .pages
        .iter()
        .find(|page| page.kind == ComparisonPageKind::Changed)
        .unwrap();
    let title = changed
        .fields
        .iter()
        .find(|field| field.field == "titles")
        .unwrap();
    assert_eq!(title.before.value, Some(json!(["Old exact title"])));
    assert_eq!(title.after.value, Some(json!(["New exact title"])));
    assert_eq!(title.outcome, ComparisonFieldOutcome::Changed);
    assert_eq!(result.website_requests, 0);
    assert!(!result.before_snapshot.coverage.website_completeness_proven);
}

#[test]
fn timestamp_and_generation_only_changes_keep_value_classification_unchanged() {
    let (before_page, html) = sample("https://fixture.test/same", "Alpha beta gamma.");
    let mut after_page = before_page.clone();
    after_page.fetched_at = "2026-10-13T02:00:00Z".into();
    after_page.response_ms += 42;
    add_capture(
        &mut after_page,
        &html,
        &ContentCaptureConfig::default(),
        true,
        2,
    );
    let before = report(vec![before_page]);
    let mut after = report(vec![after_page]);
    after.summary.started_at = "new start".into();
    after.summary.finished_at = "new finish".into();
    after.summary.elapsed_ms = 100;
    let result = compare(&before, &after);
    assert_eq!(result.pages[0].kind, ComparisonPageKind::Unchanged);
    assert_ne!(
        result.before_snapshot.typed_report_sha256,
        result.after_snapshot.typed_report_sha256
    );
    let selected = field(&result, "text_sources.raw_http_html.selected_text");
    assert_eq!(selected.outcome, ComparisonFieldOutcome::Unchanged);
    assert_ne!(selected.before.provenance, selected.after.provenance);
}

#[test]
fn graph_and_jsonld_changes_are_detected_with_same_issue_count() {
    let mut a = html_page(
        "https://fixture.test/page",
        "<body><a href='/a'>Same</a><a href='/a'>Same</a></body>",
    );
    a.structured_data = vec![json!({"@type":"Thing","name":"Old"})];
    let mut b = a.clone();
    b.links[1].url = "https://fixture.test/b".into();
    b.structured_data[0]["name"] = json!("New");
    let before = report(vec![a]);
    let after = report(vec![b]);
    assert_eq!(before.issues.len(), after.issues.len());
    let result = compare(&before, &after);
    assert_eq!(
        field(&result, "links").outcome,
        ComparisonFieldOutcome::Changed
    );
    assert_eq!(
        field(&result, "structured_data").outcome,
        ComparisonFieldOutcome::Changed
    );
    assert_eq!(
        field(&result, "links")
            .before
            .value
            .as_ref()
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(result.pages[0].kind, ComparisonPageKind::Changed);
}

#[test]
fn json_object_key_order_is_ignored_but_block_and_array_order_are_retained() {
    let mut a = html_page("https://fixture.test/page", "<body>content</body>");
    a.structured_data =
        vec![serde_json::from_str(r#"{"name":"A","@type":"Thing","values":[1,2]}"#).unwrap()];
    let mut b = a.clone();
    b.structured_data =
        vec![serde_json::from_str(r#"{"values":[1,2],"@type":"Thing","name":"A"}"#).unwrap()];
    let result = compare(&report(vec![a.clone()]), &report(vec![b.clone()]));
    assert_eq!(
        field(&result, "structured_data").outcome,
        ComparisonFieldOutcome::Unchanged
    );
    b.structured_data[0]["values"] = json!([2, 1]);
    assert_eq!(
        field(
            &compare(&report(vec![a]), &report(vec![b])),
            "structured_data"
        )
        .outcome,
        ComparisonFieldOutcome::Changed
    );
}

#[test]
fn empty_legacy_arrays_are_unknown_and_never_proven_equal() {
    let page = PageRecord {
        url: "https://fixture.test/legacy".into(),
        ..Default::default()
    };
    let result = compare(&report(vec![page.clone()]), &report(vec![page]));
    for name in ["links", "structured_data", "titles"] {
        let row = field(&result, name);
        assert_eq!(row.before.state, ComparisonFactState::Unknown);
        assert_eq!(row.before.value, None);
        assert_eq!(row.outcome, ComparisonFieldOutcome::Unknown);
    }
    assert_eq!(result.pages[0].kind, ComparisonPageKind::Unchanged);
    assert!(!result.pages[0].equality_proven_for_compared_fields);
}

#[test]
fn unrecognized_dom_engine_cannot_make_declared_empty_arrays_complete() {
    let mut page = html_page("https://fixture.test/page", "<body>same</body>");
    page.dom_metadata.as_mut().unwrap().engine = "unknown_parser_v42".into();
    let result = compare(&report(vec![page.clone()]), &report(vec![page]));
    assert_eq!(
        field(&result, "links").before.state,
        ComparisonFactState::Partial
    );
    assert!(!field(&result, "links").comparable_complete_values);
}

#[test]
fn observed_empty_graph_and_empty_captured_text_are_not_missing() {
    let html = "<html lang=en><body><nav>excluded text</nav></body></html>";
    let mut page = html_page("https://fixture.test/empty", html);
    add_capture(&mut page, html, &ContentCaptureConfig::default(), true, 1);
    let result = compare(&report(vec![page.clone()]), &report(vec![page]));
    let links = field(&result, "links");
    assert_eq!(links.before.state, ComparisonFactState::ObservedEmpty);
    assert_eq!(links.before.value, Some(json!([])));
    let text = field(&result, "text_sources.raw_http_html.selected_text");
    assert_eq!(text.before.state, ComparisonFactState::ObservedEmpty);
    assert_eq!(text.before.value.as_ref().unwrap()["text_utf8_bytes"], 0);
    assert_eq!(text.outcome, ComparisonFieldOutcome::Unchanged);
}

#[test]
fn incomplete_equal_text_prefixes_are_unknown_and_remain_partial() {
    let html = "<html lang=en><body>prefix retained</body></html>";
    let mut page = html_page("https://fixture.test/partial", html);
    page.truncated = true;
    add_capture(&mut page, html, &ContentCaptureConfig::default(), false, 1);
    let result = compare(&report(vec![page.clone()]), &report(vec![page]));
    let text = field(&result, "text_sources.raw_http_html.selected_text");
    assert_eq!(text.before.state, ComparisonFactState::Partial);
    assert!(text.before.value.is_some());
    assert_eq!(text.outcome, ComparisonFieldOutcome::Unknown);
    assert!(!text.comparable_complete_values);
}

#[test]
fn missing_source_becoming_complete_records_availability_not_old_empty_content() {
    let (captured, _) = sample("https://fixture.test/page", "Alpha beta gamma.");
    let mut missing = captured.clone();
    missing.content_captures = None;
    let result = compare(&report(vec![missing]), &report(vec![captured]));
    let text = field(&result, "text_sources.raw_http_html.selected_text");
    assert_eq!(text.before.state, ComparisonFactState::Missing);
    assert_eq!(text.before.value, None);
    assert!(text.availability_changed);
    assert!(!text.comparable_complete_values);
    assert_eq!(result.pages[0].kind, ComparisonPageKind::Changed);
}

#[test]
fn changed_selection_profile_is_reported_separately_from_content_equivalence() {
    let (a, html) = sample("https://fixture.test/page", "Same selected text.");
    let mut b = a.clone();
    let cfg = ContentCaptureConfig {
        include_selectors: vec!["main".into()],
        ..Default::default()
    };
    add_capture(&mut b, &html, &cfg, true, 1);
    let result = compare(&report(vec![a]), &report(vec![b]));
    assert_eq!(
        field(&result, "text_sources.raw_http_html.profile").outcome,
        ComparisonFieldOutcome::Changed
    );
    let text = field(&result, "text_sources.raw_http_html.selected_text");
    assert_eq!(text.outcome, ComparisonFieldOutcome::Unknown);
    assert_eq!(
        text.reason.as_deref(),
        Some("different_observation_profiles")
    );
    assert_eq!(result.pages[0].kind, ComparisonPageKind::Changed);
}

#[test]
fn selected_text_is_only_included_when_explicitly_requested() {
    let (page, _) = sample("https://fixture.test/page", "Stored text.");
    let before = report(vec![page.clone()]);
    let after = report(vec![page]);
    let hidden = compare(&before, &after);
    assert!(
        field(&hidden, "text_sources.raw_http_html.selected_text")
            .before
            .value
            .as_ref()
            .unwrap()["text"]
            .is_null()
    );
    let cfg = SavedComparisonConfig {
        include_selected_text: true,
        ..Default::default()
    };
    let visible = compare_saved_crawls(&before, &after, &cfg, None, None).unwrap();
    assert_eq!(
        field(&visible, "text_sources.raw_http_html.selected_text")
            .before
            .value
            .as_ref()
            .unwrap()["text"],
        "Stored text."
    );
}

#[test]
fn changed_capture_text_cannot_bypass_existing_binding_integrity() {
    let (a, _) = sample("https://fixture.test/page", "Stored text.");
    let mut b = a.clone();
    b.content_captures.as_mut().unwrap()[0]
        .result
        .as_mut()
        .unwrap()
        .text = Some("tampered".into());
    let error = compare_saved_crawls(
        &report(vec![a]),
        &report(vec![b]),
        &SavedComparisonConfig::default(),
        None,
        None,
    )
    .unwrap_err();
    assert_eq!(error.code, "invalid_saved_text_binding");
}

#[test]
fn exact_query_order_and_encoding_are_distinct_without_mapping() {
    let a = html_page("https://fixture.test/a?x=1&x=%2F", "<body>same</body>");
    let b = html_page("https://fixture.test/a?x=%2F&x=1", "<body>same</body>");
    let result = compare(&report(vec![a]), &report(vec![b]));
    assert_eq!((result.counts.added, result.counts.removed), (1, 1));
}

#[test]
fn explicit_origin_and_path_mapping_preserves_original_facts_and_raw_query() {
    let old_url = "https://staging.test/deploy/a?x=%2F&x=1";
    let new_url = "https://production.test/site/a?x=%2F&x=1";
    let before = report(vec![html_page(old_url, "<body>same</body>")]);
    let after = report(vec![html_page(new_url, "<body>same</body>")]);
    let cfg = SavedComparisonConfig {
        mappings: vec![mapping("/deploy", "/site")],
        ..Default::default()
    };
    let result = compare_saved_crawls(&before, &after, &cfg, None, None).unwrap();
    assert_eq!(result.pages.len(), 1);
    assert_eq!(result.pages[0].comparison_key, new_url);
    assert_eq!(
        result.pages[0]
            .before
            .as_ref()
            .unwrap()
            .original_requested_url,
        old_url
    );
    assert_eq!(
        field(&result, "final_url").before.value,
        Some(json!(old_url))
    );
    assert_eq!(
        field(&result, "final_url").after.value,
        Some(json!(new_url))
    );
    assert_eq!((result.counts.added, result.counts.removed), (0, 0));
}

#[test]
fn path_mapping_obeys_slash_boundaries_and_preserves_encoded_slashes() {
    let cfg = SavedComparisonConfig {
        mappings: vec![mapping("/deploy", "/site")],
        ..Default::default()
    };
    assert_eq!(
        join_key(
            "https://staging.test/deploy/a%2Fb?q=%2F",
            ComparisonSide::Before,
            &cfg
        )
        .unwrap(),
        "https://production.test/site/a%2Fb?q=%2F"
    );
    for url in [
        "https://staging.test/deployment/a",
        "https://staging.test/deploy%2Fa",
    ] {
        assert_eq!(join_key(url, ComparisonSide::Before, &cfg).unwrap(), url);
    }
    let root_cfg = SavedComparisonConfig {
        mappings: vec![mapping("/", "/site")],
        ..Default::default()
    };
    assert_eq!(
        join_key(
            "https://staging.test/a?q=1",
            ComparisonSide::Before,
            &root_cfg
        )
        .unwrap(),
        "https://production.test/site/a?q=1"
    );
    assert_eq!(
        join_key("https://staging.test/", ComparisonSide::Before, &root_cfg).unwrap(),
        "https://production.test/site"
    );
    assert_eq!(
        join_key("https://staging.test//a", ComparisonSide::Before, &root_cfg).unwrap(),
        "https://production.test/site//a"
    );
    assert_eq!(
        join_key(
            "https://staging.test/deploy//a",
            ComparisonSide::Before,
            &cfg
        )
        .unwrap(),
        "https://production.test/site//a"
    );
    assert_eq!(
        join_key("https://staging.test/deploy/", ComparisonSide::Before, &cfg).unwrap(),
        "https://production.test/site/"
    );
    let root_to_root = SavedComparisonConfig {
        mappings: vec![mapping("/", "/")],
        ..Default::default()
    };
    assert_eq!(
        join_key(
            "https://staging.test//a",
            ComparisonSide::Before,
            &root_to_root
        )
        .unwrap(),
        "https://production.test//a"
    );
}

#[test]
fn collisions_on_both_sides_are_reported_before_join_with_all_original_ordinals() {
    let before = report(vec![
        html_page("https://staging.test/deploy/a", "<body>same</body>"),
        html_page("https://production.test/site/a", "<body>same</body>"),
    ]);
    let after_page = html_page("https://production.test/site/a", "<body>same</body>");
    let after = report(vec![after_page.clone(), after_page]);
    let cfg = SavedComparisonConfig {
        mappings: vec![mapping("/deploy", "/site")],
        ..Default::default()
    };
    let error = compare_saved_crawls(&before, &after, &cfg, None, None).unwrap_err();
    assert_eq!(error.code, "ambiguous_snapshot_identity");
    assert_eq!(error.collisions.len(), 2);
    assert_eq!(error.collisions[0].side, ComparisonSide::Before);
    assert_eq!(error.collisions[1].side, ComparisonSide::After);
    assert_eq!(
        error.collisions[0]
            .occurrences
            .iter()
            .map(|row| row.ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
}

#[test]
fn overlapping_mapping_rules_reject_even_if_current_cohort_has_no_matching_url() {
    let cfg = SavedComparisonConfig {
        mappings: vec![
            mapping("/deploy", "/site"),
            mapping("/deploy/section", "/section"),
        ],
        ..Default::default()
    };
    let error = compare_saved_crawls(&report(Vec::new()), &report(Vec::new()), &cfg, None, None)
        .unwrap_err();
    assert_eq!(error.code, "ambiguous_mapping_rules");
}

#[test]
fn pending_selected_text_stays_unknown_and_retains_historical_provenance() {
    let (page, _) = sample("https://fixture.test/page", "Old saved text.");
    let before = report(vec![page.clone()]);
    let mut after = report(vec![page]);
    after.operation = Some(operation(vec![target(
        "https://fixture.test/page",
        RecrawlTargetState::Pending,
        None,
    )]));
    let result = compare(&before, &after);
    let text = field(&result, "text_sources.raw_http_html.selected_text");
    assert_eq!(text.after.state, ComparisonFactState::Unknown);
    assert_eq!(text.after.value, None);
    assert_eq!(
        result.pages[0].after.as_ref().unwrap().basis,
        "pending_selected_historical_parent"
    );
    assert!(text.after.provenance.is_some());
    assert!(!result.pages[0].equality_proven_for_compared_fields);
    assert!(result.after_snapshot.coverage.mixed_age_selected_recrawl);
}

#[test]
fn selected_generation_mismatch_and_duplicate_targets_reject() {
    let (page, _) = sample("https://fixture.test/page", "Saved text.");
    let before = report(vec![page.clone()]);
    let mut after = report(vec![page]);
    after.operation = Some(operation(vec![target(
        "https://fixture.test/page",
        RecrawlTargetState::Committed,
        Some(2),
    )]));
    assert_eq!(
        compare_saved_crawls(
            &before,
            &after,
            &SavedComparisonConfig::default(),
            None,
            None
        )
        .unwrap_err()
        .code,
        "selected_text_generation_mismatch"
    );
    after.operation.as_mut().unwrap().targets = vec![
        target(
            "https://fixture.test/page",
            RecrawlTargetState::Pending,
            None
        );
        2
    ];
    assert_eq!(
        compare_saved_crawls(
            &before,
            &after,
            &SavedComparisonConfig::default(),
            None,
            None
        )
        .unwrap_err()
        .code,
        "ambiguous_selected_target"
    );
}

#[test]
fn native_generation_is_never_invented_and_typed_caller_hash_is_checked() {
    let before = report(vec![html_page(
        "https://fixture.test/page",
        "<body>same</body>",
    )]);
    let after = before.clone();
    let default_result = compare(&before, &after);
    assert!(default_result.before_snapshot.caller_binding.is_none());
    let binding = ComparisonCallerBinding {
        job_id: Some("saved-job".into()),
        generation: Some(7),
        origin: Some("native_declared_by_caller".into()),
        snapshot_sha256: Some(default_result.before_snapshot.typed_report_sha256.clone()),
        sha_profile: Some(COMPARISON_REPORT_HASH_PROFILE.into()),
        ..Default::default()
    };
    let bound = compare_saved_crawls(
        &before,
        &after,
        &SavedComparisonConfig::default(),
        Some(&binding),
        None,
    )
    .unwrap();
    assert_eq!(
        bound
            .before_snapshot
            .caller_binding
            .as_ref()
            .unwrap()
            .generation,
        Some(7)
    );
    assert!(!bound.before_snapshot.caller_binding_attested_by_core);
    let wrong = ComparisonCallerBinding {
        snapshot_sha256: Some("0".repeat(64)),
        ..binding
    };
    assert_eq!(
        compare_saved_crawls(
            &before,
            &after,
            &SavedComparisonConfig::default(),
            Some(&wrong),
            None
        )
        .unwrap_err()
        .code,
        "typed_snapshot_hash_mismatch"
    );
}

#[test]
fn declared_exact_byte_hash_has_separate_profile_and_no_core_attestation() {
    let snapshot = report(Vec::new());
    let binding = ComparisonCallerBinding {
        snapshot_sha256: Some("f".repeat(64)),
        sha_profile: Some("exact-stored-json-bytes-v1".into()),
        ..Default::default()
    };
    let result = compare_saved_crawls(
        &snapshot,
        &snapshot,
        &SavedComparisonConfig::default(),
        Some(&binding),
        None,
    )
    .unwrap();
    assert_eq!(
        result
            .before_snapshot
            .caller_binding
            .as_ref()
            .unwrap()
            .snapshot_sha256
            .as_deref(),
        Some("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
    );
    assert!(!result.before_snapshot.caller_binding_attested_by_core);
    assert_ne!(result.before_snapshot.typed_report_sha256, "f".repeat(64));
}

#[test]
fn combined_input_exact_boundary_succeeds_and_plus_one_rejects() {
    let snapshot = report(vec![html_page(
        "https://fixture.test/page",
        "<body>same</body>",
    )]);
    let size = serde_json::to_vec(&snapshot).unwrap().len() * 2;
    let mut cfg = SavedComparisonConfig::default();
    cfg.limits.max_input_json_bytes = size;
    compare_saved_crawls(&snapshot, &snapshot, &cfg, None, None).unwrap();
    cfg.limits.max_input_json_bytes -= 1;
    assert_eq!(
        compare_saved_crawls(&snapshot, &snapshot, &cfg, None, None)
            .unwrap_err()
            .code,
        "serialized_byte_budget"
    );
}

#[test]
fn page_and_output_budgets_fail_without_silent_first_n() {
    let snapshot = report(vec![
        html_page("https://fixture.test/a", "<body>a</body>"),
        html_page("https://fixture.test/b", "<body>b</body>"),
    ]);
    let mut cfg = SavedComparisonConfig::default();
    cfg.limits.max_pages_per_snapshot = 1;
    assert_eq!(
        compare_saved_crawls(&snapshot, &snapshot, &cfg, None, None)
            .unwrap_err()
            .code,
        "page_count_budget"
    );
    cfg = SavedComparisonConfig::default();
    cfg.limits.max_output_json_bytes = 1;
    assert_eq!(
        compare_saved_crawls(&snapshot, &snapshot, &cfg, None, None)
            .unwrap_err()
            .code,
        "serialized_byte_budget"
    );
}

#[test]
fn strict_engine_limits_unknown_config_and_invalid_mapping_inputs_reject() {
    assert!(serde_json::from_value::<SavedComparisonConfig>(json!({"unknown":true})).is_err());
    let snapshot = report(Vec::new());
    let mut cfg = SavedComparisonConfig {
        engine: "unknown/v3".into(),
        ..Default::default()
    };
    assert_eq!(
        compare_saved_crawls(&snapshot, &snapshot, &cfg, None, None)
            .unwrap_err()
            .code,
        "unsupported_comparison_engine"
    );
    cfg = SavedComparisonConfig::default();
    cfg.limits.max_mappings = 0;
    assert_eq!(
        compare_saved_crawls(&snapshot, &snapshot, &cfg, None, None)
            .unwrap_err()
            .code,
        "invalid_comparison_limit"
    );
    cfg = SavedComparisonConfig {
        mappings: vec![mapping("/deploy", "/site")],
        ..Default::default()
    };
    cfg.mappings[0].from_origin = "https://staging.test/".into();
    assert_eq!(
        compare_saved_crawls(&snapshot, &snapshot, &cfg, None, None)
            .unwrap_err()
            .code,
        "invalid_mapping_origin"
    );
    cfg.mappings[0].from_origin = "https://staging.test".into();
    cfg.mappings[0].from_path_prefix = "/deploy/../escape".into();
    assert_eq!(
        compare_saved_crawls(&snapshot, &snapshot, &cfg, None, None)
            .unwrap_err()
            .code,
        "invalid_mapping_path_prefix"
    );
}

#[test]
fn failed_selection_cannot_make_retained_prefix_html_hash_complete() {
    let html = "<html lang=en><body>captured prefix</body></html>";
    let mut page = html_page("https://fixture.test/error", html);
    page.truncated = true;
    let mut config = ContentCaptureConfig::default();
    config.limits.max_html_utf8_bytes = 1;
    add_capture(&mut page, html, &config, false, 1);
    assert_eq!(
        page.content_captures.as_ref().unwrap()[0].status,
        PageContentCaptureStatus::Error
    );
    let result = compare(&report(vec![page.clone()]), &report(vec![page]));
    let html_fact = field(&result, "text_sources.raw_http_html.html_representation");
    assert_eq!(html_fact.before.state, ComparisonFactState::Error);
    assert!(
        html_fact.before.value.is_some(),
        "actual retained prefix hash remains available"
    );
    assert!(!html_fact.comparable_complete_values);
    assert_eq!(html_fact.outcome, ComparisonFieldOutcome::Unknown);
}

#[test]
fn default_indexability_and_header_arrays_have_no_fabricated_observation_basis() {
    let page = PageRecord {
        url: "https://fixture.test/legacy".into(),
        final_status: Some(200),
        ..Default::default()
    };
    let result = compare(&report(vec![page.clone()]), &report(vec![page]));
    for name in [
        "indexable",
        "effective_robots",
        "indexability_agent",
        "header_canonical",
        "x_robots_tag",
    ] {
        assert_eq!(
            field(&result, name).before.state,
            ComparisonFactState::Unknown,
            "{name}"
        );
        assert_eq!(field(&result, name).before.value, None, "{name}");
    }
    // Extraction records a DOM and robots agent but does not execute indexability.
    // A default false result with no reasons is still not an evaluated outcome.
    let page = html_page("https://fixture.test/parsed", "<body>parsed</body>");
    assert!(page.dom_metadata.is_some());
    assert!(!page.indexability_agent.is_empty());
    assert!(!page.indexable);
    assert!(page.indexability_reasons.is_empty());
    let result = compare(&report(vec![page.clone()]), &report(vec![page]));
    for name in ["indexable", "indexability_reasons"] {
        assert_eq!(
            field(&result, name).before.state,
            ComparisonFactState::Unknown
        );
        assert_eq!(field(&result, name).before.value, None);
    }
    let mut page = html_page("https://fixture.test/evaluated", "<body>indexable</body>");
    crate::audit::set_indexability(&mut page);
    assert!(page.indexable);
    assert!(page.indexability_reasons.is_empty());
    let result = compare(&report(vec![page.clone()]), &report(vec![page]));
    assert_eq!(field(&result, "indexable").before.value, Some(json!(true)));
    assert!(field(&result, "indexability_reasons").comparable_complete_values);

    let mut page = html_page(
        "https://fixture.test/noindex",
        "<meta name=robots content=noindex><body>excluded</body>",
    );
    crate::audit::set_indexability(&mut page);
    assert!(!page.indexable);
    assert!(!page.indexability_reasons.is_empty());
    let result = compare(&report(vec![page.clone()]), &report(vec![page]));
    assert_eq!(field(&result, "indexable").before.value, Some(json!(false)));
    assert!(field(&result, "indexability_reasons").comparable_complete_values);
}

#[test]
fn complete_declared_header_inventory_supports_observed_empty_relationships() {
    let mut page = html_page("https://fixture.test/page", "<body>same</body>");
    page.headers
        .insert("content-type".into(), "text/html".into());
    let result = compare(&report(vec![page.clone()]), &report(vec![page]));
    assert_eq!(
        field(&result, "header_canonical").before.value,
        Some(json!([]))
    );
    assert!(field(&result, "header_canonical").comparable_complete_values);
}

#[test]
fn actual_bounded_and_failed_custom_extractions_keep_partial_error_states() {
    let html = "<body><p>one</p><p>two</p></body>";
    let mut page = html_page("https://fixture.test/page", html);
    let definition = crate::CustomExtractionConfig {
        name: "paragraph".into(),
        selector: "p".into(),
        max_results: 1,
        ..Default::default()
    };
    page.custom_extractions =
        vec![crate::extract_configured(html, &definition, "raw_html").unwrap()];
    assert!(page.custom_extractions[0].truncated);
    let result = compare(&report(vec![page.clone()]), &report(vec![page.clone()]));
    assert_eq!(
        field(&result, "custom_extractions").before.state,
        ComparisonFactState::Partial
    );
    assert!(!field(&result, "custom_extractions").comparable_complete_values);
    page.custom_extractions[0].error = Some("controlled extraction error".into());
    let failed = compare(&report(vec![page.clone()]), &report(vec![page]));
    assert_eq!(
        field(&failed, "custom_extractions").before.state,
        ComparisonFactState::Error
    );
    assert!(!field(&failed, "custom_extractions").comparable_complete_values);
}

#[test]
fn selected_target_index_rejects_targets_without_saved_rows() {
    let before = report(vec![html_page(
        "https://fixture.test/page",
        "<body>same</body>",
    )]);
    let mut after = before.clone();
    after.operation = Some(operation(vec![target(
        "https://fixture.test/absent",
        RecrawlTargetState::Pending,
        None,
    )]));
    assert_eq!(
        compare_saved_crawls(
            &before,
            &after,
            &SavedComparisonConfig::default(),
            None,
            None
        )
        .unwrap_err()
        .code,
        "selected_target_not_in_snapshot"
    );
}

#[test]
fn inherited_and_refreshed_selected_rows_retain_separate_actual_generation_provenance() {
    let (a, _) = sample("https://fixture.test/inherited", "Inherited text.");
    let (b, html) = sample("https://fixture.test/refreshed", "Refreshed text.");
    let before = report(vec![a.clone(), b.clone()]);
    let mut new_b = b;
    add_capture(&mut new_b, &html, &ContentCaptureConfig::default(), true, 2);
    let mut after = report(vec![a, new_b]);
    let mut selected = operation(vec![target(
        "https://fixture.test/refreshed",
        RecrawlTargetState::Committed,
        Some(2),
    )]);
    selected.pending_count = 0;
    selected.refreshed_count = 1;
    selected.admitted_this_run = 1;
    selected.committed_this_run = 1;
    after.operation = Some(selected);
    let result = compare(&before, &after);
    let inherited = result
        .pages
        .iter()
        .find(|page| page.comparison_key.ends_with("/inherited"))
        .unwrap();
    let refreshed = result
        .pages
        .iter()
        .find(|page| page.comparison_key.ends_with("/refreshed"))
        .unwrap();
    assert_eq!(inherited.after.as_ref().unwrap().basis, "inherited_parent");
    assert_eq!(
        refreshed.after.as_ref().unwrap().basis,
        "refreshed_selected"
    );
    assert_eq!(
        refreshed
            .after
            .as_ref()
            .unwrap()
            .committed_selected_generation,
        Some(2)
    );
    assert_eq!(
        inherited
            .after
            .as_ref()
            .unwrap()
            .committed_selected_generation,
        None
    );
}

#[test]
fn review_legacy_body_hash_requires_complete_declared_matching_source() {
    let mut page = html_page("https://fixture.test/page", "<body>retained prefix</body>");
    assert!(page.content_hash.is_some());
    page.truncated = true;
    let result = compare(&report(vec![page.clone()]), &report(vec![page]));
    let hash = field(&result, "content_hash");
    assert_eq!(hash.before.state, ComparisonFactState::Partial);
    assert!(!hash.comparable_complete_values);
    assert_eq!(hash.outcome, ComparisonFieldOutcome::Unknown);

    let (page, _) = sample("https://fixture.test/complete", "captured complete text");
    let result = compare(&report(vec![page.clone()]), &report(vec![page.clone()]));
    assert!(field(&result, "content_hash").comparable_complete_values);
    assert!(field(&result, "text_sources.raw_http_html.selected_text").comparable_complete_values);
    let mut legacy = page;
    legacy.dom_metadata = None;
    let result = compare(&report(vec![legacy.clone()]), &report(vec![legacy]));
    assert_eq!(
        field(&result, "content_hash").before.state,
        ComparisonFactState::Partial
    );
    assert!(!field(&result, "content_hash").comparable_complete_values);

    let html = "<body>same body text</body>";
    let raw = html_page("https://fixture.test/profile", html);
    let mut rendered = raw.clone();
    crate::extract::extract_html_in_source(
        &mut rendered,
        html,
        &[Url::parse(&raw.url).unwrap()],
        false,
        &[],
        "rendered_html",
    );
    assert_eq!(raw.content_hash, rendered.content_hash);
    let result = compare(&report(vec![raw]), &report(vec![rendered]));
    let hash = field(&result, "content_hash");
    assert_eq!(hash.outcome, ComparisonFieldOutcome::Unknown);
    assert!(!hash.comparable_complete_values);
    assert_ne!(hash.before.profile, hash.after.profile);
}

#[test]
fn review_mapping_rejects_ambiguous_backslash_authority_instead_of_wrong_join() {
    let before = report(vec![PageRecord {
        url: r"https://staging.test\private".into(),
        ..Default::default()
    }]);
    let after = report(vec![html_page(
        "https://production.test/",
        "<body>root</body>",
    )]);
    let config = SavedComparisonConfig {
        mappings: vec![mapping("/", "/")],
        ..Default::default()
    };
    let error = compare_saved_crawls(&before, &after, &config, None, None).unwrap_err();
    assert_eq!(error.code, "ambiguous_mapping_identity");
    let exact = compare(&before, &before);
    assert_eq!(
        exact.pages[0].comparison_key,
        r"https://staging.test\private"
    );
    assert_eq!(
        exact.pages[0]
            .before
            .as_ref()
            .unwrap()
            .original_requested_url,
        r"https://staging.test\private"
    );
    assert_eq!(
        join_key(
            "https://staging.test/%5Cprivate",
            ComparisonSide::Before,
            &config
        )
        .unwrap(),
        "https://production.test/%5Cprivate"
    );
    assert_eq!(
        join_key(
            r"https://staging.test/path?q=\private",
            ComparisonSide::Before,
            &config
        )
        .unwrap(),
        r"https://production.test/path?q=\private"
    );
}

#[test]
fn review_navigated_header_fields_use_rendered_inventory_not_native_predecessor() {
    let mut page = html_page("https://fixture.test/native", "<body>native</body>");
    page.headers
        .insert("content-type".into(), "text/html".into());
    page.rendered = true;
    page.rendered_url = Some("https://fixture.test/navigated".into());
    page.rendered_status = Some(200);
    page.rendered_headers.clear();
    let result = compare(&report(vec![page.clone()]), &report(vec![page.clone()]));
    for name in [
        "header_canonical",
        "header_hreflang",
        "x_robots_tag",
        "header_link_values",
    ] {
        assert_eq!(
            field(&result, name).before.state,
            ComparisonFactState::Unknown,
            "{name}"
        );
        assert!(!field(&result, name).comparable_complete_values, "{name}");
    }
    page.rendered_headers
        .insert("content-type".into(), "text/html".into());
    let result = compare(&report(vec![page.clone()]), &report(vec![page.clone()]));
    let headers = field(&result, "header_canonical");
    assert!(headers.comparable_complete_values);
    assert_eq!(
        headers.before.profile.as_deref(),
        Some("declared-rendered-header-inventory-v1")
    );
    assert_eq!(
        headers.before.provenance.as_ref().unwrap()["document_url"],
        "https://fixture.test/navigated"
    );

    page.rendered_url = Some(page.final_url.clone());
    page.rendered_headers.clear();
    let result = compare(&report(vec![page.clone()]), &report(vec![page]));
    let headers = field(&result, "header_canonical");
    assert!(headers.comparable_complete_values);
    assert_eq!(
        headers.before.profile.as_deref(),
        Some("declared-native-final-header-inventory-v1")
    );
}

#[test]
fn review_mapping_rejects_empty_raw_authority_but_preserves_actual_path_slashes() {
    let config = SavedComparisonConfig {
        mappings: vec![mapping("/", "/")],
        ..Default::default()
    };
    let ambiguous = "https:////staging.test/private";
    assert_eq!(
        Url::parse(ambiguous).unwrap().host_str(),
        Some("staging.test")
    );
    let result = join_key(ambiguous, ComparisonSide::Before, &config);
    assert!(
        result.is_err(),
        "mapped an empty raw authority into an invented path: {result:?}"
    );
    assert_eq!(result.unwrap_err().code, "ambiguous_mapping_identity");
    assert_eq!(
        join_key(
            ambiguous,
            ComparisonSide::Before,
            &SavedComparisonConfig::default()
        )
        .unwrap(),
        ambiguous
    );
    assert_eq!(
        join_key(
            "https://staging.test//private",
            ComparisonSide::Before,
            &config
        )
        .unwrap(),
        "https://production.test//private"
    );
}
