//! Authored read-only MCP controls; these tests do not contact a website.
use super::*;
#[test]
fn descriptor_is_read_only_strict_and_additional_to_existing_tools() {
    let definition = definition();
    assert_eq!(definition["name"], "spider_compare_saved");
    assert_eq!(definition["annotations"]["readOnlyHint"], true);
    assert_eq!(definition["annotations"]["openWorldHint"], false);
    assert_eq!(definition["inputSchema"]["additionalProperties"], false);
    let names: Vec<_> = inventory()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(names.len(), 23);
    assert!(names.contains(&"spider_compare".into()));
    assert_eq!(names.last().unwrap(), "spider_compare_saved");
}
#[test]
fn structured_api_collision_failure_is_retained_in_mcp_result() {
    let bytes = serde_json::to_vec(&json!({"website_requests":0,"error":{
        "code":"ambiguous_snapshot_identity","message":"collision","collisions":[{
            "side":"before","comparison_key":"https://fixture.test/","occurrences":[
                {"ordinal":0,"original_requested_url":"https://fixture.test/"},
                {"ordinal":1,"original_requested_url":"https://fixture.test/"}
            ]
        }]
    }}))
    .unwrap();
    let result = decode(reqwest::StatusCode::BAD_REQUEST, &bytes);
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["structuredContent"]["error"]["collisions"][0]["occurrences"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(result["structuredContent"]["website_requests"], 0);
}
#[test]
fn nonstructured_api_error_is_explicit_adapter_failure() {
    let result = decode(reqwest::StatusCode::CONFLICT, br#"{"error":"legacy text"}"#);
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["structuredContent"]["error"]["code"],
        "invalid_comparison_api_error"
    );
}
