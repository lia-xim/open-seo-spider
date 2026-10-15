//! Explicit source-only work budgets; retained membership limits stay independent.
//!
//! Absence of this profile is interpreted by CrawlConfig, preserving its legacy
//! page-body clamp. Merely raising a budget does not attest bytes, XML validity,
//! complete retained rows, or a durable source commit.
use crate::{SitemapFailure, SitemapParserLimits};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, MapAccess, Visitor},
};

pub const MAX_SITEMAP_SOURCE_POST_HTTP_ENTITY_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_SITEMAP_SOURCE_DECODED_XML_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_SITEMAP_SOURCE_XML_EVENTS: usize = 2_000_000;

/// Opt-in source work limits, independent of a page document's body budget.
/// Defaults preserve 8 MiB entity/XML and 500,000 events. The 64 MiB hard
/// ceilings allow actual 52,428,801-byte XML observations without changing the
/// protocol's 52,428,800-byte threshold or the retained catalog limits.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SitemapSourceProcessingConfig {
    /// Actual bytes supplied after HTTP Content-Decoding, never wire bytes.
    pub max_post_http_entity_bytes: usize,
    /// Actual identity XML or combined file-gzip output work budget.
    pub max_decoded_xml_bytes: usize,
    /// Recognizer events, including its actual EOF event.
    pub max_xml_events: usize,
}

impl Default for SitemapSourceProcessingConfig {
    fn default() -> Self {
        let parser = SitemapParserLimits::default();
        Self {
            max_post_http_entity_bytes: parser.max_entity_bytes,
            max_decoded_xml_bytes: parser.max_xml_bytes,
            max_xml_events: parser.max_xml_events,
        }
    }
}

impl<'de> Deserialize<'de> for SitemapSourceProcessingConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier)]
        enum Field {
            #[serde(rename = "max_post_http_entity_bytes")]
            PostHttpEntityBytes,
            #[serde(rename = "max_decoded_xml_bytes")]
            DecodedXmlBytes,
            #[serde(rename = "max_xml_events")]
            XmlEvents,
        }

        struct ProfileVisitor;
        impl<'input> Visitor<'input> for ProfileVisitor {
            type Value = SitemapSourceProcessingConfig;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an object containing sitemap source processing limits")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'input>,
            {
                let mut profile = SitemapSourceProcessingConfig::default();
                let mut seen = [false; 3];
                while let Some(field) = map.next_key::<Field>()? {
                    match field {
                        Field::PostHttpEntityBytes => {
                            if seen[0] {
                                return Err(de::Error::duplicate_field(
                                    "max_post_http_entity_bytes",
                                ));
                            }
                            seen[0] = true;
                            profile.max_post_http_entity_bytes = map.next_value()?;
                        }
                        Field::DecodedXmlBytes => {
                            if seen[1] {
                                return Err(de::Error::duplicate_field("max_decoded_xml_bytes"));
                            }
                            seen[1] = true;
                            profile.max_decoded_xml_bytes = map.next_value()?;
                        }
                        Field::XmlEvents => {
                            if seen[2] {
                                return Err(de::Error::duplicate_field("max_xml_events"));
                            }
                            seen[2] = true;
                            profile.max_xml_events = map.next_value()?;
                        }
                    }
                }
                Ok(profile)
            }
        }

        // Named-struct derivation also accepts sequences. This opt-in profile
        // requires a map in every Serde format, with identical field defaults.
        deserializer.deserialize_map(ProfileVisitor)
    }
}

impl SitemapSourceProcessingConfig {
    pub fn validate(&self) -> Result<(), SitemapFailure> {
        for (name, value, maximum) in [
            (
                "max_post_http_entity_bytes",
                self.max_post_http_entity_bytes,
                MAX_SITEMAP_SOURCE_POST_HTTP_ENTITY_BYTES,
            ),
            (
                "max_decoded_xml_bytes",
                self.max_decoded_xml_bytes,
                MAX_SITEMAP_SOURCE_DECODED_XML_BYTES,
            ),
            (
                "max_xml_events",
                self.max_xml_events,
                MAX_SITEMAP_SOURCE_XML_EVENTS,
            ),
        ] {
            if value == 0 || value > maximum {
                return Err(SitemapFailure::new(
                    "sitemap_invalid_limits",
                    format!("{name} must be 1..={maximum}"),
                    None,
                ));
            }
        }
        Ok(())
    }

    /// Validate every supplied field before producing the shared decode/XML
    /// limits. Entry, location, text, field, depth and namespace settings retain
    /// their existing defaults; no input value is clamped or repaired.
    pub fn parser_limits(&self) -> Result<SitemapParserLimits, SitemapFailure> {
        self.validate()?;
        let limits = SitemapParserLimits {
            max_entity_bytes: self.max_post_http_entity_bytes,
            max_xml_bytes: self.max_decoded_xml_bytes,
            max_xml_events: self.max_xml_events,
            ..SitemapParserLimits::default()
        };
        limits.validate()?;
        Ok(limits)
    }
}

#[cfg(test)]
#[path = "sitemap_source_processing_tests.rs"]
mod tests;
