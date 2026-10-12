//! Original network fixtures for DOM evidence, optional audit and old-report compatibility.
use open_seo_spider_core::{
    CrawlConfig, CrawlStore, Crawler, FragmentStatus, LinkPositionRule, RendererConfig,
    validate_fragments,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, atomic::AtomicBool},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};
type Response = (u16, Vec<(String, String)>, Vec<u8>);
struct Fixture {
    origin: String,
    counts: Arc<Mutex<HashMap<String, usize>>>,
    task: JoinHandle<()>,
}
impl Fixture {
    async fn new(handler: impl Fn(&str, &str, &str) -> Response + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(HashMap::new()));
        let counts = seen.clone();
        let server_origin = origin.clone();
        let handler = Arc::new(handler);
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let handler = handler.clone();
                let seen = seen.clone();
                let origin = server_origin.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let mut chunk = [0; 4096];
                    let header_end = loop {
                        if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n")
                        {
                            break end + 4;
                        }
                        if bytes.len() > 1024 * 1024 {
                            return;
                        }
                        let Ok(Ok(size)) = tokio::time::timeout(
                            std::time::Duration::from_secs(3),
                            stream.read(&mut chunk),
                        )
                        .await
                        else {
                            return;
                        };
                        if size == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&chunk[..size]);
                    };
                    let head = String::from_utf8_lossy(&bytes[..header_end]);
                    let length = head
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                                .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if length > 1024 * 1024 {
                        return;
                    }
                    while bytes.len() < header_end + length {
                        let Ok(Ok(size)) = tokio::time::timeout(
                            std::time::Duration::from_secs(3),
                            stream.read(&mut chunk),
                        )
                        .await
                        else {
                            return;
                        };
                        if size == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&chunk[..size]);
                    }
                    let request = String::from_utf8_lossy(&bytes);
                    let path = request.split_whitespace().nth(1).unwrap_or("/");
                    *seen.lock().unwrap().entry(path.to_owned()).or_insert(0) += 1;
                    let body = String::from_utf8_lossy(&bytes[header_end..]);
                    let (status, headers, body) = handler(&origin, path, &body);
                    let mut response = format!(
                        "HTTP/1.1 {status} Fixture\r\nConnection: close\r\nContent-Length: {}\r\n",
                        body.len()
                    );
                    for (name, value) in headers {
                        response.push_str(&format!("{name}: {value}\r\n"));
                    }
                    response.push_str("\r\n");
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                    let _ = stream.shutdown().await;
                });
            }
        });
        Self {
            origin,
            counts,
            task,
        }
    }
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin)
    }
    fn count(&self, path: &str) -> usize {
        *self.counts.lock().unwrap().get(path).unwrap_or(&0)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
fn response(mime: &str, body: &str) -> Response {
    (
        200,
        vec![("Content-Type".into(), mime.into())],
        body.as_bytes().to_vec(),
    )
}
fn config(url: String) -> CrawlConfig {
    CrawlConfig {
        seed_urls: vec![url],
        obey_robots: false,
        discover_sitemaps: false,
        delay_ms: 0,
        concurrency: 4,
        validate_fragments: true,
        ..Default::default()
    }
}
fn cancel() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

#[tokio::test]
async fn complete_dom_facts_fragment_dedupe_responsive_discovery_and_sqlite_roundtrip() {
    let fixture=Fixture::new(|_,path,_|match path{
        "/start"=>response("text/html",r#"<h2>Before</h2><h1>Main</h1><meta name='keywords' content='one, 🦀'><meta name='keywords' content=''><nav class='hero'><a href='/target#café'>Unicode</a><a href='/target#absent'>Missing</a><a href='/target#café'>Again</a></nav><div href='/not-fetch'>Candidate</div><a onclick="goto('/secret')">Script</a><picture><source srcset='/wide.svg 2x'><img alt='Per occurrence' srcset='/small.svg 1x, data:image/png;base64,AAA 2x'></picture>"#),
        "/target"=>response("text/html","<h1 id='café'>Target</h1>"),
        "/wide.svg"|"/small.svg"=>response("image/svg+xml","<svg width='2' height='3'></svg>"),
        _=>(404,vec![("Content-Type".into(),"text/plain".into())],b"Missing".to_vec()),
    }).await;
    let mut cfg = config(fixture.url("/start"));
    cfg.link_position_rules = vec![
        LinkPositionRule {
            name: "Hero".into(),
            selector: ".hero".into(),
        },
        LinkPositionRule {
            name: "Menu".into(),
            selector: "nav".into(),
        },
    ];
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("dom.sqlite");
    let report = Crawler::new(cfg.clone())
        .unwrap()
        .with_store(&store)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert_eq!(report.pages.len(), 4);
    assert_eq!(fixture.count("/target"), 1);
    assert_eq!(fixture.count("/wide.svg"), 1);
    assert_eq!(fixture.count("/small.svg"), 1);
    assert_eq!(fixture.count("/not-fetch"), 0);
    assert_eq!(fixture.count("/secret"), 0);
    let source = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/start"))
        .unwrap();
    let facts = source.dom_metadata.as_ref().unwrap();
    assert!(facts.complete);
    assert_eq!(
        facts
            .headings
            .iter()
            .map(|heading| heading.tag_level)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    assert_eq!(facts.meta_keywords.len(), 2);
    assert_eq!(
        facts.responsive_images[0].alt.as_deref(),
        Some("Per occurrence")
    );
    assert_eq!(
        source
            .links
            .iter()
            .find(|link| link.kind == "anchor")
            .unwrap()
            .position
            .as_deref(),
        Some("Hero")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "h1_nonsequential")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "meta_keywords_empty")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "nonstandard_internal_navigation")
    );
    let rows = validate_fragments(&report);
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter()
            .filter(|row| row.status == FragmentStatus::MatchedId)
            .count(),
        2
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.status == FragmentStatus::Missing)
            .count(),
        1
    );
    let persisted = CrawlStore::open(&store)
        .unwrap()
        .load_report()
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&persisted).unwrap(),
        serde_json::to_value(&report).unwrap()
    );
    assert_eq!(persisted.config.link_position_rules[0].name, "Hero");
    assert!(persisted.config.validate_fragments);
}

#[tokio::test]
async fn mime_case_and_charset_are_exact_while_unsupported_redirects_stay_unknown() {
    let fixture=Fixture::new(|_,path,_|match path{
        "/start"=>response("Text/HTML; Charset=UTF-8","<a href='/target#x'>Mixed case</a><a href='/latin#x'>Latin</a><a href='/fake#x'>Fake</a><a href='/xhtml#x'>XML</a><a href='/redirect#x'>Redirect</a><a href='/svg-html#x'>Inline SVG HTML</a><a href='/svg-resource#x'>SVG resource</a><a href='/svg-fake#x'>SVG misleading MIME</a>"),
        "/target"=>response("TEXT/HTML","<h1 id='x'>Real</h1>"),
        "/latin"=>(200,vec![("Content-Type".into(),"Text/HTML; Charset=ISO-8859-1".into())],b"<title>Caf\xe9</title><div id='x'></div>".to_vec()),
        "/fake"=>response("application/x-text/html-extra","<h1 id='x'>Not HTML MIME</h1>"),
        "/xhtml"=>response("Application/XHTML+XML","<html xmlns='http://www.w3.org/1999/xhtml'><body><h1 id='x'>XML</h1></body></html>"),
        "/svg-html"=>response("Text/HTML","<svg id='x'><circle r='1'></circle></svg>"),
        "/svg-resource"=>response("Image/SVG+XML; Charset=UTF-8","<svg id='x' width='2' height='3'></svg>"),
        "/svg-fake"=>response("application/x-text/html-extra","<svg id='x' width='4' height='5'></svg>"),
        "/redirect"=>(302,vec![("Location".into(),"/target#override".into())],vec![]),
        _=>response("text/plain",""),
    }).await;
    let report = Crawler::new(config(fixture.url("/start")))
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    let source = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/start"))
        .unwrap();
    assert!(source.is_html());
    assert_eq!(source.content_type, "Text/HTML; Charset=UTF-8");
    let latin = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/latin"))
        .unwrap();
    assert_eq!(latin.title(), "Café");
    assert!(latin.dom_metadata.is_some());
    let fake = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/fake"))
        .unwrap();
    assert!(!fake.is_html());
    assert!(fake.dom_metadata.is_none());
    let svg_resource = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/svg-resource"))
        .unwrap();
    assert!(!svg_resource.is_html());
    assert!(svg_resource.dom_metadata.is_none());
    let intrinsic = svg_resource.image_metadata.as_ref().unwrap();
    assert_eq!(intrinsic.width, Some(2.0));
    assert_eq!(intrinsic.height, Some(3.0));
    let svg_fake = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/svg-fake"))
        .unwrap();
    assert!(!svg_fake.is_html());
    assert!(svg_fake.dom_metadata.is_none());
    let rows = validate_fragments(&report);
    let find = |path: &str| {
        rows.iter()
            .find(|row| row.target_url.ends_with(path))
            .unwrap()
    };
    assert_eq!(find("/target").status, FragmentStatus::MatchedId);
    assert_eq!(find("/latin").status, FragmentStatus::MatchedId);
    assert_eq!(find("/svg-html").status, FragmentStatus::MatchedId);
    assert_eq!(find("/svg-resource").status, FragmentStatus::Unsupported);
    assert_eq!(find("/svg-fake").status, FragmentStatus::Unsupported);
    assert_eq!(find("/fake").status, FragmentStatus::Unsupported);
    assert_eq!(find("/xhtml").status, FragmentStatus::Unsupported);
    assert_eq!(find("/redirect").status, FragmentStatus::Unknown);
    assert_eq!(
        find("/redirect").reason,
        "redirect_or_navigation_fragment_propagation_unknown"
    );
}

/// Membership acceptance is scoped to these independently authored 24 HTML
/// cases, whose actual reference CSV memberships were observed locally. It is
/// deliberately not a claim of universal vendor/filter equivalence.
#[tokio::test]
async fn authored_24_heading_cases_match_bounded_reference_membership_with_native_requests() {
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/parity/reference-headings/fixture-cases.json"
    ))
    .unwrap();
    let cases = source["html_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 24);
    let bodies: HashMap<String, String> = cases
        .iter()
        .map(|case| {
            (
                case["path"].as_str().unwrap().to_owned(),
                case["response"]["html"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let fixture = Fixture::new(move |_, path, _| {
        bodies.get(path).map_or_else(
            || response("text/plain", ""),
            |body| response("text/html; charset=utf-8", body),
        )
    })
    .await;
    let mut options = config(fixture.url("/"));
    options.seed_urls = cases
        .iter()
        .map(|case| fixture.url(case["path"].as_str().unwrap()))
        .collect();
    options.crawl_links = false;
    options.validate_fragments = false;
    let report = Crawler::new(options)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert_eq!(report.pages.len(), 24);
    let expected_h1 = [
        "/css-hidden-h2-before-h1",
        "/empty-h2-before-h1",
        "/h2-h1",
        "/h3-h1-h2",
        "/hidden-h2-before-h1",
        "/image-alt-h2-before-h1",
    ];
    let expected_h2 = ["/h1-h3-h1-h2", "/h1-h3-h2"];
    for (code, expected) in [
        ("h1_nonsequential", expected_h1.as_slice()),
        ("h2_nonsequential", expected_h2.as_slice()),
    ] {
        let mut actual = report
            .issues
            .iter()
            .filter(|issue| issue.code == code)
            .map(|issue| url::Url::parse(&issue.url).unwrap().path().to_owned())
            .collect::<Vec<_>>();
        actual.sort();
        assert_eq!(actual, expected, "{code}");
    }
    for (path, alt_text) in [
        ("/image-alt-only-h1-h2", "Image-only primary"),
        ("/image-alt-h2-before-h1", "Image-only early section"),
    ] {
        let page = report
            .pages
            .iter()
            .find(|page| page.url.ends_with(path))
            .unwrap();
        let heading = &page.dom_metadata.as_ref().unwrap().headings[0];
        assert_eq!(heading.raw_text, "");
        assert_eq!(heading.text_with_image_alt.as_deref(), Some(alt_text));
    }
}

#[tokio::test]
async fn configured_positions_fail_before_any_request_and_defaults_remain_compatible() {
    let fixture = Fixture::new(|_, _, _| response("text/html", "<h1>Control</h1>")).await;
    for rules in [
        vec![LinkPositionRule {
            name: "name".into(),
            selector: "[".into(),
        }],
        vec![LinkPositionRule {
            name: " ".into(),
            selector: "a".into(),
        }],
        vec![
            LinkPositionRule {
                name: "same".into(),
                selector: "a".into(),
            },
            LinkPositionRule {
                name: " same ".into(),
                selector: "p".into(),
            },
        ],
        vec![
            LinkPositionRule {
                name: "name".into(),
                selector: "a".into()
            };
            33
        ],
    ] {
        let mut cfg = config(fixture.url("/start"));
        cfg.link_position_rules = rules;
        assert!(Crawler::new(cfg).is_err());
    }
    assert_eq!(fixture.count("/start"), 0);
    let old: CrawlConfig = serde_json::from_str("{}").unwrap();
    assert!(!old.validate_fragments);
    assert!(old.link_position_rules.is_empty());
}

#[tokio::test]
async fn renderer_keeps_native_snapshot_and_replaces_current_dom_facts_with_provenance() {
    let site=Fixture::new(|_,_,_|response("text/html","<h2>Native order</h2><h1 id='old'>Native</h1><meta name='keywords' content='raw'><a href='#old'>Raw</a>")).await;
    let worker=Fixture::new(|_,_,body|{
        let input:serde_json::Value=serde_json::from_str(body).unwrap();let payload=serde_json::json!({"html":"<h1 id='new'>Rendered</h1><h2>Section</h2><meta name='keywords' content='rendered'><a href='#new'>Rendered</a>","final_url":input["url"],"status":200,"headers":{},"title":"Rendered","console_errors":[],"network_requests":[],"rendered_at":"2026-10-12T00:00:00Z"});response("application/json",&payload.to_string())
    }).await;
    let mut cfg = config(site.url("/start"));
    cfg.crawl_links = false;
    cfg.renderer = Some(RendererConfig {
        endpoint: worker.url("/render"),
        wait_ms: 0,
        ..Default::default()
    });
    let report = Crawler::new(cfg)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    let page = &report.pages[0];
    assert!(page.rendered);
    let raw = page
        .raw_metadata
        .as_ref()
        .unwrap()
        .dom_metadata
        .as_ref()
        .unwrap();
    let rendered = page.dom_metadata.as_ref().unwrap();
    assert_eq!(raw.source, "raw_html");
    assert_eq!(rendered.source, "rendered_html");
    assert_eq!(raw.headings[0].tag_level, 2);
    assert_eq!(rendered.headings[0].tag_level, 1);
    assert_eq!(raw.meta_keywords[0].content.as_deref(), Some("raw"));
    assert_eq!(
        rendered.meta_keywords[0].content.as_deref(),
        Some("rendered")
    );
    assert_eq!(page.links[0].fragment.as_deref(), Some("new"));
    assert_eq!(
        validate_fragments(&report)[0].status,
        FragmentStatus::MatchedId
    );
    assert!(
        !report
            .issues
            .iter()
            .any(|issue| issue.code == "h1_nonsequential")
    );
}
