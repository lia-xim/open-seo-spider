//! Legacy CLI config files are raw CrawlConfig JSON, not portable profiles.
use anyhow::{Result, bail};
use open_seo_spider_core::CrawlConfig;
use serde::de::{Deserializer, IgnoredAny, MapAccess, Visitor};
use std::fmt;

const PROFILE_GUIDANCE: &str = "Versioned configuration profiles are not supported by crawl --config. Import the profile in browser Settings or use /api/crawls/from-profile with explicit header bindings. --config requires raw CrawlConfig JSON.";

pub(super) fn parse_raw_config(bytes: &[u8]) -> Result<CrawlConfig> {
    let mut markers = ProfileMarkers::default();
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    // Inspect only top-level keys. IgnoredAny keeps legacy unknown-value depth
    // and number handling; it does not allocate a second complete JSON Value.
    // Observed markers survive a later parse error, including truncated values.
    let _ = decoder.deserialize_map(MarkerVisitor(&mut markers));
    if markers.config && (markers.version || markers.required_headers) {
        bail!(PROFILE_GUIDANCE);
    }
    // Always parse the ORIGINAL bytes. A Value roundtrip would hide duplicate
    // known fields that the existing typed parser correctly rejects.
    Ok(serde_json::from_slice::<CrawlConfig>(bytes)?)
}

#[derive(Default)]
struct ProfileMarkers {
    version: bool,
    config: bool,
    required_headers: bool,
}

struct MarkerVisitor<'a>(&'a mut ProfileMarkers);
impl<'de> Visitor<'de> for MarkerVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a legacy crawl configuration object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "version" => self.0.version = true,
                "config" => self.0.config = true,
                "required_headers" => self.0.required_headers = true,
                _ => (),
            }
            map.next_value::<IgnoredAny>()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_native_options_and_free_nested_markers_keep_the_legacy_contract() {
        let raw = br#"{"concurrency":65,"request_timeout_secs":301,"max_body_bytes":1572865,"max_redirects":3,"discover_sitemaps":false,"crawl_links":false,"custom_headers":{"config":"owned-value","version":"owned-value"},"unknown_legacy":{"version":1,"config":{},"required_headers":[]}}"#;
        let parsed = parse_raw_config(raw).unwrap();
        assert_eq!(parsed.concurrency, 65);
        assert_eq!(parsed.request_timeout_secs, 301);
        assert_eq!(parsed.max_body_bytes, 1572865);
        assert_eq!(parsed.max_redirects, 3);
        assert!(!parsed.discover_sitemaps);
        assert!(!parsed.crawl_links);
        assert_eq!(parsed.custom_headers["config"], "owned-value");
        for raw in [
            br#"{"version":1,"concurrency":7}"#.as_slice(),
            br#"{"config":{},"concurrency":7}"#.as_slice(),
            br#"{"required_headers":[],"concurrency":7}"#.as_slice(),
            br#"{"unknown_legacy":1e999,"concurrency":7}"#.as_slice(),
        ] {
            assert_eq!(parse_raw_config(raw).unwrap().concurrency, 7);
        }
    }

    #[test]
    fn duplicate_known_fields_and_original_typed_errors_are_not_collapsed() {
        for raw in [
            br#"{"concurrency":1,"concurrency":2}"#.as_slice(),
            br#"{"renderer":{"endpoint":"http://127.0.0.1:9876/render","wait_ms":1,"wait_ms":2}}"#
                .as_slice(),
            br#"{"concurrency":"wrong-type"}"#.as_slice(),
            br#"{"concurrency":7} trailing"#.as_slice(),
            br#"{"concurrency":"#.as_slice(),
        ] {
            let expected = serde_json::from_slice::<CrawlConfig>(raw).unwrap_err();
            assert_eq!(
                parse_raw_config(raw).unwrap_err().to_string(),
                expected.to_string()
            );
        }
    }

    #[test]
    fn recognizable_v1_v2_wrong_shapes_and_partial_profiles_are_explicitly_refused() {
        for raw in [
            br#"{"version":1,"config":{},"required_headers":[]}"#.as_slice(),
            br#"{"version":2,"config":{}}"#.as_slice(),
            br#"{"version":null,"config":[]}"#.as_slice(),
            br#"{"config":null,"required_headers":null}"#.as_slice(),
            br#"{"required_headers":["X-Owned-Required"],"config":{}}"#.as_slice(),
            br#"{"version":1,"version":2,"config":{}}"#.as_slice(),
            br#"{"version":1,"config":"#.as_slice(),
            br#"{"config":{},"required_headers":"#.as_slice(),
        ] {
            let error = parse_raw_config(raw).unwrap_err().to_string();
            assert_eq!(error, PROFILE_GUIDANCE);
            assert!(error.contains("browser Settings"));
            assert!(error.contains("/api/crawls/from-profile"));
        }
    }

    #[test]
    fn deep_ignored_values_do_not_change_legacy_parsing_or_hide_profile_markers() {
        let nested = format!("{}0{}", "[".repeat(512), "]".repeat(512));
        let raw = format!(r#"{{"unknown_legacy":{nested},"concurrency":7}}"#);
        let expected = serde_json::from_slice::<CrawlConfig>(raw.as_bytes()).unwrap();
        assert_eq!(
            parse_raw_config(raw.as_bytes()).unwrap().concurrency,
            expected.concurrency
        );
        for raw in [
            format!(r#"{{"version":1,"config":{nested}}}"#),
            format!(r#"{{"config":{nested},"required_headers":[]}}"#),
        ] {
            assert_eq!(
                parse_raw_config(raw.as_bytes()).unwrap_err().to_string(),
                PROFILE_GUIDANCE
            );
        }
    }
}
