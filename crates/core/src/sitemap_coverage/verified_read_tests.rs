//! Authored actual Store/decoder/parser/SQLite controls; NOT_RUN under HOLD.
use super::*;
use crate::sitemap_ledger::verified_read::verify_raw;
use crate::store::{DiscoveryEntry, DiscoveryKind, SitemapCatalogInit};
const ORIGIN: &str = "https://example.org/";
const ROOT: &str = "https://example.org/root.xml";
const CHILD: &str = "https://example.org/child.xml";

struct Fixture {
    temp: tempfile::TempDir,
    store: crate::CrawlStore,
    generation: u64,
}
impl Fixture {
    fn new(originals: Option<Vec<String>>, robots: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let store = crate::CrawlStore::open(temp.path().join("crawl.sqlite")).unwrap();
        let config = CrawlConfig {
            seed_urls: vec![ORIGIN.into()],
            explicit_sitemap_inputs: originals.clone(),
            sitemap_urls: originals
                .clone()
                .unwrap_or_default()
                .iter()
                .filter_map(|v| crate::normalize_url(v, None))
                .map(|v| v.to_string())
                .collect(),
            obey_robots: robots,
            ..Default::default()
        };
        store.prepare(&config).unwrap();
        let generation = store.begin_run("compact-coverage").unwrap();
        let connection = Connection::open(temp.path().join("crawl.sqlite")).unwrap();
        let saved: Vec<u8> = connection
            .query_row(
                "SELECT CAST(value AS BLOB) FROM metadata WHERE key='config'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let sha = crate::sitemap_ledger::digest(saved);
        let mut seen = BTreeSet::new();
        let roots =
            crate::sitemap_initial_roots::initial_roots(originals.as_deref(), &[], &sha, None)
                .unwrap()
                .unwrap_or_default()
                .into_iter()
                .map(|mut item| {
                    if let Some(url) = &item.root.normalized_document_url {
                        item.root.admission.decision = if seen.insert(url.clone()) {
                            Admission::Scheduled
                        } else {
                            Admission::Coalesced
                        };
                        item.root.admission.admitted_fetch_url = Some(url.clone());
                    }
                    item.root
                })
                .collect();
        let mut entries = config
            .sitemap_urls
            .iter()
            .map(|url| DiscoveryEntry {
                kind: DiscoveryKind::Sitemap,
                url: url.clone(),
            })
            .collect::<Vec<_>>();
        if robots {
            entries.push(DiscoveryEntry {
                kind: DiscoveryKind::Robots,
                url: ORIGIN.into(),
            });
        }
        store
            .initialize_sitemap_discovery(
                &entries,
                generation,
                &SitemapCatalogInit {
                    config_sha256: sha,
                    limits: Default::default(),
                    roots,
                    legacy_history_unknown: originals.is_none(),
                },
            )
            .unwrap();
        Self {
            temp,
            store,
            generation,
        }
    }
    fn connection(&self) -> Connection {
        Connection::open(self.temp.path().join("crawl.sqlite")).unwrap()
    }
    fn batch(&self, task: &str, xml: &str) -> crate::SitemapDiscoveryBatch {
        self.batch_with_status(task, xml, 200)
    }
    fn batch_with_status(
        &self,
        task: &str,
        xml: &str,
        status: u16,
    ) -> crate::SitemapDiscoveryBatch {
        let limits = crate::SitemapParserLimits::default();
        let decoded = crate::decode_sitemap_entity(xml.as_bytes(), &limits).unwrap();
        let observation = crate::observe_sitemap_xml(
            &decoded.xml,
            crate::SitemapParserContext {
                requested_document_url: Some(task.into()),
                final_document_url: Some(task.into()),
                originating_occurrence_id: None,
            },
            &limits,
        )
        .unwrap();
        let sha = crate::sitemap_ledger::digest(xml);
        let mut batch = crate::sitemap_discovery::xml_batch(
            task,
            self.generation,
            &sha,
            crate::SitemapTransportFacts {
                request_started: true,
                requested_url: Some(task.into()),
                final_url: Some(task.into()),
                initial_status: Some(status),
                final_status: Some(status),
                blocked_by_robots: false,
                body_complete: true,
                post_http_decoding_bytes: xml.len() as u64,
                post_http_decoding_sha256: Some(sha.clone()),
                retained_post_http_prefix_sha256: Some(sha.clone()),
                failure_code: None,
            },
            Some(&decoded),
            Some(observation),
            None,
        )
        .unwrap();
        for loc in &mut batch.locations {
            if loc.admission.decision == Admission::Unbound {
                loc.admission.decision = Admission::Scheduled;
                loc.admission.admitted_fetch_url = loc.observed.normalized_fetch_url.clone();
                if loc.observed.role == crate::SitemapLocationRole::ChildSitemap {
                    loc.referenced_sitemap_task = loc.admission.admitted_fetch_url.clone();
                }
            }
        }
        batch
    }
    fn xml(&self, task: &str, xml: &str, children: &[&str]) {
        self.store
            .complete_discovery_observed(
                &DiscoveryEntry {
                    kind: DiscoveryKind::Sitemap,
                    url: task.into(),
                },
                &[],
                &children
                    .iter()
                    .map(|v| DiscoveryEntry {
                        kind: DiscoveryKind::Sitemap,
                        url: (*v).into(),
                    })
                    .collect::<Vec<_>>(),
                &[],
                self.generation,
                self.batch(task, xml),
            )
            .unwrap();
    }
}

fn context_values(connection: &Connection) -> (Option<Vec<u8>>, Option<String>, Option<u64>, bool) {
    let bytes: Option<Vec<u8>> = connection
        .query_row(
            "SELECT CAST(value AS BLOB) FROM metadata WHERE key='config'",
            [],
            |r| r.get(0),
        )
        .optional()
        .unwrap();
    let sha = bytes.as_ref().map(crate::sitemap_ledger::digest);
    let run: Option<crate::CrawlRunState> = scalar_text(connection, "run_state")
        .unwrap()
        .map(|s| serde_json::from_str(&s).unwrap());
    (
        bytes,
        sha,
        run.map(|r| r.generation).filter(|n| *n > 0),
        scalar_text(connection, "discovery_initialized")
            .unwrap()
            .as_deref()
            == Some("true"),
    )
}

fn compact(connection: &Connection) -> VerifiedCoverageRead {
    let (bytes, sha, generation, discovery_initialized) = context_values(connection);
    let raw = verify_raw(connection).unwrap();
    project_read(
        connection,
        raw.as_ref(),
        CoverageReadContext {
            saved_config_bytes: bytes.as_deref(),
            saved_config_sha256: sha.as_deref(),
            generation,
            discovery_initialized,
        },
    )
    .unwrap()
}

/// Test oracle intentionally materializes the OLD actual inventory on the SAME
/// connection/transaction, not a fabricated positive DTO or a separate read.
fn oracle(connection: &Connection) -> (SitemapCoverage, Option<SitemapCoverageInventory>) {
    let catalog = crate::sitemap_ledger::load_catalog(connection).unwrap();
    let (bytes, sha, generation, initialized) = context_values(connection);
    let inventory = match (&catalog, bytes.as_deref(), sha.as_deref(), generation) {
        (Some(catalog), Some(bytes), Some(sha), Some(generation)) => {
            let tasks = if task_table(connection).unwrap() {
                connection
                    .prepare("SELECT kind,url,done FROM discovery_frontier")
                    .unwrap()
                    .query_map([], |r| {
                        let kind: String = r.get(0)?;
                        Ok(SitemapCoverageTask {
                            kind: match kind.as_str() {
                                "robots" => SitemapSourceKind::RobotsDirectives,
                                "sitemap" => SitemapSourceKind::SitemapXml,
                                _ => panic!("oracle invalid kind"),
                            },
                            task_url: r.get(1)?,
                            done: r.get::<_, i64>(2)? == 1,
                        })
                    })
                    .unwrap()
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .unwrap()
            } else {
                vec![]
            };
            let config: CrawlConfig = serde_json::from_slice(bytes).unwrap();
            let policy_urls = if config.crawl_policy.is_some() {
                Some(connection.prepare("SELECT json_extract(data,'$.occurrence.decision.fetch_candidate_url') FROM policy_occurrences WHERE json_extract(data,'$.occurrence.input_basis')='declared_input' AND json_extract(data,'$.occurrence.task_role')='page_document' AND json_extract(data,'$.occurrence.decision.source_kind') IN ('seed','list_page') AND json_extract(data,'$.occurrence.admission_reason') IN ('scheduled','coalesced') ORDER BY sequence").unwrap().query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<std::result::Result<Vec<_>,_>>().unwrap())
            } else {
                None
            };
            Some(
                SitemapCoverageInventory::from_saved_snapshot(
                    catalog,
                    bytes,
                    sha,
                    generation,
                    initialized,
                    tasks,
                    policy_urls.as_deref(),
                )
                .unwrap(),
            )
        }
        _ => None,
    };
    (
        project_sitemap_coverage(catalog.as_ref(), inventory.as_ref()).unwrap(),
        inventory,
    )
}

fn agrees(connection: &Connection) -> VerifiedCoverageRead {
    let result = compact(connection);
    let (old, inventory) = oracle(connection);
    assert_eq!(result.coverage, old);
    assert_eq!(
        result.inventory_sha256,
        inventory
            .as_ref()
            .map(|v| crate::sitemap_ledger::digest(serde_json::to_vec(v).unwrap()))
    );
    assert!(!result.materializing_selected_compatibility);
    result
}

#[test]
fn initialized_empty_pending_robots_and_legacy_unknown_match_exact_old_reasons() {
    for (originals, robots, completion) in [
        (Some(vec![]), false, SitemapCohortCompletion::KnownEmpty),
        (Some(vec![]), true, SitemapCohortCompletion::Unknown),
        (None, false, SitemapCohortCompletion::Unknown),
    ] {
        let f = Fixture::new(originals, robots);
        let mut c = f.connection();
        let tx = c.transaction().unwrap();
        let result = agrees(&tx);
        assert_eq!(result.coverage.completion, completion);
        if robots {
            assert_eq!(
                result
                    .coverage
                    .reason_counts
                    .get(&SitemapCoverageReason::PendingSource),
                Some(&1)
            );
        }
    }
}

#[test]
fn recursive_cycle_duplicate_roots_and_invalid_page_locations_match_full_oracle() {
    let f = Fixture::new(Some(vec![ROOT.into(), format!("{ROOT}#duplicate")]), false);
    f.xml(ROOT,&format!("<sitemapindex><sitemap><loc>{CHILD}</loc></sitemap><sitemap><loc>{CHILD}</loc></sitemap></sitemapindex>"),&[CHILD]);
    f.xml(
        CHILD,
        &format!("<sitemapindex><sitemap><loc>{ROOT}</loc></sitemap></sitemapindex>"),
        &[],
    );
    let mut c = f.connection();
    let tx = c.transaction().unwrap();
    let result = agrees(&tx);
    assert_eq!(
        result.coverage.completion,
        SitemapCohortCompletion::Complete
    );
    assert_eq!(result.coverage.counts.reached_xml_tasks, 2);
    assert!(
        result
            .coverage
            .current_valid_original_location_negative_permitted
    );
    drop(tx);
    let f = Fixture::new(Some(vec![ROOT.into()]), false);
    f.xml(
        ROOT,
        "<urlset><url/><url><loc>relative</loc></url></urlset>",
        &[],
    );
    let mut c = f.connection();
    let tx = c.transaction().unwrap();
    let result = agrees(&tx);
    assert_eq!(result.coverage.counts.invalid_or_missing_page_locations, 2);
    assert_eq!(
        result.coverage.completion,
        SitemapCohortCompletion::Complete
    );
}

#[test]
fn denied_invalid_and_incomplete_child_sources_preserve_exact_reason_multiplicity() {
    for denied in [false, true] {
        let f = Fixture::new(Some(vec![ROOT.into()]), false);
        let xml = format!(
            "<sitemapindex><sitemap><loc>{CHILD}</loc></sitemap><sitemap><loc>relative</loc></sitemap></sitemapindex>"
        );
        let mut batch = f.batch(ROOT, &xml);
        if denied {
            batch.locations[0].admission.decision = Admission::FilterDenied;
            batch.locations[0].admission.admitted_fetch_url = None;
            batch.locations[0].referenced_sitemap_task = None;
        }
        f.store
            .complete_discovery_observed(
                &DiscoveryEntry {
                    kind: DiscoveryKind::Sitemap,
                    url: ROOT.into(),
                },
                &[],
                &if denied {
                    vec![]
                } else {
                    vec![DiscoveryEntry {
                        kind: DiscoveryKind::Sitemap,
                        url: CHILD.into(),
                    }]
                },
                &[],
                f.generation,
                batch,
            )
            .unwrap();
        let mut c = f.connection();
        let tx = c.transaction().unwrap();
        let result = agrees(&tx);
        assert_eq!(result.coverage.completion, SitemapCohortCompletion::Unknown);
        assert_eq!(
            result
                .coverage
                .reason_counts
                .get(&SitemapCoverageReason::ChildInvalid),
            Some(&1)
        );
        if denied {
            assert_eq!(
                result
                    .coverage
                    .reason_counts
                    .get(&SitemapCoverageReason::ChildDenied),
                Some(&1)
            );
        } else {
            assert_eq!(
                result
                    .coverage
                    .reason_counts
                    .get(&SitemapCoverageReason::PendingSource),
                Some(&1)
            );
        }
    }
    let f = Fixture::new(Some(vec![ROOT.into()]), false);
    f.xml(
        ROOT,
        "<sitemapindex><sitemap><loc>https://example.org/child.xml</loc></sitemap>",
        &[],
    );
    let mut c = f.connection();
    let tx = c.transaction().unwrap();
    let result = agrees(&tx);
    assert!(
        result
            .coverage
            .reason_counts
            .contains_key(&SitemapCoverageReason::SourceParserIncomplete)
    );
    assert!(
        result
            .coverage
            .reason_counts
            .contains_key(&SitemapCoverageReason::ChildUnbound)
    );
}

#[test]
fn last_pending_task_beyond_page_size_changes_exact_inventory_hash_and_reasons() {
    let f = Fixture::new(Some(vec![]), false);
    let mut c = f.connection();
    let first = {
        let tx = c.transaction().unwrap();
        agrees(&tx).inventory_sha256
    };
    {
        let tx = c.transaction().unwrap();
        for i in 0..1201 {
            tx.execute(
                "INSERT INTO discovery_frontier(kind,url,done) VALUES('sitemap',?1,0)",
                [format!("https://example.org/z{i:04}.xml")],
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }
    let tx = c.transaction().unwrap();
    let result = agrees(&tx);
    assert_ne!(result.inventory_sha256, first);
    assert_eq!(result.coverage.counts.discovery_tasks, 1201);
    assert_eq!(
        result
            .coverage
            .reason_counts
            .get(&SitemapCoverageReason::PendingSource),
        Some(&1201)
    );
    assert_eq!(
        result
            .coverage
            .reason_counts
            .get(&SitemapCoverageReason::SourceGraphUnboundTask),
        Some(&1201)
    );
}

#[test]
fn actual_config_bytes_generation_and_discovery_marker_are_checked_not_reserialized() {
    let f = Fixture::new(Some(vec![]), false);
    let mut c = f.connection();
    c.execute(
        "UPDATE metadata SET value=char(10)||value||'  ' WHERE key='config'",
        [],
    )
    .unwrap();
    let tx = c.transaction().unwrap();
    let (bytes, sha, generation, initialized) = context_values(&tx);
    let raw = verify_raw(&tx).unwrap();
    assert_eq!(
        agrees(&tx)
            .coverage
            .basis
            .unwrap()
            .inventory_saved_config_sha256,
        sha
    );
    for (wrong_bytes, wrong_sha, wrong_generation, wrong_initialized) in [
        (
            Some(b"{}".as_slice()),
            Some(crate::sitemap_ledger::digest(b"{}")),
            generation,
            initialized,
        ),
        (
            bytes.as_deref(),
            Some("b".repeat(64)),
            generation,
            initialized,
        ),
        (
            bytes.as_deref(),
            sha.clone(),
            Some(generation.unwrap() + 1),
            initialized,
        ),
        (bytes.as_deref(), sha.clone(), generation, !initialized),
    ] {
        assert!(
            project_read(
                &tx,
                raw.as_ref(),
                CoverageReadContext {
                    saved_config_bytes: wrong_bytes,
                    saved_config_sha256: wrong_sha.as_deref(),
                    generation: wrong_generation,
                    discovery_initialized: wrong_initialized
                }
            )
            .is_err()
        );
    }
}

#[test]
fn same_transaction_wal_keeps_config_tasks_and_generation_together() {
    let f = Fixture::new(Some(vec![]), false);
    let mut reader = f.connection();
    reader.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
    let tx = reader.transaction().unwrap();
    let before = agrees(&tx);
    let mut writer = f.connection();
    let write = writer.transaction().unwrap();
    let mut config: CrawlConfig =
        serde_json::from_slice(&context_values(&write).0.unwrap()).unwrap();
    config.max_pages = Some(2);
    write
        .execute(
            "UPDATE metadata SET value=?1 WHERE key='config'",
            [serde_json::to_string(&config).unwrap()],
        )
        .unwrap();
    let mut run: crate::CrawlRunState =
        serde_json::from_str(&scalar_text(&write, "run_state").unwrap().unwrap()).unwrap();
    run.generation += 1;
    write
        .execute(
            "UPDATE metadata SET value=?1 WHERE key='run_state'",
            [serde_json::to_string(&run).unwrap()],
        )
        .unwrap();
    write.execute("INSERT INTO discovery_frontier(kind,url,done) VALUES('robots','https://other.example/',0)",[]).unwrap();
    write.commit().unwrap();
    assert_eq!(agrees(&tx).coverage, before.coverage);
    assert_eq!(agrees(&tx).inventory_sha256, before.inventory_sha256);
    tx.commit().unwrap();
    let tx = reader.transaction().unwrap();
    let after = agrees(&tx);
    assert_ne!(after.inventory_sha256, before.inventory_sha256);
    assert_eq!(
        after.coverage.basis.unwrap().inventory_generation,
        Some(f.generation + 1)
    );
}

#[test]
fn task_sort_uses_rust_binary_identity_and_rejects_duplicates_or_bad_tail_flags() {
    let f = Fixture::new(Some(vec![]), false);
    let mut c = f.connection();
    c.execute_batch("ALTER TABLE discovery_frontier RENAME TO old_discovery;CREATE TABLE discovery_frontier(kind TEXT COLLATE NOCASE,url TEXT COLLATE NOCASE,done INTEGER);DROP TABLE old_discovery;INSERT INTO discovery_frontier VALUES('sitemap','https://example.org/A.xml',0),('sitemap','https://example.org/a.xml',1);").unwrap();
    {
        let tx = c.transaction().unwrap();
        let result = agrees(&tx);
        assert_eq!(result.coverage.counts.discovery_tasks, 2);
    }
    c.execute(
        "INSERT INTO discovery_frontier VALUES('sitemap','https://example.org/a.xml',0)",
        [],
    )
    .unwrap();
    {
        let tx = c.transaction().unwrap();
        let (bytes, sha, generation, discovery_initialized) = context_values(&tx);
        let raw = verify_raw(&tx).unwrap();
        assert!(
            project_read(
                &tx,
                raw.as_ref(),
                CoverageReadContext {
                    saved_config_bytes: bytes.as_deref(),
                    saved_config_sha256: sha.as_deref(),
                    generation,
                    discovery_initialized
                }
            )
            .is_err()
        );
    }
    c.execute("DELETE FROM discovery_frontier WHERE done=0", [])
        .unwrap();
    c.execute("UPDATE discovery_frontier SET done=7", [])
        .unwrap();
    let tx = c.transaction().unwrap();
    let (bytes, sha, generation, discovery_initialized) = context_values(&tx);
    let raw = verify_raw(&tx).unwrap();
    assert!(
        project_read(
            &tx,
            raw.as_ref(),
            CoverageReadContext {
                saved_config_bytes: bytes.as_deref(),
                saved_config_sha256: sha.as_deref(),
                generation,
                discovery_initialized
            }
        )
        .is_err()
    );
}

#[test]
fn missing_context_remains_unknown_and_connection_proofs_cannot_be_substituted() {
    let f = Fixture::new(Some(vec![]), false);
    let mut c = f.connection();
    c.execute(
        "DELETE FROM metadata WHERE key IN ('config','run_state')",
        [],
    )
    .unwrap();
    {
        let tx = c.transaction().unwrap();
        let result = agrees(&tx);
        assert!(result.inventory_sha256.is_none());
        assert!(
            result
                .coverage
                .reason_counts
                .contains_key(&SitemapCoverageReason::SourceInventoryUnavailable)
        );
    }
    let other = Fixture::new(Some(vec![]), false);
    let mut other_connection = other.connection();
    let other_tx = other_connection.transaction().unwrap();
    let other_raw = verify_raw(&other_tx).unwrap();
    let tx = c.transaction().unwrap();
    assert!(
        project_read(
            &tx,
            other_raw.as_ref(),
            CoverageReadContext {
                saved_config_bytes: None,
                saved_config_sha256: None,
                generation: None,
                discovery_initialized: true
            }
        )
        .is_err()
    );
    drop(tx);
    assert!(
        project_read(
            &c,
            None,
            CoverageReadContext {
                saved_config_bytes: None,
                saved_config_sha256: None,
                generation: None,
                discovery_initialized: true
            }
        )
        .is_err()
    );
}

#[test]
fn real_policy_initial_page_admissions_determine_required_robots_origins() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("crawl.sqlite");
    let store = crate::CrawlStore::open(&path).unwrap();
    let mut policy = crate::CrawlPolicyConfig::default();
    policy.scope.roots.push(crate::CrawlScopeRoot {
        origin: "https://example.org".into(),
        subdomains: false,
        folders: vec!["/".into()],
        exact_paths: vec![],
    });
    let config = CrawlConfig {
        seed_urls: vec![ORIGIN.into()],
        explicit_sitemap_inputs: Some(vec![]),
        crawl_policy: Some(policy.clone()),
        obey_robots: true,
        ..Default::default()
    };
    store.prepare(&config).unwrap();
    let generation = store.begin_run("compact-policy-origins").unwrap();
    let compiled = crate::CompiledCrawlPolicy::new(&policy).unwrap();
    let mut rows = vec![crate::policy_ledger::decide(
        &compiled,
        "initial-coverage-seed",
        0,
        crate::CandidateSourceKind::Seed,
        ORIGIN.into(),
        None,
        None,
        crate::PolicyInputBasis::DeclaredInput,
        crate::PolicyTaskRole::PageDocument,
    )];
    store
        .initialize_policy(&compiled, &mut rows, &Default::default(), generation)
        .unwrap();
    let mut c = Connection::open(path).unwrap();
    let saved: Vec<u8> = c
        .query_row(
            "SELECT CAST(value AS BLOB) FROM metadata WHERE key='config'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    store
        .initialize_sitemap_discovery(
            &[],
            generation,
            &SitemapCatalogInit {
                config_sha256: crate::sitemap_ledger::digest(saved),
                limits: Default::default(),
                roots: vec![],
                legacy_history_unknown: false,
            },
        )
        .unwrap();
    let tx = c.transaction().unwrap();
    let result = agrees(&tx);
    assert_eq!(
        result
            .coverage
            .reason_counts
            .get(&SitemapCoverageReason::RequiredRobotsTaskMissing),
        Some(&1)
    );
    assert_eq!(
        expected_robots(&tx, &config).unwrap(),
        vec![ORIGIN.to_string()]
    );
}

#[test]
fn read_only_projection_preserves_saved_database_and_wal_bytes() {
    let f = Fixture::new(Some(vec![ROOT.into()]), false);
    f.xml(ROOT, "<urlset/>", &[]);
    let path = f.temp.path().join("crawl.sqlite");
    let wal = f.temp.path().join("crawl.sqlite-wal");
    let before = std::fs::read(&path).unwrap();
    let wal_before = std::fs::read(&wal).ok();
    {
        let mut c =
            Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let tx = c.transaction().unwrap();
        let result = agrees(&tx);
        assert_eq!(
            result.coverage.completion,
            SitemapCohortCompletion::Complete
        );
        tx.commit().unwrap();
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read(&wal).ok(), wal_before);
}

#[test]
fn inherited_actual_parent_inventory_keeps_historical_scope_and_legacy_none_unknown() {
    for has_inventory in [false, true] {
        let f = Fixture::new(Some(vec![ROOT.into()]), false);
        f.xml(ROOT, "<urlset/>", &[]);
        let mut c = f.connection();
        let tx = c.transaction().unwrap();
        let (coverage, inventory) = oracle(&tx);
        assert_eq!(coverage.completion, SitemapCohortCompletion::Complete);
        let parent = crate::CrawlReport {
            config: serde_json::from_slice(&context_values(&tx).0.unwrap()).unwrap(),
            pages: vec![],
            issues: vec![],
            warnings: vec![],
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
            sitemap_catalog: crate::sitemap_ledger::load_catalog(&tx).unwrap(),
            sitemap_coverage_inventory: if has_inventory { inventory } else { None },
            sitemap_source_measurements: None,
            summary: Default::default(),
        };
        let bytes = serde_json::to_vec(&parent).unwrap();
        let sha = crate::sitemap_ledger::digest(&bytes);
        tx.commit().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let child = crate::CrawlStore::open(temp.path().join("child.sqlite")).unwrap();
        child.prepare(&parent.config).unwrap();
        child.begin_run("selected-low-level-compatibility").unwrap();
        let mut child_connection = Connection::open(temp.path().join("child.sqlite")).unwrap();
        let tx = child_connection.transaction().unwrap();
        tx.execute(
            "INSERT INTO recrawl_base(id,snapshot) VALUES(1,?1)",
            [&bytes],
        )
        .unwrap();
        tx.execute("INSERT INTO metadata(key,value) VALUES('run_plan',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::json!({"kind":"selected_recrawl","plan":{"engine":crate::SELECTED_RECRAWL_ENGINE,"parent":{"snapshot_sha256":sha}}}).to_string()]).unwrap();
        // This is the low-level exact parent/catalog/inventory compatibility
        // control. Full Store selected-plan guards are a separate required gate.
        crate::sitemap_ledger::inherit(&tx, parent.sitemap_catalog.as_ref().unwrap(), &sha)
            .unwrap();
        let result = compact(&tx);
        assert!(result.materializing_selected_compatibility);
        assert_eq!(
            result.coverage.scope,
            SitemapCoverageScope::InheritedHistoricalDeclaredSources
        );
        assert!(
            !result
                .coverage
                .current_valid_original_location_negative_permitted
        );
        assert_eq!(
            result
                .coverage
                .historical_valid_original_location_negative_permitted,
            has_inventory
        );
        assert_eq!(result.inventory_sha256.is_some(), has_inventory);
    }
}

#[test]
fn root_visitor_preserves_portable_constructor_items_unknown_and_callback_failure() {
    let originals = vec![ROOT.into(), "invalid".into(), format!("{ROOT}#other")];
    let sha = "a".repeat(64);
    let old = crate::sitemap_initial_roots::initial_roots(Some(&originals), &[], &sha, None)
        .unwrap()
        .unwrap();
    let mut streamed = vec![];
    assert!(
        crate::sitemap_initial_roots::visit_initial_roots(Some(&originals), &[], &sha, None, |v| {
            streamed.push(v.root);
            Ok(())
        })
        .unwrap()
    );
    assert_eq!(
        streamed,
        old.into_iter().map(|v| v.root).collect::<Vec<_>>()
    );
    assert!(
        !crate::sitemap_initial_roots::visit_initial_roots(
            None,
            &[],
            "unobserved-invalid-sha",
            None,
            |_| panic!("unknown must not visit")
        )
        .unwrap()
    );
    let mut called = 0;
    let result = crate::sitemap_initial_roots::visit_initial_roots(
        Some(&originals),
        &[],
        &sha,
        None,
        |_| {
            called += 1;
            anyhow::bail!("stop visitor")
        },
    );
    assert!(result.is_err());
    assert_eq!(called, 1);
}

#[test]
fn actual_empty_robots_completion_and_http_206_source_differ_from_pending_inventory() {
    let f = Fixture::new(Some(vec![]), true);
    let mut buffer = crate::RobotsBodyBuffer::default();
    buffer.observe_eof();
    let source = crate::RobotsSitemapSource::observed(
        format!("{ORIGIN}robots.txt"),
        format!("{ORIGIN}robots.txt"),
        200,
        &buffer,
        &url::Url::parse(ORIGIN).unwrap(),
    );
    let batch = crate::sitemap_discovery::robots_batch(
        ORIGIN,
        f.generation,
        &crate::sitemap_ledger::digest([]),
        &source,
    )
    .unwrap();
    f.store
        .complete_discovery_observed(
            &DiscoveryEntry {
                kind: DiscoveryKind::Robots,
                url: ORIGIN.into(),
            },
            &[],
            &[],
            &[],
            f.generation,
            batch,
        )
        .unwrap();
    {
        let mut c = f.connection();
        let tx = c.transaction().unwrap();
        let result = agrees(&tx);
        assert_eq!(
            result.coverage.completion,
            SitemapCohortCompletion::KnownEmpty
        );
        assert_eq!(result.coverage.counts.complete_documents, 1);
    }
    // Wrapper/model integration control with explicitly recorded HTTP206 facts;
    // it is not evidence that a native HTTP client fetched this fixture.
    let f = Fixture::new(Some(vec![ROOT.into()]), false);
    let batch = f.batch_with_status(
        ROOT,
        "<urlset><url><loc>https://example.org/p</loc></url></urlset>",
        206,
    );
    f.store
        .complete_discovery_observed(
            &DiscoveryEntry {
                kind: DiscoveryKind::Sitemap,
                url: ROOT.into(),
            },
            &[],
            &[],
            &[],
            f.generation,
            batch,
        )
        .unwrap();
    let mut c = f.connection();
    let tx = c.transaction().unwrap();
    let result = agrees(&tx);
    assert_eq!(result.coverage.completion, SitemapCohortCompletion::Unknown);
    assert_eq!(
        result
            .coverage
            .reason_counts
            .get(&SitemapCoverageReason::SourceHttpNot200),
        Some(&1)
    );
}

#[test]
fn actual_uninitialized_store_and_corrupt_raw_tail_cannot_manufacture_a_complete_projection() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("not-initialized.sqlite");
    let store = crate::CrawlStore::open(&path).unwrap();
    store
        .prepare(&CrawlConfig {
            explicit_sitemap_inputs: Some(vec![]),
            obey_robots: false,
            ..Default::default()
        })
        .unwrap();
    store.begin_run("not-initialized").unwrap();
    let mut c = Connection::open(path).unwrap();
    let tx = c.transaction().unwrap();
    let result = agrees(&tx);
    assert!(result.coverage.basis.is_none());
    assert!(result.inventory_sha256.is_none());
    assert_eq!(
        result
            .coverage
            .reason_counts
            .get(&SitemapCoverageReason::CatalogUnavailable),
        Some(&1)
    );
    drop(tx);
    let f = Fixture::new(Some(vec![ROOT.into()]), false);
    f.xml(
        ROOT,
        "<urlset><url><loc>https://example.org/p</loc></url></urlset>",
        &[],
    );
    let mut c = f.connection();
    c.execute("UPDATE sitemap_locations SET data=data||' ' WHERE sequence=(SELECT max(sequence) FROM sitemap_locations)",[]).unwrap();
    let tx = c.transaction().unwrap();
    assert!(verify_raw(&tx).is_err());
    assert!(crate::sitemap_ledger::load_catalog(&tx).is_err());
}

#[test]
fn graph_identity_budget_is_explicit_and_duplicate_reachability_never_charges_twice() {
    let mut reached = BTreeSet::new();
    let mut queue = VecDeque::new();
    let mut bytes = MAX_INVENTORY_TASK_BYTES;
    assert!(enqueue(&mut reached, &mut queue, &mut bytes, "x").is_err());
    assert!(reached.is_empty());
    assert!(queue.is_empty());
    let mut bytes = 0;
    enqueue(&mut reached, &mut queue, &mut bytes, ROOT).unwrap();
    let charged = bytes;
    enqueue(&mut reached, &mut queue, &mut bytes, ROOT).unwrap();
    assert_eq!(bytes, charged);
    assert_eq!(reached.len(), 1);
    assert_eq!(queue.len(), 1);
}

// SOURCE-ONLY STATELESS OPTIMIZATION REGRESSIONS. NOT_COMPILED / NOT_RUN.
#[test]
fn stateless_row_coverage_reuses_verified_counts_without_another_raw_visit_and_matches_old_projection()
 {
    for legacy in [false, true] {
        let f = Fixture::new(
            if legacy {
                None
            } else {
                Some(vec![ROOT.into()])
            },
            false,
        );
        if !legacy {
            f.xml(
                ROOT,
                &format!("<sitemapindex><sitemap><loc>{CHILD}</loc></sitemap></sitemapindex>"),
                &[CHILD],
            );
            f.xml(CHILD, "<urlset><url/><url><loc>relative</loc></url><url><loc>https://example.org/p</loc></url></urlset>", &[]);
        }
        let mut connection = f.connection();
        let tx = connection.transaction().unwrap();
        let raw = verify_raw(&tx).unwrap().unwrap();
        let work = raw.verification_work();
        let mut counted = empty_coverage();
        count_rows(&raw, &mut counted, &SitemapReadControl::default()).unwrap();
        assert_eq!(
            raw.verification_work(),
            work,
            "coverage count reuse must not execute another raw traversal"
        );
        let (bytes, sha, generation, initialized) = context_values(&tx);
        let result = project_read(
            &tx,
            Some(&raw),
            CoverageReadContext {
                saved_config_bytes: bytes.as_deref(),
                saved_config_sha256: sha.as_deref(),
                generation,
                discovery_initialized: initialized,
            },
        )
        .unwrap();
        let (old, inventory) = oracle(&tx);
        assert_eq!(result.coverage, old);
        assert_eq!(
            result.inventory_sha256,
            inventory
                .as_ref()
                .map(|value| crate::sitemap_ledger::digest(serde_json::to_vec(value).unwrap()))
        );
        assert_eq!(counted.counts.root_declarations, raw.summary().root_rows);
        assert_eq!(
            counted.counts.observed_documents,
            raw.summary().document_rows
        );
        if !legacy {
            assert_eq!(counted.counts.child_declarations, 1);
            assert_eq!(counted.counts.invalid_or_missing_page_locations, 2);
        }
    }
}
