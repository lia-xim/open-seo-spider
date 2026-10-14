//! Independent full-stored predicates over one actual verified read transaction.
//! No session constructor, SQL handle, fabricated proof or transport belongs here.
use super::CrawlStore;
use super::sitemap_read_context::{PreparedSitemapRead, with_prepared_sitemap_read};
use crate::SitemapReadControl;
use crate::sitemap_findings::predicate::{assess_saved_page, assess_source};
use crate::sitemap_findings::source::{PreparedSourceFindings, prepare_source_findings};
use crate::sitemap_findings::*;
use crate::sitemap_membership_analysis::{
    validate_saved_page_candidate_facts, visit_saved_page_candidates,
};
use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{io, ops::ControlFlow};

/// Borrowed capability for one fixed query and one actual SQLite read snapshot.
/// Context clones are ordinary output DTOs and cannot construct this capability.
/// Do not reenter the same Store in a callback: its connection lock is held.
pub struct SitemapVerifiedFindings<'read> {
    prepared: &'read PreparedSitemapRead<'read>,
    sources: Option<&'read PreparedSourceFindings<'read, 'read>>,
    context: &'read SitemapFindingsContext,
    selection: &'read SitemapFindingSelection,
    control: &'read SitemapReadControl,
}

struct FindingCandidate {
    position: SitemapFindingPosition,
    evidence: SitemapFindingEvidence,
}

impl SitemapVerifiedFindings<'_> {
    pub fn context(&self) -> &SitemapFindingsContext {
        self.context
    }

    /// Visit every selected stored result. This method has no page work-window
    /// limit. Controls and the already verified dataset bounds still apply.
    /// Repeated visits retain the same query, facts, context and transaction.
    pub fn visit_all(
        &self,
        mut visitor: impl FnMut(&SitemapFindingRow) -> Result<()>,
    ) -> Result<()> {
        self.visit_candidates(None, |candidate| {
            let row = self.assess(candidate)?;
            if self.accepts(&row) {
                visitor(&row)?;
            }
            Ok(ControlFlow::Continue(()))
        })?;
        self.control.checkpoint()
    }

    fn accepts(&self, row: &SitemapFindingRow) -> bool {
        let identity = match &row.evidence {
            SitemapFindingEvidence::SavedPage { page } => &page.finding.observation.requested_url,
            SitemapFindingEvidence::SourceAttempt { source } => source.original_task_url(),
        };
        self.selection.accepts_identity(identity)
            && self.selection.assessment.accepts(row.assessment.verdict)
    }

    fn assess(&self, candidate: FindingCandidate) -> Result<SitemapFindingRow> {
        self.control.checkpoint()?;
        let assessment = match &candidate.evidence {
            SitemapFindingEvidence::SavedPage { page } => {
                assess_saved_page(self.selection.filter, page, self.prepared.coverage)?
            }
            SitemapFindingEvidence::SourceAttempt { source } => {
                assess_source(self.selection.filter, source)?
            }
        };
        Ok(SitemapFindingRow {
            position: candidate.position,
            assessment,
            evidence: candidate.evidence,
        })
    }

    /// Internal traversal includes candidates excluded by the identity/verdict
    /// query. Thus a sparse work window cannot silently become a false EOF.
    fn visit_candidates(
        &self,
        after: Option<&SitemapFindingPosition>,
        mut visitor: impl FnMut(FindingCandidate) -> Result<ControlFlow<()>>,
    ) -> Result<bool> {
        self.control.checkpoint()?;
        if self.context.availability == SitemapFindingsAvailability::Unavailable {
            return Err(SitemapFindingsError::Unavailable.into());
        }
        if let Some(after) = after {
            after.validate(self.selection.filter.domain())?;
        }
        let result = match self.selection.filter.domain() {
            SitemapFindingDomain::SavedPage => visit_saved_page_candidates(
                self.prepared.analysis,
                self.prepared.raw,
                self.prepared.coverage,
                self.control,
                after,
                |position, page| {
                    self.control.checkpoint()?;
                    visitor(FindingCandidate {
                        position,
                        evidence: SitemapFindingEvidence::SavedPage {
                            page: Box::new(page),
                        },
                    })
                },
            ),
            SitemapFindingDomain::SourceAttempt => self
                .sources
                .ok_or(SitemapFindingsError::Unavailable)?
                .visit_after(after, |position, source| {
                    self.control.checkpoint()?;
                    visitor(FindingCandidate {
                        position,
                        evidence: SitemapFindingEvidence::SourceAttempt {
                            source: Box::new(source),
                        },
                    })
                }),
        };
        self.control.map_result(result)
    }
}

impl CrawlStore {
    /// Fixed-query, repeatable, complete-stored visits in the same verified TX.
    /// The expected context is checked before the callback or any cursor lookup.
    /// Only owned T returns after the proofs and the read transaction have ended.
    pub fn with_verified_sitemap_findings<T>(
        &self,
        selection: &SitemapFindingSelection,
        expected_context_sha256: Option<&str>,
        control: &SitemapReadControl,
        callback: impl for<'read> FnOnce(&SitemapVerifiedFindings<'read>) -> Result<T>,
    ) -> Result<T> {
        selection.validate()?;
        if expected_context_sha256.is_some_and(|value| !valid_hash(value)) {
            return Err(SitemapFindingsError::InvalidContinuation.into());
        }
        // Canonical typed query serialization is bounded incrementally. A large
        // escaped legacy task selector must not allocate a whole JSON buffer.
        let selection_sha256 = bounded_json_sha256(
            selection,
            MAX_SITEMAP_FINDINGS_SELECTION_BYTES,
            SitemapFindingsError::SelectionByteBudget,
            control,
        )?;
        with_prepared_sitemap_read(self, None, control, |prepared| {
            if selection.filter.domain() == SitemapFindingDomain::SavedPage
                && prepared.header.analysis_context.available
            {
                // New typed qualification facts are validated over the whole
                // actual report, including rows excluded by after/selection or
                // an early visitor break, before any user callback can run.
                validate_saved_page_candidate_facts(prepared.analysis, control)?;
            }
            let sources = if selection.filter.domain() == SitemapFindingDomain::SourceAttempt {
                Some(prepare_source_findings(
                    prepared.connection,
                    prepared.raw,
                    prepared.source,
                    prepared.header.generation,
                    prepared.header.crawl_config_sha256.as_deref(),
                    control,
                )?)
            } else {
                None
            };
            let context = findings_context(
                selection,
                selection_sha256,
                &prepared,
                sources.as_ref(),
                control,
            )?;
            if expected_context_sha256.is_some_and(|expected| expected != context.context_sha256) {
                return Err(SitemapFindingsError::StaleContext.into());
            }
            let session = SitemapVerifiedFindings {
                prepared: &prepared,
                sources: sources.as_ref(),
                context: &context,
                selection,
                control,
            };
            callback(&session)
        })
    }

    /// Predicates and exact original-identity selection precede visible limits.
    /// A work-window stop carries actual candidate progress even with zero rows.
    pub fn sitemap_findings_page(
        &self,
        request: &SitemapFindingsRequest,
        control: &SitemapReadControl,
    ) -> Result<SitemapFindingsPage> {
        request.validate()?;
        self.with_verified_sitemap_findings(
            &request.selection,
            request.expected_context_sha256.as_deref(),
            control,
            |session| {
                let context = session.context().clone();
                if context.availability == SitemapFindingsAvailability::Unavailable {
                    if request.after.is_some() {
                        return Err(SitemapFindingsError::InvalidContinuation.into());
                    }
                    let page = SitemapFindingsPage {
                        context,
                        rows: None,
                        scan: SitemapFindingsScan {
                            evaluated_candidates: 0,
                            exhausted: false,
                            stop: SitemapFindingsStop::Unavailable,
                            more_matches: None,
                        },
                        continuation: None,
                    };
                    bounded_page(&page, control)?;
                    return Ok(page);
                }
                let mut rows = Vec::with_capacity(request.limit);
                let mut row_bytes = 2usize; // JSON array delimiters.
                let mut evaluated_candidates = 0u64;
                let mut last_evaluated = None;
                let mut last_returned = None;
                let mut stop = None;
                let exhausted = session.visit_candidates(request.after.as_ref(), |candidate| {
                    if evaluated_candidates >= u64::try_from(request.candidate_limit)? {
                        stop = Some(SitemapFindingsStop::CandidateBudget);
                        return Ok(ControlFlow::Break(()));
                    }
                    evaluated_candidates += 1;
                    last_evaluated = Some(candidate.position.clone());
                    let row = session.assess(candidate)?;
                    if session.accepts(&row) {
                        if rows.len() == request.limit {
                            // This is the first selected lookahead, not a row
                            // the caller has received. Resume after its actual
                            // last returned match, never after this lookahead.
                            stop = Some(SitemapFindingsStop::MoreMatches);
                            return Ok(ControlFlow::Break(()));
                        }
                        let bytes = bounded_json_bytes(
                            &row,
                            MAX_SITEMAP_FINDINGS_PAGE_BYTES,
                            SitemapFindingsError::ResponseByteBudget,
                            control,
                        )?;
                        row_bytes = row_bytes
                            .checked_add(bytes)
                            .and_then(|v| v.checked_add(usize::from(!rows.is_empty())))
                            .filter(|v| *v <= MAX_SITEMAP_FINDINGS_PAGE_BYTES)
                            .ok_or(SitemapFindingsError::ResponseByteBudget)?;
                        last_returned = Some(row.position.clone());
                        rows.push(row);
                    }
                    Ok(ControlFlow::Continue(()))
                })?;
                let stop = if exhausted {
                    SitemapFindingsStop::Exhausted
                } else {
                    stop.ok_or_else(|| {
                        anyhow::anyhow!("Findings traversal stopped without progress reason")
                    })?
                };
                let continuation_position = match stop {
                    SitemapFindingsStop::MoreMatches => last_returned,
                    SitemapFindingsStop::CandidateBudget => last_evaluated,
                    _ => None,
                };
                let continuation = continuation_position.map(|after| SitemapFindingsContinuation {
                    context_sha256: context.context_sha256.clone(),
                    after,
                    reason: stop,
                });
                let more_matches = match stop {
                    SitemapFindingsStop::Exhausted => Some(false),
                    SitemapFindingsStop::MoreMatches => Some(true),
                    _ => None,
                };
                let page = SitemapFindingsPage {
                    context,
                    rows: Some(rows),
                    scan: SitemapFindingsScan {
                        evaluated_candidates,
                        exhausted,
                        stop,
                        more_matches,
                    },
                    continuation,
                };
                bounded_page(&page, control)?;
                Ok(page)
            },
        )
    }
}

fn findings_context(
    selection: &SitemapFindingSelection,
    selection_sha256: String,
    prepared: &PreparedSitemapRead<'_>,
    sources: Option<&PreparedSourceFindings<'_, '_>>,
    control: &SitemapReadControl,
) -> Result<SitemapFindingsContext> {
    let source_scope = sources.map(|source| source.scope().clone());
    let mut reasons = Vec::new();
    let availability = match selection.filter.domain() {
        SitemapFindingDomain::SavedPage => {
            if !prepared.header.analysis_context.available {
                reasons.push(SitemapFindingReason::SavedPageAnalysisUnavailable);
                if prepared.header.ledger.is_none() {
                    reasons.push(SitemapFindingReason::CatalogUnavailable);
                }
                SitemapFindingsAvailability::Unavailable
            } else {
                SitemapFindingsAvailability::Available
            }
        }
        SitemapFindingDomain::SourceAttempt => {
            let scope = source_scope
                .as_ref()
                .ok_or(SitemapFindingsError::Unavailable)?;
            if scope.unknown_history {
                reasons.push(SitemapFindingReason::SourceHistoryUnknown);
            }
            if scope.omission.is_some() {
                reasons.push(SitemapFindingReason::SourceCaptureOmission);
            }
            if !scope.current_task_inventory_available {
                reasons.push(SitemapFindingReason::SourceTaskInventoryUnavailable);
            }
            if scope.unknown_history
                || scope.omission.is_some()
                || !scope.current_task_inventory_available
            {
                SitemapFindingsAvailability::AvailableWithUnknownHistory
            } else {
                SitemapFindingsAvailability::Available
            }
        }
    };
    #[derive(Serialize)]
    struct Binding<'a> {
        engine: &'static str,
        selection_sha256: &'a str,
        availability: SitemapFindingsAvailability,
        reasons: &'a [SitemapFindingReason],
        basis: &'a super::SitemapVerifiedReadHeader,
        source_scope: &'a Option<SitemapSourceFindingsScope>,
    }
    let context_sha256 = bounded_json_sha256(
        &Binding {
            engine: SITEMAP_FINDINGS_ENGINE,
            selection_sha256: &selection_sha256,
            availability,
            reasons: &reasons,
            basis: &prepared.header,
            source_scope: &source_scope,
        },
        MAX_SITEMAP_FINDINGS_SELECTION_BYTES,
        SitemapFindingsError::SelectionByteBudget,
        control,
    )?;
    Ok(SitemapFindingsContext {
        engine: SITEMAP_FINDINGS_ENGINE.into(),
        selection_sha256,
        context_sha256,
        availability,
        reasons,
        basis: prepared.header.clone(),
        source_scope,
    })
}

/// Counting/hash sink retains no serialized query, report, task list or rows.
/// serde may erase an I/O source, so the actual budget/stop state is restored.
struct JsonSink<'a> {
    bytes: usize,
    max: usize,
    exceeded: bool,
    hash: Option<Sha256>,
    control: &'a SitemapReadControl,
}
impl io::Write for JsonSink<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.control.checkpoint().map_err(io::Error::other)?;
        let next = self.bytes.checked_add(bytes.len());
        if next.is_none_or(|next| next > self.max) {
            self.exceeded = true;
            return Err(io::Error::other(
                "Sitemap findings JSON byte budget exceeded",
            ));
        }
        self.bytes = next.expect("checked byte count");
        if let Some(hash) = self.hash.as_mut() {
            for chunk in bytes.chunks(65_536) {
                self.control.checkpoint().map_err(io::Error::other)?;
                hash.update(chunk);
            }
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.control.checkpoint().map_err(io::Error::other)
    }
}
fn serialize_bounded(
    value: &impl Serialize,
    max: usize,
    error: SitemapFindingsError,
    control: &SitemapReadControl,
    hash: bool,
) -> Result<(usize, Option<Sha256>)> {
    let mut sink = JsonSink {
        bytes: 0,
        max,
        exceeded: false,
        hash: hash.then(Sha256::new),
        control,
    };
    let result = serde_json::to_writer(&mut sink, value);
    control.checkpoint()?;
    if sink.exceeded {
        return Err(error.into());
    }
    result?;
    Ok((sink.bytes, sink.hash))
}
fn bounded_json_sha256(
    value: &impl Serialize,
    max: usize,
    error: SitemapFindingsError,
    control: &SitemapReadControl,
) -> Result<String> {
    let (_, hash) = serialize_bounded(value, max, error, control, true)?;
    Ok(format!(
        "{:x}",
        hash.expect("hash sink requested").finalize()
    ))
}
fn bounded_json_bytes(
    value: &impl Serialize,
    max: usize,
    error: SitemapFindingsError,
    control: &SitemapReadControl,
) -> Result<usize> {
    Ok(serialize_bounded(value, max, error, control, false)?.0)
}
fn bounded_page(page: &SitemapFindingsPage, control: &SitemapReadControl) -> Result<()> {
    bounded_json_bytes(
        page,
        MAX_SITEMAP_FINDINGS_PAGE_BYTES,
        SitemapFindingsError::ResponseByteBudget,
        control,
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "sitemap_findings_read_tests.rs"]
mod tests;
