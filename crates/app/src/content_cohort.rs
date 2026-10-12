//! Offline projection of explicitly selected persisted capture observations.
use anyhow::{Result, ensure};
use open_seo_spider_core::{
    ContentAnalysisConfig, ContentAnalysisInput, ContentCaptureSourceKind, ContentCaptureStatus,
    ContentTextInput, CrawlReport, PageContentCapture, PageRecord, RecrawlTargetState,
    SELECTED_RECRAWL_COVERAGE, SELECTED_RECRAWL_ENGINE, analyze_content,
    validate_page_content_captures,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::Write;

pub const SAVED_CONTENT_ENGINE: &str = "open-seo-spider/saved-content-cohort-v1";
pub const REPORT_HASH_PROFILE: &str = "typed-crawl-report-serde-json-v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SavedContentRequest {
    pub urls: Vec<String>,
    #[serde(default = "default_source")]
    pub source_kind: ContentCaptureSourceKind,
    #[serde(default)]
    pub config: ContentAnalysisConfig,
    #[serde(default)]
    pub indexable_only: bool,
    #[serde(default)]
    pub complete_only: bool,
    #[serde(default)]
    pub expected_generation: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotContext {
    pub sha256: String,
    pub sha_profile: &'static str,
    pub generation: Option<u64>,
    pub origin: String,
}

pub fn default_source() -> ContentCaptureSourceKind {
    ContentCaptureSourceKind::RawHttpHtml
}

struct HashWriter(Sha256);
impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub fn snapshot_context(
    report: &CrawlReport,
    generation: Option<u64>,
    origin: &str,
) -> Result<SnapshotContext> {
    let mut sink = HashWriter(Sha256::new());
    serde_json::to_writer(&mut sink, report)?;
    Ok(SnapshotContext {
        sha256: format!("{:x}", sink.0.finalize()),
        sha_profile: REPORT_HASH_PROFILE,
        generation,
        origin: origin.into(),
    })
}

fn source_label(source: ContentCaptureSourceKind) -> &'static str {
    match source {
        ContentCaptureSourceKind::RawHttpHtml => "raw_http_html",
        ContentCaptureSourceKind::RenderedDomHtml => "rendered_dom_html",
    }
}

fn observation_basis(
    report: &CrawlReport,
    page: &PageRecord,
) -> Result<(&'static str, Option<u64>)> {
    let Some(operation) = &report.operation else {
        return Ok(("captured_in_crawl", None));
    };
    if operation.engine != SELECTED_RECRAWL_ENGINE
        || operation.coverage != SELECTED_RECRAWL_COVERAGE
    {
        return Ok(("unknown_operation", None));
    }
    let mut targets = operation
        .targets
        .iter()
        .filter(|target| target.requested_url == page.url);
    let Some(target) = targets.next() else {
        return Ok(("inherited_parent", None));
    };
    ensure!(
        targets.next().is_none(),
        "Duplicate selected target identity"
    );
    match target.state {
        RecrawlTargetState::Pending => Ok(("pending_selected", None)),
        RecrawlTargetState::Committed => {
            ensure!(
                target.admitted_attempts > 0
                    && target
                        .committed_generation
                        .is_some_and(|generation| generation > 0),
                "Committed selected target has no positive admission/generation"
            );
            Ok(("refreshed_selected", target.committed_generation))
        }
    }
}

fn captured_source<'a>(
    report: &CrawlReport,
    page: &'a PageRecord,
    source: ContentCaptureSourceKind,
) -> Result<(&'static str, Option<&'a PageContentCapture>)> {
    let (basis, generation) = observation_basis(report, page)?;
    // A pending selected row is still a parent observation, never current source text.
    if basis == "pending_selected" {
        return Ok((basis, None));
    }
    validate_page_content_captures(page)?;
    let mut sources = page
        .content_captures
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|capture| capture.source_kind == source);
    let capture = sources.next();
    ensure!(
        sources.next().is_none(),
        "Duplicate captured source for requested identity"
    );
    if let Some(capture) = capture
        && let Some(generation) = generation
    {
        ensure!(
            capture.observed_generation == Some(generation),
            "Selected source generation differs from committed target generation"
        );
    }
    Ok((basis, capture))
}

pub fn document_summary(
    report: &CrawlReport,
    ordinal: usize,
    source: ContentCaptureSourceKind,
) -> Result<Value> {
    let page = report
        .pages
        .get(ordinal)
        .ok_or_else(|| anyhow::anyhow!("Unknown saved page ordinal"))?;
    let (basis, capture) = captured_source(report, page, source)?;
    let result = capture.and_then(|capture| capture.result.as_ref());
    let status = capture
        .map(|capture| serde_json::to_value(capture.status))
        .transpose()?
        .unwrap_or_else(|| json!("missing"));
    let reason = if basis == "pending_selected" {
        Some("selected_target_pending")
    } else if capture.is_none() {
        Some("no_saved_source_capture")
    } else {
        None
    };
    Ok(json!({
        "ordinal":ordinal,"requested_url":page.url,"basis":basis,"source_kind":source,
        "parent_source_kind":report.operation.as_ref().map(|operation|operation.parent.source_kind),
        "parent_snapshot_sha256":report.operation.as_ref().map(|operation|&operation.parent.snapshot_sha256),
        "page_http_status":page.final_status,"indexable":page.indexable,"status":status,"reason":reason,
        "source_document_url":capture.and_then(|capture|capture.source_document_url.as_deref()),
        "observed_generation":capture.and_then(|capture|capture.observed_generation),
        "source_revision":capture.map(|capture|&capture.source_revision),
        "capture_engine":capture.map(|capture|&capture.engine),
        "selection_profile":result.map(|result|&result.selection_profile),
        "capture_config_sha256":capture.map(|capture|&capture.config_sha256),
        "html_language":result.and_then(|result|result.html_language.as_deref()),
        "html_language_reason":result.and_then(|result|result.html_language_reason),
        "text_sha256":result.and_then(|result|result.text_sha256.as_deref()),
        "text_bytes":result.and_then(|result|result.text.as_ref().map(String::len)),
        "sample_complete":result.is_some_and(|result|matches!(result.status, ContentCaptureStatus::Complete|ContentCaptureStatus::Empty)),
        "capture_reasons":result.map(|result|&result.reasons),"capture_failure":capture.and_then(|capture|capture.failure.as_ref()),
        "text_included":false
    }))
}

pub fn inventory(
    report: &CrawlReport,
    context: &SnapshotContext,
    source: ContentCaptureSourceKind,
    after: usize,
    limit: usize,
) -> Result<Value> {
    ensure!((1..=200).contains(&limit), "limit must be 1..200");
    ensure!(
        after <= report.pages.len(),
        "after_ordinal exceeds the saved page count"
    );
    let end = after.saturating_add(limit).min(report.pages.len());
    let rows = (after..end)
        .map(|ordinal| document_summary(report, ordinal, source))
        .collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"engine":SAVED_CONTENT_ENGINE,"snapshot":context,"source_kind":source,"after_ordinal":after,"limit":limit,
        "total_saved_pages":report.pages.len(),"rows":rows,"next_after_ordinal":(end<report.pages.len()).then_some(end),
        "has_more":end<report.pages.len(),"text_included":false,"website_requests":0,"site_wide_coverage":false}),
    )
}

pub fn saved_document(
    report: &CrawlReport,
    context: &SnapshotContext,
    source: ContentCaptureSourceKind,
    url: &str,
) -> Result<Value> {
    let mut pages = report
        .pages
        .iter()
        .enumerate()
        .filter(|(_, page)| page.url == url);
    let (ordinal, page) = pages
        .next()
        .ok_or_else(|| anyhow::anyhow!("Exact requested URL is not in the saved report"))?;
    ensure!(
        pages.next().is_none(),
        "Saved report has duplicate requested identities"
    );
    let (_, capture) = captured_source(report, page, source)?;
    Ok(
        json!({"engine":SAVED_CONTENT_ENGINE,"snapshot":context,"source_kind":source,"row":document_summary(report,ordinal,source)?,
        "capture":capture,"website_requests":0,"site_wide_coverage":false}),
    )
}

pub fn prepare_saved_analysis(
    report: &CrawlReport,
    context: &SnapshotContext,
    request: &SavedContentRequest,
) -> Result<(Value, Option<ContentAnalysisInput>)> {
    request.config.limits.validate()?;
    // Validate the complete declared profile even when explicit filters exclude every row.
    analyze_content(&ContentAnalysisInput {
        corpus_id: "configuration-validation".into(),
        corpus_scope: "configuration validation with one missing sample".into(),
        corpus_complete: false,
        config: request.config.clone(),
        documents: vec![ContentTextInput {
            id: "config-check".into(),
            source: "configuration_validation".into(),
            text: None,
            language: None,
            complete: false,
            truncation_reason: None,
        }],
    })?;
    ensure!(
        !request.urls.is_empty() && request.urls.len() <= request.config.limits.max_documents,
        "Select 1..=max_documents exact requested URLs (maximum128)"
    );
    if let Some(expected) = request.expected_generation {
        ensure!(
            context.generation == Some(expected),
            "Saved snapshot generation changed or is unknown"
        );
    }
    let mut seen = HashSet::new();
    ensure!(
        request
            .urls
            .iter()
            .all(|url| url.len() <= 16 * 1024 && seen.insert(url)),
        "Duplicate or oversized selected URL"
    );
    let mut index = HashMap::new();
    for (ordinal, page) in report.pages.iter().enumerate() {
        ensure!(
            index.insert(&page.url, ordinal).is_none(),
            "Saved report has duplicate requested identities"
        );
    }
    let mut documents = Vec::new();
    let mut provenance = Vec::new();
    let mut excluded = Vec::new();
    let mut text_bytes = 0usize;
    for (selection_ordinal, url) in request.urls.iter().enumerate() {
        let ordinal = *index.get(url).ok_or_else(|| {
            anyhow::anyhow!("Exact requested URL is not in the saved report: {url}")
        })?;
        let page = &report.pages[ordinal];
        let row = document_summary(report, ordinal, request.source_kind)?;
        let (basis, capture) = captured_source(report, page, request.source_kind)?;
        let result = capture.and_then(|capture| capture.result.as_ref());
        let complete = result.is_some_and(|result| {
            matches!(
                result.status,
                ContentCaptureStatus::Complete | ContentCaptureStatus::Empty
            )
        });
        let excluded_reason = if request.indexable_only && !page.indexable {
            Some("indexability_filter")
        } else if request.complete_only && !complete {
            Some("complete_sample_filter")
        } else {
            None
        };
        if let Some(reason) = excluded_reason {
            excluded.push(json!({"row":row,"reason":reason}));
            continue;
        }
        let text = result.and_then(|result| result.text.as_ref());
        text_bytes = text_bytes
            .checked_add(text.map_or(0, String::len))
            .ok_or_else(|| anyhow::anyhow!("Selected text byte overflow"))?;
        ensure!(
            text_bytes <= request.config.limits.max_input_bytes,
            "Selected saved text exceeds max_input_bytes; select a smaller explicit cohort"
        );
        let id = format!("selection-{selection_ordinal}");
        let truncation_reason = if complete {
            None
        } else {
            Some(
                if basis == "pending_selected" {
                    "selected_target_pending"
                } else if result.is_none() {
                    "saved_source_unavailable"
                } else {
                    "saved_sample_incomplete"
                }
                .into(),
            )
        };
        documents.push(ContentTextInput {
            id: id.clone(),
            source: format!("saved_{}", source_label(request.source_kind)),
            text: text.cloned(),
            language: result.and_then(|result| result.html_language.clone()),
            complete,
            truncation_reason,
        });
        provenance.push(json!({"analysis_document_id":id,"row":row}));
    }
    let input=(!documents.is_empty()).then(||ContentAnalysisInput{corpus_id:context.sha256.clone(),corpus_scope:"explicit selected saved requested URLs, one source per URL; website coverage is not inferred".into(),corpus_complete:true,config:request.config.clone(),documents});
    let envelope = json!({"engine":SAVED_CONTENT_ENGINE,"snapshot":context,"source_kind":request.source_kind,"requested_urls":request.urls,
        "indexable_only":request.indexable_only,"complete_only":request.complete_only,"selected_count":request.urls.len(),
        "included_count":provenance.len(),"excluded":excluded,"documents":provenance,"website_requests":0,"site_wide_coverage":false,
        "limitations":["Source bindings are persisted consistency evidence; arbitrary imported JSON is declarative.","No stored HTML selection replay or fresh website observation is inferred.","Finite explicit cohort only; no site-wide duplicate coverage or vendor-equivalence claim."]});
    Ok((envelope, input))
}

pub fn finish_saved_analysis(
    mut envelope: Value,
    input: Option<ContentAnalysisInput>,
) -> Result<Value> {
    if let Some(input) = input {
        envelope["analysis"] = serde_json::to_value(analyze_content(&input)?)?;
        envelope["status"] = json!("ready");
    } else {
        envelope["analysis"] = Value::Null;
        envelope["status"] = json!("empty_filtered_cohort");
    }
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use open_seo_spider_core::{CrawlConfig, CrawlSummary};
    fn report() -> CrawlReport {
        CrawlReport {
            config: CrawlConfig::default(),
            pages: vec![
                PageRecord {
                    url: "https://example.test/one".into(),
                    ..Default::default()
                },
                PageRecord {
                    url: "https://example.test/two".into(),
                    ..Default::default()
                },
            ],
            issues: vec![],
            summary: CrawlSummary::default(),
            warnings: vec![],
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
        }
    }
    fn request() -> SavedContentRequest {
        serde_json::from_value(
            json!({"urls":["https://example.test/one","https://example.test/two"]}),
        )
        .unwrap()
    }
    #[test]
    fn legacy_absence_remains_missing_with_unknown_pairs_and_metadata_only_inventory() {
        let report = report();
        let context = snapshot_context(&report, Some(1), "declared_imported_report").unwrap();
        let (envelope, input) = prepare_saved_analysis(&report, &context, &request()).unwrap();
        let result = finish_saved_analysis(envelope, input).unwrap();
        assert_eq!(result["analysis"]["documents"][0]["status"], "missing");
        assert!(result["analysis"]["pairs"][0]["near_duplicate"].is_null());
        let page = inventory(&report, &context, default_source(), 0, 1).unwrap();
        assert_eq!(page["rows"][0]["status"], "missing");
        assert_eq!(page["next_after_ordinal"], 1);
        assert!(page["rows"][0].get("text").is_none());
    }
    #[test]
    fn exact_selection_identity_and_generation_are_not_silently_repaired() {
        let report = report();
        let context = snapshot_context(&report, Some(3), "native_saved_report").unwrap();
        let mut input = request();
        input.urls.push(input.urls[0].clone());
        assert!(prepare_saved_analysis(&report, &context, &input).is_err());
        input = request();
        input.urls[0].push('/');
        assert!(prepare_saved_analysis(&report, &context, &input).is_err());
        input = request();
        input.expected_generation = Some(2);
        assert!(prepare_saved_analysis(&report, &context, &input).is_err());
        assert!(inventory(&report, &context, default_source(), 3, 200).is_err());
        assert!(inventory(&report, &context, default_source(), 0, 201).is_err());
    }
    #[test]
    fn filtering_all_rows_is_explicit_and_preserves_excluded_identities() {
        let report = report();
        let context = snapshot_context(&report, None, "declared_imported_report").unwrap();
        let mut input = request();
        input.complete_only = true;
        let (envelope, input) = prepare_saved_analysis(&report, &context, &input).unwrap();
        let result = finish_saved_analysis(envelope, input).unwrap();
        assert_eq!(result["status"], "empty_filtered_cohort");
        assert!(result["analysis"].is_null());
        assert_eq!(result["excluded"].as_array().unwrap().len(), 2);
        assert_eq!(
            result["excluded"][0]["row"]["requested_url"],
            "https://example.test/one"
        );
        let mut invalid = request();
        invalid.complete_only = true;
        invalid.config.similarity.threshold = 2.0;
        assert!(prepare_saved_analysis(&report, &context, &invalid).is_err());
    }
    #[test]
    fn pending_selected_rows_never_supply_parent_capture_text() {
        let mut report = report();
        report.operation=Some(serde_json::from_value(json!({"engine":SELECTED_RECRAWL_ENGINE,"coverage":SELECTED_RECRAWL_COVERAGE,"parent":{"job_id":"parent","checkpoint_revision":1,"source_kind":"stored_json","snapshot_sha256":"declared"},"plan_sha256":"declared","inherited_config_sha256":"declared","selection":[],"retained_count":1,"refreshed_count":0,"pending_count":1,"max_attempts":null,"admitted_this_run":0,"committed_this_run":0,"targets":[{"requested_url":"https://example.test/one","parent_ordinal":0,"parent_page_sha256":"declared","state":"pending","admitted_attempts":0,"committed_generation":null}]})).unwrap());
        let context = snapshot_context(&report, Some(1), "declared_imported_report").unwrap();
        let (envelope, input) = prepare_saved_analysis(&report, &context, &request()).unwrap();
        assert!(input.unwrap().documents[0].text.is_none());
        assert_eq!(envelope["documents"][0]["row"]["basis"], "pending_selected");
        assert_eq!(envelope["documents"][1]["row"]["basis"], "inherited_parent");
        assert!(
            saved_document(
                &report,
                &context,
                default_source(),
                "https://example.test/one"
            )
            .unwrap()["capture"]
                .is_null()
        );
        report.operation.as_mut().unwrap().engine = "unrecognized-imported-profile".into();
        assert_eq!(
            document_summary(&report, 0, default_source()).unwrap()["basis"],
            "unknown_operation"
        );
    }
    #[test]
    fn request_unknown_fields_are_rejected_and_typed_snapshot_hash_is_stable() {
        assert!(
            serde_json::from_value::<SavedContentRequest>(
                json!({"urls":["https://example.test/one"],"html":"invented"})
            )
            .is_err()
        );
        let report = report();
        let first = snapshot_context(&report, None, "declared_imported_report").unwrap();
        let restored: CrawlReport =
            serde_json::from_slice(&serde_json::to_vec(&report).unwrap()).unwrap();
        assert_eq!(
            first.sha256,
            snapshot_context(&restored, Some(2), "native_saved_report")
                .unwrap()
                .sha256
        );
    }
}
