use super::*;
#[tokio::test]
async fn spelling_mcp_binds_exact_request_snapshot_mode_dictionary_and_crawl() {
    let request = crate::spelling_app::tests::request();
    let mut report = crate::spelling_app::run(crate::spelling_app::tests::prepared(
        request.clone(),
        None,
        false,
    ))
    .await
    .unwrap();
    let id = uuid::Uuid::new_v4();
    report.crawl_id = Some(id);
    let value = json!({"report":report,"crawl_id":id,"persisted":true});
    let raw = serde_json::to_vec(&value).unwrap();
    let mut args = Args {
        id,
        action: Action::Analyze,
        run_id: None,
        request: Some(request),
    };
    assert!(decode(&raw, &args).is_ok());
    args.id = uuid::Uuid::new_v4();
    assert!(decode(&raw, &args).is_err());
    args.id = id;
    args.request.as_mut().unwrap().selection.reporting_mode =
        Some(crate::content_cohort::ReportingMode::Reported);
    assert!(decode(&raw, &args).is_err());
    args.request = Some(crate::spelling_app::tests::request());
    args.request
        .as_mut()
        .unwrap()
        .selection
        .expected_snapshot_sha256 = Some("b".repeat(64));
    assert!(decode(&raw, &args).is_err());
    args.request = Some(crate::spelling_app::tests::request());
    args.request.as_mut().unwrap().expected_dictionary_sha256 = "c".repeat(64);
    assert!(decode(&raw, &args).is_err());
}
#[tokio::test]
async fn spelling_mcp_rejects_spoofed_request_with_missing_projection_and_fabricated_zero() {
    let mut request = crate::spelling_app::tests::request();
    request.selection.reporting_mode = Some(crate::content_cohort::ReportingMode::Reported);
    let mut p = crate::spelling_app::tests::prepared(request.clone(), None, false);
    // The simulated saved envelope intentionally lacks the explicitly requested view.
    assert!(crate::spelling_app::run(p).await.is_err());
    request.selection.reporting_mode = None;
    p = crate::spelling_app::tests::prepared(request.clone(), None, false);
    let mut report = crate::spelling_app::run(p).await.unwrap();
    report.crawl_id = Some(uuid::Uuid::nil());
    report.accepted_count = Some(0);
    let raw = serde_json::to_vec(&json!({"report":report,"crawl_id":uuid::Uuid::nil()})).unwrap();
    let args = Args {
        id: uuid::Uuid::nil(),
        action: Action::Analyze,
        run_id: None,
        request: Some(request),
    };
    assert!(decode(&raw, &args).is_err());
}
#[test]
fn spelling_mcp_schema_has_explicit_mutating_local_analysis_and_no_engine_launch() {
    let v = definition();
    assert_eq!(v["name"], NAME);
    assert_eq!(v["inputSchema"]["additionalProperties"], false);
    assert_eq!(v["annotations"]["readOnlyHint"], false);
    assert_eq!(v["annotations"]["openWorldHint"], false);
    assert!(
        serde_json::from_value::<Args>(
            json!({"id":uuid::Uuid::nil(),"action":"analyze","engine_path":"java"})
        )
        .is_err()
    );
}
fn reject_saved_report(mut report: crate::spelling_app::Report) {
    report.crawl_id = Some(uuid::Uuid::nil());
    let request = report.request.clone();
    let raw =
        serde_json::to_vec(&json!({"report":report,"crawl_id":uuid::Uuid::nil(),"persisted":true}))
            .unwrap();
    let args = Args {
        id: uuid::Uuid::nil(),
        action: Action::Analyze,
        run_id: None,
        request: Some(request),
    };
    assert!(decode(&raw, &args).is_err());
}
#[tokio::test]
async fn spelling_mcp_rejects_omitted_duplicate_unknown_and_raw_excluded_selected_work() {
    use crate::spelling_app::tests::{
        excluded_report_fixture, two_row_fixture, zero_report_fixture,
    };
    let one = zero_report_fixture().await;
    let mut omitted = one.clone();
    omitted
        .request
        .selection
        .urls
        .push("https://saved.invalid/two".into());
    omitted.saved_capture["requested_urls"] = json!(omitted.request.selection.urls);
    omitted.saved_capture["selected_count"] = json!(2);
    reject_saved_report(omitted);
    let mut duplicate = two_row_fixture(one);
    duplicate.documents[1].url = duplicate.documents[0].url.clone();
    duplicate.documents[1].source["requested_url"] = json!(duplicate.documents[0].url);
    duplicate.saved_capture["documents"][1]["row"] = duplicate.documents[1].source.clone();
    reject_saved_report(duplicate);
    let all = excluded_report_fixture().await;
    let mut unknown = all.clone();
    unknown.saved_capture["excluded"][0]["row"]["requested_url"] =
        json!("https://saved.invalid/unknown");
    reject_saved_report(unknown);
    let mut raw = all.clone();
    raw.request.selection.reporting_mode = Some(crate::content_cohort::ReportingMode::Raw);
    raw.saved_capture["reporting"]["identity"]["mode"] = json!("raw");
    raw.saved_capture["reporting"]["identity_sha256"] = json!(
        open_seo_spider_core::spelling::hash(&raw.saved_capture["reporting"]["identity"]).unwrap()
    );
    raw.saved_capture["reporting"]["eligible_page_observations"] =
        raw.saved_capture["reporting"]["captured_page_observations"].clone();
    reject_saved_report(raw);
    let id = uuid::Uuid::nil();
    let mut valid = all;
    valid.crawl_id = Some(id);
    let request = valid.request.clone();
    let bytes =
        serde_json::to_vec(&json!({"report":valid,"crawl_id":id,"persisted":true})).unwrap();
    assert!(
        decode(
            &bytes,
            &Args {
                id,
                action: Action::Analyze,
                run_id: None,
                request: Some(request)
            }
        )
        .is_ok()
    );
}
#[tokio::test]
async fn spelling_mcp_rejects_disabled_and_empty_checked_zero_but_accepts_unknown_states() {
    use crate::spelling_app::tests::{
        disabled_success_fixture, empty_success_fixture, zero_report_fixture,
    };
    let actual = zero_report_fixture().await;
    reject_saved_report(disabled_success_fixture(actual.clone()));
    reject_saved_report(empty_success_fixture(actual));
    for empty in [false, true] {
        let mut request = crate::spelling_app::tests::request();
        if empty {
            request.config.enabled = true;
            request.config.acknowledge_local_text_transfer = true;
            request.config.endpoint = Some("http://127.0.0.1:1/v2/check".into());
        }
        let mut report = crate::spelling_app::run(crate::spelling_app::tests::prepared(
            request.clone(),
            Some(if empty { "" } else { "saved text" }),
            true,
        ))
        .await
        .unwrap();
        let id = uuid::Uuid::nil();
        report.crawl_id = Some(id);
        let raw =
            serde_json::to_vec(&json!({"report":report,"crawl_id":id,"persisted":true})).unwrap();
        assert!(
            decode(
                &raw,
                &Args {
                    id,
                    action: Action::Analyze,
                    run_id: None,
                    request: Some(request)
                }
            )
            .is_ok()
        );
    }
}
