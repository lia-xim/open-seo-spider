//! Verified two-pass CSV transport. AUTHORED ONLY under compute HOLD.
//! The final positive chunk cannot escape before Core TX return and worker join.
use crate::sitemap_findings::{self as findings, FindingsExportSnapshot, PreparedCsv};
use crate::sitemap_membership::{self as membership, CsvQuery, Failure, Snapshot};
use axum::body::{Body, Bytes};
use futures::FutureExt;
use open_seo_spider_core::{
    CrawlStore, SitemapReadControl, SitemapReadControlError, SitemapReadWindowError,
    SitemapVerifiedFindings, SitemapVerifiedStoredRows,
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::{self, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot, watch};
use uuid::Uuid;

const CHUNK_BYTES: usize = 65_536;
const QUEUE_CHUNKS: usize = 4;

#[derive(Clone, Copy)]
struct Limits {
    prepare_timeout: Duration,
    stream_timeout: Duration,
    stall_timeout: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            prepare_timeout: Duration::from_secs(120),
            stream_timeout: Duration::from_secs(300),
            stall_timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Clone)]
struct Stop {
    control: SitemapReadControl,
    changed: watch::Sender<bool>,
}
impl Stop {
    fn cancel(&self) {
        self.control.cancel();
        self.changed.send_replace(true);
    }
    fn expire(&self) {
        self.control.expire();
        self.changed.send_replace(true);
    }
    fn failure(&self) -> Failure {
        self.control
            .checkpoint()
            .err()
            .map(classify)
            .unwrap_or_else(|| {
                Failure::new(
                    "sitemap_export_cancelled",
                    "CSV export consumer disconnected",
                )
            })
    }
}

fn classify(error: anyhow::Error) -> Failure {
    if let Some(failure) = error.downcast_ref::<Failure>() {
        return failure.clone();
    }
    let code = if let Some(stop) = error.downcast_ref::<SitemapReadControlError>() {
        match stop {
            SitemapReadControlError::Cancelled => "sitemap_export_cancelled",
            SitemapReadControlError::DeadlineExceeded => "sitemap_export_timeout",
        }
    } else if let Some(read) = error.downcast_ref::<SitemapReadWindowError>() {
        match read {
            SitemapReadWindowError::StaleContext => "stale_sitemap_cursor",
            SitemapReadWindowError::InvalidContinuation => "invalid_sitemap_cursor",
            SitemapReadWindowError::ResponseByteBudget => "sitemap_response_byte_budget",
        }
    } else if error.to_string().contains("budget exceeded")
        || error.to_string().contains("exceeds bounded input")
    {
        "sitemap_analysis_input_budget"
    } else {
        "sitemap_read_failure"
    };
    Failure::new(code, error)
}

#[derive(Default)]
struct Counters {
    started: AtomicU64,
    active: AtomicU64,
    joined: AtomicU64,
    failed: AtomicU64,
    core_completed: AtomicU64,
    prefix_bytes: AtomicU64,
    peak_queued: AtomicUsize,
    max_chunk_bytes: AtomicUsize,
    prepared_sessions: AtomicU64,
    completed_passes: AtomicU64,
    first_pass_rows: AtomicU64,
    second_pass_rows: AtomicU64,
    drained: tokio::sync::Notify,
}
#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(super) struct ExportStats {
    pub started: u64,
    pub active: u64,
    pub joined: u64,
    pub failed: u64,
    pub core_completed: u64,
    pub prefix_bytes: u64,
    pub peak_queued: usize,
    pub max_chunk_bytes: usize,
    pub prepared_sessions: u64,
    pub completed_passes: u64,
    pub first_pass_rows: u64,
    pub second_pass_rows: u64,
}
struct Task {
    stop: Stop,
    worker: tokio::sync::Mutex<Worker>,
    supervisor: Mutex<Option<Arc<Supervisor>>>,
    supervisor_failure: Mutex<Option<String>>,
    finished: AtomicBool,
    failed: AtomicBool,
}
struct Worker {
    join: Option<tokio::task::JoinHandle<Result<Completion, Failure>>>,
    completion: Option<oneshot::Sender<Result<Completion, Failure>>>,
    terminal: Option<WorkerObservation>,
}
#[derive(Clone, Copy)]
struct WorkerObservation {
    completion_lost: bool,
}
struct Supervisor {
    join: tokio::sync::Mutex<Option<tokio::task::JoinHandle<Result<(), Failure>>>>,
}
#[derive(Default)]
struct Tasks {
    closed: bool,
    poisoned: bool,
    running: HashMap<u64, Arc<Task>>,
    first_failure: Option<String>,
}
struct RegistryInner {
    tasks: Mutex<Tasks>,
    next: AtomicU64,
    counters: Arc<Counters>,
    limits: Limits,
    #[cfg(test)]
    hooks: ExportTestHooks,
}
#[derive(Clone)]
pub(super) struct ExportRegistry(Arc<RegistryInner>);
impl Default for ExportRegistry {
    fn default() -> Self {
        Self(Arc::new(RegistryInner {
            tasks: Mutex::new(Tasks::default()),
            next: AtomicU64::new(1),
            counters: Arc::new(Counters::default()),
            limits: Limits::default(),
            #[cfg(test)]
            hooks: Default::default(),
        }))
    }
}
fn slot_lock<T>(slot: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // These slots only protect handle publication/diagnostics, never source facts.
    slot.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn mark_failed(task: &Task, counters: &Counters) {
    if !task.failed.swap(true, Ordering::SeqCst) {
        counters.failed.fetch_add(1, Ordering::SeqCst);
    }
}
fn finish_active(task: &Task, counters: &Counters) {
    // Call only after the actual worker has been joined. Normal supervision also
    // retains the existing BodyDone/stopped condition before calling this.
    if !task.finished.swap(true, Ordering::SeqCst) {
        counters.active.fetch_sub(1, Ordering::SeqCst);
        counters.drained.notify_waiters();
    }
}
fn record_supervisor_error(task: &Task, counters: &Counters, tasks: &mut Tasks, message: String) {
    task.stop.cancel();
    mark_failed(task, counters);
    slot_lock(&task.supervisor_failure).get_or_insert_with(|| message.clone());
    tasks.first_failure.get_or_insert(message);
}
async fn observe_worker(
    task: &Task,
    counters: &Counters,
    forced_failure: Option<Failure>,
) -> Result<WorkerObservation, Failure> {
    let mut worker = task.worker.lock().await;
    if let Some(observed) = worker.terminal {
        return Ok(observed);
    }
    let handle = worker.join.as_mut().ok_or_else(|| {
        Failure::new(
            "sitemap_read_failure",
            "CSV worker has neither retained join nor observation",
        )
    })?;
    // Await by reference: cancellation of supervisor/drain releases the mutex,
    // but the actual blocking handle remains in the shared record.
    let result = handle.await;
    worker.join.take();
    counters.joined.fetch_add(1, Ordering::SeqCst);
    let mut result = result.unwrap_or_else(|error| {
        Err(Failure::new(
            "sitemap_read_failure",
            format!("CSV blocking producer failed: {error}"),
        ))
    });
    if result.is_ok()
        && let Some(failure) = forced_failure
    {
        result = Err(failure);
    }
    if result.is_err() {
        mark_failed(task, counters);
    }
    let completion_lost = worker
        .completion
        .take()
        .is_none_or(|sender| sender.send(result).is_err());
    let observed = WorkerObservation { completion_lost };
    worker.terminal = Some(observed);
    Ok(observed)
}
impl ExportRegistry {
    fn lock_tasks(&self) -> std::sync::MutexGuard<'_, Tasks> {
        match self.0.tasks.lock() {
            Ok(state) => state,
            Err(poisoned) => {
                // Ownership must remain drainable even after a bookkeeping
                // panic. Close admission and retain an explicit sticky error.
                let mut state = poisoned.into_inner();
                state.closed = true;
                state.poisoned = true;
                state
                    .first_failure
                    .get_or_insert_with(|| "CSV registry lock poisoned".into());
                state
            }
        }
    }
    #[cfg(test)]
    pub(super) fn snapshot(&self) -> ExportStats {
        let s = &self.0.counters;
        ExportStats {
            started: s.started.load(Ordering::SeqCst),
            active: s.active.load(Ordering::SeqCst),
            joined: s.joined.load(Ordering::SeqCst),
            failed: s.failed.load(Ordering::SeqCst),
            core_completed: s.core_completed.load(Ordering::SeqCst),
            prefix_bytes: s.prefix_bytes.load(Ordering::SeqCst),
            peak_queued: s.peak_queued.load(Ordering::SeqCst),
            max_chunk_bytes: s.max_chunk_bytes.load(Ordering::SeqCst),
            prepared_sessions: s.prepared_sessions.load(Ordering::SeqCst),
            completed_passes: s.completed_passes.load(Ordering::SeqCst),
            first_pass_rows: s.first_pass_rows.load(Ordering::SeqCst),
            second_pass_rows: s.second_pass_rows.load(Ordering::SeqCst),
        }
    }
    pub(super) async fn shutdown_and_drain(&self) -> anyhow::Result<()> {
        // Snapshot shared ownership under admission lock. A canceled/concurrent
        // drain can never take or detach either actual handle.
        let tasks = {
            let mut state = self.lock_tasks();
            state.closed = true;
            for task in state.running.values() {
                task.stop.cancel();
            }
            state
                .running
                .iter()
                .map(|(id, task)| (*id, task.clone()))
                .collect::<Vec<_>>()
        };
        let mut failures = Vec::new();
        for (id, task) in tasks {
            if let Err(error) = self.finish_task(id, &task).await {
                failures.push(error.to_string());
            }
        }
        let state = self.lock_tasks();
        if let Some(message) = &state.first_failure {
            failures.push(message.clone());
        }
        if !state.running.is_empty() {
            failures.push("CSV drain retains an unobserved task".into());
        }
        if self.0.counters.active.load(Ordering::SeqCst) != 0
            || self.0.counters.joined.load(Ordering::SeqCst)
                != self.0.counters.started.load(Ordering::SeqCst)
        {
            failures.push("CSV drain worker accounting is incomplete".into());
        }
        anyhow::ensure!(
            failures.is_empty(),
            "CSV supervisor drain failed: {}",
            failures.join("; ")
        );
        Ok(())
    }
    async fn finish_task(&self, id: u64, task: &Arc<Task>) -> anyhow::Result<()> {
        let supervisor = slot_lock(&task.supervisor).clone();
        if let Some(supervisor) = &supervisor {
            let mut supervisor_join = supervisor.join.lock().await;
            if let Some(handle) = supervisor_join.as_mut() {
                let result = handle.await;
                supervisor_join.take();
                let message = match result {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(format!("CSV supervisor failed: {error}")),
                    Err(error) => Some(format!("CSV supervisor failed to join: {error}")),
                };
                if let Some(message) = message {
                    let mut state = self.lock_tasks();
                    record_supervisor_error(task, &self.0.counters, &mut state, message);
                }
            }
        } else {
            let mut state = self.lock_tasks();
            record_supervisor_error(
                task,
                &self.0.counters,
                &mut state,
                "CSV registered task has no retained supervisor slot".into(),
            );
        }
        // Do not hold the supervisor slot while waiting for a worker. Both
        // shared slots remain recoverable if this drain is canceled here.
        let forced_failure = slot_lock(&task.supervisor_failure)
            .as_ref()
            .map(|message| Failure::new("sitemap_read_failure", message));
        observe_worker(task, &self.0.counters, forced_failure).await?;
        finish_active(task, &self.0.counters);
        anyhow::ensure!(
            supervisor.is_some(),
            "CSV supervisor observation is unavailable; worker joined but task retained"
        );
        // Both actual handles have been observed, including the aborted case.
        self.lock_tasks().running.remove(&id);
        Ok(())
    }
    pub(super) fn response_build_fault(&self) -> bool {
        #[cfg(test)]
        {
            matches!(self.0.hooks.fault, ExportTestFault::ResponseBuildAfterReady)
        }
        #[cfg(not(test))]
        {
            false
        }
    }
    pub(super) fn start(
        &self,
        path: PathBuf,
        job_id: Uuid,
        generation: u64,
        config: String,
        query: CsvQuery,
    ) -> Result<PendingExport, Failure> {
        let expected = query.expected_context(job_id)?;
        self.start_with(move |ports| {
            produce(
                &ports.registry,
                path,
                job_id,
                generation,
                config,
                query,
                expected,
                ports.stop,
                ports.sender,
                ports.progress,
                ports.ready,
                ports.runtime,
            )
        })
    }

    pub(super) fn start_findings(
        &self,
        path: PathBuf,
        query: PreparedCsv,
    ) -> Result<PendingExport<FindingsExportSnapshot>, Failure> {
        self.start_with(move |ports| {
            produce_findings(
                &ports.registry,
                path,
                query,
                ports.stop,
                ports.sender,
                ports.progress,
                ports.ready,
                ports.runtime,
            )
        })
    }

    fn start_with<C: Send + 'static>(
        &self,
        producer: impl FnOnce(ProducerPorts<C>) -> Result<Completion, Failure> + Send + 'static,
    ) -> Result<PendingExport<C>, Failure> {
        let mut state = self.lock_tasks();
        if state.poisoned {
            return Err(Failure::new(
                "sitemap_read_failure",
                "CSV registry lock poisoned",
            ));
        }
        if state.closed {
            return Err(Failure::new(
                "sitemap_export_cancelled",
                "CSV export service is shutting down",
            ));
        }
        // Observe already-finished supervisor handles without taking pending
        // handles. Normal completed jobs remain bounded between starts; failed
        // supervisors with an unobserved worker stay owned for shutdown recovery.
        let completed: Vec<_> = state
            .running
            .iter()
            .filter_map(|(id, task)| {
                let supervisor = slot_lock(&task.supervisor).clone()?;
                let mut joined = supervisor.join.try_lock().ok()?;
                let handle = joined.as_mut()?;
                if !handle.is_finished() {
                    return None;
                }
                let result = (&mut *handle).now_or_never()?;
                joined.take();
                Some((*id, task.clone(), result))
            })
            .collect();
        for (id, task, result) in completed {
            let message = match result {
                Ok(Ok(())) => None,
                Ok(Err(error)) => Some(format!("CSV supervisor failed: {error}")),
                Err(error) => Some(format!("CSV supervisor failed to join: {error}")),
            };
            if let Some(message) = message {
                record_supervisor_error(&task, &self.0.counters, &mut state, message);
            }
            if task.finished.load(Ordering::SeqCst) {
                state.running.remove(&id);
            }
        }
        let started_at = Instant::now();
        let control = SitemapReadControl::with_deadline(started_at + self.0.limits.prepare_timeout);
        let (changed, _) = watch::channel(false);
        let stop = Stop { control, changed };
        let (progress, updates) = watch::channel(Progress {
            ready_at: None,
            last_sent: started_at,
        });
        let (sender, receiver) = mpsc::channel(QUEUE_CHUNKS);
        let (ready, ready_result) = oneshot::channel();
        let (completion, completed_result) = oneshot::channel();
        let (body_done, body_done_result) = watch::channel(false);
        let worker_stop = stop.clone();
        let body_progress = progress.clone();
        let registry = self.clone();
        let runtime = tokio::runtime::Handle::current();
        self.0.counters.started.fetch_add(1, Ordering::SeqCst);
        self.0.counters.active.fetch_add(1, Ordering::SeqCst);
        let (start, started) = oneshot::channel::<()>();
        let worker = tokio::task::spawn_blocking(move || {
            started.blocking_recv().map_err(|_| {
                Failure::new("sitemap_read_failure", "CSV start admission canceled")
            })?;
            producer(ProducerPorts {
                registry,
                stop: worker_stop,
                sender,
                progress,
                ready,
                runtime,
            })
        });
        let counters = self.0.counters.clone();
        let task = Arc::new(Task {
            stop: stop.clone(),
            worker: tokio::sync::Mutex::new(Worker {
                join: Some(worker),
                completion: Some(completion),
                terminal: None,
            }),
            supervisor: Mutex::new(None),
            supervisor_failure: Mutex::new(None),
            finished: AtomicBool::new(false),
            failed: AtomicBool::new(false),
        });
        let supervised = task.clone();
        let limits = self.0.limits;
        let join = tokio::spawn(async move {
            supervise(
                supervised,
                updates,
                body_done_result,
                counters,
                limits,
                started_at,
            )
            .await
        });
        *slot_lock(&task.supervisor) = Some(Arc::new(Supervisor {
            join: tokio::sync::Mutex::new(Some(join)),
        }));
        let id = self.0.next.fetch_add(1, Ordering::SeqCst);
        state.running.insert(id, task);
        // All handles are now reachable under the same admission/closing lock.
        let _ = start.send(());
        Ok(PendingExport {
            ready: ready_result,
            body: Some(BodyState {
                receiver,
                completion: Some(completed_result),
                joined: None,
                prefix_closed: false,
                delivered: 0,
                expected: None,
                stop_rx: stop.changed.subscribe(),
                stop,
                body_done,
                progress: body_progress,
                finished: false,
            }),
        })
    }
}

struct ProducerPorts<C> {
    registry: ExportRegistry,
    stop: Stop,
    sender: mpsc::Sender<Bytes>,
    progress: watch::Sender<Progress>,
    ready: oneshot::Sender<Result<Ready<C>, Failure>>,
    runtime: tokio::runtime::Handle,
}

#[derive(Clone, Copy)]
struct Progress {
    ready_at: Option<Instant>,
    last_sent: Instant,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Receipt {
    length: u64,
    sha256: String,
    rows: u64,
}
#[derive(Debug, Clone)]
pub(super) struct Ready<C = Snapshot> {
    pub snapshot: C,
    pub length: u64,
    pub sha256: String,
    receipt: Receipt,
}
struct Completion {
    receipt: Receipt,
    tail: Bytes,
}
pub(super) struct PendingExport<C = Snapshot> {
    ready: oneshot::Receiver<Result<Ready<C>, Failure>>,
    body: Option<BodyState>,
}
impl<C> PendingExport<C> {
    pub(super) async fn ready(&mut self) -> Result<Ready<C>, Failure> {
        let body = self
            .body
            .as_mut()
            .ok_or_else(|| Failure::new("sitemap_read_failure", "CSV body already consumed"))?;
        if *body.stop_rx.borrow() {
            return Err(body.stop.failure());
        }
        let ready = tokio::select! {
            biased;
            _ = body.stop_rx.changed() => return Err(body.stop.failure()),
            result = &mut self.ready => result.map_err(|_| Failure::new("sitemap_read_failure", "CSV producer ended before preparation"))??,
        };
        body.stop.control.checkpoint().map_err(classify)?;
        body.expected = Some(ready.receipt.clone());
        Ok(ready)
    }
    pub(super) fn into_body(mut self) -> Result<Body, Failure> {
        let state = self
            .body
            .take()
            .ok_or_else(|| Failure::new("sitemap_read_failure", "CSV body already consumed"))?;
        if state.expected.is_none() {
            return Err(Failure::new(
                "sitemap_read_failure",
                "CSV body was not prepared",
            ));
        }
        let stream = futures::stream::unfold(state, |mut state| async move {
            state.next().await.map(|item| (item, state))
        });
        Ok(Body::from_stream(stream))
    }
}

async fn supervise(
    task: Arc<Task>,
    mut progress: watch::Receiver<Progress>,
    mut body_done: watch::Receiver<bool>,
    counters: Arc<Counters>,
    limits: Limits,
    started_at: Instant,
) -> Result<(), Failure> {
    let stop = task.stop.clone();
    let mut stop_rx = stop.changed.subscribe();
    let mut joined = false;
    let mut stopped = *stop_rx.borrow();
    let mut progress_open = true;
    loop {
        let body_finished = *body_done.borrow();
        if joined && (body_finished || stopped) {
            break;
        }
        let current = *progress.borrow();
        let deadline = match current.ready_at {
            Some(ready) => {
                (ready + limits.stream_timeout).min(current.last_sent + limits.stall_timeout)
            }
            None => started_at + limits.prepare_timeout,
        };
        tokio::select! {
            biased;
            observed = observe_worker(&task, &counters, None), if !joined => {
                let observed = observed?;
                joined = true;
                if observed.completion_lost { stop.cancel(); stopped = true; }
            }
            change = body_done.changed(), if !body_finished => {
                if change.is_err() || *body_done.borrow() {
                    if !joined { stop.cancel(); stopped = true; }
                    if joined { break; }
                }
            }
            _ = stop_rx.changed(), if !stopped => { stopped = true; }
            change = progress.changed(), if progress_open => { if change.is_err() { progress_open = false; } }
            _ = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)), if !stopped => { stop.expire(); stopped = true; }
        }
    }
    // Every path leaves only after the actual blocking worker has been joined.
    finish_active(&task, &counters);
    Ok(())
}

async fn receive_completion(
    receiver: &mut Option<oneshot::Receiver<Result<Completion, Failure>>>,
) -> Result<Result<Completion, Failure>, oneshot::error::RecvError> {
    match receiver.as_mut() {
        Some(receiver) => receiver.await,
        None => std::future::pending().await,
    }
}

struct BodyState {
    receiver: mpsc::Receiver<Bytes>,
    completion: Option<oneshot::Receiver<Result<Completion, Failure>>>,
    joined: Option<Completion>,
    prefix_closed: bool,
    delivered: u64,
    expected: Option<Receipt>,
    stop_rx: watch::Receiver<bool>,
    stop: Stop,
    body_done: watch::Sender<bool>,
    progress: watch::Sender<Progress>,
    finished: bool,
}
impl BodyState {
    fn fail(&mut self, failure: Failure) -> Option<Result<Bytes, io::Error>> {
        self.receiver.close();
        while self.receiver.try_recv().is_ok() {}
        self.stop.cancel();
        self.finished = true;
        self.body_done.send_replace(true);
        Some(Err(io::Error::other(failure)))
    }
    async fn next(&mut self) -> Option<Result<Bytes, io::Error>> {
        loop {
            if self.finished {
                return None;
            }
            let stopped = *self.stop_rx.borrow();
            if stopped {
                return self.fail(self.stop.failure());
            }
            if self.prefix_closed
                && let Some(done) = self.joined.take()
            {
                let valid = self.expected.as_ref().is_some_and(|expected| {
                    expected == &done.receipt
                        && !done.tail.is_empty()
                        && self.delivered.checked_add(done.tail.len() as u64)
                            == Some(expected.length)
                });
                if !valid {
                    return self.fail(Failure::new(
                        "sitemap_read_failure",
                        "CSV joined completion receipt differs",
                    ));
                }
                self.finished = true;
                self.body_done.send_replace(true);
                return Some(Ok(done.tail));
            }
            tokio::select! {
                biased;
                _ = self.stop_rx.changed() => return self.fail(self.stop.failure()),
                result = receive_completion(&mut self.completion) => {
                    self.completion = None;
                    match result {
                        Ok(Ok(done)) => self.joined = Some(done),
                        Ok(Err(failure)) => return self.fail(failure),
                        Err(_) => return self.fail(Failure::new("sitemap_read_failure", "CSV supervisor lost its completion")),
                    }
                }
                prefix = self.receiver.recv(), if !self.prefix_closed => {
                    match prefix {
                        Some(bytes) => {
                            let count = self.delivered.checked_add(bytes.len() as u64);
                            if bytes.len() > CHUNK_BYTES || count.is_none_or(|count| self.expected.as_ref().is_none_or(|expected| count >= expected.length)) {
                                return self.fail(Failure::new("sitemap_read_failure", "CSV prefix consumed its terminal byte"));
                            }
                            self.delivered = count.unwrap_or(self.delivered);
                            // A joined producer can leave queued prefix chunks.
                            // Real consumer progress must keep the idle timer
                            // alive while the absolute stream deadline stays fixed.
                            self.progress.send_modify(|progress| progress.last_sent = Instant::now());
                            return Some(Ok(bytes));
                        }
                        None => self.prefix_closed = true,
                    }
                }
            }
        }
    }
}
impl Drop for BodyState {
    fn drop(&mut self) {
        self.receiver.close();
        while self.receiver.try_recv().is_ok() {}
        if !self.finished {
            self.stop.cancel();
        }
        self.body_done.send_replace(true);
    }
}

struct CountHash {
    length: u64,
    hash: Sha256,
    rows: u64,
    failure: Option<Failure>,
}
impl CountHash {
    fn new() -> Self {
        Self {
            length: 0,
            hash: Sha256::new(),
            rows: 0,
            failure: None,
        }
    }
    fn receipt(&self) -> Receipt {
        Receipt {
            length: self.length,
            sha256: format!("{:x}", self.hash.clone().finalize()),
            rows: self.rows,
        }
    }
    fn check(&mut self, length: usize) -> io::Result<u64> {
        let next = self
            .length
            .checked_add(length as u64)
            .filter(|length| *length <= membership::MAX_CSV_BYTES as u64);
        match next {
            Some(next) => Ok(next),
            None => {
                let error = Failure::new("sitemap_csv_byte_budget", "CSV exceeds 64 MiB");
                self.failure = Some(error.clone());
                Err(io::Error::other(error))
            }
        }
    }
}
impl Write for CountHash {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self.check(bytes.len())?;
        self.hash.update(bytes);
        self.length = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
struct ChunkTailWriter {
    count: CountHash,
    tail: Vec<u8>,
    sender: mpsc::Sender<Bytes>,
    stop: Stop,
    runtime: tokio::runtime::Handle,
    progress: watch::Sender<Progress>,
    counters: Arc<Counters>,
    limits: Limits,
    #[cfg(test)]
    fault: ExportTestFault,
}
impl ChunkTailWriter {
    fn failure(&mut self, failure: Failure) -> io::Error {
        self.count.failure = Some(failure.clone());
        io::Error::other(failure)
    }
    fn send_tail(&mut self) -> io::Result<()> {
        self.stop
            .control
            .checkpoint()
            .map_err(|error| self.failure(classify(error)))?;
        let mut stopped = self.stop.changed.subscribe();
        let sender = self.sender.clone();
        let stop = self.stop.clone();
        let timeout = self.limits.stall_timeout;
        let bytes = Bytes::copy_from_slice(&self.tail);
        let length = bytes.len();
        let result = self.runtime.block_on(async {
            if *stopped.borrow() { return Err(stop.failure()); }
            tokio::select! {
                biased;
                _ = stopped.changed() => Err(stop.failure()),
                permit = sender.reserve() => {
                    let permit = permit.map_err(|_| Failure::new("sitemap_export_cancelled", "CSV consumer disconnected"))?;
                    stop.control.checkpoint().map_err(classify)?;
                    permit.send(bytes);
                    Ok(())
                }
                _ = tokio::time::sleep(timeout) => { stop.expire(); Err(stop.failure()) }
            }
        });
        result.map_err(|error| self.failure(error))?;
        self.counters.peak_queued.fetch_max(
            self.sender.max_capacity() - self.sender.capacity(),
            Ordering::SeqCst,
        );
        self.counters
            .max_chunk_bytes
            .fetch_max(length, Ordering::SeqCst);
        self.counters
            .prefix_bytes
            .fetch_add(length as u64, Ordering::SeqCst);
        self.progress
            .send_modify(|progress| progress.last_sent = Instant::now());
        self.tail.clear();
        Ok(())
    }
}
impl Write for ChunkTailWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.stop
            .control
            .checkpoint()
            .map_err(|error| self.failure(classify(error)))?;
        let next = self.count.check(bytes.len())?;
        #[cfg(test)]
        if let ExportTestFault::SecondPassAfterBytes(limit) = self.fault
            && next > limit
        {
            return Err(self.failure(Failure::new(
                "sitemap_read_failure",
                "Injected second-pass sink failure",
            )));
        }
        let mut remaining = bytes;
        while !remaining.is_empty() {
            if self.tail.len() == CHUNK_BYTES {
                self.send_tail()?;
            }
            let take = remaining.len().min(CHUNK_BYTES - self.tail.len());
            self.tail.extend_from_slice(&remaining[..take]);
            remaining = &remaining[take..];
        }
        self.count.hash.update(bytes);
        self.count.length = next;
        Ok(bytes.len())
    }
    // A flush must never release the last positive chunk.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn format_pass(
    session: &SitemapVerifiedStoredRows<'_>,
    writer: &mut impl Write,
    on_row: impl FnMut(),
) -> anyhow::Result<u64> {
    membership::write_csv_header(writer)?;
    let mut rows = 0u64;
    let mut on_row = on_row;
    session.visit_all_filtered(|row| {
        membership::write_csv_row(writer, row)?;
        rows = rows
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("CSV row count overflow"))?;
        on_row();
        Ok(())
    })?;
    Ok(rows)
}

trait CsvFormatter {
    fn pass<W: Write>(&self, writer: &mut W, on_row: &mut dyn FnMut()) -> anyhow::Result<u64>;
}
struct RawFormatter<'s, 'read>(&'s SitemapVerifiedStoredRows<'read>);
impl CsvFormatter for RawFormatter<'_, '_> {
    fn pass<W: Write>(&self, writer: &mut W, on_row: &mut dyn FnMut()) -> anyhow::Result<u64> {
        format_pass(self.0, writer, on_row)
    }
}
struct FindingsFormatter<'s, 'read>(&'s SitemapVerifiedFindings<'read>);
impl CsvFormatter for FindingsFormatter<'_, '_> {
    fn pass<W: Write>(&self, writer: &mut W, on_row: &mut dyn FnMut()) -> anyhow::Result<u64> {
        findings::write_findings_csv_session_observed(self.0, writer, on_row)
    }
}

#[allow(clippy::too_many_arguments)]
fn produce_session<C>(
    registry: &ExportRegistry,
    stop: &Stop,
    sender: mpsc::Sender<Bytes>,
    progress: watch::Sender<Progress>,
    ready: &mut Option<oneshot::Sender<Result<Ready<C>, Failure>>>,
    runtime: tokio::runtime::Handle,
    snapshot: C,
    formatter: impl CsvFormatter,
) -> anyhow::Result<Completion> {
    let mut count = CountHash::new();
    let first = formatter.pass(&mut count, &mut || {
        registry
            .0
            .counters
            .first_pass_rows
            .fetch_add(1, Ordering::SeqCst);
    });
    count.rows = match first {
        Ok(rows) => rows,
        Err(error) => {
            return Err(count
                .failure
                .take()
                .map(anyhow::Error::new)
                .unwrap_or(error));
        }
    };
    registry
        .0
        .counters
        .completed_passes
        .fetch_add(1, Ordering::SeqCst);
    let receipt = count.receipt();
    stop.control.checkpoint()?;
    #[cfg(test)]
    wait_gate(registry.0.hooks.before_ready.as_ref(), &stop.control)?;
    let now = Instant::now();
    stop.control
        .set_deadline(Some(now + registry.0.limits.stream_timeout));
    progress.send_replace(Progress {
        ready_at: Some(now),
        last_sent: now,
    });
    let metadata = Ready {
        snapshot,
        length: receipt.length,
        sha256: receipt.sha256.clone(),
        receipt: receipt.clone(),
    };
    ready
        .take()
        .ok_or_else(|| anyhow::anyhow!("CSV preparation sent twice"))?
        .send(Ok(metadata))
        .map_err(|_| {
            stop.cancel();
            anyhow::Error::new(stop.failure())
        })?;
    #[cfg(test)]
    wait_gate(registry.0.hooks.after_preflight.as_ref(), &stop.control)?;
    let mut writer = ChunkTailWriter {
        count: CountHash::new(),
        tail: Vec::with_capacity(CHUNK_BYTES),
        sender,
        stop: stop.clone(),
        runtime,
        progress,
        counters: registry.0.counters.clone(),
        limits: registry.0.limits,
        #[cfg(test)]
        fault: registry.0.hooks.fault,
    };
    let pass = formatter.pass(&mut writer, &mut || {
        registry
            .0
            .counters
            .second_pass_rows
            .fetch_add(1, Ordering::SeqCst);
    });
    writer.count.rows = match pass {
        Ok(rows) => rows,
        Err(error) => {
            return Err(writer
                .count
                .failure
                .take()
                .map(anyhow::Error::new)
                .unwrap_or(error));
        }
    };
    registry
        .0
        .counters
        .completed_passes
        .fetch_add(1, Ordering::SeqCst);
    let actual = writer.count.receipt();
    #[cfg(test)]
    if matches!(
        registry.0.hooks.fault,
        ExportTestFault::SecondPassDigestMismatch
    ) {
        anyhow::bail!("Injected second-pass digest mismatch");
    }
    anyhow::ensure!(
        actual == receipt && !writer.tail.is_empty(),
        "CSV second-pass byte/hash/count differs"
    );
    stop.control.checkpoint()?;
    #[cfg(test)]
    {
        wait_gate(registry.0.hooks.before_core_return.as_ref(), &stop.control)?;
        if matches!(registry.0.hooks.fault, ExportTestFault::BeforeCoreReturn) {
            anyhow::bail!("Injected failure before Core transaction return");
        }
    }
    Ok(Completion {
        receipt: actual,
        tail: Bytes::from(writer.tail),
    })
}

fn after_core(
    registry: &ExportRegistry,
    stop: &Stop,
    completion: Completion,
) -> anyhow::Result<Completion> {
    registry
        .0
        .counters
        .core_completed
        .fetch_add(1, Ordering::SeqCst);
    #[cfg(test)]
    {
        wait_gate(registry.0.hooks.after_core_return.as_ref(), &stop.control)?;
        if matches!(
            registry.0.hooks.fault,
            ExportTestFault::AfterCoreReturnPanic
        ) {
            panic!("Injected producer panic after actual Core transaction return");
        }
    }
    stop.control.checkpoint()?;
    Ok(completion)
}

#[allow(clippy::too_many_arguments)]
fn produce(
    registry: &ExportRegistry,
    path: PathBuf,
    job_id: Uuid,
    generation: u64,
    config: String,
    query: CsvQuery,
    expected: open_seo_spider_core::SitemapReadWindowExpectedContext,
    stop: Stop,
    sender: mpsc::Sender<Bytes>,
    progress: watch::Sender<Progress>,
    ready: oneshot::Sender<Result<Ready, Failure>>,
    runtime: tokio::runtime::Handle,
) -> Result<Completion, Failure> {
    let mut ready = Some(ready);
    let result = (|| -> anyhow::Result<Completion> {
        stop.control.checkpoint()?;
        let store = CrawlStore::open_read_only(path)?;
        let filter = query.stored_filter();
        let completion = store.with_verified_sitemap_stored_rows(
            &filter,
            Some(&expected),
            &stop.control,
            |session| {
                registry
                    .0
                    .counters
                    .prepared_sessions
                    .fetch_add(1, Ordering::SeqCst);
                let snapshot =
                    query.bind_verified_header(job_id, generation, &config, session.header())?;
                produce_session(
                    registry,
                    &stop,
                    sender,
                    progress,
                    &mut ready,
                    runtime,
                    snapshot,
                    RawFormatter(session),
                )
            },
        )?;
        after_core(registry, &stop, completion)
    })()
    .map_err(classify);
    if let Err(failure) = &result
        && let Some(ready) = ready.take()
    {
        let _ = ready.send(Err(failure.clone()));
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn produce_findings(
    registry: &ExportRegistry,
    path: PathBuf,
    query: PreparedCsv,
    stop: Stop,
    sender: mpsc::Sender<Bytes>,
    progress: watch::Sender<Progress>,
    ready: oneshot::Sender<Result<Ready<FindingsExportSnapshot>, Failure>>,
    runtime: tokio::runtime::Handle,
) -> Result<Completion, Failure> {
    let mut ready = Some(ready);
    let result = (|| -> anyhow::Result<Completion> {
        stop.control.checkpoint()?;
        let store = CrawlStore::open_read_only(path)?;
        let completion = store.with_verified_sitemap_findings(
            query.selection(),
            Some(query.expected_context_sha256()),
            &stop.control,
            |session| {
                registry
                    .0
                    .counters
                    .prepared_sessions
                    .fetch_add(1, Ordering::SeqCst);
                let snapshot = query.bind_session(session)?;
                produce_session(
                    registry,
                    &stop,
                    sender,
                    progress,
                    &mut ready,
                    runtime,
                    snapshot,
                    FindingsFormatter(session),
                )
            },
        )?;
        after_core(registry, &stop, completion)
    })()
    .map_err(findings::typed_failure);
    if let Err(failure) = &result
        && let Some(ready) = ready.take()
    {
        let _ = ready.send(Err(failure.clone()));
    }
    result
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(super) struct ExportTestLimits {
    pub prepare_timeout: Duration,
    pub stream_timeout: Duration,
    pub stall_timeout: Duration,
}
#[cfg(test)]
impl Default for ExportTestLimits {
    fn default() -> Self {
        let limits = Limits::default();
        Self {
            prepare_timeout: limits.prepare_timeout,
            stream_timeout: limits.stream_timeout,
            stall_timeout: limits.stall_timeout,
        }
    }
}
#[cfg(test)]
#[derive(Clone, Copy, Default)]
pub(super) enum ExportTestFault {
    #[default]
    None,
    SecondPassAfterBytes(u64),
    BeforeCoreReturn,
    AfterCoreReturnPanic,
    SecondPassDigestMismatch,
    ResponseBuildAfterReady,
}
#[cfg(test)]
#[derive(Clone, Default)]
pub(super) struct ExportTestHooks {
    pub before_ready: Option<Arc<ExportTestGate>>,
    pub after_preflight: Option<Arc<ExportTestGate>>,
    pub before_core_return: Option<Arc<ExportTestGate>>,
    pub after_core_return: Option<Arc<ExportTestGate>>,
    pub fault: ExportTestFault,
}
#[cfg(test)]
#[derive(Default)]
pub(super) struct ExportTestGate {
    reached: std::sync::atomic::AtomicBool,
    released: Mutex<bool>,
    changed: std::sync::Condvar,
    notify: tokio::sync::Notify,
}
#[cfg(test)]
impl ExportTestGate {
    pub(super) fn is_reached(&self) -> bool {
        self.reached.load(Ordering::SeqCst)
    }
    pub(super) async fn wait_reached(&self) {
        loop {
            let notified = self.notify.notified();
            if self.is_reached() {
                return;
            }
            notified.await;
        }
    }
    pub(super) fn release(&self) {
        if let Ok(mut released) = self.released.lock() {
            *released = true;
            self.changed.notify_all();
        }
    }
    fn wait(&self, control: &SitemapReadControl) -> anyhow::Result<()> {
        self.reached.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
        let mut released = self
            .released
            .lock()
            .map_err(|_| anyhow::anyhow!("CSV test gate poisoned"))?;
        while !*released {
            control.checkpoint()?;
            released = self
                .changed
                .wait_timeout(released, Duration::from_millis(10))
                .map_err(|_| anyhow::anyhow!("CSV test gate poisoned"))?
                .0;
        }
        control.checkpoint()
    }
}
#[cfg(test)]
fn wait_gate(
    gate: Option<&Arc<ExportTestGate>>,
    control: &SitemapReadControl,
) -> anyhow::Result<()> {
    if let Some(gate) = gate {
        gate.wait(control)?;
    }
    Ok(())
}
#[cfg(test)]
impl ExportRegistry {
    pub(super) fn for_test(limits: ExportTestLimits, hooks: ExportTestHooks) -> Self {
        Self(Arc::new(RegistryInner {
            tasks: Mutex::new(Tasks::default()),
            next: AtomicU64::new(1),
            counters: Arc::new(Counters::default()),
            limits: Limits {
                prepare_timeout: limits.prepare_timeout,
                stream_timeout: limits.stream_timeout,
                stall_timeout: limits.stall_timeout,
            },
            hooks,
        }))
    }
}

#[cfg(test)]
#[path = "sitemap_csv_stream_sink_tests.rs"]
mod sink_tests;

#[cfg(test)]
#[path = "sitemap_csv_stream_lifecycle_tests.rs"]
mod lifecycle_tests;
