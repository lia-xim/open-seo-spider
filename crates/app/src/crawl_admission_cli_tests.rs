//! AUTHORED / NOT_RUN. Actual Clap parser, not a parallel fake CLI.
use super::*;

#[test]
fn static_limits_cli_accepts_zero_only_for_folder_query_and_exposes_explicit_units() {
    let cli = Cli::try_parse_from([
        "open-seo-spider",
        "crawl",
        "https://example.test/",
        "--max-folder-depth",
        "0",
        "--max-query-tokens",
        "0",
        "--max-fetch-url-utf8-bytes",
        "8192",
    ])
    .unwrap();
    match cli.command.unwrap() {
        Command::Crawl {
            max_folder_depth,
            max_query_tokens,
            max_fetch_url_utf8_bytes,
            ..
        } => {
            assert_eq!(max_folder_depth, Some(0));
            assert_eq!(max_query_tokens, Some(0));
            assert_eq!(max_fetch_url_utf8_bytes, Some(8192));
        }
        _ => panic!("actual Crawl command required"),
    }
    assert!(
        Cli::try_parse_from([
            "open-seo-spider",
            "crawl",
            "https://example.test/",
            "--max-query-tokens",
            "-1"
        ])
        .is_err()
    );
    let absent =
        Cli::try_parse_from(["open-seo-spider", "crawl", "https://example.test/"]).unwrap();
    match absent.command.unwrap() {
        Command::Crawl {
            max_folder_depth,
            max_query_tokens,
            max_fetch_url_utf8_bytes,
            ..
        } => assert_eq!(
            (max_folder_depth, max_query_tokens, max_fetch_url_utf8_bytes),
            (None, None, None)
        ),
        _ => panic!("actual Crawl command required"),
    }
}
