//! Bind live search observations without retaining or silently substituting HTML.
use crate::{
    CompiledCustomSearchConfig, ContentCaptureDecodeStatus, ContentCaptureSourceKind,
    CustomSearchInput, CustomSearchSource, PageContentCaptureStatus, PageRecord,
};
use sha2::{Digest, Sha256};

const SOURCES: [CustomSearchSource; 4] = [
    CustomSearchSource::RawHtml,
    CustomSearchSource::RenderedHtml,
    CustomSearchSource::RawSelectedText,
    CustomSearchSource::RenderedSelectedText,
];

pub(crate) fn initialize(
    page: &mut PageRecord,
    compiled: &CompiledCustomSearchConfig,
    generation: Option<u64>,
) {
    page.custom_search = Some(
        SOURCES
            .into_iter()
            .flat_map(|source| {
                compiled.observe(
                    source,
                    CustomSearchInput {
                        observed_generation: generation,
                        reason: Some("source_not_observed"),
                        ..Default::default()
                    },
                )
            })
            .collect(),
    );
    seal(page, compiled);
}

fn seal(page: &mut PageRecord, compiled: &CompiledCustomSearchConfig) {
    let Some(results) = page.custom_search.as_mut() else {
        return;
    };
    results.sort_by_key(|row| {
        (
            compiled
                .config()
                .rules
                .iter()
                .position(|rule| rule.name == row.rule_name),
            SOURCES.iter().position(|source| *source == row.source),
        )
    });
    if let Err(error) = compiled
        .enforce_output_budget(results)
        .and_then(|()| compiled.validate_results(results))
    {
        // A failure never preserves a usable-looking partial success. The
        // preflight guarantees bounded metadata for this unavailable fallback.
        *results = SOURCES
            .into_iter()
            .flat_map(|source| {
                compiled.observe(
                    source,
                    CustomSearchInput {
                        reason: Some("output_budget_or_invalid_observation"),
                        ..Default::default()
                    },
                )
            })
            .collect();
        page.warnings.push(format!(
            "Custom search observation unavailable: {}",
            error.to_string().chars().take(256).collect::<String>()
        ));
    }
}

pub(crate) fn observe(
    page: &mut PageRecord,
    compiled: &CompiledCustomSearchConfig,
    source: CustomSearchSource,
    input: CustomSearchInput<'_>,
) {
    let observations = compiled.observe(source, input);
    replace(page, compiled, source, observations);
}

fn replace(
    page: &mut PageRecord,
    compiled: &CompiledCustomSearchConfig,
    source: CustomSearchSource,
    observations: Vec<crate::CustomSearchResult>,
) {
    if page.custom_search.is_none() {
        initialize(page, compiled, None);
    }
    let rows = page.custom_search.as_mut().expect("Initialized above");
    rows.retain(|row| row.source != source);
    rows.extend(observations);
    seal(page, compiled);
}

/// Revision of the observed decoded UTF-8 input, not of HTTP wire bytes.
pub(crate) fn html_revision(
    page: &PageRecord,
    source: CustomSearchSource,
    text: &str,
    document_url: &str,
    generation: Option<u64>,
) -> String {
    let input_sha256 = format!("{:x}", Sha256::digest(text.as_bytes()));
    let binding = serde_json::to_vec(&(
        "open-seo-spider/custom-search-input-v1",
        source,
        &page.url,
        document_url,
        &page.fetched_at,
        &page.rendered_at,
        generation,
        input_sha256,
    ))
    .expect("String and integer binding serializes");
    format!("{:x}", Sha256::digest(binding))
}

pub(crate) fn selected_text(page: &mut PageRecord, compiled: &CompiledCustomSearchConfig) {
    for (source, capture_kind) in [
        (
            CustomSearchSource::RawSelectedText,
            ContentCaptureSourceKind::RawHttpHtml,
        ),
        (
            CustomSearchSource::RenderedSelectedText,
            ContentCaptureSourceKind::RenderedDomHtml,
        ),
    ] {
        let observations = if let Some(capture) = page
            .content_captures
            .as_ref()
            .and_then(|captures| captures.iter().find(|row| row.source_kind == capture_kind))
        {
            let complete = matches!(
                capture.status,
                PageContentCaptureStatus::Complete | PageContentCaptureStatus::Empty
            ) && capture.result.as_ref().is_some_and(|result| {
                result.source_complete && result.decode_status == ContentCaptureDecodeStatus::Exact
            });
            compiled.observe(
                source,
                CustomSearchInput {
                    text: capture
                        .result
                        .as_ref()
                        .and_then(|result| result.text.as_deref()),
                    complete,
                    reason: (!complete)
                        .then_some("selected_text_capture_incomplete_or_unavailable"),
                    document_url: capture.source_document_url.as_deref(),
                    observed_generation: capture.observed_generation,
                    source_revision: Some(&capture.source_revision),
                },
            )
        } else {
            compiled.observe(
                source,
                CustomSearchInput {
                    reason: Some("selected_text_capture_not_observed"),
                    ..Default::default()
                },
            )
        };
        replace(page, compiled, source, observations);
    }
}
