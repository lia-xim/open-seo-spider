//! Pure local findings decisions over already verified, captured evidence.
//! The caller owns same-TX/report/cohort authenticity and graph availability.
//! These functions neither fetch nor infer facts from aliases or reason text.
use super::{
    SitemapFindingAssessment, SitemapFindingDomain, SitemapFindingFilter as F,
    SitemapFindingPageCandidate, SitemapFindingReason as R, SitemapFindingVerdict as V,
    SitemapFindingsError, SitemapSourceFindingEvidence,
};
use crate::{
    ResourceAnalysisStatus, SitemapCoverage, SitemapSavedPageMembership, SitemapSourceLimitVerdict,
    SitemapTargetState,
};
use anyhow::{Result, ensure};

fn assessment(verdict: V, reasons: impl IntoIterator<Item = R>) -> SitemapFindingAssessment {
    SitemapFindingAssessment {
        verdict,
        reasons: reasons.into_iter().collect(),
    }
}

fn membership(
    candidate: &SitemapFindingPageCandidate,
    coverage: &SitemapCoverage,
) -> SitemapFindingAssessment {
    match candidate.finding.membership {
        SitemapSavedPageMembership::ListedOriginal => assessment(V::Match, [R::ListedOriginal]),
        SitemapSavedPageMembership::NotListedCurrentDeclaredSources
            if coverage.current_valid_original_location_negative_permitted =>
        {
            assessment(V::NotMatched, [R::NotListedCurrentDeclaredSources])
        }
        SitemapSavedPageMembership::NotListedHistoricalDeclaredSources
            if coverage.historical_valid_original_location_negative_permitted =>
        {
            assessment(V::NotMatched, [R::NotListedHistoricalDeclaredSources])
        }
        _ => assessment(
            V::Unknown,
            [R::MembershipUnknown, R::SourceCohortIncomplete],
        ),
    }
}

/// The target decision is about the original native requested observation.
/// A browser navigation/status never supplies an original HTTP obstruction.
fn non_indexability(candidate: &SitemapFindingPageCandidate) -> SitemapFindingAssessment {
    let page = &candidate.finding.observation;
    if page.redirect_chain_present
        || page.status.is_some_and(|s| (300..400).contains(&s))
        || (!page.final_url.is_empty() && page.final_url != page.requested_url)
    {
        return assessment(V::Match, [R::OriginalRedirect]);
    }
    if page.final_status.or(page.status).is_some_and(|s| s != 200) {
        return assessment(V::Match, [R::OriginalNon200]);
    }
    if page.blocked_by_robots {
        return if !candidate.robots_warning_field_observed {
            assessment(V::Unknown, [R::RobotsEvidenceUnobserved])
        } else if candidate.robots_warning.is_some() {
            assessment(V::Unknown, [R::RobotsPolicyUnavailable])
        } else {
            assessment(V::Match, [R::OriginalRobotsDisallow])
        };
    }
    if page.error.is_some() {
        return assessment(V::Unknown, [R::TargetFetchFailed]);
    }
    if page.truncated || page.renderer_error.is_some() {
        return assessment(V::Unknown, [R::TargetObservationIncomplete]);
    }
    if page.rendered_navigation_changed
        || (page.rendered
            && page
                .rendered_fetch_url
                .as_ref()
                .is_some_and(|url| url != &page.final_url))
        || page.state == SitemapTargetState::RenderedNavigationChanged
    {
        return assessment(V::Unknown, [R::RenderedNavigationChanged]);
    }
    if (page.rendered && page.rendered_status.is_some_and(|s| s != 200))
        || page.state == SitemapTargetState::RenderedNon200
    {
        return assessment(V::Unknown, [R::RenderedResponseNon200]);
    }
    if (page.rendering_expected && !page.rendered)
        || (page.rendered && (page.rendered_status.is_none() || page.rendered_fetch_url.is_none()))
    {
        return assessment(V::Unknown, [R::TargetObservationIncomplete]);
    }
    // A missing/partial PDF object can set legacy indexable=false without a
    // complete target conclusion. Never recover its meaning from human text.
    let media_type = candidate
        .content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim();
    let pdf = media_type.eq_ignore_ascii_case("application/pdf")
        || candidate.pdf_record_present
        || candidate.pdf_status.is_some();
    if pdf {
        if !candidate.pdf_field_observed
            || !candidate.pdf_record_present
            || candidate.pdf_status.is_none()
        {
            return assessment(V::Unknown, [R::PdfAnalysisUnobserved]);
        }
        if !matches!(
            candidate.pdf_status,
            Some(ResourceAnalysisStatus::Complete | ResourceAnalysisStatus::Disabled)
        ) {
            return assessment(V::Unknown, [R::PdfAnalysisIncomplete]);
        }
    }
    let unavailable = match page.state {
        SitemapTargetState::FetchFailed => Some(R::TargetFetchFailed),
        SitemapTargetState::IncompleteObservation => Some(R::TargetObservationIncomplete),
        SitemapTargetState::NoResponse => Some(R::TargetNoResponse),
        SitemapTargetState::Pending => Some(R::TargetPending),
        SitemapTargetState::NoStoredRequestedObservation => Some(R::TargetUnobserved),
        SitemapTargetState::InvalidOrMissingOriginalLocation => {
            Some(R::OriginalLocationUnavailable)
        }
        // These states must have a typed witness above; the state name alone
        // does not replace missing native status/identity or robots facts.
        SitemapTargetState::BlockedByRobots => Some(R::RobotsEvidenceUnobserved),
        SitemapTargetState::Redirected | SitemapTargetState::Non200 => {
            Some(R::TargetObservationIncomplete)
        }
        _ => None,
    };
    if let Some(reason) = unavailable {
        return assessment(V::Unknown, [reason]);
    }
    if page.final_status.or(page.status).is_none() {
        return assessment(V::Unknown, [R::TargetNoResponse]);
    }
    if page.final_url != page.requested_url {
        return assessment(V::Unknown, [R::TargetObservationIncomplete]);
    }
    if page.indexable && page.indexability_reasons.is_empty() {
        assessment(V::NotMatched, [R::CompleteIndexableObservation])
    } else if !page.indexable && !page.indexability_reasons.is_empty() {
        // Scoped directives/canonical/document-type policy is the saved local
        // audit's decision. This evaluator does not parse or merge its strings.
        assessment(V::Match, [R::ObservedNonIndexability])
    } else {
        assessment(V::Unknown, [R::IndexabilityReasonsUnavailable])
    }
}

pub(crate) fn assess_saved_page(
    filter: F,
    candidate: &SitemapFindingPageCandidate,
    coverage: &SitemapCoverage,
) -> Result<SitemapFindingAssessment> {
    ensure!(
        filter.domain() == SitemapFindingDomain::SavedPage,
        SitemapFindingsError::InvalidSelection("source filter needs source evidence")
    );
    let member = membership(candidate, coverage);
    Ok(match filter {
        F::UrlsInSitemap => member,
        F::UrlsNotInSitemap => assessment(
            match member.verdict {
                V::Match => V::NotMatched,
                V::NotMatched => V::Match,
                V::Unknown => V::Unknown,
            },
            member.reasons,
        ),
        F::OrphanUrls => {
            if candidate
                .finding
                .observed_inlinks
                .nonself_anchor_occurrences
                > 0
            {
                assessment(
                    V::NotMatched,
                    [R::ObservedNonselfAnchors, R::CapturedGraphOnly],
                )
            } else if member.verdict == V::NotMatched {
                member
            } else if member.verdict == V::Match {
                assessment(
                    V::Match,
                    [
                        R::ListedOriginal,
                        R::NoObservedNonselfAnchors,
                        R::CapturedGraphOnly,
                    ],
                )
            } else {
                assessment(
                    V::Unknown,
                    member.reasons.into_iter().chain([R::CapturedGraphOnly]),
                )
            }
        }
        F::NonIndexableUrlsInSitemap => {
            if member.verdict == V::NotMatched {
                // Known false conjunct: target incompleteness cannot make a
                // known not-listed page an unknown sitemap finding.
                member
            } else {
                let target = non_indexability(candidate);
                if target.verdict == V::NotMatched {
                    target
                } else {
                    let verdict = if member.verdict == V::Match && target.verdict == V::Match {
                        V::Match
                    } else {
                        V::Unknown
                    };
                    assessment(verdict, member.reasons.into_iter().chain(target.reasons))
                }
            }
        }
        F::UrlsInMultipleSitemaps => {
            if candidate.finding.original_source_document_count > 1 {
                assessment(V::Match, [R::ObservedMultipleOriginalSources])
            } else if coverage.current_valid_original_location_negative_permitted
                || coverage.historical_valid_original_location_negative_permitted
            {
                assessment(V::NotMatched, [R::CompleteCohortAtMostOneOriginalSource])
            } else {
                assessment(V::Unknown, [R::SourceCohortIncomplete])
            }
        }
        // The domain guard above rejects these before any page evaluation.
        F::XmlSitemapWithOver50kUrls | F::XmlSitemapOver50mb => unreachable!(),
    })
}

/// Source readers supply the already validated existing protocol projection.
/// Captured membership, association, and source completion are retained on the
/// evidence; they cannot raise/lower a protocol threshold verdict here.
pub(crate) fn assess_source(
    filter: F,
    evidence: &SitemapSourceFindingEvidence,
) -> Result<SitemapFindingAssessment> {
    ensure!(
        filter.domain() == SitemapFindingDomain::SourceAttempt,
        SitemapFindingsError::InvalidSelection("page filter needs saved page evidence")
    );
    Ok(match evidence {
        SitemapSourceFindingEvidence::Measured {
            record, protocol, ..
        } => {
            ensure!(
                protocol.measurement_id == record.measurement.measurement_id,
                "Sitemap finding protocol measurement identity differs"
            );
            let limit = match filter {
                F::XmlSitemapWithOver50kUrls => &protocol.entry_limit,
                F::XmlSitemapOver50mb => &protocol.decoded_xml_byte_limit,
                _ => unreachable!(),
            };
            match limit.verdict {
                SitemapSourceLimitVerdict::ObservedExceeded => {
                    assessment(V::Match, [R::SourceObservedProtocolExcess])
                }
                SitemapSourceLimitVerdict::WithinLimit => {
                    assessment(V::NotMatched, [R::SourceWithinProtocolLimit])
                }
                SitemapSourceLimitVerdict::Unknown => {
                    assessment(V::Unknown, [R::SourceProtocolUnknown])
                }
            }
        }
        SitemapSourceFindingEvidence::LegacyDocument { .. } => assessment(
            V::Unknown,
            [
                R::MeasurementUnavailable,
                R::LegacyDocumentWithoutMeasurement,
            ],
        ),
        SitemapSourceFindingEvidence::UnmeasuredTask { task } => assessment(
            V::Unknown,
            [
                R::MeasurementUnavailable,
                if task.done {
                    R::DoneSourceWithoutDocument
                } else {
                    R::PendingSourceTask
                },
            ],
        ),
    })
}

#[cfg(test)]
#[path = "predicate_tests.rs"]
mod tests;
