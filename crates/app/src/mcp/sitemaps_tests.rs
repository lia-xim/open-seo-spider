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
        ("page_limit", json!(null), "page_limit must"),
        ("page_limit", json!(-1), "page_limit must"),
        ("page_limit", json!(1.5), "page_limit must"),
        ("page_limit", json!(201), "page_limit must"),
        ("page_cursor", json!(null), "page_cursor must"),
        ("page_cursor", json!("bad!"), "invalid_sitemap_cursor"),
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

fn cursor_control(page: bool) -> String {
    // Canonical protocol-role controls, not positive stored-report evidence.
    use base64::Engine as _;
    use sha2::{Digest, Sha256};
    #[derive(serde::Serialize)]
    struct Payload {
        schema_version: u32,
        snapshot: crate::sitemap_membership::Snapshot,
        filter_sha256: String,
        after_sequence: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        after_page_ordinal: Option<u64>,
    }
    #[derive(serde::Serialize)]
    struct Token {
        payload: Payload,
        checksum_sha256: String,
    }
    let snapshot = crate::sitemap_membership::Snapshot {
        job_id: uuid::Uuid::nil(),
        crawl_generation: 1,
        crawl_config_sha256: "a".repeat(64),
        catalog_config_sha256: "b".repeat(64),
        catalog_revision: 1,
        catalog_rolling_sha256: "c".repeat(64),
        catalog_state_sha256: "d".repeat(64),
        inherited_basis: None,
    };
    let filter = crate::sitemap_membership::Filter::default();
    let payload = Payload {
        schema_version: if page { 2 } else { 1 },
        snapshot,
        filter_sha256: format!("{:x}", Sha256::digest(serde_json::to_vec(&filter).unwrap())),
        after_sequence: if page { None } else { Some(1) },
        after_page_ordinal: if page { Some(0) } else { None },
    };
    let checksum_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&payload).unwrap())
    );
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&Token {
            payload,
            checksum_sha256,
        })
        .unwrap(),
    )
}
#[tokio::test]
async fn sitemap_tool_validates_separate_cursor_roles_before_api_io() {
    let bridge = Bridge::new("http://127.0.0.1:1").unwrap();
    let raw = cursor_control(false);
    let page = cursor_control(true);
    for args in [
        json!({"id":uuid::Uuid::nil(),"cursor":page}),
        json!({"id":uuid::Uuid::nil(),"page_cursor":raw}),
        json!({"id":uuid::Uuid::nil(),"page_cursor":cursor_control(true),"page_limit":0}),
    ] {
        let response=bridge.dispatch(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"spider_sitemaps","arguments":args}})).await.unwrap();
        assert_eq!(response["result"]["isError"], true);
        let message = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            message.contains("invalid_sitemap_cursor") || message.contains("invalid_sitemap_query"),
            "{response}"
        );
        assert!(
            !message.contains("connect"),
            "Typed validation precedes I/O"
        );
    }
}
#[tokio::test]
async fn sitemap_tool_transports_both_cursor_roles_and_page_limit_in_actual_local_get() {
    use tokio::io::AsyncReadExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = vec![];
        let mut block = [0u8; 2048];
        while !bytes.windows(4).any(|v| v == b"\r\n\r\n") {
            let size = socket.read(&mut block).await.unwrap();
            assert!(size > 0);
            bytes.extend_from_slice(&block[..size]);
            assert!(bytes.len() < 65536);
        }
        let request = String::from_utf8(bytes).unwrap();
        let head = request.lines().next().unwrap();
        assert!(head.starts_with("GET /api/crawls/"));
        let route = head.split_whitespace().nth(1).unwrap().to_string();
        let body = serde_json::to_string(&json!({"received":route})).unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
        route
    });
    let raw = cursor_control(false);
    let page = cursor_control(true);
    let bridge = Bridge::new(&origin).unwrap();
    let response=tokio::time::timeout(std::time::Duration::from_secs(5),bridge.dispatch(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"spider_sitemaps","arguments":{"id":uuid::Uuid::nil(),"limit":1,"cursor":raw,"page_limit":2,"page_cursor":page}}}))).await.unwrap().unwrap();
    assert_eq!(response["result"]["isError"], false, "{response}");
    let route = task.await.unwrap();
    let url = Url::parse(&format!("http://own.invalid{route}")).unwrap();
    let pairs: std::collections::BTreeMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(pairs["limit"], "1");
    assert_eq!(pairs["page_limit"], "2");
    assert_eq!(pairs["cursor"], cursor_control(false));
    assert_eq!(pairs["page_cursor"], cursor_control(true));
    let tool = inventory()
        .into_iter()
        .find(|v| v["name"] == "spider_sitemaps")
        .unwrap();
    assert_eq!(
        tool["inputSchema"]["properties"]["page_limit"]["minimum"],
        0
    );
    assert_eq!(
        tool["inputSchema"]["properties"]["page_limit"]["maximum"],
        200
    );
    assert_eq!(inventory().len(), 25);
}
