//! Actual public crawler -> saved SQLite -> actual HTTP route controls.
//! AUTHORED ONLY: no runtime or physical resource-limit acceptance implied.
use super::*;
use open_seo_spider_core::{SitemapFindingAssessmentSelection, SitemapFindingFilter};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use tokio::sync::oneshot;

struct OwnedDirectory(PathBuf);
impl OwnedDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("oss-findings-http-{}", Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Http {
    base: String,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}
impl Http {
    async fn start(router: Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
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
            base,
            stop: Some(stop),
            task: Some(task),
        }
    }
    async fn finish(mut self) {
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

struct Fixture {
    dir: OwnedDirectory,
    state: AppState,
    id: Uuid,
    site: Http,
    requests: Arc<AtomicUsize>,
}
impl Fixture {
    async fn new() -> Self {
        let requests = Arc::new(AtomicUsize::new(0));
        let count = requests.clone();
        let router = Router::new().fallback(move |uri: axum::http::Uri, headers:axum::http::HeaderMap| {
            let count = count.clone();
            async move {
                count.fetch_add(1,Ordering::SeqCst);
                if uri.path() == "/task.xml" {
                    let host = headers[header::HOST].to_str().unwrap();
                    let xml = format!("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'>{}</urlset>", (0..6).map(|i|format!("<url><loc>http://{host}/p{i}</loc></url>")).collect::<String>());
                    ([(header::CONTENT_TYPE,"application/xml")], xml)
                } else {
                    ([(header::CONTENT_TYPE,"text/html")], "<html><head><title>actual fixture</title></head><body>saved page</body></html>".to_string())
                }
            }
        });
        let site = Http::start(router).await;
        let dir = OwnedDirectory::new();
        let id = Uuid::new_v4();
        let config = CrawlConfig {
            seed_urls: (0..12).map(|i| format!("{}/p{i}", site.base)).collect(),
            sitemap_urls: vec![format!("{}/task.xml", site.base)],
            concurrency: 1,
            delay_ms: 0,
            obey_robots: false,
            discover_sitemaps: false,
            crawl_links: false,
            request_timeout_secs: 3,
            ..Default::default()
        };
        let crawler = Crawler::new(config)
            .unwrap()
            .with_store(dir.0.join(format!("{id}.sqlite")))
            .unwrap();
        let result = crawler
            .run_controlled(None, CrawlControl::new())
            .await
            .unwrap();
        assert_eq!(result.outcome, CrawlOutcome::Completed);
        assert_eq!(result.report.pages.len(), 12);
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
            discovery_pending: 0,
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
        let state = AppState {
            mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
            jobs: Arc::new(RwLock::new(HashMap::from([(
                id,
                Arc::new(RwLock::new(job)),
            )]))),
            data_dir: Arc::new(dir.0.clone()),
            exports: Default::default(),
            finding_reads: Default::default(),
        };
        drop(crawler);
        // Establish SQLite read-only WAL-index startup before byte invariance.
        CrawlStore::open_read_only(dir.0.join(format!("{id}.sqlite")))
            .unwrap()
            .sitemap_read_snapshot()
            .unwrap();
        assert_eq!(requests.load(Ordering::SeqCst), 13);
        Self {
            dir,
            state,
            id,
            site,
            requests,
        }
    }
    async fn api(&self) -> Http {
        Http::start(routes().with_state(self.state.clone())).await
    }
    fn url(&self, api: &Http) -> String {
        format!("{}/api/crawls/{}/sitemaps/findings", api.base, self.id)
    }
    async fn finish(self, api: Http) {
        self.state.exports.shutdown_and_drain().await.unwrap();
        self.state.finding_reads.shutdown_and_drain().await.unwrap();
        api.finish().await;
        self.site.finish().await;
        assert_eq!(
            self.requests.load(Ordering::SeqCst),
            13,
            "saved findings made no website requests"
        );
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap()
}
fn files(path: &std::path::Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(path)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}
fn query() -> ReadQuery {
    ReadQuery {
        filter: SitemapFindingFilter::UrlsInSitemap,
        assessment: SitemapFindingAssessmentSelection::All,
        limit: Some(2),
        ..Default::default()
    }
}
fn csv_query(page: &Value) -> Value {
    json!({"filter":"urls-in-sitemap","assessment":"all","requested_url":null,"source_task_url":null,"search":null,"snapshot":page["export_token"]})
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn findings_actual_http_get_post_cursor_and_full_csv_share_saved_context_without_site_requests()
 {
    let fixture = Fixture::new().await;
    let api = fixture.api().await;
    let before = files(&fixture.dir.0);
    let url = fixture.url(&api);
    let response = client().post(&url).json(&query()).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let first: Value = response.json().await.unwrap();
    assert_eq!(first["website_requests"], 0);
    assert_eq!(first["findings"]["rows"].as_array().unwrap().len(), 2);
    assert!(first["next_cursor"].is_string());
    let continuation = client()
        .get(&url)
        .query(&[
            ("filter", "urls-in-sitemap"),
            ("assessment", "all"),
            ("limit", "3"),
            ("cursor", first["next_cursor"].as_str().unwrap()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(continuation.status(), StatusCode::OK);
    let next: Value = continuation.json().await.unwrap();
    assert_eq!(next["findings"]["rows"].as_array().unwrap().len(), 3);
    assert_eq!(
        next["findings"]["context"]["context_sha256"],
        first["findings"]["context"]["context_sha256"]
    );
    assert_ne!(
        next["findings"]["rows"][0]["position"],
        first["findings"]["rows"][0]["position"]
    );
    let response = client()
        .post(format!("{url}/export.csv"))
        .json(&csv_query(&first))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let length: usize = response.headers()[header::CONTENT_LENGTH]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    let hash = response.headers()["x-sitemap-csv-sha256"]
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        response.headers()["x-sitemap-findings-context-sha256"]
            .to_str()
            .unwrap(),
        first["findings"]["context"]["context_sha256"]
            .as_str()
            .unwrap()
    );
    let bytes = response.bytes().await.unwrap();
    assert_eq!(bytes.len(), length);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), hash);
    let csv = String::from_utf8(bytes.to_vec()).unwrap();
    // All JSON newlines are escaped; this fixture's original URLs contain none.
    assert_eq!(
        csv.split("\r\n").filter(|s| !s.is_empty()).count(),
        14,
        "header + actual context + all12 saved pages, not only the2-row JSON page"
    );
    assert!(csv.starts_with("\"record_type\",\"schema_version\""));
    assert_eq!(files(&fixture.dir.0), before);
    fixture.finish(api).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn findings_actual_http_rejects_wrong_roles_changed_queries_unknown_fields_and_stale_app_generation()
 {
    let fixture = Fixture::new().await;
    let api = fixture.api().await;
    let url = fixture.url(&api);
    let first: Value = client()
        .post(&url)
        .json(&query())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let malformed = client()
        .post(&url)
        .json(&json!({"filter":"urls-in-sitemap","unexpected":true}))
        .send()
        .await
        .unwrap();
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    let role = client()
        .post(&url)
        .json(&json!({"cursor":first["export_token"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(role.status(), StatusCode::BAD_REQUEST);
    let changed = client().post(&url).json(&json!({"filter":"urls-not-in-sitemap","assessment":"all","cursor":first["next_cursor"]})).send().await.unwrap();
    assert_eq!(changed.status(), StatusCode::CONFLICT);
    let job = get_job(&fixture.state, fixture.id).await.ok().unwrap();
    job.write().await.generation += 1;
    let stale = client()
        .post(format!("{url}/export.csv"))
        .json(&csv_query(&first))
        .send()
        .await
        .unwrap();
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert!(stale.headers().get(header::CONTENT_DISPOSITION).is_none());
    let stale: Value = stale.json().await.unwrap();
    assert_eq!(stale["code"], "stale_sitemap_findings_cursor");
    fixture.finish(api).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn findings_actual_http_detached_report_is_explicitly_unavailable_without_fabricated_core_proof()
 {
    let fixture = Fixture::new().await;
    let detached = OwnedDirectory::new();
    let mut state = fixture.state.clone();
    state.data_dir = Arc::new(detached.0.clone());
    let api = Http::start(routes().with_state(state)).await;
    let response = client().get(fixture.url(&api)).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let result: Value = response.json().await.unwrap();
    assert!(result["findings"].is_null());
    assert!(result["export_token"].is_null());
    assert!(result["next_cursor"].is_null());
    assert_eq!(result["availability_reason"], "saved_sqlite_unavailable");
    assert_eq!(result["website_requests"], 0);
    assert!(files(&detached.0).is_empty());
    fixture.finish(api).await;
}
