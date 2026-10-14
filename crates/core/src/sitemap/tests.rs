//! Directed candidate fixtures. AUTHORED / NOT RUN by the drafting agent.
use super::*;
use flate2::{Compression, write::GzEncoder};
use std::io::Write;

fn inspect(xml: &str) -> SitemapParseObservation {
    observe_sitemap_xml(
        xml.as_bytes(),
        SitemapParserContext::default(),
        &SitemapParserLimits::default(),
    )
    .unwrap()
}
fn accepted(xml: &str) -> SitemapParseObservation {
    let observation = inspect(xml);
    assert!(observation.complete, "{:?}", observation.failure);
    assert!(observation.failure.is_none());
    observation
}
fn rejected(xml: &str) -> SitemapParseObservation {
    let observation = inspect(xml);
    assert!(!observation.complete, "unexpected acceptance: {xml}");
    assert!(observation.failure.is_some());
    assert_eq!(observation.admission_locations().count(), 0);
    observation
}
fn fail_code(xml: &[u8], limits: &SitemapParserLimits) -> String {
    match observe_sitemap_xml(xml, SitemapParserContext::default(), limits) {
        Err(error) => error.code,
        Ok(observation) => {
            assert!(!observation.complete);
            assert_eq!(observation.admission_locations().count(), 0);
            observation.failure.unwrap().code
        }
    }
}
fn document(value: &str) -> String {
    format!("<urlset><url><loc>{value}</loc></url></urlset>")
}
fn gz(bytes: &[u8]) -> Vec<u8> {
    let mut writer = GzEncoder::new(Vec::new(), Compression::default());
    writer.write_all(bytes).unwrap();
    writer.finish().unwrap()
}
fn gzip_failure(bytes: &[u8], code: &str, limits: &SitemapParserLimits) {
    assert_eq!(decode_sitemap_entity(bytes, limits).unwrap_err().code, code);
}

#[test]
fn keeps_existing_adapter_index_and_failure_controls() {
    let result = parse_sitemap("<sitemapindex><sitemap><loc>https://e.org/a.xml?a=1&amp;b=2</loc></sitemap></sitemapindex>").unwrap();
    assert!(result.is_index);
    assert_eq!(result.locations, ["https://e.org/a.xml?a=1&b=2"]);
    for xml in [
        "<urlset><loc>bad</wrong>",
        "<html><loc>https://e.org/</loc></html>",
        "",
        "<urlset>",
    ] {
        assert!(parse_sitemap(xml).is_err());
    }
    assert!(parse_sitemap("<urlset/>").is_ok());
}

#[test]
fn accepts_default_and_prefixed_sitemap09_namespaces() {
    for xml in [
        "<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>https://e.org/</loc></url></urlset>",
        "<s:urlset xmlns:s='http://www.sitemaps.org/schemas/sitemap/0.9'><s:url><s:loc>https://e.org/</s:loc></s:url></s:urlset>",
    ] {
        let result = accepted(xml);
        assert_eq!(
            result.namespace_profile,
            Some(SitemapNamespaceProfile::Sitemap09)
        );
        assert_eq!(
            result
                .admission_locations()
                .next()
                .unwrap()
                .normalized_fetch_url
                .as_deref(),
            Some("https://e.org/")
        );
    }
}

#[test]
fn decodes_namespace_attribute_references_before_resolution() {
    let result = accepted(
        "<s:urlset xmlns:s='http://www.sitemaps.org/schemas/sitemap/&#48;.9'><s:url><s:loc>https://e.org/</s:loc></s:url></s:urlset>",
    );
    assert_eq!(
        result.namespace_profile,
        Some(SitemapNamespaceProfile::Sitemap09)
    );
    assert_eq!(result.locations.len(), 1);
}

#[test]
fn unqualified_profile_is_explicit_and_can_be_disabled() {
    assert_eq!(
        accepted("<urlset/>").namespace_profile,
        Some(SitemapNamespaceProfile::LegacyUnqualified)
    );
    let limits = SitemapParserLimits {
        allow_legacy_unqualified: false,
        ..Default::default()
    };
    assert_eq!(
        fail_code(b"<urlset/>", &limits),
        "sitemap_invalid_root_namespace"
    );
}

#[test]
fn rejects_wrong_root_and_wrong_root_namespace() {
    for xml in [
        "<html/>",
        "<urlset xmlns='urn:wrong'/>",
        "<x:urlset xmlns:x='urn:wrong'/>",
    ] {
        rejected(xml);
    }
}

#[test]
fn foreign_extension_locations_never_become_page_or_child_locations() {
    let result = accepted(
        "<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9' xmlns:i='urn:image' xmlns:v='urn:video'><url><loc>https://e.org/page</loc><i:image><i:loc>https://e.org/image-sentinel</i:loc></i:image><v:video><v:loc>https://e.org/video-sentinel</v:loc></v:video></url></urlset>",
    );
    assert_eq!(result.locations.len(), 1);
    assert_eq!(
        result.locations[0].decoded_value.as_deref(),
        Some("https://e.org/page")
    );
    assert_eq!(result.entries[0].loc_occurrence_ordinals, [1]);
}

#[test]
fn namespace_reset_does_not_fall_back_to_local_name_matching() {
    let result = accepted(
        "<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc xmlns=''>https://e.org/false-sentinel</loc></url></urlset>",
    );
    assert_eq!(result.locations.len(), 1);
    assert_eq!(
        result.locations[0].validation_error.as_deref(),
        Some("missing_location")
    );
    assert!(result.locations[0].decoded_value.is_none());
    assert_eq!(result.admission_locations().count(), 0);
}

#[test]
fn unbound_prefixes_fail_even_on_ignored_extensions_and_attributes() {
    for xml in [
        "<urlset><x:ignored/></urlset>",
        "<urlset x:ignored='yes'/>",
        "<x:urlset/>",
    ] {
        assert_eq!(
            rejected(xml).failure.unwrap().code,
            "sitemap_unbound_namespace_prefix"
        );
    }
}

#[test]
fn validates_reserved_namespace_bindings_and_expanded_attribute_duplicates() {
    for xml in [
        "<urlset xmlns:xml='urn:wrong'/>",
        "<urlset xmlns:p='http://www.w3.org/XML/1998/namespace'/>",
        "<urlset xmlns='http://www.w3.org/2000/xmlns/'/>",
        "<urlset xmlns:xmlns='urn:wrong'/>",
        "<urlset xmlns:p=''/>",
    ] {
        assert_eq!(
            rejected(xml).failure.unwrap().code,
            "sitemap_invalid_namespace"
        );
    }
    let duplicate = rejected("<urlset xmlns:a='urn:same' xmlns:b='urn:same' a:x='1' b:x='2'/>");
    assert_eq!(
        duplicate.failure.unwrap().code,
        "sitemap_duplicate_expanded_attribute"
    );
    accepted("<urlset xmlns:p='urn:other' x='1' p:x='2' xml:lang='en'/>");
}

#[test]
fn validates_attribute_separators_quotes_names_and_lexical_duplicates() {
    for xml in [
        "<urlset a='1'b='2'/>",
        "<urlset a=1/>",
        "<urlset a='x<y'/>",
        "<urlset 1a='x'/>",
        "<urlset a::b='x'/>",
        "<urlset a='1' a='2'/>",
    ] {
        rejected(xml);
    }
    accepted("<urlset a = '1' b=\"2\"/>");
}

#[test]
fn unsupported_unicode_qnames_fail_explicitly() {
    for xml in [
        "<urlset><é:ignored xmlns:é='urn:e'/></urlset>",
        "<urlset é='x'/>",
    ] {
        assert_eq!(
            rejected(xml).failure.unwrap().code,
            "sitemap_unsupported_xml_name"
        );
    }
}

#[test]
fn recognized_elements_require_immediate_expected_parent() {
    for xml in [
        "<urlset><loc>https://e.org/</loc></urlset>",
        "<urlset><url><extra><loc>https://e.org/</loc></extra></url></urlset>",
        "<urlset><sitemap><loc>https://e.org/</loc></sitemap></urlset>",
        "<sitemapindex><url><loc>https://e.org/</loc></url></sitemapindex>",
        "<urlset><url><loc><extra/></loc></url></urlset>",
    ] {
        rejected(xml);
    }
}

#[test]
fn retains_distinct_duplicate_entry_occurrences_without_deduplication() {
    let xml = "<urlset><url><loc>https://e.org/a</loc></url><url><loc>https://e.org/a</loc></url></urlset>";
    let result = accepted(xml);
    assert_eq!(result.entries.len(), 2);
    assert_eq!(result.locations.len(), 2);
    assert_eq!(result.locations[0].entry_ordinal, 1);
    assert_eq!(result.locations[1].entry_ordinal, 2);
    assert_eq!(result.locations[0].ordinal, 1);
    assert_eq!(result.locations[1].ordinal, 2);
    assert!(result.locations[0].xml_byte_start < result.locations[1].xml_byte_start);
    assert_eq!(
        parse_sitemap(xml).unwrap().locations,
        ["https://e.org/a", "https://e.org/a"]
    );
}

#[test]
fn multiple_immediate_locations_retain_evidence_but_fail_closed() {
    let xml = "<urlset><url><loc>https://e.org/a</loc><loc>https://e.org/b</loc></url></urlset>";
    let result = rejected(xml);
    assert_eq!(
        result.failure.unwrap().code,
        "sitemap_ambiguous_multiple_locations"
    );
    assert_eq!(result.locations.len(), 2);
    assert_eq!(result.entries.len(), 1);
    assert!(result.entries[0].complete);
    assert_eq!(result.entries[0].loc_occurrence_ordinals, [1, 2]);
    for (index, row) in result.locations.iter().enumerate() {
        assert_eq!(row.loc_ordinal, Some(index + 1));
        assert_eq!(
            row.validation_error.as_deref(),
            Some("ambiguous_multiple_locations")
        );
    }
    assert!(parse_sitemap(xml).is_err());
}

#[test]
fn missing_empty_and_whitespace_location_are_separate_facts() {
    let xml = "<urlset><url/><url><loc/></url><url><loc> \t </loc></url></urlset>";
    let result = accepted(xml);
    assert_eq!(result.locations.len(), 3);
    assert_eq!(result.locations[0].loc_ordinal, None);
    assert_eq!(result.locations[0].decoded_value, None);
    assert_eq!(
        result.locations[0].validation_error.as_deref(),
        Some("missing_location")
    );
    assert_eq!(result.locations[1].decoded_value.as_deref(), Some(""));
    assert_eq!(
        result.locations[1].raw_element_xml.as_deref(),
        Some("<loc/>")
    );
    assert_eq!(result.locations[2].decoded_value.as_deref(), Some(" \t "));
    assert_eq!(result.locations[2].raw_inner_xml.as_deref(), Some(" \t "));
    assert_eq!(result.admission_locations().count(), 0);
    assert!(parse_sitemap(xml).unwrap().locations.is_empty());
}

#[test]
fn retains_raw_xml_entities_cdata_and_untrimmed_decoded_value() {
    let inner = " https://Example.org:443/A?b=2&amp;a=1<![CDATA[#fragment]]> ";
    let xml = document(inner);
    let result = accepted(&xml);
    let row = &result.locations[0];
    assert_eq!(row.raw_inner_xml.as_deref(), Some(inner));
    assert_eq!(
        row.raw_element_xml.as_deref(),
        Some(format!("<loc>{inner}</loc>").as_str())
    );
    assert_eq!(
        row.decoded_value.as_deref(),
        Some(" https://Example.org:443/A?b=2&a=1#fragment ")
    );
    assert_eq!(
        row.normalized_fetch_url.as_deref(),
        Some("https://example.org/A?b=2&a=1")
    );
    assert_eq!(
        &xml[row.xml_byte_start as usize..row.xml_byte_end as usize],
        row.raw_element_xml.as_deref().unwrap()
    );
}

#[test]
fn original_byte_offsets_include_bom_and_multibyte_utf8() {
    let xml = "\u{feff}<urlset><url><lastmod>ü</lastmod><loc>https://e.org/ä</loc></url></urlset>";
    let result = accepted(xml);
    assert_eq!(result.xml_bytes, xml.len());
    assert_eq!(
        result.entries[0].xml_byte_start as usize,
        xml.find("<url>").unwrap()
    );
    let row = &result.locations[0];
    assert_eq!(row.xml_byte_start as usize, xml.find("<loc>").unwrap());
    assert_eq!(
        &xml[row.xml_byte_start as usize..row.xml_byte_end as usize],
        "<loc>https://e.org/ä</loc>"
    );
    assert_eq!(row.decoded_value.as_deref(), Some("https://e.org/ä"));
}

#[test]
fn raw_lastmod_repeats_are_retained_without_date_validation() {
    let result = accepted(
        "<urlset><url><loc>https://e.org/</loc><lastmod>not-a-date</lastmod><lastmod><![CDATA[ 2026-99-99 ]]></lastmod></url></urlset>",
    );
    let fields = &result.entries[0].lastmod;
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].ordinal, 1);
    assert_eq!(fields[0].decoded_value, "not-a-date");
    assert_eq!(fields[1].ordinal, 2);
    assert_eq!(fields[1].raw_inner_xml, "<![CDATA[ 2026-99-99 ]]>");
    assert_eq!(fields[1].decoded_value, " 2026-99-99 ");
}

#[test]
fn invalid_relative_nonhttp_credentials_and_controls_remain_observed() {
    let xml = "<urlset><url><loc>/relative</loc></url><url><loc>ftp://e.org/a</loc></url><url><loc>https://user:secret@e.org/</loc></url><url><loc>https://e.org/a&#10;b</loc></url><url><loc>not a url</loc></url></urlset>";
    let result = accepted(xml);
    assert_eq!(result.locations.len(), 5);
    assert_eq!(result.admission_locations().count(), 0);
    for row in &result.locations {
        assert_eq!(
            row.validation_error.as_deref(),
            Some("invalid_absolute_http_location")
        );
    }
    assert_eq!(
        parse_sitemap(xml).unwrap().locations,
        [
            "/relative",
            "ftp://e.org/a",
            "https://user:secret@e.org/",
            "https://e.org/a\nb",
            "not a url"
        ]
    );
}

#[test]
fn location_edge_profile_allows_xml_s_and_rejects_other_edges_or_internal_controls() {
    for inner in [" \t\r\nhttps://e.org/\n ", "&#9;https://e.org/&#10;"] {
        let result = accepted(&document(inner));
        assert_eq!(result.admission_locations().count(), 1);
        assert_eq!(
            result.locations[0].normalized_fetch_url.as_deref(),
            Some("https://e.org/")
        );
        assert_ne!(
            result.locations[0].decoded_value.as_deref(),
            Some("https://e.org/")
        );
        assert_eq!(result.locations[0].raw_inner_xml.as_deref(), Some(inner));
    }
    for inner in [
        "https://e.org/a&#9;b",
        "https://e.org/a&#10;b",
        "\u{a0}https://e.org/",
        "https://e.org/\u{a0}",
        "&#160;https://e.org/",
        "https://e.org/&#160;",
        "\u{2003}https://e.org/",
    ] {
        let result = accepted(&document(inner));
        assert_eq!(result.admission_locations().count(), 0);
        assert_eq!(
            result.locations[0].validation_error.as_deref(),
            Some("invalid_absolute_http_location")
        );
        assert_eq!(result.locations[0].raw_inner_xml.as_deref(), Some(inner));
        assert!(result.locations[0].normalized_fetch_url.is_none());
    }
}

#[test]
fn parser_does_not_invent_scope_filters_or_target_http_status() {
    let result = accepted(&document("https://other-origin.test:8443/outside"));
    assert_eq!(result.admission_locations().count(), 1);
    assert_eq!(
        result.locations[0].normalized_fetch_url.as_deref(),
        Some("https://other-origin.test:8443/outside")
    );
    let json = serde_json::to_value(result).unwrap();
    assert!(json["locations"][0].get("status_code").is_none());
    assert!(json.get("discovery_complete").is_none());
    assert!(json.get("source_cohort_complete").is_none());
}

#[test]
fn sitemap_index_locations_are_child_documents_with_original_values() {
    let result = accepted(
        "<sitemapindex><sitemap><loc> https://e.org/child.xml.gz#x </loc></sitemap></sitemapindex>",
    );
    assert_eq!(result.kind, Some(SitemapDocumentKind::SitemapIndex));
    assert_eq!(result.locations[0].role, SitemapLocationRole::ChildSitemap);
    assert_eq!(
        result.locations[0].decoded_value.as_deref(),
        Some(" https://e.org/child.xml.gz#x ")
    );
    assert_eq!(
        result.locations[0].normalized_fetch_url.as_deref(),
        Some("https://e.org/child.xml.gz")
    );
}

#[test]
fn unknown_entities_dtd_and_illegal_references_reject_without_expansion() {
    for xml in [
        "<!DOCTYPE urlset [<!ENTITY x 'https://e.org/'>]><urlset><url><loc>&x;</loc></url></urlset>",
        "<urlset><url><loc>&unknown;</loc></url></urlset>",
        "<urlset a='&unknown;'/>",
        "<urlset a='&nbsp;'/>",
        "<urlset><url><loc>&#1;</loc></url></urlset>",
        "<urlset a='&#xFFFE;'/>",
        "<urlset><url><loc>&#x110000;</loc></url></urlset>",
    ] {
        rejected(xml);
    }
}

#[test]
fn legal_references_and_xml_line_end_normalization_preserve_raw() {
    let xml = document("https://e.org/a?x=&quot;q&quot;&amp;y=&apos;r&apos;\r\n");
    let result = accepted(&xml);
    assert_eq!(
        result.locations[0].decoded_value.as_deref(),
        Some("https://e.org/a?x=\"q\"&y='r'\n")
    );
    assert!(
        result.locations[0]
            .raw_inner_xml
            .as_ref()
            .unwrap()
            .ends_with("\r\n")
    );
}

#[test]
fn late_malformed_xml_retains_closed_occurrences_without_admission() {
    let xml = "<urlset><url><loc>https://e.org/a</loc></url><url><loc>https://e.org/b</loc></wrong></urlset>";
    let result = rejected(xml);
    assert_eq!(result.locations.len(), 2);
    assert_eq!(result.entries.len(), 2);
    assert!(result.entries[0].complete);
    assert!(!result.entries[1].complete);
    assert_eq!(result.entries[1].loc_occurrence_ordinals, [2]);
    assert_eq!(
        result.locations[1].decoded_value.as_deref(),
        Some("https://e.org/b")
    );
    assert!(parse_sitemap(xml).is_err());
}

#[test]
fn incomplete_entry_keeps_closed_lastmod_and_does_not_invent_missing_loc() {
    let result = rejected("<urlset><url><lastmod>raw</lastmod><ignored>");
    assert_eq!(result.entries.len(), 1);
    assert!(!result.entries[0].complete);
    assert_eq!(result.entries[0].lastmod[0].decoded_value, "raw");
    assert_eq!(result.entries[0].diagnostics, ["incomplete_entry"]);
    assert!(result.locations.is_empty());
}

#[test]
fn rejects_two_roots_text_cdata_entities_and_unicode_whitespace_outside_root() {
    for xml in [
        "<urlset/><urlset/>",
        "text<urlset/>",
        "<urlset/>text",
        "<![CDATA[ ]]><urlset/>",
        "&#32;<urlset/>",
        "\u{a0}<urlset/>",
        "<urlset/>\u{feff}",
    ] {
        rejected(xml);
    }
    accepted(" \t\r\n<!--before--><urlset/><!--after-->\n");
}

#[test]
fn rejects_illegal_comments_and_character_data_close_delimiter() {
    for xml in [
        "<urlset><!--a--b--></urlset>",
        "<urlset><!--a---></urlset>",
        "<urlset>]]></urlset>",
        "<urlset><url><loc>https://e.org/]]></loc></url></urlset>",
    ] {
        rejected(xml);
    }
    accepted("<urlset><!--legal--><?target data?></urlset>");
}

#[test]
fn xml_declaration_has_utf8_order_and_literal_pseudoattributes() {
    accepted("<?xml version='1.0' encoding='UTF-8' standalone='yes'?><urlset/>");
    for xml in [
        " <?xml version='1.0'?><urlset/>",
        "<?xml version='1.1'?><urlset/>",
        "<?xml version='1.0' encoding='ISO-8859-1'?><urlset/>",
        "<?xml encoding='UTF-8' version='1.0'?><urlset/>",
        "<?xml version='1.0' standalone='yes' encoding='UTF-8'?><urlset/>",
        "<?xml version='&#49;.0'?><urlset/>",
        "<?XML version='1.0'?><urlset/>",
        "<?xml version='1.0'?><urlset/><?xml version='1.0'?>",
    ] {
        rejected(xml);
    }
}

#[test]
fn rejects_invalid_utf8_and_raw_xml_control_characters() {
    assert_eq!(
        fail_code(b"<urlset>\xff</urlset>", &Default::default()),
        "sitemap_invalid_utf8"
    );
    assert_eq!(
        fail_code(b"<urlset>\x01</urlset>", &Default::default()),
        "sitemap_illegal_xml_character"
    );
}

#[test]
fn context_is_preserved_as_caller_declaration_without_source_attestation() {
    let context = SitemapParserContext {
        requested_document_url: Some("https://e.org/request.xml".into()),
        final_document_url: Some("https://e.org/final.xml".into()),
        originating_occurrence_id: Some("caller-source:7".into()),
    };
    let result = observe_sitemap_xml(b"<urlset/>", context.clone(), &Default::default()).unwrap();
    assert_eq!(result.context, context);
    assert_eq!(result.context_basis, "caller_declared_parser_context");
    assert!(result.complete);
}

#[test]
fn xml_byte_budget_accepts_exact_fit_and_rejects_one_byte_more() {
    let xml = b"<urlset/>";
    let exact = SitemapParserLimits {
        max_xml_bytes: xml.len(),
        ..Default::default()
    };
    assert!(
        observe_sitemap_xml(xml, Default::default(), &exact)
            .unwrap()
            .complete
    );
    let smaller = SitemapParserLimits {
        max_xml_bytes: xml.len() - 1,
        ..Default::default()
    };
    assert_eq!(fail_code(xml, &smaller), "sitemap_xml_byte_budget");
}

#[test]
fn xml_event_budget_counts_true_eof_with_exact_fit_control() {
    let exact = SitemapParserLimits {
        max_xml_events: 2,
        ..Default::default()
    };
    let result = observe_sitemap_xml(b"<urlset/>", Default::default(), &exact).unwrap();
    assert!(result.complete);
    assert_eq!(result.events_observed, 2);
    let smaller = SitemapParserLimits {
        max_xml_events: 1,
        ..Default::default()
    };
    assert_eq!(
        fail_code(b"<urlset/>", &smaller),
        "sitemap_xml_event_budget"
    );
}

#[test]
fn xml_depth_budget_has_exact_fit_and_extension_depth_controls() {
    let xml = document("https://e.org/");
    let exact = SitemapParserLimits {
        max_xml_depth: 3,
        ..Default::default()
    };
    assert!(
        observe_sitemap_xml(xml.as_bytes(), Default::default(), &exact)
            .unwrap()
            .complete
    );
    let smaller = SitemapParserLimits {
        max_xml_depth: 2,
        ..Default::default()
    };
    assert_eq!(
        fail_code(xml.as_bytes(), &smaller),
        "sitemap_xml_depth_budget"
    );
    assert_eq!(
        fail_code(b"<urlset><a><b><c/></b></a></urlset>", &exact),
        "sitemap_xml_depth_budget"
    );
}

#[test]
fn entry_and_occurrence_budgets_count_missing_rows_and_duplicate_entries() {
    let xml = b"<urlset><url/><url/></urlset>";
    let exact = SitemapParserLimits {
        max_entries: 2,
        max_location_occurrences: 2,
        ..Default::default()
    };
    assert!(
        observe_sitemap_xml(xml, Default::default(), &exact)
            .unwrap()
            .complete
    );
    let entries = SitemapParserLimits {
        max_entries: 1,
        ..Default::default()
    };
    assert_eq!(fail_code(xml, &entries), "sitemap_entry_budget");
    let locations = SitemapParserLimits {
        max_location_occurrences: 1,
        ..Default::default()
    };
    assert_eq!(fail_code(xml, &locations), "sitemap_location_budget");
}

#[test]
fn exhausted_missing_row_budget_keeps_closed_lastmod_in_incomplete_entry() {
    let xml = b"<urlset><url/><url><lastmod>known</lastmod></url></urlset>";
    let limits = SitemapParserLimits {
        max_location_occurrences: 1,
        ..Default::default()
    };
    let result = observe_sitemap_xml(xml, Default::default(), &limits).unwrap();
    assert!(!result.complete);
    assert_eq!(
        result.failure.as_ref().unwrap().code,
        "sitemap_location_budget"
    );
    assert_eq!(result.admission_locations().count(), 0);
    assert_eq!(result.locations.len(), 1);
    assert_eq!(result.entries.len(), 2);
    assert!(!result.entries[1].complete);
    assert_eq!(result.entries[1].lastmod[0].decoded_value, "known");
    assert_eq!(result.entries[1].diagnostics, ["incomplete_entry"]);
    assert!(result.entries[1].loc_occurrence_ordinals.is_empty());
}

#[test]
fn field_byte_budget_applies_to_raw_element_not_only_decoded_value() {
    let xml = document("https://e.org/");
    let exact = SitemapParserLimits {
        max_field_utf8_bytes: "<loc>https://e.org/</loc>".len(),
        ..Default::default()
    };
    assert!(
        observe_sitemap_xml(xml.as_bytes(), Default::default(), &exact)
            .unwrap()
            .complete
    );
    let smaller = SitemapParserLimits {
        max_field_utf8_bytes: exact.max_field_utf8_bytes - 1,
        ..Default::default()
    };
    assert_eq!(
        fail_code(xml.as_bytes(), &smaller),
        "sitemap_field_byte_budget"
    );
    let escaped = document("&#104;&#116;&#116;&#112;&#115;://e.org/");
    assert_eq!(
        fail_code(escaped.as_bytes(), &exact),
        "sitemap_field_byte_budget"
    );
}

#[test]
fn retained_input_text_budget_has_exact_fit_and_context_control() {
    let xml = document("https://e.org/");
    let retained = "<loc>https://e.org/</loc>".len() + 3 * "https://e.org/".len();
    let exact = SitemapParserLimits {
        max_retained_text_utf8_bytes: retained,
        ..Default::default()
    };
    let result = observe_sitemap_xml(xml.as_bytes(), Default::default(), &exact).unwrap();
    assert!(result.complete);
    assert_eq!(result.retained_text_utf8_bytes, retained);
    let smaller = SitemapParserLimits {
        max_retained_text_utf8_bytes: retained - 1,
        ..Default::default()
    };
    assert_eq!(
        fail_code(xml.as_bytes(), &smaller),
        "sitemap_retained_text_byte_budget"
    );
    let context = SitemapParserContext {
        originating_occurrence_id: Some("123".into()),
        ..Default::default()
    };
    let limit = SitemapParserLimits {
        max_retained_text_utf8_bytes: 2,
        ..Default::default()
    };
    assert_eq!(
        observe_sitemap_xml(b"<urlset/>", context, &limit)
            .unwrap_err()
            .code,
        "sitemap_retained_text_byte_budget"
    );
}

#[test]
fn rejects_zero_or_above_ceiling_limits_before_inspection() {
    let zero = SitemapParserLimits {
        max_xml_depth: 0,
        ..Default::default()
    };
    assert_eq!(fail_code(b"<urlset/>", &zero), "sitemap_invalid_limits");
    let larger = SitemapParserLimits {
        max_entity_bytes: 64 * 1024 * 1024 + 1,
        ..Default::default()
    };
    gzip_failure(b"<urlset/>", "sitemap_invalid_limits", &larger);
}

#[test]
fn identity_decoder_does_not_guess_gzip_from_url_or_text() {
    let input = b"<urlset/>";
    let result = decode_sitemap_entity(input, &Default::default()).unwrap();
    assert_eq!(result.encoding, SitemapEntityEncoding::Identity);
    assert_eq!(result.xml, input);
    assert_eq!(result.post_http_decode_entity_bytes, input.len());
    assert_eq!(result.decoded_xml_bytes, input.len());
    assert!(serde_json::to_value(result).unwrap().get("xml").is_none());
}

#[test]
fn gzip_single_member_returns_actual_decoded_xml() {
    let xml = document("https://e.org/");
    let input = gz(xml.as_bytes());
    let decoded = decode_sitemap_entity(&input, &Default::default()).unwrap();
    assert_eq!(decoded.encoding, SitemapEntityEncoding::GzipFileEntity);
    assert_eq!(decoded.xml, xml.as_bytes());
    assert_eq!(decoded.post_http_decode_entity_bytes, input.len());
    assert_eq!(decoded.decoded_xml_bytes, xml.len());
    let observation =
        observe_sitemap_xml(&decoded.xml, Default::default(), &Default::default()).unwrap();
    assert!(observation.complete);
    assert_eq!(decoded.decoded_xml_sha256, observation.xml_sha256);
}

#[test]
fn gzip_members_concatenate_even_across_a_utf8_character_boundary() {
    let xml = document("https://e.org/ä");
    let split = xml.find('ä').unwrap() + 1;
    let mut input = gz(&xml.as_bytes()[..split]);
    input.extend(gz(&xml.as_bytes()[split..]));
    let decoded = decode_sitemap_entity(&input, &Default::default()).unwrap();
    assert_eq!(decoded.xml, xml.as_bytes());
    assert!(
        observe_sitemap_xml(&decoded.xml, Default::default(), &Default::default())
            .unwrap()
            .complete
    );
}

#[test]
fn gzip_empty_member_then_xml_is_complete() {
    let mut input = gz(b"");
    input.extend(gz(b"<urlset/>"));
    assert_eq!(
        decode_sitemap_entity(&input, &Default::default())
            .unwrap()
            .xml,
        b"<urlset/>"
    );
}

#[test]
fn gzip_inflation_cap_checks_combined_members_and_true_eof_at_fit() {
    let xml = b"<urlset/>";
    let mut input = gz(&xml[..4]);
    input.extend(gz(&xml[4..]));
    let exact = SitemapParserLimits {
        max_xml_bytes: xml.len(),
        ..Default::default()
    };
    assert_eq!(decode_sitemap_entity(&input, &exact).unwrap().xml, xml);
    let smaller = SitemapParserLimits {
        max_xml_bytes: xml.len() - 1,
        ..Default::default()
    };
    gzip_failure(&input, "sitemap_xml_inflation_byte_budget", &smaller);
    let mut corrupt_at_fit = input.clone();
    let last = corrupt_at_fit.len() - 1;
    corrupt_at_fit[last] ^= 1;
    gzip_failure(&corrupt_at_fit, "sitemap_invalid_gzip_entity", &exact);
}

#[test]
fn gzip_post_http_entity_cap_accepts_fit_and_rejects_plus_one() {
    let input = gz(b"<urlset/>");
    let exact = SitemapParserLimits {
        max_entity_bytes: input.len(),
        ..Default::default()
    };
    assert!(decode_sitemap_entity(&input, &exact).is_ok());
    let smaller = SitemapParserLimits {
        max_entity_bytes: input.len() - 1,
        ..Default::default()
    };
    gzip_failure(&input, "sitemap_entity_byte_budget", &smaller);
}

#[test]
fn gzip_crc32_and_isize_corruptions_are_both_rejected() {
    let input = gz(b"<urlset/>");
    for footer_offset in [8, 4] {
        let mut corrupt = input.clone();
        let index = corrupt.len() - footer_offset;
        corrupt[index] ^= 1;
        gzip_failure(&corrupt, "sitemap_invalid_gzip_entity", &Default::default());
    }
}

#[test]
fn gzip_truncated_header_deflate_and_footer_are_rejected() {
    let input = gz(b"<urlset/>");
    for length in [2, 5, 9, 10, 11, input.len() - 8, input.len() - 1] {
        gzip_failure(
            &input[..length],
            "sitemap_invalid_gzip_entity",
            &Default::default(),
        );
    }
}

#[test]
fn gzip_valid_first_member_cannot_hide_corrupt_second_member() {
    let first = gz(b"<urlset/>");
    let mut second = gz(b"");
    let index = second.len() - 8;
    second[index] ^= 1;
    let mut input = first;
    input.extend(second);
    gzip_failure(&input, "sitemap_invalid_gzip_entity", &Default::default());
}

#[test]
fn gzip_trailing_junk_and_zero_padding_are_rejected() {
    let first = gz(b"<urlset/>");
    for length in 1..=12 {
        for byte in [0u8, b'x'] {
            let mut input = first.clone();
            input.extend(std::iter::repeat_n(byte, length));
            gzip_failure(&input, "sitemap_invalid_gzip_entity", &Default::default());
        }
    }
}

#[test]
fn gzip_reserved_header_flags_and_nondeflate_method_are_rejected() {
    let input = gz(b"<urlset/>");
    let mut reserved = input.clone();
    reserved[3] |= 0x20;
    gzip_failure(
        &reserved,
        "sitemap_invalid_gzip_entity",
        &Default::default(),
    );
    let mut method = input;
    method[2] = 0;
    gzip_failure(&method, "sitemap_invalid_gzip_entity", &Default::default());
}

#[test]
fn gzip_nested_file_entity_is_not_recursively_inflated() {
    let input = gz(&gz(b"<urlset/>"));
    gzip_failure(&input, "sitemap_nested_gzip_entity", &Default::default());
}

#[test]
fn gzip_success_is_distinct_from_wellformed_xml_success() {
    let mut input = gz(b"<urlset/>");
    input.extend(gz(b"<urlset/>"));
    let decoded = decode_sitemap_entity(&input, &Default::default()).unwrap();
    assert_eq!(decoded.xml, b"<urlset/><urlset/>");
    let result =
        observe_sitemap_xml(&decoded.xml, Default::default(), &Default::default()).unwrap();
    assert!(!result.complete);
    assert_eq!(result.admission_locations().count(), 0);
    assert_eq!(result.failure.unwrap().code, "sitemap_multiple_roots");
}

#[test]
fn identity_entity_and_xml_caps_are_independent() {
    let input = b"<urlset/>";
    let entity = SitemapParserLimits {
        max_entity_bytes: input.len() - 1,
        ..Default::default()
    };
    gzip_failure(input, "sitemap_entity_byte_budget", &entity);
    let xml = SitemapParserLimits {
        max_xml_bytes: input.len() - 1,
        ..Default::default()
    };
    gzip_failure(input, "sitemap_xml_byte_budget", &xml);
}
