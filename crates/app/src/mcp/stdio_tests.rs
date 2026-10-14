//! Physical byte-frame tests for the actual shared stdio loop, not a mock parser.
use super::*;
use tokio::{
    io::{DuplexStream, ReadHalf, WriteHalf, duplex, split},
    task::JoinHandle,
    time::{Duration, timeout},
};

type Writer = WriteHalf<DuplexStream>;
type Reader = BufReader<ReadHalf<DuplexStream>>;
type Server = JoinHandle<anyhow::Result<()>>;
const DEADLINE: Duration = Duration::from_secs(10);

fn stream() -> (Writer, Reader, Server) {
    let (client, server) = duplex(16 * 1024);
    let (client_read, client_write) = split(client);
    let (server_read, server_write) = split(server);
    let bridge = Bridge::new("http://127.0.0.1:1").unwrap();
    let task = tokio::spawn(async move { stdio_stream(&bridge, server_read, server_write).await });
    (client_write, BufReader::new(client_read), task)
}

fn ping(id: u64) -> Vec<u8> {
    serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"method":"ping"})).unwrap()
}

fn padded_frame(id: u64, bytes: usize, ending: &[u8]) -> Vec<u8> {
    let mut frame = ping(id);
    assert!(bytes >= frame.len() + ending.len());
    frame.resize(bytes - ending.len(), b' ');
    frame.extend_from_slice(ending);
    assert_eq!(frame.len(), bytes);
    frame
}

async fn replies(mut reader: Reader, server: Server) -> Vec<Value> {
    // Tiny ping/error replies fit the duplex output. Completion proves the server
    // closes even if an oversized-frame producer has not sent a delimiter or EOF.
    timeout(DEADLINE, server)
        .await
        .expect("stdio loop did not close promptly")
        .expect("stdio task panicked")
        .expect("stdio loop returned I/O failure");
    let mut bytes = Vec::new();
    timeout(DEADLINE, reader.read_to_end(&mut bytes))
        .await
        .expect("stdio output did not reach EOF")
        .unwrap();
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect()
}

fn assert_one_oversize_error(values: &[Value]) {
    assert_eq!(values.len(), 1, "no prefix/suffix may be dispatched");
    assert_eq!(values[0]["jsonrpc"], "2.0");
    assert_eq!(values[0]["id"], Value::Null);
    assert_eq!(values[0]["error"]["code"], -32600);
    assert_eq!(values[0]["error"]["message"], "Request exceeds 8 MiB");
    assert!(values[0].get("result").is_none());
}

#[tokio::test]
async fn stdio_exact_eight_mib_line_and_following_request_are_dispatched() {
    let (mut writer, reader, server) = stream();
    let exact = padded_frame(41, MAX_STDIO_REQUEST_BYTES, b"\n");
    let writing = tokio::spawn(async move {
        writer.write_all(&exact).await.unwrap();
        writer.write_all(&ping(42)).await.unwrap();
        writer.write_all(b"\n").await.unwrap();
        writer.shutdown().await.unwrap();
    });
    timeout(DEADLINE, writing).await.unwrap().unwrap();
    let values = replies(reader, server).await;
    assert_eq!(values.len(), 2);
    for (value, id) in values.iter().zip([41, 42]) {
        assert_eq!(value["id"], id);
        assert_eq!(value["result"], json!({}));
        assert!(value.get("error").is_none());
    }
}

#[tokio::test]
async fn stdio_plus_one_without_delimiter_rejects_with_open_producer_and_closes() {
    let (mut writer, reader, server) = stream();
    // The prefix is valid JSON, followed by whitespace. It must never dispatch
    // merely because a bounded read or a client write happens to split the frame.
    let oversized = padded_frame(51, MAX_STDIO_REQUEST_BYTES + 1, b"");
    let writing = tokio::spawn(async move {
        writer.write_all(&oversized).await.unwrap();
        writer // Keep the producer open: no newline, shutdown or EOF is sent.
    });
    let mut writer = timeout(DEADLINE, writing)
        .await
        .expect("server did not consume the bounded sentinel")
        .unwrap();
    let values = replies(reader, server).await;
    assert_one_oversize_error(&values);
    let mut suffix = b"\n".to_vec();
    suffix.extend(ping(52));
    suffix.push(b'\n');
    assert!(
        timeout(DEADLINE, writer.write_all(&suffix))
            .await
            .unwrap()
            .is_err(),
        "overflow must close input instead of accepting a new suffix frame"
    );
}

#[tokio::test]
async fn stdio_oversized_terminated_frame_never_dispatches_prefix_or_following_frame() {
    let (mut writer, reader, server) = stream();
    let mut bytes = padded_frame(61, MAX_STDIO_REQUEST_BYTES + 1, b"\n");
    bytes.extend(ping(62));
    bytes.push(b'\n');
    let writing = tokio::spawn(async move {
        let result = writer.write_all(&bytes).await;
        if let Err(error) = &result {
            assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        }
        // Closing on overflow can interrupt this write; completion of the suffix
        // write is not a prerequisite for rejecting the first oversized frame.
        result
    });
    let values = replies(reader, server).await;
    let _ = timeout(DEADLINE, writing).await.unwrap().unwrap();
    assert_one_oversize_error(&values);
}

#[tokio::test]
async fn stdio_fragmented_crlf_notification_invalid_json_and_final_eof_are_preserved() {
    let (mut writer, reader, server) = stream();
    let mut bytes =
        serde_json::to_vec(&json!({"jsonrpc":"2.0","method":"notifications/initialized"})).unwrap();
    bytes.push(b'\n');
    bytes.extend(ping(71));
    bytes.extend_from_slice(b"\r\n{invalid-json}\n");
    bytes.extend(ping(72)); // A final valid JSON request without a newline.
    let writing = tokio::spawn(async move {
        for chunk in bytes.chunks(3) {
            writer.write_all(chunk).await.unwrap();
            tokio::task::yield_now().await;
        }
        writer.shutdown().await.unwrap();
    });
    timeout(DEADLINE, writing).await.unwrap().unwrap();
    let values = replies(reader, server).await;
    assert_eq!(values.len(), 3, "notification must not emit a response");
    assert_eq!(values[0]["id"], 71);
    assert_eq!(values[0]["result"], json!({}));
    assert_eq!(values[1]["id"], Value::Null);
    assert_eq!(values[1]["error"]["code"], -32700);
    assert_eq!(values[2]["id"], 72);
    assert_eq!(values[2]["result"], json!({}));
}

#[tokio::test]
async fn stdio_empty_input_and_final_eof_notification_emit_no_response() {
    for bytes in [
        vec![],
        serde_json::to_vec(&json!({"jsonrpc":"2.0","method":"notifications/initialized"})).unwrap(),
    ] {
        let (mut writer, reader, server) = stream();
        writer.write_all(&bytes).await.unwrap();
        writer.shutdown().await.unwrap();
        assert!(replies(reader, server).await.is_empty());
    }
}
