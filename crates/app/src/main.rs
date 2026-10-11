mod analysis;
mod mcp;
mod server;

use clap::{Parser, Subcommand};
use open_seo_spider_core::{CrawlConfig, Crawler, export_pages_csv, export_sitemap_xml};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

#[derive(Parser)]
#[command(
    name = "open-seo-spider",
    version,
    about = "Independent local SEO crawler. No commercial URL cap."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Serve MCP tools over stdio against the running local application.
    Mcp {
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        server: String,
    },
    /// Launch the local browser application.
    Serve {
        #[arg(long, default_value = "127.0.0.1:8787")]
        listen: String,
        #[arg(long, default_value = "data")]
        data_dir: PathBuf,
    },
    /// Crawl without the browser application and export the complete report.
    Crawl {
        urls: Vec<String>,
        #[arg(long)]
        sitemap: Vec<String>,
        #[arg(long)]
        list: bool,
        #[arg(long, default_value_t = 8)]
        concurrency: usize,
        #[arg(long)]
        max_pages: Option<usize>,
        #[arg(long, default_value = "crawl.json")]
        output: PathBuf,
        #[arg(long)]
        csv: Option<PathBuf>,
        #[arg(long)]
        xml_sitemap: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "open_seo_spider=info".into()),
        )
        .init();
    match Cli::parse().command.unwrap_or(Command::Serve {
        listen: "127.0.0.1:8787".into(),
        data_dir: "data".into(),
    }) {
        Command::Serve { listen, data_dir } => server::serve(&listen, data_dir).await?,
        Command::Mcp { server } => mcp::stdio(&server).await?,
        Command::Crawl {
            urls,
            sitemap,
            list,
            concurrency,
            max_pages,
            output,
            csv,
            xml_sitemap,
        } => {
            let config = CrawlConfig {
                seed_urls: urls,
                sitemap_urls: sitemap,
                crawl_links: !list,
                concurrency,
                max_pages: max_pages.filter(|value| *value > 0),
                ..Default::default()
            };
            let mut report = Crawler::new(config)?
                .run(None, Arc::new(AtomicBool::new(false)))
                .await?;
            analysis::extend_report(&mut report);
            tokio::fs::write(&output, serde_json::to_vec_pretty(&report)?).await?;
            if let Some(path) = csv {
                tokio::fs::write(path, export_pages_csv(&report)?).await?;
            }
            if let Some(path) = xml_sitemap {
                tokio::fs::write(path, export_sitemap_xml(&report)).await?;
            }
            println!(
                "{} URLs, {} issues. Report: {}",
                report.pages.len(),
                report.issues.len(),
                output.display()
            );
        }
    }
    Ok(())
}
