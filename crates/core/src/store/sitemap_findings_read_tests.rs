//! AUTHORED / NOT_COMPILED / NOT_RUN. Actual Store/TX with legacy task evidence.
use super::*;
use crate::store::{DiscoveryEntry, DiscoveryKind};
use crate::{CrawlConfig, SitemapReadControlError};
use rusqlite::Connection;
use std::sync::atomic::{AtomicBool, Ordering};

fn fixture(count: usize) -> (tempfile::TempDir, CrawlStore) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("findings.sqlite");
    let store = CrawlStore::open(&path).unwrap();
    store
        .connection
        .lock()
        .unwrap()
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    let config = CrawlConfig {
        seed_urls: vec!["https://findings.example/".into()],
        obey_robots: false,
        ..Default::default()
    };
    store.prepare(&config).unwrap();
    store
        .begin_run("actual legacy task finding fixture")
        .unwrap();
    let mut tasks: Vec<_> = (0..count)
        .map(|n| DiscoveryEntry {
            kind: DiscoveryKind::Sitemap,
            url: format!("https://findings.example/map-{n:04}.xml"),
        })
        .collect();
    tasks.push(DiscoveryEntry {
        kind: DiscoveryKind::Robots,
        url: "https://findings.example/robots.txt".into(),
    });
    store.initialize_discovery(&tasks).unwrap();
    (dir, store)
}
fn source_query() -> SitemapFindingsRequest {
    SitemapFindingsRequest {
        selection: SitemapFindingSelection {
            filter: SitemapFindingFilter::XmlSitemapOver50mb,
            assessment: SitemapFindingAssessmentSelection::Unknown,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn next_request(
    query: &SitemapFindingsRequest,
    page: &SitemapFindingsPage,
) -> SitemapFindingsRequest {
    let continuation = page.continuation.as_ref().unwrap();
    SitemapFindingsRequest {
        after: Some(continuation.after.clone()),
        expected_context_sha256: Some(continuation.context_sha256.clone()),
        ..query.clone()
    }
}

#[test]
fn findings_actual_legacy_tasks_are_unknown_not_fabricated_measurements_or_robots_xml() {
    let (_dir, store) = fixture(3);
    let page = store
        .sitemap_findings_page(&source_query(), &Default::default())
        .unwrap();
    assert_eq!(
        page.context.availability,
        SitemapFindingsAvailability::AvailableWithUnknownHistory
    );
    assert!(page.context.basis.ledger.is_none());
    assert!(
        page.context
            .basis
            .sitemap_source_measurement_summary
            .is_none()
    );
    let scope = page.context.source_scope.unwrap();
    assert_eq!(scope.current_frontier_task_count, Some(4));
    assert!(scope.current_task_inventory_available);
    assert!(!scope.measurement_history_available);
    assert!(scope.unknown_history);
    let rows = page.rows.unwrap();
    assert_eq!(rows.len(), 3);
    for row in rows {
        assert_eq!(row.assessment.verdict, SitemapFindingVerdict::Unknown);
        let SitemapFindingEvidence::SourceAttempt { source } = row.evidence else {
            panic!("source domain")
        };
        let SitemapSourceFindingEvidence::UnmeasuredTask { task } = *source else {
            panic!("actual task evidence")
        };
        assert_eq!(task.kind, crate::SitemapSourceKind::SitemapXml);
        assert!(!task.done);
    }
    assert!(page.scan.exhausted);
    assert_eq!(page.scan.more_matches, Some(false));
}

#[test]
fn findings_sparse_zero_match_windows_progress_to_actual_match_after_four_hundred() {
    let (_dir, store) = fixture(451);
    let mut query = source_query();
    query.limit = 1;
    query.candidate_limit = 100;
    query.selection.source_task_url = Some("https://findings.example/map-0450.xml".into());
    let mut seen = Vec::new();
    let mut empty_windows = 0;
    loop {
        let page = store
            .sitemap_findings_page(&query, &Default::default())
            .unwrap();
        let rows = page.rows.as_ref().unwrap();
        if rows.is_empty() {
            empty_windows += 1;
            assert!(!page.scan.exhausted);
            assert_eq!(page.scan.stop, SitemapFindingsStop::CandidateBudget);
            assert_eq!(page.scan.more_matches, None);
            assert_eq!(page.scan.evaluated_candidates, 100);
        }
        seen.extend(rows.iter().map(|row| row.position.clone()));
        if page.continuation.is_none() {
            assert!(page.scan.exhausted);
            break;
        }
        query = next_request(&query, &page);
        assert!(empty_windows <= 5);
    }
    assert_eq!(empty_windows, 4);
    assert_eq!(
        seen,
        vec![SitemapFindingPosition::UnmeasuredSourceTask { ordinal: 451 }]
    );
}

#[test]
fn findings_source_pages_one_ten_two_hundred_equal_same_tx_full_visitor_without_duplicates() {
    let (_dir, store) = fixture(451);
    let expected = store
        .with_verified_sitemap_findings(
            &source_query().selection,
            None,
            &Default::default(),
            |session| {
                let mut first = Vec::new();
                session.visit_all(|row| {
                    first.push(row.clone());
                    Ok(())
                })?;
                let mut second = Vec::new();
                session.visit_all(|row| {
                    second.push(row.clone());
                    Ok(())
                })?;
                assert_eq!(first, second);
                Ok(first)
            },
        )
        .unwrap();
    for limit in [1, 10, 200] {
        let mut query = source_query();
        query.limit = limit;
        let mut actual = Vec::new();
        loop {
            let page = store
                .sitemap_findings_page(&query, &Default::default())
                .unwrap();
            actual.extend(page.rows.as_ref().unwrap().iter().cloned());
            if page.continuation.is_none() {
                assert!(page.scan.exhausted);
                break;
            }
            assert_eq!(page.scan.stop, SitemapFindingsStop::MoreMatches);
            assert_eq!(page.scan.more_matches, Some(true));
            assert_eq!(
                page.continuation.as_ref().unwrap().after,
                actual.last().unwrap().position
            );
            query = next_request(&query, &page);
            assert!(actual.len() <= 451);
        }
        assert_eq!(actual, expected);
    }
}

#[test]
fn findings_known_source_match_selection_does_not_turn_unmeasured_tasks_into_within_limit() {
    let (_dir, store) = fixture(3);
    for assessment in [
        SitemapFindingAssessmentSelection::Match,
        SitemapFindingAssessmentSelection::NotMatched,
    ] {
        let mut query = source_query();
        query.selection.assessment = assessment;
        let page = store
            .sitemap_findings_page(&query, &Default::default())
            .unwrap();
        assert!(page.rows.unwrap().is_empty());
        assert!(page.scan.exhausted);
        assert_eq!(
            page.context.availability,
            SitemapFindingsAvailability::AvailableWithUnknownHistory
        );
    }
}

#[test]
fn findings_invalid_query_role_or_missing_expected_is_typed_preflight_without_writes() {
    let (_dir, store) = fixture(1);
    let changes = store.connection.lock().unwrap().total_changes();
    let mut invalid = Vec::new();
    let mut q = source_query();
    q.limit = 0;
    invalid.push(q);
    let mut q = source_query();
    q.candidate_limit = 5_001;
    invalid.push(q);
    let mut q = source_query();
    q.selection.requested_url = Some("https://findings.example/".into());
    invalid.push(q);
    let mut q = source_query();
    q.after = Some(SitemapFindingPosition::UnmeasuredSourceTask { ordinal: 1 });
    invalid.push(q);
    let mut q = source_query();
    q.after = Some(SitemapFindingPosition::SavedPage { ordinal: 0 });
    q.expected_context_sha256 = Some("0".repeat(64));
    invalid.push(q);
    for query in invalid {
        assert!(
            store
                .sitemap_findings_page(&query, &Default::default())
                .unwrap_err()
                .downcast_ref::<SitemapFindingsError>()
                .is_some()
        );
    }
    assert_eq!(store.connection.lock().unwrap().total_changes(), changes);
    assert!(store.connection.lock().unwrap().is_autocommit());
}

#[test]
fn findings_changed_selector_stale_context_precedes_nonexistent_position_and_callback() {
    let (_dir, store) = fixture(3);
    let original = source_query();
    let first = store
        .sitemap_findings_page(&original, &Default::default())
        .unwrap();
    let mut changed = original.clone();
    changed.selection.search = Some("not-found".into());
    changed.after = Some(SitemapFindingPosition::UnmeasuredSourceTask { ordinal: 99_999 });
    changed.expected_context_sha256 = Some(first.context.context_sha256.clone());
    assert_eq!(
        store
            .sitemap_findings_page(&changed, &Default::default())
            .unwrap_err()
            .downcast_ref::<SitemapFindingsError>(),
        Some(&SitemapFindingsError::StaleContext)
    );
    let entered = AtomicBool::new(false);
    assert!(
        store
            .with_verified_sitemap_findings(
                &changed.selection,
                Some(&first.context.context_sha256),
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

#[test]
fn findings_raw_none_actual_task_mutation_invalidates_context_before_cursor_lookup() {
    let (_dir, store) = fixture(3);
    let first = store
        .sitemap_findings_page(&source_query(), &Default::default())
        .unwrap();
    assert!(first.context.basis.catalog_state_sha256.is_none());
    store
        .connection
        .lock()
        .unwrap()
        .execute(
            "UPDATE discovery_frontier SET done=1 WHERE kind='sitemap' AND url=?1",
            ["https://findings.example/map-0002.xml"],
        )
        .unwrap();
    let changed = store
        .sitemap_findings_page(&source_query(), &Default::default())
        .unwrap();
    assert_ne!(first.context.context_sha256, changed.context.context_sha256);
    let mut old = source_query();
    old.after = Some(SitemapFindingPosition::UnmeasuredSourceTask { ordinal: 999 });
    old.expected_context_sha256 = Some(first.context.context_sha256);
    assert_eq!(
        store
            .sitemap_findings_page(&old, &Default::default())
            .unwrap_err()
            .downcast_ref::<SitemapFindingsError>(),
        Some(&SitemapFindingsError::StaleContext)
    );
}

#[test]
fn findings_unavailable_saved_analysis_is_none_not_empty_and_full_visit_fails() {
    let (_dir, store) = fixture(1);
    let request = SitemapFindingsRequest::default();
    let page = store
        .sitemap_findings_page(&request, &Default::default())
        .unwrap();
    assert_eq!(
        page.context.availability,
        SitemapFindingsAvailability::Unavailable
    );
    assert!(page.rows.is_none());
    assert!(!page.scan.exhausted);
    assert_eq!(page.scan.stop, SitemapFindingsStop::Unavailable);
    store
        .with_verified_sitemap_findings(&request.selection, None, &Default::default(), |session| {
            let error = session
                .visit_all(|_| panic!("unavailable cannot yield a row"))
                .unwrap_err();
            assert_eq!(
                error.downcast_ref::<SitemapFindingsError>(),
                Some(&SitemapFindingsError::Unavailable)
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn findings_actual_readonly_same_tx_visits_survive_wal_task_writer_without_requests_or_writes() {
    let (dir, original) = fixture(3);
    let path = dir.path().join("findings.sqlite");
    drop(original);
    let readonly = CrawlStore::open_read_only(&path).unwrap();
    let changes = readonly.connection.lock().unwrap().total_changes();
    let before_context = readonly
        .with_verified_sitemap_findings(
            &source_query().selection,
            None,
            &Default::default(),
            |session| {
                let mut before = Vec::new();
                session.visit_all(|row| {
                    before.push(row.clone());
                    Ok(())
                })?;
                let writer = Connection::open(&path).unwrap();
                assert_eq!(
                    writer
                        .execute(
                            "UPDATE discovery_frontier SET done=1 WHERE kind='sitemap'",
                            []
                        )
                        .unwrap(),
                    3
                );
                let mut after = Vec::new();
                session.visit_all(|row| {
                    after.push(row.clone());
                    Ok(())
                })?;
                assert_eq!(before, after);
                Ok(session.context().context_sha256.clone())
            },
        )
        .unwrap();
    let after = readonly
        .sitemap_findings_page(&source_query(), &Default::default())
        .unwrap();
    assert_ne!(before_context, after.context.context_sha256);
    assert_eq!(readonly.connection.lock().unwrap().total_changes(), changes);
    assert!(readonly.connection.lock().unwrap().is_autocommit());
}

#[test]
fn findings_pre_cancelled_control_prevents_user_callback_and_restores_typed_stop() {
    let (_dir, store) = fixture(1);
    let control = SitemapReadControl::default();
    control.cancel();
    let entered = AtomicBool::new(false);
    let error = store
        .with_verified_sitemap_findings(&source_query().selection, None, &control, |_| {
            entered.store(true, Ordering::SeqCst);
            Ok(())
        })
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<SitemapReadControlError>(),
        Some(&SitemapReadControlError::Cancelled)
    );
    assert!(!entered.load(Ordering::SeqCst));
}

#[test]
fn findings_hash_sink_uses_exact_typed_serialization_and_inclusive_budget() {
    let value = serde_json::json!({"literal":"quotes\" slash\\ control\u{0001}"});
    let bytes = serde_json::to_vec(&value).unwrap();
    let control = SitemapReadControl::default();
    let hash = bounded_json_sha256(
        &value,
        bytes.len(),
        SitemapFindingsError::SelectionByteBudget,
        &control,
    )
    .unwrap();
    assert_eq!(hash, format!("{:x}", Sha256::digest(&bytes)));
    assert_eq!(
        bounded_json_bytes(
            &value,
            bytes.len(),
            SitemapFindingsError::ResponseByteBudget,
            &control
        )
        .unwrap(),
        bytes.len()
    );
    assert_eq!(
        bounded_json_bytes(
            &value,
            bytes.len() - 1,
            SitemapFindingsError::ResponseByteBudget,
            &control
        )
        .unwrap_err()
        .downcast_ref::<SitemapFindingsError>(),
        Some(&SitemapFindingsError::ResponseByteBudget)
    );
}

#[test]
fn findings_valid_large_legacy_task_reports_response_budget_without_narrowing_input_or_dropping_row()
 {
    let (_dir, store) = fixture(0);
    let url = format!("https://findings.example/{}", "a".repeat(8 * 1024 * 1024));
    store
        .connection
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO discovery_frontier(kind,url,done) VALUES('sitemap',?1,0)",
            [&url],
        )
        .unwrap();
    let query = source_query();
    let error = store
        .sitemap_findings_page(&query, &Default::default())
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<SitemapFindingsError>(),
        Some(&SitemapFindingsError::ResponseByteBudget)
    );
    let mut actual = 0;
    store
        .with_verified_sitemap_findings(&query.selection, None, &Default::default(), |session| {
            session.visit_all(|row| {
                let SitemapFindingEvidence::SourceAttempt { source } = &row.evidence else {
                    panic!("source")
                };
                assert_eq!(source.original_task_url(), url);
                actual += 1;
                Ok(())
            })
        })
        .unwrap();
    assert_eq!(actual, 1);
}
