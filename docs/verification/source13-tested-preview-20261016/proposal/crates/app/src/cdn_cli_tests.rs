//! AUTHORED_NOT_RUN. Existing --config CLI read path, no new subcommand or synthetic crawl.
use super::*;

#[tokio::test]
async fn cdn_cli_actual_config_file_parser_preserves_declarations_and_input_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("config.json");
    let value = serde_json::json!({"seed_urls":["https://www.example/"],"cdn_classification":{"version":1,"origins":["https://cdn.example"]}});
    let before = serde_json::to_vec(&value).unwrap();
    std::fs::write(&input, &before).unwrap();
    let parsed = Cli::try_parse_from([
        "open-seo-spider",
        "crawl",
        "--config",
        input.to_str().unwrap(),
    ])
    .unwrap();
    let path = match parsed.command.unwrap() {
        Command::Crawl {
            config: Some(path), ..
        } => path,
        _ => panic!("actual --config path required"),
    };
    let config: CrawlConfig =
        serde_json::from_slice(&read_bounded(&path, 32 * 1024 * 1024).await.unwrap()).unwrap();
    let crawler = Crawler::new(config).unwrap();
    assert_eq!(
        serde_json::to_value(&crawler.effective_config().cdn_classification).unwrap(),
        value["cdn_classification"]
    );
    assert!(!crawler.effective_config().check_external);
    assert!(crawler.effective_config().resource_discovery.is_none());
    assert_eq!(std::fs::read(&input).unwrap(), before);
}
