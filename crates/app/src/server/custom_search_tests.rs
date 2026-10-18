use super::*;

fn search_profile() -> Value {
    json!({"version":1,"rules":[{"name":"owned rule","method":"regex","pattern":"(?i)owned","regex_flags":"","sources":["raw_html","rendered_selected_text"]}],"limits":{"max_saved_matches":0}})
}

#[test]
fn custom_search_api_configuration_is_strict_and_preserves_optional_absence() {
    let profile = search_profile();
    let config =
        config_from_input(json!({"seed_url":"https://example.test/","custom_search":profile}))
            .unwrap();
    let saved = serde_json::to_value(&config).unwrap();
    assert_eq!(saved["custom_search"]["rules"], profile["rules"]);
    assert_eq!(saved["custom_search"]["limits"]["max_saved_matches"], 0);
    assert_eq!(
        serde_json::to_value(config_from_input(saved.clone()).unwrap()).unwrap(),
        saved
    );
    for invalid in [
        json!([]),
        json!({"rules":profile["rules"]}),
        json!({"version":2,"rules":profile["rules"]}),
        json!({"version":1,"rules":[]}),
        json!({"version":1,"rules":[{"name":"invalid","method":"regex","pattern":"(?=unsupported)","sources":["raw_html"]}]}),
        json!({"version":1,"rules":[{"name":"invalid","method":"literal","pattern":"x","regex_flags":"i","sources":["raw_html"]}]}),
        json!({"version":1,"rules":profile["rules"],"endpoint":"http://127.0.0.1:1"}),
        json!({"version":1,"rules":profile["rules"],"limits":{"max_saved_matches":257}}),
    ] {
        assert!(
            config_from_input(json!({"seed_url":"https://example.test/","custom_search":invalid}))
                .is_err()
        );
    }
    let absent = config_from_input(json!({"seed_url":"https://example.test/"})).unwrap();
    let null = config_from_input(json!({"seed_url":"https://example.test/","custom_search":null}))
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&null).unwrap()
    );
    assert!(
        serde_json::to_value(absent)
            .unwrap()
            .get("custom_search")
            .is_none()
    );
}

fn saved_result(status: &str, count: Value) -> Value {
    json!({"engine":"open-seo-spider/custom-search-v1","rule_name":"=RULE()","method":"literal","pattern_sha256":"a".repeat(64),"config_sha256":"b".repeat(64),"source":"raw_html","status":status,"count":count,"matches_observed":0,"evidence_complete":status=="complete","matches":[],"input_utf8_bytes":if status=="complete"{json!(0)}else{Value::Null},"searched_utf8_bytes":0,"input_sha256":if status=="complete"{json!("c".repeat(64))}else{Value::Null},"document_url":if status=="complete"{json!("https://example.test/")}else{Value::Null},"observed_generation":if status=="complete"{json!(7)}else{Value::Null},"source_revision":if status=="complete"{json!("d".repeat(64))}else{Value::Null},"reasons":if status=="complete"{json!([])}else{json!(["missing_source"])}})
}

#[tokio::test]
async fn custom_search_saved_http_export_preserves_unknowns_exact_zero_and_recovered_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let state = AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Default::default(),
        data_dir: Arc::new(temp.path().to_path_buf()),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle: Default::default(),
    };
    let report: CrawlReport = serde_json::from_value(json!({"config":CrawlConfig::default(),"pages":[
        {"url":"https://example.test/zero","custom_search":[saved_result("complete",json!(0)),saved_result("unavailable",Value::Null)]},
        {"url":"https://example.test/legacy"}
    ],"issues":[],"summary":open_seo_spider_core::CrawlSummary::default(),"warnings":[]})).unwrap();
    let original = serde_json::to_value(&report.pages[0].custom_search).unwrap();
    let router = Router::new()
        .route("/api/crawls/import", post(import_report))
        .route("/api/crawls/{id}/project.json", get(export_project))
        .route("/api/crawls/{id}/custom-search.csv", get(export_search_csv))
        .route("/api/capabilities", get(capabilities_with_mcp_files))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let (stop, closed) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = closed.await;
            })
            .await
            .unwrap();
    });
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let created: Value = client
        .post(format!("{origin}/api/crawls/import"))
        .json(&report)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    let disk = temp.path().join(format!("{id}.json"));
    let before = tokio::fs::read(&disk).await.unwrap();
    let saved: CrawlReport = client
        .get(format!("{origin}/api/crawls/{id}/project.json"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&saved.pages[0].custom_search).unwrap(),
        original
    );
    assert!(saved.pages[1].custom_search.is_none());
    let response = client
        .get(format!("{origin}/api/crawls/{id}/custom-search.csv"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let expected_hash = response.headers()["x-content-sha256"]
        .to_str()
        .unwrap()
        .to_owned();
    let expected_length = response.content_length().unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        response.headers()[header::CONTENT_DISPOSITION],
        "attachment; filename=\"custom-search.csv\""
    );
    let bytes = response.bytes().await.unwrap();
    assert_eq!(bytes.len() as u64, expected_length);
    assert_eq!(
        open_seo_spider_core::api_enrichment::api_sha(&bytes),
        expected_hash
    );
    assert_eq!(
        bytes.as_ref(),
        export_custom_search_csv(&saved).unwrap().as_bytes()
    );
    let csv = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(csv.contains("'=RULE()"));
    assert!(csv.contains(",complete,0,0,true,0,[],0,0,"));
    assert!(csv.contains(",unavailable,,0,false,0,[],"));
    assert!(csv.contains("missing_saved_observation"));
    state.jobs.write().await.clear();
    load_reports(&state).await.unwrap();
    let recovered = client
        .get(format!("{origin}/api/crawls/{id}/custom-search.csv"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(recovered, bytes);
    assert_eq!(tokio::fs::read(disk).await.unwrap(), before);
    let capabilities: Value = client
        .get(format!("{origin}/api/capabilities"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        capabilities["custom_search"]["missing_source"],
        "unavailable_count_null"
    );
    assert!(
        capabilities["exports"]
            .as_array()
            .unwrap()
            .contains(&json!("custom_search_csv"))
    );
    assert_eq!(
        capabilities["custom_search"]["configuration_schema"],
        custom_search_config_schema()
    );
    let mut oversized = saved.clone();
    oversized.pages[0].custom_search.as_mut().unwrap()[0]
        .reasons
        .push("x".repeat(open_seo_spider_core::CUSTOM_SEARCH_MAX_OUTPUT_BYTES));
    let oversized_id = Uuid::new_v4();
    register_report(&state, oversized_id, oversized).await;
    let refusal = client
        .get(format!(
            "{origin}/api/crawls/{oversized_id}/custom-search.csv"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(refusal.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert!(refusal.headers().get(header::CONTENT_DISPOSITION).is_none());
    let refusal: Value = refusal.json().await.unwrap();
    assert!(refusal["error"].as_str().unwrap().contains("byte budget"));
    stop.send(()).unwrap();
    server.await.unwrap();
    lifecycle::close_admission(&state);
    lifecycle::drain(&state).await.unwrap();
}
