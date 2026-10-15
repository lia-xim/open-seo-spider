use super::*;
use serde_json::json;
use sha2::Digest;

fn database(report: &str) -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch("CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT)").unwrap();
    connection.execute("INSERT INTO metadata(key,value) VALUES('report',?1)", [report]).unwrap();
    connection
}
fn reset_work() { WORK.with(|cell| cell.set(Work::default())); }
fn work() -> Work { WORK.with(std::cell::Cell::get) }
fn large_report() -> String {
    json!({"pages":[{"url":"https://example.test/one","links":[{"kind":"anchor","url":"https://example.test/two","text":"quoted \"α\""}]}],"summary":{"completed":1,"pending":0},"operation":{"targets":[{"requested_url":"https://example.test/one","committed_generation":7}]},"sitemap_catalog":{"rows":[{"padding":"x".repeat(4*1024*1024)}]},"sitemap_source_measurements":{"unchanged":true},"config":{"renderer":null}}).to_string()
}
#[test]
fn report_projection_real_large_sql_report_one_page_preserves_guards_and_actual_hash_work() {
    let report = large_report();
    let connection = database(&report);
    let tx = connection.unchecked_transaction().unwrap();
    let control = SitemapReadControl::default();
    reset_work();
    let original_hash = super::super::report_hash(&tx, &control).unwrap().unwrap();
    assert_eq!(original_hash.1, report.len() as u64);
    assert_eq!(original_hash.0, format!("{:x}", sha2::Sha256::digest(report.as_bytes())));
    let compact = compact_report(&tx, &control).unwrap().unwrap();
    assert!(report.len() > 4*1024*1024 && compact.len() < 1024);
    assert_eq!(identity_counts(&tx, None).unwrap(), (1, 0, 1));
    assert_eq!(identity_counts(&tx, Some(&compact)).unwrap(), identity_counts(&tx, None).unwrap());
    assert_eq!(selected(&tx, Some(&compact)).unwrap(), selected(&tx, None).unwrap());
    let before: serde_json::Value = serde_json::from_str(&report).unwrap();
    let after: serde_json::Value = serde_json::from_str(&compact).unwrap();
    let mut expected = before;expected.as_object_mut().unwrap().remove("sitemap_catalog");
    assert_eq!(after, expected);
    let actual = work();
    assert_eq!(actual.original_report_hash_passes,1);
    assert_eq!(actual.original_report_hashed_bytes,report.len());
    assert_eq!(actual.projection_selects,1);
    assert_eq!(actual.accepted_projections,1);
    assert_eq!(actual.retained_projection_bytes,compact.len());
    assert_eq!((actual.original_guard_queries,actual.compact_guard_queries),(3,2));
    assert_eq!(tx.total_changes(),1);
}
#[test]
fn report_projection_exact_optional_budget_falls_back_without_new_input_rejection() {
    let connection = database(&large_report());
    let tx = connection.unchecked_transaction().unwrap();let control = SitemapReadControl::default();
    let actual = compact_report(&tx,&control).unwrap().unwrap();
    assert_eq!(compact_with_limit(&tx,&control,actual.len()).unwrap().as_deref(),Some(actual.as_str()));
    assert!(compact_with_limit(&tx,&control,actual.len()-1).unwrap().is_none());
    assert_eq!(identity_counts(&tx,None).unwrap(),(1,0,1));
    let too_large = json!({"pages":[{"url":"https://example.test/one","links":[]}],"operation":null,"retained_unrelated_field":"y".repeat(COMPACT_REPORT_BYTES),"sitemap_catalog":{"rows":[]}}).to_string();
    tx.execute("UPDATE metadata SET value=?1 WHERE key='report'",[too_large]).unwrap();
    assert!(compact_report(&tx,&control).unwrap().is_none());
    assert_eq!(identity_counts(&tx,None).unwrap(),(1,0,0));assert!(!selected(&tx,None).unwrap());
}
#[test]
fn report_projection_old_forms_and_duplicate_unicode_fields_keep_original_sql_semantics() {
    let forms = [
        r#"{"pages":[],"operation":null}"#,
        r#"{"pages":[],"operation":true,"sitemap_catalog":null}"#,
        r#"{"pages":[],"operation":"{\"targets\":[]}","sitemap_catalog":[]}"#,
        r#"{"pages":[{"url":"https://example.test/α","links":[]},{"url":"https://example.test/α","links":[]}],"operation":"{\"targets\":[]}","sitemap_catalog":{"rows":[]}}"#,
        r#"{"pages":[{"url":"https://example.test/one","links":null}],"operation":{},"sitemap_catalog":{"rows":[]}}"#,
        r#"{"pages":[{"url":"first","url":"second","links":[]}],"operation":null,"operation":{},"sitemap_catalog":{"rows":[]}}"#,
    ];
    for (index,report) in forms.into_iter().enumerate() {
        let connection=database(report);let tx=connection.unchecked_transaction().unwrap();
        let compact=compact_report(&tx,&Default::default()).unwrap();
        if index<3 {assert!(compact.is_none());}
        assert_eq!(identity_counts(&tx,compact.as_deref()).unwrap(),identity_counts(&tx,None).unwrap());
        assert_eq!(selected(&tx,compact.as_deref()).unwrap(),selected(&tx,None).unwrap());
    }
    let connection=database(&large_report());assert!(compact_report(&connection,&Default::default()).unwrap().is_none());
    let tx=connection.unchecked_transaction().unwrap();
    tx.execute("UPDATE metadata SET value=CAST(value AS BLOB) WHERE key='report'",[]).unwrap();
    assert!(compact_report(&tx,&Default::default()).unwrap().is_none());
    assert_eq!(identity_counts(&tx,None).unwrap(),(1,0,1));
}
#[test]
fn report_projection_original_hash_changes_in_removed_tail_and_invalid_json_still_refuses() {
    let report=large_report();let connection=database(&report);let tx=connection.unchecked_transaction().unwrap();let control=SitemapReadControl::default();
    let first_hash=super::super::report_hash(&tx,&control).unwrap().unwrap();let first_projection=compact_report(&tx,&control).unwrap().unwrap();
    let mut changed:serde_json::Value=serde_json::from_str(&report).unwrap();changed["sitemap_catalog"]["rows"][0]["padding"]=json!("z".repeat(4*1024*1024));
    tx.execute("UPDATE metadata SET value=?1 WHERE key='report'",[changed.to_string()]).unwrap();
    assert_ne!(super::super::report_hash(&tx,&control).unwrap().unwrap().0,first_hash.0);
    assert_eq!(compact_report(&tx,&control).unwrap().unwrap(),first_projection);
    // The old original JSON guard is retained in prepare_context before the new
    // optimization. Invalidity in the removed field remains a real refusal.
    tx.execute("UPDATE metadata SET value=?1 WHERE key='report'",[r#"{"pages":[],"sitemap_catalog":{"bad":invalid}}"#]).unwrap();
    let old_guard:rusqlite::Result<(bool,Option<String>)>=tx.query_row("SELECT json_valid(value),json_type(value,'$.pages') FROM metadata WHERE key='report'",[],|row|Ok((row.get(0)?,row.get(1)?)));
    assert!(old_guard.is_err());
}
#[test]
fn report_projection_cooperative_cancel_preserves_stop_type_and_does_not_return_optimization() {
    let connection=database(&large_report());let tx=connection.unchecked_transaction().unwrap();let control=SitemapReadControl::default();control.cancel();reset_work();
    let error=compact_report(&tx,&control).unwrap_err();
    assert_eq!(error.downcast_ref::<crate::SitemapReadControlError>(),Some(&crate::SitemapReadControlError::Cancelled));
    assert_eq!(work(),Work::default());
}

#[test]
fn report_projection_unsupported_utf8_text_uses_old_queries_without_new_refusal() {
    // This is actual SQL TEXT with an uninterpreted invalid UTF8 byte, not a
    // replacement Rust string. Historical hashing uses its original bytes;
    // historical JSON guards inspect only their existing selected fields.
    let mut report = br#"{"pages":[{"url":"https://example.test/one","links":[]}],"summary":{"completed":1,"pending":0},"operation":{"targets":[]},"unrelated":""#.to_vec();
    report.push(0x80);
    report.extend_from_slice(br#"","sitemap_catalog":{"rows":[]}}"#);
    assert!(std::str::from_utf8(&report).is_err());
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch("CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT)").unwrap();
    connection.execute("INSERT INTO metadata(key,value) VALUES('report',CAST(?1 AS TEXT))", [&report]).unwrap();
    let tx = connection.unchecked_transaction().unwrap();
    let control = SitemapReadControl::default();
    let (kind, valid, shape, pages, completed, pending): (String, bool, String, i64, i64, i64) = tx.query_row(
        "SELECT typeof(value),json_valid(value),json_type(value,'$.pages'),json_array_length(value,'$.pages'),json_extract(value,'$.summary.completed'),json_extract(value,'$.summary.pending') FROM metadata WHERE key='report'",
        [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?)),
    ).unwrap();
    assert_eq!((kind.as_str(),valid,shape.as_str(),pages,completed,pending),("text",true,"array",1,1,0));
    let mut statement = tx.prepare("SELECT value FROM metadata WHERE key='report'").unwrap();
    let mut rows = statement.query([]).unwrap();
    let row = rows.next().unwrap().unwrap();
    let value = row.get_ref(0).unwrap();
    assert!(matches!(value,rusqlite::types::ValueRef::Text(_)));
    assert_eq!(value.as_bytes().unwrap(),report.as_slice());
    assert!(value.as_str().is_err());
    drop(rows);
    drop(statement);
    reset_work();
    let original_hash = super::super::report_hash(&tx,&control).unwrap().unwrap();
    assert_eq!(original_hash.0,format!("{:x}",sha2::Sha256::digest(&report)));
    assert_eq!(original_hash.1,report.len() as u64);
    // json_remove must not turn an unsupported borrowed UTF8 representation
    // into a new refusal. The unchanged original SQL remains authoritative.
    assert!(compact_report(&tx,&control).unwrap().is_none());
    assert_eq!(identity_counts(&tx,None).unwrap(),(1,0,0));
    assert!(selected(&tx,None).unwrap());
    let actual=work();
    assert_eq!(actual.original_report_hash_passes,1);
    assert_eq!(actual.original_report_hashed_bytes,report.len());
    assert_eq!(actual.projection_selects,1);
    assert_eq!(actual.accepted_projections,0);
    assert_eq!(actual.retained_projection_bytes,0);
    assert_eq!((actual.original_guard_queries,actual.compact_guard_queries),(2,0));
    assert_eq!(tx.total_changes(),1);
}
