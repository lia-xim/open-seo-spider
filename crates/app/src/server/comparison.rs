//! Additional saved comparison API; existing GET /api/compare is untouched.
use super::*;
use crate::comparison_app::{
    ComparisonConflictContext, SavedComparisonFailure, SavedComparisonRequest,
    SavedComparisonResponse, bounded_bundle_failure, failure,
};
use axum::extract::rejection::JsonRejection;
use open_seo_spider_core::{
    ComparisonCallerBinding, ComparisonSide, SavedComparisonLimits, compare_saved_crawls,
};

type ComparisonApiResult<T> = Result<T, (StatusCode, Json<SavedComparisonFailure>)>;

pub(super) struct StableSavedReport {
    pub(super) report: CrawlReport,
    pub(super) binding: ComparisonCallerBinding,
}
fn conflict(
    job: &Job,
    side: ComparisonSide,
    expected: Option<u64>,
    code: &str,
    message: &str,
) -> (StatusCode, Json<SavedComparisonFailure>) {
    let mut error = failure(code, message);
    error.context = Some(Box::new(ComparisonConflictContext {
        side,
        job_id: job.id,
        status: job.status.clone(),
        observed_generation: job.generation,
        expected_generation: expected,
    }));
    (StatusCode::CONFLICT, Json(error))
}
pub(super) fn stable_saved_report(
    job: &Job,
    side: ComparisonSide,
    expected: Option<u64>,
) -> ComparisonApiResult<StableSavedReport> {
    if matches!(job.status.as_str(), "running" | "pausing" | "cancelling") {
        return Err(conflict(
            job,
            side,
            expected,
            "unstable_snapshot",
            "Wait for a stable saved report before comparison",
        ));
    }
    if expected.is_some_and(|expected| expected != job.generation) {
        return Err(conflict(
            job,
            side,
            expected,
            "generation_conflict",
            "Expected App job generation differs from the generation under the read lock",
        ));
    }
    let report = job.report.as_ref().ok_or_else(|| {
        conflict(
            job,
            side,
            expected,
            "saved_report_unavailable",
            "Saved report is unavailable",
        )
    })?;
    if report.pages.len() > SavedComparisonLimits::default().max_pages_per_snapshot {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(failure(
                "page_count_budget",
                "Snapshot page count exceeds the comparison ceiling; no clone/join performed",
            )),
        ));
    }
    // These are cached App facts read with the report. No SQL file, generation,
    // checkpoint digest or native report authenticity is attested here.
    let origin = if job.checkpoint.is_some() {
        "app_declared_job_report_with_cached_checkpoint"
    } else if !job.frontier_available {
        "app_declared_imported_or_detached_report"
    } else {
        "app_declared_job_report_without_cached_checkpoint"
    };
    let basis = json!({
        "declaration":"app_declared", "source":"job_readlock",
        "status":job.status,"frontier":job.frontier_available,
        "checkpoint_revision":job.checkpoint.as_ref().map(|value| value.revision),
        "checkpoint_generation":job.checkpoint.as_ref().map(|value| value.generation),
        "sql_attested":false,"report_generation_attested":false
    })
    .to_string();
    Ok(StableSavedReport {
        report: report.clone(),
        binding: ComparisonCallerBinding {
            job_id: Some(job.id.to_string()),
            // Zero is the App's import/detached sentinel, not an observed saved generation.
            generation: (job.generation > 0).then_some(job.generation),
            origin: Some(origin.into()),
            basis: Some(basis),
            active_checkpoint: Some(false),
            snapshot_sha256: None,
            sha_profile: None,
        },
    })
}
async fn stable_pair(
    state: &AppState,
    request: &SavedComparisonRequest,
) -> ComparisonApiResult<(StableSavedReport, StableSavedReport)> {
    let (before_job, after_job) = {
        let registry = state.jobs.read().await;
        let before = registry.get(&request.before).cloned().ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(failure(
                    "comparison_job_not_found",
                    format!("Before job {} was not found", request.before),
                )),
            )
        })?;
        let after = registry.get(&request.after).cloned().ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(failure(
                    "comparison_job_not_found",
                    format!("After job {} was not found", request.after),
                )),
            )
        })?;
        (before, after)
    };
    if request.before == request.after {
        // A single guard also makes contradictory expected generations visible;
        // it cannot observe two revisions of the same job in one comparison.
        let guard = before_job.read().await;
        return Ok((
            stable_saved_report(
                &guard,
                ComparisonSide::Before,
                request.expected_before_generation,
            )?,
            stable_saved_report(
                &guard,
                ComparisonSide::After,
                request.expected_after_generation,
            )?,
        ));
    }
    // Both guards stay held through status/generation checks and both clones.
    // Deterministic ordering avoids crossed pair-lock acquisition.
    let (first, second) = if request.before.as_u128() < request.after.as_u128() {
        (&before_job, &after_job)
    } else {
        (&after_job, &before_job)
    };
    let first_guard = first.read().await;
    let second_guard = second.read().await;
    let (before, after) = if request.before.as_u128() < request.after.as_u128() {
        (&*first_guard, &*second_guard)
    } else {
        (&*second_guard, &*first_guard)
    };
    Ok((
        stable_saved_report(
            before,
            ComparisonSide::Before,
            request.expected_before_generation,
        )?,
        stable_saved_report(
            after,
            ComparisonSide::After,
            request.expected_after_generation,
        )?,
    ))
}
fn compare_stable_reports(
    before: StableSavedReport,
    after: StableSavedReport,
    config: open_seo_spider_core::SavedComparisonConfig,
    include_original_reports: bool,
) -> Result<SavedComparisonResponse, SavedComparisonFailure> {
    let result = compare_saved_crawls(
        &before.report,
        &after.report,
        &config,
        Some(&before.binding),
        Some(&after.binding),
    )
    .map_err(SavedComparisonFailure::from)?;
    if include_original_reports {
        // Move the very reports passed by reference to Core; no second read
        // or clone supplies the original-report bundle after releasing locks.
        SavedComparisonResponse::with_original_reports(result, before.report, after.report)
    } else {
        SavedComparisonResponse::new(result)
    }
}
pub(super) async fn compare_saved(
    State(state): State<AppState>,
    request: Result<Json<SavedComparisonRequest>, JsonRejection>,
) -> ComparisonApiResult<Json<SavedComparisonResponse>> {
    let Json(request) = request.map_err(|error| {
        (
            StatusCode::BAD_REQUEST,
            Json(failure("invalid_comparison_request", error.body_text())),
        )
    })?;
    let include_original_reports = request.include_original_reports;
    let (before, after) =
        stable_pair(&state, &request)
            .await
            .map_err(|(status, Json(error))| {
                if include_original_reports {
                    let bounded = bounded_bundle_failure(error);
                    let status = if bounded.error.code == "original_report_bundle_byte_budget" {
                        StatusCode::BAD_REQUEST
                    } else {
                        status
                    };
                    (status, Json(bounded))
                } else {
                    (status, Json(error))
                }
            })?;
    let config = request.config.unwrap_or_default();
    // All locks are released. Core alone computes and validates report hashes.
    state
        .lifecycle
        .blocking
        .run_blocking(move || {
            compare_stable_reports(before, after, config, include_original_reports)
        })
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(failure("comparison_task_failed", error.to_string())),
            )
        })?
        .map(Json)
        .map_err(|error| {
            let error = if include_original_reports {
                bounded_bundle_failure(error)
            } else {
                error
            };
            let status = if error.error.code == "original_report_binding_mismatch" {
                StatusCode::INTERNAL_SERVER_ERROR
            } else {
                StatusCode::BAD_REQUEST
            };
            (status, Json(error))
        })
}

#[cfg(test)]
#[path = "comparison_tests.rs"]
mod tests;
