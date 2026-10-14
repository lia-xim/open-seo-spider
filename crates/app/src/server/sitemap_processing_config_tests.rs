//! Actual create-config adapter controls; no request producer or website fetch.
use super::{config_from_input, json};

#[test]
fn create_dto_preserves_source_work_limits_without_widening_page_bodies() {
    let input = json!({
        "seed_urls": ["https://source-profile.example/"],
        "max_body_bytes": 128,
        "sitemap_source_processing": {
            "max_post_http_entity_bytes": 67_108_864,
            "max_decoded_xml_bytes": 67_108_864,
            "max_xml_events": 2_000_000
        }
    });
    let config = config_from_input(input).unwrap();
    assert_eq!(config.max_body_bytes, 128);
    assert_eq!(config.sitemap_source_body_limit().unwrap(), 67_108_864);
    let limits = config.sitemap_source_parser_limits().unwrap();
    assert_eq!(limits.max_xml_bytes, 67_108_864);
    assert_eq!(limits.max_xml_events, 2_000_000);
    assert_eq!(limits.max_entries, 50_000);
    let saved = serde_json::to_value(&config).unwrap();
    assert_eq!(saved["max_body_bytes"], 128);
    assert_eq!(
        saved["sitemap_source_processing"]["max_post_http_entity_bytes"],
        67_108_864
    );
    let repeated = config_from_input(saved.clone()).unwrap();
    assert_eq!(serde_json::to_value(repeated).unwrap(), saved);
}

#[test]
fn create_dto_missing_or_null_source_profile_keeps_historical_configuration() {
    let input = json!({"seed_urls": ["https://source-profile.example/"]});
    let absent = config_from_input(input.clone()).unwrap();
    let mut with_null = input;
    with_null["sitemap_source_processing"] = serde_json::Value::Null;
    let explicit_null = config_from_input(with_null).unwrap();
    assert!(absent.sitemap_source_processing.is_none());
    assert_eq!(absent.sitemap_source_body_limit().unwrap(), 5 * 1024 * 1024);
    assert_eq!(
        absent.sitemap_source_parser_limits().unwrap().max_xml_bytes,
        8 * 1024 * 1024
    );
    let saved = serde_json::to_value(absent).unwrap();
    assert!(saved.get("sitemap_source_processing").is_none());
    assert_eq!(serde_json::to_value(explicit_null).unwrap(), saved);
}

#[test]
fn create_dto_rejects_invalid_or_unknown_source_profile_fields() {
    for profile in [
        json!({"max_post_http_entity_bytes": 0}),
        json!({"max_post_http_entity_bytes": 67_108_865}),
        json!({"max_decoded_xml_bytes": 0}),
        json!({"max_decoded_xml_bytes": 67_108_865}),
        json!({"max_xml_events": 0}),
        json!({"max_xml_events": 2_000_001}),
        json!({"max_post_http_entity_bytes": "67108864"}),
        json!({"test_override": true}),
    ] {
        let result = config_from_input(json!({
            "seed_urls": ["https://source-profile.example/"],
            "sitemap_source_processing": profile
        }));
        assert!(result.is_err(), "invalid profile accepted: {profile}");
    }
}
