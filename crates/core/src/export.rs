use crate::{CrawlReport, PageChange};
use std::collections::{BTreeMap, BTreeSet};

/// Original list occurrences joined to captured request identities, without fetching.
pub fn export_input_occurrences_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "Source List",
        "Input SHA256",
        "Provenance",
        "Source Index",
        "Source Row",
        "Source Line",
        "Sheet",
        "Cell",
        "Raw Value",
        "Cell Type",
        "Formula Metadata",
        "Candidate Kind",
        "Normalized URL",
        "Fragment",
        "Input Error",
        "Observation",
        "Status",
        "Final Status",
        "Final URL",
        "Title",
        "Crawl Error",
    ])?;
    let mut captured = std::collections::HashMap::new();
    for page in &report.pages {
        captured.entry(page.url.as_str()).or_insert(page);
    }
    for (list_index, input) in report.config.input_imports.iter().enumerate() {
        crate::validate_import_result(input)?;
        for row in &input.rows {
            let page = row
                .normalized_url
                .as_deref()
                .and_then(|url| captured.get(url).copied());
            let observation = if row.error.is_some() {
                "invalid_input"
            } else if row.candidate_kind == crate::ListCandidateKind::SitemapDocument {
                "sitemap_document"
            } else if page.is_some_and(|p| p.blocked_by_robots) {
                "blocked_by_robots"
            } else if page.is_some_and(|p| p.error.is_some()) {
                "request_error"
            } else if page.is_some() {
                "captured"
            } else {
                "unfetched"
            };
            let page = page.filter(|_| {
                row.error.is_none() && row.candidate_kind == crate::ListCandidateKind::PageUrl
            });
            writer.write_record([
                (list_index + 1).to_string(),
                input.input_sha256.clone(),
                "declared_client_input".into(),
                row.source_index.to_string(),
                row.source_row.to_string(),
                row.source_line.map(|n| n.to_string()).unwrap_or_default(),
                csv_text(row.sheet.as_deref().unwrap_or_default()),
                csv_text(row.cell.as_deref().unwrap_or_default()),
                csv_text(&row.raw_value),
                csv_text(&row.cell_type),
                csv_text(
                    &row.formula_metadata
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()?
                        .unwrap_or_default(),
                ),
                match row.candidate_kind {
                    crate::ListCandidateKind::PageUrl => "page_url",
                    crate::ListCandidateKind::SitemapDocument => "sitemap_document",
                }
                .into(),
                csv_text(row.normalized_url.as_deref().unwrap_or_default()),
                csv_text(row.fragment.as_deref().unwrap_or_default()),
                csv_text(row.error.as_deref().unwrap_or_default()),
                observation.into(),
                page.and_then(|p| p.status)
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                page.and_then(|p| p.final_status)
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                csv_text(page.map(|p| p.final_url.as_str()).unwrap_or_default()),
                csv_text(page.map(|p| p.title()).unwrap_or_default()),
                csv_text(page.and_then(|p| p.error.as_deref()).unwrap_or_default()),
            ])?;
        }
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

pub fn export_pages_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let raw_report = report;
    let visible_report = crate::reported_report(report);
    let report = visible_report.as_ref();
    let mut writer = csv::Writer::from_writer(Vec::new());
    let tls_columns = raw_report
        .config
        .tls
        .as_ref()
        .is_some_and(|tls| !tls.is_inactive())
        || raw_report.pages.iter().any(|page| page.tls.is_some());
    let mut headers = vec![
        "URL",
        "Final URL",
        "Status",
        "Final Status",
        "Content Type",
        "Title",
        "Description",
        "H1",
        "H2",
        "Canonical",
        "Indexable",
        "Indexability",
        "Depth",
        "Response ms",
        "Bytes",
        "Words",
        "Outlinks",
        "Images",
        "Error",
        "Rendered",
        "Raw Title",
        "Rendered URL",
        "Render Error",
        "Browser Requests",
        "Console Errors",
        "HTML Canonical",
        "HTTP Canonical",
        "Indexability Agent",
        "Effective Robots",
        "Pagination Next",
        "Pagination Prev",
        "AMPHTML",
        "Title Pixels",
        "Description Pixels",
        "Title Pixel Measurements",
        "Description Pixel Measurements",
        "Text Metrics Engine",
        "Crawl Coverage",
        "Observation Basis",
        "Selected State",
        "Parent Snapshot SHA256",
        "Fetched At",
    ];
    if tls_columns {
        headers.extend(["Local Transport Upgrade", "HSTS Upgrade Evidence"]);
    }
    writer.write_record(headers)?;
    let known_operation = report
        .operation
        .as_ref()
        .filter(|operation| operation.engine == crate::SELECTED_RECRAWL_ENGINE);
    let targets: std::collections::HashMap<_, _> = known_operation
        .map(|operation| {
            operation
                .targets
                .iter()
                .map(|target| (target.requested_url.as_str(), target))
                .collect()
        })
        .unwrap_or_default();
    for page in &report.pages {
        let target = targets.get(page.url.as_str());
        let selected_state = target
            .map(|target| match target.state {
                crate::RecrawlTargetState::Pending => "pending",
                crate::RecrawlTargetState::Committed => "committed",
            })
            .unwrap_or_default();
        let observation_basis = match (report.operation.as_ref(), known_operation, selected_state) {
            (None, _, _) => "captured",
            (Some(_), None, _) => "unknown_operation",
            (Some(_), Some(_), "committed") => "refreshed_in_selected_job",
            (Some(_), Some(_), "pending") => "selected_pending_not_refreshed",
            (Some(_), Some(_), _) => "inherited_from_parent",
        };
        let mut row = vec![
            csv_text(&page.url),
            csv_text(&page.final_url),
            page.status
                .map(|status| status.to_string())
                .unwrap_or_default(),
            page.final_status
                .map(|status| status.to_string())
                .unwrap_or_default(),
            csv_text(&page.content_type),
            csv_text(page.title()),
            csv_text(page.description()),
            csv_text(&page.h1.join(" | ")),
            csv_text(&page.h2.join(" | ")),
            csv_text(&page.canonical.join(" | ")),
            page.indexable.to_string(),
            csv_text(&page.indexability_reasons.join(" | ")),
            page.depth.to_string(),
            page.response_ms.to_string(),
            page.size_bytes.to_string(),
            page.word_count.to_string(),
            page.links.len().to_string(),
            page.images.len().to_string(),
            csv_text(page.error.as_deref().unwrap_or_default()),
            page.rendered.to_string(),
            csv_text(
                page.raw_metadata
                    .as_ref()
                    .and_then(|raw| raw.titles.first())
                    .map(String::as_str)
                    .unwrap_or_default(),
            ),
            csv_text(page.rendered_url.as_deref().unwrap_or_default()),
            csv_text(page.renderer_error.as_deref().unwrap_or_default()),
            page.network_requests.len().to_string(),
            page.console_errors.len().to_string(),
            csv_text(&page.html_canonical.join(" | ")),
            csv_text(&page.header_canonical.join(" | ")),
            if page.indexability_agent.is_empty() {
                "googlebot".into()
            } else {
                csv_text(&page.indexability_agent)
            },
            csv_text(&crate::applicable_robot_directives(page).join(" | ")),
            csv_text(&page.pagination_next.join(" | ")),
            csv_text(&page.pagination_prev.join(" | ")),
            csv_text(&page.amphtml.join(" | ")),
            page.title_pixel_widths
                .first()
                .and_then(|width| *width)
                .map(|width| width.to_string())
                .unwrap_or_default(),
            page.description_pixel_widths
                .first()
                .and_then(|width| *width)
                .map(|width| width.to_string())
                .unwrap_or_default(),
            if page.title_pixel_widths.is_empty() {
                String::new()
            } else {
                serde_json::to_string(&page.title_pixel_widths)?
            },
            if page.description_pixel_widths.is_empty() {
                String::new()
            } else {
                serde_json::to_string(&page.description_pixel_widths)?
            },
            csv_text(page.text_metrics_engine.as_deref().unwrap_or_default()),
            csv_text(
                report
                    .operation
                    .as_ref()
                    .map_or("crawl_snapshot", |operation| operation.coverage.as_str()),
            ),
            observation_basis.into(),
            selected_state.into(),
            csv_text(
                report
                    .operation
                    .as_ref()
                    .map_or("", |operation| operation.parent.snapshot_sha256.as_str()),
            ),
            csv_text(&page.fetched_at),
        ];
        if tls_columns {
            row.push(
                if page
                    .tls
                    .as_ref()
                    .is_some_and(|tls| !tls.hsts_upgrades.is_empty())
                {
                    "HSTS Policy 307".into()
                } else {
                    String::new()
                },
            );
            row.push(
                page.tls
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()?
                    .unwrap_or_default(),
            );
        }
        writer.write_record(row)?;
    }
    crate::reporting_csv(raw_report, String::from_utf8(writer.into_inner()?)?)
}

pub fn export_issues_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let raw_report = report;
    let visible_report = crate::reported_report(report);
    let report = visible_report.as_ref();
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(["Severity", "Code", "URL", "Message", "Related URLs"])?;
    for issue in &report.issues {
        writer.write_record([
            format!("{:?}", issue.severity),
            csv_text(&issue.code),
            csv_text(&issue.url),
            csv_text(&issue.message),
            csv_text(&issue.related_urls.join(" | ")),
        ])?;
    }
    crate::reporting_csv(raw_report, String::from_utf8(writer.into_inner()?)?)
}

pub fn export_links_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let raw_report = report;
    let visible_report = crate::reported_report(report);
    let report = visible_report.as_ref();
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "Source",
        "Target",
        "Anchor Text",
        "Kind",
        "Internal",
        "Rel",
        "Nofollow",
        "Raw Href",
        "Resolved Href",
        "Fragment",
        "DOM Source",
        "DOM Index",
        "DOM Path",
        "DOM Path Truncated",
        "Semantic Ancestors",
        "Position",
    ])?;
    for page in &report.pages {
        for link in &page.links {
            writer.write_record([
                csv_text(&page.url),
                csv_text(&link.url),
                csv_text(&link.text),
                csv_text(&link.kind),
                link.internal.to_string(),
                csv_text(&link.rel),
                link.nofollow.to_string(),
                csv_text(link.raw_href.as_deref().unwrap_or_default()),
                csv_text(link.resolved_href.as_deref().unwrap_or_default()),
                csv_text(link.fragment.as_deref().unwrap_or_default()),
                csv_text(
                    link.dom_location
                        .as_ref()
                        .and(page.dom_metadata.as_ref())
                        .map(|dom| dom.source.as_str())
                        .unwrap_or_default(),
                ),
                link.dom_location
                    .as_ref()
                    .map(|location| location.index.to_string())
                    .unwrap_or_default(),
                csv_text(
                    link.dom_location
                        .as_ref()
                        .and_then(|location| location.path.as_deref())
                        .unwrap_or_default(),
                ),
                link.dom_location
                    .as_ref()
                    .map(|location| location.path_truncated.to_string())
                    .unwrap_or_default(),
                csv_text(
                    &link
                        .dom_location
                        .as_ref()
                        .map(|location| location.semantic_ancestors.join(" | "))
                        .unwrap_or_default(),
                ),
                csv_text(link.position.as_deref().unwrap_or_default()),
            ])?;
        }
    }
    crate::reporting_csv(raw_report, String::from_utf8(writer.into_inner()?)?)
}

pub fn export_fragments_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "Source URL",
        "Source Document URL",
        "Target URL",
        "Resolved Href",
        "Fragment",
        "Decoded Fragment",
        "Status",
        "Reason",
        "Target Document URL",
        "Target Complete",
        "Source DOM JSON",
        "Matched Target JSON",
    ])?;
    for row in crate::reported_fragments(report) {
        let status = serde_json::to_value(row.status)?;
        writer.write_record([
            csv_text(&row.source_url),
            csv_text(&row.source_document_url),
            csv_text(&row.target_url),
            csv_text(&row.resolved_href),
            csv_text(&row.fragment),
            csv_text(row.decoded_fragment.as_deref().unwrap_or_default()),
            csv_text(status.as_str().unwrap_or_default()),
            csv_text(&row.reason),
            csv_text(row.target_document_url.as_deref().unwrap_or_default()),
            row.target_complete.to_string(),
            row.source_dom_location
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?
                .unwrap_or_default(),
            row.matched_target
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?
                .unwrap_or_default(),
        ])?;
    }
    crate::reporting_csv(report, String::from_utf8(writer.into_inner()?)?)
}

/// Original typed facts, with explicit legacy/availability records per snapshot.
pub fn export_dom_observations_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let raw_report = report;
    let visible_report = crate::reported_report(report);
    let report = visible_report.as_ref();
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "URL",
        "Snapshot Role",
        "Source",
        "Collection Complete",
        "Family",
        "DOM Index",
        "DOM Path",
        "DOM Path Truncated",
        "Details JSON",
    ])?;
    for page in &report.pages {
        let mut snapshots = vec![("effective", page.dom_metadata.as_ref())];
        if let Some(raw) = page.raw_metadata.as_ref() {
            snapshots.push(("native_predecessor", raw.dom_metadata.as_ref()));
        }
        for (role, dom) in snapshots {
            let source = dom.map(|value| value.source.as_str()).unwrap_or_default();
            let complete = dom
                .map(|value| value.complete.to_string())
                .unwrap_or_default();
            let mut fact = |family: &str,
                            location: Option<&crate::DomLocation>,
                            value: serde_json::Value|
             -> anyhow::Result<()> {
                writer.write_record([
                    csv_text(&page.url),
                    role.into(),
                    csv_text(source),
                    complete.clone(),
                    family.into(),
                    location
                        .map(|value| value.index.to_string())
                        .unwrap_or_default(),
                    csv_text(
                        location
                            .and_then(|value| value.path.as_deref())
                            .unwrap_or_default(),
                    ),
                    location
                        .map(|value| value.path_truncated.to_string())
                        .unwrap_or_default(),
                    serde_json::to_string(&value)?,
                ])?;
                Ok(())
            };
            fact(
                "availability",
                None,
                serde_json::json!({"available":dom.is_some(),"engine":dom.map(|value|value.engine.as_str()),"limits_reached":dom.map(|value|&value.limits_reached)}),
            )?;
            let Some(dom) = dom else { continue };
            for value in &dom.headings {
                fact(
                    "heading",
                    Some(&value.dom_location),
                    serde_json::to_value(value)?,
                )?;
            }
            for value in &dom.meta_keywords {
                fact(
                    "meta_keywords",
                    Some(&value.dom_location),
                    serde_json::to_value(value)?,
                )?;
            }
            for value in &dom.fragment_targets {
                fact(
                    "fragment_target",
                    Some(&value.dom_location),
                    serde_json::to_value(value)?,
                )?;
            }
            for value in &dom.navigation_candidates {
                fact(
                    "navigation_candidate",
                    Some(&value.dom_location),
                    serde_json::to_value(value)?,
                )?;
            }
            for value in &dom.responsive_images {
                fact(
                    "responsive_image",
                    Some(&value.dom_location),
                    serde_json::to_value(value)?,
                )?;
            }
        }
    }
    crate::reporting_csv(raw_report, String::from_utf8(writer.into_inner()?)?)
}

/// One result per row preserves values containing commas, quotes or delimiters.
pub fn export_custom_extractions_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let raw_report = report;
    let visible_report = crate::reported_report(report);
    let report = visible_report.as_ref();
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "URL",
        "Name",
        "Selector",
        "Attribute",
        "Source",
        "Match Count",
        "Match Index",
        "Value",
        "Method",
        "Engine",
        "Result Type",
        "Observed Match Count",
        "Truncated",
        "Error",
        "Output Mode",
    ])?;
    for page in &report.pages {
        for extraction in &page.custom_extractions {
            let rows = if extraction.values.is_empty() {
                vec![(String::new(), String::new())]
            } else {
                extraction
                    .values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| ((index + 1).to_string(), value.clone()))
                    .collect()
            };
            for (index, value) in rows {
                writer.write_record([
                    csv_text(&page.url),
                    csv_text(&extraction.name),
                    csv_text(&extraction.selector),
                    csv_text(extraction.attribute.as_deref().unwrap_or_default()),
                    csv_text(&extraction.source),
                    extraction.values.len().to_string(),
                    index,
                    if extraction.result_type == "number" && value.parse::<f64>().is_ok() {
                        value
                    } else {
                        csv_text(&value)
                    },
                    match extraction.method {
                        crate::ExtractionMethod::Css => "css",
                        crate::ExtractionMethod::XPath => "xpath",
                        crate::ExtractionMethod::Regex => "regex",
                    }
                    .into(),
                    csv_text(&extraction.engine),
                    csv_text(&extraction.result_type),
                    extraction
                        .matched_count
                        .map(|count| count.to_string())
                        .unwrap_or_default(),
                    extraction.truncated.to_string(),
                    csv_text(extraction.error.as_deref().unwrap_or_default()),
                    match extraction.output {
                        crate::ExtractionOutput::Auto => "auto",
                        crate::ExtractionOutput::Text => "text",
                        crate::ExtractionOutput::InnerHtml => "inner_html",
                        crate::ExtractionOutput::OuterHtml => "outer_html",
                        crate::ExtractionOutput::FunctionValue => "function_value",
                    }
                    .into(),
                ])?;
            }
        }
    }
    crate::reporting_csv(raw_report, String::from_utf8(writer.into_inner()?)?)
}

/// One row per executed script. JSON preserves the difference between a missing
/// result, null, a string and structured data, including embedded CSV delimiters.
pub fn export_custom_javascript_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let raw_report = report;
    let visible_report = crate::reported_report(report);
    let report = visible_report.as_ref();
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "URL",
        "Script Name",
        "Value Type",
        "Value JSON",
        "Error",
        "Elapsed ms",
    ])?;
    for page in &report.pages {
        for result in &page.custom_javascript_results {
            writer.write_record([
                csv_text(&page.url),
                scalar_json_field(result, "name"),
                scalar_json_field(result, "value_type"),
                result
                    .get("value")
                    .map(serde_json::to_string)
                    .transpose()?
                    .unwrap_or_default(),
                scalar_json_field(result, "error"),
                scalar_json_field(result, "elapsed_ms"),
            ])?;
        }
    }
    crate::reporting_csv(raw_report, String::from_utf8(writer.into_inner()?)?)
}

/// Captured cookie attributes only. An allowlist deliberately omits cookie
/// values and all unknown fields, including in imported legacy reports.
pub fn export_browser_cookies_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let raw_report = report;
    let visible_report = crate::reported_report(report);
    let report = visible_report.as_ref();
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "URL",
        "Name",
        "Domain",
        "Path",
        "Expires",
        "HTTP Only",
        "Secure",
        "Same Site",
        "Value Length",
    ])?;
    for page in &report.pages {
        for cookie in &page.browser_cookies {
            writer.write_record([
                csv_text(&page.url),
                scalar_json_field(cookie, "name"),
                scalar_json_field(cookie, "domain"),
                scalar_json_field(cookie, "path"),
                scalar_json_field(cookie, "expires"),
                scalar_json_field(cookie, "httpOnly"),
                scalar_json_field(cookie, "secure"),
                scalar_json_field(cookie, "sameSite"),
                cookie
                    .get("value_length")
                    .and_then(serde_json::Value::as_u64)
                    .map(|length| length.to_string())
                    .unwrap_or_default(),
            ])?;
        }
    }
    crate::reporting_csv(raw_report, String::from_utf8(writer.into_inner()?)?)
}

/// Native PDF observations. Unknown numbers remain empty cells, while parse
/// errors, unsupported extraction and exhausted limits stay explicit.
pub fn export_pdf_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let raw_report = report;
    let visible_report = crate::reported_report(report);
    let report = visible_report.as_ref();
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "URL",
        "Final URL",
        "HTTP Status",
        "Analysis Status",
        "Engine",
        "PDF Version",
        "Encrypted",
        "Page Count",
        "Declared Page Count",
        "Text Pages Processed",
        "Text Characters",
        "Word Count",
        "Text Complete",
        "Annotations Complete",
        "Info JSON",
        "Text Preview",
        "Text Hash",
        "Page Text JSON",
        "Annotation Links JSON",
        "Warnings JSON",
        "Errors JSON",
        "Parse Error",
        "Limits Reached JSON",
    ])?;
    for page in &report.pages {
        let Some(pdf) = &page.pdf else { continue };
        writer.write_record([
            csv_text(&page.url),
            csv_text(&page.final_url),
            page.final_status.map(|v| v.to_string()).unwrap_or_default(),
            resource_status(pdf.status),
            csv_text(&pdf.engine),
            csv_text(pdf.version.as_deref().unwrap_or_default()),
            pdf.encrypted.map(|v| v.to_string()).unwrap_or_default(),
            pdf.page_count.map(|v| v.to_string()).unwrap_or_default(),
            pdf.declared_page_count
                .map(|v| v.to_string())
                .unwrap_or_default(),
            pdf.text_pages_processed.to_string(),
            pdf.text_chars.map(|v| v.to_string()).unwrap_or_default(),
            pdf.word_count.map(|v| v.to_string()).unwrap_or_default(),
            pdf.text_complete.to_string(),
            pdf.annotations_complete.to_string(),
            serde_json::to_string(&pdf.info_metadata)?,
            csv_text(pdf.text_preview.as_deref().unwrap_or_default()),
            csv_text(pdf.text_hash.as_deref().unwrap_or_default()),
            serde_json::to_string(&pdf.text_pages)?,
            serde_json::to_string(&pdf.annotation_links)?,
            serde_json::to_string(&pdf.warnings)?,
            serde_json::to_string(&pdf.errors)?,
            csv_text(pdf.parse_error.as_deref().unwrap_or_default()),
            serde_json::to_string(&pdf.limits_reached)?,
        ])?;
    }
    crate::reporting_csv(raw_report, String::from_utf8(writer.into_inner()?)?)
}

/// Fetched image resource metadata, separate from declared img width/height/alt.
pub fn export_image_resources_csv(report: &CrawlReport) -> anyhow::Result<String> {
    let raw_report = report;
    let visible_report = crate::reported_report(report);
    let report = visible_report.as_ref();
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "URL",
        "Final URL",
        "HTTP Status",
        "Analysis Status",
        "Engine",
        "Format",
        "Width",
        "Height",
        "Dimension Unit",
        "Encoded Bytes",
        "Body Complete",
        "SVG ViewBox JSON",
        "Warnings JSON",
        "Parse Error",
    ])?;
    for page in &report.pages {
        let Some(image) = &page.image_metadata else {
            continue;
        };
        writer.write_record([
            csv_text(&page.url),
            csv_text(&page.final_url),
            page.final_status.map(|v| v.to_string()).unwrap_or_default(),
            resource_status(image.status),
            csv_text(&image.engine),
            csv_text(image.format.as_deref().unwrap_or_default()),
            image.width.map(|v| v.to_string()).unwrap_or_default(),
            image.height.map(|v| v.to_string()).unwrap_or_default(),
            csv_text(image.dimension_unit.as_deref().unwrap_or_default()),
            image.encoded_bytes.to_string(),
            image.body_complete.to_string(),
            serde_json::to_string(&image.svg_view_box)?,
            serde_json::to_string(&image.warnings)?,
            csv_text(image.parse_error.as_deref().unwrap_or_default()),
        ])?;
    }
    crate::reporting_csv(raw_report, String::from_utf8(writer.into_inner()?)?)
}

fn resource_status(status: crate::ResourceAnalysisStatus) -> String {
    match status {
        crate::ResourceAnalysisStatus::Complete => "complete",
        crate::ResourceAnalysisStatus::Partial => "partial",
        crate::ResourceAnalysisStatus::Error => "error",
        crate::ResourceAnalysisStatus::Encrypted => "encrypted",
        crate::ResourceAnalysisStatus::Disabled => "disabled",
    }
    .into()
}

fn scalar_json_field(value: &serde_json::Value, field: &str) -> String {
    match value.get(field) {
        Some(serde_json::Value::String(value)) => csv_text(value),
        Some(value @ (serde_json::Value::Number(_) | serde_json::Value::Bool(_))) => {
            value.to_string()
        }
        _ => String::new(),
    }
}

/// Spreadsheet-oriented CSV prefixes formula-like text with an apostrophe.
/// Leading Unicode whitespace, tabs, CR/LF and BOM cannot hide a formula prefix.
/// Numeric/boolean columns bypass this helper; JSON project exports keep the
/// original strings exactly, including any leading apostrophe or whitespace.
pub fn csv_text(value: &str) -> String {
    let significant = value
        .trim_start_matches(|character: char| character.is_whitespace() || character == '\u{feff}');
    if matches!(significant.chars().next(), Some('=' | '+' | '-' | '@')) {
        format!("'{value}")
    } else {
        value.into()
    }
}

pub fn export_sitemap_xml(report: &CrawlReport) -> String {
    let counts = crate::reporting_visibility(&report.config, &report.pages, &report.issues);
    let visible_report = crate::reported_report(report);
    let report = visible_report.as_ref();
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    if let Some(counts) = counts {
        xml.push_str(&format!(
            "<!-- Captured Pages: {}; Visible Pages: {}; Hidden Pages: {} -->\n",
            counts.captured_pages, counts.visible_pages, counts.hidden_pages
        ));
    }
    for page in report
        .pages
        .iter()
        .filter(|page| page.internal && page.indexable)
    {
        xml.push_str(&format!(
            "  <url><loc>{}</loc></url>\n",
            quick_xml::escape::escape(&page.url)
        ));
    }
    xml.push_str("</urlset>\n");
    xml
}

pub fn compare_crawls(before: &CrawlReport, after: &CrawlReport) -> Vec<PageChange> {
    let old: BTreeMap<_, _> = before.pages.iter().map(|page| (&page.url, page)).collect();
    let new: BTreeMap<_, _> = after.pages.iter().map(|page| (&page.url, page)).collect();
    let keys: BTreeSet<_> = old.keys().chain(new.keys()).copied().collect();
    let mut changes = Vec::new();
    for url in keys {
        match (old.get(url), new.get(url)) {
            (None, Some(_)) => changes.push(PageChange {
                url: url.clone(),
                kind: "added".into(),
                fields: vec![],
            }),
            (Some(_), None) => changes.push(PageChange {
                url: url.clone(),
                kind: "removed".into(),
                fields: vec![],
            }),
            (Some(a), Some(b)) => {
                let mut fields = Vec::new();
                macro_rules! changed { ($($field:ident),+) => { $(if a.$field != b.$field { fields.push(stringify!($field).to_owned()); })+ }; }
                changed!(
                    status,
                    final_status,
                    final_url,
                    titles,
                    meta_descriptions,
                    h1,
                    h2,
                    canonical,
                    meta_robots,
                    x_robots_tag,
                    indexable,
                    content_hash,
                    word_count,
                    content_type,
                    error,
                    blocked_by_robots,
                    rendered,
                    renderer_error,
                    rendered_url,
                    rendered_status
                );
                if !fields.is_empty() {
                    changes.push(PageChange {
                        url: url.clone(),
                        kind: "changed".into(),
                        fields,
                    });
                }
            }
            _ => {}
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CrawlConfig, CrawlSummary, PageRecord};
    use serde_json::json;

    fn report(pages: Vec<PageRecord>) -> CrawlReport {
        CrawlReport {
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
            sitemap_catalog: None,
            sitemap_coverage_inventory: None,
            sitemap_source_measurements: None,
            config: CrawlConfig::default(),
            pages,
            issues: vec![],
            summary: CrawlSummary::default(),
            warnings: vec![],
        }
    }

    fn read_csv(text: &str) -> (Vec<String>, Vec<Vec<String>>) {
        let mut reader = csv::Reader::from_reader(text.as_bytes());
        let headers = reader
            .headers()
            .unwrap()
            .iter()
            .map(str::to_owned)
            .collect();
        let rows = reader
            .records()
            .map(|record| record.unwrap().iter().map(str::to_owned).collect())
            .collect();
        (headers, rows)
    }

    #[test]
    fn link_dom_csv_keeps_legacy_columns_and_unknowns_separate_from_zero() {
        let mut page = PageRecord {
            url: "https://fixture.test/source".into(),
            ..Default::default()
        };
        page.links = vec![
            crate::LinkRecord {
                url: "https://fixture.test/target".into(),
                kind: "anchor".into(),
                raw_href: Some(" \t=SUM(1,2)".into()),
                fragment: Some("=fragment".into()),
                position: Some("@custom".into()),
                dom_location: Some(crate::DomLocation {
                    index: 0,
                    path: Some("html/body/a[1]".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            crate::LinkRecord {
                url: "https://fixture.test/legacy".into(),
                kind: "anchor".into(),
                ..Default::default()
            },
        ];
        page.dom_metadata = Some(crate::DomMetadata {
            source: "raw_html".into(),
            complete: true,
            ..Default::default()
        });
        let (headers, rows) = read_csv(&export_links_csv(&report(vec![page])).unwrap());
        assert_eq!(
            &headers[..7],
            &[
                "Source",
                "Target",
                "Anchor Text",
                "Kind",
                "Internal",
                "Rel",
                "Nofollow"
            ]
        );
        assert_eq!(rows[0][7], "' \t=SUM(1,2)");
        assert_eq!(rows[0][9], "'=fragment");
        assert_eq!(rows[0][11], "0");
        assert_eq!(rows[0][15], "'@custom");
        assert!(rows[1][7..].iter().all(String::is_empty));
    }

    #[test]
    fn dom_csv_preserves_raw_rendered_facts_and_exact_json_with_legacy_availability() {
        let text = " \t=SUM(1,2), \"Über\"\n<img src=x>";
        let dom = crate::DomMetadata {
            source: "rendered_html".into(),
            complete: true,
            headings: vec![crate::HeadingRecord {
                tag_level: 2,
                text: text.into(),
                dom_location: crate::DomLocation {
                    index: 0,
                    path: Some("html/body/h2[1]".into()),
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut raw_dom = dom.clone();
        raw_dom.source = "raw_html".into();
        raw_dom.headings[0].text = "Original".into();
        let page = PageRecord {
            url: "https://fixture.test/observed".into(),
            dom_metadata: Some(dom),
            raw_metadata: Some(crate::RawMetadata {
                dom_metadata: Some(raw_dom),
                ..Default::default()
            }),
            ..Default::default()
        };
        let legacy = PageRecord {
            url: "https://fixture.test/legacy".into(),
            ..Default::default()
        };
        let (headers, rows) =
            read_csv(&export_dom_observations_csv(&report(vec![page, legacy])).unwrap());
        assert_eq!(headers.len(), 9);
        let heading = rows
            .iter()
            .find(|row| row[1] == "effective" && row[4] == "heading")
            .unwrap();
        assert_eq!(heading[5], "0");
        assert_eq!(heading[3], "true");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&heading[8]).unwrap()["text"],
            text
        );
        assert!(rows.iter().any(|row| row[1] == "native_predecessor"
            && row[2] == "raw_html"
            && row[4] == "heading"));
        let legacy = rows.iter().find(|row| row[0].ends_with("/legacy")).unwrap();
        assert_eq!(legacy[4], "availability");
        assert_eq!(legacy[3], "");
        assert_eq!(legacy[5], "");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&legacy[8]).unwrap()["available"],
            false
        );
    }

    #[test]
    fn fragment_csv_preserves_occurrences_and_unobserved_targets_are_unknown() {
        let url = "https://fixture.test/source";
        let mut page = PageRecord {
            url: url.into(),
            final_url: url.into(),
            status: Some(200),
            final_status: Some(200),
            content_type: "text/html".into(),
            internal: true,
            ..Default::default()
        };
        crate::extract_html(
            &mut page,
            "<h1 id='here'>Here</h1><a href='#here'>Found</a><a href='#gone'>Missing</a><a href='/absent#id'>Unobserved</a>",
            &[url::Url::parse(url).unwrap()],
            false,
        );
        let mut snapshot = report(vec![page]);
        snapshot.config.validate_fragments = true;
        let (headers, rows) = read_csv(&export_fragments_csv(&snapshot).unwrap());
        assert_eq!(headers.len(), 12);
        assert_eq!(rows.len(), 3);
        assert!(
            rows.iter()
                .any(|row| row[4] == "here" && row[6] == "matched_id" && !row[11].is_empty())
        );
        assert!(
            rows.iter()
                .any(|row| row[4] == "gone" && row[6] == "missing" && row[9] == "true")
        );
        assert!(
            rows.iter().any(|row| row[4] == "id"
                && row[6] == "unknown"
                && row[7] == "target_not_observed")
        );
        snapshot.config.validate_fragments = false;
        assert!(
            read_csv(&export_fragments_csv(&snapshot).unwrap())
                .1
                .is_empty()
        );
    }

    #[test]
    fn javascript_csv_roundtrips_json_types_errors_and_csv_delimiters() {
        let url = "https://fixture.test/a?query=one,two";
        let values = [
            json!(null),
            json!(false),
            json!(0),
            json!("comma, quote\"\nand newline\nUTF-8 ä"),
            json!([1, "two"]),
            json!({"title":"value,quoted\"", "nested":{"enabled":true}}),
        ];
        let page = PageRecord {url: url.into(), custom_javascript_results: values.iter().enumerate().map(|(index, value)| {
            json!({"name":format!("script,{index}\nname"),"value":value,"value_type":"observed_type","error":null,"elapsed_ms":0})
        }).chain(std::iter::once(json!({"name":"failure", "value":null, "value_type":null,
            "error":"fixture, error\"\nnext line", "elapsed_ms":17}))).collect(), ..Default::default()};
        let (headers, rows) = read_csv(&export_custom_javascript_csv(&report(vec![page])).unwrap());
        assert_eq!(
            headers,
            [
                "URL",
                "Script Name",
                "Value Type",
                "Value JSON",
                "Error",
                "Elapsed ms"
            ]
        );
        assert_eq!(rows.len(), 7);
        for (index, value) in values.iter().enumerate() {
            assert_eq!(rows[index][0], url);
            assert_eq!(rows[index][1], format!("script,{index}\nname"));
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&rows[index][3]).unwrap(),
                *value
            );
            assert_eq!(rows[index][4], "");
            assert_eq!(rows[index][5], "0");
        }
        assert_eq!(rows[6][2], "");
        assert_eq!(rows[6][3], "null");
        assert_eq!(rows[6][4], "fixture, error\"\nnext line");
        assert_eq!(rows[6][5], "17");
    }

    #[test]
    fn javascript_csv_missing_value_is_unknown_and_not_an_inferred_null() {
        let page = PageRecord {
            url: "https://fixture.test/a".into(),
            custom_javascript_results: vec![
                json!({"name":"legacy", "error":"value not captured"}),
                json!({"name":"null", "value":null, "value_type":"null"}),
            ],
            ..Default::default()
        };
        let (_, rows) = read_csv(&export_custom_javascript_csv(&report(vec![page])).unwrap());
        assert_eq!(rows[0][2], "");
        assert_eq!(rows[0][3], "");
        assert_eq!(rows[0][5], "");
        assert_eq!(rows[1][2], "null");
        assert_eq!(rows[1][3], "null");
    }

    #[test]
    fn cookie_csv_uses_only_attribute_allowlist_even_for_imported_cookie_values() {
        let page = PageRecord {
            url: "https://fixture.test/a".into(),
            browser_cookies: vec![
                json!({"name":"session,one", "domain":".fixture.test", "path":"/a,b", "expires":-1,
                "httpOnly":true,"secure":false,"sameSite":"Lax","value_length":24,
                "value":"DO_NOT_EXPORT_COOKIE_VALUE", "unknown":{"value":"DO_NOT_EXPORT_UNKNOWN_FIELD"}}),
                json!({"name":"utf8", "domain":"fixture.test", "path":"/", "expires":1700000000.25,
                "httpOnly":false,"secure":true,"sameSite":"Strict","value_length":0}),
                json!({"name":"legacy", "value":"DO_NOT_DERIVE_LENGTH_FROM_VALUE"}),
            ],
            ..Default::default()
        };
        let exported = export_browser_cookies_csv(&report(vec![page])).unwrap();
        assert!(!exported.contains("DO_NOT_"));
        let (headers, rows) = read_csv(&exported);
        assert_eq!(
            headers,
            [
                "URL",
                "Name",
                "Domain",
                "Path",
                "Expires",
                "HTTP Only",
                "Secure",
                "Same Site",
                "Value Length"
            ]
        );
        assert_eq!(rows[0][1], "session,one");
        assert_eq!(rows[0][3], "/a,b");
        assert_eq!(&rows[0][4..], ["-1", "true", "false", "Lax", "24"]);
        assert_eq!(
            &rows[1][4..],
            ["1700000000.25", "false", "true", "Strict", "0"]
        );
        assert_eq!(rows[2][8], "");
    }

    #[test]
    fn pages_csv_appends_provenance_without_moving_existing_columns() {
        let title = "Title, \"quoted\"\nsecond line";
        let page = PageRecord {
            url: "https://fixture.test/a".into(),
            titles: vec![title.into()],
            canonical: vec!["https://fixture.test/canonical".into()],
            html_canonical: vec!["https://fixture.test/html".into()],
            header_canonical: vec!["https://fixture.test/http".into()],
            indexability_agent: "googlebot".into(),
            x_robots_tag: vec!["bingbot: noindex, googlebot: nofollow".into()],
            pagination_next: vec!["https://fixture.test/next".into()],
            pagination_prev: vec!["https://fixture.test/prev".into()],
            amphtml: vec!["https://fixture.test/amp".into()],
            ..Default::default()
        };
        let (headers, rows) = read_csv(&export_pages_csv(&report(vec![page])).unwrap());
        assert_eq!(
            &headers[..25],
            [
                "URL",
                "Final URL",
                "Status",
                "Final Status",
                "Content Type",
                "Title",
                "Description",
                "H1",
                "H2",
                "Canonical",
                "Indexable",
                "Indexability",
                "Depth",
                "Response ms",
                "Bytes",
                "Words",
                "Outlinks",
                "Images",
                "Error",
                "Rendered",
                "Raw Title",
                "Rendered URL",
                "Render Error",
                "Browser Requests",
                "Console Errors"
            ]
        );
        assert_eq!(
            &headers[25..32],
            [
                "HTML Canonical",
                "HTTP Canonical",
                "Indexability Agent",
                "Effective Robots",
                "Pagination Next",
                "Pagination Prev",
                "AMPHTML"
            ]
        );
        assert_eq!(rows[0][5], title);
        assert_eq!(rows[0][9], "https://fixture.test/canonical");
        assert_eq!(
            &rows[0][25..32],
            [
                "https://fixture.test/html",
                "https://fixture.test/http",
                "googlebot",
                "nofollow",
                "https://fixture.test/next",
                "https://fixture.test/prev",
                "https://fixture.test/amp"
            ]
        );
    }

    #[test]
    fn pages_csv_appends_measured_pixels_with_unknown_positions_and_legacy_empty_fields() {
        let observed = PageRecord {
            url: "https://fixture.test/observed".into(),
            title_pixel_widths: vec![Some(561), None, Some(0)],
            description_pixel_widths: vec![None, Some(985)],
            text_metrics_engine: Some("observed-font-profile".into()),
            ..Default::default()
        };
        let legacy: PageRecord =
            serde_json::from_value(json!({"url":"https://fixture.test/legacy"})).unwrap();
        let zero = PageRecord {
            title_pixel_widths: vec![Some(0)],
            description_pixel_widths: vec![Some(0)],
            ..Default::default()
        };
        let (headers, rows) =
            read_csv(&export_pages_csv(&report(vec![observed, legacy, zero])).unwrap());
        assert_eq!(headers.len(), 42);
        assert_eq!(
            &headers[32..37],
            [
                "Title Pixels",
                "Description Pixels",
                "Title Pixel Measurements",
                "Description Pixel Measurements",
                "Text Metrics Engine"
            ]
        );
        assert_eq!(
            &rows[0][32..37],
            [
                "561",
                "",
                "[561,null,0]",
                "[null,985]",
                "observed-font-profile"
            ]
        );
        let positions: Vec<Option<u32>> = serde_json::from_str(&rows[0][34]).unwrap();
        assert_eq!(positions, vec![Some(561), None, Some(0)]);
        assert!(rows[1][32..37].iter().all(String::is_empty));
        assert_eq!(&rows[2][32..37], ["0", "0", "[0]", "[0]", ""]);
        assert_eq!(&rows[1][37..], ["crawl_snapshot", "captured", "", "", ""]);
    }

    #[test]
    fn legacy_pages_and_empty_browser_reports_export_without_fabricated_observations() {
        let legacy: PageRecord = serde_json::from_value(
            json!({"url":"https://fixture.test/legacy", "meta_robots":["noindex"]}),
        )
        .unwrap();
        let (_, rows) = read_csv(&export_pages_csv(&report(vec![legacy])).unwrap());
        assert_eq!(rows[0][25], "");
        assert_eq!(rows[0][26], "");
        assert_eq!(rows[0][27], "googlebot");
        assert_eq!(rows[0][28], "noindex");
        assert!(rows[0][29..37].iter().all(String::is_empty));
        assert_eq!(&rows[0][37..], ["crawl_snapshot", "captured", "", "", ""]);
        let empty = report(vec![]);
        for csv in [
            export_custom_javascript_csv(&empty).unwrap(),
            export_browser_cookies_csv(&empty).unwrap(),
        ] {
            assert!(read_csv(&csv).1.is_empty());
        }
    }

    #[test]
    fn pages_csv_preserves_selected_observation_basis_and_protects_untrusted_metadata() {
        let mut source = report(vec![
            PageRecord {
                url: "https://fixture.test/retained".into(),
                fetched_at: "2026-10-11T12:00:00Z".into(),
                ..Default::default()
            },
            PageRecord {
                url: "https://fixture.test/refreshed".into(),
                fetched_at: "2026-10-12T12:00:00Z".into(),
                ..Default::default()
            },
            // Imported reports can contain a stale pending page; this must never claim freshness.
            PageRecord {
                url: "https://fixture.test/pending".into(),
                fetched_at: "\t=untrusted_timestamp".into(),
                ..Default::default()
            },
        ]);
        source.config.seed_urls = vec!["https://fixture.test/retained".into()];
        let prepared = crate::prepare_selected_recrawl(
            crate::SelectedRecrawlParent {
                job_id: "csv-parent".into(),
                checkpoint_revision: Some(1),
                source_kind: crate::RecrawlSourceKind::CapturedCheckpoint,
            },
            &serde_json::to_vec(&source).unwrap(),
            &[
                "https://fixture.test/refreshed".into(),
                "https://fixture.test/pending".into(),
            ],
            Some(1),
        )
        .unwrap();
        let targets = prepared
            .plan
            .targets
            .iter()
            .map(|target| crate::RecrawlTargetObservation {
                requested_url: target.requested_url.clone(),
                parent_ordinal: target.parent_ordinal,
                parent_page_sha256: target.parent_page_sha256.clone(),
                state: if target.requested_url.ends_with("/refreshed") {
                    crate::RecrawlTargetState::Committed
                } else {
                    crate::RecrawlTargetState::Pending
                },
                admitted_attempts: usize::from(target.requested_url.ends_with("/refreshed")),
                committed_generation: target.requested_url.ends_with("/refreshed").then_some(1),
            })
            .collect();
        let mut operation = prepared.plan.operation(1, targets, Some(1), 1, 1);
        operation.parent.snapshot_sha256 = "=untrusted_parent".into();
        operation.coverage = "+untrusted_coverage".into();
        source.operation = Some(operation);
        let (headers, rows) = read_csv(&export_pages_csv(&source).unwrap());
        assert_eq!(
            &headers[37..],
            [
                "Crawl Coverage",
                "Observation Basis",
                "Selected State",
                "Parent Snapshot SHA256",
                "Fetched At"
            ]
        );
        assert_eq!(
            &rows[0][37..],
            [
                "'+untrusted_coverage",
                "inherited_from_parent",
                "",
                "'=untrusted_parent",
                "2026-10-11T12:00:00Z"
            ]
        );
        assert_eq!(
            &rows[1][37..],
            [
                "'+untrusted_coverage",
                "refreshed_in_selected_job",
                "committed",
                "'=untrusted_parent",
                "2026-10-12T12:00:00Z"
            ]
        );
        assert_eq!(
            &rows[2][37..],
            [
                "'+untrusted_coverage",
                "selected_pending_not_refreshed",
                "pending",
                "'=untrusted_parent",
                "'\t=untrusted_timestamp"
            ]
        );
        source.operation.as_mut().unwrap().engine = "future-operation/v2".into();
        let (_, future_rows) = read_csv(&export_pages_csv(&source).unwrap());
        for row in future_rows {
            assert_eq!(row[38], "unknown_operation");
            assert_eq!(row[39], "");
        }
    }

    #[test]
    fn extraction_csv_preserves_old_columns_and_retains_limit_error_and_unknown_evidence() {
        let legacy = serde_json::from_value(
            json!({"name":"legacy", "selector":".price", "values":["10"], "source":"raw_html"}),
        )
        .unwrap();
        let page = PageRecord {
            url: "https://fixture.test/a".into(),
            custom_extractions: vec![
                crate::CustomExtraction {
                    name: "bounded".into(),
                    selector: r#"price=([0-9]+)"#.into(),
                    method: crate::ExtractionMethod::Regex,
                    engine: "rust-regex".into(),
                    result_type: "string".into(),
                    matched_count: Some(5),
                    truncated: true,
                    values: vec!["10,\"quote\"".into(), "20\nline".into()],
                    source: "rendered_html".into(),
                    ..Default::default()
                },
                crate::CustomExtraction {
                    name: "failed".into(),
                    selector: "/invalid[".into(),
                    method: crate::ExtractionMethod::XPath,
                    engine: "sxd-xpath".into(),
                    error: Some("Parser, error\nnext line".into()),
                    ..Default::default()
                },
                legacy,
            ],
            ..Default::default()
        };
        let (headers, rows) =
            read_csv(&export_custom_extractions_csv(&report(vec![page])).unwrap());
        assert_eq!(
            &headers[..8],
            [
                "URL",
                "Name",
                "Selector",
                "Attribute",
                "Source",
                "Match Count",
                "Match Index",
                "Value"
            ]
        );
        assert_eq!(
            &headers[8..],
            [
                "Method",
                "Engine",
                "Result Type",
                "Observed Match Count",
                "Truncated",
                "Error",
                "Output Mode"
            ]
        );
        assert_eq!(rows.len(), 4);
        assert_eq!(&rows[0][4..8], ["rendered_html", "2", "1", "10,\"quote\""]);
        assert_eq!(
            &rows[0][8..],
            ["regex", "rust-regex", "string", "5", "true", "", "auto"]
        );
        assert_eq!(rows[1][7], "20\nline");
        assert_eq!(rows[2][5], "0");
        assert_eq!(rows[2][6], "");
        assert_eq!(
            &rows[2][8..],
            [
                "xpath",
                "sxd-xpath",
                "",
                "",
                "false",
                "Parser, error\nnext line",
                "auto"
            ]
        );
        assert_eq!(rows[3][8], "css");
        assert_eq!(rows[3][9], "");
        assert_eq!(rows[3][11], "");
    }

    #[test]
    fn every_csv_export_neutralizes_formula_text_and_preserves_typed_numbers() {
        use crate::{AuditIssue, CustomExtraction, ExtractionMethod, LinkRecord, Severity};
        let mut page = PageRecord {
            url: "https://fixture.test/a".into(),
            titles: vec!["=SUM(1,2)".into()],
            meta_descriptions: vec![" +dangerous".into()],
            h1: vec!["\t@dangerous".into()],
            h2: vec!["\r\n -dangerous".into()],
            error: Some("\u{feff}=ERROR()".into()),
            text_metrics_engine: Some("\t=IMPORTED_PROFILE()".into()),
            response_ms: 42,
            links: vec![LinkRecord {
                url: "https://fixture.test/target".into(),
                text: "\t=LINK()".into(),
                rel: "@directive".into(),
                ..Default::default()
            }],
            custom_extractions: vec![
                CustomExtraction {
                    name: "-configured".into(),
                    selector: " +selector".into(),
                    values: vec!["\r=VALUE()".into()],
                    error: Some("@error".into()),
                    ..Default::default()
                },
                CustomExtraction {
                    name: "numeric".into(),
                    method: ExtractionMethod::XPath,
                    result_type: "number".into(),
                    values: vec!["-2.5".into()],
                    ..Default::default()
                },
            ],
            custom_javascript_results: vec![
                json!({"name":"@script","value":-7,"value_type":"number","error":"\t+error","elapsed_ms":0}),
            ],
            browser_cookies: vec![
                json!({"name":"-cookie","domain":"fixture.test","path":"@path","expires":-1,"httpOnly":false,"secure":true,"sameSite":"Lax","value_length":0}),
            ],
            ..Default::default()
        };
        page.raw_metadata = Some(crate::RawMetadata {
            titles: vec!["\n@RAW()".into()],
            ..Default::default()
        });
        let mut snapshot = report(vec![page]);
        snapshot.issues.push(AuditIssue {
            severity: Severity::Notice,
            code: "+code".into(),
            url: "https://fixture.test/a".into(),
            message: " \t=ISSUE()".into(),
            related_urls: vec!["-related".into()],
        });
        let (_, pages) = read_csv(&export_pages_csv(&snapshot).unwrap());
        assert_eq!(pages[0][5], "'=SUM(1,2)");
        assert_eq!(pages[0][6], "' +dangerous");
        assert_eq!(pages[0][7], "'\t@dangerous");
        assert_eq!(pages[0][8], "'\r\n -dangerous");
        assert_eq!(pages[0][18], "'\u{feff}=ERROR()");
        assert_eq!(pages[0][20], "'\n@RAW()");
        assert_eq!(pages[0][13], "42");
        assert_eq!(pages[0][36], "'\t=IMPORTED_PROFILE()");
        let (_, issues) = read_csv(&export_issues_csv(&snapshot).unwrap());
        assert_eq!(issues[0][1], "'+code");
        assert_eq!(issues[0][3], "' \t=ISSUE()");
        assert_eq!(issues[0][4], "'-related");
        let (_, links) = read_csv(&export_links_csv(&snapshot).unwrap());
        assert_eq!(links[0][2], "'\t=LINK()");
        assert_eq!(links[0][5], "'@directive");
        let (_, extractions) = read_csv(&export_custom_extractions_csv(&snapshot).unwrap());
        assert_eq!(extractions[0][1], "'-configured");
        assert_eq!(extractions[0][2], "' +selector");
        assert_eq!(extractions[0][7], "'\r=VALUE()");
        assert_eq!(extractions[0][13], "'@error");
        assert_eq!(extractions[1][7], "-2.5");
        let (_, javascript) = read_csv(&export_custom_javascript_csv(&snapshot).unwrap());
        assert_eq!(javascript[0][1], "'@script");
        assert_eq!(javascript[0][3], "-7");
        assert_eq!(javascript[0][4], "'\t+error");
        assert_eq!(javascript[0][5], "0");
        let (_, cookies) = read_csv(&export_browser_cookies_csv(&snapshot).unwrap());
        assert_eq!(cookies[0][1], "'-cookie");
        assert_eq!(cookies[0][3], "'@path");
        assert_eq!(cookies[0][4], "-1");
        assert_eq!(cookies[0][5], "false");
        assert_eq!(cookies[0][8], "0");
        let json = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(json["pages"][0]["titles"][0], "=SUM(1,2)");
        assert_eq!(json["issues"][0]["message"], " \t=ISSUE()");
    }

    #[test]
    fn resource_csv_distinguishes_unknown_from_zero_and_preserves_errors() {
        let pages = vec![
            PageRecord {
                url: "https://fixture.test/document.pdf".into(),
                pdf: Some(crate::PdfRecord {
                    status: crate::ResourceAnalysisStatus::Partial,
                    page_count: Some(2),
                    text_chars: None,
                    word_count: Some(0),
                    parse_error: Some(" \t=FAILED()".into()),
                    limits_reached: vec!["max_pages".into()],
                    ..Default::default()
                }),
                ..Default::default()
            },
            PageRecord {
                url: "https://fixture.test/vector.svg".into(),
                image_metadata: Some(crate::ImageResourceMetadata {
                    format: Some("svg".into()),
                    width: None,
                    height: Some(30.5),
                    body_complete: true,
                    svg_view_box: Some([0.0, 0.0, 80.0, 40.0]),
                    parse_error: Some("decode, error\nnext".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ];
        let snapshot = report(pages);
        let (_, pdf) = read_csv(&export_pdf_csv(&snapshot).unwrap());
        assert_eq!(pdf.len(), 1);
        assert_eq!(pdf[0][3], "partial");
        assert_eq!(pdf[0][7], "2");
        assert_eq!(pdf[0][10], "");
        assert_eq!(pdf[0][11], "0");
        assert_eq!(pdf[0][21], "' \t=FAILED()");
        let (_, images) = read_csv(&export_image_resources_csv(&snapshot).unwrap());
        assert_eq!(images.len(), 1);
        assert_eq!(images[0][6], "");
        assert_eq!(images[0][7], "30.5");
        assert_eq!(images[0][11], "[0.0,0.0,80.0,40.0]");
        assert_eq!(images[0][13], "decode, error\nnext");
        assert!(
            read_csv(&export_pdf_csv(&report(vec![])).unwrap())
                .1
                .is_empty()
        );
        assert!(
            read_csv(&export_image_resources_csv(&report(vec![])).unwrap())
                .1
                .is_empty()
        );
    }

    #[test]
    fn spreadsheet_prefix_is_applied_only_to_formula_like_text() {
        for text in [
            "=1",
            "+1",
            "-1",
            "@function",
            " \t\r\n=1",
            "\u{a0}-1",
            "\u{feff}=1",
        ] {
            assert_eq!(csv_text(text), format!("'{text}"));
        }
        for text in [
            "",
            "regular text",
            "\tregular text",
            "'already safe",
            "value=1",
            "https://fixture.test/@path",
            "42",
        ] {
            assert_eq!(csv_text(text), text);
        }
    }
    #[test]
    fn input_export_keeps_duplicate_invalid_and_unfetched_rows_without_fake_status() {
        let input = crate::import_url_list(b"https://example.test/a#first\n=FORMULA()\nhttps://example.test/a#second\nhttps://example.test/unfetched", &crate::ListImportOptions {format:crate::ListImportFormat::Txt,limits:crate::ListImportLimits::default()}).unwrap();
        let mut report = report(vec![PageRecord {
            url: "https://example.test/a".into(),
            final_url: "https://example.test/final".into(),
            status: Some(301),
            final_status: Some(200),
            ..Default::default()
        }]);
        report.config.input_imports.push(input);
        let text = export_input_occurrences_csv(&report).unwrap();
        let mut reader = csv::Reader::from_reader(text.as_bytes());
        let rows = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(&rows[0][3], "1");
        assert_eq!(&rows[0][13], "first");
        assert_eq!(&rows[2][13], "second");
        assert_eq!(&rows[0][16], "301");
        assert_eq!(&rows[2][17], "200");
        assert_eq!(&rows[1][8], "'=FORMULA()");
        assert_eq!(&rows[1][15], "invalid_input");
        assert_eq!(&rows[1][16], "");
        assert_eq!(&rows[3][15], "unfetched");
        assert_eq!(&rows[3][17], "");
    }
}
