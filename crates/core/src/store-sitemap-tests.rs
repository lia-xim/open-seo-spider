//! Directed Store integration candidates. AUTHORED, NOT COMPILED OR RUN.
//! Includes the ACTUAL Store methods and ACTUAL parser/decoder/ledger, no shadow implementation.
use super::*;

const TASK: &str = "https://example.org/sitemap.xml";
const PAGE: &str = "https://example.org/from-sitemap";

fn fixture(
    policy: bool,
    limits: crate::SitemapLedgerLimits,
) -> (tempfile::TempDir, CrawlStore, u64, SitemapCatalogInit) {
    let temp = tempfile::tempdir().unwrap();
    let store = CrawlStore::open(temp.path().join("crawl.sqlite")).unwrap();
    let mut config = CrawlConfig {
        seed_urls: vec!["https://example.org/".into()],
        obey_robots: false,
        ..Default::default()
    };
    // This fixture declares TASK on both ordinary and policy paths; preserve its original value.
    config.sitemap_urls = vec![TASK.into()];
    config.explicit_sitemap_inputs = Some(vec![TASK.into()]);
    if policy {
        let mut policy = crate::CrawlPolicyConfig::default();
        policy.scope.roots.push(crate::CrawlScopeRoot {
            origin: "https://example.org".into(),
            subdomains: false,
            folders: vec!["/".into()],
            exact_paths: vec![],
        });
        config.crawl_policy = Some(policy);
        config.sitemap_urls = vec![TASK.into()];
        config.explicit_sitemap_inputs = Some(vec![TASK.into()]);
    }
    config.prepare_input_lists().unwrap();
    store.prepare(&config).unwrap();
    let generation = store.begin_run("store-sitemap-fixture").unwrap();
    if let Some(policy) = &config.crawl_policy {
        let policy = crate::CompiledCrawlPolicy::new(policy).unwrap();
        let revision = format!(
            "initial:{}:explicit:sitemap_document",
            policy.policy_sha256()
        );
        let mut initial_rows = vec![crate::policy_ledger::decide(
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
            .initialize_policy(
                &policy,
                &mut initial_rows,
                &crate::LiveCrawlPolicyLimits::default(),
                generation,
            )
            .unwrap();
        assert!(
            store
                .task_policy(crate::PolicyTaskRole::SitemapDocument, TASK)
                .unwrap()
                .is_some(),
            "Policy source fixture needs actual durable sitemap task permission"
        );
    }
    let config_text = metadata_text(&store.connection.lock().unwrap(), "config")
        .unwrap()
        .unwrap();
    let init = SitemapCatalogInit {
        config_sha256: sha256(&config_text),
        limits,
        roots: vec![],
        legacy_history_unknown: false,
    };
    (temp, store, generation, init)
}
fn entry() -> DiscoveryEntry {
    DiscoveryEntry {
        kind: DiscoveryKind::Sitemap,
        url: TASK.into(),
    }
}
fn initialize(store: &CrawlStore, generation: u64, init: &SitemapCatalogInit) {
    store
        .initialize_sitemap_discovery(&[entry()], generation, init)
        .unwrap();
}
fn batch(generation: u64, xml: &str, ordinary: bool) -> crate::SitemapDiscoveryBatch {
    use crate::sitemap_ledger::*;
    let limits = crate::SitemapParserLimits::default();
    let decoded = crate::decode_sitemap_entity(xml.as_bytes(), &limits).unwrap();
    let observed = crate::observe_sitemap_xml(
        &decoded.xml,
        crate::SitemapParserContext {
            requested_document_url: Some(TASK.into()),
            final_document_url: Some(TASK.into()),
            originating_occurrence_id: None,
        },
        &limits,
    )
    .unwrap();
    let revision = digest(xml.as_bytes());
    let document = SitemapSourceDocument {
        sequence: 0,
        observation_id: source_observation_id(
            SitemapSourceKind::SitemapXml,
            TASK,
            generation,
            &revision,
        )
        .unwrap(),
        kind: SitemapSourceKind::SitemapXml,
        task_url: TASK.into(),
        generation,
        source_revision: revision,
        transport: Some(SitemapTransportFacts {
            request_started: true,
            requested_url: Some(TASK.into()),
            final_url: Some(TASK.into()),
            initial_status: Some(200),
            final_status: Some(200),
            blocked_by_robots: false,
            body_complete: true,
            post_http_decoding_bytes: xml.len() as u64,
            post_http_decoding_sha256: Some(digest(xml)),
            retained_post_http_prefix_sha256: Some(digest(xml)),
            failure_code: None,
        }),
        entity_decode: Some(SitemapDecodeFacts::from(&decoded)),
        parser: None,
        declaration_scan_complete: false,
        declaration_utf8_lossy: None,
        declaration_count: None,
        retained_rows_complete: true,
        failure_code: None,
    };
    let mut batch = SitemapDiscoveryBatch::from_parser(document, observed).unwrap();
    if ordinary {
        for location in &mut batch.locations {
            if location.admission.decision == SitemapAdmissionDecision::Unbound {
                location.admission.decision = SitemapAdmissionDecision::Scheduled;
                location.admission.admitted_fetch_url =
                    location.observed.normalized_fetch_url.clone();
                if location.observed.role == crate::SitemapLocationRole::ChildSitemap {
                    location.referenced_sitemap_task =
                        location.admission.admitted_fetch_url.clone();
                }
            }
        }
    }
    batch
}
fn one_page_batch(generation: u64) -> crate::SitemapDiscoveryBatch {
    batch(
        generation,
        &format!("<urlset><url><loc>{PAGE}</loc></url></urlset>"),
        true,
    )
}
fn queued_page() -> QueueEntry {
    QueueEntry {
        url: PAGE.into(),
        depth: 0,
        discovered_from: Some(TASK.into()),
        admission: None,
    }
}
fn done(store: &CrawlStore) -> i64 {
    store
        .connection
        .lock()
        .unwrap()
        .query_row(
            "SELECT done FROM discovery_frontier WHERE kind='sitemap' AND url=?1",
            [TASK],
            |row| row.get(0),
        )
        .unwrap()
}
fn saved_state(store: &CrawlStore) -> Vec<(String, Vec<String>)> {
    let connection = store.connection.lock().unwrap();
    let mut output = Vec::new();
    for table in [
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
        "recrawl_base",
        "recrawl_pages",
        "recrawl_targets",
    ] {
        let mut query = connection
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap();
        let columns = query.column_count();
        let rows = query
            .query_map([], |row| {
                let mut values = Vec::new();
                for index in 0..columns {
                    values.push(format!("{:?}", row.get_ref(index)?));
                }
                Ok(values.join("|"))
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        output.push((table.into(), rows));
    }
    output
}
fn ordinary_commit(store: &CrawlStore, generation: u64) -> anyhow::Result<()> {
    store.complete_discovery_observed(
        &entry(),
        &[queued_page()],
        &[],
        &["source warning".into()],
        generation,
        one_page_batch(generation),
    )
}
fn report(store: &CrawlStore, catalog: Option<crate::SitemapCatalog>) -> CrawlReport {
    let snapshot = store.sitemap_read_snapshot().unwrap();
    CrawlReport {
        config: store.config().unwrap().unwrap(),
        pages: store.pages().unwrap(),
        issues: vec![],
        warnings: vec![],
        operation: store.recrawl_operation().unwrap(),
        policy_ledger: store.policy_ledger_summary().unwrap(),
        resource_ledger: store.resource_ledger_summary().unwrap(),
        sitemap_catalog: catalog,
        sitemap_coverage_inventory: snapshot.coverage_inventory,
        sitemap_source_measurements: snapshot.sitemap_source_measurements,
        summary: crate::CrawlSummary {
            completed: store.pages().unwrap().len(),
            pending: store.pending_count().unwrap(),
            ..Default::default()
        },
    }
}
fn assert_error(error: anyhow::Error, needle: &str) {
    assert!(
        format!("{error:#}").contains(needle),
        "Expected {needle:?}, got {error:#}"
    );
}

#[test]
fn writable_schema_does_not_establish_empty_sitemap_history() {
    let (_temp, store, _generation, _init) = fixture(false, Default::default());
    assert!(store.sitemap_catalog().unwrap().is_none());
    assert!(store.sitemap_ledger_summary().unwrap().is_none());
    let connection = store.connection.lock().unwrap();
    for name in [
        "sitemap_documents",
        "sitemap_roots",
        "sitemap_entries",
        "sitemap_locations",
    ] {
        assert!(
            connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name=?1)",
                    [name],
                    |row| row.get::<_, bool>(0)
                )
                .unwrap()
        );
    }
}
#[test]
fn readonly_legacy_catalog_read_creates_no_tables_or_files() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("legacy.sqlite");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);")
        .unwrap();
    drop(connection);
    let before = std::fs::read(&path).unwrap();
    let mut names_before: Vec<_> = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names_before.sort();
    let reader = CrawlStore::open_read_only(&path).unwrap();
    assert!(reader.sitemap_catalog().unwrap().is_none());
    assert!(reader.sitemap_ledger_summary().unwrap().is_none());
    drop(reader);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let mut names_after: Vec<_> = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names_after.sort();
    assert_eq!(names_after, names_before);
    let connection = Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name LIKE 'sitemap_%'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}
#[test]
fn first_initialization_binds_saved_config_but_never_declares_cohort_complete() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    initialize(&store, generation, &init);
    let summary = store.sitemap_ledger_summary().unwrap().unwrap();
    assert_eq!(summary.config_sha256, init.config_sha256);
    assert!(!summary.legacy_history_unknown);
    assert!(!summary.declared_source_cohort_complete);
    assert_eq!((summary.root_rows, summary.document_rows), (0, 0));
    assert_eq!(done(&store), 0);
}
#[test]
fn incorrect_first_config_sha_rolls_back_initialization_and_enqueue() {
    let (_temp, store, generation, mut init) = fixture(false, Default::default());
    init.config_sha256 = "0".repeat(64);
    let before = saved_state(&store);
    assert_error(
        store
            .initialize_sitemap_discovery(&[entry()], generation, &init)
            .unwrap_err(),
        "actual saved configuration bytes",
    );
    assert_eq!(saved_state(&store), before);
    assert!(store.sitemap_catalog().unwrap().is_none());
}
#[test]
fn init_finalizer_failure_rolls_back_catalog_roots_tasks_and_markers() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    let before = saved_state(&store);
    let error = store
        .initialize_sitemap_discovery_with(&[entry()], generation, &init, |_, _| {
            anyhow::bail!("injected root finalizer failure")
        })
        .unwrap_err();
    assert_error(error, "injected root finalizer failure");
    assert_eq!(saved_state(&store), before);
}
#[test]
fn legacy_completed_discovery_remains_history_unknown_after_typed_initialization() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    store.initialize_discovery(&[entry()]).unwrap();
    store
        .complete_discovery(&entry(), &[], &[], &[], generation)
        .unwrap();
    assert!(store.sitemap_catalog().unwrap().is_none());
    store
        .initialize_sitemap_discovery_with(&[entry()], generation, &init, |_, _| {
            anyhow::bail!("legacy roots must not be retroactively fabricated")
        })
        .unwrap();
    let summary = store.sitemap_ledger_summary().unwrap().unwrap();
    assert!(summary.legacy_history_unknown);
    assert!(!summary.declared_source_cohort_complete);
    assert_eq!(summary.document_rows, 0);
    assert_eq!(done(&store), 1);
}
#[test]
fn operational_resume_preserves_initial_config_hash_and_existing_roots() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    initialize(&store, generation, &init);
    let before = store.sitemap_catalog().unwrap();
    let mut config = store.config().unwrap().unwrap();
    config.max_pages = Some(123);
    config.concurrency += 1;
    config.delay_ms += 1;
    config.request_timeout_secs += 1;
    assert!(store.prepare(&config).unwrap());
    let actual = metadata_text(&store.connection.lock().unwrap(), "config")
        .unwrap()
        .unwrap();
    assert_ne!(sha256(&actual), init.config_sha256);
    let mut resumed = init.clone();
    resumed.config_sha256 = sha256(&actual);
    store
        .initialize_sitemap_discovery_with(&[entry()], generation, &resumed, |_, _| {
            anyhow::bail!("existing catalog must not repeat root projector")
        })
        .unwrap();
    assert_eq!(store.sitemap_catalog().unwrap(), before);
}
#[test]
fn repeated_ordinary_source_rejects_before_frontier_or_warning_mutation() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    initialize(&store, generation, &init);
    ordinary_commit(&store, generation).unwrap();
    let before = saved_state(&store);
    assert_error(
        ordinary_commit(&store, generation).unwrap_err(),
        "missing/already committed",
    );
    assert_eq!(saved_state(&store), before);
    assert_eq!(done(&store), 1);
}
#[test]
fn stale_source_generation_rejects_before_any_mutation() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    initialize(&store, generation, &init);
    let before = saved_state(&store);
    assert_error(
        ordinary_commit(&store, generation + 1).unwrap_err(),
        "Stale/terminal sitemap generation",
    );
    assert_eq!(saved_state(&store), before);
    assert_eq!(done(&store), 0);
}
#[test]
fn legacy_none_batch_cannot_silently_create_gap_in_initialized_catalog() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    initialize(&store, generation, &init);
    let before = saved_state(&store);
    assert_error(
        store
            .complete_discovery(
                &entry(),
                &[queued_page()],
                &[],
                &["unrecorded".into()],
                generation,
            )
            .unwrap_err(),
        "requires an observed source batch",
    );
    assert_eq!(saved_state(&store), before);
}
#[test]
fn late_location_sql_failure_rolls_back_frontier_warnings_and_source() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    initialize(&store, generation, &init);
    store.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_location AFTER INSERT ON sitemap_locations BEGIN SELECT RAISE(ABORT,'injected Store location failure'); END;").unwrap();
    let before = saved_state(&store);
    assert_error(
        ordinary_commit(&store, generation).unwrap_err(),
        "injected Store location failure",
    );
    assert_eq!(saved_state(&store), before);
    assert_eq!(done(&store), 0);
}
#[test]
fn done_update_failure_rolls_back_already_appended_catalog_and_outputs() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    initialize(&store, generation, &init);
    store.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_done BEFORE UPDATE ON discovery_frontier WHEN NEW.done=1 BEGIN SELECT RAISE(ABORT,'injected Store done failure'); END;").unwrap();
    let before = saved_state(&store);
    assert_error(
        ordinary_commit(&store, generation).unwrap_err(),
        "injected Store done failure",
    );
    assert_eq!(saved_state(&store), before);
    assert_eq!(done(&store), 0);
}
#[test]
fn closure_metadata_failure_rolls_back_done_row_and_entire_source() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    initialize(&store, generation, &init);
    store.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_closure BEFORE INSERT ON metadata WHEN NEW.key='discovery_complete' BEGIN SELECT RAISE(ABORT,'injected Store closure failure'); END;").unwrap();
    let before = saved_state(&store);
    assert_error(
        ordinary_commit(&store, generation).unwrap_err(),
        "injected Store closure failure",
    );
    assert_eq!(saved_state(&store), before);
    assert_eq!(done(&store), 0);
}
#[test]
fn source_row_budget_failure_keeps_source_pending_without_partial_outputs() {
    let limits = crate::SitemapLedgerLimits {
        max_rows: 1,
        ..Default::default()
    };
    let (_temp, store, generation, init) = fixture(false, limits);
    initialize(&store, generation, &init);
    let before = saved_state(&store);
    assert_error(
        ordinary_commit(&store, generation).unwrap_err(),
        "entire source remains pending",
    );
    assert_eq!(saved_state(&store), before);
    assert_eq!(done(&store), 0);
}
#[test]
fn incomplete_transport_rejects_scheduler_outputs_even_if_xml_is_complete() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    initialize(&store, generation, &init);
    let mut value = one_page_batch(generation);
    value.document.transport.as_mut().unwrap().body_complete = false;
    value
        .document
        .transport
        .as_mut()
        .unwrap()
        .post_http_decoding_sha256 = None;
    let before = saved_state(&store);
    assert_error(
        store
            .complete_discovery_observed(&entry(), &[queued_page()], &[], &[], generation, value)
            .unwrap_err(),
        "Incomplete sitemap source cannot admit",
    );
    assert_eq!(saved_state(&store), before);
}
#[test]
fn failed_policy_finalizer_rolls_back_actual_committed_policy_rows() {
    let (_temp, store, generation, init) = fixture(true, Default::default());
    initialize(&store, generation, &init);
    let value = batch(
        generation,
        &format!("<urlset><url><loc>{PAGE}</loc></url></urlset>"),
        false,
    );
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
    let before = saved_state(&store);
    let failure = store
        .complete_discovery_with_policy_observed(
            &entry(),
            &mut rows,
            (&revision, true),
            &[],
            generation,
            &crate::LiveCrawlPolicyLimits::default(),
            value,
            |transaction, _, finalized| {
                assert_eq!(finalized[0].admission_reason, "scheduled");
                assert!(transaction.query_row(
                    "SELECT EXISTS(SELECT 1 FROM policy_occurrences WHERE occurrence_id=?1)",
                    [&finalized[0].decision.occurrence_id],
                    |row| row.get::<_, bool>(0)
                )?);
                anyhow::bail!("injected final-policy projection failure")
            },
        )
        .unwrap_err();
    assert_error(failure, "injected final-policy projection failure");
    assert_eq!(rows, before_rows);
    assert_eq!(saved_state(&store), before);
    assert_eq!(done(&store), 0);
}
#[test]
fn policy_source_revision_or_completeness_mismatch_rejects_before_policy_commit() {
    let (_temp, store, generation, init) = fixture(true, Default::default());
    initialize(&store, generation, &init);
    let value = batch(generation, "<urlset/>", false);
    let revision = value.document.source_revision.clone();
    let before = saved_state(&store);
    for source in [("wrong-revision", true), (revision.as_str(), false)] {
        assert_error(
            store
                .complete_discovery_with_policy_observed(
                    &entry(),
                    &mut [],
                    source,
                    &[],
                    generation,
                    &crate::LiveCrawlPolicyLimits::default(),
                    value.clone(),
                    |_, _, _| anyhow::bail!("mismatch reached finalizer"),
                )
                .unwrap_err(),
            "Policy source completeness/revision differs",
        );
        assert_eq!(saved_state(&store), before);
    }
}
#[test]
fn same_caller_policy_slice_can_retry_after_finalizer_rollback_without_missing_grant() {
    let (_temp, store, generation, init) = fixture(true, Default::default());
    initialize(&store, generation, &init);
    let value = batch(
        generation,
        &format!("<urlset><url><loc>{PAGE}</loc></url></urlset>"),
        false,
    );
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
    let original = rows.clone();
    let before = saved_state(&store);
    let failure = store
        .complete_discovery_with_policy_observed(
            &entry(),
            &mut rows,
            (&revision, true),
            &[],
            generation,
            &crate::LiveCrawlPolicyLimits::default(),
            value.clone(),
            |_, _, _| anyhow::bail!("one-shot finalizer failure"),
        )
        .unwrap_err();
    assert_error(failure, "one-shot finalizer failure");
    assert_eq!(rows, original);
    assert_eq!(saved_state(&store), before);
    let (pages, _) = store
        .complete_discovery_with_policy_observed(
            &entry(),
            &mut rows,
            (&revision, true),
            &[],
            generation,
            &crate::LiveCrawlPolicyLimits::default(),
            value,
            |_, _, _| Ok(()),
        )
        .unwrap();
    assert_eq!(rows[0].admission_reason, "scheduled");
    assert_eq!(
        pages[0].admission.as_ref().unwrap().occurrence_id,
        rows[0].decision.occurrence_id
    );
    let connection = store.connection.lock().unwrap();
    crate::policy_ledger::verify(&connection, &policy).unwrap();
    assert!(
        crate::policy_ledger::saved_admission(
            &connection,
            crate::PolicyTaskRole::PageDocument,
            PAGE
        )
        .unwrap()
        .is_some()
    );
}
#[test]
fn actual_final_policy_loc_binding_commits_and_repeat_skips_finalizer() {
    let (_temp, store, generation, init) = fixture(true, Default::default());
    initialize(&store, generation, &init);
    let value = batch(
        generation,
        &format!("<urlset><url><loc>{PAGE}</loc></url></urlset>"),
        false,
    );
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
    let input = crate::policy_ledger::decide(
        &policy,
        &revision,
        1,
        crate::CandidateSourceKind::SitemapLocation,
        PAGE.into(),
        None,
        Some(TASK.into()),
        crate::PolicyInputBasis::ParserObservedXml,
        crate::PolicyTaskRole::PageDocument,
    );
    let mut rows = vec![input.clone()];
    let (pages, children) = store
        .complete_discovery_with_policy_observed(
            &entry(),
            &mut rows,
            (&revision, true),
            &[],
            generation,
            &crate::LiveCrawlPolicyLimits::default(),
            value.clone(),
            |_, _, _| Ok(()),
        )
        .unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].url, PAGE);
    assert!(children.is_empty());
    let catalog = store.sitemap_catalog().unwrap().unwrap();
    let location = catalog
        .rows
        .as_ref()
        .unwrap()
        .iter()
        .find_map(|row| match row {
            crate::SitemapLedgerRow::Location(row) => Some(row),
            _ => None,
        })
        .unwrap();
    assert_eq!(location.observed.ordinal, 1);
    assert_eq!(
        location.admission.policy_occurrence_id,
        Some(rows[0].decision.occurrence_id.clone())
    );
    assert_eq!(location.admission.admitted_fetch_url.as_deref(), Some(PAGE));
    let before = saved_state(&store);
    let mut repeated = vec![input];
    assert_error(
        store
            .complete_discovery_with_policy_observed(
                &entry(),
                &mut repeated,
                (&revision, true),
                &[],
                generation,
                &crate::LiveCrawlPolicyLimits::default(),
                value,
                |_, _, _| anyhow::bail!("repeat reached finalizer"),
            )
            .unwrap_err(),
        "missing/already committed",
    );
    assert_eq!(saved_state(&store), before);
    assert_eq!(repeated[0].admission_reason, "pending_admission");
}
#[test]
fn checkpoint_omitted_catalog_rejected_without_terminal_publication() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    initialize(&store, generation, &init);
    let before = saved_state(&store);
    assert_error(
        store
            .save_checkpoint(
                &report(&store, None),
                crate::CrawlOutcome::Paused,
                generation,
            )
            .unwrap_err(),
        "Report sitemap catalog differs",
    );
    assert_eq!(saved_state(&store), before);
}
#[test]
fn checkpoint_and_inspection_bind_the_same_complete_typed_catalog() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    initialize(&store, generation, &init);
    ordinary_commit(&store, generation).unwrap();
    let catalog = store.sitemap_catalog().unwrap();
    let report = report(&store, catalog.clone());
    let checkpoint = store
        .save_checkpoint(&report, crate::CrawlOutcome::Paused, generation)
        .unwrap();
    assert_eq!(
        checkpoint.sitemap_ledger,
        catalog.map(|value| value.summary)
    );
    let inspection = store.inspection().unwrap();
    assert!(inspection.checkpoint_valid);
    assert_eq!(inspection.sitemap_ledger, checkpoint.sitemap_ledger);
    assert_eq!(
        store.load_report().unwrap().unwrap().sitemap_catalog,
        report.sitemap_catalog
    );
    assert!(
        !checkpoint
            .sitemap_ledger
            .unwrap()
            .declared_source_cohort_complete
    );
}
#[test]
fn active_saved_report_is_old_snapshot_while_sql_catalog_can_advance() {
    let (_temp, store, generation, init) = fixture(false, Default::default());
    initialize(&store, generation, &init);
    let old = report(&store, store.sitemap_catalog().unwrap());
    store
        .save_checkpoint(&old, crate::CrawlOutcome::Paused, generation)
        .unwrap();
    let next = store.begin_run("resumed").unwrap();
    ordinary_commit(&store, next).unwrap();
    assert_ne!(store.sitemap_catalog().unwrap(), old.sitemap_catalog);
    assert_eq!(
        store.load_report().unwrap().unwrap().sitemap_catalog,
        old.sitemap_catalog
    );
    assert!(!store.inspection().unwrap().checkpoint_valid);
}
#[test]
fn selected_initialization_checks_new_tables_before_writing_any_metadata() {
    let (temp, source, generation, init) = fixture(false, Default::default());
    initialize(&source, generation, &init);
    let mut parent = report(&source, source.sitemap_catalog().unwrap());
    parent.pages = vec![PageRecord {
        url: PAGE.into(),
        ..Default::default()
    }];
    let prepared = crate::prepare_selected_recrawl(
        crate::SelectedRecrawlParent {
            job_id: "parent".into(),
            checkpoint_revision: None,
            source_kind: crate::RecrawlSourceKind::ImportedSnapshot,
        },
        &serde_json::to_vec_pretty(&parent).unwrap(),
        &[PAGE.into()],
        None,
    )
    .unwrap();
    let child = CrawlStore::open(temp.path().join("child.sqlite")).unwrap();
    child.connection.lock().unwrap().execute("INSERT INTO sitemap_roots(sequence,row_id,ordinal,data,data_sha256) VALUES(1,'occupied',0,'{}','x')", []).unwrap();
    let before = saved_state(&child);
    assert_error(
        child.initialize_selected_recrawl(&prepared).unwrap_err(),
        "requires a new empty database",
    );
    assert_eq!(saved_state(&child), before);
}
#[test]
fn selected_inheritance_preserves_rows_and_rejects_missing_marker_downgrade() {
    let (temp, source, generation, init) = fixture(false, Default::default());
    initialize(&source, generation, &init);
    ordinary_commit(&source, generation).unwrap();
    let catalog = source.sitemap_catalog().unwrap().unwrap();
    let mut parent = report(&source, Some(catalog.clone()));
    parent.pages = vec![PageRecord {
        url: PAGE.into(),
        ..Default::default()
    }];
    let prepared = crate::prepare_selected_recrawl(
        crate::SelectedRecrawlParent {
            job_id: "parent".into(),
            checkpoint_revision: None,
            source_kind: crate::RecrawlSourceKind::ImportedSnapshot,
        },
        &serde_json::to_vec_pretty(&parent).unwrap(),
        &[PAGE.into()],
        None,
    )
    .unwrap();
    let child = CrawlStore::open(temp.path().join("child.sqlite")).unwrap();
    child.initialize_selected_recrawl(&prepared).unwrap();
    let inherited = child.sitemap_catalog().unwrap().unwrap();
    assert_eq!(inherited.rows, catalog.rows);
    assert_eq!(
        inherited.summary.rolling_sha256,
        catalog.summary.rolling_sha256
    );
    let basis = inherited.inherited.unwrap();
    assert!(!basis.current_discovery_executed);
    assert_eq!(
        basis.parent_snapshot_sha256,
        prepared.plan.parent.snapshot_sha256
    );
    child.connection.lock().unwrap().execute_batch("DELETE FROM sitemap_locations; DELETE FROM sitemap_entries; DELETE FROM sitemap_roots; DELETE FROM sitemap_documents; DELETE FROM metadata WHERE key IN ('sitemap_ledger','sitemap_inherited_basis');").unwrap();
    assert_error(
        child.sitemap_catalog().unwrap_err(),
        "presence differs from immutable parent",
    );
}
#[test]
fn selected_legacy_parent_keeps_catalog_absent_without_fresh_empty_upgrade() {
    let (temp, source, _generation, _init) = fixture(false, Default::default());
    let mut parent = report(&source, None);
    parent.pages = vec![PageRecord {
        url: PAGE.into(),
        ..Default::default()
    }];
    let prepared = crate::prepare_selected_recrawl(
        crate::SelectedRecrawlParent {
            job_id: "legacy-parent".into(),
            checkpoint_revision: None,
            source_kind: crate::RecrawlSourceKind::ImportedSnapshot,
        },
        &serde_json::to_vec_pretty(&parent).unwrap(),
        &[PAGE.into()],
        None,
    )
    .unwrap();
    let child = CrawlStore::open(temp.path().join("child.sqlite")).unwrap();
    child.initialize_selected_recrawl(&prepared).unwrap();
    assert!(child.sitemap_catalog().unwrap().is_none());
    assert!(child.sitemap_ledger_summary().unwrap().is_none());
}
