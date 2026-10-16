//! Explicit stable-snapshot saved-text analysis; no background worker or website fetch.
use super::*;
use crate::spelling_app;
use open_seo_spider_core::spelling as domain;

fn root(state: &AppState) -> std::path::PathBuf {
    state.data_dir.join("spelling")
}
fn bad(error: impl std::fmt::Display) -> (StatusCode, Json<ApiError>) {
    api_error(StatusCode::BAD_REQUEST, error.to_string())
}
async fn dictionary(state: &AppState) -> anyhow::Result<domain::Dictionary> {
    let d = spelling_app::read_json(&root(state).join("dictionary.json"), 1024 * 1024)
        .await?
        .unwrap_or_default();
    domain::Dictionary::validate(&d).map_err(anyhow::Error::msg)?;
    Ok(d)
}
pub(super) async fn get_dictionary(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let _guard = state.lifecycle.spelling_gate.lock().await;
    spelling_app::dictionary_view(&dictionary(&state).await.map_err(bad)?)
        .map(Json)
        .map_err(bad)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DictionaryUpdate {
    version: u32,
    expected_sha256: String,
    dictionary: domain::Dictionary,
}
pub(super) async fn put_dictionary(
    State(state): State<AppState>,
    Json(request): Json<DictionaryUpdate>,
) -> ApiResult<Json<Value>> {
    if request.version != 1 {
        return Err(bad("Unsupported dictionary update version"));
    }
    request.dictionary.validate().map_err(bad)?;
    let _guard = state.lifecycle.spelling_gate.lock().await;
    let old = dictionary(&state).await.map_err(bad)?;
    if domain::hash(&old).map_err(bad)? != request.expected_sha256 {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Dictionary revision changed; reload before saving",
        ));
    }
    spelling_app::save_json(
        &root(&state).join("dictionary.json"),
        &request.dictionary,
        true,
    )
    .await
    .map_err(bad)?;
    spelling_app::dictionary_view(&request.dictionary)
        .map(Json)
        .map_err(bad)
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct Profile {
    #[serde(default)]
    config: domain::Config,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProfileUpdate {
    version: u32,
    expected_sha256: String,
    profile: Profile,
}
fn profile_path(state: &AppState, id: Uuid) -> std::path::PathBuf {
    root(state).join(format!("{id}-profile.json"))
}
async fn load_profile(state: &AppState, id: Uuid) -> anyhow::Result<Profile> {
    let p: Profile = spelling_app::read_json(&profile_path(state, id), 128 * 1024)
        .await?
        .unwrap_or_default();
    p.config.validate().map_err(anyhow::Error::msg)?;
    Ok(p)
}
pub(super) async fn get_profile(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    get_job(&state, id).await?;
    let _guard = state.lifecycle.spelling_gate.lock().await;
    let p = load_profile(&state, id).await.map_err(bad)?;
    Ok(Json(
        json!({"profile":p,"sha256":domain::hash(&p).map_err(bad)?,"scope":"crawl_spelling_preferences","dictionary_scope":"separate_application_dictionary"}),
    ))
}
pub(super) async fn put_profile(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<ProfileUpdate>,
) -> ApiResult<Json<Value>> {
    get_job(&state, id).await?;
    if request.version != 1 {
        return Err(bad("Unsupported profile update version"));
    }
    request.profile.config.validate().map_err(bad)?;
    let _guard = state.lifecycle.spelling_gate.lock().await;
    let p = load_profile(&state, id).await.map_err(bad)?;
    if domain::hash(&p).map_err(bad)? != request.expected_sha256 {
        return Err(api_error(
            StatusCode::CONFLICT,
            "Profile changed; reload before saving",
        ));
    }
    spelling_app::save_json(&profile_path(&state, id), &request.profile, true)
        .await
        .map_err(bad)?;
    Ok(Json(
        json!({"profile":request.profile,"sha256":domain::hash(&request.profile).map_err(bad)?,"scope":"crawl_spelling_preferences"}),
    ))
}
fn run_path(state: &AppState, id: Uuid, run: Uuid) -> std::path::PathBuf {
    root(state)
        .join(id.to_string())
        .join("runs")
        .join(format!("{run}.json"))
}
async fn revalidate(
    state: &AppState,
    id: Uuid,
    report: &mut spelling_app::Report,
) -> anyhow::Result<()> {
    let job = get_job(state, id)
        .await
        .map_err(|_| anyhow::anyhow!("Saved job unavailable"))?;
    let guard = job.read().await;
    let current = content::stable_report(&guard).and_then(|r| content::context(state, &guard, r));
    let saved = &report.saved_capture["snapshot"];
    let snapshot_matches = current.is_ok_and(|c| {
        saved["sha256"].as_str() == Some(&c.sha256)
            && saved["generation"] == json!(c.generation)
            && saved["origin"] == c.origin
    });
    let dictionary_matches = domain::hash(&dictionary(state).await?).map_err(anyhow::Error::msg)?
        == report.dictionary_sha256;
    report.binding_current = snapshot_matches && dictionary_matches;
    report.binding_qualification=if report.binding_current {
        "Snapshot/generation and application dictionary match at this response; immutable run retains its exact request/config/dictionary"
    } else {"Historical analysis: snapshot/generation or application dictionary changed/unavailable; recheck saved text for a current view"}.into();
    if !report.binding_current {
        report.complete = false;
        report.accepted_count = None;
    }
    Ok(())
}
pub(super) async fn analyze(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<spelling_app::Request>,
) -> ApiResult<Json<Value>> {
    request.validate().map_err(bad)?;
    let job = get_job(&state, id).await?;
    let prepared = {
        let _dictionary_guard = state.lifecycle.spelling_gate.lock().await;
        let d = dictionary(&state).await.map_err(bad)?;
        let guard = job.read().await;
        let report = content::stable_report(&guard)?;
        let context = content::context(&state, &guard, report)?;
        spelling_app::prepare(report, &context, request, d).map_err(bad)?
    };
    // Await in the existing owned HTTP request task, bounded90s. No detached service job.
    let mut report = spelling_app::run(prepared).await.map_err(bad)?;
    report.crawl_id = Some(id);
    let _guard = state.lifecycle.spelling_gate.lock().await;
    revalidate(&state, id, &mut report).await.map_err(bad)?;
    spelling_app::validate_report(&report, None).map_err(bad)?;
    spelling_app::save_json(&run_path(&state, id, report.run_id), &report, false)
        .await
        .map_err(bad)?;
    Ok(Json(
        json!({"report":report,"persisted":true,"crawl_id":id}),
    ))
}
pub(super) async fn result(
    State(state): State<AppState>,
    Path((id, run)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Value>> {
    get_job(&state, id).await?;
    let _guard = state.lifecycle.spelling_gate.lock().await;
    let mut report: spelling_app::Report =
        spelling_app::read_json(&run_path(&state, id, run), spelling_app::STORE_BYTES)
            .await
            .map_err(bad)?
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Spelling run not found"))?;
    if report.run_id != run || report.crawl_id != Some(id) {
        return Err(bad("Spelling run identity mismatch"));
    }
    revalidate(&state, id, &mut report).await.map_err(bad)?;
    spelling_app::validate_report(&report, None).map_err(bad)?;
    Ok(Json(
        json!({"report":report,"persisted":true,"crawl_id":id}),
    ))
}
pub(super) async fn csv(
    State(state): State<AppState>,
    Path((id, run)): Path<(Uuid, Uuid)>,
) -> ApiResult<impl IntoResponse> {
    get_job(&state, id).await?;
    let _guard = state.lifecycle.spelling_gate.lock().await;
    let mut report: spelling_app::Report =
        spelling_app::read_json(&run_path(&state, id, run), spelling_app::STORE_BYTES)
            .await
            .map_err(bad)?
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Spelling run not found"))?;
    if report.run_id != run || report.crawl_id != Some(id) {
        return Err(bad("Spelling run identity mismatch"));
    }
    revalidate(&state, id, &mut report).await.map_err(bad)?;
    spelling_app::validate_report(&report, None).map_err(bad)?;
    let bytes = spelling_app::csv(&report).map_err(bad)?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"spelling-{run}.csv\""),
            ),
            (header::CONTENT_LENGTH, bytes.len().to_string()),
            (
                "x-content-sha256"
                    .parse::<header::HeaderName>()
                    .expect("static header"),
                domain::digest(bytes.as_bytes()),
            ),
        ],
        bytes,
    ))
}
