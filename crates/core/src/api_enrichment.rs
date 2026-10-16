//! API-only supplied-URL observations. Never constructs a crawler or PageRecord.
use crate::{ListImportOptions, ListImportResult, import_url_list};
use anyhow::{Result, ensure};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
    sync::Arc,
    time::Duration,
};
use tokio::sync::watch;

pub const API_ENRICHMENT_ENGINE: &str = "open-seo-spider/api-only-pagespeed-v1";
pub const API_REPORT_BYTES: usize = 64 * 1024 * 1024;
pub const API_INPUT_BYTES: usize = 8 * 1024 * 1024;
pub const PAGESPEED_ENDPOINT: &str = "https://www.googleapis.com/pagespeedonline/v5/runPagespeed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageSpeedStrategy {
    Mobile,
    Desktop,
}
impl PageSpeedStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mobile => "mobile",
            Self::Desktop => "desktop",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiCredentialProfile {
    Anonymous,
    PageSpeedEnvironment,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiEnrichmentOptions {
    pub version: u32,
    pub strategy: PageSpeedStrategy,
    pub credential_profile: ApiCredentialProfile,
    pub max_provider_calls: usize,
    pub max_response_bytes: usize,
    pub timeout_ms: u64,
    pub max_job_ms: u64,
    pub retries: u8,
}
impl Default for ApiEnrichmentOptions {
    fn default() -> Self {
        Self {
            version: 1,
            strategy: PageSpeedStrategy::Mobile,
            credential_profile: ApiCredentialProfile::Anonymous,
            max_provider_calls: 50_000,
            max_response_bytes: 8 * 1024 * 1024,
            timeout_ms: 60_000,
            max_job_ms: 900_000,
            retries: 1,
        }
    }
}
impl ApiEnrichmentOptions {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.version == 1, "Unsupported API-only options version");
        ensure!(
            (1..=100_000).contains(&self.max_provider_calls),
            "Provider call budget must be 1..=100000"
        );
        ensure!(
            (1..=32 * 1024 * 1024).contains(&self.max_response_bytes),
            "Provider byte budget must be 1..=32MiB"
        );
        ensure!(
            (100..=120_000).contains(&self.timeout_ms),
            "Provider timeout must be 100..=120000ms"
        );
        ensure!(
            (100..=3_600_000).contains(&self.max_job_ms),
            "Job deadline must be 100..=3600000ms"
        );
        ensure!(
            self.retries <= 2,
            "At most two explicit transient retries are supported"
        );
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiEnrichmentPlan {
    pub engine: String,
    pub input: ListImportResult,
    pub options: ApiEnrichmentOptions,
    pub unique_urls: Vec<String>,
    pub fingerprint: String,
    pub application_website_requests: usize,
    pub provider_may_fetch_submitted_site: bool,
}
#[derive(Clone)]
pub struct ApiCancellation(Arc<watch::Sender<bool>>);
impl Default for ApiCancellation {
    fn default() -> Self {
        Self(Arc::new(watch::channel(false).0))
    }
}
impl ApiCancellation {
    pub fn cancel(&self) {
        self.0.send_replace(true);
    }
    pub fn is_cancelled(&self) -> bool {
        *self.0.borrow()
    }
    pub async fn cancelled(&self) {
        let mut rx = self.0.subscribe();
        loop {
            if *rx.borrow_and_update() {
                return;
            }
            if rx.changed().await.is_err() {
                return;
            }
        }
    }
}
pub fn api_sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn prepare_api_enrichment(
    bytes: &[u8],
    import: &ListImportOptions,
    options: &ApiEnrichmentOptions,
) -> Result<ApiEnrichmentPlan> {
    ensure!(bytes.len() <= API_INPUT_BYTES, "API-only input exceeds8MiB");
    options.validate()?;
    let input = import_url_list(bytes, import)?;
    let mut seen = BTreeSet::new();
    let mut urls = Vec::new();
    for row in &input.rows {
        if let Some(value) = &row.normalized_url {
            ensure!(value.len() <= 8192, "API URL byte budget exceeded");
            let u = url::Url::parse(value)?;
            ensure!(
                u.username().is_empty() && u.password().is_none(),
                "Supplied URLs must not contain credentials"
            );
            if seen.insert(value.clone()) {
                urls.push(value.clone());
            }
        }
    }
    ensure!(!urls.is_empty(), "No accepted supplied HTTP/HTTPS URLs");
    ensure!(
        urls.len() <= options.max_provider_calls,
        "Unique URLs exceed provider call budget"
    );
    let fingerprint = api_sha(&bounded_api_json(
        &(API_ENRICHMENT_ENGINE, &input, options, &urls),
        API_REPORT_BYTES,
    )?);
    Ok(ApiEnrichmentPlan {
        engine: API_ENRICHMENT_ENGINE.into(),
        input,
        options: options.clone(),
        unique_urls: urls,
        fingerprint,
        application_website_requests: 0,
        provider_may_fetch_submitted_site: true,
    })
}
struct Capped {
    data: Vec<u8>,
    max: usize,
}
impl Write for Capped {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self
            .data
            .len()
            .checked_add(b.len())
            .is_none_or(|n| n > self.max)
        {
            return Err(io::Error::other("API-only output byte budget exceeded"));
        }
        self.data.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub fn bounded_api_json<T: Serialize>(value: &T, max: usize) -> Result<Vec<u8>> {
    let mut w = Capped {
        data: Vec::new(),
        max,
    };
    serde_json::to_writer(&mut w, value)?;
    Ok(w.data)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiOutcome {
    Success,
    Unavailable,
    Failed,
    Cancelled,
    BudgetExceeded,
}
impl ApiOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Unavailable => "unavailable",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::BudgetExceeded => "budget_exceeded",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiFailureCode {
    Network,
    Timeout,
    Authentication,
    RateLimited,
    ProviderHttp,
    LighthouseRuntime,
    InvalidJson,
    InvalidSchema,
    ResponseBudget,
    JobDeadline,
    CallBudget,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageSpeedMetrics {
    pub provider_requested_url: Option<String>,
    pub provider_final_url: Option<String>,
    pub provider_fetch_time: Option<String>,
    pub lighthouse_version: Option<String>,
    pub performance_score: Option<f64>,
    pub first_contentful_paint_ms: Option<f64>,
    pub largest_contentful_paint_ms: Option<f64>,
    pub total_blocking_time_ms: Option<f64>,
    pub cumulative_layout_shift: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiObservation {
    pub requested_url: String,
    pub outcome: ApiOutcome,
    pub failure: Option<ApiFailureCode>,
    pub api_http_status: Option<u16>,
    pub provider_request_attempts: usize,
    pub response_bytes: usize,
    pub response_sha256: Option<String>,
    pub metrics: Option<PageSpeedMetrics>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiEnrichmentReport {
    pub engine: String,
    pub plan: ApiEnrichmentPlan,
    pub observations: Vec<ApiObservation>,
    pub provider_request_attempts: usize,
    pub application_website_requests: usize,
    pub started_at: String,
    pub finished_at: String,
    pub outcome: String,
}
// This request has no Debug/Serialize implementation: credential bytes never become a report.
pub struct PageSpeedRequest<'a> {
    pub requested_url: &'a str,
    pub strategy: PageSpeedStrategy,
    pub timeout_ms: u64,
    pub max_response_bytes: usize,
    pub key: Option<&'a str>,
}
pub struct PageSpeedReply {
    pub status: u16,
    pub bytes: Vec<u8>,
    pub retry_after_secs: Option<u64>,
}
pub trait PageSpeedTransport: Send + Sync {
    fn execute<'a>(
        &'a self,
        request: PageSpeedRequest<'a>,
        cancel: ApiCancellation,
    ) -> BoxFuture<'a, std::result::Result<PageSpeedReply, ApiFailureCode>>;
}
mod pagespeed;
pub use pagespeed::PageSpeedHttp;
fn optional_text(value: &serde_json::Value, key: &str, max: usize) -> Result<Option<String>> {
    let Some(v) = value.get(key) else {
        return Ok(None);
    };
    if v.is_null() {
        return Ok(None);
    };
    let s = v
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid provider text"))?;
    ensure!(
        s.len() <= max && !s.chars().any(char::is_control),
        "Invalid bounded provider text"
    );
    Ok(Some(s.into()))
}
fn metric(value: &serde_json::Value, pointer: &str, score: bool) -> Result<Option<f64>> {
    let Some(v) = value.pointer(pointer) else {
        return Ok(None);
    };
    if v.is_null() {
        return Ok(None);
    };
    let n = v
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("Invalid provider metric"))?;
    ensure!(
        n.is_finite() && n >= 0.0 && (!score || n <= 1.0),
        "Invalid provider metric range"
    );
    Ok(Some(n))
}
enum ParsedMetrics {
    Observed(PageSpeedMetrics),
    Unavailable,
    RuntimeFailure,
}
fn metrics(bytes: &[u8]) -> Result<ParsedMetrics> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    ensure!(value.is_object(), "Provider result must be an object");
    let Some(l) = value.get("lighthouseResult").filter(|v| !v.is_null()) else {
        return Ok(ParsedMetrics::Unavailable);
    };
    ensure!(l.is_object(), "Invalid Lighthouse object");
    // Do not persist raw provider runtimeError messages or key material. HTTP200 is not Lighthouse success.
    if let Some(error) = l.get("runtimeError").filter(|v| !v.is_null()) {
        ensure!(error.is_object(), "Invalid Lighthouse runtime error");
        return Ok(ParsedMetrics::RuntimeFailure);
    }
    let requested = optional_text(l, "requestedUrl", 8192)?;
    let final_url = optional_text(l, "finalUrl", 8192)?;
    for s in [&requested, &final_url].into_iter().flatten() {
        let u = url::Url::parse(s)?;
        ensure!(
            matches!(u.scheme(), "http" | "https")
                && u.username().is_empty()
                && u.password().is_none(),
            "Invalid provider URL"
        );
    }
    let observed = PageSpeedMetrics {
        provider_requested_url: requested,
        provider_final_url: final_url,
        provider_fetch_time: optional_text(l, "fetchTime", 128)?,
        lighthouse_version: optional_text(l, "lighthouseVersion", 128)?,
        performance_score: metric(l, "/categories/performance/score", true)?,
        first_contentful_paint_ms: metric(l, "/audits/first-contentful-paint/numericValue", false)?,
        largest_contentful_paint_ms: metric(
            l,
            "/audits/largest-contentful-paint/numericValue",
            false,
        )?,
        total_blocking_time_ms: metric(l, "/audits/total-blocking-time/numericValue", false)?,
        cumulative_layout_shift: metric(l, "/audits/cumulative-layout-shift/numericValue", false)?,
    };
    if [
        observed.performance_score,
        observed.first_contentful_paint_ms,
        observed.largest_contentful_paint_ms,
        observed.total_blocking_time_ms,
        observed.cumulative_layout_shift,
    ]
    .iter()
    .all(Option::is_none)
    {
        Ok(ParsedMetrics::Unavailable)
    } else {
        Ok(ParsedMetrics::Observed(observed))
    }
}
fn empty(url: &str) -> ApiObservation {
    ApiObservation {
        requested_url: url.into(),
        outcome: ApiOutcome::Failed,
        failure: None,
        api_http_status: None,
        provider_request_attempts: 0,
        response_bytes: 0,
        response_sha256: None,
        metrics: None,
    }
}
fn validate_plan(plan: &ApiEnrichmentPlan) -> Result<()> {
    plan.options.validate()?;
    ensure!(
        plan.engine == API_ENRICHMENT_ENGINE
            && plan.input.engine == crate::list_import::LIST_IMPORT_ENGINE,
        "Unsupported API-only plan"
    );
    crate::validate_import_result(&plan.input)?;
    ensure!(
        plan.application_website_requests == 0
            && plan.input.website_requests == 0
            && plan.provider_may_fetch_submitted_site,
        "Invalid API-only observation scope"
    );
    ensure!(
        plan.input.input_bytes <= API_INPUT_BYTES && plan.input.rows.len() <= 50_000,
        "API-only input budget exceeded"
    );
    let mut seen = BTreeSet::new();
    let mut ordered = Vec::new();
    for row in &plan.input.rows {
        if let Some(value) = &row.normalized_url {
            ensure!(value.len() <= 8192, "API URL byte budget exceeded");
            let u = url::Url::parse(value)?;
            ensure!(
                matches!(u.scheme(), "http" | "https")
                    && u.username().is_empty()
                    && u.password().is_none(),
                "Invalid supplied URL"
            );
            if seen.insert(value.clone()) {
                ordered.push(value.clone());
            }
        }
    }
    ensure!(
        !ordered.is_empty()
            && ordered == plan.unique_urls
            && ordered.len() <= plan.options.max_provider_calls,
        "Supplied URL order/dedup authority mismatch"
    );
    let expected = api_sha(&bounded_api_json(
        &(
            API_ENRICHMENT_ENGINE,
            &plan.input,
            &plan.options,
            &plan.unique_urls,
        ),
        API_REPORT_BYTES,
    )?);
    ensure!(
        expected == plan.fingerprint,
        "API input/options fingerprint mismatch"
    );
    Ok(())
}
pub async fn run_api_enrichment<T: PageSpeedTransport>(
    plan: ApiEnrichmentPlan,
    transport: &T,
    key: Option<&str>,
    cancel: ApiCancellation,
) -> Result<ApiEnrichmentReport> {
    validate_plan(&plan)?;
    ensure!(
        matches!(
            (plan.options.credential_profile, key),
            (ApiCredentialProfile::Anonymous, None)
                | (ApiCredentialProfile::PageSpeedEnvironment, Some(_))
        ),
        "Credential profile does not match the explicit runtime credential"
    );
    if let Some(k) = key {
        ensure!(
            !k.is_empty() && k.len() <= 4096 && k.bytes().all(|b| b.is_ascii_graphic()),
            "Invalid credential profile value"
        );
    }
    let started_at = chrono::Utc::now().to_rfc3339();
    let deadline = tokio::time::Instant::now() + Duration::from_millis(plan.options.max_job_ms);
    let mut all = Vec::new();
    let mut calls = 0usize;
    let mut observation_bytes = 0usize;
    for url in &plan.unique_urls {
        let mut item = empty(url);
        for attempt in 0..=plan.options.retries {
            if cancel.is_cancelled() {
                item.outcome = ApiOutcome::Cancelled;
                item.failure = Some(ApiFailureCode::Cancelled);
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                item.failure = Some(ApiFailureCode::JobDeadline);
                break;
            }
            if calls >= plan.options.max_provider_calls {
                item.outcome = ApiOutcome::BudgetExceeded;
                item.failure = Some(ApiFailureCode::CallBudget);
                break;
            }
            calls = calls
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("Provider counter overflow"))?;
            item.provider_request_attempts += 1;
            let request = PageSpeedRequest {
                requested_url: url,
                strategy: plan.options.strategy,
                timeout_ms: plan.options.timeout_ms,
                max_response_bytes: plan.options.max_response_bytes,
                key,
            };
            let reply = tokio::select! {biased;_=cancel.cancelled()=>Err(ApiFailureCode::Cancelled),r=tokio::time::timeout_at(deadline,transport.execute(request,cancel.clone()))=>r.unwrap_or(Err(ApiFailureCode::JobDeadline))};
            let mut retry = None;
            match reply {
                Err(code) => {
                    item.failure = Some(code);
                    if code == ApiFailureCode::Cancelled {
                        item.outcome = ApiOutcome::Cancelled;
                    }
                    if code == ApiFailureCode::ResponseBudget {
                        item.outcome = ApiOutcome::BudgetExceeded;
                    }
                    if matches!(code, ApiFailureCode::Network | ApiFailureCode::Timeout) {
                        retry = Some(1);
                    }
                }
                Ok(reply) => {
                    item.api_http_status = Some(reply.status);
                    item.response_bytes = reply.bytes.len();
                    item.response_sha256 = Some(api_sha(&reply.bytes));
                    if reply.bytes.len() > plan.options.max_response_bytes {
                        item.outcome = ApiOutcome::BudgetExceeded;
                        item.failure = Some(ApiFailureCode::ResponseBudget);
                    } else if reply.status == 429 {
                        item.failure = Some(ApiFailureCode::RateLimited);
                        retry = Some(reply.retry_after_secs.unwrap_or(1).min(30));
                    } else if reply.status >= 500 {
                        item.failure = Some(ApiFailureCode::ProviderHttp);
                        retry = Some(1);
                    } else if matches!(reply.status, 401 | 403) {
                        item.failure = Some(ApiFailureCode::Authentication);
                    } else if reply.status != 200 {
                        item.failure = Some(ApiFailureCode::ProviderHttp);
                    } else {
                        match metrics(&reply.bytes) {
                            Ok(ParsedMetrics::Observed(m)) => {
                                item.metrics = Some(m);
                                item.outcome = ApiOutcome::Success;
                                item.failure = None;
                            }
                            Ok(ParsedMetrics::Unavailable) => {
                                item.outcome = ApiOutcome::Unavailable;
                                item.failure = None;
                            }
                            Ok(ParsedMetrics::RuntimeFailure) => {
                                item.outcome = ApiOutcome::Failed;
                                item.failure = Some(ApiFailureCode::LighthouseRuntime);
                            }
                            Err(_) => item.failure = Some(ApiFailureCode::InvalidSchema),
                        };
                    }
                }
            }
            let Some(seconds) = retry.filter(|_| attempt < plan.options.retries) else {
                break;
            };
            tokio::select! {biased;_=cancel.cancelled()=>{item.outcome=ApiOutcome::Cancelled;item.failure=Some(ApiFailureCode::Cancelled);break;},_=tokio::time::sleep_until((tokio::time::Instant::now()+Duration::from_secs(seconds)).min(deadline))=>{}}
        }
        observation_bytes = observation_bytes
            .checked_add(bounded_api_json(&item, 65536)?.len())
            .filter(|n| *n <= 32 * 1024 * 1024)
            .ok_or_else(|| anyhow::anyhow!("Logical API observation budget exceeded"))?;
        all.push(item);
    }
    let outcome = if cancel.is_cancelled() {
        "cancelled"
    } else if all
        .iter()
        .all(|i| matches!(i.outcome, ApiOutcome::Success | ApiOutcome::Unavailable))
    {
        "complete"
    } else {
        "partial"
    };
    let report = ApiEnrichmentReport {
        engine: API_ENRICHMENT_ENGINE.into(),
        plan,
        observations: all,
        provider_request_attempts: calls,
        application_website_requests: 0,
        started_at,
        finished_at: chrono::Utc::now().to_rfc3339(),
        outcome: outcome.into(),
    };
    bounded_api_json(&report, API_REPORT_BYTES)?;
    Ok(report)
}
pub fn validate_api_report(report: &ApiEnrichmentReport) -> Result<()> {
    ensure!(
        report.engine == API_ENRICHMENT_ENGINE,
        "Unsupported API report"
    );
    validate_plan(&report.plan)?;
    ensure!(
        report.application_website_requests == 0,
        "Invalid API-only scope"
    );
    ensure!(
        matches!(
            report.outcome.as_str(),
            "complete" | "partial" | "cancelled"
        ),
        "Invalid API-only report status"
    );
    let _started = chrono::DateTime::parse_from_rfc3339(&report.started_at)?;
    let _finished = chrono::DateTime::parse_from_rfc3339(&report.finished_at)?;
    ensure!(
        report.observations.len() == report.plan.unique_urls.len(),
        "Incomplete API observation array"
    );
    let mut calls = 0usize;
    for (url, row) in report.plan.unique_urls.iter().zip(&report.observations) {
        ensure!(url == &row.requested_url, "API key order mismatch");
        calls = calls
            .checked_add(row.provider_request_attempts)
            .ok_or_else(|| anyhow::anyhow!("API counter overflow"))?;
        ensure!(
            row.provider_request_attempts <= usize::from(report.plan.options.retries) + 1,
            "Invalid provider attempt count"
        );
        ensure!(
            row.outcome != ApiOutcome::Success || row.metrics.is_some(),
            "Success needs typed provider metrics"
        );
    }
    ensure!(
        calls == report.provider_request_attempts
            && calls <= report.plan.options.max_provider_calls,
        "Provider call budget mismatch"
    );
    bounded_api_json(report, API_REPORT_BYTES)?;
    Ok(())
}
fn safe_csv(s: &str) -> String {
    if s.starts_with('\t')
        || s.starts_with('\r')
        || matches!(s.trim_start().chars().next(), Some('=' | '+' | '-' | '@'))
    {
        format!("'{s}")
    } else {
        s.into()
    }
}
pub fn export_api_enrichment_csv(report: &ApiEnrichmentReport, out: &mut impl Write) -> Result<()> {
    struct Budget<'a, W: Write> {
        out: &'a mut W,
        written: usize,
    }
    impl<W: Write> Write for Budget<'_, W> {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            if self
                .written
                .checked_add(b.len())
                .is_none_or(|n| n > API_REPORT_BYTES)
            {
                return Err(io::Error::other("API-only CSV byte budget exceeded"));
            }
            let n = self.out.write(b)?;
            self.written += n;
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.out.flush()
        }
    }
    validate_api_report(report)?;
    let mut csv = csv::Writer::from_writer(Budget { out, written: 0 });
    csv.write_record([
        "source_index",
        "source_row",
        "raw_value",
        "normalized_url",
        "input_error",
        "provider",
        "outcome",
        "api_http_status",
        "provider_request_attempts",
        "performance_score",
        "lcp_ms",
        "provider_final_url",
    ])?;
    let by_url = report
        .observations
        .iter()
        .map(|r| (&r.requested_url, r))
        .collect::<BTreeMap<_, _>>();
    for row in &report.plan.input.rows {
        let result = row.normalized_url.as_ref().and_then(|u| by_url.get(u));
        csv.write_record([
            row.source_index.to_string(),
            row.source_row.to_string(),
            safe_csv(&row.raw_value),
            safe_csv(row.normalized_url.as_deref().unwrap_or("")),
            safe_csv(row.error.as_deref().unwrap_or("")),
            "pagespeed_lighthouse_v1".into(),
            result
                .map(|r| r.outcome.as_str().into())
                .unwrap_or_else(|| "input_rejected".into()),
            result
                .and_then(|r| r.api_http_status)
                .map(|n| n.to_string())
                .unwrap_or_default(),
            result
                .map(|r| r.provider_request_attempts.to_string())
                .unwrap_or_default(),
            result
                .and_then(|r| r.metrics.as_ref())
                .and_then(|m| m.performance_score)
                .map(|n| n.to_string())
                .unwrap_or_default(),
            result
                .and_then(|r| r.metrics.as_ref())
                .and_then(|m| m.largest_contentful_paint_ms)
                .map(|n| n.to_string())
                .unwrap_or_default(),
            safe_csv(
                result
                    .and_then(|r| r.metrics.as_ref())
                    .and_then(|m| m.provider_final_url.as_deref())
                    .unwrap_or(""),
            ),
        ])?;
    }
    csv.flush()?;
    Ok(())
}
#[cfg(test)]
#[path = "api_enrichment_tests.rs"]
mod tests;
