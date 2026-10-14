use super::*;
use crate::{SitemapCohortCompletion as Completion, SitemapCoverageReason as Reason};

fn fixture(robots: bool) -> (tempfile::TempDir, CrawlStore, u64) {
    let temp = tempfile::tempdir().unwrap();
    let store = CrawlStore::open(temp.path().join("crawl.sqlite")).unwrap();
    let config = CrawlConfig {
        seed_urls: vec!["https://example.org/".into()],
        explicit_sitemap_inputs: Some(vec![]),
        obey_robots: robots,
        ..Default::default()
    };
    store.prepare(&config).unwrap();
    let generation = store.begin_run("coverage-store-fixture").unwrap();
    let saved = metadata_text(&store.connection.lock().unwrap(), "config")
        .unwrap()
        .unwrap();
    let entries = if robots {
        vec![DiscoveryEntry {
            kind: DiscoveryKind::Robots,
            url: "https://example.org/".into(),
        }]
    } else {
        vec![]
    };
    store
        .initialize_sitemap_discovery(
            &entries,
            generation,
            &SitemapCatalogInit {
                config_sha256: sha256(&saved),
                limits: Default::default(),
                roots: vec![],
                legacy_history_unknown: false,
            },
        )
        .unwrap();
    (temp, store, generation)
}
fn portable(store: &CrawlStore) -> CrawlReport {
    let snapshot = store.sitemap_read_snapshot().unwrap();
    CrawlReport {
        config: store.config().unwrap().unwrap(),
        pages: vec![],
        issues: vec![],
        warnings: vec![],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: snapshot.catalog,
        sitemap_coverage_inventory: snapshot.coverage_inventory,
        sitemap_source_measurements: snapshot.sitemap_source_measurements,
        summary: Default::default(),
    }
}

#[test]
fn readonly_capture_preserves_every_committed_sql_row_and_actual_config_byte_hash() {
    let (_temp, store, _) = fixture(false);
    let original = metadata_text(&store.connection.lock().unwrap(), "config")
        .unwrap()
        .unwrap();
    let spaced = format!("\n  {original}\n");
    put_metadata(&store.connection.lock().unwrap(), "config", &spaced).unwrap();
    let before: Vec<(String, String)> = store
        .connection
        .lock()
        .unwrap()
        .prepare("SELECT key,value FROM metadata ORDER BY key")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let snapshot = store.sitemap_read_snapshot().unwrap();
    assert_eq!(snapshot.crawl_config_sha256, Some(sha256(&spaced)));
    assert_eq!(
        snapshot
            .coverage
            .basis
            .as_ref()
            .unwrap()
            .inventory_saved_config_sha256,
        Some(sha256(&spaced))
    );
    assert_eq!(snapshot.coverage.completion, Completion::KnownEmpty);
    let after: Vec<(String, String)> = store
        .connection
        .lock()
        .unwrap()
        .prepare("SELECT key,value FROM metadata ORDER BY key")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(before, after);
    assert!(
        !snapshot
            .catalog
            .unwrap()
            .summary
            .declared_source_cohort_complete
    );
}

#[test]
fn discovery_complete_marker_and_done_flag_do_not_replace_required_source_observation() {
    let (_temp, store, _) = fixture(true);
    let connection = store.connection.lock().unwrap();
    put_metadata(&connection, "discovery_complete", "true").unwrap();
    connection
        .execute("UPDATE discovery_frontier SET done=1", [])
        .unwrap();
    drop(connection);
    let snapshot = store.sitemap_read_snapshot().unwrap();
    assert_eq!(snapshot.coverage.completion, Completion::Unknown);
    assert!(
        snapshot
            .coverage
            .reason_counts
            .contains_key(&Reason::DoneSourceWithoutObservation)
    );
    assert!(
        !snapshot
            .coverage
            .current_valid_original_location_negative_permitted
    );
}

#[test]
fn missing_required_robots_row_prevents_false_empty_after_initialized_marker() {
    let (_temp, store, _) = fixture(true);
    store
        .connection
        .lock()
        .unwrap()
        .execute("DELETE FROM discovery_frontier", [])
        .unwrap();
    let snapshot = store.sitemap_read_snapshot().unwrap();
    assert_eq!(snapshot.coverage.completion, Completion::Unknown);
    assert!(
        snapshot
            .coverage
            .reason_counts
            .contains_key(&Reason::RequiredRobotsTaskMissing)
    );
}

#[test]
fn malformed_task_context_is_rejected_before_projection() {
    for (kind, done) in [("unexpected", 0), ("robots", 7)] {
        let (_temp, store, _) = fixture(false);
        store.connection.lock().unwrap().execute("INSERT INTO discovery_frontier(kind,url,done) VALUES(?1,'https://example.org/',?2)",params![kind,done]).unwrap();
        assert!(store.sitemap_read_snapshot().is_err());
    }
}

#[test]
fn current_config_task_inventory_and_generation_share_one_wal_read_transaction() {
    let (temp, store, _) = fixture(false);
    let path = temp.path().join("crawl.sqlite");
    let writer = std::thread::spawn(move || {
        let mut connection = Connection::open(path).unwrap();
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        for generation in 2u64..=90 {
            let transaction = connection.transaction().unwrap();
            let mut config: CrawlConfig = metadata(&transaction, "config").unwrap().unwrap();
            config.max_pages = Some(generation as usize);
            put_metadata(
                &transaction,
                "config",
                &serde_json::to_string(&config).unwrap(),
            )
            .unwrap();
            put_metadata(
                &transaction,
                "run_state",
                &serde_json::to_string(&CrawlRunState {
                    generation,
                    started_at: "writer".into(),
                    finished_at: None,
                    outcome: None,
                })
                .unwrap(),
            )
            .unwrap();
            transaction
                .execute("DELETE FROM discovery_frontier", [])
                .unwrap();
            transaction
                .execute(
                    "INSERT INTO discovery_frontier(kind,url,done) VALUES('robots',?1,?2)",
                    params![
                        format!("https://generation-{generation}.example.org/"),
                        i64::from(generation % 2 == 0)
                    ],
                )
                .unwrap();
            transaction.commit().unwrap();
        }
    });
    for _ in 0..90 {
        let snapshot = store.sitemap_read_snapshot().unwrap();
        assert_eq!(
            snapshot
                .coverage
                .basis
                .as_ref()
                .unwrap()
                .inventory_generation,
            snapshot.generation
        );
        assert_eq!(
            snapshot
                .coverage
                .basis
                .as_ref()
                .unwrap()
                .inventory_saved_config_sha256,
            snapshot.crawl_config_sha256
        );
        let inventory_json = serde_json::to_value(snapshot.coverage_inventory.unwrap()).unwrap();
        let generation = snapshot.generation.unwrap();
        if generation > 1 {
            assert_eq!(
                inventory_json["tasks"][0]["task_url"],
                format!("https://generation-{generation}.example.org/")
            );
            assert_eq!(inventory_json["tasks"][0]["done"], generation % 2 == 0);
        } else {
            assert_eq!(inventory_json["tasks"].as_array().unwrap().len(), 0);
        }
    }
    writer.join().unwrap();
}

#[test]
fn checkpoint_binds_exact_source_inventory_and_saved_portable_report() {
    let (_temp, store, generation) = fixture(false);
    let mut report = portable(&store);
    let original = report.sitemap_coverage_inventory.clone();
    let mut invalid =
        serde_json::to_value(report.sitemap_coverage_inventory.as_ref().unwrap()).unwrap();
    invalid["discovery_initialized"] = serde_json::Value::Bool(false);
    report.sitemap_coverage_inventory = Some(serde_json::from_value(invalid).unwrap());
    assert!(
        store
            .save_checkpoint(&report, CrawlOutcome::Completed, generation)
            .is_err()
    );
    assert!(store.checkpoint().unwrap().is_none());
    report.sitemap_coverage_inventory = original;
    let checkpoint = store
        .save_checkpoint(&report, CrawlOutcome::Completed, generation)
        .unwrap();
    let saved = store.load_report().unwrap().unwrap();
    assert_eq!(
        saved.sitemap_coverage_inventory,
        report.sitemap_coverage_inventory
    );
    assert_eq!(
        store
            .inspection()
            .unwrap()
            .checkpoint
            .unwrap()
            .report_sha256,
        checkpoint.report_sha256
    );
    let snapshot = store.sitemap_read_snapshot().unwrap();
    assert_eq!(snapshot.coverage.completion, Completion::KnownEmpty);
}

#[test]
fn actual_selected_store_rebinds_complete_parent_only_as_historical_and_legacy_is_unknown() {
    for captured in [true, false] {
        let (_temp, parent, _) = fixture(false);
        let mut report = portable(&parent);
        report.pages = vec![PageRecord {
            url: "https://example.org/a".into(),
            ..Default::default()
        }];
        if !captured {
            report.sitemap_coverage_inventory = None;
        }
        let bytes = serde_json::to_vec_pretty(&report).unwrap();
        let prepared = crate::prepare_selected_recrawl(
            crate::SelectedRecrawlParent {
                job_id: "captured-parent".into(),
                checkpoint_revision: None,
                source_kind: crate::RecrawlSourceKind::StoredJson,
            },
            &bytes,
            &["https://example.org/a".into()],
            None,
        )
        .unwrap();
        let child_temp = tempfile::tempdir().unwrap();
        let child = CrawlStore::open(child_temp.path().join("child.sqlite")).unwrap();
        child.initialize_selected_recrawl(&prepared).unwrap();
        child.begin_run("selected-child").unwrap();
        let snapshot = child.sitemap_read_snapshot().unwrap();
        assert_eq!(
            snapshot.coverage.scope,
            crate::SitemapCoverageScope::InheritedHistoricalDeclaredSources
        );
        assert!(
            !snapshot
                .coverage
                .current_valid_original_location_negative_permitted
        );
        assert_eq!(
            snapshot
                .coverage
                .historical_valid_original_location_negative_permitted,
            captured
        );
        assert_eq!(
            snapshot.coverage.completion,
            if captured {
                Completion::KnownEmpty
            } else {
                Completion::Unknown
            }
        );
        assert_eq!(child.parent_snapshot_bytes().unwrap(), Some(bytes));
    }
}

#[test]
fn imported_report_inventory_without_matching_original_config_is_rejected() {
    let (_temp, store, _) = fixture(false);
    let mut report = portable(&store);
    report.config.explicit_sitemap_inputs = Some(vec!["https://example.org/new.xml".into()]);
    assert!(store.save_report(&report).is_err());
    assert!(store.load_report().unwrap().is_none());
}

#[test]
fn terminal_checkpoint_detects_inventory_mutation_even_with_unchanged_pending_count() {
    let (_temp, store, generation) = fixture(false);
    let report = portable(&store);
    store
        .save_checkpoint(&report, CrawlOutcome::Completed, generation)
        .unwrap();
    store.connection.lock().unwrap().execute("INSERT INTO discovery_frontier(kind,url,done) VALUES('robots','https://unexpected.example.org/',1)", []).unwrap();
    // This is a done row, so existing discovery_pending/checkpoint counts alone match.
    assert!(store.inspection().is_err());
}

#[test]
fn policy_robots_inventory_uses_actual_initial_admissions_excluding_denied_origins() {
    let temp = tempfile::tempdir().unwrap();
    let store = CrawlStore::open(temp.path().join("policy.sqlite")).unwrap();
    let mut policy = crate::CrawlPolicyConfig::default();
    policy.scope.roots.push(crate::CrawlScopeRoot {
        origin: "https://example.org".into(),
        subdomains: false,
        folders: vec!["/".into()],
        exact_paths: vec![],
    });
    let config = CrawlConfig {
        seed_urls: vec![
            "https://example.org/start".into(),
            "https://denied.example.net/start".into(),
        ],
        explicit_sitemap_inputs: Some(vec![]),
        crawl_policy: Some(policy.clone()),
        ..Default::default()
    };
    store.prepare(&config).unwrap();
    let generation = store.begin_run("policy-coverage").unwrap();
    let compiled = crate::CompiledCrawlPolicy::new(&policy).unwrap();
    let revision = format!(
        "initial:{}:explicit:page_document",
        compiled.policy_sha256()
    );
    let mut rows = config
        .seed_urls
        .iter()
        .enumerate()
        .map(|(ordinal, url)| {
            crate::policy_ledger::decide(
                &compiled,
                &revision,
                ordinal,
                crate::CandidateSourceKind::Seed,
                url.clone(),
                None,
                None,
                crate::PolicyInputBasis::DeclaredInput,
                crate::PolicyTaskRole::PageDocument,
            )
        })
        .collect::<Vec<_>>();
    store
        .initialize_policy(&compiled, &mut rows, &Default::default(), generation)
        .unwrap();
    assert_eq!(rows[0].admission_reason, "scheduled");
    assert_eq!(rows[1].admission_reason, "policy_denied");
    let saved = metadata_text(&store.connection.lock().unwrap(), "config")
        .unwrap()
        .unwrap();
    store
        .initialize_sitemap_discovery(
            &[DiscoveryEntry {
                kind: DiscoveryKind::Robots,
                url: "https://example.org/".into(),
            }],
            generation,
            &SitemapCatalogInit {
                config_sha256: sha256(&saved),
                limits: Default::default(),
                roots: vec![],
                legacy_history_unknown: false,
            },
        )
        .unwrap();
    let mut body = crate::RobotsBodyBuffer::default();
    body.observe_eof();
    let source = crate::RobotsSitemapSource::observed(
        "https://example.org/robots.txt".into(),
        "https://example.org/robots.txt".into(),
        200,
        &body,
        &url::Url::parse("https://example.org/").unwrap(),
    );
    let batch = crate::sitemap_discovery::robots_batch(
        "https://example.org/",
        generation,
        &crate::sitemap_ledger::digest([]),
        &source,
    )
    .unwrap();
    store
        .complete_discovery_with_policy_observed(
            &DiscoveryEntry {
                kind: DiscoveryKind::Robots,
                url: "https://example.org/".into(),
            },
            &mut [],
            (&batch.document.source_revision.clone(), true),
            &[],
            generation,
            &Default::default(),
            batch,
            |_, _, _| Ok(()),
        )
        .unwrap();
    let snapshot = store.sitemap_read_snapshot().unwrap();
    let inventory = serde_json::to_value(snapshot.coverage_inventory.unwrap()).unwrap();
    assert_eq!(
        inventory["expected_robots_sources"],
        serde_json::json!(["https://example.org/"])
    );
    assert_eq!(snapshot.coverage.completion, Completion::KnownEmpty);
    assert!(
        snapshot
            .coverage
            .current_valid_original_location_negative_permitted
    );
}

#[test]
fn ordinary_sitemap_only_scope_requires_robots_origin_derived_from_original_protocol_inputs() {
    let temp = tempfile::tempdir().unwrap();
    let store = CrawlStore::open(temp.path().join("sitemap-only.sqlite")).unwrap();
    let task = "https://example.org/map.xml";
    let config = CrawlConfig {
        seed_urls: vec![],
        sitemap_urls: vec![task.into()],
        explicit_sitemap_inputs: Some(vec![task.into()]),
        ..Default::default()
    };
    store.prepare(&config).unwrap();
    let generation = store.begin_run("sitemap-only-coverage").unwrap();
    let saved = metadata_text(&store.connection.lock().unwrap(), "config")
        .unwrap()
        .unwrap();
    let sha = sha256(&saved);
    let mut roots = crate::sitemap_initial_roots::initial_roots(
        config.explicit_sitemap_inputs.as_deref(),
        &[],
        &sha,
        None,
    )
    .unwrap()
    .unwrap();
    roots[0].root.admission.decision = crate::SitemapAdmissionDecision::Scheduled;
    roots[0].root.admission.admitted_fetch_url = Some(task.into());
    // Model a missing required source task. Empty page seeds do not mean Robots
    // discovery was disabled: Crawler's ordinary sitemap-only scope uses this origin.
    store
        .initialize_sitemap_discovery(
            &[DiscoveryEntry {
                kind: DiscoveryKind::Sitemap,
                url: task.into(),
            }],
            generation,
            &SitemapCatalogInit {
                config_sha256: sha,
                limits: Default::default(),
                roots: roots.into_iter().map(|r| r.root).collect(),
                legacy_history_unknown: false,
            },
        )
        .unwrap();
    let snapshot = store.sitemap_read_snapshot().unwrap();
    assert!(
        snapshot
            .coverage
            .reason_counts
            .contains_key(&Reason::RequiredRobotsTaskMissing)
    );
    assert_eq!(snapshot.coverage.completion, Completion::Unknown);
    let inventory = serde_json::to_value(snapshot.coverage_inventory.unwrap()).unwrap();
    assert_eq!(
        inventory["expected_robots_sources"],
        serde_json::json!(["https://example.org/"])
    );
}
