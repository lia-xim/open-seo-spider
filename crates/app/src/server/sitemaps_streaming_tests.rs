//! Actual HTTP/Store regression source. AUTHORED / NOT_COMPILED / NOT_RUN (HOLD).
//! Physical exact64MiB/+1 is deliberately an OPEN native gate, not a tiny-cap test.
use super::*;
use crate::server::sitemap_csv_stream::{
    ExportRegistry, ExportStats, ExportTestFault, ExportTestGate, ExportTestHooks, ExportTestLimits,
};
use futures::StreamExt;
use open_seo_spider_core::SitemapLedgerRow;
use std::sync::atomic::AtomicU64;

#[path = "sitemaps_streaming_fixture.rs"]
mod fixture;
use fixture::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_csv_stream_matches_buffered_oracle_all_stored_exact_filters() {
    let site = Website::start(260, 0).await;
    let (dir, state, id) = site.setup().await;
    let before = files(&dir.0);
    let rows = raw_rows(&dir, id);
    let source = rows
        .iter()
        .find_map(|row| match row {
            SitemapLedgerRow::Document(d) if d.task_url.ends_with("leaf-a.xml") => {
                Some(d.observation_id.clone())
            }
            _ => None,
        })
        .unwrap();
    let mut queries: Vec<ReadQuery> = [
        RowKind::All,
        RowKind::Root,
        RowKind::Document,
        RowKind::Entry,
        RowKind::Location,
    ]
    .into_iter()
    .map(|kind| ReadQuery {
        kind,
        limit: Some(200),
        page_limit: Some(0),
        ..Default::default()
    })
    .collect();
    queries.extend([
        ReadQuery {
            source_observation_id: Some(source.clone()),
            ..Default::default()
        },
        ReadQuery {
            kind: RowKind::Location,
            source_observation_id: Some(source),
            ..Default::default()
        },
        ReadQuery {
            kind: RowKind::Location,
            membership_url: Some(format!("{}/member/duplicate?a=1&b=2", site.origin)),
            ..Default::default()
        },
        ReadQuery {
            kind: RowKind::Location,
            membership_url: Some(format!("{}/missing", site.origin)),
            ..Default::default()
        },
    ]);
    let http = Http::start(&state).await;
    for read in queries {
        let inventory = page(&state, id, read.clone()).await;
        if matches!(read.kind, RowKind::All | RowKind::Entry | RowKind::Location)
            && read.source_observation_id.is_none()
            && read.membership_url.is_none()
        {
            assert!(
                inventory.has_more,
                "actual stored cohort exceeds the first200"
            );
            assert_eq!(inventory.rows.as_ref().unwrap().len(), 200);
        }
        let q = query(&state, id, read).await;
        let expected = oracle(&dir, &state, id, &q).await;
        let cells = csv_cells(&expected);
        if matches!(q.kind, RowKind::All | RowKind::Entry | RowKind::Location)
            && q.source_observation_id.is_none()
            && q.membership_url.is_none()
        {
            assert!(cells.len() > 201);
        }
        for cells in &cells[1..] {
            let original: SitemapLedgerRow = serde_json::from_str(&cells[9]).unwrap();
            assert_eq!(cells[1], original.sequence().to_string());
            assert!(
                rows.contains(&original),
                "exact original typed row, not reconstructed membership"
            );
        }
        if q.kind == RowKind::All && q.source_observation_id.is_none() && q.membership_url.is_none()
        {
            assert!(
                cells.iter().any(|row| row[5] == "'=1+1"),
                "formula is neutralized, typed JSON above stays original"
            );
        }
        assert_response(http.get(id, &q).await, &expected).await;
    }
    let stats = drained(&state.exports).await;
    assert_eq!(stats.prepared_sessions, 9);
    assert_eq!(stats.completed_passes, 18);
    assert_eq!(stats.failed, 0);
    assert_eq!(stats.core_completed, 9);
    http.finish().await;
    assert_same_files(&dir.0, &before);
    site.finish(4).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_inventory_snapshot_token_opens_same_tx_two_pass_csv() {
    let site = Fixture::new().await;
    let (dir, state, id) = setup(&site, Mode::Native).await;
    let before = files(&dir.0);
    let requests = site.count.load(Ordering::SeqCst);
    let read = ReadQuery {
        kind: RowKind::Location,
        limit: Some(1),
        page_limit: Some(1),
        ..Default::default()
    };
    let first = page(&state, id, read.clone()).await;
    assert!(first.next_cursor.is_some());
    assert!(first.next_page_cursor.is_some());
    let mut raw = read.clone();
    raw.cursor = first.next_cursor.clone();
    let raw = page(&state, id, raw).await;
    let mut saved = read.clone();
    saved.page_cursor = first.next_page_cursor.clone();
    let saved = page(&state, id, saved).await;
    assert_eq!(raw.snapshot_token, first.snapshot_token);
    assert_eq!(saved.snapshot_token, first.snapshot_token);
    assert_ne!(
        raw.rows.as_ref().unwrap()[0].sequence(),
        first.rows.as_ref().unwrap()[0].sequence()
    );
    assert_eq!(saved.rows.as_ref().unwrap(), first.rows.as_ref().unwrap());
    let q = CsvQuery {
        kind: read.kind,
        source_observation_id: None,
        membership_url: None,
        snapshot: first.snapshot_token.unwrap(),
    };
    let expected = oracle(&dir, &state, id, &q).await;
    let count = (csv_cells(&expected).len() - 1) as u64;
    let http = Http::start(&state).await;
    let response = http.get(id, &q).await;
    let snapshot = first.snapshot.as_ref().unwrap();
    assert_eq!(
        response.headers()["content-disposition"],
        format!("attachment; filename=\"{id}-sitemaps.csv\"")
    );
    assert_eq!(
        response.headers()["x-sitemap-catalog-revision"],
        snapshot.catalog_revision.to_string()
    );
    assert_eq!(
        response.headers()["x-sitemap-catalog-state-sha256"],
        snapshot.catalog_state_sha256
    );
    assert_eq!(
        response.headers()["x-sitemap-catalog-rolling-sha256"],
        snapshot.catalog_rolling_sha256
    );
    assert_eq!(
        response.headers()["x-crawl-generation"],
        snapshot.crawl_generation.to_string()
    );
    assert_eq!(
        response.headers()["x-crawl-config-sha256"],
        snapshot.crawl_config_sha256
    );
    assert_response(response, &expected).await;
    let stats = drained(&state.exports).await;
    assert_eq!(
        (
            stats.started,
            stats.prepared_sessions,
            stats.completed_passes
        ),
        (1, 1, 2)
    );
    assert_eq!(
        (stats.first_pass_rows, stats.second_pass_rows),
        (count, count)
    );
    assert_eq!(
        (stats.core_completed, stats.joined, stats.failed),
        (1, 1, 0)
    );
    http.finish().await;
    assert_same_files(&dir.0, &before);
    site.finish(requests).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_csv_stream_rejects_full_tail_source_report_and_inventory_corruption_before_ready() {
    // Each fault starts with an actual healthy PublicCrawler Store and genuine
    // filtered token. Corruption is beyond/excluded from the selected root rows.
    for fault in [
        "UPDATE sitemap_locations SET data_sha256=printf('%064d',0) WHERE sequence=(SELECT max(sequence) FROM sitemap_locations)",
        "UPDATE sitemap_source_measurements SET data=data||' ' WHERE sequence=(SELECT max(sequence) FROM sitemap_source_measurements)",
        "UPDATE sitemap_source_measurement_commits SET data_sha256=printf('%064d',0) WHERE document_sequence=(SELECT max(document_sequence) FROM sitemap_source_measurement_commits)",
        "UPDATE metadata SET value=value||' ' WHERE key='report'",
        "REHASHED_REPORT_SOURCE_TAMPER",
        "UPDATE discovery_frontier SET done=7 WHERE kind='sitemap'",
    ] {
        let site = Fixture::new().await;
        let (dir, state, id) = setup(&site, Mode::Native).await;
        let requests = site.count.load(Ordering::SeqCst);
        let q = query(
            &state,
            id,
            ReadQuery {
                kind: RowKind::Root,
                ..Default::default()
            },
        )
        .await;
        if fault == "REHASHED_REPORT_SOURCE_TAMPER" {
            let mut report: Value = serde_json::from_str(&report_text(&dir, id)).unwrap();
            report["sitemap_source_measurements"]["records"][0]["staged_document_sha256"] =
                "f".repeat(64).into();
            controlled_report_bytes(&dir, id, &serde_json::to_string(&report).unwrap());
        } else {
            sql(&dir, id, fault);
        }
        warm_corrupt_store_bookkeeping(&dir, id);
        let before = files(&dir.0);
        let http = Http::start(&state).await;
        failure(http.get(id, &q).await, 500, "sitemap_read_failure").await;
        let stats = drained(&state.exports).await;
        assert_eq!(
            (
                stats.prepared_sessions,
                stats.completed_passes,
                stats.prefix_bytes
            ),
            (0, 0, 0),
            "{fault}"
        );
        assert_eq!((stats.started, stats.joined, stats.failed), (1, 1, 1));
        http.finish().await;
        assert_same_files(&dir.0, &before);
        site.finish(requests).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_csv_stream_stales_a_marker_pending_and_terminal_generation_context() {
    // Pending-only: exact canonical pending row changes inventory state while
    // the original raw catalog revision and saved report remain identical.
    let site = Fixture::new().await;
    let (dir, state, id) = setup(&site, Mode::Native).await;
    let requests = site.count.load(Ordering::SeqCst);
    let q = query(&state, id, Default::default()).await;
    let original = raw_rows(&dir, id);
    rusqlite::Connection::open(db(&dir, id))
        .unwrap()
        .execute(
            "INSERT INTO discovery_frontier(kind,url,done) VALUES('robots',?1,0)",
            [site.url("/later-robots.txt")],
        )
        .unwrap();
    let http = Http::start(&state).await;
    failure(http.get(id, &q).await, 409, "stale_sitemap_cursor").await;
    assert_eq!(raw_rows(&dir, id), original);
    http.finish().await;
    site.finish(requests).await;

    // An actual A commit survives an actual B trigger abort. Restore these
    // exact durable SQL bytes, not a DTO with invented hashes or commit proof.
    let site = Fixture::new().await;
    let (dir, state, id) = setup(&site, Mode::Native).await;
    let q = query(&state, id, Default::default()).await;
    let original = raw_rows(&dir, id);
    let config = get_job(&state, id)
        .await
        .ok()
        .unwrap()
        .read()
        .await
        .config
        .clone();
    let shadow = OwnedDirectory::new();
    let shadow_id = Uuid::new_v4();
    drop(CrawlStore::open(db(&shadow, shadow_id)).unwrap());
    sql(
        &shadow,
        shadow_id,
        "CREATE TRIGGER reject_actual_membership BEFORE INSERT ON sitemap_documents WHEN NEW.source_kind='sitemap' BEGIN SELECT RAISE(ABORT,'test B rollback after actual A'); END",
    );
    site.offline.store(false, Ordering::SeqCst);
    let crawler = Crawler::from_saved_config(config)
        .unwrap()
        .with_store(db(&shadow, shadow_id))
        .unwrap();
    assert!(
        crawler
            .run_controlled(None, CrawlControl::new())
            .await
            .is_err()
    );
    drop(crawler);
    site.offline.store(true, Ordering::SeqCst);
    let requests = site.count.load(Ordering::SeqCst);
    let actual = rusqlite::Connection::open(db(&shadow, shadow_id)).unwrap();
    let count: i64 = actual
        .query_row(
            "SELECT count(*) FROM sitemap_source_measurements",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(count > 0);
    let commits: i64 = actual
        .query_row(
            "SELECT count(*) FROM sitemap_source_measurement_commits",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(commits, 0);
    drop(actual);
    let mut connection = rusqlite::Connection::open(db(&dir, id)).unwrap();
    connection
        .execute(
            "ATTACH DATABASE ?1 AS actual_a",
            [db(&shadow, shadow_id).to_str().unwrap()],
        )
        .unwrap();
    let tx = connection.transaction().unwrap();
    let same:bool=tx.query_row("SELECT (SELECT value FROM metadata WHERE key='config')=(SELECT value FROM actual_a.metadata WHERE key='config')",[],|r|r.get(0)).unwrap();
    assert!(same);
    tx.execute_batch("DELETE FROM sitemap_source_measurement_commits; DELETE FROM sitemap_source_measurements;
        INSERT INTO sitemap_source_measurements SELECT * FROM actual_a.sitemap_source_measurements;
        INSERT INTO sitemap_source_measurement_commits SELECT * FROM actual_a.sitemap_source_measurement_commits;
        UPDATE metadata SET value=(SELECT value FROM actual_a.metadata WHERE key='sitemap_source_store') WHERE key='sitemap_source_store';").unwrap();
    tx.commit().unwrap();
    connection
        .execute_batch("DETACH DATABASE actual_a")
        .unwrap();
    drop(connection);
    let snapshot = CrawlStore::open_read_only(db(&dir, id))
        .unwrap()
        .sitemap_read_snapshot()
        .unwrap();
    assert!(
        !snapshot
            .sitemap_source_measurements
            .as_ref()
            .unwrap()
            .records
            .is_empty()
    );
    assert!(
        snapshot
            .sitemap_source_measurements
            .as_ref()
            .unwrap()
            .commits
            .is_empty()
    );
    assert_eq!(raw_rows(&dir, id), original);
    let http = Http::start(&state).await;
    failure(http.get(id, &q).await, 409, "stale_sitemap_cursor").await;
    http.finish().await;
    site.finish(requests).await;

    // A legitimate long source identity produces the durable CaptureUnavailable
    // marker through the public Crawler, not a fabricated zero-record DTO.
    let site = Fixture::new().await;
    let dir = OwnedDirectory::new();
    let id = Uuid::new_v4();
    let config = CrawlConfig {
        seed_urls: vec![site.url("/seed")],
        sitemap_urls: vec![site.url(&format!("/{}.xml", "x".repeat(6000)))],
        concurrency: 1,
        delay_ms: 0,
        obey_robots: false,
        discover_sitemaps: false,
        crawl_links: false,
        request_timeout_secs: 3,
        ..Default::default()
    };
    let crawler = Crawler::new(config)
        .unwrap()
        .with_store(db(&dir, id))
        .unwrap();
    let result = crawler
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(result.outcome, CrawlOutcome::Completed);
    drop(crawler);
    let state = state_from_result(&dir, id, result);
    warm(&dir, id);
    let captured = CrawlStore::open_read_only(db(&dir, id))
        .unwrap()
        .sitemap_read_snapshot()
        .unwrap();
    let summary = &captured
        .sitemap_source_measurements
        .as_ref()
        .unwrap()
        .summary;
    assert_eq!(
        summary.omission,
        Some(open_seo_spider_core::SitemapSourceStoreOmission::CaptureUnavailable)
    );
    assert!(
        captured
            .sitemap_source_measurements
            .as_ref()
            .unwrap()
            .records
            .is_empty()
    );
    let connection = rusqlite::Connection::open(db(&dir, id)).unwrap();
    let actual_marker: String = connection
        .query_row(
            "SELECT value FROM metadata WHERE key='sitemap_source_store'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    drop(connection);
    historical_source_none(&dir, id);
    let q = query(&state, id, Default::default()).await;
    let expected = oracle(&dir, &state, id, &q).await;
    let connection = rusqlite::Connection::open(db(&dir, id)).unwrap();
    connection
        .execute(
            "INSERT INTO metadata(key,value) VALUES('sitemap_source_store',?1)",
            [actual_marker],
        )
        .unwrap();
    drop(connection);
    let restored = CrawlStore::open_read_only(db(&dir, id))
        .unwrap()
        .sitemap_read_snapshot()
        .unwrap();
    assert_eq!(restored.catalog, captured.catalog);
    assert_eq!(restored.coverage_inventory, captured.coverage_inventory);
    assert_eq!(
        restored
            .sitemap_source_measurements
            .as_ref()
            .unwrap()
            .summary,
        *summary
    );
    let requests = site.count.load(Ordering::SeqCst);
    let http = Http::start(&state).await;
    failure(http.get(id, &q).await, 409, "stale_sitemap_cursor").await;
    let fresh = query(&state, id, Default::default()).await;
    assert_response(http.get(id, &fresh).await, &expected).await;
    http.finish().await;
    site.finish(requests).await;

    // Same actual report values, different exact saved bytes: controlled pretty
    // serialization keeps SQL/source values valid and changes only the report
    // SHA/length and its exact checkpoint seal. No fabricated page is admitted.
    let site = Fixture::new().await;
    let (dir, state, id) = setup(&site, Mode::Native).await;
    let requests = site.count.load(Ordering::SeqCst);
    let q = query(&state, id, Default::default()).await;
    let expected = oracle(&dir, &state, id, &q).await;
    let read = CrawlStore::open_read_only(db(&dir, id))
        .unwrap()
        .sitemap_read_snapshot()
        .unwrap();
    let original = report_text(&dir, id);
    let value: Value = serde_json::from_str(&original).unwrap();
    let pretty = serde_json::to_string_pretty(&value).unwrap();
    assert_ne!(original, pretty);
    controlled_report_bytes(&dir, id, &pretty);
    let current = CrawlStore::open_read_only(db(&dir, id))
        .unwrap()
        .sitemap_read_snapshot()
        .unwrap();
    assert_eq!(current.catalog, read.catalog);
    assert_eq!(current.generation, read.generation);
    assert_eq!(current.coverage_inventory, read.coverage_inventory);
    assert_eq!(
        current.sitemap_source_measurements,
        read.sitemap_source_measurements
    );
    let http = Http::start(&state).await;
    failure(http.get(id, &q).await, 409, "stale_sitemap_cursor").await;
    let fresh = query(&state, id, Default::default()).await;
    assert_response(http.get(id, &fresh).await, &expected).await;
    http.finish().await;
    site.finish(requests).await;

    // This is a REAL next invocation/terminal checkpoint, not an isolated
    // report-only claim. No new source or page request is required on resume.
    let site = Fixture::new().await;
    let (dir, state, id) = setup(&site, Mode::Native).await;
    let requests = site.count.load(Ordering::SeqCst);
    let q = query(&state, id, Default::default()).await;
    let config = get_job(&state, id)
        .await
        .ok()
        .unwrap()
        .read()
        .await
        .config
        .clone();
    let crawler = Crawler::from_saved_config(config)
        .unwrap()
        .with_store(db(&dir, id))
        .unwrap();
    let result = crawler
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(result.checkpoint.as_ref().unwrap().generation, 2);
    drop(crawler);
    let http = Http::start(&state).await;
    failure(http.get(id, &q).await, 409, "stale_sitemap_cursor").await;
    http.finish().await;
    site.finish(requests).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_csv_stream_selected_presence_and_exact_parent_guards() {
    let site = Fixture::new().await;
    let (_parent_dir, parent, parent_id) = setup(&site, Mode::Native).await;
    // Selected refresh uses a real page response (including legitimate410).
    for fault in [
        None,
        Some(
            "DELETE FROM sitemap_source_measurement_commits; DELETE FROM sitemap_source_measurements; DELETE FROM metadata WHERE key='sitemap_source_store'",
        ),
        Some("UPDATE recrawl_base SET snapshot=CAST(snapshot||' ' AS BLOB) WHERE id=1"),
    ] {
        let (dir, state, id) = selected(&parent, parent_id).await;
        let requests = site.count.load(Ordering::SeqCst);
        let q = query(&state, id, Default::default()).await;
        let expected = oracle(&dir, &state, id, &q).await;
        let http = Http::start(&state).await;
        if let Some(fault) = fault {
            sql(&dir, id, fault);
            failure(http.get(id, &q).await, 500, "sitemap_read_failure").await;
        } else {
            assert_response(http.get(id, &q).await, &expected).await;
        }
        http.finish().await;
        assert_eq!(
            site.count.load(Ordering::SeqCst),
            requests,
            "selected export made no website request"
        );
    }
    let requests = site.count.load(Ordering::SeqCst);
    site.finish(requests).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_csv_stream_pins_original_tx_while_later_writer_changes_generation() {
    let site = Fixture::new().await;
    let (dir, mut state, id) = setup(&site, Mode::Native).await;
    let requests = site.count.load(Ordering::SeqCst);
    let gate = Arc::new(ExportTestGate::default());
    state.exports = registry(ExportTestHooks {
        after_preflight: Some(gate.clone()),
        ..Default::default()
    });
    let q = query(&state, id, Default::default()).await;
    let expected = oracle(&dir, &state, id, &q).await;
    let http = Http::start(&state).await;
    let response = http.get(id, &q).await;
    reached(&gate).await;
    assert_eq!(response.headers()["x-crawl-generation"], "1");
    let job = get_job(&state, id).await.ok().unwrap();
    let config = tokio::time::timeout(Duration::from_secs(2), job.write())
        .await
        .expect("download must release App job guard after Ready")
        .config
        .clone();
    let crawler = Crawler::from_saved_config(config)
        .unwrap()
        .with_store(db(&dir, id))
        .unwrap();
    let result = crawler
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(result.checkpoint.as_ref().unwrap().generation, 2);
    drop(crawler);
    {
        let mut guard = job.write().await;
        guard.generation = 2;
        guard.checkpoint = result.checkpoint;
        guard.report = Some(result.report);
    }
    gate.release();
    assert_response(response, &expected).await;
    failure(http.get(id, &q).await, 409, "stale_sitemap_cursor").await;
    let fresh = query(&state, id, Default::default()).await;
    assert_response(
        http.get(id, &fresh).await,
        &oracle(&dir, &state, id, &fresh).await,
    )
    .await;
    let stats = drained(&state.exports).await;
    assert_eq!(
        (stats.started, stats.joined, stats.core_completed),
        (3, 3, 2)
    );
    http.finish().await;
    site.finish(requests).await;
}

// No #[test] named exact64MiB: that physical gate is OPEN, with an actual
// accepted-producer calibration design in HANDOFF.md. Never lower the real cap.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_http_csv_large_body_uses_bounded_chunks_and_backpressure() {
    let site = Website::start(600, 5000).await;
    let (dir, state, id) = site.setup().await;
    let q = query(&state, id, Default::default()).await;
    let expected = oracle(&dir, &state, id, &q).await;
    assert!(
        expected.len() > 8 * 1024 * 1024 && expected.len() < membership::MAX_CSV_BYTES,
        "real unchanged-cap cohort must exceed socket buffering, actual={}",
        expected.len()
    );
    let before = files(&dir.0);
    let http = Http::start(&state).await;
    let (mut socket, length) = http.raw(id, &q).await;
    assert_eq!(length, expected.len());
    wait_stats(&state.exports, |s| s.peak_queued == 4 && s.active == 1).await;
    // Observe a stable actual formatter position while the real TCP consumer
    // is stalled. Merely seeing a full queue once would not prove backpressure.
    let stable = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let a = state.exports.snapshot();
            tokio::time::sleep(Duration::from_millis(150)).await;
            let b = state.exports.snapshot();
            if a.second_pass_rows == b.second_pass_rows
                && a.prefix_bytes == b.prefix_bytes
                && b.active == 1
            {
                break b;
            }
        }
    })
    .await
    .unwrap();
    assert!(stable.second_pass_rows < stable.first_pass_rows);
    assert_eq!(stable.core_completed, 0);
    assert!(stable.max_chunk_bytes <= 65536);
    assert!(stable.peak_queued <= 4);
    let mut bytes = Vec::new();
    tokio::time::timeout(Duration::from_secs(20), socket.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, expected);
    let stats = drained(&state.exports).await;
    assert_eq!(
        (
            stats.prepared_sessions,
            stats.completed_passes,
            stats.core_completed,
            stats.failed
        ),
        (1, 2, 1, 0)
    );
    assert_eq!(stats.first_pass_rows, stats.second_pass_rows);
    http.finish().await;
    assert_same_files(&dir.0, &before);
    site.finish(4).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_http_csv_client_disconnect_cancels_and_joins_producer() {
    let site = Website::start(600, 5000).await;
    let (dir, state, id) = site.setup().await;
    let q = query(&state, id, Default::default()).await;
    let expected = oracle(&dir, &state, id, &q).await;
    assert!(expected.len() > 8 * 1024 * 1024);
    let before = files(&dir.0);
    let http = Http::start(&state).await;
    let (socket, _) = http.raw(id, &q).await;
    wait_stats(&state.exports, |s| s.peak_queued == 4 && s.active == 1).await;
    drop(socket);
    let stats = drained(&state.exports).await;
    assert_eq!(
        (
            stats.started,
            stats.joined,
            stats.failed,
            stats.core_completed
        ),
        (1, 1, 1, 0)
    );
    // A cancelled export must not poison the real Store or the next download.
    assert_response(http.get(id, &q).await, &expected).await;
    http.finish().await;
    assert_same_files(&dir.0, &before);
    site.finish(4).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_http_csv_idle_and_absolute_deadline_close_body_and_drain() {
    let site = Website::start(600, 5000).await;
    let (dir, base, id) = site.setup().await;
    let q = query(&base, id, Default::default()).await;
    assert!(oracle(&dir, &base, id, &q).await.len() > 8 * 1024 * 1024);
    // Real idle TCP consumer; no byte/queue/chunk cap is reduced.
    let mut state = base.clone();
    state.exports = ExportRegistry::for_test(
        ExportTestLimits {
            stall_timeout: Duration::from_millis(200),
            ..Default::default()
        },
        Default::default(),
    );
    let http = Http::start(&state).await;
    let (mut socket, length) = http.raw(id, &q).await;
    let stats = drained(&state.exports).await;
    assert_eq!((stats.failed, stats.core_completed), (1, 0));
    let mut bytes = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(3), socket.read_to_end(&mut bytes))
        .await
        .unwrap();
    assert!(bytes.len() < length);
    drop(socket);
    http.finish().await;
    // Absolute stream deadline while the actual producer is at a cooperative
    // gate; no stall deadline competes, and no release is supplied by this test.
    let gate = Arc::new(ExportTestGate::default());
    let mut state = base.clone();
    state.exports = ExportRegistry::for_test(
        ExportTestLimits {
            stream_timeout: Duration::from_millis(250),
            stall_timeout: Duration::from_secs(5),
            ..Default::default()
        },
        ExportTestHooks {
            after_preflight: Some(gate.clone()),
            ..Default::default()
        },
    );
    let http = Http::start(&state).await;
    let response = http.get(id, &q).await;
    reached(&gate).await;
    truncated(response).await;
    let stats = drained(&state.exports).await;
    assert_eq!(
        (stats.completed_passes, stats.failed, stats.core_completed),
        (1, 1, 0)
    );
    http.finish().await;
    site.finish(4).await;
    // Preparation deadline is after a real complete Pass1 and before HTTP200.
    // Use the genuine small producer here so this timing control does not
    // depend on a multi-MiB debug formatter finishing before a short deadline.
    let site = Fixture::new().await;
    let (_dir, mut state, id) = setup(&site, Mode::Native).await;
    let requests = site.count.load(Ordering::SeqCst);
    let q = query(&state, id, Default::default()).await;
    let gate = Arc::new(ExportTestGate::default());
    state.exports = ExportRegistry::for_test(
        ExportTestLimits {
            prepare_timeout: Duration::from_secs(2),
            ..Default::default()
        },
        ExportTestHooks {
            before_ready: Some(gate.clone()),
            ..Default::default()
        },
    );
    let http = Http::start(&state).await;
    failure(http.get(id, &q).await, 504, "sitemap_export_timeout").await;
    assert!(gate.is_reached());
    let stats = drained(&state.exports).await;
    assert_eq!(
        (stats.completed_passes, stats.prefix_bytes, stats.failed),
        (1, 0, 1)
    );
    http.finish().await;
    site.finish(requests).await;

    // Positive slow ACTIVE drain after the real worker has joined. Polling the
    // actual Axum Body here isolates transport progress from OS socket buffers;
    // separate controls above exercise the real HTTP stalled client. Queue and
    // chunk caps are unchanged, and no Session/Header/window is fabricated.
    let site = Website::start(260, 0).await;
    let (dir, mut state, id) = site.setup().await;
    let q = query(
        &state,
        id,
        ReadQuery {
            kind: RowKind::Location,
            ..Default::default()
        },
    )
    .await;
    let expected = oracle(&dir, &state, id, &q).await;
    let prefixes = (expected.len() - 1) / 65_536;
    assert!(
        prefixes >= 3,
        "actual fixed-filter fixture needs at least3 prefixchunks, got{prefixes}"
    );
    state.exports = ExportRegistry::for_test(
        ExportTestLimits {
            stream_timeout: Duration::from_secs(15),
            stall_timeout: Duration::from_secs(1),
            ..Default::default()
        },
        Default::default(),
    );
    let response = export_csv(State(state.clone()), Path(id), Ok(Query(q)))
        .await
        .unwrap();
    let mut stream = response.into_body().into_data_stream();
    let mut bytes = Vec::new();
    // If this actual port/row encoding yields more than four prefixes, first
    // drain only the excess, leaving the real bounded queue3..4 positive chunks.
    for _ in 0..prefixes.saturating_sub(4) {
        let chunk = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk.len(), 65_536);
        bytes.extend_from_slice(&chunk);
    }
    let stats = wait_stats(&state.exports, |s| s.core_completed == 1 && s.joined == 1).await;
    assert_eq!(
        stats.active, 1,
        "actual joined worker, unconsumed transport Body"
    );
    let started = std::time::Instant::now();
    for _ in 0..prefixes.min(4) {
        tokio::time::sleep(Duration::from_millis(750)).await;
        let chunk = stream
            .next()
            .await
            .expect("actual prefix")
            .expect("active progress keeps idle deadline alive");
        assert_eq!(chunk.len(), 65_536);
        bytes.extend_from_slice(&chunk);
        assert_eq!(state.exports.snapshot().core_completed, 1);
    }
    assert!(started.elapsed() > Duration::from_secs(1));
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk.unwrap());
    }
    assert_eq!(bytes, expected);
    let stats = drained(&state.exports).await;
    assert_eq!(
        (
            stats.failed,
            stats.started,
            stats.joined,
            stats.completed_passes
        ),
        (0, 1, 1, 2)
    );
    state.exports.shutdown_and_drain().await.unwrap();
    site.finish(4).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_http_csv_tail_is_withheld_until_worker_join_and_tx_close() {
    let site = Website::start(260, 0).await;
    let (dir, mut state, id) = site.setup().await;
    let before = Arc::new(ExportTestGate::default());
    let after = Arc::new(ExportTestGate::default());
    state.exports = registry(ExportTestHooks {
        before_core_return: Some(before.clone()),
        after_core_return: Some(after.clone()),
        ..Default::default()
    });
    let q = query(&state, id, Default::default()).await;
    let expected = oracle(&dir, &state, id, &q).await;
    let http = Http::start(&state).await;
    let response = http.get(id, &q).await;
    let length = response.content_length().unwrap() as usize;
    let received = Arc::new(AtomicU64::new(0));
    let observed = received.clone();
    let registry = state.exports.clone();
    let collector = tokio::spawn(async move {
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            bytes.extend_from_slice(&chunk);
            observed.store(bytes.len() as u64, Ordering::SeqCst);
            if bytes.len() == length {
                let stats = registry.snapshot();
                assert_eq!(stats.joined, 1);
                assert_eq!(stats.core_completed, 1);
            }
        }
        bytes
    });
    reached(&before).await;
    let stats = state.exports.snapshot();
    assert_eq!(
        (stats.completed_passes, stats.core_completed, stats.joined),
        (2, 0, 0)
    );
    assert!(received.load(Ordering::SeqCst) < length as u64);
    before.release();
    reached(&after).await;
    let stats = state.exports.snapshot();
    assert_eq!((stats.core_completed, stats.joined), (1, 0));
    assert!(received.load(Ordering::SeqCst) < length as u64);
    // The actual read transaction has returned at this gate. A fresh writer
    // checkpoint is permitted; this is not inferred merely from queue state.
    let connection = rusqlite::Connection::open(db(&dir, id)).unwrap();
    let busy: i64 = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0))
        .unwrap();
    assert_eq!(busy, 0);
    drop(connection);
    after.release();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(10), collector)
            .await
            .unwrap()
            .unwrap(),
        expected
    );
    let stats = drained(&state.exports).await;
    assert_eq!(stats.failed, 0);
    http.finish().await;
    site.finish(4).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_csv_stream_second_pass_failure_and_response_build_failure_drain() {
    let site = Website::start(260, 0).await;
    let (dir, base, id) = site.setup().await;
    let q = query(&base, id, Default::default()).await;
    assert!(oracle(&dir, &base, id, &q).await.len() > 131072);
    for fault in [
        ExportTestFault::SecondPassAfterBytes(131072),
        ExportTestFault::BeforeCoreReturn,
        ExportTestFault::SecondPassDigestMismatch,
        ExportTestFault::AfterCoreReturnPanic,
    ] {
        let mut state = base.clone();
        let gate = Arc::new(ExportTestGate::default());
        state.exports = registry(ExportTestHooks {
            after_preflight: Some(gate.clone()),
            fault,
            ..Default::default()
        });
        let http = Http::start(&state).await;
        let response = http.get(id, &q).await;
        reached(&gate).await;
        gate.release();
        truncated(response).await;
        let stats = drained(&state.exports).await;
        assert_eq!((stats.started, stats.joined, stats.failed), (1, 1, 1));
        assert_eq!(
            stats.core_completed,
            u64::from(matches!(fault, ExportTestFault::AfterCoreReturnPanic))
        );
        http.finish().await;
    }
    let mut state = base.clone();
    let gate = Arc::new(ExportTestGate::default());
    state.exports = registry(ExportTestHooks {
        after_preflight: Some(gate.clone()),
        fault: ExportTestFault::ResponseBuildAfterReady,
        ..Default::default()
    });
    let http = Http::start(&state).await;
    failure(http.get(id, &q).await, 500, "sitemap_read_failure").await;
    // Dropping the real Body cancels the producer held before Pass2. The test
    // supplies no release; failed/joined therefore witness the cancellation.
    let stats = drained(&state.exports).await;
    assert_eq!((stats.started, stats.joined), (1, 1));
    assert!(stats.failed > 0);
    http.finish().await;
    // Dropping the actual pending HTTP handler before Ready must also cancel
    // its registered worker, without manually releasing the gate.
    let gate = Arc::new(ExportTestGate::default());
    let mut state = base;
    state.exports = registry(ExportTestHooks {
        before_ready: Some(gate.clone()),
        ..Default::default()
    });
    let http = Http::start(&state).await;
    let url = http.export_url(id, &q);
    let request = tokio::spawn(async move { client().get(url).send().await });
    reached(&gate).await;
    request.abort();
    let _ = request.await;
    let stats = drained(&state.exports).await;
    assert_eq!((stats.started, stats.joined, stats.failed), (1, 1, 1));
    assert_eq!(stats.prefix_bytes, 0);
    http.finish().await;
    site.finish(4).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_csv_stream_shutdown_cancels_and_drains_active_exports() {
    let site = Website::start(600, 5000).await;
    let (dir, state, id) = site.setup().await;
    let q = query(&state, id, Default::default()).await;
    assert!(oracle(&dir, &state, id, &q).await.len() > 8 * 1024 * 1024);
    let http = Http::start(&state).await;
    let (mut socket, length) = http.raw(id, &q).await;
    wait_stats(&state.exports, |s| s.peak_queued == 4 && s.active == 1).await;
    let (a, b) = tokio::join!(
        state.exports.shutdown_and_drain(),
        state.exports.shutdown_and_drain()
    );
    a.unwrap();
    b.unwrap();
    let stats = state.exports.snapshot();
    assert_eq!(
        (stats.started, stats.joined, stats.active, stats.failed),
        (1, 1, 0, 1)
    );
    let mut bytes = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), socket.read_to_end(&mut bytes))
        .await
        .unwrap();
    assert!(bytes.len() < length);
    drop(socket);
    failure(http.get(id, &q).await, 503, "sitemap_export_cancelled").await;
    assert_eq!(
        state.exports.snapshot().started,
        1,
        "shutdown rejects new admission"
    );
    http.finish().await;
    site.finish(4).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_csv_stream_error_priority_preserves_existing_statuses_and_token_roles() {
    let site = Fixture::new().await;
    let (dir, mut state, id) = setup(&site, Mode::Native).await;
    let requests = site.count.load(Ordering::SeqCst);
    let read = ReadQuery {
        kind: RowKind::Location,
        limit: Some(1),
        page_limit: Some(1),
        ..Default::default()
    };
    let inventory = page(&state, id, read.clone()).await;
    let q = query(&state, id, read).await;
    let http = Http::start(&state).await;
    for token in [
        inventory.next_cursor.unwrap(),
        inventory.next_page_cursor.unwrap(),
        "malformed".into(),
    ] {
        let mut invalid = q.clone();
        invalid.snapshot = token;
        failure(
            http.get(Uuid::new_v4(), &invalid).await,
            400,
            "invalid_sitemap_cursor",
        )
        .await;
    }
    failure(http.get(Uuid::new_v4(), &q).await, 404, "job_not_found").await;
    let mut wrong = q.clone();
    wrong.kind = RowKind::All;
    failure(http.get(id, &wrong).await, 409, "stale_sitemap_cursor").await;
    // UUID/filter mismatch is checked before any Store path is opened.
    let (foreign_dir, foreign, foreign_id) = setup_fresh_small().await;
    failure(
        http.get(id, &query(&foreign, foreign_id, Default::default()).await)
            .await,
        409,
        "stale_sitemap_cursor",
    )
    .await;
    drop(foreign);
    drop(foreign_dir);
    let url = http.export_url(id, &q);
    failure(
        client()
            .get(format!("{url}&unexpected=1"))
            .send()
            .await
            .unwrap(),
        400,
        "invalid_sitemap_query",
    )
    .await;
    {
        let job = get_job(&state, id).await.ok().unwrap();
        job.write().await.generation += 1;
    }
    failure(http.get(id, &q).await, 409, "sitemap_job_snapshot_conflict").await;
    {
        let job = get_job(&state, id).await.ok().unwrap();
        job.write().await.generation -= 1;
    }
    http.finish().await;
    state.exports = ExportRegistry::default();
    // Existing DB with historical RawNone, but a previously valid full token:
    // catalog unavailable has priority over token/DB state inequality.
    let connection = rusqlite::Connection::open(db(&dir, id)).unwrap();
    connection.execute_batch("PRAGMA foreign_keys=OFF; DELETE FROM sitemap_source_measurement_commits; DELETE FROM sitemap_source_measurements;
        DELETE FROM sitemap_locations; DELETE FROM sitemap_entries; DELETE FROM sitemap_roots; DELETE FROM sitemap_documents;
        DELETE FROM metadata WHERE key IN ('sitemap_source_store','sitemap_ledger');").unwrap();
    drop(connection);
    // Establish the actual catalog marker key, never assume historical absence
    // can be created by deleting unrelated report/config state.
    let raw = CrawlStore::open_read_only(db(&dir, id))
        .unwrap()
        .sitemap_catalog()
        .unwrap();
    assert!(raw.is_none(), "fixture must actually be RawNone");
    let http = Http::start(&state).await;
    failure(http.get(id, &q).await, 409, "sitemap_catalog_unavailable").await;
    http.finish().await;
    state.exports = ExportRegistry::default();
    let held = db(&dir, id).with_extension("sqlite-held");
    std::fs::rename(db(&dir, id), &held).unwrap();
    let http = Http::start(&state).await;
    failure(http.get(id, &q).await, 409, "sitemap_catalog_unavailable").await;
    assert_eq!(
        state.exports.snapshot().started,
        0,
        "missing DB precedes worker admission"
    );
    http.finish().await;
    std::fs::rename(&held, db(&dir, id)).unwrap();
    site.finish(requests).await;
}

async fn setup_fresh_small() -> (OwnedDirectory, AppState, Uuid) {
    let site = Fixture::new().await;
    let result = setup(&site, Mode::Native).await;
    let requests = site.count.load(Ordering::SeqCst);
    site.finish(requests).await;
    result
}
