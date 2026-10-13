//! Original selected-recrawl fixtures. Parent snapshots are authored imported
//! observations; only fresh child requests are asserted as network evidence.
use open_seo_spider_core::{
    AuditIssue, CrawlConfig, CrawlControl, CrawlEvent, CrawlOutcome, CrawlReport, CrawlStore,
    CrawlSummary, Crawler, PageRecord, RecrawlResumeOverride, RecrawlSourceKind,
    RecrawlTargetState, RendererConfig, SelectedRecrawlParent, Severity, audit_pages, extract_html,
    prepare_selected_recrawl,
};
use sha2::{Digest, Sha256};
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
    status: u16,
    body: String,
    headers: Vec<(String, String)>,
    gate: Option<Arc<Semaphore>>,
}
impl Response {
    fn html(body: &str) -> Self {
        Self {
            status: 200,
            body: body.into(),
            headers: vec![("Content-Type".into(), "text/html".into())],
            gate: None,
        }
    }
    fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }
    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
    fn gated(mut self, gate: &Arc<Semaphore>) -> Self {
        self.gate = Some(gate.clone());
        self
    }
}
struct Fixture {
    origin: String,
    hits: Arc<Mutex<BTreeMap<String, usize>>>,
    credentials: Arc<Mutex<Vec<(String, bool)>>>,
    started: mpsc::UnboundedReceiver<String>,
    seen: BTreeSet<String>,
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
        let hits = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
        let credentials = Arc::new(Mutex::new(Vec::new()));
        let (sender, started) = mpsc::unbounded_channel();
        let handler = Arc::new(handler);
        let task_hits = hits.clone();
        let task_origin = origin.clone();
        let task_credentials = credentials.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let handler = handler.clone();
                let sender = sender.clone();
                let hits = task_hits.clone();
                let origin = task_origin.clone();
                let credentials = task_credentials.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let mut buffer = [0; 2048];
                    loop {
                        let Ok(length) = socket.read(&mut buffer).await else {
                            return;
                        };
                        if length == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buffer[..length]);
                        if bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                            break;
                        }
                        if bytes.len() > 65536 {
                            return;
                        }
                    }
                    let request = String::from_utf8_lossy(&bytes);
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/");
                    *hits.lock().unwrap().entry(path.into()).or_default() += 1;
                    credentials.lock().unwrap().push((
                        path.into(),
                        request
                            .lines()
                            .any(|line| line.to_ascii_lowercase().starts_with("authorization:")),
                    ));
                    let response = handler(&origin, path);
                    let _ = sender.send(path.into());
                    if let Some(gate) = response.gate {
                        gate.acquire().await.unwrap().forget();
                    }
                    let headers = response
                        .headers
                        .iter()
                        .map(|(name, value)| format!("{name}: {value}\r\n"))
                        .collect::<String>();
                    let head = format!(
                        "HTTP/1.1 {} Fixture\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
                        response.status,
                        response.body.len()
                    );
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(response.body.as_bytes()).await;
                });
            }
        });
        Self {
            origin,
            hits,
            credentials,
            started,
            seen: BTreeSet::new(),
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
        if self.seen.contains(path) {
            return;
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(actual) = self.started.recv().await {
                self.seen.insert(actual.clone());
                if actual == path {
                    return;
                }
            }
            panic!("fixture closed")
        })
        .await
        .unwrap();
    }
}
fn config(fixture: &Fixture) -> CrawlConfig {
    CrawlConfig {
        seed_urls: vec![fixture.url("/seed")],
        obey_robots: false,
        discover_sitemaps: true,
        sitemap_urls: vec![fixture.url("/sitemap.xml")],
        concurrency: 1,
        request_timeout_secs: 5,
        ..Default::default()
    }
}
fn page(url: String, title: &str) -> PageRecord {
    let mut page = PageRecord {
        url: url.clone(),
        final_url: url,
        depth: 3,
        discovered_from: Some("https://parent.example/discovery".into()),
        internal: true,
        status: Some(200),
        final_status: Some(200),
        content_type: "text/html".into(),
        fetched_at: "immutable-parent-time".into(),
        indexable: true,
        ..Default::default()
    };
    extract_html(
        &mut page,
        &format!("<title>{title}</title><h1>{title}</h1>"),
        &[],
        false,
    );
    page
}
fn snapshot(config: CrawlConfig, pages: Vec<PageRecord>) -> Vec<u8> {
    let mut issues = audit_pages(&pages);
    issues.push(AuditIssue {
        code: "obsolete_parent_issue".into(),
        severity: Severity::Warning,
        url: pages[0].url.clone(),
        message: "Historical only".into(),
        related_urls: Vec::new(),
    });
    let report = CrawlReport {
        config,
        pages,
        issues,
        summary: CrawlSummary::default(),
        warnings: vec!["historical-parent-warning".into()],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
    };
    let mut value = serde_json::to_value(report).unwrap();
    value["future_unknown_field"] = serde_json::json!({"preserve":"exact source bytes"});
    serde_json::to_vec_pretty(&value).unwrap()
}
fn parent() -> SelectedRecrawlParent {
    SelectedRecrawlParent {
        job_id: "authored-parent".into(),
        checkpoint_revision: None,
        source_kind: RecrawlSourceKind::ImportedSnapshot,
    }
}
fn sha(value: &PageRecord) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(value).unwrap()))
}
async fn controlled(crawler: &Crawler) -> open_seo_spider_core::ControlledCrawlResult {
    crawler
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap()
}

#[tokio::test]
async fn more_than_500_retained_pages_do_not_consume_selected_budget_and_old_issues_disappear() {
    let fixture = Fixture::new(|_, path| {
        Response::html(&format!(
            "<title>Fresh {path}</title><h1>Fresh</h1><a href='/outward'>outward</a>"
        ))
    })
    .await;
    let mut cfg = config(&fixture);
    cfg.max_pages = Some(1);
    let mut pages = (0..601)
        .map(|i| {
            page(
                fixture.url(&format!("/retained/{i}")),
                &format!("Unique inherited {i}"),
            )
        })
        .collect::<Vec<_>>();
    pages.insert(2, page(fixture.url("/a"), "Duplicate old title"));
    pages.insert(5, page(fixture.url("/b"), "Duplicate old title"));
    let bytes = snapshot(cfg, pages.clone());
    let selected = vec![fixture.url("/b"), fixture.url("/a"), fixture.url("/a")];
    let prepared = prepare_selected_recrawl(parent(), &bytes, &selected, Some(1)).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("child.sqlite");
    let store = CrawlStore::open(&path).unwrap();
    store.initialize_selected_recrawl(&prepared).unwrap();
    assert!(store.initialize_selected_recrawl(&prepared).is_err());
    assert_eq!(store.parent_snapshot_bytes().unwrap().unwrap(), bytes);
    let result = controlled(
        &Crawler::new(prepared.config.clone())
            .unwrap()
            .with_store(&path)
            .unwrap(),
    )
    .await;
    assert_eq!(result.outcome, CrawlOutcome::Limited);
    assert_eq!(result.report.pages.len(), 602);
    assert_eq!(result.report.summary.pending, 1);
    assert_eq!(result.report.config.max_pages, Some(1));
    let operation = result.report.operation.as_ref().unwrap();
    assert_eq!(
        (
            operation.retained_count,
            operation.refreshed_count,
            operation.pending_count
        ),
        (601, 1, 1)
    );
    assert_eq!(
        (operation.admitted_this_run, operation.committed_this_run),
        (1, 1)
    );
    assert_eq!(operation.selection[2].coalesced_into, Some(2));
    assert!(
        !result
            .report
            .issues
            .iter()
            .any(|issue| issue.code == "obsolete_parent_issue" || issue.code == "title_duplicate")
    );
    let resumed = controlled(
        &Crawler::new(prepared.config.clone())
            .unwrap()
            .with_store(&path)
            .unwrap()
            .with_recrawl_resume_override(RecrawlResumeOverride {
                max_attempts: Some(0),
            }),
    )
    .await;
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    assert_eq!(resumed.report.pages.len(), 603);
    let operation = resumed.report.operation.as_ref().unwrap();
    assert_eq!(operation.max_attempts, None);
    assert_eq!((operation.refreshed_count, operation.pending_count), (2, 0));
    assert_eq!(
        resumed
            .report
            .pages
            .iter()
            .map(|page| page.url.as_str())
            .collect::<Vec<_>>(),
        pages
            .iter()
            .map(|page| page.url.as_str())
            .collect::<Vec<_>>()
    );
    for old in pages
        .iter()
        .filter(|page| page.url != fixture.url("/a") && page.url != fixture.url("/b"))
    {
        let retained = resumed
            .report
            .pages
            .iter()
            .find(|p| p.url == old.url)
            .unwrap();
        assert_eq!(sha(retained), sha(old));
    }
    let again = controlled(
        &Crawler::new(prepared.config.clone())
            .unwrap()
            .with_store(&path)
            .unwrap(),
    )
    .await;
    assert_eq!(again.report.operation.unwrap().max_attempts, None);
    assert_eq!(fixture.count("/a"), 1);
    assert_eq!(fixture.count("/b"), 1);
    for unwanted in ["/seed", "/sitemap.xml", "/outward"] {
        assert_eq!(fixture.count(unwanted), 0, "{unwanted}");
    }
    assert_eq!(store.parent_snapshot_bytes().unwrap().unwrap(), bytes);
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn pause_excludes_pending_old_rows_drains_active_and_sqlite_only_resume_is_selected() {
    let gate = Arc::new(Semaphore::new(0));
    let blocked = gate.clone();
    let mut fixture = Fixture::new(move |_, path| {
        let response = Response::html(&format!(
            "<title>New {path}</title><h1>New</h1><a href='/not-selected'>link</a>"
        ));
        if path == "/a" {
            response.gated(&blocked)
        } else {
            response
        }
    })
    .await;
    let pages = vec![
        page(fixture.url("/retained"), "Retained"),
        page(fixture.url("/a"), "Old A"),
        page(fixture.url("/b"), "Old B"),
    ];
    let bytes = snapshot(config(&fixture), pages);
    let prepared = prepare_selected_recrawl(
        parent(),
        &bytes,
        &[fixture.url("/a"), fixture.url("/b")],
        None,
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("paused.sqlite");
    CrawlStore::open(&path)
        .unwrap()
        .initialize_selected_recrawl(&prepared)
        .unwrap();
    let crawler = Crawler::new(prepared.config.clone())
        .unwrap()
        .with_store(&path)
        .unwrap();
    let control = CrawlControl::new();
    let handle = control.clone();
    let (sender, mut events) = mpsc::unbounded_channel();
    let task =
        tokio::spawn(async move { crawler.run_controlled(Some(sender), handle).await.unwrap() });
    fixture.started("/a").await;
    assert!(control.request_pause());
    let inspection = CrawlStore::inspect(&path).unwrap();
    assert_eq!(inspection.operation.unwrap().pending_count, 2);
    assert_eq!(inspection.completed, 1);
    assert!(!task.is_finished());
    gate.add_permits(1);
    let result = task.await.unwrap();
    assert_eq!(result.outcome, CrawlOutcome::Paused);
    assert_eq!(result.report.pages.len(), 2);
    assert!(
        !result
            .report
            .pages
            .iter()
            .any(|p| p.url == fixture.url("/b"))
    );
    assert_eq!(fixture.count("/b"), 0);
    assert_eq!(fixture.count("/not-selected"), 0);
    let mut delta = false;
    while let Ok(event) = events.try_recv() {
        if let CrawlEvent::RecrawlProgress { target, .. } = event {
            delta |= target.state == RecrawlTargetState::Committed;
        }
    }
    assert!(delta);
    let inspected = CrawlStore::inspect(&path).unwrap();
    assert!(inspected.checkpoint_valid);
    assert_eq!(inspected.operation.unwrap().pending_count, 1);
    let read_only = CrawlStore::open_read_only(&path).unwrap();
    assert_eq!(read_only.parent_snapshot_bytes().unwrap().unwrap(), bytes);
    let cfg = read_only.config().unwrap().unwrap();
    drop(read_only);
    let resumed = controlled(&Crawler::new(cfg).unwrap().with_store(&path).unwrap()).await;
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    assert_eq!(resumed.report.operation.unwrap().refreshed_count, 2);
    assert_eq!(fixture.count("/a"), 1);
    assert_eq!(fixture.count("/b"), 1);
    assert_eq!(fixture.count("/seed"), 0);
}

#[tokio::test]
async fn interrupted_child_recovers_only_pending_targets_and_records_at_least_once_admission() {
    let gate = Arc::new(Semaphore::new(0));
    let blocked = gate.clone();
    let mut fixture = Fixture::new(move |_, path| {
        Response::html(&format!("<title>{path}</title><h1>New</h1>")).gated(&blocked)
    })
    .await;
    let bytes = snapshot(config(&fixture), vec![page(fixture.url("/a"), "Old")]);
    let prepared = prepare_selected_recrawl(parent(), &bytes, &[fixture.url("/a")], None).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("crash.sqlite");
    CrawlStore::open(&path)
        .unwrap()
        .initialize_selected_recrawl(&prepared)
        .unwrap();
    let cfg = prepared.config.clone();
    let crawler = Crawler::new(cfg).unwrap().with_store(&path).unwrap();
    let task = tokio::spawn(async move { crawler.run_controlled(None, CrawlControl::new()).await });
    fixture.started("/a").await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let before = fixture.hits.lock().unwrap().clone();
    let inspection = CrawlStore::inspect(&path).unwrap();
    assert!(!inspection.checkpoint_valid);
    assert_eq!(inspection.completed, 0);
    assert_eq!(inspection.pending, 1);
    assert_eq!(
        inspection.operation.as_ref().unwrap().targets[0].admitted_attempts,
        1
    );
    assert_eq!(*fixture.hits.lock().unwrap(), before);
    gate.add_permits(2);
    let resumed = controlled(
        &Crawler::new(inspection.config.unwrap())
            .unwrap()
            .with_store(&path)
            .unwrap(),
    )
    .await;
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    let target = &resumed.report.operation.unwrap().targets[0];
    assert_eq!(target.admitted_attempts, 2);
    assert_eq!(target.committed_generation, Some(2));
    assert_eq!(fixture.count("/a"), 2);
    assert_eq!(fixture.count("/seed"), 0);
}

#[tokio::test]
async fn selected_alias_and_external_target_do_not_change_credential_origins_or_retained_direct_row()
 {
    let external =
        Fixture::new(|_, _| Response::html("<title>Fresh external</title><h1>External</h1>")).await;
    let external_url = external.url("/b");
    let target = external_url.clone();
    let origin = Fixture::new(move |_, path| {
        if path == "/a" {
            Response::html("").status(302).header("Location", &target)
        } else {
            Response::html("<title>Seed</title>")
        }
    })
    .await;
    let mut cfg = config(&origin);
    cfg.check_external = true;
    cfg.custom_headers
        .insert("Authorization".into(), "Bearer owned-fixture".into());
    let mut b = page(external_url.clone(), "Unchanged direct B");
    b.internal = false;
    let old_b = sha(&b);
    let bytes = snapshot(cfg, vec![page(origin.url("/a"), "Old A"), b]);
    let prepared = prepare_selected_recrawl(parent(), &bytes, &[origin.url("/a")], None).unwrap();
    let result = controlled(
        &Crawler::new(prepared.config.clone())
            .unwrap()
            .with_selected_recrawl(prepared)
            .unwrap(),
    )
    .await;
    let a = result
        .report
        .pages
        .iter()
        .find(|p| p.url == origin.url("/a"))
        .unwrap();
    assert_eq!(a.final_url, external_url);
    let b = result
        .report
        .pages
        .iter()
        .find(|p| p.url == external_url)
        .unwrap();
    assert_eq!(sha(b), old_b);
    assert!(
        origin
            .credentials
            .lock()
            .unwrap()
            .iter()
            .all(|(_, has)| *has)
    );
    assert!(
        external
            .credentials
            .lock()
            .unwrap()
            .iter()
            .all(|(_, has)| !*has)
    );
    let direct = prepare_selected_recrawl(parent(), &bytes, &[external_url], None).unwrap();
    let _ = controlled(
        &Crawler::new(direct.config.clone())
            .unwrap()
            .with_selected_recrawl(direct)
            .unwrap(),
    )
    .await;
    assert!(
        external
            .credentials
            .lock()
            .unwrap()
            .iter()
            .all(|(_, has)| !*has)
    );
    assert_eq!(external.count("/b"), 2);
    assert_eq!(origin.count("/seed"), 0);
}

#[tokio::test]
async fn new_failure_blocked_truncated_and_renderer_error_never_inherit_stale_success_artifacts() {
    for variant in ["non200", "blocked", "truncated", "renderer"] {
        let fixture = Fixture::new(move |_, path| match (variant, path) {
            ("blocked", "/robots.txt") => Response::html("User-agent: *\nDisallow: /selected\n"),
            ("non200", _) => Response::html("<title>Fresh 404</title><h1>Error</h1>").status(404),
            ("truncated", _) => {
                Response::html(&format!("<title>Fresh</title>{}", "x".repeat(4096)))
            }
            ("renderer", "/render") => {
                Response::html("{\"error\":{\"message\":\"owned fixture\"}}").status(502)
            }
            _ => Response::html("<title>Fresh native</title><h1>Fresh</h1>"),
        })
        .await;
        let mut cfg = config(&fixture);
        if variant == "blocked" {
            cfg.obey_robots = true;
        }
        if variant == "truncated" {
            cfg.max_body_bytes = 64;
        }
        if variant == "renderer" {
            cfg.renderer = Some(RendererConfig {
                endpoint: fixture.url("/render"),
                ..Default::default()
            });
        }
        let mut old = page(fixture.url("/selected"), "Old stale title");
        old.screenshot_base64 = Some("old-screenshot".into());
        old.rendered = true;
        old.rendered_status = Some(200);
        old.rendered_url = Some(fixture.url("/old-alias"));
        old.browser_cookies
            .push(serde_json::json!({"name":"old-cookie"}));
        old.custom_javascript_results
            .push(serde_json::json!({"name":"old-script","value":"stale"}));
        old.headers.insert("x-old-artifact".into(), "stale".into());
        let bytes = snapshot(cfg, vec![old]);
        let prepared =
            prepare_selected_recrawl(parent(), &bytes, &[fixture.url("/selected")], None).unwrap();
        let result = controlled(
            &Crawler::new(prepared.config.clone())
                .unwrap()
                .with_selected_recrawl(prepared)
                .unwrap(),
        )
        .await;
        assert_eq!(result.report.operation.unwrap().refreshed_count, 1);
        let new = &result.report.pages[0];
        assert_eq!(new.screenshot_base64, None);
        assert!(new.browser_cookies.is_empty());
        assert!(new.custom_javascript_results.is_empty());
        assert!(!new.headers.contains_key("x-old-artifact"));
        assert!(!new.titles.iter().any(|v| v == "Old stale title"));
        assert!(
            !result
                .report
                .warnings
                .iter()
                .any(|v| v == "historical-parent-warning")
        );
        match variant {
            "non200" => assert_eq!(new.final_status, Some(404)),
            "blocked" => {
                assert!(new.blocked_by_robots);
                assert_eq!(fixture.count("/selected"), 0)
            }
            "truncated" => assert!(new.truncated),
            _ => assert!(new.renderer_error.is_some()),
        }
    }
}

#[tokio::test]
async fn tampered_saved_plan_and_pending_provenance_are_rejected_before_http_or_new_generation() {
    let fixture = Fixture::new(|_, _| Response::html("<title>Must not be fetched</title>")).await;
    let bytes = snapshot(
        config(&fixture),
        vec![
            page(fixture.url("/selected"), "Old"),
            page(fixture.url("/retained"), "Retained"),
        ],
    );
    let prepared =
        prepare_selected_recrawl(parent(), &bytes, &[fixture.url("/selected")], None).unwrap();
    for mutation in [
        "UPDATE frontier SET depth=999",
        "UPDATE frontier SET discovered_from='forged'",
        "UPDATE metadata SET value=json_set(value,'$.plan.unknown_future_policy',true) WHERE key='run_plan'",
        "UPDATE pages SET data='{}'",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("invalid.sqlite");
        let source = temp.path().join("parent.json");
        std::fs::write(&source, &bytes).unwrap();
        let store = CrawlStore::open(&path).unwrap();
        store.initialize_selected_recrawl(&prepared).unwrap();
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(mutation)
            .unwrap();
        assert!(CrawlStore::inspect(&path).is_err());
        let result = Crawler::new(prepared.config.clone())
            .unwrap()
            .with_store(&path)
            .unwrap()
            .run_controlled(None, CrawlControl::new())
            .await;
        assert!(result.is_err());
        assert_eq!(fixture.count("/selected"), 0);
        assert_eq!(fixture.count("/seed"), 0);
        assert_eq!(std::fs::read(source).unwrap(), bytes);
        assert_eq!(store.parent_snapshot_bytes().unwrap().unwrap(), bytes);
        let connection = rusqlite::Connection::open(&path).unwrap();
        let runs: i64 = connection
            .query_row(
                "SELECT count(*) FROM metadata WHERE key='run_state'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(runs, 0);
    }
}

#[tokio::test]
#[ignore = "Requires the local browser worker at OPEN_SEO_SPIDER_RENDERER or 127.0.0.1:8790"]
async fn live_browser_worker_selected_recrawl_replaces_rendered_artifacts_without_expansion() {
    let fixture=Fixture::new(|_,_|Response::html("<title>Fresh native</title><h1>Fresh</h1><a href='/outward'>Outward</a><script>document.title='Fresh rendered';</script>")).await;
    let mut cfg = config(&fixture);
    cfg.request_timeout_secs = 20;
    cfg.renderer = Some(RendererConfig {
        endpoint: std::env::var("OPEN_SEO_SPIDER_RENDERER")
            .unwrap_or_else(|_| "http://127.0.0.1:8790/render".into()),
        screenshot: true,
        collect_cookies: true,
        ..Default::default()
    });
    let mut old = page(fixture.url("/selected"), "Old");
    old.screenshot_base64 = Some("old".into());
    old.rendered = true;
    old.rendered_status = Some(200);
    let bytes = snapshot(cfg, vec![page(fixture.url("/retained"), "Retained"), old]);
    let prepared =
        prepare_selected_recrawl(parent(), &bytes, &[fixture.url("/selected")], None).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("browser.sqlite");
    CrawlStore::open(&path)
        .unwrap()
        .initialize_selected_recrawl(&prepared)
        .unwrap();
    let result = controlled(
        &Crawler::new(prepared.config.clone())
            .unwrap()
            .with_store(&path)
            .unwrap(),
    )
    .await;
    let fresh = &result.report.pages[1];
    assert!(fresh.rendered, "{:?}", fresh.renderer_error);
    assert_eq!(fresh.title(), "Fresh rendered");
    assert_eq!(
        fresh.raw_metadata.as_ref().unwrap().titles,
        vec!["Fresh native"]
    );
    assert!(fresh.screenshot_base64.as_ref().unwrap().len() > 1000);
    assert_eq!(result.report.operation.unwrap().refreshed_count, 1);
    assert_eq!(fixture.count("/selected"), 2);
    for url in ["/seed", "/sitemap.xml", "/outward", "/retained"] {
        assert_eq!(fixture.count(url), 0)
    }
    assert_eq!(
        CrawlStore::open_read_only(&path)
            .unwrap()
            .parent_snapshot_bytes()
            .unwrap()
            .unwrap(),
        bytes
    );
}
