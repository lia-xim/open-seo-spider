//! Durable resource source decisions and immutable independent role grants.
use crate::resource_policy::*;
use anyhow::Context;
use anyhow::{Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

pub(crate) const SCHEMA:&str="
CREATE TABLE IF NOT EXISTS resource_occurrences(sequence INTEGER PRIMARY KEY,occurrence_id TEXT NOT NULL,evaluation TEXT NOT NULL,data TEXT NOT NULL,UNIQUE(occurrence_id,evaluation));
CREATE TABLE IF NOT EXISTS resource_grants(occurrence_id TEXT PRIMARY KEY,url TEXT NOT NULL,data TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS resource_tasks(url TEXT PRIMARY KEY,data TEXT NOT NULL,inherited INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS resource_transports(url TEXT PRIMARY KEY,data TEXT NOT NULL,payload_sha256 TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS resource_expansions(url TEXT PRIMARY KEY,processed_revision TEXT NOT NULL,pending INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS resource_expansions_pending ON resource_expansions(pending,url);
CREATE TABLE IF NOT EXISTS resource_sources(revision TEXT PRIMARY KEY,complete INTEGER NOT NULL);
";

fn read<T: serde::de::DeserializeOwned>(conn: &Connection, key: &str) -> Result<Option<T>> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM metadata WHERE key=?1", [key], |row| {
            row.get(0)
        })
        .optional()?;
    value
        .map(|value| serde_json::from_str(&value).map_err(Into::into))
        .transpose()
}
fn save<T: Serialize>(conn: &Connection, key: &str, value: &T) -> Result<()> {
    conn.execute("INSERT INTO metadata(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,serde_json::to_string(value)?])?;
    Ok(())
}
pub(crate) fn initialize(
    conn: &Connection,
    cfg: &ResourceDiscoveryConfig,
    history: Option<ResourceInheritedHistory>,
) -> Result<()> {
    let sha = cfg.sha256()?;
    if let Some(saved) = summary(conn)? {
        ensure!(saved.config_sha256 == sha, "Saved resource config changed");
        return Ok(());
    }
    save(
        conn,
        "resource_ledger",
        &ResourceLedgerSummary {
            engine: RESOURCE_DISCOVERY_ENGINE.into(),
            config_sha256: sha,
            revision: 0,
            decision_rows: 0,
            committed_bytes: 0,
            rolling_sha256: digest([]),
            completed_transports: 0,
            retained_responses: 0,
            pending_transports: 0,
            pending_expansions: 0,
            incomplete_sources: 0,
            complete: false,
            inherited_history: history,
        },
    )
}
pub(crate) fn summary(conn: &Connection) -> Result<Option<ResourceLedgerSummary>> {
    let Some(mut state) = read::<ResourceLedgerSummary>(conn, "resource_ledger")? else {
        return Ok(None);
    };
    // A cached permission revision avoids rehashing all stored tasks on events.
    state.complete = state.pending_transports == 0
        && state.pending_expansions == 0
        && state.incomplete_sources == 0
        && conn.query_row(
            "SELECT NOT EXISTS(SELECT 1 FROM discovery_frontier WHERE done=0 LIMIT 1)",
            [],
            |row| row.get::<_, bool>(0),
        )?
        && read::<bool>(conn, "discovery_complete")?.unwrap_or(false)
        && state
            .inherited_history
            .as_ref()
            .is_none_or(|history| history.rows_available);
    Ok(Some(state))
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedTask {
    permission: ResourceTaskPermission,
    revision: String,
}
pub(crate) fn permission(conn: &Connection, url: &str) -> Result<Option<ResourceTaskPermission>> {
    let data: Option<String> = conn
        .query_row(
            "SELECT data FROM resource_tasks WHERE url=?1",
            [url],
            |row| row.get(0),
        )
        .optional()?;
    data.map(|data| {
        let saved: SavedTask = serde_json::from_str(&data)?;
        saved.permission.validate()?;
        ensure!(
            saved.revision == saved.permission.revision()?,
            "Resource permission revision changed"
        );
        Ok(saved.permission)
    })
    .transpose()
}
pub(crate) fn save_permission(
    conn: &Connection,
    permission: &ResourceTaskPermission,
    inherited: bool,
) -> Result<()> {
    conn.execute("INSERT INTO resource_tasks(url,data,inherited) VALUES(?1,?2,?3) ON CONFLICT(url) DO UPDATE SET data=excluded.data",params![permission.fetch_url,
        serde_json::to_string(&SavedTask{permission:permission.clone(),revision:permission.revision()?})?,inherited])?;
    Ok(())
}

pub(crate) fn commit(
    conn: &Connection,
    rows: &mut [StoredResourceOccurrence],
    limits: &ResourceDiscoveryLimits,
    source: Option<(&str, bool)>,
) -> Result<Vec<String>> {
    limits.validate()?;
    ensure!(
        rows.len() <= limits.max_source_candidates,
        "Resource source candidate budget exceeded; source remains pending"
    );
    let mut state = read::<ResourceLedgerSummary>(conn, "resource_ledger")?
        .context("Resource ledger missing")?;
    let mut scheduled = Vec::new();
    let mut batch_bytes = 0usize;
    for row in rows {
        if conn.query_row("SELECT EXISTS(SELECT 1 FROM resource_occurrences WHERE occurrence_id=?1 AND evaluation=?2)",params![row.occurrence.occurrence_id,row.evaluation_revision],|row|row.get::<_,bool>(0))? {continue;}
        if let Some(grant) = &row.decision.grant {
            ensure!(
                grant.engine == state.engine
                    && grant.config_sha256 == state.config_sha256
                    && grant.occurrence_id == row.occurrence.occurrence_id,
                "Resource grant identity mismatch"
            );
            let existing: Option<String> = conn
                .query_row(
                    "SELECT data FROM resource_grants WHERE occurrence_id=?1",
                    [&grant.occurrence_id],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(existing) = existing {
                ensure!(
                    serde_json::from_str::<ResourceGrant>(&existing)? == *grant,
                    "Immutable resource grant changed"
                );
            } else {
                conn.execute(
                    "INSERT INTO resource_grants(occurrence_id,url,data) VALUES(?1,?2,?3)",
                    params![
                        grant.occurrence_id,
                        grant.fetch_url,
                        serde_json::to_string(grant)?
                    ],
                )?;
            }
            let (task, changed) = match permission(conn, &grant.fetch_url)? {
                Some(mut task) => {
                    row.decision.coalesced_into = task
                        .authorizing_grants
                        .first()
                        .map(|grant| grant.occurrence_id.clone());
                    let changed = task.merge(grant)?;
                    (task, changed)
                }
                None => (ResourceTaskPermission::new(grant.clone()), true),
            };
            if changed {
                let existing=conn.query_row("SELECT EXISTS(SELECT 1 FROM resource_transports p JOIN resource_expansions e ON p.url=e.url WHERE p.url=?1 AND e.pending=0)",[&grant.fetch_url],|row|row.get::<_,bool>(0))?;
                if existing {
                    state.pending_expansions += 1;
                }
                conn.execute(
                    "UPDATE resource_expansions SET pending=1 WHERE url=?1",
                    [&grant.fetch_url],
                )?;
                save_permission(conn, &task, false)?;
                scheduled.push(grant.fetch_url.clone());
            }
        }
        state.decision_rows = state
            .decision_rows
            .checked_add(1)
            .context("Resource sequence overflow")?;
        row.sequence = state.decision_rows as u64;
        ensure!(row.generation > 0, "Resource generation must be positive");
        let data = serde_json::to_string(row)?;
        ensure!(
            data.len() <= limits.max_occurrence_utf8_bytes,
            "Resource occurrence byte budget exceeded; source remains pending"
        );
        batch_bytes = batch_bytes
            .checked_add(data.len())
            .context("Resource source byte overflow")?;
        ensure!(
            batch_bytes <= limits.max_source_batch_bytes,
            "Resource source byte budget exceeded; source remains pending"
        );
        state.committed_bytes = state
            .committed_bytes
            .checked_add(data.len())
            .context("Resource ledger size overflow")?;
        ensure!(
            limits
                .max_ledger_bytes
                .is_none_or(|limit| state.committed_bytes <= limit),
            "Resource ledger byte budget exceeded; source remains pending"
        );
        state.rolling_sha256 = digest(format!("{}:{}:{}", state.rolling_sha256, data.len(), data));
        conn.execute("INSERT INTO resource_occurrences(sequence,occurrence_id,evaluation,data) VALUES(?1,?2,?3,?4)",params![i64::try_from(row.sequence)?,row.occurrence.occurrence_id,row.evaluation_revision,data])?;
    }
    if let Some((revision, complete)) = source
        && conn.execute(
            "INSERT OR IGNORE INTO resource_sources(revision,complete) VALUES(?1,?2)",
            params![revision, complete],
        )? == 1
        && !complete
    {
        state.incomplete_sources += 1;
    }
    state.revision = state
        .revision
        .checked_add(1)
        .context("Resource revision overflow")?;
    save(conn, "resource_ledger", &state)?;
    Ok(scheduled)
}
pub(crate) fn save_transport(
    conn: &Connection,
    page: &crate::PageRecord,
    generation: u64,
    config: &crate::CrawlConfig,
    limits: &ResourceDiscoveryLimits,
) -> Result<()> {
    let permission = page
        .resource_permission
        .as_ref()
        .context("Resource response permission missing")?;
    let projection = crate::resource_capture::private_projection(page, config, generation)?;
    let observation = ResourceTransportObservation {
        engine: RESOURCE_DISCOVERY_ENGINE.into(),
        requested_url: page.url.clone(),
        observed_generation: generation,
        retained_response: permission.store,
        projection,
    };
    let data = serde_json::to_string(&observation)?;
    ensure!(
        data.len() <= limits.max_source_batch_bytes,
        "Resource response metadata budget exceeded; source remains pending"
    );
    let exists = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM resource_transports WHERE url=?1)",
        [&page.url],
        |row| row.get::<_, bool>(0),
    )?;
    ensure!(
        !exists,
        "Resource transport was already completed; use metadata expansion"
    );
    conn.execute(
        "INSERT INTO resource_transports(url,data,payload_sha256) VALUES(?1,?2,?3)",
        params![page.url, data, digest(&data)],
    )?;
    let mut state = read::<ResourceLedgerSummary>(conn, "resource_ledger")?
        .context("Resource ledger missing")?;
    state.completed_transports += 1;
    let pending = self::permission(conn, &page.url)?
        .context("Resource task missing")?
        .revision()?
        != permission.revision()?;
    if pending {
        state.pending_expansions += 1;
    }
    save(conn, "resource_ledger", &state)?;
    conn.execute(
        "INSERT INTO resource_expansions(url,processed_revision,pending) VALUES(?1,?2,?3)",
        params![page.url, permission.revision()?, pending],
    )?;
    Ok(())
}
pub(crate) fn mark_expanded(
    conn: &Connection,
    url: &str,
    permission: &ResourceTaskPermission,
) -> Result<()> {
    let mut state = read::<ResourceLedgerSummary>(conn, "resource_ledger")?
        .context("Resource ledger missing")?;
    let pending = conn.query_row(
        "SELECT pending FROM resource_expansions WHERE url=?1",
        [url],
        |row| row.get::<_, bool>(0),
    )?;
    let remains = self::permission(conn, url)?
        .context("Resource task missing")?
        .revision()?
        != permission.revision()?;
    if pending && !remains {
        state.pending_expansions = state
            .pending_expansions
            .checked_sub(1)
            .context("Resource expansion counter mismatch")?;
    }
    if !pending && remains {
        state.pending_expansions += 1;
    }
    save(conn, "resource_ledger", &state)?;
    conn.execute(
        "UPDATE resource_expansions SET processed_revision=?2,pending=?3 WHERE url=?1",
        params![url, permission.revision()?, remains],
    )?;
    if permission.store {
        let data: String = conn.query_row(
            "SELECT data FROM resource_transports WHERE url=?1",
            [url],
            |row| row.get(0),
        )?;
        let mut observation: ResourceTransportObservation = serde_json::from_str(&data)?;
        if !observation.retained_response {
            // Current report inclusion can change without another fetch. The
            // private projection still declares its original unavailable body.
            observation.retained_response = true;
            let data = serde_json::to_string(&observation)?;
            conn.execute(
                "UPDATE resource_transports SET data=?2,payload_sha256=?3 WHERE url=?1",
                params![url, data, digest(&data)],
            )?;
        }
    }
    Ok(())
}
pub(crate) fn initialize_counts(conn: &Connection) -> Result<()> {
    let mut state = read::<ResourceLedgerSummary>(conn, "resource_ledger")?
        .context("Resource ledger missing")?;
    state.pending_transports = conn
        .query_row("SELECT count(*) FROM frontier WHERE done=0", [], |row| {
            row.get::<_, i64>(0)
        })?
        .try_into()?;
    state.retained_responses = conn
        .query_row("SELECT count(*) FROM pages", [], |row| row.get::<_, i64>(0))?
        .try_into()?;
    save(conn, "resource_ledger", &state)
}
pub(crate) fn frontier_committed(
    conn: &Connection,
    new_pending: usize,
    completed: bool,
    new_retained: bool,
) -> Result<()> {
    let mut state = read::<ResourceLedgerSummary>(conn, "resource_ledger")?
        .context("Resource ledger missing")?;
    state.pending_transports = state
        .pending_transports
        .checked_add(new_pending)
        .context("Resource pending count overflow")?;
    if completed {
        state.pending_transports = state
            .pending_transports
            .checked_sub(1)
            .context("Resource completed frontier counter mismatch")?;
    }
    if new_retained {
        state.retained_responses += 1;
    }
    save(conn, "resource_ledger", &state)
}
pub(crate) fn pending_expansion(conn: &Connection) -> Result<Option<crate::PageRecord>> {
    let value:Option<(String,String)>=conn.query_row("SELECT p.url,p.data FROM resource_expansions e JOIN resource_transports p ON e.url=p.url WHERE e.pending=1 ORDER BY e.url LIMIT 1",[],|row|Ok((row.get(0)?,row.get(1)?))).optional()?;
    value
        .map(|(url, data)| {
            let observation: ResourceTransportObservation = serde_json::from_str(&data)?;
            let retained: Option<String> = conn
                .query_row("SELECT data FROM pages WHERE url=?1", [&url], |row| {
                    row.get(0)
                })
                .optional()?;
            let mut page = if let Some(retained) = retained {
                let page: crate::PageRecord = serde_json::from_str(&retained)?;
                ensure!(
                    page.url == observation.projection.url
                        && page.fetched_at == observation.projection.fetched_at
                        && page.resource_source == observation.projection.resource_source,
                    "Retained resource source differs from original transport inventory"
                );
                page
            } else {
                observation.projection
            };
            page.resource_permission = permission(conn, &url)?;
            Ok(page)
        })
        .transpose()
}
pub(crate) fn page(
    conn: &Connection,
    after: Option<u64>,
    limit: usize,
    max_bytes: usize,
) -> Result<ResourceOccurrencePage> {
    ensure!(
        (1..=1000).contains(&limit),
        "Resource page limit must be 1..=1000"
    );
    let ledger = summary(conn)?;
    if ledger.is_none() {
        return Ok(ResourceOccurrencePage {
            snapshot: read_snapshot(conn)?,
            ledger,
            rows: vec![],
            next_sequence: None,
            has_more: false,
        });
    }
    let mut statement = conn.prepare(
        "SELECT data FROM resource_occurrences WHERE sequence>?1 ORDER BY sequence LIMIT ?2",
    )?;
    let mut rows = vec![];
    let mut bytes = 0usize;
    let mut has_more = false;
    for data in statement.query_map(
        params![
            i64::try_from(after.unwrap_or(0))?,
            i64::try_from(limit + 1)?
        ],
        |row| row.get::<_, String>(0),
    )? {
        let data = data?;
        if rows.len() == limit || bytes.saturating_add(data.len()) > max_bytes {
            ensure!(!rows.is_empty(), "Resource row exceeds page byte budget");
            has_more = true;
            break;
        }
        bytes += data.len();
        rows.push(serde_json::from_str::<StoredResourceOccurrence>(&data)?);
    }
    let next_sequence = rows.last().map(|row| row.sequence);
    Ok(ResourceOccurrencePage {
        snapshot: read_snapshot(conn)?,
        ledger,
        rows,
        next_sequence,
        has_more,
    })
}
pub(crate) fn transports(
    conn: &Connection,
    after: Option<&str>,
    limit: usize,
    max_bytes: usize,
) -> Result<ResourceTransportPage> {
    ensure!(
        (1..=1000).contains(&limit),
        "Resource transport page limit must be 1..=1000"
    );
    let ledger = summary(conn)?;
    if ledger.is_none() {
        return Ok(ResourceTransportPage {
            snapshot: read_snapshot(conn)?,
            ledger,
            rows: vec![],
            next_url: None,
            has_more: false,
        });
    }
    let mut statement =
        conn.prepare("SELECT data FROM resource_transports WHERE url>?1 ORDER BY url LIMIT ?2")?;
    let mut rows = vec![];
    let mut bytes = 0usize;
    let mut has_more = false;
    for data in statement.query_map(
        params![after.unwrap_or(""), i64::try_from(limit + 1)?],
        |row| row.get::<_, String>(0),
    )? {
        let data = data?;
        if rows.len() == limit || bytes.saturating_add(data.len()) > max_bytes {
            ensure!(
                !rows.is_empty(),
                "Resource transport exceeds page byte budget"
            );
            has_more = true;
            break;
        }
        bytes += data.len();
        rows.push(serde_json::from_str::<ResourceTransportObservation>(&data)?);
    }
    let next_url = rows.last().map(|row| row.requested_url.clone());
    Ok(ResourceTransportPage {
        snapshot: read_snapshot(conn)?,
        ledger,
        rows,
        next_url,
        has_more,
    })
}
pub(crate) fn verify(conn: &Connection, cfg: &crate::CrawlConfig) -> Result<()> {
    let Some(config) = cfg.resource_discovery.as_ref() else {
        return Ok(());
    };
    let Some(state) = summary(conn)? else {
        ensure!(
            conn.query_row(
                "SELECT NOT EXISTS(SELECT 1 FROM resource_tasks LIMIT 1)",
                [],
                |row| row.get::<_, bool>(0)
            )?,
            "Resource tasks exist without ledger"
        );
        return Ok(());
    };
    ensure!(
        state.engine == RESOURCE_DISCOVERY_ENGINE && state.config_sha256 == config.sha256()?,
        "Resource ledger config identity changed"
    );
    let generation =
        read::<crate::CrawlRunState>(conn, "run_state")?.map_or(0, |run| run.generation);
    let mut hash = digest([]);
    let mut bytes = 0usize;
    let mut count = 0usize;
    let mut statement = conn.prepare(
        "SELECT sequence,occurrence_id,evaluation,data FROM resource_occurrences ORDER BY sequence",
    )?;
    for data in statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })? {
        let (sequence, id, evaluation, data) = data?;
        let row: StoredResourceOccurrence = serde_json::from_str(&data)?;
        count += 1;
        ensure!(
            row.sequence == count as u64
                && row.sequence == u64::try_from(sequence)?
                && row.occurrence.occurrence_id == id
                && row.evaluation_revision == evaluation
                && row.generation > 0
                && row.generation <= generation,
            "Resource row identity/generation changed"
        );
        bytes = bytes
            .checked_add(data.len())
            .context("Resource ledger size overflow")?;
        hash = digest(format!("{hash}:{}:{data}", data.len()));
        if let Some(grant) = row.decision.grant {
            let saved: String = conn.query_row(
                "SELECT data FROM resource_grants WHERE occurrence_id=?1",
                [&grant.occurrence_id],
                |row| row.get(0),
            )?;
            ensure!(
                serde_json::from_str::<ResourceGrant>(&saved)? == grant,
                "Resource immutable grant differs from occurrence"
            );
        }
    }
    ensure!(
        count == state.decision_rows
            && bytes == state.committed_bytes
            && hash == state.rolling_sha256,
        "Resource ledger row digest/count changed"
    );
    for (sql, expected) in [
        (
            "SELECT count(*) FROM resource_transports",
            state.completed_transports,
        ),
        ("SELECT count(*) FROM pages", state.retained_responses),
        (
            "SELECT count(*) FROM frontier WHERE done=0",
            state.pending_transports,
        ),
        (
            "SELECT count(*) FROM resource_sources WHERE complete=0",
            state.incomplete_sources,
        ),
        (
            "SELECT count(*) FROM resource_expansions WHERE pending=1",
            state.pending_expansions,
        ),
    ] {
        ensure!(
            usize::try_from(conn.query_row(sql, [], |row| row.get::<_, i64>(0))?)? == expected,
            "Resource saved counter differs from committed rows"
        );
    }
    let saved_plan = read::<crate::RunPlan>(conn, "run_plan")?;
    let inherited_targets: std::collections::HashMap<_, _> = match &saved_plan {
        Some(crate::RunPlan::SelectedRecrawl(plan)) => plan
            .targets
            .iter()
            .filter_map(|target| {
                target
                    .resource_permission
                    .as_ref()
                    .map(|permission| (target.requested_url.as_str(), permission))
            })
            .collect(),
        _ => Default::default(),
    };
    let mut statement = conn.prepare("SELECT url,data,inherited FROM resource_tasks")?;
    for value in statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, bool>(2)?,
        ))
    })? {
        let (url, data, inherited) = value?;
        let saved: SavedTask = serde_json::from_str(&data)?;
        saved.permission.validate()?;
        ensure!(
            saved.permission.fetch_url == url
                && saved.permission.config_sha256 == state.config_sha256
                && saved.permission.revision()? == saved.revision,
            "Saved resource task binding changed"
        );
        if inherited {
            ensure!(
                inherited_targets.get(url.as_str()).copied() == Some(&saved.permission),
                "Inherited resource task differs from immutable selected target"
            );
        } else {
            for grant in &saved.permission.authorizing_grants {
                let data: String = conn.query_row(
                    "SELECT data FROM resource_grants WHERE occurrence_id=?1",
                    [&grant.occurrence_id],
                    |row| row.get(0),
                )?;
                ensure!(
                    serde_json::from_str::<ResourceGrant>(&data)? == *grant,
                    "Task permission differs from committed grant"
                );
            }
        }
    }
    let mut statement = conn.prepare("SELECT url,data,payload_sha256 FROM resource_transports")?;
    for value in statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })? {
        let (url, data, sha) = value?;
        let observation: ResourceTransportObservation = serde_json::from_str(&data)?;
        let retained: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM pages WHERE url=?1)",
            [&url],
            |row| row.get(0),
        )?;
        ensure!(
            digest(&data) == sha
                && observation.requested_url == url
                && observation.projection.url == url
                && observation.retained_response == retained
                && observation.observed_generation > 0
                && observation.observed_generation <= generation,
            "Resource transport projection binding changed"
        );
        let facts = observation
            .projection
            .resource_response
            .as_ref()
            .context("Resource projection lacks observed response facts")?;
        let source = observation
            .projection
            .resource_source
            .as_ref()
            .context("Resource projection lacks original source inventory")?;
        let (processed, pending): (String, bool) = conn.query_row(
            "SELECT processed_revision,pending FROM resource_expansions WHERE url=?1",
            [&url],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        ensure!(
            pending
                == (processed
                    != permission(conn, &url)?
                        .context("Resource task missing")?
                        .revision()?),
            "Resource expansion index differs from actual saved role revision"
        );
        let source_complete: bool = conn.query_row(
            "SELECT complete FROM resource_sources WHERE revision=?1",
            [&source.source_revision],
            |row| row.get(0),
        )?;
        ensure!(
            source.complete == source_complete,
            "Resource source completeness differs from its captured inventory"
        );
        ensure!(
            facts.engine == RESOURCE_DISCOVERY_ENGINE
                && facts.observed_generation == observation.observed_generation
                && !facts.entity_body_retained,
            "Resource response facts generation/retention changed"
        );
        ensure!(
            observation.projection.screenshot_base64.is_none()
                && observation.projection.network_requests.is_empty()
                && observation.projection.console_errors.is_empty()
                && observation.projection.custom_javascript_results.is_empty()
                && observation.projection.custom_extractions.is_empty(),
            "Resource private projection contains unretained body-derived payloads"
        );
        crate::content_binding::validate_native_content_captures(
            &observation.projection,
            cfg,
            Some(observation.observed_generation),
            true,
        )?;
    }
    ensure!(
        usize::try_from(
            conn.query_row("SELECT count(*) FROM resource_sources", [], |row| row
                .get::<_, i64>(0))?
        )? == state.completed_transports,
        "Resource source count differs from captured transports"
    );
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceTaskPermission {
    pub engine: String,
    pub config_sha256: String,
    pub fetch_url: String,
    pub store: bool,
    pub can_expand_navigation: bool,
    pub can_expand_resources: bool,
    pub can_expand_css: bool,
    pub can_expand_pdf: bool,
    /// At most six immutable grants suffice to prove each effective permission.
    /// Coalescing adds independent grants; it never mutates a check-only grant.
    pub authorizing_grants: Vec<ResourceGrant>,
}
impl ResourceTaskPermission {
    pub(crate) fn new(grant: ResourceGrant) -> Self {
        Self {
            engine: grant.engine.clone(),
            config_sha256: grant.config_sha256.clone(),
            fetch_url: grant.fetch_url.clone(),
            store: grant.store,
            can_expand_navigation: grant.can_expand_navigation,
            can_expand_resources: grant.can_expand_resources,
            can_expand_css: grant.can_expand_css,
            can_expand_pdf: grant.can_expand_pdf,
            authorizing_grants: vec![grant],
        }
    }
    pub(crate) fn merge(&mut self, grant: &ResourceGrant) -> Result<bool> {
        ensure!(
            grant.engine == RESOURCE_DISCOVERY_ENGINE
                && self.engine == grant.engine
                && self.config_sha256 == grant.config_sha256
                && self.fetch_url == grant.fetch_url,
            "Resource permission identity changed"
        );
        let changed = (grant.store && !self.store)
            || (grant.can_expand_navigation && !self.can_expand_navigation)
            || (grant.can_expand_resources && !self.can_expand_resources)
            || (grant.can_expand_css && !self.can_expand_css)
            || (grant.can_expand_pdf && !self.can_expand_pdf);
        if changed {
            self.store |= grant.store;
            self.can_expand_navigation |= grant.can_expand_navigation;
            self.can_expand_resources |= grant.can_expand_resources;
            self.can_expand_css |= grant.can_expand_css;
            self.can_expand_pdf |= grant.can_expand_pdf;
            self.authorizing_grants.push(grant.clone());
        }
        self.validate()?;
        Ok(changed)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.engine == RESOURCE_DISCOVERY_ENGINE
                && !self.authorizing_grants.is_empty()
                && self.authorizing_grants.len() <= 6,
            "Unknown or invalid resource task permission"
        );
        let mut store = false;
        let mut navigation = false;
        let mut resources = false;
        let mut css = false;
        let mut pdf = false;
        for grant in &self.authorizing_grants {
            ensure!(
                grant.engine == self.engine
                    && grant.config_sha256 == self.config_sha256
                    && grant.fetch_url == self.fetch_url,
                "Resource authorizing grant identity differs"
            );
            store |= grant.store;
            navigation |= grant.can_expand_navigation;
            resources |= grant.can_expand_resources;
            css |= grant.can_expand_css;
            pdf |= grant.can_expand_pdf;
        }
        ensure!(
            self.store == store
                && self.can_expand_navigation == navigation
                && self.can_expand_resources == resources
                && self.can_expand_css == css
                && self.can_expand_pdf == pdf,
            "Resource task permission is not derived from immutable grants"
        );
        Ok(())
    }
    pub(crate) fn revision(&self) -> Result<String> {
        self.validate()?;
        Ok(crate::resource_policy::digest(serde_json::to_vec(self)?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceObservationBasis {
    Current,
    Inherited,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredResourceOccurrence {
    pub sequence: u64,
    pub generation: u64,
    pub basis: ResourceObservationBasis,
    /// A source occurrence is stable; a later independent role grant creates a
    /// new decision evaluation, without rewriting the previous observation.
    pub evaluation_revision: String,
    pub occurrence: ResourceOccurrence,
    pub decision: ResourceDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLedgerSummary {
    pub engine: String,
    pub config_sha256: String,
    pub revision: u64,
    pub decision_rows: usize,
    pub committed_bytes: usize,
    pub rolling_sha256: String,
    pub completed_transports: usize,
    pub retained_responses: usize,
    pub pending_transports: usize,
    pub pending_expansions: usize,
    pub incomplete_sources: usize,
    pub complete: bool,
    /// A selected/imported report lacks the full original SQLite decision
    /// history, even if its parent summary declares zero rows.
    pub inherited_history: Option<ResourceInheritedHistory>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceInheritedHistory {
    pub parent_snapshot_sha256: String,
    pub declared_parent_summary: Option<Box<ResourceLedgerSummary>>,
    pub rows_available: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceOccurrencePage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<ResourceReadSnapshot>,
    pub ledger: Option<ResourceLedgerSummary>,
    pub rows: Vec<StoredResourceOccurrence>,
    pub next_sequence: Option<u64>,
    pub has_more: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceTransportObservation {
    pub engine: String,
    pub requested_url: String,
    pub observed_generation: u64,
    /// Current inclusion in the retained page/report table, not body retention.
    pub retained_response: bool,
    /// Extracted response/source metadata only, never an entity HTML/CSS body.
    pub projection: crate::PageRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceResponseFacts {
    pub engine: String,
    pub observed_generation: u64,
    pub retained_response: bool,
    pub entity_body_retained: bool,
    pub captured_entity_sha256: Option<String>,
    pub captured_entity_bytes: usize,
    pub captured_entity_complete: bool,
    pub decoded_source_sha256: Option<String>,
    pub decode_status: crate::ContentCaptureDecodeStatus,
    pub decode_encoding: Option<String>,
    pub metadata_projection_complete: bool,
    pub unretained_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceTransportPage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<ResourceReadSnapshot>,
    pub ledger: Option<ResourceLedgerSummary>,
    pub rows: Vec<ResourceTransportObservation>,
    pub next_url: Option<String>,
    pub has_more: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceReadSnapshot {
    pub crawl_config_sha256: String,
    pub generation: Option<u64>,
}
fn read_snapshot(conn: &Connection) -> Result<Option<ResourceReadSnapshot>> {
    read::<crate::CrawlConfig>(conn, "config")?
        .map(|config| {
            Ok(ResourceReadSnapshot {
                crawl_config_sha256: digest(serde_json::to_vec(&config)?),
                generation: read::<crate::CrawlRunState>(conn, "run_state")?
                    .map(|run| run.generation),
            })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grant(id: &str, nav: bool) -> ResourceGrant {
        ResourceGrant {
            engine: RESOURCE_DISCOVERY_ENGINE.into(),
            config_sha256: "cfg".into(),
            occurrence_id: id.into(),
            fetch_url: "https://example.test/image".into(),
            store: false,
            can_expand_navigation: nav,
            can_expand_resources: nav,
            can_expand_css: false,
            can_expand_pdf: false,
            policy_sha256: None,
            policy_occurrence_id: None,
        }
    }
    #[test]
    fn independent_grants_never_mutate_check_only_and_cannot_forge_union() {
        let check = grant("check", false);
        let mut task = ResourceTaskPermission::new(check.clone());
        assert!(task.merge(&grant("anchor", true)).unwrap());
        assert!(task.can_expand_navigation);
        assert_eq!(task.authorizing_grants[0], check);
        assert!(!check.can_expand_navigation);
        task.can_expand_css = true;
        assert!(task.validate().is_err());
    }
    #[test]
    fn duplicates_do_not_grow_permission_projection() {
        let mut task = ResourceTaskPermission::new(grant("initial", true));
        for index in 0..1000 {
            assert!(
                !task
                    .merge(&grant(&format!("duplicate{index}"), true))
                    .unwrap()
            );
        }
        assert_eq!(task.authorizing_grants.len(), 1);
        task.validate().unwrap();
    }
}
