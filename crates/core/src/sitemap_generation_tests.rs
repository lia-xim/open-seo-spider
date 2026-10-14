use super::*;
use crate::{CrawlConfig, CrawlSummary, HreflangRecord, ImageRecord};
use std::io::Read;
fn page(identity: &str) -> PageRecord {
    PageRecord {
        url: identity.into(),
        final_url: identity.into(),
        status: Some(200),
        final_status: Some(200),
        internal: true,
        indexable: true,
        content_type: "text/html; charset=utf-8".into(),
        ..Default::default()
    }
}
fn report(pages: Vec<PageRecord>) -> CrawlReport {
    CrawlReport {
        config: CrawlConfig {
            seed_urls: vec!["https://example.org/".into()],
            ..Default::default()
        },
        summary: CrawlSummary {
            completed: pages.len(),
            ..Default::default()
        },
        pages,
        issues: vec![],
        warnings: vec![],
        operation: None,
        policy_ledger: None,
        resource_ledger: None,
        sitemap_catalog: None,
        sitemap_coverage_inventory: None,
        sitemap_source_measurements: None,
    }
}
fn options() -> SitemapGenerationOptions {
    SitemapGenerationOptions {
        public_base_url: "https://example.org/".into(),
        ..Default::default()
    }
}
fn xml_files(bundle: &SitemapGeneratedBundle) -> Vec<&SitemapGeneratedFile> {
    bundle
        .files
        .iter()
        .filter(|file| file.descriptor.kind == "urlset")
        .collect()
}
fn parse_loc_values(bytes: &[u8]) -> Vec<String> {
    let source = std::str::from_utf8(bytes).unwrap();
    let package = sxd_document::parser::parse(source).unwrap();
    let mut context = sxd_xpath::Context::new();
    context.set_namespace("s", NS);
    let expression = sxd_xpath::Factory::new()
        .build("/s:urlset/s:url/s:loc")
        .unwrap()
        .unwrap();
    match expression
        .evaluate(&context, package.as_document().root())
        .unwrap()
    {
        sxd_xpath::Value::Nodeset(nodes) => nodes
            .document_order()
            .iter()
            .map(|node| node.string_value())
            .collect(),
        _ => panic!("nodeset"),
    }
}
#[test]
fn exact_url_count_boundary_stable_flat_zip_and_hashes() {
    let r = report(
        (0..4)
            .map(|i| page(&format!("https://example.org/p{i}")))
            .collect(),
    );
    let mut o = options();
    o.max_urls_per_file = 3;
    let b = generate_sitemap_bundle(&r, &o).unwrap();
    assert_eq!(
        xml_files(&b)
            .iter()
            .map(|f| f.descriptor.url_count)
            .collect::<Vec<_>>(),
        vec![3, 1]
    );
    assert_eq!(b.manifest.memberships.len(), 4);
    let actual = xml_files(&b)
        .iter()
        .flat_map(|f| parse_loc_values(&f.bytes))
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        b.manifest
            .memberships
            .iter()
            .map(|m| m.url.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        b.zip_bytes,
        generate_sitemap_bundle(&r, &o).unwrap().zip_bytes
    );
    assert_eq!(b.zip_sha256, sha(&b.zip_bytes));
    assert_eq!(
        b.manifest.report_sha256,
        sha(serde_json::to_vec(&r).unwrap())
    );
    let mut archive = zip::ZipArchive::new(Cursor::new(&b.zip_bytes)).unwrap();
    assert_eq!(archive.len(), b.files.len());
    for file in &b.files {
        let mut stored = archive.by_name(&file.descriptor.name).unwrap();
        let mut bytes = Vec::new();
        stored.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, file.bytes);
        assert_eq!(sha(&bytes), file.descriptor.sha256);
        assert!(!file.descriptor.name.contains(['/', '\\']));
    }
    assert!(
        String::from_utf8(
            b.files
                .iter()
                .find(|f| f.descriptor.kind == "sitemapindex")
                .unwrap()
                .bytes
                .clone()
        )
        .unwrap()
        .contains("https://example.org/sitemap-00002.xml")
    );
}
#[test]
fn fifty_thousand_and_one_urls_split_at_official_boundary() {
    let r = report(
        (0..50_001)
            .map(|i| page(&format!("https://example.org/p{i:06}")))
            .collect(),
    );
    let b = generate_sitemap_bundle(&r, &options()).unwrap();
    assert_eq!(
        xml_files(&b)
            .iter()
            .map(|f| f.descriptor.url_count)
            .collect::<Vec<_>>(),
        vec![50_000, 1]
    );
    assert_eq!(b.manifest.memberships.len(), 50_001);
    assert_eq!(
        b.manifest
            .memberships
            .iter()
            .map(|m| m.url.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        50_001
    );
    assert!(
        b.files
            .iter()
            .filter(|f| f.descriptor.kind != "manifest")
            .all(|f| f.bytes.len() <= 52_428_800)
    );
}
#[test]
fn exact_utf8_xml_bytes_split_and_single_entry_plus_one_failure() {
    let r = report(vec![
        page("https://example.org/a?a=1&b=2"),
        page("https://example.org/b"),
    ]);
    let mut o = options();
    o.max_urls_per_file = 1;
    let initial = generate_sitemap_bundle(&r, &o).unwrap();
    let exact = xml_files(&initial)[0].bytes.len();
    o.max_urls_per_file = 50_000;
    o.max_urlset_uncompressed_bytes = exact;
    let b = generate_sitemap_bundle(&r, &o).unwrap();
    assert_eq!(xml_files(&b).len(), 2);
    assert_eq!(xml_files(&b)[0].bytes.len(), exact);
    o.max_urlset_uncompressed_bytes = exact - 1;
    assert_eq!(
        generate_sitemap_bundle(&r, &o).unwrap_err().code,
        SitemapGenerationFailureCode::EntryByteLimit
    );
}
#[test]
fn index_independent_exact_byte_cap_and_underflow_are_explicit() {
    let r = report(vec![
        page("https://example.org/a"),
        page("https://example.org/b"),
    ]);
    let mut o = options();
    o.max_urls_per_file = 1;
    let b = generate_sitemap_bundle(&r, &o).unwrap();
    let size = b
        .files
        .iter()
        .find(|f| f.descriptor.kind == "sitemapindex")
        .unwrap()
        .bytes
        .len();
    o.max_index_uncompressed_bytes = size;
    assert!(generate_sitemap_bundle(&r, &o).is_ok());
    o.max_index_uncompressed_bytes = size - 1;
    assert_eq!(
        generate_sitemap_bundle(&r, &o).unwrap_err().code,
        SitemapGenerationFailureCode::IndexLimit
    );
}
#[test]
fn complete_zip_including_headers_has_exact_total_budget() {
    let r = report(vec![page("https://example.org/a")]);
    let mut o = options();
    for _ in 0..4 {
        let n = generate_sitemap_bundle(&r, &o).unwrap().zip_bytes.len();
        o.max_total_output_bytes = n;
    }
    let b = generate_sitemap_bundle(&r, &o).unwrap();
    assert_eq!(b.zip_bytes.len(), o.max_total_output_bytes);
    o.max_total_output_bytes -= 1;
    assert_eq!(
        generate_sitemap_bundle(&r, &o).unwrap_err().code,
        SitemapGenerationFailureCode::TotalOutputBudget
    );
}
#[test]
fn images_hreflang_and_lastmod_escape_roundtrip_in_independent_dom() {
    let identity = "https://example.org/'quoted'?a=1&b=2";
    let mut p = page(identity);
    p.images = vec![ImageRecord {
        url: "https://example.org/img.png?a=1&b=2".into(),
        ..Default::default()
    }];
    p.hreflang = vec![
        HreflangRecord {
            language: "en".into(),
            url: identity.into(),
        },
        HreflangRecord {
            language: "de-CH".into(),
            url: "https://example.de/p".into(),
        },
    ];
    let mut o = options();
    o.include_images = true;
    o.include_hreflang = true;
    o.lastmod = SitemapGenerationLastmod::Provided {
        values: vec![SitemapProvidedLastmod {
            page_url: identity.into(),
            value: " 2024-02-29T12:34:56.1200+05:30 ".into(),
            reference: "manual content change record #17".into(),
        }],
    };
    let b = generate_sitemap_bundle(&report(vec![p]), &o).unwrap();
    let bytes = &xml_files(&b)[0].bytes;
    assert_eq!(parse_loc_values(bytes), vec![identity]);
    let text = std::str::from_utf8(bytes).unwrap();
    let package = sxd_document::parser::parse(text).unwrap();
    let document = package.as_document();
    let mut context = sxd_xpath::Context::new();
    context.set_namespace("s", NS);
    context.set_namespace("image", "http://www.google.com/schemas/sitemap-image/1.1");
    context.set_namespace("xhtml", "http://www.w3.org/1999/xhtml");
    let eval = |expression: &str| {
        sxd_xpath::Factory::new()
            .build(expression)
            .unwrap()
            .unwrap()
            .evaluate(&context, document.root())
            .unwrap()
    };
    assert_eq!(
        eval("string(/s:urlset/s:url/image:image/image:loc)"),
        sxd_xpath::Value::String("https://example.org/img.png?a=1&b=2".into())
    );
    assert_eq!(
        eval("count(/s:urlset/s:url/xhtml:link)"),
        sxd_xpath::Value::Number(2.0)
    );
    assert_eq!(
        eval("string(/s:urlset/s:url/s:lastmod)"),
        sxd_xpath::Value::String("2024-02-29T12:34:56.1200+05:30".into())
    );
    assert!(text.contains("&amp;"));
    assert!(text.contains("&apos;"));
    assert!(
        b.manifest.memberships[0]
            .hreflang
            .iter()
            .any(|a| a.language == "de-CH" && a.source_ordinals == vec![1])
    );
    assert!(
        matches!(&b.manifest.memberships[0].lastmod.as_ref().unwrap().source,SitemapGeneratedLastmodSource::Provided{reference,..} if reference=="manual content change record #17")
    );
}
#[test]
fn invalid_dates_references_and_provided_duplicate_identity_fail_atomically() {
    let r = report(vec![page("https://example.org/a")]);
    for value in [
        "2023-02-29",
        "2024",
        "2024-01-01T10:00:00+15:00",
        "2024-01-01&#x0a;",
        "2024-01-01\0",
    ] {
        let mut o = options();
        o.lastmod = SitemapGenerationLastmod::Provided {
            values: vec![SitemapProvidedLastmod {
                page_url: r.pages[0].url.clone(),
                value: value.into(),
                reference: "recorded editorial source".into(),
            }],
        };
        assert_eq!(
            generate_sitemap_bundle(&r, &o).unwrap_err().code,
            SitemapGenerationFailureCode::InvalidLastmod
        );
    }
    let mut o = options();
    let item = SitemapProvidedLastmod {
        page_url: r.pages[0].url.clone(),
        value: "2024-02-29".into(),
        reference: "source".into(),
    };
    o.lastmod = SitemapGenerationLastmod::Provided {
        values: vec![item.clone(), item],
    };
    assert!(generate_sitemap_bundle(&r, &o).is_err());
}
#[test]
fn unsafe_base_limits_and_lexical_priority_are_validated_before_generation() {
    let r = report(vec![page("https://example.org/a")]);
    for public in [
        "file:///C:/temp/",
        "https://user@example.org/",
        "https://example.org/sitemaps",
        "https://example.org/?x=1",
        "https://example.org/#f",
        "https://example.org/a/../",
        "https://example.org/%2f/",
    ] {
        let mut o = options();
        o.public_base_url = public.into();
        assert_eq!(
            generate_sitemap_bundle(&r, &o).unwrap_err().code,
            SitemapGenerationFailureCode::InvalidOptions
        );
    }
    let mut o = options();
    o.max_urls_per_file = 50_001;
    assert!(generate_sitemap_bundle(&r, &o).is_err());
    for value in [
        "+0.50000000000000000000000000000000000001",
        ".5",
        "1.",
        "01.000",
        "-0.00",
        " 0.5 ",
    ] {
        let mut o = options();
        o.priority = Some(value.into());
        assert!(generate_sitemap_bundle(&r, &o).is_ok(), "{value}");
    }
    for value in ["1.1", "-0.1", "1e0", "NaN", ".", "0..1"] {
        let mut o = options();
        o.priority = Some(value.into());
        assert!(generate_sitemap_bundle(&r, &o).is_err(), "{value}");
    }
}
#[test]
fn redirects_non200_nonindexable_nondocument_and_path_origin_are_diagnosed_not_admitted() {
    let mut values = vec![page("https://example.org/a/accepted")];
    let mut redirect = page("https://example.org/a/redirect");
    redirect.status = Some(301);
    redirect.final_url = "https://example.org/a/accepted".into();
    values.push(redirect);
    let mut missing = page("https://example.org/a/missing");
    missing.final_status = Some(404);
    values.push(missing);
    let mut noindex = page("https://example.org/a/noindex");
    noindex.indexable = false;
    values.push(noindex);
    let mut css = page("https://example.org/a/style.css");
    css.content_type = "text/css".into();
    values.push(css);
    values.push(page("https://example.net/a/wrong"));
    values.push(page("https://example.org/elsewhere"));
    let mut o = options();
    o.public_base_url = "https://example.org/a/".into();
    let b = generate_sitemap_bundle(&report(values), &o).unwrap();
    assert_eq!(b.manifest.memberships.len(), 1);
    assert_eq!(b.manifest.excluded_pages, 6);
    for reason in [
        SitemapGenerationDiagnosticCode::Redirected,
        SitemapGenerationDiagnosticCode::NotHttp200,
        SitemapGenerationDiagnosticCode::NonIndexable,
        SitemapGenerationDiagnosticCode::NonDocument,
        SitemapGenerationDiagnosticCode::WrongOrigin,
        SitemapGenerationDiagnosticCode::OutsidePublicBasePath,
    ] {
        assert!(b.manifest.diagnostics.iter().any(|d| d.code == reason));
    }
}
#[test]
fn strict_url_character_and_control_boundary_never_normalizes_unsafe_originals() {
    let prefix = "https://example.org/";
    let accepted = format!("{prefix}{}", "a".repeat(2047 - prefix.len()));
    assert!(generate_sitemap_bundle(&report(vec![page(&accepted)]), &options()).is_ok());
    for identity in [
        format!("{prefix}{}", "a".repeat(2048 - prefix.len())),
        "https://example.org/a\nb".into(),
    ] {
        let error =
            generate_sitemap_bundle(&report(vec![page(&identity)]), &options()).unwrap_err();
        assert_eq!(error.code, SitemapGenerationFailureCode::NoEligibleUrls);
        assert_eq!(
            error
                .diagnostic_counts
                .get(&SitemapGenerationDiagnosticCode::InvalidRequestedUrl),
            Some(&1)
        );
    }
}
#[test]
fn duplicate_candidates_extensions_and_conflicting_hreflang_are_explicit() {
    let mut p = page("https://example.org/a");
    p.images = vec![
        ImageRecord {
            url: "https://cdn.example.net/a.png".into(),
            ..Default::default()
        };
        2
    ];
    p.hreflang = vec![
        HreflangRecord {
            language: "en".into(),
            url: p.url.clone(),
        },
        HreflangRecord {
            language: "en".into(),
            url: "https://example.org/conflict".into(),
        },
    ];
    let mut o = options();
    o.include_images = true;
    o.include_hreflang = true;
    let b = generate_sitemap_bundle(&report(vec![p.clone()]), &o).unwrap();
    assert!(b.manifest.memberships[0].images.is_empty());
    assert!(b.manifest.memberships[0].hreflang.is_empty());
    o.allow_external_images = true;
    let b = generate_sitemap_bundle(&report(vec![p]), &o).unwrap();
    assert_eq!(b.manifest.memberships[0].images.len(), 1);
    assert!(
        b.manifest
            .diagnostics
            .iter()
            .any(|d| d.code == SitemapGenerationDiagnosticCode::ImageVerificationNotProven)
    );
    let error = generate_sitemap_bundle(
        &report(vec![
            page("https://example.org/a"),
            page("https://example.org/a"),
        ]),
        &o,
    )
    .unwrap_err();
    assert_eq!(
        error
            .diagnostic_counts
            .get(&SitemapGenerationDiagnosticCode::DuplicateRequestedUrl),
        Some(&2)
    );
}

#[tokio::test]
async fn genuine_crawler_saved_sitemap_lastmod_images_and_hreflang_generate_offline() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::oneshot,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    let site = origin.clone();
    let (stop, mut stopped) = oneshot::channel();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) =
                tokio::select! {_=&mut stopped=>break,v=listener.accept()=>v.unwrap()};
            let mut bytes = vec![0; 4096];
            let n = socket.read(&mut bytes).await.unwrap();
            let head = String::from_utf8_lossy(&bytes[..n]);
            let path = head
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap();
            c.fetch_add(1, Ordering::SeqCst);
            let (body, kind) = if path == "/map.xml" {
                (
                    format!(
                        "<urlset xmlns='{NS}'><url><loc>{site}/start</loc><lastmod>2024-02-29</lastmod></url></urlset>"
                    ),
                    "application/xml",
                )
            } else {
                (
                    format!(
                        "<html><head><title>saved source</title><link rel='alternate' hreflang='en' href='{site}/start'><link rel='alternate' hreflang='de' href='{site}/de'></head><body><img src='{site}/image.png?a=1&amp;b=2' alt='image'>body</body></html>"
                    ),
                    "text/html",
                )
            };
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nLast-Modified: Wed, 01 Jan 2020 00:00:00 GMT\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        }
    });
    let temp = tempfile::tempdir().unwrap();
    let crawler = crate::Crawler::new(CrawlConfig {
        seed_urls: vec![format!("{origin}/start")],
        sitemap_urls: vec![format!("{origin}/map.xml")],
        crawl_links: false,
        obey_robots: false,
        delay_ms: 0,
        ..Default::default()
    })
    .unwrap()
    .with_store(temp.path().join("crawl.sqlite"))
    .unwrap();
    let captured = crawler
        .run(None, Arc::new(std::sync::atomic::AtomicBool::new(false)))
        .await
        .unwrap();
    let before = serde_json::to_vec(&captured).unwrap();
    let mut o = options();
    o.public_base_url = format!("{origin}/");
    o.include_images = true;
    o.include_hreflang = true;
    let no_dates = generate_sitemap_bundle(&captured, &o).unwrap();
    assert!(
        no_dates
            .manifest
            .memberships
            .iter()
            .all(|m| m.lastmod.is_none())
    );
    o.lastmod = SitemapGenerationLastmod::CapturedSitemap;
    let b = generate_sitemap_bundle(&captured, &o).unwrap();
    assert_eq!(b.manifest.memberships.len(), 1);
    assert_eq!(
        b.manifest.memberships[0].lastmod.as_ref().unwrap().value,
        "2024-02-29"
    );
    assert_eq!(b.manifest.memberships[0].images.len(), 1);
    assert_eq!(b.manifest.memberships[0].hreflang.len(), 2);
    assert_eq!(serde_json::to_vec(&captured).unwrap(), before);
    assert_eq!(count.load(Ordering::SeqCst), 2);
    stop.send(()).unwrap();
    task.await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn rendered_failure_non200_and_javascript_navigation_are_not_apparently_indexable_raw200() {
    let mut accepted = page("https://example.org/accepted");
    accepted.rendered = true;
    accepted.rendered_status = Some(200);
    accepted.rendered_url = Some(accepted.url.clone());
    let mut failed = page("https://example.org/render-failed");
    failed.renderer_error = Some("actual renderer error".into());
    let mut missing = page("https://example.org/render-missing");
    missing.rendered = true;
    let mut moved = page("https://example.org/js-redirect");
    moved.rendered = true;
    moved.rendered_status = Some(200);
    moved.rendered_url = Some("https://example.org/destination".into());
    let mut error = page("https://example.org/render-status");
    error.rendered = true;
    error.rendered_status = Some(500);
    error.rendered_url = Some(error.url.clone());
    let b = generate_sitemap_bundle(
        &report(vec![accepted, failed, missing, moved, error]),
        &options(),
    )
    .unwrap();
    assert_eq!(b.manifest.memberships.len(), 1);
    assert_eq!(b.manifest.excluded_pages, 4);
    for code in [
        SitemapGenerationDiagnosticCode::RenderedObservationFailed,
        SitemapGenerationDiagnosticCode::RenderedObservationIncomplete,
        SitemapGenerationDiagnosticCode::RenderedNavigationChanged,
        SitemapGenerationDiagnosticCode::RenderedStatusNot200,
    ] {
        assert!(b.manifest.diagnostics.iter().any(|d| d.code == code));
    }
}

#[tokio::test]
async fn captured_dates_count_all_original_locations_before_date_eligibility() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        sync::oneshot,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let site = origin.clone();
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = requests.clone();
    let (stop, mut stopped) = oneshot::channel();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) =
                tokio::select! {_=&mut stopped=>break,result=listener.accept()=>result.unwrap()};
            let mut bytes = vec![0u8; 4096];
            let n = socket.read(&mut bytes).await.unwrap();
            let head = String::from_utf8_lossy(&bytes[..n]);
            let route = head
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap();
            counter.fetch_add(1, Ordering::SeqCst);
            let (status, kind, extra, body) = match route {
                "/map.xml" => {
                    let mut xml = format!("<urlset xmlns='{NS}'>");
                    for name in [
                        "absent",
                        "invalid",
                        "multi",
                        "same",
                        "incomplete",
                        "unique",
                        "alias-a",
                        "alias-b",
                    ] {
                        xml += &format!(
                            "<url><loc>{site}/{name}</loc><lastmod>2024-02-29</lastmod></url>"
                        );
                    }
                    for (name, fields) in [
                        ("absent", ""),
                        ("invalid", "<lastmod>2026-02-30</lastmod>"),
                        (
                            "multi",
                            "<lastmod>2026-10-14</lastmod><lastmod>2026-10-15</lastmod>",
                        ),
                        ("same", "<lastmod>2024-02-29</lastmod>"),
                    ] {
                        xml += &format!("<url><loc>{site}/{name}</loc>{fields}</url>");
                    }
                    xml += "</urlset>";
                    ("200 OK", "application/xml", String::new(), xml)
                }
                "/broken.xml" => (
                    "200 OK",
                    "application/xml",
                    String::new(),
                    format!(
                        "<urlset xmlns='{NS}'><url><loc>{site}/incomplete</loc><lastmod>2026-10-15</lastmod>"
                    ),
                ),
                "/alias-a" => (
                    "302 Found",
                    "text/html",
                    format!("Location: {site}/alias-b\r\n"),
                    String::new(),
                ),
                _ => (
                    "200 OK",
                    "text/html",
                    String::new(),
                    "<html><head><title>saved original</title></head><body>captured</body></html>"
                        .into(),
                ),
            };
            socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let report = crate::Crawler::new(CrawlConfig {
        seed_urls: vec![format!("{origin}/unique")],
        sitemap_urls: vec![format!("{origin}/map.xml"), format!("{origin}/broken.xml")],
        crawl_links: false,
        obey_robots: false,
        delay_ms: 0,
        ..Default::default()
    })
    .unwrap()
    .with_store(directory.path().join("crawl.sqlite"))
    .unwrap()
    .run(None, Arc::new(AtomicBool::new(false)))
    .await
    .unwrap();
    let before = serde_json::to_vec(&report).unwrap();
    let count = requests.load(Ordering::SeqCst);
    let mut options = options();
    options.public_base_url = format!("{origin}/");
    options.lastmod = SitemapGenerationLastmod::CapturedSitemap;
    let mut diagnostics = Vec::new();
    let projected = lastmods(&report, &options, &mut diagnostics).unwrap();
    for name in ["absent", "invalid", "multi", "same", "incomplete"] {
        let original = format!("{origin}/{name}");
        assert!(!projected.contains_key(&original), "{name}");
        assert_eq!(
            diagnostics
                .iter()
                .filter(|item| item.code
                    == SitemapGenerationDiagnosticCode::CapturedLastmodAmbiguous
                    && item.value.as_deref() == Some(original.as_str()))
                .count(),
            1,
            "{name}"
        );
    }
    for name in ["unique", "alias-a", "alias-b"] {
        assert_eq!(projected[&format!("{origin}/{name}")].value, "2024-02-29");
    }
    let generated = generate_sitemap_bundle(&report, &options).unwrap();
    let unique = generated
        .manifest
        .memberships
        .iter()
        .find(|m| m.url == format!("{origin}/unique"))
        .unwrap();
    assert_eq!(unique.lastmod.as_ref().unwrap().value, "2024-02-29");
    assert!(
        !generated
            .manifest
            .memberships
            .iter()
            .any(|m| m.url == format!("{origin}/alias-a"))
    );
    for name in ["absent", "invalid", "multi", "same", "incomplete"] {
        let membership = generated
            .manifest
            .memberships
            .iter()
            .find(|m| m.url == format!("{origin}/{name}"))
            .unwrap();
        assert!(membership.lastmod.is_none());
    }
    assert_eq!(serde_json::to_vec(&report).unwrap(), before);
    assert_eq!(requests.load(Ordering::SeqCst), count);
    stop.send(()).unwrap();
    task.await.unwrap();
    assert_eq!(requests.load(Ordering::SeqCst), count);
}

#[test]
fn stored_render_fragment_only_keeps_http_identity_and_raw_observations() {
    // Authored stored-observation fixtures; this does not execute a renderer.
    // Actual renderer navigation is a separate native acceptance gate.
    let facts = [
        (
            "https://example.org/page",
            "https://example.org/page#section",
        ),
        (
            "https://example.org/query?a=1&b=2",
            "https://example.org/query?a=1&b=2#section?view=2",
        ),
        ("https://example.org/empty", "https://example.org/empty#"),
        (
            "https://example.org/escaped",
            "https://example.org/escaped#section%20one",
        ),
    ];
    let pages = facts
        .iter()
        .map(|(original, rendered)| {
            let mut observed = page(original);
            observed.rendered = true;
            observed.rendered_status = Some(200);
            observed.rendered_url = Some((*rendered).into());
            observed
        })
        .collect();
    let captured = report(pages);
    let before = serde_json::to_vec(&captured).unwrap();
    let bundle = generate_sitemap_bundle(&captured, &options()).unwrap();
    assert_eq!(bundle.manifest.memberships.len(), facts.len());
    assert_eq!(bundle.manifest.excluded_pages, 0);
    assert_eq!(bundle.manifest.report_sha256, sha(&before));
    for (ordinal, (original, rendered)) in facts.iter().enumerate() {
        assert!(bundle.manifest.memberships.iter().any(|membership|
            membership.url == *original && membership.page_ordinal == ordinal));
        assert_eq!(
            captured.pages[ordinal].rendered_url.as_deref(),
            Some(*rendered)
        );
    }
    let mut xml_urls = xml_files(&bundle)
        .iter()
        .flat_map(|file| parse_loc_values(&file.bytes))
        .collect::<Vec<_>>();
    xml_urls.sort();
    let mut originals = facts
        .iter()
        .map(|(original, _)| original.to_string())
        .collect::<Vec<_>>();
    originals.sort();
    assert_eq!(xml_urls, originals);
    assert!(!bundle.manifest.diagnostics.iter().any(|item| matches!(
        item.code,
        SitemapGenerationDiagnosticCode::RenderedNavigationChanged
            | SitemapGenerationDiagnosticCode::RenderedObservationIncomplete
    )));
    assert_eq!(serde_json::to_vec(&captured).unwrap(), before);
}

#[test]
fn stored_render_navigation_and_unsafe_urls_remain_excluded_with_fragment_fix() {
    // These deliberately authored stored facts are not actual browser observations.
    use SitemapGenerationDiagnosticCode as D;
    let cases = [
        (
            "https://example.org/path",
            Some("https://example.org/elsewhere#section"),
            D::RenderedNavigationChanged,
        ),
        (
            "https://example.org/query?a=1&b=2",
            Some("https://example.org/query?a=1&b=3#section"),
            D::RenderedNavigationChanged,
        ),
        (
            "https://example.org/order?a=1&b=2",
            Some("https://example.org/order?b=2&a=1#section"),
            D::RenderedNavigationChanged,
        ),
        (
            "https://example.org/origin",
            Some("https://other.example/origin#section"),
            D::RenderedNavigationChanged,
        ),
        (
            "https://example.org/scheme",
            Some("http://example.org/scheme#section"),
            D::RenderedNavigationChanged,
        ),
        (
            "https://example.org/port",
            Some("https://example.org:8443/port#section"),
            D::RenderedNavigationChanged,
        ),
        (
            "https://example.org/malformed",
            Some("not a URL"),
            D::RenderedObservationIncomplete,
        ),
        (
            "https://example.org/credentials",
            Some("https://user:fixture@example.org/credentials#section"),
            D::RenderedObservationIncomplete,
        ),
        (
            "https://example.org/control",
            Some("https://example.org/control#section\n"),
            D::RenderedObservationIncomplete,
        ),
        (
            "https://example.org/whitespace",
            Some(" https://example.org/whitespace#section"),
            D::RenderedObservationIncomplete,
        ),
        (
            "https://example.org/backslash",
            Some("https://example.org\\backslash#section"),
            D::RenderedObservationIncomplete,
        ),
        (
            "https://example.org/relative",
            Some("/relative#section"),
            D::RenderedObservationIncomplete,
        ),
        (
            "https://example.org/non-http",
            Some("file:///non-http#section"),
            D::RenderedObservationIncomplete,
        ),
        (
            "https://example.org/missing",
            None,
            D::RenderedObservationIncomplete,
        ),
        (
            "https://example.org/bad-percent",
            Some("https://example.org/bad-percent#section%zz"),
            D::RenderedObservationIncomplete,
        ),
    ];
    let mut pages = vec![page("https://example.org/healthy")];
    for (original, rendered, _) in &cases {
        let mut observed = page(original);
        observed.rendered = true;
        observed.rendered_status = Some(200);
        observed.rendered_url = rendered.map(str::to_owned);
        pages.push(observed);
    }
    let mut status = page("https://example.org/render-status-with-fragment");
    status.rendered = true;
    status.rendered_status = Some(500);
    status.rendered_url = Some(format!("{}#section", status.url));
    pages.push(status);
    let captured = report(pages);
    let before = serde_json::to_vec(&captured).unwrap();
    let bundle = generate_sitemap_bundle(&captured, &options()).unwrap();
    assert_eq!(bundle.manifest.memberships.len(), 1);
    assert_eq!(
        bundle.manifest.memberships[0].url,
        "https://example.org/healthy"
    );
    assert_eq!(bundle.manifest.excluded_pages, cases.len() + 1);
    for (offset, (_, _, code)) in cases.iter().enumerate() {
        assert!(
            bundle
                .manifest
                .diagnostics
                .iter()
                .any(|item| item.page_ordinal == Some(offset + 1) && item.code == *code),
            "case {offset}"
        );
    }
    assert!(
        bundle
            .manifest
            .diagnostics
            .iter()
            .any(|item| item.page_ordinal == Some(cases.len() + 1)
                && item.code == D::RenderedStatusNot200)
    );
    assert_eq!(serde_json::to_vec(&captured).unwrap(), before);
}

#[test]
fn valid_percent_escapes_preserve_page_image_hreflang_and_xml_ampersands() {
    // Authored stored facts; encoded octets are not fetched or decoded.
    let identity = "https://example.org/space%20name/%25/%E2%82%AC?a=%20&literal=%25&utf8=%C3%A4";
    let image = "https://example.org/img%20name/%25/%E2%82%AC.png?x=%25&y=%C3%A4";
    let alternate = "https://example.org/de%20name/%25/%e2%82%ac?x=%c3%a4&literal=%25";
    let mut observed = page(identity);
    observed.images = vec![ImageRecord {
        url: image.into(),
        ..Default::default()
    }];
    observed.hreflang = vec![
        HreflangRecord {
            language: "en".into(),
            url: identity.into(),
        },
        HreflangRecord {
            language: "de".into(),
            url: alternate.into(),
        },
    ];
    let captured = report(vec![observed]);
    let before = serde_json::to_vec(&captured).unwrap();
    let mut o = options();
    o.include_images = true;
    o.include_hreflang = true;
    o.lastmod = SitemapGenerationLastmod::Provided {
        values: vec![SitemapProvidedLastmod {
            page_url: identity.into(),
            value: "2026-10-14".into(),
            reference: "explicit percent-escape control".into(),
        }],
    };
    validate_sitemap_generation_options(&o).unwrap();
    let bundle = generate_sitemap_bundle(&captured, &o).unwrap();
    assert_eq!(bundle.manifest.memberships.len(), 1);
    let membership = &bundle.manifest.memberships[0];
    assert_eq!(membership.url, identity);
    assert_eq!(membership.images, vec![image]);
    assert!(
        membership
            .hreflang
            .iter()
            .any(|entry| entry.url == alternate)
    );
    assert_eq!(membership.lastmod.as_ref().unwrap().value, "2026-10-14");
    let bytes = &xml_files(&bundle)[0].bytes;
    assert_eq!(parse_loc_values(bytes), vec![identity]);
    let source = std::str::from_utf8(bytes).unwrap();
    assert!(source.contains("?a=%20&amp;literal=%25&amp;utf8=%C3%A4"));
    let package = sxd_document::parser::parse(source).unwrap();
    let mut context = sxd_xpath::Context::new();
    context.set_namespace("s", NS);
    context.set_namespace("image", "http://www.google.com/schemas/sitemap-image/1.1");
    context.set_namespace("xhtml", "http://www.w3.org/1999/xhtml");
    let evaluate = |expression: &str| {
        sxd_xpath::Factory::new()
            .build(expression)
            .unwrap()
            .unwrap()
            .evaluate(&context, package.as_document().root())
            .unwrap()
    };
    assert_eq!(
        evaluate("string(/s:urlset/s:url/image:image/image:loc)"),
        sxd_xpath::Value::String(image.into())
    );
    assert_eq!(
        evaluate("string(/s:urlset/s:url/xhtml:link[@hreflang='de']/@href)"),
        sxd_xpath::Value::String(alternate.into())
    );
    assert_eq!(bundle.manifest.report_sha256, sha(&before));
    assert_eq!(serde_json::to_vec(&captured).unwrap(), before);
}

#[test]
fn malformed_percent_option_identities_fail_pure_preflight() {
    // This API takes options only: no report, store, client or file I/O is involved.
    for base in [
        "https://example.org/maps%/",
        "https://example.org/maps%2/",
        "https://example.org/maps%ZZ/",
    ] {
        let mut o = options();
        o.public_base_url = base.into();
        assert_eq!(
            validate_sitemap_generation_options(&o).unwrap_err().code,
            SitemapGenerationFailureCode::InvalidOptions,
            "{base}"
        );
    }
    for identity in [
        "https://example.org/p%",
        "https://example.org/p%2",
        "https://example.org/p%ZZ",
        "https://example.org/p?x=%",
        "https://example.org/p?x=%2",
        "https://example.org/p?x=%ZZ",
    ] {
        let mut o = options();
        o.lastmod = SitemapGenerationLastmod::Provided {
            values: vec![SitemapProvidedLastmod {
                page_url: identity.into(),
                value: "2026-10-14".into(),
                reference: "explicit identity validation control".into(),
            }],
        };
        assert_eq!(
            validate_sitemap_generation_options(&o).unwrap_err().code,
            SitemapGenerationFailureCode::InvalidLastmod,
            "{identity}"
        );
    }
    let mut valid = options();
    valid.public_base_url = "https://example.org/maps%20space/%25/%E2%82%AC/".into();
    valid.lastmod = SitemapGenerationLastmod::Provided {
        values: vec![SitemapProvidedLastmod {
            page_url: format!("{}page%20name?x=%25&utf8=%c3%a4", valid.public_base_url),
            value: "2026-10-14".into(),
            reference: "valid percent-escaped identity control".into(),
        }],
    };
    validate_sitemap_generation_options(&valid).unwrap();
}

#[test]
fn malformed_percent_saved_records_are_omitted_and_diagnosed_without_rewriting() {
    // These are authored stored-observation fixtures, not actual crawl assertions.
    use SitemapGenerationDiagnosticCode as D;
    let invalid = [
        "https://example.org/p%",
        "https://example.org/p%2",
        "https://example.org/p%ZZ",
        "https://example.org/p?x=%",
    ];
    let valid_image = "https://example.org/image%20name%25.png?x=%C3%A4&y=ok";
    let mut healthy = page("https://example.org/healthy");
    healthy.images = invalid
        .iter()
        .map(|identity| ImageRecord {
            url: (*identity).into(),
            ..Default::default()
        })
        .collect();
    healthy.images.push(ImageRecord {
        url: valid_image.into(),
        ..Default::default()
    });
    healthy.hreflang = vec![HreflangRecord {
        language: "en".into(),
        url: healthy.url.clone(),
    }];
    for (identity, language) in invalid.iter().zip(["de", "fr", "es", "it"]) {
        healthy.hreflang.push(HreflangRecord {
            language: language.into(),
            url: (*identity).into(),
        });
    }
    let mut pages = vec![healthy];
    pages.extend(invalid.iter().map(|identity| page(identity)));
    let captured = report(pages);
    let before = serde_json::to_vec(&captured).unwrap();
    let mut o = options();
    o.include_images = true;
    o.include_hreflang = true;
    let bundle = generate_sitemap_bundle(&captured, &o).unwrap();
    assert_eq!(bundle.manifest.memberships.len(), 1);
    assert_eq!(
        bundle.manifest.memberships[0].url,
        "https://example.org/healthy"
    );
    assert_eq!(bundle.manifest.memberships[0].images, vec![valid_image]);
    assert!(bundle.manifest.memberships[0].hreflang.is_empty());
    assert_eq!(bundle.manifest.excluded_pages, invalid.len());
    for (offset, identity) in invalid.iter().enumerate() {
        for (code, ordinal) in [
            (D::InvalidRequestedUrl, offset + 1),
            (D::InvalidImageUrl, 0),
            (D::InvalidHreflangUrl, 0),
        ] {
            assert!(
                bundle
                    .manifest
                    .diagnostics
                    .iter()
                    .any(|item| item.code == code
                        && item.page_ordinal == Some(ordinal)
                        && item.value.as_deref() == Some(*identity)),
                "{code:?}: {identity}"
            );
        }
    }
    for file in xml_files(&bundle) {
        let text = std::str::from_utf8(&file.bytes).unwrap();
        assert!(text.contains("image%20name%25.png?x=%C3%A4&amp;y=ok"));
        assert!(invalid.iter().all(|identity| !text.contains(identity)));
    }
    assert_eq!(bundle.manifest.report_sha256, sha(&before));
    assert_eq!(serde_json::to_vec(&captured).unwrap(), before);
}
