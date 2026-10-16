//! AUTHORED / NOT_RUN. API config/strict resume DTO regressions before launch IO.
use super::*;
#[test]
fn quotas_api_preserves_absence_and_rejects_invalid_or_unsupported_config() {
    let legacy =
        super::super::config_from_input(json!({"seed_url":"https://example.test/"})).unwrap();
    assert!(legacy.crawl_quotas.is_none());
    let config=super::super::config_from_input(json!({"seed_url":"https://example.test/","crawl_quotas":{"version":1,"max_urls_per_depth":0,"url_pattern_quotas":[{"id":"shop","pattern":"/shop/","max_urls":2}]}})).unwrap();
    assert_eq!(config.crawl_quotas.unwrap().max_urls_per_depth, Some(0));
    for invalid in [
        json!({"version":2,"max_urls_per_depth":1}),
        json!({"version":1,"max_urls_per_depth":-1}),
        json!({"version":1,"max_urls_per_depth":1,"unknown":true}),
        json!({"version":1,"url_pattern_quotas":[{"id":"same","pattern":"a","max_urls":1},{"id":"same","pattern":"b","max_urls":2}]}),
    ] {
        assert!(
            super::super::config_from_input(
                json!({"seed_url":"https://example.test/","crawl_quotas":invalid})
            )
            .is_err()
        );
    }
    assert!(super::super::config_from_input(json!({"seed_url":"https://example.test/","crawl_quotas":{"version":1,"max_urls_per_depth":1},"renderer":open_seo_spider_core::RendererConfig::default()})).is_err());
}
#[test]
fn quotas_api_resume_requires_full_core_authority_and_summary_has_no_actions() {
    let valid = json!({"quota_increases":{"expected_generation":3,"expected_profile_revision":2,"expected_profile_sha256":"a".repeat(64),"max_urls_per_depth":5,"url_pattern_maxima":{}}});
    assert!(serde_json::from_value::<super::super::ResumeOptions>(valid.clone()).is_ok());
    for field in [
        "expected_generation",
        "expected_profile_revision",
        "expected_profile_sha256",
    ] {
        let mut invalid = valid.clone();
        invalid["quota_increases"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(serde_json::from_value::<super::super::ResumeOptions>(invalid).is_err());
    }
    assert!(serde_json::from_value::<SummaryQuery>(json!({"action":"csv"})).is_err());
    let mut invalid = valid;
    invalid["quota_increases"]["pattern"] = json!("new rule");
    assert!(serde_json::from_value::<super::super::ResumeOptions>(invalid).is_err());
}
