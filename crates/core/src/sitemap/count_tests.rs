//! Independent fixture expectations; AUTHORED / NOT RUN under the compute hold.
use super::*;
use sha2::{Digest, Sha256};

const NS: &str = "http://www.sitemaps.org/schemas/sitemap/0.9";
fn scan(xml: &str) -> SitemapCountObservation {
    count_sitemap_xml(xml.as_bytes(), &SitemapCountLimits::default()).unwrap()
}
fn complete(xml: &str) -> SitemapCountObservation {
    let value = scan(xml);
    assert!(value.xml_scan_complete, "{:?}", value.failure);
    assert!(value.failure.is_none());
    assert_eq!(value.input_xml_bytes, xml.len());
    assert_eq!(value.input_xml_sha256, format!("{:x}", Sha256::digest(xml)));
    value
}
fn urlset(body: &str) -> String {
    format!("<urlset xmlns='{NS}'>{body}</urlset>")
}
fn loc(value: &str) -> String {
    format!("<url><loc>{value}</loc></url>")
}
fn rejected_retained(xml: &str, limits: &SitemapParserLimits) -> String {
    match observe_sitemap_xml(xml.as_bytes(), SitemapParserContext::default(), limits) {
        Err(error) => error.code,
        Ok(value) => {
            assert!(!value.complete);
            assert_eq!(value.admission_locations().count(), 0);
            value.failure.unwrap().code
        }
    }
}

#[test]
fn count_namespaces_extensions_duplicates_and_legacy_are_separate() {
    let xml = format!(
        "<s:urlset xmlns:s='{NS}' xmlns:f='urn:foreign'><!-- <s:url/> -->\
        <f:url><f:loc>https://ignored.test/</f:loc></f:url>\
        <s:url><s:loc>https://e.test/repeated</s:loc></s:url>\
        <s:url><s:loc>https://e.test/repeated</s:loc></s:url>\
        <f:wrap><![CDATA[<s:url><s:loc>https://fake.test/</s:loc></s:url>]]>\
        <f:url><f:loc>https://also-ignored.test/</f:loc></f:url></f:wrap></s:urlset>"
    );
    let value = complete(&xml);
    assert_eq!(
        value.namespace_profile,
        Some(SitemapNamespaceProfile::Sitemap09)
    );
    assert_eq!(value.kind, Some(SitemapDocumentKind::UrlSet));
    assert_eq!(value.counts.immediate_entry_starts, 2);
    assert_eq!(value.counts.completed_immediate_entries, 2);
    assert_eq!(value.counts.started_document_namespace_loc_elements, 2);
    assert_eq!(value.counts.protocol_eligible_completed_entries, 2);
    assert_eq!(value.counts.ignored_nonfamily_entry_elements, 2);
    assert_eq!(value.counts.ignored_nonfamily_loc_elements, 2);
    let mutation = complete(&format!(
        "<urlset xmlns='{NS}' xmlns:s='{NS}'><url xmlns='urn:changed'><loc>https://foreign.test/</loc></url><s:url><s:loc>https://e.test/</s:loc></s:url></urlset>"
    ));
    assert_eq!(mutation.counts.protocol_eligible_completed_entries, 1);
    assert_eq!(mutation.counts.ignored_nonfamily_entry_elements, 1);
    let legacy = complete("<urlset><url><loc>https://e.test/</loc></url></urlset>");
    assert_eq!(
        legacy.namespace_profile,
        Some(SitemapNamespaceProfile::LegacyUnqualified)
    );
    assert_eq!(legacy.counts.protocol_eligible_completed_entries, 1);
    let strict = count_sitemap_xml(
        b"<urlset/>",
        &SitemapCountLimits {
            allow_legacy_unqualified: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!strict.xml_scan_complete);
    assert_eq!(
        strict.failure.unwrap().code,
        "sitemap_invalid_root_namespace"
    );
    let index = complete(&format!(
        "<s:sitemapindex xmlns:s='{NS}'><s:sitemap><s:loc>https://e.test/child.xml</s:loc></s:sitemap></s:sitemapindex>"
    ));
    assert_eq!(index.kind, Some(SitemapDocumentKind::SitemapIndex));
    assert_eq!(index.counts.protocol_eligible_completed_entries, 1);
}

#[test]
fn count_recoverable_loc_diagnostics_continue_with_disjoint_entry_classes() {
    let xml = urlset(&format!(
        "<url/><url><loc/></url>\
        <url><loc>https://e.test/a</loc><loc>https://e.test/b</loc></url>\
        <url><loc>https://e.test/a</loc><loc>https://e.test/%GG</loc></url>\
        {}{}{}{}",
        loc("https://user:pass@e.test/"),
        loc(" &#9;https://e.test/trim&#10; "),
        loc("&#160;https://e.test/non-xml-space"),
        loc("https://e.test/a")
    ));
    let value = complete(&xml);
    let facts = value.counts;
    assert_eq!(facts.immediate_entry_starts, 8);
    assert_eq!(facts.completed_immediate_entries, 8);
    assert_eq!(facts.started_document_namespace_loc_elements, 9);
    assert_eq!(facts.actual_document_namespace_loc_occurrences, 9);
    assert_eq!(facts.protocol_eligible_completed_entries, 2);
    assert_eq!(facts.missing_location_entries, 1);
    assert_eq!(facts.multiple_location_entries, 2);
    assert_eq!(facts.invalid_single_location_entries, 3);
    assert_eq!(facts.invalid_location_occurrences, 4);
    assert_eq!(facts.empty_location_occurrences, 1);
    assert_eq!(facts.credential_location_occurrences, 1);
    assert_eq!(facts.malformed_percent_location_occurrences, 1);
    assert_eq!(facts.invalid_absolute_http_location_occurrences, 1);
    assert_eq!(facts.xml_space_trimmed_location_occurrences, 1);
    assert_eq!(
        rejected_retained(&xml, &SitemapParserLimits::default()),
        "sitemap_ambiguous_multiple_locations"
    );
}

#[test]
fn count_strict_loc_profile_has_independent_lexical_and_character_boundaries() {
    let prefix = "https://e.test/";
    let length_2047 = format!("{prefix}{}", "x".repeat(2047 - prefix.len()));
    let length_2048 = format!("{prefix}{}", "x".repeat(2048 - prefix.len()));
    let unicode_2047 = format!("{prefix}{}", "é".repeat(2047 - prefix.len()));
    for (original, eligible) in [
        (length_2047.as_str(), true),
        (length_2048.as_str(), false),
        (unicode_2047.as_str(), true),
        ("HTTP://e.test/%2f?q=a&amp;b=b", true),
        ("https://e.test/%", false),
        ("https://e.test/%2", false),
        ("https://e.test/%GG", false),
        ("https://@e.test/", false),
        ("https://user:pass@e.test/", false),
        ("ftp://e.test/", false),
        ("https:repair-me", false),
        ("https:///repair-me", false),
        ("https://e.test/a\\b", false),
        ("https://e.test/a&#9;b", false),
        ("https://e.test/a b", false),
        ("https://e.test/[invalid-path]", false),
        ("https://[::1]/valid-path", true),
        ("https://e.test/#fragment", false),
        (" &#9;https://e.test/edge&#10; ", true),
        ("&#160;https://e.test/", false),
    ] {
        let value = complete(&urlset(&loc(original)));
        assert_eq!(
            value.counts.protocol_eligible_completed_entries,
            u64::from(eligible),
            "{original}"
        );
        assert_eq!(
            value.counts.invalid_single_location_entries,
            u64::from(!eligible)
        );
        assert_eq!(value.loc_lexical_profile, "absolute-http-xml-loc-v1");
        assert_eq!(value.loc_whitespace_profile, "xml-s-edge-trim-v1");
    }
    assert_eq!(
        complete(&urlset(&loc(&length_2048)))
            .counts
            .over_character_limit_location_occurrences,
        1
    );
}

#[test]
fn count_same_strict_reader_stops_preserve_only_actual_completed_prefix_counts() {
    let first = loc("https://e.test/accepted");
    for (suffix, expected_code, starts, locs) in [
        ("<!DOCTYPE urlset>", "sitemap_dtd_unsupported", 1, 1),
        (
            "<url><loc>&unknown;</loc></url>",
            "sitemap_invalid_entity",
            2,
            2,
        ),
        (
            "<url><loc>&#0;</loc></url>",
            "sitemap_illegal_xml_character",
            2,
            2,
        ),
        ("<f:url/>", "sitemap_unbound_namespace_prefix", 1, 1),
        (
            "<extra><url/></extra>",
            "sitemap_invalid_element_parent",
            1,
            1,
        ),
        (
            "<url><loc><b/></loc></url>",
            "sitemap_nested_field_markup",
            2,
            2,
        ),
    ] {
        let source = format!("<urlset xmlns='{NS}'>{first}{suffix}</urlset>");
        let result = scan(&source);
        assert!(!result.xml_scan_complete, "{suffix}");
        assert_eq!(result.failure.unwrap().code, expected_code);
        assert_eq!(result.counts.immediate_entry_starts, starts);
        assert_eq!(result.counts.started_document_namespace_loc_elements, locs);
        assert_eq!(result.counts.completed_immediate_entries, 1);
        assert_eq!(result.counts.protocol_eligible_completed_entries, 1);
        assert_eq!(result.counts.missing_location_entries, 0);
    }
    for bytes in [
        b"<urlset>\xff</urlset>".as_slice(),
        b"<urlset>\0</urlset>".as_slice(),
    ] {
        let result = count_sitemap_xml(bytes, &SitemapCountLimits::default()).unwrap();
        assert!(!result.xml_scan_complete);
        assert_eq!(result.events_observed, 0);
        assert_eq!(result.counts, SitemapXmlCountFacts::default());
        assert_eq!(result.input_xml_bytes, bytes.len());
        assert_eq!(
            result.input_xml_sha256,
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    for source in [
        format!("<urlset xmlns='{NS}' a='1'b='2'/>"),
        format!("<urlset xmlns='{NS}' xmlns:a='urn:x' xmlns:b='urn:x' a:k='1' b:k='2'/>"),
        format!("<?XML reserved?><urlset xmlns='{NS}'/>"),
    ] {
        let counted = scan(&source);
        assert!(!counted.xml_scan_complete);
        assert_eq!(
            counted.failure.unwrap().code,
            rejected_retained(&source, &SitemapParserLimits::default())
        );
        assert_eq!(counted.counts.protocol_eligible_completed_entries, 0);
    }
}

#[test]
fn count_work_bounds_remain_separate_from_unchanged_retention_caps() {
    let source = urlset(&loc("https://e.test/a").repeat(3));
    for (limits, code) in [
        (
            SitemapParserLimits {
                max_entries: 1,
                ..Default::default()
            },
            "sitemap_entry_budget",
        ),
        (
            SitemapParserLimits {
                max_location_occurrences: 1,
                ..Default::default()
            },
            "sitemap_location_budget",
        ),
        (
            SitemapParserLimits {
                max_retained_text_utf8_bytes: 1,
                ..Default::default()
            },
            "sitemap_retained_text_byte_budget",
        ),
    ] {
        assert_eq!(rejected_retained(&source, &limits), code);
        assert_eq!(
            complete(&source).counts.protocol_eligible_completed_entries,
            3
        );
    }
    for (limits, code) in [
        (
            SitemapCountLimits {
                max_xml_events: 3,
                ..Default::default()
            },
            "sitemap_xml_event_budget",
        ),
        (
            SitemapCountLimits {
                max_xml_depth: 2,
                ..Default::default()
            },
            "sitemap_xml_depth_budget",
        ),
    ] {
        let value = count_sitemap_xml(source.as_bytes(), &limits).unwrap();
        assert!(!value.xml_scan_complete);
        assert_eq!(value.failure.unwrap().code, code);
        assert_eq!(value.counts.protocol_eligible_completed_entries, 0);
        assert_eq!(value.effective_limits, limits);
    }
    let long = urlset(&format!(
        "{}{}",
        loc("https://e.test/a"),
        loc(&"x".repeat(65))
    ));
    let value = count_sitemap_xml(
        long.as_bytes(),
        &SitemapCountLimits {
            max_field_utf8_bytes: 64,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(value.failure.unwrap().code, "sitemap_field_byte_budget");
    assert_eq!(value.counts.protocol_eligible_completed_entries, 1);
    assert_eq!(value.counts.completed_immediate_entries, 1);
    assert_eq!(value.counts.started_document_namespace_loc_elements, 2);
    let exact = SitemapCountLimits {
        max_xml_bytes: source.len(),
        ..Default::default()
    };
    assert!(
        count_sitemap_xml(source.as_bytes(), &exact)
            .unwrap()
            .xml_scan_complete
    );
    assert_eq!(
        count_sitemap_xml(format!("{source} ").as_bytes(), &exact)
            .unwrap_err()
            .code,
        "sitemap_xml_byte_budget"
    );
    for invalid in [
        SitemapCountLimits {
            max_xml_events: 0,
            ..Default::default()
        },
        SitemapCountLimits {
            max_xml_bytes: 8 * 1024 * 1024 + 1,
            ..Default::default()
        },
    ] {
        assert_eq!(
            count_sitemap_xml(source.as_bytes(), &invalid)
                .unwrap_err()
                .code,
            "sitemap_invalid_limits"
        );
    }
}

#[test]
fn count_actual_49999_50000_50001_entries_and_unfinished_threshold_controls() {
    for (root, entry) in [("urlset", "url"), ("sitemapindex", "sitemap")] {
        let opening = format!("<{root} xmlns='{NS}'>");
        // Independent fixture uses the same original URL in distinct entries: no dedupe allowed.
        let one = format!("<{entry}><loc>https://e.test/repeated</loc></{entry}>");
        for count in [49_999usize, 50_000, 50_001] {
            let source = format!("{opening}{}</{root}>", one.repeat(count));
            let value = complete(&source);
            assert_eq!(value.counts.immediate_entry_starts, count as u64);
            assert_eq!(value.counts.completed_immediate_entries, count as u64);
            assert_eq!(
                value.counts.started_document_namespace_loc_elements,
                count as u64
            );
            assert_eq!(
                value.counts.actual_document_namespace_loc_occurrences,
                count as u64
            );
            assert_eq!(
                value.counts.protocol_eligible_completed_entries,
                count as u64
            );
            if root == "urlset" && count == 50_001 {
                let legacy = observe_sitemap_xml(
                    source.as_bytes(),
                    SitemapParserContext::default(),
                    &SitemapParserLimits::default(),
                )
                .unwrap();
                assert!(!legacy.complete);
                let failure = legacy.failure.as_ref().unwrap();
                assert_eq!(failure.code, "sitemap_entry_budget");
                assert_eq!(
                    failure.xml_byte_offset,
                    Some((opening.len() + 50_000 * one.len()) as u64)
                );
                assert_eq!(legacy.entries.len(), 50_000);
                assert_eq!(legacy.locations.len(), 50_000);
                assert_eq!(legacy.admission_locations().count(), 0);
            }
        }
    }
    let opening = format!("<urlset xmlns='{NS}'>");
    let first = loc("https://e.test/accepted").repeat(50_000);
    for (suffix, started_locs, closed_locs) in [
        ("<url>", 50_000, 50_000),
        ("<url><loc>", 50_001, 50_000),
        ("<url><loc>https://e.test/unfinished</loc>", 50_001, 50_001),
    ] {
        let value = scan(&format!("{opening}{first}{suffix}"));
        assert!(!value.xml_scan_complete);
        assert_eq!(value.counts.immediate_entry_starts, 50_001);
        assert_eq!(value.counts.completed_immediate_entries, 50_000);
        assert_eq!(
            value.counts.started_document_namespace_loc_elements,
            started_locs
        );
        assert_eq!(
            value.counts.actual_document_namespace_loc_occurrences,
            closed_locs
        );
        assert_eq!(value.counts.protocol_eligible_completed_entries, 50_000);
        assert_eq!(value.counts.missing_location_entries, 0);
    }
    let late = scan(&format!(
        "{opening}{first}{}<!DOCTYPE urlset>",
        loc("https://e.test/50001")
    ));
    assert!(!late.xml_scan_complete);
    assert_eq!(late.failure.unwrap().code, "sitemap_dtd_unsupported");
    assert_eq!(late.counts.protocol_eligible_completed_entries, 50_001);
}

#[test]
fn retained_xml_raw_spans_context_hashes_and_typed_json_match_independent_golden() {
    let input = include_bytes!("count-retained-golden.xml");
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("count-retained-golden.json")).unwrap();
    let observed = observe_sitemap_xml(
        input,
        SitemapParserContext::default(),
        &SitemapParserLimits::default(),
    )
    .unwrap();
    assert_eq!(serde_json::to_value(&observed).unwrap(), expected);
    assert_eq!(observed.admission_locations().count(), 1);
    let duplicate = urlset(
        "<url><loc>https://e.test/a</loc><loc>https://e.test/b</loc></url><url><loc>https://unseen.test/</loc></url>",
    );
    let legacy = observe_sitemap_xml(
        duplicate.as_bytes(),
        SitemapParserContext::default(),
        &SitemapParserLimits::default(),
    )
    .unwrap();
    assert_eq!(legacy.entries.len(), 1);
    assert_eq!(legacy.locations.len(), 2);
    assert!(
        legacy
            .locations
            .iter()
            .all(|row| row.validation_error.as_deref() == Some("ambiguous_multiple_locations"))
    );
    assert_eq!(legacy.admission_locations().count(), 0);
    assert_eq!(complete(&duplicate).counts.completed_immediate_entries, 2);
}
