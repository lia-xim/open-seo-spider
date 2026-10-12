//! Actual native source inventories and bounded metadata-only retention.
use crate::{
    ContentCaptureDecodeStatus as D, PageRecord, ResourceDiscoveryConfig, ResourceDiscoveryLimits,
    ResourceOccurrence, ResourceReferenceKind as K, ResourceSourceBatch, ResourceSourceKind,
};
use anyhow::{Result, ensure};
use std::collections::BTreeMap;
use url::Url;

pub(crate) fn decode_css(body: &[u8], content_type: &str) -> (String, D, String) {
    let http = content_type.split(';').skip(1).find_map(|part| {
        part.split_once('=')
            .filter(|(name, _)| name.trim().eq_ignore_ascii_case("charset"))
            .map(|(_, value)| value.trim().trim_matches(['\'', '"']))
    });
    let bom = encoding_rs::Encoding::for_bom(body).map(|(encoding, _)| encoding);
    let declared = http.and_then(|value| encoding_rs::Encoding::for_label(value.as_bytes()));
    // CSS Syntax's byte-level @charset signature, never a regex over decoded CSS.
    let css_label = body
        .strip_prefix(b"@charset \"")
        .and_then(|tail| {
            tail.iter()
                .position(|byte| *byte == b'"')
                .filter(|end| tail.get(end + 1) == Some(&b';'))
                .map(|end| &tail[..end])
        })
        .and_then(|label| encoding_rs::Encoding::for_label(label));
    let mut encoding = bom.or(declared).or(css_label).unwrap_or(encoding_rs::UTF_8);
    if bom.is_none() && (encoding == encoding_rs::UTF_16LE || encoding == encoding_rs::UTF_16BE) {
        encoding = encoding_rs::UTF_8;
    }
    let (text, used, errors) = encoding.decode(body);
    let status = if errors {
        D::Lossy
    } else if bom.is_none()
        && ((http.is_some() && declared.is_none())
            || (declared.is_none() && css_label.is_none() && !body.is_ascii()))
    {
        // A parent stylesheet/document encoding could change non-ASCII
        // fallback text. That environment is not durably propagated in v1.
        D::Unknown
    } else {
        D::Exact
    };
    (text.into_owned(), status, used.name().into())
}

pub(crate) fn inventory(
    page: &PageRecord,
    text: Option<&str>,
    decode: D,
    seeds: &[Url],
    subdomains: bool,
    cfg: &ResourceDiscoveryConfig,
    limits: &ResourceDiscoveryLimits,
) -> Result<ResourceSourceBatch> {
    let complete = page.final_status == Some(200)
        && page.error.is_none()
        && !page.truncated
        && !page.blocked_by_robots;
    let revision = crate::resource_policy::digest(format!(
        "native:{}:{}:{}",
        page.url,
        page.fetched_at,
        text.map(crate::resource_policy::digest)
            .unwrap_or_else(|| page.content_hash.clone().unwrap_or_default())
    ));
    let mut batch = if page.is_html()
        && let Some(text) = text
    {
        let mut html =
            crate::resource_html::inventory_html(page, text, seeds, subdomains, cfg, limits)?;
        html.batch.complete &= decode == D::Exact;
        let base = page.document_base_url.as_deref().unwrap_or(&page.final_url);
        for source in html.inline_css {
            append_css(
                &mut html.batch,
                &source.text,
                base,
                source.declaration_list,
                true,
                source.dom_location,
                limits,
            )?;
        }
        html.batch
    } else {
        ResourceSourceBatch {
            engine: crate::RESOURCE_DISCOVERY_ENGINE.into(),
            source_revision: revision,
            requested_url: page.url.clone(),
            document_url: page.final_url.clone(),
            source_kind: if page.media_type().eq_ignore_ascii_case("text/css") {
                ResourceSourceKind::NativeCss
            } else if page.pdf.is_some() {
                ResourceSourceKind::PdfAnnotation
            } else {
                ResourceSourceKind::HttpLinkHeader
            },
            complete,
            diagnostics: vec![],
            occurrences: vec![],
        }
    };
    if page.media_type().eq_ignore_ascii_case("text/css") && cfg.css.stylesheets {
        batch.complete &= decode == D::Exact && text.is_some();
        if let Some(css) = text {
            append_css(&mut batch, css, &page.final_url, false, false, None, limits)?;
        }
    }
    if let Some(pdf) = &page.pdf {
        batch.complete &= pdf.status == crate::ResourceAnalysisStatus::Complete;
        for link in &pdf.annotation_links {
            append(
                &mut batch,
                link.raw_uri.clone(),
                link.url.clone(),
                K::PdfAnnotation,
                "pdf_uri_annotation",
                None,
                BTreeMap::from([("page".into(), link.page.to_string())]),
                limits,
            )?;
        }
    }
    // Keep HTTP declarations distinct from identical HTML declarations.
    for target in page
        .link_relationships
        .iter()
        .filter(|row| row.source == "http_header" && row.applies_to_document)
    {
        for rel in &target.rel {
            let kind = match rel.as_str() {
                "canonical" => Some(K::Canonical),
                "next" | "prev" => Some(K::Pagination),
                "amphtml" => Some(K::Amp),
                "alternate" if !target.hreflang.is_empty() => Some(K::Hreflang),
                _ => None,
            };
            if let Some(kind) = kind {
                append(
                    &mut batch,
                    target.url.clone(),
                    Some(target.url.clone()),
                    kind,
                    "resolved_http_link_header_target",
                    None,
                    BTreeMap::from([
                        ("header_rel".into(), rel.clone()),
                        (
                            "original_link_fields".into(),
                            serde_json::to_string(&page.header_link_values)?,
                        ),
                    ]),
                    limits,
                )?;
                batch.occurrences.last_mut().unwrap().source_complete = complete;
                if cfg.contract.is_some() {
                    let row = batch.occurrences.last_mut().unwrap();
                    row.attributes.insert("rel".into(), target.rel.join(" "));
                    row.nofollow = target
                        .rel
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case("nofollow"));
                }
            }
        }
    }
    if !complete {
        batch.complete = false;
        batch
            .diagnostics
            .push("response_incomplete_or_non200".into());
    }
    if matches!(decode, D::Lossy | D::Unknown)
        && (page.is_html() || page.media_type().eq_ignore_ascii_case("text/css"))
    {
        batch.complete = false;
        batch.diagnostics.push("character_decode_not_exact".into());
    }
    if !complete
        || ((page.is_html() || page.media_type().eq_ignore_ascii_case("text/css"))
            && decode != D::Exact)
    {
        for row in &mut batch.occurrences {
            row.source_complete = false;
        }
    }
    limits.check_batch(&batch)?;
    Ok(batch)
}

#[allow(clippy::too_many_arguments)]
fn append(
    batch: &mut ResourceSourceBatch,
    raw: String,
    resolved: Option<String>,
    kind: K,
    basis: &str,
    path: Option<String>,
    attributes: BTreeMap<String, String>,
    limits: &ResourceDiscoveryLimits,
) -> Result<()> {
    let ordinal = batch.occurrences.len();
    ensure!(
        ordinal < limits.max_source_candidates,
        "Resource source candidate budget exceeded; source remains pending"
    );
    batch.occurrences.push(ResourceOccurrence {
        source_complete: batch.complete,
        source_unit_revision: batch.source_revision.clone(),
        occurrence_id: crate::resource_policy::occurrence_id(&batch.source_revision, ordinal),
        source_revision: batch.source_revision.clone(),
        source_ordinal: ordinal,
        requested_source_url: batch.requested_url.clone(),
        source_document_url: batch.document_url.clone(),
        raw_value: raw,
        decoded_value: None,
        observation_error: resolved
            .is_none()
            .then(|| "invalid_or_non_http_reference".into()),
        resolved_url: resolved,
        reference_kind: kind,
        classification_basis: basis.into(),
        declared_mime: None,
        dom_location: path,
        byte_start: None,
        byte_end: None,
        attributes,
        nofollow: false,
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_css(
    batch: &mut ResourceSourceBatch,
    css: &str,
    base: &str,
    declaration_list: bool,
    inline: bool,
    path: Option<String>,
    limits: &ResourceDiscoveryLimits,
) -> Result<()> {
    let inventory = crate::discover_css(
        &crate::CssSourceInput {
            css_text: css.into(),
            base_url: base.into(),
            source_url: batch.document_url.clone(),
            source_revision: batch.source_revision.clone(),
            inline,
            syntax: if declaration_list {
                crate::CssSourceSyntax::DeclarationList
            } else {
                crate::CssSourceSyntax::Stylesheet
            },
        },
        &crate::CssInventoryLimits {
            max_utf8_bytes: limits.max_css_utf8_bytes,
            max_tokens: limits.max_css_tokens,
            max_nesting: limits.max_css_nesting,
            max_candidates: limits.max_source_candidates,
            max_diagnostics: limits.max_diagnostics,
            max_url_utf8_bytes: limits.max_occurrence_utf8_bytes,
            max_retained_utf8_bytes: limits.max_source_batch_bytes,
        },
    )?;
    batch.complete &= inventory.complete;
    for diagnostic in inventory.diagnostics {
        batch.diagnostics.push(format!("css:{}", diagnostic.code));
    }
    for item in inventory.dependencies {
        let kind = match item.category_hint {
            crate::CssDependencyCategory::Stylesheet => K::Css,
            crate::CssDependencyCategory::Image => K::Image,
            _ => K::Other,
        };
        let mut attributes = BTreeMap::from([
            (
                "css_source".into(),
                if inline { "inline" } else { "external" }.into(),
            ),
            (
                "css_syntax".into(),
                if declaration_list {
                    "declaration_list"
                } else {
                    "stylesheet"
                }
                .into(),
            ),
        ]);
        if let Some(property) = item.property {
            attributes.insert("property".into(), property);
        }
        if let Some(rule) = item.at_rule {
            attributes.insert("at_rule".into(), rule);
        }
        if let Some(set) = item.image_set {
            attributes.insert("image_set".into(), serde_json::to_string(&set)?);
        }
        append(
            batch,
            item.raw_token,
            item.resolved_url,
            kind,
            "css_parser_declared_dependency",
            path.clone(),
            attributes,
            limits,
        )?;
        let row = batch.occurrences.last_mut().unwrap();
        row.source_complete = inventory.complete;
        row.source_unit_revision = crate::resource_policy::digest(format!(
            "{}:{}:{:?}",
            inventory.source_revision, inventory.source_utf8_sha256, row.dom_location
        ));
        row.decoded_value = Some(item.decoded_url);
        row.byte_start = Some(item.byte_start);
        row.byte_end = Some(item.byte_end);
        row.observation_error = item.resolution_error;
        if item.deferred {
            row.attributes.insert("deferred".into(), "true".into());
        }
    }
    Ok(())
}

pub(crate) fn private_projection(
    page: &PageRecord,
    config: &crate::CrawlConfig,
    generation: u64,
) -> Result<PageRecord> {
    let mut projection = page.clone();
    projection.screenshot_base64 = None;
    projection.network_requests.clear();
    projection.console_errors.clear();
    projection.accessibility = None;
    projection.custom_javascript_results.clear();
    projection.browser_cookies.clear();
    // Arbitrary custom text/HTML extraction values can contain an entire body.
    projection.custom_extractions.clear();
    if let Some(pdf) = &mut projection.pdf {
        pdf.text_preview = None;
        pdf.text_pages.clear();
        pdf.text_complete = false;
    }
    if let Some(capture) = config.content_capture.as_ref() {
        let compiled = capture.compile()?;
        projection.content_captures = None;
        crate::content_binding::unavailable_sources(
            &mut projection,
            &compiled,
            Some(generation),
            "response_source_not_retained",
        );
    }
    if let Some(facts) = &mut projection.resource_response {
        facts.retained_response = false;
        facts.entity_body_retained = false;
        facts.metadata_projection_complete = false;
        facts.unretained_fields = vec![
            "entity_body".into(),
            "custom_extractions".into(),
            "pdf_text".into(),
            "content_capture_text".into(),
            "screenshot".into(),
            "browser_network_console_accessibility".into(),
            "custom_javascript_results".into(),
            "browser_cookies".into(),
        ];
    }
    Ok(projection)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn css_byte_decoder_prioritizes_bom_then_protocol_then_exact_charset_signature() {
        let (text, status, encoding) = decode_css(
            b"\xef\xbb\xbfbody{content:'\xc3\xa9'}",
            "text/css;charset=windows-1252",
        );
        assert!(text.contains('é'));
        assert_eq!(status, D::Exact);
        assert_eq!(encoding, "UTF-8");
        let mut utf16 = vec![0xff, 0xfe];
        for word in "body{content:'é'}".encode_utf16() {
            utf16.extend(word.to_le_bytes());
        }
        let (text, status, encoding) = decode_css(&utf16, "text/css;charset=utf-8");
        assert!(text.contains('é'));
        assert_eq!(status, D::Exact);
        assert_eq!(encoding, "UTF-16LE");
        let (_, status, encoding) = decode_css(
            b"@charset \"windows-1252\";body{content:'\xe9'}",
            "text/css",
        );
        assert_eq!(status, D::Exact);
        assert_eq!(encoding, "windows-1252");
        assert_eq!(
            decode_css(
                b" @charset \"windows-1252\";body{content:'\xe9'}",
                "text/css"
            )
            .1,
            D::Lossy
        );
        assert_eq!(
            decode_css(b"body{}", "text/css;charset=unknown-fixture").1,
            D::Unknown
        );
        assert_eq!(
            decode_css(b"body{content:'\xe9'}", "text/css;charset=utf-8").1,
            D::Lossy
        );
        assert_eq!(
            decode_css("body{background:url('é.png')}".as_bytes(), "text/css").1,
            D::Unknown
        );
    }
    #[test]
    fn exact_css_limits_and_renderer_are_validated_before_native_requests() {
        let base = crate::CrawlConfig {
            seed_urls: vec!["http://127.0.0.1:9/".into()],
            resource_discovery: Some(ResourceDiscoveryConfig::default()),
            ..Default::default()
        };
        let mut renderer = base.clone();
        renderer.renderer = Some(crate::RendererConfig::default());
        assert!(crate::Crawler::new(renderer).is_err());
        for limits in [
            ResourceDiscoveryLimits {
                max_css_utf8_bytes: 64 * 1024 * 1024 + 1,
                ..Default::default()
            },
            ResourceDiscoveryLimits {
                max_css_tokens: 8_000_001,
                ..Default::default()
            },
            ResourceDiscoveryLimits {
                max_css_nesting: 129,
                ..Default::default()
            },
            ResourceDiscoveryLimits {
                max_diagnostics: 10001,
                ..Default::default()
            },
            ResourceDiscoveryLimits {
                max_occurrence_utf8_bytes: 65537,
                ..Default::default()
            },
        ] {
            let mut config = base.clone();
            config.resource_discovery_limits = Some(limits);
            assert!(crate::Crawler::new(config).is_err());
        }
        let value = serde_json::to_value(crate::CrawlConfig::default()).unwrap();
        assert!(value.get("resource_discovery").is_none());
        assert!(value.get("resource_discovery_limits").is_none());
        assert!(
            serde_json::to_value(PageRecord::default())
                .unwrap()
                .get("resource_response")
                .is_none()
        );
    }
}
