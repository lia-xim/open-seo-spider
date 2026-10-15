//! AUTHORED / NOT_COMPILED / NOT_RUN. The61-second case is a deliberate
//! actual HTTP deadline regression, never a simulated clock/constant assertion.
use super::*;

#[tokio::test]
async fn findings_postfix_stdio_exact_utf8_rpc_id_and_plus_one_preserve_following_frame() {
    let f = Fixture::actual(2).await;
    let (bridge, api, stop, join) = provider(&f).await;
    let (mut writer, mut reader, server) = stdio(bridge);
    let exact = "ä".repeat(512);
    assert_eq!(exact.len(), 1024);
    send(&mut writer, request(json!(exact), json!({"id":api.id}))).await;
    let accepted = next(&mut reader).await;
    assert_eq!(accepted["id"], exact);
    assert_eq!(accepted["result"]["isError"], false);
    assert_eq!(api.hits.load(Ordering::SeqCst), 1);
    send(
        &mut writer,
        request(json!(format!("{exact}x")), json!({"id":api.id})),
    )
    .await;
    let rejected = next(&mut reader).await;
    assert!(rejected["id"].is_null());
    assert_eq!(rejected["error"]["code"], -32600);
    assert!(serde_json::to_vec(&rejected).unwrap().len() < 8192);
    send(
        &mut writer,
        json!({"jsonrpc":"2.0","id":42,"method":"ping"}),
    )
    .await;
    let following = next(&mut reader).await;
    assert_eq!(following["id"], 42);
    assert_eq!(following["result"], json!({}));
    assert_eq!(
        api.hits.load(Ordering::SeqCst),
        1,
        "rejected id must not open another API request"
    );
    writer.shutdown().await.unwrap();
    tokio::time::timeout(WAIT, server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    stop.send(()).unwrap();
    join.await.unwrap();
    f.finish().await;
}

async fn delayed_findings(State(api): State<Api>, bytes: axum::body::Bytes) -> Response {
    tokio::time::sleep(Duration::from_secs(61)).await;
    actual(State(api), bytes).await
}
async fn delayed_legacy() -> Response {
    tokio::time::sleep(Duration::from_secs(61)).await;
    Response::builder()
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap()
}

#[tokio::test]
async fn findings_postfix_actual_sixty_one_second_http_response_succeeds_while_legacy_times_out() {
    let f = Fixture::actual(2).await;
    let api = Api {
        path: f.path.clone(),
        id: uuid::Uuid::new_v4(),
        hits: Arc::default(),
        received: Arc::default(),
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bridge = Bridge::new(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let routes = Router::new()
        .route("/api/crawls/{id}/sitemaps/findings", post(delayed_findings))
        .route("/api/crawls/{id}", axum::routing::get(delayed_legacy))
        .with_state(api.clone());
    let (stop, stopped) = oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, routes)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    // Both real requests begin together. The old shared client deadline remains
    //60s; only the actual findings POST has the150s per-request override.
    let (finding, legacy) = tokio::time::timeout(Duration::from_secs(90), async {
        tokio::join!(
            call(&bridge, json!({"id":api.id})),
            bridge.call("spider_status", json!({"id":api.id}))
        )
    })
    .await
    .expect("real deadline regression exceeded its outer test budget");
    assert!(!finding.failed, "{}", finding.value);
    assert_eq!(finding.value["access_target"]["job_id"], api.id.to_string());
    assert_eq!(finding.value["website_requests"], 0);
    let legacy = legacy.expect_err("legacy request must keep inherited60s timeout");
    assert!(
        legacy
            .downcast_ref::<reqwest::Error>()
            .is_some_and(reqwest::Error::is_timeout),
        "{legacy:#}"
    );
    assert_eq!(api.hits.load(Ordering::SeqCst), 1);
    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), server)
        .await
        .unwrap()
        .unwrap();
    f.finish().await;
}
