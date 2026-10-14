//! AUTHORED / NOT COMPILED / NOT RUN under compute HOLD.
//! Real temp SQLite + actual CrawlStore bootstrap, decoder, XML scanner/parser,
//! legacy ledger append/validation. No substitute persistence implementation.
use super::*;
use crate::*;
use flate2::{Compression, GzBuilder};

const TASK: &str = "https://source-store.example/sitemap.xml";
const EMPTY: &[u8] = b"<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'/>";
struct Fixture {
    _temp: tempfile::TempDir,
    store: crate::CrawlStore,
    connection: Connection,
    path: std::path::PathBuf,
    generation: u64,
}
fn fixture(limits: SitemapSourceStoreLimits, ledger_limits: SitemapLedgerLimits) -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sources.sqlite");
    let store = crate::CrawlStore::open(&path).unwrap();
    let mut config = CrawlConfig {
        seed_urls: vec!["https://source-store.example/".into()],
        sitemap_urls: vec![TASK.into()],
        obey_robots: false,
        ..Default::default()
    };
    config.prepare_input_lists().unwrap();
    store.prepare(&config).unwrap();
    let generation = store.begin_run("source-store-real-sqlite").unwrap();
    let mut connection = Connection::open(&path).unwrap();
    connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    connection.execute_batch(SCHEMA).unwrap();
    let tx = connection.transaction().unwrap();
    let config_sha = actual_config(&tx).unwrap().0;
    // Actual legacy initialization, not a parallel mock schema or ledger. Use
    // direct existing Core API so production Store's later default-limit hook
    // cannot obscure these deliberately reduced storage/legacy limit controls.
    crate::sitemap_ledger::initialize(&tx, &config_sha, &ledger_limits, false).unwrap();
    tx.execute(
        "INSERT INTO discovery_frontier(kind,url,done) VALUES('sitemap',?1,0)",
        [TASK],
    )
    .unwrap();
    initialize(&tx, generation, &limits).unwrap();
    tx.commit().unwrap();
    Fixture {
        _temp: temp,
        store,
        connection,
        path,
        generation,
    }
}
fn normal() -> Fixture {
    fixture(Default::default(), Default::default())
}
fn gzip(body: &[u8], mtime: u32) -> Vec<u8> {
    let mut e = GzBuilder::new()
        .mtime(mtime)
        .write(Vec::new(), Compression::default());
    e.write_all(body).unwrap();
    e.finish().unwrap()
}
fn source(f: &Fixture, body: &[u8]) -> (SitemapDiscoveryBatch, SitemapSourceMeasurement) {
    let limits = SitemapParserLimits::default();
    let decoded = crate::decode_sitemap_entity_observed(body, &limits);
    let observation = decoded.observation;
    let decoded = decoded.result.unwrap();
    let count = crate::count_sitemap_xml(&decoded.xml, &SitemapCountLimits::default()).unwrap();
    let parsed = crate::observe_sitemap_xml(
        &decoded.xml,
        SitemapParserContext {
            requested_document_url: Some(TASK.into()),
            final_document_url: Some(TASK.into()),
            originating_occurrence_id: None,
        },
        &limits,
    )
    .unwrap();
    let failure = parsed.failure.as_ref().map(|f| f.code.clone());
    let revision = digest(
        serde_json::to_vec(&(
            "sitemap-decoded-xml-source-v1",
            TASK,
            &decoded.decoded_xml_sha256,
        ))
        .unwrap(),
    );
    let transport = SitemapTransportFacts {
        request_started: true,
        requested_url: Some(TASK.into()),
        final_url: Some(TASK.into()),
        initial_status: Some(200),
        final_status: Some(200),
        blocked_by_robots: false,
        body_complete: true,
        post_http_decoding_bytes: body.len() as u64,
        post_http_decoding_sha256: Some(digest(body)),
        retained_post_http_prefix_sha256: Some(digest(body)),
        failure_code: failure.clone(),
    };
    let batch = crate::sitemap_discovery::xml_batch(
        TASK,
        f.generation,
        &revision,
        transport,
        Some(&decoded),
        Some(parsed),
        failure,
    )
    .unwrap();
    let (config_sha, cfg) = actual_config(&f.connection).unwrap();
    let c = count.counts;
    let record = SitemapSourceMeasurement::new(
        SitemapSourceMeasurementPayload {
            schema_version: 1,
            engine: SITEMAP_SOURCE_MEASUREMENT_ENGINE.into(),
            binding: SitemapSourceMeasurementBinding {
                crawl_config_sha256: config_sha,
                run_generation: f.generation,
                source_kind: SitemapSourceKind::SitemapXml,
                requested_task_url: TASK.into(),
                expected_source_observation_id: batch.document.observation_id.clone(),
                source_revision: revision,
            },
            transport: SitemapSourceTransportMeasurement {
                http_decode_profile: SITEMAP_SOURCE_HTTP_PROFILE.into(),
                request_started: true,
                requested_url: Some(TASK.into()),
                final_url: Some(TASK.into()),
                initial_status: Some(200),
                final_status: Some(200),
                blocked_by_robots: false,
                body_complete: true,
                post_http_received_bytes: SitemapMeasuredValue::exact(body.len() as u64),
                retained_post_http_prefix_bytes: body.len() as u64,
                retained_post_http_prefix_sha256: Some(digest(body)),
                complete_post_http_entity_sha256: Some(digest(body)),
            },
            decode: SitemapSourceDecodeMeasurement {
                encoding: observation.encoding,
                decoded_xml_bytes: SitemapMeasuredValue::exact(decoded.xml.len() as u64),
                decoded_prefix_sha256: observation.decoded_prefix_sha256,
                decoded_complete_sha256: observation.decoded_complete_sha256,
                integrity: SitemapSourceDecodeIntegrity::Complete,
                stop_code: None,
            },
            scan: SitemapSourceScanMeasurement {
                xml_profile: count.syntax_profile,
                loc_lexical_profile: count.loc_lexical_profile,
                document_kind: count.kind,
                namespace_profile: count.namespace_profile,
                status: SitemapSourceScanStatus::CompleteProfile,
                stop_code: None,
                events_observed: SitemapMeasuredValue::exact(count.events_observed as u64),
                counts: SitemapSourceEntryMeasurements {
                    immediate_entry_starts: SitemapMeasuredValue::exact(c.immediate_entry_starts),
                    completed_immediate_entries: SitemapMeasuredValue::exact(
                        c.completed_immediate_entries,
                    ),
                    actual_document_namespace_loc_occurrences: SitemapMeasuredValue::exact(
                        c.actual_document_namespace_loc_occurrences,
                    ),
                    protocol_eligible_completed_entries: SitemapMeasuredValue::exact(
                        c.protocol_eligible_completed_entries,
                    ),
                    missing_loc_entries: c.missing_location_entries,
                    multiple_loc_entries: c.multiple_location_entries,
                    invalid_single_loc_entries: c.invalid_single_location_entries,
                },
            },
            membership_retention: if batch.document.retained_rows_complete {
                SitemapMembershipRetention::Complete
            } else {
                SitemapMembershipRetention::Incomplete
            },
            bounds: SitemapSourceCaptureBounds {
                max_retained_post_http_bytes: cfg.max_body_bytes.min(limits.max_entity_bytes)
                    as u64,
                max_retained_xml_bytes: limits.max_xml_bytes as u64,
                max_xml_events: limits.max_xml_events as u64,
                max_xml_depth: limits.max_xml_depth as u64,
                max_field_utf8_bytes: limits.max_field_utf8_bytes as u64,
                max_retained_entries: limits.max_entries as u64,
                max_retained_location_occurrences: limits.max_location_occurrences as u64,
                max_retained_text_utf8_bytes: limits.max_retained_text_utf8_bytes as u64,
            },
        },
        &Default::default(),
    )
    .unwrap();
    (batch, record)
}
fn persist(
    f: &mut Fixture,
    doc: &SitemapSourceDocument,
    measurement: &SitemapSourceMeasurement,
) -> SourceMeasurementReceipt {
    let tx = f.connection.transaction().unwrap();
    let receipt = record_a(&tx, doc, measurement).unwrap();
    tx.commit().unwrap();
    receipt.after_a_commit(&f.connection).unwrap()
}
fn commit(
    f: &mut Fixture,
    receipt: &SourceMeasurementReceipt,
    batch: SitemapDiscoveryBatch,
) -> Result<SitemapSourceStoreSummary> {
    let tx = f.connection.transaction()?;
    let mut guard = verify_b_before_admission(&tx, receipt, &batch.document)?;
    tx.execute(
        "INSERT INTO metadata(key,value) VALUES('source_store_b_control','staged')",
        [],
    )?;
    crate::sitemap_ledger::append_source(&tx, guard.take_pending()?, batch)?;
    let summary = link_b_after_append(&tx, guard)?;
    tx.commit()?;
    Ok(summary)
}
fn snapshot(f: &Fixture) -> SitemapSourceStoreSnapshot {
    let tx = f.connection.unchecked_transaction().unwrap();
    let out = load(&tx).unwrap().unwrap();
    tx.commit().unwrap();
    // Independent physical serde output witnesses the COMPLETE portable envelope,
    // including array delimiters/summary fixed point, rather than copied counters.
    assert_eq!(
        serde_json::to_vec(&out).unwrap().len() as u64,
        out.summary.logical_json_bytes
    );
    assert_eq!(
        serde_json::to_vec(&out.summary).unwrap().len() as u64,
        out.summary.metadata_json_bytes
    );
    assert_eq!(
        out.summary.logical_json_bytes + out.summary.reserved_json_bytes,
        out.summary.charged_json_bytes
    );
    out
}
fn done(f: &Fixture) -> i64 {
    f.connection
        .query_row(
            "SELECT done FROM discovery_frontier WHERE kind='sitemap' AND url=?1",
            [TASK],
            |r| r.get(0),
        )
        .unwrap()
}

#[test]
fn actual_a_survives_real_legacy_row_budget_rollback_and_cold_readonly() {
    let mut f = fixture(
        Default::default(),
        SitemapLedgerLimits {
            max_rows: 1,
            ..Default::default()
        },
    );
    let xml=b"<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>https://source-store.example/page</loc></url></urlset>";
    let (batch, measurement) = source(&f, xml);
    let doc = batch.document.clone();
    let legacy_before = crate::sitemap_ledger::load_catalog(&f.connection).unwrap();
    let receipt = persist(&mut f, &doc, &measurement);
    let a = snapshot(&f);
    assert_eq!(a.records.len(), 1);
    assert!(a.commits.is_empty());
    assert_eq!(done(&f), 0);
    let error = commit(&mut f, &receipt, batch).unwrap_err();
    assert!(
        error.to_string().contains("Sitemap row budget exceeded"),
        "{error:#}"
    );
    assert_eq!(snapshot(&f), a);
    assert_eq!(done(&f), 0);
    assert_eq!(
        crate::sitemap_ledger::load_catalog(&f.connection).unwrap(),
        legacy_before
    );
    assert!(
        bounded_metadata(&f.connection, "source_store_b_control", 64)
            .unwrap()
            .is_none()
    );
    let ro =
        Connection::open_with_flags(&f.path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let read = ro.unchecked_transaction().unwrap();
    assert_eq!(load(&read).unwrap(), Some(a));
    read.commit().unwrap();
}

#[test]
fn actual_success_links_exact_document_and_late_link_fault_rolls_back_all_b() {
    for fault in [true, false] {
        let mut f = normal();
        let (batch, m) = source(&f, EMPTY);
        let receipt = persist(&mut f, &batch.document, &m);
        let before = snapshot(&f);
        if fault {
            f.connection.execute_batch("CREATE TEMP TRIGGER fail_link BEFORE INSERT ON sitemap_source_measurement_commits BEGIN SELECT RAISE(ABORT,'late link fault'); END;").unwrap();
        }
        let result = commit(&mut f, &receipt, batch);
        if fault {
            let error = result.unwrap_err();
            assert!(error.to_string().contains("late link fault"), "{error:#}");
            assert_eq!(snapshot(&f), before);
            assert_eq!(done(&f), 0);
            assert!(
                bounded_metadata(&f.connection, "source_store_b_control", 64)
                    .unwrap()
                    .is_none()
            );
        } else {
            result.unwrap();
            let after = snapshot(&f);
            assert_eq!(done(&f), 1);
            assert_eq!(after.commits.len(), 1);
            assert_eq!(after.commits[0].measurement_id, m.measurement_id);
            assert!(after.summary.charged_json_bytes <= before.summary.charged_json_bytes);
            assert_eq!(after.records, before.records);
        }
    }
}

#[test]
fn actual_wal_a_only_revision_stales_context_without_changing_legacy_snapshot() {
    let mut f = normal();
    let reader =
        Connection::open_with_flags(&f.path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let read = reader.unchecked_transaction().unwrap();
    let old = load(&read).unwrap().unwrap();
    let old_hash = snapshot_context(Some(&old)).unwrap();
    let legacy = crate::sitemap_ledger::load_catalog(&read).unwrap();
    let (batch, m) = source(&f, EMPTY);
    persist(&mut f, &batch.document, &m);
    assert_eq!(load(&read).unwrap(), Some(old));
    assert_eq!(crate::sitemap_ledger::load_catalog(&read).unwrap(), legacy);
    read.commit().unwrap();
    let next = reader.unchecked_transaction().unwrap();
    let new = load(&next).unwrap().unwrap();
    assert_ne!(snapshot_context(Some(&new)).unwrap(), old_hash);
    assert_eq!(crate::sitemap_ledger::load_catalog(&next).unwrap(), legacy);
    next.commit().unwrap();
}

#[test]
fn actual_dedupe_requires_fresh_pending_and_a_commit_confirmation() {
    let mut f = normal();
    let (batch, m) = source(&f, EMPTY);
    let receipt = persist(&mut f, &batch.document, &m);
    let before = snapshot(&f);
    let tx = f.connection.transaction().unwrap();
    let duplicate = record_a(&tx, &batch.document, &m).unwrap();
    assert!(verify_b_before_admission(&tx, &duplicate, &batch.document).is_err());
    tx.commit().unwrap();
    let duplicate = duplicate.after_a_commit(&f.connection).unwrap();
    assert_eq!(snapshot(&f), before);
    commit(&mut f, &receipt, batch.clone()).unwrap();
    let tx = f.connection.transaction().unwrap();
    assert!(record_a(&tx, &batch.document, &m).is_err());
    assert!(verify_b_before_admission(&tx, &duplicate, &batch.document).is_err());
    tx.rollback().unwrap();
}

#[test]
fn actual_distinct_gzip_captures_share_legacy_id_but_only_exact_a_gets_association() {
    let mut f = normal();
    let (first, m1) = source(&f, &gzip(EMPTY, 1));
    let (second, m2) = source(&f, &gzip(EMPTY, 2));
    assert_eq!(
        first.document.observation_id,
        second.document.observation_id
    );
    assert_ne!(m1.measurement_id, m2.measurement_id);
    let r1 = persist(&mut f, &first.document, &m1);
    let r2 = persist(&mut f, &second.document, &m2);
    {
        let tx = f.connection.transaction().unwrap();
        assert!(verify_b_before_admission(&tx, &r1, &second.document).is_err());
        tx.rollback().unwrap();
    }
    commit(&mut f, &r2, second).unwrap();
    let s = snapshot(&f);
    assert_eq!(s.records.len(), 2);
    assert_eq!(s.commits.len(), 1);
    assert_eq!(s.commits[0].measurement_id, m2.measurement_id);
    assert!(
        s.records
            .iter()
            .any(|r| r.measurement.measurement_id == m1.measurement_id)
    );
}

#[test]
fn actual_config_resume_generation_and_payload_bounds_cannot_relabel_capture() {
    let mut f = normal();
    let (batch, m) = source(&f, EMPTY);
    let receipt = persist(&mut f, &batch.document, &m);
    let before = snapshot(&f);
    let mut bad = m.payload.clone();
    bad.bounds.max_xml_events -= 1;
    let bad = SitemapSourceMeasurement::new(bad, &Default::default()).unwrap();
    {
        let tx = f.connection.transaction().unwrap();
        assert!(record_a(&tx, &batch.document, &bad).is_err());
        tx.rollback().unwrap();
    }
    let mut cfg = actual_config(&f.connection).unwrap().1;
    cfg.delay_ms += 1;
    f.store.prepare(&cfg).unwrap();
    {
        let tx = f.connection.transaction().unwrap();
        assert!(record_a(&tx, &batch.document, &m).is_err());
        assert!(verify_b_before_admission(&tx, &receipt, &batch.document).is_err());
        tx.rollback().unwrap();
    }
    f.store.begin_run("resumed-generation").unwrap();
    {
        let tx = f.connection.transaction().unwrap();
        assert!(record_a(&tx, &batch.document, &m).is_err());
        tx.rollback().unwrap();
    }
    assert_eq!(snapshot(&f), before);
    assert_eq!(done(&f), 0);
}

#[test]
fn actual_record_and_total_exact_bytes_plus_one_omit_durably_without_replacement() {
    let f = normal();
    let (batch, m) = source(&f, EMPTY);
    let record = SitemapSourceStoredMeasurement {
        schema_version: 1,
        engine: SITEMAP_SOURCE_STORE_ENGINE.into(),
        sequence: 1,
        initial_catalog_config_sha256: actual_config(&f.connection).unwrap().0,
        staged_document_sha256: stage_sha(&batch.document).unwrap(),
        measurement: m.clone(),
    };
    let physical = serde_json::to_vec(&record).unwrap().len();
    let mut projected = snapshot(&f);
    projected.records.push(record);
    refresh(&mut projected).unwrap();
    let charged = projected.summary.charged_json_bytes as usize;
    for (record_cap, total_cap, persisted) in [
        (physical, MAX_SOURCE_STORE_BYTES, true),
        (physical - 1, MAX_SOURCE_STORE_BYTES, false),
        (MAX_SOURCE_RECORD_BYTES, charged, true),
        (MAX_SOURCE_RECORD_BYTES, charged - 1, false),
    ] {
        let mut f = fixture(
            SitemapSourceStoreLimits {
                max_record_json_bytes: record_cap,
                max_catalog_json_bytes: total_cap,
                ..Default::default()
            },
            Default::default(),
        );
        let (batch, m) = source(&f, EMPTY);
        let r = persist(&mut f, &batch.document, &m);
        assert_eq!(r.is_persisted(), persisted);
        let s = snapshot(&f);
        if persisted {
            assert_eq!(s.records.len(), 1);
            assert_eq!(s.summary.record_json_bytes, physical as u64);
            commit(&mut f, &r, batch).unwrap();
        } else {
            assert!(s.records.is_empty());
            assert_eq!(
                s.summary.omission,
                Some(SitemapSourceStoreOmission::StorageBudgetExceeded)
            );
            assert_eq!(s.summary.omission_revision, Some(1));
            let again = persist(&mut f, &batch.document, &m);
            assert!(!again.is_persisted());
            assert_eq!(snapshot(&f), s);
            commit(&mut f, &r, batch).unwrap();
            assert!(snapshot(&f).commits.is_empty());
        }
    }
    let escaped = "𝄞\"\\\n";
    let actual = serde_json::to_vec(&escaped).unwrap();
    assert_eq!(
        limited_json(&escaped, actual.len())
            .unwrap()
            .unwrap()
            .as_bytes(),
        actual
    );
    assert!(limited_json(&escaped, actual.len() - 1).unwrap().is_none());
}

#[test]
fn actual_capture_unknown_marker_and_a_sql_fault_do_not_invent_measurement_or_done() {
    let mut f = normal();
    let (batch, m) = source(&f, EMPTY);
    let config = m.payload.binding.crawl_config_sha256.clone();
    let tx = f.connection.transaction().unwrap();
    let r = record_unavailable_a(&tx, &batch.document, &config).unwrap();
    tx.commit().unwrap();
    let r = r.after_a_commit(&f.connection).unwrap();
    assert_eq!(
        r.omission_reason(),
        Some(SitemapSourceStoreOmission::CaptureUnavailable)
    );
    let s = snapshot(&f);
    assert_eq!(s.summary.record_count, 0);
    assert_eq!(done(&f), 0);
    commit(&mut f, &r, batch).unwrap();
    assert!(snapshot(&f).commits.is_empty());
    let mut f = normal();
    let (batch, m) = source(&f, EMPTY);
    let before = snapshot(&f);
    f.connection.execute_batch("CREATE TEMP TRIGGER fail_a BEFORE UPDATE ON metadata WHEN NEW.key='sitemap_source_store' BEGIN SELECT RAISE(ABORT,'A metadata fault'); END;").unwrap();
    {
        let tx = f.connection.transaction().unwrap();
        assert!(record_a(&tx, &batch.document, &m).is_err());
        tx.rollback().unwrap();
    }
    assert_eq!(snapshot(&f), before);
    assert_eq!(done(&f), 0);
}

#[test]
fn actual_load_rejects_sql_envelope_summary_and_portable_catalog_tampering() {
    let mut f = normal();
    let (batch, m) = source(&f, EMPTY);
    let r = persist(&mut f, &batch.document, &m);
    commit(&mut f, &r, batch).unwrap();
    let before = snapshot(&f);
    for sql in [
        "UPDATE sitemap_source_measurements SET data_sha256=printf('%064d',0)",
        "UPDATE sitemap_source_measurements SET task_url='https://wrong.example/'",
        "UPDATE sitemap_source_measurement_commits SET data_sha256=printf('%064d',0)",
        "UPDATE metadata SET value=json_set(value,'$.engine','unknown-future') WHERE key='sitemap_source_store'",
        "UPDATE sitemap_documents SET data_sha256=printf('%064d',0)",
    ] {
        let tx = f.connection.transaction().unwrap();
        tx.execute_batch(sql).unwrap();
        assert!(load(&tx).is_err(), "{sql}");
        tx.rollback().unwrap();
        assert_eq!(snapshot(&f), before);
    }
    let mut catalog = crate::sitemap_ledger::load_catalog(&f.connection)
        .unwrap()
        .unwrap();
    catalog.summary.config_sha256 = digest(b"unrelated initial config");
    assert!(validate_catalog_binding(&before, &catalog).is_err());
}

#[test]
fn actual_missing_historical_schema_and_done_sources_remain_unknown() {
    let mut f = normal();
    f.connection.execute_batch("DELETE FROM metadata WHERE key='sitemap_source_store'; DROP TABLE sitemap_source_measurement_commits; DROP TABLE sitemap_source_measurements;").unwrap();
    let reader =
        Connection::open_with_flags(&f.path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let schema_before: String = reader
        .query_row("SELECT group_concat(sql) FROM sqlite_master", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(load(&reader).unwrap().is_none());
    assert_eq!(
        reader
            .query_row::<String, _, _>("SELECT group_concat(sql) FROM sqlite_master", [], |r| r
                .get(0))
            .unwrap(),
        schema_before
    );
    f.connection.execute_batch(SCHEMA).unwrap();
    assert!(load(&f.connection).unwrap().is_none());
    f.connection
        .execute("UPDATE discovery_frontier SET done=1", [])
        .unwrap();
    let tx = f.connection.transaction().unwrap();
    let s = initialize(&tx, f.generation, &Default::default()).unwrap();
    assert!(s.legacy_history_unknown);
    assert_eq!(s.record_count, 0);
    tx.commit().unwrap();
    assert_ne!(
        snapshot_context(None).unwrap(),
        snapshot_context(Some(&snapshot(&f))).unwrap()
    );
    f.connection.execute_batch("DELETE FROM metadata WHERE key='sitemap_source_store'; DROP TABLE sitemap_source_measurement_commits;").unwrap();
    assert!(load(&f.connection).is_err());
}

#[test]
fn actual_selected_parent_keeps_original_captures_and_exact_blob_anchor() {
    let mut f = normal();
    let (batch, m) = source(&f, EMPTY);
    let r = persist(&mut f, &batch.document, &m);
    commit(&mut f, &r, batch).unwrap();
    let parent = snapshot(&f);
    let report = CrawlReport {
        config: actual_config(&f.connection).unwrap().1,
        pages: vec![PageRecord {
            url: "https://source-store.example/page".into(),
            ..Default::default()
        }],
        sitemap_catalog: crate::sitemap_ledger::load_catalog(&f.connection).unwrap(),
        sitemap_source_measurements: Some(parent.clone()),
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
            job_id: "source-parent".into(),
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
    let mut conn = Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    conn.execute_batch(SCHEMA).unwrap();
    // Root's integrated selected initializer may already copy the sidecar. Clear
    // only this test sidecar so this directed control calls actual inherit itself.
    conn.execute_batch("DELETE FROM sitemap_source_measurement_commits; DELETE FROM sitemap_source_measurements; DELETE FROM metadata WHERE key='sitemap_source_store';").unwrap();
    let tx = conn.transaction().unwrap();
    inherit(&tx, &parent, &digest(&bytes)).unwrap();
    tx.commit().unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    let inherited = load(&tx).unwrap().unwrap();
    tx.commit().unwrap();
    assert_eq!(inherited.records, parent.records);
    assert_eq!(inherited.commits, parent.commits);
    assert_eq!(
        inherited
            .summary
            .inherited
            .as_ref()
            .unwrap()
            .current_discovery_executed,
        false
    );
    let child_generation = child.begin_run("selected-refresh").unwrap();
    let mut attempted = report
        .sitemap_catalog
        .as_ref()
        .unwrap()
        .rows
        .as_ref()
        .unwrap()
        .iter()
        .find_map(|r| match r {
            SitemapLedgerRow::Document(d) => Some((**d).clone()),
            _ => None,
        })
        .unwrap();
    attempted.sequence = 0;
    attempted.generation = child_generation;
    attempted.observation_id = crate::sitemap_ledger::source_observation_id(
        attempted.kind,
        &attempted.task_url,
        attempted.generation,
        &attempted.source_revision,
    )
    .unwrap();
    {
        let tx = conn.transaction().unwrap();
        assert!(record_unavailable_a(&tx, &attempted, &actual_config(&tx).unwrap().0).is_err());
        tx.rollback().unwrap();
    }
    conn.execute(
        "UPDATE recrawl_base SET snapshot=CAST(snapshot || ' ' AS BLOB) WHERE id=1",
        [],
    )
    .unwrap();
    assert!(load(&conn).is_err());
}

#[test]
fn actual_crash_window_resume_preserves_old_unjoined_capture_and_rechecks_generation() {
    let mut f = normal();
    let (old_batch, old_measurement) = source(&f, EMPTY);
    let old_receipt = persist(&mut f, &old_batch.document, &old_measurement);
    let before = snapshot(&f);
    assert_eq!(done(&f), 0);
    // Cold read on a separate SQLite connection observes durable A without B.
    let cold =
        Connection::open_with_flags(&f.path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let tx = cold.unchecked_transaction().unwrap();
    assert_eq!(load(&tx).unwrap(), Some(before.clone()));
    tx.commit().unwrap();
    drop(cold);
    f.generation = f.store.begin_run("actual-source-resume").unwrap();
    {
        let tx = f.connection.transaction().unwrap();
        assert!(verify_b_before_admission(&tx, &old_receipt, &old_batch.document).is_err());
        tx.rollback().unwrap();
    }
    let (new_batch, new_measurement) = source(&f, EMPTY);
    assert_ne!(
        old_measurement.measurement_id,
        new_measurement.measurement_id
    );
    let new_receipt = persist(&mut f, &new_batch.document, &new_measurement);
    commit(&mut f, &new_receipt, new_batch).unwrap();
    let after = snapshot(&f);
    assert_eq!(after.records.len(), 2);
    assert_eq!(after.records[0], before.records[0]);
    assert_eq!(after.commits.len(), 1);
    assert_eq!(
        after.commits[0].measurement_id,
        new_measurement.measurement_id
    );
}

#[test]
fn actual_record_count_omission_and_marker_sql_failure_preserve_prior_bytes() {
    let mut f = fixture(
        SitemapSourceStoreLimits {
            max_records: 1,
            ..Default::default()
        },
        Default::default(),
    );
    let (first, m1) = source(&f, EMPTY);
    persist(&mut f, &first.document, &m1);
    let original = snapshot(&f);
    let (second, m2) = source(&f, &gzip(EMPTY, 7));
    let r = persist(&mut f, &second.document, &m2);
    assert!(!r.is_persisted());
    let omitted = snapshot(&f);
    assert_eq!(omitted.records, original.records);
    assert_eq!(omitted.summary.record_count, 1);
    assert_eq!(
        omitted.summary.omission,
        Some(SitemapSourceStoreOmission::StorageBudgetExceeded)
    );
    assert_eq!(done(&f), 0);
    let mut f = fixture(
        SitemapSourceStoreLimits {
            max_record_json_bytes: 1,
            ..Default::default()
        },
        Default::default(),
    );
    let (batch, m) = source(&f, EMPTY);
    let before = snapshot(&f);
    f.connection.execute_batch("CREATE TEMP TRIGGER fail_marker BEFORE UPDATE ON metadata WHEN NEW.key='sitemap_source_store' BEGIN SELECT RAISE(ABORT,'fixed marker fault'); END;").unwrap();
    {
        let tx = f.connection.transaction().unwrap();
        assert!(record_a(&tx, &batch.document, &m).is_err());
        tx.rollback().unwrap();
    }
    assert_eq!(snapshot(&f), before);
    assert_eq!(done(&f), 0);
}

#[test]
fn actual_b_needs_consumed_same_transaction_token_and_post_append_done() {
    let mut f = normal();
    let (batch, m) = source(&f, EMPTY);
    let receipt = persist(&mut f, &batch.document, &m);
    let before = snapshot(&f);
    {
        let tx = f.connection.transaction().unwrap();
        let guard = verify_b_before_admission(&tx, &receipt, &batch.document).unwrap();
        assert!(link_b_after_append(&tx, guard).is_err());
        tx.rollback().unwrap();
    }
    {
        let tx = f.connection.transaction().unwrap();
        let mut guard = verify_b_before_admission(&tx, &receipt, &batch.document).unwrap();
        drop(guard.take_pending().unwrap());
        assert!(guard.take_pending().is_err());
        assert!(link_b_after_append(&tx, guard).is_err());
        tx.rollback().unwrap();
    }
    assert_eq!(snapshot(&f), before);
    assert_eq!(done(&f), 0);
}

#[test]
fn actual_writable_schema_never_repairs_missing_initialized_empty_tables() {
    let f = normal();
    assert_eq!(snapshot(&f).summary.record_count, 0);
    f.connection
        .execute_batch("DROP TABLE sitemap_source_measurement_commits;")
        .unwrap();
    assert!(initialize_writable_schema(&f.connection).is_err());
    assert_eq!(tables_present(&f.connection).unwrap(), 1);
    f.connection
        .execute_batch("DROP TABLE sitemap_source_measurements;")
        .unwrap();
    assert!(initialize_writable_schema(&f.connection).is_err());
    assert_eq!(tables_present(&f.connection).unwrap(), 0);
    f.connection
        .execute("DELETE FROM metadata WHERE key=?1", [KEY])
        .unwrap();
    initialize_writable_schema(&f.connection).unwrap();
    assert_eq!(tables_present(&f.connection).unwrap(), 2);
    assert!(load(&f.connection).unwrap().is_none());
}

#[path = "sitemap_source_store/verified_read_tests.rs"]
mod verified_read_tests;
