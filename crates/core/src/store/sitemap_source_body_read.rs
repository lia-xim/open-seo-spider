//! Own stored-source descriptors and selected verified bytes, never a client path.
use super::*;
use crate::sitemap_source_archive as archive;
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapBodyReadContext {
    pub generation: u64,
    pub config_sha256: String,
    pub snapshot_sha256: String,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapBodyCatalogState {
    NotRecordedOrLegacyUnknown,
    InheritedArchiveNotCopied,
    RecordedMetadata,
}
#[derive(Debug, Clone, Serialize)]
pub struct SitemapBodyWindow {
    pub state: SitemapBodyCatalogState,
    pub context: Option<SitemapBodyReadContext>,
    pub summary: Option<crate::SitemapSourceArchiveSummary>,
    pub captures: Vec<crate::SitemapArchivedCapture>,
    pub next_sequence: Option<u64>,
    pub descriptor_consistency: &'static str,
    pub byte_availability: &'static str,
    pub download_available: Option<bool>,
    pub source_document_id: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SitemapVerifiedBodyCopy {
    pub context: SitemapBodyReadContext,
    pub capture: crate::SitemapArchivedCapture,
    pub body: crate::SitemapArchivedBody,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SitemapBodyReadError {
    StaleContext,
    Unavailable,
    MissingBody,
    InvalidRequest,
}
impl std::fmt::Display for SitemapBodyReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::StaleContext => "Stale source body snapshot",
            Self::Unavailable => "No own retained source body archive",
            Self::MissingBody => "Source body descriptor not found",
            Self::InvalidRequest => "Invalid source body request",
        })
    }
}
impl std::error::Error for SitemapBodyReadError {}

// These limits apply only to this new body-read seam, before invoking existing
// Store authorities. They are independent of archive JSON, App JSON and binary
// budgets. SQLite transcoding/scratch and total RSS are not bounded here.
const RUN_STATE_READ_CAP: i64 = 65_536;
const RUN_PLAN_READ_CAP: i64 = 67_108_864;
const CONFIG_READ_CAP: i64 = 67_108_864;
const PARENT_READ_CAP: i64 = 268_435_456;
fn metadata_text_preflight(c: &Connection, key: &str, cap: i64) -> anyhow::Result<()> {
    let pre: Option<(String, Option<i64>)> = c
        .query_row(
            "SELECT typeof(value),length(CAST(value AS BLOB)) FROM metadata WHERE key=?1",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((kind, bytes)) = pre else {
        return Ok(());
    };
    let bytes = bytes.context("Source body metadata has no byte length")?;
    anyhow::ensure!(
        kind == "text" && (1..=cap).contains(&bytes),
        "Source body {key} metadata type/byte budget exceeded"
    );
    let mut statement=c.prepare("SELECT value FROM metadata WHERE key=?1 AND typeof(value)='text' AND length(CAST(value AS BLOB))=?2")?;
    let mut rows = statement.query(params![key, bytes])?;
    let row = rows
        .next()?
        .context("Source body metadata changed within read snapshot")?;
    let text = row.get_ref(0)?.as_str()?;
    anyhow::ensure!(
        (1..=cap as usize).contains(&text.len()),
        "Source body {key} UTF8 metadata byte budget exceeded"
    );
    Ok(())
}
fn metadata_preflight(c: &Connection) -> anyhow::Result<()> {
    for (key, cap) in [
        ("run_state", RUN_STATE_READ_CAP),
        ("run_plan", RUN_PLAN_READ_CAP),
        ("operation_kind", 128),
        ("config", CONFIG_READ_CAP),
    ] {
        metadata_text_preflight(c, key, cap)?;
    }
    let exists: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='recrawl_base')",
        [],
        |r| r.get(0),
    )?;
    if exists {
        let pre:Option<(String,Option<i64>)>=c.query_row("SELECT typeof(snapshot),length(CAST(snapshot AS BLOB)) FROM recrawl_base WHERE id=1",[],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        if let Some((kind, bytes)) = pre {
            anyhow::ensure!(
                kind == "blob" && bytes.is_some_and(|n| (1..=PARENT_READ_CAP).contains(&n)),
                "Source body selected parent type/byte budget exceeded"
            );
        }
    }
    Ok(())
}

fn context(
    c: &Connection,
    snapshot: &crate::SitemapSourceArchiveSnapshot,
) -> anyhow::Result<SitemapBodyReadContext> {
    let run: crate::CrawlRunState =
        metadata(c, "run_state")?.context("Source body read lacks run generation")?;
    let bytes: i64 = c.query_row(
        "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key='config'",
        [],
        |r| r.get(0),
    )?;
    anyhow::ensure!(
        (1..=67_108_864).contains(&bytes),
        "Source body config exceeds bounded input"
    );
    let mut statement=c.prepare("SELECT value FROM metadata WHERE key='config' AND typeof(value)='text' AND length(CAST(value AS BLOB))=?1")?;
    let mut rows = statement.query([bytes])?;
    let row = rows.next()?.context("Source body config changed")?;
    let value = row.get_ref(0)?.as_str()?;
    anyhow::ensure!(
        value.len() as i64 == bytes,
        "Source body archive requires UTF-8 SQLite storage"
    );
    let config_sha256 = sha256(value);
    let source = crate::sitemap_source_store::load(c)?;
    let source_context = crate::sitemap_source_store::snapshot_context(source.as_ref())?;
    let snapshot_sha256 = archive::digest(serde_json::to_vec(&(
        "stored-source-body-read-v1",
        run.generation,
        &config_sha256,
        &snapshot.summary,
        source_context,
    ))?);
    Ok(SitemapBodyReadContext {
        generation: run.generation,
        config_sha256,
        snapshot_sha256,
    })
}
fn snapshot(
    c: &Connection,
) -> anyhow::Result<(
    SitemapBodyCatalogState,
    Option<crate::SitemapSourceArchiveSnapshot>,
    Option<SitemapBodyReadContext>,
)> {
    metadata_preflight(c)?;
    if matches!(load_run_plan(c)?, RunPlan::SelectedRecrawl(_)) {
        return Ok((
            SitemapBodyCatalogState::InheritedArchiveNotCopied,
            None,
            None,
        ));
    }
    archive::require_storage_for_existing(c)?;
    let saved = archive::load(c)?;
    match saved {
        Some(s) => {
            let ctx = context(c, &s)?;
            Ok((
                SitemapBodyCatalogState::RecordedMetadata,
                Some(s),
                Some(ctx),
            ))
        }
        None => Ok((
            SitemapBodyCatalogState::NotRecordedOrLegacyUnknown,
            None,
            None,
        )),
    }
}
impl CrawlStore {
    /// Metadata consistency only. Does not open/hash body files or fetch URLs.
    pub fn sitemap_source_body_window(
        &self,
        after: u64,
        limit: usize,
        expected: Option<&SitemapBodyReadContext>,
    ) -> anyhow::Result<SitemapBodyWindow> {
        self.sitemap_source_body_window_for_source(after, limit, expected, None)
    }
    pub fn sitemap_source_body_window_for_source(
        &self,
        after: u64,
        limit: usize,
        expected: Option<&SitemapBodyReadContext>,
        document_id: Option<&str>,
    ) -> anyhow::Result<SitemapBodyWindow> {
        if !(1..=200).contains(&limit) || after > 0 && expected.is_none() {
            return Err(SitemapBodyReadError::InvalidRequest.into());
        }
        if let Some(id) = document_id {
            archive::valid_sha(id)?;
        }
        let mut c = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let tx = c.transaction()?;
        let (state, saved, ctx) = snapshot(&tx)?;
        if let Some(expected) = expected
            && ctx.as_ref() != Some(expected)
        {
            return Err(SitemapBodyReadError::StaleContext.into());
        }
        let mut captures = Vec::new();
        let mut next_sequence = None;
        let summary = saved.as_ref().map(|s| s.summary.clone());
        if let Some(s) = saved {
            if after > s.summary.captures {
                return Err(SitemapBodyReadError::InvalidRequest.into());
            }
            for row in s.captures.into_iter().filter(|r| r.sequence > after) {
                if let Some(document) = document_id {
                    let matches:bool=tx.query_row("SELECT expected_document_id COLLATE BINARY=?2 COLLATE BINARY FROM sitemap_source_measurements WHERE measurement_id COLLATE BINARY=?1 COLLATE BINARY",params![&row.measurement_id,document],|r|r.get(0))?;
                    if !matches {
                        continue;
                    }
                }
                if captures.len() == limit {
                    next_sequence = captures
                        .last()
                        .map(|r: &crate::SitemapArchivedCapture| r.sequence);
                    break;
                }
                captures.push(row);
            }
        }
        tx.commit()?;
        Ok(SitemapBodyWindow {
            state,
            context: ctx,
            summary,
            captures,
            next_sequence,
            descriptor_consistency: "validated_in_snapshot",
            byte_availability: "not_checked",
            download_available: None,
            source_document_id: document_id.map(str::to_owned),
        })
    }
    /// Write only to a caller's private unadvertised sink. Return is not HTTP.
    /// Context is re-read after the copy TX closes; stale bytes are never ready.
    pub fn copy_sitemap_source_body(
        &self,
        id: &str,
        expected: &SitemapBodyReadContext,
        sink: &mut (impl Write + ?Sized),
        mut checkpoint: impl FnMut() -> anyhow::Result<()>,
    ) -> anyhow::Result<SitemapVerifiedBodyCopy> {
        self.copy_sitemap_source_body_for_source(id, expected, None, sink, &mut checkpoint)
    }
    pub fn copy_sitemap_source_body_for_source(
        &self,
        id: &str,
        expected: &SitemapBodyReadContext,
        document_id: Option<&str>,
        sink: &mut (impl Write + ?Sized),
        mut checkpoint: impl FnMut() -> anyhow::Result<()>,
    ) -> anyhow::Result<SitemapVerifiedBodyCopy> {
        archive::valid_sha(id)?;
        archive::valid_sha(&expected.config_sha256)?;
        archive::valid_sha(&expected.snapshot_sha256)?;
        if let Some(document) = document_id {
            archive::valid_sha(document)?;
        }
        checkpoint()?;
        let mut c = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let tx = c.transaction()?;
        let (_, saved, ctx) = snapshot(&tx)?;
        let saved = saved.ok_or(SitemapBodyReadError::Unavailable)?;
        if ctx.as_ref() != Some(expected) {
            return Err(SitemapBodyReadError::StaleContext.into());
        }
        let (capture, body) = saved
            .captures
            .iter()
            .find_map(|r| {
                r.bodies
                    .iter()
                    .find(|b| b.body_id == id)
                    .map(|b| (r.clone(), b.clone()))
            })
            .ok_or(SitemapBodyReadError::MissingBody)?;
        if let Some(document) = document_id {
            let matches:bool=tx.query_row("SELECT expected_document_id COLLATE BINARY=?2 COLLATE BINARY FROM sitemap_source_measurements WHERE measurement_id COLLATE BINARY=?1 COLLATE BINARY",params![&capture.measurement_id,document],|r|r.get(0))?;
            if !matches {
                return Err(SitemapBodyReadError::MissingBody.into());
            }
        }
        checkpoint()?;
        archive::files::copy_verified(&self.source_archive_path, &body, sink, &mut checkpoint)?;
        checkpoint()?;
        tx.commit()?;
        let check = c.transaction()?;
        let (_, _, current) = snapshot(&check)?;
        if current.as_ref() != Some(expected) {
            return Err(SitemapBodyReadError::StaleContext.into());
        }
        check.commit()?;
        checkpoint()?;
        Ok(SitemapVerifiedBodyCopy {
            context: expected.clone(),
            capture,
            body,
        })
    }
}
