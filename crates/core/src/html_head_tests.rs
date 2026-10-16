//! Authored observations/regressions; not executed by the proposal author.
use super::*;

fn page() -> PageRecord {
    PageRecord {
        url: "https://example.test/page".into(),
        final_url: "https://example.test/page".into(),
        fetched_at: "2026-10-15T00:00:00Z".into(),
        internal: true,
        status: Some(200),
        final_status: Some(200),
        content_type: "text/html; charset=utf-8".into(),
        ..Default::default()
    }
}
fn captured(html: &str) -> PageRecord {
    let mut p = page();
    p.html_head_observation = Some(observe(&p, html, ContentCaptureDecodeStatus::Exact));
    p
}
fn findings(p: &PageRecord) -> Vec<AuditIssue> {
    let mut out = Vec::new();
    audit(p, &mut out);
    out
}

#[test]
fn raw_head_invalid_child_points_to_source_and_only_following_metadata() {
    let html = "<!doctype html>\n<html><head><title>Earlier</title><img src='x'><meta name='description' content='Later'><link rel='canonical' href='/x'></head><body></body>";
    let p = captured(html);
    let o = p.html_head_observation.as_ref().unwrap();
    assert_eq!(o.status, HtmlHeadStatus::Complete);
    let invalid = o.first_invalid_child.as_ref().unwrap();
    assert_eq!(invalid.start_utf8_byte, html.find("<img").unwrap());
    assert_eq!(
        &html[invalid.start_utf8_byte..invalid.end_utf8_byte],
        "<img src='x'>"
    );
    assert_eq!(invalid.raw_excerpt, "<img src='x'>");
    assert_eq!(
        o.following_metadata
            .iter()
            .map(|s| s.element.as_str())
            .collect::<Vec<_>>(),
        vec!["meta", "link"]
    );
    let issues = findings(&p);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, "html_head_metadata_after_invalid_child");
    assert_eq!(issues[0].severity, Severity::Warning);
    assert!(issues[0].message.contains("decoded UTF-8 bytes"));
    assert!(issues[0].message.contains("not proof of Google processing"));
    assert!(!o.following_metadata.iter().any(|s| s.element == "title"));
}

#[test]
fn raw_head_invalid_without_later_metadata_is_a_distinct_notice() {
    for child in ["<div></div>", "<iframe></iframe>", "<img>", "<input>"] {
        let p = captured(&format!(
            "<head><meta name='robots' content='index'>{child}</head><body>Body</body>"
        ));
        let issues = findings(&p);
        assert_eq!(issues.len(), 1, "{child}");
        assert_eq!(issues[0].code, "html_head_invalid_child");
        assert_eq!(issues[0].severity, Severity::Notice);
        assert!(
            p.html_head_observation
                .as_ref()
                .unwrap()
                .following_metadata
                .is_empty()
        );
    }
}

#[test]
fn raw_head_comments_and_attributes_cannot_forge_children() {
    let html = "<head><!-- <img><meta name='fake'> --><meta name='description' content='a > b <img>'><link href='x?u=%3Cimg%3E'></head><body></body>";
    let p = captured(html);
    let o = p.html_head_observation.as_ref().unwrap();
    assert_eq!(o.status, HtmlHeadStatus::Complete);
    assert!(o.first_invalid_child.is_none());
    assert!(findings(&p).is_empty());
}

#[test]
fn raw_head_script_style_title_and_enabled_noscript_are_real_tokenizer_contexts() {
    let html = r#"<head><title>&lt;img&gt; &amp; title</title><script>const x="</head><img><meta name='fake'>";</script><style>p:before{content:"<img><meta>"}</style><noscript><img><meta name='fake'></noscript><meta name='description' content='Real'></head><body></body>"#;
    let p = captured(html);
    let o = p.html_head_observation.as_ref().unwrap();
    assert_eq!(o.status, HtmlHeadStatus::Complete);
    assert!(o.first_invalid_child.is_none());
    assert!(findings(&p).is_empty());
}

#[test]
fn raw_head_script_double_escape_does_not_create_a_false_head_boundary() {
    let p = captured(
        "<head><script><!--<script>var x = '</script>'; <img><meta>--></script><meta name='description' content='Real'></head><body></body>",
    );
    assert_eq!(
        p.html_head_observation.as_ref().unwrap().status,
        HtmlHeadStatus::Complete
    );
    assert!(findings(&p).is_empty());
}

#[test]
fn raw_head_nested_template_contents_are_inert_and_following_metadata_is_outside() {
    let html = "<head><template><div><img><template><meta name='fake'></template><script>const x='</head><img>';</script></div></template><img><template><meta name='still-inert'></template><meta name='real'></head><body></body>";
    let p = captured(html);
    let o = p.html_head_observation.as_ref().unwrap();
    assert_eq!(o.status, HtmlHeadStatus::Complete);
    assert_eq!(
        o.first_invalid_child.as_ref().unwrap().start_utf8_byte,
        html.find("</template><img>").unwrap() + "</template>".len()
    );
    assert_eq!(
        o.following_metadata
            .iter()
            .map(|s| s.element.as_str())
            .collect::<Vec<_>>(),
        vec!["template", "meta"]
    );
    assert_eq!(
        o.following_metadata.last().unwrap().raw_excerpt,
        "<meta name='real'>"
    );
}

#[test]
fn raw_head_unicode_crlf_and_quoted_greater_than_keep_exact_decoded_byte_ranges() {
    let html = "<head><title>α🦀</title>\r\n  <ImG alt=\"é > <meta fake>\"><META name='r' content='x'></head>";
    let p = captured(html);
    let o = p.html_head_observation.as_ref().unwrap();
    let s = o.first_invalid_child.as_ref().unwrap();
    assert_eq!(o.status, HtmlHeadStatus::Complete);
    assert_eq!(s.line, 2);
    assert_eq!(s.unicode_column, 3);
    assert_eq!(s.start_utf8_byte, html.find("<ImG").unwrap());
    assert_eq!(s.raw_excerpt, "<ImG alt=\"é > <meta fake>\">");
    assert_eq!(&html[s.start_utf8_byte..s.end_utf8_byte], s.raw_excerpt);
    assert_eq!(o.following_metadata[0].element, "meta");
    assert_eq!(
        o.scanned_prefix_sha256,
        format!(
            "{:x}",
            Sha256::digest(&html.as_bytes()[..o.scanned_prefix_utf8_bytes])
        )
    );
}

#[test]
fn raw_head_body_boundary_and_optional_head_end_do_not_scan_the_body() {
    for html in [
        "<head><meta name='x'><body><img><meta name='body'>",
        "<head><title>complete</title>",
        "<head></head><body><img><meta>",
    ] {
        let p = captured(html);
        let o = p.html_head_observation.as_ref().unwrap();
        assert_eq!(o.status, HtmlHeadStatus::Complete, "{html}");
        assert!(o.first_invalid_child.is_none());
        assert!(o.following_metadata.is_empty());
    }
    for html in [
        "<title>implicit head</title><body>",
        "text<head><img><meta></head>",
    ] {
        let p = captured(html);
        assert_eq!(
            p.html_head_observation.as_ref().unwrap().status,
            HtmlHeadStatus::Unknown
        );
        assert!(findings(&p).is_empty());
    }
}

#[test]
fn raw_head_truncated_lossy_failed_and_xhtml_sources_remain_unknown() {
    let html = "<head><img><meta name='robots'></head>";
    for decode in [
        ContentCaptureDecodeStatus::Lossy,
        ContentCaptureDecodeStatus::Unknown,
    ] {
        let o = observe(&page(), html, decode);
        assert_eq!(o.status, HtmlHeadStatus::Unknown);
        assert!(o.first_invalid_child.is_none());
        assert_eq!(o.scanned_prefix_utf8_bytes, 0);
    }
    let mut variants = Vec::new();
    let mut p = page();
    p.truncated = true;
    variants.push(p);
    let mut p = page();
    p.error = Some("read failed".into());
    variants.push(p);
    let mut p = page();
    p.final_status = Some(404);
    variants.push(p);
    let mut p = page();
    p.content_type = "application/xhtml+xml".into();
    variants.push(p);
    for p in variants {
        let o = observe(&p, html, ContentCaptureDecodeStatus::Exact);
        assert_eq!(o.status, HtmlHeadStatus::Unknown);
        assert!(o.first_invalid_child.is_none());
    }
}

#[test]
fn raw_head_budgets_and_unresolved_contexts_never_report_clean() {
    let sources = [
        format!(
            "<head>{}<meta></head>",
            " ".repeat(MAX_HEAD_SCAN_UTF8_BYTES)
        ),
        format!(
            "<head><img alt='{}'><meta></head>",
            "x".repeat(MAX_HEAD_TAG_UTF8_BYTES)
        ),
        format!(
            "<head><img>{}</head>",
            "<meta>".repeat(MAX_HEAD_FOLLOWING + 1)
        ),
        format!("<head>{}</head>", "<meta>".repeat(MAX_HEAD_TOKENS + 1)),
        format!("<head>{}</head>", "<template>".repeat(33)),
        "<head><script>unterminated".into(),
        "<head><!-- unterminated".into(),
        "<head><img alt='unterminated".into(),
        "<head>&#x41;<meta></head>".into(),
        "<head><svg><title>foreign</title></svg><meta></head>".into(),
    ];
    for html in sources {
        let p = captured(&html);
        let o = p.html_head_observation.as_ref().unwrap();
        assert_eq!(o.status, HtmlHeadStatus::Unknown, "{}", o.reason);
        assert!(findings(&p).is_empty());
        assert!(o.scanned_prefix_utf8_bytes <= MAX_HEAD_SCAN_UTF8_BYTES);
    }
}

#[test]
fn raw_head_legacy_absence_and_wrong_source_identity_are_not_new_observations() {
    let legacy = page();
    assert!(findings(&legacy).is_empty());
    let mut json = serde_json::to_value(legacy).unwrap();
    json.as_object_mut()
        .unwrap()
        .remove("html_head_observation");
    let legacy: PageRecord = serde_json::from_value(json).unwrap();
    assert!(legacy.html_head_observation.is_none());
    for field in [
        "source",
        "document_url",
        "fetched_at",
        "engine",
        "parser_profile",
    ] {
        let p = captured("<head><img><meta></head>");
        let mut value = serde_json::to_value(p).unwrap();
        value["html_head_observation"][field] = serde_json::json!("different");
        let p: PageRecord = serde_json::from_value(value).unwrap();
        assert!(findings(&p).is_empty());
    }
}

#[test]
fn raw_head_inconsistent_stored_spans_are_not_promoted_to_diagnostics() {
    for field in ["offset", "prefix", "name", "ordering", "head", "hash"] {
        let mut p = captured("<head><img><meta><link></head>");
        let o = p.html_head_observation.as_mut().unwrap();
        match field {
            "offset" => o.first_invalid_child.as_mut().unwrap().end_utf8_byte = usize::MAX,
            "prefix" => o.scanned_prefix_utf8_bytes = MAX_HEAD_SCAN_UTF8_BYTES + 1,
            "name" => o.first_invalid_child.as_mut().unwrap().element = "meta".into(),
            "ordering" => o.following_metadata.reverse(),
            "head" => o.explicit_head = None,
            "hash" => o.scanned_prefix_sha256 = "not a hash".into(),
            _ => unreachable!(),
        }
        assert!(findings(&p).is_empty(), "{field}");
    }
}

#[test]
fn raw_head_observation_is_not_replaced_by_repaired_or_rendered_metadata() {
    let mut p = captured("<head><img><meta name='description' content='Raw'></head>");
    let saved = p.html_head_observation.clone();
    let seeds = vec![url::Url::parse("https://example.test/").unwrap()];
    crate::extract::extract_html_in_source(
        &mut p,
        "<head><title>DOM</title><meta name='description' content='DOM'></head><body></body>",
        &seeds,
        false,
        &[],
        "rendered_html",
    );
    p.rendered = true;
    p.rendered_url = Some("https://example.test/dom-jump".into());
    p.renderer_error = Some("DOM later failed".into());
    assert_eq!(p.html_head_observation, saved);
    assert_eq!(p.description(), "DOM");
    let issues = crate::audit_pages(&[p]);
    assert!(
        issues
            .iter()
            .any(|i| i.code == "html_head_metadata_after_invalid_child")
    );
}

#[test]
fn raw_head_serialized_page_and_existing_issues_csv_keep_source_evidence() {
    let p = captured("<head><img alt='α'><meta name='description' content='x'></head>");
    let issues = findings(&p);
    let message = issues[0].message.clone();
    let report = crate::CrawlReport {
        config: Default::default(),
        pages: vec![p],
        issues,
        summary: Default::default(),
        warnings: Vec::new(),
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
        sitemap_source_measurements: None,
    };
    let serialized = serde_json::to_value(&report).unwrap();
    assert_eq!(
        serialized["pages"][0]["html_head_observation"]["source"],
        "raw_http_decoded_utf8"
    );
    assert_eq!(serialized["issues"][0]["message"], message);
    let bytes = crate::export_issues_csv(&report).unwrap();
    let mut reader = csv::Reader::from_reader(bytes.as_bytes());
    let rows = reader.records().collect::<csv::Result<Vec<_>>>().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(&rows[0][1], "html_head_metadata_after_invalid_child");
    assert_eq!(&rows[0][3], message);
}

#[test]
fn raw_head_utf8_excerpt_cap_never_splits_a_scalar_or_changes_full_tag_end() {
    let html = format!("<head><img alt='{}'><meta></head>", "🦀".repeat(100));
    let p = captured(&html);
    let o = p.html_head_observation.as_ref().unwrap();
    assert_eq!(o.status, HtmlHeadStatus::Complete);
    let s = o.first_invalid_child.as_ref().unwrap();
    assert!(s.excerpt_truncated);
    assert!(s.raw_excerpt.len() <= MAX_HEAD_EXCERPT_UTF8_BYTES);
    assert!(html.is_char_boundary(s.start_utf8_byte + s.raw_excerpt.len()));
    assert!(s.end_utf8_byte - s.start_utf8_byte > s.raw_excerpt.len());
    assert!(findings(&p)[0].message.contains("excerpt truncated"));
}

#[tokio::test]
async fn raw_head_real_http_crawl_observes_without_capture_and_survives_store_reopen() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) =
            tokio::time::timeout(std::time::Duration::from_secs(10), listener.accept())
                .await
                .unwrap()
                .unwrap();
        let mut request = Vec::new();
        loop {
            let mut bytes = [0u8; 1024];
            let n =
                tokio::time::timeout(std::time::Duration::from_secs(10), socket.read(&mut bytes))
                    .await
                    .unwrap()
                    .unwrap();
            assert!(n > 0);
            request.extend_from_slice(&bytes[..n]);
            assert!(request.len() <= 8192);
            if request.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let body = "<!doctype html><html><head><div></div><meta name='description' content='Observed real HTTP'></head><body><h1>Owned</h1></body></html>";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
        String::from_utf8(request).unwrap()
    });
    let config = crate::CrawlConfig {
        seed_urls: vec![format!("http://{address}/")],
        obey_robots: false,
        crawl_links: false,
        delay_ms: 0,
        concurrency: 1,
        max_pages: Some(1),
        ..Default::default()
    };
    assert!(config.content_capture.is_none());
    assert!(config.renderer.is_none());
    let crawler = crate::Crawler::new(config).unwrap();
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        crawler.run(None, cancel),
    )
    .await;
    let request = tokio::time::timeout(std::time::Duration::from_secs(15), server)
        .await
        .unwrap()
        .unwrap();
    let report = result.unwrap().unwrap();
    assert!(request.starts_with("GET / HTTP/1.1\r\n"));
    assert_eq!(report.pages.len(), 1);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == "html_head_metadata_after_invalid_child")
    );
    let observation = report.pages[0].html_head_observation.clone();
    assert_eq!(
        observation.as_ref().unwrap().status,
        HtmlHeadStatus::Complete
    );
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("crawl.sqlite");
    let store = crate::CrawlStore::open(&database).unwrap();
    store.save_report(&report).unwrap();
    drop(store);
    let store = crate::CrawlStore::open_read_only(&database).unwrap();
    let restored = store.load_report().unwrap().unwrap();
    assert_eq!(restored.pages[0].html_head_observation, observation);
    assert_eq!(
        restored
            .issues
            .iter()
            .filter(|i| i.code == "html_head_metadata_after_invalid_child")
            .count(),
        1
    );
}
