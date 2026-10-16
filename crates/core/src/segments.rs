//! Bounded, ordered, multi-label projection of saved page facts. No transport or Config mutation.
use crate::{AuditIssue, CrawlReport, PageRecord};
use anyhow::{Result, ensure};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
};

pub const ENGINE: &str = "saved_segments_v1";
/// Display/chart values are declared typed report values; missing legacy/import fields may have defaulted0.
/// No depth segment predicate is supported without a separate observation-presence authority.
pub const DEPTH_POLICY: &str =
    "declared_saved_depth_including_possible_legacy_or_import_default_zero_not_a_segment_predicate";
pub const REQUEST_BYTES: usize = 64 * 1024;
pub const REPORT_BYTES: usize = 64 * 1024 * 1024;
pub const OUTPUT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_PAGES: usize = 50_000;
pub const MAX_ISSUES: usize = 100_000;
const WORK_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    RequestedUrl,
    FinalUrl,
    Status,
    MediaType,
    StoredTitle,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Equals,
    Contains,
    Regex,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub field: Field,
    pub operator: Operator,
    pub value: String,
    #[serde(default)]
    pub case_insensitive: bool,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Join {
    #[default]
    All,
    Any,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub join: Join,
    pub conditions: Vec<Condition>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub version: u32,
    pub definitions: Vec<Definition>,
}
impl Default for Rules {
    fn default() -> Self {
        Self {
            version: 1,
            definitions: vec![],
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selection {
    #[default]
    All,
    Segment {
        id: String,
    },
    Unassigned,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Truth {
    Yes,
    No,
    Unknown,
}
struct Compiled {
    conditions: Vec<Vec<Regex>>,
}

pub fn validate_rules(rules: &Rules) -> Result<()> {
    compile(rules).map(|_| ())
}
fn compile(rules: &Rules) -> Result<Compiled> {
    ensure!(rules.version == 1, "Unsupported segment rules version");
    ensure!(
        rules.definitions.len() <= 32,
        "At most32 segments are supported"
    );
    let mut ids = BTreeSet::new();
    let mut patterns = Vec::new();
    let mut bytes = 0usize;
    for d in &rules.definitions {
        ensure!(
            !d.id.is_empty()
                && d.id.len() <= 64
                && d.id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
            "Segment ID requires1..64 ASCII letters, digits, underscores or hyphens"
        );
        ensure!(ids.insert(d.id.as_str()), "Segment IDs must be unique");
        ensure!(
            !d.name.trim().is_empty() && d.name.len() <= 256,
            "Segment name requires1..256 UTF8 bytes"
        );
        ensure!(
            (1..=8).contains(&d.conditions.len()),
            "Each segment requires1..8 conditions"
        );
        let mut row = Vec::new();
        for c in &d.conditions {
            ensure!(c.value.len() <= 2048, "A pattern exceeds2048 UTF8 bytes");
            bytes = bytes
                .checked_add(c.value.len())
                .filter(|n| *n <= 32 * 1024)
                .ok_or_else(|| anyhow::anyhow!("Total pattern budget exceeded"))?;
            let pattern = match c.operator {
                Operator::Equals => format!("\\A(?:{})\\z", regex::escape(&c.value)),
                Operator::Contains => regex::escape(&c.value),
                Operator::Regex => c.value.clone(),
            };
            row.push(
                RegexBuilder::new(&pattern)
                    .case_insensitive(c.case_insensitive)
                    .size_limit(128 * 1024)
                    .dfa_size_limit(128 * 1024)
                    .build()?,
            );
        }
        patterns.push(row);
    }
    Ok(Compiled {
        conditions: patterns,
    })
}
fn value(p: &PageRecord, field: Field) -> Option<std::borrow::Cow<'_, str>> {
    use std::borrow::Cow::{Borrowed, Owned};
    match field {
        Field::RequestedUrl => (!p.url.is_empty()).then_some(Borrowed(p.url.as_str())),
        Field::FinalUrl => (!p.final_url.is_empty()).then_some(Borrowed(p.final_url.as_str())),
        Field::Status => p
            .status
            .filter(|s| (100..=999).contains(s))
            .map(|n| Owned(n.to_string())),
        Field::MediaType => (!p.media_type().is_empty()).then_some(Borrowed(p.media_type())),
        // These are stored document metadata, not raw-markup observations. Absence is unknown,
        // including legacy empty vectors. A present empty title is a known empty string.
        Field::StoredTitle => {
            if p.truncated
                || p.error.is_some()
                || p.renderer_error.is_some()
                || (p.rendered && (p.rendered_status != Some(200) || p.rendered_url.is_none()))
            {
                None
            } else {
                p.titles.first().map(|s| Borrowed(s.as_str()))
            }
        }
    }
}
fn charge(total: &mut usize, n: usize, cap: usize) -> Result<()> {
    *total = total.checked_add(n).filter(|n| *n <= cap).ok_or_else(|| {
        anyhow::anyhow!("Segment projection byte/work budget exceeded; no partial result")
    })?;
    Ok(())
}
#[derive(Debug, Clone, Serialize)]
pub struct Count {
    pub id: String,
    pub name: String,
    pub matched: usize,
    pub unknown: usize,
    pub selected_matched: usize,
    pub selected_unknown: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub page_ordinal: usize,
    pub url: String,
    pub document_url: String,
    pub status: Option<u16>,
    pub depth: usize,
    pub stored_title: Option<String>,
    pub matched_indices: Vec<usize>,
    pub unknown_indices: Vec<usize>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Charts {
    pub status: BTreeMap<String, usize>,
    pub depth: BTreeMap<usize, usize>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Projection {
    pub engine: &'static str,
    pub rules: Rules,
    pub selection: Selection,
    pub depth_policy: &'static str,
    pub total_pages: usize,
    pub selected_pages: usize,
    pub multiple_matches: usize,
    pub unassigned_pages: usize,
    pub unknown_pages: usize,
    pub orphan_issues: usize,
    pub counts: Vec<Count>,
    pub charts: Charts,
    pub pages: Vec<Row>,
    pub issues: Vec<AuditIssue>,
}
pub fn project(report: &CrawlReport, rules: &Rules, selection: &Selection) -> Result<Projection> {
    let compiled = compile(rules)?;
    ensure!(
        report.pages.len() <= MAX_PAGES && report.issues.len() <= MAX_ISSUES,
        "Segment input row ceiling exceeded"
    );
    let wanted = match selection {
        Selection::Segment { id } => Some(
            rules
                .definitions
                .iter()
                .position(|d| d.id == *id)
                .ok_or_else(|| anyhow::anyhow!("Unknown segment ID"))?,
        ),
        _ => None,
    };
    let mut counts: Vec<_> = rules
        .definitions
        .iter()
        .map(|d| Count {
            id: d.id.clone(),
            name: d.name.clone(),
            matched: 0,
            unknown: 0,
            selected_matched: 0,
            selected_unknown: 0,
        })
        .collect();
    let mut pages = Vec::new();
    let mut selected_urls = BTreeSet::new();
    let mut all_urls = BTreeSet::new();
    let (mut unassigned, mut unknown, mut overlaps, mut work, mut copied) = (0, 0, 0, 0, 0);
    let mut charts = Charts {
        status: BTreeMap::new(),
        depth: BTreeMap::new(),
    };
    for (ordinal, p) in report.pages.iter().enumerate() {
        charge(&mut copied, p.url.len(), OUTPUT_BYTES)?;
        all_urls.insert(p.url.as_str());
        let (mut matched, mut missing) = (Vec::new(), Vec::new());
        for (i, d) in rules.definitions.iter().enumerate() {
            let (mut yes, mut no, mut absent) = (false, false, false);
            for (c, re) in d.conditions.iter().zip(&compiled.conditions[i]) {
                let truth = match value(p, c.field) {
                    None => Truth::Unknown,
                    Some(v) => {
                        ensure!(v.len() <= 64 * 1024, "Segment field exceeds64KiB");
                        charge(&mut work, v.len().max(1), WORK_BYTES)?;
                        if re.is_match(&v) {
                            Truth::Yes
                        } else {
                            Truth::No
                        }
                    }
                };
                match truth {
                    Truth::Yes => yes = true,
                    Truth::No => no = true,
                    Truth::Unknown => absent = true,
                }
            }
            let outcome = match d.join {
                Join::All => {
                    if no {
                        Truth::No
                    } else if absent {
                        Truth::Unknown
                    } else {
                        Truth::Yes
                    }
                }
                Join::Any => {
                    if yes {
                        Truth::Yes
                    } else if absent {
                        Truth::Unknown
                    } else {
                        Truth::No
                    }
                }
            };
            match outcome {
                Truth::Yes => {
                    matched.push(i);
                    counts[i].matched += 1;
                }
                Truth::Unknown => {
                    missing.push(i);
                    counts[i].unknown += 1;
                }
                Truth::No => {}
            }
        }
        let unresolved = matched.is_empty() && !missing.is_empty();
        let empty = matched.is_empty() && missing.is_empty();
        unassigned += usize::from(empty);
        unknown += usize::from(unresolved);
        overlaps += usize::from(matched.len() > 1);
        let take = match selection {
            Selection::All => true,
            Selection::Segment { .. } => wanted.is_some_and(|i| matched.contains(&i)),
            Selection::Unassigned => empty,
            Selection::Unknown => unresolved,
        };
        if take {
            for i in &matched {
                counts[*i].selected_matched += 1;
            }
            for i in &missing {
                counts[*i].selected_unknown += 1;
            }
            selected_urls.insert(p.url.as_str());
            let title = value(p, Field::StoredTitle);
            charge(&mut copied, p.document_url().len(), OUTPUT_BYTES)?;
            charge(
                &mut copied,
                title.as_ref().map_or(0, |s| s.len()),
                OUTPUT_BYTES,
            )?;
            *charts
                .status
                .entry(
                    p.status
                        .filter(|s| (100..=999).contains(s))
                        .map_or_else(|| "unknown".into(), |n| n.to_string()),
                )
                .or_default() += 1;
            *charts.depth.entry(p.depth).or_default() += 1;
            pages.push(Row {
                page_ordinal: ordinal,
                url: p.url.clone(),
                document_url: p.document_url().into(),
                status: p.status,
                depth: p.depth,
                stored_title: title.map(|s| s.into_owned()),
                matched_indices: matched,
                unknown_indices: missing,
            });
        }
    }
    let mut issues = Vec::new();
    let mut orphan_issues = 0;
    for i in &report.issues {
        if !all_urls.contains(i.url.as_str()) {
            orphan_issues += 1;
        }
        if selected_urls.contains(i.url.as_str()) {
            for s in [&i.code, &i.url, &i.message]
                .into_iter()
                .chain(i.related_urls.iter())
            {
                charge(&mut copied, s.len(), OUTPUT_BYTES)?;
            }
            issues.push(i.clone());
        }
    }
    Ok(Projection {
        engine: ENGINE,
        depth_policy: DEPTH_POLICY,
        rules: rules.clone(),
        selection: selection.clone(),
        total_pages: report.pages.len(),
        selected_pages: pages.len(),
        multiple_matches: overlaps,
        unassigned_pages: unassigned,
        unknown_pages: unknown,
        orphan_issues,
        counts,
        charts,
        pages,
        issues,
    })
}

struct Bounded {
    bytes: Vec<u8>,
    cap: usize,
}
impl Write for Bounded {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if b.len() > self.cap.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other(
                "Segment output exceeds byte ceiling; no partial result",
            ));
        }
        self.bytes.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut sink = Bounded {
        bytes: Vec::new(),
        cap: OUTPUT_BYTES,
    };
    serde_json::to_writer(&mut sink, value)?;
    Ok(sink.bytes)
}
fn safe_cell(s: &str) -> String {
    if matches!(
        s.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
            .chars()
            .next(),
        Some('=' | '+' | '-' | '@')
    ) {
        format!("'{s}")
    } else {
        s.into()
    }
}
/// Complete selected pages AND issues, plus counts/charts/rules/binding supplied by App.
/// Every row has an explicit type. JSON cells preserve nested values and exact Unicode.
pub fn csv_bytes(projection: &Projection, context: &impl Serialize) -> Result<Vec<u8>> {
    let sink = Bounded {
        bytes: Vec::new(),
        cap: OUTPUT_BYTES,
    };
    let mut w = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(sink);
    w.write_record([
        "row_type",
        "page_ordinal",
        "url",
        "segment_indices_json",
        "unknown_indices_json",
        "data_json",
    ])?;
    let mut record = |kind: &str,
                      ordinal: String,
                      url: &str,
                      matches: String,
                      unknown: String,
                      data: String|
     -> Result<()> {
        w.write_record([
            kind.to_owned(),
            ordinal,
            safe_cell(url),
            matches,
            unknown,
            safe_cell(&data),
        ])?;
        Ok(())
    };
    record(
        "context",
        String::new(),
        "",
        String::new(),
        String::new(),
        serde_json::to_string(context)?,
    )?;
    record(
        "summary",
        String::new(),
        "",
        String::new(),
        String::new(),
        serde_json::to_string(
            &serde_json::json!({"engine":projection.engine,"depth_policy":projection.depth_policy,"rules":projection.rules,"selection":projection.selection,"total_pages":projection.total_pages,"selected_pages":projection.selected_pages,"multiple_matches":projection.multiple_matches,"unassigned_pages":projection.unassigned_pages,"unknown_pages":projection.unknown_pages,"orphan_issues":projection.orphan_issues,"counts":projection.counts,"charts":projection.charts}),
        )?,
    )?;
    for r in &projection.pages {
        record(
            "page",
            r.page_ordinal.to_string(),
            &r.url,
            serde_json::to_string(&r.matched_indices)?,
            serde_json::to_string(&r.unknown_indices)?,
            serde_json::to_string(r)?,
        )?;
    }
    for i in &projection.issues {
        record(
            "issue",
            String::new(),
            &i.url,
            String::new(),
            String::new(),
            serde_json::to_string(i)?,
        )?;
    }
    w.flush()?;
    Ok(w.into_inner()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .bytes)
}
#[cfg(test)]
#[path = "segments_tests.rs"]
mod tests;
