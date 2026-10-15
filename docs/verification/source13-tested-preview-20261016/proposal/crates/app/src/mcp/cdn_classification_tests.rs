//! AUTHORED_NOT_RUN. Actual inventory plus Bridge request to a finite owned API stub.
use super::*;

#[test]
fn cdn_mcp_inventory_describes_exact_origin_reporting_and_no_network_grant() {
    let tools = inventory();
    let tool = tools
        .iter()
        .find(|v| v["name"] == "spider_start_crawl")
        .unwrap();
    let schema = &tool["inputSchema"]["properties"]["config"]["properties"]["cdn_classification"];
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["required"], json!(["version", "origins"]));
    assert_eq!(schema["properties"]["version"]["const"], 1);
    assert_eq!(schema["properties"]["origins"]["maxItems"], 32);
    assert!(
        schema["description"]
            .as_str()
            .unwrap()
            .contains("not grant")
    );
}

#[tokio::test]
async fn cdn_mcp_real_bridge_posts_exact_optional_config_and_propagates_validation_error() {
    async fn validate(
        axum::Json(value): axum::Json<Value>,
    ) -> Result<axum::Json<Value>, (axum::http::StatusCode, axum::Json<Value>)> {
        let result = serde_json::from_value::<open_seo_spider_core::CrawlConfig>(value.clone())
            .map_err(anyhow::Error::from)
            .and_then(|config| {
                if let Some(cdn) = &config.cdn_classification {
                    cdn.validate()?;
                }
                Ok(())
            });
        match result {
            Ok(()) => Ok(axum::Json(json!({"echo":value}))),
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
    let cfg = json!({"seed_urls":["https://www.example/"],"cdn_classification":{"version":1,"origins":["https://cdn.example"]}});
    let response = bridge
        .call("spider_start_crawl", json!({"config":cfg}))
        .await
        .unwrap();
    assert_eq!(response["echo"], cfg);
    assert!(bridge.call("spider_start_crawl",json!({"config":{"seed_urls":["https://www.example/"],"cdn_classification":{"version":1,"origins":["https://*.example"]}}})).await.is_err());
    bridge.shutdown_owned_work().await.unwrap();
    drop(bridge);
    stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), owner)
        .await
        .unwrap()
        .unwrap();
}
