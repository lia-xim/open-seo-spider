//! Local opt-in archive foundation: measurement A -> archive A_body -> membership B.
//! No public HTTP/MCP/UI/bundle, selected copy, wire capture or authenticity claim.
use crate::{
    CrawlConfig, SitemapMeasuredPrecision, SitemapSourceDecodeIntegrity,
    SitemapSourceStoredMeasurement,
};
use anyhow::{Context, Result, bail, ensure};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use sha2::{Digest, Sha256};
#[path = "sitemap_source_archive_models.rs"]
mod models;
pub(crate) use models::SourceBodyCandidate;
use models::{ARCHIVE_STATE_RESERVE, MAX_ARCHIVE_CAPTURE_BYTES, MAX_ARCHIVE_METADATA_BYTES};
pub use models::{
    MAX_SITEMAP_ARCHIVE_BODY_BYTES, SITEMAP_SOURCE_ARCHIVE_ENGINE, SitemapArchiveOmission,
    SitemapArchivedBody, SitemapArchivedCapture, SitemapBodyExtent, SitemapBodyRepresentation,
    SitemapSourceArchiveConfig, SitemapSourceArchiveInspection, SitemapSourceArchiveSnapshot,
    SitemapSourceArchiveSummary,
};
#[path = "sitemap_source_archive_fs.rs"]
pub(crate) mod files;
const SCHEMA: &str = include_str!("sitemap_source_archive_schema.sql");
const STATE: &str = "sitemap_body_archive_state";
const RECORDS: &str = "sitemap_body_archive_captures";
pub(crate) fn digest(b: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(b.as_ref()))
}
pub(crate) fn valid_sha(s: &str) -> Result<()> {
    ensure!(
        s.len() == 64
            && s.bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "Invalid archive digest"
    );
    Ok(())
}
fn canonical<T: serde::Serialize>(value: &T, limit: usize) -> Result<String> {
    let data = serde_json::to_string(value)?;
    ensure!(data.len() <= limit, "Archive metadata byte budget exceeded");
    Ok(data)
}
fn roll(previous: &str, data: &str) -> String {
    digest(format!(
        "sitemap-body-catalog-v1\n{previous}\n{}",
        digest(data)
    ))
}
fn seed() -> String {
    digest(b"sitemap-body-catalog-v1")
}
fn table_count(c: &Connection) -> Result<i64> {
    Ok(c.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('sitemap_body_archive_state','sitemap_body_archive_captures')",[],|r|r.get(0))?)
}
// This first archive format intentionally requires UTF8 SQLite storage. Existing
// source/Store config hashes bind actual database BLOB bytes; do not silently
// reinterpret those authorities or claim the raw ledger's broader encoding set.
fn require_utf8_storage(c: &Connection) -> Result<()> {
    let encoding: String = c.query_row("PRAGMA encoding", [], |r| r.get(0))?;
    ensure!(
        encoding == "UTF-8",
        "Source body archive requires UTF-8 SQLite storage"
    );
    Ok(())
}
pub(crate) fn require_storage_for_config(c: &Connection, config: &CrawlConfig) -> Result<()> {
    if config.sitemap_source_archive.is_some() {
        require_utf8_storage(c)?;
    }
    Ok(())
}
pub(crate) fn require_storage_for_existing(c: &Connection) -> Result<()> {
    if table_count(c)? != 0 {
        require_utf8_storage(c)?;
    }
    Ok(())
}
fn text(c: &Connection, table: &str, key: &str, value: &str, cap: usize) -> Result<Option<String>> {
    ensure!(
        [STATE, RECORDS, "sitemap_source_measurements"].contains(&table),
        "Internal archive table"
    );
    ensure!(
        ["id", "measurement_id"].contains(&key),
        "Internal archive key"
    );
    // Both payload and identity columns are bounded before Rust String allocation.
    let sql = format!(
        "SELECT typeof(data),length(CAST(data AS BLOB)),typeof(data_sha256),length(CAST(data_sha256 AS BLOB)) FROM {table} WHERE {key}=?1"
    );
    let pre: Option<(String, i64, String, i64)> = c
        .query_row(&sql, [value], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .optional()?;
    let Some((data_type, length, hash_type, hash_length)) = pre else {
        return Ok(None);
    };
    ensure!(
        data_type == "text"
            && (0..=cap as i64).contains(&length)
            && hash_type == "text"
            && hash_length == 64,
        "Archive SQL payload/hash type or byte cap before allocation"
    );
    let sql = format!(
        "SELECT data,data_sha256 FROM {table} WHERE {key}=?1 AND typeof(data)='text' AND length(CAST(data AS BLOB))=?2 AND typeof(data_sha256)='text' AND length(CAST(data_sha256 AS BLOB))=64"
    );
    let (data, hash): (String, String) =
        c.query_row(&sql, params![value, length], |r| Ok((r.get(0)?, r.get(1)?)))?;
    valid_sha(&hash)?;
    ensure!(
        data.len() as i64 == length && digest(&data) == hash,
        "Archive SQL payload bytes/hash differ"
    );
    Ok(Some(data))
}
fn profile(c: &Connection) -> Result<Option<SitemapSourceArchiveConfig>> {
    let pre:Option<(Option<String>,Option<i64>)>=c.query_row("SELECT json_type(value,'$.sitemap_source_archive'),length(CAST(json_extract(value,'$.sitemap_source_archive') AS BLOB)) FROM metadata WHERE key='config'",[],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    match pre {
        None | Some((None, _)) => Ok(None),
        Some((Some(t), _)) if t == "null" => Ok(None),
        Some((Some(t), Some(n))) => {
            require_utf8_storage(c)?;
            ensure!(
                t == "object" && (1..=4096).contains(&n),
                "Invalid/oversized saved archive profile"
            );
            let raw:String=c.query_row("SELECT json_extract(value,'$.sitemap_source_archive') FROM metadata WHERE key='config' AND length(CAST(json_extract(value,'$.sitemap_source_archive') AS BLOB))=?1",[n],|r|r.get(0))?;
            ensure!(raw.len() as i64 == n, "Saved archive profile changed");
            Ok(Some(serde_json::from_str(&raw)?))
        }
        _ => bail!("Invalid saved archive declaration"),
    }
}
fn save_state(tx: &Transaction<'_>, s: &SitemapSourceArchiveSummary) -> Result<()> {
    let data = canonical(s, ARCHIVE_STATE_RESERVE as usize)?;
    tx.execute(
        "UPDATE sitemap_body_archive_state SET data=?1,data_sha256=?2 WHERE id=1",
        params![data, digest(&data)],
    )?;
    Ok(())
}
pub(crate) fn initialize(
    tx: &Transaction<'_>,
    config: &CrawlConfig,
    config_sha: &str,
) -> Result<()> {
    let Some(p) = &config.sitemap_source_archive else {
        ensure!(
            table_count(tx)? == 0,
            "Archive tables require their saved opt-in profile"
        );
        return Ok(());
    };
    p.validate()?;
    require_storage_for_config(tx, config)?;
    valid_sha(config_sha)?;
    match table_count(tx)? {
        0 => {
            tx.execute_batch(SCHEMA)?;
            let legacy: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM sitemap_source_measurements)",
                [],
                |r| r.get(0),
            )?;
            let s = SitemapSourceArchiveSummary {
                engine: SITEMAP_SOURCE_ARCHIVE_ENGINE.into(),
                config: p.clone(),
                initial_config_sha256: config_sha.into(),
                captures: 0,
                body_bytes: 0,
                capture_json_bytes: 0,
                content_sha256: seed(),
                first_omission: None,
                legacy_history_unknown: legacy,
                directory_entry_crash_durability_proven: false,
            };
            let data = canonical(&s, ARCHIVE_STATE_RESERVE as usize)?;
            tx.execute(
                "INSERT INTO sitemap_body_archive_state(id,data,data_sha256) VALUES(1,?1,?2)",
                params![data, digest(&data)],
            )?;
        }
        2 => {
            load(tx)?.context("Initialized archive marker missing")?;
        }
        _ => bail!("Partial archive schema must not be repaired"),
    }
    Ok(())
}
fn measurement(c: &Connection, id: &str) -> Result<(SitemapSourceStoredMeasurement, String)> {
    valid_sha(id)?;
    let raw = text(
        c,
        "sitemap_source_measurements",
        "measurement_id",
        id,
        crate::MAX_SOURCE_RECORD_BYTES,
    )?
    .context("Archive has no committed measurement A")?;
    let record: SitemapSourceStoredMeasurement = serde_json::from_str(&raw)?;
    ensure!(
        record.schema_version == 1
            && record.engine == crate::SITEMAP_SOURCE_STORE_ENGINE
            && record.measurement.measurement_id == id,
        "Archive measurement profile/identity differs"
    );
    crate::validate_sitemap_source_measurement(&record.measurement, &Default::default())?;
    ensure!(
        canonical(&record, crate::MAX_SOURCE_RECORD_BYTES)? == raw,
        "Archive measurement envelope is not canonical"
    );
    Ok((record, digest(raw)))
}
fn body_id(measurement: &str, body: &SitemapArchivedBody) -> Result<String> {
    Ok(digest(canonical(
        &(
            SITEMAP_SOURCE_ARCHIVE_ENGINE,
            measurement,
            body.representation,
            body.extent,
            body.bytes,
            &body.sha256,
        ),
        1024,
    )?))
}
fn validate_capture(
    c: &Connection,
    row: &SitemapArchivedCapture,
    config: &SitemapSourceArchiveConfig,
) -> Result<()> {
    ensure!(
        row.schema_version == 1 && row.sequence > 0 && row.bodies.len() <= 2,
        "Archive capture schema/count differs"
    );
    let (record, hash) = measurement(c, &row.measurement_id)?;
    let m = &record.measurement.payload;
    ensure!(
        row.measurement_envelope_sha256 == hash
            && row.staged_document_sha256 == record.staged_document_sha256
            && row.capture_config_sha256 == m.binding.crawl_config_sha256
            && row.capture_generation == m.binding.run_generation,
        "Archive capture measurement/config/generation seal differs"
    );
    ensure!(
        row.omission.is_some() == row.bodies.is_empty(),
        "Archive omission/body state differs"
    );
    if row.omission.is_none() {
        let expected_post = config.retain_post_http_entity
            && m.transport.request_started
            && m.transport.retained_post_http_prefix_sha256.is_some();
        let expected_xml = config.retain_decoded_xml
            && m.decode.integrity == SitemapSourceDecodeIntegrity::Complete
            && m.decode.decoded_complete_sha256.is_some();
        ensure!(
            row.bodies
                .iter()
                .any(|b| b.representation == SitemapBodyRepresentation::PostHttpEntity)
                == expected_post
                && row
                    .bodies
                    .iter()
                    .any(|b| b.representation == SitemapBodyRepresentation::DecodedXml)
                    == expected_xml,
            "Archive silently omitted an observed selected representation"
        );
    }
    let mut seen = Vec::new();
    for b in &row.bodies {
        ensure!(
            !seen.contains(&b.representation),
            "Duplicate archive representation"
        );
        seen.push(b.representation);
        valid_sha(&b.sha256)?;
        valid_sha(&b.body_id)?;
        ensure!(
            b.bytes <= config.max_bytes_per_body && b.body_id == body_id(&row.measurement_id, b)?,
            "Archive body ID/size differs"
        );
        match b.representation {
            SitemapBodyRepresentation::PostHttpEntity => {
                ensure!(
                    config.retain_post_http_entity
                        && m.transport.request_started
                        && m.transport.retained_post_http_prefix_bytes == b.bytes
                        && m.transport.retained_post_http_prefix_sha256.as_ref() == Some(&b.sha256),
                    "Archive post-HTTP binding differs"
                );
                ensure!(
                    if m.transport.body_complete {
                        b.extent == SitemapBodyExtent::Complete
                            && m.transport.complete_post_http_entity_sha256.as_ref()
                                == Some(&b.sha256)
                    } else {
                        b.extent == SitemapBodyExtent::ObservedPrefix
                    },
                    "Archive transport completeness differs"
                );
            }
            SitemapBodyRepresentation::DecodedXml => ensure!(
                config.retain_decoded_xml
                    && b.extent == SitemapBodyExtent::Complete
                    && m.decode.integrity == SitemapSourceDecodeIntegrity::Complete
                    && m.decode.decoded_xml_bytes.precision == SitemapMeasuredPrecision::Exact
                    && m.decode.decoded_xml_bytes.value == Some(b.bytes)
                    && m.decode.decoded_complete_sha256.as_ref() == Some(&b.sha256),
                "Archive decoded XML binding differs"
            ),
        }
    }
    Ok(())
}
pub(crate) fn load(c: &Connection) -> Result<Option<SitemapSourceArchiveSnapshot>> {
    require_storage_for_existing(c)?;
    let declared = profile(c)?;
    match table_count(c)? {
        0 => {
            ensure!(
                declared.is_none(),
                "Enabled archive lacks initialized catalog"
            );
            return Ok(None);
        }
        2 => (),
        _ => bail!("Partial archive schema"),
    }
    for declaration in SCHEMA.split(';') {
        if let Some(start) = declaration.find("CREATE TABLE ") {
            let expected = declaration[start..].trim();
            let table = expected
                .strip_prefix("CREATE TABLE ")
                .context("Archive DDL prefix")?
                .split_whitespace()
                .next()
                .context("Archive DDL table")?;
            let actual: String = c.query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |r| r.get(0),
            )?;
            ensure!(
                actual.trim() == expected,
                "Archive table schema differs; no repair"
            );
        }
    }
    ensure!(
        c.query_row("SELECT count(*) FROM sitemap_body_archive_state", [], |r| r
            .get::<_, i64>(0))?
            == 1,
        "Archive state row count differs"
    );
    let raw = text(c, STATE, "id", "1", ARCHIVE_STATE_RESERVE as usize)?
        .context("Archive state missing")?;
    let summary: SitemapSourceArchiveSummary = serde_json::from_str(&raw)?;
    ensure!(
        canonical(&summary, ARCHIVE_STATE_RESERVE as usize)? == raw
            && summary.engine == SITEMAP_SOURCE_ARCHIVE_ENGINE
            && declared.as_ref() == Some(&summary.config),
        "Archive state/config/profile differs"
    );
    summary.config.validate()?;
    valid_sha(&summary.initial_config_sha256)?;
    ensure!(
        !summary.directory_entry_crash_durability_proven,
        "This archive profile has no directory-entry crash proof"
    );
    let (count,total,big,invalid_columns):(i64,i64,bool,bool)=c.query_row("SELECT count(*),coalesce(sum(length(CAST(data AS BLOB))),0),coalesce(max(length(CAST(data AS BLOB))),0)>8192,coalesce(max(CASE WHEN typeof(data)!='text' OR typeof(measurement_id)!='text' OR length(CAST(measurement_id AS BLOB))!=64 OR typeof(data_sha256)!='text' OR length(CAST(data_sha256 AS BLOB))!=64 THEN 1 ELSE 0 END),0)>0 FROM sitemap_body_archive_captures",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
    ensure!(
        count >= 0
            && count as u64 <= summary.config.max_capture_records
            && total >= 0
            && total as u64 + ARCHIVE_STATE_RESERVE <= MAX_ARCHIVE_METADATA_BYTES
            && !big
            && !invalid_columns,
        "Archive SQL catalog preflight budget or identity column"
    );
    // Repeated SQL predicates are a bounded projection, not permission to skip bad rows.
    // The preflight rejects any bad row in this same read transaction first.
    let mut statement=c.prepare("SELECT sequence,CASE WHEN typeof(measurement_id)='text' AND length(CAST(measurement_id AS BLOB))=64 THEN measurement_id END,CASE WHEN typeof(data)='text' AND length(CAST(data AS BLOB))<=8192 THEN data END,CASE WHEN typeof(data_sha256)='text' AND length(CAST(data_sha256 AS BLOB))=64 THEN data_sha256 END FROM sitemap_body_archive_captures ORDER BY sequence")?;
    let mut rows = statement.query([])?;
    let mut captures = Vec::new();
    let mut bytes = 0u64;
    let mut json_bytes = 0u64;
    let mut hash = seed();
    let mut identities = std::collections::HashSet::new();
    while let Some(row) = rows.next()? {
        let sequence: i64 = row.get(0)?;
        let id: String = row.get(1)?;
        let data: String = row.get(2)?;
        let data_sha: String = row.get(3)?;
        ensure!(
            data.len() <= MAX_ARCHIVE_CAPTURE_BYTES && digest(&data) == data_sha,
            "Archive capture SQL hash/length differs"
        );
        let r: SitemapArchivedCapture = serde_json::from_str(&data)?;
        ensure!(
            sequence == captures.len() as i64 + 1
                && r.sequence == sequence as u64
                && r.measurement_id == id
                && identities.insert(id)
                && canonical(&r, MAX_ARCHIVE_CAPTURE_BYTES)? == data,
            "Archive capture index/canonical bytes differ"
        );
        validate_capture(c, &r, &summary.config)?;
        for b in &r.bodies {
            bytes = bytes
                .checked_add(b.bytes)
                .context("Archive total overflow")?;
        }
        json_bytes = json_bytes
            .checked_add(data.len() as u64)
            .context("Archive JSON total overflow")?;
        hash = roll(&hash, &data);
        captures.push(r);
    }
    ensure!(
        captures.len() as u64 == summary.captures
            && bytes == summary.body_bytes
            && bytes <= summary.config.max_total_body_bytes
            && json_bytes == summary.capture_json_bytes
            && hash == summary.content_sha256,
        "Archive catalog aggregate bytes/hash differs"
    );
    Ok(Some(SitemapSourceArchiveSnapshot { summary, captures }))
}
pub(crate) struct Plan<'a> {
    pub(crate) base: SitemapSourceArchiveSummary,
    pub(crate) capture: Option<SitemapArchivedCapture>,
    pub(crate) bodies: Vec<(SitemapArchivedBody, &'a [u8])>,
    pub(crate) existing: bool,
    pub(crate) omission: Option<SitemapArchiveOmission>,
}
impl Plan<'_> {
    pub(crate) fn seal(&self) -> Result<String> {
        canonical(
            &(&self.base, &self.capture, self.existing, self.omission),
            MAX_ARCHIVE_CAPTURE_BYTES + ARCHIVE_STATE_RESERVE as usize,
        )
    }
}
pub(crate) fn plan<'a>(
    c: &Connection,
    id: &str,
    candidate: &'a SourceBodyCandidate,
) -> Result<Plan<'a>> {
    let snapshot = load(c)?.context("Archive capture needs explicit initialized catalog")?;
    let (record, hash) = measurement(c, id)?;
    let m = &record.measurement.payload;
    let config = &snapshot.summary.config;
    ensure!(
        candidate.post_http_entity.is_some()
            == (config.retain_post_http_entity
                && m.transport.request_started
                && m.transport.retained_post_http_prefix_sha256.is_some()),
        "Post-HTTP candidate presence differs from actual observed selected bytes"
    );
    ensure!(
        candidate.decoded_xml.is_some()
            == (config.retain_decoded_xml
                && m.decode.integrity == SitemapSourceDecodeIntegrity::Complete
                && m.decode.decoded_complete_sha256.is_some()),
        "Decoded candidate presence differs from actual observed selected bytes"
    );
    let mut bodies = Vec::new();
    for (representation, value) in [
        (
            SitemapBodyRepresentation::PostHttpEntity,
            &candidate.post_http_entity,
        ),
        (
            SitemapBodyRepresentation::DecodedXml,
            &candidate.decoded_xml,
        ),
    ] {
        if let Some(data) = value {
            let extent = match representation {
                SitemapBodyRepresentation::PostHttpEntity => {
                    if m.transport.body_complete {
                        SitemapBodyExtent::Complete
                    } else {
                        SitemapBodyExtent::ObservedPrefix
                    }
                }
                SitemapBodyRepresentation::DecodedXml => SitemapBodyExtent::Complete,
            };
            let mut b = SitemapArchivedBody {
                body_id: String::new(),
                representation,
                extent,
                bytes: data.len() as u64,
                sha256: digest(data),
            };
            b.body_id = body_id(id, &b)?;
            bodies.push((b, data.as_slice()));
        }
    }
    let mut capture = SitemapArchivedCapture {
        schema_version: 1,
        sequence: snapshot
            .summary
            .captures
            .checked_add(1)
            .context("Archive sequence overflow")?,
        measurement_id: id.into(),
        measurement_envelope_sha256: hash,
        staged_document_sha256: record.staged_document_sha256,
        capture_config_sha256: m.binding.crawl_config_sha256.clone(),
        capture_generation: m.binding.run_generation,
        bodies: bodies.iter().map(|(b, _)| b.clone()).collect(),
        omission: if bodies.is_empty() {
            Some(SitemapArchiveOmission::BytesUnobserved)
        } else {
            None
        },
    };
    // Validate actual byte bindings before any quota omission can hide bad facts.
    let mut validation = config.clone();
    validation.max_bytes_per_body = MAX_SITEMAP_ARCHIVE_BODY_BYTES;
    validate_capture(c, &capture, &validation)?;
    if let Some(old) = snapshot.captures.iter().find(|r| r.measurement_id == id) {
        capture.sequence = old.sequence;
        if old.omission.is_some() {
            bodies.clear();
            capture = old.clone();
        } else {
            ensure!(
                *old == capture,
                "Same archive capture has different bytes/context"
            );
        }
        return Ok(Plan {
            base: snapshot.summary,
            capture: Some(capture),
            bodies,
            existing: true,
            omission: None,
        });
    }
    let requested = bodies.iter().try_fold(0u64, |n, (b, _)| {
        n.checked_add(b.bytes).context("Archive pair byte overflow")
    })?;
    let omission = if snapshot.summary.captures >= config.max_capture_records {
        Some(SitemapArchiveOmission::CaptureCountBudget)
    } else if bodies
        .iter()
        .any(|(b, _)| b.bytes > config.max_bytes_per_body)
    {
        Some(SitemapArchiveOmission::BodyByteBudget)
    } else if snapshot
        .summary
        .body_bytes
        .checked_add(requested)
        .context("Archive quota overflow")?
        > config.max_total_body_bytes
    {
        Some(SitemapArchiveOmission::TotalByteBudget)
    } else {
        capture.omission
    };
    if let Some(reason) = omission {
        capture.omission = Some(reason);
        capture.bodies.clear();
        bodies.clear();
    }
    let raw = canonical(&capture, MAX_ARCHIVE_CAPTURE_BYTES)?;
    let metadata_over = snapshot
        .summary
        .capture_json_bytes
        .checked_add(raw.len() as u64)
        .and_then(|n| n.checked_add(ARCHIVE_STATE_RESERVE))
        .context("Archive metadata overflow")?
        > MAX_ARCHIVE_METADATA_BYTES;
    let no_record = omission == Some(SitemapArchiveOmission::CaptureCountBudget) || metadata_over;
    Ok(Plan {
        base: snapshot.summary,
        capture: if no_record { None } else { Some(capture) },
        bodies: if no_record { Vec::new() } else { bodies },
        existing: false,
        omission: if metadata_over {
            Some(SitemapArchiveOmission::MetadataByteBudget)
        } else {
            omission
        },
    })
}
pub(crate) fn commit(tx: &Transaction<'_>, plan: &Plan<'_>) -> Result<()> {
    let current = load(tx)?.context("Archive disappeared before commit")?;
    ensure!(
        current.summary == plan.base,
        "Archive quota/state changed before commit"
    );
    if plan.existing {
        return Ok(());
    }
    let mut s = current.summary;
    if s.first_omission.is_none() {
        s.first_omission = plan.omission;
    }
    if let Some(c) = &plan.capture {
        validate_capture(tx, c, &s.config)?;
        let data = canonical(c, MAX_ARCHIVE_CAPTURE_BYTES)?;
        s.captures = s
            .captures
            .checked_add(1)
            .context("Archive count overflow")?;
        for b in &c.bodies {
            s.body_bytes = s
                .body_bytes
                .checked_add(b.bytes)
                .context("Archive bytes overflow")?;
        }
        s.capture_json_bytes = s
            .capture_json_bytes
            .checked_add(data.len() as u64)
            .context("Archive metadata overflow")?;
        s.content_sha256 = roll(&s.content_sha256, &data);
        ensure!(
            s.captures <= s.config.max_capture_records
                && s.body_bytes <= s.config.max_total_body_bytes
                && s.capture_json_bytes + ARCHIVE_STATE_RESERVE <= MAX_ARCHIVE_METADATA_BYTES,
            "Archive commit quota changed"
        );
        tx.execute("INSERT INTO sitemap_body_archive_captures(sequence,measurement_id,data,data_sha256) VALUES(?1,?2,?3,?4)",params![i64::try_from(c.sequence)?,c.measurement_id,data,digest(&data)])?;
    }
    save_state(tx, &s)?;
    Ok(())
}
pub(crate) fn note_measurement_unavailable(tx: &Transaction<'_>) -> Result<()> {
    let mut snapshot = load(tx)?.context("Archive omission needs initialized catalog")?;
    if snapshot.summary.first_omission.is_none() {
        snapshot.summary.first_omission = Some(SitemapArchiveOmission::MeasurementUnavailable);
        save_state(tx, &snapshot.summary)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "sitemap_source_archive_tests.rs"]
mod tests;
