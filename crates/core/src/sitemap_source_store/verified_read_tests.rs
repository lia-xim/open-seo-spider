//! AUTHORED ONLY: no compiler, SQLite fixture, decoder or test executed under HOLD.
//! Registered as a child of existing source-store tests to reuse their actual
//! CrawlStore/observed-decoder/XML/A-before-B fixtures rather than a fake writer.
use super::*;
use crate::sitemap_ledger::verified_read::verify_raw;
use crate::sitemap_source_store::verified_read::verify_source;

fn compare_in_tx(connection: &Connection, selected: bool) -> Option<SitemapSourceStoreSummary> {
    assert!(!connection.is_autocommit());
    let portable = load(connection).unwrap();
    let raw = verify_raw(connection).unwrap();
    let verified = verify_source(connection, raw.as_ref()).unwrap();
    match (portable, verified) {
        (None, None) => None,
        (Some(portable), Some(verified)) => {
            assert_eq!(verified.summary(), &portable.summary);
            assert_eq!(
                verified.context(),
                snapshot_context(Some(&portable)).unwrap()
            );
            // Freeze v1's actual tuple/tags and field order independently of
            // either reader's shared context helper.
            assert_eq!(
                verified.context(),
                digest(
                    serde_json::to_vec(&(
                        SITEMAP_SOURCE_STORE_ENGINE,
                        "present",
                        &portable.summary,
                    ))
                    .unwrap()
                )
            );
            assert!(std::ptr::eq(verified.connection(), connection));
            assert_eq!(verified.materializing_selected_compatibility(), selected);
            Some(portable.summary)
        }
        _ => panic!("streamed/portable source presence differs"),
    }
}
fn compare(f: &Fixture) -> SitemapSourceStoreSummary {
    let read = f.connection.unchecked_transaction().unwrap();
    compare_in_tx(&read, false).unwrap()
}
fn both_reject(connection: &Connection) {
    assert!(load(connection).is_err());
    let raw = verify_raw(connection).unwrap();
    assert!(verify_source(connection, raw.as_ref()).is_err());
}

#[test]
fn verified_source_actual_empty_a_only_and_b_match_portable_accounting_and_v1_context() {
    let mut f = normal();
    let empty = compare(&f);
    assert_eq!(empty.record_count, 0);
    assert_eq!(empty.commit_count, 0);
    let raw_before = crate::sitemap_ledger::load_catalog(&f.connection).unwrap();
    let (batch, measurement) = source(&f, EMPTY);
    let receipt = persist(&mut f, &batch.document, &measurement);
    let a = compare(&f);
    assert_eq!((a.record_count, a.commit_count), (1, 0));
    assert_eq!(a.revision, empty.revision + 1);
    assert!(a.reserved_json_bytes >= SOURCE_COMMIT_RESERVE_BYTES);
    assert_ne!(a.content_sha256, empty.content_sha256);
    assert_eq!(
        crate::sitemap_ledger::load_catalog(&f.connection).unwrap(),
        raw_before
    );
    commit(&mut f, &receipt, batch).unwrap();
    let b = compare(&f);
    assert_eq!((b.record_count, b.commit_count), (1, 1));
    assert_eq!(b.revision, a.revision + 1);
    assert_eq!(b.content_sha256, a.content_sha256);
    assert_ne!(b.links_sha256, a.links_sha256);
    assert!(b.charged_json_bytes <= a.charged_json_bytes);
}

#[test]
fn verified_source_actual_capture_unknown_and_capacity_markers_keep_reserve_and_revision() {
    for capacity in [false, true] {
        let mut f = fixture(
            SitemapSourceStoreLimits {
                max_records: if capacity { 1 } else { 100_000 },
                ..Default::default()
            },
            Default::default(),
        );
        let (first, m1) = source(&f, EMPTY);
        persist(&mut f, &first.document, &m1);
        let before = compare(&f);
        let (second, m2) = source(&f, &gzip(EMPTY, 77));
        let receipt = if capacity {
            persist(&mut f, &second.document, &m2)
        } else {
            let tx = f.connection.transaction().unwrap();
            let receipt = record_unavailable_a(
                &tx,
                &second.document,
                &m2.payload.binding.crawl_config_sha256,
            )
            .unwrap();
            tx.commit().unwrap();
            receipt.after_a_commit(&f.connection).unwrap()
        };
        assert!(!receipt.is_persisted());
        let after = compare(&f);
        assert_eq!(after.record_count, 1);
        assert_eq!(after.commit_count, 0);
        assert_eq!(after.revision, before.revision + 1);
        assert_eq!(after.omission_revision, Some(after.revision));
        assert_eq!(after.content_sha256, before.content_sha256);
        assert_eq!(
            after.omission,
            Some(if capacity {
                SitemapSourceStoreOmission::StorageBudgetExceeded
            } else {
                SitemapSourceStoreOmission::CaptureUnavailable
            })
        );
        assert!(after.reserved_json_bytes >= SOURCE_COMMIT_RESERVE_BYTES);
        assert_eq!(done(&f), 0);
    }
}

#[test]
fn verified_source_checks_last_unjoined_record_and_whole_summary_not_only_first_link() {
    let mut f = normal();
    let (first, m1) = source(&f, EMPTY);
    let receipt = persist(&mut f, &first.document, &m1);
    let (tail, m2) = source(&f, &gzip(EMPTY, 12));
    persist(&mut f, &tail.document, &m2);
    commit(&mut f, &receipt, first).unwrap();
    let original = snapshot(&f);
    compare(&f);
    assert_eq!((original.records.len(), original.commits.len()), (2, 1));
    // Explicit SQL-corruption fixture. The canonical tail remains a valid
    // typed capture; the unchanged rolling summary must still detect it.
    let mut tail = original.records[1].clone();
    tail.staged_document_sha256 = digest(b"resealed-unjoined-tail");
    let tail_data = record_json(&tail, &original.summary.limits).unwrap();
    {
        let tx = f.connection.transaction().unwrap();
        tx.execute(
            "UPDATE sitemap_source_measurements SET data=?1,data_sha256=?2 WHERE sequence=2",
            params![tail_data, digest(&tail_data)],
        )
        .unwrap();
        both_reject(&tx);
        tx.rollback().unwrap();
    }
    for sql in [
        "UPDATE sitemap_source_measurements SET task_url='https://wrong.example/tail' WHERE sequence=2",
        "UPDATE sitemap_source_measurements SET sequence=9 WHERE sequence=2",
        "UPDATE metadata SET value=json_set(value,'$.reserved_json_bytes',0) WHERE key='sitemap_source_store'",
        "UPDATE metadata SET value=json_set(value,'$.omission_revision',1) WHERE key='sitemap_source_store'",
    ] {
        let tx = f.connection.transaction().unwrap();
        tx.execute_batch(sql).unwrap();
        both_reject(&tx);
        tx.rollback().unwrap();
    }
    assert_eq!(snapshot(&f), original);
}

#[test]
fn verified_source_resealed_link_and_record_still_need_actual_raw_document_seals() {
    let mut f = normal();
    let (batch, m) = source(&f, EMPTY);
    let receipt = persist(&mut f, &batch.document, &m);
    commit(&mut f, &receipt, batch).unwrap();
    let original = snapshot(&f);
    for fault in ["document_payload", "stage", "missing_record"] {
        // Recompute portable summary and envelope hashes intentionally: these
        // are controlled corruptions, not claims of a production writer path.
        let mut changed = original.clone();
        match fault {
            "document_payload" => {
                changed.commits[0].document_payload_sha256 = digest(b"wrong-document")
            }
            "stage" => {
                changed.records[0].staged_document_sha256 = digest(b"wrong-stage");
                changed.commits[0].measurement_envelope_sha256 =
                    digest(record_json(&changed.records[0], &changed.summary.limits).unwrap());
            }
            _ => changed.commits[0].measurement_id = digest(b"absent-measurement"),
        }
        refresh(&mut changed).unwrap();
        if fault != "missing_record" {
            validate_snapshot(&changed).unwrap();
        }
        let record = record_json(&changed.records[0], &changed.summary.limits).unwrap();
        let link = commit_json(&changed.commits[0]).unwrap();
        if fault == "missing_record" {
            f.connection
                .execute_batch("PRAGMA foreign_keys=OFF;")
                .unwrap();
        }
        let tx = f.connection.transaction().unwrap();
        tx.execute(
            "UPDATE sitemap_source_measurements SET data=?1,data_sha256=?2 WHERE sequence=1",
            params![record, digest(&record)],
        )
        .unwrap();
        tx.execute("UPDATE sitemap_source_measurement_commits SET measurement_id=?1,data=?2,data_sha256=?3",
            params![changed.commits[0].measurement_id, link, digest(&link)]).unwrap();
        save_summary(&tx, &changed.summary).unwrap();
        both_reject(&tx);
        tx.rollback().unwrap();
        if fault == "missing_record" {
            f.connection
                .execute_batch("PRAGMA foreign_keys=ON;")
                .unwrap();
        }
    }
    compare(&f);
}

#[test]
fn verified_source_detects_duplicate_ids_even_if_sql_unique_constraint_was_removed() {
    let mut f = normal();
    let (batch, measurement) = source(&f, EMPTY);
    persist(&mut f, &batch.document, &measurement);
    let mut changed = snapshot(&f);
    // Controlled damaged schema keeps the exact existing column/PK shape.
    // Foreign-key disabling is fixture-only and restored before verification.
    f.connection.execute_batch("PRAGMA foreign_keys=OFF;
        CREATE TABLE weakened_records(sequence INTEGER PRIMARY KEY,measurement_id TEXT,source_kind TEXT,task_url TEXT,generation INTEGER,source_revision TEXT,expected_document_id TEXT,capture_config_sha256 TEXT,initial_catalog_config_sha256 TEXT,data TEXT,data_sha256 TEXT);
        INSERT INTO weakened_records SELECT * FROM sitemap_source_measurements;
        DROP TABLE sitemap_source_measurements;
        ALTER TABLE weakened_records RENAME TO sitemap_source_measurements;").unwrap();
    let mut duplicate = changed.records[0].clone();
    duplicate.sequence = 2;
    insert_record(&f.connection, &duplicate, &changed.summary.limits).unwrap();
    changed.records.push(duplicate);
    refresh(&mut changed).unwrap();
    save_summary(&f.connection, &changed.summary).unwrap();
    f.connection
        .execute_batch("PRAGMA foreign_keys=ON;")
        .unwrap();
    check_schema(&f.connection).unwrap();
    let read = f.connection.unchecked_transaction().unwrap();
    let old_error = load(&read).err().expect("portable duplicate must fail");
    assert!(
        old_error
            .to_string()
            .contains("Duplicate source measurement")
    );
    let raw = verify_raw(&read).unwrap();
    let error = verify_source(&read, raw.as_ref())
        .err()
        .expect("streamed duplicate must fail");
    assert!(
        error
            .to_string()
            .contains("Duplicate source measurement/commit identity")
    );
}

#[test]
fn verified_source_needs_actual_transaction_and_same_connection_raw_proof() {
    let f = normal();
    assert!(verify_source(&f.connection, None).is_err());
    let other = Connection::open(&f.path).unwrap();
    let read = f.connection.unchecked_transaction().unwrap();
    let other_read = other.unchecked_transaction().unwrap();
    let raw = verify_raw(&read).unwrap().unwrap();
    assert!(verify_source(&other_read, Some(&raw)).is_err());
    assert!(verify_source(&read, None).is_err());
    let token = verify_source(&read, Some(&raw)).unwrap().unwrap();
    assert!(std::ptr::eq(token.connection(), &*read));
}

#[test]
fn verified_source_historical_none_empty_schema_and_partial_schema_are_distinct() {
    let f = normal();
    let empty = compare(&f);
    assert_eq!(empty.record_count, 0);
    f.connection.execute_batch("DELETE FROM metadata WHERE key='sitemap_source_store'; DROP TABLE sitemap_source_measurement_commits; DROP TABLE sitemap_source_measurements;").unwrap();
    {
        let read = f.connection.unchecked_transaction().unwrap();
        assert!(compare_in_tx(&read, false).is_none());
    }
    f.connection.execute_batch(SCHEMA).unwrap();
    {
        let read = f.connection.unchecked_transaction().unwrap();
        assert!(compare_in_tx(&read, false).is_none());
    }
    f.connection
        .execute_batch("DROP TABLE sitemap_source_measurement_commits;")
        .unwrap();
    let read = f.connection.unchecked_transaction().unwrap();
    both_reject(&read);
}

fn selected_child(present: bool) -> (tempfile::TempDir, CrawlStore, Connection) {
    let mut f = normal();
    let (batch, m) = source(&f, EMPTY);
    let receipt = persist(&mut f, &batch.document, &m);
    commit(&mut f, &receipt, batch).unwrap();
    let report = CrawlReport {
        config: actual_config(&f.connection).unwrap().1,
        pages: vec![PageRecord {
            url: "https://source-store.example/page".into(),
            ..Default::default()
        }],
        sitemap_catalog: crate::sitemap_ledger::load_catalog(&f.connection).unwrap(),
        sitemap_source_measurements: present.then(|| snapshot(&f)),
        issues: vec![],
        warnings: vec![],
        summary: Default::default(),
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_coverage_inventory: None,
    };
    let bytes = serde_json::to_vec_pretty(&report).unwrap();
    let prepared = crate::prepare_selected_recrawl(
        SelectedRecrawlParent {
            job_id: "stream-source-parent".into(),
            checkpoint_revision: None,
            source_kind: RecrawlSourceKind::StoredJson,
        },
        &bytes,
        &["https://source-store.example/page".into()],
        None,
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("selected.sqlite");
    let child = CrawlStore::open(&path).unwrap();
    child.initialize_selected_recrawl(&prepared).unwrap();
    let connection = Connection::open(&path).unwrap();
    (temp, child, connection)
}

#[test]
fn verified_source_selected_exact_parent_some_and_none_remain_explicit_compatibility() {
    for present in [false, true] {
        let (_temp, _child, connection) = selected_child(present);
        {
            let read = connection.unchecked_transaction().unwrap();
            assert_eq!(compare_in_tx(&read, true).is_some(), present);
        }
        connection
            .execute(
                "UPDATE recrawl_base SET snapshot=CAST(snapshot || ' ' AS BLOB) WHERE id=1",
                [],
            )
            .unwrap();
        let read = connection.unchecked_transaction().unwrap();
        // Corrupt parent bytes must fail even for historical SourceNone. Raw
        // may reject first; either way no successful source header is emitted.
        match verify_raw(&read) {
            Err(_) => {}
            Ok(raw) => assert!(verify_source(&read, raw.as_ref()).is_err()),
        }
    }
}

#[test]
fn verified_source_selected_parent_some_current_none_delete_all_never_becomes_historical() {
    let (_temp, _child, connection) = selected_child(true);
    connection.execute_batch("DELETE FROM sitemap_source_measurement_commits; DELETE FROM sitemap_source_measurements; DELETE FROM metadata WHERE key='sitemap_source_store';").unwrap();
    let read = connection.unchecked_transaction().unwrap();
    assert!(load(&read).unwrap().is_none()); // Old low-level absence alone is insufficient.
    let raw = verify_raw(&read).unwrap();
    assert!(verify_source(&read, raw.as_ref()).is_err());
}
