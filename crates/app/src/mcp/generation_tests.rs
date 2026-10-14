use super::*;
use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    response::Response,
    routing::post,
};
use open_seo_spider_core::{CrawlConfig, CrawlReport, PageRecord, generate_sitemap_bundle};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone)]
struct Wire {
    body: Vec<u8>,
    headers: Vec<(String, String)>,
    status: u16,
    requests: Arc<AtomicUsize>,
    received: Arc<Mutex<Option<(String, Value)>>>,
}
async fn handle(State(w): State<Wire>, request: Request) -> Response {
    w.requests.fetch_add(1, Ordering::SeqCst);
    let path = request.uri().to_string();
    let bytes = axum::body::to_bytes(request.into_body(), MAX_ZIP_BYTES + 1)
        .await
        .unwrap();
    *w.received.lock().unwrap() = Some((path, serde_json::from_slice(&bytes).unwrap()));
    let mut response = Response::builder().status(w.status);
    for (name, value) in &w.headers {
        response = response.header(name, value);
    }
    response.body(Body::from(w.body)).unwrap()
}
fn saved_bundle() -> open_seo_spider_core::SitemapGeneratedBundle {
    let page = "https://example.org/p?a=1&b=2";
    let report = CrawlReport {
        config: CrawlConfig::default(),
        summary: Default::default(),
        pages: vec![PageRecord {
            url: page.into(),
            final_url: page.into(),
            status: Some(200),
            final_status: Some(200),
            internal: true,
            indexable: true,
            content_type: "text/html".into(),
            ..Default::default()
        }],
        issues: vec![],
        warnings: vec![],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
    };
    generate_sitemap_bundle(
        &report,
        &SitemapGenerationOptions {
            public_base_url: "https://example.org/".into(),
            ..Default::default()
        },
    )
    .unwrap()
}
fn wire(id: uuid::Uuid) -> Wire {
    let b = saved_bundle();
    Wire {
        headers: vec![
            ("content-type".into(), "application/zip".into()),
            ("content-length".into(), b.zip_bytes.len().to_string()),
            (
                "content-disposition".into(),
                format!("attachment; filename=\"{id}-sitemap-bundle.zip\""),
            ),
            (
                "x-sitemap-generation-engine".into(),
                SITEMAP_GENERATION_ENGINE.into(),
            ),
            ("x-sitemap-bundle-sha256".into(), b.zip_sha256),
            ("x-sitemap-report-sha256".into(), b.manifest.report_sha256),
            ("x-sitemap-generated-urls".into(), "1".into()),
            ("x-sitemap-excluded-pages".into(), "0".into()),
        ],
        body: b.zip_bytes,
        status: 200,
        requests: Arc::new(AtomicUsize::new(0)),
        received: Arc::new(Mutex::new(None)),
    }
}
async fn provider(
    w: Wire,
) -> (
    Bridge,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bridge = Bridge::new(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new()
                .route("/api/crawls/{id}/sitemap-bundle", post(handle))
                .with_state(w),
        )
        .with_graceful_shutdown(async {
            let _ = rx.await;
        })
        .await
        .unwrap();
    });
    (bridge, tx, task)
}
fn args(id: uuid::Uuid) -> Value {
    json!({"id":id,"options":{"public_base_url":"https://example.org/"}})
}
fn request(id: uuid::Uuid) -> Value {
    json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"spider_generate_sitemaps","arguments":args(id)}})
}

#[tokio::test]
async fn generation_mcp_actual_binary_roundtrip_is_single_encoded_hash_bound_and_readonly() {
    let id = uuid::Uuid::new_v4();
    let w = wire(id);
    let (bridge, stop, task) = provider(w.clone()).await;
    let response = bridge.dispatch(request(id)).await.unwrap();
    assert_eq!(response["result"]["isError"], false, "{response}");
    let value = &response["result"]["structuredContent"];
    let actual = STANDARD
        .decode(value["data_base64"].as_str().unwrap())
        .unwrap();
    assert_eq!(actual, w.body);
    assert_eq!(value["bytes"], actual.len());
    assert_eq!(value["sha256"], format!("{:x}", Sha256::digest(&actual)));
    assert_eq!(value["filename"], format!("{id}-sitemap-bundle.zip"));
    assert_eq!(value["artifact_stored"], false);
    assert!(
        !response["result"]["content"]
            .to_string()
            .contains(value["data_base64"].as_str().unwrap())
    );
    assert!(serde_json::to_vec(&response).unwrap().len() < MAX_RESPONSE_BYTES);
    assert_eq!(w.requests.load(Ordering::SeqCst), 1);
    let (path, body) = w.received.lock().unwrap().clone().unwrap();
    assert_eq!(path, format!("/api/crawls/{id}/sitemap-bundle"));
    assert_eq!(body["max_total_output_bytes"], MAX_ZIP_BYTES);
    assert_eq!(body["lastmod"]["source"], "omit");
    stop.send(()).unwrap();
    task.await.unwrap();
}
#[tokio::test]
async fn generation_mcp_strict_preflight_makes_zero_api_requests() {
    let id = uuid::Uuid::new_v4();
    let w = wire(id);
    let (bridge, stop, task) = provider(w.clone()).await;
    let invalid = vec![
        json!({"id":id,"options":null}),
        json!({"id":"not-uuid","options":{"public_base_url":"https://example.org/"}}),
        json!({"id":id,"options":{"public_base_url":"https://example.org/a/../"}}),
        json!({"id":id,"options":{"public_base_url":"https://example.org/","max_total_output_bytes":8388609}}),
        json!({"id":id,"options":{"public_base_url":"https://example.org/","max_urls_per_file":null}}),
        json!({"id":id,"options":{"public_base_url":"https://example.org/","unknown":true}}),
        json!({"id":id,"options":{"public_base_url":"https://example.org/","lastmod":{"source":"provided","values":[{"page_url":"https://example.org/p","value":"2026-02-30","reference":"explicit"}]}}}),
        json!({"id":id,"options":{"public_base_url":"https://example.org/","lastmod":{"source":"provided","values":[{"page_url":"https://example.org/p","value":"2026-10-14","reference":"explicit"},{"page_url":"https://example.org/p","value":"2026-10-15","reference":"explicit"}]}}}),
    ];
    for args in invalid {
        let error = call(&bridge, args).await.unwrap_err();
        assert_eq!(error.code, "invalid_arguments");
    }
    assert_eq!(w.requests.load(Ordering::SeqCst), 0);
    stop.send(()).unwrap();
    task.await.unwrap();
    for base in [
        "http://127.0.0.1:8787/path",
        "http://127.0.0.1:8787/path/../",
        "http://127.0.0.1:8787/%2e/",
        "http://127.0.0.1:8787/?x=1",
        "http://127.0.0.1:8787/#frag",
        "http://127.0.0.1:8787/\n",
        "https://127.0.0.1:8787/",
        "http://example.org/",
    ] {
        assert!(Bridge::new(base).is_err(), "{base:?}");
    }
    assert!(Bridge::new("http://[::1]:8787").is_ok());
}
#[tokio::test]
async fn generation_mcp_corrupt_or_over_budget_responses_never_return_binary() {
    let id = uuid::Uuid::new_v4();
    for (name, value, expected) in [
        (
            "x-sitemap-bundle-sha256",
            "0".repeat(64),
            "invalid_response",
        ),
        ("content-type", "text/plain".into(), "invalid_response"),
        (
            "x-sitemap-generation-engine",
            "untrusted-engine".into(),
            "invalid_response",
        ),
        (
            "content-disposition",
            "attachment; filename=\"../unsafe.zip\"".into(),
            "invalid_response",
        ),
        (
            "content-length",
            (MAX_ZIP_BYTES + 1).to_string(),
            "too_large",
        ),
    ] {
        let mut w = wire(id);
        w.headers.iter_mut().find(|(n, _)| n == name).unwrap().1 = value;
        if name == "content-length" {
            w.body.resize(MAX_ZIP_BYTES + 1, 0);
        }
        let (bridge, stop, task) = provider(w.clone()).await;
        let result = call(&bridge, args(id)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, expected);
        assert_eq!(w.requests.load(Ordering::SeqCst), 1);
        stop.send(()).unwrap();
        task.await.unwrap();
    }
}
#[tokio::test]
async fn generation_mcp_api_error_body_is_bounded_and_explicit() {
    let id = uuid::Uuid::new_v4();
    let mut w = wire(id);
    w.status = 422;
    w.body = br#"{"code":"no_eligible_urls","error":"No eligible saved pages"}"#.to_vec();
    w.headers = vec![("content-type".into(), "application/json".into())];
    let (bridge, stop, task) = provider(w.clone()).await;
    let response = bridge.dispatch(request(id)).await.unwrap();
    assert_eq!(response["result"]["isError"], true);
    assert_eq!(response["result"]["structuredContent"]["code"], "api_error");
    assert!(response.to_string().contains("no_eligible_urls"));
    assert!(
        response["result"]["structuredContent"]
            .get("data_base64")
            .is_none()
    );
    stop.send(()).unwrap();
    task.await.unwrap();
    w.body = vec![b'x'; MAX_ERROR_BYTES + 1];
    let (bridge, stop, task) = provider(w).await;
    assert_eq!(call(&bridge, args(id)).await.unwrap_err().code, "too_large");
    stop.send(()).unwrap();
    task.await.unwrap();
}
#[tokio::test]
async fn generation_mcp_maximum_binary_single_encoding_fits_complete_transport_budget() {
    // Transport-envelope stress, not a claim that this synthetic byte sequence is a ZIP.
    let encoded = STANDARD.encode(vec![0u8; MAX_ZIP_BYTES]);
    let response = complete_response(
        json!(1),
        Ok(json!({"bytes":MAX_ZIP_BYTES,"sha256":"0".repeat(64),"data_base64":encoded})),
    )
    .await;
    assert_eq!(response["result"]["isError"], false);
    let serialized = serde_json::to_vec(&response).unwrap();
    assert!(serialized.len() < MAX_RESPONSE_BYTES);
    assert_eq!(
        std::str::from_utf8(&serialized)
            .unwrap()
            .matches("\"data_base64\":")
            .count(),
        1
    );
}

#[tokio::test]
async fn generation_mcp_over_budget_result_preserves_ordinary_request_id() {
    // An intentionally oversized transport value, not a generated ZIP authority fixture.
    let response = complete_response(
        json!(7),
        Ok(json!({"bytes":MAX_RESPONSE_BYTES+1,"sha256":"0".repeat(64),"data_base64":"a".repeat(MAX_RESPONSE_BYTES+1)})),
    ).await;
    assert_eq!(response["id"], 7);
    assert_eq!(response["error"]["code"], -32603);
    assert!(response.get("result").is_none());
    assert!(serde_json::to_vec(&response).unwrap().len() < 1024);
    assert_eq!(
        bounded_protocol_error(json!(7), "Sitemap response worker failed")["id"],
        7
    );
}
