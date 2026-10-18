//! A complete saved cohort for the dashboard, never a replacement crawl snapshot.
use super::*;
use crate::segments_app::{self as service, Request, View};
use anyhow::{Result, ensure};
use axum::extract::rejection::JsonRejection;
use open_seo_spider_core::{self as core, segments as engine};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
};

const PAGE_BYTES: usize = 32 * 1024 * 1024;
const EXPORT_BYTES: usize = 64 * 1024 * 1024;
const ANALYSIS_WORK_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub(super) struct Failure {
    pub code: String,
    pub message: String,
}
type DResult<T> = Result<T, (StatusCode, Json<Failure>)>;
fn failure(status: StatusCode, code: &str, error: impl ToString) -> (StatusCode, Json<Failure>) {
    (
        status,
        Json(Failure {
            code: code.into(),
            message: error.to_string(),
        }),
    )
}
fn bad(error: impl ToString) -> (StatusCode, Json<Failure>) {
    failure(StatusCode::BAD_REQUEST, "segment_dashboard_refused", error)
}
fn budget(error: impl ToString) -> (StatusCode, Json<Failure>) {
    failure(
        StatusCode::PAYLOAD_TOO_LARGE,
        "segment_dashboard_budget",
        error,
    )
}

/// Bounds bytes while they are encoded. No successful prefix is returned.
struct Bounded {
    bytes: Vec<u8>,
    cap: usize,
}
impl Bounded {
    fn new(cap: usize) -> Self {
        Self {
            bytes: Vec::new(),
            cap,
        }
    }
}
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let required = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| io::Error::other("Segment dashboard output byte budget exceeded"))?;
        if required > self.bytes.capacity() {
            let capacity = self
                .bytes
                .capacity()
                .saturating_mul(2)
                .max(4096)
                .max(required)
                .min(self.cap);
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(|_| io::Error::other("Segment dashboard allocation budget exceeded"))?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn encode(value: &impl Serialize, cap: usize) -> Result<Vec<u8>> {
    let mut out = Bounded::new(cap);
    serde_json::to_writer(&mut out, value)?;
    Ok(out.bytes)
}

#[derive(Serialize)]
struct Membership<'a> {
    matched_indices: &'a [usize],
    unknown_indices: &'a [usize],
}
#[derive(Serialize)]
struct Page<'a> {
    page_ordinal: usize,
    membership: Membership<'a>,
    page: &'a PageRecord,
}
fn page_rows<'a>(report: &'a CrawlReport, rows: &'a [engine::Row]) -> Vec<Page<'a>> {
    rows.iter()
        .map(|row| Page {
            page_ordinal: row.page_ordinal,
            membership: Membership {
                matched_indices: &row.matched_indices,
                unknown_indices: &row.unknown_indices,
            },
            page: &report.pages[row.page_ordinal],
        })
        .collect()
}
#[derive(Default, Serialize)]
struct IssueCounts {
    error: usize,
    warning: usize,
    notice: usize,
}
#[derive(Default, Serialize)]
struct Overview {
    success: usize,
    redirect: usize,
    client_error: usize,
    server_error: usize,
    unfetched: usize,
}
#[derive(Serialize)]
struct Summary {
    total_pages: usize,
    selected_pages: usize,
    selected_issues: usize,
    multiple_matches: usize,
    unassigned_pages: usize,
    unknown_pages: usize,
    orphan_issues: usize,
    internal_pages: usize,
    html_pages: usize,
    indexable_pages: usize,
    failed_pages: usize,
    issue_counts: IssueCounts,
}
#[derive(Serialize)]
struct Charts<'a> {
    status: &'a BTreeMap<String, usize>,
    depth: &'a BTreeMap<usize, usize>,
    effective_status: BTreeMap<String, usize>,
    overview: Overview,
}
#[derive(Serialize)]
struct Cohort<'a> {
    engine: &'static str,
    depth_policy: &'static str,
    rules: &'a engine::Rules,
    selection: &'a engine::Selection,
    summary: Summary,
    counts: &'a [engine::Count],
    charts: Charts<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reporting: Option<&'a engine::Reporting>,
    scope: &'static str,
}
fn cohort<'a>(report: &CrawlReport, e: &'a service::Evaluation) -> Cohort<'a> {
    let p = &e.projection;
    let mut summary = Summary {
        total_pages: p.total_pages,
        selected_pages: p.selected_pages,
        selected_issues: p.issues.len(),
        multiple_matches: p.multiple_matches,
        unassigned_pages: p.unassigned_pages,
        unknown_pages: p.unknown_pages,
        orphan_issues: p.orphan_issues,
        internal_pages: 0,
        html_pages: 0,
        indexable_pages: 0,
        failed_pages: 0,
        issue_counts: IssueCounts::default(),
    };
    let mut effective_status = BTreeMap::new();
    let mut overview = Overview::default();
    for row in &p.pages {
        let page = &report.pages[row.page_ordinal];
        summary.internal_pages += usize::from(page.internal);
        summary.html_pages += usize::from(page.internal && page.is_html());
        summary.indexable_pages += usize::from(page.indexable);
        summary.failed_pages += usize::from(page.error.is_some());
        let status = page
            .final_status
            .or(page.status)
            .filter(|v| (100..=999).contains(v));
        *effective_status
            .entry(
                status
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "unknown".into()),
            )
            .or_default() += 1;
        if page.internal {
            overview.success += usize::from(status.is_some_and(|v| (200..300).contains(&v)));
            overview.redirect += usize::from(
                page.status.is_some_and(|v| (300..400).contains(&v))
                    || !page.redirect_chain.is_empty(),
            );
            overview.client_error += usize::from(status.is_some_and(|v| (400..500).contains(&v)));
            overview.server_error += usize::from(status.is_some_and(|v| v >= 500));
            overview.unfetched +=
                usize::from(page.blocked_by_robots || page.error.is_some() || status.is_none());
        }
    }
    for issue in &p.issues {
        match issue.severity {
            core::Severity::Error => summary.issue_counts.error += 1,
            core::Severity::Warning => summary.issue_counts.warning += 1,
            core::Severity::Notice => summary.issue_counts.notice += 1,
        }
    }
    Cohort {
        engine: engine::ENGINE,
        depth_policy: engine::DEPTH_POLICY,
        rules: &p.rules,
        selection: &p.selection,
        summary,
        counts: &p.counts,
        charts: Charts {
            status: &p.charts.status,
            depth: &p.charts.depth,
            effective_status,
            overview,
        },
        reporting: p.reporting.as_ref(),
        scope: "Selected original page observations; issue ownership uses exact requested URLs. Initial status and effective final-or-initial status charts are distinct. Overview buckets overlap and use internal observations. Crawl activity/counters are unchanged. True orphan issues have no selected page owner and are counted separately. Missing predicate values remain unknown, never an inferred zero.",
    }
}
#[derive(Serialize)]
#[serde(untagged)]
enum Rows<'a> {
    Pages(Vec<Page<'a>>),
    Issues(&'a [AuditIssue]),
}
#[derive(Serialize)]
struct PageReply<'a> {
    context: &'a service::Context,
    cohort: Cohort<'a>,
    captured_crawl_summary: &'a core::CrawlSummary,
    view: View,
    total: usize,
    rows: Rows<'a>,
    next: Option<service::Cursor>,
}
fn paged(
    report: &CrawlReport,
    e: &service::Evaluation,
    request: &Request,
    cap: usize,
) -> Result<Vec<u8>> {
    let total = match request.view {
        View::Pages => e.projection.pages.len(),
        View::Issues => e.projection.issues.len(),
    };
    let offset = request.cursor.as_ref().map_or(0, |c| c.offset);
    ensure!(offset <= total, "Cursor offset exceeds selected cohort");
    let end = offset.saturating_add(request.limit).min(total);
    let rows = match request.view {
        View::Pages => Rows::Pages(page_rows(report, &e.projection.pages[offset..end])),
        View::Issues => Rows::Issues(&e.projection.issues[offset..end]),
    };
    encode(
        &PageReply {
            context: &e.context,
            cohort: cohort(report, e),
            captured_crawl_summary: &report.summary,
            view: request.view,
            total,
            rows,
            next: (end < total).then(|| service::Cursor {
                offset: end,
                view: request.view,
                snapshot_token: e.context.snapshot_token.clone(),
            }),
        },
        cap,
    )
}
#[derive(Serialize)]
struct FullReply<'a> {
    kind: &'static str,
    context: &'a service::Context,
    cohort: Cohort<'a>,
    captured_config: &'a CrawlConfig,
    captured_crawl_summary: &'a core::CrawlSummary,
    pages: Vec<Page<'a>>,
    issues: &'a [AuditIssue],
}
fn full(report: &CrawlReport, e: &service::Evaluation, cap: usize) -> Result<Vec<u8>> {
    encode(
        &FullReply {
            kind: "segment_dashboard_cohort_not_raw_crawl",
            context: &e.context,
            cohort: cohort(report, e),
            captured_config: &report.config,
            captured_crawl_summary: &report.summary,
            pages: page_rows(report, &e.projection.pages),
            issues: &e.projection.issues,
        },
        cap,
    )
}
fn owners(e: &service::Evaluation) -> BTreeSet<&str> {
    e.projection
        .pages
        .iter()
        .map(|row| row.url.as_str())
        .collect()
}

// A conservative work proxy, not a CPU deadline or a universal RSS guarantee.
// Analysis repeats owner/document addresses in diagnostics, graph edges, and
// fragment/relationship rows. Charge all declared roles before deriving them.
fn analysis_preflight(report: &CrawlReport, cap: usize) -> Result<()> {
    let mut work = 0usize;
    let mut fragment_occurrences = 0usize;
    let mut largest_fragment_target = 0usize;
    let mut largest_document_url = 0usize;
    let mut charge = |amount: usize| -> Result<()> {
        work = work
            .checked_add(amount)
            .filter(|n| *n <= cap)
            .ok_or_else(|| {
                anyhow::anyhow!("Original analysis work byte budget exceeded before derivation")
            })?;
        Ok(())
    };
    for page in &report.pages {
        largest_document_url = largest_document_url.max(page.document_url().len());
        let owner = page
            .url
            .len()
            .saturating_add(page.final_url.len())
            .saturating_add(page.document_url().len())
            .max(1);
        let roles = page
            .links
            .len()
            .saturating_mul(8)
            .saturating_add(page.redirect_chain.len().saturating_mul(8))
            .saturating_add(page.link_relationships.len().saturating_mul(16))
            .saturating_add(
                page.hreflang
                    .len()
                    .saturating_add(page.html_hreflang.len())
                    .saturating_add(page.header_hreflang.len())
                    .saturating_mul(16),
            )
            .saturating_add(
                page.canonical
                    .len()
                    .saturating_add(page.html_canonical.len())
                    .saturating_add(page.header_canonical.len())
                    .saturating_mul(8),
            )
            .saturating_add(
                page.pagination_next
                    .len()
                    .saturating_add(page.pagination_prev.len())
                    .saturating_add(page.amphtml.len())
                    .saturating_mul(16),
            )
            .saturating_add(page.headers.len().saturating_mul(8))
            .saturating_add(32);
        charge(
            owner
                .checked_mul(roles)
                .ok_or_else(|| anyhow::anyhow!("Original analysis work overflow"))?,
        )?;
        for link in &page.links {
            if report.config.validate_fragments && link.kind == "anchor" && link.fragment.is_some()
            {
                fragment_occurrences = fragment_occurrences.checked_add(1).ok_or_else(|| {
                    anyhow::anyhow!("Original fragment occurrence budget overflow")
                })?;
            }
            charge(
                link.url
                    .len()
                    .saturating_add(link.fragment.as_ref().map_or(0, String::len))
                    .saturating_mul(16),
            )?;
        }
        for hop in &page.redirect_chain {
            charge(
                hop.url
                    .len()
                    .saturating_add(hop.target.len())
                    .saturating_mul(8),
            )?;
        }
        for declaration in page
            .hreflang
            .iter()
            .chain(&page.html_hreflang)
            .chain(&page.header_hreflang)
        {
            charge(
                declaration
                    .url
                    .len()
                    .saturating_add(declaration.language.len())
                    .saturating_mul(16),
            )?;
        }
        for declaration in page
            .canonical
            .iter()
            .chain(&page.html_canonical)
            .chain(&page.header_canonical)
            .chain(&page.pagination_next)
            .chain(&page.pagination_prev)
            .chain(&page.amphtml)
        {
            charge(declaration.len().saturating_mul(16))?;
        }
        for (key, value) in &page.headers {
            charge(key.len().saturating_add(value.len()).saturating_mul(8))?;
        }
        for declaration in &page.link_relationships {
            let mut counter = Count { bytes: 0, cap };
            serde_json::to_writer(&mut counter, declaration)?;
            charge(counter.bytes.saturating_mul(16))?;
        }
        if let Some(dom) = &page.dom_metadata {
            for target in &dom.fragment_targets {
                let mut counter = Count { bytes: 0, cap };
                serde_json::to_writer(&mut counter, target)?;
                largest_fragment_target = largest_fragment_target.max(counter.bytes);
            }
        }
        for value in &page.structured_data {
            let mut counter = Count { bytes: 0, cap };
            serde_json::to_writer(&mut counter, value)?;
            charge(counter.bytes.saturating_mul(4))?;
        }
    }
    // Each resolved occurrence may copy the complete target DOM location and
    // target document address, not only the short fragment identifier. The
    // largest target is an intentional overestimate; no matching predicate is
    // rerun here and no first-N prefix is substituted for the full analysis.
    let fragment_payload = largest_fragment_target
        .checked_add(largest_document_url)
        .and_then(|n| n.checked_add(1024))
        .and_then(|n| n.checked_mul(fragment_occurrences))
        .ok_or_else(|| anyhow::anyhow!("Original fragment fanout work overflow"))?;
    charge(fragment_payload)?;
    grouped_analysis_work(report, &mut charge)?;
    Ok(())
}

/// Charge actual declared groups/cycles rather than all pages squared. Alias
/// lookup uses the analyzer's first-observation/direct-request precedence. The
/// graph includes unusable declarations too, so it is a conservative superset;
/// it does not produce diagnoses or change segment/analysis predicates.
fn grouped_analysis_work(
    report: &CrawlReport,
    charge: &mut impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    let mut pages = BTreeMap::new();
    let mut keywords: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for page in &report.pages {
        pages.entry(page.url.as_str()).or_insert(page);
        if page.indexable
            && let Some(dom) = &page.dom_metadata
            && dom.complete
        {
            for record in &dom.meta_keywords {
                if !record.normalized.is_empty() {
                    keywords
                        .entry(record.normalized.as_str())
                        .or_default()
                        .insert(page.url.as_str());
                }
            }
        }
    }
    for (text, urls) in keywords.into_iter().filter(|(_, urls)| urls.len() > 1) {
        let url_bytes = urls
            .iter()
            .try_fold(0usize, |sum, url| {
                sum.checked_add(url.len().checked_mul(6)?.checked_add(64)?)
            })
            .ok_or_else(|| anyhow::anyhow!("Original keyword URL fanout overflow"))?;
        let payload = url_bytes
            .checked_mul(urls.len() - 1)
            .and_then(|n| {
                text.len()
                    .checked_mul(6)?
                    .checked_add(1024)?
                    .checked_mul(urls.len())?
                    .checked_add(n)
            })
            .ok_or_else(|| anyhow::anyhow!("Original keyword group work overflow"))?;
        charge(payload)?;
    }
    let mut lookup = pages.clone();
    for page in pages.values() {
        if !page.final_url.is_empty() {
            lookup.entry(page.final_url.as_str()).or_insert(*page);
        }
        if let Some(address) = page
            .rendered_url
            .as_deref()
            .filter(|_| page.rendered && page.renderer_error.is_none())
        {
            lookup.entry(address).or_insert(*page);
        }
    }
    let mut next = BTreeMap::new();
    for page in pages.values() {
        let targets: BTreeSet<_> = page.canonical.iter().map(String::as_str).collect();
        if targets.len() == 1
            && let Some(target) = lookup.get(*targets.iter().next().unwrap())
        {
            next.insert(page.url.as_str(), target.url.as_str());
        }
    }
    let mut finished = BTreeSet::new();
    for &start in next.keys() {
        if finished.contains(start) {
            continue;
        }
        let mut positions = BTreeMap::new();
        let mut path = Vec::new();
        let mut current = start;
        while !finished.contains(current) {
            if let Some(&position) = positions.get(current) {
                let cycle: &[&str] = &path[position..];
                let bytes = cycle
                    .iter()
                    .try_fold(0usize, |sum, url| {
                        sum.checked_add(url.len().checked_mul(6)?.checked_add(64)?)
                    })
                    .and_then(|n| n.checked_mul(cycle.len()))
                    .ok_or_else(|| anyhow::anyhow!("Original canonical cycle work overflow"))?;
                charge(bytes)?;
                break;
            }
            positions.insert(current, path.len());
            path.push(current);
            let Some(&target) = next.get(current) else {
                break;
            };
            current = target;
        }
        finished.extend(path);
    }
    Ok(())
}
struct Count {
    bytes: usize,
    cap: usize,
}
impl Write for Count {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| io::Error::other("Original analysis payload budget exceeded"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn derive_analysis(
    report: &CrawlReport,
    cap: usize,
    derive: impl FnOnce(&CrawlReport) -> Value,
) -> Result<Value> {
    analysis_preflight(report, cap)?;
    Ok(derive(report))
}
fn filtered_analysis(report: &CrawlReport, e: &service::Evaluation) -> Result<Value> {
    let mut value = derive_analysis(report, ANALYSIS_WORK_BYTES, crate::analysis::analyze_report)?;
    let selected = owners(e);
    let keep = |row: &Value, key: &str| row[key].as_str().is_some_and(|url| selected.contains(url));
    for key in ["issues", "structured_data"] {
        if let Some(rows) = value[key].as_array_mut() {
            rows.retain(|row| keep(row, "url"));
        }
    }
    for key in ["nodes", "orphan_candidates"] {
        if let Some(rows) = value["graph"][key].as_array_mut() {
            rows.retain(|row| keep(row, "url"));
        }
    }
    if let Some(rows) = value["graph"]["edges"].as_array_mut() {
        rows.retain(|row| keep(row, "source"));
    }
    if let Some(rows) = value["fragments"]["observations"].as_array_mut() {
        rows.retain(|row| keep(row, "source_url"));
    }
    if let Some(cycles) = value["canonical_cycles"].as_array_mut() {
        cycles.retain(|cycle| {
            cycle.as_array().is_some_and(|urls| {
                urls.iter()
                    .any(|url| url.as_str().is_some_and(|url| selected.contains(url)))
            })
        });
    }
    value["segment_basis"] = json!({
        "membership":"original selected page ordinals; analysis owners use exact requested URL",
        "duplicate_url_analysis":"existing analyzer uses the first captured observation for each exact URL; this does not change page-observation membership",
        "graph_metrics":"computed on complete captured report before any profile or segment presentation filter; no induced segment graph",
        "relationships":"selected source owners retain complete captured target references, including outside the cohort",
        "coverage_and_depth_distribution":"original capture scope, not selected totals",
        "issues":"additional independently derived source-owned issues; saved issues are returned separately, never silently counted twice",
        "true_orphan_issues":"no selected page owner; separate cohort orphan count",
        "selected_graph_nodes":value["graph"]["nodes"].as_array().map_or(0, Vec::len),
        "selected_source_edges":value["graph"]["edges"].as_array().map_or(0, Vec::len),
        "selected_orphan_candidates":value["graph"]["orphan_candidates"].as_array().map_or(0, Vec::len),
        "work_proxy_bytes":ANALYSIS_WORK_BYTES,"universal_memory_bound_proven":false,
        "website_requests":0,"raw_snapshot_modified":false
    });
    Ok(value)
}
#[derive(Serialize)]
struct Target<'a> {
    page_ordinal: usize,
    url: &'a str,
    final_url: &'a str,
    rendered_url: &'a Option<String>,
    status: Option<u16>,
    final_status: Option<u16>,
    content_type: &'a str,
    internal: bool,
    image_metadata: &'a Option<core::ImageResourceMetadata>,
}
#[derive(Serialize)]
struct Fragments {
    enabled: bool,
    observations: Vec<core::FragmentObservation>,
    scope: &'static str,
    unknown_is_not_missing: bool,
}
fn ordinals(e: &service::Evaluation) -> BTreeSet<usize> {
    e.projection
        .pages
        .iter()
        .map(|row| row.page_ordinal)
        .collect()
}
fn fragments(
    report: &CrawlReport,
    e: &service::Evaluation,
) -> Result<Vec<core::FragmentObservation>> {
    let mut rows = Vec::new();
    core::visit_fragment_observations(report, Some(&ordinals(e)), |row| {
        rows.push(row);
        Ok(())
    })?;
    Ok(rows)
}
fn selected_report(report: &CrawlReport, e: &service::Evaluation) -> CrawlReport {
    let mut config = report.config.clone();
    // Membership already applied the explicit raw/reported view. Encoding must
    // neither apply that profile a second time nor relabel cohort as capture.
    config.directive_reporting = None;
    CrawlReport {
        config,
        pages: e
            .projection
            .pages
            .iter()
            .map(|r| report.pages[r.page_ordinal].clone())
            .collect(),
        issues: e.projection.issues.clone(),
        summary: report.summary.clone(),
        operation: report.operation.clone(),
        ..Default::default()
    }
}
#[derive(Serialize)]
struct AnalysisReply<'a> {
    context: &'a service::Context,
    cohort: Cohort<'a>,
    captured_crawl_summary: &'a core::CrawlSummary,
    analysis: Value,
    target_lookup: Vec<Target<'a>>,
    responses: Vec<core::ResponseObservation>,
    fragments: Fragments,
}
fn analysis(report: &CrawlReport, e: &service::Evaluation, cap: usize) -> Result<Vec<u8>> {
    let value = filtered_analysis(report, e)?;
    let selected = selected_report(report, e);
    encode(
        &AnalysisReply {
            context: &e.context,
            cohort: cohort(report, e),
            captured_crawl_summary: &report.summary,
            analysis: value,
            target_lookup: report
                .pages
                .iter()
                .enumerate()
                .map(|(page_ordinal, page)| Target {
                    page_ordinal,
                    url: &page.url,
                    final_url: &page.final_url,
                    rendered_url: &page.rendered_url,
                    status: page.status,
                    final_status: page.final_status,
                    content_type: &page.content_type,
                    internal: page.internal,
                    image_metadata: &page.image_metadata,
                })
                .collect(),
            responses: core::response_observations(&selected),
            fragments: Fragments {
                enabled: report.config.validate_fragments,
                observations: fragments(report, e)?,
                scope: "original captured target validation with selected raw source page ordinals",
                unknown_is_not_missing: true,
            },
        },
        cap,
    )
}

#[derive(Clone, Copy)]
enum CsvKind {
    Pages,
    Issues,
    Links,
    Dom,
    Extractions,
    Javascript,
    Cookies,
    Pdf,
    Images,
    CustomSearch,
    Accessibility,
    Responses,
    Fragments,
}
impl CsvKind {
    fn parse(kind: &str) -> Result<Self> {
        Ok(match kind {
            "pages.csv" => Self::Pages,
            "issues.csv" => Self::Issues,
            "links.csv" => Self::Links,
            "dom.csv" => Self::Dom,
            "extractions.csv" => Self::Extractions,
            "javascript.csv" => Self::Javascript,
            "cookies.csv" => Self::Cookies,
            "pdf.csv" => Self::Pdf,
            "images.csv" => Self::Images,
            "custom-search.csv" => Self::CustomSearch,
            "accessibility.csv" => Self::Accessibility,
            "responses.csv" => Self::Responses,
            "fragments.csv" => Self::Fragments,
            _ => anyhow::bail!("Unsupported page-owned segment CSV kind"),
        })
    }
}
fn extra(report: &CrawlReport, e: &service::Evaluation) -> Result<Vec<(String, String)>> {
    let pairs = [
        ("Segment Snapshot Token", e.context.snapshot_token.clone()),
        ("Segment Native Generation", e.context.app_generation.map(|v| v.to_string()).unwrap_or_default()),
        ("Segment Typed Report SHA256", e.context.typed_report_sha256.clone()),
        ("Segment Rules SHA256", e.context.rules_sha256.clone()),
        ("Segment Selection JSON", serde_json::to_string(&e.projection.selection)?),
        ("Segment Reporting View JSON", serde_json::to_string(&e.context.reporting_view)?),
        ("Segment Captured Pages", report.pages.len().to_string()),
        ("Segment Eligible Pages", e.projection.total_pages.to_string()),
        ("Segment Selected Pages", e.projection.selected_pages.to_string()),
        ("Segment Captured Issues", report.issues.len().to_string()),
        ("Segment Selected Issues", e.projection.issues.len().to_string()),
        ("Segment Orphan Issues", e.projection.orphan_issues.to_string()),
        ("Segment Scope", "selected saved observations; crawl activity and relationship targets unchanged; no SQL or origin attestation".into()),
    ];
    Ok(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn csv_bytes(
    report: &CrawlReport,
    e: &service::Evaluation,
    kind: CsvKind,
    cap: usize,
) -> Result<Vec<u8>> {
    let selected = selected_report(report, e);
    let extra = extra(report, e)?;
    let mut out = Bounded::new(cap);
    match kind {
        CsvKind::Pages => {
            core::write_pages_csv(&selected, &mut out, &extra)?;
        }
        CsvKind::Issues => {
            core::write_issues_csv(&selected, &mut out, &extra)?;
        }
        CsvKind::Links => {
            core::write_links_csv(&selected, &mut out, &extra)?;
        }
        CsvKind::Dom => {
            core::write_dom_observations_csv(&selected, &mut out, &extra)?;
        }
        CsvKind::Extractions => {
            core::write_custom_extractions_csv(&selected, &mut out, &extra)?;
        }
        CsvKind::Javascript => {
            core::write_custom_javascript_csv(&selected, &mut out, &extra)?;
        }
        CsvKind::Cookies => {
            core::write_browser_cookies_csv(&selected, &mut out, &extra)?;
        }
        CsvKind::Pdf => {
            core::write_pdf_csv(&selected, &mut out, &extra)?;
        }
        CsvKind::Images => {
            core::write_image_resources_csv(&selected, &mut out, &extra)?;
        }
        CsvKind::CustomSearch => {
            core::write_custom_search_csv(&selected, &mut out, &extra)?;
        }
        CsvKind::Accessibility => {
            let visible = report
                .pages
                .iter()
                .filter(|p| core::directive_page_visible(&report.config, p))
                .count();
            accessibility::write_cohort_csv(
                &selected,
                &mut out,
                &extra,
                report.pages.len(),
                visible,
            )?;
        }
        CsvKind::Responses => {
            core::write_response_observations_csv(&selected, &mut out, &extra)?;
        }
        CsvKind::Fragments => {
            core::write_selected_fragments_csv(report, &ordinals(e), &mut out, &extra)?;
        }
    }
    Ok(out.bytes)
}
#[derive(Clone, Copy)]
enum Operation {
    Page,
    Analysis,
    Json,
    Csv(CsvKind),
}
fn response(bytes: Vec<u8>, context: &service::Context, operation: Operation) -> Response {
    let csv = matches!(operation, Operation::Csv(_));
    let mut response = Response::builder()
        .header(
            header::CONTENT_TYPE,
            if csv {
                "text/csv; charset=utf-8"
            } else {
                "application/json; charset=utf-8"
            },
        )
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::CONTENT_LENGTH, bytes.len())
        .header("x-content-sha256", service::sha(&bytes))
        .header("x-segment-snapshot-token", &context.snapshot_token)
        .header(
            "x-segment-generation",
            context
                .app_generation
                .map(|v| v.to_string())
                .unwrap_or_default(),
        );
    if matches!(operation, Operation::Json | Operation::Csv(_)) {
        response = response.header(
            header::CONTENT_DISPOSITION,
            if csv {
                "attachment; filename=segment-dashboard.csv"
            } else {
                "attachment; filename=segment-dashboard.json"
            },
        );
    }
    response
        .body(axum::body::Body::from(bytes))
        .expect("fixed safe response headers")
}
async fn read(
    state: AppState,
    id: Uuid,
    request: Result<Json<Request>, JsonRejection>,
    operation: Operation,
) -> DResult<Response> {
    let Json(request) = request.map_err(bad)?;
    request.validate().map_err(bad)?;
    if !matches!(operation, Operation::Page)
        && (request.expected_snapshot_token.is_none() || request.cursor.is_some())
    {
        return Err(bad(
            "Complete cohort analysis/export requires the preview snapshot token and no page cursor",
        ));
    }
    let job = get_job(&state, id)
        .await
        .map_err(|(s, Json(e))| failure(s, "segment_dashboard_job_not_found", e.error))?;
    let guard = job.read_owned().await;
    state
        .lifecycle
        .blocking
        .run_blocking(move || {
            let e = segments::evaluate(&guard, &request)
                .map_err(|(s, Json(e))| failure(s, &e.code, e.message))?;
            let report = guard.report.as_ref().expect("evaluated stable report");
            if matches!(operation, Operation::Page) {
                let total = match request.view {
                    View::Pages => e.projection.pages.len(),
                    View::Issues => e.projection.issues.len(),
                };
                if request
                    .cursor
                    .as_ref()
                    .is_some_and(|cursor| cursor.offset > total)
                {
                    return Err(bad("Cursor offset exceeds selected cohort"));
                }
            }
            let bytes = match operation {
                Operation::Page => paged(report, &e, &request, PAGE_BYTES),
                Operation::Analysis => analysis(report, &e, PAGE_BYTES),
                Operation::Json => full(report, &e, EXPORT_BYTES),
                Operation::Csv(kind) => csv_bytes(report, &e, kind, EXPORT_BYTES),
            }
            .map_err(budget)?;
            Ok(response(bytes, &e.context, operation))
        })
        .await
        .map_err(|e| {
            failure(
                StatusCode::SERVICE_UNAVAILABLE,
                "segment_dashboard_worker_unavailable",
                e,
            )
        })?
}
pub(super) async fn preview(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<Request>, JsonRejection>,
) -> DResult<Response> {
    read(state, id, request, Operation::Page).await
}
pub(super) async fn analyze(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<Request>, JsonRejection>,
) -> DResult<Response> {
    read(state, id, request, Operation::Analysis).await
}
pub(super) async fn json(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<Request>, JsonRejection>,
) -> DResult<Response> {
    read(state, id, request, Operation::Json).await
}
pub(super) async fn csv(
    State(state): State<AppState>,
    Path((id, kind)): Path<(Uuid, String)>,
    request: Result<Json<Request>, JsonRejection>,
) -> DResult<Response> {
    let kind = CsvKind::parse(&kind).map_err(bad)?;
    read(state, id, request, Operation::Csv(kind)).await
}

#[cfg(test)]
#[path = "segment_dashboard_tests.rs"]
mod tests;
