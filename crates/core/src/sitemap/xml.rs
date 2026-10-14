use super::count_models::*;
use super::models::*;
use quick_xml::{Reader, events::Event};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

type XmlResult<T> = Result<T, SitemapFailure>;
fn fail(code: &str, message: impl Into<String>, at: usize) -> SitemapFailure {
    SitemapFailure::new(code, message, Some(at as u64))
}
fn xml_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}
fn xml_char(ch: char) -> bool {
    matches!(ch as u32, 0x9 | 0xa | 0xd | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff)
}
fn qname(name: &str, at: usize) -> XmlResult<(Option<&str>, &str)> {
    // Explicit bounded ASCII name profile, not an XML Unicode-name validator.
    let mut pieces = name.split(':');
    let first = pieces.next().unwrap_or_default();
    let second = pieces.next();
    if pieces.next().is_some() {
        return Err(fail(
            "sitemap_unsupported_xml_name",
            "QName has multiple colons",
            at,
        ));
    }
    for piece in [Some(first), second].into_iter().flatten() {
        let bytes = piece.as_bytes();
        if bytes.is_empty() || !matches!(bytes[0], b'A'..=b'Z' | b'a'..=b'z' | b'_')
            || bytes[1..].iter().any(|byte| !matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.')) {
            return Err(fail("sitemap_unsupported_xml_name", "XML name is outside ascii-qname-v1", at));
        }
    }
    Ok(match second {
        Some(local) => (Some(first), local),
        None => (None, first),
    })
}
fn field(value: &str, limits: &SitemapParserLimits, at: usize) -> XmlResult<()> {
    if value.len() > limits.max_field_utf8_bytes {
        return Err(fail(
            "sitemap_field_byte_budget",
            "XML field exceeds UTF-8 byte budget",
            at,
        ));
    }
    if !value.chars().all(xml_char) {
        return Err(fail(
            "sitemap_illegal_xml_character",
            "Field contains an illegal XML1.0 character",
            at,
        ));
    }
    Ok(())
}

/// quick-xml attribute iteration alone does not prove separators/QName/value grammar.
/// Normalize namespace declarations ourselves rather than NsReader's raw URI binding.
fn tag_parts(
    raw: &str,
    limits: &SitemapParserLimits,
    at: usize,
) -> XmlResult<(String, Vec<(String, String)>)> {
    field(raw, limits, at)?;
    let bytes = raw.as_bytes();
    let mut index = bytes
        .iter()
        .position(|byte| xml_space(*byte))
        .unwrap_or(bytes.len());
    let name = &raw[..index];
    qname(name, at)?;
    let mut keys = BTreeSet::new();
    let mut attributes = Vec::new();
    while index < bytes.len() {
        if !xml_space(bytes[index]) {
            return Err(fail(
                "sitemap_invalid_attribute",
                "Attribute requires whitespace separator",
                at + index,
            ));
        }
        while index < bytes.len() && xml_space(bytes[index]) {
            index += 1;
        }
        if index == bytes.len() {
            break;
        }
        let start = index;
        while index < bytes.len() && !xml_space(bytes[index]) && bytes[index] != b'=' {
            index += 1;
        }
        let key = &raw[start..index];
        qname(key, at + start)?;
        if !keys.insert(key.to_owned()) {
            return Err(fail(
                "sitemap_duplicate_attribute",
                "Duplicate lexical attribute name",
                at + start,
            ));
        }
        while index < bytes.len() && xml_space(bytes[index]) {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            return Err(fail(
                "sitemap_invalid_attribute",
                "Attribute lacks equals sign",
                at + index,
            ));
        }
        index += 1;
        while index < bytes.len() && xml_space(bytes[index]) {
            index += 1;
        }
        let quote = *bytes.get(index).ok_or_else(|| {
            fail(
                "sitemap_invalid_attribute",
                "Attribute lacks a value",
                at + index,
            )
        })?;
        if !matches!(quote, b'\'' | b'"') {
            return Err(fail(
                "sitemap_invalid_attribute",
                "Attribute value must be quoted",
                at + index,
            ));
        }
        index += 1;
        let value_start = index;
        while index < bytes.len() && bytes[index] != quote {
            index += 1;
        }
        if index == bytes.len() {
            return Err(fail(
                "sitemap_invalid_attribute",
                "Unclosed attribute value",
                at + value_start,
            ));
        }
        let lexical = &raw[value_start..index];
        if lexical.contains('<') {
            return Err(fail(
                "sitemap_invalid_attribute",
                "Literal less-than in attribute value",
                at + value_start,
            ));
        }
        // Literal XML whitespace is normalized BEFORE reference expansion;
        // a character-reference LF remains a LF rather than becoming a space.
        let normalized = lexical
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace(['\n', '\t'], " ");
        let decoded =
            quick_xml::escape::unescape_with(&normalized, quick_xml::escape::resolve_xml_entity)
                .map_err(|error| {
                    fail(
                        "sitemap_invalid_entity",
                        error.to_string(),
                        at + value_start,
                    )
                })?
                .into_owned();
        field(&decoded, limits, at + value_start)?;
        attributes.push((key.into(), decoded));
        index += 1;
    }
    Ok((name.into(), attributes))
}

#[derive(Clone)]
struct Frame {
    qualified: String,
    local: String,
    namespace: Option<String>,
    bindings: BTreeMap<String, String>,
}
fn resolve(
    name: &str,
    bindings: &BTreeMap<String, String>,
    attribute: bool,
    at: usize,
) -> XmlResult<(Option<String>, String)> {
    let (prefix, local) = qname(name, at)?;
    let namespace = if let Some(prefix) = prefix {
        if prefix == "xmlns" {
            return Err(fail(
                "sitemap_invalid_namespace",
                "Reserved xmlns prefix is not an ordinary QName",
                at,
            ));
        }
        Some(
            bindings
                .get(prefix)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    fail(
                        "sitemap_unbound_namespace_prefix",
                        "QName prefix has no namespace binding",
                        at,
                    )
                })?
                .clone(),
        )
    } else if attribute {
        None
    } else {
        bindings.get("").filter(|value| !value.is_empty()).cloned()
    };
    Ok((namespace, local.into()))
}
fn frame(
    qualified: String,
    attributes: Vec<(String, String)>,
    previous: Option<&Frame>,
    at: usize,
) -> XmlResult<Frame> {
    let mut bindings = previous
        .map(|value| value.bindings.clone())
        .unwrap_or_else(|| BTreeMap::from([("xml".into(), XML_NS.into())]));
    for (key, value) in &attributes {
        let prefix = if key == "xmlns" {
            Some("")
        } else {
            key.strip_prefix("xmlns:")
        };
        if let Some(prefix) = prefix {
            if prefix == "xmlns"
                || value == XMLNS_NS
                || (prefix == "xml" && value != XML_NS)
                || (prefix != "xml" && value == XML_NS)
                || (!prefix.is_empty() && value.is_empty())
            {
                return Err(fail(
                    "sitemap_invalid_namespace",
                    "Invalid reserved/empty namespace declaration",
                    at,
                ));
            }
            bindings.insert(prefix.into(), value.clone());
        }
    }
    let mut expanded = BTreeSet::new();
    for (key, _) in attributes
        .iter()
        .filter(|(key, _)| key != "xmlns" && !key.starts_with("xmlns:"))
    {
        if !expanded.insert(resolve(key, &bindings, true, at)?) {
            return Err(fail(
                "sitemap_duplicate_expanded_attribute",
                "Two attributes have the same expanded name",
                at,
            ));
        }
    }
    let (namespace, local) = resolve(&qualified, &bindings, false, at)?;
    Ok(Frame {
        qualified,
        local,
        namespace,
        bindings,
    })
}
fn family(frame: &Frame, profile: &SitemapNamespaceProfile) -> bool {
    match profile {
        SitemapNamespaceProfile::Sitemap09 => frame.namespace.as_deref() == Some(SITEMAP_XML_NS),
        SitemapNamespaceProfile::LegacyUnqualified => frame.namespace.is_none(),
    }
}
enum CaptureKind {
    Loc,
    Lastmod,
}
struct Capture {
    kind: CaptureKind,
    start: usize,
    inner_start: usize,
    decoded: String,
}
struct Entry {
    ordinal: usize,
    start: usize,
    locs: Vec<usize>,
    lastmod: Vec<SitemapFieldOccurrence>,
}
// Count-only sink owns no retained row arrays or per-entry vectors.
#[derive(Default)]
struct CountSink {
    facts: SitemapXmlCountFacts,
    active_entry: Option<CountEntry>,
}
#[derive(Default)]
struct CountEntry {
    actual_locs: u64,
    first_location_eligible: bool,
}
#[derive(Clone, Copy)]
enum CountLoc {
    Eligible,
    Empty,
    CharacterLimit,
    Credentials,
    MalformedPercent,
    Fragment,
    InvalidAbsoluteHttp,
}
fn count_loc(value: &str) -> CountLoc {
    // Only XML S is formatting; do not let URL parsing silently trim/repair it.
    let candidate = value.trim_matches(|ch| matches!(ch, ' ' | '\t' | '\r' | '\n'));
    if candidate.is_empty() {
        return CountLoc::Empty;
    }
    if candidate.chars().count() >= 2048 {
        return CountLoc::CharacterLimit;
    }
    if candidate.chars().any(|ch| {
        ch.is_control()
            || ch.is_whitespace()
            || matches!(ch, '\\' | '<' | '>' | '"' | '{' | '}' | '|' | '^')
            || ch == char::from(96u8)
    }) {
        return CountLoc::InvalidAbsoluteHttp;
    }
    let bytes = candidate.as_bytes();
    for (at, byte) in bytes.iter().enumerate() {
        if *byte == b'%'
            && !(bytes.get(at + 1).is_some_and(u8::is_ascii_hexdigit)
                && bytes.get(at + 2).is_some_and(u8::is_ascii_hexdigit))
        {
            return CountLoc::MalformedPercent;
        }
    }
    let Some((scheme, rest)) = candidate.split_once("://") else {
        return CountLoc::InvalidAbsoluteHttp;
    };
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")) {
        return CountLoc::InvalidAbsoluteHttp;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        return CountLoc::Credentials;
    }
    if authority.is_empty() {
        return CountLoc::InvalidAbsoluteHttp;
    }
    if candidate.contains('#') {
        return CountLoc::Fragment;
    }
    // URL parsers may accept raw square brackets and other repairs in a path.
    // The ASCII path/query profile is RFC3986 pchar plus '/' and '?'; brackets
    // remain available only to the parsed authority's IPv6 host syntax.
    if rest[authority.len()..].chars().any(|ch| {
        ch.is_ascii()
            && !(ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '-' | '.'
                        | '_'
                        | '~'
                        | '!'
                        | '$'
                        | '&'
                        | '\''
                        | '('
                        | ')'
                        | '*'
                        | '+'
                        | ','
                        | ';'
                        | '='
                        | ':'
                        | '@'
                        | '/'
                        | '?'
                        | '%'
                ))
    }) {
        return CountLoc::InvalidAbsoluteHttp;
    }
    match url::Url::parse(candidate) {
        Ok(url)
            if matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none() =>
        {
            CountLoc::Eligible
        }
        _ => CountLoc::InvalidAbsoluteHttp,
    }
}
struct Inspection<'a> {
    source: &'a str,
    limits: &'a SitemapParserLimits,
    observation: SitemapParseObservation,
    stack: Vec<Frame>,
    root_closed: bool,
    declaration_seen: bool,
    active_entry: Option<Entry>,
    count_sink: Option<CountSink>,
    capture: Option<Capture>,
}
impl Inspection<'_> {
    fn entry_active(&self) -> bool {
        self.count_sink
            .as_ref()
            .map_or(self.active_entry.is_some(), |sink| {
                sink.active_entry.is_some()
            })
    }
    fn finish_count(&mut self, end_start: usize, end: usize, empty: bool) -> XmlResult<()> {
        if self.stack.len() == 3 && self.capture.is_some() {
            let capture = self.capture.take().expect("checked count capture");
            // Borrow source spans: raw fields are bounded but never copied into a retained row.
            let raw_element = &self.source[capture.start..end];
            let raw_inner = if empty {
                ""
            } else {
                &self.source[capture.inner_start..end_start]
            };
            field(raw_element, self.limits, capture.start)?;
            field(raw_inner, self.limits, capture.start)?;
            if matches!(capture.kind, CaptureKind::Loc) {
                let sink = self.count_sink.as_mut().expect("count sink");
                sink.facts.actual_document_namespace_loc_occurrences += 1;
                let candidate = capture
                    .decoded
                    .trim_matches(|ch| matches!(ch, ' ' | '\t' | '\r' | '\n'));
                if candidate != capture.decoded {
                    sink.facts.xml_space_trimmed_location_occurrences += 1;
                }
                let assessment = count_loc(&capture.decoded);
                let eligible = matches!(assessment, CountLoc::Eligible);
                if !eligible {
                    sink.facts.invalid_location_occurrences += 1;
                    match assessment {
                        CountLoc::Empty => sink.facts.empty_location_occurrences += 1,
                        CountLoc::CharacterLimit => {
                            sink.facts.over_character_limit_location_occurrences += 1
                        }
                        CountLoc::Credentials => sink.facts.credential_location_occurrences += 1,
                        CountLoc::MalformedPercent => {
                            sink.facts.malformed_percent_location_occurrences += 1
                        }
                        CountLoc::Fragment => sink.facts.fragment_location_occurrences += 1,
                        CountLoc::InvalidAbsoluteHttp => {
                            sink.facts.invalid_absolute_http_location_occurrences += 1
                        }
                        CountLoc::Eligible => unreachable!("checked assessment"),
                    }
                }
                let entry = sink.active_entry.as_mut().expect("count entry capture");
                if entry.actual_locs == 1 {
                    entry.first_location_eligible = eligible;
                }
            }
        }
        if self.stack.len() == 2 {
            let sink = self.count_sink.as_mut().expect("count sink");
            if let Some(entry) = sink.active_entry.take() {
                sink.facts.completed_immediate_entries += 1;
                match entry.actual_locs {
                    0 => sink.facts.missing_location_entries += 1,
                    1 if entry.first_location_eligible => {
                        sink.facts.protocol_eligible_completed_entries += 1
                    }
                    1 => sink.facts.invalid_single_location_entries += 1,
                    _ => sink.facts.multiple_location_entries += 1,
                }
            }
        }
        self.stack
            .pop()
            .ok_or_else(|| fail("sitemap_unbalanced_end", "Unmatched XML end", end_start))?;
        if self.stack.is_empty() {
            self.root_closed = true;
        }
        Ok(())
    }
    fn retain(&mut self, bytes: usize, at: usize) -> XmlResult<()> {
        let total = self
            .observation
            .retained_text_utf8_bytes
            .checked_add(bytes)
            .ok_or_else(|| {
                fail(
                    "sitemap_retained_text_byte_budget",
                    "Retained text count overflow",
                    at,
                )
            })?;
        if total > self.limits.max_retained_text_utf8_bytes {
            return Err(fail(
                "sitemap_retained_text_byte_budget",
                "Retained input-derived string bytes exceed budget",
                at,
            ));
        }
        self.observation.retained_text_utf8_bytes = total;
        Ok(())
    }
    fn begin(&mut self, current: Frame, start: usize, end: usize, empty: bool) -> XmlResult<()> {
        if self.stack.len() >= self.limits.max_xml_depth {
            return Err(fail(
                "sitemap_xml_depth_budget",
                "XML depth exceeds budget",
                start,
            ));
        }
        if self.capture.is_some() {
            return Err(fail(
                "sitemap_nested_field_markup",
                "Nested markup inside loc/lastmod is unsupported",
                start,
            ));
        }
        if self.stack.is_empty() {
            if self.root_closed || self.observation.kind.is_some() {
                return Err(fail(
                    "sitemap_multiple_roots",
                    "XML must have exactly one root",
                    start,
                ));
            }
            self.observation.kind = Some(match current.local.as_str() {
                "urlset" => SitemapDocumentKind::UrlSet,
                "sitemapindex" => SitemapDocumentKind::SitemapIndex,
                _ => {
                    return Err(fail(
                        "sitemap_invalid_root",
                        "Expected urlset or sitemapindex root",
                        start,
                    ));
                }
            });
            self.observation.namespace_profile = Some(match current.namespace.as_deref() {
                Some(SITEMAP_XML_NS) => SitemapNamespaceProfile::Sitemap09,
                None if self.limits.allow_legacy_unqualified => {
                    SitemapNamespaceProfile::LegacyUnqualified
                }
                _ => {
                    return Err(fail(
                        "sitemap_invalid_root_namespace",
                        "Root namespace does not match the selected sitemap profile",
                        start,
                    ));
                }
            });
        } else {
            let recognized = family(
                &current,
                self.observation
                    .namespace_profile
                    .as_ref()
                    .expect("root profile"),
            );
            let entry_name = if self.observation.kind == Some(SitemapDocumentKind::SitemapIndex) {
                "sitemap"
            } else {
                "url"
            };
            if let Some(sink) = &mut self.count_sink {
                if !recognized && current.local == entry_name {
                    sink.facts.ignored_nonfamily_entry_elements += 1;
                }
                if !recognized && current.local == "loc" {
                    sink.facts.ignored_nonfamily_loc_elements += 1;
                }
            }
            if recognized && current.local == entry_name && self.stack.len() == 1 {
                if let Some(sink) = &mut self.count_sink {
                    sink.facts.immediate_entry_starts += 1;
                    sink.active_entry = Some(CountEntry::default());
                } else {
                    if self.observation.entries.len() >= self.limits.max_entries {
                        return Err(fail(
                            "sitemap_entry_budget",
                            "Entry count exceeds budget",
                            start,
                        ));
                    }
                    self.active_entry = Some(Entry {
                        ordinal: self.observation.entries.len() + 1,
                        start,
                        locs: Vec::new(),
                        lastmod: Vec::new(),
                    });
                }
            } else if recognized
                && matches!(current.local.as_str(), "loc" | "lastmod")
                && self.stack.len() == 2
                && self.entry_active()
            {
                if current.local == "loc"
                    && let Some(sink) = &mut self.count_sink
                {
                    sink.facts.started_document_namespace_loc_elements += 1;
                    sink.active_entry.as_mut().expect("count entry").actual_locs += 1;
                }
                self.capture = Some(Capture {
                    kind: if current.local == "loc" {
                        CaptureKind::Loc
                    } else {
                        CaptureKind::Lastmod
                    },
                    start,
                    inner_start: end,
                    decoded: String::new(),
                });
            } else if recognized
                && matches!(
                    current.local.as_str(),
                    "url" | "sitemap" | "loc" | "lastmod" | "urlset" | "sitemapindex"
                )
            {
                if let Some(sink) = &mut self.count_sink {
                    sink.facts.misplaced_family_elements += 1;
                }
                return Err(fail(
                    "sitemap_invalid_element_parent",
                    "Recognized sitemap element is not an immediate child of its expected parent",
                    start,
                ));
            }
        }
        self.stack.push(current);
        if empty {
            self.finish(end, end, true)?;
        }
        Ok(())
    }
    fn text(&mut self, text: &str, at: usize) -> XmlResult<()> {
        if !text.chars().all(xml_char) {
            return Err(fail(
                "sitemap_illegal_xml_character",
                "Decoded text contains an illegal XML1.0 character",
                at,
            ));
        }
        if let Some(capture) = &mut self.capture {
            let length = capture
                .decoded
                .len()
                .checked_add(text.len())
                .ok_or_else(|| {
                    fail("sitemap_field_byte_budget", "Field byte count overflow", at)
                })?;
            if length > self.limits.max_field_utf8_bytes {
                return Err(fail(
                    "sitemap_field_byte_budget",
                    "Decoded XML field exceeds byte budget",
                    at,
                ));
            }
            capture.decoded.push_str(text);
        } else if self.stack.is_empty() && !text.as_bytes().iter().all(|byte| xml_space(*byte)) {
            return Err(fail(
                "sitemap_text_outside_root",
                "Nonwhitespace text outside document root",
                at,
            ));
        }
        Ok(())
    }
    fn row_capacity(&self, at: usize) -> XmlResult<()> {
        if self.observation.locations.len() >= self.limits.max_location_occurrences {
            return Err(fail(
                "sitemap_location_budget",
                "Location/diagnostic occurrence count exceeds budget",
                at,
            ));
        }
        Ok(())
    }
    fn finish(&mut self, end_start: usize, end: usize, empty: bool) -> XmlResult<()> {
        if self.count_sink.is_some() {
            return self.finish_count(end_start, end, empty);
        }
        if self.stack.len() == 3 && self.capture.is_some() {
            let capture = self.capture.take().expect("checked capture");
            let raw_element = self.source[capture.start..end].to_owned();
            let raw_inner = if empty {
                String::new()
            } else {
                self.source[capture.inner_start..end_start].to_owned()
            };
            field(&raw_element, self.limits, capture.start)?;
            field(&raw_inner, self.limits, capture.start)?;
            let mut retained = raw_element.len() + raw_inner.len() + capture.decoded.len();
            match capture.kind {
                CaptureKind::Loc => {
                    self.row_capacity(capture.start)?;
                    // Explicit loc edge profile: XML S is formatting, including
                    // decoded tab/LF references. Preserve it in decoded_value.
                    // Do not let normalize_url's Unicode trim erase NBSP/other
                    // non-XML whitespace edges. Internal controls never reach it.
                    let candidate = capture
                        .decoded
                        .trim_matches(|ch| matches!(ch, ' ' | '\t' | '\r' | '\n'));
                    let normalized = if candidate.trim() == candidate
                        && !candidate.chars().any(char::is_control)
                    {
                        crate::normalize_url(candidate, None).map(|value| value.to_string())
                    } else {
                        None
                    };
                    if let Some(url) = &normalized {
                        retained += url.len();
                    }
                    self.retain(retained, capture.start)?;
                    let entry = self.active_entry.as_mut().expect("entry capture");
                    let ordinal = self.observation.locations.len() + 1;
                    entry.locs.push(ordinal);
                    self.observation.locations.push(SitemapLocationOccurrence {
                        ordinal,
                        entry_ordinal: entry.ordinal,
                        loc_ordinal: Some(entry.locs.len()),
                        role: if self.observation.kind == Some(SitemapDocumentKind::SitemapIndex) {
                            SitemapLocationRole::ChildSitemap
                        } else {
                            SitemapLocationRole::PageLocation
                        },
                        xml_byte_start: capture.start as u64,
                        xml_byte_end: end as u64,
                        raw_element_xml: Some(raw_element),
                        raw_inner_xml: Some(raw_inner),
                        validation_error: if candidate.is_empty() {
                            Some("empty_location".into())
                        } else if normalized.is_none() {
                            Some("invalid_absolute_http_location".into())
                        } else {
                            None
                        },
                        decoded_value: Some(capture.decoded),
                        normalized_fetch_url: normalized,
                    });
                }
                CaptureKind::Lastmod => {
                    self.retain(retained, capture.start)?;
                    let entry = self.active_entry.as_mut().expect("entry capture");
                    entry.lastmod.push(SitemapFieldOccurrence {
                        ordinal: entry.lastmod.len() + 1,
                        xml_byte_start: capture.start as u64,
                        xml_byte_end: end as u64,
                        raw_element_xml: raw_element,
                        raw_inner_xml: raw_inner,
                        decoded_value: capture.decoded,
                    });
                }
            }
        }
        if self.stack.len() == 2 && self.active_entry.is_some() {
            let pending = self.active_entry.as_ref().expect("entry");
            if pending.locs.is_empty() {
                self.row_capacity(pending.start)?;
            }
            let entry = self.active_entry.take().expect("entry");
            let mut diagnostics = Vec::new();
            if entry.locs.is_empty() {
                diagnostics.push("missing_location".into());
                self.observation.locations.push(SitemapLocationOccurrence {
                    ordinal: self.observation.locations.len() + 1,
                    entry_ordinal: entry.ordinal,
                    loc_ordinal: None,
                    role: if self.observation.kind == Some(SitemapDocumentKind::SitemapIndex) {
                        SitemapLocationRole::ChildSitemap
                    } else {
                        SitemapLocationRole::PageLocation
                    },
                    xml_byte_start: entry.start as u64,
                    xml_byte_end: end as u64,
                    raw_element_xml: None,
                    raw_inner_xml: None,
                    decoded_value: None,
                    normalized_fetch_url: None,
                    validation_error: Some("missing_location".into()),
                });
            }
            let ambiguous = entry.locs.len() > 1;
            if ambiguous {
                diagnostics.push("ambiguous_multiple_locations".into());
                for ordinal in &entry.locs {
                    self.observation.locations[*ordinal - 1].validation_error =
                        Some("ambiguous_multiple_locations".into());
                }
            }
            self.observation.entries.push(SitemapEntryObservation {
                ordinal: entry.ordinal,
                xml_byte_start: entry.start as u64,
                xml_byte_end: end as u64,
                complete: true,
                loc_occurrence_ordinals: entry.locs,
                lastmod: entry.lastmod,
                diagnostics,
            });
            if ambiguous {
                return Err(fail(
                    "sitemap_ambiguous_multiple_locations",
                    "Entry contains multiple immediate loc elements; all observed occurrences retained without admission",
                    entry.start,
                ));
            }
        }
        self.stack
            .pop()
            .ok_or_else(|| fail("sitemap_unbalanced_end", "Unmatched XML end", end_start))?;
        if self.stack.is_empty() {
            self.root_closed = true;
        }
        Ok(())
    }
    fn run(&mut self) -> XmlResult<()> {
        // Reader offsets are re-bound to original byte input, retaining BOM offset.
        let reader_source = self.source.strip_prefix('\u{feff}').unwrap_or(self.source);
        let base = self.source.len() - reader_source.len();
        let mut reader = Reader::from_str(reader_source);
        reader.config_mut().trim_text(false);
        reader.config_mut().expand_empty_elements = false;
        loop {
            let start = base + reader.buffer_position() as usize;
            let event = reader
                .read_event()
                .map_err(|error| fail("sitemap_invalid_xml", error.to_string(), start))?;
            let end = base + reader.buffer_position() as usize;
            self.observation.events_observed += 1;
            if self.observation.events_observed > self.limits.max_xml_events {
                return Err(fail(
                    "sitemap_xml_event_budget",
                    "XML event count exceeds budget",
                    start,
                ));
            }
            match event {
                Event::Start(_) | Event::Empty(_) => {
                    let empty = matches!(event, Event::Empty(_));
                    let raw = self.source[start..end]
                        .strip_prefix('<')
                        .and_then(|value| {
                            if empty {
                                value.strip_suffix("/>")
                            } else {
                                value.strip_suffix('>')
                            }
                        })
                        .ok_or_else(|| {
                            fail(
                                "sitemap_invalid_xml_span",
                                "Unexpected start-tag source span",
                                start,
                            )
                        })?;
                    let (name, attributes) = tag_parts(raw, self.limits, start + 1)?;
                    let current = frame(name, attributes, self.stack.last(), start)?;
                    self.begin(current, start, end, empty)?;
                }
                Event::End(tag) => {
                    let expected = &self
                        .stack
                        .last()
                        .ok_or_else(|| fail("sitemap_unbalanced_end", "Unexpected end tag", start))?
                        .qualified;
                    if tag.name().as_ref() != expected.as_str() {
                        return Err(fail(
                            "sitemap_mismatched_end",
                            "End tag differs from its start tag",
                            start,
                        ));
                    }
                    self.finish(start, end, false)?;
                }
                Event::Text(text) => {
                    if text.as_ref().contains("]]>") {
                        return Err(fail(
                            "sitemap_invalid_character_data",
                            "CDATA close delimiter inside ordinary character data",
                            start,
                        ));
                    }
                    self.text(&text.xml10_content(), start)?;
                }
                Event::CData(text) => {
                    if self.stack.is_empty() {
                        return Err(fail(
                            "sitemap_cdata_outside_root",
                            "CDATA outside root",
                            start,
                        ));
                    }
                    self.text(&text.xml10_content(), start)?;
                }
                Event::GeneralRef(reference) => {
                    if self.stack.is_empty() {
                        return Err(fail(
                            "sitemap_entity_outside_root",
                            "Entity outside root",
                            start,
                        ));
                    }
                    let value = if let Some(ch) = reference
                        .resolve_char_ref()
                        .map_err(|error| fail("sitemap_invalid_entity", error.to_string(), start))?
                    {
                        ch.to_string()
                    } else {
                        match reference.as_ref() {
                            "amp" => "&",
                            "lt" => "<",
                            "gt" => ">",
                            "quot" => "\"",
                            "apos" => "'",
                            _ => {
                                return Err(fail(
                                    "sitemap_invalid_entity",
                                    "Unknown XML entity; declarations/expansion unsupported",
                                    start,
                                ));
                            }
                        }
                        .into()
                    };
                    self.text(&value, start)?;
                }
                Event::Decl(_) => {
                    if self.declaration_seen || self.observation.events_observed != 1 {
                        return Err(fail(
                            "sitemap_invalid_xml_declaration",
                            "XML declaration must occur once at the beginning",
                            start,
                        ));
                    }
                    let raw = self.source[start..end]
                        .strip_prefix("<?")
                        .and_then(|value| value.strip_suffix("?>"))
                        .ok_or_else(|| {
                            fail(
                                "sitemap_invalid_xml_declaration",
                                "Unexpected declaration span",
                                start,
                            )
                        })?;
                    if raw.contains('&') {
                        return Err(fail(
                            "sitemap_invalid_xml_declaration",
                            "References are not permitted in XML declaration pseudo-attributes",
                            start,
                        ));
                    }
                    let (name, attributes) = tag_parts(raw, self.limits, start + 2)?;
                    if name != "xml"
                        || attributes
                            .first()
                            .map(|(key, value)| (key.as_str(), value.as_str()))
                            != Some(("version", "1.0"))
                    {
                        return Err(fail(
                            "sitemap_invalid_xml_declaration",
                            "Only XML1.0 declarations are supported",
                            start,
                        ));
                    }
                    let mut seen_standalone = false;
                    for (key, value) in attributes.iter().skip(1) {
                        match key.as_str() {
                            "encoding"
                                if !seen_standalone && value.eq_ignore_ascii_case("utf-8") => {}
                            "standalone" if matches!(value.as_str(), "yes" | "no") => {
                                seen_standalone = true
                            }
                            _ => {
                                return Err(fail(
                                    "sitemap_invalid_xml_declaration",
                                    "Only UTF-8 and valid ordered standalone declaration fields are supported",
                                    start,
                                ));
                            }
                        }
                    }
                    self.declaration_seen = true;
                }
                Event::DocType(_) => {
                    return Err(fail(
                        "sitemap_dtd_unsupported",
                        "DTD/entity declarations are unsupported",
                        start,
                    ));
                }
                Event::Comment(comment) => {
                    if comment.as_ref().contains("--") || comment.as_ref().ends_with('-') {
                        return Err(fail(
                            "sitemap_invalid_xml_comment",
                            "Illegal XML comment delimiter",
                            start,
                        ));
                    }
                }
                Event::PI(_) => {
                    let raw = self.source[start..end]
                        .strip_prefix("<?")
                        .and_then(|value| value.strip_suffix("?>"))
                        .ok_or_else(|| {
                            fail(
                                "sitemap_invalid_processing_instruction",
                                "Unexpected PI source span",
                                start,
                            )
                        })?;
                    let name = raw
                        .split(|ch: char| ch.is_ascii_whitespace())
                        .next()
                        .unwrap_or_default();
                    qname(name, start)?;
                    if name.eq_ignore_ascii_case("xml") {
                        return Err(fail(
                            "sitemap_invalid_processing_instruction",
                            "Reserved XML processing target",
                            start,
                        ));
                    }
                }
                Event::Eof => break,
            }
        }
        if self.observation.kind.is_none()
            || !self.root_closed
            || !self.stack.is_empty()
            || self.capture.is_some()
            || self.entry_active()
        {
            return Err(fail(
                "sitemap_incomplete_xml",
                "Empty or incomplete sitemap document",
                self.source.len(),
            ));
        }
        Ok(())
    }
}

/// Complete document is staged before admission. Error observations retain only
/// already closed fields/entries as incomplete evidence and expose no candidates.
pub fn observe_sitemap_xml(
    input: &[u8],
    context: SitemapParserContext,
    limits: &SitemapParserLimits,
) -> XmlResult<SitemapParseObservation> {
    limits.validate()?;
    if input.len() > limits.max_xml_bytes {
        return Err(SitemapFailure::new(
            "sitemap_xml_byte_budget",
            "Actual XML bytes exceed budget",
            None,
        ));
    }
    let source = std::str::from_utf8(input).map_err(|_| {
        SitemapFailure::new(
            "sitemap_invalid_utf8",
            "Sitemap parser requires exact UTF-8 bytes",
            None,
        )
    })?;
    if !source.chars().all(xml_char) {
        return Err(SitemapFailure::new(
            "sitemap_illegal_xml_character",
            "Raw input contains an illegal XML1.0 character",
            None,
        ));
    }
    let mut context_bytes = 0usize;
    for value in [
        &context.requested_document_url,
        &context.final_document_url,
        &context.originating_occurrence_id,
    ]
    .into_iter()
    .flatten()
    {
        field(value, limits, 0)?;
        context_bytes += value.len();
    }
    let mut inspection = Inspection {
        source,
        limits,
        stack: Vec::new(),
        root_closed: false,
        declaration_seen: false,
        active_entry: None,
        count_sink: None,
        capture: None,
        observation: SitemapParseObservation {
            schema_version: 1,
            engine: SITEMAP_XML_ENGINE.into(),
            context_basis: "caller_declared_parser_context".into(),
            context,
            xml_bytes: input.len(),
            xml_sha256: format!("{:x}", Sha256::digest(input)),
            xml_sha_profile: "decoded-sitemap-xml-bytes-v1".into(),
            namespace_profile: None,
            xml_name_profile: "ascii-qname-v1".into(),
            kind: None,
            events_observed: 0,
            retained_text_utf8_bytes: 0,
            complete: false,
            failure: None,
            entries: Vec::new(),
            locations: Vec::new(),
        },
    };
    inspection.retain(context_bytes, 0)?;
    match inspection.run() {
        Ok(()) => inspection.observation.complete = true,
        Err(error) => {
            // Retain already closed fields in an incomplete entry without inventing
            // a missing-loc conclusion for a close tag that was never accepted.
            if let Some(entry) = inspection.active_entry.take() {
                let end = error
                    .xml_byte_offset
                    .unwrap_or(input.len() as u64)
                    .max(entry.start as u64)
                    .min(input.len() as u64);
                inspection
                    .observation
                    .entries
                    .push(SitemapEntryObservation {
                        ordinal: entry.ordinal,
                        xml_byte_start: entry.start as u64,
                        xml_byte_end: end,
                        complete: false,
                        loc_occurrence_ordinals: entry.locs,
                        lastmod: entry.lastmod,
                        diagnostics: vec!["incomplete_entry".into()],
                    });
            }
            inspection.observation.failure = Some(error);
        }
    }
    Ok(inspection.observation)
}

/// Second bounded pass over supplied decoded XML. It does not fetch, admit, retain
/// membership rows or attest HTTP/entity integrity. Only invalid limits/input size
/// return Err; bounded UTF8/control/syntax failures retain zero/prefix count facts.
pub fn count_sitemap_xml(
    input: &[u8],
    limits: &SitemapCountLimits,
) -> XmlResult<SitemapCountObservation> {
    limits.validate()?;
    if input.len() > limits.max_xml_bytes {
        return Err(SitemapFailure::new(
            "sitemap_xml_byte_budget",
            "Actual XML bytes exceed budget",
            None,
        ));
    }
    let mut out = SitemapCountObservation {
        schema_version: 1,
        engine: SITEMAP_COUNT_XML_ENGINE.into(),
        syntax_profile: "ascii-qname-xml10-no-dtd-v1".into(),
        loc_lexical_profile: SITEMAP_COUNT_LOC_PROFILE.into(),
        loc_whitespace_profile: "xml-s-edge-trim-v1".into(),
        loc_character_limit_exclusive: 2048,
        effective_limits: limits.clone(),
        input_xml_bytes: input.len(),
        input_xml_sha256: format!("{:x}", Sha256::digest(input)),
        input_xml_sha_profile: "decoded-sitemap-xml-bytes-v1".into(),
        kind: None,
        namespace_profile: None,
        events_observed: 0,
        xml_scan_complete: false,
        failure: None,
        counts: SitemapXmlCountFacts::default(),
    };
    let source = match std::str::from_utf8(input) {
        Ok(source) => source,
        Err(_) => {
            out.failure = Some(SitemapFailure::new(
                "sitemap_invalid_utf8",
                "Sitemap parser requires exact UTF-8 bytes",
                None,
            ));
            return Ok(out);
        }
    };
    if !source.chars().all(xml_char) {
        out.failure = Some(SitemapFailure::new(
            "sitemap_illegal_xml_character",
            "Raw input contains an illegal XML1.0 character",
            None,
        ));
        return Ok(out);
    }
    let parser_limits = limits.parser_limits();
    let mut inspection = Inspection {
        source,
        limits: &parser_limits,
        stack: Vec::new(),
        root_closed: false,
        declaration_seen: false,
        active_entry: None,
        count_sink: Some(CountSink::default()),
        capture: None,
        observation: SitemapParseObservation {
            schema_version: 1,
            engine: SITEMAP_XML_ENGINE.into(),
            context_basis: "caller_declared_parser_context".into(),
            context: SitemapParserContext::default(),
            xml_bytes: input.len(),
            xml_sha256: String::new(),
            xml_sha_profile: "decoded-sitemap-xml-bytes-v1".into(),
            namespace_profile: None,
            xml_name_profile: "ascii-qname-v1".into(),
            kind: None,
            events_observed: 0,
            retained_text_utf8_bytes: 0,
            complete: false,
            failure: None,
            entries: Vec::new(),
            locations: Vec::new(),
        },
    };
    match inspection.run() {
        Ok(()) => out.xml_scan_complete = true,
        Err(error) => out.failure = Some(error),
    }
    out.kind = inspection.observation.kind;
    out.namespace_profile = inspection.observation.namespace_profile;
    out.events_observed = inspection.observation.events_observed;
    out.counts = inspection
        .count_sink
        .expect("count sink remains present")
        .facts;
    debug_assert!(
        inspection.observation.entries.is_empty() && inspection.observation.locations.is_empty()
    );
    Ok(out)
}
