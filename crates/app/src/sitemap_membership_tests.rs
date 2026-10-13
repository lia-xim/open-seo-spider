//! Pure protocol/projection controls, not stored-crawl authenticity evidence.
use super::*;
fn snapshot() -> Snapshot {
    Snapshot {
        job_id: Uuid::nil(),
        crawl_generation: 1,
        crawl_config_sha256: "a".repeat(64),
        catalog_config_sha256: "b".repeat(64),
        catalog_revision: 2,
        catalog_rolling_sha256: "c".repeat(64),
        catalog_state_sha256: "d".repeat(64),
        inherited_basis: None,
    }
}
#[test]
fn canonical_cursor_rejects_malformed_tampered_unknown_and_noncanonical_bytes() {
    let current = snapshot();
    let good = token(&current, &Filter::default(), Some(1)).unwrap();
    assert!(decode(&good).is_ok());
    for bad in [
        String::new(),
        "!".into(),
        "a".repeat(4097),
        format!("{good}="),
    ] {
        assert_eq!(decode(&bad).unwrap_err().code, "invalid_sitemap_cursor");
    }
    let mut value: serde_json::Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(&good).unwrap()).unwrap();
    value["extra"] = serde_json::json!(true);
    let unknown = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&value).unwrap());
    assert_eq!(decode(&unknown).unwrap_err().code, "invalid_sitemap_cursor");
    let bytes = URL_SAFE_NO_PAD.decode(&good).unwrap();
    let spaced = URL_SAFE_NO_PAD.encode([b" ".as_slice(), bytes.as_slice()].concat());
    assert_eq!(decode(&spaced).unwrap_err().code, "invalid_sitemap_cursor");
    let mut parsed = decode(&good).unwrap();
    parsed.payload.after_sequence = Some(2);
    let changed = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&parsed).unwrap());
    assert_eq!(decode(&changed).unwrap_err().code, "invalid_sitemap_cursor");
}
#[test]
fn cursor_pins_job_generation_config_revision_rolling_inheritance_and_filter() {
    let current = snapshot();
    let filter = Filter::default();
    let good = token(&current, &filter, Some(1)).unwrap();
    assert_eq!(
        validate_token(&good, &current, &filter, true).unwrap(),
        Some(1)
    );
    let mut variants = Vec::new();
    let mut next = current.clone();
    next.job_id = Uuid::new_v4();
    variants.push(next);
    let mut next = current.clone();
    next.crawl_generation += 1;
    variants.push(next);
    let mut next = current.clone();
    next.crawl_config_sha256 = "e".repeat(64);
    variants.push(next);
    let mut next = current.clone();
    next.catalog_config_sha256 = "e".repeat(64);
    variants.push(next);
    let mut next = current.clone();
    next.catalog_revision += 1;
    variants.push(next);
    let mut next = current.clone();
    next.catalog_rolling_sha256 = "e".repeat(64);
    variants.push(next);
    let mut next = current.clone();
    next.catalog_state_sha256 = "e".repeat(64);
    variants.push(next);
    let mut next = current.clone();
    next.inherited_basis = Some(SitemapInheritedBasis {
        parent_snapshot_sha256: "e".repeat(64),
        parent_catalog_sha256: "f".repeat(64),
        current_discovery_executed: false,
    });
    variants.push(next);
    for changed in variants {
        assert_eq!(
            validate_token(&good, &changed, &filter, true)
                .unwrap_err()
                .code,
            "stale_sitemap_cursor"
        );
    }
    let changed = Filter {
        kind: RowKind::Location,
        ..Default::default()
    };
    assert_eq!(
        validate_token(&good, &current, &changed, true)
            .unwrap_err()
            .code,
        "stale_sitemap_cursor"
    );
}
#[test]
fn query_bounds_reject_without_clamping_or_alias_normalization() {
    for limit in [0, 201, usize::MAX] {
        assert_eq!(
            ReadQuery {
                limit: Some(limit),
                ..Default::default()
            }
            .validate()
            .unwrap_err()
            .code,
            "invalid_sitemap_query"
        );
    }
    assert_eq!(ReadQuery::default().validate().unwrap(), 100);
    assert_eq!(
        ReadQuery {
            limit: Some(200),
            ..Default::default()
        }
        .validate()
        .unwrap(),
        200
    );
    for member in [
        "/relative",
        "https://e.test/p#fragment",
        " https://e.test/p ",
        "ftp://e.test/p",
        "https://user:secret@e.test/p",
        "https://e.test/\tbad",
    ] {
        assert_eq!(
            Filter {
                membership_url: Some(member.into()),
                ..Default::default()
            }
            .validate()
            .unwrap_err()
            .code,
            "invalid_sitemap_query"
        );
    }
    assert!(
        Filter {
            membership_url: Some("https://e.test/p?a=1&b=2".into()),
            ..Default::default()
        }
        .validate()
        .is_ok()
    );
    assert!(
        Filter {
            source_observation_id: Some("a".repeat(64)),
            ..Default::default()
        }
        .validate()
        .is_ok()
    );
    assert!(
        Filter {
            source_observation_id: Some("A".repeat(64)),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}
#[test]
fn original_membership_filter_never_matches_policy_fetch_alias() {
    // Synthetic DTO filter control only; never passed as a validated SQL snapshot.
    let row = SitemapLedgerRow::Root(open_seo_spider_core::SitemapRootDeclaration {
        sequence: 1,
        declaration_id: "root".into(),
        kind: open_seo_spider_core::SitemapRootKind::ExplicitConfig,
        source_observation_id: None,
        source_ordinal: 0,
        raw_line: None,
        raw_value: "https://e.test/original".into(),
        normalized_document_url: Some("https://e.test/original".into()),
        robots_evidence: None,
        admission: open_seo_spider_core::SitemapAdmissionBinding {
            decision: open_seo_spider_core::SitemapAdmissionDecision::Scheduled,
            membership_url: Some("https://e.test/original".into()),
            admitted_fetch_url: Some("https://e.test/alias".into()),
            coalesced_policy_occurrence_id: None,
            policy_occurrence_id: Some("policy".into()),
            policy_source_revision: Some("revision".into()),
        },
    });
    assert!(
        Filter {
            membership_url: Some("https://e.test/original".into()),
            ..Default::default()
        }
        .matches(&row)
    );
    assert!(
        !Filter {
            membership_url: Some("https://e.test/alias".into()),
            ..Default::default()
        }
        .matches(&row)
    );
    assert!(
        !Filter {
            kind: RowKind::Location,
            ..Default::default()
        }
        .matches(&row)
    );
}
#[test]
fn csv_writer_is_exact_at_cap_rejects_plus_one_and_escapes_original_text() {
    let mut bounded = BoundedBytes {
        bytes: Vec::new(),
        max: 5,
    };
    bounded.write_all(b"12345").unwrap();
    assert!(bounded.write_all(b"6").is_err());
    assert_eq!(bounded.bytes, b"12345");
    let mut output = Vec::new();
    csv_record(
        &mut output,
        &[
            "ä,日\n\"x\"".into(),
            open_seo_spider_core::csv_text("=1+1"),
            "".into(),
        ],
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "\"ä,日\n\"\"x\"\"\",\"'=1+1\",\"\"\r\n"
    );
}
#[test]
fn export_and_continuation_roles_and_zero_sequence_are_not_interchangeable() {
    let current = snapshot();
    let filter = Filter::default();
    let export = token(&current, &filter, None).unwrap();
    assert_eq!(
        validate_token(&export, &current, &filter, true)
            .unwrap_err()
            .code,
        "invalid_sitemap_cursor"
    );
    assert!(
        validate_token(&export, &current, &filter, false)
            .unwrap()
            .is_none()
    );
    let cursor = token(&current, &filter, Some(1)).unwrap();
    assert_eq!(
        validate_token(&cursor, &current, &filter, false)
            .unwrap_err()
            .code,
        "invalid_sitemap_cursor"
    );
    let zero = token(&current, &filter, Some(0)).unwrap();
    assert_eq!(
        ReadQuery {
            cursor: Some(zero),
            ..Default::default()
        }
        .validate()
        .unwrap_err()
        .code,
        "invalid_sitemap_cursor"
    );
}
