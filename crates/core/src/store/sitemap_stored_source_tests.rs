//! AUTHORED / NOT_COMPILED / NOT_RUN. Child of actual A/B/parent fixtures.
use super::*;
use crate::{SitemapReadControl, SitemapStoredRowsFilter, SitemapVerifiedReadHeader};
use std::sync::atomic::{AtomicBool, Ordering};

fn captured_header(store: &CrawlStore) -> SitemapVerifiedReadHeader {
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
fn actual_stored_rows_a_only_and_committed_source_state_precede_user_output() {
    let (_dir, store, generation) = fixture(false, Default::default());
    let before_window = store
        .sitemap_read_window_with_membership_analysis(&Default::default(), None)
        .unwrap();
    let before = captured_header(&store);
    let old = rich_expected_context(&before_window);
    let batch = batch(generation, &body(), true);
    let receipt = a(&store, generation, &batch, &body());
    let staged = captured_header(&store);
    assert_eq!(staged.ledger, before.ledger);
    assert_ne!(staged.catalog_state_sha256, before.catalog_state_sha256);
    assert_eq!(
        staged
            .sitemap_source_measurement_summary
            .as_ref()
            .unwrap()
            .record_count,
        1
    );
    assert_eq!(
        staged
            .sitemap_source_measurement_summary
            .as_ref()
            .unwrap()
            .commit_count,
        0
    );
    let entered = AtomicBool::new(false);
    let error = store
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
        error.downcast_ref::<crate::SitemapReadWindowError>(),
        Some(&crate::SitemapReadWindowError::StaleContext)
    );
    assert!(!entered.load(Ordering::SeqCst));
    b(&store, generation, batch, &receipt).unwrap();
    let full = store.sitemap_read_snapshot().unwrap();
    let (committed, first, second) = store
        .with_verified_sitemap_stored_rows(
            &SitemapStoredRowsFilter::default(),
            None,
            &Default::default(),
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
    assert_eq!(
        committed
            .sitemap_source_measurement_summary
            .unwrap()
            .commit_count,
        1
    );
    assert_ne!(committed.catalog_state_sha256, staged.catalog_state_sha256);
    assert_eq!(Some(first.clone()), full.catalog.unwrap().rows);
    assert_eq!(second, first);
}

#[test]
fn actual_stored_rows_selected_presence_and_historical_none_keep_exact_parent_guards() {
    for measurements in [false, true] {
        let (_parent_dir, _parent, prepared) = prepared_parent_for_source_presence(measurements);
        let child_dir = tempfile::tempdir().unwrap();
        let child = CrawlStore::open(child_dir.path().join("stored-selected.sqlite")).unwrap();
        child.initialize_selected_recrawl(&prepared).unwrap();
        let header = captured_header(&child);
        assert!(header.materializing_selected_compatibility);
        assert!(header.ledger.is_some());
        assert_eq!(
            header.sitemap_source_measurement_summary.is_some(),
            measurements
        );
        let mut connection = child.connection.lock().unwrap();
        let tx = connection.transaction().unwrap();
        if measurements {
            tx.execute("DELETE FROM sitemap_source_measurement_commits", [])
                .unwrap();
            tx.execute("DELETE FROM sitemap_source_measurements", [])
                .unwrap();
            tx.execute("DELETE FROM metadata WHERE key='sitemap_source_store'", [])
                .unwrap();
        } else {
            tx.execute_batch("DELETE FROM sitemap_locations; DELETE FROM sitemap_entries; DELETE FROM sitemap_roots; DELETE FROM sitemap_documents; DELETE FROM metadata WHERE key IN ('sitemap_ledger','sitemap_inherited_basis');").unwrap();
        }
        tx.commit().unwrap();
        drop(connection);
        let entered = AtomicBool::new(false);
        assert!(
            child
                .with_verified_sitemap_stored_rows(
                    &Default::default(),
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
    }
    let (_parent_dir, _parent, modern) = prepared_parent_for_source_presence(false);
    // Explicit historical portable absence, not an accepted modern producer
    // deleting its evidence. Reconstruct an actual imported selected plan.
    let mut historical = modern.base_report;
    historical.sitemap_catalog = None;
    historical.sitemap_coverage_inventory = None;
    historical.sitemap_source_measurements = None;
    let bytes = crate::recrawl::bounded_report_json(&historical)
        .unwrap()
        .into_bytes();
    let prepared = crate::prepare_selected_recrawl(
        crate::SelectedRecrawlParent {
            job_id: "stored-rows-historical-none".into(),
            checkpoint_revision: None,
            source_kind: crate::RecrawlSourceKind::ImportedSnapshot,
        },
        &bytes,
        &[PAGE.into()],
        None,
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let child = CrawlStore::open(dir.path().join("stored-historical-none.sqlite")).unwrap();
    child.initialize_selected_recrawl(&prepared).unwrap();
    child
        .with_verified_sitemap_stored_rows(
            &Default::default(),
            None,
            &Default::default(),
            |session| {
                assert!(session.header().materializing_selected_compatibility);
                assert!(session.header().ledger.is_none());
                assert!(
                    session
                        .header()
                        .sitemap_source_measurement_summary
                        .is_none()
                );
                assert!(session.visit_all_filtered(|_| Ok(())).is_err());
                Ok(())
            },
        )
        .unwrap();
    let mut changed = prepared.base_snapshot_bytes;
    changed.push(b' ');
    child
        .connection
        .lock()
        .unwrap()
        .execute("UPDATE recrawl_base SET snapshot=?1 WHERE id=1", [changed])
        .unwrap();
    let entered = AtomicBool::new(false);
    assert!(
        child
            .with_verified_sitemap_stored_rows(
                &Default::default(),
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
}
