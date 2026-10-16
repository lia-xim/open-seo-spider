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
    let report = Aggregate::legacy(aggregate_saved_ngrams(&docs, &Default::default()).unwrap());
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
    let report = Aggregate::legacy(aggregate_saved_ngrams(&docs, &options).unwrap());
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

pub(super) fn reporting_profile(mask: u8) -> open_seo_spider_core::DirectiveReportingConfig {
    open_seo_spider_core::DirectiveReportingConfig {
        version: 1,
        respect_noindex: mask & 1 != 0,
        respect_canonical: mask & 2 != 0,
        respect_next_prev: mask & 4 != 0,
    }
}
// Independent finite RFC4180 decoder for exported cells, including embedded quotes/newlines.
pub(super) fn reporting_csv(bytes: &[u8]) -> Vec<Vec<String>> {
    let text = std::str::from_utf8(bytes).unwrap();
    let mut chars = text.chars().peekable();
    let (mut rows, mut row, mut cell, mut quoted) = (Vec::new(), Vec::new(), String::new(), false);
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cell.push('"');
                } else {
                    quoted = false;
                }
            } else {
                cell.push(c);
            }
        } else {
            match c {
                '"' => {
                    assert!(cell.is_empty());
                    quoted = true;
                }
                ',' => row.push(std::mem::take(&mut cell)),
                '\r' => {
                    assert_eq!(chars.next(), Some('\n'));
                    row.push(std::mem::take(&mut cell));
                    rows.push(std::mem::take(&mut row));
                }
                _ => cell.push(c),
            }
        }
    }
    assert!(!quoted && row.is_empty() && cell.is_empty());
    rows
}
#[test]
fn saved_ngram_reporting_request_serde_strict_and_default_legacy_bytes_token_csv() {
    let report = legacy_report();
    let absent: Request = serde_json::from_str("{}").unwrap();
    let null: Request = serde_json::from_str(r#"{"reporting_mode":null}"#).unwrap();
    assert_eq!(
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&null).unwrap()
    );
    assert!(
        serde_json::to_value(&absent)
            .unwrap()
            .get("reporting_mode")
            .is_none()
    );
    for value in [json!("other"), json!(true), json!({})] {
        assert!(serde_json::from_value::<Request>(json!({"reporting_mode":value})).is_err());
    }
    let request = Request {
        view: View::Sources,
        ..Default::default()
    };
    let ctx = context(&report, Some("legacy".into()), Some(7), "declared").unwrap();
    let corpus = aggregate(&report, &request).unwrap();
    assert_eq!(
        bounded_hash(&corpus, CSV_BYTES).unwrap(),
        bounded_hash(&corpus.corpus, CSV_BYTES).unwrap(),
        "Legacy aggregate bytes are unchanged by omitted envelope"
    );
    let (original, snapshot) = preview(&corpus, ctx.clone(), &request).unwrap();
    let old_options = bounded_hash(&json!({"source_kind":request.source_kind,"options":request.options,"view":request.view,"n":request.n,"query":request.query,"exact_tokens":request.exact_tokens,"limit":request.limit}), REQUEST_BYTES).unwrap();
    assert_eq!(snapshot.options_sha256, old_options);
    assert_eq!(
        snapshot.snapshot_token,
        bounded_hash(
            &(&ctx, &old_options, &snapshot.aggregate_sha256),
            REQUEST_BYTES
        )
        .unwrap()
    );
    let export = Request {
        expected_snapshot_token: Some(snapshot.snapshot_token.clone()),
        ..request.clone()
    };
    let (original_csv, _) = csv(&corpus, ctx.clone(), &export).unwrap();
    let golden = format!(
        "\"snapshot_token\",\"view\",\"n\",\"tokens_json\",\"occurrences\",\"document_frequency\",\"source_ordinal\",\"requested_url\",\"source_revision\",\"capture_config_sha256\",\"text_sha256\",\"basis\",\"observed_generation\",\"source_state\",\"source_counted\",\"first_positions_json\",\"positions_sampled\",\"website_requests\"\r\n\"{}\",\"sources\",\"\",\"null\",\"\",\"\",\"0\",\"https://example.test/legacy\",\"\",\"\",\"\",\"captured_in_crawl\",\"\",\"missing_or_unavailable\",\"false\",\"null\",\"\",\"0\"\r\n",
        snapshot.snapshot_token
    );
    assert_eq!(original_csv, golden.as_bytes());
    for profile in [None, Some(reporting_profile(0))] {
        let mut r = report.clone();
        r.config.directive_reporting = profile;
        let c = context(&r, Some("legacy".into()), Some(7), "declared").unwrap();
        let a = aggregate(&r, &request).unwrap();
        assert_eq!(
            preview(&a, c.clone(), &null).unwrap().1.snapshot_token,
            preview(&a, c.clone(), &absent).unwrap().1.snapshot_token
        );
        assert_eq!(preview(&a, c.clone(), &request).unwrap().0, original);
        assert_eq!(csv(&a, c, &export).unwrap().0, original_csv);
    }
}
#[test]
fn saved_ngram_reporting_equal_rows_still_bind_mode_profile_and_aggregate_basis() {
    let mut report = legacy_report();
    report.config.directive_reporting = Some(reporting_profile(1));
    let original = serde_json::to_vec(&report).unwrap();
    let q = Request {
        view: View::Sources,
        ..Default::default()
    };
    let ctx = context(&report, Some("job".into()), Some(7), "declared").unwrap();
    let shown = aggregate(&report, &q).unwrap();
    let raw_request = Request {
        reporting_mode: Some(ReportingMode::Raw),
        ..q.clone()
    };
    let raw = aggregate(&report, &raw_request).unwrap();
    assert_eq!(
        serde_json::to_value(&shown.corpus).unwrap(),
        serde_json::to_value(&raw.corpus).unwrap()
    );
    let (_, a) = preview(&shown, ctx.clone(), &q).unwrap();
    let (_, b) = preview(&raw, ctx.clone(), &raw_request).unwrap();
    assert_ne!(a.snapshot_token, b.snapshot_token);
    assert_eq!(a.caller.typed_report_sha256, b.caller.typed_report_sha256);
    assert!(
        preview(&shown, ctx.clone(), &raw_request).is_err(),
        "Cannot reinterpret an already aggregated corpus, even without a supplied token"
    );
    let explicit = Request {
        reporting_mode: Some(ReportingMode::Reported),
        ..q.clone()
    };
    assert_eq!(
        preview(&shown, ctx.clone(), &explicit)
            .unwrap()
            .1
            .snapshot_token,
        a.snapshot_token
    );
    let stale = Request {
        expected_snapshot_token: Some(a.snapshot_token),
        ..raw_request
    };
    assert!(preview(&raw, ctx.clone(), &stale).is_err());
    report.config.directive_reporting = Some(reporting_profile(3));
    let changed = aggregate(&report, &q).unwrap();
    let changed_ctx = context(&report, Some("job".into()), Some(7), "declared").unwrap();
    assert!(preview(&shown, changed_ctx.clone(), &q).is_err());
    assert_ne!(
        preview(&changed, changed_ctx, &q).unwrap().1.snapshot_token,
        b.snapshot_token
    );
    report.config.directive_reporting = Some(reporting_profile(1));
    assert_eq!(serde_json::to_vec(&report).unwrap(), original);
}
#[test]
fn saved_ngram_reporting_source_paging_csv_keeps_dense_and_raw_ordinals_distinct() {
    let mut report = legacy_report();
    report.config.directive_reporting = Some(reporting_profile(1));
    report.pages = (0..4)
        .map(|i| {
            let mut p = report.pages[0].clone();
            p.url = format!("https://example.test/{i}");
            p.final_url = p.url.clone();
            if i % 2 == 0 {
                p.meta_robots = vec!["noindex".into()];
            }
            p
        })
        .collect();
    let raw_before = serde_json::to_vec(&report).unwrap();
    let q = Request {
        view: View::Sources,
        limit: 1,
        ..Default::default()
    };
    let a = aggregate(&report, &q).unwrap();
    let ctx = context(&report, Some("job".into()), Some(7), "declared").unwrap();
    let (first, snapshot) = preview(&a, ctx.clone(), &q).unwrap();
    let first: Value = serde_json::from_slice(&first).unwrap();
    assert_eq!(first["rows"][0]["source"]["ordinal"], 0);
    assert_eq!(first["rows"][0]["source"]["raw_page_ordinal"], 1);
    assert_eq!(first["reporting"]["captured_page_observations"], 4);
    assert_eq!(first["reporting"]["eligible_page_observations"], 2);
    let continuation = Request {
        after: Some(1),
        expected_snapshot_token: Some(snapshot.snapshot_token.clone()),
        ..q.clone()
    };
    let next: Value =
        serde_json::from_slice(&preview(&a, ctx.clone(), &continuation).unwrap().0).unwrap();
    assert_eq!(next["rows"][0]["source"]["ordinal"], 1);
    assert_eq!(next["rows"][0]["source"]["raw_page_ordinal"], 3);
    assert_eq!(next["has_more"], false);
    assert!(
        preview(
            &a,
            ctx.clone(),
            &Request {
                reporting_mode: Some(ReportingMode::Raw),
                ..continuation
            }
        )
        .is_err()
    );
    let export = Request {
        expected_snapshot_token: Some(snapshot.snapshot_token),
        ..q
    };
    let rows = reporting_csv(&csv(&a, ctx, &export).unwrap().0);
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].len(), 20);
    assert_eq!(rows[1][1], "reporting");
    assert_eq!(rows[2][6], "0");
    assert_eq!(rows[2][19], "1");
    assert_eq!(rows[3][6], "1");
    assert_eq!(rows[3][19], "3");
    assert_eq!(serde_json::to_vec(&report).unwrap(), raw_before);
}
#[test]
fn saved_ngram_reporting_wholly_hidden_retains_counts_and_raw_global_ceiling() {
    let mut report = legacy_report();
    report.config.directive_reporting = Some(reporting_profile(1));
    report.pages[0].meta_robots = vec!["noindex".into()];
    let q = Request {
        view: View::Sources,
        ..Default::default()
    };
    let a = aggregate(&report, &q).unwrap();
    let ctx = context(&report, None, None, "declared").unwrap();
    let (bytes, s) = preview(&a, ctx.clone(), &q).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["total_rows"], 0);
    assert_eq!(value["saved_documents"], 0);
    assert_eq!(value["reporting"]["captured_page_observations"], 1);
    assert_eq!(value["reporting"]["profile_excluded_page_observations"], 1);
    let rows = reporting_csv(
        &csv(
            &a,
            ctx,
            &Request {
                expected_snapshot_token: Some(s.snapshot_token),
                ..q.clone()
            },
        )
        .unwrap()
        .0,
    );
    assert_eq!(rows.len(), 2);
    let metadata: Value = serde_json::from_str(&rows[1][18]).unwrap();
    assert_eq!(metadata["eligible_page_observations"], 0);
    report.pages = vec![report.pages[0].clone(); 50_001];
    assert!(
        aggregate(&report, &q).is_err(),
        "Hidden observations cannot evade the global raw page ceiling"
    );
}
#[tokio::test]
async fn saved_ngram_reporting_offline_cli_modes_and_stale_token_preserve_raw_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("report.json");
    let options = dir.path().join("options.json");
    let mut report = legacy_report();
    report.config.directive_reporting = Some(reporting_profile(1));
    report.pages[0].meta_robots = vec!["noindex".into()];
    let before = serde_json::to_vec(&report).unwrap();
    tokio::fs::write(&path, &before).await.unwrap();
    for (name, mode, expected) in [("reported", None, 0), ("raw", Some(ReportingMode::Raw), 1)] {
        let q = Request {
            view: View::Sources,
            reporting_mode: mode,
            ..Default::default()
        };
        tokio::fs::write(&options, serde_json::to_vec(&q).unwrap())
            .await
            .unwrap();
        let out = dir.path().join(format!("{name}.json"));
        let csv = dir.path().join(format!("{name}.csv"));
        run_cli(
            path.clone(),
            Some(options.clone()),
            out.clone(),
            Some(csv.clone()),
        )
        .await
        .unwrap();
        let value: Value = serde_json::from_slice(&tokio::fs::read(out).await.unwrap()).unwrap();
        assert_eq!(value["saved_documents"], expected);
        assert_eq!(
            reporting_csv(&tokio::fs::read(csv).await.unwrap()).len(),
            expected as usize + 2
        );
    }
    let q = Request {
        view: View::Sources,
        reporting_mode: Some(ReportingMode::Raw),
        ..Default::default()
    };
    let a = aggregate(&report, &q).unwrap();
    let ctx = context(&report, None, None, "explicit_cli_report_file_app_declared").unwrap();
    let (_, token) = preview(&a, ctx, &q).unwrap();
    tokio::fs::write(
        &options,
        serde_json::to_vec(&Request {
            reporting_mode: None,
            expected_snapshot_token: Some(token.snapshot_token),
            ..q
        })
        .unwrap(),
    )
    .await
    .unwrap();
    let out = dir.path().join("must-not-exist.json");
    assert!(
        run_cli(path.clone(), Some(options), out.clone(), None)
            .await
            .is_err()
    );
    assert!(!out.exists());
    assert_eq!(tokio::fs::read(path).await.unwrap(), before);
}
