//! AUTHORED / NOT_RUN. Actual tools/list and Bridge POST dispatch to an owned
//! validation-only local API fixture; this does not pretend to crawl a website.
use super::*;

#[test]
fn static_limits_mcp_inventory_documents_version_units_zero_and_renderer_boundary() {
    let tools = inventory();
    let start = tools
        .iter()
        .find(|v| v["name"] == "spider_start_crawl")
        .unwrap();
    let limits =
        &start["inputSchema"]["properties"]["config"]["properties"]["crawl_admission_limits"];
    assert_eq!(limits["additionalProperties"], false);
    assert_eq!(limits["required"], json!(["version"]));
    assert_eq!(limits["properties"]["version"]["const"], 1);
    assert_eq!(limits["properties"]["max_folder_depth"]["minimum"], 0);
    assert_eq!(limits["properties"]["max_query_tokens"]["minimum"], 0);
    assert_eq!(
        limits["properties"]["max_fetch_url_utf8_bytes"]["minimum"],
        1
    );
    assert_eq!(
        limits["properties"]["max_fetch_url_utf8_bytes"]["maximum"],
        1048576
    );
    assert!(limits["description"].as_str().unwrap().contains("renderer"));
    assert!(
        limits["description"]
            .as_str()
            .unwrap()
            .contains("present empty=1")
    );
}

#[tokio::test]
async fn static_limits_mcp_bridge_posts_exact_config_and_propagates_validation_refusal() {
    async fn validate(
        axum::Json(value): axum::Json<Value>,
    ) -> Result<axum::Json<Value>, (axum::http::StatusCode, axum::Json<Value>)> {
        let config = serde_json::from_value::<open_seo_spider_core::CrawlConfig>(value.clone())
            .and_then(|cfg| {
                cfg.validate_crawl_admission_limits()
                    .map(|()| cfg)
                    .map_err(serde::de::Error::custom)
            });
        match config {
            Ok(_) => Ok(axum::Json(json!({"echo":value}))),
            Err(error) => Err((
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(json!({"error":error.to_string()})),
            )),
        }
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
    let config = json!({"seed_urls":["https://example.test/"],"crawl_admission_limits":{"version":1,"max_query_tokens":0,"max_fetch_url_utf8_bytes":8192}});
    let response = bridge
        .call("spider_start_crawl", json!({"config":config}))
        .await
        .unwrap();
    assert_eq!(response["echo"], config);
    assert!(bridge.call("spider_start_crawl",json!({"config":{"seed_urls":["https://example.test/"],"crawl_admission_limits":{"version":1,"max_fetch_url_utf8_bytes":0}}})).await.is_err());
    bridge.shutdown_owned_work().await.unwrap();
    drop(bridge);
    stop.send(()).unwrap();
    owner.await.unwrap();
}
