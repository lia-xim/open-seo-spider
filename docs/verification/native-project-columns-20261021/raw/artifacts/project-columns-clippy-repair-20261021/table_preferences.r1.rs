//! Explicit native column preferences. Reports, capture generations, queries
//! and browser-global view catalogs are outside this sidecar contract.
use super::{AppState, get_job};
use axum::{
    Json,
    extract::{Path, Request, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::fs::{Dir, OpenOptions};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{File, TryLockError},
    io::{self, Read, Write},
    path::{Component, Path as FsPath, PathBuf},
    time::Duration,
};
use tokio::sync::OwnedSemaphorePermit;
use uuid::Uuid;

const ENGINE: &str = "open-seo-spider/table-column-preferences-v1";
const DIRECTORY: &str = "table-preferences";
const MAX_DOCUMENT_BYTES: usize = 512 * 1024;
pub(super) const MAX_REQUEST_BYTES: usize = MAX_DOCUMENT_BYTES + 4096;
const MAX_REPLY_BYTES: usize = MAX_DOCUMENT_BYTES + 4096;
const MAX_ENTRIES: usize = 64;
const MAX_COLUMNS: usize = 128;

#[derive(Debug)]
pub(super) struct Failure {
    status: StatusCode,
    code: &'static str,
    message: String,
}
type Result<T> = std::result::Result<T, Failure>;
impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        (
            self.status,
            [(header::CACHE_CONTROL, "no-store")],
            Json(serde_json::json!({"code":self.code,"error":self.message})),
        )
            .into_response()
    }
}
fn failure(status: StatusCode, code: &'static str, message: impl Into<String>) -> Failure {
    Failure {
        status,
        code,
        message: message.into(),
    }
}
fn unavailable() -> Failure {
    failure(
        StatusCode::SERVICE_UNAVAILABLE,
        "preferences_unavailable",
        "Native column preferences are not available; saved data was not replaced.",
    )
}
fn invalid_request() -> Failure {
    failure(
        StatusCode::BAD_REQUEST,
        "invalid_preferences_request",
        "Column preference data has an invalid shape or projection.",
    )
}
fn limit() -> Failure {
    failure(
        StatusCode::PAYLOAD_TOO_LARGE,
        "preferences_limit",
        "Column preferences exceed their byte or declaration limit.",
    )
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
enum Fingerprint {
    Missing {},
    Present { revision: Uuid, sha256: String },
    Invalid { sha256: String },
}
impl Fingerprint {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Missing {} => Ok(()),
            Self::Present { sha256, .. } | Self::Invalid { sha256 } if digest(sha256) => Ok(()),
            _ => Err(invalid_request()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ColumnType {
    Text,
    Number,
    Auto,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Column {
    key: String,
    r#type: ColumnType,
    mandatory: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    section: String,
    schema: Vec<Column>,
    visible_keys: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    engine: String,
    version: u8,
    project_id: Uuid,
    revision: Uuid,
    entries: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveRequest {
    expected: Fingerprint,
    entries: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResetRequest {
    expected: Fingerprint,
}
#[derive(Serialize)]
struct Problem {
    code: &'static str,
    message: &'static str,
}
#[derive(Serialize)]
struct Reply {
    project_id: Uuid,
    fingerprint: Fingerprint,
    document: Option<Document>,
    error: Option<Problem>,
}

fn identifier(value: &str, bytes: usize) -> bool {
    !value
        .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
        .is_empty()
        && value.len() <= bytes
        && !value.chars().any(char::is_control)
}
fn validate_entries(entries: &[Entry]) -> Result<()> {
    if entries.len() > MAX_ENTRIES {
        return Err(limit());
    }
    let mut sections = HashSet::new();
    for entry in entries {
        if !identifier(&entry.section, 128)
            || !sections.insert(entry.section.as_str())
            || entry.schema.is_empty()
            || entry.schema.len() > MAX_COLUMNS
            || entry.visible_keys.is_empty()
            || entry.visible_keys.len() > MAX_COLUMNS
        {
            return Err(invalid_request());
        }
        let mut columns = HashSet::new();
        for column in &entry.schema {
            if !identifier(&column.key, 256) || !columns.insert(column.key.as_str()) {
                return Err(invalid_request());
            }
        }
        let mut visible = HashSet::new();
        for key in &entry.visible_keys {
            if !columns.contains(key.as_str()) || !visible.insert(key.as_str()) {
                return Err(invalid_request());
            }
        }
        let mandatory = entry
            .schema
            .iter()
            .filter(|column| column.mandatory)
            .map(|column| &column.key)
            .collect::<Vec<_>>();
        if mandatory.is_empty()
            || entry.visible_keys.len() < mandatory.len()
            || !entry
                .visible_keys
                .iter()
                .take(mandatory.len())
                .eq(mandatory)
        {
            return Err(invalid_request());
        }
    }
    Ok(())
}
fn strict<T: serde::de::DeserializeOwned>(bytes: &[u8], maximum: usize) -> Result<T> {
    open_seo_spider_core::config_profile::parse_strict_json(bytes, maximum)
        .map_err(|_| invalid_request())
}
struct Budget {
    bytes: Vec<u8>,
    cap: usize,
}
impl Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| io::Error::other("Column preference byte limit exceeded"))?;
        if next > self.bytes.capacity() {
            let target = next
                .max(self.bytes.capacity().saturating_mul(2))
                .max(1024)
                .min(self.cap);
            self.bytes
                .try_reserve_exact(target.saturating_sub(self.bytes.len()))
                .map_err(io::Error::other)?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn encode(value: &impl Serialize, cap: usize) -> Result<Vec<u8>> {
    let mut writer = Budget { bytes: vec![], cap };
    serde_json::to_writer(&mut writer, value).map_err(|_| limit())?;
    Ok(writer.bytes)
}

/// Return a capability to the selected directory without following any parent
/// symlink/reparse path. A read of missing preferences never creates files.
fn directory(data: &FsPath, create: bool) -> Result<Option<Dir>> {
    let absolute = if data.is_absolute() {
        data.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|_| unavailable())?
            .join(data)
    };
    let mut anchor = PathBuf::new();
    let mut parts = vec![];
    for component in absolute.components() {
        match component {
            Component::Prefix(part) => anchor.push(part.as_os_str()),
            Component::RootDir => anchor.push(component.as_os_str()),
            Component::Normal(part) => parts.push(part.to_owned()),
            Component::CurDir => (),
            Component::ParentDir => return Err(unavailable()),
        }
    }
    let mut dir =
        Dir::open_ambient_dir(anchor, cap_std::ambient_authority()).map_err(|_| unavailable())?;
    for part in parts {
        dir = dir.open_dir_nofollow(part).map_err(|_| unavailable())?;
    }
    match dir.open_dir_nofollow(DIRECTORY) {
        Ok(dir) => Ok(Some(dir)),
        Err(error) if error.kind() == io::ErrorKind::NotFound && !create => Ok(None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            match dir.create_dir(DIRECTORY) {
                Ok(()) => (),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
                Err(_) => return Err(unavailable()),
            }
            Ok(Some(
                dir.open_dir_nofollow(DIRECTORY)
                    .map_err(|_| unavailable())?,
            ))
        }
        Err(_) => Err(unavailable()),
    }
}
fn filename(id: Uuid) -> String {
    format!("{id}.json")
}
fn read_snapshot(dir: Option<&Dir>, id: Uuid) -> Result<Reply> {
    let missing = || Reply {
        project_id: id,
        fingerprint: Fingerprint::Missing {},
        document: None,
        error: None,
    };
    let Some(dir) = dir else {
        return Ok(missing());
    };
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No).nonblock(true);
    let mut file = match dir.open_with(filename(id), &options) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(missing()),
        Err(_) => return Err(unavailable()),
    };
    let metadata = file.metadata().map_err(|_| unavailable())?;
    if !metadata.is_file() {
        return Err(unavailable());
    }
    if metadata.len() > MAX_DOCUMENT_BYTES as u64 {
        return Err(failure(
            StatusCode::PAYLOAD_TOO_LARGE,
            "stored_preferences_too_large",
            "Saved preferences exceed the bounded reader; no full-file fingerprint or reset token is available.",
        ));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(metadata.len() as usize)
        .map_err(|_| unavailable())?;
    Read::by_ref(&mut file)
        .take(MAX_DOCUMENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable())?;
    if bytes.len() > MAX_DOCUMENT_BYTES
        || bytes.len() as u64 != metadata.len()
        || file.metadata().map_err(|_| unavailable())?.len() != metadata.len()
    {
        return Err(unavailable());
    }
    let hash = sha(&bytes);
    let parsed = strict::<Document>(&bytes, MAX_DOCUMENT_BYTES).and_then(|document| {
        if document.engine != ENGINE || document.version != 1 || document.project_id != id {
            return Err(invalid_request());
        }
        validate_entries(&document.entries)?;
        Ok(document)
    });
    match parsed {
        Ok(document) => Ok(Reply {
            project_id: id,
            fingerprint: Fingerprint::Present {
                revision: document.revision,
                sha256: hash,
            },
            document: Some(document),
            error: None,
        }),
        Err(_) => Ok(Reply {
            project_id: id,
            fingerprint: Fingerprint::Invalid { sha256: hash },
            document: None,
            error: Some(Problem {
                code: "stored_document_invalid",
                message: "Saved column preferences are invalid. Their bytes remain unchanged; reset explicitly to recover.",
            }),
        }),
    }
}
fn load(data: &FsPath, id: Uuid) -> Result<Vec<u8>> {
    let dir = directory(data, false)?;
    encode(&read_snapshot(dir.as_ref(), id)?, MAX_REPLY_BYTES)
}

/// Stable lock inode: never remove it after reset or successful publication.
/// This coordinates lock-honouring processes, not arbitrary external writers.
fn project_lock(dir: &Dir, id: Uuid) -> Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .follow(FollowSymlinks::No)
        .nonblock(true);
    let file = dir
        .open_with(format!("{id}.lock"), &options)
        .map_err(|_| unavailable())?
        .into_std();
    if !file.metadata().map_err(|_| unavailable())?.is_file() {
        return Err(unavailable());
    }
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(failure(
            StatusCode::CONFLICT,
            "preferences_busy",
            "Another process is saving these project columns; try explicitly again.",
        )),
        Err(TryLockError::Error(_)) => Err(unavailable()),
    }
}
fn publish(dir: &Dir, id: Uuid, bytes: &[u8]) -> Result<()> {
    publish_named(
        dir,
        &filename(id),
        &format!(".{id}.{}.tmp", Uuid::new_v4()),
        bytes,
    )
}
fn publish_named(dir: &Dir, target: &str, temporary: &str, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    // If create_new fails this is not our temp, and must never be removed.
    let mut file = dir
        .open_with(temporary, &options)
        .map_err(|_| unavailable())?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    if written.is_err() || dir.rename(temporary, dir, target).is_err() {
        if dir.remove_file(temporary).is_err() {
            return Err(failure(
                StatusCode::SERVICE_UNAVAILABLE,
                "preferences_unavailable",
                "Publication failed and its temporary file could not be removed; saved preferences were not replaced.",
            ));
        }
        return Err(unavailable());
    }
    Ok(())
}
fn mutate(
    data: &FsPath,
    id: Uuid,
    expected: Fingerprint,
    entries: Vec<Entry>,
    resetting: bool,
) -> Result<Vec<u8>> {
    expected.validate()?;
    validate_entries(&entries)?;
    let document = Document {
        engine: ENGINE.into(),
        version: 1,
        project_id: id,
        revision: Uuid::new_v4(),
        entries,
    };
    let bytes = encode(&document, MAX_DOCUMENT_BYTES)?;
    let next = Reply {
        project_id: id,
        fingerprint: Fingerprint::Present {
            revision: document.revision,
            sha256: sha(&bytes),
        },
        document: Some(document),
        error: None,
    };
    let reply = encode(&next, MAX_REPLY_BYTES)?;
    let dir = directory(data, true)?.ok_or_else(unavailable)?;
    let _lock = project_lock(&dir, id)?;
    let current = read_snapshot(Some(&dir), id)?;
    if current.fingerprint != expected {
        return Err(failure(
            StatusCode::CONFLICT,
            "stale_preferences",
            "Saved project columns changed. Load them explicitly before saving again.",
        ));
    }
    if !resetting && matches!(current.fingerprint, Fingerprint::Invalid { .. }) {
        return Err(failure(
            StatusCode::CONFLICT,
            "invalid_saved_preferences",
            "Invalid saved columns require an explicit reset.",
        ));
    }
    publish(&dir, id, &bytes)?;
    Ok(reply)
}

fn slot(state: &AppState) -> Result<OwnedSemaphorePermit> {
    state
        .lifecycle
        .table_preferences_gate
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            failure(
                StatusCode::SERVICE_UNAVAILABLE,
                "preferences_unavailable",
                "Column preference preparation is busy; try explicitly again.",
            )
        })
}
async fn owned<F>(state: &AppState, permit: OwnedSemaphorePermit, operation: F) -> Result<Vec<u8>>
where
    F: FnOnce() -> Result<Vec<u8>> + Send + 'static,
{
    state
        .lifecycle
        .blocking
        .run_blocking(move || {
            let _permit = permit;
            operation()
        })
        .await
        .map_err(|_| unavailable())?
}
async fn body(request: Request) -> Result<Vec<u8>> {
    let json = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"));
    if !json {
        return Err(failure(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "json_required",
            "Column preference mutations require application/json.",
        ));
    }
    if request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|value| value > MAX_REQUEST_BYTES as u64)
    {
        return Err(limit());
    }
    tokio::time::timeout(Duration::from_secs(30), async move {
        let mut stream = request.into_body().into_data_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| invalid_request())?;
            let next = bytes
                .len()
                .checked_add(chunk.len())
                .filter(|length| *length <= MAX_REQUEST_BYTES)
                .ok_or_else(limit)?;
            if next > bytes.capacity() {
                let target = next
                    .max(bytes.capacity().saturating_mul(2))
                    .max(1024)
                    .min(MAX_REQUEST_BYTES);
                bytes
                    .try_reserve_exact(target.saturating_sub(bytes.len()))
                    .map_err(|_| unavailable())?;
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    })
    .await
    .map_err(|_| {
        failure(
            StatusCode::REQUEST_TIMEOUT,
            "preferences_unavailable",
            "Column preference request body timed out.",
        )
    })?
}
async fn project(state: &AppState, id: Uuid) -> Result<()> {
    get_job(state, id)
        .await
        .map(|_| ())
        .map_err(|_| failure(StatusCode::NOT_FOUND, "crawl_not_found", "Crawl not found."))
}
fn response(bytes: Vec<u8>) -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response()
}
pub(super) async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Response> {
    project(&state, id).await?;
    let permit = slot(&state)?;
    let data = state.data_dir.clone();
    Ok(response(
        owned(&state, permit, move || load(&data, id)).await?,
    ))
}
pub(super) async fn put(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Request,
) -> Result<Response> {
    project(&state, id).await?;
    let permit = slot(&state)?;
    let bytes = body(request).await?;
    let data = state.data_dir.clone();
    Ok(response(
        owned(&state, permit, move || {
            let request: SaveRequest = strict(&bytes, MAX_REQUEST_BYTES)?;
            mutate(&data, id, request.expected, request.entries, false)
        })
        .await?,
    ))
}
pub(super) async fn reset(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    request: Request,
) -> Result<Response> {
    project(&state, id).await?;
    let permit = slot(&state)?;
    let bytes = body(request).await?;
    let data = state.data_dir.clone();
    Ok(response(
        owned(&state, permit, move || {
            let request: ResetRequest = strict(&bytes, MAX_REQUEST_BYTES)?;
            mutate(&data, id, request.expected, vec![], true)
        })
        .await?,
    ))
}

#[cfg(test)]
#[path = "table_preferences_tests.rs"]
mod tests;
