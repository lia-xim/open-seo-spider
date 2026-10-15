use super::*;
use open_seo_spider_core::{
    ContentCaptureConfig, CrawlConfig, CrawlControl, Crawler, NgramHtmlTextChannel as Channel,
    NgramHtmlTextProfile, RendererConfig,
};
fn request(channel: Channel, source_kind: ContentCaptureSourceKind) -> Request {
    Request {
        source_kind,
        html_text_channel: Some(NgramHtmlTextProfile {
            version: 1,
            channel,
        }),
        ..Default::default()
    }
}

// A genuine native HTML fetch and an explicitly labelled local renderer-protocol
// response. No Chromium/provenance claim: the renderer input is a controlled stub.
async fn captured_report_with_config(content_capture: Option<ContentCaptureConfig>) -> CrawlReport {
    use axum::{
        Json, Router,
        routing::{get, post},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let gets = Arc::new(AtomicUsize::new(0));
    let read = gets.clone();
    let html = "<!doctype html><html><head><title>Raw alpha</title><title>Raw alpha</title><meta name=description content='raw delta'><meta name=description content=''></head><body><h1>Raw one</h1><h1></h1><h2>Raw two</h2><h3>third <img alt='never substitute'>heading</h3><h4></h4><h5>fifth</h5><h6>sixth</h6><p>old body old</p></body></html>";
    let site = Router::new().route(
        "/",
        get(move || {
            let gets = read.clone();
            async move {
                gets.fetch_add(1, Ordering::SeqCst);
                ([("content-type", "text/html; charset=utf-8")], html)
            }
        }),
    );
    let renderer=Router::new().route("/render",post(|Json(input):Json<Value>|async move{Json(json!({"html":"<!doctype html><html><head><title>DOM beta</title><meta name=description content='DOM gamma'></head><body><h1>DOM first</h1><h1>DOM second</h1><p>separate rendered body</p></body></html>","final_url":input["url"],"status":200,"headers":{"content-type":"text/html; charset=utf-8"},"rendered_at":"2026-10-15T00:00:00Z"}))}));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let render_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/render", render_listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let (render_stop, render_stopped) = tokio::sync::oneshot::channel::<()>();
    let website = tokio::spawn(async move {
        axum::serve(listener, site)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let renderer = tokio::spawn(async move {
        axum::serve(render_listener, renderer)
            .with_graceful_shutdown(async {
                let _ = render_stopped.await;
            })
            .await
            .unwrap();
    });
    let crawler = Crawler::new(CrawlConfig {
        seed_urls: vec![url],
        content_capture,
        renderer: Some(RendererConfig {
            endpoint,
            wait_ms: 0,
            ..Default::default()
        }),
        crawl_links: false,
        obey_robots: false,
        discover_sitemaps: false,
        request_timeout_secs: 5,
        ..Default::default()
    })
    .unwrap();
    let report = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        crawler.run_controlled(None, CrawlControl::new()),
    )
    .await
    .unwrap()
    .unwrap()
    .report;
    drop(crawler);
    stop.send(()).unwrap();
    render_stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), website)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), renderer)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(gets.load(Ordering::SeqCst), 1);
    assert_eq!(report.pages.len(), 1);
    assert!(report.pages[0].rendered);
    report
}

async fn captured_report() -> CrawlReport {
    captured_report_with_config(Some(ContentCaptureConfig {
        sources: vec![
            ContentCaptureSourceKind::RawHttpHtml,
            ContentCaptureSourceKind::RenderedDomHtml,
        ],
        ..Default::default()
    }))
    .await
}
fn metadata_request(channel: Channel, source_kind: ContentCaptureSourceKind) -> Request {
    Request {
        source_kind,
        retained_metadata_channel: Some(open_seo_spider_core::NgramRetainedMetadataProfile {
            version: 1,
            channel,
        }),
        ..Default::default()
    }
}
#[test]
fn saved_html_channel_none_and_null_keep_old_request_snapshot_and_body_csv_bytes() {
    let report = tests::legacy_report();
    let absent: Request = serde_json::from_str("{}").unwrap();
    let null: Request = serde_json::from_str(r#"{"html_text_channel":null}"#).unwrap();
    assert_eq!(
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&null).unwrap()
    );
    assert!(
        serde_json::to_value(&absent)
            .unwrap()
            .get("html_text_channel")
            .is_none()
    );
    let body = aggregate(&report, &absent).unwrap();
    let context = context(&report, Some("old".into()), Some(7), "declared").unwrap();
    let (bytes, s) = preview(&body, context.clone(), &absent).unwrap();
    let old_hash=bounded_hash(&json!({"source_kind":absent.source_kind,"options":absent.options,"view":absent.view,"n":absent.n,"query":absent.query,"exact_tokens":absent.exact_tokens,"limit":absent.limit}),REQUEST_BYTES).unwrap();
    assert_eq!(s.options_sha256, old_hash);
    assert_eq!(
        s.snapshot_token,
        bounded_hash(&(&context, &old_hash, &s.aggregate_sha256), REQUEST_BYTES).unwrap()
    );
    assert!(
        serde_json::to_value(&s)
            .unwrap()
            .get("html_text_channel")
            .is_none()
    );
    assert_eq!(preview(&body, context.clone(), &null).unwrap().0, bytes);
    let mut a = absent;
    a.view = View::Sources;
    let (_, s) = preview(&body, context.clone(), &a).unwrap();
    a.expected_snapshot_token = Some(s.snapshot_token.clone());
    let (csv, _) = csv(&body, context, &a).unwrap();
    let expected = format!(
        "\"snapshot_token\",\"view\",\"n\",\"tokens_json\",\"occurrences\",\"document_frequency\",\"source_ordinal\",\"requested_url\",\"source_revision\",\"capture_config_sha256\",\"text_sha256\",\"basis\",\"observed_generation\",\"source_state\",\"source_counted\",\"first_positions_json\",\"positions_sampled\",\"website_requests\"\r\n\"{}\",\"sources\",\"\",\"null\",\"\",\"\",\"0\",\"https://example.test/legacy\",\"\",\"\",\"\",\"captured_in_crawl\",\"\",\"missing_or_unavailable\",\"false\",\"null\",\"\",\"0\"\r\n",
        s.snapshot_token
    );
    assert_eq!(csv, expected.as_bytes());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn saved_html_channels_real_raw_capture_and_renderer_stub_metadata_stay_separate_and_element_bounded()
 {
    let report = captured_report().await;
    let before = serde_json::to_vec(&report).unwrap();
    assert_eq!(
        report.pages[0]
            .rendered_headers
            .get("content-type")
            .map(String::as_str),
        Some("text/html; charset=utf-8")
    );
    for kind in [
        ContentCaptureSourceKind::RawHttpHtml,
        ContentCaptureSourceKind::RenderedDomHtml,
    ] {
        let capture = report.pages[0]
            .content_captures
            .as_ref()
            .unwrap()
            .iter()
            .find(|c| c.source_kind == kind)
            .unwrap();
        assert_eq!(
            capture.status,
            open_seo_spider_core::PageContentCaptureStatus::Complete,
            "{kind:?}"
        );
        assert!(capture.source_html_utf8_sha256.is_some());
    }
    for (kind, word, count) in [
        (ContentCaptureSourceKind::RawHttpHtml, "raw", 2),
        (ContentCaptureSourceKind::RenderedDomHtml, "dom", 1),
    ] {
        let r = aggregate(&report, &request(Channel::Title, kind)).unwrap();
        let row = r.rows.iter().find(|r| r.tokens == [word]).unwrap();
        assert_eq!(row.occurrences, count);
        assert_eq!(row.document_frequency, 1);
        assert!(!r.rows.iter().any(|r| r.n == 3));
        assert_eq!(r.sources[0].channel.as_ref().unwrap().source_kind, kind);
    }
    let h1 = aggregate(
        &report,
        &request(Channel::H1, ContentCaptureSourceKind::RawHttpHtml),
    )
    .unwrap();
    assert_eq!(
        h1.sources[0]
            .channel
            .as_ref()
            .unwrap()
            .elements
            .as_ref()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(h1.sources[0].tokens, 2);
    let h3 = aggregate(
        &report,
        &request(Channel::H3, ContentCaptureSourceKind::RawHttpHtml),
    )
    .unwrap();
    assert!(h3.rows.iter().any(|r| r.tokens == ["third", "heading"]));
    assert!(
        !h3.rows
            .iter()
            .any(|r| r.tokens.iter().any(|s| s == "never" || s == "substitute"))
    );
    let description = aggregate(
        &report,
        &request(
            Channel::MetaDescription,
            ContentCaptureSourceKind::RawHttpHtml,
        ),
    )
    .unwrap();
    assert_eq!(
        description.sources[0]
            .channel
            .as_ref()
            .unwrap()
            .elements
            .as_ref()
            .unwrap()
            .len(),
        2
    );
    let empty = aggregate(
        &report,
        &request(Channel::H4, ContentCaptureSourceKind::RawHttpHtml),
    )
    .unwrap();
    assert_eq!(empty.sources[0].state, "complete_empty");
    assert_eq!(
        empty.sources[0]
            .channel
            .as_ref()
            .unwrap()
            .elements
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    let absent = aggregate(
        &report,
        &request(Channel::H6, ContentCaptureSourceKind::RenderedDomHtml),
    )
    .unwrap();
    assert_eq!(absent.sources[0].state, "complete_empty");
    assert!(
        absent.sources[0]
            .channel
            .as_ref()
            .unwrap()
            .elements
            .as_ref()
            .unwrap()
            .is_empty()
    );
    assert_eq!(serde_json::to_vec(&report).unwrap(), before);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn saved_html_channels_legacy_incomplete_wrong_source_and_missing_raw_never_fall_back_or_count_zero()
 {
    let report = captured_report().await;
    let req = request(Channel::H1, ContentCaptureSourceKind::RawHttpHtml);
    for mode in 0..5 {
        let mut altered = report.clone();
        let p = &mut altered.pages[0];
        match mode {
            0 => p.raw_metadata = None,
            1 => p.raw_metadata.as_mut().unwrap().dom_metadata = None,
            2 => {
                p.raw_metadata
                    .as_mut()
                    .unwrap()
                    .dom_metadata
                    .as_mut()
                    .unwrap()
                    .complete = false
            }
            3 => {
                p.raw_metadata
                    .as_mut()
                    .unwrap()
                    .dom_metadata
                    .as_mut()
                    .unwrap()
                    .source = "rendered_html".into()
            }
            _ => p.content_captures = None,
        };
        let r = aggregate(&altered, &req).unwrap();
        assert!(!r.sources[0].counted);
        assert!(r.rows.is_empty());
        assert!(r.sources[0].channel.as_ref().unwrap().elements.is_none());
        assert!(!r.all_supplied_sources_complete);
    }
    let mut tampered = report.clone();
    tampered.pages[0].content_captures.as_mut().unwrap()[0].source_document_url =
        Some("https://wrong.test/".into());
    assert!(aggregate(&tampered, &req).is_err());
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn saved_html_channels_pending_selected_and_changed_metadata_invalidate_the_saved_channel_snapshot()
 {
    let report = captured_report().await;
    let mut req = request(Channel::Title, ContentCaptureSourceKind::RawHttpHtml);
    let r = aggregate(&report, &req).unwrap();
    let c = context(&report, Some("job".into()), Some(7), "declared").unwrap();
    let (_, s) = preview(&r, c, &req).unwrap();
    req.expected_snapshot_token = Some(s.snapshot_token);
    let mut changed = report.clone();
    changed.pages[0]
        .raw_metadata
        .as_mut()
        .unwrap()
        .titles
        .push("changed retained metadata".into());
    let r = aggregate(&changed, &req).unwrap();
    let c = context(&changed, Some("job".into()), Some(7), "declared").unwrap();
    assert!(preview(&r, c, &req).is_err());
    let mut pending = report.clone();
    pending.operation=Some(serde_json::from_value(json!({"engine":open_seo_spider_core::SELECTED_RECRAWL_ENGINE,"coverage":open_seo_spider_core::SELECTED_RECRAWL_COVERAGE,"parent":{"job_id":"parent","checkpoint_revision":1,"source_kind":"stored_json","snapshot_sha256":"declared"},"plan_sha256":"declared","inherited_config_sha256":"declared","selection":[],"retained_count":0,"refreshed_count":0,"pending_count":1,"max_attempts":null,"admitted_this_run":0,"committed_this_run":0,"targets":[{"requested_url":pending.pages[0].url,"parent_ordinal":0,"parent_page_sha256":"declared","state":"pending","admitted_attempts":0,"committed_generation":null}]})).unwrap());
    let r = aggregate(&pending, &req).unwrap();
    assert_eq!(r.sources[0].basis, "pending_selected");
    assert!(!r.sources[0].counted);
    assert!(r.rows.is_empty());
    assert!(r.sources[0].channel.as_ref().unwrap().elements.is_none());
    assert!(
        preview(
            &aggregate(&report, &Request::default()).unwrap(),
            context(&report, None, None, "declared").unwrap(),
            &req
        )
        .is_err()
    );
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn saved_html_channel_snapshot_filters_full_csv_and_cli_reopen_bind_the_explicit_channel() {
    let report = captured_report().await;
    let mut req = request(Channel::Title, ContentCaptureSourceKind::RawHttpHtml);
    req.view = View::Occurrences;
    req.limit = 1;
    let c = context(&report, Some("job".into()), Some(7), "declared").unwrap();
    let r = aggregate(&report, &req).unwrap();
    let (first, s) = preview(&r, c.clone(), &req).unwrap();
    let v: Value = serde_json::from_slice(&first).unwrap();
    assert_eq!(
        v["scope"],
        "all_saved_requested_page_rows_retained_html_elements_v1"
    );
    assert_eq!(v["rows"].as_array().unwrap().len(), 1);
    assert!(v["has_more"].as_bool().unwrap());
    req.expected_snapshot_token = Some(s.snapshot_token);
    let (full, _) = csv(&r, c.clone(), &req).unwrap();
    let text = std::str::from_utf8(&full).unwrap();
    assert_eq!(text.lines().count(), r.rows.len() + 1);
    assert!(text.contains("html_text_channel_json"));
    assert!(text.contains("one_based_channel_token_offset_with_explicit_element_spans_v1"));
    for i in 0..4 {
        let mut changed = req.clone();
        match i {
            0 => {
                changed.html_text_channel = Some(NgramHtmlTextProfile {
                    version: 1,
                    channel: Channel::H1,
                })
            }
            1 => changed.source_kind = ContentCaptureSourceKind::RenderedDomHtml,
            2 => changed.query = "raw".into(),
            _ => changed.limit = 2,
        };
        let current = aggregate(&report, &changed).unwrap();
        assert!(preview(&current, c.clone(), &changed).is_err());
    }
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("saved.json");
    let options = dir.path().join("options.json");
    let out = dir.path().join("result.json");
    let csv_path = dir.path().join("full.csv");
    tokio::fs::write(&input, serde_json::to_vec(&report).unwrap())
        .await
        .unwrap();
    req.expected_snapshot_token = None;
    tokio::fs::write(&options, serde_json::to_vec(&req).unwrap())
        .await
        .unwrap();
    let before = tokio::fs::read(&input).await.unwrap();
    run_cli(
        input.clone(),
        Some(options),
        out.clone(),
        Some(csv_path.clone()),
    )
    .await
    .unwrap();
    let reopened: Value = serde_json::from_slice(&tokio::fs::read(out).await.unwrap()).unwrap();
    assert_eq!(reopened["html_text_channel"]["channel"], "title");
    assert_eq!(reopened["website_requests"], 0);
    let expected = text.replace(
        v["snapshot"]["snapshot_token"].as_str().unwrap(),
        reopened["snapshot"]["snapshot_token"].as_str().unwrap(),
    );
    assert_eq!(
        tokio::fs::read(csv_path).await.unwrap(),
        expected.as_bytes()
    );
    assert_eq!(tokio::fs::read(input).await.unwrap(), before);
}
#[test]
fn saved_html_channel_request_rejects_unknown_versions_fields_and_non_object_profiles() {
    for invalid in [
        json!({"html_text_channel":{"version":0,"channel":"h1"}}),
        json!({"html_text_channel":{"version":1,"channel":"body"}}),
        json!({"html_text_channel":{"version":1,"channel":"title","fallback":true}}),
        json!({"html_text_channel":[1,"title"]}),
    ] {
        assert!(serde_json::from_value::<Request>(invalid).is_err());
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn saved_retained_metadata_real_raw_and_render_stub_without_capture_reopen_with_complete_csv()
{
    let report = captured_report_with_config(None).await;
    assert!(report.pages[0].content_captures.is_none());
    assert!(report.config.content_capture.is_none());
    let before = serde_json::to_vec(&report).unwrap();
    for (kind, word, count) in [
        (ContentCaptureSourceKind::RawHttpHtml, "raw", 2),
        (ContentCaptureSourceKind::RenderedDomHtml, "dom", 1),
    ] {
        let r = aggregate(&report, &metadata_request(Channel::Title, kind)).unwrap();
        let row = r.rows.iter().find(|r| r.tokens == [word]).unwrap();
        assert_eq!((row.occurrences, row.document_frequency), (count, 1));
        assert!(r.sources[0].counted);
        let c = r.sources[0].channel.as_ref().unwrap();
        assert!(c.capture_html_sha256.is_none());
        assert!(c.capture_status.is_none());
        assert_eq!(
            c.metadata_source.as_ref().unwrap().authority_profile,
            "retained_html_metadata_v1"
        );
        assert!(r.sources[0].source_revision.is_none());
        assert_eq!(c.source_kind, kind);
    }
    let empty = aggregate(
        &report,
        &metadata_request(Channel::H4, ContentCaptureSourceKind::RawHttpHtml),
    )
    .unwrap();
    assert_eq!(empty.sources[0].state, "complete_empty");
    assert_eq!(
        empty.sources[0]
            .channel
            .as_ref()
            .unwrap()
            .elements
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    for mode in 0..5 {
        let mut changed = report.clone();
        let p = &mut changed.pages[0];
        match mode {
            0 => p.raw_metadata.as_mut().unwrap().dom_metadata = None,
            1 => {
                p.raw_metadata
                    .as_mut()
                    .unwrap()
                    .dom_metadata
                    .as_mut()
                    .unwrap()
                    .complete = false
            }
            2 => {
                p.raw_metadata
                    .as_mut()
                    .unwrap()
                    .dom_metadata
                    .as_mut()
                    .unwrap()
                    .source = "rendered_html".into()
            }
            3 => p.truncated = true,
            _ => p.raw_metadata = None,
        };
        let r = aggregate(
            &changed,
            &metadata_request(Channel::Title, ContentCaptureSourceKind::RawHttpHtml),
        )
        .unwrap();
        assert!(!r.sources[0].counted);
        assert!(
            r.sources[0]
                .channel
                .as_ref()
                .unwrap()
                .metadata_source
                .is_none()
        );
        assert!(r.rows.is_empty());
    }
    let body = aggregate(&report, &Request::default()).unwrap();
    assert!(!body.sources[0].counted);
    assert!(body.rows.is_empty());
    let mut req = metadata_request(Channel::Title, ContentCaptureSourceKind::RawHttpHtml);
    req.view = View::Sources;
    let context = context(&report, Some("saved".into()), Some(7), "declared").unwrap();
    let (json, s) = preview(&aggregate(&report, &req).unwrap(), context.clone(), &req).unwrap();
    let value: Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(
        value["scope"],
        "all_saved_requested_page_rows_retained_metadata_elements_v1"
    );
    assert_eq!(value["retained_metadata_channel"]["channel"], "title");
    assert!(value.get("html_text_channel").is_none());
    req.expected_snapshot_token = Some(s.snapshot_token.clone());
    let (full, _) = csv(&aggregate(&report, &req).unwrap(), context, &req).unwrap();
    let full_text = std::str::from_utf8(&full).unwrap();
    assert!(full_text.contains("retained_metadata_channel_json"));
    assert!(full_text.contains("retained_html_metadata_v1"));
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("report.json");
    let options = dir.path().join("metadata-options.json");
    let out = dir.path().join("metadata-page.json");
    let csv_path = dir.path().join("metadata-full.csv");
    tokio::fs::write(&input, &before).await.unwrap();
    req.expected_snapshot_token = None;
    tokio::fs::write(&options, serde_json::to_vec(&req).unwrap())
        .await
        .unwrap();
    run_cli(
        input.clone(),
        Some(options),
        out.clone(),
        Some(csv_path.clone()),
    )
    .await
    .unwrap();
    let reopened: Value = serde_json::from_slice(&tokio::fs::read(out).await.unwrap()).unwrap();
    let token = reopened["snapshot"]["snapshot_token"].as_str().unwrap();
    assert_eq!(
        tokio::fs::read(csv_path).await.unwrap(),
        full_text.replace(&s.snapshot_token, token).as_bytes()
    );
    assert_eq!(tokio::fs::read(input).await.unwrap(), before);
    assert_eq!(serde_json::to_vec(&report).unwrap(), before);
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn saved_retained_metadata_body_budget_only_foreign_capture_and_snapshot_never_change_authority()
 {
    let mut config = ContentCaptureConfig {
        sources: vec![
            ContentCaptureSourceKind::RawHttpHtml,
            ContentCaptureSourceKind::RenderedDomHtml,
        ],
        ..Default::default()
    };
    config.limits.max_text_utf8_bytes = 1;
    let report = captured_report_with_config(Some(config)).await;
    let original = serde_json::to_vec(&report).unwrap();
    let req = metadata_request(Channel::Title, ContentCaptureSourceKind::RawHttpHtml);
    let r = aggregate(&report, &req).unwrap();
    assert!(r.sources[0].counted);
    assert_eq!(
        r.rows
            .iter()
            .find(|r| r.tokens == ["raw"])
            .unwrap()
            .occurrences,
        2
    );
    let source = r.sources[0].channel.as_ref().unwrap();
    assert_eq!(
        source.capture_status,
        Some(open_seo_spider_core::PageContentCaptureStatus::Incomplete)
    );
    assert_eq!(
        source.capture_html_sha256.as_deref(),
        report.pages[0].content_captures.as_ref().unwrap()[0]
            .source_html_utf8_sha256
            .as_deref()
    );
    assert_ne!(
        source.metadata_source.as_ref().unwrap().revision,
        source.capture_html_sha256.as_deref().unwrap()
    );
    assert!(!aggregate(&report, &Request::default()).unwrap().sources[0].counted);
    assert!(
        !aggregate(
            &report,
            &request(Channel::Title, ContentCaptureSourceKind::RawHttpHtml)
        )
        .unwrap()
        .sources[0]
            .counted
    );
    let c = context(&report, Some("saved".into()), Some(7), "declared").unwrap();
    let (_, s) = preview(&r, c.clone(), &req).unwrap();
    let mut bound = req.clone();
    bound.expected_snapshot_token = Some(s.snapshot_token);
    for mode in 0..4 {
        let mut changed = bound.clone();
        match mode {
            0 => changed.retained_metadata_channel.as_mut().unwrap().channel = Channel::H1,
            1 => changed.source_kind = ContentCaptureSourceKind::RenderedDomHtml,
            2 => {
                changed.retained_metadata_channel = None;
                changed.html_text_channel = Some(NgramHtmlTextProfile {
                    version: 1,
                    channel: Channel::Title,
                });
            }
            _ => changed.expected_generation = Some(8),
        };
        assert!(preview(&aggregate(&report, &changed).unwrap(), c.clone(), &changed).is_err());
    }
    let mut mixed = req.clone();
    mixed.html_text_channel = Some(NgramHtmlTextProfile {
        version: 1,
        channel: Channel::Title,
    });
    assert!(mixed.validate().is_err());
    let foreign = captured_report().await;
    let mut invalid = report.clone();
    invalid.pages[0].content_captures = foreign.pages[0].content_captures.clone();
    assert!(aggregate(&invalid, &req).is_err());
    let mut pending = report.clone();
    pending.operation=Some(serde_json::from_value(json!({"engine":open_seo_spider_core::SELECTED_RECRAWL_ENGINE,"coverage":open_seo_spider_core::SELECTED_RECRAWL_COVERAGE,"parent":{"job_id":"parent","checkpoint_revision":1,"source_kind":"stored_json","snapshot_sha256":"declared"},"plan_sha256":"declared","inherited_config_sha256":"declared","selection":[],"retained_count":0,"refreshed_count":0,"pending_count":1,"max_attempts":null,"admitted_this_run":0,"committed_this_run":0,"targets":[{"requested_url":pending.pages[0].url,"parent_ordinal":0,"parent_page_sha256":"declared","state":"pending","admitted_attempts":0,"committed_generation":null}]})).unwrap());
    let r = aggregate(&pending, &req).unwrap();
    assert!(!r.sources[0].counted);
    assert_eq!(r.sources[0].state, "pending_selected_metadata");
    assert!(
        r.sources[0]
            .channel
            .as_ref()
            .unwrap()
            .metadata_source
            .is_none()
    );
    assert_eq!(serde_json::to_vec(&report).unwrap(), original);
}
