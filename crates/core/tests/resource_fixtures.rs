//! Original binary fixtures exercise network capture, discovery and persistence.
use lopdf::{Document, Stream, dictionary, text_string};
use open_seo_spider_core::{CrawlConfig, CrawlStore, Crawler, ResourceAnalysisStatus};
use std::{
    collections::HashMap,
    io::Write,
    sync::{Arc, Mutex, atomic::AtomicBool},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

type Response = (u16, Vec<(String, String)>, Vec<u8>);

struct BinaryFixture {
    origin: String,
    counts: Arc<Mutex<HashMap<String, usize>>>,
    task: JoinHandle<()>,
}

impl BinaryFixture {
    async fn new(handler: impl Fn(&str, &str) -> Response + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server_origin = origin.clone();
        let handler = Arc::new(handler);
        let counts = Arc::new(Mutex::new(HashMap::new()));
        let seen = counts.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let handler = handler.clone();
                let origin = server_origin.clone();
                let counts = seen.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut chunk = [0; 4096];
                    while request.len() < 65536 && !request.windows(4).any(|v| v == b"\r\n\r\n") {
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
                        request.extend_from_slice(&chunk[..size]);
                    }
                    let request = String::from_utf8_lossy(&request);
                    let path = request.split_whitespace().nth(1).unwrap_or("/");
                    *counts.lock().unwrap().entry(path.into()).or_insert(0) += 1;
                    let (status, headers, body) = handler(&origin, path);
                    let mut head = format!(
                        "HTTP/1.1 {status} Fixture\r\nConnection: close\r\nContent-Length: {}\r\n",
                        body.len()
                    );
                    for (name, value) in headers {
                        head.push_str(&format!("{name}: {value}\r\n"));
                    }
                    head.push_str("\r\n");
                    let _ = stream.write_all(head.as_bytes()).await;
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

impl Drop for BinaryFixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn config(seeds: Vec<String>) -> CrawlConfig {
    CrawlConfig {
        seed_urls: seeds,
        discover_sitemaps: false,
        delay_ms: 0,
        concurrency: 4,
        obey_robots: false,
        ..Default::default()
    }
}
fn cancel() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}
fn html(body: &str) -> Response {
    (
        200,
        vec![("Content-Type".into(), "text/html".into())],
        body.as_bytes().to_vec(),
    )
}

fn original_pdf(base: &str) -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let pages = doc.new_object_id();
    let font = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding" });
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf (Original PDF network fixture) Tj ET".to_vec(),
    ));
    let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()], "Contents" => content, "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } }, "Annots" => vec![
        dictionary! { "Subtype" => "Link", "A" => dictionary! { "S" => "URI", "URI" => text_string("../contact") } }.into(),
        dictionary! { "Subtype" => "Link", "A" => dictionary! { "S" => "URI", "URI" => text_string("mailto:test@example.org") } }.into(),
    ] });
    doc.set_object(
        pages,
        dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages, "URI" => dictionary! { "Base" => text_string(&format!("{base}/manuals/")) } });
    doc.trailer.set("Root", catalog);
    let info = doc.add_object(dictionary! { "Title" => text_string("Grüße network fixture"), "Author" => text_string("Open SEO Spider tests") });
    doc.trailer.set("Info", info);
    doc.compress();
    let mut body = Vec::new();
    doc.save_to(&mut body).unwrap();
    body
}

fn original_png(width: u32, height: u32) -> Vec<u8> {
    fn chunk(target: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        target.extend_from_slice(&(data.len() as u32).to_be_bytes());
        target.extend_from_slice(kind);
        target.extend_from_slice(data);
        let mut crc = u32::MAX;
        for byte in kind.iter().chain(data) {
            crc ^= *byte as u32;
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb88320u32.wrapping_mul(crc & 1));
            }
        }
        target.extend_from_slice(&(!crc).to_be_bytes());
    }
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &ihdr);
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    for _ in 0..height {
        encoder.write_all(&[0]).unwrap();
        encoder.write_all(&vec![0; width as usize * 4]).unwrap();
    }
    chunk(&mut png, b"IDAT", &encoder.finish().unwrap());
    chunk(&mut png, b"IEND", &[]);
    png
}

#[tokio::test]
async fn binary_pdf_image_and_annotation_discovery_survive_sqlite_and_json_roundtrip() {
    let fixture = BinaryFixture::new(|origin, path| match path {
        "/" => html("<html><head><title>Original resource fixture</title></head><body><h1>Resources</h1><a href='/manual.pdf'>Manual</a><img src='/image.png' width='999' height='888' alt='Declared markup size'></body></html>"),
        "/manual.pdf" => (200, vec![("Content-Type".into(), "application/pdf".into()), ("Link".into(), format!("<{origin}/manual.pdf>; rel=canonical")), ("Set-Cookie".into(), "sid=private_fixture_value; HttpOnly; Secure; Path=/".into())], original_pdf(origin)),
        "/image.png" => (200, vec![("Content-Type".into(), "image/png".into())], original_png(2, 3)),
        "/contact" => html("<html><title>PDF discovered contact</title><h1>Contact</h1></html>"),
        _ => (404, vec![], Vec::new()),
    }).await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("resources.sqlite");
    let report = Crawler::new(config(vec![fixture.url("/")]))
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    assert_eq!(report.pages.len(), 4);
    assert_eq!(fixture.count("/contact"), 1);
    let document = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/manual.pdf"))
        .unwrap();
    let pdf = document.pdf.as_ref().unwrap();
    assert_eq!(pdf.status, ResourceAnalysisStatus::Complete, "{pdf:#?}");
    assert_eq!(pdf.page_count, Some(1));
    assert_eq!(pdf.word_count, Some(4));
    assert_eq!(pdf.info_metadata["Title"], "Grüße network fixture");
    assert_eq!(pdf.annotation_links[0].url, Some(fixture.url("/contact")));
    assert!(pdf.annotation_links[1].url.is_none());
    assert_eq!(document.canonical, vec![fixture.url("/manual.pdf")]);
    assert!(document.indexable);
    assert_eq!(
        document.headers["set-cookie"],
        "sid=[redacted]; HttpOnly; Secure; Path=/"
    );
    assert!(
        !document
            .warnings
            .iter()
            .any(|warning| warning.contains("invalid bytes"))
    );
    let image = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/image.png"))
        .unwrap();
    let metadata = image.image_metadata.as_ref().unwrap();
    assert_eq!(metadata.status, ResourceAnalysisStatus::Complete);
    assert_eq!((metadata.width, metadata.height), (Some(2.0), Some(3.0)));
    assert_eq!(metadata.format.as_deref(), Some("png"));
    let occurrence = &report
        .pages
        .iter()
        .find(|page| page.url == fixture.url("/"))
        .unwrap()
        .images[0];
    assert_eq!(occurrence.width.as_deref(), Some("999"));
    assert!(metadata.body_complete);
    let store = CrawlStore::open(&path).unwrap();
    let saved = store.load_report().unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(&saved).unwrap(),
        serde_json::to_value(&report).unwrap()
    );
    assert_eq!(
        store.config().unwrap().unwrap().pdf_analysis.max_pages,
        1000
    );
    assert_eq!(store.pages().unwrap().len(), 4);
    let json = serde_json::to_string(&saved).unwrap();
    assert!(!json.contains("private_fixture_value"));
    assert!(json.contains("[redacted]"));
}

#[tokio::test]
async fn truncated_and_invalid_binary_documents_keep_unknown_counts_and_dimensions() {
    let fixture = BinaryFixture::new(|origin, path| match path {
        "/truncated.pdf" => (
            200,
            vec![("Content-Type".into(), "application/pdf".into())],
            original_pdf(origin),
        ),
        "/truncated.png" => (
            200,
            vec![("Content-Type".into(), "image/png".into())],
            original_png(2, 3),
        ),
        "/broken.pdf" => (
            200,
            vec![("Content-Type".into(), "application/pdf".into())],
            b"%PDF-1.7 broken".to_vec(),
        ),
        _ => (
            200,
            vec![("Content-Type".into(), "image/png".into())],
            b"not an image".to_vec(),
        ),
    })
    .await;
    let mut options = config(vec![
        fixture.url("/truncated.pdf"),
        fixture.url("/truncated.png"),
        fixture.url("/broken.pdf"),
        fixture.url("/broken.png"),
    ]);
    options.crawl_links = false;
    options.max_body_bytes = 40;
    let report = Crawler::new(options)
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    let truncated_pdf = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/truncated.pdf"))
        .unwrap();
    assert!(truncated_pdf.truncated);
    assert_eq!(
        truncated_pdf.pdf.as_ref().unwrap().status,
        ResourceAnalysisStatus::Partial
    );
    assert!(truncated_pdf.pdf.as_ref().unwrap().text_chars.is_none());
    assert!(!truncated_pdf.indexable);
    let image = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/truncated.png"))
        .unwrap()
        .image_metadata
        .as_ref()
        .unwrap();
    assert_eq!(image.status, ResourceAnalysisStatus::Partial);
    assert!(image.width.is_none() && image.height.is_none());
    assert!(!image.body_complete);
    let broken_pdf = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/broken.pdf"))
        .unwrap()
        .pdf
        .as_ref()
        .unwrap();
    assert_eq!(broken_pdf.status, ResourceAnalysisStatus::Error);
    assert!(broken_pdf.page_count.is_none() && broken_pdf.parse_error.is_some());
    let broken_image = report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/broken.png"))
        .unwrap()
        .image_metadata
        .as_ref()
        .unwrap();
    assert_eq!(broken_image.status, ResourceAnalysisStatus::Error);
    assert!(broken_image.width.is_none() && broken_image.parse_error.is_some());
}

#[tokio::test]
async fn whitespace_robot_directives_and_multiple_cookie_headers_use_native_capture_policy() {
    let fixture = BinaryFixture::new(|_, _| (200, vec![
        ("Content-Type".into(), "text/html".into()),
        ("X-Robots-Tag".into(), "bingbot:max-snippet:50 nofollow, googlebot:max-image-preview:large noindex".into()),
        ("Set-Cookie".into(), "one=\"private;quoted\"; HttpOnly; Path=/".into()),
        ("Set-Cookie".into(), "two=another_private_value; SameSite=Strict; Expires=Wed, 03 Dec 2099 13:00:00 GMT".into()),
    ], b"<html><title>Robots fixture</title><meta name='robots' content='max-video-preview: 10 noarchive'><h1>Robots</h1></html>".to_vec())).await;
    let report = Crawler::new(config(vec![fixture.url("/")]))
        .unwrap()
        .run(None, cancel())
        .await
        .unwrap();
    let page = &report.pages[0];
    assert!(page.effective_robots.contains(&"noindex".into()));
    assert!(page.effective_robots.contains(&"noarchive".into()));
    assert!(!page.effective_robots.contains(&"nofollow".into()));
    assert!(!page.indexable);
    assert!(page.headers["set-cookie"].contains("one=[redacted]; HttpOnly; Path=/"));
    assert!(
        page.headers["set-cookie"]
            .contains("two=[redacted]; SameSite=Strict; Expires=Wed, 03 Dec 2099 13:00:00 GMT")
    );
    let json = serde_json::to_string(page).unwrap();
    assert!(!json.contains("private;quoted"));
    assert!(!json.contains("another_private_value"));
}

#[tokio::test]
async fn invalid_pdf_configuration_fails_before_any_website_request_and_legacy_models_default() {
    let fixture = BinaryFixture::new(|_, _| html("Should never be requested")).await;
    let mut options = config(vec![fixture.url("/")]);
    options.pdf_analysis.max_pages = 0;
    assert!(Crawler::new(options).is_err());
    assert_eq!(fixture.count("/"), 0);
    let page: open_seo_spider_core::PageRecord = serde_json::from_str("{}").unwrap();
    assert!(page.pdf.is_none() && page.image_metadata.is_none());
    let legacy: CrawlConfig = serde_json::from_str("{}").unwrap();
    assert!(legacy.pdf_analysis.enabled);
}
