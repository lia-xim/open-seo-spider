//! AUTHORED / NOT_COMPILED / NOT_RUN under HOLD.
use super::*;
use clap::Parser;
use serde_json::{Value, json};
#[path = "sitemap_findings_fixture.rs"]
pub(crate) mod fixture;
use fixture::{Fixture, OwnedDirectory};

#[test]
fn findings_cli_parser_requires_request_and_one_new_destination() {
    for suffix in [
        vec!["--output", "page.json"],
        vec!["--csv", "all.csv"],
        vec!["--output", "page.json", "--csv", "all.csv"],
    ] {
        let mut args = vec![
            "open-seo-spider",
            "sitemap-findings",
            "store.sqlite",
            "--request",
            "query.json",
        ];
        args.extend(suffix);
        assert!(matches!(
            crate::Cli::try_parse_from(args).unwrap().command,
            Some(crate::Command::SitemapFindings { .. })
        ));
    }
    for args in [
        vec![
            "open-seo-spider",
            "sitemap-findings",
            "db",
            "--request",
            "q",
        ],
        vec![
            "open-seo-spider",
            "sitemap-findings",
            "db",
            "--output",
            "out",
        ],
    ] {
        assert!(crate::Cli::try_parse_from(args).is_err());
    }
}

#[test]
fn findings_cli_actual_handle_request_exact_sixty_four_mib_and_sentinel() {
    let dir = OwnedDirectory::new();
    let path = dir.0.join("request.json");
    let mut file = File::create(&path).unwrap();
    file.write_all(b"{}").unwrap();
    let chunk = [b' '; 16384];
    let mut remaining = MAX_REQUEST_BYTES - 2;
    while remaining > 0 {
        let n = remaining.min(chunk.len());
        file.write_all(&chunk[..n]).unwrap();
        remaining -= n;
    }
    file.sync_all().unwrap();
    assert_eq!(file.metadata().unwrap().len(), MAX_REQUEST_BYTES as u64);
    assert!(bounded_request(&path).is_ok());
    file.write_all(b" ").unwrap();
    file.sync_all().unwrap();
    assert!(
        bounded_request(&path)
            .unwrap_err()
            .to_string()
            .contains("exceeds 64 MiB")
    );
}

#[tokio::test]
async fn findings_cli_invalid_query_precedes_database_and_preserves_existing_outputs() {
    let dir = OwnedDirectory::new();
    let request = dir.0.join("query.json");
    let db = dir.0.join("absent.sqlite");
    let output = dir.0.join("existing.json");
    fs::write(&output, b"previous bytes").unwrap();
    for bad in [
        json!({"limit":0}),
        json!({"unknown":true}),
        json!({"filter":"orphan-urls","source_task_url":"x"}),
        json!({"cursor":"malformed"}),
    ] {
        fs::write(&request, serde_json::to_vec(&bad).unwrap()).unwrap();
        let e = run(db.clone(), request.clone(), Some(output.clone()), None)
            .await
            .unwrap_err();
        assert!(!format!("{e:#}").contains("unable to open database"));
        assert!(!db.exists());
        assert_eq!(fs::read(&output).unwrap(), b"previous bytes");
    }
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
}

#[tokio::test]
async fn findings_cli_actual_crawler_page_and_full_csv_are_same_context_beyond_two_hundred() {
    let f = Fixture::actual(225).await;
    let request = f.dir.0.join("query.json");
    let json_path = f.dir.0.join("page.json");
    let csv = f.dir.0.join("full.csv");
    fs::write(&request, br#"{"filter":"urls-not-in-sitemap","limit":2}"#).unwrap();
    let before = fs::read(&f.path).unwrap();
    let receipt = run(
        f.path.clone(),
        request.clone(),
        Some(json_path.clone()),
        Some(csv.clone()),
    )
    .await
    .unwrap();
    let page: Value = serde_json::from_slice(&fs::read(&json_path).unwrap()).unwrap();
    assert_eq!(page["access_target"]["kind"], "offline");
    assert_eq!(page["findings"]["rows"].as_array().unwrap().len(), 2);
    assert!(page["next_cursor"].is_string());
    assert_eq!(
        receipt.csv_full_selection.as_ref().unwrap().data_rows,
        Some(224)
    );
    let prepared = bounded_request(&request)
        .unwrap()
        .prepare(&AccessTarget::Offline)
        .unwrap();
    let store = CrawlStore::open_read_only(&f.path).unwrap();
    let oracle = store
        .with_verified_sitemap_findings(
            prepared.selection(),
            None,
            &Default::default(),
            |session| {
                let mut bytes = Vec::new();
                assert_eq!(
                    sitemap_findings::write_findings_csv_session(session, &mut bytes)?,
                    224
                );
                Ok(bytes)
            },
        )
        .unwrap();
    assert_eq!(fs::read(&csv).unwrap(), oracle);
    assert_eq!(fs::read(&f.path).unwrap(), before);
    let first = fs::read(&json_path).unwrap();
    assert!(
        run(f.path.clone(), request, Some(json_path.clone()), None)
            .await
            .is_err()
    );
    assert_eq!(fs::read(&json_path).unwrap(), first);
    drop(store);
    f.finish().await;
}

#[tokio::test]
async fn findings_cli_continuation_still_exports_full_selection_and_changed_context_refuses_all_outputs()
 {
    let f = Fixture::actual(12).await;
    let query = f.dir.0.join("query.json");
    let first = f.dir.0.join("first.json");
    fs::write(&query, br#"{"filter":"urls-not-in-sitemap","limit":1}"#).unwrap();
    run(f.path.clone(), query.clone(), Some(first.clone()), None)
        .await
        .unwrap();
    let first: Value = serde_json::from_slice(&fs::read(first).unwrap()).unwrap();
    fs::write(
        &query,
        serde_json::to_vec(
            &json!({"filter":"urls-not-in-sitemap","limit":2,"cursor":first["next_cursor"]}),
        )
        .unwrap(),
    )
    .unwrap();
    let csv = f.dir.0.join("continued.csv");
    assert_eq!(
        run(f.path.clone(), query.clone(), None, Some(csv))
            .await
            .unwrap()
            .csv_full_selection
            .unwrap()
            .data_rows,
        Some(11)
    );
    // Actual missing terminal proof is an error/unavailable; never successful prefix CSV.
    rusqlite::Connection::open(&f.path)
        .unwrap()
        .execute("DELETE FROM metadata WHERE key='checkpoint'", [])
        .unwrap();
    let output = f.dir.0.join("must-not-exist.json");
    let csv = f.dir.0.join("must-not-exist.csv");
    assert!(
        run(
            f.path.clone(),
            query,
            Some(output.clone()),
            Some(csv.clone())
        )
        .await
        .is_err()
    );
    assert!(!output.exists() && !csv.exists());
    f.finish().await;
}

#[tokio::test]
async fn findings_cli_unknown_zero_source_keeps_context_and_unavailable_csv_publishes_nothing() {
    let dir = OwnedDirectory::new();
    let db = dir.0.join("historical.sqlite");
    drop(CrawlStore::open(&db).unwrap());
    let q = dir.0.join("query.json");
    fs::write(
        &q,
        br#"{"filter":"xml-sitemap-over-50mb","assessment":"unknown"}"#,
    )
    .unwrap();
    let json_path = dir.0.join("history.json");
    let csv = dir.0.join("history.csv");
    let receipt = run(
        db.clone(),
        q.clone(),
        Some(json_path.clone()),
        Some(csv.clone()),
    )
    .await
    .unwrap();
    assert_eq!(receipt.csv_full_selection.unwrap().data_rows, Some(0));
    let text = String::from_utf8(fs::read(csv).unwrap()).unwrap();
    assert!(text.contains("context") && text.contains("unknown_history"));
    // Saved-page domain has no actual terminal report in this historical DB.
    fs::write(&q, b"{}").unwrap();
    let csv = dir.0.join("unavailable.csv");
    assert!(run(db, q, None, Some(csv.clone())).await.is_err());
    assert!(!csv.exists());
}

#[test]
fn findings_cli_second_publication_race_retains_complete_first_and_concurrent_winner() {
    let dir = OwnedDirectory::new();
    let (mut first, mut a) = Staged::create(dir.0.join("first.json")).unwrap();
    a.write_all(b"COMPLETE JSON").unwrap();
    a.sync_all().unwrap();
    drop(a);
    first.receipt.bytes = 13;
    let (mut second, mut b) = Staged::create(dir.0.join("second.csv")).unwrap();
    b.write_all(b"COMPLETE CSV").unwrap();
    b.sync_all().unwrap();
    drop(b);
    second.receipt.bytes = 12;
    fs::write(&second.receipt.path, b"concurrent winner").unwrap();
    let message = format!(
        "{:#}",
        publish_outputs(StagedOutputs {
            json: Some(first),
            csv: Some(second)
        })
        .unwrap_err()
    );
    assert!(
        message.contains("Partially published output pair")
            && message.contains("first.json")
            && message.contains("COMPLETE first artifact retained")
    );
    assert_eq!(
        fs::read(dir.0.join("first.json")).unwrap(),
        b"COMPLETE JSON"
    );
    assert_eq!(
        fs::read(dir.0.join("second.csv")).unwrap(),
        b"concurrent winner"
    );
    assert!(
        fs::read_dir(&dir.0).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp"))
    );
}

#[test]
fn findings_cli_stage_cancel_and_json_budget_leave_no_final_or_temp_output() {
    let dir = OwnedDirectory::new();
    let output = dir.0.join("never.json");
    let (stage, file) = Staged::create(output.clone()).unwrap();
    let temp = stage.temporary.clone();
    let control = SitemapReadControl::default();
    let mut writer = Measured::new(file, MAX_JSON_BYTES, &control);
    let block = [b'x'; 16384];
    for _ in 0..MAX_JSON_BYTES / block.len() {
        writer.write_all(&block).unwrap();
    }
    assert!(writer.write_all(b"+").is_err());
    control.cancel();
    let before_digest = writer.digest();
    // Write::write_all skips Write::write for empty input. A real nonempty
    // write must observe cancellation before either the budget or filesystem.
    let error = writer.write_all(b"cancelled write").unwrap_err();
    assert_eq!(
        error.to_string(),
        open_seo_spider_core::SitemapReadControlError::Cancelled.to_string()
    );
    assert_eq!(writer.bytes, MAX_JSON_BYTES as u64);
    assert_eq!(writer.digest(), before_digest);
    assert_eq!(fs::metadata(&temp).unwrap().len(), MAX_JSON_BYTES as u64);
    drop(writer);
    drop(stage);
    assert!(!temp.exists() && !output.exists());
}
