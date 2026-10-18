//! B12.1 Work-only sibling of crawler.rs. AUTHORED / NOT COMPILED / NOT RUN.
//! Requires protocol-types.rs.txt + fetch-wrappers.rs.txt + materialized request-loop changes.
//! Tests call the real private wrappers; no mirrored fetch/parser implementation.
use super::*;
use flate2::{Compression, write::GzEncoder};
use std::{
    collections::BTreeMap,
    io::Write,
    sync::atomic::{AtomicUsize, Ordering},
};
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
fn assert_credentials(final_requests: &FinalRequests, path: &str, expected: bool) {
    let matched = final_requests
        .requests
        .iter()
        .filter(|request| request.path == path)
        .collect::<Vec<_>>();
    assert!(!matched.is_empty(), "no recorded request for {path}");
    for request in matched {
        for (key, sentinel) in [
            ("authorization", "Bearer sitemap-fixture-only"),
            ("cookie", "sitemap_fixture=only"),
            ("x-seed-secret", "sitemap-fixture-only"),
        ] {
            assert_eq!(
                request.headers.get(key).map(String::as_str),
                expected.then_some(sentinel),
                "{path}: {key}"
            );
        }
    }
}
fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}
fn xml(origin: &str) -> Vec<u8> {
    format!("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>{origin}/page-sentinel</loc></url></urlset>").into_bytes()
}
fn policy_config(origin: &str) -> crate::CrawlPolicyConfig {
    crate::CrawlPolicyConfig {
        scope: crate::CrawlScopeConfig {
            roots: vec![crate::CrawlScopeRoot {
                origin: origin.into(),
                subdomains: false,
                folders: vec!["/".into()],
                exact_paths: vec![],
            }],
            ..Default::default()
        },
        ..Default::default()
    }
}
fn config(fixture: &BinaryFixture, native_policy: bool, resources: bool) -> CrawlConfig {
    CrawlConfig {
        seed_urls: vec![fixture.url("/seed")],
        discover_sitemaps: false,
        crawl_links: false,
        delay_ms: 0,
        obey_robots: false,
        concurrency: 1,
        request_timeout_secs: 3,
        custom_headers: BTreeMap::from([
            ("Authorization".into(), "Bearer sitemap-fixture-only".into()),
            ("Cookie".into(), "sitemap_fixture=only".into()),
            ("X-Seed-Secret".into(), "sitemap-fixture-only".into()),
        ]),
        crawl_policy: native_policy.then(|| policy_config(&fixture.origin)),
        resource_discovery: resources.then(crate::ResourceDiscoveryConfig::default),
        ..Default::default()
    }
}
fn context(config: CrawlConfig) -> FetchContext {
    let crawler = Crawler::new(config).unwrap();
    context_from_crawler(&crawler)
}
fn context_from_crawler(crawler: &Crawler) -> FetchContext {
    FetchContext {
        quotas: None,
        store: crawler.store.clone(),
        capture: crawler.capture.clone(),
        custom_search: crawler.custom_search.clone(),
        generation: None,
        config: crawler.config.clone(),
        seeds: crawler.seeds.clone(),
        client: crawler.client.clone(),
        headers: crawler.headers.clone(),
        robots: crawler.robots.clone(),
        hsts: crawler.hsts.clone(),
        rate: crawler.rate.clone(),
        pdf_slots: crawler.pdf_slots.clone(),
        pdf_tasks: crate::TaskOwner::new("sitemap_fixture_pdf"),
        policy: crawler.policy.clone(),
        pdf_parse_hook: None,
        quota_start_hook: None,
        cdn_classification: None,
        cdn_directory_reporting: None,
    }
}
fn admission(
    policy: &crate::CompiledCrawlPolicy,
    url: &str,
    role: crate::PolicyTaskRole,
) -> crate::PolicyAdmission {
    let row = crate::policy_ledger::decide(
        policy,
        "sitemap-protocol-native-fixture",
        1,
        crate::CandidateSourceKind::SitemapDocument,
        url.into(),
        None,
        None,
        crate::PolicyInputBasis::DeclaredInput,
        role,
    );
    crate::policy_ledger::admission(&row).expect("fixture scope should admit the declared URL")
}
fn entry(context: &FetchContext, url: &str, role: crate::PolicyTaskRole) -> QueueEntry {
    QueueEntry {
        url: url.into(),
        depth: 0,
        discovered_from: None,
        admission: context
            .policy
            .as_ref()
            .map(|policy| admission(policy, url, role)),
    }
}
async fn fetch(context: &FetchContext, url: &str) -> (PageRecord, SitemapProtocolObservation) {
    context
        .sitemap_protocol(entry(context, url, crate::PolicyTaskRole::SitemapDocument))
        .await
}
fn assert_success(
    page: &PageRecord,
    observed: &SitemapProtocolObservation,
    post_http_bytes: &[u8],
    expected_xml: &[u8],
    encoding: crate::SitemapEntityEncoding,
) {
    assert_eq!(page.final_status, Some(200));
    assert!(page.error.is_none());
    assert!(!page.truncated);
    assert!(!page.blocked_by_robots);
    assert!(observed.request_started && observed.body_complete);
    assert!(observed.failure.is_none());
    assert_eq!(observed.requested_url.as_deref(), Some(page.url.as_str()));
    assert_eq!(observed.final_url.as_deref(), Some(page.final_url.as_str()));
    assert_eq!(observed.initial_status, page.status);
    assert_eq!(observed.final_status, Some(200));
    assert!(!observed.blocked_by_robots);
    assert!(!observed.truncated);
    assert_eq!(observed.redirect_count, page.redirect_chain.len());
    assert_eq!(
        observed.retained_post_http_entity_bytes,
        post_http_bytes.len()
    );
    assert_eq!(
        observed.post_http_entity_sha256.as_deref(),
        Some(crate::resource_policy::digest(post_http_bytes).as_str())
    );
    let entity = observed.entity_decode.as_ref().unwrap();
    assert_eq!(entity.encoding, encoding);
    assert_eq!(entity.xml, expected_xml);
    assert_eq!(entity.post_http_decode_entity_bytes, post_http_bytes.len());
    assert_eq!(entity.decoded_xml_bytes, expected_xml.len());
    let parser = observed.parser.as_ref().unwrap();
    assert!(parser.complete);
    assert!(parser.failure.is_none());
    assert_eq!(parser.admission_locations().count(), 1);
    assert_eq!(
        parser.xml_sha256,
        crate::resource_policy::digest(expected_xml)
    );
    assert_eq!(
        parser.context.requested_document_url.as_deref(),
        Some(page.url.as_str())
    );
    assert_eq!(
        parser.context.final_document_url.as_deref(),
        Some(page.final_url.as_str())
    );
    assert!(parser.context.originating_occurrence_id.is_none());
    assert!(page.pdf.is_none());
    assert!(page.resource_permission.is_none());
    assert!(page.resource_source.is_none());
    assert!(page.resource_response.is_none());
    assert!(
        page.links.is_empty(),
        "protocol wrapper must return before page sniffing/expansion"
    );
}

#[tokio::test]
async fn sitemap_fetch_works_in_all_four_policy_resource_modes_without_page_grants() {
    for (native_policy, resources) in [(false, false), (false, true), (true, false), (true, true)] {
        let fixture =
            BinaryFixture::new(|origin, _| BinaryResponse::bytes(xml(origin), "application/xml"))
                .await;
        let url = fixture.url("/sitemap.xml");
        let expected = xml(&fixture.origin);
        let context = context(config(&fixture, native_policy, resources));
        let (page, observed) = fetch(&context, &url).await;
        let page_without_grant = if resources {
            Some(
                context
                    .document(entry(
                        &context,
                        &fixture.url("/page-without-grant"),
                        crate::PolicyTaskRole::PageDocument,
                    ))
                    .await,
            )
        } else {
            None
        };
        let final_requests = fixture.finish().await;
        assert_requests(&final_requests, &[("/sitemap.xml", 1)]);
        assert_credentials(&final_requests, "/sitemap.xml", true);
        assert_success(
            &page,
            &observed,
            &expected,
            &expected,
            crate::SitemapEntityEncoding::Identity,
        );
        if let Some((blocked, payload)) = page_without_grant {
            assert!(payload.is_none());
            assert!(
                blocked
                    .error
                    .as_deref()
                    .unwrap()
                    .contains("resource permission missing")
            );
            assert!(blocked.final_status.is_none());
        }
    }
}

#[tokio::test]
async fn sitemap_entity_bytes_override_file_suffix_and_content_type_without_pdf_parsing() {
    for (path, file_gzip, content_type) in [
        ("/file", true, "application/octet-stream"),
        ("/file.gz", true, "application/pdf"),
        ("/plain.gz", false, "application/gzip"),
        ("/plain.xml", false, "application/pdf"),
    ] {
        let fixture = BinaryFixture::new(move |origin, _| {
            let body = xml(origin);
            BinaryResponse::bytes(if file_gzip { gzip(&body) } else { body }, content_type)
        })
        .await;
        let expected = xml(&fixture.origin);
        let post_http = if file_gzip {
            gzip(&expected)
        } else {
            expected.clone()
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let hook_calls = calls.clone();
        let mut context = context(config(&fixture, true, true));
        context.pdf_parse_hook = Some(Arc::new(move || {
            hook_calls.fetch_add(1, Ordering::SeqCst);
        }));
        let (page, observed) = fetch(&context, &fixture.url(path)).await;
        let final_requests = fixture.finish().await;
        assert_requests(&final_requests, &[(path, 1)]);
        assert_credentials(&final_requests, path, true);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_success(
            &page,
            &observed,
            &post_http,
            &expected,
            if file_gzip {
                crate::SitemapEntityEncoding::GzipFileEntity
            } else {
                crate::SitemapEntityEncoding::Identity
            },
        );
    }
}

#[tokio::test]
async fn http_gzip_is_decoded_by_reqwest_once_before_optional_file_gzip() {
    for file_gzip in [false, true] {
        let fixture = BinaryFixture::new(move |origin, _| {
            let xml = xml(origin);
            let entity = if file_gzip { gzip(&xml) } else { xml };
            BinaryResponse::bytes(gzip(&entity), "application/octet-stream")
                .header("Content-Encoding", "gzip")
        })
        .await;
        let expected = xml(&fixture.origin);
        let post_http = if file_gzip {
            gzip(&expected)
        } else {
            expected.clone()
        };
        let context = context(config(&fixture, false, true));
        let (page, observed) = fetch(&context, &fixture.url("/http-layer")).await;
        let final_requests = fixture.finish().await;
        assert_requests(&final_requests, &[("/http-layer", 1)]);
        assert_success(
            &page,
            &observed,
            &post_http,
            &expected,
            if file_gzip {
                crate::SitemapEntityEncoding::GzipFileEntity
            } else {
                crate::SitemapEntityEncoding::Identity
            },
        );
    }
}

#[tokio::test]
async fn complete_transport_does_not_hide_decoder_utf8_or_late_xml_failure() {
    for (case, code, decoder_retained, parser_retained) in [
        ("crc", "sitemap_invalid_gzip_entity", false, false),
        ("nested", "sitemap_nested_gzip_entity", false, false),
        ("utf8", "sitemap_invalid_utf8", true, false),
        ("xml-tail", "sitemap_invalid_xml", true, true),
    ] {
        let fixture = BinaryFixture::new(move |origin, _| {
            let expected = xml(origin);
            let body = match case {
                "crc" => {
                    let mut body = gzip(&expected);
                    let index = body.len() - 8;
                    body[index] ^= 1;
                    body
                }
                "nested" => gzip(&gzip(&expected)),
                "utf8" => b"<urlset>\xff</urlset>".to_vec(),
                "xml-tail" => format!(
                    "<urlset><url><loc>{origin}/page-sentinel</loc></url><url></wrong></urlset>"
                )
                .into_bytes(),
                _ => unreachable!(),
            };
            BinaryResponse::bytes(body, "application/octet-stream")
        })
        .await;
        let context = context(config(&fixture, true, true));
        let (page, observed) = fetch(&context, &fixture.url("/failing-sitemap")).await;
        let final_requests = fixture.finish().await;
        assert_requests(&final_requests, &[("/failing-sitemap", 1)]);
        assert!(page.error.is_none() && !page.truncated);
        assert_eq!(page.final_status, Some(200));
        assert!(page.pdf.is_none());
        assert!(observed.request_started && observed.body_complete);
        assert!(observed.post_http_entity_sha256.is_some());
        assert_eq!(observed.failure.as_ref().unwrap().code, code);
        assert_eq!(observed.entity_decode.is_some(), decoder_retained);
        assert_eq!(observed.parser.is_some(), parser_retained);
        if let Some(parser) = &observed.parser {
            assert!(!parser.complete);
            assert_eq!(parser.admission_locations().count(), 0);
            assert_eq!(parser.locations.len(), 1);
            assert_eq!(parser.entries.len(), 2);
            assert!(parser.entries[0].complete);
            assert!(!parser.entries[1].complete);
        }
    }
}

#[tokio::test]
async fn role_mismatch_missing_or_invalid_admission_stops_before_any_request_or_robots() {
    for (native_policy, resources) in [(false, false), (false, true), (true, false), (true, true)] {
        let fixture = BinaryFixture::new(|origin, path| {
            if path == "/robots.txt" {
                BinaryResponse::robots(b"User-agent: *\nAllow: /\n".to_vec())
            } else {
                BinaryResponse::bytes(xml(origin), "application/xml")
            }
        })
        .await;
        let mut settings = config(&fixture, native_policy, resources);
        settings.obey_robots = true;
        let context = context(settings);
        let url = fixture.url("/no-request");
        let source_policy =
            crate::CompiledCrawlPolicy::new(&policy_config(&fixture.origin)).unwrap();
        let mut page_rejections = Vec::new();
        let mut protocol_rejections = Vec::new();
        for role in [
            crate::PolicyTaskRole::SitemapDocument,
            crate::PolicyTaskRole::ObservationOnly,
        ] {
            let queue = QueueEntry {
                url: url.clone(),
                depth: 0,
                discovered_from: None,
                admission: Some(admission(&source_policy, &url, role)),
            };
            page_rejections.push(context.document(queue).await);
        }
        for role in [
            crate::PolicyTaskRole::PageDocument,
            crate::PolicyTaskRole::ObservationOnly,
        ] {
            let queue = QueueEntry {
                url: url.clone(),
                depth: 0,
                discovered_from: None,
                admission: Some(admission(&source_policy, &url, role)),
            };
            protocol_rejections.push(context.sitemap_protocol(queue).await);
        }
        if let Some(policy) = &context.policy {
            protocol_rejections.push(
                context
                    .sitemap_protocol(QueueEntry {
                        url: url.clone(),
                        depth: 0,
                        discovered_from: None,
                        admission: None,
                    })
                    .await,
            );
            let mut invalid = admission(policy, &url, crate::PolicyTaskRole::SitemapDocument);
            invalid.policy_sha256 = "untrusted-policy".into();
            protocol_rejections.push(
                context
                    .sitemap_protocol(QueueEntry {
                        url: url.clone(),
                        depth: 0,
                        discovered_from: None,
                        admission: Some(invalid),
                    })
                    .await,
            );
        }
        let final_requests = fixture.finish().await;
        assert_requests(&final_requests, &[]);
        for (page, payload) in page_rejections {
            assert!(page.error.is_some());
            assert!(page.final_status.is_none());
            assert!(payload.is_none());
        }
        for (page, observed) in protocol_rejections {
            assert!(page.error.is_some());
            assert!(!observed.request_started && !observed.body_complete);
            assert_eq!(observed.retained_post_http_entity_bytes, 0);
            assert!(observed.post_http_entity_sha256.is_none());
            assert!(observed.entity_decode.is_none() && observed.parser.is_none());
            assert_eq!(
                observed.failure.unwrap().code,
                "sitemap_request_not_started"
            );
        }
    }
}

#[tokio::test]
async fn external_resource_origin_cannot_grant_sitemap_redirect_permission() {
    for native_policy in [false, true] {
        let external =
            BinaryFixture::new(|origin, _| BinaryResponse::bytes(xml(origin), "application/xml"))
                .await;
        let target = external.url("/external-sitemap");
        let external_origin = external.origin.clone();
        let fixture = BinaryFixture::new(move |_, _| BinaryResponse::redirect(&target)).await;
        let mut settings = config(&fixture, native_policy, true);
        settings
            .resource_discovery
            .as_mut()
            .unwrap()
            .resource_scope
            .external_origins = vec![external_origin];
        let context = context(settings);
        let (page, observed) = fetch(&context, &fixture.url("/redirect")).await;
        let requests = fixture.finish().await;
        let external_requests = external.finish().await;
        assert_requests(&requests, &[("/redirect", 1)]);
        assert_requests(&external_requests, &[]);
        assert_credentials(&requests, "/redirect", true);
        assert_eq!(page.status, Some(302));
        assert!(page.final_status.is_none());
        assert_eq!(page.redirect_chain.len(), 1);
        assert!(page.resource_permission.is_none());
        assert!(observed.request_started && !observed.body_complete);
        assert!(observed.failure.is_some());
        assert!(observed.post_http_entity_sha256.is_none());
        assert!(observed.entity_decode.is_none() && observed.parser.is_none());
        if native_policy {
            assert_eq!(
                page.policy.as_ref().unwrap().redirect_decisions[0]
                    .decision
                    .decision,
                crate::CrawlPolicyDecisionKind::Deny
            );
        }
    }
}

#[tokio::test]
async fn explicit_external_check_follows_sitemap_redirect_without_seed_credentials() {
    for native_policy in [false, true] {
        let external = BinaryFixture::new(|origin, path| {
            if path == "/robots.txt" {
                BinaryResponse::robots(b"User-agent: *\nAllow: /\n".to_vec())
            } else {
                BinaryResponse::bytes(xml(origin), "application/xml")
            }
        })
        .await;
        let target = external.url("/external-sitemap");
        let expected = xml(&external.origin);
        let fixture = BinaryFixture::new(move |_, path| {
            if path == "/robots.txt" {
                BinaryResponse::robots(b"User-agent: *\nAllow: /\n".to_vec())
            } else {
                BinaryResponse::redirect(&target)
            }
        })
        .await;
        let mut settings = config(&fixture, native_policy, true);
        settings.check_external = true;
        settings.obey_robots = true;
        if let Some(policy) = &mut settings.crawl_policy {
            policy.scope.external = crate::OutsideScopePolicy::CheckOnce;
        }
        let context = context(settings);
        let url = fixture.url("/redirect");
        let (page, observed) = fetch(&context, &url).await;
        let requests = fixture.finish().await;
        let external_requests = external.finish().await;
        assert_requests(&requests, &[("/robots.txt", 1), ("/redirect", 1)]);
        assert_requests(
            &external_requests,
            &[("/robots.txt", 1), ("/external-sitemap", 1)],
        );
        assert_credentials(&requests, "/redirect", true);
        assert_credentials(&requests, "/robots.txt", false);
        assert_credentials(&external_requests, "/robots.txt", false);
        assert_credentials(&external_requests, "/external-sitemap", false);
        assert_success(
            &page,
            &observed,
            &expected,
            &expected,
            crate::SitemapEntityEncoding::Identity,
        );
        assert_eq!(page.redirect_chain.len(), 1);
        assert_eq!(observed.initial_status, Some(302));
        assert_eq!(observed.redirect_count, 1);
        assert_eq!(
            observed
                .parser
                .as_ref()
                .unwrap()
                .context
                .requested_document_url
                .as_deref(),
            Some(url.as_str())
        );
    }
}

#[tokio::test]
async fn redirected_sitemap_checks_target_robots_before_send_with_bounded_robots_prefix() {
    for oversized_robots in [false, true] {
        let mut robots = b"User-agent: *\nDisallow: /blocked-sitemap\n# ".to_vec();
        if oversized_robots {
            robots.resize(512 * 1024 + 1, b'x');
        }
        let expected_robots_bytes = robots.len().min(512 * 1024);
        let external = BinaryFixture::new(move |origin, path| {
            if path == "/robots.txt" {
                BinaryResponse::robots(robots.clone())
            } else {
                BinaryResponse::bytes(xml(origin), "application/xml")
            }
        })
        .await;
        let target = external.url("/blocked-sitemap");
        let target_url = Url::parse(&target).unwrap();
        let fixture = BinaryFixture::new(move |_, path| {
            if path == "/robots.txt" {
                BinaryResponse::robots(b"User-agent: *\nAllow: /\n".to_vec())
            } else {
                BinaryResponse::redirect(&target)
            }
        })
        .await;
        let mut settings = config(&fixture, true, true);
        settings.check_external = true;
        settings.obey_robots = true;
        settings.crawl_policy.as_mut().unwrap().scope.external =
            crate::OutsideScopePolicy::CheckOnce;
        let context = context(settings);
        let (page, observed) = fetch(&context, &fixture.url("/redirect")).await;
        let cached = context.robots.get(&target_url).await;
        let requests = fixture.finish().await;
        let external_requests = external.finish().await;
        assert_requests(&requests, &[("/robots.txt", 1), ("/redirect", 1)]);
        assert_requests(&external_requests, &[("/robots.txt", 1)]);
        assert_credentials(&external_requests, "/robots.txt", false);
        assert_eq!(cached.text.len(), expected_robots_bytes);
        assert!(!cached.allowed(&context.config.user_agent, target_url.as_str()));
        // This proves a denial in the retained prefix, not complete observation
        // of a >512-KiB robots source or of all its sitemap declarations.
        assert!(page.blocked_by_robots);
        assert!(page.final_status.is_none());
        assert!(observed.request_started && !observed.body_complete);
        assert_eq!(observed.failure.unwrap().code, "sitemap_robots_denied");
        assert!(observed.post_http_entity_sha256.is_none());
        assert!(observed.entity_decode.is_none() && observed.parser.is_none());
    }
}

#[tokio::test]
async fn sitemap_transport_cap_counts_post_http_decoded_bytes_with_exact_and_plus_one_controls() {
    for (http_gzip, file_gzip, too_small) in [
        (false, false, false),
        (false, false, true),
        (true, false, false),
        (true, false, true),
        (false, true, false),
        (false, true, true),
    ] {
        let fixture = BinaryFixture::new(move |origin, _| {
            let xml = xml(origin);
            let entity = if file_gzip { gzip(&xml) } else { xml };
            if http_gzip {
                BinaryResponse::bytes(gzip(&entity), "application/octet-stream")
                    .header("Content-Encoding", "gzip")
            } else {
                BinaryResponse::bytes(entity, "application/octet-stream")
            }
        })
        .await;
        let expected = xml(&fixture.origin);
        let post_http = if file_gzip {
            gzip(&expected)
        } else {
            expected.clone()
        };
        let cap = post_http.len() - usize::from(too_small);
        let mut settings = config(&fixture, false, true);
        settings.max_body_bytes = cap;
        let context = context(settings);
        let (page, observed) = fetch(&context, &fixture.url("/bounded")).await;
        let requests = fixture.finish().await;
        assert_requests(&requests, &[("/bounded", 1)]);
        assert_eq!(page.size_bytes, cap);
        assert_eq!(observed.retained_post_http_entity_bytes, cap);
        if too_small {
            assert!(page.truncated);
            assert!(!observed.body_complete);
            assert_eq!(
                observed.failure.unwrap().code,
                "sitemap_transport_body_byte_budget"
            );
            assert!(observed.post_http_entity_sha256.is_none());
            assert!(observed.entity_decode.is_none() && observed.parser.is_none());
        } else {
            assert_success(
                &page,
                &observed,
                &post_http,
                &expected,
                if file_gzip {
                    crate::SitemapEntityEncoding::GzipFileEntity
                } else {
                    crate::SitemapEntityEncoding::Identity
                },
            );
        }
    }
}

#[tokio::test]
async fn sitemap_file_gzip_inflation_cap_is_separate_from_small_complete_transport_entity() {
    let xml_cap = crate::SitemapParserLimits::default().max_xml_bytes;
    let mut inflated = b"<urlset/>".to_vec();
    inflated.resize(xml_cap + 1, b' ');
    let compressed = gzip(&inflated);
    let compressed_bytes = compressed.len();
    let compressed_sha = crate::resource_policy::digest(&compressed);
    let fixture = BinaryFixture::new(move |_, _| {
        BinaryResponse::bytes(compressed.clone(), "application/gzip")
    })
    .await;
    let context = context(config(&fixture, true, true));
    let (page, observed) = fetch(&context, &fixture.url("/inflation.gz")).await;
    let requests = fixture.finish().await;
    assert_requests(&requests, &[("/inflation.gz", 1)]);
    assert!(compressed_bytes < context.config.max_body_bytes);
    assert!(!page.truncated);
    assert_eq!(page.size_bytes, compressed_bytes);
    assert!(observed.request_started && observed.body_complete);
    assert_eq!(observed.retained_post_http_entity_bytes, compressed_bytes);
    assert_eq!(
        observed.post_http_entity_sha256.as_deref(),
        Some(compressed_sha.as_str())
    );
    assert_eq!(
        observed.failure.unwrap().code,
        "sitemap_xml_inflation_byte_budget"
    );
    assert!(observed.entity_decode.is_none() && observed.parser.is_none());
    let mut large_config = context.config.clone();
    large_config.max_body_bytes = xml_cap + 1;
    assert_eq!(
        FetchPurpose::SitemapProtocol.body_limit(&large_config),
        crate::SitemapParserLimits::default().max_entity_bytes
    );
    assert_eq!(
        FetchPurpose::PageDocument.body_limit(&large_config),
        large_config.max_body_bytes
    );
}

#[tokio::test]
async fn sitemap_redirect_loop_and_limit_never_fetch_stopped_target() {
    for (loop_case, max_redirects, expected_paths) in [
        (true, 5usize, vec![("/start", 1)]),
        (false, 1usize, vec![("/start", 1), ("/middle", 1)]),
    ] {
        let fixture = BinaryFixture::new(move |origin, path| match path {
            "/start" => BinaryResponse::redirect(if loop_case { "/start" } else { "/middle" }),
            "/middle" => BinaryResponse::redirect("/final"),
            _ => BinaryResponse::bytes(xml(origin), "application/xml"),
        })
        .await;
        let mut settings = config(&fixture, true, true);
        settings.max_redirects = max_redirects;
        let context = context(settings);
        let (page, observed) = fetch(&context, &fixture.url("/start")).await;
        let requests = fixture.finish().await;
        assert_requests(&requests, &expected_paths);
        assert!(page.final_status.is_none());
        assert!(page.error.as_deref().unwrap().contains(if loop_case {
            "Redirect loop"
        } else {
            "Redirect limit"
        }));
        assert!(observed.request_started && !observed.body_complete);
        assert_eq!(
            observed.failure.unwrap().code,
            "sitemap_transport_incomplete"
        );
        assert!(observed.post_http_entity_sha256.is_none());
        assert!(observed.entity_decode.is_none() && observed.parser.is_none());
    }
}

#[tokio::test]
async fn sitemap_non200_and_incomplete_transport_are_distinct_from_xml_failure() {
    for non200 in [false, true] {
        let fixture = BinaryFixture::new(move |origin, _| {
            let mut response = BinaryResponse::bytes(xml(origin), "application/xml");
            if non200 {
                response.status = 403;
            } else {
                response.declared_length = Some(response.body.len() + 7);
            }
            response
        })
        .await;
        let expected = xml(&fixture.origin);
        let context = context(config(&fixture, false, true));
        let (page, observed) = fetch(&context, &fixture.url("/not-a-complete-200")).await;
        let requests = fixture.finish().await;
        assert_requests(&requests, &[("/not-a-complete-200", 1)]);
        assert!(observed.request_started);
        assert!(observed.entity_decode.is_none() && observed.parser.is_none());
        assert_eq!(page.size_bytes, observed.retained_post_http_entity_bytes);
        if non200 {
            assert_eq!(page.final_status, Some(403));
            assert!(page.error.is_none());
            assert!(observed.body_complete);
            assert_eq!(observed.failure.unwrap().code, "sitemap_http_non200");
            assert_eq!(
                observed.post_http_entity_sha256.as_deref(),
                Some(crate::resource_policy::digest(&expected).as_str())
            );
        } else {
            assert_eq!(page.final_status, Some(200));
            assert!(page.error.is_some());
            assert!(!observed.body_complete);
            assert_eq!(
                observed.failure.unwrap().code,
                "sitemap_transport_incomplete"
            );
            assert!(observed.post_http_entity_sha256.is_none());
            assert!(observed.retained_post_http_entity_bytes <= expected.len());
        }
    }
}
