use super::*;
use std::sync::{Condvar, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Release(Arc<(Mutex<bool>, Condvar)>);
impl Release {
    fn release(&self) {
        let (flag, changed) = &*self.0;
        *flag.lock().unwrap() = true;
        changed.notify_all();
    }
}
impl Drop for Release {
    fn drop(&mut self) {
        self.release();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancellation_then_pause_cannot_publish_checkpoint_before_started_pdf_worker_joins() {
    let mut pdf = lopdf::Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let page = pdf.add_object(lopdf::dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![0.into(),0.into(),612.into(),792.into()],
        "Resources" => lopdf::dictionary! {},
    });
    pdf.objects.insert(
        pages,
        lopdf::dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }.into(),
    );
    let root = pdf.add_object(lopdf::dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/owned.pdf", listener.local_addr().unwrap());
    let website = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        while !request.windows(4).any(|v| v == b"\r\n\r\n") {
            let n = socket.read(&mut buffer).await.unwrap();
            assert!(n > 0);
            request.extend_from_slice(&buffer[..n]);
        }
        let header = format!(
            "HTTP/1.1 200 Fixture\r\nContent-Type: application/pdf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        );
        socket.write_all(header.as_bytes()).await.unwrap();
        socket.write_all(&bytes).await.unwrap();
    });
    let release = Release(Arc::new((Mutex::new(false), Condvar::new())));
    let held = release.0.clone();
    let (entered, started) = tokio::sync::oneshot::channel();
    let entered = Mutex::new(Some(entered));
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("owned.sqlite");
    let mut crawler = Crawler::new(CrawlConfig {
        seed_urls: vec![url],
        obey_robots: false,
        discover_sitemaps: false,
        crawl_links: false,
        concurrency: 1,
        request_timeout_secs: 5,
        ..Default::default()
    })
    .unwrap()
    .with_store(&database)
    .unwrap();
    crawler.pdf_parse_hook = Some(Arc::new(move || {
        entered.lock().unwrap().take().unwrap().send(()).unwrap();
        let (flag, changed) = &*held;
        let mut released = flag.lock().unwrap();
        while !*released {
            released = changed.wait(released).unwrap();
        }
    }));
    let control = CrawlControl::new();
    let owned_control = control.clone();
    let mut run = tokio::spawn(async move { crawler.run_controlled(None, owned_control).await });
    tokio::time::timeout(Duration::from_secs(5), started)
        .await
        .unwrap()
        .unwrap();
    assert!(control.request_cancel());
    assert!(
        !control.request_pause(),
        "Shutdown cannot lower an existing cancellation"
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(200), &mut run)
            .await
            .is_err(),
        "An actual held parser must keep the whole run pending even after cancellation"
    );
    let before = CrawlStore::inspect(&database).unwrap();
    assert!(!before.checkpoint_valid);
    assert_eq!(before.completed, 0);
    assert_eq!(before.pending, 1);
    release.release();
    let result = tokio::time::timeout(Duration::from_secs(5), run)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(result.outcome, CrawlOutcome::Cancelled);
    assert_eq!(result.report.pages.len(), 0);
    let after = CrawlStore::inspect(&database).unwrap();
    assert!(after.checkpoint_valid);
    assert_eq!(after.completed, 0);
    assert_eq!(after.pending, 1);
    website.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_pdf_parser_panic_returns_typed_owned_failure_without_terminal_checkpoint() {
    let mut pdf = lopdf::Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let page = pdf.add_object(lopdf::dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![0.into(),0.into(),612.into(),792.into()],
        "Resources" => lopdf::dictionary! {},
    });
    pdf.objects.insert(
        pages,
        lopdf::dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }.into(),
    );
    let root = pdf.add_object(lopdf::dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/panic.pdf", listener.local_addr().unwrap());
    let website = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        while !request.windows(4).any(|v| v == b"\r\n\r\n") {
            let n = socket.read(&mut buffer).await.unwrap();
            assert!(n > 0);
            request.extend_from_slice(&buffer[..n]);
        }
        socket.write_all(format!("HTTP/1.1 200 Fixture\r\nContent-Type: application/pdf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).as_bytes()).await.unwrap();
        socket.write_all(&bytes).await.unwrap();
    });
    let temp = tempfile::tempdir().unwrap();
    let database = temp.path().join("panic.sqlite");
    let mut crawler = Crawler::new(CrawlConfig {
        seed_urls: vec![url],
        obey_robots: false,
        discover_sitemaps: false,
        crawl_links: false,
        concurrency: 1,
        request_timeout_secs: 5,
        ..Default::default()
    })
    .unwrap()
    .with_store(&database)
    .unwrap();
    let (entered, started) = tokio::sync::oneshot::channel();
    let entered = Mutex::new(Some(entered));
    crawler.pdf_parse_hook = Some(Arc::new(move || {
        entered.lock().unwrap().take().unwrap().send(()).unwrap();
        panic!("Actual owned PDF parser regression panic");
    }));
    let error = tokio::time::timeout(
        Duration::from_secs(5),
        crawler.run_controlled(None, CrawlControl::new()),
    )
    .await
    .unwrap()
    .unwrap_err();
    tokio::time::timeout(Duration::from_secs(5), started)
        .await
        .unwrap()
        .unwrap();
    assert!(
        error
            .chain()
            .any(|cause| cause.downcast_ref::<crate::OwnedTaskError>().is_some()),
        "Core must preserve the concrete owned join/drain failure for App propagation: {error:#}"
    );
    assert!(
        !CrawlStore::inspect(&database).unwrap().checkpoint_valid,
        "A failed actual PDF worker cannot publish a clean terminal checkpoint"
    );
    website.await.unwrap();
}
