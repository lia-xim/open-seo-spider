//! AUTHORED / NOT COMPILED / NOT RUN under compute HOLD.
//! Positive contexts/rows/session capabilities come from actual public Crawler.
use super::*;
use open_seo_spider_core::{CrawlConfig, CrawlControl, CrawlStore, Crawler, SitemapFindingsStop};
use rusqlite::Connection;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct OwnedDir(std::path::PathBuf);
impl OwnedDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("oss-s6-adapter-owned-{}", Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for OwnedDir {
    fn drop(&mut self) {
        if self.0.parent() == Some(std::env::temp_dir().as_path())
            && self
                .0
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("oss-s6-adapter-owned-"))
        {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
struct Fixture {
    _temp: OwnedDir,
    path: std::path::PathBuf,
    origin: String,
    hits: Arc<AtomicUsize>,
    requests: usize,
    stop: tokio::sync::oneshot::Sender<()>,
    join: tokio::task::JoinHandle<()>,
}
impl Fixture {
    async fn actual() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let served = origin.clone();
        let hits = Arc::new(AtomicUsize::new(0));
        let requests = hits.clone();
        let (stop, mut stopping) = tokio::sync::oneshot::channel();
        let join = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _=&mut stopping=>break,
                    accepted=listener.accept()=>{
                        let (mut socket,_)=accepted.unwrap();
                        let mut bytes=vec![];
                        let mut chunk=[0;2048];
                        tokio::time::timeout(std::time::Duration::from_secs(5),async {
                            while !bytes.windows(4).any(|w|w==b"\r\n\r\n") {
                                let count=socket.read(&mut chunk).await.unwrap();
                                if count==0 {break;}
                                bytes.extend_from_slice(&chunk[..count]);
                                assert!(bytes.len()<=65_536);
                            }
                        }).await.unwrap();
                        let request=String::from_utf8(bytes).unwrap();
                        let route=request.lines().next().unwrap().split_whitespace().nth(1).unwrap();
                        requests.fetch_add(1,Ordering::SeqCst);
                        let (mime,body)=if route=="/source.xml" {
                            ("application/xml",format!("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>{served}/p0011</loc></url></urlset>"))
                        } else {("text/html","<html><head><title>Actual finding</title></head><body>Captured &quot;data&quot;</body></html>".into())};
                        let header=format!("HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
                        socket.write_all(header.as_bytes()).await.unwrap();
                        socket.write_all(body.as_bytes()).await.unwrap();
                        socket.shutdown().await.unwrap();
                    }
                }
            }
        });
        let directory = OwnedDir::new();
        let path = directory.0.join("actual-findings.sqlite");
        let config = CrawlConfig {
            seed_urls: (0..12).map(|i| format!("{origin}/p{i:04}")).collect(),
            sitemap_urls: vec![format!("{origin}/source.xml")],
            obey_robots: false,
            discover_sitemaps: false,
            crawl_links: false,
            concurrency: 1,
            delay_ms: 0,
            ..Default::default()
        };
        let crawler = Crawler::new(config).unwrap().with_store(&path).unwrap();
        let completed = crawler
            .run_controlled(None, CrawlControl::new())
            .await
            .unwrap();
        assert_eq!(
            completed.outcome,
            open_seo_spider_core::CrawlOutcome::Completed
        );
        assert_eq!(completed.report.pages.len(), 12);
        drop(crawler);
        let count = hits.load(Ordering::SeqCst);
        Self {
            _temp: directory,
            path,
            origin,
            hits,
            requests: count,
            stop,
            join,
        }
    }
    fn store(&self) -> CrawlStore {
        CrawlStore::open_read_only(&self.path).unwrap()
    }
    fn job(&self) -> AccessTarget {
        let snapshot = self.store().sitemap_read_snapshot().unwrap();
        AccessTarget::Job {
            job_id: Uuid::new_v4(),
            generation: snapshot.generation.unwrap(),
            config_sha256: snapshot.crawl_config_sha256.unwrap(),
        }
    }
    fn page(&self, query: ReadQuery, target: &AccessTarget) -> FindingsEnvelope {
        let prepared = query.prepare(target).unwrap();
        self.store()
            .with_verified_sitemap_findings(
                prepared.selection(),
                prepared.request().expected_context_sha256.as_deref(),
                &Default::default(),
                |session| {
                    let page = session.page(prepared.request())?;
                    Ok(bind_page(target, &prepared, page)?)
                },
            )
            .unwrap()
    }
    async fn finish(self) {
        assert_eq!(
            self.hits.load(Ordering::SeqCst),
            self.requests,
            "Adapter reads must not fetch website"
        );
        let _ = self.stop.send(());
        tokio::time::timeout(std::time::Duration::from_secs(10), self.join)
            .await
            .unwrap()
            .unwrap();
    }
}
fn source_query(assessment: SitemapFindingAssessmentSelection) -> ReadQuery {
    ReadQuery {
        filter: SitemapFindingFilter::XmlSitemapOver50mb,
        assessment,
        ..Default::default()
    }
}
fn export_query(envelope: &FindingsEnvelope, selection: &SitemapFindingSelection) -> CsvQuery {
    CsvQuery {
        filter: selection.filter,
        assessment: selection.assessment,
        requested_url: selection.requested_url.clone(),
        source_task_url: selection.source_task_url.clone(),
        search: selection.search.clone(),
        snapshot: envelope.export_token.clone().unwrap(),
    }
}

// Independent limited RFC4180 decoder for assertions, not the production writer.
fn csv_rows(bytes: &[u8]) -> Vec<Vec<String>> {
    let mut result = vec![];
    let mut row = vec![];
    let mut field = vec![];
    let mut index = 0;
    let mut quoted = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if quoted {
            if byte == b'"' && bytes.get(index + 1) == Some(&b'"') {
                field.push(b'"');
                index += 1;
            } else if byte == b'"' {
                quoted = false;
            } else {
                field.push(byte);
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b',' => row.push(String::from_utf8(std::mem::take(&mut field)).unwrap()),
                b'\r' => {
                    assert_eq!(bytes.get(index + 1), Some(&b'\n'));
                    row.push(String::from_utf8(std::mem::take(&mut field)).unwrap());
                    result.push(std::mem::take(&mut row));
                    index += 1;
                }
                _ => panic!("Expected quoted RFC4180 field"),
            }
        }
        index += 1;
    }
    assert!(!quoted && field.is_empty() && row.is_empty());
    result
}

#[test]
fn finding_queries_strict_shape_domains_and_large_literal_selectors_preserve_core_contract() {
    for bad in [
        r#"{"unknown":1}"#,
        r#"{"limit":1,"limit":2}"#,
        r#"{"limit":"1"}"#,
        r#"{"assessment":"maybe"}"#,
        r#"{"filter":"raw"}"#,
    ] {
        assert!(serde_json::from_str::<ReadQuery>(bad).is_err(), "{bad}");
    }
    let wrong: ReadQuery = serde_json::from_str(r#"{"source_task_url":"literal"}"#).unwrap();
    assert_eq!(status(&wrong.validate().unwrap_err()), 400);
    let literal = format!("legacy-prefix{}", "x".repeat(8193));
    let prepared = ReadQuery {
        source_task_url: Some(literal.clone()),
        ..source_query(SitemapFindingAssessmentSelection::All)
    }
    .prepare(&AccessTarget::Offline)
    .unwrap();
    assert_eq!(
        prepared.selection().source_task_url.as_deref(),
        Some(literal.as_str())
    );
    for assessment in [
        SitemapFindingAssessmentSelection::Match,
        SitemapFindingAssessmentSelection::NotMatched,
        SitemapFindingAssessmentSelection::Unknown,
        SitemapFindingAssessmentSelection::All,
    ] {
        source_query(assessment)
            .prepare(&AccessTarget::Offline)
            .unwrap();
    }
    for (limit, candidates) in [(0, 1), (201, 1), (1, 0), (1, 5001)] {
        assert!(
            ReadQuery {
                limit: Some(limit),
                candidate_limit: Some(candidates),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
}

#[tokio::test]
async fn finding_actual_sparse_page_continuations_keep_query_hash_while_limits_change() {
    let f = Fixture::actual().await;
    let target = f.job();
    let mut cursor = None;
    let mut pages = 0;
    let mut matches = vec![];
    let mut hash = None;
    loop {
        let query = ReadQuery {
            requested_url: Some(format!("{}/p0011", f.origin)),
            limit: Some(if pages % 2 == 0 { 1 } else { 10 }),
            candidate_limit: Some(2),
            cursor,
            ..Default::default()
        };
        let response = f.page(query, &target);
        let page = response.findings.as_ref().unwrap();
        if let Some(hash) = &hash {
            assert_eq!(&page.context.selection_sha256, hash);
        } else {
            hash = Some(page.context.selection_sha256.clone());
        }
        if page.rows.as_ref().unwrap().is_empty() && !page.scan.exhausted {
            assert_eq!(page.scan.stop, SitemapFindingsStop::CandidateBudget);
            assert_eq!(page.scan.more_matches, None);
            assert!(response.next_cursor.is_some());
        }
        matches.extend(
            page.rows
                .as_ref()
                .unwrap()
                .iter()
                .map(|r| r.position.clone()),
        );
        cursor = response.next_cursor;
        pages += 1;
        if cursor.is_none() {
            assert!(page.scan.exhausted);
            break;
        }
        assert!(pages < 10);
    }
    assert_eq!(matches.len(), 1);
    assert!(pages > 1);
    f.finish().await;
}

#[tokio::test]
async fn finding_actual_page_export_raw_roles_and_job_offline_target_are_distinct() {
    let f = Fixture::actual().await;
    let target = f.job();
    let response = f.page(
        ReadQuery {
            limit: Some(1),
            ..Default::default()
        },
        &target,
    );
    // Source fixture lists one saved page, so source-All supplies a real export
    // token; page row role is additionally obtained from all12 page candidates.
    let many = f.page(
        ReadQuery {
            assessment: SitemapFindingAssessmentSelection::All,
            limit: Some(1),
            ..Default::default()
        },
        &target,
    );
    let cursor = many.next_cursor.clone().unwrap();
    assert_eq!(
        status(
            &ReadQuery {
                cursor: response.export_token.clone(),
                ..Default::default()
            }
            .validate()
            .unwrap_err()
        ),
        400
    );
    let export = CsvQuery {
        snapshot: cursor.clone(),
        filter: SitemapFindingFilter::UrlsInSitemap,
        assessment: SitemapFindingAssessmentSelection::All,
        requested_url: None,
        source_task_url: None,
        search: None,
    };
    assert_eq!(status(&export.validate().unwrap_err()), 400);
    let changed = ReadQuery {
        filter: SitemapFindingFilter::OrphanUrls,
        assessment: SitemapFindingAssessmentSelection::All,
        cursor: Some(cursor.clone()),
        ..Default::default()
    }
    .prepare(&target)
    .err()
    .unwrap();
    assert_eq!(status(&changed), 409);
    let offline = ReadQuery {
        assessment: SitemapFindingAssessmentSelection::All,
        cursor: Some(cursor.clone()),
        ..Default::default()
    }
    .prepare(&AccessTarget::Offline)
    .err()
    .unwrap();
    assert_eq!(status(&offline), 409);
    let mut raw: serde_json::Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(&cursor).unwrap()).unwrap();
    raw["payload"]["role"] = "raw_export".into();
    assert_eq!(
        status(
            &ReadQuery {
                cursor: Some(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&raw).unwrap())),
                ..Default::default()
            }
            .validate()
            .unwrap_err()
        ),
        400
    );
    assert!(
        !String::from_utf8(URL_SAFE_NO_PAD.decode(cursor).unwrap())
            .unwrap()
            .contains("p000")
    );
    f.finish().await;
}

#[tokio::test]
async fn finding_actual_shared_csv_two_pass_schema_full_payload_and_readonly_receipts_match() {
    let f = Fixture::actual().await;
    let target = AccessTarget::Offline;
    let query = source_query(SitemapFindingAssessmentSelection::All);
    let prepared = query.prepare(&target).unwrap();
    let response = f.page(
        source_query(SitemapFindingAssessmentSelection::All),
        &target,
    );
    let csv = export_query(&response, prepared.selection())
        .prepare(&target)
        .unwrap();
    let before = Sha256::digest(std::fs::read(&f.path).unwrap());
    let (first, second, snapshot, count, expected) = f
        .store()
        .with_verified_sitemap_findings(
            csv.selection(),
            Some(csv.expected_context_sha256()),
            &Default::default(),
            |session| {
                csv.validate_session(session)?;
                let snapshot = csv.bind_session(session)?;
                let mut first = vec![];
                let count = write_findings_csv_session(session, &mut first)?;
                let mut second = vec![];
                let mut observed = 0u64;
                assert_eq!(
                    count,
                    write_findings_csv_session_observed(session, &mut second, || observed += 1)?
                );
                assert_eq!(observed, count, "Context/header are not data-row progress");
                let mut expected = vec![];
                session.visit_all(|row| {
                    expected.push(serde_json::to_value(row)?);
                    Ok(())
                })?;
                Ok((first, second, snapshot, count, expected))
            },
        )
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(snapshot.access_target, AccessTarget::Offline);
    let rows = csv_rows(&first);
    assert_eq!(rows.len() as u64, count + 2);
    assert!(rows.iter().all(|row| row.len() == 10));
    assert_eq!(rows[0][0], "record_type");
    assert_eq!(rows[1][0], "context");
    let context: serde_json::Value = serde_json::from_str(&rows[1][9]).unwrap();
    assert_eq!(
        context["context"]["context_sha256"],
        snapshot.core_context_sha256
    );
    for (row, expected) in rows[2..].iter().zip(expected) {
        assert_eq!(row[0], "finding");
        assert!(row[9].is_empty());
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&row[8]).unwrap(),
            expected
        );
        assert!(row[5].starts_with("http://"));
    }
    assert_eq!(before, Sha256::digest(std::fs::read(&f.path).unwrap()));
    f.finish().await;
}

// Explicit historical compatibility/restoration control, not a new producer or
// fabricated positive measurement: remove only optional Source history from an
// actual terminal crawl and rebind the actual rewritten report byte seal.
fn historical_source(path: &std::path::Path) {
    let mut connection = Connection::open(path).unwrap();
    let tx = connection.transaction().unwrap();
    tx.execute("DELETE FROM sitemap_source_measurement_commits", [])
        .unwrap();
    tx.execute("DELETE FROM sitemap_source_measurements", [])
        .unwrap();
    tx.execute("DELETE FROM metadata WHERE key='sitemap_source_store'", [])
        .unwrap();
    let raw: String = tx
        .query_row("SELECT value FROM metadata WHERE key='report'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let checkpoint: String = tx
        .query_row(
            "SELECT value FROM metadata WHERE key='checkpoint'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut report: open_seo_spider_core::CrawlReport = serde_json::from_str(&raw).unwrap();
    let mut checkpoint: open_seo_spider_core::CrawlCheckpoint =
        serde_json::from_str(&checkpoint).unwrap();
    report.sitemap_source_measurements = None;
    checkpoint.sitemap_source_measurements = None;
    let raw = serde_json::to_string(&report).unwrap();
    checkpoint.report_sha256 = format!("{:x}", Sha256::digest(raw.as_bytes()));
    tx.execute("UPDATE metadata SET value=?1 WHERE key='report'", [raw])
        .unwrap();
    tx.execute(
        "UPDATE metadata SET value=?1 WHERE key='checkpoint'",
        [serde_json::to_string(&checkpoint).unwrap()],
    )
    .unwrap();
    tx.commit().unwrap();
}

#[tokio::test]
async fn finding_actual_unknown_history_zero_selected_rows_exports_one_scope_record_without_fake_finding()
 {
    let f = Fixture::actual().await;
    historical_source(&f.path);
    let target = AccessTarget::Offline;
    let response = f.page(
        source_query(SitemapFindingAssessmentSelection::Match),
        &target,
    );
    let page = response.findings.as_ref().unwrap();
    assert_eq!(
        page.context.availability,
        SitemapFindingsAvailability::AvailableWithUnknownHistory
    );
    assert!(page.rows.as_ref().unwrap().is_empty() && page.scan.exhausted);
    assert!(page.context.source_scope.as_ref().unwrap().unknown_history);
    let query = source_query(SitemapFindingAssessmentSelection::Match)
        .prepare(&target)
        .unwrap();
    let csv = export_query(&response, query.selection())
        .prepare(&target)
        .unwrap();
    let mut bytes = vec![];
    f.store()
        .with_verified_sitemap_findings(
            csv.selection(),
            Some(csv.expected_context_sha256()),
            &Default::default(),
            |session| {
                csv.bind_session(session)?;
                assert_eq!(write_findings_csv_session(session, &mut bytes)?, 0);
                Ok(())
            },
        )
        .unwrap();
    let rows = csv_rows(&bytes);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1][0], "context");
    assert!(rows[1][9].contains("available_with_unknown_history"));
    f.finish().await;
}

#[tokio::test]
async fn finding_actual_unavailable_core_omits_export_and_rejects_csv_before_header() {
    let f = Fixture::actual().await;
    // Actual saved-crawl checkpoint absence is an unavailable terminal analysis.
    Connection::open(&f.path)
        .unwrap()
        .execute("DELETE FROM metadata WHERE key='checkpoint'", [])
        .unwrap();
    let target = AccessTarget::Offline;
    let prepared = ReadQuery::default().prepare(&target).unwrap();
    f.store()
        .with_verified_sitemap_findings(
            prepared.selection(),
            None,
            &Default::default(),
            |session| {
                assert_eq!(
                    session.context().availability,
                    SitemapFindingsAvailability::Unavailable
                );
                let response = bind_page(&target, &prepared, session.page(prepared.request())?)?;
                assert!(response.findings.as_ref().unwrap().rows.is_none());
                assert!(response.export_token.is_none() && response.next_cursor.is_none());
                let mut writer = vec![];
                let error = write_findings_csv_session(session, &mut writer).unwrap_err();
                assert_eq!(status(&typed_failure(error)), 409);
                assert!(writer.is_empty());
                Ok(())
            },
        )
        .unwrap();
    let response = unavailable_page(&target).unwrap();
    assert!(response.findings.is_none() && response.export_token.is_none());
    assert_eq!(
        response.availability_reason,
        Some("saved_sqlite_unavailable")
    );
    f.finish().await;
}

#[tokio::test]
async fn finding_actual_stale_context_and_app_sql_mismatch_fail_without_writer_output() {
    let f = Fixture::actual().await;
    let target = f.job();
    let response = f.page(
        source_query(SitemapFindingAssessmentSelection::All),
        &target,
    );
    let selected = source_query(SitemapFindingAssessmentSelection::All)
        .prepare(&target)
        .unwrap();
    let mut other = target.clone();
    if let AccessTarget::Job { generation, .. } = &mut other {
        *generation += 1;
    }
    let error = export_query(&response, selected.selection())
        .prepare(&other)
        .err()
        .unwrap();
    assert_eq!(status(&error), 409);
    let prepared = source_query(SitemapFindingAssessmentSelection::All)
        .prepare(&other)
        .unwrap();
    let error = f
        .store()
        .with_verified_sitemap_findings(
            prepared.selection(),
            None,
            &Default::default(),
            |session| {
                Ok(bind_page(
                    &other,
                    &prepared,
                    session.page(prepared.request())?,
                )?)
            },
        )
        .err()
        .unwrap();
    assert_eq!(status(&typed_failure(error)), 409);
    // Change actual task context through an explicit legacy-restoration fixture.
    let csv = export_query(&response, selected.selection())
        .prepare(&target)
        .unwrap();
    Connection::open(&f.path)
        .unwrap()
        .execute(
            "INSERT INTO discovery_frontier(kind,url,done) VALUES('sitemap',?1,0)",
            [format!("{}/new-task.xml", f.origin)],
        )
        .unwrap();
    let mut called = false;
    let error = f
        .store()
        .with_verified_sitemap_findings(
            csv.selection(),
            Some(csv.expected_context_sha256()),
            &Default::default(),
            |_| {
                called = true;
                Ok(())
            },
        )
        .unwrap_err();
    assert!(!called);
    assert_eq!(status(&typed_failure(error)), 409);
    f.finish().await;
}

#[tokio::test]
async fn finding_actual_csv_preserves_typed_writer_causes_and_cancel_not_all_io_is_413() {
    struct Failing(io::ErrorKind);
    impl Write for Failing {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(self.0, "actual writer fault"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let f = Fixture::actual().await;
    f.store()
        .with_verified_sitemap_findings(
            &source_query(SitemapFindingAssessmentSelection::All)
                .prepare(&AccessTarget::Offline)
                .unwrap()
                .request
                .selection,
            None,
            &Default::default(),
            |session| {
                let error = write_findings_csv_session(
                    session,
                    &mut Failing(io::ErrorKind::PermissionDenied),
                )
                .unwrap_err();
                assert_eq!(status(&typed_failure(error)), 500);
                Ok(())
            },
        )
        .unwrap();
    let failure = typed_failure(anyhow::Error::new(io::Error::other(Failure::new(
        "sitemap_findings_csv_byte_budget",
        "actual outer sink cap",
    ))));
    assert_eq!(status(&failure), 413);
    assert_eq!(
        status(&typed_failure(SitemapReadControlError::Cancelled.into())),
        503
    );
    assert_eq!(
        status(&typed_failure(
            SitemapReadControlError::DeadlineExceeded.into()
        )),
        504
    );
    f.finish().await;
}

#[test]
fn finding_csv_human_formula_prefix_does_not_rewrite_full_typed_machine_payload() {
    // Format-only negative, explicitly not a verified finding/Store fixture.
    for value in ["=SUM(1)", " +1", "\u{feff}@command", "-1"] {
        let mut bytes = vec![];
        text(&mut bytes, &open_seo_spider_core::csv_text(value)).unwrap();
        comma(&mut bytes).unwrap();
        json_field(&mut bytes, &value).unwrap();
        bytes.extend_from_slice(b"\r\n");
        let rows = csv_rows(&bytes);
        assert!(rows[0][0].starts_with('\''));
        assert_eq!(serde_json::from_str::<String>(&rows[0][1]).unwrap(), value);
    }
}

#[test]
fn finding_streaming_json_and_csv_resource_boundaries_are_inclusive_without_materialized_rows() {
    // Primitive sink controls only: these are NOT physical8MiB/64MiB receipts.
    let mut sink = JsonSink {
        bytes: 0,
        max: 3,
        exceeded: false,
        hash: Sha256::new(),
    };
    sink.write_all(b"123").unwrap();
    assert!(sink.write_all(b"4").is_err() && sink.exceeded);
    let mut output = vec![];
    let mut sink = CsvBounded {
        writer: &mut output,
        bytes: MAX_CSV_BYTES - 3,
        exceeded: false,
    };
    sink.write_all(b"123").unwrap();
    assert!(sink.write_all(b"4").is_err() && sink.exceeded);
    assert_eq!(output, b"123");
    assert_eq!(MAX_REQUEST_BYTES, 64 * 1024 * 1024);
    assert_eq!(
        status(&typed_failure(
            SitemapFindingsError::ResponseByteBudget.into()
        )),
        413
    );
}
