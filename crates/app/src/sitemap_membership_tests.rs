//! Pure protocol/projection controls, not stored-crawl authenticity evidence.
use super::*;
fn snapshot() -> Snapshot {
    Snapshot {
        job_id: Uuid::nil(),
        crawl_generation: 1,
        crawl_config_sha256: "a".repeat(64),
        catalog_config_sha256: "b".repeat(64),
        catalog_revision: 2,
        catalog_rolling_sha256: "c".repeat(64),
        catalog_state_sha256: "d".repeat(64),
        inherited_basis: None,
    }
}
#[test]
fn canonical_cursor_rejects_malformed_tampered_unknown_and_noncanonical_bytes() {
    let current = snapshot();
    let good = token(&current, &Filter::default(), Some(1)).unwrap();
    assert!(decode(&good).is_ok());
    for bad in [
        String::new(),
        "!".into(),
        "a".repeat(4097),
        format!("{good}="),
    ] {
        assert_eq!(decode(&bad).unwrap_err().code, "invalid_sitemap_cursor");
    }
    let mut value: serde_json::Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(&good).unwrap()).unwrap();
    value["extra"] = serde_json::json!(true);
    let unknown = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&value).unwrap());
    assert_eq!(decode(&unknown).unwrap_err().code, "invalid_sitemap_cursor");
    let bytes = URL_SAFE_NO_PAD.decode(&good).unwrap();
    let spaced = URL_SAFE_NO_PAD.encode([b" ".as_slice(), bytes.as_slice()].concat());
    assert_eq!(decode(&spaced).unwrap_err().code, "invalid_sitemap_cursor");
    let mut parsed = decode(&good).unwrap();
    parsed.payload.after_sequence = Some(2);
    let changed = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&parsed).unwrap());
    assert_eq!(decode(&changed).unwrap_err().code, "invalid_sitemap_cursor");
}
#[test]
fn cursor_pins_job_generation_config_revision_rolling_inheritance_and_filter() {
    let current = snapshot();
    let filter = Filter::default();
    let good = token(&current, &filter, Some(1)).unwrap();
    assert_eq!(
        validate_token(&good, &current, &filter, true).unwrap(),
        Some(1)
    );
    let mut variants = Vec::new();
    let mut next = current.clone();
    next.job_id = Uuid::new_v4();
    variants.push(next);
    let mut next = current.clone();
    next.crawl_generation += 1;
    variants.push(next);
    let mut next = current.clone();
    next.crawl_config_sha256 = "e".repeat(64);
    variants.push(next);
    let mut next = current.clone();
    next.catalog_config_sha256 = "e".repeat(64);
    variants.push(next);
    let mut next = current.clone();
    next.catalog_revision += 1;
    variants.push(next);
    let mut next = current.clone();
    next.catalog_rolling_sha256 = "e".repeat(64);
    variants.push(next);
    let mut next = current.clone();
    next.catalog_state_sha256 = "e".repeat(64);
    variants.push(next);
    let mut next = current.clone();
    next.inherited_basis = Some(SitemapInheritedBasis {
        parent_snapshot_sha256: "e".repeat(64),
        parent_catalog_sha256: "f".repeat(64),
        current_discovery_executed: false,
    });
    variants.push(next);
    for changed in variants {
        assert_eq!(
            validate_token(&good, &changed, &filter, true)
                .unwrap_err()
                .code,
            "stale_sitemap_cursor"
        );
    }
    let changed = Filter {
        kind: RowKind::Location,
        ..Default::default()
    };
    assert_eq!(
        validate_token(&good, &current, &changed, true)
            .unwrap_err()
            .code,
        "stale_sitemap_cursor"
    );
}
#[test]
fn query_bounds_reject_without_clamping_or_alias_normalization() {
    for limit in [0, 201, usize::MAX] {
        assert_eq!(
            ReadQuery {
                limit: Some(limit),
                ..Default::default()
            }
            .validate()
            .unwrap_err()
            .code,
            "invalid_sitemap_query"
        );
    }
    assert_eq!(ReadQuery::default().validate().unwrap(), 100);
    assert_eq!(
        ReadQuery {
            limit: Some(200),
            ..Default::default()
        }
        .validate()
        .unwrap(),
        200
    );
    for member in [
        "/relative",
        "https://e.test/p#fragment",
        " https://e.test/p ",
        "ftp://e.test/p",
        "https://user:secret@e.test/p",
        "https://e.test/\tbad",
    ] {
        assert_eq!(
            Filter {
                membership_url: Some(member.into()),
                ..Default::default()
            }
            .validate()
            .unwrap_err()
            .code,
            "invalid_sitemap_query"
        );
    }
    assert!(
        Filter {
            membership_url: Some("https://e.test/p?a=1&b=2".into()),
            ..Default::default()
        }
        .validate()
        .is_ok()
    );
    assert!(
        Filter {
            source_observation_id: Some("a".repeat(64)),
            ..Default::default()
        }
        .validate()
        .is_ok()
    );
    assert!(
        Filter {
            source_observation_id: Some("A".repeat(64)),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn saved_page_cursor_has_a_distinct_bounded_role_and_ordinal_zero_is_valid() {
    let current = snapshot();
    let page = page_token(&current, &Filter::default(), 0).unwrap();
    assert!(page.len() <= MAX_TOKEN_BYTES);
    assert_eq!(decode(&page).unwrap().payload.after_page_ordinal, Some(0));
    assert!(
        ReadQuery {
            page_cursor: Some(page.clone()),
            ..Default::default()
        }
        .analysis_request()
        .is_ok()
    );
    assert!(
        ReadQuery {
            cursor: Some(page.clone()),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        CsvQuery {
            kind: RowKind::All,
            source_observation_id: None,
            membership_url: None,
            snapshot: page.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        ReadQuery {
            page_cursor: Some(page),
            page_limit: Some(0),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        ReadQuery {
            page_limit: Some(201),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    let raw = token(&current, &Filter::default(), Some(1)).unwrap();
    assert!(
        ReadQuery {
            page_cursor: Some(raw),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}
#[test]
fn original_membership_filter_never_matches_policy_fetch_alias() {
    // Synthetic DTO filter control only; never passed as a validated SQL snapshot.
    let row = SitemapLedgerRow::Root(open_seo_spider_core::SitemapRootDeclaration {
        sequence: 1,
        declaration_id: "root".into(),
        kind: open_seo_spider_core::SitemapRootKind::ExplicitConfig,
        source_observation_id: None,
        source_ordinal: 0,
        raw_line: None,
        raw_value: "https://e.test/original".into(),
        normalized_document_url: Some("https://e.test/original".into()),
        robots_evidence: None,
        admission: open_seo_spider_core::SitemapAdmissionBinding {
            decision: open_seo_spider_core::SitemapAdmissionDecision::Scheduled,
            membership_url: Some("https://e.test/original".into()),
            admitted_fetch_url: Some("https://e.test/alias".into()),
            coalesced_policy_occurrence_id: None,
            policy_occurrence_id: Some("policy".into()),
            policy_source_revision: Some("revision".into()),
        },
    });
    assert!(
        Filter {
            membership_url: Some("https://e.test/original".into()),
            ..Default::default()
        }
        .matches(&row)
    );
    assert!(
        !Filter {
            membership_url: Some("https://e.test/alias".into()),
            ..Default::default()
        }
        .matches(&row)
    );
    assert!(
        !Filter {
            kind: RowKind::Location,
            ..Default::default()
        }
        .matches(&row)
    );
}
#[test]
fn csv_writer_is_exact_at_cap_rejects_plus_one_and_escapes_original_text() {
    let mut bounded = BoundedBytes {
        bytes: Vec::new(),
        max: 5,
    };
    bounded.write_all(b"12345").unwrap();
    assert!(bounded.write_all(b"6").is_err());
    assert_eq!(bounded.bytes, b"12345");
    let mut output = Vec::new();
    csv_record(
        &mut output,
        &[
            "ä,日\n\"x\"".into(),
            open_seo_spider_core::csv_text("=1+1"),
            "".into(),
        ],
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "\"ä,日\n\"\"x\"\"\",\"'=1+1\",\"\"\r\n"
    );
}

/// Small actual serde/CSV projection control. These DTOs are never submitted as
/// a positive stored-crawl witness, and this does not execute either native cap.
#[test]
fn budget_calibration_json_and_csv_original_quote_amplification() {
    assert_eq!(MAX_JSON_BYTES, 8_388_608);
    assert_eq!(MAX_CSV_BYTES, 67_108_864);
    fn record(quotes: usize, ascii: usize) -> (SitemapLedgerRow, Vec<u8>, Vec<u8>) {
        let raw = format!(
            "https://e.test/empty.xml#{}{}",
            "\"".repeat(quotes),
            "a".repeat(ascii)
        );
        let row = SitemapLedgerRow::Root(open_seo_spider_core::SitemapRootDeclaration {
            sequence: 1,
            declaration_id: "a".repeat(64),
            kind: open_seo_spider_core::SitemapRootKind::ExplicitConfig,
            source_observation_id: None,
            source_ordinal: 0,
            raw_line: None,
            raw_value: raw.clone(),
            normalized_document_url: Some("https://e.test/empty.xml".into()),
            robots_evidence: None,
            admission: open_seo_spider_core::SitemapAdmissionBinding {
                decision: open_seo_spider_core::SitemapAdmissionDecision::Scheduled,
                membership_url: Some("https://e.test/empty.xml".into()),
                admitted_fetch_url: Some("https://e.test/empty.xml".into()),
                coalesced_policy_occurrence_id: None,
                policy_occurrence_id: None,
                policy_source_revision: None,
            },
        });
        let json = encode(&row).unwrap();
        let fields = vec![
            "root".into(),
            "1".into(),
            String::new(),
            "a".repeat(64),
            "0".into(),
            raw,
            "https://e.test/empty.xml".into(),
            "https://e.test/empty.xml".into(),
            "scheduled".into(),
            String::from_utf8(json.clone()).unwrap(),
        ];
        let fields = fields
            .into_iter()
            .enumerate()
            .map(|(index, text)| {
                if matches!(index, 1 | 4) {
                    text
                } else {
                    open_seo_spider_core::csv_text(&text)
                }
            })
            .collect::<Vec<_>>();
        let mut csv = Vec::new();
        csv_record(&mut csv, &fields).unwrap();
        (row, json, csv)
    }
    let (_, json_base, csv_base) = record(128, 0);
    let (_, json_quote, csv_quote) = record(129, 0);
    let (_, json_ascii, csv_ascii) = record(128, 1);
    assert_eq!(json_quote.len() - json_base.len(), 2);
    assert_eq!(csv_quote.len() - csv_base.len(), 5);
    assert_eq!(json_ascii.len() - json_base.len(), 1);
    assert_eq!(csv_ascii.len() - csv_base.len(), 2);
    assert_eq!(
        serde_json::from_slice::<SitemapLedgerRow>(&json_quote).unwrap(),
        record(129, 0).0
    );
    assert!(csv_quote.ends_with(b"\r\n"));
    eprintln!(
        "BUDGET_CALIBRATION_CSV quotes=128 row_json_bytes={} csv_record_bytes={} quote_json_increment=2 quote_csv_increment=5 ascii_json_increment=1 ascii_csv_increment=2",
        json_base.len(),
        csv_base.len()
    );
}
#[test]
fn export_and_continuation_roles_and_zero_sequence_are_not_interchangeable() {
    let current = snapshot();
    let filter = Filter::default();
    let export = token(&current, &filter, None).unwrap();
    assert_eq!(
        validate_token(&export, &current, &filter, true)
            .unwrap_err()
            .code,
        "invalid_sitemap_cursor"
    );
    assert!(
        validate_token(&export, &current, &filter, false)
            .unwrap()
            .is_none()
    );
    let cursor = token(&current, &filter, Some(1)).unwrap();
    assert_eq!(
        validate_token(&cursor, &current, &filter, false)
            .unwrap_err()
            .code,
        "invalid_sitemap_cursor"
    );
    let zero = token(&current, &filter, Some(0)).unwrap();
    assert_eq!(
        ReadQuery {
            cursor: Some(zero),
            ..Default::default()
        }
        .validate()
        .unwrap_err()
        .code,
        "invalid_sitemap_cursor"
    );
}

// The tests below create a real crawler/SQLite source history. They never
// submit authored positive catalog/coverage DTOs to the projection.
struct LastmodWebsite {
    origin: String,
    requests: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    running: Option<tokio::task::JoinHandle<()>>,
}
impl Drop for LastmodWebsite {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
impl LastmodWebsite {
    async fn start(template: String) -> Self {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let xml = Arc::new(template.replace("{origin}", &origin));
        let requests = Arc::new(AtomicUsize::new(0));
        let count = requests.clone();
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
        let running = tokio::spawn(async move {
            let mut pending = tokio::task::JoinSet::new();
            loop {
                tokio::select! { biased;
                    _ = &mut stopped => break,
                    accepted = listener.accept() => {
                        let (mut socket, _) = accepted.unwrap();
                        count.fetch_add(1, Ordering::SeqCst);
                        let xml = xml.clone();
                        pending.spawn(async move {
                            let mut request = Vec::new();
                            let mut block = [0u8; 2048];
                            tokio::time::timeout(std::time::Duration::from_secs(3), async {
                                while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                                    let size = socket.read(&mut block).await.unwrap();
                                    assert!(size > 0);
                                    request.extend_from_slice(&block[..size]);
                                    assert!(request.len() <= 65536);
                                }
                            }).await.unwrap();
                            let text = String::from_utf8(request).unwrap();
                            let route = text.lines().next().unwrap().split_whitespace().nth(1).unwrap();
                            let (body, mime) = if route == "/source.xml" {
                                (xml.as_str(), "application/xml")
                            } else {
                                ("<html><head><title>Own lastmod fixture</title></head><body>Own local fixture</body></html>", "text/html")
                            };
                            let headers = format!("HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                            socket.write_all(headers.as_bytes()).await.unwrap();
                            socket.write_all(body.as_bytes()).await.unwrap();
                            socket.shutdown().await.unwrap();
                        });
                    }
                }
            }
            drop(listener);
            while let Some(result) = pending.join_next().await {
                result.unwrap();
            }
        });
        Self {
            origin,
            requests,
            stop: Some(stop),
            running: Some(running),
        }
    }
    fn count(&self) -> usize {
        self.requests.load(std::sync::atomic::Ordering::SeqCst)
    }
    async fn finish(mut self, expected: usize) {
        let _ = self.stop.take().unwrap().send(());
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.running.take().unwrap(),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            self.count(),
            expected,
            "No projection/export-triggered HTTP after fixture drain"
        );
    }
}
struct LastmodStored {
    dir: std::path::PathBuf,
    db: std::path::PathBuf,
    job: Uuid,
    generation: u64,
    config_sha: String,
}
impl Drop for LastmodStored {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
impl LastmodStored {
    async fn crawl(website: &LastmodWebsite, sitemap: bool, legacy: bool) -> Self {
        use open_seo_spider_core::{CrawlConfig, CrawlControl, CrawlOutcome, CrawlStore, Crawler};
        let job = Uuid::new_v4();
        let dir = std::env::temp_dir().join(format!("oss-lastmod-app-{job}"));
        std::fs::create_dir(&dir).unwrap();
        let db = dir.join(format!("{job}.sqlite"));
        let config = CrawlConfig {
            seed_urls: vec![format!("{}/seed", website.origin)],
            sitemap_urls: if sitemap {
                vec![format!("{}/source.xml", website.origin)]
            } else {
                vec![]
            },
            obey_robots: false,
            discover_sitemaps: false,
            crawl_links: false,
            concurrency: 1,
            delay_ms: 0,
            request_timeout_secs: 3,
            ..Default::default()
        };
        let crawler = if legacy {
            Crawler::from_saved_config(config)
        } else {
            Crawler::new(config)
        }
        .unwrap()
        .with_store(&db)
        .unwrap();
        let completed = crawler
            .run_controlled(None, CrawlControl::new())
            .await
            .unwrap();
        assert_eq!(completed.outcome, CrawlOutcome::Completed);
        let config_sha = digest(encode(&completed.report.config).unwrap());
        drop(crawler);
        let read = CrawlStore::open_read_only(&db)
            .unwrap()
            .sitemap_read_snapshot()
            .unwrap();
        assert_eq!(
            read.crawl_config_sha256.as_deref(),
            Some(config_sha.as_str())
        );
        Self {
            dir,
            db,
            job,
            generation: read.generation.unwrap(),
            config_sha,
        }
    }
    fn read(&self) -> SitemapReadSnapshot {
        open_seo_spider_core::CrawlStore::open_read_only(&self.db)
            .unwrap()
            .sitemap_read_snapshot()
            .unwrap()
    }
    fn page(&self, query: ReadQuery) -> Inventory {
        inventory(
            self.job,
            self.generation,
            &self.config_sha,
            Some(self.read()),
            query,
        )
        .unwrap()
    }
    fn file_hashes(&self) -> std::collections::BTreeMap<String, String> {
        std::fs::read_dir(&self.dir)
            .unwrap()
            .map(|item| {
                let item = item.unwrap();
                assert!(item.file_type().unwrap().is_file());
                (
                    item.file_name().to_string_lossy().into_owned(),
                    digest(std::fs::read(item.path()).unwrap()),
                )
            })
            .collect()
    }
}

#[tokio::test]
async fn actual_stored_lastmod_assessments_are_visible_entry_only_raw_preserving_and_read_only() {
    use open_seo_spider_core::{
        SitemapCohortCompletion, SitemapLastmodFinding, SitemapLastmodPrecision,
        SitemapLastmodValueAssessment,
    };
    let xml = "<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>{origin}/p1</loc><lastmod>\t&#50;000-01-01\r\n</lastmod><lastmod>bad</lastmod></url><url><loc>{origin}/p2</loc><lastmod>2026-01-01Z</lastmod></url><url><loc>{origin}/p3</loc><lastmod>2026</lastmod></url><url><loc>{origin}/p4</loc><lastmod>2016-12-31T23:59:60Z</lastmod></url><url><loc>{origin}/p5</loc><lastmod/></url><url><loc>{origin}/p6</loc></url><url><loc>{origin}/p7</loc><lastmod>2026-01-01T12:34Z</lastmod></url><url><loc>{origin}/p8</loc><lastmod>-0001-02-29</lastmod></url></urlset>";
    let website = LastmodWebsite::start(xml.into()).await;
    let stored = LastmodStored::crawl(&website, true, false).await;
    let requests = website.count();
    let captured = stored.read();
    let originals = captured.catalog.clone().unwrap();
    let before = stored.file_hashes();
    let mut cursor = None;
    let mut actual_assessments = Vec::new();
    let mut actual_entries = Vec::new();
    let mut export_token = None;
    loop {
        let page = stored.page(ReadQuery {
            kind: RowKind::Entry,
            limit: Some(2),
            cursor,
            ..Default::default()
        });
        assert_eq!(
            page.coverage.derived.completion,
            SitemapCohortCompletion::Complete
        );
        assert!(
            page.coverage
                .derived
                .current_valid_original_location_negative_permitted
        );
        assert!(
            !page
                .summary
                .as_ref()
                .unwrap()
                .declared_source_cohort_complete
        );
        assert!(!page.coverage.declared_source_cohort_complete);
        assert_eq!(page.coverage.negative_membership, "unknown");
        assert_eq!(page.website_requests, 0);
        assert_eq!(page.lastmod_assessment_engine, SITEMAP_LASTMOD_ENGINE);
        let rows = page.rows.unwrap();
        let assessments = page.lastmod_assessments.unwrap();
        assert_eq!(rows.len(), assessments.len());
        for (row, assessment) in rows.iter().zip(&assessments) {
            let SitemapLedgerRow::Entry(entry) = row else {
                panic!("Expected Entry row")
            };
            assert_eq!(
                (
                    assessment.entry_id.as_str(),
                    assessment.observation_id.as_str(),
                    assessment.entry_sequence
                ),
                (
                    entry.entry_id.as_str(),
                    entry.observation_id.as_str(),
                    entry.sequence
                )
            );
            assert_eq!(
                assessment
                    .fields
                    .iter()
                    .map(|field| &field.observed)
                    .collect::<Vec<_>>(),
                entry.observed.lastmod.iter().collect::<Vec<_>>()
            );
            assert!(!assessment.freshness_assessed);
            assert!(!assessment.http_status_assessed);
        }
        export_token.get_or_insert(page.snapshot_token.unwrap());
        actual_entries.extend(rows);
        actual_assessments.extend(assessments);
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(actual_entries.len(), 8);
    assert_eq!(
        actual_entries,
        originals
            .rows
            .as_ref()
            .unwrap()
            .iter()
            .filter(|row| matches!(row, SitemapLedgerRow::Entry(_)))
            .cloned()
            .collect::<Vec<_>>()
    );
    assert!(
        actual_assessments[0]
            .findings
            .contains(&SitemapLastmodFinding::MultipleLastmodElements { observed_count: 2 })
    );
    assert_eq!(
        actual_assessments[0].fields[0].evaluated_value,
        "2000-01-01"
    );
    assert!(matches!(
        actual_assessments[1].fields[0].xsd,
        SitemapLastmodValueAssessment::Valid { .. }
    ));
    assert!(matches!(
        actual_assessments[1].fields[0].w3c_note,
        SitemapLastmodValueAssessment::Invalid { .. }
    ));
    assert!(matches!(
        actual_assessments[2].fields[0].xsd,
        SitemapLastmodValueAssessment::Invalid { .. }
    ));
    assert!(matches!(
        actual_assessments[2].fields[0].w3c_note,
        SitemapLastmodValueAssessment::Valid { .. }
    ));
    assert!(matches!(
        actual_assessments[3].fields[0].xsd,
        SitemapLastmodValueAssessment::Undetermined { .. }
    ));
    assert!(actual_assessments[5].fields.is_empty());
    if let SitemapLastmodValueAssessment::Valid { value } =
        &actual_assessments[6].fields[0].w3c_note
    {
        assert_eq!(value.precision, SitemapLastmodPrecision::Minute);
    } else {
        panic!("Expected W3C minute precision")
    }
    assert!(matches!(
        actual_assessments[7].fields[0].xsd,
        SitemapLastmodValueAssessment::Valid { .. }
    ));
    let no_entry_page = stored.page(ReadQuery {
        kind: RowKind::Root,
        ..Default::default()
    });
    assert!(no_entry_page.lastmod_assessments.unwrap().is_empty());
    let export = CsvQuery {
        kind: RowKind::Entry,
        source_observation_id: None,
        membership_url: None,
        snapshot: export_token.unwrap(),
    };
    let (raw_csv, _) = csv(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(stored.read()),
        export,
    )
    .unwrap();
    let raw_csv = String::from_utf8(raw_csv).unwrap();
    assert!(raw_csv.starts_with("\"kind\",\"sequence\",\"observation_id\",\"occurrence_id\",\"source_ordinal\",\"raw_value\",\"membership_url\",\"admitted_fetch_url\",\"admission_decision\",\"typed_row_json\"\r\n"));
    assert!(raw_csv.contains("&#50;"));
    assert!(!raw_csv.contains(SITEMAP_LASTMOD_ENGINE));
    assert_eq!(stored.read().catalog, Some(originals));
    assert_eq!(stored.file_hashes(), before);
    website.finish(requests).await;
}

#[tokio::test]
async fn actual_context_only_task_change_stales_json_and_csv_tokens_without_rewriting_raw_catalog()
{
    use open_seo_spider_core::{SitemapCohortCompletion, SitemapCoverageReason};
    let website = LastmodWebsite::start(
        "<urlset><url><loc>{origin}/p</loc><lastmod>2000-01-01</lastmod></url></urlset>".into(),
    )
    .await;
    let stored = LastmodStored::crawl(&website, true, false).await;
    let requests = website.count();
    let before_read = stored.read();
    let before_page = stored.page(ReadQuery {
        limit: Some(1),
        ..Default::default()
    });
    assert_eq!(
        before_page.coverage.derived.completion,
        SitemapCohortCompletion::Complete
    );
    let old_export = before_page.snapshot_token.unwrap();
    let old_cursor = before_page.next_cursor.unwrap();
    // Explicit isolated fixture mutation: this verifies context invalidation;
    // it is not evidence for a natural lifecycle or a positive source result.
    let writer = rusqlite::Connection::open(&stored.db).unwrap();
    assert_eq!(
        writer
            .execute(
                "UPDATE discovery_frontier SET done=0 WHERE kind='sitemap' AND url=?1",
                [format!("{}/source.xml", website.origin)]
            )
            .unwrap(),
        1
    );
    drop(writer);
    let changed_read = stored.read();
    assert_eq!(changed_read.catalog, before_read.catalog);
    assert_eq!(changed_read.generation, before_read.generation);
    assert_eq!(
        changed_read.crawl_config_sha256,
        before_read.crawl_config_sha256
    );
    assert_ne!(
        changed_read.coverage_inventory,
        before_read.coverage_inventory
    );
    let changed_page = stored.page(ReadQuery {
        limit: Some(1),
        ..Default::default()
    });
    assert_eq!(
        changed_page.coverage.derived.completion,
        SitemapCohortCompletion::Unknown
    );
    assert!(
        changed_page
            .coverage
            .derived
            .reason_counts
            .contains_key(&SitemapCoverageReason::PendingSource)
    );
    assert_ne!(
        changed_page.snapshot.unwrap().catalog_state_sha256,
        before_page.snapshot.unwrap().catalog_state_sha256
    );
    let continuation = ReadQuery {
        limit: Some(1),
        cursor: Some(old_cursor),
        ..Default::default()
    };
    assert_eq!(
        inventory(
            stored.job,
            stored.generation,
            &stored.config_sha,
            Some(stored.read()),
            continuation
        )
        .unwrap_err()
        .code,
        "stale_sitemap_cursor"
    );
    let export = CsvQuery {
        kind: RowKind::All,
        source_observation_id: None,
        membership_url: None,
        snapshot: old_export,
    };
    assert_eq!(
        csv(
            stored.job,
            stored.generation,
            &stored.config_sha,
            Some(stored.read()),
            export
        )
        .unwrap_err()
        .code,
        "stale_sitemap_cursor"
    );
    website.finish(requests).await;
}

#[tokio::test]
async fn actual_empty_legacy_and_incomplete_sources_keep_unknown_and_optional_cases_honest() {
    use open_seo_spider_core::{SitemapCohortCompletion, SitemapLastmodPresence};
    let website =
        LastmodWebsite::start("<urlset><url><lastmod>2000-01-01</lastmod><lastmod>".into()).await;
    let empty = LastmodStored::crawl(&website, false, false).await;
    let page = empty.page(ReadQuery::default());
    assert_eq!(
        page.coverage.derived.completion,
        SitemapCohortCompletion::KnownEmpty
    );
    assert!(page.lastmod_assessments.unwrap().is_empty());
    let legacy = LastmodStored::crawl(&website, false, true).await;
    let page = legacy.page(ReadQuery::default());
    assert_eq!(
        page.coverage.derived.completion,
        SitemapCohortCompletion::Unknown
    );
    assert!(
        !page
            .coverage
            .derived
            .current_valid_original_location_negative_permitted
    );
    let incomplete = LastmodStored::crawl(&website, true, false).await;
    let page = incomplete.page(ReadQuery {
        kind: RowKind::Entry,
        ..Default::default()
    });
    assert_eq!(
        page.coverage.derived.completion,
        SitemapCohortCompletion::Unknown
    );
    let entries = page.lastmod_assessments.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].presence, SitemapLastmodPresence::ObservedOne);
    assert!(!entries[0].all_entry_fields_observed);
    assert_eq!(entries[0].fields.len(), 1);
    let missing_sql = inventory(
        Uuid::new_v4(),
        1,
        &"a".repeat(64),
        None,
        ReadQuery::default(),
    )
    .unwrap();
    assert!(missing_sql.lastmod_assessments.is_none());
    assert_eq!(
        missing_sql.coverage.derived.completion,
        SitemapCohortCompletion::Unknown
    );
    assert!(missing_sql.rows.is_none());
    let requests = website.count();
    empty.page(ReadQuery::default());
    legacy.page(ReadQuery::default());
    incomplete.page(ReadQuery::default());
    website.finish(requests).await;
}

#[tokio::test]
async fn actual_derived_json_byte_cap_fails_atomically_and_smaller_visible_page_recovers() {
    // 200 real bounded raw fields remain below the source/row budgets, while
    // raw rows plus additive assessed copies exceed the API's actual 8 MiB cap.
    let field = "a".repeat(7000);
    let xml = format!(
        "<urlset>{}</urlset>",
        format!("<url><lastmod>{field}</lastmod></url>").repeat(200)
    );
    let website = LastmodWebsite::start(xml).await;
    let stored = LastmodStored::crawl(&website, true, false).await;
    let requests = website.count();
    let captured = stored.read();
    assert_eq!(
        captured
            .catalog
            .as_ref()
            .unwrap()
            .rows
            .as_ref()
            .unwrap()
            .iter()
            .filter(|row| matches!(row, SitemapLedgerRow::Entry(_)))
            .count(),
        200
    );
    let before = stored.file_hashes();
    let oversized = inventory(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(stored.read()),
        ReadQuery {
            kind: RowKind::Entry,
            limit: Some(200),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(oversized.code, "sitemap_response_byte_budget");
    assert_eq!(oversized.status(), 413);
    let recovered = stored.page(ReadQuery {
        kind: RowKind::Entry,
        limit: Some(100),
        ..Default::default()
    });
    assert_eq!(recovered.rows.as_ref().unwrap().len(), 100);
    assert_eq!(recovered.lastmod_assessments.as_ref().unwrap().len(), 100);
    assert!(recovered.has_more);
    assert!(encode(&recovered).unwrap().len() <= MAX_JSON_BYTES);
    assert_eq!(stored.read().catalog, captured.catalog);
    assert_eq!(stored.file_hashes(), before);
    website.finish(requests).await;
}

fn actual_analysis_read(stored: &LastmodStored, query: &ReadQuery) -> SitemapReadSnapshot {
    open_seo_spider_core::CrawlStore::open_read_only(&stored.db)
        .unwrap()
        .sitemap_read_snapshot_with_membership_analysis(&query.analysis_request().unwrap())
        .unwrap()
}
fn actual_analysis_page(stored: &LastmodStored, query: ReadQuery) -> Inventory {
    let read = actual_analysis_read(stored, &query);
    inventory(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(read),
        query,
    )
    .unwrap()
}
#[tokio::test]
async fn actual_saved_report_analysis_pages_are_independent_and_raw_csv_remains_exact() {
    let website=LastmodWebsite::start("<urlset><url><loc>{origin}/a</loc></url><url><loc>{origin}/b</loc></url><url><loc>{origin}/c</loc></url></urlset>".into()).await;
    let stored = LastmodStored::crawl(&website, true, false).await;
    let requests = website.count();
    let before = stored.file_hashes();
    let mut query = ReadQuery {
        kind: RowKind::Location,
        limit: Some(1),
        page_limit: Some(1),
        ..Default::default()
    };
    let first = actual_analysis_page(&stored, query.clone());
    let snapshot_token = first.snapshot_token.clone();
    let raw_sequence = first.rows.as_ref().unwrap()[0].sequence();
    let mut saved_urls = vec![];
    let mut response = first;
    loop {
        assert_eq!(response.snapshot_token, snapshot_token);
        assert_eq!(
            response.rows.as_ref().unwrap()[0].sequence(),
            raw_sequence,
            "Saved-page pagination must not move original rows"
        );
        let analysis = response.membership_analysis.as_ref().unwrap();
        assert!(analysis.context.available);
        assert!(!analysis.context.website_wide_orphan_inference);
        assert_eq!(
            analysis.location_findings.as_ref().unwrap()[0].sequence,
            raw_sequence
        );
        saved_urls.extend(
            analysis
                .saved_pages
                .as_ref()
                .unwrap()
                .iter()
                .map(|v| v.observation.requested_url.clone()),
        );
        let Some(cursor) = response.next_page_cursor.clone() else {
            break;
        };
        query.page_cursor = Some(cursor);
        response = actual_analysis_page(&stored, query.clone());
    }
    assert_eq!(saved_urls.len(), 4);
    assert_eq!(
        saved_urls
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    let captured = open_seo_spider_core::CrawlStore::open_read_only(&stored.db)
        .unwrap()
        .sitemap_read_snapshot_with_membership_analysis(&Default::default())
        .unwrap();
    let (actual, _) = csv(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(captured),
        CsvQuery {
            kind: RowKind::Location,
            source_observation_id: None,
            membership_url: None,
            snapshot: snapshot_token.unwrap(),
        },
    )
    .unwrap();
    let legacy = stored.page(ReadQuery {
        kind: RowKind::Location,
        ..Default::default()
    });
    let (original, _) = csv(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(stored.read()),
        CsvQuery {
            kind: RowKind::Location,
            source_observation_id: None,
            membership_url: None,
            snapshot: legacy.snapshot_token.unwrap(),
        },
    )
    .unwrap();
    assert_eq!(
        actual, original,
        "Raw original CSV must not acquire derived target facts"
    );
    assert_eq!(stored.file_hashes(), before);
    website.finish(requests).await;
}
#[tokio::test]
async fn actual_stored_report_context_only_change_invalidates_raw_page_and_csv_tokens() {
    let website = LastmodWebsite::start(
        "<urlset><url><loc>{origin}/a</loc></url><url><loc>{origin}/b</loc></url></urlset>".into(),
    )
    .await;
    let stored = LastmodStored::crawl(&website, true, false).await;
    let requests = website.count();
    let base = ReadQuery {
        kind: RowKind::Location,
        limit: Some(1),
        page_limit: Some(1),
        ..Default::default()
    };
    let first = actual_analysis_page(&stored, base.clone());
    let old_read = stored.read();
    // Isolated fixture context-invalidation control, not a natural crawler
    // lifecycle or a production-positive witness: retain exact typed report
    // semantics but change actual stored bytes and matching checkpoint byte SHA.
    let mut connection = rusqlite::Connection::open(&stored.db).unwrap();
    let tx = connection.transaction().unwrap();
    let report: String = tx
        .query_row("SELECT value FROM metadata WHERE key='report'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let changed = format!("{report}\n");
    let checkpoint: String = tx
        .query_row(
            "SELECT value FROM metadata WHERE key='checkpoint'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut checkpoint: open_seo_spider_core::CrawlCheckpoint =
        serde_json::from_str(&checkpoint).unwrap();
    checkpoint.report_sha256 = digest(changed.as_bytes());
    tx.execute("UPDATE metadata SET value=?1 WHERE key='report'", [changed])
        .unwrap();
    tx.execute(
        "UPDATE metadata SET value=?1 WHERE key='checkpoint'",
        [serde_json::to_string(&checkpoint).unwrap()],
    )
    .unwrap();
    tx.commit().unwrap();
    drop(connection);
    let next = actual_analysis_page(&stored, base.clone());
    assert_eq!(old_read.catalog, stored.read().catalog);
    assert_eq!(
        old_read.coverage_inventory,
        stored.read().coverage_inventory
    );
    assert_eq!(old_read.generation, stored.read().generation);
    assert_ne!(
        first.snapshot.as_ref().unwrap().catalog_state_sha256,
        next.snapshot.as_ref().unwrap().catalog_state_sha256
    );
    for query in [
        ReadQuery {
            cursor: first.next_cursor.clone(),
            ..base.clone()
        },
        ReadQuery {
            page_cursor: first.next_page_cursor.clone(),
            ..base
        },
    ] {
        let read = actual_analysis_read(&stored, &query);
        let error = inventory(
            stored.job,
            stored.generation,
            &stored.config_sha,
            Some(read),
            query,
        )
        .unwrap_err();
        assert_eq!(error.code, "stale_sitemap_cursor");
    }
    let read = open_seo_spider_core::CrawlStore::open_read_only(&stored.db)
        .unwrap()
        .sitemap_read_snapshot_with_membership_analysis(&Default::default())
        .unwrap();
    let failure = csv(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(read),
        CsvQuery {
            kind: RowKind::Location,
            source_observation_id: None,
            membership_url: None,
            snapshot: first.snapshot_token.unwrap(),
        },
    )
    .unwrap_err();
    assert_eq!(failure.code, "stale_sitemap_cursor");
    website.finish(requests).await;
}

// Directed actual SQLite/HTTP context invalidation controls; authored only.
#[path = "sitemap_source_context_tests.rs"]
mod source_context;
