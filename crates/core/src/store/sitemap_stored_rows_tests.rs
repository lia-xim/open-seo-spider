//! AUTHORED / NOT_COMPILED / NOT_RUN. Real Store/TX controls, no supplied proof.
use super::*;
use crate::SitemapReadControlError;
use std::sync::atomic::{AtomicBool, Ordering};

fn fixture(count: usize, initialized: bool) -> (tempfile::TempDir, CrawlStore, CrawlConfig) {
    let dir = tempfile::tempdir().unwrap();
    let store = CrawlStore::open(dir.path().join("stored-rows.sqlite")).unwrap();
    let urls: Vec<_> = (0..count)
        .map(|n| format!("https://stored-rows.example/map-{n:04}.xml"))
        .collect();
    let mut config = CrawlConfig {
        seed_urls: vec!["https://stored-rows.example/".into()],
        sitemap_urls: urls.clone(),
        explicit_sitemap_inputs: Some(urls),
        obey_robots: false,
        ..Default::default()
    };
    config.prepare_input_lists().unwrap();
    store.prepare(&config).unwrap();
    let generation = store
        .begin_run("actual stored-row callback fixture")
        .unwrap();
    if initialized {
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
    (dir, store, config)
}
fn expected(header: &SitemapVerifiedReadHeader) -> SitemapReadWindowExpectedContext {
    let summary = header.ledger.as_ref().unwrap();
    SitemapReadWindowExpectedContext {
        generation: header.generation.unwrap(),
        crawl_config_sha256: header.crawl_config_sha256.clone().unwrap(),
        catalog_config_sha256: summary.config_sha256.clone(),
        catalog_revision: summary.revision,
        catalog_rolling_sha256: summary.rolling_sha256.clone(),
        catalog_state_sha256: header.catalog_state_sha256.clone().unwrap(),
        inherited_basis: header.inherited.clone(),
    }
}
fn header(store: &CrawlStore) -> SitemapVerifiedReadHeader {
    store
        .with_verified_sitemap_stored_rows(
            &Default::default(),
            None,
            &SitemapReadControl::default(),
            |session| Ok(session.header().clone()),
        )
        .unwrap()
}

#[test]
fn actual_stored_rows_two_full_visits_match_materialized_cohort_and_rich_state() {
    let (dir, store, _config) = fixture(451, true);
    let full = store.sitemap_read_snapshot().unwrap();
    let oracle = full.catalog.as_ref().unwrap().rows.as_ref().unwrap();
    assert_eq!(oracle.len(), 451);
    let rich = store
        .sitemap_read_window_with_membership_analysis(&Default::default(), None)
        .unwrap();
    let readonly = CrawlStore::open_read_only(dir.path().join("stored-rows.sqlite")).unwrap();
    let changes = readonly.connection.lock().unwrap().total_changes();
    let (actual, first, second) = readonly
        .with_verified_sitemap_stored_rows(
            &Default::default(),
            None,
            &SitemapReadControl::default(),
            |session| {
                let mut first = Vec::new();
                session.visit_all_filtered(|row| {
                    first.push(row.clone());
                    Ok(())
                })?;
                let mut second = Vec::new();
                session.visit_all_filtered(|row| {
                    second.push(row.clone());
                    Ok(())
                })?;
                Ok((session.header().clone(), first, second))
            },
        )
        .unwrap();
    assert_eq!(&first, oracle);
    assert_eq!(second, first);
    assert_eq!(actual.catalog_state_sha256, rich.catalog_state_sha256);
    assert_eq!(actual.coverage, rich.coverage);
    assert_eq!(
        actual.coverage_inventory_sha256,
        rich.coverage_inventory_sha256
    );
    assert_eq!(
        actual.analysis_context,
        rich.membership_analysis.unwrap().context
    );
    assert_eq!(actual.ledger, rich.ledger);
    assert!(!actual.materializing_selected_compatibility);
    assert_eq!(readonly.connection.lock().unwrap().total_changes(), changes);
    assert!(readonly.connection.lock().unwrap().is_autocommit());
}

#[test]
fn actual_stored_rows_exact_original_filter_and_known_empty_matches_stored_oracle() {
    let (_dir, store, config) = fixture(451, true);
    let all = store.sitemap_catalog().unwrap().unwrap().rows.unwrap();
    for (filter, expected_rows) in [
        (SitemapStoredRowsFilter::default(), all.clone()),
        (
            SitemapStoredRowsFilter {
                kind: crate::SitemapAnalysisRowKind::Root,
                ..Default::default()
            },
            all.clone(),
        ),
        (
            SitemapStoredRowsFilter {
                membership_url: Some(config.sitemap_urls[300].clone()),
                ..Default::default()
            },
            vec![all[300].clone()],
        ),
        (
            SitemapStoredRowsFilter {
                membership_url: Some("https://stored-rows.example/MAP-0300.xml".into()),
                ..Default::default()
            },
            vec![],
        ),
        (
            SitemapStoredRowsFilter {
                source_observation_id: Some("0".repeat(64)),
                ..Default::default()
            },
            vec![],
        ),
        (
            SitemapStoredRowsFilter {
                kind: crate::SitemapAnalysisRowKind::Document,
                ..Default::default()
            },
            vec![],
        ),
        (
            SitemapStoredRowsFilter {
                kind: crate::SitemapAnalysisRowKind::Entry,
                ..Default::default()
            },
            vec![],
        ),
        (
            SitemapStoredRowsFilter {
                kind: crate::SitemapAnalysisRowKind::Location,
                ..Default::default()
            },
            vec![],
        ),
    ] {
        let actual = store
            .with_verified_sitemap_stored_rows(&filter, None, &Default::default(), |session| {
                let mut rows = Vec::new();
                session.visit_all_filtered(|row| {
                    rows.push(row.clone());
                    Ok(())
                })?;
                Ok(rows)
            })
            .unwrap();
        assert_eq!(actual, expected_rows);
    }
}

#[test]
fn actual_stored_rows_stale_state_and_full_tail_corruption_never_enter_callback() {
    let (_dir, store, mut config) = fixture(8, true);
    let old = expected(&header(&store));
    config.delay_ms += 1;
    store.prepare(&config).unwrap();
    let entered = AtomicBool::new(false);
    let stale = store
        .with_verified_sitemap_stored_rows(
            &Default::default(),
            Some(&old),
            &Default::default(),
            |_| {
                entered.store(true, Ordering::SeqCst);
                Ok(())
            },
        )
        .unwrap_err();
    assert_eq!(
        stale.downcast_ref::<SitemapReadWindowError>(),
        Some(&SitemapReadWindowError::StaleContext)
    );
    assert!(!entered.load(Ordering::SeqCst));
    let current = expected(&header(&store));
    store.connection.lock().unwrap().execute(
        "UPDATE sitemap_roots SET data_sha256=?1 WHERE sequence=(SELECT max(sequence) FROM sitemap_roots)", ["0".repeat(64)],
    ).unwrap();
    // The requested exact filter excludes the bad final row. Full verification
    // must still fail before user code can output a successful CSV prefix.
    let corrupt = store
        .with_verified_sitemap_stored_rows(
            &SitemapStoredRowsFilter {
                membership_url: Some(config.sitemap_urls[0].clone()),
                ..Default::default()
            },
            Some(&current),
            &Default::default(),
            |_| {
                entered.store(true, Ordering::SeqCst);
                Ok(())
            },
        )
        .unwrap_err();
    assert!(corrupt.downcast_ref::<SitemapReadWindowError>().is_none());
    assert!(!entered.load(Ordering::SeqCst));
    assert!(store.connection.lock().unwrap().is_autocommit());
}

#[test]
fn actual_stored_rows_wal_keeps_two_passes_while_later_valid_writer_changes_generation() {
    let (dir, store, mut config) = fixture(251, true);
    let writer = CrawlStore::open(dir.path().join("stored-rows.sqlite")).unwrap();
    let before = header(&store);
    let old = expected(&before);
    let (captured, first, second) = store
        .with_verified_sitemap_stored_rows(
            &Default::default(),
            Some(&old),
            &Default::default(),
            |session| {
                let mut first = Vec::new();
                session.visit_all_filtered(|row| {
                    first.push(row.clone());
                    Ok(())
                })?;
                config.delay_ms += 1;
                writer.prepare(&config)?;
                writer.begin_run("later actual valid writer generation")?;
                let mut second = Vec::new();
                session.visit_all_filtered(|row| {
                    second.push(row.clone());
                    Ok(())
                })?;
                Ok((session.header().clone(), first, second))
            },
        )
        .unwrap();
    assert_eq!(first.len(), 251);
    assert_eq!(second, first);
    assert_eq!(captured.catalog_state_sha256, before.catalog_state_sha256);
    assert_eq!(captured.generation, before.generation);
    let current = header(&store);
    assert_ne!(current.generation, before.generation);
    assert_ne!(current.catalog_state_sha256, before.catalog_state_sha256);
}

#[test]
fn actual_stored_rows_cancel_deadline_and_callback_failure_release_real_transaction() {
    let (_dir, store, _config) = fixture(251, true);
    for timed_out in [false, true] {
        let control = SitemapReadControl::default();
        let mut visited = 0;
        let error = store
            .with_verified_sitemap_stored_rows(
                &Default::default(),
                None,
                &control,
                |session| -> anyhow::Result<()> {
                    session.visit_all_filtered(|_| {
                        visited += 1;
                        if visited == 10 {
                            if timed_out {
                                control.expire();
                            } else {
                                control.cancel();
                            }
                        }
                        Ok(())
                    })?;
                    panic!("cancelled producer cannot return successful completion")
                },
            )
            .unwrap_err();
        assert_eq!(visited, 10);
        assert_eq!(
            error.downcast_ref::<SitemapReadControlError>(),
            Some(&if timed_out {
                SitemapReadControlError::DeadlineExceeded
            } else {
                SitemapReadControlError::Cancelled
            })
        );
        assert!(store.connection.lock().unwrap().is_autocommit());
        assert!(header(&store).ledger.is_some());
    }
    #[derive(Debug)]
    struct ActualCallbackFailure;
    impl std::fmt::Display for ActualCallbackFailure {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("actual callback sink failed")
        }
    }
    impl std::error::Error for ActualCallbackFailure {}
    let error = store
        .with_verified_sitemap_stored_rows(
            &Default::default(),
            None,
            &Default::default(),
            |session| -> anyhow::Result<()> {
                session.visit_all_filtered(|_| Err(ActualCallbackFailure.into()))
            },
        )
        .unwrap_err();
    assert!(error.is::<ActualCallbackFailure>());
    assert!(store.connection.lock().unwrap().is_autocommit());
    assert!(header(&store).ledger.is_some());
}

#[test]
fn actual_stored_rows_pre_cancel_expired_control_and_invalid_filter_never_enter_callback() {
    let (_dir, store, _config) = fixture(251, true);
    for timed_out in [false, true] {
        let control = SitemapReadControl::default();
        if timed_out {
            control.expire();
        } else {
            control.cancel();
        }
        let entered = AtomicBool::new(false);
        let error = store
            .with_verified_sitemap_stored_rows(&Default::default(), None, &control, |_| {
                entered.store(true, Ordering::SeqCst);
                Ok(())
            })
            .unwrap_err();
        assert!(error.is::<SitemapReadControlError>());
        assert!(!entered.load(Ordering::SeqCst));
    }
    let entered = AtomicBool::new(false);
    assert!(
        store
            .with_verified_sitemap_stored_rows(
                &SitemapStoredRowsFilter {
                    source_observation_id: Some("INVALID".into()),
                    ..Default::default()
                },
                None,
                &Default::default(),
                |_| {
                    entered.store(true, Ordering::SeqCst);
                    Ok(())
                }
            )
            .is_err()
    );
    assert!(!entered.load(Ordering::SeqCst));
    assert!(store.connection.lock().unwrap().is_autocommit());
}

#[test]
fn actual_stored_rows_historical_none_and_known_empty_remain_distinct() {
    let (_basis_dir, basis, _config) = fixture(1, true);
    let previously_available = expected(&header(&basis));
    for initialized in [false, true] {
        let (_dir, store, _config) = fixture(0, initialized);
        store
            .with_verified_sitemap_stored_rows(
                &Default::default(),
                if initialized {
                    None
                } else {
                    Some(&previously_available)
                },
                &Default::default(),
                |session| {
                    assert_eq!(session.header().ledger.is_some(), initialized);
                    assert_eq!(session.header().catalog_state_sha256.is_some(), initialized);
                    let mut rows = 0;
                    let result = session.visit_all_filtered(|_| {
                        rows += 1;
                        Ok(())
                    });
                    assert_eq!(result.is_ok(), initialized);
                    assert_eq!(rows, 0);
                    assert!(!session.header().analysis_context.available);
                    if initialized {
                        assert_eq!(
                            session.header().coverage.completion,
                            crate::SitemapCohortCompletion::KnownEmpty
                        );
                    } else {
                        assert_eq!(
                            session.header().coverage.completion,
                            crate::SitemapCohortCompletion::Unknown
                        );
                    }
                    Ok(())
                },
            )
            .unwrap();
    }
}
