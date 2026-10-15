//! Actual stored XML-source candidates in the caller's verified read snapshot.
//! No HTTP attempt total, synthetic capture, or durable association is inferred.
use super::{
    SitemapFindingDomain, SitemapFindingPosition as Position, SitemapFindingsError,
    SitemapSourceFindingEvidence as SourceEvidence, SitemapSourceFindingsScope,
};
use crate::sitemap_coverage::verified_read::{task_table, visit_tasks};
use crate::sitemap_ledger::verified_read::VerifiedRawRead;
use crate::sitemap_read_control::SitemapReadControl;
use crate::sitemap_source_store::verified_read::VerifiedSourceRead;
use crate::{SitemapCoverageTask, SitemapSourceKind};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{
    Serialize,
    ser::{Error as _, SerializeSeq},
};
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    io::{self, Write},
    ops::ControlFlow,
};

const TASK_INVENTORY_PROFILE: &str = "open-seo-spider/source-findings-current-task-inventory-v1";
const MAX_CONFIG_BYTES: i64 = 64 * 1024 * 1024;

pub(crate) struct PreparedSourceFindings<'a, 'conn> {
    connection: &'conn Connection,
    raw: Option<&'a VerifiedRawRead<'conn>>,
    source: Option<&'a VerifiedSourceRead<'conn>>,
    generation: Option<u64>,
    config_sha: Option<String>,
    tasks_exist: bool,
    scope: SitemapSourceFindingsScope,
    control: SitemapReadControl,
}

fn bounded_metadata(connection: &Connection, key: &str) -> Result<Option<String>> {
    let size: Option<i64> = connection
        .query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key=?1",
            [key],
            |r| r.get(0),
        )
        .optional()?;
    ensure!(
        size.is_none_or(|n| (0..=65_536).contains(&n)),
        "Source-findings metadata bound"
    );
    Ok(connection
        .query_row("SELECT value FROM metadata WHERE key=?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

// Verify actual bytes in place; no config reserialization and no 64MiB copy.
fn validate_bindings(
    connection: &Connection,
    generation: Option<u64>,
    config_sha: Option<&str>,
    control: &SitemapReadControl,
) -> Result<()> {
    let size: Option<i64> = connection
        .query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key='config'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    match (size, config_sha) {
        (None, None) => {}
        (Some(n), Some(expected)) => {
            ensure!(
                (1..=MAX_CONFIG_BYTES).contains(&n),
                "Source-findings saved config bound"
            );
            let mut statement = connection
                .prepare("SELECT CAST(value AS BLOB) FROM metadata WHERE key='config'")?;
            let mut rows = statement.query([])?;
            let bytes = rows.next()?.context("Source-findings config disappeared")?;
            let bytes = bytes.get_ref(0)?.as_blob()?;
            ensure!(
                i64::try_from(bytes.len())? == n && control.sha256(bytes)? == expected,
                "Source-findings actual config SHA differs"
            );
        }
        _ => anyhow::bail!("Source-findings config presence differs"),
    }
    let run: Option<crate::CrawlRunState> = bounded_metadata(connection, "run_state")?
        .map(|text| serde_json::from_str(&text))
        .transpose()?;
    ensure!(
        run.map(|r| r.generation).filter(|n| *n > 0) == generation,
        "Source-findings actual generation differs"
    );
    Ok(())
}

struct TaskSequence<'a> {
    connection: &'a Connection,
    exists: bool,
    count: &'a Cell<u64>,
    control: &'a SitemapReadControl,
}
impl Serialize for TaskSequence<'_> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(None)?;
        self.control
            .map_result(visit_tasks(
                self.connection,
                self.exists,
                self.control,
                |task| {
                    self.count.set(
                        self.count
                            .get()
                            .checked_add(1)
                            .context("Source task count overflow")?,
                    );
                    sequence
                        .serialize_element(task)
                        .map_err(|e| anyhow::anyhow!(e.to_string()))
                },
            ))
            .map_err(S::Error::custom)?;
        sequence.end()
    }
}
struct HashWriter<'a> {
    hash: Sha256,
    control: &'a SitemapReadControl,
}
impl Write for HashWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for chunk in bytes.chunks(65_536) {
            self.control.checkpoint().map_err(io::Error::other)?;
            self.hash.update(chunk);
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn prepare_source_findings<'a, 'conn>(
    connection: &'conn Connection,
    raw: Option<&'a VerifiedRawRead<'conn>>,
    source: Option<&'a VerifiedSourceRead<'conn>>,
    generation: Option<u64>,
    config_sha: Option<&str>,
    control: &SitemapReadControl,
) -> Result<PreparedSourceFindings<'a, 'conn>> {
    let _binding = control.bind_connection(connection)?;
    control.run(|| {
        ensure!(
            !connection.is_autocommit(),
            "Source findings need the caller's read transaction"
        );
        if let Some(raw) = raw {
            ensure!(
                std::ptr::eq(raw.connection(), connection),
                "Source findings/raw connection differs"
            );
        }
        if let Some(source) = source {
            ensure!(
                std::ptr::eq(source.connection(), connection) && raw.is_some(),
                "Source findings/source connection or raw presence differs"
            );
        }
        validate_bindings(connection, generation, config_sha, control)?;
        let tasks_exist = task_table(connection)?;
        let count = Cell::new(0u64);
        let mut writer = HashWriter {
            hash: Sha256::new(),
            control,
        };
        // Validate/hash the COMPLETE inventory even without a raw catalog, and
        // include robots so changes outside XML candidates still invalidate it.
        serde_json::to_writer(
            &mut writer,
            &(
                TASK_INVENTORY_PROFILE,
                generation,
                config_sha,
                TaskSequence {
                    connection,
                    exists: tasks_exist,
                    count: &count,
                    control,
                },
            ),
        )?;
        let inventory_sha = format!("{:x}", writer.hash.finalize());
        control.checkpoint()?;
        let scope = SitemapSourceFindingsScope {
            current_task_inventory_available: tasks_exist,
            current_task_inventory_sha256: tasks_exist.then_some(inventory_sha),
            current_frontier_task_count: tasks_exist.then_some(count.get()),
            measurement_history_available: source.is_some(),
            unknown_history: source.is_none_or(|s| s.summary().legacy_history_unknown)
                || raw.is_some_and(|r| r.summary().legacy_history_unknown),
            omission: source.and_then(|s| s.summary().omission),
            inherited: raw.is_some_and(|r| r.inherited().is_some()),
        };
        Ok(PreparedSourceFindings {
            connection,
            raw,
            source,
            generation,
            config_sha: config_sha.map(str::to_owned),
            tasks_exist,
            scope,
            control: control.clone(),
        })
    })
}

impl PreparedSourceFindings<'_, '_> {
    pub(crate) fn scope(&self) -> &SitemapSourceFindingsScope {
        &self.scope
    }

    fn measurement_exists(&self, sequence: u64) -> Result<bool> {
        if self.source.is_none() {
            return Ok(false);
        }
        Ok(self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sitemap_source_measurements WHERE sequence=?1)",
            [i64::try_from(sequence)?],
            |r| r.get(0),
        )?)
    }

    fn legacy_sql(&self, anchor: bool) -> String {
        let exclusion = if self.source.is_some() {
            "AND NOT EXISTS(SELECT 1 FROM sitemap_source_measurement_commits c WHERE c.document_id COLLATE BINARY=d.row_id COLLATE BINARY)"
        } else {
            ""
        };
        format!(
            "SELECT d.sequence,d.row_id FROM sitemap_documents d WHERE d.source_kind COLLATE BINARY='sitemap' {exclusion} AND d.sequence {} ?1 ORDER BY d.sequence",
            if anchor { "=" } else { ">" }
        )
    }
    fn legacy_exists(&self, sequence: u64) -> Result<bool> {
        if self.raw.is_none() {
            return Ok(false);
        }
        Ok(self
            .connection
            .query_row(&self.legacy_sql(true), [i64::try_from(sequence)?], |_| {
                Ok(())
            })
            .optional()?
            .is_some())
    }

    fn task_candidates_sql(&self) -> String {
        let measured = if self.source.is_some() {
            "LEFT JOIN (SELECT DISTINCT m.task_url COLLATE BINARY AS task_url FROM sitemap_source_measurements m CROSS JOIN binding b WHERE m.source_kind COLLATE BINARY='sitemap' AND m.generation=b.generation AND m.capture_config_sha256 COLLATE BINARY=b.config COLLATE BINARY) measured ON measured.task_url COLLATE BINARY=t.url COLLATE BINARY"
        } else {
            ""
        };
        // A legacy document has no capture-config field. It can suppress a
        // current task only when its generation and immutable initial config
        // prove that basis. Otherwise keep the current task explicitly unknown.
        let document_basis = self.raw.is_some_and(|raw| {
            self.config_sha.as_deref() == Some(raw.summary().config_sha256.as_str())
        });
        let documents = if document_basis {
            "LEFT JOIN (SELECT DISTINCT d.task_url COLLATE BINARY AS task_url FROM sitemap_documents d CROSS JOIN binding b WHERE d.source_kind COLLATE BINARY='sitemap' AND d.generation=b.generation) observed ON observed.task_url COLLATE BINARY=t.url COLLATE BINARY"
        } else {
            ""
        };
        let measured_filter = if self.source.is_some() {
            "AND measured.task_url IS NULL"
        } else {
            ""
        };
        let document_filter = if document_basis {
            "AND observed.task_url IS NULL"
        } else {
            ""
        };
        format!(
            "WITH binding(generation,config) AS (VALUES(?1,?2)) SELECT t.url,t.done FROM discovery_frontier t CROSS JOIN binding {measured} {documents} WHERE t.kind COLLATE BINARY='sitemap' {measured_filter} {document_filter} ORDER BY t.kind COLLATE BINARY,t.url COLLATE BINARY"
        )
    }

    fn visit_tasks_after(
        &self,
        after: u64,
        mut visitor: impl FnMut(Position, SourceEvidence) -> Result<ControlFlow<()>>,
    ) -> Result<bool> {
        if !self.tasks_exist {
            return Ok(true);
        }
        let mut ordinal = 0u64;
        // The shared validator already checked EVERY frontier task, including
        // robots and the late tail, before this capability exists. SQL does the
        // set subtraction; no per-task source-table scan or all-task Rust Vec.
        // SQLite may materialize distinct task identities; RSS/latency unproven.
        let mut statement = self.connection.prepare(&self.task_candidates_sql())?;
        let generation = self.generation.map(i64::try_from).transpose()?;
        let mut rows = statement.query(params![generation, self.config_sha.as_deref()])?;
        while let Some(row) = rows.next()? {
            self.control.checkpoint()?;
            ordinal = ordinal
                .checked_add(1)
                .context("Source task ordinal overflow")?;
            if ordinal <= after {
                continue;
            }
            let done: i64 = row.get(1)?;
            ensure!(matches!(done, 0 | 1), "Source task done projection differs");
            let task = SitemapCoverageTask {
                kind: SitemapSourceKind::SitemapXml,
                task_url: row.get(0)?,
                done: done == 1,
            };
            if visitor(
                Position::UnmeasuredSourceTask { ordinal },
                SourceEvidence::UnmeasuredTask { task },
            )?
            .is_break()
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn validate_after(&self, after: Option<&Position>) -> Result<()> {
        let Some(after) = after else {
            return Ok(());
        };
        after.validate(SitemapFindingDomain::SourceAttempt)?;
        let exists = match *after {
            Position::SourceMeasurement { sequence } => self.measurement_exists(sequence)?,
            Position::LegacySourceDocument { sequence } => self.legacy_exists(sequence)?,
            Position::UnmeasuredSourceTask { ordinal } => {
                let mut found = false;
                self.visit_tasks_after(ordinal - 1, |position, _| {
                    found = position == *after;
                    Ok(ControlFlow::Break(()))
                })?;
                found
            }
            Position::SavedPage { .. } => false,
        };
        if !exists {
            return Err(SitemapFindingsError::InvalidContinuation.into());
        }
        Ok(())
    }

    pub(crate) fn visit_after(
        &self,
        after: Option<&Position>,
        mut visitor: impl FnMut(Position, SourceEvidence) -> Result<ControlFlow<()>>,
    ) -> Result<bool> {
        let _binding = self.control.bind_connection(self.connection)?;
        self.control.run(|| {
            ensure!(
                !self.connection.is_autocommit(),
                "Source findings read transaction ended"
            );
            self.validate_after(after)?;
            if after.is_none_or(|p| matches!(p, Position::SourceMeasurement { .. }))
                && let (Some(source), Some(raw)) = (self.source, self.raw)
            {
                let lower = match after {
                    Some(Position::SourceMeasurement { sequence }) => *sequence,
                    _ => 0,
                };
                let exhausted = source.visit_measurements_after(
                    lower,
                    &self.control,
                    |record, association| {
                        let position = Position::SourceMeasurement {
                            sequence: record.sequence,
                        };
                        let committed_document = association
                            .as_ref()
                            .map(|a| {
                                raw.document_by_id(&a.document_id).and_then(|d| {
                                    d.context("Verified source association document disappeared")
                                })
                            })
                            .transpose()?
                            .map(Box::new);
                        let protocol = crate::assess_sitemap_source_protocol_limits(
                            &record.measurement,
                            &Default::default(),
                        )?;
                        visitor(
                            position,
                            SourceEvidence::Measured {
                                record: Box::new(record),
                                protocol,
                                association: association.map(Box::new),
                                committed_document,
                            },
                        )
                    },
                )?;
                if !exhausted {
                    return Ok(false);
                }
            }
            if !matches!(after, Some(Position::UnmeasuredSourceTask { .. }))
                && let Some(raw) = self.raw
            {
                let lower = match after {
                    Some(Position::LegacySourceDocument { sequence }) => *sequence,
                    _ => 0,
                };
                let mut statement = self.connection.prepare(&self.legacy_sql(false))?;
                let mut rows = statement.query([i64::try_from(lower)?])?;
                while let Some(row) = rows.next()? {
                    self.control.checkpoint()?;
                    let sequence = u64::try_from(row.get::<_, i64>(0)?)?;
                    let id = row.get_ref(1)?.as_str()?;
                    let document = raw
                        .document_by_id(id)?
                        .context("Verified legacy XML source disappeared")?;
                    ensure!(
                        document.sequence == sequence
                            && document.kind == SitemapSourceKind::SitemapXml,
                        "Legacy XML candidate projection differs"
                    );
                    if visitor(
                        Position::LegacySourceDocument { sequence },
                        SourceEvidence::LegacyDocument {
                            document: Box::new(document),
                        },
                    )?
                    .is_break()
                    {
                        return Ok(false);
                    }
                }
            }
            let lower = match after {
                Some(Position::UnmeasuredSourceTask { ordinal }) => *ordinal,
                _ => 0,
            };
            self.visit_tasks_after(lower, &mut visitor)
        })
    }
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
