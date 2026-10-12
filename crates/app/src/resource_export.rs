//! One pinned SQLite decision snapshot, streamed without collecting its full ledger.
use anyhow::{Context, Result, ensure};
use open_seo_spider_core::{
    CrawlStore, ResourceLedgerSummary, ResourceReadSnapshot, StoredResourceOccurrence,
};
use std::io::Write;

#[derive(Clone, Copy)]
pub enum Format {
    Json,
    Csv,
}

pub fn write_snapshot(
    store: &CrawlStore,
    format: Format,
    writer: &mut impl Write,
    mut on_start: impl FnMut(&ResourceLedgerSummary, &ResourceReadSnapshot) -> Result<()>,
) -> Result<ResourceLedgerSummary> {
    let mut first = true;
    let mut count = 0usize;
    let summary = store
        .visit_resource_occurrences(200, |page| {
            let summary = page.ledger.as_ref().context("This database has no native resource ledger")?;
            let snapshot=page.snapshot.as_ref().context("Resource ledger has no saved full crawl read-snapshot binding")?;
            if first {
                on_start(summary,snapshot)?;
                match format {
                    Format::Json => {
                        let mut header = serde_json::to_vec(&serde_json::json!({
                            "enabled":true,"engine":summary.engine,"checkpoint":summary,"snapshot":snapshot,
                            "ledger_available":true,"snapshot_scope":"sqlite_read_snapshot",
                            "website_requests":0,"entity_bodies_included":false,
                        }))?;
                        header.pop();
                        writer.write_all(&header)?;
                        writer.write_all(b",\"observations\":[")?;
                    }
                    Format::Csv => write_csv_record(writer, &[
                        "sequence", "generation", "basis", "evaluation_revision", "occurrence_id",
                        "source_revision", "source_unit_revision", "source_complete", "source_ordinal", "requested_source_url", "source_document_url",
                        "reference_kind", "classification_basis", "raw_value", "decoded_value", "resolved_url",
                        "declared_mime", "dom_location", "byte_start", "byte_end", "nofollow",
                        "observation_error", "decision_reason", "coalesced_into", "grant_json", "attributes_json",
                        "complete_observation_json",
                    ])?,
                }
            }
            for row in &page.rows {
                match format {
                    Format::Json => {
                        if count > 0 { writer.write_all(b",")?; }
                        serde_json::to_writer(&mut *writer, row)?;
                    }
                    Format::Csv => write_csv_row(writer, row)?,
                }
                count = count.checked_add(1).context("Resource export count overflow")?;
            }
            first = false;
            Ok(())
        })?
        .context("This database has no native resource ledger")?;
    ensure!(
        count == summary.decision_rows,
        "Export row count differs from snapshot resource ledger"
    );
    if matches!(format, Format::Json) {
        writer.write_all(b"],\"export\":")?;
        serde_json::to_writer(
            &mut *writer,
            &serde_json::json!({
                "row_count":count,"snapshot_revision":summary.revision,
                "all_snapshot_rows_exported":true,"discovery_complete":summary.complete,
                "inherited_history_rows_available":summary.inherited_history.as_ref().map(|history| history.rows_available),
            }),
        )?;
        writer.write_all(b"}")?;
    }
    writer.flush()?;
    Ok(summary)
}

fn write_csv_row(writer: &mut impl Write, row: &StoredResourceOccurrence) -> Result<()> {
    let occurrence = &row.occurrence;
    let mut fields = [
        row.sequence.to_string(),
        row.generation.to_string(),
        enum_name(&row.basis)?,
        row.evaluation_revision.clone(),
        occurrence.occurrence_id.clone(),
        occurrence.source_revision.clone(),
        occurrence.source_unit_revision.clone(),
        occurrence.source_complete.to_string(),
        occurrence.source_ordinal.to_string(),
        occurrence.requested_source_url.clone(),
        occurrence.source_document_url.clone(),
        enum_name(&occurrence.reference_kind)?,
        occurrence.classification_basis.clone(),
        occurrence.raw_value.clone(),
        occurrence.decoded_value.clone().unwrap_or_default(),
        occurrence.resolved_url.clone().unwrap_or_default(),
        occurrence.declared_mime.clone().unwrap_or_default(),
        occurrence.dom_location.clone().unwrap_or_default(),
        occurrence
            .byte_start
            .map(|v| v.to_string())
            .unwrap_or_default(),
        occurrence
            .byte_end
            .map(|v| v.to_string())
            .unwrap_or_default(),
        occurrence.nofollow.to_string(),
        occurrence.observation_error.clone().unwrap_or_default(),
        row.decision.reason.clone(),
        row.decision.coalesced_into.clone().unwrap_or_default(),
        serde_json::to_string(&row.decision.grant)?,
        serde_json::to_string(&occurrence.attributes)?,
        serde_json::to_string(row)?,
    ];
    // Numeric and boolean fields remain their typed values. The complete JSON
    // cell retains exact original text alongside protected spreadsheet cells.
    for (index, field) in fields.iter_mut().enumerate() {
        if !matches!(index, 0 | 1 | 7 | 8 | 18 | 19 | 20) {
            *field = open_seo_spider_core::csv_text(field);
        }
    }
    write_csv_record(
        writer,
        &fields.iter().map(String::as_str).collect::<Vec<_>>(),
    )
}
fn enum_name(value: &impl serde::Serialize) -> Result<String> {
    serde_json::to_value(value)?
        .as_str()
        .map(str::to_owned)
        .context("Expected resource enum string")
}
fn write_csv_record(writer: &mut impl Write, fields: &[&str]) -> Result<()> {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            writer.write_all(b",")?;
        }
        writer.write_all(b"\"")?;
        for (index, piece) in field.split('"').enumerate() {
            if index > 0 {
                writer.write_all(b"\"\"")?;
            }
            writer.write_all(piece.as_bytes())?;
        }
        writer.write_all(b"\"")?;
    }
    writer.write_all(b"\r\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_retains_quotes_newlines_unicode_and_empty_fields() {
        let mut output = Vec::new();
        write_csv_record(&mut output, &["", "a,b", "c\"d", "ä\n日"]).unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "\"\",\"a,b\",\"c\"\"d\",\"ä\n日\"\r\n"
        );
    }
    #[test]
    fn observed_formula_text_is_protected_without_changing_original_json() {
        let row:StoredResourceOccurrence=serde_json::from_value(serde_json::json!({
            "sequence":1,"generation":2,"basis":"current","evaluation_revision":"own-evaluation",
            "occurrence":{"occurrence_id":"own-occurrence","source_revision":"own-source","source_unit_revision":"own-unit","source_complete":true,
                "source_ordinal":0,"requested_source_url":"https://example.test/","source_document_url":"https://example.test/",
                "raw_value":"=1+1","decoded_value":"\t@SUM(1,2)","resolved_url":null,"reference_kind":"image",
                "classification_basis":"own-control","declared_mime":null,"dom_location":"\u{feff}-1+2","byte_start":0,"byte_end":4,
                "attributes":{"quoted":"\"line\n=FORMULA"},"nofollow":false,"observation_error":null},
            "decision":{"reason":"not_http","grant":null,"coalesced_into":null}
        })).unwrap();
        let before = serde_json::to_value(&row).unwrap();
        let mut output = Vec::new();
        write_csv_row(&mut output, &row).unwrap();
        let csv = String::from_utf8(output).unwrap();
        assert!(csv.contains("\"'=1+1\""));
        assert!(csv.contains("\"'\t@SUM(1,2)\""));
        assert!(csv.contains("\"'\u{feff}-1+2\""));
        assert!(csv.starts_with("\"1\",\"2\",\"current\""));
        assert_eq!(serde_json::to_value(&row).unwrap(), before);
        assert!(csv.contains("\"\"raw_value\"\":\"\"=1+1\"\""));
    }
}
