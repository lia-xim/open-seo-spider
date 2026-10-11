use crate::urls::{normalize_url, same_scope};
use crate::{
    CustomExtraction, CustomExtractionConfig, HreflangRecord, ImageRecord, LinkRecord, PageRecord,
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
    let base = doc
        .select(&select("base[href]"))
        .next()
        .and_then(|n| n.value().attr("href"))
        .and_then(|href| normalize_url(href, Some(&request_url)))
        .unwrap_or_else(|| request_url.clone());
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
            "robots" | "googlebot" => page.meta_robots.push(content),
            "viewport" => page.viewport = Some(content),
            _ => {}
        }
    }
    for node in doc.select(&select("link[href]")) {
        let rel = node.value().attr("rel").unwrap_or("").to_ascii_lowercase();
        let Some(target) = node
            .value()
            .attr("href")
            .and_then(|s| normalize_url(s, Some(&base)))
        else {
            continue;
        };
        if rel.split_whitespace().any(|r| r == "canonical") {
            page.canonical.push(target.to_string());
        }
        if rel.split_whitespace().any(|r| r == "alternate")
            && let Some(language) = node.value().attr("hreflang")
        {
            page.hreflang.push(HreflangRecord {
                language: language.into(),
                url: target.to_string(),
            });
        }
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
}

/// CSS selectors are explicit user input; invalid selectors return an error.
pub fn extract_custom(
    html: &str,
    selector: &str,
    attribute: Option<&str>,
) -> anyhow::Result<CustomExtraction> {
    let parsed =
        Selector::parse(selector).map_err(|e| anyhow::anyhow!("Invalid CSS selector: {e:?}"))?;
    let doc = Html::parse_document(html);
    let values = doc
        .select(&parsed)
        .filter_map(|node| match attribute {
            Some(attribute) => node.value().attr(attribute).map(str::to_owned),
            None => Some(clean(&node.text().collect::<String>())),
        })
        .collect();
    Ok(CustomExtraction {
        selector: selector.into(),
        attribute: attribute.map(str::to_owned),
        values,
        ..Default::default()
    })
}

pub(crate) fn validate_custom_extractions(config: &[CustomExtractionConfig]) -> anyhow::Result<()> {
    if config.len() > 128 {
        anyhow::bail!("At most 128 custom extraction definitions are supported per crawl");
    }
    let mut names = std::collections::HashSet::new();
    for extraction in config {
        if extraction.name.trim().is_empty() || extraction.name.chars().count() > 128 {
            anyhow::bail!("Custom extraction name must contain 1 to 128 characters");
        }
        if !names.insert(extraction.name.trim().to_ascii_lowercase()) {
            anyhow::bail!("Duplicate custom extraction name: {}", extraction.name);
        }
        if extraction.selector.len() > 8192 {
            anyhow::bail!("CSS selector exceeds 8192 bytes");
        }
        Selector::parse(&extraction.selector).map_err(|error| {
            anyhow::anyhow!(
                "Invalid CSS selector for extraction {}: {error:?}",
                extraction.name
            )
        })?;
        if let Some(attribute) = &extraction.attribute
            && (attribute.is_empty()
                || attribute.len() > 128
                || attribute.chars().any(|character| {
                    character.is_whitespace()
                        || character.is_control()
                        || matches!(character, '<' | '>' | '=' | '\'' | '"')
                }))
        {
            anyhow::bail!(
                "Invalid attribute name for custom extraction {}",
                extraction.name
            );
        }
    }
    Ok(())
}

pub(crate) fn apply_custom_extractions(
    page: &mut PageRecord,
    html: &str,
    config: &[CustomExtractionConfig],
    source: &str,
) {
    page.custom_extractions.clear();
    if config.is_empty() {
        return;
    }
    let doc = Html::parse_document(html);
    for extraction in config {
        // Validated before the first HTTP request; keep a visible error if that
        // invariant is ever broken rather than panicking while crawling.
        let parsed = match Selector::parse(&extraction.selector) {
            Ok(selector) => selector,
            Err(error) => {
                page.warnings.push(format!(
                    "Custom extraction {} failed: {error:?}",
                    extraction.name
                ));
                continue;
            }
        };
        let values = doc
            .select(&parsed)
            .filter_map(|node| match &extraction.attribute {
                Some(attribute) => node.value().attr(attribute).map(str::to_owned),
                None => Some(clean(&node.text().collect::<String>())),
            })
            .collect();
        page.custom_extractions.push(CustomExtraction {
            name: extraction.name.clone(),
            selector: extraction.selector.clone(),
            attribute: extraction.attribute.clone(),
            values,
            source: source.into(),
        });
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
}
