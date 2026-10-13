//! Additional read-only MCP adapter; all pre-existing tool routing stays unchanged.
use super::*;
use crate::comparison_app::{
    MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, SavedComparisonFailure, SavedComparisonRequest,
    SavedComparisonResponse, failure, request_schema,
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
    json!({"content":[{"type":"text","text":value.to_string()}],
        "structuredContent":value,"isError":is_error})
}
fn error_result(error: SavedComparisonFailure) -> Value {
    tool_result(json!(error), true)
}
fn decode(status: reqwest::StatusCode, bytes: &[u8]) -> Value {
    if status.is_success() {
        match serde_json::from_slice::<SavedComparisonResponse>(bytes) {
            Ok(value) if value.website_requests == 0 && value.comparison.website_requests == 0 => {
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
        match serde_json::from_slice::<SavedComparisonFailure>(bytes) {
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
pub(super) async fn call(bridge: &Bridge, arguments: Value) -> Value {
    let request = match serde_json::from_value::<SavedComparisonRequest>(arguments) {
        Ok(value) => value,
        Err(error) => {
            return error_result(failure("invalid_comparison_request", error.to_string()));
        }
    };
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
            Ok(None) => return decode(status, &bytes),
            Err(error) => {
                return error_result(failure("comparison_loopback_transport", error.to_string()));
            }
        }
    }
}

#[cfg(test)]
#[path = "saved_comparison_tests.rs"]
mod tests;
