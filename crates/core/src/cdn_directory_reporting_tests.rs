//! AUTHORED_NOT_RUN. Pure declaration controls plus finite real local HTTP ledgers.
use super::*;
use crate::{CrawlConfig, Crawler};

fn config(rules: &[&str]) -> CdnDirectoryReportingConfig {
    CdnDirectoryReportingConfig {
        version: 1,
        rules: rules.iter().map(|s| (*s).into()).collect(),
    }
}

#[test]
fn cdn_directory_cross_scheme_one_www_exact_port_and_no_subdomain_or_suffix_grant() {
    let c = config(&[
        "HTTPS://WWW.CDN.example/assets/",
        "http://files.example:8443/root/",
    ])
    .compile()
    .unwrap();
    for value in [
        "http://cdn.example/assets/x",
        "https://www.cdn.example/assets/",
        "http://files.example:8443/root/x",
        "https://files.example:8443/root/x",
    ] {
        let o = c.observation(value, false).unwrap();
        assert_eq!(o.normalization, CDN_DIRECTORY_NORMALIZATION);
        assert!(!o.seed_scope_internal && !o.grants_network_or_credentials);
    }
    for value in [
        "https://sub.cdn.example/assets/x",
        "https://www.www.cdn.example/assets/x",
        "https://cdn.example.attacker.test/assets/x",
        "https://cdn.example/assets2/x",
        "https://cdn.example:8443/assets/x",
        "https://files.example/root/x",
        "http://files.example:8444/root/x",
        "https://user@cdn.example/assets/x",
    ] {
        assert!(c.observation(value, false).is_none(), "{value}");
    }
    let reverse = config(&["http://cdn.example/"]).compile().unwrap();
    assert!(
        reverse
            .observation("https://www.cdn.example/anything", false)
            .is_some()
    );
}

#[test]
fn cdn_directory_paths_and_percent_hex_are_literal_and_all_overlaps_keep_declarations() {
    let raw = [
        "https://CDN.example/",
        "http://www.cdn.example/Assets/",
        "https://cdn.example/Assets/%2F/",
    ];
    let c = config(&raw).compile().unwrap();
    let o = c
        .observation("http://cdn.example/Assets/%2F/x?q=/else#fragment", true)
        .unwrap();
    assert_eq!(
        o.matching_rules
            .iter()
            .map(|r| r.rule_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        o.matching_rules
            .iter()
            .map(|r| r.declared_directory.as_str())
            .collect::<Vec<_>>(),
        raw
    );
    assert!(o.seed_scope_internal);
    assert_eq!(
        c.observation("https://cdn.example/assets/%2f/x", false)
            .unwrap()
            .matching_rules
            .len(),
        1
    );
    let literal = config(&["https://cdn.example/a+b.(x)/", "https://cdn.example/%2F/"])
        .compile()
        .unwrap();
    assert!(
        literal
            .observation("https://cdn.example/a+b.(x)/file", false)
            .is_some()
    );
    assert!(
        literal
            .observation("https://cdn.example/aaabZx/file", false)
            .is_none()
    );
    assert!(
        literal
            .observation("https://cdn.example//file", false)
            .is_none()
    );
    assert!(
        literal
            .observation("https://cdn.example/%2f/file", false)
            .is_none()
    );
}

#[test]
fn cdn_directory_strict_shapes_unsupported_normalizations_duplicates_and_exact_budgets() {
    for text in [
        r#"[]"#,
        r#"[1,["https://cdn.example/"]]"#,
        r#"{}"#,
        r#"{"version":1,"rules":[],"fetch":true}"#,
        r#"{"version":1,"version":1,"rules":[]}"#,
    ] {
        assert!(
            serde_json::from_str::<CdnDirectoryReportingConfig>(text).is_err(),
            "{text}"
        );
    }
    for value in [
        "cdn.example/",
        "ftp://cdn.example/",
        "https://cdn.example/file",
        "https://cdn.example/a/../",
        "https://cdn.example/%2e/",
        "https://cdn.example/.%2E/",
        "https://cdn.example/%xz/",
        "https://cdn.example/%/",
        "https://cdn.example:80/",
        "http://cdn.example:443/",
        "https://cdn.example:0/",
        "https://cdn.example:65536/",
        "https://bücher.example/",
        "https://xn--bcher-kva.example/",
        "https://[::1]/",
        "https://127.0.0.1/",
        "https://*.example/",
        "https://u@cdn.example/",
        "https://cdn.example/?",
        "https://cdn.example/#",
        "https://cdn.example\\path/",
        " https://cdn.example/",
        "https://cdn.example/é/",
        "https://cdn.example./",
    ] {
        assert!(config(&[value]).validate().is_err(), "{value}");
    }
    assert!(config(&[]).validate().is_err());
    assert!(
        CdnDirectoryReportingConfig {
            version: 2,
            ..config(&["https://cdn.example/"])
        }
        .validate()
        .is_err()
    );
    assert!(
        config(&[
            "http://CDN.example/assets/",
            "https://www.cdn.example/assets/"
        ])
        .validate()
        .is_err()
    );
    let mut many = CdnDirectoryReportingConfig {
        version: 1,
        rules: (0..32)
            .map(|i| format!("https://cdn.example/d{i}/"))
            .collect(),
    };
    assert!(many.validate().is_ok());
    many.rules.push("https://cdn.example/extra/".into());
    assert!(many.validate().is_err());
    let prefix = "https://cdn.example/";
    let exact = format!("{prefix}{}/", "a".repeat(2048 - prefix.len() - 1));
    assert_eq!(exact.len(), 2048);
    assert!(config(&[&exact]).validate().is_ok());
    assert!(config(&[&format!("{exact}/")]).validate().is_err());
    // The separate old exact-origin contract still supports explicit defaults/IDNA.
    assert!(
        crate::CdnClassificationConfig {
            version: 1,
            origins: vec!["https://bücher.example:443/".into()]
        }
        .validate()
        .is_ok()
    );
}

#[test]
fn cdn_directory_optional_json_and_legacy_reports_never_gain_observations_on_load() {
    let absent: CrawlConfig = serde_json::from_str("{}").unwrap();
    let null: CrawlConfig = serde_json::from_str(r#"{"cdn_directory_reporting":null}"#).unwrap();
    assert_eq!(
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&null).unwrap()
    );
    assert!(
        serde_json::to_value(absent)
            .unwrap()
            .get("cdn_directory_reporting")
            .is_none()
    );
    let page: PageRecord =
        serde_json::from_str(r#"{"url":"https://cdn.example/assets/x","internal":false}"#).unwrap();
    assert!(!page.internal && page.cdn_directory_reporting.is_none());
    assert!(
        serde_json::to_value(page)
            .unwrap()
            .get("cdn_directory_reporting")
            .is_none()
    );
    assert!(
        serde_json::to_value(LinkRecord::default())
            .unwrap()
            .get("cdn_directory_reporting")
            .is_none()
    );
}

#[tokio::test]
async fn cdn_directory_composition_preserves_original_seed_fact_and_never_grants_scope_or_resources()
 {
    let mut page = PageRecord {
        url: "https://cdn.example/assets/doc".into(),
        final_url: "https://cdn.example/assets/doc".into(),
        links: vec![LinkRecord {
            url: "https://cdn.example/assets/child".into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    crate::CdnClassificationConfig {
        version: 1,
        origins: vec!["https://cdn.example".into()],
    }
    .compile()
    .unwrap()
    .apply(&mut page);
    let c = config(&["http://www.cdn.example/assets/"])
        .compile()
        .unwrap();
    c.apply(&mut page);
    assert!(page.internal && page.links[0].internal);
    assert!(
        !page
            .cdn_directory_reporting
            .as_ref()
            .unwrap()
            .seed_scope_internal
    );
    assert!(
        !page.links[0]
            .cdn_directory_reporting
            .as_ref()
            .unwrap()
            .seed_scope_internal
    );
    let seeds = vec![Url::parse("https://www.example/").unwrap()];
    assert!(!crate::cdn_classification::navigation_source_internal(
        &page, &seeds, false, true
    ));
    assert!(!crate::cdn_classification::navigation_target_internal(
        &page.links[0],
        &seeds,
        false,
        true
    ));
    let target = Url::parse("https://cdn.example/assets/a.js").unwrap();
    assert!(
        !crate::ResourceDiscoveryConfig::default().permits_resource_origin(&target, &seeds, false)
    );
    assert!(!crate::urls::same_scope(&target, &seeds, false));
    let mut seed = PageRecord {
        url: "https://www.example/".into(),
        final_url: "https://cdn.example/assets/doc".into(),
        internal: true,
        ..Default::default()
    };
    c.apply(&mut seed);
    assert!(seed.cdn_directory_reporting.is_none()); // Requested URL, never redirect target.
    assert!(!crate::cdn_classification::navigation_source_internal(
        &seed, &seeds, false, true
    ));

    // Genuine production resource-discovery comparison; the assertions above
    // remain the original pure controls, not substitutes for these actual IO rows.
    let png = resource_fixture_png();
    let listener_a = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let listener_b = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_a = format!("http://{}", listener_a.local_addr().unwrap());
    let port_b = listener_b.local_addr().unwrap().port();
    let origin_b = format!("http://localhost:{port_b}");
    let denied_origin = format!("http://www.localhost:{port_b}");
    let root_html = format!(
        "<html><head><link rel='stylesheet' href='{origin_b}/assets/style.css'></head><body><img src='{origin_b}/assets/direct.png'><img src='{origin_b}/assets/html-image'><img src='{origin_b}/assets/image-redirect'><img src='{denied_origin}/assets/not-granted.png'><a href='/nav-redirect'>Internal redirect</a><a href='{origin_b}/assets/external-doc'>External navigation disabled</a></body></html>"
    );
    let routes_a = std::collections::BTreeMap::from([
        (
            "/".into(),
            ResourceFixtureResponse::new("text/html; charset=utf-8", root_html.into_bytes()),
        ),
        (
            "/nav-redirect".into(),
            ResourceFixtureResponse::redirect(format!("{origin_b}/assets/no-nav")),
        ),
    ]);
    let routes_b = std::collections::BTreeMap::from([
        ("/assets/style.css".into(), ResourceFixtureResponse::new("text/css; charset=utf-8", b"@import '/assets/nested.css';body{background:url('/assets/css.png')}".to_vec())),
        ("/assets/nested.css".into(), ResourceFixtureResponse::new("text/css; charset=utf-8", b"body{color:black}".to_vec())),
        ("/assets/css.png".into(), ResourceFixtureResponse::new("image/png", png.clone())),
        ("/assets/direct.png".into(), ResourceFixtureResponse::new("image/png", png)),
        ("/assets/html-image".into(), ResourceFixtureResponse::new("text/html; charset=utf-8", b"<html><body><a href='/assets/child'>Must not expand</a><img src='/assets/from-image.png'></body></html>".to_vec())),
        ("/assets/image-redirect".into(), ResourceFixtureResponse::redirect(format!("{denied_origin}/assets/denied-img"))),
    ]);
    let expected_a = routes_a.clone();
    let expected_b = routes_b.clone();
    let mut resource = crate::ResourceDiscoveryConfig {
        contract: Some(crate::ResourceContractConfig::default()),
        ..Default::default()
    };
    resource
        .resource_scope
        .external_origins
        .push(origin_b.clone());
    let mut base = CrawlConfig {
        seed_urls: vec![format!("{origin_a}/")],
        resource_discovery: Some(resource.clone()),
        discover_sitemaps: false,
        obey_robots: false,
        concurrency: 1,
        delay_ms: 0,
        request_timeout_secs: 5,
        max_pages: Some(32),
        ..Default::default()
    };
    base.custom_headers.insert(
        "Authorization".into(),
        "Bearer owned-cdn-resource-fixture".into(),
    );
    base.custom_headers
        .insert("Cookie".into(), "owned_cdn_resource=local".into());
    base.custom_headers
        .insert("X-Own-Secret".into(), "owned-local-header".into());
    let directory = config(&[&format!("{origin_b}/assets/")]);
    assert!(
        directory
            .compile()
            .unwrap()
            .observation(&format!("{denied_origin}/assets/not-granted.png"), false)
            .is_some()
    );
    let temporary = tempfile::tempdir().unwrap();
    let fixture_a = ResourceFixture::start(listener_a, routes_a);
    let fixture_b = ResourceFixture::start(listener_b, routes_b);
    use futures::FutureExt;
    let phases = std::panic::AssertUnwindSafe(async {
        let mut phases = Vec::new();
        for (index, enabled) in [false, true].into_iter().enumerate() {
            let mut cfg = base.clone();
            cfg.cdn_directory_reporting = enabled.then(|| directory.clone());
            let database = temporary.path().join(format!("resource-{index}.sqlite"));
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                Crawler::new(cfg)?
                    .with_store(&database)?
                    .run_controlled(None, crate::CrawlControl::new()),
            )
            .await
            .map_err(|_| anyhow::anyhow!("resource crawl deadline"))??;
            let store = crate::CrawlStore::open_read_only(&database)?;
            let mut occurrences = Vec::new();
            store.visit_resource_occurrences(32, |page| {
                occurrences.extend(page.rows.clone());
                Ok(())
            })?;
            let mut transports = Vec::new();
            let mut after = None;
            loop {
                let page = store.resource_transports(after.as_deref(), 32)?;
                transports.extend(page.rows);
                anyhow::ensure!(
                    transports.len() <= 64,
                    "unexpected resource transport expansion"
                );
                if !page.has_more {
                    break;
                }
                let next = page
                    .next_url
                    .ok_or_else(|| anyhow::anyhow!("missing transport cursor"))?;
                anyhow::ensure!(after.as_ref() != Some(&next), "unchanged transport cursor");
                after = Some(next);
            }
            let cold = store
                .load_report()?
                .ok_or_else(|| anyhow::anyhow!("missing stored report"))?;
            let checkpoint = crate::CrawlStore::inspect(&database)?.checkpoint_valid;
            let log_a = fixture_a.snapshot().await?;
            let log_b = fixture_b.snapshot().await?;
            phases.push((
                result,
                occurrences,
                transports,
                cold,
                checkpoint,
                log_a,
                log_b,
            ));
        }
        Ok::<_, anyhow::Error>(phases)
    })
    .catch_unwind()
    .await;
    // Always request both owned stops and retain both actual joins before any
    // functional assertion/unwrap of the crawl observations. Abort is failure-only.
    let (closed_a, closed_b) = tokio::join!(fixture_a.stop(), fixture_b.stop());
    let closed_a = closed_a.unwrap();
    let closed_b = closed_b.unwrap();
    let phases = match phases {
        Ok(result) => result.unwrap(),
        Err(panic) => std::panic::resume_unwind(panic),
    };
    assert_eq!(phases.len(), 2);
    assert_eq!(closed_a, phases[1].5);
    assert_eq!(closed_b, phases[1].6);
    let mut comparable = Vec::new();
    for (index, (result, rows, transports, cold, checkpoint, all_a, all_b)) in
        phases.iter().enumerate()
    {
        let enabled = index == 1;
        assert_eq!(result.outcome, crate::CrawlOutcome::Completed);
        assert!(*checkpoint);
        assert_eq!(result.report.summary.pending, 0);
        assert_eq!(
            serde_json::to_value(&cold.config).unwrap(),
            serde_json::to_value(&result.report.config).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&cold.pages).unwrap(),
            serde_json::to_value(&result.report.pages).unwrap()
        );
        let ledger = result.report.resource_ledger.as_ref().unwrap();
        assert_eq!(ledger.completed_transports, 8);
        assert_eq!(ledger.pending_transports, 0);
        assert_eq!(ledger.pending_expansions, 0);
        // The two deliberately refused redirects have incomplete response sources.
        assert_eq!(ledger.incomplete_sources, 2);
        assert!(!ledger.complete);
        assert_eq!(transports.len(), 8);
        let start_a = if enabled { phases[0].5.len() } else { 0 };
        let start_b = if enabled { phases[0].6.len() } else { 0 };
        let mut log_a = all_a[start_a..].to_vec();
        let mut log_b = all_b[start_b..].to_vec();
        log_a.sort_by(|a, b| a.path.cmp(&b.path));
        log_b.sort_by(|a, b| a.path.cmp(&b.path));
        assert_eq!(
            log_a.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
            vec!["/", "/nav-redirect"]
        );
        assert_eq!(
            log_b.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
            vec![
                "/assets/css.png",
                "/assets/direct.png",
                "/assets/html-image",
                "/assets/image-redirect",
                "/assets/nested.css",
                "/assets/style.css"
            ]
        );
        assert!(
            log_a
                .iter()
                .chain(&log_b)
                .all(|r| r.method == "GET" && r.client_eof)
        );
        assert!(
            log_a
                .iter()
                .all(|r| r.authorization && r.cookie && r.own_secret)
        );
        assert!(
            log_b
                .iter()
                .all(|r| !r.authorization && !r.cookie && !r.own_secret)
        );
        for (origin, routes, log) in [
            (&origin_a, &expected_a, &log_a),
            (&origin_b, &expected_b, &log_b),
        ] {
            for request in log {
                let expected = routes.get(&request.path).unwrap();
                assert_eq!(request.status, expected.status);
                assert_eq!(request.body_bytes, expected.body.len());
                let url = format!("{origin}{}", request.path);
                let transport = transports.iter().find(|r| r.requested_url == url).unwrap();
                assert!(transport.retained_response);
                if expected.status == 200 {
                    assert_eq!(transport.projection.final_status, Some(200));
                    let facts = transport.projection.resource_response.as_ref().unwrap();
                    assert!(facts.captured_entity_complete);
                    assert_eq!(facts.captured_entity_bytes, expected.body.len());
                    assert_eq!(
                        facts.captured_entity_sha256.as_deref(),
                        Some(format!("{:x}", Sha256::digest(&expected.body))).as_deref()
                    );
                }
            }
        }
        for path in [
            "/assets/style.css",
            "/assets/nested.css",
            "/assets/css.png",
            "/assets/direct.png",
            "/assets/html-image",
            "/assets/image-redirect",
        ] {
            let url = format!("{origin_b}{path}");
            let page = result.report.pages.iter().find(|p| p.url == url).unwrap();
            assert_eq!(page.internal, enabled);
            assert_eq!(page.cdn_directory_reporting.is_some(), enabled);
            if let Some(o) = &page.cdn_directory_reporting {
                assert!(!o.seed_scope_internal && !o.grants_network_or_credentials);
            }
            let p = page.resource_permission.as_ref().unwrap();
            assert!(!p.can_expand_navigation && !p.can_expand_resources);
            assert_eq!(p.can_expand_css, path.ends_with(".css"));
        }
        for path in [
            "/assets/no-nav",
            "/assets/external-doc",
            "/assets/child",
            "/assets/from-image.png",
        ] {
            let url = format!("{origin_b}{path}");
            assert!(!transports.iter().any(|r| r.requested_url == url));
        }
        for path in ["/assets/not-granted.png", "/assets/denied-img"] {
            let url = format!("{denied_origin}{path}");
            assert!(!transports.iter().any(|r| r.requested_url == url));
        }
        let denied = rows
            .iter()
            .find(|r| {
                r.occurrence.resolved_url.as_deref()
                    == Some(format!("{denied_origin}/assets/not-granted.png").as_str())
            })
            .unwrap();
        assert_eq!(
            denied.occurrence.reference_kind,
            crate::ResourceReferenceKind::Image
        );
        assert_eq!(denied.decision.reason, "scope_denied");
        assert!(denied.decision.grant.is_none());
        let external = rows
            .iter()
            .find(|r| {
                r.occurrence.resolved_url.as_deref()
                    == Some(format!("{origin_b}/assets/external-doc").as_str())
            })
            .unwrap();
        assert_eq!(
            external.occurrence.reference_kind,
            crate::ResourceReferenceKind::ExternalAnchor
        );
        assert_eq!(external.decision.reason, "fetch_disabled");
        assert!(external.decision.grant.is_none());
        for path in ["/assets/child", "/assets/from-image.png"] {
            let row = rows
                .iter()
                .find(|r| {
                    r.occurrence.requested_source_url == format!("{origin_b}/assets/html-image")
                        && r.occurrence.raw_value == path
                })
                .unwrap();
            assert_eq!(row.decision.reason, "source_check_only");
            assert!(row.decision.grant.is_none());
        }
        for (url, target, kind) in [
            (
                format!("{origin_a}/nav-redirect"),
                format!("{origin_b}/assets/no-nav"),
                crate::ResourceReferenceKind::InternalAnchor,
            ),
            (
                format!("{origin_b}/assets/image-redirect"),
                format!("{denied_origin}/assets/denied-img"),
                crate::ResourceReferenceKind::Image,
            ),
        ] {
            let page = &transports
                .iter()
                .find(|r| r.requested_url == url)
                .unwrap()
                .projection;
            assert_eq!(page.status, Some(302));
            assert_eq!(page.final_status, None);
            let decisions = &page.resource_response.as_ref().unwrap().redirect_decisions;
            assert_eq!(decisions.len(), 1);
            let decision = &decisions[0];
            assert_eq!(decision.target_url.as_deref(), Some(target.as_str()));
            assert_eq!(decision.original_reference_kinds, vec![kind]);
            assert_eq!(decision.reason, "redirect_role_scope_denied");
            assert!(!decision.scope_allowed && !decision.allowed && !decision.request_started);
            assert!(decision.authorizing_occurrence_ids.is_empty());
        }
        let mut decisions = rows.iter().map(|row| {
            let grant = row.decision.grant.as_ref().map(|g| serde_json::json!({"url":g.fetch_url,"role":g.reference_kind,"store":g.store,"nav":g.can_expand_navigation,"resources":g.can_expand_resources,"css":g.can_expand_css,"pdf":g.can_expand_pdf,"config":g.config_sha256}));
            serde_json::json!({"source":row.occurrence.requested_source_url,"raw":row.occurrence.raw_value,"url":row.occurrence.resolved_url,"role":row.occurrence.reference_kind,"reason":row.decision.reason,"qualification":row.decision.qualification,"grant":grant})
        }).collect::<Vec<_>>();
        decisions.sort_by_cached_key(|value| value.to_string());
        let mut cfg = serde_json::to_value(&result.report.config).unwrap();
        cfg.as_object_mut()
            .unwrap()
            .remove("cdn_directory_reporting");
        comparable.push((log_a, log_b, decisions, cfg));
    }
    // Timestamp-derived source revisions/occurrence IDs are not interchangeable
    // observations. Compare all stable decisions/roles/permissions and actual IO.
    assert_eq!(comparable[0], comparable[1]);
}

async fn site(
    listener: tokio::net::TcpListener,
    body: String,
    redirect: Option<String>,
    forbid_secret: bool,
) -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<Vec<String>>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (stop, mut stopped) = tokio::sync::oneshot::channel();
    let owner = tokio::spawn(async move {
        let mut paths = Vec::new();
        loop {
            let accepted = tokio::select! { _ = &mut stopped => break, accepted = listener.accept() => accepted };
            let (mut socket, _) = accepted.unwrap();
            let mut request = Vec::new();
            loop {
                let mut buf = [0u8; 1024];
                let n =
                    tokio::time::timeout(std::time::Duration::from_secs(5), socket.read(&mut buf))
                        .await
                        .unwrap()
                        .unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buf[..n]);
                assert!(request.len() <= 16384);
                if request.windows(4).any(|v| v == b"\r\n\r\n") {
                    break;
                }
            }
            let text = String::from_utf8(request).unwrap();
            if forbid_secret {
                assert!(!text.to_ascii_lowercase().contains("x-owned-secret:"));
            }
            paths.push(
                text.lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_owned(),
            );
            let response = if let Some(target) = &redirect {
                format!(
                    "HTTP/1.1 302 Found\r\nLocation: {target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
            } else {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
            };
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        }
        paths
    });
    (stop, owner)
}

#[tokio::test]
async fn cdn_directory_real_external_check_never_expands_html_or_sends_seed_headers_and_cold_is_exact()
 {
    let a = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let b = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let primary = format!("http://{}/", a.local_addr().unwrap());
    let cdn = format!("http://localhost:{}", b.local_addr().unwrap().port());
    let (stop_a, owner_a) = site(
        a,
        format!("<a href='{cdn}/assets/doc'>CDN</a>"),
        None,
        false,
    )
    .await;
    let (stop_b, owner_b) = site(b, "<a href='/assets/child'>Must not expand</a><a href='http://localhost:1/foreign'>Must not visit</a>".into(), None, true).await;
    let mut cfg = CrawlConfig {
        seed_urls: vec![primary],
        cdn_directory_reporting: Some(config(&[&format!("{cdn}/assets/")])),
        discover_sitemaps: false,
        obey_robots: false,
        delay_ms: 0,
        concurrency: 1,
        max_pages: Some(10),
        ..Default::default()
    };
    cfg.custom_headers
        .insert("x-owned-secret".into(), "owned-local-test".into());
    let report = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        Crawler::new(cfg.clone()).unwrap().run(
            None,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(report.pages.len(), 1);
    assert!(report.pages[0].links[0].cdn_directory_reporting.is_some());
    cfg.check_external = true;
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("crawl.sqlite");
    let report = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        Crawler::new(cfg.clone())
            .unwrap()
            .with_store(&db)
            .unwrap()
            .run(
                None,
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(report.pages.len(), 2);
    let page = report
        .pages
        .iter()
        .find(|p| p.url == format!("{cdn}/assets/doc"))
        .unwrap();
    assert!(
        page.internal
            && !page
                .cdn_directory_reporting
                .as_ref()
                .unwrap()
                .seed_scope_internal
    );
    stop_a.send(()).unwrap();
    stop_b.send(()).unwrap();
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(5), owner_a)
            .await
            .unwrap()
            .unwrap(),
        vec!["/", "/"]
    );
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(5), owner_b)
            .await
            .unwrap()
            .unwrap(),
        vec!["/assets/doc"]
    );
    let restored = crate::CrawlStore::open_read_only(&db)
        .unwrap()
        .load_report()
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&restored.config).unwrap(),
        serde_json::to_vec(&report.config).unwrap()
    );
    assert_eq!(
        restored.config.cdn_directory_reporting,
        cfg.cdn_directory_reporting
    );
    assert_eq!(
        serde_json::to_value(restored.pages).unwrap(),
        serde_json::to_value(report.pages).unwrap()
    );
}

#[tokio::test]
async fn cdn_directory_real_external_redirect_and_invalid_config_have_no_target_get() {
    let a = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let b = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let primary = format!("http://{}/", a.local_addr().unwrap());
    let cdn = format!("http://localhost:{}", b.local_addr().unwrap().port());
    let (stop_a, owner_a) = site(
        a,
        String::new(),
        Some(format!("{cdn}/assets/blocked")),
        false,
    )
    .await;
    let (stop_b, owner_b) = site(b, "Forbidden target".into(), None, true).await;
    let cfg = CrawlConfig {
        seed_urls: vec![primary],
        cdn_directory_reporting: Some(config(&[&format!("{cdn}/assets/")])),
        discover_sitemaps: false,
        obey_robots: false,
        delay_ms: 0,
        request_timeout_secs: 3,
        ..Default::default()
    };
    let invalid = CrawlConfig {
        cdn_directory_reporting: Some(config(&["https://cdn.example:443/"])),
        ..cfg.clone()
    };
    assert!(Crawler::new(invalid).is_err());
    let report = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        Crawler::new(cfg).unwrap().run(
            None,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(report.pages.len(), 1);
    let p = &report.pages[0];
    assert_eq!(p.status, Some(302));
    assert_eq!(p.final_status, None);
    assert_eq!(p.redirect_chain[0].target, format!("{cdn}/assets/blocked"));
    assert!(p.cdn_directory_reporting.is_none());
    assert!(p.warnings.iter().any(|w| w.contains("outside crawl scope")));
    stop_a.send(()).unwrap();
    stop_b.send(()).unwrap();
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(5), owner_a)
            .await
            .unwrap()
            .unwrap(),
        vec!["/"]
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(5), owner_b)
            .await
            .unwrap()
            .unwrap()
            .is_empty()
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResourceFixtureRequest {
    method: String,
    path: String,
    authorization: bool,
    cookie: bool,
    own_secret: bool,
    status: u16,
    body_bytes: usize,
    client_eof: bool,
}
#[derive(Clone)]
struct ResourceFixtureResponse {
    status: u16,
    mime: &'static str,
    body: Vec<u8>,
    location: Option<String>,
}
impl ResourceFixtureResponse {
    fn new(mime: &'static str, body: Vec<u8>) -> Self {
        Self {
            status: 200,
            mime,
            body,
            location: None,
        }
    }
    fn redirect(target: String) -> Self {
        Self {
            status: 302,
            mime: "text/plain",
            body: Vec::new(),
            location: Some(target),
        }
    }
}
enum ResourceFixtureCommand {
    Snapshot(tokio::sync::oneshot::Sender<Vec<ResourceFixtureRequest>>),
    Stop,
}
struct ResourceFixture {
    commands: tokio::sync::mpsc::Sender<ResourceFixtureCommand>,
    owner: tokio::task::JoinHandle<anyhow::Result<Vec<ResourceFixtureRequest>>>,
}
impl ResourceFixture {
    fn start(
        listener: tokio::net::TcpListener,
        routes: std::collections::BTreeMap<String, ResourceFixtureResponse>,
    ) -> Self {
        let (commands, mut receiver) = tokio::sync::mpsc::channel(4);
        let owner = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut log = Vec::new();
            loop {
                let accepted = tokio::select! {
                    message = receiver.recv() => match message {
                        Some(ResourceFixtureCommand::Snapshot(reply)) => { let _ = reply.send(log.clone()); continue; }
                        Some(ResourceFixtureCommand::Stop) | None => break,
                    },
                    accepted = listener.accept() => accepted,
                };
                let (mut socket, _) = accepted?;
                let row = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                    let mut bytes = Vec::new();
                    loop {
                        let mut buffer = [0u8; 1024];
                        let count = socket.read(&mut buffer).await?;
                        anyhow::ensure!(count > 0, "request ended before headers");
                        bytes.extend_from_slice(&buffer[..count]);
                        anyhow::ensure!(bytes.len() <= 16384, "fixture request header cap");
                        if bytes.windows(4).any(|v| v == b"\r\n\r\n") { break; }
                    }
                    let text = String::from_utf8(bytes)?;
                    let first = text.lines().next().ok_or_else(|| anyhow::anyhow!("missing request line"))?;
                    let mut words = first.split_ascii_whitespace();
                    let method = words.next().ok_or_else(|| anyhow::anyhow!("missing request method"))?.to_owned();
                    let path = words.next().ok_or_else(|| anyhow::anyhow!("missing request path"))?.to_owned();
                    let names = text.lines().skip(1).filter_map(|s| s.split_once(':').map(|(name, _)| name.to_ascii_lowercase())).collect::<Vec<_>>();
                    let fallback = ResourceFixtureResponse { status: 404, mime: "text/plain", body: b"Unexpected owned route".to_vec(), location: None };
                    let response = routes.get(&path).unwrap_or(&fallback);
                    let mut head = format!("HTTP/1.1 {} Owned\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n", response.status, response.mime, response.body.len());
                    if let Some(target) = &response.location { head.push_str(&format!("Location: {target}\r\n")); }
                    head.push_str("\r\n");
                    socket.write_all(head.as_bytes()).await?;
                    socket.write_all(&response.body).await?;
                    socket.shutdown().await?;
                    let mut tail = [0u8; 1];
                    let client_end = socket.read(&mut tail).await?;
                    anyhow::ensure!(client_end == 0, "client sent data after complete GET response");
                    Ok::<_,anyhow::Error>(ResourceFixtureRequest { method, path, authorization: names.iter().any(|s| s == "authorization"), cookie: names.iter().any(|s| s == "cookie"), own_secret: names.iter().any(|s| s == "x-own-secret"), status: response.status, body_bytes: response.body.len(), client_eof: true })
                }).await.map_err(|_| anyhow::anyhow!("fixture HTTP exchange/normal close deadline"))??;
                log.push(row);
            }
            Ok(log)
        });
        Self { commands, owner }
    }
    async fn snapshot(&self) -> anyhow::Result<Vec<ResourceFixtureRequest>> {
        let (send, receive) = tokio::sync::oneshot::channel();
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.commands.send(ResourceFixtureCommand::Snapshot(send)),
        )
        .await
        .map_err(|_| anyhow::anyhow!("fixture snapshot send deadline"))??;
        Ok(
            tokio::time::timeout(std::time::Duration::from_secs(10), receive)
                .await
                .map_err(|_| anyhow::anyhow!("fixture snapshot response deadline"))??,
        )
    }
    async fn stop(self) -> anyhow::Result<Vec<ResourceFixtureRequest>> {
        let sent = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.commands.send(ResourceFixtureCommand::Stop),
        )
        .await;
        let mut owner = self.owner;
        let joined = match tokio::time::timeout(std::time::Duration::from_secs(10), &mut owner)
            .await
        {
            Ok(joined) => joined,
            Err(_) => {
                owner.abort();
                let _ = owner.await;
                anyhow::bail!(
                    "owned fixture required failed abort-and-join cleanup, no normal close acceptance"
                );
            }
        };
        let rows = joined??;
        sent.map_err(|_| anyhow::anyhow!("fixture stop send deadline"))??;
        Ok(rows)
    }
}
fn resource_fixture_png() -> Vec<u8> {
    use std::io::Write;
    fn chunk(bytes: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        bytes.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        bytes.extend_from_slice(kind);
        bytes.extend_from_slice(data);
        let mut crc = u32::MAX;
        for byte in kind.iter().chain(data) {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb88320u32.wrapping_mul(crc & 1));
            }
        }
        bytes.extend_from_slice(&(!crc).to_be_bytes());
    }
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut bytes, b"IHDR", &ihdr);
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&[0, 0, 0, 0, 255]).unwrap();
    chunk(&mut bytes, b"IDAT", &encoder.finish().unwrap());
    chunk(&mut bytes, b"IEND", &[]);
    bytes
}
