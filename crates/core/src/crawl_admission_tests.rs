//! Authored source regressions; not compiled or run by the author.
use super::*;
use crate::{CrawlConfig, Crawler, PageRecord};

fn config(
    folder: Option<usize>,
    query: Option<usize>,
    bytes: Option<usize>,
) -> CrawlAdmissionLimits {
    CrawlAdmissionLimits {
        max_folder_depth: folder,
        max_query_tokens: query,
        max_fetch_url_utf8_bytes: bytes,
        ..Default::default()
    }
}
fn assess(limits: &CrawlAdmissionLimits, value: &str) -> CrawlAdmissionAssessment {
    limits.assess(
        &crate::normalize_url(value, None).unwrap(),
        CrawlAdmissionPhase::Initial,
    )
}

#[test]
fn static_limits_versioned_object_rejects_shapes_unknowns_duplicates_and_bad_ranges() {
    for raw in [
        "[]",
        "[1,0,0,1]",
        "true",
        "0",
        "\"limits\"",
        "{}",
        "{\"version\":2,\"max_query_tokens\":0}",
        "{\"version\":1}",
        "{\"version\":1,\"max_query_tokens\":null}",
        "{\"version\":1,\"max_query_tokens\":-1}",
        "{\"version\":1,\"max_query_tokens\":1.5}",
        "{\"version\":1,\"max_query_tokens\":\"1\"}",
        "{\"version\":1,\"max_folder_depth\":65536}",
        "{\"version\":1,\"max_query_tokens\":65536}",
        "{\"version\":1,\"max_fetch_url_utf8_bytes\":0}",
        "{\"version\":1,\"max_fetch_url_utf8_bytes\":1048577}",
        "{\"version\":1,\"max_query_tokens\":0,\"unknown\":true}",
        "{\"version\":1,\"max_query_tokens\":0,\"max_query_tokens\":1}",
    ] {
        assert!(
            serde_json::from_str::<CrawlAdmissionLimits>(raw).is_err(),
            "{raw}"
        );
    }
    let zero: CrawlAdmissionLimits =
        serde_json::from_str(r#"{"version":1,"max_folder_depth":0,"max_query_tokens":0}"#).unwrap();
    assert_eq!(zero, config(Some(0), Some(0), None));
    assert!(
        config(Some(65535), Some(65535), Some(1048576))
            .validate()
            .is_ok()
    );
}

#[test]
fn static_limits_literal_folder_boundaries_count_empty_segments_without_decoding_slashes() {
    for (path, depth) in [
        ("/", 0),
        ("/a", 0),
        ("/a/", 1),
        ("/a/b", 1),
        ("/a//b", 2),
        ("//", 1),
        ("/a%2Fb", 0),
        ("/a%2fb/", 1),
        ("/ä/日", 1),
    ] {
        let url = format!("https://example.test{path}");
        let at = assess(&config(Some(depth), None, None), &url);
        assert_eq!(at.folder_depth, depth, "{path}");
        assert_eq!(at.denied, None, "equal {path}");
        assert_eq!(
            assess(&config(Some(depth + 1), None, None), &url).denied,
            None,
            "minus {path}"
        );
        if depth > 0 {
            assert_eq!(
                assess(&config(Some(depth - 1), None, None), &url).denied,
                Some(CrawlAdmissionDenial::FolderDepth),
                "plus {path}"
            );
        }
    }
}

#[test]
fn static_limits_raw_query_tokens_preserve_duplicate_and_empty_tokens_and_encoded_delimiters() {
    for (suffix, count) in [
        ("", 0),
        ("?", 1),
        ("?a", 1),
        ("?a=1&a=2", 2),
        ("?&&", 3),
        ("?a&", 2),
        ("?a%26b=1;x=2", 1),
        ("?x=%26&x=ä", 2),
    ] {
        let url = format!("https://example.test/{suffix}");
        assert_eq!(
            assess(&config(None, Some(count), None), &url).query_tokens,
            count,
            "{suffix}"
        );
        assert_eq!(
            assess(&config(None, Some(count), None), &url).denied,
            None,
            "equal {suffix}"
        );
        assert_eq!(
            assess(&config(None, Some(count + 1), None), &url).denied,
            None,
            "minus {suffix}"
        );
        if count > 0 {
            assert_eq!(
                assess(&config(None, Some(count - 1), None), &url).denied,
                Some(CrawlAdmissionDenial::QueryTokens),
                "plus {suffix}"
            );
        }
    }
}

#[test]
fn static_limits_fetch_byte_boundary_uses_normalized_idna_encoding_and_excludes_fragment() {
    let original = "https://bücher.example/ä?q=日#retained-original";
    let effective = crate::normalize_url(original, None).unwrap();
    assert_eq!(
        effective.as_str(),
        "https://xn--bcher-kva.example/%C3%A4?q=%E6%97%A5"
    );
    let bytes = effective.as_str().len();
    assert!(bytes > original.split('#').next().unwrap().chars().count());
    let at = assess(&config(None, None, Some(bytes)), original);
    assert_eq!(at.fetch_url_utf8_bytes, bytes);
    assert_eq!(at.denied, None);
    assert_eq!(
        assess(&config(None, None, Some(bytes + 1)), original).denied,
        None
    );
    assert_eq!(
        assess(&config(None, None, Some(bytes - 1)), original).denied,
        Some(CrawlAdmissionDenial::FetchUrlUtf8Bytes)
    );
    assert_eq!(
        at.fetch_url_sha256,
        format!("{:x}", Sha256::digest(effective.as_str().as_bytes()))
    );
    assert_eq!(original, "https://bücher.example/ä?q=日#retained-original");
}

#[test]
fn static_limits_none_null_config_and_legacy_page_report_bytes_and_hashes_match() {
    let absent: CrawlConfig =
        serde_json::from_str(r#"{"seed_urls":["https://example.test/"]}"#).unwrap();
    let with_null: CrawlConfig = serde_json::from_str(
        r#"{"seed_urls":["https://example.test/"],"crawl_admission_limits":null}"#,
    )
    .unwrap();
    let a = serde_json::to_vec(&absent).unwrap();
    let b = serde_json::to_vec(&with_null).unwrap();
    assert_eq!(a, b);
    assert_eq!(Sha256::digest(&a), Sha256::digest(&b));
    assert!(
        !serde_json::to_value(&absent)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("crawl_admission_limits")
    );
    let page: PageRecord =
        serde_json::from_str(r#"{"url":"https://example.test/","status":200,"final_status":200}"#)
            .unwrap();
    let mut explicit: PageRecord =
        serde_json::from_value(serde_json::to_value(&page).unwrap()).unwrap();
    explicit.crawl_admission = None;
    assert!(
        !serde_json::to_value(&page)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("crawl_admission")
    );
    let report_a = serde_json::json!({"config":absent,"pages":[page]});
    let report_b = serde_json::json!({"config":with_null,"pages":[explicit]});
    let a = serde_json::to_vec(&report_a).unwrap();
    let b = serde_json::to_vec(&report_b).unwrap();
    assert_eq!(a, b);
    assert_eq!(Sha256::digest(&a), Sha256::digest(&b));
}

#[test]
fn static_limits_config_is_durable_immutable_on_resume_and_renderer_is_refused_before_store() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("limits.sqlite");
    let cfg = CrawlConfig {
        seed_urls: vec!["https://example.test/".into()],
        crawl_admission_limits: Some(config(Some(0), Some(2), Some(8192))),
        ..Default::default()
    };
    let crawler = Crawler::new(cfg.clone()).unwrap();
    let effective = crawler.effective_config().clone();
    let store = crate::CrawlStore::open(&db).unwrap();
    store.prepare(&effective).unwrap();
    let old = store.saved_config_bytes_sha256().unwrap();
    let mut operational = effective.clone();
    operational.concurrency = 2;
    assert!(store.prepare(&operational).unwrap());
    let mut changed = effective.clone();
    changed.crawl_admission_limits = None;
    assert!(store.prepare(&changed).is_err());
    assert_eq!(
        store.config().unwrap().unwrap().crawl_admission_limits,
        cfg.crawl_admission_limits
    );
    drop(store);
    let reopened = crate::CrawlStore::open_read_only(&db).unwrap();
    assert_eq!(
        reopened.config().unwrap().unwrap().crawl_admission_limits,
        cfg.crawl_admission_limits
    );
    // The existing operational override intentionally changes stored config;
    // compare its real bytes/hash with a second read, not the original SHA.
    assert_ne!(reopened.saved_config_bytes_sha256().unwrap(), old);
    let mut unsupported = cfg;
    unsupported.renderer = Some(crate::RendererConfig::default());
    let fresh = dir.path().join("must-not-exist.sqlite");
    assert!(
        Crawler::new(unsupported)
            .and_then(|c| c.with_store(&fresh))
            .is_err()
    );
    assert!(!fresh.exists());
}
