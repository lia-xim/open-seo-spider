//! One action-discriminated MCP adapter to the same loopback application API.
use super::Bridge;
use anyhow::{Result,ensure};
use base64::Engine as _;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value,json};
pub(super) const NAME:&str="spider_api_enrichment";
pub(super) fn definition()->Value{json!({"name":NAME,"description":"Preview/start a bounded PageSpeed-only enrichment of supplied URLs, or status/read/cancel/export an existing API-only job. Our application does not crawl supplied websites; Google PageSpeed may fetch them remotely. Explicit start can send provider requests. Keys are resolved only from a selected server credential profile, never tool arguments.","inputSchema":{"type":"object","additionalProperties":false,"required":["action"],"properties":{"action":{"enum":["preview","start","status","configuration","read","csv","cancel"]},"id":{"type":"string","format":"uuid"},"input":{"type":"object"},"request":{"type":"object"}}},"annotations":{"readOnlyHint":false,"destructiveHint":false,"idempotentHint":false,"openWorldHint":true}})}
#[derive(Clone,Copy,Deserialize)]
#[serde(rename_all="snake_case")]
enum Action{Preview,Start,Status,Configuration,Read,Csv,Cancel}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args{action:Action,id:Option<uuid::Uuid>,input:Option<crate::enrichment_app::StartRequest>,request:Option<crate::enrichment_app::ReadRequest>}
pub(super) async fn call(bridge:&Bridge,args:Value)->Result<Value>{
    let a:Args=serde_json::from_value(args)?;let (route,body)=match a.action{
        Action::Preview|Action::Start=>{ensure!(a.id.is_none()&&a.request.is_none(),"preview/start accepts input only");let input=a.input.ok_or_else(||anyhow::anyhow!("input required"))?;let route=if matches!(a.action,Action::Preview){"api/enrichments/preview"}else{"api/enrichments"};(route.into(),Some(serde_json::to_value(input)?))},
        Action::Status|Action::Configuration|Action::Cancel=>{ensure!(a.input.is_none()&&a.request.is_none(),"status/configuration/cancel accepts id only");let id=a.id.ok_or_else(||anyhow::anyhow!("id required"))?;(format!("api/enrichments/{id}{}",match a.action{Action::Cancel=>"/cancel",Action::Configuration=>"/configuration",_=>""}),if matches!(a.action,Action::Cancel){Some(json!({}))}else{None})},
        Action::Read|Action::Csv=>{ensure!(a.input.is_none(),"read/csv does not accept new input");let id=a.id.ok_or_else(||anyhow::anyhow!("id required"))?;let request=a.request.unwrap_or_default();ensure!(request.limit<=200,"MCP row page exceeds200");(format!("api/enrichments/{id}/{}",if matches!(a.action,Action::Csv){"csv"}else{"results"}),Some(serde_json::to_value(request)?))}
    };
    let url=bridge.base.join(&route)?;let response=if let Some(body)=body{bridge.client.post(url).json(&body).send().await?}else{bridge.client.get(url).send().await?};let status=response.status();let content=response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|x|x.to_str().ok()).unwrap_or("").to_owned();let expected=response.headers().get("x-content-sha256").and_then(|x|x.to_str().ok()).map(str::to_owned);let mut stream=response.bytes_stream();let mut bytes=Vec::new();while let Some(chunk)=stream.next().await{let c=chunk?;ensure!(bytes.len().checked_add(c.len()).is_some_and(|n|n<=1024*1024),"MCP API-only result exceeds1MiB; use smaller JSON pages or explicit CLI full export");bytes.extend_from_slice(&c);}
    ensure!(status.is_success(),"API-only operation refused with HTTP{}",status.as_u16());let hash=open_seo_spider_core::api_enrichment::api_sha(&bytes);if let Some(h)=expected{ensure!(h==hash,"API-only response SHA mismatch");}
    if content.starts_with("text/csv"){Ok(json!({"content_type":"text/csv","sha256":hash,"bytes":bytes.len(),"base64":base64::engine::general_purpose::STANDARD.encode(&bytes),"application_website_requests":0}))}else{Ok(serde_json::from_slice(&bytes)?)}
}
#[cfg(test)]
#[path="enrichment_tests.rs"]
mod tests;
