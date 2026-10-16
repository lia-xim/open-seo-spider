//! Own durable target admission quotas. No vendor equivalence is asserted.
use anyhow::{Context, Result, ensure};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use url::Url;

pub const CRAWL_QUOTA_ENGINE: &str = "open-seo-spider/crawl-quotas-v1";
pub const MAX_CRAWL_QUOTA: u64 = 1_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrawlUrlPatternQuota {
    pub id: String,
    pub pattern: String,
    pub max_urls: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrawlQuotaConfig {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_urls_per_depth: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_urls_per_subdomain: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub url_pattern_quotas: Vec<CrawlUrlPatternQuota>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrawlQuotaIncreases {
    pub expected_generation: u64,
    pub expected_profile_revision: u64,
    pub expected_profile_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_urls_per_depth: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_urls_per_subdomain: Option<u64>,
    #[serde(default, deserialize_with = "unique_pattern_maxima")]
    pub url_pattern_maxima: BTreeMap<String, u64>,
}

fn unique_pattern_maxima<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<BTreeMap<String, u64>, D::Error> {
    struct Unique;
    impl<'de> serde::de::Visitor<'de> for Unique {
        type Value = BTreeMap<String, u64>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("unique quota rule maxima")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, u64>()? {
                if values.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("Duplicate quota rule maximum"));
                }
            }
            Ok(values)
        }
    }
    d.deserialize_map(Unique)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlQuotaBucketUsage {
    pub dimension: String,
    pub key: String,
    pub used: u64,
    pub limit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlQuotaSummary {
    pub engine: String,
    pub generation: u64,
    pub profile_revision: u64,
    pub profile_sha256: String,
    pub config: CrawlQuotaConfig,
    pub reserved_distinct_targets: u64,
    /// Durable pre-send dispatch intents. Unknown crashed attempts may not have
    /// reached the wire; actual website requests require the outside IO ledger.
    pub actual_started_attempts: u64,
    pub settled_attempts: u64,
    pub interrupted_or_unknown_attempts: u64,
    pub active_attempts: u64,
    pub deferred_targets: u64,
    pub bucket_rows: Vec<CrawlQuotaBucketUsage>,
    pub bucket_rows_omitted: u64,
    pub causes: Vec<String>,
}

pub(crate) struct CompiledCrawlQuotas {
    pub config: CrawlQuotaConfig,
    pub sha256: String,
    patterns: Vec<Regex>,
}

pub(crate) fn quota_hash<T: Serialize>(value: &T) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

impl CrawlQuotaConfig {
    pub fn validate(&self) -> Result<()> {
        self.compile().map(|_| ())
    }
    pub(crate) fn compile(&self) -> Result<CompiledCrawlQuotas> {
        ensure!(self.version == 1, "Unsupported crawl_quotas version");
        ensure!(
            self.max_urls_per_depth.is_some()
                || self.max_urls_per_subdomain.is_some()
                || !self.url_pattern_quotas.is_empty(),
            "At least one crawl quota is required"
        );
        ensure!(
            self.max_urls_per_depth.is_none_or(|n| n <= MAX_CRAWL_QUOTA)
                && self
                    .max_urls_per_subdomain
                    .is_none_or(|n| n <= MAX_CRAWL_QUOTA),
            "Quota exceeds own hard ceiling"
        );
        ensure!(
            self.url_pattern_quotas.len() <= 32,
            "At most32 URL quota rules"
        );
        let mut ids = BTreeSet::new();
        let mut bytes = 0usize;
        let mut patterns = Vec::new();
        for rule in &self.url_pattern_quotas {
            ensure!(
                (1..=64).contains(&rule.id.len())
                    && rule
                        .id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-')),
                "Quota rule ID must be1..64 safe ASCII bytes"
            );
            ensure!(ids.insert(&rule.id), "Duplicate quota rule ID");
            ensure!(
                !rule.pattern.is_empty()
                    && rule.pattern.len() <= 4096
                    && rule.max_urls <= MAX_CRAWL_QUOTA,
                "Invalid URL quota rule bounds"
            );
            bytes = bytes
                .checked_add(rule.pattern.len())
                .context("Quota pattern bytes overflow")?;
            ensure!(bytes <= 32768, "Total quota pattern byte bound");
            patterns.push(
                RegexBuilder::new(&rule.pattern)
                    .size_limit(256 * 1024)
                    .dfa_size_limit(256 * 1024)
                    .build()
                    .context("Invalid or oversized URL quota regex")?,
            );
        }
        Ok(CompiledCrawlQuotas {
            config: self.clone(),
            sha256: quota_hash(self)?,
            patterns,
        })
    }
    pub(crate) fn increased(&self, increase: &CrawlQuotaIncreases) -> Result<Self> {
        let mut new = self.clone();
        for (old, value) in [
            (&mut new.max_urls_per_depth, increase.max_urls_per_depth),
            (
                &mut new.max_urls_per_subdomain,
                increase.max_urls_per_subdomain,
            ),
        ] {
            if let Some(value) = value {
                let prior = (*old).context("Cannot add a quota dimension during resume")?;
                ensure!(
                    value >= prior && value <= MAX_CRAWL_QUOTA,
                    "Resume can only increase existing quota maxima"
                );
                *old = Some(value);
            }
        }
        for (id, max) in &increase.url_pattern_maxima {
            let rule = new
                .url_pattern_quotas
                .iter_mut()
                .find(|r| &r.id == id)
                .context("Resume cannot add a URL quota rule")?;
            ensure!(
                *max >= rule.max_urls && *max <= MAX_CRAWL_QUOTA,
                "Resume cannot lower a URL quota maximum"
            );
            rule.max_urls = *max;
        }
        ensure!(new != *self, "Quota increase must actually raise a maximum");
        new.validate()?;
        Ok(new)
    }
}

impl CompiledCrawlQuotas {
    pub fn memberships(&self, url: &Url, depth: usize) -> Result<Vec<(String, String, u64)>> {
        ensure!(
            url.as_str().len() <= 1024 * 1024,
            "Quota target URL exceeds own1MiB work bound"
        );
        let mut result = Vec::new();
        if let Some(max) = self.config.max_urls_per_depth {
            result.push(("depth".into(), depth.to_string(), max));
        }
        if let Some(max) = self.config.max_urls_per_subdomain {
            result.push((
                "host".into(),
                url.host_str().context("Quota target needs host")?.into(),
                max,
            ));
        }
        for (rule, regex) in self.config.url_pattern_quotas.iter().zip(&self.patterns) {
            if regex.is_match(url.as_str()) {
                result.push(("pattern".into(), rule.id.clone(), rule.max_urls));
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
#[path = "crawl_quotas_tests.rs"]
mod tests;
