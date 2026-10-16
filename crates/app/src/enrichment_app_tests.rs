use super::*;
use futures::future::BoxFuture;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
#[derive(Clone, Default)]
pub(crate) struct Mock {
    pub(crate) calls: Arc<AtomicUsize>,
}
impl PageSpeedTransport for Mock {
    fn execute<'a>(
        &'a self,
        r: api::PageSpeedRequest<'a>,
        _: ApiCancellation,
    ) -> BoxFuture<'a, std::result::Result<api::PageSpeedReply, api::ApiFailureCode>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(api::PageSpeedReply{status:200,bytes:serde_json::to_vec(&serde_json::json!({"lighthouseResult":{"requestedUrl":r.requested_url,"finalUrl":r.requested_url,"categories":{"performance":{"score":0.5}}}})).unwrap(),retry_after_secs:None})
        })
    }
}
pub(crate) fn request(input: &str) -> StartRequest {
    StartRequest {
        acknowledge_remote_fetch: true,
        input_utf8: input.into(),
        import: ListImportOptions {
            format: open_seo_spider_core::ListImportFormat::Txt,
            limits: Default::default(),
        },
        options: ApiEnrichmentOptions {
            retries: 0,
            ..Default::default()
        },
    }
}
pub(crate) async fn settled(service: &Service, id: Uuid) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while locked(&service.active).contains_key(&id) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    service.owner.reap_completed().await;
}
pub(crate) async fn fixture(root: PathBuf, input: &str) -> (Service, Uuid, Mock) {
    let s = Service::open(root, TaskAdmission::default(), false).unwrap();
    let r = request(input);
    let mock = Mock::default();
    let started = s
        .start_mock(
            prepare(&r).unwrap(),
            input.as_bytes().to_vec(),
            mock.clone(),
            None,
        )
        .await
        .unwrap();
    let id = serde_json::from_value(started["id"].clone()).unwrap();
    settled(&s, id).await;
    (s, id, mock)
}

#[tokio::test]
async fn api_enrichment_snapshot_pages_preserve_all_occurrences_filters_and_cold_no_provider_resume()
 {
    let dir = tempfile::tempdir().unwrap();
    let input = (0..205)
        .map(|i| format!("https://example.test/{i}\n"))
        .collect::<String>()
        + "https://example.test/0\ninvalid";
    let root = dir.path().join("jobs");
    let (service, id, mock) = fixture(root.clone(), &input).await;
    assert_eq!(mock.calls.load(Ordering::SeqCst), 205);
    let first: serde_json::Value =
        serde_json::from_slice(&service.page(id, &Default::default()).unwrap()).unwrap();
    assert_eq!(first["total"], 207);
    assert_eq!(first["rows"].as_array().unwrap().len(), 100);
    let snapshot = first["snapshot"].as_str().unwrap().to_owned();
    let after = first["after"].as_str().unwrap().to_owned();
    let second = ReadRequest {
        expected_snapshot: Some(snapshot.clone()),
        after: Some(after.clone()),
        ..Default::default()
    };
    let page: serde_json::Value =
        serde_json::from_slice(&service.page(id, &second).unwrap()).unwrap();
    assert_eq!(page["rows"].as_array().unwrap().len(), 100);
    assert_ne!(
        first["rows"][0]["input"]["raw_value"],
        page["rows"][0]["input"]["raw_value"]
    );
    assert!(
        service
            .page(
                id,
                &ReadRequest {
                    after: Some(after.clone()),
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert!(
        service
            .page(
                id,
                &ReadRequest {
                    expected_snapshot: Some("stale".into()),
                    after: Some(after.clone()),
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert!(
        service
            .page(
                id,
                &ReadRequest {
                    outcome: Some(ApiOutcome::Failed),
                    ..second.clone()
                }
            )
            .is_err()
    );
    assert!(service.page(Uuid::new_v4(), &second).is_err());
    let csv = service
        .csv(
            id,
            &ReadRequest {
                expected_snapshot: Some(snapshot.clone()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(String::from_utf8(csv.clone()).unwrap().lines().count(), 208);
    service.drain().await.unwrap();
    drop(service);
    let reopened = Service::open(root, TaskAdmission::default(), false).unwrap();
    assert_eq!(reopened.status(id).unwrap()["status"], "complete");
    assert_eq!(
        reopened
            .csv(
                id,
                &ReadRequest {
                    expected_snapshot: Some(snapshot),
                    ..Default::default()
                }
            )
            .unwrap(),
        csv
    );
    assert_eq!(mock.calls.load(Ordering::SeqCst), 205);
    assert_eq!(reopened.stats()["admitted"], 0);
    reopened.drain().await.unwrap();
}
#[tokio::test]
async fn api_enrichment_close_cancels_and_actually_joins_owned_inflight_work_before_return() {
    struct Held {
        entered: Arc<tokio::sync::Notify>,
        dropped: Arc<AtomicUsize>,
    }
    impl PageSpeedTransport for Held {
        fn execute<'a>(
            &'a self,
            _: api::PageSpeedRequest<'a>,
            _: ApiCancellation,
        ) -> BoxFuture<'a, std::result::Result<api::PageSpeedReply, api::ApiFailureCode>> {
            Box::pin(async move {
                struct D(Arc<AtomicUsize>);
                impl Drop for D {
                    fn drop(&mut self) {
                        self.0.fetch_add(1, Ordering::SeqCst);
                    }
                }
                let _d = D(self.dropped.clone());
                self.entered.notify_one();
                std::future::pending().await
            })
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let s = Service::open(dir.path().join("jobs"), TaskAdmission::default(), false).unwrap();
    assert!(!dir.path().join("jobs").exists());
    let entered = Arc::new(tokio::sync::Notify::new());
    let dropped = Arc::new(AtomicUsize::new(0));
    let r = request("https://example.test/a\nhttps://example.test/b");
    let result = s
        .start_mock(
            prepare(&r).unwrap(),
            r.input_utf8.into_bytes(),
            Held {
                entered: entered.clone(),
                dropped: dropped.clone(),
            },
            None,
        )
        .await
        .unwrap();
    let id = serde_json::from_value(result["id"].clone()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    s.close();
    tokio::time::timeout(Duration::from_secs(3), s.drain())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    assert_eq!(s.stats()["admitted"], 1);
    assert_eq!(s.stats()["started"], 1);
    assert_eq!(s.stats()["joined"], 1);
    assert_eq!(s.stats()["retained"], 0);
    assert_eq!(s.stats()["active_jobs"], 0);
    assert_eq!(s.status(id).unwrap()["status"], "cancelled");
    assert_eq!(s.report(id).unwrap().0.provider_request_attempts, 1);
    let r = request("https://example.test/new");
    assert!(
        s.start_mock(
            prepare(&r).unwrap(),
            r.input_utf8.into_bytes(),
            Mock::default(),
            None
        )
        .await
        .is_err()
    );
}
#[tokio::test]
async fn api_enrichment_original_input_config_and_completed_report_hashes_refuse_tamper() {
    let dir = tempfile::tempdir().unwrap();
    let input = "https://example.test/a#part\nhttps://example.test/a#other";
    let (s, id, mock) = fixture(dir.path().join("jobs"), input).await;
    let saved: StartRequest = serde_json::from_slice(&s.configuration(id).unwrap()).unwrap();
    assert_eq!(saved.input_utf8, input);
    assert_eq!(saved.options, request(input).options);
    assert!(!saved.acknowledge_remote_fetch);
    let (report, snapshot) = s.report(id).unwrap();
    assert_eq!(report.plan.input.rows.len(), 2);
    assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
    fs::write(s.directory(id).join("input.bin"), b"https://changed.test/").unwrap();
    assert!(s.configuration(id).is_err());
    fs::write(s.directory(id).join("report.json"), b"{}").unwrap();
    assert!(s.report(id).is_err());
    assert!(
        s.csv(
            id,
            &ReadRequest {
                expected_snapshot: Some(snapshot),
                ..Default::default()
            }
        )
        .is_err()
    );
    s.drain().await.unwrap();
}
#[tokio::test]
async fn api_enrichment_cli_protects_existing_inputs_outputs_and_returns_complete_json_csv_from_same_service()
 {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.txt");
    let options = dir.path().join("options.json");
    let output = dir.path().join("out.json");
    let csv = dir.path().join("out.csv");
    let text = "https://example.test/a\nhttps://example.test/a\ninvalid";
    fs::write(&input, text).unwrap();
    let mut r = request("");
    r.input_utf8.clear();
    fs::write(&options, serde_json::to_vec(&r).unwrap()).unwrap();
    let mock = Mock::default();
    let calls = mock.calls.clone();
    let result = run_cli_inner(
        input.clone(),
        options.clone(),
        dir.path().join("jobs"),
        output.clone(),
        Some(csv.clone()),
        mock,
        |_| Ok(None),
    )
    .await
    .unwrap();
    assert_eq!(result["provider_request_attempts"], 1);
    assert_eq!(result["application_website_requests"], 0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let report: ApiEnrichmentReport = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(report.plan.input.rows.len(), 3);
    assert_eq!(
        String::from_utf8(fs::read(&csv).unwrap())
            .unwrap()
            .lines()
            .count(),
        4
    );
    assert_eq!(fs::read(&input).unwrap(), text.as_bytes());
    let old = fs::read(&output).unwrap();
    let mock = Mock::default();
    let calls = mock.calls.clone();
    assert!(
        run_cli_inner(
            input.clone(),
            options.clone(),
            dir.path().join("not-created"),
            output.clone(),
            None,
            mock,
            |_| Ok(None)
        )
        .await
        .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(fs::read(&output).unwrap(), old);
    assert!(!dir.path().join("not-created").exists());
    let mock = Mock::default();
    assert!(
        run_cli_inner(
            input.clone(),
            options,
            dir.path().join("also-not-created"),
            input.clone(),
            None,
            mock,
            |_| Ok(None)
        )
        .await
        .is_err()
    );
    assert_eq!(fs::read(input).unwrap(), text.as_bytes());
}
#[tokio::test]
async fn api_enrichment_owner_panic_is_reported_and_retained_join_is_not_lost() {
    struct Panics;
    impl PageSpeedTransport for Panics {
        fn execute<'a>(
            &'a self,
            _: api::PageSpeedRequest<'a>,
            _: ApiCancellation,
        ) -> BoxFuture<'a, std::result::Result<api::PageSpeedReply, api::ApiFailureCode>> {
            Box::pin(async move { panic!("test-only provider panic") })
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let s = Service::open(dir.path().join("jobs"), TaskAdmission::default(), false).unwrap();
    let r = request("https://example.test/a");
    let result = s
        .start_mock(
            prepare(&r).unwrap(),
            r.input_utf8.into_bytes(),
            Panics,
            None,
        )
        .await
        .unwrap();
    let id = serde_json::from_value(result["id"].clone()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while s.owner.snapshot().first_failure.is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(s.status(id).is_err());
    assert!(s.drain().await.is_err());
    assert_eq!(s.stats()["retained"], 0);
    assert_eq!(s.stats()["joined"], 1);
    let cold = Service::open(dir.path().join("jobs"), TaskAdmission::default(), false).unwrap();
    assert_eq!(cold.status(id).unwrap()["status"], "interrupted");
    assert_eq!(cold.stats()["admitted"], 0);
    cold.drain().await.unwrap();
}

#[test]
fn api_enrichment_ui_first_csv_column_one_matches_real_import_consumer() {
    let value = serde_json::json!({"acknowledge_remote_fetch":false,"input_utf8":"https://example.test/a,ignored\nhttps://example.test/b,ignored","import":{"format":{"kind":"csv","delimiter":"comma","column":{"kind":"index","value":1},"start_row":1}},"options":{"version":1,"strategy":"mobile","credential_profile":"anonymous"}});
    let r: StartRequest = serde_json::from_value(value.clone()).unwrap();
    let p = prepare(&r).unwrap();
    assert_eq!(
        p.unique_urls,
        ["https://example.test/a", "https://example.test/b"]
    );
    assert_eq!(p.input.rows.len(), 2);
    assert_eq!(p.input.selected_column, Some(1));
    let mut bad = value;
    bad["import"]["format"]["column"]["value"] = serde_json::json!(0);
    let bad: StartRequest = serde_json::from_value(bad).unwrap();
    assert!(prepare(&bad).is_err());
}
#[tokio::test]
async fn api_enrichment_normalized_overcap_plan_is_refused_before_store_owner_or_provider_admission()
 {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("must-not-be-created");
    let s = Service::open(root.clone(), TaskAdmission::default(), false).unwrap();
    let prefix = "https://example.test/";
    let raw = format!(
        "{prefix}{}{}a",
        "é".repeat(1000),
        "a".repeat(8192 - prefix.len() - 6000)
    );
    assert!(raw.len() < 8192);
    let r = request(&raw);
    assert!(prepare(&r).is_err());
    assert!(!root.exists());
    assert_eq!(s.stats()["admitted"], 0);
    assert!(open_seo_spider_core::import_url_list(raw.as_bytes(), &r.import).is_err());
    let exact_raw = raw.strip_suffix('a').unwrap();
    let exact = prepare(&request(exact_raw)).unwrap();
    assert_eq!(exact.unique_urls[0].len(), 8192);
    // Synthetic forged legacy-like typed plan, NOT a successful import of an8193-byte normalized URL.
    // The unchanged App service must re-import the original bytes and refuse before store/owner/provider admission.
    let mut legacy = exact;
    legacy.input.input_bytes = raw.len();
    legacy.input.input_sha256 = api::api_sha(raw.as_bytes());
    legacy.input.rows[0].raw_value = raw.clone();
    legacy.input.rows[0].normalized_url = Some(format!("{}a", legacy.unique_urls[0]));
    legacy.unique_urls = vec![legacy.input.rows[0].normalized_url.clone().unwrap()];
    assert_eq!(legacy.unique_urls[0].len(), 8193);
    legacy.fingerprint = api::api_sha(
        &api::bounded_api_json(
            &(
                api::API_ENRICHMENT_ENGINE,
                &legacy.input,
                &legacy.options,
                &legacy.unique_urls,
            ),
            api::API_REPORT_BYTES,
        )
        .unwrap(),
    );
    let mock = Mock::default();
    assert!(
        s.start_mock(legacy, raw.into_bytes(), mock.clone(), None)
            .await
            .is_err()
    );
    assert_eq!(mock.calls.load(Ordering::SeqCst), 0);
    assert_eq!(s.stats()["admitted"], 0);
    assert_eq!(s.stats()["active_jobs"], 0);
    assert!(!root.exists());
    s.drain().await.unwrap();
}
