//! Owned blocking reads. Request cancellation never detaches the actual worker.
//! Root must drain this registry before shutting down its Tokio runtime.
use crate::sitemap_findings::typed_failure;
use crate::sitemap_membership::Failure;
use futures::FutureExt;
use open_seo_spider_core::SitemapReadControl;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::{Notify, oneshot},
    task::JoinHandle,
};

const READ_TIMEOUT: Duration = Duration::from_secs(120);

struct Task {
    control: SitemapReadControl,
    join: JoinHandle<()>,
    #[cfg(test)]
    worker_completion: tokio::task::AbortHandle,
}
#[derive(Default)]
struct Tasks {
    closed: bool,
    next_id: u64,
    running: HashMap<u64, Task>,
    supervisor_failure: Option<String>,
}
#[derive(Default)]
struct Counters {
    active: AtomicUsize,
    drained: Notify,
    #[cfg(test)]
    started: AtomicUsize,
    #[cfg(test)]
    joined: AtomicUsize,
    #[cfg(test)]
    failed: AtomicUsize,
}
struct Inner {
    tasks: Mutex<Tasks>,
    counters: Counters,
    timeout: Duration,
}
impl Inner {
    fn tasks(&self) -> MutexGuard<'_, Tasks> {
        match self.tasks.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                guard
                    .supervisor_failure
                    .get_or_insert_with(|| "Read registry lock poisoned".into());
                guard
            }
        }
    }
}

#[derive(Clone)]
pub(super) struct ReadRegistry(Arc<Inner>);
impl Default for ReadRegistry {
    fn default() -> Self {
        Self(Arc::new(Inner {
            tasks: Mutex::new(Tasks::default()),
            counters: Counters::default(),
            timeout: READ_TIMEOUT,
        }))
    }
}

// The request guard is independent of the supervisor's scheduling. Dropping the
// HTTP/request future immediately cancels the same actual Core control, whose
// attached SQLite InterruptHandles are owned by the running read session.
struct CancelOnDrop {
    control: SitemapReadControl,
    armed: bool,
}
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if self.armed {
            self.control.cancel();
        }
    }
}

// Construct ONLY after awaiting the actual blocking JoinHandle. Then even a
// formatting/classification panic cannot leak active accounting. This guard is
// deliberately not created before worker join: supervisor failure may not claim
// that a still-running blocking worker has completed.
struct Joined(Arc<Inner>);
impl Drop for Joined {
    fn drop(&mut self) {
        #[cfg(test)]
        self.0.counters.joined.fetch_add(1, Ordering::SeqCst);
        self.0.counters.active.fetch_sub(1, Ordering::SeqCst);
        self.0.counters.drained.notify_waiters();
    }
}

fn reap_finished(state: &mut Tasks) {
    let finished: Vec<_> = state
        .running
        .iter()
        .filter(|(_, task)| task.join.is_finished())
        .map(|(id, _)| *id)
        .collect();
    for id in finished {
        if let Some(task) = state.running.remove(&id) {
            // Only completed coordinators are removed. is_finished guarantees
            // polling this JoinHandle is ready; no unfinished task is detached.
            match task.join.now_or_never() {
                Some(Ok(())) => {}
                Some(Err(error)) => {
                    state
                        .supervisor_failure
                        .get_or_insert_with(|| format!("Read supervisor join failed: {error}"));
                }
                None => {
                    state
                        .supervisor_failure
                        .get_or_insert_with(|| "Finished read supervisor was not joinable".into());
                }
            }
        }
    }
}

impl ReadRegistry {
    pub(super) async fn read<T: Send + 'static>(
        &self,
        operation: impl FnOnce(SitemapReadControl) -> anyhow::Result<T> + Send + 'static,
    ) -> Result<T, Failure> {
        let (receiver, control) = {
            let mut state = self.0.tasks();
            reap_finished(&mut state);
            if state.closed {
                return Err(Failure::new(
                    "sitemap_findings_cancelled",
                    "Read service is shutting down",
                ));
            }
            if let Some(error) = &state.supervisor_failure {
                return Err(Failure::new("sitemap_findings_read_failure", error));
            }
            let runtime = tokio::runtime::Handle::try_current()
                .map_err(|e| Failure::new("sitemap_findings_read_failure", e))?;
            let id = state.next_id.checked_add(1).ok_or_else(|| {
                Failure::new(
                    "sitemap_findings_read_failure",
                    "Read registry identity exhausted",
                )
            })?;
            state.next_id = id;
            let deadline = Instant::now() + self.0.timeout;
            let control = SitemapReadControl::with_deadline(deadline);
            let worker_control = control.clone();
            let (sender, receiver) = oneshot::channel();
            self.0.counters.active.fetch_add(1, Ordering::SeqCst);
            #[cfg(test)]
            self.0.counters.started.fetch_add(1, Ordering::SeqCst);
            let worker = runtime.spawn_blocking(move || operation(worker_control));
            #[cfg(test)]
            let worker_completion = worker.abort_handle();
            let inner = self.0.clone();
            let supervisor_control = control.clone();
            let join = runtime.spawn(supervise(
                worker,
                sender,
                supervisor_control,
                deadline,
                inner,
            ));
            state.running.insert(
                id,
                Task {
                    control: control.clone(),
                    join,
                    #[cfg(test)]
                    worker_completion,
                },
            );
            (receiver, control)
        };
        let mut cancellation = CancelOnDrop {
            control,
            armed: true,
        };
        let result = receiver.await.map_err(|error| {
            Failure::new(
                "sitemap_findings_read_failure",
                format!("Read supervisor lost its result: {error}"),
            )
        })?;
        cancellation.armed = false;
        result
    }

    pub(super) async fn shutdown_and_drain(&self) -> anyhow::Result<()> {
        {
            let mut state = self.0.tasks();
            state.closed = true;
            for task in state.running.values() {
                task.control.cancel();
            }
        }
        loop {
            // Register before checking the predicate; notify_waiters between a
            // plain predicate check and the first poll must not be lost.
            let notified = self.0.counters.drained.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let (empty, failure) = {
                let mut state = self.0.tasks();
                reap_finished(&mut state);
                (state.running.is_empty(), state.supervisor_failure.clone())
            };
            let active = self.0.counters.active.load(Ordering::SeqCst);
            if empty && active == 0 {
                if let Some(failure) = failure {
                    return Err(anyhow::anyhow!(failure));
                }
                return Ok(());
            }
            if active == 0 {
                // Joined drops immediately before the supervisor returns. On
                // another executor thread its JoinHandle may need one turn to
                // become finished after the final notification. Never await a
                // second notification that cannot occur, or remove that handle.
                tokio::task::yield_now().await;
            } else {
                notified.await;
            }
        }
    }

    #[cfg(test)]
    fn with_timeout(timeout: Duration) -> Self {
        Self(Arc::new(Inner {
            tasks: Mutex::new(Tasks::default()),
            counters: Counters::default(),
            timeout,
        }))
    }
    #[cfg(test)]
    pub(super) fn snapshot(&self) -> ReadStats {
        let state = self.0.tasks();
        ReadStats {
            started: self.0.counters.started.load(Ordering::SeqCst),
            active: self.0.counters.active.load(Ordering::SeqCst),
            joined: self.0.counters.joined.load(Ordering::SeqCst),
            failed: self.0.counters.failed.load(Ordering::SeqCst),
            retained: state.running.len(),
            closed: state.closed,
        }
    }
}

async fn supervise<T: Send + 'static>(
    mut worker: JoinHandle<anyhow::Result<T>>,
    mut reply: oneshot::Sender<Result<T, Failure>>,
    control: SitemapReadControl,
    deadline: Instant,
    inner: Arc<Inner>,
) {
    let joined = tokio::select! {
        biased;
        result=&mut worker=>result,
        _=reply.closed()=>{control.cancel();worker.await},
        _=tokio::time::sleep_until(tokio::time::Instant::from_std(deadline))=>{control.expire();worker.await},
    };
    // No result/cancel/deadline is delivered until the blocking worker joined.
    let _joined = Joined(inner.clone());
    // The biased worker branch can win when both futures become ready before
    // this supervisor is polled. A worker may have changed its cooperative
    // deadline, so enforce this independent immutable deadline after join too.
    if Instant::now() >= deadline {
        control.expire();
    }
    let result = match joined {
        Ok(Ok(value)) => control.checkpoint().map(|_| value).map_err(typed_failure),
        Ok(Err(error)) => Err(typed_failure(error)),
        Err(error) => Err(Failure::new(
            "sitemap_findings_read_failure",
            format!("Blocking read worker join failed: {error}"),
        )),
    };
    #[cfg(test)]
    if result.is_err() {
        inner.counters.failed.fetch_add(1, Ordering::SeqCst);
    }
    let _ = reply.send(result);
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(super) struct ReadStats {
    pub started: usize,
    pub active: usize,
    pub joined: usize,
    pub failed: usize,
    pub retained: usize,
    pub closed: bool,
}

#[cfg(test)]
#[path = "sitemap_findings_reads_tests.rs"]
mod tests;
