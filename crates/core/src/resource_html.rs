//! Actual repaired-HTML declarations, separate from browser loading/selection.
use crate::resource_policy::{
    self as rp, ResourceDiscoveryConfig, ResourceDiscoveryLimits, ResourceOccurrence,
    ResourceReferenceKind as K, ResourceSourceBatch, ResourceSourceKind,
};
use anyhow::{Result, ensure};
use scraper::{ElementRef, Html};
use std::collections::BTreeMap;
use url::Url;

type HtmlReference = (String, K, String, Option<String>, Option<String>);

pub(crate) struct InlineCssSource {
    pub text: String,
    pub declaration_list: bool,
    pub dom_location: Option<String>,
}
pub(crate) struct HtmlResourceInventory {
    pub batch: ResourceSourceBatch,
    pub inline_css: Vec<InlineCssSource>,
}

fn embedded_kind(node: ElementRef<'_>, raw: &str) -> (K, String) {
    let declared = node.value().attr("type").map(|value| {
        value
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
    });
    if let Some(mime) = declared {
        let kind = if mime.starts_with("image/") {
            Some(K::Image)
        } else if mime.starts_with("video/") || mime.starts_with("audio/") {
            Some(K::Media)
        } else if mime == "application/pdf" {
            Some(K::Pdf)
        } else if mime == "application/x-shockwave-flash" {
            Some(K::Swf)
        } else {
            None
        };
        if let Some(kind) = kind {
            return (kind, "declared_mime_hint".into());
        }
    }
    let path = raw
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let kind = if path.ends_with(".pdf") {
        K::Pdf
    } else if path.ends_with(".swf") {
        K::Swf
    } else if [
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".avif", ".svg", ".bmp", ".ico",
    ]
    .iter()
    .any(|ext| path.ends_with(ext))
    {
        K::Image
    } else if [".mp4", ".webm", ".mp3", ".wav", ".ogg", ".m4a"]
        .iter()
        .any(|ext| path.ends_with(ext))
    {
        K::Media
    } else {
        K::Other
    };
    (kind, "path_hint_not_response_mime".into())
}

pub(crate) fn inventory_html(
    page: &crate::PageRecord,
    html: &str,
    seeds: &[Url],
    subdomains: bool,
    config: &ResourceDiscoveryConfig,
    limits: &ResourceDiscoveryLimits,
) -> Result<HtmlResourceInventory> {
    limits.validate()?;
    let document = Url::parse(&page.final_url)?;
    let base = page
        .document_base_url
        .as_deref()
        .and_then(|value| Url::parse(value).ok())
        .unwrap_or_else(|| document.clone());
    let doc = Html::parse_document(html);
    let dom = crate::dom_facts::DomContext::new(&doc, &[]);
    let revision = rp::digest(format!(
        "html:{}:{}:{}",
        page.url,
        page.fetched_at,
        rp::digest(html)
    ));
    let mut batch = ResourceSourceBatch {
        engine: rp::RESOURCE_DISCOVERY_ENGINE.into(),
        source_revision: revision.clone(),
        requested_url: page.url.clone(),
        document_url: page.final_url.clone(),
        source_kind: ResourceSourceKind::NativeHtml,
        complete: page.final_status == Some(200)
            && page.error.is_none()
            && !page.truncated
            && !page.blocked_by_robots,
        diagnostics: Vec::new(),
        occurrences: Vec::new(),
    };
    let mut inline_css = Vec::new();
    let mut retained = 0usize;
    for node in doc
        .root_element()
        .descendants()
        .filter_map(ElementRef::wrap)
    {
        let name = node.value().name();
        if node.ancestors().any(|parent| {
            parent
                .value()
                .as_element()
                .is_some_and(|element| element.name() == "template")
        }) {
            continue;
        }
        let mut link = crate::LinkRecord::default();
        dom.enrich_link(&mut link, node, "", &base);
        let location = link.dom_location.and_then(|location| location.path);
        if config.css.style_attributes
            && let Some(text) = node.value().attr("style")
        {
            retained = retained.saturating_add(text.len());
            ensure!(
                retained <= limits.max_source_batch_bytes,
                "Inline CSS source byte budget exceeded; source remains pending"
            );
            inline_css.push(InlineCssSource {
                text: text.into(),
                declaration_list: true,
                dom_location: location.clone(),
            });
        }
        if name == "style" && config.css.style_elements {
            let text = node.text().collect::<String>();
            retained = retained.saturating_add(text.len());
            ensure!(
                retained <= limits.max_source_batch_bytes,
                "Inline CSS source byte budget exceeded; source remains pending"
            );
            inline_css.push(InlineCssSource {
                text,
                declaration_list: false,
                dom_location: location.clone(),
            });
        }
        let mut references: Vec<HtmlReference> = Vec::new();
        let mut add_attr = |attr: &str, kind: K| {
            if let Some(raw) = node.value().attr(attr) {
                references.push((
                    raw.into(),
                    kind,
                    "html_attribute_context".into(),
                    None,
                    None,
                ));
            }
        };
        match name {
            "a" | "area" => {
                if let Some(raw) = node.value().attr("href") {
                    let internal = crate::normalize_url(raw, Some(&base))
                        .is_some_and(|url| crate::urls::same_scope(&url, seeds, subdomains));
                    references.push((
                        raw.into(),
                        if internal {
                            K::InternalAnchor
                        } else {
                            K::ExternalAnchor
                        },
                        "anchor_context".into(),
                        None,
                        None,
                    ));
                }
            }
            "img" => add_attr("src", K::Image),
            "script" => add_attr("src", K::JavaScript),
            "video" => {
                add_attr("src", K::Media);
                add_attr("poster", K::Image);
            }
            "audio" => add_attr("src", K::Media),
            "track" => add_attr("src", K::Other),
            "iframe" => add_attr("src", K::Iframe),
            "object" | "embed" => {
                let attribute = if name == "object" { "data" } else { "src" };
                if let Some(raw) = node.value().attr(attribute) {
                    let (kind, basis) = embedded_kind(node, raw);
                    references.push((raw.into(), kind, basis, None, None));
                }
            }
            "source" => {
                if let Some(raw) = node.value().attr("src") {
                    let parent = node
                        .parent()
                        .and_then(ElementRef::wrap)
                        .map(|parent| parent.value().name().to_owned());
                    let media = parent
                        .as_deref()
                        .is_some_and(|name| matches!(name, "audio" | "video"));
                    references.push((
                        raw.into(),
                        if media { K::Media } else { K::Other },
                        "source_parent_context".into(),
                        None,
                        (!media).then(|| "source_src_without_media_parent".into()),
                    ));
                }
            }
            "link" => {
                if let Some(raw) = node.value().attr("href") {
                    let rel = node.value().attr("rel").unwrap_or("");
                    for word in rel.split_ascii_whitespace() {
                        let kind = match word.to_ascii_lowercase().as_str() {
                            "stylesheet" => Some(K::Css),
                            "canonical" => Some(K::Canonical),
                            "next" | "prev" => Some(K::Pagination),
                            "amphtml" => Some(K::Amp),
                            "alternate" => {
                                if node.value().attr("hreflang").is_some() {
                                    Some(K::Hreflang)
                                } else if node.value().attr("media").is_some() {
                                    Some(K::MobileAlternate)
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        };
                        if let Some(kind) = kind {
                            references.push((raw.into(), kind, "html_link_rel".into(), None, None));
                        }
                    }
                }
            }
            "meta" => {
                if node
                    .value()
                    .attr("http-equiv")
                    .is_some_and(|value| value.eq_ignore_ascii_case("refresh"))
                    && let Some(content) = node.value().attr("content")
                {
                    let reference = content
                        .split_once(';')
                        .and_then(|(_, target)| target.trim().split_once('='))
                        .filter(|(key, _)| key.trim().eq_ignore_ascii_case("url"))
                        .map(|(_, value)| value.trim().trim_matches(['\'', '"']).to_owned());
                    references.push((
                        content.into(),
                        K::Refresh,
                        "meta_refresh_declaration".into(),
                        reference.clone(),
                        reference
                            .is_none()
                            .then(|| "refresh_url_not_observed".into()),
                    ));
                }
            }
            _ => {}
        }
        let source_picture = name == "source"
            && node
                .parent()
                .and_then(ElementRef::wrap)
                .is_some_and(|parent| parent.value().name() == "picture");
        if (name == "img" || source_picture)
            && let Some(srcset) = node.value().attr("srcset")
        {
            let candidates =
                crate::srcset::parse_srcset_bounded(srcset, limits.max_source_candidates + 1);
            ensure!(
                candidates.len() <= limits.max_source_candidates,
                "Srcset candidate budget exceeded; source remains pending"
            );
            for candidate in candidates {
                references.push((
                    candidate.raw_url.clone(),
                    K::Image,
                    "html_srcset_candidate".into(),
                    None,
                    (!candidate.eligible).then(|| {
                        format!(
                            "invalid_srcset_descriptor:{}",
                            candidate.error.as_deref().unwrap_or("unknown")
                        )
                    }),
                ));
            }
        }
        for (raw, kind, basis, decoded, error) in references {
            let ordinal = batch.occurrences.len();
            ensure!(
                ordinal < limits.max_source_candidates,
                "Resource candidate budget exceeded; source remains pending"
            );
            let value = decoded.as_deref().unwrap_or(&raw);
            let local = value.trim().starts_with('#') && !kind.is_navigation();
            let resolved = (!value.trim().is_empty() && !local)
                .then(|| crate::normalize_url(value, Some(&base)))
                .flatten();
            let error = error
                .or_else(|| local.then(|| "local_fragment".into()))
                .or_else(|| {
                    resolved
                        .is_none()
                        .then(|| "invalid_empty_or_non_http_reference".into())
                });
            let attributes = node
                .value()
                .attrs()
                .filter(|(name, _)| {
                    matches!(
                        *name,
                        "srcset"
                            | "type"
                            | "media"
                            | "sizes"
                            | "alt"
                            | "width"
                            | "height"
                            | "rel"
                            | "hreflang"
                            | "kind"
                            | "srclang"
                    )
                })
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect::<BTreeMap<_, _>>();
            let row = ResourceOccurrence {
                source_complete: batch.complete,
                source_unit_revision: revision.clone(),
                occurrence_id: rp::occurrence_id(&revision, ordinal),
                source_revision: revision.clone(),
                source_ordinal: ordinal,
                requested_source_url: page.url.clone(),
                source_document_url: page.final_url.clone(),
                raw_value: raw,
                decoded_value: decoded,
                resolved_url: resolved.map(|url| url.to_string()),
                reference_kind: kind,
                classification_basis: basis,
                declared_mime: node.value().attr("type").map(str::to_owned),
                dom_location: location.clone(),
                byte_start: None,
                byte_end: None,
                attributes,
                nofollow: node.value().attr("rel").is_some_and(|value| {
                    value
                        .split_ascii_whitespace()
                        .any(|word| word.eq_ignore_ascii_case("nofollow"))
                }),
                observation_error: error,
            };
            retained = retained.saturating_add(serde_json::to_vec(&row)?.len());
            ensure!(
                retained <= limits.max_source_batch_bytes,
                "Resource source byte budget exceeded; source remains pending"
            );
            batch.occurrences.push(row);
        }
    }
    limits.check_batch(&batch)?;
    Ok(HtmlResourceInventory { batch, inline_css })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page() -> crate::PageRecord {
        crate::PageRecord {
            url: "https://example.test/start".into(),
            final_url: "https://example.test/path/doc".into(),
            document_base_url: Some("https://example.test/assets/".into()),
            final_status: Some(200),
            fetched_at: "own_fixture".into(),
            ..Default::default()
        }
    }
    #[test]
    fn observed_parent_context_poster_embed_and_non_http_are_separate() {
        let input = r#"<img src='one.png' srcset='data:image/png,AAA 1x, two.png 2x' alt='Own'><picture><source src='invalid.mp4' srcset='three.png 3x'></picture><video src='movie' poster='poster.png'><source src='movie2'></video><source src='orphan'><object data='report' type='application/pdf'></object><embed src='run.swf'><a href='one.png'>independent</a><iframe src='child' srcdoc='own'></iframe>"#;
        let inventory = inventory_html(
            &page(),
            input,
            &[Url::parse("https://example.test/").unwrap()],
            false,
            &ResourceDiscoveryConfig::default(),
            &ResourceDiscoveryLimits::default(),
        )
        .unwrap();
        let rows = inventory.batch.occurrences;
        assert_eq!(
            rows.iter()
                .find(|row| row.raw_value == "poster.png")
                .unwrap()
                .reference_kind,
            K::Image
        );
        assert!(
            rows.iter()
                .find(|row| row.raw_value == "invalid.mp4")
                .unwrap()
                .observation_error
                .is_some()
        );
        assert!(
            rows.iter()
                .find(|row| row.raw_value == "data:image/png,AAA")
                .unwrap()
                .resolved_url
                .is_none()
        );
        assert_eq!(
            rows.iter()
                .find(|row| row.raw_value == "report")
                .unwrap()
                .reference_kind,
            K::Pdf
        );
        assert_eq!(
            rows.iter()
                .find(|row| row.raw_value == "run.swf")
                .unwrap()
                .reference_kind,
            K::Swf
        );
        assert_eq!(
            rows.iter().filter(|row| row.raw_value == "one.png").count(),
            2
        );
        assert!(rows.iter().all(|row| row.dom_location.is_some()));
        assert!(
            rows.iter()
                .find(|row| row.raw_value == "poster.png")
                .unwrap()
                .resolved_url
                .as_ref()
                .unwrap()
                .ends_with("/assets/poster.png")
        );
    }
    #[test]
    fn relationship_observations_ignore_fetch_switch_and_template() {
        let input = "<link rel='canonical alternate' hreflang='de' href='/alt'><link rel='next' href='/p2'><meta http-equiv='refresh' content='0; URL=\"/next\"'><template><img src='hidden.png'></template>";
        let inventory = inventory_html(
            &page(),
            input,
            &[],
            false,
            &ResourceDiscoveryConfig::default(),
            &ResourceDiscoveryLimits::default(),
        )
        .unwrap();
        assert_eq!(inventory.batch.occurrences.len(), 4);
        assert_eq!(
            inventory.batch.occurrences[3].resolved_url.as_deref(),
            Some("https://example.test/next")
        );
        assert!(
            inventory
                .batch
                .occurrences
                .iter()
                .any(|row| row.reference_kind == K::Hreflang)
        );
    }
    #[test]
    fn bounded_source_fails_before_retaining_any_partial_batch() {
        let limits = ResourceDiscoveryLimits {
            max_source_candidates: 1,
            ..Default::default()
        };
        assert!(
            inventory_html(
                &page(),
                "<img src='a'><img src='b'>",
                &[],
                false,
                &ResourceDiscoveryConfig::default(),
                &limits
            )
            .is_err()
        );
        let inventory = inventory_html(
            &page(),
            "<div style='background:url(a)'></div><style>@import 'b';</style>",
            &[],
            false,
            &ResourceDiscoveryConfig::default(),
            &ResourceDiscoveryLimits::default(),
        )
        .unwrap();
        assert_eq!(inventory.inline_css.len(), 2);
        assert!(inventory.inline_css[0].declaration_list);
        assert!(!inventory.inline_css[1].declaration_list);
    }
}
