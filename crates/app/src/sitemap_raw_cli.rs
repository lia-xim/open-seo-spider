//! Raw-only reader. Rich App membership/coverage cursors have a separate scope.
use open_seo_spider_core::{CrawlStore, SitemapRawReadRequest};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use tokio::io::AsyncReadExt;

const MAX_REQUEST_BYTES: usize = 128 * 1024;
async fn read_request(path: &Path) -> anyhow::Result<SitemapRawReadRequest> {
    // Bound the actual handle read, including a single refusal sentinel. A
    // metadata check followed by fs::read would allow growth before allocation.
    let file = tokio::fs::File::open(path).await?;
    let mut bytes = Vec::with_capacity(MAX_REQUEST_BYTES + 1);
    file.take((MAX_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await?;
    anyhow::ensure!(
        bytes.len() <= MAX_REQUEST_BYTES,
        "Raw sitemap request exceeds 128 KiB"
    );
    Ok(serde_json::from_slice(&bytes)?)
}

pub(crate) async fn run(
    store: PathBuf,
    request: Option<PathBuf>,
    output: Option<PathBuf>,
) -> anyhow::Result<()> {
    let request: SitemapRawReadRequest = match request {
        Some(path) => read_request(&path).await?,
        None => Default::default(),
    };
    request.validate()?;
    if let Some(path) = &output {
        anyhow::ensure!(
            !tokio::fs::try_exists(path).await?,
            "Raw sitemap JSON destination must be a new file"
        );
    }
    let window = tokio::task::spawn_blocking(move || {
        CrawlStore::open_read_only(store)?.sitemap_raw_window(&request)
    })
    .await??;
    // The Core checks this exact compact JSON envelope against its page cap.
    let bytes = serde_json::to_vec(&window)?;
    match output {
        Some(path) => crate::write_new_report(&path, &bytes).await?,
        None => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(&bytes)?;
            stdout.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "sitemap_raw_cli_tests.rs"]
mod tests;
