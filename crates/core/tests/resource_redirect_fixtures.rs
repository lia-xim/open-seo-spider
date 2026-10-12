//! Frozen B10 redirect templates, explicit legacy V1 and opt-in V2 oracles.
//! Local authored HTTP sources/binaries; no proprietary or browser parity claim.
use anyhow::{Context, Result, bail};
use lopdf::{Document, Stream, dictionary, text_string};
use open_seo_spider_core::*;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const FAMILIES: [&str; 7] = [
    "css",
    "javascript",
    "images",
    "media",
    "pdf",
    "swf",
    "other",
];
const SECRET: &str = "Bearer own-redirect-fixture";
const COOKIE: &str = "own_fixture=controlled";

#[derive(Clone)]
struct Response {
    status: u16,
    mime: String,
    body: Vec<u8>,
    headers: Vec<(String, String)>,
}
impl Response {
    fn new(mime: &str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            mime: mime.into(),
            body: body.into(),
            headers: vec![],
        }
    }
    fn html(body: impl Into<Vec<u8>>) -> Self {
        Self::new("text/html; charset=utf-8", body)
    }
    fn redirect(status: u16, location: Option<&str>, tag: &str) -> Self {
        let mut response = Self::new("text/plain; charset=utf-8", Vec::new());
        response.status = status;
        response.headers.push(("X-Own-Hop".into(), tag.into()));
        response
            .headers
            .push(("X-Robots-Tag".into(), "nofollow".into()));
        if let Some(location) = location {
            response.headers.push(("Location".into(), location.into()));
        }
        response
    }
}

struct Fixture {
    origin: String,
    routes: Arc<Mutex<BTreeMap<String, Response>>>,
    requests: Arc<Mutex<Vec<Value>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Fixture {
    async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let routes = Arc::new(Mutex::new(BTreeMap::<String, Response>::new()));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let task_routes = routes.clone();
        let task_requests = requests.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let routes = task_routes.clone();
                let requests = task_requests.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buffer = [0u8; 4096];
                    while request.len() < 16384 {
                        let Ok(count) = socket.read(&mut buffer).await else {
                            return;
                        };
                        if count == 0 {
                            return;
                        }
                        request.extend_from_slice(&buffer[..count]);
                        if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let text = String::from_utf8_lossy(&request);
                    let mut lines = text.lines();
                    let mut first = lines.next().unwrap_or("").split_whitespace();
                    let method = first.next().unwrap_or("").to_owned();
                    let path = first.next().unwrap_or("").to_owned();
                    let headers = lines
                        .filter_map(|line| line.split_once(':'))
                        .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
                        .collect::<BTreeMap<_, _>>();
                    requests
                        .lock()
                        .unwrap()
                        .push(json!({"method":method,"path":path,"headers":headers}));
                    let response =
                        routes
                            .lock()
                            .unwrap()
                            .get(&path)
                            .cloned()
                            .unwrap_or_else(|| {
                                let mut r = Response::new("text/plain", "No authored route");
                                r.status = 404;
                                r
                            });
                    let mut head = format!(
                        "HTTP/1.1 {} Owned\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
                        response.status,
                        response.mime,
                        response.body.len()
                    );
                    for (key, value) in response.headers {
                        head.push_str(&format!("{key}: {value}\r\n"));
                    }
                    head.push_str("\r\n");
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(&response.body).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        Self {
            origin,
            routes,
            requests,
            task,
        }
    }
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin)
    }
    fn put(&self, path: &str, response: Response) {
        self.routes.lock().unwrap().insert(path.into(), response);
    }
    fn log(&self) -> Vec<Value> {
        self.requests.lock().unwrap().clone()
    }
    fn counts(&self) -> BTreeMap<String, usize> {
        let mut out = BTreeMap::new();
        for row in self.log() {
            *out.entry(row["path"].as_str().unwrap().into()).or_default() += 1;
        }
        out
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Default)]
struct Checks {
    rows: Vec<Value>,
}
impl Checks {
    fn eq(&mut self, key: &str, actual: impl Serialize, expected: impl Serialize) {
        let actual = serde_json::to_value(actual).unwrap();
        let expected = serde_json::to_value(expected).unwrap();
        self.rows
            .push(json!({"key":key,"pass":actual==expected,"actual":actual,"expected":expected}));
    }
    fn passed(&self) -> bool {
        !self.rows.is_empty() && self.rows.iter().all(|r| r["pass"] == true)
    }
}
fn hash(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
fn pdf(link: &str) -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let pages = doc.new_object_id();
    let font = doc.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica","Encoding"=>"WinAnsiEncoding"});
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf (Own redirect PDF text) Tj ET".to_vec(),
    ));
    let annotation: lopdf::Object = dictionary! {"Subtype"=>"Link","Rect"=>vec![0.into(),0.into(),10.into(),10.into()],"A"=>dictionary!{"S"=>"URI","URI"=>text_string(link)}}.into();
    let page = doc.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()],"Contents"=>content,"Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}},"Annots"=>vec![annotation]});
    doc.set_object(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1},
    );
    let catalog = doc.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    doc.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

fn png() -> Vec<u8> {
    fn chunk(bytes: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(data);
        let mut crc = u32::MAX;
        for byte in kind.iter().chain(data) {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb88320u32.wrapping_mul(crc & 1));
            }
        }
        bytes.extend_from_slice(&(!crc).to_be_bytes());
    }
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut bytes, b"IHDR", &ihdr);
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&[0, 0, 0, 0, 255]).unwrap();
    chunk(&mut bytes, b"IDAT", &encoder.finish().unwrap());
    chunk(&mut bytes, b"IEND", &[]);
    bytes
}
fn wav() -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&38u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&8000u32.to_le_bytes());
    bytes.extend_from_slice(&16000u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&0i16.to_le_bytes());
    bytes
}
fn declaration(category: &str, url: &str) -> String {
    match category {
        "css" => format!("<link rel='stylesheet' href='{url}'>"),
        "javascript" => format!("<script src='{url}'></script>"),
        "images" => format!("<img src='{url}'>"),
        "media" => format!("<audio src='{url}'></audio>"),
        "pdf" => format!("<object type='application/pdf' data='{url}'></object>"),
        "swf" => format!("<embed type='application/x-shockwave-flash' src='{url}'>"),
        "other" => format!("<video><track src='{url}' kind='captions' srclang='en'></video>"),
        "internal_anchors" => format!("<a href='{url}'>Own navigation</a>"),
        _ => unreachable!(),
    }
}
fn reference_kind(category: &str) -> ResourceReferenceKind {
    match category {
        "css" => ResourceReferenceKind::Css,
        "javascript" => ResourceReferenceKind::JavaScript,
        "images" => ResourceReferenceKind::Image,
        "media" => ResourceReferenceKind::Media,
        "pdf" => ResourceReferenceKind::Pdf,
        "swf" => ResourceReferenceKind::Swf,
        "other" => ResourceReferenceKind::Other,
        "internal_anchors" => ResourceReferenceKind::InternalAnchor,
        _ => unreachable!(),
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum Boundary {
    Planned,
    Status(u16),
    Limit(usize),
    InvalidLocation(bool),
}
#[derive(Clone)]
struct Case {
    id: String,
    template: String,
    category: String,
    definition: Value,
    boundary: Boundary,
}
impl Case {
    fn nav(&self) -> bool {
        self.category == "internal_anchors"
    }
    fn legacy_exception(&self, profile: &str) -> bool {
        profile == "v1"
            && matches!(
                self.template.as_str(),
                "hops-checkexternal-not-resource-wildcard"
                    | "hops-policy-wide-resource-narrow"
                    | "nav-redirect-resource-origin-only"
                    | "nav-allowlisted-backlink-current-behavior"
            )
    }
    fn policy(&self) -> bool {
        matches!(
            self.template.as_str(),
            "hops-policy-wide-resource-narrow" | "hops-policy-deny" | "hops-policy-checkonce"
        )
    }
    fn planned_gets(&self, profile: &str) -> Result<Vec<usize>> {
        let mut gets = if self.nav() || self.template == "image-redirect-html-no-upgrade" {
            self.definition[if profile == "v1" {
                "current_v1_predicted_gets"
            } else {
                "desired_gets"
            }]
            .as_array()
            .context("Navigation GET oracle missing")?
            .iter()
            .take(2)
            .map(|v| v.as_u64().unwrap() as usize)
            .collect::<Vec<_>>()
        } else {
            self.definition["required_gets"]
                .as_array()
                .context("Template GET oracle missing")?
                .iter()
                .map(|v| v.as_u64().unwrap() as usize)
                .collect::<Vec<_>>()
        };
        if self.legacy_exception(profile) && !self.nav() {
            gets[2] = 1;
        }
        match self.boundary {
            Boundary::Status(code) if !matches!(code, 301 | 302 | 303 | 307 | 308) => {
                gets = vec![1, 0, 0]
            }
            Boundary::Limit(1) => gets = vec![1, 1, 0],
            Boundary::InvalidLocation(_) => gets = vec![1, 0, 0],
            _ => {}
        }
        Ok(gets)
    }
}

fn final_response(category: &str, html: bool, backlink: &str, image: &str) -> Response {
    if html {
        return Response::html(format!(
            "<title>Own redirect landing</title><a href='{backlink}'>Own backlink</a><img src='{image}'>"
        ));
    }
    match category {
        "css" => Response::new(
            "text/css; charset=utf-8",
            "@charset \"UTF-8\";@import './css-child';body{background:url('./image-child')}",
        ),
        "javascript" => Response::new(
            "text/javascript; charset=utf-8",
            "/* Own script transport; no execution */",
        ),
        "images" => Response::new("image/png", png()),
        "media" => Response::new("audio/wav", wav()),
        "pdf" => Response::new("application/pdf", pdf(backlink)),
        "swf" => Response::new(
            "application/x-shockwave-flash",
            b"FWS\x09\x10\x00\x00\x00\x08\x00\x00\x0c\x01\x00\x00\x00".to_vec(),
        ),
        "other" => Response::new(
            "text/vtt; charset=utf-8",
            "WEBVTT\n\n00:00.000 --> 00:00.500\nOwned caption\n",
        ),
        _ => unreachable!(),
    }
}
fn root(origin: &str) -> CrawlScopeRoot {
    CrawlScopeRoot {
        origin: origin.into(),
        subdomains: false,
        folders: vec!["/".into()],
        exact_paths: vec![],
    }
}
fn configuration(
    case: &Case,
    profile: &str,
    fixtures: &[Fixture; 3],
    source: &str,
    negative: &str,
) -> Result<CrawlConfig> {
    let mut discovery = ResourceDiscoveryConfig::default();
    for field in [
        &mut discovery.resources.css,
        &mut discovery.resources.javascript,
        &mut discovery.resources.images,
        &mut discovery.resources.media,
        &mut discovery.resources.pdf,
        &mut discovery.resources.swf,
        &mut discovery.resources.other,
    ] {
        *field = ResourceSwitch {
            store: false,
            fetch: false,
        };
    }
    discovery.resources.images = ResourceSwitch {
        store: true,
        fetch: true,
    };
    for field in [
        &mut discovery.navigation.internal_anchors,
        &mut discovery.navigation.external_anchors,
        &mut discovery.navigation.iframes,
        &mut discovery.navigation.refresh,
        &mut discovery.navigation.canonical,
        &mut discovery.navigation.hreflang,
        &mut discovery.navigation.pagination,
        &mut discovery.navigation.amp,
        &mut discovery.navigation.mobile_alternate,
        &mut discovery.navigation.pdf_annotations,
    ] {
        *field = NavigationSwitch {
            store: false,
            fetch: false,
            expand: false,
        };
    }
    if case.nav() {
        discovery.navigation.internal_anchors = NavigationSwitch {
            store: true,
            fetch: true,
            expand: true,
        };
    } else {
        let enabled = ResourceSwitch {
            store: true,
            fetch: true,
        };
        match case.category.as_str() {
            "css" => discovery.resources.css = enabled,
            "javascript" => discovery.resources.javascript = enabled,
            "images" => discovery.resources.images = enabled,
            "media" => discovery.resources.media = enabled,
            "pdf" => {
                discovery.resources.pdf = enabled;
                discovery.navigation.pdf_annotations = NavigationSwitch {
                    store: true,
                    fetch: true,
                    expand: false,
                };
            }
            "swf" => discovery.resources.swf = enabled,
            "other" => discovery.resources.other = enabled,
            _ => bail!("Unmapped planned resource family"),
        }
    }
    discovery.resource_scope.external_origins = if case.template == "hops-scope-scheme" {
        vec![fixtures[1].origin.replacen("http:", "https:", 1)]
    } else {
        let mut allowed = vec![fixtures[1].origin.clone()];
        if matches!(
            case.template.as_str(),
            "hops-two-allowlisted"
                | "hops-robots-middle-denied"
                | "hops-robots-final-denied"
                | "hops-policy-deny"
        ) {
            allowed.push(fixtures[2].origin.clone());
        }
        if case.template == "nav-redirect-with-explicit-nav-scope" {
            allowed.clear();
        }
        allowed
    };
    if profile == "v2" {
        discovery.contract = Some(ResourceContractConfig {
            engine: RESOURCE_CONTRACT_ENGINE.into(),
            follow_internal_qualified_links: false,
            follow_external_qualified_links: false,
        });
    }
    let mut cfg = CrawlConfig {
        seed_urls: vec![source.into()],
        obey_robots: true,
        discover_sitemaps: false,
        concurrency: 1,
        delay_ms: 0,
        request_timeout_secs: 5,
        max_pages: Some(32),
        max_depth: Some(5),
        resource_discovery: Some(discovery),
        check_external: matches!(
            case.template.as_str(),
            "hops-checkexternal-not-resource-wildcard" | "nav-redirect-with-explicit-nav-scope"
        ),
        exclude_patterns: vec![format!("{negative}$")],
        ..Default::default()
    };
    cfg.custom_headers
        .insert("Authorization".into(), SECRET.into());
    cfg.custom_headers.insert("Cookie".into(), COOKIE.into());
    cfg.custom_headers
        .insert("X-Own-Secret".into(), "own-private-header".into());
    if let Boundary::Limit(limit) = case.boundary {
        cfg.max_redirects = limit;
    }
    if case.policy() {
        // Native policy owns filtering; mixing it with legacy filters is an
        // invalid configuration and would prevent the intended HTTP fixture.
        cfg.exclude_patterns.clear();
        let mut policy = CrawlPolicyConfig {
            scope: CrawlScopeConfig {
                roots: vec![root(&fixtures[0].origin)],
                ..Default::default()
            },
            exclude_patterns: vec![format!("{negative}$")],
            ..Default::default()
        };
        if case.template != "hops-policy-checkonce" {
            policy.scope.roots.push(root(&fixtures[1].origin));
        }
        if case.template == "hops-policy-wide-resource-narrow" {
            policy.scope.roots.push(root(&fixtures[2].origin));
        }
        if case.template == "hops-policy-checkonce" {
            policy.scope.external = OutsideScopePolicy::CheckOnce;
        }
        cfg.crawl_policy = Some(policy);
    }
    Ok(cfg)
}
fn bump(map: &mut BTreeMap<String, usize>, path: &str, count: usize) {
    if count != 0 {
        *map.entry(path.into()).or_default() += count;
    }
}

async fn execute(
    case: &Case,
    profile: &str,
    fixtures: &[Fixture; 3],
    directory: &std::path::Path,
) -> Result<Value> {
    // No target appears in a second seed, control declaration or unrelated role.
    let prefix = format!("/{}-{profile}", case.id.replace('/', "-"));
    let source_path = format!("{prefix}/source");
    let start_path = format!("{prefix}/start");
    let mid_path = format!("{prefix}/mid");
    let final_path = format!("{prefix}/final");
    let positive_path = format!("{prefix}/positive-control");
    let negative_path = format!("{prefix}/negative-control");
    let css_child_path = format!("{prefix}/css-child");
    let image_child_path = format!("{prefix}/image-child");
    let nav_child_path = format!("{prefix}/nav-child");
    let source_url = fixtures[0].url(&source_path);
    let requested_url = fixtures[0].url(&start_path);
    let two = case.nav()
        || matches!(
            case.template.as_str(),
            "hops-scope-port" | "hops-scope-scheme" | "image-redirect-html-no-upgrade"
        );
    let end_origin = if matches!(
        case.template.as_str(),
        "hops-return-seed" | "hops-policy-checkonce" | "hops-loop"
    ) {
        0
    } else if case.nav()
        || case.template == "hops-scope-scheme"
        || case.template == "image-redirect-html-no-upgrade"
    {
        1
    } else {
        2
    };
    let end_path = if case.template == "hops-loop" {
        &start_path
    } else {
        &final_path
    };
    let mut chain = vec![(0usize, start_path.clone())];
    if !two {
        chain.push((1, mid_path.clone()));
    }
    chain.push((end_origin, end_path.clone()));
    let literals = chain
        .iter()
        .skip(1)
        .map(|(index, path)| fixtures[*index].url(path))
        .collect::<Vec<_>>();
    let code_index = FAMILIES
        .iter()
        .position(|c| *c == case.category)
        .unwrap_or(0);
    let first_status = if let Boundary::Status(code) = case.boundary {
        code
    } else {
        [301, 302, 303, 307, 308, 301, 302][code_index]
    };
    let second_status = [308, 307, 303, 302, 301, 308, 307][code_index];
    let final_html = case.nav() || case.template == "image-redirect-html-no-upgrade";
    let mut final_body = final_response(
        &case.category,
        final_html,
        &fixtures[0].url(&nav_child_path),
        &fixtures[0].url(&image_child_path),
    );
    final_body
        .headers
        .push(("X-Own-Hop".into(), "final-own-response".into()));
    let source = Response::html(format!(
        "<title>Own complete redirect source</title>{}{}{}",
        declaration(&case.category, &requested_url),
        declaration("images", &fixtures[0].url(&positive_path)),
        declaration("images", &fixtures[0].url(&negative_path))
    ));
    fixtures[0].put(&source_path, source.clone());
    fixtures[0].put(&positive_path, Response::new("image/png", png()));
    fixtures[0].put(&negative_path, Response::new("image/png", png()));
    for (index, fixture) in fixtures.iter().enumerate() {
        let denied = if case.template == "hops-robots-middle-denied" && index == 1 {
            Some(&mid_path)
        } else if case.template == "hops-robots-final-denied" && index == 2 {
            Some(&final_path)
        } else {
            None
        };
        fixture.put(
            "/robots.txt",
            Response::new(
                "text/plain",
                format!(
                    "User-agent: *\n{}\n",
                    denied.map_or_else(|| "Allow: /".into(), |path| format!("Disallow: {path}"))
                ),
            ),
        );
        fixture.put(
            &css_child_path,
            Response::new("text/css; charset=utf-8", "/* Own child stylesheet */"),
        );
        fixture.put(&image_child_path, Response::new("image/png", png()));
        fixture.put(
            &nav_child_path,
            Response::html("<title>Own child leaf</title>"),
        );
    }
    if case.template != "hops-loop" {
        fixtures[end_origin].put(end_path, final_body.clone());
    }
    fixtures[0].put(
        &start_path,
        Response::redirect(first_status, Some(&literals[0]), "initial-own-redirect"),
    );
    if !two {
        fixtures[1].put(
            &mid_path,
            Response::redirect(second_status, Some(&literals[1]), "middle-own-redirect"),
        );
    }
    let invalid_literal = "mailto:owned@example.invalid";
    if let Boundary::InvalidLocation(missing) = case.boundary {
        fixtures[0].put(
            &start_path,
            Response::redirect(
                first_status,
                (!missing).then_some(invalid_literal),
                "invalid-own-redirect",
            ),
        );
    }
    let terminal_status = if let Boundary::Status(code) = case.boundary {
        (!matches!(code, 301 | 302 | 303 | 307 | 308)).then_some(code)
    } else {
        None
    };
    if let Some(code) = terminal_status {
        final_body = Response::new("image/png", if code == 304 { vec![] } else { png() });
        final_body.status = code;
        final_body
            .headers
            .push(("Location".into(), literals[0].clone()));
        final_body
            .headers
            .push(("X-Own-Hop".into(), "terminal-own-status".into()));
        fixtures[0].put(&start_path, final_body.clone());
    }
    let cfg = configuration(case, profile, fixtures, &source_url, &negative_path)?;
    let crawler = Crawler::new(cfg)?;
    let cfg = crawler.effective_config().clone();
    let path = directory.join("redirect.sqlite");
    let crawl = crawler
        .with_store(&path)?
        .run_controlled(None, CrawlControl::new())
        .await?;
    let store = CrawlStore::open_read_only(&path)?;
    let mut rows = Vec::new();
    store.visit_resource_occurrences(3, |page| {
        rows.extend(page.rows.clone());
        Ok(())
    })?;
    let mut again = Vec::new();
    store.visit_resource_occurrences(2, |page| {
        again.extend(page.rows.clone());
        Ok(())
    })?;
    let transports = store.resource_transports(None, 1000)?.rows;
    let transport = transports
        .iter()
        .find(|t| t.requested_url == requested_url)
        .context("Requested target transport missing")?;
    let page = &transport.projection;
    let parent_rows = rows
        .iter()
        .filter(|row| {
            row.occurrence.requested_source_url == source_url
                && row.occurrence.resolved_url.as_deref() == Some(&requested_url)
        })
        .collect::<Vec<_>>();
    let parent = parent_rows
        .first()
        .context("Original role occurrence missing")?;
    let grant = parent
        .decision
        .grant
        .as_ref()
        .context("Original role grant missing")?;
    let permission = page
        .resource_permission
        .as_ref()
        .context("Original task permission missing")?;
    let facts = page
        .resource_response
        .as_ref()
        .context("Actual response facts missing")?;
    let batch = page
        .resource_source
        .as_ref()
        .context("Captured source inventory missing")?;
    let gets = case.planned_gets(profile)?;
    let final_fetched = terminal_status.is_some()
        || (gets.last() == Some(&1)
            && case.template != "hops-loop"
            && !matches!(case.boundary, Boundary::InvalidLocation(_)));
    let expected_status = terminal_status.or(final_fetched.then_some(200));
    let complete = expected_status == Some(200);
    let invalid = matches!(case.boundary, Boundary::InvalidLocation(_));
    let mut expected = [BTreeMap::new(), BTreeMap::new(), BTreeMap::new()];
    bump(&mut expected[0], "/robots.txt", 1);
    bump(&mut expected[0], &source_path, 1);
    bump(&mut expected[0], &positive_path, 1);
    for ((index, path), count) in chain.iter().zip(&gets) {
        bump(&mut expected[*index], path, *count);
        if *count != 0 {
            expected[*index].insert("/robots.txt".into(), 1);
        }
    }
    if case.template == "hops-robots-middle-denied" {
        expected[1].insert("/robots.txt".into(), 1);
    }
    if case.template == "hops-robots-final-denied" {
        expected[2].insert("/robots.txt".into(), 1);
    }
    let capped = case.template == "hops-policy-checkonce";
    let css_scope = !case.legacy_exception(profile) || case.nav();
    let css_children = usize::from(complete && case.category == "css" && !capped && css_scope);
    let pdf_children = usize::from(complete && case.category == "pdf" && !capped);
    let nav_children = usize::from(complete && case.nav());
    if css_children != 0 {
        bump(&mut expected[end_origin], &css_child_path, 1);
        bump(&mut expected[end_origin], &image_child_path, 1);
    }
    bump(
        &mut expected[0],
        &nav_child_path,
        pdf_children + nav_children,
    );
    bump(&mut expected[0], &image_child_path, nav_children);
    let mut checks = Checks::default();
    checks.eq(
        "one_initial_input_no_hidden_seeds",
        &cfg.seed_urls,
        vec![source_url.clone()],
    );
    checks.eq("no_sitemap_inputs", cfg.sitemap_urls.len(), 0usize);
    checks.eq("one_original_role_occurrence", parent_rows.len(), 1usize);
    checks.eq(
        "immutable_original_role",
        parent.occurrence.reference_kind,
        reference_kind(&case.category),
    );
    checks.eq(
        "original_source_complete",
        parent.occurrence.source_complete,
        true,
    );
    checks.eq(
        "immutable_original_occurrence_id",
        &parent.occurrence.occurrence_id,
        hash(format!(
            "resource:{}:{}",
            parent.occurrence.source_revision, parent.occurrence.source_ordinal
        )),
    );
    checks.eq(
        "original_grant_fetch_identity",
        &grant.fetch_url,
        &requested_url,
    );
    checks.eq(
        "original_grant_config_hash",
        &grant.config_sha256,
        cfg.resource_discovery.as_ref().unwrap().sha256()?,
    );
    checks.eq(
        "original_grant_role_profile",
        grant.reference_kind,
        (profile == "v2").then_some(reference_kind(&case.category)),
    );
    checks.eq(
        "original_transport_witness_exact",
        &permission.authorizing_grants,
        vec![grant.clone()],
    );
    checks.eq(
        "original_css_capability",
        grant.can_expand_css,
        case.category == "css",
    );
    checks.eq(
        "original_navigation_capability",
        grant.can_expand_navigation,
        case.nav(),
    );
    checks.eq(
        "original_resource_capability",
        grant.can_expand_resources,
        case.nav(),
    );
    checks.eq(
        "original_pdf_capability",
        grant.can_expand_pdf,
        case.category == "pdf",
    );
    checks.eq("immutable_verified_ledger_read", &rows, &again);
    checks.eq(
        "requested_url_not_replaced_by_landing",
        &page.url,
        &requested_url,
    );
    checks.eq(
        "source_requested_identity_preserved",
        &batch.requested_url,
        &requested_url,
    );
    checks.eq(
        "contract_engine_provenance",
        facts.contract_engine.as_deref(),
        (profile == "v2").then_some(RESOURCE_CONTRACT_ENGINE),
    );
    for (index, fixture) in fixtures.iter().enumerate() {
        checks.eq(
            &format!("exact_origin_{index}_http_with_robots_source_controls_descendants"),
            fixture.counts(),
            &expected[index],
        );
        for (ordinal, request) in fixture.log().iter().enumerate() {
            let credentialed = index == 0 && request["path"] != "/robots.txt";
            checks.eq(
                &format!("origin_{index}_request_{ordinal}_method_get"),
                &request["method"],
                "GET",
            );
            checks.eq(
                &format!("origin_{index}_request_{ordinal}_authorization"),
                request["headers"]["authorization"].as_str(),
                credentialed.then_some(SECRET),
            );
            checks.eq(
                &format!("origin_{index}_request_{ordinal}_cookie"),
                request["headers"]["cookie"].as_str(),
                credentialed.then_some(COOKIE),
            );
            checks.eq(
                &format!("origin_{index}_request_{ordinal}_custom_header"),
                request["headers"]["x-own-secret"].as_str(),
                credentialed.then_some("own-private-header"),
            );
        }
    }
    checks.eq(
        "independent_positive_control_get",
        fixtures[0]
            .counts()
            .get(&positive_path)
            .copied()
            .unwrap_or(0),
        1usize,
    );
    checks.eq(
        "independent_negative_control_get",
        fixtures[0]
            .counts()
            .get(&negative_path)
            .copied()
            .unwrap_or(0),
        0usize,
    );
    checks.eq(
        "negative_control_observed_and_denied",
        rows.iter()
            .filter(|r| {
                r.occurrence.resolved_url.as_deref() == Some(&fixtures[0].url(&negative_path))
                    && r.decision.reason
                        == if case.policy() {
                            "policy_denied"
                        } else {
                            "filter_denied"
                        }
                    && r.decision.grant.is_none()
            })
            .count(),
        1usize,
    );
    let expected_hops = if terminal_status.is_some() || invalid {
        0
    } else if case.template == "hops-robots-middle-denied" {
        1
    } else {
        chain.len() - 1
    };
    checks.eq(
        "observed_redirect_hop_count",
        page.redirect_chain.len(),
        expected_hops,
    );
    checks.eq("initial_status_literal", page.status, Some(first_status));
    if !invalid {
        checks.eq(
            "terminal_status_or_stopped_before_target",
            page.final_status,
            expected_status,
        );
    }
    checks.eq("no_body_retention_claim", facts.entity_body_retained, false);
    checks.eq(
        "known_complete_source_only_after_final_200",
        batch.complete,
        complete,
    );
    checks.eq(
        "captured_entity_complete",
        facts.captured_entity_complete,
        expected_status.is_some(),
    );
    checks.eq(
        "captured_entity_hash",
        &facts.captured_entity_sha256,
        expected_status.map(|_| hash(&final_body.body)),
    );
    checks.eq(
        "captured_entity_bytes",
        facts.captured_entity_bytes,
        if expected_status.is_some() {
            final_body.body.len()
        } else {
            0usize
        },
    );
    if expected_status.is_some() {
        let final_url = if terminal_status.is_some() {
            requested_url.clone()
        } else {
            fixtures[end_origin].url(end_path)
        };
        checks.eq("actual_landing_url", &page.final_url, &final_url);
        checks.eq("actual_landing_mime", &page.content_type, &final_body.mime);
        checks.eq(
            "source_document_landing_identity",
            &batch.document_url,
            &final_url,
        );
        checks.eq(
            "final_headers_replace_intermediate_headers",
            page.headers.get("x-own-hop").map(String::as_str),
            Some(if terminal_status.is_some() {
                "terminal-own-status"
            } else {
                "final-own-response"
            }),
        );
        checks.eq(
            "intermediate_nofollow_not_promoted_to_final",
            page.effective_robots.iter().any(|v| v == "nofollow"),
            false,
        );
    }
    for (index, hop) in page.redirect_chain.iter().enumerate() {
        checks.eq(
            &format!("hop_{index}_source_identity"),
            &hop.url,
            fixtures[chain[index].0].url(&chain[index].1),
        );
        checks.eq(
            &format!("hop_{index}_target_literal_resolution"),
            &hop.target,
            &literals[index],
        );
        checks.eq(
            &format!("hop_{index}_status_literal"),
            hop.status,
            if index == 0 {
                first_status
            } else {
                second_status
            },
        );
    }
    if profile == "v1" {
        checks.eq(
            "legacy_no_synthesized_v2_hop_facts",
            facts.redirect_decisions.len(),
            0usize,
        );
    } else {
        let strict_count = if let Boundary::InvalidLocation(missing) = case.boundary {
            usize::from(!missing)
        } else {
            expected_hops
        };
        checks.eq(
            "v2_observed_decision_count",
            facts.redirect_decisions.len(),
            strict_count,
        );
        for (index, hop) in facts.redirect_decisions.iter().enumerate() {
            let scope_denied = (matches!(
                case.template.as_str(),
                "hops-third-denied"
                    | "hops-checkexternal-not-resource-wildcard"
                    | "hops-policy-wide-resource-narrow"
            ) && index == 1)
                || matches!(
                    case.template.as_str(),
                    "hops-scope-port"
                        | "hops-scope-scheme"
                        | "nav-redirect-resource-origin-only"
                        | "nav-allowlisted-backlink-current-behavior"
                );
            let policy_denied = case.template == "hops-policy-deny" && index == 1;
            let robots_denied = (case.template == "hops-robots-middle-denied" && index == 0)
                || (case.template == "hops-robots-final-denied" && index == 1);
            let loop_denied = case.template == "hops-loop" && index == 1;
            let limit_denied = matches!(case.boundary, Boundary::Limit(1)) && index == 1;
            let request_started = !invalid && gets.get(index + 1) == Some(&1);
            let scope_allowed = !scope_denied && !invalid;
            let reason = if invalid {
                "redirect_invalid"
            } else if loop_denied {
                "redirect_stopped_loop"
            } else if limit_denied {
                "redirect_stopped_limit"
            } else if policy_denied {
                "redirect_policy_denied"
            } else if scope_denied {
                "redirect_role_scope_denied"
            } else if robots_denied {
                "redirect_robots_denied"
            } else {
                "redirect_allowed"
            };
            checks.eq(
                &format!("v2_hop_{index}_immutable_original_role"),
                &hop.original_reference_kinds,
                vec![reference_kind(&case.category)],
            );
            checks.eq(
                &format!("v2_hop_{index}_source"),
                &hop.source_url,
                fixtures[chain[index].0].url(&chain[index].1),
            );
            checks.eq(
                &format!("v2_hop_{index}_raw_location"),
                &hop.raw_location,
                if invalid {
                    invalid_literal
                } else {
                    &literals[index]
                },
            );
            checks.eq(
                &format!("v2_hop_{index}_target"),
                &hop.target_url,
                (!invalid).then(|| literals[index].clone()),
            );
            checks.eq(
                &format!("v2_hop_{index}_same_original_authority"),
                &hop.authorizing_occurrence_ids,
                if scope_allowed {
                    vec![grant.occurrence_id.clone()]
                } else {
                    vec![]
                },
            );
            checks.eq(
                &format!("v2_hop_{index}_role_scope_ceiling"),
                hop.scope_allowed,
                scope_allowed,
            );
            checks.eq(
                &format!("v2_hop_{index}_policy_check"),
                hop.policy_check,
                case.policy().then_some(!policy_denied),
            );
            checks.eq(
                &format!("v2_hop_{index}_policy_traverse"),
                hop.policy_traverse,
                case.policy()
                    .then_some(!(policy_denied || capped && index == 0)),
            );
            checks.eq(
                &format!("v2_hop_{index}_robots_before_request"),
                hop.robots_allowed,
                if robots_denied {
                    Some(false)
                } else {
                    request_started.then_some(true)
                },
            );
            checks.eq(
                &format!("v2_hop_{index}_allowed"),
                hop.allowed,
                request_started,
            );
            checks.eq(
                &format!("v2_hop_{index}_request_started"),
                hop.request_started,
                request_started,
            );
            checks.eq(&format!("v2_hop_{index}_stop_reason"), &hop.reason, reason);
        }
    }
    if case.template == "hops-loop" {
        checks.eq(
            "loop_error_literal",
            page.error.as_deref(),
            Some("Redirect loop detected"),
        );
    }
    if matches!(case.boundary, Boundary::Limit(1)) {
        checks.eq(
            "limit_error_literal",
            page.error.as_deref(),
            Some("Redirect limit (1) exceeded"),
        );
    }
    if invalid {
        checks.eq(
            "invalid_location_error_literal",
            page.error.as_deref(),
            Some("Redirect has no valid HTTP(S) Location"),
        );
    }
    checks.eq(
        "robots_block_state",
        page.blocked_by_robots,
        matches!(
            case.template.as_str(),
            "hops-robots-middle-denied" | "hops-robots-final-denied"
        ),
    );
    if case.policy() {
        let policy = page
            .policy
            .as_ref()
            .context("Native policy provenance missing")?;
        checks.eq(
            "policy_literal_redirect_observation_count",
            policy.redirect_decisions.len(),
            expected_hops,
        );
        if capped {
            checks.eq(
                "nontraversable_hop_ceiling_never_resurrected",
                policy.effective_can_expand,
                false,
            );
            checks.eq(
                "checkonce_hop_observed_check_only",
                policy.redirect_decisions[0].decision.decision,
                CrawlPolicyDecisionKind::CheckOnly,
            );
            checks.eq(
                "checkonce_followed_by_traversable_return",
                policy.redirect_decisions[1].decision.traverse,
                true,
            );
        }
        if case.template == "hops-policy-deny" {
            checks.eq(
                "policy_deny_literal",
                &policy.redirect_decisions[1].admission_reason,
                "redirect_denied",
            );
        }
    }
    if complete && case.category == "css" {
        checks.eq(
            "css_complete_decode",
            facts.decode_status,
            ContentCaptureDecodeStatus::Exact,
        );
        checks.eq(
            "css_owned_dependency_count",
            batch.occurrences.len(),
            2usize,
        );
        checks.eq(
            "css_relative_dependency_final_origin",
            batch
                .occurrences
                .iter()
                .map(|o| o.resolved_url.clone())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                Some(fixtures[end_origin].url(&css_child_path)),
                Some(fixtures[end_origin].url(&image_child_path)),
            ]),
        );
    }
    if complete && case.category == "pdf" {
        let pdf = page
            .pdf
            .as_ref()
            .context("Actual PDF parser metadata missing")?;
        checks.eq(
            "pdf_complete_parser",
            pdf.status,
            ResourceAnalysisStatus::Complete,
        );
        checks.eq("pdf_actual_page_count", pdf.page_count, Some(1usize));
        checks.eq("pdf_annotations_complete", pdf.annotations_complete, true);
        checks.eq(
            "pdf_owned_annotation_observed",
            batch
                .occurrences
                .iter()
                .filter(|o| {
                    o.reference_kind == ResourceReferenceKind::PdfAnnotation
                        && o.resolved_url.as_deref() == Some(&fixtures[0].url(&nav_child_path))
                })
                .count(),
            1usize,
        );
    }
    if complete && case.category == "images" && !final_html {
        let image = page
            .image_metadata
            .as_ref()
            .context("Actual PNG metadata missing")?;
        checks.eq("png_complete_parser", image.status, "complete");
        checks.eq("png_width", image.width, Some(1.0f64));
        checks.eq("png_height", image.height, Some(1.0f64));
    }
    if complete && final_html {
        checks.eq(
            "html_response_does_not_upgrade_role",
            grant.can_expand_navigation,
            case.nav(),
        );
        checks.eq(
            "html_owned_children_observed",
            batch.occurrences.len(),
            2usize,
        );
    }
    if complete && (capped || case.template == "image-redirect-html-no-upgrade") {
        let target_rows = rows
            .iter()
            .filter(|r| r.occurrence.requested_source_url == requested_url)
            .collect::<Vec<_>>();
        checks.eq(
            "owned_descendant_inventory_count_by_actual_mime",
            target_rows.len(),
            if case.category == "css" || final_html {
                2usize
            } else if case.category == "pdf" {
                1usize
            } else {
                0usize
            },
        );
        checks.eq(
            "nontraversable_original_role_descendants_denied",
            target_rows
                .iter()
                .all(|r| r.decision.reason == "source_check_only" && r.decision.grant.is_none()),
            true,
        );
    }
    checks.eq(
        "all_occurrences_keep_revision_identity",
        rows.iter().all(|r| {
            r.occurrence.occurrence_id
                == hash(format!(
                    "resource:{}:{}",
                    r.occurrence.source_revision, r.occurrence.source_ordinal
                ))
        }),
        true,
    );
    checks.eq(
        "checkpoint_integrity",
        CrawlStore::inspect(&path)?.checkpoint_valid,
        true,
    );
    let ledger = store
        .resource_ledger_summary()?
        .context("Resource ledger summary missing")?;
    checks.eq(
        "ledger_completeness_tracks_actual_source",
        ledger.complete,
        complete,
    );
    checks.eq(
        "ledger_http_tasks_not_redirect_hop_inflation",
        ledger.completed_transports,
        3usize + 2 * css_children + pdf_children + 2 * nav_children,
    );
    checks.eq(
        "original_task_retained_response",
        transport.retained_response,
        true,
    );
    checks.eq(
        "requested_identity_present_in_report",
        crawl.report.pages.iter().any(|p| p.url == requested_url),
        true,
    );
    checks.eq(
        "body_projection_text_unavailable",
        page.content_captures.is_none(),
        true,
    );
    let actual_logs = fixtures
        .iter()
        .enumerate()
        .map(|(i, f)| (format!("origin_{i}"), f.log()))
        .collect::<BTreeMap<_, _>>();
    let compact_transports=transports.iter().map(|t|json!({"requested_url":t.requested_url,"retained_response":t.retained_response,"status":t.projection.status,"final_status":t.projection.final_status,"final_url":t.projection.final_url,"actual_mime":t.projection.content_type,"response_facts":t.projection.resource_response,"original_transport_permission":t.projection.resource_permission,"source_facts":t.projection.resource_source.as_ref().map(|s|json!({"source_kind":s.source_kind,"source_revision":s.source_revision,"complete":s.complete,"requested_url":s.requested_url,"document_url":s.document_url,"occurrence_count":s.occurrences.len()}))})).collect::<Vec<_>>();
    Ok(
        json!({"id":case.id,"template_id":case.template,"category":case.category,"profile":profile,"boundary":case.boundary,"planned_definition":case.definition,"status":if checks.passed(){"PASS"}else{"FAIL"},"oracle":if profile=="v1"{"preserved_legacy_v1"}else{"desired_original_role_v2"},"legacy_exception_expected":case.legacy_exception(profile),"config":cfg,"source_fixture":{"url":source_url,"mime":source.mime,"bytes":source.body.len(),"sha256":hash(&source.body)},"requested_url":requested_url,"authored_final_response":{"mime":final_body.mime,"status":final_body.status,"bytes":final_body.body.len(),"sha256":hash(&final_body.body)},"expected_chain_gets":gets,"expected_http":expected,"request_log":actual_logs,"original_role_occurrences":parent_rows,"occurrences":rows,"transports":compact_transports,"requested_transport_facts":{"final_status":page.final_status,"final_url":page.final_url,"actual_mime":page.content_type,"error":page.error,"blocked_by_robots":page.blocked_by_robots,"redirect_chain":page.redirect_chain,"response_facts":facts,"original_transport_permission":permission,"source_batch":batch,"policy":page.policy},"ledger":ledger,"checks":checks.rows,"full_vendor_parity":"NOT_PROVEN"}),
    )
}

async fn run_case(case: &Case, profile: &str) -> Value {
    let fixtures = [
        Fixture::new().await,
        Fixture::new().await,
        Fixture::new().await,
    ];
    let temp = tempfile::tempdir().unwrap();
    let result = execute(case, profile, &fixtures, temp.path()).await;
    let mut result = match result {
        Ok(value) => value,
        Err(error) => {
            json!({"id":case.id,"template_id":case.template,"category":case.category,"profile":profile,"boundary":case.boundary,"status":"ERROR","error":format!("{error:#}"),"request_log":fixtures.iter().enumerate().map(|(i,f)|(format!("origin_{i}"),f.log())).collect::<BTreeMap<_,_>>()})
        }
    };
    if result["status"] != "PASS" {
        result["retained_failed_fixture_directory"] = json!(temp.keep());
    }
    result
}

#[tokio::test]
async fn all_88_frozen_redirect_chains_and_explicit_boundaries_in_both_profiles() {
    let plan: Value = serde_json::from_str(include_str!(
        "../../../docs/parity/resource-matrix-plan.json"
    ))
    .unwrap();
    let templates = plan["redirect_controls"]["templates"].as_array().unwrap();
    let navigation = plan["redirect_controls"]["navigation_role_controls"]
        .as_array()
        .unwrap();
    assert_eq!(templates.len(), 12);
    assert_eq!(navigation.len(), 4);
    assert_eq!(
        navigation
            .iter()
            .map(|d| d["id"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "nav-redirect-resource-origin-only",
            "nav-redirect-with-explicit-nav-scope",
            "image-redirect-html-no-upgrade",
            "nav-allowlisted-backlink-current-behavior"
        ])
    );
    assert_eq!(
        plan["family_definitions"]["resources"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(FAMILIES)
    );
    let mapped_templates = BTreeSet::from([
        "hops-two-allowlisted",
        "hops-third-denied",
        "hops-checkexternal-not-resource-wildcard",
        "hops-policy-wide-resource-narrow",
        "hops-robots-middle-denied",
        "hops-robots-final-denied",
        "hops-return-seed",
        "hops-policy-deny",
        "hops-policy-checkonce",
        "hops-scope-port",
        "hops-scope-scheme",
        "hops-loop",
    ]);
    assert_eq!(
        templates
            .iter()
            .map(|d| d["id"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        mapped_templates
    );
    let mut cases = templates
        .iter()
        .flat_map(|d| {
            FAMILIES.map(|family| Case {
                id: format!("{}/{family}", d["id"].as_str().unwrap()),
                template: d["id"].as_str().unwrap().into(),
                category: family.into(),
                definition: d.clone(),
                boundary: Boundary::Planned,
            })
        })
        .collect::<Vec<_>>();
    for definition in navigation {
        let id = definition["id"].as_str().unwrap();
        cases.push(Case {
            id: id.into(),
            template: id.into(),
            category: if id == "image-redirect-html-no-upgrade" {
                "images"
            } else {
                "internal_anchors"
            }
            .into(),
            definition: definition.clone(),
            boundary: Boundary::Planned,
        });
    }
    assert_eq!(cases.len(), 88);
    assert_eq!(
        cases.iter().map(|c| &c.id).collect::<BTreeSet<_>>().len(),
        88
    );
    let boundary_definition = templates
        .iter()
        .find(|d| d["id"] == "hops-two-allowlisted")
        .unwrap();
    for code in 301..=308 {
        cases.push(Case {
            id: format!("boundary-status-{code}"),
            template: "hops-two-allowlisted".into(),
            category: "images".into(),
            definition: boundary_definition.clone(),
            boundary: Boundary::Status(code),
        });
    }
    for limit in [1, 2] {
        cases.push(Case {
            id: format!("boundary-limit-{limit}"),
            template: "hops-two-allowlisted".into(),
            category: "images".into(),
            definition: boundary_definition.clone(),
            boundary: Boundary::Limit(limit),
        });
    }
    for missing in [false, true] {
        cases.push(Case {
            id: if missing {
                "boundary-missing-location"
            } else {
                "boundary-invalid-non-http-location"
            }
            .into(),
            template: "hops-two-allowlisted".into(),
            category: "images".into(),
            definition: boundary_definition.clone(),
            boundary: Boundary::InvalidLocation(missing),
        });
    }
    let started = chrono::Utc::now();
    let mut results = Vec::new();
    for case in &cases {
        for profile in ["v1", "v2"] {
            let result = run_case(case, profile).await;
            if result["status"] != "PASS" {
                eprintln!(
                    "Redirect failure {} {profile}: {}",
                    case.id,
                    json!({"error":result["error"],"failed_checks":result["checks"].as_array().map(|rows|rows.iter().filter(|r|r["pass"]!=true).collect::<Vec<_>>()),"retained_failed_fixture_directory":result["retained_failed_fixture_directory"]})
                );
            }
            results.push(result);
        }
    }
    let failed = results.iter().filter(|r| r["status"] != "PASS").count();
    let report = json!({"engine":"open-seo-spider/resource-native-redirect-controls-v1","status":if failed==0{"PASS"}else{"FAIL"},"started_at":started.to_rfc3339(),"finished_at":chrono::Utc::now().to_rfc3339(),"unique_planned_chains":88,"planned_profile_executions":176,"additional_unique_boundaries":12,"additional_boundary_profile_executions":24,"native_executions":results.len(),"passed_executions":results.len()-failed,"failed_executions":failed,"profiles":["v1","v2"],"source_plan_sha256":hash(include_bytes!("../../../docs/parity/resource-matrix-plan.json")),"source_test_sha256":hash(include_bytes!("resource_redirect_fixtures.rs")),"compiled_source_sha256":{"resource_policy.rs":hash(include_bytes!("../src/resource_policy.rs")),"resource_html.rs":hash(include_bytes!("../src/resource_html.rs")),"resource_capture.rs":hash(include_bytes!("../src/resource_capture.rs")),"resource_ledger.rs":hash(include_bytes!("../src/resource_ledger.rs")),"crawler.rs":hash(include_bytes!("../src/crawler.rs")),"store.rs":hash(include_bytes!("../src/store.rs")),"metadata.rs":hash(include_bytes!("../src/metadata.rs"))},"scope":"Frozen 12 templates x 7 authored resource families plus four exact navigation controls; fresh SQLite and three owned origins per cell/profile; single source seed, full HTTP including robots and credential-origin controls; literal per-hop status/Location/role/authority plus actual final MIME/hash/parser metadata and source-relative expansion. V1 exceptions remain explicit legacy expectations. Limit, invalid/missing Location and status301..308 are additional boundaries rather than extra planned chains.","unproven_boundaries":["A trusted owned HTTPS positive fixture is not executed; the planned scheme mismatch uses an actual owned HTTP target with only its HTTPS origin allowlisted.","No SWF, audio or JavaScript execution; no browser or proprietary implementation parity."],"full_vendor_parity":"NOT_PROVEN","results":results});
    if let Some(path) = std::env::var_os("SPIDER_REDIRECT_RESULTS") {
        let path = std::path::PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    assert_eq!(results.len(), 200);
    assert_eq!(
        failed, 0,
        "Not every exact planned redirect or boundary execution passed"
    );
}
