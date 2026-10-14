//! AUTHORED / NOT_COMPILED / NOT_RUN. Child of actual LastmodStored fixtures.
use super::*;
use open_seo_spider_core::{CrawlStore, SitemapReadWindowError};

fn rich_read(stored: &LastmodStored, query: &ReadQuery) -> anyhow::Result<SitemapReadWindow> {
    let expected = query.expected_context(stored.job).unwrap();
    CrawlStore::open_read_only(&stored.db)?.sitemap_read_window_with_membership_analysis(
        &query.analysis_request().unwrap(),
        expected.as_ref(),
    )
}
fn rich_page(stored: &LastmodStored, query: ReadQuery) -> Inventory {
    let read = rich_read(stored, &query).unwrap();
    inventory_window(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(read),
        query,
    )
    .unwrap()
}
fn legacy_state(read: &SitemapReadSnapshot) -> String {
    let catalog = read.catalog.as_ref().unwrap();
    // Exact pre-migration App tuple from the frozen old implementation. This
    // is a legacy token fixture, not a substitute Core header or validator.
    digest(
        encode(&(
            &catalog.summary,
            &catalog.inherited,
            &read.coverage_inventory,
            &read.coverage,
            &read.sitemap_source_measurement_context,
            SITEMAP_LASTMOD_ENGINE,
            open_seo_spider_core::SITEMAP_MEMBERSHIP_ANALYSIS_ENGINE,
            read.membership_analysis
                .as_ref()
                .map(|value| &value.context),
        ))
        .unwrap(),
    )
}

#[tokio::test]
async fn actual_compact_lastmod_rows_and_context_match_full_oracle_and_csv_bytes() {
    let website = LastmodWebsite::start("<urlset><url><loc>{origin}/a</loc><lastmod>2000-01-01</lastmod></url><url><loc>{origin}/b</loc><lastmod>bad</lastmod></url><url><loc>{origin}/c</loc></url></urlset>".into()).await;
    let stored = LastmodStored::crawl(&website, true, false).await;
    let requests = website.count();
    let before = stored.file_hashes();
    let query = ReadQuery {
        kind: RowKind::Entry,
        limit: Some(2),
        page_limit: Some(1),
        ..Default::default()
    };
    let oracle = actual_analysis_page(&stored, query.clone());
    let actual = rich_page(&stored, query.clone());
    assert_eq!(
        serde_json::to_value(&actual).unwrap(),
        serde_json::to_value(&oracle).unwrap()
    );
    assert_eq!(actual.lastmod_assessments.as_ref().unwrap().len(), 2);
    let captured = actual_analysis_read(&stored, &ReadQuery::default());
    let export = CsvQuery {
        kind: query.kind,
        source_observation_id: None,
        membership_url: None,
        snapshot: actual.snapshot_token.unwrap(),
    };
    let (actual_bytes, snapshot) = csv(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(captured),
        export,
    )
    .unwrap();
    assert_eq!(Some(snapshot), oracle.snapshot);
    let (oracle_bytes, _) = csv(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(actual_analysis_read(&stored, &ReadQuery::default())),
        CsvQuery {
            kind: query.kind,
            source_observation_id: None,
            membership_url: None,
            snapshot: oracle.snapshot_token.unwrap(),
        },
    )
    .unwrap();
    assert_eq!(actual_bytes, oracle_bytes);
    assert_eq!(stored.file_hashes(), before);
    website.finish(requests).await;
}

#[tokio::test]
async fn actual_old_profile_raw_page_and_export_tokens_parse_then_become_stale() {
    let website = LastmodWebsite::start(
        "<urlset><url><loc>{origin}/a</loc></url><url><loc>{origin}/b</loc></url></urlset>".into(),
    )
    .await;
    let stored = LastmodStored::crawl(&website, true, false).await;
    let requests = website.count();
    let before = stored.file_hashes();
    let query = ReadQuery {
        kind: RowKind::Location,
        limit: Some(1),
        page_limit: Some(1),
        ..Default::default()
    };
    let fresh = rich_page(&stored, query.clone());
    let full = actual_analysis_read(&stored, &query);
    let mut old = fresh.snapshot.clone().unwrap();
    old.catalog_state_sha256 = legacy_state(&full);
    assert_ne!(old, fresh.snapshot.clone().unwrap());
    let filter = query.filter();
    let raw = token(
        &old,
        &filter,
        Some(fresh.rows.as_ref().unwrap()[0].sequence()),
    )
    .unwrap();
    let page = page_token(&old, &filter, 0).unwrap();
    let export = token(&old, &filter, None).unwrap();
    for value in [&raw, &page, &export] {
        assert_eq!(decode(value).unwrap().payload.snapshot, old);
    }
    for (cursor, page_cursor) in [(Some(raw), None), (None, Some(page))] {
        let mut continuation = query.clone();
        continuation.cursor = cursor;
        continuation.page_cursor = page_cursor;
        let error = rich_read(&stored, &continuation).unwrap_err();
        assert_eq!(
            error.downcast_ref::<SitemapReadWindowError>(),
            Some(&SitemapReadWindowError::StaleContext)
        );
        let actual = rich_read(&stored, &query).unwrap();
        let error = inventory_window(
            stored.job,
            stored.generation,
            &stored.config_sha,
            Some(actual),
            continuation,
        )
        .unwrap_err();
        assert_eq!(error.code, "stale_sitemap_cursor");
        assert_eq!(error.status(), 409);
    }
    let error = csv(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(full),
        CsvQuery {
            kind: query.kind,
            source_observation_id: None,
            membership_url: None,
            snapshot: export,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "stale_sitemap_cursor");
    assert_eq!(stored.file_hashes(), before);
    website.finish(requests).await;
}

#[tokio::test]
async fn actual_expected_state_precedes_invalid_row_and_token_job_filter_context_prechecks() {
    let website =
        LastmodWebsite::start("<urlset><url><loc>{origin}/a</loc></url></urlset>".into()).await;
    let stored = LastmodStored::crawl(&website, true, false).await;
    let requests = website.count();
    let before = stored.file_hashes();
    let query = ReadQuery {
        limit: Some(1),
        page_limit: Some(1),
        ..Default::default()
    };
    let current = rich_page(&stored, query.clone());
    let snapshot = current.snapshot.unwrap();
    let mut invalid = query.clone();
    invalid.cursor = Some(token(&snapshot, &query.filter(), Some(u64::MAX)).unwrap());
    let error = rich_read(&stored, &invalid).unwrap_err();
    assert_eq!(
        error.downcast_ref::<SitemapReadWindowError>(),
        Some(&SitemapReadWindowError::InvalidContinuation)
    );
    let mut stale = snapshot.clone();
    stale.catalog_state_sha256 = digest(b"old actual context");
    invalid.cursor = Some(token(&stale, &query.filter(), Some(u64::MAX)).unwrap());
    let error = rich_read(&stored, &invalid).unwrap_err();
    assert_eq!(
        error.downcast_ref::<SitemapReadWindowError>(),
        Some(&SitemapReadWindowError::StaleContext)
    );
    let mut both = query.clone();
    both.cursor = Some(token(&snapshot, &query.filter(), Some(1)).unwrap());
    both.page_cursor = Some(page_token(&stale, &query.filter(), 0).unwrap());
    assert_eq!(
        both.expected_context(stored.job).unwrap_err().code,
        "stale_sitemap_cursor"
    );
    let mut wrong_job = query.clone();
    wrong_job.cursor = both.cursor.clone();
    assert_eq!(
        wrong_job.expected_context(Uuid::new_v4()).unwrap_err().code,
        "stale_sitemap_cursor"
    );
    let mut wrong_filter = query;
    wrong_filter.kind = RowKind::Location;
    wrong_filter.cursor = both.cursor;
    assert_eq!(
        wrong_filter.expected_context(stored.job).unwrap_err().code,
        "stale_sitemap_cursor"
    );
    let mut invalid_saved = ReadQuery {
        limit: Some(1),
        page_limit: Some(1),
        ..Default::default()
    };
    invalid_saved.page_cursor = Some(page_token(&snapshot, &invalid_saved.filter(), 999).unwrap());
    let captured = rich_read(&stored, &invalid_saved).unwrap();
    let error = inventory_window(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(captured),
        invalid_saved,
    )
    .unwrap_err();
    assert_eq!(error.code, "invalid_sitemap_cursor");
    assert_eq!(error.status(), 400);
    assert_eq!(stored.file_hashes(), before);
    website.finish(requests).await;
}

#[tokio::test]
async fn actual_rich_unknown_empty_and_incomplete_match_full_optional_evidence() {
    let website =
        LastmodWebsite::start("<urlset><url><lastmod>2000-01-01</lastmod><lastmod>".into()).await;
    for (sitemap, legacy) in [(false, false), (true, true), (true, false)] {
        let stored = LastmodStored::crawl(&website, sitemap, legacy).await;
        let requests = website.count();
        let before = stored.file_hashes();
        let query = ReadQuery::default();
        let old = actual_analysis_page(&stored, query.clone());
        let actual = rich_page(&stored, query);
        assert_eq!(
            serde_json::to_value(&actual).unwrap(),
            serde_json::to_value(&old).unwrap()
        );
        if legacy {
            assert!(!actual.catalog_available);
            assert!(actual.rows.is_none());
        } else {
            assert!(actual.catalog_available);
            assert!(actual.rows.is_some());
        }
        assert_eq!(stored.file_hashes(), before);
        assert_eq!(website.count(), requests);
    }
    let requests = website.count();
    let missing =
        inventory_window(Uuid::nil(), 1, &"a".repeat(64), None, ReadQuery::default()).unwrap();
    assert!(!missing.catalog_available);
    assert!(missing.rows.is_none());
    assert!(missing.lastmod_assessments.is_none());
    assert_eq!(missing.availability_reason, "sqlite_history_unavailable");
    website.finish(requests).await;
}

#[tokio::test]
async fn actual_rich_visible_lastmod_copy_hits_real_json_cap_and_smaller_page_recovers() {
    let field = "a".repeat(7000);
    let xml = format!(
        "<urlset>{}</urlset>",
        format!("<url><lastmod>{field}</lastmod></url>").repeat(200)
    );
    let website = LastmodWebsite::start(xml).await;
    let stored = LastmodStored::crawl(&website, true, false).await;
    let requests = website.count();
    let before = stored.file_hashes();
    let query = ReadQuery {
        kind: RowKind::Entry,
        limit: Some(200),
        page_limit: Some(0),
        ..Default::default()
    };
    let read = rich_read(&stored, &query).unwrap();
    assert_eq!(read.rows.as_ref().unwrap().len(), 200);
    let error = inventory_window(
        stored.job,
        stored.generation,
        &stored.config_sha,
        Some(read),
        query,
    )
    .unwrap_err();
    assert_eq!(error.code, "sitemap_response_byte_budget");
    assert_eq!(error.status(), 413);
    let recovered = rich_page(
        &stored,
        ReadQuery {
            kind: RowKind::Entry,
            limit: Some(100),
            page_limit: Some(0),
            ..Default::default()
        },
    );
    assert_eq!(recovered.rows.as_ref().unwrap().len(), 100);
    assert_eq!(recovered.lastmod_assessments.as_ref().unwrap().len(), 100);
    assert!(recovered.has_more);
    assert!(encode(&recovered).unwrap().len() <= MAX_JSON_BYTES);
    assert_eq!(stored.file_hashes(), before);
    website.finish(requests).await;
}
