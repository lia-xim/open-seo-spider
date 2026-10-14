//! Bounded sitemap XML observations and gzip file-entity decoding.
//! Pure bounded XML/entity inspection; no request, storage or source-authenticity claim.
#[path = "sitemap/count_models.rs"]
mod count_models;
#[path = "sitemap/entity.rs"]
mod entity;
#[path = "sitemap/entity_observation.rs"]
mod entity_observation;
pub use count_models::*;
#[path = "sitemap/models.rs"]
mod models;
#[path = "sitemap/xml.rs"]
mod xml;
pub use entity::{
    SitemapEntityDecodeOutcome, decode_sitemap_entity, decode_sitemap_entity_observed,
};
pub use entity_observation::*;
pub use models::*;
pub use xml::{count_sitemap_xml, observe_sitemap_xml};

#[derive(Debug, Clone, Default)]
pub struct SitemapEntries {
    pub is_index: bool,
    pub locations: Vec<String>,
}

/// Existing signature retained for ordinary/policy callers. New callers must
/// retain the observation, not this intentionally lossy compatibility projection.
/// Invalid URI values are deliberately passed to the existing policy evaluator.
pub fn parse_sitemap(xml: &str) -> anyhow::Result<SitemapEntries> {
    let observation = observe_sitemap_xml(
        xml.as_bytes(),
        SitemapParserContext::default(),
        &SitemapParserLimits::default(),
    )
    .map_err(|error| anyhow::anyhow!("{}: {}", error.code, error.message))?;
    if !observation.complete {
        let error = observation
            .failure
            .expect("incomplete observations have a failure");
        anyhow::bail!("{}: {}", error.code, error.message);
    }
    Ok(SitemapEntries {
        is_index: observation.kind == Some(SitemapDocumentKind::SitemapIndex),
        locations: observation
            .locations
            .into_iter()
            .filter_map(|row| row.decoded_value)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .collect(),
    })
}

#[cfg(test)]
#[path = "sitemap/tests.rs"]
mod occurrence_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_indexes_and_escaped_locations() {
        let sitemap = parse_sitemap("<sitemapindex><sitemap><loc>https://e.org/a.xml?a=1&amp;b=2</loc></sitemap></sitemapindex>").unwrap();
        assert!(sitemap.is_index);
        assert_eq!(sitemap.locations, vec!["https://e.org/a.xml?a=1&b=2"]);
        assert!(parse_sitemap("<urlset><loc>bad</wrong>").is_err());
        assert!(parse_sitemap("<html><loc>https://e.org/</loc></html>").is_err());
        assert!(parse_sitemap("").is_err());
        assert!(parse_sitemap("<urlset>").is_err());
        assert!(parse_sitemap("<urlset/>").is_ok());
    }
}

#[cfg(test)]
#[path = "sitemap/count_tests.rs"]
mod count_tests;
