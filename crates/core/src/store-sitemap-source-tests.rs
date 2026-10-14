//! AUTHORED / NOT COMPILED / NOT RUN under coordinated compute hold.
//! Actual Store A/B, SQLite, observed decoder/scanner and immutable reports.
//! Transport rows below are explicitly stored-observation fixtures, not HTTP proof.
use super::*;
use flate2::{Compression, write::GzEncoder};
use std::io::Write;

const TASK: &str = "https://source-store.example/sitemap.xml";
const PAGE: &str = "https://source-store.example/page";
fn task() -> DiscoveryEntry {
    DiscoveryEntry {
        kind: DiscoveryKind::Sitemap,
        url: TASK.into(),
    }
}
fn body() -> Vec<u8> {
    format!("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>{PAGE}</loc></url></urlset>").into_bytes()
}
fn fixture(
    policy: bool,
    limits: crate::SitemapLedgerLimits,
) -> (tempfile::TempDir, CrawlStore, u64) {
    fixture_with_measurement_limits(policy, limits, Default::default())
}
fn fixture_with_measurement_limits(
    policy: bool,
    limits: crate::SitemapLedgerLimits,
    measurement_limits: crate::SitemapSourceStoreLimits,
) -> (tempfile::TempDir, CrawlStore, u64) {
    let temp = tempfile::tempdir().unwrap();
    let store = CrawlStore::open(temp.path().join("source.sqlite")).unwrap();
    let mut config = CrawlConfig {
        seed_urls: vec!["https://source-store.example/".into()],
        sitemap_urls: vec![TASK.into()],
        explicit_sitemap_inputs: Some(vec![TASK.into()]),
        obey_robots: false,
        ..Default::default()
    };
    if policy {
        let mut p = crate::CrawlPolicyConfig::default();
        p.scope.roots.push(crate::CrawlScopeRoot {
            origin: "https://source-store.example".into(),
            subdomains: false,
            folders: vec!["/".into()],
            exact_paths: vec![],
        });
        config.crawl_policy = Some(p);
    }
    config.prepare_input_lists().unwrap();
    store.prepare(&config).unwrap();
    let generation = store
        .begin_run("stored-source-observation-fixture")
        .unwrap();
    if let Some(policy) = &config.crawl_policy {
        let policy = crate::CompiledCrawlPolicy::new(policy).unwrap();
        let revision = format!(
            "initial:{}:explicit:sitemap_document",
            policy.policy_sha256()
        );
        let mut rows = vec![crate::policy_ledger::decide(
            &policy,
            &revision,
            0,
            crate::CandidateSourceKind::SitemapDocument,
            TASK.into(),
            None,
            None,
            crate::PolicyInputBasis::DeclaredInput,
            crate::PolicyTaskRole::SitemapDocument,
        )];
        store
            .initialize_policy(&policy, &mut rows, &Default::default(), generation)
            .unwrap();
    }
    let sha = sha256(
        &metadata_text(&store.connection.lock().unwrap(), "config")
            .unwrap()
            .unwrap(),
    );
    store
        .initialize_sitemap_discovery_with_measurement_limits(
            &[task()],
            generation,
            &SitemapCatalogInit {
                config_sha256: sha,
                limits,
                roots: vec![],
                legacy_history_unknown: false,
            },
            &measurement_limits,
            |_, _| Ok(()),
        )
        .unwrap();
    (temp, store, generation)
}
fn batch(generation: u64, entity: &[u8], ordinary: bool) -> crate::SitemapDiscoveryBatch {
    let limits = crate::SitemapParserLimits::default();
    let decoded = crate::decode_sitemap_entity_observed(entity, &limits)
        .result
        .unwrap();
    let parser = crate::observe_sitemap_xml(
        &decoded.xml,
        crate::SitemapParserContext {
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
    let hash = crate::sitemap_ledger::digest(entity);
    let mut value = crate::sitemap_discovery::xml_batch(
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
    .unwrap();
    if ordinary {
        for loc in &mut value.locations {
            if loc.admission.decision == crate::SitemapAdmissionDecision::Unbound {
                loc.admission.decision = crate::SitemapAdmissionDecision::Scheduled;
                loc.admission.admitted_fetch_url = loc.observed.normalized_fetch_url.clone();
            }
        }
    }
    value
}
fn measurement(
    store: &CrawlStore,
    value: &crate::SitemapDiscoveryBatch,
    entity: &[u8],
) -> crate::SitemapSourceMeasurement {
    let sha = sha256(
        &metadata_text(&store.connection.lock().unwrap(), "config")
            .unwrap()
            .unwrap(),
    );
    let cap = store
        .config()
        .unwrap()
        .unwrap()
        .max_body_bytes
        .min(crate::SitemapParserLimits::default().max_entity_bytes);
    crate::crawler::test_sitemap_measurement_from_stored_transport(
        &value.document,
        entity,
        sha,
        cap,
    )
}
fn a(
    store: &CrawlStore,
    generation: u64,
    value: &crate::SitemapDiscoveryBatch,
    entity: &[u8],
) -> crate::sitemap_source_store::SourceMeasurementReceipt {
    store
        .record_sitemap_source_capture(
            &task(),
            &value.document,
            &measurement(store, value, entity),
            generation,
        )
        .unwrap()
}
fn queued() -> QueueEntry {
    QueueEntry {
        url: PAGE.into(),
        depth: 0,
        discovered_from: Some(TASK.into()),
        admission: None,
    }
}
fn b(
    store: &CrawlStore,
    generation: u64,
    value: crate::SitemapDiscoveryBatch,
    receipt: &crate::sitemap_source_store::SourceMeasurementReceipt,
) -> anyhow::Result<()> {
    store.complete_discovery_measured(
        &task(),
        &[queued()],
        &[],
        &["actual source warning".into()],
        generation,
        value,
        receipt,
    )
}
fn done(store: &CrawlStore) -> i64 {
    store
        .connection
        .lock()
        .unwrap()
        .query_row(
            "SELECT done FROM discovery_frontier WHERE kind='sitemap' AND url=?1",
            [TASK],
            |r| r.get(0),
        )
        .unwrap()
}
fn state(store: &CrawlStore) -> Vec<(String, Vec<String>)> {
    let c = store.connection.lock().unwrap();
    [
        "metadata",
        "frontier",
        "discovery_frontier",
        "pages",
        "policy_occurrences",
        "policy_sources",
        "policy_admissions",
        "sitemap_documents",
        "sitemap_roots",
        "sitemap_entries",
        "sitemap_locations",
        "sitemap_source_measurements",
        "sitemap_source_measurement_commits",
    ]
    .into_iter()
    .map(|table| {
        let mut q = c
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap();
        let columns = q.column_count();
        let rows = q
            .query_map([], |row| {
                Ok((0..columns)
                    .map(|i| format!("{:?}", row.get_ref(i).unwrap()))
                    .collect::<Vec<_>>()
                    .join("|"))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        (table.into(), rows)
    })
    .collect()
}
fn snapshot(store: &CrawlStore) -> crate::SitemapSourceStoreSnapshot {
    store
        .sitemap_read_snapshot()
        .unwrap()
        .sitemap_source_measurements
        .unwrap()
}
fn report(store: &CrawlStore) -> CrawlReport {
    let snapshot = store.sitemap_read_snapshot().unwrap();
    let pages = store.pages().unwrap();
    CrawlReport {
        config: store.config().unwrap().unwrap(),
        summary: crate::CrawlSummary {
            completed: pages.len(),
            pending: store.pending_count().unwrap(),
            ..Default::default()
        },
        pages,
        issues: vec![],
        warnings: vec![],
        operation: store.recrawl_operation().unwrap(),
        policy_ledger: store.policy_ledger_summary().unwrap(),
        resource_ledger: store.resource_ledger_summary().unwrap(),
        sitemap_catalog: snapshot.catalog,
        sitemap_coverage_inventory: snapshot.coverage_inventory,
        sitemap_source_measurements: snapshot.sitemap_source_measurements,
    }
}

#[test]
fn real_store_a_survives_ordinary_membership_row_budget_rollback_and_readonly_reopen() {
    let (temp, store, generation) = fixture(
        false,
        crate::SitemapLedgerLimits {
            max_rows: 1,
            ..Default::default()
        },
    );
    let value = batch(generation, &body(), true);
    let receipt = a(&store, generation, &value, &body());
    assert!(receipt.is_persisted());
    let before = state(&store);
    assert!(b(&store, generation, value, &receipt).is_err());
    assert_eq!(
        state(&store),
        before,
        "B must roll back all frontier/warnings/catalog/link writes, not A"
    );
    assert_eq!(done(&store), 0);
    let saved = snapshot(&store);
    assert_eq!(saved.records.len(), 1);
    assert!(saved.commits.is_empty());
    assert_eq!(
        saved.records[0].measurement.payload.membership_retention,
        crate::SitemapMembershipRetention::Complete,
        "private parser retention remains independent of durable membership rollback"
    );
    drop(store);
    let readonly = CrawlStore::open_read_only(temp.path().join("source.sqlite")).unwrap();
    assert_eq!(snapshot(&readonly), saved);
    assert_eq!(done(&readonly), 0);
}

#[test]
fn real_store_policy_late_link_failure_rolls_back_grants_and_caller_rows_then_commits_exact_link() {
    let (_temp, store, generation) = fixture(true, Default::default());
    let value = batch(generation, &body(), false);
    let receipt = a(&store, generation, &value, &body());
    let revision = value.document.source_revision.clone();
    let policy = crate::CompiledCrawlPolicy::new(
        store
            .config()
            .unwrap()
            .unwrap()
            .crawl_policy
            .as_ref()
            .unwrap(),
    )
    .unwrap();
    let mut rows = vec![crate::policy_ledger::decide(
        &policy,
        &revision,
        1,
        crate::CandidateSourceKind::SitemapLocation,
        PAGE.into(),
        None,
        Some(TASK.into()),
        crate::PolicyInputBasis::ParserObservedXml,
        crate::PolicyTaskRole::PageDocument,
    )];
    let before_rows = rows.clone();
    store.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_measured_link BEFORE INSERT ON sitemap_source_measurement_commits BEGIN SELECT RAISE(ABORT,'injected actual B link failure'); END;").unwrap();
    let before = state(&store);
    let result = store.complete_discovery_with_policy_measured(
        &task(),
        &mut rows,
        (&revision, true),
        &["must rollback".into()],
        generation,
        &Default::default(),
        value.clone(),
        &receipt,
        |_, _, _| Ok(()),
    );
    assert!(result.is_err());
    assert_eq!(rows, before_rows);
    assert_eq!(state(&store), before);
    assert_eq!(done(&store), 0);
    assert_eq!(snapshot(&store).records.len(), 1);
    assert!(snapshot(&store).commits.is_empty());
    store
        .connection
        .lock()
        .unwrap()
        .execute_batch("DROP TRIGGER fail_measured_link;")
        .unwrap();
    let (pages, children) = store
        .complete_discovery_with_policy_measured(
            &task(),
            &mut rows,
            (&revision, true),
            &[],
            generation,
            &Default::default(),
            value,
            &receipt,
            |_, _, _| Ok(()),
        )
        .unwrap();
    assert_eq!(pages.len(), 1);
    assert!(children.is_empty());
    assert_eq!(done(&store), 1);
    assert_eq!(snapshot(&store).commits.len(), 1);
    assert_eq!(rows[0].admission_reason, "scheduled");
    assert!(
        b(
            &store,
            generation,
            batch(generation, &body(), true),
            &receipt
        )
        .is_err(),
        "done task cannot dedupe around fresh B guard"
    );
}

#[test]
fn real_store_fresh_config_generation_guards_preserve_old_a_and_require_new_capture() {
    let (_temp, store, first) = fixture(false, Default::default());
    let value = batch(first, &body(), true);
    let receipt = a(&store, first, &value, &body());
    let captured = snapshot(&store).records.clone();
    let mut cfg = store.config().unwrap().unwrap();
    cfg.delay_ms += 1;
    store.prepare(&cfg).unwrap();
    let before = state(&store);
    assert!(
        b(&store, first, value.clone(), &receipt).is_err(),
        "same generation but changed actual config invalidates B"
    );
    assert_eq!(state(&store), before);
    assert!(
        store
            .record_sitemap_source_capture(
                &task(),
                &value.document,
                &captured[0].measurement,
                first
            )
            .is_err(),
        "old dedupe cannot bypass current config guard"
    );
    let second = store.begin_run("resumed actual stored source").unwrap();
    assert!(b(&store, first, value, &receipt).is_err());
    let fresh = batch(second, &body(), true);
    let new_receipt = a(&store, second, &fresh, &body());
    b(&store, second, fresh, &new_receipt).unwrap();
    let saved = snapshot(&store);
    assert_eq!(saved.records.len(), 2);
    assert_eq!(saved.commits.len(), 1);
    assert_eq!(
        saved.records[0], captured[0],
        "immutable historical capture config/generation cannot be rewritten"
    );
    assert_eq!(done(&store), 1);
}

#[test]
fn real_wal_read_transaction_observes_atomic_a_and_link_state_without_mixed_context() {
    let (temp, store, generation) = fixture(false, Default::default());
    let mut reader = Connection::open(temp.path().join("source.sqlite")).unwrap();
    let tx = reader.transaction().unwrap();
    let old = load_sitemap_read_snapshot_verified(&tx).unwrap();
    let value = batch(generation, &body(), true);
    let receipt = a(&store, generation, &value, &body());
    let still_old = load_sitemap_read_snapshot_verified(&tx).unwrap();
    assert_eq!(
        still_old.sitemap_source_measurement_context,
        old.sitemap_source_measurement_context
    );
    assert_eq!(
        still_old.sitemap_source_measurements,
        old.sitemap_source_measurements
    );
    let a_only = store.sitemap_read_snapshot().unwrap();
    assert_eq!(
        a_only.catalog, old.catalog,
        "A changes no legacy membership"
    );
    assert_ne!(
        a_only.sitemap_source_measurement_context,
        old.sitemap_source_measurement_context
    );
    b(&store, generation, value, &receipt).unwrap();
    assert_eq!(
        load_sitemap_read_snapshot_verified(&tx)
            .unwrap()
            .sitemap_source_measurements,
        old.sitemap_source_measurements
    );
    tx.commit().unwrap();
    let linked = store.sitemap_read_snapshot().unwrap();
    assert_ne!(
        linked.sitemap_source_measurement_context,
        a_only.sitemap_source_measurement_context
    );
    assert_eq!(linked.sitemap_source_measurements.unwrap().commits.len(), 1);
}

#[test]
fn actual_identity_and_gzip_captures_with_same_legacy_id_only_link_exact_b_receipt() {
    let (_temp, store, generation) = fixture(false, Default::default());
    let plain = batch(generation, &body(), true);
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&body()).unwrap();
    let compressed = encoder.finish().unwrap();
    let zipped = batch(generation, &compressed, true);
    assert_eq!(
        plain.document.observation_id,
        zipped.document.observation_id
    );
    let plain_receipt = a(&store, generation, &plain, &body());
    let gzip_receipt = a(&store, generation, &zipped, &compressed);
    assert_ne!(
        plain_receipt.measurement_id(),
        gzip_receipt.measurement_id()
    );
    let before = state(&store);
    assert!(
        b(&store, generation, plain, &gzip_receipt).is_err(),
        "same expected legacy ID is not an exact capture/document seal"
    );
    assert_eq!(state(&store), before);
    b(&store, generation, zipped, &gzip_receipt).unwrap();
    let saved = snapshot(&store);
    assert_eq!(saved.records.len(), 2);
    assert_eq!(saved.commits.len(), 1);
    assert_eq!(
        saved.commits[0].measurement_id,
        gzip_receipt.measurement_id().unwrap()
    );
}

#[test]
fn real_capture_unavailable_marker_survives_b_rollback_without_becoming_budget_or_zero_measurement()
{
    let (_temp, store, generation) = fixture(
        false,
        crate::SitemapLedgerLimits {
            max_rows: 1,
            ..Default::default()
        },
    );
    let value = batch(generation, &body(), true);
    let config_sha = store
        .sitemap_read_snapshot()
        .unwrap()
        .crawl_config_sha256
        .unwrap();
    let receipt = store
        .record_unavailable_sitemap_source_capture(
            &task(),
            &value.document,
            &config_sha,
            generation,
        )
        .unwrap();
    assert!(!receipt.is_persisted());
    assert_eq!(
        receipt.omission_reason(),
        Some(crate::SitemapSourceStoreOmission::CaptureUnavailable)
    );
    let before = snapshot(&store);
    assert_eq!(
        before.summary.omission,
        Some(crate::SitemapSourceStoreOmission::CaptureUnavailable)
    );
    assert!(before.records.is_empty() && before.commits.is_empty());
    assert!(b(&store, generation, value, &receipt).is_err());
    assert_eq!(snapshot(&store), before);
    assert_eq!(done(&store), 0);
}

#[test]
fn legacy_readonly_has_no_measurement_catalog_and_does_not_create_or_migrate_files() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("legacy.sqlite");
    let c = Connection::open(&path).unwrap();
    c.execute_batch("CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    drop(c);
    let bytes = std::fs::read(&path).unwrap();
    let mut files = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<Vec<_>>();
    files.sort();
    let reader = CrawlStore::open_read_only(&path).unwrap();
    assert!(
        reader
            .sitemap_read_snapshot()
            .unwrap()
            .sitemap_source_measurements
            .is_none()
    );
    drop(reader);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let mut after = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<Vec<_>>();
    after.sort();
    assert_eq!(after, files);
    let writable = CrawlStore::open(&path).unwrap();
    assert!(
        writable
            .sitemap_read_snapshot()
            .unwrap()
            .sitemap_source_measurements
            .is_none(),
        "schema existence alone is not measured initialization"
    );
}

#[test]
fn actual_checkpoint_selected_parent_preserves_measurements_and_rejects_exact_blob_tamper() {
    let (_temp, store, generation) = fixture(false, Default::default());
    let value = batch(generation, &body(), true);
    let receipt = a(&store, generation, &value, &body());
    b(&store, generation, value, &receipt).unwrap();
    store
        .complete(
            &PageRecord {
                url: PAGE.into(),
                final_url: PAGE.into(),
                status: Some(200),
                final_status: Some(200),
                ..Default::default()
            },
            &[],
            generation,
        )
        .unwrap();
    let saved = report(&store);
    let checkpoint = store
        .save_checkpoint(&saved, CrawlOutcome::Completed, generation)
        .unwrap();
    assert!(store.inspection().unwrap().checkpoint_valid);
    assert_eq!(
        checkpoint.sitemap_source_measurements,
        Some(snapshot(&store).summary)
    );
    let bytes = crate::recrawl::bounded_report_json(&saved)
        .unwrap()
        .into_bytes();
    let prepared = crate::prepare_selected_recrawl(
        crate::SelectedRecrawlParent {
            job_id: "actual-source-parent".into(),
            checkpoint_revision: Some(checkpoint.revision),
            source_kind: crate::RecrawlSourceKind::CapturedCheckpoint,
        },
        &bytes,
        &[PAGE.into()],
        None,
    )
    .unwrap();
    let child_dir = tempfile::tempdir().unwrap();
    let child = CrawlStore::open(child_dir.path().join("selected.sqlite")).unwrap();
    child.initialize_selected_recrawl(&prepared).unwrap();
    let inherited = snapshot(&child);
    assert_eq!(
        inherited.records,
        saved.sitemap_source_measurements.as_ref().unwrap().records
    );
    assert_eq!(
        inherited.commits,
        saved.sitemap_source_measurements.as_ref().unwrap().commits
    );
    assert!(
        inherited
            .summary
            .inherited
            .as_ref()
            .is_some_and(|basis| basis.parent_snapshot_sha256
                == prepared.plan.parent.snapshot_sha256
                && !basis.current_discovery_executed)
    );
    let second = child
        .begin_run("selected page refresh, no sitemap discovery")
        .unwrap();
    child.start_recrawl_invocation(second, None).unwrap();
    assert_eq!(
        snapshot(&child),
        inherited,
        "child invocation cannot rewrite parent captures"
    );
    assert!(
        child
            .record_sitemap_source_capture(
                &task(),
                &batch(second, &body(), true).document,
                &measurement(&child, &batch(second, &body(), true), &body()),
                second
            )
            .is_err()
    );
    let mut changed = bytes.clone();
    changed.push(b' ');
    child
        .connection
        .lock()
        .unwrap()
        .execute("UPDATE recrawl_base SET snapshot=?1 WHERE id=1", [changed])
        .unwrap();
    assert!(
        child.sitemap_read_snapshot().is_err(),
        "even JSON-equivalent exact parent bytes must stay bound"
    );
}

#[test]
fn actual_checkpoint_rejects_measurement_only_change_and_failed_publication_preserves_old_report() {
    let (_temp, store, generation) = fixture(false, Default::default());
    let value = batch(generation, &body(), true);
    let receipt = a(&store, generation, &value, &body());
    b(&store, generation, value, &receipt).unwrap();
    let saved = report(&store);
    let first = store
        .save_checkpoint(&saved, CrawlOutcome::Paused, generation)
        .unwrap();
    let next = store
        .begin_run("new generation invalidates terminal binding")
        .unwrap();
    let mut changed = report(&store);
    changed.sitemap_source_measurements = None;
    assert!(
        store
            .save_checkpoint(&changed, CrawlOutcome::Paused, next)
            .is_err()
    );
    assert_eq!(store.checkpoint().unwrap(), Some(first));
    assert_eq!(
        store
            .load_report()
            .unwrap()
            .unwrap()
            .sitemap_source_measurements,
        saved.sitemap_source_measurements
    );
    store.connection.lock().unwrap().execute("UPDATE metadata SET value=json_set(value,'$.engine','tampered') WHERE key='sitemap_source_store'",[]).unwrap();
    assert!(
        store.inspection().is_err(),
        "source-only metadata corruption cannot preserve a trusted checkpoint"
    );
}

#[test]
fn real_storage_budget_marker_is_durable_across_membership_rollback_and_not_a_measured_zero() {
    let (_temp, store, generation) = fixture_with_measurement_limits(
        false,
        crate::SitemapLedgerLimits {
            max_rows: 1,
            ..Default::default()
        },
        crate::SitemapSourceStoreLimits {
            max_record_json_bytes: 1,
            ..Default::default()
        },
    );
    let value = batch(generation, &body(), true);
    let receipt = a(&store, generation, &value, &body());
    assert!(!receipt.is_persisted());
    assert_eq!(
        receipt.omission_reason(),
        Some(crate::SitemapSourceStoreOmission::StorageBudgetExceeded)
    );
    let before = snapshot(&store);
    assert_eq!(
        before.summary.omission,
        Some(crate::SitemapSourceStoreOmission::StorageBudgetExceeded)
    );
    assert!(before.summary.omission_revision.is_some());
    assert!(before.records.is_empty() && before.commits.is_empty());
    let state_after_a = state(&store);
    assert!(b(&store, generation, value, &receipt).is_err());
    assert_eq!(snapshot(&store), before);
    assert_eq!(state(&store), state_after_a);
    assert_eq!(done(&store), 0);
}

#[test]
fn actual_a_record_and_marker_sql_failures_never_produce_receipts_or_admit_sources() {
    for marker in [false, true] {
        let (_temp, store, generation) = fixture_with_measurement_limits(
            false,
            Default::default(),
            crate::SitemapSourceStoreLimits {
                max_record_json_bytes: if marker { 1 } else { 16_384 },
                ..Default::default()
            },
        );
        let value = batch(generation, &body(), true);
        let measurement = measurement(&store, &value, &body());
        let sql = if marker {
            "CREATE TRIGGER fail_a_marker BEFORE INSERT ON metadata WHEN NEW.key='sitemap_source_store' BEGIN SELECT RAISE(ABORT,'injected actual A marker failure'); END;"
        } else {
            "CREATE TRIGGER fail_a_record BEFORE INSERT ON sitemap_source_measurements BEGIN SELECT RAISE(ABORT,'injected actual A record failure'); END;"
        };
        store.connection.lock().unwrap().execute_batch(sql).unwrap();
        let before = state(&store);
        assert!(
            store
                .record_sitemap_source_capture(&task(), &value.document, &measurement, generation)
                .is_err()
        );
        assert_eq!(state(&store), before);
        assert_eq!(done(&store), 0);
        assert!(snapshot(&store).records.is_empty());
        assert!(
            snapshot(&store).summary.omission.is_none(),
            "failed marker SQL must not be reported as durable omission"
        );
    }
}

#[test]
fn portable_report_counter_applies_exact_utf8_json_byte_boundary_without_partial_publication() {
    let (_temp, store, generation) = fixture(false, Default::default());
    let value = batch(generation, &body(), true);
    let receipt = a(&store, generation, &value, &body());
    b(&store, generation, value, &receipt).unwrap();
    let mut saved = report(&store);
    saved.warnings = vec!["€ quoted \" and slash \\ & <".into()];
    let encoded = serde_json::to_vec(&saved).unwrap();
    crate::recrawl::validate_portable_report_size_with_limit(&saved, encoded.len()).unwrap();
    assert!(
        crate::recrawl::validate_portable_report_size_with_limit(&saved, encoded.len() - 1)
            .is_err()
    );
    assert!(crate::recrawl::validate_portable_report_size_with_limit(&saved, 268_435_457).is_err());
    assert!(
        store.load_report().unwrap().is_none(),
        "validation cannot partially publish a report"
    );
    // This exact small UTF8 boundary exercises the actual production counter;
    // it is not a physical256MiB native memory/serialization acceptance claim.
}

#[test]
fn writable_open_does_not_silently_recreate_missing_empty_measurement_tables() {
    for mutation in 0..3 {
        let (temp, store, _generation) = fixture(false, Default::default());
        assert!(snapshot(&store).records.is_empty());
        let connection = store.connection.lock().unwrap();
        connection
            .execute_batch("DROP TABLE sitemap_source_measurement_commits")
            .unwrap();
        if mutation == 1 {
            connection
                .execute_batch("DROP TABLE sitemap_source_measurements")
                .unwrap();
        }
        if mutation == 2 {
            connection
                .execute("DELETE FROM metadata WHERE key='sitemap_source_store'", [])
                .unwrap();
        }
        drop(connection);
        drop(store);
        let path = temp.path().join("source.sqlite");
        assert!(
            CrawlStore::open_read_only(&path)
                .unwrap()
                .sitemap_read_snapshot()
                .is_err()
        );
        assert!(
            CrawlStore::open(&path).is_err(),
            "writable open must reject incomplete sidecar, mutation{mutation}"
        );
    }
}

fn prepared_parent_for_source_presence(
    include_measurements: bool,
) -> (
    tempfile::TempDir,
    CrawlStore,
    crate::PreparedSelectedRecrawl,
) {
    let (temp, store, generation) = fixture(false, Default::default());
    let value = batch(generation, &body(), true);
    let receipt = a(&store, generation, &value, &body());
    b(&store, generation, value, &receipt).unwrap();
    store
        .complete(
            &PageRecord {
                url: PAGE.into(),
                final_url: PAGE.into(),
                status: Some(200),
                final_status: Some(200),
                ..Default::default()
            },
            &[],
            generation,
        )
        .unwrap();
    if !include_measurements {
        // Explicit SQL historical-absence fixture, not an assertion that a new
        // production writer may delete captures. The actual saved parent below
        // has no optional source snapshot, and is the child's immutable authority.
        let mut connection = store.connection.lock().unwrap();
        let tx = connection.transaction().unwrap();
        tx.execute("DELETE FROM sitemap_source_measurement_commits", [])
            .unwrap();
        tx.execute("DELETE FROM sitemap_source_measurements", [])
            .unwrap();
        tx.execute("DELETE FROM metadata WHERE key='sitemap_source_store'", [])
            .unwrap();
        tx.commit().unwrap();
    }
    let parent = report(&store);
    assert_eq!(
        parent.sitemap_source_measurements.is_some(),
        include_measurements
    );
    let checkpoint = store
        .save_checkpoint(&parent, CrawlOutcome::Completed, generation)
        .unwrap();
    assert!(store.inspection().unwrap().checkpoint_valid);
    let bytes = crate::recrawl::bounded_report_json(&parent)
        .unwrap()
        .into_bytes();
    let prepared = crate::prepare_selected_recrawl(
        crate::SelectedRecrawlParent {
            job_id: format!("parent-source-presence-{include_measurements}"),
            checkpoint_revision: Some(checkpoint.revision),
            source_kind: crate::RecrawlSourceKind::CapturedCheckpoint,
        },
        &bytes,
        &[PAGE.into()],
        None,
    )
    .unwrap();
    (temp, store, prepared)
}

#[test]
fn selected_deleted_measurement_rows_and_marker_fail_verified_read_report_and_checkpoint_paths() {
    let (_parent_dir, _parent, prepared) = prepared_parent_for_source_presence(true);
    let child_dir = tempfile::tempdir().unwrap();
    let path = child_dir.path().join("missing-child-history.sqlite");
    let child = CrawlStore::open(&path).unwrap();
    child.initialize_selected_recrawl(&prepared).unwrap();
    assert_eq!(snapshot(&child).records.len(), 1);
    assert_eq!(snapshot(&child).commits.len(), 1);
    let generation = child
        .begin_run("valid paused selected child before source fault")
        .unwrap();
    child.start_recrawl_invocation(generation, None).unwrap();
    let saved = report(&child);
    let checkpoint = child
        .save_checkpoint(&saved, CrawlOutcome::Paused, generation)
        .unwrap();
    assert!(child.inspection().unwrap().checkpoint_valid);
    let report_before = metadata_text(&child.connection.lock().unwrap(), "report").unwrap();
    {
        let mut connection = child.connection.lock().unwrap();
        let tx = connection.transaction().unwrap();
        tx.execute("DELETE FROM sitemap_source_measurement_commits", [])
            .unwrap();
        tx.execute("DELETE FROM sitemap_source_measurements", [])
            .unwrap();
        tx.execute("DELETE FROM metadata WHERE key='sitemap_source_store'", [])
            .unwrap();
        tx.commit().unwrap();
    }
    assert!(
        crate::sitemap_source_store::load(&child.connection.lock().unwrap())
            .unwrap()
            .is_none(),
        "lowlevel storage absence remains valid during initialization; higher reader owns parent presence"
    );
    assert!(
        child.sitemap_catalog().unwrap().is_some(),
        "legacy exact parent evidence remains intact"
    );
    let error = child.sitemap_read_snapshot().unwrap_err();
    assert!(error.to_string().contains("presence differs"), "{error:#}");
    assert!(
        child
            .sitemap_read_snapshot_with_membership_analysis(&Default::default())
            .is_err()
    );
    assert!(child.inspection().is_err());
    assert!(child.load_report().is_err());
    assert!(child.save_report(&saved).is_err());
    let readonly = CrawlStore::open_read_only(&path).unwrap();
    assert!(readonly.sitemap_read_snapshot().is_err());
    assert!(readonly.load_report().is_err());
    assert!(readonly.inspection().is_err());
    drop(readonly);
    let next = child
        .begin_run("new child run cannot hide missing parent history")
        .unwrap();
    assert!(
        child
            .save_checkpoint(&saved, CrawlOutcome::Paused, next)
            .is_err()
    );
    assert_eq!(child.checkpoint().unwrap(), Some(checkpoint));
    assert_eq!(
        metadata_text(&child.connection.lock().unwrap(), "report").unwrap(),
        report_before,
        "failed save_report/save_checkpoint never replace the old exact snapshot"
    );
}

#[test]
fn selected_historical_parent_none_remains_unknown_and_exact_parent_tamper_still_fails() {
    let (_parent_dir, _parent, prepared) = prepared_parent_for_source_presence(false);
    let child_dir = tempfile::tempdir().unwrap();
    let path = child_dir.path().join("legacy-parent-none.sqlite");
    let child = CrawlStore::open(&path).unwrap();
    child.initialize_selected_recrawl(&prepared).unwrap();
    assert!(
        child
            .sitemap_read_snapshot()
            .unwrap()
            .sitemap_source_measurements
            .is_none()
    );
    let generation = child.begin_run("selected legacy parent control").unwrap();
    child.start_recrawl_invocation(generation, None).unwrap();
    let saved = report(&child);
    assert!(saved.sitemap_source_measurements.is_none());
    child
        .save_checkpoint(&saved, CrawlOutcome::Paused, generation)
        .unwrap();
    assert!(child.inspection().unwrap().checkpoint_valid);
    assert!(
        CrawlStore::open_read_only(&path)
            .unwrap()
            .load_report()
            .unwrap()
            .unwrap()
            .sitemap_source_measurements
            .is_none()
    );
    let mut changed = prepared.base_snapshot_bytes.clone();
    changed.push(b' ');
    child
        .connection
        .lock()
        .unwrap()
        .execute("UPDATE recrawl_base SET snapshot=?1 WHERE id=1", [changed])
        .unwrap();
    assert!(
        child.sitemap_read_snapshot().is_err(),
        "None is not permission to skip exact BLOB/saved-plan validation"
    );
    assert!(child.load_report().is_err());
    assert!(child.inspection().is_err());
}

#[test]
fn actual_raw_window_a_only_capture_changes_state_without_inventing_committed_raw_rows() {
    let (_temp, store, generation) = fixture(false, Default::default());
    let request = crate::SitemapRawReadRequest::default();
    let before = store.sitemap_raw_window(&request).unwrap();
    let value = batch(generation, &body(), true);
    let _receipt = a(&store, generation, &value, &body());
    let after = store.sitemap_raw_window(&request).unwrap();
    assert_eq!(after.context.ledger, before.context.ledger);
    assert_eq!(after.rows, before.rows);
    assert_eq!(
        after.context.source_summary.as_ref().unwrap().record_count,
        1
    );
    assert_eq!(
        after.context.source_summary.as_ref().unwrap().commit_count,
        0
    );
    assert_ne!(after.state_sha256, before.state_sha256);
    let old = store.sitemap_read_snapshot().unwrap();
    assert_eq!(
        after.context.source_context_sha256,
        old.sitemap_source_measurement_context
    );
    let stale = crate::SitemapRawReadRequest {
        expected_state_sha256: Some(before.state_sha256),
        ..Default::default()
    };
    assert!(
        store
            .sitemap_raw_window(&stale)
            .unwrap_err()
            .to_string()
            .contains("state stale")
    );
}

#[test]
fn actual_raw_window_selected_parent_presence_guards_remain_materializing_compatibility() {
    for measurements in [false, true] {
        let (_parent_dir, _parent, prepared) = prepared_parent_for_source_presence(measurements);
        let child_dir = tempfile::tempdir().unwrap();
        let child = CrawlStore::open(child_dir.path().join("raw-selected.sqlite")).unwrap();
        child.initialize_selected_recrawl(&prepared).unwrap();
        let read = child.sitemap_raw_window(&Default::default()).unwrap();
        assert!(read.context.materializing_selected_compatibility);
        assert_eq!(read.context.source_summary.is_some(), measurements);
        if measurements {
            let mut connection = child.connection.lock().unwrap();
            let tx = connection.transaction().unwrap();
            tx.execute("DELETE FROM sitemap_source_measurement_commits", [])
                .unwrap();
            tx.execute("DELETE FROM sitemap_source_measurements", [])
                .unwrap();
            tx.execute("DELETE FROM metadata WHERE key='sitemap_source_store'", [])
                .unwrap();
            tx.commit().unwrap();
            drop(connection);
            assert!(child.sitemap_raw_window(&Default::default()).is_err());
        } else {
            let mut changed = prepared.base_snapshot_bytes.clone();
            changed.push(b' ');
            child
                .connection
                .lock()
                .unwrap()
                .execute("UPDATE recrawl_base SET snapshot=?1 WHERE id=1", [changed])
                .unwrap();
            assert!(child.sitemap_raw_window(&Default::default()).is_err());
        }
    }
}

fn rich_expected_context(
    read: &crate::SitemapReadWindow,
) -> crate::SitemapReadWindowExpectedContext {
    let summary = read.ledger.as_ref().unwrap();
    crate::SitemapReadWindowExpectedContext {
        generation: read.generation.unwrap(),
        crawl_config_sha256: read.crawl_config_sha256.clone().unwrap(),
        catalog_config_sha256: summary.config_sha256.clone(),
        catalog_revision: summary.revision,
        catalog_rolling_sha256: summary.rolling_sha256.clone(),
        catalog_state_sha256: read.catalog_state_sha256.clone().unwrap(),
        inherited_basis: read.inherited.clone(),
    }
}

#[test]
fn actual_rich_window_a_only_and_committed_measurements_bind_the_shared_read_state() {
    let (_temp, store, generation) = fixture(false, Default::default());
    let request = crate::SitemapMembershipAnalysisRequest {
        row_limit: 200,
        page_limit: 0,
        ..Default::default()
    };
    let before = store
        .sitemap_read_window_with_membership_analysis(&request, None)
        .unwrap();
    let value = batch(generation, &body(), true);
    let receipt = a(&store, generation, &value, &body());
    let staged = store
        .sitemap_read_window_with_membership_analysis(&request, None)
        .unwrap();
    assert_eq!(staged.ledger, before.ledger);
    assert_eq!(staged.rows, before.rows);
    assert_ne!(staged.catalog_state_sha256, before.catalog_state_sha256);
    assert_eq!(
        staged
            .sitemap_source_measurement_summary
            .as_ref()
            .unwrap()
            .record_count,
        1
    );
    assert_eq!(
        staged
            .sitemap_source_measurement_summary
            .as_ref()
            .unwrap()
            .commit_count,
        0
    );
    let stale = store
        .sitemap_read_window_with_membership_analysis(
            &request,
            Some(&rich_expected_context(&before)),
        )
        .unwrap_err();
    assert_eq!(
        stale.downcast_ref::<crate::SitemapReadWindowError>(),
        Some(&crate::SitemapReadWindowError::StaleContext)
    );
    b(&store, generation, value, &receipt).unwrap();
    let committed = store
        .sitemap_read_window_with_membership_analysis(&request, None)
        .unwrap();
    let old = store
        .sitemap_read_snapshot_with_membership_analysis(&request)
        .unwrap();
    assert_eq!(committed.rows, old.catalog.unwrap().rows);
    assert_eq!(committed.membership_analysis, old.membership_analysis);
    assert_eq!(
        committed
            .sitemap_source_measurement_summary
            .as_ref()
            .unwrap()
            .commit_count,
        1
    );
    assert_ne!(committed.catalog_state_sha256, staged.catalog_state_sha256);
}

#[test]
fn actual_rich_window_selected_raw_and_source_parent_presence_are_not_downgraded_to_unknown() {
    for measurements in [false, true] {
        let (_parent_dir, _parent, prepared) = prepared_parent_for_source_presence(measurements);
        let child_dir = tempfile::tempdir().unwrap();
        let child = CrawlStore::open(child_dir.path().join("rich-selected.sqlite")).unwrap();
        child.initialize_selected_recrawl(&prepared).unwrap();
        let request = crate::SitemapMembershipAnalysisRequest {
            row_limit: 200,
            ..Default::default()
        };
        let read = child
            .sitemap_read_window_with_membership_analysis(&request, None)
            .unwrap();
        assert!(read.materializing_selected_compatibility);
        assert!(read.ledger.is_some());
        assert_eq!(
            read.sitemap_source_measurement_summary.is_some(),
            measurements
        );
        let mut connection = child.connection.lock().unwrap();
        let tx = connection.transaction().unwrap();
        if measurements {
            tx.execute("DELETE FROM sitemap_source_measurement_commits", [])
                .unwrap();
            tx.execute("DELETE FROM sitemap_source_measurements", [])
                .unwrap();
            tx.execute("DELETE FROM metadata WHERE key='sitemap_source_store'", [])
                .unwrap();
        } else {
            tx.execute_batch(
                "DELETE FROM sitemap_locations; DELETE FROM sitemap_entries;
                DELETE FROM sitemap_roots; DELETE FROM sitemap_documents;
                DELETE FROM metadata WHERE key IN ('sitemap_ledger','sitemap_inherited_basis');",
            )
            .unwrap();
        }
        tx.commit().unwrap();
        drop(connection);
        assert!(
            child
                .sitemap_read_window_with_membership_analysis(&request, None)
                .is_err()
        );
    }
}

#[test]
fn actual_rich_window_selected_imported_legacy_none_is_unknown_with_exact_parent_guard() {
    let (_parent_dir, _parent, modern) = prepared_parent_for_source_presence(false);
    // Explicit portable historical-absence fixture, not an accepted modern
    // writer deleting its saved evidence. The actual import reconstructs a plan.
    let mut historical = modern.base_report;
    historical.sitemap_catalog = None;
    historical.sitemap_coverage_inventory = None;
    historical.sitemap_source_measurements = None;
    let bytes = crate::recrawl::bounded_report_json(&historical)
        .unwrap()
        .into_bytes();
    let prepared = crate::prepare_selected_recrawl(
        crate::SelectedRecrawlParent {
            job_id: "imported-historical-none-fixture".into(),
            checkpoint_revision: None,
            source_kind: crate::RecrawlSourceKind::ImportedSnapshot,
        },
        &bytes,
        &[PAGE.into()],
        None,
    )
    .unwrap();
    let child_dir = tempfile::tempdir().unwrap();
    let child = CrawlStore::open(child_dir.path().join("rich-historical-none.sqlite")).unwrap();
    child.initialize_selected_recrawl(&prepared).unwrap();
    let request = crate::SitemapMembershipAnalysisRequest {
        row_limit: 10,
        ..Default::default()
    };
    let read = child
        .sitemap_read_window_with_membership_analysis(&request, None)
        .unwrap();
    assert!(read.materializing_selected_compatibility);
    assert!(read.ledger.is_none());
    assert!(read.rows.is_none());
    assert!(read.sitemap_source_measurement_summary.is_none());
    assert_eq!(
        read.coverage.scope,
        crate::SitemapCoverageScope::Unavailable
    );
    assert_eq!(
        read.coverage.completion,
        crate::SitemapCohortCompletion::Unknown
    );
    let mut changed = prepared.base_snapshot_bytes;
    changed.push(b' ');
    child
        .connection
        .lock()
        .unwrap()
        .execute("UPDATE recrawl_base SET snapshot=?1 WHERE id=1", [changed])
        .unwrap();
    assert!(
        child
            .sitemap_read_window_with_membership_analysis(&request, None)
            .is_err()
    );
}

#[path = "store/sitemap_stored_source_tests.rs"]
mod stored_source_session_tests;
