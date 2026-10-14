//! Same-transaction coverage projection without a raw/source catalog or task Vec.
//! The caller owns the read transaction and the higher selected-parent guards.
//! Selected history deliberately retains the old materializing compatibility path.
use super::*;
use crate::sitemap_ledger::verified_read::{SitemapLedgerFilter, VerifiedRawRead};
use crate::sitemap_read_control::SitemapReadControl;
use rusqlite::{Connection, OptionalExtension, params};
use serde::ser::{Error as _, SerializeSeq, SerializeStruct};
use sha2::{Digest, Sha256};
use std::io::{self, Write};

#[derive(Clone, Copy)]
pub(crate) struct CoverageReadContext<'a> {
    pub(crate) saved_config_bytes: Option<&'a [u8]>,
    pub(crate) saved_config_sha256: Option<&'a str>,
    pub(crate) generation: Option<u64>,
    pub(crate) discovery_initialized: bool,
}

pub(crate) struct VerifiedCoverageRead {
    pub(crate) coverage: SitemapCoverage,
    /// SHA of the exact existing serde inventory BODY. None is not hash(null).
    pub(crate) inventory_sha256: Option<String>,
    pub(crate) materializing_selected_compatibility: bool,
}

fn scalar_text(connection: &Connection, key: &str) -> Result<Option<String>> {
    let length: Option<i64> = connection
        .query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key=?1",
            [key],
            |row| row.get(0),
        )
        .optional()?;
    ensure!(
        length.is_none_or(|n| (0..=65_536).contains(&n)),
        "Coverage context metadata bound"
    );
    Ok(connection
        .query_row("SELECT value FROM metadata WHERE key=?1", [key], |row| {
            row.get(0)
        })
        .optional()?)
}

fn validate_context(
    connection: &Connection,
    context: CoverageReadContext<'_>,
    control: &SitemapReadControl,
) -> Result<()> {
    control.checkpoint()?;
    let length: Option<i64> = connection
        .query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key='config'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    match (
        length,
        context.saved_config_bytes,
        context.saved_config_sha256,
    ) {
        (None, None, None) => {}
        (Some(length), Some(bytes), Some(sha)) => {
            ensure!(
                (1..=MAX_SAVED_CONFIG_BYTES as i64).contains(&length)
                    && i64::try_from(bytes.len())? == length
                    && control.sha256(bytes)? == sha,
                "Coverage actual saved-config byte/hash context differs"
            );
            let equal: bool = connection.query_row(
                "SELECT CAST(value AS BLOB) FROM metadata WHERE key='config'",
                [],
                |row| Ok(row.get_ref(0)?.as_blob()? == bytes),
            )?;
            ensure!(
                equal,
                "Coverage supplied bytes differ from actual SQL config"
            );
        }
        _ => anyhow::bail!("Coverage saved-config presence context differs"),
    }
    let run: Option<crate::CrawlRunState> = scalar_text(connection, "run_state")?
        .map(|raw| serde_json::from_str(&raw))
        .transpose()?;
    ensure!(
        run.map(|r| r.generation).filter(|n| *n > 0) == context.generation,
        "Coverage actual generation context differs"
    );
    ensure!(
        (scalar_text(connection, "discovery_initialized")?.as_deref() == Some("true"))
            == context.discovery_initialized,
        "Coverage actual discovery marker differs"
    );
    Ok(())
}

fn task_table(connection: &Connection) -> Result<bool> {
    Ok(connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='discovery_frontier')", [], |r| r.get(0))?)
}

fn visit_tasks(
    connection: &Connection,
    exists: bool,
    control: &SitemapReadControl,
    mut visit: impl FnMut(&SitemapCoverageTask) -> Result<()>,
) -> Result<()> {
    if !exists {
        return Ok(());
    }
    let (rows, bytes): (i64, i64) = connection.query_row(
        "SELECT count(*),coalesce(sum(length(CAST(url AS BLOB))),0) FROM discovery_frontier",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    ensure!(
        (0..=MAX_INVENTORY_TASKS as i64).contains(&rows)
            && (0..=MAX_INVENTORY_TASK_BYTES as i64).contains(&bytes),
        "Coverage task inventory budget exceeded"
    );
    let mut query=connection.prepare("SELECT kind,url,done FROM discovery_frontier ORDER BY kind COLLATE BINARY,url COLLATE BINARY")?;
    let mut cursor = query.query([])?;
    let mut previous: Option<(SitemapSourceKind, String)> = None;
    while let Some(row) = cursor.next()? {
        control.checkpoint()?;
        let kind = match row.get_ref(0)?.as_str()? {
            "robots" => SitemapSourceKind::RobotsDirectives,
            "sitemap" => SitemapSourceKind::SitemapXml,
            _ => anyhow::bail!("Unknown coverage discovery task kind"),
        };
        let url = row.get_ref(1)?.as_str()?;
        let done: i64 = row.get(2)?;
        ensure!(
            matches!(done, 0 | 1),
            "Invalid coverage discovery done flag"
        );
        ensure!(
            crate::normalize_url(url, None).is_some_and(|v| v.as_str() == url),
            "Unsafe coverage task identity"
        );
        if let Some((old_kind, old_url)) = &previous {
            ensure!(
                (old_kind.frontier_kind(), old_url.as_str()) < (kind.frontier_kind(), url),
                "Duplicate/unsorted coverage task identity"
            );
        }
        let task = SitemapCoverageTask {
            kind,
            task_url: url.into(),
            done: done == 1,
        };
        visit(&task)?;
        previous = Some((kind, task.task_url));
    }
    Ok(())
}

fn task_exists(connection: &Connection, exists: bool, kind: &str, url: &str) -> Result<bool> {
    if !exists {
        return Ok(false);
    }
    Ok(connection.query_row("SELECT EXISTS(SELECT 1 FROM discovery_frontier WHERE kind COLLATE BINARY=?1 AND url COLLATE BINARY=?2)", params![kind,url], |r|r.get(0))?)
}

fn document_identity(
    connection: &Connection,
    kind: &str,
    url: &str,
) -> Result<Option<(String, u64)>> {
    let value:Option<(String,i64)>=connection.query_row(
        "SELECT row_id,generation FROM sitemap_documents WHERE source_kind COLLATE BINARY=?1 AND task_url COLLATE BINARY=?2", params![kind,url],
        |r| Ok((r.get(0)?,r.get(1)?)),
    ).optional()?;
    value
        .map(|(id, generation)| Ok((id, u64::try_from(generation)?)))
        .transpose()
}

#[cfg(test)]
fn expected_robots(connection: &Connection, config: &CrawlConfig) -> Result<Vec<String>> {
    expected_robots_with_control(connection, config, &SitemapReadControl::default())
}
fn expected_robots_with_control(
    connection: &Connection,
    config: &CrawlConfig,
    control: &SitemapReadControl,
) -> Result<Vec<String>> {
    control.checkpoint()?;
    if !(config.crawl_links && config.discover_sitemaps && config.obey_robots) {
        return Ok(vec![]);
    }
    let mut origins = BTreeSet::new();
    let mut origin_bytes = 0usize;
    if config.crawl_policy.is_none() {
        for value in if config.seed_urls.is_empty() {
            &config.sitemap_urls
        } else {
            &config.seed_urls
        } {
            control.checkpoint()?;
            insert_origin(&mut origins, &mut origin_bytes, value)?;
        }
        return Ok(origins.into_iter().collect());
    }
    let mut query=connection.prepare("SELECT json_extract(data,'$.occurrence.decision.fetch_candidate_url') FROM policy_occurrences WHERE json_extract(data,'$.occurrence.input_basis')='declared_input' AND json_extract(data,'$.occurrence.task_role')='page_document' AND json_extract(data,'$.occurrence.decision.source_kind') IN ('seed','list_page') AND json_extract(data,'$.occurrence.admission_reason') IN ('scheduled','coalesced') ORDER BY sequence")?;
    let mut cursor = query.query([])?;
    while let Some(row) = cursor.next()? {
        control.checkpoint()?;
        let value = row.get_ref(0)?.as_str()?;
        insert_origin(&mut origins, &mut origin_bytes, value)?;
    }
    Ok(origins.into_iter().collect())
}

fn insert_origin(origins: &mut BTreeSet<String>, bytes: &mut usize, value: &str) -> Result<()> {
    let url = crate::normalize_url(value, None).context("Invalid required Robots source input")?;
    let origin = format!("{}/", url.origin().ascii_serialization());
    if !origins.contains(&origin) {
        *bytes = bytes
            .checked_add(origin.len())
            .context("Coverage Robots text overflow")?;
        ensure!(
            *bytes <= MAX_SAVED_CONFIG_BYTES && origins.len() < MAX_INVENTORY_TASKS,
            "Coverage Robots inventory budget exceeded"
        );
        origins.insert(origin);
    }
    Ok(())
}

struct RootsJson<'a> {
    config: &'a CrawlConfig,
    initial_sha: &'a str,
    control: &'a SitemapReadControl,
}
impl Serialize for RootsJson<'_> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(None)?;
        crate::sitemap_initial_roots::visit_initial_roots(
            self.config.explicit_sitemap_inputs.as_deref(),
            &self.config.input_imports,
            self.initial_sha,
            None,
            |item| {
                self.control.checkpoint()?;
                seq.serialize_element(&item.root)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))
            },
        )
        .map_err(S::Error::custom)?;
        seq.end()
    }
}
struct TasksJson<'a> {
    connection: &'a Connection,
    exists: bool,
    control: &'a SitemapReadControl,
}
impl Serialize for TasksJson<'_> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(None)?;
        self.control
            .map_result(visit_tasks(
                self.connection,
                self.exists,
                self.control,
                |task| {
                    seq.serialize_element(task)
                        .map_err(|e| anyhow::anyhow!(e.to_string()))
                },
            ))
            .map_err(S::Error::custom)?;
        seq.end()
    }
}
struct InventoryJson<'a> {
    connection: &'a Connection,
    config: &'a CrawlConfig,
    context: CoverageReadContext<'a>,
    initial_sha: &'a str,
    robots: &'a [String],
    tasks_exist: bool,
    control: &'a SitemapReadControl,
}
impl Serialize for InventoryJson<'_> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("SitemapCoverageInventory", 8)?;
        state.serialize_field("basis", &InventoryBasis::Current)?;
        state.serialize_field("saved_config_sha256", &self.context.saved_config_sha256)?;
        state.serialize_field("catalog_initial_config_sha256", self.initial_sha)?;
        state.serialize_field("generation", &self.context.generation)?;
        state.serialize_field("discovery_initialized", &self.context.discovery_initialized)?;
        if self.config.explicit_sitemap_inputs.is_some() {
            state.serialize_field(
                "expected_initial_roots",
                &RootsJson {
                    config: self.config,
                    initial_sha: self.initial_sha,
                    control: self.control,
                },
            )?;
        } else {
            state.serialize_field("expected_initial_roots", &Option::<()>::None)?;
        }
        state.serialize_field("expected_robots_sources", self.robots)?;
        state.serialize_field(
            "tasks",
            &TasksJson {
                connection: self.connection,
                exists: self.tasks_exist,
                control: self.control,
            },
        )?;
        state.end()
    }
}
struct InventoryHash<'a> {
    control: &'a SitemapReadControl,
    next_check: usize,
    bytes: usize,
    hash: Sha256,
}
impl Write for InventoryHash<'_> {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(value.len())
            .ok_or_else(|| io::Error::other("Coverage inventory byte overflow"))?;
        if self.bytes > MAX_SAVED_CONFIG_BYTES {
            return Err(io::Error::other(
                "Coverage inventory serialized budget exceeded",
            ));
        }
        let start = self.bytes - value.len();
        for (index, chunk) in value.chunks(65_536).enumerate() {
            let offset = start + index * 65_536;
            if offset + chunk.len() >= self.next_check {
                self.control.checkpoint().map_err(io::Error::other)?;
                self.next_check = offset + chunk.len() + 65_536;
            }
            self.hash.update(chunk);
        }
        Ok(value.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn count_rows(
    raw: &VerifiedRawRead<'_>,
    out: &mut SitemapCoverage,
    control: &SitemapReadControl,
) -> Result<()> {
    raw.visit_filtered(&SitemapLedgerFilter::default(), |row| {
        control.checkpoint()?;
        match row {
            SitemapLedgerRow::Document(document) => {
                out.counts.observed_documents += 1;
                out.counts.complete_documents += u64::from(document.complete());
                if !document.complete() {
                    incomplete_document_reasons(out, document);
                }
            }
            SitemapLedgerRow::Root(_) => out.counts.root_declarations += 1,
            SitemapLedgerRow::Location(location)
                if location.observed.role == crate::SitemapLocationRole::ChildSitemap =>
            {
                out.counts.child_declarations += 1
            }
            SitemapLedgerRow::Location(location)
                if location.observed.role == crate::SitemapLocationRole::PageLocation
                    && (location.observed.validation_error.is_some()
                        || location.observed.normalized_fetch_url.is_none()
                        || location.observed.loc_ordinal.is_none()) =>
            {
                out.counts.invalid_or_missing_page_locations += 1
            }
            _ => {}
        }
        Ok(())
    })
}

fn compare_initial_roots(
    connection: &Connection,
    raw: &VerifiedRawRead<'_>,
    config: &CrawlConfig,
    out: &mut SitemapCoverage,
    control: &SitemapReadControl,
) -> Result<()> {
    let mut explicit = 0u64;
    let mut imported = 0u64;
    let known = crate::sitemap_initial_roots::visit_initial_roots(
        config.explicit_sitemap_inputs.as_deref(),
        &config.input_imports,
        &raw.summary().config_sha256,
        None,
        |item| {
            control.checkpoint()?;
            let expected = item.root;
            match expected.kind {
                SitemapRootKind::ExplicitConfig => explicit += 1,
                SitemapRootKind::ImportedSitemapRow => imported += 1,
                SitemapRootKind::RobotsDirective => unreachable!(),
            }
            let data: Option<String> = connection
                .query_row(
                    "SELECT data FROM sitemap_roots WHERE row_id COLLATE BINARY=?1",
                    [&expected.declaration_id],
                    |r| r.get(0),
                )
                .optional()?;
            match data {
                None => reason(out, SitemapCoverageReason::MissingInitialRootDeclaration),
                Some(data) => {
                    let SitemapLedgerRow::Root(actual) = serde_json::from_str(&data)? else {
                        anyhow::bail!("Expected root projection");
                    };
                    if actual.kind != expected.kind
                        || actual.source_ordinal != expected.source_ordinal
                        || actual.raw_value != expected.raw_value
                        || actual.normalized_document_url != expected.normalized_document_url
                    {
                        reason(out, SitemapCoverageReason::InitialRootCoordinatesDiffer);
                    }
                }
            }
            Ok(())
        },
    )?;
    if !known {
        reason(out, SitemapCoverageReason::InitialRootInventoryUnknown);
        return Ok(());
    }
    raw.visit_filtered(
        &SitemapLedgerFilter {
            kind: crate::sitemap_ledger::verified_read::SitemapLedgerRowKind::Root,
            ..Default::default()
        },
        |row| {
            control.checkpoint()?;
            if let SitemapLedgerRow::Root(root) = row {
                let expected = match root.kind {
                    SitemapRootKind::ExplicitConfig => root.source_ordinal < explicit,
                    SitemapRootKind::ImportedSitemapRow => root.source_ordinal < imported,
                    SitemapRootKind::RobotsDirective => true,
                };
                if !expected {
                    reason(out, SitemapCoverageReason::UnexpectedInitialRootDeclaration);
                }
            }
            Ok(())
        },
    )
}

fn enqueue(
    reached: &mut BTreeSet<String>,
    queue: &mut VecDeque<String>,
    bytes: &mut usize,
    url: &str,
) -> Result<()> {
    if reached.contains(url) {
        return Ok(());
    }
    *bytes = bytes
        .checked_add(url.len())
        .context("Coverage graph text overflow")?;
    ensure!(
        reached.len() < MAX_INVENTORY_TASKS && *bytes <= MAX_INVENTORY_TASK_BYTES,
        "Coverage graph identity budget exceeded"
    );
    reached.insert(url.into());
    queue.push_back(url.into());
    Ok(())
}

fn project_current(
    connection: &Connection,
    raw: &VerifiedRawRead<'_>,
    context: CoverageReadContext<'_>,
    control: &SitemapReadControl,
) -> Result<VerifiedCoverageRead> {
    use SitemapCoverageReason as R;
    let mut out = empty_coverage();
    bind_coverage_basis(&mut out, raw.summary(), None);
    if raw.summary().legacy_history_unknown {
        reason(&mut out, R::LegacyHistoryUnknown);
    }
    count_rows(raw, &mut out, control)?;
    let (Some(bytes), Some(sha), Some(generation)) = (
        context.saved_config_bytes,
        context.saved_config_sha256,
        context.generation,
    ) else {
        reason(&mut out, R::SourceInventoryUnavailable);
        return Ok(VerifiedCoverageRead {
            coverage: out,
            inventory_sha256: None,
            materializing_selected_compatibility: false,
        });
    };
    let config: CrawlConfig =
        serde_json::from_slice(bytes).context("Invalid saved coverage configuration")?;
    config.validate_input_provenance_budget()?;
    // Same authoritative policy verifier as Store's old rich snapshot, even
    // when Robots discovery is disabled. It retains no ordinary task vector.
    if let Some(policy) = &config.crawl_policy {
        crate::policy_ledger::verify(connection, &crate::CompiledCrawlPolicy::new(policy)?)?;
    }
    control.checkpoint()?;
    let tasks_exist = task_table(connection)?;
    let robots = expected_robots_with_control(connection, &config, control)?;
    ensure!(
        robots.len() <= MAX_INVENTORY_TASKS,
        "Coverage Robots inventory row budget exceeded"
    );
    let mut hash = InventoryHash {
        control,
        next_check: 65_536,
        bytes: 0,
        hash: Sha256::new(),
    };
    serde_json::to_writer(
        &mut hash,
        &InventoryJson {
            connection,
            config: &config,
            context,
            initial_sha: &raw.summary().config_sha256,
            robots: &robots,
            tasks_exist,
            control,
        },
    )?;
    let inventory_sha256 = format!("{:x}", hash.hash.finalize());
    let basis = out.basis.as_mut().expect("catalog basis");
    basis.captured_inventory_sha256 = Some(inventory_sha256.clone());
    basis.inventory_saved_config_sha256 = Some(sha.into());
    basis.inventory_generation = Some(generation);
    if !context.discovery_initialized {
        reason(&mut out, R::DiscoveryNotInitialized);
    }
    control.checkpoint()?;
    compare_initial_roots(connection, raw, &config, &mut out, control)?;
    for origin in robots {
        control.checkpoint()?;
        if !task_exists(connection, tasks_exist, "robots", &origin)? {
            reason(&mut out, R::RequiredRobotsTaskMissing);
        }
    }
    visit_tasks(connection, tasks_exist, control, |task| {
        control.checkpoint()?;
        out.counts.discovery_tasks += 1;
        match document_identity(connection, task.kind.frontier_kind(), &task.task_url)? {
            None => reason(
                &mut out,
                if task.done {
                    R::DoneSourceWithoutObservation
                } else {
                    R::PendingSource
                },
            ),
            Some((_, observed_generation)) => {
                if !task.done {
                    reason(&mut out, R::PendingSource);
                }
                if observed_generation > generation {
                    reason(&mut out, R::SourceGenerationAfterInventory);
                }
            }
        }
        Ok(())
    })?;
    raw.visit_filtered(
        &SitemapLedgerFilter {
            kind: crate::sitemap_ledger::verified_read::SitemapLedgerRowKind::Document,
            ..Default::default()
        },
        |row| {
            control.checkpoint()?;
            if let SitemapLedgerRow::Document(doc) = row
                && !task_exists(
                    connection,
                    tasks_exist,
                    doc.kind.frontier_kind(),
                    &doc.task_url,
                )?
            {
                reason(&mut out, R::ObservationWithoutTask);
            }
            Ok(())
        },
    )?;
    let mut reached = BTreeSet::new();
    let mut queue = VecDeque::new();
    let mut graph_bytes = 0usize;
    raw.visit_filtered(
        &SitemapLedgerFilter {
            kind: crate::sitemap_ledger::verified_read::SitemapLedgerRowKind::Root,
            ..Default::default()
        },
        |row| {
            control.checkpoint()?;
            if let SitemapLedgerRow::Root(root) = row
                && let Some(url) = admitted_source_url(&mut out, &root.admission, true)
            {
                enqueue(&mut reached, &mut queue, &mut graph_bytes, url)?;
            }
            Ok(())
        },
    )?;
    while let Some(url) = queue.pop_front() {
        control.checkpoint()?;
        if !task_exists(connection, tasks_exist, "sitemap", &url)? {
            reason(&mut out, R::SourceTaskMissing);
        }
        if let Some((id, _)) = document_identity(connection, "sitemap", &url)? {
            // Direct indexed source adjacency query, never an all-locations scan
            // for each document and never retained raw edge rows.
            let mut query=connection.prepare("SELECT data FROM sitemap_locations WHERE observation_id COLLATE BINARY=?1 AND json_extract(data,'$.row.observed.role')='child_sitemap' ORDER BY sequence")?;
            let mut cursor = query.query([id])?;
            while let Some(row) = cursor.next()? {
                control.checkpoint()?;
                let SitemapLedgerRow::Location(edge) =
                    serde_json::from_str(row.get_ref(0)?.as_str()?)?
                else {
                    anyhow::bail!("Expected child Location row");
                };
                if let Some(child) = admitted_source_url(&mut out, &edge.admission, false) {
                    ensure!(
                        edge.referenced_sitemap_task.as_deref() == Some(child),
                        "Coverage child task edge differs"
                    );
                    enqueue(&mut reached, &mut queue, &mut graph_bytes, child)?;
                }
            }
        }
    }
    out.counts.reached_xml_tasks = reached.len() as u64;
    visit_tasks(connection, tasks_exist, control, |task| {
        control.checkpoint()?;
        if task.kind == SitemapSourceKind::SitemapXml && !reached.contains(&task.task_url) {
            reason(&mut out, R::SourceGraphUnboundTask);
        }
        Ok(())
    })?;
    finish_coverage(&mut out);
    Ok(VerifiedCoverageRead {
        coverage: out,
        inventory_sha256: Some(inventory_sha256),
        materializing_selected_compatibility: false,
    })
}

fn project_selected(
    connection: &Connection,
    control: &SitemapReadControl,
) -> Result<VerifiedCoverageRead> {
    control.checkpoint()?;
    // Higher Store selected-plan/source-parent presence gates MUST already run.
    // This is explicit bounded, materializing legacy compatibility, not ordinary
    // streaming and not a complete S4 claim.
    let catalog = crate::sitemap_ledger::load_catalog(connection)?
        .context("Inherited coverage catalog unavailable")?;
    let length: i64 = connection.query_row(
        "SELECT length(CAST(snapshot AS BLOB)) FROM recrawl_base WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    ensure!(
        (1..=268_435_456).contains(&length),
        "Inherited coverage snapshot bound exceeded"
    );
    let bytes: Vec<u8> = connection.query_row(
        "SELECT CAST(snapshot AS BLOB) FROM recrawl_base WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    ensure!(
        i64::try_from(bytes.len())? == length,
        "Inherited coverage snapshot byte length differs"
    );
    let parent: crate::CrawlReport = serde_json::from_slice(&bytes)?;
    validate_sitemap_coverage_inventory(
        &parent.config,
        parent.sitemap_catalog.as_ref(),
        parent.sitemap_coverage_inventory.as_ref(),
    )?;
    let inventory = parent
        .sitemap_coverage_inventory
        .map(|inventory| {
            inventory.bind_inherited_parent(
                parent
                    .sitemap_catalog
                    .as_ref()
                    .context("Inherited parent catalog missing")?,
                &bytes,
            )
        })
        .transpose()?;
    control.checkpoint()?;
    let coverage = project_sitemap_coverage(Some(&catalog), inventory.as_ref())?;
    control.checkpoint()?;
    let inventory_sha256 = coverage
        .basis
        .as_ref()
        .and_then(|b| b.captured_inventory_sha256.clone());
    Ok(VerifiedCoverageRead {
        coverage,
        inventory_sha256,
        materializing_selected_compatibility: true,
    })
}

#[cfg(test)]
pub(crate) fn project_read(
    connection: &Connection,
    raw: Option<&VerifiedRawRead<'_>>,
    context: CoverageReadContext<'_>,
) -> Result<VerifiedCoverageRead> {
    project_read_with_control(connection, raw, context, &SitemapReadControl::default())
}
pub(crate) fn project_read_with_control(
    connection: &Connection,
    raw: Option<&VerifiedRawRead<'_>>,
    context: CoverageReadContext<'_>,
    control: &SitemapReadControl,
) -> Result<VerifiedCoverageRead> {
    let _binding = control.bind_connection(connection)?;
    control.run(|| project_read_inner(connection, raw, context, control))
}
fn project_read_inner(
    connection: &Connection,
    raw: Option<&VerifiedRawRead<'_>>,
    context: CoverageReadContext<'_>,
    control: &SitemapReadControl,
) -> Result<VerifiedCoverageRead> {
    ensure!(
        !connection.is_autocommit(),
        "Coverage read requires caller-owned transaction"
    );
    validate_context(connection, context, control)?;
    let Some(raw) = raw else {
        return Ok(VerifiedCoverageRead {
            coverage: project_sitemap_coverage(None, None)?,
            inventory_sha256: None,
            materializing_selected_compatibility: false,
        });
    };
    ensure!(
        std::ptr::eq(connection, raw.connection()),
        "Coverage raw proof belongs to another connection"
    );
    if raw.inherited().is_some() {
        project_selected(connection, control)
    } else {
        project_current(connection, raw, context, control)
    }
}

#[cfg(test)]
#[path = "verified_read_tests.rs"]
mod tests;
