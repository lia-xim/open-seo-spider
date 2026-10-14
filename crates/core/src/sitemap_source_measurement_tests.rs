//! Synthetic caller-supplied typed evidence for the PURE contract only.
//! These fixtures are not captured HTTP/XML, scanner tests or native threshold proof.
use super::*;
use SitemapMeasuredPrecision as P;
use SitemapSourceLimitVerdict as V;
use SitemapSourceMeasurementFailureCode as F;

fn digest(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
fn limits() -> SitemapSourceMeasurementLimits {
    SitemapSourceMeasurementLimits::default()
}
fn bind(p: &mut SitemapSourceMeasurementPayload) {
    p.binding.expected_source_observation_id = crate::sitemap_ledger::source_observation_id(
        p.binding.source_kind,
        &p.binding.requested_task_url,
        p.binding.run_generation,
        &p.binding.source_revision,
    )
    .unwrap();
}
fn complete(entries: u64, xml_bytes: u64) -> SitemapSourceMeasurementPayload {
    let mut p = SitemapSourceMeasurementPayload {
        schema_version: 1,
        engine: SITEMAP_SOURCE_MEASUREMENT_ENGINE.into(),
        binding: SitemapSourceMeasurementBinding {
            crawl_config_sha256: digest(b"unit config"),
            run_generation: 1,
            source_kind: SitemapSourceKind::SitemapXml,
            requested_task_url: "https://unit.example/sitemap.xml".into(),
            expected_source_observation_id: String::new(),
            source_revision: "opaque-unit-source-revision".into(),
        },
        transport: SitemapSourceTransportMeasurement {
            http_decode_profile: SITEMAP_SOURCE_HTTP_PROFILE.into(),
            request_started: true,
            requested_url: Some("https://unit.example/sitemap.xml".into()),
            final_url: Some("https://unit.example/sitemap.xml".into()),
            initial_status: Some(200),
            final_status: Some(200),
            blocked_by_robots: false,
            body_complete: true,
            post_http_received_bytes: SitemapMeasuredValue::exact(100),
            retained_post_http_prefix_bytes: 100,
            retained_post_http_prefix_sha256: Some(digest(b"unit entity")),
            complete_post_http_entity_sha256: Some(digest(b"unit entity")),
        },
        decode: SitemapSourceDecodeMeasurement {
            encoding: Some(SitemapEntityEncoding::GzipFileEntity),
            decoded_xml_bytes: SitemapMeasuredValue::exact(xml_bytes),
            decoded_prefix_sha256: Some(digest(b"unit decoded")),
            decoded_complete_sha256: Some(digest(b"unit decoded")),
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
            max_retained_post_http_bytes: 8_388_608,
            max_retained_xml_bytes: 8_388_608,
            max_xml_events: 1_000_000,
            max_xml_depth: 64,
            max_field_utf8_bytes: 8192,
            max_retained_entries: 50_000,
            max_retained_location_occurrences: 100_000,
            max_retained_text_utf8_bytes: 33_554_432,
        },
    };
    bind(&mut p);
    p
}
fn assessed(p: SitemapSourceMeasurementPayload) -> SitemapSourceProtocolAssessment {
    let record = SitemapSourceMeasurement::new(p, &limits()).unwrap();
    assess_sitemap_source_protocol_limits(&record, &limits()).unwrap()
}
fn reject(p: SitemapSourceMeasurementPayload, expected: F) {
    assert_eq!(
        SitemapSourceMeasurement::new(p, &limits())
            .unwrap_err()
            .code,
        expected
    );
}
fn failed_scan(p: &mut SitemapSourceMeasurementPayload) {
    p.scan.status = SitemapSourceScanStatus::Failed;
    p.scan.stop_code = Some(SitemapSourceMeasurementStop::DtdDenied);
    p.scan.events_observed.precision = P::AtLeast;
    let c = &mut p.scan.counts;
    c.immediate_entry_starts.precision = P::AtLeast;
    c.completed_immediate_entries.precision = P::AtLeast;
    c.actual_document_namespace_loc_occurrences.precision = P::AtLeast;
    c.protocol_eligible_completed_entries.precision = P::AtLeast;
}
fn no_scan(p: &mut SitemapSourceMeasurementPayload, stop: SitemapSourceMeasurementStop) {
    p.scan.status = SitemapSourceScanStatus::NotObserved;
    p.scan.stop_code = Some(stop);
    p.scan.document_kind = None;
    p.scan.namespace_profile = None;
    p.scan.events_observed = SitemapMeasuredValue::unavailable();
    p.scan.counts = SitemapSourceEntryMeasurements {
        immediate_entry_starts: SitemapMeasuredValue::unavailable(),
        completed_immediate_entries: SitemapMeasuredValue::unavailable(),
        actual_document_namespace_loc_occurrences: SitemapMeasuredValue::unavailable(),
        protocol_eligible_completed_entries: SitemapMeasuredValue::unavailable(),
        missing_loc_entries: 0,
        multiple_loc_entries: 0,
        invalid_single_loc_entries: 0,
    };
}
fn failed_gzip(bytes: u64) -> SitemapSourceMeasurementPayload {
    let mut p = complete(0, bytes);
    p.decode.integrity = SitemapSourceDecodeIntegrity::Failed;
    p.decode.stop_code = Some(SitemapSourceMeasurementStop::InvalidGzip);
    p.decode.decoded_xml_bytes = SitemapMeasuredValue::at_least(bytes);
    if bytes == 0 {
        p.decode.decoded_prefix_sha256 = Some(digest(b""));
    }
    p.decode.decoded_complete_sha256 = None;
    no_scan(&mut p, SitemapSourceMeasurementStop::InvalidGzip);
    p
}

#[test]
fn exact_count_ceiling_minus_equal_plus_for_urlsets_and_indexes() {
    for kind in [
        SitemapDocumentKind::UrlSet,
        SitemapDocumentKind::SitemapIndex,
    ] {
        for count in [49_999, 50_000, 50_001] {
            let mut p = complete(count, 1_000_000);
            p.scan.document_kind = Some(kind);
            let a = assessed(p);
            assert_eq!(a.document_kind, Some(kind));
            assert_eq!(a.entry_limit.threshold, 50_000);
            assert_eq!(
                a.entry_limit.verdict,
                if count > 50_000 {
                    V::ObservedExceeded
                } else {
                    V::WithinLimit
                }
            );
            assert_eq!(a.entry_limit.proof_quantity.value, Some(count));
            assert_eq!(
                a.membership_retention,
                SitemapMembershipRetention::Incomplete
            );
        }
    }
}
#[test]
fn exact_decoded_byte_ceiling_minus_equal_plus_is_independent_of_xml_count() {
    for bytes in [52_428_799, 52_428_800, 52_428_801] {
        let a = assessed(complete(1, bytes));
        assert_eq!(a.entry_limit.verdict, V::WithinLimit);
        assert_eq!(
            a.decoded_xml_byte_limit.verdict,
            if bytes > 52_428_800 {
                V::ObservedExceeded
            } else {
                V::WithinLimit
            }
        );
        assert_eq!(
            a.decoded_xml_byte_limit.proof_quantity,
            SitemapMeasuredValue::exact(bytes)
        );
    }
}
#[test]
fn lower_bound_eligible_count_can_prove_excess_after_dtd_but_never_within() {
    for count in [49_999, 50_000, 50_001] {
        let mut p = complete(count, 1_000_000);
        failed_scan(&mut p);
        let a = assessed(p);
        assert_eq!(
            a.entry_limit.verdict,
            if count > 50_000 {
                V::ObservedExceeded
            } else {
                V::Unknown
            }
        );
        assert_eq!(a.entry_limit.proof_quantity.precision, P::AtLeast);
        assert_eq!(a.xml_scan_status, SitemapSourceScanStatus::Failed);
        assert_eq!(a.decode_integrity, SitemapSourceDecodeIntegrity::Complete);
    }
}
#[test]
fn gzip_crc_failure_preserves_byte_excess_lower_bound_without_integrity_success() {
    for bytes in [52_428_799, 52_428_800, 52_428_801] {
        let a = assessed(failed_gzip(bytes));
        assert_eq!(
            a.decoded_xml_byte_limit.verdict,
            if bytes > 52_428_800 {
                V::ObservedExceeded
            } else {
                V::Unknown
            }
        );
        assert_eq!(
            a.decoded_xml_byte_limit.proof_quantity,
            SitemapMeasuredValue::at_least(bytes)
        );
        assert_eq!(a.decode_integrity, SitemapSourceDecodeIntegrity::Failed);
        assert_eq!(a.entry_limit.verdict, V::Unknown);
    }
}
#[test]
fn entry_start_without_completed_loc_is_not_url_excess_or_count_within() {
    let mut p = complete(50_000, 1_000_000);
    failed_scan(&mut p);
    p.scan.counts.immediate_entry_starts = SitemapMeasuredValue::at_least(50_001);
    // An active entry can already contain a closed loc before its own close.
    p.scan.counts.actual_document_namespace_loc_occurrences =
        SitemapMeasuredValue::at_least(50_001);
    let a = assessed(p);
    assert!(a.observed_entry_starts_exceeded);
    assert!(!a.observed_completed_entry_slots_exceeded);
    assert_eq!(a.entry_limit.verdict, V::Unknown);
}
#[test]
fn many_locs_in_ambiguous_entries_are_not_many_eligible_urls() {
    let mut p = complete(25_001, 1_000_000);
    p.scan.counts.actual_document_namespace_loc_occurrences = SitemapMeasuredValue::exact(50_001);
    p.scan.counts.protocol_eligible_completed_entries = SitemapMeasuredValue::exact(1);
    p.scan.counts.multiple_loc_entries = 25_000;
    let a = assessed(p);
    assert_eq!(a.entry_limit.verdict, V::WithinLimit); // structural upper bound only
    assert_eq!(
        a.entry_limit.basis,
        SitemapSourceLimitBasis::CompleteStructuralUpperBound
    );
    assert_eq!(a.entry_limit.proof_quantity.value, Some(25_001));
}
#[test]
fn structural_excess_with_missing_or_invalid_entries_keeps_valid_url_excess_unknown() {
    for invalid in [false, true] {
        let mut p = complete(50_001, 1_000_000);
        p.scan.counts.protocol_eligible_completed_entries = SitemapMeasuredValue::exact(50_000);
        if invalid {
            p.scan.counts.invalid_single_loc_entries = 1;
        } else {
            p.scan.counts.missing_loc_entries = 1;
            p.scan.counts.actual_document_namespace_loc_occurrences =
                SitemapMeasuredValue::exact(50_000);
        }
        let a = assessed(p);
        assert!(a.observed_completed_entry_slots_exceeded);
        assert_eq!(a.entry_limit.verdict, V::Unknown);
        assert_eq!(
            a.entry_limit.basis,
            SitemapSourceLimitBasis::StructuralExcessWithoutEligibleExcess
        );
    }
}
#[test]
fn valid_plus_invalid_dual_loc_is_one_ineligible_entry() {
    let mut p = complete(1, 100);
    p.scan.counts.protocol_eligible_completed_entries = SitemapMeasuredValue::exact(0);
    p.scan.counts.actual_document_namespace_loc_occurrences = SitemapMeasuredValue::exact(2);
    p.scan.counts.multiple_loc_entries = 1;
    assert_eq!(assessed(p).entry_limit.verdict, V::WithinLimit);
}
#[test]
fn standard_and_legacy_namespace_have_separate_count_and_byte_outcomes() {
    let mut p = complete(50_001, 52_428_801);
    p.scan.namespace_profile = Some(SitemapNamespaceProfile::LegacyUnqualified);
    let a = assessed(p);
    assert_eq!(a.entry_limit.verdict, V::Unknown);
    assert_eq!(
        a.entry_limit.basis,
        SitemapSourceLimitBasis::UnrecognizedStandardSource
    );
    assert_eq!(
        a.namespace_profile,
        Some(SitemapNamespaceProfile::LegacyUnqualified)
    );
    assert_eq!(a.decoded_xml_byte_limit.verdict, V::ObservedExceeded);
}
#[test]
fn exact_full_bytes_can_be_within_when_xml_suffix_failed() {
    let mut p = complete(10, 52_428_800);
    failed_scan(&mut p);
    let a = assessed(p);
    assert_eq!(a.entry_limit.verdict, V::Unknown);
    assert_eq!(a.decoded_xml_byte_limit.verdict, V::WithinLimit);
    assert_eq!(a.xml_scan_status, SitemapSourceScanStatus::Failed);
}
#[test]
fn truly_empty_recognized_source_is_distinct_from_unavailable() {
    let a = assessed(complete(0, 100));
    assert_eq!(a.entry_limit.verdict, V::WithinLimit);
    assert_eq!(a.entry_limit.proof_quantity, SitemapMeasuredValue::exact(0));
    let mut p = complete(0, 100);
    p.transport.request_started = false;
    p.transport.initial_status = None;
    p.transport.final_status = None;
    p.transport.final_url = None;
    p.transport.body_complete = false;
    p.transport.post_http_received_bytes = SitemapMeasuredValue::unavailable();
    p.transport.retained_post_http_prefix_bytes = 0;
    p.transport.retained_post_http_prefix_sha256 = None;
    p.transport.complete_post_http_entity_sha256 = None;
    p.decode = SitemapSourceDecodeMeasurement {
        encoding: None,
        decoded_xml_bytes: SitemapMeasuredValue::unavailable(),
        decoded_prefix_sha256: None,
        decoded_complete_sha256: None,
        integrity: SitemapSourceDecodeIntegrity::NotObserved,
        stop_code: Some(SitemapSourceMeasurementStop::RequestNotStarted),
    };
    no_scan(&mut p, SitemapSourceMeasurementStop::RequestNotStarted);
    p.membership_retention = SitemapMembershipRetention::Unobserved;
    let a = assessed(p);
    assert_eq!(a.entry_limit.verdict, V::Unknown);
    assert_eq!(
        a.entry_limit.proof_quantity,
        SitemapMeasuredValue::unavailable()
    );
    assert_eq!(a.decoded_xml_byte_limit.verdict, V::Unknown);
}
#[test]
fn complete_206_response_is_not_a_complete_http200_source() {
    let mut p = complete(0, 100);
    p.transport.initial_status = Some(206);
    p.transport.final_status = Some(206);
    p.decode = SitemapSourceDecodeMeasurement {
        encoding: None,
        decoded_xml_bytes: SitemapMeasuredValue::unavailable(),
        decoded_prefix_sha256: None,
        decoded_complete_sha256: None,
        integrity: SitemapSourceDecodeIntegrity::NotObserved,
        stop_code: Some(SitemapSourceMeasurementStop::HttpStatusNot200),
    };
    no_scan(&mut p, SitemapSourceMeasurementStop::HttpStatusNot200);
    let a = assessed(p);
    assert_eq!(a.entry_limit.verdict, V::Unknown);
    assert_eq!(a.decoded_xml_byte_limit.verdict, V::Unknown);
}
#[test]
fn lower_byte_prefix_at_resource_cap_is_unknown_and_known_zero_is_not_missing() {
    for bytes in [0, 5_242_880, 8_388_609] {
        let mut p = failed_gzip(bytes);
        p.decode.integrity = SitemapSourceDecodeIntegrity::Incomplete;
        p.decode.stop_code = Some(SitemapSourceMeasurementStop::DecodedByteLimit);
        no_scan(&mut p, SitemapSourceMeasurementStop::DecodedByteLimit);
        let a = assessed(p);
        assert_eq!(a.decoded_xml_byte_limit.verdict, V::Unknown);
        assert_eq!(a.decoded_xml_byte_limit.proof_quantity.value, Some(bytes));
    }
}
#[test]
fn binding_preserves_actual_legacy_percent_and_opaque_revision_lexemes() {
    let mut p = complete(1, 100);
    p.binding.requested_task_url = "https://unit.example/sitemap%ZZ.xml?x=%".into();
    p.binding.source_revision = "legacy opaque revision: \"original\"".into();
    p.transport.requested_url = Some(p.binding.requested_task_url.clone());
    p.transport.final_url = Some("https://unit.example/final%2.xml".into());
    bind(&mut p);
    let before = p.clone();
    let r = SitemapSourceMeasurement::new(p, &limits()).unwrap();
    assert_eq!(r.payload, before);
    let json = serde_json::to_vec(&r).unwrap();
    assert_eq!(
        parse_sitemap_source_measurement_json(&json, &limits()).unwrap(),
        r
    );
    assert_eq!(
        assess_sitemap_source_protocol_limits(&r, &limits())
            .unwrap()
            .entry_limit
            .verdict,
        V::WithinLimit
    );
}
#[test]
fn content_id_is_stable_and_binds_every_typed_payload_axis() {
    let p = complete(1, 100);
    let r = SitemapSourceMeasurement::new(p.clone(), &limits()).unwrap();
    assert_eq!(
        SitemapSourceMeasurement::new(p.clone(), &limits())
            .unwrap()
            .measurement_id,
        r.measurement_id
    );
    let mut altered = r.clone();
    altered.payload.membership_retention = SitemapMembershipRetention::Complete;
    assert_eq!(
        validate_sitemap_source_measurement(&altered, &limits())
            .unwrap_err()
            .code,
        F::MeasurementIdMismatch
    );
    let re_sealed = SitemapSourceMeasurement::new(altered.payload, &limits()).unwrap();
    assert_ne!(r.measurement_id, re_sealed.measurement_id);
    let mut altered = p;
    altered.binding.crawl_config_sha256 = digest(b"different config");
    assert_ne!(
        r.measurement_id,
        SitemapSourceMeasurement::new(altered, &limits())
            .unwrap()
            .measurement_id
    );
}
#[test]
fn source_observation_binding_checks_the_existing_exact_legacy_tuple() {
    let mut p = complete(1, 100);
    p.binding.run_generation = 2; // expected source ID still belongs to generation1
    reject(p, F::InvalidBinding);
    let mut p = complete(1, 100);
    p.transport.requested_url = Some("https://unit.example/other.xml".into());
    reject(p, F::InvalidBinding);
    let mut p = complete(1, 100);
    p.binding.source_kind = SitemapSourceKind::RobotsDirectives;
    bind(&mut p);
    reject(p, F::InvalidBinding);
}
#[test]
fn exact_atleast_unavailable_shapes_and_cross_axis_contradictions_are_rejected() {
    let mut p = complete(1, 100);
    p.decode.decoded_xml_bytes = SitemapMeasuredValue {
        value: Some(0),
        precision: P::Unavailable,
    };
    reject(p, F::InvalidQuantity);
    let mut p = complete(1, 100);
    p.decode.decoded_xml_bytes = SitemapMeasuredValue {
        value: None,
        precision: P::Exact,
    };
    reject(p, F::InvalidQuantity);
    let mut p = complete(1, 100);
    p.decode.decoded_xml_bytes.precision = P::AtLeast;
    reject(p, F::InvalidDecode);
    let mut p = complete(1, 100);
    p.transport.body_complete = false; // cannot retain an exact full response claim
    reject(p, F::InvalidTransport);
    let mut p = complete(1, 100);
    p.scan.counts.immediate_entry_starts.precision = P::AtLeast;
    reject(p, F::InvalidScan);
    let mut p = complete(1, 100);
    p.decode.encoding = Some(SitemapEntityEncoding::Identity);
    p.decode.decoded_xml_bytes = SitemapMeasuredValue::exact(101);
    reject(p, F::InvalidDecode);
}
#[test]
fn inconsistent_counter_categories_and_event_accounting_are_rejected() {
    let mut p = complete(1, 100);
    p.scan.counts.protocol_eligible_completed_entries = SitemapMeasuredValue::exact(2);
    reject(p, F::CounterInconsistency);
    let mut p = complete(1, 100);
    p.scan.counts.missing_loc_entries = 1; // manufactures a second classification
    reject(p, F::CounterInconsistency);
    let mut p = complete(1, 100);
    p.scan.counts.multiple_loc_entries = 1;
    p.scan.counts.protocol_eligible_completed_entries = SitemapMeasuredValue::exact(0);
    reject(p, F::CounterInconsistency); // only one actual loc supplied
    let mut p = complete(1, 100);
    p.scan.events_observed = SitemapMeasuredValue::exact(1);
    reject(p, F::CounterInconsistency);
    let mut p = complete(1, 100);
    p.scan.counts.actual_document_namespace_loc_occurrences = SitemapMeasuredValue::exact(2);
    reject(p, F::CounterInconsistency); // no active or multiple-Loc entry can account for it
    let mut p = complete(1, 100);
    failed_scan(&mut p);
    p.scan.counts.immediate_entry_starts = SitemapMeasuredValue::at_least(3);
    reject(p, F::CounterInconsistency); // at most one strict immediate entry is active
}
#[test]
fn scalar_range_checked_arithmetic_and_effective_bounds_are_not_coerced() {
    let mut p = complete(0, 100);
    p.scan.counts.immediate_entry_starts = SitemapMeasuredValue::exact(u64::MAX);
    reject(p, F::ScalarOutOfRange);
    let mut p = complete(0, 100);
    p.scan.counts.missing_loc_entries = MAX_SCALAR;
    p.scan.counts.multiple_loc_entries = MAX_SCALAR;
    p.scan.counts.invalid_single_loc_entries = MAX_SCALAR;
    reject(p, F::ArithmeticOverflow);
    let mut p = complete(0, 100);
    p.bounds.max_retained_xml_bytes = 0;
    reject(p, F::InvalidLimits);
    let mut p = complete(0, 100);
    p.bounds.max_retained_post_http_bytes = 99;
    reject(p, F::InvalidTransport);
    let mut p = complete(0, 100);
    p.bounds.max_xml_events = 1;
    p.scan.events_observed = SitemapMeasuredValue::exact(3);
    reject(p, F::InvalidScan);
}
#[test]
fn bounded_json_accepts_exact_limit_rejects_plus_one_and_unknown_fields() {
    let r = SitemapSourceMeasurement::new(complete(1, 100), &limits()).unwrap();
    let bytes = serde_json::to_vec(&r).unwrap();
    let exact = SitemapSourceMeasurementLimits {
        max_record_json_bytes: bytes.len(),
        ..limits()
    };
    assert_eq!(
        parse_sitemap_source_measurement_json(&bytes, &exact).unwrap(),
        r
    );
    let small = SitemapSourceMeasurementLimits {
        max_record_json_bytes: bytes.len() - 1,
        ..limits()
    };
    assert_eq!(
        parse_sitemap_source_measurement_json(&bytes, &small)
            .unwrap_err()
            .code,
        F::JsonBytesLimit
    );
    for pointer in [
        "/unexpected",
        "/payload/unexpected",
        "/payload/scan/counts/unexpected",
    ] {
        let mut value = serde_json::to_value(&r).unwrap();
        let (parent, _) = pointer.rsplit_once('/').unwrap();
        value
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), serde_json::json!(true));
        assert_eq!(
            parse_sitemap_source_measurement_json(&serde_json::to_vec(&value).unwrap(), &limits())
                .unwrap_err()
                .code,
            F::InvalidJson
        );
    }
}
#[test]
fn unknown_enums_versions_profiles_and_oversized_identity_are_rejected() {
    let mut p = complete(0, 100);
    p.schema_version = 2;
    reject(p, F::UnrecognizedProfile);
    let mut p = complete(0, 100);
    p.scan.loc_lexical_profile = "unrecognized".into();
    reject(p, F::UnrecognizedProfile);
    let mut p = complete(0, 100);
    p.binding.source_revision = "x".repeat(MAX_IDENTITY_BYTES + 1);
    bind(&mut p);
    reject(p, F::InvalidIdentity);
    let r = SitemapSourceMeasurement::new(complete(1, 100), &limits()).unwrap();
    let mut value = serde_json::to_value(&r).unwrap();
    value["payload"]["decode"]["integrity"] = serde_json::json!("probably_complete");
    assert_eq!(
        parse_sitemap_source_measurement_json(&serde_json::to_vec(&value).unwrap(), &limits())
            .unwrap_err()
            .code,
        F::InvalidJson
    );
    let invalid = SitemapSourceMeasurementLimits {
        max_record_json_bytes: MAX_RECORD_BYTES + 1,
        ..limits()
    };
    assert_eq!(
        SitemapSourceMeasurement::new(complete(0, 100), &invalid)
            .unwrap_err()
            .code,
        F::InvalidLimits
    );
}
#[test]
fn constructor_enforces_serialized_budget_before_hashing_large_retained_strings() {
    let mut p = complete(0, 100);
    p.binding.requested_task_url = "x".repeat(MAX_IDENTITY_BYTES);
    p.transport.requested_url = Some(p.binding.requested_task_url.clone());
    p.transport.final_url = Some(p.binding.requested_task_url.clone());
    bind(&mut p);
    reject(p, F::JsonBytesLimit);
}

#[test]
fn scan_cannot_manufacture_counts_without_any_emitted_decode_bytes() {
    let mut p = complete(0, 100);
    failed_scan(&mut p);
    p.scan.document_kind = None;
    p.scan.namespace_profile = None;
    p.scan.events_observed = SitemapMeasuredValue::at_least(0);
    p.scan.stop_code = Some(SitemapSourceMeasurementStop::InvalidUtf8);
    let a = assessed(p);
    assert_eq!(a.entry_limit.verdict, V::Unknown); // observed prefix zero is not a complete source zero
    assert_eq!(a.decoded_xml_byte_limit.verdict, V::WithinLimit);
    let mut p = failed_gzip(100);
    p.decode.decoded_xml_bytes = SitemapMeasuredValue::unavailable();
    p.decode.decoded_prefix_sha256 = None;
    p.scan = complete(50_001, 1_000_000).scan;
    failed_scan(&mut p);
    reject(p, F::InvalidScan);
    let mut p = complete(0, 0);
    p.decode.decoded_prefix_sha256 = Some(digest(b""));
    p.decode.decoded_complete_sha256 = Some(digest(b""));
    reject(p, F::InvalidScan);
}

#[test]
fn zero_byte_hash_and_decode_stop_axes_must_match_the_recorded_extent() {
    let mut p = failed_gzip(0);
    p.decode.decoded_prefix_sha256 = Some(digest(b"nonempty"));
    reject(p, F::InvalidQuantity);
    let mut p = failed_gzip(100);
    p.decode.stop_code = Some(SitemapSourceMeasurementStop::XmlSyntax);
    reject(p, F::InvalidDecode);
    let mut p = failed_gzip(100);
    p.decode.integrity = SitemapSourceDecodeIntegrity::Incomplete;
    p.decode.encoding = Some(SitemapEntityEncoding::Identity);
    reject(p, F::InvalidDecode); // gzip truncation cannot claim an identity encoding
}

#[test]
fn partial_http200_capture_may_prove_excess_without_claiming_completion() {
    let mut p = complete(50_001, 52_428_801);
    p.transport.body_complete = false;
    p.transport.post_http_received_bytes = SitemapMeasuredValue::at_least(100);
    p.transport.complete_post_http_entity_sha256 = None;
    p.decode.integrity = SitemapSourceDecodeIntegrity::Incomplete;
    p.decode.stop_code = Some(SitemapSourceMeasurementStop::TransportIncomplete);
    p.decode.decoded_xml_bytes.precision = P::AtLeast;
    p.decode.decoded_complete_sha256 = None;
    failed_scan(&mut p);
    p.scan.stop_code = Some(SitemapSourceMeasurementStop::TransportIncomplete);
    let a = assessed(p);
    assert_eq!(a.entry_limit.verdict, V::ObservedExceeded);
    assert_eq!(a.decoded_xml_byte_limit.verdict, V::ObservedExceeded);
    assert_eq!(a.decode_integrity, SitemapSourceDecodeIntegrity::Incomplete);
    assert_eq!(a.xml_scan_status, SitemapSourceScanStatus::Failed);
    assert_eq!(
        a.membership_retention,
        SitemapMembershipRetention::Incomplete
    );
}

#[test]
fn helper_shaped_unexpected_eof_keeps_incomplete_gzip_reason_with_complete_http_body() {
    // Typed helper-shaped states only, not execution of a decoder or HTTP fixture.
    // The helper retains legacy sitemap_invalid_gzip_entity for UnexpectedEof;
    // it must not be rewritten to HTTP TransportIncomplete.
    for bytes in [0, 13, 52_428_801] {
        let mut p = failed_gzip(bytes);
        p.decode.integrity = SitemapSourceDecodeIntegrity::Incomplete;
        let r = SitemapSourceMeasurement::new(p, &limits()).unwrap();
        assert!(r.payload.transport.body_complete);
        assert_eq!(
            r.payload.transport.post_http_received_bytes.precision,
            P::Exact
        );
        assert_eq!(
            r.payload.decode.stop_code,
            Some(SitemapSourceMeasurementStop::InvalidGzip)
        );
        let a = assess_sitemap_source_protocol_limits(&r, &limits()).unwrap();
        assert_eq!(a.decode_integrity, SitemapSourceDecodeIntegrity::Incomplete);
        assert_eq!(
            a.decoded_xml_byte_limit.proof_quantity,
            SitemapMeasuredValue::at_least(bytes)
        );
        assert_eq!(
            a.decoded_xml_byte_limit.verdict,
            if bytes > 52_428_800 {
                V::ObservedExceeded
            } else {
                V::Unknown
            }
        );
        assert!(r.payload.decode.decoded_complete_sha256.is_none());
        let mut dishonest_http_reason = r.payload;
        dishonest_http_reason.decode.stop_code =
            Some(SitemapSourceMeasurementStop::TransportIncomplete);
        reject(dishonest_http_reason, F::InvalidDecode);
    }
}

#[test]
fn helper_shaped_identity_xml_preflight_preserves_known_encoding_without_output() {
    let mut p = complete(0, 100);
    p.bounds.max_retained_xml_bytes = 99;
    p.decode = SitemapSourceDecodeMeasurement {
        encoding: Some(SitemapEntityEncoding::Identity),
        decoded_xml_bytes: SitemapMeasuredValue::unavailable(),
        decoded_prefix_sha256: None,
        decoded_complete_sha256: None,
        integrity: SitemapSourceDecodeIntegrity::NotObserved,
        stop_code: Some(SitemapSourceMeasurementStop::DecodedByteLimit),
    };
    no_scan(&mut p, SitemapSourceMeasurementStop::DecodedByteLimit);
    p.membership_retention = SitemapMembershipRetention::Unobserved;
    let r = SitemapSourceMeasurement::new(p.clone(), &limits()).unwrap();
    assert_eq!(r.payload, p); // preserve sniff and unavailable output exactly
    let a = assess_sitemap_source_protocol_limits(&r, &limits()).unwrap();
    assert_eq!(a.entry_limit.verdict, V::Unknown);
    assert_eq!(a.decoded_xml_byte_limit.verdict, V::Unknown);
    assert_eq!(
        a.decoded_xml_byte_limit.proof_quantity,
        SitemapMeasuredValue::unavailable()
    );
    let mut no_xml_excess = p.clone();
    no_xml_excess.bounds.max_retained_xml_bytes = 100;
    reject(no_xml_excess, F::InvalidDecode);
    let mut invented_gzip_sniff = p.clone();
    invented_gzip_sniff.decode.encoding = Some(SitemapEntityEncoding::GzipFileEntity);
    reject(invented_gzip_sniff, F::InvalidDecode);
    for (denied, stop) in [
        (false, SitemapSourceMeasurementStop::RequestNotStarted),
        (true, SitemapSourceMeasurementStop::RobotsDenied),
    ] {
        let mut unobserved = p.clone();
        unobserved.transport.request_started = false;
        unobserved.transport.blocked_by_robots = denied;
        unobserved.transport.initial_status = None;
        unobserved.transport.final_status = None;
        unobserved.transport.final_url = None;
        unobserved.transport.body_complete = false;
        unobserved.transport.post_http_received_bytes = SitemapMeasuredValue::unavailable();
        unobserved.transport.retained_post_http_prefix_bytes = 0;
        unobserved.transport.retained_post_http_prefix_sha256 = None;
        unobserved.transport.complete_post_http_entity_sha256 = None;
        unobserved.decode.stop_code = Some(stop);
        no_scan(&mut unobserved, stop);
        reject(unobserved, F::InvalidDecode); // cannot invent encoding before sniff
    }
}

#[test]
fn whole_scan_event_cap_is_exact_and_only_event_limit_failure_can_have_sentinel() {
    let mut at_cap = complete(1, 100); // eight observed events in this typed fixture
    at_cap.bounds.max_xml_events = 8;
    assert_eq!(assessed(at_cap.clone()).entry_limit.verdict, V::WithinLimit);
    let mut impossible_complete = at_cap.clone();
    impossible_complete.bounds.max_xml_events = 7;
    reject(impossible_complete, F::InvalidScan);
    let mut failed_at_sentinel = at_cap;
    failed_at_sentinel.bounds.max_xml_events = 7;
    failed_scan(&mut failed_at_sentinel);
    failed_at_sentinel.scan.stop_code = Some(SitemapSourceMeasurementStop::EventLimit);
    let a = assessed(failed_at_sentinel.clone());
    assert_eq!(a.entry_limit.verdict, V::Unknown);
    assert_eq!(a.xml_scan_status, SitemapSourceScanStatus::Failed);
    for other_stop in [
        SitemapSourceMeasurementStop::DtdDenied,
        SitemapSourceMeasurementStop::XmlSyntax,
    ] {
        let mut wrong_sentinel = failed_at_sentinel.clone();
        wrong_sentinel.scan.stop_code = Some(other_stop);
        reject(wrong_sentinel, F::InvalidScan);
    }
    failed_at_sentinel.bounds.max_xml_events = 8;
    reject(failed_at_sentinel, F::InvalidScan); // EventLimit cannot occur before cap+1
}
