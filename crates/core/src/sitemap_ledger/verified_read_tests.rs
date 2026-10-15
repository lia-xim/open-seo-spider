//! Authored SQLite regressions only; NOT_RUN under the compute hold.
use super::*;

const TASK: &str = "https://example.org/sitemap.xml";
fn empty(connection: &Connection) {
    connection.execute_batch("PRAGMA foreign_keys=ON;
      CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);
      CREATE TABLE discovery_frontier(kind TEXT,url TEXT,done INTEGER DEFAULT 0);
      CREATE TABLE policy_occurrences(sequence INTEGER PRIMARY KEY,occurrence_id TEXT UNIQUE,generation INTEGER,data TEXT);
      CREATE TABLE recrawl_base(id INTEGER PRIMARY KEY,snapshot BLOB);").unwrap();
    connection.execute_batch(SCHEMA).unwrap();
}
fn source_rows(xml: &str) -> Vec<SitemapLedgerRow> {
    source_rows_for(xml, TASK)
}
fn source_rows_for(xml: &str, task: &str) -> Vec<SitemapLedgerRow> {
    let limits = crate::SitemapParserLimits::default();
    let decoded = crate::decode_sitemap_entity(xml.as_bytes(), &limits).unwrap();
    let parsed = crate::observe_sitemap_xml(
        &decoded.xml,
        crate::SitemapParserContext {
            requested_document_url: Some(task.into()),
            final_document_url: Some(task.into()),
            originating_occurrence_id: None,
        },
        &limits,
    )
    .unwrap();
    let revision = digest(xml);
    let document = SitemapSourceDocument {
        sequence: 0,
        observation_id: source_observation_id(SitemapSourceKind::SitemapXml, task, 1, &revision)
            .unwrap(),
        kind: SitemapSourceKind::SitemapXml,
        task_url: task.into(),
        generation: 1,
        source_revision: revision,
        transport: Some(SitemapTransportFacts {
            request_started: true,
            requested_url: Some(task.into()),
            final_url: Some(task.into()),
            initial_status: Some(200),
            final_status: Some(200),
            blocked_by_robots: false,
            body_complete: true,
            post_http_decoding_bytes: xml.len() as u64,
            post_http_decoding_sha256: Some(digest(xml)),
            retained_post_http_prefix_sha256: Some(digest(xml)),
            failure_code: None,
        }),
        entity_decode: Some(SitemapDecodeFacts::from(&decoded)),
        parser: None,
        declaration_scan_complete: false,
        declaration_utf8_lossy: None,
        declaration_count: None,
        retained_rows_complete: true,
        failure_code: None,
    };
    let mut batch = SitemapDiscoveryBatch::from_parser(document, parsed).unwrap();
    let mut seen = BTreeSet::new();
    for loc in &mut batch.locations {
        if loc.admission.decision == SitemapAdmissionDecision::Unbound {
            let url = loc.observed.normalized_fetch_url.clone().unwrap();
            loc.admission.decision = if seen.insert(url.clone()) {
                SitemapAdmissionDecision::Scheduled
            } else {
                SitemapAdmissionDecision::Coalesced
            };
            loc.admission.admitted_fetch_url = Some(url);
            if loc.observed.role == crate::SitemapLocationRole::ChildSitemap {
                loc.referenced_sitemap_task = loc.admission.admitted_fetch_url.clone();
            }
        }
    }
    let mut rows = vec![SitemapLedgerRow::Document(Box::new(batch.document))];
    rows.extend(batch.entries.into_iter().map(SitemapLedgerRow::Entry));
    rows.extend(batch.locations.into_iter().map(SitemapLedgerRow::Location));
    rows
}
fn xml() -> &'static str {
    "<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>https://example.org/p?a=1&amp;b=2</loc></url><url><loc>https://example.org/q</loc></url><url/></urlset>"
}
fn populated(connection: &mut Connection) {
    empty(connection);
    let tx = connection.transaction().unwrap();
    initialize(&tx, &"a".repeat(64), &Default::default(), false).unwrap();
    append(&tx, source_rows(xml())).unwrap();
    tx.commit().unwrap();
}

#[test]
fn imported_nocase_task_column_preserves_distinct_case_sensitive_source_urls() {
    let mut connection = Connection::open_in_memory().unwrap();
    empty(&connection);
    {
        let tx = connection.transaction().unwrap();
        initialize(&tx, &"a".repeat(64), &Default::default(), false).unwrap();
        for task in ["https://example.org/A.xml", "https://example.org/a.xml"] {
            append(&tx, source_rows_for(xml(), task)).unwrap();
        }
        tx.commit().unwrap();
    }
    let original = load_catalog(&connection).unwrap().unwrap();
    assert_eq!(original.summary.document_rows, 2);
    // Explicit imported-schema compatibility fixture. The old reader validates
    // payload semantics and does not require the producer's original DDL.
    connection
        .execute_batch(
            "PRAGMA foreign_keys=OFF;
        CREATE TABLE raw_document_backup AS SELECT * FROM sitemap_documents;
        DROP TABLE sitemap_documents;
        CREATE TABLE sitemap_documents(
          sequence INTEGER, row_id TEXT, source_kind TEXT,
          task_url TEXT COLLATE NOCASE, generation INTEGER,
          source_revision TEXT, data TEXT, data_sha256 TEXT);
        INSERT INTO sitemap_documents SELECT * FROM raw_document_backup;
        DROP TABLE raw_document_backup;
        PRAGMA foreign_keys=ON;",
        )
        .unwrap();
    let tx = connection.transaction().unwrap();
    assert_eq!(load_catalog(&tx).unwrap().unwrap(), original);
    let raw = verify_raw(&tx).unwrap().unwrap();
    assert_eq!(raw.summary(), &original.summary);
    let page = raw.window(&Default::default(), None, 1000).unwrap();
    assert_eq!(page.rows, original.rows.unwrap());
}
fn memory() -> Connection {
    let mut connection = Connection::open_in_memory().unwrap();
    populated(&mut connection);
    connection
}
/// Explicit corruption fixture: retain correct canonical payloads, SQL indexes,
/// rolling hash and envelope size while bypassing the old semantic validator.
fn install_unchecked(connection: &Connection, mut catalog: SitemapCatalog) {
    connection.execute_batch("DELETE FROM sitemap_locations;DELETE FROM sitemap_entries;DELETE FROM sitemap_roots;DELETE FROM sitemap_documents;").unwrap();
    let mut summary = catalog.summary.clone();
    summary.last_sequence = 0;
    summary.root_rows = 0;
    summary.document_rows = 0;
    summary.entry_rows = 0;
    summary.location_rows = 0;
    summary.incomplete_documents = 0;
    summary.rolling_sha256 = seed_hash();
    for row in catalog.rows.as_ref().unwrap() {
        account(&mut summary, row).unwrap();
        insert_row(connection, row, &summary.limits).unwrap();
    }
    catalog.summary = summary;
    refresh_size(&mut catalog).unwrap();
    save(connection, SUMMARY_KEY, &catalog.summary).unwrap();
    if let Some(basis) = &catalog.inherited {
        save(connection, INHERITED_KEY, basis).unwrap();
    }
}

#[test]
fn verified_raw_requires_transaction_and_preserves_unknown_and_initialized_empty() {
    let mut connection = Connection::open_in_memory().unwrap();
    empty(&connection);
    assert!(verify_raw(&connection).is_err());
    let tx = connection.transaction().unwrap();
    assert!(verify_raw(&tx).unwrap().is_none());
    initialize(&tx, &"a".repeat(64), &Default::default(), false).unwrap();
    let old = load_catalog(&tx).unwrap().unwrap();
    let read = verify_raw(&tx).unwrap().unwrap();
    assert_eq!(read.summary(), &old.summary);
    assert!(std::ptr::eq(read.connection(), &*tx));
    assert!(read.inherited().is_none());
    assert!(
        read.window(&Default::default(), None, 1)
            .unwrap()
            .rows
            .is_empty()
    );
    assert!(read.document_by_id(&"b".repeat(64)).unwrap().is_none());
}

#[test]
fn raw_window_and_visitor_agree_with_full_oracle_for_all_filter_roles() {
    let mut connection = memory();
    let tx = connection.transaction().unwrap();
    let old = load_catalog(&tx).unwrap().unwrap();
    let read = verify_raw(&tx).unwrap().unwrap();
    assert_eq!(read.summary(), &old.summary);
    let old_rows = old.rows.as_ref().unwrap();
    let doc = old_rows
        .iter()
        .find_map(|r| {
            if let SitemapLedgerRow::Document(d) = r {
                Some(d)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        read.document_by_id(&doc.observation_id).unwrap().as_ref(),
        Some(doc.as_ref())
    );
    for kind in [
        SitemapLedgerRowKind::All,
        SitemapLedgerRowKind::Root,
        SitemapLedgerRowKind::Document,
        SitemapLedgerRowKind::Entry,
        SitemapLedgerRowKind::Location,
    ] {
        let filter = SitemapLedgerFilter {
            kind,
            ..Default::default()
        };
        let expected: Vec<_> = old_rows
            .iter()
            .filter(|r| kind == SitemapLedgerRowKind::All || r.kind() == kind.name())
            .cloned()
            .collect();
        let mut visited = Vec::new();
        read.visit_filtered(&filter, |r| {
            visited.push(r.clone());
            Ok(())
        })
        .unwrap();
        assert_eq!(visited, expected);
        let mut pages = Vec::new();
        let mut after = None;
        loop {
            let page = read.window(&filter, after, 1).unwrap();
            assert!(page.rows.len() <= 1);
            assert_eq!(page.ledger.as_ref(), Some(&old.summary));
            pages.extend(page.rows);
            if !page.has_more {
                break;
            }
            after = page.next_after_sequence;
        }
        assert_eq!(pages, expected);
    }
    let filter = SitemapLedgerFilter {
        source_observation_id: Some(doc.observation_id.clone()),
        membership_url: Some("https://example.org/p?a=1&b=2".into()),
        ..Default::default()
    };
    let page = read.window(&filter, None, 100).unwrap();
    assert_eq!(page.rows.len(), 1);
    assert!(matches!(&page.rows[0], SitemapLedgerRow::Location(_)));
    assert!(
        read.window(&filter, Some(doc.sequence), 1).is_err(),
        "after must match the filter"
    );
    assert!(read.window(&filter, Some(0), 1).is_err());
    assert!(read.window(&filter, None, 0).is_err());
    assert!(read.window(&filter, None, 1001).is_err());
}

#[test]
fn filter_validation_is_exact_and_visitor_errors_propagate() {
    let mut connection = memory();
    let tx = connection.transaction().unwrap();
    let read = verify_raw(&tx).unwrap().unwrap();
    for filter in [
        SitemapLedgerFilter {
            source_observation_id: Some("A".repeat(64)),
            ..Default::default()
        },
        SitemapLedgerFilter {
            membership_url: Some("https://example.org".into()),
            ..Default::default()
        },
        SitemapLedgerFilter {
            membership_url: Some("https://example.org/\n".into()),
            ..Default::default()
        },
    ] {
        assert!(read.window(&filter, None, 1).is_err());
    }
    let mut visited = 0;
    let result = read.visit_filtered(&Default::default(), |_| {
        visited += 1;
        anyhow::bail!("directed visitor failure")
    });
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("directed visitor failure")
    );
    assert_eq!(visited, 1);
}

#[test]
fn corrupt_tail_and_same_row_sql_projection_fail_before_any_window() {
    for projection in [false, true] {
        let mut connection = memory();
        if projection {
            connection.execute("UPDATE sitemap_locations SET ordinal=999 WHERE sequence=(SELECT max(sequence) FROM sitemap_locations)",[]).unwrap();
        } else {
            connection.execute("UPDATE sitemap_locations SET data=data||' ' WHERE sequence=(SELECT max(sequence) FROM sitemap_locations)",[]).unwrap();
        }
        let tx = connection.transaction().unwrap();
        assert!(load_catalog(&tx).is_err());
        assert!(
            verify_raw(&tx).is_err(),
            "full tail verification precedes even a first one-row window"
        );
    }
}

#[test]
fn global_id_duplicates_are_rejected_even_without_imported_unique_constraints() {
    let mut connection = memory();
    connection.execute_batch("ALTER TABLE sitemap_roots RENAME TO original_roots;
      CREATE TABLE sitemap_roots(sequence INTEGER,row_id TEXT,observation_id TEXT,ordinal INTEGER,data TEXT,data_sha256 TEXT);
      DROP TABLE original_roots;
      INSERT INTO sitemap_roots SELECT 999,row_id,NULL,0,data,data_sha256 FROM sitemap_documents;").unwrap();
    let tx = connection.transaction().unwrap();
    let error = verify_raw(&tx).err().expect("duplicate across kinds");
    assert!(
        error.to_string().contains("Duplicate raw row ID"),
        "{error:#}"
    );
    assert!(load_catalog(&tx).is_err());
}

#[test]
fn duplicate_global_sequence_and_document_ordinal_reorder_are_rejected() {
    for global_sequence in [true, false] {
        let mut connection = memory();
        let mut old = load_catalog(&connection).unwrap().unwrap();
        if global_sequence {
            let row = old
                .rows
                .as_mut()
                .unwrap()
                .iter_mut()
                .find(|r| matches!(r, SitemapLedgerRow::Entry(_)))
                .unwrap();
            row.set_sequence(1);
            let data = row_data(row, &old.summary.limits).unwrap();
            connection
                .execute(
                    "UPDATE sitemap_entries SET sequence=1,data=?1,data_sha256=?2 WHERE ordinal=1",
                    params![data, digest(&data)],
                )
                .unwrap();
        } else {
            let rows = old.rows.as_mut().unwrap();
            let positions: Vec<_> = rows
                .iter()
                .enumerate()
                .filter_map(|(i, r)| matches!(r, SitemapLedgerRow::Entry(_)).then_some(i))
                .collect();
            rows.swap(positions[0], positions[1]);
            for (index, row) in rows.iter_mut().enumerate() {
                row.set_sequence(index as u64 + 1);
            }
            install_unchecked(&connection, old);
        }
        let tx = connection.transaction().unwrap();
        assert!(load_catalog(&tx).is_err());
        assert!(verify_raw(&tx).is_err());
    }
}

#[test]
fn entry_declared_loc_order_is_not_tightened_but_multiset_must_match() {
    let multi = "<urlset><url><loc>https://example.org/a</loc><loc>https://example.org/b</loc></url></urlset>";
    let mut connection = Connection::open_in_memory().unwrap();
    empty(&connection);
    {
        let tx = connection.transaction().unwrap();
        initialize(&tx, &"a".repeat(64), &Default::default(), false).unwrap();
        let mut rows = source_rows(multi);
        let entry = rows
            .iter_mut()
            .find_map(|r| {
                if let SitemapLedgerRow::Entry(v) = r {
                    Some(v)
                } else {
                    None
                }
            })
            .unwrap();
        entry.observed.loc_occurrence_ordinals.reverse();
        append(&tx, rows).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = connection.transaction().unwrap();
        let old = load_catalog(&tx).unwrap().unwrap();
        assert_eq!(verify_raw(&tx).unwrap().unwrap().summary(), &old.summary);
    }
    let mut invalid = load_catalog(&connection).unwrap().unwrap();
    let entry = invalid
        .rows
        .as_mut()
        .unwrap()
        .iter_mut()
        .find_map(|r| {
            if let SitemapLedgerRow::Entry(v) = r {
                Some(v)
            } else {
                None
            }
        })
        .unwrap();
    entry.observed.loc_occurrence_ordinals = vec![1, 1];
    install_unchecked(&connection, invalid);
    let tx = connection.transaction().unwrap();
    assert!(load_catalog(&tx).is_err());
    assert!(
        verify_raw(&tx).is_err(),
        "re-sealed hashes do not replace relation verification"
    );
}

#[test]
fn exact_full_serialized_envelope_size_is_verified_not_just_sql_data_sum() {
    let mut connection = memory();
    let mut catalog = load_catalog(&connection).unwrap().unwrap();
    catalog.summary.catalog_json_bytes -= 1;
    save(&connection, SUMMARY_KEY, &catalog.summary).unwrap();
    let tx = connection.transaction().unwrap();
    assert!(load_catalog(&tx).is_err());
    let error = verify_raw(&tx).err().expect("wrong complete envelope size");
    assert!(
        error.to_string().contains("serialized envelope"),
        "{error:#}"
    );
}

#[test]
fn missing_entry_and_actual_committed_policy_row_are_not_skipped() {
    for policy in [false, true] {
        let mut connection = memory();
        let mut catalog = load_catalog(&connection).unwrap().unwrap();
        if policy {
            let revision = catalog
                .rows
                .as_ref()
                .unwrap()
                .iter()
                .find_map(|r| {
                    if let SitemapLedgerRow::Document(d) = r {
                        Some(d.source_revision.clone())
                    } else {
                        None
                    }
                })
                .unwrap();
            let loc = catalog
                .rows
                .as_mut()
                .unwrap()
                .iter_mut()
                .find_map(|r| {
                    if let SitemapLedgerRow::Location(l) = r {
                        Some(l)
                    } else {
                        None
                    }
                })
                .unwrap();
            loc.admission.decision = SitemapAdmissionDecision::PolicyDenied;
            loc.admission.admitted_fetch_url = None;
            loc.admission.policy_occurrence_id = Some("b".repeat(64));
            loc.admission.policy_source_revision = Some(revision);
            install_unchecked(&connection, catalog);
        } else {
            connection
                .execute_batch(
                    "PRAGMA foreign_keys=OFF;DELETE FROM sitemap_entries WHERE ordinal=1;",
                )
                .unwrap();
        }
        let tx = connection.transaction().unwrap();
        assert!(load_catalog(&tx).is_err());
        assert!(verify_raw(&tx).is_err());
    }
}

#[test]
fn inherited_raw_anchor_uses_explicit_old_materializing_compatibility() {
    let parent_connection = memory();
    let parent = load_catalog(&parent_connection).unwrap().unwrap();
    let bytes = serde_json::to_vec(&serde_json::json!({"sitemap_catalog":parent})).unwrap();
    let parent_sha = digest(&bytes);
    let mut connection = Connection::open_in_memory().unwrap();
    empty(&connection);
    connection
        .execute(
            "INSERT INTO recrawl_base(id,snapshot) VALUES(1,?1)",
            [&bytes],
        )
        .unwrap();
    save(
        &connection,
        "run_plan",
        &serde_json::json!({"kind":"selected_recrawl","plan":{
            "engine":crate::SELECTED_RECRAWL_ENGINE,"parent":{"snapshot_sha256":parent_sha}
        }}),
    )
    .unwrap();
    {
        let tx = connection.transaction().unwrap();
        // This fixture exercises only the ledger's low-level exact-parent
        // compatibility contract, not Store's full selected-plan validation.
        inherit(&tx, &parent, &parent_sha).unwrap();
        let old = load_catalog(&tx).unwrap().unwrap();
        let read = verify_raw(&tx).unwrap().unwrap();
        assert_eq!(read.summary(), &old.summary);
        assert_eq!(read.inherited(), old.inherited.as_ref());
        assert_eq!(
            read.window(&Default::default(), None, 100).unwrap().rows,
            old.rows.unwrap()
        );
        tx.commit().unwrap();
    }
    connection
        .execute(
            "UPDATE recrawl_base SET snapshot=CAST(snapshot AS TEXT)||' ' WHERE id=1",
            [],
        )
        .unwrap();
    let tx = connection.transaction().unwrap();
    assert!(verify_raw(&tx).is_err());
}

#[test]
fn read_only_file_and_wal_snapshot_are_immutable_and_stable() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("raw.sqlite");
    {
        let mut writer = Connection::open(&path).unwrap();
        writer.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        populated(&mut writer);
    }
    let before = std::fs::read(&path).unwrap();
    let mut reader =
        Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    {
        let tx = reader.transaction().unwrap();
        let read = verify_raw(&tx).unwrap().unwrap();
        let original = read.window(&Default::default(), None, 100).unwrap().rows;
        let writer = Connection::open(&path).unwrap();
        writer.execute("UPDATE sitemap_locations SET data=data||' ' WHERE sequence=(SELECT max(sequence) FROM sitemap_locations)",[]).unwrap();
        assert_eq!(
            read.window(&Default::default(), None, 100).unwrap().rows,
            original
        );
        tx.commit().unwrap();
    }
    let tx = reader.transaction().unwrap();
    assert!(
        verify_raw(&tx).is_err(),
        "new snapshot sees the corrupt writer tail"
    );
    tx.commit().unwrap();
    drop(reader);
    // Writer corruption changes the fixture DB. A separate fresh read-only
    // verification checks byte immutability without mixing that deliberate write.
    let clean_path = temp.path().join("clean.sqlite");
    {
        let mut conn = Connection::open(&clean_path).unwrap();
        populated(&mut conn);
    }
    let clean_before = std::fs::read(&clean_path).unwrap();
    {
        let mut conn =
            Connection::open_with_flags(&clean_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let tx = conn.transaction().unwrap();
        verify_raw(&tx)
            .unwrap()
            .unwrap()
            .visit_filtered(&Default::default(), |_| Ok(()))
            .unwrap();
        tx.commit().unwrap();
    }
    assert_eq!(std::fs::read(&clean_path).unwrap(), clean_before);
    assert!(!before.is_empty());
}

#[test]
fn partial_initialized_schema_is_rejected_without_repair() {
    let mut connection = memory();
    connection
        .execute_batch("DROP TABLE sitemap_locations;")
        .unwrap();
    let tx = connection.transaction().unwrap();
    assert!(verify_raw(&tx).is_err());
    let still_absent: bool = tx
        .query_row(
            "SELECT NOT EXISTS(SELECT 1 FROM sqlite_master WHERE name='sitemap_locations')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(
        still_absent,
        "verification must not create or repair tables"
    );
}

#[test]
fn oversized_sql_projection_is_rejected_against_bounded_canonical_payload() {
    let mut connection = memory();
    // SQL creates the deliberately inconsistent large index string only if this
    // authored regression is executed. The raw JSON remains the actual small
    // parser-derived document. Production borrows this projection before compare.
    connection
        .execute(
            "UPDATE sitemap_documents SET task_url=printf('%01048576d',1)",
            [],
        )
        .unwrap();
    let tx = connection.transaction().unwrap();
    assert!(load_catalog(&tx).is_err());
    let error = verify_raw(&tx)
        .err()
        .expect("SQL/document projection mismatch");
    assert!(
        error.to_string().contains("Document SQL indexes"),
        "{error:#}"
    );
}

#[test]
fn real_eight_mib_window_cap_refuses_large_page_and_smaller_window_recovers() {
    // Actual unchanged 8 MiB page cap, not a lowered test-only threshold. XML is
    // below the default 8 MiB source cap; repeated bounded lastmod fields make
    // real typed retained Entry rows large enough for the JSON page to exceed it.
    let mut xml = String::from("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'>");
    let value = "1".repeat(6000);
    for index in 0..1000 {
        xml.push_str(&format!(
            "<url><loc>https://example.org/p{index}</loc><lastmod>{value}</lastmod></url>"
        ));
    }
    xml.push_str("</urlset>");
    assert!(xml.len() < 8 * 1024 * 1024);
    let mut connection = Connection::open_in_memory().unwrap();
    empty(&connection);
    {
        let tx = connection.transaction().unwrap();
        initialize(&tx, &"a".repeat(64), &Default::default(), false).unwrap();
        append(&tx, source_rows(&xml)).unwrap();
        tx.commit().unwrap();
    }
    let tx = connection.transaction().unwrap();
    let old = load_catalog(&tx).unwrap().unwrap();
    let read = verify_raw(&tx).unwrap().unwrap();
    assert_eq!(read.summary(), &old.summary);
    let filter = SitemapLedgerFilter {
        kind: SitemapLedgerRowKind::Entry,
        ..Default::default()
    };
    let budget = read.window(&filter, None, 1000).unwrap_err();
    assert!(budget.is::<RawWindowByteBudget>());
    let small = read.window(&filter, None, 10).unwrap();
    assert_eq!(small.rows.len(), 10);
    assert!(small.has_more);
    let mut count = 0;
    read.visit_filtered(&filter, |_| {
        count += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(
        count, 1000,
        "full visitor is not truncated to the failed page"
    );
}

// SOURCE-ONLY STATELESS OPTIMIZATION REGRESSIONS. NOT_COMPILED / NOT_RUN.
#[test]
fn stateless_canonical_envelope_matches_unchanged_full_serializer_for_empty_unicode_and_incomplete_rows()
 {
    let variants = [
        None,
        Some("<urlset/>"),
        Some(
            "<urlset><url><loc>https://example.org/p?a=1&amp;b=2</loc><lastmod><![CDATA[välué 💡 \" \\]]></lastmod></url></urlset>",
        ),
        Some(
            "<urlset><url><loc>https://example.org/a</loc><loc>https://example.org/b</loc></url><url/></urlset>",
        ),
        Some("<urlset><url><loc>https://example.org/incomplete</loc>"),
    ];
    for xml in variants {
        let mut connection = Connection::open_in_memory().unwrap();
        empty(&connection);
        let tx = connection.transaction().unwrap();
        initialize(&tx, &"a".repeat(64), &Default::default(), false).unwrap();
        if let Some(xml) = xml {
            append(&tx, source_rows(xml)).unwrap();
        }
        let original = load_catalog(&tx).unwrap().unwrap();
        let raw = verify_raw(&tx).unwrap().unwrap();
        assert_eq!(raw.summary(), &original.summary);
        let work = raw.verification_work();
        assert_eq!(work.primary_scans, 1);
        assert_eq!(work.primary_rows_verified, original.summary.last_sequence);
        assert_eq!(
            work.primary_canonical_serializations,
            original.summary.last_sequence
        );
        assert_eq!(
            work.filtered_rows_decoded, 0,
            "production size verification must not revisit rows"
        );
        // This is the old serializer implementation, unchanged from SourceV4.
        let oracle = measured(
            &CatalogEnvelope { read: &raw },
            original.summary.limits.max_catalog_json_bytes,
        )
        .unwrap();
        assert_eq!(oracle, original.summary.catalog_json_bytes);
        assert_eq!(serde_json::to_vec(&original).unwrap().len() as u64, oracle);
        assert_eq!(
            raw.verification_work().filtered_rows_decoded,
            original.summary.last_sequence,
            "the unchanged oracle must actually traverse every retained row"
        );
    }
}

#[test]
fn stateless_fifty_thousand_pairs_have_one_primary_pass_one_merge_join_and_exact_reopen_oracle() {
    let mut xml = String::from("<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'>");
    for ordinal in 0..50_000 {
        xml.push_str(&format!("<url><loc>http://a/{ordinal:05}</loc></url>"));
    }
    xml.push_str("</urlset>");
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("actual-50000-stateless.sqlite");
    let mut connection = Connection::open(&file).unwrap();
    empty(&connection);
    let limits = SitemapLedgerLimits {
        max_catalog_json_bytes: 128 * 1024 * 1024,
        ..Default::default()
    };
    let config = crate::CrawlConfig {
        sitemap_membership_limits: Some(limits.clone()),
        ..Default::default()
    };
    save(&connection, "config", &config).unwrap();
    {
        let tx = connection.transaction().unwrap();
        initialize(
            &tx,
            &digest(serde_json::to_vec(&config).unwrap()),
            &limits,
            false,
        )
        .unwrap();
        append(&tx, source_rows(&xml)).unwrap();
        tx.commit().unwrap();
    }
    for cold in [false, true] {
        let mut read_only;
        let actual = if cold {
            read_only =
                Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .unwrap();
            &mut read_only
        } else {
            &mut connection
        };
        let tx = actual.transaction().unwrap();
        let raw = verify_raw(&tx).unwrap().unwrap();
        assert_eq!(raw.summary().entry_rows, 50_000);
        assert_eq!(raw.summary().location_rows, 50_000);
        assert_eq!(raw.summary().last_sequence, 100_001);
        assert!(raw.summary().catalog_json_bytes > 32 * 1024 * 1024);
        let work = raw.verification_work();
        assert_eq!(work.primary_scans, 1);
        assert_eq!(work.primary_rows_verified, 100_001);
        assert_eq!(work.primary_canonical_serializations, 100_001);
        assert_eq!(work.entry_loc_join_scans, 1);
        assert_eq!(work.relation_entry_statements, 1);
        assert_eq!(work.relation_location_statements, 1);
        assert_eq!(work.relation_entries_decoded, 50_000);
        assert_eq!(work.relation_locations_decoded, 50_000);
        assert_eq!(work.filtered_rows_decoded, 0);
        assert_eq!(
            measured(
                &CatalogEnvelope { read: &raw },
                limits.max_catalog_json_bytes
            )
            .unwrap(),
            raw.summary().catalog_json_bytes
        );
        assert_eq!(raw.verification_work().filtered_rows_decoded, 100_001);
        let tail = raw
            .window(
                &SitemapLedgerFilter {
                    kind: SitemapLedgerRowKind::Location,
                    ..Default::default()
                },
                Some(100_000),
                200,
            )
            .unwrap();
        assert_eq!(tail.rows.len(), 1);
        assert!(!tail.has_more);
        assert!(matches!(&tail.rows[0], SitemapLedgerRow::Location(loc)
            if loc.observed.decoded_value.as_deref() == Some("http://a/49999")));
        drop(raw);
        tx.commit().unwrap();
    }
}

#[test]
fn stateless_streaming_relation_keeps_orphan_span_prior_sequence_and_missing_loc_guards() {
    for mutation in [
        "orphan",
        "span",
        "prior_sequence",
        "incomplete_missing",
        "declared_multiset",
    ] {
        let xml = if mutation == "incomplete_missing" {
            "<urlset><url/></urlset>"
        } else {
            "<urlset><url><loc>https://example.org/p</loc></url></urlset>"
        };
        let mut connection = Connection::open_in_memory().unwrap();
        empty(&connection);
        {
            let tx = connection.transaction().unwrap();
            initialize(&tx, &"a".repeat(64), &Default::default(), false).unwrap();
            append(&tx, source_rows(xml)).unwrap();
            tx.commit().unwrap();
        }
        let mut catalog = load_catalog(&connection).unwrap().unwrap();
        let rows = catalog.rows.as_mut().unwrap();
        match mutation {
            "orphan" => {
                for row in rows.iter_mut() {
                    match row {
                        SitemapLedgerRow::Entry(entry) => {
                            entry.observed.loc_occurrence_ordinals.clear()
                        }
                        SitemapLedgerRow::Location(loc) => loc.observed.entry_ordinal = 999,
                        _ => {}
                    }
                }
            }
            "span" => {
                for row in rows.iter_mut() {
                    if let SitemapLedgerRow::Location(loc) = row {
                        loc.observed.xml_byte_start = 0;
                    }
                }
            }
            "prior_sequence" => {
                rows.swap(1, 2);
                for (index, row) in rows.iter_mut().enumerate() {
                    row.set_sequence(index as u64 + 1);
                }
            }
            "incomplete_missing" => {
                for row in rows.iter_mut() {
                    if let SitemapLedgerRow::Entry(entry) = row {
                        entry.observed.complete = false;
                    }
                }
            }
            "declared_multiset" => {
                for row in rows.iter_mut() {
                    if let SitemapLedgerRow::Entry(entry) = row {
                        entry.observed.loc_occurrence_ordinals = vec![999];
                    }
                }
            }
            _ => unreachable!(),
        }
        connection
            .execute_batch("PRAGMA foreign_keys=OFF;")
            .unwrap();
        // Re-seal actual canonical row/hash/summary bytes. Mere hash checking
        // cannot reject these controls; semantic relations must still do so.
        install_unchecked(&connection, catalog);
        let tx = connection.transaction().unwrap();
        assert!(
            load_catalog(&tx).is_err(),
            "old materialized oracle: {mutation}"
        );
        assert!(
            verify_raw(&tx).is_err(),
            "complete streaming relation: {mutation}"
        );
    }
}

#[test]
fn stateless_streaming_join_works_without_foreign_keys_or_indexes_and_rejects_tail_corruption() {
    let mut connection = memory();
    let original = load_catalog(&connection).unwrap().unwrap();
    connection.execute_batch("PRAGMA foreign_keys=OFF;PRAGMA automatic_index=OFF;
        CREATE TABLE stateless_entries AS SELECT * FROM sitemap_entries;
        CREATE TABLE stateless_locations AS SELECT * FROM sitemap_locations;
        DROP TABLE sitemap_locations;DROP TABLE sitemap_entries;
        CREATE TABLE sitemap_entries(sequence INTEGER,row_id TEXT COLLATE NOCASE,observation_id TEXT,ordinal INTEGER,data TEXT,data_sha256 TEXT);
        CREATE TABLE sitemap_locations(sequence INTEGER,row_id TEXT,observation_id TEXT,entry_id TEXT COLLATE NOCASE,ordinal INTEGER,data TEXT,data_sha256 TEXT);
        INSERT INTO sitemap_entries SELECT * FROM stateless_entries;
        INSERT INTO sitemap_locations SELECT * FROM stateless_locations;
        DROP TABLE stateless_entries;DROP TABLE stateless_locations;").unwrap();
    {
        let tx = connection.transaction().unwrap();
        let raw = verify_raw(&tx).unwrap().unwrap();
        assert_eq!(raw.summary(), &original.summary);
        assert_eq!(
            raw.window(&Default::default(), None, 1000).unwrap().rows,
            original.rows.unwrap()
        );
        assert_eq!(raw.verification_work().entry_loc_join_scans, 1);
    }
    connection.execute("UPDATE sitemap_locations SET entry_id=upper(entry_id) WHERE sequence=(SELECT max(sequence) FROM sitemap_locations)", []).unwrap();
    let tx = connection.transaction().unwrap();
    assert!(load_catalog(&tx).is_err());
    assert!(
        verify_raw(&tx).is_err(),
        "NOCASE join cannot conceal a bad exact tail SQL index"
    );
}

#[test]
fn stateless_single_canonical_serialization_rejects_rehashed_whitespace_and_equivalent_json_escaping()
 {
    for escaped in [false, true] {
        let mut connection = memory();
        let (sequence, data): (i64, String) = connection
            .query_row(
                "SELECT sequence,data FROM sitemap_locations ORDER BY sequence LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let altered = if escaped {
            data.replacen("https", r#"h\u0074tps"#, 1)
        } else {
            format!("{data} ")
        };
        assert_ne!(altered, data);
        assert_eq!(
            serde_json::from_str::<SitemapLedgerRow>(&altered).unwrap(),
            serde_json::from_str::<SitemapLedgerRow>(&data).unwrap()
        );
        connection
            .execute(
                "UPDATE sitemap_locations SET data=?1,data_sha256=?2 WHERE sequence=?3",
                params![altered, digest(&altered), sequence],
            )
            .unwrap();
        let tx = connection.transaction().unwrap();
        assert!(load_catalog(&tx).is_err());
        let error = verify_raw(&tx)
            .err()
            .expect("rehashed noncanonical bytes refused");
        assert!(error.to_string().contains("Noncanonical"), "{error:#}");
    }
}

// Database TEXT encoding and canonical JSON UTF8 bytes are distinct measures.
// These disk/reopen regressions are authored only; NOT_COMPILED / NOT_RUN.
fn assert_stateless_utf16_disk_catalog_and_read_only_reopen(encoding: &str) {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join(format!("canonical-{encoding}.sqlite"));
    let mut connection = Connection::open(&file).unwrap();
    connection
        .execute_batch(match encoding {
            "UTF-16le" => "PRAGMA encoding='UTF-16le';",
            "UTF-16be" => "PRAGMA encoding='UTF-16be';",
            _ => unreachable!(),
        })
        .unwrap();
    // Encoding must be selected before the first table is created.
    empty(&connection);
    {
        let tx = connection.transaction().unwrap();
        initialize(&tx, &"a".repeat(64), &Default::default(), false).unwrap();
        append(&tx, source_rows("<urlset><url><loc>https://example.org/über?x=é&amp;y=💡</loc><lastmod><![CDATA[välué 💡]]></lastmod></url><url/></urlset>")).unwrap();
        tx.commit().unwrap();
    }
    let original = load_catalog(&connection).unwrap().unwrap();
    let canonical_utf8_bytes: u64 = original
        .rows
        .as_ref()
        .unwrap()
        .iter()
        .map(|row| serde_json::to_vec(row).unwrap().len() as u64)
        .sum();
    let largest_canonical_utf8_row = original
        .rows
        .as_ref()
        .unwrap()
        .iter()
        .map(|row| serde_json::to_vec(row).unwrap().len())
        .max()
        .unwrap();
    let check = |connection: &mut Connection| {
        let tx = connection.transaction().unwrap();
        let actual_encoding: String = tx
            .query_row("PRAGMA encoding", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            actual_encoding, encoding,
            "real disk database must retain its UTF16 encoding"
        );
        let mut sql_blob_bytes = 0u64;
        let mut largest_sql_blob = 0u64;
        for table in TABLES {
            let (bytes, max): (i64, i64) = tx.query_row(&format!(
                "SELECT coalesce(sum(length(CAST(data AS BLOB))),0),coalesce(max(length(CAST(data AS BLOB))),0) FROM {table}"), [],
                |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
            sql_blob_bytes += u64::try_from(bytes).unwrap();
            largest_sql_blob = largest_sql_blob.max(u64::try_from(max).unwrap());
        }
        assert_ne!(
            sql_blob_bytes, canonical_utf8_bytes,
            "SQL BLOB bytes are database-encoded, not the canonical Rust UTF8 row sum"
        );
        assert!(sql_blob_bytes <= original.summary.limits.max_catalog_json_bytes as u64);
        assert!(largest_sql_blob <= original.summary.limits.max_row_json_bytes as u64);
        assert!(
            largest_sql_blob > largest_canonical_utf8_row as u64,
            "later negative control must isolate the unchanged SQL BLOB row cap"
        );
        let old = load_catalog(&tx).unwrap().unwrap();
        assert_eq!(old, original);
        let raw = verify_raw(&tx).unwrap().unwrap();
        assert_eq!(raw.summary(), &original.summary);
        assert_eq!(
            raw.verification_work().primary_canonical_serializations,
            original.summary.last_sequence
        );
        assert_eq!(raw.verification_work().filtered_rows_decoded, 0);
        let oracle = measured(
            &CatalogEnvelope { read: &raw },
            original.summary.limits.max_catalog_json_bytes,
        )
        .unwrap();
        assert_eq!(oracle, original.summary.catalog_json_bytes);
        assert_eq!(oracle, serde_json::to_vec(&old).unwrap().len() as u64);
        assert_eq!(
            raw.verification_work().filtered_rows_decoded,
            original.summary.last_sequence,
            "unchanged full serializer must really traverse all rows"
        );
        let window = raw.window(&Default::default(), None, 1000).unwrap();
        assert_eq!(&window.rows, original.rows.as_ref().unwrap());
        assert!(!window.has_more);
        drop(raw);
        tx.commit().unwrap();
    };
    check(&mut connection);
    drop(connection);
    // The writer Connection is actually dropped before this read-only reopen.
    let mut read_only =
        Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    check(&mut read_only);
    drop(read_only);

    // Re-seal a catalog whose canonical UTF8 rows fit a smaller row budget,
    // while its legal UTF16 SQL BLOB rows exceed that same declared budget.
    // Both old and new readers must retain the conservative SQL preflight cap.
    let connection = Connection::open(&file).unwrap();
    let mut narrow = original.clone();
    narrow.summary.limits.max_row_json_bytes = largest_canonical_utf8_row;
    refresh_size(&mut narrow).unwrap();
    save(&connection, SUMMARY_KEY, &narrow.summary).unwrap();
    drop(connection);
    let mut read_only =
        Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let tx = read_only.transaction().unwrap();
    assert!(
        load_catalog(&tx).is_err(),
        "old materialized reader retains SQL BLOB cap: {encoding}"
    );
    let error = verify_raw(&tx)
        .err()
        .expect("UTF16 SQL BLOB row cap must remain enforced");
    assert!(
        error.to_string().contains("Oversized raw row"),
        "{encoding}: {error:#}"
    );
}

#[test]
fn stateless_utf16le_disk_catalog_and_read_only_reopen_match_old_serializer_and_preserve_blob_caps()
{
    assert_stateless_utf16_disk_catalog_and_read_only_reopen("UTF-16le");
}

#[test]
fn stateless_utf16be_disk_catalog_and_read_only_reopen_match_old_serializer_and_preserve_blob_caps()
{
    assert_stateless_utf16_disk_catalog_and_read_only_reopen("UTF-16be");
}
