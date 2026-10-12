//! Every named native MIME/decoding/response control from the frozen own plan.
//! Binary fixtures are authored here. Unknown parser/source states stay unknown.
use anyhow::{Context, Result};
use lopdf::{
    Document, EncryptionState, EncryptionVersion, Permissions, Stream, dictionary, text_string,
};
use open_seo_spider_core::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Semaphore,
};

#[derive(Clone)]
struct Response {
    status: u16,
    mime: String,
    body: Vec<u8>,
    gate: Option<Arc<Semaphore>>,
}
impl Response {
    fn new(mime: &str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            mime: mime.into(),
            body: body.into(),
            gate: None,
        }
    }
    fn html(body: impl Into<Vec<u8>>) -> Self {
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
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let routes = task_routes.clone();
                let requests = task_requests.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let mut buffer = [0; 4096];
                    while bytes.len() < 65536 && !bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                        let Ok(Ok(size)) =
                            tokio::time::timeout(Duration::from_secs(3), socket.read(&mut buffer))
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
                    let mut line = request.lines().next().unwrap_or("").split_whitespace();
                    let method = line.next().unwrap_or("");
                    let path = line.next().unwrap_or("/");
                    requests
                        .lock()
                        .unwrap()
                        .push(json!({"method":method,"path":path}));
                    let response = routes
                        .lock()
                        .unwrap()
                        .get(path)
                        .cloned()
                        .unwrap_or_else(|| Response {
                            status: 404,
                            ..Response::new("text/plain", b"Unexpected fixture request".to_vec())
                        });
                    if let Some(gate) = &response.gate {
                        gate.acquire().await.unwrap().forget();
                    }
                    // Missing MIME means no Content-Type field, not an empty field.
                    let mime = if response.mime.is_empty() {
                        String::new()
                    } else {
                        format!("Content-Type: {}\r\n", response.mime)
                    };
                    let head = format!(
                        "HTTP/1.1 {} OwnResponse\r\n{mime}Content-Length: {}\r\nConnection: close\r\n\r\n",
                        response.status,
                        response.body.len()
                    );
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(&response.body).await;
                    let _ = socket.shutdown().await;
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
            Response::new("text/plain", b"User-agent: *\nAllow: /\n".to_vec()),
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
        for row in self.log() {
            *counts
                .entry(row["path"].as_str().unwrap().into())
                .or_default() += 1;
        }
        counts
    }
    async fn started(&self, path: &str) -> Result<()> {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if self.counts().contains_key(path) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .context("Gated source never started")?;
        Ok(())
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
fn pdf(links: &[String], encrypted: bool) -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let pages = doc.new_object_id();
    let font=doc.add_object(dictionary!{"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica","Encoding"=>"WinAnsiEncoding"});
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf (Own response fixture PDF text) Tj ET".to_vec(),
    ));
    let annotations=links.iter().map(|url|dictionary!{"Subtype"=>"Link","Rect"=>vec![0.into(),0.into(),10.into(),10.into()],"A"=>dictionary!{"S"=>"URI","URI"=>text_string(url)}}.into()).collect::<Vec<lopdf::Object>>();
    let page=doc.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()],"Contents"=>content,"Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}},"Annots"=>annotations});
    doc.set_object(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1},
    );
    let catalog = doc.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    doc.trailer.set("Root", catalog);
    if encrypted {
        doc.trailer.set(
            "ID",
            vec![
                text_string("OwnedFixture0001"),
                text_string("OwnedFixture0001"),
            ],
        );
        let state = EncryptionState::try_from(EncryptionVersion::V2 {
            document: &doc,
            owner_password: "own-fixture-owner",
            user_password: "own-fixture-user",
            key_length: 128,
            permissions: Permissions::all(),
        })
        .unwrap();
        doc.encrypt(&state).unwrap();
    }
    let mut body = Vec::new();
    doc.save_to(&mut body).unwrap();
    body
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
        "other" => format!("<style>@font-face{{font-family:Own;src:url('{url}')}}</style>"),
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
fn config(
    source: &str,
    profile: &str,
    body_cap: bool,
    pdf_annotations: bool,
) -> Result<CrawlConfig> {
    let mut resources = ResourceDiscoveryConfig::default();
    resources.navigation.pdf_annotations = NavigationSwitch {
        store: true,
        fetch: pdf_annotations,
        expand: false,
    };
    let cfg = CrawlConfig {
        seed_urls: vec![source.into()],
        discover_sitemaps: false,
        obey_robots: true,
        concurrency: 1,
        delay_ms: 0,
        request_timeout_secs: 5,
        max_pages: Some(32),
        max_depth: Some(5),
        max_body_bytes: if body_cap { 512 } else { 1024 * 1024 },
        resource_discovery: Some(resources),
        ..Default::default()
    };
    let mut value = serde_json::to_value(cfg)?;
    if profile == "v2" {
        value["resource_discovery"]["contract"] = json!({"engine":"open-seo-spider/resource-contract-v2","follow_internal_qualified_links":false,"follow_external_qualified_links":false});
    }
    Ok(serde_json::from_value(value)?)
}
fn response(id: &str, mime: &str, css: &str, image: &str, nav: &str) -> Response {
    let css_body = format!("@import '{css}';body{{background:url('{image}')}}");
    let html_body = format!(
        "<title>Own MIME mismatch</title><a href='{nav}'>Own child</a><img src='{image}'><style>@import '{css}';</style>"
    );
    match id {
        "mime-css-actual-css"
        | "mime-css-missing"
        | "mime-css-misleading"
        | "mime-css-mixed-case" => Response::new(mime, css_body),
        "mime-css-actual-html"
        | "mime-js-actual-html"
        | "mime-image-actual-html"
        | "mime-media-actual-html" => Response::new(mime, html_body),
        "mime-css-lossy" => {
            let mut body = format!("@import '{css}/").into_bytes();
            body.extend_from_slice(b"\xff.css';");
            Response::new(mime, body)
        }
        "mime-css-nonascii-fallback" => Response::new(mime, format!("@import '{css}/naïve.css';")),
        "mime-css-non200" => Response {
            status: 404,
            ..Response::new(mime, css_body)
        },
        "mime-css-body-cap" => Response::new(
            mime,
            format!("{css_body}/*{}*/", "own padding ".repeat(400)),
        ),
        "mime-image-valid-png" => Response::new(mime, png()),
        "mime-media-valid-wav" => Response::new(mime, wav()),
        "mime-pdf-complete" | "mime-pdf-body-cap" => Response::new(mime, pdf(&[nav.into()], false)),
        "mime-pdf-encrypted" => Response::new(mime, pdf(&[nav.into()], true)),
        "mime-pdf-malformed" => {
            Response::new(mime, b"%PDF-1.7\nOwned malformed fixture only.".to_vec())
        }
        "mime-swf-transport" => Response::new(mime, b"FWS\x09\x08\x00\x00\x00".to_vec()),
        "mime-other-font" => Response::new(
            mime,
            b"wOF2Own transport bytes; not a conforming font/parser claim".to_vec(),
        ),
        "mime-nav-actual-css" => Response::new(mime, format!("@import '{css}';")),
        _ => unreachable!(),
    }
}

async fn run_case(definition: &Value, profile: &str, annotations: bool) -> Value {
    let fixture = Arc::new(Fixture::new().await);
    let directory = tempfile::tempdir().unwrap();
    let mut result=run_case_inner(definition,profile,annotations,&fixture,directory.path()).await.unwrap_or_else(|error|json!({"id":definition["id"],"profile":profile,"annotations_enabled":annotations,"status":"ERROR","error":format!("{error:#}"),"request_log":fixture.log()}));
    if result["status"] != "PASS" {
        result["retained_failed_fixture_directory"] = json!(directory.keep());
    }
    result
}
async fn run_case_inner(
    definition: &Value,
    profile: &str,
    annotations: bool,
    fixture: &Arc<Fixture>,
    directory: &std::path::Path,
) -> Result<Value> {
    let id = definition["id"].as_str().unwrap();
    let category = definition["category"].as_str().unwrap();
    let prefix = format!("/{id}-{profile}-a{}", u8::from(annotations));
    let source_path = format!("{prefix}/source");
    let target_path = format!("{prefix}/target");
    let css_path = format!("{prefix}/css-child");
    let image_path = format!("{prefix}/image-child");
    let nav_path = format!("{prefix}/nav-child");
    let positive_path = format!("{prefix}/positive-css");
    let positive_child_path = format!("{prefix}/positive-child");
    let late_path = format!("{prefix}/late-role");
    let target_url = fixture.url(&target_path);
    let source_url = fixture.url(&source_path);
    let path = directory.join("response.sqlite");
    let cfg = config(&source_url, profile, id.ends_with("body-cap"), annotations)?;
    let target = response(
        id,
        definition["actual_mime"].as_str().unwrap(),
        &fixture.url(&css_path),
        &fixture.url(&image_path),
        &fixture.url(&nav_path),
    );
    let mut source = declaration(category, &target_path);
    source.push_str(&declaration("css", &positive_path));
    let gate = Arc::new(Semaphore::new(0));
    if id == "mime-nav-actual-css" {
        source.push_str(&declaration("internal_anchors", &late_path));
        fixture.put(
            &late_path,
            Response {
                gate: Some(gate.clone()),
                ..Response::html(declaration("css", &target_path))
            },
        );
    }
    fixture.put(&source_path, Response::html(source));
    fixture.put(&target_path, target.clone());
    fixture.put(
        &css_path,
        Response::new(
            "text/css; charset=utf-8",
            b"/* Own empty CSS child */".to_vec(),
        ),
    );
    fixture.put(&image_path, Response::new("image/png", png()));
    fixture.put(
        &nav_path,
        Response::html(b"<title>Own child</title>".to_vec()),
    );
    fixture.put(
        &positive_path,
        Response::new(
            "text/css; charset=utf-8",
            format!("@import '{}';", fixture.url(&positive_child_path)),
        ),
    );
    fixture.put(
        &positive_child_path,
        Response::new("text/css; charset=utf-8", b"/* Positive child */".to_vec()),
    );
    let crawler = Crawler::new(cfg.clone())?.with_store(&path)?;
    let mut checks = Checks::default();
    let mut before_late = None;
    let crawl = if id == "mime-nav-actual-css" {
        let task =
            tokio::spawn(async move { crawler.run_controlled(None, CrawlControl::new()).await });
        if let Err(error) = fixture.started(&late_path).await {
            task.abort();
            return Err(error);
        }
        checks.eq(
            "before_late_target_fetched_exactly_once",
            fixture.counts().get(&target_path).copied().unwrap_or(0),
            1usize,
        );
        checks.eq(
            "before_late_no_css_child",
            fixture.counts().get(&css_path).copied().unwrap_or(0),
            0usize,
        );
        match CrawlStore::open_read_only(&path)
            .and_then(|store| store.resource_occurrences(None, 1000))
        {
            Ok(live) => {
                checks.eq(
                    "before_late_css_reference_check_only",
                    live.rows
                        .iter()
                        .find(|row| {
                            row.occurrence.requested_source_url == target_url
                                && row.occurrence.resolved_url.as_deref()
                                    == Some(&fixture.url(&css_path))
                        })
                        .map(|row| &row.decision.reason),
                    Some(&"source_check_only".to_owned()),
                );
                before_late = Some(json!({"request_log":fixture.log(),"snapshot":live}));
            }
            Err(error) => {
                checks.eq("before_late_verified_read_available", false, true);
                before_late =
                    Some(json!({"request_log":fixture.log(),"read_error":format!("{error:#}")}));
            }
        }
        gate.add_permits(1);
        tokio::time::timeout(Duration::from_secs(20), task)
            .await
            .context("Late-role response timed out")???
    } else {
        tokio::time::timeout(
            Duration::from_secs(20),
            crawler.run_controlled(None, CrawlControl::new()),
        )
        .await
        .context("Response fixture timed out")??
    };
    let store = CrawlStore::open_read_only(&path)?;
    let mut rows = Vec::new();
    store.visit_resource_occurrences(3, |page| {
        rows.extend(page.rows.clone());
        Ok(())
    })?;
    let transports = store.resource_transports(None, 1000)?.rows;
    let transport = transports
        .iter()
        .find(|row| row.requested_url == target_url)
        .context("Target transport missing")?;
    let page = &transport.projection;
    let facts = page
        .resource_response
        .as_ref()
        .context("Actual response facts missing")?;
    let parent = rows
        .iter()
        .find(|row| {
            row.occurrence.requested_source_url == source_url
                && row.occurrence.resolved_url.as_deref() == Some(&target_url)
        })
        .context("Declared parent occurrence missing")?;
    checks.eq(
        "declared_reference_kind_not_inferred_from_response",
        parent.occurrence.reference_kind,
        reference_kind(category),
    );
    checks.eq(
        "parent_grant_bound_to_own_declaration",
        parent
            .decision
            .grant
            .as_ref()
            .map(|grant| &grant.occurrence_id),
        Some(&parent.occurrence.occurrence_id),
    );
    checks.eq(
        "actual_response_mime",
        &page.content_type,
        definition["actual_mime"].as_str().unwrap(),
    );
    checks.eq(
        "actual_response_status",
        page.final_status,
        Some(target.status),
    );
    let captured = &target.body[..target.body.len().min(cfg.max_body_bytes)];
    checks.eq(
        "actual_entity_prefix_hash",
        facts.captured_entity_sha256.as_deref(),
        Some(format!("{:x}", Sha256::digest(captured))).as_deref(),
    );
    checks.eq(
        "actual_entity_captured_bytes",
        facts.captured_entity_bytes,
        captured.len(),
    );
    checks.eq(
        "actual_response_truncation",
        page.truncated,
        target.body.len() > cfg.max_body_bytes,
    );
    checks.eq(
        "entity_capture_complete_independent_from_parser_success",
        facts.captured_entity_complete,
        target.body.len() <= cfg.max_body_bytes,
    );
    checks.eq(
        "metadata_projection_contains_no_entity_body",
        facts.entity_body_retained,
        false,
    );
    let mut expected = BTreeMap::from([
        ("/robots.txt".into(), 1usize),
        (source_path, 1),
        (target_path.clone(), 1),
        (positive_path, 1),
        (positive_child_path, 1),
    ]);
    if matches!(id, "mime-css-actual-css" | "mime-css-mixed-case") {
        expected.insert(css_path.clone(), 1);
        expected.insert(image_path.clone(), 1);
    }
    if id == "mime-pdf-complete" && annotations {
        expected.insert(nav_path.clone(), 1);
    }
    if id == "mime-nav-actual-css" {
        expected.insert(late_path.clone(), 1);
        expected.insert(css_path.clone(), 1);
    }
    checks.eq(
        "exact_requests_positive_and_negative_controls",
        fixture.counts(),
        expected,
    );
    checks.eq(
        "no_navigation_permission_from_resource_mime",
        parent
            .decision
            .grant
            .as_ref()
            .map(|grant| grant.can_expand_navigation),
        Some(category == "internal_anchors"),
    );
    let batch = page
        .resource_source
        .as_ref()
        .context("Response source observation missing")?;
    match id {
        "mime-css-lossy" => {
            checks.eq(
                "css_decode_status",
                facts.decode_status,
                ContentCaptureDecodeStatus::Lossy,
            );
            checks.eq("css_source_incomplete", batch.complete, false);
            checks.eq(
                "owned_lossy_css_reference_observed",
                batch.occurrences.len(),
                1usize,
            );
            checks.eq(
                "all_css_occurrences_incomplete",
                batch.occurrences.iter().all(|row| !row.source_complete),
                true,
            );
        }
        "mime-css-nonascii-fallback" => {
            checks.eq(
                "css_decode_status",
                facts.decode_status,
                ContentCaptureDecodeStatus::Unknown,
            );
            checks.eq("css_source_incomplete", batch.complete, false);
            checks.eq(
                "owned_unknown_css_reference_observed",
                batch.occurrences.len(),
                1usize,
            );
        }
        "mime-css-non200" | "mime-css-body-cap" => {
            checks.eq("css_source_incomplete", batch.complete, false);
            checks.eq(
                "child_not_admitted_from_incomplete_source",
                fixture.counts().get(&css_path).copied().unwrap_or(0),
                0usize,
            );
        }
        "mime-css-actual-css" | "mime-css-mixed-case" => {
            checks.eq(
                "css_exact_decode",
                facts.decode_status,
                ContentCaptureDecodeStatus::Exact,
            );
            checks.eq("css_source_complete", batch.complete, true);
        }
        "mime-css-missing" | "mime-css-misleading" => {
            checks.eq(
                "no_mime_inferred_from_css_looking_body",
                batch.source_kind,
                ResourceSourceKind::HttpLinkHeader,
            );
            checks.eq(
                "no_fake_css_import_inventory",
                batch.occurrences.len(),
                0usize,
            );
        }
        "mime-css-actual-html"
        | "mime-js-actual-html"
        | "mime-image-actual-html"
        | "mime-media-actual-html" => {
            checks.eq(
                "actual_html_metadata_observed",
                page.title(),
                "Own MIME mismatch",
            );
            checks.eq(
                "all_three_owned_html_descendants_observed",
                batch.occurrences.len(),
                3usize,
            );
            checks.eq(
                "actual_html_does_not_upgrade_original_role",
                parent
                    .decision
                    .grant
                    .as_ref()
                    .map(|grant| grant.can_expand_resources),
                Some(false),
            );
            checks.eq(
                "observed_html_descendants_are_check_only",
                rows.iter()
                    .filter(|row| row.occurrence.requested_source_url == target_url)
                    .all(|row| row.decision.reason == "source_check_only"),
                true,
            );
        }
        "mime-image-valid-png" => {
            checks.eq(
                "png_complete_analysis",
                page.image_metadata.as_ref().map(|image| &image.status),
                Some(&ResourceAnalysisStatus::Complete),
            );
            checks.eq(
                "png_intrinsic_pixels",
                page.image_metadata
                    .as_ref()
                    .map(|image| (image.width, image.height)),
                Some((Some(1.0), Some(1.0))),
            );
        }
        "mime-pdf-complete" => {
            let pdf = page.pdf.as_ref().context("PDF observation missing")?;
            checks.eq(
                "pdf_complete_analysis",
                pdf.status,
                ResourceAnalysisStatus::Complete,
            );
            checks.eq("pdf_page_count", pdf.page_count, Some(1usize));
            checks.eq("pdf_annotations_complete", pdf.annotations_complete, true);
            checks.eq(
                "actual_uri_annotation_observed",
                pdf.annotation_links
                    .iter()
                    .any(|link| link.url.as_deref() == Some(&fixture.url(&nav_path))),
                true,
            );
        }
        "mime-pdf-encrypted" | "mime-pdf-malformed" | "mime-pdf-body-cap" => {
            let pdf = page
                .pdf
                .as_ref()
                .context("Incomplete PDF observation missing")?;
            checks.eq(
                "pdf_analysis_state",
                pdf.status,
                if id == "mime-pdf-encrypted" {
                    ResourceAnalysisStatus::Encrypted
                } else if id == "mime-pdf-malformed" {
                    ResourceAnalysisStatus::Error
                } else {
                    ResourceAnalysisStatus::Partial
                },
            );
            checks.eq("pdf_page_count_unknown", pdf.page_count, None::<usize>);
            checks.eq(
                "pdf_annotations_unknown_not_known_absent",
                pdf.annotations_complete,
                false,
            );
            checks.eq("pdf_text_unknown", pdf.text_complete, false);
            let retained_pdf = crawl
                .report
                .pages
                .iter()
                .find(|page| page.url == target_url)
                .and_then(|page| page.pdf.as_ref())
                .context("Retained parser observation missing")?;
            checks.eq(
                "actual_parser_does_not_claim_complete_text",
                retained_pdf.text_complete,
                false,
            );
            checks.eq(
                "actual_parser_word_count_unknown",
                retained_pdf.word_count,
                None::<usize>,
            );
            checks.eq("pdf_failure_observed", pdf.parse_error.is_some(), true);
            checks.eq("pdf_source_incomplete", batch.complete, false);
            if id == "mime-pdf-encrypted" {
                checks.eq("pdf_encryption_observed", pdf.encrypted, Some(true));
            }
        }
        "mime-media-valid-wav" | "mime-swf-transport" | "mime-other-font" => {
            checks.eq(
                "no_unimplemented_pdf_analysis_claim",
                page.pdf.is_some(),
                false,
            );
            checks.eq(
                "no_unimplemented_image_analysis_claim",
                page.image_metadata.is_some(),
                false,
            );
            checks.eq("metadata_only_no_renderer", page.rendered, false);
        }
        "mime-nav-actual-css" => {
            // The transport preserves the permission actually used for its
            // single HTTP request. The retained report carries later replay
            // permission; these are deliberately different observations.
            let original_permission = page
                .resource_permission
                .as_ref()
                .context("Original transport permission missing")?;
            let permission = crawl
                .report
                .pages
                .iter()
                .find(|page| page.url == target_url)
                .and_then(|page| page.resource_permission.as_ref())
                .context("Final merged report permission missing")?;
            let original_grant = parent
                .decision
                .grant
                .as_ref()
                .context("Original navigation grant missing")?;
            let late_grants = rows
                .iter()
                .filter(|row| {
                    row.occurrence.requested_source_url == fixture.url(&late_path)
                        && row.occurrence.resolved_url.as_deref() == Some(&target_url)
                        && row.occurrence.reference_kind == ResourceReferenceKind::Css
                })
                .filter_map(|row| row.decision.grant.as_ref())
                .collect::<Vec<_>>();
            checks.eq(
                "one_independent_late_ledger_grant",
                late_grants.len(),
                1usize,
            );
            checks.eq(
                "original_transport_grant_snapshot_immutable",
                &original_permission.authorizing_grants,
                vec![original_grant.clone()],
            );
            checks.eq(
                "original_transport_css_permission_immutable",
                original_permission.can_expand_css,
                false,
            );
            checks.eq(
                "initial_navigation_grant_not_mutated",
                parent
                    .decision
                    .grant
                    .as_ref()
                    .map(|grant| grant.can_expand_css),
                Some(false),
            );
            checks.eq(
                "independent_late_css_grant",
                permission
                    .authorizing_grants
                    .iter()
                    .filter(|grant| grant.can_expand_css)
                    .count(),
                1usize,
            );
            checks.eq(
                "two_independent_role_witnesses",
                permission.authorizing_grants.len(),
                2usize,
            );
            checks.eq(
                "final_report_witnesses_exact_ledger_grants",
                &permission.authorizing_grants,
                std::iter::once(original_grant.clone())
                    .chain(late_grants.into_iter().cloned())
                    .collect::<Vec<_>>(),
            );
            let evaluations = rows
                .iter()
                .filter(|row| {
                    row.occurrence.requested_source_url == target_url
                        && row.occurrence.resolved_url.as_deref() == Some(&fixture.url(&css_path))
                })
                .collect::<Vec<_>>();
            checks.eq(
                "original_css_denial_still_retained",
                evaluations.iter().any(|row| {
                    row.decision.reason == "source_check_only" && row.decision.grant.is_none()
                }),
                true,
            );
            checks.eq(
                "late_css_admission_independent",
                evaluations.iter().any(|row| row.decision.grant.is_some()),
                true,
            );
        }
        _ => unreachable!(),
    }
    checks.eq(
        "checkpoint_integrity",
        CrawlStore::inspect(&path)?.checkpoint_valid,
        true,
    );
    checks.eq(
        "report_retains_actual_target",
        crawl.report.pages.iter().any(|page| page.url == target_url),
        true,
    );
    Ok(
        json!({"id":id,"profile":profile,"annotations_enabled":annotations,"status":if checks.passed(){"PASS"}else{"FAIL"},"definition":definition,"config":cfg,"source_url":source_url,"target_url":target_url,"authored_response":{"status":target.status,"mime":target.mime,"bytes":target.body.len(),"sha256":format!("{:x}",Sha256::digest(&target.body))},"before_late_css_role":before_late,"final_target_report_permission":crawl.report.pages.iter().find(|page|page.url==target_url).and_then(|page|page.resource_permission.as_ref()),"request_log":fixture.log(),"occurrences":rows,"transports":transports,"checks":checks.rows}),
    )
}

#[tokio::test]
async fn all_21_frozen_response_controls_have_native_positive_and_negative_evidence() {
    let plan: Value = serde_json::from_str(include_str!(
        "../../../docs/parity/resource-matrix-plan.json"
    ))
    .unwrap();
    let definitions = plan["response_controls"].as_array().unwrap();
    assert_eq!(definitions.len(), 21);
    let named = definitions
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(named.len(), 21);
    let started = chrono::Utc::now();
    let mut results = Vec::new();
    for definition in definitions {
        for profile in ["v1", "v2"] {
            for annotations in if definition["id"] == "mime-pdf-complete" {
                vec![false, true]
            } else {
                vec![true]
            } {
                let result = run_case(definition, profile, annotations).await;
                if result["status"] != "PASS" {
                    let failed = result["checks"].as_array().map(|checks| {
                        checks
                            .iter()
                            .filter(|check| check["pass"] != true)
                            .collect::<Vec<_>>()
                    });
                    eprintln!(
                        "Response failure {} {profile}: {}",
                        definition["id"],
                        json!({"failed_checks":failed,"error":result["error"],"retained_failed_fixture_directory":result["retained_failed_fixture_directory"]})
                    );
                }
                results.push(result);
            }
        }
    }
    let failed = results.iter().filter(|row| row["status"] != "PASS").count();
    let report = json!({"engine":"open-seo-spider/resource-response-controls-v1","status":if failed==0{"PASS"}else{"FAIL"},"started_at":started.to_rfc3339(),"finished_at":chrono::Utc::now().to_rfc3339(),"unique_planned_controls":21,"profiles":["v1","v2"],"native_executions":results.len(),"failed_executions":failed,"passed_executions":results.len()-failed,"full_vendor_parity":"NOT_PROVEN","scope":"Own declared role versus actual MIME, decode and parser state; actual HTTP with independent allowed CSS control; exact truncated prefix hashes; complete/encrypted/malformed/truncated authored PDFs; original navigation CSS denial followed by an independent late stylesheet grant. No codecs, font/SWF execution, browser or vendor-parity claim.","source_test_sha256":format!("{:x}",Sha256::digest(include_bytes!("resource_response_fixtures.rs"))),"source_plan_sha256":format!("{:x}",Sha256::digest(include_bytes!("../../../docs/parity/resource-matrix-plan.json"))),"results":results});
    if let Some(path) = std::env::var_os("SPIDER_RESPONSE_RESULTS") {
        let path = std::path::PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    assert_eq!(failed, 0, "Not every named native response control passed");
}
