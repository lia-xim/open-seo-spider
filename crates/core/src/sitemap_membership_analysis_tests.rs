use super::*;
use crate::{CrawlConfig, CrawlControl, CrawlStore, Crawler};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct Site {
    origin: String,
    count: Arc<AtomicUsize>,
    stop: tokio::sync::oneshot::Sender<()>,
    join: tokio::task::JoinHandle<()>,
}
impl Site {
    async fn start(incomplete: bool) -> Self {
        Self::start_mode(incomplete, false).await
    }
    async fn start_mode(incomplete: bool, siblings: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let prefix = origin.clone();
        let count = Arc::new(AtomicUsize::new(0));
        let seen = count.clone();
        let (stop, mut stopping) = tokio::sync::oneshot::channel();
        let join = tokio::spawn(async move {
            let mut jobs = tokio::task::JoinSet::new();
            loop {
                tokio::select! {_=&mut stopping=>break, accepted=listener.accept()=>{
                    let (mut socket,_)=accepted.unwrap();seen.fetch_add(1,Ordering::SeqCst);let prefix=prefix.clone();
                    jobs.spawn(async move {
                        let mut request=vec![];let mut buffer=[0;2048];
                        while !request.windows(4).any(|v|v==b"\r\n\r\n") {let n=socket.read(&mut buffer).await.unwrap();if n==0{return}request.extend_from_slice(&buffer[..n]);assert!(request.len()<65536);}
                        let text=String::from_utf8(request).unwrap();let route=text.lines().next().unwrap().split_whitespace().nth(1).unwrap();
                        let (status,mime,extra,body)=match route {
                            "/robots.txt"=>("200 OK","text/plain",String::new(),"User-agent: *\nDisallow: /blocked\n".into()),
                            "/source.xml" if siblings=>("200 OK","application/xml",String::new(),format!("<sitemapindex><sitemap><loc>{prefix}/leaf-one.xml</loc></sitemap><sitemap><loc>{prefix}/leaf-one.xml</loc></sitemap><sitemap><loc>{prefix}/leaf-two.xml</loc></sitemap></sitemapindex>")),
                            "/leaf-one.xml"=>("200 OK","application/xml",String::new(),format!("<urlset><url><loc>{prefix}/same-doc</loc></url><url><loc>{prefix}/same-doc</loc></url><url><loc>{prefix}/two-docs</loc></url></urlset>")),
                            "/leaf-two.xml"=>("200 OK","application/xml",String::new(),format!("<urlset><url><loc>{prefix}/two-docs</loc></url></urlset>")),
                            "/source.xml"=>{let paths=["/listed","/listed","/redirect","/missing","/noindex","/blocked","/sitemap-only","/old"];
                                let mut xml=format!("<urlset>{}",paths.iter().map(|p|format!("<url><loc>{prefix}{p}</loc></url>")).collect::<String>());
                                xml.push_str(if incomplete {"<url><loc>"}else{"</urlset>"});("200 OK","application/xml",String::new(),xml)},
                            "/redirect"=>("301 Moved Permanently","text/html",format!("Location: {prefix}/final\r\n"),String::new()),
                            "/missing"=>("404 Not Found","text/html",String::new(),"<html><body>Missing</body></html>".into()),
                            "/noindex"=>("200 OK","text/html",String::new(),"<html><head><meta name='robots' content='noindex'></head><body>Private</body></html>".into()),
                            "/seed"=>("200 OK","text/html",String::new(),"<html><body><a href='/listed'>One</a><a href='/listed' rel='nofollow'>Two</a><a href='/not-listed'>Other</a><a href='/final'>Final</a></body></html>".into()),
                            "/listed"=>("200 OK","text/html",String::new(),"<html><body><a href='/listed'>Self</a><img src='/sitemap-only'></body></html>".into()),
                            _=>("200 OK","text/html",String::new(),"<html><head><title>Own fixture</title></head><body>Own page</body></html>".into()),
                        };
                        let headers=format!("HTTP/1.1 {status}\r\nContent-Type: {mime}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",body.len());
                        socket.write_all(headers.as_bytes()).await.unwrap();socket.write_all(body.as_bytes()).await.unwrap();let _=socket.shutdown().await;
                    });
                }}
            }
            while let Some(result) = jobs.join_next().await {
                result.unwrap();
            }
        });
        Self {
            origin,
            count,
            stop,
            join,
        }
    }
    fn config(&self) -> CrawlConfig {
        CrawlConfig {
            seed_urls: vec![format!("{}/seed", self.origin)],
            sitemap_urls: vec![format!("{}/source.xml", self.origin)],
            obey_robots: true,
            discover_sitemaps: false,
            crawl_links: true,
            concurrency: 1,
            delay_ms: 0,
            request_timeout_secs: 3,
            ..Default::default()
        }
    }
    async fn finish(self, requests: usize) {
        let _ = self.stop.send(());
        tokio::time::timeout(std::time::Duration::from_secs(5), self.join)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            self.count.load(Ordering::SeqCst),
            requests,
            "Read-only analysis did not fetch"
        );
    }
}
async fn crawl(
    site: &Site,
    config: CrawlConfig,
) -> (
    tempfile::TempDir,
    std::path::PathBuf,
    crate::ControlledCrawlResult,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crawl.sqlite");
    let crawler = Crawler::new(config).unwrap().with_store(&path).unwrap();
    let result = crawler
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    drop(crawler);
    assert_eq!(result.checkpoint.as_ref().unwrap().generation, 1);
    assert!(site.count.load(Ordering::SeqCst) > 0);
    (dir, path, result)
}
fn request() -> SitemapMembershipAnalysisRequest {
    SitemapMembershipAnalysisRequest {
        kind: SitemapAnalysisRowKind::Location,
        row_limit: 200,
        page_limit: 200,
        ..Default::default()
    }
}
fn analyze(
    path: &std::path::Path,
    request: &SitemapMembershipAnalysisRequest,
) -> SitemapReadSnapshot {
    CrawlStore::open_read_only(path)
        .unwrap()
        .sitemap_read_snapshot_with_membership_analysis(request)
        .unwrap()
}
#[tokio::test]
async fn real_crawler_original_redirect_404_noindex_robots_and_exact_observed_inlinks() {
    let site = Site::start(false).await;
    let (_dir, path, result) = crawl(&site, site.config()).await;
    assert_eq!(result.outcome, CrawlOutcome::Completed);
    let requests = site.count.load(Ordering::SeqCst);
    let saved = std::fs::read(&path).unwrap();
    let read = analyze(&path, &request());
    let coverage = read.coverage.clone();
    assert!(coverage.current_valid_original_location_negative_permitted);
    let analysis = read.membership_analysis.unwrap();
    assert!(analysis.context.available);
    assert!(!analysis.context.website_wide_orphan_inference);
    let locations = analysis.location_findings.unwrap();
    let finding = |path: &str| {
        locations
            .iter()
            .find(|v| {
                v.original_membership_url.as_deref()
                    == Some(format!("{}{path}", site.origin).as_str())
            })
            .unwrap()
    };
    assert_eq!(
        finding("/redirect").original_state,
        SitemapTargetState::Redirected
    );
    let redirect = finding("/redirect").original_observation.as_ref().unwrap();
    assert_eq!(redirect.status, Some(301));
    assert_eq!(redirect.final_status, Some(200));
    assert_eq!(redirect.final_url, format!("{}/final", site.origin));
    assert_eq!(redirect.requested_url, format!("{}/redirect", site.origin));
    assert_eq!(
        finding("/missing").original_state,
        SitemapTargetState::Non200
    );
    assert_eq!(
        finding("/noindex").original_state,
        SitemapTargetState::NonIndexable
    );
    assert_eq!(
        finding("/blocked").original_state,
        SitemapTargetState::BlockedByRobots
    );
    let links = finding("/listed")
        .observed_original_inlinks
        .as_ref()
        .unwrap();
    assert_eq!(links.anchor_occurrences, 3);
    assert_eq!(links.nonself_anchor_occurrences, 2);
    assert_eq!(links.distinct_nonself_requested_sources, 1);
    assert!(links.samples.iter().any(|v| v.nofollow));
    let pages = analysis.saved_pages.unwrap();
    let page = |path: &str| {
        pages
            .iter()
            .find(|v| v.observation.requested_url == format!("{}{path}", site.origin))
            .unwrap()
    };
    assert_eq!(page("/listed").original_membership_occurrences, 2);
    assert_eq!(
        page("/final").membership,
        SitemapSavedPageMembership::NotListedCurrentDeclaredSources,
        "Redirect end identity must not inherit original membership"
    );
    assert_eq!(
        page("/not-listed").membership,
        SitemapSavedPageMembership::NotListedCurrentDeclaredSources
    );
    assert!(page("/sitemap-only").listed_without_observed_nonself_inlinks);
    assert_eq!(
        page("/sitemap-only").observed_inlinks.anchor_occurrences,
        0,
        "An img URL is not an anchor"
    );
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    site.finish(requests).await;
}
#[tokio::test]
async fn real_policy_rewrite_preserves_original_and_exposes_separate_actual_alias() {
    let site = Site::start(false).await;
    let mut config = site.config();
    config.crawl_policy = Some(crate::CrawlPolicyConfig {
        scope: crate::CrawlScopeConfig {
            roots: vec![crate::CrawlScopeRoot {
                origin: site.origin.clone(),
                folders: vec!["/".into()],
                subdomains: false,
                exact_paths: vec![],
            }],
            ..Default::default()
        },
        rewrites: vec![crate::RewriteRule {
            id: "old-to-new".into(),
            pattern: "/old$".into(),
            replacement: crate::RewriteReplacement::Literal {
                text: "/new".into(),
            },
        }],
        ..Default::default()
    });
    let (_dir, path, _) = crawl(&site, config).await;
    let requests = site.count.load(Ordering::SeqCst);
    let read = analyze(&path, &request());
    let analysis = read.membership_analysis.unwrap();
    let old = analysis
        .location_findings
        .as_ref()
        .unwrap()
        .iter()
        .find(|v| {
            v.original_membership_url.as_deref() == Some(format!("{}/old", site.origin).as_str())
        })
        .unwrap();
    assert_eq!(
        old.admitted_fetch_url.as_deref(),
        Some(format!("{}/new", site.origin).as_str())
    );
    assert_eq!(
        old.original_state,
        SitemapTargetState::NoStoredRequestedObservation
    );
    assert!(old.original_observation.is_none());
    assert_eq!(
        old.admitted_alias_observation
            .as_ref()
            .unwrap()
            .requested_url,
        format!("{}/new", site.origin)
    );
    let alias = analysis
        .saved_pages
        .as_ref()
        .unwrap()
        .iter()
        .find(|v| v.observation.requested_url == format!("{}/new", site.origin))
        .unwrap();
    assert_eq!(alias.original_membership_occurrences, 0);
    assert_eq!(alias.admitted_alias_occurrences, 1);
    assert_eq!(
        alias.membership,
        SitemapSavedPageMembership::NotListedCurrentDeclaredSources
    );
    site.finish(requests).await;
}
#[tokio::test]
async fn incomplete_source_prevents_negative_and_independent_page_pagination_has_no_gaps() {
    let site = Site::start(true).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let requests = site.count.load(Ordering::SeqCst);
    let mut query = request();
    query.row_limit = 1;
    query.page_limit = 2;
    let mut urls = vec![];
    let mut context = None;
    loop {
        let read = analyze(&path, &query);
        assert!(
            !read
                .coverage
                .current_valid_original_location_negative_permitted
        );
        let analysis = read.membership_analysis.unwrap();
        assert!(analysis.context.available);
        if let Some(previous) = &context {
            assert_eq!(previous, &analysis.context)
        } else {
            context = Some(analysis.context.clone());
        }
        for page in analysis.saved_pages.unwrap() {
            if page.original_membership_occurrences == 0 {
                assert_eq!(page.membership, SitemapSavedPageMembership::Unknown);
            }
            urls.push(page.observation.requested_url);
        }
        if !analysis.saved_pages_have_more {
            break;
        }
        query.after_page_ordinal = analysis.last_saved_page_ordinal;
    }
    let actual = CrawlStore::open_read_only(&path)
        .unwrap()
        .load_report()
        .unwrap()
        .unwrap();
    let expected: BTreeSet<_> = actual.pages.iter().map(|v| v.url.clone()).collect();
    assert_eq!(urls.iter().cloned().collect::<BTreeSet<_>>(), expected);
    assert_eq!(urls.len(), expected.len());
    site.finish(requests).await;
}
#[tokio::test]
async fn actual_limited_checkpoint_exposes_pending_and_live_resume_exposes_no_saved_join() {
    let site = Site::start(false).await;
    let mut config = site.config();
    config.max_pages = Some(1);
    let (_dir, path, result) = crawl(&site, config).await;
    assert_eq!(result.outcome, CrawlOutcome::Limited);
    let requests = site.count.load(Ordering::SeqCst);
    let read = analyze(&path, &request());
    let analysis = read.membership_analysis.unwrap();
    assert!(analysis.context.available);
    assert_eq!(
        analysis.context.observation_scope,
        "partial_terminal_saved_report_observations"
    );
    assert!(
        analysis
            .location_findings
            .unwrap()
            .iter()
            .any(|v| v.original_state == SitemapTargetState::Pending)
    );
    let store = CrawlStore::open(&path).unwrap();
    store.begin_run("real-resume-without-checkpoint").unwrap();
    drop(store);
    let live = analyze(&path, &request()).membership_analysis.unwrap();
    assert!(!live.context.available);
    assert_eq!(
        live.context.reason,
        "live_crawl_target_analysis_unavailable"
    );
    assert!(live.location_findings.is_none());
    assert!(live.saved_pages.is_none());
    site.finish(requests).await;
}
#[tokio::test]
async fn real_selected_recrawl_retains_historical_membership_and_mixed_page_age() {
    let site = Site::start(false).await;
    let (_parent_dir, _, parent) = crawl(&site, site.config()).await;
    let bytes = serde_json::to_vec(&parent.report).unwrap();
    let prepared = crate::prepare_selected_recrawl(
        crate::SelectedRecrawlParent {
            job_id: "actual-parent".into(),
            checkpoint_revision: parent.checkpoint.as_ref().map(|v| v.revision),
            source_kind: crate::RecrawlSourceKind::StoredJson,
        },
        &bytes,
        &[format!("{}/listed", site.origin)],
        None,
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("selected.sqlite");
    let crawler = Crawler::from_saved_config(prepared.config.clone())
        .unwrap()
        .with_selected_recrawl(prepared)
        .unwrap()
        .with_store(&path)
        .unwrap();
    let result = crawler
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    drop(crawler);
    assert_eq!(result.outcome, CrawlOutcome::Completed);
    let requests = site.count.load(Ordering::SeqCst);
    let read = analyze(&path, &request());
    assert!(
        read.coverage
            .historical_valid_original_location_negative_permitted
    );
    assert!(
        !read
            .coverage
            .current_valid_original_location_negative_permitted
    );
    let analysis = read.membership_analysis.unwrap();
    assert_eq!(
        analysis.context.observation_scope,
        "mixed_age_selected_saved_report"
    );
    let pages = analysis.saved_pages.unwrap();
    let selected = pages
        .iter()
        .find(|v| v.observation.requested_url.ends_with("/listed"))
        .unwrap();
    assert_eq!(
        selected.observation.age_scope,
        "selected_current_generation"
    );
    let other = pages
        .iter()
        .find(|v| v.observation.requested_url.ends_with("/not-listed"))
        .unwrap();
    assert_eq!(other.observation.age_scope, "inherited_parent_observation");
    assert_eq!(
        other.membership,
        SitemapSavedPageMembership::NotListedHistoricalDeclaredSources
    );
    site.finish(requests).await;
}
#[tokio::test]
async fn report_integrity_fails_atomically_before_findings() {
    let site = Site::start(false).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let requests = site.count.load(Ordering::SeqCst);
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE metadata SET value=value||' ' WHERE key='report'",
            [],
        )
        .unwrap();
    drop(connection);
    let failure = CrawlStore::open_read_only(&path)
        .unwrap()
        .sitemap_read_snapshot_with_membership_analysis(&request())
        .unwrap_err();
    assert!(failure.to_string().contains("SHA differs"));
    site.finish(requests).await;
}

#[test]
fn directed_stored_render_field_projection_qualifies_navigation_status_and_missing_evidence() {
    // Directed stored-field SQL projection controls, not evidence that an actual
    // browser renderer executed these navigation/status cases. End-to-end real
    // renderer acceptance remains a separate gate.
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    let base = crate::PageRecord {
        url: "https://example.org/original".into(),
        final_url: "https://example.org/original".into(),
        status: Some(200),
        final_status: Some(200),
        content_type: "text/html".into(),
        indexable: true,
        fetched_at: "stored-control".into(),
        ..Default::default()
    };
    let cases = [
        (
            "same",
            Some("https://example.org/original"),
            Some(200),
            true,
            SitemapTargetState::Indexable,
            "observed_200",
        ),
        (
            "fragment-only",
            Some("https://example.org/original#section"),
            Some(200),
            true,
            SitemapTargetState::Indexable,
            "observed_200",
        ),
        (
            "navigation",
            Some("https://example.org/elsewhere"),
            Some(200),
            true,
            SitemapTargetState::RenderedNavigationChanged,
            "navigation_changed",
        ),
        (
            "navigation404",
            Some("https://example.org/elsewhere"),
            Some(404),
            true,
            SitemapTargetState::RenderedNavigationChanged,
            "non200",
        ),
        (
            "non200",
            Some("https://example.org/original"),
            Some(404),
            true,
            SitemapTargetState::RenderedNon200,
            "non200",
        ),
        (
            "noStatus",
            Some("https://example.org/original"),
            None,
            true,
            SitemapTargetState::IncompleteObservation,
            "response_unobserved",
        ),
        (
            "noUrl",
            None,
            Some(200),
            true,
            SitemapTargetState::IncompleteObservation,
            "url_unobserved",
        ),
        (
            "notRendered",
            None,
            None,
            false,
            SitemapTargetState::IncompleteObservation,
            "required_but_unobserved",
        ),
    ];
    for (name, url, status, rendered, state, rendering_state) in cases {
        let mut page = base.clone();
        page.rendered = rendered;
        page.rendered_url = url.map(str::to_owned);
        page.rendered_status = status;
        let report = serde_json::json!({"config":{"renderer":{}},"pages":[page]});
        connection
            .execute(
                "INSERT OR REPLACE INTO metadata VALUES('report',?1)",
                [serde_json::to_string(&report).unwrap()],
            )
            .unwrap();
        let records =
            read_target_pages(&connection, &BTreeSet::from([base.url.clone()]), false, 1).unwrap();
        let record = &records[&base.url];
        assert_eq!(record.state, state, "{name}");
        assert_eq!(record.rendering_state, rendering_state, "{name}");
        assert!(record.rendering_expected, "{name}");
        assert_eq!(record.requested_url, base.url);
        assert_eq!(record.final_url, base.final_url);
        assert_eq!(record.status, Some(200));
        assert_eq!(record.final_status, Some(200));
        assert_eq!(record.rendered_url.as_deref(), url);
        assert_eq!(record.rendered_status, status);
    }
}

#[tokio::test]
async fn real_sibling_documents_distinguish_multiple_sitemaps_from_same_document_duplicates() {
    let site = Site::start_mode(false, true).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let requests = site.count.load(Ordering::SeqCst);
    let read = analyze(&path, &request());
    assert!(
        read.coverage
            .current_valid_original_location_negative_permitted
    );
    let analysis = read.membership_analysis.unwrap();
    let pages = analysis.saved_pages.as_ref().unwrap();
    let same = pages
        .iter()
        .find(|v| v.observation.requested_url == format!("{}/same-doc", site.origin))
        .unwrap();
    assert_eq!(same.original_membership_occurrences, 2);
    assert_eq!(same.original_source_document_count, 1);
    assert_eq!(same.original_source_observation_count, 1);
    assert_eq!(
        same.original_source_document_samples[0].task_url,
        format!("{}/leaf-one.xml", site.origin)
    );
    let multiple = pages
        .iter()
        .find(|v| v.observation.requested_url == format!("{}/two-docs", site.origin))
        .unwrap();
    assert_eq!(multiple.original_membership_occurrences, 2);
    assert_eq!(multiple.original_source_document_count, 2);
    assert_eq!(multiple.original_source_observation_count, 2);
    let source_urls: BTreeSet<_> = multiple
        .original_source_document_samples
        .iter()
        .map(|v| v.task_url.clone())
        .collect();
    assert_eq!(
        source_urls,
        BTreeSet::from([
            format!("{}/leaf-one.xml", site.origin),
            format!("{}/leaf-two.xml", site.origin)
        ])
    );
    assert!(!multiple.source_document_samples_have_more);
    site.finish(requests).await;
}
