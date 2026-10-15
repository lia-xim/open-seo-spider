//! Dedicated read-only findings bridge. The API envelope occurs once on the wire.
use super::{Bridge, error};
use crate::sitemap_findings::{MAX_JSON_BYTES, ReadQuery};
use anyhow::{Context, ensure};
use serde_json::{Value, json};
use std::{
    io::{self, Write},
    time::Duration,
};

mod stdio;
pub(super) use stdio::stdio_stream;
#[cfg(test)]
mod tests;

pub(super) const NAME: &str = "spider_sitemap_findings";
const MAX_RPC_ID_BYTES: usize = 1024;
const MAX_ERROR_BYTES: usize = 64 * 1024;
// App's owned read deadline is120s. Allow bounded outer transport margin;
// this does not promise hard SQL interruption or worker-join latency.
const FINDINGS_REQUEST_TIMEOUT: Duration = Duration::from_secs(150);
// Reserve the actual stdio newline as part of the eight-MiB wire ceiling.
const MAX_WIRE_JSON_BYTES: usize = MAX_JSON_BYTES - 1;
#[derive(Debug)]
struct ResponseBudget;
impl std::fmt::Display for ResponseBudget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Finding API response exceeds bounded MCP input")
    }
}
impl std::error::Error for ResponseBudget {}

pub(super) fn definition() -> Value {
    json!({"name":NAME,"description":"Read all seven local sitemap finding filters with bounded JSON paging, qualified Unknown evidence and optional pinned CSV export descriptor; no website requests or CSV body download.","inputSchema":{"type":"object","properties":{
        "id":{"type":"string","format":"uuid"},
        "filter":{"type":"string","enum":["urls-in-sitemap","urls-not-in-sitemap","orphan-urls","non-indexable-urls-in-sitemap","urls-in-multiple-sitemaps","xml-sitemap-with-over-50k-urls","xml-sitemap-over-50mb"]},
        "assessment":{"type":"string","enum":["match","not_matched","unknown","all"]},
        "requested_url":{"type":"string","minLength":1,"maxLength":65536},
        "source_task_url":{"type":"string","minLength":1,"maxLength":33554432},
        "search":{"type":"string","minLength":1,"maxLength":1024},
        "limit":{"type":"integer","minimum":1,"maximum":200},
        "candidate_limit":{"type":"integer","minimum":1,"maximum":5000},
        "cursor":{"type":"string","minLength":1,"maxLength":4096},
        "include_export_descriptor":{"type":"boolean"}
    },"required":["id"],"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false}})
}

pub(super) fn valid_rpc_id(id: &Value) -> bool {
    match id {
        Value::String(value) => value.len() <= MAX_RPC_ID_BYTES,
        Value::Number(number) => number.is_i64() || number.is_u64(),
        _ => false,
    }
}

fn arguments(mut args: Value) -> anyhow::Result<(uuid::Uuid, ReadQuery, bool)> {
    let object = args
        .as_object_mut()
        .context("Arguments must be an object")?;
    ensure!(
        object.keys().all(|key| matches!(
            key.as_str(),
            "id" | "filter"
                | "assessment"
                | "requested_url"
                | "source_task_url"
                | "search"
                | "limit"
                | "candidate_limit"
                | "cursor"
                | "include_export_descriptor"
        )),
        "Unexpected finding argument"
    );
    for key in [
        "id",
        "filter",
        "assessment",
        "requested_url",
        "source_task_url",
        "search",
        "cursor",
    ] {
        ensure!(
            object.get(key).is_none_or(Value::is_string),
            "{key} must be a string"
        );
    }
    for key in ["limit", "candidate_limit"] {
        ensure!(
            object.get(key).is_none_or(|v| v.as_u64().is_some()),
            "{key} must be a nonnegative integer"
        );
    }
    ensure!(
        object
            .get("include_export_descriptor")
            .is_none_or(Value::is_boolean),
        "include_export_descriptor must be boolean"
    );
    let id = object.remove("id").context("id required")?;
    let id = uuid::Uuid::parse_str(id.as_str().context("id must be a UUID string")?)?;
    let include = object
        .remove("include_export_descriptor")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let query: ReadQuery = serde_json::from_value(args)?;
    query.validate()?;
    Ok((id, query, include))
}

pub(super) struct ToolResult {
    value: Value,
    failed: bool,
}
impl ToolResult {
    fn failure(code: &str, message: &str, status: Option<u16>, detail: Option<Value>) -> Self {
        Self {
            value: json!({"code":code,"error":message,"status":status,"website_requests":0,"api_error":detail}),
            failed: true,
        }
    }
}

pub(super) async fn call(bridge: &Bridge, args: Value) -> ToolResult {
    let (id, query, include) = match arguments(args) {
        Ok(value) => value,
        Err(_) => {
            return ToolResult::failure(
                "invalid_sitemap_findings_query",
                "Invalid finding arguments; check types, domain, token and limits",
                Some(400),
                None,
            );
        }
    };
    let result = async {
        let url = bridge.base.join(&format!("api/crawls/{id}/sitemaps/findings"))?;
        let mut response = bridge.client.post(url).timeout(FINDINGS_REQUEST_TIMEOUT).json(&query).send().await?;
        let status = response.status();
        let limit = if status.is_success() { MAX_JSON_BYTES } else { MAX_ERROR_BYTES };
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if chunk.len() > limit.saturating_sub(bytes.len()) { return Err(ResponseBudget.into()); }
            bytes.extend_from_slice(&chunk);
        }
        // Parse off the Windows async stack; successful response wrapping and
        // recursive destruction run in complete_response's worker below.
        let mut value: Value = bridge.workers.run_blocking(move || serde_json::from_slice(&bytes)).await??;
        if !status.is_success() {
            return Ok(ToolResult::failure("sitemap_findings_api_error", "Local findings API rejected the request", Some(status.as_u16()), Some(value)));
        }
        ensure!(value.is_object(), "Finding API envelope must be an object");
        ensure!(value["engine"] == crate::sitemap_findings::ENGINE && value["website_requests"] == 0 && value["access_target"]["kind"] == "job" && value["access_target"]["job_id"].as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()) == Some(id), "Finding API envelope target/engine differs");
        ensure!(value.get("findings").is_some_and(|v| v.is_object() || v.is_null()) && value.get("export_token").is_some_and(|v| v.is_null() || v.as_str().is_some_and(|s| !s.is_empty() && s.len() <= 4096)), "Finding API envelope shape differs");
        let descriptor = if include {
            value.get("export_token").and_then(Value::as_str).map(|token| {
                let mut body=json!({"filter":query.filter,"assessment":query.assessment,"snapshot":token});
                let fields=body.as_object_mut().expect("object");
                // Move the owned selectors after POST; do not clone a large
                // literal source-task selector just to build a descriptor.
                for (key,selector) in [("requested_url",query.requested_url),("source_task_url",query.source_task_url),("search",query.search)] {
                    fields.insert(key.into(),selector.map(Value::String).unwrap_or(Value::Null));
                }
                let mut descriptor=json!({"method":"POST","path":format!("/api/crawls/{id}/sitemaps/findings/export.csv"),"media_type":"text/csv; charset=utf-8"});
                descriptor.as_object_mut().expect("object").insert("body".into(),body);
                descriptor
            })
        } else { None };
        if include {
            value.as_object_mut().expect("checked envelope").insert("export_descriptor".into(), descriptor.unwrap_or(Value::Null));
        }
        Ok::<_, anyhow::Error>(ToolResult { value, failed: false })
    }.await;
    result.unwrap_or_else(|error| {
        if error.downcast_ref::<ResponseBudget>().is_some() {
            ToolResult::failure(
                "sitemap_findings_response_byte_budget",
                "Local findings response exceeded the MCP input budget; reduce limit",
                Some(413),
                None,
            )
        } else if error
            .downcast_ref::<reqwest::Error>()
            .is_some_and(reqwest::Error::is_timeout)
        {
            ToolResult::failure(
                "sitemap_findings_timeout",
                "Local findings API request timed out",
                Some(504),
                None,
            )
        } else {
            ToolResult::failure(
                "sitemap_findings_bridge_failure",
                "Local findings request failed; no complete findings result is available",
                None,
                None,
            )
        }
    })
}

struct BoundedSize(usize);
impl Write for BoundedSize {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_WIRE_JSON_BYTES.saturating_sub(self.0) {
            return Err(io::Error::other("MCP finding wire response exceeds 8 MiB"));
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
pub(super) async fn complete_response(id: Value, result: ToolResult) -> Value {
    // The entire response (id, JSON-RPC wrapper, structured data and text) is capped.
    let fallback_id = if valid_rpc_id(&id) {
        id.clone()
    } else {
        Value::Null
    };
    tokio::task::spawn_blocking(move || {
        if !valid_rpc_id(&id) {
            return error(Value::Null, -32600, "Finding RPC id must be an integer or at most 1024 UTF-8 bytes");
        }
        let ToolResult {value,failed}=result;
        let mut inner=json!({"content":[{"type":"text","text":if failed { "Local sitemap findings request failed; inspect structuredContent." } else { "Local sitemap findings page; evidence and export descriptor are in structuredContent." }}],"isError":failed});
        inner.as_object_mut().expect("object").insert("structuredContent".into(),value);
        let mut response=json!({"jsonrpc":"2.0","id":id});
        response.as_object_mut().expect("object").insert("result".into(),inner);
        if serde_json::to_writer(&mut BoundedSize(0), &response).is_err() {
            return error(response["id"].clone(), -32603, "Finding response exceeds 8 MiB including MCP framing; reduce limit or selector size");
        }
        response
    }).await.unwrap_or_else(|_| error(fallback_id, -32603, "Finding response worker failed"))
}

pub(super) async fn complete_response_owned(
    owner: &open_seo_spider_core::TaskOwner,
    id: Value,
    result: ToolResult,
) -> Value {
    // The entire response (id, JSON-RPC wrapper, structured data and text) is capped.
    let fallback_id = if valid_rpc_id(&id) {
        id.clone()
    } else {
        Value::Null
    };
    owner.run_blocking(move || {
        if !valid_rpc_id(&id) {
            return error(Value::Null, -32600, "Finding RPC id must be an integer or at most 1024 UTF-8 bytes");
        }
        let ToolResult {value,failed}=result;
        let mut inner=json!({"content":[{"type":"text","text":if failed { "Local sitemap findings request failed; inspect structuredContent." } else { "Local sitemap findings page; evidence and export descriptor are in structuredContent." }}],"isError":failed});
        inner.as_object_mut().expect("object").insert("structuredContent".into(),value);
        let mut response=json!({"jsonrpc":"2.0","id":id});
        response.as_object_mut().expect("object").insert("result".into(),inner);
        if serde_json::to_writer(&mut BoundedSize(0), &response).is_err() {
            return error(response["id"].clone(), -32603, "Finding response exceeds 8 MiB including MCP framing; reduce limit or selector size");
        }
        response
    }).await.unwrap_or_else(|_| error(fallback_id, -32603, "Finding response worker failed"))
}
