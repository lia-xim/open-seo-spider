//! AUTHORED / NOT_COMPILED / NOT_RUN. Disk SQLite + real decoder/parser.
//! Transport is caller-declared stored-observation test data, not actual HTTP.
use super::*;
use crate::sitemap_source_archive::SourceBodyCandidate;
const TASK: &str = "https://body.example/sitemap.xml";
fn entry() -> DiscoveryEntry {
    DiscoveryEntry {
        kind: DiscoveryKind::Sitemap,
        url: TASK.into(),
    }
}
fn xml() -> Vec<u8> {
    b"\xef\xbb\xbf<?xml version='1.0'?>\r\n<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><!-- original &amp; whitespace --></urlset>".to_vec()
}
fn fixture(
    profile: Option<crate::SitemapSourceArchiveConfig>,
) -> (tempfile::TempDir, CrawlStore, u64) {
    let temp = tempfile::tempdir().unwrap();
    let store = CrawlStore::open(temp.path().join("body.sqlite")).unwrap();
    let mut config = CrawlConfig {
        seed_urls: vec!["https://body.example/".into()],
        sitemap_urls: vec![TASK.into()],
        explicit_sitemap_inputs: Some(vec![TASK.into()]),
        sitemap_source_archive: profile,
        obey_robots: false,
        ..Default::default()
    };
    config.prepare_input_lists().unwrap();
    store.prepare(&config).unwrap();
    let generation = store.begin_run("archive-stored-observation-test").unwrap();
    let hash = sha256(
        &metadata_text(&store.connection.lock().unwrap(), "config")
            .unwrap()
            .unwrap(),
    );
    store
        .initialize_sitemap_discovery(
            &[entry()],
            generation,
            &SitemapCatalogInit {
                config_sha256: hash,
                limits: Default::default(),
                roots: vec![],
                legacy_history_unknown: false,
            },
        )
        .unwrap();
    (temp, store, generation)
}
fn batch(generation: u64, entity: &[u8]) -> crate::SitemapDiscoveryBatch {
    let decoded = crate::decode_sitemap_entity(entity, &Default::default()).unwrap();
    let parser = crate::observe_sitemap_xml(
        &decoded.xml,
        crate::SitemapParserContext {
            requested_document_url: Some(TASK.into()),
            final_document_url: Some(TASK.into()),
            originating_occurrence_id: None,
        },
        &Default::default(),
    )
    .unwrap();
    let revision = crate::sitemap_ledger::digest(
        serde_json::to_vec(&(
            "sitemap-decoded-xml-source-v1",
            TASK,
            &decoded.decoded_xml_sha256,
        ))
        .unwrap(),
    );
    let hash = crate::sitemap_ledger::digest(entity);
    crate::sitemap_discovery::xml_batch(
        TASK,
        generation,
        &revision,
        crate::SitemapTransportFacts {
            request_started: true,
            requested_url: Some(TASK.into()),
            final_url: Some(TASK.into()),
            initial_status: Some(200),
            final_status: Some(200),
            blocked_by_robots: false,
            body_complete: true,
            post_http_decoding_bytes: entity.len() as u64,
            post_http_decoding_sha256: Some(hash.clone()),
            retained_post_http_prefix_sha256: Some(hash),
            failure_code: None,
        },
        Some(&decoded),
        Some(parser),
        None,
    )
    .unwrap()
}
fn a(
    store: &CrawlStore,
    generation: u64,
    batch: &crate::SitemapDiscoveryBatch,
    entity: &[u8],
) -> crate::sitemap_source_store::SourceMeasurementReceipt {
    let hash = sha256(
        &metadata_text(&store.connection.lock().unwrap(), "config")
            .unwrap()
            .unwrap(),
    );
    let limit = store
        .config()
        .unwrap()
        .unwrap()
        .sitemap_source_body_limit()
        .unwrap();
    let measurement = crate::crawler::test_sitemap_measurement_from_stored_transport(
        &batch.document,
        entity,
        hash,
        limit,
    );
    store
        .record_sitemap_source_capture(&entry(), &batch.document, &measurement, generation)
        .unwrap()
}
fn candidate(entity: Vec<u8>) -> SourceBodyCandidate {
    let decoded = crate::decode_sitemap_entity(&entity, &Default::default()).unwrap();
    SourceBodyCandidate {
        post_http_entity: Some(entity),
        decoded_xml: Some(decoded.xml),
    }
}
fn snapshot(store: &CrawlStore) -> crate::SitemapSourceArchiveSnapshot {
    match store.sitemap_source_archive_inspection().unwrap() {
        crate::SitemapSourceArchiveInspection::VerifiedLocal(s) => s,
        other => panic!("{other:?}"),
    }
}
fn record_count(store: &CrawlStore, table: &str) -> i64 {
    store
        .connection
        .lock()
        .unwrap()
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}
fn measurements(store: &CrawlStore) -> Option<crate::SitemapSourceStoreSnapshot> {
    store
        .sitemap_read_snapshot()
        .unwrap()
        .sitemap_source_measurements
}

#[test]
fn source_body_archive_commits_actual_bytes_after_a_and_reopens_before_membership_b() {
    let (temp, store, generation) = fixture(Some(Default::default()));
    let body = xml();
    let unit = batch(generation, &body);
    let receipt = a(&store, generation, &unit, &body);
    let a_before = measurements(&store);
    store
        .record_sitemap_source_bodies(
            &unit.document,
            &receipt,
            candidate(body.clone()),
            generation,
        )
        .unwrap();
    let archived = snapshot(&store);
    assert_eq!(archived.summary.captures, 1);
    assert_eq!(archived.summary.body_bytes, 2 * body.len() as u64);
    assert_eq!(archived.captures[0].bodies.len(), 2);
    assert_eq!(record_count(&store, "sitemap_documents"), 0);
    for b in &archived.captures[0].bodies {
        assert_eq!(
            std::fs::read(
                temp.path()
                    .join("body.sqlite.sitemap-bodies")
                    .join(format!("{}.bin", b.body_id))
            )
            .unwrap(),
            body
        );
    }
    assert_eq!(measurements(&store), a_before);
    drop(store);
    let reopened = CrawlStore::open_read_only(temp.path().join("body.sqlite")).unwrap();
    assert_eq!(snapshot(&reopened), archived);
}

#[test]
fn source_body_archive_membership_b_rollback_preserves_a_and_body_commit() {
    let (_temp, store, generation) = fixture(Some(Default::default()));
    let body = xml();
    let unit = batch(generation, &body);
    let receipt = a(&store, generation, &unit, &body);
    store
        .record_sitemap_source_bodies(&unit.document, &receipt, candidate(body), generation)
        .unwrap();
    let archived = snapshot(&store);
    store.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_member_document BEFORE INSERT ON sitemap_documents BEGIN SELECT RAISE(FAIL,'body foundation B rollback'); END;").unwrap();
    assert!(
        store
            .complete_discovery_measured(&entry(), &[], &[], &[], generation, unit, &receipt)
            .is_err()
    );
    assert_eq!(snapshot(&store), archived);
    assert_eq!(record_count(&store, "sitemap_source_measurements"), 1);
    assert_eq!(record_count(&store, "sitemap_documents"), 0);
    let done: i64 = store
        .connection
        .lock()
        .unwrap()
        .query_row(
            "SELECT done FROM discovery_frontier WHERE kind='sitemap' AND url=?1",
            [TASK],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(done, 0);
}

#[test]
fn source_body_archive_pair_budget_is_all_or_none_and_does_not_change_measurement() {
    let body = xml();
    let profile = crate::SitemapSourceArchiveConfig {
        max_total_body_bytes: body.len() as u64,
        ..Default::default()
    };
    let (temp, store, generation) = fixture(Some(profile));
    let unit = batch(generation, &body);
    let receipt = a(&store, generation, &unit, &body);
    let before = measurements(&store);
    store
        .record_sitemap_source_bodies(&unit.document, &receipt, candidate(body), generation)
        .unwrap();
    let archived = snapshot(&store);
    assert_eq!(archived.summary.body_bytes, 0);
    assert!(archived.captures[0].bodies.is_empty());
    assert_eq!(
        archived.captures[0].omission,
        Some(crate::SitemapArchiveOmission::TotalByteBudget)
    );
    assert_eq!(measurements(&store), before);
    assert_eq!(
        std::fs::read_dir(temp.path().join("body.sqlite.sitemap-bodies"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn source_body_archive_sql_insert_failure_keeps_sealed_orphans_unadvertised_and_retryable() {
    let (temp, store, generation) = fixture(Some(Default::default()));
    let body = xml();
    let unit = batch(generation, &body);
    let receipt = a(&store, generation, &unit, &body);
    store.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_body BEFORE INSERT ON sitemap_body_archive_captures BEGIN SELECT RAISE(FAIL,'body metadata rollback'); END;").unwrap();
    assert!(
        store
            .record_sitemap_source_bodies(
                &unit.document,
                &receipt,
                candidate(body.clone()),
                generation
            )
            .is_err()
    );
    assert_eq!(snapshot(&store).summary.captures, 0);
    assert_eq!(record_count(&store, "sitemap_source_measurements"), 1);
    assert_eq!(
        std::fs::read_dir(temp.path().join("body.sqlite.sitemap-bodies"))
            .unwrap()
            .count(),
        2
    );
    store
        .connection
        .lock()
        .unwrap()
        .execute_batch("DROP TRIGGER fail_body;")
        .unwrap();
    store
        .record_sitemap_source_bodies(&unit.document, &receipt, candidate(body), generation)
        .unwrap();
    assert_eq!(snapshot(&store).summary.captures, 1);
}

#[test]
fn source_body_archive_rejects_stale_generation_wrong_bytes_and_file_corruption() {
    let (temp, store, generation) = fixture(Some(Default::default()));
    let body = xml();
    let unit = batch(generation, &body);
    let receipt = a(&store, generation, &unit, &body);
    assert!(
        store
            .record_sitemap_source_bodies(
                &unit.document,
                &receipt,
                candidate(body.clone()),
                generation + 1
            )
            .is_err()
    );
    let mut wrong = candidate(body.clone());
    wrong.post_http_entity.as_mut().unwrap()[0] ^= 1;
    assert!(
        store
            .record_sitemap_source_bodies(&unit.document, &receipt, wrong, generation)
            .is_err()
    );
    assert_eq!(snapshot(&store).summary.captures, 0);
    store
        .record_sitemap_source_bodies(&unit.document, &receipt, candidate(body), generation)
        .unwrap();
    let saved = snapshot(&store);
    let file = temp
        .path()
        .join("body.sqlite.sitemap-bodies")
        .join(format!("{}.bin", saved.captures[0].bodies[0].body_id));
    std::fs::write(file, b"corrupt").unwrap();
    assert!(store.sitemap_source_archive_inspection().is_err());
}

#[test]
fn source_body_archive_absence_and_partial_schema_never_become_empty_success() {
    let (temp, legacy, _) = fixture(None);
    assert_eq!(
        legacy.sitemap_source_archive_inspection().unwrap(),
        crate::SitemapSourceArchiveInspection::NotRecordedOrLegacyUnknown
    );
    assert!(!temp.path().join("body.sqlite.sitemap-bodies").exists());
    let (_temp, store, _) = fixture(Some(Default::default()));
    store
        .connection
        .lock()
        .unwrap()
        .execute_batch("DROP TABLE sitemap_body_archive_captures;")
        .unwrap();
    assert!(store.sitemap_source_archive_inspection().is_err());
}

#[test]
fn source_body_archive_reopen_rejects_oversized_or_wrong_type_sql_identity_columns_before_allocation()
 {
    // Each mutation uses a fresh real disk SQLite catalog. A and body files stay intact.
    for (table, column) in [
        ("sitemap_body_archive_state", "data_sha256"),
        ("sitemap_body_archive_captures", "data_sha256"),
        ("sitemap_body_archive_captures", "measurement_id"),
    ] {
        for blob in [false, true] {
            let (_temp, store, generation) = fixture(Some(Default::default()));
            let body = xml();
            let unit = batch(generation, &body);
            let receipt = a(&store, generation, &unit, &body);
            store
                .record_sitemap_source_bodies(&unit.document, &receipt, candidate(body), generation)
                .unwrap();
            let before = measurements(&store);
            let connection = store.connection.lock().unwrap();
            connection
                .execute_batch("PRAGMA foreign_keys=OFF;")
                .unwrap();
            let sql = if blob {
                format!("UPDATE {table} SET {column}=zeroblob(1048576)")
            } else {
                format!("UPDATE {table} SET {column}=?1")
            };
            if blob {
                connection.execute(&sql, []).unwrap();
            } else {
                connection.execute(&sql, ["z".repeat(1048576)]).unwrap();
            }
            drop(connection);
            let error = store
                .sitemap_source_archive_inspection()
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("before allocation")
                    || error.contains("preflight budget or identity column"),
                "{table}.{column}: {error}"
            );
            assert_eq!(measurements(&store), before);
        }
    }
}

// The archive's new storage policy is deliberately narrower than the raw ledger.
// Actual UTF16 databases are refused, not represented as empty verified archives.
// These are authored SQLite/FS regressions, NOT_RUN and not native HTTP evidence.
fn assert_source_body_archive_utf16_refusal_without_archive_side_effects(encoding: &str) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("body.sqlite");
    let database = Connection::open(&path).unwrap();
    database
        .execute_batch(match encoding {
            "UTF-16le" => "PRAGMA encoding='UTF-16le';",
            "UTF-16be" => "PRAGMA encoding='UTF-16be';",
            _ => unreachable!(),
        })
        .unwrap();
    database
        .execute_batch(
            "CREATE TABLE encoding_anchor(value TEXT); INSERT INTO encoding_anchor VALUES('é💡');",
        )
        .unwrap();
    drop(database);
    let store = CrawlStore::open(&path).unwrap();
    let mut config = CrawlConfig {
        seed_urls: vec!["https://body.example/".into()],
        sitemap_urls: vec![TASK.into()],
        explicit_sitemap_inputs: Some(vec![TASK.into()]),
        sitemap_source_archive: Some(Default::default()),
        obey_robots: false,
        ..Default::default()
    };
    config.prepare_input_lists().unwrap();
    store.prepare(&config).unwrap();
    let generation = store.begin_run("archive-utf16-refusal-test").unwrap();
    let saved = metadata_text(&store.connection.lock().unwrap(), "config")
        .unwrap()
        .unwrap();
    let init = SitemapCatalogInit {
        config_sha256: sha256(&saved),
        limits: Default::default(),
        roots: vec![],
        legacy_history_unknown: false,
    };
    let metadata_rows = |store: &CrawlStore| -> Vec<(String, String)> {
        let c = store.connection.lock().unwrap();
        let mut s = c
            .prepare("SELECT key,value FROM metadata ORDER BY key COLLATE BINARY")
            .unwrap();
        let values = s
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        values
    };
    let before = metadata_rows(&store);
    let sidecar = crate::sitemap_source_archive::files::archive_path(&path).unwrap();
    let error = store
        .initialize_sitemap_discovery(&[entry()], generation, &init)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Source body archive requires UTF-8 SQLite storage"),
        "{encoding}: {error:#}"
    );
    assert_eq!(
        metadata_rows(&store),
        before,
        "the complete discovery initialization transaction must roll back"
    );
    for table in [
        "sitemap_roots",
        "sitemap_documents",
        "sitemap_entries",
        "sitemap_locations",
        "sitemap_source_measurements",
        "discovery_frontier",
    ] {
        assert_eq!(record_count(&store, table), 0, "{encoding}: {table}");
    }
    assert!(
        !sidecar.exists(),
        "refusal must not create a body directory or body files"
    );
    {
        let mut c = store.connection.lock().unwrap();
        let tx = c.transaction().unwrap();
        // The absent feature retains its legacy branch, including on UTF16.
        crate::sitemap_source_archive::initialize(&tx, &CrawlConfig::default(), &"a".repeat(64))
            .unwrap();
        tx.execute(
            "UPDATE metadata SET value=?1 WHERE key='config'",
            [serde_json::to_string(&CrawlConfig::default()).unwrap()],
        )
        .unwrap();
        assert!(crate::sitemap_source_archive::load(&tx).unwrap().is_none());
        tx.execute("UPDATE metadata SET value=?1 WHERE key='config'", [&saved])
            .unwrap();
        assert_eq!(
            crate::sitemap_source_archive::load(&tx)
                .unwrap_err()
                .to_string(),
            "Source body archive requires UTF-8 SQLite storage"
        );
        // Saved opt-in is authoritative for reads even before catalog creation.
        let error =
            crate::sitemap_source_archive::initialize(&tx, &config, &sha256(&saved)).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Source body archive requires UTF-8 SQLite storage")
        );
        tx.rollback().unwrap();
        // Deliberately unsupported imported table shape. No descriptor/body is
        // trusted: presence alone requires UTF8 before other archive reads.
        c.execute_batch(include_str!("sitemap_source_archive_schema.sql"))
            .unwrap();
    }
    assert!(!sidecar.exists());
    drop(store);
    let reopened = CrawlStore::open_read_only(&path).unwrap();
    let connection = reopened.connection.lock().unwrap();
    let actual: String = connection
        .query_row("PRAGMA encoding", [], |r| r.get(0))
        .unwrap();
    assert_eq!(actual, encoding);
    let bytes_before = std::fs::read(&path).unwrap();
    drop(connection);
    let error = reopened.sitemap_source_archive_inspection().unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Source body archive requires UTF-8 SQLite storage"),
        "{encoding}: {error:#}"
    );
    assert!(!sidecar.exists());
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes_before,
        "read-only refusal must preserve database bytes"
    );
}

#[test]
fn source_body_archive_utf16le_opt_in_and_existing_catalog_are_explicitly_refused_without_body_writes()
 {
    assert_source_body_archive_utf16_refusal_without_archive_side_effects("UTF-16le");
}

#[test]
fn source_body_archive_utf16be_opt_in_and_existing_catalog_are_explicitly_refused_without_body_writes()
 {
    assert_source_body_archive_utf16_refusal_without_archive_side_effects("UTF-16be");
}

#[test]
fn source_body_archive_utf8_non_ascii_original_bytes_and_quotas_survive_read_only_reopen() {
    let body="\u{feff}<?xml version='1.0'?>\r\n<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><!-- über é 💡 original &amp; bytes --></urlset>".as_bytes().to_vec();
    for total in [2 * body.len() as u64, 2 * body.len() as u64 - 1] {
        let profile = crate::SitemapSourceArchiveConfig {
            max_total_body_bytes: total,
            ..Default::default()
        };
        let (temp, store, generation) = fixture(Some(profile));
        let connection = store.connection.lock().unwrap();
        let encoding: String = connection
            .query_row("PRAGMA encoding", [], |r| r.get(0))
            .unwrap();
        assert_eq!(encoding, "UTF-8");
        drop(connection);
        let unit = batch(generation, &body);
        let receipt = a(&store, generation, &unit, &body);
        let before = measurements(&store);
        store
            .record_sitemap_source_bodies(
                &unit.document,
                &receipt,
                candidate(body.clone()),
                generation,
            )
            .unwrap();
        let saved = snapshot(&store);
        if total == 2 * body.len() as u64 {
            assert_eq!(saved.summary.body_bytes, total);
            assert_eq!(saved.captures[0].bodies.len(), 2);
            for b in &saved.captures[0].bodies {
                let actual = std::fs::read(
                    temp.path()
                        .join("body.sqlite.sitemap-bodies")
                        .join(format!("{}.bin", b.body_id)),
                )
                .unwrap();
                assert_eq!(actual, body);
                assert_eq!(b.bytes, body.len() as u64);
                assert_eq!(b.sha256, crate::sitemap_source_archive::digest(&body));
            }
        } else {
            assert_eq!(saved.summary.body_bytes, 0);
            assert!(saved.captures[0].bodies.is_empty());
            assert_eq!(
                saved.captures[0].omission,
                Some(crate::SitemapArchiveOmission::TotalByteBudget)
            );
        }
        assert_eq!(measurements(&store), before);
        drop(store);
        let reopened = CrawlStore::open_read_only(temp.path().join("body.sqlite")).unwrap();
        assert_eq!(snapshot(&reopened), saved);
        assert_eq!(measurements(&reopened), before);
    }
}
