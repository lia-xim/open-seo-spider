//! AUTHORED_NOT_RUN. Real finite HTTP, durable admissions, recovery and no target synthesis.
use super::*;
use crate::{CrawlQuotaConfig, CrawlQuotaIncreases, CrawlUrlPatternQuota};
use std::sync::Mutex;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    task::JoinSet,
};

struct Site {
    origin: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}
impl Site {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = requests.clone();
        let (stop, mut stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut children = JoinSet::new();
            loop {
                tokio::select! {_=&mut stopped=>break,accepted=listener.accept()=>{let(mut stream,_)=accepted.unwrap();let log=log.clone();children.spawn(async move{let mut raw=Vec::new();let mut chunk=[0u8;1024];while !raw.windows(4).any(|b|b==b"\r\n\r\n"){let n=stream.read(&mut chunk).await.unwrap();assert!(n>0);raw.extend_from_slice(&chunk[..n]);assert!(raw.len()<16384);}let text=String::from_utf8(raw).unwrap();let target=text.lines().next().unwrap().split_whitespace().nth(1).unwrap().to_owned();log.lock().unwrap().push(target.clone());let(status,extra,body)=match target.as_str(){"/depth"=>(200,"","<a href='/d/1'>1</a><a href='/d/1#x'>dup</a><a href='/d/2'>2</a><a href='/d/3'>3</a>".to_owned()),"/d/1"=>(200,"","<a href='/e/1'>1</a><a href='/e/2'>2</a><a href='/e/3'>3</a>".to_owned()),"/race"=>(200,"",(1..=8).map(|n|format!("<a href='/r/{n}'>r</a>")).collect()),"/atomic"=>(200,"","<a href='/p/1'>denied</a><a href='/free/1'>1</a><a href='/free/2'>2</a>".to_owned()),"/jump"=>(302,"Location: /p/next\r\n",String::new()),"/robots.txt"=>(200,"","User-agent: *\nAllow: /\n".to_owned()),_ =>(200,"",String::new())};let response=format!("HTTP/1.1 {status} Owned\r\nContent-Type: text/html; charset=utf-8\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());stream.write_all(response.as_bytes()).await.unwrap();stream.shutdown().await.unwrap();});}}
            }
            drop(listener);
            while let Some(result) = children.join_next().await {
                result.unwrap();
            }
        });
        Self {
            origin,
            requests,
            stop: Some(stop),
            task: Some(task),
        }
    }
    async fn finish(mut self) -> Vec<String> {
        self.stop.take().unwrap().send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        self.requests.lock().unwrap().clone()
    }
}
impl Drop for Site {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
fn cfg(site: &Site, path: &str, quota: CrawlQuotaConfig) -> CrawlConfig {
    CrawlConfig {
        seed_urls: vec![format!("{}{path}", site.origin)],
        crawl_quotas: Some(quota),
        discover_sitemaps: false,
        obey_robots: false,
        delay_ms: 0,
        concurrency: 1,
        ..Default::default()
    }
}
fn quotas(
    depth: Option<u64>,
    host: Option<u64>,
    rules: Vec<CrawlUrlPatternQuota>,
) -> CrawlQuotaConfig {
    CrawlQuotaConfig {
        version: 1,
        max_urls_per_depth: depth,
        max_urls_per_subdomain: host,
        url_pattern_quotas: rules,
    }
}
async fn run(c: CrawlConfig, p: &Path) -> ControlledCrawlResult {
    Crawler::new(c)
        .unwrap()
        .with_store(p)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap()
}

#[tokio::test]
async fn quotas_real_depth_limits_preserve_duplicate_links_and_resume_only_new_targets() {
    let site = Site::start().await;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let config = cfg(&site, "/depth", quotas(Some(2), None, vec![]));
    let first = run(config, &p).await;
    assert_eq!(first.outcome, CrawlOutcome::Limited);
    assert_eq!(first.report.pages.len(), 5);
    assert_eq!(first.report.summary.pending, 2);
    let root = first
        .report
        .pages
        .iter()
        .find(|r| r.url.ends_with("/depth"))
        .unwrap()
        .clone();
    assert_eq!(root.links.len(), 4);
    let store = CrawlStore::open_read_only(&p).unwrap();
    let before = store.crawl_quota_summary().unwrap().unwrap();
    let cp = store.checkpoint().unwrap().unwrap();
    assert_eq!(before.deferred_targets, 2);
    assert_eq!(before.active_attempts, 0);
    let inc = CrawlQuotaIncreases {
        expected_generation: cp.generation,
        expected_profile_revision: before.profile_revision,
        expected_profile_sha256: before.profile_sha256.clone(),
        max_urls_per_depth: Some(3),
        max_urls_per_subdomain: None,
        url_pattern_maxima: Default::default(),
    };
    let config = store.preview_crawl_quota_increases(&inc).unwrap();
    drop(store);
    let resumed = Crawler::from_saved_config(config)
        .unwrap()
        .with_store(&p)
        .unwrap()
        .with_quota_increases(inc.clone())
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    assert_eq!(resumed.report.pages.len(), 7);
    assert_eq!(resumed.report.summary.pending, 0);
    assert_eq!(
        serde_json::to_value(
            resumed
                .report
                .pages
                .iter()
                .find(|r| r.url == root.url)
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(root).unwrap()
    );
    let cold = CrawlStore::open_read_only(&p).unwrap();
    let after = cold.crawl_quota_summary().unwrap().unwrap();
    assert_eq!(after.profile_revision, 2);
    assert_eq!(after.reserved_distinct_targets, 7);
    assert_eq!(after.deferred_targets, 0);
    assert!(cold.preview_crawl_quota_increases(&inc).is_err());
    assert!(cold.inspection().unwrap().checkpoint_valid);
    let mut got = site.finish().await;
    got.sort();
    assert_eq!(
        got,
        vec!["/d/1", "/d/2", "/d/3", "/depth", "/e/1", "/e/2", "/e/3"]
    );
}

#[tokio::test]
async fn quotas_real_concurrent_starts_cannot_overspend_depth_capacity() {
    let site = Site::start().await;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let mut c = cfg(&site, "/race", quotas(Some(2), None, vec![]));
    c.concurrency = 8;
    let r = run(c, &p).await;
    assert_eq!(r.report.pages.len(), 3);
    assert_eq!(r.report.summary.pending, 6);
    let s = CrawlStore::open_read_only(&p)
        .unwrap()
        .crawl_quota_summary()
        .unwrap()
        .unwrap();
    assert_eq!(s.reserved_distinct_targets, 3);
    assert_eq!(s.deferred_targets, 6);
    assert_eq!(s.active_attempts, 0);
    let got = site.finish().await;
    assert_eq!(got.len(), 3);
    assert!(got.contains(&"/race".into()));
    assert_eq!(
        got.iter()
            .filter(|p| p.starts_with("/r/") && (1..=8).any(|n| p.as_str() == format!("/r/{n}")))
            .count(),
        2
    );
}

#[tokio::test]
async fn quotas_real_all_dimension_denial_keeps_unrelated_positive_branches() {
    let site = Site::start().await;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let c = cfg(
        &site,
        "/atomic",
        quotas(
            Some(2),
            Some(3),
            vec![CrawlUrlPatternQuota {
                id: "p".into(),
                pattern: "/p/".into(),
                max_urls: 0,
            }],
        ),
    );
    let r = run(c, &p).await;
    assert_eq!(r.report.pages.len(), 3);
    assert_eq!(r.report.summary.pending, 1);
    assert!(r.report.pages.iter().all(|p| !p.url.ends_with("/p/1")));
    let s = CrawlStore::open_read_only(&p)
        .unwrap()
        .crawl_quota_summary()
        .unwrap()
        .unwrap();
    assert_eq!(s.reserved_distinct_targets, 3);
    assert!(s.causes.contains(&"pattern:p".to_owned()));
    let mut got = site.finish().await;
    got.sort();
    assert_eq!(got, vec!["/atomic", "/free/1", "/free/2"]);
}

#[tokio::test]
async fn quotas_real_redirect_deferred_preserves_actual302_separately_and_never_fetches_target() {
    let site = Site::start().await;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let c = cfg(
        &site,
        "/jump",
        quotas(
            None,
            None,
            vec![CrawlUrlPatternQuota {
                id: "p".into(),
                pattern: "/p/".into(),
                max_urls: 0,
            }],
        ),
    );
    let r = run(c, &p).await;
    assert_eq!(r.outcome, CrawlOutcome::Limited);
    assert!(r.report.pages.is_empty());
    assert_eq!(r.report.summary.pending, 1);
    let conn =
        rusqlite::Connection::open_with_flags(&p, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let status: Option<i64> = conn
        .query_row(
            "SELECT response_status FROM crawl_quota_attempts WHERE target=?1",
            [format!("{}/jump", site.origin)],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, Some(302));
    let s = CrawlStore::open_read_only(&p)
        .unwrap()
        .crawl_quota_summary()
        .unwrap()
        .unwrap();
    assert_eq!(s.actual_started_attempts, 1);
    assert_eq!(s.deferred_targets, 1);
    assert_eq!(site.finish().await, vec!["/jump"]);
}

#[tokio::test]
async fn quotas_zero_admission_precedes_robots_and_rate_and_cold_read_has_no_website_io() {
    let site = Site::start().await;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let mut c = cfg(&site, "/zero", quotas(Some(0), None, vec![]));
    c.obey_robots = true;
    c.delay_ms = 60000;
    let r = tokio::time::timeout(Duration::from_secs(3), run(c, &p))
        .await
        .unwrap();
    assert_eq!(r.outcome, CrawlOutcome::Limited);
    assert!(r.report.pages.is_empty());
    assert_eq!(r.report.summary.pending, 1);
    let s = CrawlStore::open_read_only(&p)
        .unwrap()
        .crawl_quota_summary()
        .unwrap()
        .unwrap();
    assert_eq!(s.reserved_distinct_targets, 0);
    assert_eq!(s.actual_started_attempts, 0);
    assert_eq!(s.deferred_targets, 1);
    assert_eq!(
        CrawlStore::open_read_only(&p)
            .unwrap()
            .crawl_quota_summary()
            .unwrap(),
        Some(s)
    );
    assert!(site.finish().await.is_empty());
}

#[tokio::test]
async fn quotas_real_host_capacity_is_shared_across_explicit_seed_ports() {
    let a = Site::start().await;
    let b = Site::start().await;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let mut c = cfg(&a, "/h/1", quotas(None, Some(2), vec![]));
    c.seed_urls
        .extend([format!("{}/h/2", b.origin), format!("{}/h/3", a.origin)]);
    c.crawl_links = false;
    let r = run(c, &p).await;
    assert_eq!(r.report.pages.len(), 2);
    assert_eq!(r.report.summary.pending, 1);
    let s = CrawlStore::open_read_only(&p)
        .unwrap()
        .crawl_quota_summary()
        .unwrap()
        .unwrap();
    assert_eq!(s.bucket_rows.len(), 1);
    assert_eq!(s.bucket_rows[0].dimension, "host");
    assert_eq!(s.bucket_rows[0].key, "127.0.0.1");
    assert_eq!(s.bucket_rows[0].used, 2);
    assert_eq!(a.finish().await, vec!["/h/1"]);
    assert_eq!(b.finish().await, vec!["/h/2"]);
}

#[tokio::test]
async fn quotas_actual_pause_drains_held_request_then_cold_resume_reuses_same_ledger() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let (held, held_rx) = oneshot::channel();
    let (release, release_rx) = oneshot::channel();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let log = requests.clone();
    let server = tokio::spawn(async move {
        let mut held = Some(held);
        let mut release = Some(release_rx);
        for _ in 0..3 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut raw = Vec::new();
            let mut buf = [0u8; 1024];
            while !raw.windows(4).any(|b| b == b"\r\n\r\n") {
                let n = socket.read(&mut buf).await.unwrap();
                assert!(n > 0);
                raw.extend_from_slice(&buf[..n]);
                assert!(raw.len() < 16384);
            }
            let target = String::from_utf8(raw)
                .unwrap()
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
                .to_owned();
            log.lock().unwrap().push(target.clone());
            if target == "/hold" {
                held.take().unwrap().send(()).unwrap();
                release.take().unwrap().await.unwrap();
            }
            let body = if target == "/start" {
                "<a href='/hold'>hold</a><a href='/tail'>tail</a>"
            } else {
                ""
            };
            let response = format!(
                "HTTP/1.1 200 Owned\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        }
    });
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let c = CrawlConfig {
        seed_urls: vec![format!("{origin}/start")],
        crawl_quotas: Some(quotas(Some(2), Some(3), vec![])),
        discover_sitemaps: false,
        obey_robots: false,
        delay_ms: 0,
        concurrency: 1,
        ..Default::default()
    };
    let crawler = Crawler::new(c).unwrap().with_store(&p).unwrap();
    let control = CrawlControl::new();
    let run_control = control.clone();
    let task = tokio::spawn(async move { crawler.run_controlled(None, run_control).await });
    tokio::time::timeout(Duration::from_secs(5), held_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(control.request_pause());
    release.send(()).unwrap();
    let first = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(first.outcome, CrawlOutcome::Paused);
    assert_eq!(first.report.pages.len(), 2);
    assert_eq!(first.report.summary.pending, 1);
    assert_eq!(requests.lock().unwrap().as_slice(), ["/start", "/hold"]);
    let cold = CrawlStore::open_read_only(&p).unwrap();
    let before = cold.crawl_quota_summary().unwrap().unwrap();
    assert_eq!(before.reserved_distinct_targets, 2);
    assert_eq!(before.active_attempts, 0);
    let config = cold.config().unwrap().unwrap();
    assert!(cold.inspection().unwrap().checkpoint_valid);
    drop(cold);
    assert_eq!(requests.lock().unwrap().len(), 2);
    let result = Crawler::from_saved_config(config)
        .unwrap()
        .with_store(&p)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(result.outcome, CrawlOutcome::Completed);
    assert_eq!(result.report.pages.len(), 3);
    let after = CrawlStore::open_read_only(&p)
        .unwrap()
        .crawl_quota_summary()
        .unwrap()
        .unwrap();
    assert_eq!(after.profile_revision, before.profile_revision);
    assert_eq!(after.reserved_distinct_targets, 3);
    assert_eq!(after.actual_started_attempts, 3);
    assert_eq!(after.active_attempts, 0);
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        requests.lock().unwrap().as_slice(),
        ["/start", "/hold", "/tail"]
    );
}

#[tokio::test]
async fn quotas_actual_policy_alias_is_charged_without_overwriting_two_original_inputs() {
    let site = Site::start().await;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let raw1 = format!("{}/alias?drop=1&keep=3", site.origin);
    let raw2 = format!("{}/alias?drop=2&keep=3", site.origin);
    let effective = format!("{}/alias?keep=3", site.origin);
    let mut c = cfg(
        &site,
        "/",
        quotas(
            None,
            None,
            vec![CrawlUrlPatternQuota {
                id: "effective".into(),
                pattern: "/alias[?]keep=3$".into(),
                max_urls: 0,
            }],
        ),
    );
    c.seed_urls = vec![raw1.clone(), raw2.clone()];
    c.crawl_links = false;
    c.crawl_policy = Some(crate::CrawlPolicyConfig {
        scope: crate::CrawlScopeConfig {
            roots: vec![crate::CrawlScopeRoot {
                origin: site.origin.clone(),
                subdomains: false,
                folders: vec!["/".into()],
                exact_paths: vec![],
            }],
            ..Default::default()
        },
        query: crate::QueryPolicy {
            remove_keys: vec!["drop".into()],
            ..Default::default()
        },
        initial_inputs: crate::InitialInputPolicy {
            rewrite: crate::InitialRewritePolicy::Apply,
            ..Default::default()
        },
        ..Default::default()
    });
    let r = run(c, &p).await;
    assert_eq!(r.outcome, CrawlOutcome::Limited);
    assert!(r.report.pages.is_empty());
    assert_eq!(r.report.summary.pending, 1);
    let store = CrawlStore::open_read_only(&p).unwrap();
    let rows = store.policy_occurrences(None, 200).unwrap();
    for raw in [&raw1, &raw2] {
        let row = rows
            .rows
            .iter()
            .find(|r| &r.occurrence.decision.raw_value == raw)
            .unwrap();
        assert_eq!(
            row.occurrence.decision.fetch_candidate_url.as_deref(),
            Some(effective.as_str())
        );
    }
    let summary = store.crawl_quota_summary().unwrap().unwrap();
    assert_eq!(summary.reserved_distinct_targets, 0);
    assert_eq!(summary.deferred_targets, 1);
    assert!(summary.causes.contains(&"pattern:effective".to_owned()));
    assert!(site.finish().await.is_empty());
}

#[tokio::test]
async fn quotas_unsupported_renderer_resource_and_memory_paths_refuse_without_website_requests() {
    let site = Site::start().await;
    let base = cfg(&site, "/", quotas(Some(1), None, vec![]));
    let mut renderer = base.clone();
    renderer.renderer = Some(crate::RendererConfig::default());
    assert!(Crawler::new(renderer).is_err());
    let mut resource = base.clone();
    resource.resource_discovery = Some(crate::ResourceDiscoveryConfig::default());
    assert!(Crawler::new(resource).is_err());
    assert!(
        Crawler::new(base)
            .unwrap()
            .run_controlled(None, CrawlControl::new())
            .await
            .is_err()
    );
    assert!(site.finish().await.is_empty());
}

#[tokio::test]
async fn quotas_cold_same_count_response_mutation_is_detected_by_full_ledger_binding() {
    let site = Site::start().await;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crawl.sqlite");
    let report = run(
        cfg(&site, "/cold-integrity", quotas(Some(1), Some(1), vec![])),
        &path,
    )
    .await;
    assert_eq!(report.outcome, CrawlOutcome::Completed);
    let before = CrawlStore::open_read_only(&path)
        .unwrap()
        .crawl_quota_summary()
        .unwrap()
        .unwrap();
    assert_eq!(before.actual_started_attempts, 1);
    assert_eq!(before.settled_attempts, 1);
    assert_eq!(site.finish().await, vec!["/cold-integrity"]);
    // Negative-only corruption: counts and target identities stay unchanged.
    // The actual terminal binding must still reject an altered response fact.
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        conn.execute(
            "UPDATE crawl_quota_attempts SET response_status=201 WHERE response_status=200",
            []
        )
        .unwrap(),
        1
    );
    drop(conn);
    assert!(
        CrawlStore::open_read_only(&path)
            .unwrap()
            .crawl_quota_summary()
            .is_err()
    );
    assert!(CrawlStore::inspect(&path).is_err());
}

#[tokio::test]
async fn quotas_two_actual_frontier_redirects_share_target_membership_but_own_attempts() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let log = Arc::new(Mutex::new(Vec::new()));
    let capture = log.clone();
    let server = tokio::spawn(async move {
        for _ in 0..4 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut raw = Vec::new();
            let mut buf = [0u8; 1024];
            while !raw.windows(4).any(|b| b == b"\r\n\r\n") {
                let n = socket.read(&mut buf).await.unwrap();
                assert!(n > 0);
                raw.extend_from_slice(&buf[..n]);
                assert!(raw.len() < 16384);
            }
            let target = String::from_utf8(raw)
                .unwrap()
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
                .to_owned();
            capture.lock().unwrap().push(target.clone());
            let response = if target == "/shared" {
                "HTTP/1.1 200 Owned\r\nContent-Type: text/html\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            } else {
                assert!(matches!(target.as_str(), "/one" | "/two"));
                "HTTP/1.1 302 Owned\r\nLocation: /shared\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            };
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        }
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crawl.sqlite");
    let config = CrawlConfig {
        seed_urls: vec![format!("{origin}/one"), format!("{origin}/two")],
        crawl_quotas: Some(quotas(
            None,
            None,
            vec![CrawlUrlPatternQuota {
                id: "shared".into(),
                pattern: "/shared$".into(),
                max_urls: 1,
            }],
        )),
        concurrency: 2,
        discover_sitemaps: false,
        obey_robots: false,
        delay_ms: 0,
        crawl_links: false,
        ..Default::default()
    };
    let result = run(config, &path).await;
    assert_eq!(result.outcome, CrawlOutcome::Completed);
    assert_eq!(result.report.pages.len(), 2);
    assert!(
        result
            .report
            .pages
            .iter()
            .all(|p| p.final_url == format!("{origin}/shared") && p.final_status == Some(200))
    );
    let summary = CrawlStore::open_read_only(&path)
        .unwrap()
        .crawl_quota_summary()
        .unwrap()
        .unwrap();
    assert_eq!(summary.reserved_distinct_targets, 3);
    assert_eq!(summary.actual_started_attempts, 4);
    assert_eq!(summary.settled_attempts, 4);
    assert_eq!(summary.active_attempts, 0);
    assert_eq!(summary.deferred_targets, 0);
    assert_eq!(summary.bucket_rows.len(), 1);
    assert_eq!(summary.bucket_rows[0].used, 1);
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
    let mut requests = log.lock().unwrap().clone();
    requests.sort();
    assert_eq!(requests, vec!["/one", "/shared", "/shared", "/two"]);
}

#[tokio::test]
async fn quotas_cold_missing_exactly_once_index_refuses_without_repair_or_website_io() {
    let site = Site::start().await;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crawl.sqlite");
    let result = run(
        cfg(&site, "/cold-index", quotas(Some(1), Some(1), vec![])),
        &path,
    )
    .await;
    assert_eq!(result.outcome, CrawlOutcome::Completed);
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
    assert_eq!(site.finish().await, vec!["/cold-index"]);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP INDEX crawl_quota_one_task_target")
        .unwrap();
    drop(connection);
    assert!(
        CrawlStore::open_read_only(&path)
            .unwrap()
            .crawl_quota_summary()
            .is_err()
    );
    assert!(CrawlStore::inspect(&path).is_err());
    let connection =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let remains: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='crawl_quota_one_task_target')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!remains);
}

#[tokio::test]
async fn quotas_actual_cancel_drops_held_request_keeps_paid_memberships_and_cold_unknown_checkpoint()
 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let url = format!("{origin}/cancel-held");
    let (captured, captured_rx) = oneshot::channel();
    let (release, mut release_rx) = oneshot::channel();
    let website = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 1024];
        while !bytes.windows(4).any(|part| part == b"\r\n\r\n") {
            let count = socket.read(&mut buffer).await.unwrap();
            assert!(count > 0);
            bytes.extend_from_slice(&buffer[..count]);
            assert!(bytes.len() < 16384);
        }
        let request = String::from_utf8(bytes).unwrap();
        let line = request.lines().next().unwrap().to_owned();
        assert_eq!(line, "GET /cancel-held HTTP/1.1");
        captured.send(line.clone()).unwrap();
        // Keep the real socket and withhold the whole response until the test
        // has observed cancellation, durable unknown state and cold recovery.
        // Continue watching this owned listener so any extra website request
        // during those reads cannot hide behind an unaccepted connection.
        tokio::select! {
            released = &mut release_rx => released.unwrap(),
            unexpected = listener.accept() => {
                let (extra, _) = unexpected.unwrap();
                drop(extra);
                panic!("A cancelled quota crawl or cold read opened another website connection");
            }
        }
        let listener = listener.into_std().unwrap();
        listener.set_nonblocking(true).unwrap();
        match listener.accept() {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Ok((extra, _)) => {
                drop(extra);
                panic!("Another website connection was queued before fixture release");
            }
            Err(error) => panic!("Owned listener cleanup failed: {error}"),
        }
        // Explicit release, no response fabrication and no aborted fixture task.
        drop(socket);
        drop(listener);
        vec![line]
    });
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("cancel-quotas.sqlite");
    let config = CrawlConfig {
        seed_urls: vec![url.clone()],
        crawl_quotas: Some(quotas(
            Some(1),
            Some(1),
            vec![CrawlUrlPatternQuota {
                id: "cancel".into(),
                pattern: "/cancel-held$".into(),
                max_urls: 1,
            }],
        )),
        discover_sitemaps: false,
        obey_robots: false,
        delay_ms: 0,
        concurrency: 1,
        crawl_links: false,
        request_timeout_secs: 30,
        ..Default::default()
    };
    let crawler = Crawler::new(config).unwrap().with_store(&database).unwrap();
    let control = CrawlControl::new();
    let owned_control = control.clone();
    let run = tokio::spawn(async move { crawler.run_controlled(None, owned_control).await });
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), captured_rx)
            .await
            .unwrap()
            .unwrap(),
        "GET /cancel-held HTTP/1.1"
    );
    let active = CrawlStore::open_read_only(&database).unwrap();
    let before = active.crawl_quota_summary().unwrap().unwrap();
    assert_eq!(before.reserved_distinct_targets, 1);
    assert_eq!(before.actual_started_attempts, 1);
    assert_eq!(before.active_attempts, 1);
    assert_eq!(before.settled_attempts, 0);
    assert_eq!(before.interrupted_or_unknown_attempts, 0);
    assert!(!active.inspection().unwrap().checkpoint_valid);
    drop(active);
    assert!(control.request_cancel());
    let result = tokio::time::timeout(Duration::from_secs(5), run)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(result.outcome, CrawlOutcome::Cancelled);
    assert!(control.is_finished());
    assert!(result.report.pages.is_empty());
    assert_eq!(result.report.summary.completed, 0);
    assert_eq!(result.report.summary.pending, 1);
    assert!(result.report.summary.cancelled);
    let checkpoint = result.checkpoint.as_ref().unwrap();
    assert_eq!(checkpoint.outcome, CrawlOutcome::Cancelled);
    assert_eq!(checkpoint.completed, 0);
    assert_eq!(checkpoint.pending, 1);
    let stored = CrawlStore::open_read_only(&database).unwrap();
    let after = stored.crawl_quota_summary().unwrap().unwrap();
    assert_eq!(after.generation, before.generation);
    assert_eq!(after.profile_revision, before.profile_revision);
    assert_eq!(after.profile_sha256, before.profile_sha256);
    assert_eq!(after.reserved_distinct_targets, 1);
    assert_eq!(after.actual_started_attempts, 1);
    assert_eq!(after.interrupted_or_unknown_attempts, 1);
    assert_eq!(after.settled_attempts, 0);
    assert_eq!(after.active_attempts, 0);
    assert_eq!(after.deferred_targets, 0);
    assert_eq!(after.bucket_rows, before.bucket_rows);
    assert_eq!(after.bucket_rows.len(), 3);
    assert!(
        after
            .bucket_rows
            .iter()
            .all(|row| row.used == 1 && row.limit == 1)
    );
    assert!(stored.inspection().unwrap().checkpoint_valid);
    assert_eq!(stored.checkpoint().unwrap(), result.checkpoint);
    drop(stored);
    let cold = CrawlStore::open_read_only(&database).unwrap();
    assert_eq!(cold.crawl_quota_summary().unwrap(), Some(after.clone()));
    assert_eq!(cold.crawl_quota_summary().unwrap(), Some(after));
    let inspection = cold.inspection().unwrap();
    assert!(inspection.checkpoint_valid);
    assert_eq!(inspection.completed, 0);
    assert_eq!(inspection.pending, 1);
    assert_eq!(
        inspection.run_state.unwrap().outcome,
        Some(CrawlOutcome::Cancelled)
    );
    drop(cold);
    let connection = rusqlite::Connection::open_with_flags(
        &database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let attempt: (String, String, String, u64, Option<i64>) = connection
        .query_row(
            "SELECT task_url,target,state,started,response_status FROM crawl_quota_attempts",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    crate::crawl_quota_ledger::sql_u64(row, 3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(attempt, (url.clone(), url, "unknown".into(), 1, None));
    drop(connection);
    release.send(()).unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), website)
            .await
            .unwrap()
            .unwrap(),
        vec!["GET /cancel-held HTTP/1.1"]
    );
}
