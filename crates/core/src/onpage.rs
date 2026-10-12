//! Independent onpage observations with explicit, configurable thresholds.
//! Length/URL/link heuristics are review notices, not search-engine verdicts.
//! Inventory names were checked against docs/parity/reference-cli-inventory.json;
//! matching a filter name is not proof of equivalent vendor filter membership.
use crate::{AuditIssue, AuditThresholds, PageRecord, Severity};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn complete_html(page: &PageRecord) -> bool {
    page.internal
        && page.is_html()
        && page.final_status.or(page.status) == Some(200)
        && (!page.rendered || page.rendered_status == Some(200))
        && page.error.is_none()
        && !page.blocked_by_robots
        && !page.truncated
        && page.renderer_error.is_none()
}

fn notice(
    issues: &mut Vec<AuditIssue>,
    page: &PageRecord,
    code: &str,
    message: String,
    related_urls: Vec<String>,
) {
    issues.push(AuditIssue {
        code: code.into(),
        severity: Severity::Notice,
        url: page.url.clone(),
        message,
        related_urls: related_urls
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    });
}

pub(crate) fn audit_page(
    page: &PageRecord,
    limits: &AuditThresholds,
    issues: &mut Vec<AuditIssue>,
) {
    if page.internal {
        audit_url(page, limits, issues);
    }
    if !complete_html(page) {
        return;
    }
    for (code, label, text, min, max) in [
        (
            "title",
            "Primary title",
            page.title(),
            limits.min_title_chars,
            limits.max_title_chars,
        ),
        (
            "description",
            "Primary meta description",
            page.description(),
            limits.min_description_chars,
            limits.max_description_chars,
        ),
    ] {
        let count = text.trim().chars().count();
        if count == 0 {
            continue;
        }
        if let Some(min) = min
            && count < min
        {
            notice(
                issues,
                page,
                &format!("{code}_short"),
                format!(
                    "{label} has {count} Unicode characters, below configured minimum {min}; review content and search appearance"
                ),
                vec![],
            );
        }
        if let Some(max) = max
            && count > max
        {
            notice(
                issues,
                page,
                &format!("{code}_long"),
                format!(
                    "{label} has {count} Unicode characters, above configured maximum {max}; review content and search appearance"
                ),
                vec![],
            );
        }
    }
    for (code, label, widths, min, max) in [
        (
            "title",
            "Primary title",
            &page.title_pixel_widths,
            limits.min_title_pixels,
            limits.max_title_pixels,
        ),
        (
            "description",
            "Primary meta description",
            &page.description_pixel_widths,
            limits.min_description_pixels,
            limits.max_description_pixels,
        ),
    ] {
        if let Some(Some(width)) = widths.first() {
            let engine = page
                .text_metrics_engine
                .as_deref()
                .unwrap_or("unspecified captured metric");
            if let Some(min) = min
                && (*width as usize) < min
            {
                notice(
                    issues,
                    page,
                    &format!("{code}_pixels_short"),
                    format!(
                        "{label} measured {width}px below configured {min}px minimum (profile {engine})"
                    ),
                    vec![],
                );
            }
            if let Some(max) = max
                && (*width as usize) > max
            {
                notice(
                    issues,
                    page,
                    &format!("{code}_pixels_long"),
                    format!(
                        "{label} measured {width}px above configured {max}px maximum (profile {engine})"
                    ),
                    vec![],
                );
            }
        }
    }
    for (code, headings, max) in [
        ("h1", &page.h1, limits.max_h1_chars),
        ("h2", &page.h2, limits.max_h2_chars),
    ] {
        if let Some(max) = max {
            let long: Vec<_> = headings
                .iter()
                .map(|value| value.trim().chars().count())
                .filter(|&count| count > max)
                .collect();
            if !long.is_empty() {
                notice(
                    issues,
                    page,
                    &format!("{code}_long"),
                    format!(
                        "{} {code} heading(s) exceed configured {max} Unicode characters; largest observed length {}",
                        long.len(),
                        long.iter().max().unwrap()
                    ),
                    vec![],
                );
            }
        }
    }
    if page.h2.iter().all(|heading| heading.trim().is_empty()) {
        notice(issues,page,"h2_missing","No nonempty H2 heading was observed in the complete captured document; sections may not need H2 headings".into(),vec![]);
    }
    if page.h2.len() > 1 {
        notice(
            issues,
            page,
            "h2_multiple",
            format!(
                "{} H2 elements observed; multiple sections can legitimately use multiple H2 headings",
                page.h2.len()
            ),
            vec![],
        );
    }
    let empty_alt: Vec<_> = page
        .images
        .iter()
        .filter(|image| image.alt.as_ref().is_some_and(|alt| alt.trim().is_empty()))
        .map(|image| image.url.clone())
        .collect();
    if !empty_alt.is_empty() {
        notice(
            issues,
            page,
            "image_alt_empty",
            format!(
                "{} image occurrence(s) have an empty alt attribute; this can be intentional for decorative images",
                empty_alt.len()
            ),
            empty_alt,
        );
    }
    if let Some(max) = limits.max_alt_chars {
        let long_alt: Vec<_> = page
            .images
            .iter()
            .filter(|image| {
                image
                    .alt
                    .as_ref()
                    .is_some_and(|alt| alt.chars().count() > max)
            })
            .map(|image| image.url.clone())
            .collect();
        if !long_alt.is_empty() {
            notice(
                issues,
                page,
                "image_alt_long",
                format!(
                    "{} image occurrence(s) have alt text above configured {max} Unicode characters; review the text in its context",
                    long_alt.len()
                ),
                long_alt,
            );
        }
    }
    let missing_size: Vec<_> = page
        .images
        .iter()
        .filter(|image| {
            image
                .width
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
                || image
                    .height
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
        })
        .map(|image| image.url.clone())
        .collect();
    if !missing_size.is_empty() {
        notice(
            issues,
            page,
            "image_size_attributes_missing",
            format!(
                "{} image occurrence(s) have missing or empty width/height attributes; intrinsic and rendered dimensions are not inferred",
                missing_size.len()
            ),
            missing_size,
        );
    }
    if let Some(min) = limits.min_word_count
        && page.word_count < min
    {
        notice(
            issues,
            page,
            "content_low_word_count",
            format!(
                "Captured normalized visible text has {} words, below configured minimum {min}; word count alone does not establish content quality",
                page.word_count
            ),
            vec![],
        );
    }
    audit_links(page, limits, issues);
}

fn audit_links(page: &PageRecord, limits: &AuditThresholds, issues: &mut Vec<AuditIssue>) {
    let page_nofollow = crate::applicable_robot_directives(page)
        .iter()
        .any(|directive| matches!(directive.as_str(), "nofollow" | "none"));
    for (internal, scope, max) in [
        (true, "internal", limits.max_internal_outlinks),
        (false, "external", limits.max_external_outlinks),
    ] {
        let links: Vec<_> = page
            .links
            .iter()
            .filter(|link| link.kind == "anchor" && link.internal == internal)
            .collect();
        if let Some(max) = max
            && links.len() > max
        {
            notice(
                issues,
                page,
                &format!("high_{scope}_outlinks"),
                format!(
                    "{} observed {scope} anchor occurrences exceed configured maximum {max}; repeated destinations are counted as separate occurrences",
                    links.len()
                ),
                links.iter().map(|link| link.url.clone()).collect(),
            );
        }
        if internal && links.is_empty() {
            notice(issues,page,"internal_outlinks_missing","No internal anchor outlinks were observed in the complete captured document; this can be intentional".into(),vec![]);
        }
        for rel in ["nofollow", "sponsored", "ugc"] {
            let tagged: Vec<_> = links
                .iter()
                .filter(|link| {
                    link.rel
                        .split_whitespace()
                        .any(|token| token.eq_ignore_ascii_case(rel))
                        || (rel == "nofollow" && (link.nofollow || page_nofollow))
                })
                .map(|link| link.url.clone())
                .collect();
            if !tagged.is_empty() {
                notice(
                    issues,
                    page,
                    &format!("{scope}_{rel}_outlinks"),
                    format!(
                        "{} observed {scope} anchor occurrence(s) use {rel}{}; these annotations can be intentional",
                        tagged.len(),
                        if rel == "nofollow" {
                            " on the link or through the selected agent's page policy"
                        } else {
                            ""
                        }
                    ),
                    tagged,
                );
            }
        }
        if internal {
            let empty: Vec<_> = links
                .iter()
                .filter(|link| link.text.trim().is_empty())
                .map(|link| link.url.clone())
                .collect();
            if !empty.is_empty() {
                notice(
                    issues,
                    page,
                    "internal_outlinks_empty_anchor",
                    format!(
                        "{} internal anchor occurrence(s) have no captured text; image alt/ARIA accessible names are not inferred",
                        empty.len()
                    ),
                    empty,
                );
            }
        }
    }
}

fn decode_component(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) = (
                (bytes[index + 1] as char).to_digit(16),
                (bytes[index + 2] as char).to_digit(16),
            )
        {
            result.push((high * 16 + low) as u8);
            index += 3;
        } else {
            result.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(result).ok()
}

fn audit_url(page: &PageRecord, limits: &AuditThresholds, issues: &mut Vec<AuditIssue>) {
    let Ok(url) = url::Url::parse(&page.url) else {
        return;
    };
    // Invalid UTF-8 makes that component's decoded character properties unknown.
    // Do not misclassify uppercase hexadecimal escapes as actual uppercase text.
    let path = decode_component(url.path()).unwrap_or_default();
    let query = decode_component(url.query().unwrap_or_default()).unwrap_or_default();
    let components = format!("{path}{query}");
    if components.chars().any(|character| character.is_uppercase()) {
        notice(issues,page,"url_uppercase","Decoded path/query contains uppercase characters; casing can be meaningful and is a URL-consistency review observation".into(),vec![]);
    }
    if components.contains('_') {
        notice(
            issues,
            page,
            "url_underscores",
            "Decoded path/query contains underscores; review URL naming consistency".into(),
            vec![],
        );
    }
    if !components.is_ascii() {
        notice(issues,page,"url_non_ascii","Path/query contains literal or valid UTF-8 percent-encoded non-ASCII characters; localized URLs can legitimately use them".into(),vec![]);
    }
    if components.contains(' ') {
        notice(issues,page,"url_contains_space","Decoded path/query contains a literal or percent-encoded space; query plus signs are not inferred to be spaces".into(),vec![]);
    }
    if url.path().contains("//") {
        notice(
            issues,
            page,
            "url_multiple_slashes",
            "URL path contains repeated slashes; the required scheme separator is excluded".into(),
            vec![],
        );
    }
    let parameters = url.query_pairs().count();
    if parameters > 0 {
        notice(
            issues,
            page,
            "url_parameters",
            format!(
                "URL contains {parameters} parsed query parameter(s); parameters are not inherently a crawling or indexing problem"
            ),
            vec![],
        );
    }
    if let Some(max) = limits.max_url_chars {
        let count = page.url.chars().count();
        if count > max {
            notice(
                issues,
                page,
                "url_long",
                format!(
                    "Normalized request URL has {count} Unicode characters, above configured maximum {max}"
                ),
                vec![],
            );
        }
    }
}

pub(crate) fn audit_heading_duplicates(pages: &[PageRecord], issues: &mut Vec<AuditIssue>) {
    for (code, label) in [("h1_duplicate", "H1"), ("h2_duplicate", "H2")] {
        let mut groups: BTreeMap<String, BTreeMap<&str, &PageRecord>> = BTreeMap::new();
        for page in pages
            .iter()
            .filter(|page| complete_html(page) && page.indexable)
        {
            let headings = if label == "H1" { &page.h1 } else { &page.h2 };
            for heading in headings {
                let value = heading
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase();
                if !value.is_empty() {
                    groups.entry(value).or_default().insert(&page.url, page);
                }
            }
        }
        for (text, group) in groups.into_iter().filter(|(_, group)| group.len() > 1) {
            for page in group.values() {
                notice(
                    issues,
                    page,
                    code,
                    format!(
                        "Case-insensitive normalized {label} text {text:?} occurs on {} captured indexable URLs; shared headings can be intentional",
                        group.len()
                    ),
                    group
                        .keys()
                        .filter(|&&url| url != page.url)
                        .map(|url| (*url).into())
                        .collect(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ImageRecord, LinkRecord};

    fn page(path: &str) -> PageRecord {
        let address = format!("https://fixture.test/{path}");
        PageRecord {
            url: address.clone(),
            final_url: address,
            status: Some(200),
            final_status: Some(200),
            content_type: "text/html".into(),
            internal: true,
            indexable: true,
            titles: vec!["Ordinary title".into()],
            h1: vec!["One heading".into()],
            h2: vec!["One section".into()],
            ..Default::default()
        }
    }
    fn disabled() -> AuditThresholds {
        AuditThresholds {
            min_title_chars: None,
            max_title_chars: None,
            min_description_chars: None,
            max_description_chars: None,
            max_h1_chars: None,
            max_h2_chars: None,
            max_alt_chars: None,
            max_url_chars: None,
            min_word_count: None,
            max_internal_outlinks: None,
            max_external_outlinks: None,
            min_title_pixels: None,
            max_title_pixels: None,
            min_description_pixels: None,
            max_description_pixels: None,
        }
    }
    fn evaluate(page: &PageRecord, limits: &AuditThresholds) -> Vec<AuditIssue> {
        let mut issues = vec![];
        audit_page(page, limits, &mut issues);
        issues
    }
    fn has(issues: &[AuditIssue], code: &str) -> bool {
        issues.iter().any(|issue| issue.code == code)
    }
    fn anchor(path: &str, internal: bool, rel: &str) -> LinkRecord {
        LinkRecord {
            url: format!("https://fixture.test/{path}"),
            internal,
            kind: "anchor".into(),
            text: "Observed text".into(),
            rel: rel.into(),
            ..Default::default()
        }
    }

    #[test]
    fn length_limits_count_unicode_scalars_at_strict_boundaries_and_can_be_disabled() {
        let mut p = page("a");
        let mut limits = disabled();
        limits.min_title_chars = Some(30);
        limits.max_title_chars = Some(60);
        limits.min_description_chars = Some(70);
        limits.max_description_chars = Some(155);
        p.titles = vec!["ä".repeat(29)];
        p.meta_descriptions = vec!["é".repeat(69)];
        let issues = evaluate(&p, &limits);
        assert!(has(&issues, "title_short"));
        assert!(has(&issues, "description_short"));
        assert!(
            issues
                .iter()
                .all(|issue| issue.severity == Severity::Notice)
        );
        p.titles = vec!["ä".repeat(30)];
        p.meta_descriptions = vec!["é".repeat(70)];
        let issues = evaluate(&p, &limits);
        assert!(!has(&issues, "title_short"));
        assert!(!has(&issues, "description_short"));
        p.titles = vec!["ä".repeat(60)];
        p.meta_descriptions = vec!["é".repeat(155)];
        let issues = evaluate(&p, &limits);
        assert!(!has(&issues, "title_long"));
        assert!(!has(&issues, "description_long"));
        p.titles = vec!["ä".repeat(61)];
        p.meta_descriptions = vec!["é".repeat(156)];
        let issues = evaluate(&p, &limits);
        assert!(has(&issues, "title_long"));
        assert!(has(&issues, "description_long"));
        let issues = evaluate(&p, &disabled());
        assert!(!has(&issues, "title_long"));
        assert!(!has(&issues, "description_long"));
        p.titles = vec![" ".into()];
        p.meta_descriptions = vec!["\t".into()];
        let issues = evaluate(&p, &limits);
        assert!(!has(&issues, "title_short"));
        assert!(!has(&issues, "description_short"));
    }

    #[test]
    fn pixel_limits_require_observed_width_and_preserve_the_measured_profile() {
        let mut p = page("a");
        let mut limits = disabled();
        limits.min_title_pixels = Some(200);
        limits.max_title_pixels = Some(561);
        limits.min_description_pixels = Some(400);
        limits.max_description_pixels = Some(985);
        p.title_pixel_widths = vec![None];
        p.description_pixel_widths = vec![None];
        let issues = evaluate(&p, &limits);
        assert!(!has(&issues, "title_pixels_short"));
        assert!(!has(&issues, "description_pixels_long"));
        p.title_pixel_widths = vec![Some(199)];
        p.description_pixel_widths = vec![Some(986)];
        p.text_metrics_engine = Some("fixture_profile_20px_14px".into());
        let issues = evaluate(&p, &limits);
        assert!(has(&issues, "title_pixels_short"));
        assert!(has(&issues, "description_pixels_long"));
        assert!(
            issues
                .iter()
                .filter(|issue| issue.code.contains("pixels"))
                .all(|issue| issue.message.contains("fixture_profile_20px_14px"))
        );
        p.title_pixel_widths = vec![Some(561)];
        p.description_pixel_widths = vec![Some(400)];
        let issues = evaluate(&p, &limits);
        assert!(!issues.iter().any(|issue| issue.code.contains("pixels")));
        p.title_pixel_widths = vec![Some(562)];
        p.description_pixel_widths = vec![Some(399)];
        let issues = evaluate(&p, &limits);
        assert!(has(&issues, "title_pixels_long"));
        assert!(has(&issues, "description_pixels_short"));
        assert!(
            !evaluate(&p, &disabled())
                .iter()
                .any(|issue| issue.code.contains("pixels"))
        );
    }

    #[test]
    fn heading_and_alt_observations_distinguish_empty_attributes_from_missing() {
        let mut p = page("a");
        let mut limits = disabled();
        limits.max_h1_chars = Some(2);
        limits.max_h2_chars = Some(2);
        limits.max_alt_chars = Some(2);
        p.h1 = vec!["ééé".into()];
        p.h2 = vec!["a".into(), "bbb".into()];
        p.images = vec![
            ImageRecord {
                url: "https://fixture.test/decorative.png".into(),
                alt: Some("".into()),
                width: Some("100%".into()),
                height: Some("auto".into()),
                ..Default::default()
            },
            ImageRecord {
                url: "https://fixture.test/long.png".into(),
                alt: Some("ééé".into()),
                ..Default::default()
            },
            ImageRecord {
                url: "https://fixture.test/absent.png".into(),
                alt: None,
                ..Default::default()
            },
        ];
        let issues = evaluate(&p, &limits);
        for code in [
            "h1_long",
            "h2_long",
            "h2_multiple",
            "image_alt_empty",
            "image_alt_long",
            "image_size_attributes_missing",
        ] {
            assert!(has(&issues, code), "missing {code}");
        }
        let empty = issues
            .iter()
            .find(|issue| issue.code == "image_alt_empty")
            .unwrap();
        assert_eq!(
            empty.related_urls,
            vec!["https://fixture.test/decorative.png"]
        );
        assert!(empty.message.contains("intentional"));
        let sizes = issues
            .iter()
            .find(|issue| issue.code == "image_size_attributes_missing")
            .unwrap();
        assert!(
            !sizes
                .related_urls
                .contains(&"https://fixture.test/decorative.png".into())
        );
        p.h2 = vec!["  ".into()];
        assert!(has(&evaluate(&p, &limits), "h2_missing"));
    }

    #[test]
    fn link_scopes_rel_tokens_page_policy_and_occurrence_limits_are_explicit() {
        let mut p = page("a");
        let mut limits = disabled();
        limits.max_internal_outlinks = Some(1);
        limits.max_external_outlinks = Some(0);
        p.links = vec![
            anchor("i", true, "NOFOLLOW sponsored UGC"),
            anchor("i", true, ""),
            anchor("e", false, "sponsored ugc"),
            LinkRecord {
                url: "https://fixture.test/script".into(),
                internal: true,
                kind: "script".into(),
                rel: "nofollow".into(),
                ..Default::default()
            },
        ];
        p.links[1].text = "".into();
        p.x_robots_tag = vec!["bingbot: nofollow".into()];
        p.indexability_agent = "googlebot".into();
        let issues = evaluate(&p, &limits);
        for code in [
            "internal_nofollow_outlinks",
            "internal_sponsored_outlinks",
            "internal_ugc_outlinks",
            "external_sponsored_outlinks",
            "external_ugc_outlinks",
            "internal_outlinks_empty_anchor",
            "high_internal_outlinks",
            "high_external_outlinks",
        ] {
            assert!(has(&issues, code), "missing {code}");
        }
        assert!(!has(&issues, "external_nofollow_outlinks"));
        let high = issues
            .iter()
            .find(|issue| issue.code == "high_internal_outlinks")
            .unwrap();
        assert!(high.message.starts_with("2 observed"));
        assert_eq!(high.related_urls.len(), 1);
        p.indexability_agent = "bingbot".into();
        assert!(has(&evaluate(&p, &limits), "external_nofollow_outlinks"));
        p.links.clear();
        assert!(has(&evaluate(&p, &limits), "internal_outlinks_missing"));
    }

    #[test]
    fn url_rules_inspect_decoded_path_query_without_scheme_and_host_false_positives() {
        let mut p = page("A_B//caf%C3%A9/a%20b?Foo=1&bar=2");
        let mut limits = disabled();
        limits.max_url_chars = Some(30);
        let issues = evaluate(&p, &limits);
        for code in [
            "url_uppercase",
            "url_underscores",
            "url_multiple_slashes",
            "url_non_ascii",
            "url_contains_space",
            "url_parameters",
            "url_long",
        ] {
            assert!(has(&issues, code), "missing {code}");
        }
        p.url = "HTTPS://FIXTURE.TEST/plain/path?x=a+b".into();
        let issues = evaluate(&p, &disabled());
        assert!(has(&issues, "url_parameters"));
        for code in [
            "url_uppercase",
            "url_multiple_slashes",
            "url_non_ascii",
            "url_contains_space",
        ] {
            assert!(!has(&issues, code), "false {code}");
        }
        p.url = "https://fixture.test/%2F%FF".into();
        let issues = evaluate(&p, &disabled());
        assert!(!has(&issues, "url_uppercase"));
        assert!(!has(&issues, "url_non_ascii"));
        assert!(!has(&issues, "url_multiple_slashes"));
        p.url = "https://fixture.test/%C3%89".into();
        let issues = evaluate(&p, &disabled());
        assert!(has(&issues, "url_uppercase"));
        assert!(has(&issues, "url_non_ascii"));
    }

    #[test]
    fn incomplete_or_failed_body_capture_never_generates_body_absence_or_length_notices() {
        let mut base = page("UPPER_CASE");
        base.h1.clear();
        base.h2.clear();
        base.titles.clear();
        base.word_count = 0;
        for condition in 0..8 {
            let mut p = base.clone();
            match condition {
                0 => p.truncated = true,
                1 => p.renderer_error = Some("unavailable".into()),
                2 => p.error = Some("timeout".into()),
                3 => p.blocked_by_robots = true,
                4 => p.final_status = Some(404),
                5 => {
                    p.rendered = true;
                    p.rendered_status = Some(500)
                }
                6 => {
                    p.rendered = true;
                    p.rendered_status = None
                }
                _ => p.content_type = "application/pdf".into(),
            }
            let issues = evaluate(&p, &AuditThresholds::default());
            assert!(has(&issues, "url_uppercase"));
            assert!(
                !issues.iter().any(|issue| !issue.code.starts_with("url_")),
                "body inference for condition {condition}"
            );
        }
    }

    #[test]
    fn heading_duplicates_require_distinct_complete_indexable_documents() {
        let mut a = page("a");
        a.h1 = vec![" Ä  shared ".into(), "Ä shared".into()];
        a.h2 = vec!["Section".into()];
        let mut b = page("b");
        b.h1 = vec!["ä shared".into()];
        b.h2 = vec!["SECTION".into()];
        let mut incomplete = page("incomplete");
        incomplete.h1 = b.h1.clone();
        incomplete.truncated = true;
        let mut nonindexable = page("noindex");
        nonindexable.h2 = b.h2.clone();
        nonindexable.indexable = false;
        let mut issues = vec![];
        audit_heading_duplicates(
            &[a.clone(), b.clone(), incomplete, nonindexable],
            &mut issues,
        );
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.code == "h1_duplicate")
                .count(),
            2
        );
        assert_eq!(
            issues
                .iter()
                .filter(|issue| issue.code == "h2_duplicate")
                .count(),
            2
        );
        assert!(
            issues
                .iter()
                .all(|issue| issue.related_urls.len() == 1 && issue.severity == Severity::Notice)
        );
        let mut reverse = vec![];
        audit_heading_duplicates(&[b, a], &mut reverse);
        assert_eq!(
            serde_json::to_value(issues).unwrap(),
            serde_json::to_value(reverse).unwrap()
        );
    }

    #[test]
    fn low_word_count_and_url_length_respect_exact_and_disabled_boundaries() {
        let mut p = page("a");
        let mut limits = disabled();
        limits.min_word_count = Some(200);
        limits.max_url_chars = Some(p.url.chars().count());
        p.word_count = 200;
        let issues = evaluate(&p, &limits);
        assert!(!has(&issues, "content_low_word_count"));
        assert!(!has(&issues, "url_long"));
        p.word_count = 199;
        limits.max_url_chars = Some(p.url.chars().count() - 1);
        let issues = evaluate(&p, &limits);
        assert!(has(&issues, "content_low_word_count"));
        assert!(has(&issues, "url_long"));
        let issues = evaluate(&p, &disabled());
        assert!(!has(&issues, "content_low_word_count"));
        assert!(!has(&issues, "url_long"));
    }
}
