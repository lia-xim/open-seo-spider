//! Bounded local-application bridge. Does not start LanguageTool or fetch websites.
use super::Bridge;
use anyhow::{Result, ensure};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
pub(super) const NAME: &str = "spider_spelling";
pub(super) fn definition() -> Value {
    json!({"name":NAME,"description":"Explicitly check up to32 saved HTML text captures using an acknowledged configured literal-loopback LanguageTool server, or read a persisted run. Default config is off. Analysis persists a new local run and may POST saved text to the configured local service; no website fetch, recrawl, automatic cloud, PDF or title/meta coverage. First read the separate application dictionary revision and exact saved content inventory. Failure/incomplete counts are unknown, not zero. Request128KiB/response8MiB. Dictionary/profile edits use the application's explicit UI/API.",
        "inputSchema":{"type":"object","additionalProperties":false,"required":["id","action"],"properties":{
            "id":{"type":"string","format":"uuid"},"action":{"enum":["dictionary","analyze","result"]},
            "run_id":{"type":["string","null"],"format":"uuid"},"request":{"type":["object","null"]}}},
        "annotations":{"readOnlyHint":false,"destructiveHint":false,"idempotentHint":false,"openWorldHint":false}})
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Dictionary,
    Analyze,
    Result,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    id: uuid::Uuid,
    action: Action,
    #[serde(default)]
    run_id: Option<uuid::Uuid>,
    #[serde(default)]
    request: Option<crate::spelling_app::Request>,
}
fn decode(raw: &[u8], args: &Args) -> Result<Value> {
    let value: Value = serde_json::from_slice(raw)?;
    if matches!(args.action, Action::Dictionary) {
        let dictionary: open_seo_spider_core::spelling::Dictionary =
            serde_json::from_value(value["dictionary"].clone())?;
        dictionary.validate().map_err(anyhow::Error::msg)?;
        ensure!(
            value["sha256"].as_str()
                == Some(
                    &open_seo_spider_core::spelling::hash(&dictionary)
                        .map_err(anyhow::Error::msg)?
                ),
            "Dictionary digest mismatch"
        );
        return Ok(value);
    }
    ensure!(
        value["crawl_id"].as_str() == Some(&args.id.to_string()),
        "Spelling reply crawl mismatch"
    );
    let report: crate::spelling_app::Report = serde_json::from_value(value["report"].clone())?;
    ensure!(
        report.crawl_id == Some(args.id),
        "Spelling report persisted crawl binding differs"
    );
    crate::spelling_app::validate_report(&report, args.request.as_ref())?;
    ensure!(
        report.engine == open_seo_spider_core::spelling::ENGINE
            && report.website_requests == 0
            && report.offset_unit == "utf16_code_units",
        "Unexpected spelling response contract"
    );
    ensure!(
        report.documents.len() <= 32 && report.engine_requests <= 32,
        "Spelling response bounds"
    );
    ensure!(
        report.config_sha256
            == open_seo_spider_core::spelling::hash(&report.request.config)
                .map_err(anyhow::Error::msg)?
            && report.dictionary_sha256
                == open_seo_spider_core::spelling::hash(&report.dictionary)
                    .map_err(anyhow::Error::msg)?,
        "Spelling config/dictionary binding mismatch"
    );
    if let Some(request) = &args.request {
        ensure!(
            serde_json::to_value(request)? == serde_json::to_value(&report.request)?,
            "Spelling response differs from exact request"
        );
        let s = &request.selection;
        let saved = &report.saved_capture["snapshot"];
        ensure!(
            s.expected_generation
                .is_none_or(|g| saved["generation"] == g),
            "Spelling response generation mismatch"
        );
        ensure!(
            s.expected_snapshot_sha256
                .as_ref()
                .is_none_or(|h| saved["sha256"] == *h),
            "Spelling response snapshot mismatch"
        );
        ensure!(
            s.expected_reporting_identity
                .as_ref()
                .is_none_or(|h| report.saved_capture["reporting"]["identity_sha256"] == *h),
            "Spelling reporting identity mismatch"
        );
    }
    if let Some(run) = args.run_id {
        ensure!(report.run_id == run, "Spelling response run mismatch");
    }
    ensure!(
        report.complete || report.accepted_count.is_none(),
        "Incomplete spelling reply claims aggregate count"
    );
    Ok(value)
}
pub(super) fn valid_id(id: &Value) -> bool {
    id.as_str()
        .is_some_and(|v| v.len() <= 1024 && !v.contains('\0'))
        || (id.is_number() && id.to_string().len() <= 128)
}
pub(super) fn complete_response(id: Value, result: Result<Value>) -> Value {
    let (body, failed) = match result {
        Ok(value) => (value, false),
        Err(error) => (
            json!({"error":error.to_string().chars().take(4096).collect::<String>()}),
            true,
        ),
    };
    let envelope = json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":if failed {"Saved spelling operation failed; see structuredContent."}else{"Saved spelling operation completed; inspect status/counts and provenance in structuredContent."}}],"structuredContent":body,"isError":failed}});
    let mut counter = super::FileWireCounter(0);
    if serde_json::to_writer(&mut counter, &envelope).is_ok() {
        envelope
    } else {
        json!({"jsonrpc":"2.0","id":envelope["id"],"result":{"content":[{"type":"text","text":"Spelling MCP wire exceeds8MiB; persisted run may exist. Read via API/CLI with a smaller cohort."}],"isError":true}})
    }
}
pub(super) async fn call(bridge: &Bridge, value: Value) -> Result<Value> {
    ensure!(
        serde_json::to_vec(&value)?.len() <= crate::spelling_app::REQUEST_BYTES,
        "Spelling request exceeds128KiB"
    );
    let args: Args = serde_json::from_value(value)?;
    match args.action {
        Action::Analyze => {
            ensure!(args.run_id.is_none(), "Analyze forbids run_id");
            args.request
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("request required"))?
                .validate()?;
        }
        Action::Dictionary => {
            ensure!(
                args.request.is_none() && args.run_id.is_none(),
                "Dictionary forbids request/run"
            );
        }
        Action::Result => {
            ensure!(
                args.request.is_none() && args.run_id.is_some(),
                "Result requires run_id only"
            );
        }
    }
    let route = match args.action {
        Action::Dictionary => "api/spelling/dictionary".into(),
        Action::Analyze => format!("api/crawls/{}/spelling/analyze", args.id),
        Action::Result => format!(
            "api/crawls/{}/spelling/runs/{}",
            args.id,
            args.run_id.ok_or_else(|| anyhow::anyhow!("Missing run"))?
        ),
    };
    let url = bridge.base.join(&route)?;
    let response = if matches!(args.action, Action::Analyze) {
        bridge
            .client
            .post(url)
            .timeout(std::time::Duration::from_secs(100))
            .json(&args.request)
            .send()
            .await?
    } else {
        bridge
            .client
            .get(url)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await?
    };
    let status = response.status();
    let media = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    ensure!(
        media
            .split(';')
            .next()
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json")),
        "Spelling JSON media type required"
    );
    let length = response.content_length();
    ensure!(
        length.is_none_or(|v| v <= crate::spelling_app::STORE_BYTES as u64),
        "Spelling response exceeds8MiB"
    );
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        ensure!(
            bytes.len().saturating_add(chunk.len()) <= crate::spelling_app::STORE_BYTES,
            "Spelling response exceeds8MiB"
        );
        bytes.extend_from_slice(&chunk);
    }
    ensure!(
        length.is_none_or(|v| v == bytes.len() as u64),
        "Spelling response length mismatch"
    );
    ensure!(
        status.is_success(),
        "Spelling HTTP{}: {}",
        status.as_u16(),
        String::from_utf8_lossy(&bytes)
    );
    decode(&bytes, &args)
}
#[cfg(test)]
mod tests;
