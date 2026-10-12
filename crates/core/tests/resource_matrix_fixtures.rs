//! Complete, independently authored native switch matrix. Each cell/profile owns
//! fresh origins, unique declarations, a SQLite checkpoint and actual request log.
//! The 108 planned cells run under both V1 and V2; this is not vendor parity.
use anyhow::{Context, Result};
use lopdf::{Document, Stream, dictionary, text_string};
use open_seo_spider_core::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const RESOURCES: [&str; 7] = [
    "css",
    "javascript",
    "images",
    "media",
    "pdf",
    "swf",
    "other",
];
const NAVIGATION: [&str; 10] = [
    "internal_anchors",
    "external_anchors",
    "iframes",
    "refresh",
    "canonical",
    "hreflang",
    "pagination",
    "amp",
    "mobile_alternate",
    "pdf_annotations",
];

#[derive(Clone)]
struct Response {
    mime: String,
    body: Vec<u8>,
}
impl Response {
    fn new(mime: &str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            mime: mime.into(),
            body: body.into(),
        }
    }
    fn html(body: String) -> Self {
        Self::new("text/html; charset=utf-8", body)
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
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let routes = task_routes.clone();
                let requests = task_requests.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let mut buffer = [0; 4096];
                    while bytes.len() < 65536 && !bytes.windows(4).any(|v| v == b"\r\n\r\n") {
                        let Ok(Ok(size)) =
                            tokio::time::timeout(Duration::from_secs(3), stream.read(&mut buffer))
                                .await
                        else {
                            return;
                        };
                        if size == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buffer[..size]);
                    }
                    let request = String::from_utf8_lossy(&bytes);
                    let mut parts = request.lines().next().unwrap_or("").split_whitespace();
                    let method = parts.next().unwrap_or("");
                    let path = parts.next().unwrap_or("/");
                    let headers = request
                        .lines()
                        .skip(1)
                        .filter_map(|line| line.split_once(':'))
                        .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
                        .collect::<BTreeMap<_, _>>();
                    requests
                        .lock()
                        .unwrap()
                        .push(json!({"method":method,"path":path,"headers":headers}));
                    let response = routes.lock().unwrap().get(path).cloned();
                    let (status, response) = response.map(|r| (200, r)).unwrap_or_else(|| {
                        (
                            404,
                            Response::new("text/plain", b"Unexpected matrix request".to_vec()),
                        )
                    });
                    let head = format!(
                        "HTTP/1.1 {status} Matrix\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        response.mime,
                        response.body.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&response.body).await;
                    let _ = stream.shutdown().await;
                });
            }
        });
        let fixture = Self {
            origin,
            routes,
            requests,
            task,
        };
        fixture.put(
            "/robots.txt",
            Response::new(
                "text/plain; charset=utf-8",
                b"User-agent: *\nAllow: /\n".to_vec(),
            ),
        );
        fixture
    }
    fn put(&self, path: &str, response: Response) {
        self.routes.lock().unwrap().insert(path.into(), response);
    }
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin)
    }
    fn log(&self) -> Vec<Value> {
        self.requests.lock().unwrap().clone()
    }
    fn counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for request in self.log() {
            *counts
                .entry(request["path"].as_str().unwrap().into())
                .or_default() += 1;
        }
        counts
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn resource_switch(cfg: &mut ResourceDiscoveryConfig, kind: &str, switch: ResourceSwitch) {
    match kind {
        "css" => cfg.resources.css = switch,
        "javascript" => cfg.resources.javascript = switch,
        "images" => cfg.resources.images = switch,
        "media" => cfg.resources.media = switch,
        "pdf" => cfg.resources.pdf = switch,
        "swf" => cfg.resources.swf = switch,
        "other" => cfg.resources.other = switch,
        _ => unreachable!(),
    }
}
fn navigation_switch(cfg: &mut ResourceDiscoveryConfig, kind: &str, switch: NavigationSwitch) {
    match kind {
        "internal_anchors" => cfg.navigation.internal_anchors = switch,
        "external_anchors" => cfg.navigation.external_anchors = switch,
        "iframes" => cfg.navigation.iframes = switch,
        "refresh" => cfg.navigation.refresh = switch,
        "canonical" => cfg.navigation.canonical = switch,
        "hreflang" => cfg.navigation.hreflang = switch,
        "pagination" => cfg.navigation.pagination = switch,
        "amp" => cfg.navigation.amp = switch,
        "mobile_alternate" => cfg.navigation.mobile_alternate = switch,
        "pdf_annotations" => cfg.navigation.pdf_annotations = switch,
        _ => unreachable!(),
    }
}
fn reference_kind(kind: &str) -> ResourceReferenceKind {
    match kind {
        "css" => ResourceReferenceKind::Css,
        "javascript" => ResourceReferenceKind::JavaScript,
        "images" => ResourceReferenceKind::Image,
        "media" => ResourceReferenceKind::Media,
        "pdf" => ResourceReferenceKind::Pdf,
        "swf" => ResourceReferenceKind::Swf,
        "other" => ResourceReferenceKind::Other,
        "internal_anchors" => ResourceReferenceKind::InternalAnchor,
        "external_anchors" => ResourceReferenceKind::ExternalAnchor,
        "iframes" => ResourceReferenceKind::Iframe,
        "refresh" => ResourceReferenceKind::Refresh,
        "canonical" => ResourceReferenceKind::Canonical,
        "hreflang" => ResourceReferenceKind::Hreflang,
        "pagination" => ResourceReferenceKind::Pagination,
        "amp" => ResourceReferenceKind::Amp,
        "mobile_alternate" => ResourceReferenceKind::MobileAlternate,
        "pdf_annotations" => ResourceReferenceKind::PdfAnnotation,
        _ => unreachable!(),
    }
}
fn declaration(kind: &str, url: &str) -> String {
    match kind {
        "css" => format!("<link rel='stylesheet' href='{url}'>"),
        "javascript" => format!("<script src='{url}'></script>"),
        "images" => format!("<img src='{url}' alt='Own matrix'>"),
        "media" => format!("<audio src='{url}'></audio>"),
        "pdf" => format!("<object type='application/pdf' data='{url}'></object>"),
        "swf" => format!("<embed type='application/x-shockwave-flash' src='{url}'>"),
        "other" => format!("<track src='{url}' kind='captions' srclang='en'>"),
        "internal_anchors" | "external_anchors" => format!("<a href='{url}'>Own link</a>"),
        "iframes" => format!("<iframe src='{url}'></iframe>"),
        "refresh" => format!("<meta http-equiv='refresh' content='0;url={url}'>"),
        "canonical" => format!("<link rel='canonical' href='{url}'>"),
        "hreflang" => format!("<link rel='alternate' hreflang='en' href='{url}'>"),
        "pagination" => format!("<link rel='next' href='{url}'>"),
        "amp" => format!("<link rel='amphtml' href='{url}'>"),
        "mobile_alternate" => format!(
            "<link rel='alternate' media='only screen and (max-width: 640px)' href='{url}'>"
        ),
        _ => unreachable!(),
    }
}
fn pdf(links: &[String]) -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let pages = doc.new_object_id();
    let font = doc.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica","Encoding"=>"WinAnsiEncoding"});
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf (Own complete matrix PDF) Tj ET".to_vec(),
    ));
    let annotations = links.iter().map(|url| dictionary! {"Subtype"=>"Link","Rect"=>vec![0.into(),0.into(),10.into(),10.into()],"A"=>dictionary! {"S"=>"URI","URI"=>text_string(url)}}.into()).collect::<Vec<lopdf::Object>>();
    let page = doc.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()],"Contents"=>content,"Resources"=>dictionary! {"Font"=>dictionary! {"F1"=>font}},"Annots"=>annotations});
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
fn authored_response(kind: &str, css_child: &str, image_child: &str, pdf_child: &str) -> Response {
    match kind {
        "css" => Response::new(
            "text/css; charset=utf-8",
            format!("@import '{css_child}';body{{background:url('{image_child}')}}"),
        ),
        "javascript" => Response::new(
            "text/javascript; charset=utf-8",
            b"/* Own script fixture: declaration only, no execution. */".to_vec(),
        ),
        "images" => Response::new("image/png", png()),
        "media" => Response::new("audio/wav", wav()),
        "pdf" => Response::new("application/pdf", pdf(&[pdf_child.into()])),
        "swf" => Response::new(
            "application/x-shockwave-flash",
            b"FWS\x09\x08\x00\x00\x00".to_vec(),
        ),
        "other" => Response::new(
            "text/vtt; charset=utf-8",
            b"WEBVTT\n\n00:00.000 --> 00:00.500\nOwn matrix text\n".to_vec(),
        ),
        _ => unreachable!(),
    }
}

#[derive(Default)]
struct Checks {
    rows: Vec<Value>,
}
impl Checks {
    fn eq(&mut self, key: &str, actual: impl serde::Serialize, expected: impl serde::Serialize) {
        let actual = serde_json::to_value(actual).unwrap();
        let expected = serde_json::to_value(expected).unwrap();
        self.rows
            .push(json!({"key":key,"pass":actual==expected,"actual":actual,"expected":expected}));
    }
    fn passed(&self) -> bool {
        self.rows.iter().all(|row| row["pass"] == true)
    }
}

#[derive(Clone, Copy)]
struct Cell {
    kind: &'static str,
    navigation: bool,
    store: bool,
    fetch: bool,
    expand: bool,
}
impl Cell {
    fn id(self) -> String {
        if self.navigation {
            format!(
                "navigation-{}-s{}f{}e{}",
                self.kind,
                u8::from(self.store),
                u8::from(self.fetch),
                u8::from(self.expand)
            )
        } else {
            format!(
                "resource-{}-s{}f{}",
                self.kind,
                u8::from(self.store),
                u8::from(self.fetch)
            )
        }
    }
    fn valid(self) -> bool {
        !self.navigation || !self.expand || self.fetch
    }
}
fn config(cell: Cell, profile: &str, source: &str, negative_path: &str) -> Result<CrawlConfig> {
    let mut discovery = ResourceDiscoveryConfig::default();
    for kind in RESOURCES {
        resource_switch(
            &mut discovery,
            kind,
            ResourceSwitch {
                store: false,
                fetch: false,
            },
        );
    }
    for kind in NAVIGATION {
        navigation_switch(
            &mut discovery,
            kind,
            NavigationSwitch {
                store: false,
                fetch: false,
                expand: false,
            },
        );
    }
    if cell.navigation {
        // Actual children have independent enabled switches; only their parent
        // expansion grant can reach them. No child refers back to the target.
        resource_switch(
            &mut discovery,
            "images",
            ResourceSwitch {
                store: true,
                fetch: true,
            },
        );
        resource_switch(
            &mut discovery,
            "media",
            ResourceSwitch {
                store: true,
                fetch: true,
            },
        );
        navigation_switch(
            &mut discovery,
            "internal_anchors",
            NavigationSwitch {
                store: true,
                fetch: true,
                expand: false,
            },
        );
        navigation_switch(
            &mut discovery,
            cell.kind,
            NavigationSwitch {
                store: cell.store,
                fetch: cell.fetch,
                expand: cell.expand,
            },
        );
    } else {
        navigation_switch(
            &mut discovery,
            "internal_anchors",
            NavigationSwitch {
                store: true,
                fetch: true,
                expand: false,
            },
        );
        if cell.kind == "css" {
            resource_switch(
                &mut discovery,
                "images",
                ResourceSwitch {
                    store: true,
                    fetch: true,
                },
            );
        }
        if cell.kind == "pdf" {
            navigation_switch(
                &mut discovery,
                "pdf_annotations",
                NavigationSwitch {
                    store: true,
                    fetch: true,
                    expand: false,
                },
            );
        }
        resource_switch(
            &mut discovery,
            cell.kind,
            ResourceSwitch {
                store: cell.store,
                fetch: cell.fetch,
            },
        );
    }
    let cfg = CrawlConfig {
        seed_urls: vec![source.into()],
        discover_sitemaps: false,
        crawl_links: true,
        include_subdomains: false,
        concurrency: 1,
        request_timeout_secs: 5,
        max_pages: Some(32),
        max_depth: Some(4),
        delay_ms: 0,
        obey_robots: true,
        check_external: cell.kind == "external_anchors",
        max_body_bytes: 1024 * 1024,
        exclude_patterns: vec![format!("{}$", negative_path)],
        resource_discovery: Some(discovery),
        resource_discovery_limits: Some(ResourceDiscoveryLimits {
            max_source_candidates: 64,
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut value = serde_json::to_value(cfg)?;
    if profile == "v2" {
        value["resource_discovery"]["contract"] = json!({"engine":"open-seo-spider/resource-contract-v2","follow_internal_qualified_links":false,"follow_external_qualified_links":false});
    }
    serde_json::from_value(value)
        .context("Matrix profile must exist in the production typed config")
}

async fn run_cell(cell: Cell, profile: &str) -> Value {
    let fixture = Fixture::new().await;
    let external = Fixture::new().await;
    let directory = tempfile::tempdir().unwrap();
    let mut result = run_cell_inner(cell, profile, &fixture, &external, directory.path())
        .await
        .unwrap_or_else(|error| json!({"id":cell.id(),"profile":profile,"status":"ERROR","config_valid":cell.valid(),"error":format!("{error:#}"),"request_log":{"source_origin":fixture.log(),"secondary_origin":external.log()}}));
    if result["status"] != "PASS" {
        result["retained_failed_fixture_directory"] = json!(directory.keep());
    }
    result
}

async fn run_cell_inner(
    cell: Cell,
    profile: &str,
    fixture: &Fixture,
    external: &Fixture,
    directory: &std::path::Path,
) -> Result<Value> {
    let id = cell.id();
    let prefix = format!("/{id}-{profile}");
    let source_path = format!("{prefix}/source");
    let target_path = format!("{prefix}/target");
    let css_probe_path = format!("{prefix}/css-probe");
    let control_path = format!("{prefix}/positive-control");
    let negative_path = format!("{prefix}/negative-control");
    let css_child_path = format!("{prefix}/css-child");
    let image_child_path = format!("{prefix}/image-child");
    let nav_child_path = format!("{prefix}/nav-child");
    let forbidden_import_path = format!("{prefix}/forbidden-css-import");
    let forbidden_image_path = format!("{prefix}/forbidden-css-image");
    let target_server = if cell.kind == "external_anchors" {
        external
    } else {
        fixture
    };
    let source_url = fixture.url(&source_path);
    let target_url = target_server.url(&target_path);
    let css_probe_url = target_server.url(&css_probe_path);
    let cfg = config(cell, profile, &source_url, &negative_path)?;
    let database_path = directory.join("matrix.sqlite");
    let mut checks = Checks::default();
    let valid = cell.valid();
    let created = Crawler::new(cfg.clone());
    if !valid {
        let error = created.err().map(|error| error.to_string());
        checks.eq("configuration_rejected", error.is_some(), true);
        checks.eq(
            "specific_validation_reason",
            error
                .as_deref()
                .is_some_and(|error| error.contains("Navigation expand requires fetch")),
            true,
        );
        checks.eq(
            "all_original_origin_http_including_robots_source",
            fixture.log().len(),
            0usize,
        );
        checks.eq("all_secondary_origin_http", external.log().len(), 0usize);
        checks.eq("checkpoint_never_created", database_path.exists(), false);
        return Ok(
            json!({"id":id,"profile":profile,"status":if checks.passed(){"PASS"}else{"FAIL"},"config_valid":false,"validation_error":error,"config":cfg,"request_log":{"source_origin":fixture.log(),"secondary_origin":external.log()},"checks":checks.rows}),
        );
    }
    let crawler = created?;
    let effective_config = crawler.effective_config().clone();
    // Sources are exactly one initial input. Their unique targets are never
    // referenced by controls, descendants, another source or another seed.
    let source = if cell.kind == "pdf_annotations" {
        Response::new(
            "application/pdf",
            pdf(&[
                target_url.clone(),
                css_probe_url.clone(),
                fixture.url(&negative_path),
            ]),
        )
    } else {
        let mut body = declaration(cell.kind, &target_url);
        if cell.navigation {
            body.push_str(&declaration(cell.kind, &css_probe_url));
        }
        body.push_str(&declaration(
            if cell.navigation {
                "media"
            } else {
                "internal_anchors"
            },
            &fixture.url(&control_path),
        ));
        body.push_str(&declaration(
            if cell.navigation {
                cell.kind
            } else {
                "canonical"
            },
            &fixture.url(&negative_path),
        ));
        Response::html(body)
    };
    fixture.put(&source_path, source.clone());
    fixture.put(
        &control_path,
        if cell.navigation {
            Response::new("audio/wav", wav())
        } else {
            Response::html("<title>Own independent positive control</title>".into())
        },
    );
    fixture.put(&image_child_path, Response::new("image/png", png()));
    fixture.put(
        &css_child_path,
        Response::new(
            "text/css; charset=utf-8",
            b"/* Own leaf stylesheet. */".to_vec(),
        ),
    );
    fixture.put(
        &nav_child_path,
        Response::html("<title>Own leaf navigation</title>".into()),
    );
    fixture.put(
        &negative_path,
        Response::html("<title>This target must never be requested</title>".into()),
    );
    fixture.put(
        &forbidden_import_path,
        Response::new("text/css; charset=utf-8", b"/* forbidden */".to_vec()),
    );
    fixture.put(&forbidden_image_path, Response::new("image/png", png()));
    let target_response = if cell.navigation {
        Response::html(format!(
            "<title>Own matrix target</title><a href='{}'>Own child</a><img src='{}' alt='Own child'>",
            fixture.url(&nav_child_path),
            fixture.url(&image_child_path)
        ))
    } else {
        authored_response(
            cell.kind,
            &fixture.url(&css_child_path),
            &fixture.url(&image_child_path),
            &fixture.url(&nav_child_path),
        )
    };
    target_server.put(&target_path, target_response.clone());
    if cell.navigation {
        target_server.put(
            &css_probe_path,
            Response::new(
                "text/css; charset=utf-8",
                format!(
                    "@import '{}';body{{background:url('{}')}}",
                    fixture.url(&forbidden_import_path),
                    fixture.url(&forbidden_image_path)
                ),
            ),
        );
    }
    let crawl = tokio::time::timeout(
        Duration::from_secs(20),
        crawler
            .with_store(&database_path)?
            .run_controlled(None, CrawlControl::new()),
    )
    .await
    .context("Matrix native crawl timed out")??;
    let store = CrawlStore::open_read_only(&database_path)?;
    let mut occurrences = Vec::new();
    store.visit_resource_occurrences(2, |page| {
        occurrences.extend(page.rows.clone());
        Ok(())
    })?;
    let mut second_read = Vec::new();
    store.visit_resource_occurrences(3, |page| {
        second_read.extend(page.rows.clone());
        Ok(())
    })?;
    checks.eq(
        "immutable_decisions_survive_independent_verified_read",
        serde_json::to_value(&occurrences)?,
        serde_json::to_value(second_read)?,
    );
    let mut transports = Vec::new();
    let mut cursor = None;
    loop {
        let page = store.resource_transports(cursor.as_deref(), 2)?;
        let more = page.has_more;
        cursor = page.next_url;
        transports.extend(page.rows);
        if !more {
            break;
        }
    }
    let primary = occurrences
        .iter()
        .filter(|row| {
            row.occurrence.requested_source_url == source_url
                && row.occurrence.resolved_url.as_deref() == Some(&target_url)
        })
        .collect::<Vec<_>>();
    checks.eq(
        "one_unique_primary_source_occurrence",
        primary.len(),
        1usize,
    );
    if let Some(row) = primary.first() {
        checks.eq(
            "declared_reference_role",
            row.occurrence.reference_kind,
            reference_kind(cell.kind),
        );
        checks.eq(
            "complete_actual_source",
            row.occurrence.source_complete,
            true,
        );
        checks.eq(
            "immutable_occurrence_id",
            &row.occurrence.occurrence_id,
            format!(
                "{:x}",
                Sha256::digest(format!(
                    "resource:{}:{}",
                    row.occurrence.source_revision, row.occurrence.source_ordinal
                ))
            ),
        );
        let decision_value = serde_json::to_value(&row.decision)?;
        checks.eq(
            "versioned_qualification_provenance",
            decision_value.get("qualification").is_some(),
            profile == "v2",
        );
        if profile == "v2" {
            checks.eq(
                "qualification_engine",
                &decision_value["qualification"]["engine"],
                "open-seo-spider/resource-contract-v2",
            );
            checks.eq(
                "qualification_target_scope",
                &decision_value["qualification"]["scope"],
                if cell.kind == "external_anchors" {
                    "external"
                } else {
                    "internal"
                },
            );
            checks.eq(
                "qualification_without_rel_tokens",
                &decision_value["qualification"]["rel_qualifiers"],
                json!([]),
            );
            checks.eq(
                "qualification_without_document_nofollow",
                &decision_value["qualification"]["document_nofollow"],
                false,
            );
            checks.eq(
                "unqualified_matrix_target_follow_allowed",
                &decision_value["qualification"]["follow_allowed"],
                true,
            );
        }
        checks.eq(
            "source_generation_matches_checkpoint",
            Some(row.generation),
            crawl
                .checkpoint
                .as_ref()
                .map(|checkpoint| checkpoint.generation),
        );
        checks.eq(
            "grant_present_only_when_fetch_enabled",
            row.decision.grant.is_some(),
            cell.fetch,
        );
        checks.eq(
            "decision_reason",
            &row.decision.reason,
            if cell.fetch {
                "allowed"
            } else if cell.kind == "pdf_annotations" {
                "source_check_only"
            } else {
                "fetch_disabled"
            },
        );
        if let Some(grant) = &row.decision.grant {
            checks.eq("grant_store_independent_of_fetch", grant.store, cell.store);
            checks.eq(
                "grant_bound_to_original_occurrence",
                &grant.occurrence_id,
                &row.occurrence.occurrence_id,
            );
            checks.eq("grant_bound_to_fetch_url", &grant.fetch_url, &target_url);
            checks.eq(
                "versioned_grant_reference_provenance",
                serde_json::to_value(grant)?.get("reference_kind").cloned(),
                if profile == "v2" {
                    Some(serde_json::to_value(reference_kind(cell.kind))?)
                } else {
                    None
                },
            );
            checks.eq(
                "grant_bound_to_typed_resource_config",
                &grant.config_sha256,
                cfg.resource_discovery.as_ref().unwrap().sha256()?,
            );
            checks.eq(
                "grant_navigation_capability",
                grant.can_expand_navigation,
                cell.navigation && cell.expand,
            );
            checks.eq(
                "grant_resource_capability",
                grant.can_expand_resources,
                cell.navigation && cell.expand,
            );
            checks.eq(
                "grant_css_capability",
                grant.can_expand_css,
                !cell.navigation && cell.kind == "css",
            );
            checks.eq(
                "grant_pdf_capability",
                grant.can_expand_pdf,
                cell.kind == "pdf" || (cell.kind == "pdf_annotations" && cell.expand),
            );
        }
    }
    let target_transport = transports
        .iter()
        .find(|row| row.requested_url == target_url);
    checks.eq(
        "private_or_retained_transport_exists_only_after_fetch",
        target_transport.is_some(),
        cell.fetch,
    );
    checks.eq(
        "target_report_retention_independent_from_inventory",
        crawl.report.pages.iter().any(|page| page.url == target_url),
        cell.store && cell.fetch,
    );
    if let Some(transport) = target_transport {
        checks.eq(
            "transport_current_report_inclusion",
            transport.retained_response,
            cell.store,
        );
        checks.eq(
            "actual_response_mime",
            &transport.projection.content_type,
            &target_response.mime,
        );
        checks.eq(
            "actual_response_status",
            transport.projection.final_status,
            Some(200u16),
        );
        checks.eq(
            "actual_body_size",
            transport.projection.size_bytes,
            target_response.body.len(),
        );
        let facts = transport
            .projection
            .resource_response
            .as_ref()
            .context("Observed transport lacks actual entity facts")?;
        checks.eq(
            "actual_entity_sha256",
            facts.captured_entity_sha256.as_deref(),
            Some(format!("{:x}", Sha256::digest(&target_response.body))).as_deref(),
        );
        checks.eq(
            "actual_entity_complete",
            facts.captured_entity_complete,
            true,
        );
        checks.eq(
            "projection_never_retains_entity_body",
            facts.entity_body_retained,
            false,
        );
        checks.eq(
            "permission_immutable_primary_grant",
            &transport
                .projection
                .resource_permission
                .as_ref()
                .context("Observed transport lacks durable task permission")?
                .authorizing_grants,
            primary
                .first()
                .context("Observed target has no source occurrence")?
                .decision
                .grant
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
        );
        if cell.kind == "pdf" {
            checks.eq(
                "valid_pdf_analysis",
                transport.projection.pdf.as_ref().map(|p| &p.status),
                Some(&ResourceAnalysisStatus::Complete),
            );
        }
        if cell.kind == "images" {
            checks.eq(
                "generated_png_dimensions",
                transport
                    .projection
                    .image_metadata
                    .as_ref()
                    .map(|p| (p.width, p.height)),
                Some((Some(1.0f64), Some(1.0f64))),
            );
        }
        if !cell.store {
            checks.eq(
                "private_projection_retention_provenance",
                facts.retained_response,
                false,
            );
            checks.eq(
                "private_projection_omits_custom_and_browser_data",
                transport.projection.custom_extractions.is_empty()
                    && transport.projection.screenshot_base64.is_none()
                    && transport.projection.network_requests.is_empty()
                    && transport.projection.browser_cookies.is_empty(),
                true,
            );
            if let Some(pdf) = &transport.projection.pdf {
                checks.eq(
                    "private_pdf_text_absence",
                    pdf.text_preview.is_none() && pdf.text_pages.is_empty() && !pdf.text_complete,
                    true,
                );
            }
        }
    }
    let mut expected_source = BTreeMap::from([
        ("/robots.txt".to_owned(), 1usize),
        (source_path.clone(), 1usize),
    ]);
    let mut expected_secondary = BTreeMap::new();
    if cell.kind != "pdf_annotations" {
        expected_source.insert(control_path, 1);
    }
    if cell.fetch {
        let requests = if cell.kind == "external_anchors" {
            &mut expected_secondary
        } else {
            &mut expected_source
        };
        requests.insert(target_path, 1);
        if cell.navigation {
            requests.insert(css_probe_path, 1);
        }
        if cell.kind == "external_anchors" {
            requests.insert("/robots.txt".into(), 1);
        }
        if (cell.navigation && cell.expand) || cell.kind == "pdf" {
            expected_source.insert(nav_child_path.clone(), 1);
        }
        if (cell.navigation && cell.expand) || cell.kind == "css" {
            expected_source.insert(image_child_path.clone(), 1);
        }
        if cell.kind == "css" {
            expected_source.insert(css_child_path, 1);
        }
    }
    checks.eq(
        "exact_original_origin_http_including_robots_source_controls_descendants",
        fixture.counts(),
        expected_source,
    );
    checks.eq(
        "exact_secondary_origin_http_including_robots",
        external.counts(),
        expected_secondary,
    );
    checks.eq(
        "no_requests_from_css_mime_navigation_role",
        fixture
            .counts()
            .get(&forbidden_import_path)
            .copied()
            .unwrap_or(0)
            + fixture
                .counts()
                .get(&forbidden_image_path)
                .copied()
                .unwrap_or(0),
        0usize,
    );
    checks.eq(
        "negative_control_never_fetched",
        fixture.counts().get(&negative_path).copied().unwrap_or(0),
        0usize,
    );
    checks.eq(
        "all_actual_requests_are_get",
        fixture
            .log()
            .iter()
            .chain(external.log().iter())
            .all(|row| row["method"] == "GET"),
        true,
    );
    checks.eq(
        "checkpoint_integrity_valid",
        CrawlStore::inspect(&database_path)?.checkpoint_valid,
        true,
    );
    checks.eq(
        "ledger_complete",
        crawl
            .report
            .resource_ledger
            .as_ref()
            .map(|ledger| ledger.complete),
        Some(true),
    );
    checks.eq(
        "only_requested_transports_have_facts",
        transports.len(),
        fixture
            .counts()
            .iter()
            .chain(external.counts().iter())
            .filter(|(path, _)| path.as_str() != "/robots.txt")
            .map(|(_, count)| count)
            .sum::<usize>(),
    );
    Ok(
        json!({"id":id,"profile":profile,"status":if checks.passed(){"PASS"}else{"FAIL"},"config_valid":true,"config":effective_config,"own_v1_expected":true,"initial_inputs":[source_url],"source_fixture":{"mime":source.mime,"entity_sha256":format!("{:x}",Sha256::digest(&source.body)),"bytes":source.body.len()},"target_url":target_url,"target_authored_fixture":{"mime":target_response.mime,"entity_sha256":format!("{:x}",Sha256::digest(&target_response.body)),"bytes":target_response.body.len()},"request_log":{"source_origin":fixture.log(),"secondary_origin":external.log()},"occurrences":occurrences,"transports":transports,"retained_report_urls":crawl.report.pages.iter().map(|page|&page.url).collect::<Vec<_>>(),"ledger":crawl.report.resource_ledger,"checks":checks.rows}),
    )
}

#[tokio::test]
async fn all_108_native_resource_and_navigation_cells_in_both_versioned_profiles() {
    let mut cells = Vec::new();
    for kind in RESOURCES {
        for store in [false, true] {
            for fetch in [false, true] {
                cells.push(Cell {
                    kind,
                    navigation: false,
                    store,
                    fetch,
                    expand: false,
                });
            }
        }
    }
    for kind in NAVIGATION {
        for store in [false, true] {
            for fetch in [false, true] {
                for expand in [false, true] {
                    cells.push(Cell {
                        kind,
                        navigation: true,
                        store,
                        fetch,
                        expand,
                    });
                }
            }
        }
    }
    assert_eq!(cells.len(), 108);
    let plan: Value = serde_json::from_str(include_str!(
        "../../../docs/parity/resource-matrix-plan.json"
    ))
    .unwrap();
    let declared_ids = plan["resource_matrix"]
        .as_array()
        .unwrap()
        .iter()
        .chain(plan["navigation_matrix"].as_array().unwrap())
        .map(|row| row["id"].as_str().unwrap().to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    let executed_ids = cells
        .iter()
        .map(|cell| cell.id())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        executed_ids, declared_ids,
        "Native matrix must cover every named plan cell exactly"
    );
    let started = chrono::Utc::now();
    let mut results = Vec::new();
    for cell in &cells {
        for profile in ["v1", "v2"] {
            let result = run_cell(*cell, profile).await;
            if result["status"] != "PASS" {
                let failed_checks = result["checks"].as_array().map(|checks| {
                    checks
                        .iter()
                        .filter(|check| check["pass"] != true)
                        .collect::<Vec<_>>()
                });
                eprintln!(
                    "{} {} {}",
                    cell.id(),
                    profile,
                    json!({"status":result["status"],"failed_checks":failed_checks,"error":result["error"],"retained_failed_fixture_directory":result["retained_failed_fixture_directory"]})
                );
            }
            results.push(result);
        }
    }
    let failures = results
        .iter()
        .filter(|row| row["status"] != "PASS")
        .map(|row| {
            format!(
                "{}:{}",
                row["id"].as_str().unwrap(),
                row["profile"].as_str().unwrap()
            )
        })
        .collect::<Vec<_>>();
    let report = json!({
        "engine":"open-seo-spider/resource-native-matrix-v1",
        "started_at":started.to_rfc3339(),"finished_at":chrono::Utc::now().to_rfc3339(),
        "status":if failures.is_empty(){"PASS"}else{"FAIL"},
        "unique_planned_cells":cells.len(),"profiles":["v1","v2"],"profile_executions":results.len(),
        "valid_unique_cells":cells.iter().filter(|cell|cell.valid()).count(),
        "invalid_unique_pre_http_cells":cells.iter().filter(|cell|!cell.valid()).count(),
        "passed_profile_executions":results.iter().filter(|row|row["status"]=="PASS").count(),
        "failed_profile_executions":failures,
        "scope":"Own V1 expected switch semantics under both versioned profiles; complete sources, one immutable initial input per cell, fresh SQLite, exact HTTP logs including robots, retained/private metadata and immutable grants. No renderer, proprietary source, license change, or vendor parity claim.",
        "full_vendor_parity":"NOT_PROVEN",
        "source_test_sha256":format!("{:x}",Sha256::digest(include_bytes!("resource_matrix_fixtures.rs"))),
        "source_plan_sha256":format!("{:x}",Sha256::digest(include_bytes!("../../../docs/parity/resource-matrix-plan.json"))),
        "compiled_source_sha256":{
            "resource_policy.rs":format!("{:x}",Sha256::digest(include_bytes!("../src/resource_policy.rs"))),
            "resource_html.rs":format!("{:x}",Sha256::digest(include_bytes!("../src/resource_html.rs"))),
            "resource_capture.rs":format!("{:x}",Sha256::digest(include_bytes!("../src/resource_capture.rs"))),
            "resource_ledger.rs":format!("{:x}",Sha256::digest(include_bytes!("../src/resource_ledger.rs"))),
            "crawler.rs":format!("{:x}",Sha256::digest(include_bytes!("../src/crawler.rs"))),
            "store.rs":format!("{:x}",Sha256::digest(include_bytes!("../src/store.rs"))),
            "metadata.rs":format!("{:x}",Sha256::digest(include_bytes!("../src/metadata.rs")))
        },
        "results":results
    });
    if let Some(path) = std::env::var_os("SPIDER_MATRIX_RESULTS") {
        let path = std::path::PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    assert!(
        failures.is_empty(),
        "Native matrix failed for {} of216 profile executions: {}",
        failures.len(),
        failures.join(", ")
    );
}
