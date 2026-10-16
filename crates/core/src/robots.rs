use crate::robots_sitemap_source::{ROBOTS_BODY_CAP, RobotsBodyBuffer, RobotsSitemapSource};
use crate::urls::origin;
use anyhow::Context;
use reqwest::Client;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use url::Url;

#[derive(Debug, Clone, Default)]
pub(crate) struct RobotsPolicy {
    pub text: String,
    pub deny_all: bool,
    pub warning: Option<String>,
    pub sitemaps: Vec<String>,
    pub sitemap_source: Option<RobotsSitemapSource>,
}

impl RobotsPolicy {
    pub fn allowed(&self, agent: &str, url: &str) -> bool {
        !self.deny_all
            && robotstxt::DefaultMatcher::default()
                .one_agent_allowed_by_robots(&self.text, agent, url)
    }
}

pub(crate) struct RobotsCache {
    client: Client,
    hsts: Option<Arc<crate::tls::HstsCache>>,
    policies: Mutex<HashMap<String, Arc<RobotsPolicy>>>,
}

impl RobotsCache {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            hsts: None,
            policies: Mutex::new(HashMap::new()),
        }
    }
    pub(crate) fn with_hsts(client: Client, hsts: Option<Arc<crate::tls::HstsCache>>) -> Self {
        Self {
            client,
            hsts,
            policies: Mutex::new(HashMap::new()),
        }
    }
    async fn response(&self, url: &str) -> anyhow::Result<reqwest::Response> {
        let Some(hsts) = &self.hsts else {
            return Ok(self.client.get(url).send().await?);
        };
        let mut current = Url::parse(url)?;
        let mut visited = std::collections::HashSet::new();
        for redirects in 0..=5 {
            if let Some(upgrade) = hsts.upgrade(&current, redirects)? {
                current = Url::parse(&upgrade.target_url)?;
            }
            if !visited.insert(current.to_string()) {
                anyhow::bail!("robots HTTP/HSTS redirect loop");
            }
            let response = self.client.get(current.clone()).send().await?;
            if hsts
                .observe(
                    response.url(),
                    response.headers(),
                    crate::HstsSourceKind::Robots,
                )
                .is_err()
            {
                // Retain the actual response/status/body. Sticky cache failure stops
                // the page send and terminal checkpoint, and no redirect is followed.
                return Ok(response);
            }
            if matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
                if redirects == 5 {
                    anyhow::bail!("robots redirect limit exceeded");
                }
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .context("robots redirect has no Location")?
                    .to_str()?;
                current = crate::normalize_url(location, Some(&current))
                    .context("robots redirect has invalid HTTP(S) Location")?;
            } else {
                return Ok(response);
            }
        }
        unreachable!("bounded robots redirect loop")
    }
    pub async fn get(&self, url: &Url) -> Arc<RobotsPolicy> {
        let origin = origin(url);
        // Prevent duplicate robots requests when a whole origin is scheduled at once.
        let mut cache = self.policies.lock().await;
        if let Some(policy) = cache.get(&origin) {
            return policy.clone();
        }
        let robots_url = format!("{origin}/robots.txt");
        let mut policy = RobotsPolicy::default();
        match self.response(&robots_url).await {
            Ok(mut response) if response.status().is_success() => {
                let final_url = response.url().to_string();
                let status = response.status().as_u16();
                let mut body = RobotsBodyBuffer::default();
                // Preserve 512KiB of matching input. Exact fit still needs true EOF;
                // actual overflow retains a qualified prefix and stops the transfer.
                loop {
                    match response.chunk().await {
                        Ok(Some(chunk)) => {
                            if !body.push_chunk(&chunk) {
                                break;
                            }
                        }
                        Ok(None) => {
                            body.observe_eof();
                            break;
                        }
                        Err(error) => {
                            let message = format!("robots.txt body failed: {error}");
                            body.read_failure = Some(message.clone());
                            // A new EOF probe after the historical matching cap must not change allow/deny behavior.
                            if body.bytes.len() < ROBOTS_BODY_CAP {
                                policy.warning = Some(message);
                            }
                            break;
                        }
                    }
                }
                policy.deny_all = body.read_failure.is_some() && body.bytes.len() < ROBOTS_BODY_CAP;
                policy.text = String::from_utf8_lossy(&body.bytes).into_owned();
                policy.sitemap_source = Some(RobotsSitemapSource::observed(
                    robots_url.clone(),
                    final_url,
                    status,
                    &body,
                    url,
                ));
                policy.sitemaps = policy
                    .sitemap_source
                    .as_ref()
                    .expect("observed robots")
                    .legacy_sitemaps
                    .clone();
            }
            Ok(response)
                if response.status().is_client_error() && response.status().as_u16() != 429 =>
            {
                policy.sitemap_source = Some(RobotsSitemapSource::unavailable(
                    robots_url.clone(),
                    true,
                    Some(response.url().to_string()),
                    Some(response.status().as_u16()),
                    format!("robots_http_status_{}", response.status().as_u16()),
                ));
            }
            Ok(response) => {
                policy.sitemap_source = Some(RobotsSitemapSource::unavailable(
                    robots_url.clone(),
                    true,
                    Some(response.url().to_string()),
                    Some(response.status().as_u16()),
                    format!("robots_http_status_{}", response.status().as_u16()),
                ));
                policy.deny_all = true;
                policy.warning = Some(format!(
                    "robots.txt returned {}; this origin was skipped",
                    response.status()
                ));
            }
            Err(error) => {
                policy.sitemap_source = Some(RobotsSitemapSource::unavailable(
                    robots_url.clone(),
                    true,
                    None,
                    None,
                    "robots_transport_failed".into(),
                ));
                policy.deny_all = true;
                policy.warning = Some(format!(
                    "robots.txt unavailable: {error}; this origin was skipped"
                ));
            }
        }
        let policy = Arc::new(policy);
        cache.insert(origin, policy.clone());
        policy
    }
}

pub(crate) struct RateLimiter {
    delay: Duration,
    next: Mutex<HashMap<String, Instant>>,
}
impl RateLimiter {
    pub fn new(delay_ms: u64) -> Self {
        Self {
            delay: Duration::from_millis(delay_ms),
            next: Mutex::new(HashMap::new()),
        }
    }
    pub async fn wait(&self, url: &Url) {
        if self.delay.is_zero() {
            return;
        }
        let scheduled = {
            let mut next = self.next.lock().await;
            let slot = next.entry(origin(url)).or_insert_with(Instant::now);
            let scheduled = (*slot).max(Instant::now());
            *slot = scheduled + self.delay;
            scheduled
        };
        tokio::time::sleep_until(tokio::time::Instant::from_std(scheduled)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn google_parser_respects_agents_allow_and_wildcards() {
        let policy = RobotsPolicy { text: "User-agent: *\nDisallow: /private*\nAllow: /private/public\nUser-agent: Other\nDisallow: /\n".into(), ..Default::default() };
        assert!(!policy.allowed("OpenSEOSpider", "https://e.org/private/a"));
        assert!(policy.allowed("OpenSEOSpider", "https://e.org/private/public"));
        assert!(!policy.allowed("Other", "https://e.org/"));
    }
}

#[cfg(test)]
#[path = "robots/sitemap_source_tests.rs"]
mod sitemap_source_tests;
