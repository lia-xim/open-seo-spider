mod analysis;
mod mcp;
mod policy_export;
mod server;

use clap::{Parser, Subcommand};
use open_seo_spider_core::{
    CrawlConfig, Crawler, ListColumnSelector, ListCsvDelimiter, ListImportFormat, ListImportLimits,
    ListImportOptions, RecrawlSourceKind, SelectedRecrawlParent, export_input_occurrences_csv,
    export_pages_csv, export_sitemap_xml, export_url_list_csv, import_url_list, inspect_xlsx_list,
    prepare_selected_recrawl,
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
    /// Read native admission evidence locally or export every row in one read snapshot.
    PolicyLedger {
        store: PathBuf,
        #[arg(long)]
        after_sequence: Option<u64>,
        #[arg(long, default_value_t = 200)]
        limit: usize,
        /// New JSON destination for the complete snapshot; pagination options do not apply.
        #[arg(long)]
        json: Option<PathBuf>,
        /// New CSV destination for the complete snapshot; pagination options do not apply.
        #[arg(long)]
        csv: Option<PathBuf>,
    },
    /// Analyze supplied text locally; no website is fetched.
    AnalyzeContent {
        input: PathBuf,
        /// New output file; an existing file is refused.
        #[arg(long)]
        output: Option<PathBuf>,
    },
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
    /// Preview explicit scope, query and rewrite decisions locally; no crawl is started.
    PreviewPolicy {
        input: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Refresh selected exact requested URLs in a derived report; retain the source snapshot.
    Recrawl {
        snapshot: PathBuf,
        #[arg(required = true)]
        urls: Vec<String>,
        #[arg(long)]
        max_attempts: Option<usize>,
        /// Optional new SQLite destination; an existing file is refused.
        #[arg(long)]
        store: Option<PathBuf>,
        #[arg(long, default_value = "recrawl.json")]
        output: PathBuf,
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
        /// Durable SQLite frontier, required when the optional native policy is enabled.
        #[arg(long)]
        store: Option<PathBuf>,
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
        Command::PolicyLedger {
            store,
            after_sequence,
            limit,
            json,
            csv,
        } => {
            anyhow::ensure!((1..=1000).contains(&limit), "limit must be 1..1000");
            anyhow::ensure!(
                after_sequence.is_none() || (json.is_none() && csv.is_none()),
                "Complete policy exports cannot be combined with after_sequence"
            );
            let source = tokio::fs::canonicalize(&store).await?;
            let mut destinations = std::collections::HashSet::new();
            for path in json.iter().chain(csv.iter()) {
                let resolved = canonical_destination(path).await?;
                anyhow::ensure!(
                    resolved != source,
                    "Policy export cannot overwrite its SQLite source"
                );
                anyhow::ensure!(
                    destinations.insert(destination_identity(&resolved)),
                    "Policy JSON and CSV destinations must differ"
                );
                anyhow::ensure!(
                    !tokio::fs::try_exists(path).await?,
                    "Policy export destinations must be new files"
                );
            }
            let exporting = json.is_some() || csv.is_some();
            tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
                let database = open_seo_spider_core::CrawlStore::open_read_only(store)?;
                for (path, format) in json
                    .iter()
                    .map(|path| (path, policy_export::Format::Json))
                    .chain(csv.iter().map(|path| (path, policy_export::Format::Csv)))
                {
                    let file = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(path)?;
                    let mut writer = std::io::BufWriter::new(file);
                    policy_export::write_snapshot(&database, format, &mut writer, |_| Ok(()))?;
                }
                if !exporting {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &database.policy_occurrences(after_sequence, limit)?
                        )?
                    );
                }
                Ok(())
            })
            .await??;
        }
        Command::AnalyzeContent { input, output } => {
            let request: open_seo_spider_core::ContentAnalysisInput =
                serde_json::from_slice(&read_bounded(&input, 8 * 1024 * 1024).await?)?;
            let result = open_seo_spider_core::analyze_content(&request)?;
            let json = serde_json::to_vec_pretty(&result)?;
            if let Some(output) = output {
                let mut file = tokio::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(output)
                    .await?;
                tokio::io::AsyncWriteExt::write_all(&mut file, &json).await?;
            } else {
                println!("{}", String::from_utf8(json)?);
            }
        }
        Command::PreviewPolicy { input, output } => {
            let request: open_seo_spider_core::CrawlPolicyPreviewInput =
                serde_json::from_slice(&read_bounded(&input, 8 * 1024 * 1024).await?)?;
            let preview = open_seo_spider_core::preview_crawl_policy(&request)?;
            let json = serde_json::to_string_pretty(&preview)?;
            if let Some(output) = output {
                tokio::fs::write(output, json).await?;
            } else {
                println!("{json}");
            }
        }
        Command::Recrawl {
            snapshot,
            urls,
            max_attempts,
            store,
            output,
            csv,
        } => {
            let source = tokio::fs::canonicalize(&snapshot).await?;
            let mut destinations = std::collections::HashSet::new();
            for destination in std::iter::once(&output)
                .chain(csv.iter())
                .chain(store.iter())
            {
                let resolved = canonical_destination(destination).await?;
                anyhow::ensure!(
                    resolved != source,
                    "A selected recrawl destination cannot overwrite its parent snapshot"
                );
                anyhow::ensure!(
                    destinations.insert(destination_identity(&resolved)),
                    "Selected recrawl JSON, CSV and SQLite destinations must be distinct"
                );
                match tokio::fs::symlink_metadata(destination).await {
                    Ok(_) => anyhow::bail!(
                        "Selected recrawl destinations must be new files: {}",
                        destination.display()
                    ),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
            let bytes = read_bounded(&snapshot, 256 * 1024 * 1024).await?;
            let prepared = prepare_selected_recrawl(
                SelectedRecrawlParent {
                    job_id: "cli-json".into(),
                    checkpoint_revision: None,
                    source_kind: RecrawlSourceKind::StoredJson,
                },
                &bytes,
                &urls,
                max_attempts,
            )?;
            let mut crawler =
                Crawler::new(prepared.config.clone())?.with_selected_recrawl(prepared)?;
            if let Some(store) = store {
                // Reserve only a new destination, preserving an unrelated existing store.
                let reservation = tokio::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&store)
                    .await?;
                drop(reservation);
                crawler = crawler.with_store(store)?;
            }
            let mut report = crawler.run(None, Arc::new(AtomicBool::new(false))).await?;
            analysis::extend_report(&mut report);
            write_new_report(&output, &serde_json::to_vec_pretty(&report)?).await?;
            if let Some(csv) = csv {
                write_new_report(&csv, export_pages_csv(&report)?.as_bytes()).await?;
            }
            println!(
                "Refreshed {} selected targets; retained {} source observations; {} selected targets remain pending.",
                report
                    .operation
                    .as_ref()
                    .map_or(0, |operation| operation.refreshed_count),
                report
                    .operation
                    .as_ref()
                    .map_or(0, |operation| operation.retained_count),
                report
                    .operation
                    .as_ref()
                    .map_or(0, |operation| operation.pending_count)
            );
        }
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
            store,
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
            let mut crawler = Crawler::new(config)?;
            if let Some(store) = store {
                crawler = crawler.with_store(store)?;
            }
            let mut report = crawler.run(None, Arc::new(AtomicBool::new(false))).await?;
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

async fn write_new_report(path: &std::path::Path, bytes: &[u8]) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await?;
    file.write_all(bytes).await?;
    file.flush().await?;
    Ok(())
}

fn destination_identity(path: &std::path::Path) -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(path.as_os_str().to_string_lossy().to_lowercase())
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

async fn canonical_destination(path: &std::path::Path) -> anyhow::Result<PathBuf> {
    if let Ok(existing) = tokio::fs::canonicalize(path).await {
        return Ok(existing);
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Output destination needs a file name"))?;
    Ok(tokio::fs::canonicalize(parent).await?.join(name))
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
