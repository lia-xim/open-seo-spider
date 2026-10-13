//! B12 typed discovery HTTP + actual Store candidates. AUTHORED, NOT COMPILED OR RUN.
//! Requires coherent Root CoreStore + LedgerV5 + source helper and Crawler fragments.
//! Fixtures copied from source-bound Main protocol tests; candidates exercise actual typed methods and Store.
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
    let mut policy = crate::CrawlPolicyConfig::default();
    policy.scope.roots.push(crate::CrawlScopeRoot {
        origin: origin.into(),
        subdomains: false,
        folders: vec!["/".into()],
        exact_paths: vec![],
    });
    policy
}
fn config(fixture: &BinaryFixture, policy: bool) -> CrawlConfig {
    CrawlConfig {
        seed_urls: vec![fixture.url("/seed")],
        sitemap_urls: vec![fixture.url("/task.xml")],
        explicit_sitemap_inputs: Some(vec![fixture.url("/task.xml")]),
        discover_sitemaps: false,
        crawl_links: false,
        delay_ms: 0,
        obey_robots: false,
        concurrency: 1,
        request_timeout_secs: 3,
        crawl_policy: policy.then(|| policy_config(&fixture.origin)),
        custom_headers: BTreeMap::from([
            ("Authorization".into(), "Bearer sitemap-fixture-only".into()),
            ("Cookie".into(), "sitemap_fixture=only".into()),
            ("X-Seed-Secret".into(), "sitemap-fixture-only".into()),
        ]),
        ..Default::default()
    }
}
fn context(crawler: &Crawler, generation: u64) -> FetchContext {
    FetchContext {
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
        policy: crawler.policy.clone(),
        pdf_parse_hook: None,
    }
}
fn setup(
    config: CrawlConfig,
    limits: crate::SitemapLedgerLimits,
    robots: bool,
) -> (tempfile::TempDir, Crawler, u64, DiscoveryEntry) {
    let temp = tempfile::tempdir().unwrap();
    let crawler = Crawler::new(config)
        .unwrap()
        .with_store(temp.path().join("crawl.sqlite"))
        .unwrap();
    let store = crawler.store.as_ref().unwrap();
    store.prepare(&crawler.config).unwrap();
    let generation = store.begin_run("typed-discovery-http-fixture").unwrap();
    let task = DiscoveryEntry {
        kind: DiscoveryKind::Sitemap,
        url: crawler.config.sitemap_urls[0].clone(),
    };
    if let Some(policy) = &crawler.policy {
        let revision = format!(
            "initial:{}:explicit:sitemap_document",
            policy.policy_sha256()
        );
        let mut rows = vec![crate::policy_ledger::decide(
            policy,
            &revision,
            0,
            crate::CandidateSourceKind::SitemapDocument,
            task.url.clone(),
            None,
            None,
            crate::PolicyInputBasis::DeclaredInput,
            crate::PolicyTaskRole::SitemapDocument,
        )];
        store
            .initialize_policy(policy, &mut rows, &crawler.policy_limits, generation)
            .unwrap();
        assert_eq!(
            rows[0].admission_reason, "scheduled",
            "real durable source-task grant required"
        );
    }
    let mut entries = vec![task.clone()];
    if robots {
        entries.push(DiscoveryEntry {
            kind: DiscoveryKind::Robots,
            url: format!("{}/", origin(&crawler.seeds[0])),
        });
    }
    let saved = store.config().unwrap().unwrap();
    let config_sha256 = crate::sitemap_ledger::digest(serde_json::to_vec(&saved).unwrap());
    // This unit fixture bootstraps source storage with no initial Root rows.
    // Initial explicit/import provenance is a separate Root-owned acceptance gate.
    store
        .initialize_sitemap_discovery(
            &entries,
            generation,
            &crate::store::SitemapCatalogInit {
                config_sha256,
                limits,
                roots: vec![],
                legacy_history_unknown: false,
            },
        )
        .unwrap();
    (temp, crawler, generation, task)
}
fn catalog(crawler: &Crawler) -> crate::SitemapCatalog {
    crawler
        .store
        .as_ref()
        .unwrap()
        .sitemap_catalog()
        .unwrap()
        .unwrap()
}
fn locations(
    catalog: &crate::SitemapCatalog,
) -> Vec<&crate::sitemap_ledger::StoredSitemapLocation> {
    catalog
        .rows
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|row| match row {
            crate::SitemapLedgerRow::Location(row) => Some(row),
            _ => None,
        })
        .collect()
}
fn documents(
    catalog: &crate::SitemapCatalog,
) -> Vec<&crate::sitemap_ledger::SitemapSourceDocument> {
    catalog
        .rows
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|row| match row {
            crate::SitemapLedgerRow::Document(row) => Some(row.as_ref()),
            _ => None,
        })
        .collect()
}
fn roots(catalog: &crate::SitemapCatalog) -> Vec<&crate::SitemapRootDeclaration> {
    catalog
        .rows
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|row| match row {
            crate::SitemapLedgerRow::Root(row) => Some(row),
            _ => None,
        })
        .collect()
}
fn varied_xml(origin: &str) -> Vec<u8> {
    format!("<urlset><url/><url><loc>  {origin}/page?a=1&amp;b=2  </loc></url><url><loc>{origin}/page?a=1&amp;b=2</loc></url><url><loc>ftp://example.test/no</loc></url></urlset>").into_bytes()
}
async fn run(
    crawler: &Crawler,
    generation: u64,
    seen: &mut HashSet<String>,
) -> anyhow::Result<DiscoveryProgress> {
    crawler
        .run_discovery(&CrawlControl::new(), &None, seen, (0, 0), Some(generation))
        .await
}

#[tokio::test]
async fn ordinary_original_missing_duplicate_invalid_loc_survive_http_and_store() {
    use crate::sitemap_ledger::SitemapAdmissionDecision as D;
    let fixture = BinaryFixture::new(|origin, path| {
        if path == "/task.xml" {
            BinaryResponse::bytes(varied_xml(origin), "application/xml")
        } else {
            BinaryResponse::bytes(Vec::new(), "text/plain")
        }
    })
    .await;
    let (_temp, crawler, generation, _) = setup(config(&fixture, false), Default::default(), false);
    let result = run(&crawler, generation, &mut HashSet::new())
        .await
        .unwrap();
    assert_eq!(result.pages.len(), 1);
    assert_eq!(result.pending, 0);
    let cat = catalog(&crawler);
    let loc = locations(&cat);
    assert_eq!(
        loc.iter()
            .map(|row| row.observed.ordinal)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(
        loc.iter()
            .map(|row| row.admission.decision.clone())
            .collect::<Vec<_>>(),
        vec![
            D::MissingLocation,
            D::Scheduled,
            D::Coalesced,
            D::InvalidLocation
        ]
    );
    assert_eq!(
        loc[1].observed.decoded_value.as_deref(),
        Some(format!("  {}/page?a=1&b=2  ", fixture.origin).as_str())
    );
    assert_ne!(loc[1].occurrence_id, loc[2].occurrence_id);
    assert_eq!(
        loc[1].admission.membership_url,
        loc[2].admission.membership_url
    );
    assert!(
        loc.iter()
            .all(|row| row.admission.policy_occurrence_id.is_none())
    );
    assert!(!cat.summary.declared_source_cohort_complete);
    assert!(documents(&cat)[0].complete());
    let requests = fixture.finish().await;
    assert_requests(&requests, &[("/task.xml", 1)]);
    assert_credentials(&requests, "/task.xml", true);
}

#[tokio::test]
async fn policy_uses_original_one_based_loc_ordinals_and_committed_sql_bindings() {
    use crate::sitemap_ledger::SitemapAdmissionDecision as D;
    let fixture = BinaryFixture::new(|origin, _| {
        BinaryResponse::bytes(varied_xml(origin), "application/xml")
    })
    .await;
    let (_temp, crawler, generation, _) = setup(config(&fixture, true), Default::default(), false);
    let result = run(&crawler, generation, &mut HashSet::new())
        .await
        .unwrap();
    assert_eq!(result.pages.len(), 1);
    assert!(result.pages[0].admission.is_some());
    let cat = catalog(&crawler);
    let loc = locations(&cat);
    assert_eq!(loc[1].observed.ordinal, 2);
    assert_eq!(loc[2].observed.ordinal, 3);
    assert_eq!(loc[1].admission.decision, D::Scheduled);
    assert_eq!(loc[2].admission.decision, D::Coalesced);
    assert!(loc[0].admission.policy_occurrence_id.is_none());
    assert!(loc[3].admission.policy_occurrence_id.is_none());
    assert_eq!(
        loc[1].observed.decoded_value.as_deref(),
        Some(format!("  {}/page?a=1&b=2  ", fixture.origin).as_str())
    );
    assert!(loc[1].admission.policy_occurrence_id.is_some());
    assert_eq!(
        loc[2].admission.coalesced_policy_occurrence_id,
        loc[1].admission.policy_occurrence_id
    );
    let page = loc[1].admission.admitted_fetch_url.as_ref().unwrap();
    assert!(
        crawler
            .store
            .as_ref()
            .unwrap()
            .task_policy(crate::PolicyTaskRole::PageDocument, page)
            .unwrap()
            .is_some()
    );
    assert!(!cat.summary.declared_source_cohort_complete);
    assert_requests(&fixture.finish().await, &[("/task.xml", 1)]);
}

#[tokio::test]
async fn robots_raw_zero_based_directives_use_strict_inputs_and_sql_root_finalizer() {
    use crate::sitemap_ledger::SitemapAdmissionDecision as D;
    let fixture = BinaryFixture::new(|origin, path| {
        if path == "/robots.txt" {
            BinaryResponse::robots(
                format!("Sitemap:\r\nSitemap: /relative.xml\r\nSitemap: {origin}/bot.xml\r\n")
                    .into_bytes(),
            )
        } else {
            BinaryResponse::bytes(b"<urlset/>".to_vec(), "application/xml")
        }
    })
    .await;
    let (_temp, crawler, generation, _) = setup(config(&fixture, true), Default::default(), true);
    let result = run(&crawler, generation, &mut HashSet::new())
        .await
        .unwrap();
    assert_eq!(result.pending, 0);
    let cat = catalog(&crawler);
    let roots = roots(&cat);
    assert_eq!(
        roots.iter().map(|r| r.source_ordinal).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(roots[0].admission.decision, D::InvalidLocation);
    assert_eq!(roots[1].admission.decision, D::InvalidLocation);
    assert!(roots[0].admission.policy_occurrence_id.is_none());
    assert!(roots[1].admission.policy_occurrence_id.is_none());
    assert_eq!(roots[2].raw_value, format!(" {}/bot.xml", fixture.origin));
    assert_eq!(roots[2].admission.decision, D::Scheduled);
    assert!(roots[2].admission.policy_occurrence_id.is_some());
    assert_eq!(
        roots[2].admission.membership_url,
        Some(fixture.url("/bot.xml"))
    );
    assert!(!cat.summary.declared_source_cohort_complete);
    assert_requests(
        &fixture.finish().await,
        &[("/task.xml", 1), ("/robots.txt", 1), ("/bot.xml", 1)],
    );
}

#[tokio::test]
async fn robots_partial_content_true_eof_never_admits_complete_source() {
    use crate::sitemap_ledger::SitemapAdmissionDecision as D;
    let fixture = BinaryFixture::new(|origin, path| {
        if path == "/robots.txt" {
            let mut response =
                BinaryResponse::robots(format!("Sitemap: {origin}/blocked.xml\n").into_bytes());
            response.status = 206;
            response
        } else {
            BinaryResponse::bytes(b"<urlset/>".to_vec(), "application/xml")
        }
    })
    .await;
    let (_temp, crawler, generation, _) = setup(config(&fixture, true), Default::default(), true);
    let result = run(&crawler, generation, &mut HashSet::new())
        .await
        .unwrap();
    assert_eq!(result.pending, 0);
    let cat = catalog(&crawler);
    let roots = roots(&cat);
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].admission.decision, D::IncompleteSource);
    assert!(roots[0].admission.policy_occurrence_id.is_none());
    let doc = documents(&cat)
        .into_iter()
        .find(|d| d.kind == crate::SitemapSourceKind::RobotsDirectives)
        .unwrap();
    assert!(doc.transport.as_ref().unwrap().body_complete);
    assert!(!doc.complete());
    assert_eq!(doc.transport.as_ref().unwrap().final_status, Some(206));
    assert_requests(
        &fixture.finish().await,
        &[("/task.xml", 1), ("/robots.txt", 1)],
    );
}

#[tokio::test]
async fn corrupt_file_gzip_footer_is_durable_incomplete_source_without_targets() {
    let fixture = BinaryFixture::new(|origin, _| {
        let mut body = gzip(&xml(origin));
        let index = body.len() - 8;
        body[index] ^= 1;
        BinaryResponse::bytes(body, "application/octet-stream")
    })
    .await;
    let (_temp, crawler, generation, _) = setup(config(&fixture, false), Default::default(), false);
    let result = run(&crawler, generation, &mut HashSet::new())
        .await
        .unwrap();
    assert!(result.pages.is_empty());
    let cat = catalog(&crawler);
    let doc = documents(&cat)[0];
    assert!(!doc.complete());
    assert!(doc.transport.as_ref().unwrap().body_complete);
    assert!(doc.failure_code.is_some());
    assert!(locations(&cat).is_empty());
    assert!(
        crawler
            .store
            .as_ref()
            .unwrap()
            .discovery_complete()
            .unwrap()
    );
    assert!(!cat.summary.declared_source_cohort_complete);
    assert_requests(&fixture.finish().await, &[("/task.xml", 1)]);
}

#[tokio::test]
async fn file_gzip_index_self_cycle_and_duplicate_child_are_fetched_once() {
    use crate::sitemap_ledger::SitemapAdmissionDecision as D;
    let fixture=BinaryFixture::new(|origin,path|if path=="/task.xml" {
        BinaryResponse::bytes(gzip(format!("<sitemapindex><sitemap><loc>{origin}/task.xml</loc></sitemap><sitemap><loc>{origin}/child.xml</loc></sitemap><sitemap><loc>{origin}/child.xml</loc></sitemap></sitemapindex>").as_bytes()),"text/plain")
    }else{BinaryResponse::bytes(xml(origin),"application/xml")}).await;
    let (_temp, crawler, generation, _) = setup(config(&fixture, false), Default::default(), false);
    let result = run(&crawler, generation, &mut HashSet::new())
        .await
        .unwrap();
    assert_eq!(result.pages.len(), 1);
    let cat = catalog(&crawler);
    let loc = locations(&cat);
    assert_eq!(
        loc.iter()
            .map(|r| r.admission.decision.clone())
            .collect::<Vec<_>>(),
        vec![D::Coalesced, D::Scheduled, D::Coalesced, D::Scheduled]
    );
    assert_eq!(
        loc[0].referenced_sitemap_task,
        Some(fixture.url("/task.xml"))
    );
    assert_eq!(documents(&cat).len(), 2);
    assert!(!cat.summary.declared_source_cohort_complete);
    assert_requests(
        &fixture.finish().await,
        &[("/task.xml", 1), ("/child.xml", 1)],
    );
}

#[tokio::test]
async fn invalid_task_and_external_preflight_preserve_no_request_facts() {
    let fixture =
        BinaryFixture::new(|origin, _| BinaryResponse::bytes(xml(origin), "application/xml")).await;
    let (_temp, crawler, generation, _) = setup(config(&fixture, false), Default::default(), false);
    let ctx = context(&crawler, generation);
    for task in [
        "not-an-absolute-url".to_owned(),
        "http://example.invalid:12345/external.xml".into(),
    ] {
        let entry = DiscoveryEntry {
            kind: DiscoveryKind::Sitemap,
            url: task,
        };
        let output = crawler.discover_unit_observed(&entry, &ctx).await.unwrap();
        let batch = output.sitemap_batch.unwrap();
        assert!(!batch.document.complete());
        let transport = batch.document.transport.unwrap();
        assert!(!transport.request_started);
        assert!(transport.requested_url.is_none());
        assert!(transport.final_url.is_none());
        assert!(output.pages.is_empty());
        assert!(output.children.is_empty());
        assert!(output.policy_rows.is_empty());
    }
    assert_requests(&fixture.finish().await, &[]);
}

#[tokio::test]
async fn external_protocol_permission_preserves_origin_bound_credentials() {
    let target =
        BinaryFixture::new(|origin, _| BinaryResponse::bytes(xml(origin), "application/xml")).await;
    let seed = BinaryFixture::new(|_, _| BinaryResponse::redirect("/unused")).await;
    let mut cfg = config(&seed, false);
    cfg.sitemap_urls = vec![target.url("/outside.xml")];
    cfg.explicit_sitemap_inputs = Some(cfg.sitemap_urls.clone());
    let (_temp, denied, generation, entry) = setup(cfg.clone(), Default::default(), false);
    let rejected = denied
        .discover_unit_observed(&entry, &context(&denied, generation))
        .await
        .unwrap();
    assert!(
        !rejected
            .sitemap_batch
            .unwrap()
            .document
            .transport
            .unwrap()
            .request_started
    );
    cfg.check_external = true;
    let (_other, allowed, generation, entry) = setup(cfg, Default::default(), false);
    let mut output = allowed
        .discover_unit_observed(&entry, &context(&allowed, generation))
        .await
        .unwrap();
    allowed
        .finalize_ordinary_protocol(&mut output, &HashSet::new(), &HashSet::new())
        .unwrap();
    assert!(output.pages.is_empty());
    assert!(output.children.is_empty());
    let batch = output.sitemap_batch.unwrap();
    assert!(batch.document.complete());
    assert_eq!(
        batch.locations[0].admission.decision,
        crate::SitemapAdmissionDecision::SourceCheckOnly
    );
    assert!(batch.locations[0].admission.admitted_fetch_url.is_none());
    let requests = target.finish().await;
    assert_requests(&requests, &[("/outside.xml", 1)]);
    assert_credentials(&requests, "/outside.xml", false);
    assert_requests(&seed.finish().await, &[]);
}

#[tokio::test]
async fn cancelled_before_source_keeps_pending_task_and_zero_http() {
    let fixture =
        BinaryFixture::new(|origin, _| BinaryResponse::bytes(xml(origin), "application/xml")).await;
    let (_temp, crawler, generation, task) =
        setup(config(&fixture, false), Default::default(), false);
    let control = CrawlControl::new();
    assert!(control.request_cancel());
    let result = crawler
        .run_discovery(
            &control,
            &None,
            &mut HashSet::new(),
            (0, 0),
            Some(generation),
        )
        .await
        .unwrap();
    assert_eq!(result.pending, 1);
    assert_eq!(
        crawler
            .store
            .as_ref()
            .unwrap()
            .discovery_entries(true)
            .unwrap()[0]
            .url,
        task.url
    );
    let cat = catalog(&crawler);
    assert!(cat.rows.as_ref().unwrap().is_empty());
    assert!(!cat.summary.declared_source_cohort_complete);
    assert_requests(&fixture.finish().await, &[]);
}

#[tokio::test]
async fn source_row_budget_failure_keeps_task_pending_and_global_dedupe_unmodified() {
    let fixture =
        BinaryFixture::new(|origin, _| BinaryResponse::bytes(xml(origin), "application/xml")).await;
    let limits = crate::SitemapLedgerLimits {
        max_rows: 1,
        ..Default::default()
    };
    let (_temp, crawler, generation, task) = setup(config(&fixture, false), limits, false);
    let marker = "https://example.test/preexisting".to_owned();
    let mut seen = HashSet::from([marker]);
    let before = seen.clone();
    let error = run(&crawler, generation, &mut seen)
        .await
        .err()
        .expect("row budget must fail");
    assert!(
        format!("{error:#}").contains("entire source remains pending"),
        "{error:#}"
    );
    assert_eq!(seen, before);
    let cat = catalog(&crawler);
    assert!(cat.rows.as_ref().unwrap().is_empty());
    assert_eq!(
        crawler
            .store
            .as_ref()
            .unwrap()
            .discovery_entries(true)
            .unwrap()[0]
            .url,
        task.url
    );
    assert!(
        !crawler
            .store
            .as_ref()
            .unwrap()
            .discovery_complete()
            .unwrap()
    );
    assert_requests(&fixture.finish().await, &[("/task.xml", 1)]);
}
