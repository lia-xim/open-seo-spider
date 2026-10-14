//! B12 full Crawler bootstrap/report candidate tests. AUTHORED, NOT COMPILED OR RUN.
//! Requires Root coherent initial-policy/bootstrap/report/selected + frozen typed source patches.
//! HTTP fixture instrumentation from frozen typed-unit tests; candidates invoke actual full run_controlled.
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
async fn website() -> BinaryFixture {
    BinaryFixture::new(|origin,path|match path {
        "/task.xml"=>BinaryResponse::redirect(&format!("{origin}/canonical.xml")),
        "/canonical.xml"=>BinaryResponse::bytes(gzip(&xml(origin)),"application/octet-stream"),
        _=>BinaryResponse::bytes(format!("<html><head><title>fixture {path}</title><link rel='canonical' href='{origin}{path}'></head><body>fixture</body></html>").into_bytes(),"text/html"),
    }).await
}
fn config(fixture: &BinaryFixture, policy: bool) -> CrawlConfig {
    let mut config = CrawlConfig {
        seed_urls: vec![fixture.url("/seed")],
        sitemap_urls: vec![],
        explicit_sitemap_inputs: None,
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
        ..Default::default()
    };
    if policy {
        let mut compiled = crate::CrawlPolicyConfig::default();
        compiled.scope.roots.push(crate::CrawlScopeRoot {
            origin: fixture.origin.clone(),
            subdomains: false,
            folders: vec!["/".into()],
            exact_paths: vec![],
        });
        config.crawl_policy = Some(compiled);
    }
    config
}
fn overlapping_config(fixture: &BinaryFixture, policy: bool) -> CrawlConfig {
    let mut cfg = config(fixture, policy);
    cfg.sitemap_urls = vec![
        format!("  {}#first  ", fixture.url("/task.xml")),
        format!("{}#second", fixture.url("/task.xml")),
    ];
    let input = format!(
        "<sitemapindex xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><sitemap><loc>{}/task.xml#third</loc></sitemap><sitemap><loc>{}/task.xml#fourth</loc></sitemap></sitemapindex>",
        fixture.origin, fixture.origin
    );
    cfg.input_imports.push(
        crate::import_url_list(
            input.as_bytes(),
            &crate::ListImportOptions {
                format: crate::ListImportFormat::Xml,
                limits: Default::default(),
            },
        )
        .unwrap(),
    );
    cfg
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
fn documents(catalog: &crate::SitemapCatalog) -> Vec<&crate::SitemapSourceDocument> {
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
fn assert_checkpoint(crawler: &Crawler, result: &ControlledCrawlResult) {
    let store = crawler.store.as_ref().unwrap();
    let inspected = store.inspection().unwrap();
    assert!(inspected.checkpoint_valid);
    assert_eq!(result.outcome, CrawlOutcome::Completed);
    let checkpoint = result.checkpoint.as_ref().unwrap();
    assert_eq!(inspected.checkpoint.as_ref(), Some(checkpoint));
    assert_eq!(
        checkpoint.sitemap_ledger,
        result
            .report
            .sitemap_catalog
            .as_ref()
            .map(|c| c.summary.clone())
    );
    assert_eq!(inspected.sitemap_ledger, checkpoint.sitemap_ledger);
    assert_eq!(
        store.sitemap_catalog().unwrap(),
        result.report.sitemap_catalog
    );
    let saved = store.load_report().unwrap().unwrap();
    assert_eq!(saved.sitemap_catalog, result.report.sitemap_catalog);
    assert_eq!(
        checkpoint.report_sha256,
        crate::sitemap_ledger::digest(serde_json::to_vec(&result.report).unwrap())
    );
    assert_eq!(
        checkpoint.config_sha256,
        crate::sitemap_ledger::digest(serde_json::to_vec(&result.report.config).unwrap())
    );
    assert!(inspected.discovery_complete);
    assert_eq!(inspected.discovery_pending, 0);
}
fn assert_original_roots(
    catalog: &crate::SitemapCatalog,
    cfg: &CrawlConfig,
    fixture: &BinaryFixture,
) {
    let rows = roots(catalog);
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].kind, crate::SitemapRootKind::ExplicitConfig);
    assert_eq!(rows[1].kind, crate::SitemapRootKind::ExplicitConfig);
    let original = cfg
        .explicit_sitemap_inputs
        .as_ref()
        .expect("fresh original capture");
    assert_eq!(original.len(), 2);
    assert_eq!((rows[0].source_ordinal, rows[1].source_ordinal), (0, 1));
    assert_eq!(rows[0].raw_value, original[0]);
    assert_eq!(rows[1].raw_value, original[1]);
    let imported = cfg
        .input_imports
        .iter()
        .flat_map(|input| input.rows.iter())
        .filter(|row| row.candidate_kind == crate::ListCandidateKind::SitemapDocument)
        .collect::<Vec<_>>();
    assert_eq!(imported.len(), 2);
    for (ordinal, (root, input)) in rows[2..].iter().zip(imported).enumerate() {
        assert_eq!(root.kind, crate::SitemapRootKind::ImportedSitemapRow);
        assert_eq!(root.source_ordinal, ordinal as u64);
        assert_eq!(root.raw_value, input.raw_value);
        assert_eq!(root.normalized_document_url, input.normalized_url);
    }
    assert_eq!(
        rows.iter()
            .map(|r| &r.declaration_id)
            .collect::<HashSet<_>>()
            .len(),
        4
    );
    for root in rows {
        assert_eq!(root.normalized_document_url, Some(fixture.url("/task.xml")));
        assert_eq!(root.admission.membership_url, root.normalized_document_url);
        assert_eq!(
            root.admission.admitted_fetch_url,
            root.normalized_document_url
        );
    }
    assert!(!catalog.summary.declared_source_cohort_complete);
    assert!(!catalog.summary.legacy_history_unknown);
    let docs = documents(catalog);
    assert_eq!(docs.len(), 1);
    assert!(docs[0].complete());
    assert_eq!(docs[0].task_url, fixture.url("/task.xml"));
    let transport = docs[0].transport.as_ref().unwrap();
    assert_eq!(transport.requested_url, Some(fixture.url("/task.xml")));
    assert_eq!(transport.final_url, Some(fixture.url("/canonical.xml")));
    assert_eq!(transport.initial_status, Some(302));
    assert_eq!(transport.final_status, Some(200));
}
fn policy_rows(crawler: &Crawler) -> Vec<crate::StoredPolicyOccurrence> {
    let page = crawler
        .store
        .as_ref()
        .unwrap()
        .policy_occurrences(None, 128)
        .unwrap();
    assert!(!page.has_more);
    page.rows
}
fn policy_payload_bytes(path: &Path, id: &str) -> Vec<u8> {
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    connection.busy_timeout(Duration::from_secs(5)).unwrap();
    let length: i64 = connection
        .query_row(
            "SELECT length(CAST(data AS BLOB)) FROM policy_occurrences WHERE occurrence_id=?1",
            [id],
            |row| row.get(0),
        )
        .unwrap();
    assert!((0..=1024 * 1024).contains(&length));
    connection
        .query_row(
            "SELECT CAST(data AS BLOB) FROM policy_occurrences WHERE occurrence_id=?1",
            [id],
            |row| row.get(0),
        )
        .unwrap()
}
fn assert_root_sql(crawler: &Crawler, catalog: &crate::SitemapCatalog, generation: u64) {
    let rows = policy_rows(crawler);
    let initial = rows
        .iter()
        .filter(|r| {
            r.occurrence.task_role == crate::PolicyTaskRole::SitemapDocument
                && r.occurrence.input_basis == crate::PolicyInputBasis::DeclaredInput
        })
        .collect::<Vec<_>>();
    assert_eq!(
        initial.len(),
        4,
        "duplicates and explicit/import overlap must be durable inputs"
    );
    for root in roots(catalog) {
        let id = root
            .admission
            .policy_occurrence_id
            .as_ref()
            .expect("actual policy binding");
        let stored = initial
            .iter()
            .find(|r| &r.occurrence.decision.occurrence_id == id)
            .unwrap();
        assert_eq!(stored.generation, generation);
        assert_eq!(stored.occurrence.decision.raw_value, root.raw_value);
        assert!(stored.occurrence.raw_value_observed);
        assert_eq!(
            root.admission.policy_source_revision.as_ref(),
            Some(&stored.occurrence.source_revision)
        );
        assert!(matches!(
            stored.occurrence.admission_reason.as_str(),
            "scheduled" | "coalesced"
        ));
        if root.kind == crate::SitemapRootKind::ExplicitConfig {
            assert_eq!(
                stored.occurrence.source_ordinal,
                root.source_ordinal as usize
            );
        }
    }
    assert_eq!(
        initial
            .iter()
            .filter(|r| r.occurrence.admission_reason == "scheduled")
            .count(),
        1
    );
    assert_eq!(
        initial
            .iter()
            .filter(|r| r.occurrence.admission_reason == "coalesced")
            .count(),
        3
    );
}
async fn full(crawler: &Crawler) -> ControlledCrawlResult {
    crawler
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap()
}
fn prepare_selection(
    report: &CrawlReport,
    checkpoint: Option<&crate::CrawlCheckpoint>,
    target: &str,
) -> crate::PreparedSelectedRecrawl {
    let bytes = serde_json::to_vec(report).unwrap();
    crate::prepare_selected_recrawl(
        crate::SelectedRecrawlParent {
            job_id: "bootstrap-source-parent".into(),
            checkpoint_revision: checkpoint.map(|c| c.revision),
            source_kind: crate::RecrawlSourceKind::CapturedCheckpoint,
        },
        &bytes,
        &[target.into(), target.into()],
        None,
    )
    .unwrap()
}

#[tokio::test]
async fn fresh_ordinary_full_crawl_retains_original_raw_duplicates_and_import_overlap() {
    use crate::SitemapAdmissionDecision as D;
    let fixture = website().await;
    let cfg = overlapping_config(&fixture, false);
    let originals = cfg.sitemap_urls.clone();
    let temp = tempfile::tempdir().unwrap();
    let crawler = Crawler::new(cfg)
        .unwrap()
        .with_store(temp.path().join("crawl.sqlite"))
        .unwrap();
    let result = full(&crawler).await;
    assert_checkpoint(&crawler, &result);
    assert_eq!(
        result.report.config.explicit_sitemap_inputs.as_ref(),
        Some(&originals)
    );
    let cat = result.report.sitemap_catalog.as_ref().unwrap();
    assert_original_roots(cat, &result.report.config, &fixture);
    let roots = roots(cat);
    assert_eq!(roots[0].admission.decision, D::Scheduled);
    assert!(roots[1..].iter().all(
        |r| r.admission.decision == D::Coalesced && r.admission.policy_occurrence_id.is_none()
    ));
    assert_eq!(result.report.pages.len(), 2);
    assert_eq!(result.report.summary.pending, 0);
    let requests = fixture.finish().await;
    assert_requests(
        &requests,
        &[
            ("/seed", 1),
            ("/task.xml", 1),
            ("/canonical.xml", 1),
            ("/page-sentinel", 1),
        ],
    );
    for path in ["/seed", "/task.xml", "/canonical.xml", "/page-sentinel"] {
        assert_credentials(&requests, path, true);
    }
}

/// Actual fresh constructor, HTTP and durable writer control. The larger roots
/// establish a feasible CSV candidate; this is not a native CSV-cap acceptance.
#[tokio::test]
async fn budget_calibration_fresh_quoted_fragments_preserve_originals_and_coalesce() {
    use crate::SitemapAdmissionDecision as D;
    let fixture = BinaryFixture::new(|_, path| match path {
        "/empty.xml" => BinaryResponse::bytes(b"<urlset/>".to_vec(), "application/xml"),
        "/seed" => BinaryResponse::bytes(b"<html><title>seed</title></html>".to_vec(), "text/html"),
        _ => panic!("unexpected calibration request: {path}"),
    })
    .await;
    let task_url = fixture.url("/empty.xml");
    let originals = [128usize, 129, 122_000]
        .map(|quotes| format!("{task_url}#{}", "\"".repeat(quotes)))
        .to_vec();
    for original in &originals {
        assert_eq!(
            crate::normalize_url(original, None).unwrap().as_str(),
            task_url
        );
    }
    let mut cfg = config(&fixture, false);
    cfg.sitemap_urls = originals.clone();
    assert!(cfg.explicit_sitemap_inputs.is_none());
    let temp = tempfile::tempdir().unwrap();
    let crawler = Crawler::new(cfg)
        .unwrap()
        .with_store(temp.path().join("crawl.sqlite"))
        .unwrap();
    let result = full(&crawler).await;
    assert_checkpoint(&crawler, &result);
    assert_eq!(
        result.report.config.explicit_sitemap_inputs.as_ref(),
        Some(&originals)
    );
    let catalog = result.report.sitemap_catalog.as_ref().unwrap();
    let retained = roots(catalog);
    assert_eq!(retained.len(), originals.len());
    for (ordinal, (row, original)) in retained.iter().zip(&originals).enumerate() {
        assert_eq!(row.raw_value, *original);
        assert_eq!(row.source_ordinal, ordinal as u64);
        assert_eq!(
            row.normalized_document_url.as_deref(),
            Some(task_url.as_str())
        );
        assert_eq!(
            row.admission.admitted_fetch_url.as_deref(),
            Some(task_url.as_str())
        );
        assert_eq!(
            row.admission.decision,
            if ordinal == 0 {
                D::Scheduled
            } else {
                D::Coalesced
            }
        );
        let bytes = serde_json::to_vec(&crate::SitemapLedgerRow::Root((*row).clone()))
            .unwrap()
            .len();
        assert!(bytes < crate::SitemapLedgerLimits::default().max_row_json_bytes);
        eprintln!(
            "BUDGET_CALIBRATION_ROOT ordinal={ordinal} quotes={} serialized_row_bytes={bytes}",
            original.matches('"').count()
        );
    }
    assert_eq!(documents(catalog).len(), 1);
    assert_requests(&fixture.finish().await, &[("/seed", 1), ("/empty.xml", 1)]);
}

#[tokio::test]
async fn fresh_policy_full_crawl_retains_every_original_sitemap_policy_input() {
    let fixture = website().await;
    let temp = tempfile::tempdir().unwrap();
    let crawler = Crawler::new(overlapping_config(&fixture, true))
        .unwrap()
        .with_store(temp.path().join("crawl.sqlite"))
        .unwrap();
    let result = full(&crawler).await;
    assert_checkpoint(&crawler, &result);
    let cat = result.report.sitemap_catalog.as_ref().unwrap();
    assert_original_roots(cat, &result.report.config, &fixture);
    assert_root_sql(&crawler, cat, 1);
    assert_eq!(result.report.pages.len(), 2);
    assert!(result.report.pages.iter().all(|p| p.policy.is_some()));
    assert_requests(
        &fixture.finish().await,
        &[
            ("/seed", 1),
            ("/task.xml", 1),
            ("/canonical.xml", 1),
            ("/page-sentinel", 1),
        ],
    );
}

#[tokio::test]
async fn known_empty_sources_are_distinct_from_saved_legacy_unknown_and_memory() {
    let fixture = website().await;
    for known in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let cfg = config(&fixture, false);
        let crawler = if known {
            Crawler::new(cfg)
        } else {
            Crawler::from_saved_config(cfg)
        }
        .unwrap()
        .with_store(temp.path().join("crawl.sqlite"))
        .unwrap();
        let result = full(&crawler).await;
        assert_checkpoint(&crawler, &result);
        assert_eq!(result.report.pages.len(), 1);
        if known {
            let cat = result.report.sitemap_catalog.as_ref().unwrap();
            assert_eq!(result.report.config.explicit_sitemap_inputs, Some(vec![]));
            assert!(cat.rows.as_ref().unwrap().is_empty());
            assert!(!cat.summary.legacy_history_unknown);
            assert!(!cat.summary.declared_source_cohort_complete);
        } else {
            assert!(result.report.config.explicit_sitemap_inputs.is_none());
            assert!(result.report.sitemap_catalog.is_none());
        }
    }
    let memory = Crawler::new(config(&fixture, false)).unwrap();
    let result = full(&memory).await;
    assert!(result.report.sitemap_catalog.is_none());
    assert!(result.checkpoint.is_none());
    assert_requests(&fixture.finish().await, &[("/seed", 3)]);
}

#[tokio::test]
async fn legacy_normalized_sitemap_config_never_fabricates_original_root_history() {
    let fixture = website().await;
    for policy in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let mut cfg = overlapping_config(&fixture, policy);
        cfg.sitemap_urls = vec![fixture.url("/task.xml")];
        cfg.explicit_sitemap_inputs = None;
        let crawler = Crawler::from_saved_config(cfg)
            .unwrap()
            .with_store(temp.path().join("legacy.sqlite"))
            .unwrap();
        let result = full(&crawler).await;
        assert_checkpoint(&crawler, &result);
        assert!(result.report.config.explicit_sitemap_inputs.is_none());
        assert!(result.report.sitemap_catalog.is_none());
        assert_eq!(result.report.pages.len(), 2);
        assert!(
            crawler
                .store
                .as_ref()
                .unwrap()
                .sitemap_catalog()
                .unwrap()
                .is_none()
        );
    }
    assert_requests(
        &fixture.finish().await,
        &[
            ("/seed", 2),
            ("/task.xml", 2),
            ("/canonical.xml", 2),
            ("/page-sentinel", 2),
        ],
    );
}

#[tokio::test]
async fn interrupted_initial_policy_generation_one_bootstraps_original_roots_in_run_two() {
    let fixture = website().await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("interrupted.sqlite");
    let first = Crawler::new(overlapping_config(&fixture, true))
        .unwrap()
        .with_store(&path)
        .unwrap();
    let store = first.store.as_ref().unwrap();
    store.prepare(&first.config).unwrap();
    let generation = store
        .begin_run("interrupted-after-policy-before-sitemap-bootstrap")
        .unwrap();
    assert_eq!(generation, 1);
    let policy = first.policy.as_ref().unwrap();
    let mut rows = first.initial_policy_rows(policy);
    store
        .initialize_policy(policy, &mut rows, &first.policy_limits, generation)
        .unwrap();
    assert!(store.sitemap_catalog().unwrap().is_none());
    let prior = policy_rows(&first);
    assert_eq!(
        prior
            .iter()
            .filter(|r| r.occurrence.task_role == crate::PolicyTaskRole::SitemapDocument)
            .count(),
        4
    );
    let prior_payloads = prior
        .iter()
        .map(|row| {
            let id = row.occurrence.decision.occurrence_id.clone();
            let bytes = policy_payload_bytes(&path, &id);
            (id, bytes)
        })
        .collect::<BTreeMap<_, _>>();
    let saved = store.config().unwrap().unwrap();
    drop(first);
    let second = Crawler::from_saved_config(saved)
        .unwrap()
        .with_store(&path)
        .unwrap();
    let result = full(&second).await;
    assert_checkpoint(&second, &result);
    assert_eq!(result.checkpoint.as_ref().unwrap().generation, 2);
    let cat = result.report.sitemap_catalog.as_ref().unwrap();
    assert_original_roots(cat, &result.report.config, &fixture);
    assert_root_sql(&second, cat, 1);
    let docs = documents(cat);
    assert_eq!(docs[0].generation, 2);
    let after = policy_rows(&second);
    for row in prior {
        let current = after
            .iter()
            .find(|r| r.occurrence.decision.occurrence_id == row.occurrence.decision.occurrence_id)
            .unwrap();
        assert_eq!(current, &row);
    }
    for (id, bytes) in prior_payloads {
        assert_eq!(
            policy_payload_bytes(&path, &id),
            bytes,
            "actual SQL BLOB payload changed: {id}"
        );
    }
    assert_requests(
        &fixture.finish().await,
        &[
            ("/seed", 1),
            ("/task.xml", 1),
            ("/canonical.xml", 1),
            ("/page-sentinel", 1),
        ],
    );
}

#[tokio::test]
async fn operational_resume_retains_first_catalog_lineage_and_once_only_roots() {
    let fixture = website().await;
    for policy in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("resume.sqlite");
        let first = Crawler::new(overlapping_config(&fixture, policy))
            .unwrap()
            .with_store(&path)
            .unwrap();
        let original = full(&first).await;
        assert_checkpoint(&first, &original);
        let cat = original.report.sitemap_catalog.clone().unwrap();
        let mut saved = first.store.as_ref().unwrap().config().unwrap().unwrap();
        saved.max_pages = Some(saved.max_pages.unwrap_or(100_000).saturating_add(1));
        saved.concurrency = 2;
        saved.delay_ms = 1;
        saved.request_timeout_secs = 4;
        drop(first);
        let second = Crawler::from_saved_config(saved)
            .unwrap()
            .with_store(&path)
            .unwrap();
        let resumed = full(&second).await;
        assert_checkpoint(&second, &resumed);
        assert_eq!(resumed.checkpoint.as_ref().unwrap().generation, 2);
        assert_eq!(resumed.report.sitemap_catalog.as_ref(), Some(&cat));
        assert_eq!(roots(&cat).len(), 4);
        assert_eq!(
            cat.summary.config_sha256,
            original.checkpoint.as_ref().unwrap().config_sha256
        );
        assert_ne!(
            resumed.checkpoint.as_ref().unwrap().config_sha256,
            cat.summary.config_sha256
        );
        if policy {
            assert_root_sql(&second, &cat, 1);
        }
    }
    assert_requests(
        &fixture.finish().await,
        &[
            ("/seed", 2),
            ("/task.xml", 2),
            ("/canonical.xml", 2),
            ("/page-sentinel", 2),
        ],
    );
}

#[tokio::test]
async fn selected_store_and_memory_inherit_exact_rows_without_new_sitemap_sources() {
    let fixture = website().await;
    let parent_temp = tempfile::tempdir().unwrap();
    let parent = Crawler::new(overlapping_config(&fixture, false))
        .unwrap()
        .with_store(parent_temp.path().join("parent.sqlite"))
        .unwrap();
    let base = full(&parent).await;
    assert_checkpoint(&parent, &base);
    let original = base.report.sitemap_catalog.clone().unwrap();
    let prepared = prepare_selection(
        &base.report,
        base.checkpoint.as_ref(),
        &fixture.url("/page-sentinel"),
    );
    assert_eq!(prepared.plan.selection.len(), 2);
    assert_eq!(prepared.plan.targets.len(), 1);
    let expected_parent_sha = prepared.plan.parent.snapshot_sha256.clone();
    let child_temp = tempfile::tempdir().unwrap();
    let child = Crawler::from_saved_config(prepared.config.clone())
        .unwrap()
        .with_store(child_temp.path().join("child.sqlite"))
        .unwrap()
        .with_selected_recrawl(prepared.clone())
        .unwrap();
    let persisted = full(&child).await;
    assert_checkpoint(&child, &persisted);
    assert_eq!(persisted.report.pages.len(), 2);
    let memory = Crawler::from_saved_config(prepared.config.clone())
        .unwrap()
        .with_selected_recrawl(prepared)
        .unwrap();
    let transient = full(&memory).await;
    assert!(transient.checkpoint.is_none());
    assert_eq!(transient.report.pages.len(), 2);
    for result in [&persisted, &transient] {
        let inherited = result.report.sitemap_catalog.as_ref().unwrap();
        assert_eq!(inherited.rows, original.rows);
        assert_eq!(
            inherited.summary.rolling_sha256,
            original.summary.rolling_sha256
        );
        assert!(!inherited.summary.declared_source_cohort_complete);
        let basis = inherited.inherited.as_ref().unwrap();
        assert_eq!(basis.parent_snapshot_sha256, expected_parent_sha);
        assert!(!basis.current_discovery_executed);
        assert!(result.report.operation.is_some());
        assert_eq!(result.discovery_pending, 0);
    }
    assert_requests(
        &fixture.finish().await,
        &[
            ("/seed", 1),
            ("/task.xml", 1),
            ("/canonical.xml", 1),
            ("/page-sentinel", 3),
        ],
    );
}

#[tokio::test]
async fn selected_legacy_parent_preserves_unavailable_catalog_in_store_and_memory() {
    let fixture = website().await;
    let parent_temp = tempfile::tempdir().unwrap();
    let parent = Crawler::from_saved_config(config(&fixture, false))
        .unwrap()
        .with_store(parent_temp.path().join("parent.sqlite"))
        .unwrap();
    let base = full(&parent).await;
    assert_checkpoint(&parent, &base);
    assert!(base.report.sitemap_catalog.is_none());
    let prepared = prepare_selection(
        &base.report,
        base.checkpoint.as_ref(),
        &fixture.url("/seed"),
    );
    let child_temp = tempfile::tempdir().unwrap();
    let child = Crawler::from_saved_config(prepared.config.clone())
        .unwrap()
        .with_store(child_temp.path().join("child.sqlite"))
        .unwrap()
        .with_selected_recrawl(prepared.clone())
        .unwrap();
    let persisted = full(&child).await;
    assert_checkpoint(&child, &persisted);
    assert!(persisted.report.sitemap_catalog.is_none());
    let memory = Crawler::from_saved_config(prepared.config.clone())
        .unwrap()
        .with_selected_recrawl(prepared)
        .unwrap();
    let transient = full(&memory).await;
    assert!(transient.report.sitemap_catalog.is_none());
    assert!(transient.checkpoint.is_none());
    assert!(persisted.report.config.explicit_sitemap_inputs.is_none());
    assert!(transient.report.config.explicit_sitemap_inputs.is_none());
    assert_requests(&fixture.finish().await, &[("/seed", 3)]);
}
