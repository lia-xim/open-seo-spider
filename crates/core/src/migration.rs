//! Bounded, offline migration destination comparison over supplied saved pages.
//! This does not fetch, authenticate a snapshot, select the latest duplicate,
//! compare browser/JavaScript navigation, or assert server fragment behavior.
use crate::{
    ListColumnSelector, ListCsvDelimiter, ListImportFormat, ListImportLimits, ListImportOptions,
    ListImportRow, PageRecord, import_url_list, normalize_url,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

pub const HTTP_MIGRATION_ENGINE: &str = "open-seo-spider/offline-http-migration-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MigrationObservationLimits {
    pub max_pages: usize,
    pub max_redirects_per_page: usize,
    /// Only the relevant supplied URL/time/error/hop text, before any projection.
    pub max_text_bytes: usize,
    /// Exact compact serialized report bytes, independently of CSV import caps.
    pub max_output_bytes: usize,
}
impl Default for MigrationObservationLimits {
    fn default() -> Self {
        Self {
            max_pages: 50_000,
            max_redirects_per_page: 32,
            max_text_bytes: 32 * 1024 * 1024,
            max_output_bytes: 8 * 1024 * 1024,
        }
    }
}
impl MigrationObservationLimits {
    pub fn validate(&self) -> Result<()> {
        let ceiling = Self::default();
        for (name, value, cap) in [
            ("max_pages", self.max_pages, ceiling.max_pages),
            (
                "max_redirects_per_page",
                self.max_redirects_per_page,
                ceiling.max_redirects_per_page,
            ),
            (
                "max_text_bytes",
                self.max_text_bytes,
                ceiling.max_text_bytes,
            ),
            (
                "max_output_bytes",
                self.max_output_bytes,
                ceiling.max_output_bytes,
            ),
        ] {
            ensure!(value > 0 && value <= cap, "{name} must be 1..={cap}");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MigrationCsvOptions {
    pub delimiter: ListCsvDelimiter,
    pub old_column: ListColumnSelector,
    pub expected_column: ListColumnSelector,
    pub start_row: usize,
    pub import_limits: ListImportLimits,
    pub observation_limits: MigrationObservationLimits,
}
impl Default for MigrationCsvOptions {
    fn default() -> Self {
        Self {
            delimiter: ListCsvDelimiter::Comma,
            old_column: ListColumnSelector::Index { value: 1 },
            expected_column: ListColumnSelector::Index { value: 2 },
            start_row: 2,
            import_limits: ListImportLimits::default(),
            observation_limits: MigrationObservationLimits::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationVerdict {
    InvalidMapping,
    MissingObservation,
    AmbiguousObservation,
    BlockedByRobots,
    FetchError,
    UnobservedFinal,
    InvalidObservation,
    InvalidRedirectChain,
    FinalHttpFailure,
    DestinationMismatch,
    DestinationMatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MigrationRedirect {
    pub url: String,
    pub status: u16,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MigrationObservation {
    /// One-based position in the caller-supplied PageRecord slice, not a DB ID.
    pub page_index: usize,
    pub requested_url: String,
    pub fetched_at: String,
    pub initial_status: Option<u16>,
    pub final_status: Option<u16>,
    /// May be only a proposed redirect target when final_status is absent.
    pub raw_final_url: String,
    /// Present only with an actual syntactically valid final response status.
    pub normalized_final_url: Option<String>,
    pub blocked_by_robots: bool,
    pub error: Option<String>,
    pub redirect_chain: Vec<MigrationRedirect>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MigrationRow {
    pub source_index: usize,
    pub source_row: usize,
    pub source_line: Option<usize>,
    pub old_value: String,
    pub expected_value: String,
    pub old_url: Option<String>,
    pub expected_url: Option<String>,
    pub old_fragment: Option<String>,
    pub expected_fragment: Option<String>,
    pub old_error: Option<String>,
    pub expected_error: Option<String>,
    /// First CSV source_row with the identical normalized old/new pair.
    pub duplicate_of_row: Option<usize>,
    /// Same normalized old URL has multiple different valid expected targets.
    pub conflicting_expectations: bool,
    pub observation_count: usize,
    pub observation: Option<MigrationObservation>,
    /// A URL equality observation, independent of successful HTTP status.
    pub destination_matches: Option<bool>,
    pub verdict: MigrationVerdict,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationReport {
    pub engine: &'static str,
    pub input_sha256: String,
    pub input_bytes: usize,
    pub options: MigrationCsvOptions,
    pub website_requests: usize,
    pub observation_basis: &'static str,
    pub comparison: &'static str,
    pub supplied_pages: usize,
    pub summary: BTreeMap<MigrationVerdict, usize>,
    pub rows: Vec<MigrationRow>,
}

/// Reuses the strict list importer for both columns of the same immutable bytes.
/// Its record positions, BOM, quoting, missing fields and normalization apply.
/// Duplicate expectations and duplicate supplied observations are never dropped.
pub fn evaluate_migration_csv(
    bytes: &[u8],
    options: &MigrationCsvOptions,
    pages: &[PageRecord],
) -> Result<MigrationReport> {
    options.import_limits.validate()?;
    options.observation_limits.validate()?;
    let select = |column: ListColumnSelector| ListImportOptions {
        format: ListImportFormat::Csv {
            delimiter: options.delimiter,
            column,
            start_row: options.start_row,
        },
        limits: options.import_limits.clone(),
    };
    let old = import_url_list(bytes, &select(options.old_column.clone()))?;
    let expected = import_url_list(bytes, &select(options.expected_column.clone()))?;
    ensure!(
        old.selected_column != expected.selected_column,
        "old and expected columns must differ"
    );
    ensure!(
        old.rows.len() == expected.rows.len(),
        "CSV column record counts differ"
    );

    // Relevant text/count preflight precedes normalized indexing or observation
    // cloning. Caller-owned unrelated HTML/page fields are never cloned here.
    let index = observation_index(pages, options)?;
    let mut targets: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (left, right) in old.rows.iter().zip(&expected.rows) {
        ensure!(
            left.source_row == right.source_row && left.source_line == right.source_line,
            "CSV column source positions differ"
        );
        if left.error.is_none()
            && right.error.is_none()
            && let (Some(a), Some(b)) = (
                left.normalized_url.as_deref(),
                right.normalized_url.as_deref(),
            )
        {
            targets.entry(a).or_default().insert(b);
        }
    }
    let mut report = MigrationReport {
        engine: HTTP_MIGRATION_ENGINE,
        input_sha256: old.input_sha256.clone(),
        input_bytes: bytes.len(),
        options: options.clone(),
        website_requests: 0,
        observation_basis: "caller_supplied_saved_page_records_not_snapshot_authentication",
        comparison: "normalized_http_final_response_url_query_order_path_case_slash_preserved_fragments_ignored_no_renderer_navigation",
        supplied_pages: pages.len(),
        summary: BTreeMap::new(),
        rows: Vec::new(),
    };
    let cap = options.observation_limits.max_output_bytes;
    let envelope = serialized_bytes(&report, cap)?;
    // Summary has exactly eleven possible short keys and at most usize digits
    // per count. A conservative 1024B reserve is independently checked exactly
    // after construction; row bytes are charged BEFORE retention in the Vec.
    let mut retained_bytes = envelope.checked_add(1024).context("report byte overflow")?;
    ensure!(
        retained_bytes <= cap,
        "migration report envelope exceeds max_output_bytes"
    );
    let mut duplicates: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for (left, right) in old.rows.iter().zip(&expected.rows) {
        let mut row = mapping_row(left, right);
        if row.old_error.is_none()
            && row.expected_error.is_none()
            && let (Some(a), Some(b)) = (
                left.normalized_url.as_deref(),
                right.normalized_url.as_deref(),
            )
        {
            row.duplicate_of_row = duplicates.get(&(a, b)).copied();
            duplicates.entry((a, b)).or_insert(row.source_row);
            row.conflicting_expectations = targets.get(a).is_some_and(|values| values.len() > 1);
            let matches = index.get(a).map(Vec::as_slice).unwrap_or(&[]);
            row.observation_count = matches.len();
            match matches {
                [] => set(
                    &mut row,
                    MigrationVerdict::MissingObservation,
                    "No supplied PageRecord for this normalized old URL",
                ),
                [page_index] => evaluate_observation(&mut row, &pages[*page_index], *page_index),
                _ => set(
                    &mut row,
                    MigrationVerdict::AmbiguousObservation,
                    "Multiple supplied PageRecords; no newest/first observation was chosen",
                ),
            }
        }
        let size = serialized_bytes(&row, cap)?;
        retained_bytes = retained_bytes
            .checked_add(size)
            .and_then(|n| n.checked_add(1))
            .context("report byte overflow")?;
        ensure!(
            retained_bytes <= cap,
            "migration rows exceed max_output_bytes"
        );
        *report.summary.entry(row.verdict).or_default() += 1;
        report.rows.push(row);
    }
    serialized_bytes(&report, cap)?;
    Ok(report)
}

fn mapping_row(old: &ListImportRow, expected: &ListImportRow) -> MigrationRow {
    MigrationRow {
        source_index: old.source_index,
        source_row: old.source_row,
        source_line: old.source_line,
        old_value: old.raw_value.clone(),
        expected_value: expected.raw_value.clone(),
        old_url: old.normalized_url.clone(),
        expected_url: expected.normalized_url.clone(),
        old_fragment: old.fragment.clone(),
        expected_fragment: expected.fragment.clone(),
        old_error: old.error.clone(),
        expected_error: expected.error.clone(),
        duplicate_of_row: None,
        conflicting_expectations: false,
        observation_count: 0,
        observation: None,
        destination_matches: None,
        verdict: MigrationVerdict::InvalidMapping,
        reason:
            "Invalid or missing old/expected HTTP URL; original values and field errors retained"
                .into(),
    }
}
fn set(row: &mut MigrationRow, verdict: MigrationVerdict, reason: &str) {
    row.verdict = verdict;
    row.reason = reason.into();
}
fn strict_url(value: &str) -> Option<String> {
    if value.chars().any(char::is_control) {
        return None;
    }
    normalize_url(value, None).map(|url| url.to_string())
}
fn observation_index(
    pages: &[PageRecord],
    options: &MigrationCsvOptions,
) -> Result<BTreeMap<String, Vec<usize>>> {
    let limits = &options.observation_limits;
    ensure!(
        pages.len() <= limits.max_pages,
        "supplied observations exceed max_pages"
    );
    let mut text_bytes = 0usize;
    let mut field = |value: &str| -> Result<()> {
        ensure!(
            value.len() <= options.import_limits.max_field_bytes,
            "observation field exceeds max_field_bytes"
        );
        text_bytes = text_bytes
            .checked_add(value.len())
            .context("observation text overflow")?;
        ensure!(
            text_bytes <= limits.max_text_bytes,
            "observations exceed max_text_bytes"
        );
        Ok(())
    };
    for page in pages {
        ensure!(
            page.redirect_chain.len() <= limits.max_redirects_per_page,
            "observation exceeds max_redirects_per_page"
        );
        field(&page.url)?;
        field(&page.final_url)?;
        field(&page.fetched_at)?;
        if let Some(error) = &page.error {
            field(error)?;
        }
        for hop in &page.redirect_chain {
            field(&hop.url)?;
            field(&hop.target)?;
        }
    }
    let mut result: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (at, page) in pages.iter().enumerate() {
        let key =
            strict_url(&page.url).context("supplied PageRecord has invalid requested HTTP URL")?;
        result.entry(key).or_default().push(at);
    }
    Ok(result)
}
fn evaluate_observation(row: &mut MigrationRow, page: &PageRecord, index: usize) {
    let final_url = page
        .final_status
        .filter(|status| (100..=599).contains(status))
        .and_then(|_| strict_url(&page.final_url));
    row.destination_matches = final_url
        .as_ref()
        .map(|url| Some(url) == row.expected_url.as_ref());
    row.observation = Some(MigrationObservation {
        page_index: index + 1,
        requested_url: page.url.clone(),
        fetched_at: page.fetched_at.clone(),
        initial_status: page.status,
        final_status: page.final_status,
        raw_final_url: page.final_url.clone(),
        normalized_final_url: final_url,
        blocked_by_robots: page.blocked_by_robots,
        error: page.error.clone(),
        redirect_chain: page
            .redirect_chain
            .iter()
            .map(|hop| MigrationRedirect {
                url: hop.url.clone(),
                status: hop.status,
                target: hop.target.clone(),
            })
            .collect(),
    });
    if page.blocked_by_robots {
        set(
            row,
            MigrationVerdict::BlockedByRobots,
            "Saved page was blocked by robots; no successful destination assertion",
        );
    } else if page.error.is_some() {
        set(
            row,
            MigrationVerdict::FetchError,
            "Saved fetch has an error; recorded targets are not successful migration evidence",
        );
    } else if page.final_status.is_none() {
        set(
            row,
            MigrationVerdict::UnobservedFinal,
            "No final response status; final_url may be only an unfetched redirect target",
        );
    } else if page.status.is_none_or(|s| !(100..=599).contains(&s))
        || page.final_status.is_some_and(|s| !(100..=599).contains(&s))
        || row
            .observation
            .as_ref()
            .is_none_or(|value| value.normalized_final_url.is_none())
    {
        set(
            row,
            MigrationVerdict::InvalidObservation,
            "Missing/invalid response status or final HTTP URL in supplied observation",
        );
    } else if let Some(reason) = invalid_chain(page) {
        set(row, MigrationVerdict::InvalidRedirectChain, reason);
    } else if !page.final_status.is_some_and(|s| (200..=299).contains(&s)) {
        set(
            row,
            MigrationVerdict::FinalHttpFailure,
            "Final response is outside 200..299, even when its URL equals the expectation",
        );
    } else if row.destination_matches == Some(true) {
        set(
            row,
            MigrationVerdict::DestinationMatch,
            "Observed successful HTTP final response URL equals the normalized expectation",
        );
    } else {
        set(
            row,
            MigrationVerdict::DestinationMismatch,
            "Observed successful HTTP final response URL differs from the normalized expectation",
        );
    }
}
fn invalid_chain(page: &PageRecord) -> Option<&'static str> {
    let mut current = strict_url(&page.url)?;
    let mut visited = BTreeSet::from([current.clone()]);
    for (index, hop) in page.redirect_chain.iter().enumerate() {
        if !matches!(hop.status, 301 | 302 | 303 | 307 | 308) {
            return Some("Saved redirect hop has a non-redirect HTTP status");
        }
        if strict_url(&hop.url).as_ref() != Some(&current) {
            return Some("Saved redirect chain is not contiguous from the requested URL");
        }
        if index == 0 && page.status != Some(hop.status) {
            return Some("Initial response status contradicts the first redirect hop");
        }
        let Some(target) = strict_url(&hop.target) else {
            return Some("Saved redirect target is not an absolute HTTP URL");
        };
        if !visited.insert(target.clone()) {
            return Some("Saved redirect chain contains a repeated normalized HTTP URL");
        }
        current = target;
    }
    if strict_url(&page.final_url).as_ref() != Some(&current) {
        return Some("Saved final response URL contradicts the requested URL/redirect chain tail");
    }
    if page.redirect_chain.is_empty() && page.status != page.final_status {
        return Some("No redirect hops explain different initial and final response statuses");
    }
    None
}

struct ByteCounter {
    bytes: usize,
    cap: usize,
}
impl Write for ByteCounter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let total = self
            .bytes
            .checked_add(bytes.len())
            .filter(|total| *total <= self.cap)
            .ok_or_else(|| io::Error::other("migration serialization exceeds max_output_bytes"))?;
        self.bytes = total;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn serialized_bytes(value: &impl Serialize, cap: usize) -> Result<usize> {
    let mut writer = ByteCounter { bytes: 0, cap };
    serde_json::to_writer(&mut writer, value)
        .context("migration report exceeds max_output_bytes")?;
    Ok(writer.bytes)
}

#[cfg(test)]
#[path = "migration_tests.rs"]
mod tests;
