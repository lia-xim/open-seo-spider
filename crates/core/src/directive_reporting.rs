//! Opt-in presentation of saved observations. This module has no crawl/store I/O.
use crate::{AuditIssue, CrawlConfig, CrawlReport, PageRecord};
use serde::{Deserialize, Deserializer, Serialize};
use std::{borrow::Cow, collections::HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DirectiveReportingConfig {
    pub version: u8,
    pub respect_noindex: bool,
    pub respect_canonical: bool,
    pub respect_next_prev: bool,
}

impl Default for DirectiveReportingConfig {
    fn default() -> Self {
        Self {
            version: 1,
            respect_noindex: false,
            respect_canonical: false,
            respect_next_prev: false,
        }
    }
}

impl<'de> Deserialize<'de> for DirectiveReportingConfig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            version: u8,
            respect_noindex: bool,
            respect_canonical: bool,
            respect_next_prev: bool,
        }
        struct ObjectProfile;
        impl<'de> serde::de::Visitor<'de> for ObjectProfile {
            type Value = Wire;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a strict directive reporting object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<Wire, A::Error> {
                Wire::deserialize(serde::de::value::MapAccessDeserializer::new(map))
            }
        }
        let value = deserializer.deserialize_map(ObjectProfile)?;
        let config = Self {
            version: value.version,
            respect_noindex: value.respect_noindex,
            respect_canonical: value.respect_canonical,
            respect_next_prev: value.respect_next_prev,
        };
        config.validate().map_err(serde::de::Error::custom)?;
        Ok(config)
    }
}

impl DirectiveReportingConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.version == 1, "Unsupported directive reporting version");
        Ok(())
    }
    pub fn active(&self) -> bool {
        self.respect_noindex || self.respect_canonical || self.respect_next_prev
    }
    /// Explicit all-false is semantically absence and serializes as legacy absence.
    pub fn is_inactive(value: &Option<Self>) -> bool {
        value.as_ref().is_none_or(|profile| !profile.active())
    }
}

/// Use the same scoped robots evaluator and document identity as the audit.
/// Incomplete/failed/non-200 and other non-indexable rows are not blanket hidden.
pub fn directive_page_visible(config: &CrawlConfig, page: &PageRecord) -> bool {
    let Some(profile) = &config.directive_reporting else {
        return true;
    };
    !(profile.respect_noindex
        && crate::applicable_robot_directives(page)
            .iter()
            .any(|value| matches!(value.as_str(), "noindex" | "none"))
        || profile.respect_canonical && page.canonical.iter().any(|url| url != page.document_url())
        || profile.respect_next_prev && !page.pagination_prev.is_empty())
}

/// Shared presentation selection for source-owned link/resource occurrences.
/// None preserves old JSON/CSV/token formulas for an inactive saved profile.
pub fn source_occurrence_reporting_identity(
    config: &CrawlConfig,
    requested: Option<crate::ReportingMode>,
) -> anyhow::Result<Option<crate::ReportingIdentity>> {
    if let Some(profile) = &config.directive_reporting {
        profile.validate()?;
    }
    let active = !DirectiveReportingConfig::is_inactive(&config.directive_reporting);
    if !active && requested.is_none() {
        return Ok(None);
    }
    Ok(Some(crate::ReportingIdentity {
        engine: "open-seo-spider/source-occurrence-reporting-v1".into(),
        mode: requested.unwrap_or(if active {
            crate::ReportingMode::Reported
        } else {
            crate::ReportingMode::Raw
        }),
        directive_reporting: config
            .directive_reporting
            .clone()
            .filter(DirectiveReportingConfig::active),
    }))
}

/// URL-owned issues are hidden only when every captured observation of that URL
/// is hidden. A visible duplicate keeps its issues; an orphan URL is never hidden.
/// Shared by API issue rows, detached report projections and visibility counters.
pub fn directive_hidden_issue_urls<'a>(
    config: &CrawlConfig,
    pages: &'a [PageRecord],
) -> HashSet<&'a str> {
    let mut hidden = HashSet::new();
    if DirectiveReportingConfig::is_inactive(&config.directive_reporting) {
        return hidden;
    }
    let mut visible = HashSet::new();
    for page in pages {
        if directive_page_visible(config, page) {
            visible.insert(page.url.as_str());
        } else {
            hidden.insert(page.url.as_str());
        }
    }
    hidden.retain(|url| !visible.contains(url));
    hidden
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportingVisibility {
    pub engine: &'static str,
    pub captured_pages: usize,
    pub visible_pages: usize,
    pub hidden_pages: usize,
    pub captured_issues: usize,
    pub visible_issues: usize,
    pub hidden_issues: usize,
    pub captured_source_links: usize,
    pub visible_source_links: usize,
    pub hidden_source_links: usize,
}

pub fn reporting_visibility(
    config: &CrawlConfig,
    pages: &[PageRecord],
    issues: &[AuditIssue],
) -> Option<ReportingVisibility> {
    if DirectiveReportingConfig::is_inactive(&config.directive_reporting) {
        return None;
    }
    let hidden_rows = pages
        .iter()
        .filter(|page| !directive_page_visible(config, page))
        .count();
    let hidden = directive_hidden_issue_urls(config, pages);
    let visible_issues = issues
        .iter()
        .filter(|issue| !hidden.contains(issue.url.as_str()))
        .count();
    let captured_links = pages.iter().map(|page| page.links.len()).sum();
    let visible_links = pages
        .iter()
        .filter(|page| directive_page_visible(config, page))
        .map(|page| page.links.len())
        .sum();
    Some(ReportingVisibility {
        engine: "open-seo-spider/directive-reporting-v1",
        captured_pages: pages.len(),
        visible_pages: pages.len() - hidden_rows,
        hidden_pages: hidden_rows,
        captured_issues: issues.len(),
        visible_issues,
        hidden_issues: issues.len() - visible_issues,
        captured_source_links: captured_links,
        visible_source_links: visible_links,
        hidden_source_links: captured_links - visible_links,
    })
}

/// Detached row projection only. Never save this back into the raw job/store.
/// Summary remains captured crawl statistics, not a recomputed crawl result.
pub fn reported_report(report: &CrawlReport) -> Cow<'_, CrawlReport> {
    if DirectiveReportingConfig::is_inactive(&report.config.directive_reporting) {
        return Cow::Borrowed(report);
    }
    let hidden = directive_hidden_issue_urls(&report.config, &report.pages);
    let mut projected = report.clone();
    projected
        .pages
        .retain(|page| directive_page_visible(&report.config, page));
    projected
        .issues
        .retain(|issue| !hidden.contains(issue.url.as_str()));
    Cow::Owned(projected)
}

/// Analyze complete captured target facts before suppressing source-owned rows.
pub fn reported_fragments(report: &CrawlReport) -> Vec<crate::FragmentObservation> {
    let hidden = directive_hidden_issue_urls(&report.config, &report.pages);
    crate::validate_fragments(report)
        .into_iter()
        .filter(|row| !hidden.contains(row.source_url.as_str()))
        .collect()
}

/// Redirect hops retain their original request owner and response identities.
pub fn reported_responses(report: &CrawlReport) -> Vec<crate::ResponseObservation> {
    let visible = reported_report(report);
    crate::response_observations(visible.as_ref())
}

/// Public visible rows have a distinct envelope, never a raw snapshot identity.
pub fn reporting_json(report: &CrawlReport) -> anyhow::Result<serde_json::Value> {
    let visible = reported_report(report);
    let counts = reporting_visibility(&report.config, &report.pages, &report.issues)
        .unwrap_or_else(|| ReportingVisibility {
            engine: "open-seo-spider/directive-reporting-v1",
            captured_pages: report.pages.len(),
            visible_pages: report.pages.len(),
            hidden_pages: 0,
            captured_issues: report.issues.len(),
            visible_issues: report.issues.len(),
            hidden_issues: 0,
            captured_source_links: report.pages.iter().map(|page| page.links.len()).sum(),
            visible_source_links: report.pages.iter().map(|page| page.links.len()).sum(),
            hidden_source_links: 0,
        });
    Ok(serde_json::json!({
        "engine":"open-seo-spider/directive-reporting-v1",
        "kind":"visible_report_rows_not_raw_snapshot",
        "directive_reporting":report.config.directive_reporting,
        "reporting_visibility":counts,
        "captured_crawl_summary":report.summary,
        "pages":visible.pages,
        "issues":visible.issues,
        "qualification":"Source-owned rows are filtered; retained page/link relationship facts are not rewritten. Raw project, crawl counters, frontier and explicit source/ledger APIs remain complete. Analysis rows use the same visibility policy while their metrics retain the complete capture basis."
    }))
}

/// Counts are appended only for active profiles, preserving old CSV bytes.
pub fn reporting_csv(report: &CrawlReport, csv: String) -> anyhow::Result<String> {
    let Some(counts) = reporting_visibility(&report.config, &report.pages, &report.issues) else {
        return Ok(csv);
    };
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(csv.as_bytes());
    let mut writer = csv::Writer::from_writer(Vec::new());
    let labels = [
        "Captured Pages",
        "Visible Pages",
        "Hidden Pages",
        "Captured Issues",
        "Visible Issues",
        "Hidden Issues",
    ];
    let values = [
        counts.captured_pages,
        counts.visible_pages,
        counts.hidden_pages,
        counts.captured_issues,
        counts.visible_issues,
        counts.hidden_issues,
    ]
    .map(|value| value.to_string());
    for (index, record) in reader.records().enumerate() {
        let mut record = record?;
        if index == 0 {
            for label in labels {
                record.push_field(label);
            }
        } else {
            for value in &values {
                record.push_field(value);
            }
        }
        writer.write_record(&record)?;
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LinkRecord, RobotsDirective};

    fn pages() -> Vec<PageRecord> {
        let plain = PageRecord {
            url: "https://fixture.test/plain".into(),
            final_url: "https://fixture.test/plain".into(),
            ..Default::default()
        };
        let mut noindex = plain.clone();
        noindex.url = "https://fixture.test/noindex".into();
        noindex.final_url = noindex.url.clone();
        noindex.meta_robots = vec!["noindex".into()];
        let mut canonical = plain.clone();
        canonical.url = "https://fixture.test/canonical".into();
        canonical.final_url = canonical.url.clone();
        canonical.canonical = vec![plain.url.clone()];
        let mut prev = plain.clone();
        prev.url = "https://fixture.test/prev".into();
        prev.final_url = prev.url.clone();
        prev.pagination_prev = vec![plain.url.clone()];
        vec![plain, noindex, canonical, prev]
    }
    #[test]
    fn eight_independent_switch_combinations_keep_raw_links_and_crawl_configuration() {
        let mut rows = pages();
        for page in &mut rows {
            page.links.push(LinkRecord {
                url: "https://fixture.test/outlink".into(),
                ..Default::default()
            });
        }
        let raw = serde_json::to_vec(&rows).unwrap();
        for mask in 0..8 {
            let config = CrawlConfig {
                directive_reporting: Some(DirectiveReportingConfig {
                    version: 1,
                    respect_noindex: mask & 1 != 0,
                    respect_canonical: mask & 2 != 0,
                    respect_next_prev: mask & 4 != 0,
                }),
                ..Default::default()
            };
            assert!(directive_page_visible(&config, &rows[0]));
            for (index, bit) in [(1, 1), (2, 2), (3, 4)] {
                assert_eq!(
                    directive_page_visible(&config, &rows[index]),
                    mask & bit == 0
                );
            }
            assert_eq!(serde_json::to_vec(&rows).unwrap(), raw);
            assert!(config.crawl_links);
            assert!(config.resource_discovery.is_none());
        }
    }
    #[test]
    fn noindex_uses_scoped_audit_evaluator_not_nonindexable_or_stale_effective_cache() {
        let config = CrawlConfig {
            directive_reporting: Some(DirectiveReportingConfig {
                respect_noindex: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut row = pages().remove(0);
        row.indexable = false;
        row.status = Some(404);
        row.effective_robots = vec!["noindex".into()];
        assert!(directive_page_visible(&config, &row));
        row.indexability_agent = "googlebot".into();
        row.robots_directives = vec![RobotsDirective {
            agent: Some("bingbot".into()),
            directives: vec!["noindex".into()],
            ..Default::default()
        }];
        assert!(directive_page_visible(&config, &row));
        row.robots_directives[0].agent = Some("googlebot".into());
        assert!(!directive_page_visible(&config, &row));
    }
    #[test]
    fn canonical_uses_final_or_successful_render_document_identity_and_prev_is_independent() {
        let config = CrawlConfig {
            directive_reporting: Some(DirectiveReportingConfig {
                respect_canonical: true,
                respect_next_prev: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut row = pages().remove(0);
        row.canonical = vec![row.final_url.clone()];
        row.pagination_next = vec!["https://fixture.test/page2".into()];
        assert!(directive_page_visible(&config, &row));
        row.rendered = true;
        row.rendered_url = Some("https://fixture.test/rendered".into());
        row.canonical = vec![row.rendered_url.clone().unwrap()];
        assert!(directive_page_visible(&config, &row));
        row.pagination_prev = vec!["https://fixture.test/page1".into()];
        assert!(!directive_page_visible(&config, &row));
    }
    #[test]
    fn strict_saved_profile_default_none_null_and_all_false_keep_legacy_config_bytes() {
        let raw = serde_json::to_value(CrawlConfig::default()).unwrap();
        assert!(raw.get("directive_reporting").is_none());
        for profile in [
            serde_json::Value::Null,
            serde_json::json!({"version":1,"respect_noindex":false,"respect_canonical":false,"respect_next_prev":false}),
        ] {
            let mut value = raw.clone();
            value["directive_reporting"] = profile;
            let config: CrawlConfig = serde_json::from_value(value).unwrap();
            assert_eq!(serde_json::to_value(config).unwrap(), raw);
        }
        for profile in [
            serde_json::json!({}),
            serde_json::json!([1, true, false, false]),
            serde_json::json!({"version":2,"respect_noindex":true,"respect_canonical":false,"respect_next_prev":false}),
            serde_json::json!({"version":1,"respect_noindex":"true","respect_canonical":false,"respect_next_prev":false}),
            serde_json::json!({"version":1,"respect_noindex":true,"respect_canonical":false,"respect_next_prev":false,"follow":true}),
        ] {
            let mut value = raw.clone();
            value["directive_reporting"] = profile;
            assert!(serde_json::from_value::<CrawlConfig>(value).is_err());
        }
    }
    #[test]
    fn default_standard_csv_and_summary_raw_bytes_are_preserved_and_duplicate_rows_count() {
        let mut report = CrawlReport {
            config: CrawlConfig::default(),
            pages: pages(),
            issues: Vec::new(),
            summary: crate::CrawlSummary::default(),
            warnings: Vec::new(),
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
            sitemap_catalog: None,
            sitemap_coverage_inventory: None,
            sitemap_source_measurements: None,
        };
        let legacy = crate::export_pages_csv(&report).unwrap();
        for profile in [None, Some(DirectiveReportingConfig::default())] {
            report.config.directive_reporting = profile;
            assert_eq!(crate::export_pages_csv(&report).unwrap(), legacy);
            assert!(matches!(reported_report(&report), Cow::Borrowed(_)));
        }
        report.config.directive_reporting = Some(DirectiveReportingConfig {
            respect_noindex: true,
            ..Default::default()
        });
        report.pages.push(report.pages[1].clone());
        let raw = serde_json::to_vec(&report).unwrap();
        let counts = reporting_visibility(&report.config, &report.pages, &report.issues).unwrap();
        assert_eq!(counts.captured_pages, 5);
        assert_eq!(counts.hidden_pages, 2);
        assert_eq!(counts.visible_pages, 3);
        let visible = reporting_json(&report).unwrap();
        assert_eq!(visible["pages"].as_array().unwrap().len(), 3);
        let csv = crate::export_pages_csv(&report).unwrap();
        assert!(csv.lines().next().unwrap().contains("Captured Pages"));
        assert_eq!(serde_json::to_vec(&report).unwrap(), raw);
        let mut same_url_visible = report.pages[1].clone();
        same_url_visible.meta_robots.clear();
        report.pages.push(same_url_visible);
        let projected = reported_report(&report);
        assert_eq!(projected.pages.len(), 4);
        assert_eq!(
            reporting_visibility(&report.config, &report.pages, &report.issues)
                .unwrap()
                .visible_pages,
            projected.pages.len()
        );
    }

    #[test]
    fn mixed_duplicate_issues_stay_visible_all_hidden_hide_and_orphans_survive() {
        let mut mixed_hidden = pages()[1].clone();
        mixed_hidden.url = "https://fixture.test/mixed".into();
        mixed_hidden.final_url = mixed_hidden.url.clone();
        let mut mixed_visible = mixed_hidden.clone();
        mixed_visible.meta_robots.clear();
        let mut all_hidden = mixed_hidden.clone();
        all_hidden.url = "https://fixture.test/fully-hidden".into();
        all_hidden.final_url = all_hidden.url.clone();
        let plain = pages()[0].clone();
        let issue = |code: &str, url: &str| AuditIssue {
            code: code.into(),
            severity: crate::Severity::Notice,
            url: url.into(),
            message: "Authored URL-owned issue fixture".into(),
            related_urls: Vec::new(),
        };
        let mut report = CrawlReport {
            config: CrawlConfig::default(),
            pages: vec![
                plain.clone(),
                mixed_hidden,
                mixed_visible,
                all_hidden.clone(),
                all_hidden,
            ],
            issues: vec![
                issue("mixed-retained", "https://fixture.test/mixed"),
                issue("fully-hidden", "https://fixture.test/fully-hidden"),
                issue("orphan-retained", "https://fixture.test/orphan"),
                issue("plain-retained", &plain.url),
            ],
            summary: crate::CrawlSummary::default(),
            warnings: Vec::new(),
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
            sitemap_catalog: None,
            sitemap_coverage_inventory: None,
            sitemap_source_measurements: None,
        };
        let legacy_csv = crate::export_issues_csv(&report).unwrap();
        let legacy_config = serde_json::to_value(CrawlConfig::default()).unwrap();
        for profile in [
            None,
            Some(serde_json::Value::Null),
            Some(serde_json::json!({
                "version":1, "respect_noindex":false, "respect_canonical":false, "respect_next_prev":false
            })),
        ] {
            let mut config = legacy_config.clone();
            if let Some(profile) = profile {
                config["directive_reporting"] = profile;
            }
            report.config = serde_json::from_value(config).unwrap();
            assert!(directive_hidden_issue_urls(&report.config, &report.pages).is_empty());
            assert_eq!(crate::export_issues_csv(&report).unwrap(), legacy_csv);
            assert_eq!(reported_report(&report).issues.len(), 4);
            assert!(reporting_visibility(&report.config, &report.pages, &report.issues).is_none());
        }
        report.config.directive_reporting = Some(DirectiveReportingConfig {
            respect_noindex: true,
            ..Default::default()
        });
        let raw = serde_json::to_vec(&report).unwrap();
        let hidden = directive_hidden_issue_urls(&report.config, &report.pages);
        assert_eq!(hidden, HashSet::from(["https://fixture.test/fully-hidden"]));
        // This is the exact public API row predicate, using the shared helper.
        let api_rows: Vec<_> = report
            .issues
            .iter()
            .filter(|row| !hidden.contains(row.url.as_str()))
            .collect();
        let projected = reported_report(&report);
        let expected = ["mixed-retained", "orphan-retained", "plain-retained"];
        assert_eq!(
            api_rows
                .iter()
                .map(|row| row.code.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            projected
                .issues
                .iter()
                .map(|row| row.code.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        let counts = reporting_visibility(&report.config, &report.pages, &report.issues).unwrap();
        assert_eq!(
            (
                counts.captured_pages,
                counts.visible_pages,
                counts.hidden_pages
            ),
            (5, 2, 3)
        );
        assert_eq!(
            (
                counts.captured_issues,
                counts.visible_issues,
                counts.hidden_issues
            ),
            (4, 3, 1)
        );
        let exported = reporting_json(&report).unwrap();
        assert_eq!(exported["issues"].as_array().unwrap().len(), 3);
        let csv = crate::export_issues_csv(&report).unwrap();
        let rows: Vec<_> = csv::Reader::from_reader(csv.as_bytes())
            .records()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            rows.iter()
                .map(|row| row.get(1).unwrap())
                .collect::<Vec<_>>(),
            expected
        );
        assert!(
            rows.iter()
                .all(|row| row.get(9) == Some("3") && row.get(10) == Some("1"))
        );
        assert_eq!(serde_json::to_vec(&report).unwrap(), raw);
    }

    #[test]
    fn fragment_reporting_keeps_hidden_target_facts_and_default_csv_bytes() {
        let mut rows = pages();
        for row in &mut rows {
            row.content_type = "text/html".into();
            row.status = Some(200);
            row.final_status = Some(200);
        }
        let seeds = [crate::normalize_url(&rows[0].url, None).unwrap()];
        crate::extract_html(
            &mut rows[0],
            "<a href='/noindex#present'>Known</a>",
            &seeds,
            false,
        );
        crate::extract_html(
            &mut rows[1],
            "<meta name='robots' content='noindex'><h1 id='present'>Target</h1><a href='/plain#absent'>Hidden source</a>",
            &seeds,
            false,
        );
        let mut report = CrawlReport {
            config: CrawlConfig {
                validate_fragments: true,
                ..Default::default()
            },
            pages: rows,
            issues: vec![],
            summary: Default::default(),
            warnings: vec![],
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
            sitemap_catalog: None,
            sitemap_coverage_inventory: None,
            sitemap_source_measurements: None,
        };
        let raw = crate::validate_fragments(&report);
        assert_eq!(raw.len(), 2);
        let csv = crate::export_fragments_csv(&report).unwrap();
        report.config.directive_reporting = Some(DirectiveReportingConfig::default());
        assert_eq!(crate::export_fragments_csv(&report).unwrap(), csv);
        report.config.directive_reporting = Some(DirectiveReportingConfig {
            respect_noindex: true,
            ..Default::default()
        });
        let saved = serde_json::to_vec(&report).unwrap();
        let visible = reported_fragments(&report);
        assert_eq!(visible.len(), 1);
        let expected = raw
            .iter()
            .find(|row| row.source_url == report.pages[0].url)
            .unwrap();
        assert_eq!(
            serde_json::to_value(&visible[0]).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
        assert!(
            visible
                .iter()
                .all(|row| row.source_url != report.pages[1].url)
        );
        assert_eq!(visible[0].status, crate::FragmentStatus::MatchedId);
        let csv = crate::export_fragments_csv(&report).unwrap();
        assert_eq!(
            csv::Reader::from_reader(csv.as_bytes()).records().count(),
            1
        );
        assert!(csv.lines().next().unwrap().contains("Captured Pages"));
        assert_eq!(serde_json::to_vec(&report).unwrap(), saved);
    }

    #[test]
    fn response_reporting_filters_owner_observations_preserving_redirect_hop_facts() {
        let mut rows = pages();
        for row in &mut rows {
            row.status = Some(200);
            row.final_status = Some(200);
        }
        let hop = crate::RedirectHop {
            url: rows[0].url.clone(),
            status: 302,
            target: rows[1].url.clone(),
        };
        rows[0].redirect_chain.push(hop);
        rows[0].final_url = rows[1].url.clone();
        let mut report = CrawlReport {
            config: CrawlConfig::default(),
            pages: rows,
            issues: vec![],
            summary: Default::default(),
            warnings: vec![],
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
            sitemap_catalog: None,
            sitemap_coverage_inventory: None,
            sitemap_source_measurements: None,
        };
        let raw = crate::response_observations(&report);
        let csv = crate::export_response_observations_csv(&report).unwrap();
        assert_eq!(raw.len(), 5);
        report.config.directive_reporting = Some(DirectiveReportingConfig::default());
        assert_eq!(
            crate::export_response_observations_csv(&report).unwrap(),
            csv
        );
        report.config.directive_reporting = Some(DirectiveReportingConfig {
            respect_noindex: true,
            ..Default::default()
        });
        let saved = serde_json::to_vec(&report).unwrap();
        let view = reported_responses(&report);
        assert_eq!(view.len(), 4);
        assert_eq!(
            serde_json::to_value(&view[0]).unwrap(),
            serde_json::to_value(&raw[0]).unwrap()
        );
        assert_eq!(
            view[0].location.as_deref(),
            Some(report.pages[1].url.as_str())
        );
        assert!(
            view.iter()
                .all(|row| row.requested_url != report.pages[1].url)
        );
        let csv = crate::export_response_observations_csv(&report).unwrap();
        assert_eq!(
            csv::Reader::from_reader(csv.as_bytes()).records().count(),
            4
        );
        assert!(csv.lines().next().unwrap().contains("Captured Pages"));
        assert_eq!(serde_json::to_vec(&report).unwrap(), saved);
    }
}
