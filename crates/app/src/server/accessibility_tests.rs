use super::*;

fn valid() -> Value {
    json!({"engine":"axe-core","version":"4.14.0","automated_only":true,
        "violations":[],"incomplete":[],"passes_count":12,"inapplicable_count":34})
}
fn finding() -> Value {
    json!({"id":"=unsafe-rule","impact":"serious","description":"Review 雪","help":"A readable rule",
        "helpUrl":"https://owned.invalid/help","tags":["wcag2a","wcag111"],"nodes":[{
            "target":[["#shadow","img"]],"html":"<img onerror=\"window.injected=true\">",
            "failureSummary":"Check this element\nwith its context","any":[{"id":"image-alt","impact":"serious","message":"Name is missing","data":{"original":true},"relatedNodes":[]}],"all":[],"none":[]}]})
}
fn page(raw: Option<Value>) -> PageRecord {
    PageRecord {
        url: "https://owned.invalid/雪".into(),
        accessibility: raw,
        ..Default::default()
    }
}

#[test]
fn accessibility_unknown_is_not_zero_and_needs_review_is_not_a_violation_or_compliance_claim() {
    let zero = page(Some(valid()));
    let observed = observation(&zero);
    assert_eq!(observed.status, "observed_zero");
    assert_eq!(observed.violations.unwrap().len(), 0);
    assert_eq!(observed.incomplete.unwrap().len(), 0);
    assert_eq!(observed.passes, Some(12));
    let mut review = valid();
    review["incomplete"] = json!([finding()]);
    let review = page(Some(review));
    let observed = observation(&review);
    assert_eq!(observed.status, "needs_review");
    assert_eq!(observed.violations.unwrap().len(), 0);
    assert_eq!(observed.incomplete.unwrap().len(), 1);
    let mut malformed = vec![
        Value::Null,
        json!([]),
        json!({}),
        json!({"error":"actual engine error","violations":[],"incomplete":[]}),
    ];
    for (key, value) in [
        ("automated_only", json!(false)),
        ("automated_only", Value::Null),
        ("engine", json!("manual")),
        ("violations", Value::Null),
        ("incomplete", json!({})),
        ("passes_count", json!(-1)),
        ("passes_count", json!(9_007_199_254_740_992_u64)),
    ] {
        let mut raw = valid();
        raw[key] = value;
        malformed.push(raw);
    }
    let mut invalid_rule = valid();
    let mut bad = finding();
    bad["nodes"][0]["any"][0]["message"] = json!({"malformed":true});
    invalid_rule["violations"] = json!([bad]);
    malformed.push(invalid_rule);
    for raw in malformed {
        let unknown = page(Some(raw));
        let observed = observation(&unknown);
        assert!(matches!(observed.status, "error" | "malformed"));
        assert!(
            observed.violations.is_none()
                && observed.incomplete.is_none()
                && observed.passes.is_none()
                && observed.inapplicable.is_none()
        );
    }
    assert_eq!(observation(&page(None)).status, "missing");
    let mut failed = page(None);
    failed.renderer_error = Some("real renderer failure".into());
    assert_eq!(observation(&failed).status, "renderer_error");
}

fn parse(input: &[u8]) -> Vec<Vec<String>> {
    let text = std::str::from_utf8(input).unwrap();
    let mut values = Vec::new();
    let mut record = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if quoted && chars.peek() == Some(&'"') {
                    chars.next();
                    cell.push('"');
                } else {
                    quoted = !quoted;
                }
            }
            ',' if !quoted => record.push(std::mem::take(&mut cell)),
            '\r' if !quoted => {
                assert_eq!(chars.next(), Some('\n'));
                record.push(std::mem::take(&mut cell));
                values.push(std::mem::take(&mut record));
            }
            c => cell.push(c),
        }
    }
    assert!(!quoted && cell.is_empty() && record.is_empty());
    values
}

#[test]
fn accessibility_complete_csv_preserves_saved_details_unknowns_profile_external_pages_and_atomic_budgets()
 {
    let mut raw = valid();
    raw["incomplete"] = json!([finding()]);
    let mut report = CrawlReport {
        pages: vec![page(Some(raw.clone())), page(None), page(Some(valid()))],
        ..Default::default()
    };
    report.pages[1].url = "https://owned.invalid/missing".into();
    report.pages[2].url = "https://cdn.invalid/zero".into();
    report.pages[2].internal = false;
    let bytes = export(&report, CSV_BYTES, PAGE_BYTES).unwrap();
    let rows = parse(&bytes);
    assert_eq!(rows[0], HEADERS);
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[1][9], "needs_review");
    assert_eq!(&rows[1][11..15], ["0", "1", "12", "34"]);
    assert_eq!(serde_json::from_str::<Value>(&rows[1][28]).unwrap(), raw);
    assert_eq!(rows[2][15], "needs_review");
    assert_eq!(rows[2][16], "'=unsafe-rule");
    assert_eq!(
        serde_json::from_str::<Value>(&rows[2][23]).unwrap(),
        raw["incomplete"][0]["nodes"][0]["target"]
    );
    let checks: Value = serde_json::from_str(&rows[2][26]).unwrap();
    assert_eq!(checks["any"][0]["data"], json!({"original":true}));
    assert_eq!(rows[2][24], "<img onerror=\"window.injected=true\">");
    assert_eq!(rows[3][9], "missing");
    assert!(rows[3][11..15].iter().all(String::is_empty));
    assert_eq!(rows[4][0], "https://cdn.invalid/zero");
    assert_eq!(&rows[4][11..13], ["0", "0"]);
    assert_eq!(export(&report, bytes.len(), PAGE_BYTES).unwrap(), bytes);
    assert!(
        export(&report, bytes.len() - 1, PAGE_BYTES)
            .unwrap_err()
            .to_string()
            .contains("budget")
    );
    assert!(
        export(&report, CSV_BYTES, 64)
            .unwrap_err()
            .to_string()
            .contains("budget")
    );
    // Measure escaped JSON bytes, not the shorter original string length.
    report.pages[0].accessibility = Some(json!({"error":"\0".repeat(100)}));
    assert!(export(&report, CSV_BYTES, 250).is_err());
    report.pages[0] = page(Some(raw));
    report.pages[0].meta_robots = vec!["noindex".into()];
    report.config.directive_reporting = Some(open_seo_spider_core::DirectiveReportingConfig {
        respect_noindex: true,
        ..Default::default()
    });
    let filtered = parse(&export(&report, CSV_BYTES, PAGE_BYTES).unwrap());
    assert_eq!(filtered.len(), 3);
    assert!(
        filtered
            .iter()
            .skip(1)
            .all(|row| row[0] != "https://owned.invalid/雪")
    );
    assert_eq!(&filtered[1][29..32], ["3", "2", "1"]);
}

#[tokio::test]
async fn accessibility_saved_handler_rejects_active_stale_oversize_and_returns_verified_full_csv() {
    let temp = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let state = AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Default::default(),
        data_dir: Arc::new(temp.path().to_path_buf()),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle: Default::default(),
    };
    let report = CrawlReport {
        pages: (0..205)
            .map(|index| PageRecord {
                url: format!("https://owned.invalid/{index}"),
                accessibility: Some(valid()),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    register_report(&state, id, report).await;
    let job = get_job(&state, id).await.ok().unwrap();
    let generation = job.read().await.generation;
    let response = csv(
        State(state.clone()),
        Path(id),
        Query(ExportQuery {
            generation: Some(generation),
        }),
    )
    .await
    .unwrap_or_else(|(status, Json(error))| {
        panic!("Accessibility export failed with {status}: {}", error.error)
    });
    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), CSV_BYTES)
        .await
        .unwrap();
    assert_eq!(parse(&bytes).len(), 206);
    assert_eq!(headers[header::CONTENT_LENGTH], bytes.len().to_string());
    assert_eq!(
        headers["x-content-sha256"],
        open_seo_spider_core::api_enrichment::api_sha(&bytes)
    );
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        csv(
            State(state.clone()),
            Path(id),
            Query(ExportQuery {
                generation: Some(generation + 1)
            })
        )
        .await
        .unwrap_err()
        .0,
        StatusCode::CONFLICT
    );
    job.write().await.status = "running".into();
    assert_eq!(
        csv(
            State(state.clone()),
            Path(id),
            Query(ExportQuery::default())
        )
        .await
        .unwrap_err()
        .0,
        StatusCode::CONFLICT
    );
    {
        let mut job = job.write().await;
        job.status = "completed".into();
        job.report.as_mut().unwrap().pages[0].accessibility =
            Some(json!({"error":"x".repeat(PAGE_BYTES)}));
    }
    let error = csv(
        State(state.clone()),
        Path(id),
        Query(ExportQuery::default()),
    )
    .await
    .unwrap_err();
    assert_eq!(error.0, StatusCode::PAYLOAD_TOO_LARGE);
    lifecycle::close_admission(&state);
    lifecycle::drain(&state).await.unwrap();
}

#[tokio::test]
async fn accessibility_native_handler_actual_64mib_exact_and_plus_one_refuse_atomically() {
    // Synthetic saved error observations test the real physical output ceiling,
    // not an axe execution, saved-origin attestation or a reduced-cap analogue.
    let mut report = CrawlReport {
        pages: (0..21)
            .map(|index| PageRecord {
                url: format!("https://budget-control.invalid/{index}"),
                accessibility: Some(json!({"error":"x".repeat(3 * 1024 * 1024)})),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    report.pages.push(PageRecord {
        url: "https://budget-control.invalid/final".into(),
        accessibility: Some(json!({"error":""})),
        ..Default::default()
    });
    let baseline_length = export(&report, CSV_BYTES, PAGE_BYTES).unwrap().len();
    assert!(baseline_length < CSV_BYTES);
    let padding = CSV_BYTES - baseline_length;
    assert!(padding < PAGE_BYTES - 1024);
    let Value::String(last_error) = report
        .pages
        .last_mut()
        .unwrap()
        .accessibility
        .as_mut()
        .unwrap()
        .get_mut("error")
        .unwrap()
    else {
        panic!("error string")
    };
    last_error.push_str(&"x".repeat(padding));
    let temp = tempfile::tempdir().unwrap();
    let state = AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Default::default(),
        data_dir: Arc::new(temp.path().to_path_buf()),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle: Default::default(),
    };
    let id = Uuid::new_v4();
    register_report(&state, id, CrawlReport::default()).await;
    let job = get_job(&state, id).await.ok().unwrap();
    job.write().await.report = Some(report);
    let response = csv(
        State(state.clone()),
        Path(id),
        Query(ExportQuery::default()),
    )
    .await
    .unwrap_or_else(|(status, Json(error))| {
        panic!("Accessibility export failed with {status}: {}", error.error)
    });
    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers().clone();
    assert_eq!(headers[header::CONTENT_LENGTH], CSV_BYTES.to_string());
    let bytes = axum::body::to_bytes(response.into_body(), CSV_BYTES)
        .await
        .unwrap();
    assert_eq!(bytes.len(), CSV_BYTES);
    assert_eq!(
        headers["x-content-sha256"],
        open_seo_spider_core::api_enrichment::api_sha(&bytes)
    );
    assert_eq!(
        std::str::from_utf8(&bytes)
            .unwrap()
            .match_indices("\",\"page_summary\",\"")
            .count(),
        22
    );
    assert_eq!(bytes.iter().filter(|byte| **byte == b'\n').count(), 23);
    drop(bytes);
    {
        let mut guard = job.write().await;
        let Value::String(last_error) = guard
            .report
            .as_mut()
            .unwrap()
            .pages
            .last_mut()
            .unwrap()
            .accessibility
            .as_mut()
            .unwrap()
            .get_mut("error")
            .unwrap()
        else {
            panic!("error string")
        };
        last_error.push('x');
    }
    let error = csv(
        State(state.clone()),
        Path(id),
        Query(ExportQuery::default()),
    )
    .await
    .unwrap_err();
    assert_eq!(error.0, StatusCode::PAYLOAD_TOO_LARGE);
    let refusal = error.into_response();
    assert!(!refusal.headers().contains_key(header::CONTENT_DISPOSITION));
    assert!(!refusal.headers().contains_key("x-content-sha256"));
    assert_eq!(refusal.headers()[header::CONTENT_TYPE], "application/json");
    lifecycle::close_admission(&state);
    lifecycle::drain(&state).await.unwrap();
}
