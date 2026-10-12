//! Own local HTTP/browser observations; no external provider or vendor source.
use open_seo_spider_core::*;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{Semaphore, mpsc},
};

#[tokio::test]
async fn readonly_saved_raw_rendered_and_selected_capture_recovery_fits_a_bounded_blocking_thread()
{
    let fixture = Fixture::new(|request| Response::html(format!(
        "<html lang='de'><head><title>Saved {}</title></head><body><main>Native captured {}</main></body></html>",
        request.path, request.path,
    ))).await;
    let worker = Fixture::new(|request| {
        rendered_response(
            request,
            "<html lang='en'><body><main>Independently rendered capture</main></body></html>",
            Some(200),
            Some("text/html;charset=utf-8"),
            Some("2026-10-12T18:00:00Z"),
        )
    })
    .await;
    let temp = tempfile::tempdir().unwrap();
    let raw_path = temp.path().join("raw.sqlite");
    let cfg = config(&fixture, &["/a", "/b", "/c", "/d", "/e", "/f"]);
    Crawler::new(cfg.clone())
        .unwrap()
        .with_store(&raw_path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    let mut rendered_cfg = cfg;
    both(&mut rendered_cfg, &worker);
    let rendered_path = temp.path().join("rendered.sqlite");
    let parent_report = Crawler::new(rendered_cfg)
        .unwrap()
        .with_store(&rendered_path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap()
        .report;
    let parent_bytes = serde_json::to_vec(&parent_report).unwrap();
    let prepared =
        prepare_selected_recrawl(parent(), &parent_bytes, &[fixture.url("/a")], None).unwrap();
    let selected_path = temp.path().join("selected.sqlite");
    Crawler::new(prepared.config.clone())
        .unwrap()
        .with_store(&selected_path)
        .unwrap()
        .with_selected_recrawl(prepared)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    // Recovery cannot accidentally depend on any still-running fixture service.
    drop(fixture);
    drop(worker);
    std::thread::Builder::new()
        .name("bounded-recovery".into())
        .stack_size(512 * 1024)
        .spawn(move || {
            for path in [raw_path, rendered_path, selected_path] {
                let inspection = CrawlStore::inspect(&path).unwrap();
                assert!(inspection.checkpoint_valid);
                assert_eq!(inspection.completed, 6);
                let store = CrawlStore::open_read_only(&path).unwrap();
                let report = store.load_report().unwrap().unwrap();
                assert_eq!(report.pages.len(), 6);
                for page in &report.pages {
                    validate_page_content_captures(page).unwrap();
                }
                let _ = store.run_plan().unwrap();
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

struct Response {
    status: u16,
    body: Vec<u8>,
    mime: String,
    headers: Vec<(String, String)>,
    gate: Option<Arc<Semaphore>>,
}
impl Response {
    fn html(body: impl AsRef<[u8]>) -> Self {
        Self {
            status: 200,
            body: body.as_ref().to_vec(),
            mime: "text/html; charset=utf-8".into(),
            headers: vec![],
            gate: None,
        }
    }
    fn json(body: serde_json::Value) -> Self {
        let mut value = Self::html(serde_json::to_vec(&body).unwrap());
        value.mime = "application/json".into();
        value
    }
    fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }
    fn mime(mut self, mime: &str) -> Self {
        self.mime = mime.into();
        self
    }
    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
    fn gated(mut self, gate: Arc<Semaphore>) -> Self {
        self.gate = Some(gate);
        self
    }
}
struct Request {
    path: String,
    body: Vec<u8>,
}
struct Fixture {
    origin: String,
    hits: Arc<Mutex<BTreeMap<String, usize>>>,
    started: mpsc::UnboundedReceiver<String>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Fixture {
    async fn new(handler: impl Fn(Request) -> Response + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
        let shared = hits.clone();
        let handler = Arc::new(handler);
        let (sender, started) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let handler = handler.clone();
                let hits = shared.clone();
                let sender = sender.clone();
                tokio::spawn(async move {
                    let mut bytes = vec![];
                    let mut buf = [0u8; 2048];
                    let header_end = loop {
                        let Ok(n) = socket.read(&mut buf).await else {
                            return;
                        };
                        if n == 0 {
                            return;
                        };
                        bytes.extend_from_slice(&buf[..n]);
                        if let Some(index) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                            break index + 4;
                        }
                        if bytes.len() > 65536 {
                            return;
                        }
                    };
                    let header = String::from_utf8_lossy(&bytes[..header_end]);
                    let path = header
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/")
                        .to_owned();
                    let length = header
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                                .and_then(|(_, v)| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if length > 1024 * 1024 {
                        return;
                    }
                    while bytes.len() - header_end < length {
                        let Ok(n) = socket.read(&mut buf).await else {
                            return;
                        };
                        if n == 0 {
                            return;
                        };
                        bytes.extend_from_slice(&buf[..n]);
                    }
                    *hits.lock().unwrap().entry(path.clone()).or_default() += 1;
                    let response = handler(Request {
                        path: path.clone(),
                        body: bytes[header_end..header_end + length].to_vec(),
                    });
                    let _ = sender.send(path);
                    if let Some(gate) = response.gate {
                        gate.acquire().await.unwrap().forget();
                    }
                    let additional = response
                        .headers
                        .iter()
                        .map(|(k, v)| format!("{k}: {v}\r\n"))
                        .collect::<String>();
                    let head = format!(
                        "HTTP/1.1 {} Fixture\r\nContent-Type: {}\r\nContent-Length: {}\r\n{additional}Connection: close\r\n\r\n",
                        response.status,
                        response.mime,
                        response.body.len()
                    );
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(&response.body).await;
                });
            }
        });
        Self {
            origin,
            hits,
            started,
            task,
        }
    }
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin)
    }
    fn count(&self, path: &str) -> usize {
        self.hits.lock().unwrap().get(path).copied().unwrap_or(0)
    }
    async fn wait(&mut self, path: &str) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(value) = self.started.recv().await {
                if value == path {
                    return;
                }
            }
            panic!("fixture closed")
        })
        .await
        .unwrap();
    }
}
fn config(fixture: &Fixture, paths: &[&str]) -> CrawlConfig {
    CrawlConfig {
        seed_urls: paths.iter().map(|path| fixture.url(path)).collect(),
        obey_robots: false,
        discover_sitemaps: false,
        crawl_links: false,
        delay_ms: 0,
        concurrency: 1,
        request_timeout_secs: 5,
        content_capture: Some(ContentCaptureConfig::default()),
        ..Default::default()
    }
}
fn sha(body: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(body.as_ref()))
}
fn row(page: &PageRecord, kind: ContentCaptureSourceKind) -> &PageContentCapture {
    page.content_captures
        .as_ref()
        .unwrap()
        .iter()
        .find(|row| row.source_kind == kind)
        .unwrap()
}
fn raw(page: &PageRecord) -> &PageContentCapture {
    row(page, ContentCaptureSourceKind::RawHttpHtml)
}
async fn run(config: CrawlConfig) -> ControlledCrawlResult {
    Crawler::new(config)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap()
}

#[tokio::test]
async fn exact_entity_decode_html_text_language_and_requested_final_addresses_are_bound() {
    let body=b"<html lang='de'><body><nav>Excluded</nav><main>Gr\xfc\xdfe &amp; Tests</main></body></html>";
    let fixture = Fixture::new(move |request| {
        if request.path == "/start" {
            Response::html("")
                .status(302)
                .header("Location", "/document")
        } else {
            Response::html(body).mime("Text/HTML; charset=windows-1252")
        }
    })
    .await;
    let result = run(config(&fixture, &["/start"])).await;
    let page = &result.report.pages[0];
    let capture = raw(page);
    let selected = capture.result.as_ref().unwrap();
    assert_eq!(capture.status, PageContentCaptureStatus::Complete);
    assert_eq!(capture.requested_url, fixture.url("/start"));
    assert_eq!(capture.source_document_url, Some(fixture.url("/document")));
    assert_eq!(capture.encoded_body_sha256, Some(sha(body)));
    assert_eq!(capture.encoded_body_bytes, Some(body.len()));
    assert_eq!(capture.encoded_body_complete, Some(true));
    assert_eq!(capture.decode_encoding.as_deref(), Some("windows-1252"));
    let (html, _, _) = encoding_rs::WINDOWS_1252.decode(body);
    assert_eq!(capture.source_html_utf8_sha256, Some(sha(html.as_bytes())));
    assert_ne!(capture.source_html_utf8_sha256, capture.encoded_body_sha256);
    assert_eq!(selected.text.as_deref(), Some("Grüße & Tests"));
    assert_eq!(selected.html_language.as_deref(), Some("de"));
    assert_eq!(capture.observed_generation, None);
    validate_page_content_captures(page).unwrap();
}

#[tokio::test]
async fn literal_replacement_character_is_exact_but_invalid_bytes_and_unknown_charset_are_incomplete()
 {
    let fixture = Fixture::new(|request| match request.path.as_str() {
        "/invalid" => Response::html(b"<body>bad \xff</body>"),
        "/unknown" => Response::html("<body>Unknown decode</body>")
            .mime("text/html; charset=unsupported-own-fixture"),
        _ => Response::html("<body>Literal �</body>"),
    })
    .await;
    let result = run(config(&fixture, &["/literal", "/invalid", "/unknown"])).await;
    for (path, decode, status) in [
        (
            "/literal",
            ContentCaptureDecodeStatus::Exact,
            PageContentCaptureStatus::Complete,
        ),
        (
            "/invalid",
            ContentCaptureDecodeStatus::Lossy,
            PageContentCaptureStatus::Incomplete,
        ),
        (
            "/unknown",
            ContentCaptureDecodeStatus::Unknown,
            PageContentCaptureStatus::Incomplete,
        ),
    ] {
        let page = result
            .report
            .pages
            .iter()
            .find(|page| page.url == fixture.url(path))
            .unwrap();
        let value = raw(page);
        assert_eq!(value.status, status);
        assert_eq!(value.result.as_ref().unwrap().decode_status, decode);
        validate_page_content_captures(page).unwrap();
    }
}

#[tokio::test]
async fn empty_success_is_distinct_from_non_html_and_non200_partial_html() {
    let fixture = Fixture::new(|request| match request.path.as_str() {
        "/plain" => Response::html("Not HTML").mime("text/plain"),
        "/misleading" => {
            Response::html("<body>Not an HTML media type</body>").mime("x-text/html-extra")
        }
        "/missing" => Response::html("<body>Missing document</body>").status(404),
        _ => Response::html(""),
    })
    .await;
    let result = run(config(
        &fixture,
        &["/empty", "/plain", "/misleading", "/missing"],
    ))
    .await;
    for page in &result.report.pages {
        let value = raw(page);
        validate_page_content_captures(page).unwrap();
        if page.url.ends_with("/empty") {
            assert_eq!(value.status, PageContentCaptureStatus::Empty);
            assert_eq!(value.result.as_ref().unwrap().text.as_deref(), Some(""));
            assert_eq!(value.source_html_utf8_sha256, Some(sha(b"")));
        } else if page.url.ends_with("/missing") {
            assert_eq!(value.status, PageContentCaptureStatus::Incomplete);
            assert_eq!(
                value.result.as_ref().unwrap().text.as_deref(),
                Some("Missing document")
            );
        } else {
            assert_eq!(value.status, PageContentCaptureStatus::Unavailable);
            assert!(value.result.as_ref().unwrap().text.is_none());
            assert!(value.source_html_utf8_sha256.is_none());
            assert!(value.encoded_body_sha256.is_some());
        }
    }
}

#[tokio::test]
async fn robots_and_stopped_redirects_have_unavailable_sources_without_fake_html_hashes() {
    let fixture = Fixture::new(|request| match request.path.as_str() {
        "/robots.txt" => Response::html("User-agent: *\nDisallow: /blocked\n").mime("text/plain"),
        "/loop" => Response::html("<body>Redirect body not a document</body>")
            .status(302)
            .header("Location", "/loop"),
        _ => Response::html("Must never fetch"),
    })
    .await;
    let mut cfg = config(&fixture, &["/blocked", "/loop"]);
    cfg.obey_robots = true;
    let result = run(cfg).await;
    assert_eq!(fixture.count("/blocked"), 0);
    assert_eq!(fixture.count("/loop"), 1);
    for page in &result.report.pages {
        let value = raw(page);
        assert_eq!(value.status, PageContentCaptureStatus::Unavailable);
        assert!(value.source_html_utf8_sha256.is_none());
        assert!(value.encoded_body_sha256.is_none());
        validate_page_content_captures(page).unwrap();
    }
}

#[tokio::test]
async fn source_text_traversal_and_capture_resource_budgets_are_explicit_without_aborting_crawl() {
    let fixture = Fixture::new(|_| {
        Response::html(
            "<html lang='en'><body><main>Long αβγδεζηθ Text</main><p>Second</p></body></html>",
        )
    })
    .await;
    for case in 0..5 {
        let mut cfg = config(&fixture, &["/"]);
        let limits = &mut cfg.content_capture.as_mut().unwrap().limits;
        match case {
            0 => cfg.max_body_bytes = 50,
            1 => limits.max_text_utf8_bytes = 8,
            2 => limits.max_visited_dom_nodes = 4,
            3 => limits.max_html_utf8_bytes = 20,
            _ => limits.max_output_json_bytes = 100,
        }
        let result = run(cfg).await;
        assert_eq!(result.outcome, CrawlOutcome::Completed);
        let page = &result.report.pages[0];
        let value = raw(page);
        validate_page_content_captures(page).unwrap();
        if case >= 3 {
            assert_eq!(value.status, PageContentCaptureStatus::Error);
            assert!(value.result.is_none());
            assert!(value.failure.as_ref().unwrap().detail.len() <= 512);
            assert!(!page.warnings.is_empty());
            assert_eq!(
                value.failure.as_ref().unwrap().code,
                if case == 3 {
                    "html_input_limit"
                } else {
                    "output_json_limit"
                }
            );
        } else {
            assert_eq!(value.status, PageContentCaptureStatus::Incomplete);
            let selected = value.result.as_ref().unwrap();
            if case == 0 {
                assert!(!selected.source_complete);
                assert_eq!(value.encoded_body_complete, Some(false));
            }
            if case == 1 {
                assert!(selected.evidence.as_ref().unwrap().text_truncated);
                assert!(selected.text.as_ref().unwrap().len() <= 8);
            }
            if case == 2 {
                assert!(!selected.evidence.as_ref().unwrap().traversal_complete);
            }
        }
    }
}

#[tokio::test]
async fn invalid_capture_css_limits_and_live_policy_renderer_combination_fail_before_requests() {
    let fixture = Fixture::new(|_| Response::html("Unexpected request")).await;
    for case in 0..3 {
        let mut cfg = config(&fixture, &["/"]);
        match case {
            0 => cfg.content_capture.as_mut().unwrap().include_selectors = vec!["[[".into()],
            1 => {
                cfg.content_capture
                    .as_mut()
                    .unwrap()
                    .limits
                    .max_text_utf8_bytes = 0
            }
            _ => {
                cfg.crawl_policy = Some(CrawlPolicyConfig::default());
                cfg.renderer = Some(RendererConfig::default());
            }
        }
        assert!(Crawler::new(cfg).is_err(), "invalid case {case}");
    }
    assert_eq!(fixture.count("/"), 0);
}

fn rendered_response(
    request: Request,
    html: &str,
    status: Option<u16>,
    mime: Option<&str>,
    timestamp: Option<&str>,
) -> Response {
    let input: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    let mut headers = BTreeMap::new();
    if let Some(mime) = mime {
        headers.insert("content-type", mime);
    }
    let mut url = normalize_url(input["url"].as_str().unwrap(), None).unwrap();
    url.set_path("/rendered-document");
    Response::json(
        serde_json::json!({"html":html,"final_url":url,"status":status,"headers":headers,"rendered_at":timestamp,"title":"fixture","console_errors":[],"network_requests":[]}),
    )
}
fn both(cfg: &mut CrawlConfig, worker: &Fixture) {
    cfg.content_capture.as_mut().unwrap().sources = vec![
        ContentCaptureSourceKind::RawHttpHtml,
        ContentCaptureSourceKind::RenderedDomHtml,
    ];
    cfg.renderer = Some(RendererConfig {
        endpoint: worker.url("/render"),
        wait_ms: 0,
        ..Default::default()
    });
}

#[tokio::test]
async fn raw_and_rendered_captures_preserve_independent_html_language_hash_address_and_selection() {
    let native = "<html lang='de'><body><main>Nativer Text</main></body></html>";
    let rendered = "<html lang='en'><body><main>Rendered text</main></body></html>";
    let fixture = Fixture::new(move |_| Response::html(native)).await;
    let worker = Fixture::new(move |request| {
        rendered_response(
            request,
            rendered,
            Some(200),
            Some("Text/HTML; charset=utf-8"),
            Some("2026-10-12T10:00:00Z"),
        )
    })
    .await;
    let mut cfg = config(&fixture, &["/"]);
    both(&mut cfg, &worker);
    let result = run(cfg).await;
    let page = &result.report.pages[0];
    validate_page_content_captures(page).unwrap();
    let raw = raw(page);
    let rendered_row = row(page, ContentCaptureSourceKind::RenderedDomHtml);
    assert_eq!(raw.source_html_utf8_sha256, Some(sha(native)));
    assert_eq!(rendered_row.source_html_utf8_sha256, Some(sha(rendered)));
    assert_eq!(
        raw.result.as_ref().unwrap().html_language.as_deref(),
        Some("de")
    );
    assert_eq!(
        rendered_row
            .result
            .as_ref()
            .unwrap()
            .html_language
            .as_deref(),
        Some("en")
    );
    assert_eq!(raw.source_document_url, Some(fixture.url("/")));
    assert_eq!(
        rendered_row.source_document_url,
        Some(fixture.url("/rendered-document"))
    );
    assert_eq!(
        raw.result.as_ref().unwrap().text.as_deref(),
        Some("Nativer Text")
    );
    assert_eq!(
        rendered_row.result.as_ref().unwrap().text.as_deref(),
        Some("Rendered text")
    );
    assert!(rendered_row.encoded_body_sha256.is_none());
    assert!(rendered_row.decode_encoding.is_none());
    assert_eq!(worker.count("/render"), 1);
}

#[tokio::test]
async fn failed_unverified_non_html_and_missing_timestamp_renderers_never_fall_back_to_raw_text() {
    let fixture = Fixture::new(|_| {
        Response::html("<html lang='de'><body>Raw fallback forbidden</body></html>")
    })
    .await;
    for case in 0..5 {
        let worker = Fixture::new(move |request| match case {
            0 => Response::json(serde_json::json!({"error":{"message":"Own failed worker"}}))
                .status(502),
            1 => rendered_response(
                request,
                "<body>Unverified</body>",
                None,
                Some("text/html"),
                Some("now"),
            ),
            2 => rendered_response(
                request,
                "<body>Bad</body>",
                Some(404),
                Some("text/html"),
                Some("now"),
            ),
            3 => rendered_response(
                request,
                "<body>Unknown MIME</body>",
                Some(200),
                None,
                Some("now"),
            ),
            _ => rendered_response(
                request,
                "<body>Observed but timestamp unknown</body>",
                Some(200),
                Some("text/html"),
                None,
            ),
        })
        .await;
        let mut cfg = config(&fixture, &["/"]);
        both(&mut cfg, &worker);
        let result = run(cfg).await;
        let page = &result.report.pages[0];
        let rendered = row(page, ContentCaptureSourceKind::RenderedDomHtml);
        validate_page_content_captures(page).unwrap();
        assert_eq!(raw(page).status, PageContentCaptureStatus::Complete);
        if case == 4 {
            assert_eq!(rendered.status, PageContentCaptureStatus::Incomplete);
            assert_eq!(
                rendered.result.as_ref().unwrap().text.as_deref(),
                Some("Observed but timestamp unknown")
            );
        } else {
            assert_eq!(rendered.status, PageContentCaptureStatus::Unavailable);
            assert!(rendered.result.as_ref().unwrap().text.is_none());
            assert!(rendered.source_html_utf8_sha256.is_none());
        }
    }
    let mut cfg = config(&fixture, &["/"]);
    cfg.content_capture.as_mut().unwrap().sources = vec![ContentCaptureSourceKind::RenderedDomHtml];
    let result = run(cfg).await;
    assert_eq!(
        result.report.pages[0]
            .content_captures
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        row(
            &result.report.pages[0],
            ContentCaptureSourceKind::RenderedDomHtml
        )
        .status,
        PageContentCaptureStatus::Unavailable
    );
}

#[tokio::test]
async fn sqlite_only_pause_recovery_retains_observed_generation_and_preserves_prior_checkpoint() {
    let gate = Arc::new(Semaphore::new(0));
    let handler_gate = gate.clone();
    let mut fixture = Fixture::new(move |request| {
        let response = Response::html(format!(
            "<html lang='en'><body>{}</body></html>",
            request.path
        ));
        if request.path == "/one" {
            response.gated(handler_gate.clone())
        } else {
            response
        }
    })
    .await;
    let cfg = config(&fixture, &["/one", "/two"]);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("capture.sqlite");
    let crawler = Crawler::new(cfg.clone())
        .unwrap()
        .with_store(&path)
        .unwrap();
    let control = CrawlControl::new();
    let handle = control.clone();
    let task = tokio::spawn(async move { crawler.run_controlled(None, handle).await.unwrap() });
    fixture.wait("/one").await;
    assert!(control.request_pause());
    gate.add_permits(1);
    let paused = task.await.unwrap();
    assert_eq!(paused.outcome, CrawlOutcome::Paused);
    assert_eq!(fixture.count("/two"), 0);
    assert_eq!(raw(&paused.report.pages[0]).observed_generation, Some(1));
    let reader = CrawlStore::open_read_only(&path).unwrap();
    assert!(reader.inspection().unwrap().checkpoint_valid);
    let historical = serde_json::to_vec(&reader.load_report().unwrap()).unwrap();
    drop(reader);
    let resumed = Crawler::new(cfg)
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    assert_eq!(fixture.count("/one"), 1);
    assert_eq!(fixture.count("/two"), 1);
    for page in &resumed.report.pages {
        assert_eq!(
            raw(page).observed_generation,
            Some(if page.url.ends_with("/one") { 1 } else { 2 })
        );
        validate_page_content_captures(page).unwrap();
    }
    assert_ne!(
        historical,
        serde_json::to_vec(
            &CrawlStore::open_read_only(&path)
                .unwrap()
                .load_report()
                .unwrap()
        )
        .unwrap()
    );
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn tampered_capture_text_or_generation_is_rejected_before_resume_state_and_http() {
    let fixture = Fixture::new(|_| Response::html("<body>Actual source text</body>")).await;
    for mutation in [
        "UPDATE pages SET data=json_set(data,'$.content_captures[0].result.text','Forged text')",
        "UPDATE pages SET data=json_set(data,'$.content_captures[0].observed_generation',999)",
        "UPDATE pages SET data=json_remove(data,'$.content_captures')",
        "UPDATE metadata SET value=json_remove(value,'$.content_capture') WHERE key='config'",
    ] {
        let cfg = config(&fixture, &["/"]);
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("capture.sqlite");
        let crawler = Crawler::new(cfg.clone())
            .unwrap()
            .with_store(&path)
            .unwrap();
        crawler
            .run_controlled(None, CrawlControl::new())
            .await
            .unwrap();
        let db = rusqlite::Connection::open(&path).unwrap();
        let prior: String = db
            .query_row(
                "SELECT value FROM metadata WHERE key='run_state'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        db.execute(mutation, []).unwrap();
        let before = fixture.count("/");
        assert!(CrawlStore::inspect(&path).is_err());
        let crawler = Crawler::new(cfg).unwrap().with_store(&path).unwrap();
        assert!(
            crawler
                .run_controlled(None, CrawlControl::new())
                .await
                .is_err()
        );
        assert_eq!(fixture.count("/"), before);
        let after: String = db
            .query_row(
                "SELECT value FROM metadata WHERE key='run_state'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(prior, after);
    }
}

fn parent() -> SelectedRecrawlParent {
    SelectedRecrawlParent {
        job_id: "own-parent".into(),
        checkpoint_revision: Some(1),
        source_kind: RecrawlSourceKind::StoredJson,
    }
}
#[tokio::test]
async fn selected_recrawl_keeps_unselected_capture_bytes_and_replaces_failed_selected_source_without_expansion()
 {
    let fresh = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = fresh.clone();
    let fixture = Fixture::new(move |request| {
        if flag.load(std::sync::atomic::Ordering::Acquire) && request.path == "/selected" {
            Response::html("<body>Fresh failure &lt;old text absent&gt;</body>").status(404)
        } else {
            Response::html(format!(
                "<html lang='de'><body>Old {} <a href='/outward'>Outward</a></body></html>",
                request.path
            ))
        }
    })
    .await;
    let cfg = config(&fixture, &["/retained", "/selected"]);
    let temp = tempfile::tempdir().unwrap();
    let parent_path = temp.path().join("parent.sqlite");
    let initial = Crawler::new(cfg)
        .unwrap()
        .with_store(&parent_path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    let bytes = serde_json::to_vec(&initial.report).unwrap();
    let retained = initial
        .report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/retained"))
        .unwrap()
        .clone();
    let prepared =
        prepare_selected_recrawl(parent(), &bytes, &[fixture.url("/selected")], None).unwrap();
    let child = temp.path().join("child.sqlite");
    CrawlStore::open(&child)
        .unwrap()
        .initialize_selected_recrawl(&prepared)
        .unwrap();
    assert!(
        CrawlStore::open_read_only(&child)
            .unwrap()
            .pages()
            .unwrap()
            .iter()
            .all(|page| page.url.ends_with("/retained"))
    );
    fresh.store(true, std::sync::atomic::Ordering::Release);
    let result = Crawler::new(prepared.config)
        .unwrap()
        .with_store(&child)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    let actual_retained = result
        .report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/retained"))
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&retained).unwrap(),
        serde_json::to_vec(actual_retained).unwrap()
    );
    let selected = result
        .report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/selected"))
        .unwrap();
    assert_eq!(raw(selected).status, PageContentCaptureStatus::Incomplete);
    assert!(
        raw(selected)
            .result
            .as_ref()
            .unwrap()
            .text
            .as_ref()
            .unwrap()
            .starts_with("Fresh failure")
    );
    let op = result.report.operation.as_ref().unwrap();
    assert_eq!(
        raw(selected).observed_generation,
        op.targets[0].committed_generation
    );
    assert_eq!(fixture.count("/outward"), 0);
    assert_eq!(fixture.count("/retained"), 1);
    assert_eq!(fixture.count("/selected"), 2);
    let reader = CrawlStore::open_read_only(&child).unwrap();
    assert_eq!(reader.parent_snapshot_bytes().unwrap().unwrap(), bytes);
    assert!(reader.inspection().unwrap().checkpoint_valid);
    reader.pages().unwrap();
}

#[tokio::test]
async fn source_page_frontier_and_capture_rollback_together_after_sqlite_commit_failure() {
    let gate = Arc::new(Semaphore::new(0));
    let block = gate.clone();
    let mut fixture = Fixture::new(move |_| {
        Response::html("<body>Real captured text</body>").gated(block.clone())
    })
    .await;
    let cfg = config(&fixture, &["/"]);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("atomic.sqlite");
    let crawler = Crawler::new(cfg).unwrap().with_store(&path).unwrap();
    let task = tokio::spawn(async move { crawler.run_controlled(None, CrawlControl::new()).await });
    fixture.wait("/").await;
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TRIGGER reject_capture BEFORE UPDATE OF done ON frontier WHEN NEW.done=1 BEGIN SELECT RAISE(ABORT,'own atomic fixture'); END;").unwrap();
    gate.add_permits(1);
    assert!(task.await.unwrap().is_err());
    assert_eq!(
        db.query_row("SELECT count(*) FROM pages", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT done FROM frontier", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM metadata WHERE key='checkpoint'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[tokio::test]
#[ignore = "Requires real loopback Chrome worker on port 8790"]
async fn live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness() {
    let fixture=Fixture::new(|_|Response::html("<html lang='de'><body><main>Nativer Text</main><script>document.documentElement.lang='en';document.querySelector('main').textContent='Rendered own text';</script></body></html>")).await;
    let mut cfg = config(&fixture, &["/selected", "/retained"]);
    cfg.content_capture.as_mut().unwrap().sources = vec![
        ContentCaptureSourceKind::RawHttpHtml,
        ContentCaptureSourceKind::RenderedDomHtml,
    ];
    cfg.request_timeout_secs = 20;
    cfg.renderer = Some(RendererConfig {
        wait_ms: 20,
        ..Default::default()
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("real.sqlite");
    let initial = Crawler::new(cfg)
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    for page in &initial.report.pages {
        assert!(page.rendered, "{:?}", page.renderer_error);
        validate_page_content_captures(page).unwrap();
        let raw = raw(page);
        let rendered = row(page, ContentCaptureSourceKind::RenderedDomHtml);
        assert_eq!(
            raw.result.as_ref().unwrap().text.as_deref(),
            Some("Nativer Text")
        );
        assert_eq!(
            raw.result.as_ref().unwrap().html_language.as_deref(),
            Some("de")
        );
        assert_eq!(
            rendered.result.as_ref().unwrap().text.as_deref(),
            Some("Rendered own text")
        );
        assert_eq!(
            rendered.result.as_ref().unwrap().html_language.as_deref(),
            Some("en")
        );
        assert_ne!(raw.source_revision, rendered.source_revision);
        assert_ne!(
            raw.source_html_utf8_sha256,
            rendered.source_html_utf8_sha256
        );
    }
    let bytes = serde_json::to_vec(&initial.report).unwrap();
    let prepared =
        prepare_selected_recrawl(parent(), &bytes, &[fixture.url("/selected")], None).unwrap();
    let child = temp.path().join("child.sqlite");
    CrawlStore::open(&child)
        .unwrap()
        .initialize_selected_recrawl(&prepared)
        .unwrap();
    let result = Crawler::new(prepared.config)
        .unwrap()
        .with_store(&child)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    let selected = result
        .report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/selected"))
        .unwrap();
    let old = initial
        .report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/selected"))
        .unwrap();
    assert_ne!(raw(selected).source_revision, raw(old).source_revision);
    assert_ne!(
        row(selected, ContentCaptureSourceKind::RenderedDomHtml).source_revision,
        row(old, ContentCaptureSourceKind::RenderedDomHtml).source_revision
    );
    assert!(CrawlStore::inspect(&child).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn gzip_entity_and_bom_decoder_report_the_actual_decompressed_and_decoded_representation() {
    use std::io::Write;
    let html = "<html lang='de'><body>Eigene Grüße</body></html>";
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(html.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();
    let wire_digest = sha(&compressed);
    let mut utf16 = vec![0xff, 0xfe];
    for unit in html.encode_utf16() {
        utf16.extend_from_slice(&unit.to_le_bytes());
    }
    let bom_digest = sha(&utf16);
    let fixture = Fixture::new(move |request| {
        if request.path == "/gzip" {
            Response::html(&compressed).header("Content-Encoding", "gzip")
        } else {
            Response::html(&utf16)
        }
    })
    .await;
    let result = run(config(&fixture, &["/gzip", "/bom"])).await;
    for page in &result.report.pages {
        validate_page_content_captures(page).unwrap();
        let capture = raw(page);
        assert_eq!(capture.status, PageContentCaptureStatus::Complete);
        assert_eq!(capture.source_html_utf8_sha256, Some(sha(html)));
        assert_eq!(
            capture.result.as_ref().unwrap().text.as_deref(),
            Some("Eigene Grüße")
        );
        if page.url.ends_with("/gzip") {
            assert_eq!(capture.encoded_body_sha256, Some(sha(html)));
            assert_ne!(capture.encoded_body_sha256, Some(wire_digest.clone()));
            assert_eq!(page.size_bytes, html.len());
        } else {
            assert_eq!(capture.encoded_body_sha256, Some(bom_digest.clone()));
            assert_eq!(capture.decode_encoding.as_deref(), Some("UTF-16LE"));
        }
    }
}

#[tokio::test]
async fn interrupted_sqlite_only_run_retries_source_and_records_only_the_new_generation() {
    let gate = Arc::new(Semaphore::new(0));
    let handler_gate = gate.clone();
    let mut fixture = Fixture::new(move |_| {
        Response::html("<html lang='de'><body>New observed generation</body></html>")
            .gated(handler_gate.clone())
    })
    .await;
    let cfg = config(&fixture, &["/"]);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("crash.sqlite");
    let crawler = Crawler::new(cfg.clone())
        .unwrap()
        .with_store(&path)
        .unwrap();
    let task = tokio::spawn(async move { crawler.run_controlled(None, CrawlControl::new()).await });
    fixture.wait("/").await;
    task.abort();
    let _ = task.await;
    let inspection = CrawlStore::inspect(&path).unwrap();
    assert_eq!(inspection.completed, 0);
    assert_eq!(inspection.pending, 1);
    assert!(!inspection.checkpoint_valid);
    let crawler = Crawler::new(cfg).unwrap().with_store(&path).unwrap();
    let task = tokio::spawn(async move {
        crawler
            .run_controlled(None, CrawlControl::new())
            .await
            .unwrap()
    });
    fixture.wait("/").await;
    gate.add_permits(2);
    let result = task.await.unwrap();
    assert_eq!(fixture.count("/"), 2);
    assert_eq!(raw(&result.report.pages[0]).observed_generation, Some(2));
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn capture_is_an_immutable_optional_setting_on_legacy_resume() {
    let fixture = Fixture::new(|_| Response::html("<body>Legacy run</body>")).await;
    let mut cfg = config(&fixture, &["/"]);
    cfg.content_capture = None;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("legacy.sqlite");
    let result = Crawler::new(cfg.clone())
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert!(result.report.pages[0].content_captures.is_none());
    let db = rusqlite::Connection::open(&path).unwrap();
    let state: String = db
        .query_row(
            "SELECT value FROM metadata WHERE key='run_state'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    cfg.content_capture = Some(ContentCaptureConfig::default());
    assert!(
        Crawler::new(cfg)
            .unwrap()
            .with_store(&path)
            .unwrap()
            .run_controlled(None, CrawlControl::new())
            .await
            .is_err()
    );
    assert_eq!(fixture.count("/"), 1);
    assert_eq!(
        state,
        db.query_row::<String, _, _>(
            "SELECT value FROM metadata WHERE key='run_state'",
            [],
            |row| row.get(0)
        )
        .unwrap()
    );
}

#[tokio::test]
async fn live_policy_source_ledger_capture_page_and_frontier_share_one_commit() {
    let gate = Arc::new(Semaphore::new(0));
    let handler_gate = gate.clone();
    let mut fixture = Fixture::new(move |request| {
        if request.path == "/" {
            Response::html("<html lang='en'><body>Source <a href='/child'>Child</a></body></html>")
                .gated(handler_gate.clone())
        } else {
            Response::html("<body>Child text</body>")
        }
    })
    .await;
    let mut cfg = config(&fixture, &["/"]);
    cfg.crawl_links = true;
    cfg.crawl_policy = Some(CrawlPolicyConfig {
        scope: CrawlScopeConfig {
            roots: vec![CrawlScopeRoot {
                origin: fixture.origin.clone(),
                folders: vec!["/".into()],
                exact_paths: vec![],
                subdomains: false,
            }],
            ..Default::default()
        },
        ..Default::default()
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("policy.sqlite");
    let crawler = Crawler::new(cfg.clone())
        .unwrap()
        .with_store(&path)
        .unwrap();
    let task = tokio::spawn(async move { crawler.run_controlled(None, CrawlControl::new()).await });
    fixture.wait("/").await;
    let db = rusqlite::Connection::open(&path).unwrap();
    let before: i64 = db
        .query_row("SELECT count(*) FROM policy_occurrences", [], |row| {
            row.get(0)
        })
        .unwrap();
    db.execute_batch("CREATE TRIGGER reject_source BEFORE UPDATE OF done ON frontier WHEN NEW.done=1 BEGIN SELECT RAISE(ABORT,'own policy atomic fixture'); END;").unwrap();
    gate.add_permits(1);
    assert!(task.await.unwrap().is_err());
    assert_eq!(
        db.query_row::<i64, _, _>("SELECT count(*) FROM pages", [], |row| row.get(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row::<i64, _, _>("SELECT count(*) FROM policy_occurrences", [], |row| row
            .get(0))
            .unwrap(),
        before
    );
    assert_eq!(
        db.query_row::<i64, _, _>("SELECT count(*) FROM frontier", [], |row| row.get(0))
            .unwrap(),
        1
    );
    assert_eq!(fixture.count("/child"), 0);
    db.execute_batch("DROP TRIGGER reject_source;").unwrap();
    gate.add_permits(1);
    let result = Crawler::new(cfg)
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    for page in &result.report.pages {
        assert_eq!(raw(page).observed_generation, Some(2));
        assert!(page.policy.is_some());
        validate_page_content_captures(page).unwrap();
    }
    let reader = CrawlStore::open_read_only(&path).unwrap();
    let rows = reader.policy_occurrences(None, 100).unwrap();
    assert!(rows.rows.iter().filter(|row| row.generation == 2).count() > 0);
    assert!(reader.inspection().unwrap().checkpoint_valid);
    assert_eq!(fixture.count("/child"), 1);
}
