//! Private verified spool and retained blocking producer/body lifetime.
//! Slow unpolled TCP transport abortion is a separate Source6 integration gate.
use axum::body::{Body, Bytes};
use cap_fs_ext::{DirExt, FollowSymlinks, MetadataExt, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::fs::{Dir, OpenOptions};
use open_seo_spider_core::{OwnedTaskError, SitemapVerifiedBodyCopy, TaskAdmission, TaskOwner};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    future::Future,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
    pin::Pin,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::{Notify, mpsc, oneshot};
const BODY_CAP: u64 = 67_108_864;
type Ticket = Pin<Box<dyn Future<Output = Result<End, OwnedTaskError>> + Send>>;
#[derive(Clone)]
pub(super) struct Registry(Arc<Inner>);
struct Inner {
    tasks: TaskOwner,
    slots: Mutex<BTreeMap<String, Arc<Slot>>>,
    failure: Mutex<Option<String>>,
}
struct Slot {
    name: String,
    cancel: AtomicBool,
    consumer_done: AtomicBool,
    changed: Notify,
    wait: Condvar,
    wait_lock: Mutex<()>,
    entry: Mutex<Option<Entry>>,
    ticket: tokio::sync::Mutex<Option<Ticket>>,
    published: tokio::sync::watch::Sender<bool>,
    joined: Mutex<Option<Result<(), String>>>,
}
struct Entry {
    dir: Dir,
    name: String,
    identity: Option<(u64, u64)>,
}
struct End {
    failure: Option<String>,
}
fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
impl Registry {
    pub(super) fn new(admission: TaskAdmission) -> Self {
        Self(Arc::new(Inner {
            tasks: TaskOwner::with_admission("source_body_transports", admission),
            slots: Mutex::new(BTreeMap::new()),
            failure: Mutex::new(None),
        }))
    }
    pub(super) fn close_admission(&self) {
        self.0.tasks.close_admission();
        for slot in lock(&self.0.slots).values() {
            slot.cancel();
        }
    }
    pub(super) fn stats(&self) -> serde_json::Value {
        let s = self.0.tasks.snapshot();
        serde_json::json!({"owner":s.owner,"admitted":s.admitted,"started":s.started,"joined":s.joined,"retained":s.retained,"spool_leases":lock(&self.0.slots).len(),"logical_spool_byte_limit":2*BODY_CAP,"first_failure":lock(&self.0.failure).clone(),"unresponsive_transport_shutdown":"not_proven"})
    }
    async fn join(&self, slot: &Arc<Slot>) -> Result<(), String> {
        let mut published = slot.published.subscribe();
        loop {
            let ready = *published.borrow_and_update();
            if ready {
                break;
            }
            published.changed().await.map_err(|e| e.to_string())?;
        }
        if let Some(r) = lock(&slot.joined).clone() {
            return r;
        }
        let mut ticket = slot.ticket.lock().await;
        if let Some(r) = lock(&slot.joined).clone() {
            return r;
        }
        let result = match ticket.as_mut() {
            Some(t) => match t.await {
                Ok(end) => end.failure.map_or(Ok(()), Err),
                Err(e) => Err(e.to_string()),
            },
            None => Err("Source body join slot missing".into()),
        };
        ticket.take();
        *lock(&slot.joined) = Some(result.clone());
        if let Err(e) = &result {
            lock(&self.0.failure).get_or_insert(e.clone());
            self.0.tasks.record_failure(e);
        } else {
            lock(&self.0.slots).remove(&slot.name);
        }
        result
    }
    pub(super) async fn shutdown_and_drain(&self) -> anyhow::Result<()> {
        self.close_admission();
        let slots: Vec<_> = lock(&self.0.slots).values().cloned().collect();
        let mut errors = Vec::new();
        for slot in slots {
            if let Err(e) = self.join(&slot).await {
                errors.push(e);
            }
        }
        if let Err(e) = self.0.tasks.shutdown_and_drain().await {
            errors.push(e.to_string());
        }
        if let Some(e) = lock(&self.0.failure).clone() {
            errors.push(e);
        }
        anyhow::ensure!(
            errors.is_empty(),
            "Source body drain failed: {}",
            errors.join("; ")
        );
        Ok(())
    }
    #[cfg(test)]
    pub(super) async fn prepare<F>(
        &self,
        data: PathBuf,
        operation: F,
    ) -> anyhow::Result<(SitemapVerifiedBodyCopy, Body)>
    where
        F: FnOnce(
                &mut dyn Write,
                &mut dyn FnMut() -> anyhow::Result<()>,
            ) -> anyhow::Result<SitemapVerifiedBodyCopy>
            + Send
            + 'static,
    {
        self.prepare_checked(data, operation, || Ok(())).await
    }
    pub(super) async fn prepare_checked<F, V>(
        &self,
        data: PathBuf,
        operation: F,
        final_check: V,
    ) -> anyhow::Result<(SitemapVerifiedBodyCopy, Body)>
    where
        F: FnOnce(
                &mut dyn Write,
                &mut dyn FnMut() -> anyhow::Result<()>,
            ) -> anyhow::Result<SitemapVerifiedBodyCopy>
            + Send
            + 'static,
        V: FnOnce() -> anyhow::Result<()> + Send + 'static,
    {
        self.0.tasks.reap_completed().await;
        let finished: Vec<_> = lock(&self.0.slots)
            .values()
            .filter(|s| s.consumer_done.load(Ordering::SeqCst))
            .cloned()
            .collect();
        for slot in finished {
            self.join(&slot).await.map_err(anyhow::Error::msg)?;
        }
        let name = format!("{}.spool", uuid::Uuid::new_v4().simple());
        let slot = Arc::new(Slot {
            name: name.clone(),
            cancel: AtomicBool::new(false),
            consumer_done: AtomicBool::new(false),
            changed: Notify::new(),
            wait: Condvar::new(),
            wait_lock: Mutex::new(()),
            entry: Mutex::new(None),
            ticket: tokio::sync::Mutex::new(None),
            published: tokio::sync::watch::channel(false).0,
            joined: Mutex::new(None),
        });
        let guard = Consumer { slot: slot.clone() };
        let (ready, waiting) = oneshot::channel();
        let (sender, receiver) = mpsc::channel::<io::Result<Bytes>>(4);
        {
            let mut slots = lock(&self.0.slots);
            anyhow::ensure!(
                lock(&self.0.failure).is_none(),
                "Source body cleanup failure retains spool capacity"
            );
            anyhow::ensure!(slots.len() < 2, "Source body spool capacity unavailable");
            slots.insert(name.clone(), slot.clone());
        }
        let running = slot.clone();
        let registry = self.clone();
        let ticket = match self.0.tasks.spawn_blocking(move || {
            worker(
                registry,
                running,
                data,
                operation,
                Box::new(final_check),
                ready,
                sender,
            )
        }) {
            Ok(ticket) => ticket,
            Err(e) => {
                *lock(&slot.joined) = Some(Ok(()));
                slot.published.send_replace(true);
                lock(&self.0.slots).remove(&name);
                return Err(e.into());
            }
        };
        *slot.ticket.lock().await = Some(ticket);
        slot.published.send_replace(true);
        let prepared = match waiting.await {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => {
                drop(guard);
                self.join(&slot).await.map_err(anyhow::Error::msg)?;
                anyhow::bail!(error)
            }
            Err(error) => {
                drop(guard);
                let joined = self.join(&slot).await;
                anyhow::bail!("Source body preparation reply lost: {error}; join: {joined:?}")
            }
        };
        let state = StreamState {
            receiver,
            guard: Some(guard),
            slot,
            registry: self.clone(),
            terminal: false,
        };
        let stream = futures::stream::unfold(state, |mut state| async move {
            if state.terminal {
                return None;
            }
            let next = if state.slot.cancel.load(Ordering::SeqCst) {
                None
            } else {
                tokio::select! {biased;_=state.slot.changed.notified()=>None,value=state.receiver.recv()=>value}
            };
            if let Some(value) = next {
                state.slot.wait.notify_all();
                return Some((value, state));
            }
            let cancelled = state.slot.cancel.load(Ordering::SeqCst);
            drop(state.guard.take());
            let joined = state.registry.join(&state.slot).await;
            state.terminal = true;
            if cancelled {
                return Some((
                    Err(io::Error::other("Source body transport aborted")),
                    state,
                ));
            }
            match joined {
                Ok(()) => None,
                Err(e) => Some((Err(io::Error::other(e)), state)),
            }
        });
        Ok((prepared, Body::from_stream(stream)))
    }
}
struct StreamState {
    receiver: mpsc::Receiver<io::Result<Bytes>>,
    guard: Option<Consumer>,
    slot: Arc<Slot>,
    registry: Registry,
    terminal: bool,
}
struct Consumer {
    slot: Arc<Slot>,
}
impl Drop for Consumer {
    fn drop(&mut self) {
        self.slot.consumer_done.store(true, Ordering::SeqCst);
        self.slot.cancel();
    }
}
impl Slot {
    fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.changed.notify_waiters();
        self.wait.notify_all();
    }
    fn checkpoint(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.cancel.load(Ordering::SeqCst),
            "Source body consumer cancelled"
        );
        Ok(())
    }
    fn wait_consumer(&self) {
        let mut guard = lock(&self.wait_lock);
        while !self.consumer_done.load(Ordering::SeqCst) {
            guard = self
                .wait
                .wait_timeout(guard, Duration::from_millis(25))
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
        }
    }
}
fn root(data: &Path) -> anyhow::Result<Dir> {
    let absolute = if data.is_absolute() {
        data.to_owned()
    } else {
        std::env::current_dir()?.join(data)
    };
    let mut anchor = PathBuf::new();
    let mut parts = Vec::new();
    for p in absolute.components() {
        match p {
            Component::Prefix(p) => anchor.push(p.as_os_str()),
            Component::RootDir => anchor.push(p.as_os_str()),
            Component::Normal(p) => parts.push(p.to_os_string()),
            Component::CurDir => (),
            Component::ParentDir => anyhow::bail!("Spool path traversal forbidden"),
        }
    }
    let mut dir = Dir::open_ambient_dir(anchor, cap_std::ambient_authority())?;
    for p in parts {
        dir = dir.open_dir_nofollow(p)?;
    }
    match dir.create_dir(".source-body-spool") {
        Ok(()) => (),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(e.into()),
    };
    Ok(dir.open_dir_nofollow(".source-body-spool")?)
}
fn create(registry: &Registry, slot: &Slot, data: &Path) -> anyhow::Result<cap_std::fs::File> {
    let dir = root(data)?;
    let known: HashSet<_> = lock(&registry.0.slots).keys().cloned().collect();
    for entry in dir.entries()? {
        let name = entry?.file_name();
        anyhow::ensure!(
            name.to_str().is_some_and(|n| known.contains(n)),
            "Unknown/stale spool retained; capacity requires offline recovery"
        );
    }
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    let file = dir.open_with(&slot.name, &options)?;
    *lock(&slot.entry) = Some(Entry {
        dir,
        name: slot.name.clone(),
        identity: None,
    });
    let m = file.metadata()?;
    anyhow::ensure!(m.is_file(), "Spool must be an ordinary file");
    lock(&slot.entry)
        .as_mut()
        .expect("Created spool entry retained")
        .identity = Some((m.dev(), m.ino()));
    Ok(file)
}
fn cleanup(slot: &Slot) -> anyhow::Result<()> {
    let mut entry = lock(&slot.entry);
    let Some(owned) = entry.as_ref() else {
        return Ok(());
    };
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No).nonblock(true);
    let file = owned.dir.open_with(&owned.name, &options)?;
    let m = file.metadata()?;
    anyhow::ensure!(
        m.is_file() && Some((m.dev(), m.ino())) == owned.identity,
        "Spool cleanup identity uncertain; retained and charged"
    );
    drop(file);
    owned.dir.remove_file(&owned.name)?;
    entry.take();
    Ok(())
}
struct CappedSpool<'a> {
    file: &'a mut cap_std::fs::File,
    bytes: u64,
}
impl Write for CappedSpool<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if data.len() as u64 > BODY_CAP - self.bytes {
            return Err(io::Error::other("Source body private spool exceeds64MiB"));
        }
        let n = self.file.write(data)?;
        self.bytes += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
fn worker<F>(
    registry: Registry,
    slot: Arc<Slot>,
    data: PathBuf,
    operation: F,
    final_check: Box<dyn FnOnce() -> anyhow::Result<()> + Send>,
    ready: oneshot::Sender<Result<SitemapVerifiedBodyCopy, String>>,
    sender: mpsc::Sender<io::Result<Bytes>>,
) -> End
where
    F: FnOnce(
        &mut dyn Write,
        &mut dyn FnMut() -> anyhow::Result<()>,
    ) -> anyhow::Result<SitemapVerifiedBodyCopy>,
{
    let mut ready = Some(ready);
    let mut sender = Some(sender);
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> anyhow::Result<()> {
            slot.checkpoint()?;
            let mut file = create(&registry, &slot, &data)?;
            let mut checkpoint = || slot.checkpoint();
            let manifest = {
                let mut sink = CappedSpool {
                    file: &mut file,
                    bytes: 0,
                };
                operation(&mut sink, &mut checkpoint)?
            };
            anyhow::ensure!(
                manifest.body.bytes <= BODY_CAP && file.metadata()?.len() == manifest.body.bytes,
                "Verified spool length/cap differs"
            );
            file.sync_all()?;
            file.seek(SeekFrom::Start(0))?;
            let mut hash = Sha256::new();
            let mut count = 0u64;
            let mut buffer = [0u8; 65_536];
            loop {
                slot.checkpoint()?;
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                count += n as u64;
                anyhow::ensure!(
                    count <= manifest.body.bytes && count <= BODY_CAP,
                    "Spool exceeded byte cap"
                );
                hash.update(&buffer[..n]);
            }
            anyhow::ensure!(
                count == manifest.body.bytes
                    && format!("{:x}", hash.finalize()) == manifest.body.sha256,
                "Verified spool full hash/EOF differs"
            );
            file.seek(SeekFrom::Start(0))?;
            slot.checkpoint()?;
            final_check()?;
            slot.checkpoint()?;
            let expected = manifest.body.clone();
            if ready
                .take()
                .expect("Ready sender is owned once")
                .send(Ok(manifest))
                .is_err()
            {
                slot.cancel();
                return Ok(());
            }
            let mut sent = 0u64;
            let mut pump_hash = Sha256::new();
            loop {
                slot.checkpoint()?;
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                sent += n as u64;
                anyhow::ensure!(sent <= expected.bytes, "Spool changed during response");
                pump_hash.update(&buffer[..n]);
                let mut chunk = Ok(Bytes::copy_from_slice(&buffer[..n]));
                loop {
                    slot.checkpoint()?;
                    match sender
                        .as_ref()
                        .expect("Sender held during pump")
                        .try_send(chunk)
                    {
                        Ok(()) => break,
                        Err(mpsc::error::TrySendError::Full(value)) => {
                            chunk = value;
                            let guard = lock(&slot.wait_lock);
                            drop(
                                slot.wait
                                    .wait_timeout(guard, Duration::from_millis(10))
                                    .unwrap_or_else(std::sync::PoisonError::into_inner),
                            );
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            slot.cancel();
                            anyhow::bail!("Source body consumer disconnected")
                        }
                    }
                }
            }
            anyhow::ensure!(
                sent == expected.bytes && format!("{:x}", pump_hash.finalize()) == expected.sha256,
                "Spool response hash/EOF changed"
            );
            Ok(())
        }));
    let (panicked, error) = match result {
        Ok(Ok(())) => (false, None),
        Ok(Err(e)) => (false, Some(e.to_string())),
        Err(_) => (true, Some("Source body worker panicked".to_string())),
    };
    let had_headers = ready.is_none();
    if had_headers && error.is_some() {
        slot.cancel();
    }
    if let Some(ready) = ready.take() {
        let _ = ready.send(Err(error
            .clone()
            .unwrap_or_else(|| "Source body preparation failed".into())));
    }
    drop(sender.take());
    if had_headers {
        slot.wait_consumer();
    }
    let cleaned = cleanup(&slot);
    let failure = if let Err(e) = cleaned {
        Some(format!("Source body cleanup failed: {e:#}"))
    } else if panicked {
        error
    } else {
        None
    };
    End { failure }
}

#[cfg(test)]
#[path = "source_body_stream_tests.rs"]
mod tests;
