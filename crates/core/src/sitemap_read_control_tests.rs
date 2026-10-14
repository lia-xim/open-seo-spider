//! AUTHORED / NOT COMPILED / NOT RUN while the compute hold is active.
//! Real connections/Store writers below are directed fixtures, never fake read proofs.
use super::*;
use crate::sitemap_coverage::verified_read::{
    CoverageReadContext, project_read, project_read_with_control,
};
use crate::sitemap_ledger::verified_read::{verify_raw, verify_raw_with_control};
use crate::sitemap_membership_analysis::{
    AnalysisReadInput, prepare_window_analysis, prepare_window_analysis_with_control,
};
use crate::sitemap_source_store::verified_read::{verify_source, verify_source_with_control};
use crate::store::{DiscoveryEntry, DiscoveryKind, SitemapCatalogInit};
use crate::{CrawlConfig, CrawlStore};
use std::{sync::mpsc, time::Duration};

fn stop<T>(result: Result<T>, expected: SitemapReadControlError) {
    let error = match result {
        Ok(_) => panic!("Expected typed sitemap stop"),
        Err(error) => error,
    };
    assert_eq!(
        error.downcast_ref::<SitemapReadControlError>(),
        Some(&expected),
        "{error:#}"
    );
}

#[test]
fn control_default_clones_and_first_observed_stop_are_sticky() {
    let control = SitemapReadControl::default();
    control.checkpoint().unwrap();
    let clone = control.clone();
    clone.cancel();
    control.expire();
    control.set_deadline(None);
    stop(control.checkpoint(), SitemapReadControlError::Cancelled);
    stop(clone.checkpoint(), SitemapReadControlError::Cancelled);
    let expired = SitemapReadControl::default();
    expired.expire();
    expired.cancel();
    stop(
        expired.checkpoint(),
        SitemapReadControlError::DeadlineExceeded,
    );
}

#[test]
fn control_deadlines_are_shared_movable_clearable_and_typed() {
    let now = Instant::now();
    let control = SitemapReadControl::with_deadline(now + Duration::from_secs(3600));
    control.clone().set_deadline(None);
    control.checkpoint().unwrap();
    control.set_deadline(Some(now.checked_sub(Duration::from_secs(1)).unwrap()));
    stop(
        control.clone().checkpoint(),
        SitemapReadControlError::DeadlineExceeded,
    );
    control.set_deadline(Some(now + Duration::from_secs(7200)));
    stop(
        control.checkpoint(),
        SitemapReadControlError::DeadlineExceeded,
    );
    stop(
        SitemapReadControl::with_deadline(now).checkpoint(),
        SitemapReadControlError::DeadlineExceeded,
    );
}

#[test]
fn control_recovers_only_its_own_poisoned_state_lock() {
    let control = SitemapReadControl::default();
    let poison = control.clone();
    assert!(
        std::thread::spawn(move || {
            let _locked = poison.inner.state.lock().unwrap();
            panic!("directed private mutex poison");
        })
        .join()
        .is_err()
    );
    control.set_deadline(None);
    control.checkpoint().unwrap();
    control.cancel();
    stop(control.checkpoint(), SitemapReadControlError::Cancelled);
}

#[test]
fn control_owned_interrupt_guards_drop_only_their_attachment_and_outlive_connection() {
    let control = SitemapReadControl::default();
    let mut first = Connection::open_in_memory().unwrap();
    let second = Connection::open_in_memory().unwrap();
    let a = control.bind_connection(&first).unwrap();
    let b = control.bind_connection(&second).unwrap();
    assert_eq!(control.state().attachments.len(), 2);
    // An owned guard must not borrow first: mutable TX/commit and close remain possible.
    first.transaction().unwrap().commit().unwrap();
    first.close().unwrap();
    drop(a);
    assert_eq!(control.state().attachments.len(), 1);
    assert!(control.state().attachments.contains_key(&b.id));
    control.cancel(); // Actual second handle still attached; closed first is gone.
    drop(b);
    assert!(control.state().attachments.is_empty());
    assert!(control.bind_connection(&second).is_err());
    assert!(control.state().attachments.is_empty());
}

#[test]
fn control_maps_actual_sql_interrupt_but_preserves_unrelated_errors() {
    let control = SitemapReadControl::default();
    let thread_control = control.clone();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let connection = Connection::open_in_memory().unwrap();
        let _binding = thread_control.bind_connection(&connection).unwrap();
        ready_tx.send(connection.get_interrupt_handle()).unwrap();
        // Deliberately enter actual SQL without an initial run/checkpoint: this
        // fixture must exercise the real handle, not win via a pre-SQL stop.
        let raw: Result<i64> = connection.query_row("WITH RECURSIVE t(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM t WHERE n<1000000000) SELECT sum(n) FROM t", [], |r| r.get(0)).map_err(Into::into);
        let actual_interrupt = raw.as_ref().err().is_some_and(|e| matches!(e.downcast_ref::<rusqlite::Error>(),Some(rusqlite::Error::SqliteFailure(code,_)) if code.code==rusqlite::ErrorCode::OperationInterrupted));
        // Match the SQL -> serde custom -> outer reader mapping boundary.
        let erased: Result<i64> = thread_control
            .map_result(raw)
            .map_err(|e| <serde_json::Error as serde::de::Error>::custom(e.to_string()).into());
        let result = thread_control.map_result(erased);
        done_tx.send((result, actual_interrupt)).unwrap();
    });
    let emergency = ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    // Repeat notification covers SQLite's nonpersistent interrupt-before-step race.
    let mut finished = None;
    for _ in 0..200 {
        control.cancel();
        match done_rx.recv_timeout(Duration::from_millis(10)) {
            Ok(value) => {
                finished = Some(value);
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(error) => panic!("worker disconnected: {error}"),
        }
    }
    let (result, actual_interrupt) = finished.unwrap_or_else(|| {
        emergency.interrupt();
        done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("owned SQL worker must stop")
    });
    worker.join().unwrap();
    assert!(
        actual_interrupt,
        "Actual SQL must return SQLITE_INTERRUPT before serde conversion"
    );
    stop(result, SitemapReadControlError::Cancelled);
    let unrelated: Result<()> = Err(anyhow::anyhow!("Directed corrupt payload"));
    let error = control.map_result(unrelated).unwrap_err();
    assert!(error.downcast_ref::<SitemapReadControlError>().is_none());
    assert_eq!(error.to_string(), "Directed corrupt payload");
    let external = SitemapReadControl::default();
    let interrupted: Result<()> = Err(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERRUPT),
        None,
    )
    .into());
    let error = external.map_result(interrupted).unwrap_err();
    assert!(error.downcast_ref::<SitemapReadControlError>().is_none());
    assert!(error.downcast_ref::<rusqlite::Error>().is_some());
}

#[test]
fn control_chunk_hash_preserves_exact_bytes_and_stops_before_more_work() {
    use sha2::Digest;
    let bytes = vec![b'x'; 131_073];
    let control = SitemapReadControl::default();
    assert_eq!(
        control.sha256(&bytes).unwrap(),
        format!("{:x}", sha2::Sha256::digest(&bytes))
    );
    control.expire();
    stop(
        control.sha256(&bytes),
        SitemapReadControlError::DeadlineExceeded,
    );
    // Restore typed control erased into a serde custom-error string.
    let erased: Result<()> = Err(<serde_json::Error as serde::ser::Error>::custom(
        "Sitemap read deadline exceeded",
    )
    .into());
    stop(
        control.map_result(erased),
        SitemapReadControlError::DeadlineExceeded,
    );
}

struct Fixture {
    _dir: tempfile::TempDir,
    path: std::path::PathBuf,
    config_bytes: Vec<u8>,
    sha: String,
    generation: u64,
}
impl Fixture {
    fn new(root: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("read.sqlite");
        let store = CrawlStore::open(&path).unwrap();
        let inputs = if root {
            vec!["https://example.org/root.xml".to_string()]
        } else {
            vec![]
        };
        let config = CrawlConfig {
            seed_urls: vec!["https://example.org/".into()],
            obey_robots: false,
            explicit_sitemap_inputs: Some(inputs.clone()),
            sitemap_urls: inputs.clone(),
            ..Default::default()
        };
        store.prepare(&config).unwrap();
        let generation = store.begin_run("controlled-read").unwrap();
        let connection = Connection::open(&path).unwrap();
        let config_bytes: Vec<u8> = connection
            .query_row(
                "SELECT CAST(value AS BLOB) FROM metadata WHERE key='config'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let sha = crate::sitemap_ledger::digest(&config_bytes);
        let roots = crate::sitemap_initial_roots::initial_roots(Some(&inputs), &[], &sha, None)
            .unwrap()
            .unwrap()
            .into_iter()
            .map(|mut v| {
                v.root.admission.decision = crate::SitemapAdmissionDecision::Scheduled;
                v.root.admission.admitted_fetch_url = v.root.normalized_document_url.clone();
                v.root
            })
            .collect();
        let entries = inputs
            .into_iter()
            .map(|url| DiscoveryEntry {
                kind: DiscoveryKind::Sitemap,
                url,
            })
            .collect::<Vec<_>>();
        store
            .initialize_sitemap_discovery(
                &entries,
                generation,
                &SitemapCatalogInit {
                    config_sha256: sha.clone(),
                    limits: Default::default(),
                    roots,
                    legacy_history_unknown: false,
                },
            )
            .unwrap();
        Self {
            _dir: dir,
            path,
            config_bytes,
            sha,
            generation,
        }
    }
    fn read_only(&self) -> Connection {
        Connection::open_with_flags(&self.path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap()
    }
    fn context(&self) -> CoverageReadContext<'_> {
        CoverageReadContext {
            saved_config_bytes: Some(&self.config_bytes),
            saved_config_sha256: Some(&self.sha),
            generation: Some(self.generation),
            discovery_initialized: true,
        }
    }
}

#[test]
fn controlled_default_real_empty_store_matches_all_old_reader_results_without_mutation() {
    let fixture = Fixture::new(false);
    let mut connection = fixture.read_only();
    let tx = connection.transaction().unwrap();
    let changes = tx.total_changes();
    let control = SitemapReadControl::default();
    let old_raw = verify_raw(&tx).unwrap().unwrap();
    let raw = verify_raw_with_control(&tx, &control).unwrap().unwrap();
    assert_eq!(
        old_raw.window(&Default::default(), None, 20).unwrap(),
        raw.window(&Default::default(), None, 20).unwrap()
    );
    let old_source = verify_source(&tx, Some(&old_raw)).unwrap().unwrap();
    let source = verify_source_with_control(&tx, Some(&raw), &control)
        .unwrap()
        .unwrap();
    assert_eq!(old_source.summary(), source.summary());
    assert_eq!(old_source.context(), source.context());
    let old_coverage = project_read(&tx, Some(&old_raw), fixture.context()).unwrap();
    let coverage = project_read_with_control(&tx, Some(&raw), fixture.context(), &control).unwrap();
    assert_eq!(old_coverage.coverage, coverage.coverage);
    assert_eq!(old_coverage.inventory_sha256, coverage.inventory_sha256);
    let input = AnalysisReadInput {
        raw: Some(&raw),
        source: Some(&source),
        generation: Some(fixture.generation),
        crawl_config_sha256: Some(&fixture.sha),
        coverage: &coverage.coverage,
    };
    let old_analysis = prepare_window_analysis(&tx, &input).unwrap();
    let analysis = prepare_window_analysis_with_control(&tx, &input, &control).unwrap();
    assert_eq!(old_analysis.context(), analysis.context());
    assert_eq!(tx.total_changes(), changes);
    tx.commit().unwrap();
}

#[test]
fn controlled_actual_root_visit_cancel_never_returns_a_successful_prefix() {
    let fixture = Fixture::new(true);
    let mut connection = fixture.read_only();
    let tx = connection.transaction().unwrap();
    let control = SitemapReadControl::default();
    let raw = verify_raw_with_control(&tx, &control).unwrap().unwrap();
    assert_eq!(raw.summary().root_rows, 1);
    let mut visited = 0;
    let result = raw.visit_filtered(&Default::default(), |_| {
        visited += 1;
        control.cancel();
        Ok(())
    });
    stop(result, SitemapReadControlError::Cancelled);
    assert_eq!(visited, 1);
    stop(
        raw.window(&Default::default(), None, 20),
        SitemapReadControlError::Cancelled,
    );
    assert_eq!(tx.total_changes(), 0);
}

#[test]
fn controlled_readers_stop_before_sql_even_without_metadata_schema() {
    let connection = Connection::open_in_memory().unwrap();
    let coverage = crate::project_sitemap_coverage(None, None).unwrap();
    for deadline in [false, true] {
        let control = SitemapReadControl::default();
        let expected = if deadline {
            control.expire();
            SitemapReadControlError::DeadlineExceeded
        } else {
            control.cancel();
            SitemapReadControlError::Cancelled
        };
        stop(verify_raw_with_control(&connection, &control), expected);
        stop(
            verify_source_with_control(&connection, None, &control),
            expected,
        );
        stop(
            project_read_with_control(
                &connection,
                None,
                CoverageReadContext {
                    saved_config_bytes: None,
                    saved_config_sha256: None,
                    generation: None,
                    discovery_initialized: false,
                },
                &control,
            ),
            expected,
        );
        stop(
            prepare_window_analysis_with_control(
                &connection,
                &AnalysisReadInput {
                    raw: None,
                    source: None,
                    generation: None,
                    crawl_config_sha256: None,
                    coverage: &coverage,
                },
                &control,
            ),
            expected,
        );
        stop(
            crate::sitemap_source_store::verified_report::verify_saved_report_with_control(
                &connection,
                None,
                &control,
            ),
            expected,
        );
    }
}

#[test]
fn controlled_prepared_analysis_and_raw_lookup_keep_the_shared_stop_after_prepare() {
    let fixture = Fixture::new(false);
    let mut connection = fixture.read_only();
    let tx = connection.transaction().unwrap();
    let control = SitemapReadControl::default();
    let raw = verify_raw_with_control(&tx, &control).unwrap().unwrap();
    let source = verify_source_with_control(&tx, Some(&raw), &control)
        .unwrap()
        .unwrap();
    let coverage = project_read_with_control(&tx, Some(&raw), fixture.context(), &control).unwrap();
    let prepared = prepare_window_analysis_with_control(
        &tx,
        &AnalysisReadInput {
            raw: Some(&raw),
            source: Some(&source),
            generation: Some(fixture.generation),
            crawl_config_sha256: Some(&fixture.sha),
            coverage: &coverage.coverage,
        },
        &control,
    )
    .unwrap();
    control.clone().expire();
    stop(
        crate::sitemap_membership_analysis::project_prepared_window(
            &prepared,
            Some(&raw),
            &coverage.coverage,
            &[],
            &Default::default(),
        ),
        SitemapReadControlError::DeadlineExceeded,
    );
    stop(
        raw.document_by_id(&"a".repeat(64)),
        SitemapReadControlError::DeadlineExceeded,
    );
    assert_eq!(tx.total_changes(), 0);
}
