use crate::{CrawlReport, PageChange};
use std::collections::{BTreeMap, BTreeSet};

pub fn export_pages_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "URL",
        "Final URL",
        "Status",
        "Final Status",
        "Content Type",
        "Title",
        "Description",
        "H1",
        "H2",
        "Canonical",
        "Indexable",
        "Indexability",
        "Depth",
        "Response ms",
        "Bytes",
        "Words",
        "Outlinks",
        "Images",
        "Error",
        "Rendered",
        "Raw Title",
        "Rendered URL",
        "Render Error",
        "Browser Requests",
        "Console Errors",
    ])?;
    for page in &report.pages {
        writer.write_record([
            page.url.clone(),
            page.final_url.clone(),
            page.status
                .map(|status| status.to_string())
                .unwrap_or_default(),
            page.final_status
                .map(|status| status.to_string())
                .unwrap_or_default(),
            page.content_type.clone(),
            page.title().into(),
            page.description().into(),
            page.h1.join(" | "),
            page.h2.join(" | "),
            page.canonical.join(" | "),
            page.indexable.to_string(),
            page.indexability_reasons.join(" | "),
            page.depth.to_string(),
            page.response_ms.to_string(),
            page.size_bytes.to_string(),
            page.word_count.to_string(),
            page.links.len().to_string(),
            page.images.len().to_string(),
            page.error.clone().unwrap_or_default(),
            page.rendered.to_string(),
            page.raw_metadata
                .as_ref()
                .and_then(|raw| raw.titles.first())
                .cloned()
                .unwrap_or_default(),
            page.rendered_url.clone().unwrap_or_default(),
            page.renderer_error.clone().unwrap_or_default(),
            page.network_requests.len().to_string(),
            page.console_errors.len().to_string(),
        ])?;
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

pub fn export_issues_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(["Severity", "Code", "URL", "Message", "Related URLs"])?;
    for issue in &report.issues {
        writer.write_record([
            format!("{:?}", issue.severity),
            issue.code.clone(),
            issue.url.clone(),
            issue.message.clone(),
            issue.related_urls.join(" | "),
        ])?;
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

pub fn export_links_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "Source",
        "Target",
        "Anchor Text",
        "Kind",
        "Internal",
        "Rel",
        "Nofollow",
    ])?;
    for page in &report.pages {
        for link in &page.links {
            writer.write_record([
                page.url.clone(),
                link.url.clone(),
                link.text.clone(),
                link.kind.clone(),
                link.internal.to_string(),
                link.rel.clone(),
                link.nofollow.to_string(),
            ])?;
        }
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

/// One result per row preserves values containing commas, quotes or delimiters.
pub fn export_custom_extractions_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "URL",
        "Name",
        "Selector",
        "Attribute",
        "Source",
        "Match Count",
        "Match Index",
        "Value",
    ])?;
    for page in &report.pages {
        for extraction in &page.custom_extractions {
            let rows = if extraction.values.is_empty() {
                vec![(String::new(), String::new())]
            } else {
                extraction
                    .values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| ((index + 1).to_string(), value.clone()))
                    .collect()
            };
            for (index, value) in rows {
                writer.write_record([
                    page.url.clone(),
                    extraction.name.clone(),
                    extraction.selector.clone(),
                    extraction.attribute.clone().unwrap_or_default(),
                    extraction.source.clone(),
                    extraction.values.len().to_string(),
                    index,
                    value,
                ])?;
            }
        }
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

pub fn export_sitemap_xml(report: &CrawlReport) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for page in report
        .pages
        .iter()
        .filter(|page| page.internal && page.indexable)
    {
        xml.push_str(&format!(
            "  <url><loc>{}</loc></url>\n",
            quick_xml::escape::escape(&page.url)
        ));
    }
    xml.push_str("</urlset>\n");
    xml
}

pub fn compare_crawls(before: &CrawlReport, after: &CrawlReport) -> Vec<PageChange> {
    let old: BTreeMap<_, _> = before.pages.iter().map(|page| (&page.url, page)).collect();
    let new: BTreeMap<_, _> = after.pages.iter().map(|page| (&page.url, page)).collect();
    let keys: BTreeSet<_> = old.keys().chain(new.keys()).copied().collect();
    let mut changes = Vec::new();
    for url in keys {
        match (old.get(url), new.get(url)) {
            (None, Some(_)) => changes.push(PageChange {
                url: url.clone(),
                kind: "added".into(),
                fields: vec![],
            }),
            (Some(_), None) => changes.push(PageChange {
                url: url.clone(),
                kind: "removed".into(),
                fields: vec![],
            }),
            (Some(a), Some(b)) => {
                let mut fields = Vec::new();
                macro_rules! changed { ($($field:ident),+) => { $(if a.$field != b.$field { fields.push(stringify!($field).to_owned()); })+ }; }
                changed!(
                    status,
                    final_status,
                    final_url,
                    titles,
                    meta_descriptions,
                    h1,
                    h2,
                    canonical,
                    meta_robots,
                    x_robots_tag,
                    indexable,
                    content_hash,
                    word_count,
                    content_type,
                    error,
                    blocked_by_robots,
                    rendered,
                    renderer_error,
                    rendered_url,
                    rendered_status
                );
                if !fields.is_empty() {
                    changes.push(PageChange {
                        url: url.clone(),
                        kind: "changed".into(),
                        fields,
                    });
                }
            }
            _ => {}
        }
    }
    changes
}
