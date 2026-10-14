//! Transport-neutral cooperative stop control for owned sitemap read sessions.
//! An interrupt attachment is an actual SQLite handle, not a snapshot proof.
use anyhow::Result;
use rusqlite::{Connection, InterruptHandle};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    time::Instant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SitemapReadControlError {
    Cancelled,
    DeadlineExceeded,
}
impl fmt::Display for SitemapReadControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Cancelled => "Sitemap read cancelled",
            Self::DeadlineExceeded => "Sitemap read deadline exceeded",
        })
    }
}
impl Error for SitemapReadControlError {}

#[derive(Default)]
struct State {
    deadline: Option<Instant>,
    next_id: u64,
    attachments: BTreeMap<u64, InterruptHandle>,
}
#[derive(Default)]
struct Inner {
    stop: AtomicU8,
    has_deadline: AtomicBool,
    state: Mutex<State>,
}

/// Clones share stop/deadline state. Stops are sticky: the first observed stop
/// wins; changing a deadline does not resume a cancelled/expired operation.
/// Deadlines are checked at cooperative checkpoints. A caller that needs to
/// interrupt a currently blocked SQL step must call `expire`/`cancel` from its
/// owned deadline/disconnect watcher; this type starts no timer/thread.
#[derive(Clone, Default)]
pub struct SitemapReadControl {
    inner: Arc<Inner>,
}
impl SitemapReadControl {
    pub fn with_deadline(deadline: Instant) -> Self {
        let control = Self::default();
        control.set_deadline(Some(deadline));
        control
    }
    fn state(&self) -> MutexGuard<'_, State> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub fn cancel(&self) {
        self.stop(1);
    }
    pub fn expire(&self) {
        self.stop(2);
    }
    fn stop(&self, value: u8) {
        let _ = self
            .inner
            .stop
            .compare_exchange(0, value, Ordering::AcqRel, Ordering::Acquire);
        // Repeat notifications also interrupt a query that started just after a
        // previous notification; SQLite interrupt is not a persistent token.
        for handle in self.state().attachments.values() {
            handle.interrupt();
        }
    }
    pub fn set_deadline(&self, deadline: Option<Instant>) {
        {
            let mut state = self.state();
            state.deadline = deadline;
            self.inner
                .has_deadline
                .store(deadline.is_some(), Ordering::Release);
        }
        if deadline.is_some_and(|value| Instant::now() >= value) {
            self.expire();
        }
    }
    pub fn checkpoint(&self) -> Result<()> {
        if self.inner.stop.load(Ordering::Acquire) == 0
            && self.inner.has_deadline.load(Ordering::Acquire)
        {
            let deadline = self.state().deadline;
            if deadline.is_some_and(|value| Instant::now() >= value) {
                self.expire();
            }
        }
        match self.inner.stop.load(Ordering::Acquire) {
            0 => Ok(()),
            1 => Err(SitemapReadControlError::Cancelled.into()),
            _ => Err(SitemapReadControlError::DeadlineExceeded.into()),
        }
    }

    /// Owned handle is safe after Connection close (rusqlite clears its shared
    /// pointer). No Connection borrow blocks starting/closing the caller's TX.
    pub(crate) fn bind_connection(
        &self,
        connection: &Connection,
    ) -> Result<SitemapReadConnectionGuard> {
        self.checkpoint()?;
        let handle = connection.get_interrupt_handle();
        let id = {
            let mut state = self.state();
            state.next_id = state
                .next_id
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("Sitemap read attachment identity exhausted"))?;
            let id = state.next_id;
            state.attachments.insert(id, handle);
            id
        };
        let guard = SitemapReadConnectionGuard {
            control: self.clone(),
            id,
        };
        self.checkpoint()?;
        Ok(guard)
    }

    pub(crate) fn run<T>(&self, operation: impl FnOnce() -> Result<T>) -> Result<T> {
        self.checkpoint()?;
        self.map_result(operation())
    }

    pub(crate) fn sha256(&self, bytes: &[u8]) -> Result<String> {
        use sha2::Digest;
        let mut hash = sha2::Sha256::new();
        self.checkpoint()?;
        for chunk in bytes.chunks(65_536) {
            self.checkpoint()?;
            hash.update(chunk);
        }
        self.checkpoint()?;
        Ok(format!("{:x}", hash.finalize()))
    }

    /// Preserve unrelated corruption/I/O failures. Restore a typed stop across
    /// actual SQLite INTERRUPT or serde custom-error erasure from our checkpoints.
    pub(crate) fn map_result<T>(&self, result: Result<T>) -> Result<T> {
        match result {
            Ok(value) => {
                self.checkpoint()?;
                Ok(value)
            }
            Err(error) => {
                let interrupted=error.chain().any(|cause|{
                    cause.downcast_ref::<SitemapReadControlError>().is_some()
                    || matches!(cause.downcast_ref::<rusqlite::Error>(),Some(rusqlite::Error::SqliteFailure(code,_)) if code.code==rusqlite::ErrorCode::OperationInterrupted)
                    || cause.to_string().starts_with("Sitemap read cancelled")
                    || cause.to_string().starts_with("Sitemap read deadline exceeded")
                });
                if interrupted {
                    self.checkpoint()?;
                }
                Err(error)
            }
        }
    }
}

pub(crate) struct SitemapReadConnectionGuard {
    control: SitemapReadControl,
    id: u64,
}
impl Drop for SitemapReadConnectionGuard {
    fn drop(&mut self) {
        self.control.state().attachments.remove(&self.id);
    }
}

#[cfg(test)]
#[path = "sitemap_read_control_tests.rs"]
mod tests;
