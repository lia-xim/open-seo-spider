//! Read-only strict summary adapter. No configuration mutation or invented read/CSV variants.
use super::Bridge;
use anyhow::{Result, ensure};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
pub(super) const NAME: &str = "spider_crawl_quotas";
pub(super) fn definition() -> Value {
    json!({"name":NAME,"description":"Read the durable native crawl quota summary only. Legacy accounting stays unavailable; counts are never reconstructed from saved pages. No website requests, exports or profile mutation. Increase existing caps only through spider_resume with generation/profile revision/hash authority.","inputSchema":{"type":"object","additionalProperties":false,"required":["id","action"],"properties":{"id":{"type":"string","format":"uuid"},"action":{"const":"summary"}}},"annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Summary,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    id: uuid::Uuid,
    action: Action,
}
pub(super) async fn call(bridge: &Bridge, value: Value) -> Result<Value> {
    let args: Args = serde_json::from_value(value)?;
    let Action::Summary = args.action;
    let response = bridge
        .client
        .get(
            bridge
                .base
                .join(&format!("api/crawls/{}/quotas", args.id))?,
        )
        .send()
        .await?;
    let status = response.status();
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        ensure!(
            chunk.len() <= (256usize * 1024).saturating_sub(bytes.len()),
            "Quota response exceeds256KiB"
        );
        bytes.extend_from_slice(&chunk);
    }
    ensure!(
        status.is_success(),
        "Quota summary refused HTTP{}",
        status.as_u16()
    );
    Ok(serde_json::from_slice(&bytes)?)
}
pub(super) fn config_schema() -> Value {
    json!({"type":["object","null"],"additionalProperties":false,"required":["version"],"description":"Opt-in durable native page/redirect quotas. Zero is a real cap; null/absence disables a scalar. Ordered URL regex rules all charge matching buckets. Rust regex syntax; patterns are at most4096 UTF8 bytes each and32768 combined, compiled regex size is bounded by Core. No renderer, resource_discovery or selected recrawl. Not vendor parity.","properties":{"version":{"const":1,"type":"integer"},"max_urls_per_depth":{"type":["integer","null"],"minimum":0,"maximum":1000000000},"max_urls_per_subdomain":{"type":["integer","null"],"minimum":0,"maximum":1000000000},"url_pattern_quotas":{"type":"array","maxItems":32,"items":{"type":"object","additionalProperties":false,"required":["id","pattern","max_urls"],"properties":{"id":{"type":"string","minLength":1,"maxLength":64,"pattern":"^[A-Za-z0-9_-]{1,64}$"},"pattern":{"type":"string","minLength":1,"maxLength":4096},"max_urls":{"type":"integer","minimum":0,"maximum":1000000000}}}}}})
}
pub(super) fn increases_schema() -> Value {
    json!({"type":["object","null"],"additionalProperties":false,"required":["expected_generation","expected_profile_revision","expected_profile_sha256"],"description":"Strict monotone increases of existing quota caps only; exact terminal generation and durable profile revision/hash are mandatory. Unknown rule IDs, decreases and profile shape changes are refused atomically.","properties":{"expected_generation":{"type":"integer","minimum":0},"expected_profile_revision":{"type":"integer","minimum":0},"expected_profile_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"},"max_urls_per_depth":{"type":["integer","null"],"minimum":0,"maximum":1000000000},"max_urls_per_subdomain":{"type":["integer","null"],"minimum":0,"maximum":1000000000},"url_pattern_maxima":{"type":"object","maxProperties":32,"propertyNames":{"pattern":"^[A-Za-z0-9_-]{1,64}$"},"additionalProperties":{"type":"integer","minimum":0,"maximum":1000000000}}}})
}
#[cfg(test)]
#[path = "crawl_quotas_tests.rs"]
mod tests;
