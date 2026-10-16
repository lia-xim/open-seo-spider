// Mock HTTP replies below are adapter simulations. Actual engine cases are a separate opt-in feature.
use super::*;
pub(crate) fn request() -> Request {
    Request {
        version: 1,
        selection: Selection {
            urls: vec!["https://saved.invalid/one".into()],
            source_kind: content_cohort::default_source(),
            reporting_mode: None,
            expected_generation: Some(2),
            expected_snapshot_sha256: Some("a".repeat(64)),
            expected_reporting_identity: None,
        },
        config: spelling::Config::default(),
        expected_dictionary_sha256: spelling::hash(&spelling::Dictionary::default()).unwrap(),
    }
}
pub(crate) fn prepared(mut r: Request, text: Option<&str>, complete: bool) -> Prepared {
    r.expected_dictionary_sha256 = spelling::hash(&spelling::Dictionary::default()).unwrap();
    let row = json!({"requested_url":r.selection.urls[0],"ordinal":0,"html_language":"en-US","source_kind":r.selection.source_kind,
        "text_sha256":text.map(|v|spelling::digest(v.as_bytes())),"sample_complete":complete});
    let envelope = json!({"engine":content_cohort::SAVED_CONTENT_ENGINE,"snapshot":{"sha_profile":content_cohort::REPORT_HASH_PROFILE,"sha256":"a".repeat(64),"generation":2,"origin":"synthetic_fixture"},
        "requested_urls":r.selection.urls,"source_kind":r.selection.source_kind,"website_requests":0,
        "indexable_only":false,"complete_only":false,"site_wide_coverage":false,"selected_count":r.selection.urls.len(),"included_count":1,"documents":[{"analysis_document_id":"selection-0","row":row}],"excluded":[]});
    Prepared {
        request: r,
        dictionary: Default::default(),
        envelope,
        documents: vec![open_seo_spider_core::ContentTextInput {
            id: "selection-0".into(),
            source: "synthetic_adapter_fixture".into(),
            text: text.map(str::to_owned),
            language: Some("en-US".into()),
            complete,
            truncation_reason: (!complete).then(|| "simulation incomplete".into()),
        }],
    }
}
fn response() -> Value {
    json!({"software":{"name":"LanguageTool","version":"simulated-v1","buildDate":"fixture","apiVersion":1},
    "language":{"name":"English","code":"en-US"},"warnings":{"incompleteResults":false},
    "matches":[{"message":"Synthetic issue","offset":3,"length":3,"context":{"text":"😀 bad","offset":3,"length":3},
    "replacements":[{"value":"good"}],"rule":{"id":"MOCK_RULE","description":"Synthetic","issueType":"misspelling","category":{"id":"TYPOS","name":"Typos"}}}]})
}
async fn mock(body: Value, status: u16) -> (String, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let owner = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut raw = Vec::new();
        let mut buffer = [0; 4096];
        let header_end = loop {
            let n = socket.read(&mut buffer).await.unwrap();
            assert_ne!(n, 0);
            raw.extend_from_slice(&buffer[..n]);
            assert!(raw.len() <= 256 * 1024);
            if let Some(i) = raw.windows(4).position(|v| v == b"\r\n\r\n") {
                break i + 4;
            }
        };
        let headers = String::from_utf8(raw[..header_end].to_vec()).unwrap();
        assert!(headers.starts_with("POST /v2/check HTTP/1.1"));
        let length = headers
            .lines()
            .find_map(|line| {
                line.split_once(':')
                    .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                    .map(|(_, v)| v.trim().parse::<usize>().unwrap())
            })
            .unwrap();
        while raw.len() < header_end + length {
            let n = socket.read(&mut buffer).await.unwrap();
            assert_ne!(n, 0);
            raw.extend_from_slice(&buffer[..n]);
            assert!(raw.len() <= 256 * 1024);
        }
        let body = serde_json::to_vec(&body).unwrap();
        let head = format!(
            "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        socket.write_all(head.as_bytes()).await.unwrap();
        socket.write_all(&body).await.unwrap();
        socket.shutdown().await.unwrap();
        String::from_utf8(raw[header_end..].to_vec()).unwrap()
    });
    (format!("http://{address}/v2/check"), owner)
}
#[tokio::test]
async fn spelling_disabled_missing_empty_and_large_saved_text_never_claim_checked_zero() {
    let disabled = run(prepared(request(), Some("saved"), true)).await.unwrap();
    assert_eq!(disabled.engine_requests, 0);
    assert_eq!(disabled.accepted_count, None);
    assert_eq!(disabled.documents[0].status, "disabled");
    let mut r = request();
    r.config.enabled = true;
    r.config.acknowledge_local_text_transfer = true;
    r.config.endpoint = Some("http://127.0.0.1:1/v2/check".into());
    for (text, complete, expected) in [
        (None, false, "incomplete_capture"),
        (Some(""), true, "empty_saved_text"),
    ] {
        let report = run(prepared(r.clone(), text, complete)).await.unwrap();
        assert_eq!(report.engine_requests, 0);
        assert_eq!(report.documents[0].status, expected);
        assert!(!report.complete);
    }
    let huge = "x".repeat(spelling::MAX_TEXT_BYTES + 1);
    let report = run(prepared(r, Some(&huge), true)).await.unwrap();
    assert_eq!(report.documents[0].status, "text_bound");
    assert_eq!(report.engine_requests, 0);
}
#[tokio::test]
async fn spelling_simulated_loopback_post_retains_exact_body_and_utf16_and_rechecks_dictionary() {
    let (endpoint, owner) = mock(response(), 200).await;
    let mut r = request();
    r.config.enabled = true;
    r.config.endpoint = Some(endpoint);
    r.config.acknowledge_local_text_transfer = true;
    let report = run(prepared(r, Some("😀 bad"), true)).await.unwrap();
    let form = owner.await.unwrap();
    let mut form_url = reqwest::Url::parse("http://127.0.0.1/").unwrap();
    form_url.set_query(Some(&form));
    let pairs: std::collections::HashMap<_, _> = form_url.query_pairs().into_owned().collect();
    assert_eq!(pairs.get("text").unwrap(), "😀 bad");
    assert_eq!(pairs.get("language").unwrap(), "en-US");
    assert_eq!(report.accepted_count, Some(1));
    assert_eq!(
        report.documents[0].checked.as_ref().unwrap().issues[0].offset_utf16,
        3
    );
    assert!(report.documents[0].provider.as_ref().unwrap().body_complete);
    validate_report(&report, Some(&report.request)).unwrap();
    let mut tampered = report.clone();
    tampered.dictionary.words.push(spelling::Word {
        language: "en".into(),
        word: "bad".into(),
    });
    assert!(validate_report(&tampered, None).is_err());
    let mut wrong = report.request.clone();
    wrong.selection.reporting_mode = Some(ReportingMode::Reported);
    assert!(validate_report(&report, Some(&wrong)).is_err());
    let mut tampered = report.clone();
    tampered.documents[0].checked.as_mut().unwrap().issues[0].offset_utf16 = 0;
    assert!(validate_report(&tampered, None).is_err());
    let mut stale = report.clone();
    stale.dictionary.words.push(spelling::Word {
        language: "en".into(),
        word: "bad".into(),
    });
    stale.dictionary_sha256 = spelling::hash(&stale.dictionary).unwrap();
    stale.request.expected_dictionary_sha256 = stale.dictionary_sha256.clone();
    assert!(
        validate_report(&stale, None).is_err(),
        "Changing only dictionary hashes must not rewrite prior dispositions"
    );
}
#[tokio::test]
async fn spelling_simulated_incomplete_and_http_error_are_not_successful_empty_results() {
    for (body, status, expected) in [
        (
            json!({"error":"fixture unavailable"}),
            503,
            "provider_error",
        ),
        (
            {
                let mut v = response();
                v["warnings"]["incompleteResults"] = json!(true);
                v["matches"] = json!([]);
                v
            },
            200,
            "provider_incomplete",
        ),
    ] {
        let (endpoint, owner) = mock(body, status).await;
        let mut r = request();
        r.config.enabled = true;
        r.config.endpoint = Some(endpoint);
        r.config.acknowledge_local_text_transfer = true;
        let report = run(prepared(r, Some("😀 bad"), true)).await.unwrap();
        owner.await.unwrap();
        assert_eq!(report.documents[0].status, expected);
        assert_eq!(report.accepted_count, None);
        assert!(!report.complete);
    }
}
#[tokio::test]
async fn spelling_persistent_dictionary_and_crawl_preferences_are_separate_and_revision_bound() {
    let dir = tempfile::tempdir().unwrap();
    let dictionary = spelling::Dictionary {
        version: 1,
        words: vec![spelling::Word {
            language: "de".into(),
            word: "Eigenwort".into(),
        }],
    };
    save_json(&dir.path().join("dictionary.json"), &dictionary, false)
        .await
        .unwrap();
    save_json(
        &dir.path().join("crawl-profile.json"),
        &json!({"ignored_rules":["MY_RULE"]}),
        false,
    )
    .await
    .unwrap();
    let loaded: spelling::Dictionary = read_json(&dir.path().join("dictionary.json"), 1024 * 1024)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(dictionary, loaded);
    assert_ne!(
        spelling::hash(&loaded).unwrap(),
        request().expected_dictionary_sha256
    );
    assert!(
        save_json(
            &dir.path().join("dictionary.json"),
            &spelling::Dictionary::default(),
            false
        )
        .await
        .is_err()
    );
}
#[tokio::test]
async fn spelling_csv_retains_failure_rows_and_formula_safe_cells() {
    let mut r = request();
    r.selection.urls[0] = "=HYPERLINK(\"bad\")".into();
    let mut p = prepared(r, None, false);
    p.envelope["documents"][0]["row"]["requested_url"] = json!("=HYPERLINK(\"bad\")");
    let report = run(p).await.unwrap();
    let text = csv(&report).unwrap();
    assert!(text.contains("'=HYPERLINK"));
    assert!(text.contains("disabled"));
    assert!(text.contains("offset_utf16"));
    assert_eq!(report.accepted_count, None);
}
#[test]
fn spelling_requests_refuse_unknowns_version_oversize_and_wrong_dictionary_revision() {
    let mut r = request();
    r.version = 2;
    assert!(r.validate().is_err());
    let mut v = serde_json::to_value(request()).unwrap();
    v["website_url"] = json!("https://example.test");
    assert!(serde_json::from_value::<Request>(v).is_err());
    let mut r = request();
    r.expected_dictionary_sha256 = "wrong".into();
    assert!(r.validate().is_err());
    let mut r = request();
    r.selection.urls = vec!["x".into(); 33];
    assert!(r.validate().is_err());
}
#[cfg(feature = "languagetool-integration")]
#[tokio::test]
async fn spelling_real_local_engine_english_german_utf16_and_dictionary_recheck_without_website() {
    // Root must supply an exact pinned running service. Absence is a failure, never a silent skip.
    let endpoint = std::env::var("OPEN_SEO_SPIDER_TEST_LANGUAGETOOL")
        .expect("Pinned local LanguageTool endpoint required");
    let version = std::env::var("OPEN_SEO_SPIDER_TEST_LANGUAGETOOL_VERSION")
        .expect("Actual software.version pin required");
    spelling::validate_endpoint(&endpoint).unwrap();
    for (language, text, misspelling) in [
        ("en-US", "😀 This is a sentense.", "sentense"),
        ("de-DE", "😀 Das ist ein Feler.", "Feler"),
    ] {
        let mut r = request();
        r.config = spelling::Config {
            enabled: true,
            endpoint: Some(endpoint.clone()),
            acknowledge_local_text_transfer: true,
            expected_engine_version: Some(version.clone()),
            manual_language: Some(language.into()),
            ..Default::default()
        };
        let report = run(prepared(r.clone(), Some(text), true)).await.unwrap();
        assert!(report.complete, "{:#?}", report.documents);
        let issues = &report.documents[0].checked.as_ref().unwrap().issues;
        let actual = issues
            .iter()
            .find(|i| i.matched_text == misspelling && i.issue_type == "misspelling")
            .expect("Actual misspelling rule");
        assert_eq!(
            spelling::utf16_slice(text, actual.offset_utf16, actual.length_utf16).unwrap(),
            misspelling
        );
        let mut p = prepared(r, Some(text), true);
        p.dictionary.words.push(spelling::Word {
            language: language.into(),
            word: misspelling.into(),
        });
        p.request.expected_dictionary_sha256 = spelling::hash(&p.dictionary).unwrap();
        let repeated = run(p).await.unwrap();
        assert!(repeated.complete);
        assert_eq!(repeated.website_requests, 0);
        assert!(
            repeated.documents[0]
                .checked
                .as_ref()
                .unwrap()
                .issues
                .iter()
                .any(|i| i.matched_text == misspelling && i.disposition == "dictionary")
        );
    }
}
fn saved_report() -> CrawlReport {
    use open_seo_spider_core::{CrawlConfig, CrawlSummary, PageRecord};
    CrawlReport {
        config: CrawlConfig::default(),
        pages: vec![PageRecord {
            url: "https://saved.invalid/one".into(),
            ..Default::default()
        }],
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
#[tokio::test]
async fn spelling_saved_missing_capture_stays_unknown_and_raw_report_is_unchanged() {
    let raw = saved_report();
    let before = serde_json::to_vec(&raw).unwrap();
    let context =
        content_cohort::snapshot_context(&raw, Some(2), "declared_imported_report").unwrap();
    let mut r = request();
    r.selection.expected_snapshot_sha256 = Some(context.sha256.clone());
    let report = run(prepare(&raw, &context, r, Default::default()).unwrap())
        .await
        .unwrap();
    assert_eq!(report.engine_requests, 0);
    assert_eq!(report.accepted_count, None);
    assert_eq!(serde_json::to_vec(&raw).unwrap(), before);
    assert_eq!(
        report.saved_capture["snapshot"]["origin"],
        "declared_imported_report"
    );
}
#[test]
fn spelling_saved_selection_refuses_duplicate_unknown_generation_snapshot_and_dictionary() {
    let raw = saved_report();
    let context =
        content_cohort::snapshot_context(&raw, Some(2), "declared_imported_report").unwrap();
    let mut r = request();
    r.selection.expected_snapshot_sha256 = Some(context.sha256.clone());
    let mut bad = r.clone();
    bad.selection.urls.push(bad.selection.urls[0].clone());
    assert!(prepare(&raw, &context, bad, Default::default()).is_err());
    let mut bad = r.clone();
    bad.selection.expected_generation = Some(1);
    assert!(prepare(&raw, &context, bad, Default::default()).is_err());
    let mut bad = r.clone();
    bad.selection.expected_snapshot_sha256 = Some("b".repeat(64));
    assert!(prepare(&raw, &context, bad, Default::default()).is_err());
    let mut bad = r.clone();
    bad.expected_dictionary_sha256 = "c".repeat(64);
    assert!(prepare(&raw, &context, bad, Default::default()).is_err());
    let mut duplicated = raw.clone();
    duplicated.pages.push(duplicated.pages[0].clone());
    let duplicate_context =
        content_cohort::snapshot_context(&duplicated, Some(2), "declared_imported_report").unwrap();
    r.selection.expected_snapshot_sha256 = Some(duplicate_context.sha256.clone());
    assert!(prepare(&duplicated, &duplicate_context, r, Default::default()).is_err());
}
#[tokio::test]
async fn spelling_reporting_projection_excludes_owner_before_any_engine_check() {
    let mut raw = saved_report();
    raw.config.directive_reporting = Some(open_seo_spider_core::DirectiveReportingConfig {
        respect_canonical: true,
        ..Default::default()
    });
    raw.pages[0].canonical = vec!["https://saved.invalid/canonical".into()];
    let context =
        content_cohort::snapshot_context(&raw, Some(2), "declared_imported_report").unwrap();
    let mut r = request();
    r.selection.expected_snapshot_sha256 = Some(context.sha256.clone());
    r.selection.reporting_mode = Some(ReportingMode::Reported);
    r.config.enabled = true;
    r.config.acknowledge_local_text_transfer = true;
    r.config.endpoint = Some("http://127.0.0.1:1/v2/check".into());
    let report = run(prepare(&raw, &context, r.clone(), Default::default()).unwrap())
        .await
        .unwrap();
    assert!(report.documents.is_empty());
    assert_eq!(report.engine_requests, 0);
    assert_eq!(report.accepted_count, None);
    assert_eq!(
        report.saved_capture["excluded"][0]["reason"],
        "excluded_reporting"
    );
    r.selection.reporting_mode = Some(ReportingMode::Raw);
    let raw_view = run(prepare(&raw, &context, r, Default::default()).unwrap())
        .await
        .unwrap();
    assert_eq!(raw_view.documents.len(), 1);
    assert_eq!(raw_view.documents[0].status, "incomplete_capture");
}
#[test]
fn spelling_request_does_not_change_default_crawl_serialization_or_fingerprint() {
    let config = open_seo_spider_core::CrawlConfig::default();
    let before = serde_json::to_value(&config).unwrap();
    assert!(before.get("spelling").is_none());
    let _separate = spelling::Config::default();
    assert_eq!(serde_json::to_value(&config).unwrap(), before);
}
#[tokio::test]
async fn spelling_mock_timeout_and_truncated_wire_remain_explicit_incomplete_evidence() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v2/check", listener.local_addr().unwrap());
    let (release, done) = tokio::sync::oneshot::channel();
    let owner = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut b = [0; 4096];
        let _ = s.read(&mut b).await.unwrap();
        done.await.unwrap();
    });
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap();
    let evidence = exchange(
        &client,
        &endpoint,
        "saved text",
        "en-US",
        RESPONSE_TOTAL,
        tokio::time::Instant::now() + Duration::from_secs(90),
    )
    .await;
    release.send(()).unwrap();
    owner.await.unwrap();
    assert!(evidence.error.is_some());
    assert!(!evidence.body_complete);
    assert_eq!(
        evidence.body_sha256,
        spelling::digest(evidence.body.as_bytes())
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v2/check", listener.local_addr().unwrap());
    let owner = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut b = [0; 4096];
        let _ = s.read(&mut b).await.unwrap();
        s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{}").await.unwrap();
        s.shutdown().await.unwrap();
    });
    let evidence = exchange(
        &client,
        &endpoint,
        "saved text",
        "en-US",
        RESPONSE_TOTAL,
        tokio::time::Instant::now() + Duration::from_secs(90),
    )
    .await;
    owner.await.unwrap();
    assert!(evidence.error.is_some());
    assert!(!evidence.body_complete);
    assert_ne!(evidence.body_bytes, 100);
}
#[cfg(feature = "languagetool-integration")]
#[tokio::test]
async fn spelling_real_local_engine_reports_actual_grammar_separately_from_spelling() {
    let endpoint = std::env::var("OPEN_SEO_SPIDER_TEST_LANGUAGETOOL")
        .expect("Pinned local LanguageTool endpoint required");
    let version = std::env::var("OPEN_SEO_SPIDER_TEST_LANGUAGETOOL_VERSION")
        .expect("Actual software.version required");
    let mut r = request();
    r.config = spelling::Config {
        enabled: true,
        endpoint: Some(endpoint),
        acknowledge_local_text_transfer: true,
        expected_engine_version: Some(version),
        manual_language: Some("en-US".into()),
        ..Default::default()
    };
    let report = run(prepared(r, Some("😀 This are a sentence."), true))
        .await
        .unwrap();
    assert!(report.complete, "{:#?}", report.documents);
    assert!(
        report.documents[0]
            .checked
            .as_ref()
            .unwrap()
            .issues
            .iter()
            .any(|i| i.issue_type != "misspelling" && i.disposition == "accepted")
    );
    assert_eq!(report.website_requests, 0);
}
// V2 regression fixtures are simulated provider traffic only. No engine quality proof.
pub(crate) async fn zero_report_fixture() -> Report {
    let mut value = response();
    value["matches"] = json!([]);
    let (endpoint, owner) = mock(value, 200).await;
    let mut r = request();
    r.config.enabled = true;
    r.config.endpoint = Some(endpoint);
    r.config.acknowledge_local_text_transfer = true;
    let report = run(prepared(r, Some("😀 bad"), true)).await.unwrap();
    owner.await.unwrap();
    report
}
pub(crate) fn two_row_fixture(mut report: Report) -> Report {
    let mut second = report.documents[0].clone();
    second.id = "selection-1".into();
    second.url = "https://saved.invalid/two".into();
    second.source["requested_url"] = json!(second.url);
    second.source["ordinal"] = json!(1);
    second.provider.as_mut().unwrap().retained_limit_bytes =
        RESPONSE_TOTAL - report.documents[0].provider.as_ref().unwrap().body_bytes;
    report.request.selection.urls.push(second.url.clone());
    report.saved_capture["requested_urls"] = json!(report.request.selection.urls);
    report.saved_capture["selected_count"] = json!(2);
    report.saved_capture["included_count"] = json!(2);
    report.saved_capture["documents"]
        .as_array_mut()
        .unwrap()
        .push(json!({"analysis_document_id":second.id,"row":second.source}));
    report.documents.push(second);
    report.engine_requests = 2;
    report
}
pub(crate) fn disabled_success_fixture(mut report: Report) -> Report {
    report.request.config.enabled = false;
    report.request.config.acknowledge_local_text_transfer = false;
    report.config_sha256 = spelling::hash(&report.request.config).unwrap();
    report
}
pub(crate) fn empty_success_fixture(mut report: Report) -> Report {
    let d = &mut report.documents[0];
    d.text.clear();
    d.text_sha256 = spelling::digest(&[]);
    d.source["text_sha256"] = json!(d.text_sha256);
    report.saved_capture["documents"][0]["row"] = d.source.clone();
    let p = d.provider.as_ref().unwrap();
    d.checked = Some(
        spelling::parse_reply(
            p.body.as_bytes(),
            "",
            d.language.as_ref().unwrap(),
            &report.request.config,
            &report.dictionary,
        )
        .unwrap(),
    );
    report
}
pub(crate) async fn excluded_report_fixture() -> Report {
    let mut raw = saved_report();
    raw.config.directive_reporting = Some(open_seo_spider_core::DirectiveReportingConfig {
        respect_canonical: true,
        ..Default::default()
    });
    raw.pages[0].canonical = vec!["https://saved.invalid/canonical".into()];
    let context =
        content_cohort::snapshot_context(&raw, Some(2), "declared_imported_report").unwrap();
    let mut r = request();
    r.selection.expected_snapshot_sha256 = Some(context.sha256.clone());
    r.selection.reporting_mode = Some(ReportingMode::Reported);
    r.config.enabled = true;
    r.config.acknowledge_local_text_transfer = true;
    r.config.endpoint = Some("http://127.0.0.1:1/v2/check".into());
    run(prepare(&raw, &context, r, Default::default()).unwrap())
        .await
        .unwrap()
}
fn add_second_prepared(mut p: Prepared) -> Prepared {
    let input = &p.documents[0];
    let second = open_seo_spider_core::ContentTextInput {
        id: "selection-1".into(),
        source: input.source.clone(),
        text: input.text.clone(),
        language: input.language.clone(),
        complete: input.complete,
        truncation_reason: input.truncation_reason.clone(),
    };
    let url = "https://saved.invalid/two";
    p.request.selection.urls.push(url.into());
    p.envelope["requested_urls"] = json!(p.request.selection.urls);
    p.envelope["selected_count"] = json!(2);
    p.envelope["included_count"] = json!(2);
    let mut row = p.envelope["documents"][0]["row"].clone();
    row["requested_url"] = json!(url);
    row["ordinal"] = json!(1);
    p.envelope["documents"]
        .as_array_mut()
        .unwrap()
        .push(json!({"analysis_document_id":"selection-1","row":row}));
    p.documents.push(second);
    p
}
fn padded_reply(size: usize) -> Vec<u8> {
    let mut value = response();
    value["matches"] = json!([]);
    value["additive_padding"] = json!("");
    let overhead = serde_json::to_vec(&value).unwrap().len();
    value["additive_padding"] = json!("x".repeat(size - overhead));
    let bytes = serde_json::to_vec(&value).unwrap();
    assert_eq!(bytes.len(), size);
    bytes
}
async fn two_large_replies(chunked_second: bool) -> (String, tokio::task::JoinHandle<usize>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let owner = tokio::spawn(async move {
        let body = padded_reply(3 * 1024 * 1024);
        let mut requests = 0;
        for index in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            requests += 1;
            let mut raw = Vec::new();
            let mut buffer = [0; 4096];
            let end = loop {
                let n = socket.read(&mut buffer).await.unwrap();
                assert_ne!(n, 0);
                raw.extend_from_slice(&buffer[..n]);
                assert!(raw.len() <= 256 * 1024);
                if let Some(i) = raw.windows(4).position(|v| v == b"\r\n\r\n") {
                    break i + 4;
                }
            };
            let headers = String::from_utf8(raw[..end].to_vec()).unwrap();
            assert!(headers.starts_with("POST /v2/check HTTP/1.1"));
            let length = headers
                .lines()
                .find_map(|line| {
                    line.split_once(':')
                        .filter(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                        .map(|(_, v)| v.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            while raw.len() < end + length {
                let n = socket.read(&mut buffer).await.unwrap();
                assert_ne!(n, 0);
                raw.extend_from_slice(&buffer[..n]);
                assert!(raw.len() <= 256 * 1024);
            }
            let chunked = index == 1 && chunked_second;
            let framing = if chunked {
                "Transfer-Encoding: chunked".to_owned()
            } else {
                format!("Content-Length: {}", body.len())
            };
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{framing}\r\nConnection: close\r\n\r\n"
            );
            if socket.write_all(header.as_bytes()).await.is_err() {
                continue;
            }
            let mut delivered = true;
            for part in body.chunks(64 * 1024) {
                if chunked
                    && socket
                        .write_all(format!("{:x}\r\n", part.len()).as_bytes())
                        .await
                        .is_err()
                {
                    delivered = false;
                    break;
                }
                if socket.write_all(part).await.is_err() {
                    delivered = false;
                    break;
                }
                if chunked && socket.write_all(b"\r\n").await.is_err() {
                    delivered = false;
                    break;
                }
            }
            if chunked && delivered {
                let _ = socket.write_all(b"0\r\n\r\n").await;
            }
            // Client intentionally cancels an oversized declared/chunked body.
            let _ = socket.shutdown().await;
        }
        requests
    });
    (format!("http://{address}/v2/check"), owner)
}
async fn aggregate_budget_case(chunked: bool) -> Report {
    let (endpoint, owner) = two_large_replies(chunked).await;
    let mut r = request();
    r.config.enabled = true;
    r.config.acknowledge_local_text_transfer = true;
    r.config.endpoint = Some(endpoint);
    let report = run(add_second_prepared(prepared(r, Some("😀 bad"), true)))
        .await
        .unwrap();
    assert_eq!(owner.await.unwrap(), 2);
    assert_eq!(report.engine_requests, 2);
    assert!(!report.complete);
    assert_eq!(report.accepted_count, None);
    let first = report.documents[0].provider.as_ref().unwrap();
    let second = report.documents[1].provider.as_ref().unwrap();
    assert_eq!(first.body_bytes, 3 * 1024 * 1024);
    assert!(first.body_complete);
    assert_eq!(second.retained_limit_bytes, 1024 * 1024);
    assert!(
        second
            .error
            .as_ref()
            .unwrap()
            .contains("remaining retained response budget")
    );
    assert!(!second.body_complete);
    assert_eq!(report.documents[1].status, "provider_error");
    assert_eq!(report.documents[1].accepted_count, None);
    let sum = report
        .documents
        .iter()
        .filter_map(|d| d.provider.as_ref())
        .map(|p| p.body_bytes)
        .sum::<usize>();
    assert!(sum <= RESPONSE_TOTAL);
    if chunked {
        assert_eq!(sum, RESPONSE_TOTAL);
        assert_eq!(second.body_bytes, 1024 * 1024);
    } else {
        assert_eq!(sum, 3 * 1024 * 1024);
        assert_eq!(second.body_bytes, 0);
    }
    validate_report(&report, None).unwrap();
    report
}
#[tokio::test]
async fn spelling_aggregate_budget_rejects_declared_second_body_before_retaining_it() {
    let report = aggregate_budget_case(false).await;
    let mut tampered = report.clone();
    tampered.documents[1]
        .provider
        .as_mut()
        .unwrap()
        .retained_limit_bytes = RESPONSE_TOTAL;
    assert!(validate_report(&tampered, None).is_err());
}
#[tokio::test]
async fn spelling_aggregate_budget_caps_chunked_failure_prefix_before_append() {
    let report = aggregate_budget_case(true).await;
    let mut tampered = report.clone();
    let p = tampered.documents[1].provider.as_mut().unwrap();
    p.body.push('x');
    p.body_bytes = p.body.len();
    p.body_sha256 = spelling::digest(p.body.as_bytes());
    assert!(validate_report(&tampered, None).is_err());
}
#[tokio::test]
async fn spelling_exact_partition_rejects_omitted_duplicate_and_reordered_selected_rows() {
    let one = zero_report_fixture().await;
    let two = two_row_fixture(one.clone());
    validate_report(&two, None).unwrap();
    let mut omitted = one;
    omitted
        .request
        .selection
        .urls
        .push("https://saved.invalid/two".into());
    omitted.saved_capture["requested_urls"] = json!(omitted.request.selection.urls);
    omitted.saved_capture["selected_count"] = json!(2);
    assert!(
        validate_report(&omitted, None).is_err(),
        "A second selected URL cannot disappear from both arrays"
    );
    let mut duplicated = two.clone();
    duplicated.documents[1].url = duplicated.documents[0].url.clone();
    duplicated.documents[1].source["requested_url"] = json!(duplicated.documents[0].url);
    duplicated.saved_capture["documents"][1]["row"] = duplicated.documents[1].source.clone();
    assert!(validate_report(&duplicated, None).is_err());
    let mut reordered = two.clone();
    reordered.documents.swap(0, 1);
    reordered.saved_capture["documents"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert!(validate_report(&reordered, None).is_err());
    let mut ordinal = two.clone();
    ordinal.documents[1].source["ordinal"] = json!(0);
    ordinal.saved_capture["documents"][1]["row"] = ordinal.documents[1].source.clone();
    assert!(validate_report(&ordinal, None).is_err());
    let mut duplicate_request = two;
    duplicate_request.request.selection.urls[1] =
        duplicate_request.request.selection.urls[0].clone();
    duplicate_request.saved_capture["requested_urls"] =
        json!(duplicate_request.request.selection.urls);
    assert!(validate_report(&duplicate_request, None).is_err());
}
#[tokio::test]
async fn spelling_exact_partition_allows_reported_all_excluded_but_not_unknown_raw_or_missing_capture_filters()
 {
    let all = excluded_report_fixture().await;
    validate_report(&all, None).unwrap();
    assert!(!all.complete);
    assert_eq!(all.accepted_count, None);
    assert_eq!(all.engine_requests, 0);
    let mut unknown = all.clone();
    unknown.saved_capture["excluded"][0]["row"]["requested_url"] =
        json!("https://saved.invalid/unknown");
    assert!(validate_report(&unknown, None).is_err());
    let mut raw = all.clone();
    raw.request.selection.reporting_mode = Some(ReportingMode::Raw);
    raw.saved_capture["reporting"]["identity"]["mode"] = json!("raw");
    raw.saved_capture["reporting"]["identity_sha256"] =
        json!(spelling::hash(&raw.saved_capture["reporting"]["identity"]).unwrap());
    raw.saved_capture["reporting"]["eligible_page_observations"] =
        raw.saved_capture["reporting"]["captured_page_observations"].clone();
    assert!(validate_report(&raw, None).is_err());
    let mut legacy = all.clone();
    legacy.request.selection.reporting_mode = None;
    legacy
        .saved_capture
        .as_object_mut()
        .unwrap()
        .remove("reporting");
    assert!(validate_report(&legacy, None).is_err());
    let mut hidden_by_completeness = all;
    hidden_by_completeness.saved_capture["excluded"][0]["reason"] = json!("complete_sample_filter");
    assert!(validate_report(&hidden_by_completeness, None).is_err());
    let mut request = request();
    request.config.enabled = true;
    request.config.acknowledge_local_text_transfer = true;
    request.config.endpoint = Some("http://127.0.0.1:1/v2/check".into());
    let missing = run(prepared(request, None, false)).await.unwrap();
    validate_report(&missing, None).unwrap();
    assert_eq!(missing.documents.len(), 1);
    assert_eq!(missing.documents[0].status, "incomplete_capture");
    assert_eq!(missing.accepted_count, None);
}
#[tokio::test]
async fn spelling_saved_success_cannot_be_forged_for_disabled_or_empty_text() {
    let actual = zero_report_fixture().await;
    validate_report(&actual, None).unwrap();
    assert_eq!(actual.accepted_count, Some(0));
    assert!(validate_report(&disabled_success_fixture(actual.clone()), None).is_err());
    assert!(validate_report(&empty_success_fixture(actual), None).is_err());
    let disabled = run(prepared(request(), Some("saved text"), true))
        .await
        .unwrap();
    validate_report(&disabled, None).unwrap();
    assert_eq!(disabled.accepted_count, None);
    let mut r = request();
    r.config.enabled = true;
    r.config.acknowledge_local_text_transfer = true;
    r.config.endpoint = Some("http://127.0.0.1:1/v2/check".into());
    let empty = run(prepared(r, Some(""), true)).await.unwrap();
    validate_report(&empty, None).unwrap();
    assert_eq!(empty.documents[0].status, "empty_saved_text");
    assert_eq!(empty.accepted_count, None);
    assert_eq!(empty.engine_requests, 0);
}
