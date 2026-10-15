//! AUTHORED / NOT COMPILED / NOT RUN while coordinated compute HOLD is active.
//! Real CrawlStore lifecycle/A/B, observed decoder/scanner and actual read TXs.
//! Transport fixtures are saved observations, not native HTTP acceptance.
use super::*;
use crate::sitemap_ledger::verified_read::verify_raw_with_control;
use crate::sitemap_source_store::verified_read::verify_source_with_control;
use crate::{
    CrawlConfig, CrawlRunState, CrawlStore, SitemapDiscoveryBatch, SitemapParserContext,
    SitemapParserLimits, SitemapReadControlError, SitemapSourceStoreOmission,
    SitemapTransportFacts, decode_sitemap_entity_observed, observe_sitemap_xml,
};
use flate2::{Compression, GzBuilder};
use rusqlite::OpenFlags;

const TASK: &str = "https://source-findings.example/sitemap.xml";
const EMPTY: &[u8] = b"<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'/>";
struct Fixture {
    _temp: tempfile::TempDir,
    path: std::path::PathBuf,
    store: CrawlStore,
    config: CrawlConfig,
    generation: u64,
}
fn task() -> crate::store::DiscoveryEntry {
    crate::store::DiscoveryEntry {
        kind: crate::store::DiscoveryKind::Sitemap,
        url: TASK.into(),
    }
}
fn fixture(initialize: bool) -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("source-findings.sqlite");
    let store = CrawlStore::open(&path).unwrap();
    let mut config = CrawlConfig {
        seed_urls: vec!["https://source-findings.example/".into()],
        sitemap_urls: vec![TASK.into()],
        obey_robots: false,
        ..Default::default()
    };
    config.prepare_input_lists().unwrap();
    store.prepare(&config).unwrap();
    let generation = store.begin_run("actual-source-findings-fixture").unwrap();
    if initialize {
        let connection = Connection::open(&path).unwrap();
        let sha = config_sha(&connection);
        store
            .initialize_sitemap_discovery(
                &[task()],
                generation,
                &crate::store::SitemapCatalogInit {
                    config_sha256: sha,
                    limits: Default::default(),
                    roots: vec![],
                    legacy_history_unknown: false,
                },
            )
            .unwrap();
    }
    Fixture {
        _temp: temp,
        path,
        store,
        config,
        generation,
    }
}
fn config_sha(connection: &Connection) -> String {
    let text: String = connection
        .query_row("SELECT value FROM metadata WHERE key='config'", [], |r| {
            r.get(0)
        })
        .unwrap();
    crate::sitemap_ledger::digest(text)
}
fn read<T>(
    f: &Fixture,
    control: &SitemapReadControl,
    callback: impl FnOnce(&PreparedSourceFindings<'_, '_>) -> T,
) -> T {
    let connection =
        Connection::open_with_flags(&f.path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let transaction = connection.unchecked_transaction().unwrap();
    let raw = verify_raw_with_control(&transaction, control).unwrap();
    let source = verify_source_with_control(&transaction, raw.as_ref(), control).unwrap();
    let run: String = transaction
        .query_row(
            "SELECT value FROM metadata WHERE key='run_state'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let generation = serde_json::from_str::<CrawlRunState>(&run)
        .unwrap()
        .generation;
    let sha = config_sha(&transaction);
    let prepared = prepare_source_findings(
        &transaction,
        raw.as_ref(),
        source.as_ref(),
        Some(generation),
        Some(&sha),
        control,
    )
    .unwrap();
    callback(&prepared)
}
fn candidates(f: &Fixture) -> (SitemapSourceFindingsScope, Vec<(Position, SourceEvidence)>) {
    read(f, &Default::default(), |prepared| {
        let mut rows = vec![];
        assert!(
            prepared
                .visit_after(None, |p, e| {
                    rows.push((p, e));
                    Ok(ControlFlow::Continue(()))
                })
                .unwrap()
        );
        (prepared.scope().clone(), rows)
    })
}
fn batch(f: &Fixture, body: &[u8]) -> SitemapDiscoveryBatch {
    let limits = SitemapParserLimits::default();
    let decoded = decode_sitemap_entity_observed(body, &limits)
        .result
        .unwrap();
    let parsed = observe_sitemap_xml(
        &decoded.xml,
        SitemapParserContext {
            requested_document_url: Some(TASK.into()),
            final_document_url: Some(TASK.into()),
            originating_occurrence_id: None,
        },
        &limits,
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
    let sha = crate::sitemap_ledger::digest(body);
    crate::sitemap_discovery::xml_batch(
        TASK,
        f.generation,
        &revision,
        SitemapTransportFacts {
            request_started: true,
            requested_url: Some(TASK.into()),
            final_url: Some(TASK.into()),
            initial_status: Some(200),
            final_status: Some(200),
            blocked_by_robots: false,
            body_complete: true,
            post_http_decoding_bytes: body.len() as u64,
            post_http_decoding_sha256: Some(sha.clone()),
            retained_post_http_prefix_sha256: Some(sha),
            failure_code: None,
        },
        Some(&decoded),
        Some(parsed),
        None,
    )
    .unwrap()
}
fn capture(
    f: &Fixture,
    value: &SitemapDiscoveryBatch,
    body: &[u8],
) -> crate::sitemap_source_store::SourceMeasurementReceipt {
    let connection = Connection::open(&f.path).unwrap();
    let measurement = crate::crawler::test_sitemap_measurement_from_stored_transport(
        &value.document,
        body,
        config_sha(&connection),
        f.config
            .max_body_bytes
            .min(SitemapParserLimits::default().max_entity_bytes),
    );
    f.store
        .record_sitemap_source_capture(&task(), &value.document, &measurement, f.generation)
        .unwrap()
}
fn gzip(mtime: u32) -> Vec<u8> {
    let mut writer = GzBuilder::new()
        .mtime(mtime)
        .write(Vec::new(), Compression::default());
    writer.write_all(EMPTY).unwrap();
    writer.finish().unwrap()
}

#[test]
fn source_findings_actual_a_only_then_exact_b_and_shared_source_id_do_not_attach_other_capture() {
    let f = fixture(true);
    let first_bytes = gzip(1);
    let second_bytes = gzip(2);
    let first = batch(&f, &first_bytes);
    let second = batch(&f, &second_bytes);
    assert_eq!(
        first.document.observation_id,
        second.document.observation_id
    );
    let first_receipt = capture(&f, &first, &first_bytes);
    let second_receipt = capture(&f, &second, &second_bytes);
    let (_, before) = candidates(&f);
    assert_eq!(before.len(), 2);
    assert!(before.iter().all(|(_, e)| matches!(
        e,
        SourceEvidence::Measured {
            association: None,
            committed_document: None,
            ..
        }
    )));
    f.store
        .complete_discovery_measured(&task(), &[], &[], &[], f.generation, first, &first_receipt)
        .unwrap();
    let (scope, after) = candidates(&f);
    assert!(scope.measurement_history_available);
    assert!(!scope.unknown_history);
    assert_eq!(after.len(), 2);
    let links: Vec<_> = after
        .iter()
        .filter_map(|(_, e)| match e {
            SourceEvidence::Measured {
                record,
                association: Some(link),
                committed_document: Some(document),
                ..
            } => {
                assert_eq!(link.measurement_id, record.measurement.measurement_id);
                assert_eq!(document.observation_id, link.document_id);
                Some(link.measurement_id.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        links,
        vec![first_receipt.measurement_id().unwrap().to_owned()]
    );
    assert_ne!(
        first_receipt.measurement_id(),
        second_receipt.measurement_id()
    );
}

#[test]
fn source_findings_legacy_document_is_unknown_and_not_a_fabricated_measurement_or_second_current_task()
 {
    let f = fixture(true);
    f.store
        .complete_discovery_observed(&task(), &[], &[], &[], f.generation, batch(&f, EMPTY))
        .unwrap();
    let (_, rows) = candidates(&f);
    assert_eq!(rows.len(), 1);
    assert!(matches!(
        &rows[0],
        (
            Position::LegacySourceDocument { .. },
            SourceEvidence::LegacyDocument { .. }
        )
    ));
    // Resume changes actual config bytes while preserving historical catalog
    // SHA. A legacy document lacks proof of this new current config binding.
    let mut updated = f.config.clone();
    updated.delay_ms += 1;
    f.store.prepare(&updated).unwrap();
    let (_, rows) = candidates(&f);
    assert_eq!(rows.len(), 2);
    assert!(matches!(&rows[1].1, SourceEvidence::UnmeasuredTask {task} if task.done));
}

#[test]
fn source_findings_old_generation_capture_does_not_hide_current_pending_task() {
    let f = fixture(true);
    let value = batch(&f, EMPTY);
    capture(&f, &value, EMPTY);
    let new_generation = f.store.begin_run("actual-next-generation").unwrap();
    assert!(new_generation > f.generation);
    let (_, rows) = candidates(&f);
    assert_eq!(rows.len(), 2);
    assert!(matches!(
        &rows[0].1,
        SourceEvidence::Measured {
            association: None,
            ..
        }
    ));
    assert!(matches!(&rows[1].1, SourceEvidence::UnmeasuredTask {task} if !task.done));
}

#[test]
fn source_findings_same_generation_old_config_capture_does_not_hide_current_pending_task() {
    let f = fixture(true);
    let value = batch(&f, EMPTY);
    capture(&f, &value, EMPTY);
    let (before, rows) = candidates(&f);
    assert_eq!(rows.len(), 1);
    let mut updated = f.config.clone();
    updated.delay_ms += 1;
    f.store.prepare(&updated).unwrap();
    let (after, rows) = candidates(&f);
    assert_eq!(rows.len(), 2);
    assert_ne!(
        before.current_task_inventory_sha256,
        after.current_task_inventory_sha256
    );
    assert!(matches!(
        &rows[0].1,
        SourceEvidence::Measured {
            association: None,
            ..
        }
    ));
    assert!(matches!(&rows[1].1, SourceEvidence::UnmeasuredTask {task} if !task.done));
}

#[test]
fn source_findings_raw_none_validates_all_tasks_and_hashes_pending_done_robots_and_long_identity() {
    let f = fixture(false);
    let connection = Connection::open(&f.path).unwrap();
    let long = format!("https://source-findings.example/{}", "a".repeat(9_000));
    for (kind, url, done) in [
        ("sitemap", long.as_str(), 0),
        ("sitemap", TASK, 1),
        ("robots", "https://source-findings.example/", 0),
    ] {
        connection
            .execute(
                "INSERT INTO discovery_frontier(kind,url,done) VALUES(?1,?2,?3)",
                params![kind, url, done],
            )
            .unwrap();
    }
    let (scope, rows) = candidates(&f);
    assert_eq!(scope.current_frontier_task_count, Some(3));
    assert!(!scope.measurement_history_available && scope.unknown_history);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|(_, e)| e.original_task_url() == long));
    connection
        .execute(
            "UPDATE discovery_frontier SET done=1 WHERE kind='robots'",
            [],
        )
        .unwrap();
    let (changed, unchanged_rows) = candidates(&f);
    assert_ne!(
        scope.current_task_inventory_sha256,
        changed.current_task_inventory_sha256
    );
    assert_eq!(rows, unchanged_rows);
    // Explicit corrupt-restoration fixture: invalid LATE task must fail before
    // a token/callback, even when raw history is absent and early rows are valid.
    connection
        .execute(
            "INSERT INTO discovery_frontier(kind,url,done) VALUES('sitemap','zz-not-a-url',0)",
            [],
        )
        .unwrap();
    let tx = connection.unchecked_transaction().unwrap();
    let sha = config_sha(&tx);
    let error = prepare_source_findings(
        &tx,
        None,
        None,
        Some(f.generation),
        Some(&sha),
        &Default::default(),
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("Unsafe coverage task identity"));
}

#[test]
fn source_findings_binary_task_order_and_real_anchor_validation_precede_all_output() {
    let f = fixture(false);
    let connection = Connection::open(&f.path).unwrap();
    for suffix in ["a.xml", "A.xml", "b.xml"] {
        connection
            .execute(
                "INSERT INTO discovery_frontier(kind,url,done) VALUES('sitemap',?1,0)",
                [format!("https://source-findings.example/{suffix}")],
            )
            .unwrap();
    }
    read(&f, &Default::default(), |prepared| {
        let mut first = vec![];
        assert!(
            !prepared
                .visit_after(None, |p, e| {
                    first.push((p, e));
                    Ok(ControlFlow::Break(()))
                })
                .unwrap()
        );
        assert!(first[0].1.original_task_url().ends_with("/A.xml"));
        let mut rest = vec![];
        assert!(
            prepared
                .visit_after(Some(&first[0].0), |p, e| {
                    rest.push((p, e));
                    Ok(ControlFlow::Continue(()))
                })
                .unwrap()
        );
        assert_eq!(rest.len(), 2);
        for invalid in [
            Position::SourceMeasurement { sequence: 1 },
            Position::LegacySourceDocument { sequence: 1 },
            Position::UnmeasuredSourceTask { ordinal: 4 },
            Position::SavedPage { ordinal: 0 },
        ] {
            let mut callbacks = 0;
            let error = prepared
                .visit_after(Some(&invalid), |_, _| {
                    callbacks += 1;
                    Ok(ControlFlow::Continue(()))
                })
                .unwrap_err();
            assert_eq!(callbacks, 0);
            assert!(matches!(
                error.downcast_ref::<SitemapFindingsError>(),
                Some(SitemapFindingsError::InvalidContinuation)
            ));
        }
    });
}

#[test]
fn source_findings_actual_fixed_omission_marker_is_scope_only_not_a_synthetic_attempt() {
    let f = fixture(true);
    let value = batch(&f, EMPTY);
    let connection = Connection::open(&f.path).unwrap();
    let receipt = f
        .store
        .record_unavailable_sitemap_source_capture(
            &task(),
            &value.document,
            &config_sha(&connection),
            f.generation,
        )
        .unwrap();
    assert!(!receipt.is_persisted());
    let (scope, rows) = candidates(&f);
    assert_eq!(
        scope.omission,
        Some(SitemapSourceStoreOmission::CaptureUnavailable)
    );
    assert_eq!(rows.len(), 1);
    assert!(matches!(&rows[0].1, SourceEvidence::UnmeasuredTask { .. }));
}

#[test]
fn source_findings_known_empty_and_historical_absence_keep_distinct_scope() {
    let f = fixture(false);
    let (none, rows) = candidates(&f);
    assert!(rows.is_empty());
    assert!(!none.measurement_history_available && none.unknown_history);
    let initialized = fixture(true);
    let (some, rows) = candidates(&initialized);
    assert!(some.measurement_history_available && !some.unknown_history);
    assert_eq!(rows.len(), 1); // genuine pending task, not measured zero.
    let connection = Connection::open(&f.path).unwrap();
    connection
        .execute_batch("DROP TABLE discovery_frontier;")
        .unwrap();
    let (unavailable, rows) = candidates(&f);
    assert!(rows.is_empty());
    assert!(!unavailable.current_task_inventory_available);
    assert_eq!(unavailable.current_frontier_task_count, None);
    assert_eq!(unavailable.current_task_inventory_sha256, None);
}

#[test]
fn source_findings_actual_same_tx_wal_inventory_is_stable_and_later_read_observes_change() {
    let f = fixture(false);
    let writer = Connection::open(&f.path).unwrap();
    writer.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
    writer
        .execute(
            "INSERT INTO discovery_frontier(kind,url,done) VALUES('sitemap',?1,0)",
            [TASK],
        )
        .unwrap();
    read(&f, &Default::default(), |prepared| {
        let before = prepared.scope().clone();
        writer
            .execute("UPDATE discovery_frontier SET done=1 WHERE url=?1", [TASK])
            .unwrap();
        let raw = verify_raw_with_control(prepared.connection, &Default::default()).unwrap();
        let source =
            verify_source_with_control(prepared.connection, raw.as_ref(), &Default::default())
                .unwrap();
        let later_same_tx = prepare_source_findings(
            prepared.connection,
            raw.as_ref(),
            source.as_ref(),
            prepared.generation,
            prepared.config_sha.as_deref(),
            &Default::default(),
        )
        .unwrap();
        assert_eq!(later_same_tx.scope(), &before);
        let mut seen = vec![];
        assert!(
            prepared
                .visit_after(None, |_, e| {
                    seen.push(e);
                    Ok(ControlFlow::Continue(()))
                })
                .unwrap()
        );
        assert!(matches!(&seen[0], SourceEvidence::UnmeasuredTask {task} if !task.done));
    });
    let (new, rows) = candidates(&f);
    assert!(matches!(&rows[0].1, SourceEvidence::UnmeasuredTask {task} if task.done));
    assert!(new.current_task_inventory_sha256.is_some());
}

#[test]
fn source_findings_rejects_actual_context_and_connection_mismatch_without_callback() {
    let f = fixture(true);
    let connection = Connection::open(&f.path).unwrap();
    let tx = connection.unchecked_transaction().unwrap();
    let raw = verify_raw_with_control(&tx, &Default::default()).unwrap();
    let source = verify_source_with_control(&tx, raw.as_ref(), &Default::default()).unwrap();
    let sha = config_sha(&tx);
    assert!(
        prepare_source_findings(
            &tx,
            raw.as_ref(),
            source.as_ref(),
            Some(f.generation + 1),
            Some(&sha),
            &Default::default()
        )
        .is_err()
    );
    assert!(
        prepare_source_findings(
            &tx,
            raw.as_ref(),
            source.as_ref(),
            Some(f.generation),
            Some(&"0".repeat(64)),
            &Default::default()
        )
        .is_err()
    );
    let other = Connection::open(&f.path).unwrap();
    let other_tx = other.unchecked_transaction().unwrap();
    let error = prepare_source_findings(
        &other_tx,
        raw.as_ref(),
        source.as_ref(),
        Some(f.generation),
        Some(&sha),
        &Default::default(),
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("connection differs"));
}

#[test]
fn source_findings_cancellation_and_deadline_stops_are_typed_before_prepare_or_visit() {
    let f = fixture(true);
    read(&f, &Default::default(), |prepared| {
        prepared.control.cancel();
        let error = prepared
            .visit_after(None, |_, _| panic!("cancelled callback"))
            .unwrap_err();
        assert_eq!(
            error.downcast_ref::<SitemapReadControlError>(),
            Some(&SitemapReadControlError::Cancelled)
        );
    });
    let connection = Connection::open(&f.path).unwrap();
    let tx = connection.unchecked_transaction().unwrap();
    let control = SitemapReadControl::default();
    control.expire();
    let error = prepare_source_findings(&tx, None, None, None, None, &control)
        .err()
        .unwrap();
    assert_eq!(
        error.downcast_ref::<SitemapReadControlError>(),
        Some(&SitemapReadControlError::DeadlineExceeded)
    );
}
