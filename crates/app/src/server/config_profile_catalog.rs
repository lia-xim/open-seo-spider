//! Named portable documents in the selected application's data directory.
use super::*;
use open_seo_spider_core::{
    CONFIG_PROFILE_CATALOG_ENGINE, CONFIG_PROFILE_CATALOG_ENTRIES,
    CONFIG_PROFILE_CATALOG_NAME_BYTES, CONFIG_PROFILE_CATALOG_TOTAL_BYTES, ConfigProfileCatalog,
    ConfigProfileCatalogError, ConfigProfileCatalogErrorKind, config_profile as profile,
};
use std::io::{self, Write};
use tokio::sync::OwnedSemaphorePermit;

const MAX_REPLY_BYTES: usize = config_profiles::MAX_REPLY_BYTES + 64 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateRequest {
    name: String,
    profile_text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplaceRequest {
    expected_revision: String,
    expected_sha256: String,
    name: String,
    profile_text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameRequest {
    expected_revision: String,
    expected_sha256: String,
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteRequest {
    expected_revision: String,
    expected_sha256: String,
}

fn catalog(state: &AppState) -> ConfigProfileCatalog {
    ConfigProfileCatalog::new(
        state
            .data_dir
            .join("configuration-profiles")
            .join("catalog.sqlite"),
    )
}
fn store_error(error: ConfigProfileCatalogError) -> (StatusCode, Json<ApiError>) {
    use ConfigProfileCatalogErrorKind::*;
    let status = match error.kind {
        Validation => StatusCode::BAD_REQUEST,
        Missing => StatusCode::NOT_FOUND,
        Conflict => StatusCode::CONFLICT,
        Limit => StatusCode::PAYLOAD_TOO_LARGE,
        Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    };
    api_error(status, error)
}
async fn owned<T, F>(state: &AppState, permit: OwnedSemaphorePermit, operation: F) -> ApiResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> ApiResult<T> + Send + 'static,
{
    state
        .lifecycle
        .blocking
        .run_blocking(move || {
            let _permit = permit;
            operation()
        })
        .await
        .map_err(|error| api_error(StatusCode::SERVICE_UNAVAILABLE, error))?
}

/// Counting and encoding share the same logical cap; refusal is atomic, never a
/// truncated JSON success. The count-only preview runs BEFORE catalog writes.
struct Budget {
    bytes: Option<Vec<u8>>,
    length: usize,
    cap: usize,
}
impl Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .length
            .checked_add(bytes.len())
            .filter(|&n| n <= self.cap)
            .ok_or_else(|| {
                io::Error::other("Configuration profile catalog reply exceeds its byte limit")
            })?;
        if let Some(output) = &mut self.bytes {
            if next > output.capacity() {
                let capacity = next.max(output.capacity().saturating_mul(2)).min(self.cap);
                output
                    .try_reserve_exact(capacity.saturating_sub(output.len()))
                    .map_err(io::Error::other)?;
            }
            output.extend_from_slice(bytes);
        }
        self.length = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn measure(value: &Value, cap: usize) -> ApiResult<()> {
    serde_json::to_writer(
        &mut Budget {
            bytes: None,
            length: 0,
            cap,
        },
        value,
    )
    .map_err(|error| api_error(StatusCode::PAYLOAD_TOO_LARGE, error))
}
fn reply(value: Value) -> ApiResult<Response> {
    let mut output = Budget {
        bytes: Some(Vec::new()),
        length: 0,
        cap: MAX_REPLY_BYTES,
    };
    serde_json::to_writer(&mut output, &value)
        .map_err(|error| api_error(StatusCode::PAYLOAD_TOO_LARGE, error))?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        output.bytes.unwrap(),
    )
        .into_response())
}
fn prepare(text: &str) -> ApiResult<Value> {
    if text.len() > profile::CONFIG_PROFILE_BYTES {
        return Err(api_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Configuration profile exceeds64MiB",
        ));
    }
    let parsed = profile::parse_config_profile(text.as_bytes())
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
    let preview = config_profiles::preview_value(parsed)
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
    measure(&preview, config_profiles::MAX_REPLY_BYTES)?;
    Ok(preview)
}

pub(super) async fn list(State(state): State<AppState>) -> ApiResult<Response> {
    let permit = config_profiles::slot(&state)?;
    let store = catalog(&state);
    owned(&state,permit,move || reply(json!({
        "engine":CONFIG_PROFILE_CATALOG_ENGINE,"version":1,
        "scope":"selected_application_data_directory",
        "entries":store.list().map_err(store_error)?,
        "limits":{"max_entries":CONFIG_PROFILE_CATALOG_ENTRIES,"max_name_bytes":CONFIG_PROFILE_CATALOG_NAME_BYTES,
            "max_profile_bytes":profile::CONFIG_PROFILE_BYTES,"max_total_profile_bytes":CONFIG_PROFILE_CATALOG_TOTAL_BYTES}
    }))).await
}
pub(super) async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let permit = config_profiles::slot(&state)?;
    let store = catalog(&state);
    owned(&state, permit, move || {
        let document = store.get(&id.to_string()).map_err(store_error)?;
        let preview = prepare(&document.profile_text)
            .map_err(|error| api_error(StatusCode::SERVICE_UNAVAILABLE, error.1.0.error))?;
        reply(json!({"entry":document.entry,"profile":preview}))
    })
    .await
}
pub(super) async fn create(State(state): State<AppState>, request: Request) -> ApiResult<Response> {
    let (permit, bytes) = config_profiles::body(&state, request).await?;
    let store = catalog(&state);
    owned(&state, permit, move || {
        let request: CreateRequest =
            profile::parse_strict_json(&bytes, config_profiles::MAX_REQUEST_BYTES)
                .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
        let _preview = prepare(&request.profile_text)?;
        let entry = store
            .create(
                &Uuid::new_v4().to_string(),
                &request.name,
                &request.profile_text,
            )
            .map_err(store_error)?;
        reply(json!({"entry":entry}))
    })
    .await
}
pub(super) async fn replace(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Request,
) -> ApiResult<Response> {
    let (permit, bytes) = config_profiles::body(&state, request).await?;
    let store = catalog(&state);
    owned(&state, permit, move || {
        let request: ReplaceRequest =
            profile::parse_strict_json(&bytes, config_profiles::MAX_REQUEST_BYTES)
                .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
        let _preview = prepare(&request.profile_text)?;
        let entry = store
            .replace(
                &id.to_string(),
                &request.expected_revision,
                &request.expected_sha256,
                &request.name,
                &request.profile_text,
            )
            .map_err(store_error)?;
        reply(json!({"entry":entry}))
    })
    .await
}
pub(super) async fn rename(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Request,
) -> ApiResult<Response> {
    let (permit, bytes) = config_profiles::body(&state, request).await?;
    let store = catalog(&state);
    owned(&state, permit, move || {
        let request: RenameRequest =
            profile::parse_strict_json(&bytes, config_profiles::MAX_REQUEST_BYTES)
                .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
        let entry = store
            .rename(
                &id.to_string(),
                &request.expected_revision,
                &request.expected_sha256,
                &request.name,
            )
            .map_err(store_error)?;
        reply(json!({"entry":entry}))
    })
    .await
}
pub(super) async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Request,
) -> ApiResult<Response> {
    let (permit, bytes) = config_profiles::body(&state, request).await?;
    let store = catalog(&state);
    owned(&state, permit, move || {
        let request: DeleteRequest =
            profile::parse_strict_json(&bytes, config_profiles::MAX_REQUEST_BYTES)
                .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))?;
        store
            .delete(
                &id.to_string(),
                &request.expected_revision,
                &request.expected_sha256,
            )
            .map_err(store_error)?;
        reply(json!({"deleted_id":id.to_string()}))
    })
    .await
}

#[cfg(test)]
#[path = "config_profile_catalog_tests.rs"]
mod tests;
