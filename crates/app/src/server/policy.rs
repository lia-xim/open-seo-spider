use super::*;
use crate::policy_export::{self, Format};
use axum::body::{Body, Bytes};
use std::io::{self, Write};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PolicyQuery {
    after_sequence: Option<u64>,
    limit: Option<usize>,
}

pub(super) async fn inventory(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<PolicyQuery>,
) -> ApiResult<Json<Value>> {
    let limit = query.limit.unwrap_or(200);
    if !(1..=1000).contains(&limit) {
        return Err(api_error(StatusCode::BAD_REQUEST, "limit must be 1..1000"));
    }
    if query
        .after_sequence
        .is_some_and(|value| value > i64::MAX as u64)
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "after_sequence exceeds SQLite integer range",
        ));
    }
    let job = get_job(&state, id).await?;
    let guard = job.read().await;
    let enabled = guard.config.crawl_policy.is_some();
    let scope = guard
        .config
        .crawl_policy
        .as_ref()
        .map(|policy| &policy.scope);
    let metadata = json!({"enabled":enabled,"scope":scope,
        "engine":open_seo_spider_core::LIVE_CRAWL_POLICY_ENGINE});
    let imported_summary = guard.policy_ledger.clone();
    drop(guard);
    let path = state.data_dir.join(format!("{id}.sqlite"));
    if !enabled
        || !tokio::fs::try_exists(&path)
            .await
            .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        return Ok(Json(json!({"enabled":enabled,"engine":metadata["engine"],
            "scope":metadata["scope"],"policy_sha256":imported_summary.as_ref().map(|ledger|&ledger.policy_sha256),
            "checkpoint":imported_summary,"observations":[],"next_after_sequence":null,"has_more":false,
            "ledger_available":false,"reason":if enabled {"initializing_or_imported_history_without_sqlite_rows"}else{"native_policy_disabled"}})));
    }
    let page = state
        .lifecycle
        .blocking
        .run_blocking(move || {
            CrawlStore::open_read_only(path)?.policy_occurrences(query.after_sequence, limit)
        })
        .await
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(
        json!({"enabled":true,"engine":metadata["engine"],"scope":metadata["scope"],
        "policy_sha256":page.ledger.as_ref().map(|ledger|&ledger.policy_sha256),
        "ledger_available":page.ledger.is_some(),"checkpoint":page.ledger,
        "observations":page.rows,"next_after_sequence":page.next_sequence,"has_more":page.has_more}),
    ))
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
        self.0
            .blocking_send(Ok(Bytes::copy_from_slice(bytes)))
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Policy export consumer disconnected",
                )
            })?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

async fn export(state: AppState, id: Uuid, format: Format) -> ApiResult<Response> {
    let job = get_job(&state, id).await?;
    if job.read().await.config.crawl_policy.is_none() {
        return Err(api_error(
            StatusCode::CONFLICT,
            "This crawl has no native policy ledger",
        ));
    }
    let path = state.data_dir.join(format!("{id}.sqlite"));
    if !tokio::fs::try_exists(&path)
        .await
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Native policy SQLite rows are not available",
        ));
    }
    let (sender, receiver) = tokio::sync::mpsc::channel(8);
    let (ready, ready_result) = tokio::sync::oneshot::channel();
    state.lifecycle.blocking.reap_completed().await;
    let ticket = state
        .lifecycle
        .blocking
        .spawn_blocking(move || {
            let mut ready = Some(ready);
            let result = (|| {
                let store = CrawlStore::open_read_only(path)?;
                let mut writer =
                    std::io::BufWriter::with_capacity(65536, ChannelWriter(sender.clone()));
                policy_export::write_snapshot(&store, format, &mut writer, |summary| {
                    if let Some(ready) = ready.take() {
                        ready
                            .send(Ok(summary.clone()))
                            .map_err(|_| anyhow::anyhow!("Policy export request disconnected"))?;
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
        })
        .map_err(|error| api_error(StatusCode::SERVICE_UNAVAILABLE, error))?;
    // Eager registration starts the bounded producer now. Its actual handles
    // remain owned after this ticket drops and after the Response is returned.
    drop(ticket);
    let summary = ready_result
        .await
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let stream = futures::stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|item| (item, receiver))
    });
    let (content_type, extension) = match format {
        Format::Json => ("application/json; charset=utf-8", "json"),
        Format::Csv => ("text/csv; charset=utf-8", "csv"),
    };
    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{id}-policy.{extension}\""),
        )
        .header("x-policy-snapshot-revision", summary.revision.to_string())
        .header(
            "x-policy-snapshot-occurrences",
            summary.occurrences.to_string(),
        )
        .header("x-policy-sha256", summary.policy_sha256)
        .header("x-policy-ledger-sha256", summary.rolling_sha256)
        .header("x-policy-ledger-complete", summary.complete.to_string())
        .body(Body::from_stream(stream))
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))
}
