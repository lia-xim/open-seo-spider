//! AUTHORED / NOT COMPILED / NOT RUN under compute HOLD.
//! Child of the existing actual PublicCrawler/SQLite analysis tests. No new
//! mock persistence layer; materialized DTOs below are independent test oracles.
use super::*;
use crate::sitemap_ledger::verified_read::{SitemapLedgerFilter, SitemapLedgerRowKind, verify_raw};
use crate::sitemap_membership_analysis::{
    AnalysisReadInput, prepare_window_analysis, project_prepared_window,
};
use crate::sitemap_source_store::verified_read::verify_source;

fn filter(request: &SitemapMembershipAnalysisRequest) -> SitemapLedgerFilter {
    SitemapLedgerFilter {
        kind: match request.kind {
            SitemapAnalysisRowKind::All => SitemapLedgerRowKind::All,
            SitemapAnalysisRowKind::Root => SitemapLedgerRowKind::Root,
            SitemapAnalysisRowKind::Document => SitemapLedgerRowKind::Document,
            SitemapAnalysisRowKind::Entry => SitemapLedgerRowKind::Entry,
            SitemapAnalysisRowKind::Location => SitemapLedgerRowKind::Location,
        },
        source_observation_id: request.source_observation_id.clone(),
        membership_url: request.membership_url.clone(),
    }
}
fn compact(
    path: &std::path::Path,
    read: &SitemapReadSnapshot,
    request: &SitemapMembershipAnalysisRequest,
) -> Result<SitemapMembershipAnalysis> {
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let tx = connection.unchecked_transaction()?;
    let raw = verify_raw(&tx)?;
    let source = verify_source(&tx, raw.as_ref())?;
    let prepared = prepare_window_analysis(
        &tx,
        &AnalysisReadInput {
            raw: raw.as_ref(),
            source: source.as_ref(),
            generation: read.generation,
            crawl_config_sha256: read.crawl_config_sha256.as_deref(),
            coverage: &read.coverage,
        },
    )?;
    let visible = if request.row_limit == 0 {
        vec![]
    } else if let Some(raw) = &raw {
        raw.window(&filter(request), request.after_sequence, request.row_limit)?
            .rows
    } else {
        vec![]
    };
    let actual =
        project_prepared_window(&prepared, raw.as_ref(), &read.coverage, &visible, request)?;
    let oracle = crate::sitemap_membership_analysis::project(&tx, read, request)?;
    assert_eq!(
        actual, oracle,
        "Compact whole-cohort analysis equals the retained-row oracle"
    );
    Ok(actual)
}
fn saved_read(path: &std::path::Path) -> SitemapReadSnapshot {
    CrawlStore::open_read_only(path)
        .unwrap()
        .sitemap_read_snapshot()
        .unwrap()
}
fn original_report(path: &std::path::Path) -> String {
    Connection::open(path)
        .unwrap()
        .query_row("SELECT value FROM metadata WHERE key='report'", [], |r| {
            r.get(0)
        })
        .unwrap()
}
fn rehash_report(path: &std::path::Path, bytes: &str) {
    let mut connection = Connection::open(path).unwrap();
    let tx = connection.transaction().unwrap();
    let checkpoint: String = tx
        .query_row(
            "SELECT value FROM metadata WHERE key='checkpoint'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut checkpoint: serde_json::Value = serde_json::from_str(&checkpoint).unwrap();
    checkpoint["report_sha256"] = crate::sitemap_ledger::digest(bytes.as_bytes()).into();
    tx.execute("UPDATE metadata SET value=?1 WHERE key='report'", [bytes])
        .unwrap();
    tx.execute(
        "UPDATE metadata SET value=?1 WHERE key='checkpoint'",
        [serde_json::to_string(&checkpoint).unwrap()],
    )
    .unwrap();
    tx.commit().unwrap();
}

#[tokio::test]
async fn compact_actual_targets_inlinks_filters_and_independent_whole_membership_match_oracle() {
    let site = Site::start_mode(false, true).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let read = saved_read(&path);
    for kind in [
        SitemapAnalysisRowKind::All,
        SitemapAnalysisRowKind::Document,
        SitemapAnalysisRowKind::Location,
        SitemapAnalysisRowKind::Entry,
        SitemapAnalysisRowKind::Root,
    ] {
        let mut query = request();
        query.kind = kind;
        query.row_limit = 1;
        let actual = compact(&path, &read, &query).unwrap();
        assert!(actual.context.available);
        let page = actual
            .saved_pages
            .unwrap()
            .into_iter()
            .find(|v| v.observation.requested_url.ends_with("/two-docs"))
            .unwrap();
        assert_eq!(page.original_membership_occurrences, 2);
        assert_eq!(page.original_source_document_count, 2);
        assert_eq!(page.original_source_observation_count, 2);
        assert_eq!(page.original_source_document_samples.len(), 2);
    }
    let location = read
        .catalog
        .as_ref()
        .unwrap()
        .rows
        .as_ref()
        .unwrap()
        .iter()
        .find_map(|row| match row {
            SitemapLedgerRow::Location(v)
                if v.observed.role == SitemapLocationRole::PageLocation =>
            {
                Some(v)
            }
            _ => None,
        })
        .unwrap();
    let mut query = request();
    query.source_observation_id = Some(location.observation_id.clone());
    query.membership_url = location.admission.membership_url.clone();
    query.row_limit = 1;
    let actual = compact(&path, &read, &query).unwrap();
    let two = actual
        .saved_pages
        .unwrap()
        .into_iter()
        .find(|v| v.observation.requested_url.ends_with("/two-docs"))
        .unwrap();
    assert_eq!(
        two.original_source_document_count, 2,
        "Saved-page membership ignores visible source/member filters"
    );
    let requests = site.count.load(Ordering::SeqCst);
    site.finish(requests).await;
}

#[tokio::test]
async fn compact_prepared_context_is_reused_across_independent_saved_page_windows() {
    let site = Site::start(true).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let read = saved_read(&path);
    let connection = Connection::open(&path).unwrap();
    let tx = connection.unchecked_transaction().unwrap();
    let raw = verify_raw(&tx).unwrap();
    let source = verify_source(&tx, raw.as_ref()).unwrap();
    let prepared = prepare_window_analysis(
        &tx,
        &AnalysisReadInput {
            raw: raw.as_ref(),
            source: source.as_ref(),
            generation: read.generation,
            crawl_config_sha256: read.crawl_config_sha256.as_deref(),
            coverage: &read.coverage,
        },
    )
    .unwrap();
    assert!(prepared.context().available);
    let context = prepared.context().clone();
    let mut query = request();
    query.row_limit = 0;
    query.page_limit = 2;
    let mut actual = vec![];
    loop {
        let page =
            project_prepared_window(&prepared, raw.as_ref(), &read.coverage, &[], &query).unwrap();
        assert_eq!(page.context, context);
        assert_eq!(
            page,
            crate::sitemap_membership_analysis::project(&tx, &read, &query).unwrap()
        );
        actual.extend(
            page.saved_pages
                .unwrap()
                .into_iter()
                .map(|v| v.observation.report_page_ordinal),
        );
        if !page.saved_pages_have_more {
            break;
        }
        query.after_page_ordinal = page.last_saved_page_ordinal;
    }
    assert_eq!(
        actual,
        (0..context.saved_page_count.unwrap()).collect::<Vec<_>>()
    );
    assert!(
        !read
            .coverage
            .current_valid_original_location_negative_permitted
    );
    drop(prepared);
    drop(source);
    drop(raw);
    drop(tx);
    drop(connection);
    let requests = site.count.load(Ordering::SeqCst);
    site.finish(requests).await;
}

#[tokio::test]
async fn compact_actual_policy_aliases_keep_original_membership_and_observed_inlinks() {
    let site = Site::start(false).await;
    let mut config = site.config();
    config.crawl_policy = Some(crate::CrawlPolicyConfig {
        scope: crate::CrawlScopeConfig {
            roots: vec![crate::CrawlScopeRoot {
                origin: site.origin.clone(),
                folders: vec!["/".into()],
                subdomains: false,
                exact_paths: vec![],
            }],
            ..Default::default()
        },
        rewrites: vec![crate::RewriteRule {
            id: "compact-old-to-new".into(),
            pattern: "/old$".into(),
            replacement: crate::RewriteReplacement::Literal {
                text: "/new".into(),
            },
        }],
        ..Default::default()
    });
    let (_dir, path, _) = crawl(&site, config).await;
    let actual = compact(&path, &saved_read(&path), &request()).unwrap();
    let old = actual
        .location_findings
        .unwrap()
        .into_iter()
        .find(|v| {
            v.original_membership_url
                .as_ref()
                .is_some_and(|u| u.ends_with("/old"))
        })
        .unwrap();
    assert_eq!(
        old.original_state,
        SitemapTargetState::NoStoredRequestedObservation
    );
    assert!(
        old.admitted_alias_observation
            .unwrap()
            .requested_url
            .ends_with("/new")
    );
    let pages = actual.saved_pages.unwrap();
    let alias = pages
        .iter()
        .find(|v| v.observation.requested_url.ends_with("/new"))
        .unwrap();
    assert_eq!(
        (
            alias.original_membership_occurrences,
            alias.admitted_alias_occurrences
        ),
        (0, 1)
    );
    let listed = pages
        .iter()
        .find(|v| v.observation.requested_url.ends_with("/listed"))
        .unwrap();
    assert_eq!(listed.observed_inlinks.anchor_occurrences, 3);
    assert_eq!(listed.observed_inlinks.nonself_anchor_occurrences, 2);
    let requests = site.count.load(Ordering::SeqCst);
    site.finish(requests).await;
}

#[tokio::test]
async fn compact_rehashed_actual_report_source_changes_are_never_summary_only() {
    let site = Site::start(false).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let read = saved_read(&path);
    let original = original_report(&path);
    let value: serde_json::Value = serde_json::from_str(&original).unwrap();
    let mut mutations = vec![];
    for key in ["records", "commits"] {
        let mut bad = value.clone();
        bad["sitemap_source_measurements"][key]
            .as_array_mut()
            .unwrap()
            .clear();
        mutations.push(serde_json::to_string(&bad).unwrap());
    }
    let mut bad = value.clone();
    bad["sitemap_source_measurements"]["records"][0]["staged_document_sha256"] =
        "f".repeat(64).into();
    mutations.push(serde_json::to_string(&bad).unwrap());
    let mut bad = value.clone();
    bad["sitemap_source_measurements"]["summary"]["revision"] = 9999.into();
    mutations.push(serde_json::to_string(&bad).unwrap());
    for bad in mutations {
        rehash_report(&path, &bad);
        assert!(
            compact(&path, &read, &request()).is_err(),
            "Actual checkpoint SHA recomputation cannot attest SQL equality"
        );
    }
    rehash_report(&path, &original);
    compact(&path, &read, &request()).unwrap();
    let requests = site.count.load(Ordering::SeqCst);
    site.finish(requests).await;
}

#[tokio::test]
async fn compact_source_stream_rejects_duplicate_unknown_type_and_whole_envelope_faults() {
    let site = Site::start(false).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let read = saved_read(&path);
    let original = original_report(&path);
    let value: serde_json::Value = serde_json::from_str(&original).unwrap();
    let canonical = serde_json::to_string(&value).unwrap();
    let snapshot = serde_json::to_string(&value["sitemap_source_measurements"]).unwrap();
    let mut malformed = vec![
        canonical.replacen("\"records\":[", "\"records\":[],\"records\":[", 1),
        canonical.replacen(
            &snapshot,
            &format!(
                "{{\"schema_version\":1,{}",
                snapshot.strip_prefix('{').unwrap()
            ),
            1,
        ),
        format!(
            "{},\"sitemap_source_measurements\":{snapshot}}}",
            canonical.strip_suffix('}').unwrap()
        ),
        format!("{canonical} true"),
    ];
    for replacement in [
        serde_json::json!(null),
        serde_json::json!([]),
        serde_json::json!("object"),
    ] {
        let mut bad = value.clone();
        bad["sitemap_source_measurements"] = replacement;
        malformed.push(serde_json::to_string(&bad).unwrap());
    }
    let mut bad = value.clone();
    bad["sitemap_source_measurements"]["unknown"] = true.into();
    malformed.push(serde_json::to_string(&bad).unwrap());
    let mut bad = value.clone();
    bad["sitemap_source_measurements"]["records"][0]["engine"] = " ".repeat(110_000).into();
    malformed.push(serde_json::to_string(&bad).unwrap());
    let mut bad = value.clone();
    bad.as_object_mut()
        .unwrap()
        .insert("unknown".repeat(1024), true.into());
    malformed.push(serde_json::to_string(&bad).unwrap());
    for bad in malformed {
        rehash_report(&path, &bad);
        assert!(compact(&path, &read, &request()).is_err());
    }
    // One early escape forces serde_json's copied-key scratch path. Assert the
    // RAW preflight error, not merely an eventual decoded-field visitor error.
    // Both the exact 6,144+1 boundary and a larger literal suffix are below the
    // unchanged256MiB report budget and leave the actual source envelope intact.
    for key in [
        format!("{}a", "\\u0061".repeat(1024)),
        format!("\\u0061{}", "a".repeat(65_536)),
    ] {
        let bad = format!("{},\"{key}\":0}}", original.strip_suffix('}').unwrap());
        rehash_report(&path, &bad);
        let error = compact(&path, &read, &request())
            .err()
            .expect("overbudget escaped report key");
        assert!(
            format!("{error:#}").contains("Saved report outer key encoded byte budget exceeded"),
            "{error:#}"
        );
    }
    rehash_report(&path, &original);
    compact(&path, &read, &request()).unwrap();
    let requests = site.count.load(Ordering::SeqCst);
    site.finish(requests).await;
}

#[tokio::test]
async fn compact_stream_accepts_pretty_escaped_equivalent_source_and_ignores_unrelated_value() {
    let site = Site::start(false).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let read = saved_read(&path);
    let original = original_report(&path);
    let mut value: serde_json::Value = serde_json::from_str(&original).unwrap();
    value.as_object_mut().unwrap().insert(
        "unrelated".into(),
        serde_json::json!({"ignored":["u".repeat(120_000)]}),
    );
    let pretty = serde_json::to_string_pretty(&value)
        .unwrap()
        .replace("\"records\"", "\"\\u0072ecords\"")
        .replace(
            "\"sitemap_source_measurements\"",
            "\"\\u0073itemap_source_measurements\"",
        )
        .replace("\"unrelated\"", "\"\\u0075nrelated\"");
    // Exactly1,024 decoded bytes/6,144 encoded content bytes: legal unknown
    // outer key at the raw bound, with small escaped known/unknown keys above.
    let pretty = format!(
        "{},\"{}\":true}}",
        pretty.strip_suffix('}').unwrap(),
        "\\u0061".repeat(1024)
    );
    rehash_report(&path, &pretty);
    compact(&path, &read, &request()).unwrap();
    rehash_report(&path, &original);
    let requests = site.count.load(Ordering::SeqCst);
    site.finish(requests).await;
}

#[tokio::test]
async fn compact_actual_checkpoint_wrong_connection_source_and_ended_transaction_controls() {
    let site = Site::start(false).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let read = saved_read(&path);
    let connection = Connection::open(&path).unwrap();
    let other = Connection::open(&path).unwrap();
    let tx = connection.unchecked_transaction().unwrap();
    let other_tx = other.unchecked_transaction().unwrap();
    let raw = verify_raw(&tx).unwrap();
    let source = verify_source(&tx, raw.as_ref()).unwrap();
    let other_raw = verify_raw(&other_tx).unwrap();
    let other_source = verify_source(&other_tx, other_raw.as_ref()).unwrap();
    assert!(
        prepare_window_analysis(
            &tx,
            &AnalysisReadInput {
                raw: raw.as_ref(),
                source: other_source.as_ref(),
                generation: read.generation,
                crawl_config_sha256: read.crawl_config_sha256.as_deref(),
                coverage: &read.coverage
            }
        )
        .is_err()
    );
    let prepared = prepare_window_analysis(
        &tx,
        &AnalysisReadInput {
            raw: raw.as_ref(),
            source: source.as_ref(),
            generation: read.generation,
            crawl_config_sha256: read.crawl_config_sha256.as_deref(),
            coverage: &read.coverage,
        },
    )
    .unwrap();
    let context_only = SitemapMembershipAnalysisRequest::default();
    assert!(
        project_prepared_window(
            &prepared,
            other_raw.as_ref(),
            &read.coverage,
            &[],
            &context_only
        )
        .is_err()
    );
    let mut bad_coverage = read.coverage.clone();
    bad_coverage.current_valid_original_location_negative_permitted =
        !bad_coverage.current_valid_original_location_negative_permitted;
    assert!(
        project_prepared_window(&prepared, raw.as_ref(), &bad_coverage, &[], &context_only)
            .is_err()
    );
    // Deliberate contract violation only as a negative control: the entry point
    // must reject an ended transaction, not expose a cached valid context.
    tx.execute_batch("ROLLBACK").unwrap();
    assert!(
        project_prepared_window(&prepared, raw.as_ref(), &read.coverage, &[], &context_only)
            .is_err()
    );
    drop(prepared);
    drop(source);
    drop(raw);
    drop(tx);
    drop(other_source);
    drop(other_raw);
    drop(other_tx);
    let requests = site.count.load(Ordering::SeqCst);
    site.finish(requests).await;
}

#[tokio::test]
async fn compact_actual_selected_parent_keeps_historical_membership_and_mixed_age() {
    let site = Site::start(false).await;
    let (_parent_dir, _, parent) = crawl(&site, site.config()).await;
    let prepared = crate::prepare_selected_recrawl(
        crate::SelectedRecrawlParent {
            job_id: "compact-real-parent".into(),
            checkpoint_revision: parent.checkpoint.as_ref().map(|v| v.revision),
            source_kind: crate::RecrawlSourceKind::StoredJson,
        },
        &serde_json::to_vec(&parent.report).unwrap(),
        &[format!("{}/listed", site.origin)],
        None,
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("selected.sqlite");
    let crawler = Crawler::from_saved_config(prepared.config.clone())
        .unwrap()
        .with_selected_recrawl(prepared)
        .unwrap()
        .with_store(&path)
        .unwrap();
    let result = crawler
        .run_controlled(None, CrawlControl::new())
        .await
        .unwrap();
    drop(crawler);
    assert_eq!(result.outcome, CrawlOutcome::Completed);
    let read = saved_read(&path);
    let actual = compact(&path, &read, &request()).unwrap();
    assert_eq!(
        actual.context.observation_scope,
        "mixed_age_selected_saved_report"
    );
    let pages = actual.saved_pages.unwrap();
    assert_eq!(
        pages
            .iter()
            .find(|v| v.observation.requested_url.ends_with("/listed"))
            .unwrap()
            .observation
            .age_scope,
        "selected_current_generation"
    );
    let inherited = pages
        .iter()
        .find(|v| v.observation.requested_url.ends_with("/not-listed"))
        .unwrap();
    assert_eq!(
        inherited.observation.age_scope,
        "inherited_parent_observation"
    );
    assert_eq!(
        inherited.membership,
        SitemapSavedPageMembership::NotListedHistoricalDeclaredSources
    );
    let requests = site.count.load(Ordering::SeqCst);
    site.finish(requests).await;
}

#[tokio::test]
async fn compact_historical_absent_source_is_unknown_and_forbids_a_report_only_envelope() {
    let site = Site::start(false).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let original = original_report(&path);
    let original_value: serde_json::Value = serde_json::from_str(&original).unwrap();
    let mut report = original_value.clone();
    report
        .as_object_mut()
        .unwrap()
        .remove("sitemap_source_measurements");
    let connection = Connection::open(&path).unwrap();
    connection.execute_batch("DELETE FROM sitemap_source_measurement_commits; DELETE FROM sitemap_source_measurements; DELETE FROM metadata WHERE key='sitemap_source_store';").unwrap();
    let checkpoint: String = connection
        .query_row(
            "SELECT value FROM metadata WHERE key='checkpoint'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut checkpoint: serde_json::Value = serde_json::from_str(&checkpoint).unwrap();
    checkpoint
        .as_object_mut()
        .unwrap()
        .remove("sitemap_source_measurements");
    connection
        .execute(
            "UPDATE metadata SET value=?1 WHERE key='checkpoint'",
            [serde_json::to_string(&checkpoint).unwrap()],
        )
        .unwrap();
    let absent = serde_json::to_string(&report).unwrap();
    rehash_report(&path, &absent);
    let read = saved_read(&path);
    assert!(read.sitemap_source_measurements.is_none());
    assert!(compact(&path, &read, &request()).unwrap().context.available);
    report["sitemap_source_measurements"] = serde_json::Value::Null;
    rehash_report(&path, &serde_json::to_string(&report).unwrap());
    compact(&path, &read, &request()).unwrap();
    // Actual original captures in a report cannot manufacture a missing SQL
    // sidecar basis, even when the report/checkpoint hash is recomputed.
    rehash_report(&path, &original);
    assert!(compact(&path, &read, &request()).is_err());
    let requests = site.count.load(Ordering::SeqCst);
    site.finish(requests).await;
}

#[tokio::test]
async fn compact_actual_checkpoint_source_mismatch_stays_unavailable_before_any_rows() {
    let site = Site::start(false).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let read = saved_read(&path);
    let connection = Connection::open(&path).unwrap();
    let checkpoint: String = connection
        .query_row(
            "SELECT value FROM metadata WHERE key='checkpoint'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let mut checkpoint: serde_json::Value = serde_json::from_str(&checkpoint).unwrap();
    checkpoint["sitemap_source_measurements"]["revision"] = 9999.into();
    connection
        .execute(
            "UPDATE metadata SET value=?1 WHERE key='checkpoint'",
            [serde_json::to_string(&checkpoint).unwrap()],
        )
        .unwrap();
    let actual = compact(&path, &read, &request()).unwrap();
    assert!(!actual.context.available);
    assert_eq!(
        actual.context.reason,
        "saved_source_measurement_context_mismatch"
    );
    assert!(actual.saved_pages.is_none() && actual.location_findings.is_none());
    let requests = site.count.load(Ordering::SeqCst);
    site.finish(requests).await;
}
