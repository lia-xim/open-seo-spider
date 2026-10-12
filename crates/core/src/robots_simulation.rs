//! Pure offline robots decisions, using the same policy/matcher as the crawler.
//! RFC 9309: https://www.rfc-editor.org/rfc/rfc9309.html#section-2.2
//! No HTTP client, robots fetch/cache status, crawl settings or remote mutations.
use crate::robots::RobotsPolicy;
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

const MAX_ROBOTS_BYTES: usize = 512 * 1024;
const MAX_URLS: usize = 10_000;
const MAX_URL_BYTES: usize = 8192;
const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_EVALUATION_BYTES: usize = 64 * 1024 * 1024;
const MAX_AGENT_BYTES: usize = 256;

pub const ROBOTS_SIMULATION_ENGINE: &str = "robotstxt-0.3.0/Google-DefaultMatcher/crawler-policy";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct RobotsSimulationLimits {
    pub max_robots_bytes: usize,
    pub max_urls: usize,
    pub max_url_bytes: usize,
    pub max_input_bytes: usize,
    /// The matcher parses the text for each row; bound this work independently.
    pub max_evaluation_bytes: usize,
}

impl Default for RobotsSimulationLimits {
    fn default() -> Self {
        Self {
            max_robots_bytes: MAX_ROBOTS_BYTES,
            max_urls: MAX_URLS,
            max_url_bytes: MAX_URL_BYTES,
            max_input_bytes: MAX_INPUT_BYTES,
            max_evaluation_bytes: MAX_EVALUATION_BYTES,
        }
    }
}

impl RobotsSimulationLimits {
    pub fn validate(&self) -> anyhow::Result<()> {
        for (name, value, ceiling) in [
            ("max_robots_bytes", self.max_robots_bytes, MAX_ROBOTS_BYTES),
            ("max_urls", self.max_urls, MAX_URLS),
            ("max_url_bytes", self.max_url_bytes, MAX_URL_BYTES),
            ("max_input_bytes", self.max_input_bytes, MAX_INPUT_BYTES),
            (
                "max_evaluation_bytes",
                self.max_evaluation_bytes,
                MAX_EVALUATION_BYTES,
            ),
        ] {
            if value == 0 || value > ceiling {
                bail!("Robots simulation {name} must be between 1 and {ceiling}");
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RobotsSimulationInput {
    pub robots_text: String,
    /// Same matcher input as a crawl's HTTP user_agent, not indexability_agent.
    pub user_agent: String,
    pub urls: Vec<String>,
    #[serde(default)]
    pub limits: RobotsSimulationLimits,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct RobotsSimulationRow {
    pub source_row: usize,
    pub input_url: String,
    pub normalized_url: Option<String>,
    /// Exact path/query presented to the installed matcher, not a rule trace.
    pub path_query: Option<String>,
    /// Retained input fragment; excluded from the crawl URL and robots matching.
    pub fragment: Option<String>,
    pub allowed: Option<bool>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct RobotsSimulation {
    pub engine: String,
    pub rules_sha256: String,
    pub rules_bytes: usize,
    pub user_agent: String,
    pub rows: Vec<RobotsSimulationRow>,
    pub website_requests: usize,
    pub ignored_directives: Vec<String>,
    pub limitations: Vec<String>,
}

fn validate(input: &RobotsSimulationInput) -> anyhow::Result<()> {
    input.limits.validate()?;
    if input.user_agent.trim().is_empty()
        || input.user_agent.len() > MAX_AGENT_BYTES
        || !input.user_agent.is_ascii()
        || input.user_agent.chars().any(char::is_control)
    {
        bail!("Robots user_agent must contain 1..256 ASCII bytes without control characters");
    }
    if input.robots_text.len() > input.limits.max_robots_bytes {
        bail!("Robots text exceeds max_robots_bytes; simulation does not silently truncate rules");
    }
    if input.urls.is_empty() || input.urls.len() > input.limits.max_urls {
        bail!("Robots simulation needs between 1 and max_urls input rows");
    }
    let mut bytes = input
        .robots_text
        .len()
        .checked_add(input.user_agent.len())
        .context("Robots input byte count overflow")?;
    for url in &input.urls {
        if url.len() > input.limits.max_url_bytes {
            bail!("Robots URL field exceeds max_url_bytes");
        }
        bytes = bytes
            .checked_add(url.len())
            .context("Robots input byte count overflow")?;
    }
    if bytes > input.limits.max_input_bytes {
        bail!("Robots input exceeds max_input_bytes");
    }
    let evaluation_bytes = input
        .robots_text
        .len()
        .max(1)
        .checked_mul(input.urls.len())
        .context("Robots evaluation byte count overflow")?;
    if evaluation_bytes > input.limits.max_evaluation_bytes {
        bail!(
            "Robots rows multiplied by rule bytes exceed max_evaluation_bytes; use smaller batches"
        );
    }
    Ok(())
}

fn ignored_directives(text: &str) -> Vec<String> {
    #[derive(Default)]
    struct Inventory(std::collections::BTreeSet<String>);
    impl robotstxt::RobotsParseHandler for Inventory {
        fn handle_robots_start(&mut self) {}
        fn handle_robots_end(&mut self) {}
        fn handle_user_agent(&mut self, _: u32, _: &str) {}
        fn handle_allow(&mut self, _: u32, _: &str) {}
        fn handle_disallow(&mut self, _: u32, _: &str) {}
        fn handle_sitemap(&mut self, _: u32, _: &str) {
            self.0.insert("sitemap".into());
        }
        fn handle_unknown_action(&mut self, _: u32, action: &str, _: &str) {
            let name = action.trim().to_ascii_lowercase();
            if !name.is_empty() && name.len() <= 128 {
                self.0.insert(name);
            }
        }
    }
    // Ask the installed parser to classify fields, including its tolerated
    // standard-key aliases; a lexical scan would falsely label useragent ignored.
    let mut inventory = Inventory::default();
    robotstxt::parse_robotstxt(text, &mut inventory);
    inventory.0.into_iter().collect()
}

fn row(policy: &RobotsPolicy, user_agent: &str, input: &str, index: usize) -> RobotsSimulationRow {
    let mut result = RobotsSimulationRow {
        source_row: index + 1,
        input_url: input.into(),
        normalized_url: None,
        path_query: None,
        fragment: None,
        allowed: None,
        error: None,
    };
    let parsed = if input.chars().any(char::is_control) {
        result.error = Some("URL contains raw control characters".into());
        return result;
    } else {
        match Url::parse(input.trim()) {
            Ok(url) => url,
            Err(error) => {
                result.error = Some(format!("Invalid absolute URL: {error}"));
                return result;
            }
        }
    };
    result.fragment = parsed.fragment().map(str::to_owned);
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        result.error = Some("Only absolute HTTP/HTTPS URLs with a host are supported".into());
        return result;
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        result.error = Some("URLs containing userinfo are unsupported, as in the crawler".into());
        return result;
    }
    let Some(url) = crate::normalize_url(input, None) else {
        result.error = Some("URL cannot be normalized by the crawler".into());
        return result;
    };
    result.path_query = Some(robotstxt::get_path_params_query(url.as_str()).into_owned());
    result.allowed = Some(policy.allowed(user_agent, url.as_str()));
    result.normalized_url = Some(url.into());
    result
}

pub fn simulate_robots(input: &RobotsSimulationInput) -> anyhow::Result<RobotsSimulation> {
    validate(input)?;
    let policy = RobotsPolicy {
        text: input.robots_text.clone(),
        ..Default::default()
    };
    Ok(RobotsSimulation {
        engine: ROBOTS_SIMULATION_ENGINE.into(),
        rules_sha256: format!("{:x}", Sha256::digest(input.robots_text.as_bytes())),
        rules_bytes: input.robots_text.len(),
        user_agent: input.user_agent.clone(),
        rows: input
            .urls
            .iter()
            .enumerate()
            .map(|(index, url)| row(&policy, &input.user_agent, url, index))
            .collect(),
        website_requests: 0,
        ignored_directives: ignored_directives(&input.robots_text),
        limitations: vec![
            "Only pasted UTF-8 robots text is simulated. HTTP retrieval, cache state, redirects, robots fetch failures, site scope and meta/X-Robots-Tag indexability are not simulated.".into(),
            "Allow/Disallow and User-agent decisions use the crawler's installed Google DefaultMatcher. Crawl-delay, Request-rate, Host, Noindex and Sitemap do not change this access decision; no sitemap is fetched.".into(),
            "Input URL order/duplicates and fragments are retained. Matching uses the crawler-normalized HTTP URL with fragments removed and path/query preserved; no additional percent-decoding is applied, and encoded-unreserved RFC equivalence is not asserted.".into(),
            "The matcher does not expose a verified winning rule/line trace. No matched-rule explanation is fabricated; ignored_directives lists parser-classified nondecision fields, not a complete syntax validator.".into(),
        ],
    })
}

pub fn export_robots_simulation_csv(input: &RobotsSimulationInput) -> anyhow::Result<String> {
    let simulation = simulate_robots(input)?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "Source Row",
        "Input URL",
        "Normalized URL",
        "Path Query",
        "Input Fragment",
        "Allowed",
        "Error",
        "User Agent",
        "Rules SHA256",
        "Engine",
    ])?;
    for row in simulation.rows {
        writer.write_record([
            row.source_row.to_string(),
            crate::export::csv_text(&row.input_url),
            crate::export::csv_text(row.normalized_url.as_deref().unwrap_or_default()),
            crate::export::csv_text(row.path_query.as_deref().unwrap_or_default()),
            crate::export::csv_text(row.fragment.as_deref().unwrap_or_default()),
            row.allowed
                .map(|allowed| allowed.to_string())
                .unwrap_or_default(),
            crate::export::csv_text(row.error.as_deref().unwrap_or_default()),
            crate::export::csv_text(&simulation.user_agent),
            simulation.rules_sha256.clone(),
            simulation.engine.clone(),
        ])?;
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(rules: &str, agent: &str, paths: &[&str]) -> RobotsSimulationInput {
        RobotsSimulationInput {
            robots_text: rules.into(),
            user_agent: agent.into(),
            urls: paths
                .iter()
                .map(|path| format!("https://example.test{path}"))
                .collect(),
            limits: Default::default(),
        }
    }

    #[test]
    fn specific_agents_merged_groups_longest_allow_ties_and_case_match_shared_crawler() {
        let rules = "User-agent: *\nDisallow: /wild\nUser-agent: ExampleBot\nDisallow: /private\nAllow: /private/public\nDisallow: /tie\nAllow: /tie\nUser-agent: EXAMPLEBOT\nDisallow: /second\n";
        let request = input(
            rules,
            "ExampleBot",
            &[
                "/private",
                "/private/public/a",
                "/tie",
                "/second",
                "/wild",
                "/Private",
            ],
        );
        let result = simulate_robots(&request).unwrap();
        assert_eq!(
            result
                .rows
                .iter()
                .map(|row| row.allowed)
                .collect::<Vec<_>>(),
            vec![
                Some(false),
                Some(true),
                Some(true),
                Some(false),
                Some(true),
                Some(true)
            ]
        );
        let policy = RobotsPolicy {
            text: rules.into(),
            ..Default::default()
        };
        for row in &result.rows {
            assert_eq!(
                row.allowed,
                Some(policy.allowed(&request.user_agent, row.normalized_url.as_deref().unwrap()))
            );
        }
        assert_eq!(result.website_requests, 0);
    }

    #[test]
    fn wildcard_end_anchor_query_fragments_order_and_duplicate_inputs_are_observed() {
        let request = input(
            "User-agent: *\nDisallow: /*.pdf$\nDisallow: /search?blocked=1$\n",
            "TestBot",
            &[
                "/a.pdf#one",
                "/a.pdf?download=1",
                "/search?blocked=1#two",
                "/search?blocked=1&other=2",
                "/a.pdf#one",
            ],
        );
        let result = simulate_robots(&request).unwrap();
        assert_eq!(
            result
                .rows
                .iter()
                .map(|row| row.allowed)
                .collect::<Vec<_>>(),
            vec![
                Some(false),
                Some(true),
                Some(false),
                Some(true),
                Some(false)
            ]
        );
        assert_eq!(result.rows[0].fragment.as_deref(), Some("one"));
        assert_eq!(result.rows[0].path_query.as_deref(), Some("/a.pdf"));
        assert_eq!(
            result.rows[2].path_query.as_deref(),
            Some("/search?blocked=1")
        );
        assert_eq!(result.rows[0].input_url, result.rows[4].input_url);
        assert_eq!(result.rows[4].source_row, 5);
    }

    #[test]
    fn unicode_and_percent_encoded_urls_retain_crawler_encoding_without_a_fake_rule_trace() {
        let request = input(
            "User-agent: *\nDisallow: /caf%C3%A9\nDisallow: /reserved%2Fpart\n",
            "Bot",
            &[
                "/café?a=2&a=1#über",
                "/reserved%2Fpart",
                "/reserved/part",
                "/%62ar",
            ],
        );
        let result = simulate_robots(&request).unwrap();
        assert_eq!(result.rows[0].allowed, Some(false));
        assert_eq!(
            result.rows[0].normalized_url.as_deref(),
            Some("https://example.test/caf%C3%A9?a=2&a=1")
        );
        assert_eq!(result.rows[0].fragment.as_deref(), Some("%C3%BCber"));
        assert_eq!(result.rows[1].allowed, Some(false));
        assert_eq!(result.rows[2].allowed, Some(true));
        assert_eq!(result.rows[3].path_query.as_deref(), Some("/%62ar"));
        assert!(
            result
                .limitations
                .iter()
                .any(|text| text.contains("encoded-unreserved"))
        );
        assert!(
            serde_json::to_value(result)
                .unwrap()
                .get("matched_rule")
                .is_none()
        );
    }

    #[test]
    fn invalid_rows_are_unknown_without_dropping_or_deduplicating_them() {
        let mut request = input("User-agent: *\nDisallow: /\n", "Bot", &[]);
        request.urls = [
            "",
            "not a URL",
            "ftp://example.test/a",
            "https://user:secret@example.test/a",
            "https://example.test/a\n",
            "https://EXAMPLE.test:443/a#frag",
            "not a URL",
        ]
        .map(str::to_owned)
        .into();
        let result = simulate_robots(&request).unwrap();
        assert_eq!(result.rows.len(), 7);
        for row in result.rows.iter().filter(|row| row.source_row != 6) {
            assert!(row.allowed.is_none() && row.error.is_some() && row.normalized_url.is_none());
        }
        assert_eq!(result.rows[5].allowed, Some(false));
        assert_eq!(
            result.rows[5].normalized_url.as_deref(),
            Some("https://example.test/a")
        );
        assert_eq!(result.rows[1].input_url, result.rows[6].input_url);
    }

    #[test]
    fn empty_rules_ignored_directives_and_hash_preserve_actual_pasted_bytes() {
        let empty = simulate_robots(&input("", "Bot", &["/"])).unwrap();
        assert_eq!(empty.rows[0].allowed, Some(true));
        assert_eq!(
            empty.rules_sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let rules = "User-agent: *\nCrawl-delay: 9\nNoindex: /\nSitemap: https://example.test/map.xml\nHost: other.test\nrequest-rate: 1/3\n# Disallow: /\nAllow: /\n";
        let result = simulate_robots(&input(rules, "Bot", &["/"])).unwrap();
        assert_eq!(result.rows[0].allowed, Some(true));
        assert_eq!(
            result.ignored_directives,
            ["crawl-delay", "host", "noindex", "request-rate", "sitemap"]
        );
        assert_eq!(result.rules_bytes, rules.len());
        assert_ne!(
            result.rules_sha256,
            simulate_robots(&input(&(rules.to_owned() + "\n"), "Bot", &["/"]))
                .unwrap()
                .rules_sha256
        );
    }

    #[test]
    fn limits_are_positive_bounded_and_enforced_without_silent_truncation() {
        let base = input("User-agent: *\nAllow: /\n", "Bot", &["/"]);
        let mut bad = base.clone();
        bad.limits.max_urls = 0;
        assert!(simulate_robots(&bad).is_err());
        bad = base.clone();
        bad.limits.max_robots_bytes = MAX_ROBOTS_BYTES + 1;
        assert!(simulate_robots(&bad).is_err());
        bad = base.clone();
        bad.limits.max_robots_bytes = 1;
        assert!(simulate_robots(&bad).is_err());
        bad = base.clone();
        bad.limits.max_url_bytes = 1;
        assert!(simulate_robots(&bad).is_err());
        bad = base.clone();
        bad.limits.max_input_bytes = 1;
        assert!(simulate_robots(&bad).is_err());
        bad = base.clone();
        bad.limits.max_evaluation_bytes = 1;
        assert!(simulate_robots(&bad).is_err());
        bad = base.clone();
        bad.urls.clear();
        assert!(simulate_robots(&bad).is_err());
        bad = base.clone();
        bad.user_agent = " \t".into();
        assert!(simulate_robots(&bad).is_err());
        bad = base.clone();
        bad.user_agent = "éBot".into();
        assert!(simulate_robots(&bad).is_err());
        bad = base;
        bad.user_agent = "B".repeat(MAX_AGENT_BYTES + 1);
        assert!(simulate_robots(&bad).is_err());
        assert!(serde_json::from_str::<RobotsSimulationLimits>("{\"unknown\":1}").is_err());
        assert!(serde_json::from_str::<RobotsSimulationInput>("{\"robots_text\":\"\",\"user_agent\":\"Bot\",\"urls\":[\"https://example.test/\"],\"fetch\":true}").is_err());
    }

    #[test]
    fn csv_protects_formula_like_input_and_retains_unknown_instead_of_false() {
        let mut request = input("User-agent: *\nAllow: /\n", "Bot", &["/"]);
        request.urls = vec![
            " \t=FORMULA()".into(),
            "https://example.test/?a=1&b=2#@fragment".into(),
        ];
        let csv = export_robots_simulation_csv(&request).unwrap();
        let records = csv::Reader::from_reader(csv.as_bytes())
            .records()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(&records[0][1], "' \t=FORMULA()");
        assert_eq!(&records[0][5], "");
        assert!(!records[0][6].is_empty());
        assert_eq!(&records[1][4], "'@fragment");
        assert_eq!(&records[1][5], "true");
        assert_eq!(
            simulate_robots(&request).unwrap().rows[0].input_url,
            request.urls[0]
        );
    }
}
