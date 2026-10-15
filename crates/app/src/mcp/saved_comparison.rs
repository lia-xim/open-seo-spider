//! Additional read-only MCP adapter; all pre-existing tool routing stays unchanged.
use super::*;
use crate::comparison_app::{
    MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, SavedComparisonFailure, SavedComparisonRequest,
    SavedComparisonResponse, check_serialized_budget, failure, request_schema,
};

pub(super) fn definition() -> Value {
    json!({
        "name":"spider_compare_saved",
        "description":"Compare two stable saved report cohorts with exact identities, explicit mappings and actual old/new facts; no website is fetched",
        "inputSchema":request_schema(),
        "annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false}
    })
}
fn tool_result(value: Value, is_error: bool) -> Value {
    // Value may reorder object keys. Report hashes/counts refer to compact
    // typed Rust reserialization, never to a slice of this MCP text/Value.
    json!({"content":[{"type":"text","text":value.to_string()}],
        "structuredContent":value,"isError":is_error})
}
fn error_result(error: SavedComparisonFailure) -> Value {
    tool_result(json!(error), true)
}
// Focused insertion into crates/app/src/mcp/saved_comparison.rs.
// Requires App serde_json features float_roundtrip + unbounded_depth.
// In decode(), replace both serde_json::from_slice::<...>(bytes) calls with
// parse_saved_api_json::<...>(bytes, include_original_reports).
// Default false uses the old serde_json::from_slice path unchanged.
// Authored, NOT COMPILED OR RUN by this author.
const MAX_BUNDLE_JSON_DEPTH: usize = 256;

fn preflight_bundle_json_depth(bytes: &[u8]) -> Result<(), String> {
    let mut containers = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    for &byte in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'{' | b'[' => {
                if containers.len() >= MAX_BUNDLE_JSON_DEPTH {
                    return Err("Original-report JSON nesting exceeds 256 containers".into());
                }
                containers.push(byte);
            }
            b'}' | b']' => {
                let expected = if byte == b'}' { b'{' } else { b'[' };
                if containers.pop() != Some(expected) {
                    return Err("Original-report JSON has unmatched container delimiters".into());
                }
            }
            _ => {}
        }
    }
    if quoted || escaped {
        return Err("Original-report JSON has an unclosed quoted string".into());
    }
    if !containers.is_empty() {
        return Err("Original-report JSON has unclosed containers".into());
    }
    Ok(())
}

fn parse_saved_api_json<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    include_original_reports: bool,
) -> Result<T, serde_json::Error> {
    if !include_original_reports {
        return serde_json::from_slice(bytes);
    }
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Original-report JSON exceeds 64 MiB",
        )));
    }
    preflight_bundle_json_depth(bytes).map_err(|message| {
        serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        ))
    })?;
    // The lexical preflight bounded every object/array outside quoted strings.
    // Only this already size/depth-bounded opt-in path disables the default
    // limit so that wrapper containers cannot reject valid saved report facts.
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    deserializer.disable_recursion_limit();
    let value = T::deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value)
}

fn decode(status: reqwest::StatusCode, bytes: &[u8], include_original_reports: bool) -> Value {
    if status.is_success() {
        match parse_saved_api_json::<SavedComparisonResponse>(bytes, include_original_reports) {
            Ok(value) if value.website_requests == 0 && value.comparison.website_requests == 0 => {
                if value.original_reports.is_some() != include_original_reports {
                    return error_result(failure(
                        "invalid_comparison_api_response",
                        "Original-report bundle presence differs from the explicit request",
                    ));
                }
                if let Err(error) = value.validate_original_reports() {
                    return error_result(error);
                }
                tool_result(json!(value), false)
            }
            Ok(_) => error_result(failure(
                "invalid_comparison_api_response",
                "Saved comparison unexpectedly declared website requests",
            )),
            Err(error) => error_result(failure(
                "invalid_comparison_api_response",
                error.to_string(),
            )),
        }
    } else {
        match parse_saved_api_json::<SavedComparisonFailure>(bytes, include_original_reports) {
            Ok(value) if value.website_requests == 0 => error_result(value),
            _ => error_result(failure(
                "invalid_comparison_api_error",
                format!(
                    "API {status} did not return the structured saved-comparison error envelope"
                ),
            )),
        }
    }
}
pub(super) fn complete_response(id: Value, result: Value, include_original_reports: bool) -> Value {
    complete_response_with_limit(id, result, include_original_reports, MAX_RESPONSE_BYTES)
}
#[cfg(test)]
pub(super) async fn complete_response_on_worker(
    id: Value,
    result: Value,
    include_original_reports: bool,
) -> Value {
    if !include_original_reports {
        return complete_response(id, result, false);
    }
    match tokio::task::spawn_blocking(move || complete_response(id, result, true)).await {
        Ok(response) => response,
        Err(error) => json!({"jsonrpc":"2.0","id":null,"error":{
            "code":-32603,"message":format!("comparison_response_task_failed: {error}")}}),
    }
}

pub(super) async fn complete_response_on_worker_owned(
    owner: &open_seo_spider_core::TaskOwner,
    id: Value,
    result: Value,
    include_original_reports: bool,
) -> Value {
    if !include_original_reports {
        return complete_response(id, result, false);
    }
    match owner
        .run_blocking(move || complete_response(id, result, true))
        .await
    {
        Ok(response) => response,
        Err(error) => json!({"jsonrpc":"2.0","id":null,"error":{
            "code":-32603,"message":format!("comparison_response_task_failed: {error}")}}),
    }
}
fn complete_response_with_limit(
    id: Value,
    result: Value,
    include_original_reports: bool,
    maximum: usize,
) -> Value {
    let response = json!({"jsonrpc":"2.0","id":id,"result":result});
    if !include_original_reports
        || check_serialized_budget(
            &response,
            maximum.min(MAX_RESPONSE_BYTES),
            "original_report_bundle_byte_budget",
        )
        .is_ok()
    {
        return response;
    }
    // Includes the actual request ID, both representations and JSON escaping.
    // Do not substitute a truncated/first-N success for the requested bundle.
    let rejected = json!({"jsonrpc":"2.0","id":response["id"],"result":error_result(failure(
        "original_report_bundle_byte_budget",
        "Complete original-report JSON-RPC response exceeds its output byte budget"))});
    if check_serialized_budget(
        &rejected,
        maximum.min(MAX_RESPONSE_BYTES),
        "original_report_bundle_byte_budget",
    )
    .is_ok()
    {
        rejected
    } else {
        // An unrepresentable request ID cannot be echoed within this limit.
        // The small protocol failure deliberately uses null and never reports success.
        json!({"jsonrpc":"2.0","id":null,"error":{"code":-32600,
            "message":"original_report_bundle_byte_budget: request ID cannot fit bounded response"}})
    }
}
pub(super) async fn call(bridge: &Bridge, arguments: Value) -> Value {
    let request = match serde_json::from_value::<SavedComparisonRequest>(arguments) {
        Ok(value) => value,
        Err(error) => {
            return error_result(failure("invalid_comparison_request", error.to_string()));
        }
    };
    let include_original_reports = request.include_original_reports;
    let body = match serde_json::to_vec(&request) {
        Ok(bytes) if bytes.len() <= MAX_REQUEST_BYTES => bytes,
        Ok(_) => {
            return error_result(failure(
                "comparison_request_byte_budget",
                "Saved comparison request exceeds 1 MiB",
            ));
        }
        Err(error) => {
            return error_result(failure("invalid_comparison_request", error.to_string()));
        }
    };
    let url = match bridge.base.join("api/compare/saved") {
        Ok(value) => value,
        Err(error) => return error_result(failure("comparison_loopback_route", error.to_string())),
    };
    // Bridge.new already requires HTTP loopback, disables proxies and redirects.
    // This single POST is to the local App; no requested website is fetched.
    let mut response = match bridge
        .client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
    {
        Ok(value) => value,
        Err(error) => {
            return error_result(failure("comparison_loopback_transport", error.to_string()));
        }
    };
    let status = response.status();
    let mut bytes = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if bytes
                    .len()
                    .checked_add(chunk.len())
                    .is_none_or(|size| size > MAX_RESPONSE_BYTES)
                {
                    return error_result(failure(
                        "comparison_response_byte_budget",
                        "Saved comparison API response exceeds 64 MiB",
                    ));
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) if include_original_reports => {
                // Typed reports can be valid at the standalone depth limit yet
                // require more stack under response wrappers. Keep recursive
                // parsing, hash validation and Value construction off the small
                // Windows stdio main stack and off asynchronous HTTP workers.
                return match bridge
                    .workers
                    .run_blocking(move || decode(status, &bytes, true))
                    .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        error_result(failure("comparison_decode_task_failed", error.to_string()))
                    }
                };
            }
            Ok(None) => return decode(status, &bytes, false),
            Err(error) => {
                return error_result(failure("comparison_loopback_transport", error.to_string()));
            }
        }
    }
}

#[cfg(test)]
#[path = "saved_comparison_tests.rs"]
mod tests;
