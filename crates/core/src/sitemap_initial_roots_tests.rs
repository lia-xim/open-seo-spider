//! Authored regression candidates for Root sitemap_initial_roots_v2.
//! Requires Ledger V5 shared original-coordinate check, adapter V3, actual Core policy APIs.
//! Append #[cfg(test)] #[path="sitemap_initial_roots_tests.rs"] mod tests; in Root's module.
//! NOT compiled/run by this author. SQLite is in-memory; no HTTP/app/native launches.
use super::*;
use crate::sitemap_ledger::{self, SitemapLedgerLimits, SitemapLedgerRow, SitemapLedgerSummary};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

const A: &str = "https://example.org/a.xml";
const B: &str = "https://example.org/b.xml";

fn policy() -> crate::CompiledCrawlPolicy {
    let mut config = crate::CrawlPolicyConfig::default();
    config.scope.roots = vec![crate::CrawlScopeRoot {
        origin: "https://example.org".into(),
        subdomains: false,
        folders: vec!["/".into()],
        exact_paths: vec![],
    }];
    crate::CompiledCrawlPolicy::new(&config).unwrap()
}
fn import_xml(locations: &[&str]) -> crate::ListImportResult {
    let xml = format!(
        "<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">{}</sitemapindex>",
        locations
            .iter()
            .map(|value| format!("<sitemap><loc>{value}</loc></sitemap>"))
            .collect::<String>()
    );
    crate::import_url_list(
        xml.as_bytes(),
        &crate::ListImportOptions {
            format: crate::ListImportFormat::Xml,
            limits: crate::ListImportLimits::default(),
        },
    )
    .unwrap()
}
fn imports() -> Vec<crate::ListImportResult> {
    vec![
        import_xml(&[
            "https://example.org/shared.xml",
            "ftp://example.org/rejected.xml",
            "https://example.org/imported-only.xml",
        ]),
        crate::import_url_list(
            b"https://example.org/page\n",
            &crate::ListImportOptions {
                format: crate::ListImportFormat::Txt,
                limits: crate::ListImportLimits::default(),
            },
        )
        .unwrap(),
        import_xml(&[
            "https://example.org/shared.xml",
            "https://example.org/second.xml",
        ]),
    ]
}
fn explicit() -> Vec<String> {
    vec![
        "HTTPS://EXAMPLE.ORG/shared.xml#one".into(),
        "HTTPS://EXAMPLE.ORG/shared.xml#one".into(),
        "https://example.org/explicit-only.xml".into(),
    ]
}
fn run(connection: &Connection, generation: u64, terminal: bool) {
    let state = crate::CrawlRunState {
        generation,
        started_at: "owned-fixture".into(),
        finished_at: terminal.then(|| "owned-fixture-finished".into()),
        outcome: terminal.then_some(crate::CrawlOutcome::Paused),
    };
    connection
        .execute(
            "INSERT OR REPLACE INTO metadata(key,value) VALUES('run_state',?1)",
            [serde_json::to_string(&state).unwrap()],
        )
        .unwrap();
}
fn fixture(
    explicit: Option<&[String]>,
    imports: &[crate::ListImportResult],
    with_policy: bool,
) -> (Connection, String) {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(
        "CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);
        CREATE TABLE frontier(url TEXT PRIMARY KEY,done INTEGER NOT NULL DEFAULT 0);
         CREATE TABLE discovery_frontier(kind TEXT NOT NULL,url TEXT NOT NULL,done INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(kind,url));"
    ).unwrap();
    connection
        .execute_batch(crate::policy_ledger::SCHEMA)
        .unwrap();
    connection.execute_batch(sitemap_ledger::SCHEMA).unwrap();
    // Actual serialized saved config bytes bind the fixture; no invented SHA/context.
    // JSON field injection keeps this test readable before Core field integration.
    let mut sitemap_urls: Vec<String> = explicit
        .unwrap_or(&[])
        .iter()
        .filter_map(|raw| crate::normalize_url(raw, None).map(|url| url.to_string()))
        .collect();
    for input in imports {
        for row in &input.rows {
            if row.candidate_kind == crate::ListCandidateKind::SitemapDocument
                && row.error.is_none()
                && let Some(url) = &row.normalized_url
                && !sitemap_urls.contains(url)
            {
                sitemap_urls.push(url.clone());
            }
        }
    }
    let mut value = serde_json::to_value(crate::CrawlConfig {
        seed_urls: vec!["https://example.org/".into()],
        sitemap_urls,
        input_imports: imports.to_vec(),
        ..Default::default()
    })
    .unwrap();
    match explicit {
        Some(values) => value["explicit_sitemap_inputs"] = serde_json::json!(values),
        None => {
            value
                .as_object_mut()
                .unwrap()
                .remove("explicit_sitemap_inputs");
        }
    }
    if with_policy {
        value["crawl_policy"] = serde_json::to_value(policy().config()).unwrap();
    }
    let config = serde_json::to_string(&value).unwrap();
    let sha = sitemap_ledger::digest(&config);
    connection
        .execute(
            "INSERT INTO metadata(key,value) VALUES('config',?1)",
            [&config],
        )
        .unwrap();
    run(&connection, 1, false);
    (connection, sha)
}
fn roots(
    explicit: &[String],
    imports: &[crate::ListImportResult],
    sha: &str,
    policy: bool,
) -> Vec<InitialSitemapRoot> {
    let policy_sha = policy.then(|| self::policy().policy_sha256().to_owned());
    initial_roots(Some(explicit), imports, sha, policy_sha.as_deref())
        .unwrap()
        .unwrap()
}
fn tasks(roots: &[InitialSitemapRoot]) -> Vec<String> {
    let mut unique = std::collections::BTreeSet::new();
    for item in roots {
        if item.root.admission.decision == D::Unbound {
            unique.insert(item.root.normalized_document_url.clone().unwrap());
        }
    }
    unique.into_iter().collect()
}
fn enqueue(connection: &Connection, tasks: &[String]) {
    for task in tasks {
        connection
            .execute(
                "INSERT OR IGNORE INTO discovery_frontier(kind,url) VALUES('sitemap',?1)",
                [task],
            )
            .unwrap();
    }
}
fn counts(roots: &[InitialSitemapRoot]) -> (usize, usize, usize) {
    (
        roots
            .iter()
            .filter(|item| item.root.admission.decision == D::Scheduled)
            .count(),
        roots
            .iter()
            .filter(|item| item.root.admission.decision == D::Coalesced)
            .count(),
        roots
            .iter()
            .filter(|item| item.root.admission.decision == D::InvalidLocation)
            .count(),
    )
}
fn make_policy_rows(
    explicit: &[String],
    imports: &[crate::ListImportResult],
) -> Vec<crate::PolicyOccurrence> {
    let policy = policy();
    let mut rows = vec![];
    // Actual existing initializer order is imports first; do not fabricate grants.
    for (input_index, input) in imports.iter().enumerate() {
        crate::validate_import_result(input).unwrap();
        let revision = format!(
            "initial:{}:{input_index}:{}",
            policy.policy_sha256(),
            input.input_sha256
        );
        for row in &input.rows {
            let (kind, role) = match row.candidate_kind {
                crate::ListCandidateKind::PageUrl => (
                    crate::CandidateSourceKind::ListPage,
                    crate::PolicyTaskRole::PageDocument,
                ),
                crate::ListCandidateKind::SitemapDocument => (
                    crate::CandidateSourceKind::SitemapDocument,
                    crate::PolicyTaskRole::SitemapDocument,
                ),
            };
            let mut occurrence = crate::policy_ledger::decide(
                &policy,
                &revision,
                row.source_index,
                kind,
                row.raw_value.clone(),
                None,
                None,
                crate::PolicyInputBasis::DeclaredInput,
                role,
            );
            if row.error.is_some() || row.normalized_url.is_none() {
                occurrence.admission_reason = "input_row_rejected".into();
                occurrence.input_error = row
                    .error
                    .clone()
                    .or_else(|| Some("Importer supplied no accepted normalized URL".into()));
            }
            rows.push(occurrence);
        }
    }
    let revision = format!(
        "initial:{}:explicit:sitemap_document",
        policy.policy_sha256()
    );
    for (ordinal, raw) in explicit.iter().enumerate() {
        rows.push(crate::policy_ledger::decide(
            &policy,
            &revision,
            ordinal,
            crate::CandidateSourceKind::SitemapDocument,
            raw.clone(),
            None,
            None,
            crate::PolicyInputBasis::DeclaredInput,
            crate::PolicyTaskRole::SitemapDocument,
        ));
    }
    rows
}
fn commit_initial_policy(
    connection: &mut Connection,
    explicit: &[String],
    imports: &[crate::ListImportResult],
    creation: u64,
) {
    run(connection, creation, false);
    let policy = policy();
    let tx = connection.transaction().unwrap();
    crate::policy_ledger::initialize(&tx, policy.policy_sha256(), None, false).unwrap();
    let mut rows = make_policy_rows(explicit, imports);
    crate::policy_ledger::commit(
        &tx,
        &mut rows,
        creation,
        &crate::LiveCrawlPolicyLimits::default(),
        None,
    )
    .unwrap();
    tx.execute(
        "INSERT INTO metadata(key,value) VALUES('policy_initialized','true')",
        [],
    )
    .unwrap();
    for row in &rows {
        if let Some(url) = &row.scheduled_fetch_url
            && row.task_role == crate::PolicyTaskRole::SitemapDocument
        {
            enqueue(&tx, std::slice::from_ref(url));
        }
    }
    tx.commit().unwrap();
    crate::policy_ledger::verify(connection, &policy).unwrap();
}
fn ordinary_catalog(explicit: &[String], imports: &[crate::ListImportResult]) -> Connection {
    let (mut connection, sha) = fixture(Some(explicit), imports, false);
    let mut root_rows = roots(explicit, imports, &sha, false);
    let tasks = tasks(&root_rows);
    let tx = connection.transaction().unwrap();
    sitemap_ledger::initialize(&tx, &sha, &SitemapLedgerLimits::default(), false).unwrap();
    enqueue(&tx, &tasks);
    finalize_ordinary_roots(&tx, &mut root_rows, &tasks).unwrap();
    sitemap_ledger::append_roots(&tx, 1, root_rows.into_iter().map(|row| row.root).collect())
        .unwrap();
    tx.commit().unwrap();
    sitemap_ledger::load_catalog(&connection).unwrap().unwrap();
    connection
}

#[test]
fn duplicate_original_declarations_have_distinct_zero_based_ids_and_preserved_raw_order() {
    let values = explicit();
    let (_, sha) = fixture(Some(&values), &[], false);
    let root_rows = roots(&values, &[], &sha, false);
    assert_eq!(
        root_rows
            .iter()
            .map(|row| row.root.raw_value.clone())
            .collect::<Vec<_>>(),
        values
    );
    assert_eq!(
        root_rows
            .iter()
            .map(|row| row.root.source_ordinal)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_ne!(
        root_rows[0].root.declaration_id,
        root_rows[1].root.declaration_id
    );
    assert_eq!(
        root_rows[0].root.normalized_document_url,
        root_rows[1].root.normalized_document_url
    );
}

#[test]
fn explicit_import_overlap_keeps_every_original_occurrence_and_separate_kind_identity() {
    let explicit = explicit();
    let imports = imports();
    let (_, sha) = fixture(Some(&explicit), &imports, false);
    let root_rows = roots(&explicit, &imports, &sha, false);
    assert_eq!(root_rows.len(), 8);
    assert_eq!(
        root_rows[0].root.normalized_document_url,
        root_rows[3].root.normalized_document_url
    );
    assert_ne!(
        root_rows[0].root.declaration_id,
        root_rows[3].root.declaration_id
    );
    assert!(matches!(
        root_rows[0].root.kind,
        SitemapRootKind::ExplicitConfig
    ));
    assert!(matches!(
        root_rows[3].root.kind,
        SitemapRootKind::ImportedSitemapRow
    ));
}

#[test]
fn imported_flattening_excludes_page_rows_but_preserves_original_source_index_revision_raw_and_error()
 {
    let explicit = explicit();
    let imports = imports();
    let (_, sha) = fixture(Some(&explicit), &imports, true);
    let root_rows = roots(&explicit, &imports, &sha, true);
    let imported = &root_rows[3..];
    assert_eq!(
        imported
            .iter()
            .map(|row| row.root.source_ordinal)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
    assert_eq!(
        imported
            .iter()
            .map(|row| row.policy_source_ordinal)
            .collect::<Vec<_>>(),
        [1, 2, 3, 1, 2]
    );
    for (at, input_index, source_index) in [(0, 0, 1), (1, 0, 2), (2, 0, 3), (3, 2, 1), (4, 2, 2)] {
        let original = &imports[input_index].rows[source_index - 1];
        assert_eq!(imported[at].root.raw_value, original.raw_value);
        assert_eq!(
            imported[at].root.normalized_document_url,
            original.normalized_url
        );
        assert_eq!(
            imported[at].policy_source_revision.as_deref(),
            Some(
                format!(
                    "initial:{}:{input_index}:{}",
                    policy().policy_sha256(),
                    imports[input_index].input_sha256
                )
                .as_str()
            )
        );
    }
    assert!(imports[0].rows[1].error.is_some());
    assert_eq!(imported[1].root.admission.decision, D::InvalidLocation);
}

#[test]
fn legacy_none_is_unknown_and_known_empty_is_distinct_even_with_known_imports() {
    let imports = imports();
    let (_, sha) = fixture(None, &imports, false);
    assert!(initial_roots(None, &imports, &sha, None).unwrap().is_none());
    assert_eq!(
        initial_roots(Some(&[]), &imports, &sha, None)
            .unwrap()
            .unwrap()
            .len(),
        5
    );
    assert!(
        initial_roots(Some(&[]), &[], &sha, None)
            .unwrap()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn invalid_explicit_raw_values_remain_original_invalid_without_task_admission() {
    let explicit = vec![
        "".into(),
        "not a URL".into(),
        "ftp://example.org/a.xml".into(),
        "/relative.xml".into(),
    ];
    let (mut connection, sha) = fixture(Some(&explicit), &[], false);
    let mut root_rows = roots(&explicit, &[], &sha, false);
    let tx = connection.transaction().unwrap();
    finalize_ordinary_roots(&tx, &mut root_rows, &[]).unwrap();
    for (ordinal, item) in root_rows.iter().enumerate() {
        assert_eq!(item.root.raw_value, explicit[ordinal]);
        assert_eq!(item.root.admission.decision, D::InvalidLocation);
        assert!(item.root.admission.admitted_fetch_url.is_none());
    }
}

#[test]
fn forged_import_normalized_target_is_rejected_before_original_root_construction() {
    let mut imports = imports();
    imports[0].rows[0].normalized_url = Some(B.into());
    assert!(initial_roots(Some(&[]), &imports, &"a".repeat(64), None).is_err());
}

#[test]
fn ordinary_commit_keeps_all_occurrences_and_coalesces_only_task_aliases() {
    let explicit = explicit();
    let imports = imports();
    let (mut connection, sha) = fixture(Some(&explicit), &imports, false);
    let mut root_rows = roots(&explicit, &imports, &sha, false);
    let task_urls = tasks(&root_rows);
    let tx = connection.transaction().unwrap();
    sitemap_ledger::initialize(&tx, &sha, &SitemapLedgerLimits::default(), false).unwrap();
    enqueue(&tx, &task_urls);
    finalize_ordinary_roots(&tx, &mut root_rows, &task_urls).unwrap();
    assert_eq!(counts(&root_rows), (4, 3, 1));
    sitemap_ledger::append_roots(&tx, 1, root_rows.into_iter().map(|row| row.root).collect())
        .unwrap();
    tx.commit().unwrap();
    let catalog = sitemap_ledger::load_catalog(&connection).unwrap().unwrap();
    assert_eq!(catalog.summary.root_rows, 8);
    assert!(!catalog.summary.declared_source_cohort_complete);
}

#[test]
fn ordinary_existing_task_cannot_justify_mutated_original_normalized_alias() {
    let explicit = vec![A.into()];
    let imports = vec![import_xml(&[B])];
    let (mut connection, sha) = fixture(Some(&explicit), &imports, false);
    let mut root_rows = roots(&explicit, &imports, &sha, false);
    root_rows[0].root.normalized_document_url = Some(B.into());
    root_rows[0].root.admission.membership_url = Some(B.into());
    let before =
        serde_json::to_value(root_rows.iter().map(|row| &row.root).collect::<Vec<_>>()).unwrap();
    let tx = connection.transaction().unwrap();
    let error = finalize_ordinary_roots(&tx, &mut root_rows, &[A.into(), B.into()])
        .unwrap_err()
        .to_string();
    assert!(error.contains("original normalized identity"), "{error}");
    assert_eq!(
        serde_json::to_value(root_rows.iter().map(|row| &row.root).collect::<Vec<_>>()).unwrap(),
        before
    );
}

#[test]
fn ordinary_matching_raw_and_alias_still_must_match_original_config_ordinal() {
    let explicit = vec![A.into()];
    let imports = vec![import_xml(&[B])];
    let (mut connection, sha) = fixture(Some(&explicit), &imports, false);
    let mut root_rows = roots(&explicit, &imports, &sha, false);
    root_rows[0].root.raw_value = B.into();
    root_rows[0].root.normalized_document_url = Some(B.into());
    root_rows[0].root.admission.membership_url = Some(B.into());
    let tx = connection.transaction().unwrap();
    let error = finalize_ordinary_roots(&tx, &mut root_rows, &[A.into(), B.into()])
        .unwrap_err()
        .to_string();
    assert!(error.contains("source ordinal/raw"), "{error}");
}

#[test]
fn ordinary_known_import_error_cannot_be_relabelled_as_policy_denial() {
    let imports = imports();
    let (mut connection, sha) = fixture(Some(&[]), &imports, false);
    let mut root_rows = roots(&[], &imports, &sha, false);
    root_rows[1].root.admission.decision = D::PolicyDenied;
    let tx = connection.transaction().unwrap();
    let error = finalize_ordinary_roots(&tx, &mut root_rows, &[])
        .unwrap_err()
        .to_string();
    assert!(error.contains("original error/invalid marker"), "{error}");
}

#[test]
fn late_missing_ordinary_task_rolls_back_entire_catalog_initialization() {
    let explicit = vec![A.into(), B.into()];
    let (mut connection, sha) = fixture(Some(&explicit), &[], false);
    let mut root_rows = roots(&explicit, &[], &sha, false);
    {
        let tx = connection.transaction().unwrap();
        sitemap_ledger::initialize(&tx, &sha, &SitemapLedgerLimits::default(), false).unwrap();
        enqueue(&tx, &[A.into()]);
        assert!(finalize_ordinary_roots(&tx, &mut root_rows, &[A.into()]).is_err());
        // Caller discards mutated candidates and propagates the error; Drop rolls back.
    }
    assert!(sitemap_ledger::load_catalog(&connection).unwrap().is_none());
    let count: i64 = connection
        .query_row("SELECT count(*) FROM discovery_frontier", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
}

fn historical_policy_bootstrap(creation: u64, active: u64) {
    let explicit = explicit();
    let imports = imports();
    let (mut connection, sha) = fixture(Some(&explicit), &imports, true);
    commit_initial_policy(&mut connection, &explicit, &imports, creation);
    run(&connection, active, false);
    crate::policy_ledger::verify(&connection, &policy()).unwrap();
    let mut root_rows = roots(&explicit, &imports, &sha, true);
    let tx = connection.transaction().unwrap();
    sitemap_ledger::initialize(&tx, &sha, &SitemapLedgerLimits::default(), false).unwrap();
    finalize_policy_roots(&tx, &mut root_rows, active).unwrap();
    assert_eq!(counts(&root_rows), (4, 3, 1));
    for item in &root_rows {
        let id = item.root.admission.policy_occurrence_id.as_ref().unwrap();
        let generation: i64 = tx
            .query_row(
                "SELECT generation FROM policy_occurrences WHERE occurrence_id=?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(u64::try_from(generation).unwrap(), creation);
    }
    sitemap_ledger::append_roots(
        &tx,
        active,
        root_rows.into_iter().map(|row| row.root).collect(),
    )
    .unwrap();
    tx.commit().unwrap();
    let catalog = sitemap_ledger::load_catalog(&connection).unwrap().unwrap();
    assert_eq!(catalog.summary.root_rows, 8);
}
#[test]
fn interrupted_generation_one_policy_bootstrap_succeeds_during_generation_two() {
    historical_policy_bootstrap(1, 2);
}
#[test]
fn actual_initial_creation_generation_is_not_guessed_as_one_or_current_run() {
    historical_policy_bootstrap(3, 7);
}

#[test]
fn future_committed_generation_cannot_bind_current_initial_roots() {
    let explicit = vec![A.into()];
    let (mut connection, sha) = fixture(Some(&explicit), &[], true);
    commit_initial_policy(&mut connection, &explicit, &[], 3);
    run(&connection, 2, false);
    let mut root_rows = roots(&explicit, &[], &sha, true);
    let tx = connection.transaction().unwrap();
    let error = finalize_policy_roots(&tx, &mut root_rows, 2)
        .unwrap_err()
        .to_string();
    assert!(error.contains("creation generation/index"), "{error}");
}

#[test]
fn active_generation_and_terminal_guard_apply_even_when_initial_root_set_is_empty() {
    let (mut connection, _) = fixture(Some(&[]), &[], true);
    let mut empty = vec![];
    {
        let tx = connection.transaction().unwrap();
        assert!(
            finalize_policy_roots(&tx, &mut empty, 2)
                .unwrap_err()
                .to_string()
                .contains("Stale/terminal")
        );
    }
    run(&connection, 1, true);
    let tx = connection.transaction().unwrap();
    assert!(
        finalize_policy_roots(&tx, &mut empty, 1)
            .unwrap_err()
            .to_string()
            .contains("Stale/terminal")
    );
}

#[test]
fn late_policy_raw_mismatch_rolls_back_sitemap_catalog_but_preserves_committed_policy_rows() {
    let explicit = explicit();
    let imports = imports();
    let (mut connection, sha) = fixture(Some(&explicit), &imports, true);
    commit_initial_policy(&mut connection, &explicit, &imports, 1);
    run(&connection, 2, false);
    let mut root_rows = roots(&explicit, &imports, &sha, true);
    root_rows[1].root.raw_value = "https://example.org/forged.xml".into();
    let count_before: i64 = connection
        .query_row("SELECT count(*) FROM policy_occurrences", [], |row| {
            row.get(0)
        })
        .unwrap();
    {
        let tx = connection.transaction().unwrap();
        sitemap_ledger::initialize(&tx, &sha, &SitemapLedgerLimits::default(), false).unwrap();
        assert!(finalize_policy_roots(&tx, &mut root_rows, 2).is_err());
    }
    assert!(sitemap_ledger::load_catalog(&connection).unwrap().is_none());
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM policy_occurrences", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        count_before
    );
    crate::policy_ledger::verify(&connection, &policy()).unwrap();
}

// Deliberately repairs integrity checks so identity tests cannot pass merely
// because the tampered SQL broke an ordinary SHA. This is test-only, not an API.
fn rehash_same_length_root_tamper(connection: &Connection, change_raw: bool) {
    let mut statement = connection
        .prepare("SELECT sequence,data FROM sitemap_roots ORDER BY sequence")
        .unwrap();
    let mut rows: Vec<(i64, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(|row| row.unwrap())
        .collect();
    drop(statement);
    let mut row: SitemapLedgerRow = serde_json::from_str(&rows[0].1).unwrap();
    let SitemapLedgerRow::Root(root) = &mut row else {
        panic!("root fixture")
    };
    if change_raw {
        root.raw_value = B.into();
    }
    root.normalized_document_url = Some(B.into());
    root.admission.membership_url = Some(B.into());
    root.admission.admitted_fetch_url = Some(B.into());
    let changed = serde_json::to_string(&row).unwrap();
    assert_eq!(
        changed.len(),
        rows[0].1.len(),
        "fixed lengths isolate coordinate validation"
    );
    rows[0].1 = changed;
    connection
        .execute(
            "UPDATE sitemap_roots SET data=?1,data_sha256=?2 WHERE sequence=?3",
            params![rows[0].1, sitemap_ledger::digest(&rows[0].1), rows[0].0],
        )
        .unwrap();
    let mut rolling = sitemap_ledger::digest(b"open-seo-spider/sitemap-ledger-empty-v1");
    for (_, data) in rows {
        let mut hash = Sha256::new();
        hash.update(b"open-seo-spider/sitemap-ledger-row-v1\0");
        hash.update(rolling.as_bytes());
        hash.update(4u64.to_be_bytes());
        hash.update(b"root");
        hash.update((data.len() as u64).to_be_bytes());
        hash.update(data.as_bytes());
        rolling = format!("{:x}", hash.finalize());
    }
    let text: String = connection
        .query_row(
            "SELECT value FROM metadata WHERE key='sitemap_ledger'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut summary: SitemapLedgerSummary = serde_json::from_str(&text).unwrap();
    summary.rolling_sha256 = rolling;
    // Counts, IDs, source ordinals and serialized catalog byte length are unchanged.
    connection
        .execute(
            "UPDATE metadata SET value=?1 WHERE key='sitemap_ledger'",
            [serde_json::to_string(&summary).unwrap()],
        )
        .unwrap();
}

#[test]
fn ordinary_durable_reader_rejects_fully_rehashed_normalized_alias_tamper() {
    let connection = ordinary_catalog(&[A.into()], &[import_xml(&[B])]);
    rehash_same_length_root_tamper(&connection, false);
    let error = sitemap_ledger::load_catalog(&connection)
        .unwrap_err()
        .to_string();
    assert!(error.contains("original normalized identity"), "{error}");
}
#[test]
fn ordinary_durable_reader_rejects_fully_rehashed_raw_ordinal_tamper() {
    let connection = ordinary_catalog(&[A.into()], &[import_xml(&[B])]);
    rehash_same_length_root_tamper(&connection, true);
    let error = sitemap_ledger::load_catalog(&connection)
        .unwrap_err()
        .to_string();
    assert!(error.contains("source ordinal/raw"), "{error}");
}
