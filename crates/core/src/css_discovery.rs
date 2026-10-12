//! Bounded inventory of declared dependencies in supplied decoded UTF-8 CSS. No I/O.
//! Syntax recovery, URL eligibility and browser/cascade evaluation are separate facts.
use anyhow::{Context, Result, ensure};
use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, Parser, ParserInput, ParserState,
    QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser, Token,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

pub const CSS_DISCOVERY_ENGINE: &str = "open-seo-spider/css-dependency-inventory-v1";
pub const CSS_DISCOVERY_PARSER_PROFILE: &str = "cssparser-0.37.0-contextual-declarations-v1";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CssSourceSyntax {
    #[default]
    Stylesheet,
    DeclarationList,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CssSourceInput {
    pub css_text: String,
    pub base_url: String,
    pub source_url: String,
    pub source_revision: String,
    pub inline: bool,
    #[serde(default)]
    pub syntax: CssSourceSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CssInventoryLimits {
    pub max_utf8_bytes: usize,
    pub max_tokens: usize,
    pub max_nesting: usize,
    pub max_candidates: usize,
    pub max_diagnostics: usize,
    pub max_url_utf8_bytes: usize,
    pub max_retained_utf8_bytes: usize,
}

impl Default for CssInventoryLimits {
    fn default() -> Self {
        Self {
            max_utf8_bytes: 16 * 1024 * 1024,
            max_tokens: 1_000_000,
            max_nesting: 64,
            max_candidates: 100_000,
            max_diagnostics: 1000,
            max_url_utf8_bytes: 65_536,
            max_retained_utf8_bytes: 32 * 1024 * 1024,
        }
    }
}

impl CssInventoryLimits {
    pub fn validate(&self) -> Result<()> {
        for (name, value, ceiling) in [
            ("max_utf8_bytes", self.max_utf8_bytes, 64 * 1024 * 1024),
            ("max_tokens", self.max_tokens, 8_000_000),
            ("max_nesting", self.max_nesting, 128),
            ("max_candidates", self.max_candidates, 1_000_000),
            ("max_diagnostics", self.max_diagnostics, 10_000),
            ("max_url_utf8_bytes", self.max_url_utf8_bytes, 65_536),
            (
                "max_retained_utf8_bytes",
                self.max_retained_utf8_bytes,
                128 * 1024 * 1024,
            ),
        ] {
            ensure!(
                (1..=ceiling).contains(&value),
                "{name} must be within1..={ceiling}"
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CssDependencyCategory {
    Stylesheet,
    Image,
    Font,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CssResolution {
    pub raw: String,
    pub value: f64,
    pub unit: String,
    pub valid: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CssImageSetCandidate {
    pub resolution: Option<CssResolution>,
    pub declared_mime: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CssDependency {
    /// Positive occurrence index in document order; duplicates are retained.
    pub source_ordinal: usize,
    pub decoded_url: String,
    /// HTTP(S) target only; this is not permission to fetch or browser request evidence.
    pub resolved_url: Option<String>,
    pub resolution_error: Option<String>,
    pub raw_token: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub property: Option<String>,
    pub at_rule: Option<String>,
    pub at_rule_path: Vec<String>,
    pub category_hint: CssDependencyCategory,
    pub image_set: Option<CssImageSetCandidate>,
    /// Custom properties and substitution alternatives are not known to be applied.
    pub deferred: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CssDiagnostic {
    pub code: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CssInventory {
    pub engine: String,
    pub parser_profile: String,
    pub source_url: String,
    pub source_revision: String,
    pub base_url: String,
    pub inline: bool,
    pub syntax: CssSourceSyntax,
    pub source_utf8_bytes: usize,
    pub source_utf8_sha256: String,
    pub limits_sha256: String,
    pub dependencies: Vec<CssDependency>,
    pub diagnostics: Vec<CssDiagnostic>,
    /// Complete only under this declared-dependency profile, not CSS semantics or body decoding.
    pub complete: bool,
    pub visited_tokens: usize,
    pub max_observed_nesting: usize,
    pub coverage_limits: Vec<String>,
    pub website_requests: usize,
}

type CssResult<'i, T = ()> = std::result::Result<T, ParseError<'i, String>>;

#[derive(Clone)]
struct SpanToken<'i> {
    token: Token<'i>,
    start: usize,
    end: usize,
}

fn next_token<'i>(input: &mut Parser<'i, '_>) -> Option<SpanToken<'i>> {
    let start = input.position().byte_index();
    let token = input.next_including_whitespace_and_comments().ok()?.clone();
    Some(SpanToken {
        token,
        start,
        end: input.position().byte_index(),
    })
}

fn next_significant<'i>(input: &mut Parser<'i, '_>) -> Option<SpanToken<'i>> {
    loop {
        let token = next_token(input)?;
        if !matches!(token.token, Token::WhiteSpace(_) | Token::Comment(_)) {
            return Some(token);
        }
    }
}

fn opening_close(token: &Token<'_>) -> Option<u8> {
    match token {
        Token::Function(_) | Token::ParenthesisBlock => Some(b')'),
        Token::SquareBracketBlock => Some(b']'),
        Token::CurlyBracketBlock => Some(b'}'),
        _ => None,
    }
}

fn closed_string(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    if bytes.len() < 2 || bytes.last() != bytes.first() {
        return false;
    }
    bytes[..bytes.len() - 1]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count()
        % 2
        == 0
}

fn supplied_delimiter(raw: &str, delimiter: u8) -> bool {
    let bytes = raw.as_bytes();
    bytes.last() == Some(&delimiter)
        && bytes[..bytes.len().saturating_sub(1)]
            .iter()
            .rev()
            .take_while(|&&byte| byte == b'\\')
            .count()
            % 2
            == 0
}

struct Preflight<'s> {
    source: &'s str,
    limits: &'s CssInventoryLimits,
    tokens: usize,
    nesting: usize,
    diagnostics: Vec<CssDiagnostic>,
}

impl Preflight<'_> {
    fn diagnostic<'i>(
        &mut self,
        parser: &Parser<'i, '_>,
        code: &str,
        start: usize,
        end: usize,
        message: &str,
    ) -> CssResult<'i> {
        if self.diagnostics.len() >= self.limits.max_diagnostics {
            return Err(parser.new_custom_error::<_, String>("CSS max_diagnostics exceeded"));
        }
        self.diagnostics.push(CssDiagnostic {
            code: code.into(),
            byte_start: start,
            byte_end: end,
            message: message.into(),
        });
        Ok(())
    }

    fn visit<'i>(&mut self, input: &mut Parser<'i, '_>, depth: usize) -> CssResult<'i> {
        if depth > self.limits.max_nesting {
            return Err(input.new_custom_error::<_, String>("CSS max_nesting exceeded"));
        }
        self.nesting = self.nesting.max(depth);
        while let Some(token) = next_token(input) {
            self.tokens = self
                .tokens
                .checked_add(1)
                .ok_or_else(|| input.new_custom_error::<_, String>("CSS token counter overflow"))?;
            if self.tokens > self.limits.max_tokens {
                return Err(input.new_custom_error::<_, String>("CSS max_tokens exceeded"));
            }
            if token.token.is_parse_error() {
                self.diagnostic(input, "css_bad_token", token.start, token.end,
                    "Tokenizer emitted a malformed or unmatched token; no repaired URL is inferred.")?;
            }
            let raw = &self.source[token.start..token.end];
            if matches!(token.token, Token::QuotedString(_)) && !closed_string(raw) {
                self.diagnostic(input, "css_eof_string_recovery", token.start, token.end,
                    "Tokenizer recovered an EOF-terminated string; it is not a complete URL operand.")?;
            }
            if matches!(token.token, Token::UnquotedUrl(_)) && !supplied_delimiter(raw, b')') {
                self.diagnostic(
                    input,
                    "css_eof_url_recovery",
                    token.start,
                    token.end,
                    "Tokenizer recovered an EOF-terminated URL token; no fetch target is inferred.",
                )?;
            }
            if matches!(token.token, Token::Comment(_)) && !raw.ends_with("*/") {
                self.diagnostic(
                    input,
                    "css_eof_comment_recovery",
                    token.start,
                    token.end,
                    "Tokenizer recovered an EOF-terminated comment; source completeness is unknown.",
                )?;
            }
            if let Some(close) = opening_close(&token.token) {
                let mut stop = token.end;
                input.parse_nested_block(|nested| {
                    self.visit(nested, depth + 1)?;
                    stop = nested.position().byte_index();
                    Ok(())
                })?;
                let end = input.position().byte_index();
                if self.source.as_bytes().get(stop) == Some(&close) && end == stop + 1 {
                    self.tokens = self.tokens.checked_add(1).ok_or_else(|| {
                        input.new_custom_error::<_, String>("CSS token counter overflow")
                    })?;
                    if self.tokens > self.limits.max_tokens {
                        return Err(input.new_custom_error::<_, String>("CSS max_tokens exceeded"));
                    }
                } else {
                    self.diagnostic(input, "css_eof_block_recovery", token.start, end,
                        "Parser supplied an EOF block/function closure; source completeness is unknown.")?;
                }
            }
        }
        Ok(())
    }
}

struct Extractor<'s> {
    source: &'s str,
    limits: &'s CssInventoryLimits,
    base: Url,
    dependencies: Vec<CssDependency>,
    diagnostics: Vec<CssDiagnostic>,
    at_rules: Vec<String>,
    retained_bytes: usize,
    fatal: Option<String>,
}

#[derive(Clone)]
struct ValueContext {
    property: String,
    category: CssDependencyCategory,
    deferred: bool,
    inside_image_set: bool,
}

struct AtPrelude {
    name: String,
    reference: Option<(String, usize, usize)>,
}

impl Extractor<'_> {
    fn fail<'i, T>(&mut self, input: &Parser<'i, '_>, message: &str) -> CssResult<'i, T> {
        self.fatal = Some(message.into());
        Err(input.new_custom_error::<_, String>(message))
    }

    fn diagnostic<'i>(
        &mut self,
        input: &Parser<'i, '_>,
        code: &str,
        start: usize,
        end: usize,
        message: &str,
    ) -> CssResult<'i> {
        if self.diagnostics.len() >= self.limits.max_diagnostics {
            return self.fail(input, "CSS max_diagnostics exceeded");
        }
        self.diagnostics.push(CssDiagnostic {
            code: code.into(),
            byte_start: start,
            byte_end: end,
            message: message.into(),
        });
        Ok(())
    }

    fn rules<'i>(&mut self, input: &mut Parser<'i, '_>) -> CssResult<'i> {
        let mut rules = StyleSheetParser::new(input, self);
        while let Some(result) = rules.next() {
            if rules.parser.fatal.is_some() {
                return Err(rules
                    .input
                    .new_custom_error::<_, String>(rules.parser.fatal.clone().unwrap()));
            }
            if let Err((_, slice)) = result {
                let end = rules.input.position().byte_index();
                rules.parser.diagnostic(rules.input, "css_rule_recovery", end.saturating_sub(slice.len()), end,
                    "A rule could not be inventoried under this profile; later independent rules are retained.")?;
            }
        }
        Ok(())
    }

    fn body<'i>(&mut self, input: &mut Parser<'i, '_>) -> CssResult<'i> {
        let mut rules = RuleBodyParser::new(input, self);
        while let Some(result) = rules.next() {
            if rules.parser.fatal.is_some() {
                return Err(rules
                    .input
                    .new_custom_error::<_, String>(rules.parser.fatal.clone().unwrap()));
            }
            if let Err((_, slice)) = result {
                let end = rules.input.position().byte_index();
                rules.parser.diagnostic(rules.input, "css_declaration_recovery", end.saturating_sub(slice.len()), end,
                    "A declaration or nested rule is unsupported or malformed; no repaired dependency is inferred.")?;
            }
        }
        Ok(())
    }

    fn ignored(input: &mut Parser<'_, '_>) {
        while input.next().is_ok() {}
    }

    fn reference<'i>(
        &mut self,
        input: &mut Parser<'i, '_>,
        token: SpanToken<'i>,
        allow_string: bool,
    ) -> CssResult<'i, Option<(String, usize, usize)>> {
        match token.token {
            Token::UnquotedUrl(value) => {
                if supplied_delimiter(&self.source[token.start..token.end], b')') {
                    Ok(Some((value.to_string(), token.start, token.end)))
                } else {
                    Ok(None)
                }
            }
            Token::QuotedString(value) if allow_string => {
                if closed_string(&self.source[token.start..token.end]) {
                    Ok(Some((value.to_string(), token.start, token.end)))
                } else {
                    Ok(None)
                }
            }
            Token::Function(name) if name.eq_ignore_ascii_case("url") => {
                let mut value = None;
                let mut close_pos = token.end;
                input.parse_nested_block(|nested| {
                    if let Some(string) = next_significant(nested)
                        && let Token::QuotedString(decoded) = string.token
                        && closed_string(&self.source[string.start..string.end])
                    {
                        value = Some(decoded.to_string());
                    }
                    if nested.next().is_ok() {
                        value = None;
                        Self::ignored(nested);
                    }
                    close_pos = nested.position().byte_index();
                    Ok::<(), ParseError<'i, String>>(())
                })?;
                let end = input.position().byte_index();
                if self.source.as_bytes().get(close_pos) != Some(&b')') || end != close_pos + 1 {
                    value = None;
                }
                if let Some(value) = value {
                    Ok(Some((value, token.start, end)))
                } else {
                    self.diagnostic(input, "css_url_operand_invalid", token.start, end,
                        "Supported url() requires one complete quoted string and its supplied closing delimiter.")?;
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn append<'i>(
        &mut self,
        input: &Parser<'i, '_>,
        reference: (String, usize, usize),
        context: Option<&ValueContext>,
        category: CssDependencyCategory,
    ) -> CssResult<'i> {
        let (decoded, start, end) = reference;
        if decoded.len() > self.limits.max_url_utf8_bytes {
            return self.fail(input, "CSS max_url_utf8_bytes exceeded");
        }
        if self.dependencies.len() >= self.limits.max_candidates {
            return self.fail(input, "CSS max_candidates exceeded");
        }
        let (resolved_url, resolution_error) = resolve(&self.base, &decoded);
        let dependency = CssDependency {
            source_ordinal: self.dependencies.len() + 1,
            decoded_url: decoded,
            resolved_url,
            resolution_error,
            raw_token: self.source[start..end].into(),
            byte_start: start,
            byte_end: end,
            property: context.map(|item| item.property.clone()),
            at_rule: self.at_rules.last().cloned(),
            at_rule_path: self.at_rules.clone(),
            category_hint: category,
            image_set: None,
            deferred: context.is_some_and(|item| item.deferred),
        };
        let size = serde_json::to_vec(&dependency)
            .map_err(|_| {
                input.new_custom_error::<_, String>("CSS dependency serialization failed")
            })?
            .len();
        self.retained_bytes = self.retained_bytes.checked_add(size).ok_or_else(|| {
            input.new_custom_error::<_, String>("CSS retained byte counter overflow")
        })?;
        if self.retained_bytes > self.limits.max_retained_utf8_bytes {
            return self.fail(input, "CSS max_retained_utf8_bytes exceeded");
        }
        self.dependencies.push(dependency);
        Ok(())
    }

    fn value<'i>(&mut self, input: &mut Parser<'i, '_>, context: &ValueContext) -> CssResult<'i> {
        while let Some(token) = next_significant(input) {
            self.value_token(input, token, context)?;
        }
        Ok(())
    }

    fn value_token<'i>(
        &mut self,
        input: &mut Parser<'i, '_>,
        token: SpanToken<'i>,
        context: &ValueContext,
    ) -> CssResult<'i> {
        match &token.token {
            Token::UnquotedUrl(_) => {
                if let Some(reference) = self.reference(input, token, false)? {
                    self.append(input, reference, Some(context), context.category)?;
                }
            }
            Token::Function(name) if name.eq_ignore_ascii_case("url") => {
                if let Some(reference) = self.reference(input, token, false)? {
                    self.append(input, reference, Some(context), context.category)?;
                }
            }
            Token::Function(name)
                if name.eq_ignore_ascii_case("image-set")
                    || name.eq_ignore_ascii_case("-webkit-image-set") =>
            {
                if context.inside_image_set {
                    input.parse_nested_block(|nested| {
                        Self::ignored(nested);
                        Ok::<(), ParseError<'i, String>>(())
                    })?;
                    self.diagnostic(
                        input,
                        "css_image_set_nested_unsupported",
                        token.start,
                        input.position().byte_index(),
                        "An image-set option cannot contain image-set() directly or indirectly; inner targets are not inferred.",
                    )?;
                } else {
                    input.parse_nested_block(|nested| self.image_set(nested, context))?;
                }
            }
            Token::Function(name) if name.eq_ignore_ascii_case("image") => {
                input.parse_nested_block(|nested| self.image(nested, context))?;
            }
            Token::Function(name)
                if name.eq_ignore_ascii_case("local") || name.eq_ignore_ascii_case("type") =>
            {
                input.parse_nested_block(|nested| {
                    Self::ignored(nested);
                    Ok::<(), ParseError<'i, String>>(())
                })?;
            }
            Token::Function(name) if name.eq_ignore_ascii_case("src") => {
                input.parse_nested_block(|nested| {
                    Self::ignored(nested);
                    Ok::<(), ParseError<'i, String>>(())
                })?;
                self.diagnostic(
                    input,
                    "css_src_function_unsupported",
                    token.start,
                    input.position().byte_index(),
                    "Modern src() URL modifiers are outside this declared-dependency profile.",
                )?;
            }
            Token::Function(name) => {
                let mut nested_context = context.clone();
                if name.eq_ignore_ascii_case("var") || name.eq_ignore_ascii_case("env") {
                    nested_context.deferred = true;
                }
                input.parse_nested_block(|nested| self.value(nested, &nested_context))?;
            }
            Token::CurlyBracketBlock if !context.property.starts_with("--") => {
                // Enables the public rule-body parser to retry a nested selector instead of
                // treating `a:hover { ... }` as a property named `a`.
                return Err(
                    input.new_custom_error::<_, String>("Nested rule is not a declaration value")
                );
            }
            Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                input.parse_nested_block(|nested| self.value(nested, context))?;
            }
            _ => {}
        }
        Ok(())
    }

    fn image<'i>(&mut self, input: &mut Parser<'i, '_>, context: &ValueContext) -> CssResult<'i> {
        let mut first = next_significant(input);
        if first.as_ref().is_some_and(|token| matches!(&token.token, Token::Ident(value) if value.eq_ignore_ascii_case("ltr") || value.eq_ignore_ascii_case("rtl"))) {
            first = next_significant(input);
        }
        if let Some(token) = first
            && let Some(reference) = self.reference(input, token, true)?
        {
            self.append(
                input,
                reference,
                Some(context),
                CssDependencyCategory::Image,
            )?;
        }
        Self::ignored(input);
        Ok(())
    }

    fn image_set<'i>(
        &mut self,
        input: &mut Parser<'i, '_>,
        context: &ValueContext,
    ) -> CssResult<'i> {
        let mut first = true;
        let mut start = self.dependencies.len();
        let mut resolution = None;
        let mut mime = None;
        let mut invalid = false;
        let mut diagnostic_start = self.diagnostics.len();
        while let Some(token) = next_significant(input) {
            if matches!(token.token, Token::Comma) {
                if first {
                    invalid = true;
                }
                invalid |= self.diagnostics[diagnostic_start..]
                    .iter()
                    .any(|item| item.code == "css_image_set_nested_unsupported");
                self.finish_image_option(input, start, resolution.take(), mime.take(), invalid)?;
                start = self.dependencies.len();
                first = true;
                invalid = false;
                diagnostic_start = self.diagnostics.len();
                continue;
            }
            if first {
                first = false;
                match &token.token {
                    Token::QuotedString(_) | Token::UnquotedUrl(_) => {
                        if let Some(reference) = self.reference(input, token, true)? {
                            self.append(
                                input,
                                reference,
                                Some(context),
                                CssDependencyCategory::Image,
                            )?;
                        } else {
                            invalid = true;
                        }
                    }
                    Token::Function(name) if name.eq_ignore_ascii_case("url") => {
                        if let Some(reference) = self.reference(input, token, false)? {
                            self.append(
                                input,
                                reference,
                                Some(context),
                                CssDependencyCategory::Image,
                            )?;
                        } else {
                            invalid = true;
                        }
                    }
                    Token::Function(name)
                        if name.eq_ignore_ascii_case("image-set")
                            || name.eq_ignore_ascii_case("-webkit-image-set") =>
                    {
                        invalid = true;
                        input.parse_nested_block(|nested| {
                            Self::ignored(nested);
                            Ok::<(), ParseError<'i, String>>(())
                        })?;
                        self.diagnostic(
                            input,
                            "css_image_set_nested_unsupported",
                            token.start,
                            input.position().byte_index(),
                            "An image-set option cannot contain image-set() directly or indirectly; inner targets are not inferred.",
                        )?;
                    }
                    Token::Function(_) => {
                        let mut image_context = context.clone();
                        image_context.category = CssDependencyCategory::Image;
                        image_context.inside_image_set = true;
                        self.value_token(input, token, &image_context)?;
                    }
                    _ => {
                        invalid = true;
                    }
                }
                continue;
            }
            match token.token {
                Token::Dimension { value, unit, .. } => {
                    let unit = unit.to_ascii_lowercase();
                    let valid = value.is_finite()
                        && value > 0.0
                        && ["x", "dppx", "dpi", "dpcm"].contains(&unit.as_str());
                    if resolution.is_some() || !valid {
                        invalid = true;
                    }
                    resolution = Some(CssResolution {
                        raw: self.source[token.start..token.end].into(),
                        value: if value.is_finite() { value.into() } else { 0.0 },
                        unit,
                        valid,
                    });
                }
                Token::Function(name) if name.eq_ignore_ascii_case("type") => {
                    let mut value = None;
                    input.parse_nested_block(|nested| {
                        if let Some(token) = next_significant(nested)
                            && let Token::QuotedString(string) = token.token
                            && closed_string(&self.source[token.start..token.end])
                        {
                            value = Some(string.to_string());
                        }
                        if nested.next().is_ok() {
                            value = None;
                            Self::ignored(nested);
                        }
                        Ok::<(), ParseError<'i, String>>(())
                    })?;
                    if mime.is_some() || value.is_none() {
                        invalid = true;
                    }
                    if value
                        .as_ref()
                        .is_some_and(|item| item.len() > self.limits.max_url_utf8_bytes)
                    {
                        return self.fail(
                            input,
                            "CSS image-set type metadata exceeds max_url_utf8_bytes",
                        );
                    }
                    mime = value;
                }
                _ => {
                    invalid = true;
                    if opening_close(&token.token).is_some() {
                        input.parse_nested_block(|nested| {
                            Self::ignored(nested);
                            Ok::<(), ParseError<'i, String>>(())
                        })?;
                    }
                }
            }
        }
        if first {
            invalid = true;
        }
        invalid |= self.diagnostics[diagnostic_start..]
            .iter()
            .any(|item| item.code == "css_image_set_nested_unsupported");
        self.finish_image_option(input, start, resolution, mime, invalid)
    }

    fn finish_image_option<'i>(
        &mut self,
        input: &Parser<'i, '_>,
        start: usize,
        resolution: Option<CssResolution>,
        declared_mime: Option<String>,
        invalid: bool,
    ) -> CssResult<'i> {
        for dependency in &mut self.dependencies[start..] {
            let old = serde_json::to_vec(dependency)
                .map_err(|_| {
                    input.new_custom_error::<_, String>("CSS metadata serialization failed")
                })?
                .len();
            dependency.image_set = Some(CssImageSetCandidate {
                resolution: resolution.clone(),
                declared_mime: declared_mime.clone(),
            });
            if invalid {
                dependency.resolved_url = None;
                dependency.resolution_error = Some("css_image_set_option_invalid".into());
            }
            let new = serde_json::to_vec(dependency)
                .map_err(|_| {
                    input.new_custom_error::<_, String>("CSS metadata serialization failed")
                })?
                .len();
            self.retained_bytes = self
                .retained_bytes
                .checked_sub(old)
                .and_then(|value| value.checked_add(new))
                .ok_or_else(|| {
                    input.new_custom_error::<_, String>("CSS retained byte counter overflow")
                })?;
        }
        if self.retained_bytes > self.limits.max_retained_utf8_bytes {
            return self.fail(input, "CSS max_retained_utf8_bytes exceeded");
        }
        if invalid {
            let position = input.position().byte_index();
            self.diagnostic(input, "css_image_set_option_invalid", position, position,
                "An image-set option is outside the supported URL/resolution/type grammar; no target from that option is eligible.")?;
        }
        Ok(())
    }
}

impl<'i> DeclarationParser<'i> for Extractor<'_> {
    type Declaration = ();
    type Error = String;
    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
        _: &ParserState,
    ) -> CssResult<'i> {
        let property = if name.starts_with("--") {
            name.to_string()
        } else {
            name.to_ascii_lowercase()
        };
        if property.len() > self.limits.max_url_utf8_bytes {
            return self.fail(input, "CSS property metadata exceeds max_url_utf8_bytes");
        }
        let context = ValueContext {
            category: category(&property, self.at_rules.last().map(String::as_str)),
            deferred: property.starts_with("--"),
            inside_image_set: false,
            property,
        };
        let count = self.dependencies.len();
        let bytes = self.retained_bytes;
        let result = self.value(input, &context);
        if result.is_err() {
            self.dependencies.truncate(count);
            self.retained_bytes = bytes;
        }
        result
    }
}

impl<'i> QualifiedRuleParser<'i> for Extractor<'_> {
    type Prelude = ();
    type QualifiedRule = ();
    type Error = String;
    fn parse_prelude<'t>(&mut self, input: &mut Parser<'i, 't>) -> CssResult<'i> {
        Self::ignored(input);
        Ok(())
    }
    fn parse_block<'t>(
        &mut self,
        _: (),
        _: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> CssResult<'i> {
        self.body(input)
    }
}

impl<'i> AtRuleParser<'i> for Extractor<'_> {
    type Prelude = AtPrelude;
    type AtRule = ();
    type Error = String;
    fn parse_prelude<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
    ) -> CssResult<'i, AtPrelude> {
        let name = name.to_ascii_lowercase();
        if name.len() > self.limits.max_url_utf8_bytes {
            return self.fail(input, "CSS at-rule metadata exceeds max_url_utf8_bytes");
        }
        let reference = if name == "import" {
            if let Some(token) = next_significant(input) {
                self.reference(input, token, true)?
            } else {
                None
            }
        } else {
            None
        };
        Self::ignored(input);
        Ok(AtPrelude { name, reference })
    }
    fn rule_without_block(
        &mut self,
        prelude: AtPrelude,
        _: &ParserState,
    ) -> std::result::Result<(), ()> {
        if prelude.name == "import" {
            if let Some(reference) = prelude.reference {
                let mut backing = ParserInput::new("");
                let parser = Parser::new(&mut backing);
                self.at_rules.push("import".into());
                let result =
                    self.append(&parser, reference, None, CssDependencyCategory::Stylesheet);
                self.at_rules.pop();
                result.map_err(|_| ())?;
            } else {
                return Err(());
            }
        }
        Ok(())
    }
    fn parse_block<'t>(
        &mut self,
        prelude: AtPrelude,
        _: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> CssResult<'i> {
        self.at_rules.push(prelude.name.clone());
        let result = match prelude.name.as_str() {
            "media" | "supports" | "layer" | "container" | "scope" | "starting-style"
            | "keyframes" | "-webkit-keyframes" | "document" | "-moz-document" => self.rules(input),
            "font-face" | "page" | "counter-style" | "property" | "font-palette-values" => {
                self.body(input)
            }
            _ => {
                let start = input.position().byte_index();
                Self::ignored(input);
                self.diagnostic(input, "css_at_rule_block_unsupported", start, input.position().byte_index(),
                    "This at-rule block is outside the versioned dependency contexts; absence is not proven.")
            }
        };
        self.at_rules.pop();
        result
    }
}

impl<'i> RuleBodyItemParser<'i, (), String> for Extractor<'_> {
    fn parse_declarations(&self) -> bool {
        true
    }
    fn parse_qualified(&self) -> bool {
        true
    }
}

fn category(property: &str, at_rule: Option<&str>) -> CssDependencyCategory {
    if at_rule == Some("font-face") && property == "src" {
        return CssDependencyCategory::Font;
    }
    if [
        "background",
        "background-image",
        "border-image",
        "border-image-source",
        "list-style",
        "list-style-image",
        "mask",
        "mask-image",
        "mask-border",
        "mask-border-source",
        "-webkit-mask",
        "-webkit-mask-image",
        "cursor",
        "content",
        "shape-outside",
        "symbols",
    ]
    .contains(&property)
    {
        CssDependencyCategory::Image
    } else {
        CssDependencyCategory::Other
    }
}

fn resolve(base: &Url, value: &str) -> (Option<String>, Option<String>) {
    let url_input = value.trim_matches(|character: char| character <= '\u{20}');
    if url_input.is_empty() {
        return (None, Some("empty_reference".into()));
    }
    if url_input.starts_with('#') {
        return (None, Some("local_fragment_reference".into()));
    }
    match base.join(url_input) {
        Err(_) => (None, Some("invalid_url".into())),
        Ok(mut url) if ["http", "https"].contains(&url.scheme()) => {
            if !url.username().is_empty() || url.password().is_some() {
                return (None, Some("url_credentials_not_supported".into()));
            }
            url.set_fragment(None);
            (Some(url.to_string()), None)
        }
        Ok(_) => (None, Some("non_http_reference".into())),
    }
}

/// Validate and inventory an already decoded source. Bounds fail atomically with Err.
/// `complete` relates only to this parser profile; the caller must bind response/decode coverage.
pub fn discover_css(input: &CssSourceInput, limits: &CssInventoryLimits) -> Result<CssInventory> {
    limits.validate()?;
    ensure!(
        input.css_text.len() <= limits.max_utf8_bytes,
        "CSS max_utf8_bytes exceeded"
    );
    ensure!(
        !input.source_revision.is_empty() && input.source_revision.len() <= 1024,
        "CSS source_revision must have1..1024 UTF-8 bytes"
    );
    for (name, value) in [
        ("base_url", &input.base_url),
        ("source_url", &input.source_url),
    ] {
        ensure!(
            value.len() <= limits.max_url_utf8_bytes,
            "CSS {name} exceeds max_url_utf8_bytes"
        );
        let parsed = Url::parse(value).with_context(|| format!("Invalid CSS {name}"))?;
        ensure!(
            ["http", "https"].contains(&parsed.scheme()) && parsed.has_host(),
            "CSS {name} must be absolute HTTP(S)"
        );
        ensure!(
            parsed.username().is_empty() && parsed.password().is_none(),
            "CSS {name} cannot contain URL credentials"
        );
    }
    let base = Url::parse(&input.base_url)?;
    let mut preflight = Preflight {
        source: &input.css_text,
        limits,
        tokens: 0,
        nesting: 0,
        diagnostics: Vec::new(),
    };
    let mut first = ParserInput::new(&input.css_text);
    preflight
        .visit(&mut Parser::new(&mut first), 0)
        .map_err(|error| anyhow::anyhow!("CSS preflight: {error:?}"))?;
    let tokens = preflight.tokens;
    let nesting = preflight.nesting;
    let mut extractor = Extractor {
        source: &input.css_text,
        limits,
        base,
        dependencies: Vec::new(),
        diagnostics: preflight.diagnostics,
        at_rules: Vec::new(),
        retained_bytes: 0,
        fatal: None,
    };
    let mut second = ParserInput::new(&input.css_text);
    let mut parser = Parser::new(&mut second);
    let result = match input.syntax {
        CssSourceSyntax::Stylesheet => extractor.rules(&mut parser),
        CssSourceSyntax::DeclarationList => extractor.body(&mut parser),
    };
    if let Some(fatal) = extractor.fatal {
        anyhow::bail!(fatal);
    }
    result.map_err(|error| anyhow::anyhow!("CSS contextual inventory: {error:?}"))?;
    let inventory = CssInventory {
        engine: CSS_DISCOVERY_ENGINE.into(), parser_profile: CSS_DISCOVERY_PARSER_PROFILE.into(),
        source_url: input.source_url.clone(), source_revision: input.source_revision.clone(),
        base_url: input.base_url.clone(), inline: input.inline, syntax: input.syntax,
        source_utf8_bytes: input.css_text.len(), source_utf8_sha256: hex_sha(input.css_text.as_bytes()),
        limits_sha256: hex_sha(&serde_json::to_vec(limits)?), dependencies: extractor.dependencies,
        complete: extractor.diagnostics.is_empty(), diagnostics: extractor.diagnostics,
        visited_tokens: tokens, max_observed_nesting: nesting,
        coverage_limits: [
            "Declared dependency inventory, not browser selection, execution, cascade or conditional evaluation.",
            "Supplied UTF-8 source hash is not HTTP entity/wire hash; response MIME, decode/body completeness are caller-owned.",
            "Property grammar validity and arbitrary substitutions are not evaluated; custom-property and var/env references are deferred.",
            "Modern src() modifiers, unsupported at-rule blocks and indirect image-set nesting require separate profiles.",
            "Font hints use other-resource controls unless caller defines a distinct font policy; namespace/type/local names are not fetch targets.",
        ].into_iter().map(String::from).collect(), website_requests: 0,
    };
    ensure!(
        serde_json::to_vec(&inventory)?.len() <= limits.max_retained_utf8_bytes,
        "CSS max_retained_utf8_bytes exceeded by inventory"
    );
    Ok(inventory)
}

fn hex_sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(css: &str) -> CssSourceInput {
        CssSourceInput {
            css_text: css.into(),
            base_url: "https://example.test/styles/current.css".into(),
            source_url: "https://example.test/styles/current.css".into(),
            source_revision: "own-source-v1".into(),
            inline: false,
            syntax: CssSourceSyntax::Stylesheet,
        }
    }
    fn inventory(css: &str) -> CssInventory {
        discover_css(&input(css), &CssInventoryLimits::default()).unwrap()
    }
    fn values(result: &CssInventory) -> Vec<&str> {
        result
            .dependencies
            .iter()
            .map(|item| item.decoded_url.as_str())
            .collect()
    }

    #[test]
    fn escaped_quoted_unquoted_and_exact_original_spans() {
        let css = "/* Grüße 🐸 url(no.png) */ .x { background:u\\72l(\"im\\61 ge.png\"); mask:url(mask.svg#clip); }";
        let result = inventory(css);
        assert_eq!(values(&result), ["image.png", "mask.svg#clip"]);
        assert!(result.complete);
        for (index, dependency) in result.dependencies.iter().enumerate() {
            assert_eq!(dependency.source_ordinal, index + 1);
            assert_eq!(
                &css[dependency.byte_start..dependency.byte_end],
                dependency.raw_token
            );
        }
        assert_eq!(
            result.dependencies[0].resolved_url.as_deref(),
            Some("https://example.test/styles/image.png")
        );
        assert_eq!(
            result.dependencies[1].resolved_url.as_deref(),
            Some("https://example.test/styles/mask.svg")
        );
        assert_eq!(result.website_requests, 0);
    }

    #[test]
    fn nested_rule_context_import_duplicates_and_final_base() {
        let result = inventory(
            "@import \"theme.css\" layer(site) supports(display:grid) screen; @import url(theme.css); @media print { @supports (display:grid) { .x { background:url(asset.png); } } }",
        );
        assert_eq!(values(&result), ["theme.css", "theme.css", "asset.png"]);
        assert_eq!(
            result.dependencies[0].category_hint,
            CssDependencyCategory::Stylesheet
        );
        assert_eq!(result.dependencies[2].at_rule_path, ["media", "supports"]);
        assert_eq!(
            result.dependencies[2].property.as_deref(),
            Some("background")
        );
        assert_eq!(
            result.dependencies[2].resolved_url.as_deref(),
            Some("https://example.test/styles/asset.png")
        );
    }

    #[test]
    fn image_set_options_capture_resolution_mime_without_type_string_dependency() {
        let result = inventory(
            ".x { background:image-set(\"small.png\" 1x, url(\"large.png\") 2x type(\"image/png\"), linear-gradient(red,blue) 3x); }",
        );
        assert_eq!(values(&result), ["small.png", "large.png"]);
        let candidate = result.dependencies[1].image_set.as_ref().unwrap();
        assert_eq!(candidate.declared_mime.as_deref(), Some("image/png"));
        assert_eq!(candidate.resolution.as_ref().unwrap().value, 2.0);
        assert_eq!(candidate.resolution.as_ref().unwrap().unit, "x");
        assert!(result.complete);
    }

    #[test]
    fn image_set_direct_and_indirect_nesting_never_produces_eligible_targets() {
        let result = inventory(
            ".x { background:image-set(cross-fade(image-set(\"inner.png\" 1x),url(outer.png)) 1x,\"control.png\" 2x); mask:image-set(image-set(\"direct.png\" 1x) 1x); }",
        );
        assert_eq!(values(&result), ["outer.png", "control.png"]);
        assert_eq!(result.dependencies[0].resolved_url, None);
        assert_eq!(
            result.dependencies[0].resolution_error.as_deref(),
            Some("css_image_set_option_invalid")
        );
        assert_eq!(
            result.dependencies[1].resolved_url.as_deref(),
            Some("https://example.test/styles/control.png")
        );
        assert!(!result.complete);
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|item| item.code == "css_image_set_nested_unsupported")
                .count(),
            2
        );
    }

    #[test]
    fn image_url_string_orientation_and_negative_ordinary_strings() {
        let result = inventory(
            "@namespace p url(https://example.test/ns); .x[data-x=\"a.png\"] { content:\"b.png\"; font-family:\"c.png\"; background:image(ltr \"actual.png\", red); } @font-face { src:local(\"d.png\"),url(font.woff2); }",
        );
        assert_eq!(values(&result), ["actual.png", "font.woff2"]);
        assert_eq!(
            result.dependencies[1].category_hint,
            CssDependencyCategory::Font
        );
        assert_eq!(result.dependencies[1].at_rule.as_deref(), Some("font-face"));
    }

    #[test]
    fn declaration_list_is_explicit_and_custom_substitution_is_deferred() {
        let mut supplied = input(
            "background:url(a.png); --Case: url(b.png); mask:var(--known,url(fallback.svg));",
        );
        supplied.syntax = CssSourceSyntax::DeclarationList;
        supplied.inline = true;
        supplied.base_url = "https://example.test/container/".into();
        let result = discover_css(&supplied, &CssInventoryLimits::default()).unwrap();
        assert_eq!(values(&result), ["a.png", "b.png", "fallback.svg"]);
        assert_eq!(result.dependencies[1].property.as_deref(), Some("--Case"));
        assert!(!result.dependencies[0].deferred);
        assert!(result.dependencies[1].deferred && result.dependencies[2].deferred);
        assert_eq!(
            result.dependencies[0].resolved_url.as_deref(),
            Some("https://example.test/container/a.png")
        );
    }

    #[test]
    fn nested_selector_does_not_leak_selector_strings_or_context() {
        let result = inventory(
            ".root { a:hover { background:url(nested.png); } &:has([src=\"selector.png\"]) { mask:url(mask.svg); } }",
        );
        assert_eq!(values(&result), ["nested.png", "mask.svg"]);
        assert_eq!(
            result.dependencies[0].property.as_deref(),
            Some("background")
        );
        assert_eq!(result.dependencies[1].property.as_deref(), Some("mask"));
    }

    #[test]
    fn non_http_local_empty_invalid_and_credentials_remain_observations() {
        let result = inventory(
            ".x { mask:url(#clip); background:url(data:image/png;base64,AA); cursor:url(blob:https://example.test/id),auto; border-image:url(); list-style:url(\"http://user:password@example.test/a\"); }",
        );
        assert_eq!(result.dependencies.len(), 5);
        assert!(
            result
                .dependencies
                .iter()
                .all(|item| item.resolved_url.is_none())
        );
        assert_eq!(
            result.dependencies[0].resolution_error.as_deref(),
            Some("local_fragment_reference")
        );
        assert_eq!(
            result.dependencies[1].resolution_error.as_deref(),
            Some("non_http_reference")
        );
        assert_eq!(
            result.dependencies[3].resolution_error.as_deref(),
            Some("empty_reference")
        );
        assert_eq!(
            result.dependencies[4].resolution_error.as_deref(),
            Some("url_credentials_not_supported")
        );
    }

    #[test]
    fn bad_token_and_eof_recovery_never_fabricate_targets() {
        let result = inventory(".x { background:url(bad path.png); } .y { mask:url(ok.svg); }");
        assert_eq!(values(&result), ["ok.svg"]);
        assert!(!result.complete);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "css_bad_token")
        );
        let result = inventory(".x { background:url(\"unterminated");
        assert!(result.dependencies.is_empty());
        assert!(!result.complete);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "css_eof_string_recovery")
        );
        let result = inventory(".x { background:url(unclosed");
        assert!(result.dependencies.is_empty());
        assert!(!result.complete);
    }

    #[test]
    fn unsupported_context_is_explicit_and_invalid_image_set_has_no_target() {
        let result = inventory(
            "@future { .x { background:url(hidden.png); } } .x { background:src(\"modern.png\"); mask:image-set(\"invalid.png\" 0x, \"ok.png\" 2x); }",
        );
        assert_eq!(values(&result), ["invalid.png", "ok.png"]);
        assert!(!result.complete);
        assert_eq!(result.dependencies[0].resolved_url, None);
        assert_eq!(
            result.dependencies[0].resolution_error.as_deref(),
            Some("css_image_set_option_invalid")
        );
        assert!(result.dependencies[1].resolved_url.is_some());
    }

    #[test]
    fn global_token_depth_candidate_diagnostic_and_output_limits_are_atomic() {
        let text = ".x{background:url(\"a.png\");mask:url(\"b.svg\")}";
        let result = inventory(text);
        let exact = CssInventoryLimits {
            max_tokens: result.visited_tokens,
            max_candidates: 2,
            max_nesting: result.max_observed_nesting,
            ..Default::default()
        };
        assert!(discover_css(&input(text), &exact).is_ok());
        for limits in [
            CssInventoryLimits {
                max_tokens: result.visited_tokens - 1,
                ..Default::default()
            },
            CssInventoryLimits {
                max_candidates: 1,
                ..Default::default()
            },
            CssInventoryLimits {
                max_utf8_bytes: text.len() - 1,
                ..Default::default()
            },
            CssInventoryLimits {
                max_nesting: 1,
                ..Default::default()
            },
            CssInventoryLimits {
                max_retained_utf8_bytes: 100,
                ..Default::default()
            },
            CssInventoryLimits {
                max_url_utf8_bytes: 4,
                ..Default::default()
            },
        ] {
            assert!(discover_css(&input(text), &limits).is_err());
        }
        let bad = input(".x { a:url(bad path); b:url(other bad); }");
        assert!(
            discover_css(
                &bad,
                &CssInventoryLimits {
                    max_diagnostics: 1,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn strict_shapes_source_and_limit_validation_precede_parsing() {
        let mut value = serde_json::to_value(input("")).unwrap();
        value["unknown"] = true.into();
        assert!(serde_json::from_value::<CssSourceInput>(value).is_err());
        let mut value = serde_json::to_value(CssInventoryLimits::default()).unwrap();
        value["unknown"] = true.into();
        assert!(serde_json::from_value::<CssInventoryLimits>(value).is_err());
        assert!(
            discover_css(
                &input(""),
                &CssInventoryLimits {
                    max_tokens: 0,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            discover_css(
                &input(""),
                &CssInventoryLimits {
                    max_nesting: usize::MAX,
                    ..Default::default()
                }
            )
            .is_err()
        );
        let mut source = input("");
        source.base_url = "file:///tmp/source.css".into();
        assert!(discover_css(&source, &CssInventoryLimits::default()).is_err());
        source = input("");
        source.source_revision.clear();
        assert!(discover_css(&source, &CssInventoryLimits::default()).is_err());
    }

    #[test]
    fn source_hash_revisions_and_inventory_roundtrip_are_reproducible() {
        let result = inventory("");
        assert!(result.complete && result.dependencies.is_empty());
        assert_eq!(
            result.source_utf8_sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(result.visited_tokens, 0);
        let first = inventory(".x{background:url(é.png)}");
        let second = inventory(".x{background:url(é.png)} ");
        assert_ne!(first.source_utf8_sha256, second.source_utf8_sha256);
        let json = serde_json::to_value(&first).unwrap();
        assert_eq!(serde_json::from_value::<CssInventory>(json).unwrap(), first);
    }

    #[test]
    fn authored_fixture_controls_preserve_order_and_coverage() {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fixture {
            id: String,
            css_text: String,
            syntax: CssSourceSyntax,
            expected_urls: Vec<String>,
            expected_complete: bool,
        }
        let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
            "../tests/fixtures/css-discovery/declared-dependencies.json"
        ))
        .unwrap();
        assert_eq!(fixtures.len(), 15);
        for fixture in fixtures {
            let mut supplied = input(&fixture.css_text);
            supplied.syntax = fixture.syntax;
            let result = discover_css(&supplied, &CssInventoryLimits::default()).unwrap();
            assert_eq!(values(&result), fixture.expected_urls, "{}", fixture.id);
            assert_eq!(result.complete, fixture.expected_complete, "{}", fixture.id);
            assert_eq!(result.website_requests, 0);
            for dependency in &result.dependencies {
                assert_eq!(
                    &fixture.css_text[dependency.byte_start..dependency.byte_end],
                    dependency.raw_token
                );
            }
        }
        let local = inventory(".x { mask:url(\"  #clip\"); background:url(\"  \" ); }");
        assert!(
            local
                .dependencies
                .iter()
                .all(|item| item.resolved_url.is_none())
        );
        let query = inventory(".x { background:url(asset.png?v=1+2&x=%2F#clip); }");
        assert_eq!(
            query.dependencies[0].resolved_url.as_deref(),
            Some("https://example.test/styles/asset.png?v=1+2&x=%2F")
        );
    }

    #[test]
    fn eof_comment_and_escaped_closing_url_do_not_claim_complete_source() {
        let result = inventory("/* unfinished url(not-a-target.png)");
        assert!(!result.complete && result.dependencies.is_empty());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "css_eof_comment_recovery")
        );
        let result = inventory(".x{background:url(escaped\\)");
        assert!(!result.complete && result.dependencies.is_empty());
        let result = inventory(".x{background:url(escaped\\))}");
        assert_eq!(values(&result), ["escaped)"]);
        assert!(result.complete);
    }
}
