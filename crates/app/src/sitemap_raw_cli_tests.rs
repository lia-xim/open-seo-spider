//! AUTHORED / NOT RUN under coordinated compute hold. Actual CLI parser/handler.
use super::*;
use clap::Parser;

struct OwnedDirectory(PathBuf);
impl OwnedDirectory {
    fn new() -> Self {
        let base = std::fs::canonicalize(std::env::temp_dir()).unwrap();
        let path = base.join(format!("oss-raw-sitemap-cli-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        // This private value is created once above; no caller-supplied path.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn raw_sitemap_cli_parser_accepts_only_the_explicit_read_command_arguments() {
    let parsed = crate::Cli::try_parse_from([
        "open-seo-spider",
        "sitemap-rows",
        "crawl.sqlite",
        "--request",
        "page.json",
        "--output",
        "rows.json",
    ])
    .unwrap();
    assert!(
        matches!(parsed.command, Some(crate::Command::SitemapRows { store, request: Some(_), output: Some(_) }) if store == PathBuf::from("crawl.sqlite"))
    );
    assert!(
        crate::Cli::try_parse_from([
            "open-seo-spider",
            "sitemap-rows",
            "crawl.sqlite",
            "--csv",
            "rows.csv"
        ])
        .is_err()
    );
}

#[tokio::test]
async fn actual_raw_sitemap_cli_reads_unknown_history_and_refuses_existing_destination() {
    let temp = OwnedDirectory::new();
    let source = temp.path().join("historical.sqlite");
    let output = temp.path().join("rows.json");
    drop(CrawlStore::open(&source).unwrap());
    let before = std::fs::read(&source).unwrap();
    run(source.clone(), None, Some(output.clone()))
        .await
        .unwrap();
    let bytes = std::fs::read(&output).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        value["context"]["engine"],
        open_seo_spider_core::SITEMAP_RAW_READ_ENGINE
    );
    assert_eq!(
        value["context"]["scope"],
        "raw_rows_and_source_measurements_only"
    );
    assert!(value["context"]["ledger"].is_null());
    assert_eq!(value["rows"], serde_json::json!([]));
    assert_eq!(std::fs::read(&source).unwrap(), before);
    assert!(run(source, None, Some(output.clone())).await.is_err());
    assert_eq!(std::fs::read(output).unwrap(), bytes);
}

#[tokio::test]
async fn actual_raw_sitemap_cli_rejects_invalid_request_before_database_or_output_creation() {
    let temp = OwnedDirectory::new();
    let source = temp.path().join("must-not-be-created.sqlite");
    let request = temp.path().join("bad-request.json");
    let output = temp.path().join("must-not-be-created.json");
    for bad in [
        br#"{"limit":0}"#.as_slice(),
        br#"{"unexpected":true}"#.as_slice(),
        br#"{"after_sequence":1}"#.as_slice(),
    ] {
        tokio::fs::write(&request, bad).await.unwrap();
        let error = run(source.clone(), Some(request.clone()), Some(output.clone()))
            .await
            .unwrap_err();
        assert!(
            !error.to_string().contains("unable to open database"),
            "{error:#}"
        );
        assert!(!source.exists());
        assert!(!output.exists());
    }
}

#[tokio::test]
async fn actual_raw_sitemap_cli_request_file_accepts_exact_cap_and_refuses_sentinel_byte() {
    let temp = OwnedDirectory::new();
    let request = temp.path().join("bounded-request.json");
    let mut exact = b"{\"limit\":1}".to_vec();
    exact.resize(MAX_REQUEST_BYTES, b' ');
    tokio::fs::write(&request, &exact).await.unwrap();
    assert_eq!(read_request(&request).await.unwrap().limit, 1);
    exact.push(b' ');
    tokio::fs::write(&request, &exact).await.unwrap();
    assert!(
        read_request(&request)
            .await
            .unwrap_err()
            .to_string()
            .contains("exceeds 128 KiB")
    );
}
