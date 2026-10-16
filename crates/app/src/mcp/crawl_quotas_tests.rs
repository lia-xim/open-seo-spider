//! AUTHORED / NOT_RUN. Actual strict adapter rejects ambiguous actions before loopback.
use super::*;
#[tokio::test]
async fn quotas_mcp_has_only_strict_readonly_summary_and_refuses_other_actions_before_io() {
    let bridge = Bridge::new("http://127.0.0.1:1").unwrap();
    for args in [
        json!({"id":uuid::Uuid::nil()}),
        json!({"id":"../../other","action":"summary"}),
        json!({"id":uuid::Uuid::nil(),"action":"read"}),
        json!({"id":uuid::Uuid::nil(),"action":"csv"}),
        json!({"id":uuid::Uuid::nil(),"action":"summary","quota_increases":{}}),
    ] {
        assert!(call(&bridge, args).await.is_err());
    }
    let d = definition();
    assert_eq!(d["inputSchema"]["properties"]["action"]["const"], "summary");
    assert_eq!(d["annotations"]["readOnlyHint"], true);
    assert_eq!(d["annotations"]["openWorldHint"], false);
}
#[tokio::test]
async fn quotas_mcp_actual_get_adapter_preserves_legacy_unknown_and_closes_server() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bridge = Bridge::new(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    let id = uuid::Uuid::new_v4();
    let route = format!("/api/crawls/{id}/quotas");
    let router = axum::Router::new().route(
        &route,
        axum::routing::get(|| async {
            axum::Json(json!({"enabled":false,"availability":"legacy_unavailable","summary":null}))
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
    let result = call(&bridge, json!({"id":id,"action":"summary"})).await;
    stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
    let body = result.unwrap();
    assert!(body["summary"].is_null());
    assert_eq!(body["availability"], "legacy_unavailable");
}
