use super::*;
use crate::content_cohort::{self, SavedContentRequest};
use open_seo_spider_core::ContentCaptureSourceKind;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InventoryQuery {
    #[serde(default)]
    after_ordinal: usize,
    #[serde(default)]
    reporting_mode: Option<content_cohort::ReportingMode>,
    #[serde(default)]
    expected_generation: Option<u64>,
    #[serde(default)]
    expected_snapshot_sha256: Option<String>,
    #[serde(default)]
    expected_reporting_identity: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default = "content_cohort::default_source")]
    source_kind: ContentCaptureSourceKind,
}
fn default_limit() -> usize {
    200
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DocumentQuery {
    url: String,
    #[serde(default)]
    reporting_mode: Option<content_cohort::ReportingMode>,
    #[serde(default)]
    expected_generation: Option<u64>,
    #[serde(default)]
    expected_snapshot_sha256: Option<String>,
    #[serde(default)]
    expected_reporting_identity: Option<String>,
    #[serde(default = "content_cohort::default_source")]
    source_kind: ContentCaptureSourceKind,
}
pub(super) fn stable_report(job: &Job) -> ApiResult<&CrawlReport> {
    if matches!(job.status.as_str(), "running" | "pausing" | "cancelling") {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Wait for a stable saved checkpoint before reading content",
        ));
    }
    job.report
        .as_ref()
        .ok_or_else(|| api_error(StatusCode::CONFLICT, "Saved report is unavailable"))
}
pub(super) fn context(
    state: &AppState,
    job: &Job,
    report: &CrawlReport,
) -> ApiResult<content_cohort::SnapshotContext> {
    let mut context = content_cohort::snapshot_context(report, None, "declared_imported_report")
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?;
    // A path alone is not evidence of native storage. Use the validated cached
    // terminal checkpoint and bind its report hash/generation to this projection.
    if state.data_dir.join(format!("{}.sqlite", job.id)).exists()
        && job.checkpoint.as_ref().is_some_and(|checkpoint| {
            checkpoint.generation > 0
                && checkpoint.generation == job.generation
                && checkpoint.report_sha256 == context.sha256
        })
    {
        context.origin = "native_saved_report".into();
        context.generation = Some(job.generation);
    }
    Ok(context)
}
pub(super) async fn inventory(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<InventoryQuery>,
) -> ApiResult<Json<Value>> {
    let job = get_job(&state, id).await?;
    let guard = job.read().await;
    let report = stable_report(&guard)?;
    let context = context(&state, &guard, report)?;
    content_cohort::inventory_with_reporting(
        report,
        &context,
        query.source_kind,
        query.after_ordinal,
        query.limit,
        &content_cohort::ReportingOptions {
            mode: query.reporting_mode,
            expected_generation: query.expected_generation,
            expected_snapshot_sha256: query.expected_snapshot_sha256,
            expected_reporting_identity: query.expected_reporting_identity,
        },
    )
    .map(Json)
    .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))
}
pub(super) async fn document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<DocumentQuery>,
) -> ApiResult<Json<Value>> {
    let job = get_job(&state, id).await?;
    let guard = job.read().await;
    let report = stable_report(&guard)?;
    let context = context(&state, &guard, report)?;
    content_cohort::saved_document_with_reporting(
        report,
        &context,
        query.source_kind,
        &query.url,
        &content_cohort::ReportingOptions {
            mode: query.reporting_mode,
            expected_generation: query.expected_generation,
            expected_snapshot_sha256: query.expected_snapshot_sha256,
            expected_reporting_identity: query.expected_reporting_identity,
        },
    )
    .map(Json)
    .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))
}
pub(super) async fn analyze(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<SavedContentRequest>,
) -> ApiResult<Json<Value>> {
    let job = get_job(&state, id).await?;
    let prepared = {
        let guard = job.read().await;
        let report = stable_report(&guard)?;
        let context = context(&state, &guard, report)?;
        content_cohort::prepare_saved_analysis(report, &context, &request)
            .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?
    };
    state
        .lifecycle
        .blocking
        .run_blocking(move || content_cohort::finish_saved_analysis(prepared.0, prepared.1))
        .await
        .map_err(|error| api_error(StatusCode::INTERNAL_SERVER_ERROR, error))?
        .map(Json)
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))
}
