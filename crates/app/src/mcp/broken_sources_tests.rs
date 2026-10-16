use super::*;
use axum::{
    Json, Router,
    body::Body,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type Observed = Arc<Mutex<Vec<(String, Value)>>>;
struct Fixture {
    bridge: Bridge,
    observed: Observed,
    stop: tokio::sync::oneshot::Sender<()>,
    server: tokio::task::JoinHandle<()>,
}
fn token(scope: &str, id: &str, raw: bool) -> String {
    let hash = if raw { "b".repeat(64) } else { "a".repeat(64) };
    if scope == "resource-broken-sources" {
        format!("resource-sources-v1:{id}:7:{hash}")
    } else {
        hash
    }
}
fn preview_body(scope: &str, id: &str, token: &str) -> Value {
    if scope == "resource-broken-sources" {
        json!({"scope":crate::resource_broken_sources_app::SCOPE,"job_id":id,"generation":7,"snapshot_token":token,"website_requests":0,
            "page":{"scope":crate::resource_broken_sources_app::SCOPE,"website_requests":0,"available":true,"rows":[{"url":"https://should-not-be-fetched.invalid/missing.css"}]}})
    } else {
        json!({"scope":crate::broken_sources::SCOPE,"snapshot":{"job_id":id,"app_generation":7},"snapshot_token":token,"website_requests":0,
            "rows":[{"target_requested_url":"https://should-not-be-fetched.invalid/missing"}]})
    }
}
async fn start() -> Fixture {
    let observed: Observed = Arc::new(Mutex::new(vec![]));
    let saved = observed.clone();
    let app = Router::new().route("/api/crawls/{id}/{scope}/{action}", post(
        move |Path((id, scope, action)): Path<(String, String, String)>, Json(request): Json<Value>| {
            let saved = saved.clone();
            async move {
                saved.lock().unwrap().push((format!("/api/crawls/{id}/{scope}/{action}"), request.clone()));
                assert!(matches!(scope.as_str(), "broken-sources" | "resource-broken-sources"));
                assert!(matches!(action.as_str(), "preview" | "csv"));
                let expected = token(&scope, &id, request["reporting_mode"] == "raw");
                if request["expected_snapshot_token"].as_str().is_some_and(|v| v != expected)
                    || request["expected_generation"].as_u64().is_some_and(|v| v != 7) {
                    return (StatusCode::CONFLICT, Json(json!({"code":"stale_snapshot","message":"Run a fresh preview","website_requests":0}))).into_response();
                }
                let bytes = if action == "csv" {
                    format!("scope,url\r\n{},https://should-not-be-fetched.invalid/ö\r\n", if scope == "broken-sources" { crate::broken_sources::SCOPE } else { crate::resource_broken_sources_app::SCOPE }).into_bytes()
                } else {
                    serde_json::to_vec(&preview_body(&scope, &id, &expected)).unwrap()
                };
                let mut builder = Response::builder().header("content-type", if action == "csv" { "text/csv; charset=utf-8" } else { "application/json; charset=utf-8" });
                if action == "csv" || scope == "resource-broken-sources" {
                    builder = builder.header("content-length", bytes.len().to_string()).header("x-content-sha256", crate::resource_broken_sources_app::bytes_sha(&bytes));
                }
                if action == "csv" && scope == "resource-broken-sources" {
                    builder = builder.header("x-resource-source-snapshot", expected);
                }
                // A stream avoids an automatic exact length on the legacy page
                // JSON fixture, so its absent checksum/length path is exercised.
                builder.body(Body::from_stream(futures::stream::once(async move { Ok::<_, std::io::Error>(bytes) }))).unwrap()
            }
        }
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    Fixture {
        bridge: Bridge::new(&base).unwrap(),
        observed,
        stop,
        server,
    }
}
async fn finish(fixture: Fixture) {
    fixture.bridge.shutdown_owned_work().await.unwrap();
    fixture.stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), fixture.server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn broken_sources_explicit_scope_strict_typed_union_and_csv_authority_before_http() {
    let fixture = start().await;
    assert_eq!(definition()["name"], NAME);
    assert_eq!(definition()["annotations"]["readOnlyHint"], true);
    assert_eq!(
        definition()["inputSchema"]["required"],
        json!(["id", "scope", "request"])
    );
    for value in [
        json!({"id":uuid::Uuid::nil(),"request":{}}),
        json!({"id":uuid::Uuid::nil(),"scope":"all","request":{}}),
        json!({"id":"https://target.invalid","scope":"page_links","request":{}}),
        json!({"id":uuid::Uuid::nil(),"scope":"page_links","request":{},"url":"https://target.invalid"}),
        json!({"id":uuid::Uuid::nil(),"scope":"page_links","request":{"after_sequence":1}}),
        json!({"id":uuid::Uuid::nil(),"scope":"resources","request":{"cursor":"invented"}}),
        json!({"id":uuid::Uuid::nil(),"scope":"page_links","request":{"filter":"failures"}}),
        json!({"id":uuid::Uuid::nil(),"scope":"resources","request":{"filter":"broken"}}),
        json!({"id":uuid::Uuid::nil(),"scope":"resources","request":{"reporting_mode":"auto"}}),
        json!({"id":uuid::Uuid::nil(),"scope":"page_links","request":{"limit":0}}),
        json!({"id":uuid::Uuid::nil(),"scope":"resources","request":{"limit":201}}),
        json!({"id":uuid::Uuid::nil(),"scope":"resources","request":{"after_sequence":1}}),
        json!({"id":uuid::Uuid::nil(),"scope":"page_links","action":"csv","request":{}}),
        json!({"id":uuid::Uuid::nil(),"scope":"resources","action":"csv","request":{}}),
        json!({"id":uuid::Uuid::nil(),"scope":"page_links","action":"csv","request":{"expected_snapshot_token":"a".repeat(64),"cursor":"continued"}}),
        json!({"id":uuid::Uuid::nil(),"scope":"resources","action":"csv","request":{"expected_snapshot_token":"bounded","after_sequence":1}}),
        json!({"id":uuid::Uuid::nil(),"scope":"page_links","request":{"padding":"x".repeat(4097)}}),
    ] {
        assert!(call(&fixture.bridge, value).await.is_err());
    }
    assert!(fixture.observed.lock().unwrap().is_empty());
    finish(fixture).await;
}

#[tokio::test]
async fn broken_sources_posts_both_existing_routes_preserves_modes_and_snapshot_refusal_without_fetches()
 {
    let fixture = start().await;
    let id = uuid::Uuid::nil();
    for (scope, route, expected_scope) in [
        ("page_links", "broken-sources", crate::broken_sources::SCOPE),
        (
            "resources",
            "resource-broken-sources",
            crate::resource_broken_sources_app::SCOPE,
        ),
    ] {
        for mode in [Value::Null, json!("raw"), json!("reported")] {
            let value = call(&fixture.bridge, json!({"id":id,"scope":scope,"request":{"reporting_mode":mode,"expected_generation":7,"limit":1}})).await.unwrap();
            assert_eq!(value["scope"], expected_scope);
            assert_eq!(value["website_requests"], 0);
            let requests = fixture.observed.lock().unwrap();
            let (path, request) = requests.last().unwrap();
            assert_eq!(path, &format!("/api/crawls/{id}/{route}/preview"));
            assert_eq!(request["reporting_mode"], mode);
            assert_eq!(request["limit"], 1);
            assert_eq!(request["expected_generation"], 7);
        }
        let current = token(route, &id.to_string(), false);
        let stale = current.replace('a', "f");
        let failure = call(
            &fixture.bridge,
            json!({"id":id,"scope":scope,"request":{"expected_snapshot_token":stale}}),
        )
        .await
        .unwrap_err();
        assert!(failure.to_string().contains("HTTP409"));
        assert!(failure.to_string().contains("stale_snapshot"));
        let csv = call(&fixture.bridge, json!({"id":id,"scope":scope,"action":"csv","request":{"expected_generation":7,"expected_snapshot_token":current}})).await.unwrap();
        assert_eq!(csv["scope"], expected_scope);
        assert_eq!(csv["snapshot_token"], current);
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(csv["base64"].as_str().unwrap())
            .unwrap();
        assert_eq!(bytes.len() as u64, csv["bytes"].as_u64().unwrap());
        assert_eq!(
            crate::resource_broken_sources_app::bytes_sha(&bytes),
            csv["sha256"].as_str().unwrap()
        );
        assert!(std::str::from_utf8(&bytes).unwrap().ends_with("/ö\r\n"));
        assert_eq!(
            fixture.observed.lock().unwrap().last().unwrap().0,
            format!("/api/crawls/{id}/{route}/csv")
        );
    }
    // Exactly one local service POST per call: URLs appearing in saved response
    // observations never become a request destination or a second adapter fetch.
    assert_eq!(fixture.observed.lock().unwrap().len(), 10);
    finish(fixture).await;
}

// A raw owned loopback peer permits dishonest/truncated headers that an Axum
// response builder might normalize. It reads one full request and closes EOF.
async fn raw_once(response: Vec<u8>) -> (Bridge, tokio::task::JoinHandle<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![];
        let (end, length) = loop {
            let mut chunk = [0u8; 1024];
            let n = socket.read(&mut chunk).await.unwrap();
            assert!(n > 0);
            request.extend_from_slice(&chunk[..n]);
            assert!(request.len() <= 16384);
            if let Some(end) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                let headers = std::str::from_utf8(&request[..end]).unwrap();
                let length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                break (end + 4, length);
            }
        };
        while request.len() < end + length {
            let mut chunk = [0u8; 1024];
            let n = socket.read(&mut chunk).await.unwrap();
            assert!(n > 0);
            request.extend_from_slice(&chunk[..n]);
        }
        let _ = socket.write_all(&response).await;
        let _ = socket.shutdown().await;
        String::from_utf8(request).unwrap()
    });
    (Bridge::new(&base).unwrap(), server)
}
fn wire(body: &[u8], headers: &[(&str, String)]) -> Vec<u8> {
    let mut bytes = b"HTTP/1.1 200 OK\r\nConnection: close\r\n".to_vec();
    for (key, value) in headers {
        bytes.extend_from_slice(format!("{key}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(body);
    bytes
}
async fn raw_result(scope: &str, action: &str, request: Value, response: Vec<u8>) -> Result<Value> {
    let (bridge, server) = raw_once(response).await;
    let result = call(
        &bridge,
        json!({"id":uuid::Uuid::nil(),"scope":scope,"action":action,"request":request}),
    )
    .await;
    bridge.shutdown_owned_work().await.unwrap();
    let observed = tokio::time::timeout(std::time::Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
    assert!(observed.starts_with(&format!(
        "POST /api/crawls/{}/{}/{action} HTTP/1.1\r\n",
        uuid::Uuid::nil(),
        if scope == "page_links" {
            "broken-sources"
        } else {
            "resource-broken-sources"
        }
    )));
    result
}

#[tokio::test]
async fn broken_sources_complete_csv_requires_eof_length_digest_and_resource_snapshot_header() {
    let body = b"scope,url\r\ncomplete,https://saved.invalid/\r\n";
    for scope in ["page_links", "resources"] {
        let token = token(
            if scope == "page_links" {
                "broken-sources"
            } else {
                "resource-broken-sources"
            },
            &uuid::Uuid::nil().to_string(),
            false,
        );
        let request = json!({"expected_snapshot_token":token});
        let complete = vec![
            ("Content-Type", "text/csv".into()),
            ("Content-Length", body.len().to_string()),
            (
                "x-content-sha256",
                crate::resource_broken_sources_app::bytes_sha(body),
            ),
            ("x-resource-source-snapshot", token.clone()),
        ];
        assert!(
            raw_result(scope, "csv", request.clone(), wire(body, &complete))
                .await
                .is_ok()
        );
        for missing in ["Content-Length", "x-content-sha256"] {
            let headers = complete
                .iter()
                .filter(|(k, _)| *k != missing)
                .cloned()
                .collect::<Vec<_>>();
            assert!(
                raw_result(scope, "csv", request.clone(), wire(body, &headers))
                    .await
                    .is_err()
            );
        }
        for (key, replacement) in [
            ("Content-Length", (body.len() + 1).to_string()),
            ("Content-Length", (body.len() - 1).to_string()),
            ("x-content-sha256", "0".repeat(64)),
            ("Content-Type", "application/json".into()),
        ] {
            let headers = complete
                .iter()
                .map(|(k, v)| {
                    (
                        *k,
                        if *k == key {
                            replacement.clone()
                        } else {
                            v.clone()
                        },
                    )
                })
                .collect::<Vec<_>>();
            assert!(
                raw_result(scope, "csv", request.clone(), wire(body, &headers))
                    .await
                    .is_err()
            );
        }
        if scope == "resources" {
            for replacement in [None, Some("wrong-snapshot".to_owned())] {
                let mut headers = complete
                    .iter()
                    .filter(|(k, _)| *k != "x-resource-source-snapshot")
                    .cloned()
                    .collect::<Vec<_>>();
                if let Some(value) = replacement {
                    headers.push(("x-resource-source-snapshot", value));
                }
                assert!(
                    raw_result(scope, "csv", request.clone(), wire(body, &headers))
                        .await
                        .is_err()
                );
            }
        }
    }
}

#[tokio::test]
async fn broken_sources_preview_rejects_cross_scope_generation_foreign_crawl_and_incomplete_json_or_hash()
 {
    let id = uuid::Uuid::nil().to_string();
    for (scope, route) in [
        ("page_links", "broken-sources"),
        ("resources", "resource-broken-sources"),
    ] {
        let value = preview_body(route, &id, &token(route, &id, false));
        for change in 0..8 {
            let mut value = value.clone();
            match change {
                0 => value["scope"] = json!("another_scope"),
                1 => value["website_requests"] = json!(1),
                2 if scope == "resources" => value["job_id"] = json!(uuid::Uuid::new_v4()),
                2 => value["snapshot"]["job_id"] = json!(uuid::Uuid::new_v4()),
                3 if scope == "resources" => value["generation"] = json!(8),
                3 => value["snapshot"]["app_generation"] = json!(8),
                4 if scope == "resources" => value["page"]["website_requests"] = json!(1),
                4 => value["scope"] = json!(crate::resource_broken_sources_app::SCOPE),
                _ => {}
            }
            let mut body = serde_json::to_vec(&value).unwrap();
            if change == 5 {
                body.extend_from_slice(b"garbage");
            }
            let mut headers = vec![
                ("Content-Type", "application/json".into()),
                ("Content-Length", body.len().to_string()),
                (
                    "x-content-sha256",
                    crate::resource_broken_sources_app::bytes_sha(&body),
                ),
            ];
            if change == 6 {
                headers[2].1 = "0".repeat(64);
            }
            if change == 7 {
                headers[1].1 = (BODY_BYTES + 1).to_string();
            }
            assert!(
                raw_result(
                    scope,
                    "preview",
                    json!({"expected_generation":7}),
                    wire(&body, &headers)
                )
                .await
                .is_err()
            );
        }
        let body = serde_json::to_vec(&value).unwrap();
        let legacy = vec![("Content-Type", "application/json".into())];
        let result = raw_result(scope, "preview", json!({}), wire(&body, &legacy)).await;
        assert_eq!(result.is_ok(), scope == "page_links");
    }
    let oversized = vec![b' '; BODY_BYTES + 1];
    assert!(
        raw_result(
            "page_links",
            "preview",
            json!({}),
            wire(&oversized, &[("Content-Type", "application/json".into())])
        )
        .await
        .is_err()
    );
}
