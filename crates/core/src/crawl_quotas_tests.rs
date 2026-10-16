//! AUTHORED_NOT_RUN. Typed contracts and actual durable reservation transactions.
use super::*;
use crate::crawl_quota_ledger::Admission;
use crate::store::QueueEntry;
use crate::{CrawlConfig, CrawlStore};

fn config() -> CrawlConfig {
    CrawlConfig {
        seed_urls: vec!["http://example.test/".into()],
        crawl_quotas: Some(CrawlQuotaConfig {
            version: 1,
            max_urls_per_depth: Some(2),
            max_urls_per_subdomain: Some(3),
            url_pattern_quotas: vec![CrawlUrlPatternQuota {
                id: "p".into(),
                pattern: "/p/".into(),
                max_urls: 0,
            }],
        }),
        discover_sitemaps: false,
        obey_robots: false,
        ..Default::default()
    }
}

#[test]
fn quota_types_reject_unknown_duplicate_bad_version_ranges_and_regex() {
    for s in [
        r#"{"version":2,"max_urls_per_depth":1}"#,
        r#"{"version":1}"#,
        r#"{"version":1,"max_urls_per_depth":-1}"#,
        r#"{"version":1,"max_urls_per_depth":1.5}"#,
        r#"{"version":1,"max_urls_per_depth":"1"}"#,
        r#"{"version":1,"max_urls_per_depth":1000000001}"#,
        r#"{"version":1,"max_urls_per_depth":1,"extra":1}"#,
        r#"{"version":1,"max_urls_per_depth":1,"max_urls_per_depth":2}"#,
        r#"{"version":1,"url_pattern_quotas":[{"id":"a","pattern":"(","max_urls":1}]}"#,
    ] {
        let parsed = serde_json::from_str::<CrawlQuotaConfig>(s);
        assert!(
            parsed.is_err() || parsed.unwrap().validate().is_err(),
            "{s}"
        );
    }
    let mut c = config().crawl_quotas.unwrap();
    c.max_urls_per_depth = Some(0);
    c.validate().unwrap();
    c.url_pattern_quotas.push(c.url_pattern_quotas[0].clone());
    assert!(c.validate().is_err());
    let legacy = serde_json::to_value(CrawlConfig::default()).unwrap();
    assert!(legacy.get("crawl_quotas").is_none());
    let value: CrawlConfig =
        serde_json::from_value(serde_json::json!({"crawl_quotas":null})).unwrap();
    assert!(value.crawl_quotas.is_none());
}

#[test]
fn quota_memberships_all_match_effective_full_url_and_separate_port_from_host_key() {
    let c = CrawlQuotaConfig {
        version: 1,
        max_urls_per_depth: Some(1),
        max_urls_per_subdomain: Some(3),
        url_pattern_quotas: vec![
            CrawlUrlPatternQuota {
                id: "folder".into(),
                pattern: "/p/".into(),
                max_urls: 2,
            },
            CrawlUrlPatternQuota {
                id: "query".into(),
                pattern: "[?]a=1$".into(),
                max_urls: 1,
            },
        ],
    }
    .compile()
    .unwrap();
    let a = crate::normalize_url("http://EXAMPLE.test:123/p/item?a=1#x", None).unwrap();
    let b = crate::normalize_url("https://example.test:456/p/item?a=1", None).unwrap();
    let am = c.memberships(&a, 4).unwrap();
    let bm = c.memberships(&b, 4).unwrap();
    assert_eq!(am, bm);
    assert_ne!(a, b);
    assert_eq!(am.len(), 4);
    assert_eq!(am[1].1, "example.test");
    assert_eq!(am[2].1, "folder");
    assert_eq!(am[3].1, "query");
}

#[test]
fn quota_real_store_atomic_denial_does_not_spend_unrelated_dimensions_or_create_page() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let store = CrawlStore::open(&p).unwrap();
    let cfg = config();
    store.prepare(&cfg).unwrap();
    let g = store.begin_run("2026-10-16T00:00:00Z").unwrap();
    let compiled = cfg.crawl_quotas.as_ref().unwrap().compile().unwrap();
    for path in ["/p/1", "/free/1", "/free/2"] {
        let target = Url::parse(&format!("http://example.test{path}")).unwrap();
        store
            .enqueue(&QueueEntry {
                url: target.to_string(),
                depth: 1,
                discovered_from: None,
                admission: None,
            })
            .unwrap();
        let result = store
            .reserve_crawl_quota_target(&compiled, g, target.as_str(), &target, 1)
            .unwrap();
        if path == "/p/1" {
            assert!(matches!(result, Admission::Deferred));
        } else {
            let Admission::Reserved(id) = result else {
                panic!("positive free target denied")
            };
            store.start_crawl_quota_attempt(g, id).unwrap();
            store.finish_crawl_quota_attempts(g, &[id], false).unwrap();
        }
    }
    let s = store.crawl_quota_summary().unwrap().unwrap();
    assert_eq!(s.reserved_distinct_targets, 2);
    assert_eq!(s.deferred_targets, 1);
    assert_eq!(s.actual_started_attempts, 2);
    assert_eq!(s.settled_attempts, 2);
    assert_eq!(s.active_attempts, 0);
    assert_eq!(store.pages().unwrap().len(), 0);
    assert!(s.bucket_rows.iter().all(|b| b.used == 2));
    assert!(s.causes.contains(&"pattern:p".to_owned()));
    drop(store);
    let read = CrawlStore::open_read_only(&p).unwrap();
    assert_eq!(read.crawl_quota_summary().unwrap(), Some(s));
}

#[test]
fn quota_reservation_sql_failure_rolls_back_every_membership_and_prevents_start() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let store = CrawlStore::open(&p).unwrap();
    let mut cfg = config();
    cfg.crawl_quotas
        .as_mut()
        .unwrap()
        .url_pattern_quotas
        .clear();
    store.prepare(&cfg).unwrap();
    let g = store.begin_run("2026-10-16T00:00:00Z").unwrap();
    let url = Url::parse("http://example.test/free").unwrap();
    store
        .enqueue(&QueueEntry {
            url: url.to_string(),
            depth: 0,
            discovered_from: None,
            admission: None,
        })
        .unwrap();
    // Negative-only failure injection in this owned test store, not positive state fabrication.
    let conn = rusqlite::Connection::open(&p).unwrap();
    conn.execute_batch("CREATE TRIGGER deny_quota_member BEFORE INSERT ON crawl_quota_members BEGIN SELECT RAISE(ABORT,'owned negative failure'); END;").unwrap();
    drop(conn);
    let compiled = cfg.crawl_quotas.as_ref().unwrap().compile().unwrap();
    assert!(
        store
            .reserve_crawl_quota_target(&compiled, g, url.as_str(), &url, 0)
            .is_err()
    );
    let s = store.crawl_quota_summary().unwrap().unwrap();
    assert_eq!(s.reserved_distinct_targets, 0);
    assert_eq!(s.actual_started_attempts, 0);
    assert!(s.bucket_rows.is_empty());
    assert_eq!(store.pending_count().unwrap(), 1);
}

#[test]
fn quota_cold_enabled_missing_ledger_is_not_silently_reinitialized() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let store = CrawlStore::open(&p).unwrap();
    let cfg = config();
    store.prepare(&cfg).unwrap();
    store.begin_run("2026-10-16T00:00:00Z").unwrap();
    drop(store);
    let conn = rusqlite::Connection::open(&p).unwrap();
    conn.execute_batch("DROP TABLE crawl_quota_buckets")
        .unwrap();
    drop(conn);
    assert!(
        CrawlStore::open_read_only(&p)
            .unwrap()
            .crawl_quota_summary()
            .is_err()
    );
    assert!(CrawlStore::inspect(&p).is_err());
}

#[test]
fn quota_owned_future_drop_keeps_capacity_unknown_and_stale_generation_cannot_send() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let store = std::sync::Arc::new(CrawlStore::open(&p).unwrap());
    let mut cfg = config();
    cfg.crawl_quotas
        .as_mut()
        .unwrap()
        .url_pattern_quotas
        .clear();
    store.prepare(&cfg).unwrap();
    let g = store.begin_run("2026-10-16T00:00:00Z").unwrap();
    let url = Url::parse("http://example.test/hold").unwrap();
    store
        .enqueue(&QueueEntry {
            url: url.to_string(),
            depth: 0,
            discovered_from: None,
            admission: None,
        })
        .unwrap();
    let compiled = std::sync::Arc::new(cfg.crawl_quotas.as_ref().unwrap().compile().unwrap());
    let mut owner = crate::crawl_quota_runtime::QuotaFetchSession::new(
        store.clone(),
        compiled.clone(),
        g,
        url.to_string(),
        0,
    );
    let id = owner.reserve(&url).unwrap().unwrap();
    owner.start(id).unwrap();
    drop(owner);
    let s = store.crawl_quota_summary().unwrap().unwrap();
    assert_eq!(s.reserved_distinct_targets, 1);
    assert_eq!(s.interrupted_or_unknown_attempts, 1);
    assert_eq!(s.active_attempts, 0);
    let next = store.begin_run("2026-10-16T00:00:01Z").unwrap();
    assert!(store.start_crawl_quota_attempt(g, id).is_err());
    let Admission::Reserved(retry) = store
        .reserve_crawl_quota_target(&compiled, next, url.as_str(), &url, 0)
        .unwrap()
    else {
        panic!("Same target slot was lost")
    };
    store.start_crawl_quota_attempt(next, retry).unwrap();
    store
        .finish_crawl_quota_attempts(next, &[retry], false)
        .unwrap();
    let after = store.crawl_quota_summary().unwrap().unwrap();
    assert_eq!(after.reserved_distinct_targets, 1);
    assert_eq!(after.actual_started_attempts, 2);
    assert!(after.bucket_rows.iter().all(|b| b.used == 1));
}

#[test]
fn quota_all_matching_rules_claim_together_and_later_overlap_cannot_spend_host_slot() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("crawl.sqlite");
    let store = CrawlStore::open(&p).unwrap();
    let mut cfg = config();
    cfg.crawl_quotas = Some(CrawlQuotaConfig {
        version: 1,
        max_urls_per_depth: None,
        max_urls_per_subdomain: Some(4),
        url_pattern_quotas: vec![
            CrawlUrlPatternQuota {
                id: "p".into(),
                pattern: "/p/".into(),
                max_urls: 2,
            },
            CrawlUrlPatternQuota {
                id: "tail".into(),
                pattern: "(?:/p/2|/q/1)$".into(),
                max_urls: 1,
            },
        ],
    });
    store.prepare(&cfg).unwrap();
    let g = store.begin_run("2026-10-16T00:00:00Z").unwrap();
    let compiled = cfg.crawl_quotas.as_ref().unwrap().compile().unwrap();
    for path in ["/p/1", "/p/2", "/q/1"] {
        let url = Url::parse(&format!("http://example.test{path}")).unwrap();
        store
            .enqueue(&QueueEntry {
                url: url.to_string(),
                depth: 0,
                discovered_from: None,
                admission: None,
            })
            .unwrap();
        let result = store
            .reserve_crawl_quota_target(&compiled, g, url.as_str(), &url, 0)
            .unwrap();
        if path == "/q/1" {
            assert!(matches!(result, Admission::Deferred));
        } else {
            let Admission::Reserved(id) = result else {
                panic!("Expected genuine reservation")
            };
            store.finish_crawl_quota_attempts(g, &[id], false).unwrap();
        }
    }
    let s = store.crawl_quota_summary().unwrap().unwrap();
    assert_eq!(s.reserved_distinct_targets, 2);
    assert_eq!(
        s.bucket_rows
            .iter()
            .find(|b| b.dimension == "host")
            .unwrap()
            .used,
        2
    );
    assert_eq!(s.bucket_rows.iter().find(|b| b.key == "p").unwrap().used, 2);
    assert_eq!(
        s.bucket_rows.iter().find(|b| b.key == "tail").unwrap().used,
        1
    );
    assert_eq!(s.deferred_targets, 1);
}

#[test]
fn quota_interrupted_nocase_membership_schema_refuses_recovery_and_reserve_without_spending() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("interrupted-nocase.sqlite");
    let mut config = config();
    config.crawl_quotas = Some(CrawlQuotaConfig {
        version: 1,
        max_urls_per_depth: None,
        max_urls_per_subdomain: Some(1),
        url_pattern_quotas: vec![],
    });
    let upper = Url::parse("http://example.test/Case").unwrap();
    let lower = Url::parse("http://example.test/case").unwrap();
    assert_ne!(upper, lower);
    let compiled = config.crawl_quotas.as_ref().unwrap().compile().unwrap();
    let store = CrawlStore::open(&database).unwrap();
    store.prepare(&config).unwrap();
    let generation = store.begin_run("2026-10-16T00:00:00Z").unwrap();
    for target in [&upper, &lower] {
        store
            .enqueue(&QueueEntry {
                url: target.to_string(),
                depth: 0,
                discovered_from: None,
                admission: None,
            })
            .unwrap();
    }
    let Admission::Reserved(reservation) = store
        .reserve_crawl_quota_target(&compiled, generation, upper.as_str(), &upper, 0)
        .unwrap()
    else {
        panic!("The genuine initial Case target must own its one membership");
    };
    assert!(reservation > 0);
    assert!(matches!(
        store
            .reserve_crawl_quota_target(&compiled, generation, lower.as_str(), &lower, 0)
            .unwrap(),
        Admission::Deferred
    ));
    let real = store.crawl_quota_summary().unwrap().unwrap();
    assert_eq!(real.reserved_distinct_targets, 1);
    assert_eq!(real.actual_started_attempts, 0);
    assert_eq!(real.active_attempts, 1);
    assert_eq!(real.deferred_targets, 1);
    assert_eq!(store.pending_count().unwrap(), 2);
    assert!(store.checkpoint().unwrap().is_none());
    drop(store); // Deliberately nonterminal; no terminal hash is available.
    // Negative-only schema mutation copies only the genuine existing member.
    // No Page, attempt, bucket or frontier success observation is invented.
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection.execute_batch(
        "BEGIN IMMEDIATE;
         ALTER TABLE crawl_quota_members RENAME TO crawl_quota_members_original;
         CREATE TABLE crawl_quota_members(kind TEXT NOT NULL,bucket TEXT NOT NULL,target TEXT COLLATE NOCASE NOT NULL,PRIMARY KEY(kind,bucket,target));
         INSERT INTO crawl_quota_members(kind,bucket,target) SELECT kind,bucket,target FROM crawl_quota_members_original;
         DROP TABLE crawl_quota_members_original;
         COMMIT;"
    ).unwrap();
    let saved_config: String = connection
        .query_row("SELECT value FROM metadata WHERE key='config'", [], |row| {
            row.get(0)
        })
        .unwrap();
    let saved_run: String = connection
        .query_row(
            "SELECT value FROM metadata WHERE key='run_state'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let unchanged: (u64, u64) = connection.query_row(
        "SELECT (SELECT used FROM crawl_quota_buckets WHERE kind='host' AND bucket='example.test'),(SELECT COUNT(*) FROM crawl_quota_members)",
        [], |row| Ok((crate::crawl_quota_ledger::sql_u64(row,0)?, crate::crawl_quota_ledger::sql_u64(row,1)?)),
    ).unwrap();
    assert_eq!(unchanged, (1, 1));
    drop(connection);
    fn schema_refusal<T>(result: anyhow::Result<T>) {
        match result {
            Err(error) => assert!(
                error
                    .to_string()
                    .contains("canonical quota table crawl_quota_members"),
                "Wrong refusal: {error:#}"
            ),
            Ok(_) => {
                panic!("NOCASE target identity must not pass canonical quota schema admission")
            }
        }
    }
    schema_refusal(
        CrawlStore::open_read_only(&database)
            .unwrap()
            .crawl_quota_summary(),
    );
    schema_refusal(CrawlStore::inspect(&database));
    let recovery = CrawlStore::open(&database).unwrap();
    schema_refusal(recovery.prepare(&config));
    schema_refusal(recovery.begin_run("2026-10-16T00:00:01Z"));
    schema_refusal(recovery.reserve_crawl_quota_target(
        &compiled,
        generation,
        lower.as_str(),
        &lower,
        0,
    ));
    drop(recovery);
    let connection = rusqlite::Connection::open_with_flags(
        &database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let actual_schema: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='crawl_quota_members'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(actual_schema.contains("target TEXT COLLATE NOCASE NOT NULL"));
    assert_eq!(
        connection
            .query_row("SELECT value FROM metadata WHERE key='config'", [], |row| {
                row.get::<_, String>(0)
            })
            .unwrap(),
        saved_config
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT value FROM metadata WHERE key='run_state'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        saved_run
    );
    let facts: (u64,u64,u64,u64,u64,u64,String,String) = connection.query_row(
        "SELECT (SELECT used FROM crawl_quota_buckets WHERE kind='host' AND bucket='example.test'),(SELECT COUNT(*) FROM crawl_quota_members),(SELECT COUNT(*) FROM crawl_quota_attempts),(SELECT COUNT(*) FROM pages),(SELECT COUNT(*) FROM frontier WHERE done=0),(SELECT COUNT(*) FROM metadata WHERE key='checkpoint'),(SELECT target FROM crawl_quota_members),(SELECT state FROM crawl_quota_attempts)",
        [], |row| Ok((crate::crawl_quota_ledger::sql_u64(row,0)?,crate::crawl_quota_ledger::sql_u64(row,1)?,crate::crawl_quota_ledger::sql_u64(row,2)?,crate::crawl_quota_ledger::sql_u64(row,3)?,crate::crawl_quota_ledger::sql_u64(row,4)?,crate::crawl_quota_ledger::sql_u64(row,5)?,row.get(6)?,row.get(7)?)),
    ).unwrap();
    assert_eq!(
        facts,
        (1, 1, 1, 0, 2, 0, upper.to_string(), "reserved".into())
    );
}

#[test]
fn quota_sql_unsigned_counters_reject_negative_null_float_and_text_without_wrapping() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    for (sql, expected) in [
        ("SELECT 0", 0),
        ("SELECT 1", 1),
        (
            "SELECT 9223372036854775807",
            u64::try_from(i64::MAX).unwrap(),
        ),
    ] {
        assert_eq!(
            connection
                .query_row(sql, [], |row| crate::crawl_quota_ledger::sql_u64(row, 0))
                .unwrap(),
            expected
        );
    }
    for sql in ["SELECT -1", "SELECT NULL", "SELECT 1.5", "SELECT '1'"] {
        assert!(
            connection
                .query_row(sql, [], |row| crate::crawl_quota_ledger::sql_u64(row, 0))
                .is_err(),
            "Invalid persisted integer was accepted: {sql}"
        );
    }
}
