//! Declared internal reporting classification; never a fetch/expansion/header grant.
use crate::{LinkRecord, PageRecord};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use url::Url;

pub const CDN_CLASSIFICATION_ENGINE: &str = "open-seo-spider/declared-cdn-classification-v1";
pub const MAX_CDN_ORIGINS: usize = 32;
pub const MAX_CDN_ORIGIN_UTF8_BYTES: usize = 2048;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CdnClassificationConfig {
    pub version: u8,
    pub origins: Vec<String>,
}

// A versioned object is required; a positional JSON array is not a declaration.
impl<'de> Deserialize<'de> for CdnClassificationConfig {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            version: u8,
            origins: Vec<String>,
        }
        struct Object;
        impl<'de> serde::de::Visitor<'de> for Object {
            type Value = CdnClassificationConfig;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a versioned CDN classification object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let f = Fields::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
                Ok(CdnClassificationConfig {
                    version: f.version,
                    origins: f.origins,
                })
            }
        }
        d.deserialize_map(Object)
    }
}

impl CdnClassificationConfig {
    pub fn validate(&self) -> Result<()> {
        self.compile().map(drop)
    }

    pub(crate) fn compile(&self) -> Result<CompiledCdnClassification> {
        ensure!(self.version == 1, "Unsupported CDN classification version");
        ensure!(
            !self.origins.is_empty() && self.origins.len() <= MAX_CDN_ORIGINS,
            "CDN classification requires1..=32 exact HTTP(S) origins"
        );
        let mut seen = HashSet::new();
        let mut origins = Vec::with_capacity(self.origins.len());
        for value in &self.origins {
            ensure!(
                value.len() <= MAX_CDN_ORIGIN_UTF8_BYTES
                    && value.trim() == value
                    && !value.chars().any(char::is_control)
                    && !value.contains(['\\', '?', '#', '@', '*']),
                "Invalid CDN origin declaration"
            );
            let (_, authority) = value
                .split_once("://")
                .context("CDN origin requires an explicit HTTP(S) scheme")?;
            let suffix = authority.find('/').map_or("", |i| &authority[i..]);
            ensure!(
                suffix.is_empty() || suffix == "/",
                "CDN origins cannot contain a path"
            );
            let url = Url::parse(value).context("Invalid CDN origin URL")?;
            ensure!(
                matches!(url.scheme(), "http" | "https")
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.path() == "/"
                    && url.query().is_none()
                    && url.fragment().is_none(),
                "CDN origins must be exact HTTP(S) origins"
            );
            let origin = url.origin().ascii_serialization();
            ensure!(
                seen.insert(origin.clone()),
                "Duplicate canonical CDN origin"
            );
            origins.push(origin);
        }
        Ok(CompiledCdnClassification {
            config_sha256: format!("{:x}", Sha256::digest(serde_json::to_vec(self)?)),
            origins,
        })
    }
}

/// A declaration-derived reporting fact, not an observed CDN provider or permission.
/// The enclosing PageRecord/LinkRecord holds the original requested/target URL.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CdnClassificationObservation {
    pub engine: String,
    pub basis: String,
    pub config_sha256: String,
    pub rule_index: usize,
    pub configured_origin: String,
    pub seed_scope_internal: bool,
    pub grants_network_or_credentials: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CompiledCdnClassification {
    config_sha256: String,
    origins: Vec<String>,
}

impl CompiledCdnClassification {
    fn observation(
        &self,
        url: &str,
        seed_scope_internal: bool,
    ) -> Option<CdnClassificationObservation> {
        let url = crate::normalize_url(url, None)?;
        let origin = url.origin().ascii_serialization();
        let rule_index = self.origins.iter().position(|allowed| allowed == &origin)?;
        Some(CdnClassificationObservation {
            engine: CDN_CLASSIFICATION_ENGINE.into(),
            basis: "declared_config_exact_origin".into(),
            config_sha256: self.config_sha256.clone(),
            rule_index,
            configured_origin: origin,
            seed_scope_internal,
            grants_network_or_credentials: false,
        })
    }

    pub(crate) fn apply(&self, page: &mut PageRecord) {
        page.cdn_classification = self.observation(&page.url, page.internal);
        if page.cdn_classification.is_some() {
            page.internal = true;
        }
        for link in &mut page.links {
            link.cdn_classification = self.observation(&link.url, link.internal);
            if link.cdn_classification.is_some() {
                link.internal = true;
            }
        }
    }
}

// Existing discovery used the report's internal bool. Once the optional report
// classification is active, recompute the original seed scope for decisions.
// No persisted declaration-derived bool can become expansion authority.
pub(crate) fn navigation_source_internal(
    page: &PageRecord,
    seeds: &[Url],
    subdomains: bool,
    configured: bool,
) -> bool {
    if !configured {
        return page.internal;
    }
    [&page.url, &page.final_url].iter().all(|value| {
        crate::normalize_url(value, None)
            .is_some_and(|url| crate::urls::same_scope(&url, seeds, subdomains))
    })
}

pub(crate) fn navigation_target_internal(
    link: &LinkRecord,
    seeds: &[Url],
    subdomains: bool,
    configured: bool,
) -> bool {
    if !configured {
        return link.internal;
    }
    crate::normalize_url(&link.url, None)
        .is_some_and(|url| crate::urls::same_scope(&url, seeds, subdomains))
}

#[cfg(test)]
#[path = "cdn_classification_tests.rs"]
mod tests;
