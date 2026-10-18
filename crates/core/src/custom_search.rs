//! Opt-in search over explicitly supplied UTF-8 sources. No I/O or HTML retention.
use anyhow::{Context, Result, ensure};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Deserializer, Serialize, de::Visitor};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, marker::PhantomData};

pub const CUSTOM_SEARCH_ENGINE: &str = "open-seo-spider/custom-search-v1";
pub const CUSTOM_SEARCH_MAX_CONFIG_BYTES: usize = 64 * 1024;
pub const CUSTOM_SEARCH_MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
pub const CUSTOM_SEARCH_MAX_REGEX_SCAN_WORK_BYTES: u64 = 16 * 1024 * 1024 * 1024;
const METADATA_RESERVE: usize = 512;
const MAX_DOCUMENT_URL_BYTES: usize = 16 * 1024;
const MAX_REVISION_BYTES: usize = 256;
const MAX_REASON_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CustomSearchSource {
    RawHtml,
    RenderedHtml,
    RawSelectedText,
    RenderedSelectedText,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CustomSearchMethod {
    Literal,
    Regex,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CustomSearchStatus {
    Complete,
    Incomplete,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CustomSearchLimits {
    pub max_input_bytes: usize,
    pub max_matches_scanned: usize,
    /// Conservative sum of remaining haystack bytes before each regex search,
    /// including the final no-match probe. A work proxy, never a CPU deadline.
    pub max_regex_scan_work_bytes: u64,
    pub max_saved_matches: usize,
    pub max_excerpt_bytes: usize,
    pub max_output_json_bytes: usize,
}

impl Default for CustomSearchLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 1024 * 1024,
            max_matches_scanned: 100_000,
            max_regex_scan_work_bytes: 1024 * 1024 * 1024,
            max_saved_matches: 100,
            max_excerpt_bytes: 256,
            max_output_json_bytes: 1024 * 1024,
        }
    }
}

// Unlike a derived struct deserializer, these configuration objects refuse
// positional JSON arrays as well as unknown keys.
struct ObjectOnly<T>(PhantomData<T>);
impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectOnly<T> {
    type Value = T;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a custom-search configuration object")
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
        T::deserialize(serde::de::value::MapAccessDeserializer::new(map))
    }
}

impl<'de> Deserialize<'de> for CustomSearchLimits {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(default, deny_unknown_fields)]
        struct Wire {
            max_input_bytes: usize,
            max_matches_scanned: usize,
            max_regex_scan_work_bytes: u64,
            max_saved_matches: usize,
            max_excerpt_bytes: usize,
            max_output_json_bytes: usize,
        }
        impl Default for Wire {
            fn default() -> Self {
                let v = CustomSearchLimits::default();
                Self {
                    max_input_bytes: v.max_input_bytes,
                    max_matches_scanned: v.max_matches_scanned,
                    max_regex_scan_work_bytes: v.max_regex_scan_work_bytes,
                    max_saved_matches: v.max_saved_matches,
                    max_excerpt_bytes: v.max_excerpt_bytes,
                    max_output_json_bytes: v.max_output_json_bytes,
                }
            }
        }
        let w = d.deserialize_map(ObjectOnly::<Wire>(PhantomData))?;
        Ok(Self {
            max_input_bytes: w.max_input_bytes,
            max_matches_scanned: w.max_matches_scanned,
            max_regex_scan_work_bytes: w.max_regex_scan_work_bytes,
            max_saved_matches: w.max_saved_matches,
            max_excerpt_bytes: w.max_excerpt_bytes,
            max_output_json_bytes: w.max_output_json_bytes,
        })
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CustomSearchRule {
    pub name: String,
    pub method: CustomSearchMethod,
    pub pattern: String,
    pub regex_flags: String,
    pub sources: Vec<CustomSearchSource>,
}

impl<'de> Deserialize<'de> for CustomSearchRule {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            name: String,
            method: CustomSearchMethod,
            pattern: String,
            #[serde(default)]
            regex_flags: String,
            sources: Vec<CustomSearchSource>,
        }
        let w = d.deserialize_map(ObjectOnly::<Wire>(PhantomData))?;
        Ok(Self {
            name: w.name,
            method: w.method,
            pattern: w.pattern,
            regex_flags: w.regex_flags,
            sources: w.sources,
        })
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CustomSearchConfig {
    pub version: u8,
    pub rules: Vec<CustomSearchRule>,
    pub limits: CustomSearchLimits,
}

impl Default for CustomSearchConfig {
    fn default() -> Self {
        Self {
            version: 1,
            rules: vec![],
            limits: CustomSearchLimits::default(),
        }
    }
}

impl<'de> Deserialize<'de> for CustomSearchConfig {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            version: u8,
            rules: Vec<CustomSearchRule>,
            #[serde(default)]
            limits: CustomSearchLimits,
        }
        let w = d.deserialize_map(ObjectOnly::<Wire>(PhantomData))?;
        Ok(Self {
            version: w.version,
            rules: w.rules,
            limits: w.limits,
        })
    }
}

impl CustomSearchConfig {
    pub fn compile(&self) -> Result<CompiledCustomSearchConfig> {
        CompiledCustomSearchConfig::compile(self)
    }
    pub fn validate(&self) -> Result<()> {
        self.compile().map(|_| ())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CustomSearchInput<'a> {
    pub text: Option<&'a str>,
    pub complete: bool,
    pub reason: Option<&'a str>,
    pub document_url: Option<&'a str>,
    pub observed_generation: Option<u64>,
    pub source_revision: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CustomSearchMatch {
    pub start_byte: u64,
    pub end_byte: u64,
    pub match_utf8_bytes: u64,
    /// Prefix of the match, bounded independently from the full matched span.
    pub text: String,
    pub text_truncated: bool,
    pub excerpt_start_byte: u64,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CustomSearchResult {
    pub engine: String,
    pub config_sha256: String,
    pub rule_name: String,
    pub method: CustomSearchMethod,
    pub pattern_sha256: String,
    pub source: CustomSearchSource,
    pub status: CustomSearchStatus,
    /// Only exact for a complete source and an exhausted match iterator.
    pub count: Option<u64>,
    pub matches_observed: u64,
    pub evidence_complete: bool,
    pub matches: Vec<CustomSearchMatch>,
    pub input_utf8_bytes: Option<u64>,
    pub searched_utf8_bytes: u64,
    /// SHA-256 of the entire supplied UTF-8 string, even when search uses a prefix.
    pub input_sha256: Option<String>,
    pub document_url: Option<String>,
    pub observed_generation: Option<u64>,
    pub source_revision: Option<String>,
    pub reasons: Vec<String>,
}

#[derive(Debug)]
enum Matcher {
    Literal,
    Regex(Regex),
}

#[derive(Debug)]
struct CompiledRule {
    matcher: Matcher,
    pattern_sha256: String,
}

#[derive(Debug)]
pub struct CompiledCustomSearchConfig {
    config: CustomSearchConfig,
    config_sha256: String,
    rules: Vec<CompiledRule>,
    result_count: usize,
    per_result_bytes: usize,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn prefix_boundary(value: &str, bytes: usize) -> usize {
    let mut end = bytes.min(value.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    end
}
fn reason(out: &mut CustomSearchResult, code: &str) {
    if !out.reasons.iter().any(|existing| existing == code) {
        out.reasons.push(code.to_owned());
    }
}
fn json_bytes(value: &impl Serialize) -> Result<usize> {
    Ok(serde_json::to_vec(value)?.len())
}

impl CompiledCustomSearchConfig {
    pub fn compile(config: &CustomSearchConfig) -> Result<Self> {
        ensure!(config.version == 1, "custom_search version must be 1");
        ensure!(
            (1..=32).contains(&config.rules.len()),
            "custom_search requires 1..=32 rules"
        );
        let limits = &config.limits;
        ensure!(
            (1..=8 * 1024 * 1024).contains(&limits.max_input_bytes),
            "custom_search max_input_bytes must be 1..=8388608"
        );
        ensure!(
            (1..=1_000_000).contains(&limits.max_matches_scanned),
            "custom_search max_matches_scanned must be 1..=1000000"
        );
        ensure!(
            (1..=CUSTOM_SEARCH_MAX_REGEX_SCAN_WORK_BYTES)
                .contains(&limits.max_regex_scan_work_bytes),
            "custom_search max_regex_scan_work_bytes must be 1..=17179869184"
        );
        ensure!(
            limits.max_saved_matches <= 256,
            "custom_search max_saved_matches must be 0..=256"
        );
        ensure!(
            (1..=4096).contains(&limits.max_excerpt_bytes),
            "custom_search max_excerpt_bytes must be 1..=4096"
        );
        ensure!(
            (64 * 1024..=CUSTOM_SEARCH_MAX_OUTPUT_BYTES).contains(&limits.max_output_json_bytes),
            "custom_search max_output_json_bytes must be 65536..=4194304"
        );
        let bytes = serde_json::to_vec(config)?;
        ensure!(
            bytes.len() <= CUSTOM_SEARCH_MAX_CONFIG_BYTES,
            "custom_search configuration exceeds 64 KiB"
        );
        let mut names = HashSet::new();
        let mut compiled = Vec::with_capacity(config.rules.len());
        let mut result_count = 0usize;
        for rule in &config.rules {
            ensure!(
                !rule.name.trim().is_empty()
                    && rule.name.len() <= 128
                    && !rule.name.chars().any(char::is_control),
                "custom_search rule name must be 1..=128 bytes without controls"
            );
            ensure!(
                names.insert(rule.name.as_str()),
                "Duplicate custom_search rule name"
            );
            ensure!(
                rule.pattern.len() <= 8192,
                "custom_search pattern exceeds 8192 bytes"
            );
            ensure!(
                (1..=4).contains(&rule.sources.len()),
                "custom_search sources must contain 1..=4 distinct sources"
            );
            let sources: HashSet<_> = rule.sources.iter().collect();
            ensure!(
                sources.len() == rule.sources.len(),
                "Duplicate custom_search source"
            );
            result_count += rule.sources.len();
            let matcher = match rule.method {
                CustomSearchMethod::Literal => {
                    ensure!(
                        !rule.pattern.is_empty(),
                        "custom_search literal pattern must not be empty"
                    );
                    ensure!(
                        rule.regex_flags.is_empty(),
                        "custom_search literal rules do not accept regex_flags"
                    );
                    Matcher::Literal
                }
                CustomSearchMethod::Regex => {
                    ensure!(
                        rule.regex_flags.len() <= 5,
                        "custom_search supports at most five regex flags"
                    );
                    let mut flags = HashSet::new();
                    let mut builder = RegexBuilder::new(&rule.pattern);
                    builder.size_limit(1024 * 1024).dfa_size_limit(1024 * 1024);
                    for flag in rule.regex_flags.chars() {
                        ensure!(flags.insert(flag), "Duplicate custom_search regex flag");
                        match flag {
                            'i' => {
                                builder.case_insensitive(true);
                            }
                            'm' => {
                                builder.multi_line(true);
                            }
                            's' => {
                                builder.dot_matches_new_line(true);
                            }
                            'x' => {
                                builder.ignore_whitespace(true);
                            }
                            'U' => {
                                builder.swap_greed(true);
                            }
                            _ => anyhow::bail!(
                                "Unsupported custom_search regex flag; supported: i,m,s,x,U"
                            ),
                        }
                    }
                    Matcher::Regex(builder.build().context("Invalid custom_search Rust regex; lookaround and backreferences are unsupported")?)
                }
            };
            compiled.push(CompiledRule {
                matcher,
                pattern_sha256: hash(rule.pattern.as_bytes()),
            });
        }
        // Reserve JSON array punctuation, then give every configured source/rule
        // an equal slot so combining all four sources remains bounded.
        let per_result_bytes = (limits.max_output_json_bytes - result_count - 1) / result_count;
        let out = Self {
            config: config.clone(),
            config_sha256: hash(&bytes),
            rules: compiled,
            result_count,
            per_result_bytes,
        };
        for (index, rule) in config.rules.iter().enumerate() {
            for &source in &rule.sources {
                let mut base = out.empty_result(index, source, CustomSearchInput::default());
                base.input_utf8_bytes = Some(u64::MAX);
                base.searched_utf8_bytes = u64::MAX;
                base.matches_observed = u64::MAX;
                base.observed_generation = Some(u64::MAX);
                base.input_sha256 = Some("f".repeat(64));
                base.count = Some(u64::MAX);
                reason(&mut base, "source_metadata_budget");
                ensure!(
                    json_bytes(&base)? + METADATA_RESERVE <= per_result_bytes,
                    "custom_search output budget cannot retain rule/source metadata"
                );
            }
        }
        Ok(out)
    }

    pub fn config(&self) -> &CustomSearchConfig {
        &self.config
    }

    fn empty_result(
        &self,
        index: usize,
        source: CustomSearchSource,
        input: CustomSearchInput<'_>,
    ) -> CustomSearchResult {
        CustomSearchResult {
            engine: CUSTOM_SEARCH_ENGINE.into(),
            config_sha256: self.config_sha256.clone(),
            rule_name: self.config.rules[index].name.clone(),
            method: self.config.rules[index].method,
            pattern_sha256: self.rules[index].pattern_sha256.clone(),
            source,
            status: CustomSearchStatus::Unavailable,
            count: None,
            matches_observed: 0,
            evidence_complete: false,
            matches: vec![],
            input_utf8_bytes: None,
            searched_utf8_bytes: 0,
            input_sha256: None,
            document_url: input
                .document_url
                .filter(|v| v.len() <= MAX_DOCUMENT_URL_BYTES)
                .map(str::to_owned),
            observed_generation: input.observed_generation,
            source_revision: input
                .source_revision
                .filter(|v| v.len() <= MAX_REVISION_BYTES)
                .map(str::to_owned),
            reasons: vec![],
        }
    }

    /// Literal matches are case-sensitive and non-overlapping. Regex iteration
    /// follows Rust regex UTF-8 and zero-width semantics. Input/scan bounds make
    /// an exact total unknown; evidence-retention bounds do not change an exact count.
    pub fn observe(
        &self,
        source: CustomSearchSource,
        input: CustomSearchInput<'_>,
    ) -> Vec<CustomSearchResult> {
        let indices: Vec<_> = self
            .config
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.sources.contains(&source))
            .map(|(index, _)| index)
            .collect();
        if indices.is_empty() {
            return vec![];
        }
        let input_hash = input.text.map(|text| hash(text.as_bytes()));
        indices
            .into_iter()
            .map(|index| self.observe_rule(index, source, input, input_hash.as_deref()))
            .collect()
    }

    fn observe_rule(
        &self,
        index: usize,
        source: CustomSearchSource,
        input: CustomSearchInput<'_>,
        input_hash: Option<&str>,
    ) -> CustomSearchResult {
        let limits = &self.config.limits;
        let mut out = self.empty_result(index, source, input);
        out.input_sha256 = input_hash.map(str::to_owned);
        out.input_utf8_bytes = input.text.map(|text| text.len() as u64);
        if let Some(message) = input.reason {
            let end = prefix_boundary(message, MAX_REASON_BYTES);
            reason(&mut out, &message[..end]);
            if end < message.len() {
                reason(&mut out, "source_reason_truncated");
            }
        }
        let invalid_metadata = input
            .document_url
            .is_some_and(|v| v.len() > MAX_DOCUMENT_URL_BYTES)
            || input
                .source_revision
                .is_some_and(|v| v.len() > MAX_REVISION_BYTES);
        if invalid_metadata
            || json_bytes(&out)
                .unwrap_or(usize::MAX)
                .saturating_add(METADATA_RESERVE)
                > self.per_result_bytes
        {
            out.document_url = None;
            out.source_revision = None;
            out.reasons.clear();
            reason(&mut out, "source_metadata_budget");
            return out;
        }
        let Some(text) = input.text else {
            reason(&mut out, "source_unavailable");
            return out;
        };
        let end = prefix_boundary(text, limits.max_input_bytes);
        let searched = &text[..end];
        out.searched_utf8_bytes = end as u64;
        if !input.complete {
            reason(&mut out, "source_incomplete");
        }
        if end < text.len() {
            reason(&mut out, "input_byte_limit");
        }
        // A regex applied to an artificial prefix may invent end anchors,
        // boundaries, or shorter alternation/greedy matches. No such evidence
        // can be called a lower bound of the complete source.
        if matches!(self.rules[index].matcher, Matcher::Regex(_))
            && (!input.complete || end < text.len())
        {
            out.status = CustomSearchStatus::Incomplete;
            out.searched_utf8_bytes = 0;
            reason(&mut out, "regex_partial_context_unknown");
            return out;
        }
        let mut matches: Box<dyn Iterator<Item = (usize, usize)> + '_> =
            match &self.rules[index].matcher {
                Matcher::Literal => Box::new(
                    searched
                        .match_indices(self.config.rules[index].pattern.as_str())
                        .map(|(start, value)| (start, start + value.len())),
                ),
                Matcher::Regex(regex) => {
                    Box::new(regex.find_iter(searched).map(|m| (m.start(), m.end())))
                }
            };
        let mut exhausted = false;
        let is_regex = matches!(self.rules[index].matcher, Matcher::Regex(_));
        let mut regex_work_left = limits.max_regex_scan_work_bytes;
        let mut regex_cursor = 0usize;
        let mut regex_work_stopped = false;
        let mut evidence_budget_exhausted = false;
        let mut evidence_bytes = json_bytes(&out).unwrap_or(self.per_result_bytes);
        while out.matches_observed < limits.max_matches_scanned as u64 {
            if is_regex {
                // find_iter may repeatedly inspect the remaining full source.
                // Charge its conservative window before each call; do not
                // shorten the source and thereby alter anchors or alternation.
                let work = searched.len().saturating_sub(regex_cursor).max(1) as u64;
                if work > regex_work_left {
                    reason(&mut out, "regex_scan_work_limit");
                    regex_work_stopped = true;
                    break;
                }
                regex_work_left -= work;
            }
            let Some((start, end)) = matches.next() else {
                exhausted = true;
                break;
            };
            regex_cursor = end;
            out.matches_observed += 1;
            if out.matches.len() >= limits.max_saved_matches {
                reason(&mut out, "saved_match_limit");
                continue;
            }
            if evidence_budget_exhausted {
                continue;
            }
            let saved = match_evidence(searched, start, end, limits.max_excerpt_bytes);
            let addition = json_bytes(&saved)
                .unwrap_or(self.per_result_bytes)
                .saturating_add(1);
            if evidence_bytes
                .saturating_add(addition)
                .saturating_add(METADATA_RESERVE)
                > self.per_result_bytes
            {
                reason(&mut out, "output_json_budget");
                evidence_budget_exhausted = true;
                continue;
            }
            evidence_bytes += addition;
            if saved.text_truncated {
                reason(&mut out, "match_text_truncated");
            }
            out.matches.push(saved);
        }
        if !exhausted && !regex_work_stopped {
            reason(&mut out, "match_scan_limit");
        }
        let complete = input.complete && end == text.len() && exhausted;
        out.status = if complete {
            CustomSearchStatus::Complete
        } else {
            CustomSearchStatus::Incomplete
        };
        out.count = complete.then_some(out.matches_observed);
        out.evidence_complete = complete
            && out.matches.len() as u64 == out.matches_observed
            && out.matches.iter().all(|m| !m.text_truncated);
        out
    }

    /// Apply after merging separately observed sources. Counts stay exact when
    /// only evidence must be dropped. Duplicate/unconfigured rows are errors.
    pub fn enforce_output_budget(&self, results: &mut Vec<CustomSearchResult>) -> Result<()> {
        self.validate_shape(results)?;
        for result in results.iter_mut() {
            while json_bytes(result)? > self.per_result_bytes {
                ensure!(
                    !result.matches.is_empty(),
                    "custom_search metadata exceeds output budget"
                );
                result.matches.pop();
                result.evidence_complete = false;
                reason(result, "output_json_budget");
            }
        }
        ensure!(
            json_bytes(results)? <= self.config.limits.max_output_json_bytes,
            "custom_search aggregate output exceeds budget"
        );
        Ok(())
    }

    fn validate_shape(&self, results: &[CustomSearchResult]) -> Result<()> {
        ensure!(
            results.len() == self.result_count,
            "custom_search requires exactly one result per configured rule/source"
        );
        let mut seen = HashSet::new();
        for result in results {
            let index = self
                .config
                .rules
                .iter()
                .position(|rule| rule.name == result.rule_name)
                .context("Unknown custom_search rule")?;
            let rule = &self.config.rules[index];
            ensure!(
                seen.insert((index, result.source)),
                "Duplicate custom_search result source"
            );
            ensure!(
                rule.sources.contains(&result.source) && rule.method == result.method,
                "custom_search source/method differs from config"
            );
            ensure!(
                result.engine == CUSTOM_SEARCH_ENGINE
                    && result.config_sha256 == self.config_sha256
                    && result.pattern_sha256 == self.rules[index].pattern_sha256,
                "custom_search result profile differs from config"
            );
        }
        Ok(())
    }

    /// Checks portable internal consistency only, never runtime or origin authority.
    /// Inherited observed generations are intentionally not compared with a new crawl.
    pub fn validate_results(&self, results: &[CustomSearchResult]) -> Result<()> {
        self.validate_shape(results)?;
        let limits = &self.config.limits;
        for result in results {
            let rule = self
                .config
                .rules
                .iter()
                .find(|rule| rule.name == result.rule_name)
                .expect("Shape validated above");
            ensure!(
                result.matches.len() <= limits.max_saved_matches
                    && result.matches.len() as u64 <= result.matches_observed,
                "custom_search saved match count is inconsistent"
            );
            ensure!(
                result.matches_observed <= limits.max_matches_scanned as u64
                    && result.searched_utf8_bytes <= limits.max_input_bytes as u64,
                "custom_search scan exceeds configured limits"
            );
            let possible_matches = if result.method == CustomSearchMethod::Literal {
                result.searched_utf8_bytes / rule.pattern.len() as u64
            } else {
                result.searched_utf8_bytes.saturating_add(1)
            };
            ensure!(
                result.matches_observed <= possible_matches,
                "custom_search match count exceeds non-overlapping UTF8 source positions"
            );
            ensure!(
                result
                    .input_utf8_bytes
                    .is_none_or(|n| result.searched_utf8_bytes <= n),
                "custom_search searched bytes exceed source"
            );
            ensure!(
                result.input_sha256.as_ref().is_none_or(|h| h.len() == 64
                    && h.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))),
                "Invalid custom_search input hash"
            );
            ensure!(
                result
                    .document_url
                    .as_ref()
                    .is_none_or(|v| v.len() <= MAX_DOCUMENT_URL_BYTES)
                    && result
                        .source_revision
                        .as_ref()
                        .is_none_or(|v| v.len() <= MAX_REVISION_BYTES),
                "custom_search metadata exceeds budget"
            );
            ensure!(
                result.reasons.len() <= 12
                    && result.reasons.iter().all(|r| r.len() <= MAX_REASON_BYTES),
                "custom_search reasons exceed budget"
            );
            match result.status {
                CustomSearchStatus::Complete => {
                    ensure!(
                        result.count == Some(result.matches_observed)
                            && result.input_utf8_bytes == Some(result.searched_utf8_bytes)
                            && result.input_sha256.is_some(),
                        "Complete custom_search result lacks exact source/count"
                    );
                    ensure!(
                        !result.reasons.iter().any(|r| matches!(
                            r.as_str(),
                            "source_incomplete"
                                | "input_byte_limit"
                                | "match_scan_limit"
                                | "source_unavailable"
                                | "source_metadata_budget"
                                | "regex_partial_context_unknown"
                                | "regex_scan_work_limit"
                        )),
                        "Complete custom_search result has incomplete source/scan reason"
                    );
                }
                CustomSearchStatus::Incomplete => {
                    ensure!(
                        result.count.is_none()
                            && result.input_utf8_bytes.is_some()
                            && result.input_sha256.is_some()
                            && !result.evidence_complete
                            && !result.reasons.is_empty(),
                        "Incomplete custom_search result claims an exact total"
                    );
                }
                CustomSearchStatus::Unavailable => {
                    ensure!(
                        result.count.is_none()
                            && result.matches_observed == 0
                            && result.searched_utf8_bytes == 0
                            && result.matches.is_empty()
                            && !result.evidence_complete
                            && !result.reasons.is_empty(),
                        "Unavailable custom_search result contains measured matches"
                    );
                }
            }
            if result.evidence_complete {
                ensure!(
                    result.status == CustomSearchStatus::Complete
                        && result.matches.len() as u64 == result.matches_observed
                        && result.matches.iter().all(|m| !m.text_truncated),
                    "custom_search complete evidence is missing matches"
                );
            }
            let mut previous_span = None;
            for m in &result.matches {
                ensure!(
                    m.start_byte <= m.end_byte
                        && m.end_byte <= result.searched_utf8_bytes
                        && m.match_utf8_bytes == m.end_byte - m.start_byte,
                    "Invalid custom_search match span"
                );
                ensure!(
                    previous_span.is_none_or(|(_, end)| m.start_byte >= end),
                    "custom_search matches are not in non-overlapping source order"
                );
                if let Some((previous_start, previous_end)) = previous_span
                    && (previous_start == previous_end || m.start_byte == m.end_byte)
                {
                    ensure!(
                        m.start_byte > previous_end,
                        "custom_search zero-width matches must advance beyond the previous span"
                    );
                }
                ensure!(
                    m.text.len() <= limits.max_excerpt_bytes
                        && m.excerpt.len() <= limits.max_excerpt_bytes
                        && m.text.len() as u64 <= m.match_utf8_bytes
                        && m.text_truncated == ((m.text.len() as u64) < m.match_utf8_bytes),
                    "Invalid custom_search bounded match text"
                );
                if result.method == CustomSearchMethod::Literal {
                    let end = prefix_boundary(&rule.pattern, limits.max_excerpt_bytes);
                    ensure!(
                        m.match_utf8_bytes == rule.pattern.len() as u64
                            && m.text == rule.pattern[..end],
                        "custom_search literal evidence differs from its configured pattern"
                    );
                }
                ensure!(
                    m.excerpt_start_byte <= m.start_byte
                        && m.excerpt_start_byte.saturating_add(m.excerpt.len() as u64)
                            <= result.searched_utf8_bytes,
                    "Invalid custom_search excerpt span"
                );
                let relative = usize::try_from(m.start_byte - m.excerpt_start_byte)?;
                ensure!(
                    m.excerpt
                        .get(relative..relative.saturating_add(m.text.len()))
                        == Some(m.text.as_str()),
                    "custom_search excerpt does not contain the saved match prefix"
                );
                previous_span = Some((m.start_byte, m.end_byte));
            }
        }
        // Check bounded fields first so malicious declared imports cannot make
        // the size check allocate a second oversized string/evidence payload.
        ensure!(
            json_bytes(&results)? <= limits.max_output_json_bytes,
            "custom_search saved output exceeds budget"
        );
        Ok(())
    }
}

fn match_evidence(text: &str, start: usize, end: usize, max: usize) -> CustomSearchMatch {
    let matched = &text[start..end];
    let saved_end = prefix_boundary(matched, max);
    let mut excerpt_start = start.saturating_sub(max.saturating_sub(matched.len()) / 2);
    while !text.is_char_boundary(excerpt_start) {
        excerpt_start += 1;
    }
    let excerpt_end = prefix_boundary(text, excerpt_start.saturating_add(max));
    CustomSearchMatch {
        start_byte: start as u64,
        end_byte: end as u64,
        match_utf8_bytes: matched.len() as u64,
        text: matched[..saved_end].into(),
        text_truncated: saved_end < matched.len(),
        excerpt_start_byte: excerpt_start as u64,
        excerpt: text[excerpt_start..excerpt_end].into(),
    }
}

#[cfg(test)]
#[path = "custom_search_tests.rs"]
mod tests;
