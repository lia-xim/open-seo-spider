//! Versioned resource contracts: legacy negative evidence and strict controls.
use open_seo_spider_core::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{Notify, Semaphore},
};

#[tokio::test]
async fn legacy_v1_preserves_ugc_and_sponsored_admission_before_contract_opt_in() {
    let fixture = Fixture::new(|_, path| match path {
        "/" => Response::html("<a href='/ugc' rel='ugc'>ugc</a><a href='/sponsored' rel='sponsored'>sponsored</a><a href='/nofollow' rel='nofollow'>control</a>"),
        _ => Response::html("<p>Own target</p>"),
    }).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("legacy-qualifiers.sqlite");
    let result = run(config(&fixture), &path).await;
    assert_eq!(fixture.count("/"), 1);
    assert_eq!(fixture.count("/ugc"), 1);
    assert_eq!(fixture.count("/sponsored"), 1);
    assert_eq!(fixture.count("/nofollow"), 0);
    let rows = ledger(&path);
    assert_eq!(decision(&rows, "/ugc"), "allowed");
    assert_eq!(decision(&rows, "/sponsored"), "allowed");
    assert_eq!(decision(&rows, "/nofollow"), "rel_nofollow");
    assert_eq!(result.outcome, CrawlOutcome::Completed);
    println!("BEFORE_FIX CG-01 legacy_v1 ugc=1 sponsored=1 nofollow=0; preserved legacy behavior");
}

#[tokio::test]
async fn legacy_v1_preserves_html_vs_http_link_nofollow_difference() {
    let fixture = Fixture::new(|origin, path| match path {
        "/" => Response::html("<link rel='canonical nofollow' href='/html'>").header(
            "Link",
            &format!("<{origin}/header>; rel=\"canonical nofollow\""),
        ),
        _ => Response::html("<p>Own target</p>"),
    })
    .await;
    let mut cfg = config(&fixture);
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .navigation
        .canonical
        .fetch = true;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("legacy-header.sqlite");
    run(cfg, &path).await;
    assert_eq!(fixture.count("/"), 1);
    assert_eq!(fixture.count("/html"), 0);
    assert_eq!(fixture.count("/header"), 1);
    let rows = ledger(&path);
    assert_eq!(decision(&rows, "/html"), "rel_nofollow");
    assert_eq!(decision(&rows, &fixture.url("/header")), "allowed");
    println!("BEFORE_FIX CG-02 legacy_v1 HTML canonical nofollow=0 HTTP Link canonical nofollow=1");
}

#[tokio::test]
async fn legacy_v1_preserves_navigation_redirect_via_resource_only_origin() {
    let seed_origin = Arc::new(Mutex::new(String::new()));
    let observed_seed = seed_origin.clone();
    let external = Fixture::new(move |_, path| match path {
        "/landing" => Response::html(&format!(
            "<a href='{}/back'>back</a><a href='/sibling'>sibling</a>",
            observed_seed.lock().unwrap()
        )),
        _ => Response::html("<p>External target</p>"),
    })
    .await;
    let location = external.url("/landing");
    let seed = Fixture::new(move |_, path| match path {
        "/" => Response::html("<a href='/nav'>nav</a>"),
        "/nav" => Response::html("").status(302).header("Location", &location),
        _ => Response::html("<p>Backlink target</p>"),
    })
    .await;
    *seed_origin.lock().unwrap() = seed.origin.clone();
    let mut cfg = config(&seed);
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .resource_scope
        .external_origins
        .push(external.origin.clone());
    let temp = tempfile::tempdir().unwrap();
    run(cfg, &temp.path().join("legacy-navigation-redirect.sqlite")).await;
    assert_eq!(seed.count("/nav"), 1);
    assert_eq!(external.count("/landing"), 1);
    assert_eq!(seed.count("/back"), 1);
    assert_eq!(external.count("/sibling"), 0);
    println!(
        "BEFORE_FIX CG-03 legacy_v1 resource-only external navigation landing=1 backlink=1 sibling=0"
    );
}

#[tokio::test]
async fn legacy_v1_preserves_check_external_resource_redirect_scope_exception() {
    let unlisted = Fixture::new(|_, _| Response::html("<p>Unlisted response</p>")).await;
    let final_url = unlisted.url("/final");
    let listed = Fixture::new(move |_, _| {
        Response::html("")
            .status(302)
            .header("Location", &final_url)
    })
    .await;
    let middle_url = listed.url("/middle");
    let seed = Fixture::new(move |_, path| match path {
        "/" => Response::html("<img src='/asset'>"),
        "/asset" => Response::html("")
            .status(302)
            .header("Location", &middle_url),
        _ => Response::html(""),
    })
    .await;
    let mut cfg = config(&seed);
    cfg.check_external = true;
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .resource_scope
        .external_origins
        .push(listed.origin.clone());
    let temp = tempfile::tempdir().unwrap();
    run(cfg, &temp.path().join("legacy-resource-redirect.sqlite")).await;
    assert_eq!(seed.count("/asset"), 1);
    assert_eq!(listed.count("/middle"), 1);
    assert_eq!(unlisted.count("/final"), 1);
    println!("BEFORE_FIX CG-03 legacy_v1 check_external unlisted resource final=1");
}

fn config(fixture: &Fixture) -> CrawlConfig {
    CrawlConfig {
        seed_urls: vec![fixture.url("/")],
        obey_robots: false,
        discover_sitemaps: false,
        concurrency: 1,
        request_timeout_secs: 5,
        resource_discovery: Some(ResourceDiscoveryConfig::default()),
        ..Default::default()
    }
}

#[test]
fn frozen_b9_config_hash_and_permission_response_serialization_remain_exact() {
    use sha2::Digest;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/resource-contract/legacy-b9-bindings.json"
    ))
    .unwrap();
    let config: CrawlConfig = serde_json::from_value(fixture["config"].clone()).unwrap();
    assert_eq!(
        format!(
            "{:x}",
            sha2::Sha256::digest(serde_json::to_vec(&config).unwrap())
        ),
        fixture["config_sha256"].as_str().unwrap()
    );
    let resource: ResourceDiscoveryConfig =
        serde_json::from_value(fixture["resource_config"].clone()).unwrap();
    assert_eq!(
        resource.sha256().unwrap(),
        fixture["resource_config_sha256"].as_str().unwrap()
    );
    assert!(resource.contract.is_none());
    let permission: ResourceTaskPermission =
        serde_json::from_value(fixture["permission"].clone()).unwrap();
    permission.validate().unwrap();
    assert_eq!(
        serde_json::to_value(permission).unwrap(),
        fixture["permission"]
    );
    let facts: ResourceResponseFacts =
        serde_json::from_value(fixture["response_facts"].clone()).unwrap();
    assert_eq!(
        serde_json::to_value(facts).unwrap(),
        fixture["response_facts"]
    );
}

#[tokio::test]
async fn frozen_plan_directive_ids_replay_v1_49_and_v2_equivalent_33() {
    let plan: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/resource-contract/directive-plan.json"
    ))
    .unwrap();
    let mut observed = 0;
    for group in [
        "nofollow_truth_table",
        "rel_controls",
        "agent_controls",
        "header_qualifier_controls",
    ] {
        for row in plan[group].as_array().unwrap() {
            let id = row["id"].as_str().unwrap();
            for v2 in [false, true] {
                if v2 && group == "nofollow_truth_table" {
                    continue;
                }
                let rel_present = row["rel_nofollow_observed"].as_bool().unwrap_or(false);
                let document = row["effective_document_nofollow"]
                    .as_bool()
                    .unwrap_or(false);
                let mut meta = row["meta_html"].as_str().unwrap_or("").to_owned();
                if document {
                    meta.push_str("<meta name='robots' content='nofollow'>");
                }
                let headers = row["x_robots_header_values"]
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .map(|value| value.as_str().unwrap().to_owned())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let header = row["source"] == "http_link_header";
                let relation = row["relation"].as_str().unwrap_or("").to_owned();
                let rel = row["rel"]
                    .as_str()
                    .unwrap_or(if rel_present { "nofollow" } else { "" })
                    .to_owned();
                let fixture = Fixture::new(move |origin, path| {
                    if path == "/" {
                        let declared = if relation == "alternate_hreflang" {
                            "alternate"
                        } else {
                            relation.as_str()
                        };
                        let extra = if relation == "alternate_hreflang" {
                            "hreflang='de'"
                        } else {
                            ""
                        };
                        let html = if relation.is_empty() {
                            format!("{meta}<a href='/target' rel='{rel}'>Own target</a>")
                        } else if header {
                            format!("{meta}<p>Own header source</p>")
                        } else {
                            format!("{meta}<link href='/target' rel='{declared} nofollow' {extra}>")
                        };
                        let mut response = Response::html(&html);
                        for value in &headers {
                            response = response.header("X-Robots-Tag", value);
                        }
                        if header {
                            response = response.header(
                                "Link",
                                &format!(
                                    "<{origin}/target>; rel=\"{declared} nofollow\"{}",
                                    if relation == "alternate_hreflang" {
                                        "; hreflang=\"de\""
                                    } else {
                                        ""
                                    }
                                ),
                            );
                        }
                        response
                    } else {
                        Response::html("<p>Own target response</p>")
                    }
                })
                .await;
                let mut cfg = if v2 {
                    strict(&fixture)
                } else {
                    config(&fixture)
                };
                cfg.indexability_agent = row["indexability_agent"]
                    .as_str()
                    .unwrap_or("googlebot")
                    .into();
                cfg.user_agent = row["transport_user_agent"]
                    .as_str()
                    .unwrap_or("OwnTransport/1")
                    .into();
                let resource = cfg.resource_discovery.as_mut().unwrap();
                resource.navigation.canonical.fetch = true;
                resource.navigation.pagination.fetch = true;
                resource.navigation.hreflang.fetch = true;
                resource.navigation.amp.fetch = true;
                resource.follow_rel_nofollow =
                    row["follow_rel_nofollow"].as_bool().unwrap_or(false);
                resource.follow_document_nofollow =
                    row["follow_document_nofollow"].as_bool().unwrap_or(false);
                let expected_gets = if group == "nofollow_truth_table" {
                    row["target_document_gets"].as_u64().unwrap() as usize
                } else if group == "agent_controls" {
                    usize::from(!row["current_v1_effective_nofollow"].as_bool().unwrap())
                } else if group == "header_qualifier_controls" {
                    if v2 {
                        0
                    } else {
                        row["current_v1_gets"].as_u64().unwrap() as usize
                    }
                } else {
                    usize::from(if v2 {
                        !["nofollow", "ugc", "sponsored"]
                            .iter()
                            .any(|word| rel_token_case(row["rel"].as_str().unwrap(), word))
                    } else {
                        !row["current_v1_nofollow"].as_bool().unwrap()
                    })
                };
                let temp = tempfile::tempdir().unwrap();
                let path = temp.path().join("frozen-plan.sqlite");
                let result = run(cfg, &path).await;
                assert_eq!(fixture.count("/target"), expected_gets, "id={id},v2={v2}");
                let rows = ledger(&path);
                let target = rows
                    .iter()
                    .find(|item| {
                        item.occurrence.resolved_url.as_deref()
                            == Some(fixture.url("/target").as_str())
                    })
                    .unwrap();
                if group == "nofollow_truth_table" {
                    assert_eq!(
                        target.decision.reason,
                        row["current_v1_reason"].as_str().unwrap(),
                        "id={id}"
                    );
                }
                assert_eq!(target.decision.qualification.is_some(), v2);
                assert_eq!(target.decision.grant.is_some(), expected_gets == 1);
                if group == "agent_controls" {
                    let page = result
                        .report
                        .pages
                        .iter()
                        .find(|page| page.url == fixture.url("/"))
                        .unwrap();
                    assert_eq!(
                        page.effective_robots
                            .iter()
                            .any(|value| value == "nofollow" || value == "none"),
                        row["current_v1_effective_nofollow"].as_bool().unwrap(),
                        "id={id}"
                    );
                }
                println!(
                    "PLAN_DIRECTIVE id={id} profile={} observed_gets={expected_gets} PASS",
                    if v2 {
                        "v2_equivalent"
                    } else {
                        "v1_frozen_contract"
                    }
                );
                observed += 1;
            }
        }
    }
    assert_eq!(observed, 82);
}
fn rel_token_case(value: &str, word: &str) -> bool {
    value
        .split_ascii_whitespace()
        .any(|token| token.eq_ignore_ascii_case(word))
}
fn strict(fixture: &Fixture) -> CrawlConfig {
    let mut cfg = config(fixture);
    cfg.resource_discovery.as_mut().unwrap().contract = Some(ResourceContractConfig::default());
    cfg
}
fn qualifier<'a>(
    rows: &'a [StoredResourceOccurrence],
    raw: &str,
) -> &'a ResourceQualificationDecision {
    rows.iter()
        .find(|row| row.occurrence.raw_value == raw)
        .unwrap()
        .decision
        .qualification
        .as_ref()
        .unwrap()
}

#[tokio::test]
async fn strict_v2_rel_token_controls_eleven_native_cases() {
    let cases = [
        ("plain", "", false),
        ("nofollow", "nofollow", true),
        ("case", "NOFOLLOW", true),
        ("whitespace", " \tnofollow\n ", true),
        ("prefix", "nofollowish", false),
        ("ordinary", "noopener noreferrer", false),
        ("ugc", "ugc", true),
        ("sponsored", "sponsored", true),
        ("ugc_nofollow", "ugc nofollow", true),
        ("sponsored_nofollow", "sponsored nofollow", true),
        ("comma_is_not_html_whitespace", "ugc,nofollow", false),
    ];
    for (name, rel, qualified) in cases {
        let rel = rel.to_owned();
        let fixture = Fixture::new(move |_, path| {
            if path == "/" {
                Response::html(&format!("<a href='/target' rel='{rel}'>Own target</a>"))
            } else {
                Response::html("<p>Own response</p>")
            }
        })
        .await;
        for follow in [false, true] {
            let mut cfg = strict(&fixture);
            cfg.resource_discovery
                .as_mut()
                .unwrap()
                .contract
                .as_mut()
                .unwrap()
                .follow_internal_qualified_links = follow;
            let before = fixture.count("/target");
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("tokens.sqlite");
            run(cfg, &path).await;
            assert_eq!(
                fixture.count("/target") - before,
                usize::from(!qualified || follow),
                "case={name},follow={follow}"
            );
            let rows = ledger(&path);
            let facts = qualifier(&rows, "/target");
            assert_eq!(facts.scope, ResourceLinkScope::Internal);
            assert_eq!(facts.source, ResourceQualifierSource::HtmlRel);
            assert_eq!(!facts.rel_qualifiers.is_empty(), qualified, "case={name}");
            assert!(!facts.document_nofollow);
            assert_eq!(facts.follow_allowed, !qualified || follow);
        }
    }
    println!("V2 DIRECTIVE rel-token controls=11, actual scoped follow controls=22");
}

#[tokio::test]
async fn strict_v2_qualifier_document_scoped_truth_table_sixteen_native_cases() {
    for rel_present in [false, true] {
        for document in [false, true] {
            for follow_internal in [false, true] {
                for follow_external in [false, true] {
                    let external =
                        Fixture::new(|_, _| Response::html("<p>External own fixture</p>")).await;
                    let external_url = external.url("/target");
                    let target = external_url.clone();
                    let fixture = Fixture::new(move |_, path| if path == "/" {
                Response::html(&format!("{}<a href='/internal' rel='{}'>Internal</a><a href='{target}' rel='{}'>External</a>",
                    if document { "<meta name='robots' content='nofollow'>" } else { "" },
                    if rel_present { "nofollow" } else { "" }, if rel_present { "ugc sponsored" } else { "" }))
            } else { Response::html("<p>Own internal</p>") }).await;
                    let mut cfg = strict(&fixture);
                    cfg.check_external = true;
                    let resource = cfg.resource_discovery.as_mut().unwrap();
                    resource.navigation.external_anchors.fetch = true;
                    resource
                        .contract
                        .as_mut()
                        .unwrap()
                        .follow_internal_qualified_links = follow_internal;
                    resource
                        .contract
                        .as_mut()
                        .unwrap()
                        .follow_external_qualified_links = follow_external;
                    let temp = tempfile::tempdir().unwrap();
                    let path = temp.path().join("truth.sqlite");
                    run(cfg, &path).await;
                    let qualifies = rel_present || document;
                    assert_eq!(
                        fixture.count("/internal"),
                        usize::from(!qualifies || follow_internal)
                    );
                    assert_eq!(
                        external.count("/target"),
                        usize::from(!qualifies || follow_external)
                    );
                    let rows = ledger(&path);
                    for (raw, scope, enabled) in [
                        ("/internal", ResourceLinkScope::Internal, follow_internal),
                        (
                            external_url.as_str(),
                            ResourceLinkScope::External,
                            follow_external,
                        ),
                    ] {
                        let facts = qualifier(&rows, raw);
                        assert_eq!(facts.document_nofollow, document);
                        assert_eq!(facts.scope, scope);
                        assert_eq!(facts.follow_allowed, !qualifies || enabled);
                    }
                }
            }
        }
    }
    println!("V2 DIRECTIVE truth-table controls=16, actual internal/external target outcomes=32");
}

#[tokio::test]
async fn strict_v2_directive_agents_and_channels_fourteen_native_cases() {
    // Transport UA deliberately differs from the configured indexing agent.
    let cases = [
        (
            "generic_meta",
            "<meta name='robots' content='nofollow'>",
            vec![],
            "googlebot",
            true,
        ),
        (
            "none",
            "<meta name='robots' content='none'>",
            vec![],
            "googlebot",
            true,
        ),
        (
            "noindex_only",
            "<meta name='robots' content='noindex'>",
            vec![],
            "googlebot",
            false,
        ),
        (
            "matched_meta",
            "<meta name='GOOGLEBOT' content='nofollow'>",
            vec![],
            "googlebot",
            true,
        ),
        (
            "unmatched_transport_meta",
            "<meta name='bingbot' content='nofollow'>",
            vec![],
            "googlebot",
            false,
        ),
        (
            "custom_meta",
            "<meta name='ownbot' content='nofollow'>",
            vec![],
            "ownbot",
            true,
        ),
        ("header_unscoped", "", vec!["nofollow"], "googlebot", true),
        (
            "header_matched",
            "",
            vec!["Googlebot: nofollow"],
            "googlebot",
            true,
        ),
        (
            "header_unmatched",
            "",
            vec!["bingbot: nofollow"],
            "googlebot",
            false,
        ),
        (
            "header_wildcard",
            "",
            vec!["*: nofollow"],
            "googlebot",
            true,
        ),
        (
            "header_scope_persists",
            "",
            vec!["bingbot: noindex, nofollow"],
            "googlebot",
            false,
        ),
        (
            "separate_fields_reset_scope",
            "",
            vec!["bingbot: noindex", "nofollow"],
            "googlebot",
            true,
        ),
        (
            "restrictive_union",
            "<meta name='robots' content='nofollow'><meta name='googlebot' content='follow'>",
            vec![],
            "googlebot",
            true,
        ),
        (
            "valued_then_boolean",
            "<meta name='robots' content='max-snippet:50 nofollow'>",
            vec![],
            "googlebot",
            true,
        ),
    ];
    for (name, meta, headers, agent, document) in cases {
        let fixture = Fixture::new(move |_, path| {
            if path == "/" {
                let mut response = Response::html(&format!("{meta}<a href='/target'>Own link</a>"));
                for header in &headers {
                    response = response.header("X-Robots-Tag", header);
                }
                response
            } else {
                Response::html("<p>Own target</p>")
            }
        })
        .await;
        for follow in [false, true] {
            let mut cfg = strict(&fixture);
            cfg.indexability_agent = agent.into();
            cfg.user_agent = "Bingbot/own-fixture".into();
            cfg.resource_discovery
                .as_mut()
                .unwrap()
                .contract
                .as_mut()
                .unwrap()
                .follow_internal_qualified_links = follow;
            let before = fixture.count("/target");
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join("agent.sqlite");
            let result = run(cfg, &path).await;
            assert_eq!(
                fixture.count("/target") - before,
                usize::from(!document || follow),
                "case={name},follow={follow}"
            );
            assert_eq!(
                qualifier(&ledger(&path), "/target").document_nofollow,
                document,
                "case={name}"
            );
            let source = result
                .report
                .pages
                .iter()
                .find(|page| page.url == fixture.url("/"))
                .unwrap();
            assert_eq!(
                source
                    .effective_robots
                    .iter()
                    .any(|value| value == "nofollow" || value == "none"),
                document,
                "case={name}"
            );
        }
    }
    println!("V2 DIRECTIVE agent/channel controls=14, actual follow outcomes=28");
}

#[tokio::test]
async fn strict_v2_html_and_http_link_relationships_eight_native_cases() {
    for (relation, kind, extra) in [
        ("canonical", ResourceReferenceKind::Canonical, ""),
        ("next", ResourceReferenceKind::Pagination, ""),
        (
            "alternate",
            ResourceReferenceKind::Hreflang,
            "hreflang='de'",
        ),
        ("amphtml", ResourceReferenceKind::Amp, ""),
    ] {
        for header in [false, true] {
            let fixture = Fixture::new(move |origin, path| {
                if path == "/" {
                    if header {
                        Response::html("<p>HTTP relation source</p>").header(
                            "Link",
                            &format!(
                                "<{origin}/target>; rel=\"{relation} nofollow\"{}",
                                if relation == "alternate" {
                                    "; hreflang=\"de\""
                                } else {
                                    ""
                                }
                            ),
                        )
                    } else {
                        Response::html(&format!(
                            "<link rel='{relation} nofollow' href='/target' {extra}>"
                        ))
                    }
                } else {
                    Response::html("<p>Own relation target</p>")
                }
            })
            .await;
            for follow in [false, true] {
                let mut cfg = strict(&fixture);
                let resource = cfg.resource_discovery.as_mut().unwrap();
                resource.navigation.canonical.fetch = true;
                resource.navigation.pagination.fetch = true;
                resource.navigation.hreflang.fetch = true;
                resource.navigation.amp.fetch = true;
                resource
                    .contract
                    .as_mut()
                    .unwrap()
                    .follow_internal_qualified_links = follow;
                let temp = tempfile::tempdir().unwrap();
                let path = temp.path().join("relations.sqlite");
                let before = fixture.count("/target");
                run(cfg, &path).await;
                assert_eq!(
                    fixture.count("/target") - before,
                    usize::from(follow),
                    "relation={relation},header={header}"
                );
                let rows = ledger(&path);
                let row = rows
                    .iter()
                    .find(|row| row.occurrence.reference_kind == kind)
                    .unwrap();
                assert_eq!(
                    row.occurrence.attributes.get("rel").unwrap(),
                    &format!("{relation} nofollow")
                );
                let facts = row.decision.qualification.as_ref().unwrap();
                assert_eq!(facts.rel_qualifiers, ["nofollow"]);
                assert_eq!(
                    facts.source,
                    if header {
                        ResourceQualifierSource::HttpLinkRel
                    } else {
                        ResourceQualifierSource::HtmlRel
                    }
                );
                assert!(!facts.document_nofollow);
                assert_eq!(facts.follow_allowed, follow);
                assert_eq!(
                    row.decision.reason,
                    if follow { "allowed" } else { "rel_nofollow" }
                );
            }
        }
    }
    println!(
        "V2 DIRECTIVE equivalent HTML/HTTP controls=8, actual follow outcomes=16; total directive groups=49"
    );
}

#[tokio::test]
async fn strict_v2_navigation_redirect_cannot_borrow_resource_allowlist() {
    let external = Fixture::new(|_, _| Response::html("<a href='/child'>Child</a>")).await;
    let landing = external.url("/landing");
    let fixture = Fixture::new(move |_, path| match path {
        "/" => Response::html("<a href='/nav'>Nav</a>"),
        "/nav" => Response::html("").status(302).header("Location", &landing),
        _ => Response::html(""),
    })
    .await;
    let mut cfg = strict(&fixture);
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .resource_scope
        .external_origins
        .push(external.origin.clone());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("navigation.sqlite");
    let result = run(cfg, &path).await;
    assert_eq!(fixture.count("/nav"), 1);
    assert_eq!(external.count("/landing"), 0);
    assert_eq!(external.count("/child"), 0);
    let page = result
        .report
        .pages
        .iter()
        .find(|page| page.url == fixture.url("/nav"))
        .unwrap();
    let hop = &page.resource_response.as_ref().unwrap().redirect_decisions[0];
    assert_eq!(
        hop.original_reference_kinds,
        [ResourceReferenceKind::InternalAnchor]
    );
    assert_eq!(hop.reason, "redirect_role_scope_denied");
    assert!(!hop.request_started);
    assert!(!hop.allowed);
    assert_eq!(page.status, Some(302));
    assert_eq!(page.final_status, None);
    assert_eq!(
        CrawlStore::open_read_only(&path)
            .unwrap()
            .resource_occurrences(None, 100)
            .unwrap()
            .snapshot
            .unwrap()
            .resource_contract_engine
            .as_deref(),
        Some(RESOURCE_CONTRACT_ENGINE)
    );
}

#[tokio::test]
async fn strict_v2_resource_redirect_cannot_borrow_check_external() {
    let unlisted = Fixture::new(|_, _| Response::html("<p>Unlisted</p>")).await;
    let final_url = unlisted.url("/final");
    let listed = Fixture::new(move |_, _| {
        Response::html("")
            .status(302)
            .header("Location", &final_url)
    })
    .await;
    let middle = listed.url("/middle");
    let fixture = Fixture::new(move |_, path| match path {
        "/" => Response::html("<img src='/asset'>"),
        "/asset" => Response::html("").status(302).header("Location", &middle),
        _ => Response::html(""),
    })
    .await;
    let mut cfg = strict(&fixture);
    cfg.check_external = true;
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .resource_scope
        .external_origins
        .push(listed.origin.clone());
    let temp = tempfile::tempdir().unwrap();
    let result = run(cfg, &temp.path().join("resource.sqlite")).await;
    assert_eq!(listed.count("/middle"), 1);
    assert_eq!(unlisted.count("/final"), 0);
    let facts = result
        .report
        .pages
        .iter()
        .find(|page| page.url == fixture.url("/asset"))
        .unwrap()
        .resource_response
        .as_ref()
        .unwrap();
    assert_eq!(facts.redirect_decisions.len(), 2);
    assert!(facts.redirect_decisions[0].request_started);
    assert_eq!(
        facts.redirect_decisions[1].reason,
        "redirect_role_scope_denied"
    );
    assert!(!facts.redirect_decisions[1].request_started);
    assert!(
        facts
            .redirect_decisions
            .iter()
            .all(|hop| hop.original_reference_kinds == [ResourceReferenceKind::Image])
    );
}

#[tokio::test]
async fn strict_v2_pre_http_validation_rejects_unknown_or_contradictory_profiles() {
    let fixture = Fixture::new(|_, _| Response::html("<p>No request expected</p>")).await;
    for case in 0..3 {
        let mut cfg = strict(&fixture);
        let resource = cfg.resource_discovery.as_mut().unwrap();
        match case {
            0 => resource.contract.as_mut().unwrap().engine = "future_unknown".into(),
            1 => resource.follow_rel_nofollow = true,
            _ => resource.follow_document_nofollow = true,
        }
        assert!(Crawler::new(cfg).is_err());
    }
    assert_eq!(fixture.count("/"), 0);
}

#[tokio::test]
async fn strict_v2_each_hop_preserves_original_roles_credentials_and_robots() {
    let seed_origin = Arc::new(Mutex::new(String::new()));
    let seed_address = seed_origin.clone();
    let external = Fixture::new(move |_, path| match path {
        "/robots.txt" => Response::html("User-agent: *\nDisallow: /denied\n"),
        "/middle" => Response::html("").status(302).header(
            "Location",
            &format!("{}/return", seed_address.lock().unwrap()),
        ),
        _ => Response::html("<p>External own response</p>"),
    })
    .await;
    let middle = external.url("/middle");
    let denied = external.url("/denied");
    let seed = Fixture::new(move |_, path| match path {
        "/robots.txt" => Response::html("User-agent: *\nAllow: /\n"),
        "/" => Response::html("<img src='/asset'><a href='/nav'>Own nav</a>"),
        "/asset" => Response::html("")
            .status(302)
            .header("Location", &middle)
            .header("X-Robots-Tag", "nofollow"),
        "/nav" => Response::html("").status(302).header("Location", &denied),
        "/return" => Response::html("<a href='/child'>No image role expansion</a>"),
        _ => Response::html(""),
    })
    .await;
    *seed_origin.lock().unwrap() = seed.origin.clone();
    let mut cfg = strict(&seed);
    cfg.check_external = true;
    cfg.obey_robots = true;
    cfg.custom_headers
        .insert("Authorization".into(), "Bearer own_private_fixture".into());
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .resource_scope
        .external_origins
        .push(external.origin.clone());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("credentials.sqlite");
    let result = run(cfg, &path).await;
    assert_eq!(seed.count("/asset"), 1);
    assert_eq!(external.count("/middle"), 1);
    assert_eq!(seed.count("/return"), 1);
    assert_eq!(external.count("/denied"), 0);
    assert_eq!(external.count("/robots.txt"), 1);
    assert_eq!(seed.count("/child"), 0);
    for requested in ["/asset", "/return"] {
        assert!(
            seed.requests(requested)
                .iter()
                .all(|raw| raw.contains("Bearer own_private_fixture"))
        );
    }
    for requested in ["/middle", "/robots.txt"] {
        assert!(
            external
                .requests(requested)
                .iter()
                .all(|raw| !raw.to_ascii_lowercase().contains("authorization:"))
        );
    }
    let asset = result
        .report
        .pages
        .iter()
        .find(|page| page.url == seed.url("/asset"))
        .unwrap();
    assert!(
        !asset.effective_robots.contains(&"nofollow".into()),
        "302 directives must not become terminal document facts"
    );
    let hops = &asset.resource_response.as_ref().unwrap().redirect_decisions;
    assert_eq!(hops.len(), 2);
    assert!(
        hops.iter()
            .all(|hop| hop.allowed && hop.robots_allowed == Some(true) && hop.request_started)
    );
    assert_eq!(
        hops[0].authorizing_occurrence_ids,
        hops[1].authorizing_occurrence_ids
    );
    let nav = result
        .report
        .pages
        .iter()
        .find(|page| page.url == seed.url("/nav"))
        .unwrap();
    let hop = &nav.resource_response.as_ref().unwrap().redirect_decisions[0];
    assert_eq!(hop.reason, "redirect_robots_denied");
    assert_eq!(hop.robots_allowed, Some(false));
    assert!(!hop.request_started);
    assert!(nav.blocked_by_robots);
    assert_eq!(
        nav.resource_response
            .as_ref()
            .unwrap()
            .captured_entity_sha256,
        None
    );
    ledger(&path); // The same immutable facts survive the verified read-only snapshot.
}

#[tokio::test]
async fn strict_v2_coalesced_roles_cannot_switch_authority_mid_chain() {
    let seed_origin = Arc::new(Mutex::new(String::new()));
    let address = seed_origin.clone();
    let external = Fixture::new(move |_, _| {
        Response::html("")
            .status(302)
            .header("Location", &format!("{}/end", address.lock().unwrap()))
    })
    .await;
    let middle = external.url("/middle");
    let seed = Fixture::new(move |_, path| match path {
        "/" => Response::html("<img src='/shared'><a href='/shared'>Independent nav</a>"),
        "/shared" => Response::html("").status(302).header("Location", &middle),
        "/end" => Response::html(
            "<a href='/child'>Must remain image check-only</a><img src='/child-image'>",
        ),
        _ => Response::html(""),
    })
    .await;
    *seed_origin.lock().unwrap() = seed.origin.clone();
    let mut cfg = strict(&seed);
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .resource_scope
        .external_origins
        .push(external.origin.clone());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("coalesced.sqlite");
    let result = run(cfg, &path).await;
    assert_eq!(seed.count("/shared"), 1);
    assert_eq!(external.count("/middle"), 1);
    assert_eq!(seed.count("/end"), 1);
    assert_eq!(seed.count("/child"), 0);
    assert_eq!(seed.count("/child-image"), 0);
    let page = result
        .report
        .pages
        .iter()
        .find(|page| page.url == seed.url("/shared"))
        .unwrap();
    let permission = page.resource_permission.as_ref().unwrap();
    assert!(permission.can_expand_navigation);
    assert_eq!(permission.authorizing_grants.len(), 2);
    let hops = &page.resource_response.as_ref().unwrap().redirect_decisions;
    let nav_id = &permission
        .authorizing_grants
        .iter()
        .find(|grant| grant.reference_kind == Some(ResourceReferenceKind::InternalAnchor))
        .unwrap()
        .occurrence_id;
    assert_eq!(hops[0].original_reference_kinds.len(), 2);
    assert!(
        hops.iter()
            .all(|hop| !hop.authorizing_occurrence_ids.contains(nav_id))
    );
    let rows = ledger(&path);
    assert_eq!(decision(&rows, "/child"), "source_check_only");
}

#[tokio::test]
async fn strict_v2_late_navigation_grant_rechecks_actual_redirect_chain_without_refetch() {
    let seed_origin = Arc::new(Mutex::new(String::new()));
    let address = seed_origin.clone();
    let external = Fixture::new(move |_, _| {
        Response::html(&format!(
            "<a href='{}/child'>Own child</a>",
            address.lock().unwrap()
        ))
    })
    .await;
    let landing = external.url("/landing");
    let seed = Fixture::new(move |_, path| match path {
        "/" => Response::html("<img src='/shared'><a href='/later'>Later independent source</a>"),
        "/shared" => Response::html("").status(302).header("Location", &landing),
        "/later" => Response::html("<a href='/shared'>Late nav grant</a>"),
        _ => Response::html("<p>Own target</p>"),
    })
    .await;
    *seed_origin.lock().unwrap() = seed.origin.clone();
    let mut cfg = strict(&seed);
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .resource_scope
        .external_origins
        .push(external.origin.clone());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("late.sqlite");
    run(cfg, &path).await;
    assert_eq!(seed.count("/shared"), 1);
    assert_eq!(external.count("/landing"), 1);
    assert_eq!(seed.count("/later"), 1);
    assert_eq!(seed.count("/child"), 0);
    let rows = ledger(&path);
    let child_rows = rows
        .iter()
        .filter(|row| row.occurrence.resolved_url.as_deref() == Some(seed.url("/child").as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        child_rows.len(),
        2,
        "Late role has an independent evaluation, retaining original observation"
    );
    assert!(
        child_rows
            .iter()
            .all(|row| row.decision.reason == "source_check_only")
    );
}

#[tokio::test]
async fn strict_v2_pause_recovery_preserves_profile_and_selected_never_expands() {
    let gate = Arc::new(Semaphore::new(0));
    let held = gate.clone();
    let fixture = Fixture::new(move |_, path| match path {
        "/" => Response::html("<a href='/active'>Active</a><a href='/pending'>Pending</a>"),
        "/active" => Response::html("<a href='/blocked' rel='sponsored'>Blocked</a>").gated(&held),
        "/pending" => Response::html("<a href='/leaf'>New leaf</a>"),
        _ => Response::html("<p>Own target</p>"),
    })
    .await;
    let cfg = strict(&fixture);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("paused.sqlite");
    let control = CrawlControl::new();
    let handle = control.clone();
    let crawler = Crawler::new(cfg.clone())
        .unwrap()
        .with_store(&path)
        .unwrap();
    let running = tokio::spawn(async move { crawler.run_controlled(None, handle).await.unwrap() });
    fixture.started("/active").await;
    assert!(control.request_pause());
    gate.add_permits(1);
    let paused = running.await.unwrap();
    assert_eq!(paused.outcome, CrawlOutcome::Paused);
    assert_eq!(fixture.count("/pending"), 0);
    assert_eq!(fixture.count("/blocked"), 0);
    let inspected = CrawlStore::inspect(&path).unwrap();
    assert!(inspected.checkpoint_valid);
    assert!(inspected.pending > 0);
    assert_eq!(
        inspected
            .config
            .unwrap()
            .resource_discovery
            .unwrap()
            .contract
            .unwrap()
            .engine,
        RESOURCE_CONTRACT_ENGINE
    );
    let before = ledger(&path);
    let checkpoint_bytes = serde_json::to_vec(&paused.report).unwrap();
    let resumed = run(cfg, &path).await;
    assert_eq!(resumed.outcome, CrawlOutcome::Completed);
    assert_eq!(fixture.count("/active"), 1);
    assert_eq!(fixture.count("/pending"), 1);
    assert_eq!(fixture.count("/leaf"), 1);
    let after = ledger(&path);
    assert!(before.iter().all(|row| after.contains(row)));
    assert_eq!(decision(&after, "/blocked"), "rel_sponsored");
    let parent = prepare_selected_recrawl(
        SelectedRecrawlParent {
            job_id: "own-paused-parent".into(),
            checkpoint_revision: paused.checkpoint.as_ref().map(|value| value.revision),
            source_kind: RecrawlSourceKind::CapturedCheckpoint,
        },
        &checkpoint_bytes,
        &[fixture.url("/")],
        None,
    )
    .unwrap();
    let child_path = temp.path().join("selected.sqlite");
    let saved_bytes = parent.base_snapshot_bytes.clone();
    let selected = Crawler::new(parent.config.clone())
        .unwrap()
        .with_store(&child_path)
        .unwrap()
        .with_selected_recrawl(parent)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(selected.outcome, CrawlOutcome::Completed);
    assert_eq!(fixture.count("/"), 2);
    assert_eq!(fixture.count("/active"), 1);
    assert_eq!(fixture.count("/pending"), 1);
    assert_eq!(fixture.count("/blocked"), 0);
    assert_eq!(
        CrawlStore::open_read_only(&child_path)
            .unwrap()
            .parent_snapshot_bytes()
            .unwrap()
            .unwrap(),
        saved_bytes
    );
    let rows = ledger(&child_path);
    assert!(
        rows.iter()
            .all(|row| row.decision.reason == "selected_no_expansion")
    );
}

#[tokio::test]
async fn legacy_v1_sqlite_resume_retains_absent_profile_and_original_bytes() {
    let fixture = Fixture::new(|_, path| {
        if path == "/" {
            Response::html("<a href='/ugc' rel='ugc'>Legacy</a>")
        } else {
            Response::html("<p>Own response</p>")
        }
    })
    .await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("legacy.sqlite");
    let cfg = config(&fixture);
    let result = run(cfg.clone(), &path).await;
    let store = CrawlStore::open_read_only(&path).unwrap();
    let before = store.resource_occurrences(None, 100).unwrap();
    assert!(
        before
            .snapshot
            .as_ref()
            .unwrap()
            .resource_contract_engine
            .is_none()
    );
    let bytes = serde_json::to_vec(&result.report).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("resource_contract_engine"));
    assert!(!String::from_utf8_lossy(&bytes).contains("\"reference_kind\":null"));
    assert!(!String::from_utf8_lossy(&bytes).contains("\"qualification\""));
    assert!(
        !serde_json::to_string(result.report.config.resource_discovery.as_ref().unwrap())
            .unwrap()
            .contains("contract")
    );
    let again = run(cfg, &path).await;
    assert_eq!(fixture.count("/"), 1);
    assert_eq!(fixture.count("/ugc"), 1);
    assert_eq!(before.rows, ledger(&path));
    assert!(
        again
            .report
            .config
            .resource_discovery
            .unwrap()
            .contract
            .is_none()
    );
}

#[tokio::test]
async fn strict_v2_store_and_late_store_cannot_borrow_a_scope_denied_grant() {
    let external = Fixture::new(|_, _| Response::html("<p>Observed external response</p>")).await;
    let landing = external.url("/landing");
    let fixture = Fixture::new(move |_, path| match path {
        "/" => Response::html(
            "<a href='/shared'>Check-only nav</a><a href='/later'>Late image source</a>",
        ),
        "/shared" => Response::html("").status(302).header("Location", &landing),
        "/later" => Response::html("<img src='/shared'>"),
        _ => Response::html(""),
    })
    .await;
    let mut cfg = strict(&fixture);
    cfg.check_external = true;
    let resource = cfg.resource_discovery.as_mut().unwrap();
    resource.navigation.internal_anchors = NavigationSwitch {
        store: false,
        fetch: true,
        expand: true,
    };
    // /later must expand so it can contribute the independent Image store grant.
    resource.resources.images.store = true;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("retention.sqlite");
    let result = run(cfg, &path).await;
    assert_eq!(fixture.count("/shared"), 1);
    assert_eq!(external.count("/landing"), 1);
    assert_eq!(fixture.count("/later"), 1);
    assert!(
        !result
            .report
            .pages
            .iter()
            .any(|page| page.url == fixture.url("/shared"))
    );
    let store = CrawlStore::open_read_only(&path).unwrap();
    let transports = store.resource_transports(None, 100).unwrap();
    let transport = transports
        .rows
        .iter()
        .find(|row| row.requested_url == fixture.url("/shared"))
        .unwrap();
    assert!(!transport.retained_response);
    assert!(
        !transport
            .projection
            .resource_response
            .as_ref()
            .unwrap()
            .retained_response
    );
    let conn = rusqlite::Connection::open(&path).unwrap();
    let data: String = conn
        .query_row(
            "SELECT data FROM resource_tasks WHERE url=?1",
            [fixture.url("/shared")],
            |row| row.get(0),
        )
        .unwrap();
    let saved: serde_json::Value = serde_json::from_str(&data).unwrap();
    let permission: ResourceTaskPermission =
        serde_json::from_value(saved["permission"].clone()).unwrap();
    assert!(
        permission.store,
        "Stored original grants retain their immutable capabilities"
    );
    assert!(
        permission
            .authorizing_grants
            .iter()
            .any(|grant| grant.reference_kind == Some(ResourceReferenceKind::Image) && grant.store)
    );
    assert!(CrawlStore::inspect(&path).unwrap().checkpoint_valid);
}

#[tokio::test]
async fn strict_v2_selected_scope_limited_attempt_retains_only_bound_private_projection() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let external = Fixture::new(|_, _| {
        Response::html(
            "<p>New external complete text must not be retained through Image store grant.</p>",
        )
    })
    .await;
    let landing = external.url("/landing");
    let changed = Arc::new(AtomicBool::new(false));
    let switch = changed.clone();
    let fixture = Fixture::new(move |_, path| match path {
        "/" => Response::html("<img src='/shared'><a href='/shared'>Check-only nav</a>"),
        "/shared" if switch.load(Ordering::SeqCst) => {
            Response::html("").status(302).header("Location", &landing)
        }
        _ => Response::html("<p>Original complete source text from own retained parent.</p>"),
    })
    .await;
    let mut cfg = strict(&fixture);
    cfg.check_external = true;
    cfg.content_capture = Some(ContentCaptureConfig::default());
    cfg.resource_discovery
        .as_mut()
        .unwrap()
        .navigation
        .internal_anchors = NavigationSwitch {
        store: false,
        fetch: true,
        expand: false,
    };
    let temp = tempfile::tempdir().unwrap();
    let parent = run(cfg, &temp.path().join("parent.sqlite")).await;
    let original = parent
        .report
        .pages
        .iter()
        .find(|page| page.url == fixture.url("/shared"))
        .unwrap();
    assert!(
        original
            .content_captures
            .as_ref()
            .unwrap()
            .iter()
            .any(|capture| capture
                .result
                .as_ref()
                .is_some_and(|result| result.text.as_deref().is_some_and(|text| !text.is_empty())))
    );
    let bytes = serde_json::to_vec(&parent.report).unwrap();
    let prepared = prepare_selected_recrawl(
        SelectedRecrawlParent {
            job_id: "own-retained-parent".into(),
            checkpoint_revision: parent.checkpoint.as_ref().map(|value| value.revision),
            source_kind: RecrawlSourceKind::CapturedCheckpoint,
        },
        &bytes,
        &[fixture.url("/shared")],
        None,
    )
    .unwrap();
    changed.store(true, Ordering::SeqCst);
    let path = temp.path().join("selected.sqlite");
    let child = Crawler::new(prepared.config.clone())
        .unwrap()
        .with_store(&path)
        .unwrap()
        .with_selected_recrawl(prepared)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(child.outcome, CrawlOutcome::Completed);
    assert_eq!(fixture.count("/shared"), 2);
    assert_eq!(external.count("/landing"), 1);
    let page = child
        .report
        .pages
        .iter()
        .find(|page| page.url == fixture.url("/shared"))
        .unwrap();
    let facts = page.resource_response.as_ref().unwrap();
    assert!(!facts.retained_response);
    assert_eq!(
        facts.selected_materialization_reason.as_deref(),
        Some("selected_scope_limited_projection")
    );
    assert!(!facts.metadata_projection_complete);
    assert!(page.custom_extractions.is_empty());
    assert!(
        page.content_captures
            .as_ref()
            .unwrap()
            .iter()
            .all(|capture| capture
                .result
                .as_ref()
                .is_none_or(|result| result.text.as_deref().is_none_or(str::is_empty)))
    );
    validate_page_content_captures(page).unwrap();
    let store = CrawlStore::open_read_only(&path).unwrap();
    assert_eq!(store.parent_snapshot_bytes().unwrap().unwrap(), bytes);
    assert!(store.inspection().unwrap().checkpoint_valid);
    let transport = store
        .resource_transports(None, 100)
        .unwrap()
        .rows
        .into_iter()
        .find(|row| row.requested_url == fixture.url("/shared"))
        .unwrap();
    assert!(
        transport.retained_response,
        "Selected attempt is explicitly materialized in current report"
    );
    assert!(
        !transport
            .projection
            .resource_response
            .as_ref()
            .unwrap()
            .retained_response
    );
    let restarted = Crawler::new(child.report.config.clone())
        .unwrap()
        .with_store(&path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    assert_eq!(restarted.outcome, CrawlOutcome::Completed);
    assert_eq!(fixture.count("/shared"), 2);
    assert_eq!(
        serde_json::to_value(&restarted.report.pages).unwrap(),
        serde_json::to_value(&child.report.pages).unwrap()
    );
}

#[tokio::test]
async fn strict_v2_late_resource_store_witness_survives_global_navigation_store_bit() {
    let external = Fixture::new(|_, _| {
        Response::html("<p>Original body remains unavailable after bounded promotion.</p>")
    })
    .await;
    let landing = external.url("/landing");
    let gate = Arc::new(Semaphore::new(0));
    let held = gate.clone();
    let fixture = Fixture::new(move |_, path| match path {
        "/" => Response::html(
            "<a href='/shared'>Nav store</a><img src='/shared'><a href='/later'>Late source</a>",
        ),
        "/shared" => Response::html("").status(302).header("Location", &landing),
        "/later" => Response::html("<script src='/shared'></script>").gated(&held),
        _ => Response::html(""),
    })
    .await;
    let mut cfg = strict(&fixture);
    cfg.content_capture = Some(ContentCaptureConfig::default());
    let resource = cfg.resource_discovery.as_mut().unwrap();
    resource.resources.images.store = false;
    resource
        .resource_scope
        .external_origins
        .push(external.origin.clone());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("witness.sqlite");
    let crawler = Crawler::new(cfg).unwrap().with_store(&path).unwrap();
    let running = tokio::spawn(async move {
        crawler
            .run_controlled(None, CrawlControl::new())
            .await
            .unwrap()
    });
    fixture.started("/later").await;
    let before = CrawlStore::open_read_only(&path)
        .unwrap()
        .resource_transports(None, 100)
        .unwrap()
        .rows
        .into_iter()
        .find(|row| row.requested_url == fixture.url("/shared"))
        .unwrap();
    assert!(!before.retained_response);
    assert_eq!(external.count("/landing"), 1);
    let nav = before
        .projection
        .resource_permission
        .as_ref()
        .unwrap()
        .authorizing_grants
        .iter()
        .find(|grant| grant.reference_kind == Some(ResourceReferenceKind::InternalAnchor))
        .unwrap()
        .clone();
    assert!(nav.store);
    gate.add_permits(1);
    let final_result = running.await.unwrap();
    assert_eq!(fixture.count("/shared"), 1);
    assert_eq!(external.count("/landing"), 1);
    let promoted = final_result
        .report
        .pages
        .iter()
        .find(|page| page.url == fixture.url("/shared"))
        .unwrap();
    let permission = promoted.resource_permission.as_ref().unwrap();
    assert!(permission.authorizing_grants.contains(&nav));
    assert!(
        permission
            .authorizing_grants
            .iter()
            .any(
                |grant| grant.reference_kind == Some(ResourceReferenceKind::JavaScript)
                    && grant.store
            )
    );
    assert!(
        !promoted
            .resource_response
            .as_ref()
            .unwrap()
            .metadata_projection_complete
    );
    assert!(
        promoted
            .content_captures
            .as_ref()
            .unwrap()
            .iter()
            .all(|capture| capture
                .result
                .as_ref()
                .is_none_or(|result| result.text.is_none()))
    );
    let store = CrawlStore::open_read_only(&path).unwrap();
    let after = store
        .resource_transports(None, 100)
        .unwrap()
        .rows
        .into_iter()
        .find(|row| row.requested_url == fixture.url("/shared"))
        .unwrap();
    assert!(after.retained_response);
    assert!(
        !after
            .projection
            .resource_response
            .as_ref()
            .unwrap()
            .retained_response
    );
    assert_eq!(after.observed_generation, before.observed_generation);
    assert_eq!(after.projection.fetched_at, before.projection.fetched_at);
    assert!(store.inspection().unwrap().checkpoint_valid);
}

#[tokio::test]
async fn strict_v2_native_policy_cannot_upgrade_resource_redirect_scope() {
    let external = Fixture::new(|_, _| {
        Response::html("<p>Allowed policy origin but disallowed resource origin</p>")
    })
    .await;
    let location = external.url("/landing");
    let fixture = Fixture::new(move |_, path| match path {
        "/" => Response::html("<img src='/asset'>"),
        "/asset" => Response::html("").status(302).header("Location", &location),
        _ => Response::html(""),
    })
    .await;
    let mut cfg = strict(&fixture);
    cfg.crawl_policy = Some(CrawlPolicyConfig {
        scope: CrawlScopeConfig {
            roots: vec![
                CrawlScopeRoot {
                    origin: fixture.origin.clone(),
                    subdomains: false,
                    folders: vec!["/".into()],
                    exact_paths: vec![],
                },
                CrawlScopeRoot {
                    origin: external.origin.clone(),
                    subdomains: false,
                    folders: vec!["/".into()],
                    exact_paths: vec![],
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    });
    let temp = tempfile::tempdir().unwrap();
    let result = run(cfg, &temp.path().join("wide-policy.sqlite")).await;
    assert_eq!(fixture.count("/asset"), 1);
    assert_eq!(external.count("/landing"), 0);
    let facts = result
        .report
        .pages
        .iter()
        .find(|page| page.url == fixture.url("/asset"))
        .unwrap()
        .resource_response
        .as_ref()
        .unwrap();
    assert_eq!(facts.redirect_decisions[0].policy_check, Some(true));
    assert!(!facts.redirect_decisions[0].scope_allowed);
    assert_eq!(
        facts.redirect_decisions[0].reason,
        "redirect_role_scope_denied"
    );
}
async fn run(cfg: CrawlConfig, path: &std::path::Path) -> ControlledCrawlResult {
    Crawler::new(cfg)
        .unwrap()
        .with_store(path)
        .unwrap()
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap()
}
fn ledger(path: &std::path::Path) -> Vec<StoredResourceOccurrence> {
    let mut rows = Vec::new();
    CrawlStore::open_read_only(path)
        .unwrap()
        .visit_resource_occurrences(100, |page| {
            rows.extend(page.rows.clone());
            Ok(())
        })
        .unwrap();
    rows
}
fn decision<'a>(rows: &'a [StoredResourceOccurrence], raw: &str) -> &'a str {
    &rows
        .iter()
        .find(|row| row.occurrence.raw_value == raw)
        .unwrap_or_else(|| panic!("missing occurrence {raw}"))
        .decision
        .reason
}
struct Response {
    status: u16,
    body: String,
    headers: Vec<(String, String)>,
    gate: Option<Arc<Semaphore>>,
}
impl Response {
    fn html(body: &str) -> Self {
        Self {
            status: 200,
            body: body.into(),
            headers: vec![("Content-Type".into(), "text/html; charset=utf-8".into())],
            gate: None,
        }
    }
    fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }
    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
    fn gated(mut self, gate: &Arc<Semaphore>) -> Self {
        self.gate = Some(gate.clone());
        self
    }
}
struct Fixture {
    origin: String,
    hits: Arc<Mutex<BTreeMap<String, usize>>>,
    requests: Arc<Mutex<BTreeMap<String, Vec<String>>>>,
    changed: Arc<Notify>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Fixture {
    async fn new(handler: impl Fn(&str, &str) -> Response + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
        let requests = Arc::new(Mutex::new(BTreeMap::<String, Vec<String>>::new()));
        let recorded = requests.clone();
        let changed = Arc::new(Notify::new());
        let notify = changed.clone();
        let seen = hits.clone();
        let address = origin.clone();
        let handler = Arc::new(handler);
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let handler = handler.clone();
                let hits = seen.clone();
                let origin = address.clone();
                let requests = recorded.clone();
                let changed = notify.clone();
                tokio::spawn(async move {
                    let mut data = Vec::new();
                    let mut buffer = [0; 2048];
                    loop {
                        let Ok(n) = socket.read(&mut buffer).await else {
                            return;
                        };
                        if n == 0 {
                            return;
                        }
                        data.extend_from_slice(&buffer[..n]);
                        if data.windows(4).any(|part| part == b"\r\n\r\n") {
                            break;
                        }
                        if data.len() > 65536 {
                            return;
                        }
                    }
                    let request = String::from_utf8_lossy(&data);
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/");
                    *hits.lock().unwrap().entry(path.into()).or_default() += 1;
                    requests
                        .lock()
                        .unwrap()
                        .entry(path.into())
                        .or_default()
                        .push(request.to_string());
                    changed.notify_one();
                    let response = handler(&origin, path);
                    if let Some(gate) = response.gate {
                        gate.acquire().await.unwrap().forget();
                    }
                    let headers = response
                        .headers
                        .iter()
                        .map(|(name, value)| format!("{name}: {value}\r\n"))
                        .collect::<String>();
                    let head = format!(
                        "HTTP/1.1 {} Fixture\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
                        response.status,
                        response.body.len()
                    );
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(response.body.as_bytes()).await;
                });
            }
        });
        Self {
            origin,
            hits,
            requests,
            changed,
            task,
        }
    }
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.origin)
    }
    fn count(&self, path: &str) -> usize {
        self.hits.lock().unwrap().get(path).copied().unwrap_or(0)
    }
    fn requests(&self, path: &str) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .unwrap_or_default()
    }
    async fn started(&self, path: &str) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let notified = self.changed.notified();
                if self.count(path) > 0 {
                    return;
                }
                notified.await;
            }
        })
        .await
        .unwrap();
    }
}
