//! AUTHORED_NOT_RUN: genuine local HTTP + durable store combination regressions.
//! A request-side observer reads the real started reservation before answering GET.
use anyhow::{Context, Result};
use lopdf::{Document, Stream, dictionary, text_string};
use open_seo_spider_core::*;
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{Semaphore, oneshot},
    task::JoinSet,
};

const KINDS: [&str; 7] = [
    "css",
    "javascript",
    "images",
    "media",
    "pdf",
    "swf",
    "other",
];

#[derive(Clone)]
struct Reply {
    status: u16,
    mime: String,
    extra: String,
    body: Vec<u8>,
    entered: Option<Arc<Semaphore>>,
    release: Option<Arc<Semaphore>>,
}
impl Reply {
    fn bytes(mime: &str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            mime: mime.into(),
            extra: String::new(),
            body: body.into(),
            entered: None,
            release: None,
        }
    }
    fn html(body: impl Into<Vec<u8>>) -> Self {
        Self::bytes("text/html; charset=utf-8", body)
    }
    fn redirect(target: &str) -> Self {
        let mut result = Self::html(Vec::new());
        result.status = 302;
        result.extra = format!("Location: {target}\r\n");
        result
    }
}

#[derive(Debug, Clone)]
struct Seen {
    target: String,
    raw: String,
    quota_state: Option<String>,
    memberships: usize,
}
struct Site {
    origin: String,
    replies: Arc<Mutex<BTreeMap<String, Reply>>>,
    seen: Arc<Mutex<Vec<Seen>>>,
    quota_database: Arc<Mutex<Option<PathBuf>>>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}
impl Site {
    async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let replies = Arc::new(Mutex::new(BTreeMap::<String, Reply>::new()));
        let seen = Arc::new(Mutex::new(Vec::<Seen>::new()));
        let quota_database = Arc::new(Mutex::new(None::<PathBuf>));
        let routes = replies.clone();
        let log = seen.clone();
        let database = quota_database.clone();
        let base = origin.clone();
        let (stop, mut stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut children = JoinSet::new();
            loop {
                tokio::select! {
                    _ = &mut stopped => break,
                    accepted = listener.accept() => {
                        let (mut socket, _) = accepted.unwrap();
                        let routes = routes.clone();
                        let log = log.clone();
                        let database = database.clone();
                        let base = base.clone();
                        children.spawn(async move {
                            let mut raw = Vec::new();
                            tokio::time::timeout(Duration::from_secs(3), async {
                                let mut chunk = [0u8; 1024];
                                while !raw.windows(4).any(|b| b == b"\r\n\r\n") {
                                    let count = socket.read(&mut chunk).await.unwrap();
                                    assert!(count > 0);
                                    raw.extend_from_slice(&chunk[..count]);
                                    assert!(raw.len() < 16_384);
                                }
                            }).await.unwrap();
                            let raw = String::from_utf8(raw).unwrap();
                            let mut first = raw.lines().next().unwrap().split_whitespace();
                            assert_eq!(first.next(), Some("GET"));
                            let target = first.next().unwrap().to_owned();
                            let owned_database = database.lock().unwrap().clone();
                            let (quota_state, memberships) = if let Some(owned_database) = owned_database {
                                let connection = rusqlite::Connection::open_with_flags(owned_database, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
                                let full_url = format!("{base}{target}");
                                let state: String = connection.query_row(
                                    "SELECT state FROM crawl_quota_attempts WHERE target=?1 AND started=1 ORDER BY id DESC LIMIT 1",
                                    [&full_url], |row| row.get(0),
                                ).expect("actual inbound GET has a committed started reservation");
                                let memberships: i64 = connection.query_row(
                                    "SELECT count(*) FROM crawl_quota_members WHERE target=?1", [&full_url], |row| row.get(0),
                                ).unwrap();
                                let memberships = usize::try_from(memberships).expect("membership count fits usize");
                                assert_eq!(state, "started");
                                assert!(memberships > 0);
                                (Some(state), memberships)
                            } else { (None, 0) };
                            {
                                let mut rows = log.lock().unwrap();
                                assert!(rows.len() < 256, "finite fixture request bound");
                                rows.push(Seen { target: target.clone(), raw, quota_state, memberships });
                            }
                            let reply = routes.lock().unwrap().get(&target).cloned().unwrap_or_else(|| {
                                let mut reply = Reply::html("Unexpected owned-fixture path");
                                reply.status = 404;
                                reply
                            });
                            if let Some(entered) = &reply.entered { entered.add_permits(1); }
                            if let Some(release) = &reply.release {
                                tokio::time::timeout(Duration::from_secs(10), release.acquire()).await.unwrap().unwrap().forget();
                            }
                            let head = format!("HTTP/1.1 {} Owned\r\nContent-Type: {}\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n", reply.status, reply.mime, reply.extra, reply.body.len());
                            // A cancellation test deliberately closes the client while held.
                            if socket.write_all(head.as_bytes()).await.is_ok() {
                                let _ = socket.write_all(&reply.body).await;
                                let _ = socket.shutdown().await;
                            }
                        });
                    }
                }
            }
            drop(listener);
            tokio::time::timeout(Duration::from_secs(12), async {
                while let Some(result) = children.join_next().await {
                    result.unwrap();
                }
            })
            .await
            .unwrap();
        });
        Self {
            origin,
            replies,
            seen,
            quota_database,
            stop: Some(stop),
            task: Some(task),
        }
    }
    fn put(&self, target: &str, reply: Reply) {
        self.replies.lock().unwrap().insert(target.into(), reply);
    }
    fn url(&self, target: &str) -> String {
        format!("{}{target}", self.origin)
    }
    fn observe_quota(&self, database: &Path) {
        *self.quota_database.lock().unwrap() = Some(database.into());
    }
    fn log(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
    async fn finish(mut self) -> Vec<Seen> {
        self.stop.take().unwrap().send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(15), self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap();
        self.log()
    }
}
impl Drop for Site {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
fn switch(config: &mut ResourceDiscoveryConfig, kind: &str, value: ResourceSwitch) {
    match kind {
        "css" => config.resources.css = value,
        "javascript" => config.resources.javascript = value,
        "images" => config.resources.images = value,
        "media" => config.resources.media = value,
        "pdf" => config.resources.pdf = value,
        "swf" => config.resources.swf = value,
        "other" => config.resources.other = value,
        _ => unreachable!(),
    }
}
fn config(site: &Site, target: &str) -> CrawlConfig {
    let discovery = ResourceDiscoveryConfig {
        contract: Some(ResourceContractConfig::default()),
        ..Default::default()
    };
    CrawlConfig {
        seed_urls: vec![site.url(target)],
        crawl_quotas: Some(CrawlQuotaConfig {
            version: 1,
            max_urls_per_depth: Some(100),
            max_urls_per_subdomain: Some(100),
            url_pattern_quotas: Vec::new(),
        }),
        resource_discovery: Some(discovery),
        discover_sitemaps: false,
        obey_robots: false,
        concurrency: 1,
        delay_ms: 0,
        request_timeout_secs: 5,
        custom_headers: BTreeMap::from([("X-Quota-Secret".into(), "owned-fixture-only".into())]),
        ..Default::default()
    }
}
async fn run(config: CrawlConfig, database: &Path) -> Result<ControlledCrawlResult> {
    tokio::time::timeout(
        Duration::from_secs(20),
        Crawler::new(config)?
            .with_store(database)?
            .run_controlled(None, CrawlControl::new()),
    )
    .await
    .context("bounded combined resource/quota crawl")?
}
fn requests(rows: &[Seen]) -> Vec<String> {
    let mut got = rows.iter().map(|r| r.target.clone()).collect::<Vec<_>>();
    got.sort();
    got
}
fn transports(store: &CrawlStore) -> Vec<ResourceTransportObservation> {
    let mut rows = Vec::new();
    let mut cursor = None;
    loop {
        let page = store.resource_transports(cursor.as_deref(), 2).unwrap();
        let more = page.has_more;
        cursor = page.next_url;
        rows.extend(page.rows);
        if !more {
            break;
        }
    }
    rows
}
fn occurrences(store: &CrawlStore) -> Vec<StoredResourceOccurrence> {
    let mut rows = Vec::new();
    store
        .visit_resource_occurrences(2, |page| {
            rows.extend(page.rows.clone());
            Ok(())
        })
        .unwrap();
    rows
}
fn increment(store: &CrawlStore, maxima: BTreeMap<String, u64>) -> CrawlQuotaIncreases {
    let summary = store.crawl_quota_summary().unwrap().unwrap();
    CrawlQuotaIncreases {
        expected_generation: store.checkpoint().unwrap().unwrap().generation,
        expected_profile_revision: summary.profile_revision,
        expected_profile_sha256: summary.profile_sha256,
        max_urls_per_depth: None,
        max_urls_per_subdomain: None,
        url_pattern_maxima: maxima,
    }
}
fn pattern(id: &str, regex: &str, max_urls: u64) -> CrawlUrlPatternQuota {
    CrawlUrlPatternQuota {
        id: id.into(),
        pattern: regex.into(),
        max_urls,
    }
}

fn declaration(kind: &str, url: &str) -> String {
    match kind {
        "css" => format!("<link rel='stylesheet' href='{url}'>"),
        "javascript" => format!("<script src='{url}'></script>"),
        "images" => format!("<img src='{url}' alt='Own matrix'>"),
        "media" => format!("<audio src='{url}'></audio>"),
        "pdf" => format!("<object type='application/pdf' data='{url}'></object>"),
        "swf" => format!("<embed type='application/x-shockwave-flash' src='{url}'>"),
        "other" => format!("<track src='{url}' kind='captions' srclang='en'>"),
        "internal_anchors" | "external_anchors" => format!("<a href='{url}'>Own link</a>"),
        "iframes" => format!("<iframe src='{url}'></iframe>"),
        "refresh" => format!("<meta http-equiv='refresh' content='0;url={url}'>"),
        "canonical" => format!("<link rel='canonical' href='{url}'>"),
        "hreflang" => format!("<link rel='alternate' hreflang='en' href='{url}'>"),
        "pagination" => format!("<link rel='next' href='{url}'>"),
        "amp" => format!("<link rel='amphtml' href='{url}'>"),
        "mobile_alternate" => format!(
            "<link rel='alternate' media='only screen and (max-width: 640px)' href='{url}'>"
        ),
        _ => unreachable!(),
    }
}

fn pdf(links: &[String]) -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let pages = doc.new_object_id();
    let font = doc.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica","Encoding"=>"WinAnsiEncoding"});
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf (Own complete matrix PDF) Tj ET".to_vec(),
    ));
    let annotations = links.iter().map(|url| dictionary! {"Subtype"=>"Link","Rect"=>vec![0.into(),0.into(),10.into(),10.into()],"A"=>dictionary! {"S"=>"URI","URI"=>text_string(url)}}.into()).collect::<Vec<lopdf::Object>>();
    let page = doc.add_object(dictionary! {"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()],"Contents"=>content,"Resources"=>dictionary! {"Font"=>dictionary! {"F1"=>font}},"Annots"=>annotations});
    doc.set_object(
        pages,
        dictionary! {"Type"=>"Pages","Kids"=>vec![page.into()],"Count"=>1},
    );
    let catalog = doc.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    doc.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}
fn png() -> Vec<u8> {
    fn chunk(bytes: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
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
fn wav() -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&38u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&8000u32.to_le_bytes());
    bytes.extend_from_slice(&16000u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&0i16.to_le_bytes());
    bytes
}

fn asset(kind: &str) -> Reply {
    match kind {
        "css" => Reply::bytes("text/css; charset=utf-8", "/* owned leaf stylesheet */"),
        "javascript" => Reply::bytes("text/javascript; charset=utf-8", "/* owned leaf script */"),
        "images" => Reply::bytes("image/png", png()),
        "media" => Reply::bytes("audio/wav", wav()),
        "pdf" => Reply::bytes("application/pdf", pdf(&[])),
        "swf" => Reply::bytes(
            "application/x-shockwave-flash",
            b"FWS\x09\x08\x00\x00\x00".to_vec(),
        ),
        "other" => Reply::bytes(
            "text/vtt; charset=utf-8",
            "WEBVTT\n\n00:00.000 --> 00:00.500\nOwned quota fixture\n",
        ),
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn native_resource_quotas_all_seven_kinds_keep_store_fetch_independent() -> Result<()> {
    let site = Site::new().await;
    let directory = tempfile::tempdir()?;
    let mut expected = Vec::new();
    for kind in KINDS {
        for store in [false, true] {
            for fetch in [false, true] {
                let prefix = format!("/{kind}-{}-{}", u8::from(store), u8::from(fetch));
                let source = format!("{prefix}/source");
                let target = format!("{prefix}/asset");
                site.put(&source, Reply::html(declaration(kind, &site.url(&target))));
                site.put(&target, asset(kind));
                let database = directory
                    .path()
                    .join(format!("{kind}-{store}-{fetch}.sqlite"));
                site.observe_quota(&database);
                let mut cfg = config(&site, &source);
                let resource = cfg.resource_discovery.as_mut().unwrap();
                for resource_kind in KINDS {
                    switch(
                        resource,
                        resource_kind,
                        ResourceSwitch {
                            store: false,
                            fetch: false,
                        },
                    );
                }
                switch(resource, kind, ResourceSwitch { store, fetch });
                let crawl = run(cfg, &database).await?;
                let cold = CrawlStore::open_read_only(&database)?;
                let quota = cold.crawl_quota_summary()?.unwrap();
                let resource = crawl.report.resource_ledger.as_ref().unwrap();
                let expected_targets = 1 + u64::from(fetch);
                assert_eq!(crawl.outcome, CrawlOutcome::Completed, "{prefix}");
                assert_eq!(
                    quota.reserved_distinct_targets, expected_targets,
                    "{prefix}"
                );
                assert_eq!(quota.actual_started_attempts, expected_targets, "{prefix}");
                assert_eq!(quota.active_attempts, 0, "{prefix}");
                assert_eq!(
                    resource.completed_transports, expected_targets as usize,
                    "{prefix}"
                );
                assert!(resource.complete, "{prefix}");
                assert_eq!(
                    crawl.report.pages.len(),
                    1 + usize::from(fetch && store),
                    "{prefix}"
                );
                let reference = occurrences(&cold)
                    .into_iter()
                    .find(|row| {
                        row.occurrence.requested_source_url == site.url(&source)
                            && row.occurrence.resolved_url.as_deref()
                                == Some(site.url(&target).as_str())
                    })
                    .unwrap();
                assert_eq!(
                    reference.decision.reason,
                    if fetch { "allowed" } else { "fetch_disabled" }
                );
                assert_eq!(
                    reference.decision.grant.as_ref().map(|grant| grant.store),
                    fetch.then_some(store)
                );
                let observations = transports(&cold);
                let target_facts = observations
                    .iter()
                    .find(|row| row.requested_url == site.url(&target));
                assert_eq!(target_facts.is_some(), fetch, "{prefix}");
                if let Some(target_facts) = target_facts {
                    assert_eq!(target_facts.retained_response, store, "{prefix}");
                    assert_eq!(target_facts.projection.status, Some(200), "{prefix}");
                }
                assert!(cold.inspection()?.checkpoint_valid, "{prefix}");
                expected.push(source);
                if fetch {
                    expected.push(target);
                }
            }
        }
    }
    let rows = site.finish().await;
    expected.sort();
    assert_eq!(requests(&rows), expected);
    assert!(
        rows.iter()
            .all(|row| row.quota_state.as_deref() == Some("started") && row.memberships > 0)
    );
    assert!(rows.iter().all(|row| {
        row.raw
            .to_ascii_lowercase()
            .contains("x-quota-secret: owned-fixture-only")
    }));
    Ok(())
}

#[tokio::test]
async fn native_resource_zero_pattern_defers_without_fake_transport_then_cold_increase_resumes()
-> Result<()> {
    let site = Site::new().await;
    site.put("/source", Reply::html("<img src='/asset' alt='owned'>"));
    site.put("/asset", asset("images"));
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("combined.sqlite");
    site.observe_quota(&database);
    let mut cfg = config(&site, "/source");
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .resources
        .images
        .store = false;
    cfg.crawl_quotas.as_mut().unwrap().url_pattern_quotas = vec![pattern("asset", "/asset$", 0)];
    let first = run(cfg, &database).await?;
    assert_eq!(first.outcome, CrawlOutcome::Limited);
    assert_eq!(first.report.pages.len(), 1);
    assert_eq!(first.report.summary.pending, 1);
    assert!(!first.report.resource_ledger.as_ref().unwrap().complete);
    assert_eq!(
        first
            .report
            .resource_ledger
            .as_ref()
            .unwrap()
            .pending_transports,
        1
    );
    assert_eq!(requests(&site.log()), vec!["/source"]);
    let cold = CrawlStore::open_read_only(&database)?;
    let before = cold.crawl_quota_summary()?.unwrap();
    assert_eq!(before.reserved_distinct_targets, 1);
    assert_eq!(before.deferred_targets, 1);
    assert_eq!(before.active_attempts, 0);
    assert!(before.causes.contains(&"pattern:asset".into()));
    assert!(before.bucket_rows.iter().all(|row| row.used == 1));
    assert_eq!(transports(&cold).len(), 1);
    let actual_reference = occurrences(&cold)
        .into_iter()
        .find(|row| row.occurrence.resolved_url.as_deref() == Some(site.url("/asset").as_str()))
        .unwrap();
    assert!(actual_reference.decision.grant.is_some());
    let increase = increment(&cold, BTreeMap::from([("asset".into(), 1)]));
    let generation = increase.expected_generation;
    let cfg = cold.preview_crawl_quota_increases(&increase)?;
    assert!(cold.inspection()?.checkpoint_valid);
    drop(cold);
    assert_eq!(requests(&site.log()), vec!["/source"]);
    let resumed = tokio::time::timeout(
        Duration::from_secs(20),
        Crawler::from_saved_config(cfg)?
            .with_store(&database)?
            .with_quota_increases(increase.clone())
            .run_controlled(None, CrawlControl::new()),
    )
    .await??;
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    assert_eq!(
        resumed.report.pages.len(),
        1,
        "fetch without store must not fabricate a retained page"
    );
    assert_eq!(
        serde_json::to_value(&resumed.report.pages[0])?,
        serde_json::to_value(&first.report.pages[0])?
    );
    let resource = resumed.report.resource_ledger.as_ref().unwrap();
    assert_eq!(resource.completed_transports, 2);
    assert_eq!(resource.retained_responses, 1);
    assert_eq!(resource.pending_transports, 0);
    assert!(resource.complete);
    let cold = CrawlStore::open_read_only(&database)?;
    let after = cold.crawl_quota_summary()?.unwrap();
    assert_eq!(cold.checkpoint()?.unwrap().generation, generation + 1);
    assert_eq!(after.profile_revision, before.profile_revision + 1);
    assert_eq!(after.reserved_distinct_targets, 2);
    assert_eq!(after.active_attempts, 0);
    assert_eq!(after.deferred_targets, 0);
    assert!(cold.preview_crawl_quota_increases(&increase).is_err());
    assert!(cold.inspection()?.checkpoint_valid);
    let target = transports(&cold)
        .into_iter()
        .find(|row| row.requested_url == site.url("/asset"))
        .unwrap();
    assert!(!target.retained_response);
    assert_eq!(target.projection.final_status, Some(200));
    drop(cold);
    assert_eq!(requests(&site.finish().await), vec!["/asset", "/source"]);
    Ok(())
}

#[tokio::test]
async fn native_resource_denial_is_atomic_across_pattern_depth_and_host_buckets() -> Result<()> {
    let site = Site::new().await;
    site.put(
        "/source",
        Reply::html("<img src='/blocked/asset'><img src='/free/asset'>"),
    );
    site.put("/blocked/asset", asset("images"));
    site.put("/free/asset", asset("images"));
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("atomic.sqlite");
    site.observe_quota(&database);
    let mut cfg = config(&site, "/source");
    cfg.crawl_quotas.as_mut().unwrap().url_pattern_quotas = vec![
        pattern("deny", "/blocked/", 0),
        pattern("overlap", "/blocked/", 1),
    ];
    let crawl = run(cfg, &database).await?;
    let cold = CrawlStore::open_read_only(&database)?;
    let quota = cold.crawl_quota_summary()?.unwrap();
    assert_eq!(crawl.outcome, CrawlOutcome::Limited);
    assert_eq!(quota.reserved_distinct_targets, 2);
    assert_eq!(quota.deferred_targets, 1);
    assert!(
        !quota
            .bucket_rows
            .iter()
            .any(|row| row.dimension == "pattern"),
        "denied overlap cannot spend the other pattern slot"
    );
    assert_eq!(
        quota
            .bucket_rows
            .iter()
            .find(|row| row.dimension == "host")
            .unwrap()
            .used,
        2
    );
    assert_eq!(
        quota
            .bucket_rows
            .iter()
            .find(|row| row.dimension == "depth" && row.key == "1")
            .unwrap()
            .used,
        1
    );
    assert_eq!(
        crawl
            .report
            .resource_ledger
            .as_ref()
            .unwrap()
            .pending_transports,
        1
    );
    assert!(cold.inspection()?.checkpoint_valid);
    drop(cold);
    assert_eq!(
        requests(&site.finish().await),
        vec!["/free/asset", "/source"]
    );
    Ok(())
}

#[tokio::test]
async fn native_resource_concurrent_candidates_cannot_overspend_one_depth_slot() -> Result<()> {
    let site = Site::new().await;
    site.put(
        "/source",
        Reply::html(
            (0..6)
                .map(|n| format!("<img src='/asset/{n}'>"))
                .collect::<String>(),
        ),
    );
    for n in 0..6 {
        site.put(&format!("/asset/{n}"), asset("images"));
    }
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("race.sqlite");
    site.observe_quota(&database);
    let mut cfg = config(&site, "/source");
    cfg.concurrency = 6;
    cfg.crawl_quotas.as_mut().unwrap().max_urls_per_depth = Some(1);
    let crawl = run(cfg, &database).await?;
    let cold = CrawlStore::open_read_only(&database)?;
    let quota = cold.crawl_quota_summary()?.unwrap();
    assert_eq!(crawl.outcome, CrawlOutcome::Limited);
    assert_eq!(quota.reserved_distinct_targets, 2);
    assert_eq!(quota.actual_started_attempts, 2);
    assert_eq!(quota.active_attempts, 0);
    assert_eq!(quota.deferred_targets, 5);
    assert_eq!(
        quota
            .bucket_rows
            .iter()
            .find(|row| row.dimension == "depth" && row.key == "1")
            .unwrap()
            .used,
        1
    );
    assert_eq!(
        crawl
            .report
            .resource_ledger
            .as_ref()
            .unwrap()
            .pending_transports,
        5
    );
    assert!(!crawl.report.resource_ledger.as_ref().unwrap().complete);
    assert!(cold.inspection()?.checkpoint_valid);
    drop(cold);
    let rows = site.finish().await;
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter()
            .filter(|row| row.target.starts_with("/asset/"))
            .count(),
        1
    );
    Ok(())
}

#[tokio::test]
async fn native_resource_redirect_deferred_records_real302_without_sending_denied_hop() -> Result<()>
{
    let site = Site::new().await;
    site.put("/source", Reply::html("<img src='/jump'>"));
    site.put("/jump", Reply::redirect("/blocked"));
    site.put("/blocked", asset("images"));
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("redirect.sqlite");
    site.observe_quota(&database);
    let mut cfg = config(&site, "/source");
    cfg.crawl_quotas.as_mut().unwrap().url_pattern_quotas =
        vec![pattern("blocked", "/blocked$", 0)];
    let crawl = run(cfg, &database).await?;
    assert_eq!(crawl.outcome, CrawlOutcome::Limited);
    assert_eq!(crawl.report.pages.len(), 1);
    assert_eq!(
        crawl
            .report
            .resource_ledger
            .as_ref()
            .unwrap()
            .pending_transports,
        1
    );
    assert!(!crawl.report.resource_ledger.as_ref().unwrap().complete);
    let connection = rusqlite::Connection::open_with_flags(
        &database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let status: Option<u16> = connection.query_row(
        "SELECT response_status FROM crawl_quota_attempts WHERE target=?1",
        [site.url("/jump")],
        |row| row.get(0),
    )?;
    assert_eq!(status, Some(302));
    let denied_attempts: i64 = connection.query_row(
        "SELECT count(*) FROM crawl_quota_attempts WHERE target=?1",
        [site.url("/blocked")],
        |row| row.get(0),
    )?;
    assert_eq!(denied_attempts, 0);
    drop(connection);
    let cold = CrawlStore::open_read_only(&database)?;
    assert_eq!(
        transports(&cold).len(),
        1,
        "a quota-deferred redirect task has no fabricated complete resource transport"
    );
    assert!(cold.inspection()?.checkpoint_valid);
    drop(cold);
    assert_eq!(requests(&site.finish().await), vec!["/jump", "/source"]);
    Ok(())
}

#[tokio::test]
async fn native_resource_css_channels_and_late_roles_reuse_paid_targets_without_refetch()
-> Result<()> {
    let site = Site::new().await;
    site.put("/source", Reply::html("<link rel='stylesheet' href='/sheet'><style>.a{background:url('/inline')}</style><div style=\"background:url('/attribute')\"></div><img src='/shared'>"));
    site.put(
        "/sheet",
        Reply::bytes(
            "text/css",
            "@import '/nested'; .b{background:url('/shared')}",
        ),
    );
    site.put("/nested", Reply::bytes("text/css", "/* leaf */"));
    for target in ["/inline", "/attribute", "/shared"] {
        site.put(target, asset("images"));
    }
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("css.sqlite");
    site.observe_quota(&database);
    let crawl = run(config(&site, "/source"), &database).await?;
    assert_eq!(crawl.outcome, CrawlOutcome::Completed);
    let cold = CrawlStore::open_read_only(&database)?;
    let quota = cold.crawl_quota_summary()?.unwrap();
    assert_eq!(quota.reserved_distinct_targets, 6);
    assert_eq!(quota.actual_started_attempts, 6);
    assert_eq!(transports(&cold).len(), 6);
    assert!(
        occurrences(&cold)
            .iter()
            .filter(
                |row| row.occurrence.resolved_url.as_deref() == Some(site.url("/shared").as_str())
            )
            .count()
            >= 2
    );
    assert!(crawl.report.resource_ledger.as_ref().unwrap().complete);
    assert!(cold.inspection()?.checkpoint_valid);
    drop(cold);
    assert_eq!(
        requests(&site.finish().await),
        vec![
            "/attribute",
            "/inline",
            "/nested",
            "/shared",
            "/sheet",
            "/source"
        ]
    );
    Ok(())
}

#[tokio::test]
async fn native_resource_quota_scope_refuses_unsupported_profiles_before_store_or_http()
-> Result<()> {
    let site = Site::new().await;
    site.put("/source", Reply::html("<img src='/asset'>"));
    let directory = tempfile::tempdir()?;
    for case in 0..6 {
        let mut cfg = config(&site, "/source");
        let expected = match case {
            0 => {
                cfg.resource_discovery.as_mut().unwrap().contract = None;
                "resource-contract-v2"
            }
            1 => {
                cfg.include_subdomains = true;
                "include_subdomains"
            }
            2 => {
                cfg.check_external = true;
                "check_external"
            }
            3 => {
                cfg.resource_discovery
                    .as_mut()
                    .unwrap()
                    .resource_scope
                    .external_origins
                    .push(site.origin.clone());
                "external resource origins"
            }
            4 => {
                cfg.renderer = Some(RendererConfig::default());
                "renderer"
            }
            _ => {
                cfg.resource_discovery
                    .as_mut()
                    .unwrap()
                    .contract
                    .as_mut()
                    .unwrap()
                    .engine = "unknown".into();
                "resource-contract-v2"
            }
        };
        let error = Crawler::new(cfg)
            .err()
            .context("unsupported combined profile was accepted")?;
        assert!(error.to_string().contains(expected), "case {case}: {error}");
    }
    assert_eq!(std::fs::read_dir(directory.path())?.count(), 0);
    assert!(site.finish().await.is_empty());
    Ok(())
}

#[tokio::test]
async fn no_quota_resource_profiles_keep_external_authority_and_origin_bound_credentials()
-> Result<()> {
    for strict in [false, true] {
        let primary = Site::new().await;
        let external = Site::new().await;
        primary.put(
            "/source",
            Reply::html(format!("<img src='{}'>", external.url("/asset"))),
        );
        external.put("/asset", asset("images"));
        let directory = tempfile::tempdir()?;
        let database = directory.path().join("legacy.sqlite");
        let mut cfg = config(&primary, "/source");
        cfg.crawl_quotas = None;
        let resource = cfg.resource_discovery.as_mut().unwrap();
        if !strict {
            resource.contract = None;
        }
        resource
            .resource_scope
            .external_origins
            .push(external.origin.clone());
        let before = serde_json::to_vec(&cfg)?;
        cfg.validate_crawl_quotas()?;
        assert_eq!(serde_json::to_vec(&cfg)?, before);
        let crawl = run(cfg, &database).await?;
        assert_eq!(crawl.outcome, CrawlOutcome::Completed);
        assert!(crawl.report.resource_ledger.as_ref().unwrap().complete);
        let cold = CrawlStore::open_read_only(&database)?;
        assert!(
            cold.crawl_quota_summary()?.is_none(),
            "no invented legacy zero usage"
        );
        assert!(cold.config()?.unwrap().crawl_quotas.is_none());
        assert!(cold.inspection()?.checkpoint_valid);
        drop(cold);
        let primary_rows = primary.finish().await;
        let external_rows = external.finish().await;
        assert_eq!(requests(&primary_rows), vec!["/source"]);
        assert_eq!(requests(&external_rows), vec!["/asset"]);
        assert!(
            primary_rows[0]
                .raw
                .to_ascii_lowercase()
                .contains("x-quota-secret: owned-fixture-only")
        );
        assert!(
            !external_rows[0]
                .raw
                .to_ascii_lowercase()
                .contains("x-quota-secret"),
            "external resource authority cannot grant credential authority"
        );
        assert!(
            primary_rows
                .iter()
                .chain(&external_rows)
                .all(|row| row.quota_state.is_none())
        );
    }
    Ok(())
}

#[tokio::test]
async fn native_resource_quota_rejects_cross_scope_redirect_without_sending_credentials()
-> Result<()> {
    let primary = Site::new().await;
    let denied = Site::new().await;
    primary.put("/source", Reply::html("<img src='/jump'>"));
    primary.put("/jump", Reply::redirect(&denied.url("/must-not-send")));
    denied.put("/must-not-send", asset("images"));
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("scope.sqlite");
    primary.observe_quota(&database);
    let crawl = run(config(&primary, "/source"), &database).await?;
    let cold = CrawlStore::open_read_only(&database)?;
    let quota = cold.crawl_quota_summary()?.unwrap();
    assert_eq!(
        quota.reserved_distinct_targets, 2,
        "denied next-hop must not reserve a resource-scoped target"
    );
    assert_eq!(quota.active_attempts, 0);
    let jump = transports(&cold)
        .into_iter()
        .find(|row| row.requested_url == primary.url("/jump"))
        .unwrap();
    let decisions = &jump
        .projection
        .resource_response
        .as_ref()
        .unwrap()
        .redirect_decisions;
    assert!(decisions.iter().any(|hop| !hop.allowed
        && hop.target_url.as_deref() == Some(denied.url("/must-not-send").as_str())));
    assert!(cold.inspection()?.checkpoint_valid);
    drop(cold);
    let source_url = primary.url("/source");
    let jump_url = primary.url("/jump");
    let primary_rows = primary.finish().await;
    assert_eq!(requests(&primary_rows), vec!["/jump", "/source"]);
    assert!(primary_rows.iter().all(|row| {
        row.raw
            .to_ascii_lowercase()
            .contains("x-quota-secret: owned-fixture-only")
    }));
    let denied_url = denied.url("/must-not-send");
    assert!(denied.finish().await.is_empty());
    // final_url retains the observed Location even when immutable scope denies its GET.
    // The established redirect report contract distinguishes observation from transport.
    assert_eq!(
        crawl
            .report
            .pages
            .iter()
            .filter(|page| page.url == jump_url)
            .count(),
        1
    );
    let jump_page = crawl
        .report
        .pages
        .iter()
        .find(|page| page.url == jump_url)
        .context("retained redirect source page")?;
    assert_eq!(jump_page.status, Some(302));
    assert_eq!(jump_page.final_status, None);
    assert_eq!(jump_page.final_url, denied_url);
    assert_eq!(jump_page.redirect_chain.len(), 1);
    let hop = &jump_page.redirect_chain[0];
    assert_eq!(hop.url, jump_url);
    assert_eq!(hop.status, 302);
    assert_eq!(hop.target, denied_url);
    assert!(jump_page.warnings.iter().any(|warning| warning
        == "Redirect target denied by immutable resource role/scope; no target request was made"));
    assert!(crawl.report.pages.iter().all(|page| page.url != denied_url));
    assert!(
        crawl
            .report
            .pages
            .iter()
            .all(|page| page.url == source_url || page.url == jump_url)
    );
    assert_eq!(
        crawl
            .report
            .pages
            .iter()
            .filter(|page| page.url == source_url)
            .count(),
        1
    );
    let source_page = crawl
        .report
        .pages
        .iter()
        .find(|page| page.url == source_url)
        .context("retained original source page")?;
    assert_eq!(source_page.final_url, source_url);
    assert_eq!(source_page.status, Some(200));
    assert_eq!(source_page.final_status, Some(200));
    assert!(source_page.redirect_chain.is_empty());
    Ok(())
}

#[tokio::test]
async fn native_resource_quota_full_checkpoint_rejects_count_preserving_negative_copies()
-> Result<()> {
    let site = Site::new().await;
    site.put("/source", Reply::html("<img src='/asset'>"));
    site.put("/asset", asset("images"));
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("genuine.sqlite");
    site.observe_quota(&database);
    let crawl = run(config(&site, "/source"), &database).await?;
    assert_eq!(crawl.outcome, CrawlOutcome::Completed);
    let cold = CrawlStore::open_read_only(&database)?;
    let genuine_summary = cold.crawl_quota_summary()?.unwrap();
    assert!(cold.inspection()?.checkpoint_valid);
    drop(cold);
    let genuine_primary = std::fs::read(&database)?;
    for case in ["quota", "resource"] {
        let negative = directory.path().join(format!("negative-{case}.sqlite"));
        std::fs::copy(&database, &negative)?;
        let connection = rusqlite::Connection::open(&negative)?;
        let table = if case == "quota" {
            "crawl_quota_attempts"
        } else {
            "resource_transports"
        };
        let count_before: i64 =
            connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })?;
        let changes = if case == "quota" {
            connection.execute(
                "UPDATE crawl_quota_attempts SET response_status=201 WHERE target=?1",
                [site.url("/asset")],
            )?
        } else {
            connection.execute(
                "UPDATE resource_transports SET payload_sha256=?1 WHERE url=?2",
                ["0".repeat(64), site.url("/asset")],
            )?
        };
        assert_eq!(changes, 1);
        let count_after: i64 =
            connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })?;
        assert_eq!(count_before, count_after);
        drop(connection);
        assert!(
            CrawlStore::inspect(&negative).is_err(),
            "count-preserving {case} mutation accepted"
        );
    }
    assert_eq!(
        std::fs::read(&database)?,
        genuine_primary,
        "only negative copies were mutated"
    );
    let cold = CrawlStore::open_read_only(&database)?;
    assert_eq!(cold.crawl_quota_summary()?, Some(genuine_summary));
    assert!(cold.inspection()?.checkpoint_valid);
    drop(cold);
    assert_eq!(requests(&site.finish().await), vec!["/asset", "/source"]);
    Ok(())
}

#[tokio::test]
async fn native_resource_quota_pause_drains_held_transport_then_reopens_both_ledgers() -> Result<()>
{
    let site = Site::new().await;
    site.put("/source", Reply::html("<img src='/hold'><img src='/tail'>"));
    let entered = Arc::new(Semaphore::new(0));
    let release = Arc::new(Semaphore::new(0));
    let mut held = asset("images");
    held.entered = Some(entered.clone());
    held.release = Some(release.clone());
    site.put("/hold", held);
    site.put("/tail", asset("images"));
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("pause.sqlite");
    site.observe_quota(&database);
    let crawler = Crawler::new(config(&site, "/source"))?.with_store(&database)?;
    let control = CrawlControl::new();
    let run_control = control.clone();
    let task = tokio::spawn(async move { crawler.run_controlled(None, run_control).await });
    tokio::time::timeout(Duration::from_secs(5), entered.acquire())
        .await??
        .forget();
    assert!(control.request_pause());
    release.add_permits(1);
    let first = tokio::time::timeout(Duration::from_secs(10), task).await???;
    assert_eq!(first.outcome, CrawlOutcome::Paused);
    assert_eq!(requests(&site.log()), vec!["/hold", "/source"]);
    assert_eq!(
        first
            .report
            .resource_ledger
            .as_ref()
            .unwrap()
            .pending_transports,
        1
    );
    assert!(!first.report.resource_ledger.as_ref().unwrap().complete);
    let cold = CrawlStore::open_read_only(&database)?;
    let before = cold.crawl_quota_summary()?.unwrap();
    assert_eq!(before.active_attempts, 0);
    assert_eq!(before.reserved_distinct_targets, 2);
    assert!(cold.inspection()?.checkpoint_valid);
    let cfg = cold.config()?.unwrap();
    drop(cold);
    let resumed = tokio::time::timeout(
        Duration::from_secs(20),
        Crawler::from_saved_config(cfg)?
            .with_store(&database)?
            .run_controlled(None, CrawlControl::new()),
    )
    .await??;
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    assert!(resumed.report.resource_ledger.as_ref().unwrap().complete);
    let cold = CrawlStore::open_read_only(&database)?;
    let after = cold.crawl_quota_summary()?.unwrap();
    assert_eq!(after.profile_revision, before.profile_revision);
    assert_eq!(after.reserved_distinct_targets, 3);
    assert_eq!(after.active_attempts, 0);
    assert!(cold.inspection()?.checkpoint_valid);
    drop(cold);
    assert_eq!(
        requests(&site.finish().await),
        vec!["/hold", "/source", "/tail"]
    );
    Ok(())
}

#[tokio::test]
async fn native_resource_quota_cancel_keeps_unknown_paid_target_and_cold_retry_does_not_repay()
-> Result<()> {
    let site = Site::new().await;
    site.put("/source", Reply::html("<img src='/hold'>"));
    let entered = Arc::new(Semaphore::new(0));
    let release = Arc::new(Semaphore::new(0));
    let mut held = asset("images");
    held.entered = Some(entered.clone());
    held.release = Some(release.clone());
    site.put("/hold", held);
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("cancel.sqlite");
    site.observe_quota(&database);
    let mut cfg = config(&site, "/source");
    cfg.crawl_quotas.as_mut().unwrap().max_urls_per_depth = Some(1);
    cfg.crawl_quotas.as_mut().unwrap().max_urls_per_subdomain = Some(2);
    let crawler = Crawler::new(cfg)?.with_store(&database)?;
    let control = CrawlControl::new();
    let run_control = control.clone();
    let task = tokio::spawn(async move { crawler.run_controlled(None, run_control).await });
    tokio::time::timeout(Duration::from_secs(5), entered.acquire())
        .await??
        .forget();
    assert!(control.request_cancel());
    let first = tokio::time::timeout(Duration::from_secs(10), task).await???;
    release.add_permits(1);
    assert_eq!(first.outcome, CrawlOutcome::Cancelled);
    assert!(!first.report.resource_ledger.as_ref().unwrap().complete);
    let cold = CrawlStore::open_read_only(&database)?;
    let before = cold.crawl_quota_summary()?.unwrap();
    assert_eq!(before.reserved_distinct_targets, 2);
    assert_eq!(before.interrupted_or_unknown_attempts, 1);
    assert_eq!(before.active_attempts, 0);
    assert!(cold.inspection()?.checkpoint_valid);
    let cfg = cold.config()?.unwrap();
    drop(cold);
    site.put("/hold", asset("images"));
    let resumed = tokio::time::timeout(
        Duration::from_secs(20),
        Crawler::from_saved_config(cfg)?
            .with_store(&database)?
            .run_controlled(None, CrawlControl::new()),
    )
    .await??;
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    assert!(resumed.report.resource_ledger.as_ref().unwrap().complete);
    let cold = CrawlStore::open_read_only(&database)?;
    let after = cold.crawl_quota_summary()?.unwrap();
    assert_eq!(after.reserved_distinct_targets, 2);
    assert_eq!(after.actual_started_attempts, 3);
    assert_eq!(after.interrupted_or_unknown_attempts, 1);
    assert_eq!(after.active_attempts, 0);
    assert!(cold.inspection()?.checkpoint_valid);
    drop(cold);
    assert_eq!(
        requests(&site.finish().await),
        vec!["/hold", "/hold", "/source"]
    );
    Ok(())
}

#[tokio::test]
async fn native_resource_reservation_sql_failure_rolls_back_before_actual_asset_get() -> Result<()>
{
    let site = Site::new().await;
    site.put("/source", Reply::html("<img src='/asset'>"));
    site.put("/asset", asset("images"));
    let directory = tempfile::tempdir()?;
    let database = directory.path().join("fault.sqlite");
    site.observe_quota(&database);
    let mut cfg = config(&site, "/source");
    cfg.crawl_quotas.as_mut().unwrap().url_pattern_quotas = vec![pattern("asset", "/asset$", 0)];
    let first = run(cfg, &database).await?;
    assert_eq!(first.outcome, CrawlOutcome::Limited);
    let cold = CrawlStore::open_read_only(&database)?;
    let increase = increment(&cold, BTreeMap::from([("asset".into(), 1)]));
    let cfg = cold.preview_crawl_quota_increases(&increase)?;
    drop(cold);
    let connection = rusqlite::Connection::open(&database)?;
    connection.execute_batch("CREATE TRIGGER owned_resource_reservation_fault BEFORE INSERT ON crawl_quota_members WHEN NEW.target LIKE '%/asset' BEGIN SELECT RAISE(ABORT,'owned resource reservation fault'); END;")?;
    drop(connection);
    let result = tokio::time::timeout(
        Duration::from_secs(20),
        Crawler::from_saved_config(cfg)?
            .with_store(&database)?
            .with_quota_increases(increase)
            .run_controlled(None, CrawlControl::new()),
    )
    .await?;
    let error = result
        .err()
        .context("injected reservation failure was accepted")?;
    assert!(
        error
            .to_string()
            .contains("owned resource reservation fault"),
        "unexpected refusal: {error}"
    );
    let connection = rusqlite::Connection::open_with_flags(
        &database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    for table in ["crawl_quota_attempts", "crawl_quota_members"] {
        let rows: i64 = connection.query_row(
            &format!("SELECT count(*) FROM {table} WHERE target=?1"),
            [site.url("/asset")],
            |row| row.get(0),
        )?;
        assert_eq!(
            rows, 0,
            "failed reservation transaction must roll back {table}"
        );
    }
    drop(connection);
    assert_eq!(requests(&site.finish().await), vec!["/source"]);
    Ok(())
}
