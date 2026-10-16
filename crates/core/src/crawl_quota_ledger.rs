//! Durable distinct target membership. Every mutation is inside the caller's SQLite TX.
use crate::crawl_quotas::{CompiledCrawlQuotas, quota_hash};
use crate::{
    CRAWL_QUOTA_ENGINE, CrawlCheckpoint, CrawlConfig, CrawlQuotaBucketUsage, CrawlQuotaConfig,
    CrawlQuotaIncreases, CrawlQuotaSummary, CrawlRunState,
};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use url::Url;

const SCHEMA: &str = "
CREATE TABLE crawl_quota_members(kind TEXT NOT NULL,bucket TEXT NOT NULL,target TEXT NOT NULL,PRIMARY KEY(kind,bucket,target));
CREATE TABLE crawl_quota_buckets(kind TEXT NOT NULL,bucket TEXT NOT NULL,used INTEGER NOT NULL CHECK(used>0),PRIMARY KEY(kind,bucket));
CREATE TABLE crawl_quota_attempts(id INTEGER PRIMARY KEY AUTOINCREMENT,generation INTEGER NOT NULL,task_url TEXT NOT NULL,target TEXT NOT NULL,depth INTEGER NOT NULL,state TEXT NOT NULL CHECK(state IN('reserved','started','settled','unknown')),started INTEGER NOT NULL DEFAULT 0 CHECK(started IN(0,1)),response_status INTEGER);
CREATE UNIQUE INDEX crawl_quota_one_task_target ON crawl_quota_attempts(generation,task_url,target);
CREATE INDEX crawl_quota_attempt_generation ON crawl_quota_attempts(generation,state);
CREATE TABLE crawl_quota_deferred(task_url TEXT PRIMARY KEY,target TEXT NOT NULL,depth INTEGER NOT NULL,reasons TEXT NOT NULL,generation INTEGER NOT NULL);
";
const TABLES: [&str; 4] = [
    "crawl_quota_members",
    "crawl_quota_buckets",
    "crawl_quota_attempts",
    "crawl_quota_deferred",
];
const REQUIRED_SCHEMA: [(&str, &str, &str); 6] = [
    (
        "table",
        "crawl_quota_members",
        "CREATE TABLE crawl_quota_members(kind TEXT NOT NULL,bucket TEXT NOT NULL,target TEXT NOT NULL,PRIMARY KEY(kind,bucket,target))",
    ),
    (
        "table",
        "crawl_quota_buckets",
        "CREATE TABLE crawl_quota_buckets(kind TEXT NOT NULL,bucket TEXT NOT NULL,used INTEGER NOT NULL CHECK(used>0),PRIMARY KEY(kind,bucket))",
    ),
    (
        "table",
        "crawl_quota_attempts",
        "CREATE TABLE crawl_quota_attempts(id INTEGER PRIMARY KEY AUTOINCREMENT,generation INTEGER NOT NULL,task_url TEXT NOT NULL,target TEXT NOT NULL,depth INTEGER NOT NULL,state TEXT NOT NULL CHECK(state IN('reserved','started','settled','unknown')),started INTEGER NOT NULL DEFAULT 0 CHECK(started IN(0,1)),response_status INTEGER)",
    ),
    (
        "table",
        "crawl_quota_deferred",
        "CREATE TABLE crawl_quota_deferred(task_url TEXT PRIMARY KEY,target TEXT NOT NULL,depth INTEGER NOT NULL,reasons TEXT NOT NULL,generation INTEGER NOT NULL)",
    ),
    (
        "index",
        "crawl_quota_one_task_target",
        "CREATE UNIQUE INDEX crawl_quota_one_task_target ON crawl_quota_attempts(generation,task_url,target)",
    ),
    (
        "index",
        "crawl_quota_attempt_generation",
        "CREATE INDEX crawl_quota_attempt_generation ON crawl_quota_attempts(generation,state)",
    ),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Header {
    pub version: u32,
    pub revision: u64,
    pub sha256: String,
    pub config: CrawlQuotaConfig,
}

fn metadata<T: serde::de::DeserializeOwned>(c: &Connection, key: &str) -> Result<Option<T>> {
    let text: Option<String> = c
        .query_row("SELECT value FROM metadata WHERE key=?1", [key], |r| {
            r.get(0)
        })
        .optional()?;
    text.map(|x| serde_json::from_str(&x).map_err(Into::into))
        .transpose()
}
fn put<T: Serialize>(c: &Connection, key: &str, value: &T) -> Result<()> {
    c.execute("INSERT INTO metadata(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,serde_json::to_string(value)?])?;
    Ok(())
}
fn generation(c: &Connection, g: u64) -> Result<()> {
    let run: CrawlRunState = metadata(c, "run_state")?.context("Quota admission has no run")?;
    ensure!(
        run.generation == g && run.outcome.is_none(),
        "Stale quota generation"
    );
    Ok(())
}
/// SQLite integers are signed. Keep unsigned public counters only after an
/// explicit checked conversion; negative/corrupt cells are typed SQL errors.
pub(crate) fn sql_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}
fn count(c: &Connection, sql: &str) -> Result<u64> {
    Ok(c.query_row(sql, [], |r| sql_u64(r, 0))?)
}

fn verify_schema(c: &Connection) -> Result<()> {
    for (kind, name, expected) in REQUIRED_SCHEMA {
        let actual: Option<String> = c
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type=?1 AND name=?2",
                params![kind, name],
                |row| row.get(0),
            )
            .optional()?;
        ensure!(
            actual.as_deref() == Some(expected),
            "Missing or different canonical quota {kind} {name}"
        );
    }
    Ok(())
}

pub(crate) fn verify(c: &Connection, config: &CrawlConfig) -> Result<Option<Header>> {
    let h: Option<Header> = metadata(c, "crawl_quota_header")?;
    let mut present = 0usize;
    for name in TABLES {
        if c.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [name],
            |r| r.get::<_, bool>(0),
        )? {
            present += 1;
        }
    }
    let Some(cfg) = &config.crawl_quotas else {
        ensure!(
            h.is_none() && present == 0,
            "Unexpected quota ledger on legacy config"
        );
        return Ok(None);
    };
    let h = h.context("Enabled quotas have no durable ledger; not reconstructed from pages")?;
    ensure!(
        h.version == 1 && h.revision > 0 && present == TABLES.len(),
        "Incomplete quota ledger schema"
    );
    verify_schema(c)?;
    ensure!(
        h.config == *cfg && h.sha256 == quota_hash(cfg)?,
        "Quota config/header differ"
    );
    cfg.validate()?;
    let mut stmt=c.prepare("SELECT b.kind,b.bucket,b.used,COUNT(m.target) FROM crawl_quota_buckets b LEFT JOIN crawl_quota_members m ON m.kind=b.kind AND m.bucket=b.bucket GROUP BY b.kind,b.bucket ORDER BY b.kind,b.bucket")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            sql_u64(r, 2)?,
            sql_u64(r, 3)?,
        ))
    })?;
    for row in rows {
        let (k, b, u, n) = row?;
        ensure!(u == n && u > 0, "Quota membership/count differs");
        let cap = match k.as_str() {
            "depth" => {
                b.parse::<usize>()?;
                cfg.max_urls_per_depth
                    .context("Unexpected depth quota bucket")?
            }
            "host" => {
                ensure!(!b.is_empty(), "Empty quota host");
                cfg.max_urls_per_subdomain
                    .context("Unexpected host quota bucket")?
            }
            "pattern" => {
                cfg.url_pattern_quotas
                    .iter()
                    .find(|r| r.id == b)
                    .context("Unknown saved quota rule")?
                    .max_urls
            }
            _ => anyhow::bail!("Unknown quota dimension"),
        };
        ensure!(u <= cap, "Saved quota exceeds capacity");
    }
    ensure!(
        count(
            c,
            "SELECT COUNT(*) FROM crawl_quota_members m WHERE NOT EXISTS(SELECT 1 FROM crawl_quota_buckets b WHERE m.kind=b.kind AND m.bucket=b.bucket)"
        )? == 0,
        "Quota member has no counter"
    );
    ensure!(
        count(
            c,
            "SELECT COUNT(*) FROM crawl_quota_members m WHERE NOT EXISTS(SELECT 1 FROM crawl_quota_attempts a WHERE a.target=m.target)"
        )? == 0,
        "Quota membership has no reservation evidence"
    );
    Ok(Some(h))
}

pub(crate) fn prospective(
    c: &Connection,
    old: &CrawlConfig,
    inc: &CrawlQuotaIncreases,
) -> Result<CrawlConfig> {
    let h = verify(c, old)?.context("Legacy crawl has no quota profile")?;
    let run: CrawlRunState = metadata(c, "run_state")?.context("Quota resume has no run")?;
    let cp: CrawlCheckpoint =
        metadata(c, "checkpoint")?.context("Quota increase needs terminal checkpoint")?;
    ensure!(
        run.outcome.is_some()
            && run.generation == inc.expected_generation
            && cp.generation == run.generation
            && cp.outcome == run.outcome.context("Run outcome unavailable")?,
        "Stale or active quota checkpoint"
    );
    ensure!(
        inc.expected_profile_revision == h.revision && inc.expected_profile_sha256 == h.sha256,
        "Stale quota profile"
    );
    verify_checkpoint(c, old, run.generation)?;
    let mut result = old.clone();
    result.crawl_quotas = Some(h.config.increased(inc)?);
    Ok(result)
}

pub(crate) fn prepare(
    c: &Connection,
    old: Option<&CrawlConfig>,
    new: &CrawlConfig,
    inc: Option<&CrawlQuotaIncreases>,
) -> Result<()> {
    if let Some(old) = old {
        let h = verify(c, old)?;
        if let Some(inc) = inc {
            let expected = prospective(c, old, inc)?;
            ensure!(
                expected.crawl_quotas == new.crawl_quotas,
                "Quota override/config mismatch"
            );
            let mut h = h.context("Quota header missing")?;
            h.revision = h
                .revision
                .checked_add(1)
                .context("Quota revision overflow")?;
            h.config = new.crawl_quotas.clone().context("Quota profile removed")?;
            h.sha256 = quota_hash(&h.config)?;
            put(c, "crawl_quota_header", &h)?;
        } else {
            ensure!(
                old.crawl_quotas == new.crawl_quotas,
                "Saved quota profile changed without bound increase"
            );
        }
    } else {
        ensure!(
            inc.is_none(),
            "Quota increases require an existing checkpoint"
        );
        let mut legacy = new.clone();
        legacy.crawl_quotas = None;
        verify(c, &legacy)?;
        if let Some(cfg) = &new.crawl_quotas {
            cfg.validate()?;
            c.execute_batch(SCHEMA)?;
            put(
                c,
                "crawl_quota_header",
                &Header {
                    version: 1,
                    revision: 1,
                    sha256: quota_hash(cfg)?,
                    config: cfg.clone(),
                },
            )?;
        }
    }
    Ok(())
}

pub(crate) fn begin(c: &Connection, config: &CrawlConfig) -> Result<()> {
    if verify(c, config)?.is_some() {
        // Old owner was dropped before a new controlled invocation. Never refund.
        c.execute(
            "UPDATE crawl_quota_attempts SET state='unknown' WHERE state IN('reserved','started')",
            [],
        )?;
        c.execute("DELETE FROM crawl_quota_deferred", [])?;
        c.execute(
            "DELETE FROM metadata WHERE key='crawl_quota_checkpoint'",
            [],
        )?;
    }
    Ok(())
}

pub(crate) enum Admission {
    Reserved(i64),
    Deferred,
}

pub(crate) fn reserve(
    c: &Connection,
    compiled: &CompiledCrawlQuotas,
    g: u64,
    task: &str,
    target: &Url,
    depth: usize,
) -> Result<Admission> {
    generation(c, g)?;
    // A nonterminal/interrupted run has no terminal digest. Validate binary
    // identity/uniqueness semantics inside this actual reservation transaction,
    // before any membership lookup, capacity change or possible page dispatch.
    verify_schema(c)?;
    let h: Header = metadata(c, "crawl_quota_header")?.context("Quota header missing")?;
    ensure!(
        h.sha256 == compiled.sha256 && h.config == compiled.config,
        "Quota profile changed before dispatch"
    );
    let exists: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM frontier WHERE url=?1 AND done=0 AND depth=?2)",
        params![task, i64::try_from(depth)?],
        |r| r.get(0),
    )?;
    ensure!(exists, "Quota target is not a real pending frontier task");
    let memberships = compiled.memberships(target, depth)?;
    let mut missing = Vec::new();
    let mut reasons = Vec::new();
    for (kind, bucket, max) in memberships {
        let present:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM crawl_quota_members WHERE kind=?1 AND bucket=?2 AND target=?3)",params![kind,bucket,target.as_str()],|r|r.get(0))?;
        if !present {
            let used: u64 = c
                .query_row(
                    "SELECT used FROM crawl_quota_buckets WHERE kind=?1 AND bucket=?2",
                    params![kind, bucket],
                    |r| sql_u64(r, 0),
                )
                .optional()?
                .unwrap_or(0);
            if used >= max {
                reasons.push(format!("{kind}:{bucket}"));
            }
            missing.push((kind, bucket));
        }
    }
    if !reasons.is_empty() {
        c.execute("INSERT INTO crawl_quota_deferred(task_url,target,depth,reasons,generation) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(task_url) DO UPDATE SET target=excluded.target,depth=excluded.depth,reasons=excluded.reasons,generation=excluded.generation",params![task,target.as_str(),i64::try_from(depth)?,serde_json::to_string(&reasons)?,i64::try_from(g)?])?;
        return Ok(Admission::Deferred);
    }
    c.execute("INSERT INTO crawl_quota_attempts(generation,task_url,target,depth,state) VALUES(?1,?2,?3,?4,'reserved')",params![i64::try_from(g)?,task,target.as_str(),i64::try_from(depth)?])?;
    let id = c.last_insert_rowid();
    for (kind, bucket) in missing {
        c.execute(
            "INSERT INTO crawl_quota_members(kind,bucket,target) VALUES(?1,?2,?3)",
            params![kind, bucket, target.as_str()],
        )?;
        c.execute("INSERT INTO crawl_quota_buckets(kind,bucket,used) VALUES(?1,?2,1) ON CONFLICT(kind,bucket) DO UPDATE SET used=used+1",params![kind,bucket])?;
    }
    c.execute("DELETE FROM crawl_quota_deferred WHERE task_url=?1", [task])?;
    Ok(Admission::Reserved(id))
}

pub(crate) fn start(c: &Connection, g: u64, id: i64) -> Result<()> {
    generation(c, g)?;
    ensure!(c.execute("UPDATE crawl_quota_attempts SET state='started',started=1 WHERE id=?1 AND generation=?2 AND state='reserved' AND started=0",params![id,i64::try_from(g)?])?==1,"Quota attempt is not an owned unused reservation");
    Ok(())
}
pub(crate) fn response(c: &Connection, g: u64, id: i64, status: u16) -> Result<()> {
    generation(c, g)?;
    ensure!(c.execute("UPDATE crawl_quota_attempts SET response_status=?3 WHERE id=?1 AND generation=?2 AND state='started'",params![id,i64::try_from(g)?,status])?==1,"Quota response has no started owner");
    Ok(())
}
pub(crate) fn finish(c: &Connection, g: u64, ids: &[i64], unknown: bool) -> Result<()> {
    generation(c, g)?;
    for id in ids {
        ensure!(c.execute("UPDATE crawl_quota_attempts SET state=?3 WHERE id=?1 AND generation=?2 AND state IN('reserved','started')",params![id,i64::try_from(g)?,if unknown{"unknown"}else{"settled"}])?==1,"Quota task lost its owned attempt");
    }
    Ok(())
}
pub(crate) fn end_generation(c: &Connection, g: u64, config: &CrawlConfig) -> Result<()> {
    if config.crawl_quotas.is_some() {
        generation(c, g)?;
        c.execute("UPDATE crawl_quota_attempts SET state='unknown' WHERE generation=?1 AND state IN('reserved','started')",[i64::try_from(g)?])?;
    }
    Ok(())
}

pub(crate) fn summary(c: &Connection, config: &CrawlConfig) -> Result<Option<CrawlQuotaSummary>> {
    let Some(h) = verify(c, config)? else {
        return Ok(None);
    };
    let run: CrawlRunState =
        metadata(c, "run_state")?.context("Quota summary has no actual run")?;
    let mut bucket_rows = Vec::new();
    let mut omitted = 0u64;
    for kind in ["depth", "host", "pattern"] {
        let total: u64 = c.query_row(
            "SELECT COUNT(*) FROM crawl_quota_buckets WHERE kind=?1",
            [kind],
            |r| sql_u64(r, 0),
        )?;
        let mut stmt=c.prepare("SELECT bucket,used FROM crawl_quota_buckets WHERE kind=?1 ORDER BY CASE WHEN kind='depth' THEN CAST(bucket AS INTEGER) END,bucket LIMIT 64")?;
        let rows = stmt.query_map([kind], |r| Ok((r.get::<_, String>(0)?, sql_u64(r, 1)?)))?;
        let mut shown = 0u64;
        for row in rows {
            let (key, used) = row?;
            let limit = match kind {
                "depth" => h.config.max_urls_per_depth.context("Depth cap missing")?,
                "host" => h
                    .config
                    .max_urls_per_subdomain
                    .context("Host cap missing")?,
                _ => {
                    h.config
                        .url_pattern_quotas
                        .iter()
                        .find(|r| r.id == key)
                        .context("Pattern cap missing")?
                        .max_urls
                }
            };
            bucket_rows.push(CrawlQuotaBucketUsage {
                dimension: kind.into(),
                key,
                used,
                limit,
            });
            shown += 1;
        }
        omitted += total - shown;
    }
    let mut causes = BTreeSet::new();
    let mut stmt =
        c.prepare("SELECT reasons FROM crawl_quota_deferred ORDER BY task_url LIMIT 64")?;
    for row in stmt.query_map([], |r| r.get::<_, String>(0))? {
        let reasons: Vec<String> = serde_json::from_str(&row?)?;
        for r in reasons {
            if causes.len() < 64 {
                causes.insert(r);
            }
        }
    }
    Ok(Some(CrawlQuotaSummary {
        engine: CRAWL_QUOTA_ENGINE.into(),
        generation: run.generation,
        profile_revision: h.revision,
        profile_sha256: h.sha256,
        config: h.config,
        reserved_distinct_targets: count(
            c,
            "SELECT COUNT(DISTINCT target) FROM crawl_quota_attempts",
        )?,
        actual_started_attempts: count(
            c,
            "SELECT COUNT(*) FROM crawl_quota_attempts WHERE started=1",
        )?,
        settled_attempts: count(
            c,
            "SELECT COUNT(*) FROM crawl_quota_attempts WHERE state='settled'",
        )?,
        interrupted_or_unknown_attempts: count(
            c,
            "SELECT COUNT(*) FROM crawl_quota_attempts WHERE state='unknown'",
        )?,
        active_attempts: count(
            c,
            "SELECT COUNT(*) FROM crawl_quota_attempts WHERE state IN('reserved','started')",
        )?,
        deferred_targets: count(c, "SELECT COUNT(*) FROM crawl_quota_deferred")?,
        bucket_rows,
        bucket_rows_omitted: omitted,
        causes: causes.into_iter().collect(),
    }))
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LedgerCheckpoint {
    version: u32,
    generation: u64,
    summary_sha256: String,
    ledger_sha256: String,
}

/// Stream every actual persisted cell in a fixed table/row/column order. The
/// terminal binding covers identities, per-attempt states/status and deferred
/// reasons as well as the bounded summary; equal counters are insufficient.
fn ledger_hash(c: &Connection) -> Result<String> {
    let mut hash = Sha256::new();
    for (tag, sql) in [
        (
            "schema",
            "SELECT type,name,sql FROM sqlite_master WHERE name IN('crawl_quota_members','crawl_quota_buckets','crawl_quota_attempts','crawl_quota_deferred','crawl_quota_one_task_target','crawl_quota_attempt_generation') ORDER BY type,name",
        ),
        (
            "members",
            "SELECT kind,bucket,target FROM crawl_quota_members ORDER BY kind,bucket,target",
        ),
        (
            "buckets",
            "SELECT kind,bucket,used FROM crawl_quota_buckets ORDER BY kind,bucket",
        ),
        (
            "attempts",
            "SELECT id,generation,task_url,target,depth,state,started,response_status FROM crawl_quota_attempts ORDER BY id",
        ),
        (
            "deferred",
            "SELECT task_url,target,depth,reasons,generation FROM crawl_quota_deferred ORDER BY task_url",
        ),
    ] {
        hash.update((tag.len() as u64).to_be_bytes());
        hash.update(tag.as_bytes());
        let mut stmt = c.prepare(sql)?;
        let columns = stmt.column_count();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            hash.update([0xff]);
            for column in 0..columns {
                match row.get_ref(column)? {
                    rusqlite::types::ValueRef::Null => hash.update([0]),
                    rusqlite::types::ValueRef::Integer(value) => {
                        hash.update([1]);
                        hash.update(value.to_be_bytes());
                    }
                    rusqlite::types::ValueRef::Text(value) => {
                        hash.update([2]);
                        hash.update((value.len() as u64).to_be_bytes());
                        hash.update(value);
                    }
                    _ => anyhow::bail!("Invalid quota ledger cell type"),
                }
            }
        }
        hash.update([0xfe]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub(crate) fn bind_checkpoint(c: &Connection, config: &CrawlConfig, g: u64) -> Result<()> {
    if let Some(s) = summary(c, config)? {
        ensure!(
            s.generation == g && s.active_attempts == 0,
            "Quota checkpoint retains active attempts"
        );
        put(
            c,
            "crawl_quota_checkpoint",
            &LedgerCheckpoint {
                version: 1,
                generation: g,
                summary_sha256: quota_hash(&s)?,
                ledger_sha256: ledger_hash(c)?,
            },
        )?;
    }
    Ok(())
}
pub(crate) fn verify_checkpoint(c: &Connection, config: &CrawlConfig, g: u64) -> Result<()> {
    if let Some(s) = summary(c, config)? {
        let expected: LedgerCheckpoint =
            metadata(c, "crawl_quota_checkpoint")?.context("Quota checkpoint binding missing")?;
        ensure!(
            expected.version == 1
                && s.generation == g
                && s.active_attempts == 0
                && expected.generation == g
                && expected.summary_sha256 == quota_hash(&s)?
                && expected.ledger_sha256 == ledger_hash(c)?,
            "Quota terminal snapshot differs from checkpoint"
        );
    }
    Ok(())
}
