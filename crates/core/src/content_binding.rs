//! Binding consistency for actual native/rendered observations; no imported-origin attestation.
use crate::{
    CompiledContentCaptureConfig, ContentCaptureDecodeStatus, ContentCaptureInput,
    ContentCaptureResult, ContentCaptureSourceKind, ContentCaptureStatus, PageRecord,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PAGE_CONTENT_CAPTURE_ENGINE: &str = "open-seo-spider/native-content-binding-v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PageContentCaptureStatus {
    Complete,
    Empty,
    Incomplete,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContentCaptureFailure {
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PageContentCapture {
    pub engine: String,
    pub source_kind: ContentCaptureSourceKind,
    pub requested_url: String,
    pub source_document_url: Option<String>,
    pub fetched_at: String,
    pub rendered_at: Option<String>,
    pub observed_generation: Option<u64>,
    pub source_revision: String,
    pub config_sha256: String,
    pub source_html_utf8_sha256: Option<String>,
    pub source_html_utf8_bytes: Option<usize>,
    /// Reqwest-decoded entity bytes before character decoding, not compressed network packets.
    pub encoded_body_sha256: Option<String>,
    pub encoded_body_bytes: Option<usize>,
    pub encoded_body_complete: Option<bool>,
    pub decode_encoding: Option<String>,
    pub status: PageContentCaptureStatus,
    pub result: Option<ContentCaptureResult>,
    pub failure: Option<ContentCaptureFailure>,
}

pub(crate) struct SourceObservation<'a> {
    pub source_kind: ContentCaptureSourceKind,
    pub html: Option<&'a str>,
    pub source_complete: bool,
    pub decode_status: ContentCaptureDecodeStatus,
    pub decode_encoding: Option<String>,
    pub incomplete_reason: Option<String>,
    pub unavailable_reason: Option<String>,
    pub document_url: Option<String>,
    pub rendered_at: Option<String>,
    pub entity: Option<(String, usize, bool)>,
}

pub(crate) fn entity_bytes(body: &[u8], complete: bool) -> (String, usize, bool) {
    (hash(body), body.len(), complete)
}

fn hash(value: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(value.as_ref()))
}
fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn bounded(value: &str) -> String {
    let clean = value
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect::<String>();
    let mut end = clean.len().min(512);
    while !clean.is_char_boundary(end) {
        end -= 1;
    }
    clean[..end].to_owned()
}
fn revision(row: &PageContentCapture) -> Result<String> {
    Ok(hash(serde_json::to_vec(&(
        (
            row.engine.as_str(),
            row.source_kind,
            row.requested_url.as_str(),
            &row.source_document_url,
            row.fetched_at.as_str(),
            &row.rendered_at,
            row.observed_generation,
            row.config_sha256.as_str(),
        ),
        (
            &row.source_html_utf8_sha256,
            row.source_html_utf8_bytes,
            &row.encoded_body_sha256,
            row.encoded_body_bytes,
            row.encoded_body_complete,
            &row.decode_encoding,
            row.status,
            &row.result,
            &row.failure,
        ),
    ))?))
}

pub(crate) fn observe(
    page: &mut PageRecord,
    compiled: &CompiledContentCaptureConfig,
    generation: Option<u64>,
    source: SourceObservation<'_>,
) {
    if !compiled.config().sources.contains(&source.source_kind) {
        return;
    }
    let (entity_hash, entity_bytes, entity_complete) = source
        .entity
        .map_or((None, None, None), |(hash, bytes, complete)| {
            (Some(hash), Some(bytes), Some(complete))
        });
    let mut row = PageContentCapture {
        engine: PAGE_CONTENT_CAPTURE_ENGINE.into(),
        source_kind: source.source_kind,
        requested_url: page.url.clone(),
        source_document_url: source.document_url,
        fetched_at: page.fetched_at.clone(),
        rendered_at: source.rendered_at,
        observed_generation: generation,
        source_revision: String::new(),
        config_sha256: compiled.config_sha256().into(),
        source_html_utf8_sha256: source.html.map(hash),
        source_html_utf8_bytes: source.html.map(str::len),
        encoded_body_sha256: entity_hash,
        encoded_body_bytes: entity_bytes,
        encoded_body_complete: entity_complete,
        decode_encoding: source.decode_encoding,
        status: PageContentCaptureStatus::Error,
        result: None,
        failure: None,
    };
    // Avoid cloning an oversized source just to discover its configured limit.
    let result = if source
        .html
        .is_some_and(|html| html.len() > compiled.config().limits.max_html_utf8_bytes)
    {
        Err(anyhow::anyhow!("HTML input exceeds max_html_utf8_bytes"))
    } else {
        compiled.capture(&ContentCaptureInput {
            source_kind: source.source_kind,
            html: source.html.map(str::to_owned),
            source_complete: source.source_complete,
            decode_status: source.decode_status,
            source_truncation_reason: source.incomplete_reason,
            unavailable_reason: source.unavailable_reason,
            config: compiled.config().clone(),
        })
    };
    match result {
        Ok(result) => {
            row.status = match result.status {
                ContentCaptureStatus::Complete => PageContentCaptureStatus::Complete,
                ContentCaptureStatus::Empty => PageContentCaptureStatus::Empty,
                ContentCaptureStatus::Incomplete => PageContentCaptureStatus::Incomplete,
                ContentCaptureStatus::Unavailable => PageContentCaptureStatus::Unavailable,
            };
            row.result = Some(result);
        }
        Err(error) => {
            let detail = bounded(&error.to_string());
            let code = if detail.contains("max_html_utf8_bytes") {
                "html_input_limit"
            } else if detail.contains("max_output_json_bytes") {
                "output_json_limit"
            } else {
                "capture_failed"
            };
            page.warnings.push(format!(
                "Content capture {:?} failed: {detail}",
                source.source_kind
            ));
            row.failure = Some(ContentCaptureFailure {
                code: code.into(),
                detail,
            });
        }
    }
    // Every value in this tuple is bounded independently by the validated config.
    row.source_revision = revision(&row).expect("Typed capture binding serializes");
    let rows = page.content_captures.get_or_insert_with(Vec::new);
    if let Some(index) = rows
        .iter()
        .position(|saved| saved.source_kind == row.source_kind)
    {
        rows[index] = row;
    } else {
        rows.push(row);
    }
    rows.sort_by_key(|row| {
        compiled
            .config()
            .sources
            .iter()
            .position(|kind| *kind == row.source_kind)
            .unwrap_or(usize::MAX)
    });
}

pub(crate) fn unavailable_sources(
    page: &mut PageRecord,
    compiled: &CompiledContentCaptureConfig,
    generation: Option<u64>,
    reason: &str,
) {
    for kind in &compiled.config().sources {
        observe(
            page,
            compiled,
            generation,
            SourceObservation {
                source_kind: *kind,
                html: None,
                source_complete: false,
                decode_status: ContentCaptureDecodeStatus::Unknown,
                decode_encoding: None,
                incomplete_reason: None,
                unavailable_reason: Some(bounded(reason)),
                document_url: None,
                rendered_at: None,
                entity: None,
            },
        );
    }
}

/// Internal hash/source consistency only. Full HTML is not retained and imported
/// JSON does not become an authenticated network observation through this check.
pub fn validate_page_content_captures(page: &PageRecord) -> Result<()> {
    let Some(rows) = &page.content_captures else {
        return Ok(());
    };
    ensure!(
        !rows.is_empty() && rows.len() <= 2,
        "Captured source list must contain one or two observations"
    );
    let mut seen = std::collections::HashSet::new();
    for row in rows {
        ensure!(
            row.engine == PAGE_CONTENT_CAPTURE_ENGINE,
            "Unknown saved content binding engine"
        );
        ensure!(
            seen.insert(serde_json::to_string(&row.source_kind)?),
            "Duplicate saved capture source"
        );
        ensure!(
            row.requested_url == page.url && row.fetched_at == page.fetched_at,
            "Capture differs from enclosing requested/fetch observation"
        );
        ensure!(
            row.observed_generation != Some(0),
            "Capture generation must be positive when present"
        );
        ensure!(
            row.source_revision == revision(row)?,
            "Capture source revision changed"
        );
        ensure!(
            is_digest(&row.config_sha256)
                && row.source_html_utf8_sha256.as_deref().is_none_or(is_digest)
                && row.encoded_body_sha256.as_deref().is_none_or(is_digest),
            "Invalid capture config digest"
        );
        ensure!(
            row.source_html_utf8_sha256.is_some() == row.source_html_utf8_bytes.is_some(),
            "Capture HTML hash/bytes availability differs"
        );
        ensure!(
            row.encoded_body_sha256.is_some() == row.encoded_body_bytes.is_some()
                && row.encoded_body_bytes.is_some() == row.encoded_body_complete.is_some(),
            "Capture entity byte/hash completeness differs"
        );
        match row.source_kind {
            ContentCaptureSourceKind::RawHttpHtml => {
                ensure!(
                    row.rendered_at.is_none(),
                    "Raw capture has rendered timestamp"
                );
                ensure!(
                    row.source_document_url
                        .as_ref()
                        .is_none_or(|url| url == &page.final_url),
                    "Raw capture document address differs"
                );
                ensure!(
                    row.encoded_body_bytes
                        .is_none_or(|bytes| bytes == page.size_bytes),
                    "Raw captured entity byte count differs"
                );
            }
            ContentCaptureSourceKind::RenderedDomHtml => {
                ensure!(
                    row.encoded_body_sha256.is_none() && row.decode_encoding.is_none(),
                    "Rendered serialization falsely attributed native byte/decoder facts"
                );
                ensure!(
                    row.source_document_url
                        .as_ref()
                        .is_none_or(|url| Some(url) == page.rendered_url.as_ref()),
                    "Rendered capture document address differs"
                );
                ensure!(
                    row.rendered_at
                        .as_ref()
                        .is_none_or(|time| Some(time) == page.rendered_at.as_ref()),
                    "Rendered capture timestamp differs"
                );
            }
        }
        if let Some(result) = &row.result {
            ensure!(
                row.failure.is_none()
                    && result.source_kind == row.source_kind
                    && result.engine == crate::CONTENT_CAPTURE_ENGINE
                    && result.selection_profile == crate::CONTENT_CAPTURE_SELECTION_PROFILE
                    && result.whitespace_profile == crate::CONTENT_CAPTURE_WHITESPACE_PROFILE
                    && result.parser_profile == crate::CONTENT_CAPTURE_PARSER_PROFILE
                    && result.html_hash_representation
                        == crate::CONTENT_CAPTURE_HTML_HASH_REPRESENTATION
                    && result.website_requests == 0,
                "Capture result source/engine differs"
            );
            result.config.validate()?;
            ensure!(
                serde_json::to_vec(result)?.len() <= result.config.limits.max_output_json_bytes,
                "Capture result exceeds configured serialized output bound"
            );
            ensure!(
                result.html_language.is_some() != result.html_language_reason.is_some(),
                "Capture language and unknown-language reason contradict each other"
            );
            if let Some(language) = &result.html_language {
                let mut segments = language.split('-');
                let first = segments.next().unwrap_or("");
                ensure!(
                    language.len() <= 128
                        && (2..=8).contains(&first.len())
                        && first.bytes().all(|byte| byte.is_ascii_alphabetic())
                        && segments.all(|segment| (1..=8).contains(&segment.len())
                            && segment.bytes().all(|byte| byte.is_ascii_alphanumeric())),
                    "Capture language violates its declared syntax profile"
                );
            }
            ensure!(
                result
                    .html_utf8_bytes
                    .is_none_or(|bytes| bytes <= result.config.limits.max_html_utf8_bytes),
                "Captured HTML exceeds configured input bound"
            );
            ensure!(
                result.config.sources.contains(&row.source_kind),
                "Capture source not selected by its config"
            );
            ensure!(
                hash(serde_json::to_vec(&result.config)?) == result.config_sha256
                    && result.config_sha256 == row.config_sha256,
                "Capture config hash differs"
            );
            ensure!(
                result.html_utf8_sha256 == row.source_html_utf8_sha256
                    && result.html_utf8_bytes == row.source_html_utf8_bytes,
                "Captured HTML provenance differs"
            );
            ensure!(
                result.text.as_ref().map(hash) == result.text_sha256,
                "Captured text digest differs"
            );
            let status = match result.status {
                ContentCaptureStatus::Complete => PageContentCaptureStatus::Complete,
                ContentCaptureStatus::Empty => PageContentCaptureStatus::Empty,
                ContentCaptureStatus::Incomplete => PageContentCaptureStatus::Incomplete,
                ContentCaptureStatus::Unavailable => PageContentCaptureStatus::Unavailable,
            };
            ensure!(row.status == status, "Capture result status differs");
            ensure!(
                result.status != ContentCaptureStatus::Incomplete || !result.reasons.is_empty(),
                "Incomplete capture has no incomplete evidence reason"
            );
            if result.status == ContentCaptureStatus::Unavailable {
                ensure!(
                    result.text.is_none()
                        && result.text_sha256.is_none()
                        && result.html_utf8_sha256.is_none()
                        && result.html_utf8_bytes.is_none()
                        && result.evidence.is_none()
                        && !result.source_complete,
                    "Unavailable capture contains supplied HTML/text or complete evidence"
                );
            } else {
                let text = result
                    .text
                    .as_ref()
                    .context("Supplied HTML capture has no selected text")?;
                let evidence = result
                    .evidence
                    .as_ref()
                    .context("Supplied HTML capture has no traversal evidence")?;
                ensure!(
                    result.html_utf8_sha256.is_some()
                        && result.html_utf8_bytes.is_some()
                        && evidence.emitted_utf8_bytes == text.len()
                        && text.len() <= result.config.limits.max_text_utf8_bytes
                        && evidence.observed_normalized_utf8_bytes >= text.len()
                        && evidence.include_match_counts.len()
                            == result.config.include_selectors.len()
                        && evidence.exclude_match_counts.len()
                            == result.config.exclude_selectors.len(),
                    "Capture text/selection bounds differ"
                );
                if !evidence.text_truncated {
                    ensure!(
                        evidence.observed_normalized_utf8_bytes == text.len()
                            && Some(&evidence.observed_normalized_utf8_sha256)
                                == result.text_sha256.as_ref(),
                        "Untruncated capture stream digest differs"
                    );
                }
                ensure!(
                    result.status != ContentCaptureStatus::Complete || !text.is_empty(),
                    "Complete text capture is empty"
                );
                ensure!(
                    result.status != ContentCaptureStatus::Empty || text.is_empty(),
                    "Empty text capture is nonempty"
                );
            }
            if matches!(
                row.status,
                PageContentCaptureStatus::Complete | PageContentCaptureStatus::Empty
            ) {
                ensure!(
                    result.source_complete
                        && result.reasons.is_empty()
                        && result.decode_status == ContentCaptureDecodeStatus::Exact
                        && result
                            .evidence
                            .as_ref()
                            .is_some_and(|e| e.traversal_complete && !e.text_truncated),
                    "Saved complete capture lacks complete source/selection evidence"
                );
                let eligible = match row.source_kind {
                    ContentCaptureSourceKind::RawHttpHtml => {
                        page.is_html()
                            && page.final_status == Some(200)
                            && !page.truncated
                            && page.error.is_none()
                            && !page.blocked_by_robots
                    }
                    ContentCaptureSourceKind::RenderedDomHtml => {
                        page.rendered
                            && page.rendered_status == Some(200)
                            && page.renderer_error.is_none()
                            && page
                                .rendered_headers
                                .iter()
                                .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
                                .map(|(_, value)| value.split(';').next().unwrap_or("").trim())
                                .is_some_and(|mime| {
                                    mime.eq_ignore_ascii_case("text/html")
                                        || mime.eq_ignore_ascii_case("application/xhtml+xml")
                                })
                    }
                };
                ensure!(
                    eligible && row.source_document_url.is_some(),
                    "Complete capture has unsuccessful or unknown source response"
                );
                if row.source_kind == ContentCaptureSourceKind::RawHttpHtml {
                    ensure!(
                        row.encoded_body_complete == Some(true) && row.decode_encoding.is_some(),
                        "Complete raw capture lacks complete entity/decoder facts"
                    );
                } else {
                    ensure!(
                        row.rendered_at.is_some(),
                        "Complete rendered capture lacks render timestamp"
                    );
                }
            }
        } else {
            ensure!(
                row.status == PageContentCaptureStatus::Error
                    && row
                        .failure
                        .as_ref()
                        .is_some_and(|error| !error.code.is_empty()
                            && error.code.len() <= 64
                            && !error.code.chars().any(char::is_control)
                            && !error.detail.is_empty()
                            && !error.detail.chars().any(char::is_control)
                            && error.detail.len() <= 512),
                "Capture error result lacks bounded failure"
            );
        }
    }
    Ok(())
}

/// Native execution/recovery binding to an immutable crawl config. Imported
/// inherited rows may have no captures and remain unobserved; fresh native rows
/// with capture enabled must contain every explicitly configured source.
pub(crate) fn validate_native_content_captures(
    page: &PageRecord,
    config: &crate::CrawlConfig,
    generation: Option<u64>,
    require_observation: bool,
) -> Result<()> {
    validate_page_content_captures(page)?;
    let Some(capture_config) = &config.content_capture else {
        ensure!(
            !require_observation || page.content_captures.is_none(),
            "Unconfigured native capture observations"
        );
        return Ok(());
    };
    let Some(rows) = &page.content_captures else {
        ensure!(
            !require_observation,
            "Configured native capture observations missing"
        );
        return Ok(());
    };
    let expected_hash = hash(serde_json::to_vec(capture_config)?);
    ensure!(
        rows.len() == capture_config.sources.len(),
        "Native capture sources missing"
    );
    for (row, source) in rows.iter().zip(&capture_config.sources) {
        ensure!(
            row.source_kind == *source && row.config_sha256 == expected_hash,
            "Native capture differs from saved configured source/order/hash"
        );
        if let Some(generation) = generation {
            ensure!(
                row.observed_generation == Some(generation),
                "Native capture differs from committed generation"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn captured() -> PageRecord {
        let html = "<html lang='de'><body>Eigener Text</body></html>";
        let mut page = PageRecord {
            url: "https://example.test/requested".into(),
            final_url: "https://example.test/document".into(),
            fetched_at: "2026-10-12T00:00:00Z".into(),
            final_status: Some(200),
            content_type: "text/html".into(),
            size_bytes: html.len(),
            ..Default::default()
        };
        observe(
            &mut page,
            &crate::ContentCaptureConfig::default().compile().unwrap(),
            Some(7),
            SourceObservation {
                source_kind: ContentCaptureSourceKind::RawHttpHtml,
                html: Some(html),
                source_complete: true,
                decode_status: ContentCaptureDecodeStatus::Exact,
                decode_encoding: Some("UTF-8".into()),
                incomplete_reason: None,
                unavailable_reason: None,
                document_url: Some("https://example.test/document".into()),
                rendered_at: None,
                entity: Some(entity_bytes(html.as_bytes(), true)),
            },
        );
        page
    }
    #[test]
    fn hashes_bind_actual_text_source_config_and_page_identity() {
        let page = captured();
        validate_page_content_captures(&page).unwrap();
        for change in 0..11 {
            let mut altered = page.clone();
            let row = &mut altered.content_captures.as_mut().unwrap()[0];
            match change {
                0 => row.result.as_mut().unwrap().text = Some("Unrelated".into()),
                1 => row.source_document_url = Some("https://example.test/wrong".into()),
                2 => row.config_sha256 = hash("different"),
                3 => row.observed_generation = Some(0),
                4 => row.result.as_mut().unwrap().website_requests = 1,
                5 => row.result.as_mut().unwrap().status = ContentCaptureStatus::Empty,
                6 => {
                    row.result
                        .as_mut()
                        .unwrap()
                        .evidence
                        .as_mut()
                        .unwrap()
                        .emitted_utf8_bytes = 0
                }
                7 => row.encoded_body_bytes = Some(1),
                8 => row
                    .result
                    .as_mut()
                    .unwrap()
                    .reasons
                    .push(crate::ContentCaptureReason {
                        code: crate::ContentCaptureReasonCode::DecodeUnknown,
                        detail: None,
                    }),
                9 => {
                    row.result.as_mut().unwrap().html_language_reason =
                        Some(crate::ContentCaptureLanguageReason::UnsupportedSyntax)
                }
                _ => {
                    let result = row.result.as_mut().unwrap();
                    result.html_utf8_bytes = Some(result.config.limits.max_html_utf8_bytes + 1);
                    row.source_html_utf8_bytes = result.html_utf8_bytes;
                }
            }
            // Even recomputing the wrapper revision cannot make inconsistent
            // source/result/page facts internally valid.
            row.source_revision = revision(row).unwrap();
            assert!(
                validate_page_content_captures(&altered).is_err(),
                "mutation {change}"
            );
        }
    }
    #[test]
    fn declared_imports_are_not_native_generation_attestations() {
        let mut page = captured();
        let cfg = crate::CrawlConfig {
            content_capture: Some(crate::ContentCaptureConfig::default()),
            ..Default::default()
        };
        validate_native_content_captures(&page, &cfg, Some(7), true).unwrap();
        assert!(validate_native_content_captures(&page, &cfg, Some(8), true).is_err());
        let row = &mut page.content_captures.as_mut().unwrap()[0];
        row.observed_generation = None;
        row.source_revision = revision(row).unwrap();
        validate_page_content_captures(&page).unwrap();
        assert!(validate_native_content_captures(&page, &cfg, Some(7), true).is_err());
        page.content_captures = None;
        validate_native_content_captures(&page, &cfg, None, false).unwrap();
        assert!(validate_native_content_captures(&page, &cfg, Some(7), true).is_err());
    }
    #[test]
    fn absent_optional_fields_preserve_legacy_json_representation() {
        let config: crate::CrawlConfig =
            serde_json::from_value(serde_json::to_value(crate::CrawlConfig::default()).unwrap())
                .unwrap();
        assert!(
            serde_json::to_value(config)
                .unwrap()
                .get("content_capture")
                .is_none()
        );
        assert!(
            serde_json::to_value(PageRecord::default())
                .unwrap()
                .get("content_captures")
                .is_none()
        );
    }
    #[test]
    fn rendered_complete_capture_requires_actual_html_response_headers() {
        let mut page = captured();
        page.rendered = true;
        page.rendered_status = Some(200);
        page.rendered_url = Some("https://example.test/rendered".into());
        page.rendered_at = Some("2026-10-12T01:00:00Z".into());
        page.rendered_headers
            .insert("content-type".into(), "Text/HTML; charset=utf-8".into());
        let config = crate::ContentCaptureConfig {
            sources: vec![ContentCaptureSourceKind::RenderedDomHtml],
            ..Default::default()
        };
        page.content_captures = None;
        observe(
            &mut page,
            &config.compile().unwrap(),
            Some(7),
            SourceObservation {
                source_kind: ContentCaptureSourceKind::RenderedDomHtml,
                html: Some("<body>Actual DOM</body>"),
                source_complete: true,
                decode_status: ContentCaptureDecodeStatus::Exact,
                decode_encoding: None,
                incomplete_reason: None,
                unavailable_reason: None,
                document_url: Some("https://example.test/rendered".into()),
                rendered_at: Some("2026-10-12T01:00:00Z".into()),
                entity: None,
            },
        );
        validate_page_content_captures(&page).unwrap();
        page.rendered_headers
            .insert("content-type".into(), "application/pdf".into());
        assert!(validate_page_content_captures(&page).is_err());
        page.rendered_headers.clear();
        assert!(validate_page_content_captures(&page).is_err());
    }
}
