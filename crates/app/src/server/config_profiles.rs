//! Pure profile preparation is separate from explicit crawl launch. Raw JSON
//! travels as text so browser number coercion cannot change native integers.
use super::*;
use axum::{body::Bytes, extract::FromRequest};
use open_seo_spider_core::config_profile as profile;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, time::Duration};
use tokio::sync::OwnedSemaphorePermit;

// A JSON-text envelope may escape every quote/backslash of a 64 MiB profile.
// Two per-process slots bound body preparation and the actual blocking work.
pub(super) const MAX_REQUEST_BYTES: usize = 2 * profile::CONFIG_PROFILE_BYTES + 256 * 1024;
const MAX_REPLY_BYTES: usize = 6 * profile::CONFIG_PROFILE_BYTES + 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigTextRequest {
    config_text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileRequest {
    profile_text: String,
    #[serde(default)]
    patches: Vec<ConfigPatch>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LaunchRequest {
    profile_text: String,
    #[serde(default)]
    patches: Vec<ConfigPatch>,
    #[serde(default)]
    header_values: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigPatch {
    path: Vec<String>,
    #[serde(default)]
    value_json: Option<String>,
    #[serde(default)]
    remove: bool,
}

fn slot(state: &AppState) -> ApiResult<OwnedSemaphorePermit> {
    state
        .lifecycle
        .config_profile_gate
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Configuration profile preparation is busy",
            )
        })
}

async fn body(state: &AppState, request: Request) -> ApiResult<(OwnedSemaphorePermit, Bytes)> {
    let json_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"));
    if !json_type {
        return Err(api_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Configuration profile requests require application/json",
        ));
    }
    let permit = slot(state)?;
    let bytes = tokio::time::timeout(Duration::from_secs(30), Bytes::from_request(request, state))
        .await
        .map_err(|_| {
            api_error(
                StatusCode::REQUEST_TIMEOUT,
                "Configuration profile body timed out",
            )
        })?
        .map_err(|error| api_error(error.status(), error.body_text()))?;
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(api_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Configuration profile envelope exceeds its byte limit",
        ));
    }
    Ok((permit, bytes))
}

async fn owned<T, F>(state: &AppState, permit: OwnedSemaphorePermit, operation: F) -> ApiResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
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
        .map_err(|error| api_error(StatusCode::BAD_REQUEST, error))
}

fn apply_patches(
    mut value: profile::CrawlConfigProfile,
    patches: Vec<ConfigPatch>,
) -> anyhow::Result<profile::CrawlConfigProfile> {
    anyhow::ensure!(
        patches.len() <= 128,
        "At most 128 configuration patches are supported"
    );
    for patch in patches {
        anyhow::ensure!(
            patch.path.len() <= 16,
            "Configuration patch path is too deep"
        );
        anyhow::ensure!(
            patch
                .path
                .iter()
                .all(|key| !key.is_empty() && key.len() <= 256),
            "Invalid configuration patch key"
        );
        anyhow::ensure!(
            patch.remove != patch.value_json.is_some(),
            "A patch must either set one JSON value or remove one key"
        );
        let replacement = patch
            .value_json
            .map(|raw| {
                profile::parse_strict_json::<Value>(raw.as_bytes(), profile::CONFIG_PROFILE_BYTES)
            })
            .transpose()?;
        if patch.path.is_empty() {
            let replacement = replacement
                .ok_or_else(|| anyhow::anyhow!("Cannot remove the complete configuration"))?;
            anyhow::ensure!(
                replacement.is_object(),
                "Complete configuration must be an object"
            );
            value.config = replacement;
            continue;
        }
        let mut parent = &mut value.config;
        for key in &patch.path[..patch.path.len() - 1] {
            parent = parent
                .as_object_mut()
                .and_then(|object| object.get_mut(key))
                .ok_or_else(|| {
                    anyhow::anyhow!("Configuration patch parent is not an existing object")
                })?;
        }
        let object = parent
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("Configuration patch parent must be an object"))?;
        let key = patch.path.last().unwrap();
        if let Some(replacement) = replacement {
            object.insert(key.clone(), replacement);
        } else {
            anyhow::ensure!(
                object.remove(key).is_some(),
                "Configuration patch key does not exist"
            );
        }
    }
    // Re-enter the same strict typed parser after patches; permissive outer
    // CrawlConfig deserialization must never hide an unknown patched setting.
    profile::parse_config_profile(&serde_json::to_vec(&value)?)
}

fn edited(request: ProfileRequest) -> anyhow::Result<profile::CrawlConfigProfile> {
    apply_patches(
        profile::parse_config_profile(request.profile_text.as_bytes())?,
        request.patches,
    )
}

fn json_reply(value: Value) -> anyhow::Result<Response> {
    let bytes = serde_json::to_vec(&value)?;
    anyhow::ensure!(
        bytes.len() <= MAX_REPLY_BYTES,
        "Configuration profile preview exceeds its byte limit"
    );
    Ok((
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response())
}

fn preview(value: profile::CrawlConfigProfile) -> anyhow::Result<Response> {
    let inspected = profile::preview_config_profile(&value)?;
    let profile_json = String::from_utf8(profile::export_config_profile(&value)?)?;
    let config_json = serde_json::to_string(&value.config)?;
    let effective_config_json = serde_json::to_string(&inspected.effective_config)?;
    // Default expansion can make the effective document larger than its input.
    // Every document has the same explicit bound enforced by the browser.
    anyhow::ensure!(
        [&profile_json, &config_json, &effective_config_json]
            .iter()
            .all(|text| text.len() <= profile::CONFIG_PROFILE_BYTES),
        "A configuration profile document exceeds64MiB after normalization"
    );
    json_reply(json!({
        "engine":profile::CONFIG_PROFILE_ENGINE,
        "version":1,
        "profile_json":profile_json,
        "config_json":config_json,
        "effective_config_json":effective_config_json,
        "normalization":inspected.normalization,
        "missing_headers":inspected.missing_headers,
        "requires_targets":inspected.requires_targets,
        "max_bytes":profile::CONFIG_PROFILE_BYTES
    }))
}

pub(super) async fn defaults(State(state): State<AppState>) -> ApiResult<Response> {
    let permit = slot(&state)?;
    owned(&state, permit, || {
        preview(profile::config_profile_from_config(&CrawlConfig::default())?)
    })
    .await
}

pub(super) async fn from_config(
    State(state): State<AppState>,
    request: Request,
) -> ApiResult<Response> {
    let (permit, bytes) = body(&state, request).await?;
    owned(&state, permit, move || {
        let request: ConfigTextRequest = profile::parse_strict_json(&bytes, MAX_REQUEST_BYTES)?;
        preview(profile::config_profile_from_json(
            request.config_text.as_bytes(),
        )?)
    })
    .await
}

pub(super) async fn selected(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let permit = slot(&state)?;
    let guard = get_job(&state, id).await?.read_owned().await;
    owned(&state, permit, move || {
        preview(profile::config_profile_from_config(&guard.config)?)
    })
    .await
}

pub(super) async fn inspect(
    State(state): State<AppState>,
    request: Request,
) -> ApiResult<Response> {
    let (permit, bytes) = body(&state, request).await?;
    owned(&state, permit, move || {
        let request: ProfileRequest = profile::parse_strict_json(&bytes, MAX_REQUEST_BYTES)?;
        preview(edited(request)?)
    })
    .await
}

pub(super) async fn export(State(state): State<AppState>, request: Request) -> ApiResult<Response> {
    let (permit, bytes) = body(&state, request).await?;
    owned(&state, permit, move || {
        let request: ProfileRequest = profile::parse_strict_json(&bytes, MAX_REQUEST_BYTES)?;
        let bytes = profile::export_config_profile(&edited(request)?)?;
        let length = bytes.len();
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let mut response = (
            [
                (header::CONTENT_TYPE, "application/json"),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=\"open-seo-spider-profile.json\"",
                ),
                (header::CACHE_CONTROL, "no-store"),
            ],
            bytes,
        )
            .into_response();
        response
            .headers_mut()
            .insert(header::CONTENT_LENGTH, length.into());
        response
            .headers_mut()
            .insert("x-content-sha256", digest.parse()?);
        Ok(response)
    })
    .await
}

pub(super) async fn create(
    State(state): State<AppState>,
    request: Request,
) -> ApiResult<Json<Value>> {
    let (permit, bytes) = body(&state, request).await?;
    lifecycle::start_operation(state.clone(), prepare_launch(state, permit, bytes)).await
}

async fn prepare_launch(
    state: AppState,
    permit: OwnedSemaphorePermit,
    bytes: Bytes,
) -> ApiResult<lifecycle::PreparedLaunch> {
    let config = owned(&state, permit, move || {
        let request: LaunchRequest = profile::parse_strict_json(&bytes, MAX_REQUEST_BYTES)?;
        let value = apply_patches(
            profile::parse_config_profile(request.profile_text.as_bytes())?,
            request.patches,
        )?;
        let config = profile::resolve_config_profile(&value, &request.header_values)?;
        // Only explicit Start constructs clients and requires target URLs.
        let crawler = Crawler::new(config)?;
        crawler
            .effective_config()
            .validate_input_provenance_budget()?;
        Ok(crawler.effective_config().clone())
    })
    .await?;
    create_crawl_with_config_prepared(state, config).await
}

#[cfg(test)]
#[path = "config_profiles_tests.rs"]
mod tests;
