//! Retained task handles. Dropping a caller or a drain waiter never takes a join.
use futures::future::BoxFuture;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tokio::{
    sync::{Mutex as AsyncMutex, oneshot},
    task::JoinHandle,
};

#[derive(Clone, Default)]
pub struct TaskAdmission(Arc<Mutex<bool>>);
impl TaskAdmission {
    pub fn close(&self) {
        *lock(&self.0) = true;
    }
    pub fn is_closed(&self) -> bool {
        *lock(&self.0)
    }
}

#[derive(Debug, Clone)]
pub struct OwnedTaskError {
    kind: &'static str,
    message: String,
}
impl std::fmt::Display for OwnedTaskError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.kind, self.message)
    }
}
impl std::error::Error for OwnedTaskError {}
fn failure(kind: &'static str, message: impl ToString) -> OwnedTaskError {
    OwnedTaskError {
        kind,
        message: message.to_string(),
    }
}
pub type OwnedTask<T> = BoxFuture<'static, Result<T, OwnedTaskError>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedTaskStats {
    pub owner: &'static str,
    pub closing: bool,
    pub admitted: u64,
    pub started: u64,
    pub joined: u64,
    pub retained: usize,
    pub first_failure: Option<String>,
}
#[derive(Clone)]
pub struct TaskOwner(Arc<Inner>);
struct Inner {
    name: &'static str,
    admission: TaskAdmission,
    tasks: Mutex<Tasks>,
    started: AtomicU64,
    joined: AtomicU64,
}
#[derive(Default)]
struct Tasks {
    next: u64,
    jobs: BTreeMap<u64, Arc<dyn RetainedJob>>,
    first_failure: Option<String>,
}
trait RetainedJob: Send + Sync {
    fn completed(&self) -> bool;
    fn finish(self: Arc<Self>) -> BoxFuture<'static, ()>;
}
struct Worker<T> {
    join: Option<JoinHandle<Result<T, OwnedTaskError>>>,
    reply: Option<oneshot::Sender<Result<T, OwnedTaskError>>>,
}
struct Job<T> {
    owner: Weak<Inner>,
    worker: AsyncMutex<Worker<T>>,
    supervisor: AsyncMutex<Option<JoinHandle<()>>>,
    completed: AtomicBool,
}
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn note_failure(owner: &Weak<Inner>, message: String) {
    if let Some(owner) = owner.upgrade() {
        lock(&owner.tasks).first_failure.get_or_insert(message);
    }
}
async fn join_worker<T: Send + 'static>(job: &Job<T>) {
    let mut worker = job.worker.lock().await;
    if let Some(join) = worker.join.as_mut() {
        let result = match join.await {
            Ok(result) => result,
            Err(error) => {
                let failure = failure("owned_task_join", error);
                note_failure(&job.owner, failure.to_string());
                Err(failure)
            }
        };
        worker.join.take();
        if let Some(owner) = job.owner.upgrade() {
            owner.joined.fetch_add(1, Ordering::SeqCst);
        }
        if let Some(reply) = worker.reply.take() {
            let _ = reply.send(result);
        }
        job.completed.store(true, Ordering::Release);
    }
}
impl<T: Send + 'static> RetainedJob for Job<T> {
    fn completed(&self) -> bool {
        self.completed.load(Ordering::Acquire)
    }
    fn finish(self: Arc<Self>) -> BoxFuture<'static, ()> {
        Box::pin(async move {
            join_worker(&self).await;
            let mut supervisor = self.supervisor.lock().await;
            if let Some(handle) = supervisor.as_mut() {
                if let Err(error) = handle.await {
                    note_failure(
                        &self.owner,
                        format!("Owned supervisor failed to join: {error}"),
                    );
                }
                supervisor.take();
            }
        })
    }
}
impl TaskOwner {
    pub fn new(name: &'static str) -> Self {
        Self::with_admission(name, TaskAdmission::default())
    }
    pub fn with_admission(name: &'static str, admission: TaskAdmission) -> Self {
        Self(Arc::new(Inner {
            name,
            admission,
            tasks: Mutex::new(Tasks::default()),
            started: AtomicU64::new(0),
            joined: AtomicU64::new(0),
        }))
    }
    pub fn close_admission(&self) {
        self.0.admission.close();
    }
    pub fn record_failure(&self, message: impl ToString) {
        lock(&self.0.tasks)
            .first_failure
            .get_or_insert(message.to_string());
    }
    pub fn snapshot(&self) -> OwnedTaskStats {
        let closing = self.0.admission.is_closed();
        let tasks = lock(&self.0.tasks);
        OwnedTaskStats {
            owner: self.0.name,
            closing,
            admitted: tasks.next,
            started: self.0.started.load(Ordering::SeqCst),
            joined: self.0.joined.load(Ordering::SeqCst),
            retained: tasks.jobs.len(),
            first_failure: tasks.first_failure.clone(),
        }
    }
    fn register<T: Send + 'static>(
        &self,
        spawn: impl FnOnce(oneshot::Receiver<()>, Weak<Inner>) -> JoinHandle<Result<T, OwnedTaskError>>,
    ) -> Result<OwnedTask<T>, OwnedTaskError> {
        // Same admission gate is shared by request, blocking and crawl owners.
        let admission = lock(&self.0.admission.0);
        if *admission {
            return Err(failure("owned_task_closing", self.0.name));
        }
        let mut tasks = lock(&self.0.tasks);
        let id = tasks
            .next
            .checked_add(1)
            .ok_or_else(|| failure("owned_task_id", "Task identifier exhausted"))?;
        let (reply, receiver) = oneshot::channel();
        let (start, ready) = oneshot::channel();
        let weak = Arc::downgrade(&self.0);
        let worker = spawn(ready, weak.clone());
        let job = Arc::new(Job {
            owner: weak,
            worker: AsyncMutex::new(Worker {
                join: Some(worker),
                reply: Some(reply),
            }),
            supervisor: AsyncMutex::new(None),
            completed: AtomicBool::new(false),
        });
        // No await under the admission lock; the monitor is published before work starts.
        let monitored = job.clone();
        let supervisor = tokio::spawn(async move {
            join_worker(&monitored).await;
        });
        *job.supervisor
            .try_lock()
            .expect("New supervisor slot is exclusively owned") = Some(supervisor);
        let erased: Arc<dyn RetainedJob> = job;
        tasks.jobs.insert(id, erased.clone());
        tasks.next = id;
        let _ = start.send(());
        let owner = self.clone();
        Ok(Box::pin(async move {
            let result = receiver
                .await
                .map_err(|error| failure("owned_task_reply", error))?;
            erased.finish().await;
            lock(&owner.0.tasks).jobs.remove(&id);
            result
        }))
    }
    pub fn spawn_async<T, F>(&self, future: F) -> Result<OwnedTask<T>, OwnedTaskError>
    where
        T: Send + 'static,
        F: std::future::Future<Output = T> + Send + 'static,
    {
        self.register(move |ready, owner| {
            tokio::spawn(async move {
                ready
                    .await
                    .map_err(|error| failure("owned_task_start", error))?;
                if let Some(owner) = owner.upgrade() {
                    owner.started.fetch_add(1, Ordering::SeqCst);
                }
                Ok(future.await)
            })
        })
    }
    pub fn spawn_blocking<T, F>(&self, operation: F) -> Result<OwnedTask<T>, OwnedTaskError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.register(move |ready, owner| {
            tokio::task::spawn_blocking(move || {
                ready
                    .blocking_recv()
                    .map_err(|error| failure("owned_task_start", error))?;
                if let Some(owner) = owner.upgrade() {
                    owner.started.fetch_add(1, Ordering::SeqCst);
                }
                Ok(operation())
            })
        })
    }
    pub async fn run_async<T, F>(&self, future: F) -> Result<T, OwnedTaskError>
    where
        T: Send + 'static,
        F: std::future::Future<Output = T> + Send + 'static,
    {
        self.reap_completed().await;
        self.spawn_async(future)?.await
    }
    pub async fn run_blocking<T, F>(&self, operation: F) -> Result<T, OwnedTaskError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.reap_completed().await;
        self.spawn_blocking(operation)?.await
    }
    pub async fn reap_completed(&self) {
        let finished: Vec<_> = lock(&self.0.tasks)
            .jobs
            .iter()
            .filter(|(_, job)| job.completed())
            .map(|(id, job)| (*id, job.clone()))
            .collect();
        for (id, job) in finished {
            job.finish().await;
            lock(&self.0.tasks).jobs.remove(&id);
        }
    }
    pub async fn shutdown_and_drain(&self) -> Result<(), OwnedTaskError> {
        self.close_admission();
        let jobs: Vec<_> = lock(&self.0.tasks)
            .jobs
            .iter()
            .map(|(id, job)| (*id, job.clone()))
            .collect();
        for (id, job) in jobs {
            job.finish().await;
            lock(&self.0.tasks).jobs.remove(&id);
        }
        let stats = self.snapshot();
        if let Some(error) = stats.first_failure {
            return Err(failure("owned_task_drain", error));
        }
        if stats.retained != 0 || stats.admitted != stats.joined || stats.started != stats.joined {
            return Err(failure(
                "owned_task_drain",
                "Actual start/join accounting is incomplete",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "task_owner_tests.rs"]
mod tests;
