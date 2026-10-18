//! Authored actual local TCP/owned-registry tests. No native process or Core
//! export-format fidelity is inferred from these deliberately isolated handlers.
use super::super::response_stream::{self, ChannelWriter, Ready};
use super::*;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use futures::StreamExt;
use std::io::Write;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
type Producer = dyn Fn(&mut ChannelWriter, &mut Ready<()>) -> anyhow::Result<()> + Send + Sync;

#[test]
fn incoming_binding_exhaustion_never_wraps_or_discards_an_existing_lease() {
    let connection = Connection::new(Arc::new(Notify::new()));
    connection.0.next.store(u64::MAX - 1, Ordering::SeqCst);
    let lease = connection.bind(Arc::new(|| {})).unwrap();
    assert_eq!(lease.id, u64::MAX - 1);
    assert!(connection.bind(Arc::new(|| {})).is_err());
    assert_eq!(connection.0.next.load(Ordering::SeqCst), u64::MAX);
    assert_eq!(lock(&connection.0.bindings).len(), 1);
    drop(lease);
    assert!(lock(&connection.0.bindings).is_empty());
}

struct Fixture {
    transport: Registry,
    responses: response_stream::Registry,
    requests: TaskOwner,
    admission: TaskAdmission,
    running: Running,
    address: std::net::SocketAddr,
    aborts: Arc<AtomicU64>,
}
async fn fixture(operation: Arc<Producer>) -> Fixture {
    let admission = TaskAdmission::default();
    let transport = Registry::new(admission.clone());
    let responses = response_stream::Registry::new(admission.clone());
    let requests = TaskOwner::with_admission("fixture_requests", admission.clone());
    let handler = responses.clone();
    let router = Router::new().route(
        "/large",
        get(move || {
            let registry = handler.clone();
            let operation = operation.clone();
            async move {
                match registry
                    .start(move |writer, ready| operation(writer, ready))
                    .await
                {
                    Ok(pending) => match pending.ready().await {
                        Ok(((), body)) => Response::new(body),
                        Err(e) => (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()).into_response(),
                    },
                    Err(e) => (StatusCode::SERVICE_UNAVAILABLE, e.to_string()).into_response(),
                }
            }
        }),
    );
    let owner = requests.clone();
    let router = router.layer(middleware::from_fn(move |request: Request, next: Next| {
        let owner = owner.clone();
        let connection = request.extensions().get::<Connection>().cloned();
        async move {
            match owner
                .run_async(scope(connection, async move { next.run(request).await }))
                .await
            {
                Ok(response) => response,
                Err(error) => (StatusCode::SERVICE_UNAVAILABLE, error.to_string()).into_response(),
            }
        }
    }));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    assert!(![8787, 8790].contains(&address.port()));
    let aborts = Arc::new(AtomicU64::new(0));
    let count = aborts.clone();
    let cancel = responses.clone();
    let running = transport
        .start(
            listener,
            router,
            Arc::new(move || {
                count.fetch_add(1, Ordering::SeqCst);
                cancel.cancel_all();
            }),
        )
        .unwrap();
    Fixture {
        transport,
        responses,
        requests,
        admission,
        running,
        address,
        aborts,
    }
}
impl Fixture {
    fn quiesce(&self) {
        let t0 = Instant::now();
        self.admission.close();
        self.responses.close_admission();
        self.transport.quiesce_at(t0);
    }
    async fn drain(&self) {
        let (t, r, q) = tokio::join!(
            self.transport.shutdown_and_drain(),
            self.responses.shutdown_and_drain(),
            self.requests.shutdown_and_drain()
        );
        t.unwrap();
        r.unwrap();
        q.unwrap();
        self.running.wait().await.unwrap();
    }
    async fn raw(&self) -> TcpStream {
        let mut socket = TcpStream::connect(self.address).await.unwrap();
        socket
            .write_all(format!("GET /large HTTP/1.1\r\nHost: {}\r\n\r\n", self.address).as_bytes())
            .await
            .unwrap();
        socket
    }
}
async fn until(mut predicate: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(20), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
fn endless() -> Arc<Producer> {
    Arc::new(|writer, ready| {
        ready.send(())?;
        let bytes = [b'Q'; 65_536];
        for _ in 0..32_768 {
            writer.write_all(&bytes)?;
        }
        Ok(())
    })
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_nonreader_reaches_real_full_queue_and_tcp_pending_then_self_aborts_and_joins() {
    let f = fixture(endless()).await;
    let mut client = f.raw().await;
    // Both markers come from actual failed reserve / AsyncWrite Pending. The
    // client is deliberately neither read nor closed until autonomous drain.
    until(|| {
        f.transport.stats()["tcp_write_pending"].as_u64().unwrap() > 0
            && f.responses.stats().full_reservations > 0
    })
    .await;
    f.quiesce();
    tokio::time::timeout(Duration::from_secs(15), f.drain())
        .await
        .unwrap();
    let stats = f.transport.stats();
    assert_eq!(f.aborts.load(Ordering::SeqCst), 1);
    assert_eq!(stats["retained"], 0);
    assert_eq!(stats["actual_io_dropped"], 1);
    assert_eq!(stats["connection_admitted"], stats["connection_joined"]);
    assert_eq!(stats["connection_retained"], 0);
    assert_eq!(f.responses.stats().complete, 0);
    assert_eq!(f.responses.stats().retained, 0);
    let mut buffer = [0u8; 65_536];
    let mut tail = Vec::new();
    let mut first = Vec::new();
    let mut length = 0u64;
    let mut socket_error = false;
    loop {
        match client.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => {
                length += n as u64;
                assert!(
                    length < 2_147_483_648,
                    "full2GiB producer completion must not masquerade as abort"
                );
                if first.len() < 8192 {
                    first.extend_from_slice(&buffer[..n.min(8192 - first.len())]);
                }
                tail.extend_from_slice(&buffer[..n]);
                if tail.len() > 32 {
                    drop(tail.drain(..tail.len() - 32));
                }
            }
            Err(_) => {
                socket_error = true;
                break;
            }
        }
    }
    assert!(first.starts_with(b"HTTP/1.1 200"));
    assert!(
        socket_error || !tail.ends_with(b"0\r\n\r\n"),
        "aborted chunked response must not receive normal final chunk"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_consuming_response_finishes_inside_shared_grace_with_complete_tail_and_eof() {
    struct ReleaseOnDrop(Option<std::sync::mpsc::Sender<()>>);
    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            if let Some(release) = self.0.take() {
                let _ = release.send(());
            }
        }
    }
    let payload = Arc::new([vec![b'A'; 2 * 1024 * 1024], b"EXACT_COMPLETE_TAIL".to_vec()].concat());
    let expected = payload.clone();
    let (release, held) = std::sync::mpsc::channel();
    let mut release = ReleaseOnDrop(Some(release));
    let held = Arc::new(Mutex::new(held));
    let waiting = Arc::new(AtomicBool::new(false));
    let entered = waiting.clone();
    let finished = Arc::new(AtomicBool::new(false));
    let completed = finished.clone();
    let f = fixture(Arc::new(move |writer, ready| {
        ready.send(())?;
        writer.write_all(&payload[..65_536])?;
        entered.store(true, Ordering::SeqCst);
        lock(&held).recv()?;
        writer.write_all(&payload[65_536..])?;
        completed.store(true, Ordering::SeqCst);
        Ok(())
    }))
    .await;
    let response = reqwest::get(format!("http://{}/large", f.address))
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    until(|| waiting.load(Ordering::SeqCst)).await;
    assert!(!finished.load(Ordering::SeqCst));
    assert_eq!(
        f.responses.stats().joined,
        0,
        "admitted producer is actually held before T0"
    );
    f.quiesce();
    assert!(
        !finished.load(Ordering::SeqCst),
        "the positive work remains unfinished at T0"
    );
    release.0.take().unwrap().send(()).unwrap();
    let reading = async {
        let bytes = response.bytes().await.unwrap();
        assert_eq!(bytes.as_ref(), expected.as_slice());
    };
    let (_, ()) = tokio::join!(f.drain(), reading);
    assert!(finished.load(Ordering::SeqCst));
    assert_eq!(f.aborts.load(Ordering::SeqCst), 0);
    assert_eq!(f.responses.stats().complete, 1);
    assert_eq!(f.responses.stats().aborted, 0);
    assert_eq!(f.responses.stats().failed, 0);
    assert_eq!(f.transport.stats()["shutdown_aborted"], 0);
    assert_eq!(f.transport.stats()["actual_io_dropped"], 1);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_preheaders_io_drop_cancels_lease_but_still_awaits_actual_held_blocking_producer()
{
    struct ReleaseOnDrop(Option<std::sync::mpsc::Sender<()>>);
    impl Drop for ReleaseOnDrop {
        fn drop(&mut self) {
            if let Some(release) = self.0.take() {
                let _ = release.send(());
            }
        }
    }
    let (started, observed) = std::sync::mpsc::channel();
    let (release, blocked) = std::sync::mpsc::channel();
    let mut release = ReleaseOnDrop(Some(release));
    let blocked = Arc::new(Mutex::new(blocked));
    let f = fixture(Arc::new(move |writer, ready| {
        started.send(()).unwrap();
        lock(&blocked).recv().unwrap();
        ready.send(())?;
        writer.write_all(b"must never become successful headers")?;
        Ok(())
    }))
    .await;
    let client = f.raw().await;
    tokio::task::spawn_blocking(move || observed.recv_timeout(Duration::from_secs(5)).unwrap())
        .await
        .unwrap();
    f.quiesce();
    tokio::time::timeout(Duration::from_secs(15), f.transport.shutdown_and_drain())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(f.transport.stats()["actual_io_dropped"], 1);
    assert_eq!(
        f.responses.stats().joined,
        0,
        "socket deadline must not invent completion of the held blocking operation"
    );
    assert_eq!(f.responses.stats().retained, 1);
    assert_eq!(f.responses.stats().complete, 0);
    release.0.take().unwrap().send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), f.drain())
        .await
        .unwrap();
    assert_eq!(f.responses.stats().complete, 0);
    assert_eq!(f.responses.stats().admitted, f.responses.stats().joined);
    drop(client);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_dropped_drain_waiter_retains_actual_coordinator_connection_and_producer_joins() {
    let f = fixture(endless()).await;
    let client = f.raw().await;
    until(|| {
        f.transport.stats()["tcp_write_pending"].as_u64().unwrap() > 0
            && f.responses.stats().full_reservations > 0
    })
    .await;
    f.quiesce();
    let registry = f.transport.clone();
    let (entered, observed) = oneshot::channel();
    let caller = tokio::spawn(async move {
        let draining = registry.shutdown_and_drain();
        tokio::pin!(draining);
        let mut entered = Some(entered);
        std::future::poll_fn(move |cx| {
            let polled = draining.as_mut().poll(cx);
            if polled.is_pending()
                && let Some(entered) = entered.take()
            {
                let _ = entered.send(());
            }
            polled
        })
        .await
    });
    tokio::time::timeout(Duration::from_secs(5), observed)
        .await
        .unwrap()
        .unwrap();
    assert!(!caller.is_finished());
    assert!(
        f.transport.0.coordinator_join.ticket.try_lock().is_err(),
        "the actual suspended JoinSlot waiter owns the ticket mutex before caller cancellation"
    );
    assert_eq!(f.transport.stats()["connection_joined"], 0);
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    // The only cancelled task is this test's waiter. Actual accept/connection
    // workers retain their tickets and their own absolute deadline coordinator.
    tokio::time::timeout(Duration::from_secs(15), f.drain())
        .await
        .unwrap();
    assert_eq!(f.transport.stats()["connection_retained"], 0);
    assert_eq!(f.transport.stats()["accept_retained"], 0);
    assert_eq!(f.transport.stats()["retained"], 0);
    assert_eq!(f.responses.stats().retained, 0);
    assert_eq!(f.responses.stats().admitted, f.responses.stats().joined);
    drop(client);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_idle_reaper_joins_completed_connections_without_next_accept_or_shutdown() {
    let f = fixture(Arc::new(|writer, ready| {
        ready.send(())?;
        writer.write_all(b"complete")?;
        Ok(())
    }))
    .await;
    let mut socket = TcpStream::connect(f.address).await.unwrap();
    socket
        .write_all(
            format!(
                "GET /large HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                f.address
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).await.unwrap();
    assert!(bytes.starts_with(b"HTTP/1.1 200"));
    assert!(bytes.ends_with(b"0\r\n\r\n"));
    until(|| f.transport.stats()["connection_joined"] == 1 && f.transport.stats()["retained"] == 0)
        .await;
    assert_eq!(
        f.transport.stats()["listener_dropped"],
        false,
        "reaping happened while the listener stayed live and without another accept"
    );
    f.quiesce();
    f.drain().await;
    assert_eq!(f.aborts.load(Ordering::SeqCst), 0);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_protocol_errors_and_post_quiesce_connect_do_not_admit_a_late_handler() {
    let calls = Arc::new(AtomicU64::new(0));
    let observed = calls.clone();
    let f = fixture(Arc::new(move |writer, ready| {
        observed.fetch_add(1, Ordering::SeqCst);
        ready.send(())?;
        writer.write_all(b"ok")?;
        Ok(())
    }))
    .await;
    let mut socket = TcpStream::connect(f.address).await.unwrap();
    socket.write_all(b"NOT HTTP\r\n\r\n").await.unwrap();
    let mut bytes = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), socket.read_to_end(&mut bytes))
        .await
        .unwrap();
    drop(socket);
    until(|| f.transport.stats()["connection_joined"] == 1 && f.transport.stats()["retained"] == 0)
        .await;
    assert!(f.transport.stats()["first_failure"].is_null());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    f.quiesce();
    until(|| f.transport.stats()["listener_dropped"] == true).await;
    assert!(TcpStream::connect(f.address).await.is_err());
    f.drain().await;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_private_task_scope_copies_binding_to_owned_work_and_lease_drop_prunes_keepalive()
{
    let connection = Connection::new(Arc::new(Notify::new()));
    let observed = Arc::new(AtomicBool::new(false));
    let cancelled = observed.clone();
    let owner = TaskOwner::new("private_scope_fixture");
    let token = connection.clone();
    let lease = owner
        .run_async(scope(Some(token), async move {
            bind_current(move || cancelled.store(true, Ordering::SeqCst))
                .unwrap()
                .unwrap()
        }))
        .await
        .unwrap();
    assert_eq!(lock(&connection.0.bindings).len(), 1);
    drop(lease);
    assert!(lock(&connection.0.bindings).is_empty());
    connection.terminate();
    assert!(!observed.load(Ordering::SeqCst));
    owner.shutdown_and_drain().await.unwrap();
    let cancelled = observed.clone();
    let late = scope(Some(connection), async move {
        bind_current(move || cancelled.store(true, Ordering::SeqCst))
            .unwrap()
            .unwrap()
    })
    .await;
    assert!(
        observed.load(Ordering::SeqCst),
        "closed connection cancels newly attached preheaders work synchronously"
    );
    drop(late);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_two_nonreaders_share_one_absolute_grace_and_repeated_quiesce_cannot_extend_it() {
    let f = fixture(endless()).await;
    let first = f.raw().await;
    let second = f.raw().await;
    until(|| {
        let state = lock(&f.transport.0.state);
        state.slots.len() == 2
            && state
                .slots
                .values()
                .all(|s| s.control.0.write_pending.load(Ordering::SeqCst) > 0)
            && f.responses.stats().full_reservations >= 2
    })
    .await;
    let t0 = Instant::now();
    f.quiesce();
    let original = *f.transport.0.phase.borrow();
    assert!(matches!(original, Phase::Quiescing(_)));
    f.transport
        .quiesce_at(Instant::now() + Duration::from_secs(30));
    assert!(
        *f.transport.0.phase.borrow() == original,
        "repeated/late drain cannot give any connection a new deadline"
    );
    tokio::time::timeout(Duration::from_secs(8), f.drain())
        .await
        .unwrap();
    assert!(
        t0.elapsed() < Duration::from_secs(8),
        "this small fixture must not receive sequential per-connection five-second waits"
    );
    assert_eq!(f.aborts.load(Ordering::SeqCst), 1);
    assert_eq!(f.transport.stats()["actual_io_dropped"], 2);
    assert_eq!(f.responses.stats().complete, 0);
    assert_eq!(f.responses.stats().admitted, f.responses.stats().joined);
    drop((first, second));
}

// This fixture exercises actual private-spool/pump/transport ownership. Its
// caller-declared manifest does not stand in for Core archive/SourceA attestation.
type BodyPreparation = dyn Fn(
        &mut dyn Write,
        &mut dyn FnMut() -> anyhow::Result<()>,
    ) -> anyhow::Result<open_seo_spider_core::SitemapVerifiedBodyCopy>
    + Send
    + Sync;
struct TransferGate {
    waiting: AtomicBool,
    release: watch::Sender<bool>,
}
impl TransferGate {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            waiting: AtomicBool::new(false),
            release: watch::channel(false).0,
        })
    }
    fn wrap(self: &Arc<Self>, body: axum::body::Body) -> axum::body::Body {
        let gate = self.clone();
        let stream = Box::pin(body.into_data_stream());
        axum::body::Body::from_stream(futures::stream::unfold(
            (stream, false, gate),
            |(mut stream, first_sent, gate)| async move {
                if first_sent {
                    let mut release = gate.release.subscribe();
                    loop {
                        let open = *release.borrow_and_update();
                        if open {
                            break;
                        }
                        gate.waiting.store(true, Ordering::SeqCst);
                        release
                            .changed()
                            .await
                            .expect("Positive transfer gate remains owned");
                    }
                }
                stream
                    .next()
                    .await
                    .map(|value| (value, (stream, true, gate)))
            },
        ))
    }
}
struct BodyFixture {
    transport: Registry,
    bodies: super::super::source_body_stream::Registry,
    requests: TaskOwner,
    admission: TaskAdmission,
    running: Running,
    address: std::net::SocketAddr,
    directory: tempfile::TempDir,
    aborts: Arc<AtomicU64>,
}
async fn body_fixture(operation: Arc<BodyPreparation>) -> BodyFixture {
    body_fixture_gated(operation, None).await
}
async fn body_fixture_gated(
    operation: Arc<BodyPreparation>,
    transfer: Option<Arc<TransferGate>>,
) -> BodyFixture {
    let directory = tempfile::tempdir().unwrap();
    let admission = TaskAdmission::default();
    let transport = Registry::new(admission.clone());
    let bodies = super::super::source_body_stream::Registry::new(admission.clone());
    let requests = TaskOwner::with_admission("fixture_body_requests", admission.clone());
    let handler = bodies.clone();
    let data = directory.path().to_owned();
    let router = Router::new().route(
        "/original",
        get(move || {
            let registry = handler.clone();
            let data = data.clone();
            let operation = operation.clone();
            let transfer = transfer.clone();
            async move {
                match registry
                    .prepare_checked(
                        data,
                        move |out, checkpoint| operation(out, checkpoint),
                        || Ok(()),
                    )
                    .await
                {
                    Ok((manifest, body)) => {
                        let body = match transfer {
                            Some(gate) => gate.wrap(body),
                            None => body,
                        };
                        Response::builder()
                            .header(
                                axum::http::header::CONTENT_LENGTH,
                                manifest.body.bytes.to_string(),
                            )
                            .body(body)
                            .unwrap()
                    }
                    Err(error) => {
                        (StatusCode::UNPROCESSABLE_ENTITY, error.to_string()).into_response()
                    }
                }
            }
        }),
    );
    let owner = requests.clone();
    let router = router.layer(middleware::from_fn(move |request: Request, next: Next| {
        let owner = owner.clone();
        let connection = request.extensions().get::<Connection>().cloned();
        async move {
            match owner
                .run_async(scope(connection, async move { next.run(request).await }))
                .await
            {
                Ok(response) => response,
                Err(error) => (StatusCode::SERVICE_UNAVAILABLE, error.to_string()).into_response(),
            }
        }
    }));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    assert!(![8787, 8790].contains(&address.port()));
    let aborts = Arc::new(AtomicU64::new(0));
    let count = aborts.clone();
    let cancel = bodies.clone();
    let running = transport
        .start(
            listener,
            router,
            Arc::new(move || {
                count.fetch_add(1, Ordering::SeqCst);
                cancel.cancel_all();
            }),
        )
        .unwrap();
    BodyFixture {
        transport,
        bodies,
        requests,
        admission,
        running,
        address,
        directory,
        aborts,
    }
}
impl BodyFixture {
    fn quiesce(&self) {
        let t0 = Instant::now();
        self.admission.close();
        self.bodies.close_admission();
        self.transport.quiesce_at(t0);
    }
    async fn drain(&self) {
        let (t, b, r) = tokio::join!(
            self.transport.shutdown_and_drain(),
            self.bodies.shutdown_and_drain(),
            self.requests.shutdown_and_drain()
        );
        t.unwrap();
        b.unwrap();
        r.unwrap();
        self.running.wait().await.unwrap();
    }
    fn assert_spool_closed_and_removed(&self) {
        let s = self.bodies.stats();
        assert_eq!(s["admitted"], s["joined"]);
        assert_eq!(s["retained"], 0);
        assert_eq!(s["spool_leases"], 0);
        assert_eq!(
            std::fs::read_dir(self.directory.path().join(".source-body-spool"))
                .unwrap()
                .count(),
            0
        );
    }
}
fn body_manifest(bytes: u64, sha256: String) -> open_seo_spider_core::SitemapVerifiedBodyCopy {
    use open_seo_spider_core::{
        SitemapArchivedBody, SitemapArchivedCapture, SitemapBodyExtent, SitemapBodyReadContext,
        SitemapBodyRepresentation, SitemapVerifiedBodyCopy,
    };
    let body = SitemapArchivedBody {
        body_id: "a".repeat(64),
        representation: SitemapBodyRepresentation::DecodedXml,
        extent: SitemapBodyExtent::Complete,
        bytes,
        sha256,
    };
    SitemapVerifiedBodyCopy {
        context: SitemapBodyReadContext {
            generation: 1,
            config_sha256: "b".repeat(64),
            snapshot_sha256: "c".repeat(64),
        },
        capture: SitemapArchivedCapture {
            schema_version: 1,
            sequence: 1,
            measurement_id: "d".repeat(64),
            measurement_envelope_sha256: "e".repeat(64),
            staged_document_sha256: "f".repeat(64),
            capture_config_sha256: "b".repeat(64),
            capture_generation: 1,
            bodies: vec![body.clone()],
            omission: None,
        },
        body,
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_verified_original_nonreader_drops_tcp_then_joins_spool_and_preserves_short_length_failure()
 {
    use sha2::{Digest, Sha256};
    let f = body_fixture(Arc::new(|out, checkpoint| {
        let chunk = [0x57u8; 65_536];
        let mut hash = Sha256::new();
        for _ in 0..1024 {
            checkpoint()?;
            out.write_all(&chunk)?;
            hash.update(chunk);
        }
        Ok(body_manifest(67_108_864, format!("{:x}", hash.finalize())))
    }))
    .await;
    let mut client = TcpStream::connect(f.address).await.unwrap();
    client
        .write_all(format!("GET /original HTTP/1.1\r\nHost: {}\r\n\r\n", f.address).as_bytes())
        .await
        .unwrap();
    until(|| {
        f.transport.stats()["tcp_write_pending"].as_u64().unwrap() > 0
            && f.bodies.stats()["spool_leases"] == 1
    })
    .await;
    f.quiesce();
    tokio::time::timeout(Duration::from_secs(15), f.drain())
        .await
        .unwrap();
    f.assert_spool_closed_and_removed();
    assert_eq!(f.aborts.load(Ordering::SeqCst), 1);
    assert_eq!(f.transport.stats()["actual_io_dropped"], 1);
    assert_eq!(
        f.transport.stats()["connection_admitted"],
        f.transport.stats()["connection_joined"]
    );
    // Do not release/read the non-reader until the server's real IO and spool
    // workers have already joined. A Content-Length-short wire is failure.
    let mut buffer = [0u8; 65_536];
    let mut first = Vec::new();
    let mut length = 0u64;
    let mut read_error = false;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match client.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => {
                    length += n as u64;
                    if first.len() < 8192 {
                        first.extend_from_slice(&buffer[..n.min(8192 - first.len())]);
                    }
                }
                Err(_) => {
                    read_error = true;
                    break;
                }
            }
        }
    })
    .await
    .unwrap();
    assert!(first.starts_with(b"HTTP/1.1 200"));
    let header_end = first.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
    let headers = std::str::from_utf8(&first[..header_end])
        .unwrap()
        .to_ascii_lowercase();
    assert!(headers.contains("content-length: 67108864\r\n"));
    assert!(
        read_error || length - (header_end as u64) < 67_108_864,
        "aborted original download must never appear complete"
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_verified_original_consumes_exact_bom_nonascii_crlf_before_complete_positive_drain()
 {
    use sha2::{Digest, Sha256};
    let mut bytes = "\u{feff}original é 💡\r\n".as_bytes().to_vec();
    bytes.extend_from_slice(&vec![0x5au8; 2 * 1024 * 1024]);
    bytes.extend_from_slice(b"EXACT_ORIGINAL_TAIL\r\n");
    let expected = Arc::new(bytes);
    let payload = expected.clone();
    let transfer = TransferGate::new();
    let f = body_fixture_gated(
        Arc::new(move |out, checkpoint| {
            checkpoint()?;
            out.write_all(&payload)?;
            checkpoint()?;
            Ok(body_manifest(
                payload.len() as u64,
                format!("{:x}", Sha256::digest(payload.as_slice())),
            ))
        }),
        Some(transfer.clone()),
    )
    .await;
    let response = reqwest::get(format!("http://{}/original", f.address))
        .await
        .unwrap();
    assert_eq!(response.content_length(), Some(expected.len() as u64));
    until(|| transfer.waiting.load(Ordering::SeqCst)).await;
    assert_eq!(f.bodies.stats()["joined"], 0);
    assert_eq!(f.bodies.stats()["spool_leases"], 1);
    assert!(
        !*transfer.release.borrow(),
        "actual Body transfer is held after its first64KiB before T0"
    );
    f.quiesce();
    assert!(
        !*transfer.release.borrow(),
        "verified preparation is complete but actual transfer remains incomplete at T0"
    );
    transfer.release.send_replace(true);
    let reading = async {
        let received = response.bytes().await.unwrap();
        assert_eq!(received.as_ref(), expected.as_slice());
        assert_eq!(
            Sha256::digest(&received),
            Sha256::digest(expected.as_slice())
        );
    };
    tokio::time::timeout(Duration::from_secs(15), async {
        tokio::join!(f.drain(), reading);
    })
    .await
    .unwrap();
    assert_eq!(f.aborts.load(Ordering::SeqCst), 0);
    assert_eq!(f.transport.stats()["shutdown_aborted"], 0);
    f.assert_spool_closed_and_removed();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_original_preheader_connection_drop_cancels_checkpoint_and_removes_unadvertised_spool()
 {
    let entered = Arc::new(AtomicBool::new(false));
    let observed = entered.clone();
    let f = body_fixture(Arc::new(move |out, checkpoint| {
        out.write_all(b"private unadvertised prefix")?;
        observed.store(true, Ordering::SeqCst);
        loop {
            checkpoint()?;
            std::thread::sleep(Duration::from_millis(1));
        }
    }))
    .await;
    let mut client = TcpStream::connect(f.address).await.unwrap();
    client
        .write_all(format!("GET /original HTTP/1.1\r\nHost: {}\r\n\r\n", f.address).as_bytes())
        .await
        .unwrap();
    until(|| entered.load(Ordering::SeqCst)).await;
    assert_eq!(f.bodies.stats()["spool_leases"], 1);
    f.quiesce();
    tokio::time::timeout(Duration::from_secs(15), f.drain())
        .await
        .unwrap();
    f.assert_spool_closed_and_removed();
    assert_eq!(f.aborts.load(Ordering::SeqCst), 1);
    assert_eq!(f.transport.stats()["actual_io_dropped"], 1);
    let mut bytes = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), client.read_to_end(&mut bytes))
        .await
        .unwrap();
    assert!(
        !bytes.starts_with(b"HTTP/1.1 200"),
        "no success headers may escape a cancelled unverified private spool"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_http1_protocol_shapes_match_same_locked_axum_for_http10_http11_h2_upgrade_and_connect()
 {
    async fn exchange(address: std::net::SocketAddr, request: &[u8]) -> std::io::Result<Vec<u8>> {
        let mut socket = TcpStream::connect(address).await?;
        socket.write_all(request).await?;
        let mut bytes = Vec::new();
        let read = tokio::time::timeout(
            Duration::from_secs(5),
            (&mut socket).take(65_537).read_to_end(&mut bytes),
        )
        .await
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "protocol fixture response did not terminate",
            )
        })?;
        if let Err(error) = read
            && error.kind() != std::io::ErrorKind::ConnectionReset
        {
            return Err(error);
        }
        if bytes.len() > 65_536 {
            return Err(std::io::Error::other(
                "protocol fixture response exceeds64KiB",
            ));
        }
        Ok(bytes)
    }
    fn shape(bytes: &[u8]) -> (Option<String>, Vec<(String, String)>, Vec<u8>) {
        let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") else {
            return (None, Vec::new(), bytes.to_vec());
        };
        let mut lines = String::from_utf8_lossy(&bytes[..end])
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let status = lines.first().cloned();
        let mut headers = Vec::new();
        for line in lines.drain(1..) {
            if let Some((name, value)) = line.split_once(':') {
                let name = name.to_ascii_lowercase();
                if [
                    "content-length",
                    "transfer-encoding",
                    "connection",
                    "upgrade",
                ]
                .contains(&name.as_str())
                {
                    headers.push((name, value.trim().to_owned()));
                }
            }
        }
        headers.sort();
        (status, headers, bytes[end + 4..].to_vec())
    }
    let router = Router::new().route("/probe", get(|| async { "same-locked-protocol-shape" }));
    let stock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let stock_address = stock_listener.local_addr().unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    let stock_router = router.clone();
    let stock = tokio::spawn(async move {
        axum::serve(stock_listener, stock_router)
            .with_graceful_shutdown(async move {
                let _ = stopped.await;
            })
            .await
    });
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    assert!(![8787, 8790].contains(&address.port()));
    let transport = Registry::new(TaskAdmission::default());
    let running = transport.start(listener, router, Arc::new(|| {})).unwrap();
    let requests:[&[u8];5]=[b"GET /probe HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\n\r\n",b"GET /probe HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n",b"GET /probe HTTP/1.1\r\nHost: localhost\r\nConnection: close, upgrade\r\nUpgrade: unsupported-fixture\r\n\r\n",b"CONNECT localhost:443 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"];
    // Gather errors instead of asserting before both actual serving fixtures
    // have been stopped/joined. HTTP2/upgrade acceptance is never inferred from
    // a dependency name or a single status on an unsupported request.
    let mut results = Vec::new();
    for request in requests {
        let reference = exchange(stock_address, request).await;
        let owned = exchange(address, request).await;
        results.push((reference, owned));
    }
    let _ = stop.send(());
    transport.quiesce_at(Instant::now());
    let (owned, stock) = tokio::join!(transport.shutdown_and_drain(), stock);
    let finished = running.wait().await;
    owned.unwrap();
    stock.unwrap().unwrap();
    finished.unwrap();
    for (index, (reference, owned)) in results.into_iter().enumerate() {
        let reference = reference.unwrap();
        let owned = owned.unwrap();
        assert_eq!(
            shape(&reference),
            shape(&owned),
            "same locked baseline protocol case{index}"
        );
        if index <= 1 {
            assert!(owned.starts_with(if index == 0 {
                b"HTTP/1.0 200"
            } else {
                b"HTTP/1.1 200"
            }));
        }
        if index == 3 {
            assert!(
                !owned.starts_with(b"HTTP/1.1 101"),
                "no application upgrade was introduced"
            );
        }
    }
    assert_eq!(
        transport.stats()["connection_admitted"],
        transport.stats()["connection_joined"]
    );
    assert_eq!(transport.stats()["retained"], 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn incoming_live_private_control_observes_actual_socket_drop_without_global_abort_and_joins_owned_work()
 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    assert!(![8787, 8790].contains(&address.port()));
    let mut client = TcpStream::connect(address).await.unwrap();
    let (stream, _) = listener.accept().await.unwrap();
    let connection = Connection::new(Arc::new(Notify::new()));
    let io = IncomingIo {
        stream: Some(stream),
        control: connection.clone(),
    };
    let admission = TaskAdmission::default();
    let owner = TaskOwner::with_admission("isolated_private_connection_request", admission.clone());
    let responses = response_stream::Registry::new(admission);
    let registry = responses.clone();
    let entered = Arc::new(AtomicBool::new(false));
    let working = entered.clone();
    let token = connection.clone();
    let pending = owner
        .run_async(scope(Some(token), async move {
            registry
                .start::<(), _>(move |writer, _ready| {
                    working.store(true, Ordering::SeqCst);
                    loop {
                        writer.flush()?;
                        std::thread::sleep(Duration::from_millis(1));
                    }
                })
                .await
        }))
        .await
        .unwrap()
        .unwrap();
    until(|| entered.load(Ordering::SeqCst)).await;
    let after_actual_drop = Arc::new(AtomicBool::new(false));
    let observed = after_actual_drop.clone();
    let state = Arc::downgrade(&connection.0);
    let probe = connection
        .bind(Arc::new(move || {
            observed.store(
                state
                    .upgrade()
                    .is_some_and(|c| c.io_dropped.load(Ordering::SeqCst)),
                Ordering::SeqCst,
            );
        }))
        .unwrap();
    assert_eq!(
        lock(&connection.0.bindings).len(),
        2,
        "both actual retained producer control and ordering-probe lease are live"
    );
    assert_eq!(responses.stats().joined, 0);
    assert!(!connection.0.io_dropped.load(Ordering::SeqCst));
    assert!(!after_actual_drop.load(Ordering::SeqCst));
    // No global Registry.abort, quiesce or cancel_all occurs in this test.
    // The real stream wrapper alone drives active lease cancellation.
    drop(io);
    assert!(connection.0.io_dropped.load(Ordering::SeqCst));
    assert!(after_actual_drop.load(Ordering::SeqCst));
    assert!(lock(&connection.0.failure).is_none());
    let mut byte = [0u8; 1];
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), client.read(&mut byte))
            .await
            .unwrap()
            .unwrap(),
        0,
        "the peer independently observes actual socket EOF"
    );
    assert!(
        tokio::time::timeout(Duration::from_secs(5), pending.ready())
            .await
            .unwrap()
            .is_err(),
        "active private cancellation stops the retained preheaders worker without global cancellation"
    );
    tokio::time::timeout(Duration::from_secs(5), responses.shutdown_and_drain())
        .await
        .unwrap()
        .unwrap();
    owner.shutdown_and_drain().await.unwrap();
    assert_eq!(responses.stats().admitted, 1);
    assert_eq!(responses.stats().admitted, responses.stats().joined);
    assert_eq!(responses.stats().response_supervisors_joined, 1);
    assert_eq!(responses.stats().retained, 0);
    assert_eq!(responses.stats().complete, 0);
    drop(probe);
    assert!(lock(&connection.0.bindings).is_empty());
    drop(client);
    drop(listener);
}
