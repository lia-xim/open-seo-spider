//! AUTHORED_NOT_RUN. Real finite local HTTP sites, no report upload or DNS claim.
use super::*;
use crate::{CrawlConfig, Crawler};

fn config(origins: &[&str]) -> CdnClassificationConfig {
    CdnClassificationConfig {
        version: 1,
        origins: origins.iter().map(|v| (*v).into()).collect(),
    }
}

#[test]
fn cdn_config_exact_scheme_host_effective_port_idna_and_no_subdomain_inheritance() {
    let c = config(&[
        "https://CDN.example:443/",
        "http://cdn.example:8080",
        "https://bücher.example",
    ])
    .compile()
    .unwrap();
    for value in [
        "https://cdn.example/a?x=1",
        "https://cdn.example:443/",
        "http://cdn.example:8080/x",
        "https://xn--bcher-kva.example/x",
    ] {
        let o = c.observation(value, false).unwrap();
        assert!(!o.seed_scope_internal);
        assert!(!o.grants_network_or_credentials);
        assert_eq!(o.basis, "declared_config_exact_origin");
        assert_eq!(o.config_sha256.len(), 64);
    }
    for value in [
        "http://cdn.example/",
        "https://cdn.example:444/",
        "https://sub.cdn.example/",
        "https://badcdn.example/",
        "https://cdn.example.attacker.test/",
        "https://user@cdn.example/",
    ] {
        assert!(c.observation(value, false).is_none(), "{value}");
    }
}

#[test]
fn cdn_config_strict_version_shapes_origins_duplicates_and_budgets() {
    for input in [
        r#"[]"#,
        r#"[1,["https://cdn.example"]]"#,
        r#"{}"#,
        r#"{"version":1,"origins":[],"expand":true}"#,
        r#"{"version":1,"version":1,"origins":[]}"#,
    ] {
        assert!(
            serde_json::from_str::<CdnClassificationConfig>(input).is_err(),
            "{input}"
        );
    }
    for value in [
        "cdn.example",
        "https://*.example/",
        "https://cdn.example/x",
        "https://cdn.example/a/..",
        "https://cdn.example/%2e",
        "https://cdn.example?",
        "https://cdn.example#",
        "https://@cdn.example",
        "https://cdn.example\\x",
        " https://cdn.example",
        "ftp://cdn.example",
    ] {
        assert!(config(&[value]).validate().is_err(), "{value}");
    }
    assert!(config(&[]).validate().is_err());
    assert!(
        CdnClassificationConfig {
            version: 2,
            ..config(&["https://cdn.example"])
        }
        .validate()
        .is_err()
    );
    assert!(
        config(&["https://CDN.example/", "https://cdn.example:443"])
            .validate()
            .is_err()
    );
    assert!(
        CdnClassificationConfig {
            version: 1,
            origins: (0..33).map(|n| format!("https://c{n}.example")).collect()
        }
        .validate()
        .is_err()
    );
    assert!(
        config(&[&format!("https://{}.example", "a".repeat(2050))])
            .validate()
            .is_err()
    );
}

#[test]
fn cdn_optional_absence_and_null_preserve_legacy_config_page_and_link_json() {
    let absent: CrawlConfig = serde_json::from_str("{}").unwrap();
    let null: CrawlConfig = serde_json::from_str(r#"{"cdn_classification":null}"#).unwrap();
    assert_eq!(
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&null).unwrap()
    );
    assert!(
        serde_json::to_value(absent)
            .unwrap()
            .get("cdn_classification")
            .is_none()
    );
    let page = PageRecord::default();
    let link = LinkRecord::default();
    assert!(
        serde_json::to_value(page)
            .unwrap()
            .get("cdn_classification")
            .is_none()
    );
    assert!(
        serde_json::to_value(link)
            .unwrap()
            .get("cdn_classification")
            .is_none()
    );
}

#[test]
fn cdn_page_and_link_reporting_is_explicit_and_never_navigation_or_credential_authority() {
    let c = config(&["https://cdn.example"]).compile().unwrap();
    let seeds = vec![Url::parse("https://www.example/").unwrap()];
    let mut page = PageRecord {
        url: "https://cdn.example/a".into(),
        final_url: "https://cdn.example/a".into(),
        links: vec![
            LinkRecord {
                url: "https://cdn.example/next".into(),
                ..Default::default()
            },
            LinkRecord {
                url: "https://foreign.example/".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    c.apply(&mut page);
    assert!(page.internal);
    assert!(page.links[0].internal);
    assert!(!page.links[1].internal);
    assert!(!navigation_source_internal(&page, &seeds, false, true));
    assert!(!navigation_target_internal(
        &page.links[0],
        &seeds,
        false,
        true
    ));
    let fact = page.cdn_classification.as_ref().unwrap();
    assert!(!fact.seed_scope_internal);
    assert!(!fact.grants_network_or_credentials);
    assert_eq!(fact.rule_index, 0);
    assert!(navigation_source_internal(&page, &seeds, false, false)); // Exact old bool behavior when the profile is absent.
    let mut seed = PageRecord {
        url: "https://www.example/".into(),
        final_url: "https://foreign.example/".into(),
        internal: true,
        ..Default::default()
    };
    assert!(!navigation_source_internal(&seed, &seeds, false, true));
    seed.final_url = seed.url.clone();
    assert!(navigation_source_internal(&seed, &seeds, false, true));
}

#[test]
fn cdn_resource_grants_and_policy_scope_remain_independent_of_reporting_declaration() {
    let c = config(&["https://cdn.example"]).compile().unwrap();
    let seeds = vec![Url::parse("https://www.example/").unwrap()];
    let target = Url::parse("https://cdn.example/a.js").unwrap();
    let resource = crate::ResourceDiscoveryConfig::default();
    assert!(c.observation(target.as_str(), false).is_some());
    assert!(!resource.permits_resource_origin(&target, &seeds, false));
    let mut granted = resource;
    granted.resource_scope.external_origins = vec!["https://cdn.example".into()];
    assert!(granted.permits_resource_origin(&target, &seeds, false));
    assert!(!crate::urls::same_scope(&target, &seeds, false));
    assert!(!granted.permits_resource_origin(
        &Url::parse("https://sub.cdn.example/a.js").unwrap(),
        &seeds,
        false
    ));
}

async fn site(
    listener: tokio::net::TcpListener,
    body: String,
    redirect: Option<String>,
) -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<Vec<String>>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (stop, mut stopped) = tokio::sync::oneshot::channel();
    let owner = tokio::spawn(async move {
        let mut paths = Vec::new();
        loop {
            let accepted =
                tokio::select! {_=&mut stopped=>break,accepted=listener.accept()=>accepted};
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
            let path = text
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
                .to_owned();
            paths.push(path);
            assert!(!text.to_ascii_lowercase().contains("x-owned-secret:"));
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
async fn cdn_real_two_origin_classification_cannot_expand_external_html_and_survives_store_reopen()
{
    let a = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let b = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let primary = format!("http://{}/", a.local_addr().unwrap());
    let cdn = format!("http://{}", b.local_addr().unwrap());
    let(stop_a,owner_a)=site(a,format!("<html><head><title>Primary</title></head><body><a href='{cdn}/doc'>CDN</a></body></html>"),None).await;
    let(stop_b,owner_b)=site(b,"<html><head><title>CDN</title></head><body><a href='/child'>Must not expand</a><a href='http://127.0.0.1:1/foreign'>Must not visit</a></body></html>".into(),None).await;
    let cfg = CrawlConfig {
        seed_urls: vec![primary],
        cdn_classification: Some(config(&[&cdn])),
        discover_sitemaps: false,
        obey_robots: false,
        delay_ms: 0,
        concurrency: 1,
        max_pages: Some(10),
        ..Default::default()
    };
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
    assert!(report.pages[0].links[0].internal);
    assert!(report.pages[0].links[0].cdn_classification.is_some());
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("crawl.sqlite");
    let mut checked = cfg;
    checked.check_external = true;
    let report = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        Crawler::new(checked.clone())
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
        .find(|p| p.url == format!("{cdn}/doc"))
        .unwrap();
    assert!(page.internal);
    assert!(
        !page
            .cdn_classification
            .as_ref()
            .unwrap()
            .seed_scope_internal
    );
    assert_eq!(page.links.len(), 2);
    stop_a.send(()).unwrap();
    stop_b.send(()).unwrap();
    let paths_a = tokio::time::timeout(std::time::Duration::from_secs(5), owner_a)
        .await
        .unwrap()
        .unwrap();
    let paths_b = tokio::time::timeout(std::time::Duration::from_secs(5), owner_b)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(paths_a, vec!["/", "/"]);
    assert_eq!(paths_b, vec!["/doc"]);
    let store = crate::CrawlStore::open_read_only(&db).unwrap();
    let restored = store.load_report().unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(restored.config.cdn_classification).unwrap(),
        serde_json::to_value(checked.cdn_classification).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&restored.pages).unwrap(),
        serde_json::to_value(&report.pages).unwrap()
    );
}

#[tokio::test]
async fn cdn_real_redirect_to_classified_origin_is_recorded_without_new_target_get() {
    let a = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let b = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let primary = format!("http://{}/", a.local_addr().unwrap());
    let cdn = format!("http://{}", b.local_addr().unwrap());
    let (stop_a, owner_a) = site(a, String::new(), Some(format!("{cdn}/blocked"))).await;
    let (stop_b, owner_b) = site(b, "Forbidden target".into(), None).await;
    let cfg = CrawlConfig {
        seed_urls: vec![primary],
        cdn_classification: Some(config(&[&cdn])),
        discover_sitemaps: false,
        obey_robots: false,
        crawl_links: true,
        delay_ms: 0,
        request_timeout_secs: 3,
        ..Default::default()
    };
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
    let page = &report.pages[0];
    assert_eq!(page.status, Some(302));
    assert_eq!(page.final_status, None);
    assert_eq!(page.redirect_chain[0].target, format!("{cdn}/blocked"));
    assert!(
        page.warnings
            .iter()
            .any(|s| s.contains("outside crawl scope"))
    );
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
