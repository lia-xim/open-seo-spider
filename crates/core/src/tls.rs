use anyhow::{Context, Result, bail};
use reqwest::{Certificate, ClientBuilder, header::HeaderMap};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use url::{Host, Url};

pub const CRAWL_TLS_ENGINE: &str = "crawl-tls-hsts-v1";
pub const MAX_TRUST_FILES: usize = 8;
pub const MAX_TRUST_FILE_BYTES: usize = 65_536;
pub const MAX_TRUST_TOTAL_BYTES: usize = 262_144;
pub const MAX_HSTS_HOSTS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TlsConfig {
    pub version: u32,
    #[serde(default)]
    pub respect_hsts: bool,
    #[serde(default)]
    pub trusted_certificates: Vec<TrustedCertificate>,
}
impl<'de> Deserialize<'de> for TlsConfig {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            version: u32,
            #[serde(default)]
            respect_hsts: bool,
            #[serde(default)]
            trusted_certificates: Vec<TrustedCertificate>,
        }
        struct Object;
        impl<'de> serde::de::Visitor<'de> for Object {
            type Value = TlsConfig;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a version1 TLS configuration object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let raw = Fields::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
                let value = TlsConfig {
                    version: raw.version,
                    respect_hsts: raw.respect_hsts,
                    trusted_certificates: raw.trusted_certificates,
                };
                value.validate().map_err(serde::de::Error::custom)?;
                Ok(value)
            }
        }
        deserializer.deserialize_map(Object)
    }
}
impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            version: 1,
            respect_hsts: false,
            trusted_certificates: Vec::new(),
        }
    }
}
impl TlsConfig {
    pub fn is_inactive(&self) -> bool {
        self.version == 1 && !self.respect_hsts && self.trusted_certificates.is_empty()
    }
    pub fn inactive_option(value: &Option<Self>) -> bool {
        value.as_ref().is_none_or(Self::is_inactive)
    }
    pub fn validate(&self) -> Result<()> {
        self.certificates().map(|_| ())
    }
    fn certificates(&self) -> Result<Vec<Certificate>> {
        if self.version != 1 {
            bail!("tls.version must be 1");
        }
        if self.trusted_certificates.len() > MAX_TRUST_FILES {
            bail!("At most 8 trusted certificate files are allowed");
        }
        let mut total = 0usize;
        let mut result = Vec::new();
        let mut names = HashSet::new();
        for file in &self.trusted_certificates {
            let name = file.name();
            if name.is_empty()
                || name.len() > 256
                || name
                    .chars()
                    .any(|c| c.is_control() || c == '/' || c == '\\')
                || !names.insert(name)
            {
                bail!("Trusted certificate names must be unique, bounded file names");
            }
            let bytes = file.decoded()?;
            total = total
                .checked_add(bytes.len())
                .context("Certificate size overflow")?;
            if bytes.is_empty()
                || bytes.len() > MAX_TRUST_FILE_BYTES
                || total > MAX_TRUST_TOTAL_BYTES
            {
                bail!("Trusted certificates exceed their per-file or total byte limit");
            }
            match file {
                TrustedCertificate::Pem { .. } => {
                    // Accept certificate blocks only, never private keys or silently ignored PEM objects.
                    let text = std::str::from_utf8(&bytes)?;
                    let mut rest = text.trim();
                    let mut count = 0usize;
                    while !rest.is_empty() {
                        rest = rest
                            .strip_prefix("-----BEGIN CERTIFICATE-----")
                            .context("Only PEM CERTIFICATE blocks are allowed")?;
                        let end = rest
                            .find("-----END CERTIFICATE-----")
                            .context("Unterminated PEM certificate")?;
                        let body = &rest[..end];
                        if body.is_empty()
                            || !body.bytes().all(|c| {
                                c.is_ascii_alphanumeric()
                                    || matches!(
                                        c,
                                        b'+' | b'/' | b'=' | b' ' | b'\t' | b'\r' | b'\n'
                                    )
                            })
                        {
                            bail!("Invalid PEM certificate body");
                        }
                        rest = rest[end + "-----END CERTIFICATE-----".len()..].trim();
                        count += 1;
                        if count > 32 {
                            bail!("Certificate bundle exceeds 32 certificates");
                        }
                    }
                    // The PEM reader requires BEGIN at a line boundary. Pass an
                    // ASCII-prefix view, while file data/digests retain raw bytes.
                    let pem_input = text.trim_start_matches(|c| {
                        matches!(c, ' ' | '\t' | '\r' | '\n' | '\u{000b}' | '\u{000c}')
                    });
                    let certs = Certificate::from_pem_bundle(pem_input.as_bytes())?;
                    if certs.len() != count || count == 0 {
                        bail!("Incomplete PEM certificate bundle");
                    }
                    result.extend(certs);
                }
                TrustedCertificate::Der { .. } => result.push(Certificate::from_der(&bytes)?),
            }
        }
        if result.len() > 32 {
            bail!("At most 32 trusted certificates are allowed");
        }
        Ok(result)
    }
    pub(crate) fn configure(&self, mut builder: ClientBuilder) -> Result<ClientBuilder> {
        for certificate in self.certificates()? {
            builder = builder.add_root_certificate(certificate);
        }
        Ok(builder)
    }
    pub(crate) fn digest(&self) -> Result<String> {
        self.validate()?;
        Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(self)?)))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "lowercase", deny_unknown_fields)]
pub enum TrustedCertificate {
    Pem {
        name: String,
        data: String,
    },
    /// Lowercase hexadecimal of the DER bytes; no filesystem path is persisted.
    Der {
        name: String,
        data: String,
    },
}
impl TrustedCertificate {
    fn name(&self) -> &str {
        match self {
            Self::Pem { name, .. } | Self::Der { name, .. } => name,
        }
    }
    fn decoded(&self) -> Result<Vec<u8>> {
        match self {
            Self::Pem { data, .. } => {
                if data.len() > MAX_TRUST_FILE_BYTES {
                    bail!("PEM input exceeds 65536 bytes");
                }
                Ok(data.as_bytes().to_vec())
            }
            Self::Der { data, .. } => {
                if data.len() > MAX_TRUST_FILE_BYTES * 2
                    || data.len() % 2 != 0
                    || !data
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    bail!("DER data must be bounded lowercase hexadecimal");
                }
                data.as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|p| {
                        let nibble = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
                        Ok((nibble(p[0]) << 4) | nibble(p[1]))
                    })
                    .collect()
            }
        }
    }
    pub fn from_file_bytes(name: String, bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() || bytes.len() > MAX_TRUST_FILE_BYTES {
            bail!("Certificate file must contain 1..65536 bytes");
        }
        // Match the UI file classifier's exact RFC7468 ASCII whitespace set.
        // Classify using a view only; retain every original byte in the value.
        let pem_start = bytes
            .iter()
            .position(|&b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c))
            .unwrap_or(bytes.len());
        let file = if bytes[pem_start..].starts_with(b"-----BEGIN") {
            Self::Pem {
                name,
                data: std::str::from_utf8(bytes)?.to_owned(),
            }
        } else {
            Self::Der {
                name,
                data: bytes.iter().map(|b| format!("{b:02x}")).collect(),
            }
        };
        TlsConfig {
            trusted_certificates: vec![file.clone()],
            ..Default::default()
        }
        .validate()?;
        Ok(file)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HstsSourceKind {
    Document,
    Sitemap,
    Robots,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HstsPolicy {
    pub host: String,
    pub include_subdomains: bool,
    pub learned_at_unix: u64,
    pub expires_at_unix: u64,
    pub learned_from_url: String,
    pub source_kind: HstsSourceKind,
    /// Exact first STS field accepted on verified HTTPS, before any header folding.
    pub header_value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HstsUpgrade {
    pub source_url: String,
    pub target_url: String,
    pub synthetic_status: u16,
    pub reason: String,
    /// This upgrade itself sends no network request and observes no HTTP response.
    pub network_request_made: bool,
    pub https_request_started: bool,
    pub network_redirect_index: usize,
    pub applied_at_unix: u64,
    pub policy: HstsPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crawl_policy: Option<crate::CandidateDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_gate: Option<crate::ResourceRedirectDecision>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageTlsObservation {
    pub hsts_upgrades: Vec<HstsUpgrade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HstsSnapshot {
    pub version: u32,
    pub tls_config_sha256: String,
    pub generation: u64,
    pub policies: BTreeMap<String, HstsPolicy>,
}
impl HstsSnapshot {
    pub(crate) fn validate(&self, config_hash: &str, generation: u64) -> Result<()> {
        if self.version != 1
            || self.tls_config_sha256 != config_hash
            || self.generation == 0
            || self.generation > generation
            || self.policies.len() > MAX_HSTS_HOSTS
        {
            bail!("HSTS state/config/generation mismatch");
        }
        for (key, policy) in &self.policies {
            let (age, include) =
                parse_sts(&policy.header_value).context("Invalid persisted STS field")?;
            if age == 0
                || include != policy.include_subdomains
                || policy.learned_at_unix.saturating_add(age) != policy.expires_at_unix
            {
                bail!("Persisted HSTS policy differs from its first STS field");
            }
            let url = Url::parse(&policy.learned_from_url)?;
            if key != &policy.host
                || canonical_host(&url).as_deref() != Some(key.as_str())
                || url.scheme() != "https"
                || url.as_str().len() > 65536
                || !url.username().is_empty()
                || url.password().is_some()
                || policy.expires_at_unix <= policy.learned_at_unix
            {
                bail!("Invalid persisted HSTS policy provenance");
            }
        }
        Ok(())
    }
}
struct HstsInner {
    policies: BTreeMap<String, HstsPolicy>,
    attached: Option<(Arc<crate::CrawlStore>, u64)>,
    failure: Option<String>,
}
pub(crate) struct HstsCache {
    hash: String,
    inner: Mutex<HstsInner>,
}
impl HstsCache {
    pub(crate) fn new(config: &TlsConfig) -> Result<Self> {
        if !config.respect_hsts {
            bail!("HSTS cache requires opt-in");
        }
        Ok(Self {
            hash: config.digest()?,
            inner: Mutex::new(HstsInner {
                policies: BTreeMap::new(),
                attached: None,
                failure: None,
            }),
        })
    }
    pub(crate) fn healthy(&self) -> Result<()> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("HSTS state lock poisoned"))?;
        if let Some(error) = &inner.failure {
            bail!("HSTS durable state failed: {error}");
        }
        Ok(())
    }
    pub(crate) fn attach(&self, store: Arc<crate::CrawlStore>, generation: u64) -> Result<()> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("HSTS state lock poisoned"))?;
        if let Some(error) = &inner.failure {
            bail!("HSTS durable state failed: {error}");
        }
        let state = store.load_hsts_state(&self.hash, generation)?;
        inner.policies = state.map(|s| s.policies).unwrap_or_default();
        inner.policies.retain(|_, p| p.expires_at_unix > unix_now());
        inner.attached = Some((store, generation));
        Ok(())
    }
    pub(crate) fn observe(
        &self,
        url: &Url,
        headers: &HeaderMap,
        source_kind: HstsSourceKind,
    ) -> Result<()> {
        self.observe_at(url, headers, source_kind, unix_now())
    }
    fn observe_at(
        &self,
        url: &Url,
        headers: &HeaderMap,
        source_kind: HstsSourceKind,
        now: u64,
    ) -> Result<()> {
        self.healthy()?;
        if url.scheme() != "https" {
            return Ok(());
        }
        let Some(host) = canonical_host(url) else {
            return Ok(());
        };
        // RFC6797 8.1: only the first header field; never merge multiple values.
        let Some(header) = headers.get_all("strict-transport-security").iter().next() else {
            return Ok(());
        };
        let Some(raw_header) = header.to_str().ok() else {
            return Ok(());
        };
        let Some((age, include_subdomains)) = parse_sts(raw_header) else {
            return Ok(());
        };
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("HSTS state lock poisoned"))?;
        if let Some(error) = &inner.failure {
            bail!("HSTS durable state failed: {error}");
        }
        let mut policies = inner.policies.clone();
        policies.retain(|_, p| p.expires_at_unix > now);
        if url.as_str().len() > 65536 {
            inner.failure = Some("HSTS source URL exceeds 65536 bytes".into());
            bail!("HSTS source URL exceeds 65536 bytes");
        }
        if age == 0 {
            policies.remove(&host);
        } else {
            policies.insert(
                host.clone(),
                HstsPolicy {
                    host,
                    include_subdomains,
                    learned_at_unix: now,
                    expires_at_unix: now.saturating_add(age),
                    learned_from_url: url.to_string(),
                    source_kind,
                    header_value: raw_header.to_owned(),
                },
            );
        }
        if policies.len() > MAX_HSTS_HOSTS {
            inner.failure =
                Some("HSTS host limit reached; no policy may be silently evicted".into());
            bail!("HSTS host limit reached");
        }
        if serde_json::to_vec(&policies)?.len() > 8 * 1024 * 1024 - 1024 {
            inner.failure = Some("HSTS policy state exceeds 8MiB".into());
            bail!("HSTS policy state exceeds 8MiB");
        }
        if let Some((store, generation)) = &inner.attached {
            let snapshot = HstsSnapshot {
                version: 1,
                tls_config_sha256: self.hash.clone(),
                generation: *generation,
                policies: policies.clone(),
            };
            if let Err(error) = store.save_hsts_state(&snapshot) {
                inner.failure = Some(error.to_string());
                return Err(error);
            }
        }
        inner.policies = policies;
        Ok(())
    }
    pub(crate) fn upgrade(&self, url: &Url, index: usize) -> Result<Option<HstsUpgrade>> {
        self.upgrade_at(url, index, unix_now())
    }
    fn upgrade_at(&self, url: &Url, index: usize, now: u64) -> Result<Option<HstsUpgrade>> {
        self.healthy()?;
        if url.scheme() != "http" {
            return Ok(None);
        }
        let Some(host) = canonical_host(url) else {
            return Ok(None);
        };
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("HSTS state lock poisoned"))?;
        inner.policies.retain(|_, p| p.expires_at_unix > now);
        let policy = inner
            .policies
            .values()
            .filter(|p| {
                p.host == host
                    || (p.include_subdomains
                        && host
                            .strip_suffix(&p.host)
                            .is_some_and(|prefix| prefix.ends_with('.')))
            })
            .max_by_key(|p| p.host.len())
            .cloned();
        let Some(policy) = policy else {
            return Ok(None);
        };
        let mut target = url.clone();
        // url::Url normalizes explicit :80 to None, both map to default HTTPS.
        let port = url.port().filter(|port| *port != 80);
        target
            .set_scheme("https")
            .map_err(|_| anyhow::anyhow!("Cannot apply HSTS scheme"))?;
        target
            .set_port(port)
            .map_err(|_| anyhow::anyhow!("Cannot apply HSTS port"))?;
        Ok(Some(HstsUpgrade {
            source_url: url.to_string(),
            target_url: target.to_string(),
            synthetic_status: 307,
            reason: "HSTS Policy".into(),
            network_request_made: false,
            https_request_started: false,
            network_redirect_index: index,
            applied_at_unix: now,
            policy,
            crawl_policy: None,
            resource_gate: None,
        }))
    }
}
fn canonical_host(url: &Url) -> Option<String> {
    match url.host()? {
        Host::Domain(host) => {
            let host = host.strip_suffix('.').unwrap_or(host).to_ascii_lowercase();
            (!host.is_empty()
                && host.len() <= 253
                && host.split('.').all(|label| !label.is_empty()))
            .then_some(host)
        }
        Host::Ipv4(_) | Host::Ipv6(_) => None,
    }
}
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|t| t.as_secs())
        .unwrap_or(0)
}

/// RFC6797 6.1 grammar, including quoted-string unescaping and unknown directives.
fn parse_sts(value: &str) -> Option<(u64, bool)> {
    if value.len() > 16384 || !value.is_ascii() {
        return None;
    }
    let token = |b: u8| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_~|".contains(&b) || b == 96;
    let mut directives = Vec::new();
    let mut start = 0usize;
    let mut quoted = false;
    let mut escape = false;
    for (i, b) in value.bytes().enumerate() {
        if escape {
            if b != b'\t' && !(32..=126).contains(&b) {
                return None;
            }
            escape = false;
            continue;
        }
        if quoted && b == b'\\' {
            escape = true;
            continue;
        }
        if b == b'"' {
            quoted = !quoted;
        }
        if !quoted && b == b';' {
            directives.push(&value[start..i]);
            start = i + 1;
        }
        if b != b'\t' && !(32..=126).contains(&b) {
            return None;
        }
    }
    if quoted || escape {
        return None;
    }
    directives.push(&value[start..]);
    let mut names = HashSet::new();
    let mut age = None;
    let mut subdomains = false;
    for item in directives {
        let item = item.trim_matches([' ', '\t']);
        if item.is_empty() {
            continue;
        }
        let (name, raw) = item.split_once('=').map_or((item, None), |(n, v)| {
            (
                n.trim_end_matches([' ', '\t']),
                Some(v.trim_matches([' ', '\t'])),
            )
        });
        if name.is_empty() || !name.bytes().all(token) {
            return None;
        }
        let name = name.to_ascii_lowercase();
        if !names.insert(name.clone()) {
            return None;
        }
        let decoded = if let Some(raw) = raw {
            if raw.starts_with('"') {
                let body = raw.strip_prefix('"')?.strip_suffix('"')?;
                let mut out = String::new();
                let mut escaping = false;
                for ch in body.chars() {
                    if escaping {
                        out.push(ch);
                        escaping = false;
                    } else if ch == '\\' {
                        escaping = true;
                    } else if ch == '"' {
                        return None;
                    } else {
                        out.push(ch);
                    }
                }
                if escaping {
                    return None;
                }
                Some(out)
            } else {
                if raw.is_empty() || !raw.bytes().all(token) {
                    return None;
                }
                Some(raw.to_owned())
            }
        } else {
            None
        };
        match name.as_str() {
            "max-age" => {
                let digits = decoded?;
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                age = Some(digits.bytes().fold(0u64, |n, b| {
                    n.saturating_mul(10).saturating_add((b - b'0') as u64)
                }));
            }
            "includesubdomains" => {
                if decoded.is_some() {
                    return None;
                }
                subdomains = true;
            }
            _ => {}
        }
    }
    age.map(|age| (age, subdomains))
}
#[cfg(test)]
#[path = "tls/local_tls_tests.rs"]
mod local_tls_tests;
#[cfg(test)]
#[path = "tls/tests.rs"]
mod tests;
pub(crate) fn configure_client(
    config: &crate::CrawlConfig,
    builder: ClientBuilder,
) -> Result<ClientBuilder> {
    match &config.tls {
        Some(tls) => tls.configure(builder),
        None => Ok(builder),
    }
}
