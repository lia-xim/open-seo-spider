//! Offline occurrence-preserving projection over a bounded App-declared saved report.
//! This is not the complete CSS/resource ledger and never performs website IO.
use crate::migration_app::{Failure, digest, fail};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::DateTime;
use open_seo_spider_core::{
    ComparisonCallerBinding, CrawlReport, LinkRecord, PageRecord, RecrawlTargetObservation,
    RecrawlTargetState,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
};
use uuid::Uuid;

pub(crate) const SCOPE: &str = "retained_page_links_only";
pub(crate) const REQUEST_BYTES: usize = 4096;
pub(crate) const REPORT_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const JSON_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const CSV_BYTES: usize = 64 * 1024 * 1024;
const MAX_PAGES: usize = 50_000;
const MAX_LINKS: usize = 250_000;
const MAX_JOINED_OBSERVATIONS: usize = 500_000;
type Result<T> = std::result::Result<T, Failure>;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Verdict {
    ObservedHttpFailure,
    Unknown,
    ObservedNonFailure,
}
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Filter {
    #[default]
    Actionable,
    Broken,
    Unknown,
    All,
}
impl Filter {
    fn includes(self, v: Verdict) -> bool {
        match self {
            Self::Actionable => v != Verdict::ObservedNonFailure,
            Self::Broken => v == Verdict::ObservedHttpFailure,
            Self::Unknown => v == Verdict::Unknown,
            Self::All => true,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Request {
    pub filter: Filter,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub expected_generation: Option<u64>,
    pub expected_snapshot_token: Option<String>,
    pub cursor: Option<String>,
}
fn default_limit() -> usize {
    100
}
impl Default for Request {
    fn default() -> Self {
        Self {
            filter: Filter::default(),
            limit: default_limit(),
            expected_generation: None,
            expected_snapshot_token: None,
            cursor: None,
        }
    }
}
impl Request {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.limit == 0 || self.limit > 200 {
            return Err(fail("invalid_page_limit", "Page limit must be 1..=200"));
        }
        if self.cursor.as_ref().is_some_and(|s| s.len() > 2048)
            || self
                .expected_snapshot_token
                .as_ref()
                .is_some_and(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err(fail(
                "invalid_continuation",
                "Invalid bounded cursor or snapshot token",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Snapshot {
    pub job_id: Uuid,
    pub app_generation: u64,
    pub status: String,
    pub typed_report_sha256: String,
    pub typed_report_bytes: usize,
    pub job_config_sha256: String,
    pub report_config_sha256: String,
    pub caller_binding: ComparisonCallerBinding,
    pub hash_profile: &'static str,
    pub sql_attested: bool,
    pub complete_website_asserted: bool,
}
impl Snapshot {
    pub(crate) fn new(
        report: &CrawlReport,
        id: Uuid,
        generation: u64,
        status: String,
        job_config_sha256: String,
        caller_binding: ComparisonCallerBinding,
    ) -> Result<Self> {
        let (typed_report_sha256, typed_report_bytes) = digest(report, REPORT_BYTES)?;
        Ok(Self {
            job_id: id,
            app_generation: generation,
            status,
            typed_report_sha256,
            typed_report_bytes,
            job_config_sha256,
            report_config_sha256: digest(&report.config, REPORT_BYTES)?.0,
            caller_binding,
            hash_profile: "app_broken_sources_typed_crawl_report_compact_serde_json_v1",
            sql_attested: false,
            complete_website_asserted: false,
        })
    }
}
// Count before the comparison authority clones the typed report. These are
// independent input/row/output caps, not an aggregate process-memory guarantee.
pub(crate) fn preflight(report: &CrawlReport) -> Result<()> {
    if report.pages.len() > MAX_PAGES
        || report
            .operation
            .as_ref()
            .is_some_and(|o| o.targets.len() > MAX_PAGES)
    {
        return Err(fail(
            "page_count_budget",
            "Saved page/selected-target count exceeds 50000",
        ));
    }
    let mut count = 0usize;
    for p in &report.pages {
        count = count
            .checked_add(p.links.len())
            .filter(|n| *n <= MAX_LINKS)
            .ok_or_else(|| {
                fail(
                    "link_count_budget",
                    "Retained links exceed 250000; no occurrences were silently removed",
                )
            })?;
    }
    digest(report, REPORT_BYTES)?;
    Ok(())
}
#[derive(Clone, Copy)]
struct Fact {
    age: Option<&'static str>,
    coherent: bool,
}
struct RowIndex {
    source: usize,
    link: usize,
    verdict: Verdict,
    reason: &'static str,
}
pub(crate) struct Evaluation {
    report: CrawlReport,
    pub snapshot: Snapshot,
    pub token: String,
    filter: Filter,
    targets: BTreeMap<String, Vec<usize>>,
    selected: BTreeMap<String, Vec<usize>>,
    facts: Vec<Fact>,
    rows: Vec<RowIndex>,
    summary: BTreeMap<Verdict, usize>,
}
fn http_identity(value: &str) -> Option<String> {
    open_seo_spider_core::normalize_url(value, None).map(|v| v.to_string())
}
fn coherent_target(page: &PageRecord) -> bool {
    let (Some(initial), Some(final_status)) = (page.status, page.final_status) else {
        return false;
    };
    if !(100..=599).contains(&initial) || !(100..=599).contains(&final_status) {
        return false;
    }
    let Some(mut current) = http_identity(&page.url) else {
        return false;
    };
    let Some(final_url) = http_identity(&page.final_url) else {
        return false;
    };
    let mut seen = BTreeSet::from([current.clone()]);
    for (i, hop) in page.redirect_chain.iter().enumerate() {
        if ![301, 302, 303, 307, 308].contains(&hop.status)
            || http_identity(&hop.url).as_deref() != Some(current.as_str())
            || i == 0 && hop.status != initial
        {
            return false;
        }
        let Some(target) = http_identity(&hop.target) else {
            return false;
        };
        if !seen.insert(target.clone()) {
            return false;
        }
        current = target;
    }
    current == final_url && (!page.redirect_chain.is_empty() || initial == final_status)
}
fn age_reason(
    report: &CrawlReport,
    page: &PageRecord,
    selected: &BTreeMap<String, Vec<usize>>,
    snapshot: &Snapshot,
) -> Option<&'static str> {
    if snapshot.caller_binding.origin.as_deref() == Some("app_declared_imported_or_detached_report")
        || snapshot.app_generation == 0
    {
        return Some("imported_or_detached_age_unknown");
    }
    if snapshot.job_config_sha256 != snapshot.report_config_sha256 {
        return Some("saved_report_config_differs");
    }
    if let Some(operation) = &report.operation {
        let Some(indices) = selected.get(&page.url) else {
            return Some("inherited_selected_observation");
        };
        if indices.len() != 1 {
            return Some("ambiguous_selected_target_state");
        }
        let t = &operation.targets[indices[0]];
        if t.state != RecrawlTargetState::Committed
            || t.committed_generation.is_none()
            || t.admitted_attempts == 0
        {
            return Some("selected_target_not_committed");
        }
        // Core committed_generation and App generation are distinct authorities.
    }
    let (Ok(start), Ok(end), Ok(at)) = (
        DateTime::parse_from_rfc3339(&report.summary.started_at),
        DateTime::parse_from_rfc3339(&report.summary.finished_at),
        DateTime::parse_from_rfc3339(&page.fetched_at),
    ) else {
        return Some("saved_run_time_unknown");
    };
    if end < start || at < start || at > end {
        return Some("outside_saved_run_time_window");
    };
    None
}
pub(crate) fn evaluate(
    report: CrawlReport,
    snapshot: Snapshot,
    filter: Filter,
) -> Result<Evaluation> {
    preflight(&report)?;
    let mut targets: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, p) in report.pages.iter().enumerate() {
        targets.entry(p.url.clone()).or_default().push(i);
    }
    let mut selected: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    if let Some(o) = &report.operation {
        for (i, t) in o.targets.iter().enumerate() {
            selected.entry(t.requested_url.clone()).or_default().push(i);
        }
    }
    let facts = report
        .pages
        .iter()
        .map(|p| Fact {
            age: age_reason(&report, p, &selected, &snapshot),
            coherent: coherent_target(p),
        })
        .collect::<Vec<_>>();
    let mut weights: BTreeMap<String, usize> = BTreeMap::new();
    for (key, indices) in &targets {
        let weight = indices
            .len()
            .checked_mul(1 + selected.get(key).map_or(0, Vec::len))
            .ok_or_else(|| {
                fail(
                    "join_fanout_budget",
                    "Observation/selected projection overflow",
                )
            })?;
        weights.insert(key.clone(), weight);
    }
    let mut rows = Vec::new();
    let mut joined = 0usize;
    let mut summary = BTreeMap::new();
    for (source, p) in report.pages.iter().enumerate() {
        for (link, l) in p.links.iter().enumerate() {
            let observations = targets.get(&l.url).map(Vec::as_slice).unwrap_or(&[]);
            let cost = weights
                .get(l.url.as_str())
                .copied()
                .unwrap_or(0)
                .checked_add(selected.get(&p.url).map_or(0, Vec::len))
                .ok_or_else(|| fail("join_fanout_budget", "Projection count overflow"))?;
            joined = joined
                .checked_add(cost)
                .filter(|n| *n <= MAX_JOINED_OBSERVATIONS)
                .ok_or_else(|| {
                    fail(
                        "join_fanout_budget",
                        "Exact-key observation multimap exceeds 500000 joined occurrences",
                    )
                })?;
            let (verdict, reason) = if facts[source].age.is_some() {
                (Verdict::Unknown, "source_age_unknown")
            } else if p.blocked_by_robots || p.error.is_some() || !facts[source].coherent {
                (Verdict::Unknown, "source_response_incomplete")
            } else if http_identity(&l.url).is_none() {
                (Verdict::Unknown, "invalid_stored_target_identity")
            } else if observations.is_empty() {
                (Verdict::Unknown, "no_requested_url_observation")
            } else if observations.len() != 1 {
                (Verdict::Unknown, "ambiguous_requested_url_observations")
            } else {
                let i = observations[0];
                let target = &report.pages[i];
                if target.blocked_by_robots {
                    (Verdict::Unknown, "target_blocked_by_robots")
                } else if target.error.is_some() {
                    (Verdict::Unknown, "recorded_target_fetch_error")
                } else if target.final_status.is_none() {
                    (Verdict::Unknown, "target_final_response_unobserved")
                } else if facts[i].age.is_some() {
                    (Verdict::Unknown, "target_age_unknown")
                } else if !facts[i].coherent {
                    (
                        Verdict::Unknown,
                        "inconsistent_stored_response_or_redirect_chain",
                    )
                } else if target
                    .final_status
                    .is_some_and(|s| (400..=599).contains(&s))
                {
                    (
                        Verdict::ObservedHttpFailure,
                        "unique_observed_final_http_4xx_or_5xx",
                    )
                } else if target
                    .final_status
                    .is_some_and(|s| (200..=299).contains(&s))
                {
                    (
                        Verdict::ObservedNonFailure,
                        "unique_observed_final_http_2xx",
                    )
                } else {
                    (Verdict::Unknown, "nonterminal_final_http_status")
                }
            };
            *summary.entry(verdict).or_insert(0) += 1;
            rows.push(RowIndex {
                source,
                link,
                verdict,
                reason,
            });
        }
    }
    let token = digest(
        &(
            "broken_sources_binding_v1",
            &snapshot,
            filter,
            SCOPE,
            MAX_PAGES,
            MAX_LINKS,
            MAX_JOINED_OBSERVATIONS,
        ),
        64 * 1024,
    )?
    .0;
    Ok(Evaluation {
        report,
        snapshot,
        token,
        filter,
        targets,
        selected,
        facts,
        rows,
        summary,
    })
}
#[derive(Serialize)]
pub(crate) struct Source<'a> {
    pub page_ordinal: usize,
    pub requested_url: &'a str,
    pub document_url: &'a str,
    pub final_url: &'a str,
    pub initial_status: Option<u16>,
    pub final_status: Option<u16>,
    pub fetched_at: &'a str,
    pub truncated: bool,
    pub rendered: bool,
    pub renderer_error: &'a Option<String>,
    pub redirect_chain: &'a [open_seo_spider_core::RedirectHop],
    pub age_unknown_reason: Option<&'static str>,
    pub selected_observations: Vec<&'a RecrawlTargetObservation>,
}
#[derive(Serialize)]
pub(crate) struct Target<'a> {
    pub page_ordinal: usize,
    pub requested_url: &'a str,
    pub final_url: &'a str,
    pub initial_status: Option<u16>,
    pub final_status: Option<u16>,
    pub fetched_at: &'a str,
    pub error: &'a Option<String>,
    pub blocked_by_robots: bool,
    pub redirect_chain: &'a [open_seo_spider_core::RedirectHop],
    pub age_unknown_reason: Option<&'static str>,
    pub selected_observations: Vec<&'a RecrawlTargetObservation>,
}
#[derive(Serialize)]
pub(crate) struct Row<'a> {
    pub source: Source<'a>,
    pub link_ordinal: usize,
    pub link: &'a LinkRecord,
    pub observation_count: usize,
    pub target_observations: Vec<Target<'a>>,
    pub verdict: Verdict,
    pub reason: &'static str,
}
#[derive(Serialize)]
pub(crate) struct Page<'a> {
    pub scope: &'static str,
    pub website_requests: usize,
    pub snapshot: &'a Snapshot,
    pub snapshot_token: &'a str,
    pub filter: Filter,
    pub total_retained_link_occurrences: usize,
    pub filtered_rows: usize,
    pub summary: &'a BTreeMap<Verdict, usize>,
    pub offset: usize,
    pub rows: Vec<Row<'a>>,
    pub next_cursor: Option<String>,
    pub observation_basis: &'static str,
    pub join_identity: &'static str,
    pub selected_recrawl: bool,
    pub source_kind_coverage_complete: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: u8,
    token: String,
    filter: Filter,
    limit: usize,
    offset: usize,
}
impl Evaluation {
    fn selected_for(&self, url: &str) -> Vec<&RecrawlTargetObservation> {
        self.selected
            .get(url)
            .into_iter()
            .flatten()
            .filter_map(|i| {
                self.report
                    .operation
                    .as_ref()
                    .and_then(|o| o.targets.get(*i))
            })
            .collect()
    }
    fn row(&self, r: &RowIndex) -> Row<'_> {
        let p = &self.report.pages[r.source];
        let l = &p.links[r.link];
        let indices = self.targets.get(&l.url).map(Vec::as_slice).unwrap_or(&[]);
        Row {
            source: Source {
                page_ordinal: r.source,
                requested_url: &p.url,
                document_url: p.document_url(),
                final_url: &p.final_url,
                initial_status: p.status,
                final_status: p.final_status,
                fetched_at: &p.fetched_at,
                truncated: p.truncated,
                rendered: p.rendered,
                renderer_error: &p.renderer_error,
                redirect_chain: &p.redirect_chain,
                age_unknown_reason: self.facts[r.source].age,
                selected_observations: self.selected_for(&p.url),
            },
            link_ordinal: r.link,
            link: l,
            observation_count: indices.len(),
            target_observations: indices
                .iter()
                .map(|i| {
                    let p = &self.report.pages[*i];
                    Target {
                        page_ordinal: *i,
                        requested_url: &p.url,
                        final_url: &p.final_url,
                        initial_status: p.status,
                        final_status: p.final_status,
                        fetched_at: &p.fetched_at,
                        error: &p.error,
                        blocked_by_robots: p.blocked_by_robots,
                        redirect_chain: &p.redirect_chain,
                        age_unknown_reason: self.facts[*i].age,
                        selected_observations: self.selected_for(&p.url),
                    }
                })
                .collect(),
            verdict: r.verdict,
            reason: r.reason,
        }
    }
    pub(crate) fn check_token(&self, request: &Request, required: bool) -> Result<()> {
        if required && request.expected_snapshot_token.is_none() {
            return Err(fail(
                "snapshot_token_required",
                "Download requires a preview snapshot token",
            ));
        }
        if request.filter != self.filter
            || request
                .expected_snapshot_token
                .as_ref()
                .is_some_and(|v| v != &self.token)
        {
            return Err(fail(
                "stale_snapshot",
                "Report/config/generation/filter changed; run a fresh preview",
            ));
        }
        Ok(())
    }
    pub(crate) fn page(&self, request: &Request) -> Result<Page<'_>> {
        request.validate()?;
        self.check_token(request, false)?;
        let offset = if let Some(raw) = &request.cursor {
            let bytes = URL_SAFE_NO_PAD
                .decode(raw)
                .map_err(|e| fail("invalid_cursor", e))?;
            let c: Cursor =
                serde_json::from_slice(&bytes).map_err(|e| fail("invalid_cursor", e))?;
            if c.version != 1
                || c.token != self.token
                || c.filter != self.filter
                || c.limit != request.limit
            {
                return Err(fail(
                    "stale_cursor",
                    "Cursor snapshot/filter/page limit differs",
                ));
            }
            c.offset
        } else {
            0
        };
        let count = self
            .rows
            .iter()
            .filter(|r| self.filter.includes(r.verdict))
            .count();
        if offset > count {
            return Err(fail(
                "invalid_cursor",
                "Cursor offset exceeds complete selected result",
            ));
        }
        let rows = self
            .rows
            .iter()
            .filter(|r| self.filter.includes(r.verdict))
            .skip(offset)
            .take(request.limit)
            .map(|r| self.row(r))
            .collect::<Vec<_>>();
        let end = offset + rows.len();
        let next_cursor = if end < count {
            Some(
                URL_SAFE_NO_PAD.encode(
                    serde_json::to_vec(&Cursor {
                        version: 1,
                        token: self.token.clone(),
                        filter: self.filter,
                        limit: request.limit,
                        offset: end,
                    })
                    .map_err(|e| fail("cursor_serialization", e))?,
                ),
            )
        } else {
            None
        };
        Ok(Page {
            scope: SCOPE,
            website_requests: 0,
            snapshot: &self.snapshot,
            snapshot_token: &self.token,
            filter: self.filter,
            total_retained_link_occurrences: self.rows.len(),
            filtered_rows: count,
            summary: &self.summary,
            offset,
            rows,
            next_cursor,
            observation_basis: "within_saved_report_run_window_not_current_website_freshness; app_declared_not_sql_attested",
            join_identity: "exact_stored_link_url_to_requested_page_url_multimap; fragments_and_query_never_collapsed_for_join",
            selected_recrawl: self.report.operation.is_some(),
            source_kind_coverage_complete: false,
        })
    }
    pub(crate) fn export_csv(&self) -> Result<Vec<u8>> {
        self.export_csv_limit(CSV_BYTES)
    }
    fn export_csv_limit(&self, max: usize) -> Result<Vec<u8>> {
        let mut out = BoundedBytes {
            bytes: Vec::new(),
            max,
        };
        csv_record(
            &mut out,
            &[
                "scope",
                "source_page_ordinal",
                "source_requested_url",
                "source_document_url",
                "source_final_url",
                "source_initial_status",
                "source_final_status",
                "source_fetched_at",
                "source_age_unknown_reason",
                "source_selected_observations_json",
                "link_ordinal",
                "retained_link_json",
                "kind",
                "anchor_text",
                "rel",
                "internal",
                "nofollow",
                "target_requested_url",
                "raw_href",
                "resolved_href",
                "fragment",
                "dom_index",
                "dom_path",
                "dom_path_truncated",
                "semantic_ancestors_json",
                "position_classification",
                "observation_count",
                "verdict",
                "reason",
                "target_observations_json",
                "source_redirect_chain_json",
                "source_truncated",
                "source_rendered",
                "source_renderer_error",
                "snapshot_token",
                "typed_report_sha256",
                "app_generation",
                "website_requests",
            ]
            .map(str::to_owned),
        )?;
        for index in self.rows.iter().filter(|r| self.filter.includes(r.verdict)) {
            let row = self.row(index);
            let l = row.link;
            let d = l.dom_location.as_ref();
            let s = &row.source;
            csv_record(
                &mut out,
                &[
                    SCOPE.into(),
                    s.page_ordinal.to_string(),
                    s.requested_url.into(),
                    s.document_url.into(),
                    s.final_url.into(),
                    s.initial_status.map(|v| v.to_string()).unwrap_or_default(),
                    s.final_status.map(|v| v.to_string()).unwrap_or_default(),
                    s.fetched_at.into(),
                    s.age_unknown_reason.unwrap_or_default().into(),
                    bounded_json(&s.selected_observations)?,
                    row.link_ordinal.to_string(),
                    bounded_json(l)?,
                    l.kind.clone(),
                    l.text.clone(),
                    l.rel.clone(),
                    l.internal.to_string(),
                    l.nofollow.to_string(),
                    l.url.clone(),
                    l.raw_href.clone().unwrap_or_default(),
                    l.resolved_href.clone().unwrap_or_default(),
                    l.fragment.clone().unwrap_or_default(),
                    d.map(|d| d.index.to_string()).unwrap_or_default(),
                    d.and_then(|d| d.path.clone()).unwrap_or_default(),
                    d.map(|d| d.path_truncated.to_string()).unwrap_or_default(),
                    bounded_json(&d.map(|d| &d.semantic_ancestors))?,
                    l.position.clone().unwrap_or_default(),
                    row.observation_count.to_string(),
                    match row.verdict {
                        Verdict::ObservedHttpFailure => "observed_http_failure",
                        Verdict::Unknown => "unknown",
                        Verdict::ObservedNonFailure => "observed_non_failure",
                    }
                    .into(),
                    row.reason.into(),
                    bounded_json(&row.target_observations)?,
                    bounded_json(s.redirect_chain)?,
                    s.truncated.to_string(),
                    s.rendered.to_string(),
                    s.renderer_error.clone().unwrap_or_default(),
                    self.token.clone(),
                    self.snapshot.typed_report_sha256.clone(),
                    self.snapshot.app_generation.to_string(),
                    "0".into(),
                ],
            )?;
        }
        Ok(out.bytes)
    }
}
fn bounded_json(value: &(impl Serialize + ?Sized)) -> Result<String> {
    digest(&value, JSON_BYTES)?;
    serde_json::to_string(&value).map_err(|e| fail("json_serialization", e))
}
pub(crate) fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>> {
    digest(value, JSON_BYTES)?;
    serde_json::to_vec(value).map_err(|e| fail("json_serialization", e))
}
struct BoundedBytes {
    bytes: Vec<u8>,
    max: usize,
}
impl Write for BoundedBytes {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.bytes
            .len()
            .checked_add(b.len())
            .filter(|n| *n <= self.max)
            .ok_or_else(|| io::Error::other("Broken-source CSV exceeds independent byte limit"))?;
        self.bytes.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn csv_record(out: &mut impl Write, fields: &[String]) -> Result<()> {
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            out.write_all(b",").map_err(|e| fail("csv_budget", e))?
        }
        let escaped = open_seo_spider_core::csv_text(field).replace('"', "\"\"");
        out.write_all(b"\"")
            .and_then(|_| out.write_all(escaped.as_bytes()))
            .and_then(|_| out.write_all(b"\""))
            .map_err(|e| fail("csv_budget", e))?;
    }
    out.write_all(b"\r\n").map_err(|e| fail("csv_budget", e))
}
#[cfg(test)]
#[path = "broken_sources_tests.rs"]
mod tests;
