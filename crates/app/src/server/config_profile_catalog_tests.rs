use super::*;
use axum::body::{Body, to_bytes};
use sha2::Digest;

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
    raw_request(&serde_json::to_string(&value).unwrap())
}
fn raw_request(value: &str) -> Request {
    Request::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}
fn portable() -> &'static str {
    r#"{"version":1,"config":{"delay_ms":9007199254740993,"custom_search":null},"required_headers":["Authorization","authorization"]}"#
}
fn db(state: &AppState) -> PathBuf {
    state
        .data_dir
        .join("configuration-profiles")
        .join("catalog.sqlite")
}
fn accepted<T>(result: ApiResult<T>) -> T {
    result.unwrap_or_else(|error| {
        panic!(
            "Unexpected catalog refusal {}: {}",
            error.0, error.1.0.error
        )
    })
}
fn refused<T>(result: ApiResult<T>, status: StatusCode) {
    match result {
        Ok(_) => panic!("Expected refusal {status}"),
        Err(error) => assert_eq!(error.0, status, "{}", error.1.0.error),
    }
}
async fn value(result: ApiResult<Response>) -> Value {
    let response = accepted(result);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-store"
    );
    serde_json::from_slice(
        &to_bytes(response.into_body(), MAX_REPLY_BYTES)
            .await
            .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn catalog_api_absent_and_validation_rejections_have_no_files() {
    let root = tempfile::tempdir().unwrap();
    let state = state(root.path());
    let listing = value(list(State(state.clone())).await).await;
    assert_eq!(listing["engine"], CONFIG_PROFILE_CATALOG_ENGINE);
    assert_eq!(listing["version"], 1);
    assert_eq!(listing["scope"], "selected_application_data_directory");
    assert_eq!(listing["entries"], json!([]));
    assert_eq!(
        listing["limits"],
        json!({"max_entries":128,"max_name_bytes":256,"max_profile_bytes":67108864,"max_total_profile_bytes":268435456})
    );
    refused(
        get(State(state.clone()), Path(Uuid::new_v4())).await,
        StatusCode::NOT_FOUND,
    );
    for raw in [
        r#"{"name":"a","name":"b","profile_text":"{}"}"#,
        r#"{"name":"a","profile_text":"{}","unknown":true}"#,
        r#"{"name":"a","profile_text":"{\"version\":2,\"config\":{}}"}"#,
        r#"{"name":"a","profile_text":"{\"version\":1,\"config\":{\"custom_headers\":{\"X-Secret\":\"sentinel\"}}}"}"#,
    ] {
        refused(
            create(State(state.clone()), raw_request(raw)).await,
            StatusCode::BAD_REQUEST,
        );
    }
    refused(
        create(
            State(state.clone()),
            request(json!({"name":" \n ","profile_text":portable()})),
        )
        .await,
        StatusCode::BAD_REQUEST,
    );
    let wrong_type = Request::builder()
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from("{}"))
        .unwrap();
    refused(
        create(State(state.clone()), wrong_type).await,
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
    );
    assert!(!db(&state).parent().unwrap().exists());
    assert!(state.jobs.read().await.is_empty());
}

#[tokio::test]
async fn catalog_api_cold_preview_exactness_and_cas_lifecycle() {
    let root = tempfile::tempdir().unwrap();
    let state = state(root.path());
    let created = value(
        create(
            State(state.clone()),
            request(json!({"name":"  Native Ä  ","profile_text":portable()})),
        )
        .await,
    )
    .await;
    let entry = created["entry"].clone();
    let id = Uuid::parse_str(entry["id"].as_str().unwrap()).unwrap();
    assert_eq!(entry["revision"], "1");
    assert_eq!(entry["name"], "  Native Ä  ");
    let read = value(get(State(self::state(root.path())), Path(id)).await).await;
    assert_eq!(read["entry"], entry);
    let preview = &read["profile"];
    assert_eq!(
        preview["missing_headers"],
        json!(["Authorization", "authorization"])
    );
    assert!(preview["requires_targets"].as_bool().unwrap());
    assert!(
        preview["config_json"]
            .as_str()
            .unwrap()
            .contains("9007199254740993")
    );
    let canonical = preview["profile_json"].as_str().unwrap();
    assert_eq!(
        entry["profile_bytes"].as_u64().unwrap(),
        canonical.len() as u64
    );
    assert_eq!(
        entry["profile_sha256"],
        format!("{:x}", sha2::Sha256::digest(canonical.as_bytes()))
    );
    assert_eq!(
        preview["effective_config_json"].as_str().unwrap(),
        config_profiles::preview_value(
            profile::parse_config_profile(portable().as_bytes()).unwrap()
        )
        .unwrap()["effective_config_json"]
            .as_str()
            .unwrap()
    );
    let before = std::fs::read(db(&state)).unwrap();
    for (revision, hash) in [
        ("2", entry["profile_sha256"].as_str().unwrap()),
        (
            "1",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ),
    ] {
        refused(
            rename(
                State(state.clone()),
                Path(id),
                request(
                    json!({"expected_revision":revision,"expected_sha256":hash,"name":"rejected"}),
                ),
            )
            .await,
            StatusCode::CONFLICT,
        );
    }
    assert_eq!(std::fs::read(db(&state)).unwrap(), before);
    let renamed=value(rename(State(state.clone()),Path(id),request(json!({"expected_revision":"1","expected_sha256":entry["profile_sha256"],"name":"Renamed"}))).await).await;
    assert_eq!(renamed["entry"]["revision"], "2");
    assert_eq!(renamed["entry"]["profile_sha256"], entry["profile_sha256"]);
    refused(
        delete(
            State(state.clone()),
            Path(id),
            request(json!({"expected_revision":"1","expected_sha256":entry["profile_sha256"]})),
        )
        .await,
        StatusCode::CONFLICT,
    );
    refused(replace(State(state.clone()),Path(id),request(json!({"expected_revision":2,"expected_sha256":entry["profile_sha256"],"name":"no","profile_text":portable()}))).await,StatusCode::BAD_REQUEST);
    let replaced=value(replace(State(state.clone()),Path(id),request(json!({"expected_revision":"2","expected_sha256":entry["profile_sha256"],"name":"Replaced","profile_text":"{\"version\":1,\"config\":{}}"}))).await).await;
    assert_eq!(replaced["entry"]["revision"], "3");
    assert_ne!(replaced["entry"]["profile_sha256"], entry["profile_sha256"]);
    let deleted=value(delete(State(state.clone()),Path(id),request(json!({"expected_revision":"3","expected_sha256":replaced["entry"]["profile_sha256"]}))).await).await;
    assert_eq!(deleted["deleted_id"], id.to_string());
    assert_eq!(
        value(list(State(state.clone())).await).await["entries"],
        json!([])
    );
    assert!(state.jobs.read().await.is_empty());
}

#[tokio::test]
async fn catalog_api_corrupt_and_unsupported_stores_are_unavailable_not_reset() {
    let root = tempfile::tempdir().unwrap();
    let state = state(root.path());
    let path = db(&state);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"").unwrap();
    refused(
        list(State(state.clone())).await,
        StatusCode::SERVICE_UNAVAILABLE,
    );
    refused(
        create(
            State(state.clone()),
            request(json!({"name":"valid","profile_text":portable()})),
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE,
    );
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
    std::fs::remove_file(&path).unwrap();
    let created = value(
        create(
            State(state.clone()),
            request(json!({"name":"valid","profile_text":portable()})),
        )
        .await,
    )
    .await;
    // user_version is part of SQLite's documented file header (offset60..64).
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[60..64].copy_from_slice(&2u32.to_be_bytes());
    std::fs::write(&path, &bytes).unwrap();
    refused(
        list(State(state.clone())).await,
        StatusCode::SERVICE_UNAVAILABLE,
    );
    refused(
        get(
            State(state.clone()),
            Path(Uuid::parse_str(created["entry"]["id"].as_str().unwrap()).unwrap()),
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE,
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[tokio::test]
async fn catalog_api_shared_slots_and_shutdown_admission_precede_writes() {
    let root = tempfile::tempdir().unwrap();
    let state = state(root.path());
    let a = state
        .lifecycle
        .config_profile_gate
        .clone()
        .try_acquire_owned()
        .unwrap();
    let b = state
        .lifecycle
        .config_profile_gate
        .clone()
        .try_acquire_owned()
        .unwrap();
    refused(
        list(State(state.clone())).await,
        StatusCode::SERVICE_UNAVAILABLE,
    );
    refused(
        create(
            State(state.clone()),
            request(json!({"name":"no","profile_text":portable()})),
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE,
    );
    drop((a, b));
    lifecycle::close_admission(&state);
    refused(
        create(
            State(state.clone()),
            request(json!({"name":"no","profile_text":portable()})),
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE,
    );
    assert!(!db(&state).parent().unwrap().exists());
}

#[test]
fn catalog_api_count_and_encode_budget_exact_boundary_is_atomic() {
    let value = json!({"utf8":"路\\\"\n","rows":[1,2,3]});
    let exact = serde_json::to_vec(&value).unwrap();
    accepted(measure(&value, exact.len()));
    refused(
        measure(&value, exact.len() - 1),
        StatusCode::PAYLOAD_TOO_LARGE,
    );
    let mut bounded = Budget {
        bytes: Some(Vec::new()),
        length: 0,
        cap: exact.len(),
    };
    serde_json::to_writer(&mut bounded, &value).unwrap();
    assert_eq!(bounded.bytes.unwrap(), exact);
    let mut refused_output = Budget {
        bytes: Some(Vec::new()),
        length: 0,
        cap: exact.len() - 1,
    };
    assert!(serde_json::to_writer(&mut refused_output, &value).is_err());
    assert!(refused_output.length < exact.len());
}
