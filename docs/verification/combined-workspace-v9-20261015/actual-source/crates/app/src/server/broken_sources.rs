//! Read-only saved-snapshot routes; no live target checks or caller paths.
use super::*;
use crate::broken_sources::{self as projection, Evaluation, Request, Snapshot};
use crate::migration_app::{Failure, digest, fail};
use axum::extract::rejection::JsonRejection;
use open_seo_spider_core::ComparisonSide;
use sha2::{Digest, Sha256};
type ApiResult<T> = Result<T, (StatusCode, Json<Failure>)>;
fn error(status: StatusCode, code: &str, message: impl ToString) -> (StatusCode, Json<Failure>) {
    (status, Json(fail(code, message)))
}
fn mapped(f: Failure) -> (StatusCode, Json<Failure>) {
    let status = match f.code.as_str() {
        "stale_snapshot" | "stale_cursor" => StatusCode::CONFLICT,
        "serialized_byte_budget" | "csv_budget" => StatusCode::PAYLOAD_TOO_LARGE,
        _ => StatusCode::BAD_REQUEST,
    };
    (status, Json(f))
}
async fn evaluate_saved(
    state: &AppState,
    id: Uuid,
    request: Request,
) -> ApiResult<(Evaluation, Request)> {
    request.validate().map_err(mapped)?;
    let job = get_job(state, id)
        .await
        .map_err(|(s, Json(e))| error(s, "broken_sources_job_not_found", e.error))?;
    let (saved, snapshot) = {
        let guard = job.read().await;
        if let Some(report) = &guard.report {
            projection::preflight(report).map_err(mapped)?;
        }
        let saved = comparison::stable_saved_report(
            &guard,
            ComparisonSide::Before,
            request.expected_generation,
        )
        .map_err(|(s, Json(e))| error(s, &e.error.code, e.error.message))?;
        let snapshot = Snapshot::new(
            &saved.report,
            id,
            guard.generation,
            guard.status.clone(),
            digest(&guard.config, projection::REPORT_BYTES)
                .map_err(mapped)?
                .0,
            saved.binding.clone(),
        )
        .map_err(mapped)?;
        (saved, snapshot)
    };
    let filter = request.filter;
    let result = state
        .lifecycle
        .blocking
        .run_blocking(move || projection::evaluate(saved.report, snapshot, filter))
        .await
        .map_err(|e| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "broken_sources_worker_failed",
                e,
            )
        })?
        .map_err(mapped)?;
    Ok((result, request))
}
pub(super) async fn preview(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<Request>, JsonRejection>,
) -> ApiResult<Response> {
    let Json(request) = request.map_err(|e| {
        error(
            StatusCode::BAD_REQUEST,
            "invalid_broken_sources_request",
            e.body_text(),
        )
    })?;
    let (evaluation, request) = evaluate_saved(&state, id, request).await?;
    let bytes = state
        .lifecycle
        .blocking
        .run_blocking(move || projection::json_bytes(&evaluation.page(&request)?))
        .await
        .map_err(|e| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "broken_sources_output_worker_failed",
                e,
            )
        })?
        .map_err(mapped)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response())
}
pub(super) async fn csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<Request>, JsonRejection>,
) -> ApiResult<Response> {
    let Json(request) = request.map_err(|e| {
        error(
            StatusCode::BAD_REQUEST,
            "invalid_broken_sources_request",
            e.body_text(),
        )
    })?;
    if request.cursor.is_some() {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "full_selection_export",
            "CSV exports the entire snapshot-bound filter; omit the page cursor",
        ));
    }
    let (evaluation, request) = evaluate_saved(&state, id, request).await?;
    evaluation.check_token(&request, true).map_err(mapped)?;
    // CSV is complete, bounded and hashed before publishing any success headers.
    // It is a byte Body, not a new producer stream or a claim about transport grace.
    let bytes = state
        .lifecycle
        .blocking
        .run_blocking(move || evaluation.export_csv())
        .await
        .map_err(|e| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "broken_sources_export_worker_failed",
                e,
            )
        })?
        .map_err(mapped)?;
    let length = bytes.len().to_string();
    let sha = format!("{:x}", Sha256::digest(&bytes));
    let mut response = (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"broken-link-sources.csv\"",
            ),
        ],
        bytes,
    )
        .into_response();
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        length.parse().map_err(|e| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "broken_sources_header",
                e,
            )
        })?,
    );
    response.headers_mut().insert(
        "x-content-sha256",
        sha.parse().map_err(|e| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "broken_sources_header",
                e,
            )
        })?,
    );
    Ok(response)
}
#[cfg(test)]
#[path = "broken_sources_tests.rs"]
mod tests;
