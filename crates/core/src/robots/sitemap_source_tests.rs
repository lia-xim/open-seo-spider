use crate::robots::RobotsCache;
use crate::robots_sitemap_source::ROBOTS_BODY_CAP;
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener},
    thread,
    time::Duration,
};
use url::Url;

async fn observe(
    status: u16,
    bytes: Vec<u8>,
    declared_bytes: usize,
) -> std::sync::Arc<crate::robots::RobotsPolicy> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    assert!(![8787, 8790].contains(&port));
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = [0; 4096];
        let mut count = 0;
        while !request[..count]
            .windows(4)
            .any(|bytes| bytes == b"\r\n\r\n")
        {
            assert!(count < request.len(), "Owned request-header budget");
            let received = stream.read(&mut request[count..]).unwrap();
            assert!(received > 0, "Owned request-header EOF");
            count += received;
        }
        assert!(String::from_utf8_lossy(&request[..count]).starts_with("GET /robots.txt HTTP/1.1"));
        let extra = if status == 206 {
            format!(
                "Content-Range: bytes 0-{}/{}\r\n",
                bytes.len() - 1,
                bytes.len() * 2
            )
        } else {
            String::new()
        };
        stream.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Length: {declared_bytes}\r\nConnection: close\r\n{extra}\r\n").as_bytes()).unwrap();
        stream.write_all(&bytes).unwrap();
        stream.shutdown(Shutdown::Write).unwrap();
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let policy = RobotsCache::new(client)
        .get(&Url::parse(&format!("http://127.0.0.1:{port}/a")).unwrap())
        .await;
    server.join().unwrap();
    policy
}
fn capped_body() -> Vec<u8> {
    let mut bytes = b"User-agent: *\nAllow: /\nSitemap: https://example.test/map.xml\n#".to_vec();
    bytes.resize(ROBOTS_BODY_CAP, b' ');
    bytes
}
#[tokio::test]
async fn native_200_source_and_matching_are_complete_positive_control() {
    let bytes = b"User-agent: *\nAllow: /\nSitemap: https://example.test/a.xml\n".to_vec();
    let policy = observe(200, bytes.clone(), bytes.len()).await;
    assert!(policy.allowed("OwnedFixture", "https://example.test/a"));
    assert!(policy.sitemap_source.as_ref().unwrap().complete);
    assert_eq!(policy.sitemaps.len(), 1);
}
#[tokio::test]
async fn native_206_clean_eof_is_qualified_without_new_matching_block() {
    let bytes = b"User-agent: *\nAllow: /\nSitemap: https://example.test/a.xml\n".to_vec();
    let policy = observe(206, bytes.clone(), bytes.len()).await;
    let source = policy.sitemap_source.as_ref().unwrap();
    assert!(source.body_complete);
    assert!(!source.complete);
    assert_eq!(source.status, Some(206));
    assert_eq!(source.declarations.len(), 1);
    assert!(!policy.deny_all);
    assert!(policy.warning.is_none());
    assert_eq!(policy.sitemaps.len(), 1);
}
#[tokio::test]
async fn native_exact_cap_true_eof_is_complete_positive_control() {
    let bytes = capped_body();
    let policy = observe(200, bytes, ROBOTS_BODY_CAP).await;
    assert!(policy.sitemap_source.as_ref().unwrap().complete);
    assert!(!policy.deny_all);
    assert!(policy.warning.is_none());
}
#[tokio::test]
async fn native_read_error_after_matching_cap_does_not_change_legacy_allow() {
    let bytes = capped_body();
    let policy = observe(200, bytes, ROBOTS_BODY_CAP + 1).await;
    let source = policy.sitemap_source.as_ref().unwrap();
    assert_eq!(source.retained_post_http_decoding_bytes, ROBOTS_BODY_CAP);
    assert!(!source.complete && !source.body_complete);
    assert!(source.failure.is_some());
    assert!(!policy.deny_all);
    assert!(policy.warning.is_none());
    assert_eq!(policy.sitemaps.len(), 1);
    assert!(policy.allowed("OwnedFixture", "https://example.test/a"));
}
#[tokio::test]
async fn native_read_error_below_matching_cap_retains_old_deny_behavior() {
    let bytes = b"User-agent: *\nAllow: /\n".to_vec();
    let policy = observe(200, bytes.clone(), bytes.len() + 1).await;
    let source = policy.sitemap_source.as_ref().unwrap();
    assert!(!source.complete);
    assert!(source.failure.is_some());
    assert!(policy.deny_all);
    assert!(policy.warning.is_some());
    assert!(!policy.allowed("OwnedFixture", "https://example.test/a"));
}
#[tokio::test]
async fn native_cr_source_keeps_original_facts_and_historical_lf_projection_separate() {
    let bytes=b"User-agent: *\rAllow: /\rSitemap: https://example.test/a.xml\rSitemap: https://example.test/b.xml\r".to_vec();
    let policy = observe(200, bytes.clone(), bytes.len()).await;
    let source = policy.sitemap_source.as_ref().unwrap();
    assert!(source.complete);
    assert_eq!(source.declarations.len(), 2);
    assert_eq!(source.declarations[0].line_ordinal, 3);
    assert!(policy.sitemaps.is_empty());
    assert!(policy.allowed("OwnedFixture", "https://example.test/a"));
}
