use super::*;
use crate::sitemap_membership::{self as membership, CsvQuery, Failure, ReadQuery};
use axum::{body::Body, extract::rejection::QueryRejection};
use sha2::{Digest, Sha256};
type SitemapApiResult<T> = Result<T, (StatusCode, Json<Failure>)>;
fn fail(error: Failure) -> (StatusCode, Json<Failure>) {
    (
        StatusCode::from_u16(error.status()).expect("fixed status"),
        Json(error),
    )
}
fn read_failure(error: anyhow::Error) -> (StatusCode, Json<Failure>) {
    use open_seo_spider_core::SitemapReadWindowError;
    let code = match error.downcast_ref::<SitemapReadWindowError>() {
        Some(SitemapReadWindowError::StaleContext) => "stale_sitemap_cursor",
        Some(SitemapReadWindowError::InvalidContinuation) => "invalid_sitemap_cursor",
        Some(SitemapReadWindowError::ResponseByteBudget) => "sitemap_response_byte_budget",
        None => {
            let message = error.to_string();
            if message.contains("budget exceeded") || message.contains("exceeds bounded input") {
                "sitemap_analysis_input_budget"
            } else {
                "sitemap_read_failure"
            }
        }
    };
    fail(Failure::new(code, error))
}
async fn read_context<T: Send + 'static, P: Send + 'static>(
    state: &AppState,
    id: Uuid,
    prepare: impl FnOnce() -> Result<P, Failure> + Send,
    read_store: impl FnOnce(CrawlStore, P) -> anyhow::Result<T> + Send + 'static,
) -> SitemapApiResult<(u64, String, Option<T>)> {
    let job = get_job(state, id).await.map_err(|(status, Json(error))| {
        (status, Json(Failure::new("job_not_found", error.error)))
    })?;
    let guard = job.read().await;
    // Missing-job404 retains priority. Canonical query/role checks ran before
    // lookup; job/filter/snapshot prechecks still precede opening the Store.
    let prepared = prepare().map_err(fail)?;
    let generation = guard.generation;
    let config_sha256 = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&guard.config)
                .map_err(|error| fail(Failure::new("sitemap_read_failure", error)))?
        )
    );
    // Keep App generation/config stable until the actual one-TX SQL snapshot is
    // captured. Runner DB source commits need no App lock and are pinned by SQL.
    let path = state.data_dir.join(format!("{id}.sqlite"));
    let exists = tokio::fs::try_exists(&path)
        .await
        .map_err(|error| fail(Failure::new("sitemap_read_failure", error)))?;
    let snapshot = if exists {
        Some(
            tokio::task::spawn_blocking(move || {
                read_store(CrawlStore::open_read_only(path)?, prepared)
            })
            .await
            .map_err(|error| fail(Failure::new("sitemap_read_failure", error)))?
            .map_err(read_failure)?,
        )
    } else {
        None
    };
    drop(guard);
    Ok((generation, config_sha256, snapshot))
}
pub(super) async fn inventory(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    query: Result<Query<ReadQuery>, QueryRejection>,
) -> SitemapApiResult<Json<membership::Inventory>> {
    let Query(query) = query.map_err(|error| fail(Failure::new("invalid_sitemap_query", error)))?;
    let analysis_request = query.analysis_request().map_err(fail)?;
    let checked_query = query.clone();
    let (generation, config, read) = read_context(
        &state,
        id,
        move || checked_query.expected_context(id),
        move |store, expected| {
            store.sitemap_read_window_with_membership_analysis(&analysis_request, expected.as_ref())
        },
    )
    .await?;
    let response = tokio::task::spawn_blocking(move || {
        membership::inventory_window(id, generation, &config, read, query)
    })
    .await
    .map_err(|error| fail(Failure::new("sitemap_read_failure", error)))?
    .map_err(fail)?;
    Ok(Json(response))
}
pub(super) async fn export_csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    query: Result<Query<CsvQuery>, QueryRejection>,
) -> SitemapApiResult<Response> {
    let Query(query) = query.map_err(|error| fail(Failure::new("invalid_sitemap_query", error)))?;
    query.validate().map_err(fail)?;
    // CSV remains the existing complete snapshot and buffered transport. Its
    // shared rich state profile accepts fresh Inventory export tokens exactly.
    let (generation, config, read) = read_context(
        &state,
        id,
        || Ok(()),
        |store, ()| store.sitemap_read_snapshot_with_membership_analysis(&Default::default()),
    )
    .await?;
    let (bytes, snapshot) =
        tokio::task::spawn_blocking(move || membership::csv(id, generation, &config, read, query))
            .await
            .map_err(|error| fail(Failure::new("sitemap_read_failure", error)))?
            .map_err(fail)?;
    Response::builder()
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{id}-sitemaps.csv\""),
        )
        .header(
            "x-sitemap-catalog-revision",
            snapshot.catalog_revision.to_string(),
        )
        .header(
            "x-sitemap-catalog-state-sha256",
            snapshot.catalog_state_sha256,
        )
        .header(
            "x-sitemap-catalog-rolling-sha256",
            snapshot.catalog_rolling_sha256,
        )
        .header("x-crawl-generation", snapshot.crawl_generation.to_string())
        .header("x-crawl-config-sha256", snapshot.crawl_config_sha256)
        .body(Body::from(bytes))
        .map_err(|error| fail(Failure::new("sitemap_read_failure", error)))
}

#[cfg(test)]
#[path = "sitemaps_tests.rs"]
mod tests;
