use super::*;
use axum::body::to_bytes;
use open_seo_spider_core::{SitemapGenerationLastmod, SitemapProvidedLastmod};
fn report() -> CrawlReport {
    let identity = "https://example.org/p?a=1&b=2";
    CrawlReport {
        config: CrawlConfig {
            seed_urls: vec!["https://example.org/".into()],
            ..Default::default()
        },
        pages: vec![PageRecord {
            url: identity.into(),
            final_url: identity.into(),
            status: Some(200),
            final_status: Some(200),
            indexable: true,
            internal: true,
            content_type: "text/html".into(),
            ..Default::default()
        }],
        issues: vec![],
        summary: Default::default(),
        warnings: vec![],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
    }
}
async fn fixture() -> (AppState, Uuid, PathBuf) {
    let directory = std::env::temp_dir().join(format!("oss-generation-{}", Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let state = AppState {
        mcp: crate::mcp::Bridge::new("http://127.0.0.1:1").unwrap(),
        jobs: Arc::new(RwLock::new(HashMap::new())),
        data_dir: Arc::new(directory.clone()),
    };
    let id = Uuid::new_v4();
    persist_import(&state, id, report())
        .await
        .unwrap_or_else(|(_, Json(error))| panic!("{}", error.error));
    (state, id, directory)
}
fn options() -> SitemapGenerationOptions {
    SitemapGenerationOptions {
        public_base_url: "https://example.org/".into(),
        ..Default::default()
    }
}
fn bytes(path: &std::path::Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(path)
        .unwrap()
        .map(|file| {
            let file = file.unwrap();
            (
                file.file_name().to_string_lossy().into_owned(),
                std::fs::read(file.path()).unwrap(),
            )
        })
        .collect()
}
use std::collections::BTreeMap;

#[tokio::test]
async fn actual_saved_import_export_is_stable_hash_bound_and_readonly() {
    use sha2::{Digest, Sha256};
    let (state, id, directory) = fixture().await;
    let before = bytes(&directory);
    let response = bundle(State(state.clone()), Path(id), Ok(Json(options())))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/zip");
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 268435456).await.unwrap();
    assert_eq!(headers[header::CONTENT_LENGTH], body.len().to_string());
    assert_eq!(
        headers["x-sitemap-bundle-sha256"],
        format!("{:x}", Sha256::digest(&body))
    );
    assert_eq!(headers["x-sitemap-generated-urls"], "1");
    assert_eq!(
        headers["x-sitemap-generation-engine"],
        open_seo_spider_core::SITEMAP_GENERATION_ENGINE
    );
    let again = bundle(State(state), Path(id), Ok(Json(options())))
        .await
        .unwrap();
    assert_eq!(body, to_bytes(again.into_body(), 268435456).await.unwrap());
    assert_eq!(before, bytes(&directory));
}
#[tokio::test]
async fn invalid_options_dates_empty_eligibility_and_bounds_have_exact_status_without_files() {
    let (state, id, directory) = fixture().await;
    let before = bytes(&directory);
    let mut bad = options();
    bad.public_base_url = "https://user@example.org/".into();
    assert_eq!(
        bundle(State(state.clone()), Path(id), Ok(Json(bad)))
            .await
            .unwrap_err()
            .0,
        StatusCode::BAD_REQUEST
    );
    let mut bad = options();
    bad.lastmod = SitemapGenerationLastmod::Provided {
        values: vec![SitemapProvidedLastmod {
            page_url: report().pages[0].url.clone(),
            value: "2023-02-29".into(),
            reference: "manual record".into(),
        }],
    };
    assert_eq!(
        bundle(State(state.clone()), Path(id), Ok(Json(bad)))
            .await
            .unwrap_err()
            .0,
        StatusCode::BAD_REQUEST
    );
    let mut bad = options();
    bad.public_base_url = "https://other.example.org/".into();
    let failure = bundle(State(state.clone()), Path(id), Ok(Json(bad)))
        .await
        .unwrap_err();
    assert_eq!(failure.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        failure
            .1
            .0
            .diagnostic_counts
            .contains_key(&open_seo_spider_core::SitemapGenerationDiagnosticCode::WrongOrigin)
    );
    let mut bad = options();
    bad.max_urlset_uncompressed_bytes = 1;
    assert_eq!(
        bundle(State(state.clone()), Path(id), Ok(Json(bad)))
            .await
            .unwrap_err()
            .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        bundle(State(state), Path(Uuid::new_v4()), Ok(Json(options())))
            .await
            .unwrap_err()
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(before, bytes(&directory));
}
