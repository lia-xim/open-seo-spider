//! Bounded observations of the decoded native HTTP source, before DOM repair.
//! This is the Google-documented head-element allowlist, not an HTML validator,
//! browser/search-engine execution, or evidence about JavaScript-created DOM.
use crate::{AuditIssue, ContentCaptureDecodeStatus, PageRecord, Severity};
use html5ever::tendril::StrTendril;
use html5ever::tokenizer::states::{Rawtext, Rcdata, ScriptData};
use html5ever::tokenizer::{
    BufferQueue, Tag, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::{Cell, RefCell};

pub const HTML_HEAD_ENGINE: &str = "open-seo-spider/raw-http-head-v1";
pub const HTML_HEAD_PROFILE: &str = "html5ever-0.39-tokenizer-explicit-head-scripting-enabled-v1";
pub const MAX_HEAD_SCAN_UTF8_BYTES: usize = 128 * 1024;
pub const MAX_HEAD_TOKENS: usize = 2048;
pub const MAX_HEAD_TAG_UTF8_BYTES: usize = 4096;
pub const MAX_HEAD_FOLLOWING: usize = 8;
pub const MAX_HEAD_EXCERPT_UTF8_BYTES: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HtmlHeadStatus {
    Complete,
    Unknown,
}

/// All offsets refer to the actual decoded source string's UTF-8 bytes, not
/// wire bytes or the repaired DOM. End is exclusive; line/column are one-based.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HtmlHeadSpan {
    pub element: String,
    pub start_utf8_byte: usize,
    pub end_utf8_byte: usize,
    pub line: usize,
    pub unicode_column: usize,
    pub raw_excerpt: String,
    pub excerpt_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HtmlHeadObservation {
    pub engine: String,
    pub parser_profile: String,
    pub source: String,
    pub document_url: String,
    pub fetched_at: String,
    pub status: HtmlHeadStatus,
    pub reason: String,
    pub decoded_source_utf8_bytes: usize,
    pub scanned_prefix_utf8_bytes: usize,
    pub scanned_prefix_sha256: String,
    pub explicit_head: Option<HtmlHeadSpan>,
    pub first_invalid_child: Option<HtmlHeadSpan>,
    /// These are following head-element tokens in the declared source window,
    /// excluding inert template contents and raw text. Not accepted SEO values.
    pub following_metadata: Vec<HtmlHeadSpan>,
}

fn permitted(name: &str) -> bool {
    matches!(
        name,
        "title" | "meta" | "link" | "script" | "style" | "base" | "noscript" | "template"
    )
}

#[derive(Default)]
struct Scan {
    head: Option<HtmlHeadSpan>,
    invalid: Option<HtmlHeadSpan>,
    following: Vec<HtmlHeadSpan>,
    template_depth: usize,
    raw_element: Option<String>,
    tokens: usize,
    done: bool,
    complete: bool,
    problem: Option<&'static str>,
    boundary: &'static str,
}

struct Sink<'a> {
    html: &'a str,
    fed_end: Cell<usize>,
    previous_markup_end: Cell<usize>,
    scan: RefCell<Scan>,
}

/// Locate a token emitted by the HTML tokenizer, never discover tags with a
/// regex. Only an unambiguous quote-delimited source lexeme ending exactly at
/// that token's consumed byte is accepted. Unsupported lexical recovery is
/// Unknown rather than an invented source position or a clean bill of health.
fn token_span(html: &str, from: usize, end: usize, tag: &Tag) -> Option<HtmlHeadSpan> {
    if end > html.len() || html.as_bytes().get(end.checked_sub(1)?) != Some(&b'>') {
        return None;
    }
    let name: &str = tag.name.as_ref();
    let mut found = None;
    for start in from..end {
        if html.as_bytes()[start] != b'<' {
            continue;
        }
        let mut cursor = start.checked_add(1)?;
        let closing = html.as_bytes().get(cursor) == Some(&b'/');
        if closing {
            cursor = cursor.checked_add(1)?;
        }
        if closing != (tag.kind == TagKind::EndTag) {
            continue;
        }
        let name_end = cursor.checked_add(name.len())?;
        if !html
            .get(cursor..name_end)
            .is_some_and(|s| s.eq_ignore_ascii_case(name))
        {
            continue;
        }
        if !html
            .as_bytes()
            .get(name_end)
            .is_some_and(|b| b.is_ascii_whitespace() || matches!(*b, b'/' | b'>'))
        {
            continue;
        }
        let mut quote = None;
        let mut tag_end = None;
        for (offset, byte) in html.as_bytes()[name_end..end].iter().copied().enumerate() {
            match (quote, byte) {
                (Some(q), b) if q == b => quote = None,
                (None, b'\'' | b'"') => quote = Some(byte),
                (None, b'>') => {
                    tag_end = Some(name_end + offset + 1);
                    break;
                }
                _ => {}
            }
        }
        if tag_end != Some(end) {
            continue;
        }
        if found.is_some() || end - start > MAX_HEAD_TAG_UTF8_BYTES {
            return None;
        }
        found = Some(start);
    }
    let start = found?;
    let mut excerpt_end = (start + MAX_HEAD_EXCERPT_UTF8_BYTES).min(end);
    while !html.is_char_boundary(excerpt_end) {
        excerpt_end -= 1;
    }
    let (mut line, mut column, mut cr) = (1usize, 1usize, false);
    for ch in html[..start].chars() {
        if ch == '\r' {
            line += 1;
            column = 1;
            cr = true;
        } else if ch == '\n' {
            if !cr {
                line += 1;
            }
            column = 1;
            cr = false;
        } else {
            column += 1;
            cr = false;
        }
    }
    Some(HtmlHeadSpan {
        element: name.to_owned(),
        start_utf8_byte: start,
        end_utf8_byte: end,
        line,
        unicode_column: column,
        raw_excerpt: html[start..excerpt_end].to_owned(),
        excerpt_truncated: excerpt_end != end,
    })
}

impl TokenSink for Sink<'_> {
    type Handle = ();
    fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<()> {
        let mut scan = self.scan.borrow_mut();
        if scan.done {
            return TokenSinkResult::Continue;
        }
        match token {
            Token::TagToken(tag) => {
                scan.tokens += 1;
                if scan.tokens > MAX_HEAD_TOKENS {
                    scan.problem = Some("token_budget");
                    scan.done = true;
                    return TokenSinkResult::Continue;
                }
                let from = self.previous_markup_end.get();
                self.previous_markup_end.set(self.fed_end.get());
                let span = || token_span(self.html, from, self.fed_end.get(), &tag);
                let name: &str = tag.name.as_ref();
                if tag.kind == TagKind::EndTag && scan.raw_element.as_deref() == Some(name) {
                    scan.raw_element = None;
                }
                if scan.head.is_none() {
                    if tag.kind == TagKind::StartTag && name == "head" {
                        let Some(position) = span() else {
                            scan.problem = Some("unresolved_source_lexeme");
                            scan.done = true;
                            return TokenSinkResult::Continue;
                        };
                        scan.head = Some(position);
                    } else if tag.kind == TagKind::StartTag && name != "html" {
                        scan.problem = Some("explicit_head_not_observed");
                        scan.done = true;
                    }
                } else if scan.template_depth != 0 {
                    if name == "template" {
                        if tag.kind == TagKind::StartTag {
                            scan.template_depth += 1;
                        } else {
                            scan.template_depth -= 1;
                        }
                    }
                } else if tag.kind == TagKind::EndTag && matches!(name, "head" | "body" | "html") {
                    scan.done = true;
                    scan.complete = true;
                    scan.boundary = "explicit_end_tag";
                } else if tag.kind == TagKind::StartTag && matches!(name, "body" | "frameset") {
                    // Body starts are legitimate omitted-head-end boundaries,
                    // not invalid head children or subsequent head metadata.
                    scan.done = true;
                    scan.complete = true;
                    scan.boundary = "body_start_boundary";
                } else if tag.kind == TagKind::StartTag && name != "head" {
                    if permitted(name) {
                        if scan.invalid.is_some() {
                            if scan.following.len() == MAX_HEAD_FOLLOWING {
                                scan.problem = Some("following_metadata_budget");
                                scan.done = true;
                            } else {
                                let Some(position) = span() else {
                                    scan.problem = Some("unresolved_source_lexeme");
                                    scan.done = true;
                                    return TokenSinkResult::Continue;
                                };
                                scan.following.push(position);
                            }
                        }
                        if name == "template" {
                            scan.template_depth = 1;
                        }
                    } else if scan.invalid.is_none() {
                        let Some(position) = span() else {
                            scan.problem = Some("unresolved_source_lexeme");
                            scan.done = true;
                            return TokenSinkResult::Continue;
                        };
                        scan.invalid = Some(position);
                    }
                    if matches!(name, "svg" | "math") {
                        scan.problem = Some("foreign_content_not_assessed");
                        scan.done = true;
                    }
                } else if tag.kind == TagKind::StartTag && name == "head" {
                    scan.problem = Some("nested_head_not_assessed");
                    scan.done = true;
                }
                if scan.template_depth > 32 {
                    scan.problem = Some("template_depth_budget");
                    scan.done = true;
                }
                if tag.kind == TagKind::StartTag {
                    let mode = match name {
                        "title" | "textarea" => Some(Rcdata),
                        "script" => Some(ScriptData),
                        "style" | "noscript" | "iframe" | "xmp" | "noembed" | "noframes" => {
                            Some(Rawtext)
                        }
                        _ => None,
                    };
                    if let Some(mode) = mode {
                        scan.raw_element = Some(name.to_owned());
                        return TokenSinkResult::RawData(mode);
                    }
                }
            }
            Token::CommentToken(_) | Token::DoctypeToken(_) => {
                self.previous_markup_end.set(self.fed_end.get())
            }
            Token::CharacterTokens(text) => {
                if scan.raw_element.is_none()
                    && scan.template_depth == 0
                    && scan.invalid.is_none()
                    && text
                        .chars()
                        .any(|c| !matches!(c, '\t' | '\n' | '\u{c}' | '\r' | ' '))
                {
                    // Text/entity recovery can implicitly end the parsed head.
                    // This rule diagnoses element children only, so do not
                    // misclassify text as a positioned invalid element.
                    scan.problem = Some(if scan.head.is_some() {
                        "non_element_head_content"
                    } else {
                        "non_whitespace_before_explicit_head"
                    });
                    scan.done = true;
                }
            }
            Token::NullCharacterToken | Token::ParseError(_) => {
                scan.problem = Some("tokenization_error")
            }
            Token::EOFToken => {
                scan.done = true;
                if scan.head.is_some() && scan.template_depth == 0 && scan.raw_element.is_none() {
                    scan.complete = true;
                    scan.boundary = "complete_source_eof";
                } else {
                    scan.problem = Some("head_or_raw_context_incomplete");
                }
            }
        }
        TokenSinkResult::Continue
    }
}

pub(crate) fn observe(
    page: &PageRecord,
    html: &str,
    decode: ContentCaptureDecodeStatus,
) -> HtmlHeadObservation {
    let mut result = HtmlHeadObservation {
        engine: HTML_HEAD_ENGINE.into(),
        parser_profile: HTML_HEAD_PROFILE.into(),
        source: "raw_http_decoded_utf8".into(),
        document_url: page.final_url.clone(),
        fetched_at: page.fetched_at.clone(),
        status: HtmlHeadStatus::Unknown,
        reason: "not_assessed".into(),
        decoded_source_utf8_bytes: html.len(),
        scanned_prefix_utf8_bytes: 0,
        scanned_prefix_sha256: format!("{:x}", Sha256::digest(b"")),
        explicit_head: None,
        first_invalid_child: None,
        following_metadata: Vec::new(),
    };
    if !page.media_type().eq_ignore_ascii_case("text/html") {
        result.reason = "html_text_media_type_required".into();
        return result;
    }
    if page.final_status != Some(200)
        || page.error.is_some()
        || page.truncated
        || page.blocked_by_robots
    {
        result.reason = "raw_http_response_incomplete_or_non200".into();
        return result;
    }
    if decode != ContentCaptureDecodeStatus::Exact {
        result.reason = "decoded_source_not_exact".into();
        return result;
    }
    let sink = Sink {
        html,
        fed_end: Cell::new(0),
        previous_markup_end: Cell::new(0),
        scan: RefCell::new(Scan::default()),
    };
    let tokenizer = Tokenizer::new(sink, Default::default());
    let input = BufferQueue::default();
    for (start, ch) in html.char_indices() {
        let end = start + ch.len_utf8();
        if end > MAX_HEAD_SCAN_UTF8_BYTES {
            break;
        }
        tokenizer.sink.fed_end.set(end);
        // Character-sized chunks bind emitted tag tokens to exact original
        // decoded byte positions. Total work and tokenizer buffering are capped.
        input.push_back(StrTendril::from_slice(&html[start..end]));
        let _ = tokenizer.feed(&input);
        if tokenizer.sink.scan.borrow().done {
            break;
        }
    }
    if !tokenizer.sink.scan.borrow().done && tokenizer.sink.fed_end.get() == html.len() {
        tokenizer.end();
    }
    let mut scan = tokenizer.sink.scan.borrow_mut();
    if !scan.done {
        scan.problem = Some("source_scan_budget");
    }
    result.scanned_prefix_utf8_bytes = tokenizer.sink.fed_end.get();
    result.scanned_prefix_sha256 = format!(
        "{:x}",
        Sha256::digest(&html.as_bytes()[..result.scanned_prefix_utf8_bytes])
    );
    result.explicit_head = scan.head.take();
    result.first_invalid_child = scan.invalid.take();
    result.following_metadata = std::mem::take(&mut scan.following);
    result.status = if scan.complete && scan.problem.is_none() {
        HtmlHeadStatus::Complete
    } else {
        HtmlHeadStatus::Unknown
    };
    result.reason = scan.problem.unwrap_or(scan.boundary).to_owned();
    result
}

fn location(span: &HtmlHeadSpan) -> String {
    format!(
        "<{}>, decoded UTF-8 bytes {}..{}, line {}, Unicode column {}: {}{}",
        span.element,
        span.start_utf8_byte,
        span.end_utf8_byte,
        span.line,
        span.unicode_column,
        span.raw_excerpt,
        if span.excerpt_truncated {
            " [excerpt truncated]"
        } else {
            ""
        }
    )
}

fn consistent_span(span: &HtmlHeadSpan, prefix_bytes: usize) -> bool {
    span.start_utf8_byte < span.end_utf8_byte
        && span.end_utf8_byte <= prefix_bytes
        && span.end_utf8_byte - span.start_utf8_byte <= MAX_HEAD_TAG_UTF8_BYTES
        && span.line > 0
        && span.unicode_column > 0
        && span.raw_excerpt.starts_with('<')
        && !span.element.is_empty()
        && span.element.len() <= MAX_HEAD_TAG_UTF8_BYTES
        && span.raw_excerpt.len() <= MAX_HEAD_EXCERPT_UTF8_BYTES
        && (if span.excerpt_truncated {
            span.raw_excerpt.len() < span.end_utf8_byte - span.start_utf8_byte
        } else {
            span.raw_excerpt.len() == span.end_utf8_byte - span.start_utf8_byte
        })
}

fn consistent(observation: &HtmlHeadObservation) -> bool {
    let limit = observation.scanned_prefix_utf8_bytes;
    if limit > MAX_HEAD_SCAN_UTF8_BYTES
        || limit > observation.decoded_source_utf8_bytes
        || observation.scanned_prefix_sha256.len() != 64
        || !observation
            .scanned_prefix_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        || observation.following_metadata.len() > MAX_HEAD_FOLLOWING
    {
        return false;
    }
    let Some(head) = &observation.explicit_head else {
        return false;
    };
    if head.element != "head" || !consistent_span(head, limit) {
        return false;
    }
    let Some(invalid) = &observation.first_invalid_child else {
        return observation.following_metadata.is_empty();
    };
    if permitted(&invalid.element)
        || matches!(invalid.element.as_str(), "head" | "body" | "frameset")
        || !consistent_span(invalid, limit)
        || invalid.start_utf8_byte < head.end_utf8_byte
    {
        return false;
    }
    let mut after = invalid.end_utf8_byte;
    for span in &observation.following_metadata {
        if !permitted(&span.element)
            || !consistent_span(span, limit)
            || span.start_utf8_byte < after
        {
            return false;
        }
        after = span.end_utf8_byte;
    }
    true
}

pub(crate) fn audit(page: &PageRecord, issues: &mut Vec<AuditIssue>) {
    let Some(observation) = &page.html_head_observation else {
        return;
    };
    if !page.internal
        || observation.engine != HTML_HEAD_ENGINE
        || observation.parser_profile != HTML_HEAD_PROFILE
        || observation.source != "raw_http_decoded_utf8"
        || observation.document_url != page.final_url
        || observation.fetched_at != page.fetched_at
        || observation.status != HtmlHeadStatus::Complete
        || !consistent(observation)
    {
        return;
    }
    let Some(invalid) = &observation.first_invalid_child else {
        return;
    };
    let following = &observation.following_metadata;
    let (code, severity, tail) = if following.is_empty() {
        ("html_head_invalid_child", Severity::Notice, "No following head metadata element was observed in this bounded explicit-head source window.".to_owned())
    } else {
        (
            "html_head_metadata_after_invalid_child",
            Severity::Warning,
            format!(
                "Following raw head metadata may be ignored under Google's documented head rule: {}",
                following
                    .iter()
                    .map(location)
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        )
    };
    issues.push(AuditIssue { code: code.into(), severity, url: page.url.clone(),
        message: format!("Invalid child in the captured raw HTTP head: {}. {} Source-prefix SHA256 {}; source URL {}. This is stored source evidence, not proof of Google processing or the rendered DOM.", location(invalid), tail, observation.scanned_prefix_sha256, observation.document_url), related_urls: Vec::new() });
}

#[cfg(test)]
#[path = "html_head_tests.rs"]
mod tests;
