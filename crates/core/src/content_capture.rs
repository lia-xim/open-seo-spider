//! Explicit, deterministic text selection from supplied UTF-8 HTML. No I/O or fetching.
use anyhow::{Context, Result, ensure};
use ego_tree::iter::Edge;
use scraper::{ElementRef, Html, Node, Selector};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CONTENT_CAPTURE_ENGINE: &str = "open-seo-spider/html-content-capture-v1";
pub const CONTENT_CAPTURE_SELECTION_PROFILE: &str = "css-ancestor-membership-document-order-v1";
pub const CONTENT_CAPTURE_WHITESPACE_PROFILE: &str =
    "unicode-whitespace-and-nonempty-node-boundaries-ascii-space-v1";
pub const CONTENT_CAPTURE_PARSER_PROFILE: &str = "scraper-0.27.0-html5ever-document-recovery-v1";
pub const CONTENT_CAPTURE_HTML_HASH_REPRESENTATION: &str = "supplied-html-string-utf8-v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentCaptureSourceKind {
    RawHttpHtml,
    RenderedDomHtml,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentCaptureDecodeStatus {
    Exact,
    Lossy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ContentCaptureConfig {
    pub sources: Vec<ContentCaptureSourceKind>,
    pub include_selectors: Vec<String>,
    pub exclude_selectors: Vec<String>,
    pub limits: ContentCaptureLimits,
}

impl Default for ContentCaptureConfig {
    fn default() -> Self {
        Self {
            sources: vec![ContentCaptureSourceKind::RawHttpHtml],
            include_selectors: vec!["body".into()],
            exclude_selectors: ["script", "style", "template", "noscript", "nav", "footer"]
                .into_iter()
                .map(String::from)
                .collect(),
            limits: ContentCaptureLimits::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ContentCaptureLimits {
    pub max_html_utf8_bytes: usize,
    pub max_text_utf8_bytes: usize,
    pub max_visited_dom_nodes: usize,
    /// Root is depth zero. Fragment nodes, including template contents, count as levels.
    pub max_dom_depth: usize,
    pub max_selectors: usize,
    pub max_selector_utf8_bytes: usize,
    pub max_selector_checks: usize,
    pub max_config_json_bytes: usize,
    pub max_output_json_bytes: usize,
}

impl Default for ContentCaptureLimits {
    fn default() -> Self {
        Self {
            max_html_utf8_bytes: 8 * 1024 * 1024,
            max_text_utf8_bytes: 512 * 1024,
            max_visited_dom_nodes: 250_000,
            max_dom_depth: 1024,
            max_selectors: 64,
            max_selector_utf8_bytes: 4096,
            max_selector_checks: 2_000_000,
            max_config_json_bytes: 64 * 1024,
            max_output_json_bytes: 4 * 1024 * 1024,
        }
    }
}

impl ContentCaptureLimits {
    pub fn validate(&self) -> Result<()> {
        let ceiling = Self::default();
        macro_rules! bounded {
            ($($field:ident),+ $(,)?) => { $(
                ensure!(self.$field > 0 && self.$field <= ceiling.$field,
                    "{} must be positive and at most {}", stringify!($field), ceiling.$field);
            )+ };
        }
        bounded!(
            max_html_utf8_bytes,
            max_text_utf8_bytes,
            max_visited_dom_nodes,
            max_dom_depth,
            max_selectors,
            max_selector_utf8_bytes,
            max_selector_checks,
            max_config_json_bytes,
            max_output_json_bytes
        );
        Ok(())
    }
}

impl ContentCaptureConfig {
    /// Validate before making any network request. No HTML is needed.
    pub fn validate(&self) -> Result<()> {
        self.compile().map(|_| ())
    }

    /// Compile once and reuse for separately labelled raw/rendered observations.
    pub fn compile(&self) -> Result<CompiledContentCaptureConfig> {
        self.limits.validate()?;
        ensure!(
            !self.sources.is_empty() && self.sources.len() <= 2,
            "sources must contain one or two distinct source kinds"
        );
        ensure!(
            self.sources.len() != 2 || self.sources[0] != self.sources[1],
            "sources must be distinct"
        );
        ensure!(
            !self.include_selectors.is_empty(),
            "include_selectors must not be empty"
        );
        let selector_count = self
            .include_selectors
            .len()
            .checked_add(self.exclude_selectors.len())
            .context("selector count overflow")?;
        ensure!(
            selector_count <= self.limits.max_selectors,
            "selector count exceeds limit"
        );
        let config_json = serde_json::to_vec(self).context("serialize capture config")?;
        ensure!(
            config_json.len() <= self.limits.max_config_json_bytes,
            "serialized config exceeds max_config_json_bytes"
        );
        let compile_selectors = |list: &[String], kind: &str| -> Result<Vec<Selector>> {
            list.iter()
                .enumerate()
                .map(|(index, value)| {
                    ensure!(
                        !value.trim().is_empty(),
                        "{kind}[{index}] must not be blank"
                    );
                    ensure!(
                        value.len() <= self.limits.max_selector_utf8_bytes,
                        "{kind}[{index}] exceeds max_selector_utf8_bytes"
                    );
                    ensure!(
                        !value.contains('\0'),
                        "{kind}[{index}] contains a null scalar"
                    );
                    Selector::parse(value).map_err(|error| {
                        anyhow::anyhow!("invalid {kind}[{index}] CSS selector: {error:?}")
                    })
                })
                .collect()
        };
        Ok(CompiledContentCaptureConfig {
            config: self.clone(),
            config_sha256: sha256(&config_json),
            include: compile_selectors(&self.include_selectors, "include_selectors")?,
            exclude: compile_selectors(&self.exclude_selectors, "exclude_selectors")?,
        })
    }
}

/// Compiled selectors are private; effective config and its hash remain inspectable.
#[derive(Debug)]
pub struct CompiledContentCaptureConfig {
    config: ContentCaptureConfig,
    config_sha256: String,
    include: Vec<Selector>,
    exclude: Vec<Selector>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContentCaptureInput {
    pub source_kind: ContentCaptureSourceKind,
    pub html: Option<String>,
    /// Caller declaration about this source observation, not attested by the pure module.
    pub source_complete: bool,
    pub decode_status: ContentCaptureDecodeStatus,
    #[serde(default)]
    pub source_truncation_reason: Option<String>,
    /// A concrete caller reason for missing HTML, e.g. non_html or renderer_failed.
    #[serde(default)]
    pub unavailable_reason: Option<String>,
    #[serde(default)]
    pub config: ContentCaptureConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentCaptureStatus {
    Complete,
    Empty,
    Incomplete,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentCaptureReasonCode {
    HtmlNotSupplied,
    SourceIncomplete,
    DecodeLossy,
    DecodeUnknown,
    TextByteLimit,
    DomNodeLimit,
    DomDepthLimit,
    SelectorCheckLimit,
}

/// Own bounded syntax acceptance, not language detection or a complete BCP47 registry check.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentCaptureLanguageReason {
    HtmlUnavailable,
    MissingAttribute,
    EmptyAttribute,
    UnsupportedSyntax,
    ByteLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContentCaptureReason {
    pub code: ContentCaptureReasonCode,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContentCaptureEvidence {
    /// Includes parser-allocated nodes detached during document recovery.
    pub allocated_dom_nodes: usize,
    /// Reachable document-tree nodes actually opened; root counts as one.
    pub visited_dom_nodes: usize,
    pub visited_elements: usize,
    pub visited_text_nodes: usize,
    pub eligible_text_nodes: usize,
    pub nonempty_eligible_text_nodes: usize,
    pub template_fragments_visited: usize,
    pub maximum_depth_visited: usize,
    pub selector_checks: usize,
    /// Per configured selector; counts are a traversal prefix when traversal is incomplete.
    pub include_match_counts: Vec<usize>,
    pub exclude_match_counts: Vec<usize>,
    pub traversal_complete: bool,
    /// Exact selected stream observed during traversal, possibly a prefix of the selection.
    pub observed_normalized_utf8_bytes: usize,
    pub observed_normalized_utf8_sha256: String,
    pub emitted_utf8_bytes: usize,
    pub text_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContentCaptureResult {
    pub engine: String,
    pub selection_profile: String,
    pub whitespace_profile: String,
    pub parser_profile: String,
    pub unicode_version: String,
    pub source_kind: ContentCaptureSourceKind,
    pub source_complete: bool,
    pub decode_status: ContentCaptureDecodeStatus,
    /// Only the supplied document's root HTML lang attribute under the stated syntax profile.
    pub html_language: Option<String>,
    pub html_language_reason: Option<ContentCaptureLanguageReason>,
    pub html_hash_representation: String,
    /// Hash of supplied UTF-8 HTML string, never claimed to hash original HTTP wire bytes.
    pub html_utf8_sha256: Option<String>,
    pub html_utf8_bytes: Option<usize>,
    pub config: ContentCaptureConfig,
    pub config_sha256: String,
    pub status: ContentCaptureStatus,
    pub text: Option<String>,
    /// Exact emitted sample, even if incomplete; None when HTML was unavailable.
    pub text_sha256: Option<String>,
    pub reasons: Vec<ContentCaptureReason>,
    pub evidence: Option<ContentCaptureEvidence>,
    pub website_requests: usize,
}

/// Calling this function is explicit opt-in. Root crawl config defaults to no capture.
pub fn capture_content(input: &ContentCaptureInput) -> Result<ContentCaptureResult> {
    input.config.compile()?.capture(input)
}

#[derive(Clone, Copy, Default)]
struct PathState {
    included: bool,
    excluded: bool,
    template: bool,
}

impl CompiledContentCaptureConfig {
    pub fn config(&self) -> &ContentCaptureConfig {
        &self.config
    }

    pub fn config_sha256(&self) -> &str {
        &self.config_sha256
    }

    pub fn capture(&self, input: &ContentCaptureInput) -> Result<ContentCaptureResult> {
        ensure!(
            input.config == self.config,
            "input config differs from compiled config"
        );
        ensure!(
            self.config.sources.contains(&input.source_kind),
            "input source_kind is not selected by config.sources"
        );
        for (name, reason) in [
            ("source_truncation_reason", &input.source_truncation_reason),
            ("unavailable_reason", &input.unavailable_reason),
        ] {
            if let Some(value) = reason {
                ensure!(
                    !value.trim().is_empty()
                        && value.len() <= 512
                        && !value.chars().any(char::is_control),
                    "{name} must be nonblank, at most 512 UTF-8 bytes, and contain no controls"
                );
            }
        }
        ensure!(
            !input.source_complete || input.source_truncation_reason.is_none(),
            "complete source cannot declare a truncation reason"
        );
        ensure!(
            input.html.is_none() || input.unavailable_reason.is_none(),
            "unavailable_reason is only valid when html is absent"
        );
        ensure!(
            input.html.is_some() || !input.source_complete,
            "absent HTML cannot declare a complete source"
        );
        if let Some(html) = &input.html {
            ensure!(
                html.len() <= self.config.limits.max_html_utf8_bytes,
                "HTML input exceeds max_html_utf8_bytes"
            );
        }

        let unicode = char::UNICODE_VERSION;
        let mut result = ContentCaptureResult {
            engine: CONTENT_CAPTURE_ENGINE.into(),
            selection_profile: CONTENT_CAPTURE_SELECTION_PROFILE.into(),
            whitespace_profile: CONTENT_CAPTURE_WHITESPACE_PROFILE.into(),
            parser_profile: CONTENT_CAPTURE_PARSER_PROFILE.into(),
            unicode_version: format!("{}.{}.{}", unicode.0, unicode.1, unicode.2),
            source_kind: input.source_kind,
            source_complete: input.source_complete,
            decode_status: input.decode_status,
            html_language: None,
            html_language_reason: Some(ContentCaptureLanguageReason::HtmlUnavailable),
            html_hash_representation: CONTENT_CAPTURE_HTML_HASH_REPRESENTATION.into(),
            html_utf8_sha256: input.html.as_ref().map(|html| sha256(html.as_bytes())),
            html_utf8_bytes: input.html.as_ref().map(String::len),
            config: self.config.clone(),
            config_sha256: self.config_sha256.clone(),
            status: ContentCaptureStatus::Unavailable,
            text: None,
            text_sha256: None,
            reasons: Vec::new(),
            evidence: None,
            website_requests: 0,
        };
        let Some(html) = &input.html else {
            result.reasons.push(ContentCaptureReason {
                code: ContentCaptureReasonCode::HtmlNotSupplied,
                detail: input.unavailable_reason.clone(),
            });
            return self.bounded_result(result);
        };
        if !input.source_complete {
            result.reasons.push(ContentCaptureReason {
                code: ContentCaptureReasonCode::SourceIncomplete,
                detail: input.source_truncation_reason.clone(),
            });
        }
        match input.decode_status {
            ContentCaptureDecodeStatus::Exact => {}
            ContentCaptureDecodeStatus::Lossy => result.reasons.push(ContentCaptureReason {
                code: ContentCaptureReasonCode::DecodeLossy,
                detail: None,
            }),
            ContentCaptureDecodeStatus::Unknown => result.reasons.push(ContentCaptureReason {
                code: ContentCaptureReasonCode::DecodeUnknown,
                detail: None,
            }),
        }

        // Parsing is bounded by input bytes; traversal budgets are not a parser RSS bound.
        let document = Html::parse_document(html);
        (result.html_language, result.html_language_reason) =
            html_language(document.root_element().attr("lang"));
        let mut evidence = ContentCaptureEvidence {
            allocated_dom_nodes: document.tree.nodes().len(),
            visited_dom_nodes: 0,
            visited_elements: 0,
            visited_text_nodes: 0,
            eligible_text_nodes: 0,
            nonempty_eligible_text_nodes: 0,
            template_fragments_visited: 0,
            maximum_depth_visited: 0,
            selector_checks: 0,
            include_match_counts: vec![0; self.include.len()],
            exclude_match_counts: vec![0; self.exclude.len()],
            traversal_complete: true,
            observed_normalized_utf8_bytes: 0,
            observed_normalized_utf8_sha256: String::new(),
            emitted_utf8_bytes: 0,
            text_truncated: false,
        };
        let mut path: Vec<PathState> = Vec::new();
        let mut normalized = NormalizedText::new(self.config.limits.max_text_utf8_bytes);
        let limits = &self.config.limits;
        'walk: for edge in document.tree.root().traverse() {
            match edge {
                Edge::Open(node) => {
                    if evidence.visited_dom_nodes == limits.max_visited_dom_nodes {
                        result.reasons.push(limit_reason(
                            ContentCaptureReasonCode::DomNodeLimit,
                            limits.max_visited_dom_nodes,
                        ));
                        evidence.traversal_complete = false;
                        break;
                    }
                    if path.len() > limits.max_dom_depth {
                        result.reasons.push(limit_reason(
                            ContentCaptureReasonCode::DomDepthLimit,
                            limits.max_dom_depth,
                        ));
                        evidence.traversal_complete = false;
                        break;
                    }
                    evidence.visited_dom_nodes += 1;
                    evidence.maximum_depth_visited = evidence.maximum_depth_visited.max(path.len());
                    let mut state = path.last().copied().unwrap_or_default();
                    if let Some(element) = ElementRef::wrap(node) {
                        evidence.visited_elements += 1;
                        for (selectors, counts, is_include) in [
                            (&self.include, &mut evidence.include_match_counts, true),
                            (&self.exclude, &mut evidence.exclude_match_counts, false),
                        ] {
                            for (index, selector) in selectors.iter().enumerate() {
                                if evidence.selector_checks == limits.max_selector_checks {
                                    result.reasons.push(limit_reason(
                                        ContentCaptureReasonCode::SelectorCheckLimit,
                                        limits.max_selector_checks,
                                    ));
                                    evidence.traversal_complete = false;
                                    break 'walk;
                                }
                                evidence.selector_checks += 1;
                                if selector.matches(&element) {
                                    counts[index] += 1;
                                    if is_include {
                                        state.included = true;
                                    } else {
                                        state.excluded = true;
                                    }
                                }
                            }
                        }
                        state.template |= element.value().name() == "template";
                    }
                    if matches!(node.value(), Node::Fragment) && state.template {
                        evidence.template_fragments_visited += 1;
                    }
                    if let Node::Text(text) = node.value() {
                        evidence.visited_text_nodes += 1;
                        if state.included && !state.excluded {
                            evidence.eligible_text_nodes += 1;
                            let mut nonempty = false;
                            for word in text.text.split_whitespace() {
                                nonempty = true;
                                normalized.append_word(word)?;
                            }
                            if nonempty {
                                evidence.nonempty_eligible_text_nodes += 1;
                            }
                        }
                    }
                    path.push(state);
                }
                Edge::Close(_) => {
                    path.pop();
                }
            }
        }
        evidence.observed_normalized_utf8_bytes = normalized.observed_bytes;
        evidence.observed_normalized_utf8_sha256 = hex_digest(normalized.observed_hash.finalize());
        evidence.emitted_utf8_bytes = normalized.sample.len();
        evidence.text_truncated = normalized.truncated;
        if normalized.truncated {
            result.reasons.push(limit_reason(
                ContentCaptureReasonCode::TextByteLimit,
                limits.max_text_utf8_bytes,
            ));
        }
        result.status = if !result.reasons.is_empty() {
            ContentCaptureStatus::Incomplete
        } else if normalized.sample.is_empty() {
            ContentCaptureStatus::Empty
        } else {
            ContentCaptureStatus::Complete
        };
        result.text_sha256 = Some(sha256(normalized.sample.as_bytes()));
        result.text = Some(normalized.sample);
        result.evidence = Some(evidence);
        self.bounded_result(result)
    }

    fn bounded_result(&self, result: ContentCaptureResult) -> Result<ContentCaptureResult> {
        let bytes = serde_json::to_vec(&result).context("serialize capture result")?;
        ensure!(
            bytes.len() <= self.config.limits.max_output_json_bytes,
            "serialized result exceeds max_output_json_bytes"
        );
        Ok(result)
    }
}

struct NormalizedText {
    sample: String,
    observed_hash: Sha256,
    observed_bytes: usize,
    has_words: bool,
    truncated: bool,
    cap: usize,
}

impl NormalizedText {
    fn new(cap: usize) -> Self {
        Self {
            sample: String::new(),
            observed_hash: Sha256::new(),
            observed_bytes: 0,
            has_words: false,
            truncated: false,
            cap,
        }
    }

    fn append_word(&mut self, word: &str) -> Result<()> {
        let separator = usize::from(self.has_words);
        self.observed_bytes = self
            .observed_bytes
            .checked_add(separator)
            .and_then(|count| count.checked_add(word.len()))
            .context("normalized byte count overflow")?;
        if self.has_words {
            self.observed_hash.update(b" ");
        }
        self.observed_hash.update(word.as_bytes());
        if !self.truncated {
            for (index, scalar) in word.chars().enumerate() {
                let prefix = if index == 0 { separator } else { 0 };
                if self.sample.len() + prefix + scalar.len_utf8() > self.cap {
                    self.truncated = true;
                    break;
                }
                // Do not emit a separator if the following scalar cannot fit.
                if prefix != 0 {
                    self.sample.push(' ');
                }
                self.sample.push(scalar);
            }
        }
        self.has_words = true;
        Ok(())
    }
}

fn limit_reason(code: ContentCaptureReasonCode, limit: usize) -> ContentCaptureReason {
    ContentCaptureReason {
        code,
        detail: Some(format!("configured limit: {limit}")),
    }
}

fn html_language(value: Option<&str>) -> (Option<String>, Option<ContentCaptureLanguageReason>) {
    let Some(value) = value else {
        return (None, Some(ContentCaptureLanguageReason::MissingAttribute));
    };
    if value.is_empty() {
        return (None, Some(ContentCaptureLanguageReason::EmptyAttribute));
    }
    if value.len() > 128 {
        return (None, Some(ContentCaptureLanguageReason::ByteLimit));
    }
    let mut segments = value.split('-');
    let language = segments.next().unwrap_or_default();
    let accepted = (2..=8).contains(&language.len())
        && language.bytes().all(|c| c.is_ascii_alphabetic())
        && segments.all(|part| {
            (1..=8).contains(&part.len()) && part.bytes().all(|c| c.is_ascii_alphanumeric())
        });
    if accepted {
        (Some(value.into()), None)
    } else {
        (None, Some(ContentCaptureLanguageReason::UnsupportedSyntax))
    }
}

fn sha256(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes))
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(html: &str) -> ContentCaptureInput {
        ContentCaptureInput {
            source_kind: ContentCaptureSourceKind::RawHttpHtml,
            html: Some(html.into()),
            source_complete: true,
            decode_status: ContentCaptureDecodeStatus::Exact,
            source_truncation_reason: None,
            unavailable_reason: None,
            config: ContentCaptureConfig::default(),
        }
    }

    fn reasons(result: &ContentCaptureResult) -> Vec<ContentCaptureReasonCode> {
        result.reasons.iter().map(|reason| reason.code).collect()
    }

    #[test]
    fn authored_selection_fixtures() {
        #[derive(Deserialize)]
        struct Fixture {
            id: String,
            html: String,
            include: Vec<String>,
            exclude: Vec<String>,
            expected: String,
        }
        let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
            "../tests/fixtures/content-capture/selection.json"
        ))
        .unwrap();
        for fixture in fixtures {
            let mut request = input(&fixture.html);
            request.config.include_selectors = fixture.include;
            request.config.exclude_selectors = fixture.exclude;
            let result = capture_content(&request).unwrap();
            assert_eq!(
                result.text.as_deref(),
                Some(fixture.expected.as_str()),
                "{}",
                fixture.id
            );
            assert_eq!(
                result.status,
                if fixture.expected.is_empty() {
                    ContentCaptureStatus::Empty
                } else {
                    ContentCaptureStatus::Complete
                },
                "{}",
                fixture.id
            );
            assert!(result.evidence.as_ref().unwrap().traversal_complete);
            assert_eq!(result.website_requests, 0);
        }
    }

    #[test]
    fn default_profile_and_nested_matches_do_not_duplicate_text() {
        let mut request = input(
            "<body><nav>nav</nav><main>one<p>two<b>three</b></p>four</main><footer>foot</footer><script>script</script></body>",
        );
        request.config.include_selectors =
            vec!["body".into(), "main".into(), "p".into(), "b".into()];
        let result = capture_content(&request).unwrap();
        assert_eq!(result.text.as_deref(), Some("one two three four"));
        let evidence = result.evidence.unwrap();
        assert_eq!(evidence.include_match_counts, [1, 1, 1, 1]);
        assert_eq!(evidence.nonempty_eligible_text_nodes, 4);
    }

    #[test]
    fn exclusions_win_over_descendant_inclusions_without_leaking_state_to_siblings() {
        let mut request =
            input("<main>A<section class='omit'><b>B</b></section><p>C</p></main><p>D</p>");
        request.config.include_selectors = vec!["main".into(), ".omit b".into()];
        request.config.exclude_selectors = vec![".omit".into()];
        assert_eq!(
            capture_content(&request).unwrap().text.as_deref(),
            Some("A C")
        );
    }

    #[test]
    fn template_fragment_is_excluded_by_ancestor_and_can_be_explicitly_included() {
        let html = "<body>A<template><p>B<template><b>C</b></template>D</p></template>E</body>";
        let request = input(html);
        let default = capture_content(&request).unwrap();
        assert_eq!(default.text.as_deref(), Some("A E"));
        assert_eq!(default.evidence.unwrap().template_fragments_visited, 2);
        let mut configured = request;
        configured.config.exclude_selectors.clear();
        assert_eq!(
            capture_content(&configured).unwrap().text.as_deref(),
            Some("A B C D E")
        );
    }

    #[test]
    fn html_facts_do_not_infer_visibility_attributes_alt_or_visual_word_boundaries() {
        let request = input(
            "<body>foo<em>bar</em>baz<span hidden>H</span><span style='display:none'>S</span><img alt='ALT'><br>Z<iframe src='https://example.invalid/'>fallback</iframe></body>",
        );
        assert_eq!(
            capture_content(&request).unwrap().text.as_deref(),
            Some("foo bar baz H S Z fallback")
        );
    }

    #[test]
    fn unicode_whitespace_entities_and_combining_scalars_are_explicit() {
        let request =
            input("<body> \t A&nbsp; B&#x2003;C&#x2028;D\n e&#x301; &#x200b;X &amp; &lt; </body>");
        let result = capture_content(&request).unwrap();
        assert_eq!(
            result.text.as_deref(),
            Some("A B C D e\u{301} \u{200b}X & <")
        );
        assert_eq!(
            result.text_sha256.as_deref(),
            Some(sha256(result.text.as_ref().unwrap().as_bytes()).as_str())
        );
        assert!(!result.unicode_version.is_empty());
    }

    #[test]
    fn absent_html_is_distinct_from_complete_empty_selection() {
        let empty = capture_content(&input("")).unwrap();
        assert_eq!(empty.status, ContentCaptureStatus::Empty);
        assert_eq!(empty.text.as_deref(), Some(""));
        assert_eq!(
            empty.text_sha256.as_deref(),
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
        assert!(empty.evidence.is_some());
        let mut missing = input("");
        missing.html = None;
        missing.source_complete = false;
        missing.unavailable_reason = Some("non_html".into());
        let missing = capture_content(&missing).unwrap();
        assert_eq!(missing.status, ContentCaptureStatus::Unavailable);
        assert!(
            missing.text.is_none() && missing.text_sha256.is_none() && missing.evidence.is_none()
        );
        assert!(missing.html_utf8_sha256.is_none());
        assert_eq!(
            reasons(&missing),
            [ContentCaptureReasonCode::HtmlNotSupplied]
        );
        assert_eq!(missing.reasons[0].detail.as_deref(), Some("non_html"));
    }

    #[test]
    fn decode_or_source_incompleteness_never_claims_complete_or_empty() {
        for html in ["", "<p>sample</p>"] {
            for decode in [
                ContentCaptureDecodeStatus::Lossy,
                ContentCaptureDecodeStatus::Unknown,
            ] {
                let mut request = input(html);
                request.decode_status = decode;
                assert_eq!(
                    capture_content(&request).unwrap().status,
                    ContentCaptureStatus::Incomplete
                );
            }
            let mut request = input(html);
            request.source_complete = false;
            request.source_truncation_reason = Some("native_response_byte_cap".into());
            let result = capture_content(&request).unwrap();
            assert_eq!(result.status, ContentCaptureStatus::Incomplete);
            assert_eq!(
                reasons(&result),
                [ContentCaptureReasonCode::SourceIncomplete]
            );
            assert_eq!(
                result.reasons[0].detail.as_deref(),
                Some("native_response_byte_cap")
            );
        }
    }

    #[test]
    fn raw_and_rendered_sources_are_distinct_explicit_and_bound_in_config_hash() {
        let mut raw = input("<p>raw</p>");
        let raw_only = capture_content(&raw).unwrap();
        raw.config
            .sources
            .push(ContentCaptureSourceKind::RenderedDomHtml);
        let compiled = raw.config.compile().unwrap();
        let raw_result = compiled.capture(&raw).unwrap();
        let mut rendered = raw.clone();
        rendered.source_kind = ContentCaptureSourceKind::RenderedDomHtml;
        rendered.html = Some("<p>executed</p>".into());
        let rendered_result = compiled.capture(&rendered).unwrap();
        assert_ne!(raw_only.config_sha256, raw_result.config_sha256);
        assert_eq!(raw_result.config_sha256, rendered_result.config_sha256);
        assert_ne!(
            raw_result.html_utf8_sha256,
            rendered_result.html_utf8_sha256
        );
        assert_eq!(rendered_result.text.as_deref(), Some("executed"));
        rendered.html = None;
        rendered.source_complete = false;
        rendered.unavailable_reason = Some("renderer_failed".into());
        assert_eq!(
            compiled.capture(&rendered).unwrap().status,
            ContentCaptureStatus::Unavailable
        );
        rendered.config.sources = vec![ContentCaptureSourceKind::RawHttpHtml];
        assert!(capture_content(&rendered).is_err());
        assert!(compiled.capture(&rendered).is_err());
    }

    #[test]
    fn language_comes_only_from_each_supplied_html_root_not_body_or_other_source() {
        let mut raw = input(
            "<html lang='de-DE'><body lang='en'><p>Text</p><template lang='fr'>T</template></body></html>",
        );
        raw.config
            .sources
            .push(ContentCaptureSourceKind::RenderedDomHtml);
        let compiled = raw.config.compile().unwrap();
        let raw_result = compiled.capture(&raw).unwrap();
        assert_eq!(raw_result.html_language.as_deref(), Some("de-DE"));
        assert!(raw_result.html_language_reason.is_none());
        let mut rendered = raw;
        rendered.source_kind = ContentCaptureSourceKind::RenderedDomHtml;
        rendered.html = Some("<html lang='en'><body lang='de'>Text</body></html>".into());
        assert_eq!(
            compiled
                .capture(&rendered)
                .unwrap()
                .html_language
                .as_deref(),
            Some("en")
        );
        let body_only = capture_content(&input("<body lang='de'>Text</body>")).unwrap();
        assert!(body_only.html_language.is_none());
        assert_eq!(
            body_only.html_language_reason,
            Some(ContentCaptureLanguageReason::MissingAttribute)
        );
    }

    #[test]
    fn unsupported_language_syntax_is_unknown_without_claiming_incomplete_text() {
        for (language, reason) in [
            ("", ContentCaptureLanguageReason::EmptyAttribute),
            (" en ", ContentCaptureLanguageReason::UnsupportedSyntax),
            ("de--DE", ContentCaptureLanguageReason::UnsupportedSyntax),
            ("i-klingon", ContentCaptureLanguageReason::UnsupportedSyntax),
        ] {
            let result = capture_content(&input(&format!(
                "<html lang='{language}'><body>Text</body></html>"
            )))
            .unwrap();
            assert!(result.html_language.is_none());
            assert_eq!(result.html_language_reason, Some(reason));
            assert_eq!(result.status, ContentCaptureStatus::Complete);
        }
        let result = capture_content(&input(&format!(
            "<html lang='{}'>Text</html>",
            "x".repeat(129)
        )))
        .unwrap();
        assert_eq!(
            result.html_language_reason,
            Some(ContentCaptureLanguageReason::ByteLimit)
        );
    }

    #[test]
    fn utf8_cap_keeps_prefix_and_never_splits_scalar_or_leaves_separator() {
        for (cap, expected) in [
            (1, "A"),
            (2, "A"),
            (5, "A"),
            (6, "A 😀"),
            (7, "A 😀"),
            (9, "A 😀 B"),
        ] {
            let mut request = input("<p>A 😀 B</p>");
            request.config.limits.max_text_utf8_bytes = cap;
            let result = capture_content(&request).unwrap();
            assert_eq!(result.text.as_deref(), Some(expected), "cap {cap}");
            let evidence = result.evidence.unwrap();
            assert_eq!(evidence.observed_normalized_utf8_bytes, 8);
            assert_eq!(
                evidence.observed_normalized_utf8_sha256,
                sha256("A 😀 B".as_bytes())
            );
            assert_eq!(evidence.text_truncated, cap < 8);
            assert_eq!(evidence.emitted_utf8_bytes, expected.len());
            assert!(!expected.ends_with(' '));
        }
        let mut request = input("<p>😀A</p>");
        request.config.limits.max_text_utf8_bytes = 3;
        let result = capture_content(&request).unwrap();
        assert_eq!(result.text.as_deref(), Some(""));
        assert_eq!(result.status, ContentCaptureStatus::Incomplete);
    }

    #[test]
    fn exact_cap_and_trailing_whitespace_are_complete() {
        let mut request = input("<p>é  \n</p>");
        request.config.limits.max_text_utf8_bytes = 2;
        let result = capture_content(&request).unwrap();
        assert_eq!(result.status, ContentCaptureStatus::Complete);
        assert_eq!(result.text.as_deref(), Some("é"));
        assert!(!result.evidence.unwrap().text_truncated);
    }

    #[test]
    fn structural_limits_return_explicit_partial_traversal_not_fabricated_totals() {
        let mut request = input("<body><p>A<b>B</b>C</p><p>D</p></body>");
        let full = capture_content(&request).unwrap();
        let full_evidence = full.evidence.unwrap();
        request.config.limits.max_visited_dom_nodes = full_evidence.visited_dom_nodes;
        assert_eq!(
            capture_content(&request).unwrap().status,
            ContentCaptureStatus::Complete
        );
        request.config.limits.max_visited_dom_nodes -= 1;
        let partial = capture_content(&request).unwrap();
        assert_eq!(partial.status, ContentCaptureStatus::Incomplete);
        assert!(reasons(&partial).contains(&ContentCaptureReasonCode::DomNodeLimit));
        assert!(!partial.evidence.unwrap().traversal_complete);
        request.config.limits.max_visited_dom_nodes = 250_000;
        request.config.limits.max_dom_depth = 2;
        let partial = capture_content(&request).unwrap();
        assert_eq!(reasons(&partial), [ContentCaptureReasonCode::DomDepthLimit]);
        let evidence = partial.evidence.unwrap();
        assert!(!evidence.traversal_complete);
        assert_eq!(evidence.maximum_depth_visited, 2);
    }

    #[test]
    fn selector_check_budget_is_exact_and_config_can_compile_without_html() {
        let mut request = input("<p>A</p>");
        request.config.exclude_selectors.clear();
        request.config.validate().unwrap();
        let full = capture_content(&request).unwrap();
        let checks = full.evidence.unwrap().selector_checks;
        request.config.limits.max_selector_checks = checks;
        assert_eq!(
            capture_content(&request).unwrap().status,
            ContentCaptureStatus::Complete
        );
        request.config.limits.max_selector_checks = checks - 1;
        let result = capture_content(&request).unwrap();
        assert_eq!(
            reasons(&result),
            [ContentCaptureReasonCode::SelectorCheckLimit]
        );
        assert!(!result.evidence.unwrap().traversal_complete);
    }

    #[test]
    fn large_nested_dom_visits_each_reachable_node_once_with_path_state() {
        let html = format!(
            "<body>{}text{}</body>",
            "<div>".repeat(400),
            "</div>".repeat(400)
        );
        let mut request = input(&html);
        request.config.include_selectors = vec!["body".into(), "div".into()];
        request.config.exclude_selectors.clear();
        let result = capture_content(&request).unwrap();
        assert_eq!(result.text.as_deref(), Some("text"));
        let evidence = result.evidence.unwrap();
        assert_eq!(evidence.include_match_counts, [1, 400]);
        assert_eq!(evidence.nonempty_eligible_text_nodes, 1);
        assert_eq!(evidence.selector_checks, evidence.visited_elements * 2);
        assert!(evidence.traversal_complete);
    }

    #[test]
    fn hashes_roundtrip_and_duplicate_selector_membership_are_deterministic() {
        let mut request = input("<p> A &amp; B </p>");
        request.config.include_selectors.push("body".into());
        let result = capture_content(&request).unwrap();
        assert_eq!(
            result.html_utf8_sha256,
            Some(sha256(request.html.as_ref().unwrap().as_bytes()))
        );
        assert_eq!(
            result.config_sha256,
            sha256(&serde_json::to_vec(&request.config).unwrap())
        );
        assert_eq!(result.text_sha256, Some(sha256(b"A & B")));
        assert_eq!(
            result.evidence.as_ref().unwrap().include_match_counts,
            [1, 1]
        );
        let restored: ContentCaptureInput =
            serde_json::from_slice(&serde_json::to_vec(&request).unwrap()).unwrap();
        assert_eq!(result, capture_content(&restored).unwrap());
        let restored_result: ContentCaptureResult =
            serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
        assert_eq!(result, restored_result);
    }

    #[test]
    fn invalid_selectors_limits_sources_and_shapes_fail_closed() {
        let mut config = ContentCaptureConfig {
            include_selectors: vec!["[".into()],
            ..Default::default()
        };
        assert!(config.validate().is_err());
        config.include_selectors = vec![];
        assert!(config.validate().is_err());
        config = ContentCaptureConfig::default();
        config.sources.clear();
        assert!(config.validate().is_err());
        config.sources = vec![ContentCaptureSourceKind::RawHttpHtml; 2];
        assert!(config.validate().is_err());
        config = ContentCaptureConfig::default();
        config.limits.max_text_utf8_bytes = 0;
        assert!(config.validate().is_err());
        config.limits.max_text_utf8_bytes = 512 * 1024 + 1;
        assert!(config.validate().is_err());
        config = ContentCaptureConfig::default();
        config.limits.max_selector_utf8_bytes = 3;
        assert!(config.validate().is_err());
        config = ContentCaptureConfig::default();
        config.limits.max_selectors = 1;
        assert!(config.validate().is_err());
        config = ContentCaptureConfig::default();
        config.limits.max_config_json_bytes = 1;
        assert!(config.validate().is_err());
        assert!(serde_json::from_str::<ContentCaptureConfig>("{\"visibility\":true}").is_err());
        assert!(
            serde_json::from_str::<ContentCaptureConfig>("{\"limits\":{\"extra\":1}}").is_err()
        );
        let mut value = serde_json::to_value(input("A")).unwrap();
        value["extra"] = true.into();
        assert!(serde_json::from_value::<ContentCaptureInput>(value).is_err());
    }

    #[test]
    fn conflicting_source_declarations_and_bounded_reasons_are_rejected() {
        let mut request = input("A");
        request.source_truncation_reason = Some("cap".into());
        assert!(capture_content(&request).is_err());
        request.source_truncation_reason = None;
        request.unavailable_reason = Some("missing".into());
        assert!(capture_content(&request).is_err());
        request.html = None;
        assert!(capture_content(&request).is_err());
        request.source_complete = false;
        request.unavailable_reason = Some("\n".into());
        assert!(capture_content(&request).is_err());
        request.unavailable_reason = Some("x".repeat(513));
        assert!(capture_content(&request).is_err());
    }

    #[test]
    fn actual_utf8_html_and_serialized_output_budgets_are_enforced() {
        let mut request = input("é");
        request.config.limits.max_html_utf8_bytes = 1;
        assert!(capture_content(&request).is_err());
        request.config.limits.max_html_utf8_bytes = 2;
        assert!(capture_content(&request).is_ok());
        let result = capture_content(&request).unwrap();
        let original_len = serde_json::to_vec(&result).unwrap().len();
        request.config.limits.max_output_json_bytes = original_len + 20;
        let len = serde_json::to_vec(&capture_content(&request).unwrap())
            .unwrap()
            .len();
        request.config.limits.max_output_json_bytes = len;
        assert!(capture_content(&request).is_ok());
        request.config.limits.max_output_json_bytes -= 1;
        assert!(capture_content(&request).is_err());
    }
}
