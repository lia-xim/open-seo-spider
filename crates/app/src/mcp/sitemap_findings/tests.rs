//! AUTHORED / NOT_COMPILED / NOT_RUN. Real Bridge/HTTP/stdio and public Store.
//! The local oracle handler below is not the production App job-lock handler;
//! that actual route is a separate official-SDK native acceptance gate.
use super::*;
use crate::sitemap_findings_cli::tests::fixture::Fixture;
use axum::{Router, body::Body, extract::State, response::Response, routing::post};
use open_seo_spider_core::{CrawlStore, SitemapReadControl};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{Notify, oneshot},
};
const WAIT: Duration = Duration::from_secs(15);

#[derive(Clone)]
struct Api {
    path: PathBuf,
    id: uuid::Uuid,
    hits: Arc<AtomicUsize>,
    received: Arc<std::sync::Mutex<Vec<Value>>>,
}
async fn actual(State(api): State<Api>, bytes: axum::body::Bytes) -> Response {
    api.hits.fetch_add(1, Ordering::SeqCst);
    api.received
        .lock()
        .unwrap()
        .push(serde_json::from_slice(&bytes).unwrap());
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<Value> {
        let store = CrawlStore::open_read_only(&api.path)?;
        let basis = store.sitemap_read_snapshot()?;
        let target = crate::sitemap_findings::AccessTarget::Job {
            job_id: api.id,
            generation: basis.generation.unwrap(),
            config_sha256: basis.crawl_config_sha256.unwrap(),
        };
        let query: ReadQuery = serde_json::from_slice(&bytes)?;
        let prepared = query.prepare(&target)?;
        store.with_verified_sitemap_findings(
            prepared.selection(),
            prepared.request().expected_context_sha256.as_deref(),
            &SitemapReadControl::default(),
            |session| {
                Ok(serde_json::to_value(crate::sitemap_findings::bind_page(
                    &target,
                    &prepared,
                    session.page(prepared.request())?,
                )?)?)
            },
        )
    })
    .await
    .unwrap();
    match result {
        Ok(value) => Response::builder()
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&value).unwrap()))
            .unwrap(),
        Err(error) => {
            let failure = crate::sitemap_findings::typed_failure(error);
            Response::builder()
                .status(crate::sitemap_findings::status(&failure))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&failure).unwrap()))
                .unwrap()
        }
    }
}
async fn provider(
    f: &Fixture,
) -> (
    Bridge,
    Api,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let api = Api {
        path: f.path.clone(),
        id: uuid::Uuid::new_v4(),
        hits: Arc::default(),
        received: Arc::default(),
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bridge = Bridge::new(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let router = Router::new()
        .route("/api/crawls/{id}/sitemaps/findings", post(actual))
        .with_state(api.clone());
    let (stop, stopped) = oneshot::channel();
    let join = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    (bridge, api, stop, join)
}
fn request(id: Value, args: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":NAME,"arguments":args}})
}

#[tokio::test]
async fn findings_mcp_actual_store_http_page_export_descriptor_and_all_seven_filters() {
    let f = Fixture::actual(12).await;
    assert!(f.origin.starts_with("http://127.0.0.1:"));
    assert!(f.dir.0.is_dir());
    let (bridge, api, stop, join) = provider(&f).await;
    let before = std::fs::read(&f.path).unwrap();
    let response=bridge.dispatch(request(json!(1),json!({"id":api.id,"filter":"urls-not-in-sitemap","limit":1,"include_export_descriptor":true}))).await.unwrap();
    assert_eq!(response["result"]["isError"], false, "{response}");
    let data = &response["result"]["structuredContent"];
    assert_eq!(data["findings"]["rows"].as_array().unwrap().len(), 1);
    assert!(data["next_cursor"].is_string());
    let descriptor = &data["export_descriptor"];
    assert_eq!(descriptor["method"], "POST");
    assert_eq!(
        descriptor["path"],
        format!("/api/crawls/{}/sitemaps/findings/export.csv", api.id)
    );
    assert_eq!(descriptor["body"]["snapshot"], data["export_token"]);
    assert_eq!(descriptor["body"]["filter"], "urls-not-in-sitemap");
    assert!(descriptor["body"].get("cursor").is_none());
    let concise = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(concise.len() < 200 && !concise.contains("p000"));
    let second=bridge.dispatch(request(json!(2),json!({"id":api.id,"filter":"urls-not-in-sitemap","limit":2,"cursor":data["next_cursor"]}))).await.unwrap();
    assert_eq!(second["result"]["isError"], false, "{second}");
    for filter in [
        "urls-in-sitemap",
        "urls-not-in-sitemap",
        "orphan-urls",
        "non-indexable-urls-in-sitemap",
        "urls-in-multiple-sitemaps",
        "xml-sitemap-with-over-50k-urls",
        "xml-sitemap-over-50mb",
    ] {
        let response = bridge
            .dispatch(request(
                json!(3),
                json!({"id":api.id,"filter":filter,"assessment":"all","limit":2}),
            ))
            .await
            .unwrap();
        assert_eq!(response["result"]["isError"], false, "{filter}: {response}");
        assert_eq!(
            response["result"]["structuredContent"]["website_requests"],
            0
        );
    }
    assert_eq!(api.hits.load(Ordering::SeqCst), 9);
    assert_eq!(
        api.received.lock().unwrap()[0]["filter"],
        "urls-not-in-sitemap"
    );
    assert_eq!(std::fs::read(&f.path).unwrap(), before);
    stop.send(()).unwrap();
    join.await.unwrap();
    f.finish().await;
}

#[tokio::test]
async fn findings_mcp_strict_schema_types_domains_cursor_and_rpc_id_fail_before_io() {
    let f = Fixture::actual(2).await;
    let (bridge, api, stop, join) = provider(&f).await;
    for extra in [
        json!({"limit":null}),
        json!({"limit":1.5}),
        json!({"candidate_limit":5001}),
        json!({"requested_url":false}),
        json!({"filter":"no-such-filter"}),
        json!({"filter":"orphan-urls","source_task_url":"x"}),
        json!({"filter":"xml-sitemap-over-50mb","requested_url":"x"}),
        json!({"cursor":"bad-token"}),
        json!({"include_export_descriptor":"true"}),
        json!({"unexpected":1}),
    ] {
        let mut args = extra;
        args["id"] = json!(api.id);
        let response = bridge.dispatch(request(json!(1), args)).await.unwrap();
        assert!(
            response.get("error").is_some() || response["result"]["isError"] == true,
            "{response}"
        );
    }
    for id in [
        Value::Null,
        json!({"nested":1}),
        json!(1.5),
        json!("x".repeat(1025)),
    ] {
        let response = bridge
            .dispatch(request(id, json!({"id":api.id})))
            .await
            .unwrap();
        assert_eq!(response["error"]["code"], -32600);
        assert!(response["id"].is_null());
    }
    for id in [json!(7), json!("x".repeat(1025))] {
        let huge = id.is_string();
        let mut wrong_version = request(id, json!({"id":api.id}));
        wrong_version["jsonrpc"] = json!("wrong-version");
        let response = bridge.dispatch(wrong_version).await.unwrap();
        assert_eq!(response["error"]["code"], -32600);
        if huge {
            assert!(response["id"].is_null());
            assert!(serde_json::to_vec(&response).unwrap().len() < 8192);
        } else {
            assert_eq!(response["id"], 7);
        }
    }
    assert_eq!(api.hits.load(Ordering::SeqCst), 0);
    let schema = definition();
    assert_eq!(schema["annotations"]["readOnlyHint"], true);
    assert_eq!(schema["inputSchema"]["additionalProperties"], false);
    stop.send(()).unwrap();
    join.await.unwrap();
    f.finish().await;
}

#[tokio::test]
async fn findings_mcp_actual_stale_api_error_preserves_status_and_typed_error_once() {
    let f = Fixture::actual(3).await;
    let (bridge, api, stop, join) = provider(&f).await;
    let first = bridge
        .dispatch(request(
            json!(1),
            json!({"id":api.id,"filter":"urls-not-in-sitemap","limit":1}),
        ))
        .await
        .unwrap();
    let cursor = first["result"]["structuredContent"]["next_cursor"].clone();
    assert!(cursor.is_string());
    rusqlite::Connection::open(&f.path)
        .unwrap()
        .execute("DELETE FROM metadata WHERE key='checkpoint'", [])
        .unwrap();
    let response = bridge
        .dispatch(request(
            json!(2),
            json!({"id":api.id,"filter":"urls-not-in-sitemap","cursor":cursor}),
        ))
        .await
        .unwrap();
    assert_eq!(response["result"]["isError"], true);
    let data = &response["result"]["structuredContent"];
    assert_eq!(data["status"], 409);
    assert_eq!(data["api_error"]["code"], "stale_sitemap_findings_cursor");
    assert!(
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .len()
            < 200
    );
    stop.send(()).unwrap();
    join.await.unwrap();
    f.finish().await;
}

#[tokio::test]
async fn findings_mcp_entire_wire_wrapper_accepts_exact_eight_mib_and_rejects_plus_one() {
    // Deliberate primitive boundary test, not a fabricated successful Store proof.
    let id = json!("i".repeat(1024));
    let empty = complete_response(
        id.clone(),
        ToolResult {
            value: json!({"padding":""}),
            failed: false,
        },
    )
    .await;
    let overhead = serde_json::to_vec(&empty).unwrap().len();
    let padding = "x".repeat(MAX_WIRE_JSON_BYTES - overhead);
    let exact = complete_response(
        id.clone(),
        ToolResult {
            value: json!({"padding":padding}),
            failed: false,
        },
    )
    .await;
    assert_eq!(
        serde_json::to_vec(&exact).unwrap().len() + 1,
        MAX_JSON_BYTES
    );
    assert!(exact.get("result").is_some());
    let too_big = complete_response(
        id,
        ToolResult {
            value: json!({"padding":"x".repeat(MAX_WIRE_JSON_BYTES-overhead+1)}),
            failed: false,
        },
    )
    .await;
    assert_eq!(too_big["error"]["code"], -32603);
    assert!(serde_json::to_vec(&too_big).unwrap().len() < 8192);
}

#[tokio::test]
async fn findings_mcp_actual_http_body_inclusive_budget_and_bounded_error_status() {
    let f = Fixture::actual(2).await;
    let (bridge, api, stop, join) = provider(&f).await;
    let real = call(&bridge, json!({"id":api.id})).await;
    assert!(!real.failed);
    let actual_envelope = serde_json::to_vec(&real.value).unwrap();
    for (status, cap, oversize) in [
        (200, MAX_JSON_BYTES, false),
        (200, MAX_JSON_BYTES, true),
        (409, MAX_ERROR_BYTES, false),
        (409, MAX_ERROR_BYTES, true),
    ] {
        let mut bytes = if status == 200 {
            actual_envelope.clone()
        } else {
            br#"{"code":"actual_error_fixture","website_requests":0,"error":"bounded status witness"}"#.to_vec()
        };
        bytes.resize(cap + usize::from(oversize), b' ');
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let peer = Bridge::new(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let response = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut received = Vec::new();
            let mut chunk = [0; 4096];
            loop {
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                received.extend_from_slice(&chunk[..n]);
                assert!(received.len() < 65536);
                if let Some(end) = received.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = std::str::from_utf8(&received[..end]).unwrap();
                    let length: usize = head
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse().unwrap())
                        })
                        .unwrap();
                    if received.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let header = format!(
                "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            );
            socket.write_all(header.as_bytes()).await.unwrap();
            let result = socket.write_all(&bytes).await;
            if !oversize {
                result.unwrap();
            } // Budget rejection may close an unread suffix.
        });
        let result = tokio::time::timeout(WAIT, call(&peer, json!({"id":api.id})))
            .await
            .unwrap();
        if oversize {
            assert!(result.failed);
            assert_eq!(
                result.value["code"],
                "sitemap_findings_response_byte_budget"
            );
            assert_eq!(result.value["status"], 413);
        } else if status == 200 {
            assert!(!result.failed);
            assert_eq!(result.value, real.value);
        } else {
            assert!(result.failed);
            assert_eq!(result.value["status"], 409);
            assert_eq!(result.value["api_error"]["code"], "actual_error_fixture");
        }
        tokio::time::timeout(WAIT, response).await.unwrap().unwrap();
    }
    stop.send(()).unwrap();
    join.await.unwrap();
    f.finish().await;
}

struct Held {
    bridge: Bridge,
    reached: Arc<Notify>,
    release: Arc<Notify>,
    closed: oneshot::Receiver<()>,
    task: tokio::task::JoinHandle<()>,
}
async fn held_api(reply: bool) -> Held {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bridge = Bridge::new(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let reached = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let a = reached.clone();
    let b = release.clone();
    let (done, closed) = oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        let mut chunk = [0; 4096];
        loop {
            let n = socket.read(&mut chunk).await.unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&chunk[..n]);
            assert!(bytes.len() < 65536);
            if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let head = std::str::from_utf8(&bytes[..end]).unwrap();
                let length: usize = head
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|n| n.trim().parse().unwrap())
                    })
                    .unwrap();
                if bytes.len() >= end + 4 + length {
                    break;
                }
            }
        }
        a.notify_one();
        if reply {
            b.notified().await;
            let body =
                b"{\"error\":\"held response\",\"code\":\"held_test\",\"website_requests\":0}";
            let head = format!(
                "HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            socket.write_all(head.as_bytes()).await.unwrap();
            socket.write_all(body).await.unwrap();
            socket.shutdown().await.unwrap();
        } else {
            // Actual TCP peer EOF witnesses the dropped reqwest operation.
            assert_eq!(
                tokio::time::timeout(WAIT, socket.read(&mut chunk))
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );
        }
        let _ = done.send(());
    });
    Held {
        bridge,
        reached,
        release,
        closed,
        task,
    }
}
type StdWriter = tokio::io::WriteHalf<tokio::io::DuplexStream>;
type StdReader = BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>;
fn stdio(
    bridge: Bridge,
) -> (
    StdWriter,
    StdReader,
    tokio::task::JoinHandle<anyhow::Result<()>>,
) {
    let (client, server) = tokio::io::duplex(16384);
    let (read, write) = tokio::io::split(client);
    let (input, output) = tokio::io::split(server);
    let task = tokio::spawn(async move { crate::mcp::stdio_stream(&bridge, input, output).await });
    (write, BufReader::new(read), task)
}
async fn send(writer: &mut StdWriter, value: Value) {
    writer
        .write_all(&serde_json::to_vec(&value).unwrap())
        .await
        .unwrap();
    writer.write_all(b"\n").await.unwrap();
}
async fn next(reader: &mut StdReader) -> Value {
    let mut line = Vec::new();
    assert!(
        tokio::time::timeout(WAIT, reader.read_until(b'\n', &mut line))
            .await
            .unwrap()
            .unwrap()
            > 0
    );
    serde_json::from_slice(&line).unwrap()
}
fn active() -> Value {
    request(json!(7), json!({"id":uuid::Uuid::nil()}))
}

#[tokio::test]
async fn findings_stdio_cancel_after_pending_and_busy_still_drops_actual_http_future() {
    let held = held_api(false).await;
    let (mut writer, mut reader, server) = stdio(held.bridge.clone());
    send(&mut writer, active()).await;
    tokio::time::timeout(WAIT, held.reached.notified())
        .await
        .unwrap();
    send(&mut writer, json!({"jsonrpc":"2.0","id":8,"method":"ping"})).await;
    send(&mut writer, json!({"jsonrpc":"2.0","id":9,"method":"ping"})).await;
    let busy = next(&mut reader).await;
    assert_eq!(busy["id"], 9);
    assert_eq!(busy["error"]["code"], -32000);
    send(
        &mut writer,
        json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":"7"}}),
    )
    .await;
    send(
        &mut writer,
        json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":7}}),
    )
    .await;
    let cancelled = next(&mut reader).await;
    assert_eq!(cancelled["id"], 7);
    assert_eq!(cancelled["error"]["code"], -32800);
    let pending = next(&mut reader).await;
    assert_eq!(pending["id"], 8);
    assert_eq!(pending["result"], json!({}));
    tokio::time::timeout(WAIT, held.closed)
        .await
        .unwrap()
        .unwrap();
    held.task.await.unwrap();
    writer.shutdown().await.unwrap();
    tokio::time::timeout(WAIT, server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn findings_stdio_eof_cancels_actual_http_request_without_result_or_detached_worker() {
    let held = held_api(false).await;
    let (mut writer, mut reader, server) = stdio(held.bridge.clone());
    send(&mut writer, active()).await;
    tokio::time::timeout(WAIT, held.reached.notified())
        .await
        .unwrap();
    writer.shutdown().await.unwrap();
    tokio::time::timeout(WAIT, server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    tokio::time::timeout(WAIT, held.closed)
        .await
        .unwrap()
        .unwrap();
    held.task.await.unwrap();
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await.unwrap();
    assert!(bytes.is_empty());
}

#[tokio::test]
async fn findings_stdio_partial_following_frame_survives_actual_response_and_replays_once() {
    let held = held_api(true).await;
    let (mut writer, mut reader, server) = stdio(held.bridge.clone());
    send(&mut writer, active()).await;
    tokio::time::timeout(WAIT, held.reached.notified())
        .await
        .unwrap();
    let ping = br#"{"jsonrpc":"2.0","id":88,"method":"ping"}"#;
    writer.write_all(&ping[..17]).await.unwrap();
    tokio::task::yield_now().await;
    held.release.notify_one();
    let completed = next(&mut reader).await;
    assert_eq!(completed["id"], 7);
    assert_eq!(completed["result"]["isError"], true);
    writer.write_all(&ping[17..]).await.unwrap();
    writer.write_all(b"\r\n").await.unwrap();
    let following = next(&mut reader).await;
    assert_eq!(following["id"], 88);
    assert_eq!(following["result"], json!({}));
    writer.shutdown().await.unwrap();
    tokio::time::timeout(WAIT, server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    held.task.await.unwrap();
}

#[tokio::test]
async fn findings_stdio_actual_active_oversize_open_producer_closes_without_suffix_dispatch() {
    let held = held_api(false).await;
    let (mut writer, mut reader, server) = stdio(held.bridge.clone());
    send(&mut writer, active()).await;
    tokio::time::timeout(WAIT, held.reached.notified())
        .await
        .unwrap();
    let oversized = vec![b' '; crate::mcp::MAX_STDIO_REQUEST_BYTES + 1];
    writer.write_all(&oversized).await.unwrap(); // Keep stdin open, no delimiter.
    let error = next(&mut reader).await;
    assert_eq!(error["error"]["code"], -32600);
    assert!(error["id"].is_null());
    tokio::time::timeout(WAIT, server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    tokio::time::timeout(WAIT, held.closed)
        .await
        .unwrap()
        .unwrap();
    held.task.await.unwrap();
    assert!(
        writer
            .write_all(b"\n{\"jsonrpc\":\"2.0\",\"id\":99,\"method\":\"ping\"}\n")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn findings_stdio_cancelled_frame_read_preserves_actual_prefix_and_remaining_sentinel_cap() {
    for oversized in [false, true] {
        let (mut producer, consumer) = tokio::io::duplex(16384);
        let mut input = BufReader::new(consumer);
        let mut partial = Vec::new();
        let mut physical =
            serde_json::to_vec(&json!({"jsonrpc":"2.0","id":41,"method":"ping"})).unwrap();
        if oversized {
            physical.resize(crate::mcp::MAX_STDIO_REQUEST_BYTES + 1, b' ');
        } else {
            physical.resize(crate::mcp::MAX_STDIO_REQUEST_BYTES - 1, b' ');
            physical.push(b'\n');
        }
        let prefix = &physical[..17];
        producer.write_all(prefix).await.unwrap();
        let mut cancelled = Box::pin(stdio::read_frame(&mut input, &mut partial));
        assert!(futures::poll!(&mut cancelled).is_pending());
        drop(cancelled);
        assert_eq!(
            partial, prefix,
            "actual successful reads must survive cancelled future"
        );
        let suffix = physical[17..].to_vec();
        let writing = tokio::spawn(async move {
            producer.write_all(&suffix).await.unwrap();
            producer
        });
        let frame = tokio::time::timeout(WAIT, stdio::read_frame(&mut input, &mut partial))
            .await
            .unwrap()
            .unwrap();
        if oversized {
            assert!(matches!(frame, stdio::Frame::Oversized));
        } else {
            let stdio::Frame::Complete(bytes) = frame else {
                panic!("exact physical frame")
            };
            assert_eq!(bytes, physical);
        }
        // Producer remains open: a real +1 sentinel, not EOF, ends the read.
        let producer = tokio::time::timeout(WAIT, writing).await.unwrap().unwrap();
        drop(producer);
    }
}

#[path = "postfix_tests.rs"]
mod postfix_tests;
