//! Real local HTTP captures through the public crawler. These tests use JSON
//! evidence assertions so their regression also applies to the older API.
use open_seo_spider_core::{CrawlConfig, Crawler, CustomExtractionConfig};
use sha2::{Digest, Sha256};
use std::sync::{Arc, atomic::AtomicBool};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct Fixture {
    origin: String,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Fixture {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    loop {
                        let mut chunk = [0; 1024];
                        let Ok(n) = socket.read(&mut chunk).await else {
                            return;
                        };
                        if n == 0 {
                            return;
                        }
                        request.extend_from_slice(&chunk[..n]);
                        if request.windows(4).any(|part| part == b"\r\n\r\n") {
                            break;
                        }
                        if request.len() > 8192 {
                            return;
                        }
                    }
                    let text = String::from_utf8_lossy(&request);
                    let path = text.split_whitespace().nth(1).unwrap_or("/");
                    let mut status = 200;
                    let mut mime = "text/html; charset=utf-8";
                    let mut extra = "";
                    let body = match path {
                        "/empty" => Vec::new(),
                        "/plain" => {
                            mime = "text/plain; charset=utf-8";
                            b"no HTML".to_vec()
                        }
                        "/partial" => {
                            format!("<html><body>{}</body></html>", "x".repeat(256)).into_bytes()
                        }
                        "/lossy" => b"<html><body>invalid\xff</body></html>".to_vec(),
                        "/literal-replacement" => "<html><body>literal \u{fffd}</body></html>"
                            .as_bytes()
                            .to_vec(),
                        "/unknown-charset" => {
                            mime = "text/html; charset=not-a-real-encoding";
                            b"<html><body>none</body></html>".to_vec()
                        }
                        "/range" => {
                            status = 206;
                            extra = "Content-Range: bytes 0-27/100\r\n";
                            b"<html><body>none</body></html>".to_vec()
                        }
                        "/not-found" => {
                            status = 404;
                            b"<html><body>none</body></html>".to_vec()
                        }
                        "/read-failed" => b"<html><body>partial".to_vec(),
                        _ => b"<html><body>none</body></html>".to_vec(),
                    };
                    let declared = body.len() + if path == "/read-failed" { 100 } else { 0 };
                    let header = format!(
                        "HTTP/1.1 {status} Test\r\nContent-Type: {mime}\r\nContent-Length: {declared}\r\n{extra}Connection: close\r\n\r\n"
                    );
                    let _ = socket.write_all(header.as_bytes()).await;
                    let _ = socket.write_all(&body).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        Self { origin, task }
    }
}

#[tokio::test]
async fn native_custom_extraction_distinguishes_full_zero_partial_zero_and_not_run() {
    let site = Fixture::start().await;
    let paths = [
        "/complete",
        "/empty",
        "/partial",
        "/lossy",
        "/literal-replacement",
        "/unknown-charset",
        "/range",
        "/not-found",
        "/read-failed",
        "/plain",
    ];
    let cfg = CrawlConfig {
        seed_urls: paths
            .iter()
            .map(|path| format!("{}{path}", site.origin))
            .collect(),
        custom_extractions: vec![CustomExtractionConfig {
            name: "absent".into(),
            selector: ".not-present".into(),
            ..Default::default()
        }],
        obey_robots: false,
        discover_sitemaps: false,
        crawl_links: false,
        delay_ms: 0,
        concurrency: 1,
        request_timeout_secs: 5,
        max_body_bytes: 80,
        ..Default::default()
    };
    let report = Crawler::new(cfg)
        .unwrap()
        .run(None, Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    assert_eq!(report.pages.len(), paths.len());
    for page in &report.pages {
        let results = serde_json::to_value(&page.custom_extractions).unwrap();
        let result = &results[0];
        assert_eq!(results.as_array().unwrap().len(), 1, "{}", page.url);
        let obs = result
            .get("source_observation")
            .expect("Native results need explicit capture provenance");
        assert_eq!(obs["engine"], "open-seo-spider/extraction-source-v1");
        assert_eq!(obs["source"], "raw_html");
        assert_eq!(obs["observed_at"], page.fetched_at);
        assert!(
            obs["source_revision"]
                .as_str()
                .is_some_and(|value| value.len() == 64)
        );
        if page.url.ends_with("/plain") {
            assert_eq!(obs["status"], "unavailable");
            assert_eq!(obs["evaluation"], "not_run");
            assert!(result["matched_count"].is_null());
            assert!(obs["input_sha256"].is_null());
            assert_eq!(result["values"], serde_json::json!([]));
        } else {
            assert_eq!(obs["evaluation"], "evaluated");
            assert_eq!(result["matched_count"], 0);
            assert_eq!(result["values"], serde_json::json!([]));
            assert_eq!(
                result["truncated"], false,
                "Source truncation must not change the result-budget flag"
            );
            let complete = ["/complete", "/empty", "/literal-replacement", "/not-found"]
                .iter()
                .any(|suffix| page.url.ends_with(suffix));
            assert_eq!(
                obs["status"],
                if complete { "complete" } else { "incomplete" },
                "{}: {obs}",
                page.url
            );
            if complete {
                assert_eq!(obs["decode_status"], "exact");
                assert_eq!(obs["reasons"], serde_json::json!([]));
            } else {
                assert!(!obs["reasons"].as_array().unwrap().is_empty());
            }
        }
        if page.url.ends_with("/empty") {
            assert_eq!(obs["input_utf8_bytes"], 0);
            assert_eq!(obs["input_sha256"], format!("{:x}", Sha256::digest(b"")));
        }
    }
}
