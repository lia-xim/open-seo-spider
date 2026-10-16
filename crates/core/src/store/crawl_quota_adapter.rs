use super::*;
use crate::crawl_quota_ledger as ledger;
use crate::crawl_quotas::CompiledCrawlQuotas;

impl CrawlStore {
    /// Bounded transaction-consistent facts, no DDL and no website activity.
    /// Legacy absence is unavailable, never an invented zero-usage ledger.
    pub fn crawl_quota_summary(&self) -> anyhow::Result<Option<crate::CrawlQuotaSummary>> {
        let mut c = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let tx = c.transaction()?;
        let config: CrawlConfig = metadata(&tx, "config")?.context("Saved config missing")?;
        let result = ledger::summary(&tx, &config)?;
        if let Some(run) = metadata::<CrawlRunState>(&tx, "run_state")?
            && run.outcome.is_some()
        {
            ledger::verify_checkpoint(&tx, &config, run.generation)?;
        }
        tx.commit()?;
        Ok(result)
    }
    pub fn preview_crawl_quota_increases(
        &self,
        increases: &crate::CrawlQuotaIncreases,
    ) -> anyhow::Result<CrawlConfig> {
        let inspection = self.inspection()?;
        anyhow::ensure!(
            inspection.checkpoint_valid,
            "Quota increase requires an intact actual checkpoint"
        );
        let mut c = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let tx = c.transaction()?;
        let old: CrawlConfig = metadata(&tx, "config")?.context("Saved config missing")?;
        let result = ledger::prospective(&tx, &old, increases)?;
        tx.commit()?;
        Ok(result)
    }
    pub(crate) fn reserve_crawl_quota_target(
        &self,
        compiled: &CompiledCrawlQuotas,
        g: u64,
        task: &str,
        target: &url::Url,
        depth: usize,
    ) -> anyhow::Result<ledger::Admission> {
        let mut c = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let tx = c.transaction()?;
        let result = ledger::reserve(&tx, compiled, g, task, target, depth)?;
        tx.commit()?;
        Ok(result)
    }
    pub(crate) fn start_crawl_quota_attempt(&self, g: u64, id: i64) -> anyhow::Result<()> {
        let mut c = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let tx = c.transaction()?;
        ledger::start(&tx, g, id)?;
        tx.commit()?;
        Ok(())
    }
    pub(crate) fn observe_crawl_quota_response(
        &self,
        g: u64,
        id: i64,
        status: u16,
    ) -> anyhow::Result<()> {
        let mut c = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let tx = c.transaction()?;
        ledger::response(&tx, g, id, status)?;
        tx.commit()?;
        Ok(())
    }
    pub(crate) fn finish_crawl_quota_attempts(
        &self,
        g: u64,
        ids: &[i64],
        unknown: bool,
    ) -> anyhow::Result<()> {
        let mut c = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let tx = c.transaction()?;
        ledger::finish(&tx, g, ids, unknown)?;
        tx.commit()?;
        Ok(())
    }
    pub(crate) fn end_crawl_quota_generation(&self, g: u64) -> anyhow::Result<()> {
        let mut c = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let tx = c.transaction()?;
        let config: CrawlConfig = metadata(&tx, "config")?.context("Saved config missing")?;
        ledger::end_generation(&tx, g, &config)?;
        tx.commit()?;
        Ok(())
    }
}
