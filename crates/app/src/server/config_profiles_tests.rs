use super::*;
use axum::body::{Body, to_bytes};

fn state(path: &std::path::Path) -> AppState {
    AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Default::default(),
        data_dir: Arc::new(path.to_path_buf()),
        exports: Default::default(),
        finding_reads: Default::default(),
        lifecycle: Default::default(),
    }
}

fn request(value: Value) -> Request {
    Request::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&value).unwrap()))
        .unwrap()
}

fn portable(config: &str) -> String {
    format!("{{\"version\":1,\"config\":{config},\"required_headers\":[]}}")
}

fn accepted<T>(value: ApiResult<T>) -> T {
    value.unwrap_or_else(|error| panic!("Unexpected profile refusal: {}", error.1.0.error))
}

#[test]
fn config_profile_app_patches_keep_native_uint64_and_untouched_nested_options() {
    let raw = portable(
        r#"{"seed_urls":["http://127.0.0.1:1/a"],"max_pages":9007199254740993,"max_body_bytes":1572865,"renderer":{"max_response_bytes":9007199254740993},"sitemap_source_processing":null}"#,
    );
    let request: ProfileRequest = profile::parse_strict_json(
        &serde_json::to_vec(&json!({"profile_text":raw,"patches":[
            {"path":["concurrency"],"value_json":"65"},
            {"path":["renderer","wait_ms"],"value_json":"501"}
        ]}))
        .unwrap(),
        MAX_REQUEST_BYTES,
    )
    .unwrap();
    let changed = edited(request).unwrap();
    assert_eq!(
        changed.config["max_pages"].as_u64(),
        Some(9_007_199_254_740_993)
    );
    assert_eq!(changed.config["max_body_bytes"], 1_572_865);
    assert_eq!(
        changed.config["renderer"]["max_response_bytes"].as_u64(),
        Some(9_007_199_254_740_993)
    );
    assert_eq!(changed.config["renderer"]["wait_ms"], 501);
    assert_eq!(changed.config["concurrency"], 65);
    assert!(
        changed
            .config
            .as_object()
            .unwrap()
            .contains_key("sitemap_source_processing")
    );
    assert!(changed.config["sitemap_source_processing"].is_null());
    assert!(
        !changed
            .config
            .as_object()
            .unwrap()
            .contains_key("crawl_policy")
    );
    let changed = apply_patches(
        changed,
        vec![ConfigPatch {
            path: vec!["max_pages".into()],
            value_json: Some("9007199254740995".into()),
            remove: false,
        }],
    )
    .unwrap();
    assert_eq!(
        changed.config["max_pages"].as_u64(),
        Some(9_007_199_254_740_995)
    );
}

#[test]
fn config_profile_app_strict_envelopes_and_bad_patches_never_accept_partial_changes() {
    for bytes in [
        br#"{"profile_text":"{}","profile_text":"{}"}"#.as_slice(),
        br#"{"profile_text":"{}","unknown":1}"#,
        br#"["{}"]"#,
        br#"{"profile_text":"{}","patches":[{"path":[],"value_json":"{}","other":true}]}"#,
    ] {
        assert!(profile::parse_strict_json::<ProfileRequest>(bytes, MAX_REQUEST_BYTES).is_err());
    }
    let original =
        profile::parse_config_profile(portable(r#"{"max_body_bytes":1572865}"#).as_bytes())
            .unwrap();
    let before = profile::export_config_profile(&original).unwrap();
    for patch in [
        ConfigPatch {
            path: vec!["unknown".into()],
            value_json: Some("1".into()),
            remove: false,
        },
        ConfigPatch {
            path: vec!["renderer".into(), "wait_ms".into()],
            value_json: Some("1".into()),
            remove: false,
        },
        ConfigPatch {
            path: vec![],
            value_json: Some("[]".into()),
            remove: false,
        },
        ConfigPatch {
            path: vec![],
            value_json: None,
            remove: true,
        },
        ConfigPatch {
            path: vec!["max_pages".into()],
            value_json: Some("1".into()),
            remove: true,
        },
        ConfigPatch {
            path: vec!["max_pages".into()],
            value_json: Some("{\"x\":1,\"x\":2}".into()),
            remove: false,
        },
    ] {
        assert!(
            apply_patches(
                original.clone(),
                vec![
                    ConfigPatch {
                        path: vec!["concurrency".into()],
                        value_json: Some("65".into()),
                        remove: false
                    },
                    patch,
                ]
            )
            .is_err()
        );
        assert_eq!(profile::export_config_profile(&original).unwrap(), before);
    }
}

#[tokio::test]
async fn config_profile_app_handlers_are_pure_and_export_only_header_requirements() {
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path());
    let raw_config = r#"{"seed_urls":["http://127.0.0.1:1/a"],"custom_headers":{"Authorization":"OWNED_PROFILE_HEADER_SECRET","__proto__":"OWNED_PROTO_HEADER_SECRET"},"max_body_bytes":1572865}"#;
    let response = accepted(
        from_config(
            State(state.clone()),
            request(json!({"config_text":raw_config})),
        )
        .await,
    );
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), MAX_REPLY_BYTES)
        .await
        .unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(!text.contains("OWNED_PROFILE_HEADER_SECRET"));
    assert!(!text.contains("OWNED_PROTO_HEADER_SECRET"));
    let inspected: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        inspected["missing_headers"],
        json!(["Authorization", "__proto__"])
    );
    let response = accepted(
        export(
            State(state.clone()),
            request(json!({"profile_text":inspected["profile_json"]})),
        )
        .await,
    );
    let length: usize = response.headers()[header::CONTENT_LENGTH]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    let digest = response.headers()["x-content-sha256"]
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let exported = to_bytes(response.into_body(), profile::CONFIG_PROFILE_BYTES)
        .await
        .unwrap();
    assert_eq!(exported.len(), length);
    assert_eq!(format!("{:x}", Sha256::digest(&exported)), digest);
    let exported: Value = serde_json::from_slice(&exported).unwrap();
    assert_eq!(exported["config"]["custom_headers"], json!({}));
    assert_eq!(
        exported["required_headers"],
        json!(["Authorization", "__proto__"])
    );
    assert_eq!(exported["config"]["max_body_bytes"], 1_572_865);
    assert!(state.jobs.read().await.is_empty());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}

#[tokio::test]
async fn config_profile_app_start_refuses_missing_targets_headers_and_invalid_patches_before_reservation()
 {
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path());
    let unresolved = r#"{"version":1,"config":{"seed_urls":["http://127.0.0.1:1/a"]},"required_headers":["Authorization"]}"#;
    for value in [
        json!({"profile_text":portable("{}")}),
        json!({"profile_text":unresolved}),
        json!({"profile_text":unresolved,"header_values":{"Authorization":"bad\r\nvalue"}}),
        json!({"profile_text":portable("{}"),"patches":[{"path":["unknown"],"value_json":"true"}]}),
    ] {
        let error = prepare_launch(
            state.clone(),
            accepted(slot(&state)),
            Bytes::from(serde_json::to_vec(&value).unwrap()),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(error.0, StatusCode::BAD_REQUEST);
        assert!(state.jobs.read().await.is_empty());
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    }
    state.lifecycle.blocking.shutdown_and_drain().await.unwrap();
}

#[tokio::test]
async fn config_profile_app_media_and_slot_refusals_do_not_consume_request_bodies() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let temp = tempfile::tempdir().unwrap();
    let state = state(temp.path());
    let polls = Arc::new(AtomicUsize::new(0));
    let make_body = |polls: Arc<AtomicUsize>| {
        Body::from_stream(futures::stream::once(async move {
            polls.fetch_add(1, Ordering::SeqCst);
            Ok::<_, std::io::Error>(Bytes::from_static(b"owned body must not be consumed"))
        }))
    };
    let invalid = Request::builder()
        .header(header::CONTENT_TYPE, "text/plain")
        .body(make_body(polls.clone()))
        .unwrap();
    assert_eq!(
        body(&state, invalid).await.err().unwrap().0,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    let first = accepted(slot(&state));
    let second = accepted(slot(&state));
    let busy = Request::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(make_body(polls.clone()))
        .unwrap();
    assert_eq!(
        body(&state, busy).await.err().unwrap().0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    drop((first, second));
    assert_eq!(state.lifecycle.config_profile_gate.available_permits(), 2);
    assert!(state.jobs.read().await.is_empty());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}
