//! AUTHORED / NOT COMPILED / NOT RUN under coordinated compute hold.
//! BinaryFixture infrastructure reused verbatim from frozen226 sitemap_protocol_tests.rs:4-216 (atomic import omitted).
//! Tests use actual loopback HTTP, private wrappers, helper/scanner and Store snapshots.
use super::*;
use flate2::{Compression, write::GzEncoder};
use std::{collections::BTreeMap, io::Write};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::{JoinHandle, JoinSet},
};

struct BinaryResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    declared_length: Option<usize>,
}
impl BinaryResponse {
    fn bytes(body: Vec<u8>, content_type: &str) -> Self {
        Self {
            status: 200,
            headers: vec![("Content-Type".into(), content_type.into())],
            body,
            declared_length: None,
        }
    }
    fn robots(body: Vec<u8>) -> Self {
        Self::bytes(body, "text/plain")
    }
    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers
            .retain(|(key, _)| !key.eq_ignore_ascii_case(name));
        self.headers.push((name.into(), value.into()));
        self
    }
    fn redirect(target: &str) -> Self {
        let mut value = Self::bytes(Vec::new(), "application/octet-stream");
        value.status = 302;
        value.header("Location", target)
    }
}
#[derive(Debug)]
struct RecordedRequest {
    path: String,
    headers: BTreeMap<String, String>,
}
#[derive(Debug, Default)]
struct FinalRequests {
    requests: Vec<RecordedRequest>,
    errors: Vec<String>,
}
impl FinalRequests {
    fn counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for request in &self.requests {
            *counts.entry(request.path.clone()).or_default() += 1;
        }
        counts
    }
}
type Handler = Arc<dyn Fn(&str, &str) -> BinaryResponse + Send + Sync>;
struct BinaryFixture {
    origin: String,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<FinalRequests>>,
}
impl Drop for BinaryFixture {
    fn drop(&mut self) {
        // Panic fallback requests shutdown too. Normal tests call finish() and
        // await the owner AND every connection task before inspecting counters.
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
fn collect_finished(
    result: Result<Result<RecordedRequest, String>, tokio::task::JoinError>,
    final_requests: &mut FinalRequests,
) {
    match result {
        Ok(Ok(request)) => final_requests.requests.push(request),
        Ok(Err(error)) => final_requests.errors.push(error),
        Err(error) => final_requests
            .errors
            .push(format!("fixture connection task: {error}")),
    }
}
async fn respond(
    mut socket: TcpStream,
    origin: String,
    handler: Handler,
) -> Result<RecordedRequest, String> {
    let head = tokio::time::timeout(Duration::from_secs(2), async {
        let mut head = Vec::new();
        let mut chunk = [0u8; 2048];
        loop {
            let count = socket
                .read(&mut chunk)
                .await
                .map_err(|error| error.to_string())?;
            if count == 0 {
                return Err("fixture request ended before header terminator".to_owned());
            }
            head.extend_from_slice(&chunk[..count]);
            if head.len() > 65536 {
                return Err("fixture request headers exceed 64 KiB".to_owned());
            }
            if head.windows(4).any(|part| part == b"\r\n\r\n") {
                return Ok(head);
            }
        }
    })
    .await
    .map_err(|_| "fixture request header timeout".to_owned())??;
    let head = String::from_utf8(head).map_err(|error| error.to_string())?;
    let path = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or("fixture request path missing")?
        .to_owned();
    let mut headers = BTreeMap::new();
    for line in head.lines().skip(1).take_while(|line| !line.is_empty()) {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    let response = handler(&origin, &path);
    let headers_text = response
        .headers
        .iter()
        .map(|(name, value)| format!("{name}: {value}\r\n"))
        .collect::<String>();
    let length = response.declared_length.unwrap_or(response.body.len());
    let head = format!(
        "HTTP/1.1 {} Fixture\r\n{headers_text}Content-Length: {length}\r\nConnection: close\r\n\r\n",
        response.status
    );
    // A deliberately bounded client may close during the body. Its GET still
    // counts; response write completion is not an assertion about client reads.
    let _ = socket.write_all(head.as_bytes()).await;
    let _ = socket.write_all(&response.body).await;
    let _ = socket.shutdown().await;
    Ok(RecordedRequest { path, headers })
}
impl BinaryFixture {
    async fn new(handler: impl Fn(&str, &str) -> BinaryResponse + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let (stop, mut stopped) = oneshot::channel();
        let owner_origin = origin.clone();
        let handler: Handler = Arc::new(handler);
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            let mut final_requests = FinalRequests::default();
            loop {
                tokio::select! {
                    biased;
                    _ = &mut stopped => break,
                    accepted = listener.accept() => match accepted {
                        Ok((socket, _)) => { connections.spawn(respond(socket, owner_origin.clone(), handler.clone())); }
                        Err(error) => { final_requests.errors.push(format!("fixture accept: {error}")); break; }
                    },
                    Some(result) = connections.join_next(), if !connections.is_empty() => collect_finished(result, &mut final_requests),
                }
            }
            drop(listener);
            while let Some(result) = connections.join_next().await {
                collect_finished(result, &mut final_requests);
            }
            final_requests
        });
        Self {
            origin,
            stop: Some(stop),
            task: Some(task),
        }
    }
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin)
    }
    async fn finish(mut self) -> FinalRequests {
        let _ = self.stop.take().unwrap().send(());
        let mut task = self.task.take().unwrap();
        match tokio::time::timeout(Duration::from_secs(5), &mut task).await {
            Ok(result) => result.expect("fixture owner panicked"),
            Err(_) => {
                task.abort();
                let _ = task.await;
                panic!("fixture owner/connection drain exceeded 5 seconds");
            }
        }
    }
}
fn assert_requests(final_requests: &FinalRequests, expected: &[(&str, usize)]) {
    assert!(
        final_requests.errors.is_empty(),
        "{:?}",
        final_requests.errors
    );
    let expected = expected
        .iter()
        .map(|(path, count)| ((*path).to_owned(), *count))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        final_requests.counts(),
        expected,
        "all listener/connection tasks already joined"
    );
}
use super::sitemap_capture::{CaptureFailure, CountCapture, RunBasis};

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut out = GzEncoder::new(Vec::new(), Compression::default());
    out.write_all(bytes).unwrap();
    out.finish().unwrap()
}
fn xml(origin: &str) -> Vec<u8> {
    format!("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>{origin}/page</loc></url></urlset>").into_bytes()
}
fn setup(
    fixture: &BinaryFixture,
    path: &str,
    change: impl FnOnce(&mut CrawlConfig),
) -> (tempfile::TempDir, Crawler, u64, DiscoveryEntry) {
    let mut cfg = CrawlConfig {
        seed_urls: vec![fixture.url("/seed")],
        sitemap_urls: vec![fixture.url(path)],
        discover_sitemaps: false,
        crawl_links: false,
        delay_ms: 0,
        obey_robots: false,
        concurrency: 1,
        request_timeout_secs: 3,
        ..Default::default()
    };
    change(&mut cfg);
    let temp = tempfile::tempdir().unwrap();
    let crawler = Crawler::new(cfg)
        .unwrap()
        .with_store(temp.path().join("capture.sqlite"))
        .unwrap();
    let store = crawler.store.as_ref().unwrap();
    store.prepare(&crawler.config).unwrap();
    let generation = store.begin_run("source-capture-http-fixture").unwrap();
    crawler.initialize_original_sitemaps(generation).unwrap();
    let entry = DiscoveryEntry {
        kind: DiscoveryKind::Sitemap,
        url: fixture.url(path),
    };
    (temp, crawler, generation, entry)
}
fn context(crawler: &Crawler, generation: u64) -> FetchContext {
    FetchContext {
        quotas: None,
        store: crawler.store.clone(),
        capture: crawler.capture.clone(),
        generation: Some(generation),
        config: crawler.config.clone(),
        seeds: crawler.seeds.clone(),
        client: crawler.client.clone(),
        headers: crawler.headers.clone(),
        robots: crawler.robots.clone(),
        rate: crawler.rate.clone(),
        pdf_slots: crawler.pdf_slots.clone(),
        pdf_tasks: crate::TaskOwner::new("sitemap_fixture_pdf"),
        policy: crawler.policy.clone(),
        pdf_parse_hook: None,
        cdn_classification: None,
        cdn_directory_reporting: None,
    }
}
async fn collect(crawler: &Crawler, generation: u64, entry: &DiscoveryEntry) -> DiscoveryOutput {
    let (_, basis) =
        sitemap_capture::discovery_basis(crawler.store.as_deref(), Some(generation)).unwrap();
    assert!(
        matches!(&basis, RunBasis::Verified { .. }),
        "actual saved config/generation basis required"
    );
    crawler
        .discover_unit_observed_bound(entry, &context(crawler, generation), &basis)
        .await
        .unwrap()
}
fn measured(output: &DiscoveryOutput) -> &crate::SitemapSourceMeasurement {
    output
        .source_capture
        .as_ref()
        .expect("captured outcome")
        .measurement
        .as_ref()
        .expect("strict measurement binding")
}
fn assess(output: &DiscoveryOutput) -> crate::SitemapSourceProtocolAssessment {
    crate::assess_sitemap_source_protocol_limits(measured(output), &Default::default()).unwrap()
}
fn count(output: &DiscoveryOutput) -> &crate::SitemapCountObservation {
    match &output.source_capture.as_ref().unwrap().captured.count {
        CountCapture::Observed(result) => match result.as_ref() {
            Ok(count) => count,
            other => panic!("expected actual scanner observation, got {other:?}"),
        },
        other => panic!("expected actual scanner observation, got {other:?}"),
    }
}

#[tokio::test]
async fn real_plain_file_gzip_and_http_gzip_capture_same_xml_without_extra_requests() {
    let fixture = BinaryFixture::new(|origin, path| {
        let bytes = xml(origin);
        match path {
            "/plain.xml" => BinaryResponse::bytes(bytes, "application/xml"),
            "/file.gz" => BinaryResponse::bytes(gzip(&bytes), "application/octet-stream"),
            "/http.xml" => BinaryResponse::bytes(gzip(&bytes), "application/xml")
                .header("Content-Encoding", "gzip"),
            "/index.xml" => BinaryResponse::bytes(format!("<sitemapindex xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><sitemap><loc>{origin}/child.xml</loc></sitemap></sitemapindex>").into_bytes(), "application/xml"),
            _ => BinaryResponse::bytes(Vec::new(), "text/plain"),
        }
    })
    .await;
    let expected_xml = xml(&fixture.origin);
    for (path, encoding) in [
        ("/plain.xml", crate::SitemapEntityEncoding::Identity),
        ("/file.gz", crate::SitemapEntityEncoding::GzipFileEntity),
        ("/http.xml", crate::SitemapEntityEncoding::Identity),
    ] {
        let (_temp, crawler, generation, entry) = setup(&fixture, path, |_| {});
        let output = collect(&crawler, generation, &entry).await;
        let record = measured(&output);
        let raw = &output.source_capture.as_ref().unwrap().captured;
        assert_eq!(raw.decoder.as_ref().unwrap().encoding, Some(encoding));
        assert_eq!(
            raw.decoder
                .as_ref()
                .unwrap()
                .decoded_complete_sha256
                .as_deref(),
            Some(crate::sitemap_ledger::digest(&expected_xml).as_str())
        );
        assert_eq!(
            count(&output).input_xml_sha256,
            crate::sitemap_ledger::digest(&expected_xml)
        );
        assert_eq!(
            record
                .payload
                .scan
                .counts
                .protocol_eligible_completed_entries
                .value,
            Some(1)
        );
        assert_eq!(
            assess(&output).entry_limit.verdict,
            crate::SitemapSourceLimitVerdict::WithinLimit
        );
        assert_eq!(
            record.payload.membership_retention,
            crate::SitemapMembershipRetention::Complete
        );
        assert_eq!(
            crawler
                .store
                .as_ref()
                .unwrap()
                .sitemap_catalog()
                .unwrap()
                .unwrap()
                .summary
                .document_rows,
            0,
            "complete in-flight parser retention is not a persisted document receipt"
        );
        assert_eq!(
            record.payload.binding.expected_source_observation_id,
            output
                .sitemap_batch
                .as_ref()
                .unwrap()
                .document
                .observation_id
        );
    }
    let (_temp, crawler, generation, entry) = setup(&fixture, "/index.xml", |_| {});
    let output = collect(&crawler, generation, &entry).await;
    assert_eq!(
        count(&output).kind,
        Some(crate::SitemapDocumentKind::SitemapIndex)
    );
    assert_eq!(count(&output).counts.protocol_eligible_completed_entries, 1);
    assert_eq!(
        assess(&output).entry_limit.verdict,
        crate::SitemapSourceLimitVerdict::WithinLimit
    );
    let requests = fixture.finish().await;
    assert_requests(
        &requests,
        &[
            ("/plain.xml", 1),
            ("/file.gz", 1),
            ("/http.xml", 1),
            ("/index.xml", 1),
        ],
    );
}

#[tokio::test]
async fn real_received_prefix_budget_206_and_robots_denial_remain_explicit() {
    let fixture = BinaryFixture::new(|origin, path| match path {
        "/robots.txt" => {
            BinaryResponse::robots(b"User-agent: *\nDisallow: /blocked.xml\n".to_vec())
        }
        "/large.xml" => BinaryResponse::bytes(vec![b'x'; 4096], "application/xml"),
        "/partial.xml" => {
            let mut r = BinaryResponse::bytes(xml(origin), "application/xml");
            r.status = 206;
            r
        }
        _ => BinaryResponse::bytes(xml(origin), "application/xml"),
    })
    .await;
    let (_temp, crawler, generation, entry) =
        setup(&fixture, "/large.xml", |cfg| cfg.max_body_bytes = 128);
    let output = collect(&crawler, generation, &entry).await;
    let p = &measured(&output).payload;
    assert_eq!(p.transport.retained_post_http_prefix_bytes, 128);
    assert_eq!(
        p.transport.post_http_received_bytes.precision,
        crate::SitemapMeasuredPrecision::AtLeast
    );
    assert!(p.transport.post_http_received_bytes.value.unwrap() > 128);
    assert!(p.transport.post_http_received_bytes.value.unwrap() <= 4096);
    assert_eq!(
        p.transport.retained_post_http_prefix_sha256.as_deref(),
        Some(crate::sitemap_ledger::digest(vec![b'x'; 128]).as_str())
    );
    assert_eq!(
        p.decode.integrity,
        crate::SitemapSourceDecodeIntegrity::NotObserved
    );
    assert_eq!(p.scan.status, crate::SitemapSourceScanStatus::NotObserved);
    assert_eq!(
        assess(&output).decoded_xml_byte_limit.verdict,
        crate::SitemapSourceLimitVerdict::Unknown
    );
    assert_eq!(
        output
            .protocol_observation
            .as_ref()
            .unwrap()
            .failure
            .as_ref()
            .unwrap()
            .code,
        "sitemap_transport_body_byte_budget"
    );
    let (_temp, crawler, generation, entry) = setup(&fixture, "/partial.xml", |_| {});
    let output = collect(&crawler, generation, &entry).await;
    assert!(measured(&output).payload.transport.body_complete);
    assert_eq!(measured(&output).payload.transport.final_status, Some(206));
    assert_eq!(
        assess(&output).entry_limit.verdict,
        crate::SitemapSourceLimitVerdict::Unknown
    );
    assert!(
        output
            .source_capture
            .as_ref()
            .unwrap()
            .captured
            .decoder
            .is_none()
    );
    let (_temp, crawler, generation, entry) =
        setup(&fixture, "/blocked.xml", |cfg| cfg.obey_robots = true);
    let output = collect(&crawler, generation, &entry).await;
    assert!(!measured(&output).payload.transport.request_started);
    assert!(measured(&output).payload.transport.blocked_by_robots);
    assert_eq!(
        measured(&output).payload.transport.post_http_received_bytes,
        crate::SitemapMeasuredValue::unavailable()
    );
    assert!(
        output
            .source_capture
            .as_ref()
            .unwrap()
            .captured
            .decoder
            .is_none()
    );
    let requests = fixture.finish().await;
    assert_requests(
        &requests,
        &[("/large.xml", 1), ("/partial.xml", 1), ("/robots.txt", 1)],
    );
}

#[tokio::test]
async fn real_gzip_crc_truncation_and_inflation_failure_keep_helper_progress_not_counts() {
    let fixture = BinaryFixture::new(|origin, path| {
        let mut body = gzip(&xml(origin));
        match path {
            "/crc.gz" => {
                let n = body.len();
                body[n - 8] ^= 1;
            }
            "/eof.gz" => {
                body.truncate(body.len() - 5);
            }
            "/inflate.gz" => {
                body = gzip(&vec![b'x'; 9 * 1024 * 1024]);
            }
            _ => {}
        }
        BinaryResponse::bytes(body, "application/octet-stream")
    })
    .await;
    for (path, integrity) in [
        ("/crc.gz", crate::SitemapEntityDecodeIntegrity::Failed),
        ("/eof.gz", crate::SitemapEntityDecodeIntegrity::Incomplete),
        (
            "/inflate.gz",
            crate::SitemapEntityDecodeIntegrity::Incomplete,
        ),
    ] {
        let (_temp, crawler, generation, entry) = setup(&fixture, path, |_| {});
        let output = collect(&crawler, generation, &entry).await;
        let raw = &output.source_capture.as_ref().unwrap().captured;
        let decoder = raw.decoder.as_ref().unwrap();
        assert_eq!(decoder.integrity, integrity);
        assert!(
            measured(&output).payload.transport.body_complete,
            "file integrity failure does not invent HTTP incompleteness"
        );
        assert!(decoder.decoded_complete_sha256.is_none());
        assert!(matches!(&raw.count, CountCapture::NotInvoked(_)));
        assert_eq!(
            assess(&output).entry_limit.verdict,
            crate::SitemapSourceLimitVerdict::Unknown
        );
        if path == "/eof.gz" {
            assert_eq!(
                decoder.read_error_kind,
                Some(crate::SitemapEntityDecodeReadErrorKind::UnexpectedEof)
            );
            assert_eq!(
                measured(&output).payload.decode.stop_code,
                Some(crate::SitemapSourceMeasurementStop::InvalidGzip)
            );
        }
        if path == "/inflate.gz" {
            assert_eq!(decoder.decoded_bytes_returned, Some(8 * 1024 * 1024 + 1));
            assert_eq!(
                output
                    .protocol_observation
                    .as_ref()
                    .unwrap()
                    .failure
                    .as_ref()
                    .unwrap()
                    .code,
                "sitemap_xml_inflation_byte_budget"
            );
        }
    }
    let requests = fixture.finish().await;
    assert_requests(
        &requests,
        &[("/crc.gz", 1), ("/eof.gz", 1), ("/inflate.gz", 1)],
    );
}

#[tokio::test]
async fn real_50001_entries_are_counted_even_when_legacy_parser_stops_retaining() {
    let fixture = BinaryFixture::new(|origin, _| {
        let mut xml = String::from("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'>");
        for _ in 0..50_001 {
            xml.push_str(&format!("<url><loc>{origin}/p</loc></url>"));
        }
        xml.push_str("</urlset>");
        BinaryResponse::bytes(xml.into_bytes(), "application/xml")
    })
    .await;
    let (_temp, crawler, generation, entry) = setup(&fixture, "/many.xml", |_| {});
    let mut output = collect(&crawler, generation, &entry).await;
    assert_eq!(
        count(&output).counts.protocol_eligible_completed_entries,
        50_001
    );
    assert!(count(&output).xml_scan_complete);
    assert_eq!(
        output
            .protocol_observation
            .as_ref()
            .unwrap()
            .failure
            .as_ref()
            .unwrap()
            .code,
        "sitemap_entry_budget"
    );
    assert_eq!(
        assess(&output).entry_limit.verdict,
        crate::SitemapSourceLimitVerdict::ObservedExceeded
    );
    assert_eq!(
        measured(&output).payload.membership_retention,
        crate::SitemapMembershipRetention::Incomplete
    );
    assert!(!output.sitemap_batch.as_ref().unwrap().document.complete());
    crawler
        .finalize_ordinary_protocol(&mut output, &HashSet::new(), &HashSet::new())
        .unwrap();
    assert!(
        output.pages.is_empty() && output.children.is_empty(),
        "count success cannot grant admission"
    );
    assert_eq!(
        crawler
            .store
            .as_ref()
            .unwrap()
            .sitemap_catalog()
            .unwrap()
            .unwrap()
            .summary
            .document_rows,
        0,
        "not persisted; full32MiB catalog admission/rollback is a separate gate"
    );
    let requests = fixture.finish().await;
    assert_requests(&requests, &[("/many.xml", 1)]);
}

#[tokio::test]
async fn real_multiloc_utf8_and_dtd_diagnostics_do_not_replace_legacy_failure() {
    let fixture=BinaryFixture::new(|origin,path| {
        let body=match path {
            "/multi.xml"=>format!("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>{origin}/a</loc><loc>invalid</loc></url><url><loc>{origin}/b</loc></url></urlset>").into_bytes(),
            "/utf8.xml"=>vec![0xff],
            "/dtd.xml"=>{let mut v=xml(origin); v.extend_from_slice(b"<!DOCTYPE urlset>");v},
            _=>xml(origin),
        };
        BinaryResponse::bytes(body,"application/xml")
    }).await;
    for (path, legacy) in [
        ("/multi.xml", "sitemap_ambiguous_multiple_locations"),
        ("/utf8.xml", "sitemap_invalid_utf8"),
        ("/dtd.xml", "sitemap_dtd_unsupported"),
    ] {
        let (_temp, crawler, generation, entry) = setup(&fixture, path, |_| {});
        let output = collect(&crawler, generation, &entry).await;
        assert_eq!(
            output
                .protocol_observation
                .as_ref()
                .unwrap()
                .failure
                .as_ref()
                .unwrap()
                .code,
            legacy
        );
        assert_eq!(
            assess(&output).decoded_xml_byte_limit.verdict,
            crate::SitemapSourceLimitVerdict::WithinLimit
        );
        if path == "/multi.xml" {
            let c = &count(&output).counts;
            assert_eq!(
                (
                    c.completed_immediate_entries,
                    c.protocol_eligible_completed_entries,
                    c.multiple_location_entries,
                    c.invalid_single_location_entries
                ),
                (2, 1, 1, 0)
            );
            assert_eq!(c.invalid_location_occurrences, 1);
            assert_eq!(
                measured(&output)
                    .payload
                    .scan
                    .counts
                    .actual_document_namespace_loc_occurrences
                    .value,
                Some(3)
            );
        } else {
            assert_eq!(
                assess(&output).entry_limit.verdict,
                crate::SitemapSourceLimitVerdict::Unknown
            );
        }
    }
    let requests = fixture.finish().await;
    assert_requests(
        &requests,
        &[("/multi.xml", 1), ("/utf8.xml", 1), ("/dtd.xml", 1)],
    );
}

#[tokio::test]
async fn actual_current_saved_config_is_not_the_historical_catalog_initial_hash() {
    let fixture =
        BinaryFixture::new(|origin, _| BinaryResponse::bytes(xml(origin), "application/xml")).await;
    let (_temp, mut crawler, _generation, entry) = setup(&fixture, "/current.xml", |_| {});
    let initial = crawler
        .store
        .as_ref()
        .unwrap()
        .sitemap_catalog()
        .unwrap()
        .unwrap()
        .summary
        .config_sha256;
    crawler.config.delay_ms = 1;
    crawler
        .store
        .as_ref()
        .unwrap()
        .prepare(&crawler.config)
        .unwrap();
    let generation = crawler
        .store
        .as_ref()
        .unwrap()
        .begin_run("capture-current-config-resume")
        .unwrap();
    let expected = crawler
        .store
        .as_ref()
        .unwrap()
        .saved_config_bytes_sha256()
        .unwrap();
    assert_ne!(
        expected, initial,
        "actual accepted operational change rewrote saved config bytes"
    );
    let output = collect(&crawler, generation, &entry).await;
    assert_eq!(
        measured(&output).payload.binding.crawl_config_sha256,
        expected
    );
    assert_ne!(
        measured(&output).payload.binding.crawl_config_sha256,
        initial
    );
    assert_eq!(measured(&output).payload.binding.run_generation, generation);
    assert_eq!(
        crawler
            .store
            .as_ref()
            .unwrap()
            .sitemap_catalog()
            .unwrap()
            .unwrap()
            .summary
            .config_sha256,
        initial
    );
    let requests = fixture.finish().await;
    assert_requests(&requests, &[("/current.xml", 1)]);
}

#[tokio::test]
async fn actual_capture_adapter_rejects_unverified_mismatch_and_counter_loss_without_mutating_batch()
 {
    let fixture =
        BinaryFixture::new(|origin, _| BinaryResponse::bytes(xml(origin), "application/xml")).await;
    let (_temp, crawler, generation, entry) = setup(&fixture, "/bind.xml", |_| {});
    let output = collect(&crawler, generation, &entry).await;
    let before = output.sitemap_batch.as_ref().unwrap().document.clone();
    let original = &output.source_capture.as_ref().unwrap().captured;
    let no_basis = original.clone().into_outcome(
        &before,
        &RunBasis::Unavailable(CaptureFailure::DirectCallerUnverified),
    );
    assert_eq!(
        no_basis.measurement.unwrap_err(),
        CaptureFailure::DirectCallerUnverified
    );
    let (_, basis) =
        sitemap_capture::discovery_basis(crawler.store.as_deref(), Some(generation)).unwrap();
    let mut bad = original.clone();
    if let CountCapture::Observed(result) = &mut bad.count
        && let Ok(count) = result.as_mut()
    {
        count.input_xml_sha256 = "0".repeat(64);
    }
    assert_eq!(
        bad.into_outcome(&before, &basis).measurement.unwrap_err(),
        CaptureFailure::InconsistentHelperFacts
    );
    let mut bad = original.clone();
    bad.decoder.as_mut().unwrap().engine = "future-engine".into();
    assert_eq!(
        bad.into_outcome(&before, &basis).measurement.unwrap_err(),
        CaptureFailure::UnsupportedHelperProfile
    );
    let mut lost = original.clone();
    lost.received.received_bytes = Some(u64::MAX);
    lost.received.returned(1);
    assert_eq!(
        lost.into_outcome(&before, &basis).measurement.unwrap_err(),
        CaptureFailure::ReceiveCounterUnavailable
    );
    let stale = RunBasis::Verified {
        config_sha256: measured(&output)
            .payload
            .binding
            .crawl_config_sha256
            .clone(),
        generation: generation + 1,
    };
    assert_eq!(
        original
            .clone()
            .into_outcome(&before, &stale)
            .measurement
            .unwrap_err(),
        CaptureFailure::GenerationMismatch
    );
    assert_eq!(
        output.sitemap_batch.as_ref().unwrap().document,
        before,
        "new failures never rewrite legacy identity/completeness"
    );
    let requests = fixture.finish().await;
    assert_requests(&requests, &[("/bind.xml", 1)]);
}

#[test]
fn pure_adapter_uses_actual_identity_budget_helper_without_inventing_xml_output() {
    // Actual helper invocation; synthetic transport/config context for a PURE
    // adapter control. This is deliberately not presented as HTTP/native proof.
    let body = b"<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'/>";
    let limits = crate::SitemapParserLimits {
        max_xml_bytes: body.len() - 1,
        ..Default::default()
    };
    let decoded = crate::decode_sitemap_entity_observed(body, &limits);
    let failure = decoded.result.unwrap_err();
    assert_eq!(failure.code, "sitemap_xml_byte_budget");
    assert_eq!(
        decoded.observation.encoding,
        Some(crate::SitemapEntityEncoding::Identity)
    );
    assert!(decoded.observation.decoded_bytes_returned.is_none());
    let mut received = sitemap_capture::ReceiveProgress::new(limits.max_entity_bytes);
    received.begin_body();
    received.returned(body.len());
    let mut raw = sitemap_capture::SourceCapture::not_invoked(
        received,
        crate::SitemapSourceMeasurementStop::DecodedByteLimit,
    );
    raw.limits = limits;
    raw.retained_prefix_sha256 = Some(crate::sitemap_ledger::digest(body));
    raw.decoder = Some(decoded.observation);
    let url = "https://pure-context.example/sitemap.xml";
    let batch = crate::sitemap_discovery::xml_batch(
        url,
        1,
        "explicit-pure-helper-context",
        crate::SitemapTransportFacts {
            request_started: true,
            requested_url: Some(url.into()),
            final_url: Some(url.into()),
            initial_status: Some(200),
            final_status: Some(200),
            blocked_by_robots: false,
            body_complete: true,
            post_http_decoding_bytes: body.len() as u64,
            post_http_decoding_sha256: Some(crate::sitemap_ledger::digest(body)),
            retained_post_http_prefix_sha256: Some(crate::sitemap_ledger::digest(body)),
            failure_code: Some(failure.code.clone()),
        },
        None,
        None,
        Some(failure.code),
    )
    .unwrap();
    let before = batch.document.clone();
    let basis = RunBasis::Verified {
        config_sha256: crate::sitemap_ledger::digest(b"explicit synthetic pure adapter context"),
        generation: 1,
    };
    let outcome = raw.into_outcome(&batch.document, &basis);
    let record = outcome.measurement.unwrap();
    assert_eq!(
        record.payload.decode.encoding,
        Some(crate::SitemapEntityEncoding::Identity)
    );
    assert_eq!(
        record.payload.decode.integrity,
        crate::SitemapSourceDecodeIntegrity::NotObserved
    );
    assert_eq!(
        record.payload.decode.decoded_xml_bytes,
        crate::SitemapMeasuredValue::unavailable()
    );
    let assessment =
        crate::assess_sitemap_source_protocol_limits(&record, &Default::default()).unwrap();
    assert_eq!(
        assessment.decoded_xml_byte_limit.verdict,
        crate::SitemapSourceLimitVerdict::Unknown
    );
    assert_eq!(batch.document, before);
}

#[tokio::test]
async fn actual_discovery_snapshot_corruption_is_error_while_no_store_is_unknown() {
    let (catalog, basis) = sitemap_capture::discovery_basis(None, Some(1)).unwrap();
    assert!(catalog.is_none());
    assert!(matches!(
        basis,
        RunBasis::Unavailable(CaptureFailure::NoDurableContext)
    ));
    let fixture =
        BinaryFixture::new(|origin, _| BinaryResponse::bytes(xml(origin), "application/xml")).await;
    let (temp, crawler, generation, _entry) = setup(&fixture, "/never-requested.xml", |_| {});
    let writer = rusqlite::Connection::open(temp.path().join("capture.sqlite")).unwrap();
    writer
        .execute(
            "UPDATE discovery_frontier SET done=2 WHERE kind='sitemap'",
            [],
        )
        .unwrap();
    assert!(
        crawler
            .store
            .as_ref()
            .unwrap()
            .sitemap_catalog()
            .unwrap()
            .is_some(),
        "healthy legacy catalog alone cannot authorize a failed full snapshot"
    );
    assert!(
        sitemap_capture::discovery_basis(crawler.store.as_deref(), Some(generation)).is_err(),
        "real corrupt SQL context must not become optional unknown or legacy fallback"
    );
    writer.execute_batch("DROP TABLE metadata").unwrap();
    assert!(
        sitemap_capture::discovery_basis(crawler.store.as_deref(), Some(generation)).is_err(),
        "missing required actual SQL table remains an explicit error"
    );
    drop(writer);
    let requests = fixture.finish().await;
    assert_requests(&requests, &[]);
}

#[tokio::test]
async fn actual_long_task_context_remains_failed_source_with_unknown_measurement_not_crawl_error() {
    let fixture =
        BinaryFixture::new(|origin, _| BinaryResponse::bytes(xml(origin), "application/xml")).await;
    let path = format!("/{}", "long-task".repeat(1024));
    let (_temp, crawler, generation, entry) = setup(&fixture, &path, |_| {});
    let output = collect(&crawler, generation, &entry).await;
    let document = &output.sitemap_batch.as_ref().unwrap().document;
    assert!(!document.complete());
    assert!(document.transport.as_ref().unwrap().request_started);
    let failure = output
        .source_capture
        .as_ref()
        .unwrap()
        .measurement
        .as_ref()
        .unwrap_err();
    assert_eq!(
        capture_unavailable_reason(failure, document),
        Some(SourceCaptureUnavailable::IdentityBytes)
    );
    assert_eq!(
        capture_unavailable_reason(&CaptureFailure::InconsistentHelperFacts, document),
        None
    );
    assert_eq!(
        capture_unavailable_reason(&CaptureFailure::GenerationMismatch, document),
        None
    );
    let store = crawler.store.as_ref().unwrap();
    let sha = store
        .sitemap_read_snapshot()
        .unwrap()
        .crawl_config_sha256
        .unwrap();
    let receipt = store
        .record_unavailable_sitemap_source_capture(&entry, document, &sha, generation)
        .unwrap();
    assert_eq!(
        receipt.omission_reason(),
        Some(crate::SitemapSourceStoreOmission::CaptureUnavailable)
    );
    store
        .complete_discovery_measured(
            &entry,
            &[],
            &[],
            &output.warnings,
            generation,
            output.sitemap_batch.unwrap(),
            &receipt,
        )
        .unwrap();
    let snapshot = store
        .sitemap_read_snapshot()
        .unwrap()
        .sitemap_source_measurements
        .unwrap();
    assert!(snapshot.records.is_empty() && snapshot.commits.is_empty());
    assert_eq!(
        snapshot.summary.omission,
        Some(crate::SitemapSourceStoreOmission::CaptureUnavailable)
    );
    let requests = fixture.finish().await;
    assert_requests(&requests, &[(path.as_str(), 1)]);
}

#[tokio::test]
async fn actual_oversized_redirect_context_retains_known_transport_with_unknown_decode_and_count() {
    let final_path = format!("/{}", "large-final-context".repeat(512));
    let expected_path = final_path.clone();
    let fixture = BinaryFixture::new(move |origin, path| {
        if path == "/redirect.xml" {
            BinaryResponse::redirect(&format!("{origin}{final_path}"))
        } else {
            BinaryResponse::bytes(xml(origin), "application/xml")
        }
    })
    .await;
    let (_temp, crawler, generation, entry) = setup(&fixture, "/redirect.xml", |_| {});
    let output = collect(&crawler, generation, &entry).await;
    let document = &output.sitemap_batch.as_ref().unwrap().document;
    assert_eq!(document.task_url, entry.url);
    assert_eq!(
        document.failure_code.as_deref(),
        Some("sitemap_transport_context_byte_budget")
    );
    let record = measured(&output);
    assert!(record.payload.transport.final_url.is_none());
    assert!(record.payload.transport.body_complete);
    assert_eq!(
        record.payload.decode.integrity,
        crate::SitemapSourceDecodeIntegrity::NotObserved
    );
    assert_eq!(
        assess(&output).entry_limit.verdict,
        crate::SitemapSourceLimitVerdict::Unknown
    );
    assert_eq!(
        assess(&output).decoded_xml_byte_limit.verdict,
        crate::SitemapSourceLimitVerdict::Unknown
    );
    let store = crawler.store.as_ref().unwrap();
    let receipt = store
        .record_sitemap_source_capture(&entry, document, record, generation)
        .unwrap();
    store
        .complete_discovery_measured(
            &entry,
            &[],
            &[],
            &output.warnings,
            generation,
            output.sitemap_batch.unwrap(),
            &receipt,
        )
        .unwrap();
    let saved = store
        .sitemap_read_snapshot()
        .unwrap()
        .sitemap_source_measurements
        .unwrap();
    assert_eq!(saved.records.len(), 1);
    assert_eq!(saved.commits.len(), 1);
    assert!(saved.summary.omission.is_none());
    let requests = fixture.finish().await;
    assert_requests(
        &requests,
        &[("/redirect.xml", 1), (expected_path.as_str(), 1)],
    );
}

#[path = "sitemap_large_capture_tests.rs"]
mod large_source_tests;
