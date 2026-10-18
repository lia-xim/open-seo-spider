use super::*;
use axum::{body::to_bytes, http::Uri};
use open_seo_spider_core::RendererConfig;

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
    bytes.extend_from_slice(kind);
    bytes.extend_from_slice(data);
    bytes.extend_from_slice(&crc32fast::hash(&bytes[4..]).to_be_bytes());
    bytes
}
fn ihdr(width: u32, height: u32) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&width.to_be_bytes());
    data.extend_from_slice(&height.to_be_bytes());
    data.extend_from_slice(&[8, 0, 0, 0, 0]);
    chunk(b"IHDR", &data)
}
/// Genuine grayscale pixels in one zlib stored block, including Adler32. Native
/// validation deliberately does not claim to decode or validate these pixels.
fn image(width: u32, height: u32) -> Vec<u8> {
    let pixels = vec![0u8; (width as usize + 1) * height as usize];
    assert!(pixels.len() <= u16::MAX as usize);
    let length = pixels.len() as u16;
    let mut zlib = vec![0x78, 0x01, 0x01];
    zlib.extend_from_slice(&length.to_le_bytes());
    zlib.extend_from_slice(&(!length).to_le_bytes());
    zlib.extend_from_slice(&pixels);
    // Every data byte is0: Adler A=1, B=number of data bytes modulo65521.
    let adler = ((pixels.len() as u32 % 65521) << 16) | 1;
    zlib.extend_from_slice(&adler.to_be_bytes());
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend(ihdr(width, height));
    png.extend(chunk(b"IDAT", &zlib));
    png.extend(chunk(b"IEND", &[]));
    png
}
fn job(id: Uuid) -> Job {
    let config = CrawlConfig {
        renderer: Some(RendererConfig {
            screenshot: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let page = PageRecord {
        url: "https://owned.test/雪?x=1&y=2".into(),
        final_url: "https://owned.test/final".into(),
        rendered: true,
        rendered_viewport: Some(json!({"width":100,"height":100})),
        screenshot_base64: Some(STANDARD.encode(image(100, 100))),
        ..Default::default()
    };
    Job {
        id,
        config: config.clone(),
        status: "completed".into(),
        started_at: String::new(),
        finished_at: None,
        pages: Vec::new(),
        issues: Vec::new(),
        queued: 0,
        discovery_pending: 0,
        in_flight: 0,
        warnings: Vec::new(),
        error: None,
        report: Some(CrawlReport {
            config,
            pages: vec![page],
            issues: Vec::new(),
            summary: Default::default(),
            warnings: Vec::new(),
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
            sitemap_catalog: None,
            sitemap_coverage_inventory: None,
            sitemap_source_measurements: None,
        }),
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
fn state(job: Job, directory: PathBuf) -> AppState {
    AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Arc::new(RwLock::new(HashMap::from([(
            job.id,
            Arc::new(RwLock::new(job)),
        )]))),
        data_dir: Arc::new(directory),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle: Default::default(),
    }
}
fn info(job: &Job, ordinal: Option<usize>) -> Value {
    let url = &job.report.as_ref().unwrap().pages[0].url;
    serde_json::to_value(inspect(job, url, ordinal).unwrap().metadata).unwrap()
}
fn query(metadata: &Value) -> PngQuery {
    PngQuery {
        url: metadata["url"].as_str().unwrap().into(),
        ordinal: metadata["ordinal"].as_u64().unwrap() as usize,
        expected_snapshot_sha256: metadata["snapshot_sha256"].as_str().unwrap().into(),
        expected_image_sha256: metadata["png_sha256"]
            .as_str()
            .unwrap_or(&"0".repeat(64))
            .into(),
        expected_generation: metadata["generation"].as_u64(),
    }
}
fn refused<T>(result: Result<T>, status: StatusCode, code: &str) {
    match result {
        Ok(_) => panic!("Expected {status} {code}"),
        Err((actual, Json(error))) => {
            assert_eq!(actual, status);
            assert_eq!(error.error, code);
        }
    }
}
async fn body(response: Response) -> (axum::http::HeaderMap, Vec<u8>) {
    assert_eq!(response.status(), StatusCode::OK);
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), PNG_BYTES + METADATA_BYTES)
        .await
        .unwrap()
        .to_vec();
    assert_eq!(headers[header::CONTENT_LENGTH], bytes.len().to_string());
    assert_eq!(headers["x-content-sha256"], digest(&bytes));
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    (headers, bytes)
}

#[test]
fn screenshot_png_crc_framing_order_and_decoded_limits_are_explicit() {
    let png = image(100, 100);
    assert_eq!(
        png_dimensions(&png).unwrap(),
        Dimensions {
            width: 100,
            height: 100
        }
    );
    let mut corrupt = png.clone();
    corrupt[50] ^= 1;
    assert_eq!(png_dimensions(&corrupt), Err("png_crc_mismatch"));
    assert_eq!(png_dimensions(b"not PNG"), Err("png_signature_invalid"));
    assert_eq!(
        png_dimensions(&png[..png.len() - 1]),
        Err("png_chunk_bounds_invalid")
    );
    let mut trailing = png.clone();
    trailing.push(0);
    assert_eq!(png_dimensions(&trailing), Err("png_iend_invalid"));
    let mut wide = b"\x89PNG\r\n\x1a\n".to_vec();
    wide.extend(ihdr(4097, 100));
    assert_eq!(png_dimensions(&wide), Err("png_dimensions_invalid"));
    let mut missing = b"\x89PNG\r\n\x1a\n".to_vec();
    missing.extend(ihdr(100, 100));
    missing.extend(chunk(b"IEND", &[]));
    assert_eq!(png_dimensions(&missing), Err("png_idat_missing"));
    let mut duplicate = b"\x89PNG\r\n\x1a\n".to_vec();
    duplicate.extend(ihdr(100, 100));
    duplicate.extend(ihdr(100, 100));
    assert_eq!(png_dimensions(&duplicate), Err("png_ihdr_invalid"));
    let mut critical = b"\x89PNG\r\n\x1a\n".to_vec();
    critical.extend(ihdr(100, 100));
    critical.extend(chunk(b"ABCD", &[]));
    assert_eq!(
        png_dimensions(&critical),
        Err("png_critical_chunk_unsupported")
    );
    let mut chunks = b"\x89PNG\r\n\x1a\n".to_vec();
    chunks.extend(ihdr(100, 100));
    for _ in 0..CHUNKS {
        chunks.extend(chunk(b"tEXt", &[]));
    }
    assert_eq!(png_dimensions(&chunks), Err("png_chunk_limit"));
    assert_eq!(
        png_dimensions(&vec![0; PNG_BYTES + 1]),
        Err("png_byte_limit")
    );
    assert_eq!(decode("%%%=").unwrap(), Err("invalid_base64"));
    let encoded = "A".repeat(PNG_BYTES.div_ceil(3) * 4 + 1);
    assert_eq!(decode(&encoded).unwrap(), Err("png_byte_limit"));
}

#[test]
fn screenshot_png_idat_must_be_contiguous_and_palette_color_must_be_valid() {
    let mut split = b"\x89PNG\r\n\x1a\n".to_vec();
    split.extend(ihdr(100, 100));
    split.extend(chunk(b"IDAT", &[1]));
    split.extend(chunk(b"tEXt", &[]));
    split.extend(chunk(b"IDAT", &[2]));
    assert_eq!(png_dimensions(&split), Err("png_chunk_order_invalid"));
    let mut palette = b"\x89PNG\r\n\x1a\n".to_vec();
    palette.extend(ihdr(100, 100));
    palette.extend(chunk(b"PLTE", &[0, 0, 0]));
    assert_eq!(png_dimensions(&palette), Err("png_palette_invalid"));
    let mut truncated = b"\x89PNG\r\n\x1a\n".to_vec();
    truncated.extend_from_slice(&u32::MAX.to_be_bytes());
    truncated.extend_from_slice(b"IDAT");
    truncated.extend_from_slice(&[0; 4]);
    assert_eq!(png_dimensions(&truncated), Err("png_chunk_bounds_invalid"));
}

#[test]
fn screenshot_missing_disabled_failed_corrupt_and_legacy_viewport_are_distinct() {
    let mut job = job(Uuid::new_v4());
    let available = info(&job, Some(0));
    assert_eq!(available["status"], "available");
    assert_eq!(available["reason"], Value::Null);
    assert_eq!(available["width"], 100);
    job.report.as_mut().unwrap().pages[0].screenshot_base64 = None;
    assert_eq!(info(&job, Some(0))["reason"], "screenshot_not_recorded");
    job.report.as_mut().unwrap().pages[0].rendered = false;
    assert_eq!(
        info(&job, Some(0))["reason"],
        "rendered_document_unavailable"
    );
    job.report.as_mut().unwrap().config.renderer = None;
    assert_eq!(info(&job, Some(0))["status"], "disabled");
    job.report.as_mut().unwrap().pages[0].renderer_error = Some("Owned failed render".into());
    assert_eq!(info(&job, Some(0))["status"], "render_failed");
    job.report.as_mut().unwrap().pages[0].renderer_error = None;
    job.report.as_mut().unwrap().pages[0].screenshot_base64 = Some("%%%=".into());
    assert_eq!(info(&job, Some(0))["reason"], "invalid_base64");
    job.report.as_mut().unwrap().pages[0].screenshot_base64 =
        Some(STANDARD.encode(image(100, 100)));
    job.report.as_mut().unwrap().pages[0].rendered_viewport = None;
    let legacy = info(&job, Some(0));
    assert_eq!(legacy["available"], true);
    assert_eq!(legacy["viewport"], Value::Null); // no invented default
    job.report.as_mut().unwrap().pages[0].rendered_viewport =
        Some(json!({"width":100,"height":101}));
    assert_eq!(info(&job, Some(0))["reason"], "viewport_mismatch");
    job.report.as_mut().unwrap().pages[0].rendered_viewport = Some(json!({"width":0,"height":100}));
    let range = info(&job, Some(0));
    assert_eq!(range["status"], "corrupt");
    assert_eq!(range["viewport"]["width"], 0);
    job.report.as_mut().unwrap().pages[0].renderer_error = Some("Owned combined failure".into());
    let failed = info(&job, Some(0));
    assert_eq!(failed["status"], "render_failed");
    assert_eq!(failed["reason"], "renderer_failed");
    assert_eq!(failed["viewport"], Value::Null);
    job.report.as_mut().unwrap().pages[0].renderer_error = None;
    job.report.as_mut().unwrap().pages[0].screenshot_base64 = None;
    let disabled = info(&job, Some(0));
    assert_eq!(disabled["status"], "disabled");
    assert_eq!(disabled["viewport"], Value::Null);
    job.report.as_mut().unwrap().config.renderer = Some(RendererConfig {
        screenshot: true,
        ..Default::default()
    });
    let missing = info(&job, Some(0));
    assert_eq!(missing["status"], "missing");
    assert_eq!(missing["reason"], "rendered_document_unavailable");
    assert_eq!(missing["viewport"], Value::Null);
    job.report.as_mut().unwrap().pages[0].rendered = true;
    assert_eq!(info(&job, Some(0))["reason"], "screenshot_not_recorded");
    job.report.as_mut().unwrap().pages[0].screenshot_base64 =
        Some(STANDARD.encode(image(100, 100)));
    let corrupt = info(&job, Some(0));
    assert_eq!(corrupt["status"], "corrupt");
    assert_eq!(corrupt["viewport"]["width"], 0);
    job.report.as_mut().unwrap().pages[0].rendered_viewport =
        Some(json!({"width":100,"height":100,"unknown":true}));
    let malformed = info(&job, Some(0));
    assert_eq!(malformed["reason"], "stored_viewport_invalid");
    assert_eq!(malformed["viewport"], Value::Null);
}

#[tokio::test]
async fn screenshot_handlers_return_original_png_integrity_and_bound_identity_without_files() {
    let directory = tempfile::tempdir().unwrap();
    let absent = directory.path().join("must-not-create");
    let job = job(Uuid::new_v4());
    let id = job.id;
    let url = job.report.as_ref().unwrap().pages[0].url.clone();
    let state = state(job, absent.clone());
    let (headers, metadata_bytes) = body(
        metadata(
            State(state.clone()),
            Path(id),
            Ok(Query(MetadataQuery {
                url: url.clone(),
                ordinal: None,
            })),
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(
        headers[header::CONTENT_TYPE],
        "application/json; charset=utf-8"
    );
    let metadata: Value = serde_json::from_slice(&metadata_bytes).unwrap();
    assert_eq!(metadata["engine"], ENGINE);
    assert_eq!(metadata["version"], 1);
    assert_eq!(metadata["ordinal"], 0);
    assert_eq!(metadata["generation"], 7);
    assert_eq!(metadata["viewport"], json!({"width":100,"height":100}));
    assert_eq!(metadata.as_object().unwrap().len(), 16);
    let (headers, bytes) = body(
        png(State(state.clone()), Path(id), Ok(Query(query(&metadata))))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(bytes, image(100, 100));
    assert_eq!(headers[header::CONTENT_TYPE], "image/png");
    assert_eq!(
        headers[header::CONTENT_DISPOSITION],
        "attachment; filename=screenshot.png"
    );
    assert_eq!(headers["x-screenshot-job-id"], id.to_string());
    assert_eq!(headers["x-screenshot-ordinal"], "0");
    assert_eq!(headers["x-screenshot-generation"], "7");
    assert_eq!(headers["x-screenshot-url-sha256"], digest(url.as_bytes()));
    assert_eq!(
        headers["x-screenshot-snapshot-sha256"],
        metadata["snapshot_sha256"].as_str().unwrap()
    );
    assert_eq!(headers["x-screenshot-image-sha256"], digest(&bytes));
    let identity = json!([
        ENGINE,
        id.to_string(),
        "0",
        url,
        "7",
        metadata["snapshot_sha256"],
        metadata["png_sha256"]
    ]);
    assert_eq!(
        headers["x-screenshot-identity-sha256"],
        digest(&serde_json::to_vec(&identity).unwrap())
    );
    assert!(!absent.exists());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}

#[test]
fn screenshot_tokens_refuse_stale_image_report_generation_and_identical_foreign_job() {
    let mut original = job(Uuid::new_v4());
    let first = info(&original, Some(0));
    let request = query(&first);
    let mut foreign = job(Uuid::new_v4());
    assert_eq!(
        serde_json::to_vec(original.report.as_ref().unwrap()).unwrap(),
        serde_json::to_vec(foreign.report.as_ref().unwrap()).unwrap()
    );
    let foreign_meta = info(&foreign, Some(0));
    assert_ne!(first["snapshot_sha256"], foreign_meta["snapshot_sha256"]);
    refused(
        png_response(inspect(&foreign, &request.url, Some(0)).unwrap(), &request),
        StatusCode::CONFLICT,
        "screenshot_snapshot_changed",
    );
    foreign.id = original.id;
    foreign.generation = 8;
    refused(
        png_response(inspect(&foreign, &request.url, Some(0)).unwrap(), &request),
        StatusCode::CONFLICT,
        "screenshot_snapshot_changed",
    );
    let mut wrong_image = query(&first);
    wrong_image.expected_image_sha256 = "0".repeat(64);
    refused(
        png_response(
            inspect(&original, &request.url, Some(0)).unwrap(),
            &wrong_image,
        ),
        StatusCode::CONFLICT,
        "screenshot_image_changed",
    );
    let mut omitted = query(&first);
    omitted.expected_generation = None;
    refused(
        png_response(inspect(&original, &request.url, Some(0)).unwrap(), &omitted),
        StatusCode::CONFLICT,
        "screenshot_snapshot_changed",
    );
    original.report.as_mut().unwrap().pages[0]
        .titles
        .push("changed saved metadata".into());
    refused(
        png_response(inspect(&original, &request.url, Some(0)).unwrap(), &request),
        StatusCode::CONFLICT,
        "screenshot_snapshot_changed",
    );
    original.generation = 0;
    let imported = info(&original, Some(0));
    assert_eq!(imported["generation"], Value::Null);
    let imported_request = query(&imported);
    assert!(
        png_response(
            inspect(&original, &request.url, Some(0)).unwrap(),
            &imported_request
        )
        .is_ok()
    );
    let mut guessed = query(&imported);
    guessed.expected_generation = Some(7);
    refused(
        png_response(inspect(&original, &request.url, Some(0)).unwrap(), &guessed),
        StatusCode::CONFLICT,
        "screenshot_snapshot_changed",
    );
}

#[test]
fn screenshot_duplicate_urls_require_ordinal_and_preserve_selected_actual_image() {
    let mut job = job(Uuid::new_v4());
    let report = job.report.as_mut().unwrap();
    let mut second = report.pages[0].clone();
    second.screenshot_base64 = Some(STANDARD.encode(image(101, 100)));
    second.rendered_viewport = Some(json!({"width":101,"height":100}));
    second.final_url.push_str("-second");
    report.pages.push(second);
    let url = report.pages[0].url.clone();
    refused(
        inspect(&job, &url, None),
        StatusCode::CONFLICT,
        "screenshot_url_ambiguous",
    );
    let first = info(&job, Some(0));
    let second = info(&job, Some(1));
    assert_eq!(second["ordinal"], 1);
    assert_eq!(second["width"], 101);
    assert_ne!(first["png_sha256"], second["png_sha256"]);
    assert_eq!(second["final_url"], "https://owned.test/final-second");
    let mut swapped = query(&first);
    swapped.ordinal = 1;
    refused(
        png_response(inspect(&job, &url, Some(1)).unwrap(), &swapped),
        StatusCode::CONFLICT,
        "screenshot_image_changed",
    );
    refused(
        inspect(&job, &url, Some(2)),
        StatusCode::NOT_FOUND,
        "screenshot_page_not_found",
    );
    refused(
        inspect(&job, "https://owned.test/other", Some(0)),
        StatusCode::CONFLICT,
        "screenshot_identity_changed",
    );
}

#[tokio::test]
async fn screenshot_query_stability_capacity_and_unavailable_downloads_fail_explicitly() {
    for uri in [
        "/?url=a&unknown=true",
        "/?url=a&url=b",
        "/?url=a&ordinal=-1",
    ] {
        assert!(Query::<MetadataQuery>::try_from_uri(&uri.parse::<Uri>().unwrap()).is_err());
    }
    for uri in [
        "/?url=a&ordinal=0",
        "/?url=a&ordinal=0&expected_snapshot_sha256=x&expected_image_sha256=y&extra=1",
    ] {
        assert!(Query::<PngQuery>::try_from_uri(&uri.parse::<Uri>().unwrap()).is_err());
    }
    let mut job = job(Uuid::new_v4());
    let url = job.report.as_ref().unwrap().pages[0].url.clone();
    job.status = "running".into();
    refused(
        inspect(&job, &url, Some(0)),
        StatusCode::CONFLICT,
        "screenshot_snapshot_unavailable",
    );
    job.status = "completed".into();
    let directory = tempfile::tempdir().unwrap();
    let id = job.id;
    let state = state(job, directory.path().to_path_buf());
    let one = state
        .lifecycle
        .config_profile_gate
        .clone()
        .try_acquire_owned()
        .unwrap();
    let two = state
        .lifecycle
        .config_profile_gate
        .clone()
        .try_acquire_owned()
        .unwrap();
    refused(
        metadata(
            State(state.clone()),
            Path(id),
            Ok(Query(MetadataQuery {
                url: url.clone(),
                ordinal: Some(0),
            })),
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE,
        "screenshot_preparation_busy",
    );
    drop(one);
    drop(two);
    refused(
        metadata(
            State(state.clone()),
            Path(Uuid::new_v4()),
            Ok(Query(MetadataQuery {
                url: url.clone(),
                ordinal: Some(0),
            })),
        )
        .await,
        StatusCode::NOT_FOUND,
        "screenshot_job_not_found",
    );
    refused(
        metadata(
            State(state.clone()),
            Path(id),
            Ok(Query(MetadataQuery {
                url: String::new(),
                ordinal: None,
            })),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "invalid_screenshot_request",
    );
    let source = get_job(&state, id)
        .await
        .unwrap_or_else(|_| panic!("fixture job"));
    let saved = {
        let guard = source.read().await;
        info(&guard, Some(0))
    };
    let request = query(&saved);
    source.write().await.report.as_mut().unwrap().pages[0].screenshot_base64 =
        Some("corrupt".into());
    let current = {
        let guard = source.read().await;
        info(&guard, Some(0))
    };
    let mut unavailable = request;
    unavailable.expected_snapshot_sha256 = current["snapshot_sha256"].as_str().unwrap().into();
    refused(
        png(State(state.clone()), Path(id), Ok(Query(unavailable))).await,
        StatusCode::UNPROCESSABLE_ENTITY,
        "screenshot_unavailable",
    );
    let mut guard = source.write().await;
    guard.report.as_mut().unwrap().pages[0].screenshot_base64 = None;
    let absent = info(&guard, Some(0));
    let unavailable = query(&absent);
    drop(guard);
    refused(
        png(State(state.clone()), Path(id), Ok(Query(unavailable))).await,
        StatusCode::NOT_FOUND,
        "screenshot_unavailable",
    );
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}

#[test]
fn screenshot_metadata_byte_limit_does_not_emit_truncated_success() {
    let mut job = job(Uuid::new_v4());
    job.report.as_mut().unwrap().pages[0].final_url = "x".repeat(METADATA_BYTES);
    let observation = inspect(&job, &job.report.as_ref().unwrap().pages[0].url, Some(0)).unwrap();
    refused(
        metadata_response(&observation),
        StatusCode::PAYLOAD_TOO_LARGE,
        "screenshot_metadata_limit",
    );
}
