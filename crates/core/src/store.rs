use crate::{
    CrawlCheckpoint, CrawlConfig, CrawlOutcome, CrawlReport, CrawlRunState, CrawlStoreInspection,
    PageRecord,
};
use anyhow::{Context, bail};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone)]
pub(crate) struct QueueEntry {
    pub url: String,
    pub depth: usize,
    pub discovered_from: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DiscoveryKind {
    Robots,
    Sitemap,
}

impl DiscoveryKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Robots => "robots",
            Self::Sitemap => "sitemap",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct DiscoveryEntry {
    pub kind: DiscoveryKind,
    pub url: String,
}

/// SQLite WAL storage. A fetched page and its frontier state commit atomically.
/// Interrupted in-flight requests remain pending and are fetched on resume.
pub struct CrawlStore {
    connection: Mutex<Connection>,
}

impl CrawlStore {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;
            CREATE TABLE IF NOT EXISTS metadata(key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS frontier(url TEXT PRIMARY KEY, depth INTEGER NOT NULL, discovered_from TEXT, done INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS pages(url TEXT PRIMARY KEY, data TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS frontier_pending ON frontier(done,depth);
            CREATE TABLE IF NOT EXISTS discovery_frontier(kind TEXT NOT NULL,url TEXT NOT NULL,done INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(kind,url));
            CREATE INDEX IF NOT EXISTS discovery_frontier_pending ON discovery_frontier(done);")?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Opens existing storage without creating files, running migrations or
    /// changing journal configuration. Suitable for cold-process recovery.
    pub fn open_read_only(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn inspect(path: impl AsRef<Path>) -> anyhow::Result<CrawlStoreInspection> {
        Self::open_read_only(path)?.inspection()
    }

    pub fn inspection(&self) -> anyhow::Result<CrawlStoreInspection> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let config: Option<CrawlConfig> = metadata(&transaction, "config")?;
        let run_state: Option<CrawlRunState> = metadata(&transaction, "run_state")?;
        let checkpoint: Option<CrawlCheckpoint> = metadata(&transaction, "checkpoint")?;
        let completed = count(&transaction, "SELECT count(*) FROM pages")?;
        let pending = count(&transaction, "SELECT count(*) FROM frontier WHERE done=0")?;
        let discovery_pending = discovery_pending_count(&transaction)?;
        let discovery_complete =
            metadata_text(&transaction, "discovery_complete")?.as_deref() == Some("true");
        let report = metadata_text(&transaction, "report")?;
        let config_text = metadata_text(&transaction, "config")?;
        let checkpoint_valid = match (&run_state, &checkpoint, &report, &config_text) {
            (Some(run), Some(checkpoint), Some(report), Some(config)) => {
                run.generation == checkpoint.generation
                    && run.outcome == Some(checkpoint.outcome)
                    && sha256(report) == checkpoint.report_sha256
                    && sha256(config) == checkpoint.config_sha256
                    && completed == checkpoint.completed
                    && pending == checkpoint.pending
                    && discovery_pending == checkpoint.discovery_pending
                    && discovery_complete == checkpoint.discovery_complete
            }
            _ => false,
        };
        transaction.commit()?;
        Ok(CrawlStoreInspection {
            config,
            run_state,
            checkpoint,
            completed,
            pending,
            discovery_pending,
            discovery_complete,
            checkpoint_valid,
        })
    }

    pub fn checkpoint(&self) -> anyhow::Result<Option<CrawlCheckpoint>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        metadata(&connection, "checkpoint")
    }

    pub(crate) fn begin_run(&self, started_at: &str) -> anyhow::Result<u64> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let previous: Option<CrawlRunState> = metadata(&transaction, "run_state")?;
        let generation = previous
            .map_or(0, |run| run.generation)
            .checked_add(1)
            .context("Crawl generation exhausted")?;
        let state = CrawlRunState {
            generation,
            started_at: started_at.into(),
            finished_at: None,
            outcome: None,
        };
        put_metadata(&transaction, "run_state", &serde_json::to_string(&state)?)?;
        transaction.commit()?;
        Ok(generation)
    }

    pub(crate) fn prepare(&self, config: &CrawlConfig) -> anyhow::Result<bool> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let saved: Option<String> = connection
            .query_row("SELECT value FROM metadata WHERE key='config'", [], |row| {
                row.get(0)
            })
            .optional()?;
        if let Some(saved) = saved {
            let mut old: CrawlConfig = serde_json::from_str(&saved)?;
            // Explicit resume may add deterministic manual-seed provenance to
            // legacy configs. Read-only recovery never runs this migration.
            old.prepare_input_lists()?;
            // Operational limits can be changed when resuming, discovery semantics cannot.
            old.max_pages = config.max_pages;
            old.concurrency = config.concurrency;
            old.delay_ms = config.delay_ms;
            old.request_timeout_secs = config.request_timeout_secs;
            if serde_json::to_value(old)? != serde_json::to_value(config)? {
                bail!(
                    "Saved crawl uses a different crawl configuration; start a new database for a new scope or discovery policy"
                );
            }
            connection.execute(
                "UPDATE metadata SET value=?1 WHERE key='config'",
                [serde_json::to_string(config)?],
            )?;
            Ok(true)
        } else {
            connection.execute(
                "INSERT INTO metadata(key,value) VALUES('config',?1)",
                [serde_json::to_string(config)?],
            )?;
            Ok(false)
        }
    }

    pub fn config(&self) -> anyhow::Result<Option<CrawlConfig>> {
        let value: Option<String> = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?
            .query_row("SELECT value FROM metadata WHERE key='config'", [], |row| {
                row.get(0)
            })
            .optional()?;
        value
            .map(|value| serde_json::from_str(&value).map_err(Into::into))
            .transpose()
    }

    pub fn pending_count(&self) -> anyhow::Result<usize> {
        let count: i64 = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?
            .query_row("SELECT count(*) FROM frontier WHERE done=0", [], |row| {
                row.get(0)
            })?;
        Ok(count as usize)
    }

    pub(crate) fn enqueue(&self, entry: &QueueEntry) -> anyhow::Result<()> {
        self.connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?
            .execute(
                "INSERT OR IGNORE INTO frontier(url,depth,discovered_from) VALUES(?1,?2,?3)",
                params![entry.url, entry.depth as i64, entry.discovered_from],
            )?;
        Ok(())
    }

    pub(crate) fn discovery_complete(&self) -> anyhow::Result<bool> {
        let value: Option<String> = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?
            .query_row(
                "SELECT value FROM metadata WHERE key='discovery_complete'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value.as_deref() == Some("true"))
    }

    pub(crate) fn initialize_discovery(&self, entries: &[DiscoveryEntry]) -> anyhow::Result<()> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        if metadata_text(&transaction, "discovery_initialized")?.as_deref() == Some("true")
            || metadata_text(&transaction, "discovery_complete")?.as_deref() == Some("true")
        {
            return Ok(());
        }
        for entry in entries {
            enqueue_discovery(&transaction, entry)?;
        }
        put_metadata(&transaction, "discovery_initialized", "true")?;
        if entries.is_empty() {
            put_metadata(&transaction, "discovery_complete", "true")?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn discovery_entries(
        &self,
        pending_only: bool,
    ) -> anyhow::Result<Vec<DiscoveryEntry>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let query = if pending_only {
            "SELECT kind,url FROM discovery_frontier WHERE done=0 ORDER BY rowid"
        } else {
            "SELECT kind,url FROM discovery_frontier ORDER BY rowid"
        };
        let mut statement = connection.prepare(query)?;
        statement
            .query_map([], |row| {
                let kind: String = row.get(0)?;
                let kind = match kind.as_str() {
                    "robots" => DiscoveryKind::Robots,
                    "sitemap" => DiscoveryKind::Sitemap,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                Ok(DiscoveryEntry {
                    kind,
                    url: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .context("Read discovery frontier")
    }

    pub(crate) fn completed_sitemaps(&self) -> anyhow::Result<usize> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        count(
            &connection,
            "SELECT count(*) FROM discovery_frontier WHERE kind='sitemap' AND done=1",
        )
    }

    pub(crate) fn discovery_warnings(&self) -> anyhow::Result<Vec<String>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        Ok(metadata(&connection, "discovery_warnings")?.unwrap_or_default())
    }

    /// Commit one fully observed discovery unit and its output before admitting
    /// another unit. A pause cannot lose children already found in an index.
    pub(crate) fn complete_discovery(
        &self,
        entry: &DiscoveryEntry,
        pages: &[QueueEntry],
        children: &[DiscoveryEntry],
        warnings: &[String],
        generation: u64,
    ) -> anyhow::Result<()> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        require_generation(&transaction, generation)?;
        transaction.execute(
            "UPDATE discovery_frontier SET done=1 WHERE kind=?1 AND url=?2",
            params![entry.kind.as_str(), entry.url],
        )?;
        for page in pages {
            transaction.execute(
                "INSERT OR IGNORE INTO frontier(url,depth,discovered_from) VALUES(?1,?2,?3)",
                params![page.url, page.depth as i64, page.discovered_from],
            )?;
        }
        for child in children {
            enqueue_discovery(&transaction, child)?;
        }
        let mut saved: Vec<String> =
            metadata(&transaction, "discovery_warnings")?.unwrap_or_default();
        for warning in warnings {
            if !saved.contains(warning) {
                saved.push(warning.clone());
            }
        }
        put_metadata(
            &transaction,
            "discovery_warnings",
            &serde_json::to_string(&saved)?,
        )?;
        if discovery_pending_count(&transaction)? == 0 {
            put_metadata(&transaction, "discovery_complete", "true")?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Stores the result and every newly discovered URL in one transaction. This
    /// prevents an interrupted crawl from losing links found in a completed page.
    pub(crate) fn complete(
        &self,
        page: &PageRecord,
        discovered: &[QueueEntry],
        generation: u64,
    ) -> anyhow::Result<()> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        require_generation(&transaction, generation)?;
        transaction.execute(
            "INSERT OR REPLACE INTO pages(url,data) VALUES(?1,?2)",
            params![page.url, serde_json::to_string(page)?],
        )?;
        transaction.execute("UPDATE frontier SET done=1 WHERE url=?1", [&page.url])?;
        for entry in discovered {
            transaction.execute(
                "INSERT OR IGNORE INTO frontier(url,depth,discovered_from) VALUES(?1,?2,?3)",
                params![entry.url, entry.depth as i64, entry.discovered_from],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn pending(&self) -> anyhow::Result<Vec<QueueEntry>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let mut statement = connection.prepare(
            "SELECT url,depth,discovered_from FROM frontier WHERE done=0 ORDER BY depth,rowid",
        )?;
        statement
            .query_map([], |row| {
                Ok(QueueEntry {
                    url: row.get(0)?,
                    depth: row.get::<_, i64>(1)? as usize,
                    discovered_from: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .context("Read saved frontier")
    }

    pub fn pages(&self) -> anyhow::Result<Vec<PageRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let mut statement = connection.prepare("SELECT data FROM pages ORDER BY rowid")?;
        let values = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        values
            .into_iter()
            .map(|value| serde_json::from_str(&value).map_err(Into::into))
            .collect()
    }

    pub fn save_report(&self, report: &CrawlReport) -> anyhow::Result<()> {
        self.connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?
            .execute(
                "INSERT OR REPLACE INTO metadata(key,value) VALUES('report',?1)",
                [serde_json::to_string(report)?],
            )?;
        Ok(())
    }

    pub(crate) fn save_checkpoint(
        &self,
        report: &CrawlReport,
        outcome: CrawlOutcome,
        generation: u64,
    ) -> anyhow::Result<CrawlCheckpoint> {
        let report_text = serde_json::to_string(report)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let mut run: CrawlRunState =
            metadata(&transaction, "run_state")?.context("Crawl has no active generation")?;
        if run.generation != generation || run.outcome.is_some() {
            bail!("Stale or already finalized crawl generation cannot publish a checkpoint");
        }
        let previous: Option<CrawlCheckpoint> = metadata(&transaction, "checkpoint")?;
        let revision = previous
            .map_or(0, |checkpoint| checkpoint.revision)
            .checked_add(1)
            .context("Crawl checkpoint revision exhausted")?;
        let completed = count(&transaction, "SELECT count(*) FROM pages")?;
        let pending = count(&transaction, "SELECT count(*) FROM frontier WHERE done=0")?;
        if completed != report.summary.completed || pending != report.summary.pending {
            bail!("Report does not match the committed page frontier");
        }
        let config =
            metadata_text(&transaction, "config")?.context("Crawl has no saved configuration")?;
        let checkpoint = CrawlCheckpoint {
            generation,
            revision,
            outcome,
            created_at: chrono::Utc::now().to_rfc3339(),
            report_sha256: sha256(&report_text),
            config_sha256: sha256(&config),
            completed,
            pending,
            discovery_pending: discovery_pending_count(&transaction)?,
            discovery_complete: metadata_text(&transaction, "discovery_complete")?.as_deref()
                == Some("true"),
        };
        run.finished_at = Some(checkpoint.created_at.clone());
        run.outcome = Some(outcome);
        put_metadata(&transaction, "report", &report_text)?;
        put_metadata(
            &transaction,
            "checkpoint",
            &serde_json::to_string(&checkpoint)?,
        )?;
        put_metadata(&transaction, "run_state", &serde_json::to_string(&run)?)?;
        transaction.commit()?;
        Ok(checkpoint)
    }

    pub fn load_report(&self) -> anyhow::Result<Option<CrawlReport>> {
        let value: Option<String> = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?
            .query_row("SELECT value FROM metadata WHERE key='report'", [], |row| {
                row.get(0)
            })
            .optional()?;
        value
            .map(|value| serde_json::from_str(&value).map_err(Into::into))
            .transpose()
    }
}

fn metadata_text(connection: &Connection, key: &str) -> anyhow::Result<Option<String>> {
    Ok(connection
        .query_row("SELECT value FROM metadata WHERE key=?1", [key], |row| {
            row.get(0)
        })
        .optional()?)
}

fn metadata<T: DeserializeOwned>(connection: &Connection, key: &str) -> anyhow::Result<Option<T>> {
    metadata_text(connection, key)?
        .map(|value| serde_json::from_str(&value).map_err(Into::into))
        .transpose()
}

fn put_metadata(connection: &Connection, key: &str, value: &str) -> anyhow::Result<()> {
    connection.execute("INSERT INTO metadata(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, value])?;
    Ok(())
}

fn count(connection: &Connection, query: &str) -> anyhow::Result<usize> {
    let count: i64 = connection.query_row(query, [], |row| row.get(0))?;
    usize::try_from(count).context("Invalid stored row count")
}

fn discovery_pending_count(connection: &Connection) -> anyhow::Result<usize> {
    let exists: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='discovery_frontier')", [], |row| row.get(0))?;
    if exists {
        count(
            connection,
            "SELECT count(*) FROM discovery_frontier WHERE done=0",
        )
    } else {
        Ok(0)
    }
}

fn enqueue_discovery(connection: &Connection, entry: &DiscoveryEntry) -> anyhow::Result<()> {
    connection.execute(
        "INSERT OR IGNORE INTO discovery_frontier(kind,url) VALUES(?1,?2)",
        params![entry.kind.as_str(), entry.url],
    )?;
    Ok(())
}

fn sha256(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn require_generation(connection: &Connection, generation: u64) -> anyhow::Result<()> {
    let state: Option<CrawlRunState> = metadata(connection, "run_state")?;
    if !state.is_some_and(|state| state.generation == generation && state.outcome.is_none()) {
        bail!("Stale or already finalized crawl generation cannot commit work");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CrawlSummary;

    fn report(config: CrawlConfig, pending: usize) -> CrawlReport {
        CrawlReport {
            config,
            pages: Vec::new(),
            issues: Vec::new(),
            summary: CrawlSummary {
                pending,
                ..Default::default()
            },
            warnings: Vec::new(),
        }
    }

    #[test]
    fn read_only_legacy_inspection_does_not_create_or_migrate_files() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing.sqlite");
        assert!(CrawlStore::inspect(&missing).is_err());
        assert!(!missing.exists());
        let path = temp.path().join("legacy.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT); CREATE TABLE pages(url TEXT PRIMARY KEY,data TEXT); CREATE TABLE frontier(url TEXT PRIMARY KEY,depth INTEGER,discovered_from TEXT,done INTEGER DEFAULT 0);").unwrap();
        drop(connection);
        let before = std::fs::read(&path).unwrap();
        let inspected = CrawlStore::inspect(&path).unwrap();
        assert!(!inspected.checkpoint_valid);
        assert_eq!(inspected.discovery_pending, 0);
        assert!(inspected.run_state.is_none());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let read_only = CrawlStore::open_read_only(&path).unwrap();
        assert!(
            read_only
                .save_report(&report(CrawlConfig::default(), 0))
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let connection = Connection::open(&path).unwrap();
        let migrated: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='discovery_frontier')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!migrated);
    }

    #[test]
    fn explicit_resume_derives_legacy_manual_seed_provenance_without_changing_settings() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("old-config.sqlite");
        let store = CrawlStore::open(&path).unwrap();
        let mut old = CrawlConfig {
            seed_urls: vec!["https://example.org/a".into()],
            concurrency: 3,
            obey_robots: false,
            max_body_bytes: 12345,
            ..Default::default()
        };
        old.custom_headers
            .insert("X-Fixture".into(), "public-test-value".into());
        let mut raw = serde_json::to_value(&old).unwrap();
        raw.as_object_mut().unwrap().remove("input_imports");
        let raw_text = serde_json::to_string(&raw).unwrap();
        store
            .connection
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO metadata(key,value) VALUES('config',?1)",
                [&raw_text],
            )
            .unwrap();
        let inspected = CrawlStore::inspect(&path).unwrap();
        assert!(inspected.config.unwrap().input_imports.is_empty());
        assert_eq!(
            metadata_text(&store.connection.lock().unwrap(), "config").unwrap(),
            Some(raw_text)
        );
        let mut effective = old.clone();
        effective.prepare_input_lists().unwrap();
        assert!(store.prepare(&effective).unwrap());
        let saved = store.config().unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(&saved).unwrap(),
            serde_json::to_value(&effective).unwrap()
        );
        assert_eq!(saved.concurrency, old.concurrency);
        assert_eq!(saved.obey_robots, old.obey_robots);
        assert_eq!(saved.custom_headers, old.custom_headers);
        assert_eq!(saved.max_body_bytes, old.max_body_bytes);
        assert_eq!(saved.input_imports.len(), 1);
        assert_eq!(
            saved.input_imports[0].input_sha256,
            format!("{:x}", Sha256::digest(old.seed_urls.join("\n").as_bytes()))
        );
        assert_eq!(saved.input_imports[0].website_requests, 0);
        assert!(saved.input_imports[0].selected_sheet.is_none());
        let mut changed = effective;
        changed.user_agent = "Changed semantic agent".into();
        assert!(store.prepare(&changed).is_err());
    }

    #[test]
    fn checkpoint_is_atomic_readable_and_invalidated_by_a_new_running_generation() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("recover.sqlite");
        let store = CrawlStore::open(&path).unwrap();
        let config = CrawlConfig::default();
        store.prepare(&config).unwrap();
        let first = store.begin_run("first").unwrap();
        store
            .enqueue(&QueueEntry {
                url: "https://example.org/pending".into(),
                depth: 0,
                discovered_from: None,
            })
            .unwrap();
        store.initialize_discovery(&[]).unwrap();
        let snapshot = report(config, 1);
        let checkpoint = store
            .save_checkpoint(&snapshot, CrawlOutcome::Paused, first)
            .unwrap();
        assert_eq!((checkpoint.generation, checkpoint.revision), (1, 1));
        let inspected = CrawlStore::inspect(&path).unwrap();
        assert!(inspected.checkpoint_valid);
        assert_eq!(inspected.pending, 1);
        assert_eq!(
            inspected.run_state.unwrap().outcome,
            Some(CrawlOutcome::Paused)
        );
        let second = store.begin_run("second").unwrap();
        assert_eq!(second, 2);
        let reopened = CrawlStore::inspect(&path).unwrap();
        assert!(!reopened.checkpoint_valid);
        assert_eq!(reopened.run_state.unwrap().outcome, None);
        assert_eq!(reopened.checkpoint.unwrap().outcome, CrawlOutcome::Paused);
        assert!(
            store
                .save_checkpoint(&snapshot, CrawlOutcome::Completed, first)
                .is_err()
        );
        assert!(
            store
                .complete(
                    &PageRecord {
                        url: "https://example.org/pending".into(),
                        ..Default::default()
                    },
                    &[],
                    first
                )
                .is_err()
        );
        assert_eq!(store.pending_count().unwrap(), 1);
        let next = store
            .save_checkpoint(&snapshot, CrawlOutcome::Cancelled, second)
            .unwrap();
        assert_eq!(next.revision, 2);
        assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
        assert!(
            store
                .save_checkpoint(&snapshot, CrawlOutcome::Paused, second)
                .is_err()
        );
    }

    #[test]
    fn failed_discovery_commit_rolls_back_done_children_pages_and_warnings() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("failed-discovery.sqlite");
        let store = CrawlStore::open(&path).unwrap();
        store.prepare(&CrawlConfig::default()).unwrap();
        let generation = store.begin_run("running").unwrap();
        let entry = DiscoveryEntry {
            kind: DiscoveryKind::Sitemap,
            url: "https://example.org/index.xml".into(),
        };
        store
            .initialize_discovery(std::slice::from_ref(&entry))
            .unwrap();
        store.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_frontier BEFORE INSERT ON frontier BEGIN SELECT RAISE(ABORT,'fixture failed write'); END;").unwrap();
        assert!(
            store
                .complete_discovery(
                    &entry,
                    &[QueueEntry {
                        url: "https://example.org/blocked".into(),
                        depth: 0,
                        discovered_from: None
                    }],
                    &[DiscoveryEntry {
                        kind: DiscoveryKind::Sitemap,
                        url: "https://example.org/child.xml".into()
                    }],
                    &["fixture warning".into()],
                    generation
                )
                .is_err()
        );
        assert_eq!(store.discovery_entries(true).unwrap(), vec![entry]);
        assert_eq!(store.pending_count().unwrap(), 0);
        assert!(!store.discovery_complete().unwrap());
        assert!(store.discovery_warnings().unwrap().is_empty());
        assert!(!CrawlStore::inspect(&path).unwrap().checkpoint_valid);
    }

    #[test]
    fn failed_checkpoint_rolls_back_report_and_outcome_without_losing_previous_revision() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("failed-checkpoint.sqlite");
        let store = CrawlStore::open(&path).unwrap();
        let config = CrawlConfig::default();
        store.prepare(&config).unwrap();
        store.initialize_discovery(&[]).unwrap();
        let first = store.begin_run("first").unwrap();
        let saved = report(config, 0);
        let checkpoint = store
            .save_checkpoint(&saved, CrawlOutcome::Paused, first)
            .unwrap();
        let second = store.begin_run("second").unwrap();
        let mut changed = saved.clone();
        changed
            .warnings
            .push("failed new report must not persist".into());
        store.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_checkpoint BEFORE INSERT ON metadata WHEN NEW.key='checkpoint' BEGIN SELECT RAISE(ABORT,'fixture checkpoint failure'); END;").unwrap();
        assert!(
            store
                .save_checkpoint(&changed, CrawlOutcome::Paused, second)
                .is_err()
        );
        let inspection = CrawlStore::inspect(&path).unwrap();
        assert_eq!(inspection.checkpoint.unwrap(), checkpoint);
        assert_eq!(inspection.run_state.unwrap().outcome, None);
        assert!(!inspection.checkpoint_valid);
        assert!(store.load_report().unwrap().unwrap().warnings.is_empty());
        store
            .connection
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_checkpoint;")
            .unwrap();
        assert_eq!(
            store
                .save_checkpoint(&changed, CrawlOutcome::Paused, second)
                .unwrap()
                .revision,
            2
        );
        assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
    }
}
