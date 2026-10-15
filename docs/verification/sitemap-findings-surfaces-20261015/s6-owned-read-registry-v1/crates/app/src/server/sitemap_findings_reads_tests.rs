//! AUTHORED ONLY: actual blocking workers, channels, timers and cancellation.
//! No test/compiler/SQLite/native/runtime executed under compute HOLD.
use super::*;
use open_seo_spider_core::SitemapReadControlError;
use std::sync::atomic::AtomicBool;

async fn supervisors_finished(registry: &ReadRegistry) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if registry
                .0
                .tasks()
                .running
                .values()
                .all(|task| task.join.is_finished())
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owned_read_default_success_is_delivered_after_actual_worker_drop_and_drain_joins_supervisor()
 {
    struct WorkerDropped(Arc<AtomicBool>);
    impl Drop for WorkerDropped {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let registry = ReadRegistry::default();
    let dropped = Arc::new(AtomicBool::new(false));
    let worker_dropped = dropped.clone();
    let result = registry
        .read(move |control| {
            let _guard = WorkerDropped(worker_dropped);
            control.checkpoint()?;
            Ok(42u64)
        })
        .await
        .unwrap();
    assert_eq!(result, 42);
    assert!(dropped.load(Ordering::SeqCst));
    registry.shutdown_and_drain().await.unwrap();
    let stats = registry.snapshot();
    assert_eq!(
        (
            stats.started,
            stats.active,
            stats.joined,
            stats.failed,
            stats.retained
        ),
        (1, 0, 1, 0, 0)
    );
    assert!(stats.closed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owned_read_request_drop_cancels_actual_control_and_drain_waits_until_blocking_worker_join()
{
    let registry = ReadRegistry::default();
    let reader = registry.clone();
    let (started, control) = oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let exited = Arc::new(AtomicBool::new(false));
    let worker_exited = exited.clone();
    let request = tokio::spawn(async move {
        reader
            .read(move |control| {
                assert!(started.send(control.clone()).is_ok());
                wait.recv()?;
                worker_exited.store(true, Ordering::SeqCst);
                Ok(17u8)
            })
            .await
    });
    let control = control.await.unwrap();
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    assert_eq!(
        control
            .checkpoint()
            .unwrap_err()
            .downcast_ref::<SitemapReadControlError>(),
        Some(&SitemapReadControlError::Cancelled)
    );
    assert_eq!(registry.snapshot().active, 1);
    assert!(!exited.load(Ordering::SeqCst));
    let draining = registry.clone();
    let mut drain = tokio::spawn(async move { draining.shutdown_and_drain().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut drain)
            .await
            .is_err()
    );
    assert_eq!(registry.snapshot().joined, 0);
    release.send(()).unwrap();
    drain.await.unwrap().unwrap();
    assert!(exited.load(Ordering::SeqCst));
    assert_eq!(registry.snapshot().active, 0);
    assert_eq!(registry.snapshot().joined, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owned_read_supervisor_deadline_actually_expires_control_but_never_returns_before_worker_join()
 {
    let registry = ReadRegistry::with_timeout(Duration::from_millis(20));
    let reader = registry.clone();
    let (started, control) = oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let mut request = tokio::spawn(async move {
        reader
            .read(move |control| {
                // This worker never calls checkpoint itself while blocked.
                assert!(started.send(control).is_ok());
                wait.recv()?;
                Ok(99u64)
            })
            .await
    });
    let control = control.await.unwrap();
    // Remove self-expiry to distinguish the actual asynchronous supervisor's
    // expire() call from checkpoint() merely observing its own elapsed clock.
    control.set_deadline(None);
    let expired = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Err(error) = control.checkpoint() {
                break error;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        expired.downcast_ref::<SitemapReadControlError>(),
        Some(&SitemapReadControlError::DeadlineExceeded)
    );
    assert!(!request.is_finished());
    assert_eq!(registry.snapshot().active, 1);
    assert_eq!(registry.snapshot().joined, 0);
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut request)
            .await
            .is_err()
    );
    release.send(()).unwrap();
    let failure = request.await.unwrap().unwrap_err();
    assert_eq!(failure.code, "sitemap_findings_timeout");
    registry.shutdown_and_drain().await.unwrap();
    assert_eq!(registry.snapshot().joined, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owned_read_concurrent_drain_closes_admission_and_cancelling_one_drain_does_not_detach_worker()
 {
    let registry = ReadRegistry::default();
    let reader = registry.clone();
    let (started, control) = oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let request = tokio::spawn(async move {
        reader
            .read(move |control| {
                assert!(started.send(control).is_ok());
                wait.recv()?;
                Ok(1u8)
            })
            .await
    });
    let control = control.await.unwrap();
    let first = registry.clone();
    let first = tokio::spawn(async move { first.shutdown_and_drain().await });
    let second = registry.clone();
    let mut second = tokio::spawn(async move { second.shutdown_and_drain().await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !registry.snapshot().closed {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut second)
            .await
            .is_err()
    );
    assert_eq!(registry.snapshot().retained, 1);
    assert_eq!(registry.snapshot().active, 1);
    let ran = Arc::new(AtomicBool::new(false));
    let called = ran.clone();
    let failure = registry
        .read(move |_| {
            called.store(true, Ordering::SeqCst);
            Ok(0u8)
        })
        .await
        .unwrap_err();
    assert_eq!(failure.code, "sitemap_findings_cancelled");
    assert!(!ran.load(Ordering::SeqCst));
    assert_eq!(
        control
            .checkpoint()
            .unwrap_err()
            .downcast_ref::<SitemapReadControlError>(),
        Some(&SitemapReadControlError::Cancelled)
    );
    release.send(()).unwrap();
    assert_eq!(
        request.await.unwrap().unwrap_err().code,
        "sitemap_findings_cancelled"
    );
    second.await.unwrap().unwrap();
    registry.shutdown_and_drain().await.unwrap();
    let stats = registry.snapshot();
    assert_eq!(
        (stats.started, stats.active, stats.joined, stats.retained),
        (1, 0, 1, 0)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owned_read_actual_worker_panic_is_joined_and_classified_without_leaking_active() {
    let registry = ReadRegistry::default();
    let failure = registry
        .read::<u8>(|_| panic!("actual owned blocking fixture panic"))
        .await
        .unwrap_err();
    assert_eq!(failure.code, "sitemap_findings_read_failure");
    assert!(failure.error.contains("worker join failed"));
    registry.shutdown_and_drain().await.unwrap();
    let stats = registry.snapshot();
    assert_eq!(
        (
            stats.started,
            stats.active,
            stats.joined,
            stats.failed,
            stats.retained
        ),
        (1, 0, 1, 1, 0)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owned_read_actual_operation_errors_keep_shared_typed_failure_instead_of_generic_timeout() {
    let registry = ReadRegistry::default();
    let invalid = registry
        .read::<u8>(|_| {
            Err(Failure::new(
                "invalid_sitemap_findings_query",
                "actual typed operation error",
            )
            .into())
        })
        .await
        .unwrap_err();
    assert_eq!(invalid.code, "invalid_sitemap_findings_query");
    let io = registry
        .read::<u8>(|_| {
            Err(
                std::io::Error::new(std::io::ErrorKind::PermissionDenied, "actual read IO error")
                    .into(),
            )
        })
        .await
        .unwrap_err();
    assert_eq!(io.code, "sitemap_findings_read_failure");
    registry.shutdown_and_drain().await.unwrap();
    assert_eq!(registry.snapshot().failed, 2);
    assert_eq!(registry.snapshot().joined, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owned_read_completed_supervisors_are_actually_reaped_before_later_admission() {
    let registry = ReadRegistry::default();
    for value in 0..8 {
        assert_eq!(registry.read(move |_| Ok(value)).await.unwrap(), value);
    }
    supervisors_finished(&registry).await;
    let reader = registry.clone();
    let (started, control) = oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let request = tokio::spawn(async move {
        reader
            .read(move |control| {
                assert!(started.send(control).is_ok());
                wait.recv()?;
                Ok(9u8)
            })
            .await
    });
    let _control = control.await.unwrap();
    assert_eq!(
        registry.snapshot().retained,
        1,
        "All previous completed handles were polled/reaped"
    );
    release.send(()).unwrap();
    assert_eq!(request.await.unwrap().unwrap(), 9);
    registry.shutdown_and_drain().await.unwrap();
    let stats = registry.snapshot();
    assert_eq!((stats.started, stats.joined, stats.retained), (9, 9, 0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owned_read_past_completion_notify_and_repeated_concurrent_drains_do_not_lose_wakeup() {
    for _ in 0..8 {
        let registry = ReadRegistry::default();
        assert_eq!(registry.read(|_| Ok(3u8)).await.unwrap(), 3);
        supervisors_finished(&registry).await;
        let left = registry.clone();
        let right = registry.clone();
        tokio::time::timeout(Duration::from_secs(2), async {
            let (a, b) = tokio::join!(left.shutdown_and_drain(), right.shutdown_and_drain());
            a.unwrap();
            b.unwrap();
        })
        .await
        .unwrap();
        assert_eq!(registry.snapshot().active, 0);
        assert_eq!(registry.snapshot().retained, 0);
    }
}
