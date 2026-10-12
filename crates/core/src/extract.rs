use crate::urls::{normalize_url, same_scope};
use crate::{
    CustomExtraction, CustomExtractionConfig, ImageRecord, LinkRecord, LinkRelationship, PageRecord,
};
use scraper::{Html, Selector};
use sha2::{Digest, Sha256};
use url::Url;

fn select(selector: &str) -> Selector {
    Selector::parse(selector).expect("static selector")
}
fn clean(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn texts(doc: &Html, selector: &str) -> Vec<String> {
    doc.select(&select(selector))
        .map(|node| clean(&node.text().collect::<String>()))
        .collect()
}

pub fn extract_html(page: &mut PageRecord, html: &str, seeds: &[Url], include_subdomains: bool) {
    let doc = Html::parse_document(html);
    let Some(request_url) = normalize_url(&page.final_url, None) else {
        return;
    };
    crate::metadata::reset_html_relationships(page);
    page.meta_descriptions.clear();
    page.meta_robots.clear();
    page.robots_directives
        .retain(|rule| rule.source.ends_with("http_header"));
    page.images.clear();
    page.links.clear();
    page.structured_data.clear();
    page.invalid_structured_data = 0;
    page.language = None;
    page.viewport = None;
    page.base_href = doc
        .select(&select("base[href]"))
        .next()
        .and_then(|n| n.value().attr("href"))
        .map(str::to_owned);
    let base = page
        .base_href
        .as_ref()
        .and_then(|href| request_url.join(href.trim()).ok())
        .filter(|url| !matches!(url.scheme(), "data" | "javascript"))
        .unwrap_or_else(|| request_url.clone());
    page.document_base_url = Some(base.to_string());
    page.titles = texts(&doc, "title");
    page.h1 = texts(&doc, "h1");
    page.h2 = texts(&doc, "h2");
    page.language = doc
        .select(&select("html"))
        .next()
        .and_then(|n| n.value().attr("lang"))
        .map(str::to_owned);
    for node in doc.select(&select("meta")) {
        let name = node.value().attr("name").unwrap_or("").to_ascii_lowercase();
        let content = node.value().attr("content").unwrap_or("").trim().to_owned();
        match name.as_str() {
            "description" => page.meta_descriptions.push(content),
            "viewport" => page.viewport = Some(content),
            _ => crate::metadata::add_meta_robots(page, &name, &content),
        }
    }
    crate::text_metrics::measure_metadata(page);
    for node in doc.select(&select("link[href]")) {
        let rel = node.value().attr("rel").unwrap_or("").to_ascii_lowercase();
        let Some(target) = node
            .value()
            .attr("href")
            .and_then(|s| normalize_url(s, Some(&base)))
        else {
            continue;
        };
        crate::metadata::apply_relationship(
            page,
            LinkRelationship {
                url: target.to_string(),
                rel: rel.split_ascii_whitespace().map(str::to_owned).collect(),
                source: "html_link".into(),
                context_url: request_url.to_string(),
                applies_to_document: true,
                hreflang: node
                    .value()
                    .attr("hreflang")
                    .map(|language| vec![language.to_owned()])
                    .unwrap_or_default(),
                media: node.value().attr("media").map(str::to_owned),
                type_hint: node.value().attr("type").map(str::to_owned),
                title: node.value().attr("title").map(str::to_owned),
                ..Default::default()
            },
        );
        if rel.split_whitespace().any(|r| r == "stylesheet") {
            page.links.push(LinkRecord {
                url: target.to_string(),
                rel,
                internal: same_scope(&target, seeds, include_subdomains),
                kind: "stylesheet".into(),
                ..Default::default()
            });
        }
    }
    for node in doc.select(&select("a[href], area[href]")) {
        let Some(target) = node
            .value()
            .attr("href")
            .and_then(|s| normalize_url(s, Some(&base)))
        else {
            continue;
        };
        let rel = node.value().attr("rel").unwrap_or("").to_ascii_lowercase();
        if !rel.is_empty() {
            crate::metadata::apply_relationship(
                page,
                LinkRelationship {
                    url: target.to_string(),
                    rel: rel.split_ascii_whitespace().map(str::to_owned).collect(),
                    source: "html_anchor".into(),
                    context_url: request_url.to_string(),
                    applies_to_document: true,
                    ..Default::default()
                },
            );
        }
        page.links.push(LinkRecord {
            url: target.to_string(),
            text: clean(&node.text().collect::<String>()),
            nofollow: rel.split_whitespace().any(|r| r == "nofollow"),
            rel,
            internal: same_scope(&target, seeds, include_subdomains),
            kind: "anchor".into(),
        });
    }
    for node in doc.select(&select("img")) {
        let Some(target) = node
            .value()
            .attr("src")
            .and_then(|s| normalize_url(s, Some(&base)))
        else {
            continue;
        };
        page.images.push(ImageRecord {
            url: target.to_string(),
            alt: node.value().attr("alt").map(str::to_owned),
            width: node.value().attr("width").map(str::to_owned),
            height: node.value().attr("height").map(str::to_owned),
            loading: node.value().attr("loading").map(str::to_owned),
        });
        page.links.push(LinkRecord {
            url: target.to_string(),
            internal: same_scope(&target, seeds, include_subdomains),
            kind: "image".into(),
            ..Default::default()
        });
    }
    for node in doc.select(&select(
        "script[src], iframe[src], video[src], audio[src], source[src]",
    )) {
        let Some(target) = node
            .value()
            .attr("src")
            .and_then(|s| normalize_url(s, Some(&base)))
        else {
            continue;
        };
        page.links.push(LinkRecord {
            url: target.to_string(),
            internal: same_scope(&target, seeds, include_subdomains),
            kind: node.value().name().into(),
            ..Default::default()
        });
    }
    page.script_count = doc.select(&select("script")).count();
    crate::metadata::evaluate_robots(page);
    for node in doc.select(&select("script[type='application/ld+json']")) {
        match serde_json::from_str(&node.text().collect::<String>()) {
            Ok(value) => page.structured_data.push(value),
            Err(_) => page.invalid_structured_data += 1,
        }
    }
    if let Some(body) = doc.select(&select("body")).next() {
        let text = body
            .descendants()
            .filter_map(|n| {
                let text = n.value().as_text()?;
                if n.ancestors().any(|a| {
                    a.value().as_element().is_some_and(|e| {
                        matches!(e.name(), "script" | "style" | "noscript" | "template")
                    })
                }) {
                    return None;
                }
                Some(text.to_string())
            })
            .collect::<Vec<_>>()
            .join(" ");
        let normalized = clean(&text);
        page.word_count = normalized.split_whitespace().count();
        page.content_hash = Some(format!("{:x}", Sha256::digest(normalized.as_bytes())));
    }
    crate::metadata::sync_relationship_links(page, seeds, include_subdomains);
}

/// CSS selectors are explicit user input; invalid selectors return an error.
pub fn extract_custom(
    html: &str,
    selector: &str,
    attribute: Option<&str>,
) -> anyhow::Result<CustomExtraction> {
    crate::extraction_engine::extract_configured(
        html,
        &CustomExtractionConfig {
            selector: selector.into(),
            attribute: attribute.map(str::to_owned),
            ..Default::default()
        },
        "raw_html",
    )
}

pub(crate) fn validate_custom_extractions(config: &[CustomExtractionConfig]) -> anyhow::Result<()> {
    crate::extraction_engine::validate_custom_extractions(config)
}

pub(crate) fn apply_custom_extractions(
    page: &mut PageRecord,
    html: &str,
    config: &[CustomExtractionConfig],
    source: &str,
) {
    page.custom_extractions = crate::extraction_engine::extract_all(html, config, source);
    for result in &page.custom_extractions {
        if let Some(error) = &result.error {
            page.warnings
                .push(format!("Custom extraction {} failed: {error}", result.name));
        } else if result.truncated {
            page.warnings.push(format!(
                "Custom extraction {} retained {} of {} matching values",
                result.name,
                result.values.len(),
                result.matched_count.unwrap_or_default()
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_metadata_base_links_and_visible_text() {
        let mut page = PageRecord {
            final_url: "https://example.org/a/".into(),
            ..Default::default()
        };
        extract_html(
            &mut page,
            r#"<html lang="de"><head><base href="/b/"><title>  Test &amp; title </title><meta name="DESCRIPTION" content="A description"><link rel="canonical" href="../a/"><link rel="alternate" hreflang="en" href="/en/"><script type="application/ld+json">{"@type":"WebPage"}</script></head><body><h1>Hello world</h1><h2>Details</h2><a href="p#fragment" rel="nofollow">Next</a><a href="mailto:a@example.org">Email</a><img src="i.png"><script>ignore these four words</script></body></html>"#,
            &[Url::parse("https://example.org/").unwrap()],
            false,
        );
        assert_eq!(page.title(), "Test & title");
        assert_eq!(page.links[0].url, "https://example.org/b/p");
        assert!(page.links[0].nofollow);
        assert_eq!(page.word_count, 5); // Hello world, Details, Next, Email
        assert_eq!(page.canonical, vec!["https://example.org/a/"]);
        assert!(page.images[0].alt.is_none());
        assert_eq!(page.structured_data.len(), 1);
    }
    #[test]
    fn custom_extraction_handles_attributes_and_invalid_selectors() {
        assert_eq!(
            extract_custom("<a href='/test'> Text </a>", "a", Some("href"))
                .unwrap()
                .values,
            vec!["/test"]
        );
        assert!(extract_custom("", "[", None).is_err());
    }

    #[test]
    fn first_base_governs_links_and_unsupported_base_does_not_invent_http_targets() {
        let seeds = [Url::parse("https://example.org/").unwrap()];
        for (base_markup, expected_base, expected_link) in [
            (
                "<base href='/first/'><base href='/second/'>",
                "https://example.org/first/",
                Some("https://example.org/first/path"),
            ),
            (
                "<base href='https://[invalid'><base href='/second/'>",
                "https://example.org/original/",
                Some("https://example.org/original/path"),
            ),
            (
                "<base href='data:text/plain,a'><base href='/second/'>",
                "https://example.org/original/",
                Some("https://example.org/original/path"),
            ),
            ("<base href='file:///tmp/'>", "file:///tmp/", None),
        ] {
            let mut page = PageRecord {
                final_url: "https://example.org/original/".into(),
                ..Default::default()
            };
            extract_html(
                &mut page,
                &format!(
                    "<html><head>{base_markup}</head><body><a href='path'>Link</a></body></html>"
                ),
                &seeds,
                false,
            );
            assert_eq!(page.document_base_url.as_deref(), Some(expected_base));
            assert_eq!(
                page.links.first().map(|link| link.url.as_str()),
                expected_link
            );
        }
    }
}
