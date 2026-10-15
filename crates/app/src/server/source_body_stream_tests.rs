//! Transport/spool tests use declared manifest data, not Core archive attestation.
use super::*;
use open_seo_spider_core::{
    SitemapArchivedBody, SitemapArchivedCapture, SitemapBodyExtent, SitemapBodyReadContext,
    SitemapBodyRepresentation,
};
fn manifest(bytes: &[u8]) -> SitemapVerifiedBodyCopy {
    let sha = format!("{:x}", Sha256::digest(bytes));
    let body = SitemapArchivedBody {
        body_id: "a".repeat(64),
        representation: SitemapBodyRepresentation::DecodedXml,
        extent: SitemapBodyExtent::Complete,
        bytes: bytes.len() as u64,
        sha256: sha,
    };
    SitemapVerifiedBodyCopy {
        context: SitemapBodyReadContext {
            generation: 1,
            config_sha256: "b".repeat(64),
            snapshot_sha256: "c".repeat(64),
        },
        capture: SitemapArchivedCapture {
            schema_version: 1,
            sequence: 1,
            measurement_id: "d".repeat(64),
            measurement_envelope_sha256: "e".repeat(64),
            staged_document_sha256: "f".repeat(64),
            capture_config_sha256: "b".repeat(64),
            capture_generation: 1,
            bodies: vec![body.clone()],
            omission: None,
        },
        body,
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn source_body_spool_ready_waits_full_verification_and_success_eof_waits_real_join() {
    let temp = tempfile::tempdir().unwrap();
    let registry = Registry::new(TaskAdmission::default());
    let data = "\u{feff}original é 💡\r\n".as_bytes().to_vec();
    let expected = data.clone();
    let (started, observed) = std::sync::mpsc::channel();
    let (release, gate) = std::sync::mpsc::channel();
    let request = registry.clone();
    let path = temp.path().to_owned();
    let mut pending = tokio::spawn(async move {
        request
            .prepare(path, move |out, _| {
                out.write_all(&data)?;
                started.send(()).unwrap();
                gate.recv().unwrap();
                Ok(manifest(&data))
            })
            .await
    });
    tokio::task::spawn_blocking(move || observed.recv_timeout(Duration::from_secs(5)).unwrap())
        .await
        .unwrap();
    assert!(
        !pending.is_finished(),
        "no successful response exists before private full verification finishes"
    );
    release.send(()).unwrap();
    let (_, body) = (&mut pending).await.unwrap().unwrap();
    let bytes = axum::body::to_bytes(body, 67_108_864).await.unwrap();
    assert_eq!(bytes.as_ref(), expected);
    registry.shutdown_and_drain().await.unwrap();
    assert_eq!(registry.stats()["spool_leases"], 0);
    assert_eq!(
        std::fs::read_dir(temp.path().join(".source-body-spool"))
            .unwrap()
            .count(),
        0
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn source_body_spool_wrong_hash_fails_before_ready_and_releases_owned_files() {
    let temp = tempfile::tempdir().unwrap();
    let registry = Registry::new(TaskAdmission::default());
    let error = match registry
        .prepare(temp.path().to_owned(), |out, _| {
            out.write_all(b"actual")?;
            Ok(manifest(b"wrong!"))
        })
        .await
    {
        Err(error) => error,
        Ok(_) => panic!("full private SHA verification must fail before response"),
    };
    assert!(error.to_string().contains("hash/EOF"));
    registry.shutdown_and_drain().await.unwrap();
    assert_eq!(registry.stats()["spool_leases"], 0);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn source_body_unpolled_response_drop_cancels_full_queue_and_actual_drain_joins() {
    let temp = tempfile::tempdir().unwrap();
    let registry = Registry::new(TaskAdmission::default());
    let bytes = vec![0x5a; 2 * 1024 * 1024];
    let (_, body) = registry
        .prepare(temp.path().to_owned(), move |out, _| {
            out.write_all(&bytes)?;
            Ok(manifest(&bytes))
        })
        .await
        .unwrap();
    drop(body);
    tokio::time::timeout(Duration::from_secs(10), registry.shutdown_and_drain())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(registry.stats()["spool_leases"], 0);
    assert_eq!(registry.stats()["admitted"], registry.stats()["joined"]);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn source_body_shutdown_emits_error_instead_of_silent_truncated_success_eof() {
    let temp = tempfile::tempdir().unwrap();
    let registry = Registry::new(TaskAdmission::default());
    let bytes = vec![0x31; 2 * 1024 * 1024];
    let (_, body) = registry
        .prepare(temp.path().to_owned(), move |out, _| {
            out.write_all(&bytes)?;
            Ok(manifest(&bytes))
        })
        .await
        .unwrap();
    registry.close_admission();
    registry.cancel_all();
    assert!(axum::body::to_bytes(body, 67_108_864).await.is_err());
    tokio::time::timeout(Duration::from_secs(10), registry.shutdown_and_drain())
        .await
        .unwrap()
        .unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn source_body_spool_worker_panic_is_joined_cleaned_and_sticky() {
    let temp = tempfile::tempdir().unwrap();
    let registry = Registry::new(TaskAdmission::default());
    assert!(
        registry
            .prepare(temp.path().to_owned(), |out, _| {
                out.write_all(b"created before panic")?;
                panic!("actual owned source body preparation panic")
            })
            .await
            .is_err()
    );
    assert!(registry.shutdown_and_drain().await.is_err());
    assert_eq!(registry.stats()["retained"], 0);
    assert_eq!(
        std::fs::read_dir(temp.path().join(".source-body-spool"))
            .unwrap()
            .count(),
        0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn source_body_private_spool_accepts_actual_64mib_and_rejects_one_extra_byte() {
    let temp = tempfile::tempdir().unwrap();
    let registry = Registry::new(TaskAdmission::default());
    let bytes = vec![0x6du8; BODY_CAP as usize];
    let expected = format!("{:x}", Sha256::digest(&bytes));
    let (_, body) = registry
        .prepare(temp.path().to_owned(), move |out, _| {
            out.write_all(&bytes)?;
            Ok(manifest(&bytes))
        })
        .await
        .unwrap();
    let received = axum::body::to_bytes(body, BODY_CAP as usize).await.unwrap();
    assert_eq!(received.len(), BODY_CAP as usize);
    assert_eq!(format!("{:x}", Sha256::digest(&received)), expected);
    drop(received);
    let error = match registry
        .prepare(temp.path().to_owned(), |out, _| {
            let chunk = [0x6du8; 65_536];
            for _ in 0..1024 {
                out.write_all(&chunk)?;
            }
            out.write_all(&[0x6d])?;
            Ok(manifest(&[]))
        })
        .await
    {
        Err(error) => error,
        Ok(_) => panic!("64MiB+1 must be rejected before a ready response"),
    };
    assert!(error.to_string().contains("exceeds64MiB"));
    registry.shutdown_and_drain().await.unwrap();
    assert_eq!(
        std::fs::read_dir(temp.path().join(".source-body-spool"))
            .unwrap()
            .count(),
        0
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn source_body_post_spool_snapshot_recheck_can_refuse_before_headers() {
    let temp = tempfile::tempdir().unwrap();
    let registry = Registry::new(TaskAdmission::default());
    let checked = Arc::new(AtomicBool::new(false));
    let actual = checked.clone();
    let result = registry
        .prepare_checked(
            temp.path().to_owned(),
            |out, _| {
                out.write_all(b"original")?;
                Ok(manifest(b"original"))
            },
            move || {
                actual.store(true, Ordering::SeqCst);
                anyhow::bail!("Stale source body snapshot")
            },
        )
        .await;
    assert!(result.is_err());
    assert!(checked.load(Ordering::SeqCst));
    registry.shutdown_and_drain().await.unwrap();
    assert_eq!(registry.stats()["spool_leases"], 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn source_body_preheader_caller_drop_keeps_actual_worker_until_cancelled_and_joined() {
    let temp = tempfile::tempdir().unwrap();
    let registry = Registry::new(TaskAdmission::default());
    let request = registry.clone();
    let path = temp.path().to_owned();
    let (started, observed) = std::sync::mpsc::channel();
    let pending = tokio::spawn(async move {
        request
            .prepare(path, move |out, checkpoint| {
                out.write_all(b"unadvertised private prefix")?;
                started.send(()).unwrap();
                loop {
                    checkpoint()?;
                    std::thread::sleep(Duration::from_millis(1));
                }
            })
            .await
    });
    tokio::task::spawn_blocking(move || observed.recv_timeout(Duration::from_secs(5)).unwrap())
        .await
        .unwrap();
    pending.abort();
    match pending.await {
        Err(e) => assert!(e.is_cancelled()),
        Ok(_) => panic!("caller should be aborted"),
    };
    tokio::time::timeout(Duration::from_secs(10), registry.shutdown_and_drain())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(registry.stats()["admitted"], registry.stats()["joined"]);
    assert_eq!(registry.stats()["spool_leases"], 0);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn source_body_spool_fifo_substitution_refuses_without_blocking_and_keeps_uncertain_lease() {
    use std::os::unix::fs::FileTypeExt;
    let temp = tempfile::tempdir().unwrap();
    let registry = Registry::new(TaskAdmission::default());
    let original = b"known retained source bytes".to_vec();
    let expected = original.clone();
    let (_, body) = registry
        .prepare(temp.path().to_owned(), move |out, _| {
            out.write_all(&original)?;
            Ok(manifest(&original))
        })
        .await
        .unwrap();
    let name = lock(&registry.0.slots).keys().next().unwrap().clone();
    let directory = temp.path().join(".source-body-spool");
    let original_path = directory.join(&name);
    let retained = directory.join("owned-original-retained");
    std::fs::rename(&original_path, &retained).unwrap();
    let status = std::process::Command::new("mkfifo")
        .arg(&original_path)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "the Unix fixture must actually create its own FIFO, not skip"
    );
    assert!(
        std::fs::symlink_metadata(&original_path)
            .unwrap()
            .file_type()
            .is_fifo()
    );
    drop(body);
    let result = tokio::time::timeout(Duration::from_secs(5), registry.shutdown_and_drain())
        .await
        .expect("nonblocking probe must finish before any FIFO writer connects");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cleanup"));
    assert_eq!(
        registry.stats()["spool_leases"],
        1,
        "uncertain replacement remains charged"
    );
    assert!(registry.stats()["first_failure"].is_string());
    assert_eq!(std::fs::read(retained).unwrap(), expected);
    assert!(
        std::fs::symlink_metadata(original_path)
            .unwrap()
            .file_type()
            .is_fifo()
    );
}
