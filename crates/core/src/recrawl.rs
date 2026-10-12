//! Independent, immutable selected-recrawl plans. This module performs no HTTP.
use crate::{CrawlConfig, CrawlReport, Crawler, normalize_url};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

pub const SELECTED_RECRAWL_ENGINE: &str = "open-seo-spider/selected-recrawl-v1";
pub const SELECTED_RECRAWL_COVERAGE: &str = "mixed_age_selected_recrawl";
const MAX_SNAPSHOT_BYTES: usize = 256 * 1024 * 1024;
const MAX_SELECTION_ROWS: usize = 50_000;
const MAX_SELECTION_BYTES: usize = 8 * 1024 * 1024;
const MAX_URL_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecrawlSourceKind {
    CapturedCheckpoint,
    ImportedSnapshot,
    CurrentProjection,
    StoredJson,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectedRecrawlParent {
    pub job_id: String,
    pub checkpoint_revision: Option<u64>,
    pub source_kind: RecrawlSourceKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecrawlParentReference {
    pub job_id: String,
    pub checkpoint_revision: Option<u64>,
    pub source_kind: RecrawlSourceKind,
    /// Hash of the exact parent JSON bytes, including whitespace/unknown fields.
    pub snapshot_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecrawlSelectionOccurrence {
    /// One-based occurrence, preserving caller input order and repetitions.
    pub occurrence: usize,
    pub raw_value: String,
    pub requested_url: String,
    /// One-based first occurrence when this repeated selection was coalesced.
    pub coalesced_into: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecrawlTarget {
    pub requested_url: String,
    /// Zero-based position in the immutable parent's page array.
    pub parent_ordinal: usize,
    pub parent_page_sha256: String,
    pub depth: usize,
    pub discovered_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_admission: Option<crate::PolicyAdmission>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_permission: Option<crate::ResourceTaskPermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectedRecrawlPlan {
    pub engine: String,
    pub parent: RecrawlParentReference,
    pub inherited_config_sha256: String,
    pub plan_sha256: String,
    pub selection: Vec<RecrawlSelectionOccurrence>,
    pub targets: Vec<RecrawlTarget>,
    /// Default admission budget for EACH invocation; None is unlimited.
    pub default_max_attempts: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
// This immutable plan is constructed once per invocation, never per page or
// event. Keep the public plan value directly accessible without a heap wrapper.
#[allow(clippy::large_enum_variant)]
#[serde(
    tag = "kind",
    content = "plan",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RunPlan {
    #[default]
    FullCrawl,
    SelectedRecrawl(SelectedRecrawlPlan),
}

/// The app may inspect this preparation before initializing a new child store.
/// It is not a status response: immutable source bytes can be large.
#[derive(Debug, Clone)]
pub struct PreparedSelectedRecrawl {
    pub config: CrawlConfig,
    pub plan: SelectedRecrawlPlan,
    pub base_snapshot_bytes: Vec<u8>,
    pub base_report: CrawlReport,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct RecrawlResumeOverride {
    /// None retains the plan default; Some(0) explicitly removes the budget.
    /// A positive value limits NEW admissions in this invocation only.
    pub max_attempts: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecrawlTargetState {
    Pending,
    Committed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecrawlTargetObservation {
    pub requested_url: String,
    pub parent_ordinal: usize,
    pub parent_page_sha256: String,
    pub state: RecrawlTargetState,
    /// Durable admissions, not a claim that every request reached its server.
    pub admitted_attempts: usize,
    pub committed_generation: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectedRecrawlOperation {
    pub engine: String,
    pub coverage: String,
    pub parent: RecrawlParentReference,
    pub plan_sha256: String,
    pub inherited_config_sha256: String,
    pub selection: Vec<RecrawlSelectionOccurrence>,
    pub retained_count: usize,
    pub refreshed_count: usize,
    pub pending_count: usize,
    pub max_attempts: Option<usize>,
    pub admitted_this_run: usize,
    pub committed_this_run: usize,
    pub targets: Vec<RecrawlTargetObservation>,
}

pub fn prepare_selected_recrawl(
    parent: SelectedRecrawlParent,
    snapshot_bytes: &[u8],
    urls: &[String],
    max_attempts: Option<usize>,
) -> Result<PreparedSelectedRecrawl> {
    ensure!(
        !snapshot_bytes.is_empty(),
        "Parent snapshot must not be empty"
    );
    ensure!(
        snapshot_bytes.len() <= MAX_SNAPSHOT_BYTES,
        "Parent snapshot exceeds 256 MiB"
    );
    ensure!(
        !parent.job_id.trim().is_empty()
            && parent.job_id.len() <= 128
            && !parent.job_id.chars().any(char::is_control),
        "Parent job ID must have 1..=128 bytes without controls"
    );
    ensure!(
        parent.checkpoint_revision != Some(0),
        "Checkpoint revision must be positive when present"
    );
    ensure!(
        !urls.is_empty() && urls.len() <= MAX_SELECTION_ROWS,
        "Select 1..=50000 requested URL occurrences"
    );
    let base_report: CrawlReport =
        serde_json::from_slice(snapshot_bytes).context("Invalid parent crawl snapshot")?;
    let crawler = Crawler::new(base_report.config.clone())?;
    let config = crawler.effective_config().clone();
    let mut pages = HashMap::new();
    for (ordinal, page) in base_report.pages.iter().enumerate() {
        crate::content_binding::validate_native_content_captures(page, &config, None, false)?;
        ensure!(
            !pages.contains_key(page.url.as_str()),
            "Parent has ambiguous duplicate requested URL: {}",
            page.url
        );
        pages.insert(page.url.as_str(), (ordinal, page));
    }
    let mut selection = Vec::with_capacity(urls.len());
    let mut targets = Vec::new();
    let mut first_occurrence = HashMap::new();
    let mut selection_bytes = 0usize;
    for (index, raw) in urls.iter().enumerate() {
        selection_bytes = selection_bytes
            .checked_add(raw.len())
            .context("Selection byte count overflow")?;
        ensure!(
            selection_bytes <= MAX_SELECTION_BYTES,
            "Selection exceeds 8 MiB"
        );
        ensure!(
            raw.len() <= MAX_URL_BYTES && !raw.chars().any(char::is_control),
            "Selected URL exceeds 16 KiB or contains controls"
        );
        let (ordinal, page) = pages
            .get(raw.as_str())
            .context("Select an exact recorded requested URL, not a final/rendered alias")?;
        let normalized = normalize_url(raw, None).context("Invalid selected HTTP(S) URL")?;
        ensure!(
            normalized.as_str() == raw
                && normalized.username().is_empty()
                && normalized.password().is_none(),
            "Recorded requested URL uses an unsupported/noncanonical identity"
        );
        ensure!(
            config.crawl_policy.is_some() || crawler.selected_target_allowed(&normalized),
            "Selected external URL is not eligible under inherited scope/external-check settings"
        );
        let coalesced_into = first_occurrence.get(raw).copied();
        if coalesced_into.is_none() {
            let policy_admission = match &config.crawl_policy {
                Some(policy) => {
                    let compiled = crate::CompiledCrawlPolicy::new(policy)?;
                    let mut saved = page
                        .policy
                        .as_ref()
                        .context("Policy parent page lacks saved admission")?
                        .admission
                        .clone();
                    saved.validate(
                        compiled.policy_sha256(),
                        raw,
                        crate::PolicyTaskRole::PageDocument,
                    )?;
                    saved.inherited = true;
                    Some(saved)
                }
                None => None,
            };
            first_occurrence.insert(raw.clone(), index + 1);
            targets.push(RecrawlTarget {
                requested_url: raw.clone(),
                parent_ordinal: *ordinal,
                parent_page_sha256: payload_sha256(page)?,
                depth: page.depth,
                discovered_from: page.discovered_from.clone(),
                policy_admission,
                resource_permission: match &base_report.config.resource_discovery {
                    Some(config) => {
                        let permission = page
                            .resource_permission
                            .clone()
                            .context("Resource parent page lacks durable task permission")?;
                        permission.validate()?;
                        ensure!(
                            permission.fetch_url == *raw
                                && permission.config_sha256 == config.sha256()?,
                            "Resource parent permission differs from config/target"
                        );
                        Some(permission)
                    }
                    None => None,
                },
            });
        }
        selection.push(RecrawlSelectionOccurrence {
            occurrence: index + 1,
            raw_value: raw.clone(),
            requested_url: raw.clone(),
            coalesced_into,
        });
    }
    // Stable page order is inherited from the parent, independent of user click order.
    targets.sort_by_key(|target| target.parent_ordinal);
    let mut plan = SelectedRecrawlPlan {
        engine: SELECTED_RECRAWL_ENGINE.into(),
        parent: RecrawlParentReference {
            job_id: parent.job_id,
            checkpoint_revision: parent.checkpoint_revision,
            source_kind: parent.source_kind,
            snapshot_sha256: bytes_sha256(snapshot_bytes),
        },
        inherited_config_sha256: payload_sha256(&config)?,
        plan_sha256: String::new(),
        selection,
        targets,
        default_max_attempts: max_attempts.filter(|value| *value > 0),
    };
    plan.plan_sha256 = payload_sha256(&plan)?;
    Ok(PreparedSelectedRecrawl {
        config,
        plan,
        base_snapshot_bytes: snapshot_bytes.to_vec(),
        base_report,
    })
}

impl PreparedSelectedRecrawl {
    /// Revalidate public preparation fields so forged/tampered values cannot
    /// turn a selected operation into a broader crawl.
    pub fn validate(&self) -> Result<()> {
        let validated = validate_saved_plan(&self.plan, &self.base_snapshot_bytes, &self.config)?;
        ensure!(
            serde_json::to_value(&validated.base_report)?
                == serde_json::to_value(&self.base_report)?,
            "Prepared parent report does not match exact snapshot bytes"
        );
        Ok(())
    }
}

pub(crate) fn validate_saved_plan(
    plan: &SelectedRecrawlPlan,
    bytes: &[u8],
    config: &CrawlConfig,
) -> Result<PreparedSelectedRecrawl> {
    ensure!(
        plan.engine == SELECTED_RECRAWL_ENGINE,
        "Unsupported selected-recrawl engine"
    );
    let prepared = prepare_selected_recrawl(
        SelectedRecrawlParent {
            job_id: plan.parent.job_id.clone(),
            checkpoint_revision: plan.parent.checkpoint_revision,
            source_kind: plan.parent.source_kind,
        },
        bytes,
        &plan
            .selection
            .iter()
            .map(|row| row.raw_value.clone())
            .collect::<Vec<_>>(),
        plan.default_max_attempts,
    )?;
    ensure!(
        prepared.plan == *plan,
        "Saved selected-recrawl plan or immutable parent hash is inconsistent"
    );
    // The same four operational knobs as ordinary resume may change. Scope,
    // headers, renderer, extraction and other semantics must stay inherited.
    let mut allowed = prepared.config.clone();
    allowed.max_pages = config.max_pages;
    allowed.concurrency = config.concurrency;
    allowed.delay_ms = config.delay_ms;
    allowed.request_timeout_secs = config.request_timeout_secs;
    ensure!(
        serde_json::to_value(allowed)? == serde_json::to_value(config)?,
        "Selected recrawl cannot change inherited crawl semantics"
    );
    Ok(prepared)
}

impl SelectedRecrawlPlan {
    pub(crate) fn targets_set(&self) -> HashSet<&str> {
        self.targets
            .iter()
            .map(|target| target.requested_url.as_str())
            .collect()
    }
    pub(crate) fn operation(
        &self,
        retained_count: usize,
        targets: Vec<RecrawlTargetObservation>,
        max_attempts: Option<usize>,
        admitted: usize,
        committed: usize,
    ) -> SelectedRecrawlOperation {
        let refreshed_count = targets
            .iter()
            .filter(|target| target.state == RecrawlTargetState::Committed)
            .count();
        SelectedRecrawlOperation {
            engine: self.engine.clone(),
            coverage: SELECTED_RECRAWL_COVERAGE.into(),
            parent: self.parent.clone(),
            plan_sha256: self.plan_sha256.clone(),
            inherited_config_sha256: self.inherited_config_sha256.clone(),
            selection: self.selection.clone(),
            retained_count,
            refreshed_count,
            pending_count: targets.len() - refreshed_count,
            max_attempts,
            admitted_this_run: admitted,
            committed_this_run: committed,
            targets,
        }
    }
}

pub(crate) fn bytes_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn payload_sha256(value: &impl Serialize) -> Result<String> {
    Ok(bytes_sha256(&serde_json::to_vec(value)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CrawlSummary, PageRecord};
    fn snapshot() -> Vec<u8> {
        serde_json::to_vec_pretty(&CrawlReport {
            config: CrawlConfig {
                seed_urls: vec!["https://example.org/".into()],
                ..Default::default()
            },
            pages: vec![
                PageRecord {
                    url: "https://example.org/a".into(),
                    depth: 3,
                    ..Default::default()
                },
                PageRecord {
                    url: "https://example.org/b".into(),
                    ..Default::default()
                },
            ],
            issues: Vec::new(),
            summary: CrawlSummary::default(),
            warnings: Vec::new(),
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
        })
        .unwrap()
    }
    fn parent() -> SelectedRecrawlParent {
        SelectedRecrawlParent {
            job_id: "fixture-parent".into(),
            checkpoint_revision: Some(1),
            source_kind: RecrawlSourceKind::CapturedCheckpoint,
        }
    }
    #[test]
    fn preparation_hashes_exact_source_and_preserves_ordered_selection_occurrences() {
        let bytes = snapshot();
        let prepared = prepare_selected_recrawl(
            parent(),
            &bytes,
            &[
                "https://example.org/b".into(),
                "https://example.org/a".into(),
                "https://example.org/b".into(),
            ],
            Some(0),
        )
        .unwrap();
        assert_eq!(prepared.base_snapshot_bytes, bytes);
        assert_eq!(prepared.plan.parent.snapshot_sha256, bytes_sha256(&bytes));
        assert_eq!(prepared.plan.selection[2].coalesced_into, Some(1));
        assert_eq!(
            prepared.plan.targets[0].requested_url,
            "https://example.org/a"
        );
        assert_eq!(prepared.plan.targets[0].depth, 3);
        assert_eq!(prepared.plan.default_max_attempts, None);
        prepared.validate().unwrap();
    }
    #[test]
    fn malformed_absent_alias_ambiguous_and_tampered_inputs_fail_without_requests() {
        let bytes = snapshot();
        for urls in [
            Vec::new(),
            vec!["https://example.org/missing".into()],
            vec!["https://example.org/a#fragment".into()],
        ] {
            assert!(prepare_selected_recrawl(parent(), &bytes, &urls, None).is_err());
        }
        assert!(
            prepare_selected_recrawl(
                parent(),
                b"not JSON",
                &["https://example.org/a".into()],
                None
            )
            .is_err()
        );
        let mut report: CrawlReport = serde_json::from_slice(&bytes).unwrap();
        report.pages.push(report.pages[0].clone());
        assert!(
            prepare_selected_recrawl(
                parent(),
                &serde_json::to_vec(&report).unwrap(),
                &["https://example.org/a".into()],
                None
            )
            .is_err()
        );
        let mut prepared =
            prepare_selected_recrawl(parent(), &bytes, &["https://example.org/a".into()], None)
                .unwrap();
        prepared.plan.targets[0].depth = 0;
        assert!(prepared.validate().is_err());
        let mut prepared =
            prepare_selected_recrawl(parent(), &bytes, &["https://example.org/a".into()], None)
                .unwrap();
        prepared.base_report.pages[0].titles.push("forged".into());
        assert!(prepared.validate().is_err());
    }
    #[test]
    fn operational_resume_changes_are_allowed_but_credentials_and_expansion_are_immutable() {
        let prepared = prepare_selected_recrawl(
            parent(),
            &snapshot(),
            &["https://example.org/a".into()],
            Some(1),
        )
        .unwrap();
        let mut config = prepared.config.clone();
        config.max_pages = Some(1);
        config.concurrency = 1;
        validate_saved_plan(&prepared.plan, &prepared.base_snapshot_bytes, &config).unwrap();
        config
            .custom_headers
            .insert("Authorization".into(), "fixture".into());
        assert!(
            validate_saved_plan(&prepared.plan, &prepared.base_snapshot_bytes, &config).is_err()
        );
    }
}
