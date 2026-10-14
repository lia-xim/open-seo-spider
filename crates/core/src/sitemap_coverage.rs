//! Pure projection of captured sitemap sources. The stored ledger is never rewritten.
//!
//! Inventory must come from the SAME read transaction as the validated catalog.
//! In particular, an empty catalog alone does not prove an empty root cohort:
//! a pending robots task has not yet supplied its declarations.
use crate::{
    CrawlConfig, SitemapAdmissionBinding, SitemapAdmissionDecision as Admission, SitemapCatalog,
    SitemapLedgerRow, SitemapRootDeclaration, SitemapRootKind, SitemapSourceDocument,
    SitemapSourceKind,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const SITEMAP_COVERAGE_ENGINE: &str = "open-seo-spider/declared-sitemap-source-coverage-v1";
const MAX_INVENTORY_TASKS: usize = 1_000_000;
const MAX_INVENTORY_TASK_BYTES: usize = 32 * 1024 * 1024;
const MAX_SAVED_CONFIG_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapCoverageTask {
    pub kind: SitemapSourceKind,
    pub task_url: String,
    /// Actual discovery_frontier value; a done flag never substitutes for a source observation.
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum InventoryBasis {
    Current,
    InheritedParent {
        parent_snapshot_sha256: String,
        parent_catalog_sha256: String,
    },
}

/// Captured discovery inventory, not caller-provided completeness flags.
/// The constructor binds exact persisted config bytes and derives original root coordinates.
/// Store checkpoints retain this DTO in immutable reports for historical coverage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapCoverageInventory {
    basis: InventoryBasis,
    saved_config_sha256: String,
    catalog_initial_config_sha256: String,
    generation: u64,
    discovery_initialized: bool,
    expected_initial_roots: Option<Vec<SitemapRootDeclaration>>,
    expected_robots_sources: Option<Vec<String>>,
    tasks: Vec<SitemapCoverageTask>,
}

impl SitemapCoverageInventory {
    /// `saved_config_bytes`, generation, initialized marker, and ALL discovery tasks
    /// are read from one validated existing SQLite transaction, with no migration/HTTP.
    /// The config hash is the actual byte hash from that transaction, not reserialization.
    pub fn from_saved_snapshot(
        catalog: &SitemapCatalog,
        saved_config_bytes: &[u8],
        saved_config_sha256: &str,
        generation: u64,
        discovery_initialized: bool,
        mut tasks: Vec<SitemapCoverageTask>,
        initial_policy_page_urls: Option<&[String]>,
    ) -> Result<Self> {
        ensure!(
            catalog.inherited.is_none(),
            "Current inventory cannot replace inherited history"
        );
        crate::sitemap_ledger::validate_catalog(catalog)?;
        ensure!(
            !saved_config_bytes.is_empty() && saved_config_bytes.len() <= MAX_SAVED_CONFIG_BYTES,
            "Saved coverage config exceeds bounded input"
        );
        ensure!(generation > 0, "Missing coverage inventory generation");
        ensure!(
            crate::sitemap_ledger::digest(saved_config_bytes) == saved_config_sha256,
            "Coverage saved config byte SHA differs"
        );
        let config: CrawlConfig = serde_json::from_slice(saved_config_bytes)
            .context("Invalid saved coverage configuration")?;
        config.validate_input_provenance_budget()?;
        let expected_initial_roots = crate::sitemap_initial_roots::initial_roots(
            config.explicit_sitemap_inputs.as_deref(),
            &config.input_imports,
            &catalog.summary.config_sha256,
            None,
        )?
        .map(|items| items.into_iter().map(|item| item.root).collect());
        let expected_robots_sources =
            if config.crawl_links && config.discover_sitemaps && config.obey_robots {
                let pages = if config.crawl_policy.is_some() {
                    initial_policy_page_urls
                } else {
                    Some(if config.seed_urls.is_empty() {
                        config.sitemap_urls.as_slice()
                    } else {
                        config.seed_urls.as_slice()
                    })
                };
                pages.map(robots_origins).transpose()?
            } else {
                Some(vec![])
            };
        ensure!(
            tasks.len() <= MAX_INVENTORY_TASKS,
            "Coverage task inventory row budget exceeded"
        );
        let mut bytes = 0usize;
        for task in &tasks {
            bytes = bytes
                .checked_add(task.task_url.len())
                .context("Coverage task byte overflow")?;
            ensure!(
                !task.task_url.is_empty() && bytes <= MAX_INVENTORY_TASK_BYTES,
                "Coverage task inventory text budget exceeded"
            );
        }
        tasks.sort_by(|a, b| task_key(a).cmp(&task_key(b)));
        ensure!(
            !tasks
                .windows(2)
                .any(|pair| task_key(&pair[0]) == task_key(&pair[1])),
            "Duplicate coverage task inventory identity"
        );
        Ok(Self {
            basis: InventoryBasis::Current,
            saved_config_sha256: saved_config_sha256.into(),
            catalog_initial_config_sha256: catalog.summary.config_sha256.clone(),
            generation,
            discovery_initialized,
            expected_initial_roots,
            expected_robots_sources,
            tasks,
        })
    }

    /// Rebind an already captured parent inventory to actual immutable parent bytes.
    /// The inventory must be persisted together with those bytes by the Store owner.
    /// This never invents tasks from an inherited child frontier or an empty task list.
    pub fn bind_inherited_parent(
        mut self,
        parent: &SitemapCatalog,
        parent_snapshot_bytes: &[u8],
    ) -> Result<Self> {
        ensure!(
            !parent_snapshot_bytes.is_empty() && parent_snapshot_bytes.len() <= 268_435_456,
            "Inherited coverage snapshot bound exceeded"
        );
        // A qualified Unknown result is not proof that the inventory belongs to
        // this parent. Never repair an incompatible historical basis by rebinding.
        let parent_coverage = project_sitemap_coverage(Some(parent), Some(&self))?;
        ensure!(
            parent_coverage
                .basis
                .as_ref()
                .is_some_and(|basis| basis.captured_inventory_sha256.is_some()),
            "Coverage inventory does not bind the actual parent basis"
        );
        #[derive(Deserialize)]
        struct Parent {
            sitemap_catalog: Option<SitemapCatalog>,
            sitemap_coverage_inventory: Option<SitemapCoverageInventory>,
        }
        let snapshot: Parent = serde_json::from_slice(parent_snapshot_bytes)
            .context("Invalid immutable coverage parent snapshot")?;
        ensure!(
            snapshot.sitemap_catalog.as_ref() == Some(parent),
            "Coverage parent catalog differs"
        );
        ensure!(
            snapshot.sitemap_coverage_inventory.as_ref() == Some(&self),
            "Parent bytes do not bind captured coverage inventory"
        );
        self.basis = InventoryBasis::InheritedParent {
            parent_snapshot_sha256: crate::sitemap_ledger::digest(parent_snapshot_bytes),
            parent_catalog_sha256: crate::sitemap_ledger::digest(serde_json::to_vec(parent)?),
        };
        Ok(self)
    }

    fn validate(&self) -> Result<()> {
        ensure!(self.generation > 0, "Missing coverage inventory generation");
        for sha in [
            &self.saved_config_sha256,
            &self.catalog_initial_config_sha256,
        ] {
            ensure!(
                sha.len() == 64
                    && sha
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "Invalid coverage SHA"
            );
        }
        if let InventoryBasis::InheritedParent {
            parent_snapshot_sha256,
            parent_catalog_sha256,
        } = &self.basis
        {
            for sha in [parent_snapshot_sha256, parent_catalog_sha256] {
                ensure!(
                    sha.len() == 64
                        && sha
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                    "Invalid inherited coverage SHA"
                );
            }
        }
        ensure!(
            self.tasks.len() <= MAX_INVENTORY_TASKS,
            "Coverage task inventory row budget exceeded"
        );
        let mut bytes = 0usize;
        let mut previous = None;
        for task in &self.tasks {
            bytes = bytes
                .checked_add(task.task_url.len())
                .context("Coverage task byte overflow")?;
            ensure!(
                bytes <= MAX_INVENTORY_TASK_BYTES,
                "Coverage task inventory text budget exceeded"
            );
            ensure!(
                crate::normalize_url(&task.task_url, None)
                    .is_some_and(|url| url.as_str() == task.task_url),
                "Unsafe coverage task identity"
            );
            let key = task_key(task);
            ensure!(
                previous.is_none_or(|old| old < key),
                "Coverage tasks must be uniquely sorted"
            );
            previous = Some(key);
        }
        if let Some(urls) = &self.expected_robots_sources {
            ensure!(
                urls.len() <= MAX_INVENTORY_TASKS,
                "Coverage Robots inventory row budget exceeded"
            );
            let mut previous: Option<&str> = None;
            for url in urls {
                ensure!(
                    crate::normalize_url(url, None).is_some_and(
                        |parsed| url == &format!("{}/", parsed.origin().ascii_serialization())
                    ),
                    "Unsafe coverage Robots task identity"
                );
                ensure!(
                    previous.is_none_or(|old| old < url.as_str()),
                    "Coverage Robots tasks must be uniquely sorted"
                );
                previous = Some(url);
            }
        }
        ensure!(
            serde_json::to_vec(self)?.len() <= MAX_SAVED_CONFIG_BYTES,
            "Coverage inventory serialized budget exceeded"
        );
        Ok(())
    }
}

/// Validate a portable inventory against the report's original configuration.
/// Absence remains compatible with old reports and cannot prove a negative.
pub fn validate_sitemap_coverage_inventory(
    config: &CrawlConfig,
    catalog: Option<&SitemapCatalog>,
    inventory: Option<&SitemapCoverageInventory>,
) -> Result<()> {
    let Some(inventory) = inventory else {
        return Ok(());
    };
    inventory.validate()?;
    let catalog = catalog.context("Coverage inventory requires a sitemap catalog")?;
    let expected = crate::sitemap_initial_roots::initial_roots(
        config.explicit_sitemap_inputs.as_deref(),
        &config.input_imports,
        &catalog.summary.config_sha256,
        None,
    )?
    .map(|items| items.into_iter().map(|item| item.root).collect::<Vec<_>>());
    ensure!(
        inventory.expected_initial_roots == expected,
        "Coverage original roots differ from report configuration"
    );
    if !(config.crawl_links && config.discover_sitemaps && config.obey_robots) {
        ensure!(
            inventory.expected_robots_sources.as_deref() == Some(&[]),
            "Disabled Robots discovery has an unexpected required-source inventory"
        );
    } else if config.crawl_policy.is_none() {
        let expected = robots_origins(if config.seed_urls.is_empty() {
            &config.sitemap_urls
        } else {
            &config.seed_urls
        })?;
        ensure!(
            inventory.expected_robots_sources.as_ref() == Some(&expected),
            "Coverage required Robots roots differ from report configuration"
        );
    }
    project_sitemap_coverage(Some(catalog), Some(inventory))?;
    Ok(())
}

fn task_key(task: &SitemapCoverageTask) -> (&str, &str) {
    (task.kind.frontier_kind(), task.task_url.as_str())
}

fn robots_origins(urls: &[String]) -> Result<Vec<String>> {
    urls.iter()
        .map(|url| {
            let url =
                crate::normalize_url(url, None).context("Invalid required Robots source input")?;
            Ok(format!("{}/", url.origin().ascii_serialization()))
        })
        .collect::<Result<BTreeSet<_>>>()
        .map(|origins| origins.into_iter().collect())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapCohortCompletion {
    /// Every declared root and recursively admitted child has complete captured source evidence.
    Complete,
    /// Original initial roots are known empty and every captured robots source declares zero roots.
    KnownEmpty,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapCoverageScope {
    CurrentDeclaredSources,
    InheritedHistoricalDeclaredSources,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SitemapCoverageReason {
    CatalogUnavailable,
    CatalogRowsUnavailable,
    LegacyHistoryUnknown,
    SourceInventoryUnavailable,
    HistoricalInventoryUnavailable,
    DiscoveryNotInitialized,
    InitialRootInventoryUnknown,
    MissingInitialRootDeclaration,
    UnexpectedInitialRootDeclaration,
    InitialRootCoordinatesDiffer,
    SourceTaskMissing,
    PendingSource,
    DoneSourceWithoutObservation,
    ObservationWithoutTask,
    SourceGenerationAfterInventory,
    SourceNotComplete,
    SourceRequestNotStarted,
    SourceTransportUnavailable,
    SourceHttpNot200,
    SourceBodyIncomplete,
    SourceBlockedByRobots,
    SourceTransportFailed,
    SourceDecodeUnavailable,
    SourceParserIncomplete,
    SourceDeclarationsIncomplete,
    SourceRowsIncomplete,
    SourceFailed,
    RootInvalid,
    RootDenied,
    RootUnbound,
    ChildInvalid,
    ChildDenied,
    ChildUnbound,
    SourceGraphUnboundTask,
    RequiredRobotsInventoryUnknown,
    RequiredRobotsTaskMissing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapCoverageBasis {
    pub initial_config_sha256: String,
    pub catalog_revision: u64,
    pub catalog_rolling_sha256: String,
    pub captured_inventory_sha256: Option<String>,
    pub inventory_saved_config_sha256: Option<String>,
    pub inventory_generation: Option<u64>,
    pub parent_snapshot_sha256: Option<String>,
    pub parent_catalog_sha256: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapCoverageCounts {
    pub root_declarations: u64,
    pub child_declarations: u64,
    pub discovery_tasks: u64,
    pub observed_documents: u64,
    pub complete_documents: u64,
    pub reached_xml_tasks: u64,
    pub invalid_or_missing_page_locations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapCoverage {
    pub engine: String,
    pub scope: SitemapCoverageScope,
    pub completion: SitemapCohortCompletion,
    /// A narrow negative about this captured declared source cohort, never the entire website.
    pub current_valid_original_location_negative_permitted: bool,
    pub historical_valid_original_location_negative_permitted: bool,
    /// Counts are per offending captured source/declaration/task; one item may have several reasons.
    pub reason_counts: BTreeMap<SitemapCoverageReason, u64>,
    pub counts: SitemapCoverageCounts,
    pub basis: Option<SitemapCoverageBasis>,
}

fn reason(out: &mut SitemapCoverage, value: SitemapCoverageReason) {
    *out.reason_counts.entry(value).or_default() += 1;
}

/// Project only captured sources and admission edges; no target-page response/status joins.
/// A positive page Loc membership remains independent of this completeness result.
/// Invalid/corrupt catalogs are errors, not weaker partial results.
pub fn project_sitemap_coverage(
    catalog: Option<&SitemapCatalog>,
    inventory: Option<&SitemapCoverageInventory>,
) -> Result<SitemapCoverage> {
    use SitemapCoverageReason as R;
    let mut out = SitemapCoverage {
        engine: SITEMAP_COVERAGE_ENGINE.into(),
        scope: SitemapCoverageScope::Unavailable,
        completion: SitemapCohortCompletion::Unknown,
        current_valid_original_location_negative_permitted: false,
        historical_valid_original_location_negative_permitted: false,
        reason_counts: BTreeMap::new(),
        counts: SitemapCoverageCounts::default(),
        basis: None,
    };
    let Some(catalog) = catalog else {
        reason(&mut out, R::CatalogUnavailable);
        return Ok(out);
    };
    out.scope = if catalog.inherited.is_some() {
        SitemapCoverageScope::InheritedHistoricalDeclaredSources
    } else {
        SitemapCoverageScope::CurrentDeclaredSources
    };
    out.basis = Some(SitemapCoverageBasis {
        initial_config_sha256: catalog.summary.config_sha256.clone(),
        catalog_revision: catalog.summary.revision,
        catalog_rolling_sha256: catalog.summary.rolling_sha256.clone(),
        captured_inventory_sha256: None,
        inventory_saved_config_sha256: None,
        inventory_generation: None,
        parent_snapshot_sha256: catalog
            .inherited
            .as_ref()
            .map(|b| b.parent_snapshot_sha256.clone()),
        parent_catalog_sha256: catalog
            .inherited
            .as_ref()
            .map(|b| b.parent_catalog_sha256.clone()),
    });
    // A deliberately unavailable report projection is unknown, not an initialized empty ledger.
    if !catalog.summary.rows_available || catalog.rows.is_none() {
        reason(&mut out, R::CatalogRowsUnavailable);
        return Ok(out);
    }
    crate::sitemap_ledger::validate_catalog(catalog)?;
    if catalog.summary.legacy_history_unknown {
        reason(&mut out, R::LegacyHistoryUnknown);
    }
    let rows = catalog
        .rows
        .as_ref()
        .expect("validated complete row envelope");
    let mut documents = BTreeMap::new();
    let mut roots = Vec::new();
    let mut children: BTreeMap<&str, Vec<&crate::StoredSitemapLocation>> = BTreeMap::new();
    for row in rows {
        match row {
            SitemapLedgerRow::Document(document) => {
                documents.insert(
                    (document.kind.frontier_kind(), document.task_url.as_str()),
                    document.as_ref(),
                );
                out.counts.observed_documents += 1;
                out.counts.complete_documents += u64::from(document.complete());
                if !document.complete() {
                    incomplete_document_reasons(&mut out, document);
                }
            }
            SitemapLedgerRow::Root(root) => {
                roots.push(root);
                out.counts.root_declarations += 1;
            }
            SitemapLedgerRow::Location(location)
                if location.observed.role == crate::SitemapLocationRole::ChildSitemap =>
            {
                out.counts.child_declarations += 1;
                children
                    .entry(location.observation_id.as_str())
                    .or_default()
                    .push(location);
            }
            SitemapLedgerRow::Location(location)
                if location.observed.role == crate::SitemapLocationRole::PageLocation
                    && (location.observed.validation_error.is_some()
                        || location.observed.normalized_fetch_url.is_none()
                        || location.observed.loc_ordinal.is_none()) =>
            {
                out.counts.invalid_or_missing_page_locations += 1;
            }
            _ => {}
        }
    }
    let Some(inventory) = inventory else {
        reason(
            &mut out,
            if catalog.inherited.is_some() {
                R::HistoricalInventoryUnavailable
            } else {
                R::SourceInventoryUnavailable
            },
        );
        return Ok(out);
    };
    inventory.validate()?;
    let compatible_basis = match (&catalog.inherited, &inventory.basis) {
        (None, InventoryBasis::Current) => true,
        (
            Some(parent),
            InventoryBasis::InheritedParent {
                parent_snapshot_sha256,
                parent_catalog_sha256,
            },
        ) => {
            parent.parent_snapshot_sha256 == *parent_snapshot_sha256
                && parent.parent_catalog_sha256 == *parent_catalog_sha256
                && !parent.current_discovery_executed
        }
        _ => false,
    };
    if !compatible_basis {
        reason(
            &mut out,
            if catalog.inherited.is_some() {
                R::HistoricalInventoryUnavailable
            } else {
                R::SourceInventoryUnavailable
            },
        );
        return Ok(out);
    }
    ensure!(
        inventory.catalog_initial_config_sha256 == catalog.summary.config_sha256,
        "Coverage inventory initial config binding differs"
    );
    let basis = out.basis.as_mut().expect("available catalog basis");
    basis.captured_inventory_sha256 = Some(crate::sitemap_ledger::digest(serde_json::to_vec(
        inventory,
    )?));
    basis.inventory_saved_config_sha256 = Some(inventory.saved_config_sha256.clone());
    basis.inventory_generation = Some(inventory.generation);
    if !inventory.discovery_initialized {
        reason(&mut out, R::DiscoveryNotInitialized);
    }
    let observed_initial = roots
        .iter()
        .copied()
        .filter(|root| root.kind != SitemapRootKind::RobotsDirective)
        .map(|root| (root.declaration_id.as_str(), root))
        .collect::<BTreeMap<_, _>>();
    if let Some(expected) = &inventory.expected_initial_roots {
        let expected_ids = expected
            .iter()
            .map(|root| root.declaration_id.as_str())
            .collect::<BTreeSet<_>>();
        for root in expected {
            match observed_initial.get(root.declaration_id.as_str()) {
                None => reason(&mut out, R::MissingInitialRootDeclaration),
                Some(actual)
                    if actual.kind != root.kind
                        || actual.source_ordinal != root.source_ordinal
                        || actual.raw_value != root.raw_value
                        || actual.normalized_document_url != root.normalized_document_url =>
                {
                    reason(&mut out, R::InitialRootCoordinatesDiffer);
                }
                _ => {}
            }
        }
        for id in observed_initial.keys() {
            if !expected_ids.contains(id) {
                reason(&mut out, R::UnexpectedInitialRootDeclaration);
            }
        }
    } else {
        reason(&mut out, R::InitialRootInventoryUnknown);
    }
    let tasks = inventory
        .tasks
        .iter()
        .map(|task| (task_key(task), task))
        .collect::<BTreeMap<_, _>>();
    ensure!(
        tasks.len() == inventory.tasks.len(),
        "Duplicate coverage task identity"
    );
    out.counts.discovery_tasks = inventory.tasks.len() as u64;
    if let Some(expected) = &inventory.expected_robots_sources {
        for url in expected {
            if !tasks.contains_key(&("robots", url.as_str())) {
                reason(&mut out, R::RequiredRobotsTaskMissing);
            }
        }
    } else {
        reason(&mut out, R::RequiredRobotsInventoryUnknown);
    }
    for (key, task) in &tasks {
        match documents.get(key) {
            None => reason(
                &mut out,
                if task.done {
                    R::DoneSourceWithoutObservation
                } else {
                    R::PendingSource
                },
            ),
            Some(document) => {
                if !task.done {
                    reason(&mut out, R::PendingSource);
                }
                if document.generation > inventory.generation {
                    reason(&mut out, R::SourceGenerationAfterInventory);
                }
            }
        }
    }
    for key in documents.keys() {
        if !tasks.contains_key(key) {
            reason(&mut out, R::ObservationWithoutTask);
        }
    }
    let mut queue = VecDeque::new();
    for root in roots {
        if let Some(url) = admitted_source_url(&mut out, &root.admission, true) {
            queue.push_back(url);
        }
    }
    // Graph traversal uses admitted task identities, never redirect-final aliases.
    // Every original root/child occurrence is inspected; visited tasks only bound recursion.
    let mut reached = BTreeSet::new();
    while let Some(url) = queue.pop_front() {
        if !reached.insert(url) {
            continue;
        }
        let key = ("sitemap", url);
        if !tasks.contains_key(&key) {
            reason(&mut out, R::SourceTaskMissing);
        }
        if let Some(document) = documents.get(&key)
            && let Some(edges) = children.get(document.observation_id.as_str())
        {
            for edge in edges {
                if let Some(child) = admitted_source_url(&mut out, &edge.admission, false) {
                    ensure!(
                        edge.referenced_sitemap_task.as_deref() == Some(child),
                        "Coverage child task edge differs"
                    );
                    queue.push_back(child);
                }
            }
        }
    }
    out.counts.reached_xml_tasks = reached.len() as u64;
    for ((kind, url), _) in tasks {
        if kind == "sitemap" && !reached.contains(url) {
            reason(&mut out, R::SourceGraphUnboundTask);
        }
    }
    if out.reason_counts.is_empty() {
        out.completion = if out.counts.root_declarations == 0 {
            SitemapCohortCompletion::KnownEmpty
        } else {
            SitemapCohortCompletion::Complete
        };
        match out.scope {
            SitemapCoverageScope::CurrentDeclaredSources => {
                out.current_valid_original_location_negative_permitted = true
            }
            SitemapCoverageScope::InheritedHistoricalDeclaredSources => {
                out.historical_valid_original_location_negative_permitted = true
            }
            SitemapCoverageScope::Unavailable => unreachable!("validated source basis"),
        }
    }
    Ok(out)
}

fn admitted_source_url<'a>(
    out: &mut SitemapCoverage,
    admission: &'a SitemapAdmissionBinding,
    root: bool,
) -> Option<&'a str> {
    use SitemapCoverageReason as R;
    match admission.decision {
        Admission::Scheduled | Admission::Coalesced => admission.admitted_fetch_url.as_deref(),
        Admission::InvalidLocation | Admission::MissingLocation => {
            reason(
                out,
                if root {
                    R::RootInvalid
                } else {
                    R::ChildInvalid
                },
            );
            None
        }
        Admission::Unbound | Admission::IncompleteSource => {
            reason(
                out,
                if root {
                    R::RootUnbound
                } else {
                    R::ChildUnbound
                },
            );
            None
        }
        _ => {
            reason(out, if root { R::RootDenied } else { R::ChildDenied });
            None
        }
    }
}

fn incomplete_document_reasons(out: &mut SitemapCoverage, document: &SitemapSourceDocument) {
    use SitemapCoverageReason as R;
    reason(out, R::SourceNotComplete);
    if document.failure_code.is_some() {
        reason(out, R::SourceFailed);
    }
    if !document.retained_rows_complete {
        reason(out, R::SourceRowsIncomplete);
    }
    match &document.transport {
        None => reason(out, R::SourceTransportUnavailable),
        Some(transport) => {
            if !transport.request_started {
                reason(out, R::SourceRequestNotStarted);
            }
            if transport.final_status != Some(200) {
                reason(out, R::SourceHttpNot200);
            }
            if !transport.body_complete {
                reason(out, R::SourceBodyIncomplete);
            }
            if transport.blocked_by_robots {
                reason(out, R::SourceBlockedByRobots);
            }
            if transport.failure_code.is_some() {
                reason(out, R::SourceTransportFailed);
            }
        }
    }
    match document.kind {
        SitemapSourceKind::RobotsDirectives => {
            if !document.declaration_scan_complete || document.declaration_utf8_lossy != Some(false)
            {
                reason(out, R::SourceDeclarationsIncomplete);
            }
        }
        SitemapSourceKind::SitemapXml => {
            if document.entity_decode.is_none() {
                reason(out, R::SourceDecodeUnavailable);
            }
            if !document
                .parser
                .as_ref()
                .is_some_and(|parser| parser.complete && parser.failure.is_none())
            {
                reason(out, R::SourceParserIncomplete);
            }
        }
    }
}

#[cfg(test)]
#[path = "sitemap_coverage_tests.rs"]
mod tests;
