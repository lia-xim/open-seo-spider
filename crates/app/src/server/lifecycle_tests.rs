use super::*;
use tokio::{
    io::AsyncWriteExt,
    time::{Duration, timeout},
};

#[tokio::test]
async fn owned_eof_reader_does_not_finish_on_nonempty_input_and_closes_only_at_actual_eof() {
    let (mut writer, reader) = tokio::io::duplex(16384);
    let mut monitor = tokio::spawn(wait_for_eof(reader));
    writer.write_all(&vec![b'x'; 12288]).await.unwrap();
    assert!(
        timeout(Duration::from_millis(30), &mut monitor)
            .await
            .is_err()
    );
    writer.shutdown().await.unwrap();
    timeout(Duration::from_secs(5), monitor)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn one_global_lifecycle_admission_closes_request_blocking_and_crawl_registries() {
    let lifecycle = Lifecycle::default();
    lifecycle.admission.close();
    assert!(lifecycle.requests.spawn_async(async {}).is_err());
    assert!(lifecycle.crawls.spawn_async(async {}).is_err());
    assert!(lifecycle.blocking.spawn_blocking(|| ()).is_err());
    let (requests, blocking, crawls) = tokio::join!(
        lifecycle.requests.shutdown_and_drain(),
        lifecycle.blocking.shutdown_and_drain(),
        lifecycle.crawls.shutdown_and_drain()
    );
    requests.unwrap();
    blocking.unwrap();
    crawls.unwrap();
}

async fn start_test_server(
    data: PathBuf,
) -> (
    String,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<anyhow::Result<()>>,
) {
    start_test_server_mode(data, false).await
}
async fn start_test_server_mode(
    data: PathBuf,
    signal_fails: bool,
) -> (
    String,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<anyhow::Result<()>>,
) {
    let (ready, address) = tokio::sync::oneshot::channel();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        serve_with_mcp_files_ready_source(
            "127.0.0.1:0",
            data,
            None,
            Some(ready),
            ShutdownSource::Injected(Box::pin(async move {
                stopped.await?;
                if signal_fails {
                    anyhow::bail!("Injected shutdown signal failure");
                }
                Ok("test_owned_eof")
            })),
        )
        .await
    });
    let address = timeout(Duration::from_secs(5), address)
        .await
        .unwrap()
        .unwrap();
    (format!("http://{address}"), stop, server)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn owned_shutdown_stops_actual_http_listener_after_exact_legacy_inventory() {
    let temp = tempfile::tempdir().unwrap();
    let (base, stop, server) = start_test_server(temp.path().join("data")).await;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let response = client
        .post(format!("{base}/mcp"))
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let inventory: Value = response.json().await.unwrap();
    let names: Vec<_> = inventory["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 28);
    assert_eq!(
        &names[..26],
        &[
            "spider_capabilities",
            "spider_list_crawls",
            "spider_start_crawl",
            "spider_status",
            "spider_pages",
            "spider_issues",
            "spider_analysis",
            "spider_policy",
            "spider_sitemaps",
            "spider_resources",
            "spider_resource_transports",
            "spider_inspect_css",
            "spider_content",
            "spider_content_document",
            "spider_analyze_crawl_content",
            "spider_analyze_content",
            "spider_preview_policy",
            "spider_cancel",
            "spider_pause",
            "spider_resume",
            "spider_recrawl",
            "spider_project",
            "spider_compare",
            "spider_generate_sitemaps",
            "spider_compare_saved",
            "spider_sitemap_findings",
        ]
    );
    assert_eq!(names[26], "spider_api_enrichment");
    assert_eq!(names[27], "spider_segments");
    assert!(names.contains(&"spider_sitemap_findings"));
    assert!(
        !names
            .iter()
            .any(|name| name.contains("shutdown") || name.starts_with("spider_write_"))
    );
    stop.send(()).unwrap();
    timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(client.get(format!("{base}/healthz")).send().await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn owned_shutdown_pauses_actual_crawl_and_waits_terminal_checkpoint_and_job_file() {
    actual_crawl_shutdown_waits_for_checkpoint(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn failed_shutdown_signal_is_returned_only_after_actual_crawl_checkpoint_and_job_write() {
    actual_crawl_shutdown_waits_for_checkpoint(true).await;
}

async fn actual_crawl_shutdown_waits_for_checkpoint(signal_fails: bool) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/held", listener.local_addr().unwrap());
    let (entered, observed) = tokio::sync::oneshot::channel();
    let (release, held) = tokio::sync::oneshot::channel();
    let website = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        while !request.windows(4).any(|v| v == b"\r\n\r\n") {
            let n = socket.read(&mut buffer).await.unwrap();
            assert!(n > 0);
            request.extend_from_slice(&buffer[..n]);
        }
        entered.send(()).unwrap();
        let _ = held.await;
        let body = b"<!doctype html><html><head><title>Owned</title></head><body>Actual held HTTP page</body></html>";
        let header = format!(
            "HTTP/1.1 200 Fixture\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        socket.write_all(header.as_bytes()).await.unwrap();
        socket.write_all(body).await.unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    let (base, stop, mut server) = start_test_server_mode(data.clone(), signal_fails).await;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let response = client.post(format!("{base}/api/crawls")).json(&json!({"seed_urls":[url],"obey_robots":false,"discover_sitemaps":false,"crawl_links":false,"concurrency":1,"delay_ms":0,"request_timeout_secs":5})).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let started: Value = response.json().await.unwrap();
    let id = started["id"].as_str().unwrap();
    timeout(Duration::from_secs(5), observed)
        .await
        .unwrap()
        .unwrap();
    stop.send(()).unwrap();
    assert!(
        timeout(Duration::from_millis(100), &mut server)
            .await
            .is_err(),
        "Process success cannot precede completion of the admitted page"
    );
    assert!(
        !CrawlStore::inspect(data.join(format!("{id}.sqlite")))
            .unwrap()
            .checkpoint_valid
    );
    release.send(()).unwrap();
    let result = timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap();
    if signal_fails {
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Injected shutdown signal failure")
        );
    } else {
        result.unwrap();
    }
    website.await.unwrap();
    let inspection = CrawlStore::inspect(data.join(format!("{id}.sqlite"))).unwrap();
    assert!(inspection.checkpoint_valid);
    assert_eq!(inspection.completed, 1);
    assert_eq!(inspection.pending, 0);
    let job: Value = serde_json::from_slice(
        &tokio::fs::read(data.join(format!("{id}.job.json")))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(job["status"], "paused");
    assert!(job["finished_at"].is_string());
    assert!(job["checkpoint"].is_object());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shutdown_before_job_insert_keeps_admitted_initialization_owned_and_late_launch_paused() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    tokio::fs::create_dir(&data).await.unwrap();
    let lifecycle = Lifecycle::default();
    let mut mcp = crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap();
    mcp.set_task_owner(lifecycle.blocking.clone());
    let state = AppState {
        mcp,
        jobs: Arc::new(RwLock::new(HashMap::new())),
        data_dir: Arc::new(data.clone()),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle,
    };
    let website = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/must-not-start", website.local_addr().unwrap());
    let value = json!({"seed_urls":[url],"obey_robots":false,"discover_sitemaps":false,"crawl_links":false,"delay_ms":0,"request_timeout_secs":1});
    let (entered, observed) = tokio::sync::oneshot::channel();
    let (proceed, held) = tokio::sync::oneshot::channel();
    let admitted = state.clone();
    let preparing = state.clone();
    let start = tokio::spawn(async move {
        start_operation(admitted, async move {
            entered.send(()).unwrap();
            let _ = held.await;
            create_crawl_prepared(preparing, value).await
        })
        .await
    });
    timeout(Duration::from_secs(5), observed)
        .await
        .unwrap()
        .unwrap();
    assert!(state.jobs.read().await.is_empty());
    close_admission(&state);
    let draining = state.clone();
    let mut drained = tokio::spawn(async move { drain(&draining).await });
    assert!(
        timeout(Duration::from_millis(50), &mut drained)
            .await
            .is_err(),
        "Empty job map cannot hide an admitted pre-insert operation"
    );
    proceed.send(()).unwrap();
    let Json(started) = timeout(Duration::from_secs(5), start)
        .await
        .unwrap()
        .unwrap()
        .unwrap_or_else(|(status, Json(error))| {
            panic!(
                "Admitted crawl initialization failed with {status}: {}",
                error.error
            )
        });
    timeout(Duration::from_secs(5), drained)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let id = started["id"].as_str().unwrap();
    let inspection = CrawlStore::inspect(data.join(format!("{id}.sqlite"))).unwrap();
    assert!(inspection.checkpoint_valid);
    assert_eq!(inspection.completed, 0);
    assert_eq!(inspection.pending, 1);
    let manifest: Value = serde_json::from_slice(
        &tokio::fs::read(data.join(format!("{id}.job.json")))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["status"], "paused");
    assert!(
        timeout(Duration::from_millis(30), website.accept())
            .await
            .is_err(),
        "Late paused launch must make no website connection"
    );
    let stats = state.lifecycle.crawls.snapshot();
    assert_eq!((stats.admitted, stats.joined, stats.retained), (1, 1, 0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn typed_owned_core_failure_is_sticky_in_app_drain_but_domain_error_is_not() {
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
    let temp = tempfile::tempdir().unwrap();
    let failed = state(temp.path().join("failed"));
    let worker = open_seo_spider_core::TaskOwner::new("actual_pdf_join_failure_source");
    let error = worker
        .run_blocking(|| -> () {
            panic!("Actual retained blocking worker panic");
        })
        .await
        .unwrap_err();
    let error =
        anyhow::Error::new(error).context("Core PDF run propagated this concrete join failure");
    record_owned_crawl_failure(&failed, &error);
    assert!(
        failed
            .lifecycle
            .crawls
            .snapshot()
            .first_failure
            .as_ref()
            .unwrap()
            .contains("Core PDF run")
    );
    assert!(
        drain(&failed).await.is_err(),
        "App cannot declare a clean drain after a typed owned Core failure"
    );
    assert_eq!(worker.snapshot().joined, 1);
    assert_eq!(worker.snapshot().retained, 0);
    assert!(worker.shutdown_and_drain().await.is_err());
    let domain = state(temp.path().join("domain"));
    record_owned_crawl_failure(
        &domain,
        &anyhow::anyhow!("Ordinary request or crawl domain failure"),
    );
    assert!(domain.lifecycle.crawls.snapshot().first_failure.is_none());
    drain(&domain).await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn policy_and_resource_http_exports_start_eagerly_and_stream_beyond_channel_capacity() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let mut html =
        String::from("<!doctype html><html><head><title>Owned export</title></head><body>");
    for i in 0..700 {
        html.push_str(&format!(
            "<a href=\"/denied/{i:04}?padding={}\">owned {i}</a>",
            "x".repeat(1024)
        ));
    }
    html.push_str("</body></html>");
    let requests = Arc::new(AtomicUsize::new(0));
    let observed = requests.clone();
    let (stop_website, mut website_stopped) = tokio::sync::oneshot::channel();
    let website = tokio::spawn(async move {
        loop {
            let socket = tokio::select! {result=listener.accept()=>result.unwrap().0,_=&mut website_stopped=>break};
            let mut socket = socket;
            observed.fetch_add(1, Ordering::SeqCst);
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            while !request.windows(4).any(|v| v == b"\r\n\r\n") {
                let n = socket.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buffer[..n]);
            }
            assert!(
                request.starts_with(b"GET /seed "),
                "Policy-denied exports fixture reference must never be fetched"
            );
            socket.write_all(format!("HTTP/1.1 200 Fixture\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",html.len()).as_bytes()).await.unwrap();
            socket.write_all(html.as_bytes()).await.unwrap();
        }
    });
    let temp = tempfile::tempdir().unwrap();
    let (base, stop, server) = start_test_server(temp.path().join("data")).await;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let response=client.post(format!("{base}/api/crawls")).json(&json!({"seed_urls":[format!("{origin}/seed")],"obey_robots":false,"discover_sitemaps":false,"crawl_links":true,"concurrency":1,"delay_ms":0,"request_timeout_secs":5,
        "crawl_policy":{"scope":{"roots":[{"origin":origin,"folders":["/"]}]},"exclude_patterns":["/denied/"]},"resource_discovery":{}})).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let started: Value = response.json().await.unwrap();
    let id = started["id"].as_str().unwrap();
    timeout(Duration::from_secs(30), async {
        loop {
            let job: Value = client
                .get(format!("{base}/api/crawls/{id}"))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            if !matches!(job["status"].as_str(), Some("running" | "pausing")) {
                assert_eq!(job["status"], "completed", "{job}");
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    for ledger in ["policy", "resources"] {
        let mut expected_count = None;
        for format in ["json", "csv"] {
            let response = client
                .get(format!("{base}/api/crawls/{id}/{ledger}.{format}"))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let header = if ledger == "policy" {
                "x-policy-snapshot-occurrences"
            } else {
                "x-resource-snapshot-observations"
            };
            let rows: usize = response.headers()[header]
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            assert!(rows >= 700);
            if let Some(old) = expected_count {
                assert_eq!(rows, old);
            } else {
                expected_count = Some(rows);
            }
            let bytes = response.bytes().await.unwrap();
            assert!(
                bytes.len() > 8 * 65536,
                "Actual {ledger}.{format} must exceed the unchanged entire bounded producer queue"
            );
            if format == "json" {
                let value: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(value["observations"].as_array().unwrap().len(), rows);
                assert_eq!(value["export"]["row_count"], json!(rows));
                assert_eq!(value["export"]["all_snapshot_rows_exported"], true);
            } else {
                let text = std::str::from_utf8(&bytes).unwrap();
                assert_eq!(
                    text.lines().count(),
                    rows + 1,
                    "Controlled fixture fields have no embedded newlines; every CSV row must reach the client"
                );
                assert!(text.ends_with('\n'));
                assert!(text.contains("padding="));
            }
        }
    }
    stop.send(()).unwrap();
    timeout(Duration::from_secs(10), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(
        requests.load(Ordering::SeqCst),
        1,
        "Only the owned seed was fetched; all subsequent API downloads are read-only"
    );
    stop_website.send(()).unwrap();
    website.await.unwrap();
}
