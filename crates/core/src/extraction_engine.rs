//! Original extraction adapters over HTML5 parsing, Rust regex and sxd XPath.
//! XPath operates on the repaired HTML DOM, not an XML parse of HTML source.
use crate::{CustomExtraction, CustomExtractionConfig, ExtractionMethod};
use anyhow::{Context as _, Result, bail};
use regex::{Regex, RegexBuilder};
use scraper::{Html, Selector, node::Node};
use std::collections::{HashMap, HashSet};
use sxd_document::{Package, QName};
use sxd_xpath::{Context, Factory, Value};

const HTML_NS: &str = "http://www.w3.org/1999/xhtml";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
const XMLNS_NS: &str = "http://www.w3.org/2000/xmlns/";
const MAX_DOM_NODES: usize = 100_000;
const MAX_DOM_DEPTH: usize = 256;

fn compile_regex(config: &CustomExtractionConfig) -> Result<Regex> {
    if config.regex_flags.len() > 5 {
        bail!("Regex flags must contain at most five distinct flags");
    }
    let mut flags = HashSet::new();
    let mut builder = RegexBuilder::new(&config.selector);
    builder.size_limit(1024 * 1024).dfa_size_limit(1024 * 1024);
    for flag in config.regex_flags.chars() {
        if !flags.insert(flag) {
            bail!("Duplicate regex flag: {flag}");
        }
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
            _ => bail!("Unsupported Rust regex flag {flag:?}; supported flags: i, m, s, x, U"),
        }
    }
    let regex = builder
        .build()
        .context("Invalid Rust regex (lookaround and backreferences are unsupported)")?;
    if config
        .regex_group
        .is_some_and(|group| group >= regex.captures_len())
    {
        bail!("Regex capture group does not exist");
    }
    Ok(regex)
}

fn prefix_valid(prefix: &str) -> bool {
    let mut chars = prefix.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// Inspect names outside literals so unsupported functions, variables and
/// unbound prefixes fail even inside predicates on an empty node set.
fn validate_xpath_names(config: &CustomExtractionConfig) -> Result<()> {
    let chars: Vec<_> = config.selector.chars().collect();
    let mut offset = 0;
    let mut nesting = 0usize;
    let mut operators = 0usize;
    while offset < chars.len() {
        let c = chars[offset];
        if matches!(c, '\'' | '"') {
            offset += 1;
            while offset < chars.len() && chars[offset] != c {
                offset += 1;
            }
            offset += 1;
            continue;
        }
        if c == '$' {
            bail!("XPath variables are unsupported");
        }
        if matches!(c, '(' | '[') {
            nesting += 1;
            if nesting > 64 {
                bail!("XPath nesting exceeds 64 levels");
            }
        } else if matches!(c, ')' | ']') {
            nesting = nesting.saturating_sub(1);
        }
        if matches!(c, '/' | '[' | '(' | '+' | '-' | '|') {
            operators += 1;
            if operators > 1024 {
                bail!("XPath expression exceeds the 1024 operation syntax budget");
            }
        }
        if c.is_alphabetic() || c == '_' {
            let start = offset;
            offset += 1;
            while offset < chars.len()
                && (chars[offset].is_alphanumeric() || matches!(chars[offset], '_' | '-' | '.'))
            {
                offset += 1;
            }
            let name: String = chars[start..offset].iter().collect();
            let mut namespaced = false;
            if chars.get(offset) == Some(&':') && chars.get(offset + 1) != Some(&':') {
                if name != "xml" && !config.namespaces.contains_key(&name) {
                    bail!("Unbound XPath namespace prefix: {name}");
                }
                namespaced = true;
                offset += 1;
                while offset < chars.len()
                    && (chars[offset].is_alphanumeric()
                        || matches!(chars[offset], '_' | '-' | '.' | '*'))
                {
                    offset += 1;
                }
            }
            let mut next = offset;
            while chars.get(next).is_some_and(|c| c.is_whitespace()) {
                next += 1;
            }
            if chars.get(next) == Some(&'(') {
                const FUNCTIONS: &[&str] = &[
                    "last",
                    "position",
                    "count",
                    "local-name",
                    "namespace-uri",
                    "name",
                    "string",
                    "concat",
                    "starts-with",
                    "contains",
                    "substring-before",
                    "substring-after",
                    "substring",
                    "string-length",
                    "normalize-space",
                    "translate",
                    "boolean",
                    "not",
                    "true",
                    "false",
                    "number",
                    "sum",
                    "floor",
                    "ceiling",
                    "round",
                    "node",
                    "text",
                    "comment",
                    "processing-instruction",
                ];
                if namespaced || !FUNCTIONS.contains(&name.as_str()) {
                    bail!(
                        "Unsupported XPath function: {name}; this engine supports XPath 1.0 core functions except id() and lang(), without extension functions"
                    );
                }
            }
            continue;
        }
        offset += 1;
    }
    Ok(())
}

pub(crate) fn validate_definition(config: &CustomExtractionConfig) -> Result<()> {
    if config.selector.trim().is_empty() || config.selector.len() > 8192 {
        bail!("Extraction expression must contain 1 to 8192 bytes");
    }
    if !(1..=100_000).contains(&config.max_results) {
        bail!("Extraction max_results must be between 1 and 100000");
    }
    if config.method != ExtractionMethod::Css && config.attribute.is_some() {
        bail!(
            "attribute is supported only for CSS; select XPath attributes with /@name or use a regex capture group"
        );
    }
    if config.method != ExtractionMethod::Regex
        && (config.regex_group.is_some() || !config.regex_flags.is_empty())
    {
        bail!("regex_group and regex_flags require method regex");
    }
    if config.method != ExtractionMethod::XPath && !config.namespaces.is_empty() {
        bail!("namespaces require method xpath");
    }
    match config.method {
        ExtractionMethod::Css => {
            Selector::parse(&config.selector)
                .map_err(|e| anyhow::anyhow!("Invalid CSS selector: {e:?}"))?;
            if let Some(attribute) = &config.attribute
                && (attribute.is_empty()
                    || attribute.len() > 128
                    || attribute.chars().any(|c| {
                        c.is_whitespace()
                            || c.is_control()
                            || matches!(c, '<' | '>' | '=' | '\'' | '"')
                    }))
            {
                bail!("Invalid CSS attribute name");
            }
        }
        ExtractionMethod::Regex => {
            compile_regex(config)?;
        }
        ExtractionMethod::XPath => {
            if config.namespaces.len() > 32 {
                bail!("At most 32 XPath namespace bindings are supported");
            }
            for (prefix, uri) in &config.namespaces {
                if !prefix_valid(prefix)
                    || prefix.len() > 128
                    || matches!(prefix.as_str(), "xml" | "xmlns")
                    || uri.trim().is_empty()
                    || uri.len() > 2048
                    || uri.chars().any(char::is_control)
                {
                    bail!("Invalid or reserved XPath namespace binding: {prefix}");
                }
            }
            validate_xpath_names(config)?;
            let xpath = Factory::new()
                .build(&config.selector)
                .context("Invalid XPath 1.0 expression")?
                .ok_or_else(|| anyhow::anyhow!("Empty XPath expression"))?;
            // Evaluating a small real tree additionally rejects wrong core function
            // arity/types in expressions that do not depend on page observations.
            let package = Package::new();
            let document = package.as_document();
            document
                .root()
                .append_child(document.create_element("html"));
            xpath
                .evaluate(&xpath_context(config), document.root())
                .context("Invalid XPath evaluation")?;
        }
    }
    Ok(())
}

pub(crate) fn validate_custom_extractions(config: &[CustomExtractionConfig]) -> Result<()> {
    if config.len() > 128 {
        bail!("At most 128 custom extraction definitions are supported per crawl");
    }
    let mut names = HashSet::new();
    for definition in config {
        if definition.name.trim().is_empty()
            || definition.name.chars().count() > 128
            || definition.name.chars().any(char::is_control)
        {
            bail!("Custom extraction name must contain 1 to 128 characters without controls");
        }
        if !names.insert(definition.name.trim().to_ascii_lowercase()) {
            bail!("Duplicate custom extraction name: {}", definition.name);
        }
        validate_definition(definition).map_err(|error| {
            anyhow::anyhow!("Invalid custom extraction {}: {error:#}", definition.name)
        })?;
    }
    Ok(())
}

fn xpath_context<'d>(config: &CustomExtractionConfig) -> Context<'d> {
    let mut context = Context::new();
    context.set_namespace("xml", XML_NS);
    for (prefix, uri) in &config.namespaces {
        context.set_namespace(prefix, uri);
    }
    context
}

/// Iterative conversion avoids XML parse errors and preserves repaired HTML,
/// text nodes, comments, foreign namespaces and attribute namespaces.
fn html_xpath_package(html: &Html) -> Result<Package> {
    let package = Package::new();
    let document = package.as_document();
    let mut elements: HashMap<_, sxd_document::dom::Element<'_>> = HashMap::new();
    let mut depths = HashMap::new();
    for (count, node) in html.tree.root().descendants().enumerate() {
        if count >= MAX_DOM_NODES {
            bail!("XPath DOM exceeds {MAX_DOM_NODES} nodes; no partial result was returned");
        }
        let parent_id = node.parent().map(|parent| parent.id());
        let depth = parent_id
            .and_then(|id| depths.get(&id))
            .copied()
            .unwrap_or(0usize)
            + 1;
        if depth > MAX_DOM_DEPTH {
            bail!("XPath DOM exceeds {MAX_DOM_DEPTH} levels; no partial result was returned");
        }
        depths.insert(node.id(), depth);
        let parent = parent_id.and_then(|id| elements.get(&id)).copied();
        match node.value() {
            Node::Element(html_element) => {
                let ns = html_element.name.ns.as_ref();
                let namespace = (!ns.is_empty() && ns != HTML_NS).then_some(ns);
                let element = document.create_element(QName::with_namespace_uri(
                    namespace,
                    html_element.name.local.as_ref(),
                ));
                if let Some(namespace) = namespace {
                    element.set_default_namespace_uri(Some(namespace));
                    // sxd's namespace axis enumerates registered prefixes only;
                    // an empty prefix represents the DOM's default namespace.
                    element.register_prefix("", namespace);
                }
                element.set_preferred_prefix(
                    html_element
                        .name
                        .prefix
                        .as_ref()
                        .map(|prefix| prefix.as_ref()),
                );
                for (name, value) in &html_element.attrs {
                    let ns = name.ns.as_ref();
                    if ns == XMLNS_NS || (ns.is_empty() && name.local.as_ref() == "xmlns") {
                        if name.local.as_ref() != "xmlns" {
                            element.register_prefix(name.local.as_ref(), value);
                        }
                        continue;
                    }
                    let attribute = element.set_attribute_value(
                        QName::with_namespace_uri(
                            (!ns.is_empty()).then_some(ns),
                            name.local.as_ref(),
                        ),
                        value,
                    );
                    attribute
                        .set_preferred_prefix(name.prefix.as_ref().map(|prefix| prefix.as_ref()));
                    if let Some(prefix) = &name.prefix {
                        element.register_prefix(prefix.as_ref(), ns);
                    }
                }
                match parent {
                    Some(parent) => parent.append_child(element),
                    None => document.root().append_child(element),
                }
                elements.insert(node.id(), element);
            }
            Node::Text(text) => {
                if let Some(parent) = parent {
                    parent.append_child(document.create_text(text));
                }
            }
            Node::Comment(comment) => {
                let comment = document.create_comment(comment);
                match parent {
                    Some(parent) => parent.append_child(comment),
                    None => document.root().append_child(comment),
                }
            }
            Node::ProcessingInstruction(pi) => {
                let instruction =
                    document.create_processing_instruction(&pi.target, Some(&pi.data));
                match parent {
                    Some(parent) => parent.append_child(instruction),
                    None => document.root().append_child(instruction),
                }
            }
            _ => {} // Document, fragment and doctype are not XPath element nodes.
        }
    }
    Ok(package)
}

fn clean(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn base_result(config: &CustomExtractionConfig, source: &str) -> CustomExtraction {
    CustomExtraction {
        name: config.name.clone(),
        selector: config.selector.clone(),
        attribute: config.attribute.clone(),
        method: config.method,
        source: source.into(),
        engine: match config.method {
            ExtractionMethod::Css => "scraper-html5-css",
            ExtractionMethod::Regex => "rust-regex",
            ExtractionMethod::XPath => "sxd-xpath-1.0-html-dom",
        }
        .into(),
        ..Default::default()
    }
}

fn append_value(result: &mut CustomExtraction, value: String, limit: usize) {
    let count = result.matched_count.get_or_insert(0);
    *count += 1;
    if result.values.len() < limit {
        result.values.push(value);
    } else {
        result.truncated = true;
    }
}

fn evaluate(
    html: &str,
    doc: Option<&Html>,
    package: Option<&Package>,
    config: &CustomExtractionConfig,
    source: &str,
) -> Result<CustomExtraction> {
    let mut result = base_result(config, source);
    result.matched_count = Some(0);
    match config.method {
        ExtractionMethod::Css => {
            let doc = doc.ok_or_else(|| anyhow::anyhow!("HTML DOM missing"))?;
            let selector = Selector::parse(&config.selector)
                .map_err(|e| anyhow::anyhow!("Invalid CSS selector: {e:?}"))?;
            result.result_type = if config.attribute.is_some() {
                "attribute"
            } else {
                "text"
            }
            .into();
            for node in doc.select(&selector) {
                let value = match &config.attribute {
                    Some(attribute) => node.value().attr(attribute).map(str::to_owned),
                    None => Some(clean(&node.text().collect::<String>())),
                };
                if let Some(value) = value {
                    append_value(&mut result, value, config.max_results);
                }
            }
        }
        ExtractionMethod::Regex => {
            let regex = compile_regex(config)?;
            result.result_type = "regex_match".into();
            let group = config.regex_group.unwrap_or(0);
            for capture in regex.captures_iter(html) {
                // An unmatched optional group is absent, not an invented empty string.
                if let Some(value) = capture.get(group) {
                    append_value(&mut result, value.as_str().into(), config.max_results);
                }
            }
        }
        ExtractionMethod::XPath => {
            let package = package.ok_or_else(|| anyhow::anyhow!("XPath DOM missing"))?;
            let xpath = Factory::new()
                .build(&config.selector)
                .context("Invalid XPath 1.0 expression")?
                .ok_or_else(|| anyhow::anyhow!("Empty XPath expression"))?;
            let value = xpath
                .evaluate(&xpath_context(config), package.as_document().root())
                .context("XPath evaluation failed")?;
            if let Value::Nodeset(nodes) = value {
                result.result_type = "nodeset".into();
                result.matched_count = Some(nodes.size());
                result.truncated = nodes.size() > config.max_results;
                result.values = nodes
                    .document_order()
                    .into_iter()
                    .take(config.max_results)
                    .map(|node| node.string_value())
                    .collect();
            } else {
                result.result_type = match &value {
                    Value::String(_) => "string",
                    Value::Number(_) => "number",
                    Value::Boolean(_) => "boolean",
                    Value::Nodeset(_) => unreachable!(),
                }
                .into();
                append_value(&mut result, value.string(), config.max_results);
            }
        }
    }
    Ok(result)
}

/// Evaluate one explicit definition. Regex examines the selected source HTML;
/// CSS/XPath examine the HTML5-repaired DOM. The source is retained as evidence.
pub fn extract_configured(
    html: &str,
    config: &CustomExtractionConfig,
    source: &str,
) -> Result<CustomExtraction> {
    validate_definition(config)?;
    let doc = (config.method != ExtractionMethod::Regex).then(|| Html::parse_document(html));
    let package = if config.method == ExtractionMethod::XPath {
        Some(html_xpath_package(
            doc.as_ref().expect("parsed XPath HTML"),
        )?)
    } else {
        None
    };
    evaluate(html, doc.as_ref(), package.as_ref(), config, source)
}

pub(crate) fn extract_all(
    html: &str,
    config: &[CustomExtractionConfig],
    source: &str,
) -> Vec<CustomExtraction> {
    let doc = config
        .iter()
        .any(|item| item.method != ExtractionMethod::Regex)
        .then(|| Html::parse_document(html));
    let package = if config
        .iter()
        .any(|item| item.method == ExtractionMethod::XPath)
    {
        Some(html_xpath_package(doc.as_ref().expect("parsed XPath HTML")))
    } else {
        None
    };
    config
        .iter()
        .map(|definition| {
            match validate_definition(definition).and_then(|()| {
                if definition.method == ExtractionMethod::XPath
                    && let Some(Err(error)) = &package
                {
                    bail!("{error:#}");
                }
                evaluate(
                    html,
                    doc.as_ref(),
                    package.as_ref().and_then(|value| value.as_ref().ok()),
                    definition,
                    source,
                )
            }) {
                Ok(result) => result,
                Err(error) => CustomExtraction {
                    error: Some(format!("{error:#}")),
                    ..base_result(definition, source)
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition(method: ExtractionMethod, selector: &str) -> CustomExtractionConfig {
        CustomExtractionConfig {
            name: "Test".into(),
            selector: selector.into(),
            method,
            ..Default::default()
        }
    }

    #[test]
    fn xpath_repairs_html_and_preserves_attributes_text_and_document_order() {
        let html = "<ul><li data-id='α'>First &amp; <b>bold<li data-id='β'>Second</ul><!--tail-->";
        for (expression, expected) in [
            ("//li/@data-id", vec!["α", "β"]),
            // HTML5 reconstructs the unclosed formatting element in the
            // second LI; its text is a descendant, not a direct LI child.
            ("//li/text()", vec!["First & "]),
            ("//li//text()", vec!["First & ", "bold", "Second"]),
            ("//li", vec!["First & bold", "Second"]),
            ("html/body/ul/li[2]/@data-id", vec!["β"]),
            ("//li[1]/following-sibling::li/@data-id", vec!["β"]),
            ("//comment()", vec!["tail"]),
            ("//missing", vec![]),
        ] {
            let result = extract_configured(
                html,
                &definition(ExtractionMethod::XPath, expression),
                "raw_html",
            )
            .unwrap();
            assert_eq!(result.values, expected, "{expression}");
            assert_eq!(result.matched_count, Some(expected.len()));
            assert_eq!(result.result_type, "nodeset");
        }
    }

    #[test]
    fn xpath_scalars_unicode_and_namespace_bindings_are_explicit() {
        let html = "<p> Grüße &amp; 世界 </p><svg><a xlink:href='/vector'>V</a></svg>";
        for (expression, expected, kind) in [
            ("count(//p)", "1", "number"),
            ("normalize-space(string(//p))", "Grüße & 世界", "string"),
            ("boolean(//missing)", "false", "boolean"),
            ("contains(string(//p), '世界')", "true", "boolean"),
        ] {
            let result = extract_configured(
                html,
                &definition(ExtractionMethod::XPath, expression),
                "rendered_html",
            )
            .unwrap();
            assert_eq!(result.values, vec![expected]);
            assert_eq!(result.result_type, kind);
            assert_eq!(result.source, "rendered_html");
        }
        let mut config = definition(ExtractionMethod::XPath, "//svg:a/@xl:href");
        config
            .namespaces
            .insert("svg".into(), "http://www.w3.org/2000/svg".into());
        config
            .namespaces
            .insert("xl".into(), "http://www.w3.org/1999/xlink".into());
        assert_eq!(
            extract_configured(html, &config, "raw_html")
                .unwrap()
                .values,
            vec!["/vector"]
        );
        config.selector = "//svg:svg/namespace::*[name() = '']".into();
        assert_eq!(
            extract_configured(html, &config, "raw_html")
                .unwrap()
                .values,
            vec!["http://www.w3.org/2000/svg"]
        );
    }

    #[test]
    fn regex_groups_flags_unicode_and_optional_groups() {
        let mut config = definition(ExtractionMethod::Regex, "<SKU>([^<]+)</SKU>");
        config.regex_flags = "i".into();
        config.regex_group = Some(1);
        assert_eq!(
            extract_configured("<sku>Grüße</sku><SKU>世界</SKU>", &config, "raw_html")
                .unwrap()
                .values,
            vec!["Grüße", "世界"]
        );
        config.selector = "(x)?y".into();
        let result = extract_configured("y xy", &config, "raw_html").unwrap();
        assert_eq!(result.values, vec!["x"]);
        assert_eq!(result.matched_count, Some(1));
        config.regex_group = None;
        config.selector = "(?m)^".into();
        assert_eq!(
            extract_configured("a\nb", &config, "raw_html")
                .unwrap()
                .values,
            vec!["", ""]
        );
    }

    #[test]
    fn every_method_records_full_count_and_explicit_result_truncation() {
        for (method, expression) in [
            (ExtractionMethod::Css, "li"),
            (ExtractionMethod::XPath, "//li"),
            (ExtractionMethod::Regex, "[abc]"),
        ] {
            let mut config = definition(method, expression);
            config.max_results = 2;
            let result = extract_configured("<li>a<li>b<li>c", &config, "raw_html").unwrap();
            assert_eq!(result.values, vec!["a", "b"]);
            assert_eq!(result.matched_count, Some(3));
            assert!(result.truncated);
        }
    }

    #[test]
    fn invalid_and_unsupported_expressions_fail_even_on_empty_selections() {
        for expression in [
            "//[",
            "//missing[unknown() = 1]",
            "//missing[id('x')]",
            "//missing[lang('en')]",
            "//missing[$variable]",
            "//unbound:missing",
            "matches('x', 'x')",
        ] {
            let error = extract_configured(
                "",
                &definition(ExtractionMethod::XPath, expression),
                "raw_html",
            )
            .unwrap_err();
            assert!(!error.to_string().is_empty(), "{expression}");
        }
        for expression in ["[", "(?<=x)y", r"(x)\1"] {
            assert!(
                extract_configured(
                    "",
                    &definition(ExtractionMethod::Regex, expression),
                    "raw_html"
                )
                .is_err()
            );
        }
        let mut config = definition(ExtractionMethod::Regex, "x");
        config.regex_group = Some(1);
        assert!(validate_definition(&config).is_err());
        config.regex_group = None;
        config.regex_flags = "g".into();
        assert!(validate_definition(&config).is_err());
        config.regex_flags.clear();
        config.max_results = 0;
        assert!(validate_definition(&config).is_err());
        let legacy: CustomExtractionConfig =
            serde_json::from_str(r#"{"name":"Old","selector":"p","attribute":null}"#).unwrap();
        assert_eq!(legacy.method, ExtractionMethod::Css);
        assert_eq!(legacy.max_results, 10000);
        let old_result: CustomExtraction = serde_json::from_str(r#"{"values":[]}"#).unwrap();
        assert_eq!(old_result.matched_count, None);
    }

    #[test]
    fn xpath_document_budget_errors_are_observable_not_empty_matches() {
        let html = format!("{}x{}", "<div>".repeat(300), "</div>".repeat(300));
        let results = extract_all(
            &html,
            &[definition(ExtractionMethod::XPath, "//div")],
            "raw_html",
        );
        assert!(results[0].error.as_ref().unwrap().contains("levels"));
        assert_eq!(results[0].matched_count, None);
        assert!(results[0].values.is_empty());
    }
}
