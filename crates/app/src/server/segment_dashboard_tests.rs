use super::*;

fn page(url: &str, title: Option<&str>) -> PageRecord {
    PageRecord {
        url: url.into(),
        final_url: url.into(),
        status: Some(200),
        final_status: Some(200),
        content_type: "text/html; charset=utf-8".into(),
        internal: true,
        indexable: true,
        titles: title.into_iter().map(str::to_owned).collect(),
        fetched_at: "2026-10-18T00:00:00Z".into(),
        ..Default::default()
    }
}
fn report() -> CrawlReport {
    let mut pages: Vec<_> = (0..205)
        .map(|i| {
            page(
                &format!("https://x.test/p/{i:04}"),
                Some(&format!("pick {i}")),
            )
        })
        .collect();
    let mut duplicate = page("https://x.test/p/0000", Some("other duplicate observation"));
    duplicate.status = Some(404);
    duplicate.final_status = Some(404);
    pages.push(duplicate);
    let mut hidden = page("https://x.test/hidden", Some("pick hidden"));
    hidden.meta_robots = vec!["noindex".into()];
    hidden.effective_robots = vec!["noindex".into()];
    hidden.indexable = false;
    pages.push(hidden);
    pages.push(page("https://x.test/unknown", None));
    pages.push(page("https://x.test/unassigned", Some("other")));
    CrawlReport {
        config: CrawlConfig {
            directive_reporting: Some(core::DirectiveReportingConfig {
                respect_noindex: true,
                ..Default::default()
            }),
            ..Default::default()
        },
        pages,
        issues: vec![
            AuditIssue {
                code: "owned".into(),
                severity: core::Severity::Warning,
                url: "https://x.test/p/0000".into(),
                message: "original message".into(),
                related_urls: vec!["https://x.test/outside".into()],
            },
            AuditIssue {
                code: "hidden".into(),
                severity: core::Severity::Error,
                url: "https://x.test/hidden".into(),
                message: "hidden".into(),
                related_urls: vec![],
            },
            AuditIssue {
                code: "orphan".into(),
                severity: core::Severity::Notice,
                url: "https://x.test/uncaptured-owner".into(),
                message: "no saved owner".into(),
                related_urls: vec![],
            },
        ],
        summary: core::CrawlSummary {
            completed: 999,
            discovered: 1001,
            pending: 2,
            limited: true,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn request() -> Request {
    Request {
        rules: engine::Rules {
            version: 1,
            definitions: vec![engine::Definition {
                id: "pick".into(),
                name: "Pick".into(),
                join: engine::Join::All,
                conditions: vec![engine::Condition {
                    field: engine::Field::StoredTitle,
                    operator: engine::Operator::Contains,
                    value: "pick".into(),
                    case_insensitive: false,
                }],
            }],
        },
        selection: engine::Selection::Segment { id: "pick".into() },
        limit: 200,
        expected_generation: Some(7),
        ..Default::default()
    }
}
fn job(id: Uuid, report: CrawlReport) -> Job {
    Job {
        id,
        config: report.config.clone(),
        status: "completed".into(),
        started_at: String::new(),
        finished_at: None,
        pages: vec![],
        issues: vec![],
        queued: 0,
        discovery_pending: 0,
        in_flight: 0,
        warnings: vec![],
        error: None,
        report: Some(report),
        control: CrawlControl::new(),
        generation: 7,
        checkpoint: None,
        frontier_available: false,
        operation: None,
        recrawl_override: Default::default(),
        quota_increases: None,
        policy_ledger: None,
        resource_ledger: None,
    }
}
fn state(id: Uuid, report: CrawlReport) -> AppState {
    AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Arc::new(RwLock::new(HashMap::from([(
            id,
            Arc::new(RwLock::new(job(id, report))),
        )]))),
        data_dir: Arc::new(PathBuf::from("unused-segment-dashboard-read-only-fixture")),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle: Default::default(),
    }
}
async fn bytes(response: Response) -> Vec<u8> {
    let hash = response.headers()["x-content-sha256"]
        .to_str()
        .unwrap()
        .to_owned();
    let length = response.headers()[header::CONTENT_LENGTH]
        .to_str()
        .unwrap()
        .parse::<usize>()
        .unwrap();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let bytes = axum::body::to_bytes(response.into_body(), EXPORT_BYTES)
        .await
        .unwrap()
        .to_vec();
    assert_eq!(bytes.len(), length);
    assert_eq!(service::sha(&bytes), hash);
    bytes
}
async fn value(response: Response) -> Value {
    serde_json::from_slice(&bytes(response).await).unwrap()
}
fn refused<T>(result: DResult<T>) -> (StatusCode, Failure) {
    match result {
        Err((s, Json(e))) => (s, e),
        Ok(_) => panic!("Expected refusal"),
    }
}
fn bound(first: &Value, mut request: Request) -> Request {
    request.expected_snapshot_token =
        Some(first["context"]["snapshot_token"].as_str().unwrap().into());
    request
}

#[tokio::test]
async fn segment_dashboard_pages_are_original_ordinals_and_full_cohort_not_first_page() {
    let id = Uuid::new_v4();
    let original = report();
    let original_bytes = serde_json::to_vec(&original).unwrap();
    let state = state(id, original.clone());
    let first = value(
        preview(State(state.clone()), Path(id), Ok(Json(request())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(first["total"], 205);
    assert_eq!(first["rows"].as_array().unwrap().len(), 200);
    assert_eq!(first["cohort"]["summary"]["selected_pages"], 205);
    assert_eq!(first["cohort"]["summary"]["selected_issues"], 1);
    assert_eq!(first["cohort"]["summary"]["orphan_issues"], 1);
    assert_eq!(first["cohort"]["summary"]["html_pages"], 205);
    assert_eq!(first["cohort"]["charts"]["status"]["200"], 205);
    assert_eq!(first["cohort"]["charts"]["effective_status"]["200"], 205);
    assert_eq!(first["captured_crawl_summary"]["completed"], 999);
    assert_eq!(first["context"]["sql_attested"], false);
    assert_eq!(first["context"]["report_generation_attested"], false);
    let mut all = first["rows"].as_array().unwrap().clone();
    let mut continuation = bound(&first, request());
    continuation.cursor = Some(serde_json::from_value(first["next"].clone()).unwrap());
    let last = value(
        preview(State(state.clone()), Path(id), Ok(Json(continuation)))
            .await
            .unwrap(),
    )
    .await;
    assert!(last["next"].is_null());
    assert_eq!(last["rows"].as_array().unwrap().len(), 5);
    assert_eq!(last["cohort"], first["cohort"]);
    assert_eq!(last["context"], first["context"]);
    all.extend(last["rows"].as_array().unwrap().iter().cloned());
    for (ordinal, row) in all.iter().enumerate() {
        assert_eq!(row["page_ordinal"], ordinal);
        assert_eq!(row["membership"]["matched_indices"], json!([0]));
        assert_eq!(row["membership"]["unknown_indices"], json!([]));
        assert_eq!(
            row["page"],
            serde_json::to_value(&original.pages[ordinal]).unwrap()
        );
    }
    assert_eq!(
        all.iter()
            .filter(|row| row["page"]["url"] == "https://x.test/p/0000")
            .count(),
        1
    );
    let mut issues = bound(&first, request());
    issues.view = View::Issues;
    let owned = value(
        preview(State(state.clone()), Path(id), Ok(Json(issues)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        owned["rows"],
        serde_json::to_value(&original.issues[..1]).unwrap()
    );
    let full = value(
        json(
            State(state.clone()),
            Path(id),
            Ok(Json(bound(&first, request()))),
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(full["kind"], "segment_dashboard_cohort_not_raw_crawl");
    assert_eq!(full["pages"], Value::Array(all));
    assert_eq!(
        full["captured_config"],
        serde_json::to_value(&original.config).unwrap()
    );
    assert_eq!(
        full["captured_crawl_summary"],
        serde_json::to_value(&original.summary).unwrap()
    );
    assert!(full.get("summary").is_none());
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|_| panic!("fixture"));
    assert_eq!(
        serde_json::to_vec(source.read().await.report.as_ref().unwrap()).unwrap(),
        original_bytes
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}

#[tokio::test]
async fn segment_dashboard_profile_unknown_and_snapshot_controls_are_strict() {
    let id = Uuid::new_v4();
    let state = state(id, report());
    let first = value(
        preview(State(state.clone()), Path(id), Ok(Json(request())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        first["cohort"]["reporting"]["captured_page_observations"],
        209
    );
    assert_eq!(
        first["cohort"]["reporting"]["profile_visible_page_observations"],
        208
    );
    let mut raw = request();
    raw.reporting_mode = Some(engine::ReportingMode::Raw);
    let raw_first = value(
        preview(State(state.clone()), Path(id), Ok(Json(raw.clone())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(raw_first["total"], 206);
    assert_eq!(
        raw_first["context"]["typed_report_sha256"],
        first["context"]["typed_report_sha256"]
    );
    assert_ne!(
        raw_first["context"]["snapshot_token"],
        first["context"]["snapshot_token"]
    );
    raw.expected_snapshot_token = Some(first["context"]["snapshot_token"].as_str().unwrap().into());
    assert_eq!(
        refused(preview(State(state.clone()), Path(id), Ok(Json(raw))).await).0,
        StatusCode::CONFLICT
    );
    let mut unknown = request();
    unknown.selection = engine::Selection::Unknown;
    let unknown = value(
        preview(State(state.clone()), Path(id), Ok(Json(unknown)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(unknown["total"], 1);
    assert_eq!(unknown["rows"][0]["page_ordinal"], 207);
    assert_eq!(
        unknown["rows"][0]["membership"]["unknown_indices"],
        json!([0])
    );
    let mut unassigned = request();
    unassigned.selection = engine::Selection::Unassigned;
    let unassigned = value(
        preview(State(state.clone()), Path(id), Ok(Json(unassigned)))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        unassigned["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["page_ordinal"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![205, 208]
    );
    assert_eq!(
        refused(analyze(State(state.clone()), Path(id), Ok(Json(request()))).await).0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        refused(
            csv(
                State(state.clone()),
                Path((id, "inputs.csv".into())),
                Ok(Json(bound(&first, request())))
            )
            .await
        )
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut beyond = bound(&first, request());
    beyond.cursor = Some(service::Cursor {
        offset: 206,
        view: View::Pages,
        snapshot_token: first["context"]["snapshot_token"].as_str().unwrap().into(),
    });
    assert_eq!(
        refused(preview(State(state.clone()), Path(id), Ok(Json(beyond))).await).0,
        StatusCode::BAD_REQUEST
    );
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|_| panic!("fixture"));
    source.write().await.status = "running".into();
    assert_eq!(
        refused(preview(State(state.clone()), Path(id), Ok(Json(request()))).await)
            .1
            .code,
        "unstable_snapshot"
    );
    source.write().await.status = "completed".into();
    source.write().await.generation = 8;
    assert_eq!(
        refused(
            json(
                State(state.clone()),
                Path(id),
                Ok(Json(bound(&first, request())))
            )
            .await
        )
        .0,
        StatusCode::CONFLICT
    );
    source.write().await.generation = 7;
    source
        .write()
        .await
        .report
        .as_mut()
        .unwrap()
        .config
        .directive_reporting
        .as_mut()
        .unwrap()
        .respect_canonical = true;
    assert_eq!(
        refused(
            json(
                State(state.clone()),
                Path(id),
                Ok(Json(bound(&first, request())))
            )
            .await
        )
        .0,
        StatusCode::CONFLICT
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}

#[tokio::test]
async fn segment_dashboard_graph_and_fragments_keep_outside_captured_target_facts() {
    let source_url = "https://x.test/source";
    let outside_url = "https://x.test/outside";
    let mut source = page(source_url, Some("pick source"));
    source.links = vec![core::LinkRecord {
        url: outside_url.into(),
        kind: "anchor".into(),
        internal: true,
        fragment: Some("target".into()),
        resolved_href: Some(format!("{outside_url}#target")),
        ..Default::default()
    }];
    let mut outside = page(outside_url, Some("other target"));
    outside.links = vec![core::LinkRecord {
        url: source_url.into(),
        kind: "anchor".into(),
        internal: true,
        ..Default::default()
    }];
    outside.dom_metadata = Some(core::DomMetadata {
        complete: true,
        fragment_targets: vec![core::FragmentTarget {
            kind: "id".into(),
            value: "target".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    let report = CrawlReport {
        config: CrawlConfig {
            validate_fragments: true,
            ..Default::default()
        },
        pages: vec![source, outside],
        ..Default::default()
    };
    let original_graph = crate::analysis::analyze_report(&report)["graph"].clone();
    let id = Uuid::new_v4();
    let state = state(id, report);
    let first = value(
        preview(State(state.clone()), Path(id), Ok(Json(request())))
            .await
            .unwrap(),
    )
    .await;
    let q = bound(&first, request());
    let result = value(
        analyze(State(state.clone()), Path(id), Ok(Json(q.clone())))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        result["analysis"]["graph"]["nodes"],
        json!([original_graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["url"] == source_url)
            .unwrap()])
    );
    assert_eq!(
        result["analysis"]["graph"]["nodes"][0]["in_degree_nonself"],
        1
    );
    assert_eq!(
        result["analysis"]["graph"]["edges"][0]["source"],
        source_url
    );
    assert_eq!(
        result["analysis"]["graph"]["edges"][0]["target"],
        outside_url
    );
    assert_eq!(result["analysis"]["graph"]["orphan_candidates"], json!([]));
    assert_eq!(result["target_lookup"].as_array().unwrap().len(), 2);
    assert_eq!(result["target_lookup"][1]["url"], outside_url);
    assert_eq!(result["target_lookup"][1]["status"], 200);
    assert_eq!(
        result["fragments"]["observations"][0]["status"],
        "matched_id"
    );
    assert_eq!(
        result["fragments"]["observations"][0]["target_complete"],
        true
    );
    assert_eq!(result["responses"].as_array().unwrap().len(), 1);
    let data = bytes(
        csv(
            State(state.clone()),
            Path((id, "fragments.csv".into())),
            Ok(Json(q)),
        )
        .await
        .unwrap(),
    )
    .await;
    let mut reader = csv::Reader::from_reader(data.as_slice());
    let headers = reader.headers().unwrap().clone();
    let rows = reader.records().collect::<csv::Result<Vec<_>>>().unwrap();
    assert_eq!(rows.len(), 1);
    let status = headers.iter().position(|h| h == "Status").unwrap();
    assert_eq!(&rows[0][status], "matched_id");
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}

#[tokio::test]
async fn segment_dashboard_csv_writers_share_original_rows_context_and_profile_mode() {
    let id = Uuid::new_v4();
    let state = state(id, report());
    let mut q = request();
    q.reporting_mode = Some(engine::ReportingMode::Raw);
    let first = value(
        preview(State(state.clone()), Path(id), Ok(Json(q.clone())))
            .await
            .unwrap(),
    )
    .await;
    let q = bound(&first, q);
    for kind in [
        "pages.csv",
        "issues.csv",
        "links.csv",
        "dom.csv",
        "extractions.csv",
        "javascript.csv",
        "cookies.csv",
        "pdf.csv",
        "images.csv",
        "custom-search.csv",
        "accessibility.csv",
        "responses.csv",
        "fragments.csv",
    ] {
        let response = csv(
            State(state.clone()),
            Path((id, kind.into())),
            Ok(Json(q.clone())),
        )
        .await
        .unwrap();
        assert_eq!(
            response.headers()["x-segment-snapshot-token"],
            first["context"]["snapshot_token"].as_str().unwrap()
        );
        assert_eq!(response.headers()["x-segment-generation"], "7");
        assert!(
            response.headers()[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/csv")
        );
        let data = bytes(response).await;
        let mut reader = csv::Reader::from_reader(data.as_slice());
        let headers = reader.headers().unwrap().clone();
        let token = headers
            .iter()
            .position(|h| h == "Segment Snapshot Token")
            .unwrap();
        let selected = headers
            .iter()
            .position(|h| h == "Segment Selected Pages")
            .unwrap();
        let captured = headers
            .iter()
            .position(|h| h == "Segment Captured Pages")
            .unwrap();
        let mut count = 0;
        for row in reader.records() {
            let row = row.unwrap();
            count += 1;
            assert_eq!(
                &row[token],
                first["context"]["snapshot_token"].as_str().unwrap()
            );
            assert_eq!(&row[selected], "206");
            assert_eq!(&row[captured], "209");
            if kind == "accessibility.csv" {
                assert_eq!(
                    &row[headers.iter().position(|h| h == "Captured Pages").unwrap()],
                    "209"
                );
                assert_eq!(
                    &row[headers.iter().position(|h| h == "Visible Pages").unwrap()],
                    "208"
                );
            }
        }
        if kind == "pages.csv" || kind == "accessibility.csv" || kind == "responses.csv" {
            assert_eq!(count, 206, "{kind}");
        }
        if kind == "issues.csv" {
            assert_eq!(count, 2);
        }
    }
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}

#[test]
fn segment_dashboard_output_boundaries_and_analysis_preflight_are_atomic() {
    let report = report();
    let q = request();
    let e = service::evaluate(&report, &q, None, Some(7), "declared-budget-control").unwrap();
    for operation in [Operation::Page, Operation::Json] {
        let initial = match operation {
            Operation::Page => paged(&report, &e, &q, EXPORT_BYTES),
            _ => full(&report, &e, EXPORT_BYTES),
        }
        .unwrap();
        let at = match operation {
            Operation::Page => paged(&report, &e, &q, initial.len()),
            _ => full(&report, &e, initial.len()),
        }
        .unwrap();
        assert_eq!(at, initial);
        assert!(
            match operation {
                Operation::Page => paged(&report, &e, &q, initial.len() - 1),
                _ => full(&report, &e, initial.len() - 1),
            }
            .is_err()
        );
    }
    let exact = csv_bytes(&report, &e, CsvKind::Pages, EXPORT_BYTES).unwrap();
    assert_eq!(
        csv_bytes(&report, &e, CsvKind::Pages, exact.len()).unwrap(),
        exact
    );
    assert!(csv_bytes(&report, &e, CsvKind::Pages, exact.len() - 1).is_err());
    let called = std::cell::Cell::new(false);
    assert!(
        derive_analysis(&report, 1, |_| {
            called.set(true);
            json!({})
        })
        .is_err()
    );
    assert!(
        !called.get(),
        "Budget refusal must precede analysis derivation"
    );
    let result = derive_analysis(&report, ANALYSIS_WORK_BYTES, |_| {
        called.set(true);
        json!({"control":true})
    })
    .unwrap();
    assert!(called.get());
    assert_eq!(result["control"], true);
    assert!(CsvKind::parse("project.json").is_err());
}

#[test]
fn segment_dashboard_preflight_refuses_fragment_and_quadratic_groups_before_derivation() {
    let refusal = |report: CrawlReport| {
        service::report_hash(&report).expect("Declared fanout control fits the input ceiling");
        let called = std::cell::Cell::new(false);
        let error = derive_analysis(&report, ANALYSIS_WORK_BYTES, |_| {
            called.set(true);
            json!({})
        })
        .unwrap_err();
        assert!(error.to_string().contains("budget"), "{error}");
        assert!(
            !called.get(),
            "Fanout must refuse before the actual analyzer is called"
        );
    };
    let mut source = page("https://x.test/source", Some("pick"));
    source.links = vec![
        core::LinkRecord {
            url: "https://x.test/target".into(),
            fragment: Some("id".into()),
            kind: "anchor".into(),
            ..Default::default()
        };
        1000
    ];
    let mut target = page("https://x.test/target", Some("target"));
    target.dom_metadata = Some(core::DomMetadata {
        complete: true,
        fragment_targets: vec![core::FragmentTarget {
            kind: "id".into(),
            value: "id".into(),
            dom_location: core::DomLocation {
                path: Some("x".repeat(4 * 1024 * 1024)),
                ..Default::default()
            },
        }],
        ..Default::default()
    });
    refusal(CrawlReport {
        config: CrawlConfig {
            validate_fragments: true,
            ..Default::default()
        },
        pages: vec![source, target],
        ..Default::default()
    });
    let cycle: Vec<_> = (0..2000)
        .map(|i| {
            let mut p = page(&format!("https://x.test/cycle/{i:04}"), Some("pick"));
            p.canonical = vec![format!("https://x.test/cycle/{:04}", (i + 1) % 2000)];
            p
        })
        .collect();
    refusal(CrawlReport {
        pages: cycle,
        ..Default::default()
    });
    let keywords: Vec<_> = (0..2000)
        .map(|i| {
            let mut p = page(&format!("https://x.test/keywords/{i:04}"), Some("pick"));
            p.dom_metadata = Some(core::DomMetadata {
                complete: true,
                meta_keywords: vec![core::MetaKeywordRecord {
                    normalized: "owned duplicate".into(),
                    ..Default::default()
                }],
                ..Default::default()
            });
            p
        })
        .collect();
    refusal(CrawlReport {
        pages: keywords,
        ..Default::default()
    });
    let ordinary = CrawlReport {
        pages: (0..1005)
            .map(|i| {
                let mut p = page(
                    &format!("https://x.test/unique/{i:04}"),
                    Some(&format!("pick {i}")),
                );
                p.canonical = vec![p.url.clone()];
                p.dom_metadata = Some(core::DomMetadata {
                    complete: true,
                    meta_keywords: vec![core::MetaKeywordRecord {
                        normalized: format!("unique {i}"),
                        ..Default::default()
                    }],
                    ..Default::default()
                });
                p
            })
            .collect(),
        ..Default::default()
    };
    let called = std::cell::Cell::new(false);
    assert!(
        derive_analysis(&ordinary, ANALYSIS_WORK_BYTES, |_| {
            called.set(true);
            json!({})
        })
        .is_ok()
    );
    assert!(
        called.get(),
        "Unique pages must not be charged as one quadratic group"
    );
}

#[tokio::test]
async fn segment_dashboard_physical_64mib_complete_csv_and_plus_one_refusal() {
    // Explicitly declared synthetic amplification/budget control, not crawl-origin proof.
    let mut source = page("https://budget.test/source", Some("pick"));
    source.links = vec![
        core::LinkRecord {
            url: "https://budget.test/target".into(),
            kind: "anchor".into(),
            internal: true,
            ..Default::default()
        };
        45_000
    ];
    let mut report = CrawlReport {
        pages: vec![source],
        ..Default::default()
    };
    let id = Uuid::new_v4();
    let q = request();
    let e = service::evaluate(
        &report,
        &q,
        Some(id.to_string()),
        Some(7),
        "app_declared_imported_or_detached_report",
    )
    .unwrap();
    let selected = selected_report(&report, &e);
    let count = core::write_links_csv(
        &selected,
        Count {
            bytes: 0,
            cap: EXPORT_BYTES,
        },
        &extra(&report, &e).unwrap(),
    )
    .unwrap()
    .bytes;
    drop(selected);
    drop(e);
    assert!(count < EXPORT_BYTES);
    let padding = EXPORT_BYTES - count;
    report.pages[0].links.last_mut().unwrap().text = "a".repeat(padding);
    service::report_hash(&report)
        .expect("Physical export control remains within the input ceiling");
    let state = state(id, report);
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|_| panic!("fixture"));
    let token = segments::evaluate(&*source.read().await, &q)
        .unwrap()
        .context
        .snapshot_token;
    let mut bound = q.clone();
    bound.expected_snapshot_token = Some(token);
    let response = csv(
        State(state.clone()),
        Path((id, "links.csv".into())),
        Ok(Json(bound)),
    )
    .await
    .unwrap();
    assert_eq!(
        response.headers()[header::CONTENT_LENGTH],
        EXPORT_BYTES.to_string()
    );
    let data = bytes(response).await;
    assert_eq!(data.len(), EXPORT_BYTES);
    let mut reader = csv::Reader::from_reader(data.as_slice());
    let mut observed = 0;
    for row in reader.records() {
        row.unwrap();
        observed += 1;
    }
    assert_eq!(observed, 45_000);
    drop(reader);
    drop(data);
    source.write().await.report.as_mut().unwrap().pages[0]
        .links
        .last_mut()
        .unwrap()
        .text
        .push('a');
    let token = segments::evaluate(&*source.read().await, &q)
        .unwrap()
        .context
        .snapshot_token;
    let mut bound = q;
    bound.expected_snapshot_token = Some(token);
    let (status, error) = refused(
        csv(
            State(state.clone()),
            Path((id, "links.csv".into())),
            Ok(Json(bound)),
        )
        .await,
    );
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(error.code, "segment_dashboard_budget");
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}
