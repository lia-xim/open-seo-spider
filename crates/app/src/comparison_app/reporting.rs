//! Presentation index over a complete raw comparison; never filter a join input.
use super::*;
use open_seo_spider_core::{
    ComparisonPageKind, DirectiveReportingConfig, ReportingMode, SavedComparisonCounts,
    directive_page_visible,
};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Identity {
    pub engine: String,
    pub mode: ReportingMode,
    pub before_profile: Option<DirectiveReportingConfig>,
    pub after_profile: Option<DirectiveReportingConfig>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Row {
    pub comparison_index: usize,
    pub comparison_key: String,
    pub before_visible: Option<bool>,
    pub after_visible: Option<bool>,
    pub included: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComparisonReporting {
    pub identity: Identity,
    pub identity_sha256: String,
    pub view_sha256: String,
    pub before_captured_page_observations: usize,
    pub after_captured_page_observations: usize,
    pub before_profile_visible_page_observations: usize,
    pub after_profile_visible_page_observations: usize,
    pub raw_compared_rows: usize,
    pub included_compared_rows: usize,
    pub excluded_compared_rows: usize,
    pub raw_counts: SavedComparisonCounts,
    pub view_counts: SavedComparisonCounts,
    pub rows: Vec<Row>,
}
fn digest<T: Serialize>(value: &T) -> Result<String, SavedComparisonFailure> {
    // Value objects sort keys recursively; arrays retain order. No raw-report
    // hash is reconstructed or replaced by this presentation identity.
    let value = serde_json::to_value(value)
        .map_err(|e| failure("comparison_reporting_identity", e.to_string()))?;
    let raw = serde_json::to_vec(&value)
        .map_err(|e| failure("comparison_reporting_identity", e.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(raw)))
}
fn profile(
    report: &CrawlReport,
) -> Result<Option<DirectiveReportingConfig>, SavedComparisonFailure> {
    if let Some(p) = &report.config.directive_reporting {
        p.validate()
            .map_err(|e| failure("invalid_comparison_reporting_profile", e.to_string()))?;
    }
    Ok(report
        .config
        .directive_reporting
        .as_ref()
        .filter(|p| p.active())
        .cloned())
}
fn counts<'a>(
    pages: impl Iterator<Item = &'a open_seo_spider_core::ComparedSavedPage>,
) -> SavedComparisonCounts {
    let mut c = SavedComparisonCounts::default();
    for p in pages {
        match p.kind {
            ComparisonPageKind::Added => c.added += 1,
            ComparisonPageKind::Removed => c.removed += 1,
            ComparisonPageKind::Changed => c.changed += 1,
            ComparisonPageKind::Unchanged => c.unchanged += 1,
        }
        c.pages_with_unknown_fields += usize::from(!p.unknown_fields.is_empty());
    }
    c
}
fn token(
    result: &SavedComparisonResult,
    view: &ComparisonReporting,
) -> Result<String, SavedComparisonFailure> {
    digest(&json!({"identity_sha256":view.identity_sha256,
        "before_snapshot_sha256":result.before_snapshot.typed_report_sha256,
        "after_snapshot_sha256":result.after_snapshot.typed_report_sha256,
        "config_sha256":result.config_sha256,"rows":view.rows}))
}
pub(crate) fn build(
    result: &SavedComparisonResult,
    before: &CrawlReport,
    after: &CrawlReport,
    requested: Option<ReportingMode>,
    expected_identity: Option<&str>,
    expected_view: Option<&str>,
) -> Result<Option<ComparisonReporting>, SavedComparisonFailure> {
    let before_profile = profile(before)?;
    let after_profile = profile(after)?;
    if requested.is_none() && before_profile.is_none() && after_profile.is_none() {
        if expected_identity.is_some() || expected_view.is_some() {
            return Err(failure(
                "comparison_reporting_changed",
                "Reporting identity is absent in inactive Auto mode",
            ));
        }
        return Ok(None);
    }
    let mode = requested.unwrap_or(ReportingMode::Reported);
    let identity = Identity {
        engine: "open-seo-spider/saved-comparison-reporting-v1".into(),
        mode,
        before_profile,
        after_profile,
    };
    let identity_sha256 = digest(&identity)?;
    if expected_identity.is_some_and(|v| v != identity_sha256) {
        return Err(failure(
            "comparison_reporting_changed",
            "Saved comparison mode/profile identity changed",
        ));
    }
    let side = |r: &open_seo_spider_core::ComparisonPageReference, report: &CrawlReport| {
        let p = report
            .pages
            .get(r.ordinal)
            .filter(|p| p.url == r.original_requested_url)
            .ok_or_else(|| {
                failure(
                    "comparison_reporting_reference",
                    "Raw comparison reference differs from its original report",
                )
            })?;
        Ok::<_, SavedComparisonFailure>(directive_page_visible(&report.config, p))
    };
    let rows = result
        .pages
        .iter()
        .enumerate()
        .map(|(comparison_index, p)| {
            let before_visible = p.before.as_ref().map(|r| side(r, before)).transpose()?;
            let after_visible = p.after.as_ref().map(|r| side(r, after)).transpose()?;
            let included = mode == ReportingMode::Raw
                || before_visible == Some(true)
                || after_visible == Some(true);
            Ok(Row {
                comparison_index,
                comparison_key: p.comparison_key.clone(),
                before_visible,
                after_visible,
                included,
            })
        })
        .collect::<Result<Vec<_>, SavedComparisonFailure>>()?;
    let view_counts = counts(
        rows.iter()
            .filter(|r| r.included)
            .map(|r| &result.pages[r.comparison_index]),
    );
    let included_compared_rows = rows.iter().filter(|r| r.included).count();
    let mut view = ComparisonReporting {
        identity,
        identity_sha256,
        view_sha256: String::new(),
        before_captured_page_observations: before.pages.len(),
        after_captured_page_observations: after.pages.len(),
        before_profile_visible_page_observations: before
            .pages
            .iter()
            .filter(|p| directive_page_visible(&before.config, p))
            .count(),
        after_profile_visible_page_observations: after
            .pages
            .iter()
            .filter(|p| directive_page_visible(&after.config, p))
            .count(),
        raw_compared_rows: rows.len(),
        included_compared_rows,
        excluded_compared_rows: rows.len() - included_compared_rows,
        raw_counts: result.counts.clone(),
        view_counts,
        rows,
    };
    view.view_sha256 = token(result, &view)?;
    if expected_view.is_some_and(|v| v != view.view_sha256) {
        return Err(failure(
            "comparison_reporting_changed",
            "Saved comparison snapshot/config/view changed",
        ));
    }
    view.validate(result)?;
    Ok(Some(view))
}
impl ComparisonReporting {
    pub(super) fn csv_metadata(&self) -> Result<Value, SavedComparisonFailure> {
        let mut value = serde_json::to_value(self)
            .map_err(|e| failure("comparison_reporting_csv", e.to_string()))?;
        value
            .as_object_mut()
            .ok_or_else(|| failure("comparison_reporting_csv", "Expected metadata object"))?
            .remove("rows");
        Ok(value)
    }
    pub(super) fn validate(
        &self,
        result: &SavedComparisonResult,
    ) -> Result<(), SavedComparisonFailure> {
        let bad = || {
            failure(
                "invalid_comparison_reporting",
                "Reporting index, counts or bound identity differs from the complete raw comparison",
            )
        };
        if self.identity.engine != "open-seo-spider/saved-comparison-reporting-v1" {
            return Err(bad());
        }
        for p in [&self.identity.before_profile, &self.identity.after_profile]
            .into_iter()
            .flatten()
        {
            if !p.active() || p.validate().is_err() {
                return Err(bad());
            }
        }
        if self.before_captured_page_observations
            != result.before_snapshot.coverage.saved_page_count
            || self.after_captured_page_observations
                != result.after_snapshot.coverage.saved_page_count
            || self.before_profile_visible_page_observations
                > self.before_captured_page_observations
            || self.after_profile_visible_page_observations > self.after_captured_page_observations
            || self.rows.len() != result.pages.len()
            || self.raw_compared_rows != self.rows.len()
            || self.raw_counts != result.counts
            || self.raw_counts != counts(result.pages.iter())
            || self.identity_sha256 != digest(&self.identity)?
            || self.view_sha256 != token(result, self)?
        {
            return Err(bad());
        }
        for (i, (r, p)) in self.rows.iter().zip(&result.pages).enumerate() {
            if r.comparison_index != i
                || r.comparison_key != p.comparison_key
                || r.before_visible.is_some() != p.before.is_some()
                || r.after_visible.is_some() != p.after.is_some()
                || r.included
                    != (self.identity.mode == ReportingMode::Raw
                        || r.before_visible == Some(true)
                        || r.after_visible == Some(true))
            {
                return Err(bad());
            }
        }
        if self
            .rows
            .iter()
            .filter(|r| r.before_visible.is_some())
            .count()
            != self.before_captured_page_observations
            || self
                .rows
                .iter()
                .filter(|r| r.after_visible.is_some())
                .count()
                != self.after_captured_page_observations
            || self
                .rows
                .iter()
                .filter(|r| r.before_visible == Some(true))
                .count()
                != self.before_profile_visible_page_observations
            || self
                .rows
                .iter()
                .filter(|r| r.after_visible == Some(true))
                .count()
                != self.after_profile_visible_page_observations
            || self.identity.before_profile.is_none()
                && self.rows.iter().any(|r| r.before_visible == Some(false))
            || self.identity.after_profile.is_none()
                && self.rows.iter().any(|r| r.after_visible == Some(false))
        {
            return Err(bad());
        }
        let included = self.rows.iter().filter(|r| r.included).count();
        if included != self.included_compared_rows
            || self.rows.len() - included != self.excluded_compared_rows
            || self.view_counts
                != counts(
                    self.rows
                        .iter()
                        .filter(|r| r.included)
                        .map(|r| &result.pages[r.comparison_index]),
                )
        {
            return Err(bad());
        }
        Ok(())
    }
}
