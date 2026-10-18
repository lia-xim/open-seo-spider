//! Local transport fixtures prove native request/response binding, not real
//! Chromium screenshot dimensions. The worker E2E owns the actual PNG proof.
use super::*;
use crate::{RendererViewport, config_profile};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[test]
fn renderer_viewport_requires_complete_object_and_strict_unsigned_dimensions() {
    for raw in [
        r#"{"width":100,"height":4096}"#,
        r#"{"width":4096,"height":100}"#,
    ] {
        let viewport: RendererViewport = serde_json::from_str(raw).unwrap();
        let renderer = RendererConfig {
            viewport: Some(viewport),
            screenshot: false,
            ..Default::default()
        };
        validate_renderer(&renderer, 30).unwrap();
    }
    for raw in [
        r#"{}"#,
        r#"{"width":320}"#,
        r#"{"height":800}"#,
        r#"{"width":320,"height":800,"scale":1}"#,
        r#"{"width":320,"width":321,"height":800}"#,
        r#"{"width":"320","height":800}"#,
        r#"{"width":320.0,"height":800}"#,
        r#"{"width":true,"height":800}"#,
        r#"{"width":null,"height":800}"#,
        r#"{"width":-1,"height":800}"#,
        r#"{"width":4294967296,"height":800}"#,
        r#"[320,800]"#,
        r#"null"#,
    ] {
        assert!(
            serde_json::from_str::<RendererViewport>(raw).is_err(),
            "Accepted {raw}"
        );
        let profile = format!(
            "{{\"version\":1,\"config\":{{\"renderer\":{{\"viewport\":{raw},\"screenshot\":false}}}}}}"
        );
        // An outer optional null is intentionally the same as omission.
        if raw != "null" {
            assert!(
                config_profile::parse_config_profile(profile.as_bytes()).is_err(),
                "Profile accepted {raw}"
            );
        }
    }
}

#[test]
fn renderer_viewport_range_is_purely_validated_even_without_screenshot() {
    for (width, height) in [
        (0, 800),
        (99, 800),
        (4097, 800),
        (320, 0),
        (320, 99),
        (320, 4097),
        (u32::MAX, u32::MAX),
    ] {
        let renderer = RendererConfig {
            viewport: Some(RendererViewport { width, height }),
            screenshot: false,
            ..Default::default()
        };
        assert!(
            validate_renderer(&renderer, 30)
                .unwrap_err()
                .to_string()
                .contains("viewport")
        );
        let profile = format!(
            "{{\"version\":1,\"config\":{{\"renderer\":{{\"viewport\":{{\"width\":{width},\"height\":{height}}},\"screenshot\":false}}}}}}"
        );
        assert!(
            config_profile::parse_config_profile(profile.as_bytes())
                .unwrap_err()
                .to_string()
                .contains("viewport")
        );
        let config = CrawlConfig {
            seed_urls: vec!["http://127.0.0.1:1/never-requested".into()],
            renderer: Some(renderer),
            ..Default::default()
        };
        assert!(
            crate::Crawler::new(config)
                .err()
                .unwrap()
                .to_string()
                .contains("viewport")
        );
    }
}

#[test]
fn renderer_viewport_omission_keeps_legacy_renderer_and_profile_bytes() {
    const OLD_RENDERER: &str = r#"{"endpoint":"http://127.0.0.1:8790/render","wait_ms":500,"accessibility":false,"screenshot":false,"custom_javascript":[],"collect_cookies":false,"max_response_bytes":33554432}"#;
    assert_eq!(
        serde_json::to_string(&RendererConfig::default()).unwrap(),
        OLD_RENDERER
    );
    let restored: RendererConfig = serde_json::from_str(OLD_RENDERER).unwrap();
    assert!(restored.viewport.is_none());
    assert_eq!(serde_json::to_string(&restored).unwrap(), OLD_RENDERER);
    const OLD_PROFILE: &str = r#"{"version":1,"config":{"renderer":{"accessibility":false,"collect_cookies":false,"custom_javascript":[],"endpoint":"http://127.0.0.1:8790/render","max_response_bytes":33554432,"screenshot":false,"wait_ms":500}},"required_headers":[]}"#;
    let portable = config_profile::parse_config_profile(OLD_PROFILE.as_bytes()).unwrap();
    assert_eq!(
        config_profile::export_config_profile(&portable).unwrap(),
        OLD_PROFILE.as_bytes()
    );
    let preview = config_profile::preview_config_profile(&portable).unwrap();
    assert!(
        serde_json::to_value(preview.effective_config.renderer.unwrap())
            .unwrap()
            .get("viewport")
            .is_none()
    );
    let typed = CrawlConfig {
        renderer: Some(restored),
        ..Default::default()
    };
    let native_portable = config_profile::config_profile_from_config(&typed).unwrap();
    assert_eq!(
        native_portable.config["renderer"],
        serde_json::from_str::<Value>(OLD_RENDERER).unwrap()
    );
    let null: RendererConfig = serde_json::from_str(r#"{"viewport":null}"#).unwrap();
    assert_eq!(serde_json::to_string(&null).unwrap(), OLD_RENDERER);
}

#[test]
fn renderer_viewport_roundtrips_exactly_through_portable_and_saved_config() {
    let raw = r#"{"version":1,"config":{"renderer":{"screenshot":true,"viewport":{"width":320,"height":4096}}},"required_headers":[]}"#;
    let profile = config_profile::parse_config_profile(raw.as_bytes()).unwrap();
    let expected = RendererViewport {
        width: 320,
        height: 4096,
    };
    assert_eq!(
        config_profile::preview_config_profile(&profile)
            .unwrap()
            .effective_config
            .renderer
            .unwrap()
            .viewport,
        Some(expected)
    );
    let exported = config_profile::export_config_profile(&profile).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&exported).unwrap()["config"]["renderer"]["viewport"],
        json!({"width":320,"height":4096})
    );
    let reimported = config_profile::parse_config_profile(&exported).unwrap();
    assert_eq!(
        config_profile::export_config_profile(&reimported).unwrap(),
        exported
    );
}

async fn fixture(reported: Option<Value>) -> (String, tokio::task::JoinHandle<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/render", listener.local_addr().unwrap());
    let worker = tokio::spawn(async move {
        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(10), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut bytes = Vec::new();
        let header_end = loop {
            let mut chunk = [0; 4096];
            let read = stream.read(&mut chunk).await.unwrap();
            assert_ne!(read, 0);
            bytes.extend_from_slice(&chunk[..read]);
            assert!(bytes.len() < 65536);
            if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                break end + 4;
            }
        };
        let headers = std::str::from_utf8(&bytes[..header_end]).unwrap();
        assert!(headers.starts_with("POST /render HTTP/1.1\r\n"));
        let length = headers
            .lines()
            .find_map(|line| {
                line.split_once(':')
                    .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .map(|(_, value)| value.trim().parse::<usize>().unwrap())
            })
            .unwrap();
        while bytes.len() < header_end + length {
            let mut chunk = [0; 4096];
            let read = stream.read(&mut chunk).await.unwrap();
            assert_ne!(read, 0);
            bytes.extend_from_slice(&chunk[..read]);
        }
        assert_eq!(bytes.len(), header_end + length);
        let request: Value = serde_json::from_slice(&bytes[header_end..]).unwrap();
        let mut response = json!({"html":"<html><head><title>Local viewport fixture</title></head><body>Owned</body></html>","final_url":"http://127.0.0.1:1/page","status":200,"screenshot_base64":"b3BhcXVlLXRyYW5zcG9ydC1maXh0dXJl","headers":{"content-type":"text/html"},"rendered_at":"2026-10-18T12:00:00Z"});
        if let Some(reported) = reported {
            response
                .as_object_mut()
                .unwrap()
                .insert("viewport".into(), reported);
        }
        let body = serde_json::to_vec(&response).unwrap();
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(header.as_bytes()).await.unwrap();
        stream.write_all(&body).await.unwrap();
        stream.shutdown().await.unwrap();
        request
    });
    (endpoint, worker)
}
fn config(endpoint: String, viewport: Option<RendererViewport>) -> CrawlConfig {
    CrawlConfig {
        renderer: Some(RendererConfig {
            endpoint,
            viewport,
            screenshot: true,
            wait_ms: 0,
            ..Default::default()
        }),
        ..Default::default()
    }
}
fn page() -> PageRecord {
    PageRecord {
        url: "http://127.0.0.1:1/page".into(),
        final_url: "http://127.0.0.1:1/page".into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn renderer_viewport_real_loopback_post_sends_only_explicit_dimensions_and_keeps_observed_metadata()
 {
    for selected in [
        None,
        Some(RendererViewport {
            width: 320,
            height: 900,
        }),
    ] {
        let reported = match selected {
            Some(viewport) => {
                json!({"width":viewport.width,"height":viewport.height,"worker_extra":"retained"})
            }
            None => json!({"width":1280,"height":800}),
        };
        let (endpoint, worker) = fixture(Some(reported.clone())).await;
        let config = config(endpoint, selected);
        let mut page = page();
        let seeds = [normalize_url(&page.final_url, None).unwrap()];
        render_page(&mut page, &config, &seeds, None, None, None).await;
        let request = tokio::time::timeout(Duration::from_secs(10), worker)
            .await
            .unwrap()
            .unwrap();
        match selected {
            Some(viewport) => {
                assert_eq!(request["viewport"], serde_json::to_value(viewport).unwrap())
            }
            None => assert!(
                request.get("viewport").is_none(),
                "Legacy body must not acquire viewport:null"
            ),
        }
        assert_eq!(request["screenshot"], true);
        assert_eq!(request["url"], "http://127.0.0.1:1/page");
        assert!(page.rendered);
        assert!(page.renderer_error.is_none());
        assert_eq!(page.rendered_viewport, Some(reported));
        assert_eq!(
            page.screenshot_base64.as_deref(),
            Some("b3BhcXVlLXRyYW5zcG9ydC1maXh0dXJl")
        );
    }
}

#[tokio::test]
async fn renderer_viewport_explicit_response_mismatch_is_failure_without_new_screenshot() {
    for reported in [
        None,
        Some(Value::Null),
        Some(json!({"width":1280,"height":800})),
        Some(json!({"width":"320","height":900})),
        Some(json!({"width":320,"height":900.0})),
        Some(json!([320, 900])),
    ] {
        let (endpoint, worker) = fixture(reported).await;
        let config = config(
            endpoint,
            Some(RendererViewport {
                width: 320,
                height: 900,
            }),
        );
        let mut page = page();
        let seeds = [normalize_url(&page.final_url, None).unwrap()];
        render_page(&mut page, &config, &seeds, None, None, None).await;
        let request = tokio::time::timeout(Duration::from_secs(10), worker)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(request["viewport"], json!({"width":320,"height":900}));
        assert!(!page.rendered);
        assert!(page.renderer_error.as_deref().unwrap().contains("viewport"));
        assert!(page.screenshot_base64.is_none());
        assert!(page.rendered_viewport.is_none());
    }
    // Old workers that omit this metadata remain compatible for unconfigured runs.
    let (endpoint, worker) = fixture(None).await;
    let config = config(endpoint, None);
    let mut page = page();
    let seeds = [normalize_url(&page.final_url, None).unwrap()];
    render_page(&mut page, &config, &seeds, None, None, None).await;
    tokio::time::timeout(Duration::from_secs(10), worker)
        .await
        .unwrap()
        .unwrap();
    assert!(page.rendered);
    assert!(page.rendered_viewport.is_none());
    assert!(page.screenshot_base64.is_some());
}
