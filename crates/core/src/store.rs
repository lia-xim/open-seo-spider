use crate::{
    CrawlCheckpoint, CrawlConfig, CrawlOutcome, CrawlReport, CrawlRunState, CrawlStoreInspection,
    PageRecord, PreparedSelectedRecrawl, RecrawlTargetObservation, RecrawlTargetState, RunPlan,
    SelectedRecrawlOperation,
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
    pub admission: Option<crate::PolicyAdmission>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DiscoveryKind {
    Robots,
    Sitemap,
}

impl DiscoveryKind {
    pub(crate) fn as_str(self) -> &'static str {
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
            CREATE INDEX IF NOT EXISTS discovery_frontier_pending ON discovery_frontier(done);
            CREATE TABLE IF NOT EXISTS recrawl_base(id INTEGER PRIMARY KEY CHECK(id=1),snapshot BLOB NOT NULL);
            CREATE TABLE IF NOT EXISTS recrawl_pages(url TEXT PRIMARY KEY,ordinal INTEGER NOT NULL,payload_sha256 TEXT NOT NULL,inherited INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS recrawl_targets(url TEXT PRIMARY KEY,admitted_attempts INTEGER NOT NULL DEFAULT 0,last_admitted_generation INTEGER,committed_generation INTEGER,committed_payload_sha256 TEXT);")?;
        connection.execute_batch(crate::policy_ledger::SCHEMA)?;
        connection.execute_batch(crate::resource_ledger::SCHEMA)?;
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
        if let Some(config) = &config {
            crate::resource_ledger::verify(&transaction, config)?;
        }
        verify_saved_content_captures(&transaction)?;
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
        let plan = load_run_plan(&transaction)?;
        let policy_ledger = if let Some(policy) = config
            .as_ref()
            .and_then(|config| config.crawl_policy.as_ref())
        {
            crate::policy_ledger::verify(&transaction, &crate::CompiledCrawlPolicy::new(policy)?)?;
            crate::policy_ledger::summary(&transaction)?
        } else {
            None
        };
        if checkpoint_valid
            && checkpoint
                .as_ref()
                .is_some_and(|checkpoint| checkpoint.policy_ledger != policy_ledger)
        {
            bail!("Terminal policy ledger differs from checkpoint binding");
        }
        let operation = if matches!(plan, RunPlan::SelectedRecrawl(_)) {
            verify_recrawl_rows(&transaction, &plan)?;
            recrawl_operation(&transaction)?
        } else {
            None
        };
        if checkpoint_valid && let Some(operation) = &operation {
            let saved: CrawlReport = serde_json::from_str(
                report
                    .as_deref()
                    .context("Terminal selected checkpoint has no report")?,
            )?;
            if saved.operation.as_ref() != Some(operation) {
                bail!("Terminal selected operation differs from its checkpoint report");
            }
        }
        let resource_ledger = crate::resource_ledger::summary(&transaction)?;
        if checkpoint_valid
            && checkpoint
                .as_ref()
                .is_some_and(|checkpoint| checkpoint.resource_ledger != resource_ledger)
        {
            bail!("Terminal resource ledger differs from checkpoint binding");
        }
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
            operation,
            policy_ledger,
            resource_ledger,
        })
    }

    /// Read-only immutable plan inspection. Unknown engines, altered parent
    /// bytes, plan/config mismatches and invalid retained rows are rejected.
    pub fn run_plan(&self) -> anyhow::Result<RunPlan> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let plan = load_run_plan(&transaction)?;
        if matches!(plan, RunPlan::SelectedRecrawl(_)) {
            verify_recrawl_rows(&transaction, &plan)?;
        }
        transaction.commit()?;
        Ok(plan)
    }

    pub fn parent_snapshot_bytes(&self) -> anyhow::Result<Option<Vec<u8>>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        parent_snapshot(&connection)
    }

    pub fn recrawl_operation(&self) -> anyhow::Result<Option<SelectedRecrawlOperation>> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let plan = load_run_plan(&transaction)?;
        if matches!(plan, RunPlan::SelectedRecrawl(_)) {
            verify_recrawl_rows(&transaction, &plan)?;
        }
        let operation = recrawl_operation(&transaction)?;
        transaction.commit()?;
        Ok(operation)
    }

    /// Atomically initializes an EMPTY derived database. The exact immutable
    /// parent JSON and compact unselected payloads are retained; selected old
    /// rows are never inserted into this child's current observation set.
    pub fn initialize_selected_recrawl(
        &self,
        prepared: &PreparedSelectedRecrawl,
    ) -> anyhow::Result<()> {
        prepared.validate()?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        for table in [
            "metadata",
            "pages",
            "frontier",
            "discovery_frontier",
            "recrawl_base",
            "recrawl_pages",
            "recrawl_targets",
        ] {
            let exists: i64 =
                transaction.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })?;
            if exists != 0 {
                bail!("Selected recrawl requires a new empty database");
            }
        }
        let selected = prepared.plan.targets_set();
        put_metadata(
            &transaction,
            "config",
            &serde_json::to_string(&prepared.config)?,
        )?;
        put_metadata(
            &transaction,
            "run_plan",
            &serde_json::to_string(&RunPlan::SelectedRecrawl(prepared.plan.clone()))?,
        )?;
        put_metadata(&transaction, "operation_kind", "selected_recrawl")?;
        transaction.execute(
            "INSERT INTO recrawl_base(id,snapshot) VALUES(1,?1)",
            [&prepared.base_snapshot_bytes],
        )?;
        for (ordinal, page) in prepared.base_report.pages.iter().enumerate() {
            let text = serde_json::to_string(page)?;
            let inherited = !selected.contains(page.url.as_str());
            transaction.execute("INSERT INTO recrawl_pages(url,ordinal,payload_sha256,inherited) VALUES(?1,?2,?3,?4)", params![page.url, i64::try_from(ordinal)?, sha256(&text), inherited])?;
            if inherited {
                transaction.execute(
                    "INSERT INTO pages(url,data) VALUES(?1,?2)",
                    params![page.url, text],
                )?;
            }
        }
        for target in &prepared.plan.targets {
            transaction.execute(
                "INSERT INTO frontier(url,depth,discovered_from) VALUES(?1,?2,?3)",
                params![
                    target.requested_url,
                    i64::try_from(target.depth)?,
                    target.discovered_from
                ],
            )?;
            transaction.execute(
                "INSERT INTO recrawl_targets(url) VALUES(?1)",
                [&target.requested_url],
            )?;
            if let Some(admission) = &target.policy_admission {
                crate::policy_ledger::save_admission(&transaction, admission)?;
            }
            if let Some(permission) = &target.resource_permission {
                crate::resource_ledger::save_permission(&transaction, permission, true)?;
            }
        }
        if let Some(policy) = &prepared.config.crawl_policy {
            let policy = crate::CompiledCrawlPolicy::new(policy)?;
            crate::policy_ledger::initialize(
                &transaction,
                policy.policy_sha256(),
                prepared.base_report.policy_ledger.as_ref(),
                true,
            )?;
            put_metadata(&transaction, "policy_initialized", "true")?;
        }
        if let Some(config) = &prepared.config.resource_discovery {
            crate::resource_ledger::initialize(
                &transaction,
                config,
                Some(crate::ResourceInheritedHistory {
                    parent_snapshot_sha256: prepared.plan.parent.snapshot_sha256.clone(),
                    declared_parent_summary: prepared
                        .base_report
                        .resource_ledger
                        .clone()
                        .map(Box::new),
                    rows_available: false,
                }),
            )?;
        }
        if prepared.config.resource_discovery.is_some() {
            crate::resource_ledger::initialize_counts(&transaction)?;
        }
        put_metadata(&transaction, "discovery_initialized", "true")?;
        put_metadata(&transaction, "discovery_complete", "true")?;
        put_metadata(
            &transaction,
            "recrawl_invocation",
            &serde_json::to_string(&RecrawlInvocation {
                generation: 0,
                max_attempts: prepared.plan.default_max_attempts,
                admitted: 0,
                committed: 0,
            })?,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn start_recrawl_invocation(
        &self,
        generation: u64,
        override_budget: Option<usize>,
    ) -> anyhow::Result<Option<usize>> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        require_generation(&transaction, generation)?;
        let RunPlan::SelectedRecrawl(plan) =
            metadata::<RunPlan>(&transaction, "run_plan")?.context("Missing selected run plan")?
        else {
            bail!("Attempt budget applies only to selected recrawl");
        };
        let previous: Option<RecrawlInvocation> = metadata(&transaction, "recrawl_invocation")?;
        let max_attempts = match override_budget {
            Some(value) => (value > 0).then_some(value),
            None => previous.map_or(plan.default_max_attempts, |old| old.max_attempts),
        };
        put_metadata(
            &transaction,
            "recrawl_invocation",
            &serde_json::to_string(&RecrawlInvocation {
                generation,
                max_attempts,
                admitted: 0,
                committed: 0,
            })?,
        )?;
        transaction.commit()?;
        Ok(max_attempts)
    }

    pub(crate) fn admit_selected(&self, url: &str, generation: u64) -> anyhow::Result<()> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        require_generation(&transaction, generation)?;
        let mut invocation: RecrawlInvocation =
            metadata(&transaction, "recrawl_invocation")?.context("Missing selected invocation")?;
        if invocation.generation != generation
            || invocation
                .max_attempts
                .is_some_and(|limit| invocation.admitted >= limit)
        {
            bail!("Selected admission exceeds this invocation budget or generation");
        }
        let updated = transaction.execute("UPDATE recrawl_targets SET admitted_attempts=admitted_attempts+1,last_admitted_generation=?2 WHERE url=?1 AND committed_generation IS NULL AND (last_admitted_generation IS NULL OR last_admitted_generation<>?2) AND EXISTS(SELECT 1 FROM frontier WHERE frontier.url=?1 AND done=0)", params![url,i64::try_from(generation)?])?;
        if updated != 1 {
            bail!("Selected admission is not a pending target");
        }
        invocation.admitted = invocation
            .admitted
            .checked_add(1)
            .context("Selected admission count overflow")?;
        put_metadata(
            &transaction,
            "recrawl_invocation",
            &serde_json::to_string(&invocation)?,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn recrawl_operation_for_runner(
        &self,
    ) -> anyhow::Result<Option<SelectedRecrawlOperation>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        recrawl_operation(&connection)
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
        verify_saved_content_captures(&transaction)?;
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
        verify_saved_content_captures(&connection)?;
        if let RunPlan::SelectedRecrawl(plan) = load_run_plan(&connection)? {
            let bytes = parent_snapshot(&connection)?
                .context("Selected recrawl has no immutable parent")?;
            crate::recrawl::validate_saved_plan(&plan, &bytes, config)?;
            verify_recrawl_rows(&connection, &RunPlan::SelectedRecrawl(plan))?;
        }
        let saved: Option<String> = connection
            .query_row("SELECT value FROM metadata WHERE key='config'", [], |row| {
                row.get(0)
            })
            .optional()?;
        if let Some(saved) = saved {
            let mut old: CrawlConfig = serde_json::from_str(&saved)?;
            crate::resource_ledger::verify(&connection, &old)?;
            if let Some(policy) = &old.crawl_policy {
                crate::policy_ledger::verify(
                    &connection,
                    &crate::CompiledCrawlPolicy::new(policy)?,
                )?;
            }
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

    pub fn resource_ledger_summary(&self) -> anyhow::Result<Option<crate::ResourceLedgerSummary>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        crate::resource_ledger::summary(&connection)
    }
    pub fn resource_occurrences(
        &self,
        after: Option<u64>,
        limit: usize,
    ) -> anyhow::Result<crate::ResourceOccurrencePage> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let config = metadata::<CrawlConfig>(&transaction, "config")?;
        if let Some(config) = &config {
            verify_resource_read_context(&transaction, config)?;
        }
        let limits = config
            .and_then(|config| config.resource_discovery_limits)
            .unwrap_or_default();
        limits.validate()?;
        let page = crate::resource_ledger::page(
            &transaction,
            after,
            limit,
            limits.max_source_batch_bytes,
        )?;
        transaction.commit()?;
        Ok(page)
    }
    pub fn resource_transports(
        &self,
        after_url: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<crate::ResourceTransportPage> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let config = metadata::<CrawlConfig>(&transaction, "config")?;
        if let Some(config) = &config {
            verify_resource_read_context(&transaction, config)?;
        }
        let limits = config
            .and_then(|config| config.resource_discovery_limits)
            .unwrap_or_default();
        limits.validate()?;
        let page = crate::resource_ledger::transports(
            &transaction,
            after_url,
            limit,
            limits.max_source_batch_bytes,
        )?;
        transaction.commit()?;
        Ok(page)
    }
    pub fn visit_resource_occurrences(
        &self,
        limit: usize,
        mut visitor: impl FnMut(&crate::ResourceOccurrencePage) -> anyhow::Result<()>,
    ) -> anyhow::Result<Option<crate::ResourceLedgerSummary>> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let config = metadata::<CrawlConfig>(&transaction, "config")?;
        if let Some(config) = &config {
            verify_resource_read_context(&transaction, config)?;
        }
        let limits = config
            .and_then(|config| config.resource_discovery_limits)
            .unwrap_or_default();
        limits.validate()?;
        let summary = crate::resource_ledger::summary(&transaction)?;
        let mut after = None;
        loop {
            let page = crate::resource_ledger::page(
                &transaction,
                after,
                limit,
                limits.max_source_batch_bytes,
            )?;
            visitor(&page)?;
            if !page.has_more {
                break;
            }
            after = page.next_sequence;
        }
        transaction.commit()?;
        Ok(summary)
    }
    pub(crate) fn resource_permission(
        &self,
        url: &str,
    ) -> anyhow::Result<Option<crate::ResourceTaskPermission>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        crate::resource_ledger::permission(&connection, url)
    }
    pub(crate) fn resource_expansion(&self) -> anyhow::Result<Option<PageRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        crate::resource_ledger::pending_expansion(&connection)
    }
    pub(crate) fn initialize_resources(
        &self,
        config: &crate::ResourceDiscoveryConfig,
        limits: &crate::ResourceDiscoveryLimits,
        generation: u64,
    ) -> anyhow::Result<()> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        require_generation(&transaction, generation)?;
        crate::resource_ledger::initialize(&transaction, config, None)?;
        let mut entries = Vec::new();
        let mut statement =
            transaction.prepare("SELECT url FROM frontier WHERE done=0 ORDER BY rowid")?;
        for url in statement.query_map([], |row| row.get::<_, String>(0))? {
            entries.push(url?);
        }
        drop(statement);
        let revision = format!("initial:{}", config.sha256()?);
        let mut rows = Vec::new();
        for (ordinal, url) in entries.iter().enumerate() {
            if crate::resource_ledger::permission(&transaction, url)?.is_some() {
                continue;
            }
            let occurrence = crate::ResourceOccurrence {
                source_complete: true,
                source_unit_revision: revision.clone(),
                occurrence_id: crate::resource_policy::occurrence_id(&revision, ordinal),
                source_revision: revision.clone(),
                source_ordinal: ordinal,
                requested_source_url: url.clone(),
                source_document_url: url.clone(),
                raw_value: url.clone(),
                decoded_value: None,
                resolved_url: Some(url.clone()),
                reference_kind: crate::ResourceReferenceKind::InitialInput,
                classification_basis: "admitted_initial_frontier".into(),
                declared_mime: None,
                dom_location: None,
                byte_start: None,
                byte_end: None,
                attributes: Default::default(),
                nofollow: false,
                observation_error: None,
            };
            let admission = crate::policy_ledger::saved_admission(
                &transaction,
                crate::PolicyTaskRole::PageDocument,
                url,
            )?;
            let traverse = admission
                .as_ref()
                .is_none_or(|admission| admission.can_expand);
            let grant = crate::ResourceGrant {
                engine: crate::RESOURCE_DISCOVERY_ENGINE.into(),
                config_sha256: config.sha256()?,
                occurrence_id: occurrence.occurrence_id.clone(),
                fetch_url: url.clone(),
                reference_kind: config
                    .contract
                    .as_ref()
                    .map(|_| crate::ResourceReferenceKind::InitialInput),
                store: true,
                can_expand_navigation: traverse,
                can_expand_resources: traverse,
                can_expand_css: traverse && config.css.stylesheets,
                can_expand_pdf: traverse && config.navigation.pdf_annotations.fetch,
                policy_sha256: admission.as_ref().map(|row| row.policy_sha256.clone()),
                policy_occurrence_id: admission.map(|row| row.occurrence_id),
            };
            rows.push(crate::StoredResourceOccurrence {
                sequence: 0,
                generation,
                basis: crate::ResourceObservationBasis::Current,
                evaluation_revision: revision.clone(),
                occurrence,
                decision: crate::ResourceDecision {
                    reason: "initial_admitted".into(),
                    grant: Some(grant),
                    coalesced_into: None,
                    qualification: None,
                },
            });
        }
        if !rows.is_empty() {
            crate::resource_ledger::commit(&transaction, &mut rows, limits, None)?;
        }
        crate::resource_ledger::initialize_counts(&transaction)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn policy_ledger_summary(&self) -> anyhow::Result<Option<crate::PolicyLedgerSummary>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        crate::policy_ledger::summary(&connection)
    }

    pub fn policy_occurrences(
        &self,
        after_sequence: Option<u64>,
        limit: usize,
    ) -> anyhow::Result<crate::PolicyOccurrencePage> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let config: Option<CrawlConfig> = metadata(&transaction, "config")?;
        let bytes = config
            .and_then(|config| config.live_policy_limits)
            .unwrap_or_default()
            .max_page_bytes;
        let page = crate::policy_ledger::page(&transaction, after_sequence, limit, bytes)?;
        transaction.commit()?;
        Ok(page)
    }

    /// One snapshot across every page. For exports concurrent with a writer,
    /// use a separate open_read_only store; visitor must not reenter this store.
    pub fn visit_policy_occurrences(
        &self,
        limit: usize,
        mut visitor: impl FnMut(&crate::PolicyOccurrencePage) -> anyhow::Result<()>,
    ) -> anyhow::Result<Option<crate::PolicyLedgerSummary>> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let config: Option<CrawlConfig> = metadata(&transaction, "config")?;
        if let Some(policy) = config
            .as_ref()
            .and_then(|config| config.crawl_policy.as_ref())
        {
            crate::policy_ledger::verify(&transaction, &crate::CompiledCrawlPolicy::new(policy)?)?;
        }
        let bytes = config
            .and_then(|config| config.live_policy_limits)
            .unwrap_or_default()
            .max_page_bytes;
        let mut cursor = None;
        let mut snapshot = None;
        loop {
            let page = crate::policy_ledger::page(&transaction, cursor, limit, bytes)?;
            if snapshot.is_none() {
                snapshot = page.ledger.clone();
            }
            visitor(&page)?;
            if !page.has_more {
                break;
            }
            cursor = page.next_sequence;
        }
        transaction.commit()?;
        Ok(snapshot)
    }

    pub(crate) fn task_policy(
        &self,
        role: crate::PolicyTaskRole,
        url: &str,
    ) -> anyhow::Result<Option<crate::PolicyAdmission>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        crate::policy_ledger::saved_admission(&connection, role, url)
    }

    pub(crate) fn initialize_policy(
        &self,
        policy: &crate::CompiledCrawlPolicy,
        rows: &mut [crate::PolicyOccurrence],
        limits: &crate::LiveCrawlPolicyLimits,
        generation: u64,
    ) -> anyhow::Result<()> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        require_generation(&transaction, generation)?;
        if metadata_text(&transaction, "policy_initialized")?.as_deref() == Some("true") {
            crate::policy_ledger::verify(&transaction, policy)?;
            return Ok(());
        }
        crate::policy_ledger::initialize(&transaction, policy.policy_sha256(), None, false)?;
        crate::policy_ledger::commit(&transaction, rows, generation, limits, None)?;
        for row in rows {
            if let Some(url) = &row.scheduled_fetch_url {
                match row.task_role {
                    crate::PolicyTaskRole::PageDocument => {
                        transaction.execute(
                            "INSERT INTO frontier(url,depth,discovered_from) VALUES(?1,0,NULL)",
                            [url],
                        )?;
                    }
                    crate::PolicyTaskRole::SitemapDocument => {
                        enqueue_discovery(
                            &transaction,
                            &DiscoveryEntry {
                                kind: DiscoveryKind::Sitemap,
                                url: url.clone(),
                            },
                        )?;
                    }
                    crate::PolicyTaskRole::ObservationOnly => {}
                }
            }
        }
        put_metadata(&transaction, "policy_initialized", "true")?;
        transaction.commit()?;
        Ok(())
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
        if discovery_pending_count(&transaction)? == 0 {
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
    pub(crate) fn complete_discovery_with_policy(
        &self,
        entry: &DiscoveryEntry,
        rows: &mut [crate::PolicyOccurrence],
        source: (&str, bool),
        warnings: &[String],
        generation: u64,
        limits: &crate::LiveCrawlPolicyLimits,
    ) -> anyhow::Result<(Vec<QueueEntry>, Vec<DiscoveryEntry>)> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        require_generation(&transaction, generation)?;
        crate::policy_ledger::commit(&transaction, rows, generation, limits, Some(source))?;
        let mut pages = Vec::new();
        let mut children = Vec::new();
        for row in rows {
            if let Some(url) = &row.scheduled_fetch_url {
                match row.task_role {
                    crate::PolicyTaskRole::PageDocument => {
                        transaction.execute(
                            "INSERT INTO frontier(url,depth,discovered_from) VALUES(?1,0,?2)",
                            params![url, entry.url],
                        )?;
                        pages.push(QueueEntry {
                            url: url.clone(),
                            depth: 0,
                            discovered_from: Some(entry.url.clone()),
                            admission: crate::policy_ledger::saved_admission(
                                &transaction,
                                row.task_role,
                                url,
                            )?,
                        });
                    }
                    crate::PolicyTaskRole::SitemapDocument => {
                        let child = DiscoveryEntry {
                            kind: DiscoveryKind::Sitemap,
                            url: url.clone(),
                        };
                        enqueue_discovery(&transaction, &child)?;
                        children.push(child);
                    }
                    crate::PolicyTaskRole::ObservationOnly => {}
                }
            }
        }
        transaction.execute(
            "UPDATE discovery_frontier SET done=1 WHERE kind=?1 AND url=?2",
            params![entry.kind.as_str(), entry.url],
        )?;
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
        Ok((pages, children))
    }

    pub(crate) fn complete(
        &self,
        page: &PageRecord,
        discovered: &[QueueEntry],
        generation: u64,
    ) -> anyhow::Result<()> {
        self.complete_impl(page, discovered, generation, None)
            .map(|_| ())
    }

    pub(crate) fn complete_with_policy(
        &self,
        page: &PageRecord,
        rows: &mut [crate::PolicyOccurrence],
        generation: u64,
        limits: &crate::LiveCrawlPolicyLimits,
    ) -> anyhow::Result<Vec<QueueEntry>> {
        self.complete_impl(page, &[], generation, Some((rows, limits)))
    }

    pub(crate) fn complete_resources(
        &self,
        page: &PageRecord,
        rows: &mut [crate::StoredResourceOccurrence],
        policy_rows: &mut [crate::PolicyOccurrence],
        generation: u64,
        expansion: bool,
    ) -> anyhow::Result<Vec<QueueEntry>> {
        let config = self.config()?.context("Saved resource config missing")?;
        let limits = config.resource_discovery_limits.clone().unwrap_or_default();
        let policy_limits = config.live_policy_limits.clone().unwrap_or_default();
        self.complete_transaction(
            page,
            &[],
            generation,
            config
                .crawl_policy
                .as_ref()
                .map(|_| (policy_rows, &policy_limits)),
            Some((rows, &limits, expansion)),
        )
    }

    fn complete_impl(
        &self,
        page: &PageRecord,
        discovered: &[QueueEntry],
        generation: u64,
        policy: Option<(
            &mut [crate::PolicyOccurrence],
            &crate::LiveCrawlPolicyLimits,
        )>,
    ) -> anyhow::Result<Vec<QueueEntry>> {
        self.complete_transaction(page, discovered, generation, policy, None)
    }
    #[allow(clippy::too_many_arguments)]
    fn complete_transaction(
        &self,
        page: &PageRecord,
        discovered: &[QueueEntry],
        generation: u64,
        policy: Option<(
            &mut [crate::PolicyOccurrence],
            &crate::LiveCrawlPolicyLimits,
        )>,
        resources: Option<(
            &mut [crate::StoredResourceOccurrence],
            &crate::ResourceDiscoveryLimits,
            bool,
        )>,
    ) -> anyhow::Result<Vec<QueueEntry>> {
        let page_text = serde_json::to_string(page)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        require_generation(&transaction, generation)?;
        let mut scheduled = Vec::new();
        let config: CrawlConfig =
            metadata(&transaction, "config")?.context("Missing saved crawl config")?;
        let expansion = resources
            .as_ref()
            .is_some_and(|(_, _, expansion)| *expansion);
        let resource_enabled = resources.is_some();
        let already_retained = resource_enabled
            && transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM pages WHERE url=?1)",
                [&page.url],
                |row| row.get::<_, bool>(0),
            )?;
        let mut new_resource_pending = 0usize;
        crate::content_binding::validate_native_content_captures(
            page,
            &config,
            if expansion {
                page.resource_response
                    .as_ref()
                    .map(|facts| facts.observed_generation)
            } else {
                Some(generation)
            },
            true,
        )?;
        if let Some((rows, limits)) = policy {
            let observation = page
                .policy
                .as_ref()
                .context("Policy response lacks saved task admission")?;
            let saved = crate::policy_ledger::saved_admission(
                &transaction,
                crate::PolicyTaskRole::PageDocument,
                &page.url,
            )?
            .context("Policy frontier admission missing")?;
            if saved != observation.admission {
                bail!("Policy response changed immutable initial permission");
            }
            crate::policy_ledger::commit(
                &transaction,
                rows,
                generation,
                limits,
                (!expansion).then_some((
                    &observation.source_revision,
                    observation.candidates_complete,
                )),
            )?;
            for row in rows.iter().filter(|_| resources.is_none()) {
                if let Some(url) = &row.scheduled_fetch_url {
                    if row.task_role != crate::PolicyTaskRole::PageDocument {
                        bail!("Page source scheduled invalid task role");
                    }
                    scheduled.push(QueueEntry {
                        url: url.clone(),
                        depth: page.depth + 1,
                        discovered_from: Some(page.url.clone()),
                        admission: crate::policy_ledger::saved_admission(
                            &transaction,
                            row.task_role,
                            url,
                        )?,
                    });
                }
            }
        }
        if metadata_text(&transaction, "operation_kind")?.as_deref() == Some("selected_recrawl") {
            if !discovered.is_empty() || !scheduled.is_empty() {
                bail!("Selected recrawl cannot expand its frontier");
            }
            let target: (i64,Option<String>) = transaction.query_row("SELECT frontier.depth,frontier.discovered_from FROM frontier JOIN recrawl_targets ON frontier.url=recrawl_targets.url WHERE frontier.url=?1", [&page.url], |row| Ok((row.get(0)?,row.get(1)?)))?;
            if page.depth != usize::try_from(target.0)? || page.discovered_from != target.1 {
                bail!("Selected observation changed parent discovery provenance");
            }
            let updated = transaction.execute("UPDATE recrawl_targets SET committed_generation=?2,committed_payload_sha256=?3 WHERE url=?1 AND admitted_attempts>0 AND last_admitted_generation=?2 AND committed_generation IS NULL", params![page.url, i64::try_from(generation)?,sha256(&page_text)])?;
            if updated != 1 {
                bail!("Selected target is not pending/admitted");
            }
            let mut invocation: RecrawlInvocation = metadata(&transaction, "recrawl_invocation")?
                .context("Selected invocation missing")?;
            if invocation.generation != generation {
                bail!("Stale selected invocation");
            }
            invocation.committed = invocation
                .committed
                .checked_add(1)
                .context("Selected commit count overflow")?;
            put_metadata(
                &transaction,
                "recrawl_invocation",
                &serde_json::to_string(&invocation)?,
            )?;
        }
        let mut retain = true;
        if let Some((rows, limits, expansion)) = resources {
            let permission = page
                .resource_permission
                .as_ref()
                .context("Resource response lacks permission")?;
            let saved = crate::resource_ledger::permission(&transaction, &page.url)?
                .context("Resource task missing")?;
            permission.validate()?;
            // An independently discovered grant may have augmented the task
            // during the fetch. The actually used permission remains a subset.
            for grant in &permission.authorizing_grants {
                if !saved.authorizing_grants.contains(grant) {
                    bail!("Resource response permission was not saved");
                }
            }
            retain = if config
                .resource_discovery
                .as_ref()
                .is_some_and(|config| config.contract.is_some())
            {
                let facts = page
                    .resource_response
                    .as_ref()
                    .context("Strict resource response facts missing")?;
                facts.retained_response
                    || (metadata_text(&transaction, "operation_kind")?.as_deref()
                        == Some("selected_recrawl")
                        && facts.selected_materialization_reason.as_deref()
                            == Some("selected_scope_limited_projection"))
            } else {
                permission.store
            };
            let batch = page
                .resource_source
                .as_ref()
                .context("Resource response source inventory missing")?;
            limits.check_batch(batch)?;
            let urls = crate::resource_ledger::commit(
                &transaction,
                rows,
                limits,
                (!expansion).then_some((batch.source_revision.as_str(), batch.complete)),
            )?;
            if expansion {
                crate::resource_ledger::mark_expanded(&transaction, &page.url, permission, retain)?;
            } else {
                crate::resource_ledger::save_transport(
                    &transaction,
                    page,
                    generation,
                    &config,
                    limits,
                    retain,
                )?;
            }
            for url in urls {
                let inserted = transaction.execute(
                    "INSERT OR IGNORE INTO frontier(url,depth,discovered_from) VALUES(?1,?2,?3)",
                    params![url, i64::try_from(page.depth + 1)?, page.url],
                )?;
                new_resource_pending += inserted;
                if inserted == 1 {
                    scheduled.push(QueueEntry {
                        url: url.clone(),
                        depth: page.depth + 1,
                        discovered_from: Some(page.url.clone()),
                        admission: crate::policy_ledger::saved_admission(
                            &transaction,
                            crate::PolicyTaskRole::PageDocument,
                            &url,
                        )?,
                    });
                }
            }
        }
        if retain {
            transaction.execute(
            "INSERT INTO pages(url,data) VALUES(?1,?2) ON CONFLICT(url) DO UPDATE SET data=excluded.data",
            params![page.url, page_text],
        )?;
        }
        transaction.execute("UPDATE frontier SET done=1 WHERE url=?1", [&page.url])?;
        if resource_enabled {
            crate::resource_ledger::frontier_committed(
                &transaction,
                new_resource_pending,
                !expansion,
                retain && !already_retained,
            )?;
        }
        for entry in discovered.iter().chain(scheduled.iter()) {
            transaction.execute(
                "INSERT OR IGNORE INTO frontier(url,depth,discovered_from) VALUES(?1,?2,?3)",
                params![entry.url, entry.depth as i64, entry.discovered_from],
            )?;
        }
        transaction.commit()?;
        Ok(scheduled)
    }

    pub(crate) fn pending(&self) -> anyhow::Result<Vec<QueueEntry>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let sql = if metadata_text(&connection, "operation_kind")?.as_deref()
            == Some("selected_recrawl")
        {
            "SELECT frontier.url,frontier.depth,frontier.discovered_from FROM frontier JOIN recrawl_pages ON frontier.url=recrawl_pages.url WHERE frontier.done=0 ORDER BY recrawl_pages.ordinal"
        } else {
            "SELECT url,depth,discovered_from FROM frontier WHERE done=0 ORDER BY depth,rowid"
        };
        let mut statement = connection.prepare(sql)?;
        statement
            .query_map([], |row| {
                Ok(QueueEntry {
                    url: row.get(0)?,
                    depth: row.get::<_, i64>(1)? as usize,
                    discovered_from: row.get(2)?,
                    admission: None,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .context("Read saved frontier")
            .and_then(|mut entries| {
                if metadata_text(&connection, "policy_ledger")?.is_some() {
                    for entry in &mut entries {
                        entry.admission = crate::policy_ledger::saved_admission(
                            &connection,
                            crate::PolicyTaskRole::PageDocument,
                            &entry.url,
                        )?;
                    }
                }
                Ok(entries)
            })
    }

    pub fn pages(&self) -> anyhow::Result<Vec<PageRecord>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let sql = if metadata_text(&connection, "operation_kind")?.as_deref()
            == Some("selected_recrawl")
        {
            "SELECT pages.data FROM pages JOIN recrawl_pages ON pages.url=recrawl_pages.url ORDER BY recrawl_pages.ordinal"
        } else {
            "SELECT data FROM pages ORDER BY rowid"
        };
        let mut statement = connection.prepare(sql)?;
        let values = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        verify_saved_content_captures(&connection)?;
        values
            .into_iter()
            .map(|value| {
                let page: PageRecord = serde_json::from_str(&value)?;
                crate::validate_page_content_captures(&page)?;
                Ok(page)
            })
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
        verify_saved_content_captures(&transaction)?;
        for page in &report.pages {
            crate::validate_page_content_captures(page)?;
            if page.content_captures.is_some() {
                let stored: String = transaction.query_row(
                    "SELECT data FROM pages WHERE url=?1",
                    [&page.url],
                    |row| row.get(0),
                )?;
                let stored: PageRecord = serde_json::from_str(&stored)?;
                if stored.content_captures != page.content_captures {
                    bail!("Checkpoint capture differs from committed page observation");
                }
            }
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
        if report.operation != recrawl_operation(&transaction)? {
            bail!("Report selected operation does not match committed target evidence");
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
            policy_ledger: crate::policy_ledger::summary(&transaction)?,
            resource_ledger: crate::resource_ledger::summary(&transaction)?,
        };
        if report.policy_ledger != checkpoint.policy_ledger {
            bail!("Report policy ledger differs from committed evidence");
        }
        if report.resource_ledger != checkpoint.resource_ledger {
            bail!("Report resource ledger differs from committed evidence");
        }
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
            .map(|value| {
                let report: CrawlReport = serde_json::from_str(&value)?;
                for page in &report.pages {
                    crate::validate_page_content_captures(page)?;
                }
                Ok(report)
            })
            .transpose()
    }
}

/// One bounded row at a time; the SQLite run state is an internal provenance
/// boundary, not authentication of imported or externally rewritten databases.
fn verify_saved_content_captures(connection: &Connection) -> anyhow::Result<()> {
    let Some(config) = metadata::<CrawlConfig>(connection, "config")? else {
        return Ok(());
    };
    if config.content_capture.is_none() {
        let has_captures: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM pages WHERE json_type(data,'$.content_captures') IS NOT NULL AND json_type(data,'$.content_captures') <> 'null' LIMIT 1)", [], |row| row.get(0))?;
        if !has_captures {
            return Ok(());
        }
    }
    let run: Option<CrawlRunState> = metadata(connection, "run_state")?;
    let selected =
        metadata_text(connection, "operation_kind")?.as_deref() == Some("selected_recrawl");
    let sql = if selected {
        "SELECT pages.url,pages.data,recrawl_pages.inherited,recrawl_targets.committed_generation FROM pages JOIN recrawl_pages ON pages.url=recrawl_pages.url LEFT JOIN recrawl_targets ON pages.url=recrawl_targets.url"
    } else {
        "SELECT url,data,0,NULL FROM pages"
    };
    let mut statement = connection.prepare(sql)?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let url: String = row.get(0)?;
        let page: PageRecord = serde_json::from_str(&row.get::<_, String>(1)?)?;
        let inherited: bool = row.get(2)?;
        let committed = row
            .get::<_, Option<i64>>(3)?
            .map(u64::try_from)
            .transpose()?;
        if page.url != url {
            bail!("Stored capture enclosing requested URL differs from row identity");
        }
        crate::content_binding::validate_native_content_captures(
            &page, &config, committed, !inherited,
        )?;
        if !inherited && let Some(captures) = &page.content_captures {
            for capture in captures {
                if !capture.observed_generation.is_some_and(|generation| {
                    run.as_ref()
                        .is_some_and(|run| generation > 0 && generation <= run.generation)
                }) {
                    bail!(
                        "Saved capture generation is missing or outside the observed run history"
                    );
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RecrawlInvocation {
    generation: u64,
    max_attempts: Option<usize>,
    admitted: usize,
    committed: usize,
}

fn parent_snapshot(connection: &Connection) -> anyhow::Result<Option<Vec<u8>>> {
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='recrawl_base')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(None);
    }
    Ok(connection
        .query_row(
            "SELECT snapshot FROM recrawl_base WHERE id=1 AND length(snapshot)<=268435456",
            [],
            |row| row.get(0),
        )
        .optional()?)
}

fn verify_resource_read_context(
    connection: &Connection,
    config: &CrawlConfig,
) -> anyhow::Result<()> {
    if config.resource_discovery.is_none() {
        return Ok(());
    }
    // The mutable plan table cannot independently authorize inherited grants.
    // Rebuild its contract from the exact saved parent before emitting a page.
    let plan = load_run_plan(connection)?;
    if matches!(plan, RunPlan::SelectedRecrawl(_)) {
        verify_recrawl_rows(connection, &plan)?;
    }
    crate::resource_ledger::verify(connection, config)
}

fn load_run_plan(connection: &Connection) -> anyhow::Result<RunPlan> {
    let plan: RunPlan = metadata(connection, "run_plan")?.unwrap_or_default();
    if let RunPlan::SelectedRecrawl(selected) = &plan {
        if metadata_text(connection, "operation_kind")?.as_deref() != Some("selected_recrawl") {
            bail!("Selected operation kind marker is inconsistent");
        }
        let bytes = parent_snapshot(connection)?
            .context("Selected recrawl has no immutable parent snapshot")?;
        let config: CrawlConfig = metadata(connection, "config")?
            .context("Selected recrawl has no inherited configuration")?;
        crate::recrawl::validate_saved_plan(selected, &bytes, &config)?;
    } else if parent_snapshot(connection)?.is_some()
        || metadata_text(connection, "operation_kind")?.is_some()
    {
        bail!("Immutable recrawl parent exists without a selected run plan");
    }
    Ok(plan)
}

fn verify_recrawl_rows(connection: &Connection, plan: &RunPlan) -> anyhow::Result<()> {
    let RunPlan::SelectedRecrawl(plan) = plan else {
        return Ok(());
    };
    let bytes = parent_snapshot(connection)?.context("Immutable parent missing")?;
    let base: CrawlReport = serde_json::from_slice(&bytes)?;
    let run: Option<CrawlRunState> = metadata(connection, "run_state")?;
    let run_generation = run.map_or(0, |run| run.generation);
    let selected = plan.targets_set();
    let target_by_url: std::collections::HashMap<_, _> = plan
        .targets
        .iter()
        .map(|target| (target.requested_url.as_str(), target))
        .collect();
    let current: usize = count(connection, "SELECT count(*) FROM recrawl_pages")?;
    if current != base.pages.len()
        || count(connection, "SELECT count(*) FROM recrawl_targets")? != plan.targets.len()
        || count(connection, "SELECT count(*) FROM frontier")? != plan.targets.len()
        || discovery_pending_count(connection)? != 0
    {
        bail!("Selected recrawl rows/frontier do not match its immutable plan");
    }
    for (ordinal, page) in base.pages.iter().enumerate() {
        let saved: (i64, String, bool) = connection.query_row(
            "SELECT ordinal,payload_sha256,inherited FROM recrawl_pages WHERE url=?1",
            [&page.url],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        if usize::try_from(saved.0)? != ordinal
            || saved.1 != crate::recrawl::payload_sha256(page)?
            || saved.2 == selected.contains(page.url.as_str())
        {
            bail!("Retained parent payload identity is inconsistent");
        }
        let data: Option<String> = connection
            .query_row("SELECT data FROM pages WHERE url=?1", [&page.url], |row| {
                row.get(0)
            })
            .optional()?;
        if saved.2 {
            if data.as_deref().map(sha256).as_deref() != Some(saved.1.as_str()) {
                bail!("Inherited page payload was altered");
            }
        } else {
            let frontier: (i64, Option<String>, i64) = connection.query_row(
                "SELECT depth,discovered_from,done FROM frontier WHERE url=?1",
                [&page.url],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
            if usize::try_from(frontier.0)? != page.depth
                || frontier.1 != page.discovered_from
                || !matches!(frontier.2, 0 | 1)
            {
                bail!("Selected frontier changed immutable parent discovery provenance");
            }
            let target = target_by_url
                .get(page.url.as_str())
                .context("Selected target missing")?;
            if let Some(admission) = &target.policy_admission
                && crate::policy_ledger::saved_admission(
                    connection,
                    crate::PolicyTaskRole::PageDocument,
                    &page.url,
                )?
                .as_ref()
                    != Some(admission)
            {
                bail!("Inherited policy admission changed immutable selected target permission");
            }
            let (attempts,last_admitted,committed,payload): (i64,Option<i64>,Option<i64>,Option<String>) = connection.query_row(
                "SELECT admitted_attempts,last_admitted_generation,committed_generation,committed_payload_sha256 FROM recrawl_targets WHERE url=?1",
                [&page.url],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
            )?;
            let attempts = u64::try_from(attempts)?;
            let last_admitted = last_admitted.map(u64::try_from).transpose()?;
            let committed = committed.map(u64::try_from).transpose()?;
            if attempts > run_generation
                || (attempts == 0) != last_admitted.is_none()
                || last_admitted
                    .is_some_and(|generation| generation == 0 || generation > run_generation)
                || committed.is_some_and(|generation| {
                    generation == 0
                        || generation > run_generation
                        || Some(generation) != last_admitted
                })
                || committed.is_some() != payload.is_some()
            {
                bail!("Selected admission/commit generation evidence is inconsistent");
            }
            let done = frontier.2 == 1;
            if committed.is_some() != done || data.is_some() != done {
                bail!("Selected pending/committed rows are inconsistent");
            }
            if let Some(data) = data {
                if payload.as_deref() != Some(sha256(&data).as_str()) {
                    bail!("Committed selected payload was altered");
                }
                let fresh: PageRecord = serde_json::from_str(&data)?;
                if fresh.url != page.url
                    || fresh.depth != page.depth
                    || fresh.discovered_from != page.discovered_from
                {
                    bail!("Selected row changed requested identity/discovery provenance");
                }
            }
        }
    }
    if count(
        connection,
        "SELECT count(*) FROM pages WHERE url NOT IN(SELECT url FROM recrawl_pages)",
    )? != 0
    {
        bail!("Unexpected current page outside selected plan");
    }
    let invocation: RecrawlInvocation =
        metadata(connection, "recrawl_invocation")?.context("Selected invocation missing")?;
    if invocation.generation > run_generation
        || invocation.committed > invocation.admitted
        || invocation
            .max_attempts
            .is_some_and(|limit| limit == 0 || invocation.admitted > limit)
    {
        bail!("Selected invocation counters/budget are inconsistent");
    }
    let admitted: i64 = connection.query_row(
        "SELECT count(*) FROM recrawl_targets WHERE last_admitted_generation=?1",
        [i64::try_from(invocation.generation)?],
        |row| row.get(0),
    )?;
    let committed: i64 = connection.query_row(
        "SELECT count(*) FROM recrawl_targets WHERE committed_generation=?1",
        [i64::try_from(invocation.generation)?],
        |row| row.get(0),
    )?;
    if usize::try_from(admitted)? != invocation.admitted
        || usize::try_from(committed)? != invocation.committed
    {
        bail!("Selected invocation counters disagree with target evidence");
    }
    Ok(())
}

fn recrawl_operation(connection: &Connection) -> anyhow::Result<Option<SelectedRecrawlOperation>> {
    let Some(RunPlan::SelectedRecrawl(plan)) = metadata::<RunPlan>(connection, "run_plan")? else {
        return Ok(None);
    };
    let invocation: RecrawlInvocation = metadata(connection, "recrawl_invocation")?
        .context("Selected invocation evidence missing")?;
    let retained = count(
        connection,
        "SELECT count(*) FROM recrawl_pages WHERE inherited=1",
    )?;
    let mut statement = connection
        .prepare("SELECT url,admitted_attempts,committed_generation FROM recrawl_targets")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut records: std::collections::HashMap<_, _> = rows
        .into_iter()
        .map(|(url, attempts, generation)| (url, (attempts, generation)))
        .collect();
    let mut targets = Vec::with_capacity(plan.targets.len());
    for target in &plan.targets {
        let (attempts, generation) = records
            .remove(&target.requested_url)
            .context("Selected target evidence missing")?;
        targets.push(RecrawlTargetObservation {
            requested_url: target.requested_url.clone(),
            parent_ordinal: target.parent_ordinal,
            parent_page_sha256: target.parent_page_sha256.clone(),
            state: if generation.is_some() {
                RecrawlTargetState::Committed
            } else {
                RecrawlTargetState::Pending
            },
            admitted_attempts: usize::try_from(attempts)?,
            committed_generation: generation.map(u64::try_from).transpose()?,
        });
    }
    if !records.is_empty() {
        bail!("Unexpected selected target evidence");
    }
    Ok(Some(plan.operation(
        retained,
        targets,
        invocation.max_attempts,
        invocation.admitted,
        invocation.committed,
    )))
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

    #[test]
    fn policy_stream_uses_one_wal_snapshot_while_another_generation_appends() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("stream.sqlite");
        let writer = CrawlStore::open(&path).unwrap();
        let policy_config = crate::CrawlPolicyConfig {
            scope: crate::CrawlScopeConfig {
                roots: vec![crate::CrawlScopeRoot {
                    origin: "https://example.org".into(),
                    subdomains: false,
                    folders: vec!["/".into()],
                    exact_paths: vec![],
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        let policy = crate::CompiledCrawlPolicy::new(&policy_config).unwrap();
        let config = CrawlConfig {
            crawl_policy: Some(policy_config),
            ..Default::default()
        };
        writer.prepare(&config).unwrap();
        let generation = writer.begin_run("first").unwrap();
        let make_row = |revision: &str, ordinal| {
            let mut row = crate::policy_ledger::decide(
                &policy,
                revision,
                ordinal,
                crate::CandidateSourceKind::Anchor,
                format!("https://example.org/item/{revision}/{ordinal}"),
                None,
                None,
                crate::PolicyInputBasis::ObservedAttribute,
                crate::PolicyTaskRole::ObservationOnly,
            );
            row.admission_reason = "observation_only".into();
            row
        };
        let mut rows = (0..3)
            .map(|index| make_row("first", index))
            .collect::<Vec<_>>();
        writer
            .initialize_policy(
                &policy,
                &mut rows,
                &crate::LiveCrawlPolicyLimits::default(),
                generation,
            )
            .unwrap();
        let original = writer.policy_ledger_summary().unwrap();
        let reader = CrawlStore::open_read_only(&path).unwrap();
        let mut seen = Vec::new();
        let mut calls = 0;
        let returned = reader
            .visit_policy_occurrences(1, |page| {
                assert_eq!(page.ledger, original);
                calls += 1;
                if calls == 1 {
                    let generation = writer.begin_run("second").unwrap();
                    let mut connection = writer.connection.lock().unwrap();
                    let transaction = connection.transaction().unwrap();
                    crate::policy_ledger::commit(
                        &transaction,
                        &mut [make_row("second", 0)],
                        generation,
                        &crate::LiveCrawlPolicyLimits::default(),
                        None,
                    )?;
                    transaction.commit()?;
                }
                seen.extend(page.rows.iter().map(|row| (row.sequence, row.generation)));
                Ok(())
            })
            .unwrap();
        assert_eq!(returned, original);
        assert_eq!(calls, 3);
        assert_eq!(seen, vec![(1, 1), (2, 1), (3, 1)]);
        assert_eq!(
            writer.policy_ledger_summary().unwrap().unwrap().occurrences,
            4
        );
        assert_eq!(
            reader.policy_occurrences(Some(3), 1).unwrap().rows[0].generation,
            2
        );
    }

    #[test]
    fn legacy_policy_stream_calls_empty_header_once_and_aborts_on_consumer_error() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("legacy-stream.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT);")
            .unwrap();
        drop(connection);
        let before = std::fs::read(&path).unwrap();
        let store = CrawlStore::open_read_only(&path).unwrap();
        let mut calls = 0;
        assert_eq!(
            store
                .visit_policy_occurrences(1, |page| {
                    calls += 1;
                    assert!(page.ledger.is_none());
                    assert!(page.rows.is_empty());
                    assert!(!page.has_more);
                    Ok(())
                })
                .unwrap(),
            None
        );
        assert_eq!(calls, 1);
        assert!(
            store
                .visit_policy_occurrences(1, |_| anyhow::bail!("disconnected fixture consumer"))
                .is_err()
        );
        assert!(
            store
                .visit_policy_occurrences(0, |_| panic!("invalid limit must not emit"))
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    fn report(config: CrawlConfig, pending: usize) -> CrawlReport {
        CrawlReport {
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
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

    fn selected_preparation() -> PreparedSelectedRecrawl {
        let mut source = report(
            CrawlConfig {
                seed_urls: vec!["https://example.org/".into()],
                ..Default::default()
            },
            0,
        );
        source.pages = vec![
            PageRecord {
                url: "https://example.org/retained".into(),
                fetched_at: "parent-time".into(),
                ..Default::default()
            },
            PageRecord {
                url: "https://example.org/selected".into(),
                depth: 3,
                discovered_from: Some("https://example.org/source".into()),
                ..Default::default()
            },
        ];
        crate::prepare_selected_recrawl(
            crate::SelectedRecrawlParent {
                job_id: "store-fixture".into(),
                checkpoint_revision: None,
                source_kind: crate::RecrawlSourceKind::ImportedSnapshot,
            },
            &serde_json::to_vec_pretty(&source).unwrap(),
            &["https://example.org/selected".into()],
            Some(1),
        )
        .unwrap()
    }

    #[test]
    fn selected_initialization_is_atomic_and_never_inserts_historical_selected_pages() {
        let temp = tempfile::tempdir().unwrap();
        let store = CrawlStore::open(temp.path().join("child.sqlite")).unwrap();
        let prepared = selected_preparation();
        store.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_initial_frontier BEFORE INSERT ON frontier BEGIN SELECT RAISE(ABORT,'fixture initialization failed'); END;").unwrap();
        assert!(store.initialize_selected_recrawl(&prepared).is_err());
        assert!(store.config().unwrap().is_none());
        assert!(store.parent_snapshot_bytes().unwrap().is_none());
        assert!(store.pages().unwrap().is_empty());
        store
            .connection
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_initial_frontier;")
            .unwrap();
        store.initialize_selected_recrawl(&prepared).unwrap();
        assert_eq!(store.pages().unwrap().len(), 1);
        assert_eq!(
            store.pages().unwrap()[0].url,
            "https://example.org/retained"
        );
        assert_eq!(
            store.parent_snapshot_bytes().unwrap().unwrap(),
            prepared.base_snapshot_bytes
        );
        let inspection = store.inspection().unwrap();
        assert_eq!(inspection.operation.unwrap().pending_count, 1);
        assert!(!inspection.checkpoint_valid);
        assert!(store.initialize_selected_recrawl(&prepared).is_err());
    }

    #[test]
    fn selected_read_only_inspection_rejects_tampered_base_plan_payload_and_frontier() {
        let mutations = [
            "UPDATE recrawl_base SET snapshot=CAST(snapshot AS TEXT)||' '",
            "UPDATE metadata SET value=json_set(value,'$.plan.unknown_future_semantics',true) WHERE key='run_plan'",
            "UPDATE pages SET data='{}' WHERE url='https://example.org/retained'",
            "INSERT INTO pages(url,data) VALUES('https://example.org/selected','{}')",
            "UPDATE frontier SET depth=999 WHERE url='https://example.org/selected'",
            "UPDATE frontier SET discovered_from='forged' WHERE url='https://example.org/selected'",
            "DELETE FROM recrawl_targets",
            "UPDATE metadata SET value='unknown' WHERE key='operation_kind'",
        ];
        for (index, mutation) in mutations.iter().enumerate() {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("tampered.sqlite");
            let store = CrawlStore::open(&path).unwrap();
            let prepared = selected_preparation();
            store.initialize_selected_recrawl(&prepared).unwrap();
            store
                .connection
                .lock()
                .unwrap()
                .execute_batch(mutation)
                .unwrap();
            assert!(CrawlStore::inspect(&path).is_err(), "mutation {index}");
            assert!(store.run_plan().is_err(), "mutation {index}");
            assert!(store.recrawl_operation().is_err(), "mutation {index}");
        }
    }

    #[test]
    fn selected_failed_page_commit_rolls_back_target_state_and_counts_and_budget_override_persists()
    {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("rollback.sqlite");
        let store = CrawlStore::open(&path).unwrap();
        let prepared = selected_preparation();
        store.initialize_selected_recrawl(&prepared).unwrap();
        let generation = store.begin_run("first").unwrap();
        assert_eq!(
            store.start_recrawl_invocation(generation, None).unwrap(),
            Some(1)
        );
        store
            .admit_selected("https://example.org/selected", generation)
            .unwrap();
        assert!(
            store
                .admit_selected("https://example.org/selected", generation)
                .is_err()
        );
        let fresh = PageRecord {
            url: "https://example.org/selected".into(),
            depth: 3,
            discovered_from: Some("https://example.org/source".into()),
            fetched_at: "new-time".into(),
            ..Default::default()
        };
        store.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_selected_page BEFORE INSERT ON pages BEGIN SELECT RAISE(ABORT,'fixture page failed'); END;").unwrap();
        assert!(store.complete(&fresh, &[], generation).is_err());
        let op = store.recrawl_operation().unwrap().unwrap();
        assert_eq!(
            (
                op.admitted_this_run,
                op.committed_this_run,
                op.pending_count
            ),
            (1, 0, 1)
        );
        assert_eq!(op.targets[0].state, RecrawlTargetState::Pending);
        assert_eq!(store.pending_count().unwrap(), 1);
        store
            .connection
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_selected_page;")
            .unwrap();
        store.complete(&fresh, &[], generation).unwrap();
        let op = store.recrawl_operation().unwrap().unwrap();
        assert_eq!(
            (op.committed_this_run, op.refreshed_count, op.pending_count),
            (1, 1, 0)
        );
        let second = store.begin_run("second").unwrap();
        assert_eq!(
            store.start_recrawl_invocation(second, Some(0)).unwrap(),
            None
        );
        let third = store.begin_run("third").unwrap();
        assert_eq!(store.start_recrawl_invocation(third, None).unwrap(), None);
        assert_eq!(
            store.parent_snapshot_bytes().unwrap().unwrap(),
            prepared.base_snapshot_bytes
        );
        assert_eq!(store.pages().unwrap()[0].fetched_at, "parent-time");
    }

    #[test]
    fn selected_committed_admission_generation_and_payload_tampering_are_rejected() {
        for mutation in [
            "UPDATE recrawl_targets SET admitted_attempts=0",
            "UPDATE recrawl_targets SET committed_generation=0",
            "UPDATE recrawl_targets SET committed_generation=999",
            "UPDATE recrawl_targets SET last_admitted_generation=0",
            "UPDATE pages SET data=json_set(data,'$.titles',json('[\"tampered\"]')) WHERE url='https://example.org/selected'",
            "UPDATE metadata SET value=json_set(value,'$.committed',999) WHERE key='recrawl_invocation'",
        ] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("committed.sqlite");
            let store = CrawlStore::open(&path).unwrap();
            let prepared = selected_preparation();
            store.initialize_selected_recrawl(&prepared).unwrap();
            let generation = store.begin_run("first").unwrap();
            store.start_recrawl_invocation(generation, None).unwrap();
            store
                .admit_selected("https://example.org/selected", generation)
                .unwrap();
            let fresh = PageRecord {
                url: "https://example.org/selected".into(),
                depth: 3,
                discovered_from: Some("https://example.org/source".into()),
                ..Default::default()
            };
            store.complete(&fresh, &[], generation).unwrap();
            let mut report = report(prepared.config.clone(), 0);
            report.pages = store.pages().unwrap();
            report.summary.completed = 2;
            report.operation = store.recrawl_operation().unwrap();
            store
                .save_checkpoint(&report, CrawlOutcome::Completed, generation)
                .unwrap();
            assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
            store
                .connection
                .lock()
                .unwrap()
                .execute_batch(mutation)
                .unwrap();
            assert!(CrawlStore::inspect(&path).is_err(), "{mutation}");
            assert!(store.run_plan().is_err(), "{mutation}");
            assert!(store.recrawl_operation().is_err(), "{mutation}");
            assert_eq!(
                store.parent_snapshot_bytes().unwrap().unwrap(),
                prepared.base_snapshot_bytes
            );
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
                admission: None,
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
                        discovered_from: None,
                        admission: None,
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
