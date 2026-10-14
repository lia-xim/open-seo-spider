//! AUTHORED / NOT_COMPILED / NOT_RUN under the coordinated compute HOLD.
//! All positive stores come from the real public Crawler, HTTP and parser path.
use super::*;
use crate::sitemap_coverage::verified_read::{CoverageReadContext, project_read_with_control};
use crate::sitemap_ledger::verified_read::verify_raw_with_control;
use crate::sitemap_source_store::verified_read::verify_source_with_control;
use crate::{CrawlConfig, CrawlControl, CrawlStore, Crawler, ResourceAnalysisStatus};
use lopdf::{
    Document, EncryptionState, EncryptionVersion, Permissions, Stream, dictionary, text_string,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[derive(Clone, Copy)]
enum Mode {
    Large,
    Pdf,
    Disallow,
    RobotsFailure,
}
struct Site {
    origin: String,
    mode: Mode,
    hits: Arc<AtomicUsize>,
    stop: tokio::sync::oneshot::Sender<()>,
    join: tokio::task::JoinHandle<()>,
}
fn pdf(pages_count: usize, encrypted: bool) -> Vec<u8> {
    let mut document = Document::with_version("1.7");
    let pages = document.new_object_id();
    let font = document.add_object(dictionary! {"Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>"Helvetica", "Encoding"=>"WinAnsiEncoding"});
    let content = document.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf (Actual S6 fixture text) Tj ET".to_vec(),
    ));
    let mut kids: Vec<lopdf::Object> = vec![];
    for _ in 0..pages_count {
        let page = document.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages, "MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()], "Contents"=>content, "Resources"=>dictionary! {"Font"=>dictionary! {"F1"=>font}}});
        kids.push(page.into());
    }
    document.set_object(
        pages,
        dictionary! {"Type"=>"Pages", "Kids"=>kids, "Count"=>pages_count as i64},
    );
    let catalog = document.add_object(dictionary! {"Type"=>"Catalog", "Pages"=>pages});
    document.trailer.set("Root", catalog);
    if encrypted {
        document.trailer.set(
            "ID",
            vec![
                text_string("S6OwnedFixture01"),
                text_string("S6OwnedFixture01"),
            ],
        );
        let state = EncryptionState::try_from(EncryptionVersion::V2 {
            document: &document,
            owner_password: "owned-fixture-owner",
            user_password: "owned-fixture-user",
            key_length: 128,
            permissions: Permissions::all(),
        })
        .unwrap();
        document.encrypt(&state).unwrap();
    }
    let mut bytes = vec![];
    document.save_to(&mut bytes).unwrap();
    bytes
}
impl Site {
    async fn start(mode: Mode) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let prefix = origin.clone();
        let hits = Arc::new(AtomicUsize::new(0));
        let seen = hits.clone();
        // These are actual authored PDF producers, evaluated only if the test runs.
        let complete = Arc::new(if matches!(mode, Mode::Pdf) {
            pdf(1, false)
        } else {
            vec![]
        });
        let partial = Arc::new(if matches!(mode, Mode::Pdf) {
            pdf(2, false)
        } else {
            vec![]
        });
        let encrypted = Arc::new(if matches!(mode, Mode::Pdf) {
            pdf(1, true)
        } else {
            vec![]
        });
        let (stop, mut stopping) = tokio::sync::oneshot::channel();
        let join = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    _ = &mut stopping => break,
                    accepted = listener.accept() => {
                        let (mut socket, _) = accepted.unwrap();
                        let prefix = prefix.clone();
                        let seen = seen.clone();
                        let complete = complete.clone();
                        let partial = partial.clone();
                        let encrypted = encrypted.clone();
                        connections.spawn(async move {
                            seen.fetch_add(1, Ordering::SeqCst);
                            let mut request = vec![];
                            let mut buffer = [0; 2048];
                            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                                while !request.windows(4).any(|v| v == b"\r\n\r\n") {
                                    let n = socket.read(&mut buffer).await.unwrap();
                                    if n == 0 { return; }
                                    request.extend_from_slice(&buffer[..n]);
                                    assert!(request.len() <= 65536);
                                }
                            }).await.unwrap();
                            let request = String::from_utf8(request).unwrap();
                            let route = request.lines().next().unwrap().split_whitespace().nth(1).unwrap();
                            let (status, mime, body) = match route {
                                "/robots.txt" if matches!(mode, Mode::RobotsFailure) => ("500 Internal Server Error", "text/plain", b"Actual policy unavailable".to_vec()),
                                "/robots.txt" => ("200 OK", "text/plain", b"User-agent: *\nDisallow: /blocked\n".to_vec()),
                                "/source.xml" if matches!(mode, Mode::Large) => ("200 OK", "application/xml", format!("<sitemapindex><sitemap><loc>{prefix}/a.xml</loc></sitemap><sitemap><loc>{prefix}/b.xml</loc></sitemap></sitemapindex>").into_bytes()),
                                "/a.xml" => ("200 OK", "application/xml", format!("<urlset><url><loc>{prefix}/p0250</loc></url><url><loc>{prefix}/p0450</loc></url><url><loc>{prefix}/p0450</loc></url></urlset>").into_bytes()),
                                "/b.xml" => ("200 OK", "application/xml", format!("<urlset><url><loc>{prefix}/p0450</loc></url></urlset>").into_bytes()),
                                "/source.xml" => {
                                    let paths: &[&str] = if matches!(mode, Mode::Pdf) { &["/normal", "/complete.pdf", "/partial.pdf", "/error.pdf", "/encrypted.pdf"] } else { &["/blocked"] };
                                    let rows = paths.iter().map(|path| format!("<url><loc>{prefix}{path}</loc></url>")).collect::<String>();
                                    ("200 OK", "application/xml", format!("<urlset>{rows}</urlset>").into_bytes())
                                }
                                "/complete.pdf" => ("200 OK", "application/pdf; fixture=complete", complete.as_ref().clone()),
                                "/partial.pdf" => ("200 OK", "application/pdf; fixture=partial", partial.as_ref().clone()),
                                "/encrypted.pdf" => ("200 OK", "application/pdf", encrypted.as_ref().clone()),
                                "/error.pdf" => ("200 OK", "application/pdf", b"%PDF-1.7\nActual malformed fixture.".to_vec()),
                                "/p0000" => ("200 OK", "text/html", b"<html><body><a href='/p0250'>Nonself</a></body></html>".to_vec()),
                                "/p0450" => ("200 OK", "text/html", b"<html><body><a href='/p0450'>Self only</a></body></html>".to_vec()),
                                _ => ("200 OK", "text/html", b"<html><head><title>Actual S6 page</title></head><body>Owned fixture</body></html>".to_vec()),
                            };
                            let headers = format!("HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                            socket.write_all(headers.as_bytes()).await.unwrap();
                            socket.write_all(&body).await.unwrap();
                            let _ = socket.shutdown().await;
                        });
                    }
                }
            }
            while let Some(result) = connections.join_next().await {
                result.unwrap();
            }
        });
        Self {
            origin,
            mode,
            hits,
            stop,
            join,
        }
    }
    fn config(&self) -> CrawlConfig {
        let paths: Vec<String> = match self.mode {
            Mode::Large => (0..500).map(|n| format!("/p{n:04}")).collect(),
            Mode::Pdf => [
                "/normal",
                "/complete.pdf",
                "/partial.pdf",
                "/error.pdf",
                "/encrypted.pdf",
            ]
            .map(str::to_owned)
            .to_vec(),
            _ => vec!["/blocked".into()],
        };
        let mut config = CrawlConfig {
            seed_urls: paths
                .iter()
                .map(|path| format!("{}{path}", self.origin))
                .collect(),
            sitemap_urls: vec![format!("{}/source.xml", self.origin)],
            obey_robots: matches!(self.mode, Mode::Disallow | Mode::RobotsFailure),
            discover_sitemaps: false,
            crawl_links: false,
            concurrency: 1,
            delay_ms: 0,
            request_timeout_secs: 3,
            ..Default::default()
        };
        config.pdf_analysis.max_pages = 1;
        config
    }
    async fn crawl(&self) -> (tempfile::TempDir, std::path::PathBuf, crate::CrawlReport) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("actual.sqlite");
        let crawler = Crawler::new(self.config())
            .unwrap()
            .with_store(&path)
            .unwrap();
        let result = crawler
            .run_controlled(None, CrawlControl::new())
            .await
            .unwrap();
        assert_eq!(result.outcome, crate::CrawlOutcome::Completed);
        drop(crawler);
        (directory, path, result.report)
    }
    async fn finish(self, requests: usize) {
        let _ = self.stop.send(());
        tokio::time::timeout(std::time::Duration::from_secs(10), self.join)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            self.hits.load(Ordering::SeqCst),
            requests,
            "The new traversal did not fetch"
        );
    }
}

// An actual readonly TX prepares raw/source/coverage/analysis proofs. This helper
// never manufactures a Header, Snapshot, Candidate or positive stored row.
fn collect(
    path: &std::path::Path,
    after: Option<&SitemapFindingPosition>,
    stop_after: Option<usize>,
    cancel_after: Option<usize>,
) -> Result<(
    bool,
    Vec<(SitemapFindingPosition, SitemapFindingPageCandidate)>,
)> {
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let tx = connection.unchecked_transaction()?;
    let control = SitemapReadControl::default();
    let config: String =
        tx.query_row("SELECT value FROM metadata WHERE key='config'", [], |row| {
            row.get(0)
        })?;
    let config_sha = crate::sitemap_ledger::digest(config.as_bytes());
    let run: String = tx.query_row(
        "SELECT value FROM metadata WHERE key='run_state'",
        [],
        |row| row.get(0),
    )?;
    let run: crate::CrawlRunState = serde_json::from_str(&run)?;
    let initialized: Option<String> = tx
        .query_row(
            "SELECT value FROM metadata WHERE key='discovery_initialized'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let raw = verify_raw_with_control(&tx, &control)?;
    let source = verify_source_with_control(&tx, raw.as_ref(), &control)?;
    let coverage = project_read_with_control(
        &tx,
        raw.as_ref(),
        CoverageReadContext {
            saved_config_bytes: Some(config.as_bytes()),
            saved_config_sha256: Some(&config_sha),
            generation: Some(run.generation),
            discovery_initialized: initialized.as_deref() == Some("true"),
        },
        &control,
    )?;
    let prepared = super::super::prepare_window_analysis_with_control(
        &tx,
        &AnalysisReadInput {
            raw: raw.as_ref(),
            source: source.as_ref(),
            generation: Some(run.generation),
            crawl_config_sha256: Some(&config_sha),
            coverage: &coverage.coverage,
        },
        &control,
    )?;
    let mut candidates = vec![];
    let exhausted = visit_saved_page_candidates(
        &prepared,
        raw.as_ref(),
        &coverage.coverage,
        &control,
        after,
        |position, candidate| {
            candidates.push((position, candidate));
            if cancel_after == Some(candidates.len()) {
                control.cancel();
            }
            Ok(if stop_after == Some(candidates.len()) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            })
        },
    )?;
    drop(prepared);
    drop(source);
    drop(raw);
    tx.commit()?;
    Ok((exhausted, candidates))
}

#[tokio::test]
async fn actual_saved_page_candidate_traversal_crosses_200_400_and_preserves_legacy_findings() {
    let site = Site::start(Mode::Large).await;
    let (_directory, path, report) = site.crawl().await;
    assert_eq!(report.pages.len(), 500);
    let requests = site.hits.load(Ordering::SeqCst);
    let (exhausted, candidates) = collect(&path, None, None, None).unwrap();
    assert!(exhausted);
    assert_eq!(candidates.len(), 500);
    for (ordinal, (position, candidate)) in candidates.iter().enumerate() {
        assert_eq!(
            position,
            &SitemapFindingPosition::SavedPage {
                ordinal: ordinal as u64
            }
        );
        assert_eq!(
            candidate.finding.observation.requested_url,
            report.pages[ordinal].url
        );
    }
    for (path, minimum) in [("/p0250", 200), ("/p0450", 400)] {
        let selected: Vec<_> = candidates
            .iter()
            .filter(|(_, c)| {
                c.finding.observation.requested_url == format!("{}{path}", site.origin)
            })
            .collect();
        assert_eq!(selected.len(), 1);
        assert!(selected[0].1.finding.observation.report_page_ordinal > minimum);
        assert_eq!(
            selected[0].1.finding.membership,
            SitemapSavedPageMembership::ListedOriginal
        );
    }
    let one = &candidates
        .iter()
        .find(|(_, c)| c.finding.observation.requested_url.ends_with("/p0250"))
        .unwrap()
        .1
        .finding;
    assert_eq!(
        (
            one.original_membership_occurrences,
            one.original_source_document_count,
            one.observed_inlinks.nonself_anchor_occurrences
        ),
        (1, 1, 1)
    );
    let two = &candidates
        .iter()
        .find(|(_, c)| c.finding.observation.requested_url.ends_with("/p0450"))
        .unwrap()
        .1
        .finding;
    assert_eq!(
        (
            two.original_membership_occurrences,
            two.original_source_document_count,
            two.observed_inlinks.anchor_occurrences
        ),
        (3, 2, 1)
    );
    assert!(two.listed_without_observed_nonself_inlinks);
    // Independent existing materialized whole-source projection, all actual
    // pages over three windows; shared builder must not change the legacy DTO.
    let store = CrawlStore::open_read_only(&path).unwrap();
    let mut legacy = vec![];
    let mut after = None;
    loop {
        let read = store
            .sitemap_read_snapshot_with_membership_analysis(&SitemapMembershipAnalysisRequest {
                row_limit: 0,
                page_limit: 200,
                after_page_ordinal: after,
                ..Default::default()
            })
            .unwrap();
        let analysis = read.membership_analysis.unwrap();
        assert!(analysis.context.available);
        after = analysis.last_saved_page_ordinal;
        legacy.extend(analysis.saved_pages.unwrap());
        if !analysis.saved_pages_have_more {
            break;
        }
    }
    assert_eq!(
        candidates
            .iter()
            .map(|(_, c)| c.finding.clone())
            .collect::<Vec<_>>(),
        legacy
    );
    let (exhausted, first) = collect(&path, None, Some(201), None).unwrap();
    assert!(!exhausted);
    let (exhausted, rest) = collect(&path, Some(&first.last().unwrap().0), None, None).unwrap();
    assert!(exhausted);
    assert_eq!(
        first.into_iter().chain(rest).collect::<Vec<_>>(),
        candidates
    );
    site.finish(requests).await;
}

#[tokio::test]
async fn actual_saved_page_candidates_retain_typed_pdf_complete_partial_error_and_encrypted() {
    let site = Site::start(Mode::Pdf).await;
    let (_directory, path, report) = site.crawl().await;
    let requests = site.hits.load(Ordering::SeqCst);
    let (_, candidates) = collect(&path, None, None, None).unwrap();
    for (name, status) in [
        ("complete", ResourceAnalysisStatus::Complete),
        ("partial", ResourceAnalysisStatus::Partial),
        ("error", ResourceAnalysisStatus::Error),
        ("encrypted", ResourceAnalysisStatus::Encrypted),
    ] {
        let url = format!("{}/{name}.pdf", site.origin);
        let page = report.pages.iter().find(|page| page.url == url).unwrap();
        assert_eq!(
            page.pdf.as_ref().unwrap().status,
            status,
            "Actual producer/parser precondition"
        );
        let candidate = &candidates
            .iter()
            .find(|(_, c)| c.finding.observation.requested_url == url)
            .unwrap()
            .1;
        assert!(candidate.pdf_field_observed && candidate.pdf_record_present);
        assert_eq!(candidate.pdf_status, Some(status));
        assert_eq!(candidate.content_type, page.content_type);
        assert!(candidate.robots_warning_field_observed);
        assert_eq!(candidate.robots_warning, page.robots_warning);
    }
    let normal = &candidates
        .iter()
        .find(|(_, c)| c.finding.observation.requested_url.ends_with("/normal"))
        .unwrap()
        .1;
    assert!(normal.pdf_field_observed);
    assert!(!normal.pdf_record_present);
    assert_eq!(normal.pdf_status, None);
    site.finish(requests).await;
}

#[tokio::test]
async fn actual_saved_page_candidates_distinguish_robots_policy_failure_from_disallow() {
    for (mode, warning) in [(Mode::Disallow, false), (Mode::RobotsFailure, true)] {
        let site = Site::start(mode).await;
        let (_directory, path, report) = site.crawl().await;
        let requests = site.hits.load(Ordering::SeqCst);
        let (_, candidates) = collect(&path, None, None, None).unwrap();
        let candidate = &candidates
            .iter()
            .find(|(_, c)| c.finding.observation.requested_url.ends_with("/blocked"))
            .unwrap()
            .1;
        let page = report
            .pages
            .iter()
            .find(|page| page.url.ends_with("/blocked"))
            .unwrap();
        assert!(page.blocked_by_robots);
        assert_eq!(
            page.robots_warning.is_some(),
            warning,
            "Actual HTTP policy precondition"
        );
        assert!(candidate.robots_warning_field_observed);
        assert_eq!(candidate.robots_warning, page.robots_warning);
        assert_eq!(
            candidate.finding.observation.state,
            SitemapTargetState::BlockedByRobots
        );
        site.finish(requests).await;
    }
}

// Explicit report representation/corruption fixture, not a native writer or a
// forged positive proof. The real report/checkpoint byte seal is rebound so the
// new typed field path, rather than a generic old-SHA rejection, is exercised.
fn alter_report(path: &std::path::Path, change: impl FnOnce(&mut serde_json::Value)) {
    let mut connection = Connection::open(path).unwrap();
    let tx = connection.transaction().unwrap();
    let raw: String = tx
        .query_row("SELECT value FROM metadata WHERE key='report'", [], |row| {
            row.get(0)
        })
        .unwrap();
    let checkpoint: String = tx
        .query_row(
            "SELECT value FROM metadata WHERE key='checkpoint'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut report: serde_json::Value = serde_json::from_str(&raw).unwrap();
    change(&mut report);
    let raw = serde_json::to_string(&report).unwrap();
    let mut checkpoint: serde_json::Value = serde_json::from_str(&checkpoint).unwrap();
    checkpoint["report_sha256"] = crate::sitemap_ledger::digest(raw.as_bytes()).into();
    tx.execute("UPDATE metadata SET value=?1 WHERE key='report'", [&raw])
        .unwrap();
    tx.execute(
        "UPDATE metadata SET value=?1 WHERE key='checkpoint'",
        [serde_json::to_string(&checkpoint).unwrap()],
    )
    .unwrap();
    tx.commit().unwrap();
}

#[tokio::test]
async fn actual_saved_page_candidates_preserve_absent_null_and_reject_invalid_late_typed_fields() {
    let site = Site::start(Mode::Large).await;
    let (_directory, path, _) = site.crawl().await;
    let requests = site.hits.load(Ordering::SeqCst);
    alter_report(&path, |report| {
        let first = report["pages"][0].as_object_mut().unwrap();
        first.remove("robots_warning");
        first.remove("pdf");
        report["pages"][1]["robots_warning"] = serde_json::Value::Null;
        report["pages"][1]["pdf"] = serde_json::Value::Null;
    });
    let (_, candidates) = collect(&path, None, None, None).unwrap();
    assert!(!candidates[0].1.robots_warning_field_observed && !candidates[0].1.pdf_field_observed);
    assert!(candidates[1].1.robots_warning_field_observed && candidates[1].1.pdf_field_observed);
    assert_eq!(candidates[0].1.robots_warning, None);
    assert_eq!(candidates[1].1.robots_warning, None);
    alter_report(&path, |report| {
        report["pages"][450]["pdf"] = serde_json::json!({"status":"unknown_status"});
    });
    assert!(
        collect(&path, None, None, None).is_err(),
        "Typed late PDF status must not become absent or Complete"
    );
    alter_report(&path, |report| {
        report["pages"][450]["pdf"] = serde_json::Value::Null;
        report["pages"][450]["robots_warning"] = serde_json::json!(true);
    });
    assert!(
        collect(&path, None, None, None).is_err(),
        "Typed late warning must not become no policy warning"
    );
    site.finish(requests).await;
}

#[tokio::test]
async fn actual_saved_page_candidate_positions_and_mid_traversal_cancellation_remain_typed() {
    let site = Site::start(Mode::Large).await;
    let (_directory, path, _) = site.crawl().await;
    let requests = site.hits.load(Ordering::SeqCst);
    let (_, after_zero) = collect(
        &path,
        Some(&SitemapFindingPosition::SavedPage { ordinal: 0 }),
        Some(1),
        None,
    )
    .unwrap();
    assert_eq!(
        after_zero[0].0,
        SitemapFindingPosition::SavedPage { ordinal: 1 }
    );
    let (exhausted, empty) = collect(
        &path,
        Some(&SitemapFindingPosition::SavedPage { ordinal: 499 }),
        None,
        None,
    )
    .unwrap();
    assert!(exhausted && empty.is_empty());
    for position in [
        SitemapFindingPosition::SavedPage { ordinal: 500 },
        SitemapFindingPosition::SourceMeasurement { sequence: 1 },
    ] {
        let error = collect(&path, Some(&position), None, None).unwrap_err();
        assert_eq!(
            error.downcast_ref::<SitemapFindingsError>(),
            Some(&SitemapFindingsError::InvalidContinuation)
        );
    }
    let error = collect(&path, None, None, Some(201)).unwrap_err();
    assert_eq!(
        error.downcast_ref::<crate::SitemapReadControlError>(),
        Some(&crate::SitemapReadControlError::Cancelled)
    );
    // An actual checkpoint/source-context disagreement is unavailable, never an
    // initialized empty candidate sequence.
    let connection = Connection::open(&path).unwrap();
    let raw: String = connection
        .query_row(
            "SELECT value FROM metadata WHERE key='checkpoint'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut checkpoint: serde_json::Value = serde_json::from_str(&raw).unwrap();
    checkpoint["sitemap_source_measurements"]["revision"] =
        (checkpoint["sitemap_source_measurements"]["revision"]
            .as_u64()
            .unwrap()
            + 1)
        .into();
    connection
        .execute(
            "UPDATE metadata SET value=?1 WHERE key='checkpoint'",
            [serde_json::to_string(&checkpoint).unwrap()],
        )
        .unwrap();
    drop(connection);
    let error = collect(&path, None, None, None).unwrap_err();
    assert_eq!(
        error.downcast_ref::<SitemapFindingsError>(),
        Some(&SitemapFindingsError::Unavailable)
    );
    site.finish(requests).await;
}
