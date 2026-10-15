//! Saved App-declared reports only. No filesystem path or uploaded PageRecord authority.
use super::*;
use axum::extract::rejection::JsonRejection;
use open_seo_spider_core::{
    ComparisonSide, DIRECTORY_TREE_CSV_BYTES, DIRECTORY_TREE_ENGINE, DIRECTORY_TREE_JSON_BYTES,
    DirectoryTree, DirectoryTreeLimits, build_directory_tree, directory_tree_csv,
    directory_tree_json,
};
use sha2::{Digest, Sha256};
use std::io::{self, Write};
pub(super) const REQUEST_BYTES: usize = 8192;
const REPORT_BYTES: usize = 64 * 1024 * 1024;
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Failure {
    pub code: String,
    pub message: String,
}
type DResult<T> = Result<T, (StatusCode, Json<Failure>)>;
fn failure(status: StatusCode, code: &str, message: impl ToString) -> (StatusCode, Json<Failure>) {
    (
        status,
        Json(Failure {
            code: code.into(),
            message: message.to_string(),
        }),
    )
}
fn default_limit() -> usize {
    100
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TreeRequest {
    #[serde(default)]
    pub expected_generation: Option<u64>,
    #[serde(default)]
    pub expected_snapshot_token: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
}
impl Default for TreeRequest {
    fn default() -> Self {
        Self {
            expected_generation: None,
            expected_snapshot_token: None,
            parent_id: None,
            offset: 0,
            limit: 100,
        }
    }
}
impl TreeRequest {
    fn validate(&self) -> DResult<()> {
        if self.limit == 0
            || self.limit > 200
            || self.offset > DirectoryTreeLimits::default().max_nodes
        {
            return Err(failure(
                StatusCode::BAD_REQUEST,
                "invalid_directory_window",
                "Use limit1..200 and a bounded nonnegative offset",
            ));
        }
        for value in [&self.expected_snapshot_token, &self.parent_id]
            .into_iter()
            .flatten()
        {
            if value.len() != 64
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(failure(
                    StatusCode::BAD_REQUEST,
                    "invalid_directory_token",
                    "Node IDs and snapshot tokens must be64 lowercase hex characters",
                ));
            }
        }
        if (self.parent_id.is_some() || self.offset > 0) && self.expected_snapshot_token.is_none() {
            return Err(failure(
                StatusCode::BAD_REQUEST,
                "directory_snapshot_required",
                "Navigation and paging require the actual preview snapshot token",
            ));
        }
        Ok(())
    }
}
#[derive(Serialize)]
struct Snapshot {
    job_id: Uuid,
    app_generation: u64,
    status: String,
    typed_report_sha256: String,
    typed_config_sha256: String,
    caller_binding: open_seo_spider_core::ComparisonCallerBinding,
    sql_attested: bool,
    report_generation_attested: bool,
}
struct HashWriter {
    hash: Sha256,
    bytes: usize,
}
impl Write for HashWriter {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if b.len() > REPORT_BYTES.saturating_sub(self.bytes) {
            return Err(io::Error::other(
                "Directory saved report/config exceeds64MiB",
            ));
        }
        self.hash.update(b);
        self.bytes += b.len();
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn digest(value: &impl Serialize) -> anyhow::Result<String> {
    let mut w = HashWriter {
        hash: Sha256::new(),
        bytes: 0,
    };
    serde_json::to_writer(&mut w, value)?;
    Ok(format!("{:x}", w.hash.finalize()))
}
struct Evaluation {
    tree: DirectoryTree,
    snapshot: Snapshot,
    token: String,
}
fn evaluate(job: &Job, r: &TreeRequest) -> DResult<Evaluation> {
    let report_hash = if let Some(report) = &job.report {
        if report.pages.len() > DirectoryTreeLimits::default().max_pages {
            return Err(failure(
                StatusCode::BAD_REQUEST,
                "directory_page_budget",
                "Saved page count exceeds directory observation budget",
            ));
        }
        Some(
            digest(report)
                .map_err(|e| failure(StatusCode::BAD_REQUEST, "directory_report_budget", e))?,
        )
    } else {
        None
    };
    let config_hash = digest(&job.config)
        .map_err(|e| failure(StatusCode::BAD_REQUEST, "directory_config_budget", e))?;
    // The existing authority remains unchanged and executes under this same read lock.
    let saved = comparison::stable_saved_report(job, ComparisonSide::Before, r.expected_generation)
        .map_err(|(status, Json(e))| failure(status, &e.error.code, e.error.message))?;
    let snapshot = Snapshot {
        job_id: job.id,
        app_generation: job.generation,
        status: job.status.clone(),
        typed_report_sha256: report_hash.expect("Unchanged saved authority requires a report"),
        typed_config_sha256: config_hash,
        caller_binding: saved.binding,
        sql_attested: false,
        report_generation_attested: false,
    };
    let limits = DirectoryTreeLimits::default();
    let token = digest(&(DIRECTORY_TREE_ENGINE, &snapshot, &limits))
        .map_err(|e| failure(StatusCode::BAD_REQUEST, "directory_token_budget", e))?;
    if r.expected_snapshot_token
        .as_ref()
        .is_some_and(|expected| expected != &token)
    {
        return Err(failure(
            StatusCode::CONFLICT,
            "stale_directory_snapshot",
            "The saved report, App status/generation or configuration changed; start a new preview",
        ));
    }
    let tree = build_directory_tree(&saved.report.pages, &limits)
        .map_err(|e| failure(StatusCode::BAD_REQUEST, "directory_tree_budget", e))?;
    Ok(Evaluation {
        tree,
        snapshot,
        token,
    })
}
async fn saved(state: &AppState, id: Uuid, r: TreeRequest) -> DResult<(Evaluation, TreeRequest)> {
    r.validate()?;
    let job = get_job(state, id)
        .await
        .map_err(|(s, Json(e))| failure(s, "directory_job_not_found", e.error))?;
    let guard = job.read_owned().await;
    // Retain the real worker until join. Its owned read guard protects the original authority,
    // pre-clone byte check, snapshot binding and projection; no async task owns a detached CPU job.
    state
        .lifecycle
        .blocking
        .run_blocking(move || {
            let e = evaluate(&guard, &r)?;
            Ok((e, r))
        })
        .await
        .map_err(|e| {
            failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "directory_worker_failed",
                e,
            )
        })?
}
fn response(bytes: Vec<u8>, mime: &str, download: bool) -> Response {
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let length = bytes.len().to_string();
    let mut out = (
        [
            (header::CONTENT_TYPE, mime.to_owned()),
            (header::CONTENT_LENGTH, length),
            (header::CACHE_CONTROL, "no-store".into()),
            (header::HeaderName::from_static("x-content-sha256"), hash),
        ],
        bytes,
    )
        .into_response();
    if download {
        out.headers_mut().insert(
            header::CONTENT_DISPOSITION,
            header::HeaderValue::from_static("attachment; filename=directory-tree.csv"),
        );
    }
    out
}
pub(super) async fn preview(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    r: Result<Json<TreeRequest>, JsonRejection>,
) -> DResult<Response> {
    let Json(r) = r.map_err(|e| {
        failure(
            StatusCode::BAD_REQUEST,
            "invalid_directory_request",
            e.body_text(),
        )
    })?;
    let (e, r) = saved(&state, id, r).await?;
    let bytes=state.lifecycle.blocking.run_blocking(move||{
        let parent=r.parent_id.as_deref().unwrap_or(&e.tree.root_id);
        let index:std::collections::BTreeMap<_,_>=e.tree.nodes.iter().map(|n|(n.id.as_str(),n)).collect();
        let current=index.get(parent).ok_or_else(||failure(StatusCode::BAD_REQUEST,"directory_parent_missing","Node is absent from this saved tree"))?;
        let mut ancestors=Vec::new();let mut p=current.parent_id.as_deref();
        while let Some(id)=p {let n=index.get(id).expect("Core tree parent");ancestors.push(*n);p=n.parent_id.as_deref();}ancestors.reverse();
        let mut children:Vec<_>=e.tree.nodes.iter().filter(|n|n.parent_id.as_deref()==Some(parent)).collect();children.sort_by(|a,b|a.address.cmp(&b.address).then(a.kind.cmp(&b.kind)));
        if r.offset>children.len(){return Err(failure(StatusCode::BAD_REQUEST,"directory_offset_out_of_range","Offset exceeds the actual child count"));}
        let end=r.offset.saturating_add(r.limit).min(children.len());
        let value=json!({"engine":e.tree.engine,"snapshot":e.snapshot,"snapshot_token":e.token,"root_id":e.tree.root_id,"current":current,"ancestors":ancestors,"children":&children[r.offset..end],"offset":r.offset,"total_children":children.len(),"next_offset":(end<children.len()).then_some(end),"input_observations":e.tree.input_observations,"invalid_url_observations":e.tree.invalid_url_observations,"unsupported_url_observations":e.tree.unsupported_url_observations,"credential_url_observations":e.tree.credential_url_observations,"total_nodes":e.tree.nodes.len(),"website_requests":0,"counts_mean":"Synthetic group counters aggregate descendants; group addresses do not prove a request. Status belongs to requested URL; final response is never substituted. Stored indexability is used only for a direct successful observed document; missing/redirect/error/robots cases remain unknown."});
        directory_tree_json(&value,DIRECTORY_TREE_JSON_BYTES).map_err(|e|failure(StatusCode::BAD_REQUEST,"directory_json_budget",e))
    }).await.map_err(|e|failure(StatusCode::INTERNAL_SERVER_ERROR,"directory_page_worker_failed",e))??;
    Ok(response(bytes, "application/json", false))
}
pub(super) async fn csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    r: Result<Json<TreeRequest>, JsonRejection>,
) -> DResult<Response> {
    let Json(r) = r.map_err(|e| {
        failure(
            StatusCode::BAD_REQUEST,
            "invalid_directory_request",
            e.body_text(),
        )
    })?;
    if r.parent_id.is_some() || r.offset != 0 || r.expected_snapshot_token.is_none() {
        return Err(failure(
            StatusCode::BAD_REQUEST,
            "directory_full_export_requires_snapshot",
            "Full CSV requires a preview snapshot token and no parent/window filter",
        ));
    }
    let (e, _) = saved(&state, id, r).await?;
    let bytes = state
        .lifecycle
        .blocking
        .run_blocking(move || directory_tree_csv(&e.tree, DIRECTORY_TREE_CSV_BYTES))
        .await
        .map_err(|e| {
            failure(
                StatusCode::INTERNAL_SERVER_ERROR,
                "directory_export_worker_failed",
                e,
            )
        })?
        .map_err(|e| failure(StatusCode::BAD_REQUEST, "directory_csv_budget", e))?;
    Ok(response(bytes, "text/csv; charset=utf-8", true))
}
#[cfg(test)]
#[path = "directory_tree_tests.rs"]
mod tests;
