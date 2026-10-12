//! Original MCP bridge; crawl state and execution belong to the local API.
use anyhow::{Context, bail};
use reqwest::{Client, Url};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
pub const PROTOCOLS: &[&str] = &["2025-06-18", "2025-11-25"];
#[derive(Clone)]
pub struct Bridge {
    client: Client,
    base: Url,
}
impl Bridge {
    pub fn new(base: &str) -> anyhow::Result<Self> {
        let mut base = Url::parse(base)?;
        if base.scheme() != "http"
            || !matches!(base.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
            || !base.username().is_empty()
            || base.password().is_some()
        {
            bail!("MCP requires an HTTP loopback API");
        }
        base.set_path("/");
        base.set_query(None);
        base.set_fragment(None);
        Ok(Self {
            base,
            client: Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(60))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        })
    }
    pub async fn dispatch(&self, request: Value) -> Option<Value> {
        if !request.is_object() {
            return Some(error(
                Value::Null,
                -32600,
                "A single JSON-RPC object is required",
            ));
        }
        let id = request.get("id")?.clone();
        if request["jsonrpc"].as_str() != Some("2.0") {
            return Some(error(id, -32600, "Invalid JSON-RPC request"));
        }
        let result = match request["method"].as_str().unwrap_or("") {
            "initialize" => {
                let requested = request["params"]["protocolVersion"].as_str().unwrap_or("");
                let version = if PROTOCOLS.contains(&requested) {
                    requested
                } else {
                    "2025-11-25"
                };
                json!({"protocolVersion":version,"serverInfo":{"name":"open-seo-spider","version":env!("CARGO_PKG_VERSION")},"capabilities":{"tools":{"listChanged":false}},"instructions":"Use the local crawler only for user-requested targets. A completed crawl does not imply full feature parity."})
            }
            "ping" => json!({}),
            "tools/list" => json!({"tools":inventory()}),
            "tools/call" => {
                let Some(name) = request["params"]["name"].as_str() else {
                    return Some(error(id, -32602, "Tool name required"));
                };
                let args = request["params"]
                    .get("arguments")
                    .cloned()
                    .unwrap_or(json!({}));
                if !args.is_object()
                    || !inventory()
                        .iter()
                        .any(|tool| tool["name"].as_str() == Some(name))
                {
                    return Some(error(id, -32602, "Invalid tool or arguments"));
                }
                match self.call(name, args).await {
                    Ok(value) => {
                        json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":value,"isError":false})
                    }
                    Err(failure) => {
                        json!({"content":[{"type":"text","text":failure.to_string()}],"isError":true})
                    }
                }
            }
            _ => return Some(error(id, -32601, "Method not found")),
        };
        Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }
    async fn call(&self, name: &str, args: Value) -> anyhow::Result<Value> {
        let mut body = None;
        let mut route = match name {
            "spider_capabilities" => "api/capabilities".to_owned(),
            "spider_list_crawls" => "api/crawls".to_owned(),
            "spider_analyze_content" => {
                body = Some(args.get("input").cloned().context("input required")?);
                "api/content/analyze".to_owned()
            }
            "spider_preview_policy" => {
                body = Some(args.get("input").cloned().context("input required")?);
                "api/policy/preview".to_owned()
            }
            "spider_start_crawl" => {
                body = Some(args.get("config").cloned().context("config required")?);
                "api/crawls".to_owned()
            }
            "spider_compare" => format!(
                "api/compare?before={}&after={}",
                id(&args, "before")?,
                id(&args, "after")?
            ),
            _ => {
                let suffix = match name {
                    "spider_status" => "",
                    "spider_pages" => "/pages",
                    "spider_issues" => "/issues",
                    "spider_analysis" => "/analysis",
                    "spider_policy" => "/policy",
                    "spider_cancel" => {
                        body = Some(json!({}));
                        "/cancel"
                    }
                    "spider_pause" => {
                        body = Some(json!({}));
                        "/pause"
                    }
                    "spider_resume" => {
                        body = Some(args.get("options").cloned().unwrap_or_else(|| json!({})));
                        "/resume"
                    }
                    "spider_recrawl" => {
                        let mut request =
                            json!({"urls":args.get("urls").context("urls required")?});
                        if let Some(value) = args.get("max_attempts") {
                            request["max_attempts"] = value.clone();
                        }
                        body = Some(request);
                        "/recrawl"
                    }
                    "spider_project" => "/project.json",
                    _ => bail!("Unknown tool"),
                };
                format!("api/crawls/{}{suffix}", id(&args, "id")?)
            }
        };
        if name == "spider_pages" {
            route.push_str(&format!(
                "?offset={}&limit={}",
                args["offset"].as_u64().unwrap_or(0),
                args["limit"].as_u64().unwrap_or(200).clamp(1, 10000)
            ));
        }
        if name == "spider_policy" {
            let limit = args["limit"].as_u64().unwrap_or(200);
            anyhow::ensure!((1..=1000).contains(&limit), "limit must be 1..1000");
            route.push_str(&format!("?limit={limit}"));
            if let Some(value) = args.get("after_sequence") {
                let sequence = value
                    .as_u64()
                    .context("after_sequence must be a nonnegative integer")?;
                route.push_str(&format!("&after_sequence={sequence}"));
            }
        }
        let url = self.base.join(&route)?;
        let mut response = if let Some(body) = body {
            self.client.post(url).json(&body).send().await?
        } else {
            self.client.get(url).send().await?
        };
        let status = response.status();
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if bytes.len() + chunk.len() > 64 * 1024 * 1024 {
                bail!("MCP response budget exceeded; use spider_pages with offset and limit");
            }
            bytes.extend_from_slice(&chunk);
        }
        if !status.is_success() {
            bail!("API {status}: {}", String::from_utf8_lossy(&bytes));
        }
        Ok(serde_json::from_slice(&bytes)?)
    }
}
fn id(args: &Value, key: &str) -> anyhow::Result<uuid::Uuid> {
    Ok(uuid::Uuid::parse_str(
        args[key]
            .as_str()
            .with_context(|| format!("{key} required"))?,
    )?)
}
fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
fn inventory() -> Vec<Value> {
    let uuid = json!({"type":"string","format":"uuid"});
    vec![
        ("spider_capabilities","Read implemented and optional capabilities",json!({}),vec![],true),
        ("spider_list_crawls","List saved and active crawls",json!({}),vec![],true),
        ("spider_start_crawl","Start a crawl for user-requested targets",json!({"config":{"type":"object","description":"CrawlConfig: seed_urls or seed_url, sitemap_urls, concurrency, delay_ms, obey_robots, max_pages, optional renderer"}}),vec!["config"],false),
        ("spider_status","Read crawl progress and warnings",json!({"id":uuid}),vec!["id"],true),
        ("spider_pages","Read a bounded page slice; increment offset for more",json!({"id":uuid,"offset":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":10000}}),vec!["id"],true),
        ("spider_issues","Read observed audit issues",json!({"id":uuid}),vec!["id"],true),
        ("spider_analysis","Read graph and relationship/security/JSON-LD observations",json!({"id":uuid}),vec!["id"],true),
        ("spider_policy","Read bounded native admission evidence; advance the returned sequence cursor",json!({"id":uuid,"after_sequence":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":1000}}),vec!["id"],true),
        ("spider_analyze_content","Analyze supplied text locally with an explicit similarity/readability profile; no website is fetched",json!({"input":{"type":"object"}}),vec!["input"],true),
        ("spider_preview_policy","Preview supplied scope, query and rewrite decisions locally; no crawl is started",json!({"input":{"type":"object"}}),vec!["input"],true),
        ("spider_cancel","Cancel and retain the saved frontier",json!({"id":uuid}),vec!["id"],false),
        ("spider_pause","Stop admitting new documents, drain active work and persist a resumable checkpoint",json!({"id":uuid}),vec!["id"],false),
        ("spider_resume","Resume a local crawl from SQLite with optional explicit operational overrides",json!({"id":uuid,"options":{"type":"object","properties":{"max_pages":{"type":"integer","minimum":0},"max_attempts":{"type":"integer","minimum":0},"concurrency":{"type":"integer","minimum":1},"delay_ms":{"type":"integer","minimum":0},"request_timeout_secs":{"type":"integer","minimum":1}},"additionalProperties":false}}),vec!["id"],false),
        ("spider_recrawl","Create a derived crawl that refreshes only selected exact requested URLs and retains the source snapshot",json!({"id":uuid,"urls":{"type":"array","items":{"type":"string"},"minItems":1},"max_attempts":{"type":"integer","minimum":0,"description":"Optional selected-attempt budget; zero means unlimited"}}),vec!["id","urls"],false),
        ("spider_project","Read the original project snapshot; paginate large crawls via spider_pages",json!({"id":uuid}),vec!["id"],true),
        ("spider_compare","Compare two crawl snapshots",json!({"before":uuid,"after":uuid}),vec!["before","after"],true),
    ].into_iter().map(|(name,description,properties,required,read)|json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":read,"destructiveHint":false,"openWorldHint":name=="spider_start_crawl"||name=="spider_resume"||name=="spider_recrawl"}})).collect()
}
pub async fn stdio(base: &str) -> anyhow::Result<()> {
    let bridge = Bridge::new(base)?;
    let mut input = BufReader::new(tokio::io::stdin());
    let mut output = tokio::io::stdout();
    let mut line = Vec::new();
    loop {
        line.clear();
        if input.read_until(b'\n', &mut line).await? == 0 {
            break;
        }
        let response = if line.len() > 8 * 1024 * 1024 {
            Some(error(Value::Null, -32600, "Request exceeds 8 MiB"))
        } else {
            match serde_json::from_slice(&line) {
                Ok(request) => bridge.dispatch(request).await,
                Err(_) => Some(error(Value::Null, -32700, "Invalid JSON")),
            }
        };
        if let Some(response) = response {
            output.write_all(response.to_string().as_bytes()).await?;
            output.write_all(b"\n").await?;
            output.flush().await?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn protocol_negotiation_and_errors() {
        let bridge = Bridge::new("http://127.0.0.1:8787").unwrap();
        let result=bridge.dispatch(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}})).await.unwrap();
        assert_eq!(result["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(
            bridge
                .dispatch(json!({"jsonrpc":"2.0","id":2,"method":"unknown"}))
                .await
                .unwrap()["error"]["code"],
            -32601
        );
        assert!(
            bridge
                .dispatch(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
                .await
                .is_none()
        );
        assert_eq!(inventory().len(), 16);
    }
    #[test]
    fn only_loopback_bridge() {
        for base in [
            "https://127.0.0.1",
            "http://example.com",
            "http://user:password@127.0.0.1",
        ] {
            assert!(Bridge::new(base).is_err());
        }
    }
}
