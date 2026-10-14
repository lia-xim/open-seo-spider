//! AUTHORED / NOT_COMPILED / NOT_RUN under HOLD. Child of actual handler tests.
use super::*;

#[tokio::test]
async fn actual_rich_inventory_handler_matches_full_oracle_and_fresh_token_exports_csv() {
    let fixture = Fixture::new().await;
    let (dir, state, id) = setup(&fixture, Mode::Native).await;
    let before = files(&dir.0);
    let requests = fixture.count.load(Ordering::SeqCst);
    for kind in [
        RowKind::All,
        RowKind::Root,
        RowKind::Document,
        RowKind::Entry,
        RowKind::Location,
    ] {
        let query = ReadQuery {
            kind,
            limit: Some(2),
            page_limit: Some(1),
            ..Default::default()
        };
        let job = get_job(&state, id)
            .await
            .ok()
            .expect("actual fixture job exists");
        let guard = job.read().await;
        let generation = guard.generation;
        let config_sha = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&guard.config).unwrap())
        );
        drop(guard);
        let saved = CrawlStore::open_read_only(dir.0.join(format!("{id}.sqlite")))
            .unwrap()
            .sitemap_read_snapshot_with_membership_analysis(&query.analysis_request().unwrap())
            .unwrap();
        let oracle =
            membership::inventory(id, generation, &config_sha, Some(saved), query.clone()).unwrap();
        let actual = page(&state, id, query).await;
        assert_eq!(
            serde_json::to_value(&actual).unwrap(),
            serde_json::to_value(&oracle).unwrap()
        );
    }
    let query = ReadQuery {
        kind: RowKind::Location,
        ..Default::default()
    };
    let actual = page(&state, id, query.clone()).await;
    let result = export_csv(
        State(state.clone()),
        Path(id),
        Ok(Query(CsvQuery {
            kind: query.kind,
            source_observation_id: None,
            membership_url: None,
            snapshot: actual.snapshot_token.clone().unwrap(),
        })),
    )
    .await
    .unwrap();
    let bytes = axum::body::to_bytes(result.into_body(), membership::MAX_CSV_BYTES)
        .await
        .unwrap();
    let cells = csv_cells(&bytes);
    let full = CrawlStore::open_read_only(dir.0.join(format!("{id}.sqlite")))
        .unwrap()
        .sitemap_catalog()
        .unwrap()
        .unwrap();
    let locations: Vec<_> = full
        .rows
        .unwrap()
        .into_iter()
        .filter(|row| matches!(row, open_seo_spider_core::SitemapLedgerRow::Location(_)))
        .collect();
    assert_eq!(cells.len(), locations.len() + 1);
    for (csv, row) in cells[1..].iter().zip(locations) {
        assert_eq!(csv[1], row.sequence().to_string());
        assert_eq!(csv[9], serde_json::to_string(&row).unwrap());
    }
    assert_same_files(&dir.0, &before);
    fixture.finish(requests).await;
}

#[tokio::test]
async fn actual_rich_handler_raw_and_saved_page_cursors_remain_independent_and_role_checked() {
    let fixture = Fixture::new().await;
    let (dir, state, id) = setup(&fixture, Mode::Native).await;
    let requests = fixture.count.load(Ordering::SeqCst);
    let before = files(&dir.0);
    let query = ReadQuery {
        kind: RowKind::Location,
        limit: Some(1),
        page_limit: Some(1),
        ..Default::default()
    };
    let first = page(&state, id, query.clone()).await;
    let raw_sequence = first.rows.as_ref().unwrap()[0].sequence();
    let first_saved = first
        .membership_analysis
        .as_ref()
        .unwrap()
        .saved_pages
        .as_ref()
        .unwrap()[0]
        .observation
        .requested_url
        .clone();
    let mut next = query.clone();
    next.cursor = first.next_cursor.clone();
    let second_raw = page(&state, id, next).await;
    assert_ne!(
        second_raw.rows.as_ref().unwrap()[0].sequence(),
        raw_sequence
    );
    assert_eq!(
        second_raw
            .membership_analysis
            .as_ref()
            .unwrap()
            .saved_pages
            .as_ref()
            .unwrap()[0]
            .observation
            .requested_url,
        first_saved
    );
    let mut saved = vec![];
    let mut current = first;
    loop {
        assert_eq!(current.rows.as_ref().unwrap()[0].sequence(), raw_sequence);
        saved.extend(
            current
                .membership_analysis
                .as_ref()
                .unwrap()
                .saved_pages
                .as_ref()
                .unwrap()
                .iter()
                .map(|page| page.observation.requested_url.clone()),
        );
        let Some(cursor) = current.next_page_cursor.clone() else {
            break;
        };
        let mut next = query.clone();
        next.page_cursor = Some(cursor);
        current = page(&state, id, next).await;
    }
    assert!(saved.len() > 1);
    assert_eq!(
        saved
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        saved.len()
    );
    let first = page(&state, id, query.clone()).await;
    for (cursor, page_cursor, expected) in [
        (
            first.next_page_cursor.clone(),
            None,
            StatusCode::BAD_REQUEST,
        ),
        (None, first.next_cursor.clone(), StatusCode::BAD_REQUEST),
        (first.snapshot_token.clone(), None, StatusCode::BAD_REQUEST),
    ] {
        let mut wrong = query.clone();
        wrong.cursor = cursor;
        wrong.page_cursor = page_cursor;
        let (status, Json(error)) = inventory(State(state.clone()), Path(id), Ok(Query(wrong)))
            .await
            .err()
            .unwrap();
        assert_eq!(status, expected);
        assert_eq!(error.code, "invalid_sitemap_cursor");
    }
    let mut missing_job_query = query.clone();
    missing_job_query.cursor = first.next_cursor.clone();
    let (status, Json(error)) = inventory(
        State(state.clone()),
        Path(Uuid::new_v4()),
        Ok(Query(missing_job_query)),
    )
    .await
    .err()
    .unwrap();
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error.code, "job_not_found");
    let mut changed_filter = query;
    changed_filter.kind = RowKind::All;
    changed_filter.cursor = first.next_cursor;
    let (status, Json(error)) =
        inventory(State(state.clone()), Path(id), Ok(Query(changed_filter)))
            .await
            .err()
            .unwrap();
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error.code, "stale_sitemap_cursor");
    assert_same_files(&dir.0, &before);
    fixture.finish(requests).await;
}

#[test]
fn rich_core_typed_errors_preserve_existing_http_codes_and_corruption_remains_failure() {
    use open_seo_spider_core::SitemapReadWindowError as E;
    for (error, status, code) in [
        (
            E::StaleContext,
            StatusCode::CONFLICT,
            "stale_sitemap_cursor",
        ),
        (
            E::InvalidContinuation,
            StatusCode::BAD_REQUEST,
            "invalid_sitemap_cursor",
        ),
        (
            E::ResponseByteBudget,
            StatusCode::PAYLOAD_TOO_LARGE,
            "sitemap_response_byte_budget",
        ),
    ] {
        let error = anyhow::Error::new(error).context("actual Core error wrapper");
        let (actual_status, Json(body)) = read_failure(error);
        assert_eq!(actual_status, status);
        assert_eq!(body.code, code);
        assert_eq!(body.website_requests, 0);
    }
    let (status, Json(body)) = read_failure(anyhow::anyhow!("corrupt canonical source row"));
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body.code, "sitemap_read_failure");
}
