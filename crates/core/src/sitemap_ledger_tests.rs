//! Meaningful candidate tests. NOT RUN by this Work author; Root owns native execution.
include!("sitemap_ledger/capacity_tests.rs");
use super::*;

const TASK: &str = "https://example.org/sitemap.xml";
fn empty_connection() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch("PRAGMA foreign_keys=ON;
        CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);
        CREATE TABLE discovery_frontier(kind TEXT NOT NULL,url TEXT NOT NULL,done INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(kind,url));
        CREATE TABLE frontier(url TEXT PRIMARY KEY);
        CREATE TABLE policy_occurrences(sequence INTEGER PRIMARY KEY,occurrence_id TEXT UNIQUE NOT NULL,generation INTEGER NOT NULL,data TEXT NOT NULL);
        CREATE TABLE recrawl_base(id INTEGER PRIMARY KEY CHECK(id=1),snapshot BLOB NOT NULL);
        CREATE TABLE policy_shadow(id INTEGER PRIMARY KEY);").unwrap();
    connection.execute_batch(SCHEMA).unwrap();
    save(
        &connection,
        "run_state",
        &crate::CrawlRunState {
            generation: 1,
            started_at: "fixture".into(),
            finished_at: None,
            outcome: None,
        },
    )
    .unwrap();
    connection
}
fn connection(limits: SitemapLedgerLimits) -> Connection {
    let mut connection = empty_connection();
    let tx = connection.transaction().unwrap();
    initialize(&tx, &"a".repeat(64), &limits, false).unwrap();
    tx.commit().unwrap();
    connection
        .execute(
            "INSERT INTO discovery_frontier(kind,url) VALUES('sitemap',?1)",
            [TASK],
        )
        .unwrap();
    connection
}
fn batch(xml: &str) -> SitemapDiscoveryBatch {
    let limits = crate::SitemapParserLimits::default();
    let decoded = crate::decode_sitemap_entity(xml.as_bytes(), &limits).unwrap();
    let observation = crate::observe_sitemap_xml(
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
        observation_id: source_observation_id(SitemapSourceKind::SitemapXml, TASK, 1, &revision)
            .unwrap(),
        kind: SitemapSourceKind::SitemapXml,
        task_url: TASK.into(),
        generation: 1,
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
            post_http_decoding_sha256: Some(digest(xml.as_bytes())),
            retained_post_http_prefix_sha256: Some(digest(xml.as_bytes())),
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
    let mut batch = SitemapDiscoveryBatch::from_parser(document, observation).unwrap();
    let mut scheduled = BTreeSet::new();
    for row in &mut batch.locations {
        if row.admission.decision == SitemapAdmissionDecision::Unbound {
            let url = row.observed.normalized_fetch_url.clone().unwrap();
            row.admission.decision = if scheduled.insert(url.clone()) {
                SitemapAdmissionDecision::Scheduled
            } else {
                SitemapAdmissionDecision::Coalesced
            };
            row.admission.admitted_fetch_url = Some(url);
            if row.observed.role == crate::SitemapLocationRole::ChildSitemap {
                row.referenced_sitemap_task = row.admission.admitted_fetch_url.clone();
            }
        }
    }
    batch
}
fn commit(
    connection: &mut Connection,
    batch: SitemapDiscoveryBatch,
) -> Result<SitemapLedgerSummary> {
    let tx = connection.transaction()?;
    let token = guard_pending_source(&tx, 1, SitemapSourceKind::SitemapXml, TASK)?;
    let state = append_source(&tx, token, batch)?;
    tx.commit()?;
    Ok(state)
}
fn count(connection: &Connection, table: &str) -> i64 {
    connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}
fn pending_done(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT done FROM discovery_frontier WHERE kind='sitemap' AND url=?1",
            [TASK],
            |row| row.get(0),
        )
        .unwrap()
}
fn row_payloads(connection: &Connection) -> Vec<String> {
    let mut query=connection.prepare("SELECT data FROM (SELECT sequence,data FROM sitemap_roots UNION ALL
        SELECT sequence,data FROM sitemap_documents UNION ALL SELECT sequence,data FROM sitemap_entries UNION ALL
        SELECT sequence,data FROM sitemap_locations) ORDER BY sequence").unwrap();
    query
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|row| row.unwrap())
        .collect()
}
// A minimal immutable SQL anchor carrier, not a native CrawlReport attestation.
// Actual store/recrawl validates full parent bytes/plan before this module runs.
fn selected_anchor(connection: &Connection, catalog: &SitemapCatalog) -> String {
    let snapshot = serde_json::to_vec(&serde_json::json!({"sitemap_catalog":catalog})).unwrap();
    let snapshot_sha = digest(&snapshot);
    let plan = crate::SelectedRecrawlPlan {
        engine: crate::SELECTED_RECRAWL_ENGINE.into(),
        parent: crate::RecrawlParentReference {
            job_id: "fixture-parent".into(),
            checkpoint_revision: None,
            source_kind: crate::RecrawlSourceKind::StoredJson,
            snapshot_sha256: snapshot_sha.clone(),
        },
        inherited_config_sha256: catalog.summary.config_sha256.clone(),
        plan_sha256: "f".repeat(64),
        selection: vec![],
        targets: vec![],
        default_max_attempts: None,
    };
    save(
        connection,
        "run_plan",
        &crate::RunPlan::SelectedRecrawl(plan),
    )
    .unwrap();
    connection
        .execute(
            "INSERT INTO recrawl_base(id,snapshot) VALUES(1,?1)",
            [snapshot],
        )
        .unwrap();
    snapshot_sha
}

#[test]
fn duplicate_invalid_missing_locations_are_distinct_durable_facts() {
    let mut connection = connection(SitemapLedgerLimits::default());
    let state=commit(&mut connection,batch("<urlset><url><loc>https://example.org/a</loc></url><url><loc>https://example.org/a</loc></url><url><loc>ftp://example.org/x</loc></url><url/></urlset>")).unwrap();
    assert_eq!(
        (state.document_rows, state.entry_rows, state.location_rows),
        (1, 4, 4)
    );
    assert_eq!(state.incomplete_documents, 0);
    assert!(!state.declared_source_cohort_complete);
    let catalog = load_catalog(&connection).unwrap().unwrap();
    let locations: Vec<_> = catalog
        .rows
        .unwrap()
        .into_iter()
        .filter_map(|row| match row {
            SitemapLedgerRow::Location(row) => Some(row),
            _ => None,
        })
        .collect();
    assert_eq!(
        locations[0].observed.raw_element_xml.as_deref(),
        Some("<loc>https://example.org/a</loc>")
    );
    assert_eq!(
        locations[0].observed.decoded_value,
        locations[1].observed.decoded_value
    );
    assert_eq!(
        (locations[0].observed.ordinal, locations[1].observed.ordinal),
        (1, 2)
    );
    assert_ne!(locations[0].occurrence_id, locations[1].occurrence_id);
    assert_eq!(
        locations[0].admission.decision,
        SitemapAdmissionDecision::Scheduled
    );
    assert_eq!(
        locations[1].admission.decision,
        SitemapAdmissionDecision::Coalesced
    );
    assert_eq!(
        locations[2].admission.decision,
        SitemapAdmissionDecision::InvalidLocation
    );
    assert_eq!(
        locations[3].admission.decision,
        SitemapAdmissionDecision::MissingLocation
    );
    assert!(locations[3].observed.raw_element_xml.is_none());
    assert_eq!(pending_done(&connection), 1);
}
#[test]
fn once_only_guard_rejects_repeat_before_admission_writes() {
    let mut connection = connection(SitemapLedgerLimits::default());
    commit(&mut connection, batch("<urlset/> ")).unwrap();
    let before = load_catalog(&connection).unwrap();
    {
        let tx = connection.transaction().unwrap();
        assert!(guard_pending_source(&tx, 1, SitemapSourceKind::SitemapXml, TASK).is_err());
    }
    assert_eq!(load_catalog(&connection).unwrap(), before);
    assert_eq!(count(&connection, "policy_shadow"), 0);
}
#[test]
fn stale_generation_guard_rejects_pending_task() {
    let mut connection = connection(SitemapLedgerLimits::default());
    {
        let tx = connection.transaction().unwrap();
        assert!(guard_pending_source(&tx, 2, SitemapSourceKind::SitemapXml, TASK).is_err());
    }
    assert_eq!(pending_done(&connection), 0);
    assert_eq!(count(&connection, "sitemap_documents"), 0);
}
#[test]
fn late_sql_failure_rolls_back_policy_frontier_rows_summary_and_source_done() {
    let mut connection = connection(SitemapLedgerLimits::default());
    let before = load_catalog(&connection).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_sitemap_location AFTER INSERT ON sitemap_locations BEGIN SELECT RAISE(ABORT,'injected late failure'); END;").unwrap();
    let result = (|| -> Result<()> {
        let tx = connection.transaction()?;
        let token = guard_pending_source(&tx, 1, SitemapSourceKind::SitemapXml, TASK)?;
        tx.execute("INSERT INTO policy_shadow(id) VALUES(1)", [])?;
        tx.execute(
            "INSERT INTO frontier(url) VALUES('https://example.org/a')",
            [],
        )?;
        append_source(
            &tx,
            token,
            batch("<urlset><url><loc>https://example.org/a</loc></url></urlset>"),
        )?;
        tx.commit()?;
        Ok(())
    })();
    assert!(format!("{:#}", result.unwrap_err()).contains("injected late failure"));
    assert_eq!(load_catalog(&connection).unwrap(), before);
    for table in [
        "policy_shadow",
        "frontier",
        "sitemap_documents",
        "sitemap_entries",
        "sitemap_locations",
    ] {
        assert_eq!(count(&connection, table), 0);
    }
    assert_eq!(pending_done(&connection), 0);
}
#[test]
fn append_savepoint_does_not_leave_half_ledger_if_outer_caller_catches_error() {
    let mut connection = connection(SitemapLedgerLimits::default());
    let before = load_catalog(&connection).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_sitemap_location AFTER INSERT ON sitemap_locations BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    {
        let tx = connection.transaction().unwrap();
        let token = guard_pending_source(&tx, 1, SitemapSourceKind::SitemapXml, TASK).unwrap();
        let failure = append_source(
            &tx,
            token,
            batch("<urlset><url><loc>https://example.org/a</loc></url></urlset>"),
        )
        .unwrap_err();
        assert!(format!("{failure:#}").contains("injected"));
        tx.commit().unwrap();
    }
    assert_eq!(load_catalog(&connection).unwrap(), before);
    assert_eq!(pending_done(&connection), 0);
}
#[test]
fn serialized_catalog_budget_rejects_whole_source_without_marking_done() {
    let limits = SitemapLedgerLimits {
        max_catalog_json_bytes: 2048,
        max_row_json_bytes: 1900,
        max_rows: 100,
    };
    let mut connection = connection(limits);
    let before = load_catalog(&connection).unwrap();
    let failure = commit(
        &mut connection,
        batch("<urlset><url><loc>https://example.org/a</loc><lastmod>raw</lastmod></url></urlset>"),
    )
    .unwrap_err();
    assert!(format!("{failure:#}").contains("budget"));
    assert_eq!(load_catalog(&connection).unwrap(), before);
    assert_eq!(pending_done(&connection), 0);
}

fn calibration_lastmods_xml(ascii: usize, amp_entities: usize) -> String {
    let full_payload =
        crate::SitemapParserLimits::default().max_field_utf8_bytes - "<lastmod></lastmod>".len();
    let mut xml = format!("<urlset><url><loc>{TASK}/target</loc>");
    for _ in 0..10 {
        xml.push_str("<lastmod>");
        xml.push_str(&"a".repeat(full_payload));
        xml.push_str("</lastmod>");
    }
    xml.push_str("<lastmod>");
    xml.push_str(&"a".repeat(ascii));
    xml.push_str(&"&amp;".repeat(amp_entities));
    xml.push_str("</lastmod></url></urlset>");
    xml
}
fn calibration_entry_bytes(xml: &str) -> usize {
    let observed = batch(xml);
    assert!(observed.document.complete());
    assert_eq!(observed.entries.len(), 1);
    assert_eq!(observed.entries[0].observed.lastmod.len(), 11);
    assert_eq!(observed.locations.len(), 1);
    assert_eq!(
        observed.locations[0].admission.decision,
        SitemapAdmissionDecision::Scheduled
    );
    let mut row = SitemapLedgerRow::Entry(observed.entries[0].clone());
    // The actual append order is Document=1, Entry=2, Location=3 for this source.
    row.set_sequence(2);
    serde_json::to_vec(&row).unwrap().len()
}
fn calibration_exact_entry(target: usize) -> (String, usize, usize) {
    // An ASCII byte contributes three retained JSON bytes; an XML &amp; entity
    // contributes eleven. Trying three residues and binary search keeps this
    // calibration bounded while measuring actual parser/serde offset changes.
    for amp_entities in 0..3 {
        let mut low = 0;
        let mut high = 8000;
        while low <= high {
            let middle = low + (high - low) / 2;
            let xml = calibration_lastmods_xml(middle, amp_entities);
            match calibration_entry_bytes(&xml).cmp(&target) {
                std::cmp::Ordering::Equal => return (xml, middle, amp_entities),
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => {
                    if middle == 0 {
                        break;
                    }
                    high = middle - 1;
                }
            }
        }
    }
    panic!("No parser/serde candidate found for physical row bytes {target}");
}
/// Actual parser/serde and in-memory production writer control at the default
/// physical row cap. HTTP App/native rollback acceptance remains separate.
#[test]
fn budget_calibration_repeated_lastmods_hit_default_row_cap_and_plus_one() {
    let cap = SitemapLedgerLimits::default().max_row_json_bytes;
    assert_eq!(cap, 262_144);
    for target in [cap - 1, cap, cap + 1] {
        let (xml, ascii, amp_entities) = calibration_exact_entry(target);
        assert_eq!(calibration_entry_bytes(&xml), target);
        assert!(xml.len() < crate::SitemapParserLimits::default().max_xml_bytes);
        let observed = batch(&xml);
        for field in &observed.entries[0].observed.lastmod {
            assert!(
                field.raw_element_xml.len()
                    <= crate::SitemapParserLimits::default().max_field_utf8_bytes
            );
            assert!(
                field.raw_inner_xml.len()
                    <= crate::SitemapParserLimits::default().max_field_utf8_bytes
            );
            assert!(
                field.decoded_value.len()
                    <= crate::SitemapParserLimits::default().max_field_utf8_bytes
            );
        }
        let mut connection = connection(SitemapLedgerLimits::default());
        let before = load_catalog(&connection).unwrap();
        if target <= cap {
            commit(&mut connection, observed).unwrap();
            let data: String = connection
                .query_row("SELECT data FROM sitemap_entries", [], |row| row.get(0))
                .unwrap();
            assert_eq!(data.len(), target);
            assert_eq!(
                serde_json::from_str::<SitemapLedgerRow>(&data)
                    .unwrap()
                    .sequence(),
                2
            );
            assert_eq!(pending_done(&connection), 1);
            assert_eq!(count(&connection, "sitemap_documents"), 1);
            assert_eq!(count(&connection, "sitemap_locations"), 1);
        } else {
            let error = commit(&mut connection, observed).unwrap_err();
            assert!(format!("{error:#}").contains("sitemap_catalog_json_budget_exceeded"));
            assert_eq!(load_catalog(&connection).unwrap(), before);
            assert_eq!(pending_done(&connection), 0);
            for table in TABLES {
                assert_eq!(count(&connection, table), 0);
            }
        }
        eprintln!(
            "BUDGET_CALIBRATION_ENTRY target={target} xml_bytes={} lastmod_count=11 full_ascii_fields=10 tail_ascii={ascii} tail_amp_entities={amp_entities}",
            xml.len()
        );
    }
}
#[test]
fn incomplete_parser_prefix_is_durable_and_admits_nothing() {
    let mut connection = connection(SitemapLedgerLimits::default());
    let prefix = batch("<urlset><url><loc>https://example.org/a</loc></url>");
    assert!(!prefix.document.complete());
    assert_eq!(
        prefix.locations[0].admission.decision,
        SitemapAdmissionDecision::IncompleteSource
    );
    let state = commit(&mut connection, prefix).unwrap();
    assert_eq!(state.incomplete_documents, 1);
    assert_eq!(state.location_rows, 1);
    assert_eq!(pending_done(&connection), 1);
    assert!(!state.declared_source_cohort_complete);
}
#[test]
fn complete_xml_in_incomplete_http_prefix_does_not_admit() {
    let mut connection = connection(SitemapLedgerLimits::default());
    let mut value = batch("<urlset><url><loc>https://example.org/a</loc></url></urlset>");
    let transport = value.document.transport.as_mut().unwrap();
    transport.body_complete = false;
    transport.post_http_decoding_sha256 = None;
    transport.failure_code = Some("sitemap_body_budget".into());
    for row in &mut value.locations {
        row.admission = SitemapAdmissionBinding::observed(&row.observed, value.document.complete());
        row.referenced_sitemap_task = None;
    }
    assert!(value.document.parser.as_ref().unwrap().complete);
    assert!(!value.document.complete());
    assert_eq!(
        commit(&mut connection, value).unwrap().incomplete_documents,
        1
    );
}
#[test]
fn metadata_engine_absence_keeps_legacy_unknown_despite_created_tables() {
    let mut connection = empty_connection();
    connection
        .execute(
            "INSERT INTO metadata(key,value) VALUES('discovery_initialized','true')",
            [],
        )
        .unwrap();
    assert!(load_catalog(&connection).unwrap().is_none());
    {
        let tx = connection.transaction().unwrap();
        assert!(initialize(&tx, &"a".repeat(64), &SitemapLedgerLimits::default(), false).is_err());
    }
    {
        let tx = connection.transaction().unwrap();
        let state =
            initialize(&tx, &"a".repeat(64), &SitemapLedgerLimits::default(), true).unwrap();
        assert!(state.legacy_history_unknown);
        assert!(!state.declared_source_cohort_complete);
        tx.commit().unwrap();
    }
}
#[test]
fn missing_engine_marker_with_nonempty_tables_is_corruption_not_empty_legacy() {
    let connection = empty_connection();
    connection.execute("INSERT INTO sitemap_documents(sequence,row_id,source_kind,task_url,generation,source_revision,data,data_sha256) VALUES(1,'id','sitemap',?1,1,'r','{}','bad')",[TASK]).unwrap();
    assert!(load_catalog(&connection).is_err());
}
#[test]
fn row_hash_and_sql_index_tampering_are_rejected() {
    let mut connection = connection(SitemapLedgerLimits::default());
    commit(&mut connection, batch("<urlset/> ")).unwrap();
    connection
        .execute(
            "UPDATE sitemap_documents SET data_sha256=?1",
            ["0".repeat(64)],
        )
        .unwrap();
    assert!(load_catalog(&connection).is_err());
    let data: String = connection
        .query_row("SELECT data FROM sitemap_documents", [], |row| row.get(0))
        .unwrap();
    connection
        .execute(
            "UPDATE sitemap_documents SET data_sha256=?1,task_url='https://example.org/forged.xml'",
            [digest(&data)],
        )
        .unwrap();
    assert!(load_catalog(&connection).is_err());
}
#[test]
fn global_sequence_and_rolling_digest_tampering_are_rejected() {
    let mut connection = connection(SitemapLedgerLimits::default());
    commit(
        &mut connection,
        batch("<urlset><url><loc>https://example.org/a</loc></url></urlset>"),
    )
    .unwrap();
    let mut catalog = load_catalog(&connection).unwrap().unwrap();
    catalog.summary.rolling_sha256 = "0".repeat(64);
    assert!(validate_catalog(&catalog).is_err());
    let mut catalog = load_catalog(&connection).unwrap().unwrap();
    catalog.rows.as_mut().unwrap()[1].set_sequence(1);
    assert!(validate_catalog(&catalog).is_err());
    connection
        .execute("UPDATE sitemap_locations SET sequence=1", [])
        .unwrap();
    assert!(load_catalog(&connection).is_err());
}
#[test]
fn selected_inheritance_preserves_exact_rows_ids_generations_sequence_and_rolling_hash() {
    let mut parent = connection(SitemapLedgerLimits::default());
    commit(&mut parent,batch("<sitemapindex><sitemap><loc>https://example.org/child.xml</loc></sitemap></sitemapindex>")).unwrap();
    let catalog = load_catalog(&parent).unwrap().unwrap();
    let original = row_payloads(&parent);
    let original_summary = summary(&parent).unwrap();
    let mut selected = empty_connection();
    let snapshot_sha = selected_anchor(&selected, &catalog);
    save(
        &selected,
        "run_state",
        &crate::CrawlRunState {
            generation: 99,
            started_at: "selected".into(),
            finished_at: None,
            outcome: None,
        },
    )
    .unwrap();
    {
        let tx = selected.transaction().unwrap();
        inherit(&tx, &catalog, &snapshot_sha).unwrap();
        tx.commit().unwrap();
    }
    let inherited = load_catalog(&selected).unwrap().unwrap();
    assert_eq!(row_payloads(&selected), original);
    assert_eq!(inherited.rows, catalog.rows);
    assert_eq!(
        inherited.summary.rolling_sha256,
        catalog.summary.rolling_sha256
    );
    assert_eq!(
        inherited.summary.last_sequence,
        catalog.summary.last_sequence
    );
    let basis = inherited.inherited.unwrap();
    assert!(!basis.current_discovery_executed);
    assert_eq!(
        basis.parent_catalog_sha256,
        digest(serde_json::to_vec(&catalog).unwrap())
    );
    assert_eq!(summary(&parent).unwrap(), original_summary);
    assert_eq!(row_payloads(&parent), original);
    let generations: Vec<_> = inherited
        .rows
        .unwrap()
        .into_iter()
        .filter_map(|row| match row {
            SitemapLedgerRow::Document(row) => Some(row.generation),
            _ => None,
        })
        .collect();
    assert_eq!(generations, vec![1]);
}
#[test]
fn inherited_envelope_overflow_rejects_atomically() {
    let mut parent = connection(SitemapLedgerLimits::default());
    commit(&mut parent, batch("<urlset/> ")).unwrap();
    let mut catalog = load_catalog(&parent).unwrap().unwrap();
    catalog.summary.limits.max_row_json_bytes = catalog
        .rows
        .as_ref()
        .unwrap()
        .iter()
        .map(|row| serde_json::to_vec(row).unwrap().len())
        .max()
        .unwrap();
    refresh_size(&mut catalog).unwrap();
    for _ in 0..5 {
        catalog.summary.limits.max_catalog_json_bytes = catalog.summary.catalog_json_bytes as usize;
        refresh_size(&mut catalog).unwrap();
    }
    validate_catalog(&catalog).unwrap();
    let mut selected = empty_connection();
    let snapshot_sha = selected_anchor(&selected, &catalog);
    {
        let tx = selected.transaction().unwrap();
        let failure = inherit(&tx, &catalog, &snapshot_sha).unwrap_err();
        assert!(format!("{failure:#}").contains("budget"));
    }
    assert!(load_catalog(&selected).unwrap().is_none());
    assert_eq!(count(&selected, "sitemap_documents"), 0);
}
#[test]
fn cursor_identity_rejects_changed_revision_and_requires_continuation_identity() {
    let mut connection = connection(SitemapLedgerLimits::default());
    commit(
        &mut connection,
        batch("<urlset><url><loc>https://example.org/a</loc></url></urlset>"),
    )
    .unwrap();
    let first = page(&connection, 0, 1, None, None).unwrap();
    assert!(first.has_more);
    let state = first.ledger.unwrap();
    let next = first.next_after_sequence.unwrap();
    assert!(page(&connection, next, 1, None, None).is_err());
    assert!(
        page(
            &connection,
            next,
            1,
            Some(state.revision + 1),
            Some(&state.rolling_sha256)
        )
        .is_err()
    );
    assert!(
        page(
            &connection,
            next,
            1,
            Some(state.revision),
            Some(&state.rolling_sha256)
        )
        .unwrap()
        .rows
        .len()
            == 1
    );
}
fn final_policy(
    location: &crate::SitemapLocationOccurrence,
    revision: &str,
) -> crate::PolicyOccurrence {
    let original = location.normalized_fetch_url.clone();
    crate::PolicyOccurrence {
        source_revision: revision.into(),
        source_ordinal: location.ordinal,
        input_basis: crate::PolicyInputBasis::ParserObservedXml,
        raw_value_observed: true,
        task_role: crate::PolicyTaskRole::PageDocument,
        scheduled_fetch_url: Some("https://example.org/alias".into()),
        coalesced_task_occurrence_id: None,
        admission_reason: "scheduled".into(),
        input_error: None,
        decision: crate::CandidateDecision {
            occurrence_id: "policy-original-loc".into(),
            source_kind: crate::CandidateSourceKind::SitemapLocation,
            source_url: Some(TASK.into()),
            source_location: None,
            raw_value: location.decoded_value.clone().unwrap(),
            resolved_discovered_url: original.clone(),
            normalized_discovered_url: original,
            fetch_candidate_url: Some("https://example.org/alias".into()),
            fragment: None,
            policy_steps: vec![],
            decision: crate::CrawlPolicyDecisionKind::FetchAndTraverse,
            reason_code: None,
            error: None,
            matched_rule_ids: vec![],
            host_internal: true,
            folder_internal: true,
            traverse: true,
            coalesced_into: None,
            policy_engine: "test-policy".into(),
            policy_sha256: "f".repeat(64),
            warnings: vec![],
            policy_complete: true,
        },
    }
}
fn insert_policy(tx: &Transaction<'_>, row: &crate::PolicyOccurrence) {
    let stored = crate::StoredPolicyOccurrence {
        sequence: 1,
        generation: 1,
        basis: crate::PolicyObservationBasis::Current,
        occurrence: row.clone(),
    };
    tx.execute(
        "INSERT INTO policy_occurrences(sequence,occurrence_id,generation,data) VALUES(1,?1,1,?2)",
        params![
            row.decision.occurrence_id,
            serde_json::to_string(&stored).unwrap()
        ],
    )
    .unwrap();
}
#[test]
fn final_policy_binding_requires_real_committed_row_and_keeps_original_vs_alias() {
    let mut connection = connection(SitemapLedgerLimits::default());
    let mut value = batch(
        "<urlset><url/><url><loc>  https://example.org/original?a=1&amp;b=2  </loc></url></urlset>",
    );
    let policy = final_policy(
        &value.locations[1].observed,
        &value.document.source_revision,
    );
    {
        let tx = connection.transaction().unwrap();
        let token = guard_pending_source(&tx, 1, SitemapSourceKind::SitemapXml, TASK).unwrap();
        assert!(bind_final_policy(&tx, &mut value, std::slice::from_ref(&policy)).is_err());
        insert_policy(&tx, &policy);
        bind_final_policy(&tx, &mut value, &[policy]).unwrap();
        assert_eq!(
            value.locations[1].admission.membership_url.as_deref(),
            Some("https://example.org/original?a=1&b=2")
        );
        assert_eq!(
            value.locations[1].admission.admitted_fetch_url.as_deref(),
            Some("https://example.org/alias")
        );
        append_source(&tx, token, value).unwrap();
        tx.commit().unwrap();
    }
    assert!(load_catalog(&connection).unwrap().is_some());
}
#[test]
fn selected_inherited_policy_references_do_not_fabricate_current_policy_rows() {
    let mut parent = connection(SitemapLedgerLimits::default());
    let mut value = batch("<urlset><url><loc>https://example.org/original</loc></url></urlset>");
    let policy = final_policy(
        &value.locations[0].observed,
        &value.document.source_revision,
    );
    {
        let tx = parent.transaction().unwrap();
        let token = guard_pending_source(&tx, 1, SitemapSourceKind::SitemapXml, TASK).unwrap();
        insert_policy(&tx, &policy);
        bind_final_policy(&tx, &mut value, &[policy]).unwrap();
        append_source(&tx, token, value).unwrap();
        tx.commit().unwrap();
    }
    let catalog = load_catalog(&parent).unwrap().unwrap();
    let mut selected = empty_connection();
    let snapshot_sha = selected_anchor(&selected, &catalog);
    {
        let tx = selected.transaction().unwrap();
        inherit(&tx, &catalog, &snapshot_sha).unwrap();
        tx.commit().unwrap();
    }
    assert_eq!(count(&selected, "policy_occurrences"), 0);
    let inherited = load_catalog(&selected).unwrap().unwrap();
    assert!(inherited.inherited.is_some());
    assert_eq!(inherited.rows, catalog.rows);
}
#[test]
fn compacted_or_trimmed_compatibility_policy_inputs_are_rejected() {
    let mut connection = connection(SitemapLedgerLimits::default());
    let mut value =
        batch("<urlset><url/><url><loc>  https://example.org/original  </loc></url></urlset>");
    let mut policy = final_policy(
        &value.locations[1].observed,
        &value.document.source_revision,
    );
    policy.source_ordinal = 0;
    {
        let tx = connection.transaction().unwrap();
        insert_policy(&tx, &policy);
        assert!(bind_final_policy(&tx, &mut value, &[policy]).is_err());
    }
    let mut policy = final_policy(
        &value.locations[1].observed,
        &value.document.source_revision,
    );
    policy.decision.raw_value = policy.decision.raw_value.trim().into();
    {
        let tx = connection.transaction().unwrap();
        insert_policy(&tx, &policy);
        assert!(bind_final_policy(&tx, &mut value, &[policy]).is_err());
    }
    assert_eq!(pending_done(&connection), 0);
}
#[test]
fn explicit_duplicate_root_declarations_keep_occurrences_while_tasks_coalesce() {
    let mut connection = connection(SitemapLedgerLimits::default());
    save(
        &connection,
        "config",
        &serde_json::json!({"explicit_sitemap_inputs":[TASK,TASK]}),
    )
    .unwrap();
    let roots = (0..2)
        .map(|ordinal| SitemapRootDeclaration {
            sequence: 0,
            declaration_id: root_declaration_id(
                &SitemapRootKind::ExplicitConfig,
                &"a".repeat(64),
                ordinal,
            ),
            kind: SitemapRootKind::ExplicitConfig,
            source_observation_id: None,
            source_ordinal: ordinal,
            raw_line: None,
            raw_value: TASK.into(),
            normalized_document_url: Some(TASK.into()),
            robots_evidence: None,
            admission: SitemapAdmissionBinding {
                decision: if ordinal == 0 {
                    SitemapAdmissionDecision::Scheduled
                } else {
                    SitemapAdmissionDecision::Coalesced
                },
                membership_url: Some(TASK.into()),
                admitted_fetch_url: Some(TASK.into()),
                coalesced_policy_occurrence_id: None,
                policy_occurrence_id: None,
                policy_source_revision: None,
            },
        })
        .collect();
    {
        let tx = connection.transaction().unwrap();
        let state = append_roots(&tx, 1, roots).unwrap();
        assert_eq!(state.root_rows, 2);
        tx.commit().unwrap();
    }
    let catalog = load_catalog(&connection).unwrap().unwrap();
    assert_eq!(catalog.rows.unwrap().len(), 2);
    assert_eq!(count(&connection, "discovery_frontier"), 1);
    assert_eq!(pending_done(&connection), 0);
}
#[test]
fn robots_empty_relative_declarations_keep_legacy_alias_separate_from_original_identity() {
    let mut connection = connection(SitemapLedgerLimits::default());
    let task = "https://example.org/";
    let requested = "https://example.org/robots.txt";
    let body = "Sitemap: /child.xml\nSitemap:\n";
    let revision = digest(body.as_bytes());
    connection
        .execute(
            "INSERT INTO discovery_frontier(kind,url) VALUES('robots',?1)",
            [task],
        )
        .unwrap();
    let id =
        source_observation_id(SitemapSourceKind::RobotsDirectives, task, 1, &revision).unwrap();
    let document = SitemapSourceDocument {
        sequence: 0,
        observation_id: id.clone(),
        kind: SitemapSourceKind::RobotsDirectives,
        task_url: task.into(),
        generation: 1,
        source_revision: revision,
        transport: Some(SitemapTransportFacts {
            request_started: true,
            requested_url: Some(requested.into()),
            final_url: Some(requested.into()),
            initial_status: Some(200),
            final_status: Some(200),
            blocked_by_robots: false,
            body_complete: true,
            post_http_decoding_bytes: body.len() as u64,
            post_http_decoding_sha256: Some(digest(body)),
            retained_post_http_prefix_sha256: Some(digest(body)),
            failure_code: None,
        }),
        entity_decode: None,
        parser: None,
        declaration_scan_complete: true,
        declaration_utf8_lossy: Some(false),
        declaration_count: Some(2),
        retained_rows_complete: true,
        failure_code: None,
    };
    let lines = [
        (
            "Sitemap: /child.xml\n",
            " /child.xml",
            "https://example.org/child.xml",
        ),
        ("Sitemap:\n", "", "https://example.org/base/"),
    ];
    let mut offset = 0;
    let roots = lines
        .into_iter()
        .enumerate()
        .map(|(ordinal, (line, value, alias))| {
            let start = offset;
            offset += line.len();
            let end = offset;
            SitemapRootDeclaration {
                sequence: 0,
                declaration_id: root_declaration_id(
                    &SitemapRootKind::RobotsDirective,
                    &id,
                    ordinal as u64,
                ),
                kind: SitemapRootKind::RobotsDirective,
                source_observation_id: Some(id.clone()),
                source_ordinal: ordinal as u64,
                raw_line: Some(line.into()),
                raw_value: value.into(),
                normalized_document_url: None,
                robots_evidence: Some(SitemapRobotsDeclarationFacts {
                    line_ordinal: ordinal + 1,
                    byte_start: start,
                    byte_end: end,
                    line_complete: true,
                    utf8_lossy: false,
                    legacy_candidate_url: Some(alias.into()),
                }),
                admission: SitemapAdmissionBinding {
                    decision: SitemapAdmissionDecision::InvalidLocation,
                    membership_url: None,
                    admitted_fetch_url: None,
                    coalesced_policy_occurrence_id: None,
                    policy_occurrence_id: None,
                    policy_source_revision: None,
                },
            }
        })
        .collect();
    {
        let tx = connection.transaction().unwrap();
        let token =
            guard_pending_source(&tx, 1, SitemapSourceKind::RobotsDirectives, task).unwrap();
        append_source(
            &tx,
            token,
            SitemapDiscoveryBatch {
                document,
                roots,
                entries: vec![],
                locations: vec![],
            },
        )
        .unwrap();
        tx.commit().unwrap();
    }
    let catalog = load_catalog(&connection).unwrap().unwrap();
    let roots: Vec<_> = catalog
        .rows
        .unwrap()
        .into_iter()
        .filter_map(|row| match row {
            SitemapLedgerRow::Root(row) => Some(row),
            _ => None,
        })
        .collect();
    assert_eq!(roots[1].raw_value, "");
    assert!(roots[1].normalized_document_url.is_none());
    assert!(roots[1].admission.membership_url.is_none());
    assert_eq!(
        roots[1]
            .robots_evidence
            .as_ref()
            .unwrap()
            .legacy_candidate_url
            .as_deref(),
        Some("https://example.org/base/")
    );
    assert!(roots[1].admission.admitted_fetch_url.is_none());
    assert_eq!(
        roots[1].admission.decision,
        SitemapAdmissionDecision::InvalidLocation
    );
}

#[test]
fn original_one_based_parser_ordinals_survive_empty_and_missing_loc_prefixes() {
    let mut connection = connection(SitemapLedgerLimits::default());
    let value = batch(
        "<urlset><url/><url><loc></loc></url><url><loc>https://example.org/a</loc></url></urlset>",
    );
    assert_eq!(
        value
            .entries
            .iter()
            .map(|row| row.observed.ordinal)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        value
            .locations
            .iter()
            .map(|row| row.observed.ordinal)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert!(value.locations[0].observed.loc_ordinal.is_none());
    let expected = value
        .locations
        .iter()
        .map(|row| row.observed.clone())
        .collect::<Vec<_>>();
    commit(&mut connection, value).unwrap();
    let actual = load_catalog(&connection)
        .unwrap()
        .unwrap()
        .rows
        .unwrap()
        .into_iter()
        .filter_map(|row| match row {
            SitemapLedgerRow::Location(row) => Some(row.observed),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn original_zero_based_relabel_is_rejected_with_named_ordinal_error() {
    let mut connection = connection(SitemapLedgerLimits::default());
    let mut value = batch("<urlset><url><loc>https://example.org/a</loc></url></urlset>");
    let id = value.document.observation_id.clone();
    value.entries[0].observed.ordinal = 0;
    value.entries[0].observed.loc_occurrence_ordinals = vec![0];
    value.entries[0].entry_id = identity("entry", &id, 0);
    value.locations[0].observed.ordinal = 0;
    value.locations[0].observed.entry_ordinal = 0;
    value.locations[0].occurrence_id = identity("location", &id, 0);
    let failure = commit(&mut connection, value).unwrap_err();
    assert!(format!("{failure:#}").contains("Parser entry/Loc ordinal gaps or reorder"));
    assert_eq!(pending_done(&connection), 0);
    assert_eq!(count(&connection, "sitemap_documents"), 0);
}

#[test]
fn robots_frontier_root_and_actual_request_url_are_distinct_and_verified() {
    for task in ["http://example.org:8080/", "https://[::1]:8443/"] {
        let requested = format!(
            "{}/robots.txt",
            crate::normalize_url(task, None)
                .unwrap()
                .origin()
                .ascii_serialization()
        );
        let revision = digest([]);
        let mut document = SitemapSourceDocument {
            sequence: 0,
            observation_id: source_observation_id(
                SitemapSourceKind::RobotsDirectives,
                task,
                1,
                &revision,
            )
            .unwrap(),
            kind: SitemapSourceKind::RobotsDirectives,
            task_url: task.into(),
            generation: 1,
            source_revision: revision,
            transport: Some(SitemapTransportFacts {
                request_started: true,
                requested_url: Some(requested.clone()),
                final_url: Some(requested),
                initial_status: Some(200),
                final_status: Some(200),
                blocked_by_robots: false,
                body_complete: true,
                post_http_decoding_bytes: 0,
                post_http_decoding_sha256: Some(digest([])),
                retained_post_http_prefix_sha256: Some(digest([])),
                failure_code: None,
            }),
            entity_decode: None,
            parser: None,
            declaration_scan_complete: true,
            declaration_utf8_lossy: Some(false),
            declaration_count: Some(0),
            retained_rows_complete: true,
            failure_code: None,
        };
        assert!(document.complete());
        validate_document(&document).unwrap();
        document.transport.as_mut().unwrap().requested_url = Some(task.into());
        assert!(!document.complete());
        let failure = validate_document(&document).unwrap_err();
        assert!(format!("{failure:#}").contains("source-kind request identity"));
    }
}

#[test]
fn inherited_same_length_parent_sha_swaps_are_rejected_against_immutable_sql_anchor() {
    let mut parent = connection(SitemapLedgerLimits::default());
    commit(&mut parent, batch("<urlset/> ")).unwrap();
    let catalog = load_catalog(&parent).unwrap().unwrap();
    for field in ["parent_snapshot_sha256", "parent_catalog_sha256"] {
        let mut selected = empty_connection();
        let snapshot_sha = selected_anchor(&selected, &catalog);
        {
            let tx = selected.transaction().unwrap();
            inherit(&tx, &catalog, &snapshot_sha).unwrap();
            tx.commit().unwrap();
        }
        let mut basis: serde_json::Value =
            serde_json::from_str(&metadata_text(&selected, INHERITED_KEY).unwrap().unwrap())
                .unwrap();
        basis[field] = serde_json::Value::String("0".repeat(64));
        save(&selected, INHERITED_KEY, &basis).unwrap();
        let failure = load_catalog(&selected).unwrap_err();
        let expected = if field == "parent_snapshot_sha256" {
            "selected run plan"
        } else {
            "parent catalog digest changed"
        };
        assert!(format!("{failure:#}").contains(expected));
    }
}

#[test]
fn inherited_parent_blob_and_run_plan_mutations_are_rejected_without_rewriting_rows() {
    let mut parent = connection(SitemapLedgerLimits::default());
    commit(&mut parent, batch("<urlset/> ")).unwrap();
    let catalog = load_catalog(&parent).unwrap().unwrap();
    for change_blob in [true, false] {
        let mut selected = empty_connection();
        let snapshot_sha = selected_anchor(&selected, &catalog);
        {
            let tx = selected.transaction().unwrap();
            inherit(&tx, &catalog, &snapshot_sha).unwrap();
            tx.commit().unwrap();
        }
        let original = row_payloads(&selected);
        if change_blob {
            selected
                .execute(
                    "UPDATE recrawl_base SET snapshot=CAST(CAST(snapshot AS TEXT)||' ' AS BLOB)",
                    [],
                )
                .unwrap();
        } else {
            save(&selected, "run_plan", &crate::RunPlan::FullCrawl).unwrap();
        }
        let failure = load_catalog(&selected).unwrap_err();
        assert!(format!("{failure:#}").contains(if change_blob {
            "snapshot bytes changed"
        } else {
            "requires selected run plan"
        }));
        assert_eq!(row_payloads(&selected), original);
    }
}

#[test]
fn inherited_catalog_requires_actual_parent_anchor_before_any_row_insertion() {
    let mut parent = connection(SitemapLedgerLimits::default());
    commit(&mut parent, batch("<urlset/> ")).unwrap();
    let catalog = load_catalog(&parent).unwrap().unwrap();
    let mut selected = empty_connection();
    {
        let tx = selected.transaction().unwrap();
        let failure = inherit(&tx, &catalog, &"b".repeat(64)).unwrap_err();
        assert!(format!("{failure:#}").contains("lacks immutable selected run plan"));
    }
    assert_eq!(count(&selected, "sitemap_documents"), 0);
    assert!(load_catalog(&selected).unwrap().is_none());
}

#[test]
fn final_policy_sql_sequence_and_generation_indexes_are_authoritative() {
    for column in ["sequence", "generation"] {
        let mut connection = connection(SitemapLedgerLimits::default());
        let mut value =
            batch("<urlset><url><loc>https://example.org/original</loc></url></urlset>");
        let policy = final_policy(
            &value.locations[0].observed,
            &value.document.source_revision,
        );
        {
            let tx = connection.transaction().unwrap();
            let token = guard_pending_source(&tx, 1, SitemapSourceKind::SitemapXml, TASK).unwrap();
            insert_policy(&tx, &policy);
            bind_final_policy(&tx, &mut value, &[policy]).unwrap();
            append_source(&tx, token, value).unwrap();
            tx.commit().unwrap();
        }
        connection
            .execute(&format!("UPDATE policy_occurrences SET {column}=2"), [])
            .unwrap();
        let failure = load_catalog(&connection).unwrap_err();
        assert!(
            format!("{failure:#}")
                .contains("Policy SQL identity/sequence/generation differs from payload")
        );
    }
}

#[test]
fn final_root_binding_rejects_page_admission_even_when_scheduled_alias_matches() {
    for wrong_role in [true, false] {
        let mut connection = connection(SitemapLedgerLimits::default());
        let value = batch("<urlset><url><loc>https://example.org/sitemap.xml</loc></url></urlset>");
        let mut policy = final_policy(
            &value.locations[0].observed,
            &value.document.source_revision,
        );
        policy.scheduled_fetch_url = Some(TASK.into());
        policy.decision.fetch_candidate_url = Some(TASK.into());
        policy.decision.raw_value = TASK.into();
        policy.input_basis = crate::PolicyInputBasis::DeclaredInput;
        policy.decision.source_url = None;
        policy.source_ordinal = 0;
        policy.source_revision = format!(
            "initial:{}:explicit:sitemap_document",
            policy.decision.policy_sha256
        );
        save(
            &connection,
            "config",
            &serde_json::json!({"explicit_sitemap_inputs":[TASK]}),
        )
        .unwrap();
        policy.task_role = if wrong_role {
            crate::PolicyTaskRole::PageDocument
        } else {
            crate::PolicyTaskRole::SitemapDocument
        };
        policy.decision.source_kind = if wrong_role {
            crate::CandidateSourceKind::Seed
        } else {
            crate::CandidateSourceKind::SitemapDocument
        };
        let root = SitemapRootDeclaration {
            sequence: 0,
            declaration_id: root_declaration_id(
                &SitemapRootKind::ExplicitConfig,
                &"a".repeat(64),
                0,
            ),
            kind: SitemapRootKind::ExplicitConfig,
            source_observation_id: None,
            source_ordinal: 0,
            raw_line: None,
            raw_value: TASK.into(),
            normalized_document_url: Some(TASK.into()),
            robots_evidence: None,
            admission: SitemapAdmissionBinding {
                decision: SitemapAdmissionDecision::Scheduled,
                membership_url: Some(TASK.into()),
                admitted_fetch_url: Some(TASK.into()),
                coalesced_policy_occurrence_id: None,
                policy_occurrence_id: Some(policy.decision.occurrence_id.clone()),
                policy_source_revision: Some(policy.source_revision.clone()),
            },
        };
        {
            let tx = connection.transaction().unwrap();
            insert_policy(&tx, &policy);
            let result = append_roots(&tx, 1, vec![root]);
            if wrong_role {
                let failure = result.unwrap_err();
                assert!(format!("{failure:#}").contains("not a sitemap protocol admission"));
            } else {
                result.unwrap();
                tx.commit().unwrap();
            }
        }
        assert_eq!(
            count(&connection, "sitemap_roots"),
            usize::from(!wrong_role) as i64
        );
    }
}

// v3 coordinates below use explicit, bounded SQL config carriers. They exercise
// this ledger's additional SQL projection, not Core full CrawlConfig validation
// or a genuine imported/native crawl. Root's actual Store gate remains separate.
fn declared_policy_root(
    kind: SitemapRootKind,
    ordinal: u64,
    raw: &str,
    revision: &str,
    policy_ordinal: usize,
) -> (SitemapRootDeclaration, crate::PolicyOccurrence) {
    let value = batch("<urlset><url><loc>https://example.org/sitemap.xml</loc></url></urlset>");
    let mut policy = final_policy(&value.locations[0].observed, revision);
    policy.source_ordinal = policy_ordinal;
    policy.input_basis = crate::PolicyInputBasis::DeclaredInput;
    policy.raw_value_observed = true;
    policy.task_role = crate::PolicyTaskRole::SitemapDocument;
    policy.decision.source_kind = crate::CandidateSourceKind::SitemapDocument;
    policy.decision.policy_sha256 = "a".repeat(64);
    policy.decision.raw_value = raw.into();
    policy.decision.source_url = None;
    policy.decision.occurrence_id =
        digest(serde_json::to_vec(&(revision, policy_ordinal, raw)).unwrap());
    policy.scheduled_fetch_url = Some(TASK.into());
    policy.decision.fetch_candidate_url = Some(TASK.into());
    let root = SitemapRootDeclaration {
        sequence: 0,
        declaration_id: root_declaration_id(&kind, &"a".repeat(64), ordinal),
        kind,
        source_observation_id: None,
        source_ordinal: ordinal,
        raw_line: None,
        raw_value: raw.into(),
        normalized_document_url: Some(TASK.into()),
        robots_evidence: None,
        admission: SitemapAdmissionBinding {
            decision: SitemapAdmissionDecision::Scheduled,
            membership_url: Some(TASK.into()),
            admitted_fetch_url: Some(TASK.into()),
            coalesced_policy_occurrence_id: None,
            policy_occurrence_id: Some(policy.decision.occurrence_id.clone()),
            policy_source_revision: Some(revision.into()),
        },
    };
    (root, policy)
}
fn append_declared_root(
    connection: &mut Connection,
    root: SitemapRootDeclaration,
    policy: &crate::PolicyOccurrence,
) -> Result<()> {
    let tx = connection.transaction()?;
    insert_policy(&tx, policy);
    append_roots(&tx, 1, vec![root])?;
    tx.commit()?;
    Ok(())
}
fn named_failure(result: Result<()>, needle: &str) {
    let failure = result.unwrap_err();
    assert!(
        format!("{failure:#}").contains(needle),
        "Expected {needle:?}, got {failure:#}"
    );
}
#[test]
fn explicit_original_coordinates_accept_exact_raw_ordinal_revision_and_large_config() {
    let mut connection = connection(Default::default());
    let revision = format!("initial:{}:explicit:sitemap_document", "a".repeat(64));
    save(&connection,"config",&serde_json::json!({"explicit_sitemap_inputs":[TASK],"irrelevant_large_value":"x".repeat(100_000)})).unwrap();
    let (root, policy) =
        declared_policy_root(SitemapRootKind::ExplicitConfig, 0, TASK, &revision, 0);
    append_declared_root(&mut connection, root, &policy).unwrap();
    let catalog = load_catalog(&connection).unwrap().unwrap();
    assert_eq!(catalog.summary.root_rows, 1);
    assert!(!catalog.summary.declared_source_cohort_complete);
}
#[test]
fn explicit_missing_or_null_original_capture_rejects_policy_provenance() {
    for capture in [
        serde_json::json!({"sitemap_urls":[TASK]}),
        serde_json::json!({"explicit_sitemap_inputs":null,"sitemap_urls":[TASK]}),
    ] {
        let mut connection = connection(Default::default());
        save(&connection, "config", &capture).unwrap();
        let revision = format!("initial:{}:explicit:sitemap_document", "a".repeat(64));
        let (root, policy) =
            declared_policy_root(SitemapRootKind::ExplicitConfig, 0, TASK, &revision, 0);
        named_failure(
            append_declared_root(&mut connection, root, &policy),
            "Original explicit sitemap coordinates unavailable",
        );
        assert_eq!(count(&connection, "sitemap_roots"), 0);
        assert_eq!(count(&connection, "policy_occurrences"), 0);
    }
}
#[test]
fn explicit_wrong_raw_basis_observation_ordinal_revision_and_source_reject_named() {
    for control in ["raw", "basis", "observed", "ordinal", "revision", "source"] {
        let mut connection = connection(Default::default());
        save(
            &connection,
            "config",
            &serde_json::json!({"explicit_sitemap_inputs":[TASK,TASK]}),
        )
        .unwrap();
        let revision = format!("initial:{}:explicit:sitemap_document", "a".repeat(64));
        let (mut root, mut policy) =
            declared_policy_root(SitemapRootKind::ExplicitConfig, 0, TASK, &revision, 0);
        let expected = match control {
            "raw" => {
                policy.decision.raw_value = "https://example.org/other.xml".into();
                "Root policy raw/basis"
            }
            "basis" => {
                policy.input_basis = crate::PolicyInputBasis::ResolvedLegacyFallback;
                "Root policy raw/basis"
            }
            "observed" => {
                policy.raw_value_observed = false;
                "Root policy raw/basis"
            }
            "ordinal" => {
                policy.source_ordinal = 1;
                "Explicit root source ordinal/raw"
            }
            "revision" => {
                policy.source_revision = "other-source".into();
                root.admission.policy_source_revision = Some(policy.source_revision.clone());
                "Explicit root policy revision/source"
            }
            "source" => {
                policy.decision.source_url = Some("https://example.org/".into());
                "Explicit root policy revision/source"
            }
            _ => unreachable!(),
        };
        named_failure(
            append_declared_root(&mut connection, root, &policy),
            expected,
        );
        assert_eq!(count(&connection, "sitemap_roots"), 0);
        assert_eq!(count(&connection, "policy_occurrences"), 0);
    }
}
#[test]
fn persisted_root_policy_raw_change_with_same_alias_is_rejected_on_read() {
    let mut connection = connection(Default::default());
    save(
        &connection,
        "config",
        &serde_json::json!({"explicit_sitemap_inputs":[TASK]}),
    )
    .unwrap();
    let revision = format!("initial:{}:explicit:sitemap_document", "a".repeat(64));
    let (root, policy) =
        declared_policy_root(SitemapRootKind::ExplicitConfig, 0, TASK, &revision, 0);
    append_declared_root(&mut connection, root, &policy).unwrap();
    load_catalog(&connection).unwrap().unwrap();
    let mut stored =
        committed_policy_occurrence(&connection, &policy.decision.occurrence_id).unwrap();
    stored.occurrence.decision.raw_value = "https://example.org/another.xml".into();
    connection
        .execute(
            "UPDATE policy_occurrences SET data=?1 WHERE occurrence_id=?2",
            params![
                serde_json::to_string(&stored).unwrap(),
                policy.decision.occurrence_id
            ],
        )
        .unwrap();
    let failure = load_catalog(&connection).unwrap_err();
    assert!(format!("{failure:#}").contains("Root policy raw/basis"));
    assert_eq!(count(&connection, "sitemap_roots"), 1);
}
#[test]
fn current_root_policy_noncurrent_basis_or_future_generation_reject_on_read() {
    for future_generation in [false, true] {
        let mut connection = connection(Default::default());
        save(
            &connection,
            "config",
            &serde_json::json!({"explicit_sitemap_inputs":[TASK]}),
        )
        .unwrap();
        let revision = format!("initial:{}:explicit:sitemap_document", "a".repeat(64));
        let (root, policy) =
            declared_policy_root(SitemapRootKind::ExplicitConfig, 0, TASK, &revision, 0);
        append_declared_root(&mut connection, root, &policy).unwrap();
        load_catalog(&connection).unwrap().unwrap();
        let mut stored =
            committed_policy_occurrence(&connection, &policy.decision.occurrence_id).unwrap();
        if future_generation {
            stored.generation = 2;
        } else {
            stored.basis = crate::PolicyObservationBasis::Inherited;
        }
        connection
            .execute(
                "UPDATE policy_occurrences SET generation=?1,data=?2 WHERE occurrence_id=?3",
                params![
                    i64::try_from(stored.generation).unwrap(),
                    serde_json::to_string(&stored).unwrap(),
                    policy.decision.occurrence_id
                ],
            )
            .unwrap();
        let failure = load_catalog(&connection).unwrap_err();
        assert!(format!("{failure:#}").contains(if future_generation {
            "generation outside actual run history"
        } else {
            "noncurrent policy SQL observation"
        }));
    }
}
#[test]
fn explicit_original_membership_swap_rejects_even_when_raw_policy_and_fetch_alias_match() {
    let mut connection = connection(Default::default());
    save(
        &connection,
        "config",
        &serde_json::json!({"explicit_sitemap_inputs":[TASK]}),
    )
    .unwrap();
    let revision = format!("initial:{}:explicit:sitemap_document", "a".repeat(64));
    let (root, policy) =
        declared_policy_root(SitemapRootKind::ExplicitConfig, 0, TASK, &revision, 0);
    {
        let tx = connection.transaction().unwrap();
        insert_policy(&tx, &policy);
        append_roots(&tx, 1, vec![root.clone()]).unwrap();
    }
    // The accepted baseline transaction is rolled back before the mutated attempt.
    assert_eq!(count(&connection, "sitemap_roots"), 0);
    assert_eq!(count(&connection, "policy_occurrences"), 0);
    let mut swapped = root;
    swapped.normalized_document_url = Some("https://example.org/swapped-original.xml".into());
    swapped.admission.membership_url = swapped.normalized_document_url.clone();
    assert_eq!(swapped.admission.admitted_fetch_url.as_deref(), Some(TASK));
    named_failure(
        append_declared_root(&mut connection, swapped, &policy),
        "Explicit root original normalized identity differs",
    );
    assert_eq!(count(&connection, "sitemap_roots"), 0);
}
fn imported_config() -> serde_json::Value {
    serde_json::json!({"input_imports":[
        {"input_sha256":"b".repeat(64),"rows":[
            {"candidate_kind":"page_url","source_index":0,"raw_value":"https://example.org/page","normalized_url":"https://example.org/page"},
            {"candidate_kind":"sitemap_document","source_index":7,"raw_value":TASK,"normalized_url":TASK,"error":null}]},
        {"input_sha256":"c".repeat(64),"rows":[
            {"candidate_kind":"page_url","source_index":3,"raw_value":"https://example.org/ignored","normalized_url":"https://example.org/ignored"},
            {"candidate_kind":"sitemap_document","source_index":42,"raw_value":TASK,"normalized_url":TASK,"error":null}]}]})
}
#[test]
fn imported_flattened_ordinal_preserves_actual_sparse_source_index_and_import_revision() {
    let mut connection = connection(Default::default());
    save(&connection, "config", &imported_config()).unwrap();
    let revision = format!("initial:{}:1:{}", "a".repeat(64), "c".repeat(64));
    let (root, policy) =
        declared_policy_root(SitemapRootKind::ImportedSitemapRow, 1, TASK, &revision, 42);
    append_declared_root(&mut connection, root, &policy).unwrap();
    let catalog = load_catalog(&connection).unwrap().unwrap();
    assert_eq!(catalog.summary.root_rows, 1);
}
#[test]
fn imported_wrong_flatten_index_source_index_revision_raw_and_normalized_reject_named() {
    for control in ["flattened", "source_index", "revision", "raw", "normalized"] {
        let mut connection = connection(Default::default());
        save(&connection, "config", &imported_config()).unwrap();
        let revision = format!("initial:{}:1:{}", "a".repeat(64), "c".repeat(64));
        let (mut root, mut policy) =
            declared_policy_root(SitemapRootKind::ImportedSitemapRow, 1, TASK, &revision, 42);
        let expected = match control {
            "flattened" => {
                root.source_ordinal = 2;
                root.declaration_id = root_declaration_id(&root.kind, &"a".repeat(64), 2);
                "Imported sitemap root ordinal has no original input row"
            }
            "source_index" => {
                policy.source_ordinal = 1;
                "Imported root raw/normalized/source index"
            }
            "revision" => {
                policy.source_revision = format!("initial:{}:0:{}", "a".repeat(64), "b".repeat(64));
                root.admission.policy_source_revision = Some(policy.source_revision.clone());
                "Imported root policy revision/source"
            }
            "raw" => {
                root.raw_value = "https://example.org/other.xml".into();
                policy.decision.raw_value = root.raw_value.clone();
                "Imported root raw/normalized/source index"
            }
            "normalized" => {
                root.normalized_document_url = Some("https://example.org/other.xml".into());
                root.admission.membership_url = root.normalized_document_url.clone();
                "Imported root raw/normalized/source index"
            }
            _ => unreachable!(),
        };
        named_failure(
            append_declared_root(&mut connection, root, &policy),
            expected,
        );
        assert_eq!(count(&connection, "sitemap_roots"), 0);
        assert_eq!(count(&connection, "policy_occurrences"), 0);
    }
}
fn policy_robots_batch(
    connection: &Connection,
) -> (SitemapDiscoveryBatch, crate::PolicyOccurrence) {
    let task = "https://example.org/";
    let requested = "https://example.org/robots.txt";
    let body = format!("Sitemap: {TASK}\n");
    let raw = format!(" {TASK}");
    let revision = digest(&body);
    connection
        .execute(
            "INSERT INTO discovery_frontier(kind,url) VALUES('robots',?1)",
            [task],
        )
        .unwrap();
    let id =
        source_observation_id(SitemapSourceKind::RobotsDirectives, task, 1, &revision).unwrap();
    let document = SitemapSourceDocument {
        sequence: 0,
        observation_id: id.clone(),
        kind: SitemapSourceKind::RobotsDirectives,
        task_url: task.into(),
        generation: 1,
        source_revision: revision.clone(),
        transport: Some(SitemapTransportFacts {
            request_started: true,
            requested_url: Some(requested.into()),
            final_url: Some(requested.into()),
            initial_status: Some(200),
            final_status: Some(200),
            blocked_by_robots: false,
            body_complete: true,
            post_http_decoding_bytes: body.len() as u64,
            post_http_decoding_sha256: Some(digest(&body)),
            retained_post_http_prefix_sha256: Some(digest(&body)),
            failure_code: None,
        }),
        entity_decode: None,
        parser: None,
        declaration_scan_complete: true,
        declaration_utf8_lossy: Some(false),
        declaration_count: Some(1),
        retained_rows_complete: true,
        failure_code: None,
    };
    let (mut root, mut policy) =
        declared_policy_root(SitemapRootKind::RobotsDirective, 0, &raw, &revision, 0);
    root.source_observation_id = Some(id.clone());
    root.declaration_id = root_declaration_id(&root.kind, &id, 0);
    root.raw_line = Some(body.clone());
    root.robots_evidence = Some(SitemapRobotsDeclarationFacts {
        line_ordinal: 1,
        byte_start: 0,
        byte_end: body.len(),
        line_complete: true,
        utf8_lossy: false,
        legacy_candidate_url: Some(TASK.into()),
    });
    policy.decision.source_url = Some(task.into());
    (
        SitemapDiscoveryBatch {
            document,
            roots: vec![root],
            entries: vec![],
            locations: vec![],
        },
        policy,
    )
}
#[test]
fn robots_policy_root_accepts_exact_raw_directive_and_original_source_coordinates() {
    let mut connection = connection(Default::default());
    let (value, policy) = policy_robots_batch(&connection);
    let task = value.document.task_url.clone();
    {
        let tx = connection.transaction().unwrap();
        let token =
            guard_pending_source(&tx, 1, SitemapSourceKind::RobotsDirectives, &task).unwrap();
        insert_policy(&tx, &policy);
        append_source(&tx, token, value).unwrap();
        tx.commit().unwrap();
    }
    let catalog = load_catalog(&connection).unwrap().unwrap();
    assert_eq!(
        (catalog.summary.document_rows, catalog.summary.root_rows),
        (1, 1)
    );
    assert_eq!(catalog.summary.incomplete_documents, 0);
}
#[test]
fn original_robots_identity_rejects_non_xml_whitespace_edges_before_core_trim() {
    assert_eq!(
        original_robots_document_url(&format!(" \t{TASK}\r\n")),
        Some(TASK.into())
    );
    for raw in [
        format!("{TASK}\u{00a0}"),
        format!("\u{00a0}{TASK}"),
        format!("{TASK}\u{0085}"),
        "https://example.org/fi\tle.xml".into(),
        "https://user:password@example.org/file.xml".into(),
    ] {
        let original = raw.clone();
        assert!(original_robots_document_url(&raw).is_none());
        assert_eq!(raw, original);
    }
}
#[test]
fn robots_policy_wrong_raw_ordinal_document_revision_and_source_reject_named() {
    for control in ["raw", "ordinal", "revision", "source", "normalized"] {
        let mut connection = connection(Default::default());
        let (mut value, mut policy) = policy_robots_batch(&connection);
        let expected = match control {
            "raw" => {
                policy.decision.raw_value = TASK.into();
                "Root policy raw/basis"
            }
            "ordinal" => {
                policy.source_ordinal = 1;
                "Policy-bound robots generation/source/ordinal/revision"
            }
            "revision" => {
                policy.source_revision = "another-source".into();
                value.roots[0].admission.policy_source_revision =
                    Some(policy.source_revision.clone());
                "Policy-bound robots generation/source/ordinal/revision"
            }
            "source" => {
                policy.decision.source_url = Some("https://example.org/robots.txt".into());
                "Policy-bound robots generation/source/ordinal/revision"
            }
            "normalized" => {
                value.roots[0].normalized_document_url =
                    Some("https://example.org/swapped.xml".into());
                value.roots[0].admission.membership_url =
                    value.roots[0].normalized_document_url.clone();
                "Robots root original normalized identity differs"
            }
            _ => unreachable!(),
        };
        let task = value.document.task_url.clone();
        let result = (|| -> Result<()> {
            let tx = connection.transaction()?;
            let token = guard_pending_source(&tx, 1, SitemapSourceKind::RobotsDirectives, &task)?;
            insert_policy(&tx, &policy);
            append_source(&tx, token, value)?;
            tx.commit()?;
            Ok(())
        })();
        named_failure(result, expected);
        assert_eq!(count(&connection, "sitemap_roots"), 0);
        assert_eq!(
            connection
                .query_row(
                    "SELECT done FROM discovery_frontier WHERE kind='robots' AND url=?1",
                    [task],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}
#[test]
fn pure_inherited_projection_equals_sql_inherit_without_rewriting_original_parent() {
    let mut parent = connection(Default::default());
    commit(
        &mut parent,
        batch("<urlset><url/><url><loc>https://example.org/a</loc></url></urlset>"),
    )
    .unwrap();
    let catalog = load_catalog(&parent).unwrap().unwrap();
    let original = catalog.clone();
    let mut child = empty_connection();
    let parent_sha = selected_anchor(&child, &catalog);
    let projected = project_inherited_catalog(&catalog, &parent_sha).unwrap();
    {
        let tx = child.transaction().unwrap();
        inherit(&tx, &catalog, &parent_sha).unwrap();
        tx.commit().unwrap();
    }
    assert_eq!(load_catalog(&child).unwrap().unwrap(), projected);
    assert_eq!(catalog, original);
    assert_eq!(projected.rows, catalog.rows);
    assert_eq!(
        projected.summary.rolling_sha256,
        catalog.summary.rolling_sha256
    );
    assert!(!projected.inherited.unwrap().current_discovery_executed);
    assert!(!projected.summary.declared_source_cohort_complete);
    assert!(project_inherited_catalog(&catalog, "invalid-sha").is_err());
}

#[test]
fn actual_prepared_large_selected_plan_does_not_use_sitemap_summary_metadata_cap() {
    let mut parent = connection(SitemapLedgerLimits::default());
    commit(&mut parent, batch("<urlset/> ")).unwrap();
    let catalog = load_catalog(&parent).unwrap().unwrap();
    let urls = (0..1200)
        .map(|index| format!("https://example.org/page/{index}"))
        .collect::<Vec<_>>();
    let pages = urls
        .iter()
        .map(|url| crate::PageRecord {
            url: url.clone(),
            final_url: url.clone(),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let snapshot=serde_json::to_vec(&serde_json::json!({
        "config":crate::CrawlConfig {seed_urls:vec!["https://example.org/".into()],..Default::default()},
        "pages":pages,"issues":[],"summary":crate::CrawlSummary::default(),"warnings":[],
        "operation":null,"policy_ledger":null,"resource_ledger":null,"sitemap_catalog":catalog
    })).unwrap();
    let prepared = crate::prepare_selected_recrawl(
        crate::SelectedRecrawlParent {
            job_id: "fixture-large-parent".into(),
            checkpoint_revision: None,
            source_kind: crate::RecrawlSourceKind::StoredJson,
        },
        &snapshot,
        &urls,
        None,
    )
    .unwrap();
    prepared.validate().unwrap();
    let mut selected = empty_connection();
    save(
        &selected,
        "run_plan",
        &crate::RunPlan::SelectedRecrawl(prepared.plan.clone()),
    )
    .unwrap();
    selected
        .execute(
            "INSERT INTO recrawl_base(id,snapshot) VALUES(1,?1)",
            [prepared.base_snapshot_bytes],
        )
        .unwrap();
    let bytes: i64 = selected
        .query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key='run_plan'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        bytes > 65536,
        "fixture must exceed the unrelated sitemap-summary bound"
    );
    {
        let tx = selected.transaction().unwrap();
        inherit(&tx, &catalog, &prepared.plan.parent.snapshot_sha256).unwrap();
        tx.commit().unwrap();
    }
    let inherited = load_catalog(&selected).unwrap().unwrap();
    assert_eq!(inherited.rows, catalog.rows);
    assert_eq!(
        inherited.summary.rolling_sha256,
        catalog.summary.rolling_sha256
    );
}

#[path = "ordinary_coordinate_controls.rs"]
mod ordinary_coordinate_controls;

#[test]
fn shared_original_root_guard_runs_before_ordinary_unbound_admission() {
    let connection = connection(Default::default());
    save(
        &connection,
        "config",
        &serde_json::json!({"explicit_sitemap_inputs":[TASK]}),
    )
    .unwrap();
    let (mut root, _) = declared_policy_root(SitemapRootKind::ExplicitConfig, 0, TASK, "unused", 0);
    root.admission.policy_occurrence_id = None;
    root.admission.policy_source_revision = None;
    root.admission.decision = SitemapAdmissionDecision::Unbound;
    root.admission.admitted_fetch_url = None;
    verify_original_root_coordinates(&connection, &root).unwrap();
    root.normalized_document_url = Some("https://example.org/foreign.xml".into());
    root.admission.membership_url = root.normalized_document_url.clone();
    let before = root.clone();
    let failure = verify_original_root_coordinates(&connection, &root).unwrap_err();
    assert!(format!("{failure:#}").contains("Explicit root original normalized identity differs"));
    assert_eq!(root, before);
    assert_eq!(count(&connection, "sitemap_roots"), 0);
}
