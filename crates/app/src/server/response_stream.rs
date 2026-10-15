//! Owned policy/resource producers and their later response streams.
//! Does not own Hyper connections or promise finite non-reader TCP shutdown.
use axum::{
    body::{Body, Bytes},
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use futures::{Stream, task::AtomicWaker};
use open_seo_spider_core::{OwnedTask, TaskAdmission, TaskOwner};
use std::{
    collections::BTreeMap,
    future::Future,
    io::{self, Write},
    pin::Pin,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};
use tokio::{
    runtime::Handle,
    sync::{Mutex as AsyncMutex, Notify, mpsc, oneshot, watch},
    task::JoinHandle,
};

const CHUNK_BYTES: usize = 65_536;
const QUEUE_CHUNKS: usize = 8;

#[derive(Debug, Clone)]
pub(super) struct Failure {
    message: String,
    readiness_lost: bool,
}
impl Failure {
    pub(super) fn readiness_lost(&self) -> bool {
        self.readiness_lost
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Failure {}
fn failure(message: impl ToString) -> Failure {
    Failure {
        message: message.to_string(),
        readiness_lost: false,
    }
}
fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BodyOutcome {
    Complete,
    Aborted,
    Failed,
}
#[derive(Default)]
struct ControlState {
    cancelled: bool,
    terminal: Option<BodyOutcome>,
}
struct Control {
    state: Mutex<ControlState>,
    cancelled: watch::Sender<bool>,
    body_done: watch::Sender<bool>,
    body_waker: AtomicWaker,
    counters: Arc<Counters>,
    connection: Mutex<Option<super::transport::Lease>>,
}
impl Control {
    fn new(counters: Arc<Counters>) -> Self {
        Self {
            state: Default::default(),
            cancelled: watch::channel(false).0,
            body_done: watch::channel(false).0,
            body_waker: AtomicWaker::new(),
            counters,
            connection: Mutex::new(None),
        }
    }
    fn is_cancelled(&self) -> bool {
        lock(&self.state).cancelled
    }
    fn check(&self) -> io::Result<()> {
        if self.is_cancelled() {
            Err(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "Response export cancelled",
            ))
        } else {
            Ok(())
        }
    }
    fn cancel(&self) {
        let mut state = lock(&self.state);
        if state.terminal == Some(BodyOutcome::Complete) {
            return;
        }
        state.cancelled = true;
        drop(state);
        self.cancelled.send_replace(true);
        self.body_waker.wake();
        self.counters.changed.notify_waiters();
    }
    fn finish_body(&self, requested: BodyOutcome) -> BodyOutcome {
        let mut state = lock(&self.state);
        if let Some(outcome) = state.terminal {
            return outcome;
        }
        let outcome = if requested == BodyOutcome::Complete && state.cancelled {
            BodyOutcome::Aborted
        } else {
            requested
        };
        state.terminal = Some(outcome);
        if outcome != BodyOutcome::Complete {
            state.cancelled = true;
        }
        match outcome {
            BodyOutcome::Complete => {
                self.counters.complete.fetch_add(1, Ordering::SeqCst);
            }
            BodyOutcome::Aborted => {
                self.counters.aborted.fetch_add(1, Ordering::SeqCst);
            }
            BodyOutcome::Failed => {
                self.counters.failed.fetch_add(1, Ordering::SeqCst);
            }
        }
        drop(state);
        if outcome != BodyOutcome::Complete {
            self.cancelled.send_replace(true);
        }
        self.body_done.send_replace(true);
        drop(lock(&self.connection).take());
        self.body_waker.wake();
        self.counters.changed.notify_waiters();
        outcome
    }
    async fn wait_body(&self) {
        let mut done = self.body_done.subscribe();
        loop {
            let finished = *done.borrow_and_update();
            if finished {
                break;
            }
            if done.changed().await.is_err() {
                break;
            }
        }
    }
}

#[derive(Default)]
struct Counters {
    complete: AtomicU64,
    aborted: AtomicU64,
    failed: AtomicU64,
    enqueues: AtomicU64,
    full_reservations: AtomicU64,
    max_chunk: AtomicUsize,
    peak_queued: AtomicUsize,
    response_supervisors_joined: AtomicU64,
    changed: Notify,
}
#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct Stats {
    pub admitted: u64,
    pub started: u64,
    pub joined: u64,
    pub retained: usize,
    pub complete: u64,
    pub aborted: u64,
    pub failed: u64,
    pub enqueues: u64,
    pub full_reservations: u64,
    pub max_chunk: usize,
    pub peak_queued: usize,
    pub response_supervisors_joined: u64,
    pub first_failure: Option<String>,
}
type WorkerResult = Result<(), Failure>;
struct Worker {
    ticket: Option<OwnedTask<WorkerResult>>,
    observed: Option<WorkerResult>,
}
struct Task {
    control: Arc<Control>,
    worker: AsyncMutex<Worker>,
    completion: Mutex<Option<oneshot::Sender<WorkerResult>>>,
    supervisor: AsyncMutex<Option<JoinHandle<()>>>,
    completed: AtomicBool,
}
#[derive(Default)]
struct Tasks {
    closed: bool,
    next: u64,
    running: BTreeMap<u64, Arc<Task>>,
    first_failure: Option<String>,
}
struct Inner {
    tasks: Mutex<Tasks>,
    workers: TaskOwner,
    counters: Arc<Counters>,
}
#[derive(Clone)]
pub(super) struct Registry(Arc<Inner>);
impl Registry {
    pub(super) fn new(admission: TaskAdmission) -> Self {
        Self(Arc::new(Inner {
            tasks: Default::default(),
            workers: TaskOwner::with_admission("http_response_producers", admission),
            counters: Default::default(),
        }))
    }
    fn tasks(&self) -> MutexGuard<'_, Tasks> {
        match self.0.tasks.lock() {
            Ok(tasks) => tasks,
            Err(poisoned) => {
                let mut tasks = poisoned.into_inner();
                tasks.closed = true;
                tasks
                    .first_failure
                    .get_or_insert_with(|| "Response registry bookkeeping lock poisoned".into());
                tasks
            }
        }
    }
    pub(super) fn close_admission(&self) {
        self.0.workers.close_admission();
        self.tasks().closed = true;
    }
    // Future owned-transport deadline calls this explicitly. Merely closing
    // admission or starting a graceful drain does not shorten a live response.
    pub(super) fn cancel_all(&self) {
        let tasks: Vec<_> = self.tasks().running.values().cloned().collect();
        for task in tasks {
            task.control.cancel();
        }
    }
    pub(super) fn stats(&self) -> Stats {
        let worker = self.0.workers.snapshot();
        let tasks = self.tasks();
        let c = &self.0.counters;
        Stats {
            admitted: worker.admitted,
            started: worker.started,
            joined: worker.joined,
            retained: tasks.running.len(),
            complete: c.complete.load(Ordering::SeqCst),
            aborted: c.aborted.load(Ordering::SeqCst),
            failed: c.failed.load(Ordering::SeqCst),
            enqueues: c.enqueues.load(Ordering::SeqCst),
            full_reservations: c.full_reservations.load(Ordering::SeqCst),
            max_chunk: c.max_chunk.load(Ordering::SeqCst),
            peak_queued: c.peak_queued.load(Ordering::SeqCst),
            response_supervisors_joined: c.response_supervisors_joined.load(Ordering::SeqCst),
            first_failure: tasks.first_failure.clone().or(worker.first_failure),
        }
    }
    pub(super) async fn start<M, F>(&self, operation: F) -> Result<Pending<M>, Failure>
    where
        M: Send + 'static,
        F: FnOnce(&mut ChannelWriter, &mut Ready<M>) -> anyhow::Result<()> + Send + 'static,
    {
        self.reap_completed().await?;
        let runtime = Handle::try_current().map_err(failure)?;
        let mut tasks = self.tasks();
        if tasks.closed {
            return Err(failure("Response export admission is closed"));
        }
        let id = tasks
            .next
            .checked_add(1)
            .ok_or_else(|| failure("Response export identifier exhausted"))?;
        let counters = self.0.counters.clone();
        let control = Arc::new(Control::new(counters.clone()));
        let disconnected = Arc::downgrade(&control);
        *lock(&control.connection) = super::transport::bind_current(move || {
            if let Some(control) = disconnected.upgrade() {
                control.cancel();
            }
        })
        .map_err(failure)?;
        let (sender, receiver) = mpsc::channel(QUEUE_CHUNKS);
        let (ready, ready_result) = oneshot::channel();
        let (completed, joined) = oneshot::channel();
        let (start, started) = oneshot::channel::<()>();
        let worker_control = control.clone();
        let ticket = self
            .0
            .workers
            .spawn_blocking(move || {
                started.blocking_recv().map_err(failure)?;
                let mut writer = ChannelWriter {
                    sender,
                    control: worker_control.clone(),
                    runtime,
                    counters,
                };
                let mut ready = Ready {
                    sender: Some(ready),
                    sent: false,
                    control: worker_control,
                };
                let mut result = operation(&mut writer, &mut ready).map_err(failure);
                if result.is_ok() && !ready.sent {
                    result = Err(failure(
                        "Response producer completed without metadata readiness",
                    ));
                }
                if let Some(sender) = ready.sender.take() {
                    let message = result
                        .as_ref()
                        .err()
                        .cloned()
                        .unwrap_or_else(|| failure("Response metadata unavailable"));
                    let _ = sender.send(Err(message));
                }
                result
            })
            .map_err(failure)?;
        let task = Arc::new(Task {
            control: control.clone(),
            worker: AsyncMutex::new(Worker {
                ticket: Some(ticket),
                observed: None,
            }),
            completion: Mutex::new(Some(completed)),
            supervisor: AsyncMutex::new(None),
            completed: AtomicBool::new(false),
        });
        let supervised = task.clone();
        let registry = self.clone();
        let supervisor = tokio::spawn(async move {
            registry.observe_worker(&supervised).await;
            supervised.control.wait_body().await;
            supervised.completed.store(true, Ordering::Release);
            registry.0.counters.changed.notify_waiters();
        });
        *task
            .supervisor
            .try_lock()
            .expect("New response supervisor slot is exclusively owned") = Some(supervisor);
        tasks.running.insert(id, task);
        tasks.next = id;
        // Publish all actual handles and the registry slot before any IO starts.
        let _ = start.send(());
        Ok(Pending {
            ready: Some(ready_result),
            receiver: Some(receiver),
            joined: Some(joined),
            control,
            attached: false,
        })
    }
    async fn observe_worker(&self, task: &Task) {
        let mut worker = task.worker.lock().await;
        if worker.observed.is_some() {
            return;
        }
        let result = if let Some(ticket) = worker.ticket.as_mut() {
            match ticket.await {
                Ok(result) => result,
                Err(error) => {
                    self.tasks()
                        .first_failure
                        .get_or_insert_with(|| error.to_string());
                    Err(failure(error))
                }
            }
        } else {
            let error = failure("Response producer has neither retained ticket nor joined outcome");
            self.tasks()
                .first_failure
                .get_or_insert_with(|| error.to_string());
            Err(error)
        };
        worker.ticket.take();
        worker.observed = Some(result.clone());
        // Core ticket resolves only after actual blocking-worker AND its Core
        // supervisor joins. Queue closure alone never supplies positive EOF.
        if let Some(completion) = lock(&task.completion).take() {
            let _ = completion.send(result);
        }
        self.0.counters.changed.notify_waiters();
    }
    async fn finish_task(&self, id: u64, task: &Arc<Task>) {
        self.observe_worker(task).await;
        let mut supervisor = task.supervisor.lock().await;
        if let Some(handle) = supervisor.as_mut() {
            if let Err(error) = handle.await {
                task.control.cancel();
                self.tasks()
                    .first_failure
                    .get_or_insert_with(|| format!("Response supervisor join failed: {error}"));
            }
            supervisor.take();
            self.0
                .counters
                .response_supervisors_joined
                .fetch_add(1, Ordering::SeqCst);
        }
        drop(supervisor);
        // Also required on the supervisor-error recovery path. A signal is not
        // proof that an unpolled network Body has actually closed.
        task.control.wait_body().await;
        let mut tasks = self.tasks();
        if tasks
            .running
            .get(&id)
            .is_some_and(|current| Arc::ptr_eq(current, task))
        {
            tasks.running.remove(&id);
        }
    }
    pub(super) async fn reap_completed(&self) -> Result<(), Failure> {
        let done: Vec<_> = self
            .tasks()
            .running
            .iter()
            .filter(|(_, t)| t.completed.load(Ordering::Acquire))
            .map(|(id, t)| (*id, t.clone()))
            .collect();
        for (id, task) in done {
            self.finish_task(id, &task).await;
        }
        if let Some(error) = self.tasks().first_failure.clone() {
            return Err(failure(error));
        }
        Ok(())
    }
    pub(super) async fn shutdown_and_drain(&self) -> anyhow::Result<()> {
        self.close_admission();
        let tasks: Vec<_> = self
            .tasks()
            .running
            .iter()
            .map(|(id, t)| (*id, t.clone()))
            .collect();
        for (id, task) in tasks {
            self.finish_task(id, &task).await;
        }
        let workers = self.0.workers.shutdown_and_drain().await;
        let stats = self.stats();
        if let Some(error) = stats.first_failure {
            anyhow::bail!("Response ownership failed: {error}");
        }
        workers?;
        anyhow::ensure!(
            stats.retained == 0
                && stats.admitted == stats.joined
                && stats.started == stats.joined
                && stats.response_supervisors_joined == stats.admitted,
            "Response actual joins are incomplete"
        );
        Ok(())
    }
}

pub(super) struct Ready<M> {
    sender: Option<oneshot::Sender<Result<M, Failure>>>,
    sent: bool,
    control: Arc<Control>,
}
impl<M> Ready<M> {
    pub(super) fn send(&mut self, metadata: M) -> anyhow::Result<()> {
        self.control.check()?;
        if let Some(sender) = self.sender.take() {
            sender
                .send(Ok(metadata))
                .map_err(|_| anyhow::anyhow!("Response request disconnected before readiness"))?;
            self.sent = true;
        }
        Ok(())
    }
}
pub(super) struct ChannelWriter {
    sender: mpsc::Sender<Bytes>,
    control: Arc<Control>,
    runtime: Handle,
    counters: Arc<Counters>,
}
impl Write for ChannelWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for chunk in bytes.chunks(CHUNK_BYTES) {
            self.control.check()?;
            let permit = match self.sender.try_reserve() {
                Ok(permit) => permit,
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "Response consumer disconnected",
                    ));
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // A real failed reservation, not a byte-size/timing guess.
                    self.counters
                        .full_reservations
                        .fetch_add(1, Ordering::SeqCst);
                    self.counters.changed.notify_waiters();
                    let mut cancelled = self.control.cancelled.subscribe();
                    let runtime = self.runtime.clone();
                    runtime.block_on(async {
                        loop {
                            self.control.check()?;
                            tokio::select! {
                                biased;
                                result=cancelled.changed()=>{if result.is_err(){return Err(io::Error::new(io::ErrorKind::ConnectionAborted,"Response cancellation source closed"));}},
                                result=self.sender.reserve()=>return result.map_err(|_|io::Error::new(io::ErrorKind::BrokenPipe,"Response consumer disconnected")),
                            }
                        }
                    })?
                }
            };
            self.control.check()?;
            permit.send(Bytes::copy_from_slice(chunk));
            self.counters.enqueues.fetch_add(1, Ordering::SeqCst);
            self.counters
                .max_chunk
                .fetch_max(chunk.len(), Ordering::SeqCst);
            self.counters
                .peak_queued
                .fetch_max(QUEUE_CHUNKS - self.sender.capacity(), Ordering::SeqCst);
            self.counters.changed.notify_waiters();
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.control.check()
    }
}

pub(super) struct Pending<M> {
    ready: Option<oneshot::Receiver<Result<M, Failure>>>,
    receiver: Option<mpsc::Receiver<Bytes>>,
    joined: Option<oneshot::Receiver<WorkerResult>>,
    control: Arc<Control>,
    attached: bool,
}
impl<M> Pending<M> {
    pub(super) async fn ready(mut self) -> Result<(M, Body), Failure> {
        let result = self
            .ready
            .as_mut()
            .expect("Pending readiness receiver exists")
            .await
            .map_err(|error| Failure {
                message: error.to_string(),
                readiness_lost: true,
            })
            .and_then(|r| r);
        let metadata = match result {
            Ok(value) => value,
            Err(error) => {
                self.control.finish_body(BodyOutcome::Failed);
                return Err(error);
            }
        };
        let stream = BodyStream {
            receiver: self.receiver.take().expect("Pending body receiver exists"),
            joined: self
                .joined
                .take()
                .expect("Pending completion receiver exists"),
            control: self.control.clone(),
            finished: false,
        };
        self.attached = true;
        Ok((metadata, Body::from_stream(stream)))
    }
}
impl<M> Drop for Pending<M> {
    fn drop(&mut self) {
        if !self.attached {
            self.control.cancel();
            self.control.finish_body(BodyOutcome::Aborted);
        }
    }
}
struct BodyStream {
    receiver: mpsc::Receiver<Bytes>,
    joined: oneshot::Receiver<WorkerResult>,
    control: Arc<Control>,
    finished: bool,
}
impl Stream for BodyStream {
    type Item = Result<Bytes, io::Error>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.finished {
            return Poll::Ready(None);
        }
        self.control.body_waker.register(cx.waker());
        if self.control.is_cancelled() {
            self.receiver.close();
            self.finished = true;
            self.control.finish_body(BodyOutcome::Aborted);
            return Poll::Ready(Some(Err(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "Response stream aborted",
            ))));
        }
        match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(bytes)) => Poll::Ready(Some(Ok(bytes))),
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => match Pin::new(&mut self.joined).poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(result) => {
                    self.finished = true;
                    match result {
                        Ok(Ok(()))
                            if self.control.finish_body(BodyOutcome::Complete)
                                == BodyOutcome::Complete =>
                        {
                            Poll::Ready(None)
                        }
                        Ok(Ok(())) => Poll::Ready(Some(Err(io::Error::new(
                            io::ErrorKind::ConnectionAborted,
                            "Response cancelled before complete EOF",
                        )))),
                        Ok(Err(error)) => {
                            self.control.finish_body(BodyOutcome::Failed);
                            Poll::Ready(Some(Err(io::Error::other(error.to_string()))))
                        }
                        Err(error) => {
                            self.control.finish_body(BodyOutcome::Failed);
                            Poll::Ready(Some(Err(io::Error::other(format!(
                                "Response lost actual joined completion: {error}"
                            )))))
                        }
                    }
                }
            },
        }
    }
}
impl Drop for BodyStream {
    fn drop(&mut self) {
        if !self.finished {
            self.control.cancel();
            self.control.finish_body(BodyOutcome::Aborted);
        }
    }
}

// Deliberately local to the four unknown-length policy/resource MethodRouters.
pub(super) async fn require_http11(request: Request, next: Next) -> Response {
    if request.version() != axum::http::Version::HTTP_10 {
        return next.run(request).await;
    }
    let bytes = Bytes::from_static(b"{\"error\":\"This streamed download requires HTTP/1.1\"}");
    let length = bytes.len().to_string();
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::HTTP_VERSION_NOT_SUPPORTED;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json"),
    );
    response.headers_mut().insert(
        header::CONNECTION,
        axum::http::HeaderValue::from_static("close"),
    );
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        length
            .parse()
            .expect("Decimal static error-body length is a valid header"),
    );
    response
}

#[cfg(test)]
#[path = "response_stream_tests.rs"]
mod tests;
