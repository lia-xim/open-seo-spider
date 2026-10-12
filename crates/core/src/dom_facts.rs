//! Original facts from the repaired HTML5 DOM. No computed visibility, browser
//! candidate selection, script execution or proprietary filter behavior is inferred.
use crate::urls::{normalize_url, same_scope};
use crate::{
    DomLocation, DomMetadata, FragmentTarget, HeadingRecord, ImageCandidate, LinkPositionRule,
    LinkRecord, MetaKeywordRecord, NavigationCandidate, ResponsiveImageRecord,
};
use ego_tree::NodeId;
use scraper::{ElementRef, Html, Selector};
use std::collections::{HashMap, HashSet};
use url::Url;

const MAX_NODES: usize = 100_000;
const MAX_DEPTH: usize = 256;
const MAX_PATH_BYTES: usize = 8192;
const MAX_FACTS: usize = 100_000;

fn clean(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn validate_position_rules(rules: &[LinkPositionRule]) -> anyhow::Result<()> {
    if rules.len() > 32 {
        anyhow::bail!("At most 32 link position rules are supported");
    }
    let mut names = HashSet::new();
    for rule in rules {
        let name = rule.name.trim();
        if name.is_empty()
            || name.len() > 128
            || name.chars().any(char::is_control)
            || !names.insert(name)
        {
            anyhow::bail!(
                "Link position names must be unique, nonempty, contain no controls and fit within 128 UTF-8 bytes"
            );
        }
        if rule.selector.is_empty() || rule.selector.len() > 8192 {
            anyhow::bail!("Link position CSS selectors must contain 1 to 8192 UTF-8 bytes");
        }
        Selector::parse(&rule.selector).map_err(|error| {
            anyhow::anyhow!(
                "Invalid link position selector {:?}: {error}",
                rule.selector
            )
        })?;
    }
    Ok(())
}

pub(crate) struct DomContext {
    locations: HashMap<NodeId, DomLocation>,
    positions: HashMap<NodeId, String>,
    limits: Vec<String>,
}

impl DomContext {
    pub(crate) fn new(doc: &Html, rules: &[LinkPositionRule]) -> Self {
        let rules: Vec<_> = rules
            .iter()
            .filter_map(|rule| {
                Selector::parse(&rule.selector)
                    .ok()
                    .map(|selector| (rule.name.trim().to_owned(), selector))
            })
            .collect();
        let mut locations: HashMap<NodeId, DomLocation> = HashMap::new();
        let mut positions = HashMap::new();
        let mut depths = HashMap::new();
        let mut sibling_counts: HashMap<(NodeId, String), usize> = HashMap::new();
        let mut limits = Vec::new();
        let mut count = 0;
        let mut path_bytes = 0usize;
        for node in doc.tree.root().descendants().filter_map(ElementRef::wrap) {
            count += 1;
            if count > MAX_NODES {
                limits.push("dom_nodes_100000".into());
                break;
            }
            let parent = node.parent();
            let parent_id = parent.map(|p| p.id());
            let depth = parent_id
                .and_then(|id| depths.get(&id))
                .copied()
                .unwrap_or(0)
                + 1;
            depths.insert(node.id(), depth);
            let tag = node.value().name();
            let sibling = sibling_counts
                .entry((parent_id.unwrap_or(node.id()), tag.into()))
                .or_default();
            *sibling += 1;
            let parent_location = parent_id.and_then(|id| locations.get(&id));
            let path = if depth > MAX_DEPTH
                || parent_location.is_some_and(|location| location.path.is_none())
            {
                None
            } else {
                let path = format!(
                    "{}/{tag}[{sibling}]",
                    parent_location
                        .and_then(|location| location.path.as_deref())
                        .unwrap_or("")
                );
                if path.len() <= MAX_PATH_BYTES
                    && path_bytes.saturating_add(path.len()) <= 8 * 1024 * 1024
                {
                    path_bytes += path.len();
                    Some(path)
                } else {
                    if !limits.iter().any(|limit| limit == "dom_path_storage_8mib") {
                        limits.push("dom_path_storage_8mib".into());
                    }
                    None
                }
            };
            let mut semantic_ancestors = parent_location
                .map(|location| location.semantic_ancestors.clone())
                .unwrap_or_default();
            if matches!(
                tag,
                "head"
                    | "body"
                    | "nav"
                    | "header"
                    | "footer"
                    | "main"
                    | "aside"
                    | "article"
                    | "section"
            ) {
                // Keep only the bounded classification chain for pathological nesting.
                semantic_ancestors.retain(|ancestor| ancestor != tag);
                semantic_ancestors.push(tag.into());
            }
            let semantic = semantic_ancestors
                .iter()
                .rev()
                .find_map(|tag| match tag.as_str() {
                    "nav" | "header" | "footer" | "aside" | "head" => Some(tag.clone()),
                    "main" | "body" => Some("content".into()),
                    _ => None,
                });
            let link_element = node.value().attr("href").is_some()
                || node.value().attr("src").is_some()
                || node.value().attr("srcset").is_some();
            let custom = rules
                .iter()
                .filter(|_| link_element)
                .find_map(|(name, selector)| {
                    std::iter::once(node)
                        .chain(node.ancestors().filter_map(ElementRef::wrap))
                        .take(MAX_DEPTH)
                        .any(|ancestor| selector.matches(&ancestor))
                        .then(|| name.clone())
                });
            if let Some(position) = custom.or(semantic) {
                positions.insert(node.id(), position);
            }
            locations.insert(
                node.id(),
                DomLocation {
                    index: count,
                    path_truncated: path.is_none(),
                    path,
                    semantic_ancestors,
                },
            );
            if depth > MAX_DEPTH && !limits.iter().any(|limit| limit == "dom_depth_256") {
                limits.push("dom_depth_256".into());
            }
        }
        Self {
            locations,
            positions,
            limits,
        }
    }

    fn location(&self, node: ElementRef<'_>) -> Option<DomLocation> {
        self.locations.get(&node.id()).cloned()
    }

    pub(crate) fn enrich_link(
        &self,
        link: &mut LinkRecord,
        node: ElementRef<'_>,
        raw: &str,
        base: &Url,
    ) {
        link.raw_href = Some(raw.into());
        let resolved = base.join(raw.trim()).ok().filter(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.username().is_empty()
                && url.password().is_none()
        });
        link.fragment = resolved
            .as_ref()
            .and_then(|url| url.fragment().map(str::to_owned));
        link.resolved_href = resolved.map(|url| url.to_string());
        link.dom_location = self.location(node);
        link.position = self.positions.get(&node.id()).cloned();
    }

    pub(crate) fn collect(
        &self,
        doc: &Html,
        base: &Url,
        seeds: &[Url],
        subdomains: bool,
        source: &str,
    ) -> DomMetadata {
        let mut facts = DomMetadata {
            engine: "independent_html5_dom_v1".into(),
            source: source.into(),
            heading_text_engine: Some("independent_dom_text_with_inline_image_alt_v1".into()),
            complete: self.limits.is_empty(),
            limits_reached: self.limits.clone(),
            ..Default::default()
        };
        let mut fact_count = 0;
        let mut candidate_budget = 100_000usize;
        let mut picture_budget = 100_000usize;
        let mut text_budget = 8 * 1024 * 1024;
        for node in doc.tree.root().descendants().filter_map(ElementRef::wrap) {
            let Some(location) = self.location(node) else {
                break;
            };
            if fact_count >= MAX_FACTS {
                facts.limits_reached.push("dom_facts_100000".into());
                facts.complete = false;
                break;
            }
            let tag = node.value().name();
            if location.path_truncated {
                continue;
            }
            // Template contents are inert document fragments, not document-tree targets.
            if node.ancestors().take(MAX_DEPTH).any(|ancestor| {
                ancestor
                    .value()
                    .as_element()
                    .is_some_and(|element| element.name() == "template")
            }) {
                continue;
            }
            let html_element = node.value().name.ns.as_ref() == "http://www.w3.org/1999/xhtml";
            if html_element && matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
                let raw_text = bounded_text(node, &mut text_budget, &mut facts);
                let inline_text = bounded_inline_alt_text(node, &mut text_budget, &mut facts);
                facts.headings.push(HeadingRecord {
                    tag_level: tag.as_bytes()[1] - b'0',
                    text: clean(&raw_text),
                    raw_text,
                    text_with_image_alt: Some(clean(&inline_text)),
                    image_alt: node
                        .select(&Selector::parse("img").expect("static selector"))
                        .map(|img| img.value().attr("alt").map(str::to_owned))
                        .collect(),
                    aria_level: node.value().attr("aria-level").map(str::to_owned),
                    heading_offset: node.value().attr("headingoffset").map(str::to_owned),
                    hidden_attribute: node.value().attr("hidden").is_some(),
                    dom_location: location.clone(),
                });
                fact_count += 1;
            }
            if html_element
                && tag == "meta"
                && node
                    .value()
                    .attr("name")
                    .is_some_and(|name| name.eq_ignore_ascii_case("keywords"))
            {
                let content = node.value().attr("content").map(str::to_owned);
                facts.meta_keywords.push(MetaKeywordRecord {
                    normalized: clean(content.as_deref().unwrap_or("")),
                    chars: content.as_ref().map(|content| content.chars().count()),
                    content,
                    dom_location: location.clone(),
                });
                fact_count += 1;
            }
            if let Some(id) = node.value().attr("id") {
                facts.fragment_targets.push(FragmentTarget {
                    kind: "id".into(),
                    value: id.into(),
                    dom_location: location.clone(),
                });
                fact_count += 1;
            }
            if html_element
                && tag == "a"
                && let Some(name) = node.value().attr("name")
            {
                facts.fragment_targets.push(FragmentTarget {
                    kind: "legacy_anchor_name".into(),
                    value: name.into(),
                    dom_location: location.clone(),
                });
                fact_count += 1;
            }
            if let Some(mut candidate) =
                navigation_candidate(node, location.clone(), base, seeds, subdomains)
            {
                candidate.text = clean(&bounded_text(node, &mut text_budget, &mut facts));
                facts.navigation_candidates.push(candidate);
                fact_count += 1;
            }
            if html_element && tag == "img" {
                let mut record = ResponsiveImageRecord {
                    src: node.value().attr("src").map(str::to_owned),
                    srcset: node.value().attr("srcset").map(str::to_owned),
                    sizes: node.value().attr("sizes").map(str::to_owned),
                    alt: node.value().attr("alt").map(str::to_owned),
                    width: node.value().attr("width").map(str::to_owned),
                    height: node.value().attr("height").map(str::to_owned),
                    loading: node.value().attr("loading").map(str::to_owned),
                    dom_location: location.clone(),
                    ..Default::default()
                };
                if let Some(raw) = &record.src {
                    record.candidates.push(ImageCandidate {
                        source: "img_src".into(),
                        raw_url: raw.clone(),
                        url: normalize_url(raw, Some(base)).map(|url| url.to_string()),
                        eligible: !raw.trim().is_empty(),
                        dom_location: location.clone(),
                        ..Default::default()
                    });
                }
                self.add_srcset(
                    &mut record,
                    node,
                    base,
                    "img_srcset",
                    true,
                    &mut facts,
                    &mut candidate_budget,
                );
                if let Some(parent) = node.parent().and_then(ElementRef::wrap)
                    && parent.value().name() == "picture"
                {
                    let mut preceding = true;
                    for sibling in parent.children().filter_map(ElementRef::wrap) {
                        if picture_budget == 0 {
                            mark_limit(&mut facts, "picture_associations_100000");
                            break;
                        }
                        picture_budget -= 1;
                        if sibling.id() == node.id() {
                            preceding = false;
                        }
                        if sibling.value().name() == "source" {
                            self.add_srcset(
                                &mut record,
                                sibling,
                                base,
                                "picture_source",
                                preceding,
                                &mut facts,
                                &mut candidate_budget,
                            );
                        }
                    }
                }
                fact_count += 1 + record.candidates.len();
                facts.responsive_images.push(record);
            }
        }
        facts
    }

    #[allow(clippy::too_many_arguments)] // The two explicit per-document budgets must be shared across img occurrences.
    fn add_srcset(
        &self,
        image: &mut ResponsiveImageRecord,
        node: ElementRef<'_>,
        base: &Url,
        source: &str,
        applicable: bool,
        facts: &mut DomMetadata,
        budget: &mut usize,
    ) {
        let Some(srcset) = node.value().attr("srcset") else {
            return;
        };
        if *budget == 0 {
            mark_limit(facts, "image_candidates_100000");
            return;
        }
        let rows = crate::srcset::parse_srcset(srcset);
        if rows.len() >= 10_000 {
            facts.complete = false;
            if !facts
                .limits_reached
                .iter()
                .any(|limit| limit == "srcset_candidates_10000")
            {
                facts.limits_reached.push("srcset_candidates_10000".into());
            }
        }
        for mut row in rows {
            if *budget == 0 {
                mark_limit(facts, "image_candidates_100000");
                break;
            }
            *budget -= 1;
            row.source = source.into();
            row.url = normalize_url(&row.raw_url, Some(base)).map(|url| url.to_string());
            row.media = node.value().attr("media").map(str::to_owned);
            row.type_hint = node.value().attr("type").map(str::to_owned);
            row.sizes = node.value().attr("sizes").map(str::to_owned);
            row.eligible &= applicable;
            row.dom_location = self.location(node).unwrap_or_else(|| DomLocation {
                path_truncated: true,
                ..Default::default()
            });
            if !applicable && row.error.is_none() {
                row.error =
                    Some("Source follows this img and cannot participate in its source set".into());
            }
            image.candidates.push(row);
        }
    }
}

fn mark_limit(facts: &mut DomMetadata, name: &str) {
    facts.complete = false;
    if !facts.limits_reached.iter().any(|limit| limit == name) {
        facts.limits_reached.push(name.into());
    }
}

fn bounded_text(node: ElementRef<'_>, budget: &mut usize, facts: &mut DomMetadata) -> String {
    bounded_text_parts(node.text(), budget, facts)
}

/// Own DOM-order profile: text nodes plus img alt at its element position. This
/// intentionally does not compute visibility, rendered text, or accessibility names.
fn bounded_inline_alt_text(
    node: ElementRef<'_>,
    budget: &mut usize,
    facts: &mut DomMetadata,
) -> String {
    let parts = node.descendants().filter_map(|descendant| {
        if descendant.ancestors().take(MAX_DEPTH).any(|ancestor| {
            ancestor
                .value()
                .as_element()
                .is_some_and(|element| element.name() == "template")
        }) {
            return None;
        }
        match descendant.value() {
            scraper::Node::Text(text) => Some(text.text.as_ref()),
            scraper::Node::Element(element)
                if element.name() == "img"
                    && element.name.ns.as_ref() == "http://www.w3.org/1999/xhtml" =>
            {
                element.attr("alt")
            }
            _ => None,
        }
    });
    bounded_text_parts(parts, budget, facts)
}

fn bounded_text_parts<'a>(
    parts: impl Iterator<Item = &'a str>,
    budget: &mut usize,
    facts: &mut DomMetadata,
) -> String {
    let max = (*budget).min(64 * 1024);
    let mut output = String::new();
    for text in parts {
        if output.len().saturating_add(text.len()) > max {
            let mut end = max.saturating_sub(output.len()).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            output.push_str(&text[..end]);
            mark_limit(facts, "dom_text_64kib_per_value_8mib_total");
            break;
        }
        output.push_str(text);
    }
    *budget = budget.saturating_sub(output.len());
    output
}

fn navigation_candidate(
    node: ElementRef<'_>,
    location: DomLocation,
    base: &Url,
    seeds: &[Url],
    subdomains: bool,
) -> Option<NavigationCandidate> {
    let element = node.value();
    let tag = element.name();
    let href = element.attr("href");
    let router = element
        .attr("routerlink")
        .or_else(|| element.attr("data-href"));
    let role_link = element.attr("role").is_some_and(|role| {
        role.split_ascii_whitespace()
            .any(|token| token.eq_ignore_ascii_case("link"))
    });
    let anchor = matches!(tag, "a" | "area");
    let raw = href.or(router);
    let resolved = raw.and_then(|raw| base.join(raw.trim()).ok());
    let potential = raw.and_then(|raw| normalize_url(raw, Some(base)));
    let reason = if anchor && href.is_some() && potential.is_some() {
        return None;
    } else if anchor && href.is_none() {
        "anchor_without_href"
    } else if anchor
        && resolved
            .as_ref()
            .is_some_and(|url| url.scheme() == "javascript")
    {
        "javascript_href"
    } else if anchor
        && resolved
            .as_ref()
            .is_some_and(|url| !matches!(url.scheme(), "http" | "https"))
    {
        "non_http_scheme"
    } else if anchor {
        "invalid_http_href"
    } else if href.is_some() && !matches!(tag, "link" | "base") {
        "href_on_non_anchor"
    } else if router.is_some() {
        "custom_navigation_attribute"
    } else if role_link {
        "role_link_without_anchor_href"
    } else {
        return None;
    };
    Some(NavigationCandidate {
        tag: tag.into(),
        raw_href: raw.map(str::to_owned),
        resolved_href: resolved.as_ref().map(Url::to_string),
        potential_url: potential.as_ref().map(Url::to_string),
        scheme: resolved.as_ref().map(|url| url.scheme().into()),
        reason: reason.into(),
        onclick: element.attr("onclick").map(str::to_owned),
        router_link: router.map(str::to_owned),
        text: String::new(), // Filled once by the bounded document-wide text collector.
        internal: potential
            .as_ref()
            .map(|url| same_scope(url, seeds, subdomains)),
        dom_location: location,
    })
}

/// Review notices based on captured tag order and metadata; not accessibility
/// computed heading levels or an assertion of search-engine ranking penalties.
pub fn audit_dom_metadata(pages: &[crate::PageRecord]) -> Vec<crate::AuditIssue> {
    use crate::{AuditIssue, Severity};
    let mut issues = Vec::new();
    for page in pages {
        let Some(facts) = &page.dom_metadata else {
            continue;
        };
        if !facts.complete || !crate::onpage::complete_html(page) {
            continue;
        }
        let mut notice = |code: &str, message: String| {
            issues.push(AuditIssue {
                code: code.into(),
                severity: Severity::Notice,
                url: page.url.clone(),
                message,
                related_urls: Vec::new(),
            })
        };
        if facts.headings.iter().any(|heading| heading.tag_level == 1)
            && facts
                .headings
                .first()
                .is_some_and(|heading| heading.tag_level != 1)
        {
            notice("h1_nonsequential", "An H1 occurs after another heading tag in captured DOM order; this is a structural review observation".into());
        }
        if let (Some(h1), Some(h2)) = (
            facts
                .headings
                .iter()
                .position(|heading| heading.tag_level == 1),
            facts
                .headings
                .iter()
                .position(|heading| heading.tag_level == 2),
        ) && h1 < h2
            && facts.headings[h1 + 1..h2]
                .iter()
                .any(|heading| heading.tag_level > 2)
        {
            notice("h2_nonsequential", "A higher-numbered heading tag occurs between the first H1 and first H2 in captured DOM order; this bounded tag-order policy has scoped local reference evidence".into());
        }
        if facts
            .headings
            .windows(2)
            .any(|pair| pair[1].tag_level > pair[0].tag_level + 1)
        {
            notice("heading_level_skipped", "Captured heading tags increase by more than one level; computed aria-level/heading offsets and visual layout are not inferred".into());
        }
        for level in [1, 2] {
            if facts.headings.iter().any(|heading| {
                heading.tag_level == level
                    && heading
                        .image_alt
                        .iter()
                        .any(|alt| alt.as_ref().is_some_and(|text| !text.trim().is_empty()))
            }) {
                notice(
                    &format!("h{level}_image_alt"),
                    format!(
                        "A captured H{level} contains an image with nonempty alt; alt is stored separately from heading text"
                    ),
                );
            }
        }
        if facts.meta_keywords.is_empty() {
            notice("meta_keywords_missing", "No meta keyword element was observed; keywords are metadata observations, not a recommended ranking input".into());
        }
        if facts
            .meta_keywords
            .iter()
            .any(|record| record.normalized.is_empty())
        {
            notice("meta_keywords_empty", "An observed meta keyword element has missing, empty or whitespace-only content; attribute presence is retained separately".into());
        }
        if facts.meta_keywords.len() > 1 {
            notice(
                "meta_keywords_multiple",
                format!(
                    "{} meta keyword elements observed in DOM order",
                    facts.meta_keywords.len()
                ),
            );
        }
        if facts.navigation_candidates.iter().any(|candidate| {
            candidate.internal == Some(true) && candidate.reason != "non_http_scheme"
        }) {
            notice("nonstandard_internal_navigation", "Potential internal navigation was observed outside an HTTP anchor href; it is excluded from anchor graph edges and not executed".into());
        }
    }
    let mut duplicates: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for page in pages
        .iter()
        .filter(|page| page.indexable && crate::onpage::complete_html(page))
    {
        if let Some(facts) = &page.dom_metadata
            && facts.complete
        {
            for record in &facts.meta_keywords {
                if !record.normalized.is_empty() {
                    duplicates
                        .entry(record.normalized.clone())
                        .or_default()
                        .insert(page.url.clone());
                }
            }
        }
    }
    for (text, urls) in duplicates.into_iter().filter(|(_, urls)| urls.len() > 1) {
        for url in &urls {
            issues.push(AuditIssue { code: "meta_keywords_duplicate".into(), severity: Severity::Notice, url: url.clone(), message: format!("Exact case-sensitive whitespace-normalized meta keyword value {text:?} occurs on {} captured indexable URLs", urls.len()), related_urls: urls.iter().filter(|other| *other != url).cloned().collect() });
        }
    }
    issues.sort_by(|a, b| (&a.url, &a.code, &a.message).cmp(&(&b.url, &b.code, &b.message)));
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PageRecord, RawMetadata, extract_html};
    fn observed(html: &str, rules: &[LinkPositionRule]) -> PageRecord {
        let mut page = PageRecord {
            url: "https://example.org/a".into(),
            final_url: "https://example.org/a".into(),
            internal: true,
            status: Some(200),
            final_status: Some(200),
            content_type: "text/html".into(),
            indexable: true,
            ..Default::default()
        };
        crate::extract::extract_html_in_source(
            &mut page,
            html,
            &[Url::parse("https://example.org").unwrap()],
            false,
            rules,
            "raw_html",
        );
        page
    }
    #[test]
    fn heading_order_keywords_occurrences_and_tree_paths_are_preserved() {
        let page = observed(
            "<head><meta name='KEYWORDS'><meta name='keywords' content=''><meta name='keywords' content=' café, 🦀 &amp; A  '></head><body><h2> Before </h2><h1 hidden aria-level='3'><img alt='Logo'> Main &amp; title</h1><h3>Detail</h3><h2>Again</h2></body>",
            &[],
        );
        let facts = page.dom_metadata.as_ref().unwrap();
        assert!(facts.complete);
        assert_eq!(facts.source, "raw_html");
        assert_eq!(
            facts
                .headings
                .iter()
                .map(|h| h.tag_level)
                .collect::<Vec<_>>(),
            vec![2, 1, 3, 2]
        );
        assert_eq!(page.h1, vec!["Main & title"]);
        assert_eq!(facts.headings[1].image_alt, vec![Some("Logo".into())]);
        assert_eq!(
            facts.headings[1].text_with_image_alt.as_deref(),
            Some("Logo Main & title")
        );
        assert_eq!(
            facts.heading_text_engine.as_deref(),
            Some("independent_dom_text_with_inline_image_alt_v1")
        );
        assert!(facts.headings[1].hidden_attribute);
        assert_eq!(facts.headings[1].aria_level.as_deref(), Some("3"));
        assert_eq!(
            facts.headings[0].dom_location.path.as_deref(),
            Some("/html[1]/body[1]/h2[1]")
        );
        assert_eq!(
            facts.headings[3].dom_location.path.as_deref(),
            Some("/html[1]/body[1]/h2[2]")
        );
        assert!(facts.headings[0].dom_location.index < facts.headings[1].dom_location.index);
        assert_eq!(facts.meta_keywords[0].content, None);
        assert_eq!(facts.meta_keywords[0].chars, None);
        assert_eq!(facts.meta_keywords[1].content.as_deref(), Some(""));
        assert_eq!(facts.meta_keywords[1].chars, Some(0));
        assert_eq!(facts.meta_keywords[2].normalized, "café, 🦀 & A");
        assert_eq!(
            serde_json::to_value(RawMetadata::from(&page)).unwrap()["dom_metadata"],
            serde_json::to_value(facts).unwrap()
        );
    }
    #[test]
    fn semantic_positions_and_ordered_custom_ancestor_rules_are_deterministic() {
        let rules = [
            LinkPositionRule {
                name: "Special".into(),
                selector: ".special".into(),
            },
            LinkPositionRule {
                name: "Navigation".into(),
                selector: "nav".into(),
            },
        ];
        let page = observed(
            "<header><nav class='special'><a href='/one#x'>One</a></nav></header><nav><a href='/two'>Two</a></nav><main><a href='/three'>Three</a></main><footer><a href='/four'>Four</a></footer>",
            &rules,
        );
        let links: Vec<_> = page
            .links
            .iter()
            .filter(|link| link.kind == "anchor")
            .collect();
        assert_eq!(
            links
                .iter()
                .map(|link| link.position.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("Special"),
                Some("Navigation"),
                Some("content"),
                Some("footer")
            ]
        );
        assert_eq!(links[0].raw_href.as_deref(), Some("/one#x"));
        assert_eq!(
            links[0].resolved_href.as_deref(),
            Some("https://example.org/one#x")
        );
        assert_eq!(links[0].fragment.as_deref(), Some("x"));
        assert_eq!(links[0].url, "https://example.org/one");
        assert_eq!(
            links[0].dom_location.as_ref().unwrap().semantic_ancestors,
            vec!["body", "header", "nav"]
        );
        assert!(
            validate_position_rules(&[LinkPositionRule {
                name: "x".into(),
                selector: "[".into()
            }])
            .is_err()
        );
        assert!(
            validate_position_rules(&[
                LinkPositionRule {
                    name: "x".into(),
                    selector: "a".into()
                },
                LinkPositionRule {
                    name: " x ".into(),
                    selector: "p".into()
                }
            ])
            .is_err()
        );
        assert!(
            validate_position_rules(&vec![
                LinkPositionRule {
                    name: "x".into(),
                    selector: "a".into()
                };
                33
            ])
            .is_err()
        );
    }
    #[test]
    fn responsive_picture_candidates_retain_alt_and_do_not_claim_selection() {
        let page = observed(
            "<base href='/assets/'><picture><source media='(min-width: 900px)' type='image/webp' srcset='large.webp 2x, invalid.webp 0w'><img alt='First' srcset='one.png 1x, data:image/png;base64,AAA 2x'><source srcset='late.png 3x'><img src='fallback.png' alt='Second'></picture>",
            &[],
        );
        let facts = page.dom_metadata.as_ref().unwrap();
        assert_eq!(facts.responsive_images.len(), 2);
        let first = &facts.responsive_images[0];
        assert_eq!(first.alt.as_deref(), Some("First"));
        assert!(first.src.is_none());
        let large = first
            .candidates
            .iter()
            .find(|candidate| candidate.raw_url == "large.webp")
            .unwrap();
        assert!(large.eligible);
        assert_eq!(
            large.url.as_deref(),
            Some("https://example.org/assets/large.webp")
        );
        assert_eq!(large.type_hint.as_deref(), Some("image/webp"));
        let late = first
            .candidates
            .iter()
            .find(|candidate| candidate.raw_url == "late.png")
            .unwrap();
        assert!(!late.eligible);
        assert!(late.error.is_some());
        assert!(
            facts.responsive_images[1]
                .candidates
                .iter()
                .find(|candidate| candidate.raw_url == "late.png")
                .unwrap()
                .eligible
        );
        assert!(
            first
                .candidates
                .iter()
                .find(|candidate| candidate.raw_url.starts_with("data:"))
                .unwrap()
                .url
                .is_none()
        );
        assert!(
            !page
                .links
                .iter()
                .any(|link| link.url.contains("invalid.webp"))
        );
        assert_eq!(page.images.len(), 1);
        assert_eq!(page.images[0].alt.as_deref(), Some("Second"));
    }
    #[test]
    fn navigation_evidence_does_not_invent_urls_or_graph_edges() {
        let page = observed(
            "<div href='/div'>Div</div><a onclick=\"goto('/secret')\">Script</a><a href='javascript:go()'>JS</a><a routerLink='/router'>Router</a><span role='link'>Role</span><a href='mailto:test@example.org'>Mail</a><a href='/good' onclick='noop()'>Good</a>",
            &[],
        );
        assert_eq!(
            page.links
                .iter()
                .filter(|link| link.kind == "anchor")
                .count(),
            1
        );
        let rows = &page.dom_metadata.as_ref().unwrap().navigation_candidates;
        assert_eq!(rows.len(), 6);
        assert_eq!(
            rows[0].potential_url.as_deref(),
            Some("https://example.org/div")
        );
        assert_eq!(rows[0].internal, Some(true));
        assert!(rows[1].potential_url.is_none());
        assert!(rows[1].onclick.as_ref().unwrap().contains("/secret"));
        assert!(
            rows.iter()
                .any(|row| row.reason == "non_http_scheme"
                    && row.scheme.as_deref() == Some("mailto"))
        );
        assert!(!page.links.iter().any(|link| link.url.ends_with("/div")
            || link.url.ends_with("/router")
            || link.url.ends_with("/secret")));
    }

    #[test]
    fn inert_template_contents_are_not_document_targets_headings_or_crawl_edges() {
        let page = observed(
            "<head><template><base href='/inert/'><meta name='keywords' content='inert'><h1>Template</h1><div id='inert'></div><a href='/inert#x'>Inert</a><img src='/inert.png'></template></head><body><h1 id='real'>Real</h1><a href='/real#real'>Real</a></body>",
            &[],
        );
        let facts = page.dom_metadata.as_ref().unwrap();
        assert_eq!(page.h1, vec!["Real"]);
        assert_eq!(facts.headings.len(), 1);
        assert!(facts.meta_keywords.is_empty());
        assert!(
            facts
                .fragment_targets
                .iter()
                .all(|target| target.value != "inert")
        );
        assert!(page.base_href.is_none());
        assert_eq!(page.links.len(), 1);
        assert_eq!(page.links[0].url, "https://example.org/real");
        assert!(page.images.is_empty());
    }
    #[test]
    fn review_notices_have_controls_and_incomplete_or_legacy_never_prove_absence() {
        let p = observed(
            "<h2>Before</h2><h1>Main<img alt='Logo'></h1><h3>Detail</h3><meta name='keywords' content=''>",
            &[],
        );
        let codes: Vec<_> = audit_dom_metadata(std::slice::from_ref(&p))
            .into_iter()
            .map(|issue| issue.code)
            .collect();
        assert!(codes.iter().any(|code| code == "h1_nonsequential"));
        assert!(!codes.iter().any(|code| code == "h2_nonsequential"));
        assert!(codes.iter().any(|code| code == "heading_level_skipped"));
        assert!(codes.iter().any(|code| code == "meta_keywords_empty"));
        let control = observed(
            "<h1>Main</h1><h2>Section</h2><h2>Other</h2><h3>Detail</h3><meta name='keywords' content='Observed'>",
            &[],
        );
        assert!(audit_dom_metadata(std::slice::from_ref(&control)).is_empty());
        for variant in 0..5 {
            let mut failed = p.clone();
            match variant {
                0 => failed.dom_metadata = None,
                1 => failed.truncated = true,
                2 => failed.renderer_error = Some("failed".into()),
                3 => failed.dom_metadata.as_mut().unwrap().complete = false,
                _ => {
                    failed.rendered = true;
                    failed.rendered_status = None;
                    failed.dom_metadata.as_mut().unwrap().source = "rendered_html".into();
                }
            };
            assert!(audit_dom_metadata(&[failed]).is_empty());
        }
        let mut other = control.clone();
        other.url = "https://example.org/other".into();
        other.final_url = other.url.clone();
        assert_eq!(
            audit_dom_metadata(&[control.clone(), other.clone()])
                .iter()
                .filter(|issue| issue.code == "meta_keywords_duplicate")
                .count(),
            2
        );
        other.dom_metadata.as_mut().unwrap().meta_keywords[0].normalized = "observed".into();
        assert!(audit_dom_metadata(&[control, other]).is_empty());
    }
    #[test]
    fn first_h1_first_h2_policy_retains_higher_tag_observation_with_repeated_h1_controls() {
        for (levels, expected) in [
            (vec![1, 3, 2], true),
            (vec![1, 3, 1, 2], true),
            (vec![1, 1, 2], false),
            (vec![1, 2, 1, 2], false),
            (vec![2, 1, 3, 2], false),
            (vec![3, 2], false),
            (vec![2], false),
            (vec![1, 2, 3, 2], false),
            (vec![1, 2, 2], false),
            (vec![3, 1, 2], false),
        ] {
            let html = levels
                .iter()
                .map(|level| format!("<h{level}>Fixture</h{level}>"))
                .collect::<String>();
            let page = observed(&html, &[]);
            assert_eq!(
                audit_dom_metadata(&[page])
                    .iter()
                    .any(|issue| issue.code == "h2_nonsequential"),
                expected,
                "{levels:?}"
            );
        }
    }

    #[test]
    fn independent_inline_alt_profile_preserves_order_whitespace_empty_and_legacy_unknown() {
        let page = observed(
            "<h1>A<img alt='🦀'><b>B<img alt=' C &amp; D '></b><img> E</h1><h2><img alt='Image-only section'></h2><h3></h3>",
            &[],
        );
        let facts = page.dom_metadata.unwrap();
        assert_eq!(facts.headings[0].text, "AB E");
        assert_eq!(
            facts.headings[0].text_with_image_alt.as_deref(),
            Some("A🦀B C & D E")
        );
        assert_eq!(facts.headings[1].text, "");
        assert_eq!(
            facts.headings[1].text_with_image_alt.as_deref(),
            Some("Image-only section")
        );
        assert_eq!(facts.headings[2].text_with_image_alt.as_deref(), Some(""));
        let legacy: HeadingRecord =
            serde_json::from_str(r#"{"tag_level":1,"text":"Legacy"}"#).unwrap();
        assert_eq!(legacy.text_with_image_alt, None);
        assert_eq!(page.h1, vec!["AB E"]);
        assert_eq!(page.h2, vec![""]);
    }
    #[test]
    fn legacy_json_is_unknown_and_explicit_bounds_are_visible() {
        let legacy:PageRecord=serde_json::from_str(r#"{"url":"https://example.org/a","links":[{"url":"https://example.org/b","text":"B","rel":"","internal":true,"nofollow":false,"kind":"anchor"}]}"#).unwrap();
        assert!(legacy.dom_metadata.is_none());
        assert!(legacy.links[0].dom_location.is_none());
        assert!(legacy.links[0].fragment.is_none());
        let page = observed(&format!("<h1>{}</h1>", "🦀".repeat(20000)), &[]);
        let facts = page.dom_metadata.unwrap();
        assert!(!facts.complete);
        assert!(
            facts
                .limits_reached
                .iter()
                .any(|limit| limit.starts_with("dom_text_"))
        );
        assert!(facts.headings[0].raw_text.len() <= 65536);
        let deep = observed(
            &format!(
                "{}<a id='deep' href='/b#x'>Deep</a>{}",
                "<div>".repeat(270),
                "</div>".repeat(270)
            ),
            &[],
        );
        assert!(!deep.dom_metadata.as_ref().unwrap().complete);
        assert!(deep.links[0].dom_location.as_ref().unwrap().path_truncated);
        let mut empty = PageRecord {
            final_url: "https://example.org/a".into(),
            ..Default::default()
        };
        extract_html(&mut empty, "", &[], false);
        assert!(empty.dom_metadata.is_some());
    }
}
