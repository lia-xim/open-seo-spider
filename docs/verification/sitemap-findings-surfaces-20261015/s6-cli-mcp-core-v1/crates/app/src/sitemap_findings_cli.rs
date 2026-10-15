//! One verified SQLite session produces a JSON page and/or complete stored CSV.
//! Publication uses new-name hard links. A pair is not a filesystem transaction.
use crate::sitemap_findings::{
    self, AccessTarget, MAX_CSV_BYTES, MAX_JSON_BYTES, MAX_REQUEST_BYTES, ReadQuery,
};
use anyhow::{Context, ensure};
use open_seo_spider_core::{CrawlStore, SitemapReadControl};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[cfg(test)]
#[path = "sitemap_findings_cli_tests.rs"]
mod tests;

#[derive(Debug, Serialize)]
pub(crate) struct ArtifactReceipt {
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
    pub data_rows: Option<u64>,
}
#[derive(Debug, Serialize)]
pub(crate) struct Receipt {
    pub engine: &'static str,
    pub access_target: AccessTarget,
    pub website_requests: u8,
    pub json_page: Option<ArtifactReceipt>,
    pub csv_full_selection: Option<ArtifactReceipt>,
    pub temporary_cleanup_warnings: Vec<String>,
}

fn bounded_request(path: &Path) -> anyhow::Result<ReadQuery> {
    let file = File::open(path).context("Open findings request")?;
    ensure!(file.metadata()?.is_file(), "Request must be a regular file");
    let mut bytes = Vec::new();
    file.take(MAX_REQUEST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= MAX_REQUEST_BYTES,
        "Finding request exceeds 64 MiB"
    );
    let query: ReadQuery =
        serde_json::from_slice(&bytes).context("Strict findings request JSON")?;
    query.validate()?;
    Ok(query)
}
fn new_destination(path: &Path) -> anyhow::Result<PathBuf> {
    ensure!(
        fs::symlink_metadata(path).is_err_and(|e| e.kind() == io::ErrorKind::NotFound),
        "Output already exists or cannot be inspected: {}",
        path.display()
    );
    let name = path.file_name().context("Output requires a file name")?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = fs::canonicalize(parent).context("Output directory must exist")?;
    ensure!(parent.is_dir(), "Output parent must be a directory");
    Ok(parent.join(name))
}
fn same_destination(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

struct Measured<W> {
    writer: W,
    bytes: u64,
    hash: Sha256,
    limit: usize,
    control: SitemapReadControl,
}
impl<W: Write> Measured<W> {
    fn new(writer: W, limit: usize, control: &SitemapReadControl) -> Self {
        Self {
            writer,
            bytes: 0,
            hash: Sha256::new(),
            limit,
            control: control.clone(),
        }
    }
    fn digest(&self) -> String {
        format!("{:x}", self.hash.clone().finalize())
    }
}
impl<W: Write> Write for Measured<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.control.checkpoint().map_err(io::Error::other)?;
        if bytes.len() > self.limit.saturating_sub(self.bytes as usize) {
            return Err(io::Error::other("Finding output byte budget exceeded"));
        }
        let count = self.writer.write(bytes)?;
        self.hash.update(&bytes[..count]);
        self.bytes += count as u64;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
struct Staged {
    temporary: PathBuf,
    receipt: ArtifactReceipt,
}
impl Drop for Staged {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.temporary);
    }
}
impl Staged {
    fn create(destination: PathBuf) -> anyhow::Result<(Self, File)> {
        let temporary = destination
            .parent()
            .context("Output parent missing")?
            .join(format!(".oss-findings-{}.tmp", uuid::Uuid::new_v4()));
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        Ok((
            Self {
                temporary,
                receipt: ArtifactReceipt {
                    path: destination,
                    bytes: 0,
                    sha256: String::new(),
                    data_rows: None,
                },
            },
            file,
        ))
    }
    fn publish(&self) -> anyhow::Result<()> {
        fs::hard_link(&self.temporary, &self.receipt.path).with_context(|| format!("Atomic new-name hard-link publication failed for {}; no existing destination was replaced", self.receipt.path.display()))
    }
    fn cleanup(&self) -> Option<String> {
        fs::remove_file(&self.temporary).err().map(|e| {
            format!(
                "Complete output published; own temporary link {} could not be removed: {e}",
                self.temporary.display()
            )
        })
    }
}
struct StagedOutputs {
    json: Option<Staged>,
    csv: Option<Staged>,
}

fn prepare_outputs(
    store: PathBuf,
    request: PathBuf,
    output: Option<PathBuf>,
    csv: Option<PathBuf>,
    control: &SitemapReadControl,
) -> anyhow::Result<StagedOutputs> {
    ensure!(
        output.is_some() || csv.is_some(),
        "At least --output or --csv is required"
    );
    // Parse and validate before opening SQLite or creating even a temporary output.
    let prepared = bounded_request(&request)?.prepare(&AccessTarget::Offline)?;
    let output = output.as_deref().map(new_destination).transpose()?;
    let csv = csv.as_deref().map(new_destination).transpose()?;
    ensure!(
        !output
            .as_ref()
            .zip(csv.as_ref())
            .is_some_and(|(a, b)| same_destination(a, b)),
        "JSON and CSV destinations must differ"
    );
    control.checkpoint()?;
    let store = CrawlStore::open_read_only(store)?;
    store.with_verified_sitemap_findings(
        prepared.selection(),
        prepared.request().expected_context_sha256.as_deref(),
        control,
        |session| {
            let page = session.page(prepared.request())?;
            let envelope = sitemap_findings::bind_page(&AccessTarget::Offline, &prepared, page)?;
            let mut result = StagedOutputs {
                json: None,
                csv: None,
            };
            // Full count/hash pass and second pass run against the SAME session/TX.
            // No final filename exists until the transaction has returned successfully.
            if let Some(destination) = csv {
                let mut first = Measured::new(io::sink(), MAX_CSV_BYTES, control);
                let rows = sitemap_findings::write_findings_csv_session(session, &mut first)?;
                let (mut stage, file) = Staged::create(destination)?;
                let mut second = Measured::new(file, MAX_CSV_BYTES, control);
                let second_rows =
                    sitemap_findings::write_findings_csv_session(session, &mut second)?;
                ensure!(
                    rows == second_rows
                        && first.bytes == second.bytes
                        && first.digest() == second.digest(),
                    "Finding CSV repeat-pass count/digest mismatch"
                );
                second.writer.sync_all()?;
                stage.receipt.bytes = second.bytes;
                stage.receipt.sha256 = second.digest();
                stage.receipt.data_rows = Some(rows);
                drop(second);
                result.csv = Some(stage);
            }
            if let Some(destination) = output {
                let (mut stage, file) = Staged::create(destination)?;
                let mut sink = Measured::new(file, MAX_JSON_BYTES, control);
                serde_json::to_writer(&mut sink, &envelope)?;
                sink.writer.sync_all()?;
                stage.receipt.bytes = sink.bytes;
                stage.receipt.sha256 = sink.digest();
                drop(sink);
                result.json = Some(stage);
            }
            control.checkpoint()?;
            Ok(result)
        },
    )
}

fn publish_outputs(staged: StagedOutputs) -> anyhow::Result<Receipt> {
    let mut published: Option<&Path> = None;
    for item in [staged.json.as_ref(), staged.csv.as_ref()]
        .into_iter()
        .flatten()
    {
        if let Err(error) = item.publish() {
            if let Some(first) = published {
                // Portable check-then-delete could remove a concurrently replaced
                // path. Preserve the COMPLETE first artifact and report the pair.
                return Err(error.context(format!("Partially published output pair: COMPLETE first artifact retained at {}; second publication failed. No success receipt; the pair is not atomic", first.display())));
            }
            return Err(error);
        }
        published = Some(&item.receipt.path);
    }
    let warnings = [staged.json.as_ref(), staged.csv.as_ref()]
        .into_iter()
        .flatten()
        .filter_map(Staged::cleanup)
        .collect();
    // Staged owns temporary cleanup only; final paths are never removed by Drop.
    Ok(Receipt {
        engine: sitemap_findings::ENGINE,
        access_target: AccessTarget::Offline,
        website_requests: 0,
        json_page: staged.json.as_ref().map(|s| ArtifactReceipt {
            path: s.receipt.path.clone(),
            bytes: s.receipt.bytes,
            sha256: s.receipt.sha256.clone(),
            data_rows: s.receipt.data_rows,
        }),
        csv_full_selection: staged.csv.as_ref().map(|s| ArtifactReceipt {
            path: s.receipt.path.clone(),
            bytes: s.receipt.bytes,
            sha256: s.receipt.sha256.clone(),
            data_rows: s.receipt.data_rows,
        }),
        temporary_cleanup_warnings: warnings,
    })
}

pub(crate) async fn run(
    store: PathBuf,
    request: PathBuf,
    output: Option<PathBuf>,
    csv: Option<PathBuf>,
) -> anyhow::Result<Receipt> {
    let deadline = Instant::now() + Duration::from_secs(300);
    let control = SitemapReadControl::with_deadline(deadline);
    let worker_control = control.clone();
    let mut worker = tokio::task::spawn_blocking(move || {
        prepare_outputs(store, request, output, csv, &worker_control)
    });
    let staged = tokio::select! {
        result = &mut worker => result??,
        signal = tokio::signal::ctrl_c() => {
            control.cancel();
            let _ = worker.await;
            signal.context("Observe CLI cancellation")?;
            anyhow::bail!("Finding CLI cancelled; worker joined, no final outputs published");
        }
        _ = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {
            control.expire();
            let _ = worker.await;
            anyhow::bail!("Finding CLI deadline exceeded; worker joined, no final outputs published");
        }
    };
    control.checkpoint()?;
    // Publication runs only after the actual worker/TX has returned and joined.
    tokio::task::spawn_blocking(move || publish_outputs(staged)).await?
}
