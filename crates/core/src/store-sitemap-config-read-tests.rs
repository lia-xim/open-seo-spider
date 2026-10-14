//! Actual SQLite byte-bound/integrity controls for the one-read config path.
use super::*;
fn fixture() -> (tempfile::TempDir, CrawlStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = CrawlStore::open(dir.path().join("config.sqlite")).unwrap();
    let config = CrawlConfig {
        seed_urls: vec!["https://example.org/".into()],
        explicit_sitemap_inputs: Some(vec![]),
        obey_robots: false,
        ..Default::default()
    };
    store.prepare(&config).unwrap();
    let generation = store.begin_run("config-byte-control").unwrap();
    let saved = metadata_text(&store.connection.lock().unwrap(), "config")
        .unwrap()
        .unwrap();
    store
        .initialize_sitemap_discovery(
            &[],
            generation,
            &SitemapCatalogInit {
                config_sha256: sha256(&saved),
                limits: Default::default(),
                roots: vec![],
                legacy_history_unknown: false,
            },
        )
        .unwrap();
    (dir, store)
}
#[test]
fn unicode_config_uses_actual_blob_bytes_and_repeated_reads_leave_sql_unchanged() {
    let (_dir, store) = fixture();
    let connection = store.connection.lock().unwrap();
    let mut config: CrawlConfig = metadata(&connection, "config").unwrap().unwrap();
    config.user_agent = "Eigener Prüfer 🕷 — Grüße".into();
    let original = format!("\n\t{}\r\n", serde_json::to_string(&config).unwrap());
    put_metadata(&connection, "config", &original).unwrap();
    let (characters, bytes): (i64, i64) = connection
        .query_row(
            "SELECT length(value),length(CAST(value AS BLOB)) FROM metadata WHERE key='config'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(bytes > characters);
    assert_eq!(bytes as usize, original.len());
    drop(connection);
    for _ in 0..2 {
        let read = store.sitemap_read_snapshot().unwrap();
        assert_eq!(
            read.crawl_config_sha256.as_deref(),
            Some(sha256(&original).as_str())
        );
        assert_eq!(
            read.coverage.basis.unwrap().inventory_saved_config_sha256,
            read.crawl_config_sha256
        );
    }
    assert_eq!(
        metadata_text(&store.connection.lock().unwrap(), "config")
            .unwrap()
            .unwrap(),
        original
    );
}
#[test]
fn exact_saved_config_byte_ceiling_is_accepted_and_plus_one_rejected() {
    let (_dir, store) = fixture();
    let mut config: CrawlConfig = metadata(&store.connection.lock().unwrap(), "config")
        .unwrap()
        .unwrap();
    config.user_agent = "Eigener Byte-Grenzprüfer 蜘蛛 🕷".into();
    let original = serde_json::to_string(&config).unwrap();
    let maximum = 64 * 1024 * 1024;
    let mut exact = String::with_capacity(maximum);
    exact.push_str(&original);
    exact.extend(std::iter::repeat_n(' ', maximum - original.len()));
    put_metadata(&store.connection.lock().unwrap(), "config", &exact).unwrap();
    let started = std::time::Instant::now();
    let read = store.sitemap_read_snapshot().unwrap();
    let elapsed = started.elapsed();
    assert_eq!(read.crawl_config_sha256, Some(sha256(&exact)));
    eprintln!(
        "ONE_READ_CONFIG_EXACT bytes={} elapsed_ms={}",
        exact.len(),
        elapsed.as_millis()
    );
    exact.push(' ');
    put_metadata(&store.connection.lock().unwrap(), "config", &exact).unwrap();
    let error = store.sitemap_read_snapshot().unwrap_err();
    assert!(error.to_string().contains("exceeds bounded input"));
    assert_eq!(
        metadata_text(&store.connection.lock().unwrap(), "config")
            .unwrap()
            .unwrap()
            .len(),
        maximum + 1
    );
}
