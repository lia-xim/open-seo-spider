//! Opt-in capability-relative MCP text files. No ambient fallback after root open.
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use cap_fs_ext::{DirExt, FollowSymlinks, MetadataExt, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard, Weak,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{Mutex as AsyncMutex, Semaphore, oneshot};
use tokio::task::JoinHandle;

pub(super) const WRITE_TEXT_MAX_BYTES: usize = 1024 * 1024;
pub(super) const READ_CHUNK_MAX_BYTES: usize = 256 * 1024;
pub(super) const READ_FILE_MAX_BYTES: usize = 32 * 1024 * 1024;
pub(super) const DIRECTORY_MAX_ENTRIES: usize = 10_000;
pub(super) const DIRECTORY_NAME_MAX_BYTES: usize = 2 * 1024 * 1024;
pub(super) const MAX_CONCURRENT_FILE_OPS: usize = 4;
pub(super) const MAX_RESULT_JSON_BYTES: usize = 2 * 1024 * 1024;
const PATH_MAX_BYTES: usize = 64 * 1024;
const COMPONENT_MAX_COUNT: usize = 256;
const CURSOR_MAX_BYTES: usize = 1024 * 1024;
const STAGING_PREFIX: &str = ".__open_seo_spider_mcp_tmp_";
const NAMES: [&str; 5] = [
    "spider_list_allowed_directory",
    "spider_list_directory",
    "spider_create_directory",
    "spider_read_text_file",
    "spider_write_text_file",
];

#[derive(Clone)]
pub(super) struct FileTools {
    inner: Arc<Inner>,
}
struct Inner {
    root: Arc<Dir>,
    scope: String,
    permits: Arc<Semaphore>,
    registry: Mutex<Registry>,
}
#[derive(Default)]
struct Registry {
    closing: bool,
    next_id: u64,
    jobs: BTreeMap<u64, Arc<Job>>,
    first_join_failure: Option<String>,
}
struct Job {
    worker: AsyncMutex<Worker>,
    supervisor: Mutex<Option<Arc<Supervisor>>>,
    completed: AtomicBool,
}
struct Worker {
    // Await by mutable reference: dropping any waiter never drops this handle.
    join: Option<JoinHandle<Result<Value>>>,
    reply: Option<oneshot::Sender<Result<Value>>>,
}
struct Supervisor {
    join: AsyncMutex<Option<JoinHandle<()>>>,
}

fn note_join_failure(inner: &Weak<Inner>, message: String) {
    if let Some(inner) = inner.upgrade() {
        lock(&inner.registry)
            .first_join_failure
            .get_or_insert(message);
    }
}
async fn join_worker(inner: &Weak<Inner>, job: &Job) {
    let mut worker = job.worker.lock().await;
    if let Some(handle) = worker.join.as_mut() {
        let result = handle.await;
        worker.join.take();
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                let message = format!("Filesystem blocking worker failed to join: {error}");
                note_join_failure(inner, message.clone());
                Err(anyhow::anyhow!(message))
            }
        };
        if let Some(reply) = worker.reply.take() {
            // A disconnected caller cannot cancel/detach the actual worker.
            let _ = reply.send(result);
        }
        job.completed.store(true, Ordering::Release);
    }
}
async fn finish_job(inner: &Arc<Inner>, id: u64, job: &Arc<Job>) {
    join_worker(&Arc::downgrade(inner), job).await;
    let supervisor = lock(&job.supervisor).clone();
    if let Some(supervisor) = supervisor {
        let mut supervisor_join = supervisor.join.lock().await;
        if let Some(handle) = supervisor_join.as_mut() {
            let result = handle.await;
            supervisor_join.take();
            if let Err(error) = result {
                note_join_failure(
                    &Arc::downgrade(inner),
                    format!("Filesystem supervisor failed to join: {error}"),
                );
            }
        }
    }
    // Both actual handles have now been joined, also when another waiter canceled.
    lock(&inner.registry).jobs.remove(&id);
}

impl FileTools {
    pub(super) fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Inner {
                root: Arc::new(open_root(path)?),
                scope: uuid::Uuid::new_v4().to_string(),
                permits: Arc::new(Semaphore::new(MAX_CONCURRENT_FILE_OPS)),
                registry: Mutex::new(Registry::default()),
            }),
        })
    }
    pub(super) fn is_tool(name: &str) -> bool {
        NAMES.contains(&name)
    }
    pub(super) fn definitions() -> Vec<Value> {
        let path = json!({"type":"string","description":"UTF-8 relative path, forward slash separates components, at most65536 UTF-8 bytes/256 components; native OS filename rules. No symlink traversal."});
        let annotations = |readonly: bool| json!({"readOnlyHint":readonly,"destructiveHint":!readonly,"idempotentHint":readonly,"openWorldHint":false});
        vec![
            json!({"name":NAMES[0],"description":"Describe the configured directory capability and explicit limits; never exposes its host path.","inputSchema":{"type":"object","properties":{},"additionalProperties":false},"annotations":annotations(true)}),
            json!({"name":NAMES[1],"description":"List sorted names and nofollow entry types. Entire inventory is bounded to10000 entries/2MiB names; over-budget is an error, never silent truncation. Cursors bind actual name/type inventory and this capability instance.","inputSchema":{"type":"object","properties":{"path":path,"limit":{"type":"integer","minimum":1,"maximum":200},"cursor":{"type":["string","null"]}},"additionalProperties":false},"annotations":annotations(true)}),
            json!({"name":NAMES[2],"description":"Create relative directory components without following symlinks. Existing directories are allowed. Earlier created components may remain if later IO fails.","inputSchema":{"type":"object","properties":{"path":path},"required":["path"],"additionalProperties":false},"annotations":annotations(false)}),
            json!({"name":NAMES[3],"description":"Read a regular UTF-8 file in codepoint-aligned byte chunks≤262144. Validates whole observed file≤33554432 bytes and content SHA each call. Nonzero offset requires previous snapshot. Special/binary files are errors; no hard filesystem latency guarantee.","inputSchema":{"type":"object","properties":{"path":path,"offset":{"type":"integer","minimum":0,"maximum":READ_FILE_MAX_BYTES},"max_bytes":{"type":"integer","minimum":1,"maximum":READ_CHUNK_MAX_BYTES},"snapshot":{"type":["string","null"],"pattern":"^[0-9a-f]{64}$"}},"required":["path"],"additionalProperties":false},"annotations":annotations(true)}),
            json!({"name":NAMES[4],"description":"Atomically replace/create a regular file with whole UTF-8 text≤1048576 bytes through a new same-parent file, sync and rename. Existing destination inode is never truncated. Existing symlink/special/directory rejected. Atomic replacement is destructive; directory-entry crash durability is not proven.","inputSchema":{"type":"object","properties":{"path":path,"text":{"type":"string","description":"At most1048576 UTF-8 bytes."}},"required":["path","text"],"additionalProperties":false},"annotations":annotations(false)}),
        ]
    }
    pub(super) async fn call(&self, name: &str, args: Value) -> Result<Value> {
        // Strict parsing is intentionally before admission, spawning or filesystem IO.
        let operation = Operation::parse(name, args)?;
        let root = self.inner.root.clone();
        let scope = self.inner.scope.clone();
        self.run(move || operation.execute(&root, &scope)).await
    }
    async fn reap_completed(&self) {
        let completed: Vec<_> = lock(&self.inner.registry)
            .jobs
            .iter()
            .filter(|(_, job)| job.completed.load(Ordering::Acquire))
            .map(|(id, job)| (*id, job.clone()))
            .collect();
        for (id, job) in completed {
            finish_job(&self.inner, id, &job).await;
        }
    }
    async fn run(
        &self,
        operation: impl FnOnce() -> Result<Value> + Send + 'static,
    ) -> Result<Value> {
        self.reap_completed().await;
        let (reply_tx, reply_rx) = oneshot::channel();
        let (start_tx, start_rx) = oneshot::channel::<()>();
        let (id, job) = {
            let mut registry = lock(&self.inner.registry);
            ensure!(!registry.closing, "Filesystem tools are shutting down");
            ensure!(
                registry.jobs.len() < MAX_CONCURRENT_FILE_OPS,
                "Filesystem operation limit reached; retry later"
            );
            let permit = self
                .inner
                .permits
                .clone()
                .try_acquire_owned()
                .context("Filesystem operation limit reached")?;
            let id = registry
                .next_id
                .checked_add(1)
                .context("Filesystem job identifier exhausted")?;
            registry.next_id = id;
            let handle = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                start_rx
                    .blocking_recv()
                    .context("Filesystem start admission canceled")?;
                operation()
            });
            let job = Arc::new(Job {
                worker: AsyncMutex::new(Worker {
                    join: Some(handle),
                    reply: Some(reply_tx),
                }),
                supervisor: Mutex::new(None),
                completed: AtomicBool::new(false),
            });
            registry.jobs.insert(id, job.clone());
            let weak = Arc::downgrade(&self.inner);
            let monitored = job.clone();
            let handle = tokio::spawn(async move {
                join_worker(&weak, &monitored).await;
            });
            *lock(&job.supervisor) = Some(Arc::new(Supervisor {
                join: AsyncMutex::new(Some(handle)),
            }));
            // Still under admission lock: shutdown cannot miss this registered worker.
            let _ = start_tx.send(());
            (id, job)
        };
        finish_job(&self.inner, id, &job).await;
        reply_rx
            .await
            .context("Filesystem worker result unavailable")?
    }
    pub(super) fn close_admission(&self) {
        lock(&self.inner.registry).closing = true;
        self.inner.permits.close();
    }
    pub(super) async fn shutdown_and_drain(&self) -> Result<()> {
        let jobs: Vec<_> = {
            let mut registry = lock(&self.inner.registry);
            registry.closing = true;
            self.inner.permits.close();
            registry
                .jobs
                .iter()
                .map(|(id, job)| (*id, job.clone()))
                .collect()
        };
        for (id, job) in jobs {
            finish_job(&self.inner, id, &job).await;
        }
        let registry = lock(&self.inner.registry);
        ensure!(
            registry.jobs.is_empty(),
            "Filesystem drain has unjoined jobs"
        );
        if let Some(message) = &registry.first_join_failure {
            bail!("{message}");
        }
        Ok(())
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[derive(Debug, Clone)]
struct RelativePath {
    text: String,
    parts: Vec<String>,
}
impl RelativePath {
    fn parse(value: String, root_allowed: bool) -> Result<Self> {
        ensure!(
            value.len() <= PATH_MAX_BYTES,
            "Relative path exceeds65536 UTF-8 bytes"
        );
        if value == "." && root_allowed {
            return Ok(Self {
                text: value,
                parts: Vec::new(),
            });
        }
        ensure!(
            !value.is_empty() && !value.starts_with('/'),
            "Relative path is required; absolute paths are forbidden"
        );
        let parts: Vec<String> = value.split('/').map(str::to_owned).collect();
        ensure!(
            parts.len() <= COMPONENT_MAX_COUNT,
            "Relative path exceeds256 components"
        );
        for component in &parts {
            ensure!(
                !component.is_empty() && component != "." && component != "..",
                "Empty/dot/parent components are forbidden"
            );
            ensure!(
                !component.contains('\0'),
                "NUL is forbidden in path components"
            );
            #[cfg(unix)]
            ensure!(
                component.len() <= 255,
                "Native Unix component exceeds255 bytes"
            );
            #[cfg(windows)]
            {
                ensure!(
                    component.encode_utf16().count() <= 255,
                    "Native Windows component exceeds255 UTF-16 units"
                );
                ensure!(
                    !component.ends_with(['.', ' ']),
                    "Windows trailing dots/spaces are forbidden"
                );
                ensure!(
                    !component.chars().any(|c| c <= '\u{1f}'
                        || matches!(c, '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*')),
                    "Windows path contains reserved syntax"
                );
                let upper = component.to_ascii_uppercase();
                let stem = upper.split('.').next().unwrap_or("").trim_end_matches(' ');
                let device = matches!(
                    stem,
                    "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$" | "CONIN$" | "CONOUT$"
                ) || ["COM", "LPT"].iter().any(|prefix| {
                    stem.strip_prefix(*prefix).is_some_and(|suffix| {
                        matches!(
                            suffix,
                            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                        )
                    })
                });
                ensure!(!device, "Reserved Windows device path component");
            }
            #[cfg(windows)]
            let internal = component.to_ascii_lowercase().starts_with(STAGING_PREFIX);
            #[cfg(not(windows))]
            let internal = component.starts_with(STAGING_PREFIX);
            ensure!(!internal, "Internal staging names are reserved");
        }
        Ok(Self { text: value, parts })
    }
    fn directory(&self, root: &Dir) -> Result<Dir> {
        let mut directory = root.try_clone()?;
        for part in &self.parts {
            directory = directory
                .open_dir_nofollow(part)
                .with_context(|| format!("Cannot open nofollow directory component {part:?}"))?;
        }
        Ok(directory)
    }
    fn parent(&self, root: &Dir) -> Result<(Dir, &str)> {
        let (name, parents) = self
            .parts
            .split_last()
            .context("File/directory leaf required")?;
        let mut directory = root.try_clone()?;
        for part in parents {
            directory = directory
                .open_dir_nofollow(part)
                .with_context(|| format!("Cannot open nofollow parent {part:?}"))?;
        }
        Ok((directory, name))
    }
}

fn open_root(path: &Path) -> Result<Dir> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    ensure!(
        absolute.is_absolute(),
        "Configured base directory must resolve to an absolute path"
    );
    let mut anchor = PathBuf::new();
    let mut parts = Vec::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(prefix) => anchor.push(prefix.as_os_str()),
            Component::RootDir => anchor.push(component.as_os_str()),
            Component::Normal(part) => parts.push(part.to_os_string()),
            Component::CurDir => (),
            Component::ParentDir => {
                bail!("Configured base directory must not contain parent components")
            }
        }
    }
    let mut root = Dir::open_ambient_dir(anchor, cap_std::ambient_authority())?;
    for part in parts {
        root = root
            .open_dir_nofollow(part)
            .context("Configured base directory must already exist without symlink traversal")?;
    }
    Ok(root)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyArgs {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArgs {
    #[serde(default = "root_path")]
    path: String,
    #[serde(default = "list_limit")]
    limit: usize,
    #[serde(default)]
    cursor: Option<String>,
}
fn root_path() -> String {
    ".".into()
}
fn list_limit() -> usize {
    100
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PathArgs {
    path: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadArgs {
    path: String,
    #[serde(default)]
    offset: u64,
    #[serde(default = "read_limit")]
    max_bytes: usize,
    #[serde(default)]
    snapshot: Option<String>,
}
fn read_limit() -> usize {
    65536
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WriteArgs {
    path: String,
    text: String,
}

enum Operation {
    Allowed,
    List {
        path: RelativePath,
        limit: usize,
        cursor: Option<ListCursor>,
    },
    Create(RelativePath),
    Read {
        path: RelativePath,
        offset: usize,
        max_bytes: usize,
        snapshot: Option<String>,
    },
    Write {
        path: RelativePath,
        text: String,
    },
}
impl Operation {
    fn parse(name: &str, args: Value) -> Result<Self> {
        ensure!(args.is_object(), "File-tool arguments must be an object");
        match name {
            "spider_list_allowed_directory" => {
                let _: EmptyArgs = serde_json::from_value(args)?;
                Ok(Self::Allowed)
            }
            "spider_list_directory" => {
                let args: ListArgs = serde_json::from_value(args)?;
                ensure!(
                    (1..=200).contains(&args.limit),
                    "Directory page limit must be1..200"
                );
                let path = RelativePath::parse(args.path, true)?;
                let cursor = args.cursor.as_deref().map(ListCursor::decode).transpose()?;
                if let Some(cursor) = &cursor {
                    ensure!(
                        cursor.path == path.text,
                        "Directory cursor belongs to another path"
                    );
                }
                Ok(Self::List {
                    path,
                    limit: args.limit,
                    cursor,
                })
            }
            "spider_create_directory" => {
                let args: PathArgs = serde_json::from_value(args)?;
                Ok(Self::Create(RelativePath::parse(args.path, false)?))
            }
            "spider_read_text_file" => {
                let args: ReadArgs = serde_json::from_value(args)?;
                let path = RelativePath::parse(args.path, false)?;
                ensure!(
                    (1..=READ_CHUNK_MAX_BYTES).contains(&args.max_bytes),
                    "Read chunk must be1..262144 bytes"
                );
                let offset =
                    usize::try_from(args.offset).context("Read offset cannot be represented")?;
                ensure!(
                    offset <= READ_FILE_MAX_BYTES,
                    "Read offset exceeds supported file budget"
                );
                if let Some(snapshot) = &args.snapshot {
                    ensure!(
                        valid_hash(snapshot),
                        "Snapshot must be64 lowercase hex bytes"
                    );
                }
                ensure!(
                    offset == 0 || args.snapshot.is_some(),
                    "Nonzero offset requires the previous observed snapshot"
                );
                Ok(Self::Read {
                    path,
                    offset,
                    max_bytes: args.max_bytes,
                    snapshot: args.snapshot,
                })
            }
            "spider_write_text_file" => {
                let args: WriteArgs = serde_json::from_value(args)?;
                let path = RelativePath::parse(args.path, false)?;
                ensure!(
                    args.text.len() <= WRITE_TEXT_MAX_BYTES,
                    "Text exceeds1MiB UTF-8 write budget"
                );
                Ok(Self::Write {
                    path,
                    text: args.text,
                })
            }
            _ => bail!("Unknown opt-in filesystem tool"),
        }
    }
    fn execute(self, root: &Dir, scope: &str) -> Result<Value> {
        let result = match self {
            Self::Allowed => {
                json!({"allowed_directory":".","authority":"configured_directory_handle","website_requests":0,"limits":{"write_text_max_bytes":WRITE_TEXT_MAX_BYTES,"read_chunk_max_bytes":READ_CHUNK_MAX_BYTES,"read_file_max_bytes":READ_FILE_MAX_BYTES,"directory_max_entries":DIRECTORY_MAX_ENTRIES,"directory_name_max_bytes":DIRECTORY_NAME_MAX_BYTES,"concurrent_file_operations":MAX_CONCURRENT_FILE_OPS}})
            }
            Self::List {
                path,
                limit,
                cursor,
            } => list(root, scope, path, limit, cursor)?,
            Self::Create(path) => create(root, path)?,
            Self::Read {
                path,
                offset,
                max_bytes,
                snapshot,
            } => read(root, scope, path, offset, max_bytes, snapshot)?,
            Self::Write { path, text } => write(root, path, text)?,
        };
        bounded_result(result)
    }
}

struct Count(usize);
impl Write for Count {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("File-result length overflow"))?;
        if next > MAX_RESULT_JSON_BYTES {
            return Err(io::Error::other("File-result JSON byte budget exceeded"));
        }
        self.0 = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn bounded_result(value: Value) -> Result<Value> {
    serde_json::to_writer(&mut Count(0), &value)
        .context("File-tool result exceeds2MiB JSON budget")?;
    Ok(value)
}
struct HashWrite(Sha256);
impl Write for HashWrite {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn json_hash(value: &impl Serialize) -> Result<String> {
    let mut writer = HashWrite(Sha256::new());
    serde_json::to_writer(&mut writer, value)?;
    Ok(format!("{:x}", writer.0.finalize()))
}
#[derive(Serialize)]
struct ListingEntry {
    name: String,
    kind: &'static str,
    addressable_name: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListCursor {
    version: u8,
    scope: String,
    path: String,
    inventory_sha256: String,
    after: String,
}
impl ListCursor {
    fn decode(value: &str) -> Result<Self> {
        ensure!(
            !value.is_empty() && value.len() <= CURSOR_MAX_BYTES,
            "Directory cursor byte budget"
        );
        ensure!(
            value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "Directory cursor must use canonical base64url"
        );
        let raw = URL_SAFE_NO_PAD.decode(value)?;
        ensure!(
            URL_SAFE_NO_PAD.encode(&raw) == value,
            "Noncanonical directory cursor"
        );
        let cursor: Self = serde_json::from_slice(&raw)?;
        ensure!(
            cursor.version == 1 && valid_hash(&cursor.inventory_sha256),
            "Unsupported directory cursor"
        );
        ensure!(
            uuid::Uuid::parse_str(&cursor.scope).is_ok()
                && cursor.path.len() <= PATH_MAX_BYTES
                && cursor.after.len() <= 1024,
            "Invalid directory cursor binding"
        );
        Ok(cursor)
    }
    fn encode(&self) -> Result<String> {
        let token = URL_SAFE_NO_PAD.encode(serde_json::to_vec(self)?);
        ensure!(
            token.len() <= CURSOR_MAX_BYTES,
            "Directory cursor output budget"
        );
        Ok(token)
    }
}
fn list(
    root: &Dir,
    scope: &str,
    path: RelativePath,
    limit: usize,
    cursor: Option<ListCursor>,
) -> Result<Value> {
    if let Some(cursor) = &cursor {
        ensure!(
            cursor.scope == scope,
            "Directory cursor belongs to another capability instance"
        );
    }
    let directory = path.directory(root)?;
    let mut entries = Vec::new();
    let mut bytes = 0usize;
    for entry in directory.entries()? {
        let entry = entry?;
        ensure!(
            entries.len() < DIRECTORY_MAX_ENTRIES,
            "Directory exceeds10000-entry scan budget; no truncated page returned"
        );
        let name = entry.file_name().into_string().map_err(|_| {
            anyhow::anyhow!("Directory contains a non-UTF8 name; no lossy listing returned")
        })?;
        bytes = bytes
            .checked_add(name.len())
            .context("Directory name-byte count overflow")?;
        ensure!(
            bytes <= DIRECTORY_NAME_MAX_BYTES,
            "Directory exceeds2MiB name-byte scan budget"
        );
        let kind = entry.file_type()?;
        let addressable_name = RelativePath::parse(name.clone(), false).is_ok();
        entries.push(ListingEntry {
            name,
            kind: if kind.is_symlink() {
                "symlink"
            } else if kind.is_dir() {
                "directory"
            } else if kind.is_file() {
                "file"
            } else {
                "other"
            },
            addressable_name,
        });
    }
    entries.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    ensure!(
        entries.windows(2).all(|pair| pair[0].name != pair[1].name),
        "Directory enumeration repeated an entry"
    );
    let inventory = json_hash(&("open-seo-spider/files-list-v1", scope, &path.text, &entries))?;
    let start = if let Some(cursor) = &cursor {
        ensure!(
            cursor.inventory_sha256 == inventory,
            "Stale directory cursor: name/type inventory changed"
        );
        entries
            .binary_search_by(|entry| entry.name.cmp(&cursor.after))
            .map(|index| index + 1)
            .map_err(|_| anyhow::anyhow!("Directory cursor anchor is not an actual entry"))?
    } else {
        0
    };
    let end = start.saturating_add(limit).min(entries.len());
    let next = if end < entries.len() {
        Some(
            ListCursor {
                version: 1,
                scope: scope.into(),
                path: path.text.clone(),
                inventory_sha256: inventory.clone(),
                after: entries[end - 1].name.clone(),
            }
            .encode()?,
        )
    } else {
        None
    };
    Ok(
        json!({"path":path.text,"entries":&entries[start..end],"total_entries":entries.len(),"inventory_sha256":inventory,"next_cursor":next,"inventory_scope":"names_and_types_only","website_requests":0}),
    )
}
fn create(root: &Dir, path: RelativePath) -> Result<Value> {
    let mut current = root.try_clone()?;
    let mut created = Vec::new();
    for part in &path.parts {
        match current.open_dir_nofollow(part) {
            Ok(next) => current = next,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                match current.create_dir(part) {
                    Ok(()) => created.push(part.clone()),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
                    Err(error) => return Err(error.into()),
                }
                current = current.open_dir_nofollow(part)?;
            }
            Err(error) => {
                return Err(error).context("Directory creation refused nofollow traversal");
            }
        }
    }
    Ok(
        json!({"path":path.text,"created_components":created,"directory_available":true,"website_requests":0}),
    )
}
fn open_regular(parent: &Dir, name: &str) -> Result<cap_std::fs::File> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No).nonblock(true);
    let file = parent.open_with(name, &options)?;
    ensure!(
        file.metadata()?.is_file(),
        "Only regular files are supported"
    );
    Ok(file)
}
fn read(
    root: &Dir,
    scope: &str,
    path: RelativePath,
    offset: usize,
    max_bytes: usize,
    expected: Option<String>,
) -> Result<Value> {
    let (parent, name) = path.parent(root)?;
    let file = open_regular(&parent, name)?;
    let before = file.metadata()?;
    ensure!(
        before.len() <= READ_FILE_MAX_BYTES as u64,
        "File exceeds32MiB whole-observation budget"
    );
    let modified = before
        .modified()
        .context("File modification time is unavailable")?;
    let mut bytes = Vec::with_capacity(before.len() as usize + 1);
    let mut bounded = (&file).take(READ_FILE_MAX_BYTES as u64 + 1);
    bounded.read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= READ_FILE_MAX_BYTES,
        "File grew beyond32MiB observation budget"
    );
    let after = file.metadata()?;
    ensure!(
        after.len() == before.len()
            && bytes.len() as u64 == before.len()
            && after.modified()? == modified,
        "File changed during observation; retry"
    );
    let content_sha = digest(&bytes);
    let snapshot = json_hash(&(
        "open-seo-spider/files-read-v1",
        scope,
        &path.text,
        &content_sha,
        bytes.len(),
    ))?;
    if let Some(expected) = expected {
        ensure!(expected == snapshot, "Stale file snapshot; content changed");
    }
    let text =
        String::from_utf8(bytes).context("File is not valid UTF-8; no lossy text returned")?;
    ensure!(
        offset <= text.len() && text.is_char_boundary(offset),
        "Offset is not a valid UTF-8 byte boundary"
    );
    let mut end = offset.saturating_add(max_bytes).min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    ensure!(
        end > offset || offset == text.len(),
        "Chunk limit is smaller than the next UTF-8 code point"
    );
    Ok(
        json!({"path":path.text,"text":&text[offset..end],"offset":offset,"byte_length":end-offset,"total_bytes":text.len(),"next_offset":if end < text.len() {Some(end)} else {None},"eof":end==text.len(),"sha256":content_sha,"snapshot":snapshot,"snapshot_profile":"whole_observed_utf8_content_v1","website_requests":0}),
    )
}
fn existing_regular(parent: &Dir, name: &str) -> Result<bool> {
    match parent.symlink_metadata(name) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "Existing destination must be a regular non-symlink file"
            );
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}
struct Staged<'a> {
    parent: &'a Dir,
    name: String,
    identity: (u64, u64),
    committed: bool,
}
impl Staged<'_> {
    fn cleanup(&mut self) -> Result<()> {
        if self.committed {
            return Ok(());
        }
        match open_regular(self.parent, &self.name) {
            Ok(file) => {
                let metadata = file.metadata()?;
                ensure!(
                    (metadata.dev(), metadata.ino()) == self.identity,
                    "Staging entry identity changed; foreign entry retained"
                );
                drop(file);
                self.parent.remove_file(&self.name)?;
                self.committed = true;
                Ok(())
            }
            Err(error)
                if error
                    .downcast_ref::<io::Error>()
                    .is_some_and(|error| error.kind() == io::ErrorKind::NotFound) =>
            {
                self.committed = true;
                Ok(())
            }
            Err(error) => Err(error).context("Cannot clean own staging entry"),
        }
    }
}
impl Drop for Staged<'_> {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
fn write(root: &Dir, path: RelativePath, text: String) -> Result<Value> {
    let (parent, name) = path.parent(root)?;
    let observed_existing = existing_regular(&parent, name)?;
    let mut staged = None;
    let mut file = None;
    for _ in 0..16 {
        let candidate = format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4().simple());
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No)
            .nonblock(true);
        match parent.open_with(&candidate, &options) {
            Ok(opened) => {
                let metadata = match opened.metadata() {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        // Without a held-handle identity we cannot safely distinguish an
                        // external replacement for cleanup. Report and retain, never hide it.
                        return Err(error).with_context(|| format!("Staging metadata unavailable; unverified relative staging entry {candidate:?} retained"));
                    }
                };
                staged = Some(Staged {
                    parent: &parent,
                    name: candidate,
                    identity: (metadata.dev(), metadata.ino()),
                    committed: false,
                });
                file = Some(opened);
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
            Err(error) => return Err(error.into()),
        }
    }
    let mut staged = staged.context("Could not reserve own unique staging file")?;
    let result: Result<Value> = (|| {
        let mut file = file.take().context("Staging handle missing")?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        existing_regular(&parent, name)?;
        let check = open_regular(&parent, &staged.name)?;
        let metadata = check.metadata()?;
        ensure!(
            (metadata.dev(), metadata.ino()) == staged.identity,
            "Staging entry identity changed before rename"
        );
        drop(check);
        parent.rename(&staged.name, &parent, name)?;
        staged.committed = true;
        Ok(
            json!({"path":path.text.clone(),"bytes_written":text.len(),"sha256":digest(text.as_bytes()),"atomic_replace":true,"existing_regular_file_observed_before_write":observed_existing,"file_synced":true,"directory_entry_crash_durability_proven":false,"website_requests":0}),
        )
    })();
    if let Err(error) = result {
        return match staged.cleanup() {
            Ok(()) => Err(error),
            Err(cleanup) => {
                Err(error).context(format!("Own staging cleanup also failed: {cleanup:#}"))
            }
        };
    }
    result
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
