//! Actual Bridge/stdio/HTTP/CLI controls. Authored under HOLD; never run here.
use super::*;
use anyhow::{Context, ensure};
use clap::Parser;
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context as TaskContext, Poll},
};
use tokio::{
    io::{duplex, split},
    sync::oneshot,
    task::JoinHandle,
    time::{Duration, timeout},
};

const DEADLINE: Duration = Duration::from_secs(10);
const FILE_NAMES: [&str; 5] = [
    "spider_list_allowed_directory",
    "spider_list_directory",
    "spider_create_directory",
    "spider_read_text_file",
    "spider_write_text_file",
];

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("open-seo-mcp-wiring-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        // Only this fixture's freshly created UUID directory is owned.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn call(id: Value, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}})
}

async fn local(bridge: &Bridge, id: u64, name: &str, arguments: Value) -> Value {
    bridge
        .dispatch(call(json!(id), name, arguments))
        .await
        .unwrap()
}

async fn trap() -> (
    String,
    Arc<AtomicUsize>,
    oneshot::Sender<()>,
    JoinHandle<std::io::Result<()>>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let app = axum::Router::new().fallback(move || {
        let observed = observed.clone();
        async move {
            observed.fetch_add(1, Ordering::SeqCst);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Unexpected API request",
            )
        }
    });
    let (stop, stopped) = oneshot::channel();
    let worker = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
    });
    (base, count, stop, worker)
}

async fn close_trap(stop: oneshot::Sender<()>, worker: JoinHandle<std::io::Result<()>>) {
    let _ = stop.send(());
    join_owned(worker, "API trap").await.unwrap().unwrap();
}

async fn join_owned<T>(mut worker: JoinHandle<T>, label: &str) -> anyhow::Result<T> {
    match timeout(DEADLINE, &mut worker).await {
        Ok(joined) => Ok(joined.with_context(|| format!("{label} worker failed"))?),
        Err(_) => {
            worker.abort();
            // Do not discard a live JoinHandle when a directed timeout fails.
            let _ = worker.await;
            anyhow::bail!("{label} worker exceeded its directed test deadline")
        }
    }
}

#[test]
fn file_tools_cli_opt_in_is_shared_by_serve_and_mcp() {
    for command in ["serve", "mcp"] {
        let without = crate::Cli::try_parse_from(["open-seo-spider", command]).unwrap();
        let path = "explicit folder with spaces";
        let with =
            crate::Cli::try_parse_from(["open-seo-spider", command, "--mcp-files-dir", path])
                .unwrap();
        match (without.command.unwrap(), with.command.unwrap()) {
            (
                crate::Command::Serve {
                    mcp_files_dir: None,
                    ..
                },
                crate::Command::Serve {
                    mcp_files_dir: Some(actual),
                    ..
                },
            )
            | (
                crate::Command::Mcp {
                    mcp_files_dir: None,
                    ..
                },
                crate::Command::Mcp {
                    mcp_files_dir: Some(actual),
                    ..
                },
            ) => assert_eq!(actual, PathBuf::from(path)),
            _ => panic!("The flag must be opt-in for its actual command"),
        }
    }
    assert!(
        crate::Cli::try_parse_from(["open-seo-spider"])
            .unwrap()
            .command
            .is_none()
    );
}

#[test]
fn file_tools_cli_requires_an_explicit_value_and_preserves_other_commands() {
    for arguments in [
        vec!["open-seo-spider", "serve", "--mcp-files-dir"],
        vec!["open-seo-spider", "mcp", "--mcp-files-dir"],
        vec![
            "open-seo-spider",
            "crawl",
            "https://example.test/",
            "--mcp-files-dir",
            "folder",
        ],
    ] {
        assert!(crate::Cli::try_parse_from(arguments).is_err());
    }
}

#[tokio::test]
async fn file_tools_inventory_is_disabled_by_default_and_declares_exact_five_when_enabled() {
    let directory = Directory::new();
    let disabled = Bridge::new("http://127.0.0.1:1").unwrap();
    let enabled =
        Bridge::new_with_file_tools("http://127.0.0.1:1", Some(directory.0.clone())).unwrap();
    let request = json!({"jsonrpc":"2.0","id":7,"method":"tools/list"});
    let old = disabled.dispatch(request.clone()).await.unwrap();
    let new = enabled.dispatch(request).await.unwrap();
    enabled.shutdown_file_tools().await.unwrap();
    let old_tools = old["result"]["tools"].as_array().unwrap();
    let new_tools = new["result"]["tools"].as_array().unwrap();
    assert_eq!(inventory().len(), 26);
    assert_eq!(old_tools.len(), 27);
    assert_eq!(new_tools.len(), 32);
    assert_eq!(&old_tools[..26], inventory().as_slice());
    assert_eq!(old_tools[26], enrichment::definition());
    assert_eq!(&new_tools[..27], old_tools.as_slice());
    let mut actual: Vec<_> = new_tools[27..]
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    actual.sort_unstable();
    let mut expected = FILE_NAMES;
    expected.sort_unstable();
    assert_eq!(actual, expected);
    for tool in &new_tools[27..] {
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
        assert!(tool["inputSchema"]["properties"].is_object());
    }
    assert!(disabled.file_tools_capability().is_none());
}

#[tokio::test]
async fn file_tools_disabled_requests_never_reach_the_api() {
    let (base, count, stop, worker) = trap().await;
    let bridge = Bridge::new(&base).unwrap();
    let mut results = Vec::new();
    for (index, name) in FILE_NAMES.iter().enumerate() {
        results.push(
            local(
                &bridge,
                index as u64,
                name,
                json!({"path":"sentinel","text":"must not write"}),
            )
            .await,
        );
    }
    bridge.shutdown_file_tools().await.unwrap();
    close_trap(stop, worker).await;
    assert_eq!(count.load(Ordering::SeqCst), 0);
    for value in results {
        assert_eq!(value["error"]["code"], -32602);
    }
}

#[test]
fn file_tools_api_address_retains_exact_loopback_and_default_fallback() {
    let mut bridge = Bridge::new("http://127.0.0.1:1").unwrap();
    for (address, expected) in [
        ("127.0.0.2:31001", "http://127.0.0.2:31001/"),
        ("[::1]:31002", "http://[::1]:31002/"),
        ("0.0.0.0:31003", "http://127.0.0.1:31003/"),
        ("[::]:31004", "http://[::1]:31004/"),
        ("192.0.2.1:31005", "http://127.0.0.1:31005/"),
    ] {
        bridge
            .set_local_api_address(address.parse().unwrap())
            .unwrap();
        assert_eq!(bridge.base.as_str(), expected);
    }
}

#[tokio::test]
async fn file_tools_all_five_execute_locally_with_no_api_calls() {
    let directory = Directory::new();
    let (base, count, stop, worker) = trap().await;
    let bridge = Bridge::new_with_file_tools(&base, Some(directory.0.clone())).unwrap();
    let results = vec![
        local(&bridge, 1, "spider_list_allowed_directory", json!({})).await,
        local(
            &bridge,
            2,
            "spider_create_directory",
            json!({"path":"reports"}),
        )
        .await,
        local(
            &bridge,
            3,
            "spider_write_text_file",
            json!({"path":"reports/one.txt","text":"first ä\n"}),
        )
        .await,
        local(
            &bridge,
            4,
            "spider_read_text_file",
            json!({"path":"reports/one.txt","max_bytes":262144}),
        )
        .await,
        local(
            &bridge,
            5,
            "spider_list_directory",
            json!({"path":"reports","limit":1}),
        )
        .await,
        local(
            &bridge,
            6,
            "spider_write_text_file",
            json!({"path":"reports/one.txt","text":"replacement 🦉"}),
        )
        .await,
    ];
    bridge.shutdown_file_tools().await.unwrap();
    close_trap(stop, worker).await;
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert!(directory.0.join("reports").is_dir());
    assert_eq!(
        std::fs::read_to_string(directory.0.join("reports/one.txt")).unwrap(),
        "replacement 🦉"
    );
    for response in results {
        assert_eq!(response["result"]["isError"], false, "{response}");
        assert!(response["result"]["structuredContent"].is_object());
        assert_eq!(response["result"]["content"].as_array().unwrap().len(), 1);
        assert!(serde_json::to_vec(&response).unwrap().len() < MAX_FILE_WIRE_BYTES);
    }
}

#[tokio::test]
async fn file_tools_rpc_id_is_bounded_before_version_errors_or_any_write() {
    let directory = Directory::new();
    let bridge =
        Bridge::new_with_file_tools("http://127.0.0.1:1", Some(directory.0.clone())).unwrap();
    let mut responses = Vec::new();
    for id in [
        Value::Null,
        json!(1.5),
        json!({"nested":"id"}),
        json!("é".repeat(513)),
        json!("x".repeat(1025)),
    ] {
        for version in ["2.0", "wrong"] {
            let mut request = call(
                id.clone(),
                "spider_write_text_file",
                json!({"path":"never.txt","text":"never"}),
            );
            request["jsonrpc"] = json!(version);
            responses.push(bridge.dispatch(request).await.unwrap());
        }
    }
    let mut wrong_version = call(
        json!(9),
        "spider_write_text_file",
        json!({"path":"never.txt","text":"never"}),
    );
    wrong_version["jsonrpc"] = json!("wrong");
    let valid_id = bridge.dispatch(wrong_version).await.unwrap();
    let allowed = bridge
        .dispatch(call(
            json!("é".repeat(512)),
            "spider_list_allowed_directory",
            json!({}),
        ))
        .await
        .unwrap();
    bridge.shutdown_file_tools().await.unwrap();
    assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 0);
    assert_eq!(valid_id["id"], 9);
    assert_eq!(valid_id["error"]["code"], -32600);
    for response in responses {
        assert!(response["id"].is_null());
        assert_eq!(response["error"]["code"], -32600);
        assert!(serde_json::to_vec(&response).unwrap().len() < 256);
    }
    assert_eq!(allowed["result"]["isError"], false);
    assert_eq!(allowed["id"].as_str().unwrap().len(), 1024);
}

#[tokio::test]
async fn file_tools_invalid_shapes_and_oversized_text_never_mutate() {
    let directory = Directory::new();
    let (base, count, stop, worker) = trap().await;
    let bridge = Bridge::new_with_file_tools(&base, Some(directory.0.clone())).unwrap();
    let outside = directory.0.with_file_name(format!(
        "{}-escape",
        directory.0.file_name().unwrap().to_string_lossy()
    ));
    let escape = format!("../{}", outside.file_name().unwrap().to_string_lossy());
    let cases = vec![
        (
            "spider_write_text_file",
            json!({"path":"never.txt","text":null}),
        ),
        (
            "spider_write_text_file",
            json!({"path":"never.txt","text":true}),
        ),
        (
            "spider_write_text_file",
            json!({"path":"never.txt","text":"small","overwrite":false}),
        ),
        (
            "spider_write_text_file",
            json!({"path":"never.txt","text":"x".repeat(1048577)}),
        ),
        ("spider_create_directory", json!({"path":[]})),
        ("spider_create_directory", json!({"path":escape})),
        ("spider_list_directory", json!({"limit":0})),
        ("spider_list_directory", json!({"limit":"1"})),
        (
            "spider_read_text_file",
            json!({"path":"never.txt","offset":-1}),
        ),
        (
            "spider_read_text_file",
            json!({"path":"never.txt","offset":1}),
        ),
    ];
    let mut results = Vec::new();
    for (index, (name, args)) in cases.into_iter().enumerate() {
        results.push(local(&bridge, index as u64, name, args).await);
    }
    bridge.shutdown_file_tools().await.unwrap();
    close_trap(stop, worker).await;
    assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 0);
    assert!(!outside.exists());
    assert_eq!(count.load(Ordering::SeqCst), 0);
    for response in results {
        assert!(
            response.get("error").is_some() || response["result"]["isError"] == true,
            "{response}"
        );
    }
}

#[tokio::test]
async fn file_tools_actual_text_response_is_single_copy_and_whole_wire_bounded() {
    let directory = Directory::new();
    let text = "\u{0001}".repeat(262144);
    std::fs::write(directory.0.join("escaped.txt"), text.as_bytes()).unwrap();
    let bridge =
        Bridge::new_with_file_tools("http://127.0.0.1:1", Some(directory.0.clone())).unwrap();
    let response = bridge
        .dispatch(call(
            json!("\u{0000}".repeat(1024)),
            "spider_read_text_file",
            json!({"path":"escaped.txt","max_bytes":262144}),
        ))
        .await
        .unwrap();
    bridge.shutdown_file_tools().await.unwrap();
    let bytes = serde_json::to_vec(&response).unwrap();
    assert_eq!(response["result"]["isError"], false, "{response}");
    assert!(bytes.len() < MAX_FILE_WIRE_BYTES);
    assert!(
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .len()
            < 128
    );
    // The actual escaped file content occurs in structuredContent only. A
    // duplicated text representation would double this physical escape count.
    assert_eq!(
        bytes.windows(6).filter(|part| *part == b"\\u0001").count(),
        262144
    );
}

#[test]
fn file_tools_wire_guard_accepts_exact_physical_frame_and_rejects_plus_one() {
    // Adapter boundary unit control, separate from the real FileTools' smaller
    // result cap. The independent oracle is actual serialized physical bytes.
    let baseline = file_result(json!(17), Ok(json!({"text":""})));
    let overhead = serde_json::to_vec(&baseline).unwrap().len() + 1;
    let exact = file_result(
        json!(17),
        Ok(json!({"text":"x".repeat(MAX_FILE_WIRE_BYTES-overhead)})),
    );
    assert_eq!(
        serde_json::to_vec(&exact).unwrap().len() + 1,
        MAX_FILE_WIRE_BYTES
    );
    assert_eq!(exact["result"]["isError"], false);
    let overflow = file_result(
        json!(17),
        Ok(json!({"text":"x".repeat(MAX_FILE_WIRE_BYTES-overhead+1)})),
    );
    assert_eq!(overflow["id"], 17);
    assert_eq!(overflow["error"]["code"], -32603);
    assert!(overflow.get("result").is_none());
    assert!(serde_json::to_vec(&overflow).unwrap().len() < 256);
}

#[tokio::test]
async fn file_tools_actual_stdio_frames_share_opt_in_and_drain_on_eof() {
    let directory = Directory::new();
    let bridge =
        Bridge::new_with_file_tools("http://127.0.0.1:1", Some(directory.0.clone())).unwrap();
    let worker_bridge = bridge.clone();
    let (client, server) = duplex(32768);
    let (client_read, mut client_write) = split(client);
    let (server_read, server_write) = split(server);
    let worker =
        tokio::spawn(
            async move { stdio_and_drain(&worker_bridge, server_read, server_write).await },
        );
    let reading = tokio::spawn(async move {
        let mut bytes = Vec::new();
        let mut input = BufReader::new(client_read);
        input.read_to_end(&mut bytes).await.unwrap();
        bytes
    });
    for request in [
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        call(
            json!(2),
            "spider_write_text_file",
            json!({"path":"stdio.txt","text":"actual stdio"}),
        ),
        call(
            json!(3),
            "spider_read_text_file",
            json!({"path":"stdio.txt"}),
        ),
        json!({"jsonrpc":"2.0","id":4,"method":"ping"}),
    ] {
        let mut bytes = serde_json::to_vec(&request).unwrap();
        bytes.push(b'\n');
        client_write.write_all(&bytes).await.unwrap();
    }
    client_write.shutdown().await.unwrap();
    let serving = join_owned(worker, "actual stdio").await;
    let drained = bridge.shutdown_file_tools().await;
    let received = join_owned(reading, "stdio receiver").await;
    serving.unwrap().unwrap();
    drained.unwrap();
    let bytes = received.unwrap();
    let values: Vec<Value> = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    let after = local(
        &bridge,
        5,
        "spider_write_text_file",
        json!({"path":"after-eof.txt","text":"must be rejected after drain"}),
    )
    .await;
    assert_eq!(values.len(), 4);
    assert_eq!(values[0]["result"]["tools"].as_array().unwrap().len(), 32);
    assert_eq!(values[1]["result"]["isError"], false);
    assert_eq!(values[2]["result"]["isError"], false);
    assert_eq!(values[3]["result"], json!({}));
    assert_eq!(
        std::fs::read_to_string(directory.0.join("stdio.txt")).unwrap(),
        "actual stdio"
    );
    assert_eq!(after["result"]["isError"], true);
    assert!(!directory.0.join("after-eof.txt").exists());
}

struct BrokenOutput;
impl AsyncWrite for BrokenOutput {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut TaskContext<'_>,
        _: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "directed output failure",
        )))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn file_tools_stdio_output_error_still_awaits_registry_drain() {
    let directory = Directory::new();
    let bridge =
        Bridge::new_with_file_tools("http://127.0.0.1:1", Some(directory.0.clone())).unwrap();
    let mut bytes = serde_json::to_vec(&call(
        json!(1),
        "spider_write_text_file",
        json!({"path":"before-error.txt","text":"completed operation"}),
    ))
    .unwrap();
    bytes.push(b'\n');
    let result = stdio_and_drain(&bridge, bytes.as_slice(), BrokenOutput).await;
    let after = local(
        &bridge,
        2,
        "spider_write_text_file",
        json!({"path":"after-error.txt","text":"must not run"}),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        std::fs::read_to_string(directory.0.join("before-error.txt")).unwrap(),
        "completed operation"
    );
    assert_eq!(after["result"]["isError"], true);
    assert!(!directory.0.join("after-error.txt").exists());
}

#[tokio::test]
async fn file_tools_server_rejects_bad_root_and_nonloopback_before_data_directory() {
    let directory = Directory::new();
    let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = occupied.local_addr().unwrap().to_string();
    let missing = directory.0.join("missing");
    let regular_file = directory.0.join("not-a-directory");
    std::fs::write(
        &regular_file,
        b"regular file cannot be a directory capability",
    )
    .unwrap();
    let data = directory.0.join("data");
    for root in [missing, regular_file] {
        let validation_error =
            Bridge::new_with_file_tools("http://127.0.0.1:0", Some(root.clone()))
                .err()
                .expect("Missing file-tools root must be rejected");
        let error = timeout(
            DEADLINE,
            crate::server::serve_with_mcp_files(&address, data.clone(), Some(root)),
        )
        .await
        .unwrap()
        .unwrap_err();
        assert_eq!(
            format!("{error:#}"),
            format!("{validation_error:#}"),
            "The actual root-validation error must precede the deliberately occupied listener"
        );
        assert!(!data.exists());
    }
    for listen in ["0.0.0.0:0", "[::]:0", "192.0.2.1:0"] {
        let error = timeout(
            DEADLINE,
            crate::server::serve_with_mcp_files(listen, data.clone(), Some(directory.0.clone())),
        )
        .await
        .unwrap()
        .unwrap_err();
        assert!(error.to_string().contains("loopback"), "{error:#}");
        assert!(!data.exists());
    }
}

async fn http_mode(enabled: bool, listen_ip: &str) -> anyhow::Result<()> {
    let directory = Directory::new();
    let files = directory.0.join("files");
    std::fs::create_dir(&files)?;
    let listen = std::net::SocketAddr::new(listen_ip.parse()?, 0).to_string();
    let data = directory.0.join("data");
    let configured = enabled.then(|| files.clone());
    let (ready, readiness) = oneshot::channel();
    let worker = tokio::spawn(async move {
        crate::server::serve_with_mcp_files_ready(&listen, data, configured, Some(ready)).await
    });
    let result = async {
        // Only the actual owned production bind/startup can supply this address.
        // A failed bind/mkdir/report load drops the channel: no foreign health probe.
        let address = timeout(DEADLINE, readiness).await??;
        ensure!(address.ip().to_string() == listen_ip && address.port() != 0);
        let client = Client::builder()
            .no_proxy()
            .pool_max_idle_per_host(0)
            .timeout(DEADLINE)
            .build()?;
        let base = format!("http://{address}");
        let capability: Value = client
            .get(format!("{base}/api/capabilities"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let listed: Value = client
            .post(format!("{base}/mcp"))
            .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        ensure!(
            listed["result"]["tools"]
                .as_array()
                .context("actual tool list")?
                .len()
                == if enabled { 32 } else { 27 }
        );
        let legacy: Value = client
            .post(format!("{base}/mcp"))
            .json(&call(json!(99), "spider_list_crawls", json!({})))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        ensure!(legacy["result"]["isError"] == false);
        ensure!(
            legacy["result"]["structuredContent"]["crawls"]
                .as_array()
                .context("actual legacy crawl list")?
                .is_empty()
        );
        let forbidden = client
            .post(format!("{base}/mcp"))
            .header(
                reqwest::header::HOST,
                format!("192.0.2.1:{}", address.port()),
            )
            .json(&call(
                json!(98),
                "spider_write_text_file",
                json!({"path":"forbidden-host.txt","text":"never"}),
            ))
            .send()
            .await?;
        ensure!(forbidden.status() == reqwest::StatusCode::FORBIDDEN);
        ensure!(!files.join("forbidden-host.txt").exists());
        let response = client
            .post(format!("{base}/mcp"))
            .json(&call(
                json!(2),
                "spider_write_text_file",
                json!({"path":"http.txt","text":"actual HTTP file operation"}),
            ))
            .send()
            .await?
            .error_for_status()?;
        let bytes = response.bytes().await?;
        ensure!(bytes.len() + 1 <= MAX_FILE_WIRE_BYTES);
        let written: Value = serde_json::from_slice(&bytes)?;
        if enabled {
            ensure!(capability["mcp"]["file_tools"]["enabled"] == true);
            ensure!(capability["mcp"]["file_tools"]["limits"]["write_text_utf8_bytes"] == 1048576);
            ensure!(capability["mcp"]["file_tools"]["limits"]["mcp_wire_bytes"] == 8388608);
            ensure!(
                capability["mcp"]["file_tools"].get("node").is_none()
                    && capability["mcp"]["file_tools"].get("npm").is_none()
            );
            ensure!(written["result"]["isError"] == false);
            ensure!(
                std::fs::read_to_string(files.join("http.txt"))? == "actual HTTP file operation"
            );
            let invalid: Value = client
                .post(format!("{base}/mcp"))
                .json(&call(
                    json!("x".repeat(1025)),
                    "spider_write_text_file",
                    json!({"path":"never-http.txt","text":"never"}),
                ))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            ensure!(invalid["id"].is_null() && invalid["error"]["code"] == -32600);
            ensure!(!files.join("never-http.txt").exists());
        } else {
            ensure!(capability["mcp"] == json!({"stdio":true,"streamable_http":"/mcp"}));
            ensure!(written["error"]["code"] == -32602);
            ensure!(std::fs::read_dir(&files)?.count() == 0);
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;
    // Tests own this task and await its actual cancellation join, after all file
    // operations above have completed. Production drain is exercised separately.
    worker.abort();
    let joined = worker.await;
    ensure!(
        joined.is_err_and(|failure| failure.is_cancelled()),
        "Owned server task did not acknowledge cancellation"
    );
    result
}

#[tokio::test]
async fn file_tools_http_opt_in_capabilities_inventory_and_dispatch_share_module() {
    http_mode(false, "127.0.0.1").await.unwrap();
    http_mode(true, "127.0.0.1").await.unwrap();
    http_mode(true, "127.0.0.2").await.unwrap();
}
