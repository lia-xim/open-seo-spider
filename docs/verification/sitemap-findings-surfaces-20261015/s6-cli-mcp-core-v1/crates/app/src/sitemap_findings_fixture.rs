//! Test-only real public Crawler source; no fabricated page/context/session.
use open_seo_spider_core::{CrawlConfig, CrawlControl, CrawlOutcome, CrawlStore, Crawler};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

pub(super) struct OwnedDirectory(pub PathBuf);
impl OwnedDirectory {
    pub fn new() -> Self {
        let base = std::fs::canonicalize(std::env::temp_dir()).unwrap();
        let path = base.join(format!("oss-s6-cli-mcp-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        if self.0.parent().is_some_and(|p| {
            Some(p.to_path_buf()) == std::fs::canonicalize(std::env::temp_dir()).ok()
        }) && self
            .0
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("oss-s6-cli-mcp-"))
        {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
pub(super) struct Fixture {
    pub dir: OwnedDirectory,
    pub path: PathBuf,
    pub origin: String,
    hits: Arc<AtomicUsize>,
    requests: usize,
    stop: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}
impl Fixture {
    pub async fn actual(pages: usize) -> Self {
        assert!(pages > 1);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let o = origin.clone();
        let hits = Arc::new(AtomicUsize::new(0));
        let count = hits.clone();
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _=&mut stopped=>break,
                    accepted=listener.accept()=>{
                        let (mut socket,_)=accepted.unwrap();
                        let mut bytes=Vec::new(); let mut chunk=[0;2048];
                        tokio::time::timeout(Duration::from_secs(5),async {
                            while !bytes.windows(4).any(|w| w==b"\r\n\r\n") {
                                let n=socket.read(&mut chunk).await.unwrap(); assert!(n>0);
                                bytes.extend_from_slice(&chunk[..n]); assert!(bytes.len()<=65536);
                            }
                        }).await.unwrap();
                        count.fetch_add(1,Ordering::SeqCst);
                        let head=String::from_utf8(bytes).unwrap();
                        let route=head.lines().next().unwrap().split_whitespace().nth(1).unwrap();
                        let (mime,body)=if route=="/source.xml" {
                            ("application/xml",format!("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>{o}/p0000</loc></url></urlset>"))
                        } else { ("text/html","<html><head><title>Actual CLI MCP</title></head><body>Captured only</body></html>".into()) };
                        let response=format!("HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
                        socket.write_all(response.as_bytes()).await.unwrap(); socket.shutdown().await.unwrap();
                    }
                }
            }
        });
        let dir = OwnedDirectory::new();
        let path = dir.0.join("actual.sqlite");
        let config = CrawlConfig {
            seed_urls: (0..pages).map(|n| format!("{origin}/p{n:04}")).collect(),
            sitemap_urls: vec![format!("{origin}/source.xml")],
            obey_robots: false,
            discover_sitemaps: false,
            crawl_links: false,
            concurrency: 1,
            delay_ms: 0,
            ..Default::default()
        };
        let crawler = Crawler::new(config).unwrap().with_store(&path).unwrap();
        let actual = crawler
            .run_controlled(None, CrawlControl::new())
            .await
            .unwrap();
        assert_eq!(actual.outcome, CrawlOutcome::Completed);
        assert_eq!(actual.report.pages.len(), pages);
        drop(crawler);
        // Baseline the real readonly SQLite opener's allowed WAL-index startup.
        let reader = CrawlStore::open_read_only(&path).unwrap();
        reader.sitemap_read_snapshot().unwrap();
        drop(reader);
        let requests = hits.load(Ordering::SeqCst);
        Self {
            dir,
            path,
            origin,
            hits,
            requests,
            stop,
            task,
        }
    }
    pub async fn finish(self) {
        assert_eq!(
            self.hits.load(Ordering::SeqCst),
            self.requests,
            "Reads fetched website"
        );
        let _ = self.stop.send(());
        tokio::time::timeout(Duration::from_secs(10), self.task)
            .await
            .unwrap()
            .unwrap();
    }
}
