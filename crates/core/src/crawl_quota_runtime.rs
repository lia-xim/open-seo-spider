use crate::{CrawlStore, crawl_quota_ledger::Admission, crawl_quotas::CompiledCrawlQuotas};
use std::sync::Arc;
use url::Url;

/// One document future owns these attempt tokens. Cancellation drops the
/// retained response/future before the outer terminal checkpoint is admitted.
pub(crate) struct QuotaFetchSession {
    store: Arc<CrawlStore>,
    compiled: Arc<CompiledCrawlQuotas>,
    generation: u64,
    task: String,
    depth: usize,
    ids: Vec<i64>,
    pub deferred: bool,
    pub failure: Option<String>,
    finished: bool,
}
impl QuotaFetchSession {
    pub fn new(
        store: Arc<CrawlStore>,
        compiled: Arc<CompiledCrawlQuotas>,
        generation: u64,
        task: String,
        depth: usize,
    ) -> Self {
        Self {
            store,
            compiled,
            generation,
            task,
            depth,
            ids: Vec::new(),
            deferred: false,
            failure: None,
            finished: false,
        }
    }
    pub fn reserve(&mut self, url: &Url) -> anyhow::Result<Option<i64>> {
        match self.store.reserve_crawl_quota_target(
            &self.compiled,
            self.generation,
            &self.task,
            url,
            self.depth,
        )? {
            Admission::Reserved(id) => {
                self.ids.push(id);
                Ok(Some(id))
            }
            Admission::Deferred => {
                self.deferred = true;
                Ok(None)
            }
        }
    }
    pub fn start(&self, id: i64) -> anyhow::Result<()> {
        self.store.start_crawl_quota_attempt(self.generation, id)
    }
    pub fn response(&self, id: i64, status: u16) -> anyhow::Result<()> {
        self.store
            .observe_crawl_quota_response(self.generation, id, status)
    }
    pub fn finish(&mut self) -> anyhow::Result<()> {
        self.store
            .finish_crawl_quota_attempts(self.generation, &self.ids, false)?;
        self.finished = true;
        Ok(())
    }
}
impl Drop for QuotaFetchSession {
    fn drop(&mut self) {
        if !self.finished && !self.ids.is_empty() {
            // On failure, preserve conservative capacity. A failed cleanup TX
            // cannot manufacture a terminal PASS: checkpoint verifies no live
            // attempts, and terminal generation handling records unknowns.
            let _ = self
                .store
                .finish_crawl_quota_attempts(self.generation, &self.ids, true);
        }
    }
}
