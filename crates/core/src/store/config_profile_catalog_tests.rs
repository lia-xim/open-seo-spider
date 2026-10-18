use super::*;
use std::sync::{Arc, Barrier};

const ID: &str = "00000000-0000-4000-8000-000000000001";
const OTHER: &str = "00000000-0000-4000-8000-000000000002";
fn text() -> &'static str {
    r#"{"version":1,"config":{"delay_ms":9007199254740993,"custom_search":null},"required_headers":["Authorization","authorization"]}"#
}
fn path(root: &Path) -> PathBuf {
    root.join("configuration-profiles").join("catalog.sqlite")
}
fn kind<T: fmt::Debug>(result: Result<T>, expected: ConfigProfileCatalogErrorKind) {
    assert_eq!(result.unwrap_err().kind, expected);
}
fn raw(path: &Path) -> Connection {
    open_connection(path, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap()
}

#[test]
fn catalog_absent_reads_and_invalid_creates_have_no_filesystem_effects() {
    let root = tempfile::tempdir().unwrap();
    let db = path(root.path());
    let store = ConfigProfileCatalog::new(&db);
    assert!(store.list().unwrap().is_empty());
    kind(store.get(ID), ConfigProfileCatalogErrorKind::Missing);
    kind(
        store.rename(ID, "1", &"0".repeat(64), "name"),
        ConfigProfileCatalogErrorKind::Missing,
    );
    kind(
        store.delete(ID, "1", &"0".repeat(64)),
        ConfigProfileCatalogErrorKind::Missing,
    );
    for (id, name, profile) in [
        ("../alias", "valid", text()),
        (ID, " \t ", text()),
        (
            ID,
            "valid",
            r#"{"version":1,"config":{"custom_headers":{"Authorization":"sentinel"}}}"#,
        ),
        (ID, "valid", r#"{"version":1,"config":{},"config":{}}"#),
    ] {
        kind(
            store.create(id, name, profile),
            ConfigProfileCatalogErrorKind::Validation,
        );
    }
    assert!(!db.parent().unwrap().exists());
}

#[test]
fn catalog_cold_roundtrip_preserves_declared_config_and_exact_metadata() {
    let root = tempfile::tempdir().unwrap();
    let db = path(root.path());
    let store = ConfigProfileCatalog::new(&db);
    let entry = store.create(ID, "  Ä profile  ", text()).unwrap();
    let expected =
        export_config_profile(&parse_config_profile(text().as_bytes()).unwrap()).unwrap();
    assert_eq!(entry.revision, "1");
    assert_eq!(entry.profile_sha256, digest(&expected));
    assert_eq!(entry.profile_bytes, expected.len());
    assert_eq!(entry.name, "  Ä profile  ");
    assert!(entry.created_at.ends_with('Z'));
    assert!(entry.created_at.len() <= 64);
    let cold = ConfigProfileCatalog::new(&db).get(ID).unwrap();
    assert_eq!(cold.entry, entry);
    assert_eq!(cold.profile_text.as_bytes(), expected);
    assert!(cold.profile_text.contains("9007199254740993"));
    assert!(cold.profile_text.contains("\"custom_search\":null"));
    assert!(!cold.profile_text.contains("seed_urls"));
    assert_eq!(
        raw(&db)
            .pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
            .unwrap(),
        "delete"
    );
    let second = store.create(OTHER, "  ä profile  ", text()).unwrap();
    assert_ne!(second.name, entry.name);
    kind(
        store.create("00000000-0000-4000-8000-000000000003", &entry.name, text()),
        ConfigProfileCatalogErrorKind::Conflict,
    );
}

#[test]
fn catalog_cas_requires_both_tokens_and_rename_retains_profile() {
    let root = tempfile::tempdir().unwrap();
    let db = path(root.path());
    let store = ConfigProfileCatalog::new(&db);
    let first = store.create(ID, "first", text()).unwrap();
    let original = store.get(ID).unwrap().profile_text;
    let before = fs::read(&db).unwrap();
    kind(
        store.rename(ID, "1", &"0".repeat(64), "wrong"),
        ConfigProfileCatalogErrorKind::Conflict,
    );
    kind(
        store.rename(ID, "2", &first.profile_sha256, "wrong"),
        ConfigProfileCatalogErrorKind::Conflict,
    );
    assert_eq!(fs::read(&db).unwrap(), before);
    let renamed = store
        .rename(ID, &first.revision, &first.profile_sha256, "renamed")
        .unwrap();
    assert_eq!(renamed.revision, "2");
    assert_eq!(renamed.profile_sha256, first.profile_sha256);
    assert_eq!(renamed.created_at, first.created_at);
    assert_eq!(store.get(ID).unwrap().profile_text, original);
    kind(
        store.delete(ID, &first.revision, &first.profile_sha256),
        ConfigProfileCatalogErrorKind::Conflict,
    );
    let next = store
        .replace(
            ID,
            &renamed.revision,
            &renamed.profile_sha256,
            "new",
            r#"{"version":1,"config":{}}"#,
        )
        .unwrap();
    assert_eq!(next.revision, "3");
    assert_ne!(next.profile_sha256, first.profile_sha256);
    store
        .delete(ID, &next.revision, &next.profile_sha256)
        .unwrap();
    assert!(store.list().unwrap().is_empty());
    kind(store.get(ID), ConfigProfileCatalogErrorKind::Missing);
}

#[test]
fn catalog_independent_connections_race_without_lost_update() {
    let root = tempfile::tempdir().unwrap();
    let db = path(root.path());
    let first = ConfigProfileCatalog::new(&db)
        .create(ID, "first", text())
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = ["left", "right"]
        .into_iter()
        .map(|name| {
            let barrier = barrier.clone();
            let db = db.clone();
            let first = first.clone();
            std::thread::spawn(move || {
                barrier.wait();
                ConfigProfileCatalog::new(db).rename(
                    ID,
                    &first.revision,
                    &first.profile_sha256,
                    name,
                )
            })
        })
        .collect();
    barrier.wait();
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(outcomes.iter().filter(|result|matches!(result,Err(error) if error.kind==ConfigProfileCatalogErrorKind::Conflict)).count(),1);
    assert_eq!(
        ConfigProfileCatalog::new(db)
            .get(ID)
            .unwrap()
            .entry
            .revision,
        "2"
    );
}

#[test]
fn catalog_name_entry_and_logical_byte_boundaries_are_atomic() {
    let root = tempfile::tempdir().unwrap();
    let db = path(root.path());
    let canonical_len = export_config_profile(&parse_config_profile(text().as_bytes()).unwrap())
        .unwrap()
        .len();
    let store = ConfigProfileCatalog {
        path: db.clone(),
        limits: Limits {
            entries: 2,
            profile: canonical_len,
            total: canonical_len * 2,
        },
    };
    let max_name = "ä".repeat(128);
    let one = store.create(ID, &max_name, text()).unwrap();
    kind(
        store.create(OTHER, &format!("{max_name}a"), text()),
        ConfigProfileCatalogErrorKind::Validation,
    );
    store.create(OTHER, "other", text()).unwrap();
    let before = fs::read(&db).unwrap();
    kind(
        store.create("00000000-0000-4000-8000-000000000003", "third", text()),
        ConfigProfileCatalogErrorKind::Limit,
    );
    kind(
        store.replace(
            ID,
            &one.revision,
            &one.profile_sha256,
            "first",
            &format!("{} ", text()),
        ),
        ConfigProfileCatalogErrorKind::Limit,
    );
    assert_eq!(fs::read(&db).unwrap(), before);
    let small = ConfigProfileCatalog {
        path: db.clone(),
        limits: Limits {
            entries: 2,
            profile: canonical_len,
            total: canonical_len * 2 - 1,
        },
    };
    kind(small.list(), ConfigProfileCatalogErrorKind::Unavailable);
    store
        .delete(ID, &one.revision, &one.profile_sha256)
        .unwrap();
    let reduced = ConfigProfileCatalog {
        path: db.clone(),
        limits: Limits {
            entries: 2,
            profile: canonical_len,
            total: canonical_len * 2 - 1,
        },
    };
    kind(
        reduced.create(ID, "first", text()),
        ConfigProfileCatalogErrorKind::Limit,
    );
    assert_eq!(reduced.list().unwrap().len(), 1);
}

#[test]
fn catalog_actual_128_entry_limit_and_revision_exhaustion() {
    let root = tempfile::tempdir().unwrap();
    let db = path(root.path());
    let store = ConfigProfileCatalog::new(&db);
    for n in 0..128 {
        store
            .create(
                &format!("00000000-0000-4000-8000-{n:012x}"),
                &format!("profile{n}"),
                r#"{"version":1,"config":{}}"#,
            )
            .unwrap();
    }
    kind(
        store.create("00000000-0000-4000-8000-000000000080", "overflow", text()),
        ConfigProfileCatalogErrorKind::Limit,
    );
    let first = store.get(ID).unwrap().entry;
    raw(&db)
        .execute(
            "UPDATE configuration_profiles SET revision=?1 WHERE id=?2",
            params![i64::MAX, ID],
        )
        .unwrap();
    let before = fs::read(&db).unwrap();
    kind(
        store.rename(ID, &i64::MAX.to_string(), &first.profile_sha256, "overflow"),
        ConfigProfileCatalogErrorKind::Limit,
    );
    assert_eq!(fs::read(&db).unwrap(), before);
}

#[test]
fn catalog_existing_corruption_and_schema_never_reset_or_become_empty() {
    let root = tempfile::tempdir().unwrap();
    let db = path(root.path());
    fs::create_dir_all(db.parent().unwrap()).unwrap();
    for bytes in [b"".as_slice(), b"not SQLite".as_slice()] {
        fs::write(&db, bytes).unwrap();
        let store = ConfigProfileCatalog::new(&db);
        kind(store.list(), ConfigProfileCatalogErrorKind::Unavailable);
        kind(
            store.create(ID, "profile", text()),
            ConfigProfileCatalogErrorKind::Unavailable,
        );
        assert_eq!(fs::read(&db).unwrap(), bytes);
    }
    fs::remove_file(&db).unwrap();
    let store = ConfigProfileCatalog::new(&db);
    store.create(ID, "first", text()).unwrap();
    raw(&db).pragma_update(None, "user_version", 2).unwrap();
    let before = fs::read(&db).unwrap();
    kind(store.list(), ConfigProfileCatalogErrorKind::Unavailable);
    assert_eq!(fs::read(&db).unwrap(), before);
    raw(&db).pragma_update(None, "user_version", 1).unwrap();
    raw(&db)
        .execute_batch(
            "CREATE TRIGGER unexpected AFTER DELETE ON configuration_profiles BEGIN SELECT 1; END;",
        )
        .unwrap();
    kind(store.list(), ConfigProfileCatalogErrorKind::Unavailable);
    raw(&db).execute_batch("DROP TRIGGER unexpected;").unwrap();
    raw(&db)
        .execute(
            "UPDATE configuration_profiles SET profile_sha256=?1",
            ["0".repeat(64)],
        )
        .unwrap();
    kind(store.list(), ConfigProfileCatalogErrorKind::Unavailable);
}

#[test]
fn catalog_sqlite_lookalike_schema_objects_are_refused_before_reads_or_writes() {
    let root = tempfile::tempdir().unwrap();
    let db = path(root.path());
    let store = ConfigProfileCatalog::new(&db);
    let entry = store.create(ID, "first", text()).unwrap();
    for (object, ddl) in [
        (
            "TRIGGER",
            "CREATE TRIGGER sqliteXunexpected AFTER UPDATE ON configuration_profiles BEGIN SELECT RAISE(ABORT,'unexpected trigger executed'); END;",
        ),
        ("TABLE", "CREATE TABLE sqliteXunexpected(value TEXT);"),
        (
            "VIEW",
            "CREATE VIEW sqliteXunexpected AS SELECT profile_text FROM configuration_profiles;",
        ),
    ] {
        raw(&db).execute_batch(ddl).unwrap();
        let before = fs::read(&db).unwrap();
        kind(store.list(), ConfigProfileCatalogErrorKind::Unavailable);
        kind(store.get(ID), ConfigProfileCatalogErrorKind::Unavailable);
        let refusal = store
            .rename(
                ID,
                &entry.revision,
                &entry.profile_sha256,
                "should not execute",
            )
            .unwrap_err();
        assert_eq!(refusal.kind, ConfigProfileCatalogErrorKind::Unavailable);
        assert!(refusal.to_string().contains("schema"));
        assert!(!refusal.to_string().contains("unexpected trigger executed"));
        assert_eq!(fs::read(&db).unwrap(), before);
        raw(&db)
            .execute_batch(&format!("DROP {object} sqliteXunexpected;"))
            .unwrap();
    }
    assert_eq!(store.get(ID).unwrap().entry, entry);
}

#[test]
fn catalog_stored_size_preflight_and_canonical_format_are_validated() {
    let root = tempfile::tempdir().unwrap();
    let db = path(root.path());
    let store = ConfigProfileCatalog::new(&db);
    let entry = store.create(ID, "first", text()).unwrap();
    let connection = raw(&db);
    for timestamp in [
        "2026-10-18T13:00:00.123+01:00",
        "2026-10-18T12:00:00.123456789Z",
        "+010000-01-01T00:00:00.000Z",
        "2026-02-30T12:00:00.000Z",
    ] {
        connection
            .execute(
                "UPDATE configuration_profiles SET created_at=?1",
                [timestamp],
            )
            .unwrap();
        let before = fs::read(&db).unwrap();
        kind(store.list(), ConfigProfileCatalogErrorKind::Unavailable);
        kind(store.get(ID), ConfigProfileCatalogErrorKind::Unavailable);
        assert_eq!(fs::read(&db).unwrap(), before);
    }
    connection
        .execute(
            "UPDATE configuration_profiles SET created_at=?1",
            [&entry.created_at],
        )
        .unwrap();
    assert_eq!(store.get(ID).unwrap().entry, entry);
    let invalid = r#"{"version":1,"config":{"custom_headers":{"X-Secret":"sentinel"}}}"#;
    connection
        .execute(
            "UPDATE configuration_profiles SET profile_text=?1,profile_bytes=?2,profile_sha256=?3",
            params![invalid, invalid.len() as i64, digest(invalid.as_bytes())],
        )
        .unwrap();
    kind(store.list(), ConfigProfileCatalogErrorKind::Unavailable);
    connection.execute("UPDATE configuration_profiles SET profile_text=CAST(zeroblob(4096) AS TEXT),profile_bytes=4096",[]).unwrap();
    let bounded = ConfigProfileCatalog {
        path: db.clone(),
        limits: Limits {
            entries: 128,
            profile: 1024,
            total: 2048,
        },
    };
    kind(bounded.list(), ConfigProfileCatalogErrorKind::Unavailable);
    drop(connection);
    fs::remove_file(&db).unwrap();
    fs::create_dir(&db).unwrap();
    kind(store.list(), ConfigProfileCatalogErrorKind::Unavailable);
}

#[cfg(windows)]
#[test]
fn catalog_deep_unicode_windows_path_uses_shared_sqlite_adapter() {
    let root = tempfile::tempdir().unwrap();
    let mut directory = root.path().to_path_buf();
    for _ in 0..10 {
        directory.push("路".repeat(40));
    }
    let db = path(&directory);
    assert!(db.to_string_lossy().len() > 1040);
    let store = ConfigProfileCatalog::new(&db);
    let created = store.create(ID, "deep", text()).unwrap();
    let cold = ConfigProfileCatalog::new(&db);
    assert_eq!(cold.get(ID).unwrap().entry, created);
    cold.delete(ID, &created.revision, &created.profile_sha256)
        .unwrap();
    assert!(cold.list().unwrap().is_empty());
    assert!(db.is_file());
}
