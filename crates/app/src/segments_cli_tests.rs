use clap::Parser;
#[test]
fn segments_cli_uses_explicit_report_request_output_and_never_creates_crawl_config() {
    let args = crate::Cli::try_parse_from([
        "spider",
        "saved-segments",
        "saved.json",
        "--request",
        "segments.json",
        "--output",
        "new.csv",
        "--csv",
    ])
    .unwrap();
    assert!(matches!(
        args.command,
        Some(crate::Command::SavedSegments { csv: true, .. })
    ));
    assert!(
        crate::Cli::try_parse_from([
            "spider",
            "saved-segments",
            "https://site/",
            "--seed-url",
            "https://site/",
            "--output",
            "new.json"
        ])
        .is_err()
    );
}
