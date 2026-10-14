use super::*;
use crate::{CrawlConfig, SitemapCountLimits};
use serde_json::{Value, json};

fn field_profile(field: &str, value: usize) -> SitemapSourceProcessingConfig {
    serde_json::from_value(json!({(field): value})).unwrap()
}

#[test]
fn source_profile_defaults_and_nested_crawl_serde_preserve_legacy_absence() {
    let profile: SitemapSourceProcessingConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(profile, SitemapSourceProcessingConfig::default());
    assert_eq!(profile.max_post_http_entity_bytes, 8 * 1024 * 1024);
    assert_eq!(profile.max_decoded_xml_bytes, 8 * 1024 * 1024);
    assert_eq!(profile.max_xml_events, 500_000);
    assert_eq!(
        profile.parser_limits().unwrap(),
        SitemapParserLimits::default()
    );

    let config: CrawlConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(config.sitemap_source_processing, None);
    assert_eq!(config.max_body_bytes, 5 * 1024 * 1024);
    assert_eq!(config.sitemap_source_body_limit().unwrap(), 5 * 1024 * 1024);
    assert_eq!(
        config.sitemap_source_parser_limits().unwrap(),
        SitemapParserLimits::default()
    );
    assert!(
        serde_json::to_value(&config)
            .unwrap()
            .get("sitemap_source_processing")
            .is_none()
    );

    let configured: CrawlConfig = serde_json::from_value(json!({
        "sitemap_source_processing": {}
    }))
    .unwrap();
    assert_eq!(configured.sitemap_source_processing, Some(profile));
    assert_eq!(
        configured.sitemap_source_body_limit().unwrap(),
        8 * 1024 * 1024
    );
    let encoded = serde_json::to_value(&configured).unwrap();
    assert!(encoded["sitemap_source_processing"].is_object());
    assert_eq!(
        serde_json::from_value::<CrawlConfig>(encoded)
            .unwrap()
            .sitemap_source_processing,
        configured.sitemap_source_processing
    );
}

#[test]
fn source_profile_rejects_unknown_fields_and_non_integer_shapes() {
    for value in [
        json!({"max_xml_bytes": 64}),
        json!({"max_post_http_entity_bytes": 64, "retain_all_rows": true}),
        json!({"max_decoded_xml_bytes": -1}),
        json!({"max_xml_events": 1.5}),
        json!({"max_xml_events": "500000"}),
        json!({"max_decoded_xml_bytes": null}),
        json!([]),
    ] {
        assert!(
            serde_json::from_value::<SitemapSourceProcessingConfig>(value.clone()).is_err(),
            "{value}"
        );
        assert!(
            serde_json::from_value::<CrawlConfig>(json!({"sitemap_source_processing": value}))
                .is_err()
        );
    }
    // Type overflow is rejected by serde before validation, on either 32/64-bit usize.
    assert!(
        serde_json::from_str::<SitemapSourceProcessingConfig>(
            r#"{"max_xml_events":18446744073709551616}"#
        )
        .is_err()
    );
}

#[test]
fn source_profile_all_three_exact_caps_and_zero_plus_one_are_explicit() {
    let fields = [
        (
            "max_post_http_entity_bytes",
            MAX_SITEMAP_SOURCE_POST_HTTP_ENTITY_BYTES,
        ),
        (
            "max_decoded_xml_bytes",
            MAX_SITEMAP_SOURCE_DECODED_XML_BYTES,
        ),
        ("max_xml_events", MAX_SITEMAP_SOURCE_XML_EVENTS),
    ];
    for (field, cap) in fields {
        for accepted in [1, cap - 1, cap] {
            let profile = field_profile(field, accepted);
            assert!(profile.validate().is_ok());
            assert!(profile.parser_limits().is_ok());
        }
        for rejected in [0, cap + 1, usize::MAX] {
            let profile = field_profile(field, rejected);
            let before = profile.clone();
            let failure = profile.validate().unwrap_err();
            assert_eq!(failure.code, "sitemap_invalid_limits");
            assert_eq!(failure.message, format!("{field} must be 1..={cap}"));
            assert_eq!(failure.xml_byte_offset, None);
            assert_eq!(profile.parser_limits().unwrap_err(), failure);
            assert_eq!(profile, before, "no clamping or repair");
        }
    }
}

#[test]
fn configured_protocol_byte_neighbors_are_valid_budgets_without_allocating_sources() {
    for bytes in [52_428_799, 52_428_800, 52_428_801] {
        let profile = SitemapSourceProcessingConfig {
            max_post_http_entity_bytes: bytes,
            max_decoded_xml_bytes: bytes,
            max_xml_events: MAX_SITEMAP_SOURCE_XML_EVENTS,
        };
        let parser = profile.parser_limits().unwrap();
        assert_eq!(parser.max_entity_bytes, bytes);
        assert_eq!(parser.max_xml_bytes, bytes);
        assert_eq!(parser.max_xml_events, 2_000_000);
        let mut original = parser.clone();
        original.max_entity_bytes = SitemapParserLimits::default().max_entity_bytes;
        original.max_xml_bytes = SitemapParserLimits::default().max_xml_bytes;
        original.max_xml_events = SitemapParserLimits::default().max_xml_events;
        assert_eq!(
            original,
            SitemapParserLimits::default(),
            "all retention settings unchanged"
        );
    }
}

#[test]
fn actual_crawl_helpers_keep_none_clamp_and_explicit_source_page_independence() {
    for (page_bytes, expected) in [
        (1, 1),
        (5 * 1024 * 1024, 5 * 1024 * 1024),
        (64 * 1024 * 1024, 8 * 1024 * 1024),
    ] {
        let config = CrawlConfig {
            max_body_bytes: page_bytes,
            ..Default::default()
        };
        assert_eq!(config.sitemap_source_body_limit().unwrap(), expected);
        assert_eq!(config.max_body_bytes, page_bytes);
    }
    let profile = SitemapSourceProcessingConfig {
        max_post_http_entity_bytes: MAX_SITEMAP_SOURCE_POST_HTTP_ENTITY_BYTES,
        max_decoded_xml_bytes: MAX_SITEMAP_SOURCE_DECODED_XML_BYTES,
        max_xml_events: MAX_SITEMAP_SOURCE_XML_EVENTS,
    };
    let config = CrawlConfig {
        max_body_bytes: 1,
        sitemap_source_processing: Some(profile.clone()),
        ..Default::default()
    };
    assert_eq!(
        config.sitemap_source_body_limit().unwrap(),
        64 * 1024 * 1024
    );
    assert_eq!(
        config.sitemap_source_parser_limits().unwrap(),
        profile.parser_limits().unwrap()
    );
    assert_eq!(
        config.max_body_bytes, 1,
        "source opt-in does not modify page budget"
    );
}

#[test]
fn crawl_body_helper_validates_other_profile_fields_before_returning_entity_limit() {
    for (field, invalid) in [
        (
            "max_post_http_entity_bytes",
            MAX_SITEMAP_SOURCE_POST_HTTP_ENTITY_BYTES + 1,
        ),
        (
            "max_decoded_xml_bytes",
            MAX_SITEMAP_SOURCE_DECODED_XML_BYTES + 1,
        ),
        ("max_xml_events", MAX_SITEMAP_SOURCE_XML_EVENTS + 1),
    ] {
        let config = CrawlConfig {
            sitemap_source_processing: Some(field_profile(field, invalid)),
            ..Default::default()
        };
        assert!(config.sitemap_source_body_limit().is_err());
        assert!(config.sitemap_source_parser_limits().is_err());
    }
    let no_body = CrawlConfig {
        max_body_bytes: 0,
        ..Default::default()
    };
    assert!(no_body.sitemap_source_body_limit().is_err());
}

#[test]
fn parser_work_caps_expand_but_all_other_default_caps_remain_enforced() {
    let high = SitemapSourceProcessingConfig {
        max_post_http_entity_bytes: MAX_SITEMAP_SOURCE_POST_HTTP_ENTITY_BYTES,
        max_decoded_xml_bytes: MAX_SITEMAP_SOURCE_DECODED_XML_BYTES,
        max_xml_events: MAX_SITEMAP_SOURCE_XML_EVENTS,
    }
    .parser_limits()
    .unwrap();
    let mut json_limits = serde_json::to_value(&high).unwrap();
    for (field, maximum) in [
        ("max_entity_bytes", 64 * 1024 * 1024),
        ("max_xml_bytes", 64 * 1024 * 1024),
        ("max_xml_events", 2_000_000),
        ("max_xml_depth", 64),
        ("max_entries", 50_000),
        ("max_location_occurrences", 100_000),
        ("max_field_utf8_bytes", 8192),
        ("max_retained_text_utf8_bytes", 32 * 1024 * 1024),
    ] {
        let prior = json_limits[field].clone();
        for rejected in [0, maximum + 1] {
            json_limits[field] = json!(rejected);
            let limits: SitemapParserLimits = serde_json::from_value(json_limits.clone()).unwrap();
            assert_eq!(
                limits.validate().unwrap_err().code,
                "sitemap_invalid_limits",
                "{field}"
            );
        }
        json_limits[field] = prior;
    }
    assert_eq!(
        serde_json::from_value::<SitemapParserLimits>(json_limits).unwrap(),
        high
    );
    assert!(high.allow_legacy_unqualified);
}

#[test]
fn count_work_limits_accept_large_profile_and_keep_its_own_default() {
    assert_eq!(SitemapCountLimits::default().max_xml_bytes, 8 * 1024 * 1024);
    assert_eq!(SitemapCountLimits::default().max_xml_events, 500_000);
    let configured = SitemapCountLimits {
        max_xml_bytes: MAX_SITEMAP_SOURCE_DECODED_XML_BYTES,
        max_xml_events: MAX_SITEMAP_SOURCE_XML_EVENTS,
        ..Default::default()
    };
    assert!(configured.validate().is_ok());
    let too_large = SitemapCountLimits {
        max_xml_bytes: MAX_SITEMAP_SOURCE_DECODED_XML_BYTES + 1,
        ..configured
    };
    assert_eq!(
        too_large.validate().unwrap_err().code,
        "sitemap_invalid_limits"
    );
}

#[test]
fn actual_small_xml_pipeline_honors_explicit_byte_and_event_boundaries() {
    let xml = br#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"/>"#;
    let profile = SitemapSourceProcessingConfig {
        max_post_http_entity_bytes: xml.len(),
        max_decoded_xml_bytes: xml.len(),
        max_xml_events: 2, // Empty root and actual EOF.
    };
    let config = CrawlConfig {
        max_body_bytes: 1,
        sitemap_source_processing: Some(profile.clone()),
        ..Default::default()
    };
    let parser = config.sitemap_source_parser_limits().unwrap();
    assert_eq!(config.sitemap_source_body_limit().unwrap(), xml.len());
    let decoded = crate::decode_sitemap_entity_observed(xml, &parser);
    assert_eq!(
        decoded.observation.decoded_bytes_returned,
        Some(xml.len() as u64)
    );
    let decoded = decoded.result.unwrap();
    assert_eq!(decoded.xml, xml);
    let count_limits = SitemapCountLimits {
        max_xml_bytes: parser.max_xml_bytes,
        max_xml_events: parser.max_xml_events,
        ..Default::default()
    };
    let count = crate::count_sitemap_xml(&decoded.xml, &count_limits).unwrap();
    assert!(count.xml_scan_complete);
    assert_eq!(count.events_observed, 2);
    assert_eq!(count.counts.completed_immediate_entries, 0);
    assert!(
        crate::observe_sitemap_xml(&decoded.xml, Default::default(), &parser)
            .unwrap()
            .complete
    );

    let no_eof_budget = SitemapCountLimits {
        max_xml_events: 1,
        ..count_limits
    };
    let count = crate::count_sitemap_xml(&decoded.xml, &no_eof_budget).unwrap();
    assert!(!count.xml_scan_complete);
    assert_eq!(count.events_observed, 2);
    assert_eq!(count.failure.unwrap().code, "sitemap_xml_event_budget");

    let mut short_xml = profile.clone();
    short_xml.max_decoded_xml_bytes -= 1;
    let failed = crate::decode_sitemap_entity_observed(xml, &short_xml.parser_limits().unwrap());
    assert_eq!(failed.result.unwrap_err().code, "sitemap_xml_byte_budget");
    assert_eq!(failed.observation.decoded_bytes_returned, None);

    let short_entity = SitemapSourceProcessingConfig {
        max_post_http_entity_bytes: xml.len() - 1,
        ..profile
    };
    let failed = crate::decode_sitemap_entity_observed(xml, &short_entity.parser_limits().unwrap());
    assert_eq!(
        failed.result.unwrap_err().code,
        "sitemap_entity_byte_budget"
    );
    assert_eq!(failed.observation.encoding, None);
    assert_eq!(failed.observation.decoded_bytes_returned, None);
}

#[test]
fn separately_lowered_entity_and_xml_budgets_are_not_reordered_or_repaired() {
    for (entity, xml) in [(1, 4096), (4096, 1)] {
        let profile = SitemapSourceProcessingConfig {
            max_post_http_entity_bytes: entity,
            max_decoded_xml_bytes: xml,
            max_xml_events: 1,
        };
        let parser = profile.parser_limits().unwrap();
        assert_eq!(parser.max_entity_bytes, entity);
        assert_eq!(parser.max_xml_bytes, xml);
        let encoded: Value = serde_json::to_value(&profile).unwrap();
        assert_eq!(encoded["max_post_http_entity_bytes"], json!(entity));
        assert_eq!(encoded["max_decoded_xml_bytes"], json!(xml));
    }
}
