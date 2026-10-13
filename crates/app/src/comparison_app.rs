//! Shared saved-report comparison request, bounded exports and local-file CLI.
use open_seo_spider_core::{
    ComparisonCallerBinding, ComparisonError, ComparisonSide, CrawlReport, SAVED_COMPARISON_ENGINE,
    SavedComparisonConfig, SavedComparisonLimits, SavedComparisonResult, compare_saved_crawls,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    io::Write,
    path::{Path, PathBuf},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

pub(crate) const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const MAX_CSV_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedComparisonRequest {
    pub before: Uuid,
    pub after: Uuid,
    #[serde(default)]
    pub config: Option<SavedComparisonConfig>,
    #[serde(default)]
    pub expected_before_generation: Option<u64>,
    #[serde(default)]
    pub expected_after_generation: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedComparisonResponse {
    pub comparison: SavedComparisonResult,
    pub website_requests: usize,
}
impl SavedComparisonResponse {
    pub fn new(comparison: SavedComparisonResult) -> Result<Self, SavedComparisonFailure> {
        let response = Self {
            comparison,
            website_requests: 0,
        };
        let mut counter = ByteBudget {
            writer: std::io::sink(),
            remaining: MAX_RESPONSE_BYTES,
        };
        serde_json::to_writer(&mut counter, &response)
            .map_err(|error| failure("comparison_response_byte_budget", error.to_string()))?;
        Ok(response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComparisonConflictContext {
    pub side: ComparisonSide,
    pub job_id: Uuid,
    pub status: String,
    pub observed_generation: u64,
    pub expected_generation: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedComparisonFailure {
    pub error: ComparisonError,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub context: Option<Box<ComparisonConflictContext>>,
    pub website_requests: usize,
}
impl From<ComparisonError> for SavedComparisonFailure {
    fn from(error: ComparisonError) -> Self {
        Self {
            error,
            context: None,
            website_requests: 0,
        }
    }
}
pub(crate) fn failure(code: &str, message: impl Into<String>) -> SavedComparisonFailure {
    ComparisonError {
        code: code.into(),
        message: message.into(),
        collisions: Vec::new(),
    }
    .into()
}

// Advertised MCP schema mirrors the strict shared request model. Core performs
// cross-field URL/mapping/content integrity validation; this is not attestation.
pub(crate) fn request_schema() -> Value {
    let ceiling = SavedComparisonLimits::default();
    let mut limit_properties = serde_json::Map::new();
    for (name, maximum) in [
        ("max_pages_per_snapshot", ceiling.max_pages_per_snapshot),
        ("max_mappings", ceiling.max_mappings),
        ("max_url_bytes", ceiling.max_url_bytes),
        ("max_input_json_bytes", ceiling.max_input_json_bytes),
        ("max_output_json_bytes", ceiling.max_output_json_bytes),
    ] {
        limit_properties.insert(
            name.into(),
            json!({"type":"integer","minimum":1,"maximum":maximum}),
        );
    }
    let config = json!({
        "type":"object","additionalProperties":false,"properties":{
            "engine":{"type":"string","enum":[SAVED_COMPARISON_ENGINE]},
            "include_selected_text":{"type":"boolean"},
            "limits":{"type":"object","additionalProperties":false,"properties":limit_properties},
            "mappings":{"type":"array","maxItems":ceiling.max_mappings,"items":{
                "type":"object","additionalProperties":false,
                "required":["side","from_origin","from_path_prefix","to_origin","to_path_prefix"],
                "properties":{
                    "side":{"type":"string","enum":["before","after"]},
                    "from_origin":{"type":"string","maxLength":ceiling.max_url_bytes},
                    "from_path_prefix":{"type":"string","maxLength":ceiling.max_url_bytes},
                    "to_origin":{"type":"string","maxLength":ceiling.max_url_bytes},
                    "to_path_prefix":{"type":"string","maxLength":ceiling.max_url_bytes}
                }
            }}
        }
    });
    let generation =
        json!({"anyOf":[{"type":"integer","minimum":0,"maximum":u64::MAX},{"type":"null"}]});
    json!({"type":"object","additionalProperties":false,"required":["before","after"],"properties":{
        "before":{"type":"string","format":"uuid"},"after":{"type":"string","format":"uuid"},
        "config":{"anyOf":[config,{"type":"null"}]},
        "expected_before_generation":generation,"expected_after_generation":generation
    }})
}

struct ByteBudget<W> {
    writer: W,
    remaining: usize,
}
impl<W: Write> Write for ByteBudget<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(std::io::Error::other(
                "comparison output byte budget exceeded",
            ));
        }
        let written = self.writer.write(bytes)?;
        self.remaining -= written;
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

fn enum_value(value: &impl Serialize) -> Result<String, SavedComparisonFailure> {
    serde_json::to_value(value)
        .map_err(|error| failure("csv_serialization", error.to_string()))?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| failure("csv_serialization", "Expected a comparison enum string"))
}
fn json_cell(value: &impl Serialize) -> Result<String, SavedComparisonFailure> {
    serde_json::to_string(value).map_err(|error| failure("csv_serialization", error.to_string()))
}
fn csv_record(writer: &mut impl Write, fields: &[String]) -> Result<(), SavedComparisonFailure> {
    for (index, field) in fields.iter().enumerate() {
        if index != 0 {
            writer.write_all(b",").map_err(csv_io)?;
        }
        let protected = open_seo_spider_core::csv_text(field);
        writer.write_all(b"\"").map_err(csv_io)?;
        writer
            .write_all(protected.replace('"', "\"\"").as_bytes())
            .map_err(csv_io)?;
        writer.write_all(b"\"").map_err(csv_io)?;
    }
    writer.write_all(b"\r\n").map_err(csv_io)
}
fn csv_io(error: std::io::Error) -> SavedComparisonFailure {
    failure("comparison_csv_write", error.to_string())
}
pub(crate) fn write_csv(
    result: &SavedComparisonResult,
    writer: &mut impl Write,
) -> Result<(), SavedComparisonFailure> {
    let mut bounded = ByteBudget {
        writer,
        remaining: MAX_CSV_BYTES,
    };
    csv_record(
        &mut bounded,
        &[
            "page_kind",
            "comparison_key",
            "before_requested_url",
            "after_requested_url",
            "before_ordinal",
            "after_ordinal",
            "field",
            "outcome",
            "before_state",
            "after_state",
            "before_value_json",
            "after_value_json",
            "availability_changed",
            "comparable_complete_values",
            "reason",
            "before_profile",
            "after_profile",
            "before_provenance_json",
            "after_provenance_json",
            "before_snapshot_sha256",
            "after_snapshot_sha256",
        ]
        .map(String::from),
    )?;
    for page in &result.pages {
        for field in &page.fields {
            csv_record(
                &mut bounded,
                &[
                    enum_value(&page.kind)?,
                    page.comparison_key.clone(),
                    page.before
                        .as_ref()
                        .map(|value| value.original_requested_url.clone())
                        .unwrap_or_default(),
                    page.after
                        .as_ref()
                        .map(|value| value.original_requested_url.clone())
                        .unwrap_or_default(),
                    page.before
                        .as_ref()
                        .map(|value| value.ordinal.to_string())
                        .unwrap_or_default(),
                    page.after
                        .as_ref()
                        .map(|value| value.ordinal.to_string())
                        .unwrap_or_default(),
                    field.field.clone(),
                    enum_value(&field.outcome)?,
                    enum_value(&field.before.state)?,
                    enum_value(&field.after.state)?,
                    json_cell(&field.before.value)?,
                    json_cell(&field.after.value)?,
                    field.availability_changed.to_string(),
                    field.comparable_complete_values.to_string(),
                    field.reason.clone().unwrap_or_default(),
                    field.before.profile.clone().unwrap_or_default(),
                    field.after.profile.clone().unwrap_or_default(),
                    json_cell(&field.before.provenance)?,
                    json_cell(&field.after.provenance)?,
                    result.before_snapshot.typed_report_sha256.clone(),
                    result.after_snapshot.typed_report_sha256.clone(),
                ],
            )?;
        }
    }
    bounded.flush().map_err(csv_io)
}

async fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, SavedComparisonFailure> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|error| failure("saved_input_read", format!("{}: {error}", path.display())))?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| failure("saved_input_read", error.to_string()))?;
    if bytes.len() > limit {
        return Err(failure(
            "saved_input_byte_budget",
            format!("{} exceeds {limit} bytes", path.display()),
        ));
    }
    Ok(bytes)
}
fn parse_json<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    path: &Path,
) -> Result<T, SavedComparisonFailure> {
    serde_json::from_slice(bytes)
        .map_err(|error| failure("invalid_saved_json", format!("{}: {error}", path.display())))
}
async fn destination(path: &Path) -> Result<PathBuf, SavedComparisonFailure> {
    if tokio::fs::symlink_metadata(path).await.is_ok() {
        return Err(failure(
            "output_destination_exists",
            format!("{} must be a new file", path.display()),
        ));
    }
    let parent = path
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path.file_name().ok_or_else(|| {
        failure(
            "invalid_output_destination",
            "Output destination needs a file name",
        )
    })?;
    let resolved = tokio::fs::canonicalize(parent)
        .await
        .map_err(|error| failure("invalid_output_destination", error.to_string()))?
        .join(name);
    #[cfg(windows)]
    {
        Ok(PathBuf::from(
            resolved.as_os_str().to_string_lossy().to_lowercase(),
        ))
    }
    #[cfg(not(windows))]
    {
        Ok(resolved)
    }
}
fn file_binding() -> ComparisonCallerBinding {
    ComparisonCallerBinding {
        origin: Some("cli_declared_saved_json_file".into()),
        basis: Some("app_declared_typed_local_file_report_not_sql_attested".into()),
        active_checkpoint: Some(false),
        ..Default::default()
    }
}

pub(crate) async fn run_cli(
    before: PathBuf,
    after: PathBuf,
    config_file: Option<PathBuf>,
    output: Option<PathBuf>,
    csv: Option<PathBuf>,
) -> Result<(), SavedComparisonFailure> {
    let mut destinations = HashSet::new();
    for path in output.iter().chain(csv.iter()) {
        if !destinations.insert(destination(path).await?) {
            return Err(failure(
                "duplicate_output_destination",
                "JSON and CSV destinations must differ",
            ));
        }
    }
    let config = if let Some(path) = config_file {
        parse_json::<SavedComparisonConfig>(&read_bounded(&path, MAX_REQUEST_BYTES).await?, &path)?
    } else {
        SavedComparisonConfig::default()
    };
    let maximum = SavedComparisonLimits::default().max_input_json_bytes;
    let before_bytes = read_bounded(&before, maximum).await?;
    let after_bytes = read_bounded(&after, maximum - before_bytes.len()).await?;
    let before_report: CrawlReport = parse_json(&before_bytes, &before)?;
    let after_report: CrawlReport = parse_json(&after_bytes, &after)?;
    drop(before_bytes);
    drop(after_bytes);
    let response = tokio::task::spawn_blocking(move || {
        let binding = file_binding();
        let result = compare_saved_crawls(
            &before_report,
            &after_report,
            &config,
            Some(&binding),
            Some(&binding),
        )
        .map_err(SavedComparisonFailure::from)?;
        SavedComparisonResponse::new(result)
    })
    .await
    .map_err(|error| failure("comparison_task_failed", error.to_string()))??;
    // Each reservation is atomic create_new. Two-file export is not a filesystem
    // transaction: an IO failure may leave newly created empty/partial artifacts.
    let mut output_file = if let Some(path) = output {
        Some(
            tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .await
                .map_err(|error| {
                    failure(
                        "output_reservation_failed",
                        format!("{}: {error}", path.display()),
                    )
                })?,
        )
    } else {
        None
    };
    let csv_file = if let Some(path) = csv {
        Some(
            tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .await
                .map_err(|error| {
                    failure(
                        "output_reservation_failed",
                        format!("{}: {error}", path.display()),
                    )
                })?,
        )
    } else {
        None
    };
    let response = if let Some(csv_file) = csv_file {
        let file = csv_file.into_std().await;
        tokio::task::spawn_blocking(move || {
            let mut writer = std::io::BufWriter::new(file);
            write_csv(&response.comparison, &mut writer)?;
            writer.flush().map_err(csv_io)?;
            writer.get_ref().sync_all().map_err(csv_io)?;
            Ok::<_, SavedComparisonFailure>(response)
        })
        .await
        .map_err(|error| failure("comparison_csv_task_failed", error.to_string()))??
    } else {
        response
    };
    let bytes = serde_json::to_vec(&response)
        .map_err(|error| failure("comparison_json_serialization", error.to_string()))?;
    if let Some(file) = output_file.as_mut() {
        file.write_all(&bytes)
            .await
            .map_err(|error| failure("comparison_json_write", error.to_string()))?;
        file.sync_all()
            .await
            .map_err(|error| failure("comparison_json_write", error.to_string()))?;
    } else {
        let mut stdout = tokio::io::stdout();
        stdout
            .write_all(&bytes)
            .await
            .map_err(|error| failure("comparison_stdout_write", error.to_string()))?;
        stdout
            .write_all(b"\n")
            .await
            .map_err(|error| failure("comparison_stdout_write", error.to_string()))?;
        stdout
            .flush()
            .await
            .map_err(|error| failure("comparison_stdout_write", error.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "comparison_app_tests.rs"]
mod tests;
