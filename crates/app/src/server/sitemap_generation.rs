use super::*;
use axum::{body::Body, extract::rejection::JsonRejection};
use open_seo_spider_core::{
    SitemapGenerationFailure, SitemapGenerationFailureCode as F, SitemapGenerationOptions,
    generate_sitemap_bundle,
};
type GenerationResult<T> = Result<T, (StatusCode, Json<SitemapGenerationFailure>)>;
fn failure(error: SitemapGenerationFailure) -> (StatusCode, Json<SitemapGenerationFailure>) {
    let status = match error.code {
        F::InvalidOptions | F::InvalidLastmod => StatusCode::BAD_REQUEST,
        F::InvalidSource => StatusCode::CONFLICT,
        F::InputBudget | F::EntryByteLimit | F::TotalOutputBudget | F::IndexLimit => {
            StatusCode::PAYLOAD_TOO_LARGE
        }
        F::NoEligibleUrls => StatusCode::UNPROCESSABLE_ENTITY,
        F::ZipFailure => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(error))
}
fn error(code: F, message: impl Into<String>) -> SitemapGenerationFailure {
    SitemapGenerationFailure {
        code,
        message: message.into(),
        diagnostic_counts: Default::default(),
    }
}
pub(super) async fn bundle(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    input: Result<Json<SitemapGenerationOptions>, JsonRejection>,
) -> GenerationResult<Response> {
    let Json(options) = input.map_err(|err| failure(error(F::InvalidOptions, err.to_string())))?;
    let report = require_report(&state, id)
        .await
        .map_err(|(status, Json(err))| (status, Json(error(F::InvalidSource, err.error))))?;
    // A captured immutable report can predate a running invocation. Neither its
    // pages nor source dates are projected as current freshness/full-site proof.
    let generated = tokio::task::spawn_blocking(move || generate_sitemap_bundle(&report, &options))
        .await
        .map_err(|err| failure(error(F::ZipFailure, err.to_string())))?
        .map_err(failure)?;
    Response::builder()
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_LENGTH,
            generated.zip_bytes.len().to_string(),
        )
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{id}-sitemap-bundle.zip\""),
        )
        .header("x-sitemap-generation-engine", generated.manifest.engine)
        .header("x-sitemap-report-sha256", generated.manifest.report_sha256)
        .header("x-sitemap-bundle-sha256", generated.zip_sha256)
        .header(
            "x-sitemap-generated-urls",
            generated.manifest.memberships.len().to_string(),
        )
        .header(
            "x-sitemap-excluded-pages",
            generated.manifest.excluded_pages.to_string(),
        )
        .body(Body::from(generated.zip_bytes))
        .map_err(|err| failure(error(F::ZipFailure, err.to_string())))
}

#[cfg(test)]
#[path = "sitemap_generation_tests.rs"]
mod tests;
