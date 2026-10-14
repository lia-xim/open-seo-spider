//! AUTHORED / NOT RUN. Actual Store integration, not a supplied-header oracle.
use super::*;

fn fixture(count: usize, initialize: bool) -> (tempfile::TempDir, CrawlStore, CrawlConfig) {
    let temp = tempfile::tempdir().unwrap();
    let store = CrawlStore::open(temp.path().join("rich-window.sqlite")).unwrap();
    let urls: Vec<_> = (0..count)
        .map(|n| format!("https://rich-window.example/map-{n}.xml"))
        .collect();
    let mut config = CrawlConfig {
        seed_urls: vec!["https://rich-window.example/".into()],
        sitemap_urls: urls.clone(),
        explicit_sitemap_inputs: Some(urls),
        obey_robots: false,
        ..Default::default()
    };
    config.prepare_input_lists().unwrap();
    store.prepare(&config).unwrap();
    let generation = store.begin_run("actual rich window fixture").unwrap();
    if initialize {
        let bytes = metadata_text(&store.connection.lock().unwrap(), "config")
            .unwrap()
            .unwrap();
        let config_sha256 = crate::sitemap_ledger::digest(bytes);
        let mut originals = crate::sitemap_initial_roots::initial_roots(
            config.explicit_sitemap_inputs.as_deref(),
            &config.input_imports,
            &config_sha256,
            None,
        )
        .unwrap()
        .unwrap();
        let init = SitemapCatalogInit {
            config_sha256,
            limits: Default::default(),
            legacy_history_unknown: false,
            roots: originals.iter().map(|item| item.root.clone()).collect(),
        };
        let tasks: Vec<_> = config
            .sitemap_urls
            .iter()
            .map(|url| DiscoveryEntry {
                kind: DiscoveryKind::Sitemap,
                url: url.clone(),
            })
            .collect();
        store
            .initialize_sitemap_discovery_with(&tasks, generation, &init, |tx, roots| {
                crate::sitemap_initial_roots::finalize_ordinary_roots(
                    tx,
                    &mut originals,
                    &config.sitemap_urls,
                )?;
                *roots = originals.into_iter().map(|item| item.root).collect();
                Ok(())
            })
            .unwrap();
    }
    (temp, store, config)
}
fn request(limit: usize) -> crate::SitemapMembershipAnalysisRequest {
    crate::SitemapMembershipAnalysisRequest {
        row_limit: limit,
        page_limit: 0,
        ..Default::default()
    }
}
fn expected(read: &SitemapReadWindow) -> SitemapReadWindowExpectedContext {
    let ledger = read.ledger.as_ref().unwrap();
    SitemapReadWindowExpectedContext {
        generation: read.generation.unwrap(),
        crawl_config_sha256: read.crawl_config_sha256.clone().unwrap(),
        catalog_config_sha256: ledger.config_sha256.clone(),
        catalog_revision: ledger.revision,
        catalog_rolling_sha256: ledger.rolling_sha256.clone(),
        catalog_state_sha256: read.catalog_state_sha256.clone().unwrap(),
        inherited_basis: read.inherited.clone(),
    }
}

#[test]
fn actual_rich_window_agrees_with_full_snapshot_context_and_exact_inventory_hash() {
    let (_temp, store, _config) = fixture(5, true);
    let req = request(2);
    let full = store
        .sitemap_read_snapshot_with_membership_analysis(&req)
        .unwrap();
    let window = store
        .sitemap_read_window_with_membership_analysis(&req, None)
        .unwrap();
    let catalog = full.catalog.as_ref().unwrap();
    assert_eq!(window.ledger.as_ref(), Some(&catalog.summary));
    assert_eq!(window.inherited, catalog.inherited);
    assert_eq!(window.coverage, full.coverage);
    assert_eq!(
        window.coverage_inventory_sha256,
        full.coverage_inventory
            .as_ref()
            .map(|inventory| crate::sitemap_ledger::digest(serde_json::to_vec(inventory).unwrap()))
    );
    assert_eq!(
        window.sitemap_source_measurement_summary.as_ref(),
        full.sitemap_source_measurements
            .as_ref()
            .map(|snapshot| &snapshot.summary)
    );
    assert_eq!(
        window.sitemap_source_measurement_context,
        full.sitemap_source_measurement_context
    );
    assert_eq!(window.crawl_config_sha256, full.crawl_config_sha256);
    assert_eq!(window.generation, full.generation);
    assert_eq!(window.membership_analysis, full.membership_analysis);
    assert_eq!(
        window.rows.as_ref().unwrap().as_slice(),
        &catalog.rows.as_ref().unwrap()[..2]
    );
    assert!(window.has_more);
    assert_eq!(window.reader_engine, SITEMAP_READ_WINDOW_ENGINE);
    assert!(!window.materializing_selected_compatibility);
    let full_state = sitemap_read_state_sha256(&SitemapReadState {
        summary: &catalog.summary,
        inherited: catalog.inherited.as_ref(),
        coverage_inventory_sha256: window.coverage_inventory_sha256.as_deref(),
        coverage: &full.coverage,
        source_measurement_context: &full.sitemap_source_measurement_context,
        analysis_context: full
            .membership_analysis
            .as_ref()
            .map(|analysis| &analysis.context),
    })
    .unwrap();
    assert_eq!(
        window.catalog_state_sha256.as_deref(),
        Some(full_state.as_str())
    );
}

#[test]
fn actual_rich_window_stale_context_precedes_invalid_after_and_current_after_is_typed() {
    let (_temp, store, mut config) = fixture(5, true);
    let first = store
        .sitemap_read_window_with_membership_analysis(&request(1), None)
        .unwrap();
    let old_expected = expected(&first);
    let mut next = request(2);
    next.after_sequence = first.next_after_sequence;
    let second = store
        .sitemap_read_window_with_membership_analysis(&next, Some(&old_expected))
        .unwrap();
    assert_eq!(second.rows.as_ref().unwrap()[0].sequence(), 2);
    assert_eq!(second.rows.as_ref().unwrap().len(), 2);
    config.delay_ms += 1;
    store.prepare(&config).unwrap();
    next.after_sequence = Some(999_999);
    let stale = store
        .sitemap_read_window_with_membership_analysis(&next, Some(&old_expected))
        .unwrap_err();
    assert_eq!(
        stale.downcast_ref::<SitemapReadWindowError>(),
        Some(&SitemapReadWindowError::StaleContext)
    );
    let current = store
        .sitemap_read_window_with_membership_analysis(&request(0), None)
        .unwrap();
    let invalid = store
        .sitemap_read_window_with_membership_analysis(&next, Some(&expected(&current)))
        .unwrap_err();
    assert_eq!(
        invalid.downcast_ref::<SitemapReadWindowError>(),
        Some(&SitemapReadWindowError::InvalidContinuation)
    );
}

#[test]
fn actual_rich_window_pending_task_only_change_invalidates_state_without_raw_revision_change() {
    let (_temp, store, _config) = fixture(0, true);
    let before = store
        .sitemap_read_window_with_membership_analysis(&request(1), None)
        .unwrap();
    assert_eq!(
        before.coverage.completion,
        crate::SitemapCohortCompletion::KnownEmpty
    );
    store.connection.lock().unwrap().execute(
        "INSERT INTO discovery_frontier(kind,url,done) VALUES('robots','https://rich-window.example/',0)", [],
    ).unwrap();
    let after = store
        .sitemap_read_window_with_membership_analysis(&request(1), None)
        .unwrap();
    assert_eq!(before.ledger, after.ledger);
    assert_eq!(before.rows, after.rows);
    assert_ne!(before.catalog_state_sha256, after.catalog_state_sha256);
    assert_eq!(
        after.coverage.completion,
        crate::SitemapCohortCompletion::Unknown
    );
    assert!(
        after
            .coverage
            .reason_counts
            .contains_key(&crate::SitemapCoverageReason::PendingSource)
    );
    let error = store
        .sitemap_read_window_with_membership_analysis(&request(1), Some(&expected(&before)))
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<SitemapReadWindowError>(),
        Some(&SitemapReadWindowError::StaleContext)
    );
}

#[test]
fn actual_rich_window_unknown_history_and_exact_member_continuation_remain_distinct() {
    let (_temp, store, _config) = fixture(0, false);
    let unknown = store
        .sitemap_read_window_with_membership_analysis(&request(1), None)
        .unwrap();
    assert!(unknown.ledger.is_none());
    assert!(unknown.rows.is_none());
    assert!(unknown.catalog_state_sha256.is_none());
    let (_temp, store, config) = fixture(3, true);
    let mut req = request(2);
    req.membership_url = Some(config.sitemap_urls[0].clone());
    let filtered = store
        .sitemap_read_window_with_membership_analysis(&req, None)
        .unwrap();
    assert_eq!(filtered.rows.as_ref().unwrap().len(), 1);
    req.after_sequence = Some(2);
    let error = store
        .sitemap_read_window_with_membership_analysis(&req, Some(&expected(&filtered)))
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<SitemapReadWindowError>(),
        Some(&SitemapReadWindowError::InvalidContinuation)
    );
}

#[test]
fn actual_rich_window_tail_corruption_is_not_reclassified_as_a_stale_cursor() {
    let (_temp, store, _config) = fixture(4, true);
    let before = store
        .sitemap_read_window_with_membership_analysis(&request(1), None)
        .unwrap();
    store.connection.lock().unwrap().execute(
        "UPDATE sitemap_roots SET data_sha256=?1 WHERE sequence=(SELECT max(sequence) FROM sitemap_roots)", ["0".repeat(64)],
    ).unwrap();
    let error = store
        .sitemap_read_window_with_membership_analysis(&request(1), Some(&expected(&before)))
        .unwrap_err();
    assert!(error.downcast_ref::<SitemapReadWindowError>().is_none());
}

#[test]
fn actual_rich_window_readonly_preserves_rows_and_bounds_unknown_history_metadata() {
    let (temp, store, _config) = fixture(2, true);
    let before = store
        .sitemap_read_window_with_membership_analysis(&request(1), None)
        .unwrap();
    let changes = store.connection.lock().unwrap().total_changes();
    let readonly = CrawlStore::open_read_only(temp.path().join("rich-window.sqlite")).unwrap();
    let after = readonly
        .sitemap_read_window_with_membership_analysis(&request(1), None)
        .unwrap();
    assert_eq!(
        serde_json::to_value(after).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    assert_eq!(store.connection.lock().unwrap().total_changes(), changes);
    let (_temp, legacy, _config) = fixture(0, false);
    legacy.connection.lock().unwrap().execute(
        "INSERT INTO metadata(key,value) VALUES('discovery_initialized',CAST(zeroblob(65537) AS TEXT))", [],
    ).unwrap();
    assert!(
        legacy
            .sitemap_read_window_with_membership_analysis(&request(1), None)
            .unwrap_err()
            .to_string()
            .contains("discovery_initialized exceeds bounded input")
    );
}
