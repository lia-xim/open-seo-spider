//! AUTHORED / NOT_RUN. Real native HTTP dispatcher and disk Store regressions.
use super::*;
use crate::{CrawlAdmissionDenial as Denial, CrawlAdmissionLimits, CrawlAdmissionPhase};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

struct Site {
    origin: String,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<Vec<String>>>,
}
impl Drop for Site {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
impl Site {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let (stop, mut stopped) = oneshot::channel();
        let fixture_origin = origin.clone();
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            loop {
                let accepted =
                    tokio::select! {_=&mut stopped=>break,value=listener.accept()=>value};
                let (mut stream, _) = accepted.unwrap();
                let mut request = Vec::new();
                let mut chunk = [0u8; 1024];
                while !request.windows(4).any(|v| v == b"\r\n\r\n") {
                    let n = stream.read(&mut chunk).await.unwrap();
                    assert!(n > 0);
                    request.extend_from_slice(&chunk[..n]);
                    assert!(request.len() <= 8192);
                }
                let text = std::str::from_utf8(&request).unwrap();
                let target = text
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_owned();
                requests.push(target.clone());
                let (status,mime,extra,body)=match target.as_str() {
                    "/" => (200,"text/html; charset=utf-8","","<a href='/flat'>ok</a><a href='/deep/path'>first</a><a href='/deep/path'>duplicate</a><a href='/query?a=1&a=2'>two</a><a href='/query?a&&'>three</a>".to_owned()),
                    "/many" => (200,"text/html; charset=utf-8","",(0..500).map(|n|format!("<a href='/many-leaf-{n:03}'>page {n}</a>")).collect::<String>()),
                    "/resources" => (200,"text/html; charset=utf-8","","<link rel='stylesheet' href='/style.css'><img src='/image'>".to_owned()),
                    "/style.css" => (200,"text/css; charset=utf-8","","a{background:url('/deep/image.png')}".to_owned()),
                    "/image" => (302,"application/octet-stream","Location: /deep/final.png\r\n",String::new()),
                    "/redir" => (302,"text/html","Location: /deep/target\r\n",String::new()),
                    "/policy-redir" => (302,"text/html","Location: /alias?drop=1&drop=2&keep=3\r\n",String::new()),
                    "/map.xml?a=1&a=2" => (200,"application/xml","",format!("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>{fixture_origin}/query?a&amp;&amp;</loc></url></urlset>")),
                    "/robots.txt" => (200,"text/plain","","User-agent: *\nAllow: /\n".to_owned()),
                    _ => (200,"text/html; charset=utf-8","","<title>owned</title>".to_owned()),
                };
                let response = format!(
                    "HTTP/1.1 {status} Owned\r\nContent-Type: {mime}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
                stream.shutdown().await.unwrap();
            }
            drop(listener);
            requests
        });
        Self {
            origin,
            stop: Some(stop),
            task: Some(task),
        }
    }
    async fn finish(mut self) -> Vec<String> {
        self.stop.take().unwrap().send(()).unwrap();
        self.task.take().unwrap().await.unwrap()
    }
}
fn limits(
    folder: Option<usize>,
    query: Option<usize>,
    bytes: Option<usize>,
) -> CrawlAdmissionLimits {
    CrawlAdmissionLimits {
        max_folder_depth: folder,
        max_query_tokens: query,
        max_fetch_url_utf8_bytes: bytes,
        ..Default::default()
    }
}
fn cfg(site: &Site, path: &str) -> CrawlConfig {
    CrawlConfig {
        seed_urls: vec![format!("{}{path}", site.origin)],
        discover_sitemaps: false,
        obey_robots: false,
        delay_ms: 0,
        concurrency: 1,
        ..Default::default()
    }
}
async fn crawl(config: CrawlConfig) -> crate::CrawlReport {
    Crawler::new(config)
        .unwrap()
        .run(None, Arc::new(AtomicBool::new(false)))
        .await
        .unwrap()
}

#[tokio::test]
async fn static_limits_real_seed_and_discovery_dispatch_keep_duplicate_occurrences_and_deny_no_get()
{
    let site = Site::start().await;
    let mut config = cfg(&site, "/");
    config.crawl_admission_limits = Some(limits(Some(0), Some(2), None));
    let report = crawl(config).await;
    let root = report
        .pages
        .iter()
        .find(|p| p.url == format!("{}/", site.origin))
        .unwrap();
    assert_eq!(
        root.links
            .iter()
            .filter(|l| l.raw_href.as_deref() == Some("/deep/path"))
            .count(),
        2
    );
    let denied = report
        .pages
        .iter()
        .find(|p| p.url.ends_with("/deep/path"))
        .unwrap();
    assert_eq!(denied.status, None);
    assert_eq!(denied.final_status, None);
    assert!(denied.error.is_some());
    assert_eq!(
        denied.crawl_admission.as_ref().unwrap().last.denied,
        Some(Denial::FolderDepth)
    );
    let query = report
        .pages
        .iter()
        .find(|p| p.url.ends_with("/query?a&&"))
        .unwrap();
    assert_eq!(
        query.crawl_admission.as_ref().unwrap().last.denied,
        Some(Denial::QueryTokens)
    );
    let mut requests = site.finish().await;
    requests.sort();
    assert_eq!(requests, vec!["/", "/flat", "/query?a=1&a=2"]);
}

#[tokio::test]
async fn static_limits_denied_initial_target_precedes_robots_rate_and_http() {
    let site = Site::start().await;
    let mut config = cfg(&site, "/deep/path");
    config.obey_robots = true;
    config.delay_ms = 60_000;
    config.crawl_admission_limits = Some(limits(Some(0), None, None));
    let report = tokio::time::timeout(Duration::from_secs(5), crawl(config))
        .await
        .unwrap();
    assert_eq!(report.pages.len(), 1);
    assert_eq!(report.pages[0].status, None);
    assert!(report.pages[0].error.is_some());
    assert!(site.finish().await.is_empty());
}

#[tokio::test]
async fn static_limits_real_fetch_url_byte_equal_and_plus_one_boundary() {
    let site = Site::start().await;
    let mut config = cfg(&site, "/short");
    config.seed_urls.push(format!("{}/shortx", site.origin));
    let cap = format!("{}/short", site.origin).len();
    config.crawl_links = false;
    config.crawl_admission_limits = Some(limits(None, None, Some(cap)));
    let report = crawl(config).await;
    assert_eq!(
        report
            .pages
            .iter()
            .find(|p| p.url.ends_with("/short"))
            .unwrap()
            .final_status,
        Some(200)
    );
    let denied = report
        .pages
        .iter()
        .find(|p| p.url.ends_with("/shortx"))
        .unwrap();
    assert_eq!(denied.status, None);
    assert_eq!(
        denied
            .crawl_admission
            .as_ref()
            .unwrap()
            .last
            .fetch_url_utf8_bytes,
        cap + 1
    );
    assert_eq!(
        denied.crawl_admission.as_ref().unwrap().last.denied,
        Some(Denial::FetchUrlUtf8Bytes)
    );
    assert_eq!(site.finish().await, vec!["/short"]);
}

#[tokio::test]
async fn static_limits_real_redirect_preserves_observed_hop_and_initial_status_without_following() {
    let site = Site::start().await;
    let mut config = cfg(&site, "/redir");
    config.crawl_admission_limits = Some(limits(Some(0), None, None));
    let report = crawl(config).await;
    let p = &report.pages[0];
    assert_eq!(p.url, format!("{}/redir", site.origin));
    assert_eq!(p.status, Some(302));
    assert_eq!(p.final_status, None);
    assert_eq!(p.final_url, format!("{}/deep/target", site.origin));
    assert_eq!(p.redirect_chain.len(), 1);
    assert_eq!(p.redirect_chain[0].target, p.final_url);
    let checked = p.crawl_admission.as_ref().unwrap();
    assert_eq!(checked.checked_targets, 2);
    assert_eq!(checked.initial.denied, None);
    assert_eq!(checked.last.phase, CrawlAdmissionPhase::Redirect);
    assert_eq!(checked.last.denied, Some(Denial::FolderDepth));
    assert_eq!(site.finish().await, vec!["/redir"]);
}

#[tokio::test]
async fn static_limits_actual_policy_fetch_alias_is_checked_without_rewriting_original_occurrence()
{
    let site = Site::start().await;
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("alias.sqlite");
    let raw = format!("{}/alias?drop=1&drop=2&keep=3", site.origin);
    let effective = format!("{}/alias?keep=3", site.origin);
    let mut config = cfg(&site, "/");
    config.seed_urls = vec![raw.clone()];
    config.crawl_links = false;
    config.crawl_admission_limits = Some(limits(None, Some(1), None));
    config.crawl_policy = Some(crate::CrawlPolicyConfig {
        scope: crate::CrawlScopeConfig {
            roots: vec![crate::CrawlScopeRoot {
                origin: site.origin.clone(),
                subdomains: false,
                folders: vec!["/".into()],
                exact_paths: vec![],
            }],
            ..Default::default()
        },
        query: crate::QueryPolicy {
            remove_keys: vec!["drop".into()],
            ..Default::default()
        },
        initial_inputs: crate::InitialInputPolicy {
            rewrite: crate::InitialRewritePolicy::Apply,
            ..Default::default()
        },
        ..Default::default()
    });
    let report = Crawler::new(config)
        .unwrap()
        .with_store(&db)
        .unwrap()
        .run(None, Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    assert_eq!(report.pages.len(), 1);
    let p = &report.pages[0];
    assert_eq!(p.url, effective);
    assert_eq!(p.final_status, Some(200));
    assert_eq!(p.crawl_admission.as_ref().unwrap().initial.query_tokens, 1);
    assert_eq!(p.crawl_admission.as_ref().unwrap().initial.denied, None);
    let saved = crate::CrawlStore::open_read_only(&db).unwrap();
    let rows = saved.policy_occurrences(None, 200).unwrap();
    let original = rows
        .rows
        .iter()
        .find(|r| r.occurrence.decision.raw_value == raw)
        .unwrap();
    assert_eq!(
        original.occurrence.decision.fetch_candidate_url.as_deref(),
        Some(effective.as_str())
    );
    assert!(original.occurrence.scheduled_fetch_url.is_some());
    assert_eq!(site.finish().await, vec!["/alias?keep=3"]);
}

#[tokio::test]
async fn static_limits_real_resource_css_and_redirect_keep_authorizing_grants_and_transport_unknown()
 {
    let site = Site::start().await;
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("resources.sqlite");
    let mut config = cfg(&site, "/resources");
    config.crawl_admission_limits = Some(limits(Some(0), None, None));
    config.resource_discovery = Some(crate::ResourceDiscoveryConfig {
        contract: Some(crate::ResourceContractConfig::default()),
        ..Default::default()
    });
    let report = Crawler::new(config)
        .unwrap()
        .with_store(&db)
        .unwrap()
        .run(None, Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    let image = report
        .pages
        .iter()
        .find(|p| p.url.ends_with("/image"))
        .unwrap();
    assert_eq!(image.status, Some(302));
    assert_eq!(image.final_status, None);
    assert_eq!(
        image.crawl_admission.as_ref().unwrap().last.denied,
        Some(Denial::FolderDepth)
    );
    assert!(image.resource_permission.is_some());
    assert!(image.error.is_some());
    let hop = &image.resource_response.as_ref().unwrap().redirect_decisions[0];
    assert_eq!(hop.raw_location, "/deep/final.png");
    assert!(hop.allowed);
    assert!(!hop.request_started);
    assert!(!hop.authorizing_occurrence_ids.is_empty());
    let css_target = report
        .pages
        .iter()
        .find(|p| p.url.ends_with("/deep/image.png"))
        .unwrap();
    assert_eq!(css_target.status, None);
    assert!(css_target.resource_permission.is_some());
    assert_eq!(
        css_target.crawl_admission.as_ref().unwrap().last.denied,
        Some(Denial::FolderDepth)
    );
    let reopened = crate::CrawlStore::open_read_only(&db).unwrap();
    let rows = reopened.resource_occurrences(None, 200).unwrap();
    let original = rows
        .rows
        .iter()
        .find(|r| r.occurrence.decoded_value.as_deref() == Some("/deep/image.png"))
        .unwrap();
    assert_eq!(original.occurrence.raw_value, "url('/deep/image.png')");
    assert!(original.decision.grant.is_some());
    assert_eq!(
        original.decision.grant.as_ref().unwrap().fetch_url,
        format!("{}/deep/image.png", site.origin)
    );
    let transports = reopened.resource_transports(None, 200).unwrap();
    let denied = transports
        .rows
        .iter()
        .find(|p| p.requested_url.ends_with("/deep/image.png"))
        .unwrap();
    assert_eq!(denied.projection.status, None);
    assert!(denied.projection.error.is_some());
    assert_eq!(
        denied.projection.crawl_admission,
        css_target.crawl_admission
    );
    let mut requests = site.finish().await;
    requests.sort();
    assert_eq!(requests, vec!["/image", "/resources", "/style.css"]);
}

#[tokio::test]
async fn static_limits_redirect_checks_actual_transport_target_not_a_different_policy_preview_alias()
 {
    let site = Site::start().await;
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("redirect-policy.sqlite");
    let mut config = cfg(&site, "/policy-redir");
    config.crawl_links = false;
    config.crawl_admission_limits = Some(limits(None, Some(1), None));
    config.crawl_policy = Some(crate::CrawlPolicyConfig {
        scope: crate::CrawlScopeConfig {
            roots: vec![crate::CrawlScopeRoot {
                origin: site.origin.clone(),
                subdomains: false,
                folders: vec!["/".into()],
                exact_paths: vec![],
            }],
            ..Default::default()
        },
        query: crate::QueryPolicy {
            remove_keys: vec!["drop".into()],
            ..Default::default()
        },
        initial_inputs: crate::InitialInputPolicy {
            rewrite: crate::InitialRewritePolicy::Apply,
            ..Default::default()
        },
        ..Default::default()
    });
    let report = Crawler::new(config)
        .unwrap()
        .with_store(&db)
        .unwrap()
        .run(None, Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    let page = &report.pages[0];
    assert_eq!(page.status, Some(302));
    assert_eq!(page.final_status, None);
    assert_eq!(
        page.redirect_chain[0].target,
        format!("{}/alias?drop=1&drop=2&keep=3", site.origin)
    );
    let expected_alias = format!("{}/alias?keep=3", site.origin);
    let actual_target = format!("{}/alias?drop=1&drop=2&keep=3", site.origin);
    let observed = &page.policy.as_ref().unwrap().redirect_decisions[0].decision;
    assert_eq!(observed.raw_value, "/alias?drop=1&drop=2&keep=3");
    assert_eq!(observed.source_kind, crate::CandidateSourceKind::Redirect);
    assert_eq!(
        observed.fetch_candidate_url.as_deref(),
        Some(actual_target.as_str())
    );
    assert!(!observed.policy_steps.iter().any(|step| matches!(
        step.kind,
        crate::CrawlPolicyStepKind::QueryRemoval | crate::CrawlPolicyStepKind::Rewrite
    )));
    // Redirect observations never run discovery transforms. The same original
    // target has a distinct discovery alias, proved through the real policy.
    let policy =
        crate::CompiledCrawlPolicy::new(report.config.crawl_policy.as_ref().unwrap()).unwrap();
    let preview = policy.evaluate(&crate::CandidateInput {
        occurrence_id: "static-limits-discovery-preview-control".into(),
        source_kind: crate::CandidateSourceKind::Anchor,
        raw_value: observed.raw_value.clone(),
        base_url: Some(format!("{}/policy-redir", site.origin)),
        source_url: Some(page.url.clone()),
        source_location: None,
    });
    assert_eq!(preview.raw_value, observed.raw_value);
    assert_eq!(
        preview.fetch_candidate_url.as_deref(),
        Some(expected_alias.as_str())
    );
    assert_ne!(preview.fetch_candidate_url, observed.fetch_candidate_url);
    let actual = page.crawl_admission.as_ref().unwrap();
    assert_eq!(actual.last.phase, CrawlAdmissionPhase::Redirect);
    assert_eq!(actual.last.query_tokens, 3);
    assert_eq!(actual.last.denied, Some(Denial::QueryTokens));
    assert_eq!(site.finish().await, vec!["/policy-redir"]);
}

#[tokio::test]
async fn static_limits_sitemap_protocol_scope_remains_separate_while_discovered_document_is_denied()
{
    let site = Site::start().await;
    let mut config = cfg(&site, "/");
    config.seed_urls.clear();
    config.sitemap_urls = vec![format!("{}/map.xml?a=1&a=2", site.origin)];
    config.crawl_admission_limits = Some(limits(None, Some(0), None));
    let report = crawl(config).await;
    assert_eq!(report.pages.len(), 1);
    let p = &report.pages[0];
    assert!(p.url.ends_with("/query?a&&"));
    assert_eq!(p.status, None);
    assert_eq!(
        p.crawl_admission.as_ref().unwrap().last.denied,
        Some(Denial::QueryTokens)
    );
    assert_eq!(site.finish().await, vec!["/map.xml?a=1&a=2"]);
}

#[tokio::test]
async fn static_limits_selected_recrawl_inherits_limits_and_cannot_turn_denial_into_fetch() {
    let site = Site::start().await;
    let mut config = cfg(&site, "/deep/path");
    config.crawl_admission_limits = Some(limits(Some(0), None, None));
    let parent = crawl(config).await;
    let snapshot = serde_json::to_vec(&parent).unwrap();
    let target = parent.pages[0].url.clone();
    let prepared = crate::prepare_selected_recrawl(
        crate::SelectedRecrawlParent {
            job_id: "static-limits-parent".into(),
            checkpoint_revision: None,
            source_kind: crate::RecrawlSourceKind::ImportedSnapshot,
        },
        &snapshot,
        std::slice::from_ref(&target),
        None,
    )
    .unwrap();
    let mut changed = prepared.config.clone();
    changed.crawl_admission_limits = None;
    assert!(
        Crawler::from_saved_config(changed)
            .unwrap()
            .with_selected_recrawl(prepared.clone())
            .is_err()
    );
    let cfg = prepared.config.clone();
    let report = Crawler::from_saved_config(cfg)
        .unwrap()
        .with_selected_recrawl(prepared)
        .unwrap()
        .run(None, Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    assert_eq!(
        report.config.crawl_admission_limits,
        parent.config.crawl_admission_limits
    );
    let page = report.pages.iter().find(|p| p.url == target).unwrap();
    assert_eq!(page.status, None);
    assert_eq!(
        page.crawl_admission.as_ref().unwrap().last.denied,
        Some(Denial::FolderDepth)
    );
    assert!(site.finish().await.is_empty());
}

#[tokio::test]
async fn static_limits_disabled_native_crawl_really_fetches_and_saves_501_pages_with_none_and_legacy_zero()
 {
    // Two genuine finite native crawls. This is a page-count baseline, not a
    // sitemap-entry/resource-request count or a throughput/RSS benchmark.
    for max_pages in [None, Some(0)] {
        let site = Site::start().await;
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("pages-501.sqlite");
        let mut config = cfg(&site, "/many");
        config.max_pages = max_pages;
        // Explicit null and omission both select the same disabled profile.
        let mut raw = serde_json::to_value(&config).unwrap();
        raw.as_object_mut()
            .unwrap()
            .insert("crawl_admission_limits".into(), serde_json::Value::Null);
        let config: CrawlConfig = serde_json::from_value(raw).unwrap();
        let crawler = Crawler::new(config).unwrap();
        assert_eq!(crawler.effective_config().max_pages, None);
        assert!(crawler.effective_config().crawl_admission_limits.is_none());
        assert!(
            serde_json::to_value(crawler.effective_config())
                .unwrap()
                .get("crawl_admission_limits")
                .is_none()
        );
        let report = crawler
            .with_store(&db)
            .unwrap()
            .run(None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        assert_eq!(report.pages.len(), 501);
        assert!(report.pages.iter().all(|p| p.status == Some(200)
            && p.final_status == Some(200)
            && p.error.is_none()
            && p.crawl_admission.is_none()));
        let mut expected = vec!["/many".to_owned()];
        expected.extend((0..500).map(|n| format!("/many-leaf-{n:03}")));
        expected.sort();
        let mut requests = site.finish().await;
        requests.sort();
        assert_eq!(requests, expected);
        let reopened = crate::CrawlStore::open_read_only(&db).unwrap();
        let saved = reopened.pages().unwrap();
        assert_eq!(saved.len(), 501);
        let mut saved_urls = saved.iter().map(|p| p.url.clone()).collect::<Vec<_>>();
        saved_urls.sort();
        let mut report_urls = report
            .pages
            .iter()
            .map(|p| p.url.clone())
            .collect::<Vec<_>>();
        report_urls.sort();
        assert_eq!(saved_urls, report_urls);
        assert!(
            saved
                .iter()
                .all(|p| p.final_status == Some(200) && p.crawl_admission.is_none())
        );
        assert!(
            reopened
                .config()
                .unwrap()
                .unwrap()
                .crawl_admission_limits
                .is_none()
        );
    }
}
