use super::models::*;
use flate2::bufread::MultiGzDecoder;
use sha2::{Digest, Sha256};
use std::io::Read;
fn gzip(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x1f, 0x8b])
}

/// Input must be actual bytes AFTER the existing HTTP Content-Decoding layer.
/// Does not fetch, inspect URL suffixes or recursively decode gzip-in-gzip.
pub fn decode_sitemap_entity(
    input: &[u8],
    limits: &SitemapParserLimits,
) -> Result<SitemapEntityDecode, SitemapFailure> {
    limits.validate()?;
    if input.len() > limits.max_entity_bytes {
        return Err(SitemapFailure::new(
            "sitemap_entity_byte_budget",
            "Post-HTTP-decoding entity exceeds byte budget",
            None,
        ));
    }
    let (encoding, xml) = if gzip(input) {
        let mut decoder = MultiGzDecoder::new(input);
        let mut output = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            // Always read the sentinel byte / true EOF even at the exact cap.
            // EOF validates footer, checksum, ISIZE and all later members.
            let remaining = limits.max_xml_bytes - output.len();
            let requested = chunk.len().min(remaining + 1);
            let count = decoder.read(&mut chunk[..requested]).map_err(|error| {
                SitemapFailure::new(
                    "sitemap_invalid_gzip_entity",
                    format!("Gzip entity did not complete: {error}"),
                    None,
                )
            })?;
            if count == 0 {
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
        if !decoder.into_inner().is_empty() {
            return Err(SitemapFailure::new(
                "sitemap_invalid_gzip_entity",
                "Gzip entity has unconsumed trailing bytes",
                None,
            ));
        }
        if gzip(&output) {
            return Err(SitemapFailure::new(
                "sitemap_nested_gzip_entity",
                "Recursive gzip file entities are unsupported",
                None,
            ));
        }
        (SitemapEntityEncoding::GzipFileEntity, output)
    } else {
        if input.len() > limits.max_xml_bytes {
            return Err(SitemapFailure::new(
                "sitemap_xml_byte_budget",
                "XML input exceeds byte budget",
                None,
            ));
        }
        (SitemapEntityEncoding::Identity, input.to_vec())
    };
    Ok(SitemapEntityDecode {
        encoding,
        post_http_decode_entity_bytes: input.len(),
        decoded_xml_bytes: xml.len(),
        decoded_xml_sha256: format!("{:x}", Sha256::digest(&xml)),
        xml,
    })
}
