//! Independent crawl admission control and durable checkpoint evidence.
use crate::{CrawlConfig, CrawlReport};
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, Ordering},
};
use tokio::sync::Notify;

const FINISHED: u8 = 0x80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlRequest {
    Running,
    PauseRequested,
    CancelRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlOutcome {
    Completed,
    Paused,
    Cancelled,
    Limited,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrawlRunState {
    pub generation: u64,
    pub started_at: String,
    pub finished_at: Option<String>,
    /// None means that this invocation has no committed terminal checkpoint.
    pub outcome: Option<CrawlOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrawlCheckpoint {
    pub generation: u64,
    pub revision: u64,
    pub outcome: CrawlOutcome,
    pub created_at: String,
    pub report_sha256: String,
    pub config_sha256: String,
    pub completed: usize,
    /// Pending page documents; discovery units are reported separately.
    pub pending: usize,
    pub discovery_pending: usize,
    pub discovery_complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_ledger: Option<crate::PolicyLedgerSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_ledger: Option<crate::ResourceLedgerSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sitemap_ledger: Option<crate::SitemapLedgerSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlStoreInspection {
    pub config: Option<CrawlConfig>,
    pub run_state: Option<CrawlRunState>,
    pub checkpoint: Option<CrawlCheckpoint>,
    pub completed: usize,
    pub pending: usize,
    pub discovery_pending: usize,
    pub discovery_complete: bool,
    /// Controlled-writer consistency: generation, hashes and row counts agree.
    /// This is not a full integrity scan of arbitrary SQLite row contents.
    pub checkpoint_valid: bool,
    #[serde(default)]
    pub operation: Option<crate::SelectedRecrawlOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_ledger: Option<crate::PolicyLedgerSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_ledger: Option<crate::ResourceLedgerSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sitemap_ledger: Option<crate::SitemapLedgerSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlledCrawlResult {
    pub report: CrawlReport,
    pub outcome: CrawlOutcome,
    /// Absent for an intentionally in-memory crawl.
    pub checkpoint: Option<CrawlCheckpoint>,
    /// Pending sitemap/robots discovery units, separate from page documents.
    pub discovery_pending: usize,
}

struct ControlInner {
    request: AtomicU8,
    changed: Notify,
    legacy_cancel: Option<Arc<AtomicBool>>,
}

/// A monotonic control handle for one crawl invocation. Pause stops admission
/// and drains existing work. Cancel retains the legacy abort behavior.
#[derive(Clone)]
pub struct CrawlControl {
    inner: Arc<ControlInner>,
}

impl Default for CrawlControl {
    fn default() -> Self {
        Self::new()
    }
}

impl CrawlControl {
    pub fn new() -> Self {
        Self::build(None)
    }

    fn build(legacy_cancel: Option<Arc<AtomicBool>>) -> Self {
        Self {
            inner: Arc::new(ControlInner {
                request: AtomicU8::new(0),
                changed: Notify::new(),
                legacy_cancel,
            }),
        }
    }

    pub(crate) fn from_cancel(cancel: Arc<AtomicBool>) -> Self {
        Self::build(Some(cancel))
    }

    /// True if pause is accepted or already requested. False after cancellation
    /// or terminal finalization. Acceptance does not mean that draining finished.
    pub fn request_pause(&self) -> bool {
        self.request_at_least(1)
    }

    /// True if cancellation is accepted or already requested. Completed
    /// checkpoints cannot be changed by a late request.
    pub fn request_cancel(&self) -> bool {
        self.request_at_least(2)
    }

    fn request_at_least(&self, requested: u8) -> bool {
        let mut current = self.inner.request.load(Ordering::Acquire);
        loop {
            if current & FINISHED != 0 || current > requested {
                return false;
            }
            if current == requested {
                return true;
            }
            match self.inner.request.compare_exchange_weak(
                current,
                requested,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.inner.changed.notify_waiters();
                    return true;
                }
                Err(next) => current = next,
            }
        }
    }

    pub fn request(&self) -> CrawlRequest {
        if self
            .inner
            .legacy_cancel
            .as_ref()
            .is_some_and(|cancel| cancel.load(Ordering::Relaxed))
        {
            self.request_cancel();
        }
        decode(self.inner.request.load(Ordering::Acquire))
    }

    pub fn is_finished(&self) -> bool {
        self.inner.request.load(Ordering::Acquire) & FINISHED != 0
    }

    pub(crate) fn finish(&self) -> CrawlRequest {
        // Read legacy cancellation before atomically freezing the final outcome.
        self.request();
        decode(self.inner.request.fetch_or(FINISHED, Ordering::AcqRel))
    }

    pub(crate) async fn changed(&self) {
        // The bounded poll also observes compatibility AtomicBool cancellation.
        // State is always rechecked by callers, so a missed notification is safe.
        tokio::select! {
            _ = self.inner.changed.notified() => {},
            _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {},
        }
    }

    pub(crate) async fn cancelled(&self) {
        while self.request() != CrawlRequest::CancelRequested {
            self.changed().await;
        }
    }
}

fn decode(request: u8) -> CrawlRequest {
    match request & !FINISHED {
        1 => CrawlRequest::PauseRequested,
        2 => CrawlRequest::CancelRequested,
        _ => CrawlRequest::Running,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_monotonic_idempotent_and_frozen_at_finish() {
        let control = CrawlControl::new();
        let other = control.clone();
        assert_eq!(control.request(), CrawlRequest::Running);
        assert!(control.request_pause());
        assert!(control.request_pause());
        assert_eq!(other.request(), CrawlRequest::PauseRequested);
        assert!(other.request_cancel());
        assert!(!control.request_pause());
        assert!(control.request_cancel());
        assert_eq!(control.finish(), CrawlRequest::CancelRequested);
        assert!(other.is_finished());
        assert!(!control.request_cancel());
        assert!(!other.request_pause());
    }

    #[test]
    fn late_pause_cannot_change_a_completed_invocation() {
        let control = CrawlControl::new();
        assert_eq!(control.finish(), CrawlRequest::Running);
        assert!(!control.request_pause());
        assert!(!control.request_cancel());
        assert_eq!(control.request(), CrawlRequest::Running);
    }

    #[tokio::test]
    async fn legacy_cancel_is_observed_and_pause_does_not_abort_waiters() {
        let cancel = Arc::new(AtomicBool::new(false));
        let control = CrawlControl::from_cancel(cancel.clone());
        assert!(control.request_pause());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), control.cancelled())
                .await
                .is_err()
        );
        cancel.store(true, Ordering::Relaxed);
        tokio::time::timeout(std::time::Duration::from_secs(1), control.cancelled())
            .await
            .unwrap();
        assert_eq!(control.finish(), CrawlRequest::CancelRequested);
    }
}
