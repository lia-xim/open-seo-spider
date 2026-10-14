//! Durable measurement envelopes. Capture retention and SQL commit evidence are separate.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

pub const SITEMAP_SOURCE_STORE_ENGINE: &str = "open-seo-spider/sitemap-source-store-v1";
pub const SITEMAP_SOURCE_STORE_HASH_PROFILE: &str = "sha256-canonical-rust-serde-source-store-v1";
pub const MAX_SOURCE_STORE_BYTES: usize = 33_554_432;
pub const MAX_SOURCE_RECORD_BYTES: usize = 16_384;
pub const SOURCE_COMMIT_RESERVE_BYTES: u64 = 1024;
pub const SOURCE_METADATA_ALLOCATION_BYTES: u64 = 4096;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SitemapSourceStoreLimits {
    pub max_catalog_json_bytes: usize,
    pub max_record_json_bytes: usize,
    pub max_records: u64,
}
impl Default for SitemapSourceStoreLimits {
    fn default() -> Self {
        Self {
            max_catalog_json_bytes: MAX_SOURCE_STORE_BYTES,
            max_record_json_bytes: MAX_SOURCE_RECORD_BYTES,
            max_records: 100_000,
        }
    }
}
impl SitemapSourceStoreLimits {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.max_catalog_json_bytes >= SOURCE_METADATA_ALLOCATION_BYTES as usize
                && self.max_catalog_json_bytes <= MAX_SOURCE_STORE_BYTES,
            "Invalid source measurement catalog byte cap"
        );
        ensure!(
            self.max_record_json_bytes > 0
                && self.max_record_json_bytes <= MAX_SOURCE_RECORD_BYTES
                && self.max_record_json_bytes <= self.max_catalog_json_bytes,
            "Invalid source measurement envelope byte cap"
        );
        ensure!(
            self.max_records > 0 && self.max_records <= 100_000,
            "Invalid source measurement record cap"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceStoredMeasurement {
    pub schema_version: u32,
    pub engine: String,
    pub sequence: u64,
    pub initial_catalog_config_sha256: String,
    /// Sequence-zero document seal, before any admission or catalog budget decision.
    pub staged_document_sha256: String,
    pub measurement: crate::SitemapSourceMeasurement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceCommitAssociation {
    pub schema_version: u32,
    pub engine: String,
    pub measurement_id: String,
    pub measurement_envelope_sha256: String,
    pub document_id: String,
    pub document_sequence: u64,
    /// SHA of exact canonical SitemapLedgerRow::Document bytes in sitemap_documents.data.
    pub document_payload_sha256: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapSourceStoreOmission {
    StorageBudgetExceeded,
    CaptureUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceStoreInheritedBasis {
    pub parent_snapshot_sha256: String,
    pub parent_catalog_sha256: String,
    pub current_discovery_executed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceStoreSummary {
    pub engine: String,
    pub hash_profile: String,
    pub initial_catalog_config_sha256: String,
    pub limits: SitemapSourceStoreLimits,
    pub revision: u64,
    pub last_sequence: u64,
    pub record_count: u64,
    pub commit_count: u64,
    pub record_json_bytes: u64,
    pub commit_json_bytes: u64,
    /// Actual canonical summary JSON length, including this field (fixed point).
    pub metadata_json_bytes: u64,
    pub logical_json_bytes: u64,
    /// Unused fixed metadata allocation plus one commit allocation per unjoined record.
    pub reserved_json_bytes: u64,
    pub charged_json_bytes: u64,
    pub content_sha256: String,
    pub links_sha256: String,
    pub legacy_history_unknown: bool,
    pub omission: Option<SitemapSourceStoreOmission>,
    pub omission_revision: Option<u64>,
    pub inherited: Option<SitemapSourceStoreInheritedBasis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceStoreSnapshot {
    pub schema_version: u32,
    pub summary: SitemapSourceStoreSummary,
    pub records: Vec<SitemapSourceStoredMeasurement>,
    pub commits: Vec<SitemapSourceCommitAssociation>,
}
