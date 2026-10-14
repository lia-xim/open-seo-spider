//! New progress assertions use known source bytes and a witness at actual Read returns.
//! The unchanged historical sitemap/tests.rs remains the legacy compatibility gate.
use super::*;
use flate2::{Compression, write::GzEncoder};
use std::io::{self, Write};

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn gz(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}
fn failure(outcome: &SitemapEntityDecodeOutcome, code: &str, message: Option<&str>) {
    let error = outcome.result.as_ref().unwrap_err();
    assert_eq!(error.code, code);
    assert_eq!(error.xml_byte_offset, None);
    if let Some(message) = message {
        assert_eq!(error.message, message);
    }
    assert_eq!(outcome.observation.stop_code.as_deref(), Some(code));
    assert!(outcome.observation.decoded_complete_sha256.is_none());
}
fn no_output(observation: &SitemapEntityDecodeObservation) {
    assert_eq!(observation.decoded_bytes_returned, None);
    assert_eq!(observation.decoded_prefix_sha256, None);
    assert_eq!(observation.decoded_complete_sha256, None);
    assert_eq!(observation.decoder_eof_observed, None);
    assert_eq!(observation.all_gzip_members_validated, None);
    assert_eq!(
        observation.integrity,
        SitemapEntityDecodeIntegrity::NotObserved
    );
}

/// This witness records actual successful delegated Read results, not a copied
/// decompressor, inferred prefix size, ISIZE, or a reimplementation of the loop.
struct ReturnedRead<R> {
    inner: R,
    returned: Vec<u8>,
    returned_zero: bool,
}
impl<R: Read> Read for ReturnedRead<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let result = self.inner.read(buffer);
        if let Ok(count) = &result {
            self.returned.extend_from_slice(&buffer[..*count]);
            self.returned_zero |= *count == 0;
        }
        result
    }
}
fn actual_gzip_read_witness(input: &[u8], limits: &SitemapParserLimits) -> Vec<u8> {
    let mut reader = ReturnedRead {
        inner: MultiGzDecoder::new(input),
        returned: Vec::new(),
        returned_zero: false,
    };
    let mut local = decode_sitemap_entity_observed(&[0x1f, 0x8b], &Default::default()).observation;
    local.decoder_eof_observed = Some(false);
    local.read_error_kind = None;
    local.integrity = SitemapEntityDecodeIntegrity::Incomplete;
    let mut progress = ReturnedProgress::new();
    let _ = read_gzip_xml(&mut reader, limits, &mut local, &mut progress);
    assert_eq!(progress.bytes, reader.returned.len() as u64);
    assert_eq!(
        format!("{:x}", progress.hash.finalize()),
        sha(&reader.returned)
    );
    assert_eq!(local.decoder_eof_observed, Some(reader.returned_zero));
    reader.returned
}
fn assert_prefix(outcome: &SitemapEntityDecodeOutcome, returned: &[u8]) {
    assert_eq!(
        outcome.observation.decoded_bytes_returned,
        Some(returned.len() as u64)
    );
    assert_eq!(
        outcome.observation.decoded_prefix_sha256.as_deref(),
        Some(sha(returned).as_str())
    );
}

#[test]
fn identity_actual_slice_and_output_have_separate_extent_without_http_claims() {
    for input in [
        b"".as_slice(),
        b"<urlset/>".as_slice(),
        "<root>ä</root>".as_bytes(),
    ] {
        let outcome = decode_sitemap_entity_observed(input, &Default::default());
        let decoded = outcome.result.as_ref().unwrap();
        assert_eq!(decoded.encoding, SitemapEntityEncoding::Identity);
        assert_eq!(decoded.xml, input);
        assert_eq!(decoded.post_http_decode_entity_bytes, input.len());
        assert_eq!(decoded.decoded_xml_bytes, input.len());
        assert_eq!(decoded.decoded_xml_sha256, sha(input));
        assert_eq!(
            outcome.observation.supplied_post_http_entity_slice_bytes,
            input.len() as u64
        );
        assert_eq!(
            outcome.observation.encoding,
            Some(SitemapEntityEncoding::Identity)
        );
        assert_eq!(
            outcome.observation.integrity,
            SitemapEntityDecodeIntegrity::Complete
        );
        assert_eq!(outcome.observation.decoder_eof_observed, None);
        assert_eq!(outcome.observation.all_gzip_members_validated, None);
        assert_eq!(
            outcome.observation.decoded_complete_sha256.as_deref(),
            Some(sha(input).as_str())
        );
        assert!(outcome.observation.stop_code.is_none());
        assert_prefix(&outcome, input);
        let serialized = serde_json::to_value(&outcome.observation).unwrap();
        assert!(serialized.get("xml").is_none());
        assert!(serialized.get("body_complete").is_none());
        assert!(serialized.get("wire_bytes").is_none());
    }
}

#[test]
fn invalid_limits_and_entity_preflight_do_not_invent_encoding_or_decoded_zero() {
    let input = gz(b"<urlset/>");
    let invalid = SitemapParserLimits {
        max_xml_bytes: 0,
        ..Default::default()
    };
    let outcome = decode_sitemap_entity_observed(&input, &invalid);
    failure(
        &outcome,
        "sitemap_invalid_limits",
        Some("max_xml_bytes must be 1..=67108864"),
    );
    assert_eq!(outcome.observation.effective_limits, None);
    assert_eq!(outcome.observation.encoding, None);
    no_output(&outcome.observation);
    let entity = SitemapParserLimits {
        max_entity_bytes: input.len() - 1,
        ..Default::default()
    };
    let outcome = decode_sitemap_entity_observed(&input, &entity);
    failure(
        &outcome,
        "sitemap_entity_byte_budget",
        Some("Post-HTTP-decoding entity exceeds byte budget"),
    );
    assert_eq!(
        outcome.observation.supplied_post_http_entity_slice_bytes,
        input.len() as u64
    );
    assert_eq!(
        outcome
            .observation
            .effective_limits
            .as_ref()
            .unwrap()
            .max_entity_bytes,
        (input.len() - 1) as u64
    );
    assert_eq!(outcome.observation.encoding, None);
    no_output(&outcome.observation);
    let above_ceiling = SitemapParserLimits {
        max_xml_bytes: 64 * 1024 * 1024 + 1,
        ..Default::default()
    };
    let outcome = decode_sitemap_entity_observed(&input, &above_ceiling);
    failure(
        &outcome,
        "sitemap_invalid_limits",
        Some("max_xml_bytes must be 1..=67108864"),
    );
    no_output(&outcome.observation);
}

#[test]
fn identity_xml_budget_keeps_supplied_extent_separate_from_unreturned_output() {
    let limits = SitemapParserLimits {
        max_xml_bytes: 9,
        ..Default::default()
    };
    let exact = decode_sitemap_entity_observed(b"<urlset/>", &limits);
    assert_eq!(exact.result.as_ref().unwrap().xml, b"<urlset/>");
    assert_prefix(&exact, b"<urlset/>");
    let plus_one = decode_sitemap_entity_observed(b"<urlset/> ", &limits);
    failure(
        &plus_one,
        "sitemap_xml_byte_budget",
        Some("XML input exceeds byte budget"),
    );
    assert_eq!(
        plus_one.observation.supplied_post_http_entity_slice_bytes,
        10
    );
    assert_eq!(
        plus_one.observation.encoding,
        Some(SitemapEntityEncoding::Identity)
    );
    no_output(&plus_one.observation);
}

#[test]
fn actual_gzip_eight_mib_exact_cap_validates_eof_and_plus_one_counts_real_sentinel() {
    let limits = SitemapParserLimits::default();
    let xml = vec![b'a'; 8 * 1024 * 1024];
    let input = gz(&xml);
    let exact = decode_sitemap_entity_observed(&input, &limits);
    assert_eq!(exact.result.as_ref().unwrap().xml, xml);
    assert_eq!(exact.observation.decoder_eof_observed, Some(true));
    assert_eq!(exact.observation.all_gzip_members_validated, Some(true));
    assert_eq!(
        exact.observation.integrity,
        SitemapEntityDecodeIntegrity::Complete
    );
    assert_prefix(&exact, &actual_gzip_read_witness(&input, &limits));
    let mut xml = xml;
    xml.push(b'z');
    let input = gz(&xml);
    let outcome = decode_sitemap_entity_observed(&input, &limits);
    failure(
        &outcome,
        "sitemap_xml_inflation_byte_budget",
        Some("Actual combined inflated bytes exceed XML byte budget"),
    );
    let returned = actual_gzip_read_witness(&input, &limits);
    assert_eq!(returned, xml);
    assert_prefix(&outcome, &returned);
    assert_eq!(
        outcome.observation.decoded_bytes_returned,
        Some(8 * 1024 * 1024 + 1)
    );
    assert_eq!(outcome.observation.decoder_eof_observed, Some(false));
    assert_eq!(outcome.observation.all_gzip_members_validated, Some(false));
    assert_eq!(
        outcome.observation.integrity,
        SitemapEntityDecodeIntegrity::Incomplete
    );
}

#[test]
fn gzip_known_zero_is_distinct_from_predecode_unavailable() {
    let empty = decode_sitemap_entity_observed(&gz(b""), &Default::default());
    assert_eq!(empty.result.as_ref().unwrap().xml, b"");
    assert_prefix(&empty, b"");
    assert_eq!(empty.observation.decoder_eof_observed, Some(true));
    assert_eq!(empty.observation.all_gzip_members_validated, Some(true));
    let failed = decode_sitemap_entity_observed(&[0x1f, 0x8b], &Default::default());
    failure(&failed, "sitemap_invalid_gzip_entity", None);
    assert_prefix(&failed, b"");
    assert_eq!(failed.observation.decoder_eof_observed, Some(false));
    assert_eq!(failed.observation.all_gzip_members_validated, Some(false));
    assert_eq!(
        failed.observation.read_error_kind,
        Some(SitemapEntityDecodeReadErrorKind::UnexpectedEof)
    );
    assert_eq!(
        failed.observation.integrity,
        SitemapEntityDecodeIntegrity::Incomplete
    );
}

#[test]
fn actual_crc_isize_and_truncated_footer_errors_preserve_only_returned_prefix() {
    let xml = vec![b'q'; 20_000];
    let input = gz(&xml);
    for offset in [8, 4] {
        let mut corrupt = input.clone();
        let index = corrupt.len() - offset;
        corrupt[index] ^= 1;
        let outcome = decode_sitemap_entity_observed(&corrupt, &Default::default());
        failure(
            &outcome,
            "sitemap_invalid_gzip_entity",
            Some(
                "Gzip entity did not complete: corrupt gzip stream does not have a matching checksum",
            ),
        );
        let returned = actual_gzip_read_witness(&corrupt, &Default::default());
        assert_eq!(returned, xml);
        assert_prefix(&outcome, &returned);
        assert_eq!(
            outcome.observation.read_error_kind,
            Some(SitemapEntityDecodeReadErrorKind::InvalidInput)
        );
        assert_eq!(
            outcome.observation.integrity,
            SitemapEntityDecodeIntegrity::Failed
        );
        assert_eq!(outcome.observation.decoder_eof_observed, Some(false));
        assert_eq!(outcome.observation.all_gzip_members_validated, Some(false));
    }
    let truncated = &input[..input.len() - 3];
    let outcome = decode_sitemap_entity_observed(truncated, &Default::default());
    failure(&outcome, "sitemap_invalid_gzip_entity", None);
    let returned = actual_gzip_read_witness(truncated, &Default::default());
    assert_eq!(returned, xml);
    assert_prefix(&outcome, &returned);
    assert_eq!(
        outcome.observation.integrity,
        SitemapEntityDecodeIntegrity::Incomplete
    );
    assert_eq!(
        outcome.observation.read_error_kind,
        Some(SitemapEntityDecodeReadErrorKind::UnexpectedEof)
    );
}

#[test]
fn actual_truncated_deflate_progress_is_a_returned_prefix_not_full_source_size() {
    let mut state = 17u64;
    let xml = (0..131_072)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state >> 32) as u8
        })
        .collect::<Vec<_>>();
    let encoded = gz(&xml);
    let input = &encoded[..encoded.len() / 2];
    let outcome = decode_sitemap_entity_observed(input, &Default::default());
    failure(&outcome, "sitemap_invalid_gzip_entity", None);
    let returned = actual_gzip_read_witness(input, &Default::default());
    assert!(!returned.is_empty());
    assert!(returned.len() < xml.len());
    assert_eq!(returned, xml[..returned.len()]);
    assert_prefix(&outcome, &returned);
    assert_eq!(outcome.observation.decoder_eof_observed, Some(false));
    assert_eq!(outcome.observation.all_gzip_members_validated, Some(false));
}

#[test]
fn concatenated_members_validate_together_and_later_corruption_cannot_hide() {
    let xml = "<urlset>ä</urlset>".as_bytes();
    let split = 9;
    let mut input = gz(&xml[..split]);
    input.extend(gz(&xml[split..]));
    let outcome = decode_sitemap_entity_observed(&input, &Default::default());
    assert_eq!(outcome.result.as_ref().unwrap().xml, xml);
    assert_prefix(&outcome, xml);
    assert_eq!(outcome.observation.all_gzip_members_validated, Some(true));
    let mut second = gz(b"");
    let crc = second.len() - 8;
    second[crc] ^= 1;
    let mut input = gz(xml);
    input.extend(second);
    let outcome = decode_sitemap_entity_observed(&input, &Default::default());
    failure(&outcome, "sitemap_invalid_gzip_entity", None);
    assert_prefix(
        &outcome,
        &actual_gzip_read_witness(&input, &Default::default()),
    );
    assert_eq!(outcome.observation.all_gzip_members_validated, Some(false));
    assert_eq!(outcome.observation.decoder_eof_observed, Some(false));
}

#[test]
fn garbage_and_nested_gzip_keep_failure_and_outer_member_evidence_separate() {
    let xml = b"<urlset/>";
    for suffix in [vec![0], vec![b'x'; 12]] {
        let mut input = gz(xml);
        input.extend(suffix);
        let outcome = decode_sitemap_entity_observed(&input, &Default::default());
        failure(&outcome, "sitemap_invalid_gzip_entity", None);
        assert_prefix(&outcome, xml);
        assert_eq!(outcome.observation.all_gzip_members_validated, Some(false));
        assert_eq!(outcome.observation.decoder_eof_observed, Some(false));
    }
    let inner = gz(xml);
    let outcome = decode_sitemap_entity_observed(&gz(&inner), &Default::default());
    failure(
        &outcome,
        "sitemap_nested_gzip_entity",
        Some("Recursive gzip file entities are unsupported"),
    );
    assert_prefix(&outcome, &inner);
    assert_eq!(outcome.observation.all_gzip_members_validated, Some(true));
    assert_eq!(outcome.observation.decoder_eof_observed, Some(true));
    assert_eq!(
        outcome.observation.integrity,
        SitemapEntityDecodeIntegrity::Failed
    );
}

#[test]
fn complete_gzip_decode_does_not_claim_wellformed_xml_or_http_body_completeness() {
    let input = gz(b"not XML and not an HTTP representation witness");
    let outcome = decode_sitemap_entity_observed(&input, &Default::default());
    assert!(outcome.result.is_ok());
    assert_eq!(
        outcome.observation.integrity,
        SitemapEntityDecodeIntegrity::Complete
    );
    let serialized = serde_json::to_value(outcome.observation).unwrap();
    assert!(serialized.get("xml_scan_complete").is_none());
    assert!(serialized.get("body_complete").is_none());
}

#[test]
fn read_error_written_buffer_bytes_are_never_counted_or_hashed() {
    struct WritesThenErr(bool);
    impl Read for WritesThenErr {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if !self.0 {
                self.0 = true;
                buffer[..3].copy_from_slice(b"abc");
                Ok(3)
            } else {
                buffer[..6].copy_from_slice(b"POISON");
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "written buffer is not returned",
                ))
            }
        }
    }
    let mut reader = ReturnedRead {
        inner: WritesThenErr(false),
        returned: Vec::new(),
        returned_zero: false,
    };
    let mut observation =
        decode_sitemap_entity_observed(&[0x1f, 0x8b], &Default::default()).observation;
    let mut progress = ReturnedProgress::new();
    let error = read_gzip_xml(
        &mut reader,
        &Default::default(),
        &mut observation,
        &mut progress,
    )
    .unwrap_err();
    assert_eq!(error.code, "sitemap_invalid_gzip_entity");
    assert_eq!(
        error.message,
        "Gzip entity did not complete: written buffer is not returned"
    );
    assert_eq!(reader.returned, b"abc");
    assert_eq!(progress.bytes, 3);
    assert_eq!(format!("{:x}", progress.hash.finalize()), sha(b"abc"));
    assert_eq!(
        observation.read_error_kind,
        Some(SitemapEntityDecodeReadErrorKind::InvalidData)
    );
    assert_eq!(observation.integrity, SitemapEntityDecodeIntegrity::Failed);
}
