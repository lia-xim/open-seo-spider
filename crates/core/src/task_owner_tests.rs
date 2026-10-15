use super::*;
use std::sync::{atomic::AtomicUsize, mpsc};
use tokio::time::{Duration, timeout};
const WAIT: Duration = Duration::from_secs(5);

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disconnected_caller_and_aborted_drain_retain_actual_worker_and_supervisor() {
    let owner = TaskOwner::new("regression");
    let (entered, started) = oneshot::channel();
    let (release, held) = mpsc::channel();
    let exited = Arc::new(AtomicBool::new(false));
    let observed = exited.clone();
    let ticket = owner
        .spawn_blocking(move || {
            entered.send(()).unwrap();
            held.recv().unwrap();
            observed.store(true, Ordering::SeqCst);
            47
        })
        .unwrap();
    timeout(WAIT, started).await.unwrap().unwrap();
    drop(ticket);
    let first = owner.clone();
    let mut draining = tokio::spawn(async move { first.shutdown_and_drain().await });
    tokio::task::yield_now().await;
    assert!(
        timeout(Duration::from_millis(30), &mut draining)
            .await
            .is_err()
    );
    assert!(!exited.load(Ordering::SeqCst));
    assert_eq!(owner.snapshot().joined, 0);
    draining.abort();
    assert!(draining.await.unwrap_err().is_cancelled());
    assert_eq!(owner.snapshot().retained, 1);
    let later = owner.clone();
    let mut recovered = tokio::spawn(async move { later.shutdown_and_drain().await });
    assert!(
        timeout(Duration::from_millis(30), &mut recovered)
            .await
            .is_err()
    );
    release.send(()).unwrap();
    timeout(WAIT, recovered).await.unwrap().unwrap().unwrap();
    assert!(exited.load(Ordering::SeqCst));
    let stats = owner.snapshot();
    assert_eq!(
        (stats.admitted, stats.started, stats.joined, stats.retained),
        (1, 1, 1, 0)
    );
    owner.shutdown_and_drain().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shared_admission_closes_all_owners_before_any_new_work_can_start() {
    let admission = TaskAdmission::default();
    let request = TaskOwner::with_admission("requests", admission.clone());
    let blocking = TaskOwner::with_admission("blocking", admission.clone());
    let crawl = TaskOwner::with_admission("crawls", admission.clone());
    let starts = Arc::new(AtomicUsize::new(0));
    let mut accepted = Vec::new();
    for _ in 0..32 {
        let own = blocking.clone();
        let starts = starts.clone();
        accepted.push(tokio::spawn(async move {
            own.spawn_blocking(move || starts.fetch_add(1, Ordering::SeqCst))
        }));
    }
    admission.close();
    for result in accepted {
        if let Ok(ticket) = result.await.unwrap() {
            ticket.await.unwrap();
        }
    }
    assert!(
        request
            .spawn_async(async { panic!("late request") })
            .is_err()
    );
    assert!(crawl.spawn_async(async { panic!("late crawl") }).is_err());
    assert!(blocking.spawn_blocking(|| panic!("late worker")).is_err());
    request.shutdown_and_drain().await.unwrap();
    blocking.shutdown_and_drain().await.unwrap();
    crawl.shutdown_and_drain().await.unwrap();
    assert_eq!(
        blocking.snapshot().joined as usize,
        starts.load(Ordering::SeqCst)
    );
    assert_eq!(request.snapshot().admitted + crawl.snapshot().admitted, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn worker_panic_is_sticky_after_actual_join_and_never_becomes_clean_drain() {
    let owner = TaskOwner::new("panic");
    let ticket = owner
        .spawn_blocking(|| -> () { panic!("actual owned regression panic") })
        .unwrap();
    assert!(ticket.await.is_err());
    assert_eq!(owner.snapshot().joined, 1);
    assert_eq!(owner.snapshot().retained, 0);
    assert!(owner.shutdown_and_drain().await.is_err());
    assert!(owner.shutdown_and_drain().await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn discarded_async_ticket_still_joins_complete_task_and_nested_worker() {
    let outer = TaskOwner::new("launch");
    let inner = TaskOwner::new("pdf");
    let (entered, started) = oneshot::channel();
    let (release, held) = mpsc::channel();
    let pdf = inner.clone();
    let ticket = outer
        .spawn_async(async move {
            pdf.run_blocking(move || {
                entered.send(()).unwrap();
                held.recv().unwrap();
            })
            .await
            .unwrap();
            73
        })
        .unwrap();
    drop(ticket);
    timeout(WAIT, started).await.unwrap().unwrap();
    let draining = outer.clone();
    let mut closed = tokio::spawn(async move { draining.shutdown_and_drain().await });
    assert!(
        timeout(Duration::from_millis(30), &mut closed)
            .await
            .is_err()
    );
    release.send(()).unwrap();
    timeout(WAIT, closed).await.unwrap().unwrap().unwrap();
    inner.shutdown_and_drain().await.unwrap();
    assert_eq!(outer.snapshot().joined, 1);
    assert_eq!(inner.snapshot().joined, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shared_gate_late_nested_work_is_rejected_but_admitted_parent_still_joins() {
    let admission = TaskAdmission::default();
    let outer = TaskOwner::with_admission("request", admission.clone());
    let inner = TaskOwner::with_admission("blocking", admission.clone());
    let (entered, observed) = oneshot::channel();
    let (proceed, held) = oneshot::channel();
    let nested = inner.clone();
    let ticket = outer
        .spawn_async(async move {
            entered.send(()).unwrap();
            held.await.unwrap();
            assert!(
                nested
                    .run_blocking(|| panic!("Late operation must not start"))
                    .await
                    .is_err()
            );
        })
        .unwrap();
    observed.await.unwrap();
    admission.close();
    drop(ticket);
    let draining = outer.clone();
    let mut close = tokio::spawn(async move { draining.shutdown_and_drain().await });
    assert!(
        timeout(Duration::from_millis(30), &mut close)
            .await
            .is_err()
    );
    proceed.send(()).unwrap();
    timeout(WAIT, close).await.unwrap().unwrap().unwrap();
    inner.shutdown_and_drain().await.unwrap();
    assert_eq!(outer.snapshot().joined, 1);
    assert_eq!(inner.snapshot().admitted, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn explicit_reap_observes_discarded_completed_launch_tickets_without_growth() {
    let owner = TaskOwner::new("launch_reap");
    for _ in 0..100 {
        owner.reap_completed().await;
        let (done, observed) = oneshot::channel();
        drop(
            owner
                .spawn_async(async move {
                    done.send(()).unwrap();
                })
                .unwrap(),
        );
        observed.await.unwrap();
        // Join completion may follow the operation's own marker. Reap until
        // the actual worker and supervisor are observed, not merely that marker.
        timeout(WAIT, async {
            while owner.snapshot().retained != 0 {
                owner.reap_completed().await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(owner.snapshot().retained, 0);
    }
    owner.shutdown_and_drain().await.unwrap();
    assert_eq!(owner.snapshot().joined, 100);
}
