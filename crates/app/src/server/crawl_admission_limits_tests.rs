//! AUTHORED / NOT_RUN. Actual public DTO adapter, no website or Store I/O.
use super::*;

#[test]
fn static_limits_api_dto_absent_null_and_legacy_zero_aliases_preserve_effective_bytes() {
    let absent = config_from_input(
        json!({"seed_url":"https://example.test/","mode":"list","max_pages":0,"max_depth":0}),
    )
    .unwrap();
    let null=config_from_input(json!({"seed_url":"https://example.test/","mode":"list","max_pages":0,"max_depth":0,"crawl_admission_limits":null})).unwrap();
    assert_eq!(
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&null).unwrap()
    );
    assert!(absent.crawl_admission_limits.is_none());
    assert_eq!(absent.max_pages, None);
    assert_eq!(absent.max_depth, None);
    assert!(
        serde_json::to_value(&absent)
            .unwrap()
            .get("crawl_admission_limits")
            .is_none()
    );
}

#[test]
fn static_limits_api_dto_roundtrip_keeps_real_zero_bounds_and_rejects_before_launch() {
    let config=config_from_input(json!({"seed_url":"https://example.test/","crawl_admission_limits":{"version":1,"max_folder_depth":0,"max_query_tokens":0,"max_fetch_url_utf8_bytes":8192}})).unwrap();
    let limits = config.crawl_admission_limits.as_ref().unwrap();
    assert_eq!(limits.max_folder_depth, Some(0));
    assert_eq!(limits.max_query_tokens, Some(0));
    let saved = serde_json::to_value(&config).unwrap();
    let reopened = config_from_input(saved.clone()).unwrap();
    assert_eq!(serde_json::to_value(&reopened).unwrap(), saved);
    for invalid in [
        json!([]),
        json!({"max_folder_depth":1}),
        json!({"version":2,"max_folder_depth":1}),
        json!({"version":1,"max_query_tokens":-1}),
        json!({"version":1,"max_fetch_url_utf8_bytes":0}),
        json!({"version":1,"max_query_tokens":1,"quota":10}),
    ] {
        assert!(
            config_from_input(
                json!({"seed_url":"https://example.test/","crawl_admission_limits":invalid})
            )
            .is_err()
        );
    }
    assert!(config_from_input(json!({"seed_url":"https://example.test/","crawl_admission_limits":{"version":1,"max_folder_depth":0},"renderer":serde_json::to_value(open_seo_spider_core::RendererConfig::default()).unwrap()})).is_err());
}
