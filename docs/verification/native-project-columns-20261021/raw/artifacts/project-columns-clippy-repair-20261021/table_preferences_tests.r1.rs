use super::*;
use axum::body::{Body, to_bytes};
use serde_json::{Value, json};
use std::sync::{Arc, Barrier};

fn entries() -> Vec<Entry> {
    strict(br#"[{"section":"titles","schema":[{"key":"url","type":"text","mandatory":true},{"key":"title","type":"text","mandatory":false},{"key":"length","type":"number","mandatory":false}],"visible_keys":["url","title"]}]"#, MAX_REQUEST_BYTES).unwrap()
}
fn snapshot(data: &FsPath, id: Uuid) -> Reply {
    let dir = directory(data, false).unwrap();
    read_snapshot(dir.as_ref(), id).unwrap()
}
fn doc_path(data: &FsPath, id: Uuid) -> PathBuf {
    data.join(DIRECTORY).join(filename(id))
}
fn decoded(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}
fn request(bytes: Vec<u8>) -> Request {
    Request::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(bytes))
        .unwrap()
}
fn app_state(data: &FsPath) -> AppState {
    AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Default::default(),
        data_dir: Arc::new(data.into()),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle: Default::default(),
    }
}
async fn registered(data: &FsPath, id: Uuid) -> AppState {
    let state = app_state(data);
    super::super::register_report(&state, id, open_seo_spider_core::CrawlReport::default()).await;
    state
}
async fn response_value(response: Response) -> Value {
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-store"
    );
    decoded(
        &to_bytes(response.into_body(), MAX_REPLY_BYTES)
            .await
            .unwrap(),
    )
}

#[test]
fn missing_read_is_nonmutating_and_save_is_raw_bound_not_report_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let report_path = directory.path().join(format!("{id}.json"));
    let report = b"{\"untouched_report\":true}\r\n";
    std::fs::write(&report_path, report).unwrap();
    let missing = decoded(&load(directory.path(), id).unwrap());
    assert_eq!(missing["fingerprint"], json!({"state":"missing"}));
    assert!(missing["document"].is_null());
    assert!(!directory.path().join(DIRECTORY).exists());
    let saved = decoded(
        &mutate(
            directory.path(),
            id,
            Fingerprint::Missing {},
            entries(),
            false,
        )
        .unwrap(),
    );
    let raw = std::fs::read(doc_path(directory.path(), id)).unwrap();
    assert_eq!(saved["fingerprint"]["sha256"], sha(&raw));
    assert_eq!(saved["document"], decoded(&raw));
    assert_eq!(saved, decoded(&load(directory.path(), id).unwrap()));
    assert_eq!(std::fs::read(&report_path).unwrap(), report);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
fn reset_is_a_present_empty_new_revision_and_prevents_missing_and_old_present_aba() {
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    mutate(
        directory.path(),
        id,
        Fingerprint::Missing {},
        entries(),
        false,
    )
    .unwrap();
    let first = snapshot(directory.path(), id);
    let first_revision = first.document.as_ref().unwrap().revision;
    let reset = decoded(
        &mutate(
            directory.path(),
            id,
            first.fingerprint.clone(),
            vec![],
            true,
        )
        .unwrap(),
    );
    assert_eq!(reset["fingerprint"]["state"], "present");
    assert_eq!(reset["document"]["entries"], json!([]));
    assert_ne!(reset["document"]["revision"], first_revision.to_string());
    assert_eq!(
        mutate(
            directory.path(),
            id,
            Fingerprint::Missing {},
            entries(),
            false
        )
        .unwrap_err()
        .code,
        "stale_preferences"
    );
    assert_eq!(
        mutate(directory.path(), id, first.fingerprint, entries(), false)
            .unwrap_err()
            .code,
        "stale_preferences"
    );
    let empty = snapshot(directory.path(), id);
    mutate(directory.path(), id, empty.fingerprint, entries(), false).unwrap();
    assert_eq!(
        snapshot(directory.path(), id).document.unwrap().entries,
        entries()
    );
    assert!(
        directory
            .path()
            .join(DIRECTORY)
            .join(format!("{id}.lock"))
            .is_file()
    );
}

#[test]
fn invalid_bytes_are_preserved_with_exact_sha_and_only_explicit_reset_recovers() {
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    std::fs::create_dir(directory.path().join(DIRECTORY)).unwrap();
    for raw in [
        b"{not-json\r\n".as_slice(),
        b"".as_slice(),
        b"{\"engine\":\"future\"}".as_slice(),
        b"\xff".as_slice(),
    ] {
        std::fs::write(doc_path(directory.path(), id), raw).unwrap();
        let base = snapshot(directory.path(), id);
        assert_eq!(base.fingerprint, Fingerprint::Invalid { sha256: sha(raw) });
        assert!(base.document.is_none());
        assert_eq!(base.error.unwrap().code, "stored_document_invalid");
        let failure = mutate(
            directory.path(),
            id,
            base.fingerprint.clone(),
            entries(),
            false,
        )
        .unwrap_err();
        assert_eq!(failure.status, StatusCode::CONFLICT);
        assert_eq!(failure.code, "invalid_saved_preferences");
        assert_eq!(std::fs::read(doc_path(directory.path(), id)).unwrap(), raw);
        mutate(directory.path(), id, base.fingerprint, vec![], true).unwrap();
        assert!(
            snapshot(directory.path(), id)
                .document
                .unwrap()
                .entries
                .is_empty()
        );
    }
}

#[test]
fn raw_byte_cas_rejects_valid_whitespace_edits_and_invalid_reset_stale_hash() {
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    mutate(
        directory.path(),
        id,
        Fingerprint::Missing {},
        entries(),
        false,
    )
    .unwrap();
    let base = snapshot(directory.path(), id);
    let mut raw = std::fs::read(doc_path(directory.path(), id)).unwrap();
    raw.extend_from_slice(b"\r\n");
    std::fs::write(doc_path(directory.path(), id), &raw).unwrap();
    assert_eq!(
        mutate(directory.path(), id, base.fingerprint, entries(), false)
            .unwrap_err()
            .code,
        "stale_preferences"
    );
    assert_eq!(std::fs::read(doc_path(directory.path(), id)).unwrap(), raw);
    std::fs::write(doc_path(directory.path(), id), b"first corrupt").unwrap();
    let invalid = snapshot(directory.path(), id);
    std::fs::write(doc_path(directory.path(), id), b"second corrupt").unwrap();
    assert_eq!(
        mutate(directory.path(), id, invalid.fingerprint, vec![], true)
            .unwrap_err()
            .code,
        "stale_preferences"
    );
    assert_eq!(
        std::fs::read(doc_path(directory.path(), id)).unwrap(),
        b"second corrupt"
    );
}

#[test]
fn cooperating_race_accepts_one_missing_cas_and_never_loses_the_winner() {
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let barrier = Arc::new(Barrier::new(3));
    let workers = [true, false]
        .into_iter()
        .map(|include_title| {
            let data = directory.path().to_owned();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut value = entries();
                if !include_title {
                    value[0].visible_keys = vec!["url".into()];
                }
                barrier.wait();
                let result = mutate(&data, id, Fingerprint::Missing {}, value.clone(), false);
                (value, result)
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let results = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        results.iter().filter(|(_, result)| result.is_ok()).count(),
        1
    );
    for (_, result) in &results {
        if let Err(error) = result {
            assert_eq!(error.status, StatusCode::CONFLICT);
        }
    }
    let winner = results
        .into_iter()
        .find(|(_, result)| result.is_ok())
        .unwrap()
        .0;
    assert_eq!(
        snapshot(directory.path(), id).document.unwrap().entries,
        winner
    );
}

#[test]
fn lock_contention_is_bounded_and_drop_releases_for_a_separate_handle() {
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let dir = super::directory(directory.path(), true).unwrap().unwrap();
    let held = project_lock(&dir, id).unwrap();
    assert_eq!(
        mutate(
            directory.path(),
            id,
            Fingerprint::Missing {},
            entries(),
            false
        )
        .unwrap_err()
        .code,
        "preferences_busy"
    );
    assert!(!doc_path(directory.path(), id).exists());
    drop(held);
    mutate(
        directory.path(),
        id,
        Fingerprint::Missing {},
        entries(),
        false,
    )
    .unwrap();
    let next = project_lock(&dir, id).unwrap();
    drop(next);
}

#[test]
fn native_file_locks_coordinate_an_actual_second_process() {
    const ROOT_ENV: &str = "OPEN_SEO_COLUMNS_UNIT_LOCK_ROOT";
    const ID_ENV: &str = "OPEN_SEO_COLUMNS_UNIT_LOCK_ID";
    const PHASE_ENV: &str = "OPEN_SEO_COLUMNS_UNIT_LOCK_PHASE";
    if let Some(data) = std::env::var_os(ROOT_ENV) {
        let id = Uuid::parse_str(&std::env::var(ID_ENV).unwrap()).unwrap();
        let result = mutate(
            FsPath::new(&data),
            id,
            Fingerprint::Missing {},
            entries(),
            false,
        );
        match std::env::var(PHASE_ENV).unwrap().as_str() {
            "held" => assert_eq!(result.unwrap_err().code, "preferences_busy"),
            "released" => {
                result.unwrap();
            }
            phase => panic!("Unexpected isolated child phase: {phase}"),
        }
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let dir = super::directory(directory.path(), true).unwrap().unwrap();
    let held = project_lock(&dir, id).unwrap();
    let run = |phase| {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "server::table_preferences::tests::native_file_locks_coordinate_an_actual_second_process", "--nocapture"])
            .env(ROOT_ENV, directory.path()).env(ID_ENV, id.to_string()).env(PHASE_ENV, phase)
            .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Isolated file-lock child did not finish");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("1 passed"),
            "The exact child test must actually execute"
        );
    };
    run("held");
    assert!(!doc_path(directory.path(), id).exists());
    drop(held);
    run("released");
    assert_eq!(
        snapshot(directory.path(), id).document.unwrap().entries,
        entries()
    );
}

#[test]
fn project_and_section_isolation_and_foreign_owner_validation_preserve_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut two = entries();
    let mut other = two[0].clone();
    other.section = "status".into();
    two.push(other);
    mutate(
        directory.path(),
        a,
        Fingerprint::Missing {},
        two.clone(),
        false,
    )
    .unwrap();
    mutate(
        directory.path(),
        b,
        Fingerprint::Missing {},
        entries(),
        false,
    )
    .unwrap();
    let a_bytes = std::fs::read(doc_path(directory.path(), a)).unwrap();
    let b_bytes = std::fs::read(doc_path(directory.path(), b)).unwrap();
    let base = snapshot(directory.path(), a);
    two[0].visible_keys = vec!["url".into()];
    mutate(directory.path(), a, base.fingerprint, two.clone(), false).unwrap();
    assert_eq!(
        snapshot(directory.path(), a).document.unwrap().entries[1],
        two[1]
    );
    assert_eq!(
        std::fs::read(doc_path(directory.path(), b)).unwrap(),
        b_bytes
    );
    std::fs::write(doc_path(directory.path(), b), &a_bytes).unwrap();
    assert!(matches!(
        snapshot(directory.path(), b).fingerprint,
        Fingerprint::Invalid { .. }
    ));
    assert_eq!(
        std::fs::read(doc_path(directory.path(), b)).unwrap(),
        a_bytes
    );
}

#[test]
fn typed_boundary_refuses_unknown_duplicate_and_unrepresentable_projection_data() {
    let good = serde_json::to_value(&entries()).unwrap();
    let mut variants = vec![];
    let mut bad = good.clone();
    bad[0]["rules"] = json!([]);
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["schema"][0]["extra"] = json!(true);
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["schema"][0]["type"] = json!("boolean");
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["schema"][0]["mandatory"] = json!(1);
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["schema"][0]["mandatory"] = json!(false);
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["schema"] = json!([["url", "text", true]]);
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["schema"] = json!([]);
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["visible_keys"] = json!([]);
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["visible_keys"] = json!(["title"]);
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["visible_keys"] = json!(["title", "url"]);
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["visible_keys"] = json!(["url", "url"]);
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["visible_keys"] = json!(["url", "missing"]);
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["schema"][1]["key"] = json!("url");
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["schema"][0]["key"] = json!("雪".repeat(86));
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["section"] = json!("\u{feff}\t");
    variants.push(bad);
    let mut bad = good.clone();
    bad[0]["section"] = json!("x\0y");
    variants.push(bad);
    let mut bad = good.clone();
    bad.as_array_mut().unwrap().push(good[0].clone());
    variants.push(bad);
    for value in variants {
        let result = strict::<Vec<Entry>>(&serde_json::to_vec(&value).unwrap(), MAX_REQUEST_BYTES)
            .and_then(|value| validate_entries(&value));
        assert!(result.is_err(), "{value}");
    }
    for raw in [
        r#"{"expected":null,"entries":[]}"#,
        r#"{"entries":[]}"#,
        r#"{"expected":{"state":"missing","sha256":null},"entries":[]}"#,
        r#"{"expected":{"state":"present","revision":null,"sha256":"00"},"entries":[]}"#,
        r#"{"expected":{"state":"missing"},"entries":[],"entries":[]}"#,
    ] {
        assert!(
            strict::<SaveRequest>(raw.as_bytes(), MAX_REQUEST_BYTES).is_err(),
            "{raw}"
        );
    }
    let mut over = entries();
    over[0].schema = (0..129)
        .map(|n| Column {
            key: format!("key{n}"),
            r#type: ColumnType::Auto,
            mandatory: false,
        })
        .collect();
    over[0].visible_keys = vec!["key0".into()];
    assert!(validate_entries(&over).is_err());
    let over = (0..65)
        .map(|n| {
            let mut entry = entries().remove(0);
            entry.section = format!("section{n}");
            entry
        })
        .collect::<Vec<_>>();
    assert!(validate_entries(&over).is_err());
}

#[test]
fn oversized_file_has_no_partial_fingerprint_and_mutation_limits_precede_publication() {
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let dir = super::directory(directory.path(), true).unwrap().unwrap();
    let file = dir
        .open_with(
            filename(id),
            OpenOptions::new().write(true).create_new(true),
        )
        .unwrap();
    file.into_std()
        .set_len(MAX_DOCUMENT_BYTES as u64 + 1)
        .unwrap();
    let error = load(directory.path(), id).unwrap_err();
    assert_eq!(error.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(error.code, "stored_preferences_too_large");
    assert_eq!(
        std::fs::metadata(doc_path(directory.path(), id))
            .unwrap()
            .len(),
        MAX_DOCUMENT_BYTES as u64 + 1
    );
    let mut large = vec![];
    for section in 0..64 {
        let mut entry = entries().remove(0);
        entry.section = format!("section{section}");
        entry.schema = (0..128)
            .map(|n| Column {
                key: format!("{n}{}", "x".repeat(240)),
                r#type: ColumnType::Text,
                mandatory: n == 0,
            })
            .collect();
        entry.visible_keys = entry
            .schema
            .iter()
            .map(|column| column.key.clone())
            .collect();
        large.push(entry);
    }
    let other = Uuid::new_v4();
    assert_eq!(
        mutate(
            directory.path(),
            other,
            Fingerprint::Missing {},
            large,
            false
        )
        .unwrap_err()
        .status,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert!(!doc_path(directory.path(), other).exists());
    assert!(
        !directory
            .path()
            .join(DIRECTORY)
            .join(format!("{other}.lock"))
            .exists()
    );
    let value = json!({"unicode":"雪 Ä","quote":"\"\\\n"});
    let raw = serde_json::to_vec(&value).unwrap();
    assert_eq!(encode(&value, raw.len()).unwrap(), raw);
    assert!(encode(&value, raw.len() - 1).is_err());
}

#[test]
fn valid_preferences_at_full_512kib_encode_save_and_load_but_one_more_byte_refuses_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let entries = (0..8)
        .map(|section| {
            let schema = (0..128)
                .map(|column| Column {
                    key: format!("c{column:04}"),
                    r#type: ColumnType::Text,
                    mandatory: column == 0,
                })
                .collect::<Vec<_>>();
            Entry {
                section: format!("section{section}"),
                visible_keys: schema.iter().map(|column| column.key.clone()).collect(),
                schema,
            }
        })
        .collect();
    let mut document = Document {
        engine: ENGINE.into(),
        version: 1,
        project_id: id,
        revision: Uuid::new_v4(),
        entries,
    };
    let mut remaining = MAX_DOCUMENT_BYTES - serde_json::to_vec(&document).unwrap().len();
    for entry in &mut document.entries {
        for (index, column) in entry.schema.iter_mut().enumerate() {
            let add = (remaining / 2).min(256 - column.key.len());
            column.key.push_str(&"x".repeat(add));
            entry.visible_keys[index] = column.key.clone();
            remaining -= 2 * add;
        }
    }
    assert!(remaining <= 1);
    if remaining == 1 {
        document.entries[0].section.push('x');
    }
    validate_entries(&document.entries).unwrap();
    assert_eq!(
        serde_json::to_vec(&document).unwrap().len(),
        MAX_DOCUMENT_BYTES
    );
    let response = mutate(
        directory.path(),
        id,
        Fingerprint::Missing {},
        document.entries.clone(),
        false,
    )
    .unwrap();
    assert!(response.len() <= MAX_REPLY_BYTES);
    let raw = std::fs::read(doc_path(directory.path(), id)).unwrap();
    assert_eq!(raw.len(), MAX_DOCUMENT_BYTES);
    assert_eq!(decoded(&response)["fingerprint"]["sha256"], sha(&raw));
    assert_eq!(
        snapshot(directory.path(), id).document.unwrap().entries,
        document.entries
    );
    document.entries[0].section.push('y');
    let refused_id = Uuid::new_v4();
    let error = mutate(
        directory.path(),
        refused_id,
        Fingerprint::Missing {},
        document.entries,
        false,
    )
    .unwrap_err();
    assert_eq!(error.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(!doc_path(directory.path(), refused_id).exists());
    assert!(
        !directory
            .path()
            .join(DIRECTORY)
            .join(format!("{refused_id}.lock"))
            .exists()
    );
    assert_eq!(std::fs::read(doc_path(directory.path(), id)).unwrap(), raw);
}

#[test]
fn failed_rename_cleans_owned_temp_and_temp_collision_never_deletes_foreign_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let dir = super::directory(directory.path(), true).unwrap().unwrap();
    dir.create_dir("blocked.json").unwrap();
    std::fs::write(
        directory.path().join(DIRECTORY).join("blocked.json/keep"),
        b"keep",
    )
    .unwrap();
    assert!(publish_named(&dir, "blocked.json", ".owned.tmp", b"new").is_err());
    assert!(!directory.path().join(DIRECTORY).join(".owned.tmp").exists());
    assert_eq!(
        std::fs::read(directory.path().join(DIRECTORY).join("blocked.json/keep")).unwrap(),
        b"keep"
    );
    std::fs::write(
        directory.path().join(DIRECTORY).join(".foreign.tmp"),
        b"foreign",
    )
    .unwrap();
    assert!(publish_named(&dir, "fresh.json", ".foreign.tmp", b"new").is_err());
    assert_eq!(
        std::fs::read(directory.path().join(DIRECTORY).join(".foreign.tmp")).unwrap(),
        b"foreign"
    );
    assert!(!directory.path().join(DIRECTORY).join("fresh.json").exists());
}

#[cfg(windows)]
#[test]
fn windows_real_sharing_denial_keeps_prior_document_and_cleans_temp() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    mutate(
        directory.path(),
        id,
        Fingerprint::Missing {},
        entries(),
        false,
    )
    .unwrap();
    let base = snapshot(directory.path(), id);
    let before = std::fs::read(doc_path(directory.path(), id)).unwrap();
    let deny_delete = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(doc_path(directory.path(), id))
        .unwrap();
    assert!(mutate(directory.path(), id, base.fingerprint, vec![], true).is_err());
    assert_eq!(
        std::fs::read(doc_path(directory.path(), id)).unwrap(),
        before
    );
    assert!(
        std::fs::read_dir(directory.path().join(DIRECTORY))
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp"))
    );
    drop(deny_delete);
}

#[cfg(unix)]
#[test]
fn nofollow_refuses_sidecar_and_parent_symlinks_without_touching_target() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    std::fs::write(outside.path().join("private"), b"untouched").unwrap();
    std::fs::create_dir(directory.path().join(DIRECTORY)).unwrap();
    symlink(
        outside.path().join("private"),
        doc_path(directory.path(), id),
    )
    .unwrap();
    assert!(load(directory.path(), id).is_err());
    assert!(
        mutate(
            directory.path(),
            id,
            Fingerprint::Missing {},
            entries(),
            false
        )
        .is_err()
    );
    assert_eq!(
        std::fs::read(outside.path().join("private")).unwrap(),
        b"untouched"
    );
    std::fs::remove_file(doc_path(directory.path(), id)).unwrap();
    std::fs::remove_dir(directory.path().join(DIRECTORY)).unwrap();
    symlink(outside.path(), directory.path().join(DIRECTORY)).unwrap();
    assert!(load(directory.path(), id).is_err());
    assert!(
        mutate(
            directory.path(),
            id,
            Fingerprint::Missing {},
            entries(),
            false
        )
        .is_err()
    );
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn nofollow_refuses_a_real_windows_parent_junction() {
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let link = directory.path().join(DIRECTORY);
    let status = std::process::Command::new("cmd.exe")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(&link)
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let read = load(directory.path(), id);
    let write = mutate(
        directory.path(),
        id,
        Fingerprint::Missing {},
        entries(),
        false,
    );
    // Remove only this owned junction itself, before either tempdir is dropped.
    assert_eq!(link.parent(), Some(directory.path()));
    std::fs::remove_dir(&link).unwrap();
    assert!(read.is_err());
    assert!(write.is_err());
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[test]
fn parent_traversal_is_never_used_as_a_preferences_root() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("..");
    let id = Uuid::new_v4();
    assert!(load(&path, id).is_err());
    assert!(mutate(&path, id, Fingerprint::Missing {}, entries(), false).is_err());
}

#[tokio::test]
async fn handlers_gate_job_content_type_limits_and_strict_body_before_files() {
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let state = registered(directory.path(), id).await;
    assert_eq!(
        get(State(state.clone()), Path(Uuid::new_v4()))
            .await
            .unwrap_err()
            .status,
        StatusCode::NOT_FOUND
    );
    let missing = response_value(get(State(state.clone()), Path(id)).await.unwrap()).await;
    assert_eq!(missing["fingerprint"], json!({"state":"missing"}));
    let invalid = request(br#"{"expected":{"state":"missing"},"entries":[],"rules":[]}"#.to_vec());
    assert_eq!(
        put(State(state.clone()), Path(id), invalid)
            .await
            .unwrap_err()
            .status,
        StatusCode::BAD_REQUEST
    );
    let wrong_type = Request::builder().body(Body::from("{}")).unwrap();
    assert_eq!(
        put(State(state.clone()), Path(id), wrong_type)
            .await
            .unwrap_err()
            .status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    let too_large = request(vec![b' '; MAX_REQUEST_BYTES + 1]);
    assert_eq!(
        put(State(state.clone()), Path(id), too_large)
            .await
            .unwrap_err()
            .status,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert!(!directory.path().join(DIRECTORY).exists());
    let saved = response_value(
        put(
            State(state.clone()),
            Path(id),
            request(
                serde_json::to_vec(&json!({"expected":{"state":"missing"},"entries":entries()}))
                    .unwrap(),
            ),
        )
        .await
        .unwrap(),
    )
    .await;
    let reset_body =
        request(serde_json::to_vec(&json!({"expected":saved["fingerprint"]})).unwrap());
    let empty = response_value(
        reset(State(state.clone()), Path(id), reset_body)
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(empty["document"]["entries"], json!([]));
    let a = state
        .lifecycle
        .table_preferences_gate
        .clone()
        .try_acquire_owned()
        .unwrap();
    let b = state
        .lifecycle
        .table_preferences_gate
        .clone()
        .try_acquire_owned()
        .unwrap();
    assert_eq!(
        get(State(state.clone()), Path(id))
            .await
            .unwrap_err()
            .status,
        StatusCode::SERVICE_UNAVAILABLE
    );
    drop((a, b));
    state.lifecycle.admission.close();
    assert_eq!(
        get(State(state.clone()), Path(id))
            .await
            .unwrap_err()
            .status,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accepted_owned_write_survives_receiver_abort_and_shutdown_drains_lock_and_publication() {
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let state = app_state(directory.path());
    let permit = slot(&state).unwrap();
    let (started, observed) = tokio::sync::oneshot::channel();
    let (release, released) = std::sync::mpsc::channel();
    let data = state.data_dir.clone();
    let owned_state = state.clone();
    let task = tokio::spawn(async move {
        owned(&owned_state, permit, move || {
            let dir = super::directory(&data, true)?.ok_or_else(unavailable)?;
            let _lock = project_lock(&dir, id)?;
            let bytes = encode(
                &Document {
                    engine: ENGINE.into(),
                    version: 1,
                    project_id: id,
                    revision: Uuid::new_v4(),
                    entries: entries(),
                },
                MAX_DOCUMENT_BYTES,
            )?;
            let _ = started.send(());
            released.recv().map_err(|_| unavailable())?;
            publish(&dir, id, &bytes)?;
            Ok(bytes)
        })
        .await
    });
    observed.await.unwrap();
    task.abort();
    let _ = task.await;
    assert_eq!(
        mutate(
            directory.path(),
            id,
            Fingerprint::Missing {},
            entries(),
            false
        )
        .unwrap_err()
        .code,
        "preferences_busy"
    );
    let lifecycle = state.lifecycle.clone();
    let mut drain =
        tokio::spawn(async move { lifecycle.blocking.shutdown_and_drain().await.unwrap() });
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut drain)
            .await
            .is_err()
    );
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), drain)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        snapshot(directory.path(), id).document.unwrap().entries,
        entries()
    );
    let dir = super::directory(directory.path(), false).unwrap().unwrap();
    let lock = project_lock(&dir, id).unwrap();
    drop(lock);
}
