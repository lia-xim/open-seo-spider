//! AUTHORED_NOT_RUN. Real config_from_input typed validation and saved JSON consumer.
use super::*;

#[test]
fn cdn_directory_api_preserves_both_declared_profiles_and_refuses_unsupported_before_launch() {
    let v = json!({"seed_url":"https://www.example/","cdn_classification":{"version":1,"origins":["https://CDN.example:443/"]},"cdn_directory_reporting":{"version":1,"rules":["HTTPS://WWW.CDN.example/assets/","http://files.example:8443/"]}});
    let c = config_from_input(v.clone()).unwrap();
    let saved = serde_json::to_value(&c).unwrap();
    assert_eq!(
        saved["cdn_directory_reporting"],
        v["cdn_directory_reporting"]
    );
    assert_eq!(saved["cdn_classification"], v["cdn_classification"]);
    assert!(!c.check_external && c.resource_discovery.is_none());
    assert_eq!(
        serde_json::to_value(config_from_input(saved.clone()).unwrap()).unwrap(),
        saved
    );
    for invalid in [
        json!([]),
        json!({"version":2,"rules":["https://cdn.example/"]}),
        json!({"version":1,"rules":["https://cdn.example:443/"]}),
        json!({"version":1,"rules":["https://cdn.example/a/../"]}),
        json!({"version":1,"rules":["https://cdn.example/"],"fetch":true}),
    ] {
        assert!(
            config_from_input(
                json!({"seed_url":"https://www.example/","cdn_directory_reporting":invalid})
            )
            .is_err()
        );
    }
}

#[test]
fn cdn_directory_api_absence_null_and_legacy_page_link_provenance_remain_unknown() {
    let absent = config_from_input(json!({"seed_url":"https://www.example/"})).unwrap();
    let null = config_from_input(
        json!({"seed_url":"https://www.example/","cdn_directory_reporting":null}),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&null).unwrap()
    );
    assert!(
        serde_json::to_value(absent)
            .unwrap()
            .get("cdn_directory_reporting")
            .is_none()
    );
    let old: open_seo_spider_core::PageRecord = serde_json::from_value(json!({"url":"https://cdn.example/assets/a","internal":false,"links":[{"url":"https://cdn.example/assets/b","internal":false}]})).unwrap();
    assert!(
        old.cdn_directory_reporting.is_none() && old.links[0].cdn_directory_reporting.is_none()
    );
    assert!(!old.internal && !old.links[0].internal);
}
