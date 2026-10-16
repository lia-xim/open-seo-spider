//! CLI relay to the existing local saved-frontier resume operation.
//! Store ownership, quota authority and all ledger transitions remain in the API/Core.
use anyhow::{Context, Result, ensure};
use futures::StreamExt;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use uuid::Uuid;
use tokio::io::{AsyncRead, AsyncReadExt};

const OPTIONS_MAX_BYTES: usize = 64 * 1024;
const REPLY_MAX_BYTES: usize = 64 * 1024;

// Shared with the existing API extractor. Deserialize behavior/field defaults are unchanged.
#[derive(Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ResumeOptions {
    /// Omitted retains the saved limit; an explicit zero removes this operator limit.
    pub(crate) max_pages: Option<usize>,
    pub(crate) max_attempts: Option<usize>,
    pub(crate) concurrency: Option<usize>,
    pub(crate) delay_ms: Option<u64>,
    pub(crate) request_timeout_secs: Option<u64>,
    pub(crate) quota_increases: Option<open_seo_spider_core::CrawlQuotaIncreases>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResumeAccepted {
    id: Uuid,
    status: AcceptedStatus,
}

#[derive(Debug, Deserialize, Serialize)]
enum AcceptedStatus {
    #[serde(rename = "running")]
    Running,
}

fn endpoint(server: &str, id: Uuid) -> Result<Url> {
    ensure!(server.len() <= 256, "Resume requires a canonical HTTP loopback origin");
    // Validate the literal authority before URL parsing, which otherwise normalizes
    // noncanonical numeric IP spellings. No hostname resolution or proxy is allowed.
    let without_scheme = server
        .strip_prefix("http://")
        .context("Resume requires an HTTP loopback origin with an explicit port")?;
    let authority = without_scheme.strip_suffix('/').unwrap_or(without_scheme);
    let port = authority
        .strip_prefix("127.0.0.1:")
        .or_else(|| authority.strip_prefix("[::1]:"))
        .context("Resume accepts only literal127.0.0.1 or[::1], never a website")?;
    let parsed = port.parse::<u16>().context("Invalid local resume API port")?;
    ensure!(parsed > 0 && port == parsed.to_string(), "Invalid local resume API port");
    // Only a single optional root slash is accepted, never a path/query/fragment,
    // user information, whitespace, numeric alias, redirect or remote host.
    let canonical = format!("http://{authority}");
    ensure!(server == canonical || server == format!("{canonical}/"), "Resume requires a canonical HTTP loopback origin");
    Url::parse(&format!("{canonical}/api/crawls/{id}/resume")).context("Invalid local resume endpoint")
}

fn client(timeout: Duration) -> Result<Client> {
    Client::builder()
        .no_proxy()
        .retry(reqwest::retry::never())
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(timeout.min(Duration::from_secs(5)))
        .timeout(timeout)
        .build()
        .context("Cannot create local resume client")
}

async fn request(client: &Client, target: Url, id: Uuid, options: &ResumeOptions) -> Result<ResumeAccepted> {
    let body = serde_json::to_vec(options)?;
    ensure!(body.len() <= OPTIONS_MAX_BYTES, "Resume options exceed64KiB");
    let response = client
        .post(target)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .context("Local resume API request failed")?;
    let status = response.status();
    ensure!(response.content_length().is_none_or(|n| n <= REPLY_MAX_BYTES as u64), "Local resume API reply exceeds64KiB");
    let mut stream = response.bytes_stream();
    let mut raw = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Local resume API body did not complete")?;
        ensure!(chunk.len() <= REPLY_MAX_BYTES.saturating_sub(raw.len()), "Local resume API reply exceeds64KiB");
        raw.extend_from_slice(&chunk);
    }
    // A refusal stays a refusal; no success output, local store writes, retry or redirect.
    ensure!(status == reqwest::StatusCode::OK, "Local resume refused HTTP{}: {}", status.as_u16(), String::from_utf8_lossy(&raw));
    let accepted: ResumeAccepted = serde_json::from_slice(&raw).context("Invalid resume acceptance; expected this job in running state")?;
    ensure!(accepted.id == id, "Resume API accepted a different crawl job");
    Ok(accepted)
}

async fn read_options_bytes<R: AsyncRead + Unpin>(reader: R) -> Result<Vec<u8>> {
    // Limit the actual opened reader, not just an earlier path metadata observation.
    // The extra byte distinguishes a full64KiB document from oversized/growing input.
    let mut limited = reader.take((OPTIONS_MAX_BYTES + 1) as u64);
    let mut bytes = Vec::with_capacity(OPTIONS_MAX_BYTES + 1);
    limited
        .read_to_end(&mut bytes)
        .await
        .context("Cannot read resume options")?;
    ensure!(bytes.len() <= OPTIONS_MAX_BYTES, "Resume options exceed64KiB");
    Ok(bytes)
}

pub(crate) async fn execute(server: &str, id: Uuid, options: Option<&Path>) -> Result<ResumeAccepted> {
    let target = endpoint(server, id)?;
    // Parse the raw file with the SAME strict API DTO, preserving duplicate-field
    // refusal and exact generation/revision/hash. No metadata/Store is read here.
    let options = match options {
        Some(path) => {
            let opened = tokio::fs::File::open(path)
                .await
                .context("Cannot open resume options")?;
            let bytes = read_options_bytes(opened).await?;
            serde_json::from_slice::<ResumeOptions>(&bytes)
                .context("Invalid strict resume options JSON")?
        }
        None => ResumeOptions::default(),
    };
    request(&client(Duration::from_secs(60))?, target, id, &options).await
}

#[cfg(test)]
#[path = "crawl_resume_tests.rs"]
mod tests;
