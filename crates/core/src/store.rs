use crate::{CrawlConfig, CrawlReport, PageRecord};
use anyhow::{Context, bail};
use rusqlite::{Connection, OptionalExtension, params};
use std::{path::Path, sync::Mutex};

#[derive(Debug, Clone)]
pub(crate) struct QueueEntry {
    pub url: String,
    pub depth: usize,
    pub discovered_from: Option<String>,
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
            CREATE INDEX IF NOT EXISTS frontier_pending ON frontier(done,depth);")?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
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

    pub(crate) fn save_discovery(
        &self,
        entries: &[QueueEntry],
        complete: bool,
    ) -> anyhow::Result<()> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        for entry in entries {
            transaction.execute(
                "INSERT OR IGNORE INTO frontier(url,depth,discovered_from) VALUES(?1,?2,?3)",
                params![entry.url, entry.depth as i64, entry.discovered_from],
            )?;
        }
        if complete {
            transaction.execute(
                "INSERT OR REPLACE INTO metadata(key,value) VALUES('discovery_complete','true')",
                [],
            )?;
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
    ) -> anyhow::Result<()> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
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
