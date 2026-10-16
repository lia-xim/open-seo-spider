//! AUTHORED_NOT_RUN. Actual inventory and Bridge HTTP request, no mock crawler.
use super::*;

#[test]
fn cdn_directory_mcp_inventory_is_separate_bounded_reporting_without_network_authority() {
    let tools = inventory();
    let start = tools
        .iter()
        .find(|v| v["name"] == "spider_start_crawl")
        .unwrap();
    let properties = &start["inputSchema"]["properties"]["config"]["properties"];
    let s = &properties["cdn_directory_reporting"];
    assert_eq!(s["required"], json!(["version", "rules"]));
    assert_eq!(s["additionalProperties"], false);
    assert_eq!(s["properties"]["rules"]["maxItems"], 32);
    assert!(
        s["description"]
            .as_str()
            .unwrap()
            .contains("Does not grant")
    );
    assert_eq!(
        properties["cdn_classification"]["required"],
        json!(["version", "origins"])
    );
}

#[tokio::test]
async fn cdn_directory_mcp_real_bridge_posts_same_config_and_propagates_typed_rule_refusal() {
    async fn validate(
        axum::Json(value): axum::Json<Value>,
    ) -> Result<axum::Json<Value>, (axum::http::StatusCode, axum::Json<Value>)> {
        let checked = serde_json::from_value::<open_seo_spider_core::CrawlConfig>(value.clone())
            .map_err(anyhow::Error::from)
            .and_then(|c| {
                if let Some(rules) = &c.cdn_directory_reporting {
                    rules.validate()?;
                }
                Ok(())
            });
        checked
            .map(|()| axum::Json(json!({"echo":value})))
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
    let cfg = json!({"seed_urls":["https://www.example/"],"cdn_directory_reporting":{"version":1,"rules":["HTTPS://CDN.example/assets/"]}});
    assert_eq!(
        bridge
            .call("spider_start_crawl", json!({"config":cfg}))
            .await
            .unwrap()["echo"],
        cfg
    );
    assert!(bridge.call("spider_start_crawl",json!({"config":{"seed_urls":["https://www.example/"],"cdn_directory_reporting":{"version":1,"rules":["https://cdn.example:443/"]}}})).await.is_err());
    bridge.shutdown_owned_work().await.unwrap();
    drop(bridge);
    stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), owner)
        .await
        .unwrap()
        .unwrap();
}
