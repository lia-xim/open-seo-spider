use super::*;
use serde_json::json;

fn profile(config: Value) -> CrawlConfigProfile {
    parse_config_profile(
        &serde_json::to_vec(&json!({"version":1,"config":config,"required_headers":[]})).unwrap(),
    )
    .unwrap()
}

#[test]
fn seedfree_profile_keeps_declarations_exact_and_has_no_runtime_requirements() {
    let document = profile(
        json!({"max_body_bytes":1000003,"max_pages":null,"delay_ms":9007199254740993u64,"concurrency":80,"discover_sitemaps":false,"renderer":null,"custom_search":null}),
    );
    let before = serde_json::to_vec(&document).unwrap();
    let preview = preview_config_profile(&document).unwrap();
    assert!(preview.requires_targets);
    assert_eq!(preview.effective_config.max_body_bytes, 1000003);
    assert_eq!(preview.effective_config.delay_ms, 9007199254740993);
    assert_eq!(preview.effective_config.concurrency, 80);
    assert_eq!(preview.effective_config.request_timeout_secs, 30);
    assert!(preview.effective_config.explicit_sitemap_inputs.is_none());
    assert!(preview.effective_config.custom_search.is_none());
    assert!(
        !document
            .config
            .as_object()
            .unwrap()
            .contains_key("request_timeout_secs")
    );
    assert!(document.config["renderer"].is_null());
    assert!(document.config["custom_search"].is_null());
    assert!(preview.normalization.is_empty());
    assert_eq!(export_config_profile(&document).unwrap(), before);
    assert!(
        String::from_utf8(before)
            .unwrap()
            .contains("9007199254740993")
    );
    assert!(resolve_config_profile(&document, &BTreeMap::new()).is_err());
}

#[test]
fn every_optional_profile_keeps_declared_absence_and_explicit_null_distinct() {
    let absent = profile(json!({}));
    assert_eq!(absent.config, json!({}));
    for field in [
        "tls",
        "explicit_sitemap_inputs",
        "sitemap_source_processing",
        "sitemap_membership_limits",
        "sitemap_source_archive",
        "crawl_admission_limits",
        "crawl_quotas",
        "cdn_classification",
        "cdn_directory_reporting",
        "max_pages",
        "max_depth",
        "directive_reporting",
        "renderer",
        "crawl_policy",
        "live_policy_limits",
        "content_capture",
        "resource_discovery",
        "resource_discovery_limits",
        "custom_search",
    ] {
        let document = profile(json!({field:Value::Null}));
        assert!(
            document.config.as_object().unwrap().contains_key(field),
            "{field}"
        );
        assert!(document.config[field].is_null());
        let bytes = export_config_profile(&document).unwrap();
        assert_eq!(
            parse_config_profile(&bytes).unwrap().config,
            document.config
        );
        let effective =
            serde_json::to_value(preview_config_profile(&document).unwrap().effective_config)
                .unwrap();
        assert!(effective.get(field).is_none_or(Value::is_null), "{field}");
        assert_eq!(absent.config, json!({}));
    }
}

#[test]
fn shared_pure_normalization_matches_real_native_constructor_without_changing_declarations() {
    let document = profile(
        json!({"seed_urls":["https://EXAMPLE.test"],"indexability_agent":" GoogleBot ","max_pages":0,"max_depth":0,"explicit_sitemap_inputs":null,"max_body_bytes":1000003,"max_redirects":7,"discover_sitemaps":false}),
    );
    let declared = document.config.clone();
    let preview = preview_config_profile(&document).unwrap();
    let source: CrawlConfig = serde_json::from_value(declared.clone()).unwrap();
    let native = crate::Crawler::from_saved_config(source).unwrap();
    assert_eq!(
        serde_json::to_value(&preview.effective_config).unwrap(),
        serde_json::to_value(native.effective_config()).unwrap()
    );
    assert_eq!(document.config, declared);
    assert_eq!(
        preview.effective_config.seed_urls,
        ["https://example.test/"]
    );
    assert_eq!(preview.effective_config.indexability_agent, "googlebot");
    assert!(preview.effective_config.max_pages.is_none());
    assert!(preview.effective_config.max_depth.is_none());
    assert!(preview.effective_config.explicit_sitemap_inputs.is_none());
    let fields = preview
        .normalization
        .iter()
        .map(|change| change.field.as_str())
        .collect::<BTreeSet<_>>();
    for field in [
        "seed_urls",
        "indexability_agent",
        "max_pages",
        "max_depth",
        "input_imports",
    ] {
        assert!(fields.contains(field), "{field}");
    }
    let fresh = crate::Crawler::new(serde_json::from_value(document.config).unwrap()).unwrap();
    assert_eq!(
        fresh.effective_config().explicit_sitemap_inputs,
        Some(Vec::new())
    );
}

#[test]
fn all_headers_become_exact_explicit_requirements_and_resolve_without_secret_exports() {
    let raw=br#"{"seed_urls":["http://owned.test/"],"max_body_bytes":1000003,"custom_headers":{"Authorization":"first-owned-secret","authorization":"last-owned-secret","X-Owned-Token":"third-owned-secret"}}"#;
    let document = config_profile_from_json(raw).unwrap();
    assert_eq!(
        document.required_headers,
        ["Authorization", "X-Owned-Token", "authorization"]
    );
    assert_eq!(document.config["custom_headers"], json!({}));
    let bytes = export_config_profile(&document).unwrap();
    assert!(
        !String::from_utf8(bytes.clone())
            .unwrap()
            .contains("owned-secret")
    );
    let preview = preview_config_profile(&document).unwrap();
    assert_eq!(preview.missing_headers, document.required_headers);
    assert!(preview.effective_config.custom_headers.is_empty());
    assert!(resolve_config_profile(&document, &BTreeMap::new()).is_err());
    let values = BTreeMap::from([
        ("Authorization".into(), "first-owned-secret".into()),
        ("authorization".into(), "last-owned-secret".into()),
        ("X-Owned-Token".into(), "third-owned-secret".into()),
    ]);
    let resolved = resolve_config_profile(&document, &values).unwrap();
    assert_eq!(resolved.custom_headers, values);
    let prepared = prepare::prepare(resolved.clone(), false, true).unwrap();
    assert_eq!(
        prepared.headers.get("authorization").unwrap(),
        "last-owned-secret"
    );
    assert_eq!(resolved.max_body_bytes, 1000003);
    let mut extra = values.clone();
    extra.insert("x-owned-token".into(), "not-the-exact-name".into());
    assert!(resolve_config_profile(&document, &extra).is_err());
    let mut invalid = values;
    invalid.insert("authorization".into(), "hidden\r\nmalformed".into());
    let error = resolve_config_profile(&document, &invalid)
        .err()
        .unwrap()
        .to_string();
    assert!(!error.contains("hidden"));
    let reexported = config_profile_from_config(&resolved).unwrap();
    assert_eq!(reexported.required_headers, document.required_headers);
    assert_eq!(reexported.config["max_body_bytes"], 1000003);
    assert!(
        reexported.config["custom_headers"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    assert!(
        !String::from_utf8(export_config_profile(&reexported).unwrap())
            .unwrap()
            .contains("owned-secret")
    );
    let duplicate = CrawlConfigProfile {
        required_headers: vec!["X-A".into(), "X-A".into()],
        ..profile(json!({}))
    };
    assert!(preview_config_profile(&duplicate).is_err());
}

#[test]
fn strict_boundary_rejects_unknown_fields_arrays_duplicates_versions_and_typed_invalidity() {
    for config in [
        json!({"data_dir":"C:/unread-owned-path"}),
        json!({"renderer":{"browser_path":"unread"}}),
        json!({"renderer":[]}),
        json!({"audit_thresholds":{"new_thing":1}}),
        json!({"audit_thresholds":[]}),
        json!({"pdf_analysis":{"local_path":"unread"}}),
        json!({"custom_extractions":[{"name":"x","selector":"h1","unknown":true}]}),
        json!({"renderer":{"custom_javascript":[{"name":"x","script":"return 1","unknown":true}]}}),
        json!({"link_position_rules":[{"name":"x","selector":"a","unknown":true}]}),
        json!({"content_capture":{"sources":["raw_http_html"],"surprise":true}}),
        json!({"custom_search":{"version":1,"rules":[],"surprise":true}}),
        json!({"concurrency":0}),
        json!({"include_patterns":["["]}),
        json!({"custom_extractions":[{"name":"x","selector":"["}]}),
        json!({"user_agent":"invalid\r\nUA"}),
        json!({"custom_headers":{"Authorization":"must-not-accept-or-echo"}}),
    ] {
        let bytes = serde_json::to_vec(&json!({"version":1,"config":config,"required_headers":[]}))
            .unwrap();
        assert!(parse_config_profile(&bytes).is_err(), "{config}");
    }
    for raw in [
        r#"{"version":2,"config":{}}"#,
        r#"{"config":{}}"#,
        r#"{"version":1,"config":{},"unknown":true}"#,
        r#"{"version":1,"version":1,"config":{}}"#,
        r#"{"version":1,"config":{"renderer":{"wait_ms":1,"wait_ms":2}}}"#,
        r#"{"version":1,"config":{"custom_extractions":[{"namespaces":{"p":"one","p":"two"}}]}}"#,
        r#"{"version":1,"config":[]}"#,
        r#"[1,{}]"#,
    ] {
        assert!(parse_config_profile(raw.as_bytes()).is_err(), "{raw}");
    }
    // The new strict profile parser must not change old permissive typed DTOs.
    assert!(serde_json::from_value::<CrawlConfig>(json!({"unknown_legacy":true})).is_ok());
}

#[test]
fn strict_adapter_supports_free_maps_tagged_enums_and_rejects_nested_ignored_fields() {
    let config = json!({"custom_extractions":[{"name":"xpath","selector":"//h:main","method":"xpath","namespaces":{"h":"urn:owned"}}]});
    assert!(preview_config_profile(&profile(config)).is_ok());
    let input = crate::import_url_list(
        b"https://owned.test/",
        &crate::ListImportOptions {
            format: crate::ListImportFormat::Txt,
            limits: Default::default(),
        },
    )
    .unwrap();
    let config = json!({"input_imports":[input],"sitemap_urls":["https://owned.test/sitemap.xml"],"explicit_sitemap_inputs":["https://owned.test/sitemap.xml"]});
    let document = profile(config.clone());
    assert_eq!(document.config, config);
    let mut invalid = config;
    invalid["input_imports"][0]["options"]["format"]["unknown"] = json!(true);
    assert!(config_profile_from_json(&serde_json::to_vec(&invalid).unwrap()).is_err());
    let value: Value = parse_strict_json(br#"{"free":{"any_name":"value"}}"#, 1000).unwrap();
    assert_eq!(value["free"]["any_name"], "value");
}

#[test]
fn portable_tagged_unit_guards_cover_all_current_variants_without_restricting_free_maps() {
    for format in [crate::ListImportFormat::Txt, crate::ListImportFormat::Xml] {
        let tag = serde_json::to_value(&format).unwrap();
        let config = json!({"input_imports":[{"options":{"format":tag}}]});
        strict::validate_crawl_tagged_units(&config).unwrap();
        let mut invalid = config;
        invalid["input_imports"][0]["options"]["format"]["unknown"] = json!(true);
        assert!(strict::validate_crawl_tagged_units(&invalid).is_err());
        // The new portable guard must not change the legacy enum's acceptance.
        assert!(
            serde_json::from_value::<crate::ListImportFormat>(
                invalid["input_imports"][0]["options"]["format"].clone()
            )
            .is_ok()
        );
    }
    let metadata = serde_json::to_value(crate::ListFormulaMetadata::Unsupported).unwrap();
    let config = json!({"input_imports":[{"rows":[{"formula_metadata":metadata}]}]});
    strict::validate_crawl_tagged_units(&config).unwrap();
    let mut invalid = config;
    invalid["input_imports"][0]["rows"][0]["formula_metadata"]["unknown"] = json!(true);
    assert!(strict::validate_crawl_tagged_units(&invalid).is_err());
    assert!(
        serde_json::from_value::<crate::ListFormulaMetadata>(
            invalid["input_imports"][0]["rows"][0]["formula_metadata"].clone()
        )
        .is_ok()
    );
    // A matching tag string in a genuine free namespace map is just data.
    strict::validate_crawl_tagged_units(&json!({"custom_extractions":[{
        "namespaces":{"kind":"txt","unknown":"urn:owned"}
    }]}))
    .unwrap();
}

#[test]
fn portable_boundary_refuses_credential_urls_and_private_keys_without_echoing_them() {
    for config in [
        json!({"seed_urls":["https://owned-user:owned-password@owned.test/"]}),
        json!({"renderer":{"endpoint":"http://owned-user:owned-password@127.0.0.1/render"}}),
        json!({"tls":{"version":1,"trusted_certificates":[{"format":"pem","name":"owned.pem","data":"-----BEGIN PRIVATE KEY-----\nowned-private-bytes\n-----END PRIVATE KEY-----"}]}}),
    ] {
        let error = config_profile_from_json(&serde_json::to_vec(&config).unwrap())
            .err()
            .unwrap()
            .to_string();
        assert!(!error.contains("owned-password"));
        assert!(!error.contains("owned-private-bytes"));
    }
    let document = profile(
        json!({"renderer":{"endpoint":"http://127.0.0.1:1/render","wait_ms":0},"request_timeout_secs":20}),
    );
    assert!(preview_config_profile(&document).unwrap().requires_targets);
    assert!(
        config_profile_from_json(br#"{"renderer":{"endpoint":"https://remote.example/render"}}"#)
            .is_err()
    );
}

#[test]
fn actual_depth_node_and_physical64mib_boundaries_are_atomic() {
    let at_depth = format!(
        "{}0{}",
        "[".repeat(CONFIG_PROFILE_MAX_DEPTH),
        "]".repeat(CONFIG_PROFILE_MAX_DEPTH)
    );
    assert!(parse_strict_json::<Value>(at_depth.as_bytes(), CONFIG_PROFILE_BYTES).is_ok());
    let over_depth = format!("[{at_depth}]");
    assert!(parse_strict_json::<Value>(over_depth.as_bytes(), CONFIG_PROFILE_BYTES).is_err());
    let at_nodes = format!("[{}0]", "0,".repeat(CONFIG_PROFILE_MAX_NODES - 2));
    assert!(parse_strict_json::<Value>(at_nodes.as_bytes(), CONFIG_PROFILE_BYTES).is_ok());
    let over_nodes = format!("[0,{}]", &at_nodes[1..at_nodes.len() - 1]);
    assert!(parse_strict_json::<Value>(over_nodes.as_bytes(), CONFIG_PROFILE_BYTES).is_err());
    let mut physical = vec![b' '; CONFIG_PROFILE_BYTES];
    physical[0] = b'0';
    assert_eq!(
        parse_strict_json::<Value>(&physical, CONFIG_PROFILE_BYTES).unwrap(),
        json!(0)
    );
    physical.push(b' ');
    assert!(parse_strict_json::<Value>(&physical, CONFIG_PROFILE_BYTES).is_err());
    assert_eq!(
        parse_strict_json::<Value>(&physical, 128 * 1024 * 1024).unwrap(),
        json!(0),
        "App JSON-text wrapper has a distinct caller-defined byte budget"
    );
    drop(physical);
    let mut text = "x".repeat(CONFIG_PROFILE_BYTES - 2);
    let encoded = bounded_json(&text).unwrap();
    assert_eq!(encoded.len(), CONFIG_PROFILE_BYTES);
    drop(encoded);
    text.push('x');
    assert!(bounded_json(&text).is_err());
}

#[test]
fn resolved_declarations_are_not_premerged_before_fresh_original_sitemap_capture() {
    let input=crate::import_url_list(br#"<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc>https://owned.test/imported.xml</loc></sitemap></sitemapindex>"#,&crate::ListImportOptions{format:crate::ListImportFormat::Xml,limits:Default::default()}).unwrap();
    assert_eq!(
        input.rows[0].candidate_kind,
        crate::ListCandidateKind::SitemapDocument
    );
    let document = profile(
        json!({"seed_urls":["https://owned.test/"],"sitemap_urls":["https://owned.test/original.xml"],"input_imports":[input],"explicit_sitemap_inputs":null}),
    );
    let resolved = resolve_config_profile(&document, &BTreeMap::new()).unwrap();
    assert_eq!(resolved.sitemap_urls, ["https://owned.test/original.xml"]);
    assert!(resolved.explicit_sitemap_inputs.is_none());
    let fresh = crate::Crawler::new(resolved).unwrap();
    assert_eq!(
        fresh.effective_config().explicit_sitemap_inputs,
        Some(vec!["https://owned.test/original.xml".into()])
    );
    assert_eq!(
        fresh.effective_config().sitemap_urls,
        [
            "https://owned.test/original.xml",
            "https://owned.test/imported.xml"
        ]
    );
    let effective = preview_config_profile(&document).unwrap();
    assert!(effective.effective_config.explicit_sitemap_inputs.is_none());
    assert_eq!(
        document.config["sitemap_urls"],
        json!(["https://owned.test/original.xml"])
    );
}

#[test]
fn native_header_map_capacity_is_an_error_instead_of_a_panic_or_silent_drop() {
    let mut config = CrawlConfig {
        seed_urls: vec!["http://owned.test/".into()],
        ..Default::default()
    };
    for index in 0..32769 {
        config
            .custom_headers
            .insert(format!("x-owned-{index}"), "owned-value".into());
    }
    let result = std::panic::catch_unwind(|| prepare::prepare(config, false, true));
    assert!(result.is_ok(), "Too many headers must not panic");
    assert!(
        result
            .unwrap()
            .err()
            .unwrap()
            .to_string()
            .contains("header-map capacity")
    );
}
