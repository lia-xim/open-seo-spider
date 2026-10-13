//! Shared request, export and local-file integrity controls.
use super::*;
use open_seo_spider_core::{ComparisonCollision, ComparisonIdentityOccurrence, PageRecord};

fn request() -> Value {
    json!({"before":"00000000-0000-0000-0000-000000000001",
        "after":"00000000-0000-0000-0000-000000000002"})
}
#[test]
fn strict_saved_request_rejects_unknown_and_invalid_nested_arguments() {
    for value in [
        json!({"before":"invalid","after":"00000000-0000-0000-0000-000000000002"}),
        {
            let mut value = request();
            value["extra"] = json!(true);
            value
        },
        {
            let mut value = request();
            value["config"] = json!({"unknown":true});
            value
        },
        {
            let mut value = request();
            value["config"] = json!({"limits":{"unknown":1}});
            value
        },
        {
            let mut value = request();
            value["expected_before_generation"] = json!(-1);
            value
        },
        {
            let mut value = request();
            value["expected_after_generation"] = json!(1.5);
            value
        },
        {
            let mut value = request();
            value["expected_after_generation"] = json!("1");
            value
        },
    ] {
        assert!(serde_json::from_value::<SavedComparisonRequest>(value).is_err());
    }
}
#[test]
fn explicit_zero_generation_and_optional_null_are_retained_without_invention() {
    let mut value = request();
    value["expected_before_generation"] = json!(0);
    value["config"] = Value::Null;
    let parsed: SavedComparisonRequest = serde_json::from_value(value).unwrap();
    assert_eq!(parsed.expected_before_generation, Some(0));
    assert_eq!(parsed.expected_after_generation, None);
    assert!(parsed.config.is_none());
    let binding = file_binding();
    assert!(binding.generation.is_none());
    assert!(binding.snapshot_sha256.is_none());
}
#[test]
fn structured_error_retains_every_collision_occurrence() {
    let original = ComparisonError {
        code: "ambiguous_snapshot_identity".into(),
        message: "collision".into(),
        collisions: vec![ComparisonCollision {
            side: ComparisonSide::Before,
            comparison_key: "https://fixture.test/".into(),
            occurrences: vec![
                ComparisonIdentityOccurrence {
                    ordinal: 0,
                    original_requested_url: "https://fixture.test/".into(),
                },
                ComparisonIdentityOccurrence {
                    ordinal: 1,
                    original_requested_url: "https://fixture.test/".into(),
                },
            ],
        }],
    };
    let envelope = SavedComparisonFailure::from(original.clone());
    let decoded: SavedComparisonFailure =
        serde_json::from_slice(&serde_json::to_vec(&envelope).unwrap()).unwrap();
    assert_eq!(decoded.error, original);
    assert_eq!(decoded.website_requests, 0);
    assert!(decoded.context.is_none());
}
#[test]
fn csv_escapes_formulas_quotes_commas_newlines_and_preserves_original_values() {
    let fields = ["=1+1", "\t @SUM(1,2)", "\u{feff}-1+2", "ä,\"日\"\n", ""].map(String::from);
    let original = fields.clone();
    let mut bytes = Vec::new();
    csv_record(&mut bytes, &fields).unwrap();
    assert_eq!(fields, original);
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "\"'=1+1\",\"'\t @SUM(1,2)\",\"'\u{feff}-1+2\",\"ä,\"\"日\"\"\n\",\"\"\r\n"
    );
}
#[test]
fn exact_output_budget_allows_boundary_and_refuses_next_byte() {
    let mut writer = ByteBudget {
        writer: Vec::new(),
        remaining: 3,
    };
    writer.write_all(b"abc").unwrap();
    assert!(writer.write_all(b"d").is_err());
    assert_eq!(writer.writer, b"abc");
}

fn bundle_report() -> CrawlReport {
    let html = "<html lang='de'><head><title>Saved original</title></head><body><main>Retained text</main><a href='/target'>Link</a></body></html>";
    let mut page = PageRecord {
        url: "https://fixture.test/page".into(),
        final_url: "https://fixture.test/page".into(),
        status: Some(200),
        final_status: Some(200),
        content_type: "text/html; charset=UTF-8".into(),
        fetched_at: "2026-10-13T01:00:00Z".into(),
        response_ms: 9_007_199_254_740_993,
        size_bytes: html.len(),
        screenshot_base64: Some("saved-local-artifact".into()),
        custom_javascript_results: vec![json!({"unprojected": "exact retained value"})],
        headers: std::collections::BTreeMap::from([("content-type".into(), "text/html".into())]),
        ..Default::default()
    };
    let seed = reqwest::Url::parse(&page.url).unwrap();
    open_seo_spider_core::extract_html(&mut page, html, &[seed], false);
    CrawlReport {
        config: Default::default(),
        pages: vec![page],
        issues: vec![],
        summary: Default::default(),
        warnings: vec!["saved report warning".into()],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
    }
}
fn bundle_comparison(before: &CrawlReport, after: &CrawlReport) -> SavedComparisonResult {
    compare_saved_crawls(before, after, &Default::default(), None, None).unwrap()
}
#[test]
fn original_option_is_strict_boolean_and_default_stays_false() {
    let absent: SavedComparisonRequest = serde_json::from_value(request()).unwrap();
    assert!(!absent.include_original_reports);
    for flag in [json!(false), json!(true)] {
        let mut value = request();
        value["include_original_reports"] = flag.clone();
        let parsed: SavedComparisonRequest = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.include_original_reports, flag.as_bool().unwrap());
    }
    for flag in [Value::Null, json!(1), json!("true"), json!({})] {
        let mut value = request();
        value["include_original_reports"] = flag;
        assert!(serde_json::from_value::<SavedComparisonRequest>(value).is_err());
    }
    let schema = request_schema();
    assert_eq!(
        schema["properties"]["include_original_reports"]["type"],
        "boolean"
    );
    assert_eq!(schema["additionalProperties"], false);
}
#[test]
fn default_response_is_byte_identical_to_prior_two_field_contract() {
    #[derive(Serialize)]
    struct Prior<'a> {
        comparison: &'a SavedComparisonResult,
        website_requests: usize,
    }
    let report = bundle_report();
    let comparison = bundle_comparison(&report, &report);
    let expected = serde_json::to_vec(&Prior {
        comparison: &comparison,
        website_requests: 0,
    })
    .unwrap();
    let response = SavedComparisonResponse::new(comparison).unwrap();
    assert_eq!(serde_json::to_vec(&response).unwrap(), expected);
    assert!(response.original_reports.is_none());
}
#[test]
fn default_request_serialization_is_byte_identical_for_existing_mcp_normalization() {
    #[derive(Serialize)]
    struct PriorRequest<'a> {
        before: Uuid,
        after: Uuid,
        config: &'a Option<SavedComparisonConfig>,
        expected_before_generation: Option<u64>,
        expected_after_generation: Option<u64>,
    }
    let parsed: SavedComparisonRequest = serde_json::from_value(request()).unwrap();
    let prior = PriorRequest {
        before: parsed.before,
        after: parsed.after,
        config: &parsed.config,
        expected_before_generation: parsed.expected_before_generation,
        expected_after_generation: parsed.expected_after_generation,
    };
    assert_eq!(
        serde_json::to_vec(&parsed).unwrap(),
        serde_json::to_vec(&prior).unwrap()
    );
    let mut explicit = parsed;
    explicit.include_original_reports = true;
    assert_eq!(
        serde_json::to_value(explicit).unwrap()["include_original_reports"],
        true
    );
}
#[test]
fn whole_reports_retain_all_loaded_fields_and_independent_hash_byte_binding() {
    use sha2::{Digest, Sha256};
    let before = bundle_report();
    let mut after = before.clone();
    after.pages[0].titles[0] = "New title".into();
    let before_bytes = serde_json::to_vec(&before).unwrap();
    let after_bytes = serde_json::to_vec(&after).unwrap();
    let expected_before = serde_json::to_value(&before).unwrap();
    let expected_after = serde_json::to_value(&after).unwrap();
    let comparison = bundle_comparison(&before, &after);
    let response =
        SavedComparisonResponse::with_original_reports(comparison, before, after).unwrap();
    response.validate_original_reports().unwrap();
    let value = serde_json::to_value(&response).unwrap();
    assert_eq!(
        value["original_reports"]["before"]["report"],
        expected_before
    );
    assert_eq!(value["original_reports"]["after"]["report"], expected_after);
    let bundle = response.original_reports.as_ref().unwrap();
    assert_eq!(
        bundle.before.typed_report_sha256,
        format!("{:x}", Sha256::digest(&before_bytes))
    );
    assert_eq!(
        bundle.after.typed_report_sha256,
        format!("{:x}", Sha256::digest(&after_bytes))
    );
    assert_eq!(bundle.before.typed_report_json_bytes, before_bytes.len());
    assert_eq!(bundle.after.typed_report_json_bytes, after_bytes.len());
    assert!(
        !response
            .comparison
            .before_snapshot
            .caller_binding_attested_by_core
    );
    assert_eq!(response.website_requests, 0);
    assert!(
        String::from_utf8(serde_json::to_vec(&response).unwrap())
            .unwrap()
            .contains("9007199254740993")
    );
}
#[test]
fn mutated_report_or_descriptor_cannot_be_bound_as_the_compared_original() {
    let before = bundle_report();
    let after = before.clone();
    let mut changed = after.clone();
    changed.warnings.push("later job mutation".into());
    let comparison = bundle_comparison(&before, &after);
    assert_eq!(
        SavedComparisonResponse::with_original_reports(comparison.clone(), before.clone(), changed)
            .unwrap_err()
            .error
            .code,
        "original_report_binding_mismatch"
    );
    let mut wrong_profile = comparison;
    wrong_profile.before_snapshot.typed_report_sha_profile = "raw-file-bytes".into();
    assert_eq!(
        SavedComparisonResponse::with_original_reports(wrong_profile, before, after)
            .unwrap_err()
            .error
            .code,
        "original_report_binding_mismatch"
    );
}
#[test]
fn bundle_wrapper_metadata_and_tampered_typed_report_are_validated() {
    let report = bundle_report();
    let comparison = bundle_comparison(&report, &report);
    let response =
        SavedComparisonResponse::with_original_reports(comparison, report.clone(), report).unwrap();
    let base = serde_json::to_value(&response).unwrap();
    for path in ["schema_version", "binding", "representation"] {
        let mut value = base.clone();
        value["original_reports"][path] = if path == "schema_version" {
            json!(2)
        } else {
            json!("unsupported")
        };
        let decoded: SavedComparisonResponse = serde_json::from_value(value).unwrap();
        assert_eq!(
            decoded.validate_original_reports().unwrap_err().error.code,
            "original_report_binding_mismatch"
        );
    }
    let mut value = base.clone();
    value["original_reports"]["before"]["extra"] = json!(true);
    assert!(serde_json::from_value::<SavedComparisonResponse>(value).is_err());
    let mut value = base;
    value["original_reports"]["after"]["report"]["warnings"] = json!(["tampered"]);
    let decoded: SavedComparisonResponse = serde_json::from_value(value).unwrap();
    assert_eq!(
        decoded.validate_original_reports().unwrap_err().error.code,
        "original_report_binding_mismatch"
    );
}
#[test]
fn oversized_structured_failure_becomes_explicit_budget_error_and_small_errors_keep_all_details() {
    let small = failure("original_report_binding_mismatch", "exact failure");
    let expected = serde_json::to_vec(&small).unwrap();
    assert_eq!(
        serde_json::to_vec(&bounded_bundle_failure(small)).unwrap(),
        expected
    );
    let large = SavedComparisonFailure::from(ComparisonError {
        code: "ambiguous_snapshot_identity".into(),
        message: "all collisions".into(),
        collisions: vec![ComparisonCollision {
            side: ComparisonSide::Before,
            comparison_key: "https://fixture.test/".into(),
            occurrences: (0..20)
                .map(|ordinal| ComparisonIdentityOccurrence {
                    ordinal,
                    original_requested_url: format!("https://fixture.test/{}", "x".repeat(100)),
                })
                .collect(),
        }],
    });
    let rejected = bounded_bundle_failure_with_limit(large, 512);
    assert_eq!(rejected.error.code, "original_report_bundle_byte_budget");
    assert!(rejected.error.collisions.is_empty());
    assert!(serde_json::to_vec(&rejected).unwrap().len() <= 512);
    assert_eq!(rejected.website_requests, 0);
}
#[test]
fn whole_envelope_fits_exact_boundary_and_refuses_one_byte_less_without_partial_success() {
    let report = bundle_report();
    let comparison = bundle_comparison(&report, &report);
    let response = SavedComparisonResponse::with_original_reports(
        comparison.clone(),
        report.clone(),
        report.clone(),
    )
    .unwrap();
    let exact = serde_json::to_vec(&response).unwrap().len();
    assert!(
        SavedComparisonResponse::with_original_reports_limit(
            comparison.clone(),
            report.clone(),
            report.clone(),
            exact
        )
        .is_ok()
    );
    let error = SavedComparisonResponse::with_original_reports_limit(
        comparison,
        report.clone(),
        report,
        exact - 1,
    )
    .unwrap_err();
    assert_eq!(error.error.code, "original_report_bundle_byte_budget");
}
#[test]
fn timestamp_only_does_not_change_compared_page_but_complete_report_hash_changes() {
    let before = bundle_report();
    let mut after = before.clone();
    after.pages[0].fetched_at = "2026-10-13T02:00:00Z".into();
    after.summary.finished_at = "2026-10-13T02:00:01Z".into();
    let comparison = bundle_comparison(&before, &after);
    assert_eq!(comparison.counts.unchanged, 1);
    assert_eq!(comparison.counts.changed, 0);
    assert_ne!(
        comparison.before_snapshot.typed_report_sha256,
        comparison.after_snapshot.typed_report_sha256
    );
    SavedComparisonResponse::with_original_reports(comparison, before, after).unwrap();
}
#[test]
fn imported_typed_report_is_not_promoted_to_raw_file_fidelity_or_known_capture() {
    use sha2::{Digest, Sha256};
    let report = bundle_report();
    let mut legacy = serde_json::to_value(&report).unwrap();
    legacy["unknown_legacy_field"] = json!("not part of loaded typed model");
    let raw_file = serde_json::to_vec_pretty(&legacy).unwrap();
    let typed: CrawlReport = serde_json::from_slice(&raw_file).unwrap();
    let binding = file_binding();
    let comparison = compare_saved_crawls(
        &typed,
        &typed,
        &Default::default(),
        Some(&binding),
        Some(&binding),
    )
    .unwrap();
    assert_ne!(
        comparison.before_snapshot.typed_report_sha256,
        format!("{:x}", Sha256::digest(&raw_file))
    );
    assert!(
        comparison.pages[0]
            .fields
            .iter()
            .any(|field| field.before.state == open_seo_spider_core::ComparisonFactState::Missing)
    );
    let response =
        SavedComparisonResponse::with_original_reports(comparison, typed.clone(), typed).unwrap();
    assert!(
        serde_json::to_value(&response).unwrap()["original_reports"]["before"]["report"]
            .get("unknown_legacy_field")
            .is_none()
    );
    let binding = response
        .comparison
        .before_snapshot
        .caller_binding
        .as_ref()
        .unwrap();
    assert!(binding.generation.is_none());
    assert!(binding.snapshot_sha256.is_none());
    assert!(
        !response
            .comparison
            .before_snapshot
            .caller_binding_attested_by_core
    );
}
#[test]
fn mappings_preserve_original_report_urls_and_do_not_project_the_complete_bundle() {
    let mut before = bundle_report();
    before.pages[0].url = "https://staging.test/page".into();
    before.pages[0].final_url = before.pages[0].url.clone();
    let after = bundle_report();
    let config = SavedComparisonConfig {
        include_selected_text: false,
        mappings: vec![open_seo_spider_core::ComparisonUrlMapping {
            side: ComparisonSide::Before,
            from_origin: "https://staging.test".into(),
            from_path_prefix: "/".into(),
            to_origin: "https://fixture.test".into(),
            to_path_prefix: "/".into(),
        }],
        ..Default::default()
    };
    let comparison = compare_saved_crawls(&before, &after, &config, None, None).unwrap();
    assert_eq!(comparison.pages.len(), 1);
    assert_eq!(
        comparison.pages[0].comparison_key,
        "https://fixture.test/page"
    );
    let expected = serde_json::to_value(&before).unwrap();
    let response =
        SavedComparisonResponse::with_original_reports(comparison, before, after).unwrap();
    assert_eq!(
        serde_json::to_value(&response).unwrap()["original_reports"]["before"]["report"],
        expected
    );
}
#[test]
fn incomplete_capture_is_retained_even_when_selected_text_projection_is_disabled() {
    use open_seo_spider_core::{
        ContentCaptureConfig, ContentCaptureDecodeStatus, ContentCaptureInput,
        ContentCaptureSourceKind, PAGE_CONTENT_CAPTURE_ENGINE, PageContentCapture,
        PageContentCaptureStatus,
    };
    use sha2::{Digest, Sha256};
    let mut report = bundle_report();
    let html = "<html lang='de'><body><main>Saved partial text prefix</main></body></html>";
    let config = ContentCaptureConfig::default();
    let seed = reqwest::Url::parse(&report.pages[0].url).unwrap();
    open_seo_spider_core::extract_html(&mut report.pages[0], html, &[seed], false);
    report.pages[0].size_bytes = html.len();
    let compiled = config.compile().unwrap();
    let result = compiled
        .capture(&ContentCaptureInput {
            source_kind: ContentCaptureSourceKind::RawHttpHtml,
            html: Some(html.into()),
            source_complete: false,
            decode_status: ContentCaptureDecodeStatus::Exact,
            source_truncation_reason: Some("fixture retained prefix".into()),
            unavailable_reason: None,
            config: config.clone(),
        })
        .unwrap();
    let mut capture = PageContentCapture {
        engine: PAGE_CONTENT_CAPTURE_ENGINE.into(),
        source_kind: ContentCaptureSourceKind::RawHttpHtml,
        requested_url: report.pages[0].url.clone(),
        source_document_url: Some(report.pages[0].final_url.clone()),
        fetched_at: report.pages[0].fetched_at.clone(),
        rendered_at: None,
        observed_generation: Some(9_007_199_254_740_993),
        source_revision: String::new(),
        config_sha256: compiled.config_sha256().into(),
        source_html_utf8_sha256: result.html_utf8_sha256.clone(),
        source_html_utf8_bytes: result.html_utf8_bytes,
        encoded_body_sha256: Some(format!("{:x}", Sha256::digest(html.as_bytes()))),
        encoded_body_bytes: Some(html.len()),
        encoded_body_complete: Some(false),
        decode_encoding: Some("UTF-8".into()),
        status: PageContentCaptureStatus::Incomplete,
        result: Some(result),
        failure: None,
    };
    // Fixture uses the published binding tuple; public validation below must
    // accept it before this authored test treats its retained facts as valid.
    let revision = serde_json::to_vec(&(
        (
            capture.engine.as_str(),
            capture.source_kind,
            capture.requested_url.as_str(),
            &capture.source_document_url,
            capture.fetched_at.as_str(),
            &capture.rendered_at,
            capture.observed_generation,
            capture.config_sha256.as_str(),
        ),
        (
            &capture.source_html_utf8_sha256,
            capture.source_html_utf8_bytes,
            &capture.encoded_body_sha256,
            capture.encoded_body_bytes,
            capture.encoded_body_complete,
            &capture.decode_encoding,
            capture.status,
            &capture.result,
            &capture.failure,
        ),
    ))
    .unwrap();
    capture.source_revision = format!("{:x}", Sha256::digest(revision));
    report.pages[0].truncated = true;
    report.pages[0].content_captures = Some(vec![capture]);
    report.config.content_capture = Some(config);
    open_seo_spider_core::validate_page_content_captures(&report.pages[0]).unwrap();
    let config = SavedComparisonConfig {
        include_selected_text: false,
        ..Default::default()
    };
    let comparison = compare_saved_crawls(&report, &report, &config, None, None).unwrap();
    let selected = comparison.pages[0]
        .fields
        .iter()
        .find(|field| field.field == "text_sources.raw_http_html.selected_text")
        .unwrap();
    assert_eq!(
        selected.before.state,
        open_seo_spider_core::ComparisonFactState::Partial
    );
    assert!(!selected.comparable_complete_values);
    assert_eq!(selected.before.value.as_ref().unwrap()["text"], Value::Null);
    let expected = serde_json::to_value(&report).unwrap();
    let response =
        SavedComparisonResponse::with_original_reports(comparison, report.clone(), report).unwrap();
    let value = serde_json::to_value(&response).unwrap();
    assert_eq!(value["original_reports"]["before"]["report"], expected);
    assert_eq!(
        value["original_reports"]["before"]["report"]["pages"][0]["content_captures"][0]["status"],
        "incomplete"
    );
    assert_eq!(
        value["original_reports"]["before"]["report"]["pages"][0]["content_captures"][0]["result"]
            ["text"],
        "Saved partial text prefix"
    );
}
struct OwnedFixture(PathBuf);
impl OwnedFixture {
    async fn new() -> Self {
        let path = std::env::temp_dir().join(format!("open-seo-spider-b11-{}", Uuid::new_v4()));
        tokio::fs::create_dir(&path).await.unwrap();
        Self(tokio::fs::canonicalize(path).await.unwrap())
    }
}
impl Drop for OwnedFixture {
    fn drop(&mut self) {
        if let Ok(temp) = std::fs::canonicalize(std::env::temp_dir())
            && self.0.starts_with(&temp)
            && self
                .0
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("open-seo-spider-b11-"))
        {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
#[tokio::test]
async fn existing_destination_is_refused_and_original_bytes_survive() {
    let fixture = OwnedFixture::new().await;
    let existing = fixture.0.join("existing.json");
    tokio::fs::write(&existing, b"keep exact bytes")
        .await
        .unwrap();
    let error = destination(&existing).await.unwrap_err();
    assert_eq!(error.error.code, "output_destination_exists");
    assert_eq!(
        tokio::fs::read(&existing).await.unwrap(),
        b"keep exact bytes"
    );
}
#[tokio::test]
async fn bounded_reader_refuses_plus_one_without_unbounded_read() {
    let fixture = OwnedFixture::new().await;
    let path = fixture.0.join("input.json");
    tokio::fs::write(&path, b"1234").await.unwrap();
    assert_eq!(read_bounded(&path, 4).await.unwrap(), b"1234");
    assert_eq!(
        read_bounded(&path, 3).await.unwrap_err().error.code,
        "saved_input_byte_budget"
    );
}
#[tokio::test]
async fn cli_json_csv_roundtrip_retains_unknown_states_and_declared_file_origin() {
    let fixture = OwnedFixture::new().await;
    let before = fixture.0.join("before.json");
    let after = fixture.0.join("after.json");
    let output = fixture.0.join("delta.json");
    let csv = fixture.0.join("delta.csv");
    let make = |title: &str| {
        let mut page = PageRecord {
            url: "https://fixture.test/page".into(),
            final_url: "https://fixture.test/page".into(),
            status: Some(200),
            final_status: Some(200),
            content_type: "text/html".into(),
            ..Default::default()
        };
        let seed = reqwest::Url::parse(&page.url).unwrap();
        open_seo_spider_core::extract_html(
            &mut page,
            &format!("<title>{title}</title><body>body</body>"),
            &[seed],
            false,
        );
        CrawlReport {
            config: Default::default(),
            pages: vec![page],
            issues: vec![],
            summary: Default::default(),
            warnings: vec![],
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
            sitemap_catalog: None,
        }
    };
    tokio::fs::write(&before, serde_json::to_vec(&make("Old")).unwrap())
        .await
        .unwrap();
    tokio::fs::write(&after, serde_json::to_vec(&make("New")).unwrap())
        .await
        .unwrap();
    run_cli(
        before,
        after,
        None,
        Some(output.clone()),
        Some(csv.clone()),
        false,
    )
    .await
    .unwrap();
    let parsed: SavedComparisonResponse =
        serde_json::from_slice(&tokio::fs::read(output).await.unwrap()).unwrap();
    assert_eq!(parsed.comparison.counts.changed, 1);
    assert_eq!(parsed.website_requests, 0);
    let binding = parsed.comparison.before_snapshot.caller_binding.unwrap();
    assert_eq!(
        binding.origin.as_deref(),
        Some("cli_declared_saved_json_file")
    );
    assert!(binding.generation.is_none());
    assert!(
        !parsed
            .comparison
            .before_snapshot
            .caller_binding_attested_by_core
    );
    let text = String::from_utf8(tokio::fs::read(csv).await.unwrap()).unwrap();
    assert!(text.contains("before_value_json"));
    assert!(text.contains("\"unknown\""));
}

#[tokio::test]
async fn cli_opt_in_exports_complete_typed_reports_and_compared_fact_csv() {
    let fixture = OwnedFixture::new().await;
    let before = fixture.0.join("bundle-before.json");
    let after = fixture.0.join("bundle-after.json");
    let output = fixture.0.join("bundle.json");
    let csv = fixture.0.join("facts.csv");
    let first = bundle_report();
    let mut second = first.clone();
    second.pages[0].titles[0] = "Changed saved title".into();
    tokio::fs::write(&before, serde_json::to_vec_pretty(&first).unwrap())
        .await
        .unwrap();
    tokio::fs::write(&after, serde_json::to_vec(&second).unwrap())
        .await
        .unwrap();
    run_cli(
        before,
        after,
        None,
        Some(output.clone()),
        Some(csv.clone()),
        true,
    )
    .await
    .unwrap();
    let bytes = tokio::fs::read(output).await.unwrap();
    let response: SavedComparisonResponse = serde_json::from_slice(&bytes).unwrap();
    response.validate_original_reports().unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        value["original_reports"]["before"]["report"],
        serde_json::to_value(&first).unwrap()
    );
    assert_eq!(
        value["original_reports"]["after"]["report"],
        serde_json::to_value(&second).unwrap()
    );
    assert!(
        String::from_utf8(bytes)
            .unwrap()
            .contains("9007199254740993")
    );
    let binding = response
        .comparison
        .before_snapshot
        .caller_binding
        .as_ref()
        .unwrap();
    assert_eq!(
        binding.origin.as_deref(),
        Some("cli_declared_saved_json_file")
    );
    assert!(binding.generation.is_none());
    assert!(binding.snapshot_sha256.is_none());
    let text = String::from_utf8(tokio::fs::read(csv).await.unwrap()).unwrap();
    assert!(text.contains("before_value_json"));
    assert!(text.contains("Changed saved title"));
    assert!(!text.contains("saved-local-artifact"));
}
#[tokio::test]
async fn cli_bundle_overflow_happens_before_either_destination_is_reserved() {
    let fixture = OwnedFixture::new().await;
    let before = fixture.0.join("before.json");
    let after = fixture.0.join("after.json");
    let output = fixture.0.join("overflow.json");
    let csv = fixture.0.join("overflow.csv");
    let report = bundle_report();
    let binding = file_binding();
    let comparison = compare_saved_crawls(
        &report,
        &report,
        &Default::default(),
        Some(&binding),
        Some(&binding),
    )
    .unwrap();
    let response =
        SavedComparisonResponse::with_original_reports(comparison, report.clone(), report.clone())
            .unwrap();
    let exact = serde_json::to_vec(&response).unwrap().len();
    for path in [&before, &after] {
        tokio::fs::write(path, serde_json::to_vec(&report).unwrap())
            .await
            .unwrap();
    }
    let error = run_cli_with_options(
        before,
        after,
        None,
        Some(output.clone()),
        Some(csv.clone()),
        SavedCliOptions {
            include_original_reports: true,
            response_maximum: exact - 1,
            csv_maximum: MAX_CSV_BYTES,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.error.code, "original_report_bundle_byte_budget");
    assert!(!output.exists());
    assert!(!csv.exists());
}
#[tokio::test]
async fn cli_csv_budget_overflow_also_happens_before_json_or_csv_reservation() {
    let fixture = OwnedFixture::new().await;
    let before = fixture.0.join("before.json");
    let after = fixture.0.join("after.json");
    let output = fixture.0.join("unreserved.json");
    let csv = fixture.0.join("unreserved.csv");
    let report = bundle_report();
    let binding = file_binding();
    let comparison = compare_saved_crawls(
        &report,
        &report,
        &Default::default(),
        Some(&binding),
        Some(&binding),
    )
    .unwrap();
    let mut csv_bytes = Vec::new();
    write_csv(&comparison, &mut csv_bytes).unwrap();
    for path in [&before, &after] {
        tokio::fs::write(path, serde_json::to_vec(&report).unwrap())
            .await
            .unwrap();
    }
    let error = run_cli_with_options(
        before,
        after,
        None,
        Some(output.clone()),
        Some(csv.clone()),
        SavedCliOptions {
            include_original_reports: true,
            response_maximum: MAX_RESPONSE_BYTES,
            csv_maximum: csv_bytes.len() - 1,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.error.code, "comparison_csv_byte_budget");
    assert!(!output.exists());
    assert!(!csv.exists());
}
#[tokio::test]
async fn cli_opt_in_refuses_existing_destination_without_changing_its_bytes() {
    let fixture = OwnedFixture::new().await;
    let existing = fixture.0.join("keep.json");
    tokio::fs::write(&existing, b"existing exact bytes")
        .await
        .unwrap();
    let csv = fixture.0.join("unreserved.csv");
    let error = run_cli(
        fixture.0.join("unused-before.json"),
        fixture.0.join("unused-after.json"),
        None,
        Some(existing.clone()),
        Some(csv.clone()),
        true,
    )
    .await
    .unwrap_err();
    assert_eq!(error.error.code, "output_destination_exists");
    assert_eq!(
        tokio::fs::read(existing).await.unwrap(),
        b"existing exact bytes"
    );
    assert!(!csv.exists());
}
