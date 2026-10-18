use super::*;

#[test]
fn sqlite_memory_temporary_and_shared_uri_aliases_keep_their_original_meaning() {
    for name in [":memory:", ""] {
        let store = CrawlStore::open(name).unwrap();
        let connection = store.connection.lock().unwrap();
        let file: String = connection
            .query_row(
                "SELECT file FROM pragma_database_list WHERE name='main'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            file.is_empty(),
            "SQLite special name became a disk file: {file}"
        );
        connection
            .execute(
                "INSERT INTO metadata(key,value) VALUES('alias_witness','1')",
                [],
            )
            .unwrap();
    }
    let temp = tempfile::tempdir().unwrap();
    let name = format!(
        "file:owned-{}?mode=memory&cache=shared",
        temp.path().file_name().unwrap().to_str().unwrap()
    );
    let first = CrawlStore::open(&name).unwrap();
    first
        .connection
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO metadata(key,value) VALUES('shared_uri_witness','17')",
            [],
        )
        .unwrap();
    let second = CrawlStore::open(&name).unwrap();
    assert_eq!(
        second
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT value FROM metadata WHERE key='shared_uri_witness'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        "17"
    );
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    let error = CrawlStore::open("file:owned-refused?vfs=owned-nonexistent-vfs")
        .err()
        .unwrap();
    assert!(
        error
            .to_string()
            .contains("open writable SQLite connection")
    );
    assert!(error.to_string().contains("no such vfs"));
    assert!(error.downcast_ref::<rusqlite::Error>().is_some());
}

#[test]
fn readonly_file_uri_keeps_original_flags_mode_and_database_identity() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("uri database.sqlite");
    let writer = CrawlStore::open(&path).unwrap();
    writer
        .connection
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO metadata(key,value) VALUES('uri_witness','23')",
            [],
        )
        .unwrap();
    drop(writer);
    let mut uri = url::Url::from_file_path(&path).unwrap();
    uri.query_pairs_mut().append_pair("mode", "ro");
    let original = Connection::open_with_flags(uri.as_str(), OpenFlags::SQLITE_OPEN_READ_ONLY);
    let adapted = CrawlStore::open_read_only(uri.as_str());
    match (original, adapted) {
        (Ok(original), Ok(adapted)) => {
            println!("SQLITE_READONLY_URI_BASELINE accepted_with_original_readonly_flags");
            assert_eq!(
                original.is_readonly(rusqlite::MAIN_DB).unwrap(),
                adapted
                    .connection
                    .lock()
                    .unwrap()
                    .is_readonly(rusqlite::MAIN_DB)
                    .unwrap()
            );
            assert_eq!(
                adapted
                    .connection
                    .lock()
                    .unwrap()
                    .query_row(
                        "SELECT value FROM metadata WHERE key='uri_witness'",
                        [],
                        |row| row.get::<_, String>(0)
                    )
                    .unwrap(),
                "23"
            );
        }
        (Err(original), Err(adapted)) => {
            // URI interpretation without SQLITE_OPEN_URI depends on SQLite's
            // existing build/global setting; this adapter must not broaden it.
            println!(
                "SQLITE_READONLY_URI_BASELINE refused_with_original_readonly_flags {original:?}"
            );
            assert_eq!(
                format!("{:?}", adapted.downcast_ref::<rusqlite::Error>().unwrap()),
                format!("{original:?}")
            );
        }
        (Ok(_), Err(error)) => panic!("Adapter rejected an existing read-only URI: {error:#}"),
        (Err(error), Ok(_)) => {
            panic!("Adapter changed a rejected read-only URI into success: {error}")
        }
    }
    let mut missing = url::Url::from_file_path(temp.path().join("missing.sqlite")).unwrap();
    missing.query_pairs_mut().append_pair("mode", "ro");
    assert!(CrawlStore::open_read_only(missing.as_str()).is_err());
    assert!(!temp.path().join("missing.sqlite").exists());
}

#[test]
fn opening_errors_keep_stage_message_and_original_sqlite_error_chain() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("no-parent").join("missing.sqlite");
    let open = CrawlStore::open_read_only(&missing).err().unwrap();
    assert!(
        open.to_string()
            .contains("open read-only SQLite connection")
    );
    assert!(open.to_string().contains("unable to open database file"));
    assert!(open.to_string().contains("SQLite CannotOpen/"));
    assert!(open.downcast_ref::<rusqlite::Error>().is_some());
    assert!(!missing.parent().unwrap().exists());

    let invalid = temp.path().join("not-a-database.sqlite");
    std::fs::write(&invalid, [b'x'; 4096]).unwrap();
    // Opening a handle alone succeeds; the actual product WAL/schema batch is
    // where this independently malformed database is rejected.
    drop(Connection::open(&invalid).unwrap());
    let schema = CrawlStore::open(&invalid).err().unwrap();
    assert!(
        schema
            .to_string()
            .contains("initialize SQLite WAL and core schema")
    );
    assert!(schema.to_string().contains("file is not a database"));
    assert!(schema.to_string().contains("SQLite NotADatabase/"));
    assert!(schema.downcast_ref::<rusqlite::Error>().is_some());
    assert_eq!(std::fs::read(&invalid).unwrap(), [b'x'; 4096]);
}
