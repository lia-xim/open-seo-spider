//! One retained metadata channel from exactly the requested captured source.
//! Imported report declarations remain declarations, not parser/native attestation.
use super::*;
use open_seo_spider_core::{
    DomMetadata, NgramChannelDocument, NgramHtmlTextChannel, NgramHtmlTextProfile,
    PageContentCapture, PageContentCaptureStatus, PageRecord, aggregate_saved_channel_ngrams,
};

fn elements<'a>(
    page: &'a PageRecord,
    capture: Option<&PageContentCapture>,
    kind: ContentCaptureSourceKind,
    profile: NgramHtmlTextProfile,
) -> std::result::Result<Vec<&'a str>, &'static str> {
    let capture = capture.ok_or("missing_or_pending_capture")?;
    if !matches!(
        capture.status,
        PageContentCaptureStatus::Complete | PageContentCaptureStatus::Empty
    ) || capture.source_html_utf8_sha256.is_none()
    {
        return Err("incomplete_or_unavailable_capture");
    }
    let (titles, descriptions, facts, expected_document, expected_source) = match kind {
        ContentCaptureSourceKind::RawHttpHtml => {
            if page.error.is_some()
                || page.truncated
                || page.blocked_by_robots
                || page.final_status != Some(200)
            {
                return Err("native_html_observation_not_complete_200");
            }
            // Rendering retains a separate raw metadata observation. Never substitute
            // current rendered fields for a missing raw observation (or vice versa).
            let (titles, descriptions, facts) = if let Some(raw) = &page.raw_metadata {
                (
                    &raw.titles,
                    &raw.meta_descriptions,
                    raw.dom_metadata.as_ref(),
                )
            } else {
                if page.rendered
                    || page.renderer_error.is_some()
                    || page.rendered_url.is_some()
                    || page.rendered_at.is_some()
                {
                    return Err("missing_separate_raw_metadata");
                }
                (
                    &page.titles,
                    &page.meta_descriptions,
                    page.dom_metadata.as_ref(),
                )
            };
            (
                titles,
                descriptions,
                facts,
                page.final_url.as_str(),
                "raw_html",
            )
        }
        ContentCaptureSourceKind::RenderedDomHtml => {
            if !page.rendered
                || page.renderer_error.is_some()
                || page.rendered_status != Some(200)
                || page.rendered_at.is_none()
            {
                return Err("rendered_html_observation_not_complete_200");
            }
            let document = page
                .rendered_url
                .as_deref()
                .ok_or("missing_rendered_document_url")?;
            (
                &page.titles,
                &page.meta_descriptions,
                page.dom_metadata.as_ref(),
                document,
                "rendered_html",
            )
        }
    };
    if capture.source_document_url.as_deref() != Some(expected_document) {
        return Err("capture_metadata_document_mismatch");
    }
    let facts: &DomMetadata = facts.ok_or("missing_legacy_dom_metadata")?;
    if facts.engine != "independent_html5_dom_v1" || facts.source != expected_source {
        return Err("metadata_engine_or_source_mismatch");
    }
    if !facts.complete || !facts.limits_reached.is_empty() {
        return Err("incomplete_dom_metadata");
    }
    // The complete matching observation establishes even Some(empty). The stored
    // extractor preserves duplicate/empty title and description elements. Headings
    // use the no-image-alt text field and exact HTML tag level, never aria-level.
    let selected: Vec<&str> = match profile.channel {
        NgramHtmlTextChannel::Title => titles
            .iter()
            .map(String::as_str)
            .take(open_seo_spider_core::MAX_NGRAM_ELEMENTS_PER_PAGE + 1)
            .collect(),
        NgramHtmlTextChannel::MetaDescription => descriptions
            .iter()
            .map(String::as_str)
            .take(open_seo_spider_core::MAX_NGRAM_ELEMENTS_PER_PAGE + 1)
            .collect(),
        channel => facts
            .headings
            .iter()
            .filter(|h| Some(h.tag_level) == channel.heading_level())
            .map(|h| h.text.as_str())
            .take(open_seo_spider_core::MAX_NGRAM_ELEMENTS_PER_PAGE + 1)
            .collect(),
    };
    if selected.len() > open_seo_spider_core::MAX_NGRAM_ELEMENTS_PER_PAGE {
        return Err("metadata_element_limit_exceeded");
    }
    Ok(selected)
}

pub(super) fn aggregate(
    report: &CrawlReport,
    request: &Request,
    profile: NgramHtmlTextProfile,
    ordinals: &[usize],
) -> Result<NgramCorpusReport> {
    ensure!(
        ordinals.len() <= request.options.limits.max_documents,
        "Saved document count exceeds requested limit"
    );
    let mut docs = Vec::with_capacity(ordinals.len());
    for &ordinal in ordinals {
        let page = &report.pages[ordinal];
        // Existing capture validation, selected-target generation, pending exclusion
        // and original source revision authority remain exactly the shared adapter.
        let (basis, capture) =
            crate::content_cohort::captured_source(report, page, request.source_kind)?;
        let selected = elements(page, capture, request.source_kind, profile);
        let complete = selected.is_ok();
        let (elements, unavailable_reason) = match selected {
            Ok(v) => (Some(v), None),
            Err(reason) => (None, Some(reason)),
        };
        docs.push(NgramChannelDocument {
            source: NgramCorpusDocument {
                requested_url: &page.url,
                source_revision: capture.map(|c| c.source_revision.as_str()),
                capture_config_sha256: capture.map(|c| c.config_sha256.as_str()),
                text_sha256: None,
                text: None,
                complete,
                basis,
                observed_generation: capture.and_then(|c| c.observed_generation),
            },
            source_kind: request.source_kind,
            source_document_url: capture.and_then(|c| c.source_document_url.as_deref()),
            capture_html_sha256: capture.and_then(|c| c.source_html_utf8_sha256.as_deref()),
            capture_status: None,
            metadata_observation: None,
            elements,
            unavailable_reason,
        });
    }
    aggregate_saved_channel_ngrams(&docs, &request.options, profile)
}

pub(super) fn csv(
    report: &Aggregate,
    request: &Request,
    snapshot: Snapshot,
) -> Result<(Vec<u8>, Snapshot)> {
    let mut out = Bytes {
        data: vec![],
        cap: CSV_BYTES,
    };
    csv_header(
        &mut out,
        &[
            "snapshot_token",
            "view",
            "n",
            "tokens_json",
            "occurrences",
            "document_frequency",
            "source_ordinal",
            "requested_url",
            "source_revision",
            "capture_config_sha256",
            "text_sha256",
            "basis",
            "observed_generation",
            "source_state",
            "source_counted",
            "first_positions_json",
            "positions_sampled",
            "website_requests",
            "html_text_channel_json",
            "channel_source_json",
        ]
        .map(str::to_owned),
        &snapshot,
    )?;
    visit(report, request, |row| {
        let source = &row["source"];
        let cell = |v: &Value| match v {
            Value::Null => String::new(),
            Value::String(v) => v.clone(),
            v => v.to_string(),
        };
        csv_record(
            &mut out,
            &[
                snapshot.snapshot_token.clone(),
                cell(&row["view"]),
                cell(&row["n"]),
                row["tokens"].to_string(),
                cell(&row["occurrences"]),
                cell(&row["document_frequency"]),
                cell(&source["ordinal"]),
                cell(&source["requested_url"]),
                cell(&source["source_revision"]),
                cell(&source["capture_config_sha256"]),
                cell(&source["text_sha256"]),
                cell(&source["basis"]),
                cell(&source["observed_generation"]),
                cell(&source["state"]),
                cell(&source["counted"]),
                row["first_positions"].to_string(),
                cell(&row["positions_sampled"]),
                "0".into(),
                serde_json::to_string(&request.html_text_channel)?,
                source["channel"].to_string(),
            ],
            &snapshot,
            &source["raw_page_ordinal"],
        )
    })?;
    Ok((out.data, snapshot))
}

// Metadata is itself a retained source observation. Optional body extraction is
// not its completeness authority and an HTML file is never reconstructed here.
fn metadata_elements(
    page: &PageRecord,
    kind: ContentCaptureSourceKind,
    profile: open_seo_spider_core::NgramRetainedMetadataProfile,
) -> std::result::Result<
    (
        Vec<&str>,
        open_seo_spider_core::NgramRetainedMetadataIdentity<'_>,
    ),
    &'static str,
> {
    if page.fetched_at.is_empty() || page.fetched_at.len() > 256 {
        return Err("missing_or_invalid_fetch_identity");
    }
    let (titles, descriptions, facts, rendered_at, expected_source) = match kind {
        ContentCaptureSourceKind::RawHttpHtml => {
            if page.error.is_some()
                || page.truncated
                || page.blocked_by_robots
                || page.final_status != Some(200)
                || page.final_url.is_empty()
            {
                return Err("native_metadata_observation_not_complete_200");
            }
            let (titles, descriptions, facts) = if let Some(raw) = &page.raw_metadata {
                (
                    &raw.titles,
                    &raw.meta_descriptions,
                    raw.dom_metadata.as_ref(),
                )
            } else {
                if page.rendered
                    || page.renderer_error.is_some()
                    || page.rendered_url.is_some()
                    || page.rendered_at.is_some()
                {
                    return Err("missing_separate_raw_metadata");
                }
                (
                    &page.titles,
                    &page.meta_descriptions,
                    page.dom_metadata.as_ref(),
                )
            };
            (titles, descriptions, facts, None, "raw_html")
        }
        ContentCaptureSourceKind::RenderedDomHtml => {
            if !page.rendered
                || page.renderer_error.is_some()
                || page.rendered_status != Some(200)
                || page.rendered_url.as_deref().is_none_or(str::is_empty)
            {
                return Err("rendered_metadata_observation_not_complete_200");
            }
            let at = page
                .rendered_at
                .as_deref()
                .filter(|v| !v.is_empty() && v.len() <= 256)
                .ok_or("missing_or_invalid_render_identity")?;
            (
                &page.titles,
                &page.meta_descriptions,
                page.dom_metadata.as_ref(),
                Some(at),
                "rendered_html",
            )
        }
    };
    let facts = facts.ok_or("missing_legacy_dom_metadata")?;
    if facts.engine != "independent_html5_dom_v1" || facts.source != expected_source {
        return Err("metadata_engine_or_source_mismatch");
    }
    if !facts.complete || !facts.limits_reached.is_empty() {
        return Err("incomplete_dom_metadata");
    }
    let selected: Vec<&str> = match profile.channel {
        NgramHtmlTextChannel::Title => titles
            .iter()
            .map(String::as_str)
            .take(open_seo_spider_core::MAX_NGRAM_ELEMENTS_PER_PAGE + 1)
            .collect(),
        NgramHtmlTextChannel::MetaDescription => descriptions
            .iter()
            .map(String::as_str)
            .take(open_seo_spider_core::MAX_NGRAM_ELEMENTS_PER_PAGE + 1)
            .collect(),
        level => facts
            .headings
            .iter()
            .filter(|h| Some(h.tag_level) == level.heading_level())
            .map(|h| h.text.as_str())
            .take(open_seo_spider_core::MAX_NGRAM_ELEMENTS_PER_PAGE + 1)
            .collect(),
    };
    if selected.len() > open_seo_spider_core::MAX_NGRAM_ELEMENTS_PER_PAGE {
        return Err("metadata_element_limit_exceeded");
    }
    Ok((
        selected,
        open_seo_spider_core::NgramRetainedMetadataIdentity {
            fetched_at: &page.fetched_at,
            rendered_at,
            engine: &facts.engine,
            source: &facts.source,
        },
    ))
}
pub(super) fn aggregate_metadata(
    report: &CrawlReport,
    request: &Request,
    profile: open_seo_spider_core::NgramRetainedMetadataProfile,
    ordinals: &[usize],
) -> Result<NgramCorpusReport> {
    ensure!(
        ordinals.len() <= request.options.limits.max_documents,
        "Saved document count exceeds requested limit"
    );
    let mut docs = Vec::with_capacity(ordinals.len());
    for &ordinal in ordinals {
        let page = &report.pages[ordinal];
        // A present original capture remains validated even though it is optional.
        // Broken/replaced/generation-foreign captures are errors, never ignored.
        open_seo_spider_core::validate_page_content_captures(page)?;
        let (basis, capture) =
            crate::content_cohort::captured_source(report, page, request.source_kind)?;
        let selected = if basis == "pending_selected" {
            Err("pending_selected_metadata")
        } else if basis == "unknown_operation" {
            Err("unknown_operation_metadata")
        } else {
            metadata_elements(page, request.source_kind, profile)
        };
        let complete = selected.is_ok();
        let (elements, metadata_observation, unavailable_reason) = match selected {
            Ok((elements, identity)) => (Some(elements), Some(identity), None),
            Err(reason) => (None, None, Some(reason)),
        };
        let generation = if basis == "refreshed_selected" {
            report
                .operation
                .as_ref()
                .and_then(|o| o.targets.iter().find(|t| t.requested_url == page.url))
                .and_then(|t| t.committed_generation)
        } else {
            capture.and_then(|c| c.observed_generation)
        };
        docs.push(NgramChannelDocument {
            source: NgramCorpusDocument {
                requested_url: &page.url,
                source_revision: capture.map(|c| c.source_revision.as_str()),
                capture_config_sha256: capture.map(|c| c.config_sha256.as_str()),
                text_sha256: None,
                text: None,
                complete,
                basis,
                observed_generation: generation,
            },
            source_kind: request.source_kind,
            source_document_url: match request.source_kind {
                ContentCaptureSourceKind::RawHttpHtml => Some(page.final_url.as_str()),
                ContentCaptureSourceKind::RenderedDomHtml => page.rendered_url.as_deref(),
            },
            capture_html_sha256: capture.and_then(|c| c.source_html_utf8_sha256.as_deref()),
            capture_status: capture.map(|c| c.status),
            metadata_observation,
            elements,
            unavailable_reason,
        });
    }
    open_seo_spider_core::aggregate_saved_metadata_ngrams(&docs, &request.options, profile)
}
pub(super) fn csv_metadata(
    report: &Aggregate,
    request: &Request,
    snapshot: Snapshot,
) -> Result<(Vec<u8>, Snapshot)> {
    let mut out = Bytes {
        data: vec![],
        cap: CSV_BYTES,
    };
    csv_header(
        &mut out,
        &[
            "snapshot_token",
            "view",
            "n",
            "tokens_json",
            "occurrences",
            "document_frequency",
            "source_ordinal",
            "requested_url",
            "source_revision",
            "capture_config_sha256",
            "text_sha256",
            "basis",
            "observed_generation",
            "source_state",
            "source_counted",
            "first_positions_json",
            "positions_sampled",
            "website_requests",
            "retained_metadata_channel_json",
            "channel_source_json",
        ]
        .map(str::to_owned),
        &snapshot,
    )?;
    visit(report, request, |row| {
        let source = &row["source"];
        let cell = |v: &Value| match v {
            Value::Null => String::new(),
            Value::String(v) => v.clone(),
            v => v.to_string(),
        };
        csv_record(
            &mut out,
            &[
                snapshot.snapshot_token.clone(),
                cell(&row["view"]),
                cell(&row["n"]),
                row["tokens"].to_string(),
                cell(&row["occurrences"]),
                cell(&row["document_frequency"]),
                cell(&source["ordinal"]),
                cell(&source["requested_url"]),
                cell(&source["source_revision"]),
                cell(&source["capture_config_sha256"]),
                cell(&source["text_sha256"]),
                cell(&source["basis"]),
                cell(&source["observed_generation"]),
                cell(&source["state"]),
                cell(&source["counted"]),
                row["first_positions"].to_string(),
                cell(&row["positions_sampled"]),
                "0".into(),
                serde_json::to_string(&request.retained_metadata_channel)?,
                source["channel"].to_string(),
            ],
            &snapshot,
            &source["raw_page_ordinal"],
        )
    })?;
    Ok((out.data, snapshot))
}
