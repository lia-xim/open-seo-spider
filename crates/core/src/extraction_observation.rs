//! Capture declarations for custom extraction. Validation checks internal
//! consistency; retained hashes do not authenticate imported HTML or values.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ContentCaptureDecodeStatus, CrawlReport, CustomExtraction, CustomExtractionConfig, PageRecord,
};

pub const EXTRACTION_SOURCE_ENGINE: &str = "open-seo-spider/extraction-source-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionSourceStatus {
    Complete,
    Incomplete,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionEvaluationStatus {
    Evaluated,
    Error,
    NotRun,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionSourceObservation {
    pub engine: String,
    pub source: String,
    pub status: ExtractionSourceStatus,
    pub evaluation: ExtractionEvaluationStatus,
    pub decode_status: ContentCaptureDecodeStatus,
    pub document_url: Option<String>,
    pub observed_generation: Option<u64>,
    pub observed_at: Option<String>,
    pub input_sha256: Option<String>,
    pub input_utf8_bytes: Option<usize>,
    pub source_revision: Option<String>,
    pub reasons: Vec<String>,
}

/// Source status is independent of evaluation errors and max_results. The
/// generation is the producer's capture generation, never a browser revision.
pub struct ExtractionInputContext<'a> {
    pub html: Option<&'a str>,
    pub source: &'a str,
    pub status: ExtractionSourceStatus,
    pub decode_status: ContentCaptureDecodeStatus,
    pub document_url: Option<&'a str>,
    pub observed_generation: Option<u64>,
    pub observed_at: Option<&'a str>,
    pub reasons: Vec<String>,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn valid_url(value: &str) -> bool {
    url::Url::parse(value)
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}
fn valid_time(value: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(value).is_ok()
}

/// SHA256 of the UTF-8 compact JSON array in this exact order. Optional fields
/// are JSON null. The binding is also present for unavailable observations.
pub fn extraction_source_revision(row: &ExtractionSourceObservation) -> Result<String> {
    Ok(digest(&serde_json::to_vec(&(
        EXTRACTION_SOURCE_ENGINE,
        &row.source,
        &row.document_url,
        &row.observed_at,
        row.observed_generation,
        &row.input_sha256,
        row.input_utf8_bytes,
    ))?))
}

impl ExtractionSourceObservation {
    pub fn exact_count(&self, result: &CustomExtraction) -> Option<usize> {
        (self.status == ExtractionSourceStatus::Complete
            && self.evaluation == ExtractionEvaluationStatus::Evaluated
            && validate_extraction_source_observation(result, self).is_ok())
        .then_some(result.matched_count)
        .flatten()
    }

    /// Partial extraction counts describe evaluated input only. DOM recovery,
    /// scalar XPath and anchored regex prevent a general lower-bound claim.
    pub fn count_scope(&self, result: &CustomExtraction) -> &'static str {
        if validate_extraction_source_observation(result, self).is_err() {
            return "unknown";
        }
        match self.evaluation {
            ExtractionEvaluationStatus::Evaluated if self.exact_count(result).is_some() => {
                "complete_source"
            }
            ExtractionEvaluationStatus::Evaluated => "evaluated_input_only",
            ExtractionEvaluationStatus::Error => "evaluation_failed",
            ExtractionEvaluationStatus::NotRun => "not_evaluated",
        }
    }
}

pub fn validate_extraction_source_observation(
    result: &CustomExtraction,
    row: &ExtractionSourceObservation,
) -> Result<()> {
    ensure!(
        row.engine == EXTRACTION_SOURCE_ENGINE,
        "Unknown extraction source engine"
    );
    let expected_engine = match result.method {
        crate::ExtractionMethod::Css => "scraper-html5-css",
        crate::ExtractionMethod::XPath => "sxd-xpath-1.0-html-dom",
        crate::ExtractionMethod::Regex => "rust-regex",
    };
    ensure!(
        result.engine == expected_engine,
        "Extraction evaluation engine differs from method"
    );
    ensure!(
        matches!(row.source.as_str(), "raw_html" | "rendered_html") && row.source == result.source,
        "Extraction source identity differs"
    );
    ensure!(
        row.document_url.as_deref().is_none_or(valid_url),
        "Invalid extraction document URL"
    );
    ensure!(
        row.observed_at.as_deref().is_none_or(valid_time),
        "Invalid extraction observation time"
    );
    ensure!(
        row.source_revision.as_deref().is_some_and(is_digest),
        "Missing or invalid extraction source revision"
    );
    ensure!(
        row.source_revision.as_deref() == Some(extraction_source_revision(row)?.as_str()),
        "Extraction source revision changed"
    );
    ensure!(
        row.reasons.len() <= 16
            && row
                .reasons
                .iter()
                .all(|reason| !reason.trim().is_empty() && reason.encode_utf16().count() <= 256),
        "Invalid or excessive extraction source reasons"
    );
    ensure!(
        row.input_sha256.is_some() == row.input_utf8_bytes.is_some(),
        "Extraction input hash/bytes availability differs"
    );
    ensure!(
        row.input_sha256.as_deref().is_none_or(is_digest),
        "Invalid extraction input hash"
    );
    if row.input_utf8_bytes == Some(0) {
        ensure!(
            row.input_sha256.as_deref() == Some(digest(b"").as_str()),
            "Empty extraction input digest differs"
        );
    }
    match row.status {
        ExtractionSourceStatus::Complete => {
            ensure!(
                row.decode_status == ContentCaptureDecodeStatus::Exact && row.reasons.is_empty(),
                "Complete extraction source has uncertain decoding or reasons"
            );
            ensure!(
                row.document_url.is_some() && row.observed_at.is_some(),
                "Complete extraction source lacks document identity/time"
            );
        }
        ExtractionSourceStatus::Incomplete | ExtractionSourceStatus::Unavailable => {
            ensure!(
                !row.reasons.is_empty(),
                "Uncertain extraction source lacks reasons"
            );
        }
    }
    if row.status == ExtractionSourceStatus::Unavailable {
        ensure!(
            row.input_sha256.is_none() && row.input_utf8_bytes.is_none(),
            "Unavailable extraction source declares input"
        );
        ensure!(
            row.decode_status == ContentCaptureDecodeStatus::Unknown,
            "Unavailable extraction source declares decoding"
        );
        ensure!(
            row.evaluation == ExtractionEvaluationStatus::NotRun,
            "Unavailable extraction was evaluated"
        );
    } else {
        ensure!(
            row.input_sha256.is_some() && row.input_utf8_bytes.is_some(),
            "Available extraction source lacks input identity"
        );
        ensure!(
            row.evaluation != ExtractionEvaluationStatus::NotRun,
            "Available extraction source was not evaluated"
        );
    }
    match row.evaluation {
        ExtractionEvaluationStatus::Evaluated => {
            let count = result
                .matched_count
                .context("Evaluated extraction count is unknown")?;
            ensure!(result.error.is_none(), "Evaluated extraction has an error");
            ensure!(
                count >= result.values.len(),
                "Retained extraction values exceed observed count"
            );
            ensure!(
                result.truncated == (count > result.values.len()),
                "Extraction result limit differs from observed count"
            );
        }
        ExtractionEvaluationStatus::Error => {
            ensure!(
                result
                    .error
                    .as_deref()
                    .is_some_and(|error| !error.trim().is_empty()),
                "Failed extraction lacks error evidence"
            );
            ensure!(
                result.matched_count.is_none() && result.values.is_empty() && !result.truncated,
                "Failed extraction retains a success count or values"
            );
        }
        ExtractionEvaluationStatus::NotRun => {
            ensure!(
                result.error.is_none()
                    && result.matched_count.is_none()
                    && result.values.is_empty()
                    && !result.truncated,
                "Not-run extraction declares evaluated results"
            );
        }
    }
    Ok(())
}

/// Legacy None is deliberately accepted; it is never upgraded into complete.
pub fn validate_page_extraction_observations(page: &PageRecord) -> Result<()> {
    for result in page.custom_extractions.iter().chain(
        page.raw_metadata
            .iter()
            .flat_map(|raw| &raw.custom_extractions),
    ) {
        if let Some(row) = &result.source_observation {
            validate_extraction_source_observation(result, row)?;
            // An unavailable early request may predate a later redirect. For
            // evaluated input the URL is bound to the actual source identity.
            if row.status != ExtractionSourceStatus::Unavailable {
                let expected = if row.source == "rendered_html" {
                    page.rendered_url.as_deref()
                } else {
                    Some(if page.final_url.is_empty() {
                        page.url.as_str()
                    } else {
                        page.final_url.as_str()
                    })
                };
                ensure!(
                    row.document_url
                        .as_deref()
                        .is_none_or(|url| Some(url) == expected),
                    "Extraction document URL differs from page source"
                );
                let expected_time = if row.source == "rendered_html" {
                    page.rendered_at.as_deref()
                } else {
                    Some(page.fetched_at.as_str())
                };
                ensure!(
                    row.observed_at
                        .as_deref()
                        .is_none_or(|time| Some(time) == expected_time),
                    "Extraction observation time differs from page source"
                );
            }
        }
    }
    Ok(())
}

pub fn validate_report_extraction_observations(report: &CrawlReport) -> Result<()> {
    for page in &report.pages {
        validate_page_extraction_observations(page)?;
    }
    Ok(())
}

/// Evaluate the existing engine against supplied input and bind only the
/// producer's source declaration. Invalid producer context is downgraded.
pub(crate) fn extract_observed(
    config: &[CustomExtractionConfig],
    mut input: ExtractionInputContext<'_>,
) -> Vec<CustomExtraction> {
    if !matches!(input.source, "raw_html" | "rendered_html") {
        input.source = "raw_html";
        input.html = None;
        input.reasons.push("invalid_source_context".into());
    }
    if input.document_url.is_some_and(|url| !valid_url(url)) {
        input.document_url = None;
        input.reasons.push("document_url_invalid".into());
    }
    if input.observed_at.is_some_and(|time| !valid_time(time)) {
        input.observed_at = None;
        input.reasons.push("observation_time_invalid".into());
    }
    if input.reasons.len() > 16
        || input
            .reasons
            .iter()
            .any(|reason| reason.trim().is_empty() || reason.encode_utf16().count() > 256)
    {
        input.reasons = vec!["invalid_source_context".into()];
    }
    if input.status == ExtractionSourceStatus::Unavailable {
        input.html = None;
    }
    if input.html.is_none() {
        input.status = ExtractionSourceStatus::Unavailable;
        input.decode_status = ContentCaptureDecodeStatus::Unknown;
        if input.reasons.is_empty() {
            input.reasons.push("source_not_observed".into());
        }
    } else {
        if input.decode_status != ContentCaptureDecodeStatus::Exact {
            input.reasons.push("decode_uncertain".into());
        }
        if input.document_url.is_none() {
            input.reasons.push("document_url_not_observed".into());
        }
        if input.observed_at.is_none() {
            input.reasons.push("observation_time_not_observed".into());
        }
        if input.status == ExtractionSourceStatus::Incomplete && input.reasons.is_empty() {
            input.reasons.push("source_completeness_unverified".into());
        }
        if !input.reasons.is_empty() {
            input.status = ExtractionSourceStatus::Incomplete;
        }
    }
    input.reasons.sort();
    input.reasons.dedup();
    if input.reasons.len() > 16 {
        input.reasons = vec!["invalid_source_context".into()];
    }
    let mut results = if let Some(html) = input.html {
        crate::extraction_engine::extract_all(html, config, input.source)
    } else {
        config
            .iter()
            .map(|definition| CustomExtraction {
                name: definition.name.clone(),
                selector: definition.selector.clone(),
                attribute: definition.attribute.clone(),
                method: definition.method,
                output: definition.output,
                source: input.source.into(),
                engine: match definition.method {
                    crate::ExtractionMethod::Css => "scraper-html5-css",
                    crate::ExtractionMethod::XPath => "sxd-xpath-1.0-html-dom",
                    crate::ExtractionMethod::Regex => "rust-regex",
                }
                .into(),
                ..Default::default()
            })
            .collect()
    };
    let prototype = ExtractionSourceObservation {
        engine: EXTRACTION_SOURCE_ENGINE.into(),
        source: input.source.into(),
        status: input.status,
        evaluation: ExtractionEvaluationStatus::NotRun,
        decode_status: input.decode_status,
        document_url: input.document_url.map(str::to_owned),
        observed_generation: input.observed_generation,
        observed_at: input.observed_at.map(str::to_owned),
        input_sha256: input.html.map(|html| digest(html.as_bytes())),
        input_utf8_bytes: input.html.map(str::len),
        source_revision: None,
        reasons: input.reasons,
    };
    for result in &mut results {
        let mut row = prototype.clone();
        row.evaluation = if input.html.is_none() {
            ExtractionEvaluationStatus::NotRun
        } else if result.error.is_some() {
            ExtractionEvaluationStatus::Error
        } else {
            ExtractionEvaluationStatus::Evaluated
        };
        // This fixed tuple contains only strings, integers and options.
        row.source_revision =
            Some(extraction_source_revision(&row).expect("Extraction source binding serializes"));
        result.source_observation = Some(row);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExtractionMethod, ExtractionOutput};

    fn config(selector: &str, method: ExtractionMethod) -> CustomExtractionConfig {
        CustomExtractionConfig {
            name: "rule".into(),
            selector: selector.into(),
            method,
            ..Default::default()
        }
    }
    fn evaluate(
        html: Option<&str>,
        status: ExtractionSourceStatus,
        config: &[CustomExtractionConfig],
    ) -> Vec<CustomExtraction> {
        extract_observed(
            config,
            ExtractionInputContext {
                html,
                source: "raw_html",
                status,
                decode_status: ContentCaptureDecodeStatus::Exact,
                document_url: Some("https://fixture.test/final"),
                observed_generation: Some(7),
                observed_at: Some("2026-10-21T12:34:56Z"),
                reasons: if status == ExtractionSourceStatus::Complete {
                    vec![]
                } else {
                    vec!["test_partial".into()]
                },
            },
        )
    }
    fn observation(result: &CustomExtraction) -> &ExtractionSourceObservation {
        result.source_observation.as_ref().unwrap()
    }
    fn rebind(result: &mut CustomExtraction) {
        let row = result.source_observation.as_mut().unwrap();
        row.source_revision = Some(extraction_source_revision(row).unwrap());
    }
    fn page(result: CustomExtraction) -> PageRecord {
        PageRecord {
            url: "https://fixture.test/request".into(),
            final_url: "https://fixture.test/final".into(),
            fetched_at: "2026-10-21T12:34:56Z".into(),
            custom_extractions: vec![result],
            ..Default::default()
        }
    }

    #[test]
    fn exact_zero_partial_zero_and_unavailable_are_distinct() {
        let cfg = [config(".absent", ExtractionMethod::Css)];
        let full = evaluate(Some("<p>none</p>"), ExtractionSourceStatus::Complete, &cfg).remove(0);
        let partial = evaluate(Some("<p>none"), ExtractionSourceStatus::Incomplete, &cfg).remove(0);
        let missing = evaluate(None, ExtractionSourceStatus::Unavailable, &cfg).remove(0);
        assert_eq!(full.matched_count, Some(0));
        assert_eq!(observation(&full).exact_count(&full), Some(0));
        assert_eq!(observation(&full).count_scope(&full), "complete_source");
        assert_eq!(partial.matched_count, Some(0));
        assert_eq!(observation(&partial).exact_count(&partial), None);
        assert_eq!(
            observation(&partial).count_scope(&partial),
            "evaluated_input_only"
        );
        assert_eq!(missing.matched_count, None);
        assert!(missing.values.is_empty() && !missing.truncated && missing.error.is_none());
        assert_eq!(
            observation(&missing).evaluation,
            ExtractionEvaluationStatus::NotRun
        );
        assert_eq!(observation(&missing).count_scope(&missing), "not_evaluated");
        assert!(observation(&missing).source_revision.is_some());
        for result in [full, partial, missing] {
            validate_page_extraction_observations(&page(result)).unwrap();
        }
    }

    #[test]
    fn result_budget_is_independent_from_complete_source_count() {
        let mut cfg = config("p", ExtractionMethod::Css);
        cfg.max_results = 2;
        let result = evaluate(
            Some("<p>a</p><p>b</p><p>c</p>"),
            ExtractionSourceStatus::Complete,
            &[cfg],
        )
        .remove(0);
        assert_eq!(result.values, ["a", "b"]);
        assert!(result.truncated);
        assert_eq!(result.matched_count, Some(3));
        assert_eq!(observation(&result).exact_count(&result), Some(3));
    }

    #[test]
    fn scalar_zero_false_and_empty_are_values_not_an_empty_nodeset() {
        let mut configs = vec![];
        for selector in [
            "count(//absent)",
            "boolean(//absent)",
            "string(//absent)",
            "//absent",
        ] {
            configs.push(config(selector, ExtractionMethod::XPath));
        }
        let results = evaluate(
            Some("<html><body/></html>"),
            ExtractionSourceStatus::Complete,
            &configs,
        );
        for (result, expected) in results[..3].iter().zip(["0", "false", ""]) {
            assert_eq!(result.values, [expected]);
            assert_eq!(observation(result).exact_count(result), Some(1));
        }
        assert!(results[3].values.is_empty());
        assert_eq!(observation(&results[3]).exact_count(&results[3]), Some(0));
    }

    #[test]
    fn partial_last_child_scalar_and_anchored_regex_do_not_claim_document_counts() {
        let configs = [
            config("p:last-child", ExtractionMethod::Css),
            config("count(//p)", ExtractionMethod::XPath),
            config("^<p>first</p>$", ExtractionMethod::Regex),
        ];
        let prefix = evaluate(
            Some("<p>first</p>"),
            ExtractionSourceStatus::Incomplete,
            &configs,
        );
        let complete = evaluate(
            Some("<p>first</p><p>last</p>"),
            ExtractionSourceStatus::Complete,
            &configs,
        );
        assert_ne!(prefix[0].values, complete[0].values);
        assert_eq!(prefix[1].values, ["1"]);
        assert_eq!(complete[1].values, ["2"]);
        assert_eq!(prefix[2].matched_count, Some(1));
        assert_eq!(complete[2].matched_count, Some(0));
        for result in prefix {
            assert_eq!(observation(&result).exact_count(&result), None);
            assert_eq!(
                observation(&result).count_scope(&result),
                "evaluated_input_only"
            );
        }
    }

    #[test]
    fn complete_source_can_have_evaluation_error_without_an_exact_count() {
        let result = evaluate(
            Some("<p>x</p>"),
            ExtractionSourceStatus::Complete,
            &[config("/invalid[", ExtractionMethod::XPath)],
        )
        .remove(0);
        assert!(result.error.is_some());
        assert_eq!(result.matched_count, None);
        assert_eq!(
            observation(&result).status,
            ExtractionSourceStatus::Complete
        );
        assert_eq!(
            observation(&result).evaluation,
            ExtractionEvaluationStatus::Error
        );
        assert_eq!(
            observation(&result).count_scope(&result),
            "evaluation_failed"
        );
        assert_eq!(observation(&result).exact_count(&result), None);
        validate_page_extraction_observations(&page(result)).unwrap();
    }

    #[test]
    fn empty_input_hash_and_legacy_serialization_preserve_unknown() {
        let cfg = config("p", ExtractionMethod::Css);
        let result = evaluate(
            Some(""),
            ExtractionSourceStatus::Complete,
            std::slice::from_ref(&cfg),
        )
        .remove(0);
        assert_eq!(observation(&result).input_utf8_bytes, Some(0));
        assert_eq!(
            observation(&result).input_sha256.as_deref(),
            Some(digest(b"").as_str())
        );
        let pure =
            crate::extraction_engine::extract_configured("<p>x</p>", &cfg, "raw_html").unwrap();
        assert!(pure.source_observation.is_none());
        assert!(
            serde_json::to_value(&pure)
                .unwrap()
                .get("source_observation")
                .is_none()
        );
        let legacy: CustomExtraction =
            serde_json::from_value(serde_json::json!({"name":"old","values":["x"]})).unwrap();
        assert!(legacy.source_observation.is_none());
        validate_page_extraction_observations(&page(legacy)).unwrap();
    }

    #[test]
    fn tampered_metadata_and_counts_are_rejected_even_if_revision_is_recomputed() {
        let original = evaluate(
            Some(""),
            ExtractionSourceStatus::Complete,
            &[config("p", ExtractionMethod::Css)],
        )
        .remove(0);
        let changes: Vec<fn(&mut CustomExtraction)> = vec![
            |r| r.source_observation.as_mut().unwrap().input_sha256 = Some("0".repeat(64)),
            |r| r.source_observation.as_mut().unwrap().source = "rendered_html".into(),
            |r| {
                r.source_observation.as_mut().unwrap().observed_at =
                    Some("2026-10-21T13:00:00Z".into())
            },
            |r| r.source_observation.as_mut().unwrap().observed_at = None,
            |r| {
                r.source_observation.as_mut().unwrap().decode_status =
                    ContentCaptureDecodeStatus::Lossy
            },
            |r| r.source_observation.as_mut().unwrap().reasons = vec!["partial".into()],
            |r| r.source_observation.as_mut().unwrap().engine = "other-engine".into(),
            |r| r.source_observation.as_mut().unwrap().reasons = vec!["x".repeat(257)],
            |r| r.source_observation.as_mut().unwrap().reasons = vec!["x".into(); 17],
            |r| {
                r.source_observation.as_mut().unwrap().document_url =
                    Some("https://other.test/".into())
            },
            |r| r.matched_count = None,
            |r| r.values.push("invented".into()),
            |r| r.truncated = true,
            |r| r.error = Some("invented failure".into()),
            |r| r.engine = "other-evaluator".into(),
        ];
        for (index, change) in changes.into_iter().enumerate() {
            let mut altered = original.clone();
            change(&mut altered);
            rebind(&mut altered);
            assert!(
                validate_page_extraction_observations(&page(altered)).is_err(),
                "tamper {index}"
            );
        }
        let mut altered = original;
        altered.source_observation.as_mut().unwrap().source_revision = Some("0".repeat(64));
        assert!(validate_page_extraction_observations(&page(altered)).is_err());
    }

    #[test]
    fn raw_metadata_is_validated_and_inherited_generation_is_not_rewritten() {
        let result = evaluate(
            Some("<p>x</p>"),
            ExtractionSourceStatus::Complete,
            &[config("p", ExtractionMethod::Css)],
        )
        .remove(0);
        let mut saved = page(result);
        saved.raw_metadata = Some(crate::RawMetadata::from(&saved));
        saved.raw_metadata.as_mut().unwrap().custom_extractions[0]
            .source_observation
            .as_mut()
            .unwrap()
            .source_revision = Some("0".repeat(64));
        assert!(validate_page_extraction_observations(&saved).is_err());
        saved.raw_metadata = None;
        validate_page_extraction_observations(&saved).unwrap();
        assert_eq!(
            observation(&saved.custom_extractions[0]).observed_generation,
            Some(7)
        );
    }

    #[test]
    fn unsafe_producer_context_downgrades_and_all_unavailable_rules_survive() {
        let configs = [
            config("p", ExtractionMethod::Css),
            config("x", ExtractionMethod::Regex),
        ];
        let result = extract_observed(
            &configs,
            ExtractionInputContext {
                html: Some("<p>x</p>"),
                source: "raw_html",
                status: ExtractionSourceStatus::Complete,
                decode_status: ContentCaptureDecodeStatus::Lossy,
                document_url: None,
                observed_generation: None,
                observed_at: Some("not-a-time"),
                reasons: vec![],
            },
        );
        assert_eq!(result.len(), 2);
        for row in &result {
            assert_eq!(observation(row).status, ExtractionSourceStatus::Incomplete);
            assert_eq!(observation(row).exact_count(row), None);
        }
        let unavailable = evaluate(None, ExtractionSourceStatus::Unavailable, &configs);
        assert_eq!(unavailable.len(), configs.len());
        for row in unavailable {
            validate_extraction_source_observation(&row, observation(&row)).unwrap();
        }
        let mut cfg = config("p", ExtractionMethod::Css);
        cfg.output = ExtractionOutput::OuterHtml;
        let malformed = extract_observed(
            &[cfg],
            ExtractionInputContext {
                html: Some("<p>x</p>"),
                source: "raw_html",
                status: ExtractionSourceStatus::Complete,
                decode_status: ContentCaptureDecodeStatus::Exact,
                document_url: Some("https://fixture.test/final"),
                observed_generation: None,
                observed_at: Some("2026-10-21T12:34:56Z"),
                reasons: vec!["x".into(); 17],
            },
        );
        assert_eq!(
            observation(&malformed[0]).status,
            ExtractionSourceStatus::Incomplete
        );
        validate_page_extraction_observations(&page(malformed[0].clone())).unwrap();
    }

    #[test]
    fn native_export_missing_rules_are_multiset_unknown_without_mutating_saved_rows() {
        let cfg = config(".absent", ExtractionMethod::Css);
        let saved = evaluate(
            Some("<p>none</p>"),
            ExtractionSourceStatus::Complete,
            std::slice::from_ref(&cfg),
        )
        .remove(0);
        let report = CrawlReport {
            config: crate::CrawlConfig {
                custom_extractions: vec![cfg.clone(), cfg],
                ..Default::default()
            },
            pages: vec![page(saved)],
            ..Default::default()
        };
        let before = serde_json::to_vec(&report).unwrap();
        let text = crate::export_custom_extractions_csv(&report).unwrap();
        let mut reader = csv::Reader::from_reader(text.as_bytes());
        let headers = reader.headers().unwrap().clone();
        assert_eq!(
            &headers.iter().skip(15).take(13).collect::<Vec<_>>(),
            &[
                "Source Status",
                "Evaluation Status",
                "Decode Status",
                "Exact Match Count",
                "Count Scope",
                "Source Reasons",
                "Document URL",
                "Observed Generation",
                "Observed At",
                "Input SHA256",
                "Input UTF8 Bytes",
                "Source Revision",
                "Source Engine"
            ]
        );
        let rows = reader.records().map(|row| row.unwrap()).collect::<Vec<_>>();
        assert_eq!(rows.len(), 2);
        assert_eq!(&rows[0][15], "complete");
        assert_eq!(&rows[0][18], "0");
        assert_eq!(&rows[0][12], "false");
        assert_eq!(&rows[1][15], "unknown");
        assert_eq!(&rows[1][18], "");
        assert_eq!(&rows[1][11], "");
        assert_eq!(&rows[1][12], "");
        assert_eq!(&rows[1][19], "unknown");
        assert_eq!(&rows[1][20], "[\"missing_saved_observation\"]");
        assert_eq!(serde_json::to_vec(&report).unwrap(), before);
    }

    #[test]
    fn store_and_selected_recrawl_admit_legacy_but_reject_tampered_present_observation() {
        let cfg = config(".absent", ExtractionMethod::Css);
        let saved = evaluate(
            Some("<p>none</p>"),
            ExtractionSourceStatus::Complete,
            std::slice::from_ref(&cfg),
        )
        .remove(0);
        let mut report = CrawlReport {
            config: crate::CrawlConfig {
                seed_urls: vec!["https://fixture.test/request".into()],
                custom_extractions: vec![cfg],
                ..Default::default()
            },
            pages: vec![page(saved)],
            ..Default::default()
        };
        let directory = tempfile::tempdir().unwrap();
        let store = crate::CrawlStore::open(directory.path().join("crawl.sqlite")).unwrap();
        store.save_report(&report).unwrap();
        assert!(
            store.load_report().unwrap().unwrap().pages[0].custom_extractions[0]
                .source_observation
                .is_some()
        );
        let prepare = |report: &CrawlReport| {
            crate::prepare_selected_recrawl(
                crate::SelectedRecrawlParent {
                    job_id: "parent".into(),
                    checkpoint_revision: None,
                    source_kind: crate::RecrawlSourceKind::StoredJson,
                },
                &serde_json::to_vec(report).unwrap(),
                &["https://fixture.test/request".into()],
                Some(1),
            )
        };
        assert!(prepare(&report).is_ok());
        report.pages[0].custom_extractions[0]
            .source_observation
            .as_mut()
            .unwrap()
            .source_revision = Some("0".repeat(64));
        assert!(store.save_report(&report).is_err());
        assert!(prepare(&report).is_err());
        assert!(crate::export_custom_extractions_csv(&report).is_err());
        report.pages[0].custom_extractions[0].source_observation = None;
        store.save_report(&report).unwrap();
        assert!(prepare(&report).is_ok());
        assert!(
            store.load_report().unwrap().unwrap().pages[0].custom_extractions[0]
                .source_observation
                .is_none()
        );
    }
}
