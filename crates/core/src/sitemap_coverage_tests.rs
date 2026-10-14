use super::*;
use crate::store::{DiscoveryEntry, DiscoveryKind, SitemapCatalogInit};
use crate::{CrawlStore, SitemapDiscoveryBatch, SitemapParserLimits, SitemapTransportFacts};

const ORIGIN: &str = "https://example.org/";
const ROOT: &str = "https://example.org/root.xml";
const CHILD: &str = "https://example.org/child.xml";

struct Fixture {
    _temp: tempfile::TempDir,
    store: CrawlStore,
    config: CrawlConfig,
    generation: u64,
}
impl Fixture {
    fn new(originals: Option<Vec<String>>, robots: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let store = CrawlStore::open(temp.path().join("crawl.sqlite")).unwrap();
        let config = CrawlConfig {
            seed_urls: vec![ORIGIN.into()],
            explicit_sitemap_inputs: originals.clone(),
            sitemap_urls: originals
                .clone()
                .unwrap_or_default()
                .iter()
                .filter_map(|url| crate::normalize_url(url, None))
                .map(|url| url.to_string())
                .collect(),
            obey_robots: robots,
            ..Default::default()
        };
        store.prepare(&config).unwrap();
        let generation = store.begin_run("coverage-fixture").unwrap();
        let sha = crate::sitemap_ledger::digest(serde_json::to_vec(&config).unwrap());
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
            _temp: temp,
            store,
            config,
            generation,
        }
    }
    fn xml(&self, task: &str, xml: &str) -> SitemapDiscoveryBatch {
        let limits = SitemapParserLimits::default();
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
        let sha = crate::sitemap_ledger::digest(xml.as_bytes());
        let mut batch = crate::sitemap_discovery::xml_batch(
            task,
            self.generation,
            &sha,
            SitemapTransportFacts {
                request_started: true,
                requested_url: Some(task.into()),
                final_url: Some(task.into()),
                initial_status: Some(200),
                final_status: Some(200),
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
        for location in &mut batch.locations {
            if location.admission.decision == Admission::Unbound {
                location.admission.decision = Admission::Scheduled;
                location.admission.admitted_fetch_url =
                    location.observed.normalized_fetch_url.clone();
                if location.observed.role == crate::SitemapLocationRole::ChildSitemap {
                    location.referenced_sitemap_task =
                        location.admission.admitted_fetch_url.clone();
                }
            }
        }
        batch
    }
    fn commit_xml(&self, task: &str, batch: SitemapDiscoveryBatch, children: &[&str]) {
        self.store
            .complete_discovery_observed(
                &DiscoveryEntry {
                    kind: DiscoveryKind::Sitemap,
                    url: task.into(),
                },
                &[],
                &children
                    .iter()
                    .map(|url| DiscoveryEntry {
                        kind: DiscoveryKind::Sitemap,
                        url: (*url).into(),
                    })
                    .collect::<Vec<_>>(),
                &[],
                self.generation,
                batch,
            )
            .unwrap();
    }
    fn robots(&self, body: &[u8], status: u16, complete: bool) {
        let mut buffer = crate::RobotsBodyBuffer::default();
        buffer.push_chunk(body);
        if complete {
            buffer.observe_eof();
        }
        let source = crate::RobotsSitemapSource::observed(
            format!("{ORIGIN}robots.txt"),
            format!("{ORIGIN}robots.txt"),
            status,
            &buffer,
            &url::Url::parse(ORIGIN).unwrap(),
        );
        let batch = crate::sitemap_discovery::robots_batch(
            ORIGIN,
            self.generation,
            &crate::sitemap_ledger::digest(body),
            &source,
        )
        .unwrap();
        self.store
            .complete_discovery_observed(
                &DiscoveryEntry {
                    kind: DiscoveryKind::Robots,
                    url: ORIGIN.into(),
                },
                &[],
                &[],
                &[],
                self.generation,
                batch,
            )
            .unwrap();
    }
    fn snapshot(&self) -> crate::SitemapReadSnapshot {
        self.store.sitemap_read_snapshot().unwrap()
    }
}

#[test]
fn complete_recursive_duplicate_roots_and_cycle_are_achievable_without_mutating_history() {
    let f = Fixture::new(
        Some(vec![ROOT.into(), format!("{ROOT}#second-original")]),
        false,
    );
    let mut root = f.xml(ROOT,&format!("<sitemapindex><sitemap><loc>{CHILD}</loc></sitemap><sitemap><loc>{CHILD}</loc></sitemap></sitemapindex>"));
    root.locations[1].admission.decision = Admission::Coalesced;
    f.commit_xml(ROOT, root, &[CHILD]);
    let mut child = f.xml(
        CHILD,
        &format!("<sitemapindex><sitemap><loc>{ROOT}</loc></sitemap></sitemapindex>"),
    );
    child.locations[0].admission.decision = Admission::Coalesced;
    f.commit_xml(CHILD, child, &[]);
    let before = f.store.sitemap_catalog().unwrap().unwrap();
    let snapshot = f.snapshot();
    assert_eq!(
        snapshot.coverage.completion,
        SitemapCohortCompletion::Complete
    );
    assert!(
        snapshot
            .coverage
            .current_valid_original_location_negative_permitted
    );
    assert!(
        !snapshot
            .coverage
            .historical_valid_original_location_negative_permitted
    );
    assert_eq!(
        (
            snapshot.coverage.counts.root_declarations,
            snapshot.coverage.counts.child_declarations,
            snapshot.coverage.counts.reached_xml_tasks
        ),
        (2, 3, 2)
    );
    assert!(snapshot.coverage.reason_counts.is_empty());
    assert_eq!(snapshot.catalog.as_ref(), Some(&before));
    assert!(!before.summary.declared_source_cohort_complete);
}

#[test]
fn known_empty_requires_completed_required_robots_not_empty_catalog() {
    let f = Fixture::new(Some(vec![]), true);
    let pending = f.snapshot();
    assert_eq!(
        pending.coverage.completion,
        SitemapCohortCompletion::Unknown
    );
    assert_eq!(
        pending
            .coverage
            .reason_counts
            .get(&SitemapCoverageReason::PendingSource),
        Some(&1)
    );
    f.robots(b"User-agent: *\nAllow: /\n", 200, true);
    let complete = f.snapshot();
    assert_eq!(
        complete.coverage.completion,
        SitemapCohortCompletion::KnownEmpty
    );
    assert!(
        complete
            .coverage
            .current_valid_original_location_negative_permitted
    );
    assert_eq!(complete.coverage.counts.complete_documents, 1);
}

#[test]
fn required_robots_task_removed_from_inventory_cannot_claim_known_empty() {
    let f = Fixture::new(Some(vec![]), true);
    let snap = f.snapshot();
    let mut inventory = snap.coverage_inventory.unwrap();
    inventory.tasks.clear();
    let out = project_sitemap_coverage(snap.catalog.as_ref(), Some(&inventory)).unwrap();
    assert_eq!(out.completion, SitemapCohortCompletion::Unknown);
    assert_eq!(
        out.reason_counts
            .get(&SitemapCoverageReason::RequiredRobotsTaskMissing),
        Some(&1)
    );
}

#[test]
fn robots_206_and_eof_missing_remain_unknown_with_zero_declarations() {
    for (status, eof, reason) in [
        (206, true, SitemapCoverageReason::SourceHttpNot200),
        (200, false, SitemapCoverageReason::SourceBodyIncomplete),
    ] {
        let f = Fixture::new(Some(vec![]), true);
        f.robots(b"", status, eof);
        let out = f.snapshot().coverage;
        assert_eq!(out.completion, SitemapCohortCompletion::Unknown);
        assert!(out.reason_counts.contains_key(&reason));
        assert!(!out.current_valid_original_location_negative_permitted);
    }
}

#[test]
fn pending_child_prevents_complete_even_after_complete_root_source() {
    let f = Fixture::new(Some(vec![ROOT.into()]), false);
    f.commit_xml(
        ROOT,
        f.xml(
            ROOT,
            &format!("<sitemapindex><sitemap><loc>{CHILD}</loc></sitemap></sitemapindex>"),
        ),
        &[CHILD],
    );
    let out = f.snapshot().coverage;
    assert_eq!(out.completion, SitemapCohortCompletion::Unknown);
    assert_eq!(out.counts.complete_documents, 1);
    assert!(
        out.reason_counts
            .contains_key(&SitemapCoverageReason::PendingSource)
    );
}

#[test]
fn denied_child_and_invalid_root_remain_explicit_unknown() {
    let f = Fixture::new(
        Some(vec![ROOT.into(), "ftp://example.org/invalid".into()]),
        false,
    );
    let mut batch = f.xml(
        ROOT,
        &format!("<sitemapindex><sitemap><loc>{CHILD}</loc></sitemap></sitemapindex>"),
    );
    batch.locations[0].admission.decision = Admission::OutsideScope;
    batch.locations[0].admission.admitted_fetch_url = None;
    batch.locations[0].referenced_sitemap_task = None;
    f.commit_xml(ROOT, batch, &[]);
    let out = f.snapshot().coverage;
    assert_eq!(out.completion, SitemapCohortCompletion::Unknown);
    assert!(
        out.reason_counts
            .contains_key(&SitemapCoverageReason::RootInvalid)
    );
    assert!(
        out.reason_counts
            .contains_key(&SitemapCoverageReason::ChildDenied)
    );
}

#[test]
fn incomplete_xml_prefix_cannot_prove_source_cohort_complete() {
    let f = Fixture::new(Some(vec![ROOT.into()]), false);
    f.commit_xml(
        ROOT,
        f.xml(ROOT, "<urlset><url><loc>https://example.org/a</loc></url>"),
        &[],
    );
    let out = f.snapshot().coverage;
    assert_eq!(out.completion, SitemapCohortCompletion::Unknown);
    assert!(
        out.reason_counts
            .contains_key(&SitemapCoverageReason::SourceParserIncomplete)
    );
}

#[test]
fn target_denial_and_invalid_page_loc_do_not_change_valid_original_source_scan() {
    let f = Fixture::new(Some(vec![ROOT.into()]), false);
    let mut batch=f.xml(ROOT,"<urlset><url><loc>https://example.org/filtered</loc></url><url><loc>ftp://example.org/invalid</loc></url><url/></urlset>");
    batch.locations[0].admission.decision = Admission::FilterDenied;
    batch.locations[0].admission.admitted_fetch_url = None;
    f.commit_xml(ROOT, batch, &[]);
    let out = f.snapshot().coverage;
    assert_eq!(out.completion, SitemapCohortCompletion::Complete);
    assert_eq!(out.counts.invalid_or_missing_page_locations, 2);
    assert!(out.current_valid_original_location_negative_permitted);
}

#[test]
fn unavailable_and_legacy_history_cannot_be_promoted_from_done_empty_tasks() {
    let unavailable = project_sitemap_coverage(None, None).unwrap();
    assert_eq!(unavailable.scope, SitemapCoverageScope::Unavailable);
    assert_eq!(unavailable.completion, SitemapCohortCompletion::Unknown);
    let f = Fixture::new(None, false);
    let out = f.snapshot().coverage;
    assert_eq!(out.completion, SitemapCohortCompletion::Unknown);
    assert!(
        out.reason_counts
            .contains_key(&SitemapCoverageReason::LegacyHistoryUnknown)
    );
    assert!(
        out.reason_counts
            .contains_key(&SitemapCoverageReason::InitialRootInventoryUnknown)
    );
}

#[test]
fn missing_initial_root_coordinates_and_extra_task_are_qualified() {
    let f = Fixture::new(Some(vec![ROOT.into()]), false);
    f.commit_xml(ROOT, f.xml(ROOT, "<urlset/>"), &[]);
    let snap = f.snapshot();
    let mut inventory = snap.coverage_inventory.unwrap();
    inventory.expected_initial_roots.as_mut().unwrap()[0].raw_value = "different-original".into();
    inventory.tasks.push(SitemapCoverageTask {
        kind: SitemapSourceKind::SitemapXml,
        task_url: CHILD.into(),
        done: true,
    });
    inventory
        .tasks
        .sort_by(|a, b| task_key(a).cmp(&task_key(b)));
    let out = project_sitemap_coverage(snap.catalog.as_ref(), Some(&inventory)).unwrap();
    assert!(
        out.reason_counts
            .contains_key(&SitemapCoverageReason::InitialRootCoordinatesDiffer)
    );
    assert!(
        out.reason_counts
            .contains_key(&SitemapCoverageReason::DoneSourceWithoutObservation)
    );
    assert!(
        out.reason_counts
            .contains_key(&SitemapCoverageReason::SourceGraphUnboundTask)
    );
    assert!(
        validate_sitemap_coverage_inventory(&f.config, snap.catalog.as_ref(), Some(&inventory))
            .is_err()
    );
}

#[test]
fn captured_inventory_hash_binds_pending_flags_generation_and_actual_config() {
    let f = Fixture::new(Some(vec![ROOT.into()]), false);
    let snap = f.snapshot();
    let inventory = snap.coverage_inventory.as_ref().unwrap();
    let initial = snap
        .coverage
        .basis
        .as_ref()
        .unwrap()
        .captured_inventory_sha256
        .clone();
    for changed in [
        {
            let mut i = inventory.clone();
            i.tasks[0].done = true;
            i
        },
        {
            let mut i = inventory.clone();
            i.generation += 1;
            i
        },
        {
            let mut i = inventory.clone();
            i.saved_config_sha256 = "f".repeat(64);
            i
        },
    ] {
        let out = project_sitemap_coverage(snap.catalog.as_ref(), Some(&changed)).unwrap();
        assert_ne!(out.basis.unwrap().captured_inventory_sha256, initial);
    }
}

#[test]
fn malformed_deserialized_inventory_is_error_not_positive_fallback() {
    let f = Fixture::new(Some(vec![]), false);
    let snap = f.snapshot();
    let mut invalid = snap.coverage_inventory.clone().unwrap();
    invalid.generation = 0;
    assert!(project_sitemap_coverage(snap.catalog.as_ref(), Some(&invalid)).is_err());
    let mut invalid = snap.coverage_inventory.clone().unwrap();
    invalid.tasks = vec![SitemapCoverageTask {
        kind: SitemapSourceKind::SitemapXml,
        task_url: "ftp://example.org/".into(),
        done: true,
    }];
    assert!(project_sitemap_coverage(snap.catalog.as_ref(), Some(&invalid)).is_err());
    let mut invalid = snap.coverage_inventory.unwrap();
    invalid.tasks = vec![
        SitemapCoverageTask {
            kind: SitemapSourceKind::RobotsDirectives,
            task_url: ORIGIN.into(),
            done: true
        };
        2
    ];
    assert!(project_sitemap_coverage(snap.catalog.as_ref(), Some(&invalid)).is_err());
}

#[test]
fn immutable_parent_coverage_is_historical_and_chain_rebind_is_separate() {
    let f = Fixture::new(Some(vec![ROOT.into()]), false);
    f.commit_xml(ROOT, f.xml(ROOT, "<urlset/>"), &[]);
    let snap = f.snapshot();
    let catalog = snap.catalog.unwrap();
    let inventory = snap.coverage_inventory.unwrap();
    let bytes = serde_json::to_vec(
        &serde_json::json!({"sitemap_catalog":catalog,"sitemap_coverage_inventory":inventory}),
    )
    .unwrap();
    let rebound = inventory.bind_inherited_parent(&catalog, &bytes).unwrap();
    let inherited = crate::sitemap_ledger::project_inherited_catalog(
        &catalog,
        &crate::sitemap_ledger::digest(&bytes),
    )
    .unwrap();
    let out = project_sitemap_coverage(Some(&inherited), Some(&rebound)).unwrap();
    assert_eq!(out.completion, SitemapCohortCompletion::Complete);
    assert_eq!(
        out.scope,
        SitemapCoverageScope::InheritedHistoricalDeclaredSources
    );
    assert!(!out.current_valid_original_location_negative_permitted);
    assert!(out.historical_valid_original_location_negative_permitted);
    assert_eq!(
        out.basis.as_ref().unwrap().parent_snapshot_sha256,
        Some(crate::sitemap_ledger::digest(&bytes))
    );
    assert_eq!(
        project_sitemap_coverage(Some(&inherited), None)
            .unwrap()
            .completion,
        SitemapCohortCompletion::Unknown
    );
    let next = serde_json::to_vec(
        &serde_json::json!({"sitemap_catalog":inherited,"sitemap_coverage_inventory":rebound}),
    )
    .unwrap();
    let chained = rebound.bind_inherited_parent(&inherited, &next).unwrap();
    let chained_catalog = crate::sitemap_ledger::project_inherited_catalog(
        &inherited,
        &crate::sitemap_ledger::digest(&next),
    )
    .unwrap();
    assert!(
        project_sitemap_coverage(Some(&chained_catalog), Some(&chained))
            .unwrap()
            .historical_valid_original_location_negative_permitted
    );
}

#[test]
fn immutable_parent_incompatible_inventory_cannot_be_repaired_by_descendant_rebind() {
    let f = Fixture::new(Some(vec![ROOT.into()]), false);
    f.commit_xml(ROOT, f.xml(ROOT, "<urlset/>"), &[]);
    let snapshot = f.snapshot();
    let parent = snapshot.catalog.unwrap();
    let inventory = snapshot.coverage_inventory.unwrap();
    let first_bytes = serde_json::to_vec(
        &serde_json::json!({"sitemap_catalog":parent,"sitemap_coverage_inventory":inventory}),
    )
    .unwrap();
    let mut historical = inventory
        .clone()
        .bind_inherited_parent(&parent, &first_bytes)
        .unwrap();
    let inherited = crate::sitemap_ledger::project_inherited_catalog(
        &parent,
        &crate::sitemap_ledger::digest(&first_bytes),
    )
    .unwrap();
    historical.basis = InventoryBasis::InheritedParent {
        parent_snapshot_sha256: "a".repeat(64),
        parent_catalog_sha256: "b".repeat(64),
    };
    for catalog in [&parent, &inherited] {
        let invalid = historical.clone();
        let prior = project_sitemap_coverage(Some(catalog), Some(&invalid)).unwrap();
        assert_eq!(prior.completion, SitemapCohortCompletion::Unknown);
        let bytes = serde_json::to_vec(
            &serde_json::json!({"sitemap_catalog":catalog,"sitemap_coverage_inventory":invalid}),
        )
        .unwrap();
        assert!(invalid.bind_inherited_parent(catalog, &bytes).is_err());
    }
}
