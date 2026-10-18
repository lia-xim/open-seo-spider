use super::*;

#[test]
fn custom_search_mcp_configuration_schema_is_the_same_bounded_api_contract() {
    let tools = inventory();
    let start = tools
        .iter()
        .find(|tool| tool["name"] == "spider_start_crawl")
        .unwrap();
    let schema = &start["inputSchema"]["properties"]["config"]["properties"]["custom_search"];
    assert_eq!(schema, &crate::server::custom_search_config_schema());
    assert_eq!(schema["required"], json!(["version", "rules"]));
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["rules"]["maxItems"], 32);
    assert_eq!(
        schema["properties"]["rules"]["items"]["properties"]["sources"]["items"]["enum"],
        json!([
            "raw_html",
            "rendered_html",
            "raw_selected_text",
            "rendered_selected_text"
        ])
    );
    assert_eq!(
        schema["properties"]["limits"]["properties"]["max_saved_matches"]["minimum"],
        0
    );
    assert_eq!(
        schema["properties"]["limits"]["properties"]["max_output_json_bytes"]["maximum"],
        4 * 1024 * 1024
    );
    assert_eq!(
        schema["properties"]["limits"]["properties"]["max_regex_scan_work_bytes"]["default"],
        json!(1073741824u64)
    );
    assert_eq!(
        schema["properties"]["limits"]["properties"]["max_regex_scan_work_bytes"]["maximum"],
        json!(17179869184u64)
    );
    assert!(tools.iter().any(|tool| tool["name"] == "spider_project"));
    assert!(
        !tools
            .iter()
            .any(|tool| tool["name"] == "spider_custom_search"),
        "Saved observations use existing project/pages contracts, without a redundant execution tool"
    );
}

#[tokio::test]
async fn custom_search_mcp_posts_exact_profile_and_surfaces_typed_validation_refusal() {
    async fn validate(
        axum::Json(value): axum::Json<Value>,
    ) -> Result<axum::Json<Value>, (axum::http::StatusCode, axum::Json<Value>)> {
        let checked = serde_json::from_value::<open_seo_spider_core::CrawlConfig>(value.clone())
            .map_err(anyhow::Error::from)
            .and_then(open_seo_spider_core::Crawler::new);
        checked
            .map(|_| axum::Json(json!({"echo":value})))
            .map_err(|error| {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    axum::Json(json!({"error":error.to_string()})),
                )
            })
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let (stop, closed) = tokio::sync::oneshot::channel();
    let owner = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().route("/api/crawls", axum::routing::post(validate)),
        )
        .with_graceful_shutdown(async {
            let _ = closed.await;
        })
        .await
        .unwrap();
    });
    let bridge = Bridge::new(&origin).unwrap();
    let config = json!({"seed_urls":["https://example.test/"],"custom_search":{"version":1,"rules":[{"name":"owned","method":"literal","pattern":"example","sources":["raw_html","rendered_selected_text"]}],"limits":{"max_saved_matches":0}}});
    assert_eq!(
        bridge
            .call("spider_start_crawl", json!({"config":config}))
            .await
            .unwrap()["echo"],
        config
    );
    let mut invalid = config.clone();
    invalid["custom_search"]["rules"][0]["sources"] = json!(["raw_html", "raw_html"]);
    assert!(
        bridge
            .call("spider_start_crawl", json!({"config":invalid}))
            .await
            .is_err()
    );
    bridge.shutdown_owned_work().await.unwrap();
    drop(bridge);
    stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), owner)
        .await
        .unwrap()
        .unwrap();
}
