//! Synthetic typed evidence tests for the local pure predicate only.
//! No HTTP/XML/PDF/browser/Store producer or physical threshold proof is implied.
use super::*;
use crate::sitemap_source_measurement::*;
use crate::{
    SitemapCohortCompletion, SitemapCoverageCounts, SitemapCoverageScope, SitemapCoverageTask,
    SitemapDocumentKind, SitemapEntityEncoding, SitemapNamespaceProfile, SitemapObservedInlinks,
    SitemapSavedPageFinding, SitemapSourceDocument, SitemapSourceKind,
    SitemapSourceStoredMeasurement, SitemapTargetObservation,
};
use std::collections::BTreeMap;

fn coverage(current: bool, historical: bool) -> SitemapCoverage {
    SitemapCoverage {
        engine: crate::SITEMAP_COVERAGE_ENGINE.into(),
        scope: if current {
            SitemapCoverageScope::CurrentDeclaredSources
        } else if historical {
            SitemapCoverageScope::InheritedHistoricalDeclaredSources
        } else {
            SitemapCoverageScope::Unavailable
        },
        completion: if current || historical {
            SitemapCohortCompletion::Complete
        } else {
            SitemapCohortCompletion::Unknown
        },
        current_valid_original_location_negative_permitted: current,
        historical_valid_original_location_negative_permitted: historical,
        reason_counts: BTreeMap::new(),
        counts: SitemapCoverageCounts::default(),
        basis: None,
    }
}
fn page(membership: SitemapSavedPageMembership) -> SitemapFindingPageCandidate {
    let listed = membership == SitemapSavedPageMembership::ListedOriginal;
    SitemapFindingPageCandidate {
        finding: SitemapSavedPageFinding {
            observation: SitemapTargetObservation {
                report_page_ordinal: 17,
                requested_url: "https://typed.example/a".into(),
                final_url: "https://typed.example/a".into(),
                status: Some(200),
                final_status: Some(200),
                indexable: true,
                indexability_reasons: vec![],
                blocked_by_robots: false,
                error: None,
                truncated: false,
                renderer_error: None,
                rendered: false,
                rendered_url: None,
                rendered_fetch_url: None,
                rendered_status: None,
                rendering_expected: false,
                rendering_state: "not_configured".into(),
                rendered_navigation_changed: false,
                redirect_chain_present: false,
                fetched_at: "2026-10-15T00:00:00Z".into(),
                age_scope: "captured_checkpoint_history".into(),
                state: SitemapTargetState::Indexable,
            },
            membership,
            original_membership_occurrences: u64::from(listed),
            original_source_document_count: u64::from(listed),
            original_source_observation_count: u64::from(listed),
            original_source_document_samples: vec![],
            source_document_samples_have_more: false,
            admitted_alias_occurrences: 0,
            observed_inlinks: SitemapObservedInlinks::default(),
            listed_without_observed_nonself_inlinks: listed,
        },
        content_type: "text/html; charset=utf-8".into(),
        robots_warning_field_observed: true,
        robots_warning: None,
        pdf_field_observed: true,
        pdf_record_present: false,
        pdf_status: None,
    }
}
fn listed() -> SitemapFindingPageCandidate {
    page(SitemapSavedPageMembership::ListedOriginal)
}
fn metadata_nonindexable(page: &mut SitemapFindingPageCandidate) {
    page.finding.observation.indexable = false;
    // An opaque existing-audit reason, intentionally not interpreted by text.
    page.finding.observation.indexability_reasons = vec!["local audit fact".into()];
    page.finding.observation.state = SitemapTargetState::NonIndexable;
}
fn expect_page(
    filter: F,
    page: &SitemapFindingPageCandidate,
    coverage: &SitemapCoverage,
    verdict: V,
    reason: R,
) {
    let actual = assess_saved_page(filter, page, coverage).unwrap();
    assert_eq!(actual.verdict, verdict, "{filter:?} {page:?}");
    assert!(actual.reasons.contains(&reason), "{actual:?}");
}

#[test]
fn exact_seven_granular_ids_and_domains_are_fixed() {
    let cases = [
        (F::UrlsInSitemap, "filter.sitemaps-urls-in-sitemap"),
        (F::UrlsNotInSitemap, "filter.sitemaps-urls-not-in-sitemap"),
        (F::OrphanUrls, "filter.sitemaps-orphan-urls"),
        (
            F::NonIndexableUrlsInSitemap,
            "filter.sitemaps-non-indexable-urls-in-sitemap",
        ),
        (
            F::UrlsInMultipleSitemaps,
            "filter.sitemaps-urls-in-multiple-sitemaps",
        ),
        (
            F::XmlSitemapWithOver50kUrls,
            "filter.sitemaps-xml-sitemap-with-over-50k-urls",
        ),
        (
            F::XmlSitemapOver50mb,
            "filter.sitemaps-xml-sitemap-over-50mb",
        ),
    ];
    for (index, (filter, expected)) in cases.into_iter().enumerate() {
        assert_eq!(filter.granular_id(), expected);
        assert_eq!(
            filter.domain(),
            if index < 5 {
                SitemapFindingDomain::SavedPage
            } else {
                SitemapFindingDomain::SourceAttempt
            }
        );
    }
}

#[test]
fn original_membership_truth_table_preserves_unknown_and_alias_control() {
    for (membership, coverage, in_verdict, out_verdict, reason) in [
        (
            SitemapSavedPageMembership::ListedOriginal,
            coverage(false, false),
            V::Match,
            V::NotMatched,
            R::ListedOriginal,
        ),
        (
            SitemapSavedPageMembership::NotListedCurrentDeclaredSources,
            coverage(true, false),
            V::NotMatched,
            V::Match,
            R::NotListedCurrentDeclaredSources,
        ),
        (
            SitemapSavedPageMembership::NotListedHistoricalDeclaredSources,
            coverage(false, true),
            V::NotMatched,
            V::Match,
            R::NotListedHistoricalDeclaredSources,
        ),
        (
            SitemapSavedPageMembership::Unknown,
            coverage(false, false),
            V::Unknown,
            V::Unknown,
            R::MembershipUnknown,
        ),
    ] {
        let mut candidate = page(membership);
        candidate.finding.admitted_alias_occurrences = 9;
        expect_page(F::UrlsInSitemap, &candidate, &coverage, in_verdict, reason);
        expect_page(
            F::UrlsNotInSitemap,
            &candidate,
            &coverage,
            out_verdict,
            reason,
        );
    }
}

#[test]
fn negative_membership_requires_the_matching_actual_cohort_permission() {
    for (membership, coverage) in [
        (
            SitemapSavedPageMembership::NotListedCurrentDeclaredSources,
            coverage(false, false),
        ),
        (
            SitemapSavedPageMembership::NotListedCurrentDeclaredSources,
            coverage(false, true),
        ),
        (
            SitemapSavedPageMembership::NotListedHistoricalDeclaredSources,
            coverage(true, false),
        ),
    ] {
        for filter in [F::UrlsInSitemap, F::UrlsNotInSitemap] {
            expect_page(
                filter,
                &page(membership.clone()),
                &coverage,
                V::Unknown,
                R::MembershipUnknown,
            );
        }
    }
}

#[test]
fn orphan_uses_all_nonself_anchors_and_known_false_conjuncts() {
    let partial = coverage(false, false);
    let mut candidate = listed();
    candidate.finding.observed_inlinks.anchor_occurrences = 1; // self only
    expect_page(
        F::OrphanUrls,
        &candidate,
        &partial,
        V::Match,
        R::CapturedGraphOnly,
    );
    candidate.finding.membership = SitemapSavedPageMembership::Unknown;
    expect_page(
        F::OrphanUrls,
        &candidate,
        &partial,
        V::Unknown,
        R::MembershipUnknown,
    );
    candidate
        .finding
        .observed_inlinks
        .nonself_anchor_occurrences = 1;
    candidate
        .finding
        .observed_inlinks
        .internal_nonself_anchor_occurrences = 0;
    expect_page(
        F::OrphanUrls,
        &candidate,
        &partial,
        V::NotMatched,
        R::ObservedNonselfAnchors,
    );
    let absent = page(SitemapSavedPageMembership::NotListedCurrentDeclaredSources);
    expect_page(
        F::OrphanUrls,
        &absent,
        &coverage(true, false),
        V::NotMatched,
        R::NotListedCurrentDeclaredSources,
    );
}

#[test]
fn multiple_sources_counts_original_distinct_tasks_not_occurrences_or_observations() {
    let mut candidate = listed();
    candidate.finding.original_membership_occurrences = 20;
    candidate.finding.original_source_observation_count = 4;
    // Repeated capture/duplicates of one task cannot prove multiple sources.
    expect_page(
        F::UrlsInMultipleSitemaps,
        &candidate,
        &coverage(false, false),
        V::Unknown,
        R::SourceCohortIncomplete,
    );
    for complete in [coverage(true, false), coverage(false, true)] {
        expect_page(
            F::UrlsInMultipleSitemaps,
            &candidate,
            &complete,
            V::NotMatched,
            R::CompleteCohortAtMostOneOriginalSource,
        );
    }
    candidate.finding.original_source_document_count = 2;
    expect_page(
        F::UrlsInMultipleSitemaps,
        &candidate,
        &coverage(false, false),
        V::Match,
        R::ObservedMultipleOriginalSources,
    );
}

#[test]
fn nonindexable_known_false_conjunct_shortcircuits_uncertain_other_conjunct() {
    let mut absent = page(SitemapSavedPageMembership::NotListedCurrentDeclaredSources);
    absent.finding.observation.error = Some("unavailable".into());
    absent.finding.observation.indexable = false;
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &absent,
        &coverage(true, false),
        V::NotMatched,
        R::NotListedCurrentDeclaredSources,
    );
    let indexable = page(SitemapSavedPageMembership::Unknown);
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &indexable,
        &coverage(false, false),
        V::NotMatched,
        R::CompleteIndexableObservation,
    );
    let mut nonindexable = indexable;
    metadata_nonindexable(&mut nonindexable);
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &nonindexable,
        &coverage(false, false),
        V::Unknown,
        R::MembershipUnknown,
    );
}

#[test]
fn native_redirect_and_non200_survive_later_render_pdf_and_robots_failures() {
    for (redirect, reason) in [(true, R::OriginalRedirect), (false, R::OriginalNon200)] {
        let mut candidate = listed();
        candidate.content_type = "application/pdf".into();
        candidate.pdf_status = Some(ResourceAnalysisStatus::Error);
        candidate.pdf_record_present = true;
        candidate.robots_warning = Some("robots acquisition failed".into());
        candidate.finding.observation.blocked_by_robots = true;
        candidate.finding.observation.error = Some("body failed".into());
        candidate.finding.observation.renderer_error = Some("browser failed".into());
        candidate.finding.observation.truncated = true;
        if redirect {
            candidate.finding.observation.status = Some(301);
            candidate.finding.observation.redirect_chain_present = true;
            candidate.finding.observation.final_url = "https://typed.example/final".into();
        } else {
            candidate.finding.observation.final_status = Some(404);
        }
        expect_page(
            F::NonIndexableUrlsInSitemap,
            &candidate,
            &coverage(false, false),
            V::Match,
            reason,
        );
    }
}

#[test]
fn robots_requires_observed_warning_field_and_actual_disallow_qualification() {
    let mut candidate = listed();
    candidate.finding.observation.blocked_by_robots = true;
    candidate.finding.observation.state = SitemapTargetState::BlockedByRobots;
    candidate.finding.observation.status = None;
    candidate.finding.observation.final_status = None;
    candidate.finding.observation.final_url.clear();
    candidate.finding.observation.renderer_error = Some("later browser failure".into());
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Match,
        R::OriginalRobotsDisallow,
    );
    for warning in [Some(String::new()), Some("robots500".into())] {
        candidate.robots_warning = warning;
        expect_page(
            F::NonIndexableUrlsInSitemap,
            &candidate,
            &coverage(false, false),
            V::Unknown,
            R::RobotsPolicyUnavailable,
        );
    }
    candidate.robots_warning = None;
    candidate.robots_warning_field_observed = false;
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Unknown,
        R::RobotsEvidenceUnobserved,
    );
}

#[test]
fn complete_metadata_decision_uses_saved_audit_and_never_reason_text_heuristics() {
    let mut candidate = listed();
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::NotMatched,
        R::CompleteIndexableObservation,
    );
    metadata_nonindexable(&mut candidate);
    // Words are opaque: scoped directives/canonical policy was evaluated by
    // the actual saved audit, not guessed from this string by this predicate.
    for reason in [
        "opaque",
        "Noindex directive",
        "index,noindex",
        "self+foreign canonical",
    ] {
        candidate.finding.observation.indexability_reasons = vec![reason.into()];
        expect_page(
            F::NonIndexableUrlsInSitemap,
            &candidate,
            &coverage(false, false),
            V::Match,
            R::ObservedNonIndexability,
        );
    }
    candidate.finding.observation.indexability_reasons.clear();
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Unknown,
        R::IndexabilityReasonsUnavailable,
    );
}

#[test]
fn target_failure_pending_missing_and_incomplete_states_stay_unknown() {
    for (state, expected) in [
        (SitemapTargetState::FetchFailed, R::TargetFetchFailed),
        (
            SitemapTargetState::IncompleteObservation,
            R::TargetObservationIncomplete,
        ),
        (SitemapTargetState::NoResponse, R::TargetNoResponse),
        (SitemapTargetState::Pending, R::TargetPending),
        (
            SitemapTargetState::NoStoredRequestedObservation,
            R::TargetUnobserved,
        ),
        (
            SitemapTargetState::InvalidOrMissingOriginalLocation,
            R::OriginalLocationUnavailable,
        ),
    ] {
        let mut candidate = listed();
        metadata_nonindexable(&mut candidate);
        candidate.finding.observation.state = state;
        expect_page(
            F::NonIndexableUrlsInSitemap,
            &candidate,
            &coverage(false, false),
            V::Unknown,
            expected,
        );
    }
    let mut candidate = listed();
    metadata_nonindexable(&mut candidate);
    candidate.finding.observation.truncated = true;
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Unknown,
        R::TargetObservationIncomplete,
    );
    candidate.finding.observation.truncated = false;
    candidate.finding.observation.status = None;
    candidate.finding.observation.final_status = None;
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Unknown,
        R::TargetNoResponse,
    );
}

#[test]
fn pdf_complete_disabled_and_unobserved_partial_error_encrypted_are_distinct() {
    let mut candidate = listed();
    metadata_nonindexable(&mut candidate);
    candidate.content_type = "Application/PDF; charset=binary".into();
    candidate.pdf_record_present = true;
    for status in [
        ResourceAnalysisStatus::Complete,
        ResourceAnalysisStatus::Disabled,
    ] {
        candidate.pdf_status = Some(status);
        expect_page(
            F::NonIndexableUrlsInSitemap,
            &candidate,
            &coverage(false, false),
            V::Match,
            R::ObservedNonIndexability,
        );
    }
    candidate.finding.observation.indexable = true;
    candidate.finding.observation.indexability_reasons.clear();
    candidate.finding.observation.state = SitemapTargetState::Indexable;
    for status in [
        ResourceAnalysisStatus::Complete,
        ResourceAnalysisStatus::Disabled,
    ] {
        candidate.pdf_status = Some(status);
        expect_page(
            F::NonIndexableUrlsInSitemap,
            &candidate,
            &coverage(false, false),
            V::NotMatched,
            R::CompleteIndexableObservation,
        );
    }
    metadata_nonindexable(&mut candidate);
    for status in [
        ResourceAnalysisStatus::Partial,
        ResourceAnalysisStatus::Error,
        ResourceAnalysisStatus::Encrypted,
    ] {
        candidate.pdf_status = Some(status);
        expect_page(
            F::NonIndexableUrlsInSitemap,
            &candidate,
            &coverage(false, false),
            V::Unknown,
            R::PdfAnalysisIncomplete,
        );
    }
    candidate.pdf_status = None;
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Unknown,
        R::PdfAnalysisUnobserved,
    );
    candidate.pdf_status = Some(ResourceAnalysisStatus::Complete);
    candidate.pdf_record_present = false;
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Unknown,
        R::PdfAnalysisUnobserved,
    );
    candidate.pdf_record_present = true;
    candidate.pdf_field_observed = false;
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Unknown,
        R::PdfAnalysisUnobserved,
    );
}

#[test]
fn rendered_same_url_complete_decision_is_separate_from_navigation_or_status() {
    let mut candidate = listed();
    candidate.finding.observation.rendering_expected = true;
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Unknown,
        R::TargetObservationIncomplete,
    );
    candidate.finding.observation.rendered = true;
    candidate.finding.observation.rendered_url = Some("https://typed.example/a".into());
    candidate.finding.observation.rendered_fetch_url = Some("https://typed.example/a".into());
    candidate.finding.observation.rendered_status = Some(200);
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::NotMatched,
        R::CompleteIndexableObservation,
    );
    metadata_nonindexable(&mut candidate);
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Match,
        R::ObservedNonIndexability,
    );
    candidate.finding.observation.rendered_status = Some(404);
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Unknown,
        R::RenderedResponseNon200,
    );
    candidate.finding.observation.rendered_status = Some(200);
    candidate.finding.observation.rendered_fetch_url =
        Some("https://typed.example/browser-target".into());
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Unknown,
        R::RenderedNavigationChanged,
    );
    candidate.finding.observation.rendered_fetch_url = None;
    candidate.finding.observation.rendered_status = None;
    expect_page(
        F::NonIndexableUrlsInSitemap,
        &candidate,
        &coverage(false, false),
        V::Unknown,
        R::TargetObservationIncomplete,
    );
}

fn source_payload(entries: u64, xml_bytes: u64) -> SitemapSourceMeasurementPayload {
    let url = "https://typed.example/sitemap.xml";
    let revision = "typed-predicate-fixture-not-a-real-fetch";
    let expected = crate::sitemap_ledger::source_observation_id(
        SitemapSourceKind::SitemapXml,
        url,
        1,
        revision,
    )
    .unwrap();
    SitemapSourceMeasurementPayload {
        schema_version: 1,
        engine: SITEMAP_SOURCE_MEASUREMENT_ENGINE.into(),
        binding: SitemapSourceMeasurementBinding {
            crawl_config_sha256: "a".repeat(64),
            run_generation: 1,
            source_kind: SitemapSourceKind::SitemapXml,
            requested_task_url: url.into(),
            expected_source_observation_id: expected,
            source_revision: revision.into(),
        },
        transport: SitemapSourceTransportMeasurement {
            http_decode_profile: SITEMAP_SOURCE_HTTP_PROFILE.into(),
            request_started: true,
            requested_url: Some(url.into()),
            final_url: Some(url.into()),
            initial_status: Some(200),
            final_status: Some(200),
            blocked_by_robots: false,
            body_complete: true,
            post_http_received_bytes: SitemapMeasuredValue::exact(100),
            retained_post_http_prefix_bytes: 100,
            retained_post_http_prefix_sha256: Some("b".repeat(64)),
            complete_post_http_entity_sha256: Some("b".repeat(64)),
        },
        decode: SitemapSourceDecodeMeasurement {
            encoding: Some(SitemapEntityEncoding::GzipFileEntity),
            decoded_xml_bytes: SitemapMeasuredValue::exact(xml_bytes),
            decoded_prefix_sha256: Some("c".repeat(64)),
            decoded_complete_sha256: Some("c".repeat(64)),
            integrity: SitemapSourceDecodeIntegrity::Complete,
            stop_code: None,
        },
        scan: SitemapSourceScanMeasurement {
            xml_profile: SITEMAP_SOURCE_XML_PROFILE.into(),
            loc_lexical_profile: SITEMAP_SOURCE_LOC_PROFILE.into(),
            document_kind: Some(SitemapDocumentKind::UrlSet),
            namespace_profile: Some(SitemapNamespaceProfile::Sitemap09),
            status: SitemapSourceScanStatus::CompleteProfile,
            stop_code: None,
            events_observed: SitemapMeasuredValue::exact(entries * 6 + 2),
            counts: SitemapSourceEntryMeasurements {
                immediate_entry_starts: SitemapMeasuredValue::exact(entries),
                completed_immediate_entries: SitemapMeasuredValue::exact(entries),
                actual_document_namespace_loc_occurrences: SitemapMeasuredValue::exact(entries),
                protocol_eligible_completed_entries: SitemapMeasuredValue::exact(entries),
                missing_loc_entries: 0,
                multiple_loc_entries: 0,
                invalid_single_loc_entries: 0,
            },
        },
        membership_retention: SitemapMembershipRetention::Incomplete,
        bounds: SitemapSourceCaptureBounds {
            max_retained_post_http_bytes: 67_108_864,
            max_retained_xml_bytes: 67_108_864,
            max_xml_events: 2_000_000,
            max_xml_depth: 64,
            max_field_utf8_bytes: 8192,
            max_retained_entries: 50_000,
            max_retained_location_occurrences: 100_000,
            max_retained_text_utf8_bytes: 33_554_432,
        },
    }
}
fn measured(payload: SitemapSourceMeasurementPayload) -> SitemapSourceFindingEvidence {
    let limits = SitemapSourceMeasurementLimits::default();
    let measurement = SitemapSourceMeasurement::new(payload, &limits).unwrap();
    let protocol = assess_sitemap_source_protocol_limits(&measurement, &limits).unwrap();
    SitemapSourceFindingEvidence::Measured {
        record: Box::new(SitemapSourceStoredMeasurement {
            schema_version: 1,
            engine: crate::SITEMAP_SOURCE_STORE_ENGINE.into(),
            sequence: 1,
            initial_catalog_config_sha256: "d".repeat(64),
            staged_document_sha256: "e".repeat(64),
            measurement,
        }),
        protocol,
        association: None, // Real A-only has no B proof; numeric verdict remains independent.
        committed_document: None,
    }
}
fn expect_source(filter: F, source: &SitemapSourceFindingEvidence, verdict: V, reason: R) {
    let actual = assess_source(filter, source).unwrap();
    assert_eq!(actual.verdict, verdict, "{source:?}");
    assert!(actual.reasons.contains(&reason), "{actual:?}");
}

#[test]
fn source_exact_entry_and_decoded_byte_thresholds_map_existing_projector() {
    for kind in [
        SitemapDocumentKind::UrlSet,
        SitemapDocumentKind::SitemapIndex,
    ] {
        for entries in [49_999, 50_000, 50_001] {
            let mut payload = source_payload(entries, 1_000_000);
            payload.scan.document_kind = Some(kind);
            let (verdict, reason) = if entries > 50_000 {
                (V::Match, R::SourceObservedProtocolExcess)
            } else {
                (V::NotMatched, R::SourceWithinProtocolLimit)
            };
            expect_source(
                F::XmlSitemapWithOver50kUrls,
                &measured(payload),
                verdict,
                reason,
            );
        }
    }
    for bytes in [52_428_799, 52_428_800, 52_428_801] {
        let (verdict, reason) = if bytes > 52_428_800 {
            (V::Match, R::SourceObservedProtocolExcess)
        } else {
            (V::NotMatched, R::SourceWithinProtocolLimit)
        };
        expect_source(
            F::XmlSitemapOver50mb,
            &measured(source_payload(1, bytes)),
            verdict,
            reason,
        );
    }
}

#[test]
fn source_eligible_lower_bound_and_structural_excess_remain_distinct() {
    let mut structural = source_payload(50_001, 1_000_000);
    structural.scan.counts.protocol_eligible_completed_entries =
        SitemapMeasuredValue::exact(50_000);
    structural.scan.counts.invalid_single_loc_entries = 1;
    expect_source(
        F::XmlSitemapWithOver50kUrls,
        &measured(structural),
        V::Unknown,
        R::SourceProtocolUnknown,
    );
    let mut stopped = source_payload(50_001, 1_000_000);
    stopped.scan.status = SitemapSourceScanStatus::Failed;
    stopped.scan.stop_code = Some(SitemapSourceMeasurementStop::DtdDenied);
    stopped.scan.events_observed.precision = SitemapMeasuredPrecision::AtLeast;
    let counts = &mut stopped.scan.counts;
    counts.immediate_entry_starts.precision = SitemapMeasuredPrecision::AtLeast;
    counts.completed_immediate_entries.precision = SitemapMeasuredPrecision::AtLeast;
    counts.actual_document_namespace_loc_occurrences.precision = SitemapMeasuredPrecision::AtLeast;
    counts.protocol_eligible_completed_entries.precision = SitemapMeasuredPrecision::AtLeast;
    expect_source(
        F::XmlSitemapWithOver50kUrls,
        &measured(stopped),
        V::Match,
        R::SourceObservedProtocolExcess,
    );
    let mut unqualified = source_payload(50_001, 52_428_801);
    unqualified.scan.namespace_profile = Some(SitemapNamespaceProfile::LegacyUnqualified);
    let unqualified = measured(unqualified);
    expect_source(
        F::XmlSitemapWithOver50kUrls,
        &unqualified,
        V::Unknown,
        R::SourceProtocolUnknown,
    );
    expect_source(
        F::XmlSitemapOver50mb,
        &unqualified,
        V::Match,
        R::SourceObservedProtocolExcess,
    );
}

#[test]
fn failed_decode_returned_lower_bound_above_limit_matches_but_below_stays_unknown() {
    for bytes in [52_428_800, 52_428_801] {
        let mut payload = source_payload(1, bytes);
        payload.decode.integrity = SitemapSourceDecodeIntegrity::Failed;
        payload.decode.stop_code = Some(SitemapSourceMeasurementStop::InvalidGzip);
        payload.decode.decoded_xml_bytes = SitemapMeasuredValue::at_least(bytes);
        payload.decode.decoded_complete_sha256 = None;
        payload.scan.document_kind = None;
        payload.scan.namespace_profile = None;
        payload.scan.status = SitemapSourceScanStatus::NotObserved;
        payload.scan.stop_code = Some(SitemapSourceMeasurementStop::InvalidGzip);
        payload.scan.events_observed = SitemapMeasuredValue::unavailable();
        payload.scan.counts = SitemapSourceEntryMeasurements {
            immediate_entry_starts: SitemapMeasuredValue::unavailable(),
            completed_immediate_entries: SitemapMeasuredValue::unavailable(),
            actual_document_namespace_loc_occurrences: SitemapMeasuredValue::unavailable(),
            protocol_eligible_completed_entries: SitemapMeasuredValue::unavailable(),
            missing_loc_entries: 0,
            multiple_loc_entries: 0,
            invalid_single_loc_entries: 0,
        };
        let evidence = measured(payload);
        let (verdict, reason) = if bytes > 52_428_800 {
            (V::Match, R::SourceObservedProtocolExcess)
        } else {
            (V::Unknown, R::SourceProtocolUnknown)
        };
        expect_source(F::XmlSitemapOver50mb, &evidence, verdict, reason);
        expect_source(
            F::XmlSitemapWithOver50kUrls,
            &evidence,
            V::Unknown,
            R::SourceProtocolUnknown,
        );
    }
}

#[test]
fn source_non200_and_large_received_entity_do_not_fabricate_decoded_extent() {
    let mut payload = source_payload(1, 1000);
    payload.transport.initial_status = Some(503);
    payload.transport.final_status = Some(503);
    payload.transport.post_http_received_bytes = SitemapMeasuredValue::exact(52_428_801);
    payload.decode = SitemapSourceDecodeMeasurement {
        encoding: None,
        decoded_xml_bytes: SitemapMeasuredValue::unavailable(),
        decoded_prefix_sha256: None,
        decoded_complete_sha256: None,
        integrity: SitemapSourceDecodeIntegrity::NotObserved,
        stop_code: Some(SitemapSourceMeasurementStop::HttpStatusNot200),
    };
    payload.scan.document_kind = None;
    payload.scan.namespace_profile = None;
    payload.scan.status = SitemapSourceScanStatus::NotObserved;
    payload.scan.stop_code = Some(SitemapSourceMeasurementStop::HttpStatusNot200);
    payload.scan.events_observed = SitemapMeasuredValue::unavailable();
    payload.scan.counts = SitemapSourceEntryMeasurements {
        immediate_entry_starts: SitemapMeasuredValue::unavailable(),
        completed_immediate_entries: SitemapMeasuredValue::unavailable(),
        actual_document_namespace_loc_occurrences: SitemapMeasuredValue::unavailable(),
        protocol_eligible_completed_entries: SitemapMeasuredValue::unavailable(),
        missing_loc_entries: 0,
        multiple_loc_entries: 0,
        invalid_single_loc_entries: 0,
    };
    let evidence = measured(payload);
    for filter in [F::XmlSitemapWithOver50kUrls, F::XmlSitemapOver50mb] {
        expect_source(filter, &evidence, V::Unknown, R::SourceProtocolUnknown);
    }
}

#[test]
fn legacy_and_pending_or_done_unmeasured_sources_never_become_numeric_zero() {
    let document = SitemapSourceFindingEvidence::LegacyDocument {
        document: Box::new(SitemapSourceDocument {
            sequence: 3,
            observation_id: "f".repeat(64),
            kind: SitemapSourceKind::SitemapXml,
            task_url: "https://typed.example/legacy.xml".into(),
            generation: 1,
            source_revision: "legacy".into(),
            transport: None,
            entity_decode: None,
            parser: None,
            declaration_scan_complete: false,
            declaration_utf8_lossy: None,
            declaration_count: None,
            retained_rows_complete: false,
            failure_code: None,
        }),
    };
    for filter in [F::XmlSitemapWithOver50kUrls, F::XmlSitemapOver50mb] {
        expect_source(
            filter,
            &document,
            V::Unknown,
            R::LegacyDocumentWithoutMeasurement,
        );
        for done in [false, true] {
            let task = SitemapSourceFindingEvidence::UnmeasuredTask {
                task: SitemapCoverageTask {
                    kind: SitemapSourceKind::SitemapXml,
                    task_url: "https://typed.example/task.xml".into(),
                    done,
                },
            };
            expect_source(
                filter,
                &task,
                V::Unknown,
                if done {
                    R::DoneSourceWithoutDocument
                } else {
                    R::PendingSourceTask
                },
            );
        }
    }
}

#[test]
fn wrong_domain_and_cross_measurement_protocol_are_errors_not_unknown() {
    let mut evidence = measured(source_payload(1, 1000));
    assert!(assess_source(F::UrlsInSitemap, &evidence).is_err());
    assert!(assess_saved_page(F::XmlSitemapOver50mb, &listed(), &coverage(false, false)).is_err());
    if let SitemapSourceFindingEvidence::Measured { protocol, .. } = &mut evidence {
        protocol.measurement_id = "0".repeat(64);
    }
    assert!(assess_source(F::XmlSitemapOver50mb, &evidence).is_err());
}
