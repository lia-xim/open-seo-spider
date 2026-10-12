//! Bounded export of one SQLite read snapshot, even while a later generation appends.
use anyhow::{Context, Result, ensure};
use open_seo_spider_core::{CrawlStore, PolicyLedgerSummary, export_policy_occurrences_csv};
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
    mut on_start: impl FnMut(&PolicyLedgerSummary) -> Result<()>,
) -> Result<PolicyLedgerSummary> {
    let mut first_page = true;
    let mut count = 0usize;
    let summary = store
        .visit_policy_occurrences(200, |page| {
            let summary = page
                .ledger
                .as_ref()
                .context("This database has no native policy ledger")?;
            if first_page {
                on_start(summary)?;
                if matches!(format, Format::Json) {
                    let mut header = serde_json::to_vec(&serde_json::json!({
                        "engine":summary.engine,"policy_sha256":summary.policy_sha256,
                        "checkpoint":summary,"scope":"sqlite_read_snapshot",
                    }))?;
                    header.pop();
                    writer.write_all(&header)?;
                    writer.write_all(b",\"observations\":[")?;
                }
            }
            match format {
                Format::Json => {
                    for row in &page.rows {
                        if count > 0 {
                            writer.write_all(b",")?;
                        }
                        serde_json::to_writer(&mut *writer, row)?;
                        count += 1;
                    }
                }
                Format::Csv => {
                    let csv = export_policy_occurrences_csv(&page.rows)?;
                    let bytes = if first_page {
                        csv.as_bytes()
                    } else {
                        let header_end = csv.find('\n').context("Missing policy CSV header")?;
                        &csv.as_bytes()[header_end + 1..]
                    };
                    writer.write_all(bytes)?;
                    count += page.rows.len();
                }
            }
            first_page = false;
            Ok(())
        })?
        .context("This database has no native policy ledger")?;
    ensure!(
        count == summary.occurrences,
        "Export row count differs from snapshot ledger"
    );
    if matches!(format, Format::Json) {
        writer.write_all(b"],\"export\":")?;
        serde_json::to_writer(
            &mut *writer,
            &serde_json::json!({
                "row_count":count,"snapshot_revision":summary.revision,
                "all_snapshot_rows_exported":true,"ledger_complete":summary.complete,
                "inherited_history_complete":summary.inherited_history.as_ref().map(|history|history.complete),
            }),
        )?;
        writer.write_all(b"}")?;
    }
    writer.flush()?;
    Ok(summary)
}
