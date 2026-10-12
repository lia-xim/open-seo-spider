mod analysis;
mod mcp;
mod server;

use clap::{Parser, Subcommand};
use open_seo_spider_core::{
    CrawlConfig, Crawler, ListColumnSelector, ListCsvDelimiter, ListImportFormat, ListImportLimits,
    ListImportOptions, export_input_occurrences_csv, export_pages_csv, export_sitemap_xml,
    export_url_list_csv, import_url_list, inspect_xlsx_list,
};
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
    /// Parse an owned URL list locally; this command makes no website requests.
    ImportList {
        file: PathBuf,
        #[arg(long)]
        options: Option<PathBuf>,
        /// Inspect XLSX sheet metadata before selecting a column.
        #[arg(long)]
        inspect: bool,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        csv: Option<PathBuf>,
    },
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
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        list_file: Vec<PathBuf>,
        #[arg(long)]
        list_options: Option<PathBuf>,
        #[arg(long)]
        concurrency: Option<usize>,
        #[arg(long)]
        max_pages: Option<usize>,
        #[arg(long, default_value = "crawl.json")]
        output: PathBuf,
        #[arg(long)]
        csv: Option<PathBuf>,
        #[arg(long)]
        input_csv: Option<PathBuf>,
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
        Command::ImportList {
            file,
            options,
            inspect,
            output,
            csv,
        } => {
            let bytes = read_bounded(&file, 8 * 1024 * 1024).await?;
            let json = if inspect {
                anyhow::ensure!(
                    csv.is_none(),
                    "Inspection has no occurrence CSV; import a selected column first"
                );
                let limits = if let Some(path) = options.as_ref() {
                    serde_json::from_slice::<ListImportOptions>(&read_bounded(path, 65536).await?)?
                        .limits
                } else {
                    ListImportLimits::default()
                };
                serde_json::to_vec_pretty(&inspect_xlsx_list(&bytes, &limits)?)?
            } else {
                let options = list_options_for(&file, options.as_ref()).await?;
                let result = import_url_list(&bytes, &options)?;
                if let Some(path) = csv {
                    tokio::fs::write(path, export_url_list_csv(&result)?).await?;
                }
                serde_json::to_vec_pretty(&result)?
            };
            if let Some(path) = output {
                tokio::fs::write(path, json).await?;
            } else {
                println!("{}", String::from_utf8(json)?);
            }
        }
        Command::Crawl {
            urls,
            sitemap,
            list,
            config,
            list_file,
            list_options,
            concurrency,
            max_pages,
            output,
            csv,
            input_csv,
            xml_sitemap,
        } => {
            let mut config = if let Some(path) = config {
                serde_json::from_slice::<CrawlConfig>(
                    &read_bounded(&path, 32 * 1024 * 1024).await?,
                )?
            } else {
                CrawlConfig::default()
            };
            if !config.seed_urls.is_empty() && config.input_imports.is_empty() {
                config.prepare_input_lists()?;
            }
            if !urls.is_empty() {
                config.input_imports.push(import_url_list(
                    urls.join("\n").as_bytes(),
                    &ListImportOptions {
                        format: ListImportFormat::Txt,
                        limits: ListImportLimits::default(),
                    },
                )?);
                config.seed_urls.extend(urls);
            }
            config.sitemap_urls.extend(sitemap);
            if list || !list_file.is_empty() {
                config.crawl_links = false;
            }
            if let Some(value) = concurrency {
                config.concurrency = value;
            }
            if let Some(value) = max_pages {
                config.max_pages = (value > 0).then_some(value);
            }
            for path in list_file {
                let options = list_options_for(&path, list_options.as_ref()).await?;
                let bytes = read_bounded(&path, options.limits.max_input_bytes).await?;
                config
                    .input_imports
                    .push(import_url_list(&bytes, &options)?);
            }
            config.prepare_input_lists()?;
            let mut report = Crawler::new(config)?
                .run(None, Arc::new(AtomicBool::new(false)))
                .await?;
            analysis::extend_report(&mut report);
            tokio::fs::write(&output, serde_json::to_vec_pretty(&report)?).await?;
            if let Some(path) = csv {
                tokio::fs::write(path, export_pages_csv(&report)?).await?;
            }
            if let Some(path) = input_csv {
                tokio::fs::write(path, export_input_occurrences_csv(&report)?).await?;
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

async fn read_bounded(path: &std::path::Path, limit: usize) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(
        tokio::fs::metadata(path).await?.len() <= limit as u64,
        "{} exceeds input byte limit {limit}",
        path.display()
    );
    let bytes = tokio::fs::read(path).await?;
    anyhow::ensure!(
        bytes.len() <= limit,
        "Input changed beyond its byte limit during read"
    );
    Ok(bytes)
}
async fn list_options_for(
    file: &std::path::Path,
    options: Option<&PathBuf>,
) -> anyhow::Result<ListImportOptions> {
    if let Some(path) = options {
        return Ok(serde_json::from_slice(&read_bounded(path, 65536).await?)?);
    }
    let format = match file
        .extension()
        .and_then(|x| x.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("txt") => ListImportFormat::Txt,
        Some("csv") => ListImportFormat::Csv {
            delimiter: ListCsvDelimiter::Comma,
            column: ListColumnSelector::Index { value: 1 },
            start_row: 1,
        },
        Some("xml") => ListImportFormat::Xml,
        Some("xlsx") => anyhow::bail!(
            "XLSX requires explicit sheet/column --options or --list-options; use import-list --inspect to inspect sheets"
        ),
        _ => anyhow::bail!(
            "Specify --options or --list-options with an explicit supported input format"
        ),
    };
    Ok(ListImportOptions {
        format,
        limits: ListImportLimits::default(),
    })
}
