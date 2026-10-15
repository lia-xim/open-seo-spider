//! Opt-in static PageDocument dispatch checks. No quotas or URL rewriting.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

pub const CRAWL_ADMISSION_LIMITS_VERSION: u32 = 1;
pub const MAX_CRAWL_FOLDER_DEPTH: usize = 65_535;
pub const MAX_CRAWL_QUERY_TOKENS: usize = 65_535;
pub const MAX_CRAWL_FETCH_URL_BYTES: usize = 1024 * 1024;

/// Version is mandatory. Each absent/null inner bound is disabled; at least one
/// bound must be supplied. Outer None is the unchanged legacy behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CrawlAdmissionLimits {
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_folder_depth: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_query_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fetch_url_utf8_bytes: Option<usize>,
}

impl Default for CrawlAdmissionLimits {
    fn default() -> Self {
        Self {
            version: CRAWL_ADMISSION_LIMITS_VERSION,
            max_folder_depth: None,
            max_query_tokens: None,
            max_fetch_url_utf8_bytes: None,
        }
    }
}

impl<'de> Deserialize<'de> for CrawlAdmissionLimits {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            version: u32,
            #[serde(default)]
            max_folder_depth: Option<usize>,
            #[serde(default)]
            max_query_tokens: Option<usize>,
            #[serde(default)]
            max_fetch_url_utf8_bytes: Option<usize>,
        }
        struct Object;
        impl<'de> serde::de::Visitor<'de> for Object {
            type Value = CrawlAdmissionLimits;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a versioned crawl admission limits object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                map: A,
            ) -> Result<Self::Value, A::Error> {
                let raw = Fields::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
                let value = CrawlAdmissionLimits {
                    version: raw.version,
                    max_folder_depth: raw.max_folder_depth,
                    max_query_tokens: raw.max_query_tokens,
                    max_fetch_url_utf8_bytes: raw.max_fetch_url_utf8_bytes,
                };
                value.validate().map_err(serde::de::Error::custom)?;
                Ok(value)
            }
        }
        d.deserialize_map(Object)
    }
}

impl CrawlAdmissionLimits {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.version == CRAWL_ADMISSION_LIMITS_VERSION,
            "Unsupported crawl_admission_limits version"
        );
        anyhow::ensure!(
            self.max_folder_depth.is_some()
                || self.max_query_tokens.is_some()
                || self.max_fetch_url_utf8_bytes.is_some(),
            "At least one static crawl admission limit is required"
        );
        anyhow::ensure!(
            self.max_folder_depth
                .is_none_or(|n| n <= MAX_CRAWL_FOLDER_DEPTH),
            "max_folder_depth must be between 0 and 65535"
        );
        anyhow::ensure!(
            self.max_query_tokens
                .is_none_or(|n| n <= MAX_CRAWL_QUERY_TOKENS),
            "max_query_tokens must be between 0 and 65535"
        );
        anyhow::ensure!(
            self.max_fetch_url_utf8_bytes
                .is_none_or(|n| (1..=MAX_CRAWL_FETCH_URL_BYTES).contains(&n)),
            "max_fetch_url_utf8_bytes must be between 1 and 1048576"
        );
        Ok(())
    }

    /// Receives the *effective normalized fetch URL* from the shared native
    /// transport, after any policy alias. Fragment removal/dot processing/IDNA
    /// are normalize_url's existing semantics, not new admission rewriting.
    pub fn assess(&self, url: &Url, phase: CrawlAdmissionPhase) -> CrawlAdmissionAssessment {
        // Count every completed literal slash after the leading root slash.
        // /a has0, /a/ has1, /a/b has1, /a//b has2. %2F is not a slash.
        let folder_depth = url
            .path()
            .bytes()
            .filter(|b| *b == b'/')
            .count()
            .saturating_sub(1);
        // A present empty query is one empty token. Repeated keys and empty
        // tokens count; percent-encoded ampersands/semicolons do not split.
        let query_tokens = url
            .query()
            .map_or(0, |q| q.bytes().filter(|b| *b == b'&').count() + 1);
        let fetch_url_utf8_bytes = url.as_str().len();
        let denied = if self.max_folder_depth.is_some_and(|n| folder_depth > n) {
            Some(CrawlAdmissionDenial::FolderDepth)
        } else if self.max_query_tokens.is_some_and(|n| query_tokens > n) {
            Some(CrawlAdmissionDenial::QueryTokens)
        } else if self
            .max_fetch_url_utf8_bytes
            .is_some_and(|n| fetch_url_utf8_bytes > n)
        {
            Some(CrawlAdmissionDenial::FetchUrlUtf8Bytes)
        } else {
            None
        };
        CrawlAdmissionAssessment {
            phase,
            fetch_url_sha256: format!("{:x}", Sha256::digest(url.as_str().as_bytes())),
            folder_depth,
            query_tokens,
            fetch_url_utf8_bytes,
            denied,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlAdmissionPhase {
    Initial,
    Redirect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlAdmissionDenial {
    FolderDepth,
    QueryTokens,
    FetchUrlUtf8Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrawlAdmissionAssessment {
    pub phase: CrawlAdmissionPhase,
    /// The corresponding full task/redirect identity stays in PageRecord and
    /// its original policy/resource observations. No second unbounded URL copy.
    pub fetch_url_sha256: String,
    pub folder_depth: usize,
    pub query_tokens: usize,
    pub fetch_url_utf8_bytes: usize,
    pub denied: Option<CrawlAdmissionDenial>,
}

/// Fixed-size per-page evidence: initial and last checked target, not a growing
/// duplicate redirect ledger. A denial is separate from every existing grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrawlAdmissionObservation {
    pub version: u32,
    pub checked_targets: usize,
    pub initial: CrawlAdmissionAssessment,
    pub last: CrawlAdmissionAssessment,
}

impl CrawlAdmissionObservation {
    pub(crate) fn record(target: &mut Option<Self>, assessment: CrawlAdmissionAssessment) {
        match target {
            Some(observation) => {
                observation.checked_targets += 1;
                observation.last = assessment;
            }
            None => {
                *target = Some(Self {
                    version: CRAWL_ADMISSION_LIMITS_VERSION,
                    checked_targets: 1,
                    initial: assessment.clone(),
                    last: assessment,
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "crawl_admission_tests.rs"]
mod tests;
