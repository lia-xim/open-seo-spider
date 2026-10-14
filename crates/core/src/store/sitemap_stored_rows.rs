//! Callback-bound verified reads over every matching stored raw row.
//! No CSV, HTTP, transport buffer or public proof constructor belongs here.
use super::sitemap_read_context::{SitemapVerifiedReadHeader, with_prepared_sitemap_read};
use super::*;
use crate::sitemap_ledger::verified_read::{
    SitemapLedgerFilter, SitemapLedgerRowKind, VerifiedRawRead,
};
use crate::sitemap_read_control::SitemapReadControl;
use anyhow::Context;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SitemapStoredRowsFilter {
    pub kind: crate::SitemapAnalysisRowKind,
    pub source_observation_id: Option<String>,
    pub membership_url: Option<String>,
}
impl SitemapStoredRowsFilter {
    pub fn validate(&self) -> anyhow::Result<()> {
        crate::SitemapMembershipAnalysisRequest {
            kind: self.kind,
            source_observation_id: self.source_observation_id.clone(),
            membership_url: self.membership_url.clone(),
            ..Default::default()
        }
        .validate()
    }
    fn raw_filter(&self) -> SitemapLedgerFilter {
        use crate::SitemapAnalysisRowKind as K;
        SitemapLedgerFilter {
            kind: match self.kind {
                K::All => SitemapLedgerRowKind::All,
                K::Root => SitemapLedgerRowKind::Root,
                K::Document => SitemapLedgerRowKind::Document,
                K::Entry => SitemapLedgerRowKind::Entry,
                K::Location => SitemapLedgerRowKind::Location,
            },
            source_observation_id: self.source_observation_id.clone(),
            membership_url: self.membership_url.clone(),
        }
    }
}

/// A private-field capability borrowed only inside its actual Store callback.
/// Header clones are ordinary DTOs; they cannot create this session or SQL proof.
/// The filter and snapshot stay fixed across all visits. Do not reenter the same
/// CrawlStore from the callback; its actual connection mutex remains held.
pub struct SitemapVerifiedStoredRows<'read> {
    header: &'read SitemapVerifiedReadHeader,
    raw: Option<&'read VerifiedRawRead<'read>>,
    filter: &'read SitemapLedgerFilter,
    control: &'read SitemapReadControl,
}
impl SitemapVerifiedStoredRows<'_> {
    pub fn header(&self) -> &SitemapVerifiedReadHeader {
        self.header
    }

    /// All matching *stored* rows, in original sequence order, without a page
    /// limit or a whole-catalog Vec. This is not a remote completeness claim.
    pub fn visit_all_filtered(
        &self,
        mut visitor: impl FnMut(&crate::SitemapLedgerRow) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        self.control.checkpoint()?;
        let raw = self
            .raw
            .context("Verified stored sitemap catalog unavailable")?;
        let result = raw.visit_filtered(self.filter, |row| {
            self.control.checkpoint()?;
            visitor(row)?;
            self.control.checkpoint()
        });
        self.control.checkpoint()?;
        result
    }
}

impl CrawlStore {
    /// Verify the complete raw/source/config/coverage/report context once, then
    /// allow repeated fixed-filter visits in the same actual read transaction.
    /// Actual available headers must match Expected before the user callback.
    /// Historical RawNone remains unavailable: the caller can inspect its real
    /// header before reporting catalog-unavailable, and a row visit still fails.
    /// The owned result is returned only after the read transaction has closed.
    /// No borrowed row/session/SQL access can escape through this generic T.
    pub fn with_verified_sitemap_stored_rows<T>(
        &self,
        filter: &SitemapStoredRowsFilter,
        expected: Option<&SitemapReadWindowExpectedContext>,
        control: &SitemapReadControl,
        callback: impl for<'read> FnOnce(&SitemapVerifiedStoredRows<'read>) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        filter.validate()?;
        control.checkpoint()?;
        let filter = filter.raw_filter();
        with_prepared_sitemap_read(self, expected, control, |prepared| {
            if prepared.header.ledger.is_some()
                && expected.is_some_and(|expected| !expected.matches_header(&prepared.header))
            {
                return Err(SitemapReadWindowError::StaleContext.into());
            }
            let session = SitemapVerifiedStoredRows {
                header: &prepared.header,
                raw: prepared.raw,
                filter: &filter,
                control,
            };
            callback(&session)
        })
    }
}

#[cfg(test)]
#[path = "sitemap_stored_rows_tests.rs"]
mod tests;
