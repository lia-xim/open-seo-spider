//! AUTHORED / NOT_RUN. No HTTP fixture: actual config/Store bootstrap and reopen.
use super::*;

fn declared_limits() -> crate::SitemapLedgerLimits {
    crate::SitemapLedgerLimits {
        max_catalog_json_bytes: 128 * 1024 * 1024,
        ..Default::default()
    }
}

#[test]
fn membership_capacity_absence_null_and_source_work_keep_legacy_serialization() {
    let absent: CrawlConfig = serde_json::from_str("{}").unwrap();
    let null: CrawlConfig = serde_json::from_str(r#"{"sitemap_membership_limits":null}"#).unwrap();
    assert!(absent.sitemap_membership_limits.is_none());
    assert_eq!(
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&null).unwrap()
    );
    assert!(
        serde_json::to_value(&absent)
            .unwrap()
            .get("sitemap_membership_limits")
            .is_none()
    );
    assert_eq!(
        absent
            .sitemap_membership_ledger_limits()
            .unwrap()
            .max_catalog_json_bytes,
        32 * 1024 * 1024
    );
    let source_only: CrawlConfig = serde_json::from_value(serde_json::json!({
        "sitemap_source_processing": {"max_post_http_entity_bytes":67108864,"max_decoded_xml_bytes":67108864,"max_xml_events":2000000}
    })).unwrap();
    assert_eq!(
        source_only.sitemap_source_body_limit().unwrap(),
        64 * 1024 * 1024
    );
    assert_eq!(
        source_only.sitemap_membership_ledger_limits().unwrap(),
        crate::SitemapLedgerLimits::default()
    );
    let membership_only: CrawlConfig = serde_json::from_value(serde_json::json!({
        "sitemap_membership_limits":{"max_catalog_json_bytes":134217728}
    }))
    .unwrap();
    assert_eq!(
        membership_only.sitemap_source_body_limit().unwrap(),
        5 * 1024 * 1024
    );
    assert_eq!(
        membership_only.sitemap_source_parser_limits().unwrap(),
        crate::SitemapParserLimits::default()
    );
    assert_eq!(
        membership_only.sitemap_membership_ledger_limits().unwrap(),
        declared_limits()
    );
    let empty: CrawlConfig = serde_json::from_str(r#"{"sitemap_membership_limits":{}}"#).unwrap();
    assert_eq!(
        empty.sitemap_membership_limits,
        Some(crate::SitemapLedgerLimits::default())
    );
    let encoded = serde_json::to_vec(&membership_only).unwrap();
    let restored: CrawlConfig = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored.sitemap_membership_limits, Some(declared_limits()));
    assert_eq!(serde_json::to_vec(&restored).unwrap(), encoded);
}

#[test]
fn membership_capacity_strict_objects_reject_shapes_fields_and_hard_limit_violations() {
    for profile in [
        serde_json::json!([]),
        serde_json::json!([33554432, 262144, 1000000]),
        serde_json::json!(false),
        serde_json::json!(0),
        serde_json::json!("128MiB"),
        serde_json::json!({"max_catalog_json_bytes":"134217728"}),
        serde_json::json!({"max_catalog_json_bytes":-1}),
        serde_json::json!({"max_catalog_json_bytes":1.5}),
        serde_json::json!({"max_catalog_json_bytes":null}),
        serde_json::json!({"uncapped":true}),
    ] {
        assert!(
            serde_json::from_value::<CrawlConfig>(
                serde_json::json!({"sitemap_membership_limits":profile})
            )
            .is_err()
        );
    }
    assert!(
        serde_json::from_str::<CrawlConfig>(
            r#"{"sitemap_membership_limits":{"max_rows":1,"max_rows":2}}"#
        )
        .is_err()
    );
    for limits in [
        crate::SitemapLedgerLimits {
            max_catalog_json_bytes: 0,
            ..declared_limits()
        },
        crate::SitemapLedgerLimits {
            max_catalog_json_bytes: 134217729,
            ..declared_limits()
        },
        crate::SitemapLedgerLimits {
            max_row_json_bytes: 0,
            ..declared_limits()
        },
        crate::SitemapLedgerLimits {
            max_row_json_bytes: 1048577,
            ..declared_limits()
        },
        crate::SitemapLedgerLimits {
            max_catalog_json_bytes: 262143,
            ..declared_limits()
        },
        crate::SitemapLedgerLimits {
            max_rows: 0,
            ..declared_limits()
        },
        crate::SitemapLedgerLimits {
            max_rows: 1000001,
            ..declared_limits()
        },
    ] {
        let config = CrawlConfig {
            seed_urls: vec!["https://example.test/".into()],
            sitemap_membership_limits: Some(limits),
            ..Default::default()
        };
        assert!(Crawler::new(config).is_err());
    }
    assert!(declared_limits().validate().is_ok());
}

#[test]
fn configured_membership_capacity_is_saved_bootstrapped_reopened_and_not_mutated_on_resume() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("membership-capacity.sqlite");
    let crawler = Crawler::new(CrawlConfig {
        sitemap_urls: vec!["https://example.test/sitemap.xml".into()],
        discover_sitemaps: false,
        sitemap_membership_limits: Some(declared_limits()),
        ..Default::default()
    })
    .unwrap()
    .with_store(&file)
    .unwrap();
    let store = crawler.store.as_ref().unwrap();
    store.prepare(crawler.effective_config()).unwrap();
    let generation = store.begin_run("membership-capacity-config").unwrap();
    crawler.initialize_original_sitemaps(generation).unwrap();
    let before = store.sitemap_catalog().unwrap().unwrap();
    assert_eq!(before.summary.limits, declared_limits());
    assert_eq!(
        before.summary.config_sha256,
        store.saved_config_bytes_sha256().unwrap()
    );
    let saved = store.config().unwrap().unwrap();
    assert_eq!(saved.sitemap_membership_limits, Some(declared_limits()));
    let mut operational = saved.clone();
    operational.concurrency = 2;
    assert!(store.prepare(&operational).unwrap());
    let mut altered = operational.clone();
    altered.sitemap_membership_limits = None;
    assert!(store.prepare(&altered).is_err());
    assert_eq!(store.sitemap_catalog().unwrap().unwrap(), before);
    drop(crawler);
    let reopened = crate::CrawlStore::open_read_only(&file).unwrap();
    assert_eq!(reopened.sitemap_catalog().unwrap().unwrap(), before);
    assert_eq!(
        reopened
            .config()
            .unwrap()
            .unwrap()
            .sitemap_membership_limits,
        Some(declared_limits())
    );
}
