//! Read-only transport over the two existing saved broken-source HTTP projections.
//! This module opens neither crawl stores nor websites and never joins their scopes.
use super::Bridge;
use anyhow::{Context, Result, ensure};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};

pub(super) const NAME: &str = "spider_broken_sources";
const BODY_BYTES: usize = 128 * 1024;

pub(super) fn definition() -> Value {
    let reporting = json!({"type":["string","null"],"enum":["raw","reported",null],"description":"Absent/null is Auto: use an active saved directive reporting profile, otherwise raw. This changes the saved projection, never fetches a website."});
    let generation = json!({"type":["integer","null"],"minimum":0});
    let page_links = json!({"type":"object","additionalProperties":false,"properties":{
        "filter":{"enum":["actionable","broken","unknown","all"]},
        "limit":{"type":"integer","minimum":1,"maximum":200},
        "expected_generation":generation,"expected_snapshot_token":{"type":["string","null"],"pattern":"^[0-9A-Fa-f]{64}$"},
        "cursor":{"type":["string","null"],"maxLength":2048},"reporting_mode":reporting
    }});
    let resources = json!({"type":"object","additionalProperties":false,"properties":{
        "filter":{"enum":["all","failures","unknown"]},
        "limit":{"type":["integer","null"],"minimum":1,"maximum":200},
        "expected_generation":generation,"expected_snapshot_token":{"type":["string","null"],"maxLength":256},
        "after_sequence":{"type":["integer","null"],"minimum":0},"reporting_mode":reporting,
        "limits":{"type":"object","additionalProperties":false,"properties":{
            "max_evaluation_rows":{"type":"integer","minimum":1,"maximum":250000},
            "max_transports":{"type":"integer","minimum":1,"maximum":50000},
            "max_column_utf8_bytes":{"type":"integer","minimum":1,"maximum":268435456},
            "max_row_utf8_bytes":{"type":"integer","minimum":1,"maximum":33554432}
        }}
    }});
    json!({"name":NAME,"description":"Inspect existing saved broken-source observations. Choose page_links for retained page links or resources for the verified resource ledger; these scopes are not interchangeable and neither claims whole-site coverage. Preview returns paged JSON. CSV requires the same preview token and no continuation, and returns complete base64 bytes plus verified HTTP length/SHA256. The resource CSV also verifies its snapshot header. Page-link JSON has no required server checksum attestation. No website requests, crawling, store paths or mutation. Request maximum4096 bytes, response maximum128KiB; use smaller pages or offline CLI CSV above that cap.","inputSchema":{
        "type":"object","additionalProperties":false,"required":["id","scope","request"],
        "properties":{"id":{"type":"string","format":"uuid"},"scope":{"enum":["page_links","resources"]},"action":{"enum":["preview","csv"],"default":"preview"},"request":{"type":"object"}},
        "oneOf":[{"properties":{"scope":{"const":"page_links"},"request":page_links}},{"properties":{"scope":{"const":"resources"},"request":resources}}]
    },"annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true,"openWorldHint":false}})
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Scope {
    PageLinks,
    Resources,
}
impl Scope {
    fn route(self) -> &'static str {
        match self {
            Self::PageLinks => "broken-sources",
            Self::Resources => "resource-broken-sources",
        }
    }
    fn contract(self) -> &'static str {
        match self {
            Self::PageLinks => crate::broken_sources::SCOPE,
            Self::Resources => crate::resource_broken_sources_app::SCOPE,
        }
    }
}
#[derive(Default, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    #[default]
    Preview,
    Csv,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    id: uuid::Uuid,
    scope: Scope,
    #[serde(default)]
    action: Action,
    request: Value,
}

struct Prepared {
    body: Vec<u8>,
    token: Option<String>,
    generation: Option<u64>,
}
fn prepare(args: &Args) -> Result<Prepared> {
    ensure!(
        args.request.is_object(),
        "Broken sources request must be an object"
    );
    let raw = serde_json::to_vec(&args.request)?;
    let (body, token, generation, cap) = match args.scope {
        Scope::PageLinks => {
            ensure!(
                raw.len() <= crate::broken_sources::REQUEST_BYTES,
                "Broken sources request exceeds4096 bytes"
            );
            let request: crate::broken_sources::Request =
                serde_json::from_value(args.request.clone())?;
            request
                .validate()
                .map_err(|e| anyhow::anyhow!("{}: {}", e.code, e.message))?;
            if matches!(args.action, Action::Csv) {
                ensure!(
                    request.expected_snapshot_token.is_some() && request.cursor.is_none(),
                    "CSV requires preview token and no continuation"
                );
            }
            (
                serde_json::to_vec(&request)?,
                request.expected_snapshot_token,
                request.expected_generation,
                crate::broken_sources::REQUEST_BYTES,
            )
        }
        Scope::Resources => {
            ensure!(
                raw.len() <= crate::resource_broken_sources_app::REQUEST_BYTES,
                "Resource sources request exceeds4096 bytes"
            );
            let request: crate::resource_broken_sources_app::Request =
                serde_json::from_value(args.request.clone())?;
            ensure!(
                (1..=200).contains(&request.limit.unwrap_or(100)),
                "Resource sources page limit must be1..=200"
            );
            ensure!(
                request
                    .expected_snapshot_token
                    .as_ref()
                    .is_none_or(|v| v.len() <= 256),
                "Resource sources token exceeds256 bytes"
            );
            ensure!(
                request.after_sequence.is_none() || request.expected_snapshot_token.is_some(),
                "Continuation requires preview token"
            );
            if matches!(args.action, Action::Csv) {
                ensure!(
                    request.expected_snapshot_token.is_some() && request.after_sequence.is_none(),
                    "CSV requires preview token and no continuation"
                );
            }
            // The HTTP service knows the real generation and applies its existing
            // request.core / limits / SQLite binding checks; do not invent one here.
            (
                serde_json::to_vec(&request)?,
                request.expected_snapshot_token,
                request.expected_generation,
                crate::resource_broken_sources_app::REQUEST_BYTES,
            )
        }
    };
    ensure!(
        body.len() <= cap,
        "Serialized broken sources request exceeds4096 bytes"
    );
    Ok(Prepared {
        body,
        token,
        generation,
    })
}

fn single_header(response: &reqwest::Response, key: &str) -> Result<Option<String>> {
    let all = response.headers().get_all(key);
    let mut values = all.iter();
    let first = values
        .next()
        .map(|v| v.to_str().map(str::to_owned))
        .transpose()?;
    ensure!(
        values.next().is_none(),
        "Duplicate broken sources response header {key}"
    );
    Ok(first)
}
fn response_length(value: Option<&str>, required: bool) -> Result<Option<usize>> {
    if let Some(value) = value {
        ensure!(
            !value.is_empty()
                && value.bytes().all(|v| v.is_ascii_digit())
                && (value == "0" || !value.starts_with('0')),
            "Invalid broken sources response length"
        );
        let length: usize = value.parse()?;
        ensure!(
            length <= BODY_BYTES,
            "Broken sources body exceeds128KiB; use smaller pages or offline CLI CSV"
        );
        Ok(Some(length))
    } else {
        ensure!(!required, "Broken sources response length missing");
        Ok(None)
    }
}
fn media_type(value: &str, expected: &str) -> bool {
    value
        .split(';')
        .next()
        .is_some_and(|v| v.trim().eq_ignore_ascii_case(expected))
}

pub(super) async fn call(bridge: &Bridge, value: Value) -> Result<Value> {
    let args: Args = serde_json::from_value(value)?;
    let prepared = prepare(&args)?;
    let csv = matches!(args.action, Action::Csv);
    let resource = matches!(args.scope, Scope::Resources);
    let suffix = if csv { "csv" } else { "preview" };
    let response = bridge
        .client
        .post(bridge.base.join(&format!(
            "api/crawls/{}/{}/{suffix}",
            args.id,
            args.scope.route()
        ))?)
        .header("content-type", "application/json")
        .header("accept-encoding", "identity")
        .body(prepared.body)
        .send()
        .await?;
    let status = response.status();
    let declared = single_header(&response, "content-length")?;
    let expected = single_header(&response, "x-content-sha256")?;
    let snapshot = single_header(&response, "x-resource-source-snapshot")?;
    let content_type = single_header(&response, "content-type")?.unwrap_or_default();
    let length = response_length(
        declared.as_deref(),
        status.is_success() && (csv || resource),
    )?;
    let mut stream = response.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        ensure!(
            chunk.len() <= BODY_BYTES.saturating_sub(bytes.len()),
            "Broken sources body exceeds128KiB; use smaller pages or offline CLI CSV"
        );
        bytes.extend_from_slice(&chunk);
    }
    ensure!(
        length.is_none_or(|length| length == bytes.len()),
        "Broken sources length differs from complete EOF body"
    );
    ensure!(
        status.is_success(),
        "Broken sources service refused HTTP{}: {}",
        status.as_u16(),
        String::from_utf8_lossy(&bytes[..bytes.len().min(2048)])
    );
    let digest = crate::resource_broken_sources_app::bytes_sha(&bytes);
    // Legacy page-link JSON is bounded EOF JSON, with no required server digest.
    // If a checksum is sent, verify it; never turn a locally computed digest into
    // an assertion that this older service attested one.
    if csv || resource || expected.is_some() {
        ensure!(
            expected.as_deref() == Some(digest.as_str()),
            "Broken sources response digest missing/mismatched"
        );
    }
    if csv {
        ensure!(
            media_type(&content_type, "text/csv"),
            "Wrong broken sources CSV response type"
        );
        if resource {
            ensure!(
                snapshot.as_ref() == prepared.token.as_ref(),
                "Resource CSV belongs to another snapshot"
            );
        }
        use base64::Engine as _;
        Ok(
            json!({"scope":args.scope.contract(),"content_type":"text/csv","snapshot_token":prepared.token,
            "snapshot_binding":if resource { "http_response_snapshot_header" } else { "existing_http_service_request_validation" },
            "website_requests":0,"bytes":bytes.len(),"sha256":digest,
            "base64":base64::engine::general_purpose::STANDARD.encode(&bytes)}),
        )
    } else {
        ensure!(
            media_type(&content_type, "application/json"),
            "Wrong broken sources JSON response type"
        );
        let preview: Value = serde_json::from_slice(&bytes)
            .context("Broken sources response is not complete JSON")?;
        ensure!(
            preview["scope"].as_str() == Some(args.scope.contract())
                && preview["website_requests"].as_u64() == Some(0),
            "Unexpected broken sources response scope"
        );
        let (job_id, generation) = if resource {
            ensure!(
                preview["page"]["scope"].as_str() == Some(args.scope.contract())
                    && preview["page"]["website_requests"].as_u64() == Some(0),
                "Unexpected nested resource sources scope"
            );
            (&preview["job_id"], &preview["generation"])
        } else {
            (
                &preview["snapshot"]["job_id"],
                &preview["snapshot"]["app_generation"],
            )
        };
        ensure!(
            job_id.as_str().and_then(|v| uuid::Uuid::parse_str(v).ok()) == Some(args.id),
            "Broken sources preview belongs to another crawl"
        );
        ensure!(
            generation.as_u64().is_some()
                && prepared
                    .generation
                    .is_none_or(|g| generation.as_u64() == Some(g)),
            "Broken sources preview generation differs"
        );
        if let Some(token) = prepared.token {
            ensure!(
                preview["snapshot_token"].as_str() == Some(token.as_str()),
                "Broken sources preview belongs to another snapshot"
            );
        }
        Ok(preview)
    }
}

#[cfg(test)]
#[path = "broken_sources_tests.rs"]
mod tests;
