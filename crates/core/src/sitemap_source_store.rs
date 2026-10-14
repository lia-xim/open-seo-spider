//! Bounded durable source observations; no HTTP or task admission.
//! Caller owns transactions. A MUST commit before B. Propagate B errors to roll
//! back policy/frontier/warnings/catalog/association together. Never reuse A's token.
use crate::{
    SitemapCatalog, SitemapLedgerRow, SitemapSourceDocument, SitemapSourceKind,
    SitemapSourceMeasurement, SitemapSourceMeasurementLimits,
};
use anyhow::{Context, Result, bail, ensure};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
};

#[path = "sitemap_source_store_models.rs"]
mod models;
pub use models::*;
pub(crate) const SCHEMA: &str = include_str!("sitemap_source_schema.sql");
const KEY: &str = "sitemap_source_store";
const TABLES: [&str; 2] = [
    "sitemap_source_measurements",
    "sitemap_source_measurement_commits",
];
const FRAME_BYTES: u64 =
    b"{\"schema_version\":1,\"summary\":,\"records\":[],\"commits\":[]}".len() as u64;

fn digest(bytes: impl AsRef<[u8]>) -> String {
    crate::sitemap_ledger::digest(bytes)
}
fn sha(value: &str) -> Result<()> {
    ensure!(
        value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "Invalid source-store SHA256"
    );
    Ok(())
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).context("Source-store byte/count overflow")
}
struct BoundedJson {
    bytes: Vec<u8>,
    cap: usize,
    exceeded: bool,
}
impl Write for BoundedJson {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.cap.saturating_sub(self.bytes.len()) {
            self.exceeded = true;
            return Err(io::Error::other("Source-store JSON cap"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn limited_json(value: &impl Serialize, cap: usize) -> Result<Option<String>> {
    let mut writer = BoundedJson {
        bytes: Vec::new(),
        cap,
        exceeded: false,
    };
    if let Err(error) = serde_json::to_writer(&mut writer, value) {
        if writer.exceeded {
            return Ok(None);
        }
        return Err(error.into());
    }
    Ok(Some(String::from_utf8(writer.bytes)?))
}
fn json(value: &impl Serialize, cap: usize) -> Result<String> {
    limited_json(value, cap)?.context("Oversized source-store canonical JSON")
}
fn bounded_metadata(connection: &Connection, key: &str, cap: usize) -> Result<Option<String>> {
    let len: Option<i64> = connection
        .query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key=?1",
            [key],
            |r| r.get(0),
        )
        .optional()?;
    let Some(len) = len else { return Ok(None) };
    ensure!(
        len >= 0 && u64::try_from(len)? <= u64::try_from(cap)?,
        "Oversized source-store metadata: {key}"
    );
    let text: String =
        connection.query_row("SELECT value FROM metadata WHERE key=?1", [key], |r| {
            r.get(0)
        })?;
    ensure!(
        i64::try_from(text.len())? == len,
        "Source-store metadata length changed"
    );
    Ok(Some(text))
}
fn metadata<T: DeserializeOwned>(
    connection: &Connection,
    key: &str,
    cap: usize,
) -> Result<Option<T>> {
    bounded_metadata(connection, key, cap)?
        .map(|v| serde_json::from_str(&v).map_err(Into::into))
        .transpose()
}
fn save_summary(connection: &Connection, summary: &SitemapSourceStoreSummary) -> Result<()> {
    let data = json(summary, SOURCE_METADATA_ALLOCATION_BYTES as usize)?;
    ensure!(
        data.len() as u64 == summary.metadata_json_bytes,
        "Source-store summary length differs"
    );
    connection.execute("INSERT INTO metadata(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![KEY,data])?;
    Ok(())
}
fn tables_present(connection: &Connection) -> Result<usize> {
    let mut total = 0;
    for name in TABLES {
        if connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [name],
            |r| r.get::<_, bool>(0),
        )? {
            total += 1;
        }
    }
    Ok(total)
}
fn check_schema(connection: &Connection) -> Result<()> {
    ensure!(
        tables_present(connection)? == 2,
        "Incomplete source-store schema"
    );
    for (table, expected) in [
        (
            TABLES[0],
            vec![
                ("sequence", "INTEGER", 1),
                ("measurement_id", "TEXT", 0),
                ("source_kind", "TEXT", 0),
                ("task_url", "TEXT", 0),
                ("generation", "INTEGER", 0),
                ("source_revision", "TEXT", 0),
                ("expected_document_id", "TEXT", 0),
                ("capture_config_sha256", "TEXT", 0),
                ("initial_catalog_config_sha256", "TEXT", 0),
                ("data", "TEXT", 0),
                ("data_sha256", "TEXT", 0),
            ],
        ),
        (
            TABLES[1],
            vec![
                ("document_id", "TEXT", 1),
                ("measurement_id", "TEXT", 0),
                ("document_sequence", "INTEGER", 0),
                ("data", "TEXT", 0),
                ("data_sha256", "TEXT", 0),
            ],
        ),
    ] {
        let mut stmt = connection.prepare(&format!("PRAGMA table_info({table})"))?;
        let found = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(5)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ensure!(
            found.len() == expected.len()
                && found
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| a.0 == b.0 && a.1 == b.1 && a.2 == b.2),
            "Unknown source-store table shape: {table}"
        );
    }
    Ok(())
}
/// Selected initialization must call this before copying any source observations.
pub(crate) fn ensure_empty(connection: &Connection) -> Result<()> {
    check_schema(connection)?;
    ensure!(
        bounded_metadata(connection, KEY, SOURCE_METADATA_ALLOCATION_BYTES as usize)?.is_none(),
        "Source-store already initialized"
    );
    for table in TABLES {
        ensure!(
            connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))?
                == 0,
            "Source-store table is not empty"
        );
    }
    Ok(())
}

/// Writable-open preflight MUST precede CREATE IF NOT EXISTS. Otherwise a
/// missing initialized empty table could be silently repaired and misrepresented.
/// Existing/read-only states never migrate; fresh two-table creation is atomic.
pub(crate) fn initialize_writable_schema(connection: &Connection) -> Result<()> {
    let marker = bounded_metadata(connection, KEY, SOURCE_METADATA_ALLOCATION_BYTES as usize)?;
    let present = tables_present(connection)?;
    ensure!(
        present == 0 || present == 2,
        "Partial source-store schema must not be repaired"
    );
    if let Some(marker) = marker {
        ensure!(present == 2, "Initialized source-store table is missing");
        let summary: SitemapSourceStoreSummary = serde_json::from_str(&marker)?;
        summary.limits.validate()?;
        ensure!(
            summary.engine == SITEMAP_SOURCE_STORE_ENGINE
                && summary.hash_profile == SITEMAP_SOURCE_STORE_HASH_PROFILE,
            "Unknown initialized source-store engine"
        );
        return check_schema(connection);
    }
    if present == 2 {
        return ensure_empty(connection);
    }
    connection.execute_batch("SAVEPOINT source_store_schema_create")?;
    let result = (|| -> Result<()> {
        connection.execute_batch(SCHEMA)?;
        check_schema(connection)?;
        connection.execute_batch("RELEASE SAVEPOINT source_store_schema_create")?;
        Ok(())
    })();
    if result.is_err() {
        connection.execute_batch("ROLLBACK TO SAVEPOINT source_store_schema_create; RELEASE SAVEPOINT source_store_schema_create").context("Failed source-store schema creation rollback")?;
    }
    result
}
fn seed(kind: &str) -> String {
    digest(serde_json::to_vec(&(SITEMAP_SOURCE_STORE_ENGINE, kind)).expect("fixed seed"))
}
fn roll(previous: &str, kind: &str, data: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"source-store-row-v1\0");
    h.update(previous.as_bytes());
    h.update((kind.len() as u64).to_be_bytes());
    h.update(kind.as_bytes());
    h.update((data.len() as u64).to_be_bytes());
    h.update(data.as_bytes());
    format!("{:x}", h.finalize())
}
fn stage_sha(document: &SitemapSourceDocument) -> Result<String> {
    let mut staged = document.clone();
    staged.sequence = 0;
    Ok(digest(json(&staged, 1024 * 1024)?))
}
fn record_json(
    record: &SitemapSourceStoredMeasurement,
    limits: &SitemapSourceStoreLimits,
) -> Result<String> {
    json(record, limits.max_record_json_bytes)
}
fn commit_json(link: &SitemapSourceCommitAssociation) -> Result<String> {
    json(link, (SOURCE_COMMIT_RESERVE_BYTES - 1) as usize)
}

fn refresh(snapshot: &mut SitemapSourceStoreSnapshot) -> Result<()> {
    let summary = &mut snapshot.summary;
    summary.record_count = u64::try_from(snapshot.records.len())?;
    summary.commit_count = u64::try_from(snapshot.commits.len())?;
    summary.last_sequence = summary.record_count;
    summary.record_json_bytes = 0;
    summary.commit_json_bytes = 0;
    summary.content_sha256 = seed("measurements");
    summary.links_sha256 = seed("commits");
    for record in &snapshot.records {
        let data = record_json(record, &summary.limits)?;
        summary.record_json_bytes = add(summary.record_json_bytes, data.len() as u64)?;
        summary.content_sha256 = roll(&summary.content_sha256, "measurement", &data);
    }
    for link in &snapshot.commits {
        let data = commit_json(link)?;
        summary.commit_json_bytes = add(summary.commit_json_bytes, data.len() as u64)?;
        summary.links_sha256 = roll(&summary.links_sha256, "commit", &data);
    }
    summary.revision = add(
        add(summary.record_count, summary.commit_count)?,
        u64::from(summary.omission.is_some()),
    )?;
    let unjoined = summary
        .record_count
        .checked_sub(summary.commit_count)
        .context("More commits than measurements")?;
    let link_reserve = unjoined
        .checked_mul(SOURCE_COMMIT_RESERVE_BYTES)
        .context("Source-store reserve overflow")?;
    let framing = add(
        FRAME_BYTES,
        add(
            summary.record_count.saturating_sub(1),
            summary.commit_count.saturating_sub(1),
        )?,
    )?;
    let data_bytes = add(
        add(summary.record_json_bytes, summary.commit_json_bytes)?,
        framing,
    )?;
    summary.charged_json_bytes = add(
        add(data_bytes, SOURCE_METADATA_ALLOCATION_BYTES)?,
        link_reserve,
    )?;
    // Allocation is fixed; actual canonical metadata and its unused reserve are
    // reported separately. An omission cannot fail because record capacity is full.
    summary.metadata_json_bytes = 0;
    for _ in 0..16 {
        ensure!(
            summary.metadata_json_bytes <= SOURCE_METADATA_ALLOCATION_BYTES,
            "Source-store metadata allocation exhausted"
        );
        summary.logical_json_bytes = add(data_bytes, summary.metadata_json_bytes)?;
        summary.reserved_json_bytes = add(
            link_reserve,
            SOURCE_METADATA_ALLOCATION_BYTES - summary.metadata_json_bytes,
        )?;
        let size = json(summary, SOURCE_METADATA_ALLOCATION_BYTES as usize)?.len() as u64;
        if size == summary.metadata_json_bytes {
            return Ok(());
        }
        summary.metadata_json_bytes = size;
    }
    bail!("Source-store metadata size did not converge")
}

/// Pure portable validation. SQL joins and exact parent bytes are checked by load.
pub fn validate_snapshot(snapshot: &SitemapSourceStoreSnapshot) -> Result<()> {
    ensure!(
        snapshot.schema_version == 1,
        "Unknown source-store snapshot version"
    );
    let s = &snapshot.summary;
    s.limits.validate()?;
    ensure!(
        s.engine == SITEMAP_SOURCE_STORE_ENGINE
            && s.hash_profile == SITEMAP_SOURCE_STORE_HASH_PROFILE,
        "Unknown source-store profile"
    );
    sha(&s.initial_catalog_config_sha256)?;
    sha(&s.content_sha256)?;
    sha(&s.links_sha256)?;
    ensure!(
        snapshot.records.len() as u64 <= s.limits.max_records
            && snapshot.commits.len() <= snapshot.records.len(),
        "Source-store record cap exceeded"
    );
    if let Some(basis) = &s.inherited {
        sha(&basis.parent_snapshot_sha256)?;
        sha(&basis.parent_catalog_sha256)?;
        ensure!(
            !basis.current_discovery_executed,
            "Inherited source captures cannot attest current discovery"
        );
    }
    ensure!(
        s.omission.is_some() == s.omission_revision.is_some(),
        "Source-store omission marker is incomplete"
    );
    if let Some(revision) = s.omission_revision {
        ensure!(
            revision > 0 && revision <= s.revision,
            "Invalid source-store omission revision"
        );
    }
    let mut by_id = BTreeMap::new();
    for (ordinal, record) in snapshot.records.iter().enumerate() {
        ensure!(
            record.schema_version == 1
                && record.engine == SITEMAP_SOURCE_STORE_ENGINE
                && record.sequence == ordinal as u64 + 1,
            "Source-store record sequence/profile differs"
        );
        ensure!(
            record.initial_catalog_config_sha256 == s.initial_catalog_config_sha256,
            "Source-store initial anchor differs"
        );
        sha(&record.staged_document_sha256)?;
        crate::validate_sitemap_source_measurement(
            &record.measurement,
            &SitemapSourceMeasurementLimits::default(),
        )?;
        ensure!(
            by_id
                .insert(record.measurement.measurement_id.as_str(), record)
                .is_none(),
            "Duplicate source measurement"
        );
    }
    let mut documents = BTreeSet::new();
    let mut measurements = BTreeSet::new();
    let mut previous = 0;
    for link in &snapshot.commits {
        ensure!(
            link.schema_version == 1
                && link.engine == SITEMAP_SOURCE_STORE_ENGINE
                && link.document_sequence > previous,
            "Source commit sequence/profile differs"
        );
        previous = link.document_sequence;
        sha(&link.measurement_envelope_sha256)?;
        sha(&link.document_payload_sha256)?;
        ensure!(
            documents.insert(&link.document_id) && measurements.insert(&link.measurement_id),
            "Duplicate source commit association"
        );
        let record = by_id
            .get(link.measurement_id.as_str())
            .context("Commit has no measurement")?;
        ensure!(
            record
                .measurement
                .payload
                .binding
                .expected_source_observation_id
                == link.document_id
                && digest(record_json(record, &s.limits)?) == link.measurement_envelope_sha256,
            "Commit measurement binding differs"
        );
    }
    let mut rebuilt = snapshot.clone();
    refresh(&mut rebuilt)?;
    ensure!(
        rebuilt.summary == snapshot.summary,
        "Source-store summary/canonical hashes or byte accounting differ"
    );
    ensure!(
        s.charged_json_bytes <= s.limits.max_catalog_json_bytes as u64,
        "Source-store total byte cap exceeded"
    );
    Ok(())
}

/// Portable counterpart of actual SQL document joins. Capture retention is not a link.
pub fn validate_catalog_binding(
    snapshot: &SitemapSourceStoreSnapshot,
    catalog: &SitemapCatalog,
) -> Result<()> {
    validate_snapshot(snapshot)?;
    crate::sitemap_ledger::validate_catalog(catalog)?;
    ensure!(
        snapshot.summary.initial_catalog_config_sha256 == catalog.summary.config_sha256,
        "Source-store/legacy initial config differs"
    );
    ensure!(
        !catalog.summary.legacy_history_unknown || snapshot.summary.legacy_history_unknown,
        "Source-store cannot narrow legacy unknown history"
    );
    ensure!(
        snapshot.summary.inherited.is_some() == catalog.inherited.is_some(),
        "Source-store/legacy inherited scope differs"
    );
    if let (Some(source), Some(legacy)) = (&snapshot.summary.inherited, &catalog.inherited) {
        ensure!(
            source.parent_snapshot_sha256 == legacy.parent_snapshot_sha256,
            "Source-store/legacy parent differs"
        );
    }
    let rows = catalog
        .rows
        .as_ref()
        .context("Source-store needs verifiable legacy rows")?;
    let documents = rows
        .iter()
        .filter_map(|row| match row {
            SitemapLedgerRow::Document(doc) => Some((doc.observation_id.as_str(), (doc, row))),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let records = snapshot
        .records
        .iter()
        .map(|r| (r.measurement.measurement_id.as_str(), r))
        .collect::<BTreeMap<_, _>>();
    for link in &snapshot.commits {
        let (doc, row) = documents
            .get(link.document_id.as_str())
            .context("Measurement commit has no actual legacy document")?;
        let record = records
            .get(link.measurement_id.as_str())
            .context("Measurement commit record missing")?;
        ensure!(
            doc.sequence == link.document_sequence
                && digest(json(row, 1024 * 1024)?) == link.document_payload_sha256
                && stage_sha(doc)? == record.staged_document_sha256,
            "Measurement commit actual document seal differs"
        );
        bind_record_document(&record.measurement, doc)?;
    }
    Ok(())
}

pub fn snapshot_context(snapshot: Option<&SitemapSourceStoreSnapshot>) -> Result<String> {
    if let Some(snapshot) = snapshot {
        validate_snapshot(snapshot)?;
        Ok(digest(serde_json::to_vec(&(
            SITEMAP_SOURCE_STORE_ENGINE,
            "present",
            &snapshot.summary,
        ))?))
    } else {
        Ok(digest(serde_json::to_vec(&(
            SITEMAP_SOURCE_STORE_ENGINE,
            "historical-unknown",
        ))?))
    }
}

fn actual_config(connection: &Connection) -> Result<(String, crate::CrawlConfig)> {
    let bytes: Vec<u8> = {
        let len: i64 = connection.query_row(
            "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key='config'",
            [],
            |r| r.get(0),
        )?;
        ensure!(
            (1..=67_108_864).contains(&len),
            "Source-store saved config exceeds bounded input"
        );
        let bytes: Vec<u8> = connection.query_row(
            "SELECT CAST(value AS BLOB) FROM metadata WHERE key='config'",
            [],
            |r| r.get(0),
        )?;
        ensure!(
            bytes.len() as i64 == len,
            "Source-store saved config length differs"
        );
        bytes
    };
    Ok((digest(&bytes), serde_json::from_slice(&bytes)?))
}
fn table_exists(connection: &Connection, name: &str) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [name],
        |r| r.get(0),
    )?)
}
fn full_active(connection: &Connection, generation: u64) -> Result<(String, crate::CrawlConfig)> {
    let run: crate::CrawlRunState =
        metadata(connection, "run_state", 4096)?.context("Missing source-store active run")?;
    ensure!(
        run.generation == generation && run.outcome.is_none(),
        "Stale/terminal source-store generation"
    );
    let plan: crate::RunPlan = metadata(connection, "run_plan", 67_108_864)?.unwrap_or_default();
    ensure!(
        matches!(plan, crate::RunPlan::FullCrawl),
        "Selected recrawl cannot record fresh source captures"
    );
    ensure!(
        bounded_metadata(connection, "operation_kind", 64)?.is_none(),
        "Full source capture has selected operation marker"
    );
    if table_exists(connection, "recrawl_base")? {
        ensure!(
            !connection.query_row("SELECT EXISTS(SELECT 1 FROM recrawl_base)", [], |r| r
                .get::<_, bool>(0))?,
            "Full source capture has an immutable selected parent"
        );
    }
    actual_config(connection)
}
fn source_identity(document: &SitemapSourceDocument) -> Result<()> {
    ensure!(
        document.kind == SitemapSourceKind::SitemapXml,
        "Only original XML sources have source measurements"
    );
    ensure!(
        document.observation_id
            == crate::sitemap_ledger::source_observation_id(
                document.kind,
                &document.task_url,
                document.generation,
                &document.source_revision
            )?,
        "Staged source observation ID differs"
    );
    Ok(())
}
fn bind_record_document(
    measurement: &SitemapSourceMeasurement,
    document: &SitemapSourceDocument,
) -> Result<()> {
    crate::validate_sitemap_source_measurement(
        measurement,
        &SitemapSourceMeasurementLimits::default(),
    )?;
    source_identity(document)?;
    let p = &measurement.payload;
    let b = &p.binding;
    ensure!(
        b.source_kind == document.kind
            && b.requested_task_url == document.task_url
            && b.run_generation == document.generation
            && b.source_revision == document.source_revision
            && b.expected_source_observation_id == document.observation_id,
        "Measurement/staged source binding differs"
    );
    let t = document
        .transport
        .as_ref()
        .context("Measured source has no legacy transport")?;
    let m = &p.transport;
    ensure!(
        m.request_started == t.request_started
            && m.requested_url == t.requested_url
            && m.final_url == t.final_url
            && m.initial_status == t.initial_status
            && m.final_status == t.final_status
            && m.blocked_by_robots == t.blocked_by_robots
            && m.body_complete == t.body_complete
            && m.retained_post_http_prefix_bytes == t.post_http_decoding_bytes
            && m.complete_post_http_entity_sha256 == t.post_http_decoding_sha256,
        "Measurement/legacy transport differs"
    );
    if let Some(hash) = &t.retained_post_http_prefix_sha256 {
        ensure!(
            m.retained_post_http_prefix_sha256.as_ref() == Some(hash),
            "Measurement/legacy prefix SHA differs"
        );
    }
    if let Some(decode) = &document.entity_decode {
        ensure!(
            p.decode.encoding == Some(decode.encoding)
                && p.decode.integrity == crate::SitemapSourceDecodeIntegrity::Complete
                && p.decode.decoded_xml_bytes
                    == crate::SitemapMeasuredValue::exact(decode.decoded_xml_bytes as u64)
                && p.decode.decoded_complete_sha256.as_deref()
                    == Some(decode.decoded_xml_sha256.as_str()),
            "Measurement/legacy decoded XML differs"
        );
    }
    let retained = match &document.parser {
        None => crate::SitemapMembershipRetention::Unobserved,
        Some(parser) if parser.complete && document.retained_rows_complete => {
            crate::SitemapMembershipRetention::Complete
        }
        Some(_) => crate::SitemapMembershipRetention::Incomplete,
    };
    ensure!(
        p.membership_retention == retained,
        "Measurement/legacy parser retention differs"
    );
    Ok(())
}

/// Call in the SAME typed initialization TX, after the actual legacy catalog exists.
pub(crate) fn initialize(
    tx: &Transaction<'_>,
    generation: u64,
    limits: &SitemapSourceStoreLimits,
) -> Result<SitemapSourceStoreSummary> {
    limits.validate()?;
    full_active(tx, generation)?;
    if let Some(existing) = load(tx)? {
        ensure!(
            existing.summary.limits == *limits && existing.summary.inherited.is_none(),
            "Source-store initialization limits/scope changed"
        );
        return Ok(existing.summary);
    }
    ensure_empty(tx)?;
    let catalog = crate::sitemap_ledger::load_catalog(tx)?
        .context("Source-store initialization needs typed legacy catalog")?;
    ensure!(
        catalog.inherited.is_none(),
        "Inherited legacy catalog cannot initialize current source captures"
    );
    let old_done: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM discovery_frontier WHERE done=1)",
        [],
        |r| r.get(0),
    )?;
    let mut snapshot = SitemapSourceStoreSnapshot {
        schema_version: 1,
        summary: SitemapSourceStoreSummary {
            engine: SITEMAP_SOURCE_STORE_ENGINE.into(),
            hash_profile: SITEMAP_SOURCE_STORE_HASH_PROFILE.into(),
            initial_catalog_config_sha256: catalog.summary.config_sha256.clone(),
            limits: limits.clone(),
            revision: 0,
            last_sequence: 0,
            record_count: 0,
            commit_count: 0,
            record_json_bytes: 0,
            commit_json_bytes: 0,
            metadata_json_bytes: 0,
            logical_json_bytes: 0,
            reserved_json_bytes: 0,
            charged_json_bytes: 0,
            content_sha256: seed("measurements"),
            links_sha256: seed("commits"),
            legacy_history_unknown: catalog.summary.legacy_history_unknown
                || catalog.summary.document_rows > 0
                || old_done,
            omission: None,
            omission_revision: None,
            inherited: None,
        },
        records: vec![],
        commits: vec![],
    };
    refresh(&mut snapshot)?;
    validate_snapshot(&snapshot)?;
    save_summary(tx, &snapshot.summary)?;
    Ok(snapshot.summary)
}

fn preflight_rows(connection: &Connection, summary: &SitemapSourceStoreSummary) -> Result<()> {
    check_schema(connection)?;
    let mut bytes = 0u64;
    for (table, max_count, max_size) in [
        (
            TABLES[0],
            summary.limits.max_records,
            summary.limits.max_record_json_bytes as u64,
        ),
        (
            TABLES[1],
            summary.limits.max_records,
            SOURCE_COMMIT_RESERVE_BYTES - 1,
        ),
    ] {
        let (count,total,max):(i64,i64,i64)=connection.query_row(&format!("SELECT count(*),COALESCE(SUM(length(CAST(data AS BLOB))),0),COALESCE(MAX(length(CAST(data AS BLOB))),0) FROM {table}"),[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        ensure!(
            u64::try_from(count)? <= max_count && u64::try_from(max)? <= max_size,
            "Source-store SQL row bounds exceeded"
        );
        bytes = add(bytes, u64::try_from(total)?)?;
    }
    ensure!(
        bytes <= summary.limits.max_catalog_json_bytes as u64,
        "Source-store SQL aggregate byte cap exceeded"
    );
    let invalid:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM sitemap_source_measurements WHERE source_kind!='sitemap' OR length(CAST(task_url AS BLOB))>8192 OR length(CAST(source_revision AS BLOB))>8192 OR length(measurement_id)!=64 OR length(expected_document_id)!=64 OR length(capture_config_sha256)!=64 OR length(initial_catalog_config_sha256)!=64 OR length(data_sha256)!=64)",[],|r|r.get(0))?;
    ensure!(
        !invalid,
        "Oversized/invalid source-store SQL identity columns"
    );
    let invalid:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM sitemap_source_measurement_commits WHERE length(document_id)!=64 OR length(measurement_id)!=64 OR length(data_sha256)!=64)",[],|r|r.get(0))?;
    ensure!(
        !invalid,
        "Oversized/invalid source-store commit SQL columns"
    );
    Ok(())
}
/// Caller MUST use its existing read TX; this function never opens another connection.
pub(crate) fn load(connection: &Connection) -> Result<Option<SitemapSourceStoreSnapshot>> {
    let text = bounded_metadata(connection, KEY, SOURCE_METADATA_ALLOCATION_BYTES as usize)?;
    let Some(text) = text else {
        match tables_present(connection)? {
            0 => return Ok(None),
            2 => {
                ensure_empty(connection)?;
                return Ok(None);
            }
            _ => bail!("Partial source-store schema without marker"),
        }
    };
    let summary: SitemapSourceStoreSummary = serde_json::from_str(&text)?;
    summary.limits.validate()?;
    ensure!(
        json(&summary, SOURCE_METADATA_ALLOCATION_BYTES as usize)? == text,
        "Noncanonical source-store summary"
    );
    preflight_rows(connection, &summary)?;
    let mut records = Vec::new();
    let mut stmt=connection.prepare("SELECT sequence,measurement_id,source_kind,task_url,generation,source_revision,expected_document_id,capture_config_sha256,initial_catalog_config_sha256,data,data_sha256 FROM sitemap_source_measurements ORDER BY sequence")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let data: String = row.get(9)?;
        let data_sha: String = row.get(10)?;
        ensure!(
            data.len() <= summary.limits.max_record_json_bytes && digest(&data) == data_sha,
            "Source measurement envelope SHA/size differs"
        );
        let record: SitemapSourceStoredMeasurement = serde_json::from_str(&data)?;
        let b = &record.measurement.payload.binding;
        ensure!(
            record_json(&record, &summary.limits)? == data
                && row.get::<_, i64>(0)? == i64::try_from(record.sequence)?
                && row.get::<_, String>(1)? == record.measurement.measurement_id
                && row.get::<_, String>(2)? == "sitemap"
                && row.get::<_, String>(3)? == b.requested_task_url
                && row.get::<_, i64>(4)? == i64::try_from(b.run_generation)?
                && row.get::<_, String>(5)? == b.source_revision
                && row.get::<_, String>(6)? == b.expected_source_observation_id
                && row.get::<_, String>(7)? == b.crawl_config_sha256
                && row.get::<_, String>(8)? == record.initial_catalog_config_sha256,
            "Source measurement SQL columns/canonical envelope differ"
        );
        records.push(record);
    }
    drop(rows);
    drop(stmt);
    let mut commits = Vec::new();
    let mut stmt=connection.prepare("SELECT document_id,measurement_id,document_sequence,data,data_sha256 FROM sitemap_source_measurement_commits ORDER BY document_sequence")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let data: String = row.get(3)?;
        let hash: String = row.get(4)?;
        ensure!(
            data.len() < (SOURCE_COMMIT_RESERVE_BYTES as usize) && digest(&data) == hash,
            "Source commit envelope SHA/size differs"
        );
        let link: SitemapSourceCommitAssociation = serde_json::from_str(&data)?;
        ensure!(
            commit_json(&link)? == data
                && row.get::<_, String>(0)? == link.document_id
                && row.get::<_, String>(1)? == link.measurement_id
                && row.get::<_, i64>(2)? == i64::try_from(link.document_sequence)?,
            "Source commit SQL columns/canonical envelope differ"
        );
        commits.push(link);
    }
    let snapshot = SitemapSourceStoreSnapshot {
        schema_version: 1,
        summary,
        records,
        commits,
    };
    validate_snapshot(&snapshot)?;
    let catalog = crate::sitemap_ledger::load_catalog(connection)?
        .context("Source-store marker has no legacy catalog")?;
    validate_catalog_binding(&snapshot, &catalog)?;
    if snapshot.summary.inherited.is_some() {
        verify_inherited_anchor(connection, &snapshot)?;
    }
    Ok(Some(snapshot))
}

/// Pure projection for in-memory selected reports. Captured payloads are unchanged.
pub fn project_inherited_snapshot(
    parent: &SitemapSourceStoreSnapshot,
    parent_snapshot_sha256: &str,
) -> Result<SitemapSourceStoreSnapshot> {
    sha(parent_snapshot_sha256)?;
    validate_snapshot(parent)?;
    let mut child = parent.clone();
    child.summary.inherited = Some(SitemapSourceStoreInheritedBasis {
        parent_snapshot_sha256: parent_snapshot_sha256.into(),
        parent_catalog_sha256: digest(json(parent, MAX_SOURCE_STORE_BYTES)?),
        current_discovery_executed: false,
    });
    refresh(&mut child)?;
    validate_snapshot(&child)?;
    Ok(child)
}
#[derive(Deserialize)]
struct ParentEnvelope {
    #[serde(default)]
    sitemap_source_measurements: Option<SitemapSourceStoreSnapshot>,
}
fn verify_inherited_anchor(
    connection: &Connection,
    snapshot: &SitemapSourceStoreSnapshot,
) -> Result<()> {
    let inherited = snapshot
        .summary
        .inherited
        .as_ref()
        .context("Missing source-store inherited basis")?;
    let plan: crate::RunPlan = metadata(connection, "run_plan", 67_108_864)?
        .context("Inherited sources need a selected run plan")?;
    let crate::RunPlan::SelectedRecrawl(plan) = plan else {
        bail!("Inherited sources need selected operation")
    };
    ensure!(
        bounded_metadata(connection, "operation_kind", 64)?.as_deref() == Some("selected_recrawl")
            && plan.parent.snapshot_sha256 == inherited.parent_snapshot_sha256,
        "Source-store selected parent/operation differs"
    );
    let len: i64 = connection.query_row(
        "SELECT length(snapshot) FROM recrawl_base WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    ensure!(
        (1..=268_435_456).contains(&len),
        "Source-store parent snapshot is oversized"
    );
    let bytes: Vec<u8> =
        connection.query_row("SELECT snapshot FROM recrawl_base WHERE id=1", [], |r| {
            r.get(0)
        })?;
    ensure!(
        bytes.len() as i64 == len && digest(&bytes) == inherited.parent_snapshot_sha256,
        "Source-store exact parent BLOB SHA differs"
    );
    let config = actual_config(connection)?.1;
    crate::recrawl::validate_saved_plan(&plan, &bytes, &config)?;
    let parent: ParentEnvelope = serde_json::from_slice(&bytes)?;
    let parent = parent
        .sitemap_source_measurements
        .context("Selected parent has no captured source sidecar")?;
    ensure!(
        project_inherited_snapshot(&parent, &inherited.parent_snapshot_sha256)? == *snapshot,
        "Source-store inherited catalog differs from exact parent"
    );
    Ok(())
}

fn insert_record(
    connection: &Connection,
    record: &SitemapSourceStoredMeasurement,
    limits: &SitemapSourceStoreLimits,
) -> Result<()> {
    let data = record_json(record, limits)?;
    let b = &record.measurement.payload.binding;
    connection.execute("INSERT INTO sitemap_source_measurements(sequence,measurement_id,source_kind,task_url,generation,source_revision,expected_document_id,capture_config_sha256,initial_catalog_config_sha256,data,data_sha256) VALUES(?1,?2,'sitemap',?3,?4,?5,?6,?7,?8,?9,?10)",params![i64::try_from(record.sequence)?,record.measurement.measurement_id,b.requested_task_url,i64::try_from(b.run_generation)?,b.source_revision,b.expected_source_observation_id,b.crawl_config_sha256,record.initial_catalog_config_sha256,data,digest(&data)])?;
    Ok(())
}
fn insert_link(connection: &Connection, link: &SitemapSourceCommitAssociation) -> Result<()> {
    let data = commit_json(link)?;
    connection.execute("INSERT INTO sitemap_source_measurement_commits(document_id,measurement_id,document_sequence,data,data_sha256) VALUES(?1,?2,?3,?4,?5)",params![link.document_id,link.measurement_id,i64::try_from(link.document_sequence)?,data,digest(&data)])?;
    Ok(())
}
pub(crate) fn inherit(
    tx: &Transaction<'_>,
    parent: &SitemapSourceStoreSnapshot,
    parent_snapshot_sha256: &str,
) -> Result<SitemapSourceStoreSummary> {
    ensure_empty(tx)?;
    let child = project_inherited_snapshot(parent, parent_snapshot_sha256)?;
    verify_inherited_anchor(tx, &child)?;
    let legacy = crate::sitemap_ledger::load_catalog(tx)?
        .context("Source-store inheritance needs inherited legacy catalog first")?;
    validate_catalog_binding(&child, &legacy)?;
    for record in &child.records {
        insert_record(tx, record, &child.summary.limits)?;
    }
    for link in &child.commits {
        insert_link(tx, link)?;
    }
    save_summary(tx, &child.summary)?;
    Ok(child.summary)
}

#[derive(Debug, Clone)]
struct ExpectedCapture {
    generation: u64,
    capture_config_sha256: String,
    initial_catalog_config_sha256: String,
    task_url: String,
    document_id: String,
    staged_document_sha256: String,
    body_limit_bytes: u64,
    parser_limits: crate::SitemapParserLimits,
}
#[derive(Debug, Clone)]
enum ReceiptProof {
    Persisted {
        measurement_id: String,
        envelope_sha256: String,
    },
    Omitted {
        marker_revision: u64,
        reason: SitemapSourceStoreOmission,
    },
}
/// Unforgeable in-process proof. A durable record is not a completed source.
/// No Clone/Deserialize: caller must commit A and confirm before passing to B.
#[derive(Debug)]
pub(crate) struct SourceMeasurementReceipt {
    expected: ExpectedCapture,
    proof: ReceiptProof,
    a_commit_observed: bool,
}
impl SourceMeasurementReceipt {
    pub(crate) fn is_persisted(&self) -> bool {
        matches!(self.proof, ReceiptProof::Persisted { .. })
    }
    pub(crate) fn measurement_id(&self) -> Option<&str> {
        match &self.proof {
            ReceiptProof::Persisted { measurement_id, .. } => Some(measurement_id),
            _ => None,
        }
    }
    /// First durable fixed omission reason for this store; not a per-attempt log.
    pub(crate) fn omission_reason(&self) -> Option<SitemapSourceStoreOmission> {
        match self.proof {
            ReceiptProof::Omitted { reason, .. } => Some(reason),
            _ => None,
        }
    }
    pub(crate) fn after_a_commit(mut self, connection: &Connection) -> Result<Self> {
        ensure!(
            connection.is_autocommit(),
            "A receipt must be confirmed after its outer transaction committed"
        );
        ensure!(!self.a_commit_observed, "A receipt already confirmed");
        // One bounded read transaction: durable rows and marker must agree, even
        // if another connection begins a new generation concurrently afterwards.
        let read = connection.unchecked_transaction()?;
        let snapshot = load(&read)?.context("A receipt has no durable source-store snapshot")?;
        verify_receipt_snapshot(&self.expected, &self.proof, &snapshot)?;
        read.commit()?;
        self.a_commit_observed = true;
        Ok(self)
    }
}
fn guarded_stage(
    tx: &Transaction<'_>,
    document: &SitemapSourceDocument,
    config_sha256: &str,
) -> Result<ExpectedCapture> {
    ensure!(
        document.sequence == 0,
        "A/B source must retain its original sequence-zero staged document"
    );
    source_identity(document)?;
    sha(config_sha256)?;
    let (actual_sha, config) = full_active(tx, document.generation)?;
    ensure!(
        actual_sha == config_sha256,
        "Capture saved-config bytes changed"
    );
    // Reuse the actual existing generation/pending guard; never invent a separate
    // admission model and never carry this A token over a transaction boundary.
    drop(crate::sitemap_ledger::guard_pending_source(
        tx,
        document.generation,
        document.kind,
        &document.task_url,
    )?);
    let legacy = crate::sitemap_ledger::load_catalog(tx)?
        .context("Capture needs initialized typed catalog")?;
    ensure!(
        legacy.inherited.is_none(),
        "Inherited history cannot capture current source XML"
    );
    Ok(ExpectedCapture {
        generation: document.generation,
        capture_config_sha256: config_sha256.into(),
        initial_catalog_config_sha256: legacy.summary.config_sha256,
        task_url: document.task_url.clone(),
        document_id: document.observation_id.clone(),
        staged_document_sha256: stage_sha(document)?,
        body_limit_bytes: u64::try_from(config.sitemap_source_body_limit()?)?,
        parser_limits: config.sitemap_source_parser_limits()?,
    })
}
fn verify_receipt_snapshot(
    expected: &ExpectedCapture,
    proof: &ReceiptProof,
    snapshot: &SitemapSourceStoreSnapshot,
) -> Result<()> {
    ensure!(
        snapshot.summary.inherited.is_none()
            && snapshot.summary.initial_catalog_config_sha256
                == expected.initial_catalog_config_sha256,
        "Receipt source-store anchor/scope differs"
    );
    match proof {
        ReceiptProof::Persisted {
            measurement_id,
            envelope_sha256,
        } => {
            let record = snapshot
                .records
                .iter()
                .find(|r| r.measurement.measurement_id == *measurement_id)
                .context("Receipt measurement missing")?;
            let b = &record.measurement.payload.binding;
            ensure!(
                digest(record_json(record, &snapshot.summary.limits)?) == *envelope_sha256
                    && record.staged_document_sha256 == expected.staged_document_sha256
                    && b.crawl_config_sha256 == expected.capture_config_sha256
                    && b.run_generation == expected.generation
                    && b.requested_task_url == expected.task_url
                    && b.expected_source_observation_id == expected.document_id,
                "Receipt immutable capture binding differs"
            );
        }
        ReceiptProof::Omitted {
            marker_revision,
            reason,
        } => ensure!(
            snapshot.summary.omission_revision == Some(*marker_revision)
                && snapshot.summary.omission == Some(*reason),
            "Receipt durable omission marker missing/different"
        ),
    }
    Ok(())
}
fn omitted_receipt(
    tx: &Transaction<'_>,
    mut snapshot: SitemapSourceStoreSnapshot,
    expected: ExpectedCapture,
    reason: SitemapSourceStoreOmission,
) -> Result<SourceMeasurementReceipt> {
    if snapshot.summary.omission.is_none() {
        snapshot.summary.omission = Some(reason);
        snapshot.summary.omission_revision = Some(add(snapshot.summary.revision, 1)?);
        refresh(&mut snapshot)?;
        validate_snapshot(&snapshot)?;
        save_summary(tx, &snapshot.summary)?;
    }
    Ok(SourceMeasurementReceipt {
        expected,
        proof: ReceiptProof::Omitted {
            marker_revision: snapshot
                .summary
                .omission_revision
                .context("Missing omission revision")?,
            reason: snapshot
                .summary
                .omission
                .context("Missing omission marker")?,
        },
        a_commit_observed: false,
    })
}
/// A writes only a measurement or fixed omission, never any membership/frontier row.
pub(crate) fn record_a(
    tx: &Transaction<'_>,
    document: &SitemapSourceDocument,
    measurement: &SitemapSourceMeasurement,
) -> Result<SourceMeasurementReceipt> {
    let expected = guarded_stage(
        tx,
        document,
        &measurement.payload.binding.crawl_config_sha256,
    )?;
    bind_record_document(measurement, document)?;
    let bounds = &measurement.payload.bounds;
    let parser = &expected.parser_limits;
    ensure!(
        bounds.max_retained_post_http_bytes == expected.body_limit_bytes
            && bounds.max_retained_xml_bytes == parser.max_xml_bytes as u64
            && bounds.max_xml_events == parser.max_xml_events as u64
            && bounds.max_xml_depth == parser.max_xml_depth as u64
            && bounds.max_field_utf8_bytes == parser.max_field_utf8_bytes as u64
            && bounds.max_retained_entries == parser.max_entries as u64
            && bounds.max_retained_location_occurrences == parser.max_location_occurrences as u64
            && bounds.max_retained_text_utf8_bytes == parser.max_retained_text_utf8_bytes as u64,
        "Measurement bounds differ from actual saved-config source processing profile"
    );
    let mut snapshot = load(tx)?.context("Source-store A requires explicit initialization")?;
    ensure!(
        snapshot.summary.initial_catalog_config_sha256 == expected.initial_catalog_config_sha256
            && snapshot.summary.inherited.is_none(),
        "Source-store A anchor differs"
    );
    // Dedupe comes strictly AFTER fresh generation/config/pending/doc guards.
    if let Some(record) = snapshot
        .records
        .iter()
        .find(|r| r.measurement.measurement_id == measurement.measurement_id)
    {
        ensure!(
            record.measurement == *measurement
                && record.staged_document_sha256 == expected.staged_document_sha256,
            "Same measurement ID has a different capture envelope"
        );
        return Ok(SourceMeasurementReceipt {
            expected,
            proof: ReceiptProof::Persisted {
                measurement_id: measurement.measurement_id.clone(),
                envelope_sha256: digest(record_json(record, &snapshot.summary.limits)?),
            },
            a_commit_observed: false,
        });
    }
    if snapshot.summary.record_count >= snapshot.summary.limits.max_records {
        return omitted_receipt(
            tx,
            snapshot,
            expected,
            SitemapSourceStoreOmission::StorageBudgetExceeded,
        );
    }
    let record = SitemapSourceStoredMeasurement {
        schema_version: 1,
        engine: SITEMAP_SOURCE_STORE_ENGINE.into(),
        sequence: add(snapshot.summary.last_sequence, 1)?,
        initial_catalog_config_sha256: expected.initial_catalog_config_sha256.clone(),
        staged_document_sha256: expected.staged_document_sha256.clone(),
        measurement: measurement.clone(),
    };
    let Some(data) = limited_json(&record, snapshot.summary.limits.max_record_json_bytes)? else {
        return omitted_receipt(
            tx,
            snapshot,
            expected,
            SitemapSourceStoreOmission::StorageBudgetExceeded,
        );
    };
    let envelope_sha256 = digest(&data);
    snapshot.records.push(record);
    refresh(&mut snapshot)?;
    if snapshot.summary.charged_json_bytes > snapshot.summary.limits.max_catalog_json_bytes as u64 {
        snapshot.records.pop();
        refresh(&mut snapshot)?;
        return omitted_receipt(
            tx,
            snapshot,
            expected,
            SitemapSourceStoreOmission::StorageBudgetExceeded,
        );
    }
    validate_snapshot(&snapshot)?;
    insert_record(
        tx,
        snapshot.records.last().context("New record missing")?,
        &snapshot.summary.limits,
    )?;
    save_summary(tx, &snapshot.summary)?;
    Ok(SourceMeasurementReceipt {
        expected,
        proof: ReceiptProof::Persisted {
            measurement_id: measurement.measurement_id.clone(),
            envelope_sha256,
        },
        a_commit_observed: false,
    })
}
/// Legitimate capture unavailability, with a real pre-fetch config binding. This
/// does not accept inconsistent DTOs or relabel a failed capture as a budget hit.
pub(crate) fn record_unavailable_a(
    tx: &Transaction<'_>,
    document: &SitemapSourceDocument,
    capture_config_sha256: &str,
) -> Result<SourceMeasurementReceipt> {
    let expected = guarded_stage(tx, document, capture_config_sha256)?;
    let snapshot = load(tx)?.context("Capture omission needs initialized source-store")?;
    ensure!(
        snapshot.summary.initial_catalog_config_sha256 == expected.initial_catalog_config_sha256
            && snapshot.summary.inherited.is_none(),
        "Capture omission anchor differs"
    );
    omitted_receipt(
        tx,
        snapshot,
        expected,
        SitemapSourceStoreOmission::CaptureUnavailable,
    )
}

pub(crate) struct SourceMeasurementCommitGuard<'tx> {
    connection: &'tx Connection,
    pending: Option<crate::sitemap_ledger::PendingSitemapSource<'tx>>,
    expected: ExpectedCapture,
    proof: ReceiptProof,
}
impl<'tx> SourceMeasurementCommitGuard<'tx> {
    pub(crate) fn take_pending(
        &mut self,
    ) -> Result<crate::sitemap_ledger::PendingSitemapSource<'tx>> {
        self.pending
            .take()
            .context("Measured B pending token already consumed")
    }
}
/// First admission-sensitive B operation. The actual fresh token goes to append_source.
pub(crate) fn verify_b_before_admission<'tx>(
    tx: &'tx Transaction<'_>,
    receipt: &SourceMeasurementReceipt,
    document: &SitemapSourceDocument,
) -> Result<SourceMeasurementCommitGuard<'tx>> {
    ensure!(
        receipt.a_commit_observed,
        "B cannot use an uncommitted/unconfirmed A receipt"
    );
    ensure!(
        document.sequence == 0
            && document.generation == receipt.expected.generation
            && document.task_url == receipt.expected.task_url
            && document.observation_id == receipt.expected.document_id
            && stage_sha(document)? == receipt.expected.staged_document_sha256,
        "B staged document differs from A"
    );
    source_identity(document)?;
    let pending = crate::sitemap_ledger::guard_pending_source(
        tx,
        document.generation,
        document.kind,
        &document.task_url,
    )?;
    ensure!(
        full_active(tx, document.generation)?.0 == receipt.expected.capture_config_sha256,
        "B current saved-config bytes differ from capture"
    );
    let snapshot = load(tx)?.context("B source-store missing")?;
    verify_receipt_snapshot(&receipt.expected, &receipt.proof, &snapshot)?;
    Ok(SourceMeasurementCommitGuard {
        connection: tx,
        pending: Some(pending),
        expected: receipt.expected.clone(),
        proof: receipt.proof.clone(),
    })
}
fn actual_document(
    connection: &Connection,
    expected: &ExpectedCapture,
) -> Result<(SitemapSourceDocument, String)> {
    let (len, stored_hash): (i64, String) = connection.query_row(
        "SELECT length(CAST(data AS BLOB)),data_sha256 FROM sitemap_documents WHERE row_id=?1",
        [&expected.document_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    ensure!(
        (1..=1_048_576).contains(&len),
        "Actual committed source document exceeds legacy row cap"
    );
    let data: String = connection.query_row(
        "SELECT data FROM sitemap_documents WHERE row_id=?1",
        [&expected.document_id],
        |r| r.get(0),
    )?;
    ensure!(
        data.len() as i64 == len && digest(&data) == stored_hash,
        "Actual committed source payload SHA differs"
    );
    let row: SitemapLedgerRow = serde_json::from_str(&data)?;
    ensure!(
        json(&row, 1024 * 1024)? == data,
        "Actual source row is not canonical"
    );
    let SitemapLedgerRow::Document(document) = row else {
        bail!("Source association has non-document row")
    };
    ensure!(
        document.sequence > 0
            && document.observation_id == expected.document_id
            && stage_sha(&document)? == expected.staged_document_sha256,
        "Actual committed source differs from staged A document"
    );
    Ok((*document, stored_hash))
}
/// AFTER actual append_source in the same B TX. done MUST now be1; no pending recheck.
pub(crate) fn link_b_after_append(
    tx: &Transaction<'_>,
    guard: SourceMeasurementCommitGuard<'_>,
) -> Result<SitemapSourceStoreSummary> {
    ensure!(
        std::ptr::eq(guard.connection, &**tx) && guard.pending.is_none(),
        "B association needs its same-TX consumed pending guard"
    );
    ensure!(
        full_active(tx, guard.expected.generation)?.0 == guard.expected.capture_config_sha256,
        "B config/generation changed before association"
    );
    let done: Option<i64> = tx
        .query_row(
            "SELECT done FROM discovery_frontier WHERE kind='sitemap' AND url=?1",
            [&guard.expected.task_url],
            |r| r.get(0),
        )
        .optional()?;
    ensure!(
        done == Some(1),
        "B association requires actual successful source completion"
    );
    let mut snapshot = load(tx)?.context("B association source-store missing")?;
    verify_receipt_snapshot(&guard.expected, &guard.proof, &snapshot)?;
    let (document, document_payload_sha256) = actual_document(tx, &guard.expected)?;
    match guard.proof {
        ReceiptProof::Omitted { .. } => Ok(snapshot.summary),
        ReceiptProof::Persisted {
            measurement_id,
            envelope_sha256,
        } => {
            ensure!(
                snapshot
                    .commits
                    .last()
                    .is_none_or(|link| link.document_sequence < document.sequence),
                "B source association is not append ordered"
            );
            ensure!(
                !snapshot
                    .commits
                    .iter()
                    .any(|link| link.document_id == document.observation_id
                        || link.measurement_id == measurement_id),
                "B source association already exists"
            );
            let link = SitemapSourceCommitAssociation {
                schema_version: 1,
                engine: SITEMAP_SOURCE_STORE_ENGINE.into(),
                measurement_id,
                measurement_envelope_sha256: envelope_sha256,
                document_id: document.observation_id,
                document_sequence: document.sequence,
                document_payload_sha256,
            };
            commit_json(&link)?;
            snapshot.commits.push(link.clone());
            refresh(&mut snapshot)?;
            validate_snapshot(&snapshot)?;
            // Every possible link byte (plus delimiter) was reserved by A. No
            // new budget allowance is introduced after legacy admission.
            ensure!(
                snapshot.summary.charged_json_bytes
                    <= snapshot.summary.limits.max_catalog_json_bytes as u64,
                "Reserved B association capacity was insufficient"
            );
            insert_link(tx, &link)?;
            save_summary(tx, &snapshot.summary)?;
            Ok(snapshot.summary)
        }
    }
}

#[cfg(test)]
#[path = "sitemap_source_store_tests.rs"]
mod tests;
