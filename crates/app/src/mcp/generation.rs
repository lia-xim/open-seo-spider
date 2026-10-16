//! Bounded binary sitemap transport; base64 occurs once, only in structuredContent.
use super::Bridge;
use base64::{Engine, engine::general_purpose::STANDARD};
use futures::StreamExt;
use open_seo_spider_core::{
    SITEMAP_GENERATION_ENGINE, SitemapGenerationOptions, validate_sitemap_generation_options,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const MAX_ZIP_BYTES: usize = 8 * 1024 * 1024;
const MAX_ERROR_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

pub(super) fn definition() -> Value {
    let lastmod = json!({"oneOf":[
        {"type":"object","additionalProperties":false,"required":["source"],"properties":{"source":{"const":"omit"}}},
        {"type":"object","additionalProperties":false,"required":["source"],"properties":{"source":{"const":"captured_sitemap"}}},
        {"type":"object","additionalProperties":false,"required":["source","values"],"properties":{"source":{"const":"provided"},"values":{"type":"array","maxItems":1000000,"items":{"type":"object","additionalProperties":false,"required":["page_url","value","reference"],"properties":{"page_url":{"type":"string","minLength":1,"maxLength":2047},"value":{"type":"string","maxLength":4096},"reference":{"type":"string","minLength":1,"maxLength":1024}}}}}}
    ]});
    json!({"name":"spider_generate_sitemaps","description":"Generate a deterministic segmented sitemap ZIP from a saved crawl. No website requests or artifact storage. Returns base64 once, actual SHA-256, filename and byte count. MCP ZIP limit 8 MiB; omitted total budget defaults to 8 MiB. Public base directory constrains origin/path. options.reporting_mode raw/reported or omitted/null Auto applies the saved active reporting profile before XML membership; full raw report SHA and raw lastmod/duplicate/relationship facts remain unchanged. Optional reporting metadata distinguishes captured/profile-visible/eligible/generated counts and equal-count mode identities. No preview/pagination token is introduced. Dates need explicit captured/provided sources; no freshness claim. UTF-8 byte bounds are enforced by the server in addition to these character bounds.","inputSchema":{"type":"object","additionalProperties":false,"required":["id","options"],"properties":{"id":{"type":"string","format":"uuid"},"options":{"type":"object","required":["public_base_url"],"additionalProperties":false,"properties":{
        "public_base_url":{"type":"string","minLength":1,"maxLength":2047},
        "reporting_mode":{"type":["string","null"],"enum":["raw","reported",null]},
        "max_urls_per_file":{"type":"integer","minimum":1,"maximum":50000},
        "max_urlset_uncompressed_bytes":{"type":"integer","minimum":1,"maximum":52428800},
        "max_index_uncompressed_bytes":{"type":"integer","minimum":1,"maximum":52428800},
        "max_total_output_bytes":{"type":"integer","minimum":1,"maximum":MAX_ZIP_BYTES},
        "include_images":{"type":"boolean"},"allow_external_images":{"type":"boolean"},"include_hreflang":{"type":"boolean"},
        "lastmod":lastmod,
        "changefreq":{"enum":[null,"always","hourly","daily","weekly","monthly","yearly","never"]},
        "priority":{"type":["string","null"],"maxLength":4096}
    }}}},"annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
}

#[derive(Debug)]
pub(super) struct Failure {
    code: &'static str,
    message: String,
}
fn failure(code: &'static str, message: impl Into<String>) -> Failure {
    Failure {
        code,
        message: message.into(),
    }
}
fn bad(message: impl Into<String>) -> Failure {
    failure("invalid_arguments", message)
}

fn arguments(args: Value) -> Result<(uuid::Uuid, SitemapGenerationOptions), Failure> {
    let object = args
        .as_object()
        .ok_or_else(|| bad("Arguments must be an object"))?;
    if object.len() != 2 || !object.contains_key("id") || !object.contains_key("options") {
        return Err(bad("Exactly id and options are required"));
    }
    let id = object["id"]
        .as_str()
        .ok_or_else(|| bad("id must be a UUID string"))?
        .parse()
        .map_err(|_| bad("id must be a UUID"))?;
    let mut value = object["options"].clone();
    let options = value
        .as_object_mut()
        .ok_or_else(|| bad("options must be an object"))?;
    if !options.contains_key("public_base_url") {
        return Err(bad("public_base_url is required"));
    }
    options
        .entry("max_total_output_bytes")
        .or_insert(json!(MAX_ZIP_BYTES));
    let options: SitemapGenerationOptions =
        serde_json::from_value(value).map_err(|e| bad(e.to_string()))?;
    if options.max_total_output_bytes > MAX_ZIP_BYTES {
        return Err(bad("MCP max_total_output_bytes must not exceed 8388608"));
    }
    validate_sitemap_generation_options(&options).map_err(|e| bad(e.to_string()))?;
    // A bounded stdio request cannot grow into an unbounded provider body after defaults.
    if serde_json::to_vec(&options)
        .map_err(|e| bad(e.to_string()))?
        .len()
        > MAX_ZIP_BYTES
    {
        return Err(bad("Options exceed 8 MiB transport input budget"));
    }
    Ok((id, options))
}

fn header(headers: &reqwest::header::HeaderMap, name: &str) -> Result<String, Failure> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .ok_or_else(|| failure("invalid_response", format!("Missing or invalid {name}")))
}
fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn reporting_header(
    headers: &reqwest::header::HeaderMap,
    mode: Option<open_seo_spider_core::ReportingMode>,
    generated: u64,
    excluded: u64,
) -> Result<Option<open_seo_spider_core::SitemapGenerationReporting>, Failure> {
    let mut values = headers.get_all("x-sitemap-reporting").iter();
    let Some(raw) = values.next() else {
        if mode.is_some() {
            return Err(failure(
                "invalid_response",
                "Explicit sitemap reporting metadata missing",
            ));
        }
        return Ok(None);
    };
    if values.next().is_some() || raw.as_bytes().len() > 8192 {
        return Err(failure(
            "invalid_response",
            "Invalid bounded sitemap reporting header",
        ));
    }
    let value: open_seo_spider_core::SitemapGenerationReporting =
        serde_json::from_slice(raw.as_bytes())
            .map_err(|_| failure("invalid_response", "Invalid sitemap reporting JSON"))?;
    let profile_valid = value
        .identity
        .directive_reporting
        .as_ref()
        .is_none_or(|p| p.validate().is_ok() && p.active());
    let computed = serde_json::to_vec(&value.identity)
        .map_err(|_| failure("invalid_response", "Invalid sitemap identity"))?;
    let counts = [
        value.captured_page_observations,
        value.profile_visible_page_observations,
        value.profile_excluded_page_observations,
        value.eligible_page_observations,
        value.excluded_reporting_page_observations,
        value.generated_pages,
    ];
    if value.identity.engine != "open-seo-spider/sitemap-generation-reporting-v1"
        || !profile_valid
        || mode.is_some_and(|mode| mode != value.identity.mode)
        || counts.iter().any(|n| *n > 1_000_000)
        || value.captured_page_observations as u64 != generated + excluded
        || value.profile_visible_page_observations + value.profile_excluded_page_observations
            != value.captured_page_observations
        || value.eligible_page_observations
            != if value.identity.mode == open_seo_spider_core::ReportingMode::Raw {
                value.captured_page_observations
            } else {
                value.profile_visible_page_observations
            }
        || value.excluded_reporting_page_observations + value.eligible_page_observations
            != value.captured_page_observations
        || value.generated_pages as u64 != generated
        || value.generated_pages > value.eligible_page_observations
        || !hash(&value.identity_sha256)
        || !hash(&value.eligible_raw_page_ordinals_sha256)
        || !hash(&value.generated_raw_page_ordinals_sha256)
        || format!("{:x}", Sha256::digest(&computed)) != value.identity_sha256
    {
        return Err(failure(
            "invalid_response",
            "Sitemap reporting counts/mode/profile/identity differ",
        ));
    }
    Ok(Some(value))
}
async fn bounded_body(response: reqwest::Response, limit: usize) -> Result<Vec<u8>, Failure> {
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err(failure(
            "too_large",
            format!("API response exceeds {limit} bytes"),
        ));
    }
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| failure("transport_error", e.to_string()))?;
        if bytes
            .len()
            .checked_add(chunk.len())
            .is_none_or(|n| n > limit)
        {
            return Err(failure(
                "too_large",
                format!("API response exceeds {limit} bytes"),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub(super) async fn call(bridge: &Bridge, args: Value) -> Result<Value, Failure> {
    let (id, options) = bridge
        .workers
        .run_blocking(move || arguments(args))
        .await
        .map_err(|e| failure("worker_error", e.to_string()))??;
    let url = bridge
        .base
        .join(&format!("api/crawls/{id}/sitemap-bundle"))
        .map_err(|e| bad(e.to_string()))?;
    let response = bridge
        .client
        .post(url)
        .json(&options)
        .send()
        .await
        .map_err(|e| failure("transport_error", e.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        let bytes = bounded_body(response, MAX_ERROR_BYTES).await?;
        return Err(failure(
            "api_error",
            format!(
                "HTTP {}: {}",
                status.as_u16(),
                String::from_utf8_lossy(&bytes)
            ),
        ));
    }
    let headers = response.headers();
    if header(headers, "content-type")? != "application/zip"
        || header(headers, "x-sitemap-generation-engine")? != SITEMAP_GENERATION_ENGINE
    {
        return Err(failure(
            "invalid_response",
            "Expected sitemap generation engine and ZIP content type",
        ));
    }
    let sha = header(headers, "x-sitemap-bundle-sha256")?;
    let report_sha = header(headers, "x-sitemap-report-sha256")?;
    if !hash(&sha) || !hash(&report_sha) {
        return Err(failure("invalid_response", "Invalid SHA-256 header"));
    }
    let filename = format!("{id}-sitemap-bundle.zip");
    if header(headers, "content-disposition")? != format!("attachment; filename=\"{filename}\"") {
        return Err(failure(
            "invalid_response",
            "Unexpected attachment filename",
        ));
    }
    let count = |name| -> Result<u64, Failure> {
        let value = header(headers, name)?
            .parse::<u64>()
            .map_err(|_| failure("invalid_response", "Invalid sitemap count"))?;
        if value > 1_000_000 {
            return Err(failure(
                "invalid_response",
                "Sitemap count exceeds report input bound",
            ));
        }
        Ok(value)
    };
    let generated = count("x-sitemap-generated-urls")?;
    let excluded = count("x-sitemap-excluded-pages")?;
    if generated == 0 || generated + excluded > 1_000_000 {
        return Err(failure(
            "invalid_response",
            "Invalid generated/excluded counts",
        ));
    }
    let reporting = reporting_header(headers, options.reporting_mode, generated, excluded)?;
    let length = response
        .content_length()
        .ok_or_else(|| failure("invalid_response", "ZIP response requires content-length"))?;
    let bytes = bounded_body(response, options.max_total_output_bytes.min(MAX_ZIP_BYTES)).await?;
    if bytes.len() as u64 != length
        || !bytes.starts_with(b"PK\x03\x04")
        || format!("{:x}", Sha256::digest(&bytes)) != sha
    {
        return Err(failure(
            "invalid_response",
            "ZIP byte length, signature or actual SHA-256 differs",
        ));
    }
    let mut value = json!({"mime_type":"application/zip","filename":filename,"bytes":bytes.len(),"sha256":sha,"report_sha256":report_sha,"engine":SITEMAP_GENERATION_ENGINE,"generated_urls":generated,"excluded_pages":excluded,"encoding":"base64","data_base64":STANDARD.encode(bytes),"artifact_stored":false,"scope":"Captured saved report; no website request, publication or freshness assertion"});
    if let Some(reporting) = reporting {
        value["reporting"] = serde_json::to_value(reporting)
            .map_err(|e| failure("invalid_response", e.to_string()))?;
    }
    Ok(value)
}

fn bounded_protocol_error(id: Value, message: &str) -> Value {
    let response = super::error(id, -32603, message);
    if serde_json::to_vec(&response).is_ok_and(|bytes| bytes.len() <= MAX_RESPONSE_BYTES) {
        response
    } else {
        // Ordinary stdio input IDs fit: the entire inbound line is at most 8 MiB.
        // A direct in-process caller can supply an ID too large even for an error.
        super::error(
            Value::Null,
            -32603,
            "Sitemap error request ID exceeds output budget",
        )
    }
}
#[cfg(test)]
pub(super) async fn complete_response(id: Value, result: Result<Value, Failure>) -> Value {
    let join_error_id = id.clone();
    tokio::task::spawn_blocking(move || {
        let budget_error_id = id.clone();
        let response = match result {
            Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":format!("Sitemap ZIP received: {} bytes, SHA-256 {}. Binary is in structuredContent.data_base64; no artifact was stored.",value["bytes"],value["sha256"])}],"structuredContent":value,"isError":false}}),
            Err(e) => json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":e.message}],"structuredContent":{"code":e.code,"message":e.message},"isError":true}}),
        };
        if serde_json::to_vec(&response).is_ok_and(|v| v.len() <= MAX_RESPONSE_BYTES) { response }
        else { bounded_protocol_error(budget_error_id,"Sitemap response exceeds 16 MiB JSON-RPC output budget") }
    }).await.unwrap_or_else(|_| bounded_protocol_error(join_error_id,"Sitemap response worker failed"))
}

pub(super) async fn complete_response_owned(
    owner: &open_seo_spider_core::TaskOwner,
    id: Value,
    result: Result<Value, Failure>,
) -> Value {
    let join_error_id = id.clone();
    owner.run_blocking(move || {
        let budget_error_id = id.clone();
        let response = match result {
            Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":format!("Sitemap ZIP received: {} bytes, SHA-256 {}. Binary is in structuredContent.data_base64; no artifact was stored.",value["bytes"],value["sha256"])}],"structuredContent":value,"isError":false}}),
            Err(e) => json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":e.message}],"structuredContent":{"code":e.code,"message":e.message},"isError":true}}),
        };
        if serde_json::to_vec(&response).is_ok_and(|v| v.len() <= MAX_RESPONSE_BYTES) { response }
        else { bounded_protocol_error(budget_error_id,"Sitemap response exceeds 16 MiB JSON-RPC output budget") }
    }).await.unwrap_or_else(|_| bounded_protocol_error(join_error_id,"Sitemap response worker failed"))
}

#[cfg(test)]
#[path = "generation_tests.rs"]
mod tests;
