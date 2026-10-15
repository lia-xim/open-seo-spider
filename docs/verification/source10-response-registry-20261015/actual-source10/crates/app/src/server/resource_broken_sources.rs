//! Saved SQLite only. Request DTOs never accept paths, URLs or observations.
use super::*;
use crate::resource_broken_sources_app::{self as workflow, Request};
use axum::extract::rejection::JsonRejection;
use open_seo_spider_core::{ResourceBrokenSourcesPage, ResourceBrokenSourcesSnapshot};
use std::io::Write;

fn failure(error: impl ToString) -> (StatusCode, Json<ApiError>) {
    let message = error.to_string();
    let status = if message.contains("Stale resource")
        || message.contains("another crawl or generation")
        || message.contains("generation changed")
        || message.contains("configuration differs")
        || message.contains("snapshot differs")
        || message.contains("no longer available")
    {
        StatusCode::CONFLICT
    } else if message.contains("budget") || message.contains("exceeds") || message.contains("limit")
    {
        StatusCode::PAYLOAD_TOO_LARGE
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    };
    api_error(status, message)
}
fn check_job(job: &Job, metadata: &Value, request: &Request) -> ApiResult<()> {
    if matches!(
        job.status.as_str(),
        "queued" | "running" | "pausing" | "cancelling"
    ) {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Wait for a stable saved crawl before resource-source analysis",
        ));
    }
    if metadata["crawl_generation"].as_u64() != Some(job.generation)
        || request
            .expected_generation
            .is_some_and(|g| g != job.generation)
    {
        return Err(api_error(StatusCode::CONFLICT, "Crawl generation changed"));
    }
    let expected = workflow::bytes_sha(&serde_json::to_vec(&job.config).map_err(failure)?);
    if metadata["crawl_config_sha256"].as_str() != Some(expected.as_str()) {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Crawl configuration changed",
        ));
    }
    Ok(())
}
fn binding(metadata: &Value, snapshot: &ResourceBrokenSourcesSnapshot) -> anyhow::Result<()> {
    resources::validate_binding(metadata, &snapshot.ledger)
        .map_err(|(_, Json(e))| anyhow::anyhow!(e.error))?;
    resources::validate_snapshot(metadata, Some(&snapshot.saved))
        .map_err(|(_, Json(e))| anyhow::anyhow!(e.error))?;
    Ok(())
}
fn unavailable(metadata: Value) -> ResourceBrokenSourcesPage {
    ResourceBrokenSourcesPage {
        scope: workflow::SCOPE,
        available: false,
        unavailable_reason: Some(if metadata["enabled"].as_bool() == Some(true) {
            "initializing_or_imported_history_without_sqlite_rows"
        } else {
            "native_resource_discovery_disabled"
        }),
        snapshot: None,
        rows: vec![],
        selected_evaluation_rows: 0,
        selected_distinct_occurrence_ids: 0,
        counts: Default::default(),
        next_sequence: None,
        has_more: false,
        website_requests: 0,
    }
}
pub(super) async fn preview(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<Request>, JsonRejection>,
) -> ApiResult<Response> {
    let Json(request) = request.map_err(|e| api_error(StatusCode::BAD_REQUEST, e.body_text()))?;
    let (metadata, path) = resources::context(&state, id).await?;
    let job = get_job(&state, id).await?;
    let guard = job.read().await;
    check_job(&guard, &metadata, &request)?;
    let generation = guard.generation;
    let core = request.core(id, generation).map_err(failure)?;
    let bytes = if let Some(path) = path {
        state
            .lifecycle
            .blocking
            .run_blocking(move || -> anyhow::Result<Vec<u8>> {
                let page = CrawlStore::open_read_only(path)?.resource_broken_sources(&core)?;
                anyhow::ensure!(
                    page.snapshot.is_some() || core.expected_snapshot_token.is_none(),
                    "Resource history no longer available"
                );
                if let Some(snapshot) = &page.snapshot {
                    binding(&metadata, snapshot)?;
                }
                workflow::page_bytes(id, generation, page)
            })
            .await
            .map_err(|e| api_error(StatusCode::SERVICE_UNAVAILABLE, e))?
            .map_err(failure)?
    } else {
        if core.expected_snapshot_token.is_some() {
            return Err(api_error(
                StatusCode::CONFLICT,
                "Resource history no longer available",
            ));
        }
        workflow::page_bytes(id, generation, unavailable(metadata)).map_err(failure)?
    };
    drop(guard);
    let length = bytes.len().to_string();
    let digest = workflow::bytes_sha(&bytes);
    Ok((
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
            (header::CONTENT_LENGTH, length.as_str()),
        ],
        [("x-content-sha256", digest.as_str())],
        bytes,
    )
        .into_response())
}
pub(super) async fn csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<Request>, JsonRejection>,
) -> ApiResult<Response> {
    let Json(request) = request.map_err(|e| api_error(StatusCode::BAD_REQUEST, e.body_text()))?;
    if request.expected_snapshot_token.is_none() || request.after_sequence.is_some() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Full filtered CSV requires the preview snapshot token and no continuation",
        ));
    }
    let (metadata, path) = resources::context(&state, id).await?;
    let job = get_job(&state, id).await?;
    let guard = job.read().await;
    check_job(&guard, &metadata, &request)?;
    let generation = guard.generation;
    let core = request.core(id, generation).map_err(failure)?;
    let path = path.ok_or_else(|| {
        api_error(
            StatusCode::CONFLICT,
            "Native resource history is unavailable",
        )
    })?;
    let pending = state
        .lifecycle
        .response_streams
        .start(move |channel, ready| {
            // Exact full CSV, cap and digest are verified before success headers.
            let csv =
                CrawlStore::open_read_only(path)?.export_resource_broken_sources_csv(&core)?;
            binding(&metadata, &csv.snapshot)?;
            anyhow::ensure!(
                csv.bytes.len() <= workflow::CSV_BYTES,
                "Resource CSV exceeds64MiB"
            );
            anyhow::ensure!(
                workflow::bytes_sha(&csv.bytes) == csv.sha256,
                "Resource CSV digest changed"
            );
            ready.send((csv.snapshot.clone(), csv.bytes.len(), csv.sha256))?;
            channel.write_all(&csv.bytes)?;
            channel.flush()?;
            Ok(())
        })
        .await
        .map_err(|e| api_error(StatusCode::SERVICE_UNAVAILABLE, e))?;
    let ((snapshot, length, digest), body) = pending.ready().await.map_err(failure)?;
    drop(guard);
    Response::builder()
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"resource-broken-sources.csv\"",
        )
        .header(header::CONTENT_LENGTH, length.to_string())
        .header("x-content-sha256", digest)
        .header(
            "x-resource-source-snapshot",
            workflow::api_token(id, generation, &snapshot.snapshot_token),
        )
        .header(
            "x-resource-transport-set-sha256",
            snapshot.transport_set_sha256,
        )
        .body(body)
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))
}

#[cfg(test)]
#[path = "resource_broken_sources_tests.rs"]
mod tests;
