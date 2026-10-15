//! Compact transaction-bound target/inlink analysis; no pretend partial catalog.
use super::*;
use crate::sitemap_ledger::verified_read::VerifiedRawRead;
use crate::sitemap_source_store::verified_read::VerifiedSourceRead;

pub(crate) struct AnalysisReadInput<'a, 'conn> {
    pub(crate) raw: Option<&'a VerifiedRawRead<'conn>>,
    pub(crate) source: Option<&'a VerifiedSourceRead<'conn>>,
    pub(crate) generation: Option<u64>,
    pub(crate) crawl_config_sha256: Option<&'a str>,
    pub(crate) coverage: &'a crate::SitemapCoverage,
}
/// Private fields, non-Clone/non-Serde. Caller must retain the same read TX and
/// perform no same-connection writes/restart while the proof is in use.
pub(crate) struct VerifiedAnalysisRead<'a, 'conn> {
    connection: &'conn Connection,
    raw: Option<&'a VerifiedRawRead<'conn>>,
    coverage: crate::SitemapCoverage,
    prepared: PreparedContext,
    query_index: Option<request_index::RequestIndex>,
}
impl VerifiedAnalysisRead<'_, '_> {
    pub(crate) fn context(&self) -> &SitemapAnalysisContext {
        &self.prepared.base.context
    }
    pub(super) fn query_index(&self) -> Option<&request_index::RequestIndex> {
        self.query_index.as_ref()
    }
    // Only the sibling findings traversal can reuse these actual borrowed
    // proofs. No constructor, Connection or mutable field becomes public.
    pub(super) fn findings_basis(
        &self,
    ) -> (
        &Connection,
        Option<&VerifiedRawRead<'_>>,
        &crate::SitemapCoverage,
        &PreparedContext,
    ) {
        (self.connection, self.raw, &self.coverage, &self.prepared)
    }
}
#[cfg(test)]
pub(crate) fn prepare_window_analysis<'a, 'conn>(
    connection: &'conn Connection,
    input: &AnalysisReadInput<'a, 'conn>,
) -> Result<VerifiedAnalysisRead<'a, 'conn>> {
    prepare_window_analysis_with_control(connection, input, &SitemapReadControl::default())
}
pub(crate) fn prepare_window_analysis_with_control<'a, 'conn>(
    connection: &'conn Connection,
    input: &AnalysisReadInput<'a, 'conn>,
    control: &SitemapReadControl,
) -> Result<VerifiedAnalysisRead<'a, 'conn>> {
    let _binding = control.bind_connection(connection)?;
    control.run(|| prepare_window_analysis_inner(connection, input, control))
}
fn prepare_window_analysis_inner<'a, 'conn>(
    connection: &'conn Connection,
    input: &AnalysisReadInput<'a, 'conn>,
    control: &SitemapReadControl,
) -> Result<VerifiedAnalysisRead<'a, 'conn>> {
    ensure!(
        !connection.is_autocommit(),
        "Sitemap analysis requires a read transaction"
    );
    if let Some(raw) = input.raw {
        ensure!(
            std::ptr::eq(connection, raw.connection()),
            "Analysis raw proof belongs to another connection"
        );
    }
    if let Some(source) = input.source {
        ensure!(
            input.raw.is_some() && std::ptr::eq(connection, source.connection()),
            "Analysis source proof has no same-connection raw basis"
        );
    }
    if let Some(basis) = &input.coverage.basis {
        let summary = input
            .raw
            .context("Analysis coverage has no raw basis")?
            .summary();
        ensure!(
            basis.initial_config_sha256 == summary.config_sha256
                && basis.catalog_revision == summary.revision
                && basis.catalog_rolling_sha256 == summary.rolling_sha256,
            "Analysis coverage/raw basis differs"
        );
    }
    let prepared = prepare_context_with_control(
        connection,
        ContextInput {
            generation: input.generation,
            config_sha: input.crawl_config_sha256,
            coverage: input.coverage,
            summary: input.raw.map(|v| v.summary()),
            source: SourceBasis::Verified(input.source),
        },
        control,
    )?;
    let query_index = match input.raw {
        Some(raw) if prepared.base.context.available => {
            request_index::RequestIndex::prepare(connection, raw, control)?
        }
        _ => None,
    };
    Ok(VerifiedAnalysisRead {
        connection,
        raw: input.raw,
        coverage: input.coverage.clone(),
        prepared,
        query_index,
    })
}

/// Visible rows must come from this proof's Raw reader and exact request, after
/// the caller compares its state/cursor. Saved-page membership is NOT restricted
/// to these rows, their source filter, their page, or a retained prefix.
pub(crate) fn project_prepared_window(
    prepared: &VerifiedAnalysisRead<'_, '_>,
    raw: Option<&VerifiedRawRead<'_>>,
    coverage: &crate::SitemapCoverage,
    visible_rows: &[SitemapLedgerRow],
    request: &SitemapMembershipAnalysisRequest,
) -> Result<SitemapMembershipAnalysis> {
    prepared.prepared.control.checkpoint()?;
    request.validate()?;
    ensure!(
        !prepared.connection.is_autocommit(),
        "Analysis read transaction ended"
    );
    ensure!(
        match (prepared.raw, raw) {
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            (None, None) => true,
            _ => false,
        },
        "Analysis raw proof changed"
    );
    ensure!(
        &prepared.coverage == coverage,
        "Analysis coverage changed after preparation"
    );
    ensure!(
        visible_rows.len() <= request.row_limit
            && visible_rows
                .windows(2)
                .all(|v| v[0].sequence() < v[1].sequence())
            && visible_rows.iter().all(|v| request.matches(v)),
        "Analysis visible selection differs from request"
    );
    ensure!(
        raw.is_some() || visible_rows.is_empty(),
        "Analysis rows have no raw basis"
    );
    let visible = visible_rows
        .iter()
        .filter_map(|r| match r {
            SitemapLedgerRow::Location(loc)
                if loc.observed.role == SitemapLocationRole::PageLocation =>
            {
                Some(loc)
            }
            _ => None,
        })
        .collect();
    project_rows(
        prepared.connection,
        &prepared.prepared,
        coverage,
        visible,
        MembershipBasis::Verified(raw),
        request,
        prepared.query_index.as_ref(),
    )
}

#[derive(Default)]
pub(super) struct WindowCounts {
    pub(super) occurrences: u64,
    pub(super) aliases: u64,
    pub(super) document_count: u64,
    pub(super) observation_count: u64,
    pub(super) samples: Vec<SitemapSourceDocumentSample>,
}
pub(super) enum MembershipBasis<'a> {
    Materialized(&'a [SitemapLedgerRow]),
    Verified(Option<&'a VerifiedRawRead<'a>>),
}
impl MembershipBasis<'_> {
    pub(super) fn counts(
        &self,
        connection: &Connection,
        pages: &[SitemapTargetObservation],
        control: &SitemapReadControl,
        query_index: Option<&request_index::RequestIndex>,
    ) -> Result<BTreeMap<String, WindowCounts>> {
        match self {
            Self::Materialized(rows) => legacy_counts(rows, pages, control),
            Self::Verified(raw) => sql_counts(
                connection,
                raw.context("Saved page membership has no verified raw basis")?,
                pages,
                control,
                query_index,
            ),
        }
    }
}
fn legacy_counts(
    rows: &[SitemapLedgerRow],
    pages: &[SitemapTargetObservation],
    control: &SitemapReadControl,
) -> Result<BTreeMap<String, WindowCounts>> {
    let documents: BTreeMap<_, _> = rows
        .iter()
        .filter_map(|r| match r {
            SitemapLedgerRow::Document(v) => Some((v.observation_id.as_str(), v.as_ref())),
            _ => None,
        })
        .collect();
    let mut members: BTreeMap<_, MembershipCounts<'_>> = pages
        .iter()
        .map(|v| (v.requested_url.clone(), MembershipCounts::default()))
        .collect();
    for row in rows {
        control.checkpoint()?;
        if let SitemapLedgerRow::Location(loc) = row {
            if loc.observed.role != SitemapLocationRole::PageLocation {
                continue;
            }
            if loc.observed.validation_error.is_none()
                && loc.observed.loc_ordinal.is_some()
                && let Some(count) = loc
                    .observed
                    .normalized_fetch_url
                    .as_ref()
                    .and_then(|u| members.get_mut(u))
            {
                count.occurrences += 1;
                let source = documents
                    .get(loc.observation_id.as_str())
                    .context("Original location source document missing")?;
                ensure!(
                    source.kind == crate::SitemapSourceKind::SitemapXml,
                    "Original page location source is not XML"
                );
                count
                    .sources
                    .entry(source.task_url.as_str())
                    .or_default()
                    .insert(source.observation_id.as_str());
            }
            if let Some(alias) = &loc.admission.admitted_fetch_url
                && Some(alias) != loc.observed.normalized_fetch_url.as_ref()
                && let Some(count) = members.get_mut(alias)
            {
                count.aliases += 1;
            }
        }
    }
    members
        .into_iter()
        .map(|(url, counts)| {
            control.checkpoint()?;
            let samples = counts
                .sources
                .iter()
                .take(SAMPLE_LIMIT)
                .map(|(task, ids)| {
                    let id = *ids.first().expect("nonempty source set");
                    SitemapSourceDocumentSample {
                        observation_id: id.into(),
                        task_url: (*task).into(),
                        source_final_url: documents[id]
                            .transport
                            .as_ref()
                            .and_then(|v| v.final_url.clone()),
                    }
                })
                .collect();
            Ok((
                url,
                WindowCounts {
                    occurrences: counts.occurrences,
                    aliases: counts.aliases,
                    document_count: counts.sources.len() as u64,
                    observation_count: counts.sources.values().map(|v| v.len() as u64).sum(),
                    samples,
                },
            ))
        })
        .collect()
}
fn sql_counts(
    connection: &Connection,
    raw: &VerifiedRawRead<'_>,
    pages: &[SitemapTargetObservation],
    control: &SitemapReadControl,
    query_index: Option<&request_index::RequestIndex>,
) -> Result<BTreeMap<String, WindowCounts>> {
    let mut out: BTreeMap<_, WindowCounts> = pages
        .iter()
        .map(|v| (v.requested_url.clone(), WindowCounts::default()))
        .collect();
    if out.is_empty() {
        return Ok(out);
    }
    ensure!(
        out.len() <= 200 && std::ptr::eq(connection, raw.connection()),
        "Invalid membership query basis"
    );
    let source_connection = connection;
    let connection = match query_index {
        Some(index) => index.connection(source_connection)?,
        None => source_connection,
    };
    let _binding = control.bind_connection(connection)?;
    let keys = serde_json::to_string(&out.keys().collect::<Vec<_>>())?;
    // Constant-number whole-cohort SQL scans for the <=200 wanted page URLs;
    // grouping/distinct/window workspace belongs to SQLite, not a Rust source
    // set or an E-times-P Rust scan. All input rows were verified by `raw`.
    let originals = if query_index.is_some() {
        "WITH matches AS (SELECT original_url url,task,observation FROM members WHERE original_url COLLATE BINARY IN (SELECT value FROM json_each(?1)))"
    } else {
        "WITH matches AS (SELECT json_extract(l.data,'$.row.observed.normalized_fetch_url') url,d.task_url task,d.row_id observation FROM sitemap_locations l JOIN sitemap_documents d ON d.row_id=l.observation_id WHERE json_extract(l.data,'$.row.observed.role')='page_location' AND json_extract(l.data,'$.row.observed.validation_error') IS NULL AND json_type(l.data,'$.row.observed.loc_ordinal')='integer' AND json_extract(l.data,'$.row.observed.normalized_fetch_url') IN (SELECT value FROM json_each(?1)))"
    };
    let sql = format!(
        "{originals} SELECT url,count(*),count(DISTINCT task COLLATE BINARY),count(DISTINCT observation COLLATE BINARY) FROM matches GROUP BY url"
    );
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query([&keys])?;
    while let Some(row) = rows.next()? {
        control.checkpoint()?;
        let url: String = row.get(0)?;
        let counts = out
            .get_mut(&url)
            .context("Unknown membership count group")?;
        counts.occurrences = u64::try_from(row.get::<_, i64>(1)?)?;
        counts.document_count = u64::try_from(row.get::<_, i64>(2)?)?;
        counts.observation_count = u64::try_from(row.get::<_, i64>(3)?)?;
    }
    let sql = if query_index.is_some() {
        "SELECT alias_url,count(*) FROM members WHERE alias_url IS NOT normalized_url AND alias_url COLLATE BINARY IN (SELECT value FROM json_each(?1)) GROUP BY alias_url"
    } else {
        "SELECT json_extract(data,'$.row.admission.admitted_fetch_url'),count(*) FROM sitemap_locations WHERE json_extract(data,'$.row.observed.role')='page_location' AND json_extract(data,'$.row.admission.admitted_fetch_url') IS NOT json_extract(data,'$.row.observed.normalized_fetch_url') AND json_extract(data,'$.row.admission.admitted_fetch_url') IN (SELECT value FROM json_each(?1)) GROUP BY json_extract(data,'$.row.admission.admitted_fetch_url')"
    };
    let mut statement = connection.prepare(sql)?;
    let mut rows = statement.query([&keys])?;
    while let Some(row) = rows.next()? {
        control.checkpoint()?;
        let url: String = row.get(0)?;
        out.get_mut(&url)
            .context("Unknown alias count group")?
            .aliases = u64::try_from(row.get::<_, i64>(1)?)?;
    }
    let sql = format!(
        "{originals}, sources AS (SELECT url,task,min(observation COLLATE BINARY) observation FROM matches GROUP BY url COLLATE BINARY,task COLLATE BINARY), ranked AS (SELECT url,task,observation,row_number() OVER (PARTITION BY url ORDER BY task COLLATE BINARY) rank FROM sources) SELECT url,task,observation FROM ranked WHERE rank<=?2 ORDER BY url COLLATE BINARY,rank"
    );
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query(params![keys, SAMPLE_LIMIT as i64])?;
    while let Some(row) = rows.next()? {
        control.checkpoint()?;
        let url: String = row.get(0)?;
        let task: String = row.get(1)?;
        let id: String = row.get(2)?;
        let document = raw
            .document_by_id(&id)?
            .context("Original source document missing")?;
        ensure!(
            document.kind == crate::SitemapSourceKind::SitemapXml && document.task_url == task,
            "Original location source is not bound XML"
        );
        out.get_mut(&url)
            .context("Unknown sample group")?
            .samples
            .push(SitemapSourceDocumentSample {
                observation_id: id,
                task_url: task,
                source_final_url: document.transport.and_then(|v| v.final_url),
            });
    }
    Ok(out)
}
