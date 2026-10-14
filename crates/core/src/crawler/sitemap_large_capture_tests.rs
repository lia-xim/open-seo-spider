//! AUTHORED / NOT_RUN. Child of the existing actual HTTP capture tests.
//! Fixture bytes are generated only when these tests execute after the HOLD.
use super::*;

const MIB: usize = 1024 * 1024;
const REAL_XML_BYTES: usize = 9 * MIB;

fn optin() -> crate::SitemapSourceProcessingConfig {
    crate::SitemapSourceProcessingConfig {
        max_post_http_entity_bytes: 64 * MIB,
        max_decoded_xml_bytes: 64 * MIB,
        max_xml_events: 2_000_000,
    }
}

fn actual_large_xml(origin: &str, index: bool) -> Vec<u8> {
    // Real XML bytes, with unchanged public protocol thresholds. A valid large
    // comment avoids fabricating a giant loc or changing per-field limits.
    let (root, entry, loc) = if index {
        ("sitemapindex", "sitemap", "child.xml")
    } else {
        ("urlset", "url", "p")
    };
    let mut bytes =
        format!("<{root} xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><!--").into_bytes();
    let tail = format!("--><{entry}><loc>{origin}/{loc}</loc></{entry}></{root}>");
    assert!(bytes.len() + tail.len() < REAL_XML_BYTES);
    bytes.resize(REAL_XML_BYTES - tail.len(), b'x');
    bytes.extend_from_slice(tail.as_bytes());
    assert_eq!(bytes.len(), REAL_XML_BYTES);
    bytes
}

#[tokio::test]
async fn real_missing_and_explicit_default_source_profiles_preserve_their_caps() {
    let fixture = BinaryFixture::new(|origin, _| {
        BinaryResponse::bytes(actual_large_xml(origin, false), "application/xml")
    })
    .await;
    for (path, page_cap, profile, expected) in [
        ("/missing.xml", 5 * MIB, None, 5 * MIB),
        ("/legacy-large-page.xml", 64 * MIB, None, 8 * MIB),
        (
            "/explicit-default.xml",
            1024,
            Some(crate::SitemapSourceProcessingConfig::default()),
            8 * MIB,
        ),
    ] {
        let (_temp, crawler, generation, entry) = setup(&fixture, path, |cfg| {
            cfg.max_body_bytes = page_cap;
            cfg.request_timeout_secs = 30;
            cfg.sitemap_source_processing = profile;
            if cfg.sitemap_source_processing.is_none() {
                let old_json = serde_json::to_value(&*cfg).unwrap();
                assert!(old_json.get("sitemap_source_processing").is_none());
                *cfg = serde_json::from_value(old_json).unwrap();
                assert!(cfg.sitemap_source_processing.is_none());
            }
        });
        let output = collect(&crawler, generation, &entry).await;
        let record = measured(&output);
        assert!(!record.payload.transport.body_complete);
        assert_eq!(
            record.payload.transport.retained_post_http_prefix_bytes,
            expected as u64
        );
        assert_eq!(
            record.payload.bounds.max_retained_post_http_bytes,
            expected as u64
        );
        assert_eq!(
            record.payload.bounds.max_retained_xml_bytes,
            (8 * MIB) as u64
        );
        assert_eq!(
            record.payload.decode.integrity,
            crate::SitemapSourceDecodeIntegrity::NotObserved
        );
        assert_eq!(
            record.payload.scan.status,
            crate::SitemapSourceScanStatus::NotObserved
        );
        assert_eq!(
            assess(&output).decoded_xml_byte_limit.verdict,
            crate::SitemapSourceLimitVerdict::Unknown
        );
        assert!(
            record
                .payload
                .transport
                .post_http_received_bytes
                .value
                .unwrap()
                > expected as u64
        );
        assert_eq!(
            record.payload.transport.post_http_received_bytes.precision,
            crate::SitemapMeasuredPrecision::AtLeast
        );
    }
    assert_requests(
        &fixture.finish().await,
        &[
            ("/missing.xml", 1),
            ("/legacy-large-page.xml", 1),
            ("/explicit-default.xml", 1),
        ],
    );
}

#[tokio::test]
async fn invalid_source_processing_profiles_fail_real_constructor_before_requests() {
    let fixture =
        BinaryFixture::new(|_, _| panic!("invalid profile must not request a source")).await;
    for field in 0..3 {
        for value in [0, if field == 2 { 2_000_001 } else { 64 * MIB + 1 }] {
            let mut profile = optin();
            match field {
                0 => profile.max_post_http_entity_bytes = value,
                1 => profile.max_decoded_xml_bytes = value,
                _ => profile.max_xml_events = value,
            }
            assert!(
                Crawler::new(CrawlConfig {
                    seed_urls: vec![fixture.url("/seed")],
                    sitemap_urls: vec![fixture.url("/invalid.xml")],
                    sitemap_source_processing: Some(profile),
                    ..Default::default()
                })
                .is_err(),
                "invalid profile must reject the actual public constructor"
            );
        }
    }
    assert_requests(&fixture.finish().await, &[]);
}

#[tokio::test]
async fn actual_nine_mib_plain_file_http_gzip_and_index_use_saved_optin_bounds() {
    let fixture = BinaryFixture::new(|origin, path| {
        if path == "/robots.txt" {
            return BinaryResponse::robots(b"User-agent: *\nDisallow: /blocked.xml\n".to_vec());
        }
        if path == "/bad-status.xml" {
            let mut response = BinaryResponse::bytes(b"failed source".to_vec(), "text/plain");
            response.status = 503;
            return response;
        }
        if path == "/page" {
            return BinaryResponse::bytes(vec![b'x'; 4096], "text/html");
        }
        let bytes = actual_large_xml(origin, path == "/large-index.xml");
        match path {
            "/large-file.gz" => BinaryResponse::bytes(gzip(&bytes), "application/octet-stream"),
            "/large-http.xml" => BinaryResponse::bytes(gzip(&bytes), "application/xml")
                .header("Content-Encoding", "gzip"),
            _ => BinaryResponse::bytes(bytes, "application/xml"),
        }
    })
    .await;
    for (path, encoding, index) in [
        (
            "/large-plain.xml",
            crate::SitemapEntityEncoding::Identity,
            false,
        ),
        (
            "/large-file.gz",
            crate::SitemapEntityEncoding::GzipFileEntity,
            false,
        ),
        (
            "/large-http.xml",
            crate::SitemapEntityEncoding::Identity,
            false,
        ),
        (
            "/large-index.xml",
            crate::SitemapEntityEncoding::Identity,
            true,
        ),
    ] {
        let (_temp, crawler, generation, entry) = setup(&fixture, path, |cfg| {
            cfg.max_body_bytes = 1024;
            cfg.request_timeout_secs = 30;
            cfg.sitemap_source_processing = Some(optin());
        });
        let output = collect(&crawler, generation, &entry).await;
        let record = measured(&output);
        let expected = actual_large_xml(&fixture.origin, index);
        assert!(record.payload.transport.body_complete);
        assert_eq!(
            record.payload.transport.post_http_received_bytes.precision,
            crate::SitemapMeasuredPrecision::Exact
        );
        assert_eq!(
            record.payload.transport.post_http_received_bytes.value,
            Some(record.payload.transport.retained_post_http_prefix_bytes)
        );
        if path == "/large-file.gz" {
            assert!(
                record.payload.transport.retained_post_http_prefix_bytes < REAL_XML_BYTES as u64
            );
        } else {
            assert_eq!(
                record.payload.transport.retained_post_http_prefix_bytes, REAL_XML_BYTES as u64,
                "HTTP gzip is measured after HTTP Content-Decoding"
            );
        }
        assert_eq!(record.payload.decode.encoding, Some(encoding));
        assert_eq!(
            record.payload.decode.integrity,
            crate::SitemapSourceDecodeIntegrity::Complete
        );
        assert_eq!(
            record.payload.decode.decoded_xml_bytes.value,
            Some(REAL_XML_BYTES as u64)
        );
        assert_eq!(
            record.payload.decode.decoded_complete_sha256.as_deref(),
            Some(crate::sitemap_ledger::digest(&expected).as_str())
        );
        assert_eq!(
            record.payload.bounds.max_retained_post_http_bytes,
            (64 * MIB) as u64
        );
        assert_eq!(
            record.payload.bounds.max_retained_xml_bytes,
            (64 * MIB) as u64
        );
        assert_eq!(record.payload.bounds.max_xml_events, 2_000_000);
        assert_eq!(record.payload.bounds.max_retained_entries, 50_000);
        assert_eq!(
            record.payload.bounds.max_retained_text_utf8_bytes,
            (32 * MIB) as u64
        );
        assert_eq!(count(&output).input_xml_bytes, REAL_XML_BYTES);
        assert!(count(&output).xml_scan_complete);
        assert_eq!(count(&output).counts.protocol_eligible_completed_entries, 1);
        assert_eq!(
            count(&output).kind,
            Some(if index {
                crate::SitemapDocumentKind::SitemapIndex
            } else {
                crate::SitemapDocumentKind::UrlSet
            })
        );
        assert_eq!(
            record.payload.membership_retention,
            crate::SitemapMembershipRetention::Complete
        );
        assert_eq!(assess(&output).decoded_xml_byte_limit.threshold, 52_428_800);
        assert_eq!(
            assess(&output).decoded_xml_byte_limit.verdict,
            crate::SitemapSourceLimitVerdict::WithinLimit
        );
        let store = crawler.store.as_ref().unwrap();
        let document = &output.sitemap_batch.as_ref().unwrap().document;
        let receipt = store
            .record_sitemap_source_capture(&entry, document, record, generation)
            .unwrap();
        assert!(receipt.is_persisted());
        let saved = store
            .sitemap_read_snapshot()
            .unwrap()
            .sitemap_source_measurements
            .unwrap();
        assert_eq!(saved.records.len(), 1);
        assert_eq!(&saved.records[0].measurement, record);
        assert!(
            saved.commits.is_empty(),
            "A capture is not B source admission"
        );
        if path == "/large-plain.xml" {
            let (page, _) = context(&crawler, generation)
                .document(QueueEntry {
                    url: fixture.url("/page"),
                    depth: 0,
                    discovered_from: None,
                    admission: None,
                })
                .await;
            assert!(page.truncated);
            assert_eq!(
                page.size_bytes, 1024,
                "source opt-in does not raise actual page response retention"
            );
        }
    }
    for (path, robots) in [("/bad-status.xml", false), ("/blocked.xml", true)] {
        let (_temp, crawler, generation, entry) = setup(&fixture, path, |cfg| {
            cfg.max_body_bytes = 1024;
            cfg.sitemap_source_processing = Some(optin());
            cfg.obey_robots = robots;
        });
        let output = collect(&crawler, generation, &entry).await;
        let record = measured(&output);
        assert_eq!(record.payload.transport.request_started, !robots);
        assert_eq!(
            record.payload.bounds.max_retained_post_http_bytes,
            (64 * MIB) as u64
        );
        assert_eq!(
            record.payload.bounds.max_retained_xml_bytes,
            (64 * MIB) as u64
        );
        assert_eq!(record.payload.bounds.max_xml_events, 2_000_000);
        assert_eq!(
            record.payload.decode.integrity,
            crate::SitemapSourceDecodeIntegrity::NotObserved
        );
        assert_eq!(
            record.payload.scan.status,
            crate::SitemapSourceScanStatus::NotObserved
        );
        if robots {
            assert_eq!(
                record.payload.transport.post_http_received_bytes.precision,
                crate::SitemapMeasuredPrecision::Unavailable
            );
        }
        assert!(
            crawler
                .store
                .as_ref()
                .unwrap()
                .record_sitemap_source_capture(
                    &entry,
                    &output.sitemap_batch.as_ref().unwrap().document,
                    record,
                    generation,
                )
                .unwrap()
                .is_persisted(),
            "failure/unstarted A must bind the actual Some profile too"
        );
    }
    assert_requests(
        &fixture.finish().await,
        &[
            ("/large-plain.xml", 1),
            ("/large-file.gz", 1),
            ("/large-http.xml", 1),
            ("/large-index.xml", 1),
            ("/page", 1),
            ("/bad-status.xml", 1),
            ("/robots.txt", 1),
        ],
    );
}

#[tokio::test]
async fn actual_optin_capture_uses_current_saved_sha_and_store_rejects_wrong_work_bounds() {
    let fixture =
        BinaryFixture::new(|origin, _| BinaryResponse::bytes(xml(origin), "application/xml")).await;
    let (_temp, mut crawler, _, entry) = setup(&fixture, "/profile.xml", |cfg| {
        cfg.max_body_bytes = 1024;
        cfg.sitemap_source_processing = Some(optin());
    });
    let store = crawler.store.as_ref().unwrap();
    let initial = store
        .sitemap_catalog()
        .unwrap()
        .unwrap()
        .summary
        .config_sha256;
    crawler.config.delay_ms = 1;
    store.prepare(&crawler.config).unwrap();
    let generation = store
        .begin_run("legitimate current-config operational override with source optin")
        .unwrap();
    let current = store.saved_config_bytes_sha256().unwrap();
    assert_ne!(initial, current);
    let output = collect(&crawler, generation, &entry).await;
    let record = measured(&output);
    assert_eq!(record.payload.binding.crawl_config_sha256, current);
    let document = &output.sitemap_batch.as_ref().unwrap().document;
    let before = store
        .sitemap_read_snapshot()
        .unwrap()
        .sitemap_source_measurements;
    for field in 0..3 {
        // Negative internally valid DTO derived from the actual capture. Its
        // wrong policy is not allowed to claim the actual saved-config profile.
        let mut payload = record.payload.clone();
        match field {
            0 => payload.bounds.max_retained_post_http_bytes -= 1,
            1 => payload.bounds.max_retained_xml_bytes -= 1,
            _ => payload.bounds.max_xml_events -= 1,
        }
        let wrong = crate::SitemapSourceMeasurement::new(payload, &Default::default()).unwrap();
        let error = store
            .record_sitemap_source_capture(&entry, document, &wrong, generation)
            .err()
            .expect("wrong bounds must not commit A");
        assert!(
            error.to_string().contains("source processing profile"),
            "{error:#}"
        );
        assert_eq!(
            store
                .sitemap_read_snapshot()
                .unwrap()
                .sitemap_source_measurements,
            before
        );
    }
    assert!(
        store
            .record_sitemap_source_capture(&entry, document, record, generation)
            .unwrap()
            .is_persisted()
    );
    let saved = store
        .sitemap_read_snapshot()
        .unwrap()
        .sitemap_source_measurements
        .unwrap();
    assert_eq!(saved.records.len(), 1);
    assert_eq!(saved.summary.initial_catalog_config_sha256, initial);
    assert_eq!(
        saved.records[0]
            .measurement
            .payload
            .binding
            .crawl_config_sha256,
        current
    );
    assert_requests(&fixture.finish().await, &[("/profile.xml", 1)]);
}
