//! API-only HTTP facade. All operations delegate to the same application service.
use super::*;
use crate::enrichment_app::{self as app, ReadRequest, StartRequest};
use axum::extract::rejection::JsonRejection;
use std::io::Write;
fn failure(error: impl ToString) -> (StatusCode, Json<ApiError>) {
    let s = error.to_string();
    let code = if s.contains("Stale") || s.contains("stable") || s.contains("snapshot") {
        StatusCode::CONFLICT
    } else if s.contains("budget") || s.contains("exceed") {
        StatusCode::PAYLOAD_TOO_LARGE
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    };
    api_error(code, s)
}
pub(super) async fn preview(
    State(state): State<AppState>,
    request: Result<Json<StartRequest>, JsonRejection>,
) -> ApiResult<Json<Value>> {
    let Json(request) = request.map_err(|e| api_error(StatusCode::BAD_REQUEST, e.body_text()))?;
    let plan = state
        .lifecycle
        .blocking
        .run_blocking(move || app::prepare(&request))
        .await
        .map_err(|e| api_error(StatusCode::SERVICE_UNAVAILABLE, e))?
        .map_err(failure)?;
    Ok(Json(app::preview(&plan)))
}
pub(super) async fn start(
    State(state): State<AppState>,
    request: Result<Json<StartRequest>, JsonRejection>,
) -> ApiResult<Json<Value>> {
    let Json(request) = request.map_err(|e| api_error(StatusCode::BAD_REQUEST, e.body_text()))?;
    if !request.acknowledge_remote_fetch {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Explicit acknowledgement that the provider may fetch the submitted site is required",
        ));
    }
    let input = request.input_utf8.as_bytes().to_vec();
    let plan = state
        .lifecycle
        .blocking
        .run_blocking(move || app::prepare(&request))
        .await
        .map_err(|e| api_error(StatusCode::SERVICE_UNAVAILABLE, e))?
        .map_err(failure)?;
    Ok(Json(
        state
            .lifecycle
            .enrichments()
            .map_err(failure)?
            .start(plan, input)
            .await
            .map_err(failure)?,
    ))
}
pub(super) async fn status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let service = state.lifecycle.enrichments().map_err(failure)?.clone();
    let v = state
        .lifecycle
        .blocking
        .run_blocking(move || service.status(id))
        .await
        .map_err(|e| api_error(StatusCode::SERVICE_UNAVAILABLE, e))?
        .map_err(failure)?;
    Ok(Json(v))
}
pub(super) async fn cancel(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    Ok(Json(
        state
            .lifecycle
            .enrichments()
            .map_err(failure)?
            .cancel(id)
            .map_err(failure)?,
    ))
}
pub(super) async fn configuration(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let service = state.lifecycle.enrichments().map_err(failure)?.clone();
    let bytes = state
        .lifecycle
        .blocking
        .run_blocking(move || service.configuration(id))
        .await
        .map_err(|e| api_error(StatusCode::SERVICE_UNAVAILABLE, e))?
        .map_err(failure)?;
    json_bytes(bytes)
}
fn json_bytes(bytes: Vec<u8>) -> ApiResult<Response> {
    let hash = open_seo_spider_core::api_enrichment::api_sha(&bytes);
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::CONTENT_LENGTH, bytes.len().to_string())
        .header("x-content-sha256", hash)
        .body(axum::body::Body::from(bytes))
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))
}
pub(super) async fn page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<ReadRequest>, JsonRejection>,
) -> ApiResult<Response> {
    let Json(request) = request.map_err(|e| api_error(StatusCode::BAD_REQUEST, e.body_text()))?;
    let service = state.lifecycle.enrichments().map_err(failure)?.clone();
    let bytes = state
        .lifecycle
        .blocking
        .run_blocking(move || service.page(id, &request))
        .await
        .map_err(|e| api_error(StatusCode::SERVICE_UNAVAILABLE, e))?
        .map_err(failure)?;
    json_bytes(bytes)
}
pub(super) async fn csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<ReadRequest>, JsonRejection>,
) -> ApiResult<Response> {
    let Json(request) = request.map_err(|e| api_error(StatusCode::BAD_REQUEST, e.body_text()))?;
    let service = state.lifecycle.enrichments().map_err(failure)?.clone();
    let pending = state
        .lifecycle
        .response_streams
        .start(move |channel, ready| {
            let bytes = service.csv(id, &request)?;
            let len = bytes.len();
            let hash = open_seo_spider_core::api_enrichment::api_sha(&bytes);
            ready.send((len, hash))?;
            channel.write_all(&bytes)?;
            channel.flush()?;
            Ok(())
        })
        .await
        .map_err(|e| api_error(StatusCode::SERVICE_UNAVAILABLE, e))?;
    let ((len, hash), body) = pending
        .ready_with_length(|(len, _)| *len)
        .await
        .map_err(failure)?;
    Response::builder()
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"api-enrichment.csv\"",
        )
        .header(header::CONTENT_LENGTH, len.to_string())
        .header("x-content-sha256", hash)
        .body(body)
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))
}
#[cfg(test)]
#[path = "enrichment_tests.rs"]
mod tests;
