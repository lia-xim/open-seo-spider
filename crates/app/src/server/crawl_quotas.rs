//! Read one transaction-consistent quota ledger. Report/page statistics are not reconstructed.
use super::*;
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SummaryQuery {}

pub(super) async fn summary(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(_): Query<SummaryQuery>,
) -> ApiResult<Json<Value>> {
    let job = get_job(&state, id).await?;
    let guard = job.read_owned().await;
    if guard.config.crawl_quotas.is_none() {
        return Ok(Json(
            json!({"id":id,"enabled":false,"availability":"legacy_unavailable","summary":null,"reason":"No durable crawl quota profile was enabled; historical counts are unknown"}),
        ));
    }
    let path = state.data_dir.join(format!("{id}.sqlite"));
    let summary = state
        .lifecycle
        .blocking
        .run_blocking(move || {
            // Keep the job read guard until the read has completed, excluding a concurrent resume.
            let result = crate::crawl_quotas_app::read_summary(&path);
            drop(guard);
            result
        })
        .await
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .map_err(|e| {
            api_error(
                StatusCode::CONFLICT,
                format!("Quota accounting unavailable: {e}"),
            )
        })?
        .ok_or_else(|| {
            api_error(
                StatusCode::CONFLICT,
                "Enabled quota profile has no durable accounting",
            )
        })?;
    Ok(Json(
        json!({"id":id,"enabled":true,"availability":"available","summary":summary,"qualification":"One durable quota read snapshot; no report/page/current-request join attestation"}),
    ))
}

#[cfg(test)]
#[path = "crawl_quotas_tests.rs"]
mod tests;
