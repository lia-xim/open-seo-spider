use super::*;
use serde_json::json;

fn configuration(method: &str, pattern: &str) -> CustomSearchConfig {
    serde_json::from_value(json!({"version":1,"rules":[{"name":"owned","method":method,"pattern":pattern,"sources":["raw_html"]}]})).unwrap()
}
fn complete(text: &str) -> CustomSearchInput<'_> {
    CustomSearchInput {
        text: Some(text),
        complete: true,
        ..Default::default()
    }
}
fn observed(config: &CustomSearchConfig, text: &str) -> CustomSearchResult {
    config
        .compile()
        .unwrap()
        .observe(CustomSearchSource::RawHtml, complete(text))
        .remove(0)
}

#[test]
fn custom_search_literal_unicode_offsets_case_and_nonoverlap_are_exact() {
    let config = configuration("literal", "😀cat");
    let result = observed(&config, "é😀cat / 😀cat CAT");
    assert_eq!(result.count, Some(2));
    assert!(result.evidence_complete);
    assert_eq!(
        result
            .matches
            .iter()
            .map(|m| (m.start_byte, m.end_byte, m.text.as_str()))
            .collect::<Vec<_>>(),
        vec![(2, 9, "😀cat"), (12, 19, "😀cat")]
    );
    assert_eq!(result.input_utf8_bytes, Some(23));
    assert_eq!(
        result.input_sha256,
        Some(hash("é😀cat / 😀cat CAT".as_bytes()))
    );
    assert_eq!(
        observed(&configuration("literal", "aa"), "aaaaa").count,
        Some(2)
    );
    assert_eq!(
        observed(&configuration("literal", "CAT"), "cat").count,
        Some(0)
    );
    config
        .compile()
        .unwrap()
        .validate_results(&[result])
        .unwrap();
}

#[test]
fn custom_search_regex_flags_zero_width_and_known_empty_remain_exact() {
    let mut config = configuration("regex", "^cat$");
    config.rules[0].regex_flags = "im".into();
    assert_eq!(observed(&config, "CAT\nCat\ndog").count, Some(2));
    assert_eq!(
        observed(&configuration("regex", ""), "😀x")
            .matches
            .iter()
            .map(|m| m.start_byte)
            .collect::<Vec<_>>(),
        vec![0, 4, 5]
    );
    assert_eq!(observed(&configuration("regex", ""), "").count, Some(1));
    let empty = observed(&configuration("literal", "cat"), "");
    assert_eq!(empty.count, Some(0));
    assert!(empty.evidence_complete);
    assert_eq!(empty.status, CustomSearchStatus::Complete);
    let mut dot = configuration("regex", "a.*z");
    dot.rules[0].regex_flags = "sU".into();
    assert_eq!(observed(&dot, "a\nz").count, Some(1));
    let mut spaced = configuration("regex", "a  z # comment");
    spaced.rules[0].regex_flags = "x".into();
    assert_eq!(observed(&spaced, "az").count, Some(1));
}

#[test]
fn custom_search_unknown_sources_never_become_successful_zeroes() {
    let compiled = configuration("literal", "needle").compile().unwrap();
    let missing = compiled.observe(
        CustomSearchSource::RawHtml,
        CustomSearchInput {
            reason: Some("renderer_unavailable"),
            ..Default::default()
        },
    );
    assert_eq!(missing[0].status, CustomSearchStatus::Unavailable);
    assert_eq!(missing[0].count, None);
    assert_eq!(missing[0].input_utf8_bytes, None);
    assert_eq!(missing[0].input_sha256, None);
    assert_eq!(missing[0].matches_observed, 0);
    assert!(!missing[0].evidence_complete);
    compiled.validate_results(&missing).unwrap();
    let partial = compiled.observe(
        CustomSearchSource::RawHtml,
        CustomSearchInput {
            text: Some("needle"),
            complete: false,
            reason: Some("body_truncated"),
            ..Default::default()
        },
    );
    assert_eq!(partial[0].count, None);
    assert_eq!(partial[0].matches_observed, 1);
    assert_eq!(partial[0].status, CustomSearchStatus::Incomplete);
    compiled.validate_results(&partial).unwrap();
    assert!(
        compiled
            .observe(CustomSearchSource::RenderedHtml, complete("needle"))
            .is_empty()
    );
}

#[test]
fn custom_search_input_and_scan_caps_do_not_claim_exact_totals() {
    let mut config = configuration("literal", "a");
    config.limits.max_input_bytes = 3;
    let result = observed(&config, "aaaaa");
    assert_eq!(result.count, None);
    assert_eq!(result.matches_observed, 3);
    assert_eq!(result.searched_utf8_bytes, 3);
    assert!(result.reasons.contains(&"input_byte_limit".into()));
    config.limits.max_input_bytes = 1;
    let unicode = observed(&config, "😀a");
    assert_eq!(unicode.searched_utf8_bytes, 0);
    assert_eq!(unicode.count, None);
    config.limits.max_input_bytes = 100;
    config.limits.max_matches_scanned = 2;
    for input in ["aa", "aaaa"] {
        let out = observed(&config, input);
        assert_eq!(out.count, None);
        assert_eq!(out.matches_observed, 2);
        assert!(out.reasons.contains(&"match_scan_limit".into()));
    }
    assert_eq!(observed(&config, "a").count, Some(1));
}

#[test]
fn custom_search_partial_regex_context_does_not_fabricate_anchors_or_alternation() {
    for (pattern, text, cut) in [
        ("a.*z|a", "aXXXz", 2),
        ("foo$", "foobar", 3),
        ("foo\\b", "foobar", 3),
        ("a+", "aaaa", 2),
    ] {
        let mut config = configuration("regex", pattern);
        config.limits.max_input_bytes = cut;
        let result = observed(&config, text);
        assert_eq!(result.status, CustomSearchStatus::Incomplete);
        assert_eq!(result.matches_observed, 0);
        assert!(result.matches.is_empty());
        assert_eq!(result.count, None);
        assert_eq!(result.searched_utf8_bytes, 0);
        assert!(
            result
                .reasons
                .contains(&"regex_partial_context_unknown".into())
        );
        config.limits.max_input_bytes = 100;
        let partial = config.compile().unwrap().observe(
            CustomSearchSource::RawHtml,
            CustomSearchInput {
                text: Some(text),
                complete: false,
                ..Default::default()
            },
        );
        assert!(partial[0].matches.is_empty());
        assert_eq!(partial[0].count, None);
        config
            .compile()
            .unwrap()
            .validate_results(&partial)
            .unwrap();
    }
}

#[test]
fn custom_search_regex_work_budget_keeps_full_context_and_charges_eof() {
    let mut config = configuration("regex", "a.*z|a");
    config.limits.max_regex_scan_work_bytes = 6;
    let partial = observed(&config, "aXXXza");
    assert_eq!(partial.count, None);
    assert_eq!(partial.matches_observed, 1);
    assert_eq!(partial.matches[0].text, "aXXXz");
    assert!(partial.reasons.contains(&"regex_scan_work_limit".into()));
    assert!(!partial.reasons.contains(&"match_scan_limit".into()));
    config
        .compile()
        .unwrap()
        .validate_results(&[partial])
        .unwrap();
    config.limits.max_regex_scan_work_bytes = 7;
    assert_eq!(observed(&config, "aXXXza").matches_observed, 2);
    assert_eq!(
        observed(&config, "aXXXza").count,
        None,
        "Final EOF probe also consumes work"
    );
    config.limits.max_regex_scan_work_bytes = 8;
    assert_eq!(observed(&config, "aXXXza").count, Some(2));

    let mut zero = configuration("regex", "");
    zero.limits.max_regex_scan_work_bytes = 11;
    let partial = observed(&zero, "😀x");
    assert_eq!(partial.matches_observed, 3);
    assert_eq!(partial.count, None);
    zero.limits.max_regex_scan_work_bytes = 12;
    assert_eq!(observed(&zero, "😀x").count, Some(3));

    let mut absent = configuration("regex", "z");
    absent.limits.max_regex_scan_work_bytes = 2;
    assert_eq!(observed(&absent, "abc").count, None);
    absent.limits.max_regex_scan_work_bytes = 3;
    assert_eq!(observed(&absent, "abc").count, Some(0));
    let mut literal = configuration("literal", "a");
    literal.limits.max_regex_scan_work_bytes = 1;
    assert_eq!(
        observed(&literal, "a".repeat(1000).as_str()).count,
        Some(1000)
    );
}

#[test]
fn custom_search_saved_evidence_caps_preserve_exact_counts() {
    let mut config = configuration("literal", "cat");
    config.limits.max_saved_matches = 1;
    let result = observed(&config, "cat cat cat");
    assert_eq!(result.count, Some(3));
    assert_eq!(result.matches.len(), 1);
    assert!(!result.evidence_complete);
    assert_eq!(result.status, CustomSearchStatus::Complete);
    config.limits.max_saved_matches = 0;
    let count_only = observed(&config, "cat");
    assert_eq!(count_only.count, Some(1));
    assert!(count_only.matches.is_empty());
    assert!(!count_only.evidence_complete);
    assert!(observed(&config, "").evidence_complete);
    config
        .compile()
        .unwrap()
        .validate_results(&[count_only])
        .unwrap();
}

#[test]
fn custom_search_huge_match_and_unicode_excerpt_are_independently_bounded() {
    let mut config = configuration("regex", ".+");
    config.limits.max_excerpt_bytes = 5;
    let long = "😀".repeat(1000);
    let result = observed(&config, &long);
    assert_eq!(result.count, Some(1));
    let evidence = &result.matches[0];
    assert_eq!(evidence.match_utf8_bytes, 4000);
    assert_eq!(evidence.text, "😀");
    assert!(evidence.text_truncated);
    assert_eq!(evidence.excerpt, "😀");
    assert!(!result.evidence_complete);
    config
        .compile()
        .unwrap()
        .validate_results(&[result])
        .unwrap();
    let mut middle = configuration("literal", "cat");
    middle.limits.max_excerpt_bytes = 8;
    let result = observed(&middle, "é😀cat😀é");
    let evidence = &result.matches[0];
    assert!(evidence.excerpt.len() <= 8);
    assert!(evidence.excerpt.contains("cat"));
    middle
        .compile()
        .unwrap()
        .validate_results(&[result])
        .unwrap();
    middle.limits.max_excerpt_bytes = 1;
    middle.rules[0].pattern = "😀".into();
    let result = observed(&middle, "😀");
    assert_eq!(result.matches[0].text, "");
    assert!(result.matches[0].text_truncated);
    middle
        .compile()
        .unwrap()
        .validate_results(&[result])
        .unwrap();
}

#[test]
fn custom_search_all_rule_source_outputs_fit_a_hard_aggregate_budget() {
    let mut config = configuration("literal", "x");
    config.rules = (0..32)
        .map(|i| CustomSearchRule {
            name: format!("rule-{i}"),
            sources: vec![
                CustomSearchSource::RawHtml,
                CustomSearchSource::RenderedHtml,
                CustomSearchSource::RawSelectedText,
                CustomSearchSource::RenderedSelectedText,
            ],
            ..config.rules[0].clone()
        })
        .collect();
    config.limits.max_output_json_bytes = 256 * 1024;
    config.limits.max_saved_matches = 256;
    config.limits.max_excerpt_bytes = 4096;
    let compiled = config.compile().unwrap();
    let text = "x".repeat(1000);
    let mut all = Vec::new();
    for source in [
        CustomSearchSource::RawHtml,
        CustomSearchSource::RenderedHtml,
        CustomSearchSource::RawSelectedText,
        CustomSearchSource::RenderedSelectedText,
    ] {
        all.extend(compiled.observe(source, complete(&text)));
    }
    compiled.enforce_output_budget(&mut all).unwrap();
    assert_eq!(all.len(), 128);
    assert!(json_bytes(&all).unwrap() <= 256 * 1024);
    assert!(
        all.iter()
            .all(|r| r.count == Some(1000) && !r.evidence_complete)
    );
    compiled.validate_results(&all).unwrap();
    let mut low = config;
    low.limits.max_output_json_bytes = 64 * 1024;
    assert!(low.compile().is_err());
}

#[test]
fn custom_search_metadata_admission_is_explicitly_unavailable_and_bounded() {
    let compiled = configuration("literal", "x").compile().unwrap();
    let url = "x".repeat(MAX_DOCUMENT_URL_BYTES + 1);
    let why = "é".repeat(500);
    let revision = "y".repeat(MAX_REVISION_BYTES + 1);
    let result = compiled.observe(
        CustomSearchSource::RawHtml,
        CustomSearchInput {
            text: Some("x"),
            complete: true,
            document_url: Some(&url),
            source_revision: Some(&revision),
            reason: Some(&why),
            ..Default::default()
        },
    );
    assert_eq!(result[0].status, CustomSearchStatus::Unavailable);
    assert_eq!(result[0].count, None);
    assert_eq!(result[0].document_url, None);
    assert_eq!(result[0].source_revision, None);
    assert_eq!(result[0].reasons, vec!["source_metadata_budget"]);
    compiled.validate_results(&result).unwrap();
}

#[test]
fn custom_search_configuration_rejects_unknown_fields_shapes_and_invalid_patterns() {
    let valid = json!({"version":1,"rules":[{"name":"one","method":"literal","pattern":"x","sources":["raw_html"]}]});
    for value in [
        json!([1, [], {}]),
        json!({"rules":[]}),
        json!({"version":1,"rules":[],"surprise":true}),
    ] {
        assert!(serde_json::from_value::<CustomSearchConfig>(value).is_err());
    }
    let mut wrong_version = configuration("literal", "x");
    wrong_version.version = 2;
    assert!(wrong_version.compile().is_err());
    let mut unknown = valid.clone();
    unknown["rules"][0]["extra"] = json!(true);
    assert!(serde_json::from_value::<CustomSearchConfig>(unknown).is_err());
    let mut unknown = valid.clone();
    unknown["limits"] = json!({"extra":1});
    assert!(serde_json::from_value::<CustomSearchConfig>(unknown).is_err());
    let mut arrays = valid.clone();
    arrays["limits"] = json!([1, 1, 1, 1, 65536]);
    assert!(serde_json::from_value::<CustomSearchConfig>(arrays).is_err());
    for pattern in ["(", "(?=x)", "(x)\\1"] {
        assert!(configuration("regex", pattern).compile().is_err());
    }
    for flags in ["ii", "z", "imsxUU"] {
        let mut c = configuration("regex", "x");
        c.rules[0].regex_flags = flags.into();
        assert!(c.compile().is_err());
    }
    assert!(configuration("literal", "").compile().is_err());
    let mut literal = configuration("literal", "x");
    literal.rules[0].regex_flags = "i".into();
    assert!(literal.compile().is_err());
    let mut duplicate = configuration("literal", "x");
    duplicate.rules.push(duplicate.rules[0].clone());
    assert!(duplicate.compile().is_err());
    let mut duplicate = configuration("literal", "x");
    duplicate.rules[0].sources.push(CustomSearchSource::RawHtml);
    assert!(duplicate.compile().is_err());
    let mut huge = configuration("literal", &"x".repeat(8193));
    assert!(huge.compile().is_err());
    huge.rules[0].pattern = "x".into();
    huge.rules[0].name = " ".into();
    assert!(huge.compile().is_err());
}

#[test]
fn custom_search_invalid_limits_are_refused_before_observation() {
    for (field, value) in [
        ("max_input_bytes", 0),
        ("max_input_bytes", 8 * 1024 * 1024 + 1),
        ("max_matches_scanned", 0),
        ("max_matches_scanned", 1_000_001),
        ("max_saved_matches", 257),
        ("max_excerpt_bytes", 0),
        ("max_excerpt_bytes", 4097),
        ("max_output_json_bytes", 65535),
        ("max_output_json_bytes", CUSTOM_SEARCH_MAX_OUTPUT_BYTES + 1),
    ] {
        let mut v = serde_json::to_value(configuration("literal", "x")).unwrap();
        v["limits"][field] = json!(value);
        assert!(
            serde_json::from_value::<CustomSearchConfig>(v)
                .unwrap()
                .compile()
                .is_err(),
            "{field} {value}"
        );
    }
    for value in [0, CUSTOM_SEARCH_MAX_REGEX_SCAN_WORK_BYTES + 1] {
        let mut v = serde_json::to_value(configuration("literal", "x")).unwrap();
        v["limits"]["max_regex_scan_work_bytes"] = json!(value);
        assert!(
            serde_json::from_value::<CustomSearchConfig>(v)
                .unwrap()
                .compile()
                .is_err()
        );
    }
}

#[test]
fn custom_search_portable_validation_rejects_forged_profiles_counts_and_spans() {
    let compiled = configuration("literal", "cat").compile().unwrap();
    let original = compiled.observe(CustomSearchSource::RawHtml, complete("cat"));
    let mut inherited = original.clone();
    inherited[0].observed_generation = Some(3);
    compiled.validate_results(&inherited).unwrap();
    let mut bad = original.clone();
    bad[0].config_sha256 = "0".repeat(64);
    assert!(compiled.validate_results(&bad).is_err());
    let mut bad = original.clone();
    bad[0].count = Some(0);
    assert!(compiled.validate_results(&bad).is_err());
    let mut bad = original.clone();
    bad[0].matches[0].end_byte = 100;
    assert!(compiled.validate_results(&bad).is_err());
    let mut bad = original.clone();
    bad[0].matches[0].excerpt = "different".into();
    assert!(compiled.validate_results(&bad).is_err());
    let mut duplicate = original.clone();
    duplicate.extend(original.clone());
    assert!(compiled.enforce_output_budget(&mut duplicate).is_err());
    let mut changed = configuration("literal", "cat");
    changed.limits.max_saved_matches = 0;
    assert!(
        changed
            .compile()
            .unwrap()
            .validate_results(&original)
            .is_err()
    );
}

#[test]
fn custom_search_portable_validation_rejects_duplicate_zero_spans_and_impossible_counts() {
    let empty_regex = configuration("regex", "").compile().unwrap();
    let mut empty = empty_regex.observe(CustomSearchSource::RawHtml, complete(""));
    let duplicate = empty[0].matches[0].clone();
    empty[0].matches.push(duplicate);
    empty[0].count = Some(2);
    empty[0].matches_observed = 2;
    assert!(
        empty_regex.validate_results(&empty).is_err(),
        "Empty source has only one possible zero-width position"
    );
    let mut repeated = empty_regex.observe(CustomSearchSource::RawHtml, complete("abc"));
    let first = repeated[0].matches[0].clone();
    repeated[0].matches = vec![first.clone(), first];
    repeated[0].count = Some(2);
    repeated[0].matches_observed = 2;
    assert!(
        empty_regex.validate_results(&repeated).is_err(),
        "Even with enough total bytes, a repeated zero span is impossible"
    );
    for (pattern, text, expected) in [
        ("^|a", "ba", vec![(0, 0), (1, 2)]),
        ("a|$", "ab", vec![(0, 1), (2, 2)]),
        ("a|b", "ab", vec![(0, 1), (1, 2)]),
        ("", "ab", vec![(0, 0), (1, 1), (2, 2)]),
    ] {
        let compiled = configuration("regex", pattern).compile().unwrap();
        let result = compiled.observe(CustomSearchSource::RawHtml, complete(text));
        assert_eq!(
            result[0]
                .matches
                .iter()
                .map(|m| (m.start_byte, m.end_byte))
                .collect::<Vec<_>>(),
            expected
        );
        compiled.validate_results(&result).unwrap();
    }
    let literal = configuration("literal", "cat").compile().unwrap();
    let mut impossible = literal.observe(CustomSearchSource::RawHtml, complete("cat"));
    impossible[0].count = Some(2);
    impossible[0].matches_observed = 2;
    impossible[0].evidence_complete = false;
    assert!(literal.validate_results(&impossible).is_err());
    let mut wrong_text = literal.observe(CustomSearchSource::RawHtml, complete("cat"));
    wrong_text[0].matches[0].text = "dog".into();
    wrong_text[0].matches[0].excerpt = "dog".into();
    assert!(
        literal.validate_results(&wrong_text).is_err(),
        "Portable literal match text must agree with the known pattern"
    );
}

#[test]
fn custom_search_legacy_defaults_preserve_absent_config_and_page_fields() {
    let config = serde_json::to_value(crate::CrawlConfig::default()).unwrap();
    assert!(config.get("custom_search").is_none());
    let page = serde_json::to_value(crate::PageRecord::default()).unwrap();
    assert!(page.get("custom_search").is_none());
    assert!(
        serde_json::from_value::<crate::CrawlConfig>(json!({}))
            .unwrap()
            .custom_search
            .is_none()
    );
    assert!(
        serde_json::from_value::<crate::PageRecord>(json!({}))
            .unwrap()
            .custom_search
            .is_none()
    );
    let config = configuration("literal", "x");
    let out = observed(&config, "x");
    assert_eq!(
        serde_json::from_value::<CustomSearchResult>(serde_json::to_value(&out).unwrap()).unwrap(),
        out
    );
}
