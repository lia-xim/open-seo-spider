use super::*;
use crate::resource_export::{self, Format};
use axum::body::{Body, Bytes};
use sha2::{Digest, Sha256};
use std::io::{self, Write};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CssInspectRequest {
    pub(crate) source: open_seo_spider_core::CssSourceInput,
    #[serde(default)]
    pub(crate) limits: open_seo_spider_core::CssInventoryLimits,
}
pub(super) async fn inspect_css(Json(input): Json<CssInspectRequest>) -> ApiResult<Json<Value>> {
    let result = tokio::task::spawn_blocking(move || {
        open_seo_spider_core::discover_css(&input.source, &input.limits)
    })
    .await
    .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
    .map_err(|e| api_error(StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!(result)))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ResourceQuery {
    after_sequence: Option<u64>,
    limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TransportQuery {
    after_url: Option<String>,
    limit: Option<usize>,
}

fn validate_limit(limit: Option<usize>) -> ApiResult<usize> {
    let limit = limit.unwrap_or(200);
    if !(1..=1000).contains(&limit) {
        return Err(api_error(StatusCode::BAD_REQUEST, "limit must be 1..1000"));
    }
    Ok(limit)
}
async fn context(state: &AppState, id: Uuid) -> ApiResult<(Value, Option<PathBuf>)> {
    let job = get_job(state, id).await?;
    let guard = job.read().await;
    let enabled = guard.config.resource_discovery.is_some();
    let config_sha256 = guard
        .config
        .resource_discovery
        .as_ref()
        .map(|config| config.sha256())
        .transpose()
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let crawl_config_sha256 = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&guard.config)
                .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        )
    );
    let metadata = json!({"enabled":enabled,"engine":open_seo_spider_core::RESOURCE_DISCOVERY_ENGINE,
        "checkpoint":guard.resource_ledger,"config_sha256":config_sha256,"crawl_config_sha256":crawl_config_sha256,"crawl_generation":guard.generation,"snapshot":null,"website_requests":0,"entity_bodies_included":false,
        "snapshot_scope":"sqlite_read_snapshot"});
    drop(guard);
    let path = state.data_dir.join(format!("{id}.sqlite"));
    let available = enabled
        && tokio::fs::try_exists(&path)
            .await
            .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((metadata, available.then_some(path)))
}
fn unavailable(mut metadata: Value) -> Value {
    metadata["ledger_available"] = json!(false);
    metadata["reason"] = json!(if metadata["enabled"].as_bool() == Some(true) {
        "initializing_or_imported_history_without_sqlite_rows"
    } else {
        "native_resource_discovery_disabled"
    });
    metadata
}
fn validate_binding(
    metadata: &Value,
    summary: &open_seo_spider_core::ResourceLedgerSummary,
) -> ApiResult<()> {
    if summary.engine != open_seo_spider_core::RESOURCE_DISCOVERY_ENGINE
        || metadata["config_sha256"].as_str() != Some(summary.config_sha256.as_str())
    {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Native resource ledger configuration differs from this crawl",
        ));
    }
    Ok(())
}
fn validate_snapshot(
    metadata: &Value,
    snapshot: Option<&open_seo_spider_core::ResourceReadSnapshot>,
) -> ApiResult<()> {
    let expected_generation = metadata["crawl_generation"].as_u64().filter(|v| *v > 0);
    if snapshot.is_none_or(|snapshot| {
        metadata["crawl_config_sha256"].as_str() != Some(snapshot.crawl_config_sha256.as_str())
            || expected_generation.is_none()
            || snapshot.generation != expected_generation
    }) {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Resource read snapshot differs from this crawl configuration or generation",
        ));
    }
    Ok(())
}
pub(super) async fn inventory(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<ResourceQuery>,
) -> ApiResult<Json<Value>> {
    let limit = validate_limit(query.limit)?;
    if query.after_sequence.is_some_and(|v| v > i64::MAX as u64) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "after_sequence exceeds SQLite integer range",
        ));
    }
    let (mut metadata, path) = context(&state, id).await?;
    let Some(path) = path else {
        metadata = unavailable(metadata);
        metadata["observations"] = json!([]);
        metadata["next_after_sequence"] = Value::Null;
        metadata["has_more"] = json!(false);
        return Ok(Json(metadata));
    };
    let page = tokio::task::spawn_blocking(move || {
        CrawlStore::open_read_only(path)?.resource_occurrences(query.after_sequence, limit)
    })
    .await
    .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
    .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    if let Some(ledger) = page.ledger.as_ref() {
        validate_binding(&metadata, ledger)?;
        validate_snapshot(&metadata, page.snapshot.as_ref())?;
        metadata["checkpoint"] = json!(page.ledger);
        metadata["ledger_available"] = json!(true);
        metadata["snapshot"] = json!(page.snapshot);
    } else {
        metadata = unavailable(metadata);
    }
    metadata["observations"] = json!(page.rows);
    metadata["next_after_sequence"] = json!(page.next_sequence);
    metadata["has_more"] = json!(page.has_more);
    Ok(Json(metadata))
}
pub(super) async fn transports(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<TransportQuery>,
) -> ApiResult<Json<Value>> {
    let limit = validate_limit(query.limit)?;
    if query
        .after_url
        .as_ref()
        .is_some_and(|v| v.is_empty() || v.len() > 65536)
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Invalid after_url length",
        ));
    }
    let (mut metadata, path) = context(&state, id).await?;
    let Some(path) = path else {
        metadata = unavailable(metadata);
        metadata["transports"] = json!([]);
        metadata["next_after_url"] = Value::Null;
        metadata["has_more"] = json!(false);
        return Ok(Json(metadata));
    };
    let page = tokio::task::spawn_blocking(move || {
        CrawlStore::open_read_only(path)?.resource_transports(query.after_url.as_deref(), limit)
    })
    .await
    .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
    .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    if let Some(ledger) = page.ledger.as_ref() {
        validate_binding(&metadata, ledger)?;
        validate_snapshot(&metadata, page.snapshot.as_ref())?;
        metadata["checkpoint"] = json!(page.ledger);
        metadata["ledger_available"] = json!(true);
        metadata["snapshot"] = json!(page.snapshot);
    } else {
        metadata = unavailable(metadata);
    }
    metadata["transports"] = json!(page.rows);
    metadata["next_after_url"] = json!(page.next_url);
    metadata["has_more"] = json!(page.has_more);
    Ok(Json(metadata))
}
pub(super) async fn export_json(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    export(state, id, Format::Json).await
}
pub(super) async fn export_csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    export(state, id, Format::Csv).await
}

struct ChannelWriter(tokio::sync::mpsc::Sender<Result<Bytes, io::Error>>);
impl Write for ChannelWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for chunk in bytes.chunks(65536) {
            self.0
                .blocking_send(Ok(Bytes::copy_from_slice(chunk)))
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "Resource export consumer disconnected",
                    )
                })?;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
async fn export(state: AppState, id: Uuid, format: Format) -> ApiResult<Response> {
    let (metadata, path) = context(&state, id).await?;
    let Some(path) = path else {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Native resource SQLite rows are not available",
        ));
    };
    let (sender, receiver) = tokio::sync::mpsc::channel(8);
    let (ready, ready_result) = tokio::sync::oneshot::channel();
    tokio::task::spawn_blocking(move || {
        let mut ready = Some(ready);
        let result = (|| {
            let store = CrawlStore::open_read_only(path)?;
            // Check availability before starting the stream. The visitor then pins
            // and validates its own complete read snapshot.
            anyhow::ensure!(
                store.resource_ledger_summary()?.is_some(),
                "Native resource ledger is not available"
            );
            let mut writer =
                std::io::BufWriter::with_capacity(65536, ChannelWriter(sender.clone()));
            resource_export::write_snapshot(&store, format, &mut writer, |summary, snapshot| {
                validate_binding(&metadata, summary)
                    .map_err(|(_, Json(error))| anyhow::anyhow!(error.error))?;
                validate_snapshot(&metadata, Some(snapshot))
                    .map_err(|(_, Json(error))| anyhow::anyhow!(error.error))?;
                if let Some(ready) = ready.take() {
                    ready
                        .send(Ok((summary.clone(), snapshot.clone())))
                        .map_err(|_| anyhow::anyhow!("Resource export request disconnected"))?;
                }
                Ok(())
            })
        })();
        if let Err(error) = result {
            if let Some(ready) = ready.take() {
                let _ = ready.send(Err(error.to_string()));
            } else {
                let _ = sender.blocking_send(Err(io::Error::other(error.to_string())));
            }
        }
    });
    let (summary, snapshot) = ready_result
        .await
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .map_err(|e| api_error(StatusCode::CONFLICT, e))?;
    let stream = futures::stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|item| (item, receiver))
    });
    let (kind, extension) = match format {
        Format::Json => ("application/json; charset=utf-8", "json"),
        Format::Csv => ("text/csv; charset=utf-8", "csv"),
    };
    Response::builder()
        .header(header::CONTENT_TYPE, kind)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{id}-resources.{extension}\""),
        )
        .header("x-resource-snapshot-revision", summary.revision.to_string())
        .header(
            "x-resource-snapshot-observations",
            summary.decision_rows.to_string(),
        )
        .header("x-resource-config-sha256", summary.config_sha256)
        .header("x-crawl-config-sha256", snapshot.crawl_config_sha256)
        .header(
            "x-crawl-generation",
            snapshot
                .generation
                .map(|v| v.to_string())
                .unwrap_or_default(),
        )
        .header("x-resource-ledger-sha256", summary.rolling_sha256)
        .header(
            "x-resource-discovery-complete",
            summary.complete.to_string(),
        )
        .body(Body::from_stream(stream))
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))
}
