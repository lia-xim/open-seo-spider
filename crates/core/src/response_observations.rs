//! Every retained native response, including intermediary redirect hops.
//! These are response observations; they do not manufacture document records.
use crate::CrawlReport;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub struct ResponseObservation {
    pub requested_url: String,
    pub response_url: String,
    pub response_index: usize,
    pub status: u16,
    pub location: Option<String>,
    pub capture_kind: &'static str,
    pub headers: Option<BTreeMap<String, String>>,
    pub content_type: Option<String>,
    pub body_bytes: Option<usize>,
    pub body_complete: Option<bool>,
    pub document_metadata_available: bool,
}

pub fn response_observations(report: &CrawlReport) -> Vec<ResponseObservation> {
    let mut rows = Vec::new();
    for page in &report.pages {
        for (index, hop) in page.redirect_chain.iter().enumerate() {
            // A valid redirect advances final_url to the next target before it
            // is requested. With no terminal response, the retained header map
            // belongs to the last captured hop, even when the target revisits an
            // earlier URL in a loop. A terminal response replaces that map.
            let last_headers =
                page.final_status.is_none() && index + 1 == page.redirect_chain.len();
            rows.push(ResponseObservation {
                requested_url: page.url.clone(),
                response_url: hop.url.clone(),
                response_index: index + 1,
                status: hop.status,
                location: Some(hop.target.clone()),
                capture_kind: "redirect_hop",
                headers: last_headers.then(|| page.headers.clone()),
                content_type: last_headers.then(|| page.content_type.clone()),
                body_bytes: None,
                body_complete: None,
                document_metadata_available: false,
            });
        }
        let Some(status) = page.final_status else {
            continue;
        };
        let redirect = matches!(status, 301 | 302 | 303 | 307 | 308);
        rows.push(ResponseObservation {
            requested_url: page.url.clone(),
            response_url: if page.final_url.is_empty() {
                page.url.clone()
            } else {
                page.final_url.clone()
            },
            response_index: page.redirect_chain.len() + 1,
            status,
            location: page.headers.get("location").cloned(),
            capture_kind: "terminal_response",
            headers: Some(page.headers.clone()),
            content_type: Some(page.content_type.clone()),
            body_bytes: (!redirect).then_some(page.size_bytes),
            body_complete: (!redirect).then_some(!page.truncated && page.error.is_none()),
            document_metadata_available: !redirect && page.is_html() && page.error.is_none(),
        });
    }
    rows
}

pub fn export_response_observations_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "Requested URL",
        "Response URL",
        "Response Index",
        "Status",
        "Location",
        "Capture Kind",
        "Headers JSON",
        "Content Type",
        "Body Bytes",
        "Body Complete",
        "Document Metadata Available",
    ])?;
    for row in response_observations(report) {
        writer.write_record([
            crate::export::csv_text(&row.requested_url),
            crate::export::csv_text(&row.response_url),
            row.response_index.to_string(),
            row.status.to_string(),
            crate::export::csv_text(row.location.as_deref().unwrap_or_default()),
            row.capture_kind.into(),
            row.headers
                .map(|value| serde_json::to_string(&value))
                .transpose()?
                .unwrap_or_default(),
            crate::export::csv_text(row.content_type.as_deref().unwrap_or_default()),
            row.body_bytes.map(|v| v.to_string()).unwrap_or_default(),
            row.body_complete.map(|v| v.to_string()).unwrap_or_default(),
            row.document_metadata_available.to_string(),
        ])?;
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PageRecord, RedirectHop};
    fn report(page: PageRecord) -> CrawlReport {
        CrawlReport {
            operation: None,
            config: Default::default(),
            pages: vec![page],
            issues: vec![],
            summary: Default::default(),
            warnings: vec![],
        }
    }
    #[test]
    fn chain_retains_intermediate_status_without_assigning_final_document_to_it() {
        let page = PageRecord {
            url: "https://fixture.test/a".into(),
            final_url: "https://fixture.test/c".into(),
            status: Some(301),
            final_status: Some(200),
            content_type: "text/html".into(),
            size_bytes: 90,
            redirect_chain: vec![
                RedirectHop {
                    url: "https://fixture.test/a".into(),
                    status: 301,
                    target: "https://fixture.test/b".into(),
                },
                RedirectHop {
                    url: "https://fixture.test/b".into(),
                    status: 302,
                    target: "https://fixture.test/c".into(),
                },
            ],
            ..Default::default()
        };
        let result = response_observations(&report(page));
        assert_eq!(result.len(), 3);
        assert_eq!(result[1].response_url, "https://fixture.test/b");
        assert_eq!(result[1].status, 302);
        assert_eq!(result[1].body_bytes, None);
        assert!(result[1].headers.is_none());
        assert!(!result[1].document_metadata_available);
        assert_eq!(result[2].body_bytes, Some(90));
        assert_eq!(result[2].body_complete, Some(true));
        assert!(result[2].document_metadata_available);
    }
    #[test]
    fn stopped_chain_and_missing_location_do_not_invent_response_body_or_new_request() {
        let page = PageRecord {
            url: "https://fixture.test/a".into(),
            final_url: "https://foreign.test/".into(),
            status: Some(301),
            final_status: None,
            redirect_chain: vec![RedirectHop {
                url: "https://fixture.test/a".into(),
                status: 301,
                target: "https://foreign.test/".into(),
            }],
            ..Default::default()
        };
        let result = response_observations(&report(page));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].body_bytes, None);
        assert!(result[0].headers.is_some());
        let failed = PageRecord {
            url: "https://fixture.test/a".into(),
            status: Some(302),
            final_status: Some(302),
            error: Some("Missing Location".into()),
            ..Default::default()
        };
        let result = response_observations(&report(failed));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, 302);
        assert_eq!(result[0].body_bytes, None);
        assert!(
            response_observations(&report(PageRecord {
                error: Some("DNS failure".into()),
                ..Default::default()
            }))
            .is_empty()
        );
    }

    // The live fixture exercises exactly the final_url/final_status transitions
    // used by the native crawler. It prevents fabricated PageRecord fixtures
    // from concealing response-header ownership mistakes.
    async fn captured_redirects(looped: bool) -> (CrawlReport, Vec<String>, usize) {
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        };
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let foreign = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let foreign_url = format!("http://{}/unrequested", foreign.local_addr().unwrap());
        let foreign_count = Arc::new(AtomicUsize::new(0));
        let foreign_seen = foreign_count.clone();
        let foreign_task = tokio::spawn(async move {
            while let Ok((mut stream, _)) = foreign.accept().await {
                foreign_seen.fetch_add(1, Ordering::Relaxed);
                let _ = stream
                    .write_all(
                        b"HTTP/1.1 404 Fixture\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    )
                    .await;
            }
        });
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        let server_origin = origin.clone();
        let server_foreign = foreign_url.clone();
        let task = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut request = Vec::new();
                let mut chunk = [0; 4096];
                loop {
                    let size = stream.read(&mut chunk).await.unwrap();
                    if size == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..size]);
                    if request.windows(4).any(|value| value == b"\r\n\r\n") {
                        break;
                    }
                    if request.len() > 65536 {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&request);
                let path = request.split_whitespace().nth(1).unwrap_or("/");
                seen.lock().unwrap().push(path.to_owned());
                let marker = if path == "/a" { "a" } else { "b" };
                let location = if looped {
                    format!("{server_origin}/{}", if path == "/a" { "b" } else { "a" })
                } else {
                    server_foreign.clone()
                };
                let status = if marker == "a" { 301 } else { 302 };
                let response = format!(
                    "HTTP/1.1 {status} Fixture\r\nLocation: {location}\r\nX-Captured-Hop: {marker}\r\nContent-Type: application/fixture-{marker}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(response.as_bytes()).await.unwrap();
                let _ = stream.shutdown().await;
            }
        });
        let config = crate::CrawlConfig {
            seed_urls: vec![format!("{origin}/a")],
            discover_sitemaps: false,
            obey_robots: false,
            crawl_links: false,
            delay_ms: 0,
            request_timeout_secs: 3,
            ..Default::default()
        };
        let report = crate::Crawler::new(config)
            .unwrap()
            .run(None, Arc::new(AtomicBool::new(false)))
            .await
            .unwrap();
        task.abort();
        foreign_task.abort();
        let captured = requests.lock().unwrap().clone();
        (report, captured, foreign_count.load(Ordering::Relaxed))
    }

    #[tokio::test]
    async fn stopped_off_scope_redirect_headers_belong_to_the_captured_source_not_unrequested_target()
     {
        let (report, requests, foreign_requests) = captured_redirects(false).await;
        assert_eq!(requests, vec!["/a"]);
        assert_eq!(foreign_requests, 0);
        let page = &report.pages[0];
        assert_eq!(page.final_status, None);
        assert_ne!(page.final_url, page.url);
        assert_eq!(page.final_url, page.redirect_chain[0].target);
        assert_eq!(page.headers["x-captured-hop"], "a");
        let rows = response_observations(&report);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].response_url, page.url);
        assert_eq!(rows[0].headers.as_ref().unwrap()["x-captured-hop"], "a");
        assert_eq!(
            rows[0].content_type.as_deref(),
            Some("application/fixture-a")
        );
        assert_eq!(rows[0].body_bytes, None);
        assert_eq!(rows[0].body_complete, None);
        let json = serde_json::to_value(&rows).unwrap();
        assert_eq!(json[0]["headers"]["x-captured-hop"], "a");
        assert!(json[0]["body_bytes"].is_null());
        let csv = export_response_observations_csv(&report).unwrap();
        let data = csv::Reader::from_reader(csv.as_bytes())
            .records()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(&data[1], page.url);
        assert_eq!(&data[2], "1");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&data[6]).unwrap()["x-captured-hop"],
            "a"
        );
        assert_eq!(&data[8], "");
        assert_eq!(&data[9], "");
    }

    #[tokio::test]
    async fn redirect_loop_retained_headers_belong_only_to_last_captured_hop_by_index() {
        let (report, requests, foreign_requests) = captured_redirects(true).await;
        assert_eq!(requests, vec!["/a", "/b"]);
        assert_eq!(foreign_requests, 0);
        let page = &report.pages[0];
        assert_eq!(page.final_status, None);
        assert_eq!(page.final_url, page.url); // Loop target A was not fetched twice.
        assert!(page.error.as_deref().unwrap().contains("loop"));
        assert_eq!(page.headers["x-captured-hop"], "b");
        let rows = response_observations(&report);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].response_index, 1);
        assert_eq!(rows[1].response_index, 2);
        assert_eq!((rows[0].status, rows[1].status), (301, 302));
        assert_eq!(rows[0].response_url, page.url);
        assert!(rows[0].headers.is_none());
        assert!(rows[0].content_type.is_none());
        assert_eq!(rows[1].headers.as_ref().unwrap()["x-captured-hop"], "b");
        assert_eq!(
            rows[1].content_type.as_deref(),
            Some("application/fixture-b")
        );
        let json = serde_json::to_value(&rows).unwrap();
        assert!(json[0]["headers"].is_null());
        assert_eq!(json[1]["headers"]["x-captured-hop"], "b");
        let csv = export_response_observations_csv(&report).unwrap();
        let data = csv::Reader::from_reader(csv.as_bytes())
            .records()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(&data[0][6], "");
        assert_eq!(&data[0][7], "");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&data[1][6]).unwrap()["x-captured-hop"],
            "b"
        );
        for row in data {
            assert_eq!(&row[8], "");
            assert_eq!(&row[9], "");
            assert_eq!(&row[10], "false");
        }
    }
}
