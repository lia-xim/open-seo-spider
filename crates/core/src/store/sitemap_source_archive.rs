//! Core-only archive foundation. A/B bodies and wire/report contracts unchanged.
use super::*;
use crate::sitemap_source_archive::{self as archive, SourceBodyCandidate};

impl CrawlStore {
    pub(crate) fn record_sitemap_source_bodies(
        &self,
        document: &crate::SitemapSourceDocument,
        receipt: &crate::sitemap_source_store::SourceMeasurementReceipt,
        candidate: SourceBodyCandidate,
        generation: u64,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.source_archive_read_only,
            "Cannot archive through read-only Store"
        );
        let _writer = self
            .source_archive_writer
            .lock()
            .map_err(|_| anyhow::anyhow!("Archive writer lock poisoned"))?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let pre = connection.transaction()?;
        require_generation(&pre, generation)?;
        anyhow::ensure!(
            matches!(load_run_plan(&pre)?, RunPlan::FullCrawl),
            "Selected recrawl has no current body capture permission"
        );
        drop(crate::sitemap_source_store::verify_b_before_admission(
            &pre, receipt, document,
        )?);
        archive::load(&pre)?.context("Source body capture needs initialized opt-in archive")?;
        if receipt.measurement_id().is_none() {
            archive::note_measurement_unavailable(&pre)?;
            pre.commit()?;
            return Ok(());
        }
        pre.commit()?;
        // Cross-process/Store capability lease, not a stale PID-based takeover.
        let mut lease = archive::files::ArchiveWriter::acquire(&self.source_archive_path)?;
        let read = connection.transaction()?;
        require_generation(&read, generation)?;
        drop(crate::sitemap_source_store::verify_b_before_admission(
            &read, receipt, document,
        )?);
        let id = receipt
            .measurement_id()
            .context("Persisted A measurement vanished")?;
        let plan = archive::plan(&read, id, &candidate)?;
        let seal = plan.seal()?;
        read.commit()?;
        let mut published = lease.publish(&plan.bodies, &plan.base.config)?;
        let outcome = (|| -> anyhow::Result<()> {
            let tx = connection.transaction()?;
            require_generation(&tx, generation)?;
            anyhow::ensure!(
                matches!(load_run_plan(&tx)?, RunPlan::FullCrawl),
                "Archive became inherited before commit"
            );
            // Recheck actual current config/generation/measurement/document seal,
            // then all quotas in this actual A_body commit TX after disk I/O.
            drop(crate::sitemap_source_store::verify_b_before_admission(
                &tx, receipt, document,
            )?);
            let fresh = archive::plan(&tx, id, &candidate)?;
            anyhow::ensure!(
                fresh.seal()? == seal,
                "Archive capture/quota changed after publication"
            );
            archive::commit(&tx, &fresh)?;
            tx.commit()?;
            Ok(())
        })();
        // After a SQL commit attempt, retain sealed objects even on ambiguous I/O
        // failure: a published uncommitted object is an orphan, never availability.
        // Do not risk deleting an object referenced by an actually durable commit.
        published.retain();
        let released = lease.release();
        match (outcome, released) {
            (Err(error), Err(cleanup)) => {
                Err(error).context(format!("Archive lease release also failed: {cleanup:#}"))
            }
            (Err(error), _) => Err(error),
            (_, Err(error)) => Err(error),
            _ => Ok(()),
        }
    }

    /// Explicit cold Core inspection, not an App/API availability advertisement.
    /// Verifies real files without URL fetch, migration, directory creation or GC.
    pub fn sitemap_source_archive_inspection(
        &self,
    ) -> anyhow::Result<crate::SitemapSourceArchiveInspection> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let tx = connection.transaction()?;
        if matches!(load_run_plan(&tx)?, RunPlan::SelectedRecrawl(_)) {
            tx.commit()?;
            return Ok(crate::SitemapSourceArchiveInspection::InheritedArchiveNotCopied);
        }
        archive::require_storage_for_existing(&tx)?;
        crate::sitemap_source_store::load(&tx)?;
        let snapshot = archive::load(&tx)?;
        if let Some(snapshot) = snapshot {
            archive::files::verify_snapshot(&self.source_archive_path, &snapshot)?;
            tx.commit()?;
            Ok(crate::SitemapSourceArchiveInspection::VerifiedLocal(
                snapshot,
            ))
        } else {
            tx.commit()?;
            Ok(crate::SitemapSourceArchiveInspection::NotRecordedOrLegacyUnknown)
        }
    }
}
