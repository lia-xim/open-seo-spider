//! AUTHORED / NOT_RUN. Supplied actual slices, no native HTTP claim.
use super::*;

#[test]
fn sitemap_body_candidate_moves_existing_post_http_and_decoded_vecs_without_clone() {
    let body=b"\xef\xbb\xbf<?xml version='1.0'?>\r\n<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><!-- original --></urlset>".to_vec();
    let post_pointer = body.as_ptr();
    let page = PageRecord {
        url: "https://body.example/sitemap.xml".into(),
        final_url: "https://body.example/sitemap.xml".into(),
        status: Some(200),
        final_status: Some(200),
        ..Default::default()
    };
    let mut observation = SitemapProtocolObservation::capture(&page, true, &body);
    let decoded_pointer = observation.entity_decode.as_ref().unwrap().xml.as_ptr();
    let before = serde_json::to_vec(&observation).unwrap();
    let config = CrawlConfig {
        sitemap_source_archive: Some(Default::default()),
        ..Default::default()
    };
    observation.retain_source_bodies(body, &config);
    let candidate = observation.source_bodies.as_ref().unwrap();
    assert_eq!(
        candidate.post_http_entity.as_ref().unwrap().as_ptr(),
        post_pointer
    );
    assert_eq!(
        candidate.decoded_xml.as_ref().unwrap().as_ptr(),
        decoded_pointer
    );
    assert_eq!(candidate.post_http_entity, candidate.decoded_xml);
    assert!(observation.entity_decode.as_ref().unwrap().xml.is_empty());
    assert_eq!(
        serde_json::to_vec(&observation).unwrap(),
        before,
        "Existing serialized protocol facts unchanged"
    );
}

#[test]
fn sitemap_body_candidate_keeps_crc_failed_input_without_complete_decoded_xml() {
    use std::io::Write;
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(b"<urlset/>").unwrap();
    let mut body = gzip.finish().unwrap();
    let n = body.len();
    body[n - 8] ^= 1;
    let page = PageRecord {
        url: "https://body.example/sitemap.gz".into(),
        final_url: "https://body.example/sitemap.gz".into(),
        status: Some(200),
        final_status: Some(200),
        ..Default::default()
    };
    let mut observed = SitemapProtocolObservation::capture(&page, true, &body);
    assert!(observed.entity_decode.is_none());
    let expected = body.clone();
    observed.retain_source_bodies(
        body,
        &CrawlConfig {
            sitemap_source_archive: Some(Default::default()),
            ..Default::default()
        },
    );
    let c = observed.source_bodies.unwrap();
    assert_eq!(c.post_http_entity, Some(expected));
    assert!(c.decoded_xml.is_none());
}
