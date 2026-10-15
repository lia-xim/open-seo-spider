//! Authored SQL differential fixtures, NOT_RUN. These do not forge a raw proof:
//! internal construction is exercised after explicit fixture checks; production
//! entry remains RequestIndex::prepare(same TX, actual VerifiedRawRead).
use super::*;
use serde_json::json;
use sha2::Digest;

fn reset() {
    WORK.with(|w| w.set(Work::default()));
}
fn work() -> Work {
    WORK.with(std::cell::Cell::get)
}
fn page(url: &str) -> serde_json::Value {
    json!({"url":url,"final_url":url,"status":200,"final_status":200,"indexable":true,"indexability_reasons":[],"blocked_by_robots":false,"error":null,"truncated":false,"renderer_error":null,"redirect_chain":[],"fetched_at":"2026-10-15T00:00:00Z","rendered":false,"rendered_url":null,"rendered_status":null,"content_type":"text/html","pdf":null,"image_metadata":null,"links":[]})
}
fn fixture(members: usize, padding: usize) -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    c.execute_batch("CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT);CREATE TABLE sitemap_documents(row_id TEXT PRIMARY KEY,task_url TEXT);CREATE TABLE sitemap_locations(sequence INTEGER PRIMARY KEY,observation_id TEXT,data TEXT);").unwrap();
    let (a, b, u) = (
        "https://example.test/a",
        "https://example.test/b",
        "https://example.test/%CE%B1?q=%22x%22",
    );
    let mut first = page(a);
    first["links"] = json!([
        {"kind":"anchor","url":a,"text":"self","rel":"","internal":true,"nofollow":false},
        {"kind":"anchor","url":b,"text":"α \"one\"","rel":"nofollow","internal":true,"nofollow":true},
        {"kind":"anchor","url":b,"text":"two","rel":"","internal":false,"nofollow":false},
        {"kind":"image","url":b,"text":"excluded","rel":"","internal":true,"nofollow":false}
    ]);
    let mut second = page(b);
    second["links"] = json!([{"kind":"anchor","url":b,"text":"self b","rel":"","internal":true,"nofollow":false}]);
    second["renderer_error"] = json!("stored failure");
    let report=json!({"pages":[first,second,page(u)],"summary":{"completed":3,"pending":0},"config":{"renderer":{}},"operation":{"targets":[{"requested_url":a,"committed_generation":7},{"requested_url":a,"committed_generation":999},{"requested_url":b,"committed_generation":2}]},"sitemap_catalog":{"padding":"x".repeat(padding)}}).to_string();
    let tx = c.transaction().unwrap();
    tx.execute("INSERT INTO metadata VALUES('report',?1)", [report])
        .unwrap();
    tx.execute(
        "INSERT INTO sitemap_documents VALUES('obs-a','https://example.test/source.xml')",
        [],
    )
    .unwrap();
    {
        let mut insert = tx
            .prepare("INSERT INTO sitemap_locations VALUES(?1,'obs-a',?2)")
            .unwrap();
        for n in 0..members {
            let url = if n % 2 == 0 {
                a.to_string()
            } else {
                format!("https://example.test/member/{n:05}")
            };
            let data=json!({"row":{"observed":{"role":"page_location","normalized_fetch_url":url,"loc_ordinal":n,"validation_error":null},"admission":{"admitted_fetch_url":if n%7==0{Some(b)}else{None}}}}).to_string();
            insert
                .execute(params![i64::try_from(n).unwrap(), data])
                .unwrap();
        }
    }
    tx.commit().unwrap();
    c
}
fn counts(c: &Connection, index: bool, keys: &BTreeSet<String>) -> Vec<(String, u64, u64, u64)> {
    let sql = if index {
        "SELECT original_url,count(*),count(DISTINCT task COLLATE BINARY),count(DISTINCT observation COLLATE BINARY) FROM members WHERE original_url COLLATE BINARY IN (SELECT value FROM json_each(?1)) GROUP BY original_url ORDER BY original_url"
    } else {
        "SELECT json_extract(l.data,'$.row.observed.normalized_fetch_url'),count(*),count(DISTINCT d.task_url COLLATE BINARY),count(DISTINCT d.row_id COLLATE BINARY) FROM sitemap_locations l JOIN sitemap_documents d ON d.row_id=l.observation_id WHERE json_extract(l.data,'$.row.observed.role')='page_location' AND json_extract(l.data,'$.row.observed.validation_error') IS NULL AND json_type(l.data,'$.row.observed.loc_ordinal')='integer' AND json_extract(l.data,'$.row.observed.normalized_fetch_url') IN (SELECT value FROM json_each(?1)) GROUP BY json_extract(l.data,'$.row.observed.normalized_fetch_url') ORDER BY json_extract(l.data,'$.row.observed.normalized_fetch_url')"
    };
    let mut s = c.prepare(sql).unwrap();
    let rows = s
        .query_map([serde_json::to_string(keys).unwrap()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    rows.into_iter()
        .map(|(url, rows, tasks, observations)| {
            (
                url,
                u64::try_from(rows).unwrap(),
                u64::try_from(tasks).unwrap(),
                u64::try_from(observations).unwrap(),
            )
        })
        .collect()
}
#[test]
fn request_index_actual_sql_50000_members_and_59m_report_commits_once_and_preserves_selected_outputs()
 {
    let c = fixture(50_000, 59_626_362);
    c.execute("UPDATE metadata SET value=json_set(json_remove(value,'$.pages[2]','$.pages[1]'),'$.summary.completed',1) WHERE key='report'",[]).unwrap();
    let tx = c.unchecked_transaction().unwrap();
    let before = tx.total_changes();
    let control = SitemapReadControl::default();
    reset();
    let hash = report_hash(&tx, &control).unwrap().unwrap();
    assert!(hash.1 > 59_626_362);
    let index = prepare_with_limits(&tx, &control, Limits::default())
        .unwrap()
        .expect("This genuine-size shape must use the bounded fast path, not fallback");
    let wanted = BTreeSet::from([
        "https://example.test/a".to_string(),
        "https://example.test/b".to_string(),
        "https://example.test/%CE%B1?q=%22x%22".to_string(),
    ]);
    assert_eq!(
        index.targets(&tx, &wanted, true, 7, &control).unwrap(),
        read_target_pages_with_control(&tx, &wanted, true, 7, &control).unwrap()
    );
    assert_eq!(
        index.inlinks(&tx, &wanted, &control).unwrap(),
        read_inlinks(&tx, &wanted, &control).unwrap()
    );
    assert_eq!(
        counts(index.connection(&tx).unwrap(), true, &wanted),
        counts(&tx, false, &wanted)
    );
    let request = SitemapMembershipAnalysisRequest {
        page_limit: 2,
        ..Default::default()
    };
    assert_eq!(
        index.window(&tx, &request, true, 7, &control).unwrap(),
        read_page_window(&tx, &request, true, 7, &control).unwrap()
    );
    let actual = work();
    assert_eq!(actual.committed, 1);
    assert_eq!(actual.original_report_selects, 2);
    assert_eq!(actual.original_membership_selects, 1);
    assert_eq!(actual.original_membership_rows, 50_000);
    assert_eq!(actual.original_page_rows, 1);
    assert_eq!(
        actual.fallback_budget
            + actual.fallback_unsupported
            + actual.fallback_full
            + actual.fallback_oom,
        0
    );
    assert!(actual.committed_logical_bytes < MAX_LOGICAL_BYTES);
    assert!(actual.committed_db_pages <= MAX_DB_PAGES);
    assert_eq!(tx.total_changes(), before);
    assert_eq!(
        index.targets(&tx, &wanted, true, 7, &control).unwrap()["https://example.test/a"].age_scope,
        "selected_current_generation"
    );
    assert!(
        !index
            .targets(&tx, &wanted, true, 7, &control)
            .unwrap()
            .contains_key("https://example.test/b")
    );
}
#[test]
fn request_index_indexed_key_batches_do_not_rescan_original_memberships_or_report() {
    let c = fixture(50_000, 1024);
    let tx = c.unchecked_transaction().unwrap();
    reset();
    let index = prepare_with_limits(&tx, &Default::default(), Limits::default())
        .unwrap()
        .unwrap();
    let after_build = work();
    let private = index.connection(&tx).unwrap();
    for batch in 0..250 {
        let keys = (batch * 200..(batch + 1) * 200)
            .map(|n| {
                if n % 2 == 0 {
                    "https://example.test/a".to_string()
                } else {
                    format!("https://example.test/member/{n:05}")
                }
            })
            .collect();
        let got = counts(private, true, &keys);
        assert!(!got.is_empty());
    }
    let after = work();
    assert_eq!(after.original_membership_rows, 50_000);
    assert_eq!(after.original_membership_selects, 1);
    assert_eq!(after.original_report_selects, 2);
    assert_eq!(after, after_build);
    // Actual index definitions belong to this private database, not any mutable
    // imported schema/index trust. They are not copied from the source schema.
    let private_indexes:i64=private.query_row("SELECT count(*) FROM sqlite_master WHERE type='index' AND name IN ('pages_url','targets_url','anchors_target','members_original','members_alias')",[],|r|r.get(0)).unwrap();
    let private_indexes = usize::try_from(private_indexes).unwrap();
    assert_eq!(private_indexes, 5);
    assert_eq!(tx.total_changes(), 50_002);
}
#[test]
fn request_index_exact_logical_budget_full_page_limit_and_row_caps_are_counted_fallbacks() {
    let c = fixture(1000, 1024);
    let tx = c.unchecked_transaction().unwrap();
    let control = SitemapReadControl::default();
    reset();
    let index = prepare_with_limits(&tx, &control, Limits::default())
        .unwrap()
        .unwrap();
    let exact = work();
    drop(index);
    reset();
    let limits = Limits {
        logical_bytes: exact.committed_logical_bytes,
        rows: exact.committed_rows,
        ..Limits::default()
    };
    assert!(
        prepare_with_limits(&tx, &control, limits)
            .unwrap()
            .is_some()
    );
    reset();
    assert!(
        prepare_with_limits(
            &tx,
            &control,
            Limits {
                logical_bytes: limits.logical_bytes - 1,
                ..limits
            }
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(work().fallback_budget, 1);
    assert_eq!(work().committed, 0);
    reset();
    assert!(
        prepare_with_limits(
            &tx,
            &control,
            Limits {
                rows: limits.rows - 1,
                ..limits
            }
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(work().fallback_budget, 1);
    reset();
    assert!(
        prepare_with_limits(
            &tx,
            &control,
            Limits {
                db_pages: 16,
                ..Limits::default()
            }
        )
        .unwrap()
        .is_none()
    );
    let full = work();
    assert_eq!(full.committed, 0);
    assert_eq!(full.fallback_full + full.fallback_budget, 1);
    // Original input remains readable by the original decoder after every drop.
    let wanted = BTreeSet::from(["https://example.test/a".to_string()]);
    assert!(
        read_target_pages_with_control(&tx, &wanted, true, 7, &control)
            .unwrap()
            .contains_key("https://example.test/a")
    );
}
#[test]
fn request_index_duplicate_generation_types_sample_order_and_legacy_fallback_are_differential() {
    let c = fixture(9, 0);
    let tx = c.unchecked_transaction().unwrap();
    let control = SitemapReadControl::default();
    let wanted = BTreeSet::from([
        "https://example.test/a".to_string(),
        "https://example.test/b".to_string(),
    ]);
    let forms = [
        r#"null"#,
        r#"[]"#,
        r#"[{"requested_url":"https://example.test/a","committed_generation":7},{"requested_url":"https://example.test/a","committed_generation":1}]"#,
        r#"[{"requested_url":"https://example.test/a","committed_generation":"wrong"}]"#,
        r#""{\"requested_url\":\"https://example.test/a\"}""#,
    ];
    for form in forms {
        tx.execute("UPDATE metadata SET value=json_set(value,'$.operation.targets',json(?1)) WHERE key='report'",[form]).unwrap();
        let legacy = read_target_pages_with_control(&tx, &wanted, true, 7, &control);
        if let Some(index) = prepare_with_limits(&tx, &control, Limits::default()).unwrap() {
            let indexed = index.targets(&tx, &wanted, true, 7, &control);
            assert_eq!(indexed.is_err(), legacy.is_err());
            if let (Ok(a), Ok(b)) = (indexed, legacy) {
                assert_eq!(a, b);
            }
            assert_eq!(
                index.inlinks(&tx, &wanted, &control).unwrap(),
                read_inlinks(&tx, &wanted, &control).unwrap()
            );
        }
    }
    // The raw p.value text retains duplicate object members. SQLite's first
    // field semantics are used on both sides, with no serde reserialization.
    tx.execute("UPDATE metadata SET value=json_set(value,'$.operation.targets',json('[]')) WHERE key='report'",[]).unwrap();
    let original: String = tx
        .query_row("SELECT value FROM metadata WHERE key='report'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let replacement = original.replace(
        "\"indexable\":true",
        "\"indexable\":true,\"indexable\":false",
    );
    tx.execute(
        "UPDATE metadata SET value=?1 WHERE key='report'",
        [replacement],
    )
    .unwrap();
    let index = prepare_with_limits(&tx, &control, Limits::default())
        .unwrap()
        .unwrap();
    assert_eq!(
        index.targets(&tx, &wanted, false, 7, &control).unwrap(),
        read_target_pages_with_control(&tx, &wanted, false, 7, &control).unwrap()
    );
}
#[test]
fn request_index_tail_corruption_full_hash_utf8_and_original_cap_are_not_cached_away() {
    let c = fixture(3, 4096);
    let tx = c.unchecked_transaction().unwrap();
    let control = SitemapReadControl::default();
    let first = report_hash(&tx, &control).unwrap().unwrap();
    let index = prepare_with_limits(&tx, &control, Limits::default())
        .unwrap()
        .unwrap();
    drop(index);
    tx.execute("UPDATE metadata SET value=json_set(value,'$.sitemap_catalog.padding','changed tail') WHERE key='report'",[]).unwrap();
    assert_ne!(report_hash(&tx, &control).unwrap().unwrap().0, first.0);
    let valid: String = tx
        .query_row("SELECT value FROM metadata WHERE key='report'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let mut bytes = valid.into_bytes();
    let spot = bytes
        .windows(b"stored failure".len())
        .position(|b| b == b"stored failure")
        .unwrap();
    bytes[spot] = 0x80;
    tx.execute(
        "UPDATE metadata SET value=CAST(?1 AS TEXT) WHERE key='report'",
        [&bytes],
    )
    .unwrap();
    // If an invalid UTF8 byte affects an indexed field it is unsupported, not a
    // new rejection. Original hashing still consumes the exact original bytes.
    assert_eq!(
        report_hash(&tx, &control).unwrap().unwrap().0,
        format!("{:x}", sha2::Sha256::digest(&bytes))
    );
    reset();
    assert!(
        prepare_with_limits(&tx, &control, Limits::default())
            .unwrap()
            .is_none()
    );
    assert_eq!(work().fallback_unsupported, 1);
    let wanted = BTreeSet::from(["https://example.test/a".to_string()]);
    assert!(
        read_target_pages_with_control(&tx, &wanted, false, 7, &control)
            .unwrap()
            .contains_key("https://example.test/a")
    );
    tx.execute(
        "UPDATE metadata SET value='{\"pages\":[],\"sitemap_catalog\":{bad}}' WHERE key='report'",
        [],
    )
    .unwrap();
    let original_guard:rusqlite::Result<bool>=tx.query_row("SELECT json_valid(value) AND json_type(value,'$.pages')='array' FROM metadata WHERE key='report'",[],|r|r.get(0));
    assert!(original_guard.is_err() || !original_guard.unwrap());
    // The unchanged production prepare_context still applies MAX_REPORT_BYTES
    // before RequestIndex::prepare; the index never substitutes its own hash.
    assert_eq!(MAX_REPORT_BYTES, 268_435_456);
}
#[test]
fn request_index_cancel_and_transaction_identity_precede_optional_fallback() {
    let c = fixture(1, 0);
    let tx = c.unchecked_transaction().unwrap();
    let control = SitemapReadControl::default();
    control.cancel();
    reset();
    let error = prepare_with_limits(&tx, &control, Limits::default())
        .err()
        .expect("Cancelled");
    assert_eq!(
        error.downcast_ref::<crate::SitemapReadControlError>(),
        Some(&crate::SitemapReadControlError::Cancelled)
    );
    assert_eq!(work(), Work::default());
    let index = prepare_with_limits(&tx, &Default::default(), Limits::default())
        .unwrap()
        .unwrap();
    let other = fixture(1, 0);
    let other_tx = other.unchecked_transaction().unwrap();
    assert!(index.connection(&other_tx).is_err());
    tx.execute("UPDATE metadata SET value=value WHERE key='report'", [])
        .unwrap();
    assert!(index.connection(&tx).is_err());
    tx.rollback().unwrap();
    assert!(index.connection(&c).is_err());
}
