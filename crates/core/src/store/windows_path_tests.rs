use super::super::{CrawlStore, INITIALIZE_WAL_AND_CORE_SCHEMA};
use super::*;
use sha2::{Digest, Sha256};
use std::os::windows::ffi::OsStrExt;

fn width(path: &Path) -> usize {
    path.as_os_str().encode_wide().count()
}

fn sized_path(base: &Path, units: usize, unicode: bool) -> PathBuf {
    let mut path = base.join(if unicode { "Ü漢🦀" } else { "ordinary" });
    let leaf = "capture.sqlite";
    let mut remaining = units.checked_sub(width(&path) + 1 + leaf.len()).unwrap();
    while remaining != 0 {
        assert!(remaining >= 2, "a component needs a name and separator");
        let mut chars = (remaining - 1).min(64);
        if remaining - chars - 1 == 1 {
            chars -= 1;
        }
        path.push("p".repeat(chars));
        remaining -= chars + 1;
    }
    path.push(leaf);
    assert_eq!(width(&path), units);
    path
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

#[test]
fn filesystem_names_are_lexical_extended_paths_without_changing_namespaces() {
    assert_eq!(
        filesystem_path(Path::new(r"C:\owned\a\..\document.sqlite")).unwrap(),
        Some(PathBuf::from(r"\\?\C:\owned\document.sqlite"))
    );
    assert_eq!(
        filesystem_path(Path::new(r"\\server\share\a\..\document.sqlite")).unwrap(),
        Some(PathBuf::from(r"\\?\UNC\server\share\document.sqlite"))
    );
    let relative = Path::new(r".\owned\..\document.sqlite");
    let absolute = std::path::absolute(relative).unwrap();
    assert_eq!(
        filesystem_path(relative).unwrap(),
        filesystem_path(&absolute).unwrap()
    );
    let with_separator = filesystem_path(Path::new(r"C:\owned\document.sqlite\"))
        .unwrap()
        .unwrap();
    assert!(with_separator.as_os_str().to_str().unwrap().ends_with('\\'));
    for name in [
        "",
        ":memory:",
        "file:owned?mode=memory&cache=shared",
        r"\\.\NUL",
        r"\\?\GLOBALROOT\Device\HarddiskVolume1\owned.sqlite",
        r"C:\owned\NUL",
        r"C:\owned\nul.sqlite",
        r"C:\owned\COM¹.txt",
        r"C:\owned\capture.sqlite.",
        r"C:\owned\directory \capture.sqlite",
        r"C:\owned\capture.sqlite ",
        r"C:\owned\capture.sqlite:stream",
    ] {
        assert_eq!(filesystem_path(Path::new(name)).unwrap(), None, "{name:?}");
    }
    for name in [r"\\?\C:\owned\NUL", r"\\?\UNC\server\share\literal. "] {
        assert_eq!(
            filesystem_path(Path::new(name)).unwrap(),
            Some(PathBuf::from(name))
        );
    }
}

fn wal_roundtrip(path: &Path) {
    let expected_archive = crate::sitemap_source_archive::files::archive_path(path).unwrap();
    let store = CrawlStore::open(path).unwrap();
    assert_eq!(store.source_archive_path, expected_archive);
    {
        let connection = store.connection.lock().unwrap();
        assert_eq!(
            connection
                .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
                .unwrap(),
            "wal"
        );
        connection
            .execute(
                "INSERT INTO metadata(key,value) VALUES('owned_long_path',?1)",
                ["\"full Unicode Ü漢🦀 value\""],
            )
            .unwrap();
        connection
            .execute("CREATE TABLE owned_path_witness(value TEXT NOT NULL)", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO owned_path_witness(value) VALUES(?1)",
                ["independent row – Ü漢🦀"],
            )
            .unwrap();
        assert_eq!(
            connection
                .query_row("SELECT value FROM owned_path_witness", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "independent row – Ü漢🦀"
        );
    }
    // The database and WAL siblings must exist at the caller's actual location.
    assert!(std::fs::metadata(path).unwrap().len() > 0);
    assert!(sibling(path, "-wal").is_file());
    assert!(sibling(path, "-shm").is_file());
    drop(store);
    let before = Sha256::digest(std::fs::read(path).unwrap());
    let cold = CrawlStore::open_read_only(path).unwrap();
    assert_eq!(cold.source_archive_path, expected_archive);
    assert!(cold.source_archive_read_only);
    let connection = cold.connection.lock().unwrap();
    assert_eq!(
        connection
            .query_row("SELECT value FROM owned_path_witness", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "independent row – Ü漢🦀"
    );
    assert!(connection.is_readonly(rusqlite::MAIN_DB).unwrap());
    let failure = connection
        .execute(
            "INSERT INTO owned_path_witness(value) VALUES('forbidden')",
            [],
        )
        .unwrap_err();
    assert!(
        matches!(failure, rusqlite::Error::SqliteFailure(code, _) if code.code == rusqlite::ErrorCode::ReadOnly)
    );
    drop(connection);
    drop(cold);
    assert_eq!(Sha256::digest(std::fs::read(path).unwrap()), before);
    // Reopening the ordinary and explicitly prefixed spelling must see that
    // same physical row, not a relocated or newly initialized database.
    let extended = filesystem_path(path).unwrap().unwrap();
    let read = CrawlStore::open_read_only(&extended).unwrap();
    assert_eq!(
        read.connection
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM owned_path_witness", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn actual_wal_and_cold_readonly_preserve_255_300_deep_unicode_and_verbatim_stores() {
    let temp = tempfile::tempdir().unwrap();
    for (units, unicode) in [(255, false), (300, false), (1200, false), (1100, true)] {
        let path = sized_path(temp.path(), units, unicode);
        if units >= 1100 {
            assert!(path.to_str().unwrap().len() > 1040);
        }
        wal_roundtrip(&path);
    }
    let path = sized_path(&temp.path().join("already-verbatim"), 1200, true);
    let extended = filesystem_path(&path).unwrap().unwrap();
    assert_eq!(
        filesystem_path(&extended).unwrap().as_ref(),
        Some(&extended)
    );
    wal_roundtrip(&extended);
    assert!(path.is_file());
}

#[test]
fn deep_missing_readonly_database_does_not_create_database_or_parent() {
    let temp = tempfile::tempdir().unwrap();
    let path = sized_path(temp.path(), 1200, true);
    let error = CrawlStore::open_read_only(&path).err().unwrap();
    assert!(
        error
            .to_string()
            .contains("open read-only SQLite connection")
    );
    assert!(error.to_string().contains("SQLite CannotOpen/"));
    assert!(error.downcast_ref::<rusqlite::Error>().is_some());
    assert!(!path.exists());
    assert!(!path.parent().unwrap().exists());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn ordinary_aliases_keep_physical_database_and_original_archive_authority() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("original.sqlite");
    let store = CrawlStore::open(&path).unwrap();
    store
        .connection
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO metadata(key,value) VALUES('physical_identity','41')",
            [],
        )
        .unwrap();
    drop(store);
    std::fs::create_dir(temp.path().join("ordinary")).unwrap();
    let dotted = temp
        .path()
        .join("ordinary")
        .join("..")
        .join("original.sqlite");
    for alias in [sibling(&path, "."), sibling(&path, " "), dotted] {
        let baseline =
            Connection::open_with_flags(&alias, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(
            baseline
                .query_row(
                    "SELECT value FROM metadata WHERE key='physical_identity'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "41"
        );
        let adapted = CrawlStore::open_read_only(&alias).unwrap();
        assert_eq!(
            adapted
                .connection
                .lock()
                .unwrap()
                .query_row(
                    "SELECT value FROM metadata WHERE key='physical_identity'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
            "41"
        );
        assert_eq!(
            adapted.source_archive_path,
            crate::sitemap_source_archive::files::archive_path(&alias).unwrap()
        );
    }
    let uncreated = temp.path().join("must-not-be-a-file.sqlite");
    assert!(CrawlStore::open(sibling(&uncreated, "\\")).is_err());
    assert!(!uncreated.exists());
    // An explicitly verbatim DOS-looking basename is a literal filesystem
    // request, unlike the ordinary NUL device name retained by the fallback.
    let literal = temp.path().join("NUL");
    let parent = filesystem_path(temp.path()).unwrap().unwrap();
    let verbatim = parent.join("NUL");
    let store = CrawlStore::open(&verbatim).unwrap();
    store
        .connection
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO metadata(key,value) VALUES('literal_verbatim_name','43')",
            [],
        )
        .unwrap();
    drop(store);
    let cold = CrawlStore::open_read_only(&verbatim).unwrap();
    assert_eq!(
        cold.connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT value FROM metadata WHERE key='literal_verbatim_name'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        "43"
    );
    drop(cold);
    assert_eq!(filesystem_path(&literal).unwrap(), None);
    std::fs::remove_file(&verbatim).unwrap();
}

#[test]
fn legacy_open_and_wal_initialization_stages_are_observed_without_assuming_host_failure() {
    let temp = tempfile::tempdir().unwrap();
    for units in [255, 300] {
        let path = sized_path(temp.path(), units, false);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let (open, wal) = match Connection::open(&path) {
            Err(error) => (format!("error: {error:?}"), "not_attempted".to_owned()),
            Ok(connection) => {
                let result = connection.execute_batch(INITIALIZE_WAL_AND_CORE_SCHEMA);
                let wal = match result {
                    Err(error) => format!("error: {error:?}"),
                    Ok(()) => {
                        connection
                            .execute(
                                "INSERT INTO metadata(key,value) VALUES('legacy_stage','1')",
                                [],
                            )
                            .unwrap();
                        assert_eq!(
                            connection
                                .query_row(
                                    "SELECT value FROM metadata WHERE key='legacy_stage'",
                                    [],
                                    |row| row.get::<_, String>(0)
                                )
                                .unwrap(),
                            "1"
                        );
                        "success_verified_write_and_read".to_owned()
                    }
                };
                ("success".to_owned(), wal)
            }
        };
        println!(
            "WINDOWS_SQLITE_LEGACY_STAGE {}",
            serde_json::json!({"db_utf16_units":units,"db_utf8_bytes":path.to_str().unwrap().len(),"journal_utf16_units":width(&sibling(&path,"-journal")),"connection_open":open,"wal_and_core_schema":wal,"database_bytes":std::fs::metadata(&path).ok().map(|metadata|metadata.len())})
        );
        // The baseline may succeed on a long-path-aware host. The adapted path
        // must work independently, and the observed stage is never fabricated.
        let adapted = path.parent().unwrap().join("adapted.sqlite");
        wal_roundtrip(&adapted);
    }
}
