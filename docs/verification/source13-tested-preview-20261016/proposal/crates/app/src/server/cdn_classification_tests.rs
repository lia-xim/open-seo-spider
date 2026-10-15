//! AUTHORED_NOT_RUN. Actual public config adapter and durable typed declaration.
use super::*;

#[test]
fn cdn_api_config_roundtrip_requires_versioned_exact_origins_and_never_creates_resource_grants() {
    let value = json!({"seed_url":"https://www.example/","cdn_classification":{"version":1,"origins":["https://cdn.example:443/","http://cdn.example:8080"]}});
    let config = config_from_input(value).unwrap();
    let saved = serde_json::to_value(&config).unwrap();
    assert_eq!(
        saved["cdn_classification"]["origins"],
        json!(["https://cdn.example:443/", "http://cdn.example:8080"])
    );
    assert!(config.resource_discovery.is_none());
    assert!(!config.check_external);
    let restored = config_from_input(saved.clone()).unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), saved);
    for invalid in [
        json!([]),
        json!({"version":2,"origins":["https://cdn.example"]}),
        json!({"version":1,"origins":["https://cdn.example/path"]}),
        json!({"version":1,"origins":["https://cdn.example","https://CDN.example:443"]}),
        json!({"version":1,"origins":["https://cdn.example"],"expand":true}),
    ] {
        assert!(
            config_from_input(
                json!({"seed_url":"https://www.example/","cdn_classification":invalid})
            )
            .is_err()
        );
    }
}

#[test]
fn cdn_api_null_and_omission_keep_old_effective_config_without_backfilling_observations() {
    let absent = config_from_input(json!({"seed_url":"https://www.example/"})).unwrap();
    let null =
        config_from_input(json!({"seed_url":"https://www.example/","cdn_classification":null}))
            .unwrap();
    assert_eq!(
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&null).unwrap()
    );
    assert!(
        serde_json::to_value(&absent)
            .unwrap()
            .get("cdn_classification")
            .is_none()
    );
    let old: open_seo_spider_core::PageRecord =
        serde_json::from_value(json!({"url":"https://cdn.example/a","internal":false})).unwrap();
    assert!(old.cdn_classification.is_none());
    assert!(!old.internal);
}
