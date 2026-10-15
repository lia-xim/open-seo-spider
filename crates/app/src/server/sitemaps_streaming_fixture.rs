//! Actual producers and actual loopback HTTP only. No synthetic Session/Header.
use super::*;
use axum::{Router, routing::get};
use futures::StreamExt;
use open_seo_spider_core::{ControlledCrawlResult, SitemapLedgerRow};
use std::net::SocketAddr;
use tokio::net::{TcpSocket, TcpStream};

pub(super) struct Website {
    pub origin: String,
    pub count: Arc<AtomicUsize>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}
impl Website {
    pub async fn start(entries: usize, quote_padding: usize) -> Self {
        assert!(entries >= 4 && entries % 2 == 0);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let index = format!(
            "<sitemapindex xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><sitemap><loc>{origin}/leaf-a.xml</loc></sitemap><sitemap><loc>{origin}/leaf-b.xml</loc></sitemap></sitemapindex>"
        );
        let leaves: Vec<String> = (0..2)
            .map(|leaf| {
                let mut xml =
                    String::from("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'>");
                for ordinal in leaf * (entries / 2)..(leaf + 1) * (entries / 2) {
                    let loc = match ordinal {
                        0 | 1 => format!("  {origin}/member/duplicate?a=1&amp;b=2  "),
                        2 => "=1+1".into(),
                        3 => format!("{origin}/member/unicode-%C3%A4?comma=,&amp;quote=%22"),
                        _ => format!("{origin}/member/{ordinal}"),
                    };
                    let lastmod = if quote_padding == 0 {
                        "2026-10-15".into()
                    } else {
                        format!("x,\r\n{}", "\"".repeat(quote_padding))
                    };
                    xml.push_str(&format!(
                        "<url><loc>{loc}</loc><lastmod>{lastmod}</lastmod></url>"
                    ));
                }
                xml.push_str("</urlset>");
                xml
            })
            .collect();
        let bodies = Arc::new([index, leaves[0].clone(), leaves[1].clone()]);
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        let (stop, mut stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! { biased;
                    _=&mut stopped=>break,
                    accepted=listener.accept()=> {
                        let (mut socket,_) = accepted.unwrap();
                        c.fetch_add(1,Ordering::SeqCst);
                        let bodies = bodies.clone();
                        connections.spawn(async move {
                            let mut bytes=Vec::new(); let mut chunk=[0u8;2048];
                            tokio::time::timeout(Duration::from_secs(5),async {
                                loop { let n=socket.read(&mut chunk).await.unwrap(); assert!(n>0);
                                    bytes.extend_from_slice(&chunk[..n]); assert!(bytes.len()<=65536);
                                    if bytes.windows(4).any(|w|w==b"\r\n\r\n") { break; }
                                }
                            }).await.unwrap();
                            let head=String::from_utf8(bytes).unwrap();
                            let path=head.lines().next().unwrap().split_whitespace().nth(1).unwrap();
                            let body=match path { "/root.xml"=>&bodies[0], "/leaf-a.xml"=>&bodies[1], "/leaf-b.xml"=>&bodies[2], _=>"<html><head><title>source</title></head><body>actual source</body></html>" };
                            let kind=if path.ends_with(".xml") {"application/xml"} else {"text/html"};
                            let response=format!("HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
                            socket.write_all(response.as_bytes()).await.unwrap();
                            socket.shutdown().await.unwrap();
                        });
                    }
                }
            }
            while let Some(result) = connections.join_next().await {
                result.unwrap();
            }
        });
        Self {
            origin,
            count,
            stop: Some(stop),
            task: Some(task),
        }
    }
    pub fn config(&self) -> CrawlConfig {
        CrawlConfig {
            seed_urls: vec![format!("{}/seed", self.origin)],
            sitemap_urls: vec![format!("{}/root.xml", self.origin)],
            exclude_patterns: vec!["/member/".into()],
            concurrency: 1,
            delay_ms: 0,
            obey_robots: false,
            discover_sitemaps: false,
            crawl_links: false,
            request_timeout_secs: 5,
            ..Default::default()
        }
    }
    pub async fn setup(&self) -> (OwnedDirectory, AppState, Uuid) {
        let dir = OwnedDirectory::new();
        let id = Uuid::new_v4();
        let crawler = Crawler::new(self.config())
            .unwrap()
            .with_store(db(&dir, id))
            .unwrap();
        let result = crawler
            .run_controlled(None, CrawlControl::new())
            .await
            .unwrap();
        assert_eq!(result.outcome, CrawlOutcome::Completed);
        drop(crawler);
        let state = state_from_result(&dir, id, result);
        // The ignored membership URLs remain actual raw observations. They do
        // not require hundreds of irrelevant website fetches for transport tests.
        assert_eq!(
            self.count.load(Ordering::SeqCst),
            4,
            "seed + index + two XML leaves"
        );
        warm(&dir, id);
        (dir, state, id)
    }
    pub async fn finish(mut self, expected: usize) {
        self.stop.take().unwrap().send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(10), self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            self.count.load(Ordering::SeqCst),
            expected,
            "export made no website requests"
        );
    }
}
impl Drop for Website {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
pub(super) fn db(dir: &OwnedDirectory, id: Uuid) -> PathBuf {
    dir.0.join(format!("{id}.sqlite"))
}
pub(super) fn warm(dir: &OwnedDirectory, id: Uuid) {
    let reader = CrawlStore::open_read_only(db(dir, id)).unwrap();
    reader.sitemap_read_snapshot().unwrap();
}
pub(super) fn warm_corrupt_store_bookkeeping(dir: &OwnedDirectory, id: Uuid) {
    // A controlled corruption writer can remove WAL/SHM when it is the last
    // writer to close. Establish the same explicit READ_ONLY startup boundary
    // as setup(), without running a verifier that must reject the corruption.
    let before = files(&dir.0);
    let reader = rusqlite::Connection::open_with_flags(
        db(dir, id),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let schema_rows: i64 = reader
        .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))
        .unwrap();
    assert!(schema_rows > 0);
    drop(reader);
    let after = files(&dir.0);
    for (name, bytes) in &before {
        assert!(after.get(name) == Some(bytes), "Startup changed {name}");
    }
    for (name, bytes) in &after {
        if !before.contains_key(name) {
            if name == &format!("{id}.sqlite-wal") {
                assert!(bytes.is_empty(), "Read startup added WAL frames");
            } else {
                assert_eq!(name, &format!("{id}.sqlite-shm"));
                assert_eq!(bytes.len(), 32768);
            }
        }
    }
}
pub(super) fn state_from_result(
    dir: &OwnedDirectory,
    id: Uuid,
    result: ControlledCrawlResult,
) -> AppState {
    let generation = result.checkpoint.as_ref().unwrap().generation;
    let job = Job {
        id,
        config: result.report.config.clone(),
        status: "completed".into(),
        started_at: String::new(),
        finished_at: None,
        pages: result.report.pages.clone(),
        issues: result.report.issues.clone(),
        queued: 0,
        discovery_pending: result.discovery_pending,
        in_flight: 0,
        warnings: result.report.warnings.clone(),
        error: None,
        report: Some(result.report),
        control: CrawlControl::new(),
        generation,
        checkpoint: result.checkpoint,
        frontier_available: true,
        operation: None,
        recrawl_override: Default::default(),
        policy_ledger: None,
        resource_ledger: None,
    };
    AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Arc::new(RwLock::new(HashMap::from([(
            id,
            Arc::new(RwLock::new(job)),
        )]))),
        data_dir: Arc::new(dir.0.clone()),
        exports: ExportRegistry::default(),
        finding_reads: Default::default(),
    }
}
pub(super) struct Http {
    pub address: SocketAddr,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
    registry: ExportRegistry,
}
impl Http {
    pub async fn start(state: &AppState) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = Router::new()
            .route("/api/crawls/{id}/sitemaps", get(inventory))
            .route("/api/crawls/{id}/sitemaps/export.csv", get(export_csv))
            .with_state(state.clone());
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        Self {
            address,
            stop: Some(stop),
            task: Some(task),
            registry: state.exports.clone(),
        }
    }
    pub fn export_url(&self, id: Uuid, query: &CsvQuery) -> String {
        let mut url = reqwest::Url::parse(&format!(
            "http://{}/api/crawls/{id}/sitemaps/export.csv",
            self.address
        ))
        .unwrap();
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("kind", query.kind.name())
            .append_pair("snapshot", &query.snapshot);
        if let Some(source) = &query.source_observation_id {
            pairs.append_pair("source_observation_id", source);
        }
        if let Some(member) = &query.membership_url {
            pairs.append_pair("membership_url", member);
        }
        drop(pairs);
        url.into()
    }
    pub async fn get(&self, id: Uuid, query: &CsvQuery) -> reqwest::Response {
        client()
            .get(self.export_url(id, query))
            .send()
            .await
            .unwrap()
    }
    pub async fn raw(&self, id: Uuid, query: &CsvQuery) -> (TcpStream, usize) {
        let url = reqwest::Url::parse(&self.export_url(id, query)).unwrap();
        let target = format!("{}?{}", url.path(), url.query().unwrap());
        let socket = TcpSocket::new_v4().unwrap();
        // Bound only the REAL CLIENT receive window, not any product queue,
        // formatter, CSV or source limit. This makes a stalled consumer
        // observable without depending on Windows receive autotuning.
        socket.set_recv_buffer_size(65_536).unwrap();
        let mut socket = socket.connect(self.address).await.unwrap();
        socket
            .write_all(
                format!(
                    "GET {target} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                    self.address
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let mut bytes = Vec::new();
        // One byte at a time preserves all CSV body bytes in the actual socket.
        loop {
            let byte = socket.read_u8().await.unwrap();
            bytes.push(byte);
            assert!(bytes.len() <= 65536);
            if bytes.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        let header = String::from_utf8(bytes).unwrap();
        assert!(header.starts_with("HTTP/1.1 200"), "{header}");
        let length = header
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::to_owned)
            })
            .unwrap()
            .parse()
            .unwrap();
        (socket, length)
    }
    pub async fn finish(mut self) {
        tokio::time::timeout(Duration::from_secs(10), self.registry.shutdown_and_drain())
            .await
            .unwrap()
            .unwrap();
        let _ = self.stop.take().unwrap().send(());
        tokio::time::timeout(Duration::from_secs(10), self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
    }
}
impl Drop for Http {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
pub(super) fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap()
}
pub(super) async fn query(state: &AppState, id: Uuid, read: ReadQuery) -> CsvQuery {
    let inventory = page(state, id, read.clone()).await;
    CsvQuery {
        kind: read.kind,
        source_observation_id: read.source_observation_id,
        membership_url: read.membership_url,
        snapshot: inventory.snapshot_token.unwrap(),
    }
}
pub(super) async fn oracle(
    dir: &OwnedDirectory,
    state: &AppState,
    id: Uuid,
    q: &CsvQuery,
) -> Vec<u8> {
    let job = get_job(state, id).await.ok().unwrap();
    let guard = job.read().await;
    let sha = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&guard.config).unwrap())
    );
    let saved = CrawlStore::open_read_only(db(dir, id))
        .unwrap()
        .sitemap_read_snapshot_with_membership_analysis(&Default::default())
        .unwrap();
    membership::csv(id, guard.generation, &sha, Some(saved), q.clone())
        .unwrap()
        .0
}
pub(super) async fn assert_response(response: reqwest::Response, expected: &[u8]) {
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers()["content-type"],
        "text/csv; charset=utf-8"
    );
    assert_eq!(response.content_length(), Some(expected.len() as u64));
    assert_eq!(
        response.headers()["x-sitemap-csv-sha256"],
        format!("{:x}", Sha256::digest(expected))
    );
    let bytes = response.bytes().await.unwrap();
    assert_eq!(&bytes[..], expected);
}
pub(super) async fn failure(response: reqwest::Response, status: u16, code: &str) {
    assert_eq!(response.status().as_u16(), status);
    assert!(!response.headers().contains_key("x-sitemap-csv-sha256"));
    let value: Value = response.json().await.unwrap();
    assert_eq!(value["code"], code);
    assert_eq!(value["website_requests"], 0);
}
pub(super) async fn wait_stats(
    registry: &ExportRegistry,
    accept: impl Fn(ExportStats) -> bool,
) -> ExportStats {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let stats = registry.snapshot();
            if accept(stats) {
                return stats;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("actual producer/coordinator state deadline")
}
pub(super) async fn drained(registry: &ExportRegistry) -> ExportStats {
    wait_stats(registry, |s| s.active == 0 && s.joined == s.started).await
}
pub(super) async fn reached(gate: &ExportTestGate) {
    tokio::time::timeout(Duration::from_secs(10), gate.wait_reached())
        .await
        .unwrap();
}
pub(super) fn registry(hooks: ExportTestHooks) -> ExportRegistry {
    ExportRegistry::for_test(Default::default(), hooks)
}
pub(super) async fn truncated(response: reqwest::Response) -> (usize, bool) {
    assert_eq!(response.status(), 200);
    let length = response.content_length().unwrap() as usize;
    let mut stream = response.bytes_stream();
    let mut bytes = 0;
    let mut failed = false;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => bytes += chunk.len(),
            Err(_) => {
                failed = true;
                break;
            }
        }
    }
    assert!(
        failed,
        "late producer failure must be an HTTP body error, not successful EOF"
    );
    assert!(
        bytes < length,
        "withheld positive tail cannot escape on failure"
    );
    (bytes, failed)
}
pub(super) fn raw_rows(dir: &OwnedDirectory, id: Uuid) -> Vec<SitemapLedgerRow> {
    CrawlStore::open_read_only(db(dir, id))
        .unwrap()
        .sitemap_catalog()
        .unwrap()
        .unwrap()
        .rows
        .unwrap()
}
pub(super) fn sql(dir: &OwnedDirectory, id: Uuid, statement: &str) {
    rusqlite::Connection::open(db(dir, id))
        .unwrap()
        .execute_batch(statement)
        .unwrap();
}
pub(super) fn historical_source_none(dir: &OwnedDirectory, id: Uuid) {
    // Explicit historical compatibility fixture from an ACTUAL terminal report.
    // Remove only the new optional source history and bind the exact rewritten
    // bytes using the existing checkpoint SHA contract. Raw/catalog/config,
    // inventory/generation/pages and all original capture evidence stay intact.
    let mut connection = rusqlite::Connection::open(db(dir, id)).unwrap();
    let tx = connection.transaction().unwrap();
    let report: String = tx
        .query_row("SELECT value FROM metadata WHERE key='report'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let checkpoint: String = tx
        .query_row(
            "SELECT value FROM metadata WHERE key='checkpoint'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut report: CrawlReport = serde_json::from_str(&report).unwrap();
    let mut checkpoint: CrawlCheckpoint = serde_json::from_str(&checkpoint).unwrap();
    report.sitemap_source_measurements = None;
    checkpoint.sitemap_source_measurements = None;
    let report = serde_json::to_string(&report).unwrap();
    checkpoint.report_sha256 = format!("{:x}", Sha256::digest(report.as_bytes()));
    tx.execute_batch("DELETE FROM sitemap_source_measurement_commits; DELETE FROM sitemap_source_measurements; DELETE FROM metadata WHERE key='sitemap_source_store'").unwrap();
    tx.execute("UPDATE metadata SET value=?1 WHERE key='report'", [report])
        .unwrap();
    tx.execute(
        "UPDATE metadata SET value=?1 WHERE key='checkpoint'",
        [serde_json::to_string(&checkpoint).unwrap()],
    )
    .unwrap();
    tx.commit().unwrap();
    drop(connection);
    assert!(
        CrawlStore::open_read_only(db(dir, id))
            .unwrap()
            .sitemap_read_snapshot()
            .unwrap()
            .sitemap_source_measurements
            .is_none()
    );
}
pub(super) fn report_text(dir: &OwnedDirectory, id: Uuid) -> String {
    rusqlite::Connection::open(db(dir, id))
        .unwrap()
        .query_row("SELECT value FROM metadata WHERE key='report'", [], |r| {
            r.get(0)
        })
        .unwrap()
}
pub(super) fn controlled_report_bytes(dir: &OwnedDirectory, id: Uuid, report: &str) {
    // Explicit SQL representation/corruption control over a REAL saved report.
    // Hashing supplied bytes is not claimed as report/source integrity proof.
    let mut connection = rusqlite::Connection::open(db(dir, id)).unwrap();
    let tx = connection.transaction().unwrap();
    let checkpoint: String = tx
        .query_row(
            "SELECT value FROM metadata WHERE key='checkpoint'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut checkpoint: CrawlCheckpoint = serde_json::from_str(&checkpoint).unwrap();
    checkpoint.report_sha256 = format!("{:x}", Sha256::digest(report.as_bytes()));
    tx.execute("UPDATE metadata SET value=?1 WHERE key='report'", [report])
        .unwrap();
    tx.execute(
        "UPDATE metadata SET value=?1 WHERE key='checkpoint'",
        [serde_json::to_string(&checkpoint).unwrap()],
    )
    .unwrap();
    tx.commit().unwrap();
}
pub(super) async fn selected(
    parent: &AppState,
    parent_id: Uuid,
) -> (OwnedDirectory, AppState, Uuid) {
    let job = get_job(parent, parent_id).await.ok().unwrap();
    let guard = job.read().await;
    let report = guard.report.as_ref().unwrap();
    let bytes = serde_json::to_vec(report).unwrap();
    let prepared = prepare_selected_recrawl(
        SelectedRecrawlParent {
            job_id: parent_id.to_string(),
            checkpoint_revision: guard.checkpoint.as_ref().map(|c| c.revision),
            source_kind: RecrawlSourceKind::CapturedCheckpoint,
        },
        &bytes,
        &[report.pages[0].url.clone()],
        None,
    )
    .unwrap();
    drop(guard);
    let dir = OwnedDirectory::new();
    let id = Uuid::new_v4();
    let crawler = Crawler::from_saved_config(prepared.config.clone())
        .unwrap()
        .with_selected_recrawl(prepared)
        .unwrap()
        .with_store(db(&dir, id))
        .unwrap();
    let result = crawler
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(result.outcome, CrawlOutcome::Completed);
    drop(crawler);
    let state = state_from_result(&dir, id, result);
    warm(&dir, id);
    (dir, state, id)
}
