//! Pure, opt-in bookmark checks against captured HTML5 document facts. No requests
//! are made. Redirect fragment propagation, XML/PDF/media and text directives are
//! deliberately not guessed from normalized crawl URL identities.
use crate::{
    CrawlReport, FragmentObservation, FragmentStatus, FragmentTarget, PageRecord, normalize_url,
};
use std::collections::{BTreeMap, HashMap};

/// Borrowed, per-captured-document lookup. Imported observation arrays may be
/// reordered, so lowest recorded DOM position wins; ties keep the first record.
/// IDs always precede legacy anchor names, independently of their DOM positions.
#[derive(Default)]
struct FragmentTargetIndex<'a> {
    ids: HashMap<&'a str, &'a FragmentTarget>,
    names: HashMap<&'a str, &'a FragmentTarget>,
}

impl<'a> FragmentTargetIndex<'a> {
    fn new(targets: &'a [FragmentTarget]) -> Self {
        let mut index = Self::default();
        for target in targets {
            let entries = match target.kind.as_str() {
                "id" => &mut index.ids,
                "legacy_anchor_name" => &mut index.names,
                _ => continue,
            };
            entries
                .entry(target.value.as_str())
                .and_modify(|previous| {
                    if target.dom_location.index < previous.dom_location.index {
                        *previous = target;
                    }
                })
                .or_insert(target);
        }
        index
    }

    fn find(&self, fragment: &str) -> Option<&'a FragmentTarget> {
        self.ids
            .get(fragment)
            .or_else(|| self.names.get(fragment))
            .copied()
    }
}

/// Returns no rows while config.validate_fragments is disabled. Every captured
/// anchor occurrence with a fragment otherwise receives a deterministic result.
pub fn validate_fragments(report: &CrawlReport) -> Vec<FragmentObservation> {
    if !report.config.validate_fragments {
        return Vec::new();
    }
    let mut requested: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut documents: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, page) in report.pages.iter().enumerate() {
        if let Some(url) = normalize_url(&page.url, None) {
            requested.entry(url.to_string()).or_default().push(index);
        }
        if let Some(url) = normalize_url(page.document_url(), None) {
            documents.entry(url.to_string()).or_default().push(index);
        }
    }
    // Build lazily once for each successful, complete DOM that is actually
    // queried. Avoid O(links * target IDs) scans and unused-document allocations.
    let mut target_indexes: Vec<Option<FragmentTargetIndex<'_>>> = std::iter::repeat_with(|| None)
        .take(report.pages.len())
        .collect();
    let mut observations = Vec::new();
    for source in &report.pages {
        for link in source.links.iter().filter(|link| link.kind == "anchor") {
            let Some(fragment) = &link.fragment else {
                continue;
            };
            let mut observation = FragmentObservation {
                source_url: source.url.clone(),
                source_document_url: source.document_url().into(),
                target_url: link.url.clone(),
                resolved_href: link
                    .resolved_href
                    .clone()
                    .unwrap_or_else(|| format!("{}#{fragment}", link.url)),
                fragment: fragment.clone(),
                decoded_fragment: Some(decode_fragment(fragment)),
                source_dom_location: link.dom_location.clone(),
                ..Default::default()
            };
            let target_key = normalize_url(&link.url, None).map(|url| url.to_string());
            let candidates = target_key
                .as_ref()
                .and_then(|url| requested.get(url).or_else(|| documents.get(url)));
            match candidates {
                None => observation.reason = "target_not_observed".into(),
                Some(targets) if targets.len() != 1 => {
                    observation.reason = "target_capture_ambiguous".into()
                }
                Some(targets) => evaluate(
                    &mut observation,
                    &report.pages[targets[0]],
                    &mut target_indexes[targets[0]],
                ),
            }
            observations.push(observation);
        }
    }
    observations.sort_by(|a, b| {
        (
            &a.source_url,
            a.source_dom_location
                .as_ref()
                .map(|location| location.index),
            &a.resolved_href,
            &a.target_url,
        )
            .cmp(&(
                &b.source_url,
                b.source_dom_location
                    .as_ref()
                    .map(|location| location.index),
                &b.resolved_href,
                &b.target_url,
            ))
    });
    observations
}

fn evaluate<'a>(
    observation: &mut FragmentObservation,
    target: &'a PageRecord,
    target_index: &mut Option<FragmentTargetIndex<'a>>,
) {
    observation.target_document_url = Some(target.document_url().into());
    let target_address = normalize_url(target.document_url(), None);
    let declared_address = normalize_url(&observation.target_url, None);
    if target_address != declared_address {
        observation.reason = "redirect_or_navigation_fragment_propagation_unknown".into();
        return;
    }
    if target.blocked_by_robots || target.error.is_some() {
        observation.reason = "target_request_incomplete".into();
        return;
    }
    if target.rendered && target.rendered_status.is_none() {
        observation.reason = "target_render_response_status_unknown".into();
        return;
    }
    if target.final_status.or(target.status) != Some(200)
        || (target.rendered && target.rendered_status != Some(200))
    {
        observation.reason = "target_not_successful_200".into();
        return;
    }
    if target.renderer_error.is_some() || target.truncated {
        observation.reason = "target_capture_incomplete".into();
        return;
    }
    if !target.media_type().eq_ignore_ascii_case("text/html") || !target.is_html() {
        observation.reason = "target_fragment_mime_semantics_unsupported".into();
        observation.status = FragmentStatus::Unsupported;
        return;
    }
    let Some(facts) = &target.dom_metadata else {
        observation.reason = "target_dom_not_observed".into();
        return;
    };
    if !facts.complete {
        observation.reason = "target_dom_collection_incomplete".into();
        return;
    }
    observation.target_complete = true;
    if observation.fragment.contains(":~:") {
        observation.status = FragmentStatus::Unsupported;
        observation.reason = "text_fragment_directive_unsupported".into();
        return;
    }
    if observation.fragment.is_empty() {
        observation.status = FragmentStatus::Top;
        observation.reason = "empty_fragment_document_top".into();
        return;
    }
    let targets =
        target_index.get_or_insert_with(|| FragmentTargetIndex::new(&facts.fragment_targets));
    if let Some(found) = targets.find(&observation.fragment) {
        matched(observation, found, "exact_fragment");
        return;
    }
    let decoded = observation.decoded_fragment.as_deref().unwrap_or("");
    if let Some(found) = targets.find(decoded) {
        matched(observation, found, "percent_decoded_utf8_fragment");
        return;
    }
    if decoded.eq_ignore_ascii_case("top") {
        observation.status = FragmentStatus::Top;
        observation.reason = "top_fragment_document_top".into();
        return;
    }
    if decoded.starts_with('/') || decoded.starts_with('!') {
        observation.status = FragmentStatus::Unsupported;
        observation.reason = "possible_client_router_fragment_unsupported".into();
        return;
    }
    observation.status = FragmentStatus::Missing;
    observation.reason = "no_matching_id_or_legacy_anchor_name_in_complete_captured_html".into();
}

fn matched(observation: &mut FragmentObservation, target: &FragmentTarget, encoding: &str) {
    observation.status = if target.kind == "id" {
        FragmentStatus::MatchedId
    } else {
        FragmentStatus::MatchedName
    };
    observation.reason = encoding.into();
    observation.matched_target = Some(target.clone());
}

fn decode_fragment(fragment: &str) -> String {
    fn digit(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }
    let input = fragment.as_bytes();
    let mut bytes = Vec::with_capacity(input.len());
    let mut at = 0;
    while at < input.len() {
        if input[at] == b'%'
            && let (Some(high), Some(low)) = (
                input.get(at + 1).and_then(|byte| digit(*byte)),
                input.get(at + 2).and_then(|byte| digit(*byte)),
            )
        {
            bytes.push(high * 16 + low);
            at += 3;
        } else {
            bytes.push(input[at]);
            at += 1;
        }
    }
    // HTML uses UTF-8 decode without BOM: malformed sequences are replaced,
    // plus is literal and an encoded percent is decoded only once.
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CrawlConfig, CrawlSummary, DomMetadata, LinkRecord, extract_html};
    use url::Url;
    fn page(path: &str, html: &str) -> PageRecord {
        let address = format!("https://example.org/{path}");
        let mut page = PageRecord {
            url: address.clone(),
            final_url: address,
            status: Some(200),
            final_status: Some(200),
            content_type: "text/html".into(),
            internal: true,
            indexable: true,
            ..Default::default()
        };
        extract_html(
            &mut page,
            html,
            &[Url::parse("https://example.org").unwrap()],
            false,
        );
        page
    }
    fn report(pages: Vec<PageRecord>) -> CrawlReport {
        CrawlReport {
            operation: None,
            policy_ledger: None,
            resource_ledger: None,
            sitemap_catalog: None,
            sitemap_coverage_inventory: None,
            sitemap_source_measurements: None,
            config: CrawlConfig {
                validate_fragments: true,
                ..Default::default()
            },
            pages,
            issues: vec![],
            summary: CrawlSummary::default(),
            warnings: vec![],
        }
    }
    #[test]
    fn exact_then_decoded_ids_names_top_case_and_invalid_utf8() {
        let source = page(
            "a",
            r#"<a href='/b#%41'>literal</a><a href='/b#caf%C3%A9'>unicode</a><a href='/b#old'>legacy</a><a href='/b#TOP'>top</a><a href='/b#'>empty</a><a href='/b#Cafe'>missing</a><a href='/b#%FF'>replacement</a><a href='/b#a+b'>plus</a><a href='/b#%2541'>once</a>"#,
        );
        let target = page(
            "b",
            "<a name='old'></a><div id='%41'></div><div id='A'></div><div id='café'></div><div id='�'></div><div id='a+b'></div>",
        );
        let rows = validate_fragments(&report(vec![source, target]));
        let find = |value: &str| rows.iter().find(|row| row.fragment == value).unwrap();
        assert_eq!(find("%41").matched_target.as_ref().unwrap().value, "%41");
        assert_eq!(find("caf%C3%A9").status, FragmentStatus::MatchedId);
        assert_eq!(find("old").status, FragmentStatus::MatchedName);
        assert_eq!(find("TOP").status, FragmentStatus::Top);
        assert_eq!(find("").status, FragmentStatus::Top);
        assert_eq!(find("Cafe").status, FragmentStatus::Missing);
        assert_eq!(find("%FF").status, FragmentStatus::MatchedId);
        assert_eq!(find("a+b").status, FragmentStatus::MatchedId);
        assert_eq!(find("%2541").matched_target.as_ref().unwrap().value, "%41");
    }
    #[test]
    fn ids_precede_earlier_names_and_duplicate_ids_use_tree_order() {
        let source = page("a", "<a href='/b#x'>x</a>");
        let target = page(
            "b",
            "<a name='x'></a><span id='x'></span><div id='x'></div>",
        );
        let rows = validate_fragments(&report(vec![source, target]));
        assert_eq!(rows[0].status, FragmentStatus::MatchedId);
        assert!(
            rows[0]
                .matched_target
                .as_ref()
                .unwrap()
                .dom_location
                .path
                .as_deref()
                .unwrap()
                .ends_with("/span[1]")
        );
    }
    #[test]
    fn indexed_duplicates_keep_dom_order_and_id_precedence_even_for_reordered_imports() {
        let source = page(
            "a",
            "<a href='/b#x'>ID</a><a href='/b#old'>Name</a><a href='/b#%41'>Raw</a>",
        );
        let mut target = page(
            "b",
            "<a name='x'></a><a name='old'></a><span id='x'></span><div id='x'></div><a name='old'></a><div id='A'></div><div id='%41'></div>",
        );
        let expected_id = target
            .dom_metadata
            .as_ref()
            .unwrap()
            .fragment_targets
            .iter()
            .find(|target| target.kind == "id" && target.value == "x")
            .unwrap()
            .clone();
        let expected_name = target
            .dom_metadata
            .as_ref()
            .unwrap()
            .fragment_targets
            .iter()
            .find(|target| target.kind == "legacy_anchor_name" && target.value == "old")
            .unwrap()
            .clone();
        // Imported report arrays need not have remained sorted by DOM index.
        target
            .dom_metadata
            .as_mut()
            .unwrap()
            .fragment_targets
            .reverse();
        let rows = validate_fragments(&report(vec![source, target]));
        let find = |fragment: &str| rows.iter().find(|row| row.fragment == fragment).unwrap();
        let actual_id = find("x").matched_target.as_ref().unwrap();
        assert_eq!(actual_id.kind, "id");
        assert_eq!(actual_id.dom_location.index, expected_id.dom_location.index);
        assert_eq!(actual_id.dom_location.path, expected_id.dom_location.path);
        let actual_name = find("old").matched_target.as_ref().unwrap();
        assert_eq!(actual_name.kind, "legacy_anchor_name");
        assert_eq!(
            actual_name.dom_location.index,
            expected_name.dom_location.index
        );
        assert_eq!(find("%41").reason, "exact_fragment");
        assert_eq!(find("%41").matched_target.as_ref().unwrap().value, "%41");
    }

    #[test]
    fn large_captured_target_and_many_links_have_deterministic_indexed_results() {
        const COUNT: usize = 10_000;
        let mut source = page("a", "");
        let mut target = page("b", "");
        let mut observations = Vec::with_capacity(COUNT * 2);
        for index in 0..COUNT {
            for (kind, prefix) in [("id", "id"), ("legacy_anchor_name", "name")] {
                let value = format!("{prefix}-{index:05}");
                observations.push(FragmentTarget {
                    kind: kind.into(),
                    value: value.clone(),
                    dom_location: crate::DomLocation {
                        index: observations.len() + 1,
                        ..Default::default()
                    },
                });
                source.links.push(LinkRecord {
                    url: target.url.clone(),
                    resolved_href: Some(format!("{}#{value}", target.url)),
                    fragment: Some(value),
                    kind: "anchor".into(),
                    internal: true,
                    dom_location: Some(crate::DomLocation {
                        index: source.links.len() + 1,
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
        }
        target.dom_metadata.as_mut().unwrap().fragment_targets = observations;
        let mut input = report(vec![source, target]);
        let rows = validate_fragments(&input);
        assert_eq!(rows.len(), COUNT * 2);
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(
                row.status,
                if index % 2 == 0 {
                    FragmentStatus::MatchedId
                } else {
                    FragmentStatus::MatchedName
                }
            );
            assert_eq!(row.reason, "exact_fragment");
            assert_eq!(
                row.matched_target.as_ref().unwrap().dom_location.index,
                index + 1
            );
            assert!(row.target_complete);
        }
        // Report/observation array order is not the matching or output-order policy.
        input.pages.reverse();
        input.pages[0]
            .dom_metadata
            .as_mut()
            .unwrap()
            .fragment_targets
            .reverse();
        assert_eq!(
            serde_json::to_value(&rows).unwrap(),
            serde_json::to_value(validate_fragments(&input)).unwrap()
        );
    }
    #[test]
    fn unavailable_legacy_incomplete_redirect_and_nonhtml_never_prove_missing() {
        let source = page("a", "<a href='/b#absent'>b</a>");
        for variant in 0..9 {
            let mut target = page("b", "<p>Nothing</p>");
            match variant {
                0 => target.dom_metadata = None,
                1 => target.truncated = true,
                2 => target.renderer_error = Some("failed".into()),
                3 => target.final_status = Some(404),
                4 => target.blocked_by_robots = true,
                5 => target.final_url = "https://example.org/c".into(),
                6 => target.content_type = "application/xhtml+xml".into(),
                7 => {
                    target.dom_metadata = Some(DomMetadata {
                        complete: false,
                        ..Default::default()
                    })
                }
                _ => {
                    target.rendered = true;
                    target.rendered_status = None;
                }
            }
            let rows = validate_fragments(&report(vec![source.clone(), target]));
            assert_ne!(rows[0].status, FragmentStatus::Missing, "variant {variant}");
        }
        assert_eq!(
            validate_fragments(&report(vec![source.clone()]))[0].reason,
            "target_not_observed"
        );
        let mut disabled = report(vec![source]);
        disabled.config.validate_fragments = false;
        assert!(validate_fragments(&disabled).is_empty());
    }
    #[test]
    fn imported_rendered_dom_without_response_status_never_proves_target_existence_or_absence() {
        let source = page(
            "a",
            "<a href='/b#present'>present</a><a href='/b#absent'>absent</a>",
        );
        let mut target = page("b", "<div id='present'></div>");
        target.rendered = true;
        target.rendered_status = None;
        target.dom_metadata.as_mut().unwrap().source = "rendered_html".into();
        let rows = validate_fragments(&report(vec![source, target]));
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.status == FragmentStatus::Unknown
            && !row.target_complete
            && row.matched_target.is_none()
            && row.reason == "target_render_response_status_unknown"));
    }
    #[test]
    fn renderer_document_alias_is_used_and_text_router_fragments_are_not_missing() {
        let source = page(
            "a",
            "<a href='/b#x'>x</a><a href='/b#:~:text=hello'>text</a><a href='/b#!/route'>router</a>",
        );
        let mut target = page("original", "<div id='x'></div>");
        target.rendered = true;
        target.rendered_url = Some("https://example.org/b".into());
        target.rendered_status = Some(200);
        let mut r = report(vec![source, target]);
        let rows = validate_fragments(&r);
        assert_eq!(
            rows.iter().find(|row| row.fragment == "x").unwrap().status,
            FragmentStatus::MatchedId
        );
        assert!(
            rows.iter()
                .filter(|row| row.fragment != "x")
                .all(|row| row.status == FragmentStatus::Unsupported)
        );
        r.pages.reverse();
        assert_eq!(
            serde_json::to_value(rows).unwrap(),
            serde_json::to_value(validate_fragments(&r)).unwrap()
        );
        r.pages[0].links.push(LinkRecord {
            url: "https://example.org/b".into(),
            kind: "pdf_annotation".into(),
            fragment: Some("x".into()),
            ..Default::default()
        });
        assert_eq!(validate_fragments(&r).len(), 3);
    }
}
