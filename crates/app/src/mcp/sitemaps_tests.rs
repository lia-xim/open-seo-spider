use super::*;
#[tokio::test]
async fn sitemap_tool_is_readonly_and_wrong_argument_types_fail_before_api_io() {
    let bridge = Bridge::new("http://127.0.0.1:1").unwrap();
    let tools = inventory();
    let tool = tools
        .iter()
        .find(|tool| tool["name"] == "spider_sitemaps")
        .unwrap();
    assert_eq!(tool["annotations"]["readOnlyHint"], true);
    assert_eq!(tool["annotations"]["openWorldHint"], false);
    let id = uuid::Uuid::nil().to_string();
    for (key, value, needle) in [
        ("limit", json!(null), "limit must"),
        ("limit", json!(-1), "limit must"),
        ("limit", json!(201), "limit must"),
        ("kind", json!("unknown"), "Invalid sitemap kind"),
        ("membership_url", json!(null), "membership_url must"),
        ("cursor", json!(null), "cursor must"),
        ("cursor", json!("bad!"), "invalid_sitemap_cursor"),
    ] {
        let mut args = json!({"id":id});
        args[key] = value;
        let response=bridge.dispatch(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"spider_sitemaps","arguments":args}})).await.unwrap();
        assert_eq!(response["result"]["isError"], true);
        assert!(
            response["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains(needle),
            "{response}"
        );
    }
    let response=bridge.dispatch(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"spider_sitemaps","arguments":{"id":id,"unexpected":true}}})).await.unwrap();
    assert_eq!(response["error"]["code"], -32602);
}
