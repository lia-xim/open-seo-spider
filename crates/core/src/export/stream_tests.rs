use super::*;
use crate::{AuditIssue, CustomExtraction, LinkRecord, PageRecord, Severity};
use serde_json::json;
use std::io::{self, Write};

#[derive(Debug)]
struct Cap {
    bytes: Vec<u8>,
    cap: usize,
    refused_writes: usize,
}
impl Cap {
    fn new(cap: usize) -> Self {
        Self {
            bytes: Vec::new(),
            cap,
            refused_writes: 0,
        }
    }
}
impl Write for Cap {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(required) = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|n| *n <= self.cap)
        else {
            self.refused_writes += 1;
            return Err(io::Error::other("owned test sink byte budget exceeded"));
        };
        if required > self.bytes.capacity() {
            let capacity = self
                .bytes
                .capacity()
                .saturating_mul(2)
                .max(4096)
                .max(required)
                .min(self.cap);
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(io::Error::other)?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn fixture() -> CrawlReport {
    CrawlReport {
        pages: vec![PageRecord {
            url: "https://owned.invalid/雪".into(),
            final_url: "https://owned.invalid/雪".into(),
            status: Some(200),
            final_status: Some(200),
            internal: true,
            content_type: "text/html".into(),
            titles: vec!["=SUM(1,2)".into()],
            links: vec![LinkRecord {
                url: "https://owned.invalid/outside".into(),
                text: "@link".into(),
                ..Default::default()
            }],
            custom_extractions: vec![CustomExtraction {
                name: "owned".into(),
                values: vec!["α,\"雪\"\nβ".into()],
                ..Default::default()
            }],
            custom_javascript_results: vec![
                json!({"name":"owned","value":0,"value_type":"number","error":null,"elapsed_ms":1}),
            ],
            browser_cookies: vec![
                json!({"name":"owned","domain":"owned.invalid","path":"/","expires":-1,"httpOnly":true,"secure":true,"sameSite":"Lax","value_length":0}),
            ],
            ..Default::default()
        }],
        issues: vec![AuditIssue {
            severity: Severity::Notice,
            code: "owned".into(),
            url: "https://owned.invalid/雪".into(),
            message: "Owned, unicode 雪".into(),
            related_urls: vec![],
        }],
        ..Default::default()
    }
}
fn encode<W: Write>(
    kind: usize,
    report: &CrawlReport,
    sink: W,
    extra: &[(String, String)],
) -> anyhow::Result<W> {
    match kind {
        0 => write_pages_csv(report, sink, extra),
        1 => write_issues_csv(report, sink, extra),
        2 => write_links_csv(report, sink, extra),
        3 => write_dom_observations_csv(report, sink, extra),
        4 => write_custom_extractions_csv(report, sink, extra),
        5 => write_custom_javascript_csv(report, sink, extra),
        6 => write_browser_cookies_csv(report, sink, extra),
        7 => write_pdf_csv(report, sink, extra),
        8 => write_image_resources_csv(report, sink, extra),
        9 => write_custom_search_csv(report, sink, extra),
        10 => crate::write_response_observations_csv(report, sink, extra),
        11 => write_fragments_csv(report, sink, extra),
        12 => write_input_occurrences_csv(report, sink, extra),
        _ => unreachable!(),
    }
}

#[test]
fn every_direct_csv_sink_includes_unambiguous_context_and_refuses_one_byte_overflow() {
    let report = fixture();
    let before = serde_json::to_vec(&report).unwrap();
    let extra = vec![
        ("Segment Snapshot Token".into(), "α,\"雪\"\nβ".into()),
        ("Segment Label".into(), " =OWNED()".into()),
    ];
    for kind in 0..13 {
        let baseline = encode(kind, &report, Vec::new(), &extra).unwrap();
        let exact = encode(kind, &report, Cap::new(baseline.len()), &extra).unwrap();
        assert_eq!(exact.bytes, baseline, "kind{kind}");
        let mut too_small = Cap::new(baseline.len() - 1);
        assert!(
            encode(kind, &report, &mut too_small, &extra).is_err(),
            "kind{kind}"
        );
        assert!(too_small.refused_writes > 0);
        assert!(too_small.bytes.len() < baseline.len());
        assert!(too_small.bytes.capacity() <= too_small.cap);
        let mut reader = csv::Reader::from_reader(baseline.as_slice());
        let headers = reader.headers().unwrap().clone();
        assert_eq!(&headers[headers.len() - 2], "Segment Snapshot Token");
        assert_eq!(&headers[headers.len() - 1], "Segment Label");
        for row in reader.records() {
            let row = row.unwrap();
            assert_eq!(row.len(), headers.len());
            assert_eq!(&row[row.len() - 2], "α,\"雪\"\nβ");
            assert_eq!(&row[row.len() - 1], "' =OWNED()");
        }
    }
    assert_eq!(serde_json::to_vec(&report).unwrap(), before);
    let mut bytes = Vec::new();
    assert!(
        write_pages_csv(
            &report,
            &mut bytes,
            &[("URL".into(), "false ownership".into())]
        )
        .is_err()
    );
    assert!(
        bytes.is_empty(),
        "Context/header collision must fail before any success bytes"
    );
    assert!(
        write_pages_csv(
            &report,
            Vec::new(),
            &[
                ("Repeated".into(), "a".into()),
                ("Repeated".into(), "b".into())
            ]
        )
        .is_err()
    );
}

#[test]
fn full_csv_encoder_rejects_huge_owner_url_fanout_inside_the_sink_not_after_building_a_string() {
    // This small input would expand beyond 29GiB if the complete CSV were built first.
    let source = format!("https://owned.invalid/{}", "x".repeat(3 * 1024 * 1024));
    let report = CrawlReport {
        pages: vec![PageRecord {
            url: source.clone(),
            final_url: source,
            links: (0..10_000)
                .map(|i| LinkRecord {
                    url: format!("https://owned.invalid/target/{i}"),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }],
        ..Default::default()
    };
    assert!(serde_json::to_vec(&report).unwrap().len() < 64 * 1024 * 1024);
    let mut sink = Cap::new(64 * 1024 * 1024);
    let error = write_links_csv(
        &report,
        &mut sink,
        &[("Segment Snapshot Token".into(), "owned".into())],
    )
    .unwrap_err();
    assert!(error.to_string().contains("byte budget"));
    assert!(sink.refused_writes > 0);
    assert!(sink.bytes.len() <= sink.cap);
    assert!(sink.bytes.capacity() <= sink.cap);
    // A late postallocation cap cannot satisfy this bounded sink contract.
    assert!(
        !sink
            .bytes
            .windows(b"target/9999".len())
            .any(|part| part == b"target/9999")
    );
}

#[test]
fn fragment_csv_stops_before_cloning_all_repeated_large_target_payloads() {
    let target_url = "https://owned.invalid/outside";
    let report = CrawlReport {
        config: crate::CrawlConfig {
            validate_fragments: true,
            ..Default::default()
        },
        pages: vec![
            PageRecord {
                url: "https://owned.invalid/source".into(),
                final_url: "https://owned.invalid/source".into(),
                links: (0..10_000)
                    .map(|index| LinkRecord {
                        kind: "anchor".into(),
                        url: target_url.into(),
                        fragment: Some("owned-target".into()),
                        dom_location: Some(crate::DomLocation {
                            index,
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            },
            PageRecord {
                url: target_url.into(),
                final_url: target_url.into(),
                status: Some(200),
                final_status: Some(200),
                content_type: "text/html".into(),
                dom_metadata: Some(crate::DomMetadata {
                    complete: true,
                    fragment_targets: vec![crate::FragmentTarget {
                        kind: "id".into(),
                        value: "owned-target".into(),
                        dom_location: crate::DomLocation {
                            path: Some("x".repeat(4 * 1024 * 1024)),
                            ..Default::default()
                        },
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert!(serde_json::to_vec(&report).unwrap().len() < 64 * 1024 * 1024);
    let selected = [0].into();
    let mut visited = 0;
    crate::visit_fragment_observations(&report, Some(&selected), |row| {
        assert_eq!(row.status, crate::FragmentStatus::MatchedId);
        assert_eq!(
            row.matched_target.unwrap().dom_location.path.unwrap().len(),
            4 * 1024 * 1024
        );
        visited += 1;
        anyhow::bail!("owned early visitor stop")
    })
    .unwrap_err();
    assert_eq!(
        visited, 1,
        "The producer must stop on the first refused row"
    );
    let mut sink = Cap::new(64 * 1024 * 1024);
    let error = write_selected_fragments_csv(&report, &selected, &mut sink, &[]).unwrap_err();
    assert!(error.to_string().contains("byte budget"));
    assert!(sink.refused_writes > 0);
    assert!(sink.bytes.len() <= sink.cap);
    assert!(sink.bytes.capacity() <= sink.cap);
}
