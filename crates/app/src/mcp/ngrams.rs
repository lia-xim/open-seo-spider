//! Read-only saved N-gram frontdoor over the existing shared HTTP projection.
use super::Bridge;
use anyhow::{Result, ensure};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
pub(super) const NAME: &str = "spider_ngrams";
const BODY_BYTES: usize = 128 * 1024;
pub(super) fn definition() -> Value {
    json!({"name":NAME,"description":"Read saved N-gram totals, occurrences or source observations through the same local HTTP service; no website requests. request.reporting_mode accepts raw/reported; absent/null applies an active saved directive reporting profile, otherwise legacy raw. HTML/retained metadata channel fields and dense source ordinals retain their existing meaning; raw_page_ordinal separately identifies the original report observation. Multiple eligible duplicate URLs are refused. Preview is paged JSON; full filtered CSV requires its preview token and no continuation. Responses above128KiB require smaller pages or offline CLI CSV.","inputSchema":{"type":"object","additionalProperties":false,"required":["id","request"],"properties":{"id":{"type":"string","format":"uuid"},"action":{"enum":["preview","csv"]},"request":{"type":"object"}}},"annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    #[default]
    Preview,
    Csv,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    id: uuid::Uuid,
    #[serde(default)]
    action: Action,
    request: crate::ngram_app::Request,
}
pub(super) async fn call(bridge: &Bridge, args: Value) -> Result<Value> {
    let a: Args = serde_json::from_value(args)?;
    a.request.validate()?;
    let options = serde_json::to_vec(&a.request)?;
    ensure!(
        options.len() <= crate::ngram_app::REQUEST_BYTES,
        "Ngram request exceeds64KiB"
    );
    if matches!(a.action, Action::Csv) {
        ensure!(
            a.request.expected_snapshot_token.is_some() && a.request.after.is_none(),
            "CSV requires preview token and no continuation"
        );
    }
    let suffix = if matches!(a.action, Action::Csv) {
        "csv"
    } else {
        "preview"
    };
    let response = bridge
        .client
        .post(
            bridge
                .base
                .join(&format!("api/crawls/{}/ngrams/{suffix}", a.id))?,
        )
        .header("content-type", "application/json")
        .body(options)
        .send()
        .await?;
    let status = response.status();
    let declared = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let expected = response
        .headers()
        .get("x-content-sha256")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let snapshot = response
        .headers()
        .get("x-ngram-snapshot")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        ensure!(
            chunk.len() <= BODY_BYTES.saturating_sub(bytes.len()),
            "MCP Ngram body exceeds128KiB; use smaller pages or CLI CSV"
        );
        bytes.extend_from_slice(&chunk);
    }
    ensure!(
        status.is_success(),
        "Ngram service refused HTTP{}",
        status.as_u16()
    );
    let declared = declared.ok_or_else(|| anyhow::anyhow!("Ngram response length missing"))?;
    ensure!(
        !declared.is_empty()
            && declared.bytes().all(|b| b.is_ascii_digit())
            && (declared == "0" || !declared.starts_with('0')),
        "Invalid Ngram response length"
    );
    let length: usize = declared.parse()?;
    ensure!(
        length <= BODY_BYTES && bytes.len() == length,
        "Ngram response length differs from complete EOF body"
    );
    let digest = crate::ngram_app::sha(&bytes);
    ensure!(
        expected.as_deref() == Some(digest.as_str()),
        "Ngram response digest missing/mismatched"
    );
    if matches!(a.action, Action::Csv) {
        ensure!(
            content_type.starts_with("text/csv"),
            "Wrong Ngram CSV response type"
        );
        ensure!(
            snapshot.as_ref() == a.request.expected_snapshot_token.as_ref(),
            "Ngram CSV belongs to another snapshot"
        );
        use base64::Engine as _;
        Ok(
            json!({"content_type":"text/csv","snapshot_token":snapshot,"sha256":digest,"bytes":bytes.len(),"base64":base64::engine::general_purpose::STANDARD.encode(&bytes)}),
        )
    } else {
        ensure!(
            content_type.starts_with("application/json"),
            "Wrong Ngram JSON response type"
        );
        let value: Value = serde_json::from_slice(&bytes)?;
        ensure!(
            value["website_requests"] == 0 && value["whole_website_coverage_attested"] == false,
            "Unexpected Ngram service scope"
        );
        if let Some(token) = &a.request.expected_snapshot_token {
            ensure!(
                value["snapshot"]["snapshot_token"].as_str() == Some(token.as_str()),
                "Ngram preview belongs to another snapshot"
            );
        }
        Ok(value)
    }
}
#[cfg(test)]
#[path = "ngrams_tests.rs"]
mod tests;
