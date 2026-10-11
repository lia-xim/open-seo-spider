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
                let mut data = Vec::new();
                // RFC 9309 requires parsing at least 500 KiB; keep a bounded 512 KiB.
                let mut failed = false;
                loop {
                    match response.chunk().await {
                        Ok(Some(chunk)) => {
                            let remaining = 512 * 1024 - data.len();
                            data.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
                            if data.len() == 512 * 1024 {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(error) => {
                            policy.warning = Some(format!("robots.txt body failed: {error}"));
                            failed = true;
                            break;
                        }
                    }
                }
                policy.deny_all = failed;
                policy.text = String::from_utf8_lossy(&data).into_owned();
                for line in policy.text.lines() {
                    if let Some((key, value)) = line.split_once(':')
                        && key.trim().eq_ignore_ascii_case("sitemap")
                        && let Some(sitemap) = crate::normalize_url(
                            value.split('#').next().unwrap_or("").trim(),
                            Some(url),
                        )
                    {
                        policy.sitemaps.push(sitemap.to_string());
                    }
                }
            }
            Ok(response)
                if response.status().is_client_error() && response.status().as_u16() != 429 => {}
            Ok(response) => {
                policy.deny_all = true;
                policy.warning = Some(format!(
                    "robots.txt returned {}; this origin was skipped",
                    response.status()
                ));
            }
            Err(error) => {
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
