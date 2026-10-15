use super::*;
use axum::{
    Json,
    body::to_bytes,
    extract::{Path, State},
};
use std::{
    collections::HashMap,
    sync::{
        Condvar,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::RwLock,
    time::{Duration, timeout},
};
use uuid::Uuid;

fn registry() -> Registry {
    Registry::new(TaskAdmission::default())
}
async fn stats_when(registry: &Registry, predicate: impl Fn(&Stats) -> bool) -> Stats {
    timeout(Duration::from_secs(15), async {
        loop {
            let changed = registry.0.counters.changed.notified();
            let stats = registry.stats();
            if predicate(&stats) {
                return stats;
            }
            changed.await;
        }
    })
    .await
    .expect("Actual response operation marker was not observed")
}
async fn drain(registry: &Registry) {
    timeout(Duration::from_secs(15), registry.shutdown_and_drain())
        .await
        .expect("Actual response drain did not finish")
        .expect("Actual response drain failed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn response_single_large_write_preserves_all_bytes_in_bounded_chunks_and_joined_eof() {
    let registry = registry();
    let expected: Vec<u8> = (0..2 * 1024 * 1024 + 17).map(|n| (n % 251) as u8).collect();
    let source = expected.clone();
    let pending = registry
        .start(move |writer, ready| {
            ready.send(73u64)?;
            writer.write_all(&source)?;
            writer.flush()?;
            Ok(())
        })
        .await
        .unwrap();
    let (metadata, body) = pending.ready().await.unwrap();
    assert_eq!(metadata, 73);
    let bytes = timeout(Duration::from_secs(15), to_bytes(body, 3 * 1024 * 1024))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes.as_ref(), expected.as_slice());
    drain(&registry).await;
    let stats = registry.stats();
    assert_eq!(
        (
            stats.admitted,
            stats.started,
            stats.joined,
            stats.response_supervisors_joined,
            stats.retained
        ),
        (1, 1, 1, 1, 0)
    );
    assert_eq!((stats.complete, stats.aborted, stats.failed), (1, 0, 0));
    assert_eq!(stats.enqueues, expected.len().div_ceil(CHUNK_BYTES) as u64);
    assert_eq!(stats.max_chunk, CHUNK_BYTES);
    assert!(stats.peak_queued <= QUEUE_CHUNKS);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn response_actual_eight_enqueues_and_ninth_full_reservation_survive_cancelled_drain_until_body_drop()
 {
    let registry = registry();
    let pending = registry
        .start(|writer, ready| {
            ready.send(())?;
            writer.write_all(&vec![b'x'; CHUNK_BYTES * 9 + 7])?;
            Ok(())
        })
        .await
        .unwrap();
    let (_, body) = pending.ready().await.unwrap();
    let stats = stats_when(&registry, |s| s.enqueues == 8 && s.full_reservations >= 1).await;
    assert_eq!(stats.peak_queued, 8);
    assert_eq!(stats.max_chunk, CHUNK_BYTES);
    assert_eq!(stats.joined, 0);
    let first = registry.clone();
    let mut waiter = tokio::spawn(async move { first.shutdown_and_drain().await });
    timeout(Duration::from_secs(5), async {
        while !registry.0.workers.snapshot().closing {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        timeout(Duration::from_millis(30), &mut waiter)
            .await
            .is_err(),
        "Held full real queue cannot be declared drained"
    );
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    assert_eq!(
        registry.stats().joined,
        0,
        "Cancelling a drain waiter must not detach the blocked actual worker"
    );
    drop(body);
    drain(&registry).await;
    let stats = registry.stats();
    assert_eq!(
        (
            stats.joined,
            stats.response_supervisors_joined,
            stats.retained
        ),
        (1, 1, 0)
    );
    assert_eq!((stats.complete, stats.aborted), (0, 1));
    assert_eq!(stats.enqueues, 8);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn response_explicit_cancel_wakes_full_reserve_but_unpolled_body_still_requires_actual_terminal()
 {
    let registry = registry();
    let pending = registry
        .start(|writer, ready| {
            ready.send(())?;
            writer.write_all(&vec![b'y'; CHUNK_BYTES * 10])?;
            Ok(())
        })
        .await
        .unwrap();
    let (_, body) = pending.ready().await.unwrap();
    stats_when(&registry, |s| s.enqueues == 8 && s.full_reservations >= 1).await;
    registry.cancel_all();
    let stats = stats_when(&registry, |s| s.joined == 1).await;
    assert_eq!(stats.complete, 0);
    assert_eq!(
        stats.aborted, 0,
        "Cancellation request alone is not actual Body terminal"
    );
    assert_eq!(stats.retained, 1);
    let bytes = timeout(Duration::from_secs(5), to_bytes(body, CHUNK_BYTES * 10))
        .await
        .unwrap();
    assert!(
        bytes.is_err(),
        "Cancellation cannot become normal short200 EOF"
    );
    drain(&registry).await;
    let stats = registry.stats();
    assert_eq!((stats.complete, stats.aborted, stats.retained), (0, 1, 0));
}

struct Release(Arc<(Mutex<bool>, Condvar)>);
impl Release {
    fn release(&self) {
        let (flag, changed) = &*self.0;
        *lock(flag) = true;
        changed.notify_all();
    }
}
impl Drop for Release {
    fn drop(&mut self) {
        self.release();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn response_pending_caller_drop_keeps_actual_preparation_owned_until_worker_and_supervisor_join()
 {
    let registry = registry();
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let release = Release(gate.clone());
    let (entered, observed) = oneshot::channel();
    let pending = registry
        .start(move |writer, ready| {
            entered.send(()).unwrap();
            let (flag, changed) = &*gate;
            let mut flag = lock(flag);
            while !*flag {
                flag = changed.wait(flag).unwrap();
            }
            drop(flag);
            ready.send(())?;
            writer.write_all(b"must not escape cancelled preparation")?;
            Ok(())
        })
        .await
        .unwrap();
    timeout(Duration::from_secs(5), observed)
        .await
        .unwrap()
        .unwrap();
    drop(pending);
    let owned = registry.clone();
    let mut waiter = tokio::spawn(async move { owned.shutdown_and_drain().await });
    assert!(
        timeout(Duration::from_millis(30), &mut waiter)
            .await
            .is_err()
    );
    assert_eq!(
        (
            registry.stats().started,
            registry.stats().joined,
            registry.stats().retained
        ),
        (1, 0, 1)
    );
    release.release();
    timeout(Duration::from_secs(5), waiter)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            registry.stats().joined,
            registry.stats().response_supervisors_joined,
            registry.stats().retained
        ),
        (1, 1, 0)
    );
    assert_eq!(registry.stats().enqueues, 0);
    assert_eq!(registry.stats().complete, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn response_actual_worker_panic_before_ready_is_sticky_after_all_actual_joins() {
    let registry = registry();
    let pending = registry
        .start::<(), _>(|_, _| {
            panic!("Actual response blocking worker panic before metadata");
        })
        .await
        .unwrap();
    let error = match timeout(Duration::from_secs(5), pending.ready())
        .await
        .unwrap()
    {
        Err(error) => error,
        Ok(_) => panic!("Actual panic cannot become metadata success"),
    };
    assert!(
        error.readiness_lost(),
        "Lost readiness preserves the existing500 error class"
    );
    let result = timeout(Duration::from_secs(5), registry.shutdown_and_drain())
        .await
        .unwrap();
    assert!(result.is_err());
    let stats = registry.stats();
    assert_eq!(
        (
            stats.admitted,
            stats.joined,
            stats.response_supervisors_joined,
            stats.retained
        ),
        (1, 1, 1, 0)
    );
    assert!(stats.first_failure.is_some());
    assert_eq!(stats.complete, 0);
}

#[tokio::test]
async fn response_registry_uses_the_same_global_admission_before_any_worker_registration() {
    let admission = TaskAdmission::default();
    let registry = Registry::new(admission.clone());
    let calls = Arc::new(AtomicUsize::new(0));
    let ran = calls.clone();
    admission.close();
    assert!(
        registry
            .start::<(), _>(move |_, _| {
                ran.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await
            .is_err()
    );
    drain(&registry).await;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        (
            registry.stats().admitted,
            registry.stats().started,
            registry.stats().joined,
            registry.stats().retained
        ),
        (0, 0, 0, 0)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn response_post_ready_panic_or_error_after_valid_json_never_becomes_successful_eof() {
    for panic in [false, true] {
        let registry = registry();
        let pending = registry
            .start(move |writer, ready| {
                ready.send(())?;
                writer.write_all(b"{\"all_snapshot_rows_exported\":true}")?;
                if panic {
                    panic!("Actual producer panic after valid JSON prefix");
                }
                anyhow::bail!("Actual producer rejected final verification");
            })
            .await
            .unwrap();
        let (_, body) = pending.ready().await.unwrap();
        assert!(
            timeout(Duration::from_secs(5), to_bytes(body, 65536))
                .await
                .unwrap()
                .is_err(),
            "Valid JSON bytes alone do not prove actual joined success"
        );
        let result = timeout(Duration::from_secs(5), registry.shutdown_and_drain())
            .await
            .unwrap();
        assert_eq!(
            result.is_err(),
            panic,
            "Domain export errors differ from actual worker join failures"
        );
        let stats = registry.stats();
        assert_eq!(stats.complete, 0);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.retained, 0);
        assert_eq!(stats.response_supervisors_joined, 1);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn response_missing_metadata_or_joined_completion_is_failure_and_never_empty_success() {
    let registry = registry();
    let pending = registry.start::<(), _>(|_, _| Ok(())).await.unwrap();
    assert!(pending.ready().await.is_err());
    drain(&registry).await;
    assert_eq!(registry.stats().complete, 0);
    let counters = Arc::new(Counters::default());
    let control = Arc::new(Control::new(counters.clone()));
    let (sender, receiver) = mpsc::channel(8);
    sender.send(Bytes::from_static(b"prefix")).await.unwrap();
    drop(sender);
    let (completion, joined) = oneshot::channel();
    drop(completion);
    let body = Body::from_stream(BodyStream {
        receiver,
        joined,
        control,
        finished: false,
    });
    assert!(
        to_bytes(body, 65536).await.is_err(),
        "A missing actual-join completion cannot be promoted to normal EOF"
    );
    assert_eq!(counters.complete.load(Ordering::SeqCst), 0);
    assert_eq!(counters.failed.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn response_positive_eof_commit_is_not_rewritten_by_late_cancel_and_closed_admission_starts_nothing()
 {
    let registry = registry();
    let pending = registry
        .start(|writer, ready| {
            ready.send(())?;
            writer.write_all(b"complete")?;
            Ok(())
        })
        .await
        .unwrap();
    let (_, body) = pending.ready().await.unwrap();
    assert_eq!(to_bytes(body, 65536).await.unwrap().as_ref(), b"complete");
    registry.cancel_all();
    drain(&registry).await;
    assert_eq!(
        (registry.stats().complete, registry.stats().aborted),
        (1, 0)
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let ran = calls.clone();
    assert!(
        registry
            .start::<(), _>(move |_, _| {
                ran.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(registry.stats().admitted, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn response_completed_slots_are_reaped_before_repeated_download_registration() {
    let registry = registry();
    for n in 0..50 {
        let pending = registry
            .start(move |writer, ready| {
                ready.send(n)?;
                writer.write_all(b"one")?;
                Ok(())
            })
            .await
            .unwrap();
        let (actual, body) = pending.ready().await.unwrap();
        assert_eq!(actual, n);
        assert_eq!(to_bytes(body, 100).await.unwrap().as_ref(), b"one");
        stats_when(&registry, |s| s.joined == n + 1).await;
        // Complete supervisor publication before checking the actual explicit reaper.
        timeout(Duration::from_secs(5), async {
            loop {
                registry.reap_completed().await.unwrap();
                if registry.stats().retained == 0 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(registry.stats().retained, 0);
    }
    drain(&registry).await;
    assert_eq!(registry.stats().response_supervisors_joined, 50);
}

async fn actual_ledger_state() -> (
    tempfile::TempDir,
    crate::server::AppState,
    Uuid,
    oneshot::Sender<()>,
    JoinHandle<()>,
    Arc<AtomicUsize>,
) {
    use crate::server::{AppState, create_crawl};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let mut html =
        String::from("<!doctype html><html><head><title>Owned response</title></head><body>");
    for n in 0..700 {
        html.push_str(&format!(
            "<a href=\"/denied/{n:04}?padding={}\">row {n}</a>",
            "x".repeat(1024)
        ));
    }
    html.push_str("</body></html>");
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let (stop, mut stopped) = oneshot::channel();
    let website = tokio::spawn(async move {
        loop {
            let mut socket =
                tokio::select! {result=listener.accept()=>result.unwrap().0,_=&mut stopped=>break};
            observed.fetch_add(1, Ordering::SeqCst);
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            while !request.windows(4).any(|v| v == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buffer[..n]);
            }
            assert!(request.starts_with(b"GET /seed "));
            socket.write_all(format!("HTTP/1.1 200 Fixture\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",html.len()).as_bytes()).await.unwrap();
            socket.write_all(html.as_bytes()).await.unwrap();
        }
    });
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    tokio::fs::create_dir(&data).await.unwrap();
    let lifecycle = crate::server::lifecycle::Lifecycle::default();
    let mut mcp = crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap();
    mcp.set_task_owner(lifecycle.blocking.clone());
    let state = AppState {
        mcp,
        jobs: Arc::new(RwLock::new(HashMap::new())),
        data_dir: Arc::new(data),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle,
    };
    let Json(started)=create_crawl(State(state.clone()),Json(serde_json::json!({"seed_urls":[format!("{origin}/seed")],"obey_robots":false,"discover_sitemaps":false,"crawl_links":true,"concurrency":1,"delay_ms":0,"request_timeout_secs":5,"crawl_policy":{"scope":{"roots":[{"origin":origin,"folders":["/"]}]},"exclude_patterns":["/denied/"]},"resource_discovery":{}}))).await.unwrap_or_else(|(status,Json(error))|panic!("Actual fixture crawl startup {status}: {}",error.error));
    let id = Uuid::parse_str(started["id"].as_str().unwrap()).unwrap();
    timeout(Duration::from_secs(30), async {
        loop {
            let job = state.jobs.read().await.get(&id).unwrap().clone();
            let guard = job.read().await;
            if !matches!(guard.status.as_str(), "running" | "pausing") {
                assert_eq!(
                    guard.status,
                    "completed",
                    "{}",
                    guard.error.as_deref().unwrap_or("")
                );
                break;
            }
            drop(guard);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    (temp, state, id, stop, website, calls)
}

async fn actual_export(
    state: crate::server::AppState,
    id: Uuid,
    resource: bool,
    csv: bool,
) -> Response {
    let result = match (resource, csv) {
        (false, false) => crate::server::policy::export_json(State(state), Path(id)).await,
        (false, true) => crate::server::policy::export_csv(State(state), Path(id)).await,
        (true, false) => crate::server::resources::export_json(State(state), Path(id)).await,
        (true, true) => crate::server::resources::export_csv(State(state), Path(id)).await,
    };
    result.unwrap_or_else(|(status, Json(error))| {
        panic!("Actual export handler {status}: {}", error.error)
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn response_actual_sqlite_policy_resource_handlers_reach_eight_chunks_then_drop_and_recover_exact_full_exports()
 {
    let (_temp, state, id, stop, website, calls) = actual_ledger_state().await;
    for resource in [false, true] {
        for csv in [false, true] {
            let old = state.lifecycle.response_streams.stats();
            let response = actual_export(state.clone(), id, resource, csv).await;
            assert_eq!(response.status(), StatusCode::OK);
            let count_header = if resource {
                "x-resource-snapshot-observations"
            } else {
                "x-policy-snapshot-occurrences"
            };
            let expected: usize = response.headers()[count_header]
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            assert!(expected >= 700);
            stats_when(&state.lifecycle.response_streams, |s| {
                s.enqueues == old.enqueues + 8 && s.full_reservations > old.full_reservations
            })
            .await;
            drop(response);
            stats_when(&state.lifecycle.response_streams, |s| {
                s.joined == old.joined + 1
            })
            .await;
            timeout(Duration::from_secs(5), async {
                loop {
                    state
                        .lifecycle
                        .response_streams
                        .reap_completed()
                        .await
                        .unwrap();
                    if state.lifecycle.response_streams.stats().retained == 0 {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            let response = actual_export(state.clone(), id, resource, csv).await;
            let bytes = timeout(
                Duration::from_secs(20),
                to_bytes(response.into_body(), 8 * 1024 * 1024),
            )
            .await
            .unwrap()
            .unwrap();
            assert!(bytes.len() > 8 * CHUNK_BYTES);
            if csv {
                let text = std::str::from_utf8(&bytes).unwrap();
                assert_eq!(text.lines().count(), expected + 1);
                assert!(text.ends_with('\n'));
                assert!(text.contains("padding="));
            } else {
                let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(value["observations"].as_array().unwrap().len(), expected);
                assert_eq!(value["export"]["row_count"], serde_json::json!(expected));
                assert_eq!(value["export"]["all_snapshot_rows_exported"], true);
            }
        }
    }
    crate::server::lifecycle::close_admission(&state);
    timeout(
        Duration::from_secs(15),
        crate::server::lifecycle::drain(&state),
    )
    .await
    .unwrap()
    .unwrap();
    let stats = state.lifecycle.response_streams.stats();
    assert_eq!(
        (
            stats.admitted,
            stats.joined,
            stats.response_supervisors_joined,
            stats.retained
        ),
        (8, 8, 8, 0)
    );
    assert_eq!((stats.complete, stats.aborted), (4, 4));
    assert_eq!(stats.max_chunk, CHUNK_BYTES);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "Only the actual seed was fetched; all eight exports used read-only SQLite"
    );
    stop.send(()).unwrap();
    website.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn response_http10_refusal_is_only_four_actual_routes_and_head_starts_no_export() {
    use crate::server::lifecycle::ShutdownSource;
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    let (ready, address) = oneshot::channel();
    let (stop, stopped) = oneshot::channel();
    let server = tokio::spawn(crate::server::serve_with_mcp_files_ready_source(
        "127.0.0.1:0",
        data.clone(),
        None,
        Some(ready),
        ShutdownSource::Injected(Box::pin(async move {
            stopped.await?;
            Ok("response_http10_test")
        })),
    ));
    let address = timeout(Duration::from_secs(5), address)
        .await
        .unwrap()
        .unwrap();
    let id = Uuid::nil();
    for suffix in [
        "policy.json",
        "policy.csv",
        "resources.json",
        "resources.csv",
    ] {
        for method in ["GET", "HEAD"] {
            let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
            socket
                .write_all(
                    format!(
                        "{method} /api/crawls/{id}/{suffix} HTTP/1.0\r\nHost: {address}\r\n\r\n"
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            let mut bytes = Vec::new();
            timeout(Duration::from_secs(5), socket.read_to_end(&mut bytes))
                .await
                .unwrap()
                .unwrap();
            let text = String::from_utf8(bytes).unwrap();
            assert_eq!(
                text.lines().next().unwrap().split_whitespace().nth(1),
                Some("505"),
                "{text}"
            );
            assert!(text.to_ascii_lowercase().contains("connection: close"));
            assert!(text.to_ascii_lowercase().contains("content-length:"));
            if method == "GET" {
                assert!(text.contains("This streamed download requires HTTP/1.1"));
            } else {
                assert!(
                    text.ends_with("\r\n\r\n"),
                    "HEAD cannot start or emit an export body"
                );
            }
        }
    }
    let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
    socket
        .write_all(format!("GET /healthz HTTP/1.0\r\nHost: {address}\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let mut health = Vec::new();
    timeout(Duration::from_secs(5), socket.read_to_end(&mut health))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        std::str::from_utf8(&health)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1),
        Some("200")
    );
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    for suffix in [
        "policy.json",
        "policy.csv",
        "resources.json",
        "resources.csv",
    ] {
        let response = client
            .get(format!("http://{address}/api/crawls/{id}/{suffix}"))
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "HTTP1.1 still reaches ordinary job lookup"
        );
    }
    stop.send(()).unwrap();
    timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        std::fs::read_dir(data).unwrap().count(),
        0,
        "Refused HTTP1.0 exports did not create any job or SQLite files"
    );
}
