//! Private full saved-page traversal over one actual prepared read transaction.
//! Legacy PAGE_COLUMNS, target DTO and shared context/hash profiles stay intact.
use super::window::{MembershipBasis, VerifiedAnalysisRead};
use super::*;
use crate::sitemap_findings::{
    SitemapFindingDomain, SitemapFindingPageCandidate, SitemapFindingPosition, SitemapFindingsError,
};
use crate::sitemap_ledger::verified_read::VerifiedRawRead;
use std::ops::ControlFlow;

const BATCH_PAGES: usize = 200;
const FINDING_TAIL_COLUMNS: &str = "json_type(p.value,'$.robots_warning'),json_extract(p.value,'$.robots_warning'),json_type(p.value,'$.pdf'),json_type(p.value,'$.pdf.status'),json_extract(p.value,'$.pdf.status')";

struct ExtraPageFacts {
    content_type: String,
    robots_warning_field_observed: bool,
    robots_warning: Option<String>,
    pdf_field_observed: bool,
    pdf_record_present: bool,
    pdf_status: Option<crate::ResourceAnalysisStatus>,
}
fn extra_facts(row: &rusqlite::Row<'_>) -> Result<ExtraPageFacts> {
    let content_type: String = row.get(18)?;
    let warning_kind: Option<String> = row.get(21)?;
    let robots_warning: Option<String> = row.get(22)?;
    ensure!(
        matches!(warning_kind.as_deref(), None | Some("null" | "text"))
            && robots_warning
                .as_ref()
                .is_none_or(|v| v.len() <= MAX_FIELD_BYTES),
        "Invalid saved robots warning field"
    );
    let pdf_kind: Option<String> = row.get(23)?;
    ensure!(
        matches!(pdf_kind.as_deref(), None | Some("null" | "object")),
        "Invalid saved PDF record field"
    );
    let status_kind: Option<String> = row.get(24)?;
    ensure!(
        matches!(status_kind.as_deref(), None | Some("null" | "text")),
        "Invalid saved PDF status field"
    );
    let pdf_status = match row.get::<_, Option<String>>(25)? {
        None => None,
        Some(value) => {
            ensure!(value.len() <= 32, "Invalid saved PDF status field");
            Some(serde_json::from_value(serde_json::Value::String(value))?)
        }
    };
    Ok(ExtraPageFacts {
        content_type,
        robots_warning_field_observed: warning_kind.is_some(),
        robots_warning,
        pdf_field_observed: pdf_kind.is_some(),
        pdf_record_present: pdf_kind.as_deref() == Some("object"),
        pdf_status,
    })
}

/// Root has compared the complete findings context before this private call.
/// True means the candidate cursor was exhausted; false means the visitor broke.
/// Neither value invents total matches or completeness of a work-limited scan.
pub(crate) fn visit_saved_page_candidates(
    prepared: &VerifiedAnalysisRead<'_, '_>,
    raw: Option<&VerifiedRawRead<'_>>,
    coverage: &crate::SitemapCoverage,
    control: &SitemapReadControl,
    after: Option<&SitemapFindingPosition>,
    mut visitor: impl FnMut(
        SitemapFindingPosition,
        SitemapFindingPageCandidate,
    ) -> Result<ControlFlow<()>>,
) -> Result<bool> {
    let (connection, actual_raw, actual_coverage, context) = prepared.findings_basis();
    control.checkpoint()?;
    context.control.checkpoint()?;
    ensure!(
        !connection.is_autocommit(),
        "Analysis read transaction ended"
    );
    ensure!(
        match (actual_raw, raw) {
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            (None, None) => true,
            _ => false,
        },
        "Analysis raw proof changed"
    );
    ensure!(
        actual_coverage == coverage,
        "Analysis coverage changed after preparation"
    );
    if !context.base.context.available {
        return Err(SitemapFindingsError::Unavailable.into());
    }
    let raw = raw.context("Saved page membership has no verified raw basis")?;
    let generation = context
        .generation
        .context("Available analysis has no generation")?;
    let after = match after {
        None => None,
        Some(position) => {
            position.validate(SitemapFindingDomain::SavedPage)?;
            let SitemapFindingPosition::SavedPage { ordinal } = position else {
                return Err(SitemapFindingsError::InvalidContinuation.into());
            };
            Some(i64::try_from(*ordinal)?)
        }
    };
    let _binding = control.bind_connection(connection)?;
    context.control.run(|| control.run(|| {
        if let Some(after) = after {
            control.checkpoint()?;
            let exists: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM metadata m,json_each(m.value,'$.pages') p WHERE m.key='report' AND CAST(p.key AS INTEGER)=?1)",
                [after], |row| row.get(0),
            )?;
            if !exists {
                return Err(SitemapFindingsError::InvalidContinuation.into());
            }
        }
        // One candidate statement survives all batches. Aggregate queries use
        // the same readonly Connection while this cursor remains positioned.
        let sql = format!(
            "SELECT {},{} FROM metadata m,json_each(m.value,'$.pages') p WHERE m.key='report' AND (?1 IS NULL OR CAST(p.key AS INTEGER)>?1) ORDER BY CAST(p.key AS INTEGER)",
            page_columns(), FINDING_TAIL_COLUMNS,
        );
        let mut statement = connection.prepare(&sql)?;
        let mut rows = statement.query([after])?;
        let mut previous = after;
        loop {
            control.checkpoint()?;
            context.control.checkpoint()?;
            let mut batch = Vec::with_capacity(BATCH_PAGES);
            while batch.len() < BATCH_PAGES {
                control.checkpoint()?;
                let Some(row) = rows.next()? else { break };
                let page = page_from_row(row, context.selected, generation)?;
                let ordinal = i64::try_from(page.report_page_ordinal)?;
                ensure!(
                    ordinal == previous.map_or(0, |previous| previous + 1),
                    "Saved finding candidate ordinals differ"
                );
                previous = Some(ordinal);
                batch.push((page, extra_facts(row)?));
            }
            if batch.is_empty() {
                let count = context.base.context.saved_page_count
                    .context("Available analysis has no saved page count")?;
                ensure!(
                    previous.map_or(0, |v| v as u64 + 1) == count,
                    "Saved finding candidate count differs"
                );
                return Ok(true);
            }
            let (pages, extras): (Vec<_>, Vec<_>) = batch.into_iter().unzip();
            let wanted = pages.iter().map(|page| page.requested_url.clone()).collect();
            let inlinks = read_inlinks(connection, &wanted, control)?;
            let members = MembershipBasis::Verified(Some(raw)).counts(connection, &pages, control)?;
            let findings = build_saved_page_findings(pages, members, &inlinks, coverage, control)?;
            ensure!(findings.len() == extras.len(), "Saved finding batch differs");
            for (finding, facts) in findings.into_iter().zip(extras) {
                control.checkpoint()?;
                context.control.checkpoint()?;
                let position = SitemapFindingPosition::SavedPage {
                    ordinal: finding.observation.report_page_ordinal,
                };
                let candidate = SitemapFindingPageCandidate {
                    finding,
                    content_type: facts.content_type,
                    robots_warning_field_observed: facts.robots_warning_field_observed,
                    robots_warning: facts.robots_warning,
                    pdf_field_observed: facts.pdf_field_observed,
                    pdf_record_present: facts.pdf_record_present,
                    pdf_status: facts.pdf_status,
                };
                if visitor(position, candidate)?.is_break() {
                    return Ok(false);
                }
            }
        }
    }))
}

#[cfg(test)]
#[path = "findings_tests.rs"]
mod tests;
