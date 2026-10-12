//! Independent native resource discovery: actual HTTP counters and durable rows.
use open_seo_spider_core::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{Semaphore, mpsc},
};
struct Response {
    status: u16,
    body: String,
    headers: Vec<(String, String)>,
    gate: Option<Arc<Semaphore>>,
}

fn config(fixture: &Fixture) -> CrawlConfig {
    CrawlConfig {
        seed_urls: vec![fixture.url("/")],
        obey_robots: false,
        discover_sitemaps: false,
        concurrency: 1,
        request_timeout_secs: 5,
        resource_discovery: Some(ResourceDiscoveryConfig::default()),
        ..Default::default()
    }
}
async fn run(config: CrawlConfig, path: &std::path::Path) -> ControlledCrawlResult {
    Crawler::new(config)
        .unwrap()
        .with_store(path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap()
}
fn ledger(path: &std::path::Path) -> Vec<StoredResourceOccurrence> {
    let mut rows = vec![];
    CrawlStore::open_read_only(path)
        .unwrap()
        .visit_resource_occurrences(2, |page| {
            rows.extend(page.rows.clone());
            Ok(())
        })
        .unwrap();
    rows
}

#[tokio::test]
async fn native_contexts_inventory_all_resource_families_and_do_not_confuse_response_mime() {
    let fixture=Fixture::new(|_,path|match path {
        "/"=>Response::html("<img src='/image'><script src='/js'></script><link rel='stylesheet' href='/css'><video src='/video' poster='/poster'><source src='/source'></video><audio src='/audio'></audio><object type='application/pdf' data='/pdf'></object><embed type='application/x-shockwave-flash' src='/swf'><track src='/other'><picture><source src='/ignored' srcset='/candidate 2x'></picture><a href='/nav'>go</a><link rel='canonical' href='/canonical'>"),
        "/css"=>Response::html("body{background:url('/css-image')}").header("Content-Type","Text/CSS; charset=UTF-8"),
        "/image"=>Response::html("<a href='/image-child'>no navigation permission</a>"),
        _=>Response::html("").header("Content-Type","application/octet-stream"),
    }).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("contexts.sqlite");
    let result = run(config(&fixture), &path).await;
    for target in [
        "/image",
        "/js",
        "/css",
        "/video",
        "/poster",
        "/source",
        "/audio",
        "/pdf",
        "/swf",
        "/other",
        "/candidate",
        "/nav",
        "/css-image",
    ] {
        assert_eq!(fixture.count(target), 1, "{target}");
    }
    for target in ["/ignored", "/canonical", "/image-child"] {
        assert_eq!(fixture.count(target), 0, "{target}");
    }
    let rows = ledger(&path);
    assert!(
        rows.iter().any(|row| row.occurrence.raw_value == "/ignored"
            && row.occurrence.observation_error.is_some())
    );
    let actual = result
        .report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/image"))
        .unwrap();
    assert!(actual.is_html());
    assert!(
        !actual
            .resource_permission
            .as_ref()
            .unwrap()
            .can_expand_navigation
    );
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn native_store_fetch_four_products_preserve_declared_graph_and_optional_report_inclusion() {
    for store in [false, true] {
        for fetch in [false, true] {
            let fixture = Fixture::new(|_, path| {
                if path == "/" {
                    Response::html("<img src='/image'><a href='/nav'>nav</a>")
                } else {
                    Response::html("image").header("Content-Type", "image/png")
                }
            })
            .await;
            let mut cfg = config(&fixture);
            cfg.resource_discovery.as_mut().unwrap().resources.images =
                ResourceSwitch { store, fetch };
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("product.sqlite");
            let result = run(cfg, &path).await;
            assert_eq!(fixture.count("/image"), usize::from(fetch));
            assert_eq!(fixture.count("/nav"), 1);
            assert_eq!(
                result
                    .report
                    .pages
                    .iter()
                    .any(|page| page.url.ends_with("/image")),
                store && fetch
            );
            let row = ledger(&path)
                .into_iter()
                .find(|row| row.occurrence.raw_value == "/image")
                .unwrap();
            assert_eq!(row.decision.grant.is_some(), fetch);
            assert_eq!(
                result
                    .report
                    .resource_ledger
                    .as_ref()
                    .unwrap()
                    .completed_transports,
                if fetch { 3 } else { 2 }
            );
            assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
        }
    }
}

#[tokio::test]
async fn native_late_navigation_reuses_one_http_and_preserves_original_retained_capture() {
    let fixture = Fixture::new(|_, path| match path {
        "/" => Response::html("<img src='/shared'><a href='/late'>late</a>"),
        "/shared" => Response::html(
            "<title>Original title</title><p>Original captured words</p><a href='/child'>child</a>",
        ),
        "/late" => Response::html("<a href='/shared'>independent role</a>"),
        _ => Response::html("<p>child</p>"),
    })
    .await;
    let mut cfg = config(&fixture);
    cfg.content_capture = Some(ContentCaptureConfig::default());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("late.sqlite");
    let result = run(cfg, &path).await;
    assert_eq!(fixture.count("/shared"), 1);
    assert_eq!(fixture.count("/child"), 1);
    let page = result
        .report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/shared"))
        .unwrap();
    assert_eq!(
        page.content_captures.as_ref().unwrap()[0].status,
        PageContentCaptureStatus::Complete
    );
    assert!(
        page.content_captures.as_ref().unwrap()[0]
            .result
            .as_ref()
            .unwrap()
            .text
            .as_deref()
            .unwrap()
            .contains("Original captured words")
    );
    let permission = page.resource_permission.as_ref().unwrap();
    assert!(permission.can_expand_navigation);
    assert!(!permission.authorizing_grants[0].can_expand_navigation);
    assert_eq!(permission.authorizing_grants.len(), 2);
    let child = ledger(&path)
        .into_iter()
        .filter(|row| row.occurrence.raw_value == "/child")
        .collect::<Vec<_>>();
    assert_eq!(child.len(), 2);
    assert_eq!(child[0].decision.reason, "source_check_only");
    assert!(child[1].decision.grant.is_some());
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn native_late_store_true_materializes_known_metadata_without_resurrecting_hidden_text() {
    let fixture=Fixture::new(|_,path|match path {"/"=>Response::html("<img src='/shared'><a href='/late'>late</a>"),
        "/shared"=>Response::html("<title>Observed metadata</title><p>private text never retained</p><a href='/child'>child</a>"),
        "/late"=>Response::html("<a href='/shared'>independent role</a>"),_=>Response::html("")}).await;
    let mut cfg = config(&fixture);
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .resources
        .images
        .store = false;
    cfg.content_capture = Some(ContentCaptureConfig::default());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("late-store.sqlite");
    let result = run(cfg, &path).await;
    assert_eq!(fixture.count("/shared"), 1);
    assert_eq!(fixture.count("/child"), 1);
    let page = result
        .report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/shared"))
        .unwrap();
    assert_eq!(page.title(), "Observed metadata");
    let row = &page.content_captures.as_ref().unwrap()[0];
    assert_eq!(row.status, PageContentCaptureStatus::Unavailable);
    assert_eq!(row.result.as_ref().unwrap().text, None);
    assert!(row.result.as_ref().unwrap().reasons.iter().any(|reason| {
        serde_json::to_string(reason)
            .unwrap()
            .contains("response_source_not_retained")
    }));
    let database = rusqlite::Connection::open(&path).unwrap();
    let saved: String = database
        .query_row(
            "SELECT data FROM resource_transports WHERE url=?1",
            [fixture.url("/shared")],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!saved.contains("private text never retained"));
    let store = CrawlStore::open_read_only(&path).unwrap();
    let transport = store
        .resource_transports(None, 200)
        .unwrap()
        .rows
        .into_iter()
        .find(|row| row.requested_url.ends_with("/shared"))
        .unwrap();
    assert!(transport.retained_response);
    assert!(
        !transport
            .projection
            .resource_response
            .as_ref()
            .unwrap()
            .retained_response
    );
    assert!(
        !transport
            .projection
            .resource_response
            .as_ref()
            .unwrap()
            .entity_body_retained
    );
    assert_eq!(transport.observed_generation, 1);
    assert_eq!(transport.projection.fetched_at, page.fetched_at);
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn native_list_navigation_off_keeps_resources_and_css_final_url_base() {
    let fixture=Fixture::new(|_,path|match path {
        "/"=>Response::html("<a href='/nav'>no navigation</a><link rel='stylesheet' href='/css-redirect'><img src='/image'>"),
        "/css-redirect"=>Response::html("").status(302).header("Location","/folder/style.css"),
        "/folder/style.css"=>Response::html("@import 'nested.css';body{background:url('\\69 mage.png')}").header("Content-Type","text/css"),
        "/folder/nested.css"=>Response::html("body{background:image-set('two.png' 2x)}").header("Content-Type","text/css"),
        _=>Response::html("").header("Content-Type","image/png"),
    }).await;
    let mut cfg = config(&fixture);
    cfg.crawl_links = false;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("css.sqlite");
    let result = run(cfg, &path).await;
    for target in [
        "/css-redirect",
        "/folder/style.css",
        "/folder/nested.css",
        "/folder/image.png",
        "/folder/two.png",
        "/image",
    ] {
        assert_eq!(fixture.count(target), 1, "{target}");
    }
    assert_eq!(fixture.count("/nav"), 0);
    assert_eq!(fixture.count("/image.png"), 0);
    assert!(result.report.resource_ledger.as_ref().unwrap().complete);
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn native_invalid_inline_css_does_not_poison_independent_html_or_css_units() {
    let fixture=Fixture::new(|_,path|if path=="/"{Response::html("<style>body{background:url('broken</style><style>body{background:url('/valid-css-image')}</style><a href='/nav'>nav</a><img src='/image'><img src='#local'><link rel='stylesheet' href=' #local'>")}
        else{Response::html("").header("Content-Type","application/octet-stream")}).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("separate.sqlite");
    let result = run(config(&fixture), &path).await;
    for target in ["/nav", "/image", "/valid-css-image"] {
        assert_eq!(fixture.count(target), 1, "{target}");
    }
    assert_eq!(fixture.count("/"), 1);
    assert!(!result.report.resource_ledger.as_ref().unwrap().complete);
    let rows = ledger(&path);
    assert_eq!(
        rows.iter()
            .filter(|row| row.occurrence.observation_error.as_deref() == Some("local_fragment"))
            .count(),
        2
    );
    assert!(
        rows.iter()
            .find(|row| row.occurrence.raw_value == "/nav")
            .unwrap()
            .occurrence
            .source_complete
    );
}

#[tokio::test]
async fn native_store_false_fetch_budget_above_500_resumes_without_refetching_committed_sources() {
    let fixture = Fixture::new(|_, path| {
        if path == "/" {
            Response::html(
                &(0..505)
                    .map(|index| format!("<img src='/i/{index}'>"))
                    .collect::<String>(),
            )
        } else {
            Response::html("").header("Content-Type", "image/png")
        }
    })
    .await;
    let mut cfg = config(&fixture);
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .resources
        .images
        .store = false;
    cfg.max_pages = Some(2);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("large.sqlite");
    let first = run(cfg.clone(), &path).await;
    assert_eq!(first.outcome, CrawlOutcome::Limited);
    assert_eq!(first.report.summary.completed, 1);
    assert_eq!(
        first
            .report
            .resource_ledger
            .as_ref()
            .unwrap()
            .completed_transports,
        2
    );
    assert_eq!(first.report.summary.pending, 504);
    cfg.max_pages = None;
    let resumed = run(cfg, &path).await;
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    assert_eq!(
        resumed
            .report
            .resource_ledger
            .as_ref()
            .unwrap()
            .completed_transports,
        506
    );
    assert_eq!(resumed.report.pages.len(), 1);
    assert_eq!(fixture.count("/"), 1);
    assert_eq!(fixture.count("/i/0"), 1);
    assert_eq!(ledger(&path).len(), 506);
    let store = CrawlStore::open_read_only(&path).unwrap();
    let page = store.resource_transports(None, 1000).unwrap();
    assert_eq!(page.rows.len(), 506);
    assert!(
        page.rows
            .iter()
            .filter(|row| !row.retained_response)
            .all(|row| row.projection.screenshot_base64.is_none())
    );
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn native_source_budget_rolls_back_all_decisions_and_leaves_the_source_pending() {
    let fixture = Fixture::new(|_, _| Response::html("<img src='/one'><img src='/two'>")).await;
    let mut cfg = config(&fixture);
    cfg.resource_discovery_limits = Some(ResourceDiscoveryLimits {
        max_source_candidates: 1,
        ..Default::default()
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("budget.sqlite");
    let error = Crawler::new(cfg)
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("budget"));
    let inspected = CrawlStore::inspect(&path).unwrap();
    assert_eq!(inspected.pending, 1);
    assert_eq!(inspected.completed, 0);
    assert_eq!(
        inspected
            .resource_ledger
            .as_ref()
            .unwrap()
            .completed_transports,
        0
    );
    assert_eq!(ledger(&path).len(), 1);
    assert_eq!(fixture.count("/one"), 0);
    assert_eq!(fixture.count("/two"), 0);
    assert!(!inspected.checkpoint_valid);
}

#[tokio::test]
async fn native_resource_scope_and_robots_preserve_original_credential_origins() {
    let external = Fixture::new(|_, path| {
        if path == "/robots.txt" {
            Response::html("User-agent: *\nDisallow: /blocked\n")
                .header("Content-Type", "text/plain")
        } else {
            Response::html("").header("Content-Type", "image/png")
        }
    })
    .await;
    let external_origin = external.origin.clone();
    let fixture=Fixture::new(move|_,path|if path=="/"{Response::html(&format!("<img src='{external_origin}/image'><img src='{external_origin}/blocked'><img src='/local'>"))}
        else if path=="/robots.txt"{Response::html("User-agent: *\nAllow: /\n").header("Content-Type","text/plain")}else{Response::html("").header("Content-Type","image/png")}).await;
    let mut cfg = config(&fixture);
    cfg.obey_robots = true;
    cfg.custom_headers
        .insert("Authorization".into(), "Bearer fixture-private".into());
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .resource_scope
        .external_origins
        .push(external.origin.clone());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("auth.sqlite");
    let result = run(cfg, &path).await;
    assert_eq!(external.count("/image"), 1);
    assert_eq!(external.count("/blocked"), 0);
    assert!(
        external
            .credentials
            .lock()
            .unwrap()
            .iter()
            .all(|(_, present)| !*present)
    );
    assert!(
        fixture
            .credentials
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _)| path != "/robots.txt")
            .all(|(_, present)| *present)
    );
    let blocked = result
        .report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/blocked"))
        .unwrap();
    assert!(blocked.blocked_by_robots);
    assert_eq!(
        blocked
            .resource_response
            .as_ref()
            .unwrap()
            .captured_entity_sha256,
        None
    );
    let empty = result
        .report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/image"))
        .unwrap();
    assert!(
        empty
            .resource_response
            .as_ref()
            .unwrap()
            .captured_entity_sha256
            .is_some()
    );
}

#[tokio::test]
async fn native_pause_drains_unretained_transport_and_selected_recrawl_never_expands() {
    let gate = Arc::new(Semaphore::new(0));
    let response_gate = gate.clone();
    let mut fixture = Fixture::new(move |_, path| {
        if path == "/" {
            Response::html("<img src='/image'><a href='/nav'>nav</a>")
        } else if path == "/image" {
            Response::html("")
                .header("Content-Type", "image/png")
                .gated(&response_gate)
        } else {
            Response::html("<img src='/new-outward'>")
        }
    })
    .await;
    let mut cfg = config(&fixture);
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .resources
        .images
        .store = false;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("pause.sqlite");
    let control = CrawlControl::new();
    let run_control = control.clone();
    let crawler = Crawler::new(cfg.clone())
        .unwrap()
        .with_store(&path)
        .unwrap();
    let running =
        tokio::spawn(async move { crawler.run_controlled(None, run_control).await.unwrap() });
    fixture.started("/image").await;
    assert!(control.request_pause());
    gate.add_permits(1);
    let paused = running.await.unwrap();
    assert_eq!(paused.outcome, CrawlOutcome::Paused);
    assert_eq!(
        paused
            .report
            .resource_ledger
            .as_ref()
            .unwrap()
            .completed_transports,
        2
    );
    assert_eq!(fixture.count("/nav"), 0);
    let parent_bytes = serde_json::to_vec(&paused.report).unwrap();
    let child = prepare_selected_recrawl(
        SelectedRecrawlParent {
            job_id: "parent".into(),
            checkpoint_revision: Some(1),
            source_kind: RecrawlSourceKind::CapturedCheckpoint,
        },
        &parent_bytes,
        &[fixture.url("/")],
        None,
    )
    .unwrap();
    let child_path = temp.path().join("child.sqlite");
    let selected = Crawler::new(child.config.clone())
        .unwrap()
        .with_store(&child_path)
        .unwrap()
        .with_selected_recrawl(child)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(fixture.count("/"), 2);
    assert_eq!(fixture.count("/image"), 1);
    assert_eq!(fixture.count("/nav"), 0);
    assert_eq!(selected.report.pages.len(), 1);
    assert!(
        !selected
            .report
            .resource_ledger
            .as_ref()
            .unwrap()
            .inherited_history
            .as_ref()
            .unwrap()
            .rows_available
    );
    assert!(
        ledger(&child_path)
            .iter()
            .all(|row| row.decision.grant.is_none())
    );
    assert_eq!(
        CrawlStore::open_read_only(&child_path)
            .unwrap()
            .parent_snapshot_bytes()
            .unwrap()
            .unwrap(),
        parent_bytes
    );
    assert!(CrawlStore::inspect(&child_path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn native_policy_aliases_and_independent_grants_preserve_both_ledgers_and_check_only_ceiling()
{
    let fixture=Fixture::new(|_,path|match path {
        "/shop/start"=>Response::html("<img src='/shop/alias'><a href='/shop/shared'>independent same task</a><img src='/outside'><a href='/outside'>still policy check-only</a>"),
        "/shop/shared"=>Response::html("<a href='/shop/child'>child</a>"),
        "/outside"=>Response::html("<a href='/shop/forbidden'>check-only source cannot expand</a>"),
        _=>Response::html("")}).await;
    let mut cfg = config(&fixture);
    cfg.seed_urls = vec![fixture.url("/shop/start")];
    cfg.crawl_policy = Some(CrawlPolicyConfig {
        scope: CrawlScopeConfig {
            roots: vec![CrawlScopeRoot {
                origin: fixture.origin.clone(),
                subdomains: false,
                folders: vec!["/shop/".into()],
                exact_paths: vec![],
            }],
            outside_folder: OutsideScopePolicy::CheckOnce,
            ..Default::default()
        },
        rewrites: vec![RewriteRule {
            id: "resource-alias".into(),
            pattern: "/shop/alias$".into(),
            replacement: RewriteReplacement::Literal {
                text: "/shop/shared".into(),
            },
        }],
        ..Default::default()
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("policy-resources.sqlite");
    let result = run(cfg, &path).await;
    assert_eq!(fixture.count("/shop/alias"), 0);
    assert_eq!(fixture.count("/shop/shared"), 1);
    assert_eq!(fixture.count("/shop/child"), 1);
    assert_eq!(fixture.count("/outside"), 1);
    assert_eq!(fixture.count("/shop/forbidden"), 0);
    let rows = ledger(&path);
    let alias = rows
        .iter()
        .find(|row| row.occurrence.raw_value == "/shop/alias")
        .unwrap();
    assert_eq!(
        alias.occurrence.resolved_url.as_deref(),
        Some(fixture.url("/shop/alias").as_str())
    );
    assert_eq!(
        alias.decision.grant.as_ref().unwrap().fetch_url,
        fixture.url("/shop/shared")
    );
    let shared = result
        .report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/shop/shared"))
        .unwrap();
    assert_eq!(
        shared
            .resource_permission
            .as_ref()
            .unwrap()
            .authorizing_grants
            .len(),
        2
    );
    let outside = result
        .report
        .pages
        .iter()
        .find(|page| page.url.ends_with("/outside"))
        .unwrap();
    assert!(
        !outside
            .resource_permission
            .as_ref()
            .unwrap()
            .can_expand_navigation
    );
    assert!(result.report.policy_ledger.is_some());
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn native_paged_snapshots_and_recovery_reject_tampered_rows_transports_and_forged_inherited_permissions()
 {
    for tamper in [
        "occurrence",
        "transport",
        "permission",
        "generation",
        "completeness",
    ] {
        let fixture = Fixture::new(|_, path| {
            if path == "/" {
                Response::html("<img src='/image'><style>body{background:url('broken</style>")
            } else {
                Response::html("").header("Content-Type", "image/png")
            }
        })
        .await;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("tamper.sqlite");
        let _ = run(config(&fixture), &path).await;
        let database = rusqlite::Connection::open(&path).unwrap();
        match tamper {
            "occurrence" => {
                database.execute("UPDATE resource_occurrences SET data=json_set(data,'$.occurrence.raw_value','forged') WHERE sequence=2",[]).unwrap();
            }
            "transport" => {
                database.execute("UPDATE resource_transports SET data=json_set(data,'$.projection.titles',json('[\"forged\"]')) WHERE url=?1",[fixture.url("/image")]).unwrap();
            }
            "generation" => {
                database.execute("UPDATE metadata SET value=json_set(value,'$.generation',0) WHERE key='run_state'",[]).unwrap();
            }
            "completeness" => {
                database.execute("UPDATE metadata SET value=json_set(value,'$.incomplete_sources',0) WHERE key='resource_ledger'",[]).unwrap();
            }
            _ => {
                database
                    .execute(
                        "UPDATE resource_tasks SET inherited=1 WHERE url=?1",
                        [fixture.url("/image")],
                    )
                    .unwrap();
            }
        }
        let store = CrawlStore::open_read_only(&path).unwrap();
        assert!(store.resource_occurrences(None, 200).is_err(), "{tamper}");
        assert!(store.resource_transports(None, 200).is_err(), "{tamper}");
        assert!(CrawlStore::inspect(&path).is_err(), "{tamper}");
        assert!(
            Crawler::new(config(&fixture))
                .unwrap()
                .with_store(&path)
                .unwrap()
                .run_controlled(None, CrawlControl::new())
                .await
                .is_err(),
            "{tamper}"
        );
        assert_eq!(fixture.count("/"), 1);
        assert_eq!(fixture.count("/image"), 1);
    }
}

#[tokio::test]
async fn native_total_ledger_budget_rolls_back_source_frontier_projection_and_dependent_admissions_atomically()
 {
    let fixture = Fixture::new(|_, path| {
        if path == "/" {
            Response::html("<img src='/one'>")
        } else {
            Response::html("")
        }
    })
    .await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ledger-budget.sqlite");
    let mut cfg = config(&fixture);
    cfg.resource_discovery_limits = Some(ResourceDiscoveryLimits {
        max_ledger_bytes: Some(1800),
        ..Default::default()
    });
    let error = Crawler::new(cfg)
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("budget"));
    let store = CrawlStore::open_read_only(&path).unwrap();
    let ledger = store.resource_ledger_summary().unwrap().unwrap();
    assert_eq!(ledger.completed_transports, 0);
    assert_eq!(store.pending_count().unwrap(), 1);
    assert_eq!(store.pages().unwrap().len(), 0);
    assert_eq!(fixture.count("/one"), 0);
    assert_eq!(store.resource_transports(None, 200).unwrap().rows.len(), 0);
    assert_eq!(store.resource_occurrences(None, 200).unwrap().rows.len(), 1);
}
#[tokio::test]
async fn native_streamed_resource_snapshot_stays_pinned_while_another_connection_changes_rows_and_generation()
 {
    let fixture = Fixture::new(|_, path| {
        if path == "/" {
            Response::html("<img src='/one'><img src='/two'><img src='/three'>")
        } else {
            Response::html("").header("Content-Type", "image/png")
        }
    })
    .await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("snapshot.sqlite");
    let result = run(config(&fixture), &path).await;
    let writer = rusqlite::Connection::open(&path).unwrap();
    let original: String = writer
        .query_row(
            "SELECT data FROM resource_occurrences WHERE sequence=4",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let run_state: String = writer
        .query_row(
            "SELECT value FROM metadata WHERE key='run_state'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let store = CrawlStore::open_read_only(&path).unwrap();
    let mut seen = Vec::new();
    let mut page_count = 0;
    let pinned = store.visit_resource_occurrences(1, |page| {
        page_count += 1;
        assert_eq!(page.snapshot.as_ref().unwrap().generation, Some(1));
        assert_eq!(page.ledger.as_ref().unwrap(), result.report.resource_ledger.as_ref().unwrap());
        if page_count == 1 {
            let tx = writer.unchecked_transaction().unwrap();
            tx.execute("UPDATE resource_occurrences SET data=json_set(data,'$.occurrence.raw_value','concurrent modification') WHERE sequence=4", []).unwrap();
            tx.execute("UPDATE metadata SET value=json_set(value,'$.generation',2) WHERE key='run_state'", []).unwrap();
            tx.commit().unwrap();
        }
        seen.extend(page.rows.clone());
        Ok(())
    }).unwrap();
    assert_eq!(pinned, result.report.resource_ledger);
    assert_eq!(page_count, 4);
    assert_eq!(seen.last().unwrap().occurrence.raw_value, "/three");
    // The next independent snapshot must see and reject the altered payload.
    assert!(store.resource_occurrences(None, 200).is_err());
    writer
        .execute(
            "UPDATE resource_occurrences SET data=?1 WHERE sequence=4",
            [&original],
        )
        .unwrap();
    writer
        .execute(
            "UPDATE metadata SET value=?1 WHERE key='run_state'",
            [&run_state],
        )
        .unwrap();
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
    assert_eq!(fixture.count("/"), 1);
}

#[tokio::test]
async fn native_selected_resource_reads_rebuild_inherited_permissions_from_immutable_parent_before_emitting()
 {
    use sha2::{Digest, Sha256};
    let fixture = Fixture::new(|_, path| {
        if path == "/" {
            Response::html("<img src='/image'>")
        } else {
            Response::html("<a href='/hidden'>no navigation grant</a>")
        }
    })
    .await;
    let temp = tempfile::tempdir().unwrap();
    let parent = run(config(&fixture), &temp.path().join("parent.sqlite")).await;
    let bytes = serde_json::to_vec(&parent.report).unwrap();
    let selected_url = fixture.url("/image");
    let prepared = prepare_selected_recrawl(
        SelectedRecrawlParent {
            job_id: "parent".into(),
            checkpoint_revision: Some(1),
            source_kind: RecrawlSourceKind::CapturedCheckpoint,
        },
        &bytes,
        std::slice::from_ref(&selected_url),
        None,
    )
    .unwrap();
    let path = temp.path().join("selected.sqlite");
    Crawler::new(prepared.config.clone())
        .unwrap()
        .with_store(&path)
        .unwrap()
        .with_selected_recrawl(prepared)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    let store = CrawlStore::open_read_only(&path).unwrap();
    assert!(store.resource_transports(None, 200).is_ok());
    let database = rusqlite::Connection::open(&path).unwrap();
    let original: String = database
        .query_row(
            "SELECT data FROM resource_tasks WHERE url=?1",
            [&selected_url],
            |row| row.get(0),
        )
        .unwrap();
    let mut task: serde_json::Value = serde_json::from_str(&original).unwrap();
    let mut permission: ResourceTaskPermission =
        serde_json::from_value(task["permission"].clone()).unwrap();
    assert!(!permission.can_expand_navigation);
    permission.can_expand_navigation = true;
    permission.authorizing_grants[0].can_expand_navigation = true;
    let revision = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&permission).unwrap())
    );
    task["permission"] = serde_json::to_value(&permission).unwrap();
    task["revision"] = revision.clone().into();
    let original_plan: String = database
        .query_row(
            "SELECT value FROM metadata WHERE key='run_plan'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut plan: RunPlan = serde_json::from_str(&original_plan).unwrap();
    let RunPlan::SelectedRecrawl(plan) = &mut plan else {
        panic!("selected plan missing")
    };
    plan.targets[0].resource_permission = Some(permission);
    // Keep the mutable permission projection and processed index coherent,
    // while leaving exact parent bytes and its reconstructed plan unchanged.
    let tx = database.unchecked_transaction().unwrap();
    tx.execute(
        "UPDATE resource_tasks SET data=?2 WHERE url=?1",
        rusqlite::params![selected_url, serde_json::to_string(&task).unwrap()],
    )
    .unwrap();
    tx.execute(
        "UPDATE resource_expansions SET processed_revision=?2 WHERE url=?1",
        rusqlite::params![selected_url, revision],
    )
    .unwrap();
    tx.execute(
        "UPDATE metadata SET value=?1 WHERE key='run_plan'",
        [serde_json::to_string(&RunPlan::SelectedRecrawl(plan.clone())).unwrap()],
    )
    .unwrap();
    tx.commit().unwrap();
    assert!(store.resource_occurrences(None, 200).is_err());
    assert!(store.resource_transports(None, 200).is_err());
    let mut emitted = false;
    assert!(
        store
            .visit_resource_occurrences(1, |_| {
                emitted = true;
                Ok(())
            })
            .is_err()
    );
    assert!(!emitted);
    assert!(CrawlStore::inspect(&path).is_err());
    assert_eq!(store.parent_snapshot_bytes().unwrap().unwrap(), bytes);
    assert_eq!(fixture.count("/image"), 2);
    assert_eq!(fixture.count("/hidden"), 0);
}

impl Response {
    fn html(body: &str) -> Self {
        Self {
            status: 200,
            body: body.into(),
            headers: vec![("Content-Type".into(), "text/html".into())],
            gate: None,
        }
    }
    fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }
    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers
            .retain(|(existing, _)| !existing.eq_ignore_ascii_case(name));
        self.headers.push((name.into(), value.into()));
        self
    }
    fn gated(mut self, gate: &Arc<Semaphore>) -> Self {
        self.gate = Some(gate.clone());
        self
    }
}
struct Fixture {
    origin: String,
    hits: Arc<Mutex<BTreeMap<String, usize>>>,
    credentials: Arc<Mutex<Vec<(String, bool)>>>,
    started: mpsc::UnboundedReceiver<String>,
    seen: BTreeSet<String>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Fixture {
    async fn new(handler: impl Fn(&str, &str) -> Response + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
        let credentials = Arc::new(Mutex::new(Vec::new()));
        let (sender, started) = mpsc::unbounded_channel();
        let handler = Arc::new(handler);
        let task_hits = hits.clone();
        let task_origin = origin.clone();
        let task_credentials = credentials.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let handler = handler.clone();
                let sender = sender.clone();
                let hits = task_hits.clone();
                let origin = task_origin.clone();
                let credentials = task_credentials.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let mut buffer = [0; 2048];
                    loop {
                        let Ok(length) = socket.read(&mut buffer).await else {
                            return;
                        };
                        if length == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buffer[..length]);
                        if bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                            break;
                        }
                        if bytes.len() > 65536 {
                            return;
                        }
                    }
                    let request = String::from_utf8_lossy(&bytes);
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/");
                    *hits.lock().unwrap().entry(path.into()).or_default() += 1;
                    credentials.lock().unwrap().push((
                        path.into(),
                        request
                            .lines()
                            .any(|line| line.to_ascii_lowercase().starts_with("authorization:")),
                    ));
                    let response = handler(&origin, path);
                    let _ = sender.send(path.into());
                    if let Some(gate) = response.gate {
                        gate.acquire().await.unwrap().forget();
                    }
                    let headers = response
                        .headers
                        .iter()
                        .map(|(name, value)| format!("{name}: {value}\r\n"))
                        .collect::<String>();
                    let head = format!(
                        "HTTP/1.1 {} Fixture\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
                        response.status,
                        response.body.len()
                    );
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(response.body.as_bytes()).await;
                });
            }
        });
        Self {
            origin,
            hits,
            credentials,
            started,
            seen: BTreeSet::new(),
            task,
        }
    }
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin)
    }
    fn count(&self, path: &str) -> usize {
        self.hits.lock().unwrap().get(path).copied().unwrap_or(0)
    }
    async fn started(&mut self, path: &str) {
        if self.seen.contains(path) {
            return;
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(actual) = self.started.recv().await {
                self.seen.insert(actual.clone());
                if actual == path {
                    return;
                }
            }
            panic!("fixture closed")
        })
        .await
        .unwrap();
    }
}
