//! Authored read-only MCP controls; these tests do not contact a website.
use super::*;
#[test]
fn descriptor_is_read_only_strict_and_additional_to_existing_tools() {
    let definition = definition();
    assert_eq!(definition["name"], "spider_compare_saved");
    assert_eq!(definition["annotations"]["readOnlyHint"], true);
    assert_eq!(definition["annotations"]["openWorldHint"], false);
    assert_eq!(definition["inputSchema"]["additionalProperties"], false);
    let names: Vec<_> = inventory()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(names.len(), 25);
    assert!(names.contains(&"spider_sitemaps".into()));
    assert!(names.contains(&"spider_compare".into()));
    assert_eq!(names.last().unwrap(), "spider_compare_saved");
}
#[test]
fn structured_api_collision_failure_is_retained_in_mcp_result() {
    let bytes = serde_json::to_vec(&json!({"website_requests":0,"error":{
        "code":"ambiguous_snapshot_identity","message":"collision","collisions":[{
            "side":"before","comparison_key":"https://fixture.test/","occurrences":[
                {"ordinal":0,"original_requested_url":"https://fixture.test/"},
                {"ordinal":1,"original_requested_url":"https://fixture.test/"}
            ]
        }]
    }}))
    .unwrap();
    let result = decode(reqwest::StatusCode::BAD_REQUEST, &bytes, false);
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["structuredContent"]["error"]["collisions"][0]["occurrences"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(result["structuredContent"]["website_requests"], 0);
}
#[test]
fn nonstructured_api_error_is_explicit_adapter_failure() {
    let result = decode(
        reqwest::StatusCode::CONFLICT,
        br#"{"error":"legacy text"}"#,
        false,
    );
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["structuredContent"]["error"]["code"],
        "invalid_comparison_api_error"
    );
}
fn bundle_api_bytes(include: bool) -> Vec<u8> {
    use open_seo_spider_core::{CrawlReport, compare_saved_crawls};
    let before = CrawlReport {
        config: Default::default(),
        pages: vec![],
        issues: vec![],
        summary: Default::default(),
        warnings: vec!["saved \\\" escaped text".repeat(300)],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
        sitemap_source_measurements: None,
    };
    let after = before.clone();
    let comparison =
        compare_saved_crawls(&before, &after, &Default::default(), None, None).unwrap();
    let response = if include {
        SavedComparisonResponse::with_original_reports(comparison, before, after).unwrap()
    } else {
        SavedComparisonResponse::new(comparison).unwrap()
    };
    serde_json::to_vec(&response).unwrap()
}
#[test]
fn opt_in_mcp_returns_complete_validated_reports_and_retains_read_only_descriptor() {
    let bytes = bundle_api_bytes(true);
    let result = decode(reqwest::StatusCode::OK, &bytes, true);
    assert_eq!(result["isError"], false);
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result["structuredContent"], value);
    let text: Value = serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text, value);
    assert_eq!(
        definition()["inputSchema"]["properties"]["include_original_reports"]["type"],
        "boolean"
    );
    assert_eq!(inventory().len(), 26);
}
#[test]
fn bundle_missing_unrequested_or_tampered_report_is_an_explicit_mcp_error() {
    for (include, bytes) in [
        (true, bundle_api_bytes(false)),
        (false, bundle_api_bytes(true)),
    ] {
        let result = decode(reqwest::StatusCode::OK, &bytes, include);
        assert_eq!(result["isError"], true);
        assert_eq!(
            result["structuredContent"]["error"]["code"],
            "invalid_comparison_api_response"
        );
    }
    let mut value: Value = serde_json::from_slice(&bundle_api_bytes(true)).unwrap();
    value["original_reports"]["after"]["report"]["warnings"] = json!(["altered response"]);
    let result = decode(
        reqwest::StatusCode::OK,
        &serde_json::to_vec(&value).unwrap(),
        true,
    );
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["structuredContent"]["error"]["code"],
        "original_report_binding_mismatch"
    );
}
#[test]
fn whole_rpc_budget_counts_actual_id_text_duplication_and_json_escaping() {
    let bytes = bundle_api_bytes(true);
    let result = decode(reqwest::StatusCode::OK, &bytes, true);
    let id = json!("request \" ID \\ with unicode 日".repeat(100));
    let complete = json!({"jsonrpc":"2.0","id":id,"result":result});
    let exact = serde_json::to_vec(&complete).unwrap().len();
    assert!(exact > bytes.len());
    assert_eq!(
        complete_response_with_limit(id.clone(), complete["result"].clone(), true, exact),
        complete
    );
    let rejected =
        complete_response_with_limit(id.clone(), complete["result"].clone(), true, exact - 1);
    assert_eq!(rejected["id"], id);
    assert_eq!(rejected["result"]["isError"], true);
    assert_eq!(
        rejected["result"]["structuredContent"]["error"]["code"],
        "original_report_bundle_byte_budget"
    );
    assert!(
        rejected["result"]["structuredContent"]
            .get("original_reports")
            .is_none()
    );
    assert!(serde_json::to_vec(&rejected).unwrap().len() < exact);
}
#[test]
fn api_body_can_fit_while_opt_in_complete_rpc_is_refused_without_a_first_n_success() {
    let bytes = bundle_api_bytes(true);
    let result = decode(reqwest::StatusCode::OK, &bytes, true);
    let rejected = complete_response_with_limit(json!(1), result, true, bytes.len());
    assert_eq!(rejected["result"]["isError"], true);
    assert_eq!(
        rejected["result"]["structuredContent"]["error"]["code"],
        "original_report_bundle_byte_budget"
    );
    assert!(serde_json::to_vec(&rejected).unwrap().len() <= bytes.len());
}
#[test]
fn unrepresentable_request_id_returns_bounded_null_id_protocol_failure() {
    let result = decode(reqwest::StatusCode::OK, &bundle_api_bytes(true), true);
    let rejected = complete_response_with_limit(json!("id".repeat(10_000)), result, true, 4096);
    assert_eq!(rejected["id"], Value::Null);
    assert_eq!(rejected["error"]["code"], -32600);
    assert!(serde_json::to_vec(&rejected).unwrap().len() <= 4096);
}
#[test]
fn default_rpc_envelope_is_exactly_preserved_without_opt_in() {
    let result = decode(reqwest::StatusCode::OK, &bundle_api_bytes(false), false);
    let expected = json!({"jsonrpc":"2.0","id":7,"result":result});
    assert_eq!(
        complete_response(json!(7), expected["result"].clone(), false),
        expected
    );
    assert!(
        expected["result"]["structuredContent"]
            .get("original_reports")
            .is_none()
    );
}
#[tokio::test]
async fn opt_in_wrong_boolean_and_extra_argument_fail_before_loopback_io() {
    let bridge = Bridge::new("http://127.0.0.1:1").unwrap();
    for flag in [json!(1), json!("true"), Value::Null] {
        let result = bridge
            .dispatch(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"spider_compare_saved","arguments":{
                "before":"00000000-0000-0000-0000-000000000001",
                "after":"00000000-0000-0000-0000-000000000002", "include_original_reports":flag
            }}}))
            .await
            .unwrap();
        assert_eq!(result["result"]["isError"], true);
        assert_eq!(
            result["result"]["structuredContent"]["error"]["code"],
            "invalid_comparison_request"
        );
    }
    let result = bridge
        .dispatch(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"spider_compare_saved","arguments":{
            "before":"00000000-0000-0000-0000-000000000001",
            "after":"00000000-0000-0000-0000-000000000002", "include_original_reports":true,
            "unknown":true
        }}}))
        .await
        .unwrap();
    assert_eq!(result["error"]["code"], -32602);
}

// Append to crates/app/src/mcp/saved_comparison_tests.rs on the integrated
// baseline before enabling serde_json float_roundtrip. Authored, NOT RUN here.
#[test]
fn mcp_bundle_native_f64_fact_preserves_original_bits_and_core_hash() {
    use open_seo_spider_core::{
        CrawlReport, ImageResourceMetadata, PageRecord, ResourceAnalysisStatus,
        compare_saved_crawls,
    };
    use sha2::{Digest, Sha256};
    // Valid fractional CSS-pixel width used by native SVG image metadata.
    // Default serde_json parsing can turn this into 51.24817837550541.
    let width = 51.248178375505404_f64;
    let before = CrawlReport {
        config: Default::default(),
        pages: vec![PageRecord {
            url: "https://fixture.test/preserved-width.svg".into(),
            final_url: "https://fixture.test/preserved-width.svg".into(),
            status: Some(200),
            final_status: Some(200),
            content_type: "image/svg+xml".into(),
            image_metadata: Some(ImageResourceMetadata {
                engine: "quick-xml-0.42/bounded-svg-lengths-v1".into(),
                status: ResourceAnalysisStatus::Complete,
                format: Some("svg".into()),
                width: Some(width),
                height: Some(2.0),
                dimension_unit: Some("css_px".into()),
                body_complete: true,
                ..Default::default()
            }),
            custom_javascript_results: vec![json!({"retained_decimal":width})],
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
    };
    let after = before.clone();
    let typed_before_bytes = serde_json::to_vec(&before).unwrap();
    let comparison =
        compare_saved_crawls(&before, &after, &Default::default(), None, None).unwrap();
    let expected_sha = comparison.before_snapshot.typed_report_sha256.clone();
    let response =
        SavedComparisonResponse::with_original_reports(comparison, before, after).unwrap();
    let api_bytes = serde_json::to_vec(&response).unwrap();
    let result = decode(reqwest::StatusCode::OK, &api_bytes, true);
    assert_eq!(
        result["isError"], false,
        "Legitimate native decimal report was rejected: {result}"
    );
    let report = &result["structuredContent"]["original_reports"]["before"]["report"];
    assert_eq!(
        report["pages"][0]["image_metadata"]["width"]
            .as_f64()
            .unwrap()
            .to_bits(),
        width.to_bits()
    );
    assert_eq!(
        report["pages"][0]["custom_javascript_results"][0]["retained_decimal"]
            .as_f64()
            .unwrap()
            .to_bits(),
        width.to_bits()
    );
    assert_eq!(
        result["structuredContent"]["original_reports"]["before"]["typed_report_sha256"],
        expected_sha
    );
    // MCP Value can reorder keys. Check the declared typed Rust profile by
    // typed reserialization, never by hashing a textual MCP object slice.
    let typed: CrawlReport = serde_json::from_value(report.clone()).unwrap();
    let roundtrip = serde_json::to_vec(&typed).unwrap();
    assert_eq!(roundtrip, typed_before_bytes);
    assert_eq!(format!("{:x}", Sha256::digest(roundtrip)), expected_sha);
}

// Append to crates/app/src/mcp/saved_comparison_tests.rs independently of the
// f64 regression and before opting into bounded deeper bundle decoding.
// Authored, NOT RUN by this author.
#[test]
fn mcp_bundle_wrappers_do_not_reject_an_accepted_standalone_report_depth() {
    use open_seo_spider_core::{CrawlReport, PageRecord, compare_saved_crawls};
    let mut nested = json!(0);
    for _ in 0..123 {
        nested = Value::Array(vec![nested]);
    }
    let before = CrawlReport {
        config: Default::default(),
        pages: vec![PageRecord {
            url: "https://fixture.test/accepted-depth".into(),
            final_url: "https://fixture.test/accepted-depth".into(),
            status: Some(200),
            final_status: Some(200),
            content_type: "text/html".into(),
            rendered_viewport: Some(nested),
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
    };
    let standalone = serde_json::to_vec(&before).unwrap();
    let accepted: CrawlReport = serde_json::from_slice(&standalone)
        .expect("Original standalone report must satisfy the unchanged default parser limit");
    assert_eq!(serde_json::to_vec(&accepted).unwrap(), standalone);
    let after = before.clone();
    let comparison =
        compare_saved_crawls(&before, &after, &Default::default(), None, None).unwrap();
    let expected_sha = comparison.before_snapshot.typed_report_sha256.clone();
    let response =
        SavedComparisonResponse::with_original_reports(comparison, before, after).unwrap();
    let api_bytes = serde_json::to_vec(&response).unwrap();
    let result = decode(reqwest::StatusCode::OK, &api_bytes, true);
    assert_eq!(
        result["isError"], false,
        "Bundle wrappers rejected an accepted typed report: {result}"
    );
    assert_eq!(
        result["structuredContent"]["original_reports"]["before"]["typed_report_sha256"],
        expected_sha
    );
    let mut retained = &result["structuredContent"]["original_reports"]["before"]["report"]["pages"]
        [0]["rendered_viewport"];
    for _ in 0..123 {
        let array = retained
            .as_array()
            .expect("Every original array occurrence remains intact");
        assert_eq!(array.len(), 1);
        retained = &array[0];
    }
    assert_eq!(*retained, json!(0));
}

// Append to crates/app/src/mcp/saved_comparison_tests.rs after the helper
// insertion. Authored controls, NOT RUN by this author.
#[test]
fn bundle_depth_preflight_ignores_container_glyphs_inside_quoted_values() {
    let value = json!({"text":"[{}]".repeat(400)});
    let bytes = serde_json::to_vec(&value).unwrap();
    preflight_bundle_json_depth(&bytes).unwrap();
    assert_eq!(parse_saved_api_json::<Value>(&bytes, true).unwrap(), value);
}
#[test]
fn bundle_depth_preflight_tracks_escaped_quotes_and_backslashes() {
    let bytes = br#"{"text":"literal \"quoted [ ] { }\" and \\ tail"}"#;
    preflight_bundle_json_depth(bytes).unwrap();
    let parsed = parse_saved_api_json::<Value>(bytes, true).unwrap();
    assert_eq!(parsed, serde_json::from_slice::<Value>(bytes).unwrap());
}
#[test]
fn bundle_depth_preflight_refuses_unclosed_strings_and_wrong_or_unclosed_containers() {
    for bytes in [
        b"[0".as_slice(),
        b"{\"a\":0",
        b"\"unterminated",
        b"\"ending\\",
        b"[}",
        b"}",
    ] {
        assert!(
            preflight_bundle_json_depth(bytes).is_err(),
            "Invalid preflight unexpectedly accepted {bytes:?}"
        );
    }
}
#[test]
fn opt_in_bundle_depth_256_fits_and_257_is_refused_before_deserialization() {
    let at_limit = format!("{}0{}", "[".repeat(256), "]".repeat(256));
    preflight_bundle_json_depth(at_limit.as_bytes()).unwrap();
    parse_saved_api_json::<Value>(at_limit.as_bytes(), true).unwrap();
    let beyond = format!("{}0{}", "[".repeat(257), "]".repeat(257));
    let error = parse_saved_api_json::<Value>(beyond.as_bytes(), true).unwrap_err();
    assert!(error.to_string().contains("nesting exceeds 256"));
}
#[test]
fn default_api_parser_retains_its_existing_recursion_limit() {
    let bytes = format!("{}0{}", "[".repeat(129), "]".repeat(129));
    assert!(parse_saved_api_json::<Value>(bytes.as_bytes(), false).is_err());
    parse_saved_api_json::<Value>(bytes.as_bytes(), true).unwrap();
}
#[test]
fn opt_in_parser_still_validates_json_grammar_and_requires_document_end() {
    for bytes in [b"[0] [1]".as_slice(), b"{\"a\" 0}"] {
        preflight_bundle_json_depth(bytes).unwrap();
        assert!(parse_saved_api_json::<Value>(bytes, true).is_err());
    }
}
// This independent, iterative counter measures container depth outside strings.
// The production lexical preflight separately verifies matching delimiters and
// serde verifies full JSON grammar. Do not deserialize recursive data on the
// deliberately small caller stack just to measure its depth.
fn worker_regression_json_depth(bytes: &[u8]) -> usize {
    let mut depth = 0usize;
    let mut maximum = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for &byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'{' | b'[' => {
                depth += 1;
                maximum = maximum.max(depth);
            }
            b'}' | b']' => {
                depth = depth.checked_sub(1).expect("Unexpected closing container");
            }
            _ => {}
        }
    }
    assert!(!quoted && !escaped);
    assert_eq!(depth, 0);
    maximum
}

// Invoke only inside spawn_blocking: construction, Core hashing, typed clone,
// API serialization and all typed report/recursive Value destruction belong to
// the worker, just as recursive decode/validation belongs there in production.
fn worker_regression_bundle_api_bytes(array_count: usize) -> (Vec<u8>, String) {
    use open_seo_spider_core::{CrawlReport, PageRecord, compare_saved_crawls};
    let mut nested = json!(9_007_199_254_740_993u64);
    for _ in 0..array_count {
        nested = Value::Array(vec![nested]);
    }
    let before = CrawlReport {
        config: Default::default(),
        pages: vec![PageRecord {
            url: "https://fixture.test/worker-maximum-depth".into(),
            final_url: "https://fixture.test/worker-maximum-depth".into(),
            status: Some(200),
            final_status: Some(200),
            content_type: "text/html".into(),
            rendered_viewport: Some(nested),
            ..Default::default()
        }],
        issues: vec![],
        summary: Default::default(),
        warnings: vec!["Quoted delimiters [ { } ] with \\\" and \\\\ stay text".into()],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
        sitemap_source_measurements: None,
    };
    let after = before.clone();
    let comparison =
        compare_saved_crawls(&before, &after, &Default::default(), None, None).unwrap();
    let expected_sha = comparison.before_snapshot.typed_report_sha256.clone();
    let response =
        SavedComparisonResponse::with_original_reports(comparison, before, after).unwrap();
    let bytes = serde_json::to_vec(&response).unwrap();
    drop(response);
    (bytes, expected_sha)
}

#[test]
fn mcp_bundle_worker_pipeline_accepts_api_depth256_and_refuses257_on_small_caller_stack() {
    // Windows stdio main stack is materially smaller than the normal Rust test
    // thread. An explicit 1 MiB caller catches accidental recursive CPU work or
    // recursive drops moved back out of the blocking closures.
    std::thread::Builder::new()
        .name("mcp-bundle-small-caller".into())
        .stack_size(1024 * 1024)
        .spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                // API wrappers contribute six containers above viewport. Its
                // scalar leaf contributes none: 250 arrays gives exact depth256.
                let ((at_limit, expected_sha), (beyond, _)) = tokio::task::spawn_blocking(|| {
                    (
                        worker_regression_bundle_api_bytes(250),
                        worker_regression_bundle_api_bytes(251),
                    )
                })
                .await
                .unwrap();
                assert_eq!(worker_regression_json_depth(&at_limit), 256);
                assert_eq!(worker_regression_json_depth(&beyond), 257);
                preflight_bundle_json_depth(&at_limit).unwrap();
                assert!(preflight_bundle_json_depth(&beyond).is_err());

                // Mirrors saved_comparison::call's opted body decode exactly:
                // parse + typed binding validation + both MCP result forms and
                // all their intermediate recursive drops happen on the worker.
                let result = tokio::task::spawn_blocking(move || {
                    let result = decode(reqwest::StatusCode::OK, &at_limit, true);
                    assert_eq!(result["isError"], false);
                    assert_eq!(
                        result["structuredContent"]["original_reports"]["before"]
                            ["typed_report_sha256"]
                            .as_str(),
                        Some(expected_sha.as_str())
                    );
                    result
                })
                .await
                .unwrap();
                let request_id = json!("full depth \\\" escaped ID \\\\ 日");
                // Calls the production opted full-RPC constructor/budget worker.
                let response = complete_response_on_worker(request_id, result, true).await;

                // Mirrors stdio's move closure. No recursive Value is copied or
                // formatted on the caller. Explicit drop occurs before bytes are
                // returned. Only Vec<u8> survives to the 1 MiB caller afterward.
                let bytes = tokio::task::spawn_blocking(move || {
                    assert_eq!(response["jsonrpc"], "2.0");
                    assert_eq!(response["result"]["isError"], false);
                    assert_eq!(
                        response["id"].as_str(),
                        Some("full depth \\\" escaped ID \\\\ 日")
                    );
                    for side in ["before", "after"] {
                        let mut node = &response["result"]["structuredContent"]["original_reports"]
                            [side]["report"]["pages"][0]["rendered_viewport"];
                        for _ in 0..250 {
                            let array = node.as_array().expect("Original array retained");
                            assert_eq!(array.len(), 1);
                            node = &array[0];
                        }
                        assert_eq!(node.as_u64(), Some(9_007_199_254_740_993));
                    }
                    // The exact text representation remains API depth256. Its
                    // own parse/validation and destruction also stay on worker.
                    let text = response["result"]["content"][0]["text"].as_str().unwrap();
                    assert_eq!(worker_regression_json_depth(text.as_bytes()), 256);
                    let text_value = parse_saved_api_json::<Value>(text.as_bytes(), true).unwrap();
                    assert_eq!(text_value["website_requests"], 0);
                    assert!(text.contains("9007199254740993"));
                    drop(text_value);

                    let bytes = serde_json::to_vec(&response).unwrap();
                    // JSON-RPC root and MCP structuredContent parent add two
                    // containers. The 256 preflight belongs to the API body;
                    // this intentionally complete output has depth258.
                    assert_eq!(worker_regression_json_depth(&bytes), 258);
                    assert!(bytes.len() <= MAX_RESPONSE_BYTES);
                    check_serialized_budget(
                        &response,
                        bytes.len(),
                        "original_report_bundle_byte_budget",
                    )
                    .unwrap();
                    drop(response);
                    bytes
                })
                .await
                .unwrap();
                assert!(!bytes.is_empty());

                // A valid typed API document one container beyond the declared
                // bound still yields a structured refusal, never partial reports.
                let refused = tokio::task::spawn_blocking(move || {
                    decode(reqwest::StatusCode::OK, &beyond, true)
                })
                .await
                .unwrap();
                let refused = complete_response_on_worker(json!(2), refused, true).await;
                let refusal_bytes = tokio::task::spawn_blocking(move || {
                    assert_eq!(refused["result"]["isError"], true);
                    assert_eq!(
                        refused["result"]["structuredContent"]["error"]["code"],
                        "invalid_comparison_api_response"
                    );
                    assert!(
                        refused["result"]["structuredContent"]["error"]["message"]
                            .as_str()
                            .unwrap()
                            .contains("nesting exceeds 256")
                    );
                    assert!(
                        refused["result"]["structuredContent"]
                            .get("original_reports")
                            .is_none()
                    );
                    let bytes = serde_json::to_vec(&refused).unwrap();
                    drop(refused);
                    bytes
                })
                .await
                .unwrap();
                assert!(!refusal_bytes.is_empty());
            });
        })
        .unwrap()
        .join()
        .unwrap();
}
