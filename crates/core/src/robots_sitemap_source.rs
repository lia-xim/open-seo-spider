//! Pure robots Sitemap-declaration facts; crawl allow/deny matching stays separate.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

pub const ROBOTS_SITEMAP_SOURCE_ENGINE: &str = "open-seo-spider/robots-sitemap-source-v2";
pub const ROBOTS_BODY_CAP: usize = 512 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RobotsSitemapSource {
    pub engine: String,
    pub requested_url: String,
    pub final_url: Option<String>,
    pub status: Option<u16>,
    pub request_started: bool,
    pub retained_post_http_decoding_bytes: usize,
    pub retained_prefix_sha256: String,
    pub body_complete: bool,
    pub body_limit_exceeded: bool,
    pub declaration_utf8_lossy: Option<bool>,
    pub declaration_scan_complete: bool,
    /// Complete successful declaration source only; absence/status is a separate fact.
    pub complete: bool,
    pub failure: Option<String>,
    pub declarations: Vec<RobotsSitemapDeclaration>,
    /// Actual historical LF/CRLF discovery projection, independently qualified.
    pub legacy_sitemaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RobotsSitemapDeclaration {
    pub ordinal: usize,
    pub line_ordinal: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    /// Original UTF8 line including EOL; lossy representation is explicitly qualified.
    pub raw_line: String,
    /// Value after the colon, before whitespace/comment handling, excluding EOL.
    pub raw_value: String,
    pub line_complete: bool,
    pub utf8_lossy: bool,
    /// Lenient projection of this original record, not proof of actual legacy scheduling.
    pub legacy_candidate_url: Option<String>,
}

#[derive(Debug, Default)]
pub struct RobotsBodyBuffer {
    pub bytes: Vec<u8>,
    pub limit_exceeded: bool,
    pub true_eof: bool,
    pub read_failure: Option<String>,
}
impl RobotsBodyBuffer {
    /// False means a limit sentinel was observed and the caller may stop reading.
    /// A chunk exactly filling the cap still requires the next chunk or true EOF.
    pub fn push_chunk(&mut self, chunk: &[u8]) -> bool {
        if self.limit_exceeded || self.true_eof || self.read_failure.is_some() {
            return false;
        }
        let remaining = ROBOTS_BODY_CAP.saturating_sub(self.bytes.len());
        self.bytes
            .extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        if chunk.len() > remaining {
            self.limit_exceeded = true;
            false
        } else {
            true
        }
    }
    pub fn observe_eof(&mut self) {
        if !self.limit_exceeded && self.read_failure.is_none() {
            self.true_eof = true;
        }
    }
}

impl RobotsSitemapSource {
    pub fn unavailable(
        requested_url: String,
        request_started: bool,
        final_url: Option<String>,
        status: Option<u16>,
        failure: String,
    ) -> Self {
        Self {
            engine: ROBOTS_SITEMAP_SOURCE_ENGINE.into(),
            requested_url,
            request_started,
            final_url,
            status,
            failure: Some(failure),
            ..Self::default()
        }
    }
    /// Caller records actual transport facts. No network request or source attestation.
    pub fn observed(
        requested_url: String,
        final_url: String,
        status: u16,
        body: &RobotsBodyBuffer,
        legacy_base: &Url,
    ) -> Self {
        let utf8_lossy = std::str::from_utf8(&body.bytes).is_err();
        let body_complete = body.true_eof && !body.limit_exceeded && body.read_failure.is_none();
        let mut source = Self {
            engine: ROBOTS_SITEMAP_SOURCE_ENGINE.into(),
            requested_url,
            final_url: Some(final_url),
            status: Some(status),
            request_started: true,
            retained_post_http_decoding_bytes: body.bytes.len(),
            retained_prefix_sha256: format!("{:x}", Sha256::digest(&body.bytes)),
            body_complete,
            body_limit_exceeded: body.limit_exceeded,
            declaration_utf8_lossy: Some(utf8_lossy),
            declaration_scan_complete: !utf8_lossy,
            failure: body
                .read_failure
                .clone()
                .or_else(|| {
                    body.limit_exceeded
                        .then(|| "robots_body_byte_budget".into())
                })
                .or_else(|| utf8_lossy.then(|| "robots_declaration_invalid_utf8".into())),
            ..Self::default()
        };
        // Keep legacy discovery behavior separate from the complete CR/LF scanner.
        source.legacy_sitemaps = String::from_utf8_lossy(&body.bytes)
            .lines()
            .filter_map(|line| {
                let (key, value) = line.split_once(':')?;
                key.trim().eq_ignore_ascii_case("sitemap").then_some(())?;
                crate::urls::normalize_url(
                    value.split('#').next().unwrap_or("").trim(),
                    Some(legacy_base),
                )
                .map(|url| url.to_string())
            })
            .collect();
        let mut start = 0;
        let mut line_index = 0;
        while start < body.bytes.len() {
            let boundary = body.bytes[start..]
                .iter()
                .position(|byte| matches!(*byte, b'\r' | b'\n'))
                .map(|offset| start + offset);
            let (content_end, end, line_complete) = match boundary {
                Some(position)
                    if body.bytes[position] == b'\r'
                        && body.bytes.get(position + 1) == Some(&b'\n') =>
                {
                    (position, position + 2, true)
                }
                Some(position) => (
                    position,
                    position + 1,
                    body.bytes[position] != b'\r'
                        || position + 1 < body.bytes.len()
                        || body_complete,
                ),
                None => (body.bytes.len(), body.bytes.len(), body_complete),
            };
            let raw_bytes = &body.bytes[start..end];
            let line = String::from_utf8_lossy(raw_bytes);
            let content = String::from_utf8_lossy(&body.bytes[start..content_end]);
            if let Some((key, value)) = content.split_once(':')
                && key.trim().eq_ignore_ascii_case("sitemap")
            {
                let candidate = crate::urls::normalize_url(
                    value.split('#').next().unwrap_or("").trim(),
                    Some(legacy_base),
                )
                .map(|url| url.to_string());
                source.declarations.push(RobotsSitemapDeclaration {
                    ordinal: source.declarations.len(),
                    line_ordinal: line_index + 1,
                    byte_start: start,
                    byte_end: end,
                    raw_line: line.to_string(),
                    raw_value: value.to_owned(),
                    line_complete,
                    utf8_lossy: std::str::from_utf8(raw_bytes).is_err(),
                    legacy_candidate_url: candidate,
                });
            }
            start = end;
            line_index += 1;
        }
        source.complete = status == 200 && body_complete && source.declaration_scan_complete;
        source
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inspect(bytes: &[u8], eof: bool) -> RobotsSitemapSource {
        let mut body = RobotsBodyBuffer::default();
        body.push_chunk(bytes);
        if eof {
            body.observe_eof();
        }
        RobotsSitemapSource::observed(
            "https://example.test/robots.txt".into(),
            "https://example.test/robots.txt".into(),
            200,
            &body,
            &Url::parse("https://example.test/a/b").unwrap(),
        )
    }
    fn inspect_status(status: u16) -> RobotsSitemapSource {
        let mut body = RobotsBodyBuffer::default();
        body.push_chunk(b"Sitemap: https://example.test/map.xml\n");
        body.observe_eof();
        RobotsSitemapSource::observed(
            "https://example.test/robots.txt".into(),
            "https://example.test/robots.txt".into(),
            status,
            &body,
            &Url::parse("https://example.test/").unwrap(),
        )
    }
    #[test]
    fn partial_206_body_is_not_a_complete_declaration_source() {
        let partial = inspect_status(206);
        assert!(partial.body_complete);
        assert!(!partial.complete);
        assert_eq!(partial.declarations.len(), 1);
        assert!(inspect_status(200).complete);
    }
    #[test]
    fn other_success_status_does_not_attest_full_robots_document() {
        let other = inspect_status(204);
        assert!(other.body_complete);
        assert!(!other.complete);
        assert!(inspect_status(200).complete);
    }
    #[test]
    fn bare_cr_preserves_every_original_declaration_after_user_agent() {
        let bytes=b"User-agent: *\rSitemap: https://example.test/a.xml\rSitemap: https://example.test/b.xml\r";
        let observed = inspect(bytes, true);
        assert!(observed.complete);
        assert_eq!(observed.declarations.len(), 2);
        assert_eq!(observed.declarations[0].line_ordinal, 2);
        assert_eq!(
            observed.declarations[1].raw_value,
            " https://example.test/b.xml"
        );
        assert!(
            observed
                .declarations
                .iter()
                .all(|row| row.line_complete && row.raw_line.ends_with('\r'))
        );
        assert!(observed.legacy_sitemaps.is_empty());
    }
    #[test]
    fn mixed_eols_preserve_byte_spans_and_global_original_ordinals() {
        let bytes=b"#header\r\nSitemap: https://example.test/a\rSitemap: https://example.test/b\nSitemap: https://example.test/c\r\n";
        let observed = inspect(bytes, true);
        assert_eq!(observed.declarations.len(), 3);
        assert_eq!(
            observed
                .declarations
                .iter()
                .map(|row| row.line_ordinal)
                .collect::<Vec<_>>(),
            vec![2, 3, 4]
        );
        for row in &observed.declarations {
            assert_eq!(
                &bytes[row.byte_start..row.byte_end],
                row.raw_line.as_bytes()
            );
        }
        assert_eq!(
            observed
                .declarations
                .iter()
                .map(|row| row.ordinal)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }
    #[test]
    fn terminal_cr_without_eof_cannot_attest_full_original_eol() {
        let bytes = b"Sitemap: https://example.test/a\r";
        assert!(!inspect(bytes, false).declarations[0].line_complete);
        assert!(inspect(bytes, true).declarations[0].line_complete);
        assert!(
            inspect(b"Sitemap: https://example.test/a\r\n", false).declarations[0].line_complete
        );
    }
    #[test]
    fn legacy_projection_stays_lf_based_and_separate_from_cr_facts() {
        let bytes =
            b"Sitemap: https://example.test/a.xml#old\rSitemap: https://example.test/b.xml\r";
        let observed = inspect(bytes, true);
        assert_eq!(observed.declarations.len(), 2);
        assert_eq!(observed.legacy_sitemaps, vec!["https://example.test/a.xml"]);
        assert_eq!(
            observed.declarations[1].legacy_candidate_url.as_deref(),
            Some("https://example.test/b.xml")
        );
    }
    #[test]
    fn complete_empty_source_is_distinct_from_unavailable() {
        let empty = inspect(b"", true);
        assert!(empty.complete && empty.body_complete);
        assert!(empty.declarations.is_empty());
        let missing = RobotsSitemapSource::unavailable(
            "https://e.test/robots.txt".into(),
            true,
            None,
            Some(404),
            "http_status_404".into(),
        );
        assert!(!missing.complete);
        assert_eq!(missing.status, Some(404));
        assert_eq!(missing.declaration_utf8_lossy, None);
    }
    #[test]
    fn raw_duplicate_invalid_and_comment_values_stay_distinct() {
        let s=inspect(b"Sitemap: https://example.test/map.xml #first\r\nsitemap: https://example.test/map.xml\nSITEMAP: ftp://example.test/no.xml\nSitemap: \n",true);
        assert!(s.complete);
        assert_eq!(s.declarations.len(), 4);
        assert_eq!(
            s.declarations[0].raw_value,
            " https://example.test/map.xml #first"
        );
        assert!(s.declarations[0].raw_line.ends_with("\r\n"));
        assert_eq!(
            s.declarations[0].legacy_candidate_url,
            s.declarations[1].legacy_candidate_url
        );
        assert_eq!(s.declarations[2].legacy_candidate_url, None);
        // Existing lenient normalization manufactures the base URL for empty
        // input. Keep it qualified rather than pretending the declaration said it.
        assert_eq!(s.declarations[3].raw_value, " ");
        assert_eq!(
            s.declarations[3].legacy_candidate_url.as_deref(),
            Some("https://example.test/a/b")
        );
        assert_eq!(
            s.declarations.iter().map(|d| d.ordinal).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
    }
    #[test]
    fn source_ordinals_and_byte_spans_count_non_directive_lines() {
        let input = "#ä\nUser-agent: *\n\nSitemap: https://example.test/x\n";
        let s = inspect(input.as_bytes(), true);
        assert_eq!(s.declarations[0].line_ordinal, 4);
        let d = &s.declarations[0];
        assert_eq!(
            &input.as_bytes()[d.byte_start..d.byte_end],
            d.raw_line.as_bytes()
        );
    }
    #[test]
    fn partial_terminal_line_is_qualified_and_not_complete_source() {
        let s = inspect(
            b"Sitemap: https://example.test/a\nSitemap: https://example.test/b",
            false,
        );
        assert!(!s.complete && !s.body_complete);
        assert_eq!(s.declarations.len(), 2);
        assert!(s.declarations[0].line_complete);
        assert!(!s.declarations[1].line_complete);
    }
    #[test]
    fn final_line_without_eol_is_complete_only_after_eof() {
        let s = inspect(b"Sitemap: https://example.test/a", true);
        assert!(s.complete && s.declarations[0].line_complete);
    }
    #[test]
    fn invalid_utf8_retains_lossiness_and_prefix_hash() {
        let s = inspect(b"#bad\xff\nSitemap: https://example.test/a\n", true);
        assert!(s.body_complete);
        assert!(!s.complete && !s.declaration_scan_complete);
        assert_eq!(s.declaration_utf8_lossy, Some(true));
        assert_eq!(
            s.failure.as_deref(),
            Some("robots_declaration_invalid_utf8")
        );
        assert_eq!(s.declarations.len(), 1);
        assert!(!s.declarations[0].utf8_lossy);
        assert_eq!(
            s.retained_prefix_sha256,
            format!(
                "{:x}",
                Sha256::digest(b"#bad\xff\nSitemap: https://example.test/a\n")
            )
        );
    }
    #[test]
    fn invalid_value_bytes_never_claim_lossless_raw_xml() {
        let s = inspect(b"Sitemap: https://example.test/\xff\n", true);
        assert!(s.declarations[0].utf8_lossy);
        assert!(!s.complete);
        assert!(s.declarations[0].raw_value.contains('\u{fffd}'));
    }
    #[test]
    fn exact_cap_without_eof_remains_unknown() {
        let mut b = RobotsBodyBuffer::default();
        assert!(b.push_chunk(&vec![b' '; ROBOTS_BODY_CAP]));
        assert_eq!(b.bytes.len(), ROBOTS_BODY_CAP);
        assert!(!b.true_eof && !b.limit_exceeded);
        b.observe_eof();
        assert!(b.true_eof);
    }
    #[test]
    fn limit_plus_one_is_retained_prefix_without_false_eof() {
        let mut b = RobotsBodyBuffer::default();
        assert!(!b.push_chunk(&vec![b' '; ROBOTS_BODY_CAP + 1]));
        assert_eq!(b.bytes.len(), ROBOTS_BODY_CAP);
        assert!(b.limit_exceeded);
        b.observe_eof();
        assert!(!b.true_eof);
    }
    #[test]
    fn split_sentinel_chunk_detects_actual_overflow() {
        let mut b = RobotsBodyBuffer::default();
        assert!(b.push_chunk(&vec![b' '; ROBOTS_BODY_CAP]));
        assert!(!b.push_chunk(b"x"));
        assert!(b.limit_exceeded);
        assert_eq!(b.bytes.len(), ROBOTS_BODY_CAP);
    }
    #[test]
    fn read_failure_prevents_completion_even_at_exact_cap() {
        let mut b = RobotsBodyBuffer::default();
        b.push_chunk(&vec![b' '; ROBOTS_BODY_CAP]);
        b.read_failure = Some("owned read error".into());
        b.observe_eof();
        assert!(!b.true_eof);
        let s = RobotsSitemapSource::observed(
            "https://e.test/robots.txt".into(),
            "https://e.test/robots.txt".into(),
            200,
            &b,
            &Url::parse("https://e.test/").unwrap(),
        );
        assert!(!s.body_complete && !s.complete);
        assert_eq!(s.failure.as_deref(), Some("owned read error"));
    }
    #[test]
    fn legacy_relative_candidate_is_qualified_not_relabelled_original() {
        let s = inspect(b"Sitemap: child.xml#legacy-fragment\n", true);
        let d = &s.declarations[0];
        assert_eq!(d.raw_value, " child.xml#legacy-fragment");
        assert_eq!(
            d.legacy_candidate_url.as_deref(),
            Some("https://example.test/a/child.xml")
        );
    }
}
