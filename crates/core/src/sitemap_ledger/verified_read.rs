//! Transaction-bound raw catalog verification. No HTTP, DDL or durable mutation.
//! Ordinary reads keep compact document counters and one bounded row. Inherited
//! anchors deliberately retain the old materializing compatibility verifier.
//! The caller owns a read transaction and must not write through its Connection
//! while the token is alive. This is a crate-private provenance boundary.
use super::*;
use serde::ser::{Error as _, SerializeSeq, SerializeStruct};

const WINDOW_BYTES: usize = 8 * 1024 * 1024;
const ROWS_SQL: &str = "SELECT 'root' AS kind,sequence,row_id,observation_id,ordinal,NULL AS source_kind,NULL AS task_url,NULL AS generation,NULL AS revision_or_entry,data,data_sha256 FROM sitemap_roots
 UNION ALL SELECT 'document',sequence,row_id,NULL,NULL,source_kind,task_url,generation,source_revision,data,data_sha256 FROM sitemap_documents
 UNION ALL SELECT 'entry',sequence,row_id,observation_id,ordinal,NULL,NULL,NULL,NULL,data,data_sha256 FROM sitemap_entries
 UNION ALL SELECT 'location',sequence,row_id,observation_id,ordinal,NULL,NULL,NULL,entry_id,data,data_sha256 FROM sitemap_locations";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum SitemapLedgerRowKind {
    #[default]
    All,
    Root,
    Document,
    Entry,
    Location,
}
impl SitemapLedgerRowKind {
    fn name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Root => "root",
            Self::Document => "document",
            Self::Entry => "entry",
            Self::Location => "location",
        }
    }
}
/// Matches App's existing exact original-identity filter. Future App adaptation
/// must share this contract rather than add alias/final-URL matching.
#[derive(Debug, Clone, Default)]
pub(crate) struct SitemapLedgerFilter {
    pub(crate) kind: SitemapLedgerRowKind,
    pub(crate) source_observation_id: Option<String>,
    pub(crate) membership_url: Option<String>,
}
impl SitemapLedgerFilter {
    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(id) = &self.source_observation_id {
            sha(id)?;
        }
        if let Some(url) = &self.membership_url {
            ensure!(
                !url.is_empty()
                    && url.len() <= 65_536
                    && !url.chars().any(char::is_control)
                    && crate::normalize_url(url, None).is_some_and(|v| v.as_str() == url),
                "Expected exact canonical original membership URL"
            );
        }
        Ok(())
    }
}
pub(crate) struct VerifiedRawRead<'conn> {
    connection: &'conn Connection,
    summary: SitemapLedgerSummary,
    inherited: Option<SitemapInheritedBasis>,
}
impl VerifiedRawRead<'_> {
    pub(crate) fn summary(&self) -> &SitemapLedgerSummary {
        &self.summary
    }
    pub(crate) fn inherited(&self) -> Option<&SitemapInheritedBasis> {
        self.inherited.as_ref()
    }
    pub(crate) fn connection(&self) -> &Connection {
        self.connection
    }
    pub(crate) fn document_by_id(&self, id: &str) -> Result<Option<SitemapSourceDocument>> {
        sha(id)?;
        document(self.connection, &self.summary.limits, id)
    }
    pub(crate) fn visit_filtered(
        &self,
        filter: &SitemapLedgerFilter,
        mut visitor: impl FnMut(&SitemapLedgerRow) -> Result<()>,
    ) -> Result<()> {
        self.filtered(filter, 0, None, -1, |row| visitor(&row))
    }
    fn filtered(
        &self,
        filter: &SitemapLedgerFilter,
        lower: u64,
        upper: Option<u64>,
        limit: i64,
        mut visitor: impl FnMut(SitemapLedgerRow) -> Result<()>,
    ) -> Result<()> {
        ensure!(
            !self.connection.is_autocommit(),
            "Verified raw read transaction ended"
        );
        filter.validate()?;
        let sql = format!("SELECT * FROM ({ROWS_SQL}) WHERE
          (?1='all' OR kind=?1)
          AND (?2 IS NULL OR CASE WHEN kind='document' THEN row_id ELSE observation_id END=?2)
          AND (?3 IS NULL OR (kind IN ('root','location') AND json_extract(data,'$.row.admission.membership_url')=?3))
          AND sequence>=?4 AND (?5 IS NULL OR sequence<=?5)
          ORDER BY sequence LIMIT ?6");
        let mut statement = self.connection.prepare(&sql)?;
        let mut cursor = statement.query(params![
            filter.kind.name(),
            filter.source_observation_id,
            filter.membership_url,
            i64::try_from(lower)?,
            upper.map(i64::try_from).transpose()?,
            limit
        ])?;
        while let Some(row) = cursor.next()? {
            visitor(decode_row(row, &self.summary.limits)?)?;
        }
        Ok(())
    }
    pub(crate) fn window(
        &self,
        filter: &SitemapLedgerFilter,
        after: Option<u64>,
        limit: usize,
    ) -> Result<SitemapLedgerPage> {
        ensure!(
            (1..=1000).contains(&limit),
            "Sitemap page limit must be 1..=1000"
        );
        filter.validate()?;
        if let Some(after) = after {
            ensure!(
                after > 0 && after <= self.summary.last_sequence,
                "Invalid raw continuation sequence"
            );
            let mut found = false;
            self.filtered(filter, after, Some(after), 1, |_| {
                found = true;
                Ok(())
            })?;
            ensure!(
                found,
                "Continuation sequence is not a matching retained row"
            );
        }
        let mut rows = Vec::new();
        let mut bytes = 0usize;
        let mut has_more = false;
        self.filtered(
            filter,
            after
                .unwrap_or(0)
                .checked_add(1)
                .context("Raw cursor overflow")?,
            None,
            i64::try_from(limit + 1)?,
            |row| {
                if rows.len() == limit {
                    has_more = true;
                    return Ok(());
                }
                bytes = bytes
                    .checked_add(row_data(&row, &self.summary.limits)?.len())
                    .context("Raw window size overflow")?;
                ensure!(bytes <= WINDOW_BYTES, "Sitemap raw window exceeds 8 MiB");
                rows.push(row);
                Ok(())
            },
        )?;
        let next = has_more.then(|| rows.last().expect("positive window").sequence());
        let page = SitemapLedgerPage {
            ledger: Some(self.summary.clone()),
            inherited: self.inherited.clone(),
            rows,
            next_after_sequence: next,
            has_more,
        };
        measured(&page, WINDOW_BYTES)?;
        Ok(page)
    }
}

fn optional_text<'a>(row: &'a rusqlite::Row<'_>, index: usize) -> Result<Option<&'a str>> {
    let value = row.get_ref(index)?;
    if matches!(value, rusqlite::types::ValueRef::Null) {
        Ok(None)
    } else {
        Ok(Some(value.as_str()?))
    }
}
fn decode_row(sql: &rusqlite::Row<'_>, limits: &SitemapLedgerLimits) -> Result<SitemapLedgerRow> {
    // Borrow projection strings: a tampered SQL index column must not allocate
    // a huge Rust String before its equality to the bounded payload is checked.
    let kind = sql.get_ref(0)?.as_str()?;
    let sequence: i64 = sql.get(1)?;
    let id = sql.get_ref(2)?.as_str()?;
    let observation = optional_text(sql, 3)?;
    let ordinal: Option<i64> = sql.get(4)?;
    let source_kind = optional_text(sql, 5)?;
    let task_url = optional_text(sql, 6)?;
    let generation: Option<i64> = sql.get(7)?;
    let revision_or_entry = optional_text(sql, 8)?;
    let data = sql.get_ref(9)?.as_str()?;
    ensure!(
        data.len() <= limits.max_row_json_bytes,
        "Oversized durable sitemap row"
    );
    let stored_sha = sql.get_ref(10)?.as_str()?;
    ensure!(
        digest(data) == stored_sha,
        "Durable sitemap row payload SHA mismatch"
    );
    let typed: SitemapLedgerRow = serde_json::from_str(data)?;
    ensure!(
        typed.kind() == kind
            && typed.sequence() == u64::try_from(sequence)?
            && row_data(&typed, limits)? == data,
        "Noncanonical sitemap row/type/sequence"
    );
    match &typed {
        SitemapLedgerRow::Root(row) => ensure!(
            id == row.declaration_id
                && observation == row.source_observation_id.as_deref()
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
                    == Some(
                        identity(
                            "entry",
                            &row.observation_id,
                            u64::try_from(row.observed.entry_ordinal)?
                        )
                        .as_str()
                    ),
            "Loc SQL indexes differ from payload"
        ),
    }
    Ok(typed)
}
fn visit_all(
    connection: &Connection,
    limits: &SitemapLedgerLimits,
    mut visitor: impl FnMut(&SitemapLedgerRow) -> Result<()>,
) -> Result<()> {
    let mut statement = connection.prepare(&format!("{ROWS_SQL} ORDER BY 2"))?;
    let mut cursor = statement.query([])?;
    while let Some(row) = cursor.next()? {
        visitor(&decode_row(row, limits)?)?;
    }
    Ok(())
}
fn document(
    connection: &Connection,
    limits: &SitemapLedgerLimits,
    id: &str,
) -> Result<Option<SitemapSourceDocument>> {
    let mut query = connection.prepare("SELECT 'document',sequence,row_id,NULL,NULL,source_kind,task_url,generation,source_revision,data,data_sha256 FROM sitemap_documents WHERE row_id=?1")?;
    let mut rows = query.query([id])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let SitemapLedgerRow::Document(value) = decode_row(row, limits)? else {
        unreachable!()
    };
    ensure!(rows.next()?.is_none(), "Duplicate document ID");
    Ok(Some(*value))
}
fn entry(
    connection: &Connection,
    limits: &SitemapLedgerLimits,
    id: &str,
) -> Result<StoredSitemapEntry> {
    let mut query = connection.prepare("SELECT 'entry',sequence,row_id,observation_id,ordinal,NULL,NULL,NULL,NULL,data,data_sha256 FROM sitemap_entries WHERE row_id=?1")?;
    let mut rows = query.query([id])?;
    let row = rows.next()?.context("Loc precedes/misses entry")?;
    let SitemapLedgerRow::Entry(value) = decode_row(row, limits)? else {
        unreachable!()
    };
    ensure!(rows.next()?.is_none(), "Duplicate entry ID");
    Ok(value)
}
struct DocumentCounters {
    entries: usize,
    locations: usize,
    roots: u64,
}
fn relation_row(
    connection: &Connection,
    summary: &SitemapLedgerSummary,
    counters: &mut BTreeMap<String, DocumentCounters>,
    last_document: &mut Option<SitemapSourceDocument>,
    row: &SitemapLedgerRow,
) -> Result<()> {
    let source_id = match row {
        SitemapLedgerRow::Document(doc) => {
            validate_document(doc)?;
            ensure!(
                counters
                    .insert(
                        doc.observation_id.clone(),
                        DocumentCounters {
                            entries: 0,
                            locations: 0,
                            roots: 0
                        }
                    )
                    .is_none(),
                "Duplicate source document"
            );
            *last_document = Some(doc.as_ref().clone());
            return Ok(());
        }
        SitemapLedgerRow::Root(root) => root.source_observation_id.as_deref(),
        SitemapLedgerRow::Entry(entry) => Some(entry.observation_id.as_str()),
        SitemapLedgerRow::Location(loc) => Some(loc.observation_id.as_str()),
    };
    if let Some(id) = source_id {
        ensure!(counters.contains_key(id), "Row precedes/misses document");
        if last_document
            .as_ref()
            .is_none_or(|doc| doc.observation_id != id)
        {
            *last_document = document(connection, &summary.limits, id)?;
        }
    }
    let doc = source_id
        .map(|_| -> Result<&SitemapSourceDocument> {
            let doc = last_document.as_ref().context("Missing raw document")?;
            ensure!(
                doc.sequence < row.sequence(),
                "Row precedes/misses document"
            );
            Ok(doc)
        })
        .transpose()?;
    match row {
        SitemapLedgerRow::Root(root) => {
            ensure!(!root.declaration_id.is_empty(), "Empty root ID");
            let basis = root
                .source_observation_id
                .as_deref()
                .unwrap_or(&summary.config_sha256);
            ensure!(
                root.declaration_id == root_declaration_id(&root.kind, basis, root.source_ordinal),
                "Root declaration ID mismatch"
            );
            let complete = if let Some(doc) = &doc {
                ensure!(
                    doc.kind == SitemapSourceKind::RobotsDirectives
                        && root.kind == SitemapRootKind::RobotsDirective,
                    "Root has wrong source kind"
                );
                let stats = counters
                    .get_mut(&doc.observation_id)
                    .expect("checked document");
                ensure!(
                    root.source_ordinal == stats.roots,
                    "Robots declaration ordinal gap/reorder"
                );
                stats.roots = stats
                    .roots
                    .checked_add(1)
                    .context("Robots ordinal overflow")?;
                let evidence = root
                    .robots_evidence
                    .as_ref()
                    .context("Robots raw-line evidence missing")?;
                let raw = root
                    .raw_line
                    .as_deref()
                    .context("Robots raw line missing")?;
                ensure!(
                    evidence.byte_start <= evidence.byte_end
                        && u64::try_from(evidence.byte_end)?
                            <= doc
                                .transport
                                .as_ref()
                                .context("Robots transport missing")?
                                .post_http_decoding_bytes,
                    "Robots raw-line byte span outside body"
                );
                if !evidence.utf8_lossy {
                    ensure!(
                        evidence.byte_end - evidence.byte_start == raw.len(),
                        "Lossless robots span differs"
                    );
                }
                let line = raw.strip_suffix('\n').unwrap_or(raw);
                let line = line.strip_suffix('\r').unwrap_or(line);
                let (key, value) = line
                    .split_once(':')
                    .context("Robots declaration has no colon")?;
                ensure!(
                    key.trim().eq_ignore_ascii_case("sitemap")
                        && value == root.raw_value
                        && evidence.line_ordinal > 0,
                    "Robots raw line/value/ordinal differs"
                );
                if doc.complete() {
                    ensure!(
                        evidence.line_complete && !evidence.utf8_lossy,
                        "Complete robots contains lossy/incomplete line"
                    );
                }
                doc.complete()
            } else {
                ensure!(
                    root.kind != SitemapRootKind::RobotsDirective && root.robots_evidence.is_none(),
                    "Unbound robots root evidence"
                );
                true
            };
            ensure!(
                root.admission.membership_url == root.normalized_document_url,
                "Root original membership changed"
            );
            check_binding(&root.admission, complete)?;
        }
        SitemapLedgerRow::Entry(value) => {
            let doc = doc.as_ref().context("Entry without document")?;
            let parser = doc.parser.as_ref().context("Entry without parser facts")?;
            ensure!(
                doc.kind == SitemapSourceKind::SitemapXml,
                "Entry without XML document"
            );
            ensure!(
                value.entry_id
                    == identity(
                        "entry",
                        &value.observation_id,
                        u64::try_from(value.observed.ordinal)?
                    ),
                "Entry ID mismatch"
            );
            let stats = counters.get_mut(&value.observation_id).expect("checked");
            stats.entries = stats
                .entries
                .checked_add(1)
                .context("Entry count overflow")?;
            ensure!(
                value.observed.ordinal == stats.entries,
                "Parser entry ordinal gap/reorder"
            );
            ensure!(
                value.observed.xml_byte_start <= value.observed.xml_byte_end
                    && value.observed.xml_byte_end <= u64::try_from(parser.xml_bytes)?,
                "Entry XML span outside document"
            );
        }
        SitemapLedgerRow::Location(value) => {
            let doc = doc.as_ref().context("Loc without document")?;
            let related = entry(
                connection,
                &summary.limits,
                &identity(
                    "entry",
                    &value.observation_id,
                    u64::try_from(value.observed.entry_ordinal)?,
                ),
            )?;
            ensure!(related.sequence < value.sequence, "Loc precedes entry");
            ensure!(
                value.occurrence_id
                    == identity(
                        "location",
                        &value.observation_id,
                        u64::try_from(value.observed.ordinal)?
                    ),
                "Loc ID mismatch"
            );
            let stats = counters.get_mut(&value.observation_id).expect("checked");
            stats.locations = stats
                .locations
                .checked_add(1)
                .context("Location count overflow")?;
            ensure!(
                value.observed.ordinal == stats.locations,
                "Parser Loc ordinal gap/reorder"
            );
            ensure!(
                value.observed.xml_byte_start >= related.observed.xml_byte_start
                    && value.observed.xml_byte_end <= related.observed.xml_byte_end
                    && value.observed.xml_byte_start <= value.observed.xml_byte_end,
                "Loc span outside entry"
            );
            ensure!(
                value.admission.membership_url == value.observed.normalized_fetch_url,
                "Original membership changed"
            );
            check_binding(&value.admission, doc.complete())?;
            if value.observed.loc_ordinal.is_none() {
                ensure!(
                    related.observed.complete
                        && value.observed.raw_element_xml.is_none()
                        && value.observed.raw_inner_xml.is_none()
                        && value.observed.decoded_value.is_none()
                        && value.observed.normalized_fetch_url.is_none()
                        && value.observed.validation_error.as_deref() == Some("missing_location"),
                    "Invalid synthetic missing-Loc row"
                );
                ensure!(
                    matches!(
                        value.admission.decision,
                        SitemapAdmissionDecision::MissingLocation
                            | SitemapAdmissionDecision::IncompleteSource
                    ),
                    "Missing Loc got admission"
                );
            } else {
                ensure!(
                    value.observed.raw_element_xml.is_some()
                        && value.observed.raw_inner_xml.is_some()
                        && value.observed.decoded_value.is_some(),
                    "Observed Loc raw fields missing"
                );
                if value.observed.validation_error.is_some()
                    || value.observed.normalized_fetch_url.is_none()
                {
                    ensure!(
                        matches!(
                            value.admission.decision,
                            SitemapAdmissionDecision::InvalidLocation
                                | SitemapAdmissionDecision::IncompleteSource
                        ),
                        "Invalid Loc got admission"
                    );
                }
            }
            let child = if value.observed.role == crate::SitemapLocationRole::ChildSitemap {
                value.admission.admitted_fetch_url.as_ref()
            } else {
                None
            };
            ensure!(
                value.referenced_sitemap_task.as_ref() == child,
                "Child alias differs"
            );
            if value.admission.policy_occurrence_id.is_some() {
                ensure!(
                    value.admission.policy_source_revision.as_deref()
                        == Some(doc.source_revision.as_str()),
                    "Policy/source revision differs"
                );
            }
        }
        SitemapLedgerRow::Document(_) => unreachable!(),
    }
    verify_row_policy_binding(connection, row, doc)?;
    Ok(())
}

/// One ordered relation pass, never a full locations scan per entry. Declared
/// lists are sorted exactly as the old validator; input order is not tightened.
fn verify_entry_locations(connection: &Connection) -> Result<()> {
    let mut query = connection.prepare(
        "SELECT e.row_id,e.data,l.data
      FROM sitemap_entries e LEFT JOIN sitemap_locations l ON l.entry_id=e.row_id
      ORDER BY e.row_id COLLATE BINARY,l.ordinal,l.sequence",
    )?;
    let mut cursor = query.query([])?;
    let mut current: Option<String> = None;
    let mut declared = Vec::new();
    let mut seen = 0usize;
    while let Some(row) = cursor.next()? {
        let id: String = row.get(0)?;
        if current.as_ref() != Some(&id) {
            ensure!(seen == declared.len(), "Entry Loc references differ");
            let SitemapLedgerRow::Entry(entry) = serde_json::from_str(row.get_ref(1)?.as_str()?)?
            else {
                bail!("Expected typed entry relation row");
            };
            declared = entry.observed.loc_occurrence_ordinals;
            declared.sort_unstable();
            seen = 0;
            current = Some(id);
        }
        let raw = row.get_ref(2)?;
        if !matches!(raw, rusqlite::types::ValueRef::Null) {
            let SitemapLedgerRow::Location(loc) = serde_json::from_str(raw.as_str()?)? else {
                bail!("Expected typed location relation row");
            };
            if loc.observed.loc_ordinal.is_some() {
                ensure!(
                    declared.get(seen) == Some(&loc.observed.ordinal),
                    "Entry Loc references differ"
                );
                seen = seen.checked_add(1).context("Entry Loc count overflow")?;
            }
        }
    }
    ensure!(seen == declared.len(), "Entry Loc references differ");
    Ok(())
}

struct StreamRows<'a, 'conn> {
    read: &'a VerifiedRawRead<'conn>,
}
impl Serialize for StreamRows<'_, '_> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(None)?;
        self.read
            .visit_filtered(&SitemapLedgerFilter::default(), |row| {
                seq.serialize_element(row)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))
            })
            .map_err(S::Error::custom)?;
        seq.end()
    }
}
struct CatalogEnvelope<'a, 'conn> {
    read: &'a VerifiedRawRead<'conn>,
}
impl Serialize for CatalogEnvelope<'_, '_> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("SitemapCatalog", 4)?;
        state.serialize_field("schema_version", &1u32)?;
        state.serialize_field("summary", &self.read.summary)?;
        state.serialize_field("inherited", &self.read.inherited)?;
        state.serialize_field("rows", &StreamRows { read: self.read })?;
        state.end()
    }
}

pub(crate) fn verify_raw(connection: &Connection) -> Result<Option<VerifiedRawRead<'_>>> {
    ensure!(
        !connection.is_autocommit(),
        "Raw verification requires a caller-owned transaction"
    );
    let summary: Option<SitemapLedgerSummary> = metadata(connection, SUMMARY_KEY)?;
    let inherited: Option<SitemapInheritedBasis> = metadata(connection, INHERITED_KEY)?;
    let Some(summary) = summary else {
        ensure!(inherited.is_none(), "Inherited marker without summary");
        if present_tables(connection)? != 0 {
            require_empty(connection)?;
        }
        return Ok(None);
    };
    // Existing exact immutable-parent validator is explicitly materializing
    // compatibility. The token does not retain its full catalog.
    if inherited.is_some() {
        let catalog = load_catalog(connection)?.context("Inherited catalog unavailable")?;
        return Ok(Some(VerifiedRawRead {
            connection,
            summary: catalog.summary,
            inherited: catalog.inherited,
        }));
    }
    summary.limits.validate()?;
    sha(&summary.config_sha256)?;
    sha(&summary.rolling_sha256)?;
    ensure!(
        summary.engine == SITEMAP_LEDGER_ENGINE
            && summary.hash_profile == SITEMAP_LEDGER_HASH_PROFILE,
        "Unknown raw catalog/hash profile"
    );
    ensure!(
        summary.rows_available && !summary.declared_source_cohort_complete,
        "Unsupported raw summary flags"
    );
    require_tables(connection)?;
    let mut total_rows = 0u64;
    let mut total_bytes = 0u64;
    for table in TABLES {
        let (count, bytes, max): (i64,i64,i64) = connection.query_row(&format!(
            "SELECT count(*),coalesce(sum(length(CAST(data AS BLOB))),0),coalesce(max(length(CAST(data AS BLOB))),0) FROM {table}"),
            [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        total_rows = total_rows
            .checked_add(u64::try_from(count)?)
            .context("Row count overflow")?;
        total_bytes = total_bytes
            .checked_add(u64::try_from(bytes)?)
            .context("Row bytes overflow")?;
        ensure!(
            u64::try_from(max)? <= summary.limits.max_row_json_bytes as u64,
            "Oversized raw row"
        );
    }
    ensure!(
        total_rows <= summary.limits.max_rows
            && total_bytes <= summary.limits.max_catalog_json_bytes as u64,
        "Raw catalog rows exceed budgets"
    );
    // Never assume imported DDL retained UNIQUE constraints.
    let duplicate: bool = connection.query_row(
        "SELECT EXISTS(SELECT row_id FROM (
       SELECT row_id FROM sitemap_roots UNION ALL SELECT row_id FROM sitemap_documents
       UNION ALL SELECT row_id FROM sitemap_entries UNION ALL SELECT row_id FROM sitemap_locations
       ) GROUP BY row_id COLLATE BINARY HAVING count(*)>1)",
        [],
        |r| r.get(0),
    )?;
    ensure!(!duplicate, "Duplicate raw row ID across catalog");
    let duplicate_task: bool = connection.query_row(
        "SELECT EXISTS(SELECT source_kind,task_url
        FROM sitemap_documents GROUP BY source_kind COLLATE BINARY,task_url COLLATE BINARY HAVING count(*)>1)",
        [],
        |r| r.get(0),
    )?;
    ensure!(!duplicate_task, "Duplicate observed source task");
    let mut computed = summary.clone();
    computed.last_sequence = 0;
    computed.root_rows = 0;
    computed.document_rows = 0;
    computed.entry_rows = 0;
    computed.location_rows = 0;
    computed.incomplete_documents = 0;
    computed.rolling_sha256 = seed_hash();
    let mut counters = BTreeMap::new();
    let mut last_document = None;
    visit_all(connection, &summary.limits, |row| {
        account(&mut computed, row)?;
        relation_row(connection, &summary, &mut counters, &mut last_document, row)
    })?;
    for (id, stats) in counters {
        let doc =
            document(connection, &summary.limits, &id)?.context("Missing counted document")?;
        if let Some(count) = doc.declaration_count {
            ensure!(
                stats.roots == u64::try_from(count)?,
                "Robots declaration count differs"
            );
        }
        if let Some(parser) = &doc.parser {
            ensure!(
                stats.entries == parser.entry_count && stats.locations == parser.location_count,
                "Retained parser row counts differ"
            );
        }
    }
    ensure!(
        computed.last_sequence <= computed.limits.max_rows
            && computed.revision <= computed.last_sequence,
        "Raw row/revision bounds invalid"
    );
    ensure!(
        computed == summary,
        "Raw counts/rolling SHA/global sequence differ"
    );
    verify_entry_locations(connection)?;
    let read = VerifiedRawRead {
        connection,
        summary,
        inherited,
    };
    ensure!(
        measured(
            &CatalogEnvelope { read: &read },
            read.summary.limits.max_catalog_json_bytes
        )? == read.summary.catalog_json_bytes,
        "Raw full serialized envelope byte count differs"
    );
    Ok(Some(read))
}

#[cfg(test)]
#[path = "verified_read_tests.rs"]
mod tests;
