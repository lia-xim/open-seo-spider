use crate::robots_sitemap_source::{ROBOTS_BODY_CAP, RobotsBodyBuffer, RobotsSitemapSource};
use crate::urls::origin;
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
    policies: Mutex<HashMap<String, Arc<RobotsPolicy>>>,
}

impl RobotsCache {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            policies: Mutex::new(HashMap::new()),
        }
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
        match self.client.get(&robots_url).send().await {
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
