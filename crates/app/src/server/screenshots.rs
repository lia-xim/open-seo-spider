//! Read-only saved screenshot artifacts. No renderer, website or filesystem URL lookup.
use super::*;
use axum::extract::rejection::QueryRejection;
use sha2::{Digest, Sha256};
use std::io::{self, Write};

pub(super) const ENGINE: &str = "open-seo-spider/screenshot-v1";
const SNAPSHOT_DOMAIN: &str = "open-seo-spider/screenshot-snapshot-v1";
const PNG_BYTES: usize = 64 * 1024 * 1024;
const CHUNKS: usize = 10_000;
const METADATA_BYTES: usize = 65_536;
const MAX_DIMENSION: u32 = 4096;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MetadataQuery {
    url: String,
    ordinal: Option<usize>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PngQuery {
    url: String,
    ordinal: usize,
    expected_snapshot_sha256: String,
    expected_image_sha256: String,
    expected_generation: Option<u64>,
}
#[derive(Debug, Serialize)]
pub(super) struct Failure {
    error: &'static str,
    message: &'static str,
}
type Result<T> = std::result::Result<T, (StatusCode, Json<Failure>)>;
fn fail(
    status: StatusCode,
    error: &'static str,
    message: &'static str,
) -> (StatusCode, Json<Failure>) {
    (status, Json(Failure { error, message }))
}
fn invalid() -> (StatusCode, Json<Failure>) {
    fail(
        StatusCode::BAD_REQUEST,
        "invalid_screenshot_request",
        "Invalid saved screenshot query",
    )
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hash_text(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn validate_url(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 8192 || value.chars().any(char::is_control) {
        return Err(invalid());
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct Metadata<'a> {
    engine: &'static str,
    version: u32,
    id: Uuid,
    ordinal: usize,
    url: &'a str,
    final_url: &'a str,
    generation: Option<u64>,
    snapshot_sha256: String,
    available: bool,
    status: &'static str,
    reason: Option<&'static str>,
    viewport: Option<&'a Value>,
    png_sha256: Option<String>,
    bytes: Option<usize>,
    width: Option<u32>,
    height: Option<u32>,
}
struct Observation<'a> {
    metadata: Metadata<'a>,
    png: Option<Vec<u8>>,
}
fn select<'a>(
    report: &'a CrawlReport,
    url: &str,
    ordinal: Option<usize>,
) -> Result<(usize, &'a PageRecord)> {
    if let Some(ordinal) = ordinal {
        let page = report.pages.get(ordinal).ok_or_else(|| {
            fail(
                StatusCode::NOT_FOUND,
                "screenshot_page_not_found",
                "Saved page ordinal was not found",
            )
        })?;
        if page.url != url {
            return Err(fail(
                StatusCode::CONFLICT,
                "screenshot_identity_changed",
                "URL and saved page ordinal identify different observations",
            ));
        }
        return Ok((ordinal, page));
    }
    let mut matches = report
        .pages
        .iter()
        .enumerate()
        .filter(|(_, page)| page.url == url);
    let selected = matches.next().ok_or_else(|| {
        fail(
            StatusCode::NOT_FOUND,
            "screenshot_page_not_found",
            "Saved page URL was not found",
        )
    })?;
    if matches.next().is_some() {
        return Err(fail(
            StatusCode::CONFLICT,
            "screenshot_url_ambiguous",
            "This URL has multiple saved observations; select an explicit raw ordinal",
        ));
    }
    Ok(selected)
}

/// Preserve a well-shaped stored object verbatim, including a recorded invalid
/// range. Other malformed historical values are explicitly unavailable, not a
/// reconstructed default viewport.
type ViewportSize = std::result::Result<Option<(u32, u32)>, &'static str>;
fn viewport(value: Option<&Value>) -> (Option<&Value>, ViewportSize) {
    let Some(value) = value else {
        return (None, Ok(None));
    };
    let Some(object) = value.as_object() else {
        return (None, Err("stored_viewport_invalid"));
    };
    let dimensions = object
        .get("width")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .zip(
            object
                .get("height")
                .and_then(Value::as_u64)
                .and_then(|v| u32::try_from(v).ok()),
        );
    let Some((width, height)) = dimensions.filter(|_| object.len() == 2) else {
        return (None, Err("stored_viewport_invalid"));
    };
    if !(100..=MAX_DIMENSION).contains(&width) || !(100..=MAX_DIMENSION).contains(&height) {
        return (Some(value), Err("stored_viewport_invalid"));
    }
    (Some(value), Ok(Some((width, height))))
}

#[derive(Debug, PartialEq, Eq)]
struct Dimensions {
    width: u32,
    height: u32,
}
fn png_dimensions(bytes: &[u8]) -> std::result::Result<Dimensions, &'static str> {
    if bytes.len() > PNG_BYTES {
        return Err("png_byte_limit");
    }
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("png_signature_invalid");
    }
    let mut offset = 8usize;
    let mut count = 0usize;
    let mut dimensions = None;
    let mut color = 0u8;
    let mut depth = 0u8;
    let mut palette = false;
    let mut idat = false;
    let mut idat_ended = false;
    let mut idat_bytes = 0usize;
    while offset < bytes.len() {
        count += 1;
        if count > CHUNKS {
            return Err("png_chunk_limit");
        }
        if bytes.len() - offset < 12 {
            return Err("png_chunk_bounds_invalid");
        }
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let end = offset
            .checked_add(12)
            .and_then(|v| v.checked_add(length))
            .filter(|end| *end <= bytes.len())
            .ok_or("png_chunk_bounds_invalid")?;
        let kind = &bytes[offset + 4..offset + 8];
        if !kind.iter().all(u8::is_ascii_alphabetic) || !kind[2].is_ascii_uppercase() {
            return Err("png_chunk_type_invalid");
        }
        let data = &bytes[offset + 8..end - 4];
        let expected_crc = u32::from_be_bytes(bytes[end - 4..end].try_into().unwrap());
        if crc32fast::hash(&bytes[offset + 4..end - 4]) != expected_crc {
            return Err("png_crc_mismatch");
        }
        if count == 1 && kind != b"IHDR" {
            return Err("png_chunk_order_invalid");
        }
        if idat && kind != b"IDAT" {
            idat_ended = true;
        }
        match kind {
            b"IHDR" => {
                if count != 1 || data.len() != 13 {
                    return Err("png_ihdr_invalid");
                }
                let width = u32::from_be_bytes(data[0..4].try_into().unwrap());
                let height = u32::from_be_bytes(data[4..8].try_into().unwrap());
                if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
                    return Err("png_dimensions_invalid");
                }
                depth = data[8];
                color = data[9];
                let valid_depth = match color {
                    0 => matches!(depth, 1 | 2 | 4 | 8 | 16),
                    2 | 4 | 6 => matches!(depth, 8 | 16),
                    3 => matches!(depth, 1 | 2 | 4 | 8),
                    _ => false,
                };
                if !valid_depth || data[10] != 0 || data[11] != 0 || data[12] > 1 {
                    return Err("png_ihdr_invalid");
                }
                dimensions = Some(Dimensions { width, height });
            }
            b"PLTE" => {
                if palette
                    || idat
                    || matches!(color, 0 | 4)
                    || data.is_empty()
                    || !data.len().is_multiple_of(3)
                    || data.len() > 768
                    || color == 3 && data.len() / 3 > 1usize << depth
                {
                    return Err("png_palette_invalid");
                }
                palette = true;
            }
            b"IDAT" => {
                if idat_ended || color == 3 && !palette {
                    return Err("png_chunk_order_invalid");
                }
                idat = true;
                idat_bytes += data.len();
            }
            b"IEND" => {
                if !data.is_empty() || end != bytes.len() {
                    return Err("png_iend_invalid");
                }
                if !idat || idat_bytes == 0 {
                    return Err("png_idat_missing");
                }
                return dimensions.ok_or("png_ihdr_invalid");
            }
            _ if kind[0].is_ascii_uppercase() => return Err("png_critical_chunk_unsupported"),
            _ => (),
        }
        offset = end;
    }
    Err("png_iend_invalid")
}
fn decode(text: &str) -> Result<std::result::Result<Vec<u8>, &'static str>> {
    let encoded_limit = PNG_BYTES.div_ceil(3) * 4;
    if text.len() > encoded_limit {
        return Ok(Err("png_byte_limit"));
    }
    let capacity = base64::decoded_len_estimate(text.len());
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(capacity).map_err(|_| {
        fail(
            StatusCode::SERVICE_UNAVAILABLE,
            "screenshot_capacity_unavailable",
            "Screenshot preparation capacity is unavailable",
        )
    })?;
    bytes.resize(capacity, 0);
    let length = match STANDARD.decode_slice(text.as_bytes(), &mut bytes) {
        Ok(length) => length,
        Err(_) => return Ok(Err("invalid_base64")),
    };
    if length > PNG_BYTES {
        return Ok(Err("png_byte_limit"));
    }
    bytes.truncate(length);
    Ok(Ok(bytes))
}
fn unavailable<'a>(
    mut observation: Observation<'a>,
    status: &'static str,
    reason: &'static str,
) -> Observation<'a> {
    // A malformed recorded range is inspectable only as a corrupt observation.
    // Keep the renderer/missing/disabled failure precedence without presenting
    // an invalid viewport as usable context for those other observations.
    if status != "corrupt" && viewport(observation.metadata.viewport).1.is_err() {
        observation.metadata.viewport = None;
    }
    observation.metadata.status = status;
    observation.metadata.reason = Some(reason);
    observation
}
fn inspect<'a>(job: &'a Job, url: &str, ordinal: Option<usize>) -> Result<Observation<'a>> {
    let report = content::stable_report(job).map_err(|_| {
        fail(
            StatusCode::CONFLICT,
            "screenshot_snapshot_unavailable",
            "Wait for a stable saved report before reading a screenshot",
        )
    })?;
    let (ordinal, page) = select(report, url, ordinal)?;
    let generation = (job.generation > 0).then_some(job.generation);
    // Stream the complete typed report without a whole-report Vec or clone.
    // This binds cached App facts under one read lock; it is not SQL attestation.
    let context = crate::content_cohort::snapshot_context(
        report,
        generation,
        "cached_job_read_lock_not_sql_attested",
    )
    .map_err(|_| {
        fail(
            StatusCode::INTERNAL_SERVER_ERROR,
            "screenshot_snapshot_failed",
            "Saved report identity could not be encoded",
        )
    })?;
    let generation_text = generation.map(|v| v.to_string());
    let snapshot_sha256 = digest(
        &serde_json::to_vec(&(
            SNAPSHOT_DOMAIN,
            job.id.to_string(),
            &generation_text,
            &context.sha256,
        ))
        .map_err(|_| invalid())?,
    );
    let (stored_viewport, expected_dimensions) = viewport(page.rendered_viewport.as_ref());
    let mut observation = Observation {
        metadata: Metadata {
            engine: ENGINE,
            version: 1,
            id: job.id,
            ordinal,
            url: &page.url,
            final_url: &page.final_url,
            generation,
            snapshot_sha256,
            available: false,
            status: "missing",
            reason: None,
            viewport: stored_viewport,
            png_sha256: None,
            bytes: None,
            width: None,
            height: None,
        },
        png: None,
    };
    if page.renderer_error.is_some() {
        return Ok(unavailable(observation, "render_failed", "renderer_failed"));
    }
    let Some(encoded) = page.screenshot_base64.as_deref() else {
        return Ok(
            if !report
                .config
                .renderer
                .as_ref()
                .is_some_and(|renderer| renderer.screenshot)
            {
                unavailable(observation, "disabled", "screenshot_not_enabled")
            } else if !page.rendered {
                unavailable(observation, "missing", "rendered_document_unavailable")
            } else {
                unavailable(observation, "missing", "screenshot_not_recorded")
            },
        );
    };
    let expected_dimensions = match expected_dimensions {
        Ok(value) => value,
        Err(reason) => return Ok(unavailable(observation, "corrupt", reason)),
    };
    let bytes = match decode(encoded)? {
        Ok(bytes) => bytes,
        Err(reason) => return Ok(unavailable(observation, "corrupt", reason)),
    };
    let actual = match png_dimensions(&bytes) {
        Ok(actual) => actual,
        Err(reason) => return Ok(unavailable(observation, "corrupt", reason)),
    };
    if expected_dimensions.is_some_and(|expected| expected != (actual.width, actual.height)) {
        return Ok(unavailable(observation, "corrupt", "viewport_mismatch"));
    }
    observation.metadata.available = true;
    observation.metadata.status = "available";
    observation.metadata.png_sha256 = Some(digest(&bytes));
    observation.metadata.bytes = Some(bytes.len());
    observation.metadata.width = Some(actual.width);
    observation.metadata.height = Some(actual.height);
    observation.png = Some(bytes);
    Ok(observation)
}

struct BoundedJson(Vec<u8>);
impl Write for BoundedJson {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > METADATA_BYTES - self.0.len() {
            return Err(io::Error::other("Screenshot metadata exceeds64KiB"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn response(bytes: Vec<u8>, mime: &'static str) -> Response {
    let sha = digest(&bytes);
    let length = bytes.len().to_string();
    (
        [
            (header::CONTENT_TYPE, mime.to_string()),
            (header::CONTENT_LENGTH, length),
            (header::CACHE_CONTROL, "no-store".into()),
            (header::HeaderName::from_static("x-content-sha256"), sha),
        ],
        bytes,
    )
        .into_response()
}
fn metadata_response(observation: &Observation<'_>) -> Result<Response> {
    let mut bytes = BoundedJson(Vec::new());
    serde_json::to_writer(&mut bytes, &observation.metadata).map_err(|_| {
        fail(
            StatusCode::PAYLOAD_TOO_LARGE,
            "screenshot_metadata_limit",
            "Screenshot metadata exceeds64KiB",
        )
    })?;
    Ok(response(bytes.0, "application/json; charset=utf-8"))
}
fn png_response(observation: Observation<'_>, query: &PngQuery) -> Result<Response> {
    let metadata = &observation.metadata;
    if query.expected_snapshot_sha256 != metadata.snapshot_sha256
        || query.expected_generation != metadata.generation
    {
        return Err(fail(
            StatusCode::CONFLICT,
            "screenshot_snapshot_changed",
            "Saved screenshot job, report or generation changed",
        ));
    }
    if !metadata.available {
        return Err(fail(
            if metadata.status == "corrupt" {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::NOT_FOUND
            },
            "screenshot_unavailable",
            "This saved page has no validated available PNG artifact",
        ));
    }
    let image_sha = metadata
        .png_sha256
        .as_ref()
        .expect("available image digest");
    if &query.expected_image_sha256 != image_sha {
        return Err(fail(
            StatusCode::CONFLICT,
            "screenshot_image_changed",
            "Saved PNG bytes changed",
        ));
    }
    let ordinal = metadata.ordinal.to_string();
    let generation = metadata.generation.map(|v| v.to_string());
    let id = metadata.id.to_string();
    let identity = digest(
        &serde_json::to_vec(&(
            ENGINE,
            &id,
            &ordinal,
            metadata.url,
            &generation,
            &metadata.snapshot_sha256,
            image_sha,
        ))
        .map_err(|_| invalid())?,
    );
    let mut headers = vec![
        (
            "x-screenshot-snapshot-sha256",
            metadata.snapshot_sha256.clone(),
        ),
        ("x-screenshot-image-sha256", image_sha.clone()),
        (
            "x-screenshot-generation",
            generation.unwrap_or_else(|| "unknown".into()),
        ),
        ("x-screenshot-ordinal", ordinal),
        ("x-screenshot-job-id", id),
        ("x-screenshot-url-sha256", digest(metadata.url.as_bytes())),
        ("x-screenshot-identity-sha256", identity),
    ];
    let mut out = response(
        observation.png.expect("available original bytes"),
        "image/png",
    );
    for (name, value) in headers.drain(..) {
        out.headers_mut().insert(
            header::HeaderName::from_static(name),
            value.parse().expect("ASCII hashes/integers/UUID only"),
        );
    }
    out.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_static("attachment; filename=screenshot.png"),
    );
    Ok(out)
}
async fn read(
    state: AppState,
    id: Uuid,
    url: String,
    ordinal: Option<usize>,
    download: Option<PngQuery>,
) -> Result<Response> {
    validate_url(&url)?;
    if download.as_ref().is_some_and(|query| {
        !hash_text(&query.expected_snapshot_sha256) || !hash_text(&query.expected_image_sha256)
    }) {
        return Err(invalid());
    }
    // Shared bound with pure profile preparation: two combined image/profile
    // preparation workers. This is not a global transport/RSS/CPU ceiling.
    let permit = state
        .lifecycle
        .config_profile_gate
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            fail(
                StatusCode::SERVICE_UNAVAILABLE,
                "screenshot_preparation_busy",
                "Image/profile preparation is busy",
            )
        })?;
    let job = get_job(&state, id).await.map_err(|_| {
        fail(
            StatusCode::NOT_FOUND,
            "screenshot_job_not_found",
            "Saved crawl was not found",
        )
    })?;
    let guard = job.read_owned().await;
    state
        .lifecycle
        .blocking
        .run_blocking(move || {
            let _permit = permit;
            let observation = inspect(&guard, &url, ordinal)?;
            if let Some(query) = download {
                png_response(observation, &query)
            } else {
                metadata_response(&observation)
            }
        })
        .await
        .map_err(|_| {
            fail(
                StatusCode::SERVICE_UNAVAILABLE,
                "screenshot_worker_unavailable",
                "Owned screenshot preparation could not complete",
            )
        })?
}
pub(super) async fn metadata(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    query: std::result::Result<Query<MetadataQuery>, QueryRejection>,
) -> Result<Response> {
    let Query(query) = query.map_err(|_| invalid())?;
    read(state, id, query.url, query.ordinal, None).await
}
pub(super) async fn png(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    query: std::result::Result<Query<PngQuery>, QueryRejection>,
) -> Result<Response> {
    let Query(query) = query.map_err(|_| invalid())?;
    read(
        state,
        id,
        query.url.clone(),
        Some(query.ordinal),
        Some(query),
    )
    .await
}

#[cfg(test)]
#[path = "screenshots_tests.rs"]
mod tests;
