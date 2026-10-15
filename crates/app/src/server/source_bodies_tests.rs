//! Authored real local HTTP source/API tests; NOT_COMPILED / NOT_RUN.
use super::*;
#[test]
fn source_body_api_config_absent_null_defaults_disable_and_strict_limits() {
    let base = json!({"seed_url":"https://example.test/"});
    let absent = config_from_input(base.clone()).unwrap();
    let mut null = base.clone();
    null["sitemap_source_archive"] = Value::Null;
    let null = config_from_input(null).unwrap();
    assert_eq!(
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&null).unwrap()
    );
    assert!(
        serde_json::to_value(&absent)
            .unwrap()
            .get("sitemap_source_archive")
            .is_none()
    );
    let mut enabled = base.clone();
    enabled["sitemap_source_archive"] = json!({});
    let config = config_from_input(enabled.clone()).unwrap();
    assert_eq!(config.sitemap_source_archive, Some(Default::default()));
    enabled
        .as_object_mut()
        .unwrap()
        .remove("sitemap_source_archive");
    assert!(
        config_from_input(enabled)
            .unwrap()
            .sitemap_source_archive
            .is_none()
    );
    for value in [
        json!([]),
        json!(false),
        json!({"extra":1}),
        json!({"max_bytes_per_body":67108865u64}),
        json!({"max_total_body_bytes":8589934593u64}),
        json!({"max_capture_records":100001}),
        json!({"retain_decoded_xml":false,"retain_post_http_entity":false}),
    ] {
        let mut invalid = base.clone();
        invalid["sitemap_source_archive"] = value;
        assert!(config_from_input(invalid).is_err());
    }
}
#[test]
fn source_body_metadata_json_and_cursor_keep_independent_bounds_and_job_binding() {
    assert!(json_response(&json!({"data":"x".repeat(JSON_CAP)})).is_err());
    let id = Uuid::new_v4();
    let context = SitemapBodyReadContext {
        generation: 1,
        config_sha256: "a".repeat(64),
        snapshot_sha256: "b".repeat(64),
    };
    let encoded = encode(&Snapshot {
        version: 1,
        job: id,
        context: context.clone(),
        document_id: None,
    })
    .unwrap();
    assert_eq!(snapshot(&encoded, id).unwrap().context, context);
    assert!(snapshot(&encoded, Uuid::new_v4()).is_err());
    assert!(decode::<Snapshot>(&"x".repeat(2049)).is_err());
}
fn state(data: PathBuf) -> AppState {
    AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Arc::new(RwLock::new(HashMap::new())),
        data_dir: Arc::new(data),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle: Default::default(),
    }
}
async fn api(
    state: AppState,
) -> (
    String,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let router = Router::new()
        .route("/api/crawls", post(create_crawl))
        .route("/api/crawls/{id}/sitemaps/source-bodies", get(inventory))
        .route(
            "/api/crawls/{id}/sitemaps/source-bodies/{body_id}",
            get(download).head(unsupported_head),
        )
        .with_state(state);
    let task = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    (origin, stop, task)
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn source_body_http_original_bytes_reopen_and_corruption_keep_descriptor_availability_separate()
 {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let temp = tempfile::tempdir().unwrap();
    let original="\u{feff}<?xml version='1.0'?>\r\n<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><!-- original é 💡 &amp; bytes --></urlset>".as_bytes().to_vec();
    let fixture_bytes = original.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let source = format!("http://{}/sitemap.xml", listener.local_addr().unwrap());
    let (stop_source, mut source_stopped) = tokio::sync::oneshot::channel();
    let requests = Arc::new(AtomicUsize::new(0));
    let observed = requests.clone();
    let website = tokio::spawn(async move {
        loop {
            let (mut socket, _) = tokio::select! {value=listener.accept()=>value.unwrap(),_=&mut source_stopped=>break};
            let mut request = [0u8; 8192];
            let n = socket.read(&mut request).await.unwrap();
            assert!(
                std::str::from_utf8(&request[..n])
                    .unwrap()
                    .starts_with("GET /sitemap.xml ")
            );
            observed.fetch_add(1, Ordering::SeqCst);
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",fixture_bytes.len()).as_bytes()).await.unwrap();
            socket.write_all(&fixture_bytes).await.unwrap();
        }
    });
    let first = state(temp.path().to_owned());
    let (origin, stop, server) = api(first.clone()).await;
    let client = reqwest::Client::new();
    let created:Value=client.post(format!("{origin}/api/crawls")).json(&json!({"seed_urls":[],"sitemap_urls":[source],"obey_robots":false,"discover_sitemaps":false,"crawl_links":false,"max_pages":1,"delay_ms":0,"sitemap_source_archive":{}})).send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
    let id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            let job = get_job(&first, id)
                .await
                .unwrap_or_else(|(status, Json(error))| {
                    panic!(
                        "Source body fixture job lookup failed with {status}: {}",
                        error.error
                    )
                });
            let guard = job.read().await;
            if guard.status == "completed" {
                break;
            }
            assert!(
                !matches!(guard.status.as_str(), "failed" | "cancelled"),
                "{:?}",
                guard.error
            );
            drop(guard);
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let metadata: Value = client
        .get(format!("{origin}/api/crawls/{id}/sitemaps/source-bodies"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(metadata["byte_availability"], "not_checked");
    assert!(metadata["download_available"].is_null());
    let body_id = metadata["window"]["captures"][0]["bodies"][0]["body_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let token = metadata["snapshot"].as_str().unwrap();
    let url = format!("{origin}/api/crawls/{id}/sitemaps/source-bodies/{body_id}?snapshot={token}");
    let response = client.get(&url).send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response.headers()["content-length"]
            .to_str()
            .unwrap()
            .parse::<usize>()
            .unwrap(),
        original.len()
    );
    assert_eq!(response.bytes().await.unwrap().as_ref(), original);
    let baseline = requests.load(Ordering::SeqCst);
    stop.send(()).unwrap();
    server.await.unwrap();
    lifecycle::close_admission(&first);
    lifecycle::drain(&first).await.unwrap();
    drop(first);
    let cold = state(temp.path().to_owned());
    load_reports(&cold).await.unwrap();
    let (origin, stop, server) = api(cold.clone()).await;
    let url = format!("{origin}/api/crawls/{id}/sitemaps/source-bodies/{body_id}?snapshot={token}");
    assert_eq!(
        client
            .get(&url)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .await
            .unwrap()
            .as_ref(),
        original
    );
    std::fs::write(
        temp.path()
            .join(format!("{id}.sqlite.sitemap-bodies"))
            .join(format!("{body_id}.bin")),
        b"corrupt retained object",
    )
    .unwrap();
    let metadata: Value = client
        .get(format!("{origin}/api/crawls/{id}/sitemaps/source-bodies"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        metadata["window"]["captures"][0]["bodies"][0]["body_id"],
        body_id
    );
    assert!(metadata["download_available"].is_null());
    let error = client.get(&url).send().await.unwrap();
    assert_eq!(error.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        error.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("application/json")
    );
    assert_eq!(
        requests.load(Ordering::SeqCst),
        baseline,
        "offline list/download/reopen must make no website requests"
    );
    assert_eq!(
        client.head(&url).send().await.unwrap().status(),
        reqwest::StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(
        client
            .get(&url)
            .header("Range", "bytes=0-10")
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::RANGE_NOT_SATISFIABLE
    );
    stop.send(()).unwrap();
    server.await.unwrap();
    lifecycle::close_admission(&cold);
    lifecycle::drain(&cold).await.unwrap();
    stop_source.send(()).unwrap();
    website.await.unwrap();
}

#[tokio::test]
async fn source_body_cursor_embedded_source_filter_cannot_change_when_outer_cursor_and_query_agree()
{
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path().to_owned());
    let job = Uuid::new_v4();
    let first = "a".repeat(64);
    let second = "b".repeat(64);
    let snapshot = encode(&Snapshot {
        version: 1,
        job,
        context: SitemapBodyReadContext {
            generation: 1,
            config_sha256: "c".repeat(64),
            snapshot_sha256: "d".repeat(64),
        },
        document_id: Some(first.clone()),
    })
    .unwrap();
    let modified = encode(&Cursor {
        snapshot: snapshot.clone(),
        after: 1,
        document_id: Some(second.clone()),
    })
    .unwrap();
    let error = inventory(
        State(state.clone()),
        Path(job),
        Ok(Query(ReadQuery {
            limit: Some(1),
            cursor: Some(modified),
            document_id: Some(second),
        })),
    )
    .await
    .err()
    .unwrap();
    assert_eq!(error.0, StatusCode::BAD_REQUEST);
    assert_eq!(error.1.0["error"], "invalid_source_body_cursor");
    let valid = encode(&Cursor {
        snapshot,
        after: 1,
        document_id: Some(first.clone()),
    })
    .unwrap();
    let error = inventory(
        State(state),
        Path(job),
        Ok(Query(ReadQuery {
            limit: Some(1),
            cursor: Some(valid),
            document_id: Some(first),
        })),
    )
    .await
    .err()
    .unwrap();
    assert_eq!(
        error.0,
        StatusCode::NOT_FOUND,
        "unchanged token/filter passes the boundary then reaches the missing-job check"
    );
    assert_eq!(error.1.0["error"], "job_not_found");
}
