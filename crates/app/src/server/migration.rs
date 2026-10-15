//! Stable App-declared saved report adapter; never opens caller paths or crawls.
use super::*;
use crate::migration_app::{self as workflow, Evaluation, Failure, Request, Snapshot};
use axum::extract::rejection::JsonRejection;
use open_seo_spider_core::ComparisonSide;
use sha2::{Digest, Sha256};
type MResult<T> = Result<T, (StatusCode, Json<Failure>)>;
fn error(status: StatusCode, code: &str, message: impl ToString) -> (StatusCode, Json<Failure>) {
    (status, Json(workflow::fail(code, message)))
}
fn mapped(f: Failure) -> (StatusCode, Json<Failure>) {
    let status = match f.code.as_str() {
        "stale_snapshot" | "stale_cursor" => StatusCode::CONFLICT,
        _ => StatusCode::BAD_REQUEST,
    };
    (status, Json(f))
}
async fn evaluate_saved(
    state: &AppState,
    id: Uuid,
    request: Request,
) -> MResult<(Evaluation, Request)> {
    request.validate().map_err(mapped)?;
    let job = get_job(state, id)
        .await
        .map_err(|(s, Json(e))| error(s, "migration_job_not_found", e.error))?;
    let (saved, snapshot) = {
        let guard = job.read().await;
        // Bound typed input before the existing comparison authority clones it.
        // The same read guard protects status/generation/report/config throughout.
        if let Some(report) = &guard.report {
            if report.pages.len() > request.options.observation_limits.max_pages {
                return Err(error(
                    StatusCode::BAD_REQUEST,
                    "page_count_budget",
                    "Saved page count exceeds migration observation limit",
                ));
            }
            workflow::digest(report, workflow::REPORT_BYTES).map_err(mapped)?;
        }
        let saved = comparison::stable_saved_report(
            &guard,
            ComparisonSide::Before,
            request.expected_generation,
        )
        .map_err(|(status, Json(e))| error(status, &e.error.code, e.error.message))?;
        let config_hash = workflow::digest(&guard.config, workflow::REPORT_BYTES)
            .map_err(mapped)?
            .0;
        let snapshot = Snapshot::from_report(
            &saved.report,
            Some(id),
            Some(guard.generation),
            guard.status.clone(),
            config_hash,
            saved.binding.clone(),
        )
        .map_err(mapped)?;
        (saved, snapshot)
    };
    let request_copy = request.clone();
    let evaluation = state
        .lifecycle
        .blocking
        .run_blocking(move || {
            workflow::evaluate(
                request_copy.csv.as_bytes(),
                &request_copy.options,
                saved.report,
                snapshot,
            )
        })
        .await
        .map_err(|e| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "migration_worker_failed",
                e,
            )
        })?
        .map_err(mapped)?;
    Ok((evaluation, request))
}
pub(super) async fn preview(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<Request>, JsonRejection>,
) -> MResult<Response> {
    let Json(request) = request.map_err(|e| {
        error(
            StatusCode::BAD_REQUEST,
            "invalid_migration_request",
            e.body_text(),
        )
    })?;
    let (evaluation, request) = evaluate_saved(&state, id, request).await?;
    let page = evaluation.page(&request).map_err(mapped)?;
    let bytes = workflow::json_bytes(&page).map_err(mapped)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response())
}
#[derive(Clone, Copy)]
enum Format {
    Json,
    Csv,
}
async fn download(
    state: AppState,
    id: Uuid,
    request: Result<Json<Request>, JsonRejection>,
    format: Format,
) -> MResult<Response> {
    let Json(request) = request.map_err(|e| {
        error(
            StatusCode::BAD_REQUEST,
            "invalid_migration_request",
            e.body_text(),
        )
    })?;
    if request.cursor.is_some() || request.outcome.is_some() {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "full_export_only",
            "Export contains every mapping row/outcome; omit page cursor and outcome filter",
        ));
    }
    let (evaluation, request) = evaluate_saved(&state, id, request).await?;
    evaluation.check_token(&request, true).map_err(mapped)?;
    let bytes = state
        .lifecycle
        .blocking
        .run_blocking(move || match format {
            Format::Json => evaluation.export_json(),
            Format::Csv => evaluation.export_csv(),
        })
        .await
        .map_err(|e| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "migration_export_worker_failed",
                e,
            )
        })?
        .map_err(mapped)?;
    let sha = format!("{:x}", Sha256::digest(&bytes));
    let length = bytes.len().to_string();
    let (mime, name) = match format {
        Format::Json => ("application/json", "migration.json"),
        Format::Csv => ("text/csv; charset=utf-8", "migration.csv"),
    };
    let mut response = (
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response();
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        length
            .parse()
            .map_err(|e| error(StatusCode::INTERNAL_SERVER_ERROR, "migration_header", e))?,
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{name}\"")
            .parse()
            .map_err(|e| error(StatusCode::INTERNAL_SERVER_ERROR, "migration_header", e))?,
    );
    response.headers_mut().insert(
        "x-content-sha256",
        sha.parse()
            .map_err(|e| error(StatusCode::INTERNAL_SERVER_ERROR, "migration_header", e))?,
    );
    Ok(response)
}
pub(super) async fn json(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<Request>, JsonRejection>,
) -> MResult<Response> {
    download(state, id, request, Format::Json).await
}
pub(super) async fn csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<Request>, JsonRejection>,
) -> MResult<Response> {
    download(state, id, request, Format::Csv).await
}

#[cfg(test)]
#[path = "migration_tests.rs"]
mod tests;
