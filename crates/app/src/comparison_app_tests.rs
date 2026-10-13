//! Shared request, export and local-file integrity controls.
use super::*;
use open_seo_spider_core::{ComparisonCollision, ComparisonIdentityOccurrence, PageRecord};

fn request() -> Value {
    json!({"before":"00000000-0000-0000-0000-000000000001",
        "after":"00000000-0000-0000-0000-000000000002"})
}
#[test]
fn strict_saved_request_rejects_unknown_and_invalid_nested_arguments() {
    for value in [
        json!({"before":"invalid","after":"00000000-0000-0000-0000-000000000002"}),
        {
            let mut value = request();
            value["extra"] = json!(true);
            value
        },
        {
            let mut value = request();
            value["config"] = json!({"unknown":true});
            value
        },
        {
            let mut value = request();
            value["config"] = json!({"limits":{"unknown":1}});
            value
        },
        {
            let mut value = request();
            value["expected_before_generation"] = json!(-1);
            value
        },
        {
            let mut value = request();
            value["expected_after_generation"] = json!(1.5);
            value
        },
        {
            let mut value = request();
            value["expected_after_generation"] = json!("1");
            value
        },
    ] {
        assert!(serde_json::from_value::<SavedComparisonRequest>(value).is_err());
    }
}
#[test]
fn explicit_zero_generation_and_optional_null_are_retained_without_invention() {
    let mut value = request();
    value["expected_before_generation"] = json!(0);
    value["config"] = Value::Null;
    let parsed: SavedComparisonRequest = serde_json::from_value(value).unwrap();
    assert_eq!(parsed.expected_before_generation, Some(0));
    assert_eq!(parsed.expected_after_generation, None);
    assert!(parsed.config.is_none());
    let binding = file_binding();
    assert!(binding.generation.is_none());
    assert!(binding.snapshot_sha256.is_none());
}
#[test]
fn structured_error_retains_every_collision_occurrence() {
    let original = ComparisonError {
        code: "ambiguous_snapshot_identity".into(),
        message: "collision".into(),
        collisions: vec![ComparisonCollision {
            side: ComparisonSide::Before,
            comparison_key: "https://fixture.test/".into(),
            occurrences: vec![
                ComparisonIdentityOccurrence {
                    ordinal: 0,
                    original_requested_url: "https://fixture.test/".into(),
                },
                ComparisonIdentityOccurrence {
                    ordinal: 1,
                    original_requested_url: "https://fixture.test/".into(),
                },
            ],
        }],
    };
    let envelope = SavedComparisonFailure::from(original.clone());
    let decoded: SavedComparisonFailure =
        serde_json::from_slice(&serde_json::to_vec(&envelope).unwrap()).unwrap();
    assert_eq!(decoded.error, original);
    assert_eq!(decoded.website_requests, 0);
    assert!(decoded.context.is_none());
}
#[test]
fn csv_escapes_formulas_quotes_commas_newlines_and_preserves_original_values() {
    let fields = ["=1+1", "\t @SUM(1,2)", "\u{feff}-1+2", "ä,\"日\"\n", ""].map(String::from);
    let original = fields.clone();
    let mut bytes = Vec::new();
    csv_record(&mut bytes, &fields).unwrap();
    assert_eq!(fields, original);
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "\"'=1+1\",\"'\t @SUM(1,2)\",\"'\u{feff}-1+2\",\"ä,\"\"日\"\"\n\",\"\"\r\n"
    );
}
#[test]
fn exact_output_budget_allows_boundary_and_refuses_next_byte() {
    let mut writer = ByteBudget {
        writer: Vec::new(),
        remaining: 3,
    };
    writer.write_all(b"abc").unwrap();
    assert!(writer.write_all(b"d").is_err());
    assert_eq!(writer.writer, b"abc");
}
struct OwnedFixture(PathBuf);
impl OwnedFixture {
    async fn new() -> Self {
        let path = std::env::temp_dir().join(format!("open-seo-spider-b11-{}", Uuid::new_v4()));
        tokio::fs::create_dir(&path).await.unwrap();
        Self(tokio::fs::canonicalize(path).await.unwrap())
    }
}
impl Drop for OwnedFixture {
    fn drop(&mut self) {
        if let Ok(temp) = std::fs::canonicalize(std::env::temp_dir())
            && self.0.starts_with(&temp)
            && self
                .0
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("open-seo-spider-b11-"))
        {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
#[tokio::test]
async fn existing_destination_is_refused_and_original_bytes_survive() {
    let fixture = OwnedFixture::new().await;
    let existing = fixture.0.join("existing.json");
    tokio::fs::write(&existing, b"keep exact bytes")
        .await
        .unwrap();
    let error = destination(&existing).await.unwrap_err();
    assert_eq!(error.error.code, "output_destination_exists");
    assert_eq!(
        tokio::fs::read(&existing).await.unwrap(),
        b"keep exact bytes"
    );
}
#[tokio::test]
async fn bounded_reader_refuses_plus_one_without_unbounded_read() {
    let fixture = OwnedFixture::new().await;
    let path = fixture.0.join("input.json");
    tokio::fs::write(&path, b"1234").await.unwrap();
    assert_eq!(read_bounded(&path, 4).await.unwrap(), b"1234");
    assert_eq!(
        read_bounded(&path, 3).await.unwrap_err().error.code,
        "saved_input_byte_budget"
    );
}
#[tokio::test]
async fn cli_json_csv_roundtrip_retains_unknown_states_and_declared_file_origin() {
    let fixture = OwnedFixture::new().await;
    let before = fixture.0.join("before.json");
    let after = fixture.0.join("after.json");
    let output = fixture.0.join("delta.json");
    let csv = fixture.0.join("delta.csv");
    let make = |title: &str| {
        let mut page = PageRecord {
            url: "https://fixture.test/page".into(),
            final_url: "https://fixture.test/page".into(),
            status: Some(200),
            final_status: Some(200),
            content_type: "text/html".into(),
            ..Default::default()
        };
        let seed = reqwest::Url::parse(&page.url).unwrap();
        open_seo_spider_core::extract_html(
            &mut page,
            &format!("<title>{title}</title><body>body</body>"),
            &[seed],
            false,
        );
        CrawlReport {
            config: Default::default(),
            pages: vec![page],
            issues: vec![],
            summary: Default::default(),
            warnings: vec![],
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
        }
    };
    tokio::fs::write(&before, serde_json::to_vec(&make("Old")).unwrap())
        .await
        .unwrap();
    tokio::fs::write(&after, serde_json::to_vec(&make("New")).unwrap())
        .await
        .unwrap();
    run_cli(before, after, None, Some(output.clone()), Some(csv.clone()))
        .await
        .unwrap();
    let parsed: SavedComparisonResponse =
        serde_json::from_slice(&tokio::fs::read(output).await.unwrap()).unwrap();
    assert_eq!(parsed.comparison.counts.changed, 1);
    assert_eq!(parsed.website_requests, 0);
    let binding = parsed.comparison.before_snapshot.caller_binding.unwrap();
    assert_eq!(
        binding.origin.as_deref(),
        Some("cli_declared_saved_json_file")
    );
    assert!(binding.generation.is_none());
    assert!(
        !parsed
            .comparison
            .before_snapshot
            .caller_binding_attested_by_core
    );
    let text = String::from_utf8(tokio::fs::read(csv).await.unwrap()).unwrap();
    assert!(text.contains("before_value_json"));
    assert!(text.contains("\"unknown\""));
}
