//! Work candidate: original source facts and final admission are distinct.
//! The store owns the pending-source guard, policy commit and outer transaction.
use crate::sitemap_ledger::{
    SitemapAdmissionBinding, SitemapAdmissionDecision as D, SitemapDiscoveryBatch,
    SitemapRobotsDeclarationFacts, SitemapRootDeclaration, SitemapRootKind, SitemapSourceDocument,
    SitemapSourceKind, SitemapTransportFacts,
};
use anyhow::{Context, Result, ensure};

pub(crate) fn xml_batch(
    task_url: &str,
    generation: u64,
    revision: &str,
    transport: SitemapTransportFacts,
    decode: Option<&crate::SitemapEntityDecode>,
    parser: Option<crate::SitemapParseObservation>,
    failure_code: Option<String>,
) -> Result<SitemapDiscoveryBatch> {
    let document = SitemapSourceDocument {
        sequence: 0,
        observation_id: crate::sitemap_ledger::source_observation_id(
            SitemapSourceKind::SitemapXml,
            task_url,
            generation,
            revision,
        )?,
        kind: SitemapSourceKind::SitemapXml,
        task_url: task_url.into(),
        generation,
        source_revision: revision.into(),
        transport: Some(transport),
        entity_decode: decode.map(Into::into),
        parser: None,
        declaration_scan_complete: false,
        declaration_utf8_lossy: None,
        declaration_count: None,
        retained_rows_complete: true,
        failure_code,
    };
    match parser {
        Some(parser) => SitemapDiscoveryBatch::from_parser(document, parser),
        None => Ok(SitemapDiscoveryBatch {
            document,
            roots: vec![],
            entries: vec![],
            locations: vec![],
        }),
    }
}

pub(crate) fn robots_batch(
    task_url: &str,
    generation: u64,
    revision: &str,
    source: &crate::RobotsSitemapSource,
) -> Result<SitemapDiscoveryBatch> {
    let observation_id = crate::sitemap_ledger::source_observation_id(
        SitemapSourceKind::RobotsDirectives,
        task_url,
        generation,
        revision,
    )?;
    let document = SitemapSourceDocument {
        sequence: 0,
        observation_id: observation_id.clone(),
        kind: SitemapSourceKind::RobotsDirectives,
        task_url: task_url.into(),
        generation,
        source_revision: revision.into(),
        transport: Some(SitemapTransportFacts {
            request_started: source.request_started,
            requested_url: Some(source.requested_url.clone()),
            final_url: source.final_url.clone(),
            // RobotsCache currently records only the terminal status; do not
            // infer the initial status when its internal client followed redirects.
            initial_status: None,
            final_status: source.status,
            blocked_by_robots: false,
            body_complete: source.body_complete,
            post_http_decoding_bytes: u64::try_from(source.retained_post_http_decoding_bytes)?,
            post_http_decoding_sha256: source
                .body_complete
                .then(|| source.retained_prefix_sha256.clone()),
            retained_post_http_prefix_sha256: (!source.retained_prefix_sha256.is_empty())
                .then(|| source.retained_prefix_sha256.clone()),
            failure_code: source.failure.clone(),
        }),
        entity_decode: None,
        parser: None,
        declaration_scan_complete: source.declaration_scan_complete,
        declaration_utf8_lossy: source.declaration_utf8_lossy,
        declaration_count: Some(source.declarations.len()),
        retained_rows_complete: true,
        failure_code: source
            .failure
            .clone()
            .or_else(|| (!source.complete).then(|| "robots_declaration_source_incomplete".into())),
    };
    let complete = document.complete();
    let roots = source
        .declarations
        .iter()
        .map(|row| {
            let ordinal = u64::try_from(row.ordinal)?;
            let url = strict_robots_location(&row.raw_value);
            Ok(SitemapRootDeclaration {
                sequence: 0,
                declaration_id: crate::sitemap_ledger::root_declaration_id(
                    &SitemapRootKind::RobotsDirective,
                    &observation_id,
                    ordinal,
                ),
                kind: SitemapRootKind::RobotsDirective,
                source_observation_id: Some(observation_id.clone()),
                source_ordinal: ordinal,
                raw_line: Some(row.raw_line.clone()),
                raw_value: row.raw_value.clone(),
                normalized_document_url: url.clone(),
                robots_evidence: Some(SitemapRobotsDeclarationFacts {
                    line_ordinal: row.line_ordinal,
                    byte_start: row.byte_start,
                    byte_end: row.byte_end,
                    line_complete: row.line_complete,
                    utf8_lossy: row.utf8_lossy,
                    legacy_candidate_url: row.legacy_candidate_url.clone(),
                }),
                admission: empty_binding(
                    url,
                    if !complete {
                        D::IncompleteSource
                    } else if strict_robots_location(&row.raw_value).is_none() {
                        D::InvalidLocation
                    } else {
                        D::Unbound
                    },
                ),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(SitemapDiscoveryBatch {
        document,
        roots,
        entries: vec![],
        locations: vec![],
    })
}

/// This new scheduling profile accepts an absolute HTTP(S) declaration, with
/// only XML/HTTP ASCII whitespace trimming. Raw line/value and the old lenient
/// comment/base projection remain evidence; empty/relative/control values do
/// not turn into an invented origin-root sitemap request.
pub(crate) fn strict_robots_location(raw: &str) -> Option<String> {
    crate::sitemap_ledger::original_robots_document_url(raw)
}

pub(crate) fn empty_binding(url: Option<String>, decision: D) -> SitemapAdmissionBinding {
    SitemapAdmissionBinding {
        decision,
        membership_url: url,
        admitted_fetch_url: None,
        coalesced_policy_occurrence_id: None,
        policy_occurrence_id: None,
        policy_source_revision: None,
    }
}

pub(crate) struct RootPolicyContext<'a> {
    pub source_revision: &'a str,
    pub source_ordinal: usize,
    pub generation: u64,
    pub source_url: Option<&'a str>,
}

pub(crate) fn finalize_root_policy(
    transaction: &rusqlite::Transaction<'_>,
    root: &mut SitemapRootDeclaration,
    row: &crate::PolicyOccurrence,
    expected: &RootPolicyContext<'_>,
) -> Result<()> {
    use rusqlite::OptionalExtension;
    let id = &row.decision.occurrence_id;
    let bytes: i64 = transaction
        .query_row(
            "SELECT length(CAST(data AS BLOB)) FROM policy_occurrences WHERE occurrence_id=?1",
            [id],
            |sql| sql.get(0),
        )
        .optional()?
        .context("Root policy row was not committed in this transaction")?;
    ensure!(
        (0..=1024 * 1024).contains(&bytes),
        "Root policy SQL payload byte budget"
    );
    let (sequence, generation, data): (i64, i64, String) = transaction.query_row(
        "SELECT sequence,generation,data FROM policy_occurrences WHERE occurrence_id=?1",
        [id],
        |sql| Ok((sql.get(0)?, sql.get(1)?, sql.get(2)?)),
    )?;
    let committed: crate::StoredPolicyOccurrence = serde_json::from_str(&data)?;
    ensure!(
        committed.sequence == u64::try_from(sequence)?
            && committed.generation == u64::try_from(generation)?
            && committed.generation == expected.generation
            && committed.occurrence == *row
            && committed.occurrence.decision.occurrence_id == *id,
        "Root policy input differs from actual committed SQL occurrence/index/generation"
    );
    ensure!(
        row.source_revision == expected.source_revision
            && row.source_ordinal == expected.source_ordinal
            && row.decision.source_url.as_deref() == expected.source_url,
        "Root policy source identity differs from original input occurrence"
    );
    ensure!(
        row.task_role == crate::PolicyTaskRole::SitemapDocument
            && row.decision.source_kind == crate::CandidateSourceKind::SitemapDocument
            && row.input_basis == crate::PolicyInputBasis::DeclaredInput
            && row.raw_value_observed
            && row.decision.raw_value == root.raw_value,
        "Root policy input does not match its original raw protocol declaration"
    );
    ensure!(
        row.admission_reason != "pending_admission",
        "Root policy decision is unfinished"
    );
    ensure!(
        root.admission.decision != D::IncompleteSource,
        "Incomplete declaration source cannot acquire a policy task"
    );
    let decision = match row.admission_reason.as_str() {
        "scheduled" => D::Scheduled,
        "coalesced" => D::Coalesced,
        "source_check_only" => D::SourceCheckOnly,
        _ if root.normalized_document_url.is_none() => D::InvalidLocation,
        _ => D::PolicyDenied,
    };
    let admitted = match decision {
        D::Scheduled => {
            ensure!(
                row.decision.policy_complete && root.normalized_document_url.is_some(),
                "Unaccepted/incomplete policy declaration cannot schedule"
            );
            Some(
                row.scheduled_fetch_url
                    .clone()
                    .context("Missing scheduled root alias")?,
            )
        }
        D::Coalesced => {
            ensure!(
                row.decision.policy_complete
                    && root.normalized_document_url.is_some()
                    && row.coalesced_task_occurrence_id.is_some(),
                "Unaccepted/incomplete policy declaration cannot coalesce"
            );
            Some(
                row.decision
                    .fetch_candidate_url
                    .clone()
                    .context("Missing coalesced root alias")?,
            )
        }
        _ => None,
    };
    root.admission = SitemapAdmissionBinding {
        decision,
        membership_url: root.normalized_document_url.clone(),
        admitted_fetch_url: admitted,
        coalesced_policy_occurrence_id: row.coalesced_task_occurrence_id.clone(),
        policy_occurrence_id: Some(row.decision.occurrence_id.clone()),
        policy_source_revision: Some(row.source_revision.clone()),
    };
    Ok(())
}

#[cfg(test)]
#[path = "sitemap_discovery_tests.rs"]
mod tests;
