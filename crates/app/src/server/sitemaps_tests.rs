//! Five genuine isolated CrawlStore/handler controls.
use super::*;
use crate::sitemap_membership::RowKind;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    task::JoinSet,
};
struct Fixture {
    origin: String,
    count: Arc<AtomicUsize>,
    offline: Arc<AtomicBool>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
impl Fixture {
    async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let count = Arc::new(AtomicUsize::new(0));
        let offline = Arc::new(AtomicBool::new(false));
        let (stop, mut stopped) = oneshot::channel();
        let o = origin.clone();
        let c = count.clone();
        let f = offline.clone();
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {biased; _=&mut stopped=>break,
                    accepted=listener.accept()=>{let (mut socket,_)=accepted.unwrap();c.fetch_add(1,Ordering::SeqCst);let origin=o.clone();let offline=f.clone();
                        connections.spawn(async move {let mut bytes=Vec::new();let mut chunk=[0u8;2048];
                            tokio::time::timeout(Duration::from_secs(3),async {loop {let size=socket.read(&mut chunk).await.unwrap();
                                assert!(size>0,"request header ended early");bytes.extend_from_slice(&chunk[..size]);assert!(bytes.len()<=65536);
                                if bytes.windows(4).any(|bytes|bytes==b"\r\n\r\n"){break;}}}).await.unwrap();
                            let head=String::from_utf8(bytes).unwrap();let path=head.lines().next().unwrap().split_whitespace().nth(1).unwrap();
                            let body=if path=="/task.xml" {format!("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url/><url><loc>  {origin}/p1?a=1&amp;b=2  </loc></url><url><loc>{origin}/p1?a=1&amp;b=2</loc></url><url><loc>{origin}/p2</loc></url><url><loc>=1+1</loc></url></urlset>")}else{"<html><head><title>fixture</title></head><body>fixture</body></html>".into()};
                            let status=if offline.load(Ordering::SeqCst){410}else{200};let response=format!("HTTP/1.1 {status} Fixture\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",if path=="/task.xml"{"application/xml"}else{"text/html"},body.len());
                            let _=socket.write_all(response.as_bytes()).await;let _=socket.shutdown().await;
                        });}
                }
            }
            drop(listener);
            while let Some(result) = connections.join_next().await {
                result.unwrap();
            }
        });
        Self {
            origin,
            count,
            offline,
            stop: Some(stop),
            task: Some(task),
        }
    }
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin)
    }
    async fn finish(mut self, expected: usize) {
        let _ = self.stop.take().unwrap().send(());
        tokio::time::timeout(Duration::from_secs(5), self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            self.count.load(Ordering::SeqCst),
            expected,
            "zero API-triggered website attempts after full fixture drain"
        );
    }
}
struct OwnedDirectory(PathBuf);
impl OwnedDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("oss-sitemap-api-{}", Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
enum Mode {
    Native,
    Empty,
    Legacy,
    Detached,
}
async fn setup(fixture: &Fixture, mode: Mode) -> (OwnedDirectory, AppState, Uuid) {
    let dir = OwnedDirectory::new();
    let id = Uuid::new_v4();
    let config = CrawlConfig {
        seed_urls: vec![fixture.url("/seed")],
        sitemap_urls: if matches!(mode, Mode::Empty) {
            vec![]
        } else {
            vec![fixture.url("/task.xml")]
        },
        concurrency: 1,
        delay_ms: 0,
        obey_robots: false,
        discover_sitemaps: false,
        crawl_links: false,
        request_timeout_secs: 3,
        ..Default::default()
    };
    let crawler = if matches!(mode, Mode::Legacy) {
        Crawler::from_saved_config(config)
    } else {
        Crawler::new(config)
    }
    .unwrap()
    .with_store(dir.0.join(format!("{id}.sqlite")))
    .unwrap();
    let result = crawler
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(result.outcome, CrawlOutcome::Completed);
    let actual_id = if matches!(mode, Mode::Detached) {
        Uuid::new_v4()
    } else {
        id
    };
    let job = Job {
        id: actual_id,
        config: result.report.config.clone(),
        status: "completed".into(),
        started_at: String::new(),
        finished_at: None,
        pages: result.report.pages.clone(),
        issues: result.report.issues.clone(),
        queued: 0,
        discovery_pending: 0,
        in_flight: 0,
        warnings: result.report.warnings.clone(),
        error: None,
        report: Some(result.report),
        control: CrawlControl::new(),
        generation: 1,
        checkpoint: result.checkpoint,
        frontier_available: !matches!(mode, Mode::Detached),
        operation: None,
        recrawl_override: Default::default(),
        policy_ledger: None,
        resource_ledger: None,
    };
    let state = AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Arc::new(RwLock::new(HashMap::from([(
            actual_id,
            Arc::new(RwLock::new(job)),
        )]))),
        data_dir: Arc::new(dir.0.clone()),
        exports: Default::default(),
    };
    drop(crawler);
    fixture.offline.store(true, Ordering::SeqCst);
    // A first READ_ONLY SQLite opener can create its WAL-index bookkeeping
    // after the last writer closes. Check that explicit startup baseline
    // before measuring the handlers' strict file-byte invariance.
    let before_read_startup = files(&dir.0);
    let before_requests = fixture.count.load(Ordering::SeqCst);
    let reader = CrawlStore::open_read_only(dir.0.join(format!("{id}.sqlite"))).unwrap();
    reader.sitemap_read_snapshot().unwrap();
    drop(reader);
    let after_read_startup = files(&dir.0);
    for (name, bytes) in &before_read_startup {
        assert!(
            after_read_startup.get(name) == Some(bytes),
            "Startup changed {name}"
        );
    }
    for (name, bytes) in &after_read_startup {
        if !before_read_startup.contains_key(name) {
            if name == &format!("{id}.sqlite-wal") {
                assert!(bytes.is_empty(), "Read startup added WAL frames");
            } else {
                assert_eq!(name, &format!("{id}.sqlite-shm"));
                assert_eq!(bytes.len(), 32768);
            }
        }
    }
    assert_eq!(fixture.count.load(Ordering::SeqCst), before_requests);
    (dir, state, actual_id)
}
fn files(path: &std::path::Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            (
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}
fn assert_same_files(path: &std::path::Path, before: &BTreeMap<String, Vec<u8>>) {
    let after = files(path);
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>()
    );
    for (name, bytes) in before {
        assert!(
            after.get(name) == Some(bytes),
            "API changed file bytes: {name}"
        );
    }
}
async fn page(state: &AppState, id: Uuid, query: ReadQuery) -> membership::Inventory {
    inventory(State(state.clone()), Path(id), Ok(Query(query)))
        .await
        .unwrap()
        .0
}
fn csv_cells(bytes: &[u8]) -> Vec<Vec<String>> {
    // Small independent RFC4180 decoder oracle, not a writer mirror.
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let mut chars = text.chars().peekable();
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                cell.push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                row.push(std::mem::take(&mut cell));
            }
            '\r' if !quoted => {
                assert_eq!(chars.next(), Some('\n'));
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            _ => cell.push(ch),
        }
    }
    assert!(!quoted);
    assert!(cell.is_empty() && row.is_empty());
    rows
}
#[tokio::test]
async fn native_inventory_is_full_validated_readonly_original_source_data() {
    let fixture = Fixture::new().await;
    let (dir, state, id) = setup(&fixture, Mode::Native).await;
    let before = files(&dir.0);
    let requests = fixture.count.load(Ordering::SeqCst);
    let result = page(&state, id, ReadQuery::default()).await;
    assert!(result.catalog_available);
    assert_eq!(result.website_requests, 0);
    assert!(!result.coverage.declared_source_cohort_complete);
    assert_eq!(result.coverage.negative_membership, "unknown");
    assert_eq!(result.snapshot.as_ref().unwrap().crawl_generation, 1);
    let reader = CrawlStore::open_read_only(dir.0.join(format!("{id}.sqlite"))).unwrap();
    let original = reader.sitemap_catalog().unwrap().unwrap();
    drop(reader);
    assert_eq!(result.rows.as_ref(), original.rows.as_ref());
    assert_eq!(result.summary.as_ref(), Some(&original.summary));
    assert_same_files(&dir.0, &before);
    fixture.finish(requests).await;
}
#[tokio::test]
async fn actual_legacy_and_detached_report_catalogs_remain_unavailable() {
    for mode in [Mode::Legacy, Mode::Detached] {
        let fixture = Fixture::new().await;
        let (dir, state, id) = setup(&fixture, mode).await;
        let before = files(&dir.0);
        let requests = fixture.count.load(Ordering::SeqCst);
        let result = page(&state, id, ReadQuery::default()).await;
        assert!(!result.catalog_available);
        assert!(result.rows.is_none());
        assert!(result.summary.is_none());
        assert!(result.snapshot.is_none());
        assert_same_files(&dir.0, &before);
        fixture.finish(requests).await;
    }
}
#[tokio::test]
async fn known_empty_is_available_without_claiming_complete_source_cohort() {
    let fixture = Fixture::new().await;
    let (dir, state, id) = setup(&fixture, Mode::Empty).await;
    let before = files(&dir.0);
    let requests = fixture.count.load(Ordering::SeqCst);
    let result = page(&state, id, ReadQuery::default()).await;
    assert!(result.catalog_available);
    assert!(result.rows.as_ref().unwrap().is_empty());
    assert!(
        !result
            .summary
            .as_ref()
            .unwrap()
            .declared_source_cohort_complete
    );
    assert!(!result.has_more);
    assert!(result.snapshot_token.is_some());
    assert_same_files(&dir.0, &before);
    fixture.finish(requests).await;
}
#[tokio::test]
async fn matching_original_memberships_are_filtered_before_bounded_cursor_paging() {
    let fixture = Fixture::new().await;
    let (dir, state, id) = setup(&fixture, Mode::Native).await;
    let before = files(&dir.0);
    let requests = fixture.count.load(Ordering::SeqCst);
    let query = ReadQuery {
        kind: RowKind::Location,
        membership_url: Some(fixture.url("/p1?a=1&b=2")),
        limit: Some(1),
        ..Default::default()
    };
    let first = page(&state, id, query.clone()).await;
    assert_eq!(first.rows.as_ref().unwrap().len(), 1);
    assert!(first.has_more);
    let mut next = query.clone();
    next.cursor = first.next_cursor.clone();
    let second = page(&state, id, next).await;
    assert!(!second.has_more);
    assert_eq!(second.snapshot_token, first.snapshot_token);
    assert_eq!(second.rows.as_ref().unwrap().len(), 1);
    assert_ne!(
        first.rows.as_ref().unwrap()[0].sequence(),
        second.rows.as_ref().unwrap()[0].sequence()
    );
    let mut changed = query;
    changed.kind = RowKind::All;
    changed.cursor = first.next_cursor;
    let (status, Json(error)) = inventory(State(state.clone()), Path(id), Ok(Query(changed)))
        .await
        .err()
        .unwrap();
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error.code, "stale_sitemap_cursor");
    assert_same_files(&dir.0, &before);
    fixture.finish(requests).await;
}
#[tokio::test]
async fn full_csv_uses_same_snapshot_filter_safe_cells_and_exact_original_typed_rows() {
    let fixture = Fixture::new().await;
    let (dir, state, id) = setup(&fixture, Mode::Native).await;
    let before = files(&dir.0);
    let requests = fixture.count.load(Ordering::SeqCst);
    let result = page(
        &state,
        id,
        ReadQuery {
            kind: RowKind::Location,
            ..Default::default()
        },
    )
    .await;
    let query = CsvQuery {
        kind: RowKind::Location,
        source_observation_id: None,
        membership_url: None,
        snapshot: result.snapshot_token.clone().unwrap(),
    };
    let response = export_csv(State(state.clone()), Path(id), Ok(Query(query.clone())))
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), membership::MAX_CSV_BYTES)
        .await
        .unwrap();
    let cells = csv_cells(&bytes);
    assert_eq!(cells.len(), result.rows.as_ref().unwrap().len() + 1);
    assert_eq!(cells[0].len(), 10);
    for (actual, expected) in cells[1..].iter().zip(result.rows.as_ref().unwrap()) {
        assert_eq!(actual.len(), 10);
        assert_eq!(actual[1], expected.sequence().to_string());
        assert_eq!(actual[9], serde_json::to_string(expected).unwrap());
        if let open_seo_spider_core::SitemapLedgerRow::Location(row) = expected {
            assert_eq!(
                actual[5],
                open_seo_spider_core::csv_text(row.observed.decoded_value.as_deref().unwrap_or(""))
            );
            assert_eq!(
                actual[6],
                open_seo_spider_core::csv_text(
                    row.admission.membership_url.as_deref().unwrap_or("")
                )
            );
            assert_eq!(
                actual[7],
                open_seo_spider_core::csv_text(
                    row.admission.admitted_fetch_url.as_deref().unwrap_or("")
                )
            );
        }
    }
    assert!(
        cells
            .iter()
            .any(|row| row.get(5).is_some_and(|cell| cell == "'=1+1"))
    );
    let mut changed = query;
    changed.kind = RowKind::All;
    let (status, Json(error)) = export_csv(State(state.clone()), Path(id), Ok(Query(changed)))
        .await
        .err()
        .unwrap();
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error.code, "stale_sitemap_cursor");
    assert_same_files(&dir.0, &before);
    fixture.finish(requests).await;
}

#[path = "sitemaps_rich_read_tests.rs"]
mod rich_read_window_tests;

#[path = "sitemaps_streaming_tests.rs"]
mod streaming_tests;
