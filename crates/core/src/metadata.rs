//! Original parsers for HTTP Web Linking and scoped indexing directives.
use crate::{HreflangRecord, LinkRelationship, PageRecord, RobotsDirective, normalize_url};
use std::collections::BTreeMap;
use url::Url;

// Delimiters inside URI references or escaped quoted strings are data.
fn split_quoted(value: &str, delimiter: char) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    let mut angle = false;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quoted && character == '\\' {
            escaped = true;
        } else if character == '"' && !angle {
            quoted = !quoted;
        } else if !quoted && character == '<' {
            angle = true;
        } else if !quoted && character == '>' {
            angle = false;
        } else if character == delimiter && !quoted && !angle {
            result.push(value[start..index].trim());
            start = index + character.len_utf8();
        }
    }
    result.push(value[start..].trim());
    result
}

fn parameter_value(value: &str) -> Option<String> {
    let value = value.trim();
    if !value.starts_with('"') {
        return (!value.is_empty() && !value.chars().any(char::is_whitespace))
            .then(|| value.to_owned());
    }
    let mut result = String::new();
    let mut escaped = false;
    let mut chars = value[1..].chars();
    while let Some(character) = chars.next() {
        if escaped {
            result.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return chars.as_str().trim().is_empty().then_some(result);
        } else {
            result.push(character);
        }
    }
    None
}

/// Parse RFC8288 Link field facts. `anchor` contexts are preserved and never
/// silently attributed to the response document. HTTP targets ignore HTML base.
pub fn parse_link_header(value: &str, response_url: &Url) -> Vec<LinkRelationship> {
    split_quoted(value, ',')
        .into_iter()
        .filter_map(|value| {
            let remainder = value.strip_prefix('<')?;
            let close = remainder.find('>')?;
            let target = &remainder[..close];
            if target.chars().any(char::is_whitespace) {
                return None;
            }
            let url = normalize_url(target, Some(response_url))?;
            let remainder = remainder[close + 1..].trim();
            if !remainder.is_empty() && !remainder.starts_with(';') {
                return None;
            }
            let mut parameters: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for parameter in split_quoted(remainder.trim_start_matches(';'), ';') {
                if parameter.is_empty() {
                    continue;
                }
                let (name, value) = parameter.split_once('=').unwrap_or((parameter, ""));
                let name = name.trim().to_ascii_lowercase();
                if name.is_empty() || name.chars().any(char::is_whitespace) {
                    return None;
                }
                let value = if value.is_empty() {
                    String::new()
                } else {
                    parameter_value(value)?
                };
                parameters.entry(name).or_default().push(value);
            }
            let first = |name: &str| {
                parameters
                    .get(name)
                    .and_then(|values| values.first())
                    .cloned()
            };
            let rel: Vec<_> = first("rel")?
                .split_ascii_whitespace()
                .map(str::to_ascii_lowercase)
                .collect();
            if rel.is_empty() {
                return None;
            }
            let context = match first("anchor") {
                Some(anchor) => response_url.join(&anchor).ok()?,
                None => response_url.clone(),
            };
            let applies_to_document = context == *response_url;
            Some(LinkRelationship {
                url: url.to_string(),
                rel,
                source: "http_header".into(),
                context_url: context.to_string(),
                applies_to_document,
                hreflang: parameters.get("hreflang").cloned().unwrap_or_default(),
                media: first("media"),
                type_hint: first("type"),
                title: first("title"),
                parameters,
            })
        })
        .collect()
}

pub(crate) fn apply_relationship(page: &mut PageRecord, relationship: LinkRelationship) {
    if relationship.applies_to_document {
        let has = |rel: &str| relationship.rel.iter().any(|value| value == rel);
        if has("canonical") && relationship.source != "html_anchor" {
            let source = if relationship.source.ends_with("http_header") {
                &mut page.header_canonical
            } else {
                &mut page.html_canonical
            };
            source.push(relationship.url.clone());
            if !page.canonical.contains(&relationship.url) {
                page.canonical.push(relationship.url.clone());
            }
        }
        if has("alternate") && relationship.source != "html_anchor" {
            for language in &relationship.hreflang {
                let record = HreflangRecord {
                    language: language.clone(),
                    url: relationship.url.clone(),
                };
                if relationship.source.ends_with("http_header") {
                    page.header_hreflang.push(record.clone());
                } else {
                    page.html_hreflang.push(record.clone());
                }
                if !page.hreflang.iter().any(|existing| {
                    existing.url == record.url
                        && existing.language.eq_ignore_ascii_case(&record.language)
                }) {
                    page.hreflang.push(record);
                }
            }
        }
        for (relation, destination) in [
            ("next", &mut page.pagination_next),
            ("prev", &mut page.pagination_prev),
            ("amphtml", &mut page.amphtml),
        ] {
            if has(relation)
                && (relation != "amphtml" || relationship.source != "html_anchor")
                && !destination.contains(&relationship.url)
            {
                destination.push(relationship.url.clone());
            }
        }
    }
    page.link_relationships.push(relationship);
}

pub(crate) fn reset_html_relationships(page: &mut PageRecord) {
    page.html_canonical.clear();
    page.header_canonical.clear();
    page.canonical.clear();
    page.html_hreflang.clear();
    page.header_hreflang.clear();
    page.hreflang.clear();
    page.pagination_next.clear();
    page.pagination_prev.clear();
    page.amphtml.clear();
    let headers: Vec<_> = page
        .link_relationships
        .drain(..)
        .filter(|link| link.source.ends_with("http_header"))
        .collect();
    for mut relationship in headers {
        relationship.applies_to_document = relationship.context_url == page.final_url;
        apply_relationship(page, relationship);
    }
}

pub(crate) fn extract_http_metadata(page: &mut PageRecord) {
    let Some(url) = normalize_url(&page.final_url, None) else {
        return;
    };
    page.link_relationships.clear();
    reset_html_relationships(page);
    for value in page.header_link_values.clone() {
        let relationships = parse_link_header(&value, &url);
        if relationships.is_empty() && !value.trim().is_empty() {
            page.warnings.push(
                "Link response header contained no valid supported HTTP(S) relationships".into(),
            );
        }
        for relationship in relationships {
            apply_relationship(page, relationship);
        }
    }
    page.robots_directives.clear();
    for value in &page.x_robots_tag {
        page.robots_directives.extend(parse_x_robots_tag(value));
    }
    evaluate_robots(page);
}

fn directive_name(value: &str) -> &str {
    value.split_once(':').map_or(value, |(name, _)| name).trim()
}

fn recognized(name: &str) -> bool {
    matches!(
        name,
        "all"
            | "none"
            | "index"
            | "noindex"
            | "follow"
            | "nofollow"
            | "noarchive"
            | "nocache"
            | "nosnippet"
            | "noimageindex"
            | "notranslate"
            | "indexifembedded"
            | "max-snippet"
            | "max-image-preview"
            | "max-video-preview"
            | "unavailable_after"
            | "nositelinkssearchbox"
    )
}

fn normalize_directives(value: &str) -> Vec<String> {
    let mut result = Vec::new();
    for part in split_quoted(value, ',') {
        let part = part.trim().to_ascii_lowercase();
        if part.contains(':') && recognized(directive_name(&part)) {
            result.push(part);
        } else {
            result.extend(
                part.split_ascii_whitespace()
                    .filter(|token| recognized(token))
                    .map(str::to_owned),
            );
        }
    }
    result
}

/// Each header field starts unscoped. A bot prefix scopes subsequent comma
/// separated directives in that field, until another explicit bot prefix.
pub fn parse_x_robots_tag(value: &str) -> Vec<RobotsDirective> {
    let mut result: Vec<RobotsDirective> = Vec::new();
    let mut agent = None;
    for part in split_quoted(value, ',') {
        let mut content = part.trim();
        if let Some((prefix, remainder)) = content.split_once(':') {
            let prefix = prefix.trim().to_ascii_lowercase();
            if !recognized(&prefix)
                && !prefix.is_empty()
                && !prefix.chars().any(char::is_whitespace)
            {
                agent = Some(prefix);
                content = remainder.trim();
            }
        }
        let directives = normalize_directives(content);
        if directives.is_empty() {
            // Date continuations are retained in the previous valued directive.
            if let Some(previous) = result.last_mut()
                && previous
                    .directives
                    .last()
                    .is_some_and(|d| d.starts_with("unavailable_after:"))
            {
                previous.raw.push_str(", ");
                previous.raw.push_str(content);
                let directive = previous.directives.last_mut().unwrap();
                directive.push_str(", ");
                directive.push_str(&content.to_ascii_lowercase());
            }
            continue;
        }
        result.push(RobotsDirective {
            source: "http_header".into(),
            agent: agent.clone(),
            raw: content.into(),
            directives,
        });
    }
    result
}

pub(crate) fn add_meta_robots(page: &mut PageRecord, name: &str, content: &str) {
    let name = name.to_ascii_lowercase();
    let known_agent = matches!(
        name.as_str(),
        "googlebot"
            | "googlebot-news"
            | "bingbot"
            | "slurp"
            | "duckduckbot"
            | "baiduspider"
            | "yandexbot"
            | "adsbot-google"
    ) || name == page.indexability_agent;
    if name != "robots" && !known_agent {
        return;
    }
    page.meta_robots.push(content.into());
    page.robots_directives.push(RobotsDirective {
        source: "html_meta".into(),
        agent: (name != "robots").then_some(name),
        raw: content.into(),
        directives: normalize_directives(content),
    });
}

pub(crate) fn evaluate_robots(page: &mut PageRecord) {
    if page.indexability_agent.is_empty() {
        page.indexability_agent = "googlebot".into();
    }
    let mut effective = Vec::new();
    for rule in &page.robots_directives {
        if rule.agent.as_ref().is_none_or(|agent| {
            agent.eq_ignore_ascii_case(&page.indexability_agent) || agent == "*"
        }) {
            for directive in &rule.directives {
                if !effective.contains(directive) {
                    effective.push(directive.clone());
                }
            }
        }
    }
    // Restrictive directives prevail; preserve the originals above as evidence.
    if effective.iter().any(|value| value == "none") {
        for directive in ["noindex", "nofollow"] {
            if !effective.iter().any(|value| value == directive) {
                effective.push(directive.into());
            }
        }
    }
    if effective.iter().any(|value| value == "noindex") {
        effective.retain(|value| value != "index");
    }
    if effective.iter().any(|value| value == "nofollow") {
        effective.retain(|value| value != "follow");
    }
    page.effective_robots = effective;
}

/// Scoped effective directives, including reports written before scoped facts
/// existed. Historical meta values have no recoverable per-tag agent identity.
pub fn applicable_robot_directives(page: &PageRecord) -> Vec<String> {
    let mut facts = PageRecord {
        indexability_agent: page.indexability_agent.clone(),
        robots_directives: page.robots_directives.clone(),
        ..Default::default()
    };
    if facts.robots_directives.is_empty() {
        for value in &page.x_robots_tag {
            facts.robots_directives.extend(parse_x_robots_tag(value));
        }
        for value in &page.meta_robots {
            facts.robots_directives.push(RobotsDirective {
                source: "html_meta".into(),
                raw: value.clone(),
                directives: normalize_directives(value),
                ..Default::default()
            });
        }
    }
    evaluate_robots(&mut facts);
    facts.effective_robots
}

pub(crate) fn sync_relationship_links(
    page: &mut PageRecord,
    seeds: &[Url],
    include_subdomains: bool,
) {
    page.links.retain(|link| link.kind != "relationship");
    for relationship in &page.link_relationships {
        if !relationship.applies_to_document
            || relationship.source == "html_anchor"
            || !relationship.rel.iter().any(|rel| {
                matches!(
                    rel.as_str(),
                    "canonical" | "alternate" | "next" | "prev" | "amphtml"
                )
            })
        {
            continue;
        }
        if let Some(target) = normalize_url(&relationship.url, None) {
            page.links.push(crate::LinkRecord {
                url: relationship.url.clone(),
                rel: relationship.rel.join(" "),
                internal: crate::urls::same_scope(&target, seeds, include_subdomains),
                nofollow: relationship.rel.iter().any(|rel| rel == "nofollow"),
                kind: "relationship".into(),
                ..Default::default()
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_header_handles_quoted_delimiters_repeated_parameters_and_contexts() {
        let url = Url::parse("https://example.org/articles/page").unwrap();
        let links = parse_link_header(
            r##"<../target?items=a,b>; REL="CANONICAL next"; title="A, B; \"quoted\""; rel=prev, </de>; rel=alternate; hreflang=de; hreflang="en"; type="application/pdf", </else>; rel=canonical; anchor="#section""##,
            &url,
        );
        assert_eq!(links.len(), 3);
        assert_eq!(links[0].url, "https://example.org/target?items=a,b");
        assert_eq!(links[0].rel, vec!["canonical", "next"]);
        assert_eq!(links[0].title.as_deref(), Some("A, B; \"quoted\""));
        assert_eq!(links[0].parameters["rel"], vec!["CANONICAL next", "prev"]);
        assert_eq!(links[1].hreflang, vec!["de", "en"]);
        assert_eq!(links[1].type_hint.as_deref(), Some("application/pdf"));
        assert!(!links[2].applies_to_document);
        assert_eq!(
            links[2].context_url,
            "https://example.org/articles/page#section"
        );
        assert!(parse_link_header("</missing>; title=none, <mailto:a@b>; rel=canonical, </broken>; rel=\"never closed", &url).is_empty());
    }

    #[test]
    fn robots_scopes_values_dates_and_conflicting_restrictions_remain_distinct() {
        let mut page = PageRecord {
            indexability_agent: "googlebot".into(),
            ..Default::default()
        };
        page.robots_directives
            .extend(parse_x_robots_tag("BingBot: noindex, nofollow"));
        page.robots_directives.extend(parse_x_robots_tag(
            "index, follow, max-snippet:50, unavailable_after: Wed, 03 Dec 2099 13:00:00 GMT",
        ));
        evaluate_robots(&mut page);
        assert!(!page.effective_robots.iter().any(|value| value == "noindex"));
        assert!(
            page.effective_robots
                .iter()
                .any(|value| value == "max-snippet:50")
        );
        assert!(
            page.effective_robots
                .iter()
                .any(|value| value == "unavailable_after: wed, 03 dec 2099 13:00:00 gmt")
        );
        page.robots_directives
            .extend(parse_x_robots_tag("googlebot: none, bingbot: index"));
        evaluate_robots(&mut page);
        assert!(page.effective_robots.iter().any(|value| value == "noindex"));
        assert!(
            page.effective_robots
                .iter()
                .any(|value| value == "nofollow")
        );
        assert!(
            !page
                .effective_robots
                .iter()
                .any(|value| value == "index" || value == "follow")
        );
        assert!(
            page.robots_directives
                .iter()
                .any(|rule| rule.agent.as_deref() == Some("bingbot")
                    && rule.directives.contains(&"index".into()))
        );
    }

    #[test]
    fn historical_json_defaults_and_robot_header_scopes_remain_usable() {
        let page: PageRecord = serde_json::from_str(
            r#"{"url":"https://example.org/","x_robots_tag":["bingbot:noindex","follow"]}"#,
        )
        .unwrap();
        assert!(page.link_relationships.is_empty());
        assert_eq!(applicable_robot_directives(&page), vec!["follow"]);
        let config: crate::CrawlConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.indexability_agent, "googlebot");
        let raw: crate::RawMetadata = serde_json::from_str("{}").unwrap();
        assert!(raw.pagination_next.is_empty());
    }
}
