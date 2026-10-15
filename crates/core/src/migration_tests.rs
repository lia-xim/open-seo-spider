use super::*;
use crate::RedirectHop;
use sha2::{Digest, Sha256};

fn page(old: &str, final_url: &str) -> PageRecord {
    let changed = normalize_url(old, None) != normalize_url(final_url, None);
    PageRecord {
        url: old.into(),
        final_url: final_url.into(),
        status: Some(if changed { 301 } else { 200 }),
        final_status: Some(200),
        fetched_at: "2026-10-15T00:00:00Z".into(),
        redirect_chain: if changed {
            vec![RedirectHop {
                url: old.into(),
                status: 301,
                target: final_url.into(),
            }]
        } else {
            vec![]
        },
        ..PageRecord::default()
    }
}
fn evaluate(csv: &str, pages: &[PageRecord]) -> MigrationReport {
    evaluate_migration_csv(csv.as_bytes(), &MigrationCsvOptions::default(), pages).unwrap()
}

#[test]
fn migration_bom_quoted_csv_headers_non_ascii_and_input_hash_use_existing_importer() {
    let input = "\u{feff}\"Old \"\"URL\"\"\",New\r\n\"HTTPS://EXAMPLE.org:443/alt,a#abschnitt\",\"https://example.org/neu?a=1,2#ziel\"\r\nhttps://example.org/straße,https://example.org/ziel\r\n";
    let options = MigrationCsvOptions {
        old_column: ListColumnSelector::Header {
            name: "Old \"URL\"".into(),
            row: 1,
        },
        expected_column: ListColumnSelector::Header {
            name: "New".into(),
            row: 1,
        },
        ..MigrationCsvOptions::default()
    };
    let report = evaluate_migration_csv(
        input.as_bytes(),
        &options,
        &[
            page("https://example.org/alt,a", "https://example.org/neu?a=1,2"),
            page("https://example.org/straße", "https://example.org/ziel"),
        ],
    )
    .unwrap();
    assert_eq!(
        report.input_sha256,
        format!("{:x}", Sha256::digest(input.as_bytes()))
    );
    assert_eq!(report.input_bytes, input.len());
    assert_eq!(report.website_requests, 0);
    assert_eq!(report.rows.len(), 2);
    assert_eq!(
        report.rows[0].old_value,
        "HTTPS://EXAMPLE.org:443/alt,a#abschnitt"
    );
    assert_eq!(
        report.rows[0].old_url.as_deref(),
        Some("https://example.org/alt,a")
    );
    assert_eq!(report.rows[0].expected_fragment.as_deref(), Some("ziel"));
    assert_eq!(report.rows[0].source_row, 2);
    assert_eq!(report.rows[0].source_line, Some(2));
    assert!(
        report
            .rows
            .iter()
            .all(|r| r.verdict == MigrationVerdict::DestinationMatch)
    );
}

#[test]
fn migration_csv_order_duplicates_and_conflicting_expectations_are_preserved() {
    let report = evaluate(
        "old,new\nhttps://x.test/a,https://x.test/b\nhttps://x.test/a,https://x.test/c\nHTTPS://X.TEST:443/a#one,https://x.test/b#two\n",
        &[page("https://x.test/a", "https://x.test/b")],
    );
    assert_eq!(
        report.rows.iter().map(|r| r.source_row).collect::<Vec<_>>(),
        vec![2, 3, 4]
    );
    assert_eq!(
        report
            .rows
            .iter()
            .map(|r| r.duplicate_of_row)
            .collect::<Vec<_>>(),
        vec![None, None, Some(2)]
    );
    assert!(report.rows.iter().all(|r| r.conflicting_expectations));
    assert_eq!(
        report.rows.iter().map(|r| r.verdict).collect::<Vec<_>>(),
        vec![
            MigrationVerdict::DestinationMatch,
            MigrationVerdict::DestinationMismatch,
            MigrationVerdict::DestinationMatch
        ]
    );
    assert_eq!(report.summary[&MigrationVerdict::DestinationMatch], 2);
    assert_eq!(report.summary[&MigrationVerdict::DestinationMismatch], 1);
}

#[test]
fn migration_missing_and_duplicate_observations_are_unknown_without_latest_selection() {
    let first = page("https://x.test/a", "https://x.test/b");
    let mut second = page("https://x.test/a#other", "https://x.test/c");
    second.fetched_at = "2099-01-01T00:00:00Z".into();
    let report = evaluate(
        "old,new\nhttps://x.test/a,https://x.test/b\nhttps://x.test/absent,https://x.test/b\n",
        &[first, second],
    );
    assert_eq!(
        report.rows[0].verdict,
        MigrationVerdict::AmbiguousObservation
    );
    assert_eq!(report.rows[0].observation_count, 2);
    assert!(report.rows[0].observation.is_none());
    assert!(report.rows[0].destination_matches.is_none());
    assert_eq!(report.rows[1].verdict, MigrationVerdict::MissingObservation);
    assert_eq!(report.rows[1].observation_count, 0);
}

#[test]
fn migration_blocked_error_and_unfetched_proposed_redirect_target_never_pass() {
    let mut blocked = page("https://x.test/a", "https://x.test/a");
    blocked.blocked_by_robots = true;
    blocked.status = None;
    blocked.final_status = None;
    let mut failed = page("https://x.test/b", "https://x.test/expected");
    failed.error = Some("Redirect loop detected".into());
    failed.final_status = None;
    let mut stopped = page("https://x.test/c", "https://x.test/expected");
    stopped.final_status = None;
    let report = evaluate(
        "old,new\nhttps://x.test/a,https://x.test/a\nhttps://x.test/b,https://x.test/expected\nhttps://x.test/c,https://x.test/expected\n",
        &[blocked, failed, stopped],
    );
    assert_eq!(
        report.rows.iter().map(|r| r.verdict).collect::<Vec<_>>(),
        vec![
            MigrationVerdict::BlockedByRobots,
            MigrationVerdict::FetchError,
            MigrationVerdict::UnobservedFinal
        ]
    );
    assert!(report.rows.iter().all(|r| r.destination_matches.is_none()));
    assert!(
        report.rows[2]
            .observation
            .as_ref()
            .unwrap()
            .normalized_final_url
            .is_none()
    );
    assert_eq!(
        report.rows[2].observation.as_ref().unwrap().raw_final_url,
        "https://x.test/expected"
    );
}

#[test]
fn migration_matching_error_http_status_and_fetch_error_remain_failures() {
    let mut missing = page("https://x.test/a", "https://x.test/b");
    missing.final_status = Some(404);
    let mut failed = page("https://x.test/c", "https://x.test/d");
    failed.error = Some("body read failed".into());
    let mut not_modified = page("https://x.test/e", "https://x.test/e");
    not_modified.status = Some(304);
    not_modified.final_status = Some(304);
    let report = evaluate(
        "old,new\nhttps://x.test/a,https://x.test/b\nhttps://x.test/c,https://x.test/d\nhttps://x.test/e,https://x.test/e\n",
        &[missing, failed, not_modified],
    );
    assert_eq!(report.rows[0].verdict, MigrationVerdict::FinalHttpFailure);
    assert_eq!(report.rows[1].verdict, MigrationVerdict::FetchError);
    assert_eq!(report.rows[2].verdict, MigrationVerdict::FinalHttpFailure);
    assert!(
        report
            .rows
            .iter()
            .all(|r| r.destination_matches == Some(true))
    );
}

#[test]
fn migration_multihop_chain_and_non_200_success_compare_actual_http_not_renderer() {
    let mut observed = page("https://x.test/a", "https://x.test/c");
    observed.status = Some(307);
    observed.final_status = Some(204);
    observed.redirect_chain = vec![
        RedirectHop {
            url: "https://x.test/a".into(),
            status: 307,
            target: "https://x.test/b".into(),
        },
        RedirectHop {
            url: "https://x.test/b".into(),
            status: 308,
            target: "https://x.test/c".into(),
        },
    ];
    observed.rendered = true;
    observed.rendered_url = Some("https://x.test/browser-only".into());
    observed.rendered_status = Some(200);
    let report = evaluate("old,new\nhttps://x.test/a,https://x.test/c\n", &[observed]);
    assert_eq!(report.rows[0].verdict, MigrationVerdict::DestinationMatch);
    assert_eq!(
        report.rows[0]
            .observation
            .as_ref()
            .unwrap()
            .redirect_chain
            .len(),
        2
    );
    assert_eq!(
        report.rows[0].observation.as_ref().unwrap().final_status,
        Some(204)
    );
}

#[test]
fn migration_incoherent_chain_status_continuity_tail_and_loop_are_not_success() {
    let good = page("https://x.test/a", "https://x.test/b");
    let mut candidates = Vec::new();
    let mut p = good.clone();
    p.redirect_chain[0].status = 200;
    candidates.push(p);
    let mut p = good.clone();
    p.redirect_chain[0].url = "https://x.test/foreign".into();
    candidates.push(p);
    let mut p = good.clone();
    p.redirect_chain[0].target = "https://x.test/other".into();
    candidates.push(p);
    let mut p = good.clone();
    p.redirect_chain[0].target = "mailto:a@example.org".into();
    candidates.push(p);
    let mut p = good.clone();
    p.status = Some(302);
    candidates.push(p);
    let mut p = good.clone();
    p.redirect_chain.clear();
    candidates.push(p);
    let mut p = good;
    p.final_url = "https://x.test/a".into();
    p.redirect_chain.push(RedirectHop {
        url: "https://x.test/b".into(),
        status: 302,
        target: "https://x.test/a".into(),
    });
    candidates.push(p);
    for observed in candidates {
        let report = evaluate("old,new\nhttps://x.test/a,https://x.test/b\n", &[observed]);
        assert_eq!(
            report.rows[0].verdict,
            MigrationVerdict::InvalidRedirectChain
        );
    }
}

#[test]
fn migration_query_order_case_slash_scheme_and_encoded_spelling_remain_distinct() {
    for target in [
        "https://x.test/Target?b=2&a=1",
        "https://x.test/target?a=1&b=2",
        "https://x.test/Target/?a=1&b=2",
        "http://x.test/Target?a=1&b=2",
        "https://x.test/%54arget?a=1&b=2",
    ] {
        let report = evaluate(
            &format!("old,new\nhttps://x.test/a,{target}\n"),
            &[page("https://x.test/a", "https://x.test/Target?a=1&b=2")],
        );
        assert_eq!(
            report.rows[0].verdict,
            MigrationVerdict::DestinationMismatch,
            "{target}"
        );
    }
    let normalized = evaluate(
        "old,new\nHTTPS://X.TEST:443/a,HTTPS://X.TEST:443/z/../Target?a=1&b=2\n",
        &[page("https://x.test/a", "https://x.test/Target?a=1&b=2")],
    );
    assert_eq!(
        normalized.rows[0].verdict,
        MigrationVerdict::DestinationMatch
    );
}

#[test]
fn migration_fragments_are_retained_but_http_comparison_does_not_claim_anchor_navigation() {
    let report = evaluate(
        "old,new\nhttps://x.test/a#request,https://x.test/b#expected\n",
        &[page("https://x.test/a#saved", "https://x.test/b#other")],
    );
    assert_eq!(report.rows[0].verdict, MigrationVerdict::DestinationMatch);
    assert_eq!(report.rows[0].old_fragment.as_deref(), Some("request"));
    assert_eq!(
        report.rows[0].expected_fragment.as_deref(),
        Some("expected")
    );
    assert_eq!(
        report.rows[0]
            .observation
            .as_ref()
            .unwrap()
            .normalized_final_url
            .as_deref(),
        Some("https://x.test/b")
    );
    assert!(report.comparison.contains("fragments_ignored"));
}

#[test]
fn migration_invalid_mapping_fields_and_blank_records_keep_source_provenance() {
    let report = evaluate(
        "old,new\nhttps://user:secret@x.test/a,https://x.test/b\nhttps://x.test/a\nrelative,https://x.test/b\n\n\"https://x.test/line\nfeed\",https://x.test/b\nhttps://x.test/last,https://x.test/b\n",
        &[],
    );
    assert_eq!(report.rows.len(), 6);
    assert!(
        report.rows[..5]
            .iter()
            .all(|r| r.verdict == MigrationVerdict::InvalidMapping)
    );
    assert!(report.rows[0].old_error.is_some());
    assert!(
        report.rows[1]
            .expected_error
            .as_ref()
            .unwrap()
            .contains("missing")
    );
    assert_eq!(report.rows[3].old_value, "");
    assert_eq!(report.rows[4].old_value, "https://x.test/line\nfeed");
    assert_eq!(report.rows[5].source_row, 7);
    assert_eq!(report.rows[5].source_line, Some(8));
    assert_eq!(report.rows[5].verdict, MigrationVerdict::MissingObservation);
}

#[test]
fn migration_malformed_utf8_quoting_same_columns_and_unknown_options_are_rejected() {
    let default = MigrationCsvOptions::default();
    for bytes in [
        b"".as_slice(),
        b"old,new\n\"unterminated",
        b"old,new\n\"https://x.test/a\"junk,https://x.test/b",
        &[0xff, 0xfe],
    ] {
        assert!(evaluate_migration_csv(bytes, &default, &[]).is_err());
    }
    let same = MigrationCsvOptions {
        expected_column: ListColumnSelector::Index { value: 1 },
        ..default.clone()
    };
    assert!(
        evaluate_migration_csv(b"old,new\nhttps://x.test/a,https://x.test/b\n", &same, &[])
            .is_err()
    );
    let mut value = serde_json::to_value(&default).unwrap();
    value["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<MigrationCsvOptions>(value).is_err());
}

#[test]
fn migration_semicolon_tab_and_header_aliases_follow_existing_list_semantics() {
    for (delimiter, separator) in [
        (ListCsvDelimiter::Semicolon, ';'),
        (ListCsvDelimiter::Tab, '\t'),
    ] {
        let options = MigrationCsvOptions {
            delimiter,
            ..MigrationCsvOptions::default()
        };
        let csv = format!("old{separator}new\r\nhttps://x.test/a{separator}https://x.test/a\r\n");
        let report = evaluate_migration_csv(
            csv.as_bytes(),
            &options,
            &[page("https://x.test/a", "https://x.test/a")],
        )
        .unwrap();
        assert_eq!(report.rows[0].verdict, MigrationVerdict::DestinationMatch);
    }
    let options = MigrationCsvOptions {
        expected_column: ListColumnSelector::Header {
            name: "old".into(),
            row: 1,
        },
        ..MigrationCsvOptions::default()
    };
    assert!(
        evaluate_migration_csv(
            b"old,new\nhttps://x.test/a,https://x.test/a\n",
            &options,
            &[]
        )
        .unwrap_err()
        .to_string()
        .contains("columns must differ")
    );
}

#[test]
fn migration_supplied_invalid_status_url_and_unexplained_final_url_are_explicit() {
    let good = page("https://x.test/a", "https://x.test/a");
    for n in 0..4 {
        let mut observed = good.clone();
        match n {
            0 => observed.final_status = Some(600),
            1 => observed.status = None,
            2 => observed.final_url = "file:///x".into(),
            _ => observed.final_url = "https://x.test/other".into(),
        }
        let report = evaluate("old,new\nhttps://x.test/a,https://x.test/a\n", &[observed]);
        assert_eq!(
            report.rows[0].verdict,
            if n == 3 {
                MigrationVerdict::InvalidRedirectChain
            } else {
                MigrationVerdict::InvalidObservation
            }
        );
    }
    let mut invalid = good;
    invalid.url = "https://user:password@x.test/".into();
    assert!(
        evaluate_migration_csv(
            b"old,new\nhttps://x.test/a,https://x.test/a\n",
            &MigrationCsvOptions::default(),
            &[invalid]
        )
        .is_err()
    );
}

#[test]
fn migration_import_row_field_input_and_observation_caps_fail_before_result_retention() {
    let csv = b"old,new\nhttps://x.test/a,https://x.test/b\n";
    let default = MigrationCsvOptions::default();
    let mut options = default.clone();
    options.import_limits.max_input_bytes = csv.len() - 1;
    assert!(evaluate_migration_csv(csv, &options, &[]).is_err());
    let mut options = default.clone();
    options.import_limits.max_rows = 1;
    assert!(evaluate_migration_csv(csv, &options, &[]).is_err());
    let mut options = default.clone();
    options.import_limits.max_field_bytes = 3;
    assert!(evaluate_migration_csv(csv, &options, &[]).is_err());
    let mut options = default.clone();
    options.observation_limits.max_pages = 1;
    let p = page("https://x.test/a", "https://x.test/b");
    assert!(evaluate_migration_csv(csv, &options, &[p.clone(), p.clone()]).is_err());
    let mut options = default.clone();
    options.observation_limits.max_text_bytes = 1;
    assert!(evaluate_migration_csv(csv, &options, std::slice::from_ref(&p)).is_err());
    let mut long = p.clone();
    long.error = Some("x".repeat(default.import_limits.max_field_bytes + 1));
    assert!(evaluate_migration_csv(csv, &default, &[long]).is_err());
    let mut many = p;
    many.redirect_chain =
        vec![RedirectHop::default(); default.observation_limits.max_redirects_per_page + 1];
    assert!(evaluate_migration_csv(csv, &default, &[many]).is_err());
    let mut options = default;
    options.observation_limits.max_pages = 50_001;
    assert!(evaluate_migration_csv(csv, &options, &[]).is_err());
}

#[test]
fn migration_exact_serialized_output_bound_and_repeated_projection_are_independent() {
    let csv = "old,new\n".to_owned() + &"https://x.test/a,https://x.test/b\n".repeat(20);
    let observed = page("https://x.test/a", "https://x.test/b");
    let report = evaluate(&csv, std::slice::from_ref(&observed));
    let encoded = serde_json::to_vec(&report).unwrap();
    assert!(encoded.len() <= report.options.observation_limits.max_output_bytes);
    assert_eq!(report.rows.len(), 20);
    assert_eq!(report.rows.last().unwrap().duplicate_of_row, Some(2));
    let mut options = MigrationCsvOptions::default();
    options.observation_limits.max_output_bytes = 4096;
    assert!(
        evaluate_migration_csv(csv.as_bytes(), &options, &[observed])
            .unwrap_err()
            .to_string()
            .contains("max_output_bytes")
    );
    let mut options = MigrationCsvOptions::default();
    options.import_limits.max_output_bytes = 1;
    assert!(evaluate_migration_csv(csv.as_bytes(), &options, &[]).is_err());
}

#[test]
fn migration_csv_mapping_remains_byte_bound_and_evaluation_has_no_crawl_side_effect() {
    let mut observed = page("https://x.test/a", "https://x.test/b");
    observed.truncated = true;
    observed.titles = vec!["Unrelated saved HTML-derived data".repeat(1000)];
    let before = serde_json::to_vec(&observed).unwrap();
    let input = b"old,new\nhttps://x.test/a,https://x.test/b\n";
    let report = evaluate_migration_csv(
        input,
        &MigrationCsvOptions::default(),
        std::slice::from_ref(&observed),
    )
    .unwrap();
    assert_eq!(serde_json::to_vec(&observed).unwrap(), before);
    assert_eq!(report.website_requests, 0);
    assert_eq!(
        report.observation_basis,
        "caller_supplied_saved_page_records_not_snapshot_authentication"
    );
    assert_eq!(report.rows[0].verdict, MigrationVerdict::DestinationMatch);
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("Unrelated saved HTML")
    );
}
