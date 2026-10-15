//! Proposed opt-in body archive foundation. Public transport/UI are separate gates.
use anyhow::{Result, ensure};
use serde::{Deserialize, Deserializer, Serialize, de};

pub const SITEMAP_SOURCE_ARCHIVE_ENGINE: &str = "open-seo-spider/sitemap-source-archive-v1";
pub const MAX_SITEMAP_ARCHIVE_BODY_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_ARCHIVE_METADATA_BYTES: u64 = 32 * 1024 * 1024;
pub(crate) const MAX_ARCHIVE_CAPTURE_BYTES: usize = 8192;
pub(crate) const ARCHIVE_STATE_RESERVE: u64 = 8192;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SitemapSourceArchiveConfig {
    pub retain_post_http_entity: bool,
    pub retain_decoded_xml: bool,
    pub max_bytes_per_body: u64,
    pub max_total_body_bytes: u64,
    pub max_capture_records: u64,
}
impl Default for SitemapSourceArchiveConfig {
    fn default() -> Self {
        Self {
            retain_post_http_entity: true,
            retain_decoded_xml: true,
            max_bytes_per_body: MAX_SITEMAP_ARCHIVE_BODY_BYTES,
            max_total_body_bytes: 512 * 1024 * 1024,
            max_capture_records: 4096,
        }
    }
}
impl<'de> Deserialize<'de> for SitemapSourceArchiveConfig {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(default, deny_unknown_fields)]
        struct Fields {
            retain_post_http_entity: bool,
            retain_decoded_xml: bool,
            max_bytes_per_body: u64,
            max_total_body_bytes: u64,
            max_capture_records: u64,
        }
        impl Default for Fields {
            fn default() -> Self {
                let c = SitemapSourceArchiveConfig::default();
                Self {
                    retain_post_http_entity: c.retain_post_http_entity,
                    retain_decoded_xml: c.retain_decoded_xml,
                    max_bytes_per_body: c.max_bytes_per_body,
                    max_total_body_bytes: c.max_total_body_bytes,
                    max_capture_records: c.max_capture_records,
                }
            }
        }
        struct Object;
        impl<'de> de::Visitor<'de> for Object {
            type Value = SitemapSourceArchiveConfig;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a sitemap source archive object")
            }
            fn visit_map<A: de::MapAccess<'de>>(
                self,
                map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let f = Fields::deserialize(de::value::MapAccessDeserializer::new(map))?;
                let c = SitemapSourceArchiveConfig {
                    retain_post_http_entity: f.retain_post_http_entity,
                    retain_decoded_xml: f.retain_decoded_xml,
                    max_bytes_per_body: f.max_bytes_per_body,
                    max_total_body_bytes: f.max_total_body_bytes,
                    max_capture_records: f.max_capture_records,
                };
                c.validate().map_err(de::Error::custom)?;
                Ok(c)
            }
        }
        d.deserialize_map(Object)
    }
}
impl SitemapSourceArchiveConfig {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.retain_post_http_entity || self.retain_decoded_xml,
            "Choose at least one sitemap archive representation"
        );
        ensure!(
            (1..=MAX_SITEMAP_ARCHIVE_BODY_BYTES).contains(&self.max_bytes_per_body),
            "Archive body cap must be 1..=64MiB"
        );
        ensure!(
            (1..=8 * 1024 * 1024 * 1024u64).contains(&self.max_total_body_bytes),
            "Archive total cap must be 1..=8GiB"
        );
        ensure!(
            (1..=100_000).contains(&self.max_capture_records),
            "Archive capture count must be 1..=100000"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapBodyRepresentation {
    PostHttpEntity,
    DecodedXml,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapBodyExtent {
    Complete,
    ObservedPrefix,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapArchivedBody {
    pub body_id: String,
    pub representation: SitemapBodyRepresentation,
    pub extent: SitemapBodyExtent,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapArchiveOmission {
    BodyByteBudget,
    TotalByteBudget,
    CaptureCountBudget,
    MetadataByteBudget,
    BytesUnobserved,
    MeasurementUnavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapArchivedCapture {
    pub schema_version: u32,
    pub sequence: u64,
    pub measurement_id: String,
    pub measurement_envelope_sha256: String,
    pub staged_document_sha256: String,
    pub capture_config_sha256: String,
    pub capture_generation: u64,
    pub bodies: Vec<SitemapArchivedBody>,
    pub omission: Option<SitemapArchiveOmission>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceArchiveSummary {
    pub engine: String,
    pub config: SitemapSourceArchiveConfig,
    pub initial_config_sha256: String,
    pub captures: u64,
    pub body_bytes: u64,
    pub capture_json_bytes: u64,
    pub content_sha256: String,
    pub first_omission: Option<SitemapArchiveOmission>,
    pub legacy_history_unknown: bool,
    pub directory_entry_crash_durability_proven: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SitemapSourceArchiveSnapshot {
    pub summary: SitemapSourceArchiveSummary,
    pub captures: Vec<SitemapArchivedCapture>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SitemapSourceArchiveInspection {
    NotRecordedOrLegacyUnknown,
    /// No child-owned files/copy proof in this foundation. Never follow parent paths.
    InheritedArchiveNotCopied,
    VerifiedLocal(SitemapSourceArchiveSnapshot),
}

/// Own actual observed Vecs. Intentionally NOT Clone/Serialize; Debug only lengths.
pub(crate) struct SourceBodyCandidate {
    pub(crate) post_http_entity: Option<Vec<u8>>,
    pub(crate) decoded_xml: Option<Vec<u8>>,
}
impl std::fmt::Debug for SourceBodyCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceBodyCandidate")
            .field(
                "post_http_bytes",
                &self.post_http_entity.as_ref().map(Vec::len),
            )
            .field(
                "decoded_xml_bytes",
                &self.decoded_xml.as_ref().map(Vec::len),
            )
            .finish()
    }
}
