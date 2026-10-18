use super::*;
use crate::segments_app::{self as service, Request};
use axum::extract::rejection::JsonRejection;
use open_seo_spider_core::{ComparisonSide, segments as core};
#[derive(Debug, Serialize)]
pub(super) struct Failure {
    pub code: String,
    pub message: String,
}
type SResult<T> = Result<T, (StatusCode, Json<Failure>)>;
fn fail(status: StatusCode, code: &str, message: impl ToString) -> (StatusCode, Json<Failure>) {
    (
        status,
        Json(Failure {
            code: code.into(),
            message: message.to_string(),
        }),
    )
}
fn bad(e: impl ToString) -> (StatusCode, Json<Failure>) {
    fail(StatusCode::BAD_REQUEST, "segments_refused", e)
}
pub(super) fn evaluate(job: &Job, request: &Request) -> SResult<service::Evaluation> {
    request.validate().map_err(bad)?;
    let report = job.report.as_ref().ok_or_else(|| {
        fail(
            StatusCode::CONFLICT,
            "report_unavailable",
            "Saved report unavailable",
        )
    })?;
    // Preserve the existing input-budget400 contract before authority checks.
    // The borrowed binding helper now avoids copying the measured report.
    service::report_hash(report).map_err(bad)?;
    let binding = comparison::stable_saved_report_binding(
        job,
        ComparisonSide::Before,
        request.expected_generation,
    )
    .map_err(|(s, Json(e))| fail(s, &e.error.code, e.error.message))?;
    let origin = binding
        .origin
        .as_deref()
        .unwrap_or("app_declared_saved_report");
    service::evaluate(
        report,
        request,
        Some(job.id.to_string()),
        Some(job.generation),
        origin,
    )
    .map_err(|e| {
        fail(
            StatusCode::CONFLICT,
            "segments_snapshot_or_rules_refused",
            e,
        )
    })
}
fn response(bytes: Vec<u8>, csv: bool, attachment: bool) -> Response {
    let digest = service::sha(&bytes);
    let mut r = (
        [
            (
                "content-type",
                if csv {
                    "text/csv; charset=utf-8"
                } else {
                    "application/json; charset=utf-8"
                },
            ),
            ("cache-control", "no-store"),
        ],
        bytes,
    )
        .into_response();
    r.headers_mut()
        .insert("x-content-sha256", digest.parse().expect("hex digest"));
    if attachment {
        r.headers_mut().insert(
            "content-disposition",
            if csv {
                "attachment; filename=segments.csv"
            } else {
                "attachment; filename=segments.json"
            }
            .parse()
            .expect("fixed attachment header"),
        );
    }
    r
}
async fn read(
    state: AppState,
    id: Uuid,
    request: Result<Json<Request>, JsonRejection>,
    export: Option<bool>,
) -> SResult<Response> {
    let Json(request) = request.map_err(bad)?;
    request.validate().map_err(bad)?;
    if export.is_some() && (request.expected_snapshot_token.is_none() || request.cursor.is_some()) {
        return Err(bad(
            "Full export requires the preview snapshot token and no page cursor",
        ));
    }
    let job = get_job(&state, id)
        .await
        .map_err(|(s, Json(e))| fail(s, "job_not_found", e.error))?;
    let guard = job.read_owned().await;
    state
        .lifecycle
        .blocking
        .run_blocking(move || {
            let e = evaluate(&guard, &request)?;
            let report = guard.report.as_ref().expect("evaluated saved report");
            let bytes = match export {
                Some(true) => service::full_csv(report, &e),
                Some(false) => service::full_json(report, &e),
                None => service::page(report, &e, &request),
            }
            .map_err(bad)?;
            Ok(response(bytes, export == Some(true), export.is_some()))
        })
        .await
        .map_err(|e| {
            fail(
                StatusCode::INTERNAL_SERVER_ERROR,
                "segments_worker_failed",
                e,
            )
        })?
}
pub(super) async fn preview(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    r: Result<Json<Request>, JsonRejection>,
) -> SResult<Response> {
    read(s, id, r, None).await
}
pub(super) async fn json(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    r: Result<Json<Request>, JsonRejection>,
) -> SResult<Response> {
    read(s, id, r, Some(false)).await
}
pub(super) async fn csv(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    r: Result<Json<Request>, JsonRejection>,
) -> SResult<Response> {
    read(s, id, r, Some(true)).await
}
pub(super) const REQUEST_BYTES: usize = core::REQUEST_BYTES;
#[cfg(test)]
#[path = "segments_tests.rs"]
mod tests;
