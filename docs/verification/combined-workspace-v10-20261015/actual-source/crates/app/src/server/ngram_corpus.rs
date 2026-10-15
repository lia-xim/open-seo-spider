use super::*;
use crate::ngram_app::{self as projection, Request};
use axum::extract::rejection::JsonRejection;
use std::io::Write;
fn failure(error: impl ToString) -> (StatusCode, Json<ApiError>) {
    let message = error.to_string();
    let status = if message.contains("Stale") || message.contains("generation changed") {
        StatusCode::CONFLICT
    } else if message.contains("limit") || message.contains("budget") || message.contains("exceeds")
    {
        StatusCode::PAYLOAD_TOO_LARGE
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    };
    api_error(status, message)
}
async fn saved(
    state: &AppState,
    id: Uuid,
    request: &Request,
) -> ApiResult<(CrawlReport, projection::Context)> {
    request.validate().map_err(failure)?;
    let job = get_job(state, id).await?;
    let guard = job.read().await;
    if guard.status == "queued" {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Wait for a stable saved report",
        ));
    }
    if let Some(report) = &guard.report {
        projection::preclone(report).map_err(failure)?;
    }
    let saved = comparison::stable_saved_report(
        &guard,
        open_seo_spider_core::ComparisonSide::After,
        request.expected_generation,
    )
    .map_err(|(s, Json(e))| api_error(s, e.error.message))?;
    let context = projection::context(
        &saved.report,
        Some(id.to_string()),
        saved.binding.generation,
        saved
            .binding
            .origin
            .as_deref()
            .unwrap_or("app_declared_unknown_origin"),
    )
    .map_err(failure)?;
    Ok((saved.report, context))
}
pub(super) async fn preview(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Result<Json<Request>, JsonRejection>,
) -> ApiResult<Response> {
    let Json(request) = request.map_err(|e| api_error(StatusCode::BAD_REQUEST, e.body_text()))?;
    let (report, context) = saved(&state, id, &request).await?;
    let (bytes, _) = state
        .lifecycle
        .blocking
        .run_blocking(move || {
            let corpus = projection::aggregate(&report, &request)?;
            projection::preview(&corpus, context, &request)
        })
        .await
        .map_err(|e| api_error(StatusCode::SERVICE_UNAVAILABLE, e))?
        .map_err(failure)?;
    let len = bytes.len().to_string();
    let hash = projection::sha(&bytes);
    Ok((
        [
            (header::CONTENT_TYPE, "application/json; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
            (header::CONTENT_LENGTH, len.as_str()),
        ],
        [("x-content-sha256", hash.as_str())],
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
    if request.expected_snapshot_token.is_none() || request.after.is_some() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Full CSV requires preview snapshot token and no continuation",
        ));
    }
    let (report, context) = saved(&state, id, &request).await?;
    let pending = state
        .lifecycle
        .response_streams
        .start(move |channel, ready| {
            let corpus = projection::aggregate(&report, &request)?;
            let (bytes, snapshot) = projection::csv(&corpus, context, &request)?;
            let len = bytes.len();
            let hash = projection::sha(&bytes);
            ready.send((snapshot.snapshot_token, len, hash))?;
            channel.write_all(&bytes)?;
            channel.flush()?;
            Ok(())
        })
        .await
        .map_err(|e| api_error(StatusCode::SERVICE_UNAVAILABLE, e))?;
    let ((token, len, hash), body) = pending.ready().await.map_err(failure)?;
    Response::builder()
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"saved-ngrams.csv\"",
        )
        .header(header::CONTENT_LENGTH, len.to_string())
        .header("x-content-sha256", hash)
        .header("x-ngram-snapshot", token)
        .body(body)
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))
}
#[cfg(test)]
#[path = "ngram_corpus_tests.rs"]
mod tests;
