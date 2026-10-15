use super::*;
use axum::body::to_bytes;
fn csv_cells(bytes: &[u8]) -> Vec<Vec<String>> {
    // Small independent RFC4180 decoder oracle, not a writer mirror.
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let mut chars = text.chars().peekable();
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                cell.push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                row.push(std::mem::take(&mut cell));
            }
            '\r' if !quoted => {
                assert_eq!(chars.next(), Some('\n'));
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            _ => cell.push(ch),
        }
    }
    assert!(!quoted);
    assert!(cell.is_empty() && row.is_empty());
    rows
}
fn job(id: Uuid, status: &str, url: &str) -> Job {
    Job {
        id,
        config: Default::default(),
        status: status.into(),
        started_at: String::new(),
        finished_at: None,
        pages: vec![],
        issues: vec![],
        queued: 0,
        discovery_pending: 0,
        in_flight: 0,
        warnings: vec![],
        error: None,
        report: Some(CrawlReport {
            config: Default::default(),
            pages: vec![PageRecord {
                url: url.into(),
                final_url: url.into(),
                status: Some(200),
                final_status: Some(200),
                indexable: true,
                internal: true,
                fetched_at: "2026-10-15T00:00:00Z".into(),
                ..Default::default()
            }],
            issues: vec![],
            summary: Default::default(),
            warnings: vec![],
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
            sitemap_catalog: None,
            sitemap_coverage_inventory: None,
            sitemap_source_measurements: None,
        }),
        control: CrawlControl::new(),
        generation: 7,
        checkpoint: None,
        frontier_available: false,
        operation: None,
        recrawl_override: Default::default(),
        policy_ledger: None,
        resource_ledger: None,
    }
}
fn state(j: Job) -> AppState {
    AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Arc::new(RwLock::new(HashMap::from([(
            j.id,
            Arc::new(RwLock::new(j)),
        )]))),
        data_dir: Arc::new(PathBuf::from("unused-directory-readonly-fixture")),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle: Default::default(),
    }
}
async fn value(r: Response) -> Value {
    serde_json::from_slice(
        &to_bytes(r.into_body(), DIRECTORY_TREE_JSON_BYTES)
            .await
            .unwrap(),
    )
    .unwrap()
}
fn failed<T>(r: DResult<T>) -> (StatusCode, Failure) {
    match r {
        Err((s, Json(e))) => (s, e),
        Ok(_) => panic!("Expected bounded refusal"),
    }
}
#[tokio::test]
async fn directory_api_reuses_unmodified_saved_authority_and_declares_imported_scope() {
    let id = Uuid::new_v4();
    let state = state(job(id, "running", "https://x.test/a/b"));
    let (status, e) =
        failed(preview(State(state.clone()), Path(id), Ok(Json(Default::default()))).await);
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(e.code, "unstable_snapshot");
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|(_, Json(e))| panic!("{}", e.error));
    {
        let mut j = source.write().await;
        j.status = "completed".into();
        j.generation = 0;
    }
    let first = value(
        preview(State(state.clone()), Path(id), Ok(Json(Default::default())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(first["snapshot"]["app_generation"], 0);
    assert_eq!(first["snapshot"]["sql_attested"], false);
    assert_eq!(first["snapshot"]["report_generation_attested"], false);
    assert_eq!(
        first["snapshot"]["caller_binding"]["origin"],
        "app_declared_imported_or_detached_report"
    );
    source.write().await.report = None;
    let (_, e) =
        failed(preview(State(state.clone()), Path(id), Ok(Json(Default::default()))).await);
    assert_eq!(e.code, "saved_report_unavailable");
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
#[tokio::test]
async fn directory_api_real_tree_navigation_keeps_synthetic_nodes_and_token_bound_paging() {
    let id = Uuid::new_v4();
    let mut j = job(id, "completed", "https://x.test/a/1");
    let report = j.report.as_mut().unwrap();
    let base = report.pages[0].clone();
    for n in 2..=205 {
        let mut p = base.clone();
        p.url = format!("https://x.test/a/{n}");
        p.final_url = p.url.clone();
        report.pages.push(p);
    }
    let state = state(j);
    let first = value(
        preview(State(state.clone()), Path(id), Ok(Json(Default::default())))
            .await
            .unwrap(),
    )
    .await;
    let mut r = TreeRequest {
        expected_generation: Some(7),
        expected_snapshot_token: Some(first["snapshot_token"].as_str().unwrap().into()),
        limit: 200,
        ..Default::default()
    };
    let mut page = first;
    for _ in 0..3 {
        r.parent_id = Some(page["children"][0]["id"].as_str().unwrap().into());
        page = value(
            preview(State(state.clone()), Path(id), Ok(Json(r.clone())))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(page["current"]["synthetic_group"], true);
    }
    assert_eq!(page["current"]["address"], "https://x.test/a/");
    assert_eq!(page["total_children"], 205);
    assert_eq!(page["children"].as_array().unwrap().len(), 200);
    assert_eq!(page["next_offset"], 200);
    r.offset = 200;
    let second = value(
        preview(State(state.clone()), Path(id), Ok(Json(r)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(second["children"].as_array().unwrap().len(), 5);
    assert!(second["next_offset"].is_null());
    assert!(
        second["children"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["synthetic_group"] == false)
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
#[tokio::test]
async fn directory_api_full_csv_binds_actual_length_sha_token_and_never_contacts_website() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/a/b", listener.local_addr().unwrap());
    let id = Uuid::new_v4();
    let state = state(job(id, "completed", &url));
    let first = value(
        preview(State(state.clone()), Path(id), Ok(Json(Default::default())))
            .await
            .unwrap(),
    )
    .await;
    let r = TreeRequest {
        expected_generation: Some(7),
        expected_snapshot_token: Some(first["snapshot_token"].as_str().unwrap().into()),
        ..Default::default()
    };
    let out = csv(State(state.clone()), Path(id), Ok(Json(r)))
        .await
        .unwrap();
    let length = out.headers()[header::CONTENT_LENGTH]
        .to_str()
        .unwrap()
        .parse::<usize>()
        .unwrap();
    let sha = out.headers()["x-content-sha256"]
        .to_str()
        .unwrap()
        .to_owned();
    let bytes = to_bytes(out.into_body(), DIRECTORY_TREE_CSV_BYTES)
        .await
        .unwrap();
    assert_eq!(bytes.len(), length);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), sha);
    let controls =
        csv_cells(b"one,two,three\r\n\"a,b\",\"quote \"\"x\"\"\",\"line\r\nbreak\"\r\n,,\r\n");
    assert_eq!(
        controls,
        vec![
            vec!["one", "two", "three"],
            vec!["a,b", "quote \"x\"", "line\r\nbreak"],
            vec!["", "", ""]
        ]
    );
    let parsed = csv_cells(&bytes);
    assert_eq!(parsed[0].len(), 23);
    assert_eq!(parsed[0][0], "kind");
    assert_eq!(parsed[0][1], "synthetic_group");
    let rows = &parsed[1..];
    assert_eq!(rows.len(), first["total_nodes"].as_u64().unwrap() as usize);
    assert_eq!(
        rows.iter()
            .filter(|r| r.get(1).map(String::as_str) == Some("false"))
            .count(),
        1
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
            .await
            .is_err(),
        "Read-only directory workflow contacted website"
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
#[tokio::test]
async fn directory_api_generation_report_config_and_status_mutation_stale_all_old_tokens() {
    let id = Uuid::new_v4();
    let state = state(job(id, "completed", "https://x.test/a"));
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|(_, Json(e))| panic!("{}", e.error));
    for field in 0..3 {
        let first = value(
            preview(State(state.clone()), Path(id), Ok(Json(Default::default())))
                .await
                .unwrap(),
        )
        .await;
        let r = TreeRequest {
            expected_snapshot_token: Some(first["snapshot_token"].as_str().unwrap().into()),
            ..Default::default()
        };
        {
            let mut j = source.write().await;
            match field {
                0 => j
                    .report
                    .as_mut()
                    .unwrap()
                    .warnings
                    .push("changed saved report".into()),
                1 => j.config.delay_ms += 1,
                _ => j.status = "paused".into(),
            }
        }
        let (s, e) = failed(csv(State(state.clone()), Path(id), Ok(Json(r))).await);
        assert_eq!(s, StatusCode::CONFLICT);
        assert_eq!(e.code, "stale_directory_snapshot");
    }
    let r = TreeRequest {
        expected_generation: Some(7),
        ..Default::default()
    };
    source.write().await.generation = 8;
    let (_, e) = failed(preview(State(state.clone()), Path(id), Ok(Json(r))).await);
    assert_eq!(e.code, "generation_conflict");
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
#[test]
fn directory_api_strict_requests_refuse_forged_paths_missing_snapshot_and_bad_windows() {
    assert!(serde_json::from_value::<TreeRequest>(json!({"path":"C:/private/file"})).is_err());
    assert!(serde_json::from_value::<TreeRequest>(json!({"pages":[]})).is_err());
    for r in [
        TreeRequest {
            limit: 0,
            ..Default::default()
        },
        TreeRequest {
            limit: 201,
            ..Default::default()
        },
        TreeRequest {
            parent_id: Some("a".repeat(64)),
            ..Default::default()
        },
        TreeRequest {
            offset: 1,
            ..Default::default()
        },
        TreeRequest {
            expected_snapshot_token: Some("x".repeat(65)),
            ..Default::default()
        },
    ] {
        assert!(r.validate().is_err());
    }
}
#[tokio::test]
async fn directory_api_real_oversized_saved_report_refuses_before_authority_clone_and_keeps_job() {
    let id = Uuid::new_v4();
    let mut j = job(id, "completed", "https://x.test/a");
    j.report
        .as_mut()
        .unwrap()
        .warnings
        .push("x".repeat(REPORT_BYTES + 1));
    let state = state(j);
    let (_, e) =
        failed(preview(State(state.clone()), Path(id), Ok(Json(Default::default()))).await);
    assert_eq!(e.code, "directory_report_budget");
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|(_, Json(e))| panic!("{}", e.error));
    let j = source.read().await;
    assert_eq!(j.generation, 7);
    assert_eq!(j.status, "completed");
    assert_eq!(
        j.report.as_ref().unwrap().warnings[0].len(),
        REPORT_BYTES + 1
    );
    drop(j);
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
