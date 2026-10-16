//! Local saved-report export. Publication is an atomic new-name hard link, never overwrite.
use anyhow::{Context, ensure};
use open_seo_spider_core::{
    CrawlReport, SitemapGenerationOptions, generate_sitemap_bundle,
    validate_sitemap_generation_options,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MAX_OPTIONS_BYTES: usize = 64 * 1024 * 1024;
const MAX_REPORT_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct Receipt {
    pub output: PathBuf,
    pub bytes: usize,
    pub sha256: String,
    pub report_sha256: String,
    pub generated_urls: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting: Option<open_seo_spider_core::SitemapGenerationReporting>,
    pub excluded_pages: usize,
    pub engine: String,
    pub temporary_cleanup_warning: Option<String>,
}

async fn bounded(path: &Path, limit: usize) -> anyhow::Result<Vec<u8>> {
    let file = tokio::fs::File::open(path)
        .await
        .context("Open bounded JSON input")?;
    ensure!(
        file.metadata().await?.is_file(),
        "Input must be a regular file"
    );
    ensure!(
        file.metadata().await?.len() <= limit as u64,
        "Input exceeds {limit} bytes"
    );
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes).await?;
    ensure!(bytes.len() <= limit, "Input changed beyond {limit} bytes");
    Ok(bytes)
}

pub async fn run(snapshot: PathBuf, options: PathBuf, output: PathBuf) -> anyhow::Result<Receipt> {
    let options_bytes = bounded(&options, MAX_OPTIONS_BYTES).await?;
    // JSON parsing and validation run on a worker, including caller-supplied date profiles.
    let options =
        tokio::task::spawn_blocking(move || -> anyhow::Result<SitemapGenerationOptions> {
            let options = serde_json::from_slice(&options_bytes)?;
            validate_sitemap_generation_options(&options)?;
            Ok(options)
        })
        .await??;
    let output = new_destination(&output).await?;
    let report_bytes = bounded(&snapshot, MAX_REPORT_BYTES).await?;
    let bundle = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let report: CrawlReport = serde_json::from_slice(&report_bytes)?;
        Ok(generate_sitemap_bundle(&report, &options)?)
    })
    .await??;
    let warning = publish_new(&output, &bundle.zip_bytes).await?;
    Ok(Receipt {
        output,
        bytes: bundle.zip_bytes.len(),
        sha256: bundle.zip_sha256,
        report_sha256: bundle.manifest.report_sha256,
        generated_urls: bundle.manifest.memberships.len(),
        reporting: bundle.manifest.reporting,
        excluded_pages: bundle.manifest.excluded_pages,
        engine: bundle.manifest.engine,
        temporary_cleanup_warning: warning,
    })
}

async fn new_destination(path: &Path) -> anyhow::Result<PathBuf> {
    ensure!(
        tokio::fs::symlink_metadata(path)
            .await
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
        "Output already exists or cannot be inspected"
    );
    let name = path.file_name().context("Output requires a file name")?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = tokio::fs::canonicalize(parent)
        .await
        .context("Output directory must exist")?;
    ensure!(
        tokio::fs::metadata(&parent).await?.is_dir(),
        "Output parent must be a directory"
    );
    Ok(parent.join(name))
}

async fn publish_new(output: &Path, bytes: &[u8]) -> anyhow::Result<Option<String>> {
    let parent = output.parent().context("Output parent missing")?;
    let temp = parent.join(format!(".oss-sitemap-{}.tmp", uuid::Uuid::new_v4()));
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .await?;
    let result = async {
        file.write_all(bytes).await?;
        file.sync_all().await?;
        drop(file);
        // Both names share a directory/filesystem. A preexisting destination makes this fail;
        // there is deliberately no rename fallback that could replace an existing file.
        tokio::fs::hard_link(&temp, output).await.context(
            "Atomic new-file publication requires hard-link support; output was not replaced",
        )?;
        Ok::<_, anyhow::Error>(())
    }
    .await;
    let cleanup = tokio::fs::remove_file(&temp).await;
    result?;
    Ok(cleanup.err().map(|e| {
        format!(
            "Complete output published, but own temporary link {} could not be removed: {e}",
            temp.display()
        )
    }))
}

#[cfg(test)]
#[path = "generation_export_tests.rs"]
mod tests;
