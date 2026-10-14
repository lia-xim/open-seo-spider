//! AUTHORED / NOT RUN. Real HTTP/Crawler/SQLite fixtures plus explicitly labelled
//! SQL state-restoration controls. No Core-private writer or authored positive
//! measurement/catalog DTO is substituted for actual captured source records.
use super::*;
use open_seo_spider_core::{
    CrawlConfig, CrawlControl, CrawlOutcome, CrawlStore, Crawler, RecrawlSourceKind,
    SelectedRecrawlParent, SitemapSourceStoreOmission, prepare_selected_recrawl,
    sitemap_source_measurement_snapshot_context,
};
use rusqlite::{Connection, OptionalExtension, types::Value};
use std::collections::BTreeMap;

const SOURCE_TABLES: [&str; 2] = [
    "sitemap_source_measurements",
    "sitemap_source_measurement_commits",
];
const SOURCE_KEY: &str = "sitemap_source_store";

#[derive(Debug, Clone, PartialEq)]
struct ActualTable {
    columns: Vec<String>,
    rows: Vec<Vec<Value>>,
}
fn identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
fn actual_table(connection: &Connection, name: &str) -> ActualTable {
    let columns = connection
        .prepare(&format!("PRAGMA table_info({})", identifier(name)))
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert!(!columns.is_empty(), "Actual Core schema must exist: {name}");
    let mut statement = connection
        .prepare(&format!("SELECT * FROM {} ORDER BY 1", identifier(name)))
        .unwrap();
    let count = statement.column_count();
    let rows = statement
        .query_map([], |row| {
            (0..count)
                .map(|index| row.get::<_, Value>(index))
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    ActualTable { columns, rows }
}
#[derive(Debug, Clone)]
struct ActualSidecar {
    tables: BTreeMap<String, ActualTable>,
    metadata: Option<String>,
}
impl ActualSidecar {
    fn capture(db: &std::path::Path) -> Self {
        // Validate through the actual public same-TX reader before copying any
        // existing fixture bytes. No summary/accounting/hash is reconstructed.
        let _ = CrawlStore::open_read_only(db)
            .unwrap()
            .sitemap_read_snapshot()
            .unwrap();
        let mut connection =
            Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let tx = connection.transaction().unwrap();
        let tables = SOURCE_TABLES
            .iter()
            .map(|name| ((*name).to_owned(), actual_table(&tx, name)))
            .collect();
        let metadata = tx
            .query_row(
                "SELECT value FROM metadata WHERE key=?1",
                [SOURCE_KEY],
                |r| r.get(0),
            )
            .optional()
            .unwrap();
        tx.commit().unwrap();
        Self { tables, metadata }
    }
    fn restore(&self, db: &std::path::Path) {
        // Explicit SQL-controlled state restoration, NOT evidence of a natural
        // terminal A/B write. All row and metadata bytes came from actual Core
        // writes. The public loader must validate the restored state afterward.
        let mut connection = Connection::open(db).unwrap();
        let tx = connection.transaction().unwrap();
        for name in SOURCE_TABLES.iter().rev() {
            assert_eq!(actual_table(&tx, name).columns, self.tables[*name].columns);
            tx.execute(&format!("DELETE FROM {}", identifier(name)), [])
                .unwrap();
        }
        for name in SOURCE_TABLES {
            let table = &self.tables[name];
            let placeholders = (1..=table.columns.len())
                .map(|number| format!("?{number}"))
                .collect::<Vec<_>>()
                .join(",");
            let columns = table
                .columns
                .iter()
                .map(|name| identifier(name))
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "INSERT INTO {}({columns}) VALUES({placeholders})",
                identifier(name)
            );
            for row in &table.rows {
                tx.execute(&sql, rusqlite::params_from_iter(row.iter()))
                    .unwrap();
            }
        }
        tx.execute("DELETE FROM metadata WHERE key=?1", [SOURCE_KEY])
            .unwrap();
        if let Some(metadata) = &self.metadata {
            tx.execute(
                "INSERT INTO metadata(key,value) VALUES(?1,?2)",
                rusqlite::params![SOURCE_KEY, metadata],
            )
            .unwrap();
        }
        tx.commit().unwrap();
        drop(connection);
        let _ = CrawlStore::open_read_only(db)
            .unwrap()
            .sitemap_read_snapshot()
            .unwrap();
    }
}
fn non_source_sql(db: &std::path::Path) -> BTreeMap<String, ActualTable> {
    let mut connection =
        Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let tx = connection.transaction().unwrap();
    let names = tx
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let mut result = BTreeMap::new();
    for name in names {
        if SOURCE_TABLES.contains(&name.as_str()) || name.starts_with("sqlite_") {
            continue;
        }
        let mut table = actual_table(&tx, &name);
        if name == "metadata" {
            let key = table.columns.iter().position(|name| name == "key").unwrap();
            table
                .rows
                .retain(|row| row[key] != Value::Text(SOURCE_KEY.into()));
        }
        result.insert(name, table);
    }
    tx.commit().unwrap();
    result
}
fn make_legacy_source_history_fixture(stored: &LastmodStored) {
    // Explicit compatibility fixture based on a real terminal capture. Remove
    // only the new optional source history from SQL/report/checkpoint and bind
    // the exact rewritten report bytes as existing historical tests do. No
    // original page, catalog, coverage, config, generation or task is invented.
    let mut connection = Connection::open(&stored.db).unwrap();
    let tx = connection.transaction().unwrap();
    for table in SOURCE_TABLES.iter().rev() {
        tx.execute(&format!("DELETE FROM {}", identifier(table)), [])
            .unwrap();
    }
    tx.execute("DELETE FROM metadata WHERE key=?1", [SOURCE_KEY])
        .unwrap();
    let report: String = tx
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
    let mut report: open_seo_spider_core::CrawlReport = serde_json::from_str(&report).unwrap();
    let mut checkpoint: open_seo_spider_core::CrawlCheckpoint =
        serde_json::from_str(&checkpoint).unwrap();
    report.sitemap_source_measurements = None;
    checkpoint.sitemap_source_measurements = None;
    let report = serde_json::to_string(&report).unwrap();
    checkpoint.report_sha256 = digest(report.as_bytes());
    tx.execute("UPDATE metadata SET value=?1 WHERE key='report'", [report])
        .unwrap();
    tx.execute(
        "UPDATE metadata SET value=?1 WHERE key='checkpoint'",
        [serde_json::to_string(&checkpoint).unwrap()],
    )
    .unwrap();
    tx.commit().unwrap();
    drop(connection);
    assert!(stored.read().sitemap_source_measurements.is_none());
}
fn source_xml() -> String {
    "<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>{origin}/p1</loc></url><url><loc>{origin}/p2</loc></url><url><loc>{origin}/p3</loc></url></urlset>".into()
}
async fn crawl_config(config: CrawlConfig) -> LastmodStored {
    let job = Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("oss-s3-source-app-{job}"));
    std::fs::create_dir(&dir).unwrap();
    let db = dir.join(format!("{job}.sqlite"));
    let crawler = Crawler::new(config).unwrap().with_store(&db).unwrap();
    let completed = crawler
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(completed.outcome, CrawlOutcome::Completed);
    let config_sha = digest(encode(&completed.report.config).unwrap());
    drop(crawler);
    let read = CrawlStore::open_read_only(&db)
        .unwrap()
        .sitemap_read_snapshot()
        .unwrap();
    LastmodStored {
        dir,
        db,
        job,
        generation: read.generation.unwrap(),
        config_sha,
    }
}
async fn actual_pending_capture_from_same_config(stored: &LastmodStored) -> LastmodStored {
    let config = CrawlStore::open_read_only(&stored.db)
        .unwrap()
        .load_report()
        .unwrap()
        .unwrap()
        .config;
    let job = Uuid::new_v4();
    let dir = std::env::temp_dir().join(format!("oss-s3-pending-app-{job}"));
    std::fs::create_dir(&dir).unwrap();
    let db = dir.join(format!("{job}.sqlite"));
    let crawler = Crawler::new(config).unwrap().with_store(&db).unwrap();
    let writer = Connection::open(&db).unwrap();
    writer.execute_batch(
        "CREATE TRIGGER app_fixture_reject_source_b BEFORE INSERT ON sitemap_source_measurement_commits BEGIN SELECT RAISE(ABORT,'fixture source B rejected after actual A'); END;",
    ).unwrap();
    drop(writer);
    let failed = crawler.run_controlled(None, CrawlControl::new()).await;
    assert!(
        failed.is_err(),
        "The real B transaction must fail and roll back"
    );
    drop(crawler);
    let read = CrawlStore::open_read_only(&db)
        .unwrap()
        .sitemap_read_snapshot()
        .unwrap();
    let sidecar = read.sitemap_source_measurements.as_ref().unwrap();
    assert_eq!(sidecar.records.len(), 1);
    assert!(sidecar.commits.is_empty());
    assert_eq!(read.catalog.as_ref().unwrap().summary.document_rows, 0);
    assert_eq!(
        read.crawl_config_sha256.as_deref(),
        Some(stored.config_sha.as_str())
    );
    LastmodStored {
        dir,
        db,
        job,
        generation: read.generation.unwrap(),
        config_sha: read.crawl_config_sha256.unwrap(),
    }
}
struct ActualTokens {
    raw: String,
    page: String,
    export: String,
    csv: Vec<u8>,
}
fn assert_saved_analysis_context_mismatch(stored: &LastmodStored) {
    let read = actual_analysis_read(stored, &ReadQuery::default());
    let analysis = read.membership_analysis.unwrap();
    assert!(!analysis.context.available);
    assert_eq!(
        analysis.context.reason,
        "saved_source_measurement_context_mismatch"
    );
    assert!(analysis.saved_pages.is_none());
}
fn actual_tokens(stored: &LastmodStored) -> ActualTokens {
    let raw = stored.page(ReadQuery {
        limit: Some(1),
        ..Default::default()
    });
    let page = actual_analysis_page(
        stored,
        ReadQuery {
            limit: Some(1),
            page_limit: Some(1),
            ..Default::default()
        },
    );
    assert!(page.membership_analysis.as_ref().unwrap().context.available);
    let export = raw.snapshot_token.unwrap();
    let csv = csv(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(stored.read()),
        CsvQuery {
            kind: RowKind::All,
            source_observation_id: None,
            membership_url: None,
            snapshot: export.clone(),
        },
    )
    .unwrap()
    .0;
    ActualTokens {
        raw: raw.next_cursor.unwrap(),
        page: page.next_page_cursor.unwrap(),
        export,
        csv,
    }
}
fn assert_stale_all_roles(stored: &LastmodStored, before: &ActualTokens) {
    let raw = inventory(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(stored.read()),
        ReadQuery {
            limit: Some(1),
            cursor: Some(before.raw.clone()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(raw.code, "stale_sitemap_cursor");
    assert_eq!(raw.status(), 409);
    let page_query = ReadQuery {
        limit: Some(1),
        page_limit: Some(1),
        page_cursor: Some(before.page.clone()),
        ..Default::default()
    };
    let page_read = actual_analysis_read(stored, &page_query);
    let page = inventory(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(page_read),
        page_query,
    )
    .unwrap_err();
    assert_eq!(page.code, "stale_sitemap_cursor");
    assert_eq!(page.status(), 409);
    let export = csv(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(stored.read()),
        CsvQuery {
            kind: RowKind::All,
            source_observation_id: None,
            membership_url: None,
            snapshot: before.export.clone(),
        },
    )
    .unwrap_err();
    assert_eq!(export.code, "stale_sitemap_cursor");
    assert_eq!(export.status(), 409);
    let fresh = stored.page(ReadQuery::default());
    let fresh_csv = csv(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(stored.read()),
        CsvQuery {
            kind: RowKind::All,
            source_observation_id: None,
            membership_url: None,
            snapshot: fresh.snapshot_token.unwrap(),
        },
    )
    .unwrap()
    .0;
    assert_eq!(fresh_csv, before.csv, "Original raw CSV body is unchanged");
}
#[tokio::test]
async fn actual_source_a_only_restoration_stales_raw_page_and_csv_with_legacy_rows_unchanged() {
    let website = LastmodWebsite::start(source_xml()).await;
    let stored = LastmodStored::crawl(&website, true, false).await;
    let pending = actual_pending_capture_from_same_config(&stored).await;
    let actual_a_only = ActualSidecar::capture(&pending.db);
    make_legacy_source_history_fixture(&stored);
    let requests = website.count();
    let before = actual_tokens(&stored);
    let before_read = stored.read();
    assert!(before_read.sitemap_source_measurements.is_none());
    let non_source = non_source_sql(&stored.db);
    actual_a_only.restore(&stored.db);
    let after = stored.read();
    let source = after.sitemap_source_measurements.as_ref().unwrap();
    assert_eq!(source.records.len(), 1);
    assert!(
        source.commits.is_empty(),
        "A capture is not an SQL admission receipt"
    );
    assert_eq!(after.catalog, before_read.catalog);
    assert_eq!(after.coverage_inventory, before_read.coverage_inventory);
    assert_eq!(non_source_sql(&stored.db), non_source);
    assert_ne!(
        after.sitemap_source_measurement_context,
        before_read.sitemap_source_measurement_context
    );
    assert_saved_analysis_context_mismatch(&stored);
    assert_stale_all_roles(&stored, &before);
    website.finish(requests).await;
}
#[tokio::test]
async fn actual_source_association_only_restoration_stales_raw_page_and_csv_without_new_capture() {
    let website = LastmodWebsite::start(source_xml()).await;
    let stored = LastmodStored::crawl(&website, true, false).await;
    let pending = actual_pending_capture_from_same_config(&stored).await;
    let committed = ActualSidecar::capture(&stored.db);
    let unlinked = ActualSidecar::capture(&pending.db);
    assert_eq!(
        committed.tables[SOURCE_TABLES[0]], unlinked.tables[SOURCE_TABLES[0]],
        "Both states contain the identical actual captured A record"
    );
    assert!(!committed.tables[SOURCE_TABLES[1]].rows.is_empty());
    assert!(unlinked.tables[SOURCE_TABLES[1]].rows.is_empty());
    let requests = website.count();
    let before = actual_tokens(&stored);
    let before_read = stored.read();
    let non_source = non_source_sql(&stored.db);
    unlinked.restore(&stored.db);
    let after = stored.read();
    assert_eq!(after.catalog, before_read.catalog);
    assert_eq!(after.coverage_inventory, before_read.coverage_inventory);
    assert_eq!(non_source_sql(&stored.db), non_source);
    assert_ne!(
        after.sitemap_source_measurement_context,
        before_read.sitemap_source_measurement_context
    );
    assert_saved_analysis_context_mismatch(&stored);
    assert_stale_all_roles(&stored, &before);
    website.finish(requests).await;
}
#[tokio::test]
async fn actual_capture_unavailable_marker_restoration_stales_tokens_without_fake_empty_source() {
    let website = LastmodWebsite::start(source_xml()).await;
    let stored = crawl_config(CrawlConfig {
        seed_urls: vec![
            format!("{}/seed", website.origin),
            format!("{}/extra1", website.origin),
            format!("{}/extra2", website.origin),
        ],
        // Each actual task identity fits the existing8KiB context bound; three
        // copies in the strict record exceed its16KiB serialized record cap.
        sitemap_urls: vec![format!("{}/{}.xml", website.origin, "x".repeat(6000))],
        obey_robots: false,
        discover_sitemaps: false,
        crawl_links: false,
        concurrency: 1,
        delay_ms: 0,
        request_timeout_secs: 3,
        ..Default::default()
    })
    .await;
    let captured = stored.read();
    let source = captured.sitemap_source_measurements.as_ref().unwrap();
    assert_eq!(
        source.summary.omission,
        Some(SitemapSourceStoreOmission::CaptureUnavailable)
    );
    assert!(source.summary.omission_revision.is_some());
    assert!(source.records.is_empty());
    let actual_marker = ActualSidecar::capture(&stored.db);
    make_legacy_source_history_fixture(&stored);
    let requests = website.count();
    let before = actual_tokens(&stored);
    let non_source = non_source_sql(&stored.db);
    actual_marker.restore(&stored.db);
    let current = stored.read();
    assert_eq!(non_source_sql(&stored.db), non_source);
    assert_eq!(current.catalog, captured.catalog);
    let current = current.sitemap_source_measurements.unwrap();
    assert_eq!(
        current.summary.omission,
        Some(SitemapSourceStoreOmission::CaptureUnavailable)
    );
    assert!(
        current.records.is_empty(),
        "Zero retained records is not a zero-URL proof"
    );
    assert_saved_analysis_context_mismatch(&stored);
    assert_stale_all_roles(&stored, &before);
    website.finish(requests).await;
}
#[tokio::test]
async fn actual_historical_source_none_remains_explicit_stable_and_readonly() {
    let website = LastmodWebsite::start(source_xml()).await;
    let stored = LastmodStored::crawl(&website, true, false).await;
    let requests = website.count();
    let before = actual_tokens(&stored);
    make_legacy_source_history_fixture(&stored);
    let files = stored.file_hashes();
    let first = stored.read();
    let second = stored.read();
    assert!(first.sitemap_source_measurements.is_none());
    assert!(second.sitemap_source_measurements.is_none());
    assert_eq!(
        first.sitemap_source_measurement_context,
        sitemap_source_measurement_snapshot_context(None).unwrap()
    );
    assert_eq!(
        first.sitemap_source_measurement_context,
        second.sitemap_source_measurement_context
    );
    let page1 = stored.page(ReadQuery::default());
    let page2 = stored.page(ReadQuery::default());
    assert_eq!(page1.snapshot_token, page2.snapshot_token);
    assert!(page1.catalog_available);
    assert!(
        actual_analysis_page(&stored, ReadQuery::default())
            .membership_analysis
            .unwrap()
            .context
            .available,
        "Coherent historical absence is available target analysis with unknown source history"
    );
    assert_stale_all_roles(&stored, &before);
    assert_eq!(
        stored.file_hashes(),
        files,
        "Read-only historical absence must not initialize storage"
    );
    website.finish(requests).await;
}
#[tokio::test]
async fn actual_selected_parent_source_context_keeps_records_links_and_historical_none_separate() {
    let website = LastmodWebsite::start(source_xml()).await;
    let parent = LastmodStored::crawl(&website, true, false).await;
    let mut expected_requests = website.count();
    let parent_hashes = parent.file_hashes();
    let report = CrawlStore::open_read_only(&parent.db)
        .unwrap()
        .load_report()
        .unwrap()
        .unwrap();
    let parent_source = parent.read().sitemap_source_measurements.unwrap();
    for legacy in [false, true] {
        let mut actual_report = report.clone();
        if legacy {
            // Explicit compatibility snapshot derived from the real parent;
            // absent new optional data must remain historical unknown.
            actual_report.sitemap_source_measurements = None;
        }
        let bytes = serde_json::to_vec(&actual_report).unwrap();
        let parent_sha = digest(&bytes);
        let prepared = prepare_selected_recrawl(
            SelectedRecrawlParent {
                job_id: parent.job.to_string(),
                checkpoint_revision: None,
                source_kind: RecrawlSourceKind::StoredJson,
            },
            &bytes,
            &[format!("{}/p1", website.origin)],
            None,
        )
        .unwrap();
        let job = Uuid::new_v4();
        let dir = std::env::temp_dir().join(format!("oss-s3-selected-app-{job}"));
        std::fs::create_dir(&dir).unwrap();
        let db = dir.join(format!("{job}.sqlite"));
        assert_eq!(website.count(), expected_requests);
        let crawler = Crawler::from_saved_config(prepared.config.clone())
            .unwrap()
            .with_selected_recrawl(prepared)
            .unwrap()
            .with_store(&db)
            .unwrap();
        let done = crawler
            .run_controlled(None, CrawlControl::new())
            .await
            .unwrap();
        assert_eq!(done.outcome, CrawlOutcome::Completed);
        let config_sha = digest(encode(&done.report.config).unwrap());
        drop(crawler);
        expected_requests += 1;
        assert_eq!(
            website.count(),
            expected_requests,
            "Selected operation must not fetch source XML again"
        );
        let read = CrawlStore::open_read_only(&db)
            .unwrap()
            .sitemap_read_snapshot()
            .unwrap();
        let child = LastmodStored {
            dir,
            db,
            job,
            generation: read.generation.unwrap(),
            config_sha,
        };
        if legacy {
            assert!(read.sitemap_source_measurements.is_none());
            assert_eq!(
                read.sitemap_source_measurement_context,
                sitemap_source_measurement_snapshot_context(None).unwrap()
            );
        } else {
            let source = read.sitemap_source_measurements.as_ref().unwrap();
            assert_eq!(source.records, parent_source.records);
            assert_eq!(source.commits, parent_source.commits);
            let inherited = source.summary.inherited.as_ref().unwrap();
            let catalog_basis = read.catalog.as_ref().unwrap().inherited.as_ref().unwrap();
            assert_eq!(inherited.parent_snapshot_sha256, parent_sha);
            assert_eq!(
                inherited.parent_snapshot_sha256,
                catalog_basis.parent_snapshot_sha256
            );
            assert_eq!(
                inherited.parent_catalog_sha256,
                digest(encode(&parent_source).unwrap())
            );
            assert_eq!(
                catalog_basis.parent_catalog_sha256,
                digest(encode(report.sitemap_catalog.as_ref().unwrap()).unwrap())
            );
            assert!(!inherited.current_discovery_executed);
            assert!(!catalog_basis.current_discovery_executed);
            assert_eq!(
                read.sitemap_source_measurement_context,
                sitemap_source_measurement_snapshot_context(Some(source)).unwrap()
            );
        }
        let tokens = actual_tokens(&child);
        assert!(!tokens.raw.is_empty() && !tokens.page.is_empty() && !tokens.export.is_empty());
        if !legacy {
            // Negative SQL fixture: mutate the actual typed inherited basis,
            // retaining its fixed-width shape. A foreign parent must fail the
            // real loader, not become an alternative valid cursor context.
            let actual_sidecar = ActualSidecar::capture(&child.db);
            let mut wrong_parent = read
                .sitemap_source_measurements
                .as_ref()
                .unwrap()
                .summary
                .clone();
            let inherited = wrong_parent.inherited.as_mut().unwrap();
            inherited.parent_snapshot_sha256 = if inherited.parent_snapshot_sha256 == "a".repeat(64)
            {
                "b".repeat(64)
            } else {
                "a".repeat(64)
            };
            Connection::open(&child.db)
                .unwrap()
                .execute(
                    "UPDATE metadata SET value=?1 WHERE key=?2",
                    rusqlite::params![serde_json::to_string(&wrong_parent).unwrap(), SOURCE_KEY],
                )
                .unwrap();
            assert!(
                CrawlStore::open_read_only(&child.db)
                    .unwrap()
                    .sitemap_read_snapshot()
                    .is_err()
            );
            actual_sidecar.restore(&child.db);
            assert_eq!(
                child.read().sitemap_source_measurement_context,
                read.sitemap_source_measurement_context
            );
        }
        assert_eq!(parent.file_hashes(), parent_hashes);
    }
    website.finish(expected_requests).await;
}
