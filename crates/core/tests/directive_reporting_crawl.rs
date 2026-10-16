//! Genuine future owned-loopback regression; authored source only, NOT RUN here.
use open_seo_spider_core::{
    CrawlConfig, CrawlReport, CrawlStore, Crawler, DirectiveReportingConfig,
    directive_page_visible, export_pages_csv, reporting_json, reporting_visibility,
};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex, atomic::AtomicBool},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    task::JoinSet,
};

#[tokio::test]
async fn eight_profiles_reopen_saved_flags_and_preserve_actual_outlink_frontier() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let hits = Arc::new(Mutex::new(Vec::<String>::new()));
    let recorded = hits.clone();
    let (stop, mut stopped) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let mut connections = JoinSet::new();
        loop {
            tokio::select! {
                _=&mut stopped=>break,
                accepted=listener.accept()=>{
                    let (mut socket,_)=accepted.unwrap();let rows=recorded.clone();
                    connections.spawn(async move {
                        let mut request=Vec::new();let mut buffer=[0;1024];
                        while !request.windows(4).any(|v|v==b"\r\n\r\n") {
                            let n=socket.read(&mut buffer).await.unwrap();assert!(n>0);request.extend_from_slice(&buffer[..n]);assert!(request.len()<=8192);
                        }
                        let request=String::from_utf8(request).unwrap();let target=request.split_whitespace().nth(1).unwrap().to_owned();rows.lock().unwrap().push(target.clone());
                        let (head,body)=match target.as_str() {
                            "/"=>("", "<a href='/noindex'>N</a><a href='/canonical'>C</a><a href='/page2'>P</a><a href='/first'>First</a>"),
                            "/noindex"=>("<meta name='robots' content='noindex'>", "<a href='/from-noindex'>Out</a>"),
                            "/canonical"=>("<link rel='canonical' href='/canonical-target'>", "<a href='/from-canonical'>Out</a>"),
                            "/page2"=>("<link rel='prev' href='/first'>", "<a href='/from-prev'>Out</a>"),
                            "/first"=>("<link rel='next' href='/page2'>", ""),
                            _=>("", "Leaf"),
                        };
                        let body=format!("<html><head><title>Owned</title>{head}</head><body>{body}</body></html>");
                        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.unwrap();socket.shutdown().await.unwrap();
                    });
                }
            }
        }
        drop(listener);
        while let Some(joined) = connections.join_next().await {
            joined.unwrap();
        }
    });
    let temp = tempfile::tempdir().unwrap();
    let mut expected_requests = None;
    let mut expected_captured = None;
    for mask in 0_u8..8 {
        let from = hits.lock().unwrap().len();
        let profile = DirectiveReportingConfig {
            version: 1,
            respect_noindex: mask & 1 != 0,
            respect_canonical: mask & 2 != 0,
            respect_next_prev: mask & 4 != 0,
        };
        let config = CrawlConfig {
            seed_urls: vec![origin.clone() + "/"],
            discover_sitemaps: false,
            obey_robots: false,
            concurrency: 1,
            delay_ms: 0,
            max_pages: None,
            request_timeout_secs: 10,
            directive_reporting: Some(profile.clone()),
            ..Default::default()
        };
        let store_path = temp.path().join(format!("profile-{mask}.sqlite"));
        let report = Crawler::new(config)
            .unwrap()
            .with_store(&store_path)
            .unwrap()
            .run(None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        for path in ["/from-noindex", "/from-canonical", "/from-prev"] {
            assert!(
                report
                    .pages
                    .iter()
                    .any(|page| page.url == origin.clone() + path)
            );
        }
        assert!(directive_page_visible(
            &report.config,
            report
                .pages
                .iter()
                .find(|page| page.url == origin.clone() + "/first")
                .unwrap()
        ));
        let requests: BTreeSet<_> = hits.lock().unwrap()[from..].iter().cloned().collect();
        if let Some(expected) = &expected_requests {
            assert_eq!(&requests, expected);
        } else {
            expected_requests = Some(requests);
        }
        let captured: BTreeSet<_> = report.pages.iter().map(|page| page.url.clone()).collect();
        if let Some(expected) = &expected_captured {
            assert_eq!(&captured, expected);
        } else {
            expected_captured = Some(captured);
        }
        let before = serde_json::to_vec(&report).unwrap();
        let snapshot = temp.path().join(format!("profile-{mask}.json"));
        std::fs::write(&snapshot, &before).unwrap();
        let reopened: CrawlReport =
            serde_json::from_slice(&std::fs::read(&snapshot).unwrap()).unwrap();
        let raw_store = CrawlStore::open_read_only(&store_path).unwrap();
        assert_eq!(raw_store.pages().unwrap().len(), report.pages.len());
        let saved = raw_store.config().unwrap().unwrap();
        assert_eq!(
            saved
                .directive_reporting
                .as_ref()
                .is_some_and(|flags| flags.active()),
            mask != 0
        );
        let visible = reporting_json(&reopened).unwrap();
        let csv = export_pages_csv(&reopened).unwrap();
        assert_eq!(
            visible["pages"].as_array().unwrap().len(),
            report.pages.len() - mask.count_ones() as usize
        );
        if mask != 0 {
            let counts =
                reporting_visibility(&reopened.config, &reopened.pages, &reopened.issues).unwrap();
            assert_eq!(counts.hidden_pages, mask.count_ones() as usize);
            assert!(csv.lines().next().unwrap().contains("Captured Pages"));
            assert_eq!(
                visible["reporting_visibility"]["captured_pages"],
                report.pages.len()
            );
        }
        assert_eq!(
            serde_json::to_vec(&report).unwrap(),
            before,
            "Reporting must not rewrite raw observations, summary, relationship facts or frontier config"
        );
        drop(raw_store);
    }
    stop.send(()).unwrap();
    server.await.unwrap();
}
