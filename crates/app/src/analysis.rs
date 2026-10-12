//! Deterministic analysis of already captured crawl observations.
//! No network requests, provider access, or proprietary rule sets are used.
use open_seo_spider_core::{
    AuditIssue, CrawlReport, FragmentStatus, PageRecord, Severity, applicable_robot_directives,
    audit_dom_metadata, normalize_url, validate_fragments,
};
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
        if let Some(address) = page
            .rendered_url
            .as_deref()
            .filter(|_| page.rendered && page.renderer_error.is_none())
        {
            lookup.entry(address).or_insert(*page);
        }
    }
    let mut issues = Vec::new();
    let existing: BTreeSet<_> = report
        .issues
        .iter()
        .map(|issue| {
            (
                issue.url.clone(),
                issue.code.clone(),
                issue.related_urls.iter().cloned().collect::<BTreeSet<_>>(),
            )
        })
        .collect();
    for page in pages.values().copied().filter(|page| usable_document(page)) {
        analyze_relationships(page, &lookup, &mut issues);
        if page.is_html() {
            analyze_security(page, &mut issues);
        }
    }
    issues.extend(audit_dom_metadata(&report.pages));
    let fragments = validate_fragments(report);
    for fragment in &fragments {
        if fragment.status == FragmentStatus::Missing
            && let Some(page) = pages.get(fragment.source_url.as_str())
            && page.internal
            && usable_html(page)
        {
            issue(
                &mut issues,
                page,
                "broken_bookmark",
                Severity::Warning,
                format!(
                    "Declared fragment {:?} has no matching ID or legacy named anchor in the complete captured target document; this is a snapshot observation",
                    fragment.fragment
                ),
                vec![fragment.resolved_href.clone()],
            );
        }
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
    let graph = analyze_graph(report, &pages);
    if let Some(candidates) = graph["orphan_candidates"].as_array() {
        for candidate in candidates {
            if let Some(page) = candidate["url"].as_str().and_then(|url| pages.get(url)) {
                issue(
                    &mut issues,
                    page,
                    "crawl_orphan_candidate",
                    Severity::Notice,
                    "No incoming anchor from another captured internal document was observed; this is a crawl-bounded candidate, not proof of a site-wide orphan",
                    page.discovered_from.iter().cloned().collect(),
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
    issues.retain(|issue| {
        !existing.contains(&(
            issue.url.clone(),
            issue.code.clone(),
            issue.related_urls.iter().cloned().collect(),
        )) && !existing.contains(&(issue.url.clone(), issue.code.clone(), BTreeSet::new()))
    });
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
            "renderer_failed_pages": pages.values().filter(|page| page.renderer_error.is_some()).count(),
            "site_wide_completeness": "not_proven",
            "note": "Unobserved targets and zero observed inlinks are crawl-scope facts, not proof about the complete website."
        },
        "graph": graph,
        "canonical_cycles": cycles,
        "structured_data": structured_data,
        "fragments": {"enabled":report.config.validate_fragments,"observations":fragments,"basis":"captured_declared_anchor_fragments","unknown_is_not_missing":true},
        "dom_rules": {"basis":"repaired_HTML5_tree_observations","headings":"tag levels and first-heading order; computed accessibility outlines and CSS visibility are not inferred","meta_keywords":"declaration and exact duplicate review observations, not a search ranking recommendation"},
        "issues": issues,
        "relationship_rules": {
            "basis": "observed_declarations_and_captured_target_responses",
            "hreflang_codes": "ISO_639_1_language_optional_ISO_15924_script_optional_ISO_3166_1_alpha2_region_or_x_default",
            "code_list_snapshot": "2026-10-12",
            "x_default": "recommended_fallback_not_required",
            "pagination": "declared_navigation_relationships; reciprocity_is_a_consistency_heuristic_not_a_Google_indexing_requirement",
            "references": [
                "https://developers.google.com/search/docs/specialty/international/localized-versions",
                "https://developers.google.com/search/docs/crawling-indexing/consolidate-duplicate-urls",
                "https://developers.google.com/search/docs/specialty/ecommerce/pagination-and-incremental-page-loading",
                "https://www.rfc-editor.org/rfc/rfc8288"
            ]
        },
        "limitations": [
            "The graph contains observed internal URLs and anchor links only; unknown destinations are listed separately.",
            "Link scores use an independent PageRank-style calculation and do not claim equality with a vendor score.",
            "Security checks inspect header presence/selected values and insecure resource references; policy correctness is not proven.",
            "Structured-data output contains JSON-LD parse/type facts only; Schema.org and Google rich-result validation are not performed.",
            "No near-duplicate or semantic content analysis is inferred from page metadata.",
            "Hreflang labels are validated against captured declarations and published code lists; detected content language and search-engine acceptance are not proven.",
            "Pagination diagnostics inspect declared next/prev links. Missing tags are not an error; Google no longer uses these tags for indexing."
        ]
    })
}

fn usable_html(page: &PageRecord) -> bool {
    page.is_html() && usable_document(page)
}

fn usable_document(page: &PageRecord) -> bool {
    page.final_status.or(page.status) == Some(200)
        && (!page.rendered || page.rendered_status == Some(200))
        && page.error.is_none()
        && !page.blocked_by_robots
}

fn complete_document(page: &PageRecord) -> bool {
    usable_document(page) && !page.truncated && page.renderer_error.is_none()
}

fn document_address(page: &PageRecord) -> &str {
    if page.document_url().is_empty() {
        &page.url
    } else {
        page.document_url()
    }
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
    applicable_robot_directives(page).iter().any(|value| {
        value
            .split(|c: char| c == ',' || c == ':' || c.is_whitespace())
            .any(|token| token.eq_ignore_ascii_case(wanted) || token.eq_ignore_ascii_case("none"))
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
    } else if page.rendered
        && page.renderer_error.is_none()
        && page.rendered_url.as_deref() == Some(address)
    {
        page.rendered_status
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
    // Metadata belongs to the final captured document, not to a redirecting
    // requested URL. Normalize fragments/default ports consistently with crawl URLs.
    normalize_url(address, None).is_some_and(|target| {
        normalize_url(document_address(page), None).is_some_and(|own| target == own)
    })
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
    if target.truncated
        || target.renderer_error.is_some()
        || (target.rendered && target.rendered_status.is_none())
    {
        issue(
            issues,
            source,
            &format!("{prefix}_target_incomplete"),
            Severity::Notice,
            "Target capture is incomplete; absent declarations and full indexability cannot be established",
            vec![address.into()],
        );
    }
    if target_status(target, address).is_none()
        && !target.blocked_by_robots
        && target.error.is_none()
    {
        issue(
            issues,
            source,
            &format!("{prefix}_target_status_unknown"),
            Severity::Notice,
            "Target is present in the report but has no observed HTTP status",
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
        if let Some(status) = target.final_status
            && status != 200
            && !(300..400).contains(&status)
        {
            issue(
                issues,
                source,
                &format!("{prefix}_target_final_non200"),
                Severity::Warning,
                format!(
                    "Declared target redirects and its observed final response returned HTTP {status}"
                ),
                vec![address.into(), final_address(target).into()],
            );
        }
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
    let html: BTreeSet<_> = page.html_canonical.iter().collect();
    let http: BTreeSet<_> = page.header_canonical.iter().collect();
    if !html.is_empty() && !http.is_empty() && html != http {
        issue(
            issues,
            page,
            "canonical_html_header_conflict",
            Severity::Warning,
            "HTML and HTTP Link header canonical declarations disagree",
            page.html_canonical
                .iter()
                .chain(&page.header_canonical)
                .cloned()
                .collect(),
        );
    }
    for canonical in canonicals {
        if !is_same_page(page, canonical) {
            analyze_target(page, canonical, "canonical", lookup, issues);
            if let Some(target) = lookup.get(canonical) {
                let source_languages = self_languages(page);
                let target_languages = self_languages(target);
                if !source_languages.is_empty()
                    && !target_languages.is_empty()
                    && source_languages.is_disjoint(&target_languages)
                {
                    issue(
                        issues,
                        page,
                        "canonical_hreflang_language_conflict",
                        Severity::Notice,
                        "Source and canonical target declare different self-hreflang languages; review whether a same-language canonical is available (a substitute language can be intentional)",
                        vec![canonical.into()],
                    );
                }
            }
        }
    }
    analyze_pagination(page, lookup, issues);
    if page.hreflang.is_empty() {
        return;
    }
    if complete_document(page)
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
        if let Some((code, reason)) = invalid_hreflang(&alternate.language) {
            issue(
                issues,
                page,
                code,
                Severity::Warning,
                format!("Hreflang {:?}: {reason}", alternate.language),
                vec![alternate.url.clone()],
            );
        }
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
            if language == "x-default" {
                issue(
                    issues,
                    page,
                    "hreflang_x_default_conflict",
                    Severity::Warning,
                    "The fallback x-default declaration points to multiple distinct URLs",
                    targets.iter().cloned().collect(),
                );
            }
        }
    }
    if complete_document(page)
        && !languages.contains_key("x-default")
        && languages
            .keys()
            .filter(|language| invalid_hreflang(language).is_none())
            .count()
            > 1
    {
        issue(
            issues,
            page,
            "hreflang_x_default_missing",
            Severity::Notice,
            "No x-default fallback was observed in this multilingual set; it is recommended, not required",
            vec![],
        );
    }
    let html_locales = hreflang_map(&page.html_hreflang);
    let header_locales = hreflang_map(&page.header_hreflang);
    for (language, html_urls) in &html_locales {
        if let Some(header_urls) = header_locales.get(language)
            && html_urls != header_urls
        {
            issue(
                issues,
                page,
                "hreflang_html_header_conflict",
                Severity::Warning,
                format!("HTML and HTTP Link hreflang declarations disagree for {language}"),
                html_urls.union(header_urls).cloned().collect(),
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
        if let Some(target) = lookup.get(address).copied() {
            let returns: Vec<_> = target
                .hreflang
                .iter()
                .filter(|alternate| is_same_page(page, &alternate.url))
                .collect();
            if complete_document(target) && returns.is_empty() {
                issue(
                    issues,
                    page,
                    "hreflang_return_missing",
                    Severity::Warning,
                    "Observed complete alternate document has no return hreflang link",
                    vec![address.into()],
                );
            }
            let self_labels: BTreeSet<_> = page
                .hreflang
                .iter()
                .filter(|alternate| {
                    is_same_page(page, &alternate.url)
                        && invalid_hreflang(&alternate.language).is_none()
                })
                .map(|alternate| alternate.language.trim().to_ascii_lowercase())
                .collect();
            let return_labels: BTreeSet<_> = returns
                .iter()
                .filter(|alternate| invalid_hreflang(&alternate.language).is_none())
                .map(|alternate| alternate.language.trim().to_ascii_lowercase())
                .collect();
            if !self_labels.is_empty()
                && !return_labels.is_empty()
                && self_labels.is_disjoint(&return_labels)
            {
                issue(
                    issues,
                    page,
                    "hreflang_return_language_conflict",
                    Severity::Warning,
                    format!(
                        "Alternate return labels {:?} disagree with this document's self labels {:?}; this compares declarations, not detected content language",
                        return_labels, self_labels
                    ),
                    vec![address.into()],
                );
            }
            let target_locales = hreflang_map(&target.hreflang);
            for (language, source_urls) in &languages {
                if let Some(target_urls) = target_locales.get(language)
                    && source_urls.is_disjoint(target_urls)
                {
                    issue(
                        issues,
                        page,
                        "hreflang_cluster_conflict",
                        Severity::Warning,
                        format!(
                            "This document and the observed alternate assign hreflang {language} to different URLs"
                        ),
                        source_urls
                            .union(target_urls)
                            .cloned()
                            .chain(std::iter::once(address.into()))
                            .collect(),
                    );
                }
            }
        }
    }
}

fn hreflang_map(
    records: &[open_seo_spider_core::HreflangRecord],
) -> BTreeMap<String, BTreeSet<String>> {
    let mut locales: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for record in records {
        locales
            .entry(record.language.trim().to_ascii_lowercase())
            .or_default()
            .insert(record.url.clone());
    }
    locales
}

fn self_languages(page: &PageRecord) -> BTreeSet<String> {
    page.hreflang
        .iter()
        .filter(|alternate| {
            is_same_page(page, &alternate.url)
                && invalid_hreflang(&alternate.language).is_none()
                && !alternate.language.trim().eq_ignore_ascii_case("x-default")
        })
        .map(|alternate| {
            alternate
                .language
                .trim()
                .split('-')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase()
        })
        .collect()
}

// Published code snapshots checked 2026-10-12. Do not confuse unrestricted BCP47
// tags with Google's documented ISO 639-1 / script / ISO alpha-2 hreflang profile.
// Languages: https://www.loc.gov/standards/iso639-2/php/code_list.php (183 active).
const LANGUAGES: &str = "aa ab ae af ak am an ar as av ay az ba be bg bi bm bn bo br bs ca ce ch co cr cs cu cv cy da de dv dz ee el en eo es et eu fa ff fi fj fo fr fy ga gd gl gn gu gv ha he hi ho hr ht hu hy hz ia id ie ig ii ik io is it iu ja jv ka kg ki kj kk kl km kn ko kr ks ku kv kw ky la lb lg li ln lo lt lu lv mg mh mi mk ml mn mr ms mt my na nb nd ne ng nl nn no nr nv ny oc oj om or os pa pi pl ps pt qu rm rn ro ru rw sa sc sd se sg si sk sl sm sn so sq sr ss st su sv sw ta te tg th ti tk tl tn to tr ts tt tw ty ug uk ur uz ve vi vo wa wo xh yi yo za zh zu";
// ISO 3166-1 assigned alpha-2 set; IANA registry File-Date 2026-09-17,
// excluding private-use, deprecated and exceptionally reserved region entries.
// https://www.iana.org/assignments/language-subtag-registry/language-subtag-registry
const REGIONS: &str = "ad ae af ag ai al am ao aq ar as at au aw ax az ba bb bd be bf bg bh bi bj bl bm bn bo bq br bs bt bv bw by bz ca cc cd cf cg ch ci ck cl cm cn co cr cu cv cw cx cy cz de dj dk dm do dz ec ee eg eh er es et fi fj fk fm fo fr ga gb gd ge gf gg gh gi gl gm gn gp gq gr gs gt gu gw gy hk hm hn hr ht hu id ie il im in io iq ir is it je jm jo jp ke kg kh ki km kn kp kr kw ky kz la lb lc li lk lr ls lt lu lv ly ma mc md me mf mg mh mk ml mm mn mo mp mq mr ms mt mu mv mw mx my mz na nc ne nf ng ni nl no np nr nu nz om pa pe pf pg ph pk pl pm pn pr ps pt pw py qa re ro rs ru rw sa sb sc sd se sg sh si sj sk sl sm sn so sr ss st sv sx sy sz tc td tf tg th tj tk tl tm tn to tr tt tv tw tz ua ug um us uy uz va vc ve vg vi vn vu wf ws ye yt za zm zw";
// https://www.unicode.org/iso15924/iso15924-codes.html (227 published codes).
const SCRIPTS: &str = "adlm afak aghb ahom arab aran armi armn avst bali bamu bass batk beng berf bhks blis bopo brah brai bugi buhd cakm cans cari cham cher chis chrs cirt copt cpmn cprt cyrl cyrs deva diak dogr dsrt dupl egyd egyh egyp elba elym ethi gara geok geor glag gong gonm goth gran grek gujr gukh guru hanb hang hani hano hans hant hatr hebr hira hluw hmng hmnp hntl hrkt hung inds ital jamo java jpan jurc kali kana kawi khar khmr khoj kitl kits knda kore kpel krai kthi lana laoo latf latg latn leke lepc limb lina linb lisu loma lyci lydi mahj maka mand mani marc maya medf mend merc mero miao mlym modi mong moon mroo mtei mult mymr nagm nand narb nbat newa nkdb nkgb nkoo nshu ogam olck onao orkh orya osge osma ougr palm pauc pcun pelm perm phag phli phlp phlv phnx piqd plrd prti psin qaaa qabx ranj rjng rohg roro runr samr sara sarb saur seal sgnw shaw shrd shui sidd sidt sind sinh sogd sogo sora soyo sund sunu sylo syrc syre syrj syrn tagb takr tale talu taml tang tavt tayo telu teng tfng tglg thaa thai tibt tirh tnsa todr tols toto tutg ugar vaii visp vith wara wcho wole xpeo xsux yezi yiii zanb zinh zmth zsye zsym zxxx zyyy zzzz";

fn invalid_hreflang(value: &str) -> Option<(&'static str, &'static str)> {
    let locale = value.trim().to_ascii_lowercase();
    if locale == "x-default" {
        return None;
    }
    let parts: Vec<_> = locale.split('-').collect();
    if parts.is_empty()
        || !LANGUAGES
            .split_whitespace()
            .any(|language| language == parts[0])
    {
        return Some((
            "hreflang_language_invalid",
            "language is not in the published ISO 639-1 active two-letter list",
        ));
    }
    let mut index = 1;
    if parts.get(index).is_some_and(|part| part.len() == 4) {
        if !SCRIPTS
            .split_whitespace()
            .any(|script| script == parts[index])
        {
            return Some((
                "hreflang_script_invalid",
                "script is not in the published ISO 15924 list",
            ));
        }
        index += 1;
    }
    if let Some(region) = parts.get(index) {
        if !REGIONS.split_whitespace().any(|known| known == *region) {
            return Some((
                "hreflang_region_invalid",
                "region is not an assigned ISO 3166-1 alpha-2 code; reserved and numeric regions are not supported by this profile",
            ));
        }
        index += 1;
    }
    if index != parts.len() {
        return Some((
            "hreflang_code_invalid",
            "expected language, optional script and optional region only",
        ));
    }
    None
}

fn analyze_pagination(
    page: &PageRecord,
    lookup: &BTreeMap<&str, &PageRecord>,
    issues: &mut Vec<AuditIssue>,
) {
    if page.pagination_next.is_empty() && page.pagination_prev.is_empty() {
        return;
    }
    let different_canonicals: Vec<_> = page
        .canonical
        .iter()
        .filter(|address| !is_same_page(page, address))
        .cloned()
        .collect();
    if !different_canonicals.is_empty() {
        issue(
            issues,
            page,
            "pagination_canonicalized",
            Severity::Notice,
            "A document with declared pagination points its canonical elsewhere; review whether each distinct sequence page should have its own canonical",
            different_canonicals,
        );
    }
    for (direction, targets) in [
        ("next", &page.pagination_next),
        ("prev", &page.pagination_prev),
    ] {
        let addresses: BTreeSet<_> = targets.iter().map(String::as_str).collect();
        if addresses.len() > 1 {
            issue(
                issues,
                page,
                &format!("pagination_{direction}_conflict"),
                Severity::Warning,
                format!("Multiple distinct rel={direction} targets were observed"),
                targets.clone(),
            );
        }
        for address in addresses {
            if is_same_page(page, address) {
                issue(
                    issues,
                    page,
                    &format!("pagination_{direction}_self"),
                    Severity::Warning,
                    format!("Declared rel={direction} target resolves to this document itself"),
                    vec![address.into()],
                );
                continue;
            }
            analyze_target(
                page,
                address,
                &format!("pagination_{direction}"),
                lookup,
                issues,
            );
            if let Some(target) = lookup.get(address).copied() {
                let reverse = if direction == "next" {
                    &target.pagination_prev
                } else {
                    &target.pagination_next
                };
                if complete_document(target) && !reverse.iter().any(|url| is_same_page(page, url)) {
                    issue(
                        issues,
                        page,
                        &format!("pagination_{direction}_return_missing"),
                        Severity::Notice,
                        format!(
                            "Complete observed rel={direction} target has no reciprocal {} declaration; this is a navigation-consistency heuristic, not a Google indexing requirement",
                            if direction == "next" { "prev" } else { "next" }
                        ),
                        vec![address.into()],
                    );
                }
                if usable_document(target) && !target.is_html() {
                    issue(
                        issues,
                        page,
                        &format!("pagination_{direction}_target_non_html"),
                        Severity::Notice,
                        "Pagination target serves a non-HTML document; review the intended navigation",
                        vec![address.into()],
                    );
                }
            }
            if page.is_html()
                && complete_document(page)
                && !page
                    .links
                    .iter()
                    .any(|link| link.kind == "anchor" && link.url == address)
            {
                issue(
                    issues,
                    page,
                    &format!("pagination_{direction}_anchor_missing"),
                    Severity::Notice,
                    format!(
                        "Declared rel={direction} destination has no observed HTML anchor link; sequential anchor links aid discovery, while Google no longer uses next/prev metadata"
                    ),
                    vec![address.into()],
                );
            }
        }
    }
}

fn canonical_cycles(
    pages: &BTreeMap<&str, &PageRecord>,
    lookup: &BTreeMap<&str, &PageRecord>,
) -> Vec<Vec<String>> {
    let mut next = BTreeMap::new();
    for page in pages.values().copied().filter(|page| usable_document(page)) {
        let targets: BTreeSet<_> = page.canonical.iter().map(String::as_str).collect();
        if targets.len() == 1 {
            let target = *targets.iter().next().unwrap();
            if !is_same_page(page, target)
                && let Some(observed) = lookup.get(target)
                && usable_document(observed)
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
    let mut aliases: BTreeMap<&str, &str> = urls.iter().map(|&url| (url, url)).collect();
    for (&url, page) in &internal {
        aliases.entry(final_address(page)).or_insert(url);
        aliases.entry(document_address(page)).or_insert(url);
    }
    let mut edges: BTreeMap<(&str, &str), (usize, bool)> = BTreeMap::new();
    let mut unknown = BTreeSet::new();
    for (&source, page) in &internal {
        if !usable_html(page) {
            continue;
        }
        let page_nofollow = has_directive(page, "nofollow");
        for link in page.links.iter().filter(|link| link.kind == "anchor") {
            if let Some(&target) = aliases.get(link.url.as_str()) {
                let edge = edges.entry((source, target)).or_insert((0, false));
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
    let mut nonself_incoming = vec![0usize; urls.len()];
    let mut nonself_in_occurrences = vec![0usize; urls.len()];
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
        if from != to {
            nonself_incoming[to] += 1;
            nonself_in_occurrences[to] += count;
        }
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
    let incomplete_sources = internal
        .values()
        .filter(|page| !usable_document(page) || (page.is_html() && !complete_document(page)))
        .count();
    let partial = report.summary.pending > 0
        || report.summary.limited
        || report.summary.cancelled
        || incomplete_sources > 0;
    for (index, &url) in urls.iter().enumerate() {
        let page = internal[url];
        *depth_distribution.entry(page.depth).or_default() += 1;
        if nonself_incoming[index] == 0
            && usable_html(page)
            && page.status == Some(200)
            && !seeds.contains(url)
            && !seeds.contains(final_address(page))
        {
            orphan_candidates.push(json!({"url": url, "discovered_from": page.discovered_from,
                "observed_anchor_inlinks_from_other_pages": 0,
                "capture_incomplete": partial, "site_wide_orphan": "not_proven",
                "reason": "No anchor inlinks from another captured internal document; self-links do not establish discoverability. This is not a site-wide orphan assertion."}));
        }
        nodes.push(json!({"url": url, "title": page.title(), "status": page.status,
            "indexable": page.indexable, "crawl_depth": page.depth,
            "shortest_link_depth": shortest[index],
            "in_degree": incoming[index], "out_degree": outgoing[index],
            "in_degree_nonself": nonself_incoming[index],
            "inlink_occurrences_nonself": nonself_in_occurrences[index],
            "inlink_occurrences": in_occurrences[index], "outlink_occurrences": out_occurrences[index],
            "document_url": document_address(page),
            "page_rank": rank[index],
            "link_score": if largest > 0.0 { rank[index] / largest * 100.0 } else { 0.0 }
        }));
    }
    let edge_values: Vec<_> = edges.iter().map(|(&(source, target), &(count, follow))| {
        json!({"source": source, "target": target, "occurrences": count, "passes_score": follow})
    }).collect();
    json!({"nodes": nodes, "edges": edge_values, "depth_distribution": depth_distribution,
        "orphan_candidates": orphan_candidates, "unobserved_internal_targets": unknown,
        "coverage": {"incomplete_anchor_sources": incomplete_sources,
            "partial_capture": partial, "site_wide_completeness": "not_proven",
            "crawl_links": report.config.crawl_links,
            "include_patterns": report.config.include_patterns,
            "exclude_patterns": report.config.exclude_patterns,
            "note": "Only successful captured internal HTML documents supply anchor edges. Redirect/final document aliases are resolved to observed nodes. No external provider URL inventory is inferred."},
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
    fn bookmark_findings_require_captured_target_absence_and_remain_deduplicated() {
        let mut source = page("a");
        let mut target = page("b");
        let seeds = [normalize_url(&source.url, None).unwrap()];
        open_seo_spider_core::extract_html(
            &mut source,
            "<h1>A</h1><a href='/b#present'>Known</a><a href='/b#missing'>Absent</a><a href='/unobserved#missing'>Unknown</a>",
            &seeds,
            false,
        );
        open_seo_spider_core::extract_html(&mut target, "<h1 id='present'>B</h1>", &seeds, false);
        let mut snapshot = report(vec![source, target]);
        snapshot.config.validate_fragments = true;
        let analysis = analyze_report(&snapshot);
        assert!(has_issue(&analysis, "broken_bookmark", "a"));
        let findings: Vec<_> = analysis["issues"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|issue| issue["code"] == "broken_bookmark")
            .collect();
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0]["related_urls"][0],
            "https://fixture.test/b#missing"
        );
        assert_eq!(
            analysis["fragments"]["observations"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert!(
            analysis["fragments"]["observations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["status"] == "unknown")
        );
        extend_report(&mut snapshot);
        assert!(
            !analyze_report(&snapshot)["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|issue| issue["code"] == "broken_bookmark")
        );
        assert_eq!(
            snapshot
                .issues
                .iter()
                .filter(|issue| issue.code == "broken_bookmark")
                .count(),
            1
        );
    }

    #[test]
    fn disabled_legacy_and_incomplete_bookmark_targets_never_become_missing() {
        let mut source = page("a");
        let target = page("b");
        let seeds = [normalize_url(&source.url, None).unwrap()];
        open_seo_spider_core::extract_html(
            &mut source,
            "<h1>A</h1><a href='/b#missing'>Link</a>",
            &seeds,
            false,
        );
        let mut snapshot = report(vec![source, target]);
        assert_eq!(analyze_report(&snapshot)["fragments"]["enabled"], false);
        assert!(
            analyze_report(&snapshot)["fragments"]["observations"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        snapshot.config.validate_fragments = true;
        let unknown = analyze_report(&snapshot);
        assert_eq!(
            unknown["fragments"]["observations"][0]["reason"],
            "target_dom_not_observed"
        );
        assert!(!has_issue(&unknown, "broken_bookmark", "a"));
        open_seo_spider_core::extract_html(&mut snapshot.pages[1], "<h1>B</h1>", &seeds, false);
        snapshot.pages[1].truncated = true;
        let partial = analyze_report(&snapshot);
        assert_eq!(partial["fragments"]["observations"][0]["status"], "unknown");
        assert!(!has_issue(&partial, "broken_bookmark", "a"));
    }

    #[test]
    fn dom_review_issues_use_shared_core_rules_and_do_not_invent_legacy_absence() {
        let mut observed = page("a");
        let legacy = page("legacy");
        let seeds = [normalize_url(&observed.url, None).unwrap()];
        open_seo_spider_core::extract_html(
            &mut observed,
            "<h1>A</h1><h3>Skipped</h3><h2>First H2</h2><meta name='keywords' content=' '><meta name='keywords'>",
            &seeds,
            false,
        );
        let mut snapshot = report(vec![observed, legacy]);
        let analysis = analyze_report(&snapshot);
        assert!(has_issue(&analysis, "h2_nonsequential", "a"));
        assert!(has_issue(&analysis, "heading_level_skipped", "a"));
        assert!(has_issue(&analysis, "meta_keywords_empty", "a"));
        assert!(has_issue(&analysis, "meta_keywords_multiple", "a"));
        assert!(!has_issue(&analysis, "meta_keywords_missing", "legacy"));
        snapshot.pages[0].renderer_error = Some("renderer failed".into());
        let incomplete = analyze_report(&snapshot);
        assert!(!has_issue(&incomplete, "h2_nonsequential", "a"));
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
        b.x_robots_tag.push("googlebot:noindex".into());
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

    #[test]
    fn hreflang_profile_accepts_registered_language_script_region_and_default() {
        for value in [
            "de",
            "BE",
            "en-GB",
            "EN-us",
            "zh-Hant",
            "zh-Hans-US",
            "sr-Latn-RS",
            "x-default",
            "X-DEFAULT",
        ] {
            assert!(
                invalid_hreflang(value).is_none(),
                "unexpected invalid {value}"
            );
        }
        for (value, code) in [
            ("", "hreflang_language_invalid"),
            ("zz", "hreflang_language_invalid"),
            ("eng", "hreflang_language_invalid"),
            ("en-UK", "hreflang_region_invalid"),
            ("en-EU", "hreflang_region_invalid"),
            ("en-UN", "hreflang_region_invalid"),
            ("es-419", "hreflang_region_invalid"),
            ("en_US", "hreflang_language_invalid"),
            ("zh-Abcd", "hreflang_script_invalid"),
            ("en-GB-extra", "hreflang_code_invalid"),
        ] {
            assert_eq!(invalid_hreflang(value).unwrap().0, code, "{value}");
        }
    }

    #[test]
    fn x_default_is_optional_and_conflicting_fallbacks_have_explicit_evidence() {
        let mut a = page("a");
        a.hreflang = vec![alternate("de", "a"), alternate("en", "b")];
        let value = analyze_report(&report(vec![a.clone(), page("b")]));
        let notice = value["issues"]
            .as_array()
            .unwrap()
            .iter()
            .find(|issue| issue["code"] == "hreflang_x_default_missing")
            .unwrap();
        assert_eq!(notice["severity"], "notice");
        assert!(notice["message"].as_str().unwrap().contains("not required"));
        a.truncated = true;
        assert!(!has_issue(
            &analyze_report(&report(vec![a.clone()])),
            "hreflang_x_default_missing",
            "a"
        ));
        a.truncated = false;
        a.hreflang
            .extend([alternate("x-default", "a"), alternate("X-DEFAULT", "b")]);
        let value = analyze_report(&report(vec![a, page("b")]));
        assert!(has_issue(&value, "hreflang_x_default_conflict", "a"));
        assert!(!has_issue(&value, "hreflang_x_default_missing", "a"));
        let mut single = page("single");
        single.hreflang.push(alternate("de", "single"));
        assert!(!has_issue(
            &analyze_report(&report(vec![single])),
            "hreflang_x_default_missing",
            "single"
        ));
    }

    #[test]
    fn html_and_http_conflicts_are_distinct_from_same_target_declarations() {
        let mut a = page("a");
        a.html_canonical.push(page("a").url);
        a.header_canonical.push(page("b").url);
        a.canonical = vec![page("a").url, page("b").url];
        a.html_hreflang.push(alternate("en", "a"));
        a.header_hreflang.push(alternate("EN", "b"));
        a.hreflang = a
            .html_hreflang
            .iter()
            .chain(&a.header_hreflang)
            .cloned()
            .collect();
        let value = analyze_report(&report(vec![a.clone(), page("b")]));
        assert!(has_issue(&value, "canonical_html_header_conflict", "a"));
        assert!(has_issue(&value, "hreflang_html_header_conflict", "a"));
        a.header_canonical = a.html_canonical.clone();
        a.canonical = a.html_canonical.clone();
        a.header_hreflang = a.html_hreflang.clone();
        a.hreflang = a.html_hreflang.clone();
        let value = analyze_report(&report(vec![a]));
        assert!(!has_issue(&value, "canonical_conflict", "a"));
        assert!(!has_issue(&value, "canonical_html_header_conflict", "a"));
        assert!(!has_issue(&value, "hreflang_html_header_conflict", "a"));
    }

    #[test]
    fn header_relationships_on_pdf_documents_receive_target_and_return_checks() {
        let mut a = page("a.pdf");
        a.content_type = "application/pdf".into();
        a.indexable = false; // Core's HTML indexability flag is not a PDF verdict.
        a.canonical.push(page("missing.pdf").url);
        a.header_hreflang = vec![alternate("de", "a.pdf"), alternate("en", "b.pdf")];
        a.hreflang = a.header_hreflang.clone();
        let mut b = page("b.pdf");
        b.content_type = "application/pdf".into();
        b.hreflang.push(alternate("en", "b.pdf"));
        let value = analyze_report(&report(vec![a, b]));
        assert!(has_issue(&value, "canonical_target_not_observed", "a.pdf"));
        assert!(has_issue(&value, "hreflang_return_missing", "a.pdf"));
        assert!(!has_issue(&value, "hreflang_target_noindex", "a.pdf"));
        assert!(!has_issue(&value, "security_csp_missing", "a.pdf"));
    }

    #[test]
    fn hreflang_cluster_and_return_labels_compare_declarations_not_html_language() {
        let mut a = page("a");
        a.hreflang = vec![alternate("de", "a"), alternate("en", "b")];
        a.language = Some("en".into()); // No content language inference from this attribute.
        let mut b = page("b");
        b.hreflang = vec![
            alternate("fr", "a"),
            alternate("en", "b"),
            alternate("de", "c"),
        ];
        let value = analyze_report(&report(vec![a.clone(), b.clone(), page("c")]));
        assert!(has_issue(&value, "hreflang_return_language_conflict", "a"));
        assert!(has_issue(&value, "hreflang_cluster_conflict", "a"));
        b.hreflang = a.hreflang.clone();
        let value = analyze_report(&report(vec![a, b]));
        assert!(!has_issue(&value, "hreflang_return_language_conflict", "a"));
        assert!(!has_issue(&value, "hreflang_cluster_conflict", "a"));
    }

    #[test]
    fn canonical_language_review_requires_self_hreflang_facts_on_both_documents() {
        let mut a = page("a");
        a.canonical.push(page("b").url);
        a.hreflang.push(alternate("de-AT", "a"));
        let mut b = page("b");
        b.hreflang.push(alternate("en-US", "b"));
        let value = analyze_report(&report(vec![a.clone(), b.clone()]));
        assert!(has_issue(
            &value,
            "canonical_hreflang_language_conflict",
            "a"
        ));
        b.hreflang = vec![alternate("de-DE", "b")];
        assert!(!has_issue(
            &analyze_report(&report(vec![a.clone(), b.clone()])),
            "canonical_hreflang_language_conflict",
            "a"
        ));
        b.hreflang.clear();
        b.language = Some("en".into());
        assert!(!has_issue(
            &analyze_report(&report(vec![a, b])),
            "canonical_hreflang_language_conflict",
            "a"
        ));
    }

    #[test]
    fn complete_reciprocal_pagination_with_anchors_has_no_pagination_diagnostics() {
        let mut a = page("a");
        a.pagination_next.push(page("b").url);
        a.links.push(anchor("b"));
        a.canonical.push(page("a").url);
        let mut b = page("b");
        b.pagination_prev.push(page("a").url);
        b.links.push(anchor("a"));
        b.canonical.push(page("b").url);
        let value = analyze_report(&report(vec![a, b]));
        assert!(
            !value["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|issue| issue["code"].as_str().unwrap().starts_with("pagination_"))
        );
    }

    #[test]
    fn pagination_distinguishes_bad_captured_targets_from_unknown_targets() {
        let mut a = page("a");
        a.pagination_next = vec![page("missing").url, page("b").url];
        a.pagination_prev = vec![page("redirect").url];
        let mut b = page("b");
        b.status = Some(404);
        b.final_status = Some(404);
        b.meta_robots.push("noindex".into());
        let mut redirected = page("redirect");
        redirected.status = Some(301);
        redirected.final_url = page("landing").url;
        let value = analyze_report(&report(vec![a, b, redirected]));
        for code in [
            "pagination_next_conflict",
            "pagination_next_target_not_observed",
            "pagination_next_target_non200",
            "pagination_next_target_noindex",
            "pagination_prev_target_redirect",
        ] {
            assert!(has_issue(&value, code, "a"), "missing {code}");
        }
        let non200 = value["issues"]
            .as_array()
            .unwrap()
            .iter()
            .find(|issue| issue["code"] == "pagination_next_target_non200")
            .unwrap();
        assert_eq!(non200["related_urls"], json!([page("b").url]));
        assert!(!has_issue(&value, "pagination_next_return_missing", "a"));
    }

    #[test]
    fn pagination_self_canonicalization_anchor_and_reciprocity_have_clear_severities() {
        let mut a = page("a");
        a.pagination_next = vec![format!("{}#page2", page("a").url), page("b").url];
        a.canonical.push(page("first").url);
        let value = analyze_report(&report(vec![a, page("b"), page("first")]));
        for code in [
            "pagination_next_self",
            "pagination_canonicalized",
            "pagination_next_anchor_missing",
            "pagination_next_return_missing",
        ] {
            assert!(has_issue(&value, code, "a"), "missing {code}");
        }
        let returns = value["issues"]
            .as_array()
            .unwrap()
            .iter()
            .find(|issue| issue["code"] == "pagination_next_return_missing")
            .unwrap();
        assert_eq!(returns["severity"], "notice");
        assert!(
            returns["message"]
                .as_str()
                .unwrap()
                .contains("not a Google indexing requirement")
        );
    }

    #[test]
    fn incomplete_capture_never_proves_absent_return_or_anchor_declarations() {
        let mut a = page("a");
        a.hreflang = vec![alternate("de", "a"), alternate("en", "b")];
        a.pagination_next.push(page("b").url);
        a.truncated = true;
        let mut b = page("b");
        for truncated in [true, false] {
            b.truncated = truncated;
            b.renderer_error = (!truncated).then(|| "Worker unavailable".into());
            let value = analyze_report(&report(vec![a.clone(), b.clone()]));
            assert!(has_issue(&value, "hreflang_target_incomplete", "a"));
            assert!(!has_issue(&value, "hreflang_return_missing", "a"));
            assert!(!has_issue(&value, "pagination_next_return_missing", "a"));
            assert!(!has_issue(&value, "pagination_next_anchor_missing", "a"));
        }
    }

    #[test]
    fn unknown_rendered_response_never_proves_absent_return_declarations() {
        let mut a = page("a");
        a.hreflang = vec![alternate("de", "a"), alternate("en", "b")];
        a.pagination_next.push(page("b").url);
        let mut b = page("b");
        b.rendered = true;
        b.rendered_status = None;
        assert!(!complete_document(&b));
        let value = analyze_report(&report(vec![a.clone(), b]));
        assert!(has_issue(&value, "hreflang_target_incomplete", "a"));
        assert!(!has_issue(&value, "hreflang_return_missing", "a"));
        assert!(!has_issue(&value, "pagination_next_return_missing", "a"));
        // The source is complete: its absent anchor is independent of the
        // target's unknown response and remains an observed source finding.
        assert!(has_issue(&value, "pagination_next_anchor_missing", "a"));
        a.rendered = true;
        a.rendered_status = None;
        let value = analyze_report(&report(vec![a, page("b")]));
        assert!(!has_issue(&value, "pagination_next_anchor_missing", "a"));
    }

    #[test]
    fn target_failed_blocked_and_status_unknown_are_separate_states() {
        let mut a = page("a");
        for path in ["failed", "blocked", "unknown", "absent"] {
            a.pagination_next.push(page(path).url);
        }
        let mut failed = page("failed");
        failed.status = None;
        failed.final_status = None;
        failed.error = Some("timeout".into());
        let mut blocked = page("blocked");
        blocked.status = None;
        blocked.final_status = None;
        blocked.blocked_by_robots = true;
        let mut unknown = page("unknown");
        unknown.status = None;
        unknown.final_status = None;
        let value = analyze_report(&report(vec![a, failed, blocked, unknown]));
        for (code, path) in [
            ("pagination_next_target_failed", "failed"),
            ("pagination_next_target_blocked", "blocked"),
            ("pagination_next_target_status_unknown", "unknown"),
            ("pagination_next_target_not_observed", "absent"),
        ] {
            let observed = value["issues"]
                .as_array()
                .unwrap()
                .iter()
                .find(|issue| issue["code"] == code)
                .unwrap();
            assert_eq!(observed["related_urls"], json!([page(path).url]));
        }
        assert!(!has_issue(&value, "pagination_next_target_non200", "a"));
        assert!(!has_issue(&value, "pagination_next_return_missing", "a"));
    }

    #[test]
    fn redirected_relationship_target_keeps_initial_and_failed_final_response_distinct() {
        let mut a = page("a");
        a.canonical.push(page("old").url);
        a.pagination_next.push(page("old").url);
        let mut target = page("old");
        target.status = Some(301);
        target.final_url = page("gone").url;
        target.final_status = Some(404);
        let value = analyze_report(&report(vec![a.clone(), target.clone()]));
        for prefix in ["canonical", "pagination_next"] {
            assert!(has_issue(&value, &format!("{prefix}_target_redirect"), "a"));
            assert!(has_issue(
                &value,
                &format!("{prefix}_target_final_non200"),
                "a"
            ));
            assert!(!has_issue(&value, &format!("{prefix}_target_non200"), "a"));
        }
        let final_response = value["issues"]
            .as_array()
            .unwrap()
            .iter()
            .find(|issue| issue["code"] == "canonical_target_final_non200")
            .unwrap();
        assert_eq!(
            final_response["related_urls"],
            json!([page("gone").url, page("old").url])
        );
        assert!(
            final_response["message"]
                .as_str()
                .unwrap()
                .contains("HTTP 404")
        );
        target.final_status = None;
        let value = analyze_report(&report(vec![a, target]));
        assert!(has_issue(&value, "canonical_target_redirect", "a"));
        assert!(!has_issue(&value, "canonical_target_final_non200", "a"));
    }

    #[test]
    fn metadata_self_identity_uses_final_and_rendered_document_addresses() {
        let mut redirected = page("old");
        redirected.status = Some(301);
        redirected.final_url = page("landing").url;
        redirected.canonical.push(page("old").url);
        redirected.hreflang.push(alternate("de", "old"));
        let value = analyze_report(&report(vec![redirected]));
        assert!(has_issue(&value, "canonical_target_redirect", "old"));
        assert!(has_issue(&value, "hreflang_self_missing", "old"));
        let mut rendered = page("a");
        rendered.rendered = true;
        rendered.rendered_status = Some(200);
        rendered.rendered_url = Some(page("spa").url);
        rendered.canonical.push(page("spa").url);
        rendered.hreflang = vec![alternate("de", "spa"), alternate("en", "b")];
        let mut b = page("b");
        b.hreflang = rendered.hreflang.clone();
        let value = analyze_report(&report(vec![rendered, b]));
        assert!(!has_issue(&value, "canonical_target_not_observed", "a"));
        assert!(!has_issue(&value, "hreflang_self_missing", "a"));
        assert!(!has_issue(&value, "hreflang_return_missing", "a"));
        assert!(!has_issue(&value, "hreflang_return_missing", "b"));
    }

    #[test]
    fn graph_self_links_do_not_disqualify_crawl_bounded_orphan_candidates() {
        let mut a = page("a");
        a.links.push(anchor("a"));
        let mut b = page("b");
        b.links.push(anchor("b"));
        b.discovered_from = Some("https://fixture.test/sitemap.xml".into());
        let mut snapshot = report(vec![a, b]);
        snapshot.summary.pending = 4;
        let value = analyze_report(&snapshot);
        assert!(!has_issue(&value, "crawl_orphan_candidate", "a"));
        assert!(has_issue(&value, "crawl_orphan_candidate", "b"));
        assert_eq!(value["graph"]["nodes"][1]["in_degree"], 1);
        assert_eq!(value["graph"]["nodes"][1]["in_degree_nonself"], 0);
        assert_eq!(
            value["graph"]["orphan_candidates"][0]["capture_incomplete"],
            true
        );
        assert_eq!(
            value["graph"]["orphan_candidates"][0]["site_wide_orphan"],
            "not_proven"
        );
    }

    #[test]
    fn graph_resolves_final_alias_prefers_direct_fetch_and_excludes_failed_sources() {
        let mut a = page("a");
        a.links.push(anchor("landing"));
        let mut redirected = page("old");
        redirected.status = Some(301);
        redirected.final_url = page("landing").url;
        let value = analyze_report(&report(vec![a.clone(), redirected.clone()]));
        assert_eq!(value["graph"]["edges"][0]["target"], page("old").url);
        assert_eq!(value["graph"]["unobserved_internal_targets"], json!([]));
        assert!(!has_issue(&value, "crawl_orphan_candidate", "old"));
        let mut failed = page("failed");
        failed.status = Some(500);
        failed.final_status = Some(500);
        failed.links.push(anchor("landing"));
        let value = analyze_report(&report(vec![a, redirected, page("landing"), failed]));
        assert_eq!(value["graph"]["edges"].as_array().unwrap().len(), 1);
        assert_eq!(value["graph"]["edges"][0]["target"], page("landing").url);
        assert_eq!(value["graph"]["coverage"]["partial_capture"], true);
    }

    #[test]
    fn target_scoped_robots_and_graph_nofollow_use_the_selected_agent() {
        let mut a = page("a");
        a.canonical.push(page("b").url);
        a.links.push(anchor("b"));
        a.indexability_agent = "googlebot".into();
        a.x_robots_tag.push("bingbot: nofollow".into());
        let mut b = page("b");
        b.indexability_agent = "googlebot".into();
        b.x_robots_tag.push("bingbot: noindex".into());
        let value = analyze_report(&report(vec![a.clone(), b.clone()]));
        assert!(!has_issue(&value, "canonical_target_noindex", "a"));
        assert_eq!(value["graph"]["edges"][0]["passes_score"], true);
        a.indexability_agent = "bingbot".into();
        b.indexability_agent = "bingbot".into();
        let value = analyze_report(&report(vec![a, b]));
        assert!(has_issue(&value, "canonical_target_noindex", "a"));
        assert_eq!(value["graph"]["edges"][0]["passes_score"], false);
    }

    #[test]
    fn deduplication_preserves_additional_relationship_target_evidence() {
        let mut a = page("a");
        a.hreflang = vec![
            alternate("de", "a"),
            alternate("en", "b"),
            alternate("fr", "c"),
        ];
        let mut snapshot = report(vec![a, page("b"), page("c")]);
        snapshot.issues.push(AuditIssue {
            code: "hreflang_return_missing".into(),
            severity: Severity::Warning,
            url: page("a").url,
            message: "Already observed b".into(),
            related_urls: vec![page("b").url],
        });
        let value = analyze_report(&snapshot);
        let returns: Vec<_> = value["issues"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|issue| {
                issue["url"] == page("a").url && issue["code"] == "hreflang_return_missing"
            })
            .collect();
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0]["related_urls"], json!([page("c").url]));
        extend_report(&mut snapshot);
        assert!(
            analyze_report(&snapshot)["issues"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}
