use super::*;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

struct Fixture {
    path: PathBuf,
}
impl Fixture {
    fn new() -> Result<Self> {
        let path =
            std::env::temp_dir().join(format!("open-seo-mcp-files-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path)?;
        Ok(Self { path })
    }
    fn tools(&self) -> Result<FileTools> {
        FileTools::open(&self.path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Only this newly allocated, fixed-prefix test directory is eligible.
        if self.path.parent() == Some(std::env::temp_dir().as_path())
            && self
                .path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("open-seo-mcp-files-"))
        {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn file_tools_schema_and_native_path_preflight_are_strict() -> Result<()> {
    assert_send_sync::<FileTools>();
    assert_eq!(FileTools::definitions().len(), 5);
    for name in NAMES {
        assert!(FileTools::is_tool(name));
    }
    assert!(!FileTools::is_tool("spider_execute"));
    for path in [
        "",
        "/etc/passwd",
        "../escape",
        "a/../b",
        "./a",
        "a//b",
        "a/",
        "a/\0b",
    ] {
        assert!(RelativePath::parse(path.into(), false).is_err(), "{path:?}");
    }
    assert!(RelativePath::parse("a/".repeat(256) + "x", false).is_err());
    assert!(RelativePath::parse("a".repeat(PATH_MAX_BYTES + 1), false).is_err());
    assert!(RelativePath::parse(format!("{STAGING_PREFIX}x"), false).is_err());
    assert!(Operation::parse(NAMES[4], json!({"path":"x","text":"x","overwrite":false})).is_err());
    for args in [
        json!(null),
        json!([]),
        json!({"path":1,"text":"x"}),
        json!({"path":"x","text":false}),
        json!({"path":"x"}),
    ] {
        assert!(Operation::parse(NAMES[4], args).is_err());
    }
    for args in [
        json!({"path":"x","offset":1}),
        json!({"path":"x","offset":-1}),
        json!({"path":"x","max_bytes":0}),
        json!({"path":"x","max_bytes":READ_CHUNK_MAX_BYTES+1}),
        json!({"path":"x","snapshot":"A".repeat(64)}),
    ] {
        assert!(Operation::parse(NAMES[3], args).is_err());
    }
    assert!(
        Operation::parse(
            NAMES[4],
            json!({"path":"x","text":"x".repeat(WRITE_TEXT_MAX_BYTES+1)})
        )
        .is_err()
    );
    assert!(
        Operation::parse(
            NAMES[4],
            json!({"path":"x","text":"x".repeat(WRITE_TEXT_MAX_BYTES)})
        )
        .is_ok()
    );
    Ok(())
}
#[test]
fn file_tools_native_unicode_and_platform_filename_boundaries() -> Result<()> {
    #[cfg(windows)]
    {
        for path in [
            "C:/escape",
            "C:escape",
            "\\\\server\\share",
            "a\\b",
            "x:stream",
            "NUL",
            "nul.txt",
            "NUL .txt",
            "COM¹.txt",
            "a.",
            "a ",
            "a?b",
            ".__OPEN_SEO_SPIDER_MCP_TMP_x",
        ] {
            assert!(RelativePath::parse(path.into(), false).is_err(), "{path:?}");
        }
        assert!(RelativePath::parse("漢".repeat(255), false).is_ok());
        assert!(RelativePath::parse("漢".repeat(256), false).is_err());
        assert!(RelativePath::parse("😀".repeat(127), false).is_ok());
        assert!(RelativePath::parse("😀".repeat(128), false).is_err());
    }
    #[cfg(unix)]
    {
        for path in [
            "legal.",
            "legal ",
            "a\\b",
            "a:b",
            "NUL",
            "a?b",
            "line\nname",
        ] {
            assert!(RelativePath::parse(path.into(), false).is_ok(), "{path:?}");
        }
        assert!(RelativePath::parse("é".repeat(127) + "a", false).is_ok());
        assert!(RelativePath::parse("é".repeat(128), false).is_err());
    }
    Ok(())
}
#[tokio::test]
async fn file_tools_existing_root_and_no_implicit_outside_creation() -> Result<()> {
    let fixture = Fixture::new()?;
    assert!(FileTools::open(&fixture.path.join("absent")).is_err());
    assert!(!fixture.path.join("absent").exists());
    std::fs::write(fixture.path.join("regular"), b"x")?;
    assert!(FileTools::open(&fixture.path.join("regular")).is_err());
    let tools = fixture.tools()?;
    let allowed = tools.call(NAMES[0], json!({})).await?;
    assert_eq!(allowed["allowed_directory"], ".");
    assert!(!serde_json::to_string(&allowed)?.contains(fixture.path.to_str().unwrap()));
    assert!(tools.call(NAMES[0], json!({"extra":true})).await.is_err());
    tools.shutdown_and_drain().await?;
    Ok(())
}
#[tokio::test]
async fn file_tools_recursive_create_long_unicode_and_partial_effects() -> Result<()> {
    let fixture = Fixture::new()?;
    let tools = fixture.tools()?;
    let long = ["漢".repeat(70), "é".repeat(90), "third".repeat(25)].join("/");
    let created = tools.call(NAMES[2], json!({"path":long})).await?;
    assert_eq!(created["created_components"].as_array().unwrap().len(), 3);
    let repeated = tools.call(NAMES[2], json!({"path":long})).await?;
    assert!(
        repeated["created_components"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    std::fs::write(fixture.path.join("blocker"), b"x")?;
    assert!(
        tools
            .call(NAMES[2], json!({"path":"new/../blocker/child"}))
            .await
            .is_err()
    );
    assert!(!fixture.path.join("new").exists()); // all path validation precedes any mkdir.
    tools.call(NAMES[2], json!({"path":"partial"})).await?;
    std::fs::write(fixture.path.join("partial/blocker"), b"x")?;
    assert!(
        tools
            .call(NAMES[2], json!({"path":"partial/blocker/child"}))
            .await
            .is_err()
    );
    assert!(fixture.path.join("partial").is_dir());
    tools.shutdown_and_drain().await?;
    Ok(())
}
#[tokio::test]
async fn file_tools_listing_has_exact_sorted_pages_and_stale_instance_anchor_checks() -> Result<()>
{
    let fixture = Fixture::new()?;
    for name in ["é", "A", "z"] {
        std::fs::write(fixture.path.join(name), name.as_bytes())?;
    }
    let tools = fixture.tools()?;
    let first = tools.call(NAMES[1], json!({"limit":1})).await?;
    assert_eq!(first["entries"][0]["name"], "A");
    assert_eq!(first["total_entries"], 3);
    let cursor = first["next_cursor"].as_str().unwrap();
    let rest = tools
        .call(NAMES[1], json!({"limit":200,"cursor":cursor}))
        .await?;
    assert_eq!(rest["entries"][0]["name"], "z");
    assert_eq!(rest["entries"][1]["name"], "é");
    assert!(rest["next_cursor"].is_null());
    let other = fixture.tools()?;
    assert!(
        other
            .call(NAMES[1], json!({"cursor":cursor}))
            .await
            .is_err()
    );
    let mut forged = ListCursor::decode(cursor)?;
    forged.after = "not-an-entry".into();
    assert!(
        tools
            .call(NAMES[1], json!({"cursor":forged.encode()?}))
            .await
            .is_err()
    );
    std::fs::write(fixture.path.join("new"), b"x")?;
    assert!(
        tools
            .call(NAMES[1], json!({"cursor":cursor}))
            .await
            .is_err()
    );
    tools.shutdown_and_drain().await?;
    other.shutdown_and_drain().await?;
    Ok(())
}
#[tokio::test]
async fn file_tools_listing_over_entry_budget_is_error_not_partial_page() -> Result<()> {
    let fixture = Fixture::new()?;
    for index in 0..=DIRECTORY_MAX_ENTRIES {
        std::fs::write(fixture.path.join(format!("f{index:05}")), b"")?;
    }
    let tools = fixture.tools()?;
    let error = tools.call(NAMES[1], json!({"limit":1})).await.unwrap_err();
    assert!(format!("{error:#}").contains("10000-entry"));
    tools.shutdown_and_drain().await?;
    Ok(())
}
#[tokio::test]
async fn file_tools_utf8_chunks_reconstruct_exact_bytes_and_detect_content_changes() -> Result<()> {
    let fixture = Fixture::new()?;
    let text = "aé😀\n漢字";
    std::fs::write(fixture.path.join("text"), text.as_bytes())?;
    let tools = fixture.tools()?;
    let first = tools
        .call(NAMES[3], json!({"path":"text","max_bytes":3}))
        .await?;
    assert_eq!(first["text"], "aé");
    assert_eq!(first["sha256"], digest(text.as_bytes()));
    let snapshot = first["snapshot"].as_str().unwrap().to_owned();
    let mut assembled = first["text"].as_str().unwrap().to_owned();
    let mut offset = first["next_offset"].as_u64().unwrap();
    loop {
        let page = tools
            .call(
                NAMES[3],
                json!({"path":"text","max_bytes":4,"offset":offset,"snapshot":snapshot}),
            )
            .await?;
        assembled.push_str(page["text"].as_str().unwrap());
        assert_eq!(page["snapshot"], snapshot);
        if page["eof"] == true {
            break;
        }
        offset = page["next_offset"].as_u64().unwrap();
    }
    assert_eq!(assembled.as_bytes(), text.as_bytes());
    assert!(
        tools
            .call(
                NAMES[3],
                json!({"path":"text","offset":2,"snapshot":snapshot})
            )
            .await
            .is_err()
    );
    assert!(
        tools
            .call(
                NAMES[3],
                json!({"path":"text","offset":3,"max_bytes":1,"snapshot":snapshot})
            )
            .await
            .is_err()
    );
    std::fs::write(fixture.path.join("text"), b"changed")?;
    assert!(
        tools
            .call(NAMES[3], json!({"path":"text","snapshot":snapshot}))
            .await
            .is_err()
    );
    tools.shutdown_and_drain().await?;
    Ok(())
}
#[tokio::test]
async fn file_tools_read_empty_invalid_utf8_and_sparse_overbudget_are_explicit() -> Result<()> {
    let fixture = Fixture::new()?;
    std::fs::write(fixture.path.join("empty"), b"")?;
    std::fs::write(fixture.path.join("binary"), [0xff, 0xfe])?;
    std::fs::File::create(fixture.path.join("large"))?.set_len(READ_FILE_MAX_BYTES as u64 + 1)?;
    let tools = fixture.tools()?;
    let empty = tools.call(NAMES[3], json!({"path":"empty"})).await?;
    assert_eq!(empty["text"], "");
    assert_eq!(empty["eof"], true);
    assert_eq!(empty["sha256"], digest(b""));
    assert!(
        tools
            .call(NAMES[3], json!({"path":"binary"}))
            .await
            .is_err()
    );
    assert!(tools.call(NAMES[3], json!({"path":"large"})).await.is_err());
    assert!(tools.call(NAMES[3], json!({"path":"."})).await.is_err());
    tools.shutdown_and_drain().await?;
    Ok(())
}
#[tokio::test]
async fn file_tools_atomic_replacement_keeps_hardlink_inode_and_cleans_own_stage() -> Result<()> {
    let fixture = Fixture::new()?;
    let outside = Fixture::new()?;
    std::fs::write(outside.path.join("original"), b"outside old")?;
    std::fs::hard_link(outside.path.join("original"), fixture.path.join("target"))?;
    let tools = fixture.tools()?;
    let value = tools
        .call(NAMES[4], json!({"path":"target","text":"replacement😀"}))
        .await?;
    assert_eq!(value["atomic_replace"], true);
    assert_eq!(value["directory_entry_crash_durability_proven"], false);
    assert_eq!(
        std::fs::read(outside.path.join("original"))?,
        b"outside old"
    );
    assert_eq!(
        std::fs::read(fixture.path.join("target"))?,
        "replacement😀".as_bytes()
    );
    tools
        .call(NAMES[4], json!({"path":"empty","text":""}))
        .await?;
    tools
        .call(
            NAMES[4],
            json!({"path":"boundary","text":"x".repeat(WRITE_TEXT_MAX_BYTES)}),
        )
        .await?;
    assert_eq!(
        std::fs::metadata(fixture.path.join("boundary"))?.len(),
        WRITE_TEXT_MAX_BYTES as u64
    );
    assert!(
        tools
            .call(
                NAMES[4],
                json!({"path":"boundary","text":"x".repeat(WRITE_TEXT_MAX_BYTES+1)})
            )
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::metadata(fixture.path.join("boundary"))?.len(),
        WRITE_TEXT_MAX_BYTES as u64
    );
    assert!(std::fs::read_dir(&fixture.path)?.all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(STAGING_PREFIX)
    }));
    tools.shutdown_and_drain().await?;
    Ok(())
}
#[tokio::test]
async fn file_tools_stage_guard_cleans_owned_failure_but_preserves_replaced_entry() -> Result<()> {
    let fixture = Fixture::new()?;
    let root = open_root(&fixture.path)?;
    let own_name = format!("{STAGING_PREFIX}owned");
    std::fs::write(fixture.path.join(&own_name), b"own")?;
    let file = open_regular(&root, &own_name)?;
    let metadata = file.metadata()?;
    let mut guard = Staged {
        parent: &root,
        name: own_name.clone(),
        identity: (metadata.dev(), metadata.ino()),
        committed: false,
    };
    drop(file);
    guard.cleanup()?;
    assert!(!fixture.path.join(&own_name).exists());
    std::fs::write(fixture.path.join(&own_name), b"first")?;
    let file = open_regular(&root, &own_name)?;
    let metadata = file.metadata()?;
    let mut guard = Staged {
        parent: &root,
        name: own_name.clone(),
        identity: (metadata.dev(), metadata.ino()),
        committed: false,
    };
    // Hold old inode while replacing its directory entry, preventing inode reuse.
    std::fs::write(fixture.path.join("foreign"), b"foreign")?;
    std::fs::rename(fixture.path.join("foreign"), fixture.path.join(&own_name))?;
    assert!(guard.cleanup().is_err());
    assert_eq!(std::fs::read(fixture.path.join(&own_name))?, b"foreign");
    guard.committed = true;
    drop(file);
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn file_tools_symlink_intermediate_final_and_root_are_never_followed() -> Result<()> {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new()?;
    let outside = Fixture::new()?;
    std::fs::write(outside.path.join("secret"), b"secret")?;
    symlink(&outside.path, fixture.path.join("link"))?;
    symlink(outside.path.join("secret"), fixture.path.join("filelink"))?;
    assert!(FileTools::open(&fixture.path.join("link")).is_err());
    let tools = fixture.tools()?;
    for (name, args) in [
        (NAMES[3], json!({"path":"link/secret"})),
        (NAMES[4], json!({"path":"link/secret","text":"overwrite"})),
        (NAMES[3], json!({"path":"filelink"})),
        (NAMES[4], json!({"path":"filelink","text":"overwrite"})),
        (NAMES[2], json!({"path":"link/new"})),
    ] {
        assert!(tools.call(name, args).await.is_err());
    }
    let listed = tools.call(NAMES[1], json!({})).await?;
    assert!(
        listed["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["kind"] == "symlink")
    );
    assert_eq!(std::fs::read(outside.path.join("secret"))?, b"secret");
    assert!(!outside.path.join("new").exists());
    tools.shutdown_and_drain().await?;
    Ok(())
}
#[cfg(unix)]
#[tokio::test]
async fn file_tools_unix_fifo_is_rejected_before_read_and_invalid_names_are_lossless_errors()
-> Result<()> {
    use std::os::unix::ffi::OsStringExt;
    let fixture = Fixture::new()?;
    // Explicit POSIX fixture dependency; missing mkfifo is a test error, never a skip/PASS.
    ensure!(
        std::process::Command::new("mkfifo")
            .arg(fixture.path.join("fifo"))
            .status()?
            .success(),
        "mkfifo fixture failed"
    );
    let tools = fixture.tools()?;
    let error = tools
        .call(NAMES[3], json!({"path":"fifo"}))
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("regular"));
    assert!(
        tools
            .call(NAMES[4], json!({"path":"fifo","text":"x"}))
            .await
            .is_err()
    );
    std::fs::write(
        fixture.path.join(std::ffi::OsString::from_vec(vec![0xff])),
        b"x",
    )?;
    assert!(tools.call(NAMES[1], json!({})).await.is_err());
    tools.shutdown_and_drain().await?;
    Ok(())
}
#[tokio::test]
async fn file_tools_root_handle_cannot_be_rebound_by_replacing_configured_path() -> Result<()> {
    let fixture = Fixture::new()?;
    let tools = fixture.tools()?;
    let moved = fixture.path.with_extension("moved");
    match std::fs::rename(&fixture.path, &moved) {
        Ok(()) => {
            std::fs::create_dir(&fixture.path)?;
            tools
                .call(NAMES[4], json!({"path":"actual","text":"held capability"}))
                .await?;
            assert!(!fixture.path.join("actual").exists());
            assert_eq!(std::fs::read(moved.join("actual"))?, b"held capability");
            tools.shutdown_and_drain().await?;
            drop(tools);
            std::fs::remove_dir_all(&moved)?;
        }
        Err(error) => {
            #[cfg(not(windows))]
            return Err(error.into());
            #[cfg(windows)]
            {
                // Windows cap directories can deny rename through FILE_SHARE_DELETE policy.
                assert!(matches!(
                    error.kind(),
                    io::ErrorKind::PermissionDenied | io::ErrorKind::Other
                ));
                assert!(fixture.path.is_dir());
                tools
                    .call(NAMES[4], json!({"path":"actual","text":"held capability"}))
                    .await?;
                assert_eq!(
                    std::fs::read(fixture.path.join("actual"))?,
                    b"held capability"
                );
                tools.shutdown_and_drain().await?;
            }
        }
    }
    Ok(())
}

fn observed_drain(tools: FileTools) -> (JoinHandle<Result<()>>, oneshot::Receiver<()>) {
    let (waiting_tx, waiting_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        let drain = tools.shutdown_and_drain();
        tokio::pin!(drain);
        let mut waiting_tx = Some(waiting_tx);
        std::future::poll_fn(|cx| {
            let result = std::future::Future::poll(drain.as_mut(), cx);
            if result.is_pending() && lock(&tools.inner.registry).closing {
                // The real drain has closed admission and reached an await that
                // is pending while the actual blocking worker remains blocked.
                if let Some(waiting_tx) = waiting_tx.take() {
                    let _ = waiting_tx.send(());
                }
            }
            result
        })
        .await
    });
    (handle, waiting_rx)
}

async fn assert_observed_drain_pending(
    tools: &FileTools,
    waiting: oneshot::Receiver<()>,
    drain: &mut JoinHandle<Result<()>>,
) -> Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(5), waiting)
        .await
        .context("Drain did not reach a pending wait after closing admission")?
        .context("Drain ended without announcing its actual pending wait")?;
    assert!(lock(&tools.inner.registry).closing);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), &mut *drain)
            .await
            .is_err(),
        "Drain finished before the blocked worker was released"
    );
    Ok(())
}

async fn assert_actual_join_slots_consumed(job: &Job) -> Result<()> {
    let worker = job.worker.lock().await;
    assert!(
        worker.join.is_none(),
        "Actual BlockingJoin was not consumed"
    );
    assert!(
        worker.reply.is_none(),
        "Worker response sender was retained"
    );
    drop(worker);
    let supervisor = lock(&job.supervisor)
        .clone()
        .context("Registered supervisor slot missing")?;
    assert!(
        supervisor.join.lock().await.is_none(),
        "Actual SupervisorJoin was not consumed"
    );
    assert!(job.completed.load(Ordering::Acquire));
    Ok(())
}

#[tokio::test]
async fn file_tools_dropped_request_and_concurrent_drain_wait_actual_worker_return() -> Result<()> {
    let fixture = Fixture::new()?;
    let tools = fixture.tools()?;
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let returned = Arc::new(AtomicUsize::new(0));
    let marked = returned.clone();
    let caller_tools = tools.clone();
    let caller = tokio::spawn(async move {
        caller_tools
            .run(move || {
                let _ = entered_tx.send(());
                release_rx.recv()?;
                marked.store(1, AtomicOrdering::Release);
                Ok(json!({"done":true}))
            })
            .await
    });
    entered_rx.await?;
    let job = lock(&tools.inner.registry)
        .jobs
        .values()
        .next()
        .cloned()
        .context("Entered worker was not registered")?;
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    let (mut drain_a, waiting_a) = observed_drain(tools.clone());
    let (mut drain_b, waiting_b) = observed_drain(tools.clone());
    assert_observed_drain_pending(&tools, waiting_a, &mut drain_a).await?;
    assert_observed_drain_pending(&tools, waiting_b, &mut drain_b).await?;
    let waited_for_worker = !drain_a.is_finished() && !drain_b.is_finished();
    let rejected_new = tools.call(NAMES[0], json!({})).await.is_err();
    release_tx.send(())?;
    drain_a.await??;
    drain_b.await??;
    assert!(waited_for_worker && rejected_new);
    assert_eq!(returned.load(AtomicOrdering::Acquire), 1);
    assert_actual_join_slots_consumed(&job).await?;
    assert!(lock(&tools.inner.registry).jobs.is_empty());
    Ok(())
}
#[tokio::test]
async fn file_tools_cancelled_drain_does_not_lose_join_handle() -> Result<()> {
    let fixture = Fixture::new()?;
    let tools = fixture.tools()?;
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let caller_tools = tools.clone();
    let caller = tokio::spawn(async move {
        caller_tools
            .run(move || {
                let _ = entered_tx.send(());
                release_rx.recv()?;
                Ok(json!({}))
            })
            .await
    });
    entered_rx.await?;
    let job = lock(&tools.inner.registry)
        .jobs
        .values()
        .next()
        .cloned()
        .context("Entered worker was not registered")?;
    let (mut drain, waiting) = observed_drain(tools.clone());
    assert_observed_drain_pending(&tools, waiting, &mut drain).await?;
    drain.abort();
    assert!(drain.await.unwrap_err().is_cancelled());
    assert!(lock(&tools.inner.registry).closing);
    release_tx.send(())?;
    tools.shutdown_and_drain().await?;
    caller.await??;
    assert_actual_join_slots_consumed(&job).await?;
    assert!(lock(&tools.inner.registry).jobs.is_empty());
    Ok(())
}

#[tokio::test]
async fn file_tools_supervisor_abort_and_caller_drop_recover_actual_join_slots() -> Result<()> {
    let fixture = Fixture::new()?;
    let tools = fixture.tools()?;
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let caller_tools = tools.clone();
    let caller = tokio::spawn(async move {
        caller_tools
            .run(move || {
                let _ = entered_tx.send(());
                release_rx.recv()?;
                Ok(json!({"released":true}))
            })
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), entered_rx)
        .await
        .context("Blocking worker did not enter")??;
    let (id, job) = {
        let registry = lock(&tools.inner.registry);
        assert_eq!(registry.jobs.len(), 1);
        registry
            .jobs
            .iter()
            .next()
            .map(|(id, job)| (*id, job.clone()))
            .context("Entered worker was not registered")?
    };
    let supervisor = lock(&job.supervisor)
        .clone()
        .context("Registered supervisor slot missing")?;
    {
        let supervisor_join = supervisor.join.lock().await;
        supervisor_join
            .as_ref()
            .context("Actual supervisor handle missing before abort")?
            .abort();
    }
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    {
        // The canceled monitor/caller release this mutex without taking the
        // actual BlockingJoin. Its closure still cannot return before release.
        let worker = tokio::time::timeout(std::time::Duration::from_secs(5), job.worker.lock())
            .await
            .context("Canceled waiters did not release the worker slot")?;
        assert!(worker.join.is_some());
        assert!(!job.completed.load(Ordering::Acquire));
    }
    assert!(supervisor.join.lock().await.is_some());
    let (mut drain, waiting) = observed_drain(tools.clone());
    assert_observed_drain_pending(&tools, waiting, &mut drain).await?;
    assert!(
        lock(&tools.inner.registry)
            .jobs
            .get(&id)
            .is_some_and(|registered| Arc::ptr_eq(registered, &job))
    );
    release_tx.send(())?;
    let drained = tokio::time::timeout(std::time::Duration::from_secs(5), &mut drain)
        .await
        .context("Drain did not join released worker and canceled supervisor")?
        .context("Observed drain task itself failed")?;
    let error = drained.expect_err("Canceled supervisor join must be reported");
    assert!(
        format!("{error:#}").contains("Filesystem supervisor failed to join"),
        "Drain did not report the actual supervisor JoinError: {error:#}"
    );
    assert_actual_join_slots_consumed(&job).await?;
    assert!(lock(&tools.inner.registry).jobs.is_empty());
    assert!(
        lock(&tools.inner.registry)
            .first_join_failure
            .as_deref()
            .is_some_and(|message| message.contains("Filesystem supervisor failed to join"))
    );
    assert!(tools.shutdown_and_drain().await.is_err());
    Ok(())
}
#[tokio::test]
async fn file_tools_four_workers_bound_and_panics_are_joined_and_reported() -> Result<()> {
    let fixture = Fixture::new()?;
    let tools = fixture.tools()?;
    let (entered_tx, mut entered_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut releases = Vec::new();
    let mut callers = Vec::new();
    for _ in 0..MAX_CONCURRENT_FILE_OPS {
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        releases.push(release_tx);
        let tools = tools.clone();
        let entered_tx = entered_tx.clone();
        callers.push(tokio::spawn(async move {
            tools
                .run(move || {
                    entered_tx.send(())?;
                    release_rx.recv()?;
                    Ok(json!({}))
                })
                .await
        }));
    }
    for _ in 0..MAX_CONCURRENT_FILE_OPS {
        entered_rx.recv().await.context("worker did not enter")?;
    }
    assert!(tools.run(|| Ok(json!({}))).await.is_err());
    for release in releases {
        release.send(())?;
    }
    for caller in callers {
        caller.await??;
    }
    assert!(
        tools
            .run(|| -> Result<Value> { panic!("directed filesystem worker panic") })
            .await
            .is_err()
    );
    assert!(tools.shutdown_and_drain().await.is_err());
    assert!(lock(&tools.inner.registry).jobs.is_empty());
    Ok(())
}
