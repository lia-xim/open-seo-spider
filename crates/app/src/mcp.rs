//! Original MCP bridge; crawl state and execution belong to the local API.
use anyhow::{Context, bail};
use reqwest::{Client, Url};
use serde_json::{Value, json};
use std::{io::Write, net::SocketAddr, path::PathBuf};
use tokio::io::{AsyncRead, AsyncWrite};
#[cfg(test)]
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
pub const PROTOCOLS: &[&str] = &["2025-06-18", "2025-11-25"];
mod broken_sources;
mod crawl_quotas;
mod enrichment;
mod files;
mod generation;
mod ngrams;
mod saved_comparison;
mod segments;
mod sitemap_findings;
mod sitemaps;
mod spelling;
fn append_content_reporting_query(url: &mut Url, args: &Value) -> anyhow::Result<()> {
    if let Some(value) = args.get("reporting_mode").filter(|v| !v.is_null()) {
        let mode = value
            .as_str()
            .context("reporting_mode must be raw/reported or null (Auto)")?;
        anyhow::ensure!(
            matches!(mode, "raw" | "reported"),
            "Unknown content reporting mode"
        );
        url.query_pairs_mut().append_pair("reporting_mode", mode);
    }
    if let Some(value) = args.get("expected_generation").filter(|v| !v.is_null()) {
        let generation = value
            .as_u64()
            .context("expected_generation must be a nonnegative integer")?;
        url.query_pairs_mut()
            .append_pair("expected_generation", &generation.to_string());
    }
    for key in ["expected_snapshot_sha256", "expected_reporting_identity"] {
        if let Some(value) = args.get(key).filter(|v| !v.is_null()) {
            let digest = value
                .as_str()
                .context("Expected saved content identity must be a string")?;
            anyhow::ensure!(
                digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()),
                "Invalid expected saved content identity"
            );
            url.query_pairs_mut().append_pair(key, digest);
        }
    }
    Ok(())
}
#[derive(Clone)]
pub struct Bridge {
    client: Client,
    base: Url,
    files: Option<files::FileTools>,
    workers: open_seo_spider_core::TaskOwner,
}
impl Bridge {
    pub fn new(base: &str) -> anyhow::Result<Self> {
        if base.len() > 2048 || base.chars().any(char::is_control) {
            bail!("Invalid MCP API base URL");
        }
        let original_base = base;
        let base = Url::parse(base)?;
        if base.scheme() != "http"
            || !matches!(base.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
            || !base.username().is_empty()
            || base.password().is_some()
            || base.path() != "/"
            || base.query().is_some()
            || base.fragment().is_some()
            || (base.as_str() != original_base
                && base.as_str().strip_suffix('/') != Some(original_base))
        {
            bail!("MCP requires an HTTP loopback API");
        }
        Ok(Self {
            base,
            client: Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(60))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            files: None,
            workers: open_seo_spider_core::TaskOwner::new("mcp_blocking"),
        })
    }

    pub fn new_with_file_tools(base: &str, directory: Option<PathBuf>) -> anyhow::Result<Self> {
        let mut bridge = Self::new(base)?;
        bridge.files = directory
            .as_deref()
            .map(files::FileTools::open)
            .transpose()?;
        Ok(bridge)
    }

    pub(crate) fn file_tools_enabled(&self) -> bool {
        self.files.is_some()
    }

    pub(crate) fn file_tools_capability(&self) -> Option<Value> {
        self.files.as_ref().map(|_| {
            json!({
                "enabled":true,"opt_in":true,"scope":"explicit_local_directory",
                "website_requests":0,"tools":files::FileTools::definitions().iter().map(|tool| tool["name"].clone()).collect::<Vec<_>>(),
                "limits":{
                    "write_text_utf8_bytes":files::WRITE_TEXT_MAX_BYTES,
                    "read_chunk_utf8_bytes":files::READ_CHUNK_MAX_BYTES,
                    "read_file_utf8_bytes":files::READ_FILE_MAX_BYTES,
                    "directory_entries":files::DIRECTORY_MAX_ENTRIES,
                    "directory_name_utf8_bytes":files::DIRECTORY_NAME_MAX_BYTES,
                    "concurrent_operations":files::MAX_CONCURRENT_FILE_OPS,
                    "result_json_bytes":files::MAX_RESULT_JSON_BYTES,
                    "mcp_wire_bytes":MAX_FILE_WIRE_BYTES,"rpc_id_utf8_bytes":1024
                }
            })
        })
    }

    pub(crate) fn set_task_owner(&mut self, workers: open_seo_spider_core::TaskOwner) {
        self.workers = workers;
    }
    pub(crate) fn close_file_tools_admission(&self) {
        if let Some(files) = &self.files {
            files.close_admission();
        }
    }
    async fn shutdown_owned_work(&self) -> anyhow::Result<()> {
        let (files, workers) = tokio::join!(
            self.shutdown_file_tools(),
            self.workers.shutdown_and_drain()
        );
        files?;
        workers?;
        Ok(())
    }
    pub(crate) async fn shutdown_file_tools(&self) -> anyhow::Result<()> {
        match &self.files {
            Some(files) => files.shutdown_and_drain().await,
            None => Ok(()),
        }
    }

    // Retain the already opened directory capability when a port-zero listener
    // obtains its actual local address. Never reopen a pathname after binding.
    pub(crate) fn set_local_api_address(&mut self, address: SocketAddr) -> anyhow::Result<()> {
        if address.ip().is_loopback() {
            self.base = Url::parse(&format!("http://{address}"))?;
            return Ok(());
        }
        let host = if address.is_ipv6() {
            "[::1]"
        } else {
            "127.0.0.1"
        };
        self.base = Url::parse(&format!("http://{host}:{}", address.port()))?;
        Ok(())
    }

    fn inventory(&self) -> Vec<Value> {
        let mut tools = inventory();
        tools.push(enrichment::definition());
        tools.push(segments::definition());
        tools.push(crawl_quotas::definition());
        tools.push(ngrams::definition());
        tools.push(broken_sources::definition());
        tools.push(spelling::definition());
        if self.files.is_some() {
            tools.extend(files::FileTools::definitions());
        }
        tools
    }

    pub async fn dispatch(&self, request: Value) -> Option<Value> {
        if !request.is_object() {
            return Some(error(
                Value::Null,
                -32600,
                "A single JSON-RPC object is required",
            ));
        }
        let id_ref = request.get("id")?;
        if request["method"] == "tools/call"
            && request["params"]["name"]
                .as_str()
                .is_some_and(files::FileTools::is_tool)
            && !sitemap_findings::valid_rpc_id(id_ref)
        {
            return Some(error(
                Value::Null,
                -32600,
                "File RPC id must be an integer or at most 1024 UTF-8 bytes",
            ));
        }
        if request["method"] == "tools/call"
            && request["params"]["name"] == sitemap_findings::NAME
            && !sitemap_findings::valid_rpc_id(id_ref)
        {
            return Some(error(
                Value::Null,
                -32600,
                "Finding RPC id must be an integer or at most 1024 UTF-8 bytes",
            ));
        }
        let id = id_ref.clone();
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
            "tools/list" => json!({"tools":self.inventory()}),
            "tools/call" => {
                let Some(name) = request["params"]["name"].as_str() else {
                    return Some(error(id, -32602, "Tool name required"));
                };
                let args = request["params"]
                    .get("arguments")
                    .cloned()
                    .unwrap_or(json!({}));
                let definitions = self.inventory();
                let definition = definitions
                    .iter()
                    .find(|tool| tool["name"].as_str() == Some(name));
                if !args.is_object() || definition.is_none() {
                    return Some(error(id, -32602, "Invalid tool or arguments"));
                }
                let schema = &definition.expect("checked definition")["inputSchema"];
                let values = args.as_object().expect("checked object");
                if values
                    .keys()
                    .any(|key| schema["properties"].get(key).is_none())
                    || schema["required"].as_array().is_some_and(|keys| {
                        keys.iter()
                            .any(|key| key.as_str().is_some_and(|key| !values.contains_key(key)))
                    })
                {
                    return Some(error(id, -32602, "Unexpected or missing tool argument"));
                }
                if files::FileTools::is_tool(name) {
                    let Some(files) = &self.files else {
                        return Some(error(id, -32602, "File tools are disabled"));
                    };
                    if !file_wire_capacity(&id) {
                        return Some(error(
                            id,
                            -32603,
                            "File result budget cannot fit MCP wire limit",
                        ));
                    }
                    let result = files.call(name, args).await;
                    return Some(file_result(id, result));
                }
                if name == spelling::NAME {
                    if !spelling::valid_id(&id) {
                        return Some(error(
                            Value::Null,
                            -32602,
                            "Spelling RPC id exceeds bound or has invalid type",
                        ));
                    }
                    let result = spelling::call(self, args).await;
                    return Some(spelling::complete_response(id, result));
                }
                if name == sitemap_findings::NAME {
                    let result = sitemap_findings::call(self, args).await;
                    return Some(
                        sitemap_findings::complete_response_owned(&self.workers, id, result).await,
                    );
                }
                if name == "spider_generate_sitemaps" {
                    let result = generation::call(self, args).await;
                    return Some(
                        generation::complete_response_owned(&self.workers, id, result).await,
                    );
                }
                if name == "spider_compare_saved" {
                    let include_original_reports = args
                        .get("include_original_reports")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let result = saved_comparison::call(self, args).await;
                    return Some(
                        saved_comparison::complete_response_on_worker_owned(
                            &self.workers,
                            id,
                            result,
                            include_original_reports,
                        )
                        .await,
                    );
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
        if name == crawl_quotas::NAME {
            return crawl_quotas::call(self, args).await;
        }
        if name == spelling::NAME {
            return spelling::call(self, args).await;
        }
        if name == broken_sources::NAME {
            return broken_sources::call(self, args).await;
        }
        if name == ngrams::NAME {
            return ngrams::call(self, args).await;
        }
        if name == segments::NAME {
            return segments::call(self, args).await;
        }
        if name == enrichment::NAME {
            return enrichment::call(self, args).await;
        }
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
            "spider_inspect_css" => {
                body = Some(args.get("input").cloned().context("input required")?);
                "api/css/inspect".to_owned()
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
                    "spider_sitemaps" => "/sitemaps",
                    "spider_resources" => "/resources",
                    "spider_resource_transports" => "/resources/transports",
                    "spider_content" => "/content",
                    "spider_content_document" => "/content/document",
                    "spider_analyze_crawl_content" => {
                        body = Some(args.get("input").cloned().context("input required")?);
                        "/content/analyze"
                    }
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
        let mut url = self.base.join(&route)?;
        if name == "spider_sitemaps" {
            let query = sitemaps::query(&args)?;
            for (key, value) in sitemaps::query_pairs(query) {
                url.query_pairs_mut().append_pair(&key, &value);
            }
        }
        if matches!(name, "spider_resources" | "spider_resource_transports") {
            let limit = args
                .get("limit")
                .map(|v| v.as_u64().context("limit must be an integer"))
                .transpose()?
                .unwrap_or(200);
            anyhow::ensure!((1..=1000).contains(&limit), "limit must be 1..1000");
            url.query_pairs_mut()
                .append_pair("limit", &limit.to_string());
            if name == "spider_resources" {
                if let Some(value) = args.get("after_sequence") {
                    let sequence = value
                        .as_u64()
                        .context("after_sequence must be a nonnegative integer")?;
                    anyhow::ensure!(
                        sequence <= i64::MAX as u64,
                        "after_sequence exceeds SQLite integer range"
                    );
                    url.query_pairs_mut()
                        .append_pair("after_sequence", &sequence.to_string());
                }
            } else if let Some(value) = args.get("after_url") {
                let cursor = value.as_str().context("after_url must be a string")?;
                anyhow::ensure!(
                    !cursor.is_empty() && cursor.len() <= 65536,
                    "Invalid after_url length"
                );
                url.query_pairs_mut().append_pair("after_url", cursor);
            }
        }
        if matches!(name, "spider_content" | "spider_content_document") {
            append_content_reporting_query(&mut url, &args)?;
            let source = args
                .get("source_kind")
                .map(|value| value.as_str().context("source_kind must be a string"))
                .transpose()?
                .unwrap_or("raw_http_html");
            anyhow::ensure!(
                matches!(source, "raw_http_html" | "rendered_dom_html"),
                "Unknown capture source_kind"
            );
            url.query_pairs_mut().append_pair("source_kind", source);
            if name == "spider_content" {
                let limit = args
                    .get("limit")
                    .map(|value| value.as_u64().context("limit must be an integer"))
                    .transpose()?
                    .unwrap_or(200);
                anyhow::ensure!((1..=200).contains(&limit), "limit must be 1..200");
                let after = args
                    .get("after_ordinal")
                    .map(|value| {
                        value
                            .as_u64()
                            .context("after_ordinal must be a nonnegative integer")
                    })
                    .transpose()?
                    .unwrap_or(0);
                url.query_pairs_mut()
                    .append_pair("limit", &limit.to_string())
                    .append_pair("after_ordinal", &after.to_string());
            } else {
                let requested = args
                    .get("url")
                    .and_then(Value::as_str)
                    .context("exact requested url required")?;
                anyhow::ensure!(
                    !requested.is_empty() && requested.len() <= 16 * 1024,
                    "Invalid requested URL length"
                );
                url.query_pairs_mut().append_pair("url", requested);
            }
        }
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

const MAX_FILE_WIRE_BYTES: usize = 8 * 1024 * 1024;

struct FileWireCounter(usize);
impl Write for FileWireCounter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let next = self.0.checked_add(bytes.len()).filter(|next| {
            // The shared stdio writer appends one physical newline.
            *next < MAX_FILE_WIRE_BYTES
        });
        let Some(next) = next else {
            return Err(std::io::Error::other(
                "File response exceeds MCP wire limit",
            ));
        };
        self.0 = next;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn file_envelope(id: Value, value: Value, failed: bool) -> Value {
    let text = if failed {
        "Local file operation failed; see structuredContent."
    } else {
        "Local file operation completed; see structuredContent."
    };
    json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":text}],"structuredContent":value,"isError":failed}})
}

fn file_wire_capacity(id: &Value) -> bool {
    // This runs BEFORE FileTools::call and therefore before any write. Its
    // declared maximum is actual encoded result JSON, not raw file/text bytes.
    // Keep the conservative null-placeholder overhead rather than subtracting it.
    let mut count = FileWireCounter(0);
    serde_json::to_writer(&mut count, &file_envelope(id.clone(), Value::Null, false)).is_ok()
        && count
            .0
            .checked_add(files::MAX_RESULT_JSON_BYTES)
            .is_some_and(|maximum| maximum < MAX_FILE_WIRE_BYTES)
}

fn file_result(id: Value, result: anyhow::Result<Value>) -> Value {
    let (value, failed) = match result {
        Ok(value) => (value, false),
        Err(failure) => {
            let mut message = failure.to_string();
            if message.len() > 4096 {
                let mut end = 4096;
                while !message.is_char_boundary(end) {
                    end -= 1;
                }
                message.truncate(end);
            }
            (json!({"code":"file_tool_failed","error":message}), true)
        }
    };
    let response = file_envelope(id, value, failed);
    if serde_json::to_writer(&mut FileWireCounter(0), &response).is_err() {
        // All framing is still private. Emit one complete small error instead of
        // a successful prefix, retaining only the already validated small ID.
        return error(
            response["id"].clone(),
            -32603,
            "File response exceeds 8 MiB MCP wire limit; request a smaller result",
        );
    }
    response
}

#[cfg(test)]
#[path = "mcp/crawl_admission_limits_tests.rs"]
mod crawl_admission_limits_tests;

#[cfg(test)]
#[path = "mcp/cdn_classification_tests.rs"]
mod cdn_classification_tests;
#[cfg(test)]
#[path = "mcp/cdn_directory_reporting_tests.rs"]
mod cdn_directory_reporting_tests;

fn inventory() -> Vec<Value> {
    let uuid = json!({"type":"string","format":"uuid"});
    let mut tools: Vec<Value> = vec![
        ("spider_capabilities","Read implemented and optional capabilities",json!({}),vec![],true),
        ("spider_list_crawls","List saved and active crawls",json!({}),vec![],true),
        ("spider_start_crawl","Start a crawl for user-requested targets",json!({"config":{"type":"object","description":"CrawlConfig: seed_urls or seed_url, sitemap_urls, concurrency, delay_ms, obey_robots, max_pages, optional renderer and sitemap_source_processing (max_post_http_entity_bytes, max_decoded_xml_bytes, max_xml_events), plus independent optional sitemap_membership_limits (max_catalog_json_bytes:1..134217728, max_row_json_bytes:1..1048576 and no greater than catalog cap, max_rows:1..1000000); absent membership limits preserve 32 MiB catalog/256 KiB row/1M rows, source work budgets do not widen page bodies or membership retention","properties":{"tls":crate::tls_config::schema(),"directive_reporting":{"type":["object","null"],"additionalProperties":false,"required":["version","respect_noindex","respect_canonical","respect_next_prev"],"properties":{"version":{"type":"integer","const":1},"respect_noindex":{"type":"boolean"},"respect_canonical":{"type":"boolean"},"respect_next_prev":{"type":"boolean"}},"description":"Optional report-row hiding only. Crawling and outlink discovery remain unchanged. Canonical uses captured document identity; next/prev hides pages with observed prev only. project.json remains raw evidence."},"crawl_quotas":crawl_quotas::config_schema(),"crawl_admission_limits":{"type":["object","null"],"additionalProperties":false,"description":"Opt-in version1 native PageDocument and redirect guards, before robots/rate/HTTP; renderer combination is refused. None/null preserves legacy behavior. At least one non-null bound is required. Folder depth counts all completed literal slash segments including empty segments, not encoded %2F. Raw query splits only on &: absent=0, present empty=1, duplicate keys/empty tokens count. Fetch bytes measure the normalized effective fetch alias, without rewriting original observations. No dynamic quotas.","required":["version"],"properties":{"version":{"type":"integer","const":1},"max_folder_depth":{"type":["integer","null"],"minimum":0,"maximum":65535},"max_query_tokens":{"type":["integer","null"],"minimum":0,"maximum":65535},"max_fetch_url_utf8_bytes":{"type":["integer","null"],"minimum":1,"maximum":1048576}},"anyOf":[{"required":["max_folder_depth"],"properties":{"max_folder_depth":{"type":"integer"}}},{"required":["max_query_tokens"],"properties":{"max_query_tokens":{"type":"integer"}}},{"required":["max_fetch_url_utf8_bytes"],"properties":{"max_fetch_url_utf8_bytes":{"type":"integer"}}}]},"cdn_directory_reporting":{"type":["object","null"],"additionalProperties":false,"description":"Optional own reporting profile: literal trailing-slash directories on ASCII DNS hosts, cross HTTP/HTTPS and one www alias, exact supported non-default port. Does not grant fetch, traversal, redirect, resource, header or credential permission; no vendor parity claim. Explicit80/443, IDNA, IPv6, wildcard and dot-segment declarations unsupported.","required":["version","rules"],"properties":{"version":{"type":"integer","const":1},"rules":{"type":"array","minItems":1,"maxItems":32,"uniqueItems":true,"items":{"type":"string","minLength":1,"maxLength":2048}}}},"cdn_classification":{"type":["object","null"],"additionalProperties":false,"description":"Optional version1 declared reporting classification for exact HTTP(S) origins. This does not grant fetch, HTML traversal, resource-origin or credential permission; no wildcard/subdomain inheritance or path scopes. None/null preserves legacy behavior.","required":["version","origins"],"properties":{"version":{"type":"integer","const":1},"origins":{"type":"array","minItems":1,"maxItems":32,"uniqueItems":true,"items":{"type":"string","minLength":1,"maxLength":2048}}}},"sitemap_membership_limits":{"type":["object","null"],"additionalProperties":false,"description":"Independent explicit durable membership capacity; null/absence keeps legacy defaults. Row bytes cannot exceed catalog bytes; Rust validates this relation before crawl IO.","properties":{"max_catalog_json_bytes":{"type":"integer","minimum":1,"maximum":134217728,"default":33554432},"max_row_json_bytes":{"type":"integer","minimum":1,"maximum":1048576,"default":262144},"max_rows":{"type":"integer","minimum":1,"maximum":1000000,"default":1000000}}}}}}),vec!["config"],false),
        ("spider_status","Read crawl progress and warnings",json!({"id":uuid}),vec!["id"],true),
        ("spider_pages","Read a bounded page slice; increment offset for more",json!({"id":uuid,"offset":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":10000}}),vec!["id"],true),
        ("spider_issues","Read observed audit issues",json!({"id":uuid}),vec!["id"],true),
        ("spider_analysis","Read graph and relationship/security/JSON-LD observations",json!({"id":uuid}),vec!["id"],true),
        ("spider_policy","Read bounded native admission evidence; advance the returned sequence cursor",json!({"id":uuid,"after_sequence":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":1000}}),vec!["id"],true),
        ("spider_sitemaps","Read original sitemap rows and independently paged saved target/inlink findings without fetching websites; missing membership and zero observed inlinks are cohort/report-qualified",json!({"id":uuid,"kind":{"type":"string","enum":["all","root","document","entry","location"]},"source_observation_id":{"type":"string","pattern":"^[0-9a-f]{64}$"},"membership_url":{"type":"string","minLength":1,"maxLength":65536},"limit":{"type":"integer","minimum":1,"maximum":200},"cursor":{"type":"string","minLength":1,"maxLength":4096},"page_limit":{"type":"integer","minimum":0,"maximum":200},"page_cursor":{"type":"string","minLength":1,"maxLength":4096}}),vec!["id"],true),
        ("spider_resources","Read bounded native source occurrences and immutable resource grants without fetching websites",json!({"id":uuid,"after_sequence":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":1000}}),vec!["id"],true),
        ("spider_resource_transports","Read bounded stored transport metadata, including checked responses excluded from report rows",json!({"id":uuid,"after_url":{"type":"string","minLength":1,"maxLength":65536},"limit":{"type":"integer","minimum":1,"maximum":1000}}),vec!["id"],true),
        ("spider_inspect_css","Inspect declared dependencies in supplied CSS with an explicit bounded parser profile; no website is fetched",json!({"input":{"type":"object"}}),vec!["input"],true),
        ("spider_content","Read a bounded saved-content inventory; optional raw/reported mode or absent/null Auto applies saved reporting profile before selection, retaining raw ordinals and snapshot identity; no website requests",json!({"id":uuid,"reporting_mode":{"type":["string","null"],"enum":["raw","reported",null],"description":"Absent/null is Auto: active saved directive_reporting profile, otherwise legacy raw."},"expected_generation":{"type":["integer","null"],"minimum":0},"expected_snapshot_sha256":{"type":["string","null"],"pattern":"^[a-fA-F0-9]{64}$"},"expected_reporting_identity":{"type":["string","null"],"pattern":"^[a-fA-F0-9]{64}$"},"source_kind":{"type":"string","enum":["raw_http_html","rendered_dom_html"]},"after_ordinal":{"type":"integer","minimum":0},"limit":{"type":"integer","minimum":1,"maximum":200}}),vec!["id"],true),
        ("spider_content_document","Read one exact saved requested URL; a known profile-hidden URL returns excluded_reporting and capture:null, while unknown URLs still refuse. Optional raw/reported or absent/null Auto; no fetch",json!({"id":uuid,"reporting_mode":{"type":["string","null"],"enum":["raw","reported",null],"description":"Absent/null is Auto: active saved directive_reporting profile, otherwise legacy raw."},"expected_generation":{"type":["integer","null"],"minimum":0},"expected_snapshot_sha256":{"type":["string","null"],"pattern":"^[a-fA-F0-9]{64}$"},"expected_reporting_identity":{"type":["string","null"],"pattern":"^[a-fA-F0-9]{64}$"},"url":{"type":"string"},"source_kind":{"type":"string","enum":["raw_http_html","rendered_dom_html"]}}),vec!["id","url"],true),
        ("spider_analyze_crawl_content","Analyze 1..128 explicit saved URLs through the same HTTP/CLI reporting service; input.reporting_mode raw/reported or absent/null Auto applies the active saved profile before analysis. Known hidden selections remain explicit exclusions; expected_generation/expected_snapshot_sha256/expected_reporting_identity bind the saved view. No website fetch",json!({"id":uuid,"input":{"type":"object"}}),vec!["id","input"],true),
        ("spider_analyze_content","Analyze supplied text locally with an explicit similarity/readability profile; no website is fetched",json!({"input":{"type":"object"}}),vec!["input"],true),
        ("spider_preview_policy","Preview supplied scope, query and rewrite decisions locally; no crawl is started",json!({"input":{"type":"object"}}),vec!["input"],true),
        ("spider_cancel","Cancel and retain the saved frontier",json!({"id":uuid}),vec!["id"],false),
        ("spider_pause","Stop admitting new documents, drain active work and persist a resumable checkpoint",json!({"id":uuid}),vec!["id"],false),
        ("spider_resume","Resume a local crawl from SQLite with optional explicit operational overrides",json!({"id":uuid,"options":{"type":"object","properties":{"max_pages":{"type":"integer","minimum":0},"max_attempts":{"type":"integer","minimum":0},"concurrency":{"type":"integer","minimum":1},"delay_ms":{"type":"integer","minimum":0},"request_timeout_secs":{"type":"integer","minimum":1},"quota_increases":crawl_quotas::increases_schema()},"additionalProperties":false}}),vec!["id"],false),
        ("spider_recrawl","Create a derived crawl that refreshes only selected exact requested URLs and retains the source snapshot",json!({"id":uuid,"urls":{"type":"array","items":{"type":"string"},"minItems":1},"max_attempts":{"type":"integer","minimum":0,"description":"Optional selected-attempt budget; zero means unlimited"}}),vec!["id","urls"],false),
        ("spider_project","Read the original project snapshot; paginate large crawls via spider_pages",json!({"id":uuid}),vec!["id"],true),
        ("spider_compare","Compare two crawl snapshots",json!({"before":uuid,"after":uuid}),vec!["before","after"],true),
    ].into_iter().map(|(name,description,properties,required,read)|json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":read,"destructiveHint":false,"openWorldHint":name=="spider_start_crawl"||name=="spider_resume"||name=="spider_recrawl"}})).collect();
    tools.push(generation::definition());
    tools.push(saved_comparison::definition());
    tools.push(sitemap_findings::definition());
    tools
}
const MAX_STDIO_REQUEST_BYTES: usize = 8 * 1024 * 1024;

pub async fn stdio(base: &str) -> anyhow::Result<()> {
    stdio_with_file_tools(base, None).await
}

pub async fn stdio_with_file_tools(base: &str, directory: Option<PathBuf>) -> anyhow::Result<()> {
    let bridge = Bridge::new_with_file_tools(base, directory)?;
    stdio_and_drain(&bridge, tokio::io::stdin(), tokio::io::stdout()).await
}

async fn stdio_and_drain<R, W>(bridge: &Bridge, input: R, output: W) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let serving = stdio_stream(bridge, input, output).await;
    let drain = bridge.shutdown_owned_work().await;
    serving?;
    drain
}

async fn stdio_stream<R, W>(bridge: &Bridge, input: R, output: W) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    sitemap_findings::stdio_stream(bridge, input, output).await
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
        assert_eq!(inventory().len(), 26);
    }
    #[test]
    fn saved_content_reporting_query_forwards_exact_mode_and_stale_guards_without_a_new_tool() {
        let mut url =
            Url::parse("http://127.0.0.1/api/crawls/00000000-0000-0000-0000-000000000000/content")
                .unwrap();
        append_content_reporting_query(&mut url,&json!({"reporting_mode":"reported","expected_generation":7,
            "expected_snapshot_sha256":"a".repeat(64),"expected_reporting_identity":"b".repeat(64)})).unwrap();
        let fields = url
            .query_pairs()
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(fields["reporting_mode"], "reported");
        assert_eq!(fields["expected_generation"], "7");
        assert_eq!(fields["expected_snapshot_sha256"], "a".repeat(64));
        assert_eq!(fields["expected_reporting_identity"], "b".repeat(64));
        let before = inventory();
        assert_eq!(
            before
                .iter()
                .filter(|v| v["name"] == "spider_content")
                .count(),
            1
        );
        assert!(
            before
                .iter()
                .find(|v| v["name"] == "spider_content")
                .unwrap()["inputSchema"]["properties"]
                .get("reporting_mode")
                .is_some()
        );
        for bad in [
            json!({"reporting_mode":"auto"}),
            json!({"reporting_mode":true}),
            json!({"expected_generation":-1}),
            json!({"expected_snapshot_sha256":"bad"}),
        ] {
            assert!(append_content_reporting_query(&mut url, &bad).is_err());
        }
        let mut auto = Url::parse("http://127.0.0.1/").unwrap();
        append_content_reporting_query(
            &mut auto,
            &json!({"reporting_mode":null,"expected_reporting_identity":null}),
        )
        .unwrap();
        assert_eq!(auto.query(), None);
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
    #[tokio::test]
    async fn advertised_resource_argument_shapes_reject_before_api_io() {
        let bridge = Bridge::new("http://127.0.0.1:1").unwrap();
        for (name, args) in [
            (
                "spider_resources",
                json!({"id":"00000000-0000-0000-0000-000000000000","after_url":"https://example.test/"}),
            ),
            (
                "spider_resource_transports",
                json!({"id":"00000000-0000-0000-0000-000000000000","after_sequence":1}),
            ),
            ("spider_inspect_css", json!({"input":{},"options":{}})),
            ("spider_resources", json!({"limit":1})),
        ] {
            let result=bridge.dispatch(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}})).await.unwrap();
            assert_eq!(result["error"]["code"], -32602);
        }
    }
}

#[cfg(test)]
#[path = "mcp/sitemaps_tests.rs"]
mod sitemap_argument_tests;

#[cfg(test)]
#[path = "mcp/stdio_tests.rs"]
mod stdio_tests;

#[cfg(test)]
#[path = "mcp/wiring_tests.rs"]
mod file_wiring_tests;
