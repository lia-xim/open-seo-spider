//! Independent source-specific resource switches. Pure code performs no HTTP.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use url::Url;

pub const RESOURCE_DISCOVERY_ENGINE: &str = "open-seo-spider/resource-discovery-v1";

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
}

impl ResourceDiscoveryConfig {
    pub fn validate(&self) -> Result<()> {
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
        } else if row.nofollow && !self.follow_rel_nofollow {
            "rel_nofollow"
        } else if facts.document_nofollow && !self.follow_document_nofollow {
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
