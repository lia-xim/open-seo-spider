// Included in the existing actual ledger tests; helper contracts are unchanged.
// AUTHORED / NOT_RUN. Genuine parser output and a real on-disk SQLite database.
fn capacity_disk_connection(path: &std::path::Path, limits: SitemapLedgerLimits) -> Connection {
    drop(crate::CrawlStore::open(path).unwrap());
    let mut connection = Connection::open(path).unwrap();
    save(&connection, "run_state", &crate::CrawlRunState {
        generation:1, started_at:"capacity-fixture".into(), finished_at:None, outcome:None,
    }).unwrap();
    let config = crate::CrawlConfig {
        sitemap_membership_limits:(limits.max_catalog_json_bytes > DEFAULT_SITEMAP_CATALOG_BYTES).then(|| limits.clone()),
        ..Default::default()
    };
    save(&connection, "config", &config).unwrap();
    let config_sha = digest(serde_json::to_vec(&config).unwrap());
    let tx = connection.transaction().unwrap();
    initialize(&tx, &config_sha, &limits, false).unwrap();
    tx.commit().unwrap();
    connection.execute("INSERT INTO discovery_frontier(kind,url) VALUES('sitemap',?1)", [TASK]).unwrap();
    connection
}

#[test]
fn complete_fifty_thousand_pairs_exceed_legacy_cap_but_commit_and_reopen_under_explicit_capacity() {
    let mut xml = String::from("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'>");
    for ordinal in 0..50_000 { xml.push_str(&format!("<url><loc>http://a/{ordinal:05}</loc></url>")); }
    xml.push_str("</urlset>");
    let observed = batch(&xml);
    assert!(observed.document.complete());
    assert_eq!(observed.entries.len(), 50_000);
    assert_eq!(observed.locations.len(), 50_000);
    let temp = tempfile::tempdir().unwrap();
    let mut legacy = capacity_disk_connection(&temp.path().join("legacy.sqlite"), SitemapLedgerLimits::default());
    let legacy_before = load_catalog(&legacy).unwrap();
    let failure = commit(&mut legacy, observed.clone()).unwrap_err();
    assert!(format!("{failure:#}").contains("budget"));
    assert_eq!(load_catalog(&legacy).unwrap(), legacy_before);
    for table in ["sitemap_documents","sitemap_entries","sitemap_locations"] { assert_eq!(count(&legacy, table), 0); }
    assert_eq!(pending_done(&legacy), 0);
    drop(legacy);

    let file = temp.path().join("explicit.sqlite");
    let explicit = SitemapLedgerLimits {max_catalog_json_bytes:128*1024*1024,..Default::default()};
    let mut connection = capacity_disk_connection(&file, explicit.clone());
    let summary = commit(&mut connection, observed).unwrap();
    assert_eq!(summary.limits, explicit);
    assert!(summary.catalog_json_bytes > DEFAULT_SITEMAP_CATALOG_BYTES as u64);
    assert!(summary.catalog_json_bytes <= explicit.max_catalog_json_bytes as u64);
    assert_eq!(summary.entry_rows, 50_000);
    assert_eq!(summary.location_rows, 50_000);
    assert_eq!(summary.document_rows, 1);
    assert_eq!(summary.incomplete_documents, 0);
    assert_eq!(summary.last_sequence, 100_001);
    assert_eq!(pending_done(&connection), 1);
    let catalog = load_catalog(&connection).unwrap().unwrap();
    assert_eq!(catalog.summary, summary);
    assert_eq!(catalog.rows.as_ref().unwrap().len(), 100_001);
    assert!(matches!(&catalog.rows.as_ref().unwrap()[0], SitemapLedgerRow::Document(d) if d.complete()));
    let first = page(&connection, 0, 1000, Some(summary.revision), Some(&summary.rolling_sha256)).unwrap();
    assert_eq!(first.rows.len(), 1000);
    assert!(first.has_more);
    let tail = page(&connection, 100_000, 1000, Some(summary.revision), Some(&summary.rolling_sha256)).unwrap();
    assert_eq!(tail.rows.len(), 1);
    assert!(!tail.has_more);
    assert!(page(&connection, 0, 1000, Some(summary.revision+1), Some(&summary.rolling_sha256)).is_err());
    drop(connection);
    let reopened = Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(load_catalog(&reopened).unwrap().unwrap(), catalog);
    assert_eq!(count(&reopened, "sitemap_entries"), 50_000);
    assert_eq!(count(&reopened, "sitemap_locations"), 50_000);
}

#[test]
fn explicit_capacity_does_not_disable_row_count_caps_or_source_atomicity() {
    let temp = tempfile::tempdir().unwrap();
    let limits = SitemapLedgerLimits {max_catalog_json_bytes:128*1024*1024,max_rows:2,..Default::default()};
    let mut connection = capacity_disk_connection(&temp.path().join("row-count.sqlite"), limits);
    let before = load_catalog(&connection).unwrap();
    assert!(commit(&mut connection, batch("<urlset><url><loc>http://a/00000</loc></url></urlset>")).is_err());
    assert_eq!(load_catalog(&connection).unwrap(), before);
    assert_eq!(count(&connection,"sitemap_documents"),0);
    assert_eq!(count(&connection,"sitemap_entries"),0);
    assert_eq!(count(&connection,"sitemap_locations"),0);
    assert_eq!(pending_done(&connection),0);

    let mut limits = SitemapLedgerLimits {max_catalog_json_bytes:128*1024*1024,..Default::default()};
    limits.max_row_json_bytes = 100;
    let mut connection = capacity_disk_connection(&temp.path().join("row-bytes.sqlite"), limits);
    let before = load_catalog(&connection).unwrap();
    assert!(commit(&mut connection, batch("<urlset><url><loc>http://a/00000</loc></url></urlset>")).is_err());
    assert_eq!(load_catalog(&connection).unwrap(), before);
    assert_eq!(pending_done(&connection),0);
}

#[test]
fn above_default_capacity_requires_declared_config_on_sql_raw_portable_and_inherited_reads() {
    let mut connection = connection(SitemapLedgerLimits::default());
    commit(&mut connection, batch("<urlset><url><loc>http://a/00000</loc></url></urlset>")).unwrap();
    let mut catalog = load_catalog(&connection).unwrap().unwrap();
    catalog.summary.limits.max_catalog_json_bytes = 128 * 1024 * 1024;
    refresh_size(&mut catalog).unwrap();
    save(&connection, SUMMARY_KEY, &catalog.summary).unwrap();
    // Self-consistent row hashes and catalog size cannot create a resource grant.
    assert!(validate_catalog(&catalog).is_ok());
    assert!(load_catalog(&connection).is_err());
    let tx = connection.transaction().unwrap();
    assert!(verified_read::verify_raw(&tx).is_err());
    tx.rollback().unwrap();
    assert!(crate::validate_sitemap_coverage_inventory(&crate::CrawlConfig::default(),Some(&catalog),None).is_err());

    let config = crate::CrawlConfig {sitemap_membership_limits:Some(catalog.summary.limits.clone()),..Default::default()};
    let mut large_outer = serde_json::to_value(&config).unwrap();
    large_outer["unrelated_config_extension"] = serde_json::json!("🚀".repeat(20_000));
    let outer_bytes = serde_json::to_vec(&large_outer).unwrap();
    assert!(outer_bytes.len() > 65536);
    save(&connection, "config", &large_outer).unwrap();
    assert_eq!(load_catalog(&connection).unwrap().unwrap(), catalog);
    let mut oversized = large_outer;
    oversized["sitemap_membership_limits"]["extra_unicode"] = serde_json::json!("🚀".repeat(4096));
    save(&connection, "config", &oversized).unwrap();
    let error = load_catalog(&connection).unwrap_err();
    assert!(format!("{error:#}").contains("Oversized sitemap membership limits declaration"));
    let tx = connection.transaction().unwrap();
    let error = match verified_read::verify_raw(&tx) { Ok(_) => panic!("Oversized declaration accepted"), Err(error) => error };
    assert!(format!("{error:#}").contains("Oversized sitemap membership limits declaration"));
    tx.rollback().unwrap();
    save(&connection, "config", &config).unwrap();
    assert_eq!(load_catalog(&connection).unwrap().unwrap(), catalog);
    let tx = connection.transaction().unwrap();
    assert!(verified_read::verify_raw(&tx).unwrap().is_some());
    tx.rollback().unwrap();
    assert!(crate::validate_sitemap_coverage_inventory(&config,Some(&catalog),None).is_ok());

    let mut child = empty_connection();
    save(&child, "config", &config).unwrap();
    let parent_sha = selected_anchor(&child, &catalog);
    let tx = child.transaction().unwrap();
    inherit(&tx, &catalog, &parent_sha).unwrap();
    tx.commit().unwrap();
    let inherited = load_catalog(&child).unwrap().unwrap();
    assert_eq!(inherited.summary.limits, catalog.summary.limits);
    assert_eq!(inherited.rows, catalog.rows);
    assert_eq!(inherited.summary.rolling_sha256, catalog.summary.rolling_sha256);
    assert!(!inherited.inherited.unwrap().current_discovery_executed);
    let mismatched = crate::CrawlConfig {sitemap_membership_limits:Some(SitemapLedgerLimits {max_catalog_json_bytes:64*1024*1024,..Default::default()}),..Default::default()};
    save(&connection, "config", &mismatched).unwrap();
    assert!(load_catalog(&connection).is_err());
    assert!(crate::validate_sitemap_coverage_inventory(&mismatched,Some(&catalog),None).is_err());
}
