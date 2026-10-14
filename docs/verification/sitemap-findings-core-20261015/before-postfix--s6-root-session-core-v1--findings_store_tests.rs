//! AUTHORED / NOT_COMPILED / NOT_RUN. Actual PublicCrawler -> SQLite -> public
//! findings APIs. Existing site/crawl helpers own all HTTP and server shutdown.
use super::*;
use crate::{
    SitemapFindingAssessmentSelection as VerdictSelection, SitemapFindingEvidence,
    SitemapFindingFilter as Filter, SitemapFindingPosition, SitemapFindingSelection,
    SitemapFindingVerdict, SitemapFindingsAvailability, SitemapFindingsError,
    SitemapFindingsRequest, SitemapFindingsStop,
};

fn query(filter: Filter) -> SitemapFindingsRequest {
    SitemapFindingsRequest {
        selection: SitemapFindingSelection {
            filter,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn row_url(row: &crate::SitemapFindingRow) -> &str {
    match &row.evidence {
        SitemapFindingEvidence::SavedPage { page } => &page.finding.observation.requested_url,
        SitemapFindingEvidence::SourceAttempt { source } => source.original_task_url(),
    }
}

#[tokio::test]
async fn findings_public_crawler_exact_original_seven_filters_use_actual_domains_and_qualified_results()
 {
    let site = Site::start_mode(false, true).await;
    let (_dir, path, _result) = crawl(&site, site.config()).await;
    let requests = site.count.load(Ordering::SeqCst);
    let store = CrawlStore::open_read_only(&path).unwrap();
    let multiple = store
        .sitemap_findings_page(&query(Filter::UrlsInMultipleSitemaps), &Default::default())
        .unwrap();
    let matches = multiple.rows.unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(row_url(&matches[0]), format!("{}/two-docs", site.origin));
    let single = format!("{}/same-doc", site.origin);
    assert!(matches.iter().all(|row| row_url(row) != single));
    for filter in [
        Filter::UrlsInSitemap,
        Filter::UrlsNotInSitemap,
        Filter::OrphanUrls,
        Filter::NonIndexableUrlsInSitemap,
        Filter::UrlsInMultipleSitemaps,
        Filter::XmlSitemapWithOver50kUrls,
        Filter::XmlSitemapOver50mb,
    ] {
        let page = store
            .sitemap_findings_page(&query(filter), &Default::default())
            .unwrap();
        assert_ne!(
            page.context.availability,
            SitemapFindingsAvailability::Unavailable
        );
        assert!(page.scan.exhausted);
        for row in page.rows.unwrap() {
            assert_eq!(row.position.domain(), filter.domain());
            assert_eq!(row.assessment.verdict, SitemapFindingVerdict::Match);
        }
    }
    site.finish(requests).await;
}

#[tokio::test]
async fn findings_public_native_obstructions_orphans_and_alias_control_follow_original_identity() {
    let site = Site::start(false).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let requests = site.count.load(Ordering::SeqCst);
    let store = CrawlStore::open_read_only(&path).unwrap();
    let page = store
        .sitemap_findings_page(
            &query(Filter::NonIndexableUrlsInSitemap),
            &Default::default(),
        )
        .unwrap();
    let actual: BTreeSet<_> = page
        .rows
        .unwrap()
        .iter()
        .map(|row| row_url(row).to_owned())
        .collect();
    for suffix in ["/redirect", "/missing", "/noindex", "/blocked"] {
        assert!(
            actual.contains(&format!("{}{suffix}", site.origin)),
            "{suffix}"
        );
    }
    assert!(!actual.contains(&format!("{}/listed", site.origin)));
    assert!(
        !actual.contains(&format!("{}/final", site.origin)),
        "redirect alias never replaces original"
    );
    let orphans = store
        .sitemap_findings_page(&query(Filter::OrphanUrls), &Default::default())
        .unwrap();
    let actual: BTreeSet<_> = orphans
        .rows
        .unwrap()
        .iter()
        .map(|row| row_url(row).to_owned())
        .collect();
    assert!(
        actual.contains(&format!("{}/sitemap-only", site.origin)),
        "image occurrence is not anchor"
    );
    assert!(
        !actual.contains(&format!("{}/listed", site.origin)),
        "observed nonself nofollow anchor counts"
    );
    site.finish(requests).await;
}

#[tokio::test]
async fn findings_actual_saved_cohort_beyond_four_hundred_sparse_windows_and_full_visitor_agree() {
    let site = Site::start(false).await;
    let mut config = site.config();
    config.seed_urls = (0..451)
        .map(|n| format!("{}/page-{n:04}", site.origin))
        .collect();
    config.crawl_links = false;
    let (_dir, path, result) = crawl(&site, config).await;
    let requests = site.count.load(Ordering::SeqCst);
    let wanted = format!("{}/page-0450", site.origin);
    let ordinal = result
        .report
        .pages
        .iter()
        .position(|page| page.url == wanted)
        .unwrap();
    assert!(ordinal > 400);
    let store = CrawlStore::open_read_only(&path).unwrap();
    let mut original = query(Filter::UrlsNotInSitemap);
    original.selection.requested_url = Some(wanted.clone());
    let full = store
        .with_verified_sitemap_findings(&original.selection, None, &Default::default(), |session| {
            let mut rows = Vec::new();
            session.visit_all(|row| {
                rows.push(row.clone());
                Ok(())
            })?;
            Ok(rows)
        })
        .unwrap();
    assert_eq!(full.len(), 1);
    assert_eq!(
        full[0].position,
        SitemapFindingPosition::SavedPage {
            ordinal: ordinal as u64
        }
    );
    for limit in [1, 10, 200] {
        let mut current = SitemapFindingsRequest {
            limit,
            candidate_limit: 100,
            ..original.clone()
        };
        let mut observed = Vec::new();
        let mut empty = 0;
        loop {
            let page = store
                .sitemap_findings_page(&current, &Default::default())
                .unwrap();
            let rows = page.rows.as_ref().unwrap();
            if rows.is_empty() && !page.scan.exhausted {
                empty += 1;
                assert_eq!(page.scan.stop, SitemapFindingsStop::CandidateBudget);
                assert_eq!(page.scan.more_matches, None);
            }
            observed.extend(rows.iter().cloned());
            let Some(continuation) = page.continuation else {
                assert!(page.scan.exhausted);
                break;
            };
            current.after = Some(continuation.after);
            current.expected_context_sha256 = Some(continuation.context_sha256);
            assert!(empty < 10);
        }
        assert!(empty >= 4);
        assert_eq!(observed, full);
    }
    assert_eq!(row_url(&full[0]), wanted);
    site.finish(requests).await;
}

#[tokio::test]
async fn findings_actual_report_integrity_and_changed_query_fail_before_output() {
    let site = Site::start(false).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let requests = site.count.load(Ordering::SeqCst);
    let store = CrawlStore::open_read_only(&path).unwrap();
    let original = query(Filter::UrlsInSitemap);
    let page = store
        .sitemap_findings_page(&original, &Default::default())
        .unwrap();
    let mut changed = original.clone();
    changed.selection.assessment = VerdictSelection::Unknown;
    changed.after = Some(SitemapFindingPosition::SavedPage { ordinal: 999_999 });
    changed.expected_context_sha256 = Some(page.context.context_sha256);
    assert_eq!(
        store
            .sitemap_findings_page(&changed, &Default::default())
            .unwrap_err()
            .downcast_ref::<SitemapFindingsError>(),
        Some(&SitemapFindingsError::StaleContext)
    );
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE metadata SET value=value||' ' WHERE key='report'",
            [],
        )
        .unwrap();
    let entered = std::sync::atomic::AtomicBool::new(false);
    assert!(
        store
            .with_verified_sitemap_findings(&original.selection, None, &Default::default(), |_| {
                entered.store(true, Ordering::SeqCst);
                Ok(())
            })
            .is_err()
    );
    assert!(!entered.load(Ordering::SeqCst));
    site.finish(requests).await;
}

#[tokio::test]
async fn findings_actual_live_resume_keeps_source_evidence_available_without_saved_page_join() {
    let site = Site::start(false).await;
    let (_dir, path, _) = crawl(&site, site.config()).await;
    let requests = site.count.load(Ordering::SeqCst);
    let store = CrawlStore::open(&path).unwrap();
    store.begin_run("actual findings live resume").unwrap();
    let pages = store
        .sitemap_findings_page(&query(Filter::UrlsInSitemap), &Default::default())
        .unwrap();
    assert_eq!(
        pages.context.availability,
        SitemapFindingsAvailability::Unavailable
    );
    assert!(pages.rows.is_none());
    let mut source_query = query(Filter::XmlSitemapOver50mb);
    source_query.selection.assessment = VerdictSelection::All;
    let sources = store
        .sitemap_findings_page(&source_query, &Default::default())
        .unwrap();
    assert_ne!(
        sources.context.availability,
        SitemapFindingsAvailability::Unavailable
    );
    assert!(sources.rows.as_ref().is_some_and(|rows| !rows.is_empty()));
    site.finish(requests).await;
}
