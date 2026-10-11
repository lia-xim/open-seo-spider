use quick_xml::{Reader, events::Event};

#[derive(Debug, Clone, Default)]
pub struct SitemapEntries {
    pub is_index: bool,
    pub locations: Vec<String>,
}

/// Reads URL sets and sitemap indexes without executing XML external entities.
pub fn parse_sitemap(xml: &str) -> anyhow::Result<SitemapEntries> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut result = SitemapEntries::default();
    let mut in_loc = false;
    let mut location = String::new();
    let mut root_seen = false;
    let mut depth = 0usize;
    loop {
        match reader.read_event()? {
            Event::Start(e) => {
                if depth == 0 {
                    if root_seen {
                        anyhow::bail!("Sitemap has multiple document roots");
                    }
                    root_seen = true;
                    match e.local_name().as_ref() {
                        "sitemapindex" => result.is_index = true,
                        "urlset" => {}
                        _ => anyhow::bail!("Expected sitemapindex or urlset document root"),
                    }
                }
                depth += 1;
                if e.local_name().as_ref() == "loc" {
                    in_loc = true;
                    location.clear();
                }
            }
            Event::Empty(e) if depth == 0 => {
                if root_seen {
                    anyhow::bail!("Sitemap has multiple document roots");
                }
                root_seen = true;
                match e.local_name().as_ref() {
                    "sitemapindex" => result.is_index = true,
                    "urlset" => {}
                    _ => anyhow::bail!("Expected sitemapindex or urlset document root"),
                }
            }
            Event::Text(text) if in_loc => location.push_str(&text),
            Event::CData(text) if in_loc => location.push_str(&text),
            Event::GeneralRef(reference) if in_loc => {
                if let Some(character) = reference.resolve_char_ref()? {
                    location.push(character);
                } else {
                    location.push_str(&quick_xml::escape::unescape(&format!("&{};", &*reference))?);
                }
            }
            Event::End(e) => {
                depth = depth.saturating_sub(1);
                if e.local_name().as_ref() == "loc" {
                    in_loc = false;
                    if !location.trim().is_empty() {
                        result.locations.push(location.trim().into());
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !root_seen || depth != 0 {
        anyhow::bail!("Sitemap document is empty or incomplete");
    }
    Ok(result)
}

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
