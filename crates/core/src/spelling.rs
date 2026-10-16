//! Saved-text spelling adapter. No network, crawl, storage or automatic language detection.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const ENGINE: &str = "open-seo-spider/saved-spelling-v1";
pub const MAX_DOCUMENTS: usize = 32;
pub const MAX_TEXT_BYTES: usize = 65_536;
pub const MAX_TEXT_UTF16: usize = 20_000;
pub const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_RESULT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ISSUES: usize = 1_000;

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn hash<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|v| digest(&v))
        .map_err(|e| e.to_string())
}
fn default_true() -> bool {
    true
}
fn default_fallback() -> String {
    "en-US".into()
}
fn default_regions() -> BTreeMap<String, String> {
    [("en", "en-US"), ("de", "de-DE"), ("pt", "pt-PT")]
        .into_iter()
        .map(|(a, b)| (a.into(), b.into()))
        .collect()
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Word {
    pub language: String,
    pub word: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Dictionary {
    pub version: u32,
    #[serde(default)]
    pub words: Vec<Word>,
}
impl Default for Dictionary {
    fn default() -> Self {
        Self {
            version: 1,
            words: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub acknowledge_local_text_transfer: bool,
    #[serde(default)]
    pub expected_engine_version: Option<String>,
    #[serde(default = "default_true")]
    pub spelling: bool,
    #[serde(default = "default_true")]
    pub grammar: bool,
    #[serde(default)]
    pub manual_language: Option<String>,
    #[serde(default = "default_true")]
    pub use_html_language: bool,
    #[serde(default = "default_fallback")]
    pub fallback_language: String,
    #[serde(default = "default_regions")]
    pub regional_defaults: BTreeMap<String, String>,
    #[serde(default)]
    pub ignored_words: Vec<Word>,
    #[serde(default)]
    pub ignored_rules: Vec<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            enabled: false,
            endpoint: None,
            acknowledge_local_text_transfer: false,
            expected_engine_version: None,
            spelling: true,
            grammar: true,
            manual_language: None,
            use_html_language: true,
            fallback_language: default_fallback(),
            regional_defaults: default_regions(),
            ignored_words: Vec::new(),
            ignored_rules: Vec::new(),
        }
    }
}
pub fn language_tag(value: &str) -> Result<String, String> {
    if value.len() > 35 || value.is_empty() || value == "auto" {
        return Err("Explicit language tag required; auto detection is not enabled".into());
    }
    let parts: Vec<_> = value.split('-').collect();
    if !(2..=8).contains(&parts[0].len())
        || !parts[0].bytes().all(|b| b.is_ascii_alphabetic())
        || parts[1..]
            .iter()
            .any(|v| v.is_empty() || v.len() > 8 || !v.bytes().all(|b| b.is_ascii_alphanumeric()))
    {
        return Err("Invalid language tag".into());
    }
    Ok(parts
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if i == 0 {
                v.to_ascii_lowercase()
            } else if v.len() == 2 {
                v.to_ascii_uppercase()
            } else {
                v.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("-"))
}
fn validate_words(words: &[Word], max: usize) -> Result<(), String> {
    if words.len() > max {
        return Err("Word collection exceeds bound".into());
    }
    let mut seen = BTreeSet::new();
    for v in words {
        if language_tag(&v.language)? != v.language
            || v.word.is_empty()
            || v.word.encode_utf16().count() > 128
            || v.word.chars().any(|c| c.is_control() || c.is_whitespace())
            || !seen.insert((&v.language, &v.word))
        {
            return Err("Invalid or duplicate dictionary word".into());
        }
    }
    Ok(())
}
impl Dictionary {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported dictionary version".into());
        }
        validate_words(&self.words, 10_000)?;
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 1024 * 1024 {
            return Err("Dictionary exceeds1MiB".into());
        }
        Ok(())
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported spelling config version".into());
        }
        language_tag(&self.fallback_language)?;
        if let Some(v) = &self.manual_language {
            language_tag(v)?;
        }
        if self.regional_defaults.len() > 16 {
            return Err("Too many regional defaults".into());
        }
        for (base, variant) in &self.regional_defaults {
            if base.contains('-')
                || language_tag(base)? != *base
                || !language_tag(variant)?.starts_with(&format!("{base}-"))
            {
                return Err("Invalid regional default".into());
            }
        }
        validate_words(&self.ignored_words, 2_000)?;
        let mut rules = BTreeSet::new();
        if self.ignored_rules.len() > 1_000
            || self.ignored_rules.iter().any(|v| {
                v.is_empty()
                    || v.len() > 128
                    || !v
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                    || !rules.insert(v)
            })
        {
            return Err("Invalid or duplicate ignored rule".into());
        }
        if let Some(v) = &self.expected_engine_version {
            bounded(v, 128, "engine version")?;
        }
        if self.enabled {
            if !self.acknowledge_local_text_transfer {
                return Err("Explicit local text transfer acknowledgement required".into());
            }
            if !self.spelling && !self.grammar {
                return Err("Select spelling or grammar".into());
            }
            validate_endpoint(
                self.endpoint
                    .as_deref()
                    .ok_or("Local engine endpoint required")?,
            )?;
        } else if let Some(v) = &self.endpoint {
            validate_endpoint(v)?;
        }
        Ok(())
    }
}
pub fn validate_endpoint(value: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(value).map_err(|_| "Invalid local endpoint")?;
    let host = url
        .host_str()
        .and_then(|v| {
            v.trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<std::net::IpAddr>()
                .ok()
        })
        .is_some_and(|ip| ip.is_loopback());
    if url.scheme() != "http"
        || !host
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_none_or(|p| p == 0)
        || url.path() != "/v2/check"
        || url.query().is_some()
        || url.fragment().is_some()
        || url.as_str() != value
    {
        return Err(
            "Endpoint must be canonical literal-loopback HTTP /v2/check with explicit port".into(),
        );
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LanguageDecision {
    pub requested: String,
    pub basis: String,
    pub html_language: Option<String>,
    pub html_language_valid: bool,
    pub regional_default_applied: bool,
    pub automatic_detection: bool,
}
pub fn choose_language(config: &Config, html: Option<&str>) -> Result<LanguageDecision, String> {
    let parsed = html.and_then(|v| language_tag(v).ok());
    let (mut requested, basis) = if let Some(v) = &config.manual_language {
        (language_tag(v)?, "manual")
    } else if config.use_html_language && parsed.is_some() {
        (parsed.clone().unwrap_or_default(), "saved_html_lang")
    } else {
        (
            language_tag(&config.fallback_language)?,
            "configured_fallback",
        )
    };
    let mut regional = false;
    if !requested.contains('-') {
        if let Some(v) = config.regional_defaults.get(&requested) {
            requested = language_tag(v)?;
            regional = true;
        } else if ["en", "de", "pt"].contains(&requested.as_str()) {
            return Err("Language requires an explicit regional default".into());
        }
    }
    Ok(LanguageDecision {
        requested,
        basis: basis.into(),
        html_language: html.map(str::to_owned),
        html_language_valid: parsed.is_some(),
        regional_default_applied: regional,
        automatic_detection: false,
    })
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Software {
    pub name: String,
    pub version: String,
    #[serde(rename = "buildDate")]
    pub build_date: String,
    #[serde(rename = "apiVersion")]
    pub api_version: u32,
}
#[derive(Deserialize)]
struct ProviderLanguage {
    name: String,
    code: String,
}
#[derive(Deserialize)]
struct ProviderWarnings {
    #[serde(rename = "incompleteResults")]
    incomplete_results: bool,
}
#[derive(Deserialize)]
struct ProviderContext {
    text: String,
    offset: usize,
    length: usize,
}
#[derive(Deserialize)]
struct Replacement {
    value: String,
}
#[derive(Deserialize)]
struct Category {
    id: String,
    name: String,
}
#[derive(Deserialize)]
struct Rule {
    id: String,
    #[serde(rename = "subId")]
    sub_id: Option<String>,
    description: String,
    #[serde(rename = "issueType")]
    issue_type: String,
    category: Category,
}
#[derive(Deserialize)]
struct Match {
    message: String,
    #[serde(rename = "shortMessage", default)]
    short_message: String,
    offset: usize,
    length: usize,
    context: ProviderContext,
    replacements: Vec<Replacement>,
    rule: Rule,
}
#[derive(Deserialize)]
struct Reply {
    software: Software,
    language: ProviderLanguage,
    warnings: ProviderWarnings,
    matches: Vec<Match>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Issue {
    pub offset_utf16: usize,
    pub length_utf16: usize,
    pub matched_text: String,
    pub message: String,
    pub short_message: String,
    pub context: String,
    pub context_offset_utf16: usize,
    pub rule_id: String,
    pub rule_sub_id: Option<String>,
    pub rule_description: String,
    pub issue_type: String,
    pub category_id: String,
    pub category_name: String,
    pub replacements: Vec<String>,
    pub disposition: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Checked {
    pub software: Software,
    pub response_language: String,
    pub response_language_name: String,
    pub complete: bool,
    pub response_sha256: String,
    pub response_bytes: usize,
    pub issues: Vec<Issue>,
    pub accepted_count: Option<usize>,
}
pub fn utf16_slice(text: &str, offset: usize, length: usize) -> Result<&str, String> {
    let end = offset.checked_add(length).ok_or("UTF16 range overflow")?;
    let mut units = 0;
    let mut start = None;
    let mut finish = None;
    for (byte, ch) in text.char_indices() {
        if units == offset {
            start = Some(byte);
        }
        if units == end {
            finish = Some(byte);
        }
        units += ch.len_utf16();
    }
    if units == offset {
        start = Some(text.len());
    }
    if units == end {
        finish = Some(text.len());
    }
    match (start, finish) {
        (Some(a), Some(b)) if a <= b => Ok(&text[a..b]),
        _ => Err("Provider UTF16 range splits a surrogate or exceeds checked text".into()),
    }
}
fn bounded(value: &str, max: usize, label: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > max || value.contains('\0') {
        return Err(format!("Invalid {label}"));
    }
    Ok(())
}
fn word_matches(entry: &Word, language: &str, word: &str) -> bool {
    entry.word == word
        && (entry.language == language
            || (!entry.language.contains('-')
                && language.starts_with(&format!("{}-", entry.language))))
}
pub fn parse_reply(
    raw: &[u8],
    text: &str,
    language: &LanguageDecision,
    config: &Config,
    dictionary: &Dictionary,
) -> Result<Checked, String> {
    config.validate()?;
    dictionary.validate()?;
    if raw.is_empty() || raw.len() > MAX_RESPONSE_BYTES {
        return Err("Provider response byte bound".into());
    }
    if text.len() > MAX_TEXT_BYTES || text.encode_utf16().count() > MAX_TEXT_UTF16 {
        return Err("Checked text bound".into());
    }
    let reply: Reply =
        serde_json::from_slice(raw).map_err(|e| format!("Malformed LanguageTool response: {e}"))?;
    if reply.software.name != "LanguageTool" || reply.software.api_version == 0 {
        return Err("Unexpected provider identity".into());
    }
    bounded(&reply.software.version, 128, "provider version")?;
    bounded(&reply.software.build_date, 128, "provider build")?;
    if config
        .expected_engine_version
        .as_ref()
        .is_some_and(|v| v != &reply.software.version)
    {
        return Err("LanguageTool version differs from explicit pin".into());
    }
    if language_tag(&reply.language.code)? != language.requested {
        return Err("Provider language differs from requested regional language".into());
    }
    bounded(&reply.language.name, 128, "provider language")?;
    if reply.matches.len() > MAX_ISSUES {
        return Err("Provider issue count exceeds1000".into());
    }
    let mut issues = Vec::new();
    for m in reply.matches {
        let matched = utf16_slice(text, m.offset, m.length)?.to_owned();
        if m.context.length != m.length
            || utf16_slice(&m.context.text, m.context.offset, m.context.length)? != matched
        {
            return Err("Provider context/range differs from checked text".into());
        }
        for (v, max, label) in [
            (&m.message, 4096, "message"),
            (&m.rule.id, 128, "rule"),
            (&m.rule.description, 1024, "description"),
            (&m.rule.issue_type, 128, "issue type"),
            (&m.rule.category.id, 128, "category"),
            (&m.rule.category.name, 256, "category name"),
            (&m.context.text, 8192, "context"),
        ] {
            bounded(v, max, label)?;
        }
        if m.short_message.len() > 1024 || m.short_message.contains('\0') {
            return Err("Invalid short message".into());
        }
        if let Some(v) = &m.rule.sub_id {
            bounded(v, 128, "sub rule")?;
        }
        if m.replacements.len() > 512
            || m.replacements
                .iter()
                .any(|v| v.value.len() > 1024 || v.value.contains('\0'))
        {
            return Err("Provider replacements exceed bound".into());
        }
        let spelling = m.rule.issue_type == "misspelling";
        let disposition = if config.ignored_rules.contains(&m.rule.id) {
            "ignored_rule"
        } else if spelling
            && dictionary
                .words
                .iter()
                .any(|v| word_matches(v, &language.requested, &matched))
        {
            "dictionary"
        } else if spelling
            && config
                .ignored_words
                .iter()
                .any(|v| word_matches(v, &language.requested, &matched))
        {
            "ignored_word"
        } else if (spelling && !config.spelling) || (!spelling && !config.grammar) {
            "disabled_kind"
        } else {
            "accepted"
        };
        issues.push(Issue {
            offset_utf16: m.offset,
            length_utf16: m.length,
            matched_text: matched,
            message: m.message,
            short_message: m.short_message,
            context: m.context.text,
            context_offset_utf16: m.context.offset,
            rule_id: m.rule.id,
            rule_sub_id: m.rule.sub_id,
            rule_description: m.rule.description,
            issue_type: m.rule.issue_type,
            category_id: m.rule.category.id,
            category_name: m.rule.category.name,
            replacements: m.replacements.into_iter().map(|v| v.value).collect(),
            disposition: disposition.into(),
        });
    }
    let complete = !reply.warnings.incomplete_results;
    let accepted_count = complete.then(|| {
        issues
            .iter()
            .filter(|v| v.disposition == "accepted")
            .count()
    });
    Ok(Checked {
        software: reply.software,
        response_language: reply.language.code,
        response_language_name: reply.language.name,
        complete,
        response_sha256: digest(raw),
        response_bytes: raw.len(),
        issues,
        accepted_count,
    })
}

#[cfg(test)]
mod tests;
