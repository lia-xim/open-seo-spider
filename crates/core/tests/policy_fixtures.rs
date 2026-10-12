//! Original local native-policy fixtures; request counters and barriers are evidence.
use open_seo_spider_core::*;
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
        self.headers
            .retain(|(existing, _)| !existing.eq_ignore_ascii_case(name));
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
        seed_urls: vec![fixture.url("/shop/start")],
        obey_robots: false,
        discover_sitemaps: false,
        concurrency: 1,
        delay_ms: 0,
        request_timeout_secs: 5,
        crawl_policy: Some(CrawlPolicyConfig {
            scope: CrawlScopeConfig {
                roots: vec![CrawlScopeRoot {
                    origin: fixture.origin.clone(),
                    subdomains: false,
                    folders: vec!["/shop/".into()],
                    exact_paths: vec!["/sitemap.xml".into(), "/child.xml".into()],
                }],
                ..Default::default()
            },
            ..Default::default()
        }),
        ..Default::default()
    }
}
fn rewrite(pattern: &str, text: &str) -> RewriteRule {
    RewriteRule {
        id: "replacement".into(),
        pattern: pattern.into(),
        replacement: RewriteReplacement::Literal { text: text.into() },
    }
}
fn ledger(path: &std::path::Path) -> Vec<StoredPolicyOccurrence> {
    let store = CrawlStore::open_read_only(path).unwrap();
    let mut all = Vec::new();
    store
        .visit_policy_occurrences(1, |page| {
            all.extend(page.rows.clone());
            Ok(())
        })
        .unwrap();
    all
}
async fn run(config: CrawlConfig, path: &std::path::Path) -> CrawlReport {
    Crawler::new(config)
        .unwrap()
        .with_store(path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap()
        .report
}

#[tokio::test]
async fn rewrites_coalesce_fetches_and_preserve_observed_fragments_and_graph_targets() {
    let fixture=Fixture::new(|_,path|match path {
        "/shop/start"=>Response::html("<a href='/shop/old?utm=a&id=1#raw'>First</a><a href='/shop/old?utm=b&id=1#other'>Second</a>"),
        _=>Response::html("<h1>One response</h1>")
    }).await;
    let mut config = config(&fixture);
    let policy = config.crawl_policy.as_mut().unwrap();
    policy.query.remove_keys = vec!["utm".into()];
    policy.rewrites = vec![rewrite("/shop/old", "/shop/new")];
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("aliases.sqlite");
    let report = run(config, &path).await;
    assert_eq!(report.pages.len(), 2);
    assert_eq!(fixture.count("/shop/new?id=1"), 1);
    assert_eq!(fixture.count("/shop/old?utm=a&id=1"), 0);
    let source = &report.pages[0];
    assert_eq!(source.links[0].url, fixture.url("/shop/old?utm=a&id=1"));
    assert_eq!(source.links[0].fragment.as_deref(), Some("raw"));
    let rows = ledger(&path);
    let anchors = rows
        .iter()
        .filter(|row| row.occurrence.decision.source_kind == CandidateSourceKind::Anchor)
        .collect::<Vec<_>>();
    assert_eq!(anchors.len(), 2);
    assert_eq!(anchors[0].occurrence.admission_reason, "scheduled");
    assert_eq!(anchors[1].occurrence.admission_reason, "coalesced");
    assert_eq!(
        anchors[1]
            .occurrence
            .coalesced_task_occurrence_id
            .as_deref(),
        Some(anchors[0].occurrence.decision.occurrence_id.as_str())
    );
    assert_eq!(
        anchors[0].occurrence.decision.fragment.as_deref(),
        Some("raw")
    );
    assert!(report.policy_ledger.unwrap().complete);
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn exact_folders_redirect_permissions_and_cross_origin_credentials_are_enforced() {
    let second =
        Fixture::new(|_, _| Response::html("<a href='/shop/not-expanded'>Never</a>")).await;
    let second_url = second.url("/checked");
    let fixture=Fixture::new(move|_,path|match path {
        "/shop/start"=>Response::html("<a href='/shop/replace'>Replace</a><a href='/shopping/check'>Outside</a><a href='/shop/redirect'>Denied redirect</a><a href='/shopping/redirect'>Check into full</a><a href='/shop/to-check'>Full into check</a>"),
        "/shop/redirect"=>Response::html("").status(302).header("Location","/shop/denied"),
        "/shopping/redirect"=>Response::html("").status(302).header("Location","/shop/final"),
        "/shop/to-check"=>Response::html("").status(302).header("Location","/shopping/final"),
        _=>Response::html("<a href='/shop/forbidden-expansion'>Never</a>")
    }).await;
    let mut config = config(&fixture);
    config
        .custom_headers
        .insert("Authorization".into(), "fixture scoped".into());
    let policy = config.crawl_policy.as_mut().unwrap();
    policy.scope.outside_folder = OutsideScopePolicy::CheckOnce;
    policy.scope.external = OutsideScopePolicy::CheckOnce;
    policy.rewrites = vec![rewrite(
        &format!("{}/shop/replace", fixture.origin),
        &second_url,
    )];
    policy.exclude_patterns = vec!["/shop/denied$".into()];
    let temp = tempfile::tempdir().unwrap();
    let report = run(config, &temp.path().join("scope.sqlite")).await;
    assert_eq!(second.count("/checked"), 1);
    assert!(
        second
            .credentials
            .lock()
            .unwrap()
            .iter()
            .all(|(_, secret)| !*secret)
    );
    assert_eq!(fixture.count("/shop/denied"), 0);
    assert_eq!(fixture.count("/shop/forbidden-expansion"), 0);
    assert_eq!(second.count("/shop/not-expanded"), 0);
    let denied = report
        .pages
        .iter()
        .find(|page| page.url == fixture.url("/shop/redirect"))
        .unwrap();
    assert_eq!(denied.status, Some(302));
    assert_eq!(denied.final_status, None);
    assert_eq!(denied.redirect_chain.len(), 1);
    for path in ["/shopping/redirect", "/shop/to-check"] {
        let page = report
            .pages
            .iter()
            .find(|page| page.url == fixture.url(path))
            .unwrap();
        assert!(!page.policy.as_ref().unwrap().effective_can_expand);
    }
    assert!(
        fixture
            .credentials
            .lock()
            .unwrap()
            .iter()
            .all(|(_, secret)| *secret)
    );
}

#[tokio::test]
async fn robots_check_actual_rewritten_paths_and_protocol_requests_have_no_credentials() {
    let fixture = Fixture::new(|_, path| match path {
        "/robots.txt" => {
            Response::html("User-agent: *\nDisallow: /shop/old\nDisallow: /shop/blocked\n")
        }
        "/shop/start" => Response::html(
            "<a href='/shop/old'>Allowed new</a><a href='/shop/allowed-old'>Blocked new</a>",
        ),
        _ => Response::html("<h1>Result</h1>"),
    })
    .await;
    let mut config = config(&fixture);
    config.obey_robots = true;
    config
        .custom_headers
        .insert("Authorization".into(), "scoped".into());
    config.crawl_policy.as_mut().unwrap().rewrites =
        vec![rewrite("/shop/(?:old|allowed-old)$", "/shop/new")];
    config
        .crawl_policy
        .as_mut()
        .unwrap()
        .rewrites
        .push(RewriteRule {
            id: "block-second".into(),
            pattern: "/shop/allowed-old$".into(),
            replacement: RewriteReplacement::Literal {
                text: "/shop/blocked".into(),
            },
        });
    // Distinct rules ensure old is allowed only after rewrite while allowed-old is blocked.
    config.crawl_policy.as_mut().unwrap().rewrites[0].pattern = "/shop/old$".into();
    let temp = tempfile::tempdir().unwrap();
    let report = run(config, &temp.path().join("robots.sqlite")).await;
    assert_eq!(fixture.count("/robots.txt"), 1);
    assert_eq!(fixture.count("/shop/old"), 0);
    assert_eq!(fixture.count("/shop/new"), 1);
    assert_eq!(fixture.count("/shop/blocked"), 0);
    assert!(
        report
            .pages
            .iter()
            .any(|page| page.url == fixture.url("/shop/blocked") && page.blocked_by_robots)
    );
    assert!(
        fixture
            .credentials
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _)| path == "/robots.txt")
            .all(|(_, secret)| !*secret)
    );
}

#[tokio::test]
async fn sitemap_roles_preserve_invalid_duplicate_locs_and_check_only_does_not_discover() {
    let fixture=Fixture::new(|origin,path|match path {
        "/sitemap.xml"=>Response::html(&format!("<sitemapindex><sitemap><loc>{origin}/child.xml</loc></sitemap><sitemap><loc>{origin}/child.xml</loc></sitemap></sitemapindex>")).header("Content-Type","application/xml"),
        "/child.xml"=>Response::html(&format!("<urlset><url><loc>{origin}/shop/located</loc></url><url><loc>not an absolute URI</loc></url><url><loc>{origin}/shopping/denied</loc></url></urlset>")).header("Content-Type","application/xml"),
        "/outside.xml"=>Response::html(&format!("<urlset><url><loc>{origin}/shop/must-not</loc></url></urlset>")),
        _=>Response::html("<h1>Capture</h1>")
    }).await;
    let mut config = config(&fixture);
    config.sitemap_urls = vec![fixture.url("/sitemap.xml"), fixture.url("/outside.xml")];
    config.crawl_policy.as_mut().unwrap().scope.outside_folder = OutsideScopePolicy::CheckOnce;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sitemap.sqlite");
    let report = run(config, &path).await;
    assert_eq!(fixture.count("/child.xml"), 1);
    assert_eq!(fixture.count("/shop/located"), 1);
    assert_eq!(fixture.count("/shop/must-not"), 0);
    let rows = ledger(&path);
    assert!(rows.iter().any(
        |row| row.occurrence.decision.raw_value == "not an absolute URI"
            && row.occurrence.admission_reason == "policy_denied"
    ));
    assert!(rows.iter().any(
        |row| row.occurrence.decision.raw_value == fixture.url("/child.xml")
            && row.occurrence.admission_reason == "coalesced"
    ));
    assert!(rows.iter().any(|row| row.occurrence.decision.raw_value
        == fixture.url("/shop/must-not")
        && row.occurrence.admission_reason == "source_check_only"));
    assert!(report.policy_ledger.unwrap().complete);
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn pause_sqlite_resume_and_selected_child_reuse_exempt_admissions_without_rewrite() {
    let gate = Arc::new(Semaphore::new(0));
    let request_gate = gate.clone();
    let mut fixture = Fixture::new(move |_, path| match path {
        "/shop/start" => Response::html("<a href='/shop/a'>A</a><a href='/shop/b'>B</a>"),
        "/shop/a" => Response::html("<h1>Drain</h1>").gated(&request_gate),
        _ => Response::html("<h1>Capture</h1>"),
    })
    .await;
    let mut config = config(&fixture);
    config.crawl_policy.as_mut().unwrap().exclude_patterns = vec!["/shop/start$".into()];
    config.crawl_policy.as_mut().unwrap().rewrites =
        vec![rewrite("/shop/start$", "/shop/rewritten-seed")];
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("pause.sqlite");
    let crawler = Crawler::new(config.clone())
        .unwrap()
        .with_store(&path)
        .unwrap();
    let control = CrawlControl::new();
    let worker_control = control.clone();
    let task =
        tokio::spawn(async move { crawler.run_controlled(None, worker_control).await.unwrap() });
    fixture.started("/shop/a").await;
    assert!(control.request_pause());
    gate.add_permits(1);
    let paused = task.await.unwrap();
    assert_eq!(paused.outcome, CrawlOutcome::Paused);
    assert_eq!(fixture.count("/shop/b"), 0);
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
    let complete = run(config.clone(), &path).await;
    assert_eq!(fixture.count("/shop/start"), 1);
    assert_eq!(fixture.count("/shop/rewritten-seed"), 0);
    assert_eq!(fixture.count("/shop/b"), 1);
    let bytes = serde_json::to_vec_pretty(&complete).unwrap();
    let child = temp.path().join("selected.sqlite");
    let prepared = prepare_selected_recrawl(
        SelectedRecrawlParent {
            job_id: "owned-parent".into(),
            checkpoint_revision: Some(2),
            source_kind: RecrawlSourceKind::CapturedCheckpoint,
        },
        &bytes,
        &[fixture.url("/shop/start")],
        Some(1),
    )
    .unwrap();
    let report = Crawler::new(prepared.config.clone())
        .unwrap()
        .with_store(&child)
        .unwrap()
        .with_selected_recrawl(prepared)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap()
        .report;
    assert_eq!(fixture.count("/shop/start"), 2);
    assert_eq!(fixture.count("/shop/a"), 1);
    assert_eq!(fixture.count("/shop/b"), 1);
    let summary = report.policy_ledger.unwrap();
    assert!(!summary.complete);
    let inherited = summary.inherited_history.unwrap();
    assert_eq!(inherited.available_occurrences, 0);
    assert!(!inherited.complete);
    assert_eq!(
        CrawlStore::open_read_only(&child)
            .unwrap()
            .parent_snapshot_bytes()
            .unwrap()
            .unwrap(),
        bytes
    );
    assert!(
        ledger(&child)
            .iter()
            .all(|row| row.basis == PolicyObservationBasis::Current)
    );
    assert!(
        ledger(&child)
            .iter()
            .any(|row| row.occurrence.admission_reason == "selected_no_expansion")
    );
}

#[tokio::test]
async fn crash_recovery_preserves_admissions_and_counts_only_committed_source_decisions() {
    let gate = Arc::new(Semaphore::new(0));
    let request_gate = gate.clone();
    let mut fixture = Fixture::new(move |_, path| {
        if path == "/shop/a" {
            Response::html("<h1>Fresh</h1>").gated(&request_gate)
        } else {
            Response::html("<a href='/shop/a'>A</a>")
        }
    })
    .await;
    let config = config(&fixture);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("crash.sqlite");
    let crawler = Crawler::new(config.clone())
        .unwrap()
        .with_store(&path)
        .unwrap();
    let task = tokio::spawn(async move { crawler.run_controlled(None, CrawlControl::new()).await });
    fixture.started("/shop/a").await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let inspection = CrawlStore::inspect(&path).unwrap();
    assert!(!inspection.checkpoint_valid);
    assert_eq!(inspection.pending, 1);
    let before = ledger(&path);
    assert_eq!(before.len(), 2);
    gate.add_permits(2);
    let report = run(config, &path).await;
    assert_eq!(fixture.count("/shop/start"), 1);
    assert_eq!(fixture.count("/shop/a"), 2);
    assert_eq!(ledger(&path).len(), 2);
    assert_eq!(report.pages.len(), 2);
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn policy_storage_failures_roll_back_source_frontier_ledger_and_children_together() {
    let fixture = Fixture::new(|_, path| {
        if path == "/shop/start" {
            Response::html("<a href='/shop/child'>Child</a>")
        } else {
            Response::html("<h1>Done</h1>")
        }
    })
    .await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("atomic.sqlite");
    let config = config(&fixture);
    let store = CrawlStore::open(&path).unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_anchor BEFORE INSERT ON policy_occurrences WHEN NEW.data LIKE '%\"source_kind\":\"anchor\"%' BEGIN SELECT RAISE(ABORT,'fixture policy write failed'); END;").unwrap();
    let result = Crawler::new(config.clone())
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await;
    assert!(result.is_err());
    let store = CrawlStore::open_read_only(&path).unwrap();
    assert_eq!(store.pages().unwrap().len(), 0);
    assert_eq!(store.pending_count().unwrap(), 1);
    assert_eq!(ledger(&path).len(), 1);
    assert_eq!(fixture.count("/shop/child"), 0);
    assert!(CrawlStore::inspect(&path).is_ok());
    connection
        .execute_batch("DROP TRIGGER fail_anchor;")
        .unwrap();
    let report = run(config, &path).await;
    assert_eq!(report.pages.len(), 2);
    assert_eq!(fixture.count("/shop/start"), 2);
    assert_eq!(fixture.count("/shop/child"), 1);
    assert_eq!(ledger(&path).len(), 2);
}

#[tokio::test]
async fn invalid_policy_renderer_and_explicit_storage_limits_do_not_fake_completion() {
    let fixture = Fixture::new(|_, _| Response::html("<h1>Result</h1>")).await;
    let temp = tempfile::tempdir().unwrap();
    let mut malformed = config(&fixture);
    malformed.crawl_policy.as_mut().unwrap().include_patterns = vec!["[".into()];
    assert!(Crawler::new(malformed).is_err());
    let mut browser = config(&fixture);
    browser.renderer = Some(RendererConfig::default());
    assert!(Crawler::new(browser).is_err());
    assert!(
        Crawler::new(config(&fixture))
            .unwrap()
            .run_controlled(None, CrawlControl::new())
            .await
            .is_err()
    );
    let mut bounded = config(&fixture);
    bounded.live_policy_limits = Some(LiveCrawlPolicyLimits {
        max_ledger_bytes: Some(10),
        ..Default::default()
    });
    let path = temp.path().join("limit.sqlite");
    assert!(
        Crawler::new(bounded)
            .unwrap()
            .with_store(&path)
            .unwrap()
            .run_controlled(None, CrawlControl::new())
            .await
            .is_err()
    );
    assert_eq!(fixture.count("/shop/start"), 0);
    assert_eq!(
        CrawlStore::open_read_only(&path)
            .unwrap()
            .pending_count()
            .unwrap(),
        0
    );
    assert!(ledger(&path).is_empty());
    assert!(CrawlStore::inspect(&path).unwrap().policy_ledger.is_none());
}

#[tokio::test]
async fn native_policy_supports_more_than_500_urls_and_bounded_paging_without_a_total_record_cap() {
    let fixture = Fixture::new(|_, path| {
        if path == "/shop/start" {
            Response::html(
                &(0..501)
                    .map(|index| format!("<a href='/shop/item/{index}'>Item</a>"))
                    .collect::<String>(),
            )
        } else {
            Response::html("<h1>Captured</h1>")
        }
    })
    .await;
    let mut config = config(&fixture);
    config.concurrency = 16;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("unlimited.sqlite");
    let report = run(config, &path).await;
    assert_eq!(report.pages.len(), 502);
    assert_eq!(report.policy_ledger.as_ref().unwrap().occurrences, 502);
    assert!(report.policy_ledger.as_ref().unwrap().complete);
    let store = CrawlStore::open_read_only(&path).unwrap();
    assert!(store.policy_occurrences(None, 0).is_err());
    assert!(store.policy_occurrences(None, 1001).is_err());
    let first = store.policy_occurrences(None, 100).unwrap();
    assert_eq!(first.rows.len(), 100);
    assert!(first.has_more);
    assert_eq!(first.next_sequence, Some(100));
    let mut seen = 0;
    let summary = store
        .visit_policy_occurrences(37, |page| {
            seen += page.rows.len();
            Ok(())
        })
        .unwrap()
        .unwrap();
    assert_eq!(seen, 502);
    assert_eq!(summary, report.policy_ledger.unwrap());
}

#[tokio::test]
async fn failed_sitemap_redirect_keeps_observed_location_in_protocol_ledger() {
    let fixture = Fixture::new(|_, path| match path {
        "/sitemap.xml" => Response::html("")
            .status(302)
            .header("Location", "/forbidden-map.xml"),
        _ => Response::html("<h1>Body</h1>"),
    })
    .await;
    let mut config = config(&fixture);
    config.sitemap_urls = vec![fixture.url("/sitemap.xml")];
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("redirect.sqlite");
    let report = run(config, &path).await;
    assert_eq!(fixture.count("/forbidden-map.xml"), 0);
    let rows = ledger(&path);
    let redirect = rows
        .iter()
        .find(|row| row.occurrence.decision.source_kind == CandidateSourceKind::Redirect)
        .unwrap();
    assert_eq!(redirect.occurrence.decision.raw_value, "/forbidden-map.xml");
    assert_eq!(redirect.occurrence.admission_reason, "redirect_denied");
    assert!(!report.policy_ledger.unwrap().complete);
}

#[tokio::test]
async fn stopped_sitemap_redirects_keep_literal_invalid_loop_and_limit_evidence() {
    for (case, reason, literal, expected_rows) in [
        ("invalid", "redirect_invalid", "http://[broken", 1),
        ("loop", "redirect_stopped_loop", "/sitemap.xml", 2),
        ("limit", "redirect_stopped_limit", "/shop/map3.xml", 2),
        ("missing", "", "", 0),
    ] {
        let fixture = Fixture::new(move |_, path| match (case, path) {
            ("invalid", "/sitemap.xml") => Response::html("")
                .status(302)
                .header("Location", "http://[broken"),
            ("missing", "/sitemap.xml") => Response::html("").status(302),
            (_, "/sitemap.xml") => Response::html("")
                .status(302)
                .header("Location", "/child.xml"),
            ("loop", "/child.xml") => Response::html("")
                .status(302)
                .header("Location", "/sitemap.xml"),
            ("limit", "/child.xml") => Response::html("")
                .status(302)
                .header("Location", "/shop/map3.xml"),
            _ => Response::html("<h1>Body</h1>"),
        })
        .await;
        let mut config = config(&fixture);
        config.sitemap_urls = vec![fixture.url("/sitemap.xml")];
        if case == "limit" {
            config.max_redirects = 1;
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("stopped.sqlite");
        let report = run(config, &path).await;
        let rows = ledger(&path)
            .into_iter()
            .filter(|row| row.occurrence.decision.source_kind == CandidateSourceKind::Redirect)
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), expected_rows, "{case}");
        if let Some(last) = rows.last() {
            assert_eq!(last.occurrence.decision.raw_value, literal, "{case}");
            assert_eq!(last.occurrence.admission_reason, reason, "{case}");
            assert!(last.occurrence.raw_value_observed);
        }
        assert_eq!(fixture.count("/sitemap.xml"), 1, "{case}");
        assert_eq!(fixture.count("/shop/map3.xml"), 0, "{case}");
        assert_eq!(
            fixture.count("/child.xml"),
            usize::from(matches!(case, "loop" | "limit")),
            "{case}"
        );
        assert!(!report.policy_ledger.unwrap().complete, "{case}");
    }
}

fn formula_workbook(url: &str) -> Vec<u8> {
    use std::io::{Cursor, Write};
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let files=[
        ("[Content_Types].xml","<Types xmlns='http://schemas.openxmlformats.org/package/2006/content-types'><Default Extension='rels' ContentType='application/vnd.openxmlformats-package.relationships+xml'/><Override PartName='/xl/workbook.xml' ContentType='application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml'/><Override PartName='/xl/worksheets/sheet1.xml' ContentType='application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml'/></Types>".into()),
        ("_rels/.rels","<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='rId1' Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument' Target='xl/workbook.xml'/></Relationships>".into()),
        ("xl/workbook.xml","<workbook xmlns='http://schemas.openxmlformats.org/spreadsheetml/2006/main' xmlns:r='http://schemas.openxmlformats.org/officeDocument/2006/relationships'><sheets><sheet name='URLs' sheetId='1' r:id='rId1'/></sheets></workbook>".into()),
        ("xl/_rels/workbook.xml.rels","<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='rId1' Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet' Target='worksheets/sheet1.xml'/></Relationships>".into()),
        ("xl/worksheets/sheet1.xml",format!("<worksheet xmlns='http://schemas.openxmlformats.org/spreadsheetml/2006/main'><sheetData><row r='1'><c r='A1' t='str'><f>\"{url}\"</f><v>{url}</v></c></row></sheetData></worksheet>")),
    ];
    for (name, body) in files {
        archive
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(body.as_bytes()).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

#[tokio::test]
async fn rejected_formula_cache_is_observed_but_cannot_be_readmitted_from_raw_url() {
    let fixture = Fixture::new(|_, _| Response::html("<h1>Body</h1>")).await;
    let bytes = formula_workbook(&fixture.url("/shop/formula-cache"));
    for policy in [
        ListFormulaPolicy::Reject,
        ListFormulaPolicy::CachedStringOnly,
    ] {
        let input = import_url_list(
            &bytes,
            &ListImportOptions {
                format: ListImportFormat::Xlsx {
                    sheet: ListSheetSelector::Name {
                        value: "URLs".into(),
                    },
                    column: ListColumnSelector::Index { value: 1 },
                    start_row: 1,
                    formula_policy: policy,
                },
                limits: ListImportLimits::default(),
            },
        )
        .unwrap();
        assert_eq!(
            input.rows[0].error.is_some(),
            policy == ListFormulaPolicy::Reject
        );
        let mut config = config(&fixture);
        config.input_imports = vec![input];
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("formula.sqlite");
        run(config, &path).await;
        let rows = ledger(&path);
        let row = rows
            .iter()
            .find(|row| row.occurrence.decision.raw_value == fixture.url("/shop/formula-cache"))
            .unwrap();
        if policy == ListFormulaPolicy::Reject {
            assert_eq!(fixture.count("/shop/formula-cache"), 0);
            assert_eq!(row.occurrence.admission_reason, "input_row_rejected");
            assert!(row.occurrence.input_error.is_some());
        } else {
            assert_eq!(fixture.count("/shop/formula-cache"), 1);
            assert_eq!(row.occurrence.admission_reason, "scheduled");
        }
    }
}

#[tokio::test]
async fn tampered_ledger_permissions_and_unknown_engines_fail_before_new_generation_or_http() {
    let fixture = Fixture::new(|_, path| {
        if path == "/shop/start" {
            Response::html("<a href='/outside/check'>Check only</a>")
        } else {
            Response::html("<h1>Body</h1>")
        }
    })
    .await;
    for mutation in [
        "UPDATE policy_admissions SET data=json_set(data,'$.can_expand',true) WHERE url LIKE '%/outside/check'",
        "UPDATE policy_admissions SET data=json_set(data,'$.inherited',true) WHERE url LIKE '%/outside/check'",
        "UPDATE policy_occurrences SET data=json_set(data,'$.occurrence.decision.raw_value','tampered') WHERE sequence=1",
        "UPDATE metadata SET value=json_set(value,'$.engine','unknown-future') WHERE key='policy_ledger'",
        "UPDATE policy_occurrences SET generation=0 WHERE sequence=1",
        "UPDATE policy_occurrences SET generation=999 WHERE sequence=1",
    ] {
        let mut config = config(&fixture);
        config.max_pages = Some(1);
        config.crawl_policy.as_mut().unwrap().scope.outside_folder = OutsideScopePolicy::CheckOnce;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("tamper.sqlite");
        run(config.clone(), &path).await;
        assert_eq!(fixture.count("/outside/check"), 0);
        let connection = rusqlite::Connection::open(&path).unwrap();
        let before: String = connection
            .query_row(
                "SELECT value FROM metadata WHERE key='run_state'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        connection.execute_batch(mutation).unwrap();
        assert!(CrawlStore::inspect(&path).is_err());
        let mut callback = false;
        assert!(
            CrawlStore::open_read_only(&path)
                .unwrap()
                .visit_policy_occurrences(1, |_| {
                    callback = true;
                    Ok(())
                })
                .is_err()
        );
        assert!(!callback);
        assert!(
            Crawler::new(config)
                .unwrap()
                .with_store(&path)
                .unwrap()
                .run_controlled(None, CrawlControl::new())
                .await
                .is_err()
        );
        let after: String = connection
            .query_row(
                "SELECT value FROM metadata WHERE key='run_state'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before, after);
        assert_eq!(fixture.count("/outside/check"), 0);
    }
}
