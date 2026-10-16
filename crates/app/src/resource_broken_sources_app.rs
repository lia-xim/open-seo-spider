//! Offline saved-resource scope, kept separate from retained page-link sources.
use anyhow::{Context, Result, ensure};
use open_seo_spider_core::{
    CrawlStore, ResourceBrokenSourcesFilter, ResourceBrokenSourcesLimits,
    ResourceBrokenSourcesPage, ResourceBrokenSourcesRequest, TaskOwner,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Write},
    path::PathBuf,
};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub(crate) const REQUEST_BYTES: usize = 4096;
pub(crate) const JSON_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const CSV_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const SCOPE: &str = "verified_resource_ledger_occurrences";

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Request {
    pub filter: ResourceBrokenSourcesFilter,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting_mode: Option<open_seo_spider_core::ReportingMode>,
    pub limit: Option<usize>,
    pub after_sequence: Option<u64>,
    pub expected_snapshot_token: Option<String>,
    pub expected_generation: Option<u64>,
    pub limits: ResourceBrokenSourcesLimits,
}
impl Request {
    pub(crate) fn core(&self, id: Uuid, generation: u64) -> Result<ResourceBrokenSourcesRequest> {
        let limit = self.limit.unwrap_or(100);
        ensure!((1..=200).contains(&limit), "Page limit must be 1..=200");
        ensure!(
            self.expected_generation.is_none_or(|g| g == generation),
            "Crawl generation changed"
        );
        let expected_snapshot_token = self
            .expected_snapshot_token
            .as_ref()
            .map(|token| {
                ensure!(token.len() <= 256, "Snapshot token exceeds256 bytes");
                let prefix = format!("resource-sources-v1:{id}:{generation}:");
                let hash = token
                    .strip_prefix(&prefix)
                    .context("Snapshot belongs to another crawl or generation")?;
                ensure!(
                    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
                    "Invalid resource snapshot hash"
                );
                Ok::<_, anyhow::Error>(hash.to_owned())
            })
            .transpose()?;
        ensure!(
            self.after_sequence.is_none() || expected_snapshot_token.is_some(),
            "Continuation needs the preview snapshot token"
        );
        Ok(ResourceBrokenSourcesRequest {
            filter: self.filter,
            reporting_mode: self.reporting_mode,
            limit,
            after_sequence: self.after_sequence,
            expected_snapshot_token,
            limits: self.limits.clone(),
        })
    }
}
pub(crate) fn api_token(id: Uuid, generation: u64, core_token: &str) -> String {
    format!("resource-sources-v1:{id}:{generation}:{core_token}")
}
#[derive(Serialize)]
struct Envelope {
    scope: &'static str,
    job_id: Uuid,
    generation: u64,
    snapshot_token: Option<String>,
    page: ResourceBrokenSourcesPage,
    website_requests: usize,
}
pub(crate) fn page_bytes(
    id: Uuid,
    generation: u64,
    page: ResourceBrokenSourcesPage,
) -> Result<Vec<u8>> {
    let snapshot_token = page
        .snapshot
        .as_ref()
        .map(|s| api_token(id, generation, &s.snapshot_token));
    json_bytes(&Envelope {
        scope: SCOPE,
        job_id: id,
        generation,
        snapshot_token,
        page,
        website_requests: 0,
    })
}
pub(crate) fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut out = BoundedBytes {
        data: vec![],
        cap: JSON_BYTES,
    };
    serde_json::to_writer(&mut out, value)?;
    Ok(out.data)
}
struct BoundedBytes {
    data: Vec<u8>,
    cap: usize,
}
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.data
            .len()
            .checked_add(bytes.len())
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| io::Error::other("Resource sources JSON exceeds8MiB"))?;
        self.data.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(crate) fn bytes_sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Explicit SQLite input and strict Core request; all destinations must be new.
/// Page JSON and optional full-selection CSV are separate, snapshot-bound outputs.
pub(crate) async fn run_cli(
    store: PathBuf,
    request_path: Option<PathBuf>,
    output: PathBuf,
    csv_output: Option<PathBuf>,
) -> Result<()> {
    let source = tokio::fs::canonicalize(&store).await?;
    let input_request = if let Some(p) = &request_path {
        Some(tokio::fs::canonicalize(p).await?)
    } else {
        None
    };
    let mut destinations = std::collections::HashSet::new();
    for destination in std::iter::once(&output).chain(csv_output.iter()) {
        let resolved = crate::canonical_destination(destination).await?;
        let identity = crate::destination_identity(&resolved);
        ensure!(
            identity != crate::destination_identity(&source),
            "Output cannot replace the SQLite source"
        );
        ensure!(
            input_request
                .as_ref()
                .is_none_or(|p| identity != crate::destination_identity(p)),
            "Output cannot replace the request input"
        );
        for suffix in ["-wal", "-shm", "-journal"] {
            let sidecar = PathBuf::from(format!("{}{suffix}", source.display()));
            ensure!(
                identity != crate::destination_identity(&sidecar),
                "Output cannot use a SQLite sidecar name"
            );
        }
        ensure!(
            destinations.insert(identity),
            "JSON and CSV destinations must differ"
        );
        ensure!(
            !tokio::fs::try_exists(destination).await?,
            "Output must be a new file"
        );
    }
    let request: ResourceBrokenSourcesRequest = if let Some(p) = request_path {
        serde_json::from_slice(&crate::read_bounded(&p, REQUEST_BYTES).await?)?
    } else {
        Default::default()
    };
    ensure!(
        csv_output.is_none() || request.after_sequence.is_none(),
        "Full CSV cannot be combined with a continuation"
    );
    let want_csv = csv_output.is_some();
    let owner = TaskOwner::new("offline-resource-broken-sources-cli");
    let result = owner
        .run_blocking(move || -> Result<(Vec<u8>, Option<Vec<u8>>)> {
            let database = CrawlStore::open_read_only(source)?;
            let page = database.resource_broken_sources(&request)?;
            let csv = if want_csv {
                let snapshot = page
                    .snapshot
                    .as_ref()
                    .context("Native resource history unavailable")?;
                let mut export = request.clone();
                export.expected_snapshot_token = Some(snapshot.snapshot_token.clone());
                let result = database.export_resource_broken_sources_csv(&export)?;
                ensure!(result.bytes.len() <= CSV_BYTES, "Resource CSV exceeds64MiB");
                ensure!(
                    result.sha256 == bytes_sha(&result.bytes),
                    "CSV digest changed"
                );
                Some(result.bytes)
            } else {
                None
            };
            Ok((json_bytes(&page)?, csv))
        })
        .await;
    let drained = owner.shutdown_and_drain().await;
    let (json, csv) = result??;
    drained?;
    // These two new files are not a filesystem transaction. A later IO failure
    // can leave partial newly owned outputs; no existing input is overwritten.
    let mut json_file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .await?;
    let mut csv_file = if let Some(path) = csv_output {
        Some(
            tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .await?,
        )
    } else {
        None
    };
    json_file.write_all(&json).await?;
    json_file.sync_all().await?;
    if let (Some(file), Some(bytes)) = (csv_file.as_mut(), csv.as_ref()) {
        file.write_all(bytes).await?;
        file.sync_all().await?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "resource_broken_sources_app_tests.rs"]
mod tests;
