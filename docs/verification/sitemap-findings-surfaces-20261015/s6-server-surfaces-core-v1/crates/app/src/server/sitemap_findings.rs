//! Full saved-data findings. Every positive result comes from one sealed Core read.
use super::*;
use crate::sitemap_findings::{self as findings, AccessTarget, CsvQuery, Failure, ReadQuery};
use axum::extract::rejection::{JsonRejection, QueryRejection};
use sha2::{Digest, Sha256};

type FindingsApiResult<T> = Result<T, (StatusCode, Json<Failure>)>;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/crawls/{id}/sitemaps/findings",
            get(inventory_get)
                .post(inventory_post)
                .layer(axum::extract::DefaultBodyLimit::max(
                    findings::MAX_REQUEST_BYTES,
                )),
        )
        .route(
            "/api/crawls/{id}/sitemaps/findings/export.csv",
            get(export_get)
                .post(export_post)
                .layer(axum::extract::DefaultBodyLimit::max(
                    findings::MAX_REQUEST_BYTES,
                )),
        )
}

fn fail(error: Failure) -> (StatusCode, Json<Failure>) {
    (
        StatusCode::from_u16(findings::status(&error)).expect("fixed findings status"),
        Json(error),
    )
}

fn json_failure(error: JsonRejection) -> (StatusCode, Json<Failure>) {
    let code = if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        "sitemap_findings_input_byte_budget"
    } else {
        "invalid_sitemap_findings_query"
    };
    fail(Failure::new(code, error.body_text()))
}

fn query_failure(error: QueryRejection) -> (StatusCode, Json<Failure>) {
    fail(Failure::new(
        "invalid_sitemap_findings_query",
        error.body_text(),
    ))
}

fn job_target(id: Uuid, job: &Job) -> anyhow::Result<AccessTarget> {
    Ok(AccessTarget::Job {
        job_id: id,
        generation: job.generation,
        config_sha256: format!("{:x}", Sha256::digest(serde_json::to_vec(&job.config)?)),
    })
}

pub(super) async fn inventory_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    query: Result<Query<ReadQuery>, QueryRejection>,
) -> FindingsApiResult<Response> {
    let Query(query) = query.map_err(query_failure)?;
    inventory(state, id, query).await
}

pub(super) async fn inventory_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    query: Result<Json<ReadQuery>, JsonRejection>,
) -> FindingsApiResult<Response> {
    let Json(query) = query.map_err(json_failure)?;
    inventory(state, id, query).await
}

async fn inventory(state: AppState, id: Uuid, query: ReadQuery) -> FindingsApiResult<Response> {
    // Strict syntax/domain/role checks precede job lookup and all Store I/O.
    // The owned read registry also covers potentially large literal selectors.
    let query = state
        .finding_reads
        .read(move |control| {
            control.checkpoint()?;
            query.validate()?;
            control.checkpoint()?;
            Ok(query)
        })
        .await
        .map_err(fail)?;
    let job = get_job(&state, id).await.map_err(|(status, Json(error))| {
        (status, Json(Failure::new("job_not_found", error.error)))
    })?;
    let guard = job.read().await;
    let target = job_target(id, &guard).map_err(|e| fail(findings::typed_failure(e)))?;
    let path = state.data_dir.join(format!("{id}.sqlite"));
    let exists = tokio::fs::try_exists(&path)
        .await
        .map_err(|e| fail(findings::typed_failure(e.into())))?;
    let bytes = state
        .finding_reads
        .read(move |control| {
            control.checkpoint()?;
            let prepared = query.prepare(&target)?;
            let envelope = if exists {
                let store = CrawlStore::open_read_only(path)?;
                store.with_verified_sitemap_findings(
                    prepared.selection(),
                    prepared.request().expected_context_sha256.as_deref(),
                    &control,
                    |session| {
                        let page = session.page(prepared.request())?;
                        findings::bind_page(&target, &prepared, page).map_err(Into::into)
                    },
                )?
            } else {
                // A detached JSON report does not manufacture a Core SQL proof.
                findings::unavailable_page(&target)?
            };
            control.checkpoint()?;
            // bind_page measures the complete envelope before this allocation.
            let bytes = serde_json::to_vec(&envelope)?;
            anyhow::ensure!(
                bytes.len() <= findings::MAX_JSON_BYTES,
                "Measured finding envelope changed"
            );
            control.checkpoint()?;
            Ok(bytes)
        })
        .await
        .map_err(fail)?;
    // Keep the actual App generation/config stable until the SQL read has
    // returned and its blocking worker has actually joined.
    drop(guard);
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_LENGTH, bytes.len().to_string())
        .body(axum::body::Body::from(bytes))
        .map_err(|e| fail(findings::typed_failure(e.into())))
}

pub(super) async fn export_get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    query: Result<Query<CsvQuery>, QueryRejection>,
) -> FindingsApiResult<Response> {
    let Query(query) = query.map_err(query_failure)?;
    export(state, id, query).await
}

pub(super) async fn export_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    query: Result<Json<CsvQuery>, JsonRejection>,
) -> FindingsApiResult<Response> {
    let Json(query) = query.map_err(json_failure)?;
    export(state, id, query).await
}

async fn export(state: AppState, id: Uuid, query: CsvQuery) -> FindingsApiResult<Response> {
    let query = state
        .finding_reads
        .read(move |control| {
            control.checkpoint()?;
            query.validate()?;
            control.checkpoint()?;
            Ok(query)
        })
        .await
        .map_err(fail)?;
    let job = get_job(&state, id).await.map_err(|(status, Json(error))| {
        (status, Json(Failure::new("job_not_found", error.error)))
    })?;
    let guard = job.read().await;
    let target = job_target(id, &guard).map_err(|e| fail(findings::typed_failure(e)))?;
    let prepared = state
        .finding_reads
        .read(move |control| {
            control.checkpoint()?;
            let prepared = query.prepare(&target)?;
            control.checkpoint()?;
            Ok(prepared)
        })
        .await
        .map_err(fail)?;
    let path = state.data_dir.join(format!("{id}.sqlite"));
    if !tokio::fs::try_exists(&path)
        .await
        .map_err(|e| fail(findings::typed_failure(e.into())))?
    {
        return Err(fail(Failure::new(
            "sitemap_findings_unavailable",
            "Saved SQLite findings unavailable",
        )));
    }
    let mut export = state.exports.start_findings(path, prepared).map_err(fail)?;
    // Ready follows the entire verified count/hash pass in the same transaction
    // that will produce the CSV body. No success header precedes that pass.
    let ready = export.ready().await.map_err(fail)?;
    drop(guard);
    let body = export.into_body().map_err(fail)?;
    let mut builder = Response::builder();
    if state.exports.response_build_fault() {
        builder = builder.header("x-csv-test-invalid", "\n");
    }
    builder
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{id}-sitemap-findings.csv\""),
        )
        .header(header::CONTENT_LENGTH, ready.length.to_string())
        .header("x-sitemap-csv-sha256", ready.sha256)
        .header(
            "x-sitemap-findings-context-sha256",
            ready.snapshot.core_context_sha256,
        )
        .header(
            "x-sitemap-findings-selection-sha256",
            ready.snapshot.selection_sha256,
        )
        .body(body)
        .map_err(|e| fail(findings::typed_failure(e.into())))
}

#[cfg(test)]
#[path = "sitemap_findings_tests.rs"]
mod tests;
