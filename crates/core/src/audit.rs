use crate::{AuditIssue, AuditThresholds, PageRecord, Severity};
use std::collections::{BTreeMap, HashMap, HashSet};

pub(crate) fn set_indexability(page: &mut PageRecord) {
    let mut reasons = Vec::new();
    if page.blocked_by_robots {
        reasons.push("Blocked by robots.txt".into());
    }
    if page.error.is_some() {
        reasons.push("Request or response failed".into());
    }
    if page.status != Some(200) {
        reasons.push("Requested URL does not return HTTP 200".into());
    }
    if !page.is_indexable_document() {
        reasons.push("Not a supported indexable document type".into());
    }
    let noindex = crate::applicable_robot_directives(page)
        .iter()
        .any(|value| matches!(value.as_str(), "noindex" | "none"));
    if noindex {
        reasons.push("Noindex directive".into());
    }
    if page.canonical.iter().any(|url| url != page.document_url()) {
        reasons.push("Canonical points to another URL".into());
    }
    if page.truncated {
        reasons.push("Response body incomplete; indexability not established".into());
    }
    if page.renderer_error.is_some() {
        reasons.push("Browser rendering incomplete; rendered indexability not established".into());
    }
    if page.rendered && page.rendered_status != Some(200) {
        reasons.push(match page.rendered_status {
            Some(status) => format!(
                "Rendered response returns HTTP {status}; rendered indexability not established"
            ),
            None => "Rendered response status not observed; rendered indexability not established"
                .into(),
        });
    }
    page.indexable = reasons.is_empty();
    page.indexability_reasons = reasons;
}

fn issue(
    issues: &mut Vec<AuditIssue>,
    page: &PageRecord,
    code: &str,
    severity: Severity,
    message: impl Into<String>,
    related_urls: Vec<String>,
) {
    issues.push(AuditIssue {
        code: code.into(),
        severity,
        url: page.url.clone(),
        message: message.into(),
        related_urls,
    });
}

pub fn audit_pages(pages: &[PageRecord]) -> Vec<AuditIssue> {
    audit_pages_with_thresholds(pages, &AuditThresholds::default())
}

/// Rules inspect captured facts and use the supplied independent review limits.
/// Absent body metadata is assessed only for complete successful HTML captures.
pub fn audit_pages_with_thresholds(
    pages: &[PageRecord],
    thresholds: &AuditThresholds,
) -> Vec<AuditIssue> {
    let mut issues = Vec::new();
    let by_url: HashMap<&str, &PageRecord> =
        pages.iter().map(|page| (page.url.as_str(), page)).collect();
    for page in pages {
        crate::onpage::audit_page(page, thresholds, &mut issues);
        if page.blocked_by_robots {
            issue(
                &mut issues,
                page,
                if page.robots_warning.is_some() {
                    "robots_unavailable"
                } else {
                    "robots_blocked"
                },
                if page.robots_warning.is_some() {
                    Severity::Warning
                } else {
                    Severity::Notice
                },
                page.robots_warning
                    .clone()
                    .unwrap_or_else(|| "Crawling disallowed by robots.txt".into()),
                vec![],
            );
            continue;
        }
        if let Some(error) = &page.error {
            issue(
                &mut issues,
                page,
                "request_failed",
                Severity::Error,
                error,
                vec![],
            );
        }
        if let Some(status) = page.status {
            if status >= 500 {
                issue(
                    &mut issues,
                    page,
                    "server_error",
                    Severity::Error,
                    format!("HTTP {status}"),
                    vec![],
                );
            } else if status >= 400 {
                issue(
                    &mut issues,
                    page,
                    "client_error",
                    Severity::Error,
                    format!("HTTP {status}"),
                    vec![],
                );
            } else if (300..400).contains(&status) {
                issue(
                    &mut issues,
                    page,
                    "redirect",
                    Severity::Notice,
                    format!("Redirects to {}", page.final_url),
                    vec![page.final_url.clone()],
                );
            }
        }
        if page.redirect_chain.len() > 1 {
            issue(
                &mut issues,
                page,
                "redirect_chain",
                Severity::Warning,
                format!("{} redirect hops", page.redirect_chain.len()),
                page.redirect_chain
                    .iter()
                    .map(|hop| hop.target.clone())
                    .collect(),
            );
        }
        if page.truncated {
            issue(
                &mut issues,
                page,
                "body_truncated",
                Severity::Warning,
                "Response exceeds configured body limit; extraction may be incomplete",
                vec![],
            );
        }
        if let Some(error) = &page.renderer_error {
            issue(
                &mut issues,
                page,
                "renderer_failed",
                Severity::Warning,
                error,
                vec![],
            );
        }
        if !page.internal
            || !page.is_html()
            || page.final_status != Some(200)
            || page.error.is_some()
            || page.truncated
            || page.renderer_error.is_some()
            || (page.rendered && page.rendered_status != Some(200))
        {
            continue;
        }
        if page.url.starts_with("http://") {
            issue(
                &mut issues,
                page,
                "http_url",
                Severity::Warning,
                "HTML served over unencrypted HTTP",
                vec![],
            );
        }
        if page.titles.is_empty() || page.title().trim().is_empty() {
            issue(
                &mut issues,
                page,
                "title_missing",
                Severity::Warning,
                "Page title is missing or empty",
                vec![],
            );
        }
        if page.titles.len() > 1 {
            issue(
                &mut issues,
                page,
                "title_multiple",
                Severity::Warning,
                "Multiple title elements",
                vec![],
            );
        }
        if page.meta_descriptions.is_empty() || page.description().trim().is_empty() {
            issue(
                &mut issues,
                page,
                "description_missing",
                Severity::Notice,
                "Meta description is missing or empty",
                vec![],
            );
        }
        if page.meta_descriptions.len() > 1 {
            issue(
                &mut issues,
                page,
                "description_multiple",
                Severity::Warning,
                "Multiple meta descriptions",
                vec![],
            );
        }
        if page.h1.is_empty() || page.h1.iter().all(|h| h.trim().is_empty()) {
            issue(
                &mut issues,
                page,
                "h1_missing",
                Severity::Warning,
                "No nonempty H1 heading",
                vec![],
            );
        }
        if page.h1.len() > 1 {
            issue(
                &mut issues,
                page,
                "h1_multiple",
                Severity::Notice,
                "Multiple H1 headings; review document structure",
                vec![],
            );
        }
        if page.canonical.is_empty() {
            issue(
                &mut issues,
                page,
                "canonical_missing",
                Severity::Notice,
                "No HTML canonical link",
                vec![],
            );
        }
        if page.canonical.len() > 1
            || page.html_canonical.len() > 1
            || page.header_canonical.len() > 1
        {
            issue(
                &mut issues,
                page,
                "canonical_multiple",
                Severity::Warning,
                "Multiple canonical links",
                page.canonical.clone(),
            );
        }
        if !page.indexable {
            issue(
                &mut issues,
                page,
                "nonindexable",
                Severity::Notice,
                page.indexability_reasons.join("; "),
                vec![],
            );
        }
        if page.language.as_deref().unwrap_or("").trim().is_empty() {
            issue(
                &mut issues,
                page,
                "language_missing",
                Severity::Notice,
                "HTML language attribute is missing",
                vec![],
            );
        }
        if page.viewport.is_none() {
            issue(
                &mut issues,
                page,
                "viewport_missing",
                Severity::Notice,
                "No viewport meta tag",
                vec![],
            );
        }
        let alt_missing = page
            .images
            .iter()
            .filter(|image| image.alt.is_none())
            .count();
        if alt_missing > 0 {
            issue(
                &mut issues,
                page,
                "image_alt_missing",
                Severity::Warning,
                format!(
                    "{alt_missing} image(s) have no alt attribute; empty alt attributes are retained for decorative images"
                ),
                page.images
                    .iter()
                    .filter(|image| image.alt.is_none())
                    .map(|image| image.url.clone())
                    .collect(),
            );
        }
        if page.invalid_structured_data > 0 {
            issue(
                &mut issues,
                page,
                "structured_data_invalid",
                Severity::Warning,
                format!(
                    "{} JSON-LD block(s) are invalid JSON",
                    page.invalid_structured_data
                ),
                vec![],
            );
        }
        if page.final_url.starts_with("https://") {
            let mixed: Vec<_> = page
                .links
                .iter()
                .filter(|link| link.kind != "anchor" && link.url.starts_with("http://"))
                .map(|link| link.url.clone())
                .collect();
            if !mixed.is_empty() {
                issue(
                    &mut issues,
                    page,
                    "mixed_content",
                    Severity::Warning,
                    "HTTPS page references unencrypted resources",
                    mixed,
                );
            }
        }
        let broken: HashSet<_> = page
            .links
            .iter()
            .filter_map(|link| {
                by_url
                    .get(link.url.as_str())
                    .filter(|target| {
                        target.final_status.is_some_and(|status| status >= 400)
                            || target.error.is_some()
                    })
                    .map(|_| link.url.clone())
            })
            .collect();
        if !broken.is_empty() {
            let mut broken: Vec<_> = broken.into_iter().collect();
            broken.sort();
            issue(
                &mut issues,
                page,
                "broken_outlinks",
                Severity::Error,
                format!("{} checked outgoing link(s) failed", broken.len()),
                broken,
            );
        }
        for canonical in &page.canonical {
            if let Some(target) = by_url.get(canonical.as_str())
                && !target.blocked_by_robots
                && (target.status.is_some_and(|status| status != 200)
                    || crate::applicable_robot_directives(target)
                        .iter()
                        .any(|rule| matches!(rule.as_str(), "noindex" | "none"))
                    || target
                        .canonical
                        .iter()
                        .any(|url| url != target.document_url()))
            {
                issue(
                    &mut issues,
                    page,
                    "canonical_nonindexable",
                    Severity::Warning,
                    "Checked canonical target is nonindexable",
                    vec![canonical.clone()],
                );
            }
        }
        let mut languages = HashSet::new();
        for alternate in &page.hreflang {
            if !languages.insert(alternate.language.to_ascii_lowercase()) {
                issue(
                    &mut issues,
                    page,
                    "hreflang_duplicate",
                    Severity::Warning,
                    format!("Repeated hreflang {}", alternate.language),
                    vec![alternate.url.clone()],
                );
            }
            if let Some(target) = by_url.get(alternate.url.as_str())
                && target.url != page.url
                && target.is_html()
                && target.final_status == Some(200)
                && target.error.is_none()
                && target.renderer_error.is_none()
                && (!target.rendered || target.rendered_status == Some(200))
                && !target.truncated
                && !target.blocked_by_robots
                && !target.hreflang.iter().any(|h| h.url == page.document_url())
            {
                issue(
                    &mut issues,
                    page,
                    "hreflang_return_missing",
                    Severity::Warning,
                    "Checked alternate has no return hreflang link",
                    vec![alternate.url.clone()],
                );
            }
        }
        if page.network_events_dropped > 0 {
            issue(
                &mut issues,
                page,
                "browser_network_capture_incomplete",
                Severity::Warning,
                format!(
                    "Browser network capture is incomplete: {} event(s) exceeded the worker resource limit",
                    page.network_events_dropped
                ),
                vec![],
            );
        }
        if !page.console_errors.is_empty() {
            issue(
                &mut issues,
                page,
                "browser_console_errors",
                Severity::Warning,
                format!(
                    "Browser reported {} console or page error(s)",
                    page.console_errors.len()
                ),
                vec![],
            );
        }
        let failed_resources: Vec<_> = page
            .network_requests
            .iter()
            .filter(|request| {
                request.get("error").is_some_and(|error| !error.is_null())
                    || request
                        .get("status")
                        .and_then(serde_json::Value::as_u64)
                        .is_some_and(|status| status >= 400)
            })
            .filter_map(|request| {
                request
                    .get("url")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .collect();
        if !failed_resources.is_empty() {
            issue(
                &mut issues,
                page,
                "browser_resource_failed",
                Severity::Warning,
                format!(
                    "{} browser resource request(s) failed",
                    failed_resources.len()
                ),
                failed_resources,
            );
        }
        if let Some(violations) = page
            .accessibility
            .as_ref()
            .and_then(|axe| axe.get("violations"))
            .and_then(serde_json::Value::as_array)
        {
            for violation in violations {
                let id = violation
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                let help = violation
                    .get("help")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Automated accessibility finding");
                let nodes = violation
                    .get("nodes")
                    .and_then(serde_json::Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0);
                let severity = match violation.get("impact").and_then(serde_json::Value::as_str) {
                    Some("critical" | "serious") => Severity::Error,
                    Some("minor") => Severity::Notice,
                    _ => Severity::Warning,
                };
                issue(
                    &mut issues,
                    page,
                    &format!("accessibility_{id}"),
                    severity,
                    format!("{help} ({nodes} affected nodes; automated check only)"),
                    vec![],
                );
            }
        }
    }
    for (code, label, getter) in [
        (
            "title_duplicate",
            "title",
            PageRecord::title as fn(&PageRecord) -> &str,
        ),
        (
            "description_duplicate",
            "meta description",
            PageRecord::description as fn(&PageRecord) -> &str,
        ),
    ] {
        let mut groups: BTreeMap<String, Vec<&PageRecord>> = BTreeMap::new();
        for page in pages.iter().filter(|page| page.internal && page.indexable) {
            let value = getter(page).trim().to_ascii_lowercase();
            if !value.is_empty() {
                groups.entry(value).or_default().push(page);
            }
        }
        for group in groups.values().filter(|group| group.len() > 1) {
            for page in group {
                issue(
                    &mut issues,
                    page,
                    code,
                    Severity::Warning,
                    format!("Same {label} on {} indexable URLs", group.len()),
                    group
                        .iter()
                        .filter(|other| other.url != page.url)
                        .map(|other| other.url.clone())
                        .collect(),
                );
            }
        }
    }
    let mut hashes: BTreeMap<&str, Vec<&PageRecord>> = BTreeMap::new();
    for page in pages
        .iter()
        .filter(|page| page.internal && page.indexable && page.word_count > 0)
    {
        if let Some(hash) = page.content_hash.as_deref() {
            hashes.entry(hash).or_default().push(page);
        }
    }
    for group in hashes.values().filter(|group| group.len() > 1) {
        for page in group {
            issue(
                &mut issues,
                page,
                "content_duplicate",
                Severity::Warning,
                "Identical normalized visible text; this is an exact-content comparison",
                group
                    .iter()
                    .filter(|other| other.url != page.url)
                    .map(|other| other.url.clone())
                    .collect(),
            );
        }
    }
    crate::onpage::audit_heading_duplicates(pages, &mut issues);
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_indexability_requires_known_success_without_inventing_noindex() {
        let base = PageRecord {
            url: "https://fixture.test/a".into(),
            final_url: "https://fixture.test/a".into(),
            status: Some(200),
            final_status: Some(200),
            content_type: "text/html".into(),
            rendered: true,
            ..Default::default()
        };
        for status in [None, Some(404), Some(200)] {
            let mut page = base.clone();
            page.rendered_status = status;
            set_indexability(&mut page);
            assert_eq!(page.indexable, status == Some(200));
            assert!(
                !page
                    .indexability_reasons
                    .iter()
                    .any(|reason| reason.contains("Noindex"))
            );
            if status == Some(200) {
                assert!(page.indexability_reasons.is_empty());
            } else {
                assert!(page.indexability_reasons.iter().any(|reason| {
                    reason.contains("Rendered response")
                        && reason.contains("indexability not established")
                }));
            }
        }
        let mut native = base;
        native.rendered = false;
        set_indexability(&mut native);
        assert!(native.indexable);
    }
    #[test]
    fn configured_long_thresholds_replace_legacy_fixed_rules_without_duplicate_notices() {
        let page = PageRecord {
            url: "https://fixture.test/a".into(),
            final_url: "https://fixture.test/a".into(),
            status: Some(200),
            final_status: Some(200),
            internal: true,
            content_type: "text/html".into(),
            titles: vec!["x".repeat(61)],
            meta_descriptions: vec!["y".repeat(156)],
            ..Default::default()
        };
        let issues = audit_pages(std::slice::from_ref(&page));
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.code == "title_long")
                .count(),
            1
        );
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.code == "description_long")
                .count(),
            1
        );
        let limits = AuditThresholds {
            max_title_chars: None,
            max_description_chars: Some(160),
            ..Default::default()
        };
        let issues = audit_pages_with_thresholds(std::slice::from_ref(&page), &limits);
        assert!(
            !issues
                .iter()
                .any(|issue| issue.code == "title_long" || issue.code == "description_long")
        );
    }

    #[test]
    fn complete_capture_guard_applies_to_old_and_new_absence_checks() {
        let base = PageRecord {
            url: "https://fixture.test/a".into(),
            final_url: "https://fixture.test/a".into(),
            status: Some(200),
            final_status: Some(200),
            internal: true,
            content_type: "text/html".into(),
            ..Default::default()
        };
        for condition in 0..4 {
            let mut page = base.clone();
            match condition {
                0 => page.truncated = true,
                1 => page.renderer_error = Some("worker unavailable".into()),
                2 => {
                    page.rendered = true;
                    page.rendered_status = Some(404)
                }
                _ => {
                    page.rendered = true;
                    page.rendered_status = None
                }
            }
            let issues = audit_pages(&[page]);
            for code in [
                "title_missing",
                "description_missing",
                "h1_missing",
                "h2_missing",
                "canonical_missing",
                "internal_outlinks_missing",
            ] {
                assert!(
                    !issues.iter().any(|issue| issue.code == code),
                    "false {code} for {condition}"
                );
            }
        }
    }

    #[test]
    fn reports_duplicates_and_broken_inlinks() {
        let page = PageRecord {
            url: "https://e.org/".into(),
            final_url: "https://e.org/".into(),
            status: Some(200),
            final_status: Some(200),
            content_type: "text/html".into(),
            internal: true,
            indexable: true,
            titles: vec!["Same".into()],
            links: vec![crate::LinkRecord {
                url: "https://e.org/bad".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut second = page.clone();
        second.url = "https://e.org/second".into();
        let failed = PageRecord {
            url: "https://e.org/bad".into(),
            status: Some(404),
            final_status: Some(404),
            ..Default::default()
        };
        let issues = audit_pages(&[page, second, failed]);
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.code == "title_duplicate")
                .count(),
            2
        );
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.code == "broken_outlinks")
                .count(),
            2
        );
    }

    #[test]
    fn hreflang_absence_requires_a_complete_successful_alternate() {
        let source = PageRecord {
            url: "https://e.org/".into(),
            final_url: "https://e.org/".into(),
            status: Some(200),
            final_status: Some(200),
            internal: true,
            content_type: "text/html".into(),
            hreflang: vec![crate::HreflangRecord {
                language: "de".into(),
                url: "https://e.org/de/".into(),
            }],
            ..Default::default()
        };
        let target = PageRecord {
            url: "https://e.org/de/".into(),
            final_url: "https://e.org/de/".into(),
            status: Some(200),
            final_status: Some(200),
            internal: true,
            content_type: "text/html".into(),
            truncated: true,
            ..Default::default()
        };
        assert!(
            !audit_pages(&[source.clone(), target.clone()])
                .iter()
                .any(|i| i.code == "hreflang_return_missing")
        );
        let mut failed = target.clone();
        failed.truncated = false;
        failed.final_status = Some(500);
        assert!(
            !audit_pages(&[source.clone(), failed])
                .iter()
                .any(|i| i.code == "hreflang_return_missing")
        );
        let mut complete = target;
        complete.truncated = false;
        let mut unknown_rendered = complete.clone();
        unknown_rendered.rendered = true;
        unknown_rendered.rendered_status = None;
        assert!(
            !audit_pages(&[source.clone(), unknown_rendered])
                .iter()
                .any(|i| i.code == "hreflang_return_missing")
        );
        let mut successful_rendered = complete.clone();
        successful_rendered.rendered = true;
        successful_rendered.rendered_status = Some(200);
        assert!(
            audit_pages(&[source.clone(), successful_rendered])
                .iter()
                .any(|i| i.code == "hreflang_return_missing")
        );
        assert!(
            audit_pages(&[source, complete])
                .iter()
                .any(|i| i.code == "hreflang_return_missing")
        );
    }
}
