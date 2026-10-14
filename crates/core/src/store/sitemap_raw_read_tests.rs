//! AUTHORED / NOT RUN while the coordinated compute hold is active.
//! These call the actual Store, SQLite ledger and new read facade.
use super::*;

fn configured(count: usize) -> (tempfile::TempDir, CrawlStore, CrawlConfig, u64) {
    let temp = tempfile::tempdir().unwrap();
    let store = CrawlStore::open(temp.path().join("raw-window.sqlite")).unwrap();
    let urls: Vec<String> = (0..count)
        .map(|ordinal| format!("https://raw-window.example/map-{ordinal}.xml"))
        .collect();
    let mut config = CrawlConfig {
        seed_urls: vec!["https://raw-window.example/".into()],
        sitemap_urls: urls.clone(),
        explicit_sitemap_inputs: Some(urls),
        obey_robots: false,
        ..Default::default()
    };
    config.prepare_input_lists().unwrap();
    store.prepare(&config).unwrap();
    let generation = store.begin_run("actual raw window fixture").unwrap();
    (temp, store, config, generation)
}
fn initialize(store: &CrawlStore, config: &CrawlConfig, generation: u64) {
    let config_text = metadata_text(&store.connection.lock().unwrap(), "config")
        .unwrap()
        .unwrap();
    let config_sha256 = super::super::sha256(&config_text);
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
        roots: originals.iter().map(|item| item.root.clone()).collect(),
        legacy_history_unknown: false,
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

#[test]
fn actual_raw_window_preserves_unknown_vs_known_empty_and_readonly_database() {
    let (temp, store, config, generation) = configured(0);
    let request = SitemapRawReadRequest::default();
    let unknown = store.sitemap_raw_window(&request).unwrap();
    assert!(unknown.context.ledger.is_none());
    assert!(unknown.context.source_summary.is_none());
    assert!(unknown.rows.is_empty());
    initialize(&store, &config, generation);
    let known = store.sitemap_raw_window(&request).unwrap();
    assert_eq!(known.context.ledger.as_ref().unwrap().last_sequence, 0);
    assert_eq!(
        known.context.source_summary.as_ref().unwrap().record_count,
        0
    );
    assert_ne!(known.state_sha256, unknown.state_sha256);
    let connection = store.connection.lock().unwrap();
    let changes = connection.total_changes();
    drop(connection);
    let readonly = CrawlStore::open_read_only(temp.path().join("raw-window.sqlite")).unwrap();
    assert_eq!(readonly.sitemap_raw_window(&request).unwrap(), known);
    assert_eq!(store.connection.lock().unwrap().total_changes(), changes);
}

#[test]
fn actual_raw_window_pagination_matches_original_catalog_and_binds_exact_filter() {
    let (_temp, store, config, generation) = configured(7);
    initialize(&store, &config, generation);
    let original = store.sitemap_catalog().unwrap().unwrap().rows.unwrap();
    for limit in [1, 3, 1000] {
        let mut request = SitemapRawReadRequest {
            limit,
            ..Default::default()
        };
        let mut collected = Vec::new();
        loop {
            let page = store.sitemap_raw_window(&request).unwrap();
            collected.extend(page.rows);
            if !page.has_more {
                break;
            }
            request.after_sequence = page.next_after_sequence;
            request.expected_state_sha256 = Some(page.state_sha256);
        }
        assert_eq!(collected, original);
    }
    let mut request = SitemapRawReadRequest {
        membership_url: Some(config.sitemap_urls[0].clone()),
        ..Default::default()
    };
    let filtered = store.sitemap_raw_window(&request).unwrap();
    assert_eq!(filtered.rows.len(), 1);
    assert_eq!(filtered.rows[0], original[0]);
    request.expected_state_sha256 = Some(filtered.state_sha256.clone());
    request.after_sequence = Some(original[1].sequence());
    assert!(
        store.sitemap_raw_window(&request).is_err(),
        "after must match the actual original-member filter"
    );
    request.after_sequence = None;
    request.membership_url = Some(config.sitemap_urls[1].clone());
    assert!(
        store
            .sitemap_raw_window(&request)
            .unwrap_err()
            .to_string()
            .contains("state stale")
    );
}

#[test]
fn actual_raw_window_rejects_corrupt_tail_before_returning_the_requested_first_row() {
    let (_temp, store, config, generation) = configured(4);
    initialize(&store, &config, generation);
    let request = SitemapRawReadRequest {
        limit: 1,
        ..Default::default()
    };
    assert_eq!(store.sitemap_raw_window(&request).unwrap().rows.len(), 1);
    store.connection.lock().unwrap().execute(
        "UPDATE sitemap_roots SET data_sha256=?1 WHERE sequence=(SELECT max(sequence) FROM sitemap_roots)",
        ["0".repeat(64)],
    ).unwrap();
    assert!(store.sitemap_raw_window(&request).is_err());
}

#[test]
fn actual_raw_window_config_change_stales_state_and_orphan_parent_marker_is_rejected() {
    let (_temp, store, mut config, generation) = configured(3);
    initialize(&store, &config, generation);
    let mut request = SitemapRawReadRequest {
        limit: 1,
        ..Default::default()
    };
    let first = store.sitemap_raw_window(&request).unwrap();
    config.delay_ms += 1;
    store.prepare(&config).unwrap();
    let current = store.sitemap_raw_window(&request).unwrap();
    assert_eq!(current.context.ledger, first.context.ledger);
    assert_ne!(
        current.context.crawl_config_sha256,
        first.context.crawl_config_sha256
    );
    request.after_sequence = first.next_after_sequence;
    request.expected_state_sha256 = Some(first.state_sha256);
    assert!(
        store
            .sitemap_raw_window(&request)
            .unwrap_err()
            .to_string()
            .contains("state stale")
    );
    store
        .connection
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO metadata(key,value) VALUES('operation_kind','selected_recrawl')",
            [],
        )
        .unwrap();
    assert!(
        store
            .sitemap_raw_window(&SitemapRawReadRequest::default())
            .is_err()
    );
}

#[test]
fn raw_window_request_rejects_unknown_types_bounds_and_unbound_continuations() {
    for invalid in [
        serde_json::json!({"limit": 0}),
        serde_json::json!({"limit": 1001}),
        serde_json::json!({"limit": "10"}),
        serde_json::json!({"unexpected": true}),
        serde_json::json!({"after_sequence": 0, "expected_state_sha256": "a".repeat(64)}),
        serde_json::json!({"after_sequence": 1}),
        serde_json::json!({"expected_state_sha256": "A".repeat(64)}),
        serde_json::json!({"source_observation_id": "a".repeat(63)}),
        serde_json::json!({"membership_url": "https://raw-window.example/#fragment"}),
    ] {
        if let Ok(request) = serde_json::from_value::<SitemapRawReadRequest>(invalid) {
            assert!(request.validate().is_err());
        }
    }
}

#[test]
fn actual_raw_window_bounds_context_metadata_even_when_raw_history_is_absent() {
    let (_temp, store, _config, _generation) = configured(0);
    assert!(store.sitemap_raw_window(&Default::default()).is_ok());
    store
        .connection
        .lock()
        .unwrap()
        .execute(
            "UPDATE metadata SET value=CAST(zeroblob(65537) AS TEXT) WHERE key='run_state'",
            [],
        )
        .unwrap();
    let error = store.sitemap_raw_window(&Default::default()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("run_state exceeds bounded input"),
        "{error:#}"
    );
}
