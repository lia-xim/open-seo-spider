//! Presentation of saved automated axe observations, never a WCAG certification.
use super::*;
use anyhow::{Result, ensure};
use std::io::{self, Write};

const CSV_BYTES: usize = 64 * 1024 * 1024;
const PAGE_BYTES: usize = 4 * 1024 * 1024;
const HEADERS: [&str; 33] = [
    "URL",
    "Rendered URL",
    "Rendered Status",
    "Rendered At",
    "Browser Version",
    "Viewport JSON",
    "Engine",
    "Axe Version",
    "Automated Only",
    "Observation Status",
    "Unknown Reason",
    "Violation Rules",
    "Needs Review Rules",
    "Pass Rules",
    "Inapplicable Rules",
    "Record Kind",
    "Rule ID",
    "Impact",
    "Description",
    "Help",
    "Help URL",
    "Tags JSON",
    "Node Index",
    "Target JSON",
    "HTML",
    "Failure Summary",
    "Checks JSON",
    "Observation SHA256",
    "Observation JSON",
    "Captured Pages",
    "Visible Pages",
    "Hidden Pages",
    "Scope",
];

fn text(value: Option<&Value>) -> &str {
    value.and_then(Value::as_str).unwrap_or("")
}
fn count(value: Option<&Value>) -> Option<u64> {
    value
        .and_then(Value::as_u64)
        .filter(|n| *n <= 9_007_199_254_740_991)
}
fn impact(value: Option<&Value>) -> bool {
    value.is_some_and(|v| {
        v.is_null()
            || matches!(
                v.as_str(),
                Some("minor" | "moderate" | "serious" | "critical")
            )
    })
}
fn target(value: &Value, depth: usize) -> bool {
    depth <= 8
        && match value {
            Value::String(_) => true,
            Value::Array(values) => {
                !values.is_empty() && values.iter().all(|v| target(v, depth + 1))
            }
            _ => false,
        }
}
fn rule(value: &Value) -> bool {
    value.is_object()
        && ["id", "description", "help", "helpUrl"]
            .iter()
            .all(|key| value.get(*key).is_some_and(Value::is_string))
        && !text(value.get("id")).is_empty()
        && impact(value.get("impact"))
        && value
            .get("tags")
            .and_then(Value::as_array)
            .is_some_and(|tags| tags.iter().all(Value::is_string))
        && value
            .get("nodes")
            .and_then(Value::as_array)
            .is_some_and(|nodes| {
                !nodes.is_empty()
                    && nodes.iter().all(|node| {
                        node.is_object()
                            && node.get("html").is_some_and(Value::is_string)
                            && node
                                .get("target")
                                .is_some_and(|v| v.is_array() && target(v, 0))
                            && node
                                .get("failureSummary")
                                .is_none_or(|v| v.is_null() || v.is_string())
                            && ["any", "all", "none"].iter().all(|key| {
                                node.get(*key)
                                    .and_then(Value::as_array)
                                    .is_some_and(|checks| {
                                        checks.iter().all(|check| {
                                            check.is_object()
                                                && check.get("id").is_some_and(Value::is_string)
                                                && impact(check.get("impact"))
                                                && check
                                                    .get("message")
                                                    .is_none_or(|v| v.is_null() || v.is_string())
                                        })
                                    })
                            })
                    })
            })
}

struct Observation<'a> {
    status: &'static str,
    reason: &'static str,
    violations: Option<&'a Vec<Value>>,
    incomplete: Option<&'a Vec<Value>>,
    passes: Option<u64>,
    inapplicable: Option<u64>,
}
fn observation(page: &PageRecord) -> Observation<'_> {
    let unknown = |status, reason| Observation {
        status,
        reason,
        violations: None,
        incomplete: None,
        passes: None,
        inapplicable: None,
    };
    let Some(raw) = &page.accessibility else {
        return unknown(
            if page.renderer_error.is_some() {
                "renderer_error"
            } else {
                "missing"
            },
            "no_saved_axe_observation",
        );
    };
    if !raw.is_object() {
        return unknown("malformed", "invalid_axe_observation");
    }
    if raw.get("error").is_some_and(|v| !v.is_null()) {
        return unknown("error", "saved_axe_error");
    }
    let violations = raw.get("violations").and_then(Value::as_array);
    let incomplete = raw.get("incomplete").and_then(Value::as_array);
    let passes = count(raw.get("passes_count"));
    let inapplicable = count(raw.get("inapplicable_count"));
    if text(raw.get("engine")) != "axe-core"
        || text(raw.get("version")).is_empty()
        || raw.get("automated_only") != Some(&Value::Bool(true))
        || passes.is_none()
        || inapplicable.is_none()
        || violations.is_none_or(|rows| !rows.iter().all(rule))
        || incomplete.is_none_or(|rows| !rows.iter().all(rule))
    {
        return unknown("malformed", "invalid_axe_observation");
    }
    let status = if !incomplete.unwrap().is_empty() {
        "needs_review"
    } else if !violations.unwrap().is_empty() {
        "violations"
    } else {
        "observed_zero"
    };
    Observation {
        status,
        reason: "",
        violations,
        incomplete,
        passes,
        inapplicable,
    }
}

struct Budget {
    bytes: Vec<u8>,
    cap: usize,
}
impl Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > self.cap)
        {
            return Err(io::Error::other("accessibility CSV byte budget exceeded"));
        }
        let required = self.bytes.len() + bytes.len();
        if required > self.bytes.capacity() {
            let capacity = self
                .bytes
                .capacity()
                .saturating_mul(2)
                .max(4096)
                .max(required)
                .min(self.cap);
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(|_| io::Error::other("accessibility CSV allocation budget exceeded"))?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
struct Counter {
    bytes: usize,
    cap: usize,
}
impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| io::Error::other("accessibility CSV page JSON budget exceeded"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn row(output: &mut impl Write, cells: &[String]) -> Result<()> {
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            output.write_all(b",")?;
        }
        output.write_all(b"\"")?;
        let safe = open_seo_spider_core::csv_text(cell);
        for (part_index, part) in safe.split('"').enumerate() {
            if part_index > 0 {
                output.write_all(b"\"\"")?;
            }
            output.write_all(part.as_bytes())?;
        }
        output.write_all(b"\"")?;
    }
    output.write_all(b"\r\n")?;
    Ok(())
}
fn json(value: Option<&Value>) -> Result<String> {
    value
        .map(serde_json::to_string)
        .transpose()
        .map(|v| v.unwrap_or_default())
        .map_err(Into::into)
}
fn number(value: Option<u64>) -> String {
    value.map(|n| n.to_string()).unwrap_or_default()
}
fn export(report: &CrawlReport, cap: usize, page_cap: usize) -> Result<Vec<u8>> {
    let output = Budget {
        bytes: Vec::new(),
        cap,
    };
    Ok(write_csv(report, output, page_cap, &[], None)?.bytes)
}

/// Cohort selection is already applied by the shared segment engine. Keep the
/// original captured/profile counts distinct from the selected-page columns.
pub(super) fn write_cohort_csv<W: Write>(
    report: &CrawlReport,
    output: W,
    extra: &[(String, String)],
    captured: usize,
    visible: usize,
) -> Result<W> {
    write_csv(report, output, PAGE_BYTES, extra, Some((captured, visible)))
}

fn write_csv<W: Write>(
    report: &CrawlReport,
    mut output: W,
    page_cap: usize,
    extra: &[(String, String)],
    counts: Option<(usize, usize)>,
) -> Result<W> {
    let mut headers = HEADERS.map(str::to_owned).to_vec();
    for (name, _) in extra {
        ensure!(
            !headers.contains(name),
            "Duplicate accessibility CSV context column"
        );
        headers.push(name.clone());
    }
    row(&mut output, &headers)?;
    let (captured, visible) = counts.unwrap_or_else(|| {
        (
            report.pages.len(),
            report
                .pages
                .iter()
                .filter(|p| open_seo_spider_core::directive_page_visible(&report.config, p))
                .count(),
        )
    });
    ensure!(
        visible <= captured,
        "Invalid accessibility captured/profile counts"
    );
    for page in report
        .pages
        .iter()
        .filter(|p| open_seo_spider_core::directive_page_visible(&report.config, p))
    {
        // Measure before copying any imported observation or metadata strings.
        serde_json::to_writer(
            &mut Counter {
                bytes: 0,
                cap: page_cap,
            },
            &(
                &page.url,
                &page.rendered_url,
                &page.rendered_status,
                &page.rendered_at,
                &page.browser_version,
                &page.rendered_viewport,
                &page.renderer_error,
                &page.accessibility,
            ),
        )?;
        let raw = page.accessibility.as_ref();
        let parsed = observation(page);
        let raw_json = json(raw)?;
        let digest = raw
            .map(|_| open_seo_spider_core::api_enrichment::api_sha(raw_json.as_bytes()))
            .unwrap_or_default();
        let mut base = vec![
            page.url.clone(),
            page.rendered_url.clone().unwrap_or_default(),
            page.rendered_status
                .map(|n| n.to_string())
                .unwrap_or_default(),
            page.rendered_at.clone().unwrap_or_default(),
            page.browser_version.clone().unwrap_or_default(),
            json(page.rendered_viewport.as_ref())?,
            text(raw.and_then(|v| v.get("engine"))).into(),
            text(raw.and_then(|v| v.get("version"))).into(),
            raw.and_then(|v| v.get("automated_only"))
                .and_then(Value::as_bool)
                .map(|v| v.to_string())
                .unwrap_or_default(),
            parsed.status.into(),
            parsed.reason.into(),
            parsed
                .violations
                .map(|v| v.len().to_string())
                .unwrap_or_default(),
            parsed
                .incomplete
                .map(|v| v.len().to_string())
                .unwrap_or_default(),
            number(parsed.passes),
            number(parsed.inapplicable),
        ];
        base.resize(33, String::new());
        base[15] = "page_summary".into();
        base[27] = digest;
        base[28] = raw_json;
        base[29] = captured.to_string();
        base[30] = visible.to_string();
        base[31] = (captured - visible).to_string();
        base[32] = "automated_only; saved_observations_not_a_WCAG_compliance_claim".into();
        base.extend(extra.iter().map(|(_, value)| value.clone()));
        row(&mut output, &base)?;
        base[28].clear();
        for (kind, rules) in [
            ("violation", parsed.violations),
            ("needs_review", parsed.incomplete),
        ] {
            for rule in rules.into_iter().flatten() {
                for (index, node) in rule["nodes"].as_array().unwrap().iter().enumerate() {
                    let mut cells = base.clone();
                    cells[15] = kind.into();
                    for (column, key) in [
                        (16, "id"),
                        (17, "impact"),
                        (18, "description"),
                        (19, "help"),
                        (20, "helpUrl"),
                    ] {
                        cells[column] = text(rule.get(key)).into();
                    }
                    cells[21] = json(rule.get("tags"))?;
                    cells[22] = index.to_string();
                    cells[23] = json(node.get("target"))?;
                    cells[24] = text(node.get("html")).into();
                    cells[25] = text(node.get("failureSummary")).into();
                    cells[26] = serde_json::to_string(
                        &json!({"any":node["any"],"all":node["all"],"none":node["none"]}),
                    )?;
                    row(&mut output, &cells)?;
                }
            }
        }
    }
    Ok(output)
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportQuery {
    generation: Option<u64>,
}
pub(super) async fn csv(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<ExportQuery>,
) -> ApiResult<Response> {
    let job = get_job(&state, id).await?;
    let (bytes, generation) = state
        .lifecycle
        .blocking
        .run_blocking(move || -> Result<_> {
            let job = job.blocking_read();
            ensure!(
                !matches!(job.status.as_str(), "running" | "pausing" | "cancelling"),
                "A stable accessibility snapshot is required"
            );
            ensure!(
                query.generation.is_none_or(|g| g == job.generation),
                "Stale accessibility generation"
            );
            let report = job
                .report
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("A stable accessibility snapshot is required"))?;
            Ok((export(report, CSV_BYTES, PAGE_BYTES)?, job.generation))
        })
        .await
        .map_err(|e| api_error(StatusCode::SERVICE_UNAVAILABLE, e))?
        .map_err(|e| {
            let status = if e.to_string().contains("stable accessibility")
                || e.to_string().contains("Stale accessibility")
            {
                StatusCode::CONFLICT
            } else {
                StatusCode::PAYLOAD_TOO_LARGE
            };
            api_error(status, e)
        })?;
    let hash = open_seo_spider_core::api_enrichment::api_sha(&bytes);
    Response::builder()
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"accessibility.csv\"",
        )
        .header(header::CONTENT_LENGTH, bytes.len())
        .header(header::CACHE_CONTROL, "no-store")
        .header("x-content-sha256", hash)
        .header("x-accessibility-generation", generation)
        .body(axum::body::Body::from(bytes))
        .map_err(|e| api_error(StatusCode::INTERNAL_SERVER_ERROR, e))
}

#[cfg(test)]
#[path = "accessibility_tests.rs"]
mod tests;
