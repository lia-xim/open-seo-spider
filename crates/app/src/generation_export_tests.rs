use super::*;
use clap::Parser;
use open_seo_spider_core::{CrawlConfig, PageRecord};
use sha2::{Digest, Sha256};

fn report() -> CrawlReport {
    CrawlReport {
        config: CrawlConfig::default(),
        summary: Default::default(),
        pages: vec![PageRecord {
            url: "https://example.org/p?a=1&b=2".into(),
            final_url: "https://example.org/p?a=1&b=2".into(),
            status: Some(200),
            final_status: Some(200),
            indexable: true,
            internal: true,
            content_type: "text/html".into(),
            ..Default::default()
        }],
        issues: vec![],
        warnings: vec![],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
        sitemap_source_measurements: None,
    }
}
fn directory() -> PathBuf {
    let path = std::env::temp_dir().join(format!("oss-generation-cli-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&path).unwrap();
    path
}
#[test]
fn generation_cli_requires_all_explicit_paths_and_rejects_unknown_flags() {
    for args in [
        vec!["oss", "generate-sitemaps"],
        vec![
            "oss",
            "generate-sitemaps",
            "crawl.json",
            "--options",
            "opts.json",
        ],
        vec![
            "oss",
            "generate-sitemaps",
            "crawl.json",
            "--output",
            "new.zip",
        ],
        vec![
            "oss",
            "generate-sitemaps",
            "crawl.json",
            "--options",
            "opts.json",
            "--output",
            "new.zip",
            "--force",
        ],
    ] {
        assert!(crate::Cli::try_parse_from(args).is_err());
    }
    let cli = crate::Cli::try_parse_from([
        "oss",
        "generate-sitemaps",
        "crawl.json",
        "--options",
        "opts.json",
        "--output",
        "new.zip",
    ])
    .unwrap();
    assert!(
        matches!(cli.command,Some(crate::Command::GenerateSitemaps { snapshot,options,output }) if snapshot==Path::new("crawl.json") && options==Path::new("opts.json") && output==Path::new("new.zip"))
    );
}
#[tokio::test]
async fn generation_cli_saved_inputs_unchanged_complete_zip_and_no_overwrite() {
    let dir = directory();
    let snapshot = dir.join("crawl.json");
    let options = dir.join("options.json");
    let output = dir.join("new.zip");
    let raw = serde_json::to_vec(&report()).unwrap();
    let opts = br#"{"public_base_url":"https://example.org/"}"#;
    tokio::fs::write(&snapshot, &raw).await.unwrap();
    tokio::fs::write(&options, opts).await.unwrap();
    let receipt = run(snapshot.clone(), options.clone(), output.clone())
        .await
        .unwrap();
    let bytes = tokio::fs::read(&output).await.unwrap();
    assert!(bytes.starts_with(b"PK\x03\x04"));
    assert_eq!(receipt.bytes, bytes.len());
    assert_eq!(receipt.sha256, format!("{:x}", Sha256::digest(&bytes)));
    assert_eq!(receipt.generated_urls, 1);
    assert!(receipt.temporary_cleanup_warning.is_none());
    assert!(
        run(snapshot.clone(), options.clone(), output.clone())
            .await
            .is_err()
    );
    assert!(
        run(snapshot.clone(), options.clone(), snapshot.clone())
            .await
            .is_err()
    );
    assert_eq!(raw, tokio::fs::read(snapshot).await.unwrap());
    assert_eq!(opts, tokio::fs::read(options).await.unwrap().as_slice());
    assert_eq!(bytes, tokio::fs::read(output).await.unwrap());
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 3);
    tokio::fs::remove_dir_all(dir).await.unwrap();
}
#[tokio::test]
async fn generation_invalid_options_precede_report_io_and_leave_no_output() {
    let dir = directory();
    let options = dir.join("options.json");
    tokio::fs::write(&options,br#"{"public_base_url":"https://example.org/","lastmod":{"source":"provided","values":[{"page_url":"https://example.org/p","value":"2026-02-30","reference":"owned-log"}]}}"#).await.unwrap();
    let error = run(dir.join("missing-report"), options, dir.join("none.zip"))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Lastmod"));
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    tokio::fs::remove_dir_all(dir).await.unwrap();
}
#[tokio::test]
async fn generation_atomic_concurrent_publication_has_one_whole_winner_and_cleans_links() {
    let dir = directory();
    let output = dir.join("new.zip");
    let a = vec![b'a'; 1_048_576];
    let b = vec![b'b'; 2_097_152];
    let (first, second) = tokio::join!(publish_new(&output, &a), publish_new(&output, &b));
    assert_ne!(first.is_ok(), second.is_ok());
    let actual = tokio::fs::read(&output).await.unwrap();
    assert!(actual == a || actual == b);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    tokio::fs::remove_dir_all(dir).await.unwrap();
}
#[tokio::test]
async fn generation_bounded_reader_rejects_exact_plus_one_and_nonregular_input() {
    let dir = directory();
    let path = dir.join("input");
    tokio::fs::write(&path, b"1234").await.unwrap();
    assert_eq!(bounded(&path, 4).await.unwrap(), b"1234");
    assert!(bounded(&path, 3).await.is_err());
    assert!(bounded(&dir, 4).await.is_err());
    tokio::fs::remove_dir_all(dir).await.unwrap();
}
