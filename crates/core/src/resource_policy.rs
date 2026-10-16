//! Independent source-specific resource switches. Pure code performs no HTTP.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use url::Url;

pub const RESOURCE_DISCOVERY_ENGINE: &str = "open-seo-spider/resource-discovery-v1";
pub const RESOURCE_CONTRACT_ENGINE: &str = "open-seo-spider/resource-contract-v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceContractConfig {
    pub engine: String,
    pub follow_internal_qualified_links: bool,
    pub follow_external_qualified_links: bool,
}
impl Default for ResourceContractConfig {
    fn default() -> Self {
        Self {
            engine: RESOURCE_CONTRACT_ENGINE.into(),
            follow_internal_qualified_links: false,
            follow_external_qualified_links: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceLinkScope {
    Internal,
    External,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceQualifierSource {
    HtmlRel,
    HttpLinkRel,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceQualificationDecision {
    pub engine: String,
    pub scope: ResourceLinkScope,
    pub rel_qualifiers: Vec<String>,
    pub source: ResourceQualifierSource,
    pub document_nofollow: bool,
    pub follow_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRedirectDecision {
    pub source_url: String,
    pub raw_location: String,
    pub target_url: Option<String>,
    pub original_reference_kinds: Vec<ResourceReferenceKind>,
    pub authorizing_occurrence_ids: Vec<String>,
    pub scope_allowed: bool,
    pub policy_check: Option<bool>,
    pub policy_traverse: Option<bool>,
    pub robots_allowed: Option<bool>,
    pub allowed: bool,
    pub reason: String,
    /// Admission/request initiation is observed; delivery to the server is not inferred.
    pub request_started: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceSwitch {
    pub store: bool,
    pub fetch: bool,
}
impl Default for ResourceSwitch {
    fn default() -> Self {
        Self {
            store: true,
            fetch: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NavigationSwitch {
    pub store: bool,
    pub fetch: bool,
    pub expand: bool,
}
impl Default for NavigationSwitch {
    fn default() -> Self {
        Self {
            store: true,
            fetch: false,
            expand: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceTypeRules {
    pub css: ResourceSwitch,
    pub javascript: ResourceSwitch,
    pub images: ResourceSwitch,
    pub media: ResourceSwitch,
    pub pdf: ResourceSwitch,
    pub swf: ResourceSwitch,
    pub other: ResourceSwitch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NavigationDiscoveryRules {
    pub internal_anchors: NavigationSwitch,
    pub external_anchors: NavigationSwitch,
    pub iframes: NavigationSwitch,
    pub refresh: NavigationSwitch,
    pub canonical: NavigationSwitch,
    pub hreflang: NavigationSwitch,
    pub pagination: NavigationSwitch,
    pub amp: NavigationSwitch,
    pub mobile_alternate: NavigationSwitch,
    pub pdf_annotations: NavigationSwitch,
}
impl Default for NavigationDiscoveryRules {
    fn default() -> Self {
        let enabled = NavigationSwitch {
            store: true,
            fetch: true,
            expand: true,
        };
        Self {
            internal_anchors: enabled,
            iframes: enabled,
            refresh: enabled,
            external_anchors: NavigationSwitch::default(),
            canonical: NavigationSwitch::default(),
            hreflang: NavigationSwitch::default(),
            pagination: NavigationSwitch::default(),
            amp: NavigationSwitch::default(),
            mobile_alternate: NavigationSwitch::default(),
            pdf_annotations: NavigationSwitch::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CssSourceRules {
    pub stylesheets: bool,
    pub style_elements: bool,
    pub style_attributes: bool,
}
impl Default for CssSourceRules {
    fn default() -> Self {
        Self {
            stylesheets: true,
            style_elements: true,
            style_attributes: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceScopeConfig {
    pub external_origins: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceDiscoveryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract: Option<ResourceContractConfig>,
    pub resources: ResourceTypeRules,
    pub navigation: NavigationDiscoveryRules,
    pub css: CssSourceRules,
    pub resource_scope: ResourceScopeConfig,
    pub follow_rel_nofollow: bool,
    pub follow_document_nofollow: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceDiscoveryLimits {
    pub max_source_candidates: usize,
    pub max_occurrence_utf8_bytes: usize,
    pub max_source_batch_bytes: usize,
    pub max_css_utf8_bytes: usize,
    pub max_css_tokens: usize,
    pub max_css_nesting: usize,
    pub max_diagnostics: usize,
    pub max_ledger_bytes: Option<usize>,
}
impl Default for ResourceDiscoveryLimits {
    fn default() -> Self {
        Self {
            max_source_candidates: 100_000,
            max_occurrence_utf8_bytes: 65_536,
            max_source_batch_bytes: 32 * 1024 * 1024,
            max_css_utf8_bytes: 16 * 1024 * 1024,
            max_css_tokens: 1_000_000,
            max_css_nesting: 64,
            max_diagnostics: 1000,
            max_ledger_bytes: None,
        }
    }
}
impl ResourceDiscoveryLimits {
    pub fn validate(&self) -> Result<()> {
        for (name, value, upper) in [
            (
                "max_source_candidates",
                self.max_source_candidates,
                1_000_000,
            ),
            (
                "max_occurrence_utf8_bytes",
                self.max_occurrence_utf8_bytes,
                65536,
            ),
            (
                "max_source_batch_bytes",
                self.max_source_batch_bytes,
                128 * 1024 * 1024,
            ),
            (
                "max_css_utf8_bytes",
                self.max_css_utf8_bytes,
                64 * 1024 * 1024,
            ),
            ("max_css_tokens", self.max_css_tokens, 8_000_000),
            ("max_css_nesting", self.max_css_nesting, 128),
            ("max_diagnostics", self.max_diagnostics, 10_000),
        ] {
            ensure!(
                value > 0 && value <= upper,
                "Resource {name} must be 1..={upper}"
            );
        }
        ensure!(
            self.max_ledger_bytes != Some(0),
            "Resource ledger budget must be positive or null"
        );
        Ok(())
    }
    pub(crate) fn check_batch(&self, batch: &ResourceSourceBatch) -> Result<()> {
        ensure!(
            batch.occurrences.len() <= self.max_source_candidates,
            "Resource candidate budget exceeded; source remains pending"
        );
        ensure!(
            batch.diagnostics.len() <= self.max_diagnostics,
            "Resource diagnostic budget exceeded; source remains pending"
        );
        for row in &batch.occurrences {
            ensure!(
                serde_json::to_vec(row)?.len() <= self.max_occurrence_utf8_bytes,
                "Resource occurrence byte budget exceeded; source remains pending"
            );
        }
        ensure!(
            serde_json::to_vec(batch)?.len() <= self.max_source_batch_bytes,
            "Resource source byte budget exceeded; source remains pending"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceReferenceKind {
    InitialInput,
    InternalAnchor,
    ExternalAnchor,
    Iframe,
    Refresh,
    Canonical,
    Hreflang,
    Pagination,
    Amp,
    MobileAlternate,
    PdfAnnotation,
    Css,
    JavaScript,
    Image,
    Media,
    Pdf,
    Swf,
    Other,
}
impl ResourceReferenceKind {
    pub fn is_navigation(self) -> bool {
        matches!(
            self,
            Self::InternalAnchor
                | Self::ExternalAnchor
                | Self::Iframe
                | Self::Refresh
                | Self::Canonical
                | Self::Hreflang
                | Self::Pagination
                | Self::Amp
                | Self::MobileAlternate
                | Self::PdfAnnotation
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceSourceKind {
    InitialInput,
    NativeHtml,
    NativeCss,
    PdfAnnotation,
    HttpLinkHeader,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceOccurrence {
    pub source_complete: bool,
    pub source_unit_revision: String,
    pub occurrence_id: String,
    pub source_revision: String,
    pub source_ordinal: usize,
    pub requested_source_url: String,
    pub source_document_url: String,
    pub raw_value: String,
    pub decoded_value: Option<String>,
    pub resolved_url: Option<String>,
    pub reference_kind: ResourceReferenceKind,
    pub classification_basis: String,
    pub declared_mime: Option<String>,
    pub dom_location: Option<String>,
    pub byte_start: Option<usize>,
    pub byte_end: Option<usize>,
    pub attributes: BTreeMap<String, String>,
    pub nofollow: bool,
    pub observation_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceSourceBatch {
    pub engine: String,
    pub source_revision: String,
    pub requested_url: String,
    pub document_url: String,
    pub source_kind: ResourceSourceKind,
    pub complete: bool,
    pub diagnostics: Vec<String>,
    pub occurrences: Vec<ResourceOccurrence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceGrant {
    pub engine: String,
    pub config_sha256: String,
    pub occurrence_id: String,
    pub fetch_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_kind: Option<ResourceReferenceKind>,
    pub store: bool,
    pub can_expand_navigation: bool,
    pub can_expand_resources: bool,
    pub can_expand_css: bool,
    pub can_expand_pdf: bool,
    pub policy_sha256: Option<String>,
    pub policy_occurrence_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDecision {
    pub reason: String,
    pub grant: Option<ResourceGrant>,
    pub coalesced_into: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualification: Option<ResourceQualificationDecision>,
}

/// Caller supplies observed source and actual native scope/policy facts. This
/// is a deterministic permission decision, never an authentication attestation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResourcePermission {
    pub source_complete: bool,
    pub source_can_expand: bool,
    pub selected: bool,
    pub navigation_enabled: bool,
    pub depth_allowed: bool,
    pub scope_allowed: bool,
    pub filters_allowed: bool,
    pub policy_check: bool,
    pub policy_traverse: bool,
    pub document_nofollow: bool,
    pub target_internal: Option<bool>,
}

impl ResourceDiscoveryConfig {
    pub fn validate(&self) -> Result<()> {
        if let Some(contract) = &self.contract {
            ensure!(
                contract.engine == RESOURCE_CONTRACT_ENGINE,
                "Unknown resource contract engine"
            );
            ensure!(
                !self.follow_rel_nofollow && !self.follow_document_nofollow,
                "Resource contract v2 conflicts with legacy follow_rel_nofollow/follow_document_nofollow; use scoped qualified-link switches"
            );
        }
        ensure!(
            self.resource_scope.external_origins.len() <= 32,
            "At most 32 external resource origins"
        );
        let mut seen = HashSet::new();
        for value in &self.resource_scope.external_origins {
            let parsed = Url::parse(value)?;
            ensure!(
                matches!(parsed.scheme(), "http" | "https")
                    && parsed.host_str().is_some()
                    && parsed
                        .host_str()
                        .is_some_and(|host| !host.contains('*')
                            && !host.to_ascii_lowercase().contains("%2a"))
                    && parsed.username().is_empty()
                    && parsed.password().is_none()
                    && parsed.path() == "/"
                    && parsed.query().is_none()
                    && parsed.fragment().is_none(),
                "Resource external origins must be HTTP(S) origins without path, query or credentials"
            );
            ensure!(
                seen.insert(crate::urls::origin(&parsed)),
                "Duplicate external resource origin"
            );
        }
        for rule in [
            self.navigation.internal_anchors,
            self.navigation.external_anchors,
            self.navigation.iframes,
            self.navigation.refresh,
            self.navigation.canonical,
            self.navigation.hreflang,
            self.navigation.pagination,
            self.navigation.amp,
            self.navigation.mobile_alternate,
            self.navigation.pdf_annotations,
        ] {
            ensure!(
                !rule.expand || rule.fetch,
                "Navigation expand requires fetch"
            );
        }
        Ok(())
    }
    pub fn sha256(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(serde_json::to_vec(self)?))
    }
    pub(crate) fn switches(&self, kind: ResourceReferenceKind) -> (ResourceSwitch, bool) {
        use ResourceReferenceKind as K;
        let nav = match kind {
            K::InternalAnchor => Some(self.navigation.internal_anchors),
            K::ExternalAnchor => Some(self.navigation.external_anchors),
            K::Iframe => Some(self.navigation.iframes),
            K::Refresh => Some(self.navigation.refresh),
            K::Canonical => Some(self.navigation.canonical),
            K::Hreflang => Some(self.navigation.hreflang),
            K::Pagination => Some(self.navigation.pagination),
            K::Amp => Some(self.navigation.amp),
            K::MobileAlternate => Some(self.navigation.mobile_alternate),
            K::PdfAnnotation => Some(self.navigation.pdf_annotations),
            _ => None,
        };
        if let Some(rule) = nav {
            return (
                ResourceSwitch {
                    store: rule.store,
                    fetch: rule.fetch,
                },
                rule.expand,
            );
        }
        let rule = match kind {
            K::InitialInput => ResourceSwitch::default(),
            K::Css => self.resources.css,
            K::JavaScript => self.resources.javascript,
            K::Image => self.resources.images,
            K::Media => self.resources.media,
            K::Pdf => self.resources.pdf,
            K::Swf => self.resources.swf,
            _ => self.resources.other,
        };
        (rule, false)
    }
    pub(crate) fn decide(
        &self,
        row: &ResourceOccurrence,
        facts: ResourcePermission,
    ) -> Result<ResourceDecision> {
        let (switch, expand) = self.switches(row.reference_kind);
        let qualification = self.contract.as_ref().map(|contract| {
            let tokens = row.attributes.get("rel").map(String::as_str).unwrap_or("");
            let mut rel_qualifiers = ["nofollow", "ugc", "sponsored"]
                .into_iter()
                .filter(|qualifier| {
                    tokens
                        .split_ascii_whitespace()
                        .any(|token| token.eq_ignore_ascii_case(qualifier))
                })
                .map(String::from)
                .collect::<Vec<_>>();
            if row.nofollow && !rel_qualifiers.iter().any(|token| token == "nofollow") {
                rel_qualifiers.insert(0, "nofollow".into());
            }
            let scope = match facts.target_internal {
                Some(true) => ResourceLinkScope::Internal,
                Some(false) => ResourceLinkScope::External,
                None => ResourceLinkScope::Unknown,
            };
            let enabled = match scope {
                ResourceLinkScope::Internal => contract.follow_internal_qualified_links,
                ResourceLinkScope::External => contract.follow_external_qualified_links,
                ResourceLinkScope::Unknown => false,
            };
            ResourceQualificationDecision {
                engine: contract.engine.clone(),
                scope,
                follow_allowed: (rel_qualifiers.is_empty() && !facts.document_nofollow) || enabled,
                rel_qualifiers,
                source: if row.classification_basis == "resolved_http_link_header_target" {
                    ResourceQualifierSource::HttpLinkRel
                } else if matches!(
                    row.classification_basis.as_str(),
                    "html_attribute_context" | "anchor_context" | "html_link_rel"
                ) {
                    ResourceQualifierSource::HtmlRel
                } else {
                    ResourceQualifierSource::NotApplicable
                },
                document_nofollow: facts.document_nofollow,
            }
        });
        let reason = if facts.selected {
            "selected_no_expansion"
        } else if row.observation_error.is_some() || row.resolved_url.is_none() {
            "invalid_or_non_http_reference"
        } else if !facts.source_complete {
            "source_incomplete"
        } else if !facts.source_can_expand {
            "source_check_only"
        } else if !facts.depth_allowed {
            "depth_limit"
        } else if row.reference_kind.is_navigation() && !facts.navigation_enabled {
            "navigation_disabled"
        } else if qualification
            .as_ref()
            .is_some_and(|fact| !fact.follow_allowed)
        {
            match qualification
                .as_ref()
                .and_then(|fact| fact.rel_qualifiers.first())
                .map(String::as_str)
            {
                Some("nofollow") => "rel_nofollow",
                Some("ugc") => "rel_ugc",
                Some("sponsored") => "rel_sponsored",
                _ => "document_nofollow",
            }
        } else if self.contract.is_none() && row.nofollow && !self.follow_rel_nofollow {
            "rel_nofollow"
        } else if self.contract.is_none()
            && facts.document_nofollow
            && !self.follow_document_nofollow
        {
            "document_nofollow"
        } else if !switch.fetch {
            "fetch_disabled"
        } else if !facts.scope_allowed {
            "scope_denied"
        } else if !facts.filters_allowed {
            "filter_denied"
        } else if !facts.policy_check {
            "policy_denied"
        } else {
            "allowed"
        };
        let grant = if reason == "allowed" {
            let initial = row.reference_kind == ResourceReferenceKind::InitialInput;
            let nav = initial || (row.reference_kind.is_navigation() && expand);
            Some(ResourceGrant {
                engine: RESOURCE_DISCOVERY_ENGINE.into(),
                config_sha256: self.sha256()?,
                occurrence_id: row.occurrence_id.clone(),
                fetch_url: row.resolved_url.clone().expect("checked URL"),
                reference_kind: self.contract.as_ref().map(|_| row.reference_kind),
                store: switch.store,
                can_expand_navigation: nav && facts.navigation_enabled && facts.policy_traverse,
                can_expand_resources: nav && facts.policy_traverse,
                can_expand_css: (initial || row.reference_kind == ResourceReferenceKind::Css)
                    && self.css.stylesheets
                    && facts.policy_traverse,
                can_expand_pdf: (initial
                    || nav
                    || row.reference_kind == ResourceReferenceKind::Pdf)
                    && self.navigation.pdf_annotations.fetch
                    && facts.policy_traverse,
                policy_sha256: None,
                policy_occurrence_id: None,
            })
        } else {
            None
        };
        Ok(ResourceDecision {
            reason: reason.into(),
            grant,
            coalesced_into: None,
            qualification,
        })
    }
    pub(crate) fn permits_resource_origin(
        &self,
        url: &Url,
        seeds: &[Url],
        subdomains: bool,
    ) -> bool {
        crate::urls::same_scope(url, seeds, subdomains)
            || self
                .resource_scope
                .external_origins
                .iter()
                .filter_map(|value| Url::parse(value).ok())
                .any(|allowed| crate::urls::origin(&allowed) == crate::urls::origin(url))
    }

    #[allow(clippy::too_many_arguments)] // Explicit original-role, origin, and policy boundaries.
    pub(crate) fn redirect_decision(
        &self,
        permission: &crate::ResourceTaskPermission,
        source: &Url,
        raw: &str,
        target: Option<&Url>,
        seeds: &[Url],
        subdomains: bool,
        check_external: bool,
        policy: Option<(bool, bool)>,
        prior: &[ResourceRedirectDecision],
    ) -> Result<ResourceRedirectDecision> {
        self.validate()?;
        ensure!(self.contract.is_some(), "Strict redirect contract missing");
        permission.validate()?;
        let mut kinds = Vec::new();
        let mut authorizing = Vec::new();
        for grant in &permission.authorizing_grants {
            let kind = grant.reference_kind.ok_or_else(|| {
                anyhow::anyhow!("Strict resource grant has no immutable original role")
            })?;
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
            let allowed = prior.iter().all(|hop| {
                hop.authorizing_occurrence_ids
                    .contains(&grant.occurrence_id)
            }) && target.is_some_and(|target| {
                if kind == ResourceReferenceKind::InitialInput || kind.is_navigation() {
                    crate::urls::same_scope(target, seeds, subdomains)
                        || check_external
                        || policy.is_some_and(|(check, _)| check)
                } else {
                    self.permits_resource_origin(target, seeds, subdomains)
                }
            });
            if allowed {
                authorizing.push(grant.occurrence_id.clone());
            }
        }
        let scope_allowed = !authorizing.is_empty();
        let allowed = target.is_some() && scope_allowed && policy.is_none_or(|(check, _)| check);
        let reason = if target.is_none() {
            "redirect_invalid"
        } else if policy.is_some_and(|(check, _)| !check) {
            "redirect_policy_denied"
        } else if !scope_allowed {
            "redirect_role_scope_denied"
        } else {
            "redirect_allowed"
        };
        Ok(ResourceRedirectDecision {
            source_url: source.to_string(),
            raw_location: raw.into(),
            target_url: target.map(ToString::to_string),
            original_reference_kinds: kinds,
            authorizing_occurrence_ids: authorizing,
            scope_allowed,
            policy_check: policy.map(|(check, _)| check),
            policy_traverse: policy.map(|(_, traverse)| traverse),
            robots_allowed: None,
            allowed,
            reason: reason.into(),
            request_started: false,
        })
    }

    pub(crate) fn eligible_authority(
        &self,
        page: &crate::PageRecord,
        permission: &crate::ResourceTaskPermission,
        seeds: &[Url],
        subdomains: bool,
        check_external: bool,
        captured_responses_only: bool,
    ) -> Result<HashSet<String>> {
        if self.contract.is_none() {
            return Ok(permission
                .authorizing_grants
                .iter()
                .map(|grant| grant.occurrence_id.clone())
                .collect());
        }
        let facts = page
            .resource_response
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Strict response facts missing"))?;
        let mut evaluated = Vec::new();
        // Synthetic HSTS gates participate in immutable grant intersection, but
        // are not added to the actual network redirect response facts.
        let mut ordered = Vec::new();
        for index in 0..=facts.redirect_decisions.len() {
            if let Some(tls) = &page.tls {
                for hop in tls
                    .hsts_upgrades
                    .iter()
                    .filter(|hop| hop.network_redirect_index == index)
                {
                    if let Some(gate) = &hop.resource_gate {
                        ordered.push(gate);
                    }
                }
            }
            if let Some(gate) = facts.redirect_decisions.get(index) {
                ordered.push(gate);
            }
        }
        for observed in ordered {
            if captured_responses_only
                && !observed.target_url.as_deref().is_some_and(|target| {
                    page.redirect_chain.iter().any(|hop| hop.url == target)
                        || (page.final_status.is_some() && page.final_url == target)
                })
            {
                break;
            }
            let source = crate::normalize_url(&observed.source_url, None)
                .ok_or_else(|| anyhow::anyhow!("Invalid observed redirect source"))?;
            let target = observed
                .target_url
                .as_deref()
                .and_then(|url| crate::normalize_url(url, None));
            let policy = observed
                .policy_check
                .map(|check| (check, observed.policy_traverse.unwrap_or(false)));
            evaluated.push(self.redirect_decision(
                permission,
                &source,
                &observed.raw_location,
                target.as_ref(),
                seeds,
                subdomains,
                check_external,
                policy,
                &evaluated,
            )?);
        }
        Ok(permission
            .authorizing_grants
            .iter()
            .filter(|grant| {
                evaluated.iter().all(|hop| {
                    hop.allowed
                        && hop
                            .authorizing_occurrence_ids
                            .contains(&grant.occurrence_id)
                })
            })
            .map(|grant| grant.occurrence_id.clone())
            .collect())
    }
}

pub(crate) fn digest(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
pub(crate) fn occurrence_id(revision: &str, ordinal: usize) -> String {
    digest(format!("resource:{revision}:{ordinal}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(kind: ResourceReferenceKind) -> ResourceOccurrence {
        ResourceOccurrence {
            source_complete: true,
            source_unit_revision: "unit".into(),
            occurrence_id: "own".into(),
            source_revision: "source".into(),
            source_ordinal: 0,
            requested_source_url: "https://example.test/".into(),
            source_document_url: "https://example.test/".into(),
            raw_value: "/asset".into(),
            decoded_value: Some("/asset".into()),
            resolved_url: Some("https://example.test/asset".into()),
            reference_kind: kind,
            classification_basis: "own_fixture".into(),
            declared_mime: None,
            dom_location: None,
            byte_start: None,
            byte_end: None,
            attributes: BTreeMap::new(),
            nofollow: false,
            observation_error: None,
        }
    }
    fn facts() -> ResourcePermission {
        ResourcePermission {
            source_complete: true,
            source_can_expand: true,
            selected: false,
            navigation_enabled: true,
            depth_allowed: true,
            scope_allowed: true,
            filters_allowed: true,
            policy_check: true,
            policy_traverse: true,
            document_nofollow: false,
            target_internal: Some(true),
        }
    }
    #[test]
    fn independent_store_fetch_product_preserves_observation() {
        for store in [false, true] {
            for fetch in [false, true] {
                let mut cfg = ResourceDiscoveryConfig::default();
                cfg.resources.images = ResourceSwitch { store, fetch };
                let original = row(ResourceReferenceKind::Image);
                let decision = cfg.decide(&original, facts()).unwrap();
                assert_eq!(decision.grant.is_some(), fetch);
                assert_eq!(
                    decision.grant.map(|grant| grant.store),
                    fetch.then_some(store)
                );
                assert_eq!(original.raw_value, "/asset");
            }
        }
    }
    #[test]
    fn disabled_image_never_blocks_independent_anchor() {
        let mut cfg = ResourceDiscoveryConfig::default();
        cfg.resources.images.fetch = false;
        assert!(
            cfg.decide(&row(ResourceReferenceKind::Image), facts())
                .unwrap()
                .grant
                .is_none()
        );
        assert!(
            cfg.decide(&row(ResourceReferenceKind::InternalAnchor), facts())
                .unwrap()
                .grant
                .unwrap()
                .can_expand_navigation
        );
    }
    #[test]
    fn list_resources_are_independent_selected_never_expands() {
        let cfg = ResourceDiscoveryConfig::default();
        let mut f = facts();
        f.navigation_enabled = false;
        assert!(
            cfg.decide(&row(ResourceReferenceKind::Css), f)
                .unwrap()
                .grant
                .unwrap()
                .can_expand_css
        );
        assert_eq!(
            cfg.decide(&row(ResourceReferenceKind::InternalAnchor), f)
                .unwrap()
                .reason,
            "navigation_disabled"
        );
        f.selected = true;
        assert!(
            cfg.decide(&row(ResourceReferenceKind::Css), f)
                .unwrap()
                .grant
                .is_none()
        );
    }
    #[test]
    fn check_only_is_immutable_and_resource_html_gets_no_navigation_grant() {
        let cfg = ResourceDiscoveryConfig::default();
        let mut f = facts();
        f.policy_traverse = false;
        let original = cfg
            .decide(&row(ResourceReferenceKind::Css), f)
            .unwrap()
            .grant
            .unwrap();
        assert!(
            !original.can_expand_css
                && !original.can_expand_resources
                && !original.can_expand_navigation
        );
        let anchor = cfg
            .decide(&row(ResourceReferenceKind::InternalAnchor), facts())
            .unwrap()
            .grant
            .unwrap();
        assert!(anchor.can_expand_navigation);
        assert!(!original.can_expand_navigation);
        let image = cfg
            .decide(&row(ResourceReferenceKind::Image), facts())
            .unwrap()
            .grant
            .unwrap();
        assert!(!image.can_expand_navigation && !image.can_expand_resources);
    }
    #[test]
    fn exact_external_origins_and_invalid_settings() {
        let mut cfg = ResourceDiscoveryConfig::default();
        cfg.resource_scope
            .external_origins
            .push("https://cdn.test:8443".into());
        cfg.validate().unwrap();
        assert!(cfg.permits_resource_origin(
            &Url::parse("https://cdn.test:8443/img").unwrap(),
            &[],
            false
        ));
        assert!(!cfg.permits_resource_origin(
            &Url::parse("https://cdn.test/img").unwrap(),
            &[],
            false
        ));
        for value in [
            "https://u:p@cdn.test",
            "https://cdn.test/path",
            "https://cdn.test/?q",
            "data:text/plain,x",
        ] {
            cfg.resource_scope.external_origins = vec![value.into()];
            assert!(cfg.validate().is_err());
        }
        cfg.resource_scope.external_origins.clear();
        cfg.navigation.canonical.expand = true;
        assert!(cfg.validate().is_err());
        assert!(
            ResourceDiscoveryLimits {
                max_css_nesting: 0,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn nofollow_only_changes_admission_and_separate_filters_fail_closed() {
        let cfg = ResourceDiscoveryConfig::default();
        let mut original = row(ResourceReferenceKind::InternalAnchor);
        original.nofollow = true;
        assert_eq!(
            cfg.decide(&original, facts()).unwrap().reason,
            "rel_nofollow"
        );
        assert!(
            ResourceDiscoveryConfig {
                follow_rel_nofollow: true,
                ..cfg.clone()
            }
            .decide(&original, facts())
            .unwrap()
            .grant
            .is_some()
        );
        original.nofollow = false;
        let mut f = facts();
        f.filters_allowed = false;
        assert_eq!(cfg.decide(&original, f).unwrap().reason, "filter_denied");
        f.filters_allowed = true;
        f.policy_check = false;
        assert_eq!(cfg.decide(&original, f).unwrap().reason, "policy_denied");
    }
    #[test]
    fn config_hash_has_no_operational_limit_or_ambiguous_unknown_field() {
        let cfg = ResourceDiscoveryConfig::default();
        let hash = cfg.sha256().unwrap();
        assert_eq!(
            hash,
            serde_json::from_slice::<ResourceDiscoveryConfig>(&serde_json::to_vec(&cfg).unwrap())
                .unwrap()
                .sha256()
                .unwrap()
        );
        assert!(
            serde_json::from_str::<ResourceDiscoveryConfig>("{\"future_scope\":true}").is_err()
        );
        assert_eq!(ResourceDiscoveryLimits::default().max_ledger_bytes, None);
    }
}
