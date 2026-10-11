use regex::{Regex, RegexBuilder};
use url::Url;

pub(crate) struct UrlFilters {
    include: Vec<Regex>,
    exclude: Vec<Regex>,
}

impl UrlFilters {
    pub fn new(include: &[String], exclude: &[String]) -> anyhow::Result<Self> {
        if include.len() + exclude.len() > 128 {
            anyhow::bail!("At most 128 URL regex filters are supported per crawl");
        }
        let compile = |patterns: &[String]| -> anyhow::Result<Vec<Regex>> {
            patterns
                .iter()
                .map(|pattern| {
                    if pattern.len() > 8192 {
                        anyhow::bail!("URL regex exceeds 8192 bytes");
                    }
                    RegexBuilder::new(pattern)
                        .size_limit(1024 * 1024)
                        .dfa_size_limit(1024 * 1024)
                        .build()
                        .map_err(|error| anyhow::anyhow!("Invalid URL filter {pattern:?}: {error}"))
                })
                .collect()
        };
        Ok(Self {
            include: compile(include)?,
            exclude: compile(exclude)?,
        })
    }
    pub fn allows(&self, url: &str) -> bool {
        (self.include.is_empty() || self.include.iter().any(|pattern| pattern.is_match(url)))
            && !self.exclude.iter().any(|pattern| pattern.is_match(url))
    }
}

/// Normalizes URL syntax while preserving query order, path case and trailing slash.
/// Those can change server behavior and must not be merged speculatively.
pub fn normalize_url(input: &str, base: Option<&Url>) -> Option<Url> {
    let mut url = match base {
        Some(base) => base.join(input.trim()),
        None => Url::parse(input.trim()),
    }
    .ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    url.set_fragment(None);
    Some(url)
}

pub(crate) fn same_scope(url: &Url, seeds: &[Url], subdomains: bool) -> bool {
    seeds.iter().any(|seed| {
        let candidate = url.host_str().unwrap_or("");
        let root = seed.host_str().unwrap_or("");
        let host_matches =
            candidate == root || (subdomains && candidate.ends_with(&format!(".{root}")));
        host_matches && url.port() == seed.port()
    })
}

pub(crate) fn origin(url: &Url) -> String {
    url.origin().ascii_serialization()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_without_merging_semantic_variants() {
        let base = Url::parse("https://Example.org:443/a/b").unwrap();
        assert_eq!(
            normalize_url("../c#x", Some(&base)).unwrap().as_str(),
            "https://example.org/c"
        );
        assert_ne!(
            normalize_url("/?a=1&b=2", Some(&base)),
            normalize_url("/?b=2&a=1", Some(&base))
        );
        assert!(normalize_url("javascript:alert(1)", Some(&base)).is_none());
        assert!(normalize_url("https://user:secret@example.org", None).is_none());
    }
    #[test]
    fn subdomain_boundary_is_exact() {
        let seeds = vec![Url::parse("https://example.org/").unwrap()];
        assert!(same_scope(
            &Url::parse("https://a.example.org").unwrap(),
            &seeds,
            true
        ));
        assert!(!same_scope(
            &Url::parse("https://badexample.org").unwrap(),
            &seeds,
            true
        ));
        assert!(!same_scope(
            &Url::parse("https://example.org.attacker.test").unwrap(),
            &seeds,
            true
        ));
    }
}
