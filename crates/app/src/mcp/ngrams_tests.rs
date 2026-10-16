use super::*;
#[tokio::test]
async fn ngram_mcp_reporting_strict_request_and_exact_inventory_extension_without_api_contact() {
    let bridge = Bridge::new("http://127.0.0.1:1").unwrap();
    let inventory = bridge
        .dispatch(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
        .await
        .unwrap();
    let tools = inventory["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 32);
    assert_eq!(tools[31], crate::mcp::spelling::definition());
    assert_eq!(tools[29], definition());
    assert_eq!(definition()["annotations"]["readOnlyHint"], true);
    for mode in ["raw", "reported"] {
        let a: Args = serde_json::from_value(
            json!({"id":uuid::Uuid::nil(),"request":{"reporting_mode":mode}}),
        )
        .unwrap();
        a.request.validate().unwrap();
        assert_eq!(
            serde_json::to_value(a.request).unwrap()["reporting_mode"],
            mode
        );
    }
    for args in [
        json!({"id":uuid::Uuid::nil(),"request":{"reporting_mode":"latest"}}),
        json!({"id":uuid::Uuid::nil(),"request":{"html":"invented"}}),
        json!({"id":uuid::Uuid::nil(),"action":"csv","request":{}}),
        json!({"id":uuid::Uuid::nil(),"request":{},"endpoint":"http://external"}),
    ] {
        assert!(call(&bridge, args).await.is_err());
    }
    bridge.shutdown_owned_work().await.unwrap();
}
#[tokio::test]
async fn ngram_mcp_reporting_shared_service_modes_csv_digest_and_stale_refusal_with_actual_eof() {
    use crate::ngram_app as service;
    use axum::{Json, Router, extract::Path, response::IntoResponse, routing::post};
    use open_seo_spider_core::{CrawlConfig, CrawlReport, PageRecord};
    let report = CrawlReport {
        config: CrawlConfig {
            directive_reporting: Some(open_seo_spider_core::DirectiveReportingConfig {
                respect_noindex: true,
                ..Default::default()
            }),
            ..Default::default()
        },
        pages: vec![PageRecord {
            url: "https://x.test/hidden".into(),
            final_url: "https://x.test/hidden".into(),
            meta_robots: vec!["noindex".into()],
            ..Default::default()
        }],
        issues: vec![],
        summary: Default::default(),
        warnings: vec![],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
        sitemap_source_measurements: None,
    };
    let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let observed = requests.clone();
    let app = Router::new().route(
        "/api/crawls/{id}/ngrams/{action}",
        post(
            move |Path((_id, action)): Path<(String, String)>, Json(q): Json<service::Request>| {
                let report = report.clone();
                let observed = observed.clone();
                async move {
                    observed
                        .lock()
                        .unwrap()
                        .push(serde_json::to_value(&q).unwrap());
                    let result = (|| -> anyhow::Result<_> {
                        let corpus = service::aggregate(&report, &q)?;
                        let context = service::context(
                            &report,
                            Some(uuid::Uuid::nil().to_string()),
                            Some(7),
                            "owned-MCP-service-fixture",
                        )?;
                        if action == "csv" {
                            service::csv(&corpus, context, &q)
                        } else {
                            service::preview(&corpus, context, &q)
                        }
                    })();
                    match result {
                        Ok((bytes, snapshot)) => {
                            let digest = if q.query == "bad-digest" {
                                "0".repeat(64)
                            } else {
                                service::sha(&bytes)
                            };
                            let len = bytes.len().to_string();
                            (
                                [
                                    (
                                        "content-type",
                                        if action == "csv" {
                                            "text/csv"
                                        } else {
                                            "application/json"
                                        },
                                    ),
                                    ("content-length", len.as_str()),
                                    ("x-content-sha256", digest.as_str()),
                                    ("x-ngram-snapshot", snapshot.snapshot_token.as_str()),
                                ],
                                bytes,
                            )
                                .into_response()
                        }
                        Err(e) => (axum::http::StatusCode::CONFLICT, e.to_string()).into_response(),
                    }
                }
            },
        ),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let bridge = Bridge::new(&base).unwrap();
    let first = call(
        &bridge,
        json!({"id":uuid::Uuid::nil(),"request":{"view":"sources"}}),
    )
    .await
    .unwrap();
    assert_eq!(first["saved_documents"], 0);
    assert_eq!(first["reporting"]["captured_page_observations"], 1);
    let token = first["snapshot"]["snapshot_token"].as_str().unwrap();
    let csv = call(&bridge, json!({"id":uuid::Uuid::nil(),"action":"csv","request":{"view":"sources","expected_snapshot_token":token}})).await.unwrap();
    assert_eq!(csv["snapshot_token"], token);
    use base64::Engine as _;
    let body = base64::engine::general_purpose::STANDARD
        .decode(csv["base64"].as_str().unwrap())
        .unwrap();
    assert_eq!(body.len(), csv["bytes"].as_u64().unwrap() as usize);
    assert_eq!(service::sha(&body), csv["sha256"].as_str().unwrap());
    assert!(std::str::from_utf8(&body).unwrap().contains("reporting"));
    assert!(call(&bridge, json!({"id":uuid::Uuid::nil(),"request":{"view":"sources","reporting_mode":"raw","expected_snapshot_token":token}})).await.is_err());
    let raw = call(
        &bridge,
        json!({"id":uuid::Uuid::nil(),"request":{"view":"sources","reporting_mode":"raw"}}),
    )
    .await
    .unwrap();
    assert_eq!(raw["saved_documents"], 1);
    assert_eq!(raw["rows"][0]["source"]["raw_page_ordinal"], 0);
    assert!(
        call(
            &bridge,
            json!({"id":uuid::Uuid::nil(),"request":{"view":"sources","query":"bad-digest"}})
        )
        .await
        .is_err()
    );
    assert_eq!(requests.lock().unwrap().len(), 5);
    assert_eq!(requests.lock().unwrap()[3]["reporting_mode"], "raw");
    bridge.shutdown_owned_work().await.unwrap();
    stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
}
