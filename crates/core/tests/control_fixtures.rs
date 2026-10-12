//! Original local fixtures: explicit start/release barriers prove admission and
//! drain behavior without assuming that a timed sleep means a request stopped.
use open_seo_spider_core::{
    CrawlConfig, CrawlControl, CrawlEvent, CrawlOutcome, CrawlRequest, CrawlStore, Crawler,
    RendererConfig,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{Semaphore, mpsc},
};

struct Response {
    content_type: &'static str,
    body: String,
    gate: Option<Arc<Semaphore>>,
}

impl Response {
    fn html(body: &str) -> Self {
        Self {
            content_type: "text/html",
            body: body.into(),
            gate: None,
        }
    }

    fn xml(body: String) -> Self {
        Self {
            content_type: "application/xml",
            body,
            gate: None,
        }
    }

    fn gated(mut self, gate: &Arc<Semaphore>) -> Self {
        self.gate = Some(gate.clone());
        self
    }
}

struct Fixture {
    origin: String,
    hits: Arc<Mutex<BTreeMap<String, usize>>>,
    started: mpsc::UnboundedReceiver<String>,
    started_paths: BTreeSet<String>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Fixture {
    async fn new(handler: impl Fn(&str, &str) -> Response + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let task_origin = origin.clone();
        let hits = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
        let task_hits = hits.clone();
        let (sender, started) = mpsc::unbounded_channel();
        let handler = Arc::new(handler);
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let hits = task_hits.clone();
                let sender = sender.clone();
                let handler = handler.clone();
                let origin = task_origin.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut bytes = [0; 2048];
                    loop {
                        let Ok(length) = socket.read(&mut bytes).await else {
                            return;
                        };
                        if length == 0 {
                            return;
                        }
                        request.extend_from_slice(&bytes[..length]);
                        if request.windows(4).any(|part| part == b"\r\n\r\n") {
                            break;
                        }
                        if request.len() > 32768 {
                            return;
                        }
                    }
                    let request = String::from_utf8_lossy(&request);
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/");
                    *hits.lock().unwrap().entry(path.into()).or_default() += 1;
                    let response = handler(&origin, path);
                    let _ = sender.send(path.into());
                    if let Some(gate) = response.gate {
                        gate.acquire().await.unwrap().forget();
                    }
                    let headers = format!(
                        "HTTP/1.1 200 Fixture\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        response.content_type,
                        response.body.len()
                    );
                    let _ = socket.write_all(headers.as_bytes()).await;
                    let _ = socket.write_all(response.body.as_bytes()).await;
                });
            }
        });
        Self {
            origin,
            hits,
            started,
            started_paths: BTreeSet::new(),
            task,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin)
    }
    fn count(&self, path: &str) -> usize {
        self.hits.lock().unwrap().get(path).copied().unwrap_or(0)
    }
    async fn started(&mut self, path: &str) {
        if self.started_paths.contains(path) {
            return;
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(actual) = self.started.recv().await {
                self.started_paths.insert(actual.clone());
                if actual == path {
                    return;
                }
            }
            panic!("fixture stopped before {path}");
        })
        .await
        .unwrap();
    }
}

fn config(seeds: Vec<String>) -> CrawlConfig {
    CrawlConfig {
        seed_urls: seeds,
        obey_robots: false,
        discover_sitemaps: false,
        concurrency: 2,
        request_timeout_secs: 5,
        ..Default::default()
    }
}

async fn next_progress(events: &mut mpsc::UnboundedReceiver<CrawlEvent>, active: usize) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(event) = events.recv().await {
            if matches!(event, CrawlEvent::Progress { in_flight, .. } if in_flight == active) {
                return;
            }
        }
        panic!("no progress for active={active}");
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn pause_drains_active_pages_and_resume_never_refetches_committed_pages() {
    let gate = Arc::new(Semaphore::new(0));
    let blocked = gate.clone();
    let mut fixture = Fixture::new(move |_, path| match path {
        "/a" => {
            Response::html("<title>A</title><h1>A</h1><a href='/child'>child</a>").gated(&blocked)
        }
        "/b" => Response::html("<title>B</title><h1>B</h1>").gated(&blocked),
        _ => Response::html("<title>Remaining</title><h1>Remaining</h1>"),
    })
    .await;
    let options = config(vec![
        fixture.url("/a"),
        fixture.url("/b"),
        fixture.url("/queued"),
    ]);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("paused.sqlite");
    let crawler = Crawler::new(options.clone())
        .unwrap()
        .with_store(&path)
        .unwrap();
    let control = CrawlControl::new();
    let task_control = control.clone();
    let (sender, mut events) = mpsc::unbounded_channel();
    let task =
        tokio::spawn(async move { crawler.run_controlled(Some(sender), task_control).await });
    fixture.started("/a").await;
    fixture.started("/b").await;
    while events.try_recv().is_ok() {}
    assert!(control.request_pause());
    next_progress(&mut events, 2).await;
    assert!(
        !task.is_finished(),
        "pausing must await the blocked active responses"
    );
    assert_eq!(fixture.count("/queued"), 0);
    assert_eq!(fixture.count("/child"), 0);
    let before = CrawlStore::inspect(&path).unwrap();
    assert_eq!(before.run_state.unwrap().outcome, None);
    assert!(!before.checkpoint_valid);
    gate.add_permits(2);
    let paused = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(paused.outcome, CrawlOutcome::Paused);
    assert!(!paused.report.summary.cancelled);
    assert_eq!(paused.report.pages.len(), 2);
    assert_eq!(paused.report.summary.pending, 2);
    assert_eq!(paused.discovery_pending, 0);
    assert!(control.is_finished());
    assert!(!control.request_cancel());
    let checkpoint = paused.checkpoint.unwrap();
    assert_eq!((checkpoint.generation, checkpoint.revision), (1, 1));
    assert_eq!(checkpoint.outcome, CrawlOutcome::Paused);
    assert_eq!((checkpoint.completed, checkpoint.pending), (2, 2));
    let inspected = CrawlStore::inspect(&path).unwrap();
    assert!(inspected.checkpoint_valid);
    assert_eq!(
        inspected.run_state.unwrap().outcome,
        Some(CrawlOutcome::Paused)
    );
    let reopened = CrawlStore::open_read_only(&path).unwrap();
    assert_eq!(reopened.load_report().unwrap().unwrap().pages.len(), 2);
    drop(reopened);
    let resumed = Crawler::new(options)
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    assert_eq!(resumed.report.pages.len(), 4);
    assert_eq!(resumed.report.summary.pending, 0);
    let checkpoint = resumed.checkpoint.unwrap();
    assert_eq!((checkpoint.generation, checkpoint.revision), (2, 2));
    assert_eq!(fixture.count("/a"), 1);
    assert_eq!(fixture.count("/b"), 1);
    assert_eq!(fixture.count("/queued"), 1);
    assert_eq!(fixture.count("/child"), 1);
}

#[tokio::test]
async fn pause_before_admission_saves_frontier_without_website_requests() {
    let fixture = Fixture::new(|_, _| Response::html("<title>Never requested</title>")).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("not-started.sqlite");
    let control = CrawlControl::new();
    control.request_pause();
    let result = Crawler::new(config(vec![fixture.url("/first")]))
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, control)
        .await
        .unwrap();
    assert_eq!(result.outcome, CrawlOutcome::Paused);
    assert_eq!(result.report.summary.completed, 0);
    assert_eq!(result.report.summary.pending, 1);
    assert_eq!(fixture.count("/first"), 0);
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn cancel_can_escalate_pause_and_leaves_aborted_requests_pending() {
    let gate = Arc::new(Semaphore::new(0));
    let blocked = gate.clone();
    let mut fixture =
        Fixture::new(move |_, _| Response::html("<title>Blocked</title>").gated(&blocked)).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("cancel.sqlite");
    let crawler = Crawler::new(config(vec![
        fixture.url("/a"),
        fixture.url("/b"),
        fixture.url("/queued"),
    ]))
    .unwrap()
    .with_store(&path)
    .unwrap();
    let control = CrawlControl::new();
    let cloned = control.clone();
    let (sender, mut events) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move { crawler.run_controlled(Some(sender), cloned).await });
    fixture.started("/a").await;
    fixture.started("/b").await;
    while events.try_recv().is_ok() {}
    control.request_pause();
    next_progress(&mut events, 2).await;
    control.request_cancel();
    let result = tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(result.outcome, CrawlOutcome::Cancelled);
    assert!(result.report.summary.cancelled);
    assert_eq!(result.report.summary.completed, 0);
    assert_eq!(result.report.summary.pending, 3);
    assert_eq!(fixture.count("/queued"), 0);
    assert_eq!(control.request(), CrawlRequest::CancelRequested);
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
    gate.add_permits(2);
}

#[tokio::test]
async fn pause_drains_current_sitemap_and_recovers_children_without_refetching_index() {
    let gate = Arc::new(Semaphore::new(0));
    let blocked = gate.clone();
    let mut fixture = Fixture::new(move |origin, path| match path {
        "/index.xml" => Response::xml(format!("<sitemapindex><sitemap><loc>{origin}/one.xml</loc></sitemap><sitemap><loc>{origin}/two.xml</loc></sitemap></sitemapindex>")).gated(&blocked),
        "/one.xml" => Response::xml(format!("<urlset><url><loc>{origin}/p1</loc></url></urlset>")),
        "/two.xml" => Response::xml(format!("<urlset><url><loc>{origin}/p2</loc></url></urlset>")),
        _ => Response::html("<title>Page</title><h1>Page</h1>"),
    }).await;
    let mut options = config(vec![fixture.url("/root")]);
    options.crawl_links = false;
    options.sitemap_urls = vec![fixture.url("/index.xml")];
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("discovery.sqlite");
    let crawler = Crawler::new(options.clone())
        .unwrap()
        .with_store(&path)
        .unwrap();
    let control = CrawlControl::new();
    let cloned = control.clone();
    let task = tokio::spawn(async move { crawler.run_controlled(None, cloned).await });
    fixture.started("/index.xml").await;
    control.request_pause();
    assert!(!task.is_finished());
    gate.add_permits(1);
    let paused = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(paused.outcome, CrawlOutcome::Paused);
    assert_eq!(paused.discovery_pending, 2);
    assert_eq!(paused.report.summary.pending, 1);
    assert_eq!(fixture.count("/one.xml"), 0);
    assert_eq!(fixture.count("/two.xml"), 0);
    assert_eq!(fixture.count("/root"), 0);
    let inspection = CrawlStore::inspect(&path).unwrap();
    assert!(inspection.checkpoint_valid);
    assert!(!inspection.discovery_complete);
    assert_eq!(inspection.discovery_pending, 2);
    let resumed = Crawler::new(options)
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    assert_eq!(resumed.report.pages.len(), 3);
    assert_eq!(fixture.count("/index.xml"), 1);
    assert_eq!(fixture.count("/one.xml"), 1);
    assert_eq!(fixture.count("/two.xml"), 1);
    assert!(CrawlStore::inspect(&path).unwrap().discovery_complete);
}

#[tokio::test]
async fn cancelled_discovery_remains_pending_and_may_repeat_the_uncommitted_request() {
    let gate = Arc::new(Semaphore::new(0));
    let blocked = gate.clone();
    let mut fixture = Fixture::new(move |origin, path| match path {
        "/map.xml" => Response::xml(format!("<urlset><url><loc>{origin}/p</loc></url></urlset>"))
            .gated(&blocked),
        _ => Response::html("<title>Page</title>"),
    })
    .await;
    let mut options = config(vec![fixture.url("/root")]);
    options.crawl_links = false;
    options.sitemap_urls = vec![fixture.url("/map.xml")];
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("aborted-discovery.sqlite");
    let crawler = Crawler::new(options.clone())
        .unwrap()
        .with_store(&path)
        .unwrap();
    let control = CrawlControl::new();
    let cloned = control.clone();
    let task = tokio::spawn(async move { crawler.run_controlled(None, cloned).await });
    fixture.started("/map.xml").await;
    control.request_cancel();
    let cancelled = tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(cancelled.outcome, CrawlOutcome::Cancelled);
    assert_eq!(cancelled.discovery_pending, 1);
    let inspected = CrawlStore::inspect(&path).unwrap();
    assert!(!inspected.discovery_complete);
    assert!(inspected.checkpoint_valid);
    gate.add_permits(2);
    let resumed = Crawler::new(options)
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    assert_eq!(fixture.count("/map.xml"), 2);
    assert_eq!(resumed.report.pages.len(), 2);
}

#[tokio::test]
async fn sitemap_resource_limit_does_not_claim_discovery_complete() {
    let fixture = Fixture::new(|origin, path| match path {
        "/index.xml" => Response::xml(format!(
            "<sitemapindex><sitemap><loc>{origin}/later.xml</loc></sitemap></sitemapindex>"
        )),
        _ => Response::html("<title>Page</title>"),
    })
    .await;
    let mut options = config(vec![fixture.url("/root")]);
    options.sitemap_urls = vec![fixture.url("/index.xml")];
    options.max_sitemap_documents = 1;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("limited-discovery.sqlite");
    let result = Crawler::new(options)
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(result.outcome, CrawlOutcome::Limited);
    assert!(result.report.summary.limited);
    assert_eq!(result.discovery_pending, 1);
    assert_eq!(fixture.count("/later.xml"), 0);
    assert!(!CrawlStore::inspect(&path).unwrap().discovery_complete);
}

#[tokio::test]
#[ignore = "requires the local real Chrome renderer worker at 127.0.0.1:8790"]
async fn live_browser_worker_pause_drains_render_and_retains_rendered_discoveries() {
    let health = reqwest::get("http://127.0.0.1:8790/health").await.unwrap();
    assert!(
        health.status().is_success(),
        "real renderer worker must be healthy"
    );
    let gate = Arc::new(Semaphore::new(0));
    let blocked = gate.clone();
    let mut fixture = Fixture::new(move |_,path| match path {
        "/page" => Response::html("<html><head><title>Native pause fixture</title><script>fetch('/hold').then(r=>r.text()).then(value=>{document.title=value;let link=document.createElement('a');link.href='/child';link.textContent='rendered child';document.body.append(link);});</script></head><body><h1>Pause fixture</h1></body></html>"),
        "/hold" => Response::html("Rendered pause fixture").gated(&blocked),
        _ => Response::html("<title>This queued URL must not start</title>"),
    }).await;
    let mut options = config(vec![fixture.url("/page"), fixture.url("/queued")]);
    options.concurrency = 1;
    options.request_timeout_secs = 10;
    options.renderer = Some(RendererConfig {
        wait_ms: 1000,
        ..Default::default()
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("browser-pause.sqlite");
    let crawler = Crawler::new(options).unwrap().with_store(&path).unwrap();
    let control = CrawlControl::new();
    let cloned = control.clone();
    let (sender, mut events) = mpsc::unbounded_channel();
    let run = tokio::spawn(async move { crawler.run_controlled(Some(sender), cloned).await });
    fixture.started("/hold").await;
    while events.try_recv().is_ok() {}
    control.request_pause();
    next_progress(&mut events, 1).await;
    assert!(
        !run.is_finished(),
        "pause must await the actual browser request"
    );
    assert!(!CrawlStore::inspect(&path).unwrap().checkpoint_valid);
    assert_eq!(fixture.count("/queued"), 0);
    gate.add_permits(1);
    let result = tokio::time::timeout(Duration::from_secs(12), run)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(result.outcome, CrawlOutcome::Paused);
    assert_eq!(result.report.pages.len(), 1);
    let page = &result.report.pages[0];
    assert!(page.rendered);
    assert_eq!(page.rendered_status, Some(200));
    assert!(page.renderer_error.is_none());
    assert_eq!(page.titles, vec!["Rendered pause fixture"]);
    assert_eq!(
        page.raw_metadata.as_ref().unwrap().titles,
        vec!["Native pause fixture"]
    );
    assert!(
        page.links
            .iter()
            .any(|link| link.url == fixture.url("/child"))
    );
    assert_eq!(
        fixture.count("/page"),
        2,
        "one native and one browser document request are observed"
    );
    assert_eq!(fixture.count("/queued"), 0);
    assert_eq!(fixture.count("/child"), 0);
    assert_eq!(result.report.summary.pending, 2);
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}
