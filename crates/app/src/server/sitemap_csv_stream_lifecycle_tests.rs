//! Real Core CSV completion held before the actual blocking worker returns.
use super::*;
use axum::{Router, http::Uri};
use futures::StreamExt;
use open_seo_spider_core::{CrawlConfig, Crawler, SitemapReadSnapshot};
use std::{future::Future, path::Path, sync::atomic::AtomicBool};

const WAIT: Duration = Duration::from_secs(5);
const STILL_PENDING: Duration = Duration::from_millis(50);

struct OwnedDirectory(PathBuf);
impl OwnedDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("oss-csv-lifecycle-{}", Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Website {
    origin: String,
    count: Arc<AtomicUsize>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}
impl Website {
    async fn new() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let count = Arc::new(AtomicUsize::new(0));
        let requests = count.clone();
        let site_origin = origin.clone();
        let app = Router::new().fallback(move |uri: Uri| {
            requests.fetch_add(1, Ordering::SeqCst);
            let origin = site_origin.clone();
            async move {
                if uri.path() == "/sitemap.xml" {
                    (
                        [("content-type", "application/xml")],
                        format!(
                            "<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>{origin}/</loc></url></urlset>"
                        ),
                    )
                } else {
                    assert_eq!(uri.path(), "/", "unexpected fixture website request");
                    (
                        [("content-type", "text/html")],
                        "<html><head><title>CSV lifecycle fixture</title></head><body>one page</body></html>".into(),
                    )
                }
            }
        });
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        Self {
            origin,
            count,
            stop: Some(stop),
            task: Some(task),
        }
    }
    async fn finish(mut self, expected: usize) {
        self.stop.take().unwrap().send(()).unwrap();
        tokio::time::timeout(WAIT, self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(self.count.load(Ordering::SeqCst), expected);
    }
}
impl Drop for Website {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

#[derive(Debug, PartialEq)]
struct SavedWitness {
    main_file_sha256: String,
    actual_snapshot: serde_json::Value,
}
fn actual_read(path: &Path) -> SitemapReadSnapshot {
    CrawlStore::open_read_only(path)
        .unwrap()
        .sitemap_read_snapshot_with_membership_analysis(&Default::default())
        .unwrap()
}
fn witness(path: &Path) -> SavedWitness {
    let read = actual_read(path);
    // This covers the persisted data and actual validated read context. SQLite
    // may create/remove WAL/SHM sidecars for read-only connections; their mere
    // presence is deliberately not classified as saved-data mutation.
    let actual_snapshot = serde_json::json!({
        "crawl_config_sha256": read.crawl_config_sha256,
        "generation": read.generation,
        "catalog": read.catalog,
        "coverage_inventory": read.coverage_inventory,
        "coverage": read.coverage,
        "membership_analysis": read.membership_analysis,
        "sitemap_source_measurements": read.sitemap_source_measurements,
        "sitemap_source_measurement_context": read.sitemap_source_measurement_context,
    });
    SavedWitness {
        main_file_sha256: format!("{:x}", Sha256::digest(std::fs::read(path).unwrap())),
        actual_snapshot,
    }
}
struct SavedFixture {
    _dir: OwnedDirectory,
    website: Website,
    path: PathBuf,
    id: Uuid,
    generation: u64,
    config: String,
    query: CsvQuery,
    snapshot: serde_json::Value,
    before: SavedWitness,
    requests: usize,
}
impl SavedFixture {
    async fn new() -> Self {
        let website = Website::new().await;
        let dir = OwnedDirectory::new();
        let id = Uuid::new_v4();
        let path = dir.0.join(format!("{id}.sqlite"));
        let config = CrawlConfig {
            seed_urls: vec![format!("{}/", website.origin)],
            sitemap_urls: vec![format!("{}/sitemap.xml", website.origin)],
            max_pages: Some(1),
            concurrency: 1,
            delay_ms: 0,
            request_timeout_secs: 3,
            obey_robots: false,
            discover_sitemaps: false,
            crawl_links: false,
            ..Default::default()
        };
        let crawler = Crawler::new(config).unwrap().with_store(&path).unwrap();
        let report =
            tokio::time::timeout(WAIT, crawler.run(None, Arc::new(AtomicBool::new(false))))
                .await
                .unwrap()
                .unwrap();
        assert_eq!(report.pages.len(), 1);
        assert!(report.sitemap_catalog.is_some());
        drop(crawler);
        let requests = website.count.load(Ordering::SeqCst);
        assert_eq!(
            requests, 2,
            "one page and one explicit sitemap were fetched"
        );
        let read = actual_read(&path);
        let generation = read.generation.unwrap();
        let config = read.crawl_config_sha256.clone().unwrap();
        let inventory = membership::inventory(
            id,
            generation,
            &config,
            Some(read),
            membership::ReadQuery::default(),
        )
        .unwrap();
        assert!(inventory.catalog_available);
        assert!(!inventory.rows.as_ref().unwrap().is_empty());
        let snapshot = serde_json::to_value(inventory.snapshot.unwrap()).unwrap();
        let query = CsvQuery {
            kind: membership::RowKind::All,
            source_observation_id: None,
            membership_url: None,
            snapshot: inventory.snapshot_token.unwrap(),
        };
        query.validate().unwrap();
        let before = witness(&path);
        Self {
            _dir: dir,
            website,
            path,
            id,
            generation,
            config,
            query,
            snapshot,
            before,
            requests,
        }
    }
    async fn finish(self) {
        assert_eq!(witness(&self.path), self.before);
        assert_eq!(self.website.count.load(Ordering::SeqCst), self.requests);
        self.website.finish(self.requests).await;
    }
}

struct HeldExport {
    pending: PendingExport,
    after_core: oneshot::Receiver<()>,
    release: std::sync::mpsc::Sender<()>,
}
fn held_export(registry: &ExportRegistry, saved: &SavedFixture) -> HeldExport {
    let path = saved.path.clone();
    let id = saved.id;
    let generation = saved.generation;
    let config = saved.config.clone();
    let query = saved.query.clone();
    let expected = query.expected_context(id).unwrap();
    let (after_core, returned) = oneshot::channel();
    let (release, released) = std::sync::mpsc::channel();
    let pending = registry
        .start_with(move |ports| {
            let completion = produce(
                &ports.registry,
                path,
                id,
                generation,
                config,
                query,
                expected,
                ports.stop,
                ports.sender,
                ports.progress,
                ports.ready,
                ports.runtime,
            )?;
            after_core.send(()).unwrap();
            // Fixture-only gate: Core has returned its genuine Completion and
            // closed the actual SQL read; cancellation cannot release this
            // blocking worker. Dropping the test's sender still unblocks it.
            let _ = released.recv();
            Ok(completion)
        })
        .unwrap();
    HeldExport {
        pending,
        after_core: returned,
        release,
    }
}
fn retained_task(registry: &ExportRegistry) -> Arc<Task> {
    let state = registry.0.tasks.lock().unwrap();
    assert_eq!(state.running.len(), 1);
    state.running.values().next().unwrap().clone()
}
fn observed_drain(
    registry: ExportRegistry,
) -> (
    tokio::task::JoinHandle<anyhow::Result<()>>,
    oneshot::Receiver<()>,
) {
    let (waiting, observed) = oneshot::channel();
    let handle = tokio::spawn(async move {
        let drain = registry.shutdown_and_drain();
        tokio::pin!(drain);
        let mut waiting = Some(waiting);
        std::future::poll_fn(|cx| {
            let result = drain.as_mut().poll(cx);
            // End the condition's MutexGuard lifetime before taking/sending
            // the latch, as in the original nested-if condition.
            let pending_and_closed = result.is_pending() && registry.0.tasks.lock().unwrap().closed;
            if pending_and_closed && let Some(waiting) = waiting.take() {
                let _ = waiting.send(());
            }
            result
        })
        .await
    });
    (handle, observed)
}
async fn await_pending(observed: oneshot::Receiver<()>) {
    tokio::time::timeout(WAIT, observed).await.unwrap().unwrap();
}
async fn assert_join_slots_observed(task: &Task, supervisor: &Supervisor) {
    let worker = tokio::time::timeout(WAIT, task.worker.lock())
        .await
        .unwrap();
    assert!(worker.join.is_none(), "actual blocking JoinHandle observed");
    assert!(worker.completion.is_none());
    assert!(worker.terminal.is_some(), "actual worker result memoized");
    assert!(worker.terminal.unwrap().completion_lost);
    drop(worker);
    assert!(
        tokio::time::timeout(WAIT, supervisor.join.lock())
            .await
            .unwrap()
            .is_none(),
        "actual Supervisor JoinHandle observed"
    );
    assert!(task.finished.load(Ordering::SeqCst));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_csv_supervisor_abort_and_body_drop_recover_actual_join_slots() {
    let saved = SavedFixture::new().await;
    let registry = ExportRegistry::default();
    let HeldExport {
        mut pending,
        after_core,
        release,
    } = held_export(&registry, &saved);
    let ready = tokio::time::timeout(WAIT, pending.ready())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(ready.snapshot).unwrap(),
        saved.snapshot
    );
    assert!(ready.length > 0 && ready.length < CHUNK_BYTES as u64);
    await_pending(after_core).await;
    let task = retained_task(&registry);
    let supervisor = task.supervisor.lock().unwrap().as_ref().unwrap().clone();
    let mut stream = pending.into_body().unwrap().into_data_stream();
    let (waiting, observed) = oneshot::channel();
    let requester = tokio::spawn(async move {
        let next = stream.next();
        tokio::pin!(next);
        let mut waiting = Some(waiting);
        std::future::poll_fn(|cx| {
            let result = next.as_mut().poll(cx);
            if result.is_pending()
                && let Some(waiting) = waiting.take()
            {
                let _ = waiting.send(());
            }
            result
        })
        .await
    });
    await_pending(observed).await;
    supervisor.join.lock().await.as_ref().unwrap().abort();
    requester.abort();
    assert!(
        tokio::time::timeout(WAIT, requester)
            .await
            .unwrap()
            .unwrap_err()
            .is_cancelled()
    );
    // Supervisor cancellation must release its async mutex guard while keeping
    // the actual blocking handle owned. An RAII counter cannot prove this.
    {
        let worker = tokio::time::timeout(WAIT, task.worker.lock())
            .await
            .unwrap();
        assert!(worker.join.is_some());
        assert!(worker.terminal.is_none());
    }
    assert!(!task.finished.load(Ordering::SeqCst));
    let (mut drain, observed) = observed_drain(registry.clone());
    await_pending(observed).await;
    assert!(registry.0.tasks.lock().unwrap().closed);
    assert!(
        tokio::time::timeout(STILL_PENDING, &mut drain)
            .await
            .is_err(),
        "drain cannot finish before actual blocking worker returns"
    );
    assert_eq!(registry.0.tasks.lock().unwrap().running.len(), 1);
    release.send(()).unwrap();
    let error = tokio::time::timeout(WAIT, drain)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("CSV supervisor failed to join"));
    assert_join_slots_observed(&task, &supervisor).await;
    assert!(task.failed.load(Ordering::SeqCst));
    assert!(
        task.supervisor_failure
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .contains("CSV supervisor failed to join")
    );
    {
        let state = registry.0.tasks.lock().unwrap();
        assert!(state.running.is_empty());
        assert!(
            state
                .first_failure
                .as_ref()
                .unwrap()
                .contains("CSV supervisor failed to join")
        );
    }
    assert!(
        registry
            .shutdown_and_drain()
            .await
            .unwrap_err()
            .to_string()
            .contains("CSV supervisor failed to join")
    );
    let stats = registry.snapshot();
    assert_eq!(
        (stats.started, stats.joined, stats.active, stats.failed),
        (1, 1, 0, 1)
    );
    assert_eq!(stats.core_completed, 1);
    saved.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_csv_cancelled_and_concurrent_drains_observe_both_retained_joins() {
    let saved = SavedFixture::new().await;
    let registry = ExportRegistry::default();
    let HeldExport {
        mut pending,
        after_core,
        release,
    } = held_export(&registry, &saved);
    let ready = tokio::time::timeout(WAIT, pending.ready())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(ready.snapshot).unwrap(),
        saved.snapshot
    );
    assert!(ready.length > 0 && ready.length < CHUNK_BYTES as u64);
    await_pending(after_core).await;
    let task = retained_task(&registry);
    let supervisor = task.supervisor.lock().unwrap().as_ref().unwrap().clone();
    drop(pending.into_body().unwrap());
    let (first, observed) = observed_drain(registry.clone());
    await_pending(observed).await;
    first.abort();
    assert!(
        tokio::time::timeout(WAIT, first)
            .await
            .unwrap()
            .unwrap_err()
            .is_cancelled()
    );
    assert!(!task.finished.load(Ordering::SeqCst));
    {
        let slot = tokio::time::timeout(WAIT, supervisor.join.lock())
            .await
            .unwrap();
        assert!(
            slot.is_some(),
            "cancelled drain retained actual Supervisor join"
        );
    }
    let (mut second, second_observed) = observed_drain(registry.clone());
    let (mut third, third_observed) = observed_drain(registry.clone());
    await_pending(second_observed).await;
    await_pending(third_observed).await;
    for drain in [&mut second, &mut third] {
        assert!(
            tokio::time::timeout(STILL_PENDING, drain).await.is_err(),
            "every concurrent drain waits for the actual blocking worker"
        );
    }
    assert!(registry.0.tasks.lock().unwrap().closed);
    assert_eq!(registry.0.tasks.lock().unwrap().running.len(), 1);
    release.send(()).unwrap();
    for drain in [second, third] {
        tokio::time::timeout(WAIT, drain)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
    assert_join_slots_observed(&task, &supervisor).await;
    assert!(!task.failed.load(Ordering::SeqCst));
    assert!(task.supervisor_failure.lock().unwrap().is_none());
    {
        let state = registry.0.tasks.lock().unwrap();
        assert!(state.running.is_empty());
        assert!(state.first_failure.is_none());
    }
    registry.shutdown_and_drain().await.unwrap();
    let stats = registry.snapshot();
    assert_eq!(
        (stats.started, stats.joined, stats.active, stats.failed),
        (1, 1, 0, 0)
    );
    assert_eq!(stats.core_completed, 1);
    saved.finish().await;
}
