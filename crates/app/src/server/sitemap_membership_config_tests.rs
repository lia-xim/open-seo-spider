// Included in actual create-config adapter tests. AUTHORED / NOT_RUN.
#[tokio::test]
async fn create_dto_preserves_explicit_membership_capacity_through_saved_config_and_mcp_dispatch_shape() {
    let input = json!({"seed_urls":["https://capacity.example/"],"max_body_bytes":1024,
        "sitemap_source_processing":{"max_post_http_entity_bytes":67108864,"max_decoded_xml_bytes":67108864,"max_xml_events":2000000},
        "sitemap_membership_limits":{"max_catalog_json_bytes":134217728,"max_row_json_bytes":262144,"max_rows":1000000}});
    let config = config_from_input(input).unwrap();
    assert_eq!(config.max_body_bytes,1024);
    assert_eq!(config.sitemap_membership_ledger_limits().unwrap().max_catalog_json_bytes,134217728);
    assert_eq!(config.sitemap_source_body_limit().unwrap(),67108864);
    let saved = serde_json::to_value(&config).unwrap();
    assert_eq!(saved["sitemap_membership_limits"]["max_catalog_json_bytes"],134217728);
    assert_eq!(serde_json::to_value(config_from_input(saved.clone()).unwrap()).unwrap(),saved);
    // MCP spider_start_crawl forwards its complete config object to this same adapter.
    let listed = crate::mcp::Bridge::new("http://127.0.0.1:0").unwrap()
        .dispatch(json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}})).await.unwrap();
    let tools = listed["result"]["tools"].as_array().unwrap();
    let start = tools.iter().find(|tool| tool["name"]=="spider_start_crawl").unwrap();
    let schema = &start["inputSchema"]["properties"]["config"]["properties"]["sitemap_membership_limits"];
    assert_eq!(schema["type"],json!(["object","null"]));
    assert_eq!(schema["additionalProperties"],false);
    assert_eq!(schema["properties"]["max_catalog_json_bytes"]["maximum"],134217728);
    assert_eq!(schema["properties"]["max_catalog_json_bytes"]["default"],33554432);
    assert_eq!(schema["properties"]["max_row_json_bytes"]["maximum"],1048576);
    assert_eq!(schema["properties"]["max_rows"]["maximum"],1000000);
}

#[test]
fn create_dto_membership_capacity_absent_null_and_source_profile_are_independent() {
    let input = json!({"seed_urls":["https://capacity.example/"]});
    let absent = config_from_input(input.clone()).unwrap();
    let mut null = input;null["sitemap_membership_limits"]=serde_json::Value::Null;
    assert_eq!(serde_json::to_value(config_from_input(null).unwrap()).unwrap(),serde_json::to_value(&absent).unwrap());
    assert!(serde_json::to_value(&absent).unwrap().get("sitemap_membership_limits").is_none());
    assert_eq!(absent.sitemap_membership_ledger_limits().unwrap().max_catalog_json_bytes,33554432);
    let config = config_from_input(json!({"seed_urls":["https://capacity.example/"],"sitemap_membership_limits":{"max_catalog_json_bytes":134217728}})).unwrap();
    assert_eq!(config.sitemap_source_body_limit().unwrap(),5*1024*1024);
}

#[test]
fn create_dto_rejects_invalid_membership_capacity_before_any_job_admission() {
    for value in [json!([]),json!(true),json!({"max_catalog_json_bytes":0}),json!({"max_catalog_json_bytes":134217729}),json!({"max_row_json_bytes":1048577}),json!({"max_rows":1000001}),json!({"max_rows":0}),json!({"max_catalog_json_bytes":100,"max_row_json_bytes":101}),json!({"drop_old_rows":true})] {
        assert!(config_from_input(json!({"seed_urls":["https://capacity.example/"],"sitemap_membership_limits":value})).is_err());
    }
}
