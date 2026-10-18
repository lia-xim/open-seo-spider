use super::*;
use crate::CrawlConfig;
use reqwest::header::HeaderValue;

fn config() -> TlsConfig {
    TlsConfig {
        respect_hsts: true,
        ..Default::default()
    }
}
fn headers(value: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        "strict-transport-security",
        HeaderValue::from_str(value).unwrap(),
    );
    h
}
fn learned(cache: &HstsCache, host: &str, value: &str, now: u64) {
    cache
        .observe_at(
            &Url::parse(&format!("https://{host}/learn")).unwrap(),
            &headers(value),
            HstsSourceKind::Document,
            now,
        )
        .unwrap();
}
fn upgrade(cache: &HstsCache, url: &str, now: u64) -> Option<HstsUpgrade> {
    cache.upgrade_at(&Url::parse(url).unwrap(), 0, now).unwrap()
}

#[test]
fn tls_none_null_inactive_preserve_config_bytes_and_fingerprint_input() {
    let old = CrawlConfig::default();
    let bytes = serde_json::to_vec(&old).unwrap();
    for value in [None, Some(TlsConfig::default())] {
        let current = CrawlConfig {
            tls: value,
            ..old.clone()
        };
        assert_eq!(serde_json::to_vec(&current).unwrap(), bytes);
        assert!(serde_json::to_value(current).unwrap().get("tls").is_none());
    }
    let absent: CrawlConfig = serde_json::from_str("{}").unwrap();
    let null: CrawlConfig = serde_json::from_str(r#"{"tls":null}"#).unwrap();
    assert_eq!(
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&null).unwrap()
    );
}
#[test]
fn tls_strict_schema_unknown_version_types_and_duplicate_fields_are_refused() {
    for text in [
        r#"[1,false,[]]"#,
        r#"{"version":2}"#,
        r#"{"version":1,"respect_hsts":null}"#,
        r#"{"version":1,"extra":0}"#,
        r#"{"version":1,"version":1}"#,
        r#"{"respect_hsts":true}"#,
        r#"{"version":1,"trusted_certificates":[{"format":"pem","name":"x","data":"x","private_key":"x"}]}"#,
    ] {
        let parsed = serde_json::from_str::<TlsConfig>(text);
        assert!(
            parsed.is_err() || parsed.unwrap().validate().is_err(),
            "{text}"
        );
    }
}
#[test]
fn tls_private_keys_unknown_pem_objects_and_excess_input_are_refused() {
    for data in [
        "-----BEGIN PRIVATE KEY-----\nAA==\n-----END PRIVATE KEY-----",
        "-----BEGIN PUBLIC KEY-----\nAA==\n-----END PUBLIC KEY-----",
        "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\nTRAILING",
    ] {
        assert!(
            TlsConfig {
                trusted_certificates: vec![TrustedCertificate::Pem {
                    name: "ca.pem".into(),
                    data: data.into()
                }],
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    assert!(TrustedCertificate::from_file_bytes("oversize.der".into(), &vec![0; 65537]).is_err());
    assert!(
        TlsConfig {
            trusted_certificates: vec![TrustedCertificate::Der {
                name: "x".into(),
                data: "AA".into()
            }],
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}
#[test]
fn tls_file_import_ascii_whitespace_preserves_real_public_pem_and_der_boundaries() {
    use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let key = KeyPair::generate().unwrap();
    let certificate = params.self_signed(&key).unwrap();
    let public_pem = certificate.pem();
    for prefix in [
        "",
        "\n",
        " ",
        "\t",
        "\r",
        "\u{000b}",
        "\u{000c}",
        " \t\r\n\u{000b}\u{000c}",
    ] {
        let raw = format!("{prefix}{public_pem}");
        let imported =
            TrustedCertificate::from_file_bytes("public.pem".into(), raw.as_bytes()).unwrap();
        assert!(
            matches!(&imported, TrustedCertificate::Pem { data, .. } if data.as_bytes() == raw.as_bytes())
        );
        let profile = TlsConfig {
            trusted_certificates: vec![imported],
            ..Default::default()
        };
        profile.validate().unwrap();
        profile
            .configure(reqwest::Client::builder())
            .unwrap()
            .build()
            .unwrap();
        // The independently supplied config-object entry point sees identical bytes.
        let direct = TlsConfig {
            trusted_certificates: vec![TrustedCertificate::Pem {
                name: "public.pem".into(),
                data: raw,
            }],
            ..Default::default()
        };
        direct
            .configure(reqwest::Client::builder())
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(profile.digest().unwrap(), direct.digest().unwrap());
    }
    let imported_der =
        TrustedCertificate::from_file_bytes("public.der".into(), certificate.der().as_ref())
            .unwrap();
    assert!(matches!(&imported_der, TrustedCertificate::Der { .. }));
    assert_eq!(imported_der.decoded().unwrap(), certificate.der().as_ref());
    TlsConfig {
        trusted_certificates: vec![imported_der],
        ..Default::default()
    }
    .configure(reqwest::Client::builder())
    .unwrap()
    .build()
    .unwrap();
    for raw in [
        format!(" \n{}", key.serialize_pem()),
        " \t-----BEGIN PUBLIC KEY-----\nAA==\n-----END PUBLIC KEY-----".into(),
    ] {
        assert!(
            TrustedCertificate::from_file_bytes("rejected.pem".into(), raw.as_bytes()).is_err()
        );
    }
    // BOM, Unicode whitespace and arbitrary preambles do not acquire PEM status.
    // Both file import and profile validation complete the same pure X509
    // trust-anchor check used by Reqwest's rustls ClientBuilder.
    for prefix in ["\u{feff}", "\u{00a0}", "arbitrary preamble\n"] {
        let raw = format!("{prefix}{public_pem}");
        assert!(TrustedCertificate::from_file_bytes("not-pem.cer".into(), raw.as_bytes()).is_err());
        assert!(
            TlsConfig {
                trusted_certificates: vec![TrustedCertificate::Pem {
                    name: "not-pem.pem".into(),
                    data: raw,
                }],
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
}

#[test]
fn pure_trust_validation_accepts_native_root_intermediate_and_leaf_anchors() {
    use rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair};

    let mut root_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    root_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let root_key = KeyPair::generate().unwrap();
    let root = root_params.self_signed(&root_key).unwrap();
    let root_issuer = Issuer::new(root_params, root_key);
    let mut intermediate_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    intermediate_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    let intermediate_key = KeyPair::generate().unwrap();
    let intermediate = intermediate_params
        .signed_by(&intermediate_key, &root_issuer)
        .unwrap();
    let intermediate_issuer = Issuer::new(intermediate_params, intermediate_key);
    let leaf_params = CertificateParams::new(vec!["localhost".into()]).unwrap();
    let leaf_key = KeyPair::generate().unwrap();
    let leaf = leaf_params
        .signed_by(&leaf_key, &intermediate_issuer)
        .unwrap();

    // Native RootCertStore::add accepts certificates as trust anchors, including
    // intermediates and non-CA leaves; preview must not invent a stricter policy.
    let certificates = [&root, &intermediate, &leaf];
    let mut native = RootCertStore::empty();
    for certificate in certificates {
        native.add(certificate.der().clone()).unwrap();
        for (name, bytes) in [
            ("anchor.der", certificate.der().as_ref().to_vec()),
            ("anchor.pem", certificate.pem().into_bytes()),
        ] {
            let imported = TrustedCertificate::from_file_bytes(name.into(), &bytes).unwrap();
            let tls = TlsConfig {
                trusted_certificates: vec![imported],
                ..Default::default()
            };
            assert_eq!(tls.certificates().unwrap().len(), 1);
            tls.validate().unwrap();
        }
    }
    assert_eq!(native.len(), 3);

    let bundle = format!("{}{}{}", root.pem(), intermediate.pem(), leaf.pem());
    let imported =
        TrustedCertificate::from_file_bytes("bundle.pem".into(), bundle.as_bytes()).unwrap();
    let tls = TlsConfig {
        trusted_certificates: vec![imported],
        ..Default::default()
    };
    assert_eq!(tls.certificates().unwrap().len(), 3);
    let declared = serde_json::json!({"tls": tls});
    let raw = serde_json::to_vec(&declared).unwrap();
    let document = crate::config_profile::config_profile_from_json(&raw).unwrap();
    assert_eq!(document.config, declared);
    let preview = crate::config_profile::preview_config_profile(&document).unwrap();
    assert!(preview.requires_targets);
    assert_eq!(
        preview
            .effective_config
            .tls
            .as_ref()
            .unwrap()
            .trusted_certificates
            .len(),
        1
    );
    assert_eq!(
        document.config["tls"]["trusted_certificates"][0]["data"],
        bundle
    );
}

#[test]
fn invalid_x509_is_refused_by_pure_validation_and_all_profile_entrypoints() {
    use crate::config_profile::{
        CrawlConfigProfile, config_profile_from_config, config_profile_from_json,
        parse_config_profile, preview_config_profile,
    };

    for certificate in [
        TrustedCertificate::Der {
            name: "invalid.der".into(),
            data: "00".into(),
        },
        TrustedCertificate::Der {
            name: "invalid.der".into(),
            data: "3000".into(),
        },
        TrustedCertificate::Pem {
            name: "invalid.pem".into(),
            data: "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----".into(),
        },
    ] {
        let tls = TlsConfig {
            trusted_certificates: vec![certificate],
            ..Default::default()
        };
        assert!(tls.validate().is_err());
        let config = CrawlConfig {
            tls: Some(tls.clone()),
            ..Default::default()
        };
        assert!(config_profile_from_config(&config).is_err());
        let declared = serde_json::json!({"tls": tls});
        assert!(config_profile_from_json(&serde_json::to_vec(&declared).unwrap()).is_err());
        let document = CrawlConfigProfile {
            version: 1,
            config: declared,
            required_headers: Vec::new(),
        };
        assert!(parse_config_profile(&serde_json::to_vec(&document).unwrap()).is_err());
        assert!(preview_config_profile(&document).is_err());
    }
}

#[test]
fn hsts_parser_quoted_unknown_order_empty_segments_and_case() {
    assert_eq!(
        parse_sts(" ; X=\"a;b\"; InCLudeSubDomains; MAX-AGE=\"12\";;"),
        Some((12, true))
    );
    assert_eq!(parse_sts(r#"max-age="\1\2""#), Some((12, false)));
    assert_eq!(
        parse_sts("max-age=999999999999999999999999999999999"),
        Some((u64::MAX, false))
    );
}
#[test]
fn hsts_parser_rejects_duplicates_missing_age_bad_syntax_and_unknown_duplicates() {
    for value in [
        "includeSubDomains",
        "max-age=-1",
        "max-age=1.5",
        "max-age=1,max-age=2",
        "max-age=1;MAX-AGE=2",
        "max-age=1;includeSubDomains=1",
        "max-age=1;x=a;x=b",
        "max-age=1;bad name=x",
        "max-age=\"1",
        "max-age=1;\r\nx=a",
        "max-age=1; x=\"a\"oops",
    ] {
        assert_eq!(parse_sts(value), None, "{value}");
    }
}
#[test]
fn hsts_first_header_only_and_insecure_header_ignored() {
    let cache = HstsCache::new(&config()).unwrap();
    let mut h = headers("max-age=0");
    h.append(
        "strict-transport-security",
        HeaderValue::from_static("max-age=100"),
    );
    cache
        .observe_at(
            &Url::parse("https://example.test/").unwrap(),
            &h,
            HstsSourceKind::Document,
            10,
        )
        .unwrap();
    assert!(upgrade(&cache, "http://example.test/", 11).is_none());
    cache
        .observe_at(
            &Url::parse("http://example.test/").unwrap(),
            &headers("max-age=100"),
            HstsSourceKind::Document,
            10,
        )
        .unwrap();
    assert!(upgrade(&cache, "http://example.test/", 11).is_none());
}
#[test]
fn hsts_invalid_header_and_missing_header_do_not_delete_prior_policy() {
    let cache = HstsCache::new(&config()).unwrap();
    learned(&cache, "a.test", "max-age=100", 10);
    cache
        .observe_at(
            &Url::parse("https://a.test/").unwrap(),
            &headers("max-age=0;max-age=1"),
            HstsSourceKind::Document,
            11,
        )
        .unwrap();
    cache
        .observe_at(
            &Url::parse("https://a.test/").unwrap(),
            &HeaderMap::new(),
            HstsSourceKind::Document,
            11,
        )
        .unwrap();
    assert!(upgrade(&cache, "http://a.test/", 12).is_some());
}
#[test]
fn hsts_expiry_boundary_and_max_age_zero_delete_exact_host() {
    let cache = HstsCache::new(&config()).unwrap();
    learned(&cache, "a.test", "max-age=10", 10);
    assert!(upgrade(&cache, "http://a.test/", 19).is_some());
    assert!(upgrade(&cache, "http://a.test/", 20).is_none());
    learned(&cache, "a.test", "max-age=100;includeSubDomains", 21);
    learned(&cache, "a.test", "max-age=0", 22);
    assert!(upgrade(&cache, "http://a.test/", 23).is_none());
    assert!(upgrade(&cache, "http://x.a.test/", 23).is_none());
}
#[test]
fn hsts_parent_policy_remains_after_child_delete_and_label_boundary_is_exact() {
    let cache = HstsCache::new(&config()).unwrap();
    learned(&cache, "example.test", "max-age=100; includeSubDomains", 1);
    learned(&cache, "child.example.test", "max-age=5", 1);
    learned(&cache, "child.example.test", "max-age=0", 2);
    assert!(upgrade(&cache, "http://child.example.test/", 3).is_some());
    assert!(upgrade(&cache, "http://badexample.test/", 3).is_none());
    assert!(upgrade(&cache, "http://example.test.evil/", 3).is_none());
}
#[test]
fn hsts_ip_literals_never_learn_or_upgrade() {
    let cache = HstsCache::new(&config()).unwrap();
    for host in ["127.0.0.1", "[::1]"] {
        learned(&cache, host, "max-age=100;includeSubDomains", 10);
        assert!(upgrade(&cache, &format!("http://{host}/"), 11).is_none());
    }
}
#[test]
fn hsts_ports_path_query_idna_and_synthetic_provenance() {
    let cache = HstsCache::new(&config()).unwrap();
    learned(&cache, "xn--bcher-kva.test", "max-age=100", 10);
    for (input, expected) in [
        (
            "http://BÜCHER.test:80/a?q=1",
            "https://xn--bcher-kva.test/a?q=1",
        ),
        (
            "http://xn--bcher-kva.test:8080/a",
            "https://xn--bcher-kva.test:8080/a",
        ),
        (
            "http://xn--bcher-kva.test:443/a",
            "https://xn--bcher-kva.test/a",
        ),
    ] {
        let hop = upgrade(&cache, input, 11).unwrap();
        assert_eq!(hop.target_url, expected);
        assert_eq!(hop.synthetic_status, 307);
        assert!(!hop.network_request_made);
        assert!(!hop.https_request_started);
        assert_eq!(hop.reason, "HSTS Policy");
    }
    assert!(upgrade(&cache, "https://xn--bcher-kva.test/a", 11).is_none());
}
#[test]
fn hsts_without_include_subdomains_does_not_upgrade_children() {
    let cache = HstsCache::new(&config()).unwrap();
    learned(&cache, "example.test", "max-age=100", 10);
    assert!(upgrade(&cache, "http://example.test/", 11).is_some());
    assert!(upgrade(&cache, "http://child.example.test/", 11).is_none());
}
#[test]
fn hsts_persistence_resume_and_stale_writer_failure_remain_bound_to_generation() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(crate::CrawlStore::open(dir.path().join("crawl.sqlite")).unwrap());
    let cfg = CrawlConfig {
        seed_urls: vec!["https://example.test/".into()],
        tls: Some(config()),
        ..Default::default()
    };
    store.prepare(&cfg).unwrap();
    let first = store.begin_run("first").unwrap();
    let cache = HstsCache::new(&config()).unwrap();
    cache.attach(store.clone(), first).unwrap();
    let now = unix_now();
    learned(
        &cache,
        "example.test",
        "max-age=3600;includeSubDomains",
        now,
    );
    let second = store.begin_run("resume").unwrap();
    let resumed = HstsCache::new(&config()).unwrap();
    resumed.attach(store.clone(), second).unwrap();
    assert!(upgrade(&resumed, "http://child.example.test/", now + 1).is_some());
    assert!(
        cache
            .observe_at(
                &Url::parse("https://example.test/").unwrap(),
                &headers("max-age=0"),
                HstsSourceKind::Document,
                now + 2
            )
            .is_err()
    );
    assert!(cache.healthy().is_err());
    assert!(
        cache.attach(store.clone(), second).is_err(),
        "sticky failure cannot be erased by reattachment"
    );
    assert!(upgrade(&resumed, "http://example.test/", now + 3).is_some());
    let changed = CrawlConfig {
        tls: Some(TlsConfig::default()),
        ..cfg
    };
    assert!(
        store.prepare(&changed).is_err(),
        "resume cannot substitute trust/HSTS config"
    );
}
#[test]
fn tls_renderer_active_combination_is_explicit_partial_refusal() {
    let mut cfg = CrawlConfig {
        seed_urls: vec!["https://example.test/".into()],
        tls: Some(config()),
        ..Default::default()
    };
    cfg.renderer = Some(Default::default());
    assert!(
        crate::Crawler::new(cfg)
            .err()
            .unwrap()
            .to_string()
            .contains("not yet supported")
    );
}
#[test]
fn tls_legacy_page_json_and_csv_have_no_added_fields_or_columns() {
    let page = crate::PageRecord::default();
    assert!(serde_json::to_value(&page).unwrap().get("tls").is_none());
    let mut report = crate::CrawlReport {
        config: Default::default(),
        pages: vec![page],
        issues: vec![],
        summary: Default::default(),
        warnings: vec![],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
        sitemap_source_measurements: None,
    };
    let legacy = crate::export_pages_csv(&report).unwrap();
    assert!(!legacy.contains("HSTS Upgrade Evidence"));
    report.config.tls = Some(config());
    let active = crate::export_pages_csv(&report).unwrap();
    assert!(active.contains("Local Transport Upgrade,HSTS Upgrade Evidence"));
    report.config.tls = Some(TlsConfig::default());
    assert_eq!(crate::export_pages_csv(&report).unwrap(), legacy);
}
#[test]
fn hsts_resource_gate_intersects_coalesced_roles_across_later_server_redirect() {
    use crate::{
        ResourceContractConfig, ResourceDiscoveryConfig, ResourceGrant, ResourceReferenceKind as K,
        ResourceTaskPermission,
    };
    let mut cfg = ResourceDiscoveryConfig {
        contract: Some(ResourceContractConfig::default()),
        ..Default::default()
    };
    cfg.resource_scope.external_origins = vec!["http://cdn.test".into()];
    let grant = |id: &str, kind: K| ResourceGrant {
        engine: crate::RESOURCE_DISCOVERY_ENGINE.into(),
        config_sha256: cfg.sha256().unwrap(),
        occurrence_id: id.into(),
        fetch_url: "http://cdn.test/a".into(),
        reference_kind: Some(kind),
        store: true,
        can_expand_navigation: kind == K::InternalAnchor,
        can_expand_resources: kind == K::InternalAnchor,
        can_expand_css: kind == K::Css,
        can_expand_pdf: false,
        policy_sha256: None,
        policy_occurrence_id: None,
    };
    let mut permission = ResourceTaskPermission::new(grant("nav", K::InternalAnchor));
    permission.merge(&grant("css", K::Css)).unwrap();
    let seeds = vec![Url::parse("https://app.test/").unwrap()];
    let from = Url::parse("http://cdn.test/a").unwrap();
    let to = Url::parse("https://cdn.test/a").unwrap();
    let hsts_gate = cfg
        .redirect_decision(
            &permission,
            &from,
            to.as_str(),
            Some(&to),
            &seeds,
            false,
            true,
            None,
            &[],
        )
        .unwrap();
    assert_eq!(hsts_gate.authorizing_occurrence_ids, vec!["nav"]);
    let back = cfg
        .redirect_decision(
            &permission,
            &to,
            from.as_str(),
            Some(&from),
            &seeds,
            false,
            true,
            None,
            std::slice::from_ref(&hsts_gate),
        )
        .unwrap();
    assert_eq!(
        back.authorizing_occurrence_ids,
        vec!["nav"],
        "CSS grant cannot reappear after an HTTPS transport gate"
    );
    let cache = HstsCache::new(&config()).unwrap();
    learned(&cache, "cdn.test", "max-age=100", 10);
    let mut hop = upgrade(&cache, from.as_str(), 11).unwrap();
    hop.resource_gate = Some(hsts_gate);
    hop.https_request_started = true;
    let page = crate::PageRecord {
        url: from.to_string(),
        final_url: from.to_string(),
        final_status: Some(200),
        redirect_chain: vec![crate::RedirectHop {
            url: to.to_string(),
            status: 302,
            target: from.to_string(),
        }],
        tls: Some(PageTlsObservation {
            hsts_upgrades: vec![hop],
        }),
        resource_response: Some(crate::ResourceResponseFacts {
            engine: crate::RESOURCE_DISCOVERY_ENGINE.into(),
            contract_engine: Some(crate::RESOURCE_CONTRACT_ENGINE.into()),
            redirect_decisions: vec![back],
            selected_materialization_reason: None,
            observed_generation: 1,
            retained_response: true,
            entity_body_retained: false,
            captured_entity_sha256: None,
            captured_entity_bytes: 0,
            captured_entity_complete: true,
            decoded_source_sha256: None,
            decode_status: crate::ContentCaptureDecodeStatus::Exact,
            decode_encoding: None,
            metadata_projection_complete: true,
            unretained_fields: vec![],
        }),
        ..Default::default()
    };
    for captured in [false, true] {
        assert_eq!(
            cfg.eligible_authority(&page, &permission, &seeds, false, true, captured)
                .unwrap(),
            HashSet::from(["nav".to_owned()])
        );
    }
}
#[test]
fn hsts_persisted_policy_recomputes_first_field_and_refuses_cross_config_or_future_generation() {
    let cache = HstsCache::new(&config()).unwrap();
    learned(&cache, "example.test", "max-age=100;includeSubDomains", 10);
    let policies = cache.inner.lock().unwrap().policies.clone();
    let state = HstsSnapshot {
        version: 1,
        tls_config_sha256: config().digest().unwrap(),
        generation: 2,
        policies,
    };
    state.validate(&config().digest().unwrap(), 2).unwrap();
    assert!(state.validate("wrong-config", 2).is_err());
    assert!(state.validate(&config().digest().unwrap(), 1).is_err());
    let mut tampered = state.clone();
    tampered
        .policies
        .get_mut("example.test")
        .unwrap()
        .include_subdomains = false;
    assert!(tampered.validate(&config().digest().unwrap(), 2).is_err());
    let mut tampered = state.clone();
    tampered
        .policies
        .get_mut("example.test")
        .unwrap()
        .expires_at_unix += 1;
    assert!(tampered.validate(&config().digest().unwrap(), 2).is_err());
    let mut tampered = state;
    tampered
        .policies
        .get_mut("example.test")
        .unwrap()
        .learned_from_url = "http://example.test/".into();
    assert!(tampered.validate(&config().digest().unwrap(), 2).is_err());
}
#[test]
fn tls_public_certificate_file_total_and_bundle_limits_are_exact() {
    let rcgen::CertifiedKey { cert, .. } =
        rcgen::generate_simple_self_signed(vec!["fixture.test".into()]).unwrap();
    let pem = cert.pem();
    let padded = format!("{}{}", pem, " ".repeat(65536 - pem.len()));
    let files = (0..4)
        .map(|n| TrustedCertificate::Pem {
            name: format!("{n}.pem"),
            data: padded.clone(),
        })
        .collect::<Vec<_>>();
    let mut cfg = TlsConfig {
        trusted_certificates: files,
        ..Default::default()
    };
    cfg.validate().unwrap();
    cfg.trusted_certificates.push(TrustedCertificate::Pem {
        name: "extra.pem".into(),
        data: pem.clone(),
    });
    assert!(
        cfg.validate().is_err(),
        "262144 decoded bytes is a real aggregate boundary"
    );
    let mut cfg = TlsConfig {
        trusted_certificates: vec![TrustedCertificate::Pem {
            name: "bundle.pem".into(),
            data: pem.repeat(32),
        }],
        ..Default::default()
    };
    cfg.validate().unwrap();
    cfg.trusted_certificates[0] = TrustedCertificate::Pem {
        name: "bundle.pem".into(),
        data: pem.repeat(33),
    };
    assert!(
        cfg.validate().is_err(),
        "33 certificates are refused even in one bounded file"
    );
    cfg.trusted_certificates = (0..9)
        .map(|n| TrustedCertificate::Pem {
            name: format!("{n}.pem"),
            data: pem.clone(),
        })
        .collect();
    assert!(cfg.validate().is_err(), "9 files are refused");
}
