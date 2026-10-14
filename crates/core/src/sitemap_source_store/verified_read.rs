//! Verified source summary over the caller's actual SQLite read transaction.
//!
//! The ordinary path retains one bounded record/link/document at a time. SQL
//! aggregates, duplicate detection and ordered joins may use SQLite working
//! storage; this is not a claim about SQLite RSS or query-plan performance.
//! Selected parent validation deliberately retains the old materializing
//! compatibility path, including the exact parent BLOB and source projection.
use super::*;
use crate::sitemap_read_control::SitemapReadControl;

use crate::sitemap_ledger::verified_read::VerifiedRawRead;

/// Constructible only after the complete source tail and summary were checked.
/// No public header constructor, Clone or deserialization capability.
pub(crate) struct VerifiedSourceRead<'conn> {
    connection: &'conn Connection,
    summary: SitemapSourceStoreSummary,
    context: String,
    selected_compatibility: bool,
}
impl VerifiedSourceRead<'_> {
    pub(crate) fn summary(&self) -> &SitemapSourceStoreSummary {
        &self.summary
    }
    pub(crate) fn context(&self) -> &str {
        &self.context
    }
    pub(crate) fn connection(&self) -> &Connection {
        self.connection
    }
    pub(crate) fn materializing_selected_compatibility(&self) -> bool {
        self.selected_compatibility
    }

    /// Visit actual measured attempts with only their exact durable B link.
    /// The complete source tail was verified before this capability existed.
    /// A-only captures remain visible; shared legacy source IDs cannot attach a
    /// second capture to a document committed by a different measurement ID.
    pub(crate) fn visit_measurements_after(
        &self,
        after_sequence: u64,
        control: &SitemapReadControl,
        mut visitor: impl FnMut(
            SitemapSourceStoredMeasurement,
            Option<SitemapSourceCommitAssociation>,
        ) -> Result<std::ops::ControlFlow<()>>,
    ) -> Result<bool> {
        let _binding = control.bind_connection(self.connection)?;
        control.run(|| {
            ensure!(!self.connection.is_autocommit(), "Verified source read transaction ended");
            let mut statement = self.connection.prepare("SELECT m.sequence,m.measurement_id,m.source_kind,m.task_url,m.generation,m.source_revision,m.expected_document_id,m.capture_config_sha256,m.initial_catalog_config_sha256,m.data,m.data_sha256,c.document_id,c.measurement_id,c.document_sequence,c.data,c.data_sha256 FROM sitemap_source_measurements m LEFT JOIN sitemap_source_measurement_commits c ON c.measurement_id COLLATE BINARY=m.measurement_id COLLATE BINARY WHERE m.sequence>?1 ORDER BY m.sequence")?;
            let mut rows = statement.query([i64::try_from(after_sequence)?])?;
            while let Some(row) = rows.next()? {
                control.checkpoint()?;
                let (record, _) = read_record_row(row, 0, &self.summary.limits)?;
                let association = if matches!(row.get_ref(11)?, rusqlite::types::ValueRef::Null) {
                    None
                } else {
                    let (association, _) = read_commit_row(row, 11)?;
                    validate_link_record(&association, &record, &self.summary.limits)?;
                    Some(association)
                };
                if visitor(record, association)?.is_break() { return Ok(false); }
            }
            Ok(true)
        })
    }
}

/// This low-level validator does not replace Store's selected row/frontier and
/// legacy-presence gates. It independently preserves source Some/None binding
/// to the actual validated immutable parent, including when no source token is
/// returned. A selected None result still incurred compatibility costs.
fn selected_parent_projection(
    connection: &Connection,
) -> Result<Option<Option<SitemapSourceStoreSnapshot>>> {
    // Match the new raw-read facade's explicit input cap. The legacy selected
    // SourceSome load/anchor below keeps its original 64MiB compatibility cap.
    let plan: crate::RunPlan = metadata(connection, "run_plan", 268_435_456)?.unwrap_or_default();
    let crate::RunPlan::SelectedRecrawl(plan) = plan else {
        ensure!(
            bounded_metadata(connection, "operation_kind", 64)?.is_none(),
            "Full source read has a selected operation marker"
        );
        if table_exists(connection, "recrawl_base")? {
            ensure!(
                !connection
                    .query_row("SELECT EXISTS(SELECT 1 FROM recrawl_base)", [], |r| r
                        .get::<_, bool>(0))?,
                "Full source read has an immutable selected parent"
            );
        }
        return Ok(None);
    };
    ensure!(
        bounded_metadata(connection, "operation_kind", 64)?.as_deref() == Some("selected_recrawl"),
        "Source-store selected operation differs"
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
        bytes.len() as i64 == len && digest(&bytes) == plan.parent.snapshot_sha256,
        "Source-store exact parent BLOB SHA differs"
    );
    let config = actual_config(connection)?.1;
    let prepared = crate::recrawl::validate_saved_plan(&plan, &bytes, &config)?;
    prepared
        .base_report
        .sitemap_source_measurements
        .as_ref()
        .map(|parent| project_inherited_snapshot(parent, &plan.parent.snapshot_sha256))
        .transpose()
        .map(Some)
}

fn bind_raw_summary(summary: &SitemapSourceStoreSummary, raw: &VerifiedRawRead<'_>) -> Result<()> {
    ensure!(
        summary.initial_catalog_config_sha256 == raw.summary().config_sha256,
        "Source-store/legacy initial config differs"
    );
    ensure!(
        !raw.summary().legacy_history_unknown || summary.legacy_history_unknown,
        "Source-store cannot narrow legacy unknown history"
    );
    ensure!(
        summary.inherited.is_some() == raw.inherited().is_some(),
        "Source-store/legacy inherited scope differs"
    );
    if let (Some(source), Some(legacy)) = (&summary.inherited, raw.inherited()) {
        ensure!(
            source.parent_snapshot_sha256 == legacy.parent_snapshot_sha256,
            "Source-store/legacy parent differs"
        );
    }
    Ok(())
}

// Schema shape checks deliberately do not assume uniqueness indices survived.
// SQL detection avoids accumulating every identity in a Rust set.
fn reject_duplicates(connection: &Connection) -> Result<()> {
    for query in [
        "SELECT EXISTS(SELECT 1 FROM sitemap_source_measurements GROUP BY measurement_id HAVING count(*)>1)",
        "SELECT EXISTS(SELECT 1 FROM sitemap_source_measurement_commits GROUP BY document_id HAVING count(*)>1)",
        "SELECT EXISTS(SELECT 1 FROM sitemap_source_measurement_commits GROUP BY measurement_id HAVING count(*)>1)",
    ] {
        ensure!(
            !connection.query_row(query, [], |row| row.get::<_, bool>(0))?,
            "Duplicate source measurement/commit identity"
        );
    }
    Ok(())
}

/// All source rows are verified before this returns a summary/context. The raw
/// token must have been verified on this exact connection in the same caller TX.
/// No connections, read-time migrations, snapshots or task completion invented.
pub(crate) fn verify_source<'conn>(
    connection: &'conn Connection,
    raw: Option<&VerifiedRawRead<'conn>>,
) -> Result<Option<VerifiedSourceRead<'conn>>> {
    verify_source_with_control(connection, raw, &SitemapReadControl::default())
}
pub(crate) fn verify_source_with_control<'conn>(
    connection: &'conn Connection,
    raw: Option<&VerifiedRawRead<'conn>>,
    control: &SitemapReadControl,
) -> Result<Option<VerifiedSourceRead<'conn>>> {
    let _binding = control.bind_connection(connection)?;
    control.run(|| verify_source_inner(connection, raw, control))
}
fn verify_source_inner<'conn>(
    connection: &'conn Connection,
    raw: Option<&VerifiedRawRead<'conn>>,
    control: &SitemapReadControl,
) -> Result<Option<VerifiedSourceRead<'conn>>> {
    ensure!(
        !connection.is_autocommit(),
        "Verified source read needs the caller's transaction"
    );
    if let Some(raw) = raw {
        ensure!(
            std::ptr::eq(raw.connection(), connection),
            "Verified source/raw reads use different connections"
        );
    }
    let selected = selected_parent_projection(connection)?;
    control.checkpoint()?;
    if let Some(expected) = selected {
        // Explicit, bounded-by-existing-limits but materializing compatibility.
        // load retains every existing canonical/SQL/doc/parent validation.
        control.checkpoint()?;
        let snapshot = load(connection)?;
        control.checkpoint()?;
        ensure!(
            snapshot == expected,
            "Selected source history presence/content differs from immutable parent"
        );
        let Some(snapshot) = snapshot else {
            return Ok(None);
        };
        let raw = raw.context("Source-store marker has no verified legacy catalog")?;
        bind_raw_summary(&snapshot.summary, raw)?;
        let context = summary_context(Some(&snapshot.summary))?;
        return Ok(Some(VerifiedSourceRead {
            connection,
            summary: snapshot.summary,
            context,
            selected_compatibility: true,
        }));
    }

    control.checkpoint()?;
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
    validate_summary_header(&summary)?;
    ensure!(
        json(&summary, SOURCE_METADATA_ALLOCATION_BYTES as usize)? == text,
        "Noncanonical source-store summary"
    );
    preflight_rows(connection, &summary)?;
    let raw = raw.context("Source-store marker has no verified legacy catalog")?;
    bind_raw_summary(&summary, raw)?;
    // Full-operation state cannot carry inherited data, even if both sidecar
    // and raw headers were rewritten consistently. The selected branch above
    // performs actual saved-plan/parent verification rather than trusting flags.
    ensure!(
        summary.inherited.is_none(),
        "Inherited sources need a selected run plan"
    );
    control.checkpoint()?;
    reject_duplicates(connection)?;
    control.checkpoint()?;

    let mut totals = SummaryAccumulator::new();
    {
        let mut statement = connection.prepare("SELECT sequence,measurement_id,source_kind,task_url,generation,source_revision,expected_document_id,capture_config_sha256,initial_catalog_config_sha256,data,data_sha256 FROM sitemap_source_measurements ORDER BY sequence")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            control.checkpoint()?;
            let (record, data) = read_record_row(row, 0, &summary.limits)?;
            validate_record(&record, &summary, add(totals.record_count, 1)?)?;
            totals.record(&data)?;
        }
    }
    let mut previous = 0;
    {
        // Joining in SQL preserves O(one bounded record) Rust retention even
        // without assuming the optional measurement-id index is intact. SQLite
        // may allocate a transient index/sort; no query-plan/RSS claim is made.
        let mut statement = connection.prepare("SELECT c.document_id,c.measurement_id,c.document_sequence,c.data,c.data_sha256,m.sequence,m.measurement_id,m.source_kind,m.task_url,m.generation,m.source_revision,m.expected_document_id,m.capture_config_sha256,m.initial_catalog_config_sha256,m.data,m.data_sha256 FROM sitemap_source_measurement_commits c LEFT JOIN sitemap_source_measurements m ON m.measurement_id=c.measurement_id ORDER BY c.document_sequence")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            control.checkpoint()?;
            let (link, data) = read_commit_row(row, 0)?;
            validate_link(&link, previous)?;
            previous = link.document_sequence;
            ensure!(
                row.get::<_, Option<i64>>(5)?.is_some(),
                "Commit has no measurement"
            );
            let (record, _) = read_record_row(row, 5, &summary.limits)?;
            validate_link_record(&link, &record, &summary.limits)?;
            let document = raw
                .document_by_id(&link.document_id)?
                .context("Measurement commit has no actual legacy document")?;
            ensure!(
                document.sequence == link.document_sequence
                    && digest(json(
                        &SitemapLedgerRow::Document(Box::new(document.clone())),
                        1024 * 1024
                    )?) == link.document_payload_sha256
                    && stage_sha(&document)? == record.staged_document_sha256,
                "Measurement commit actual document seal differs"
            );
            bind_record_document(&record.measurement, &document)?;
            totals.commit(&data)?;
        }
    }
    ensure!(
        totals.record_count <= summary.limits.max_records
            && totals.commit_count <= totals.record_count,
        "Source-store record cap exceeded"
    );
    let mut rebuilt = summary.clone();
    totals.finish(&mut rebuilt)?;
    ensure!(
        rebuilt == summary,
        "Source-store summary/canonical hashes or byte accounting differ"
    );
    ensure!(
        summary.charged_json_bytes <= summary.limits.max_catalog_json_bytes as u64,
        "Source-store total byte cap exceeded"
    );
    let context = summary_context(Some(&summary))?;
    Ok(Some(VerifiedSourceRead {
        connection,
        summary,
        context,
        selected_compatibility: false,
    }))
}
