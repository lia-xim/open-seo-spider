//! Native admission evidence. Observed targets and operator fetch aliases stay distinct.
use crate::{
    CandidateDecision, CandidateInput, CandidateSourceKind, CompiledCrawlPolicy,
    CrawlPolicyDecisionKind, PageRecord,
};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const LIVE_CRAWL_POLICY_ENGINE: &str = "open-seo-spider/native-crawl-policy-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct LiveCrawlPolicyLimits {
    pub max_row_bytes: usize,
    pub max_batch_bytes: usize,
    pub max_batch_occurrences: usize,
    pub max_ledger_bytes: Option<usize>,
    pub max_page_bytes: usize,
}
impl Default for LiveCrawlPolicyLimits {
    fn default() -> Self {
        Self {
            max_row_bytes: 262144,
            max_batch_bytes: 33554432,
            max_batch_occurrences: 100000,
            max_ledger_bytes: None,
            max_page_bytes: 1048576,
        }
    }
}
impl LiveCrawlPolicyLimits {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1024..=4 * 1024 * 1024).contains(&self.max_row_bytes),
            "Policy row limit must be 1024..=4194304 bytes"
        );
        ensure!(
            self.max_batch_bytes >= self.max_row_bytes && self.max_batch_bytes <= 256 * 1024 * 1024,
            "Policy batch byte limit must include one row and be at most 256 MiB"
        );
        ensure!(
            (1..=1000000).contains(&self.max_batch_occurrences),
            "Policy source batch limit must be 1..=1000000 occurrences"
        );
        ensure!(
            self.max_ledger_bytes != Some(0),
            "Policy ledger byte limit must be positive or absent"
        );
        ensure!(
            self.max_page_bytes >= self.max_row_bytes && self.max_page_bytes <= 16 * 1024 * 1024,
            "Policy API page limit must include one row and be at most 16 MiB"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyTaskRole {
    PageDocument,
    SitemapDocument,
    ObservationOnly,
}
impl PolicyTaskRole {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::PageDocument => "page_document",
            Self::SitemapDocument => "sitemap_document",
            Self::ObservationOnly => "observation_only",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyAdmission {
    pub engine: String,
    pub policy_sha256: String,
    pub occurrence_id: String,
    pub fetch_url: String,
    pub source_kind: CandidateSourceKind,
    pub task_role: PolicyTaskRole,
    pub can_expand: bool,
    pub can_discover_xml: bool,
    /// A child may reuse an attested page admission without having all parent ledger rows.
    pub inherited: bool,
}
impl PolicyAdmission {
    pub(crate) fn validate(&self, sha: &str, url: &str, role: PolicyTaskRole) -> Result<()> {
        ensure!(
            self.engine == LIVE_CRAWL_POLICY_ENGINE && self.policy_sha256 == sha,
            "Unknown or mismatched saved native policy admission"
        );
        ensure!(
            self.fetch_url == url && self.task_role == role && !self.occurrence_id.is_empty(),
            "Saved policy admission identity/role differs"
        );
        ensure!(
            crate::normalize_url(url, None).is_some_and(|value| value.as_str() == url),
            "Unsafe saved policy fetch identity"
        );
        ensure!(
            !self.can_discover_xml || role == PolicyTaskRole::SitemapDocument,
            "XML permission on a page task"
        );
        ensure!(
            !self.can_expand || role == PolicyTaskRole::PageDocument,
            "Page traversal on a protocol task"
        );
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyInputBasis {
    ObservedAttribute,
    ParserObservedXml,
    PdfAnnotationUri,
    DeclaredInput,
    ResolvedLegacyFallback,
    ObservedRedirectLocation,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyOccurrence {
    pub source_revision: String,
    pub source_ordinal: usize,
    pub input_basis: PolicyInputBasis,
    pub raw_value_observed: bool,
    pub decision: CandidateDecision,
    pub task_role: PolicyTaskRole,
    pub scheduled_fetch_url: Option<String>,
    pub coalesced_task_occurrence_id: Option<String>,
    pub admission_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_error: Option<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyObservationBasis {
    Current,
    Inherited,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredPolicyOccurrence {
    pub sequence: u64,
    pub generation: u64,
    pub basis: PolicyObservationBasis,
    pub occurrence: PolicyOccurrence,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InheritedPolicyHistory {
    pub parent_ledger_sha256: Option<String>,
    pub expected_occurrences: Option<usize>,
    pub available_occurrences: usize,
    pub complete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyLedgerSummary {
    pub engine: String,
    pub policy_sha256: String,
    pub revision: u64,
    pub occurrences: usize,
    pub committed_bytes: usize,
    pub rolling_sha256: String,
    pub incomplete_sources: usize,
    pub complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inherited_history: Option<InheritedPolicyHistory>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyOccurrencePage {
    pub ledger: Option<PolicyLedgerSummary>,
    pub rows: Vec<StoredPolicyOccurrence>,
    pub next_sequence: Option<u64>,
    pub has_more: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PagePolicyObservation {
    pub admission: PolicyAdmission,
    pub effective_can_expand: bool,
    pub redirect_decisions: Vec<PolicyOccurrence>,
    pub source_revision: String,
    pub candidates_complete: bool,
    pub candidate_count: usize,
}

pub(crate) fn digest(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
pub(crate) fn source_revision(page: &PageRecord) -> Result<String> {
    Ok(digest(serde_json::to_vec(&(
        page.url.as_str(),
        page.document_url(),
        &page.document_base_url,
        &page.content_hash,
        &page.links,
        &page.link_relationships,
        &page.pdf,
    ))?))
}
// Separate observed identity, provenance and task role are intentional inputs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn decide(
    policy: &CompiledCrawlPolicy,
    source_revision: &str,
    ordinal: usize,
    source_kind: CandidateSourceKind,
    raw: String,
    base: Option<String>,
    source_url: Option<String>,
    basis: PolicyInputBasis,
    role: PolicyTaskRole,
) -> PolicyOccurrence {
    let input = CandidateInput {
        occurrence_id: digest(format!("{source_revision}:{ordinal}:{}", role.key())),
        source_kind,
        raw_value: raw,
        base_url: base,
        source_url,
        source_location: None,
    };
    let decision = policy.evaluate(&input);
    PolicyOccurrence {
        source_revision: source_revision.into(),
        source_ordinal: ordinal,
        input_basis: basis,
        raw_value_observed: basis != PolicyInputBasis::ResolvedLegacyFallback,
        admission_reason: if decision.decision == CrawlPolicyDecisionKind::Deny {
            "policy_denied"
        } else {
            "pending_admission"
        }
        .into(),
        decision,
        task_role: role,
        scheduled_fetch_url: None,
        coalesced_task_occurrence_id: None,
        input_error: None,
    }
}
pub(crate) fn admission(row: &PolicyOccurrence) -> Option<PolicyAdmission> {
    if row.decision.decision == CrawlPolicyDecisionKind::Deny {
        return None;
    }
    Some(PolicyAdmission {
        engine: LIVE_CRAWL_POLICY_ENGINE.into(),
        policy_sha256: row.decision.policy_sha256.clone(),
        occurrence_id: row.decision.occurrence_id.clone(),
        fetch_url: row.decision.fetch_candidate_url.clone()?,
        source_kind: row.decision.source_kind,
        task_role: row.task_role,
        can_expand: row.task_role == PolicyTaskRole::PageDocument && row.decision.traverse,
        can_discover_xml: row.task_role == PolicyTaskRole::SitemapDocument
            && row.decision.folder_internal,
        inherited: false,
    })
}

pub(crate) const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS policy_occurrences(sequence INTEGER PRIMARY KEY,occurrence_id TEXT UNIQUE NOT NULL,generation INTEGER NOT NULL,data TEXT NOT NULL); CREATE TABLE IF NOT EXISTS policy_admissions(role TEXT NOT NULL,url TEXT NOT NULL,data TEXT NOT NULL,PRIMARY KEY(role,url)); CREATE TABLE IF NOT EXISTS policy_sources(revision TEXT PRIMARY KEY,complete INTEGER NOT NULL);";

fn metadata<T: serde::de::DeserializeOwned>(
    connection: &Connection,
    key: &str,
) -> Result<Option<T>> {
    let text: Option<String> = connection
        .query_row("SELECT value FROM metadata WHERE key=?1", [key], |row| {
            row.get(0)
        })
        .optional()?;
    text.map(|value| serde_json::from_str(&value).map_err(Into::into))
        .transpose()
}
fn save<T: Serialize>(connection: &Connection, key: &str, value: &T) -> Result<()> {
    connection.execute("INSERT INTO metadata(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,serde_json::to_string(value)?])?;
    Ok(())
}
pub(crate) fn summary(connection: &Connection) -> Result<Option<PolicyLedgerSummary>> {
    let Some(mut value) = metadata::<PolicyLedgerSummary>(connection, "policy_ledger")? else {
        return Ok(None);
    };
    ensure!(
        value.engine == LIVE_CRAWL_POLICY_ENGINE,
        "Unsupported saved live policy ledger engine"
    );
    let pending: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM frontier WHERE done=0 LIMIT 1)",
        [],
        |row| row.get(0),
    )?;
    let discovery: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM discovery_frontier WHERE done=0 LIMIT 1)",
        [],
        |row| row.get(0),
    )?;
    value.complete = !pending
        && !discovery
        && value.incomplete_sources == 0
        && value
            .inherited_history
            .as_ref()
            .is_none_or(|history| history.complete);
    Ok(Some(value))
}
pub(crate) fn initialize(
    connection: &Connection,
    sha: &str,
    parent: Option<&PolicyLedgerSummary>,
    inherited: bool,
) -> Result<()> {
    if let Some(existing) = summary(connection)? {
        ensure!(existing.policy_sha256 == sha, "Saved ledger policy changed");
        return Ok(());
    }
    let history = inherited.then(|| InheritedPolicyHistory {
        parent_ledger_sha256: parent.map(|parent| parent.rolling_sha256.clone()),
        expected_occurrences: parent.map(|parent| parent.occurrences),
        available_occurrences: 0,
        // Snapshot summary claims cannot establish availability of its rows,
        // including an imported summary which declares zero occurrences.
        complete: false,
    });
    save(
        connection,
        "policy_ledger",
        &PolicyLedgerSummary {
            engine: LIVE_CRAWL_POLICY_ENGINE.into(),
            policy_sha256: sha.into(),
            revision: 0,
            occurrences: 0,
            committed_bytes: 0,
            rolling_sha256: digest([]),
            incomplete_sources: 0,
            complete: false,
            inherited_history: history,
        },
    )
}
pub(crate) fn saved_admission(
    connection: &Connection,
    role: PolicyTaskRole,
    url: &str,
) -> Result<Option<PolicyAdmission>> {
    let value: Option<String> = connection
        .query_row(
            "SELECT data FROM policy_admissions WHERE role=?1 AND url=?2",
            params![role.key(), url],
            |row| row.get(0),
        )
        .optional()?;
    value
        .map(|value| serde_json::from_str(&value).map_err(Into::into))
        .transpose()
}
pub(crate) fn save_admission(connection: &Connection, value: &PolicyAdmission) -> Result<()> {
    connection.execute(
        "INSERT OR IGNORE INTO policy_admissions(role,url,data) VALUES(?1,?2,?3)",
        params![
            value.task_role.key(),
            value.fetch_url,
            serde_json::to_string(value)?
        ],
    )?;
    Ok(())
}
/// Caller owns the source/frontier transaction and generation guard.
pub(crate) fn commit(
    connection: &Connection,
    rows: &mut [PolicyOccurrence],
    generation: u64,
    limits: &LiveCrawlPolicyLimits,
    source: Option<(&str, bool)>,
) -> Result<()> {
    ensure!(
        rows.len() <= limits.max_batch_occurrences,
        "Policy source occurrence budget exceeded; source remains pending"
    );
    let mut state = summary(connection)?.context("Policy ledger not initialized")?;
    let mut batch_bytes = 0usize;
    let evaluations_complete = rows.iter().all(|row| row.decision.policy_complete);
    for row in rows {
        ensure!(
            row.decision.policy_sha256 == state.policy_sha256,
            "Occurrence policy hash changed"
        );
        if row.admission_reason == "pending_admission"
            && let Some(task) = admission(row)
        {
            if let Some(saved) = saved_admission(connection, row.task_role, &task.fetch_url)? {
                row.coalesced_task_occurrence_id = Some(saved.occurrence_id);
                row.admission_reason = "coalesced".into();
            } else {
                row.scheduled_fetch_url = Some(task.fetch_url.clone());
                row.admission_reason = "scheduled".into();
                save_admission(connection, &task)?;
            }
        }
        ensure!(
            !connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM policy_occurrences WHERE occurrence_id=?1)",
                [&row.decision.occurrence_id],
                |row| row.get::<_, bool>(0)
            )?,
            "Duplicate committed policy source revision"
        );
        let sequence = state
            .occurrences
            .checked_add(1)
            .context("Policy sequence exhausted")?;
        let stored = StoredPolicyOccurrence {
            sequence: sequence as u64,
            generation,
            basis: PolicyObservationBasis::Current,
            occurrence: row.clone(),
        };
        let data = serde_json::to_string(&stored)?;
        ensure!(
            data.len() <= limits.max_row_bytes,
            "Policy occurrence byte budget exceeded; source remains pending"
        );
        batch_bytes = batch_bytes
            .checked_add(data.len())
            .context("Policy batch size overflow")?;
        ensure!(
            batch_bytes <= limits.max_batch_bytes,
            "Policy source byte budget exceeded; source remains pending"
        );
        state.committed_bytes = state
            .committed_bytes
            .checked_add(data.len())
            .context("Policy ledger size overflow")?;
        ensure!(
            limits
                .max_ledger_bytes
                .is_none_or(|limit| state.committed_bytes <= limit),
            "Policy ledger byte budget exceeded; source remains pending"
        );
        state.rolling_sha256 = digest(format!("{}:{}:{}", state.rolling_sha256, data.len(), data));
        state.occurrences = sequence;
        connection.execute("INSERT INTO policy_occurrences(sequence,occurrence_id,generation,data) VALUES(?1,?2,?3,?4)",params![i64::try_from(sequence)?,row.decision.occurrence_id,i64::try_from(generation)?,data])?;
    }
    let incomplete_initial = format!("initial:{}", state.policy_sha256);
    if let Some((revision, complete)) =
        source.or_else(|| (!evaluations_complete).then_some((incomplete_initial.as_str(), false)))
    {
        let complete = complete && evaluations_complete;
        connection.execute(
            "INSERT INTO policy_sources(revision,complete) VALUES(?1,?2)",
            params![revision, complete],
        )?;
        if !complete {
            state.incomplete_sources += 1;
        }
    }
    state.revision = state
        .revision
        .checked_add(1)
        .context("Policy revision exhausted")?;
    save(connection, "policy_ledger", &state)
}
pub(crate) fn page(
    connection: &Connection,
    after: Option<u64>,
    limit: usize,
    max_bytes: usize,
) -> Result<PolicyOccurrencePage> {
    ensure!(
        (1..=1000).contains(&limit),
        "Policy page limit must be 1..=1000"
    );
    let ledger = summary(connection)?;
    if ledger.is_none() {
        return Ok(PolicyOccurrencePage {
            ledger,
            rows: vec![],
            next_sequence: None,
            has_more: false,
        });
    }
    let after = i64::try_from(after.unwrap_or(0)).context("Policy cursor exceeds range")?;
    let mut statement = connection.prepare(
        "SELECT data FROM policy_occurrences WHERE sequence>?1 ORDER BY sequence LIMIT ?2",
    )?;
    let mut rows = vec![];
    let mut bytes = 0usize;
    let mut more = false;
    for value in statement.query_map(params![after, i64::try_from(limit + 1)?], |row| {
        row.get::<_, String>(0)
    })? {
        let value = value?;
        if rows.len() == limit || bytes.saturating_add(value.len()) > max_bytes {
            ensure!(!rows.is_empty(), "Saved policy row exceeds API page budget");
            more = true;
            break;
        }
        bytes += value.len();
        rows.push(serde_json::from_str::<StoredPolicyOccurrence>(&value)?);
    }
    let next = rows.last().map(|row| row.sequence);
    Ok(PolicyOccurrencePage {
        ledger,
        rows,
        next_sequence: next,
        has_more: more,
    })
}
pub(crate) fn verify(connection: &Connection, policy: &CompiledCrawlPolicy) -> Result<()> {
    let Some(state) = summary(connection)? else {
        for table in [
            "frontier",
            "pages",
            "discovery_frontier",
            "policy_occurrences",
            "policy_admissions",
            "policy_sources",
        ] {
            ensure!(
                connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                    .get::<_, i64>(0))?
                    == 0,
                "Configured policy has missing ledger but existing work"
            );
        }
        ensure!(
            metadata::<bool>(connection, "policy_initialized")?.is_none(),
            "Initialized policy ledger missing"
        );
        return Ok(());
    };
    ensure!(
        state.policy_sha256 == policy.policy_sha256(),
        "Saved live policy hash changed"
    );
    let count: i64 =
        connection.query_row("SELECT count(*) FROM policy_occurrences", [], |row| {
            row.get(0)
        })?;
    ensure!(
        usize::try_from(count)? == state.occurrences,
        "Saved policy ledger row count mismatch"
    );
    let generation = metadata::<crate::CrawlRunState>(connection, "run_state")?
        .map_or(0, |state| state.generation);
    let mut hash = digest([]);
    let mut bytes = 0usize;
    let mut statement = connection.prepare(
        "SELECT sequence,occurrence_id,generation,data FROM policy_occurrences ORDER BY sequence",
    )?;
    for (expected_sequence, value) in (1u64..).zip(statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?) {
        let (sequence, occurrence_id, stored_generation, data) = value?;
        let row: StoredPolicyOccurrence = serde_json::from_str(&data)?;
        ensure!(
            row.sequence == expected_sequence
                && u64::try_from(sequence).ok() == Some(row.sequence)
                && occurrence_id == row.occurrence.decision.occurrence_id
                && u64::try_from(stored_generation).ok() == Some(row.generation)
                && row.generation > 0
                && row.generation <= generation
                && row.occurrence.decision.policy_sha256 == state.policy_sha256,
            "Saved policy row sequence/generation/hash identity changed"
        );
        bytes = bytes
            .checked_add(data.len())
            .context("Saved ledger size overflow")?;
        hash = digest(format!("{hash}:{}:{data}", data.len()));
    }
    ensure!(
        hash == state.rolling_sha256 && bytes == state.committed_bytes,
        "Saved policy occurrence payload differs from ledger digest"
    );
    let incomplete: i64 = connection.query_row(
        "SELECT count(*) FROM policy_sources WHERE complete=0",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        usize::try_from(incomplete)? == state.incomplete_sources,
        "Saved policy source completeness changed"
    );
    let inherited_targets: std::collections::HashMap<_, _> =
        match metadata::<crate::RunPlan>(connection, "run_plan")? {
            Some(crate::RunPlan::SelectedRecrawl(plan)) => plan
                .targets
                .into_iter()
                .filter_map(|target| {
                    target
                        .policy_admission
                        .map(|admission| (target.requested_url, admission))
                })
                .collect(),
            _ => std::collections::HashMap::new(),
        };
    for (table, role) in [
        ("frontier", PolicyTaskRole::PageDocument),
        ("discovery_frontier", PolicyTaskRole::SitemapDocument),
    ] {
        let sql = if table == "frontier" {
            "SELECT url FROM frontier"
        } else {
            "SELECT url FROM discovery_frontier WHERE kind='sitemap'"
        };
        let mut statement = connection.prepare(sql)?;
        for url in statement.query_map([], |row| row.get::<_, String>(0))? {
            let url = url?;
            let task = saved_admission(connection, role, &url)?
                .context("Saved frontier has no policy admission")?;
            task.validate(policy.policy_sha256(), &url, role)?;
            if !task.inherited {
                let data: String = connection
                    .query_row(
                        "SELECT data FROM policy_occurrences WHERE occurrence_id=?1",
                        [&task.occurrence_id],
                        |row| row.get(0),
                    )
                    .context("Saved task references missing policy occurrence")?;
                let row: StoredPolicyOccurrence = serde_json::from_str(&data)?;
                ensure!(
                    row.occurrence.admission_reason == "scheduled"
                        && row.occurrence.scheduled_fetch_url.as_deref()
                            == Some(task.fetch_url.as_str())
                        && admission(&row.occurrence).as_ref() == Some(&task),
                    "Saved task permission differs from its immutable policy decision"
                );
            } else {
                ensure!(
                    inherited_targets.get(&url) == Some(&task),
                    "Inherited admission not bound to selected parent target"
                );
            }
        }
    }
    Ok(())
}

pub fn export_policy_occurrences_csv(rows: &[StoredPolicyOccurrence]) -> Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "Sequence",
        "Generation",
        "Basis",
        "Occurrence",
        "SourceKind",
        "InputBasis",
        "RawValue",
        "ObservedURL",
        "FetchAlias",
        "Decision",
        "Admission",
        "ScheduledURL",
        "CoalescedOccurrence",
        "PolicySHA256",
    ])?;
    for row in rows {
        let value = &row.occurrence;
        writer.write_record([
            row.sequence.to_string(),
            row.generation.to_string(),
            format!("{:?}", row.basis),
            value.decision.occurrence_id.clone(),
            format!("{:?}", value.decision.source_kind),
            format!("{:?}", value.input_basis),
            crate::export::csv_text(&value.decision.raw_value),
            crate::export::csv_text(
                value
                    .decision
                    .resolved_discovered_url
                    .as_deref()
                    .unwrap_or(""),
            ),
            crate::export::csv_text(value.decision.fetch_candidate_url.as_deref().unwrap_or("")),
            format!("{:?}", value.decision.decision),
            value.admission_reason.clone(),
            crate::export::csv_text(value.scheduled_fetch_url.as_deref().unwrap_or("")),
            value
                .coalesced_task_occurrence_id
                .clone()
                .unwrap_or_default(),
            value.decision.policy_sha256.clone(),
        ])?;
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);CREATE TABLE frontier(url TEXT PRIMARY KEY,done INTEGER DEFAULT 0);CREATE TABLE discovery_frontier(kind TEXT,url TEXT,done INTEGER DEFAULT 0);").unwrap();
        connection.execute_batch(SCHEMA).unwrap();
        connection
    }
    fn policy() -> CompiledCrawlPolicy {
        CompiledCrawlPolicy::new(&crate::CrawlPolicyConfig {
            scope: crate::CrawlScopeConfig {
                roots: vec![crate::CrawlScopeRoot {
                    origin: "https://example.org".into(),
                    subdomains: false,
                    folders: vec!["/".into()],
                    exact_paths: vec![],
                }],
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap()
    }
    fn occurrence(policy: &CompiledCrawlPolicy, ordinal: usize) -> PolicyOccurrence {
        decide(
            policy,
            "unit-source",
            ordinal,
            CandidateSourceKind::Anchor,
            format!("https://example.org/item/{ordinal}"),
            None,
            None,
            PolicyInputBasis::ObservedAttribute,
            PolicyTaskRole::ObservationOnly,
        )
    }

    #[test]
    fn optional_policy_fields_do_not_change_legacy_serialized_hash_inputs() {
        let config = crate::CrawlConfig::default();
        let text = serde_json::to_string(&config).unwrap();
        let value = serde_json::to_value(&config).unwrap();
        assert!(value.get("crawl_policy").is_none());
        assert!(value.get("live_policy_limits").is_none());
        let old: crate::CrawlConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(digest(&text), digest(serde_json::to_string(&old).unwrap()));
        let page = crate::PageRecord::default();
        let value = serde_json::to_value(&page).unwrap();
        assert!(value.get("policy").is_none());
        let old: crate::PageRecord = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(old).unwrap(), value);
        let target = crate::RecrawlTarget {
            resource_permission: None,
            requested_url: "https://example.org/".into(),
            parent_ordinal: 0,
            parent_page_sha256: "prior".into(),
            depth: 0,
            discovered_from: None,
            policy_admission: None,
        };
        assert!(
            serde_json::to_value(target)
                .unwrap()
                .get("policy_admission")
                .is_none()
        );
        let checkpoint = crate::CrawlCheckpoint {
            generation: 1,
            revision: 1,
            outcome: crate::CrawlOutcome::Paused,
            created_at: "time".into(),
            report_sha256: "report".into(),
            config_sha256: "config".into(),
            completed: 0,
            pending: 1,
            discovery_pending: 0,
            discovery_complete: true,
            policy_ledger: None,
            resource_ledger: None,
            sitemap_ledger: None,
        };
        let text = serde_json::to_string(&checkpoint).unwrap();
        assert!(!text.contains("policy_ledger"));
        assert_eq!(
            text,
            serde_json::to_string(&serde_json::from_str::<crate::CrawlCheckpoint>(&text).unwrap())
                .unwrap()
        );
    }

    #[test]
    fn row_source_batch_and_total_limits_rollback_every_insert() {
        let policy = policy();
        let oversized = || {
            let mut row = occurrence(&policy, 0);
            row.decision.raw_value = format!("https://example.org/?q={}", "x".repeat(2000));
            row
        };
        for (mut rows, limits) in [
            (
                vec![oversized()],
                LiveCrawlPolicyLimits {
                    max_row_bytes: 1024,
                    ..Default::default()
                },
            ),
            (
                (0..10).map(|index| occurrence(&policy, index)).collect(),
                LiveCrawlPolicyLimits {
                    max_row_bytes: 2048,
                    max_batch_bytes: 2048,
                    ..Default::default()
                },
            ),
            (
                vec![occurrence(&policy, 0), occurrence(&policy, 1)],
                LiveCrawlPolicyLimits {
                    max_batch_occurrences: 1,
                    ..Default::default()
                },
            ),
            (
                vec![occurrence(&policy, 0)],
                LiveCrawlPolicyLimits {
                    max_ledger_bytes: Some(1),
                    ..Default::default()
                },
            ),
        ] {
            limits.validate().unwrap();
            let mut connection = database();
            initialize(&connection, policy.policy_sha256(), None, false).unwrap();
            let transaction = connection.transaction().unwrap();
            assert!(commit(&transaction, &mut rows, 1, &limits, Some(("source", true))).is_err());
            transaction.rollback().unwrap();
            assert_eq!(summary(&connection).unwrap().unwrap().occurrences, 0);
            for table in ["policy_occurrences", "policy_admissions", "policy_sources"] {
                assert_eq!(
                    connection
                        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                            .get::<_, i64>(0))
                        .unwrap(),
                    0
                );
            }
        }
        assert_eq!(LiveCrawlPolicyLimits::default().max_ledger_bytes, None);
    }

    #[test]
    fn byte_shortened_pages_keep_exact_cursor_and_zero_is_not_a_missing_row() {
        let policy = policy();
        let mut connection = database();
        initialize(&connection, policy.policy_sha256(), None, false).unwrap();
        let mut rows = vec![occurrence(&policy, 0), occurrence(&policy, 1)];
        let transaction = connection.transaction().unwrap();
        commit(
            &transaction,
            &mut rows,
            1,
            &LiveCrawlPolicyLimits::default(),
            None,
        )
        .unwrap();
        transaction.commit().unwrap();
        let first_bytes = usize::try_from(
            connection
                .query_row(
                    "SELECT length(CAST(data AS BLOB)) FROM policy_occurrences WHERE sequence=1",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
        )
        .unwrap();
        let first = page(&connection, None, 100, first_bytes).unwrap();
        assert_eq!(first.rows.len(), 1);
        assert_eq!(first.next_sequence, Some(1));
        assert!(first.has_more);
        let second = page(&connection, first.next_sequence, 100, usize::MAX).unwrap();
        assert_eq!(second.rows.len(), 1);
        assert_eq!(second.rows[0].sequence, 2);
        assert!(!second.has_more);
        assert!(page(&connection, None, 100, first_bytes - 1).is_err());
        assert!(page(&connection, Some(u64::MAX), 1, usize::MAX).is_err());
        let last = page(&connection, Some(2), 100, usize::MAX).unwrap();
        assert!(last.rows.is_empty());
        assert!(!last.has_more);
        assert_eq!(last.next_sequence, None);
    }

    #[test]
    fn inherited_summary_without_parent_ledger_is_unknown_and_never_complete() {
        let connection = database();
        initialize(&connection, "policy", None, true).unwrap();
        let summary = summary(&connection).unwrap().unwrap();
        assert!(!summary.complete);
        let history = summary.inherited_history.unwrap();
        assert_eq!(history.expected_occurrences, None);
        assert_eq!(history.parent_ledger_sha256, None);
        assert_eq!(history.available_occurrences, 0);
        assert!(!history.complete);
        let parent_connection = database();
        initialize(&parent_connection, "policy", None, false).unwrap();
        let parent = super::summary(&parent_connection).unwrap().unwrap();
        assert_eq!(parent.occurrences, 0);
        assert!(parent.complete);
        let child_connection = database();
        initialize(&child_connection, "policy", Some(&parent), true).unwrap();
        let child = super::summary(&child_connection).unwrap().unwrap();
        assert!(!child.complete);
        let history = child.inherited_history.unwrap();
        assert_eq!(history.expected_occurrences, Some(0));
        assert_eq!(history.parent_ledger_sha256, Some(parent.rolling_sha256));
        assert!(!history.complete);
    }
}
