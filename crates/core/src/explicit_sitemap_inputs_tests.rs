//! Candidate for crates/core/tests/explicit_sitemap_inputs.rs.
//! AUTHORED ONLY: no compile or test execution by this Work author.
//! No HTTP/server/native executable is started by these regression candidates.
use crate::{
    CrawlConfig, CrawlReport, CrawlStore, CrawlSummary, Crawler, ListImportFormat,
    ListImportLimits, ListImportOptions, PageRecord, RecrawlSourceKind, SelectedRecrawlParent,
    import_url_list, prepare_selected_recrawl,
};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

fn raw() -> Vec<String> {
    vec![
        "HTTPS://EXAMPLE.TEST/explicit.xml#original".into(),
        "HTTPS://EXAMPLE.TEST/explicit.xml#original".into(),
    ]
}
fn config() -> CrawlConfig {
    CrawlConfig {
        seed_urls: vec!["https://example.test/".into()],
        sitemap_urls: raw(),
        obey_robots: false,
        delay_ms: 0,
        ..Default::default()
    }
}
fn index_import() -> crate::ListImportResult {
    import_url_list(
        br#"<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc>https://example.test/imported.xml</loc></sitemap><sitemap><loc>https://example.test/imported.xml</loc></sitemap></sitemapindex>"#,
        &ListImportOptions {
            format: ListImportFormat::Xml,
            limits: ListImportLimits::default(),
        },
    )
    .unwrap()
}
fn existing_store(saved: &CrawlConfig) -> (tempfile::TempDir, PathBuf, String) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("owned-fixture.sqlite");
    drop(CrawlStore::open(&path).unwrap());
    let text = serde_json::to_string(saved).unwrap();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "INSERT INTO metadata(key,value) VALUES('config',?1)",
            [&text],
        )
        .unwrap();
    drop(connection);
    (temp, path, text)
}
fn stored_text(path: &Path) -> String {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap()
        .query_row("SELECT value FROM metadata WHERE key='config'", [], |row| {
            row.get(0)
        })
        .unwrap()
}
fn selected_snapshot(original: Option<Vec<String>>) -> Vec<u8> {
    serde_json::to_vec(&CrawlReport {
        config: CrawlConfig {
            seed_urls: vec!["https://example.test/".into()],
            sitemap_urls: vec!["https://example.test/parent.xml".into()],
            explicit_sitemap_inputs: original,
            obey_robots: false,
            ..Default::default()
        },
        pages: vec![PageRecord {
            url: "https://example.test/selected".into(),
            ..Default::default()
        }],
        issues: vec![],
        summary: CrawlSummary::default(),
        warnings: vec![],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
    })
    .unwrap()
}
fn selected(bytes: &[u8]) -> crate::PreparedSelectedRecrawl {
    prepare_selected_recrawl(
        SelectedRecrawlParent {
            job_id: "owned-input-fixture".into(),
            checkpoint_revision: Some(1),
            source_kind: RecrawlSourceKind::CapturedCheckpoint,
        },
        bytes,
        &["https://example.test/selected".into()],
        None,
    )
    .unwrap()
}

#[test]
fn fresh_capture_precedes_xml_import_merge_and_keeps_raw_duplicate_occurrences() {
    let mut input = config();
    input.input_imports.push(index_import());
    let crawler = Crawler::new(input).unwrap();
    let effective = crawler.effective_config();
    assert_eq!(effective.explicit_sitemap_inputs, Some(raw()));
    assert_eq!(effective.input_imports[0].rows.len(), 2);
    assert_eq!(
        effective
            .sitemap_urls
            .iter()
            .filter(|url| url.as_str() == "https://example.test/explicit.xml")
            .count(),
        2
    );
    assert_eq!(
        effective
            .sitemap_urls
            .iter()
            .filter(|url| url.as_str() == "https://example.test/imported.xml")
            .count(),
        1
    );
}

#[test]
fn fresh_import_only_capture_is_known_empty_and_never_relabels_imported_urls() {
    let input = CrawlConfig {
        input_imports: vec![index_import()],
        ..Default::default()
    };
    let crawler = Crawler::new(input).unwrap();
    assert_eq!(
        crawler.effective_config().explicit_sitemap_inputs,
        Some(vec![])
    );
    assert_eq!(
        crawler.effective_config().sitemap_urls,
        ["https://example.test/imported.xml"]
    );
}

#[test]
fn repeated_prepare_and_fresh_normalization_preserve_the_declared_raw_vector() {
    let mut input = config();
    input.input_imports.push(index_import());
    input.capture_explicit_sitemap_inputs();
    let crawler = Crawler::new(input).unwrap();
    let mut effective = crawler.effective_config().clone();
    let before = serde_json::to_value(&effective).unwrap();
    for _ in 0..3 {
        effective.capture_explicit_sitemap_inputs();
        effective.prepare_input_lists().unwrap();
        effective = Crawler::new(effective).unwrap().effective_config().clone();
        assert_eq!(serde_json::to_value(&effective).unwrap(), before);
        assert_eq!(effective.explicit_sitemap_inputs, Some(raw()));
    }
}

#[test]
fn ordinary_prepare_preserves_legacy_none_while_merging_imports() {
    let mut saved = CrawlConfig {
        input_imports: vec![index_import()],
        ..Default::default()
    };
    saved.prepare_input_lists().unwrap();
    saved.prepare_input_lists().unwrap();
    assert_eq!(saved.explicit_sitemap_inputs, None);
    assert_eq!(saved.sitemap_urls, ["https://example.test/imported.xml"]);
}

#[test]
fn supplied_some_empty_stays_empty_after_import_merge() {
    let mut declared = CrawlConfig {
        explicit_sitemap_inputs: Some(vec![]),
        input_imports: vec![index_import()],
        ..Default::default()
    };
    declared.capture_explicit_sitemap_inputs();
    declared.prepare_input_lists().unwrap();
    declared.capture_explicit_sitemap_inputs();
    assert_eq!(declared.explicit_sitemap_inputs, Some(vec![]));
}

#[test]
fn missing_or_null_legacy_field_stays_unknown_and_serializes_without_backfill() {
    for explicit in [None, Some(serde_json::Value::Null)] {
        let mut value = serde_json::json!({
            "seed_urls":["https://example.test/"],
            "sitemap_urls":["https://example.test/merged.xml"]
        });
        if let Some(explicit) = explicit {
            value["explicit_sitemap_inputs"] = explicit;
        }
        let mut saved: CrawlConfig = serde_json::from_value(value).unwrap();
        saved.prepare_input_lists().unwrap();
        assert_eq!(saved.explicit_sitemap_inputs, None);
        assert!(
            serde_json::to_value(&saved)
                .unwrap()
                .get("explicit_sitemap_inputs")
                .is_none()
        );
        let saved = Crawler::from_saved_config(saved).unwrap();
        assert_eq!(saved.effective_config().explicit_sitemap_inputs, None);
    }
}

#[test]
fn capture_retains_invalid_and_empty_raw_declarations_without_normalizing_them() {
    let values = vec![
        "".into(),
        " \t ".into(),
        "not a URL".into(),
        "not a URL".into(),
    ];
    let mut input = CrawlConfig {
        sitemap_urls: values.clone(),
        ..Default::default()
    };
    input.capture_explicit_sitemap_inputs();
    assert_eq!(input.explicit_sitemap_inputs, Some(values));
    assert!(Crawler::new(input).is_err());
}

#[test]
fn original_input_budget_rejects_whole_capture_instead_of_truncating_it() {
    let input = CrawlConfig {
        seed_urls: vec!["https://example.test/".into()],
        explicit_sitemap_inputs: Some(vec!["x".repeat(32 * 1024 * 1024)]),
        ..Default::default()
    };
    let error = Crawler::from_saved_config(input).err().unwrap().to_string();
    assert!(error.contains("Combined input provenance exceeds 32 MiB"));
}

#[test]
fn new_with_existing_empty_frontier_restores_legacy_none_from_metadata() {
    let saved = config();
    let (_temp, path, before) = existing_store(&saved);
    assert_eq!(
        CrawlStore::open_read_only(&path)
            .unwrap()
            .pending_count()
            .unwrap(),
        0
    );
    let crawler = Crawler::new(saved).unwrap().with_store(&path).unwrap();
    assert_eq!(crawler.effective_config().explicit_sitemap_inputs, None);
    assert_eq!(stored_text(&path), before);
}

#[test]
fn new_with_existing_store_restores_known_raw_values_without_reverse_inference() {
    let mut saved = config();
    saved.explicit_sitemap_inputs = Some(raw());
    saved.sitemap_urls = vec![
        "https://example.test/explicit.xml".into(),
        "https://example.test/imported.xml".into(),
    ];
    let (_temp, path, before) = existing_store(&saved);
    let mut candidate = saved.clone();
    candidate.explicit_sitemap_inputs = None;
    let crawler = Crawler::new(candidate).unwrap().with_store(&path).unwrap();
    assert_eq!(
        crawler.effective_config().explicit_sitemap_inputs,
        saved.explicit_sitemap_inputs
    );
    assert_eq!(stored_text(&path), before);
}

#[test]
fn preserving_constructor_retains_none_and_some_exactly_through_normalization() {
    for original in [None, Some(vec![]), Some(raw())] {
        let mut saved = config();
        saved.explicit_sitemap_inputs = original.clone();
        let crawler = Crawler::from_saved_config(saved).unwrap();
        assert_eq!(crawler.effective_config().explicit_sitemap_inputs, original);
    }
}

#[test]
fn selected_preparation_and_validation_inherit_parent_option_exactly() {
    for original in [
        None,
        Some(vec![]),
        Some(vec![
            "HTTPS://EXAMPLE.TEST/parent.xml#first".into(),
            "HTTPS://EXAMPLE.TEST/parent.xml#first".into(),
        ]),
    ] {
        let bytes = selected_snapshot(original.clone());
        let prepared = selected(&bytes);
        assert_eq!(prepared.config.explicit_sitemap_inputs, original);
        prepared.validate().unwrap();
        let crawler = Crawler::from_saved_config(prepared.config.clone())
            .unwrap()
            .with_selected_recrawl(prepared)
            .unwrap();
        assert_eq!(crawler.effective_config().explicit_sitemap_inputs, original);
    }
}

#[test]
fn existing_fresh_library_selected_chaining_restores_validated_legacy_parent_none() {
    let bytes = selected_snapshot(None);
    let prepared = selected(&bytes);
    let provisional = Crawler::new(prepared.config.clone()).unwrap();
    assert!(
        provisional
            .effective_config()
            .explicit_sitemap_inputs
            .is_some()
    );
    let inherited = provisional.with_selected_recrawl(prepared).unwrap();
    assert_eq!(inherited.effective_config().explicit_sitemap_inputs, None);
}

#[test]
fn new_store_without_config_retains_fresh_capture_despite_empty_frontier() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("new-owned.sqlite");
    let crawler = Crawler::new(config()).unwrap().with_store(&path).unwrap();
    assert_eq!(
        crawler.effective_config().explicit_sitemap_inputs,
        Some(raw())
    );
    assert!(
        CrawlStore::open_read_only(&path)
            .unwrap()
            .config()
            .unwrap()
            .is_none()
    );
}

#[test]
fn provisional_capture_budget_cannot_reject_a_valid_legacy_unknown_store_before_attachment() {
    // Current URL normalization has no product length ceiling. No request is
    // issued here. This bounded 32-MiB fixture catches the premature original-
    // provenance check without synthesizing invalid import result payloads.
    let saved = CrawlConfig {
        seed_urls: vec!["https://example.test/".into()],
        sitemap_urls: vec![format!(
            "https://example.test/{}",
            "a".repeat(32 * 1024 * 1024)
        )],
        ..Default::default()
    };
    let (_temp, path, _) = existing_store(&saved);
    let provisional = Crawler::new(saved).unwrap();
    assert!(
        provisional
            .effective_config()
            .validate_input_provenance_budget()
            .is_err()
    );
    let restored = provisional.with_store(&path).unwrap();
    assert_eq!(restored.effective_config().explicit_sitemap_inputs, None);
    restored
        .effective_config()
        .validate_input_provenance_budget()
        .unwrap();
}
