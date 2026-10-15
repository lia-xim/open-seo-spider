//! Authored storage/parser regressions, NOT_RUN by the source author.
//! Response facts here are caller-declared fixture input to the production
//! writer, never a claimed observed HTTP crawl/native acceptance proof.
use super::*;
use crate::resource_policy::ResourcePermission;
use crate::{
    ContentCaptureDecodeStatus, CrawlStore, PageRecord, ResourceDiscoveryConfig,
    ResourceDiscoveryLimits, ResourceSourceBatch,
};
use rusqlite::params;
use std::path::Path;
fn config() -> CrawlConfig {
    let mut resources = ResourceDiscoveryConfig::default();
    resources.resources.images.store = false;
    resources.css.stylesheets = true;
    CrawlConfig {
        seed_urls: vec![
            "https://fixture.test/a.css".into(),
            "https://fixture.test/b.css".into(),
        ],
        resource_discovery: Some(resources),
        discover_sitemaps: false,
        obey_robots: false,
        ..Default::default()
    }
}
fn rows(
    cfg: &CrawlConfig,
    batch: &ResourceSourceBatch,
    generation: u64,
    evaluation: &str,
    suppress_first: bool,
) -> Vec<StoredResourceOccurrence> {
    let cfg = cfg.resource_discovery.as_ref().unwrap();
    batch
        .occurrences
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let decision = cfg
                .decide(
                    o,
                    ResourcePermission {
                        source_complete: o.source_complete,
                        source_can_expand: !(suppress_first && i == 0),
                        selected: false,
                        navigation_enabled: true,
                        depth_allowed: true,
                        scope_allowed: true,
                        filters_allowed: true,
                        policy_check: true,
                        policy_traverse: true,
                        document_nofollow: false,
                        target_internal: Some(true),
                    },
                )
                .unwrap();
            StoredResourceOccurrence {
                sequence: 0,
                generation,
                basis: ResourceObservationBasis::Current,
                evaluation_revision: crate::resource_policy::digest(evaluation),
                occurrence: o.clone(),
                decision,
            }
        })
        .collect()
}
fn capture(
    store: &CrawlStore,
    cfg: &CrawlConfig,
    url: &str,
    mime: &str,
    body: &str,
    status: u16,
    generation: u64,
) -> PageRecord {
    let mut page = PageRecord {
        url: url.into(),
        final_url: url.into(),
        status: Some(status),
        final_status: Some(status),
        content_type: mime.into(),
        fetched_at: "2026-10-15T12:00:05Z".into(),
        content_hash: Some(crate::resource_policy::digest(body)),
        ..Default::default()
    };
    page.resource_permission = store.resource_permission(url).unwrap();
    let batch = crate::resource_capture::inventory(
        &page,
        Some(body),
        ContentCaptureDecodeStatus::Exact,
        &[crate::normalize_url("https://fixture.test/", None).unwrap()],
        false,
        cfg.resource_discovery.as_ref().unwrap(),
        &ResourceDiscoveryLimits::default(),
    )
    .unwrap();
    page.resource_source = Some(batch);
    page.resource_response = Some(ResourceResponseFacts {
        engine: crate::RESOURCE_DISCOVERY_ENGINE.into(),
        contract_engine: None,
        redirect_decisions: vec![],
        selected_materialization_reason: None,
        observed_generation: generation,
        retained_response: page.resource_permission.as_ref().unwrap().store,
        entity_body_retained: false,
        captured_entity_sha256: Some(crate::resource_policy::digest(body)),
        captured_entity_bytes: body.len(),
        captured_entity_complete: true,
        decoded_source_sha256: Some(crate::resource_policy::digest(body)),
        decode_status: ContentCaptureDecodeStatus::Exact,
        decode_encoding: Some("UTF-8".into()),
        metadata_projection_complete: true,
        unretained_fields: vec![],
    });
    page
}
struct Fixture {
    dir: tempfile::TempDir,
    store: CrawlStore,
    cfg: CrawlConfig,
    generation: u64,
    sources: Vec<PageRecord>,
}
impl Fixture {
    fn new(count: usize, suppress_first: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = CrawlStore::open(dir.path().join("saved.sqlite")).unwrap();
        let cfg = config();
        store.prepare(&cfg).unwrap();
        let generation = store.begin_run("2026-10-15T12:00:00Z").unwrap();
        for u in &cfg.seed_urls {
            store
                .enqueue(&crate::store::QueueEntry {
                    url: u.clone(),
                    depth: 0,
                    discovered_from: None,
                    admission: None,
                })
                .unwrap();
        }
        store
            .initialize_resources(
                cfg.resource_discovery.as_ref().unwrap(),
                &Default::default(),
                generation,
            )
            .unwrap();
        let mut sources = vec![];
        for (i, u) in cfg.seed_urls.iter().enumerate() {
            let css = if i == 0 {
                (0..count)
                    .map(|n| format!(".n{n}{{background:url('/bad.png')}}"))
                    .collect::<String>()
            } else {
                ".b{background:url('/bad.png')}".into()
            };
            let p = capture(&store, &cfg, u, "text/css", &css, 200, generation);
            let mut values = rows(
                &cfg,
                p.resource_source.as_ref().unwrap(),
                generation,
                &format!("initial{i}"),
                suppress_first && i == 0,
            );
            store
                .complete_resources(&p, &mut values, &mut [], generation, false)
                .unwrap();
            sources.push(p);
        }
        let target = capture(
            &store,
            &cfg,
            "https://fixture.test/bad.png",
            "image/png",
            "",
            404,
            generation,
        );
        store
            .complete_resources(&target, &mut [], &mut [], generation, false)
            .unwrap();
        Self {
            dir,
            store,
            cfg,
            generation,
            sources,
        }
    }
    fn path(&self) -> std::path::PathBuf {
        self.dir.path().join("saved.sqlite")
    }
}
fn request(filter: ResourceBrokenSourcesFilter) -> ResourceBrokenSourcesRequest {
    ResourceBrokenSourcesRequest {
        filter,
        ..Default::default()
    }
}
fn failures(store: &CrawlStore) -> ResourceBrokenSourcesPage {
    store
        .resource_broken_sources(&request(ResourceBrokenSourcesFilter::Failures))
        .unwrap()
}
fn update_transport(path: &Path, edit: impl FnOnce(&mut ResourceTransportObservation)) {
    let c = Connection::open(path).unwrap();
    let data: String = c
        .query_row(
            "SELECT data FROM resource_transports WHERE url='https://fixture.test/b.css'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut o: ResourceTransportObservation = serde_json::from_str(&data).unwrap();
    edit(&mut o);
    let data = serde_json::to_string(&o).unwrap();
    c.execute(
        "UPDATE resource_transports SET data=?1,payload_sha256=?2 WHERE url=?3",
        params![data, crate::resource_policy::digest(&data), o.requested_url],
    )
    .unwrap();
}
#[test]
fn resource_broken_sources_real_css_duplicates_private_404_and_cold_read_only_reopen_preserve_source_units()
 {
    let f = Fixture::new(2, false);
    let page = failures(&f.store);
    assert_eq!(page.scope, RESOURCE_BROKEN_SOURCES_SCOPE);
    assert_eq!(page.rows.len(), 3);
    let snapshot = page.snapshot.as_ref().unwrap();
    assert_eq!(snapshot.evaluation_rows, 5);
    assert_eq!(snapshot.distinct_stored_occurrence_ids, 5);
    assert_eq!(snapshot.transport_rows, 3);
    assert!(!snapshot.current_website_freshness_attested);
    assert!(!snapshot.source_kind_coverage_complete);
    let a = page
        .rows
        .iter()
        .filter(|r| {
            r.observation
                .occurrence
                .requested_source_url
                .ends_with("/a.css")
        })
        .collect::<Vec<_>>();
    assert_eq!(a.len(), 2);
    assert_ne!(
        a[0].observation.occurrence.occurrence_id,
        a[1].observation.occurrence.occurrence_id
    );
    assert!(a[0].observation.occurrence.byte_start.is_some());
    assert_ne!(
        a[0].observation.occurrence.byte_start,
        a[1].observation.occurrence.byte_start
    );
    assert!(
        page.rows.iter().all(|r| r.anchor_text.is_none()
            && r.anchor_text_state == "not_captured_in_resource_occurrence")
    );
    assert!(
        page.rows
            .iter()
            .all(|r| !r.target_transport.as_ref().unwrap().retained_response
                && r.target_transport.as_ref().unwrap().final_status == Some(404))
    );
    let c = Connection::open(f.path()).unwrap();
    let retained: i64 = c
        .query_row(
            "SELECT count(*) FROM pages WHERE url='https://fixture.test/bad.png'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(retained, 0);
    drop(c);
    let token = snapshot.snapshot_token.clone();
    let path = f.path();
    drop(f.store);
    let cold = CrawlStore::open_read_only(&path).unwrap();
    let reopened = failures(&cold);
    assert_eq!(reopened.snapshot.as_ref().unwrap().snapshot_token, token);
    assert_eq!(reopened.rows.len(), 3);
    assert_eq!(reopened.rows[0].observation, page.rows[0].observation);
}
#[test]
fn resource_broken_sources_evaluation_revisions_do_not_inflate_physical_ids_or_upgrade_old_no_grant()
 {
    let f = Fixture::new(2, true);
    let first = f
        .store
        .resource_broken_sources(&request(ResourceBrokenSourcesFilter::All))
        .unwrap();
    let physical = first
        .snapshot
        .as_ref()
        .unwrap()
        .distinct_stored_occurrence_ids;
    let source = &f.sources[0];
    let dependency = source
        .resource_source
        .as_ref()
        .unwrap()
        .occurrences
        .first()
        .unwrap();
    assert_eq!(dependency.source_ordinal, 0);
    assert_eq!(dependency.reference_kind, crate::ResourceReferenceKind::Image);
    assert_eq!(
        dependency.resolved_url.as_deref(),
        Some("https://fixture.test/bad.png")
    );
    let old = first
        .rows
        .iter()
        .find(|r| {
            r.observation.occurrence.occurrence_id == dependency.occurrence_id
                && r.observation.evaluation_revision
                    == crate::resource_policy::digest("initial0")
        })
        .unwrap();
    assert_eq!(old.state, ResourceBrokenSourcesState::NoGrant);
    assert!(
        old.target_transport.is_none(),
        "An independent request must not upgrade a no-grant evaluation"
    );
    assert_eq!(old.observation.decision.reason, "source_check_only");
    assert!(old.observation.decision.grant.is_none());
    let id = old.observation.occurrence.occurrence_id.clone();
    let initial_input = first
        .rows
        .iter()
        .find(|r| {
            r.observation.occurrence.requested_source_url == source.url
                && r.observation.occurrence.reference_kind
                    == crate::ResourceReferenceKind::InitialInput
        })
        .unwrap();
    assert_eq!(initial_input.observation.occurrence.source_ordinal, 0);
    assert_ne!(initial_input.observation.occurrence.occurrence_id, id);
    assert_eq!(
        initial_input.state,
        ResourceBrokenSourcesState::ObservedNonFailure
    );
    assert_eq!(initial_input.observation.decision.reason, "initial_admitted");
    assert_eq!(
        initial_input
            .observation
            .decision
            .grant
            .as_ref()
            .unwrap()
            .fetch_url,
        source.url
    );
    assert_eq!(
        initial_input.target_transport.as_ref().unwrap().final_status,
        Some(200)
    );
    assert!(first.rows.iter().any(|r| {
        r.observation.occurrence.occurrence_id != id
            && r.observation
                .decision
                .grant
                .as_ref()
                .is_some_and(|g| g.fetch_url == "https://fixture.test/bad.png")
            && r.state == ResourceBrokenSourcesState::ObservedHttpFailure
            && r.target_transport.as_ref().unwrap().final_status == Some(404)
    }));
    let mut new = rows(
        &f.cfg,
        source.resource_source.as_ref().unwrap(),
        f.generation,
        "later evaluation",
        false,
    );
    new.truncate(1);
    f.store
        .complete_resources(source, &mut new, &mut [], f.generation, true)
        .unwrap();
    let after = f
        .store
        .resource_broken_sources(&request(ResourceBrokenSourcesFilter::All))
        .unwrap();
    assert_eq!(
        after
            .snapshot
            .as_ref()
            .unwrap()
            .distinct_stored_occurrence_ids,
        physical
    );
    assert_eq!(
        after.snapshot.as_ref().unwrap().evaluation_rows,
        first.snapshot.unwrap().evaluation_rows + 1
    );
    let history = after
        .rows
        .iter()
        .filter(|r| r.observation.occurrence.occurrence_id == id)
        .collect::<Vec<_>>();
    assert_eq!(history.len(), 2);
    assert_ne!(
        history[0].observation.evaluation_revision,
        history[1].observation.evaluation_revision
    );
    assert_eq!(history[0].state, ResourceBrokenSourcesState::NoGrant);
    assert_eq!(
        history[1].state,
        ResourceBrokenSourcesState::ObservedHttpFailure
    );
}
#[test]
fn resource_broken_sources_full_csv_over200_keeps_tail_formula_safe_and_exact_snapshot_filter() {
    let f = Fixture::new(205, false);
    let mut req = request(ResourceBrokenSourcesFilter::Failures);
    req.limit = 200;
    let first = f.store.resource_broken_sources(&req).unwrap();
    assert_eq!(first.rows.len(), 200);
    assert!(first.has_more);
    assert_eq!(first.selected_evaluation_rows, 206);
    let token = first.snapshot.as_ref().unwrap().snapshot_token.clone();
    req.expected_snapshot_token = Some(token.clone());
    req.after_sequence = first.next_sequence;
    let second = f.store.resource_broken_sources(&req).unwrap();
    assert_eq!(second.rows.len(), 6);
    assert!(!second.has_more);
    assert_eq!(
        second
            .rows
            .last()
            .unwrap()
            .observation
            .occurrence
            .requested_source_url,
        "https://fixture.test/b.css"
    );
    assert!(f.store.export_resource_broken_sources_csv(&req).is_err());
    req.after_sequence = None;
    let csv = f.store.export_resource_broken_sources_csv(&req).unwrap();
    assert_eq!(csv.exported_evaluation_rows, 206);
    assert_eq!(csv.distinct_exported_occurrence_ids, 206);
    assert_eq!(crate::resource_policy::digest(&csv.bytes), csv.sha256);
    let text = String::from_utf8(csv.bytes).unwrap();
    assert!(text.contains(".css\""));
    assert!(text.ends_with("\"0\"\r\n"));
    assert_eq!(text.matches("\"observed_http_failure\"").count(), 206);
    req.filter = ResourceBrokenSourcesFilter::Unknown;
    assert!(f.store.resource_broken_sources(&req).is_err());
    let mut bytes = Bytes {
        data: vec![],
        cap: 1024,
    };
    csv_record(&mut bytes, &[" \u{feff}=SUM(1,2)\r\n\"ä\"".into()]).unwrap();
    assert_eq!(
        String::from_utf8(bytes.data).unwrap(),
        "\"' \u{feff}=SUM(1,2)\r\n\"\"ä\"\"\"\r\n"
    );
    let mut tiny = Bytes {
        data: vec![],
        cap: 3,
    };
    assert!(csv_record(&mut tiny, &["large".into()]).is_err());
}
#[test]
fn resource_broken_sources_complete_transport_set_stales_same_generation_even_for_unselected_healthy_target()
 {
    let f = Fixture::new(2, false);
    let first = failures(&f.store);
    let before = first.snapshot.as_ref().unwrap();
    let mut req = request(ResourceBrokenSourcesFilter::Failures);
    req.expected_snapshot_token = Some(before.snapshot_token.clone());
    update_transport(&f.path(), |o| {
        o.projection.headers.insert(
            "x-unselected-transport-witness".into(),
            "ä\r\nnot a new fetch".into(),
        );
    });
    assert!(f.store.resource_broken_sources(&req).is_err());
    assert!(f.store.export_resource_broken_sources_csv(&req).is_err());
    let current = failures(&f.store);
    let now = current.snapshot.as_ref().unwrap();
    assert_eq!(before.saved, now.saved);
    assert_eq!(before.ledger, now.ledger);
    assert_ne!(before.transport_set_sha256, now.transport_set_sha256);
    assert_ne!(before.snapshot_token, now.snapshot_token);
}
#[test]
fn resource_broken_sources_same_read_transaction_cannot_mix_transport_changes_and_new_continuation()
{
    let f = Fixture::new(1, false);
    let c = Connection::open(f.path()).unwrap();
    let tx = c.unchecked_transaction().unwrap();
    let req = request(ResourceBrokenSourcesFilter::Failures);
    let cfg = preflight(&tx, &req).unwrap();
    crate::resource_ledger::verify(&tx, cfg.as_ref().unwrap()).unwrap();
    let old = match project(&tx, cfg.as_ref(), &req, Mode::Page).unwrap() {
        Output::Page(p) => p,
        _ => panic!("page"),
    };
    update_transport(&f.path(), |o| {
        o.projection.headers.insert(
            "x-concurrent".into(),
            "committed after reader snapshot".into(),
        );
    });
    let again = match project(&tx, cfg.as_ref(), &req, Mode::Page).unwrap() {
        Output::Page(p) => p,
        _ => panic!("page"),
    };
    assert_eq!(
        old.snapshot.as_ref().unwrap().snapshot_token,
        again.snapshot.as_ref().unwrap().snapshot_token
    );
    tx.commit().unwrap();
    let now = failures(&f.store);
    assert_ne!(
        old.snapshot.unwrap().snapshot_token,
        now.snapshot.unwrap().snapshot_token
    );
}
#[test]
fn resource_broken_sources_missing_history_is_unavailable_and_caps_hash_config_generation_and_parent_corruption_refuse()
 {
    let dir = tempfile::tempdir().unwrap();
    let empty = CrawlStore::open(dir.path().join("empty.sqlite")).unwrap();
    let p = empty.resource_broken_sources(&Default::default()).unwrap();
    assert!(!p.available);
    assert!(p.snapshot.is_none());
    assert_eq!(p.unavailable_reason, Some("saved_config_unavailable"));
    assert!(
        empty
            .export_resource_broken_sources_csv(&Default::default())
            .is_err()
    );
    let f = Fixture::new(2, false);
    let mut req = request(ResourceBrokenSourcesFilter::All);
    req.limits.max_evaluation_rows = 1;
    assert!(f.store.resource_broken_sources(&req).is_err());
    req = request(ResourceBrokenSourcesFilter::All);
    req.limits.max_transports = 1;
    assert!(f.store.resource_broken_sources(&req).is_err());
    req = request(ResourceBrokenSourcesFilter::All);
    req.limits.max_row_utf8_bytes = 10;
    assert!(f.store.resource_broken_sources(&req).is_err());
    let c = Connection::open(f.path()).unwrap();
    c.execute(
        "UPDATE resource_transports SET payload_sha256=?1 WHERE url='https://fixture.test/bad.png'",
        ["x".repeat(1000)],
    )
    .unwrap();
    assert!(failures_result(&f.store).is_err());
    drop(c);
    for attack in [
        "UPDATE metadata SET value='{}' WHERE key='config'",
        "UPDATE metadata SET value='{\"generation\":0,\"started_at\":\"\",\"finished_at\":null,\"outcome\":null}' WHERE key='run_state'",
        "INSERT INTO metadata(key,value) VALUES('operation_kind','selected_recrawl')",
        "UPDATE resource_occurrences SET data='{}' WHERE sequence=1",
    ] {
        let f = Fixture::new(1, false);
        let c = Connection::open(f.path()).unwrap();
        c.execute(attack, []).unwrap();
        assert!(
            failures_result(&f.store).is_err(),
            "corrupt input was admitted: {attack}"
        );
    }
}
fn failures_result(store: &CrawlStore) -> Result<ResourceBrokenSourcesPage> {
    store.resource_broken_sources(&request(ResourceBrokenSourcesFilter::Failures))
}

#[test]
fn resource_broken_sources_policy_fetch_alias_is_separate_from_original_resolved_value_and_status_chain()
 {
    let f = Fixture::new(2, true);
    let source = &f.sources[0];
    let original = source.resource_source.as_ref().unwrap().occurrences[0]
        .resolved_url
        .clone();
    let mut evaluations = rows(
        &f.cfg,
        source.resource_source.as_ref().unwrap(),
        f.generation,
        "policy-rewritten evaluation",
        false,
    );
    evaluations.truncate(1);
    evaluations[0].decision.grant.as_mut().unwrap().fetch_url =
        "https://fixture.test/alias.png?x=1".into();
    f.store
        .complete_resources(source, &mut evaluations, &mut [], f.generation, true)
        .unwrap();
    let mut alias = capture(
        &f.store,
        &f.cfg,
        "https://fixture.test/alias.png?x=1",
        "image/png",
        "",
        404,
        f.generation,
    );
    alias.status = Some(301);
    alias.final_url = "https://fixture.test/final.png?x=2".into();
    alias.redirect_chain = vec![crate::RedirectHop {
        url: alias.url.clone(),
        status: 301,
        target: alias.final_url.clone(),
    }];
    alias.resource_source = Some(
        crate::resource_capture::inventory(
            &alias,
            Some(""),
            ContentCaptureDecodeStatus::Exact,
            &[crate::normalize_url("https://fixture.test/", None).unwrap()],
            false,
            f.cfg.resource_discovery.as_ref().unwrap(),
            &Default::default(),
        )
        .unwrap(),
    );
    f.store
        .complete_resources(&alias, &mut [], &mut [], f.generation, false)
        .unwrap();
    let page = failures(&f.store);
    let row = page
        .rows
        .iter()
        .find(|r| r.grant_fetch_url.as_deref() == Some("https://fixture.test/alias.png?x=1"))
        .unwrap();
    assert_eq!(row.observation.occurrence.resolved_url, original);
    assert_eq!(
        row.target_transport.as_ref().unwrap().requested_url,
        "https://fixture.test/alias.png?x=1"
    );
    assert_eq!(
        row.target_transport.as_ref().unwrap().initial_status,
        Some(301)
    );
    assert_eq!(
        row.target_transport.as_ref().unwrap().final_status,
        Some(404)
    );
    assert_eq!(
        row.target_transport.as_ref().unwrap().final_url,
        "https://fixture.test/final.png?x=2"
    );
    let o = &row.observation;
    let observed = row.target_transport.as_ref().unwrap();
    let mut robots = observed.clone();
    robots.blocked_by_robots = true;
    assert_eq!(
        state(o, Some(&robots)).0,
        ResourceBrokenSourcesState::RobotsBlocked
    );
    let mut error = observed.clone();
    error.error = Some("declared read-error fixture".into());
    assert_eq!(
        state(o, Some(&error)).0,
        ResourceBrokenSourcesState::StoredFetchError
    );
    let mut proposed = observed.clone();
    proposed.final_status = None;
    assert_eq!(
        state(o, Some(&proposed)).0,
        ResourceBrokenSourcesState::UnobservedFinal
    );
    assert_eq!(state(o, None).0, ResourceBrokenSourcesState::NotFetched);
    let mut inherited = o.clone();
    inherited.basis = ResourceObservationBasis::Inherited;
    assert_eq!(
        state(&inherited, Some(observed)).0,
        ResourceBrokenSourcesState::InheritedObservation
    );
    let mut incomplete = o.clone();
    incomplete.occurrence.source_complete = false;
    assert_eq!(
        state(&incomplete, Some(observed)).0,
        ResourceBrokenSourcesState::IncompleteSource
    );
}
