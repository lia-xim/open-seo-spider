//! AUTHORED_NOT_RUN. These tests were written before the CLI relay implementation.
use super::*;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const JOB: &str = "5bcdbb0c-b4ba-443b-a7e2-536358574adb";
const OTHER_JOB: &str = "68fc26dc-fffc-43c5-9b9d-2d4cbde96c28";

struct Seen {
    headers: String,
    body: Vec<u8>,
    write_error: Option<String>,
}

async fn one_response(
    status: u16,
    extra_headers: &str,
    body: Vec<u8>,
    delay: Duration,
) -> (String, tokio::task::JoinHandle<anyhow::Result<Seen>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let extra_headers = extra_headers.to_owned();
    let task = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), async move {
            let (mut socket, _) = listener.accept().await?;
            drop(listener);
            let mut raw = Vec::new();
            let mut chunk = [0; 1024];
            let end = loop {
                if let Some(index) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    break index + 4;
                }
                let n = socket.read(&mut chunk).await?;
                anyhow::ensure!(n > 0, "HTTP request ended before headers");
                raw.extend_from_slice(&chunk[..n]);
                anyhow::ensure!(raw.len() <= 8192, "Owned request header bound");
            };
            let headers = String::from_utf8(raw[..end].to_vec())?;
            let length = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .context("Owned request Content-Length")?
                .1
                .trim()
                .parse::<usize>()?;
            anyhow::ensure!(length <= OPTIONS_MAX_BYTES, "Owned body bound");
            while raw.len() - end < length {
                let n = socket.read(&mut chunk).await?;
                anyhow::ensure!(n > 0, "HTTP request ended before body");
                raw.extend_from_slice(&chunk[..n]);
                anyhow::ensure!(raw.len() <= 8192 + OPTIONS_MAX_BYTES, "Owned request bound");
            }
            anyhow::ensure!(raw.len() - end == length, "Exactly one request body");
            tokio::time::sleep(delay).await;
            let response = format!(
                "HTTP/1.1 {status} Owned\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{extra_headers}\r\n",
                body.len()
            );
            let result = async {
                socket.write_all(response.as_bytes()).await?;
                socket.write_all(&body).await?;
                socket.shutdown().await
            }
            .await;
            Ok(Seen {
                headers,
                body: raw[end..].to_vec(),
                write_error: result.err().map(|e| e.to_string()),
            })
        })
        .await
        .context("Owned HTTP fixture deadline")?
    });
    (origin, task)
}

fn authority() -> Value {
    json!({"expected_generation":7,"expected_profile_revision":4,
        "expected_profile_sha256":"a".repeat(64),"max_urls_per_depth":3,
        "url_pattern_maxima":{"first":5,"second":8}})
}

#[test]
fn cli_resume_requires_job_uuid_and_keeps_store_paths_out_of_its_syntax() {
    use clap::Parser;
    let cli = crate::Cli::try_parse_from([
        "spider",
        "resume",
        JOB,
        "--server",
        "http://127.0.0.1:8123",
        "--options",
        "increase.json",
    ])
    .unwrap();
    assert!(matches!(cli.command, Some(crate::Command::Resume { .. })));
    for argv in [
        vec!["spider", "resume", "saved.sqlite"],
        vec!["spider", "resume", JOB, "--store", "saved.sqlite"],
        vec!["spider", "resume", JOB, "--max-urls-per-depth", "3"],
    ] {
        assert!(crate::Cli::try_parse_from(argv).is_err());
    }
}

#[test]
fn cli_resume_options_preserve_zero_and_exact_generation_revision_hash_authority() {
    let input = json!({"max_pages":0,"concurrency":3,"delay_ms":0,
        "request_timeout_secs":20,"quota_increases":authority()});
    let parsed: ResumeOptions = serde_json::from_value(input.clone()).unwrap();
    let after = serde_json::to_value(parsed).unwrap();
    assert_eq!(after["max_pages"], 0);
    assert_eq!(after["delay_ms"], 0);
    assert_eq!(after["quota_increases"], input["quota_increases"]);
    assert_eq!(after["max_attempts"], Value::Null);
    let none: ResumeOptions = serde_json::from_value(json!({})).unwrap();
    assert_eq!(
        serde_json::to_value(none).unwrap()["quota_increases"],
        Value::Null
    );
}

#[test]
fn cli_resume_options_reject_unknown_and_duplicate_authority_fields() {
    for raw in [
        r#"{"unknown":1}"#.to_owned(),
        r#"{"max_pages":0,"max_pages":1}"#.to_owned(),
        format!(
            r#"{{"quota_increases":{{"expected_generation":7,"expected_profile_revision":4,"expected_profile_sha256":"{}","url_pattern_maxima":{{"same":1,"same":2}}}}}}"#,
            "a".repeat(64)
        ),
        r#"{"quota_increases":{"expected_generation":7,"max_urls_per_depth":3}}"#.to_owned(),
    ] {
        assert!(serde_json::from_slice::<ResumeOptions>(raw.as_bytes()).is_err());
    }
}

#[test]
fn cli_resume_endpoint_accepts_only_canonical_literal_loopback_origins() {
    let id = uuid::Uuid::parse_str(JOB).unwrap();
    for origin in ["http://127.0.0.1:8123", "http://[::1]:8123/"] {
        let actual = endpoint(origin, id).unwrap();
        assert_eq!(actual.path(), format!("/api/crawls/{JOB}/resume"));
    }
    for invalid in [
        "http://localhost:8123",
        "https://127.0.0.1:8123",
        "http://example.test/",
        "http://127.0.0.1:8123/api/",
        "http://u:p@127.0.0.1:8123/",
        "http://127.0.0.1:8123/?query=1",
        "http://127.0.0.1:8123/#fragment",
        "http://0177.0.0.1:8123",
        "http://127.0.0.1:0",
        " http://127.0.0.1:8123",
    ] {
        assert!(endpoint(invalid, id).is_err(), "{invalid}");
    }
}

#[tokio::test]
async fn cli_resume_invalid_options_refuse_before_any_local_connection() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server = format!("http://{}", listener.local_addr().unwrap());
    let dir = tempfile::tempdir().unwrap();
    let options = dir.path().join("bad.json");
    let bytes = b"{\"quota_increases\":{\"unexpected\":true}}";
    std::fs::write(&options, bytes).unwrap();
    let result = execute(&server, uuid::Uuid::parse_str(JOB).unwrap(), Some(&options)).await;
    let observed = tokio::time::timeout(Duration::from_millis(150), listener.accept()).await;
    drop(listener);
    assert!(result.is_err());
    assert!(observed.is_err());
    assert_eq!(std::fs::read(options).unwrap(), bytes);
}

#[tokio::test]
async fn cli_resume_posts_exact_authority_and_reports_acceptance_without_completion() {
    let input = json!({"max_pages":0,"quota_increases":authority()});
    let options: ResumeOptions = serde_json::from_value(input).unwrap();
    let (origin, task) = one_response(
        200,
        "",
        serde_json::to_vec(&json!({"id":JOB,"status":"running"})).unwrap(),
        Duration::ZERO,
    )
    .await;
    let result = request(
        &client(Duration::from_secs(2)).unwrap(),
        endpoint(&origin, uuid::Uuid::parse_str(JOB).unwrap()).unwrap(),
        uuid::Uuid::parse_str(JOB).unwrap(),
        &options,
    )
    .await;
    let seen = task.await.unwrap().unwrap(); // Drain owned listener/socket before business assertions.
    assert!(seen.write_error.is_none());
    assert!(
        seen.headers
            .starts_with(&format!("POST /api/crawls/{JOB}/resume HTTP/1.1\r\n"))
    );
    assert!(!seen.headers.to_ascii_lowercase().contains("authorization:"));
    assert!(!seen.headers.to_ascii_lowercase().contains("cookie:"));
    assert_eq!(
        serde_json::from_slice::<Value>(&seen.body).unwrap(),
        serde_json::to_value(&options).unwrap()
    );
    assert_eq!(
        serde_json::to_value(result.unwrap()).unwrap(),
        json!({"id":JOB,"status":"running"})
    );
}

#[tokio::test]
async fn cli_resume_propagates_stale_or_nonmonotone_refusal_without_claiming_success() {
    for cause in [
        "Stale quota profile",
        "Stale or active quota checkpoint",
        "Quota maxima must increase",
    ] {
        let (origin, task) = one_response(
            409,
            "",
            serde_json::to_vec(&json!({"error":cause})).unwrap(),
            Duration::ZERO,
        )
        .await;
        let result = request(
            &client(Duration::from_secs(2)).unwrap(),
            endpoint(&origin, uuid::Uuid::parse_str(JOB).unwrap()).unwrap(),
            uuid::Uuid::parse_str(JOB).unwrap(),
            &ResumeOptions::default(),
        )
        .await;
        let seen = task.await.unwrap().unwrap();
        assert!(seen.write_error.is_none());
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains("HTTP409") && error.contains(cause),
            "{error}"
        );
    }
}

#[tokio::test]
async fn cli_resume_never_follows_redirect_to_another_owned_listener() {
    let second = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let location = format!("http://{}/other", second.local_addr().unwrap());
    let second_task = tokio::spawn(async move {
        let accepted = tokio::time::timeout(Duration::from_millis(300), second.accept()).await;
        let unexpected = accepted.is_ok();
        drop(accepted);
        drop(second);
        unexpected
    });
    let (origin, task) = one_response(
        302,
        &format!("Location: {location}\r\n"),
        b"{}".to_vec(),
        Duration::ZERO,
    )
    .await;
    let result = request(
        &client(Duration::from_secs(2)).unwrap(),
        endpoint(&origin, uuid::Uuid::parse_str(JOB).unwrap()).unwrap(),
        uuid::Uuid::parse_str(JOB).unwrap(),
        &ResumeOptions::default(),
    )
    .await;
    let first = task.await.unwrap().unwrap();
    let followed = second_task.await.unwrap();
    assert!(first.write_error.is_none());
    assert!(result.unwrap_err().to_string().contains("HTTP302"));
    assert!(!followed);
}

#[tokio::test]
async fn cli_resume_rejects_other_job_or_completed_response_as_accepted_running() {
    for reply in [
        json!({"id":OTHER_JOB,"status":"running"}),
        json!({"id":JOB,"status":"completed"}),
    ] {
        let (origin, task) =
            one_response(200, "", serde_json::to_vec(&reply).unwrap(), Duration::ZERO).await;
        let result = request(
            &client(Duration::from_secs(2)).unwrap(),
            endpoint(&origin, uuid::Uuid::parse_str(JOB).unwrap()).unwrap(),
            uuid::Uuid::parse_str(JOB).unwrap(),
            &ResumeOptions::default(),
        )
        .await;
        let seen = task.await.unwrap().unwrap();
        assert!(seen.write_error.is_none());
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn cli_resume_reply_limit_and_deadline_are_real_refusals() {
    let (origin, task) =
        one_response(200, "", vec![b'a'; REPLY_MAX_BYTES + 1], Duration::ZERO).await;
    let cap = request(
        &client(Duration::from_secs(2)).unwrap(),
        endpoint(&origin, uuid::Uuid::parse_str(JOB).unwrap()).unwrap(),
        uuid::Uuid::parse_str(JOB).unwrap(),
        &ResumeOptions::default(),
    )
    .await;
    let _closed = task.await.unwrap().unwrap();
    assert!(cap.unwrap_err().to_string().contains("64KiB"));
    let (origin, task) = one_response(200, "", b"{}".to_vec(), Duration::from_millis(200)).await;
    let timed = request(
        &client(Duration::from_millis(50)).unwrap(),
        endpoint(&origin, uuid::Uuid::parse_str(JOB).unwrap()).unwrap(),
        uuid::Uuid::parse_str(JOB).unwrap(),
        &ResumeOptions::default(),
    )
    .await;
    let _closed = task.await.unwrap().unwrap();
    assert!(timed.is_err());
}

#[tokio::test]
async fn cli_resume_options_read_is_capped_during_growth_and_oversize_refuses_before_connection() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    // A controlled unit reader starts with a two-byte document, then exposes a
    // much larger suffix. Count actual bytes delivered, not initial metadata.
    struct GrowingSource {
        emitted: Arc<AtomicUsize>,
        grew: Arc<AtomicBool>,
    }
    impl tokio::io::AsyncRead for GrowingSource {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            dst: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            let emitted = self.emitted.load(Ordering::SeqCst);
            let available = if emitted < 2 {
                2
            } else {
                self.grew.store(true, Ordering::SeqCst);
                OPTIONS_MAX_BYTES * 16
            };
            let count = dst.remaining().min(4096).min(available - emitted);
            let mut chunk = [b' '; 4096];
            if emitted == 0 {
                chunk[0] = b'{';
                chunk[1] = b'}';
            }
            dst.put_slice(&chunk[..count]);
            self.emitted.fetch_add(count, Ordering::SeqCst);
            std::task::Poll::Ready(Ok(()))
        }
    }
    let emitted = Arc::new(AtomicUsize::new(0));
    let grew = Arc::new(AtomicBool::new(false));
    let result = read_options_bytes(GrowingSource {
        emitted: emitted.clone(),
        grew: grew.clone(),
    })
    .await;
    assert!(result.unwrap_err().to_string().contains("64KiB"));
    assert!(grew.load(Ordering::SeqCst));
    assert_eq!(emitted.load(Ordering::SeqCst), OPTIONS_MAX_BYTES + 1);

    let directory = tempfile::tempdir().unwrap();
    for length in [OPTIONS_MAX_BYTES - 1, OPTIONS_MAX_BYTES] {
        let path = directory.path().join(format!("exact-{length}.json"));
        let mut bytes = vec![b' '; length];
        bytes[..2].copy_from_slice(b"{}");
        std::fs::write(&path, &bytes).unwrap();
        let opened = tokio::fs::File::open(&path).await.unwrap();
        let actual = read_options_bytes(opened).await.unwrap();
        assert_eq!(actual, bytes);
        assert!(serde_json::from_slice::<ResumeOptions>(&actual).is_ok());
    }

    let path = directory.path().join("growing.json");
    std::fs::write(&path, b"{}").unwrap();
    let opened = tokio::fs::File::open(&path).await.unwrap();
    assert_eq!(opened.metadata().await.unwrap().len(), 2);
    let mut append = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    std::io::Write::write_all(&mut append, &vec![b' '; OPTIONS_MAX_BYTES * 4]).unwrap();
    drop(append);
    let grown = read_options_bytes(opened).await;
    assert!(grown.unwrap_err().to_string().contains("64KiB"));

    // The same oversized genuine options file must be refused before any API IO.
    let before = std::fs::read(&path).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server = format!("http://{}", listener.local_addr().unwrap());
    let refused = execute(&server, uuid::Uuid::parse_str(JOB).unwrap(), Some(&path)).await;
    let connected = tokio::time::timeout(Duration::from_millis(150), listener.accept()).await;
    drop(listener);
    assert!(refused.unwrap_err().to_string().contains("64KiB"));
    assert!(connected.is_err());
    assert_eq!(std::fs::read(path).unwrap(), before);
}
