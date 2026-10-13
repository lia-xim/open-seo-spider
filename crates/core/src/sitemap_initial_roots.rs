//! Root Work candidate: original declaration coordinates are independent of task aliases.
use crate::sitemap_ledger::{
    SitemapAdmissionDecision as D, SitemapRootDeclaration, SitemapRootKind,
};
use anyhow::{Result, ensure};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub(crate) struct InitialSitemapRoot {
    pub root: SitemapRootDeclaration,
    pub policy_source_revision: Option<String>,
    pub policy_source_ordinal: usize,
}

pub(crate) fn initial_roots(
    explicit_originals: Option<&[String]>,
    imports: &[crate::ListImportResult],
    saved_config_sha256: &str,
    policy_sha256: Option<&str>,
) -> Result<Option<Vec<InitialSitemapRoot>>> {
    // None is unknown legacy provenance. No reconstruction from merged URLs.
    let Some(explicit) = explicit_originals else {
        return Ok(None);
    };
    ensure!(
        saved_config_sha256.len() == 64
            && saved_config_sha256
                .bytes()
                .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v)),
        "Initial roots require actual saved config SHA"
    );
    if let Some(sha) = policy_sha256 {
        ensure!(
            sha.len() == 64
                && sha
                    .bytes()
                    .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v)),
            "Initial roots require compiled policy SHA"
        );
    }
    let mut roots = Vec::new();
    for (ordinal, raw) in explicit.iter().enumerate() {
        let normalized = crate::normalize_url(raw, None).map(|url| url.to_string());
        roots.push(InitialSitemapRoot {
            root: declaration(
                SitemapRootKind::ExplicitConfig,
                saved_config_sha256,
                ordinal,
                raw.clone(),
                normalized,
            ),
            policy_source_revision: policy_sha256
                .map(|sha| format!("initial:{sha}:explicit:sitemap_document")),
            policy_source_ordinal: ordinal,
        });
    }
    let mut flattened = 0usize;
    for (input_index, input) in imports.iter().enumerate() {
        crate::validate_import_result(input)?;
        for row in input
            .rows
            .iter()
            .filter(|row| row.candidate_kind == crate::ListCandidateKind::SitemapDocument)
        {
            let mut root = declaration(
                SitemapRootKind::ImportedSitemapRow,
                saved_config_sha256,
                flattened,
                row.raw_value.clone(),
                row.normalized_url.clone(),
            );
            // Preserve the importer's known original identity even if its row was rejected.
            if row.error.is_some() {
                root.admission.decision = D::InvalidLocation;
            }
            roots.push(InitialSitemapRoot {
                root,
                policy_source_revision: policy_sha256
                    .map(|sha| format!("initial:{sha}:{input_index}:{}", input.input_sha256)),
                policy_source_ordinal: row.source_index,
            });
            flattened = flattened
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("Initial imported root ordinal exhausted"))?;
        }
    }
    Ok(Some(roots))
}

fn declaration(
    kind: SitemapRootKind,
    basis: &str,
    ordinal: usize,
    raw: String,
    normalized: Option<String>,
) -> SitemapRootDeclaration {
    SitemapRootDeclaration {
        sequence: 0,
        declaration_id: crate::sitemap_ledger::root_declaration_id(&kind, basis, ordinal as u64),
        kind,
        source_observation_id: None,
        source_ordinal: ordinal as u64,
        raw_line: None,
        raw_value: raw,
        normalized_document_url: normalized.clone(),
        robots_evidence: None,
        admission: crate::sitemap_discovery::empty_binding(
            normalized.clone(),
            if normalized.is_some() {
                D::Unbound
            } else {
                D::InvalidLocation
            },
        ),
    }
}

/// Ordinary roots describe actual initialized document task identities, not a page frontier.
/// This is called in the Store initializer; its actual enqueue occurs in the same TX.
pub(crate) fn finalize_ordinary_roots(
    transaction: &rusqlite::Transaction<'_>,
    roots: &mut [InitialSitemapRoot],
    declared_sitemap_tasks: &[String],
) -> Result<()> {
    // Validate every original coordinate against authoritative stored input
    // before any caller-visible admission mutation. Existing task B is not
    // evidence that the original declaration A described B.
    for item in roots.iter() {
        crate::sitemap_ledger::verify_original_root_coordinates(transaction, &item.root)?;
    }
    let tasks = declared_sitemap_tasks
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    for item in roots {
        ensure!(
            item.policy_source_revision.is_none(),
            "Ordinary initial roots cannot carry a policy context"
        );
        if item.root.admission.decision != D::Unbound {
            continue;
        }
        let normalized = item
            .root
            .normalized_document_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Valid initial root lacks original identity"))?;
        ensure!(
            tasks.contains(normalized.as_str()),
            "Original initial root has no actual declared document task"
        );
        item.root.admission.decision = if seen.insert(normalized.clone()) {
            D::Scheduled
        } else {
            D::Coalesced
        };
        item.root.admission.admitted_fetch_url = Some(normalized.clone());
    }
    Ok(())
}

/// The Root expected coordinates come from original config/import values above.
/// The actual SQL row is read after initialize_policy committed it; lookup failure is fatal.
pub(crate) fn finalize_policy_roots(
    transaction: &rusqlite::Transaction<'_>,
    roots: &mut [InitialSitemapRoot],
    run_generation: u64,
) -> Result<()> {
    let run_bytes: i64 = transaction.query_row(
        "SELECT length(CAST(value AS BLOB)) FROM metadata WHERE key='run_state'",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        (0..=65536).contains(&run_bytes),
        "Initial root run state byte budget"
    );
    let run_data: String = transaction.query_row(
        "SELECT value FROM metadata WHERE key='run_state'",
        [],
        |row| row.get(0),
    )?;
    let run: crate::CrawlRunState = serde_json::from_str(&run_data)?;
    ensure!(
        run.generation == run_generation && run.outcome.is_none(),
        "Stale/terminal initial root generation"
    );
    for item in roots {
        let revision = item.policy_source_revision.as_deref().ok_or_else(|| {
            anyhow::anyhow!("Policy initial root lacks original revision context")
        })?;
        let occurrence_id = crate::policy_ledger::digest(format!(
            "{revision}:{}:sitemap_document",
            item.policy_source_ordinal
        ));
        let bytes: i64 = transaction.query_row(
            "SELECT length(CAST(data AS BLOB)) FROM policy_occurrences WHERE occurrence_id=?1",
            [&occurrence_id],
            |row| row.get(0),
        )?;
        ensure!(
            (0..=1024 * 1024).contains(&bytes),
            "Initial root policy row byte budget"
        );
        let (sequence, generation, data): (i64, i64, String) = transaction.query_row(
            "SELECT sequence,generation,data FROM policy_occurrences WHERE occurrence_id=?1",
            [&occurrence_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let stored: crate::StoredPolicyOccurrence = serde_json::from_str(&data)?;
        ensure!(
            stored.sequence == u64::try_from(sequence)?
                && stored.generation == u64::try_from(generation)?
                && stored.generation > 0
                && stored.generation <= run_generation,
            "Initial root policy creation generation/index differs from actual committed row"
        );
        crate::sitemap_discovery::finalize_root_policy(
            transaction,
            &mut item.root,
            &stored.occurrence,
            &crate::sitemap_discovery::RootPolicyContext {
                source_revision: revision,
                source_ordinal: item.policy_source_ordinal,
                generation: stored.generation,
                source_url: None,
            },
        )?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "sitemap_initial_roots_tests.rs"]
mod tests;
