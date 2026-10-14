use super::{entity_observation::*, models::*};
use flate2::bufread::MultiGzDecoder;
use sha2::{Digest, Sha256};
use std::io::Read;
fn gzip(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x1f, 0x8b])
}

/// Result/XML ownership remains separate from the compact serializable observation.
#[derive(Debug, Clone)]
pub struct SitemapEntityDecodeOutcome {
    pub result: Result<SitemapEntityDecode, SitemapFailure>,
    pub observation: SitemapEntityDecodeObservation,
}

struct ReturnedProgress {
    bytes: u64,
    hash: Sha256,
}
impl ReturnedProgress {
    fn new() -> Self {
        Self {
            bytes: 0,
            hash: Sha256::new(),
        }
    }
    fn returned(&mut self, bytes: &[u8]) {
        // The validated bounded XML ceiling plus one real sentinel fits u64.
        self.bytes = self
            .bytes
            .checked_add(u64::try_from(bytes.len()).expect("bounded read fits u64"))
            .expect("bounded decoder progress fits u64");
        self.hash.update(bytes);
    }
}

/// Input must be actual bytes AFTER the existing HTTP Content-Decoding layer.
/// Does not fetch, inspect URL suffixes or recursively decode gzip-in-gzip.
/// Legacy callers keep the exact Result/error messages/XML bytes and discard facts.
pub fn decode_sitemap_entity(
    input: &[u8],
    limits: &SitemapParserLimits,
) -> Result<SitemapEntityDecode, SitemapFailure> {
    decode_sitemap_entity_observed(input, limits).result
}

/// Observe this supplied slice on success AND failure without attesting that its
/// HTTP transport was complete. A caller must not invoke this for a retained
/// truncated entity merely to invent decoded source measurements.
pub fn decode_sitemap_entity_observed(
    input: &[u8],
    limits: &SitemapParserLimits,
) -> SitemapEntityDecodeOutcome {
    let mut observation = SitemapEntityDecodeObservation {
        schema_version: 1,
        engine: SITEMAP_ENTITY_DECODE_OBSERVATION_ENGINE.into(),
        supplied_post_http_entity_slice_bytes: u64::try_from(input.len())
            .expect("supported-target slice length fits u64"),
        encoding: None,
        effective_limits: None,
        decoded_bytes_returned: None,
        decoded_prefix_sha256: None,
        decoded_complete_sha256: None,
        decoder_eof_observed: None,
        all_gzip_members_validated: None,
        integrity: SitemapEntityDecodeIntegrity::NotObserved,
        stop_code: None,
        read_error_kind: None,
    };
    let mut progress = None;
    let result = decode_observed_inner(input, limits, &mut observation, &mut progress);
    if let Some(progress) = progress {
        observation.decoded_bytes_returned = Some(progress.bytes);
        observation.decoded_prefix_sha256 = Some(format!("{:x}", progress.hash.finalize()));
    }
    match &result {
        Ok(decoded) => {
            observation.integrity = SitemapEntityDecodeIntegrity::Complete;
            observation.decoded_complete_sha256 = Some(decoded.decoded_xml_sha256.clone());
        }
        Err(error) => observation.stop_code = Some(error.code.clone()),
    }
    SitemapEntityDecodeOutcome {
        result,
        observation,
    }
}

fn read_gzip_xml<R: Read>(
    decoder: &mut R,
    limits: &SitemapParserLimits,
    observation: &mut SitemapEntityDecodeObservation,
    progress: &mut ReturnedProgress,
) -> Result<Vec<u8>, SitemapFailure> {
    let mut output = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        // Keep the legacy exact-cap sentinel / true-EOF read and buffer sizes.
        let remaining = limits.max_xml_bytes - output.len();
        let requested = chunk.len().min(remaining + 1);
        let count = match decoder.read(&mut chunk[..requested]) {
            Ok(count) => count,
            Err(error) => {
                observation.read_error_kind = Some(match error.kind() {
                    std::io::ErrorKind::UnexpectedEof => {
                        SitemapEntityDecodeReadErrorKind::UnexpectedEof
                    }
                    std::io::ErrorKind::InvalidInput => {
                        SitemapEntityDecodeReadErrorKind::InvalidInput
                    }
                    std::io::ErrorKind::InvalidData => {
                        SitemapEntityDecodeReadErrorKind::InvalidData
                    }
                    _ => SitemapEntityDecodeReadErrorKind::Other,
                });
                observation.integrity = if error.kind() == std::io::ErrorKind::UnexpectedEof {
                    SitemapEntityDecodeIntegrity::Incomplete
                } else {
                    SitemapEntityDecodeIntegrity::Failed
                };
                // Read::Err has no returned-byte count. Do not inspect/hash chunk:
                // even a decoder that wrote to it has not returned those bytes.
                return Err(SitemapFailure::new(
                    "sitemap_invalid_gzip_entity",
                    format!("Gzip entity did not complete: {error}"),
                    None,
                ));
            }
        };
        progress.returned(&chunk[..count]);
        if count == 0 {
            observation.decoder_eof_observed = Some(true);
            break;
        }
        if count > remaining {
            return Err(SitemapFailure::new(
                "sitemap_xml_inflation_byte_budget",
                "Actual combined inflated bytes exceed XML byte budget",
                None,
            ));
        }
        output.extend_from_slice(&chunk[..count]);
    }
    Ok(output)
}

fn decode_observed_inner(
    input: &[u8],
    limits: &SitemapParserLimits,
    observation: &mut SitemapEntityDecodeObservation,
    progress: &mut Option<ReturnedProgress>,
) -> Result<SitemapEntityDecode, SitemapFailure> {
    limits.validate()?;
    observation.effective_limits = Some(SitemapEntityDecodeEffectiveLimits {
        max_entity_bytes: u64::try_from(limits.max_entity_bytes).expect("validated limit fits u64"),
        max_xml_bytes: u64::try_from(limits.max_xml_bytes).expect("validated limit fits u64"),
    });
    if input.len() > limits.max_entity_bytes {
        return Err(SitemapFailure::new(
            "sitemap_entity_byte_budget",
            "Post-HTTP-decoding entity exceeds byte budget",
            None,
        ));
    }
    let (encoding, xml) = if gzip(input) {
        observation.encoding = Some(SitemapEntityEncoding::GzipFileEntity);
        observation.integrity = SitemapEntityDecodeIntegrity::Incomplete;
        observation.decoder_eof_observed = Some(false);
        observation.all_gzip_members_validated = Some(false);
        let mut decoder = MultiGzDecoder::new(input);
        let returned = progress.insert(ReturnedProgress::new());
        let output = read_gzip_xml(&mut decoder, limits, observation, returned)?;
        if !decoder.into_inner().is_empty() {
            observation.integrity = SitemapEntityDecodeIntegrity::Failed;
            return Err(SitemapFailure::new(
                "sitemap_invalid_gzip_entity",
                "Gzip entity has unconsumed trailing bytes",
                None,
            ));
        }
        observation.all_gzip_members_validated = Some(true);
        if gzip(&output) {
            observation.integrity = SitemapEntityDecodeIntegrity::Failed;
            return Err(SitemapFailure::new(
                "sitemap_nested_gzip_entity",
                "Recursive gzip file entities are unsupported",
                None,
            ));
        }
        (SitemapEntityEncoding::GzipFileEntity, output)
    } else {
        observation.encoding = Some(SitemapEntityEncoding::Identity);
        if input.len() > limits.max_xml_bytes {
            return Err(SitemapFailure::new(
                "sitemap_xml_byte_budget",
                "XML input exceeds byte budget",
                None,
            ));
        }
        let xml = input.to_vec();
        let returned = progress.insert(ReturnedProgress::new());
        returned.returned(&xml);
        (SitemapEntityEncoding::Identity, xml)
    };
    // Identity and gzip success both record every returned XML byte in progress.
    // Finalize a cloned state; do not hash the complete XML a second time.
    let decoded_xml_sha256 = format!(
        "{:x}",
        progress
            .as_ref()
            .expect("successful identity/gzip decode has complete returned-byte progress")
            .hash
            .clone()
            .finalize()
    );
    Ok(SitemapEntityDecode {
        encoding,
        post_http_decode_entity_bytes: input.len(),
        decoded_xml_bytes: xml.len(),
        decoded_xml_sha256,
        xml,
    })
}

#[cfg(test)]
#[path = "entity_observation_tests.rs"]
mod observation_tests;
