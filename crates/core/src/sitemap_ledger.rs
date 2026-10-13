//! WORK-only durable sitemap candidate. Caller owns the outer discovery transaction.
//! Propagate ALL errors with `?`: policy/frontier/warnings and this append must roll back together.
//! No HTTP, task admission, independent transaction commit, or source-cohort completeness inference.
use anyhow::{Context, Result, bail, ensure};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
};

#[path = "sitemap_ledger_models.rs"]
mod models;
pub use models::*;

pub(crate) const SCHEMA: &str = include_str!("schema.sql");
const TABLES: [&str; 4] = [
    "sitemap_roots",
    "sitemap_documents",
    "sitemap_entries",
    "sitemap_locations",
];
const SUMMARY_KEY: &str = "sitemap_ledger";
const INHERITED_KEY: &str = "sitemap_inherited_basis";

pub(crate) fn digest(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
fn sha(value: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v)),
        "Expected lowercase SHA-256"
    );
    Ok(())
}
pub(crate) fn identity(kind: &str, source: &str, ordinal: u64) -> String {
    // Length-framed canonical tuple, not delimiter concatenation of uncontrolled input.
    digest(
        serde_json::to_vec(&(SITEMAP_LEDGER_ENGINE, kind, source, ordinal))
            .expect("String tuple serialization"),
    )
}
pub(crate) fn source_observation_id(
    kind: SitemapSourceKind,
    task_url: &str,
    generation: u64,
    source_revision: &str,
) -> Result<String> {
    ensure!(
        !task_url.is_empty() && !source_revision.is_empty(),
        "Empty source identity"
    );
    Ok(digest(serde_json::to_vec(&(
        SITEMAP_LEDGER_ENGINE,
        kind,
        task_url,
        generation,
        source_revision,
    ))?))
}
pub(crate) fn root_declaration_id(
    kind: &SitemapRootKind,
    source_basis: &str,
    ordinal: u64,
) -> String {
    let kind = match kind {
        SitemapRootKind::ExplicitConfig => "root-explicit",
        SitemapRootKind::ImportedSitemapRow => "root-imported",
        SitemapRootKind::RobotsDirective => "root-robots",
    };
    identity(kind, source_basis, ordinal)
}
fn seed_hash() -> String {
    digest(b"open-seo-spider/sitemap-ledger-empty-v1")
}
fn roll(previous: &str, kind: &str, data: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"open-seo-spider/sitemap-ledger-row-v1\0");
    // Previous digest is exactly validated 64 lowercase ASCII bytes.
    hash.update(previous.as_bytes());
    hash.update((kind.len() as u64).to_be_bytes());
    hash.update(kind.as_bytes());
    hash.update((data.len() as u64).to_be_bytes());
    hash.update(data.as_bytes());
    format!("{:x}", hash.finalize())
}
struct CountingWriter {
    count: usize,
    cap: usize,
}
impl Write for CountingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self
            .count
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("JSON byte count overflow"))?;
        if count > self.cap {
            return Err(io::Error::other("sitemap_catalog_json_budget_exceeded"));
        }
        self.count = count;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn measured<T: Serialize>(value: &T, cap: usize) -> Result<u64> {
    let mut writer = CountingWriter { count: 0, cap };
    serde_json::to_writer(&mut writer, value)
        .context("Serialized sitemap catalog/row budget exceeded")?;
    Ok(u64::try_from(writer.count)?)
}
fn row_data(row: &SitemapLedgerRow, limits: &SitemapLedgerLimits) -> Result<String> {
    measured(row, limits.max_row_json_bytes)?;
    Ok(serde_json::to_string(row)?)
}
fn refresh_size(catalog: &mut SitemapCatalog) -> Result<()> {
    // The byte-count field itself is part of the bounded serialized envelope.
    // Decimal digit changes converge; do not subtract a guessed fixed envelope size.
    for _ in 0..8 {
        let size = measured(catalog, catalog.summary.limits.max_catalog_json_bytes)?;
        if catalog.summary.catalog_json_bytes == size {
            return Ok(());
        }
        catalog.summary.catalog_json_bytes = size;
    }
    bail!("Sitemap catalog JSON size did not converge")
}
fn metadata_text(connection: &Connection, key: &str) -> Result<Option<String>> {
    let bytes: Option<i64> = connection
        .query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key=?1",
            [key],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(bytes) = bytes {
        ensure!((0..=65536).contains(&bytes), "Oversized sitemap metadata");
    }
    Ok(connection
        .query_row("SELECT value FROM metadata WHERE key=?1", [key], |row| {
            row.get(0)
        })
        .optional()?)
}
fn metadata<T: DeserializeOwned>(connection: &Connection, key: &str) -> Result<Option<T>> {
    metadata_text(connection, key)?
        .map(|text| serde_json::from_str(&text).context("Invalid sitemap metadata JSON"))
        .transpose()
}
fn save<T: Serialize>(connection: &Connection, key: &str, value: &T) -> Result<()> {
    connection.execute("INSERT INTO metadata(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key,serde_json::to_string(value)?])?;
    Ok(())
}
fn present_tables(connection: &Connection) -> Result<usize> {
    let mut count = 0;
    for table in TABLES {
        count += usize::from(connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [table],
            |row| row.get::<_, bool>(0),
        )?);
    }
    Ok(count)
}
fn require_tables(connection: &Connection) -> Result<()> {
    ensure!(
        present_tables(connection)? == TABLES.len(),
        "Sitemap marker exists but ledger tables are missing"
    );
    Ok(())
}
fn require_empty(connection: &Connection) -> Result<()> {
    require_tables(connection)?;
    for table in TABLES {
        let count: i64 =
            connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })?;
        ensure!(
            count == 0,
            "Sitemap ledger initialization/inheritance requires empty tables"
        );
    }
    Ok(())
}
fn active_generation(connection: &Connection, generation: u64) -> Result<()> {
    // Identical criterion to store::require_generation; selected reports do not call this append path.
    let run: crate::CrawlRunState =
        metadata(connection, "run_state")?.context("Missing active crawl run state")?;
    ensure!(
        run.generation == generation && run.outcome.is_none(),
        "Stale/terminal sitemap generation"
    );
    Ok(())
}
fn savepoint<T>(transaction: &Transaction<'_>, operation: impl FnOnce() -> Result<T>) -> Result<T> {
    transaction.execute_batch("SAVEPOINT sitemap_catalog_append")?;
    match operation() {
        Ok(value) => {
            transaction.execute_batch("RELEASE SAVEPOINT sitemap_catalog_append")?;
            Ok(value)
        }
        Err(error) => {
            // Outer caller still must propagate the error, so preceding policy/frontier writes roll back.
            transaction.execute_batch("ROLLBACK TO SAVEPOINT sitemap_catalog_append; RELEASE SAVEPOINT sitemap_catalog_append")
                .context("Failed rollback of sitemap append savepoint")?;
            Err(error)
        }
    }
}
pub(crate) fn initialize(
    transaction: &Transaction<'_>,
    config_sha256: &str,
    limits: &SitemapLedgerLimits,
    legacy_history_unknown: bool,
) -> Result<SitemapLedgerSummary> {
    limits.validate()?;
    sha(config_sha256)?;
    if let Some(catalog) = load_catalog(transaction)? {
        ensure!(
            catalog.summary.config_sha256 == config_sha256 && catalog.summary.limits == *limits,
            "Sitemap config/limits changed after initialization"
        );
        ensure!(
            catalog.inherited.is_none(),
            "Inherited sitemap history must not become fresh discovery"
        );
        return Ok(catalog.summary);
    }
    require_empty(transaction)?;
    let old_initialized =
        metadata_text(transaction, "discovery_initialized")?.as_deref() == Some("true");
    let old_complete = metadata_text(transaction, "discovery_complete")?.as_deref() == Some("true");
    let old_done: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM discovery_frontier WHERE done=1)",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        legacy_history_unknown || !(old_initialized || old_complete || old_done),
        "Legacy discovery exists; caller must explicitly mark history unknown"
    );
    let summary = SitemapLedgerSummary {
        engine: SITEMAP_LEDGER_ENGINE.into(),
        hash_profile: SITEMAP_LEDGER_HASH_PROFILE.into(),
        config_sha256: config_sha256.into(),
        limits: limits.clone(),
        revision: 0,
        last_sequence: 0,
        root_rows: 0,
        document_rows: 0,
        entry_rows: 0,
        location_rows: 0,
        incomplete_documents: 0,
        catalog_json_bytes: 0,
        rolling_sha256: seed_hash(),
        rows_available: true,
        declared_source_cohort_complete: false,
        legacy_history_unknown,
    };
    let mut catalog = SitemapCatalog {
        schema_version: 1,
        summary,
        inherited: None,
        rows: Some(vec![]),
    };
    refresh_size(&mut catalog)?;
    save(transaction, SUMMARY_KEY, &catalog.summary)?;
    Ok(catalog.summary)
}
pub(crate) struct PendingSitemapSource<'tx> {
    connection: &'tx Connection,
    generation: u64,
    kind: SitemapSourceKind,
    task_url: String,
}
/// MUST be the first admission-sensitive operation in the outer source transaction,
/// before policy_ledger::commit. Token cannot be cloned or fabricated by another module.
pub(crate) fn guard_pending_source<'tx>(
    transaction: &'tx Transaction<'_>,
    generation: u64,
    kind: SitemapSourceKind,
    task_url: &str,
) -> Result<PendingSitemapSource<'tx>> {
    active_generation(transaction, generation)?;
    pending(transaction, kind, task_url)?;
    Ok(PendingSitemapSource {
        connection: transaction,
        generation,
        kind,
        task_url: task_url.into(),
    })
}
fn pending(connection: &Connection, kind: SitemapSourceKind, task_url: &str) -> Result<()> {
    let done: Option<i64> = connection
        .query_row(
            "SELECT done FROM discovery_frontier WHERE kind=?1 AND url=?2",
            params![kind.frontier_kind(), task_url],
            |row| row.get(0),
        )
        .optional()?;
    ensure!(
        done == Some(0),
        "Discovery source missing/already committed; no policy or ledger append allowed"
    );
    Ok(())
}
/// Bind original XML membership to actual final policy scheduling/coalescing decisions.
/// Redirect policy rows are retained by policy ledger, not forged into XML Loc rows.
pub(crate) fn bind_final_policy(
    transaction: &Transaction<'_>,
    batch: &mut SitemapDiscoveryBatch,
    rows: &[crate::PolicyOccurrence],
) -> Result<()> {
    ensure!(
        batch.document.kind == SitemapSourceKind::SitemapXml,
        "XML binder received robots source"
    );
    let observed: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.source_revision == batch.document.source_revision
                && row.input_basis == crate::PolicyInputBasis::ParserObservedXml
        })
        .collect();
    if !batch.document.complete() {
        ensure!(
            observed.is_empty(),
            "Incomplete source must produce zero XML policy inputs"
        );
        return Ok(());
    }
    let mut bound = BTreeSet::new();
    for location in &mut batch.locations {
        if location.observed.loc_ordinal.is_none() {
            continue;
        }
        let matching: Vec<_> = observed
            .iter()
            .filter(|row| row.source_ordinal == location.observed.ordinal)
            .collect();
        ensure!(
            matching.len() == 1,
            "Missing/duplicate final policy decision for XML Loc ordinal"
        );
        let row = *matching[0];
        let committed = committed_policy_occurrence(transaction, &row.decision.occurrence_id)?;
        ensure!(
            committed.occurrence == *row && committed.generation == batch.document.generation,
            "Policy/XML decision is not the actual committed final row for this generation"
        );
        ensure!(
            row.decision.source_url.as_deref() == Some(batch.document.task_url.as_str()),
            "Policy source URL is not the requested sitemap task identity"
        );
        let expected = match location.observed.role {
            crate::SitemapLocationRole::PageLocation => crate::CandidateSourceKind::SitemapLocation,
            crate::SitemapLocationRole::ChildSitemap => crate::CandidateSourceKind::SitemapDocument,
        };
        ensure!(
            row.decision.source_kind == expected,
            "Policy/XML source kind differs"
        );
        location.admission = SitemapAdmissionBinding::from_final_policy(&location.observed, row)?;
        if location.observed.role == crate::SitemapLocationRole::ChildSitemap {
            location.referenced_sitemap_task = location.admission.admitted_fetch_url.clone();
        }
        ensure!(
            bound.insert(row.source_ordinal),
            "Duplicate bound policy ordinal"
        );
    }
    ensure!(
        bound.len() == observed.len(),
        "Policy has unexpected XML inputs not represented by observed Loc facts"
    );
    Ok(())
}
pub(crate) fn append_roots(
    transaction: &Transaction<'_>,
    generation: u64,
    roots: Vec<SitemapRootDeclaration>,
) -> Result<SitemapLedgerSummary> {
    active_generation(transaction, generation)?;
    ensure!(
        roots.iter().all(|row| row.source_observation_id.is_none()
            && row.kind != SitemapRootKind::RobotsDirective),
        "Robots roots require a committed robots source batch"
    );
    savepoint(transaction, || {
        append(
            transaction,
            roots.into_iter().map(SitemapLedgerRow::Root).collect(),
        )
    })
}
pub(crate) fn append_source(
    transaction: &Transaction<'_>,
    token: PendingSitemapSource<'_>,
    batch: SitemapDiscoveryBatch,
) -> Result<SitemapLedgerSummary> {
    ensure!(
        std::ptr::eq(token.connection, &**transaction),
        "Pending source token belongs to another connection"
    );
    active_generation(transaction, token.generation)?;
    pending(transaction, token.kind, &token.task_url)?;
    ensure!(
        batch.document.sequence == 0
            && batch.document.generation == token.generation
            && batch.document.kind == token.kind
            && batch.document.task_url == token.task_url,
        "Source batch/token mismatch"
    );
    ensure!(
        batch.document.observation_id
            == source_observation_id(
                token.kind,
                &token.task_url,
                token.generation,
                &batch.document.source_revision
            )?,
        "Source observation ID mismatch"
    );
    ensure!(
        batch
            .roots
            .iter()
            .all(|row| row.kind == SitemapRootKind::RobotsDirective
                && row.source_observation_id.as_deref()
                    == Some(batch.document.observation_id.as_str())),
        "Source root mismatch"
    );
    ensure!(
        batch.document.kind == SitemapSourceKind::RobotsDirectives || batch.roots.is_empty(),
        "Sitemap children must be Loc occurrences, not duplicate roots"
    );
    let document_id = batch.document.observation_id.clone();
    ensure!(
        batch
            .entries
            .iter()
            .all(|row| row.observation_id == document_id)
            && batch
                .locations
                .iter()
                .all(|row| row.observation_id == document_id),
        "Batch contains foreign source facts"
    );
    let mut rows = vec![SitemapLedgerRow::Document(Box::new(batch.document))];
    rows.extend(batch.roots.into_iter().map(SitemapLedgerRow::Root));
    rows.extend(batch.entries.into_iter().map(SitemapLedgerRow::Entry));
    rows.extend(batch.locations.into_iter().map(SitemapLedgerRow::Location));
    savepoint(transaction, || {
        let summary = append(transaction, rows)?;
        let changed = transaction.execute(
            "UPDATE discovery_frontier SET done=1 WHERE kind=?1 AND url=?2 AND done=0",
            params![token.kind.frontier_kind(), token.task_url],
        )?;
        ensure!(
            changed == 1,
            "Source commit did not affect exactly one pending task"
        );
        Ok(summary)
    })
}
fn append(
    transaction: &Transaction<'_>,
    mut new_rows: Vec<SitemapLedgerRow>,
) -> Result<SitemapLedgerSummary> {
    let mut catalog = load_catalog(transaction)?.context("Sitemap ledger not initialized")?;
    ensure!(
        catalog.inherited.is_none() && catalog.summary.rows_available,
        "Selected/inherited ledger cannot append discovery"
    );
    if new_rows.is_empty() {
        return Ok(catalog.summary);
    }
    let row_count = u64::try_from(new_rows.len())?;
    ensure!(
        catalog
            .summary
            .last_sequence
            .checked_add(row_count)
            .is_some_and(|v| v <= catalog.summary.limits.max_rows),
        "Sitemap row budget exceeded; entire source remains pending"
    );
    for row in &mut new_rows {
        ensure!(
            row.sequence() == 0,
            "New row already carries a committed sequence"
        );
        let sequence = catalog
            .summary
            .last_sequence
            .checked_add(1)
            .context("Sitemap sequence overflow")?;
        row.set_sequence(sequence);
        account(&mut catalog.summary, row)?;
    }
    catalog.summary.revision = catalog
        .summary
        .revision
        .checked_add(1)
        .context("Sitemap revision overflow")?;
    catalog
        .rows
        .as_mut()
        .context("Complete sitemap rows unavailable")?
        .extend(new_rows.iter().cloned());
    validate_rows(&catalog)?;
    verify_current_policy_bindings(transaction, &catalog)?;
    refresh_size(&mut catalog)?;
    // Full catalog admission budget is checked before the first SQL row insert.
    for row in &new_rows {
        insert_row(transaction, row, &catalog.summary.limits)?;
    }
    save(transaction, SUMMARY_KEY, &catalog.summary)?;
    Ok(catalog.summary)
}
fn increment(value: &mut u64) -> Result<()> {
    *value = value.checked_add(1).context("Sitemap counter overflow")?;
    Ok(())
}
fn account(summary: &mut SitemapLedgerSummary, row: &SitemapLedgerRow) -> Result<()> {
    ensure!(
        row.sequence()
            == summary
                .last_sequence
                .checked_add(1)
                .context("Sitemap sequence overflow")?,
        "Sitemap global row sequence gap/duplicate"
    );
    let data = row_data(row, &summary.limits)?;
    summary.rolling_sha256 = roll(&summary.rolling_sha256, row.kind(), &data);
    summary.last_sequence = row.sequence();
    match row {
        SitemapLedgerRow::Root(_) => increment(&mut summary.root_rows)?,
        SitemapLedgerRow::Document(row) => {
            increment(&mut summary.document_rows)?;
            if !row.complete() {
                increment(&mut summary.incomplete_documents)?;
            }
        }
        SitemapLedgerRow::Entry(_) => increment(&mut summary.entry_rows)?,
        SitemapLedgerRow::Location(_) => increment(&mut summary.location_rows)?,
    }
    Ok(())
}
fn insert_row(
    connection: &Connection,
    row: &SitemapLedgerRow,
    limits: &SitemapLedgerLimits,
) -> Result<()> {
    let data = row_data(row, limits)?;
    let hash = digest(&data);
    let sequence = i64::try_from(row.sequence())?;
    match row {
        SitemapLedgerRow::Root(row) => {
            connection.execute("INSERT INTO sitemap_roots(sequence,row_id,observation_id,ordinal,data,data_sha256) VALUES(?1,?2,?3,?4,?5,?6)",
            params![sequence,row.declaration_id,row.source_observation_id,i64::try_from(row.source_ordinal)?,data,hash])?;
        }
        SitemapLedgerRow::Document(row) => {
            connection.execute("INSERT INTO sitemap_documents(sequence,row_id,source_kind,task_url,generation,source_revision,data,data_sha256) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![sequence,row.observation_id,row.kind.frontier_kind(),row.task_url,i64::try_from(row.generation)?,row.source_revision,data,hash])?;
        }
        SitemapLedgerRow::Entry(row) => {
            connection.execute("INSERT INTO sitemap_entries(sequence,row_id,observation_id,ordinal,data,data_sha256) VALUES(?1,?2,?3,?4,?5,?6)",
            params![sequence,row.entry_id,row.observation_id,i64::try_from(row.observed.ordinal)?,data,hash])?;
        }
        SitemapLedgerRow::Location(row) => {
            connection.execute("INSERT INTO sitemap_locations(sequence,row_id,observation_id,entry_id,ordinal,data,data_sha256) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![sequence,row.occurrence_id,row.observation_id,identity("entry",&row.observation_id,u64::try_from(row.observed.entry_ordinal)?),
                i64::try_from(row.observed.ordinal)?,data,hash])?;
        }
    }
    Ok(())
}
fn check_binding(binding: &SitemapAdmissionBinding, complete: bool) -> Result<()> {
    use SitemapAdmissionDecision as D;
    ensure!(
        binding.decision != D::Unbound,
        "Unfinalized sitemap admission cannot be committed"
    );
    if !complete {
        ensure!(
            binding.decision == D::IncompleteSource
                && binding.admitted_fetch_url.is_none()
                && binding.policy_occurrence_id.is_none(),
            "Incomplete source cannot admit tasks or XML policy input"
        );
    }
    let admits = matches!(binding.decision, D::Scheduled | D::Coalesced);
    ensure!(
        admits == binding.admitted_fetch_url.is_some(),
        "Admission alias presence differs from decision"
    );
    ensure!(
        binding.policy_occurrence_id.is_some() == binding.policy_source_revision.is_some(),
        "Partial policy binding"
    );
    if binding.policy_occurrence_id.is_none() {
        ensure!(
            binding.coalesced_policy_occurrence_id.is_none(),
            "Ordinary admission has a policy coalescing reference"
        );
        if admits {
            ensure!(
                binding.membership_url.is_some()
                    && binding.admitted_fetch_url == binding.membership_url,
                "Ordinary admission alias differs from original membership"
            );
        }
    }
    if binding.policy_occurrence_id.is_some() && binding.decision == D::Coalesced {
        ensure!(
            binding.coalesced_policy_occurrence_id.is_some(),
            "Policy coalescing reference missing"
        );
    }
    if binding.decision != D::Coalesced {
        ensure!(
            binding.coalesced_policy_occurrence_id.is_none(),
            "Unexpected coalescing reference"
        );
    }
    Ok(())
}
fn validate_document(row: &SitemapSourceDocument) -> Result<()> {
    ensure!(
        !row.task_url.is_empty() && !row.source_revision.is_empty(),
        "Empty document identity"
    );
    ensure!(
        row.observation_id
            == source_observation_id(
                row.kind,
                &row.task_url,
                row.generation,
                &row.source_revision
            )?,
        "Document ID mismatch"
    );
    if let Some(transport) = &row.transport {
        if let Some(requested) = &transport.requested_url {
            ensure!(
                Some(requested.as_str()) == row.expected_requested_url().as_deref(),
                "Transport requested URL differs from source-kind request identity"
            );
        }
        ensure!(
            transport.body_complete == transport.post_http_decoding_sha256.is_some(),
            "Complete body/hash evidence mismatch"
        );
        if let Some(hash) = &transport.post_http_decoding_sha256 {
            sha(hash)?;
        }
        if let Some(hash) = &transport.retained_post_http_prefix_sha256 {
            sha(hash)?;
            if transport.body_complete {
                ensure!(
                    transport.post_http_decoding_sha256.as_deref() == Some(hash.as_str()),
                    "Complete entity digest differs from retained-prefix digest"
                );
            }
        }
        if !transport.request_started {
            ensure!(
                !transport.body_complete
                    && transport.initial_status.is_none()
                    && transport.final_status.is_none(),
                "Unstarted request has completed HTTP evidence"
            );
        }
    }
    if let Some(parser) = &row.parser {
        ensure!(
            row.kind == SitemapSourceKind::SitemapXml,
            "Robots source carries XML parser facts"
        );
        ensure!(
            parser.schema_version == 1 && parser.engine == crate::SITEMAP_XML_ENGINE,
            "Unknown parser observation engine"
        );
        sha(&parser.xml_sha256)?;
        ensure!(
            parser.complete == parser.failure.is_none(),
            "Parser completion/failure differs"
        );
        if parser.complete {
            ensure!(
                parser.kind.is_some() && parser.namespace_profile.is_some(),
                "Complete parser has no accepted document/namespace kind"
            );
        }
        if let Some(requested) = &parser.context.requested_document_url {
            ensure!(
                *requested == row.task_url,
                "Declared parser requested context differs from original task"
            );
        }
        if let (Some(final_url), Some(transport)) =
            (&parser.context.final_document_url, &row.transport)
        {
            ensure!(
                transport.final_url.as_deref() == Some(final_url.as_str()),
                "Declared parser final context differs from transport"
            );
        }
        if let Some(decode) = &row.entity_decode {
            ensure!(
                parser.xml_bytes == decode.decoded_xml_bytes
                    && parser.xml_sha256 == decode.decoded_xml_sha256,
                "Decoder/parser bytes or digest differs"
            );
        }
    }
    if let Some(decode) = &row.entity_decode {
        sha(&decode.decoded_xml_sha256)?;
        if let Some(transport) = &row.transport {
            ensure!(
                u64::try_from(decode.post_http_decode_entity_bytes)?
                    == transport.post_http_decoding_bytes,
                "Transport/decoder input byte count differs"
            );
        }
    }
    if row.kind == SitemapSourceKind::RobotsDirectives {
        ensure!(
            row.parser.is_none() && row.entity_decode.is_none() && row.declaration_count.is_some(),
            "Robots source has XML facts or missing directive count"
        );
    } else {
        ensure!(
            row.declaration_utf8_lossy.is_none() && row.declaration_count.is_none(),
            "XML source has robots markers"
        );
    }
    Ok(())
}
fn validate_rows(catalog: &SitemapCatalog) -> Result<()> {
    let rows = catalog
        .rows
        .as_ref()
        .context("Authoritative sitemap catalog rows unavailable")?;
    let mut ids = BTreeSet::new();
    let mut documents = BTreeMap::new();
    let mut entries = BTreeMap::new();
    let mut tasks = BTreeSet::new();
    let mut locs: BTreeMap<(&str, usize), &StoredSitemapLocation> = BTreeMap::new();
    let mut robots_ordinals = BTreeSet::new();
    let mut entry_locs: BTreeMap<(&str, usize), Vec<usize>> = BTreeMap::new();
    let mut document_entries: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    let mut document_locations: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    let mut document_roots: BTreeMap<&str, Vec<u64>> = BTreeMap::new();
    for row in rows {
        match row {
            SitemapLedgerRow::Document(row) => {
                validate_document(row)?;
                ensure!(
                    ids.insert(row.observation_id.as_str()),
                    "Duplicate sitemap row ID"
                );
                ensure!(
                    tasks.insert((row.kind.frontier_kind(), row.task_url.as_str())),
                    "Duplicate observed source task"
                );
                documents.insert(row.observation_id.as_str(), row);
            }
            SitemapLedgerRow::Root(row) => {
                ensure!(
                    !row.declaration_id.is_empty() && ids.insert(row.declaration_id.as_str()),
                    "Duplicate/empty root ID"
                );
                let basis = row
                    .source_observation_id
                    .as_deref()
                    .unwrap_or(catalog.summary.config_sha256.as_str());
                ensure!(
                    row.declaration_id == root_declaration_id(&row.kind, basis, row.source_ordinal),
                    "Root declaration ID mismatch"
                );
                let complete = if let Some(source) = &row.source_observation_id {
                    let document = documents
                        .get(source.as_str())
                        .context("Root precedes/misses robots source")?;
                    ensure!(
                        document.kind == SitemapSourceKind::RobotsDirectives
                            && row.kind == SitemapRootKind::RobotsDirective,
                        "Root has wrong source kind"
                    );
                    ensure!(
                        robots_ordinals.insert((source.as_str(), row.source_ordinal)),
                        "Duplicate robots directive ordinal"
                    );
                    document_roots
                        .entry(source.as_str())
                        .or_default()
                        .push(row.source_ordinal);
                    let evidence = row
                        .robots_evidence
                        .as_ref()
                        .context("Robots root raw-line qualification missing")?;
                    ensure!(
                        row.raw_line.is_some()
                            && evidence.byte_start <= evidence.byte_end
                            && u64::try_from(evidence.byte_end)?
                                <= document
                                    .transport
                                    .as_ref()
                                    .context("Robots transport missing")?
                                    .post_http_decoding_bytes,
                        "Robots raw-line byte span outside retained body"
                    );
                    let raw_line = row.raw_line.as_deref().context("Robots raw line missing")?;
                    if !evidence.utf8_lossy {
                        ensure!(
                            evidence.byte_end - evidence.byte_start == raw_line.len(),
                            "Lossless robots raw-line byte span differs from retained bytes"
                        );
                    }
                    let content = raw_line.strip_suffix('\n').unwrap_or(raw_line);
                    let content = content.strip_suffix('\r').unwrap_or(content);
                    let (key, value) = content
                        .split_once(':')
                        .context("Robots declaration has no colon")?;
                    ensure!(
                        key.trim().eq_ignore_ascii_case("sitemap")
                            && value == row.raw_value
                            && evidence.line_ordinal > 0,
                        "Robots declaration raw line/value/line ordinal differs"
                    );
                    if document.complete() {
                        ensure!(
                            evidence.line_complete && !evidence.utf8_lossy,
                            "Complete robots source contains unaccepted/lossy directive line"
                        );
                    }
                    document.complete()
                } else {
                    ensure!(
                        row.kind != SitemapRootKind::RobotsDirective
                            && row.robots_evidence.is_none(),
                        "Unbound robots root evidence"
                    );
                    true
                };
                ensure!(
                    row.admission.membership_url == row.normalized_document_url,
                    "Root original membership identity changed"
                );
                check_binding(&row.admission, complete)?;
            }
            SitemapLedgerRow::Entry(row) => {
                let document = documents
                    .get(row.observation_id.as_str())
                    .context("Entry precedes/misses document")?;
                ensure!(
                    document.kind == SitemapSourceKind::SitemapXml && document.parser.is_some(),
                    "Entry without XML parser facts"
                );
                ensure!(
                    row.entry_id
                        == identity(
                            "entry",
                            &row.observation_id,
                            u64::try_from(row.observed.ordinal)?
                        )
                        && ids.insert(row.entry_id.as_str()),
                    "Entry ID mismatch/duplicate"
                );
                ensure!(
                    entries
                        .insert((row.observation_id.as_str(), row.observed.ordinal), row)
                        .is_none(),
                    "Duplicate entry ordinal"
                );
                document_entries
                    .entry(row.observation_id.as_str())
                    .or_default()
                    .push(row.observed.ordinal);
                ensure!(
                    row.observed.xml_byte_start <= row.observed.xml_byte_end
                        && row.observed.xml_byte_end
                            <= u64::try_from(
                                document
                                    .parser
                                    .as_ref()
                                    .expect("validated parser")
                                    .xml_bytes
                            )?,
                    "Entry XML byte span outside document"
                );
            }
            SitemapLedgerRow::Location(row) => {
                let document = documents
                    .get(row.observation_id.as_str())
                    .context("Loc precedes/misses document")?;
                let entry = entries
                    .get(&(row.observation_id.as_str(), row.observed.entry_ordinal))
                    .context("Loc precedes/misses entry")?;
                ensure!(
                    row.occurrence_id
                        == identity(
                            "location",
                            &row.observation_id,
                            u64::try_from(row.observed.ordinal)?
                        )
                        && ids.insert(row.occurrence_id.as_str()),
                    "Loc ID mismatch/duplicate"
                );
                ensure!(
                    locs.insert((row.observation_id.as_str(), row.observed.ordinal), row)
                        .is_none(),
                    "Duplicate Loc row ordinal"
                );
                document_locations
                    .entry(row.observation_id.as_str())
                    .or_default()
                    .push(row.observed.ordinal);
                if row.observed.loc_ordinal.is_some() {
                    entry_locs
                        .entry((row.observation_id.as_str(), row.observed.entry_ordinal))
                        .or_default()
                        .push(row.observed.ordinal);
                }
                ensure!(
                    row.observed.xml_byte_start >= entry.observed.xml_byte_start
                        && row.observed.xml_byte_end <= entry.observed.xml_byte_end
                        && row.observed.xml_byte_start <= row.observed.xml_byte_end,
                    "Loc XML byte span outside entry"
                );
                ensure!(
                    row.admission.membership_url == row.observed.normalized_fetch_url,
                    "Original listed membership identity changed"
                );
                check_binding(&row.admission, document.complete())?;
                if row.observed.loc_ordinal.is_none() {
                    ensure!(
                        entry.observed.complete
                            && row.observed.raw_element_xml.is_none()
                            && row.observed.raw_inner_xml.is_none()
                            && row.observed.decoded_value.is_none()
                            && row.observed.normalized_fetch_url.is_none()
                            && row.observed.validation_error.as_deref() == Some("missing_location"),
                        "Invalid synthetic missing-Loc row"
                    );
                    ensure!(
                        matches!(
                            row.admission.decision,
                            SitemapAdmissionDecision::MissingLocation
                                | SitemapAdmissionDecision::IncompleteSource
                        ),
                        "Missing Loc got an admission decision"
                    );
                } else {
                    ensure!(
                        row.observed.raw_element_xml.is_some()
                            && row.observed.raw_inner_xml.is_some()
                            && row.observed.decoded_value.is_some(),
                        "Observed Loc raw/decoded fields missing"
                    );
                    if row.observed.validation_error.is_some()
                        || row.observed.normalized_fetch_url.is_none()
                    {
                        ensure!(
                            matches!(
                                row.admission.decision,
                                SitemapAdmissionDecision::InvalidLocation
                                    | SitemapAdmissionDecision::IncompleteSource
                            ),
                            "Invalid Loc got admission"
                        );
                    }
                }
                let expected_child =
                    if row.observed.role == crate::SitemapLocationRole::ChildSitemap {
                        row.admission.admitted_fetch_url.as_ref()
                    } else {
                        None
                    };
                ensure!(
                    row.referenced_sitemap_task.as_ref() == expected_child,
                    "Child edge differs from final admitted task alias"
                );
                if row.admission.policy_occurrence_id.is_some() {
                    ensure!(
                        row.admission.policy_source_revision.as_deref()
                            == Some(document.source_revision.as_str()),
                        "Policy/source revision differs"
                    );
                }
            }
        }
    }
    for ((document_id, _), entry) in entries {
        let mut actual = entry_locs
            .remove(&(document_id, entry.observed.ordinal))
            .unwrap_or_default();
        actual.sort_unstable();
        let mut declared = entry.observed.loc_occurrence_ordinals.clone();
        declared.sort_unstable();
        ensure!(
            actual == declared,
            "Entry Loc occurrence references differ from retained rows"
        );
    }
    for (document_id, document) in documents {
        if let Some(count) = document.declaration_count {
            let observed = document_roots.remove(document_id).unwrap_or_default();
            ensure!(
                observed.len() == count && observed.iter().copied().eq(0..u64::try_from(count)?),
                "Retained robots directive counts/ordinals differ from authoritative catalog"
            );
        }
        let Some(parser) = &document.parser else {
            continue;
        };
        let observed_entries = document_entries.remove(document_id).unwrap_or_default();
        let observed_locations = document_locations.remove(document_id).unwrap_or_default();
        ensure!(
            observed_entries.len() == parser.entry_count
                && observed_locations.len() == parser.location_count,
            "Retained parser row counts differ from authoritative catalog"
        );
        ensure!(
            observed_entries.iter().copied().eq(1..=parser.entry_count)
                && observed_locations
                    .iter()
                    .copied()
                    .eq(1..=parser.location_count),
            "Parser entry/Loc ordinal gaps or reorder"
        );
    }
    Ok(())
}
pub(crate) fn validate_catalog(catalog: &SitemapCatalog) -> Result<()> {
    ensure!(
        catalog.schema_version == 1
            && catalog.summary.engine == SITEMAP_LEDGER_ENGINE
            && catalog.summary.hash_profile == SITEMAP_LEDGER_HASH_PROFILE,
        "Unknown sitemap catalog/hash engine"
    );
    catalog.summary.limits.validate()?;
    sha(&catalog.summary.config_sha256)?;
    sha(&catalog.summary.rolling_sha256)?;
    ensure!(
        !catalog.summary.declared_source_cohort_complete,
        "Source-cohort coverage is not implemented by this engine"
    );
    ensure!(
        catalog.summary.rows_available && catalog.rows.is_some(),
        "Sitemap marker requires complete rows in this candidate"
    );
    if let Some(inherited) = &catalog.inherited {
        sha(&inherited.parent_snapshot_sha256)?;
        sha(&inherited.parent_catalog_sha256)?;
        ensure!(
            !inherited.current_discovery_executed,
            "Inherited history cannot attest current discovery"
        );
    }
    let mut computed = catalog.summary.clone();
    computed.last_sequence = 0;
    computed.root_rows = 0;
    computed.document_rows = 0;
    computed.entry_rows = 0;
    computed.location_rows = 0;
    computed.incomplete_documents = 0;
    computed.rolling_sha256 = seed_hash();
    for row in catalog.rows.as_ref().expect("validated rows") {
        account(&mut computed, row)?;
    }
    ensure!(
        computed.last_sequence <= computed.limits.max_rows
            && computed.revision <= computed.last_sequence,
        "Sitemap row/revision bounds invalid"
    );
    ensure!(
        computed == catalog.summary,
        "Sitemap counts/rolling SHA/global sequence verification failed"
    );
    ensure!(
        measured(catalog, catalog.summary.limits.max_catalog_json_bytes)?
            == catalog.summary.catalog_json_bytes,
        "Sitemap full serialized envelope byte count differs"
    );
    validate_rows(catalog)
}
fn committed_policy_occurrence(
    connection: &Connection,
    occurrence_id: &str,
) -> Result<crate::StoredPolicyOccurrence> {
    let bytes: i64 = connection
        .query_row(
            "SELECT length(CAST(data AS BLOB)) FROM policy_occurrences WHERE occurrence_id=?1",
            [occurrence_id],
            |row| row.get(0),
        )
        .optional()?
        .context("Sitemap references unavailable committed policy row")?;
    ensure!(
        (0..=1024 * 1024).contains(&bytes),
        "Oversized policy row referenced by sitemap"
    );
    let (sequence, generation, data): (i64, i64, String) = connection.query_row(
        "SELECT sequence,generation,data FROM policy_occurrences WHERE occurrence_id=?1",
        [occurrence_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    let stored: crate::StoredPolicyOccurrence = serde_json::from_str(&data)?;
    ensure!(
        stored.occurrence.decision.occurrence_id == occurrence_id
            && stored.sequence == u64::try_from(sequence)?
            && stored.generation == u64::try_from(generation)?,
        "Policy SQL identity/sequence/generation differs from payload"
    );
    Ok(stored)
}
/// Single shared profile for original robots declaration identity. Root's
/// strict_robots_location adapter should delegate here when integrating v3.
/// Preserve untrimmed raw facts; only surrounding XML S is trimmed for the URL.
pub(crate) fn original_robots_document_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim_matches([' ', '\t', '\r', '\n']);
    if trimmed.is_empty() || trimmed.trim() != trimmed || trimmed.chars().any(char::is_control) {
        return None;
    }
    let url = url::Url::parse(trimmed).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    crate::normalize_url(trimmed, None).map(|url| url.to_string())
}
/// Original coordinates are checked before ordinary admission and on both
/// append/read. This is an additional bounded scalar check, not the Store's
/// mandatory full typed saved-config/import/run-plan verification.
pub(crate) fn verify_original_root_coordinates(
    connection: &Connection,
    root: &SitemapRootDeclaration,
) -> Result<()> {
    ensure!(
        root.admission.membership_url == root.normalized_document_url,
        "Root original membership identity changed"
    );
    match root.kind {
        SitemapRootKind::RobotsDirective => ensure!(
            root.normalized_document_url == original_robots_document_url(&root.raw_value),
            "Robots root original normalized identity differs from raw declaration"
        ),
        SitemapRootKind::ExplicitConfig => {
            let available:Option<bool>=connection.query_row(
                "SELECT json_type(value,'$.explicit_sitemap_inputs')='array' FROM metadata WHERE key='config'",
                [],|row|row.get(0)).optional()?.flatten();
            ensure!(
                available == Some(true),
                "Original explicit sitemap coordinates unavailable"
            );
            let matched:Option<bool>=connection.query_row(
                "SELECT COALESCE(input.type='text' AND input.value=?1,0) FROM metadata AS config,json_each(config.value,'$.explicit_sitemap_inputs') AS input WHERE config.key='config' AND CAST(input.key AS INTEGER)=?2",
                params![root.raw_value,i64::try_from(root.source_ordinal)?],|row|row.get(0)).optional()?;
            ensure!(
                matched == Some(true),
                "Explicit root source ordinal/raw differs from original config input"
            );
            ensure!(
                root.normalized_document_url
                    == crate::normalize_url(&root.raw_value, None).map(|url| url.to_string()),
                "Explicit root original normalized identity differs from raw config input"
            );
            if root.normalized_document_url.is_none() {
                ensure!(
                    root.admission.decision == SitemapAdmissionDecision::InvalidLocation
                        && root.admission.admitted_fetch_url.is_none(),
                    "Explicit root original invalid marker differs from admission"
                );
            }
        }
        SitemapRootKind::ImportedSitemapRow => {
            original_imported_root_coordinates(connection, root)?;
        }
    }
    Ok(())
}
struct OriginalImportedRootCoordinates {
    input_index: i64,
    input_sha: String,
    source_index: i64,
}
fn original_imported_root_coordinates(
    connection: &Connection,
    root: &SitemapRootDeclaration,
) -> Result<OriginalImportedRootCoordinates> {
    // Flatten ONLY sitemap_document rows. Compare raw/norm/error inside SQLite;
    // return only fixed/bounded coordinates, never unrelated arrays/raw values.
    // IS handles actual normalized_url:null without SQLite nullable bool casts.
    let found:Option<(i64,String,i64,bool,bool,bool,bool)>=connection.query_row(
        "SELECT CAST(input.key AS INTEGER),CASE WHEN json_type(input.value,'$.input_sha256')='text' AND length(CAST(json_extract(input.value,'$.input_sha256') AS BLOB))=64 THEN json_extract(input.value,'$.input_sha256') ELSE '' END,CASE WHEN json_type(row.value,'$.source_index')='integer' THEN json_extract(row.value,'$.source_index') ELSE -1 END,COALESCE(json_type(row.value,'$.raw_value')='text' AND json_extract(row.value,'$.raw_value')=?1,0),COALESCE(json_type(row.value,'$.normalized_url') IN ('null','text') AND json_extract(row.value,'$.normalized_url') IS ?2,0),COALESCE(json_type(row.value,'$.error') IN ('null','text'),0),COALESCE(json_type(row.value,'$.error')='text',0) FROM metadata AS config,json_each(config.value,'$.input_imports') AS input,json_each(input.value,'$.rows') AS row WHERE config.key='config' AND json_type(config.value,'$.input_imports')='array' AND json_type(input.value,'$.rows')='array' AND json_extract(row.value,'$.candidate_kind')='sitemap_document' ORDER BY CAST(input.key AS INTEGER),CAST(row.key AS INTEGER) LIMIT 1 OFFSET ?3",
        params![root.raw_value,root.normalized_document_url,i64::try_from(root.source_ordinal)?],
        |row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?))).optional()?;
    let (input_index, input_sha, source_index, raw_equal, normalized_equal, error_known, has_error) =
        found.context("Imported sitemap root ordinal has no original input row")?;
    sha(&input_sha)?;
    ensure!(
        raw_equal && normalized_equal && input_index >= 0 && source_index >= 0,
        "Imported root raw/normalized/source index differs from original input row"
    );
    ensure!(
        error_known,
        "Imported root original error marker unavailable"
    );
    if has_error || root.normalized_document_url.is_none() {
        ensure!(
            root.admission.decision == SitemapAdmissionDecision::InvalidLocation
                && root.admission.admitted_fetch_url.is_none(),
            "Imported root original error/invalid marker differs from admission"
        );
    }
    Ok(OriginalImportedRootCoordinates {
        input_index,
        input_sha,
        source_index,
    })
}
/// Preserve policy raw/basis, observation, history, revision and source-index
/// ordering. Ordinary roots use the same original-coordinate checks without
/// inventing a policy occurrence or weakening committed-policy identity.
fn verify_declared_root_coordinates(
    connection: &Connection,
    root: &SitemapRootDeclaration,
    stored: &crate::StoredPolicyOccurrence,
) -> Result<()> {
    let occurrence = &stored.occurrence;
    ensure!(
        occurrence.decision.raw_value == root.raw_value
            && occurrence.input_basis == crate::PolicyInputBasis::DeclaredInput
            && occurrence.raw_value_observed,
        "Root policy raw/basis differs from original declaration"
    );
    ensure!(
        stored.basis == crate::PolicyObservationBasis::Current,
        "Current root references noncurrent policy SQL observation"
    );
    let run: crate::CrawlRunState = metadata(connection, "run_state")?
        .context("Root policy binding has no actual run history")?;
    ensure!(
        stored.generation > 0 && stored.generation <= run.generation,
        "Root policy generation outside actual run history"
    );
    match root.kind {
        SitemapRootKind::RobotsDirective => verify_original_root_coordinates(connection, root)?,
        SitemapRootKind::ExplicitConfig => {
            verify_original_root_coordinates(connection, root)?;
            ensure!(
                u64::try_from(occurrence.source_ordinal)? == root.source_ordinal,
                "Explicit root source ordinal/raw differs from original config input"
            );
            let expected = format!(
                "initial:{}:explicit:sitemap_document",
                occurrence.decision.policy_sha256
            );
            ensure!(
                occurrence.source_revision == expected && occurrence.decision.source_url.is_none(),
                "Explicit root policy revision/source differs from original config input"
            );
        }
        SitemapRootKind::ImportedSitemapRow => {
            let original = original_imported_root_coordinates(connection, root)?;
            ensure!(
                usize::try_from(original.source_index)? == occurrence.source_ordinal,
                "Imported root raw/normalized/source index differs from original input row"
            );
            let expected = format!(
                "initial:{}:{}:{}",
                occurrence.decision.policy_sha256, original.input_index, original.input_sha
            );
            ensure!(
                occurrence.source_revision == expected && occurrence.decision.source_url.is_none(),
                "Imported root policy revision/source differs from original input row"
            );
        }
    }
    Ok(())
}
fn verify_current_policy_bindings(connection: &Connection, catalog: &SitemapCatalog) -> Result<()> {
    // Inherited IDs refer to parent policy history, which selected stores intentionally do not copy.
    // Their basis is the verified immutable parent snapshot/catalog, not current SQL policy rows.
    if catalog.inherited.is_some() {
        return Ok(());
    }
    let rows = catalog.rows.as_ref().context("Sitemap rows unavailable")?;
    let documents: BTreeMap<_, _> = rows
        .iter()
        .filter_map(|row| match row {
            SitemapLedgerRow::Document(row) => Some((row.observation_id.as_str(), row)),
            _ => None,
        })
        .collect();
    for row in rows {
        let binding = match row {
            SitemapLedgerRow::Root(row) => &row.admission,
            SitemapLedgerRow::Location(row) => &row.admission,
            _ => continue,
        };
        let Some(id) = &binding.policy_occurrence_id else {
            if let SitemapLedgerRow::Root(root) = row {
                verify_original_root_coordinates(connection, root)?;
            }
            continue;
        };
        let stored = committed_policy_occurrence(connection, id)?;
        ensure!(
            binding.policy_source_revision.as_deref()
                == Some(stored.occurrence.source_revision.as_str()),
            "Sitemap policy reference/source revision differs"
        );
        match row {
            SitemapLedgerRow::Location(row) => {
                let document = documents
                    .get(row.observation_id.as_str())
                    .context("Policy-bound Loc document missing")?;
                ensure!(
                    stored.generation == document.generation
                        && stored.occurrence.decision.source_url.as_deref()
                            == Some(document.task_url.as_str()),
                    "Policy-bound Loc generation/source differs"
                );
                let expected = match row.observed.role {
                    crate::SitemapLocationRole::PageLocation => {
                        crate::CandidateSourceKind::SitemapLocation
                    }
                    crate::SitemapLocationRole::ChildSitemap => {
                        crate::CandidateSourceKind::SitemapDocument
                    }
                };
                ensure!(
                    stored.occurrence.decision.source_kind == expected,
                    "Policy-bound Loc source kind differs"
                );
                ensure!(
                    SitemapAdmissionBinding::from_final_policy(&row.observed, &stored.occurrence)?
                        == *binding,
                    "Persisted Loc binding differs from actual committed final policy decision"
                );
            }
            SitemapLedgerRow::Root(root) => {
                ensure!(
                    stored.occurrence.task_role == crate::PolicyTaskRole::SitemapDocument
                        && stored.occurrence.decision.source_kind
                            == crate::CandidateSourceKind::SitemapDocument,
                    "Root policy row is not a sitemap protocol admission"
                );
                verify_declared_root_coordinates(connection, root, &stored)?;
                if let Some(source) = &root.source_observation_id {
                    let document = documents
                        .get(source.as_str())
                        .context("Policy-bound robots source missing")?;
                    ensure!(
                        stored.generation == document.generation
                            && stored.occurrence.decision.source_url.as_deref()
                                == Some(document.task_url.as_str())
                            && u64::try_from(stored.occurrence.source_ordinal)?
                                == root.source_ordinal
                            && stored.occurrence.source_revision == document.source_revision,
                        "Policy-bound robots generation/source/ordinal/revision differs"
                    );
                }
                ensure!(
                    stored.occurrence.admission_reason != "pending_admission",
                    "Root policy scheduling is unfinished"
                );
                match binding.decision {
                    SitemapAdmissionDecision::Scheduled => ensure!(
                        stored.occurrence.admission_reason == "scheduled"
                            && stored.occurrence.scheduled_fetch_url == binding.admitted_fetch_url,
                        "Root policy scheduled alias differs"
                    ),
                    SitemapAdmissionDecision::Coalesced => ensure!(
                        stored.occurrence.admission_reason == "coalesced"
                            && stored.occurrence.decision.fetch_candidate_url
                                == binding.admitted_fetch_url
                            && stored.occurrence.coalesced_task_occurrence_id
                                == binding.coalesced_policy_occurrence_id,
                        "Root policy coalesced alias/reference differs"
                    ),
                    _ => ensure!(
                        stored.occurrence.scheduled_fetch_url.is_none()
                            && stored.occurrence.coalesced_task_occurrence_id.is_none(),
                        "Unadmitted root has a committed policy task"
                    ),
                }
            }
            _ => unreachable!("bindings only from roots/locations"),
        }
    }
    Ok(())
}
pub(crate) fn load_catalog(connection: &Connection) -> Result<Option<SitemapCatalog>> {
    let state: Option<SitemapLedgerSummary> = metadata(connection, SUMMARY_KEY)?;
    let inherited: Option<SitemapInheritedBasis> = metadata(connection, INHERITED_KEY)?;
    let Some(summary) = state else {
        ensure!(
            inherited.is_none(),
            "Inherited sitemap marker has no ledger summary"
        );
        let tables = present_tables(connection)?;
        if tables != 0 {
            require_empty(connection)?;
        }
        return Ok(None); // Tables created on old writable open do not prove an empty source history.
    };
    summary.limits.validate()?;
    require_tables(connection)?;
    let mut total_bytes = 0u64;
    let mut total_rows = 0u64;
    for table in TABLES {
        let (count,bytes,max):(i64,i64,i64)=connection.query_row(&format!(
            "SELECT count(*),COALESCE(SUM(length(CAST(data AS BLOB))),0),COALESCE(MAX(length(CAST(data AS BLOB))),0) FROM {table}"),
            [],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?)))?;
        total_bytes = total_bytes
            .checked_add(u64::try_from(bytes)?)
            .context("SQL row byte count overflow")?;
        total_rows = total_rows
            .checked_add(u64::try_from(count)?)
            .context("SQL row count overflow")?;
        ensure!(
            u64::try_from(max)? <= u64::try_from(summary.limits.max_row_json_bytes)?,
            "Oversized durable sitemap row"
        );
    }
    ensure!(
        total_bytes <= u64::try_from(summary.limits.max_catalog_json_bytes)?
            && total_rows <= summary.limits.max_rows,
        "Durable sitemap rows exceed bounded catalog; refusing materialization"
    );
    let mut rows = Vec::with_capacity(usize::try_from(total_rows)?);
    let sql="SELECT 'root',sequence,row_id,observation_id,ordinal,NULL,NULL,NULL,NULL,data,data_sha256 FROM sitemap_roots
        UNION ALL SELECT 'document',sequence,row_id,NULL,NULL,source_kind,task_url,generation,source_revision,data,data_sha256 FROM sitemap_documents
        UNION ALL SELECT 'entry',sequence,row_id,observation_id,ordinal,NULL,NULL,NULL,NULL,data,data_sha256 FROM sitemap_entries
        UNION ALL SELECT 'location',sequence,row_id,observation_id,ordinal,NULL,NULL,NULL,entry_id,data,data_sha256 FROM sitemap_locations
        ORDER BY 2";
    let mut statement = connection.prepare(sql)?;
    let mut cursor = statement.query([])?;
    while let Some(sql) = cursor.next()? {
        let kind: String = sql.get(0)?;
        let sequence: i64 = sql.get(1)?;
        let id: String = sql.get(2)?;
        let observation: Option<String> = sql.get(3)?;
        let ordinal: Option<i64> = sql.get(4)?;
        let source_kind: Option<String> = sql.get(5)?;
        let task_url: Option<String> = sql.get(6)?;
        let generation: Option<i64> = sql.get(7)?;
        let revision_or_entry: Option<String> = sql.get(8)?;
        let data: String = sql.get(9)?;
        let stored_hash: String = sql.get(10)?;
        ensure!(
            digest(&data) == stored_hash,
            "Durable sitemap row payload SHA mismatch"
        );
        let typed: SitemapLedgerRow =
            serde_json::from_str(&data).context("Invalid typed sitemap row JSON")?;
        ensure!(
            typed.kind() == kind
                && typed.sequence() == u64::try_from(sequence)?
                && row_data(&typed, &summary.limits)? == data,
            "Noncanonical sitemap row/type/sequence"
        );
        match &typed {
            SitemapLedgerRow::Root(row) => ensure!(
                id == row.declaration_id
                    && observation == row.source_observation_id
                    && ordinal == Some(i64::try_from(row.source_ordinal)?),
                "Root SQL indexes differ from payload"
            ),
            SitemapLedgerRow::Document(row) => ensure!(
                id == row.observation_id
                    && source_kind.as_deref() == Some(row.kind.frontier_kind())
                    && task_url.as_deref() == Some(row.task_url.as_str())
                    && generation == Some(i64::try_from(row.generation)?)
                    && revision_or_entry.as_deref() == Some(row.source_revision.as_str()),
                "Document SQL indexes differ from payload"
            ),
            SitemapLedgerRow::Entry(row) => ensure!(
                id == row.entry_id
                    && observation.as_deref() == Some(row.observation_id.as_str())
                    && ordinal == Some(i64::try_from(row.observed.ordinal)?),
                "Entry SQL indexes differ from payload"
            ),
            SitemapLedgerRow::Location(row) => ensure!(
                id == row.occurrence_id
                    && observation.as_deref() == Some(row.observation_id.as_str())
                    && ordinal == Some(i64::try_from(row.observed.ordinal)?)
                    && revision_or_entry
                        == Some(identity(
                            "entry",
                            &row.observation_id,
                            u64::try_from(row.observed.entry_ordinal)?
                        )),
                "Loc SQL indexes differ from payload"
            ),
        }
        rows.push(typed);
    }
    let catalog = SitemapCatalog {
        schema_version: 1,
        summary,
        inherited,
        rows: Some(rows),
    };
    validate_catalog(&catalog)?;
    verify_inherited_anchor(connection, &catalog)?;
    verify_current_policy_bindings(connection, &catalog)?;
    Ok(Some(catalog))
}
pub(crate) fn summary(connection: &Connection) -> Result<Option<SitemapLedgerSummary>> {
    Ok(load_catalog(connection)?.map(|catalog| catalog.summary))
}
#[derive(serde::Deserialize)]
struct ParentCatalogEnvelope {
    // Ignore unrelated report values rather than materializing/re-serializing
    // pages or arbitrary embedded JSON. Hash the exact snapshot bytes first.
    sitemap_catalog: Option<SitemapCatalog>,
}
fn verify_inherited_anchor(connection: &Connection, catalog: &SitemapCatalog) -> Result<()> {
    let Some(basis) = &catalog.inherited else {
        return Ok(());
    };
    // Full selected plans legitimately exceed the 64KiB sitemap-summary bound.
    // Project only the anchor fields through SQLite's existing JSON layer; do
    // not allocate the whole selection/targets or impose a new plan-size cap.
    // Store's existing full typed plan validator remains mandatory separately.
    let (kind,engine,parent_sha):(Option<String>,Option<String>,Option<String>)=connection.query_row(
        "SELECT json_extract(value,'$.kind'),json_extract(value,'$.plan.engine'),json_extract(value,'$.plan.parent.snapshot_sha256') FROM metadata WHERE key='run_plan'",
        [],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?
        .context("Inherited catalog lacks immutable selected run plan")?;
    ensure!(
        kind.as_deref() == Some("selected_recrawl"),
        "Inherited catalog requires selected run plan"
    );
    ensure!(
        engine.as_deref() == Some(crate::SELECTED_RECRAWL_ENGINE)
            && parent_sha.as_deref() == Some(basis.parent_snapshot_sha256.as_str()),
        "Inherited parent snapshot differs from selected run plan"
    );
    let bytes: i64 = connection
        .query_row(
            "SELECT length(snapshot) FROM recrawl_base WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .context("Inherited catalog lacks immutable parent snapshot")?;
    ensure!(
        (1..=268435456).contains(&bytes),
        "Immutable parent snapshot exceeds selected snapshot bound"
    );
    let snapshot: Vec<u8> = connection.query_row(
        "SELECT snapshot FROM recrawl_base WHERE id=1 AND length(snapshot)<=268435456",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        digest(&snapshot) == basis.parent_snapshot_sha256,
        "Inherited parent snapshot bytes changed"
    );
    let parent: SitemapCatalog = serde_json::from_slice::<ParentCatalogEnvelope>(&snapshot)
        .context("Invalid immutable parent catalog envelope")?
        .sitemap_catalog
        .context("Immutable parent has no sitemap catalog")?;
    validate_catalog(&parent)?;
    ensure!(
        digest(serde_json::to_vec(&parent)?) == basis.parent_catalog_sha256,
        "Inherited parent catalog digest changed"
    );
    let mut expected = parent;
    expected.inherited = Some(basis.clone());
    refresh_size(&mut expected)?;
    ensure!(
        expected == *catalog,
        "Inherited rows/summary differ from immutable parent catalog"
    );
    Ok(())
}
/// Selected recrawl calls inside initialize_selected_recrawl's empty-database transaction.
/// Parent snapshot SHA must have been verified against exact parent report bytes by store/recrawl.
pub(crate) fn project_inherited_catalog(
    parent: &SitemapCatalog,
    parent_snapshot_sha256: &str,
) -> Result<SitemapCatalog> {
    sha(parent_snapshot_sha256)?;
    validate_catalog(parent)?;
    let mut inherited = parent.clone();
    inherited.inherited = Some(SitemapInheritedBasis {
        parent_snapshot_sha256: parent_snapshot_sha256.into(),
        parent_catalog_sha256: digest(serde_json::to_vec(parent)?),
        current_discovery_executed: false,
    });
    refresh_size(&mut inherited)?;
    validate_catalog(&inherited)?;
    Ok(inherited)
}

pub(crate) fn inherit(
    transaction: &Transaction<'_>,
    parent: &SitemapCatalog,
    parent_snapshot_sha256: &str,
) -> Result<SitemapLedgerSummary> {
    let inherited = project_inherited_catalog(parent, parent_snapshot_sha256)?;
    require_empty(transaction)?;
    ensure!(
        metadata_text(transaction, SUMMARY_KEY)?.is_none()
            && metadata_text(transaction, INHERITED_KEY)?.is_none(),
        "Selected inheritance already initialized"
    );
    // Same original rows, IDs, generations, global sequence and rolling digest; new bounded envelope.
    // Store must have inserted validated run_plan and exact recrawl_base bytes
    // inside this same selected initialization transaction before inheritance.
    verify_inherited_anchor(transaction, &inherited)?;
    savepoint(transaction, || {
        for row in inherited
            .rows
            .as_ref()
            .context("Parent sitemap rows unavailable")?
        {
            insert_row(transaction, row, &inherited.summary.limits)?;
        }
        save(transaction, SUMMARY_KEY, &inherited.summary)?;
        save(
            transaction,
            INHERITED_KEY,
            inherited
                .inherited
                .as_ref()
                .context("Inherited basis missing")?,
        )?;
        Ok(inherited.summary)
    })
}
/// Caller uses one read transaction and passes BOTH identity fields for subsequent pages.
/// This first bounded candidate verifies/materializes the <=32MiB catalog before slicing.
pub(crate) fn page(
    connection: &Connection,
    after_sequence: u64,
    limit: usize,
    expected_revision: Option<u64>,
    expected_rolling_sha256: Option<&str>,
) -> Result<SitemapLedgerPage> {
    ensure!(
        (1..=1000).contains(&limit),
        "Sitemap page limit must be 1..=1000"
    );
    ensure!(
        expected_revision.is_some() == expected_rolling_sha256.is_some(),
        "Partial sitemap cursor identity"
    );
    let Some(catalog) = load_catalog(connection)? else {
        ensure!(
            after_sequence == 0 && expected_revision.is_none(),
            "Unavailable legacy catalog has no cursor"
        );
        return Ok(SitemapLedgerPage {
            ledger: None,
            inherited: None,
            rows: vec![],
            next_after_sequence: None,
            has_more: false,
        });
    };
    if let (Some(revision), Some(hash)) = (expected_revision, expected_rolling_sha256) {
        ensure!(
            revision == catalog.summary.revision && hash == catalog.summary.rolling_sha256,
            "Sitemap cursor stale: restart with the committed revision/hash"
        );
    }
    ensure!(
        after_sequence <= catalog.summary.last_sequence,
        "Sitemap cursor beyond durable sequence"
    );
    ensure!(
        after_sequence == 0 || expected_revision.is_some(),
        "Continuation cursor requires revision and rolling SHA"
    );
    let rows: Vec<_> = catalog
        .rows
        .context("Authoritative sitemap rows missing")?
        .into_iter()
        .filter(|row| row.sequence() > after_sequence)
        .take(limit)
        .collect();
    let next = rows.last().map(SitemapLedgerRow::sequence);
    let has_more = next.is_some_and(|sequence| sequence < catalog.summary.last_sequence);
    Ok(SitemapLedgerPage {
        ledger: Some(catalog.summary),
        inherited: catalog.inherited,
        rows,
        next_after_sequence: has_more.then_some(next).flatten(),
        has_more,
    })
}

#[cfg(test)]
#[path = "sitemap_ledger_tests.rs"]
mod tests;
