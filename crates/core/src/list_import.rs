//! Independent byte-only list import. No network, filesystem or crawl mutation.
//! XLSX is inspected/preflighted before calamine can allocate shared strings.
use anyhow::{Context, Result, bail, ensure};
use calamine::{DataRef, Reader as _, Xlsx, XlsxFormulaMetadata};
use quick_xml::{events::Event, name::ResolveResult, reader::NsReader};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
};
use url::Url;

pub const LIST_IMPORT_ENGINE: &str = "open-seo-spider/offline-list-v1/calamine-0.36.1";
const SITEMAP_NS: &str = "http://www.sitemaps.org/schemas/sitemap/0.9";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListImportOptions {
    pub format: ListImportFormat,
    #[serde(default)]
    pub limits: ListImportLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ListImportFormat {
    Txt,
    Csv {
        delimiter: ListCsvDelimiter,
        column: ListColumnSelector,
        start_row: usize,
    },
    Xml,
    Xlsx {
        sheet: ListSheetSelector,
        column: ListColumnSelector,
        start_row: usize,
        #[serde(default)]
        formula_policy: ListFormulaPolicy,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListCsvDelimiter {
    Comma,
    Semicolon,
    Tab,
}
impl ListCsvDelimiter {
    fn byte(self) -> u8 {
        match self {
            Self::Comma => b',',
            Self::Semicolon => b';',
            Self::Tab => b'\t',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ListColumnSelector {
    Index { value: usize },
    Header { name: String, row: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ListSheetSelector {
    Name { value: String },
    Index { value: usize },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListFormulaPolicy {
    #[default]
    Reject,
    CachedStringOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ListImportLimits {
    pub max_input_bytes: usize,
    pub max_rows: usize,
    pub max_field_bytes: usize,
    pub max_zip_entries: usize,
    pub max_zip_entry_bytes: usize,
    pub max_zip_total_bytes: usize,
    pub max_xml_events: usize,
    pub max_xml_depth: usize,
    pub max_shared_strings: usize,
    pub max_sheet_cells: usize,
    pub max_csv_fields: usize,
    pub max_output_bytes: usize,
    pub max_sheets: usize,
}
impl Default for ListImportLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 8 * 1024 * 1024,
            max_rows: 50_000,
            max_field_bytes: 8192,
            max_zip_entries: 1024,
            max_zip_entry_bytes: 8 * 1024 * 1024,
            max_zip_total_bytes: 32 * 1024 * 1024,
            max_xml_events: 500_000,
            max_xml_depth: 64,
            max_shared_strings: 100_000,
            max_sheet_cells: 200_000,
            max_csv_fields: 200_000,
            max_output_bytes: 8 * 1024 * 1024,
            max_sheets: 128,
        }
    }
}
impl ListImportLimits {
    pub fn validate(&self) -> Result<()> {
        let cap = Self::default();
        for (name, n, ceiling) in [
            ("max_input_bytes", self.max_input_bytes, cap.max_input_bytes),
            ("max_rows", self.max_rows, cap.max_rows),
            ("max_field_bytes", self.max_field_bytes, cap.max_field_bytes),
            ("max_zip_entries", self.max_zip_entries, cap.max_zip_entries),
            (
                "max_zip_entry_bytes",
                self.max_zip_entry_bytes,
                cap.max_zip_entry_bytes,
            ),
            (
                "max_zip_total_bytes",
                self.max_zip_total_bytes,
                cap.max_zip_total_bytes,
            ),
            ("max_xml_events", self.max_xml_events, cap.max_xml_events),
            ("max_xml_depth", self.max_xml_depth, cap.max_xml_depth),
            (
                "max_shared_strings",
                self.max_shared_strings,
                cap.max_shared_strings,
            ),
            ("max_sheet_cells", self.max_sheet_cells, cap.max_sheet_cells),
            ("max_csv_fields", self.max_csv_fields, cap.max_csv_fields),
            (
                "max_output_bytes",
                self.max_output_bytes,
                cap.max_output_bytes,
            ),
            ("max_sheets", self.max_sheets, cap.max_sheets),
        ] {
            ensure!(n > 0 && n <= ceiling, "{name} must be 1..={ceiling}");
        }
        Ok(())
    }
    fn field(&self, value: &str) -> Result<()> {
        ensure!(
            value.len() <= self.max_field_bytes,
            "decoded field exceeds max_field_bytes"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListCandidateKind {
    PageUrl,
    SitemapDocument,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ListFormulaMetadata {
    Normal {
        formula: String,
    },
    Shared {
        shared_index: usize,
        range: Option<String>,
        formula: String,
    },
    SharedDerived {
        shared_index: usize,
    },
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListImportRow {
    pub source_index: usize,
    pub source_row: usize,
    pub source_line: Option<usize>,
    pub sheet: Option<String>,
    pub cell: Option<String>,
    pub raw_value: String,
    pub cell_type: String,
    pub formula_metadata: Option<ListFormulaMetadata>,
    pub candidate_kind: ListCandidateKind,
    pub normalized_url: Option<String>,
    pub fragment: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListImportSheet {
    pub index: usize,
    pub name: String,
    pub sheet_type: String,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListImportInspection {
    pub engine: String,
    pub input_sha256: String,
    pub input_bytes: usize,
    pub sheets: Vec<ListImportSheet>,
    pub website_requests: usize,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListImportResult {
    pub engine: String,
    pub input_sha256: String,
    pub input_bytes: usize,
    pub options: ListImportOptions,
    pub selected_sheet: Option<ListImportSheet>,
    pub selected_column: Option<usize>,
    pub rows: Vec<ListImportRow>,
    pub website_requests: usize,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

fn input_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn check_input(bytes: &[u8], limits: &ListImportLimits) -> Result<()> {
    limits.validate()?;
    ensure!(!bytes.is_empty(), "input must not be empty");
    ensure!(
        bytes.len() <= limits.max_input_bytes,
        "input exceeds max_input_bytes"
    );
    Ok(())
}
fn utf8(bytes: &[u8]) -> Result<&str> {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    std::str::from_utf8(bytes).context("input requires strict UTF-8, optionally with UTF-8 BOM")
}

fn base_row(index: usize, row: usize, value: String, kind: ListCandidateKind) -> ListImportRow {
    let mut out = ListImportRow {
        source_index: index,
        source_row: row,
        source_line: None,
        sheet: None,
        cell: None,
        raw_value: value,
        cell_type: "text".into(),
        formula_metadata: None,
        candidate_kind: kind,
        normalized_url: None,
        fragment: None,
        error: None,
    };
    match normalize_value(&out.raw_value) {
        Ok((url, fragment)) => {
            out.normalized_url = Some(url);
            out.fragment = fragment;
        }
        Err(e) => out.error = Some(e.to_string()),
    }
    out
}
fn normalize_value(value: &str) -> Result<(String, Option<String>)> {
    ensure!(
        !value.chars().any(char::is_control),
        "URL contains raw control characters"
    );
    let parsed = Url::parse(value.trim()).context("invalid absolute URL")?;
    ensure!(
        matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some(),
        "only absolute HTTP/HTTPS URLs with a host are accepted"
    );
    ensure!(
        parsed.username().is_empty() && parsed.password().is_none(),
        "credential/userinfo URLs are not accepted"
    );
    let fragment = parsed.fragment().map(str::to_owned);
    let normalized =
        crate::normalize_url(value.trim(), None).context("URL normalization failed")?;
    Ok((normalized.to_string(), fragment))
}

impl ListImportOptions {
    pub fn validate(&self) -> Result<()> {
        self.limits.validate()?;
        match &self.format {
            ListImportFormat::Csv {
                column, start_row, ..
            }
            | ListImportFormat::Xlsx {
                column, start_row, ..
            } => {
                ensure!(
                    *start_row > 0 && *start_row <= self.limits.max_rows,
                    "start_row must be 1..=max_rows"
                );
                match column {
                    ListColumnSelector::Index { value } => ensure!(
                        *value > 0 && *value <= 16_384,
                        "column index must be 1..=16384"
                    ),
                    ListColumnSelector::Header { name, row } => {
                        ensure!(!name.is_empty(), "header name must not be empty");
                        self.limits.field(name)?;
                        ensure!(
                            *row > 0 && *row < *start_row,
                            "header row must precede start_row"
                        );
                    }
                }
            }
            _ => (),
        }
        if let ListImportFormat::Xlsx { sheet, .. } = &self.format {
            match sheet {
                ListSheetSelector::Name { value } => {
                    ensure!(!value.is_empty(), "sheet name must not be empty");
                    self.limits.field(value)?;
                }
                ListSheetSelector::Index { value } => ensure!(
                    *value > 0 && *value <= self.limits.max_sheets,
                    "sheet index must be 1..=max_sheets"
                ),
            }
        }
        Ok(())
    }
}

pub fn import_url_list(bytes: &[u8], options: &ListImportOptions) -> Result<ListImportResult> {
    options.validate()?;
    check_input(bytes, &options.limits)?;
    let mut result = ListImportResult { engine: LIST_IMPORT_ENGINE.into(), input_sha256: input_hash(bytes),
        input_bytes: bytes.len(), options: options.clone(), selected_sheet: None, selected_column: None,
        rows: Vec::new(), website_requests: 0, warnings: Vec::new(), limitations: vec![
            "Import does not start a crawl, follow links, evaluate formulas or fetch sitemap indexes.".into(),
            "Only absolute HTTP/HTTPS values are URL candidates; original values and duplicate rows are retained.".into(),
            "Independent import contract; complete original parser/filter parity is NOT_PROVEN.".into(),
        ] };
    match &options.format {
        ListImportFormat::Txt => {
            for (i, (_, value)) in text_lines(utf8(bytes)?, &options.limits)?
                .into_iter()
                .enumerate()
            {
                ensure!(i < options.limits.max_rows, "input exceeds max_rows");
                options.limits.field(value)?;
                let mut row = base_row(i + 1, i + 1, value.to_owned(), ListCandidateKind::PageUrl);
                row.source_line = Some(i + 1);
                result.rows.push(row);
            }
        }
        ListImportFormat::Csv {
            delimiter,
            column,
            start_row,
        } => {
            let records = parse_csv(utf8(bytes)?, delimiter.byte(), &options.limits)?;
            let selected =
                resolve_column(column, |row| records.get(row - 1).map(|r| r.fields.clone()))?;
            result.selected_column = Some(selected);
            for (i, record) in records.iter().enumerate().skip(start_row - 1) {
                let value = record.fields.get(selected - 1).cloned().unwrap_or_default();
                let mut row = base_row(
                    result.rows.len() + 1,
                    i + 1,
                    value,
                    ListCandidateKind::PageUrl,
                );
                row.source_line = Some(record.line);
                row.cell_type = "csv_string".into();
                if record.fields.len() < selected {
                    row.error = Some("selected CSV column is missing".into());
                }
                result.rows.push(row);
            }
        }
        ListImportFormat::Xml => import_xml(bytes, &options.limits, &mut result)?,
        ListImportFormat::Xlsx {
            sheet,
            column,
            start_row,
            formula_policy,
        } => import_xlsx(
            bytes,
            &options.limits,
            sheet,
            column,
            *start_row,
            *formula_policy,
            &mut result,
        )?,
    }
    ensure!(!result.rows.is_empty(), "selection contains no input rows");
    ensure!(
        result.rows.len() <= options.limits.max_rows,
        "selection exceeds max_rows"
    );
    let mut selected_bytes = 0usize;
    for row in &result.rows {
        selected_bytes = selected_bytes
            .checked_add(row.raw_value.len())
            .context("decoded output byte overflow")?;
        if let Some(
            ListFormulaMetadata::Normal { formula } | ListFormulaMetadata::Shared { formula, .. },
        ) = &row.formula_metadata
        {
            selected_bytes = selected_bytes
                .checked_add(formula.len())
                .context("decoded output byte overflow")?;
        }
        ensure!(
            selected_bytes <= options.limits.max_output_bytes,
            "decoded selected values/formulas exceed max_output_bytes"
        );
    }
    validate_import_result(&result)?;
    Ok(result)
}

/// Split CR/LF/CRLF without losing interior empty records or adding a terminator-only tail.
fn text_lines<'a>(text: &'a str, limits: &ListImportLimits) -> Result<Vec<(usize, &'a str)>> {
    let mut rows = Vec::new();
    let b = text.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\r' || b[i] == b'\n' {
            ensure!(rows.len() < limits.max_rows, "TXT exceeds max_rows");
            rows.push((start, &text[start..i]));
            if b[i] == b'\r' && b.get(i + 1) == Some(&b'\n') {
                i += 1;
            }
            i += 1;
            start = i;
        } else {
            i += 1;
        }
    }
    if start < b.len() {
        ensure!(rows.len() < limits.max_rows, "TXT exceeds max_rows");
        rows.push((start, &text[start..]));
    }
    if b.is_empty() {
        rows.push((0, ""));
    }
    Ok(rows)
}

struct CsvRecord {
    line: usize,
    fields: Vec<String>,
}
/// Strict bounded CSV framing preserves blank records, unlike a record iterator skipping them.
fn parse_csv(text: &str, delimiter: u8, limits: &ListImportLimits) -> Result<Vec<CsvRecord>> {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        Start,
        Bare,
        Quoted,
        Closed,
    }
    let b = text.as_bytes();
    let mut state = State::Start;
    let mut field = Vec::new();
    let mut fields = Vec::new();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    let mut record_line = 1;
    let mut field_count = 0usize;
    let mut finish_field = |field: &mut Vec<u8>, fields: &mut Vec<String>| -> Result<()> {
        field_count += 1;
        ensure!(
            field_count <= limits.max_csv_fields,
            "CSV exceeds max_csv_fields"
        );
        ensure!(fields.len() < 16_384, "CSV exceeds 16384 columns");
        fields.push(String::from_utf8(std::mem::take(field)).context("invalid CSV field UTF-8")?);
        Ok(())
    };
    while i < b.len() {
        let ch = b[i];
        if state != State::Quoted && (ch == b'\r' || ch == b'\n') {
            finish_field(&mut field, &mut fields)?;
            ensure!(out.len() < limits.max_rows, "CSV exceeds max_rows");
            out.push(CsvRecord {
                line: record_line,
                fields: std::mem::take(&mut fields),
            });
            if ch == b'\r' && b.get(i + 1) == Some(&b'\n') {
                i += 1;
            }
            line += 1;
            record_line = line;
            state = State::Start;
        } else if state != State::Quoted && ch == delimiter {
            finish_field(&mut field, &mut fields)?;
            state = State::Start;
        } else {
            match state {
                State::Start if ch == b'"' => state = State::Quoted,
                State::Start => {
                    state = State::Bare;
                    field.push(ch);
                }
                State::Bare => {
                    ensure!(ch != b'"', "quote inside unquoted CSV field");
                    field.push(ch);
                }
                State::Quoted if ch == b'"' => {
                    if b.get(i + 1) == Some(&b'"') {
                        field.push(ch);
                        i += 1;
                    } else {
                        state = State::Closed;
                    }
                }
                State::Quoted => {
                    field.push(ch);
                    if ch == b'\n' || (ch == b'\r' && b.get(i + 1) != Some(&b'\n')) {
                        line += 1;
                    }
                }
                State::Closed => bail!("characters after closing CSV quote"),
            }
            ensure!(
                field.len() <= limits.max_field_bytes,
                "CSV field exceeds max_field_bytes"
            );
        }
        i += 1;
    }
    ensure!(state != State::Quoted, "unterminated quoted CSV field");
    if state != State::Start || !field.is_empty() || !fields.is_empty() {
        finish_field(&mut field, &mut fields)?;
        ensure!(out.len() < limits.max_rows, "CSV exceeds max_rows");
        out.push(CsvRecord {
            line: record_line,
            fields,
        });
    }
    if out.is_empty() {
        out.push(CsvRecord {
            line: 1,
            fields: vec![String::new()],
        });
    }
    Ok(out)
}

fn resolve_column(
    column: &ListColumnSelector,
    mut fields: impl FnMut(usize) -> Option<Vec<String>>,
) -> Result<usize> {
    match column {
        ListColumnSelector::Index { value } => Ok(*value),
        ListColumnSelector::Header { name, row } => {
            let fields = fields(*row).context("selected header row is missing")?;
            let matches: Vec<_> = fields
                .iter()
                .enumerate()
                .filter(|(_, v)| *v == name)
                .map(|(i, _)| i + 1)
                .collect();
            ensure!(
                matches.len() == 1,
                "header selection requires exactly one matching header"
            );
            Ok(matches[0])
        }
    }
}

fn reference_text(reference: &quick_xml::events::BytesRef<'_>) -> Result<String> {
    if let Some(ch) = reference.resolve_char_ref()? {
        return Ok(ch.to_string());
    }
    match reference.as_ref() {
        "amp" => Ok("&".into()),
        "lt" => Ok("<".into()),
        "gt" => Ok(">".into()),
        "quot" => Ok("\"".into()),
        "apos" => Ok("'".into()),
        _ => bail!("unknown XML entity; DTD/entity expansion is unsupported"),
    }
}

fn import_xml(
    bytes: &[u8],
    limits: &ListImportLimits,
    result: &mut ListImportResult,
) -> Result<()> {
    let text = utf8(bytes)?;
    let mut budget = XmlBudget::default();
    preflight_xml(text.as_bytes(), limits, &mut budget)?;
    let mut reader = NsReader::from_str(text);
    reader.config_mut().expand_empty_elements = true;
    let mut stack: Vec<(String, bool)> = Vec::new();
    let mut root_seen = false;
    let mut root_closed = false;
    let mut kind = ListCandidateKind::PageUrl;
    let mut entry_name = "url";
    let mut entry: Option<(usize, Option<String>)> = None;
    let mut location = None;
    let mut buf = Vec::new();
    loop {
        let (namespace, event) = reader.read_resolved_event_into(&mut buf)?;
        let bound = matches!(namespace, ResolveResult::Bound(n) if n.as_ref() == SITEMAP_NS);
        match event {
            Event::Start(tag) => {
                let local = tag.local_name().as_ref().to_owned();
                if stack.is_empty() {
                    ensure!(
                        !root_seen && !root_closed,
                        "XML must contain exactly one root"
                    );
                    ensure!(bound, "XML root requires the sitemap 0.9 namespace");
                    match local.as_str() {
                        "urlset" => (),
                        "sitemapindex" => {
                            kind = ListCandidateKind::SitemapDocument;
                            entry_name = "sitemap";
                        }
                        _ => bail!("XML input must be a sitemap urlset or sitemapindex"),
                    }
                    root_seen = true;
                } else if stack.len() == 1 && bound && local.as_str() == entry_name {
                    ensure!(entry.is_none(), "nested sitemap entry is invalid");
                    ensure!(result.rows.len() < limits.max_rows, "XML exceeds max_rows");
                    entry = Some((result.rows.len() + 1, None));
                } else if stack.len() == 2 && entry.is_some() && bound && local == "loc" {
                    ensure!(
                        entry.as_ref().is_some_and(|(_, v)| v.is_none()),
                        "multiple loc values in sitemap entry"
                    );
                    ensure!(location.is_none(), "nested location is invalid");
                    location = Some(String::new());
                } else if location.is_some() {
                    bail!("nested markup inside sitemap loc is invalid");
                }
                stack.push((local, bound));
            }
            Event::End(_) => {
                let (local, ns) = stack.pop().context("unbalanced XML end tag")?;
                if stack.len() == 2
                    && ns
                    && local == "loc"
                    && let Some((_, value)) = &mut entry
                {
                    *value = Some(location.take().unwrap_or_default());
                } else if stack.len() == 1 && ns && local.as_str() == entry_name {
                    let (ordinal, value) = entry.take().context("invalid sitemap entry")?;
                    let missing = value.is_none();
                    let mut row = base_row(ordinal, ordinal, value.unwrap_or_default(), kind);
                    row.cell_type = "xml_location".into();
                    if missing {
                        row.error = Some("sitemap entry is missing its immediate loc value".into());
                    }
                    result.rows.push(row);
                }
                if stack.is_empty() {
                    root_closed = true;
                }
            }
            Event::Text(value) => {
                let value = value.xml10_content();
                if let Some(current) = &mut location {
                    current.push_str(&value);
                    limits.field(current)?;
                } else if stack.is_empty() {
                    ensure!(value.trim().is_empty(), "text outside XML root");
                }
            }
            Event::CData(value) => {
                if let Some(current) = &mut location {
                    current.push_str(&value.xml10_content());
                    limits.field(current)?;
                } else if stack.is_empty() {
                    bail!("CDATA outside XML root");
                }
            }
            Event::GeneralRef(value) => {
                let value = reference_text(&value)?;
                if let Some(current) = &mut location {
                    current.push_str(&value);
                    limits.field(current)?;
                } else if stack.is_empty() {
                    bail!("entity outside XML root");
                }
            }
            Event::DocType(_) => bail!("DTD/entity declarations are unsupported"),
            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }
    ensure!(
        root_seen && root_closed && stack.is_empty() && entry.is_none(),
        "incomplete sitemap XML"
    );
    if kind == ListCandidateKind::SitemapDocument {
        result.warnings.push("Sitemap index locations are sitemap-document candidates, not page seeds; no recursion or URL fetching occurred.".into());
    }
    result.limitations.push("XML import recognizes immediate sitemap loc values; it is not full sitemap-schema validation or alternate/image/video discovery.".into());
    Ok(())
}

#[derive(Default)]
struct XmlBudget {
    events: usize,
    shared_strings: usize,
    sheets: usize,
    warnings: BTreeSet<String>,
}
#[derive(Default)]
struct XmlInfo {
    last_row: usize,
    cells: usize,
    worksheet: bool,
    lexical_values: BTreeMap<(usize, usize), String>,
}

/// Validate allocation-driving metadata before constructing the eager workbook parser.
fn preflight_xml(
    bytes: &[u8],
    limits: &ListImportLimits,
    budget: &mut XmlBudget,
) -> Result<XmlInfo> {
    let text = utf8(bytes)?;
    let mut reader = quick_xml::Reader::from_str(text);
    reader.config_mut().expand_empty_elements = true;
    let mut info = XmlInfo::default();
    let mut stack: Vec<(String, usize)> = Vec::new();
    let mut root_seen = false;
    let mut root_closed = false;
    let mut last_cell = None;
    let mut declared_row = None;
    let mut previous_row = 0;
    let mut non_string_cell: Option<((usize, usize), Option<String>)> = None;
    let mut buf = Vec::new();
    loop {
        let event = reader.read_event_into(&mut buf)?;
        budget.events = budget
            .events
            .checked_add(1)
            .context("XML event count overflow")?;
        ensure!(
            budget.events <= limits.max_xml_events,
            "XML exceeds max_xml_events"
        );
        match event {
            Event::Start(tag) => {
                let name = tag.local_name().as_ref().to_owned();
                limits.field(tag.name().as_ref())?;
                ensure!(
                    stack.len() < limits.max_xml_depth,
                    "XML exceeds max_xml_depth"
                );
                if stack.is_empty() {
                    ensure!(!root_seen && !root_closed, "XML must contain one root");
                    root_seen = true;
                    info.worksheet = name == "worksheet";
                }
                let mut attrs = BTreeMap::new();
                for attr in tag.attributes() {
                    let attr = attr?;
                    let key = attr.key.as_ref().to_owned();
                    let value = attr
                        .normalized_value(quick_xml::XmlVersion::Explicit1_0)?
                        .into_owned();
                    limits.field(&key)?;
                    limits.field(&value)?;
                    ensure!(
                        attrs.insert(key, value).is_none(),
                        "duplicate XML attribute"
                    );
                }
                if name == "sst"
                    && let Some(count) = attrs.get("uniqueCount")
                {
                    let count: usize =
                        count.parse().context("invalid shared-string uniqueCount")?;
                    ensure!(
                        count <= limits.max_shared_strings,
                        "declared shared strings exceed max_shared_strings"
                    );
                }
                if name == "si" {
                    budget.shared_strings = budget
                        .shared_strings
                        .checked_add(1)
                        .context("shared-string count overflow")?;
                    ensure!(
                        budget.shared_strings <= limits.max_shared_strings,
                        "shared strings exceed max_shared_strings"
                    );
                }
                if name == "sheet" {
                    budget.sheets += 1;
                    ensure!(
                        budget.sheets <= limits.max_sheets,
                        "workbook exceeds max_sheets"
                    );
                }
                if name == "Relationship"
                    && attrs
                        .get("TargetMode")
                        .is_some_and(|v| v.eq_ignore_ascii_case("external"))
                {
                    let typ = attrs.get("Type").map(String::as_str).unwrap_or_default();
                    ensure!(
                        !typ.ends_with("/officeDocument") && !typ.ends_with("/worksheet"),
                        "external workbook/sheet relationships are unsupported"
                    );
                    budget.warnings.insert("External hyperlinks/book relationships are not executed or treated as URL cell values.".into());
                }
                if info.worksheet && name == "row" {
                    let row: usize = attrs
                        .get("r")
                        .context("worksheet row requires explicit r coordinate")?
                        .parse()
                        .context("invalid worksheet row coordinate")?;
                    ensure!(
                        row > previous_row && row <= 1_048_576,
                        "worksheet rows must be unique, increasing and valid"
                    );
                    previous_row = row;
                    declared_row = Some(row);
                    info.last_row = row;
                }
                if info.worksheet && name == "c" {
                    let pos = parse_cell(
                        attrs
                            .get("r")
                            .context("cell requires explicit A1 coordinate")?,
                    )?;
                    ensure!(
                        declared_row == Some(pos.0),
                        "cell coordinate does not match worksheet row"
                    );
                    ensure!(
                        last_cell.is_none_or(|previous| pos > previous),
                        "duplicate or out-of-order worksheet cell"
                    );
                    last_cell = Some(pos);
                    non_string_cell = match attrs.get("t").map(String::as_str) {
                        None | Some("n" | "b" | "d" | "e") => Some((pos, None)),
                        _ => None,
                    };
                    info.cells += 1;
                    ensure!(
                        info.cells <= limits.max_sheet_cells,
                        "worksheet exceeds max_sheet_cells"
                    );
                }
                if name == "v"
                    && let Some((_, value)) = &mut non_string_cell
                {
                    *value = Some(String::new());
                }
                stack.push((name, 0));
            }
            Event::End(_) => {
                let (name, count) = stack.pop().context("unbalanced XML end")?;
                if name == "row" {
                    declared_row = None;
                }
                if name == "c"
                    && let Some((position, value)) = non_string_cell.take()
                {
                    info.lexical_values
                        .insert(position, value.unwrap_or_default());
                }
                // Rich/shared strings may span multiple runs: bound the concatenated value too.
                let aggregate = stack
                    .iter()
                    .any(|(n, _)| matches!(n.as_str(), "si" | "c" | "loc" | "definedName"));
                if aggregate && let Some((_, parent_count)) = stack.last_mut() {
                    *parent_count = parent_count
                        .checked_add(count)
                        .context("XML text byte count overflow")?;
                    ensure!(
                        *parent_count <= limits.max_field_bytes,
                        "XML decoded field exceeds max_field_bytes"
                    );
                }
                if stack.is_empty() {
                    root_closed = true;
                }
            }
            Event::Text(value) => {
                let value = value.xml10_content();
                limits.field(&value)?;
                if stack.last().is_some_and(|(n, _)| n == "v")
                    && let Some((_, Some(lexical))) = &mut non_string_cell
                {
                    lexical.push_str(&value);
                }
                if stack
                    .iter()
                    .any(|(n, _)| matches!(n.as_str(), "si" | "c" | "loc" | "definedName"))
                {
                    let (_, count) = stack.last_mut().context("text outside XML root")?;
                    *count = count
                        .checked_add(value.len())
                        .context("XML field count overflow")?;
                    ensure!(
                        *count <= limits.max_field_bytes,
                        "XML decoded field exceeds max_field_bytes"
                    );
                } else if stack.is_empty() {
                    ensure!(value.trim().is_empty(), "text outside XML root");
                }
            }
            Event::CData(value) => {
                let value = value.xml10_content();
                if stack.last().is_some_and(|(n, _)| n == "v")
                    && let Some((_, Some(lexical))) = &mut non_string_cell
                {
                    lexical.push_str(&value);
                }
                let (_, count) = stack.last_mut().context("CDATA outside XML root")?;
                *count = count
                    .checked_add(value.len())
                    .context("XML field count overflow")?;
                ensure!(
                    *count <= limits.max_field_bytes,
                    "XML decoded field exceeds max_field_bytes"
                );
            }
            Event::GeneralRef(value) => {
                let value = reference_text(&value)?;
                if stack.last().is_some_and(|(n, _)| n == "v")
                    && let Some((_, Some(lexical))) = &mut non_string_cell
                {
                    lexical.push_str(&value);
                }
                let (_, count) = stack.last_mut().context("entity outside XML root")?;
                *count = count
                    .checked_add(value.len())
                    .context("XML field count overflow")?;
                ensure!(
                    *count <= limits.max_field_bytes,
                    "XML decoded field exceeds max_field_bytes"
                );
            }
            Event::Decl(value) => {
                if let Some(encoding) = value.encoding() {
                    ensure!(
                        encoding?.eq_ignore_ascii_case("utf-8"),
                        "XML requires UTF-8 encoding"
                    );
                }
            }
            Event::DocType(_) => bail!("DTD/entity declarations are unsupported"),
            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }
    ensure!(
        root_seen && root_closed && stack.is_empty(),
        "incomplete XML document"
    );
    Ok(info)
}

fn parse_cell(cell: &str) -> Result<(usize, usize)> {
    ensure!(
        !cell.is_empty() && cell.len() <= 10,
        "invalid A1 cell coordinate"
    );
    let split = cell
        .bytes()
        .position(|b| b.is_ascii_digit())
        .context("A1 cell requires row digits")?;
    ensure!(
        split > 0 && cell[..split].bytes().all(|b| b.is_ascii_uppercase()),
        "A1 column must contain uppercase letters"
    );
    ensure!(
        cell[split..].bytes().all(|b| b.is_ascii_digit()) && !cell[split..].starts_with('0'),
        "invalid A1 row"
    );
    let mut column = 0usize;
    for b in cell[..split].bytes() {
        column = column
            .checked_mul(26)
            .and_then(|v| v.checked_add((b - b'A' + 1) as usize))
            .context("A1 column overflow")?;
    }
    let row: usize = cell[split..].parse().context("A1 row overflow")?;
    ensure!(
        column <= 16_384 && row <= 1_048_576 && row > 0,
        "A1 coordinate exceeds XLSX bounds"
    );
    Ok((row, column))
}
fn cell_name(row: usize, column: usize) -> String {
    let mut n = column;
    let mut letters = Vec::new();
    while n > 0 {
        n -= 1;
        letters.push((b'A' + (n % 26) as u8) as char);
        n /= 26;
    }
    letters.reverse();
    format!("{}{row}", letters.into_iter().collect::<String>())
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16> {
    let end = offset.checked_add(2).context("ZIP offset overflow")?;
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..end)
            .context("truncated ZIP metadata")?
            .try_into()?,
    ))
}
fn u32_at(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset.checked_add(4).context("ZIP offset overflow")?;
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..end)
            .context("truncated ZIP metadata")?
            .try_into()?,
    ))
}
fn package_path(path: &str) -> Result<String> {
    ensure!(
        !path.is_empty() && !path.starts_with('/') && !path.contains(['\\', ':', '\0']),
        "invalid ZIP package path"
    );
    let trimmed = path.strip_suffix('/').unwrap_or(path);
    ensure!(
        !trimmed.is_empty()
            && trimmed
                .split('/')
                .all(|s| !s.is_empty() && s != "." && s != ".."),
        "ambiguous ZIP package path"
    );
    Ok(trimmed.to_ascii_lowercase())
}
/// Standard bounded ZIP only. ZIP64/multi-disk are explicitly rejected before archive allocation.
fn preflight_zip_directory(bytes: &[u8], limits: &ListImportLimits) -> Result<usize> {
    ensure!(bytes.len() >= 22, "truncated ZIP input");
    let lower = bytes.len().saturating_sub(22 + 65_535);
    let eocd = (lower..=bytes.len() - 22)
        .rev()
        .find(|&i| {
            bytes.get(i..i + 4) == Some(b"PK\x05\x06")
                && u16_at(bytes, i + 20).is_ok_and(|n| i + 22 + n as usize == bytes.len())
        })
        .context("ZIP end directory is missing or malformed")?;
    ensure!(
        u16_at(bytes, eocd + 4)? == 0 && u16_at(bytes, eocd + 6)? == 0,
        "multi-disk ZIP is unsupported"
    );
    let on_disk = u16_at(bytes, eocd + 8)?;
    let count = u16_at(bytes, eocd + 10)?;
    let size = u32_at(bytes, eocd + 12)?;
    let offset = u32_at(bytes, eocd + 16)?;
    ensure!(
        on_disk != u16::MAX && count != u16::MAX && size != u32::MAX && offset != u32::MAX,
        "ZIP64 input is unsupported within the bounded XLSX importer"
    );
    ensure!(on_disk == count, "inconsistent ZIP directory counts");
    let count = count as usize;
    ensure!(
        count > 0 && count <= limits.max_zip_entries,
        "ZIP exceeds max_zip_entries"
    );
    let start = offset as usize;
    let end = start
        .checked_add(size as usize)
        .context("ZIP directory overflow")?;
    ensure!(
        end == eocd && start >= 4 && (size as usize) >= count * 46,
        "invalid ZIP directory bounds"
    );
    let mut pos = start;
    let mut seen = BTreeSet::new();
    for _ in 0..count {
        ensure!(
            bytes.get(pos..pos + 4) == Some(b"PK\x01\x02"),
            "invalid ZIP central header"
        );
        ensure!(
            u16_at(bytes, pos + 8)? & 1 == 0,
            "encrypted ZIP is unsupported"
        );
        ensure!(
            matches!(u16_at(bytes, pos + 10)?, 0 | 8),
            "ZIP supports only stored/deflated entries"
        );
        ensure!(
            u32_at(bytes, pos + 20)? != u32::MAX
                && u32_at(bytes, pos + 24)? != u32::MAX
                && u32_at(bytes, pos + 42)? != u32::MAX,
            "ZIP64 entries are unsupported"
        );
        let name_len = u16_at(bytes, pos + 28)? as usize;
        let extra_len = u16_at(bytes, pos + 30)? as usize;
        let comment_len = u16_at(bytes, pos + 32)? as usize;
        ensure!(
            u16_at(bytes, pos + 34)? == 0,
            "multi-disk entry is unsupported"
        );
        let data = pos.checked_add(46).context("ZIP header overflow")?;
        let next = data
            .checked_add(name_len)
            .and_then(|v| v.checked_add(extra_len))
            .and_then(|v| v.checked_add(comment_len))
            .context("ZIP header overflow")?;
        ensure!(next <= end, "ZIP central header exceeds directory");
        let name = std::str::from_utf8(
            bytes
                .get(data..data + name_len)
                .context("truncated ZIP entry name")?,
        )?;
        limits.field(name)?;
        let canonical = package_path(name)?;
        ensure!(
            seen.insert(canonical),
            "duplicate/case-colliding ZIP package paths"
        );
        let mut extra = data + name_len;
        let extra_end = extra + extra_len;
        while extra < extra_end {
            ensure!(extra + 4 <= extra_end, "truncated ZIP extra field");
            ensure!(
                u16_at(bytes, extra)? != 1,
                "ZIP64 extra fields are unsupported"
            );
            extra = extra
                .checked_add(4 + u16_at(bytes, extra + 2)? as usize)
                .context("ZIP extra overflow")?;
            ensure!(extra <= extra_end, "invalid ZIP extra field length");
        }
        pos = next;
    }
    ensure!(pos == end, "ZIP count/directory bytes disagree");
    Ok(count)
}

struct ZipPreflight {
    sheet_rows: BTreeMap<String, usize>,
    sheet_lexical_values: BTreeMap<String, BTreeMap<(usize, usize), String>>,
    warnings: Vec<String>,
}
struct Relationship {
    id: String,
    typ: String,
    target: String,
    external: bool,
}
fn relationships(bytes: &[u8]) -> Result<Vec<Relationship>> {
    let mut reader = quick_xml::Reader::from_str(utf8(bytes)?);
    reader.config_mut().expand_empty_elements = true;
    let mut out = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(tag) if tag.local_name().as_ref() == "Relationship" => {
                let mut attrs = BTreeMap::new();
                for a in tag.attributes() {
                    let a = a?;
                    attrs.insert(
                        a.key.as_ref().to_owned(),
                        a.normalized_value(quick_xml::XmlVersion::Explicit1_0)?
                            .into_owned(),
                    );
                }
                out.push(Relationship {
                    id: attrs.remove("Id").context("relationship missing Id")?,
                    typ: attrs.remove("Type").context("relationship missing Type")?,
                    target: attrs
                        .remove("Target")
                        .context("relationship missing Target")?,
                    external: attrs
                        .get("TargetMode")
                        .is_some_and(|s| s.eq_ignore_ascii_case("external")),
                });
            }
            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }
    Ok(out)
}
fn relation_target(base: &str, target: &str) -> Result<String> {
    ensure!(
        !target.contains(['\\', ':', '\0', '?', '#']),
        "invalid internal package relationship target"
    );
    let mut parts = if target.starts_with('/') {
        Vec::new()
    } else {
        base.split('/')
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    for part in target.trim_start_matches('/').split('/') {
        match part {
            "" | "." => (),
            ".." => {
                ensure!(
                    parts.pop().is_some(),
                    "package relationship escapes archive"
                );
            }
            v => parts.push(v.into()),
        }
    }
    package_path(&parts.join("/"))
}
fn preflight_xlsx(bytes: &[u8], limits: &ListImportLimits) -> Result<ZipPreflight> {
    let count = preflight_zip_directory(bytes, limits)?;
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).context("invalid XLSX ZIP archive")?;
    ensure!(
        archive.len() == count,
        "ZIP parsed entry count disagrees with bounded metadata"
    );
    let mut entries = BTreeMap::new();
    let mut infos = BTreeMap::new();
    let mut total = 0usize;
    let mut budget = XmlBudget::default();
    for i in 0..count {
        let mut file = archive
            .by_index(i)
            .context("invalid/encrypted XLSX ZIP entry")?;
        ensure!(
            !file.encrypted() && !file.is_symlink(),
            "encrypted/symlink ZIP entries are unsupported"
        );
        let name = package_path(file.name())?;
        ensure!(
            file.size() <= limits.max_zip_entry_bytes as u64,
            "declared ZIP entry exceeds max_zip_entry_bytes"
        );
        if file.is_dir() {
            continue;
        }
        let budget_left = limits
            .max_zip_total_bytes
            .checked_sub(total)
            .context("ZIP total byte overflow")?;
        let cap = limits.max_zip_entry_bytes.min(budget_left);
        let mut contents = Vec::new();
        file.by_ref()
            .take((cap + 1) as u64)
            .read_to_end(&mut contents)
            .context("ZIP entry inflation/CRC failed")?;
        ensure!(
            contents.len() <= cap,
            "actual ZIP inflation exceeds entry/total byte limit"
        );
        total = total
            .checked_add(contents.len())
            .context("ZIP total byte overflow")?;
        ensure!(
            total <= limits.max_zip_total_bytes,
            "ZIP exceeds max_zip_total_bytes"
        );
        ensure!(
            !name.ends_with("vbaproject.bin"),
            "macro-enabled workbook is unsupported; select an XLSX document"
        );
        if name.ends_with(".xml") || name.ends_with(".rels") {
            let info = preflight_xml(&contents, limits, &mut budget)
                .with_context(|| format!("invalid bounded XLSX XML part {name}"))?;
            infos.insert(name.clone(), info);
        }
        entries.insert(name, contents);
    }
    let content_types = utf8(
        entries
            .get("[content_types].xml")
            .context("XLSX content types are missing")?,
    )?;
    ensure!(
        content_types
            .contains("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml")
            && !content_types.contains("macroEnabled")
            && !content_types.contains("vbaProject"),
        "input is not a supported XLSX workbook"
    );
    let package = relationships(
        entries
            .get("_rels/.rels")
            .context("XLSX package relationships are missing")?,
    )?;
    let office: Vec<_> = package
        .iter()
        .filter(|r| r.typ.ends_with("/officeDocument"))
        .collect();
    ensure!(
        office.len() == 1 && !office[0].external,
        "XLSX needs one internal workbook relationship"
    );
    let workbook_path = relation_target("", &office[0].target)?;
    let (base, filename) = workbook_path
        .rsplit_once('/')
        .unwrap_or(("", workbook_path.as_str()));
    let rel_path = if base.is_empty() {
        format!("_rels/{filename}.rels")
    } else {
        format!("{base}/_rels/{filename}.rels")
    };
    let rels = relationships(
        entries
            .get(&rel_path)
            .context("XLSX workbook relationships are missing")?,
    )?;
    let mut by_id = BTreeMap::new();
    for rel in rels {
        ensure!(
            by_id.insert(rel.id.clone(), rel).is_none(),
            "duplicate workbook relationship Id"
        );
    }
    let mut reader = quick_xml::Reader::from_str(utf8(
        entries
            .get(&workbook_path)
            .context("XLSX workbook XML is missing")?,
    )?);
    reader.config_mut().expand_empty_elements = true;
    let mut sheet_rows = BTreeMap::new();
    let mut sheet_lexical_values = BTreeMap::new();
    let mut names = BTreeSet::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(tag) if tag.local_name().as_ref() == "sheet" => {
                let mut name = None;
                let mut id = None;
                for a in tag.attributes() {
                    let a = a?;
                    let value = a
                        .normalized_value(quick_xml::XmlVersion::Explicit1_0)?
                        .into_owned();
                    if a.key.as_ref() == "name" {
                        name = Some(value);
                    } else if a.key.local_name().as_ref() == "id" {
                        id = Some(value);
                    }
                }
                let name = name.context("XLSX sheet missing name")?;
                ensure!(
                    names.insert(name.to_lowercase()),
                    "duplicate/case-colliding worksheet names"
                );
                let rel = by_id
                    .get(&id.context("XLSX sheet missing relationship id")?)
                    .context("XLSX sheet relationship is missing")?;
                ensure!(
                    !rel.external,
                    "external worksheet relationships are unsupported"
                );
                let path = relation_target(base, &rel.target)?;
                let info = infos
                    .get(&path)
                    .context("XLSX sheet XML target is missing")?;
                if rel.typ.ends_with("/worksheet") {
                    ensure!(
                        info.worksheet,
                        "worksheet relationship target is not a worksheet"
                    );
                    sheet_lexical_values.insert(name.clone(), info.lexical_values.clone());
                    sheet_rows.insert(name, info.last_row);
                }
            }
            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }
    Ok(ZipPreflight {
        sheet_rows,
        sheet_lexical_values,
        warnings: budget.warnings.into_iter().collect(),
    })
}

fn sheet_inventory(book: &Xlsx<Cursor<&[u8]>>) -> Vec<ListImportSheet> {
    book.sheets_metadata()
        .iter()
        .enumerate()
        .map(|(i, sheet)| ListImportSheet {
            index: i + 1,
            name: sheet.name.clone(),
            sheet_type: format!("{:?}", sheet.typ),
            visibility: format!("{:?}", sheet.visible),
        })
        .collect()
}
pub fn inspect_xlsx_list(bytes: &[u8], limits: &ListImportLimits) -> Result<ListImportInspection> {
    check_input(bytes, limits)?;
    let preflight = preflight_xlsx(bytes, limits)?;
    let book = Xlsx::new(Cursor::new(bytes)).context("XLSX metadata inspection failed")?;
    let sheets = sheet_inventory(&book);
    ensure!(
        sheets.len() <= limits.max_sheets,
        "workbook exceeds max_sheets"
    );
    Ok(ListImportInspection { engine: LIST_IMPORT_ENGINE.into(), input_sha256: input_hash(bytes),
        input_bytes: bytes.len(), sheets, website_requests: 0, warnings: preflight.warnings,
        limitations: vec!["Inspection lists sheet metadata only; it does not infer URL columns, evaluate formulas or start a crawl.".into(),
            "ZIP64, encrypted/macro-enabled workbooks and non-UTF-8 XML parts are explicitly unsupported.".into()] })
}

struct CellValue {
    raw: String,
    cell_type: String,
    is_string: bool,
    formula: Option<ListFormulaMetadata>,
}
fn own_cell(
    value: DataRef<'_>,
    formula: Option<XlsxFormulaMetadata>,
    limits: &ListImportLimits,
) -> Result<CellValue> {
    let (raw, typ, is_string) = match value {
        DataRef::String(s) => (s, "string", true),
        DataRef::SharedString(s) => (s.to_owned(), "shared_string", true),
        DataRef::Int(v) => (v.to_string(), "integer", false),
        DataRef::Float(v) => (v.to_string(), "float", false),
        DataRef::Bool(v) => (v.to_string(), "boolean", false),
        DataRef::Empty => (String::new(), "empty", false),
        DataRef::Error(v) => (format!("{v}"), "error", false),
        DataRef::DateTime(v) => (format!("{v:?}"), "date_time", false),
        DataRef::DateTimeIso(v) => (v, "date_time_iso", false),
        DataRef::DurationIso(v) => (v, "duration_iso", false),
    };
    limits.field(&raw)?;
    let formula = formula.map(|f| match f {
        XlsxFormulaMetadata::Normal { formula } => ListFormulaMetadata::Normal { formula },
        XlsxFormulaMetadata::Shared {
            shared_index,
            range,
            formula,
        } => ListFormulaMetadata::Shared {
            shared_index,
            range: range.map(|d| format!("{d:?}")),
            formula,
        },
        XlsxFormulaMetadata::SharedDerived { shared_index } => {
            ListFormulaMetadata::SharedDerived { shared_index }
        }
        _ => ListFormulaMetadata::Unsupported,
    });
    if let Some(
        ListFormulaMetadata::Normal { formula } | ListFormulaMetadata::Shared { formula, .. },
    ) = &formula
    {
        limits.field(formula)?;
    }
    Ok(CellValue {
        raw,
        cell_type: typ.into(),
        is_string,
        formula,
    })
}

fn import_xlsx(
    bytes: &[u8],
    limits: &ListImportLimits,
    sheet: &ListSheetSelector,
    column: &ListColumnSelector,
    start_row: usize,
    formula_policy: ListFormulaPolicy,
    result: &mut ListImportResult,
) -> Result<()> {
    let preflight = preflight_xlsx(bytes, limits)?;
    let mut book =
        Xlsx::new(Cursor::new(bytes)).context("XLSX parser failed after bounded preflight")?;
    let inventory = sheet_inventory(&book);
    let selected = match sheet {
        ListSheetSelector::Index { value } => inventory
            .get(value - 1)
            .context("selected sheet index is missing")?,
        ListSheetSelector::Name { value } => inventory
            .iter()
            .find(|s| &s.name == value)
            .context("selected exact sheet name is missing")?,
    };
    let last_row = *preflight
        .sheet_rows
        .get(&selected.name)
        .context("selected sheet is not an importable worksheet")?;
    ensure!(
        last_row >= start_row,
        "selected worksheet contains no data rows"
    );
    ensure!(
        last_row - start_row < limits.max_rows,
        "selected worksheet row span exceeds max_rows"
    );
    let header_row = match column {
        ListColumnSelector::Header { row, .. } => Some(*row),
        _ => None,
    };
    let mut reader = book.worksheet_cells_reader(&selected.name)?;
    let mut values = BTreeMap::new();
    let mut header = BTreeMap::new();
    let mut cells = 0usize;
    // Only selected column values (and explicit header cells) are retained, never a dense Range.
    let mut selected_column = match column {
        ListColumnSelector::Index { value } => Some(*value),
        _ => None,
    };
    let mut retained_bytes = 0usize;
    while let Some(record) = reader.next_cell_with_formula_metadata()? {
        cells += 1;
        ensure!(
            cells <= limits.max_sheet_cells,
            "selected sheet exceeds max_sheet_cells"
        );
        let row = record.pos.0 as usize + 1;
        let col = record.pos.1 as usize + 1;
        if Some(row) == header_row {
            let value = own_cell(record.value, record.formula, limits)?;
            charge_cell(&value, &mut retained_bytes, limits)?;
            ensure!(
                value.formula.is_none(),
                "formula-derived header selection is unsupported"
            );
            header.insert(
                col,
                if value.is_string {
                    value.raw
                } else {
                    String::new()
                },
            );
        } else if row >= start_row {
            if selected_column.is_none() {
                selected_column = Some(resolve_column(column, |_| {
                    let max = header.keys().next_back().copied()?;
                    Some(
                        (1..=max)
                            .map(|i| header.get(&i).cloned().unwrap_or_default())
                            .collect(),
                    )
                })?);
            }
            if selected_column == Some(col) {
                let mut value = own_cell(record.value, record.formula, limits)?;
                if !value.is_string
                    && let Some(lexical) = preflight
                        .sheet_lexical_values
                        .get(&selected.name)
                        .and_then(|cells| cells.get(&(row, col)))
                {
                    value.raw = lexical.clone();
                }
                charge_cell(&value, &mut retained_bytes, limits)?;
                values.insert(row, value);
            }
        }
    }
    if selected_column.is_none() {
        selected_column = Some(resolve_column(column, |_| {
            let max = header.keys().next_back().copied()?;
            Some(
                (1..=max)
                    .map(|i| header.get(&i).cloned().unwrap_or_default())
                    .collect(),
            )
        })?);
    }
    let selected_column = selected_column.context("selected column is missing")?;
    result.selected_sheet = Some(selected.clone());
    result.selected_column = Some(selected_column);
    result.warnings.extend(preflight.warnings);
    for row_number in start_row..=last_row {
        let value = values.remove(&row_number);
        let mut row = base_row(
            result.rows.len() + 1,
            row_number,
            value.as_ref().map(|v| v.raw.clone()).unwrap_or_default(),
            ListCandidateKind::PageUrl,
        );
        row.sheet = Some(selected.name.clone());
        row.cell = Some(cell_name(row_number, selected_column));
        match value {
            None => {
                row.cell_type = "missing".into();
                row.error = Some("selected XLSX cell is missing".into());
            }
            Some(value) => {
                row.cell_type = value.cell_type;
                row.formula_metadata = value.formula;
                if row.formula_metadata.is_some() && formula_policy == ListFormulaPolicy::Reject {
                    row.error = Some("formula cells require explicit cached_string_only policy; formulas are never evaluated".into());
                } else if !value.is_string {
                    row.error = Some("selected XLSX cell/cache is not a string URL value".into());
                }
                if row.error.is_some() {
                    row.normalized_url = None;
                    row.fragment = None;
                }
            }
        }
        result.rows.push(row);
    }
    result.limitations.extend([
        "XLSX cell raw_value means decoded stored literal/cache; numeric/date display formatting and hyperlink targets are not URL values.".into(),
        "Formula caches may be stale; even explicitly accepted string caches are never evaluated or refreshed.".into(),
        "ZIP64, encrypted/macro-enabled workbooks and non-UTF-8 XML parts are explicitly unsupported.".into(),
    ]);
    Ok(())
}

fn charge_cell(value: &CellValue, total: &mut usize, limits: &ListImportLimits) -> Result<()> {
    *total = total
        .checked_add(value.raw.len())
        .context("decoded retained byte overflow")?;
    if let Some(
        ListFormulaMetadata::Normal { formula } | ListFormulaMetadata::Shared { formula, .. },
    ) = &value.formula
    {
        *total = total
            .checked_add(formula.len())
            .context("decoded retained byte overflow")?;
    }
    ensure!(
        *total <= limits.max_output_bytes,
        "decoded selected/header values/formulas exceed max_output_bytes"
    );
    Ok(())
}

/// Validate transmitted provenance before seed derivation or CSV export.
/// The declared digest cannot attest original bytes after those bytes are discarded.
pub fn validate_import_result(result: &ListImportResult) -> Result<()> {
    result.options.validate()?;
    let limits = &result.options.limits;
    ensure!(
        result.engine == LIST_IMPORT_ENGINE,
        "unsupported list-import engine"
    );
    ensure!(
        result.website_requests == 0,
        "offline import must declare zero website requests"
    );
    ensure!(
        result.input_bytes > 0 && result.input_bytes <= limits.max_input_bytes,
        "invalid declared input byte count"
    );
    ensure!(
        result.input_sha256.len() == 64
            && result.input_sha256.bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid declared input SHA256"
    );
    ensure!(
        !result.rows.is_empty() && result.rows.len() <= limits.max_rows,
        "invalid imported row count"
    );
    ensure!(
        result.warnings.len() <= 64 && result.limitations.len() <= 64,
        "too many provenance warnings/limitations"
    );
    let mut total = serde_json::to_vec(&result.options)?.len();
    let mut charge = |value: &str| -> Result<()> {
        ensure!(
            value.len() <= ListImportLimits::default().max_field_bytes,
            "provenance metadata field exceeds hard ceiling"
        );
        total = total
            .checked_add(value.len())
            .context("provenance byte count overflow")?;
        ensure!(
            total <= limits.max_output_bytes,
            "provenance values/output/metadata exceed max_output_bytes"
        );
        Ok(())
    };
    charge(&result.engine)?;
    charge(&result.input_sha256)?;
    for field in result.warnings.iter().chain(result.limitations.iter()) {
        charge(field)?;
    }
    let (start, column, sheet, policy, cell_types): (
        usize,
        Option<usize>,
        Option<&ListImportSheet>,
        Option<ListFormulaPolicy>,
        &[&str],
    ) = match &result.options.format {
        ListImportFormat::Txt => {
            ensure!(
                result.selected_column.is_none() && result.selected_sheet.is_none(),
                "TXT import cannot claim sheet/column selection"
            );
            (1, None, None, None, &["text"])
        }
        ListImportFormat::Xml => {
            ensure!(
                result.selected_column.is_none() && result.selected_sheet.is_none(),
                "XML import cannot claim sheet/column selection"
            );
            (1, None, None, None, &["xml_location"])
        }
        ListImportFormat::Csv {
            column, start_row, ..
        } => {
            ensure!(
                result.selected_sheet.is_none(),
                "CSV import cannot claim a workbook sheet"
            );
            let selected = result
                .selected_column
                .context("CSV import must identify selected column")?;
            ensure!(
                selected > 0 && selected <= 16_384,
                "invalid selected column"
            );
            if let ListColumnSelector::Index { value } = column {
                ensure!(
                    *value == selected,
                    "selected CSV column contradicts options"
                );
            }
            (*start_row, Some(selected), None, None, &["csv_string"])
        }
        ListImportFormat::Xlsx {
            sheet,
            column,
            start_row,
            formula_policy,
        } => {
            let selected = result
                .selected_sheet
                .as_ref()
                .context("XLSX import must identify selected sheet")?;
            ensure!(
                selected.index > 0
                    && selected.index <= limits.max_sheets
                    && !selected.name.is_empty(),
                "invalid selected sheet"
            );
            limits.field(&selected.name)?;
            ensure!(
                selected.sheet_type == "WorkSheet"
                    && matches!(
                        selected.visibility.as_str(),
                        "Visible" | "Hidden" | "VeryHidden"
                    ),
                "invalid selected sheet type/visibility"
            );
            match sheet {
                ListSheetSelector::Name { value } => ensure!(
                    value == &selected.name,
                    "selected XLSX sheet contradicts name option"
                ),
                ListSheetSelector::Index { value } => ensure!(
                    *value == selected.index,
                    "selected XLSX sheet contradicts index option"
                ),
            }
            let selected_col = result
                .selected_column
                .context("XLSX import must identify selected column")?;
            ensure!(
                selected_col > 0 && selected_col <= 16_384,
                "invalid selected column"
            );
            if let ListColumnSelector::Index { value } = column {
                ensure!(
                    *value == selected_col,
                    "selected XLSX column contradicts options"
                );
            }
            charge(&selected.name)?;
            charge(&selected.sheet_type)?;
            charge(&selected.visibility)?;
            (
                *start_row,
                Some(selected_col),
                Some(selected),
                Some(*formula_policy),
                &[
                    "string",
                    "shared_string",
                    "integer",
                    "float",
                    "boolean",
                    "empty",
                    "missing",
                    "error",
                    "date_time",
                    "date_time_iso",
                    "duration_iso",
                ],
            )
        }
    };
    let mut last_line = 0usize;
    let first_kind = result.rows[0].candidate_kind;
    for (i, row) in result.rows.iter().enumerate() {
        ensure!(
            row.source_index == i + 1,
            "source_index must be sequential and one-based"
        );
        ensure!(
            row.source_row == start.checked_add(i).context("source row overflow")?,
            "source_row must preserve increasing input occurrence order"
        );
        ensure!(
            row.source_row > 0 && row.source_row <= 1_048_576,
            "invalid source row"
        );
        limits.field(&row.raw_value)?;
        ensure!(
            cell_types.contains(&row.cell_type.as_str()),
            "cell_type contradicts import format"
        );
        if matches!(result.options.format, ListImportFormat::Xml) {
            ensure!(
                row.candidate_kind == first_kind,
                "one sitemap document cannot mix page/index candidate kinds"
            );
        } else {
            ensure!(
                row.candidate_kind == ListCandidateKind::PageUrl,
                "non-XML import cannot claim sitemap-document candidates"
            );
        }
        match &result.options.format {
            ListImportFormat::Txt => ensure!(
                row.source_line == Some(row.source_row),
                "TXT line identity contradicts row"
            ),
            ListImportFormat::Csv { .. } => {
                let line = row.source_line.context("CSV source line is missing")?;
                ensure!(
                    line > last_line && line >= row.source_row && line <= result.input_bytes + 1,
                    "invalid CSV source line order"
                );
                last_line = line;
            }
            _ => ensure!(
                row.source_line.is_none(),
                "unmeasured XML/XLSX line must be null"
            ),
        }
        if let Some(sheet) = sheet {
            ensure!(
                row.sheet.as_deref() == Some(sheet.name.as_str()),
                "row sheet contradicts selection"
            );
            ensure!(
                row.cell.as_deref() == Some(cell_name(row.source_row, column.unwrap()).as_str()),
                "row A1 coordinate contradicts selection/source row"
            );
        } else {
            ensure!(
                row.sheet.is_none() && row.cell.is_none(),
                "non-XLSX row cannot claim sheet/cell"
            );
        }
        if let Some(formula) = &row.formula_metadata {
            ensure!(
                policy.is_some(),
                "non-XLSX row cannot claim formula metadata"
            );
            match formula {
                ListFormulaMetadata::Normal { formula } => {
                    limits.field(formula)?;
                    charge(formula)?;
                }
                ListFormulaMetadata::Shared { formula, range, .. } => {
                    limits.field(formula)?;
                    charge(formula)?;
                    if let Some(range) = range {
                        limits.field(range)?;
                        charge(range)?;
                    }
                }
                ListFormulaMetadata::SharedDerived { .. } => (),
                ListFormulaMetadata::Unsupported => ensure!(
                    row.error.is_some(),
                    "unsupported formula metadata cannot produce a URL"
                ),
            }
            if policy == Some(ListFormulaPolicy::Reject) {
                ensure!(
                    row.error.is_some(),
                    "reject formula policy cannot accept formula-derived URL"
                );
            }
        }
        charge(&row.raw_value)?;
        charge(&row.cell_type)?;
        for field in [
            &row.sheet,
            &row.cell,
            &row.normalized_url,
            &row.fragment,
            &row.error,
        ]
        .into_iter()
        .flatten()
        {
            charge(field)?;
        }
        if let Some(error) = &row.error {
            ensure!(
                !error.is_empty() && row.normalized_url.is_none() && row.fragment.is_none(),
                "error row cannot also claim normalized URL/fragment"
            );
        } else {
            if policy.is_some() {
                ensure!(
                    matches!(row.cell_type.as_str(), "string" | "shared_string"),
                    "only stored/cached string cells can produce URLs"
                );
            }
            let (normalized, fragment) =
                normalize_value(&row.raw_value).context("successful row has invalid raw URL")?;
            ensure!(
                row.normalized_url.as_deref() == Some(normalized.as_str())
                    && row.fragment == fragment,
                "normalized URL/fragment must derive from the same raw value"
            );
        }
    }
    // Count the actual serialized result, including row objects, escaping and numeric
    // metadata. The sink never allocates an output-sized buffer.
    struct OutputBudget {
        written: usize,
        cap: usize,
    }
    impl std::io::Write for OutputBudget {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let next = self
                .written
                .checked_add(bytes.len())
                .ok_or_else(|| std::io::Error::other("output byte count overflow"))?;
            if next > self.cap {
                return Err(std::io::Error::other(
                    "serialized provenance exceeds max_output_bytes",
                ));
            }
            self.written = next;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(
        OutputBudget {
            written: 0,
            cap: limits.max_output_bytes,
        },
        result,
    )
    .context("serialized provenance exceeds max_output_bytes")?;
    Ok(())
}

pub fn export_url_list_csv(result: &ListImportResult) -> Result<String> {
    validate_import_result(result)?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "Source Index",
        "Source Row",
        "Source Line",
        "Sheet",
        "Cell",
        "Raw Value",
        "Cell Type",
        "Candidate Kind",
        "Normalized URL",
        "Fragment",
        "Error",
        "Formula Metadata",
    ])?;
    for row in &result.rows {
        writer.write_record([
            row.source_index.to_string(),
            row.source_row.to_string(),
            row.source_line.map(|n| n.to_string()).unwrap_or_default(),
            crate::export::csv_text(row.sheet.as_deref().unwrap_or_default()),
            crate::export::csv_text(row.cell.as_deref().unwrap_or_default()),
            crate::export::csv_text(&row.raw_value),
            crate::export::csv_text(&row.cell_type),
            format!("{:?}", row.candidate_kind),
            crate::export::csv_text(row.normalized_url.as_deref().unwrap_or_default()),
            crate::export::csv_text(row.fragment.as_deref().unwrap_or_default()),
            crate::export::csv_text(row.error.as_deref().unwrap_or_default()),
            crate::export::csv_text(
                &row.formula_metadata
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()?
                    .unwrap_or_default(),
            ),
        ])?;
    }
    String::from_utf8(writer.into_inner()?).context("CSV output UTF-8 error")
}

#[cfg(test)]
mod tests {
    use super::*;
    const XLSX: &[u8] = include_bytes!("../tests/fixtures/list-import/ordered-formulas.xlsx");
    fn options(format: ListImportFormat) -> ListImportOptions {
        ListImportOptions {
            format,
            limits: ListImportLimits::default(),
        }
    }
    fn xlsx_options(policy: ListFormulaPolicy) -> ListImportOptions {
        options(ListImportFormat::Xlsx {
            sheet: ListSheetSelector::Name {
                value: "URLs".into(),
            },
            column: ListColumnSelector::Header {
                name: "URL".into(),
                row: 1,
            },
            start_row: 2,
            formula_policy: policy,
        })
    }
    #[test]
    fn txt_keeps_order_duplicate_bad_rows_query_and_fragment_identity() {
        let bytes = b"\xef\xbb\xbfhttps://EXAMPLE.test/Same?b=2&a=1#one\r\n\r\nhttps://EXAMPLE.test/Same?b=2&a=1#one\rftp://example.test/a\nhttps://user:secret@example.test/\nnot a URL\n";
        let result = import_url_list(bytes, &options(ListImportFormat::Txt)).unwrap();
        assert_eq!(result.rows.len(), 6);
        assert_eq!(result.website_requests, 0);
        assert_eq!(result.input_sha256, input_hash(bytes));
        assert_eq!(
            result.rows[0].raw_value,
            "https://EXAMPLE.test/Same?b=2&a=1#one"
        );
        assert_eq!(
            result.rows[0].normalized_url.as_deref(),
            Some("https://example.test/Same?b=2&a=1")
        );
        assert_eq!(result.rows[0].fragment.as_deref(), Some("one"));
        assert_eq!(result.rows[0].raw_value, result.rows[2].raw_value);
        assert_eq!(
            result.rows.iter().map(|r| r.source_row).collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6]
        );
        for i in [1, 3, 4, 5] {
            assert!(result.rows[i].error.is_some());
            assert!(result.rows[i].normalized_url.is_none());
        }
    }
    #[test]
    fn utf8_and_controls_fail_explicitly_without_silent_fixups() {
        assert!(import_url_list(b"\xff", &options(ListImportFormat::Txt)).is_err());
        let result = import_url_list(
            "https://b\u{00fc}cher.example/%C3%A4?x=1&x=2\nhttps://example.test/\tbad".as_bytes(),
            &options(ListImportFormat::Txt),
        )
        .unwrap();
        assert_eq!(
            result.rows[0].normalized_url.as_deref(),
            Some("https://xn--bcher-kva.example/%C3%A4?x=1&x=2")
        );
        assert!(result.rows[1].error.as_deref().unwrap().contains("control"));
    }
    #[test]
    fn csv_quotes_multiline_blank_ragged_and_duplicate_rows_have_real_coordinates() {
        let config = options(ListImportFormat::Csv {
            delimiter: ListCsvDelimiter::Comma,
            column: ListColumnSelector::Header {
                name: "URL".into(),
                row: 1,
            },
            start_row: 2,
        });
        let bytes = b"Name,URL\r\na,\"https://example.test/a?x=1,y=2\"\r\n\r\nb,\"https://example.test/line\npart\"\r\nc\r\nd,https://example.test/a?x=1\r\ne,https://example.test/a?x=1\r\n";
        let result = import_url_list(bytes, &config).unwrap();
        assert_eq!(result.selected_column, Some(2));
        assert_eq!(result.rows.len(), 6);
        assert_eq!(result.rows[0].raw_value, "https://example.test/a?x=1,y=2");
        assert_eq!(result.rows[1].source_row, 3);
        assert_eq!(result.rows[1].source_line, Some(3));
        assert_eq!(result.rows[2].raw_value, "https://example.test/line\npart");
        assert_eq!(result.rows[3].source_row, 5);
        assert_eq!(result.rows[3].source_line, Some(6));
        assert!(result.rows[1..4].iter().all(|r| r.error.is_some()));
        assert_eq!(result.rows[4].raw_value, result.rows[5].raw_value);
    }
    #[test]
    fn csv_selection_and_syntax_ambiguity_are_document_errors() {
        let config = options(ListImportFormat::Csv {
            delimiter: ListCsvDelimiter::Semicolon,
            column: ListColumnSelector::Header {
                name: "URL".into(),
                row: 1,
            },
            start_row: 2,
        });
        for text in [
            "URL;URL\na;b",
            "Wrong\na",
            "URL\n\"unterminated",
            "URL\na\"quote",
            "URL\n\"closed\"tail",
        ] {
            assert!(import_url_list(text.as_bytes(), &config).is_err(), "{text}");
        }
        let result = import_url_list(b"URL\n\"https://example.test/\"\n", &config).unwrap();
        assert_eq!(result.rows.len(), 1);
    }
    #[test]
    fn xml_namespace_entities_and_extensions_do_not_create_false_page_seeds() {
        let text = r#"<s:urlset xmlns:s="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:i="http://www.google.com/schemas/sitemap-image/1.1"><s:url><s:loc>https://example.test/a?b=1&amp;c=2#f</s:loc><i:image><i:loc>https://never-fetch.test/image</i:loc></i:image></s:url><s:url><s:loc>https://example.test/a?b=1&amp;c=2#f</s:loc></s:url><s:url/><s:url><s:loc>ftp://example.test/a</s:loc></s:url></s:urlset>"#;
        let result = import_url_list(text.as_bytes(), &options(ListImportFormat::Xml)).unwrap();
        assert_eq!(result.rows.len(), 4);
        assert_eq!(result.website_requests, 0);
        assert_eq!(result.rows[0].raw_value, "https://example.test/a?b=1&c=2#f");
        assert_eq!(result.rows[0].raw_value, result.rows[1].raw_value);
        assert_eq!(result.rows[0].candidate_kind, ListCandidateKind::PageUrl);
        assert!(result.rows[2].error.as_deref().unwrap().contains("missing"));
        assert!(result.rows[3].error.is_some());
    }
    #[test]
    fn xml_index_stays_sitemap_document_even_if_url_is_bad() {
        let text = r#"<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><sitemap><loc><![CDATA[https://example.test/sitemap.xml.gz]]></loc></sitemap><sitemap><loc>bad</loc></sitemap></sitemapindex>"#;
        let result = import_url_list(text.as_bytes(), &options(ListImportFormat::Xml)).unwrap();
        assert!(
            result
                .rows
                .iter()
                .all(|r| r.candidate_kind == ListCandidateKind::SitemapDocument)
        );
        assert!(result.rows[0].normalized_url.is_some());
        assert!(result.rows[1].error.is_some());
        assert_eq!(result.website_requests, 0);
        assert!(!result.warnings.is_empty());
    }
    #[test]
    fn xml_rejects_entity_dtd_wrong_root_namespace_multiple_loc_and_truncation() {
        for text in [
            "<!DOCTYPE urlset [<!ENTITY x SYSTEM 'https://never-fetch.test/entity'>]><urlset/>",
            "<urlset><url><loc>https://example.test/</loc></url></urlset>",
            "<urlset xmlns='wrong'/>",
            "<wrong xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'/>",
            "<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>https://example.test/&unknown;</loc></url></urlset>",
            "<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>a</loc><loc>b</loc></url></urlset>",
            "<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>a</loc>",
        ] {
            assert!(
                import_url_list(text.as_bytes(), &options(ListImportFormat::Xml)).is_err(),
                "{text}"
            );
        }
    }
    #[test]
    fn xlsx_inspection_is_explicit_and_reports_hidden_sheet_metadata() {
        let inspection = inspect_xlsx_list(XLSX, &ListImportLimits::default()).unwrap();
        assert_eq!(
            inspection
                .sheets
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            ["URLs", "Other", "Hidden"]
        );
        assert_eq!(inspection.sheets[2].visibility, "Hidden");
        assert_eq!(inspection.website_requests, 0);
        assert!(inspection.warnings.iter().any(|w| w.contains("External")));
        let config = options(ListImportFormat::Xlsx {
            sheet: ListSheetSelector::Index { value: 2 },
            column: ListColumnSelector::Index { value: 1 },
            start_row: 2,
            formula_policy: ListFormulaPolicy::Reject,
        });
        let result = import_url_list(XLSX, &config).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].cell.as_deref(), Some("A2"));
        assert_eq!(result.rows[0].raw_value, "https://example.test/sheet2");
    }
    #[test]
    fn xlsx_stream_keeps_duplicate_sparse_and_trailing_empty_rows_without_dense_dimensions() {
        let result = import_url_list(XLSX, &xlsx_options(ListFormulaPolicy::Reject)).unwrap();
        assert_eq!(result.selected_column, Some(2));
        assert_eq!(result.rows.len(), 13);
        assert_eq!(result.rows[0].cell.as_deref(), Some("B2"));
        assert_eq!(result.rows[12].cell.as_deref(), Some("B14"));
        assert_eq!(result.rows[0].raw_value, result.rows[1].raw_value);
        assert_eq!(
            result.rows[0].raw_value,
            "https://example.test/same?b=2&a=1#part"
        );
        assert_eq!(
            result.rows[0].normalized_url.as_deref(),
            Some("https://example.test/same?b=2&a=1")
        );
        assert!(result.rows[2].error.as_deref().unwrap().contains("missing"));
        assert!(
            result.rows[12]
                .error
                .as_deref()
                .unwrap()
                .contains("missing")
        );
        for i in [3, 4, 5, 6, 7, 8, 9, 10] {
            assert!(result.rows[i].normalized_url.is_none(), "row{}", i + 2);
        }
        assert_eq!(result.rows[11].fragment.as_deref(), Some("kept"));
        assert_eq!(result.website_requests, 0);
    }
    #[test]
    fn xlsx_formula_caches_require_explicit_policy_and_preserve_shared_provenance() {
        let result =
            import_url_list(XLSX, &xlsx_options(ListFormulaPolicy::CachedStringOnly)).unwrap();
        assert_eq!(
            result.rows[4].normalized_url.as_deref(),
            Some("https://example.test/cached")
        );
        assert!(matches!(
            result.rows[4].formula_metadata,
            Some(ListFormulaMetadata::Normal { .. })
        ));
        assert!(matches!(
            result.rows[5].formula_metadata,
            Some(ListFormulaMetadata::Shared { .. })
        ));
        assert!(matches!(
            result.rows[6].formula_metadata,
            Some(ListFormulaMetadata::SharedDerived { .. })
        ));
        assert_eq!(
            result.rows[6].normalized_url.as_deref(),
            Some("https://example.test/derived")
        );
        assert!(result.rows[10].normalized_url.is_none()); // missing cached value
        assert!(result.rows[9].normalized_url.is_none()); // external hyperlink label is not substituted
        assert_eq!(result.rows[9].raw_value, "open-me");
        assert_eq!(result.website_requests, 0);
    }
    #[test]
    fn xlsx_hostile_metadata_and_coordinates_fail_before_workbook_allocation() {
        let config = xlsx_options(ListFormulaPolicy::Reject);
        for bytes in [
            include_bytes!("../tests/fixtures/list-import/huge-shared-count.xlsx").as_slice(),
            include_bytes!("../tests/fixtures/list-import/huge-observed-row.xlsx").as_slice(),
            include_bytes!("../tests/fixtures/list-import/duplicate-cell.xlsx").as_slice(),
            include_bytes!("../tests/fixtures/list-import/wrong-cell-row.xlsx").as_slice(),
            include_bytes!("../tests/fixtures/list-import/external-worksheet.xlsx").as_slice(),
            include_bytes!("../tests/fixtures/list-import/case-colliding-entry.xlsx").as_slice(),
        ] {
            assert!(import_url_list(bytes, &config).is_err());
        }
        let error = inspect_xlsx_list(
            include_bytes!("../tests/fixtures/list-import/huge-shared-count.xlsx"),
            &config.limits,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("bounded XLSX XML"));
    }
    #[test]
    fn actual_zip_inflation_and_directory_metadata_limits_are_enforced() {
        let mut limits = ListImportLimits {
            max_zip_entry_bytes: 4096,
            ..ListImportLimits::default()
        };
        let bytes = include_bytes!("../tests/fixtures/list-import/inflation-budget.xlsx");
        assert!(inspect_xlsx_list(bytes, &limits).is_err());
        let mut malformed = XLSX.to_vec();
        let end = malformed.len() - 22;
        malformed[end + 10..end + 12].copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(
            inspect_xlsx_list(&malformed, &ListImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("ZIP64")
        );
        limits = ListImportLimits::default();
        limits.max_zip_entries = 1;
        assert!(inspect_xlsx_list(XLSX, &limits).is_err());
        limits = ListImportLimits::default();
        limits.max_zip_total_bytes = 100;
        assert!(inspect_xlsx_list(XLSX, &limits).is_err());
    }
    #[test]
    fn actual_inflation_is_bounded_even_when_zip_size_metadata_lies() {
        let mut bytes =
            include_bytes!("../tests/fixtures/list-import/inflation-budget.xlsx").to_vec();
        let end = bytes.len() - 22;
        let mut cursor = u32::from_le_bytes(bytes[end + 16..end + 20].try_into().unwrap()) as usize;
        let mut changed = false;
        while cursor < end && &bytes[cursor..cursor + 4] == b"PK\x01\x02" {
            let name_len =
                u16::from_le_bytes(bytes[cursor + 28..cursor + 30].try_into().unwrap()) as usize;
            let extra_len =
                u16::from_le_bytes(bytes[cursor + 30..cursor + 32].try_into().unwrap()) as usize;
            let comment_len =
                u16::from_le_bytes(bytes[cursor + 32..cursor + 34].try_into().unwrap()) as usize;
            if &bytes[cursor + 46..cursor + 46 + name_len] == b"xl/sharedStrings.xml" {
                let local = u32::from_le_bytes(bytes[cursor + 42..cursor + 46].try_into().unwrap())
                    as usize;
                bytes[cursor + 24..cursor + 28].copy_from_slice(&1u32.to_le_bytes());
                bytes[local + 22..local + 26].copy_from_slice(&1u32.to_le_bytes());
                changed = true;
            }
            cursor += 46 + name_len + extra_len + comment_len;
        }
        assert!(changed);
        let limits = ListImportLimits {
            max_zip_entry_bytes: 4096,
            ..ListImportLimits::default()
        };
        let error = inspect_xlsx_list(&bytes, &limits).unwrap_err().to_string();
        assert!(error.contains("actual ZIP inflation"), "{error}");
    }
    #[test]
    fn xlsx_missing_ambiguous_selection_is_a_document_error() {
        let config = xlsx_options(ListFormulaPolicy::Reject);
        assert!(
            import_url_list(
                include_bytes!("../tests/fixtures/list-import/duplicate-header.xlsx"),
                &config
            )
            .is_err()
        );
        for format in [
            ListImportFormat::Xlsx {
                sheet: ListSheetSelector::Name {
                    value: "Absent".into(),
                },
                column: ListColumnSelector::Index { value: 1 },
                start_row: 2,
                formula_policy: ListFormulaPolicy::Reject,
            },
            ListImportFormat::Xlsx {
                sheet: ListSheetSelector::Index { value: 4 },
                column: ListColumnSelector::Index { value: 1 },
                start_row: 2,
                formula_policy: ListFormulaPolicy::Reject,
            },
            ListImportFormat::Xlsx {
                sheet: ListSheetSelector::Index { value: 0 },
                column: ListColumnSelector::Index { value: 1 },
                start_row: 2,
                formula_policy: ListFormulaPolicy::Reject,
            },
            ListImportFormat::Xlsx {
                sheet: ListSheetSelector::Index { value: 1 },
                column: ListColumnSelector::Header {
                    name: "Missing".into(),
                    row: 1,
                },
                start_row: 2,
                formula_policy: ListFormulaPolicy::Reject,
            },
        ] {
            assert!(import_url_list(XLSX, &options(format)).is_err());
        }
    }
    #[test]
    fn transmitted_results_are_validated_before_seed_derivation_and_csv() {
        let good = import_url_list(
            b"https://example.test/a#f\nhttps://example.test/a#f\nbad",
            &options(ListImportFormat::Txt),
        )
        .unwrap();
        let mut candidates = Vec::new();
        macro_rules! altered {
            ($change:expr) => {{
                let mut item = good.clone();
                $change(&mut item);
                candidates.push(item);
            }};
        }
        altered!(|v: &mut ListImportResult| v.engine = "unknown".into());
        altered!(|v: &mut ListImportResult| v.website_requests = 1);
        altered!(|v: &mut ListImportResult| v.input_sha256 = "z".repeat(64));
        altered!(|v: &mut ListImportResult| v.input_bytes = 0);
        altered!(|v: &mut ListImportResult| v.rows[1].source_index = 1);
        altered!(|v: &mut ListImportResult| v.rows[1].source_row = 1);
        altered!(|v: &mut ListImportResult| v.rows[0].source_line = None);
        altered!(|v: &mut ListImportResult| v.rows[0].normalized_url =
            Some("https://other.test/".into()));
        altered!(|v: &mut ListImportResult| v.rows[0].fragment = Some("other".into()));
        altered!(|v: &mut ListImportResult| v.rows[2].normalized_url =
            Some("https://example.test/".into()));
        altered!(|v: &mut ListImportResult| v.rows[0].cell_type = "string".into());
        altered!(|v: &mut ListImportResult| v.rows[0].sheet = Some("fake".into()));
        altered!(|v: &mut ListImportResult| v.rows[0].candidate_kind =
            ListCandidateKind::SitemapDocument);
        altered!(|v: &mut ListImportResult| v.rows[0].formula_metadata =
            Some(ListFormulaMetadata::Unsupported));
        altered!(|v: &mut ListImportResult| v.warnings = vec!["x".repeat(8193)]);
        altered!(|v: &mut ListImportResult| v.warnings = vec!["x".into(); 65]);
        altered!(|v: &mut ListImportResult| v.options.limits.max_output_bytes = 1);
        for item in candidates {
            assert!(validate_import_result(&item).is_err());
            assert!(export_url_list_csv(&item).is_err());
        }
        let mut unknown = serde_json::to_value(&good).unwrap();
        unknown["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ListImportResult>(unknown).is_err());
        let mut unknown = serde_json::to_value(&good).unwrap();
        unknown["rows"][0]["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ListImportResult>(unknown).is_err());
        assert!(validate_import_result(&good).is_ok());
        let mut bounded = good.clone();
        bounded.options.limits.max_output_bytes = serde_json::to_vec(&good).unwrap().len() - 100;
        assert!(
            validate_import_result(&bounded)
                .unwrap_err()
                .to_string()
                .contains("serialized provenance")
        );
    }
    #[test]
    fn transmitted_xlsx_formula_type_and_coordinates_cannot_contradict_selection() {
        let good =
            import_url_list(XLSX, &xlsx_options(ListFormulaPolicy::CachedStringOnly)).unwrap();
        for index in 0..5 {
            let mut item = good.clone();
            match index {
                0 => item.rows[0].cell = Some("C2".into()),
                1 => item.rows[0].cell_type = "boolean".into(),
                2 => {
                    if let ListImportFormat::Xlsx { formula_policy, .. } = &mut item.options.format
                    {
                        *formula_policy = ListFormulaPolicy::Reject;
                    }
                }
                3 => item.rows[4].formula_metadata = Some(ListFormulaMetadata::Unsupported),
                _ => item.selected_sheet.as_mut().unwrap().name = "Other".into(),
            }
            assert!(validate_import_result(&item).is_err(), "mutation {index}");
            assert!(export_url_list_csv(&item).is_err());
        }
        assert!(validate_import_result(&good).is_ok());
    }
    #[test]
    fn every_bound_is_positive_and_rows_fields_events_depth_are_not_silently_truncated() {
        let object = serde_json::to_value(ListImportLimits::default()).unwrap();
        for key in object.as_object().unwrap().keys() {
            let mut changed = object.clone();
            changed[key] = serde_json::json!(0);
            assert!(
                serde_json::from_value::<ListImportLimits>(changed)
                    .unwrap()
                    .validate()
                    .is_err(),
                "{key}"
            );
            let mut changed = object.clone();
            changed[key] = serde_json::json!(object[key].as_u64().unwrap() + 1);
            assert!(
                serde_json::from_value::<ListImportLimits>(changed)
                    .unwrap()
                    .validate()
                    .is_err(),
                "{key} ceiling"
            );
        }
        let mut config = options(ListImportFormat::Txt);
        config.limits.max_rows = 1;
        assert!(
            import_url_list(b"https://example.test/a\nhttps://example.test/b", &config).is_err()
        );
        config.limits.max_rows = 2;
        config.limits.max_field_bytes = 1;
        assert!(import_url_list(b"https://example.test/a", &config).is_err());
        config = options(ListImportFormat::Xml);
        config.limits.max_xml_depth = 1;
        assert!(
            import_url_list(
                b"<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url/></urlset>",
                &config
            )
            .is_err()
        );
        config.limits.max_xml_depth = 64;
        config.limits.max_xml_events = 1;
        assert!(
            import_url_list(
                b"<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'/>",
                &config
            )
            .is_err()
        );
    }

    #[test]
    fn typed_stored_numeric_boolean_and_date_values_are_not_fabricated_as_display_text() {
        let result = import_url_list(XLSX, &xlsx_options(ListFormulaPolicy::Reject)).unwrap();
        assert_eq!(result.rows[3].raw_value, "1");
        assert_eq!(result.rows[3].cell_type, "boolean");
        assert_eq!(result.rows[8].raw_value, "2026-10-12T00:00:00Z");
        assert!(result.rows[3].normalized_url.is_none());
        assert!(result.rows[8].normalized_url.is_none());
        let mut config = xlsx_options(ListFormulaPolicy::CachedStringOnly);
        config.limits.max_output_bytes = 1;
        assert!(import_url_list(XLSX, &config).is_err());
        config = options(ListImportFormat::Csv {
            delimiter: ListCsvDelimiter::Comma,
            column: ListColumnSelector::Index { value: 1 },
            start_row: 1,
        });
        config.limits.max_csv_fields = 1;
        assert!(import_url_list(b"https://example.test/a,unused", &config).is_err());
    }
    #[test]
    fn csv_export_retains_invalid_original_rows_with_shared_formula_neutralization() {
        let result = import_url_list(b"=HYPERLINK(\"https://never-fetch.test\")\nhttps://example.test/a\n=HYPERLINK(\"https://never-fetch.test\")", &options(ListImportFormat::Txt)).unwrap();
        let output = export_url_list_csv(&result).unwrap();
        let mut reader = csv::Reader::from_reader(output.as_bytes());
        let records = reader
            .records()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(records.len(), 3);
        assert!(records[0].get(5).unwrap().starts_with("'="));
        assert_eq!(records[0].get(5), records[2].get(5));
        assert_eq!(
            result.rows[0].raw_value,
            "=HYPERLINK(\"https://never-fetch.test\")"
        );
        assert!(records[0].get(8).unwrap().is_empty());
    }
}
