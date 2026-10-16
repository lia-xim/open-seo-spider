use super::*;
#[tokio::test]
async fn segments_mcp_rejects_invalid_authority_shapes_before_loopback_and_keeps_readonly_inventory()
 {
    let bridge = Bridge::new("http://127.0.0.1:1").unwrap();
    for args in [
        json!({"id":"../../data","request":{}}),
        json!({"id":uuid::Uuid::nil(),"request":{"url":"http://evil/"}}),
        json!({"id":uuid::Uuid::nil(),"action":"csv","request":{}}),
        json!({"id":uuid::Uuid::nil(),"request":{"limit":201}}),
    ] {
        assert!(call(&bridge, args).await.is_err());
    }
    let d = definition();
    assert_eq!(d["annotations"]["readOnlyHint"], true);
    assert_eq!(d["annotations"]["openWorldHint"], false);
    let listed = bridge
        .dispatch(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
        .await
        .unwrap();
    let names = listed["result"]["tools"].as_array().unwrap();
    assert_eq!(names.len(), 28);
    assert_eq!(names[27]["name"], NAME);
    assert_eq!(names[26]["name"], "spider_api_enrichment");
}
#[tokio::test]
async fn segments_mcp_actual_http_adapter_preserves_request_digest_and_normal_server_close() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let id = uuid::Uuid::new_v4();
    let body = b"{\"rows\":[],\"complete_fixture\":true}".to_vec();
    let hash = crate::segments_app::sha(&body);
    let route = format!("/api/crawls/{id}/segments");
    let router = axum::Router::new().route(
        &route,
        axum::routing::post(move |axum::Json(q): axum::Json<Value>| {
            let b = body.clone();
            let h = hash.clone();
            async move {
                assert_eq!(q["rules"]["version"], 1);
                assert_eq!(q["selection"]["kind"], "all");
                axum::response::Response::builder()
                    .header("x-content-sha256", h)
                    .body(axum::body::Body::from(b))
                    .unwrap()
            }
        }),
    );
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let bridge = Bridge::new(&base).unwrap();
    let result = call(&bridge, json!({"id":id,"request":{}})).await;
    stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.unwrap()["complete_fixture"], true);
}
