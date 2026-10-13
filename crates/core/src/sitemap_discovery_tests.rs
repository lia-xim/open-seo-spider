use super::*;
use crate::sitemap_ledger::{self, SitemapLedgerLimits, SitemapLedgerRow};
use rusqlite::Connection;

const TASK: &str = "https://example.org/sitemap.xml";
fn xml(raw: &str) -> SitemapDiscoveryBatch {
    let limits = crate::SitemapParserLimits::default();
    let decoded = crate::decode_sitemap_entity(raw.as_bytes(), &limits).unwrap();
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
    let failure = parser.failure.as_ref().map(|value| value.code.clone());
    xml_batch(
        TASK,
        7,
        "original-unit-revision",
        SitemapTransportFacts {
            request_started: true,
            requested_url: Some(TASK.into()),
            final_url: Some(TASK.into()),
            initial_status: Some(200),
            final_status: Some(200),
            blocked_by_robots: false,
            body_complete: true,
            post_http_decoding_bytes: raw.len() as u64,
            post_http_decoding_sha256: Some(sitemap_ledger::digest(raw)),
            retained_post_http_prefix_sha256: None,
            failure_code: None,
        },
        Some(&decoded),
        Some(parser),
        failure,
    )
    .unwrap()
}
fn robots(bytes: &[u8], eof: bool, status: u16) -> SitemapDiscoveryBatch {
    let mut body = crate::RobotsBodyBuffer::default();
    body.push_chunk(bytes);
    if eof {
        body.observe_eof();
    }
    let source = crate::RobotsSitemapSource::observed(
        "https://example.org/robots.txt".into(),
        "https://cdn.example.org/robot-rules".into(),
        status,
        &body,
        &url::Url::parse("https://example.org/").unwrap(),
    );
    robots_batch("https://example.org/", 7, "robots-original-unit", &source).unwrap()
}
fn connection(kind: &str, task: &str) -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    connection.execute_batch("CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);
        CREATE TABLE discovery_frontier(kind TEXT NOT NULL,url TEXT NOT NULL,done INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(kind,url));
        CREATE TABLE policy_occurrences(sequence INTEGER PRIMARY KEY,occurrence_id TEXT UNIQUE NOT NULL,generation INTEGER NOT NULL,data TEXT NOT NULL);").unwrap();
    connection.execute_batch(sitemap_ledger::SCHEMA).unwrap();
    let state = crate::CrawlRunState {
        generation: 7,
        started_at: "fixture".into(),
        finished_at: None,
        outcome: None,
    };
    connection
        .execute(
            "INSERT INTO metadata VALUES('run_state',?1)",
            [serde_json::to_string(&state).unwrap()],
        )
        .unwrap();
    let tx = connection.transaction().unwrap();
    sitemap_ledger::initialize(&tx, &"a".repeat(64), &SitemapLedgerLimits::default(), false)
        .unwrap();
    tx.commit().unwrap();
    connection
        .execute(
            "INSERT INTO discovery_frontier(kind,url) VALUES(?1,?2)",
            [kind, task],
        )
        .unwrap();
    connection
}
fn commit(
    connection: &mut Connection,
    batch: SitemapDiscoveryBatch,
) -> crate::sitemap_ledger::SitemapCatalog {
    let tx = connection.transaction().unwrap();
    let guard =
        sitemap_ledger::guard_pending_source(&tx, 7, batch.document.kind, &batch.document.task_url)
            .unwrap();
    sitemap_ledger::append_source(&tx, guard, batch).unwrap();
    tx.commit().unwrap();
    sitemap_ledger::load_catalog(connection).unwrap().unwrap()
}
#[test]
fn xml_original_untrimmed_values_ordinals_and_missing_rows_survive_projection() {
    let batch = xml(
        "<urlset><url><loc> https://example.org/a </loc></url><url><loc>https://example.org/a</loc></url><url><loc>ftp://example.org/x</loc></url><url/></urlset>",
    );
    assert!(batch.document.complete(), "{:?}", batch.document);
    assert_eq!(batch.entries.len(), 4);
    assert_eq!(batch.locations.len(), 4);
    assert_eq!(
        batch
            .locations
            .iter()
            .map(|row| row.observed.ordinal)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(
        batch.locations[0].observed.decoded_value.as_deref(),
        Some(" https://example.org/a ")
    );
    assert_eq!(batch.locations[2].admission.decision, D::InvalidLocation);
    assert_eq!(batch.locations[3].admission.decision, D::MissingLocation);
    assert_eq!(
        batch.locations[0].observed.normalized_fetch_url,
        batch.locations[1].observed.normalized_fetch_url
    );
    assert_ne!(
        batch.locations[0].occurrence_id,
        batch.locations[1].occurrence_id
    );
}
#[test]
fn malformed_xml_prefix_is_committed_without_any_admitted_alias() {
    let batch = xml(
        "<urlset><url><loc>https://example.org/a</loc></url><url><loc>https://example.org/b</loc></url></broken>",
    );
    assert!(!batch.document.complete());
    assert!(batch.document.parser.as_ref().unwrap().failure.is_some());
    assert!(!batch.locations.is_empty());
    assert!(
        batch
            .locations
            .iter()
            .all(|row| row.admission.decision == D::IncompleteSource
                && row.admission.admitted_fetch_url.is_none())
    );
    let mut connection = connection("sitemap", TASK);
    let catalog = commit(&mut connection, batch);
    assert_eq!(catalog.summary.incomplete_documents, 1);
    assert!(!catalog.summary.declared_source_cohort_complete);
    assert_eq!(
        connection
            .query_row("SELECT done FROM discovery_frontier", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn ambiguous_multiple_locs_keep_both_original_occurrences_without_admission() {
    let batch = xml(
        "<urlset><url><loc>https://example.org/a</loc><loc>https://example.org/b</loc></url></urlset>",
    );
    assert!(!batch.document.complete());
    assert_eq!(
        batch.document.failure_code.as_deref(),
        Some("sitemap_ambiguous_multiple_locations")
    );
    assert_eq!(batch.locations.len(), 2);
    assert_eq!(
        batch
            .locations
            .iter()
            .map(|row| row.observed.ordinal)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(
        batch
            .locations
            .iter()
            .all(|row| row.admission.decision == D::IncompleteSource)
    );
    let mut connection = connection("sitemap", TASK);
    let catalog = commit(&mut connection, batch);
    assert_eq!(
        (
            catalog.summary.location_rows,
            catalog.summary.incomplete_documents
        ),
        (2, 1)
    );
}
#[test]
fn decode_failure_can_retain_complete_http_entity_without_parser_facts() {
    let raw = b"invalid-file-gzip-footer";
    let batch = xml_batch(
        TASK,
        7,
        "decoder-failed-unit",
        SitemapTransportFacts {
            request_started: true,
            requested_url: Some(TASK.into()),
            final_url: Some(TASK.into()),
            initial_status: Some(200),
            final_status: Some(200),
            blocked_by_robots: false,
            body_complete: true,
            post_http_decoding_bytes: raw.len() as u64,
            post_http_decoding_sha256: Some(sitemap_ledger::digest(raw)),
            retained_post_http_prefix_sha256: Some(sitemap_ledger::digest(raw)),
            failure_code: None,
        },
        None,
        None,
        Some("sitemap_entity_gzip_failed".into()),
    )
    .unwrap();
    assert!(!batch.document.complete());
    assert!(
        batch.document.parser.is_none() && batch.entries.is_empty() && batch.locations.is_empty()
    );
    let mut connection = connection("sitemap", TASK);
    let catalog = commit(&mut connection, batch);
    assert_eq!(
        (
            catalog.summary.document_rows,
            catalog.summary.location_rows,
            catalog.summary.incomplete_documents
        ),
        (1, 0, 1)
    );
}
#[test]
fn robots_all_newline_forms_keep_raw_lines_and_source_request_identity() {
    let mut batch = robots(b"Sitemap: https://example.org/a.xml\rSitemap: https://example.org/a.xml\r\nSitemap: /relative.xml\nSitemap:\n", true, 200);
    assert!(batch.document.complete());
    assert_eq!(batch.document.task_url, "https://example.org/");
    assert_eq!(
        batch
            .document
            .transport
            .as_ref()
            .unwrap()
            .requested_url
            .as_deref(),
        Some("https://example.org/robots.txt")
    );
    assert_eq!(
        batch
            .document
            .transport
            .as_ref()
            .unwrap()
            .final_url
            .as_deref(),
        Some("https://cdn.example.org/robot-rules")
    );
    assert!(
        batch
            .document
            .transport
            .as_ref()
            .unwrap()
            .initial_status
            .is_none()
    );
    assert_eq!(
        batch
            .roots
            .iter()
            .map(|row| row.source_ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert_eq!(
        batch.roots[0].raw_line.as_deref(),
        Some("Sitemap: https://example.org/a.xml\r")
    );
    assert_eq!(
        batch.roots[1].raw_line.as_deref(),
        Some("Sitemap: https://example.org/a.xml\r\n")
    );
    assert_eq!(batch.roots[2].admission.decision, D::InvalidLocation);
    assert!(
        batch.roots[2]
            .robots_evidence
            .as_ref()
            .unwrap()
            .legacy_candidate_url
            .is_some()
    );
    assert_eq!(batch.roots[3].admission.decision, D::InvalidLocation);
    // Deliberately observation-only fixture: no network/scheduling attestation.
    for root in &mut batch.roots {
        if root.admission.decision == D::Unbound {
            root.admission.decision = D::SourceCheckOnly;
        }
    }
    let mut connection = connection("robots", "https://example.org/");
    let catalog = commit(&mut connection, batch);
    assert_eq!(
        (catalog.summary.document_rows, catalog.summary.root_rows),
        (1, 4)
    );
    assert!(!catalog.summary.declared_source_cohort_complete);
    assert!(catalog.rows.unwrap().iter().any(
        |row| matches!(row, SitemapLedgerRow::Root(root) if root.raw_value == " /relative.xml")
    ));
}
#[test]
fn clean_http_206_eof_is_an_incomplete_declaration_source() {
    let batch = robots(b"Sitemap: https://example.org/a.xml\n", true, 206);
    assert!(batch.document.transport.as_ref().unwrap().body_complete);
    assert!(!batch.document.complete());
    assert_eq!(batch.roots[0].admission.decision, D::IncompleteSource);
    let mut connection = connection("robots", "https://example.org/");
    assert_eq!(
        commit(&mut connection, batch).summary.incomplete_documents,
        1
    );
}
#[test]
fn lossy_utf8_and_missing_true_eof_cannot_admit_robot_directives() {
    for batch in [
        robots(b"Sitemap: https://example.org/a.xml\n\xff", true, 200),
        robots(b"Sitemap: https://example.org/a.xml", false, 200),
    ] {
        assert!(!batch.document.complete());
        assert_eq!(batch.roots[0].admission.decision, D::IncompleteSource);
        assert!(batch.roots[0].admission.policy_occurrence_id.is_none());
        let mut connection = connection("robots", "https://example.org/");
        assert_eq!(
            commit(&mut connection, batch).summary.incomplete_documents,
            1
        );
    }
}
#[test]
fn strict_root_urls_reject_relative_empty_control_and_credential_inputs() {
    for raw in [
        "",
        "  ",
        "/sitemap.xml",
        "ftp://example.org/x",
        "https://user:secret@example.org/x",
        "https://example.org/a\n.xml",
    ] {
        assert!(strict_robots_location(raw).is_none(), "accepted {raw:?}");
    }
    assert_eq!(
        strict_robots_location(" \thttps://example.org/a.xml\r\n").as_deref(),
        Some("https://example.org/a.xml")
    );
}
fn policy_row(raw: &str) -> crate::PolicyOccurrence {
    let mut config = crate::CrawlPolicyConfig::default();
    config.scope.roots = vec![crate::CrawlScopeRoot {
        origin: "https://example.org".into(),
        subdomains: false,
        folders: vec!["/".into()],
        exact_paths: vec![],
    }];
    let policy = crate::CompiledCrawlPolicy::new(&config).unwrap();
    let decision = policy.evaluate(&crate::CandidateInput {
        occurrence_id: "actual-final-policy-id".into(),
        source_kind: crate::CandidateSourceKind::SitemapDocument,
        raw_value: raw.into(),
        base_url: None,
        source_url: Some("https://example.org/".into()),
        source_location: None,
    });
    crate::PolicyOccurrence {
        source_revision: "robots-original-unit".into(),
        source_ordinal: 0,
        input_basis: crate::PolicyInputBasis::DeclaredInput,
        raw_value_observed: true,
        decision,
        task_role: crate::PolicyTaskRole::SitemapDocument,
        scheduled_fetch_url: None,
        coalesced_task_occurrence_id: None,
        admission_reason: "policy_denied".into(),
        input_error: None,
    }
}
fn store_policy(connection: &Connection, row: &crate::PolicyOccurrence) {
    let saved = crate::StoredPolicyOccurrence {
        sequence: 1,
        generation: 7,
        basis: crate::PolicyObservationBasis::Current,
        occurrence: row.clone(),
    };
    connection.execute("INSERT INTO policy_occurrences(sequence,occurrence_id,generation,data) VALUES(1,?1,7,?2)",
        rusqlite::params![row.decision.occurrence_id, serde_json::to_string(&saved).unwrap()]).unwrap();
}
fn root_context() -> RootPolicyContext<'static> {
    RootPolicyContext {
        source_revision: "robots-original-unit",
        source_ordinal: 0,
        generation: 7,
        source_url: Some("https://example.org/"),
    }
}
#[test]
fn final_root_policy_refuses_raw_identity_role_and_unfinished_decisions() {
    let root = robots(b"Sitemap: https://example.org/a.xml\n", true, 200)
        .roots
        .remove(0);
    let row = policy_row(&root.raw_value);
    let mut connection = connection("robots", "https://example.org/");
    store_policy(&connection, &row);
    let tx = connection.transaction().unwrap();
    for mutation in 0..6 {
        let mut wrong = row.clone();
        match mutation {
            0 => wrong.decision.raw_value = "https://example.org/a.xml".into(),
            1 => wrong.source_revision = "wrong-revision".into(),
            2 => wrong.source_ordinal = 1,
            3 => wrong.task_role = crate::PolicyTaskRole::PageDocument,
            4 => wrong.input_basis = crate::PolicyInputBasis::ResolvedLegacyFallback,
            _ => wrong.admission_reason = "pending_admission".into(),
        }
        assert!(
            finalize_root_policy(&tx, &mut root.clone(), &wrong, &root_context()).is_err(),
            "mutation {mutation}"
        );
    }
    let mut bound = root;
    finalize_root_policy(&tx, &mut bound, &row, &root_context()).unwrap();
    assert_eq!(bound.raw_value, " https://example.org/a.xml");
    assert_eq!(bound.admission.decision, D::PolicyDenied);
    assert_eq!(
        bound.admission.policy_occurrence_id.as_deref(),
        Some("actual-final-policy-id")
    );
}
#[test]
fn incomplete_robot_source_refuses_even_an_otherwise_final_policy_row() {
    let mut root = robots(b"Sitemap: https://example.org/a.xml\n", false, 200)
        .roots
        .remove(0);
    let row = policy_row(&root.raw_value);
    let mut connection = connection("robots", "https://example.org/");
    store_policy(&connection, &row);
    let tx = connection.transaction().unwrap();
    assert!(finalize_root_policy(&tx, &mut root, &row, &root_context()).is_err());
    assert!(root.admission.policy_occurrence_id.is_none());
}
#[test]
fn root_policy_rejects_another_persisted_raw_declaration_with_same_final_alias() {
    let mut root = robots(b"Sitemap: https://example.org/a.xml\n", true, 200)
        .roots
        .remove(0);
    let actual = policy_row(" https://example.org/b.xml");
    let mut supplied = actual.clone();
    supplied.decision.raw_value = root.raw_value.clone();
    let mut connection = connection("robots", "https://example.org/");
    store_policy(&connection, &actual);
    let tx = connection.transaction().unwrap();
    let error = finalize_root_policy(&tx, &mut root, &supplied, &root_context())
        .unwrap_err()
        .to_string();
    assert!(error.contains("actual committed SQL occurrence"), "{error}");
    assert!(root.admission.policy_occurrence_id.is_none());
}
#[test]
fn root_policy_rejects_sql_index_and_expected_source_context_tampering() {
    for mutation in 0..5 {
        let mut root = robots(b"Sitemap: https://example.org/a.xml\n", true, 200)
            .roots
            .remove(0);
        let row = policy_row(&root.raw_value);
        let mut connection = connection("robots", "https://example.org/");
        store_policy(&connection, &row);
        let mut context = root_context();
        match mutation {
            0 => {
                connection
                    .execute("UPDATE policy_occurrences SET sequence=2", [])
                    .unwrap();
            }
            1 => {
                connection
                    .execute("UPDATE policy_occurrences SET generation=8", [])
                    .unwrap();
            }
            2 => context.generation = 8,
            3 => context.source_url = Some("https://elsewhere.test/"),
            _ => context.source_ordinal = 1,
        }
        let tx = connection.transaction().unwrap();
        assert!(
            finalize_root_policy(&tx, &mut root, &row, &context).is_err(),
            "mutation {mutation}"
        );
        assert!(root.admission.policy_occurrence_id.is_none());
    }
}

#[test]
fn robots_non_xml_whitespace_does_not_become_an_admitted_trimmed_alias() {
    for raw in [
        "\u{a0}https://example.org/a.xml",
        "https://example.org/a.xml\u{a0}",
        "https://example.org/a.xml\u{85}",
    ] {
        assert!(
            crate::normalize_url(raw, None).is_some(),
            "Core compatibility profile differs"
        );
        assert!(
            strict_robots_location(raw).is_none(),
            "Original declaration was silently changed: {raw:?}"
        );
        let source = format!("Sitemap: {raw}\n");
        let batch = robots(source.as_bytes(), true, 200);
        assert!(batch.document.complete());
        assert_eq!(batch.roots.len(), 1);
        assert!(batch.roots[0].normalized_document_url.is_none());
        assert_eq!(batch.roots[0].admission.decision, D::InvalidLocation);
        assert_eq!(batch.roots[0].raw_value, format!(" {raw}"));
    }
}
