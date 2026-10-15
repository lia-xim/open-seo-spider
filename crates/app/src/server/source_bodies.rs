//! Same-snapshot descriptors and selected original bytes; no website requests.
use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use open_seo_spider_core::{SitemapBodyReadContext, SitemapBodyWindow};
use sha2::{Digest, Sha256};
use std::io::Write;
type BodyResult<T> = Result<T, (StatusCode, Json<Value>)>;
const JSON_CAP: usize = 1_048_576;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    version: u32,
    job: Uuid,
    context: SitemapBodyReadContext,
    document_id: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    snapshot: String,
    after: u64,
    document_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReadQuery {
    limit: Option<usize>,
    cursor: Option<String>,
    document_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DownloadQuery {
    snapshot: String,
}
fn error(status: StatusCode, code: &str, message: impl ToString) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(
            json!({"error":code,"message":message.to_string().chars().take(512).collect::<String>()}),
        ),
    )
}
fn classify(e: impl ToString) -> (StatusCode, Json<Value>) {
    let s = e.to_string();
    let (status, code) = if s.contains("Stale source body snapshot") {
        (StatusCode::CONFLICT, "stale_source_body_snapshot")
    } else if s.contains("descriptor not found") || s.contains("No own retained") {
        (StatusCode::NOT_FOUND, "source_body_unavailable")
    } else if s.contains("requires UTF-8") {
        (StatusCode::CONFLICT, "unsupported_archive_storage")
    } else if s.contains("capacity") || s.contains("admission") {
        (StatusCode::SERVICE_UNAVAILABLE, "source_body_capacity")
    } else {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "source_body_verification_failed",
        )
    };
    error(status, code, s)
}
fn encode<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    Ok(URL_SAFE_NO_PAD.encode(serde_json::to_vec(value)?))
}
fn decode<T: serde::de::DeserializeOwned>(text: &str) -> BodyResult<T> {
    if text.len() > 2048 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "invalid_source_body_cursor",
            "Cursor exceeds2048bytes",
        ));
    }
    let raw = URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|e| error(StatusCode::BAD_REQUEST, "invalid_source_body_cursor", e))?;
    serde_json::from_slice(&raw)
        .map_err(|e| error(StatusCode::BAD_REQUEST, "invalid_source_body_cursor", e))
}
fn snapshot(text: &str, id: Uuid) -> BodyResult<Snapshot> {
    let s: Snapshot = decode(text)?;
    if s.version != 1 || s.job != id {
        return Err(error(
            StatusCode::CONFLICT,
            "stale_source_body_snapshot",
            "Snapshot belongs to a different job/profile",
        ));
    }
    Ok(s)
}
struct BoundedJson(Vec<u8>);
impl Write for BoundedJson {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if data.len() > JSON_CAP - self.0.len() {
            return Err(std::io::Error::other(
                "Source body metadata response exceeds1MiB",
            ));
        }
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn json_response(value: &impl Serialize) -> BodyResult<Response> {
    let mut out = BoundedJson(Vec::new());
    serde_json::to_writer(&mut out, value).map_err(|e| {
        error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "source_body_response_budget",
            e,
        )
    })?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        out.0,
    )
        .into_response())
}
#[derive(Serialize)]
struct Inventory {
    job: Uuid,
    generation: u64,
    config_sha256: String,
    window: Option<SitemapBodyWindow>,
    snapshot: Option<String>,
    cursor: Option<String>,
    byte_availability: &'static str,
    download_available: Option<bool>,
}
pub(super) async fn inventory(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    query: Result<Query<ReadQuery>, axum::extract::rejection::QueryRejection>,
) -> BodyResult<Response> {
    let Query(query) =
        query.map_err(|e| error(StatusCode::BAD_REQUEST, "invalid_source_body_query", e))?;
    let limit = query.limit.unwrap_or(50);
    if !(1..=200).contains(&limit) {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "invalid_source_body_query",
            "limit must be1..200",
        ));
    }
    let parsed: Option<Cursor> = query.cursor.as_deref().map(decode).transpose()?;
    let expected = parsed
        .as_ref()
        .map(|c| snapshot(&c.snapshot, id))
        .transpose()?;
    if let Some(id) = query.document_id.as_ref()
        && (id.len() != 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "invalid_source_document_id",
            "Expected64lowercase hex digits",
        ));
    }
    if parsed
        .as_ref()
        .is_some_and(|c| c.document_id != query.document_id)
        || expected
            .as_ref()
            .is_some_and(|s| s.document_id != query.document_id)
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "invalid_source_body_cursor",
            "Cursor source filter changed",
        ));
    }
    let job = get_job(&state, id)
        .await
        .map_err(|(s, Json(e))| error(s, "job_not_found", e.error))?;
    let guard = job.read().await;
    let generation = guard.generation;
    let config_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&guard.config).map_err(classify)?)
    );
    let path = state.data_dir.join(format!("{id}.sqlite"));
    let after = parsed.as_ref().map_or(0, |c| c.after);
    let expected_context = expected.as_ref().map(|s| s.context.clone());
    let document_id = query.document_id.clone();
    let window = state
        .lifecycle
        .blocking
        .run_blocking(move || -> anyhow::Result<Option<SitemapBodyWindow>> {
            if !path.try_exists()? {
                return Ok(None);
            }
            Ok(Some(
                CrawlStore::open_read_only(path)?.sitemap_source_body_window_for_source(
                    after,
                    limit,
                    expected_context.as_ref(),
                    document_id.as_deref(),
                )?,
            ))
        })
        .await
        .map_err(classify)?
        .map_err(classify)?;
    if let Some(ctx) = window.as_ref().and_then(|w| w.context.as_ref())
        && (ctx.generation != generation || ctx.config_sha256 != config_sha256)
    {
        return Err(error(
            StatusCode::CONFLICT,
            "stale_source_body_snapshot",
            "App and saved source context differ",
        ));
    }
    if window.is_none() && expected.is_some() {
        return Err(error(
            StatusCode::CONFLICT,
            "stale_source_body_snapshot",
            "Saved source archive disappeared",
        ));
    }
    let snapshot = window
        .as_ref()
        .and_then(|w| w.context.clone())
        .map(|context| {
            encode(&Snapshot {
                version: 1,
                job: id,
                context,
                document_id: query.document_id.clone(),
            })
        })
        .transpose()
        .map_err(classify)?;
    let cursor = window
        .as_ref()
        .and_then(|w| w.next_sequence)
        .zip(snapshot.as_ref())
        .map(|(after, s)| {
            encode(&Cursor {
                snapshot: s.clone(),
                after,
                document_id: query.document_id.clone(),
            })
        })
        .transpose()
        .map_err(classify)?;
    json_response(&Inventory {
        job: id,
        generation,
        config_sha256,
        window,
        snapshot,
        cursor,
        byte_availability: "not_checked",
        download_available: None,
    })
}
pub(super) async fn download(
    State(state): State<AppState>,
    Path((id, body_id)): Path<(Uuid, String)>,
    headers: axum::http::HeaderMap,
    query: Result<Query<DownloadQuery>, axum::extract::rejection::QueryRejection>,
) -> BodyResult<Response> {
    let Query(query) =
        query.map_err(|e| error(StatusCode::BAD_REQUEST, "invalid_source_body_query", e))?;
    if body_id.len() != 64
        || !body_id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "invalid_source_body_id",
            "Expected64lowercase hex digits",
        ));
    }
    if headers.contains_key(header::RANGE) {
        return Err(error(
            StatusCode::RANGE_NOT_SATISFIABLE,
            "source_body_range_unsupported",
            "Only a complete selected-body download is supported",
        ));
    }
    let expected = snapshot(&query.snapshot, id)?;
    let job = get_job(&state, id)
        .await
        .map_err(|(s, Json(e))| error(s, "job_not_found", e.error))?;
    let guard = job.read().await;
    let config = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&guard.config).map_err(classify)?)
    );
    if expected.context.generation != guard.generation || expected.context.config_sha256 != config {
        return Err(error(
            StatusCode::CONFLICT,
            "stale_source_body_snapshot",
            "App generation/config changed",
        ));
    }
    let path = state.data_dir.join(format!("{id}.sqlite"));
    let data = state.data_dir.as_ref().clone();
    let final_path = path.clone();
    let final_context = expected.context.clone();
    let final_document = expected.document_id.clone();
    let (manifest, body) = state
        .lifecycle
        .source_bodies
        .prepare_checked(
            data,
            move |sink, checkpoint| {
                CrawlStore::open_read_only(path)?.copy_sitemap_source_body_for_source(
                    &body_id,
                    &expected.context,
                    expected.document_id.as_deref(),
                    sink,
                    checkpoint,
                )
            },
            move || {
                CrawlStore::open_read_only(final_path)?.sitemap_source_body_window_for_source(
                    0,
                    1,
                    Some(&final_context),
                    final_document.as_deref(),
                )?;
                Ok(())
            },
        )
        .await
        .map_err(classify)?;
    drop(guard);
    let representation = match manifest.body.representation {
        open_seo_spider_core::SitemapBodyRepresentation::PostHttpEntity => "post-http-entity",
        open_seo_spider_core::SitemapBodyRepresentation::DecodedXml => "decoded-xml",
    };
    let extent = match manifest.body.extent {
        open_seo_spider_core::SitemapBodyExtent::Complete => "complete",
        open_seo_spider_core::SitemapBodyExtent::ObservedPrefix => "observed-prefix",
    };
    Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, manifest.body.bytes.to_string())
        .header(header::CACHE_CONTROL, "private, no-store")
        .header(header::ETAG, format!("\"sha256:{}\"", manifest.body.sha256))
        .header("x-content-type-options", "nosniff")
        .header(
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"sitemap-{}-{representation}-{extent}.bin\"",
                manifest.body.body_id
            ),
        )
        .body(body)
        .map_err(classify)
}
pub(super) async fn unsupported_head() -> BodyResult<Response> {
    Err(error(
        StatusCode::METHOD_NOT_ALLOWED,
        "source_body_head_unsupported",
        "Use descriptor metadata or a full verified GET",
    ))
}

#[cfg(test)]
#[path = "source_bodies_tests.rs"]
mod tests;
