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

#[path = "store/config_profile_catalog.rs"]
mod config_profile_catalog;
pub use config_profile_catalog::{
    CONFIG_PROFILE_CATALOG_ENGINE, CONFIG_PROFILE_CATALOG_ENTRIES,
    CONFIG_PROFILE_CATALOG_NAME_BYTES, CONFIG_PROFILE_CATALOG_TOTAL_BYTES, ConfigProfileCatalog,
    ConfigProfileCatalogDocument, ConfigProfileCatalogEntry, ConfigProfileCatalogError,
    ConfigProfileCatalogErrorKind,
};

#[path = "store/crawl_quota_adapter.rs"]
mod crawl_quota_adapter;
#[cfg(test)]
#[path = "store/open_tests.rs"]
mod open_tests;
#[path = "store/sitemap_raw_read.rs"]
mod sitemap_raw_read;
#[path = "store/tls_state.rs"]
mod tls_state;
#[cfg(windows)]
#[path = "store/windows_path.rs"]
mod windows_path;
pub use sitemap_raw_read::{
    SITEMAP_RAW_READ_ENGINE, SitemapRawReadContext, SitemapRawReadRequest, SitemapRawReadWindow,
    SitemapRawRowKind,
};
#[path = "store/sitemap_read_context.rs"]
mod sitemap_read_context;
#[path = "store/sitemap_read_window.rs"]
mod sitemap_read_window;
pub use sitemap_read_context::SitemapVerifiedReadHeader;
#[path = "store/sitemap_source_body_read.rs"]
mod sitemap_source_body_read;
pub use sitemap_source_body_read::{
    SitemapBodyCatalogState, SitemapBodyReadContext, SitemapBodyReadError, SitemapBodyWindow,
    SitemapVerifiedBodyCopy,
};
#[path = "store/sitemap_findings_read.rs"]
mod sitemap_findings_read;
#[path = "store/sitemap_stored_rows.rs"]
mod sitemap_stored_rows;
pub use sitemap_findings_read::SitemapVerifiedFindings;
pub use sitemap_read_window::{
    SITEMAP_READ_WINDOW_ENGINE, SitemapReadState, SitemapReadWindow, SitemapReadWindowError,
    SitemapReadWindowExpectedContext, sitemap_read_state_sha256,
};
pub use sitemap_stored_rows::{SitemapStoredRowsFilter, SitemapVerifiedStoredRows};

/// Roots are original declarations; source/task aliases must already be finalized.
/// config_sha256 binds the actual saved configuration bytes at FIRST initialization.
/// Resume retains that hash while Store::prepare permits operational overrides.
#[derive(Debug, Clone)]
pub(crate) struct SitemapCatalogInit {
    pub config_sha256: String,
    pub limits: crate::SitemapLedgerLimits,
    pub roots: Vec<crate::SitemapRootDeclaration>,
    pub legacy_history_unknown: bool,
}

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

/// One fully validated existing SQLite read transaction. No migration or HTTP.
#[derive(Debug, Clone)]
pub struct SitemapReadSnapshot {
    pub crawl_config_sha256: Option<String>,
    pub generation: Option<u64>,
    pub catalog: Option<crate::SitemapCatalog>,
    pub coverage_inventory: Option<crate::SitemapCoverageInventory>,
    pub coverage: crate::SitemapCoverage,
    pub membership_analysis: Option<crate::SitemapMembershipAnalysis>,
    pub sitemap_source_measurements: Option<crate::SitemapSourceStoreSnapshot>,
    /// Changes on A-only captures, commit links and durable omission markers.
    pub sitemap_source_measurement_context: String,
}

/// SQLite WAL storage. A fetched page and its frontier state commit atomically.
/// Interrupted in-flight requests remain pending and are fetched on resume.
pub struct CrawlStore {
    connection: Mutex<Connection>,
    source_archive_path: std::path::PathBuf,
    source_archive_read_only: bool,
    source_archive_writer: Mutex<()>,
}

fn open_connection(path: &Path, flags: OpenFlags) -> anyhow::Result<Connection> {
    #[cfg(windows)]
    {
        windows_path::open(path, flags)
    }
    #[cfg(not(windows))]
    {
        Ok(Connection::open_with_flags(path, flags)?)
    }
}

// App job errors currently use Display rather than anyhow's full chain. Keep
// both the failed stage and SQLite's cause/code in that first displayed message,
// while retaining the original typed error for callers inspecting the chain.
fn opening_stage_error(stage: &str, error: impl Into<anyhow::Error>) -> anyhow::Error {
    let error = error.into();
    let code = error
        .downcast_ref::<rusqlite::Error>()
        .and_then(|error| match error {
            rusqlite::Error::SqliteFailure(code, _) => {
                Some(format!(" (SQLite {:?}/{})", code.code, code.extended_code))
            }
            _ => None,
        })
        .unwrap_or_default();
    let message = format!("Crawl storage {stage}: {error:#}{code}");
    error.context(message)
}

const INITIALIZE_WAL_AND_CORE_SCHEMA: &str = "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;
            CREATE TABLE IF NOT EXISTS metadata(key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS frontier(url TEXT PRIMARY KEY, depth INTEGER NOT NULL, discovered_from TEXT, done INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS pages(url TEXT PRIMARY KEY, data TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS frontier_pending ON frontier(done,depth);
            CREATE TABLE IF NOT EXISTS discovery_frontier(kind TEXT NOT NULL,url TEXT NOT NULL,done INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(kind,url));
            CREATE INDEX IF NOT EXISTS discovery_frontier_pending ON discovery_frontier(done);
            CREATE TABLE IF NOT EXISTS recrawl_base(id INTEGER PRIMARY KEY CHECK(id=1),snapshot BLOB NOT NULL);
            CREATE TABLE IF NOT EXISTS recrawl_pages(url TEXT PRIMARY KEY,ordinal INTEGER NOT NULL,payload_sha256 TEXT NOT NULL,inherited INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS recrawl_targets(url TEXT PRIMARY KEY,admitted_attempts INTEGER NOT NULL DEFAULT 0,last_admitted_generation INTEGER,committed_generation INTEGER,committed_payload_sha256 TEXT);";

impl CrawlStore {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .map_err(|error| opening_stage_error("create parent directory", error))?;
        }
        let connection = open_connection(path, OpenFlags::default())
            .map_err(|error| opening_stage_error("open writable SQLite connection", error))?;
        connection
            .execute_batch(INITIALIZE_WAL_AND_CORE_SCHEMA)
            .map_err(|error| opening_stage_error("initialize SQLite WAL and core schema", error))?;
        connection
            .execute_batch(crate::policy_ledger::SCHEMA)
            .map_err(|error| opening_stage_error("initialize policy ledger schema", error))?;
        connection
            .execute_batch(crate::resource_ledger::SCHEMA)
            .map_err(|error| opening_stage_error("initialize resource ledger schema", error))?;
        connection
            .execute_batch(crate::sitemap_ledger::SCHEMA)
            .map_err(|error| opening_stage_error("initialize sitemap ledger schema", error))?;
        // This validates pre-existing marker/table consistency before any DDL;
        // CREATE must not conceal a dropped initialized empty sidecar table.
        crate::sitemap_source_store::initialize_writable_schema(&connection)
            .map_err(|error| opening_stage_error("initialize sitemap source schema", error))?;
        Ok(Self {
            connection: Mutex::new(connection),
            source_archive_path: crate::sitemap_source_archive::files::archive_path(path)?,
            source_archive_read_only: false,
            source_archive_writer: Mutex::new(()),
        })
    }

    /// Opens existing storage without creating files, running migrations or
    /// changing journal configuration. Suitable for cold-process recovery.
    pub fn open_read_only(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let connection = open_connection(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| opening_stage_error("open read-only SQLite connection", error))?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|error| opening_stage_error("configure read-only SQLite timeout", error))?;
        Ok(Self {
            connection: Mutex::new(connection),
            source_archive_path: crate::sitemap_source_archive::files::archive_path(path)?,
            source_archive_read_only: true,
            source_archive_writer: Mutex::new(()),
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
            crate::crawl_quota_ledger::verify(&transaction, config)?;
        }
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
        let report = bounded_saved_report_text(&transaction)?;
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
            let saved: CrawlReport = crate::heap_deserialize::from_str(
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
        let sitemap_snapshot = load_sitemap_read_snapshot_verified(&transaction)?;
        let sitemap_catalog = sitemap_snapshot.catalog.clone();
        let sitemap_source_measurements = sitemap_snapshot
            .sitemap_source_measurements
            .as_ref()
            .map(|snapshot| snapshot.summary.clone());
        let sitemap_ledger = sitemap_catalog
            .as_ref()
            .map(|catalog| catalog.summary.clone());
        if checkpoint_valid {
            anyhow::ensure!(
                checkpoint
                    .as_ref()
                    .is_some_and(|value| value.sitemap_ledger == sitemap_ledger),
                "Terminal sitemap summary differs from checkpoint binding"
            );
            let saved: CrawlReport = crate::heap_deserialize::from_str(
                report
                    .as_deref()
                    .context("Terminal sitemap checkpoint has no report")?,
            )?;
            anyhow::ensure!(
                saved.sitemap_catalog == sitemap_catalog,
                "Terminal sitemap catalog differs from checkpoint report"
            );
            anyhow::ensure!(
                checkpoint.as_ref().is_some_and(
                    |value| value.sitemap_source_measurements == sitemap_source_measurements
                ),
                "Terminal source measurement summary differs from checkpoint binding"
            );
            anyhow::ensure!(
                saved.sitemap_source_measurements == sitemap_snapshot.sitemap_source_measurements,
                "Terminal source captures/commit links differ from checkpoint report"
            );
            crate::validate_sitemap_coverage_inventory(
                &saved.config,
                saved.sitemap_catalog.as_ref(),
                saved.sitemap_coverage_inventory.as_ref(),
            )?;
            // Old reports intentionally lack inventory. New captured reports must
            // retain their exact task/config/generation context at the checkpoint.
            if saved.sitemap_coverage_inventory.is_some() {
                anyhow::ensure!(
                    saved.sitemap_coverage_inventory == sitemap_snapshot.coverage_inventory,
                    "Terminal sitemap coverage context differs from checkpoint report"
                );
            }
        }
        if checkpoint_valid && let (Some(config), Some(run)) = (&config, &run_state) {
            crate::crawl_quota_ledger::verify_checkpoint(&transaction, config, run.generation)?;
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
            sitemap_ledger,
            sitemap_source_measurements,
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
            "sitemap_documents",
            "sitemap_roots",
            "sitemap_entries",
            "sitemap_locations",
        ] {
            let exists: i64 =
                transaction.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })?;
            if exists != 0 {
                bail!("Selected recrawl requires a new empty database");
            }
        }
        crate::sitemap_source_store::ensure_empty(&transaction)?;
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
        if let Some(parent) = &prepared.base_report.sitemap_catalog {
            crate::sitemap_ledger::inherit(
                &transaction,
                parent,
                &prepared.plan.parent.snapshot_sha256,
            )?;
        }
        if let Some(parent) = &prepared.base_report.sitemap_source_measurements {
            crate::sitemap_source_store::inherit(
                &transaction,
                parent,
                &prepared.plan.parent.snapshot_sha256,
            )?;
        }
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
        if let Some(config) = metadata::<CrawlConfig>(&transaction, "config")? {
            crate::crawl_quota_ledger::begin(&transaction, &config)?;
        }
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
        self.prepare_with_quota_increases(config, None)
    }

    pub(crate) fn prepare_with_quota_increases(
        &self,
        config: &CrawlConfig,
        increases: Option<&crate::CrawlQuotaIncreases>,
    ) -> anyhow::Result<bool> {
        config.validate_input_provenance_budget()?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let connection = connection.transaction()?;
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
            crate::crawl_quota_ledger::prepare(&connection, Some(&old), config, increases)?;
            old.crawl_quotas = config.crawl_quotas.clone();
            if old.explicit_sitemap_inputs != config.explicit_sitemap_inputs {
                bail!("Saved crawl cannot change original explicit sitemap provenance");
            }
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
            connection.commit()?;
            Ok(true)
        } else {
            crate::crawl_quota_ledger::prepare(&connection, None, config, increases)?;
            connection.execute(
                "INSERT INTO metadata(key,value) VALUES('config',?1)",
                [serde_json::to_string(config)?],
            )?;
            connection.commit()?;
            Ok(false)
        }
    }

    pub(crate) fn saved_config_bytes_sha256(&self) -> anyhow::Result<String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let bytes: i64 = transaction.query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key='config'",
            [],
            |row| row.get(0),
        )?;
        anyhow::ensure!(
            bytes > 0,
            "Sitemap initialization requires actual saved config bytes"
        );
        // Hash the existing authoritative bytes in bounded chunks. Serialization
        // round trips can change byte order; an arbitrary full-config ceiling
        // would also reject otherwise valid imports or headers.
        let mut hasher = sha2::Sha256::new();
        let mut offset = 0i64;
        while offset < bytes {
            let length = (bytes - offset).min(65536);
            let chunk: Vec<u8> = transaction.query_row(
                "SELECT substr(CAST(value AS BLOB),?1,?2) FROM metadata WHERE key='config'",
                params![offset + 1, length],
                |row| row.get(0),
            )?;
            anyhow::ensure!(
                i64::try_from(chunk.len())? == length,
                "Saved config chunk differs from the authoritative byte length"
            );
            hasher.update(&chunk);
            offset += length;
        }
        let digest = format!("{:x}", hasher.finalize());
        transaction.commit()?;
        Ok(digest)
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

    /// Validated catalog from one SQLite read snapshot. Inherited values require
    /// the whole existing selected-plan/row validator plus exact parent binding.
    pub fn sitemap_read_snapshot(&self) -> anyhow::Result<SitemapReadSnapshot> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let snapshot = load_sitemap_read_snapshot_verified(&transaction)?;
        transaction.commit()?;
        Ok(snapshot)
    }

    /// Target and observed-anchor facts from the actual terminal saved report,
    /// pinned to the same transaction as the original source catalog/coverage.
    pub fn sitemap_read_snapshot_with_membership_analysis(
        &self,
        request: &crate::SitemapMembershipAnalysisRequest,
    ) -> anyhow::Result<SitemapReadSnapshot> {
        request.validate()?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let mut snapshot = load_sitemap_read_snapshot_verified(&transaction)?;
        snapshot.membership_analysis = Some(crate::sitemap_membership_analysis::project(
            &transaction,
            &snapshot,
            request,
        )?);
        transaction.commit()?;
        Ok(snapshot)
    }

    pub fn sitemap_catalog(&self) -> anyhow::Result<Option<crate::SitemapCatalog>> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let catalog = load_sitemap_catalog_verified(&transaction)?;
        transaction.commit()?;
        Ok(catalog)
    }

    pub fn sitemap_ledger_summary(&self) -> anyhow::Result<Option<crate::SitemapLedgerSummary>> {
        Ok(self.sitemap_catalog()?.map(|catalog| catalog.summary))
    }

    pub fn resource_ledger_summary(&self) -> anyhow::Result<Option<crate::ResourceLedgerSummary>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        crate::resource_ledger::summary(&connection)
    }
    /// Separate verified saved resource-occurrence scope. No caller paths or network.
    pub fn resource_broken_sources(
        &self,
        request: &crate::ResourceBrokenSourcesRequest,
    ) -> anyhow::Result<crate::ResourceBrokenSourcesPage> {
        match self
            .read_resource_broken_sources(request, crate::resource_broken_sources::Mode::Page)?
        {
            crate::resource_broken_sources::Output::Page(page) => Ok(page),
            crate::resource_broken_sources::Output::Csv(_) => {
                bail!("Unexpected resource projection mode")
            }
        }
    }
    /// Complete filtered CSV, never only a page. The buffer is returned only after
    /// the same verified read transaction and all byte limits finish successfully.
    pub fn export_resource_broken_sources_csv(
        &self,
        request: &crate::ResourceBrokenSourcesRequest,
    ) -> anyhow::Result<crate::ResourceBrokenSourcesCsv> {
        if request.expected_snapshot_token.is_none() || request.after_sequence.is_some() {
            bail!("Complete resource CSV requires a preview snapshot token and no continuation");
        }
        match self
            .read_resource_broken_sources(request, crate::resource_broken_sources::Mode::Csv)?
        {
            crate::resource_broken_sources::Output::Csv(csv) => Ok(csv),
            crate::resource_broken_sources::Output::Page(_) => {
                bail!("Unexpected resource projection mode")
            }
        }
    }
    fn read_resource_broken_sources(
        &self,
        request: &crate::ResourceBrokenSourcesRequest,
        mode: crate::resource_broken_sources::Mode,
    ) -> anyhow::Result<crate::resource_broken_sources::Output> {
        request.validate()?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let config = crate::resource_broken_sources::preflight(&transaction, request)?;
        if let Some(config) = &config
            && crate::resource_broken_sources::verification_required(&transaction, config)?
        {
            verify_resource_read_context(&transaction, config)?;
        }
        // This internal ordered join receives the same TX. It never reenters
        // another public Store method while the connection mutex is held.
        let output =
            crate::resource_broken_sources::project(&transaction, config.as_ref(), request, mode)?;
        transaction.commit()?;
        Ok(output)
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

    /// Saves initial original declarations with discovery tasks/markers in ONE
    /// transaction. Initial policy rows are already committed by initialize_policy.
    pub(crate) fn initialize_sitemap_discovery(
        &self,
        entries: &[DiscoveryEntry],
        generation: u64,
        init: &SitemapCatalogInit,
    ) -> anyhow::Result<()> {
        self.initialize_sitemap_discovery_with(entries, generation, init, |_, _| Ok(()))
    }

    pub(crate) fn initialize_sitemap_discovery_with<F>(
        &self,
        entries: &[DiscoveryEntry],
        generation: u64,
        init: &SitemapCatalogInit,
        finalize_roots: F,
    ) -> anyhow::Result<()>
    where
        F: FnOnce(
            &rusqlite::Transaction<'_>,
            &mut Vec<crate::SitemapRootDeclaration>,
        ) -> anyhow::Result<()>,
    {
        self.initialize_sitemap_discovery_with_measurement_limits(
            entries,
            generation,
            init,
            &crate::SitemapSourceStoreLimits::default(),
            finalize_roots,
        )
    }

    fn initialize_sitemap_discovery_with_measurement_limits<F>(
        &self,
        entries: &[DiscoveryEntry],
        generation: u64,
        init: &SitemapCatalogInit,
        measurement_limits: &crate::SitemapSourceStoreLimits,
        finalize_roots: F,
    ) -> anyhow::Result<()>
    where
        F: FnOnce(
            &rusqlite::Transaction<'_>,
            &mut Vec<crate::SitemapRootDeclaration>,
        ) -> anyhow::Result<()>,
    {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        require_generation(&transaction, generation)?;
        anyhow::ensure!(
            matches!(load_run_plan(&transaction)?, RunPlan::FullCrawl),
            "Selected recrawl must retain inherited sitemap history without discovery initialization"
        );
        let previous = load_sitemap_catalog_verified(&transaction)?;
        let initialized =
            metadata_text(&transaction, "discovery_initialized")?.as_deref() == Some("true");
        let complete =
            metadata_text(&transaction, "discovery_complete")?.as_deref() == Some("true");
        if let Some(previous) = previous {
            anyhow::ensure!(
                previous.summary.limits == init.limits,
                "Sitemap retention limits changed after initialization"
            );
            // Preserve initialization-config binding across allowed resume knobs.
            // Root need not re-create old policy roots using a new generation.
            crate::sitemap_ledger::initialize(
                &transaction,
                &previous.summary.config_sha256,
                &init.limits,
                previous.summary.legacy_history_unknown,
            )?;
        } else {
            let config = metadata_text(&transaction, "config")?
                .context("Sitemap initialization requires saved configuration")?;
            anyhow::ensure!(
                sha256(&config) == init.config_sha256,
                "Sitemap initialization SHA differs from actual saved configuration bytes"
            );
            let any_done: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM discovery_frontier WHERE done=1)",
                [],
                |row| row.get(0),
            )?;
            let unknown = init.legacy_history_unknown || initialized || complete || any_done;
            crate::sitemap_ledger::initialize(
                &transaction,
                &init.config_sha256,
                &init.limits,
                unknown,
            )?;
            if !unknown {
                let mut roots = init.roots.clone();
                // Root matches immutable input coordinates to actual saved finalized
                // policy rows here, including already-initialized policy.
                finalize_roots(&transaction, &mut roots)?;
                crate::sitemap_ledger::append_roots(&transaction, generation, roots)?;
            }
            // Normalized/merged legacy config cannot recover original declarations.
            // History-unknown initialization never retroactively creates roots.
        }
        // Check/catalog initialization is BEFORE the legacy scheduler early return.
        if !initialized && !complete {
            for entry in entries {
                enqueue_discovery(&transaction, entry)?;
            }
            put_metadata(&transaction, "discovery_initialized", "true")?;
            if discovery_pending_count(&transaction)? == 0 {
                put_metadata(&transaction, "discovery_complete", "true")?;
            }
        }
        let saved = metadata_text(&transaction, "config")?
            .context("Archive initialization needs saved config")?;
        let archive_config: CrawlConfig = serde_json::from_str(&saved)?;
        crate::sitemap_source_archive::require_storage_for_config(&transaction, &archive_config)?;
        crate::sitemap_source_store::initialize(&transaction, generation, measurement_limits)?;
        crate::sitemap_source_archive::initialize(&transaction, &archive_config, &sha256(&saved))?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn initialize_discovery(&self, entries: &[DiscoveryEntry]) -> anyhow::Result<()> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        anyhow::ensure!(
            crate::sitemap_ledger::load_catalog(&transaction)?.is_none(),
            "Initialized sitemap catalog requires typed discovery initialization"
        );
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
    /// Compatibility caller: no catalog may exist, otherwise observations must be supplied.
    pub(crate) fn complete_discovery(
        &self,
        entry: &DiscoveryEntry,
        pages: &[QueueEntry],
        children: &[DiscoveryEntry],
        warnings: &[String],
        generation: u64,
    ) -> anyhow::Result<()> {
        self.complete_discovery_impl(entry, pages, children, warnings, generation, None, None)
    }

    pub(crate) fn complete_discovery_observed(
        &self,
        entry: &DiscoveryEntry,
        pages: &[QueueEntry],
        children: &[DiscoveryEntry],
        warnings: &[String],
        generation: u64,
        batch: crate::SitemapDiscoveryBatch,
    ) -> anyhow::Result<()> {
        self.complete_discovery_impl(
            entry,
            pages,
            children,
            warnings,
            generation,
            Some(batch),
            None,
        )
    }

    /// Transaction A: persist only the actual capture, never frontier admission.
    /// The receipt becomes B-capable only after this transaction really commits.
    pub(crate) fn record_sitemap_source_capture(
        &self,
        entry: &DiscoveryEntry,
        document: &crate::SitemapSourceDocument,
        measurement: &crate::SitemapSourceMeasurement,
        generation: u64,
    ) -> anyhow::Result<crate::sitemap_source_store::SourceMeasurementReceipt> {
        anyhow::ensure!(
            entry.kind == DiscoveryKind::Sitemap
                && document.kind == crate::SitemapSourceKind::SitemapXml
                && document.task_url == entry.url
                && document.generation == generation,
            "Capture A task/document identity differs"
        );
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        require_generation(&transaction, generation)?;
        anyhow::ensure!(
            matches!(load_run_plan(&transaction)?, RunPlan::FullCrawl),
            "Selected recrawl cannot record current sitemap captures"
        );
        let receipt = crate::sitemap_source_store::record_a(&transaction, document, measurement)?;
        transaction.commit()?;
        receipt.after_a_commit(&connection)
    }

    /// Honest bounded profiling omission. This writes a fixed CaptureUnavailable
    /// marker, never a fabricated measurement or a storage-budget reason.
    pub(crate) fn record_unavailable_sitemap_source_capture(
        &self,
        entry: &DiscoveryEntry,
        document: &crate::SitemapSourceDocument,
        capture_config_sha256: &str,
        generation: u64,
    ) -> anyhow::Result<crate::sitemap_source_store::SourceMeasurementReceipt> {
        anyhow::ensure!(
            entry.kind == DiscoveryKind::Sitemap
                && document.kind == crate::SitemapSourceKind::SitemapXml
                && document.task_url == entry.url
                && document.generation == generation,
            "Unavailable capture A task/document identity differs"
        );
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        require_generation(&transaction, generation)?;
        anyhow::ensure!(
            matches!(load_run_plan(&transaction)?, RunPlan::FullCrawl),
            "Selected recrawl cannot record current sitemap captures"
        );
        let receipt = crate::sitemap_source_store::record_unavailable_a(
            &transaction,
            document,
            capture_config_sha256,
        )?;
        transaction.commit()?;
        receipt.after_a_commit(&connection)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn complete_discovery_measured(
        &self,
        entry: &DiscoveryEntry,
        pages: &[QueueEntry],
        children: &[DiscoveryEntry],
        warnings: &[String],
        generation: u64,
        batch: crate::SitemapDiscoveryBatch,
        receipt: &crate::sitemap_source_store::SourceMeasurementReceipt,
    ) -> anyhow::Result<()> {
        self.complete_discovery_impl(
            entry,
            pages,
            children,
            warnings,
            generation,
            Some(batch),
            Some(receipt),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn complete_discovery_impl(
        &self,
        entry: &DiscoveryEntry,
        pages: &[QueueEntry],
        children: &[DiscoveryEntry],
        warnings: &[String],
        generation: u64,
        batch: Option<crate::SitemapDiscoveryBatch>,
        receipt: Option<&crate::sitemap_source_store::SourceMeasurementReceipt>,
    ) -> anyhow::Result<()> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let kind = sitemap_source_kind(&entry.kind);
        // First admission-sensitive operation: stale/repeated tasks cannot enqueue.
        let mut measurement_guard = receipt
            .map(|receipt| {
                crate::sitemap_source_store::verify_b_before_admission(
                    &transaction,
                    receipt,
                    &batch
                        .as_ref()
                        .context("Measured B needs original source batch")?
                        .document,
                )
            })
            .transpose()?;
        let token = if let Some(guard) = &mut measurement_guard {
            guard.take_pending()?
        } else {
            crate::sitemap_ledger::guard_pending_source(&transaction, generation, kind, &entry.url)?
        };
        if batch.is_none() {
            anyhow::ensure!(
                crate::sitemap_ledger::load_catalog(&transaction)?.is_none(),
                "Initialized sitemap catalog requires an observed source batch"
            );
        }
        if let Some(batch) = &batch {
            anyhow::ensure!(
                batch.document.kind == kind
                    && batch.document.task_url == entry.url
                    && batch.document.generation == generation,
                "Discovery/batch identity differs"
            );
            anyhow::ensure!(
                batch.document.complete() || (pages.is_empty() && children.is_empty()),
                "Incomplete sitemap source cannot admit page/child tasks"
            );
        }
        for page in pages {
            transaction.execute(
                "INSERT OR IGNORE INTO frontier(url,depth,discovered_from) VALUES(?1,?2,?3)",
                params![page.url, page.depth as i64, page.discovered_from],
            )?;
        }
        for child in children {
            enqueue_discovery(&transaction, child)?;
        }
        merge_discovery_warnings(&transaction, warnings)?;
        match batch {
            Some(batch) => {
                crate::sitemap_ledger::append_source(&transaction, token, batch)?;
            }
            None => {
                // No authoritative rows are invented for legacy compatibility callers.
                mark_discovery_done_once(&transaction, entry)?;
            }
        }
        if let Some(guard) = measurement_guard {
            crate::sitemap_source_store::link_b_after_append(&transaction, guard)?;
        }
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
        self.complete_discovery_with_policy_impl(
            entry,
            rows,
            source,
            warnings,
            generation,
            limits,
            None,
            None,
            |_, _, _| Ok(()),
        )
    }

    /// Root owns the finalizer: it projects original robots/root declarations from
    /// actual finalized rows. XML bindings are enforced separately by the Store.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn complete_discovery_with_policy_observed<F>(
        &self,
        entry: &DiscoveryEntry,
        rows: &mut [crate::PolicyOccurrence],
        source: (&str, bool),
        warnings: &[String],
        generation: u64,
        limits: &crate::LiveCrawlPolicyLimits,
        batch: crate::SitemapDiscoveryBatch,
        finalize: F,
    ) -> anyhow::Result<(Vec<QueueEntry>, Vec<DiscoveryEntry>)>
    where
        F: FnOnce(
            &rusqlite::Transaction<'_>,
            &mut crate::SitemapDiscoveryBatch,
            &[crate::PolicyOccurrence],
        ) -> anyhow::Result<()>,
    {
        self.complete_discovery_with_policy_impl(
            entry,
            rows,
            source,
            warnings,
            generation,
            limits,
            Some(batch),
            None,
            finalize,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn complete_discovery_with_policy_measured<F>(
        &self,
        entry: &DiscoveryEntry,
        rows: &mut [crate::PolicyOccurrence],
        source: (&str, bool),
        warnings: &[String],
        generation: u64,
        limits: &crate::LiveCrawlPolicyLimits,
        batch: crate::SitemapDiscoveryBatch,
        receipt: &crate::sitemap_source_store::SourceMeasurementReceipt,
        finalize: F,
    ) -> anyhow::Result<(Vec<QueueEntry>, Vec<DiscoveryEntry>)>
    where
        F: FnOnce(
            &rusqlite::Transaction<'_>,
            &mut crate::SitemapDiscoveryBatch,
            &[crate::PolicyOccurrence],
        ) -> anyhow::Result<()>,
    {
        self.complete_discovery_with_policy_impl(
            entry,
            rows,
            source,
            warnings,
            generation,
            limits,
            Some(batch),
            Some(receipt),
            finalize,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn complete_discovery_with_policy_impl<F>(
        &self,
        entry: &DiscoveryEntry,
        rows: &mut [crate::PolicyOccurrence],
        source: (&str, bool),
        warnings: &[String],
        generation: u64,
        limits: &crate::LiveCrawlPolicyLimits,
        mut batch: Option<crate::SitemapDiscoveryBatch>,
        receipt: Option<&crate::sitemap_source_store::SourceMeasurementReceipt>,
        finalize: F,
    ) -> anyhow::Result<(Vec<QueueEntry>, Vec<DiscoveryEntry>)>
    where
        F: FnOnce(
            &rusqlite::Transaction<'_>,
            &mut crate::SitemapDiscoveryBatch,
            &[crate::PolicyOccurrence],
        ) -> anyhow::Result<()>,
    {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        let kind = sitemap_source_kind(&entry.kind);
        let mut measurement_guard = receipt
            .map(|receipt| {
                crate::sitemap_source_store::verify_b_before_admission(
                    &transaction,
                    receipt,
                    &batch
                        .as_ref()
                        .context("Measured policy B needs original source batch")?
                        .document,
                )
            })
            .transpose()?;
        let token = if let Some(guard) = &mut measurement_guard {
            guard.take_pending()?
        } else {
            crate::sitemap_ledger::guard_pending_source(&transaction, generation, kind, &entry.url)?
        };
        if let Some(batch) = &batch {
            anyhow::ensure!(
                batch.document.kind == kind
                    && batch.document.task_url == entry.url
                    && batch.document.generation == generation,
                "Discovery/batch identity differs"
            );
            anyhow::ensure!(
                source.0 == batch.document.source_revision && source.1 == batch.document.complete(),
                "Policy source completeness/revision differs from sitemap observation"
            );
        } else {
            anyhow::ensure!(
                crate::sitemap_ledger::load_catalog(&transaction)?.is_none(),
                "Initialized sitemap catalog requires an observed source batch"
            );
        }
        anyhow::ensure!(
            rows.len() <= limits.max_batch_occurrences,
            "Policy source occurrence budget exceeded; source remains pending"
        );
        // SQL rollback must not leave caller rows pointing at rolled-back grants.
        // Copy bounded rows, publish final in-memory state only AFTER TX commit.
        let mut staged_rows = rows.to_vec();
        crate::policy_ledger::commit(
            &transaction,
            &mut staged_rows,
            generation,
            limits,
            Some(source),
        )?;
        if let Some(batch) = &mut batch {
            let original_document = batch.document.clone();
            finalize(&transaction, batch, &staged_rows)?;
            anyhow::ensure!(
                batch.document == original_document,
                "Policy finalizer altered original source observation facts"
            );
            if kind == crate::SitemapSourceKind::SitemapXml {
                crate::sitemap_ledger::bind_final_policy(&transaction, batch, &staged_rows)?;
            }
            anyhow::ensure!(
                batch.document.complete()
                    || staged_rows
                        .iter()
                        .all(|row| row.scheduled_fetch_url.is_none()),
                "Incomplete sitemap source cannot admit policy tasks"
            );
        }
        let mut pages = Vec::new();
        let mut children = Vec::new();
        for row in &staged_rows {
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
        merge_discovery_warnings(&transaction, warnings)?;
        match batch {
            Some(batch) => {
                crate::sitemap_ledger::append_source(&transaction, token, batch)?;
            }
            None => {
                mark_discovery_done_once(&transaction, entry)?;
            }
        }
        if let Some(guard) = measurement_guard {
            crate::sitemap_source_store::link_b_after_append(&transaction, guard)?;
        }
        if discovery_pending_count(&transaction)? == 0 {
            put_metadata(&transaction, "discovery_complete", "true")?;
        }
        transaction.commit()?;
        rows.clone_from_slice(&staged_rows);
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
        crate::validate_page_extraction_observations(page)?;
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
                let page: PageRecord = crate::heap_deserialize::from_str(&value)?;
                crate::validate_page_content_captures(&page)?;
                crate::validate_page_extraction_observations(&page)?;
                Ok(page)
            })
            .collect()
    }

    pub fn save_report(&self, report: &CrawlReport) -> anyhow::Result<()> {
        crate::validate_report_extraction_observations(report)?;
        crate::validate_sitemap_coverage_inventory(
            &report.config,
            report.sitemap_catalog.as_ref(),
            report.sitemap_coverage_inventory.as_ref(),
        )?;
        if let Some(catalog) = &report.sitemap_catalog {
            crate::sitemap_ledger::validate_catalog(catalog)?;
        }
        if let Some(snapshot) = &report.sitemap_source_measurements {
            crate::sitemap_source_store::validate_snapshot(snapshot)?;
            crate::sitemap_source_store::validate_catalog_binding(
                snapshot,
                report
                    .sitemap_catalog
                    .as_ref()
                    .context("Report measurements lack source catalog")?,
            )?;
        }
        let report_text = crate::recrawl::bounded_report_json(report)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        // A stored report may be older, but the current database cannot silently
        // lose required immutable selected-parent history before any publication.
        load_sitemap_source_measurements_verified(&transaction)?;
        transaction.execute(
            "INSERT OR REPLACE INTO metadata(key,value) VALUES('report',?1)",
            [report_text],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn save_checkpoint(
        &self,
        report: &CrawlReport,
        outcome: CrawlOutcome,
        generation: u64,
    ) -> anyhow::Result<CrawlCheckpoint> {
        let report_text = crate::recrawl::bounded_report_json(report)?;
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
            crate::validate_page_extraction_observations(page)?;
            if page.content_captures.is_some() {
                let stored: String = transaction.query_row(
                    "SELECT data FROM pages WHERE url=?1",
                    [&page.url],
                    |row| row.get(0),
                )?;
                let stored: PageRecord = crate::heap_deserialize::from_str(&stored)?;
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
        let sitemap_catalog = load_sitemap_catalog_verified(&transaction)?;
        anyhow::ensure!(
            report.sitemap_catalog == sitemap_catalog,
            "Report sitemap catalog differs from committed evidence"
        );
        let coverage_snapshot = load_sitemap_read_snapshot_verified(&transaction)?;
        anyhow::ensure!(
            report.sitemap_coverage_inventory == coverage_snapshot.coverage_inventory,
            "Report sitemap coverage inventory differs from committed discovery context"
        );
        anyhow::ensure!(
            report.sitemap_source_measurements == coverage_snapshot.sitemap_source_measurements,
            "Report sitemap source measurements differ from committed capture/link context"
        );
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
            sitemap_ledger: sitemap_catalog.map(|catalog| catalog.summary),
            sitemap_source_measurements: coverage_snapshot
                .sitemap_source_measurements
                .map(|snapshot| snapshot.summary),
        };
        if report.policy_ledger != checkpoint.policy_ledger {
            bail!("Report policy ledger differs from committed evidence");
        }
        if report.resource_ledger != checkpoint.resource_ledger {
            bail!("Report resource ledger differs from committed evidence");
        }
        crate::crawl_quota_ledger::bind_checkpoint(&transaction, &report.config, generation)?;
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

    /// Returns the saved immutable report snapshot, which may predate active SQL rows.
    /// inspection/save_checkpoint bind it to current rows only for a matching checkpoint.
    pub fn load_report(&self) -> anyhow::Result<Option<CrawlReport>> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| anyhow::anyhow!("Storage lock poisoned"))?;
        let transaction = connection.transaction()?;
        load_sitemap_catalog_verified(&transaction)?;
        load_sitemap_source_measurements_verified(&transaction)?;
        let value = bounded_saved_report_text(&transaction)?;
        let report = value
            .map(|value| -> anyhow::Result<CrawlReport> {
                let report: CrawlReport = crate::heap_deserialize::from_str(&value)?;
                crate::validate_sitemap_coverage_inventory(
                    &report.config,
                    report.sitemap_catalog.as_ref(),
                    report.sitemap_coverage_inventory.as_ref(),
                )?;
                for page in &report.pages {
                    crate::validate_page_content_captures(page)?;
                    crate::validate_page_extraction_observations(page)?;
                }
                if let Some(catalog) = &report.sitemap_catalog {
                    crate::sitemap_ledger::validate_catalog(catalog)?;
                }
                if let Some(snapshot) = &report.sitemap_source_measurements {
                    crate::sitemap_source_store::validate_snapshot(snapshot)?;
                    crate::sitemap_source_store::validate_catalog_binding(
                        snapshot,
                        report
                            .sitemap_catalog
                            .as_ref()
                            .context("Saved measurements lack source catalog")?,
                    )?;
                }
                Ok(report)
            })
            .transpose()?;
        transaction.commit()?;
        Ok(report)
    }
}

fn sitemap_source_kind(kind: &DiscoveryKind) -> crate::SitemapSourceKind {
    match kind {
        DiscoveryKind::Robots => crate::SitemapSourceKind::RobotsDirectives,
        DiscoveryKind::Sitemap => crate::SitemapSourceKind::SitemapXml,
    }
}

fn bounded_saved_report_text(connection: &Connection) -> anyhow::Result<Option<String>> {
    let length: Option<i64> = connection
        .query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key='report'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    anyhow::ensure!(
        length.is_none_or(|n| (1..=268_435_456).contains(&n)),
        "Saved portable report exceeds 256 MiB"
    );
    let text = metadata_text(connection, "report")?;
    anyhow::ensure!(
        match (&text, length) {
            (None, None) => true,
            (Some(value), Some(length)) => i64::try_from(value.len())? == length,
            _ => false,
        },
        "Saved portable report byte extent differs"
    );
    Ok(text)
}

fn mark_discovery_done_once(
    transaction: &rusqlite::Transaction<'_>,
    entry: &DiscoveryEntry,
) -> anyhow::Result<()> {
    let changed = transaction.execute(
        "UPDATE discovery_frontier SET done=1 WHERE kind=?1 AND url=?2 AND done=0",
        params![entry.kind.as_str(), entry.url],
    )?;
    anyhow::ensure!(
        changed == 1,
        "Source commit did not affect exactly one pending task"
    );
    Ok(())
}

fn merge_discovery_warnings(
    transaction: &rusqlite::Transaction<'_>,
    warnings: &[String],
) -> anyhow::Result<()> {
    let mut saved: Vec<String> = metadata(transaction, "discovery_warnings")?.unwrap_or_default();
    for warning in warnings {
        if !saved.contains(warning) {
            saved.push(warning.clone());
        }
    }
    put_metadata(
        transaction,
        "discovery_warnings",
        &serde_json::to_string(&saved)?,
    )
}

/// Full selected validator first, exact inherited catalog anchor second. The
/// ledger SQL anchor projection alone is intentionally insufficient here.
fn load_sitemap_catalog_verified(
    connection: &Connection,
) -> anyhow::Result<Option<crate::SitemapCatalog>> {
    if let Some(config_text) = metadata_text(connection, "config")? {
        let config: CrawlConfig = serde_json::from_str(&config_text)?;
        verify_sitemap_config_context(connection, &config)?;
    }
    let plan = load_run_plan(connection)?;
    if matches!(plan, RunPlan::SelectedRecrawl(_)) {
        verify_recrawl_rows(connection, &plan)?;
    }
    let catalog = crate::sitemap_ledger::load_catalog(connection)?;
    if let RunPlan::SelectedRecrawl(plan) = plan {
        #[derive(serde::Deserialize)]
        struct ParentCatalog {
            sitemap_catalog: Option<crate::SitemapCatalog>,
        }
        let bytes = parent_snapshot(connection)?
            .context("Selected sitemap read lacks exact parent bytes")?;
        // load_run_plan has already verified whole exact bytes/config/typed plan.
        let parent = serde_json::from_slice::<ParentCatalog>(&bytes)?.sitemap_catalog;
        match (&parent, &catalog) {
            (None, None) => {} // Legacy absence is unknown, never invented empty history.
            (Some(_), Some(catalog)) => {
                anyhow::ensure!(
                    catalog
                        .inherited
                        .as_ref()
                        .is_some_and(|basis| basis.parent_snapshot_sha256
                            == plan.parent.snapshot_sha256
                            && !basis.current_discovery_executed),
                    "Selected sitemap catalog lacks immutable inherited basis"
                );
                // load_catalog verifies exact parent BLOB + canonical typed catalog rows.
            }
            _ => bail!("Selected sitemap catalog presence differs from immutable parent"),
        }
    }
    Ok(catalog)
}

fn verify_sitemap_config_context(
    connection: &Connection,
    config: &CrawlConfig,
) -> anyhow::Result<()> {
    for input in &config.input_imports {
        crate::validate_import_result(input)?;
    }
    if let Some(policy) = &config.crawl_policy {
        crate::policy_ledger::verify(connection, &crate::CompiledCrawlPolicy::new(policy)?)?;
    }
    Ok(())
}

/// ALL context used by the coverage projection is captured in this caller-owned
/// transaction; no migrations, source HTTP, report fallback, or independently read task lists.
fn load_sitemap_read_snapshot_verified(
    connection: &Connection,
) -> anyhow::Result<SitemapReadSnapshot> {
    let catalog = load_sitemap_catalog_verified(connection)?;
    let sitemap_source_measurements = load_sitemap_source_measurements_verified(connection)?;
    let sitemap_source_measurement_context =
        crate::sitemap_source_store::snapshot_context(sitemap_source_measurements.as_ref())?;
    let generation = metadata::<CrawlRunState>(connection, "run_state")?
        .map(|run| run.generation)
        .filter(|generation| *generation > 0);
    let length: Option<i64> = connection
        .query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key='config'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let mut saved_config = Vec::new();
    let crawl_config_sha256 = length
        .map(|length| -> anyhow::Result<String> {
            anyhow::ensure!(
                (1..=67_108_864).contains(&length),
                "Saved coverage config exceeds bounded input"
            );
            // One bounded exact byte read. Repeated substr(CAST(full TEXT AS
            // BLOB)) can recast the entire value for every small chunk.
            saved_config = connection.query_row(
                "SELECT CAST(value AS BLOB) FROM metadata WHERE key='config'",
                [],
                |row| row.get(0),
            )?;
            anyhow::ensure!(
                i64::try_from(saved_config.len())? == length,
                "Saved config byte length differs"
            );
            Ok(format!("{:x}", Sha256::digest(&saved_config)))
        })
        .transpose()?;
    let coverage_inventory = match catalog.as_ref() {
        None => None,
        Some(catalog) if catalog.inherited.is_some() => {
            let parent_bytes = parent_snapshot(connection)?
                .context("Inherited coverage lacks immutable parent bytes")?;
            let parent: CrawlReport = crate::heap_deserialize::from_slice(&parent_bytes)?;
            crate::validate_sitemap_coverage_inventory(
                &parent.config,
                parent.sitemap_catalog.as_ref(),
                parent.sitemap_coverage_inventory.as_ref(),
            )?;
            parent
                .sitemap_coverage_inventory
                .map(|inventory| {
                    inventory.bind_inherited_parent(
                        parent
                            .sitemap_catalog
                            .as_ref()
                            .context("Inherited coverage parent catalog missing")?,
                        &parent_bytes,
                    )
                })
                .transpose()?
        }
        Some(catalog) => {
            match (crawl_config_sha256.as_deref(), generation) {
                (Some(sha), Some(generation)) => {
                    let initialized = metadata_text(connection, "discovery_initialized")?
                        .as_deref()
                        == Some("true");
                    let exists: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='discovery_frontier')", [], |row| row.get(0))?;
                    let mut tasks = Vec::new();
                    if exists {
                        let (rows, bytes): (i64,i64) = connection.query_row("SELECT count(*),coalesce(sum(length(CAST(url AS BLOB))),0) FROM discovery_frontier", [], |row| Ok((row.get(0)?,row.get(1)?)))?;
                        anyhow::ensure!(
                            (0..=1_000_000).contains(&rows) && (0..=33_554_432).contains(&bytes),
                            "Coverage task inventory budget exceeded"
                        );
                        let mut query = connection.prepare(
                            "SELECT kind,url,done FROM discovery_frontier ORDER BY kind,url",
                        )?;
                        let mut values = query.query([])?;
                        while let Some(row) = values.next()? {
                            let kind: String = row.get(0)?;
                            let done: i64 = row.get(2)?;
                            anyhow::ensure!(
                                matches!(done, 0 | 1),
                                "Invalid coverage discovery done flag"
                            );
                            tasks.push(crate::SitemapCoverageTask {
                                kind: match kind.as_str() {
                                    "robots" => crate::SitemapSourceKind::RobotsDirectives,
                                    "sitemap" => crate::SitemapSourceKind::SitemapXml,
                                    _ => bail!("Unknown coverage discovery task kind"),
                                },
                                task_url: row.get(1)?,
                                done: done == 1,
                            });
                        }
                    }
                    let config: CrawlConfig = serde_json::from_slice(&saved_config)?;
                    let initial_policy_urls = if config.crawl_policy.is_some() {
                        // Existing policy verification ran above. Only ACTUAL finalized initial
                        // page admissions determine policy-mode Robots origins, including aliases.
                        let mut query = connection.prepare("SELECT json_extract(data,'$.occurrence.decision.fetch_candidate_url') FROM policy_occurrences WHERE json_extract(data,'$.occurrence.input_basis')='declared_input' AND json_extract(data,'$.occurrence.task_role')='page_document' AND json_extract(data,'$.occurrence.decision.source_kind') IN ('seed','list_page') AND json_extract(data,'$.occurrence.admission_reason') IN ('scheduled','coalesced') ORDER BY sequence")?;
                        let values = query
                            .query_map([], |row| row.get::<_, String>(0))?
                            .collect::<Result<Vec<_>, _>>()?;
                        Some(values)
                    } else {
                        None
                    };
                    Some(crate::SitemapCoverageInventory::from_saved_snapshot(
                        catalog,
                        &saved_config,
                        sha,
                        generation,
                        initialized,
                        tasks,
                        initial_policy_urls.as_deref(),
                    )?)
                }
                _ => None,
            }
        }
    };
    let coverage = crate::project_sitemap_coverage(catalog.as_ref(), coverage_inventory.as_ref())?;
    Ok(SitemapReadSnapshot {
        crawl_config_sha256,
        generation,
        catalog,
        coverage_inventory,
        coverage,
        membership_analysis: None,
        sitemap_source_measurements,
        sitemap_source_measurement_context,
    })
}

/// Sidecar and exact parent validation use the caller's existing transaction.
/// No report fallback, independently opened connection or read-time migration.
fn load_sitemap_source_measurements_verified(
    connection: &Connection,
) -> anyhow::Result<Option<crate::SitemapSourceStoreSnapshot>> {
    let plan = load_run_plan(connection)?;
    if matches!(plan, RunPlan::SelectedRecrawl(_)) {
        verify_recrawl_rows(connection, &plan)?;
    }
    let snapshot = crate::sitemap_source_store::load(connection)?;
    if let RunPlan::SelectedRecrawl(selected) = &plan {
        let bytes = parent_snapshot(connection)?
            .context("Selected source history lacks exact parent bytes")?;
        let config: CrawlConfig = metadata(connection, "config")?
            .context("Selected source history lacks actual saved configuration")?;
        // Full plan reconstruction validates the actual parent BLOB hash,
        // selection/config and portable parent report before examining presence.
        // Do not replace this with a metadata/inherited-flag-only check.
        let prepared = crate::recrawl::validate_saved_plan(selected, &bytes, &config)?;
        match (
            prepared.base_report.sitemap_source_measurements.as_ref(),
            snapshot.as_ref(),
        ) {
            (None, None) => {} // Historical absence stays unknown.
            (Some(parent), Some(current)) => {
                let expected = crate::sitemap_source_store::project_inherited_snapshot(
                    parent,
                    &selected.parent.snapshot_sha256,
                )?;
                anyhow::ensure!(
                    &expected == current,
                    "Selected source history differs from exact immutable parent projection"
                );
            }
            _ => bail!("Selected source history presence differs from immutable parent"),
        }
    }
    Ok(snapshot)
}

/// One bounded row at a time; the SQLite run state is an internal provenance
/// boundary, not authentication of imported or externally rewritten databases.
fn verify_saved_content_captures(connection: &Connection) -> anyhow::Result<()> {
    let Some(config) = metadata::<CrawlConfig>(connection, "config")? else {
        return Ok(());
    };
    if config.content_capture.is_none() {
        let has_captures: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM pages WHERE json_type(data,'$.content_captures') IS NOT NULL AND json_type(data,'$.content_captures') <> 'null' LIMIT 1)", [], |row| row.get(0))?;
        let has_extraction_observations: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM pages, json_each(pages.data, '$.custom_extractions') AS extraction WHERE json_type(extraction.value, '$.source_observation') IS NOT NULL AND json_type(extraction.value, '$.source_observation') <> 'null' UNION ALL SELECT 1 FROM pages, json_each(pages.data, '$.raw_metadata.custom_extractions') AS extraction WHERE json_type(extraction.value, '$.source_observation') IS NOT NULL AND json_type(extraction.value, '$.source_observation') <> 'null' LIMIT 1)",
            [], |row| row.get(0),
        )?;
        if !has_captures && !has_extraction_observations {
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
        let page: PageRecord = crate::heap_deserialize::from_str(&row.get::<_, String>(1)?)?;
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
        crate::validate_page_extraction_observations(&page)?;
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
    let base: CrawlReport = crate::heap_deserialize::from_slice(&bytes)?;
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
                let fresh: PageRecord = crate::heap_deserialize::from_str(&data)?;
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
            sitemap_catalog: None,
            sitemap_coverage_inventory: None,
            sitemap_source_measurements: None,
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

#[cfg(test)]
#[path = "store-sitemap-coverage-tests.rs"]
mod sitemap_coverage_store_tests;

#[cfg(test)]
#[path = "store-sitemap-config-read-tests.rs"]
mod sitemap_config_read_tests;

#[cfg(test)]
#[path = "store-sitemap-tests.rs"]
mod sitemap_store_tests;

#[path = "store/sitemap_source_archive.rs"]
mod source_body_archive;
#[cfg(test)]
#[path = "store-sitemap-body-tests.rs"]
mod source_body_archive_tests;

#[cfg(test)]
#[path = "store-sitemap-source-tests.rs"]
mod sitemap_source_store_integration_tests;

// Append as a sibling cfg(test) module in crates/core/src/store.rs.
// AUTHORED ONLY; Root owns integration and test execution.
#[cfg(test)]
mod explicit_sitemap_input_regressions {
    use super::*;

    fn configured_store(config: &CrawlConfig) -> (tempfile::TempDir, CrawlStore) {
        let temp = tempfile::tempdir().unwrap();
        let store = CrawlStore::open(temp.path().join("owned-input-fixture.sqlite")).unwrap();
        store.prepare(config).unwrap();
        (temp, store)
    }
    fn normalized_config(original: Option<Vec<String>>) -> CrawlConfig {
        let config = CrawlConfig {
            seed_urls: vec!["https://example.test/".into()],
            sitemap_urls: vec!["https://example.test/map.xml".into()],
            explicit_sitemap_inputs: original,
            ..Default::default()
        };
        crate::Crawler::from_saved_config(config)
            .unwrap()
            .effective_config()
            .clone()
    }

    #[test]
    fn legacy_unknown_resume_allows_only_four_operational_overrides() {
        let original = normalized_config(None);
        let (_temp, store) = configured_store(&original);
        let mut resumed = original.clone();
        resumed.max_pages = Some(200);
        resumed.concurrency = 3;
        resumed.delay_ms = 7;
        resumed.request_timeout_secs = 42;
        assert!(store.prepare(&resumed).unwrap());
        let saved = store.config().unwrap().unwrap();
        assert_eq!(saved.explicit_sitemap_inputs, None);
        assert_eq!(
            serde_json::to_value(&saved).unwrap(),
            serde_json::to_value(&resumed).unwrap()
        );
    }

    #[test]
    fn unknown_and_known_empty_provenance_are_not_interchangeable_on_resume() {
        for (saved, replacement) in [(None, Some(vec![])), (Some(vec![]), None)] {
            let original = normalized_config(saved);
            let (_temp, store) = configured_store(&original);
            let before = metadata_text(&store.connection.lock().unwrap(), "config").unwrap();
            let mut changed = original;
            changed.explicit_sitemap_inputs = replacement;
            assert!(store.prepare(&changed).is_err());
            assert_eq!(
                metadata_text(&store.connection.lock().unwrap(), "config").unwrap(),
                before
            );
        }
    }

    #[test]
    fn rejected_raw_reorder_dedup_and_rewrite_leave_saved_config_unchanged() {
        let raw = vec![
            "https://example.test/map.xml#first".into(),
            "https://example.test/map.xml#second".into(),
            "https://example.test/map.xml#first".into(),
        ];
        let original = normalized_config(Some(raw.clone()));
        let (_temp, store) = configured_store(&original);
        let before = metadata_text(&store.connection.lock().unwrap(), "config").unwrap();
        let variants = vec![
            vec![raw[1].clone(), raw[0].clone(), raw[2].clone()],
            vec![raw[0].clone(), raw[1].clone()],
            vec!["https://example.test/map.xml".into(); 3],
        ];
        for replacement in variants {
            let mut changed = original.clone();
            changed.explicit_sitemap_inputs = Some(replacement);
            assert!(store.prepare(&changed).is_err());
            assert_eq!(
                metadata_text(&store.connection.lock().unwrap(), "config").unwrap(),
                before
            );
        }
    }
}
