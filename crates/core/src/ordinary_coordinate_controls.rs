//! Shared authored regressions. Root executes the identical assertions against
//! unchanged V4 and V5. Minimal SQL carriers are not a full typed Store proof.
use super::*;

const ORIGINAL_A: &str = "https://example.org/map-a.xml";
const ORIGINAL_B: &str = "https://example.org/map-b.xml";

fn original_config(connection: &Connection) {
    save(connection,"config",&serde_json::json!({
        "explicit_sitemap_inputs":[ORIGINAL_A],
        "input_imports":[{"input_sha256":"b".repeat(64),"rows":[
            {"candidate_kind":"page_url","source_index":1,"raw_value":"https://example.org/page","normalized_url":"https://example.org/page","error":null},
            {"candidate_kind":"sitemap_document","source_index":27,"raw_value":ORIGINAL_B,"normalized_url":ORIGINAL_B,"error":null}]}]
    })).unwrap();
    for task in [ORIGINAL_A, ORIGINAL_B] {
        connection
            .execute(
                "INSERT OR IGNORE INTO discovery_frontier(kind,url) VALUES('sitemap',?1)",
                [task],
            )
            .unwrap();
    }
}
fn ordinary_root(
    kind: SitemapRootKind,
    ordinal: u64,
    raw: &str,
    normalized: Option<&str>,
) -> SitemapRootDeclaration {
    let normalized = normalized.map(str::to_owned);
    SitemapRootDeclaration {
        sequence: 0,
        declaration_id: root_declaration_id(&kind, &"a".repeat(64), ordinal),
        kind,
        source_observation_id: None,
        source_ordinal: ordinal,
        raw_line: None,
        raw_value: raw.into(),
        normalized_document_url: normalized.clone(),
        robots_evidence: None,
        admission: SitemapAdmissionBinding {
            decision: if normalized.is_some() {
                SitemapAdmissionDecision::Scheduled
            } else {
                SitemapAdmissionDecision::InvalidLocation
            },
            membership_url: normalized.clone(),
            admitted_fetch_url: normalized,
            coalesced_policy_occurrence_id: None,
            policy_occurrence_id: None,
            policy_source_revision: None,
        },
    }
}
fn ordinary_append(connection: &mut Connection, root: SitemapRootDeclaration) -> Result<()> {
    let tx = connection.transaction()?;
    append_roots(&tx, 1, vec![root])?;
    tx.commit()?;
    Ok(())
}
fn rewrite_root_and_all_catalog_digests(
    connection: &Connection,
    change: impl FnOnce(&mut SitemapRootDeclaration),
) {
    let mut catalog = load_catalog(connection).unwrap().unwrap();
    {
        let SitemapLedgerRow::Root(root) = &mut catalog.rows.as_mut().unwrap()[0] else {
            panic!("one root fixture")
        };
        change(root);
    }
    let row = &catalog.rows.as_ref().unwrap()[0];
    let data = row_data(row, &catalog.summary.limits).unwrap();
    connection
        .execute(
            "UPDATE sitemap_roots SET data=?1,data_sha256=?2 WHERE sequence=1",
            params![data, digest(&data)],
        )
        .unwrap();
    catalog.summary.rolling_sha256 = roll(&seed_hash(), row.kind(), &data);
    refresh_size(&mut catalog).unwrap();
    save(connection, SUMMARY_KEY, &catalog.summary).unwrap();
}

#[test]
fn ordinary_explicit_exact_original_raw_and_normalization_are_accepted() {
    let mut connection = connection(Default::default());
    let raw = format!("  {ORIGINAL_A}  ");
    save(&connection,"config",&serde_json::json!({"explicit_sitemap_inputs":[raw],"irrelevant_large_value":"x".repeat(100_000)})).unwrap();
    ordinary_append(
        &mut connection,
        ordinary_root(SitemapRootKind::ExplicitConfig, 0, &raw, Some(ORIGINAL_A)),
    )
    .unwrap();
    assert_eq!(
        load_catalog(&connection)
            .unwrap()
            .unwrap()
            .summary
            .root_rows,
        1
    );
}
#[test]
fn ordinary_two_field_original_identity_swap_to_existing_imported_task_rejects_append() {
    let mut connection = connection(Default::default());
    original_config(&connection);
    let mut root = ordinary_root(
        SitemapRootKind::ExplicitConfig,
        0,
        ORIGINAL_A,
        Some(ORIGINAL_A),
    );
    root.normalized_document_url = Some(ORIGINAL_B.into());
    root.admission.membership_url = Some(ORIGINAL_B.into());
    assert_eq!(
        root.admission.admitted_fetch_url.as_deref(),
        Some(ORIGINAL_A)
    );
    named_failure(
        ordinary_append(&mut connection, root),
        "Ordinary admission alias differs from original membership",
    );
    assert_eq!(count(&connection, "sitemap_roots"), 0);
    assert_eq!(count(&connection, "discovery_frontier"), 3);
}
#[test]
fn ordinary_raw_identity_and_alias_swap_to_existing_task_rejects_original_ordinal() {
    let mut connection = connection(Default::default());
    original_config(&connection);
    named_failure(
        ordinary_append(
            &mut connection,
            ordinary_root(
                SitemapRootKind::ExplicitConfig,
                0,
                ORIGINAL_B,
                Some(ORIGINAL_B),
            ),
        ),
        "Explicit root source ordinal/raw differs",
    );
    assert_eq!(count(&connection, "sitemap_roots"), 0);
}
#[test]
fn ordinary_original_identity_cannot_admit_a_different_existing_task_alias() {
    let mut connection = connection(Default::default());
    original_config(&connection);
    let mut root = ordinary_root(
        SitemapRootKind::ExplicitConfig,
        0,
        ORIGINAL_A,
        Some(ORIGINAL_A),
    );
    root.admission.admitted_fetch_url = Some(ORIGINAL_B.into());
    named_failure(
        ordinary_append(&mut connection, root),
        "Ordinary admission alias differs from original membership",
    );
    assert_eq!(count(&connection, "sitemap_roots"), 0);
}
#[test]
fn ordinary_xml_location_original_identity_cannot_admit_a_different_task_alias() {
    let mut connection = connection(Default::default());
    let mut value = batch(&format!(
        "<urlset><url><loc>{ORIGINAL_A}</loc></url></urlset>"
    ));
    value.locations[0].admission.admitted_fetch_url = Some(ORIGINAL_B.into());
    named_failure(
        commit(&mut connection, value).map(|_| ()),
        "Ordinary admission alias differs from original membership",
    );
    assert_eq!(count(&connection, "sitemap_locations"), 0);
    assert_eq!(pending_done(&connection), 0);
}
#[test]
fn ordinary_coalesced_admission_cannot_invent_a_policy_reference() {
    let mut connection = connection(Default::default());
    original_config(&connection);
    let mut root = ordinary_root(
        SitemapRootKind::ExplicitConfig,
        0,
        ORIGINAL_A,
        Some(ORIGINAL_A),
    );
    root.admission.decision = SitemapAdmissionDecision::Coalesced;
    root.admission.coalesced_policy_occurrence_id = Some("b".repeat(64));
    named_failure(
        ordinary_append(&mut connection, root),
        "Ordinary admission has a policy coalescing reference",
    );
}
#[test]
fn ordinary_missing_null_and_known_empty_original_capture_cannot_invent_explicit_roots() {
    for capture in [
        serde_json::json!({"sitemap_urls":[ORIGINAL_A]}),
        serde_json::json!({"explicit_sitemap_inputs":null}),
        serde_json::json!({"explicit_sitemap_inputs":[]}),
    ] {
        let mut connection = connection(Default::default());
        save(&connection, "config", &capture).unwrap();
        let expected = if capture["explicit_sitemap_inputs"].is_array() {
            "Explicit root source ordinal/raw differs"
        } else {
            "Original explicit sitemap coordinates unavailable"
        };
        named_failure(
            ordinary_append(
                &mut connection,
                ordinary_root(
                    SitemapRootKind::ExplicitConfig,
                    0,
                    ORIGINAL_A,
                    Some(ORIGINAL_A),
                ),
            ),
            expected,
        );
        assert_eq!(count(&connection, "sitemap_roots"), 0);
    }
}
#[test]
fn ordinary_explicit_out_of_range_original_ordinal_rejects() {
    let mut connection = connection(Default::default());
    original_config(&connection);
    named_failure(
        ordinary_append(
            &mut connection,
            ordinary_root(
                SitemapRootKind::ExplicitConfig,
                1,
                ORIGINAL_A,
                Some(ORIGINAL_A),
            ),
        ),
        "Explicit root source ordinal/raw differs",
    );
}
#[test]
fn ordinary_import_flattened_ordinal_accepts_sparse_actual_input_row_index() {
    let mut connection = connection(Default::default());
    save(&connection, "config", &imported_config()).unwrap();
    ordinary_append(
        &mut connection,
        ordinary_root(SitemapRootKind::ImportedSitemapRow, 1, TASK, Some(TASK)),
    )
    .unwrap();
    assert_eq!(
        load_catalog(&connection)
            .unwrap()
            .unwrap()
            .summary
            .root_rows,
        1
    );
}
#[test]
fn ordinary_import_wrong_raw_normalized_or_flattened_original_coordinates_reject() {
    for control in ["raw", "normalized", "flattened"] {
        let mut connection = connection(Default::default());
        original_config(&connection);
        let mut root = ordinary_root(
            SitemapRootKind::ImportedSitemapRow,
            0,
            ORIGINAL_B,
            Some(ORIGINAL_B),
        );
        let expected = match control {
            "raw" => {
                root.raw_value = ORIGINAL_A.into();
                "Imported root raw/normalized/source index differs"
            }
            "normalized" => {
                root.normalized_document_url = Some(ORIGINAL_A.into());
                root.admission.membership_url = Some(ORIGINAL_A.into());
                root.admission.admitted_fetch_url = Some(ORIGINAL_A.into());
                "Imported root raw/normalized/source index differs"
            }
            "flattened" => {
                root.source_ordinal = 1;
                root.declaration_id = root_declaration_id(&root.kind, &"a".repeat(64), 1);
                "Imported sitemap root ordinal has no original input row"
            }
            _ => unreachable!(),
        };
        named_failure(ordinary_append(&mut connection, root), expected);
    }
}
#[test]
fn ordinary_import_known_rejected_valid_identity_is_recorded_without_admission() {
    let mut connection = connection(Default::default());
    let mut config = imported_config();
    config["input_imports"][1]["rows"][1]["error"] =
        serde_json::json!("declared duplicate or rejection");
    save(&connection, "config", &config).unwrap();
    let mut root = ordinary_root(SitemapRootKind::ImportedSitemapRow, 1, TASK, Some(TASK));
    root.admission.decision = SitemapAdmissionDecision::InvalidLocation;
    root.admission.admitted_fetch_url = None;
    ordinary_append(&mut connection, root).unwrap();
    assert_eq!(
        load_catalog(&connection)
            .unwrap()
            .unwrap()
            .summary
            .root_rows,
        1
    );
}
#[test]
fn ordinary_import_known_error_cannot_be_relabelled_as_scheduled() {
    let mut connection = connection(Default::default());
    let mut config = imported_config();
    config["input_imports"][1]["rows"][1]["error"] = serde_json::json!("original rejected row");
    save(&connection, "config", &config).unwrap();
    named_failure(
        ordinary_append(
            &mut connection,
            ordinary_root(SitemapRootKind::ImportedSitemapRow, 1, TASK, Some(TASK)),
        ),
        "Imported root original error/invalid marker differs from admission",
    );
}
#[test]
fn ordinary_import_original_error_changed_after_append_is_rejected_on_read() {
    let mut connection = connection(Default::default());
    let mut config = imported_config();
    save(&connection, "config", &config).unwrap();
    ordinary_append(
        &mut connection,
        ordinary_root(SitemapRootKind::ImportedSitemapRow, 1, TASK, Some(TASK)),
    )
    .unwrap();
    config["input_imports"][1]["rows"][1]["error"] = serde_json::json!("original rejected row");
    save(&connection, "config", &config).unwrap();
    let error = load_catalog(&connection).unwrap_err();
    assert!(
        format!("{error:#}")
            .contains("Imported root original error/invalid marker differs from admission")
    );
}
#[test]
fn ordinary_import_actual_null_identity_accepts_invalid_fact_and_rejects_invented_url() {
    for invent in [false, true] {
        let mut connection = connection(Default::default());
        let raw = "ftp://example.org/sitemap.xml";
        save(&connection,"config",&serde_json::json!({"input_imports":[{"input_sha256":"b".repeat(64),"rows":[
            {"candidate_kind":"sitemap_document","source_index":9,"raw_value":raw,"normalized_url":null,"error":"unsupported scheme"}]}]})).unwrap();
        let root = ordinary_root(
            SitemapRootKind::ImportedSitemapRow,
            0,
            raw,
            if invent { Some(ORIGINAL_B) } else { None },
        );
        if invent {
            named_failure(
                ordinary_append(&mut connection, root),
                "Imported root raw/normalized/source index differs",
            );
        } else {
            ordinary_append(&mut connection, root).unwrap();
            assert_eq!(
                load_catalog(&connection)
                    .unwrap()
                    .unwrap()
                    .summary
                    .root_rows,
                1
            );
        }
    }
}
#[test]
fn ordinary_explicit_invalid_original_is_recorded_but_cannot_admit_alias() {
    for alias in [false, true] {
        let mut connection = connection(Default::default());
        let raw = "ftp://example.org/sitemap.xml";
        save(
            &connection,
            "config",
            &serde_json::json!({"explicit_sitemap_inputs":[raw]}),
        )
        .unwrap();
        let mut root = ordinary_root(SitemapRootKind::ExplicitConfig, 0, raw, None);
        if alias {
            root.admission.decision = SitemapAdmissionDecision::Scheduled;
            root.admission.admitted_fetch_url = Some(ORIGINAL_B.into());
            named_failure(
                ordinary_append(&mut connection, root),
                "Ordinary admission alias differs from original membership",
            );
        } else {
            ordinary_append(&mut connection, root).unwrap();
            assert_eq!(
                load_catalog(&connection)
                    .unwrap()
                    .unwrap()
                    .summary
                    .root_rows,
                1
            );
        }
    }
}
#[test]
fn ordinary_import_missing_null_field_is_unknown_rather_than_an_actual_null_fact() {
    let mut connection = connection(Default::default());
    let raw = "ftp://example.org/sitemap.xml";
    save(&connection,"config",&serde_json::json!({"input_imports":[{"input_sha256":"b".repeat(64),"rows":[
        {"candidate_kind":"sitemap_document","source_index":9,"raw_value":raw,"error":"unsupported scheme"}]}]})).unwrap();
    named_failure(
        ordinary_append(
            &mut connection,
            ordinary_root(SitemapRootKind::ImportedSitemapRow, 0, raw, None),
        ),
        "Imported root raw/normalized/source index differs",
    );
}
#[test]
fn ordinary_fully_rehashed_original_normalized_membership_and_alias_swap_rejects_read() {
    let mut connection = connection(Default::default());
    original_config(&connection);
    ordinary_append(
        &mut connection,
        ordinary_root(
            SitemapRootKind::ExplicitConfig,
            0,
            ORIGINAL_A,
            Some(ORIGINAL_A),
        ),
    )
    .unwrap();
    assert_eq!(ORIGINAL_A.len(), ORIGINAL_B.len());
    rewrite_root_and_all_catalog_digests(&connection, |root| {
        root.normalized_document_url = Some(ORIGINAL_B.into());
        root.admission.membership_url = Some(ORIGINAL_B.into());
        root.admission.admitted_fetch_url = Some(ORIGINAL_B.into());
    });
    let error = load_catalog(&connection).unwrap_err();
    assert!(format!("{error:#}").contains("Explicit root original normalized identity differs"));
}
#[test]
fn ordinary_fully_rehashed_raw_original_and_alias_swap_rejects_read() {
    let mut connection = connection(Default::default());
    original_config(&connection);
    ordinary_append(
        &mut connection,
        ordinary_root(
            SitemapRootKind::ExplicitConfig,
            0,
            ORIGINAL_A,
            Some(ORIGINAL_A),
        ),
    )
    .unwrap();
    rewrite_root_and_all_catalog_digests(&connection, |root| {
        root.raw_value = ORIGINAL_B.into();
        root.normalized_document_url = Some(ORIGINAL_B.into());
        root.admission.membership_url = Some(ORIGINAL_B.into());
        root.admission.admitted_fetch_url = Some(ORIGINAL_B.into());
    });
    let error = load_catalog(&connection).unwrap_err();
    assert!(format!("{error:#}").contains("Explicit root source ordinal/raw differs"));
}
#[test]
fn ordinary_fully_rehashed_fetch_alias_only_graft_rejects_read() {
    let mut connection = connection(Default::default());
    original_config(&connection);
    ordinary_append(
        &mut connection,
        ordinary_root(
            SitemapRootKind::ExplicitConfig,
            0,
            ORIGINAL_A,
            Some(ORIGINAL_A),
        ),
    )
    .unwrap();
    rewrite_root_and_all_catalog_digests(&connection, |root| {
        root.admission.admitted_fetch_url = Some(ORIGINAL_B.into());
    });
    let error = load_catalog(&connection).unwrap_err();
    assert!(
        format!("{error:#}").contains("Ordinary admission alias differs from original membership")
    );
}
#[test]
fn ordinary_robots_raw_source_evidence_cannot_graft_a_different_original_identity() {
    let mut connection = connection(Default::default());
    let (mut batch, _) = policy_robots_batch(&connection);
    let root = &mut batch.roots[0];
    root.admission.policy_occurrence_id = None;
    root.admission.policy_source_revision = None;
    root.normalized_document_url = Some(ORIGINAL_B.into());
    root.admission.membership_url = Some(ORIGINAL_B.into());
    root.admission.admitted_fetch_url = Some(ORIGINAL_B.into());
    let task = batch.document.task_url.clone();
    let result = (|| -> Result<()> {
        let tx = connection.transaction()?;
        let token = guard_pending_source(&tx, 1, SitemapSourceKind::RobotsDirectives, &task)?;
        append_source(&tx, token, batch)?;
        tx.commit()?;
        Ok(())
    })();
    named_failure(result, "Robots root original normalized identity differs");
    assert_eq!(count(&connection, "sitemap_roots"), 0);
    assert_eq!(
        connection
            .query_row(
                "SELECT done FROM discovery_frontier WHERE kind='robots' AND url=?1",
                [task],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}
#[test]
fn ordinary_selected_inherited_roots_keep_parent_coordinates_without_current_config_grafting() {
    let mut parent = connection(Default::default());
    original_config(&parent);
    ordinary_append(
        &mut parent,
        ordinary_root(
            SitemapRootKind::ExplicitConfig,
            0,
            ORIGINAL_A,
            Some(ORIGINAL_A),
        ),
    )
    .unwrap();
    let catalog = load_catalog(&parent).unwrap().unwrap();
    let parent_bytes = row_payloads(&parent);
    let mut child = empty_connection();
    save(
        &child,
        "config",
        &serde_json::json!({"explicit_sitemap_inputs":[ORIGINAL_B]}),
    )
    .unwrap();
    let parent_sha = selected_anchor(&child, &catalog);
    {
        let tx = child.transaction().unwrap();
        inherit(&tx, &catalog, &parent_sha).unwrap();
        tx.commit().unwrap();
    }
    let inherited = load_catalog(&child).unwrap().unwrap();
    assert_eq!(inherited.rows, catalog.rows);
    assert!(!inherited.inherited.unwrap().current_discovery_executed);
    assert_eq!(row_payloads(&child), parent_bytes);
    assert_eq!(row_payloads(&parent), parent_bytes);
}
