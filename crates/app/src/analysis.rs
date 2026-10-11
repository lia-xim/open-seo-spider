//! Deterministic analysis of already captured crawl observations.
//! No network requests, provider access, or proprietary rule sets are used.
use open_seo_spider_core::{AuditIssue, CrawlReport, PageRecord, Severity};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const DAMPING: f64 = 0.85;
const TOLERANCE: f64 = 1e-12;
const MAX_ITERATIONS: usize = 200;

/// Merge independently observed relationship/security issues into every export.
pub fn extend_report(report: &mut CrawlReport) {
    let analysis = analyze_report(report);
    if let Ok(issues) = serde_json::from_value::<Vec<AuditIssue>>(analysis["issues"].clone()) {
        report.issues.extend(issues);
    }
    report.summary.errors = report
        .issues
        .iter()
        .filter(|issue| matches!(issue.severity, Severity::Error))
        .count();
    report.summary.warnings = report
        .issues
        .iter()
        .filter(|issue| matches!(issue.severity, Severity::Warning))
        .count();
}

/// Analyze only the supplied snapshot. Absence from a snapshot is never proof
/// that a URL is missing from the website or has no links on the entire site.
pub fn analyze_report(report: &CrawlReport) -> Value {
    let mut pages = BTreeMap::new();
    for page in &report.pages {
        pages.entry(page.url.as_str()).or_insert(page);
    }
    // Prefer a directly fetched URL over a redirect observation's final alias.
    let mut lookup = pages.clone();
    for page in pages.values() {
        if !page.final_url.is_empty() {
            lookup.entry(page.final_url.as_str()).or_insert(*page);
        }
    }
    let mut issues = Vec::new();
    let existing: BTreeSet<_> = report
        .issues
        .iter()
        .map(|issue| (issue.url.clone(), issue.code.clone()))
        .collect();
    for page in pages.values().copied().filter(|page| usable_html(page)) {
        analyze_relationships(page, &lookup, &mut issues);
        analyze_security(page, &mut issues);
    }
    let cycles = canonical_cycles(&pages, &lookup);
    for cycle in &cycles {
        for url in cycle {
            if let Some(page) = pages.get(url.as_str()) {
                issue(
                    &mut issues,
                    page,
                    "canonical_loop",
                    Severity::Warning,
                    "Observed canonical declarations form a cycle",
                    cycle.clone(),
                );
            }
        }
    }
    issues.sort_by(|a, b| {
        (&a.url, &a.code, &a.related_urls, &a.message).cmp(&(
            &b.url,
            &b.code,
            &b.related_urls,
            &b.message,
        ))
    });
    issues.dedup_by(|a, b| {
        a.url == b.url
            && a.code == b.code
            && a.related_urls == b.related_urls
            && a.message == b.message
    });
    issues.retain(|issue| !existing.contains(&(issue.url.clone(), issue.code.clone())));
    let structured_data: Vec<_> = pages
        .values()
        .copied()
        .filter(|page| !page.structured_data.is_empty() || page.invalid_structured_data > 0)
        .map(structured_facts)
        .collect();
    json!({
        "schema_version": 1,
        "scope": "observations_in_supplied_crawl_report",
        "coverage": {
            "observed_urls": pages.len(), "pending": report.summary.pending,
            "limited": report.summary.limited, "cancelled": report.summary.cancelled,
            "truncated_pages": pages.values().filter(|page| page.truncated).count(),
            "site_wide_completeness": "not_proven",
            "note": "Unobserved targets and zero observed inlinks are crawl-scope facts, not proof about the complete website."
        },
        "graph": analyze_graph(report, &pages),
        "canonical_cycles": cycles,
        "structured_data": structured_data,
        "issues": issues,
        "limitations": [
            "The graph contains observed internal URLs and anchor links only; unknown destinations are listed separately.",
            "Link scores use an independent PageRank-style calculation and do not claim equality with a vendor score.",
            "Security checks inspect header presence/selected values and insecure resource references; policy correctness is not proven.",
            "Structured-data output contains JSON-LD parse/type facts only; Schema.org and Google rich-result validation are not performed.",
            "No near-duplicate or semantic content analysis is inferred from page metadata."
        ]
    })
}

fn usable_html(page: &PageRecord) -> bool {
    page.is_html()
        && page.final_status.or(page.status) == Some(200)
        && page.error.is_none()
        && !page.blocked_by_robots
}

fn final_address(page: &PageRecord) -> &str {
    if page.final_url.is_empty() {
        &page.url
    } else {
        &page.final_url
    }
}

fn scheme_is(url: &str, scheme: &str) -> bool {
    url.get(..scheme.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(scheme))
}

fn has_directive(page: &PageRecord, wanted: &str) -> bool {
    page.meta_robots
        .iter()
        .chain(&page.x_robots_tag)
        .any(|value| {
            value
                .split(|c: char| c == ',' || c == ':' || c.is_whitespace())
                .any(|token| {
                    token.eq_ignore_ascii_case(wanted) || token.eq_ignore_ascii_case("none")
                })
        })
}

fn issue(
    issues: &mut Vec<AuditIssue>,
    page: &PageRecord,
    code: &str,
    severity: Severity,
    message: impl Into<String>,
    related_urls: Vec<String>,
) {
    let related_urls = related_urls
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    issues.push(AuditIssue {
        code: code.into(),
        severity,
        url: page.url.clone(),
        message: message.into(),
        related_urls,
    });
}

fn target_status(page: &PageRecord, address: &str) -> Option<u16> {
    if page.url == address {
        page.status
    } else {
        page.final_status
    }
}

fn target_redirects(page: &PageRecord, address: &str) -> bool {
    page.url == address
        && (page
            .status
            .is_some_and(|status| (300..400).contains(&status))
            || !page.redirect_chain.is_empty()
            || (!page.final_url.is_empty() && page.final_url != page.url))
}

fn is_same_page(page: &PageRecord, address: &str) -> bool {
    address == page.url || address == final_address(page)
}

fn analyze_target(
    source: &PageRecord,
    address: &str,
    prefix: &str,
    lookup: &BTreeMap<&str, &PageRecord>,
    issues: &mut Vec<AuditIssue>,
) {
    let Some(target) = lookup.get(address).copied() else {
        issue(
            issues,
            source,
            &format!("{prefix}_target_not_observed"),
            Severity::Notice,
            "Target was not observed in this crawl; its HTTP and indexability state are unknown",
            vec![address.into()],
        );
        return;
    };
    if target.blocked_by_robots {
        issue(
            issues,
            source,
            &format!("{prefix}_target_blocked"),
            Severity::Warning,
            "Observed target is blocked by the configured robots policy",
            vec![address.into()],
        );
    }
    if target.error.is_some() {
        issue(
            issues,
            source,
            &format!("{prefix}_target_failed"),
            Severity::Warning,
            "Observed target request or response failed",
            vec![address.into()],
        );
    }
    if target_redirects(target, address) {
        issue(
            issues,
            source,
            &format!("{prefix}_target_redirect"),
            Severity::Warning,
            "Declared target redirects instead of directly serving its final document",
            vec![address.into(), final_address(target).into()],
        );
    }
    if let Some(status) = target_status(target, address)
        && status != 200
        && !(300..400).contains(&status)
    {
        issue(
            issues,
            source,
            &format!("{prefix}_target_non200"),
            Severity::Warning,
            format!("Observed target returned HTTP {status}"),
            vec![address.into()],
        );
    }
    if has_directive(target, "noindex") {
        issue(
            issues,
            source,
            &format!("{prefix}_target_noindex"),
            Severity::Warning,
            "Target response contains an observed noindex directive",
            vec![address.into()],
        );
    }
    let canonicalized: Vec<_> = target
        .canonical
        .iter()
        .filter(|canonical| !is_same_page(target, canonical))
        .cloned()
        .collect();
    if !canonicalized.is_empty() {
        let mut related = vec![address.into()];
        related.extend(canonicalized);
        issue(
            issues,
            source,
            &format!("{prefix}_target_canonicalized"),
            Severity::Warning,
            "Observed target declares a different canonical URL",
            related,
        );
    }
}

fn analyze_relationships(
    page: &PageRecord,
    lookup: &BTreeMap<&str, &PageRecord>,
    issues: &mut Vec<AuditIssue>,
) {
    let canonicals: BTreeSet<_> = page.canonical.iter().map(String::as_str).collect();
    if canonicals.len() > 1 {
        issue(
            issues,
            page,
            "canonical_conflict",
            Severity::Warning,
            "Multiple distinct canonical targets were observed",
            page.canonical.clone(),
        );
    }
    for canonical in canonicals {
        if !is_same_page(page, canonical) {
            analyze_target(page, canonical, "canonical", lookup, issues);
        }
    }
    if page.hreflang.is_empty() {
        return;
    }
    if !page.truncated
        && !page
            .hreflang
            .iter()
            .any(|alternate| is_same_page(page, &alternate.url))
    {
        issue(
            issues,
            page,
            "hreflang_self_missing",
            Severity::Warning,
            "Observed hreflang declarations do not include this page itself",
            vec![],
        );
    }
    let mut languages: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for alternate in &page.hreflang {
        languages
            .entry(alternate.language.trim().to_ascii_lowercase())
            .or_default()
            .insert(alternate.url.clone());
    }
    for (language, targets) in &languages {
        if targets.len() > 1 {
            issue(
                issues,
                page,
                "hreflang_conflict",
                Severity::Warning,
                format!("Hreflang {language} declares multiple distinct URLs"),
                targets.iter().cloned().collect(),
            );
        }
    }
    let targets: BTreeSet<_> = page
        .hreflang
        .iter()
        .map(|alternate| alternate.url.as_str())
        .collect();
    for address in targets {
        if is_same_page(page, address) {
            continue;
        }
        analyze_target(page, address, "hreflang", lookup, issues);
        if let Some(target) = lookup.get(address).copied()
            && usable_html(target)
            && !target.truncated
            && !target
                .hreflang
                .iter()
                .any(|alternate| is_same_page(page, &alternate.url))
        {
            issue(
                issues,
                page,
                "hreflang_return_missing",
                Severity::Warning,
                "Observed alternate document has no return hreflang link",
                vec![address.into()],
            );
        }
    }
}

fn canonical_cycles(
    pages: &BTreeMap<&str, &PageRecord>,
    lookup: &BTreeMap<&str, &PageRecord>,
) -> Vec<Vec<String>> {
    let mut next = BTreeMap::new();
    for page in pages.values().copied().filter(|page| usable_html(page)) {
        let targets: BTreeSet<_> = page.canonical.iter().map(String::as_str).collect();
        if targets.len() == 1 {
            let target = *targets.iter().next().unwrap();
            if !is_same_page(page, target)
                && let Some(observed) = lookup.get(target)
                && usable_html(observed)
            {
                next.insert(page.url.as_str(), observed.url.as_str());
            }
        }
    }
    let mut finished = BTreeSet::new();
    let mut cycles = Vec::new();
    for &start in next.keys() {
        if finished.contains(start) {
            continue;
        }
        let mut positions = BTreeMap::new();
        let mut path = Vec::new();
        let mut current = start;
        while !finished.contains(current) {
            if let Some(&position) = positions.get(current) {
                let mut cycle: Vec<_> = path[position..]
                    .iter()
                    .map(|url: &&str| (*url).to_string())
                    .collect();
                cycle.sort();
                cycles.push(cycle);
                break;
            }
            positions.insert(current, path.len());
            path.push(current);
            let Some(&target) = next.get(current) else {
                break;
            };
            current = target;
        }
        finished.extend(path);
    }
    cycles.sort();
    cycles
}

fn header<'a>(page: &'a PageRecord, name: &str) -> Option<&'a str> {
    page.headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn analyze_security(page: &PageRecord, issues: &mut Vec<AuditIssue>) {
    if !scheme_is(final_address(page), "https://") {
        return;
    }
    for (name, code) in [
        ("strict-transport-security", "security_hsts_missing"),
        ("content-security-policy", "security_csp_missing"),
    ] {
        if header(page, name).is_none_or(|value| value.trim().is_empty()) {
            issue(
                issues,
                page,
                code,
                Severity::Notice,
                format!("HTTPS response has no observed {name} header"),
                vec![],
            );
        }
    }
    if !header(page, "x-content-type-options")
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("nosniff"))
    {
        issue(
            issues,
            page,
            "security_nosniff_missing",
            Severity::Notice,
            "HTTPS response has no observed X-Content-Type-Options: nosniff",
            vec![],
        );
    }
    let xframe = header(page, "x-frame-options").is_some_and(|value| {
        value.trim().eq_ignore_ascii_case("deny") || value.trim().eq_ignore_ascii_case("sameorigin")
    });
    let frame_ancestors = header(page, "content-security-policy").is_some_and(|value| {
        value.split(';').any(|directive| {
            directive
                .split_whitespace()
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case("frame-ancestors"))
        })
    });
    if !xframe && !frame_ancestors {
        issue(
            issues,
            page,
            "security_frame_policy_missing",
            Severity::Notice,
            "No observed X-Frame-Options policy or CSP frame-ancestors directive; review embedding policy",
            vec![],
        );
    }
    if header(page, "referrer-policy").is_none_or(|value| value.trim().is_empty()) {
        issue(
            issues,
            page,
            "security_referrer_policy_missing",
            Severity::Notice,
            "HTTPS response has no observed Referrer-Policy header",
            vec![],
        );
    }
    let mixed: Vec<_> = page
        .links
        .iter()
        .filter(|link| link.kind != "anchor" && scheme_is(&link.url, "http://"))
        .map(|link| link.url.clone())
        .chain(
            page.images
                .iter()
                .filter(|image| scheme_is(&image.url, "http://"))
                .map(|image| image.url.clone()),
        )
        .collect();
    if !mixed.is_empty() {
        issue(
            issues,
            page,
            "mixed_content",
            Severity::Warning,
            "HTTPS response references unencrypted subresources; this observation does not prove browser loading",
            mixed,
        );
    }
    // Navigating to an HTTP anchor is different from loading mixed subresources.
    let insecure_links: Vec<_> = page
        .links
        .iter()
        .filter(|link| link.kind == "anchor" && scheme_is(&link.url, "http://"))
        .map(|link| link.url.clone())
        .collect();
    if !insecure_links.is_empty() {
        issue(
            issues,
            page,
            "http_anchor_links",
            Severity::Notice,
            "HTTPS page links to HTTP destinations; these are navigation links, not mixed-content resources",
            insecure_links,
        );
    }
}

fn analyze_graph(report: &CrawlReport, pages: &BTreeMap<&str, &PageRecord>) -> Value {
    let internal: BTreeMap<_, _> = pages
        .iter()
        .filter(|(_, page)| page.internal)
        .map(|(&url, &page)| (url, page))
        .collect();
    let urls: Vec<_> = internal.keys().copied().collect();
    let indices: BTreeMap<_, _> = urls
        .iter()
        .enumerate()
        .map(|(index, &url)| (url, index))
        .collect();
    let mut edges: BTreeMap<(&str, &str), (usize, bool)> = BTreeMap::new();
    let mut unknown = BTreeSet::new();
    for (&source, page) in &internal {
        let page_nofollow = has_directive(page, "nofollow");
        for link in page.links.iter().filter(|link| link.kind == "anchor") {
            if internal.contains_key(link.url.as_str()) {
                let edge = edges
                    .entry((source, link.url.as_str()))
                    .or_insert((0, false));
                edge.0 += 1;
                let blocked = link.nofollow
                    || link.rel.split_whitespace().any(|rel| {
                        ["nofollow", "ugc", "sponsored"]
                            .iter()
                            .any(|token| rel.eq_ignore_ascii_case(token))
                    });
                edge.1 |= !blocked && !page_nofollow;
            } else if link.internal {
                unknown.insert(link.url.clone());
            }
        }
    }
    let mut incoming = vec![0usize; urls.len()];
    let mut outgoing = vec![0usize; urls.len()];
    let mut in_occurrences = vec![0usize; urls.len()];
    let mut out_occurrences = vec![0usize; urls.len()];
    let mut rank_adjacency = vec![Vec::new(); urls.len()];
    let mut all_adjacency = vec![Vec::new(); urls.len()];
    for (&(source, target), &(count, follow)) in &edges {
        let from = indices[source];
        let to = indices[target];
        outgoing[from] += 1;
        incoming[to] += 1;
        out_occurrences[from] += count;
        in_occurrences[to] += count;
        all_adjacency[from].push(to);
        if follow {
            rank_adjacency[from].push(to);
        }
    }
    let (rank, iterations, residual) = page_rank(&rank_adjacency);
    let largest = rank.iter().copied().fold(0.0_f64, f64::max);
    let mut shortest = vec![None; urls.len()];
    let seeds: BTreeSet<_> = report.config.seed_urls.iter().map(String::as_str).collect();
    let mut queue = VecDeque::new();
    for (index, &url) in urls.iter().enumerate() {
        if seeds.contains(url) || seeds.contains(final_address(internal[url])) {
            shortest[index] = Some(0usize);
            queue.push_back(index);
        }
    }
    while let Some(from) = queue.pop_front() {
        let depth = shortest[from].unwrap() + 1;
        for &to in &all_adjacency[from] {
            if shortest[to].is_none() {
                shortest[to] = Some(depth);
                queue.push_back(to);
            }
        }
    }
    let mut depth_distribution: BTreeMap<usize, usize> = BTreeMap::new();
    let mut orphan_candidates = Vec::new();
    let mut nodes = Vec::new();
    for (index, &url) in urls.iter().enumerate() {
        let page = internal[url];
        *depth_distribution.entry(page.depth).or_default() += 1;
        if incoming[index] == 0
            && usable_html(page)
            && !seeds.contains(url)
            && !seeds.contains(final_address(page))
        {
            orphan_candidates.push(json!({"url": url,
                "reason": "No internal anchor inlinks observed in supplied crawl; not a site-wide orphan assertion"}));
        }
        nodes.push(json!({"url": url, "title": page.title(), "status": page.status,
            "indexable": page.indexable, "crawl_depth": page.depth,
            "shortest_link_depth": shortest[index],
            "in_degree": incoming[index], "out_degree": outgoing[index],
            "inlink_occurrences": in_occurrences[index], "outlink_occurrences": out_occurrences[index],
            "page_rank": rank[index],
            "link_score": if largest > 0.0 { rank[index] / largest * 100.0 } else { 0.0 }
        }));
    }
    let edge_values: Vec<_> = edges.iter().map(|(&(source, target), &(count, follow))| {
        json!({"source": source, "target": target, "occurrences": count, "passes_score": follow})
    }).collect();
    json!({"nodes": nodes, "edges": edge_values, "depth_distribution": depth_distribution,
        "orphan_candidates": orphan_candidates, "unobserved_internal_targets": unknown,
        "algorithm": {"name": "independent_observed_anchor_pagerank", "damping": DAMPING,
            "iterations": iterations, "residual_l1": residual, "tolerance_l1": TOLERANCE,
            "converged": residual <= TOLERANCE, "max_iterations": MAX_ITERATIONS,
            "normalization": "maximum_rank_maps_to_100",
            "note": "Unique followable target URLs; dangling rank redistributed uniformly. Duplicate anchors do not amplify score."}})
}

fn page_rank(adjacency: &[Vec<usize>]) -> (Vec<f64>, usize, f64) {
    let n = adjacency.len();
    if n == 0 {
        return (Vec::new(), 0, 0.0);
    }
    let nf = n as f64;
    let mut rank = vec![1.0 / nf; n];
    let mut residual = 0.0;
    for iteration in 1..=MAX_ITERATIONS {
        let dangling: f64 = adjacency
            .iter()
            .enumerate()
            .filter(|(_, targets)| targets.is_empty())
            .map(|(index, _)| rank[index])
            .sum();
        let mut next = vec![(1.0 - DAMPING) / nf + DAMPING * dangling / nf; n];
        for (source, targets) in adjacency.iter().enumerate() {
            if !targets.is_empty() {
                let contribution = DAMPING * rank[source] / targets.len() as f64;
                for &target in targets {
                    next[target] += contribution;
                }
            }
        }
        residual = next.iter().zip(&rank).map(|(a, b)| (a - b).abs()).sum();
        rank = next;
        if residual <= TOLERANCE {
            return (rank, iteration, residual);
        }
    }
    (rank, MAX_ITERATIONS, residual)
}

fn structured_facts(page: &PageRecord) -> Value {
    let mut types: BTreeMap<String, usize> = BTreeMap::new();
    let mut nodes = Vec::new();
    let mut nonstring_type_values = 0usize;
    for (index, block) in page.structured_data.iter().enumerate() {
        collect_types(
            block,
            format!("/blocks/{index}"),
            &mut types,
            &mut nodes,
            &mut nonstring_type_values,
        );
    }
    json!({"url": page.url, "parsed_blocks": page.structured_data.len(),
        "invalid_json_blocks": page.invalid_structured_data, "types": types,
        "typed_nodes": nodes, "nonstring_type_values": nonstring_type_values,
        "schema_validation": "not_performed", "google_rich_result_validation": "not_performed"})
}

fn collect_types(
    value: &Value,
    path: String,
    counts: &mut BTreeMap<String, usize>,
    nodes: &mut Vec<Value>,
    invalid: &mut usize,
) {
    match value {
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                collect_types(value, format!("{path}/{index}"), counts, nodes, invalid);
            }
        }
        Value::Object(object) => {
            if let Some(value) = object.get("@type") {
                let mut types = Vec::new();
                match value {
                    Value::String(kind) => types.push(kind.clone()),
                    Value::Array(values) => {
                        for value in values {
                            if let Some(kind) = value.as_str() {
                                types.push(kind.into());
                            } else {
                                *invalid += 1;
                            }
                        }
                    }
                    _ => *invalid += 1,
                }
                for kind in &types {
                    *counts.entry(kind.clone()).or_default() += 1;
                }
                nodes.push(json!({"path": path, "types": types, "id": object.get("@id")}));
            }
            for (key, value) in object {
                // Type coercion inside a context is a definition, not a typed node.
                if key != "@type" && key != "@context" {
                    let escaped = key.replace('~', "~0").replace('/', "~1");
                    collect_types(value, format!("{path}/{escaped}"), counts, nodes, invalid);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use open_seo_spider_core::{
        CrawlConfig, CrawlSummary, HreflangRecord, ImageRecord, LinkRecord,
    };

    fn page(path: &str) -> PageRecord {
        let url = format!("https://fixture.test/{path}");
        PageRecord {
            url: url.clone(),
            final_url: url,
            status: Some(200),
            final_status: Some(200),
            content_type: "text/html".into(),
            internal: true,
            indexable: true,
            ..Default::default()
        }
    }
    fn anchor(path: &str) -> LinkRecord {
        LinkRecord {
            url: format!("https://fixture.test/{path}"),
            kind: "anchor".into(),
            internal: true,
            ..Default::default()
        }
    }
    fn alternate(language: &str, path: &str) -> HreflangRecord {
        HreflangRecord {
            language: language.into(),
            url: format!("https://fixture.test/{path}"),
        }
    }
    fn report(pages: Vec<PageRecord>) -> CrawlReport {
        CrawlReport {
            config: CrawlConfig {
                seed_urls: vec!["https://fixture.test/a".into()],
                ..Default::default()
            },
            pages,
            issues: vec![],
            summary: CrawlSummary::default(),
            warnings: vec![],
        }
    }
    fn has_issue(value: &Value, code: &str, path: &str) -> bool {
        value["issues"].as_array().unwrap().iter().any(|issue| {
            issue["code"] == code && issue["url"] == format!("https://fixture.test/{path}")
        })
    }

    #[test]
    fn graph_separates_occurrences_resources_and_followable_rank() {
        let mut a = page("a");
        a.links = vec![anchor("b"), anchor("b")];
        a.links.push(LinkRecord {
            url: page("c").url,
            kind: "image".into(),
            internal: true,
            ..Default::default()
        });
        let mut b = page("b");
        b.links.push(anchor("a"));
        b.depth = 1;
        let c = page("c");
        let snapshot = report(vec![a, b, c]);
        let value = analyze_report(&snapshot);
        assert_eq!(value["graph"]["edges"].as_array().unwrap().len(), 2);
        let nodes = value["graph"]["nodes"].as_array().unwrap();
        assert_eq!(nodes[0]["out_degree"], 1);
        assert_eq!(nodes[0]["outlink_occurrences"], 2);
        assert_eq!(nodes[1]["inlink_occurrences"], 2);
        assert_eq!(nodes[1]["shortest_link_depth"], 1);
        let sum: f64 = nodes
            .iter()
            .map(|node| node["page_rank"].as_f64().unwrap())
            .sum();
        assert!((sum - 1.0).abs() < 1e-10);
        assert!(
            (nodes[0]["page_rank"].as_f64().unwrap() - nodes[1]["page_rank"].as_f64().unwrap())
                .abs()
                < 1e-10
        );
        assert!(
            nodes[0]["link_score"].as_f64().unwrap() > nodes[2]["link_score"].as_f64().unwrap()
        );
        assert_eq!(
            value["graph"]["orphan_candidates"][0]["url"],
            "https://fixture.test/c"
        );
        let mut reversed = snapshot;
        reversed.pages.reverse();
        assert_eq!(value, analyze_report(&reversed));
    }

    #[test]
    fn pagerank_handles_empty_dangling_and_nofollow_graphs() {
        assert_eq!(page_rank(&[]).0, Vec::<f64>::new());
        let (scores, _, residual) = page_rank(&[vec![], vec![]]);
        assert_eq!(scores, vec![0.5, 0.5]);
        assert!(residual <= TOLERANCE);
        let mut a = page("a");
        let mut link = anchor("b");
        link.nofollow = true;
        a.links.push(link);
        let value = analyze_report(&report(vec![a, page("b")]));
        assert_eq!(value["graph"]["edges"][0]["passes_score"], false);
        assert_eq!(value["graph"]["nodes"][0]["page_rank"], 0.5);
    }

    #[test]
    fn canonical_cycles_and_unobserved_targets_have_distinct_evidence() {
        let mut a = page("a");
        a.canonical.push(page("b").url);
        let mut b = page("b");
        b.canonical.push(page("a").url);
        let mut c = page("c");
        c.canonical.push(page("unobserved").url);
        let value = analyze_report(&report(vec![a, b, c]));
        assert_eq!(value["canonical_cycles"].as_array().unwrap().len(), 1);
        assert!(has_issue(&value, "canonical_loop", "a"));
        assert!(has_issue(&value, "canonical_loop", "b"));
        assert!(has_issue(&value, "canonical_target_not_observed", "c"));
        assert!(!has_issue(&value, "canonical_target_non200", "c"));
    }

    #[test]
    fn relationship_targets_keep_initial_redirect_and_noindex_facts() {
        let mut a = page("a");
        a.canonical.push(page("b").url);
        let mut b = page("b");
        b.status = Some(301);
        b.final_url = page("landing").url;
        b.meta_robots.push("noindex,follow".into());
        let value = analyze_report(&report(vec![a, b]));
        assert!(has_issue(&value, "canonical_target_redirect", "a"));
        assert!(has_issue(&value, "canonical_target_noindex", "a"));
        assert!(!has_issue(&value, "canonical_target_non200", "a"));
        let mut alias_source = page("x");
        alias_source.canonical.push(page("landing").url);
        let redirected = PageRecord {
            url: page("old").url,
            final_url: page("landing").url,
            status: Some(301),
            final_status: Some(200),
            content_type: "text/html".into(),
            internal: true,
            ..Default::default()
        };
        let value = analyze_report(&report(vec![alias_source, redirected]));
        assert!(!has_issue(&value, "canonical_target_redirect", "x"));
    }

    #[test]
    fn hreflang_checks_conflicts_self_and_fetched_return_links() {
        let mut a = page("a");
        a.hreflang = vec![alternate("de", "b"), alternate("DE", "c")];
        let mut b = page("b");
        b.meta_robots.push("googlebot:noindex".into());
        b.canonical.push(page("c").url);
        let mut c = page("c");
        c.truncated = true;
        let value = analyze_report(&report(vec![a, b, c]));
        for code in [
            "hreflang_self_missing",
            "hreflang_conflict",
            "hreflang_target_noindex",
            "hreflang_target_canonicalized",
            "hreflang_return_missing",
        ] {
            assert!(has_issue(&value, code, "a"), "missing {code}");
        }
        let missing_return: Vec<_> = value["issues"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|issue| issue["code"] == "hreflang_return_missing")
            .collect();
        assert_eq!(missing_return.len(), 1);
        assert_eq!(
            missing_return[0]["related_urls"][0],
            "https://fixture.test/b"
        );
    }

    #[test]
    fn security_distinguishes_http_navigation_and_resources() {
        let mut a = page("a");
        for (key, value) in [
            ("Strict-Transport-Security", "max-age=31536000"),
            (
                "Content-Security-Policy",
                "default-src 'self'; frame-ancestors 'none'",
            ),
            ("X-Content-Type-Options", "nosniff"),
            ("Referrer-Policy", "strict-origin"),
        ] {
            a.headers.insert(key.into(), value.into());
        }
        a.links.push(LinkRecord {
            url: "http://fixture.test/navigation".into(),
            kind: "anchor".into(),
            ..Default::default()
        });
        a.images.push(ImageRecord {
            url: "http://fixture.test/image.png".into(),
            ..Default::default()
        });
        let value = analyze_report(&report(vec![a]));
        assert!(has_issue(&value, "mixed_content", "a"));
        assert!(has_issue(&value, "http_anchor_links", "a"));
        assert!(!has_issue(&value, "security_frame_policy_missing", "a"));
        assert!(!has_issue(&value, "security_hsts_missing", "a"));
        let mixed = value["issues"]
            .as_array()
            .unwrap()
            .iter()
            .find(|issue| issue["code"] == "mixed_content")
            .unwrap();
        assert_eq!(
            mixed["related_urls"],
            json!(["http://fixture.test/image.png"])
        );
    }

    #[test]
    fn jsonld_reports_nested_type_facts_without_claiming_validation() {
        let mut a = page("a");
        a.invalid_structured_data = 2;
        a.structured_data
            .push(json!({"@context": {"url": {"@type": "@id"}}, "@graph": [
                {"@type": "Organization", "@id": "#org"},
                {"@type": ["WebPage", "Article"], "author": {"@type": "Person"}}
            ]}));
        let value = analyze_report(&report(vec![a]));
        let facts = &value["structured_data"][0];
        assert_eq!(facts["types"]["Person"], 1);
        assert_eq!(facts["types"]["@id"], Value::Null);
        assert_eq!(facts["typed_nodes"].as_array().unwrap().len(), 3);
        assert_eq!(facts["invalid_json_blocks"], 2);
        assert_eq!(facts["schema_validation"], "not_performed");
        assert_eq!(facts["google_rich_result_validation"], "not_performed");
    }

    #[test]
    fn existing_report_issues_are_not_emitted_twice_and_partial_scope_is_explicit() {
        let mut a = page("a");
        a.hreflang.push(alternate("de", "b"));
        let mut snapshot = report(vec![a, page("b")]);
        snapshot.summary.limited = true;
        snapshot.summary.pending = 4;
        snapshot.issues.push(AuditIssue {
            code: "hreflang_return_missing".into(),
            severity: Severity::Warning,
            url: "https://fixture.test/a".into(),
            message: "Existing issue".into(),
            related_urls: vec![],
        });
        let value = analyze_report(&snapshot);
        assert!(!has_issue(&value, "hreflang_return_missing", "a"));
        assert_eq!(value["coverage"]["limited"], true);
        assert_eq!(value["coverage"]["pending"], 4);
        assert_eq!(value["coverage"]["site_wide_completeness"], "not_proven");
        assert!(
            !value["graph"]["orphan_candidates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|candidate| candidate["url"] == "https://fixture.test/a")
        );
    }
}
