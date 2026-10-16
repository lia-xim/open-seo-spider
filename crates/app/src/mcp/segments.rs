//! Read-only loopback consumer of the exact same saved-report service.
use super::Bridge;
use anyhow::{Result, ensure};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
pub(super) const NAME: &str = "spider_segments";
pub(super) fn definition() -> Value {
    json!({"name":NAME,"description":"Apply ordered, overlapping field/regex segments to one stable saved crawl. Read pages/issues/counts/charts or export full JSON/CSV. Missing supported predicate facts stay unknown. Predicates support requested_url, final_url, status, media_type and stored_title only. Displayed depth may include legacy/import default0 and cannot be a predicate. No website requests or crawl permissions. CSV/JSON export requires the preview snapshot token.","inputSchema":{"type":"object","additionalProperties":false,"required":["id","request"],"properties":{"id":{"type":"string","format":"uuid"},"action":{"enum":["preview","json","csv"]},"request":{"type":"object"}}},"annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    #[default]
    Preview,
    Json,
    Csv,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    id: uuid::Uuid,
    #[serde(default)]
    action: Action,
    request: crate::segments_app::Request,
}
pub(super) async fn call(bridge: &Bridge, args: Value) -> Result<Value> {
    let a: Args = serde_json::from_value(args)?;
    a.request.validate()?;
    let suffix = match a.action {
        Action::Preview => "segments",
        Action::Json => "segments.json",
        Action::Csv => "segments.csv",
    };
    if !matches!(a.action, Action::Preview) {
        ensure!(
            a.request.expected_snapshot_token.is_some() && a.request.cursor.is_none(),
            "Export requires preview token and no page cursor"
        );
    }
    let response = bridge
        .client
        .post(bridge.base.join(&format!("api/crawls/{}/{suffix}", a.id))?)
        .json(&a.request)
        .send()
        .await?;
    let status = response.status();
    let expected = response
        .headers()
        .get("x-content-sha256")
        .and_then(|s| s.to_str().ok())
        .map(str::to_owned);
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let c = chunk?;
        ensure!(
            c.len() <= (1024usize * 1024).saturating_sub(bytes.len()),
            "MCP segment result exceeds1MiB; use smaller pages or CLI full export"
        );
        bytes.extend_from_slice(&c);
    }
    ensure!(
        status.is_success(),
        "Segment service refused HTTP{}",
        status.as_u16()
    );
    let digest = crate::segments_app::sha(&bytes);
    ensure!(
        expected.as_deref() == Some(digest.as_str()),
        "Segment response digest missing/mismatched"
    );
    if matches!(a.action, Action::Csv) {
        use base64::Engine as _;
        Ok(
            json!({"content_type":"text/csv","sha256":digest,"bytes":bytes.len(),"base64":base64::engine::general_purpose::STANDARD.encode(&bytes)}),
        )
    } else {
        Ok(serde_json::from_slice(&bytes)?)
    }
}
#[cfg(test)]
#[path = "segments_tests.rs"]
mod tests;
