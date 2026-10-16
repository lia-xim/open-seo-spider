//! AUTHORED_NOT_RUN. Existing --config interface, unchanged file input bytes.
use super::*;

#[tokio::test]
async fn cdn_directory_cli_actual_config_input_keeps_raw_rules_and_cannot_grant_network_permissions()
 {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("config.json");
    let value = serde_json::json!({"seed_urls":["https://www.example/"],"cdn_directory_reporting":{"version":1,"rules":["HTTPS://WWW.CDN.example/assets/","http://files.example:8443/"]}});
    let bytes = serde_json::to_vec(&value).unwrap();
    std::fs::write(&input, &bytes).unwrap();
    let cli = Cli::try_parse_from([
        "open-seo-spider",
        "crawl",
        "--config",
        input.to_str().unwrap(),
    ])
    .unwrap();
    let path = match cli.command.unwrap() {
        Command::Crawl {
            config: Some(path), ..
        } => path,
        _ => panic!("actual config file required"),
    };
    let c: CrawlConfig =
        serde_json::from_slice(&read_bounded(&path, 32 * 1024 * 1024).await.unwrap()).unwrap();
    let crawler = Crawler::new(c).unwrap();
    assert_eq!(
        serde_json::to_value(&crawler.effective_config().cdn_directory_reporting).unwrap(),
        value["cdn_directory_reporting"]
    );
    assert!(!crawler.effective_config().check_external);
    assert!(crawler.effective_config().resource_discovery.is_none());
    assert!(crawler.effective_config().cdn_classification.is_none());
    assert_eq!(std::fs::read(input).unwrap(), bytes);
}
