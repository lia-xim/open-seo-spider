//! A bounded declaration-derived reporting profile, never network authority.
use crate::{LinkRecord, PageRecord};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use url::Url;

pub const CDN_DIRECTORY_REPORTING_ENGINE: &str = "open-seo-spider/cdn-directory-reporting-v1";
pub const CDN_DIRECTORY_NORMALIZATION: &str = "ascii-dns-literal-directory-cross-http-www-v1";
pub const MAX_CDN_DIRECTORY_RULES: usize = 32;
pub const MAX_CDN_DIRECTORY_RULE_UTF8_BYTES: usize = 2048;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CdnDirectoryReportingConfig {
    pub version: u8,
    pub rules: Vec<String>,
}

impl<'de> Deserialize<'de> for CdnDirectoryReportingConfig {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            version: u8,
            rules: Vec<String>,
        }
        struct Object;
        impl<'de> serde::de::Visitor<'de> for Object {
            type Value = CdnDirectoryReportingConfig;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a versioned CDN directory reporting object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let f = Fields::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
                Ok(CdnDirectoryReportingConfig {
                    version: f.version,
                    rules: f.rules,
                })
            }
        }
        d.deserialize_map(Object)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Rule {
    host: String,
    port: Option<u16>,
    directory: String,
}

fn literal_directory(path: &str) -> bool {
    let bytes = path.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit()
            {
                return false;
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    !path.split('/').any(|segment| {
        let dots = segment.to_ascii_lowercase().replace("%2e", ".");
        dots == "." || dots == ".."
    })
}

fn rule(value: &str) -> Result<Rule> {
    ensure!(
        !value.is_empty()
            && value.len() <= MAX_CDN_DIRECTORY_RULE_UTF8_BYTES
            && value.is_ascii()
            && value.trim() == value
            && !value
                .bytes()
                .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
            && !value.contains(['\\', '?', '#', '@', '*']),
        "Invalid CDN directory address"
    );
    let (scheme, rest) = value
        .split_once("://")
        .context("CDN directory requires HTTP(S)")?;
    ensure!(
        scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"),
        "CDN directory requires HTTP(S)"
    );
    let (authority, directory) = rest
        .find('/')
        .map_or((rest, "/"), |at| (&rest[..at], &rest[at..]));
    let (host, port) = if let Some((host, text)) = authority.split_once(':') {
        ensure!(
            !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()),
            "Invalid CDN directory port"
        );
        let port = text.parse::<u16>().context("Invalid CDN directory port")?;
        ensure!(
            port != 0 && port != 80 && port != 443,
            "Explicit ports80/443 are unsupported by this directory profile"
        );
        (host, Some(port))
    } else {
        (authority, None)
    };
    ensure!(
        !host.is_empty()
            && host.len() <= 253
            && host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && !label.to_ascii_lowercase().starts_with("xn--")
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            }),
        "CDN directories require an ordinary ASCII DNS name"
    );
    ensure!(
        directory.starts_with('/') && directory.ends_with('/') && literal_directory(directory),
        "CDN directory must be root or a literal trailing-slash directory"
    );
    let parsed = Url::parse(value).context("Invalid CDN directory URL")?;
    ensure!(
        matches!(parsed.host(), Some(url::Host::Domain(_)))
            && parsed.path() == directory
            && parsed.port() == port,
        "Unsupported CDN directory normalization"
    );
    let host = host.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_owned();
    let base =
        Url::parse(&format!("http://{host}/")).context("Invalid CDN directory host alias")?;
    ensure!(
        matches!(base.host(), Some(url::Host::Domain(_))) && base.host_str() == Some(host.as_str()),
        "CDN directory aliases require DNS names"
    );
    Ok(Rule {
        host,
        port,
        directory: directory.into(),
    })
}

impl CdnDirectoryReportingConfig {
    pub fn validate(&self) -> Result<()> {
        self.compile().map(drop)
    }
    pub(crate) fn compile(&self) -> Result<CompiledCdnDirectoryReporting> {
        ensure!(
            self.version == 1,
            "Unsupported CDN directory reporting version"
        );
        ensure!(
            !self.rules.is_empty() && self.rules.len() <= MAX_CDN_DIRECTORY_RULES,
            "CDN directory reporting requires1..=32 rules"
        );
        let mut seen = HashSet::new();
        let mut rules = Vec::with_capacity(self.rules.len());
        for value in &self.rules {
            let compiled = rule(value)?;
            ensure!(
                seen.insert(compiled.clone()),
                "Duplicate compiled CDN directory rule"
            );
            rules.push(compiled);
        }
        Ok(CompiledCdnDirectoryReporting {
            rules,
            declarations: self.rules.clone(),
            config_sha256: format!("{:x}", Sha256::digest(serde_json::to_vec(self)?)),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CdnDirectoryRuleMatch {
    pub rule_index: usize,
    pub declared_directory: String,
}

/// Absence is unknown, including old saved reports. Reading never backfills it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CdnDirectoryReportingObservation {
    pub engine: String,
    pub normalization: String,
    pub basis: String,
    pub config_sha256: String,
    pub matching_rules: Vec<CdnDirectoryRuleMatch>,
    pub seed_scope_internal: bool,
    pub grants_network_or_credentials: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CompiledCdnDirectoryReporting {
    rules: Vec<Rule>,
    declarations: Vec<String>,
    config_sha256: String,
}

impl CompiledCdnDirectoryReporting {
    fn observation(
        &self,
        value: &str,
        seed_scope_internal: bool,
    ) -> Option<CdnDirectoryReportingObservation> {
        let url = crate::normalize_url(value, None)?;
        if !matches!(url.host(), Some(url::Host::Domain(_))) {
            return None;
        }
        let host = url.host_str()?;
        let matching_rules: Vec<_> = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                (host == r.host.as_str() || host.strip_prefix("www.") == Some(r.host.as_str()))
                    && url.port() == r.port
                    && url.path().starts_with(&r.directory)
            })
            .map(|(i, _)| CdnDirectoryRuleMatch {
                rule_index: i,
                declared_directory: self.declarations[i].clone(),
            })
            .collect();
        (!matching_rules.is_empty()).then(|| CdnDirectoryReportingObservation {
            engine: CDN_DIRECTORY_REPORTING_ENGINE.into(),
            normalization: CDN_DIRECTORY_NORMALIZATION.into(),
            basis: "declared_config_literal_directory".into(),
            config_sha256: self.config_sha256.clone(),
            matching_rules,
            seed_scope_internal,
            grants_network_or_credentials: false,
        })
    }

    pub(crate) fn apply(&self, page: &mut PageRecord) {
        let original = page
            .cdn_classification
            .as_ref()
            .map_or(page.internal, |o| o.seed_scope_internal);
        page.cdn_directory_reporting = self.observation(&page.url, original);
        if page.cdn_directory_reporting.is_some() {
            page.internal = true;
        }
        for link in &mut page.links {
            self.apply_link(link);
        }
    }
    fn apply_link(&self, link: &mut LinkRecord) {
        let original = link
            .cdn_classification
            .as_ref()
            .map_or(link.internal, |o| o.seed_scope_internal);
        link.cdn_directory_reporting = self.observation(&link.url, original);
        if link.cdn_directory_reporting.is_some() {
            link.internal = true;
        }
    }
}

#[cfg(test)]
#[path = "cdn_directory_reporting_tests.rs"]
mod tests;
