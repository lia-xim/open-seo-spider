use open_seo_spider_core::{
    CrawlConfig, CrawlEvent, CrawlStore, Crawler, CustomExtractionConfig, RendererConfig,
    compare_crawls, export_custom_extractions_csv, export_links_csv, export_pages_csv,
    export_sitemap_xml,
};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

type Response = (u16, Vec<(String, String)>, String);
struct Fixture {
    origin: String,
    hits: Arc<Mutex<HashMap<String, usize>>>,
    requests: Arc<Mutex<HashMap<String, String>>>,
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
        let hits = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
        let task_hits = hits.clone();
        let requests = Arc::new(Mutex::new(HashMap::<String, String>::new()));
        let task_requests = requests.clone();
        let handler = Arc::new(handler);
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let handler = handler.clone();
                let origin = task_origin.clone();
                let hits = task_hits.clone();
                let requests = task_requests.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buffer = [0u8; 2048];
                    loop {
                        let Ok(n) = stream.read(&mut buffer).await else {
                            return;
                        };
                        if n == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..n]);
                        if let Some(header_end) =
                            request.windows(4).position(|window| window == b"\r\n\r\n")
                        {
                            let headers = String::from_utf8_lossy(&request[..header_end]);
                            let length = headers
                                .lines()
                                .find_map(|line| {
                                    line.split_once(':')
                                        .filter(|(key, _)| {
                                            key.eq_ignore_ascii_case("content-length")
                                        })
                                        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                                })
                                .unwrap_or(0);
                            if request.len() >= header_end + 4 + length {
                                break;
                            }
                        }
                        if request.len() > 32_768 {
                            return;
                        }
                    }
                    let request = String::from_utf8_lossy(&request);
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/");
                    *hits.lock().unwrap().entry(path.to_string()).or_insert(0) += 1;
                    requests
                        .lock()
                        .unwrap()
                        .insert(path.to_string(), request.to_string());
                    let (status, headers, body) = handler(&origin, path);
                    let mut response = format!(
                        "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n",
                        body.len()
                    );
                    for (key, value) in headers {
                        response.push_str(&format!("{key}: {value}\r\n"));
                    }
                    response.push_str("\r\n");
                    response.push_str(&body);
                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
        });
        Self {
            origin,
            hits,
            requests,
            task,
        }
    }
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin)
    }
    fn count(&self, path: &str) -> usize {
        *self.hits.lock().unwrap().get(path).unwrap_or(&0)
    }
    fn request(&self, path: &str) -> String {
        self.requests
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .unwrap_or_default()
    }
}

fn html(body: impl Into<String>) -> Response {
    (
        200,
        vec![("Content-Type".into(), "text/html; charset=utf-8".into())],
        body.into(),
    )
}
fn document(origin: &str, path: &str) -> String {
    format!(
        "<html lang='en'><head><title>{path}</title><meta name='description' content='Description {path}'><meta name='viewport' content='width=device-width'><link rel='canonical' href='{origin}{path}'></head><body><h1>Page {path}</h1></body></html>"
    )
}
fn config(seed: String) -> CrawlConfig {
    CrawlConfig {
        seed_urls: vec![seed],
        delay_ms: 0,
        concurrency: 16,
        discover_sitemaps: false,
        ..Default::default()
    }
}
fn cancel() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

#[tokio::test]
async fn unlimited_crawl_fetches_more_than_500_urls_and_deduplicates_fragments() {
    let fixture = Fixture::new(|origin,path| match path {
        "/robots.txt" => (404,vec![],String::new()),
        "/" => html(format!("<html><head><title>Root</title></head><body>{}<a href='/page/0#fragment'>duplicate</a><a href='https://outside.invalid/'>external</a></body></html>",(0..600).map(|n| format!("<a href='/page/{n}'>Page {n}</a>")).collect::<String>())),
        _ => html(document(origin,path)),
    }).await;
    let report = Crawler::new(config(fixture.url("/")))
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert_eq!(report.pages.len(), 601);
    assert_eq!(report.summary.completed, 601);
    assert!(!report.summary.limited);
    assert_eq!(fixture.count("/page/0"), 1);
    assert_eq!(fixture.count("/robots.txt"), 1);
    assert!(report.pages.iter().all(|page| page.internal));
    assert!(export_pages_csv(&report).unwrap().contains("/page/599"));
    assert!(
        export_links_csv(&report)
            .unwrap()
            .contains("outside.invalid")
    );
    assert!(export_sitemap_xml(&report).contains("/page/599"));
}

#[tokio::test]
async fn robots_sitemaps_redirects_and_broken_links_are_reported() {
    let fixture = Fixture::new(|origin,path| match path {
        "/robots.txt" => (200,vec![],format!("User-agent: *\nDisallow: /private\nSitemap: {origin}/index.xml\n")),
        "/index.xml" => (200,vec![("Content-Type".into(),"application/xml".into())],format!("<sitemapindex><sitemap><loc>{origin}/child.xml</loc></sitemap></sitemapindex>")),
        "/child.xml" => (200,vec![("Content-Type".into(),"application/xml".into())],format!("<urlset><url><loc>{origin}/only-sitemap?a=1&amp;b=2</loc></url></urlset>")),
        "/" => html("<html><head><title>Root</title></head><body><a href='/redirect'>redirect</a><a href='/final'>final</a><a href='/duplicate'>duplicate</a><a href='/loop'>loop</a><a href='/private'>private</a><a href='/bad'>bad</a><a href='/canonical'>canonical</a><img src='/missing-image.png'></body></html>"),
        "/redirect" => (302,vec![("Location".into(),"/final".into())],String::new()),
        "/loop" => (301,vec![("Location".into(),"/loop2".into())],String::new()),
        "/loop2" => (302,vec![("Location".into(),"/loop".into())],String::new()),
        "/bad" | "/missing-image.png" => (404,vec![],"Missing".into()),
        "/duplicate" => html(document(origin,"/final").replace(&format!("{origin}/final"),&format!("{origin}/duplicate"))),
        "/canonical" => html(document(origin,path).replace(&format!("{origin}/canonical"),&format!("{origin}/bad"))),
        _ => html(document(origin,path)),
    }).await;
    let mut options = config(fixture.url("/"));
    options.discover_sitemaps = true;
    let report = Crawler::new(options)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert_eq!(fixture.count("/private"), 0);
    assert!(
        report
            .pages
            .iter()
            .any(|page| page.url.ends_with("/private") && page.blocked_by_robots)
    );
    assert!(
        report
            .pages
            .iter()
            .any(|page| page.url.ends_with("/only-sitemap?a=1&b=2"))
    );
    let redirected = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/redirect"))
        .unwrap();
    assert_eq!(redirected.status, Some(302));
    assert_eq!(redirected.final_status, Some(200));
    assert_eq!(redirected.redirect_chain.len(), 1);
    assert!(
        report
            .pages
            .iter()
            .find(|page| page.url.ends_with("/loop"))
            .unwrap()
            .error
            .as_ref()
            .unwrap()
            .contains("loop")
    );
    for code in [
        "robots_blocked",
        "broken_outlinks",
        "canonical_nonindexable",
        "title_duplicate",
        "image_alt_missing",
    ] {
        assert!(
            report.issues.iter().any(|issue| issue.code == code),
            "missing {code}"
        );
    }
}

#[tokio::test]
async fn sqlite_resume_retains_frontier_and_does_not_refetch_completed_pages() {
    let fixture = Fixture::new(|origin, path| match path {
        "/robots.txt" => (404, vec![], String::new()),
        "/" => html(format!(
            "<html><title>Root</title><body>{}</body></html>",
            (0..12)
                .map(|n| format!("<a href='/p/{n}'>p{n}</a>"))
                .collect::<String>()
        )),
        _ => html(document(origin, path)),
    })
    .await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("crawl.sqlite");
    let mut options = config(fixture.url("/"));
    options.max_pages = Some(4);
    let first = Crawler::new(options.clone())
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert_eq!(first.pages.len(), 4);
    assert_eq!(CrawlStore::open(&path).unwrap().pending_count().unwrap(), 9);
    assert_eq!(
        CrawlStore::open(&path)
            .unwrap()
            .config()
            .unwrap()
            .unwrap()
            .max_pages,
        Some(4)
    );
    assert!(first.summary.limited);
    assert_eq!(first.summary.pending, 9);
    options.max_pages = None;
    let second = Crawler::new(options)
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert_eq!(second.pages.len(), 13);
    assert_eq!(CrawlStore::open(&path).unwrap().pending_count().unwrap(), 0);
    assert_eq!(
        CrawlStore::open(&path)
            .unwrap()
            .config()
            .unwrap()
            .unwrap()
            .max_pages,
        None
    );
    assert_eq!(fixture.count("/"), 1);
    assert_eq!(second.summary.pending, 0);
    assert_eq!(CrawlStore::open(&path).unwrap().pages().unwrap().len(), 13);
    assert_eq!(
        CrawlStore::open(&path)
            .unwrap()
            .load_report()
            .unwrap()
            .unwrap()
            .summary
            .completed,
        13
    );
    assert_eq!(
        compare_crawls(&first, &second)
            .iter()
            .filter(|change| change.kind == "added")
            .count(),
        9
    );
}

#[tokio::test]
async fn list_mode_stays_exact_and_noindex_and_body_limits_are_visible() {
    let fixture = Fixture::new(|origin,path| match path {
        "/robots.txt" => (404,vec![],String::new()),
        "/one" => html(format!("<html><title>One</title><meta name='robots' content='noindex'><body><a href='{origin}/not-listed'>do not crawl</a><h1>One</h1></body></html>")),
        _ => html(format!("{}{}",document(origin,path),"x".repeat(2000))),
    }).await;
    let options = CrawlConfig {
        seed_urls: vec![fixture.url("/one"), fixture.url("/two")],
        crawl_links: false,
        delay_ms: 0,
        max_body_bytes: 512,
        ..Default::default()
    };
    let report = Crawler::new(options)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert_eq!(report.pages.len(), 2);
    assert_eq!(fixture.count("/not-listed"), 0);
    for page in &report.pages {
        assert!(page.browser_version.is_none());
        assert!(page.rendered_viewport.is_none());
        assert!(page.render_wait_ms.is_none());
        assert_eq!(page.network_events_dropped, 0);
    }
    assert!(
        !report
            .pages
            .iter()
            .find(|page| page.url.ends_with("/one"))
            .unwrap()
            .indexable
    );
    assert!(
        report
            .pages
            .iter()
            .find(|page| page.url.ends_with("/two"))
            .unwrap()
            .truncated
    );
}

#[tokio::test]
async fn cancellation_returns_a_resumable_report() {
    let fixture = Fixture::new(|origin, path| match path {
        "/robots.txt" => (404, vec![], String::new()),
        "/" => html(format!(
            "<html><body>{}</body></html>",
            (0..100)
                .map(|n| format!("<a href='/p/{n}'>p</a>"))
                .collect::<String>()
        )),
        _ => html(document(origin, path)),
    })
    .await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("cancel.sqlite");
    let mut options = config(fixture.url("/"));
    options.concurrency = 1;
    options.delay_ms = 5;
    let flag = cancel();
    let task_flag = flag.clone();
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let listener = tokio::spawn(async move {
        while let Some(event) = receiver.recv().await {
            if matches!(event, CrawlEvent::Page { .. }) {
                task_flag.store(true, Ordering::Relaxed);
                break;
            }
        }
    });
    let report = Crawler::new(options.clone())
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run(Some(sender), flag)
        .await
        .unwrap();
    listener.await.unwrap();
    assert!(report.summary.cancelled);
    assert!(report.summary.pending > 0);
    options.delay_ms = 0;
    let resumed = Crawler::new(options)
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert_eq!(resumed.pages.len(), 101);
    assert_eq!(fixture.count("/"), 1);
}

#[test]
fn invalid_headers_and_configuration_return_errors() {
    let mut options = config("https://example.org/".into());
    options
        .custom_headers
        .insert("bad header".into(), "value".into());
    assert!(Crawler::new(options).is_err());
    let mut options = config("https://example.org/".into());
    options.concurrency = 0;
    assert!(Crawler::new(options).is_err());
}

#[tokio::test]
async fn multiple_explicit_sitemaps_define_all_origins_in_sitemap_only_mode() {
    let handler = |origin: &str, path: &str| match path {
        "/robots.txt" => (404, vec![], String::new()),
        "/sitemap.xml" => (
            200,
            vec![("Content-Type".into(), "application/xml".into())],
            format!("<urlset><url><loc>{origin}/only</loc></url></urlset>"),
        ),
        _ => html(document(origin, path)),
    };
    let first = Fixture::new(handler).await;
    let second = Fixture::new(handler).await;
    let options = CrawlConfig {
        sitemap_urls: vec![first.url("/sitemap.xml"), second.url("/sitemap.xml")],
        crawl_links: false,
        delay_ms: 0,
        ..Default::default()
    };
    let report = Crawler::new(options)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert_eq!(report.pages.len(), 2);
    assert!(report.pages.iter().all(|page| page.internal));
    assert_eq!(first.count("/only"), 1);
    assert_eq!(second.count("/only"), 1);
}

#[tokio::test]
async fn robots_outage_does_not_become_false_permission_to_crawl() {
    let fixture = Fixture::new(|origin, path| match path {
        "/robots.txt" => (503, vec![], "Unavailable".into()),
        _ => html(document(origin, path)),
    })
    .await;
    let report = Crawler::new(config(fixture.url("/")))
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert_eq!(fixture.count("/"), 0);
    assert!(report.pages[0].blocked_by_robots);
    assert!(
        report.pages[0]
            .robots_warning
            .as_deref()
            .unwrap()
            .contains("503")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "robots_unavailable")
    );
}

#[tokio::test]
async fn credentials_are_not_forwarded_to_external_requests_or_robots() {
    let external = Fixture::new(|origin, path| match path {
        "/robots.txt" => (404, vec![], String::new()),
        _ => html(document(origin, path)),
    })
    .await;
    let external_url = external.url("/outside");
    let fixture=Fixture::new(move |origin,path| match path {
        "/robots.txt" => (404,vec![],String::new()),
        "/" => html(format!("<html><body><a href='{external_url}'>External</a><a href='/redirect'>Redirect</a></body></html>")),
        "/redirect" => (302,vec![("Location".into(),external_url.clone())],String::new()),
        _ => html(document(origin,path)),
    }).await;
    let mut options = config(fixture.url("/"));
    options.check_external = true;
    options
        .custom_headers
        .insert("Authorization".into(), "Bearer fixture-only-token".into());
    options
        .custom_headers
        .insert("X-Audit-Key".into(), "fixture-only-key".into());
    let report = Crawler::new(options)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert!(report.pages.iter().any(|page| !page.internal));
    assert!(fixture.request("/").contains("fixture-only-token"));
    assert!(fixture.request("/redirect").contains("fixture-only-key"));
    assert!(!fixture.request("/robots.txt").contains("fixture-only"));
    assert_eq!(external.count("/outside"), 2);
    assert!(!external.request("/outside").contains("fixture-only"));
}

#[tokio::test]
async fn renderer_replaces_dom_metadata_and_retains_native_evidence() {
    let native = Fixture::new(|_, path| match path {
        "/robots.txt" => (404, vec![], String::new()),
        _ => html(
            "<html><head><title>Raw title</title></head><body><h1>Raw heading</h1></body></html>",
        ),
    })
    .await;
    let final_url = native.url("/");
    let rendered=serde_json::json!({"html":"<html lang='en'><head><title>Rendered title</title></head><body><h1>Rendered heading</h1><a href='/generated'>JS link</a></body></html>","final_url":final_url,"status":200,"headers":{"x-renderer":"fixture"},"console_errors":[{"type":"error","message":"fixture console error"}],"network_requests":[{"url":native.url("/failed.js"),"status":404,"error":null}],"rendered_at":"2026-10-11T12:00:00Z","accessibility":{"violations":[{"id":"color-contrast","impact":"serious","help":"Insufficient contrast","nodes":[{}]}],"automated_only":true},"screenshot_base64":"Zml4dHVyZQ==","browser_version":"fixture-chrome-version","viewport":{"width":1280,"height":720},"render_wait_ms":500,"network_events_dropped":3}).to_string();
    let worker = Fixture::new(move |_, _| {
        (
            200,
            vec![("Content-Type".into(), "application/json".into())],
            rendered.clone(),
        )
    })
    .await;
    let mut options = config(native.url("/"));
    options.crawl_links = false;
    options
        .custom_headers
        .insert("X-Audit-Key".into(), "render-fixture-key".into());
    options.renderer = Some(RendererConfig {
        endpoint: worker.url("/render"),
        accessibility: true,
        screenshot: true,
        ..Default::default()
    });
    options.custom_extractions = vec![CustomExtractionConfig {
        name: "Headings".into(),
        selector: "h1".into(),
        attribute: None,
    }];
    let report = Crawler::new(options)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    let page = &report.pages[0];
    assert!(page.rendered);
    assert_eq!(page.title(), "Rendered title");
    assert_eq!(page.h1, vec!["Rendered heading"]);
    assert_eq!(
        page.raw_metadata.as_ref().unwrap().custom_extractions[0].values,
        vec!["Raw heading"]
    );
    assert_eq!(page.custom_extractions[0].values, vec!["Rendered heading"]);
    assert_eq!(page.custom_extractions[0].source, "rendered_html");
    assert_eq!(
        page.raw_metadata.as_ref().unwrap().titles,
        vec!["Raw title"]
    );
    assert_eq!(page.status, Some(200));
    assert_eq!(page.final_status, Some(200));
    assert_eq!(page.rendered_headers.get("x-renderer").unwrap(), "fixture");
    assert!(!page.headers.contains_key("x-renderer"));
    assert_eq!(page.links[0].url, native.url("/generated"));
    assert!(page.screenshot_base64.is_some());
    assert_eq!(
        page.browser_version.as_deref(),
        Some("fixture-chrome-version")
    );
    assert_eq!(
        page.rendered_viewport,
        Some(serde_json::json!({"width":1280,"height":720}))
    );
    assert_eq!(page.render_wait_ms, Some(500));
    assert_eq!(page.network_events_dropped, 3);
    assert!(
        page.warnings
            .iter()
            .any(|message| message.contains("network capture is incomplete"))
    );
    assert!(worker.request("/render").contains("render-fixture-key"));
    for code in [
        "browser_console_errors",
        "browser_resource_failed",
        "browser_network_capture_incomplete",
        "accessibility_color-contrast",
    ] {
        assert!(
            report.issues.iter().any(|issue| issue.code == code),
            "missing {code}"
        );
    }
}

#[tokio::test]
async fn renderer_failure_is_visible_and_never_claims_complete_dom_analysis() {
    let native = Fixture::new(|_, path| match path {
        "/robots.txt" => (404, vec![], String::new()),
        _ => html("<html><head><title>Raw title</title></head><body>App shell</body></html>"),
    })
    .await;
    let worker = Fixture::new(|_, _| {
        (
            503,
            vec![("Content-Type".into(), "application/json".into())],
            "{\"error\":{\"code\":\"unavailable\",\"message\":\"Browser unavailable\"}}".into(),
        )
    })
    .await;
    let mut options = config(native.url("/"));
    options.crawl_links = false;
    options.renderer = Some(RendererConfig {
        endpoint: worker.url("/render"),
        ..Default::default()
    });
    let report = Crawler::new(options)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    let page = &report.pages[0];
    assert!(!page.rendered);
    assert!(!page.indexable);
    assert_eq!(page.title(), "Raw title");
    assert!(
        page.renderer_error
            .as_deref()
            .unwrap()
            .contains("Browser unavailable")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "renderer_failed")
    );
    assert!(!report.issues.iter().any(|issue| issue.code == "h1_missing"));
}

#[tokio::test]
async fn disallowed_pages_do_not_trigger_browser_rendering() {
    let native = Fixture::new(|_, path| match path {
        "/robots.txt" => (200, vec![], "User-agent: *\nDisallow: /\n".into()),
        _ => html("<html><body>Forbidden</body></html>"),
    })
    .await;
    let worker = Fixture::new(|_, _| (500, vec![], String::new())).await;
    let mut options = config(native.url("/"));
    options.crawl_links = false;
    options.renderer = Some(RendererConfig {
        endpoint: worker.url("/render"),
        ..Default::default()
    });
    let report = Crawler::new(options)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert!(report.pages[0].blocked_by_robots);
    assert_eq!(native.count("/"), 0);
    assert_eq!(worker.count("/render"), 0);
}

#[test]
fn remote_renderer_endpoints_are_rejected() {
    for endpoint in [
        "http://example.org/render",
        "https://127.0.0.1:8790/render",
        "http://user:password@127.0.0.1:8790/render",
    ] {
        let mut options = config("https://example.org/".into());
        options.renderer = Some(RendererConfig {
            endpoint: endpoint.into(),
            ..Default::default()
        });
        assert!(Crawler::new(options).is_err());
    }
}

#[test]
fn renderer_wait_and_timeout_limits_match_the_worker_contract() {
    for (timeout, wait, valid) in [
        (30, 30_000, false),
        (31, 30_000, true),
        (31, 30_001, false),
        (1, 1000, false),
        (1, 999, true),
        (121, 500, false),
    ] {
        let mut options = config("https://example.org/".into());
        options.request_timeout_secs = timeout;
        options.renderer = Some(RendererConfig {
            wait_ms: wait,
            ..Default::default()
        });
        assert_eq!(
            Crawler::new(options).is_ok(),
            valid,
            "timeout={timeout}, wait={wait}"
        );
    }
}

/// Explicit integration gate against the separately started real Chrome worker.
#[tokio::test]
#[ignore = "Requires the local browser worker at OPEN_SEO_SPIDER_RENDERER or 127.0.0.1:8790"]
async fn live_browser_worker_executes_javascript_for_core_crawl() {
    let native=Fixture::new(|_,path|match path {
        "/robots.txt"=>(404,vec![],String::new()),
        _=>html("<!doctype html><html lang='en'><head><title>Raw title</title><meta name='viewport' content='width=device-width'></head><body><h1>Raw heading</h1><script>document.title='Executed title';document.querySelector('h1').textContent='JavaScript heading';const link=document.createElement('a');link.href='/javascript-discovered';link.textContent='Discovered by JavaScript';document.body.append(link);</script></body></html>"),
    }).await;
    let mut options = config(native.url("/"));
    options.crawl_links = false;
    options.renderer = Some(RendererConfig {
        endpoint: std::env::var("OPEN_SEO_SPIDER_RENDERER")
            .unwrap_or_else(|_| "http://127.0.0.1:8790/render".into()),
        wait_ms: 100,
        accessibility: true,
        screenshot: true,
        ..Default::default()
    });
    let report = Crawler::new(options)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    let page = &report.pages[0];
    assert!(page.rendered, "{:?}", page.renderer_error);
    assert_eq!(page.title(), "Executed title");
    assert_eq!(page.h1, vec!["JavaScript heading"]);
    assert_eq!(
        page.raw_metadata.as_ref().unwrap().titles,
        vec!["Raw title"]
    );
    assert_eq!(page.raw_metadata.as_ref().unwrap().h1, vec!["Raw heading"]);
    assert!(
        page.links
            .iter()
            .any(|link| link.url == native.url("/javascript-discovered"))
    );
    assert!(page.accessibility.is_some());
    assert!(
        page.screenshot_base64
            .as_ref()
            .is_some_and(|image| image.len() > 100)
    );
    assert!(!page.network_requests.is_empty());
    assert!(
        page.browser_version
            .as_ref()
            .is_some_and(|version| !version.is_empty())
    );
    assert!(page.rendered_viewport.is_some());
    assert_eq!(page.render_wait_ms, Some(100));
    assert_eq!(page.network_events_dropped, 0);
}

#[tokio::test]
async fn configured_custom_extractions_preserve_all_matches_and_export_exact_values() {
    let fixture=Fixture::new(|_,path|match path {
        "/robots.txt"=>(404,vec![],String::new()),
        _=>html("<html><title>Products</title><body><a data-sku='a,&quot;first&quot;'>First <b>link</b></a><a data-sku='second'>Second link</a><a>No attribute</a></body></html>"),
    }).await;
    let mut options = config(fixture.url("/"));
    options.crawl_links = false;
    options.custom_extractions = vec![
        CustomExtractionConfig {
            name: "Labels".into(),
            selector: "a".into(),
            attribute: None,
        },
        CustomExtractionConfig {
            name: "SKUs".into(),
            selector: "a".into(),
            attribute: Some("data-sku".into()),
        },
        CustomExtractionConfig {
            name: "Absent".into(),
            selector: ".absent".into(),
            attribute: None,
        },
    ];
    let report = Crawler::new(options)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    let extraction = &report.pages[0].custom_extractions;
    assert_eq!(extraction.len(), 3);
    assert_eq!(
        extraction[0].values,
        vec!["First link", "Second link", "No attribute"]
    );
    assert_eq!(extraction[1].values, vec!["a,\"first\"", "second"]);
    assert!(extraction[2].values.is_empty());
    assert!(extraction.iter().all(|item| item.source == "raw_html"));
    let csv = export_custom_extractions_csv(&report).unwrap();
    let rows = csv::Reader::from_reader(csv.as_bytes())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 6);
    assert!(
        rows.iter()
            .any(|row| row.get(1) == Some("SKUs") && row.get(7) == Some("a,\"first\""))
    );
    assert!(
        rows.iter()
            .any(|row| row.get(1) == Some("Absent") && row.get(5) == Some("0"))
    );
}

#[tokio::test]
async fn url_filters_apply_to_discovery_and_sitemaps_and_preserve_explicit_seeds() {
    let fixture=Fixture::new(|origin,path|match path {
        "/robots.txt"=>(200,vec![],format!("User-agent: *\nSitemap: {origin}/sitemap.xml\n")),
        "/sitemap.xml"=>(200,vec![("Content-Type".into(),"application/xml".into())],format!("<urlset><url><loc>{origin}/keep/sitemap</loc></url><url><loc>{origin}/drop/sitemap</loc></url></urlset>")),
        "/start"=>html("<html><body><a href='/keep/one'>Keep</a><a href='/keep/skip'>Skip</a><a href='/outside'>Outside</a><a href='/explicit'>Seed</a></body></html>"),
        _=>html(document(origin,path)),
    }).await;
    let options = CrawlConfig {
        seed_urls: vec![fixture.url("/start"), fixture.url("/explicit")],
        include_patterns: vec!["/keep/".into()],
        exclude_patterns: vec!["/skip|/explicit|/start".into()],
        delay_ms: 0,
        ..Default::default()
    };
    let report = Crawler::new(options)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert_eq!(report.pages.len(), 4);
    for path in ["/start", "/explicit", "/keep/one", "/keep/sitemap"] {
        assert_eq!(fixture.count(path), 1);
    }
    for path in ["/keep/skip", "/outside", "/drop/sitemap"] {
        assert_eq!(fixture.count(path), 0);
    }
    assert_eq!(fixture.count("/sitemap.xml"), 1);
    assert!(
        report
            .pages
            .iter()
            .find(|page| page.url.ends_with("/start"))
            .unwrap()
            .links
            .iter()
            .any(|link| link.url.ends_with("/keep/skip"))
    );
}

#[test]
fn extraction_and_regex_configuration_fail_before_network_requests() {
    for extraction in [
        CustomExtractionConfig {
            name: "".into(),
            selector: "a".into(),
            attribute: None,
        },
        CustomExtractionConfig {
            name: "Invalid selector".into(),
            selector: "[".into(),
            attribute: None,
        },
        CustomExtractionConfig {
            name: "Invalid attribute".into(),
            selector: "a".into(),
            attribute: Some("".into()),
        },
    ] {
        let mut options = config("https://example.org/".into());
        options.custom_extractions = vec![extraction];
        assert!(Crawler::new(options).is_err());
    }
    let mut options = config("https://example.org/".into());
    options.custom_extractions = vec![
        CustomExtractionConfig {
            name: "Name".into(),
            selector: "a".into(),
            attribute: None,
        },
        CustomExtractionConfig {
            name: "Name".into(),
            selector: "p".into(),
            attribute: None,
        },
    ];
    assert!(Crawler::new(options).is_err());
    for pattern in ["[", "(?<=x)y"] {
        let mut options = config("https://example.org/".into());
        options.exclude_patterns = vec![pattern.into()];
        assert!(Crawler::new(options).is_err());
    }
}
