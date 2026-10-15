//! Owned incoming HTTP/1 transports only. Outbound work keeps its own owner.
use axum::{
    Router,
    extract::Request,
    middleware::{self, Next},
};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
    service::TowerToHyperService,
};
use open_seo_spider_core::{OwnedTask, TaskAdmission, TaskOwner};
use std::{
    collections::BTreeMap,
    future::Future,
    io,
    pin::Pin,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream},
    sync::{Mutex as AsyncMutex, Notify, oneshot, watch},
    time::Instant,
};
const GRACE: Duration = Duration::from_secs(5);
const REAP: Duration = Duration::from_millis(25);
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Running,
    Quiescing(Instant),
    Aborting,
}
type Abort = Arc<dyn Fn() + Send + Sync>;
struct Callback(Abort);
struct ConnectionState {
    closed: AtomicBool,
    io_dropped: AtomicBool,
    write_pending: AtomicU64,
    next: AtomicU64,
    bindings: Mutex<BTreeMap<u64, Weak<Callback>>>,
    failure: Mutex<Option<String>>,
    changed: Arc<Notify>,
}
#[derive(Clone)]
pub(super) struct Connection(Arc<ConnectionState>);
pub(super) struct Lease {
    connection: Weak<ConnectionState>,
    id: u64,
    _callback: Arc<Callback>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        if let Some(c) = self.connection.upgrade() {
            lock(&c.bindings).remove(&self.id);
        }
    }
}
impl Connection {
    fn new(changed: Arc<Notify>) -> Self {
        Self(Arc::new(ConnectionState {
            closed: AtomicBool::new(false),
            io_dropped: AtomicBool::new(false),
            write_pending: AtomicU64::new(0),
            next: AtomicU64::new(0),
            bindings: Default::default(),
            failure: Default::default(),
            changed,
        }))
    }
    fn callback(&self, callback: &Callback) {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (callback.0)())).is_err() {
            lock(&self.0.failure)
                .get_or_insert_with(|| "Private response cancellation callback panicked".into());
        }
    }
    fn bind(&self, cancel: Abort) -> anyhow::Result<Lease> {
        let id = self
            .0
            .next
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_add(1))
            .map_err(|_| anyhow::anyhow!("Connection binding identifier exhausted"))?;
        let callback = Arc::new(Callback(cancel));
        let mut bindings = lock(&self.0.bindings);
        if self.0.closed.load(Ordering::SeqCst) {
            drop(bindings);
            self.callback(&callback);
        } else {
            bindings.insert(id, Arc::downgrade(&callback));
        }
        Ok(Lease {
            connection: Arc::downgrade(&self.0),
            id,
            _callback: callback,
        })
    }
    fn terminate(&self) {
        if self.0.closed.swap(true, Ordering::SeqCst) {
            return;
        }
        let callbacks: Vec<_> = lock(&self.0.bindings)
            .values()
            .filter_map(Weak::upgrade)
            .collect();
        for callback in callbacks {
            self.callback(&callback);
        }
        self.0.changed.notify_waiters();
    }
}
tokio::task_local! {static CURRENT:Connection;}
pub(super) async fn scope<F: Future>(connection: Option<Connection>, future: F) -> F::Output {
    match connection {
        Some(c) => CURRENT.scope(c, future).await,
        None => future.await,
    }
}
pub(super) fn bind_current(
    cancel: impl Fn() + Send + Sync + 'static,
) -> anyhow::Result<Option<Lease>> {
    CURRENT
        .try_with(Clone::clone)
        .ok()
        .map(|c| c.bind(Arc::new(cancel)))
        .transpose()
}

// The real socket is explicitly dropped before terminal/cancellation publication.
struct IncomingIo {
    stream: Option<TcpStream>,
    control: Connection,
}
impl Drop for IncomingIo {
    fn drop(&mut self) {
        drop(self.stream.take());
        self.control.0.io_dropped.store(true, Ordering::SeqCst);
        self.control.terminate();
    }
}
impl AsyncRead for IncomingIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(self.stream.as_mut().expect("Live incoming socket")).poll_read(cx, buffer)
    }
}
impl AsyncWrite for IncomingIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let value =
            Pin::new(self.stream.as_mut().expect("Live incoming socket")).poll_write(cx, bytes);
        if value.is_pending() {
            self.control.0.write_pending.fetch_add(1, Ordering::SeqCst);
            self.control.0.changed.notify_waiters();
        }
        value
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(self.stream.as_mut().expect("Live incoming socket")).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(self.stream.as_mut().expect("Live incoming socket")).poll_shutdown(cx)
    }
}
#[derive(Clone, Copy, Debug)]
enum Outcome {
    Complete,
    PeerError,
    ShutdownAborted,
}
struct JoinSlot<T> {
    ticket: AsyncMutex<Option<OwnedTask<T>>>,
    published: watch::Sender<bool>,
    observed: Mutex<Option<Result<T, String>>>,
}
impl<T: Clone + Send + 'static> JoinSlot<T> {
    fn new() -> Self {
        Self {
            ticket: Default::default(),
            published: watch::channel(false).0,
            observed: Default::default(),
        }
    }
    fn publish(&self, ticket: OwnedTask<T>) {
        *self
            .ticket
            .try_lock()
            .expect("Unpublished join slot exclusively held") = Some(ticket);
        self.published.send_replace(true);
    }
    async fn join(&self) -> Result<T, String> {
        let mut ready = self.published.subscribe();
        loop {
            if *ready.borrow_and_update() {
                break;
            }
            ready.changed().await.map_err(|e| e.to_string())?;
        }
        if let Some(r) = lock(&self.observed).clone() {
            return r;
        }
        let mut ticket = self.ticket.lock().await;
        if let Some(r) = lock(&self.observed).clone() {
            return r;
        }
        let result = match ticket.as_mut() {
            Some(ticket) => ticket.await.map_err(|e| e.to_string()),
            None => Err("Actual incoming task ticket missing".into()),
        };
        ticket.take();
        *lock(&self.observed) = Some(result.clone());
        result
    }
}
struct Slot {
    control: Connection,
    join: JoinSlot<Outcome>,
}
struct State {
    started: bool,
    next: u64,
    slots: BTreeMap<u64, Arc<Slot>>,
    abort: Option<Abort>,
    failure: Option<String>,
}
struct Inner {
    admission: TaskAdmission,
    coordinator: TaskOwner,
    connections: TaskOwner,
    state: Mutex<State>,
    coordinator_join: JoinSlot<Result<(), String>>,
    reaping: AsyncMutex<()>,
    phase: watch::Sender<Phase>,
    failed: watch::Sender<bool>,
    changed: Arc<Notify>,
    listener_dropped: AtomicBool,
    accepted: AtomicU64,
    rejected: AtomicU64,
    complete: AtomicU64,
    peer_errors: AtomicU64,
    aborted: AtomicU64,
    io_dropped: AtomicU64,
    write_pending: AtomicU64,
}
#[derive(Clone)]
pub(super) struct Registry(Arc<Inner>);
pub(super) struct Running {
    registry: Registry,
}
impl Drop for Running {
    fn drop(&mut self) {
        self.registry.quiesce_at(Instant::now());
    }
}
impl Running {
    pub(super) async fn wait(&self) -> anyhow::Result<()> {
        self.registry.wait_coordinator().await
    }
}
impl Registry {
    pub(super) fn new(admission: TaskAdmission) -> Self {
        Self(Arc::new(Inner {
            coordinator: TaskOwner::with_admission(
                "incoming_accept_coordinator",
                admission.clone(),
            ),
            connections: TaskOwner::with_admission("incoming_http_connections", admission.clone()),
            admission,
            state: Mutex::new(State {
                started: false,
                next: 0,
                slots: BTreeMap::new(),
                abort: None,
                failure: None,
            }),
            coordinator_join: JoinSlot::new(),
            reaping: Default::default(),
            phase: watch::channel(Phase::Running).0,
            failed: watch::channel(false).0,
            changed: Arc::new(Notify::new()),
            listener_dropped: AtomicBool::new(false),
            accepted: AtomicU64::new(0),
            rejected: AtomicU64::new(0),
            complete: AtomicU64::new(0),
            peer_errors: AtomicU64::new(0),
            aborted: AtomicU64::new(0),
            io_dropped: AtomicU64::new(0),
            write_pending: AtomicU64::new(0),
        }))
    }
    fn record_failure(&self, message: impl ToString) {
        let message = message.to_string();
        lock(&self.0.state).failure.get_or_insert(message.clone());
        self.0.coordinator.record_failure(message);
        self.0.failed.send_replace(true);
    }
    pub(super) async fn wait_failure(&self) {
        let mut changed = self.0.failed.subscribe();
        loop {
            if *changed.borrow_and_update() {
                return;
            }
            if changed.changed().await.is_err() {
                return;
            }
        }
    }
    pub(super) fn quiesce_at(&self, t0: Instant) {
        self.0.admission.close();
        self.0.phase.send_if_modified(|phase| {
            if matches!(phase, Phase::Running) {
                *phase = Phase::Quiescing(t0 + GRACE);
                true
            } else {
                false
            }
        });
    }
    fn abort(&self) {
        let first = self.0.phase.send_if_modified(|phase| {
            if *phase == Phase::Aborting {
                false
            } else {
                *phase = Phase::Aborting;
                true
            }
        });
        if !first {
            return;
        }
        let abort = lock(&self.0.state).abort.clone();
        if let Some(abort) = abort
            && std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| abort())).is_err()
        {
            self.record_failure("Incoming deadline cancellation callback panicked");
        }
    }
    pub(super) fn start(
        &self,
        listener: TcpListener,
        router: Router,
        abort: Abort,
    ) -> anyhow::Result<Running> {
        {
            let mut state = lock(&self.0.state);
            anyhow::ensure!(!state.started, "Incoming listener already registered");
            anyhow::ensure!(!self.0.admission.is_closed(), "Incoming admission closed");
            state.started = true;
            state.abort = Some(abort);
        }
        let registry = self.clone();
        let ticket = match self.0.coordinator.spawn_async(async move {
            let mut guard = CoordinatorGuard {
                registry: registry.clone(),
                finished: false,
            };
            let result = registry.accept(listener, router).await;
            guard.finished = true;
            if let Err(e) = &result {
                registry.record_failure(e);
                registry.abort();
            }
            result
        }) {
            Ok(ticket) => ticket,
            Err(e) => {
                let message = e.to_string();
                *lock(&self.0.coordinator_join.observed) = Some(Err(message.clone()));
                self.0.coordinator_join.published.send_replace(true);
                self.record_failure(&message);
                return Err(e.into());
            }
        };
        self.0.coordinator_join.publish(ticket);
        Ok(Running {
            registry: self.clone(),
        })
    }
    fn register(&self, stream: TcpStream, router: Router) -> anyhow::Result<()> {
        let control = Connection::new(self.0.changed.clone());
        let io = IncomingIo {
            stream: Some(stream),
            control: control.clone(),
        };
        let slot = Arc::new(Slot {
            control,
            join: JoinSlot::new(),
        });
        let id = {
            let mut state = lock(&self.0.state);
            state.next = state
                .next
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("Incoming connection identifier exhausted"))?;
            state.next
        };
        let (start, started) = oneshot::channel();
        let phase = self.0.phase.subscribe();
        let connection = slot.control.clone();
        let ticket = match self.0.connections.spawn_async(async move {
            if started.await.is_err() {
                drop(io);
                return Outcome::ShutdownAborted;
            }
            serve(io, router, connection, phase).await
        }) {
            Ok(ticket) => ticket,
            Err(error) => {
                self.0.rejected.fetch_add(1, Ordering::SeqCst);
                anyhow::ensure!(
                    slot.control.0.io_dropped.load(Ordering::SeqCst),
                    "Refused incoming socket did not actually drop"
                );
                self.0.io_dropped.fetch_add(1, Ordering::SeqCst);
                return if self.0.admission.is_closed() {
                    Ok(())
                } else {
                    Err(error.into())
                };
            }
        };
        slot.join.publish(ticket);
        lock(&self.0.state).slots.insert(id, slot);
        let _ = start.send(());
        Ok(())
    }
    async fn reap(&self) -> Result<(), String> {
        // Concurrent/cancelled drain waiters must not count the same actual
        // connection twice, including after a failed accept coordinator.
        let _reaping = self.0.reaping.lock().await;
        let done: Vec<_> = lock(&self.0.state)
            .slots
            .iter()
            .filter(|(_, s)| s.control.0.closed.load(Ordering::SeqCst))
            .map(|(id, s)| (*id, s.clone()))
            .collect();
        for (id, slot) in done {
            let outcome = slot.join.join().await;
            if let Some(e) = lock(&slot.control.0.failure).clone() {
                self.record_failure(e);
            }
            self.0.write_pending.fetch_add(
                slot.control.0.write_pending.load(Ordering::SeqCst),
                Ordering::SeqCst,
            );
            if slot.control.0.io_dropped.load(Ordering::SeqCst) {
                self.0.io_dropped.fetch_add(1, Ordering::SeqCst);
            } else {
                self.record_failure("Incoming IO left scope without actual socket drop");
            }
            match outcome {
                Ok(Outcome::Complete) => {
                    self.0.complete.fetch_add(1, Ordering::SeqCst);
                }
                Ok(Outcome::PeerError) => {
                    self.0.peer_errors.fetch_add(1, Ordering::SeqCst);
                }
                Ok(Outcome::ShutdownAborted) => {
                    self.0.aborted.fetch_add(1, Ordering::SeqCst);
                }
                Err(e) => self.record_failure(e),
            }
            lock(&self.0.state).slots.remove(&id);
        }
        self.0.connections.reap_completed().await;
        Ok(())
    }
    async fn accept(&self, listener: TcpListener, router: Router) -> Result<(), String> {
        let mut listener = Some(listener);
        let mut phase = self.0.phase.subscribe();
        let mut tick = tokio::time::interval(REAP);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            if !matches!(*phase.borrow_and_update(), Phase::Running) {
                break;
            }
            tokio::select! {biased;result=phase.changed()=>{result.map_err(|e|e.to_string())?;},_=tick.tick()=>self.reap().await?,accepted=axum::serve::Listener::accept(listener.as_mut().expect("Registered listener held while running"))=>{self.0.accepted.fetch_add(1,Ordering::SeqCst);if matches!(*phase.borrow(),Phase::Running){self.register(accepted.0,router.clone()).map_err(|e|e.to_string())?;}else{drop(accepted.0);self.0.io_dropped.fetch_add(1,Ordering::SeqCst);self.0.rejected.fetch_add(1,Ordering::SeqCst);}}}
        }
        drop(listener.take());
        self.0.listener_dropped.store(true, Ordering::SeqCst);
        loop {
            self.reap().await?;
            if lock(&self.0.state).slots.is_empty() {
                break;
            }
            let current = *phase.borrow_and_update();
            match current {
                Phase::Quiescing(deadline) => {
                    tokio::select! {biased;_=tokio::time::sleep_until(deadline)=>self.abort(),result=phase.changed()=>{result.map_err(|e|e.to_string())?;},_=self.0.changed.notified()=>(),_=tick.tick()=>()}
                }
                Phase::Aborting => {
                    tokio::select! {_=self.0.changed.notified()=>(),_=tick.tick()=>()}
                }
                Phase::Running => return Err("Incoming quiesce phase regressed".into()),
            }
        }
        Ok(())
    }
    async fn wait_coordinator(&self) -> anyhow::Result<()> {
        let started = lock(&self.0.state).started;
        if !started {
            return Ok(());
        }
        match self.0.coordinator_join.join().await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) | Err(e) => {
                self.record_failure(&e);
                self.abort();
                Err(anyhow::anyhow!(e))
            }
        }
    }
    pub(super) async fn shutdown_and_drain(&self) -> anyhow::Result<()> {
        self.quiesce_at(Instant::now());
        let first = self.wait_coordinator().await;
        self.abort_after_failure(&first);
        let slots: Vec<_> = lock(&self.0.state).slots.values().cloned().collect();
        for slot in slots {
            let _ = slot.join.join().await;
        }
        self.reap().await.map_err(anyhow::Error::msg)?;
        let (a, c) = tokio::join!(
            self.0.coordinator.shutdown_and_drain(),
            self.0.connections.shutdown_and_drain()
        );
        let mut errors = Vec::new();
        for r in [
            first,
            a.map_err(anyhow::Error::from),
            c.map_err(anyhow::Error::from),
        ] {
            if let Err(e) = r {
                errors.push(e.to_string());
            }
        }
        if let Some(e) = lock(&self.0.state).failure.clone() {
            errors.push(e);
        }
        anyhow::ensure!(
            errors.is_empty(),
            "Incoming transport drain failed: {}",
            errors.join("; ")
        );
        Ok(())
    }
    fn abort_after_failure(&self, result: &anyhow::Result<()>) {
        if result.is_err() {
            self.abort();
        }
    }
    pub(super) fn stats(&self) -> serde_json::Value {
        let state = lock(&self.0.state);
        let live_pending = state
            .slots
            .values()
            .map(|s| s.control.0.write_pending.load(Ordering::SeqCst))
            .sum::<u64>();
        let a = self.0.coordinator.snapshot();
        let c = self.0.connections.snapshot();
        serde_json::json!({"incoming_protocol":"http1_plain_tcp_no_application_upgrades","grace_seconds":5,"registered":state.started,"listener_dropped":self.0.listener_dropped.load(Ordering::SeqCst),"accepted":self.0.accepted.load(Ordering::SeqCst),"rejected":self.0.rejected.load(Ordering::SeqCst),"retained":state.slots.len(),"complete":self.0.complete.load(Ordering::SeqCst),"peer_errors":self.0.peer_errors.load(Ordering::SeqCst),"shutdown_aborted":self.0.aborted.load(Ordering::SeqCst),"actual_io_dropped":self.0.io_dropped.load(Ordering::SeqCst),"tcp_write_pending":live_pending+self.0.write_pending.load(Ordering::SeqCst),"accept_admitted":a.admitted,"accept_joined":a.joined,"accept_retained":a.retained,"connection_admitted":c.admitted,"connection_joined":c.joined,"connection_retained":c.retained,"first_failure":state.failure,"whole_application_five_second_bound":false})
    }
}
struct CoordinatorGuard {
    registry: Registry,
    finished: bool,
}
impl Drop for CoordinatorGuard {
    fn drop(&mut self) {
        if !self.finished {
            self.registry
                .record_failure("Incoming accept coordinator stopped unexpectedly");
            self.registry.abort();
        }
    }
}
async fn serve(
    io: IncomingIo,
    router: Router,
    control: Connection,
    mut phase: watch::Receiver<Phase>,
) -> Outcome {
    let inject = control.clone();
    let router = router.layer(middleware::from_fn(
        move |mut request: Request, next: Next| {
            let control = inject.clone();
            async move {
                request.extensions_mut().insert(control);
                next.run(request).await
            }
        },
    ));
    let outcome = {
        let io = TokioIo::new(io);
        let builder = Builder::new(TokioExecutor::new());
        let connection =
            builder.serve_connection_with_upgrades(io, TowerToHyperService::new(router));
        tokio::pin!(connection);
        let mut graceful = false;
        loop {
            let current = *phase.borrow_and_update();
            if current == Phase::Aborting {
                break Outcome::ShutdownAborted;
            }
            if matches!(current, Phase::Quiescing(_)) && !graceful {
                connection.as_mut().graceful_shutdown();
                graceful = true;
            }
            tokio::select! {biased;result=phase.changed()=>{if result.is_err(){break Outcome::ShutdownAborted}},result=connection.as_mut()=>break if result.is_ok(){Outcome::Complete}else{Outcome::PeerError}}
        }
    }; // Actual connection future and owned IncomingIo leave this lexical scope.
    if !control.0.io_dropped.load(Ordering::SeqCst) {
        lock(&control.0.failure).get_or_insert_with(|| {
            "Incoming IO escaped current non-upgrading protocol contract".into()
        });
        control.terminate();
    }
    outcome
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;
