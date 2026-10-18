//! Explicit test-executable host for browser -> native router -> local HTTP
//! provider -> saved report acceptance. This module is absent from normal builds.
use super::*;
use anyhow::{Result, ensure};
use futures::{StreamExt, future::BoxFuture};
use open_seo_spider_core::api_enrichment::{
    ApiCancellation, ApiFailureCode, PageSpeedReply, PageSpeedRequest, PageSpeedTransport,
};

struct LocalProviderTransport {
    endpoint: reqwest::Url,
    client: reqwest::Client,
}

impl LocalProviderTransport {
    fn new(endpoint: &str) -> Result<Self> {
        ensure!(endpoint.len() <= 2048, "Test provider URL is too long");
        let url = reqwest::Url::parse(endpoint)?;
        ensure!(
            endpoint.starts_with("http://127.0.0.1:")
                && url.scheme() == "http"
                && url.host_str() == Some("127.0.0.1")
                && url
                    .port()
                    .is_some_and(|port| port > 1024 && ![8787, 8790].contains(&port))
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none(),
            "Test provider requires an explicit isolated 127.0.0.1 HTTP endpoint"
        );
        let mut segments = url.path().split('/').skip(1);
        let nonce = segments.next().unwrap_or_default();
        let ordinary_segment = |segment: &str| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        };
        ensure!(
            (16..=128).contains(&nonce.len())
                && ordinary_segment(nonce)
                && url.path().len() <= 1024
                && segments.all(ordinary_segment),
            "Test provider requires a bounded nonce path"
        );
        Ok(Self {
            endpoint: url,
            client: reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        })
    }
}

impl PageSpeedTransport for LocalProviderTransport {
    fn execute<'a>(
        &'a self,
        request: PageSpeedRequest<'a>,
        cancel: ApiCancellation,
    ) -> BoxFuture<'a, std::result::Result<PageSpeedReply, ApiFailureCode>> {
        Box::pin(async move {
            // The fixture proves anonymous execution. It must never transfer an
            // inherited paid credential to the local test provider.
            if request.key.is_some() {
                return Err(ApiFailureCode::Authentication);
            }
            let pending = self
                .client
                .get(self.endpoint.clone())
                .query(&[
                    ("url", request.requested_url),
                    ("strategy", request.strategy.as_str()),
                    ("category", "performance"),
                ])
                .timeout(std::time::Duration::from_millis(request.timeout_ms))
                .send();
            let response = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(ApiFailureCode::Cancelled),
                response = pending => response.map_err(|error| if error.is_timeout() {
                    ApiFailureCode::Timeout
                } else {
                    ApiFailureCode::Network
                })?,
            };
            let status = response.status().as_u16();
            let retry_after_secs = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .map(|value| value.min(30));
            if response.content_length().is_some_and(|length| {
                usize::try_from(length)
                    .ok()
                    .is_none_or(|length| length > request.max_response_bytes)
            }) {
                return Err(ApiFailureCode::ResponseBudget);
            }
            let mut stream = response.bytes_stream();
            let mut bytes = Vec::new();
            loop {
                let chunk = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return Err(ApiFailureCode::Cancelled),
                    chunk = stream.next() => chunk,
                };
                let Some(chunk) = chunk else { break };
                let chunk = chunk.map_err(|_| ApiFailureCode::Network)?;
                if bytes
                    .len()
                    .checked_add(chunk.len())
                    .is_none_or(|length| length > request.max_response_bytes)
                {
                    return Err(ApiFailureCode::ResponseBudget);
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(PageSpeedReply {
                status,
                bytes,
                retry_after_secs,
            })
        })
    }
}

#[test]
fn test_provider_transport_refuses_non_fixture_origins_credentials_and_paths() {
    let valid = "http://127.0.0.1:43210/0123456789abcdef";
    assert!(LocalProviderTransport::new(valid).is_ok());
    assert!(LocalProviderTransport::new(&format!("{valid}/deny")).is_ok());
    for invalid in [
        "https://www.googleapis.com/pagespeedonline/v5/runPagespeed",
        "http://localhost:43210/0123456789abcdef",
        "http://127.1:43210/0123456789abcdef",
        "https://127.0.0.1:43210/0123456789abcdef",
        "http://127.0.0.1/0123456789abcdef",
        "http://127.0.0.1:0/0123456789abcdef",
        "http://127.0.0.1:8787/0123456789abcdef",
        "http://127.0.0.1:8790/0123456789abcdef",
        "http://127.0.0.1:43210/",
        "http://127.0.0.1:43210/short",
        "http://127.0.0.1:43210/0123456789abcdef/",
        "http://127.0.0.1:43210/0123456789abcdef?key=secret",
        "http://127.0.0.1:43210/0123456789abcdef#part",
        "http://user@127.0.0.1:43210/0123456789abcdef",
        "http://127.0.0.1:43210/0123456789abcdef/%2e",
    ] {
        assert!(LocalProviderTransport::new(invalid).is_err(), "{invalid}");
    }
}

/// Launch only through the owned browser coordinator. Closing this CHILD test
/// process's stdin runs the same production listener/task drain as the real app.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "owned browser coordinator supplies an isolated store and local HTTP provider"]
async fn native_enrichment_browser_fixture() -> Result<()> {
    ensure!(
        std::env::var_os("OPEN_SEO_PAGESPEED_API_KEY").is_none(),
        "Test host must not inherit a PageSpeed credential"
    );
    let endpoint = std::env::var("OPEN_SEO_TEST_ENRICHMENT_PROVIDER")?;
    let transport = LocalProviderTransport::new(&endpoint)?;
    let store = PathBuf::from(
        std::env::var_os("OPEN_SEO_TEST_ENRICHMENT_STORE")
            .ok_or_else(|| anyhow::anyhow!("Missing explicit owned enrichment test store"))?,
    );
    ensure!(
        store.is_absolute()
            && store.file_name().is_some()
            && !store
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
            && store.parent().is_some_and(|parent| parent.is_dir())
            && (!store.exists() || store.is_dir()),
        "Test host requires an absolute owned store beneath an existing directory"
    );
    // libtest may already have printed its test-name prefix without a newline.
    // Keep the normal server's post-bind readiness line complete and anchored.
    println!();
    println!(
        "Test-only enrichment provider: {}",
        json!({"kind":"local_synthetic_pagespeed_transport_v1","endpoint":endpoint,"real_google_provider":false,"credentials":false})
    );
    serve_with_mcp_files_ready_source_inner(
        "127.0.0.1:0",
        store,
        None,
        None,
        lifecycle::ShutdownSource::OwnedStdin,
        ServerContext {
            enrichment_transport: Some(crate::enrichment_app::TestTransport::new(transport)),
        },
    )
    .await
}
