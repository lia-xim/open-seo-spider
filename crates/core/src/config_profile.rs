//! Portable declarations. Import/preview never constructs clients or opens files.
pub(crate) mod prepare;
mod strict;
use crate::CrawlConfig;
use anyhow::{Context, Result, ensure};
use reqwest::header::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
};
pub use strict::parse_strict_json;

pub const CONFIG_PROFILE_ENGINE: &str = "open-seo-spider/crawl-config-profile-v1";
pub const CONFIG_PROFILE_BYTES: usize = 64 * 1024 * 1024;
pub const CONFIG_PROFILE_MAX_DEPTH: usize = 64;
pub const CONFIG_PROFILE_MAX_NODES: usize = 1_000_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrawlConfigProfile {
    pub version: u32,
    pub config: Value,
    #[serde(default)]
    pub required_headers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConfigProfileChange {
    /// Top-level typed field; values stay in the two lossless JSON documents.
    pub field: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConfigProfilePreview {
    pub profile: CrawlConfigProfile,
    pub effective_config: CrawlConfig,
    pub normalization: Vec<ConfigProfileChange>,
    pub missing_headers: Vec<String>,
    pub requires_targets: bool,
}

pub fn parse_config_profile(bytes: &[u8]) -> Result<CrawlConfigProfile> {
    let profile: CrawlConfigProfile = parse_strict_json(bytes, CONFIG_PROFILE_BYTES)?;
    prepare::prepare(decode(&profile)?, false, false)?;
    Ok(profile)
}

/// Accept native config JSON text before a browser can round uint64 values.
/// Every header is an explicit unresolved name, never a silently dropped value.
pub fn config_profile_from_json(bytes: &[u8]) -> Result<CrawlConfigProfile> {
    let config = strict::parse_value(bytes, CONFIG_PROFILE_BYTES)?;
    from_value(config)
}

pub fn config_profile_from_config(config: &CrawlConfig) -> Result<CrawlConfigProfile> {
    config_profile_from_json(&bounded_json(config)?)
}

fn from_value(mut config: Value) -> Result<CrawlConfigProfile> {
    let object = config
        .as_object_mut()
        .context("Crawl configuration must be a JSON object")?;
    let mut required_headers = Vec::new();
    if let Some(headers) = object.get_mut("custom_headers") {
        let headers = headers
            .as_object_mut()
            .context("custom_headers must be an object")?;
        for (name, value) in headers.iter() {
            HeaderName::from_bytes(name.as_bytes()).context("Invalid configuration header name")?;
            let value = value
                .as_str()
                .context("Configuration header values must be strings")?;
            HeaderValue::from_str(value).context("Invalid configuration header value")?;
            required_headers.push(name.clone());
        }
        headers.clear();
    }
    let profile = CrawlConfigProfile {
        version: 1,
        config,
        required_headers,
    };
    prepare::prepare(decode(&profile)?, false, false)?;
    Ok(profile)
}

pub fn preview_config_profile(profile: &CrawlConfigProfile) -> Result<ConfigProfilePreview> {
    let declared = decode(profile)?;
    let before = serde_json::to_value(&declared)?;
    let prepared = prepare::prepare(declared, false, false)?;
    prepared.config.validate_input_provenance_budget()?;
    let effective = serde_json::to_value(&prepared.config)?;
    let normalization = before
        .as_object()
        .unwrap()
        .keys()
        .chain(effective.as_object().unwrap().keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|field| before.get(*field) != effective.get(*field))
        .map(|field| ConfigProfileChange {
            field: field.clone(),
        })
        .collect();
    Ok(ConfigProfilePreview {
        profile: profile.clone(),
        requires_targets: prepared.config.seed_urls.is_empty()
            && prepared.config.sitemap_urls.is_empty(),
        effective_config: prepared.config,
        normalization,
        missing_headers: profile.required_headers.clone(),
    })
}

pub fn export_config_profile(profile: &CrawlConfigProfile) -> Result<Vec<u8>> {
    prepare::prepare(decode(profile)?, false, false)?;
    bounded_json(profile)
}

/// Only the explicit caller-supplied values enter the resulting runtime config.
/// Callers must not serialize this secret-bearing result as a portable profile.
pub fn resolve_config_profile(
    profile: &CrawlConfigProfile,
    values: &BTreeMap<String, String>,
) -> Result<CrawlConfig> {
    let mut config = decode(profile)?;
    let required: BTreeSet<_> = profile.required_headers.iter().collect();
    ensure!(
        values.keys().all(|name| required.contains(name)),
        "Unexpected configuration header binding"
    );
    ensure!(
        profile
            .required_headers
            .iter()
            .all(|name| values.contains_key(name)),
        "Required configuration header bindings are missing"
    );
    for (name, value) in values {
        HeaderValue::from_str(value).context("Invalid configuration header binding value")?;
        config.custom_headers.insert(name.clone(), value.clone());
    }
    // Saved-mode preparation retains absent original sitemap declarations. The
    // normal fresh crawl boundary still captures them at its established point.
    let prepared = prepare::prepare(config.clone(), false, true)?;
    prepared.config.validate_input_provenance_budget()?;
    Ok(config)
}

fn decode(profile: &CrawlConfigProfile) -> Result<CrawlConfig> {
    ensure!(
        profile.version == 1,
        "Unsupported configuration profile version"
    );
    ensure!(
        profile.config.is_object(),
        "Profile config must be a JSON object"
    );
    check_constructed_value(profile)?;
    // Public structs can be constructed by Rust callers; enforce the same limits
    // as incoming JSON before accepting them through preview/export/resolution.
    let bytes = bounded_json(profile)?;
    strict::parse_value(&bytes, CONFIG_PROFILE_BYTES)?;
    if let Some(headers) = profile.config.get("custom_headers") {
        ensure!(
            headers
                .as_object()
                .is_some_and(|headers| headers.is_empty()),
            "Portable configuration headers must be empty; use required_headers and explicit runtime bindings"
        );
    }
    let mut names = BTreeSet::new();
    for name in &profile.required_headers {
        ensure!(
            names.insert(name),
            "Duplicate required configuration header name"
        );
        HeaderName::from_bytes(name.as_bytes())
            .context("Invalid required configuration header name")?;
    }
    reject_credentials(&profile.config)?;
    strict::validate_crawl_tagged_units(&profile.config)?;
    let config: CrawlConfig =
        crate::heap_deserialize::value(strict::StrictValue(profile.config.clone()))?;
    Ok(config)
}

fn check_constructed_value(profile: &CrawlConfigProfile) -> Result<()> {
    // A Rust caller can construct Value directly without the incoming parser.
    // Check depth iteratively before any recursive serialization or clone.
    let mut remaining = CONFIG_PROFILE_MAX_NODES
        .checked_sub(6usize.saturating_add(profile.required_headers.len()))
        .context("Configuration JSON exceeds node limit")?;
    let mut pending = vec![(&profile.config, 1usize)];
    while let Some((value, depth)) = pending.pop() {
        ensure!(
            depth <= CONFIG_PROFILE_MAX_DEPTH,
            "Configuration JSON exceeds depth64"
        );
        remaining = remaining
            .checked_sub(1)
            .context("Configuration JSON exceeds node limit")?;
        match value {
            Value::Array(values) => {
                ensure!(
                    values.len() <= remaining.saturating_sub(pending.len()),
                    "Configuration JSON exceeds node limit"
                );
                pending.extend(values.iter().map(|value| (value, depth + 1)));
            }
            Value::Object(values) => {
                remaining = remaining
                    .checked_sub(values.len())
                    .context("Configuration JSON exceeds node limit")?;
                ensure!(
                    values.len() <= remaining.saturating_sub(pending.len()),
                    "Configuration JSON exceeds node limit"
                );
                pending.extend(values.values().map(|value| (value, depth + 1)));
            }
            _ => (),
        }
    }
    Ok(())
}

fn reject_credentials(value: &Value) -> Result<()> {
    match value {
        Value::String(text) => {
            ensure!(
                !text.contains("-----BEGIN PRIVATE KEY-----")
                    && !text.contains("-----BEGIN RSA PRIVATE KEY-----")
                    && !text.contains("-----BEGIN EC PRIVATE KEY-----")
                    && !text.contains("-----BEGIN ENCRYPTED PRIVATE KEY-----"),
                "Private key material is not portable configuration"
            );
            if let Ok(url) = url::Url::parse(text) {
                ensure!(
                    !matches!(url.scheme(), "http" | "https")
                        || (url.username().is_empty() && url.password().is_none()),
                    "Credential-bearing URLs are not portable configuration"
                );
            }
        }
        Value::Array(values) => {
            for value in values {
                reject_credentials(value)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                reject_credentials(value)?;
            }
        }
        _ => (),
    }
    Ok(())
}

struct Budget {
    bytes: Vec<u8>,
}
impl Write for Budget {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let length = self
            .bytes
            .len()
            .checked_add(data.len())
            .filter(|length| *length <= CONFIG_PROFILE_BYTES)
            .ok_or_else(|| {
                std::io::Error::other("Configuration profile exceeds64MiB; no partial output")
            })?;
        if length > self.bytes.capacity() {
            let capacity = length
                .max(self.bytes.capacity().saturating_mul(2))
                .min(CONFIG_PROFILE_BYTES);
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(std::io::Error::other)?;
        }
        self.bytes.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn bounded_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut sink = Budget { bytes: Vec::new() };
    serde_json::to_writer(&mut sink, value)?;
    Ok(sink.bytes)
}

#[cfg(test)]
mod tests;
