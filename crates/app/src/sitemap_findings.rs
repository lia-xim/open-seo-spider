//! Shared read-only findings transport framing; actual evidence stays in Core.
//! Tokens are canonical context checksums, not MACs or authorization grants.
pub(crate) use crate::sitemap_membership::Failure;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use open_seo_spider_core::{
    MAX_SITEMAP_FINDINGS_SELECTION_BYTES, SITEMAP_FINDINGS_ENGINE,
    SitemapFindingAssessmentSelection, SitemapFindingDomain, SitemapFindingEvidence,
    SitemapFindingFilter, SitemapFindingPosition, SitemapFindingSelection, SitemapFindingVerdict,
    SitemapFindingsAvailability, SitemapFindingsContext, SitemapFindingsError, SitemapFindingsPage,
    SitemapFindingsRequest, SitemapReadControlError, SitemapVerifiedFindings,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt,
    io::{self, Write},
};
use uuid::Uuid;

pub(crate) const ENGINE: &str = "open-seo-spider/app-sitemap-findings-v1";
pub(crate) const MAX_JSON_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const MAX_CSV_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const MAX_REQUEST_BYTES: usize = 64 * 1024 * 1024;
const MAX_TOKEN_BYTES: usize = 4096;
type Result<T> = std::result::Result<T, Failure>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum AccessTarget {
    Job {
        job_id: Uuid,
        generation: u64,
        config_sha256: String,
    },
    Offline,
}
impl AccessTarget {
    fn validate(&self) -> Result<()> {
        if let Self::Job { config_sha256, .. } = self {
            if !is_hash(config_sha256) {
                return Err(invalid_query("Invalid App config SHA"));
            }
        }
        Ok(())
    }
}

pub(crate) fn status(failure: &Failure) -> u16 {
    match failure.code.as_str() {
        "invalid_sitemap_findings_query" | "invalid_sitemap_findings_cursor" => 400,
        "stale_sitemap_findings_cursor"
        | "sitemap_findings_job_snapshot_conflict"
        | "sitemap_findings_unavailable" => 409,
        "sitemap_findings_response_byte_budget"
        | "sitemap_findings_csv_byte_budget"
        | "sitemap_findings_selection_byte_budget"
        | "sitemap_findings_input_byte_budget" => 413,
        "sitemap_findings_cancelled" => 503,
        "sitemap_findings_timeout" => 504,
        "job_not_found" => 404,
        _ => failure.status(),
    }
}
fn invalid_query(message: impl ToString) -> Failure {
    Failure::new("invalid_sitemap_findings_query", message)
}
fn invalid_cursor(message: impl ToString) -> Failure {
    Failure::new("invalid_sitemap_findings_cursor", message)
}
fn stale(message: impl ToString) -> Failure {
    Failure::new("stale_sitemap_findings_cursor", message)
}
fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn typed_failure(error: anyhow::Error) -> Failure {
    let message = error.to_string();
    typed_failure_inner(error, &message, 0)
}

const MAX_FAILURE_DEPTH: usize = 32;

// io::Error::source delegates to its inner error's source, skipping the inner
// object itself. Prefer get_ref so a directly wrapped typed error is observed.
fn inspect_failure(mut cause: &(dyn Error + 'static), message: &str) -> (Option<Failure>, bool) {
    let mut saw_io = false;
    for _ in 0..MAX_FAILURE_DEPTH {
        if let Some(failure) = cause.downcast_ref::<Failure>() {
            return (Some(failure.clone()), saw_io);
        }
        if let Some(stop) = cause.downcast_ref::<SitemapReadControlError>() {
            return (
                Some(Failure::new(
                    match stop {
                        SitemapReadControlError::Cancelled => "sitemap_findings_cancelled",
                        SitemapReadControlError::DeadlineExceeded => "sitemap_findings_timeout",
                    },
                    message,
                )),
                saw_io,
            );
        }
        if let Some(core) = cause.downcast_ref::<SitemapFindingsError>() {
            let code = match core {
                SitemapFindingsError::InvalidSelection(_) => "invalid_sitemap_findings_query",
                SitemapFindingsError::InvalidContinuation => "invalid_sitemap_findings_cursor",
                SitemapFindingsError::StaleContext => "stale_sitemap_findings_cursor",
                SitemapFindingsError::ResponseByteBudget => "sitemap_findings_response_byte_budget",
                SitemapFindingsError::SelectionByteBudget => {
                    "sitemap_findings_selection_byte_budget"
                }
                SitemapFindingsError::Unavailable => "sitemap_findings_unavailable",
            };
            return (Some(Failure::new(code, message)), saw_io);
        }
        if cause.downcast_ref::<CsvByteBudget>().is_some() {
            return (
                Some(Failure::new("sitemap_findings_csv_byte_budget", message)),
                saw_io,
            );
        }
        if cause
            .downcast_ref::<serde_json::Error>()
            .is_some_and(serde_json::Error::is_io)
        {
            saw_io = true;
        }
        if let Some(io) = cause.downcast_ref::<io::Error>() {
            saw_io = true;
            if let Some(inner) = io.get_ref() {
                cause = inner;
                continue;
            }
        }
        let Some(source) = cause.source() else {
            return (None, saw_io);
        };
        cause = source;
    }
    // A truncated/cyclic source chain cannot establish an input-budget claim.
    (None, true)
}

fn typed_failure_inner(error: anyhow::Error, message: &str, depth: usize) -> Failure {
    if depth >= MAX_FAILURE_DEPTH {
        return Failure::new("sitemap_findings_read_failure", message);
    }
    let (known, saw_io) = inspect_failure(error.as_ref(), message);
    if let Some(known) = known {
        return known;
    }
    // serde_json::Error::source also skips the owned IO object. Its public
    // conversion recovers that actual writer error; never recreate it from text.
    if error.is::<serde_json::Error>() {
        return match error.downcast::<serde_json::Error>() {
            Ok(json) if json.is_io() => {
                typed_failure_inner(io::Error::from(json).into(), message, depth + 1)
            }
            _ => untyped_failure(message, saw_io),
        };
    }
    if error.is::<io::Error>() {
        if let Ok(io) = error.downcast::<io::Error>() {
            if let Some(inner) = io.into_inner() {
                let inner = match inner.downcast::<serde_json::Error>() {
                    Ok(json) => {
                        return typed_failure_inner((*json).into(), message, depth + 1);
                    }
                    Err(inner) => inner,
                };
                if let Ok(io) = inner.downcast::<io::Error>() {
                    return typed_failure_inner((*io).into(), message, depth + 1);
                }
            }
        }
        return Failure::new("sitemap_findings_read_failure", message);
    }
    untyped_failure(message, saw_io)
}

fn untyped_failure(message: &str, saw_io: bool) -> Failure {
    // Existing Core input validators do not all have typed budget errors yet.
    // Ordinary writer IO/serde-IO never becomes a budget solely from its text.
    let code = if !saw_io
        && (message.contains("budget exceeded") || message.contains("exceeds bounded input"))
    {
        "sitemap_findings_input_byte_budget"
    } else {
        "sitemap_findings_read_failure"
    };
    Failure::new(code, message)
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ReadQuery {
    pub filter: SitemapFindingFilter,
    pub assessment: SitemapFindingAssessmentSelection,
    pub requested_url: Option<String>,
    pub source_task_url: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub candidate_limit: Option<usize>,
    pub cursor: Option<String>,
}
impl ReadQuery {
    fn borrowed_selection(&self) -> BorrowedSelection<'_> {
        BorrowedSelection {
            filter: self.filter,
            assessment: self.assessment,
            requested_url: &self.requested_url,
            source_task_url: &self.source_task_url,
            search: &self.search,
        }
    }
    pub(crate) fn validate(&self) -> Result<()> {
        self.borrowed_selection().validate()?;
        if !(1..=200).contains(&self.limit.unwrap_or(100))
            || !(1..=5000).contains(&self.candidate_limit.unwrap_or(5000))
        {
            return Err(invalid_query(
                "limit must be1..200 and candidate_limit1..5000",
            ));
        }
        if let Some(cursor) = &self.cursor {
            decode(cursor, Role::Page)?;
        }
        Ok(())
    }
    pub(crate) fn prepare(self, target: &AccessTarget) -> Result<PreparedRead> {
        self.validate()?;
        target.validate()?;
        let token = self
            .cursor
            .as_deref()
            .map(|s| decode(s, Role::Page))
            .transpose()?;
        let selection = SitemapFindingSelection {
            filter: self.filter,
            assessment: self.assessment,
            requested_url: self.requested_url,
            source_task_url: self.source_task_url,
            search: self.search,
        };
        let selection_sha256 = selection_hash(&selection)?;
        if let Some(token) = &token {
            bind_token(token, target, &selection, &selection_sha256)?;
        }
        let request = SitemapFindingsRequest {
            selection,
            limit: self.limit.unwrap_or(100),
            candidate_limit: self.candidate_limit.unwrap_or(5000),
            after: token.as_ref().and_then(|t| t.payload.after.clone()),
            expected_context_sha256: token
                .as_ref()
                .map(|t| t.payload.snapshot.core_context_sha256.clone()),
        };
        request.validate().map_err(typed_failure)?;
        Ok(PreparedRead {
            request,
            target: target.clone(),
            selection_sha256,
        })
    }
}
pub(crate) struct PreparedRead {
    request: SitemapFindingsRequest,
    target: AccessTarget,
    selection_sha256: String,
}
impl PreparedRead {
    pub(crate) fn request(&self) -> &SitemapFindingsRequest {
        &self.request
    }
    pub(crate) fn selection(&self) -> &SitemapFindingSelection {
        &self.request.selection
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CsvQuery {
    #[serde(default)]
    pub filter: SitemapFindingFilter,
    #[serde(default)]
    pub assessment: SitemapFindingAssessmentSelection,
    pub requested_url: Option<String>,
    pub source_task_url: Option<String>,
    pub search: Option<String>,
    pub snapshot: String,
}
impl CsvQuery {
    pub(crate) fn validate(&self) -> Result<()> {
        BorrowedSelection {
            filter: self.filter,
            assessment: self.assessment,
            requested_url: &self.requested_url,
            source_task_url: &self.source_task_url,
            search: &self.search,
        }
        .validate()?;
        decode(&self.snapshot, Role::Export)?;
        Ok(())
    }
    pub(crate) fn prepare(self, target: &AccessTarget) -> Result<PreparedCsv> {
        self.validate()?;
        target.validate()?;
        let token = decode(&self.snapshot, Role::Export)?;
        let selection = SitemapFindingSelection {
            filter: self.filter,
            assessment: self.assessment,
            requested_url: self.requested_url,
            source_task_url: self.source_task_url,
            search: self.search,
        };
        let selection_sha256 = selection_hash(&selection)?;
        bind_token(&token, target, &selection, &selection_sha256)?;
        Ok(PreparedCsv {
            selection,
            snapshot: token.payload.snapshot,
        })
    }
}
pub(crate) struct PreparedCsv {
    selection: SitemapFindingSelection,
    snapshot: FindingsExportSnapshot,
}
impl PreparedCsv {
    pub(crate) fn selection(&self) -> &SitemapFindingSelection {
        &self.selection
    }
    pub(crate) fn expected_context_sha256(&self) -> &str {
        &self.snapshot.core_context_sha256
    }
    pub(crate) fn bind_session(
        &self,
        session: &SitemapVerifiedFindings<'_>,
    ) -> Result<FindingsExportSnapshot> {
        self.validate_session(session)?;
        Ok(self.snapshot.clone())
    }
    pub(crate) fn validate_session(&self, session: &SitemapVerifiedFindings<'_>) -> Result<()> {
        if session.selection() != &self.selection {
            return Err(stale("Finding selection changed"));
        }
        let actual = bind_context(
            &self.snapshot.access_target,
            &self.selection,
            &self.snapshot.selection_sha256,
            session.context(),
        )?;
        if actual != self.snapshot {
            return Err(stale("Finding export context changed"));
        }
        if session.context().availability == SitemapFindingsAvailability::Unavailable {
            return Err(Failure::new(
                "sitemap_findings_unavailable",
                "Finding evidence unavailable",
            ));
        }
        Ok(())
    }
}

// The borrowed view has the exact Core Selection field order and validation,
// without cloning a potentially32MiB source selector merely to reject input.
struct BorrowedSelection<'a> {
    filter: SitemapFindingFilter,
    assessment: SitemapFindingAssessmentSelection,
    requested_url: &'a Option<String>,
    source_task_url: &'a Option<String>,
    search: &'a Option<String>,
}
impl BorrowedSelection<'_> {
    fn validate(&self) -> Result<()> {
        if (self.filter.domain() == SitemapFindingDomain::SavedPage
            && self.source_task_url.is_some())
            || (self.filter.domain() == SitemapFindingDomain::SourceAttempt
                && self.requested_url.is_some())
        {
            return Err(invalid_query("Selector domain differs"));
        }
        for (value, max) in [
            (self.requested_url, 65_536),
            (self.source_task_url, 32 * 1024 * 1024),
            (self.search, 1024),
        ] {
            if value
                .as_ref()
                .is_some_and(|v| v.is_empty() || v.len() > max)
            {
                return Err(invalid_query("Selector text budget"));
            }
        }
        // Kept byte-identical to Core validate; prepare also calls that actual
        // validator once after moving these strings into the Core request.
        let _ = self.assessment;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FindingsExportSnapshot {
    pub adapter_engine: String,
    pub core_engine: String,
    pub access_target: AccessTarget,
    pub domain: SitemapFindingDomain,
    pub selection_sha256: String,
    pub core_context_sha256: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Role {
    #[serde(rename = "findings_page_v1")]
    Page,
    #[serde(rename = "findings_export_v1")]
    Export,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    schema_version: u32,
    role: Role,
    snapshot: FindingsExportSnapshot,
    after: Option<SitemapFindingPosition>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Token {
    payload: Payload,
    checksum_sha256: String,
}
fn token(
    snapshot: &FindingsExportSnapshot,
    after: Option<SitemapFindingPosition>,
    role: Role,
) -> Result<String> {
    let payload = Payload {
        schema_version: 1,
        role,
        snapshot: snapshot.clone(),
        after,
    };
    let checksum_sha256 = small_hash(&payload)?;
    let bytes = serde_json::to_vec(&Token {
        payload,
        checksum_sha256,
    })
    .map_err(|e| Failure::new("sitemap_findings_read_failure", e))?;
    let value = URL_SAFE_NO_PAD.encode(bytes);
    if value.len() > MAX_TOKEN_BYTES {
        return Err(Failure::new(
            "sitemap_findings_read_failure",
            "Finding token profile exceeds cap",
        ));
    }
    Ok(value)
}
fn decode(value: &str, role: Role) -> Result<Token> {
    if value.is_empty() || value.len() > MAX_TOKEN_BYTES {
        return Err(invalid_cursor("Token byte length"));
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| invalid_cursor("Token base64"))?;
    let parsed: Token =
        serde_json::from_slice(&bytes).map_err(|_| invalid_cursor("Token shape/types"))?;
    let canonical =
        serde_json::to_vec(&parsed).map_err(|_| invalid_cursor("Token serialization"))?;
    let p = &parsed.payload;
    let s = &p.snapshot;
    if URL_SAFE_NO_PAD.encode(canonical) != value
        || p.schema_version != 1
        || p.role != role
        || s.adapter_engine != ENGINE
        || s.core_engine != SITEMAP_FINDINGS_ENGINE
        || !is_hash(&s.selection_sha256)
        || !is_hash(&s.core_context_sha256)
        || !is_hash(&parsed.checksum_sha256)
        || small_hash(p)? != parsed.checksum_sha256
    {
        return Err(invalid_cursor("Token canonical profile/checksum"));
    }
    s.access_target
        .validate()
        .map_err(|_| invalid_cursor("Token target shape"))?;
    match (role, &p.after) {
        (Role::Page, Some(after)) => after
            .validate(s.domain)
            .map_err(|_| invalid_cursor("Token position domain/range"))?,
        (Role::Export, None) => {}
        _ => return Err(invalid_cursor("Token role/position differs")),
    }
    Ok(parsed)
}
fn bind_token(
    token: &Token,
    target: &AccessTarget,
    selection: &SitemapFindingSelection,
    sha: &str,
) -> Result<()> {
    let s = &token.payload.snapshot;
    if &s.access_target != target
        || s.domain != selection.filter.domain()
        || s.selection_sha256 != sha
    {
        return Err(stale("Finding target or selection changed"));
    }
    Ok(())
}
fn bind_context(
    target: &AccessTarget,
    selection: &SitemapFindingSelection,
    selection_sha: &str,
    context: &SitemapFindingsContext,
) -> Result<FindingsExportSnapshot> {
    if context.engine != SITEMAP_FINDINGS_ENGINE
        || context.selection_sha256 != selection_sha
        || !is_hash(&context.context_sha256)
    {
        return Err(Failure::new(
            "sitemap_findings_read_failure",
            "Finding engine/query/context binding differs",
        ));
    }
    if let AccessTarget::Job {
        generation,
        config_sha256,
        ..
    } = target
    {
        if context.basis.generation != ((*generation > 0).then_some(*generation))
            || context.basis.crawl_config_sha256.as_deref() != Some(config_sha256.as_str())
        {
            return Err(Failure::new(
                "sitemap_findings_job_snapshot_conflict",
                "Actual SQL generation/config differs from App job",
            ));
        }
    }
    Ok(FindingsExportSnapshot {
        adapter_engine: ENGINE.into(),
        core_engine: context.engine.clone(),
        access_target: target.clone(),
        domain: selection.filter.domain(),
        selection_sha256: selection_sha.into(),
        core_context_sha256: context.context_sha256.clone(),
    })
}

#[derive(Serialize)]
pub(crate) struct FindingsEnvelope {
    pub engine: &'static str,
    pub access_target: AccessTarget,
    pub website_requests: u8,
    pub findings: Option<SitemapFindingsPage>,
    pub next_cursor: Option<String>,
    pub export_token: Option<String>,
    pub availability_reason: Option<&'static str>,
}
pub(crate) fn bind_page(
    target: &AccessTarget,
    prepared: &PreparedRead,
    page: SitemapFindingsPage,
) -> Result<FindingsEnvelope> {
    if target != &prepared.target {
        return Err(stale("Prepared access target changed"));
    }
    let snapshot = bind_context(
        target,
        prepared.selection(),
        &prepared.selection_sha256,
        &page.context,
    )?;
    if prepared
        .request
        .expected_context_sha256
        .as_deref()
        .is_some_and(|e| e != snapshot.core_context_sha256)
    {
        return Err(stale("Finding page context changed"));
    }
    let unavailable = page.context.availability == SitemapFindingsAvailability::Unavailable;
    let next_cursor = page
        .continuation
        .as_ref()
        .map(|c| {
            if unavailable || c.context_sha256 != snapshot.core_context_sha256 {
                return Err(Failure::new(
                    "sitemap_findings_read_failure",
                    "Invalid actual continuation context",
                ));
            }
            c.after.validate(snapshot.domain).map_err(typed_failure)?;
            token(&snapshot, Some(c.after.clone()), Role::Page)
        })
        .transpose()?;
    let export_token = if unavailable {
        None
    } else {
        Some(token(&snapshot, None, Role::Export)?)
    };
    let response = FindingsEnvelope {
        engine: ENGINE,
        access_target: target.clone(),
        website_requests: 0,
        findings: Some(page),
        next_cursor,
        export_token,
        availability_reason: unavailable.then_some("core_evidence_unavailable"),
    };
    measure_envelope(&response)?;
    Ok(response)
}
pub(crate) fn unavailable_page(target: &AccessTarget) -> Result<FindingsEnvelope> {
    target.validate()?;
    let response = FindingsEnvelope {
        engine: ENGINE,
        access_target: target.clone(),
        website_requests: 0,
        findings: None,
        next_cursor: None,
        export_token: None,
        availability_reason: Some("saved_sqlite_unavailable"),
    };
    measure_envelope(&response)?;
    Ok(response)
}

struct JsonSink {
    bytes: usize,
    max: usize,
    exceeded: bool,
    hash: Sha256,
}
impl Write for JsonSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self.bytes.checked_add(bytes.len());
        if next.is_none_or(|n| n > self.max) {
            self.exceeded = true;
            return Err(io::Error::other("Finding JSON budget"));
        }
        self.bytes = next.expect("checked size");
        for chunk in bytes.chunks(65_536) {
            self.hash.update(chunk);
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn json_hash(value: &impl Serialize, max: usize, code: &str) -> Result<String> {
    let mut sink = JsonSink {
        bytes: 0,
        max,
        exceeded: false,
        hash: Sha256::new(),
    };
    let result = serde_json::to_writer(&mut sink, value);
    if sink.exceeded {
        return Err(Failure::new(
            code,
            "Serialized finding JSON exceeds byte budget",
        ));
    }
    result.map_err(|e| Failure::new("sitemap_findings_read_failure", e))?;
    Ok(format!("{:x}", sink.hash.finalize()))
}
fn selection_hash(selection: &SitemapFindingSelection) -> Result<String> {
    selection.validate().map_err(typed_failure)?;
    json_hash(
        selection,
        MAX_SITEMAP_FINDINGS_SELECTION_BYTES,
        "sitemap_findings_selection_byte_budget",
    )
}
fn small_hash(value: &impl Serialize) -> Result<String> {
    json_hash(value, MAX_TOKEN_BYTES, "invalid_sitemap_findings_cursor")
}
fn measure_envelope(value: &FindingsEnvelope) -> Result<()> {
    json_hash(
        value,
        MAX_JSON_BYTES,
        "sitemap_findings_response_byte_budget",
    )
    .map(|_| ())
}

#[derive(Debug)]
struct CsvByteBudget;
impl fmt::Display for CsvByteBudget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Finding CSV exceeds64MiB")
    }
}
impl Error for CsvByteBudget {}
struct CsvBounded<'a, W: Write> {
    writer: &'a mut W,
    bytes: usize,
    exceeded: bool,
}
impl<W: Write> Write for CsvBounded<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(next) = self
            .bytes
            .checked_add(bytes.len())
            .filter(|n| *n <= MAX_CSV_BYTES)
        else {
            self.exceeded = true;
            return Err(io::Error::other(CsvByteBudget));
        };
        self.writer.write_all(bytes)?;
        self.bytes = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
// JSON UTF8 is escaped as bytes; no intermediate32MiB JSON String or row Vec.
struct CsvEscaped<'a, W: Write>(&'a mut W);
impl<W: Write> Write for CsvEscaped<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        for chunk in bytes.chunks(65_536) {
            let mut start = 0;
            for (index, value) in chunk.iter().enumerate() {
                if *value == b'"' {
                    self.0.write_all(&chunk[start..index])?;
                    self.0.write_all(b"\"\"")?;
                    start = index + 1;
                }
            }
            self.0.write_all(&chunk[start..])?;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
fn text(writer: &mut impl Write, value: &str) -> anyhow::Result<()> {
    writer.write_all(b"\"")?;
    CsvEscaped(writer).write_all(value.as_bytes())?;
    writer.write_all(b"\"")?;
    Ok(())
}
fn json_field(writer: &mut impl Write, value: &impl Serialize) -> anyhow::Result<()> {
    writer.write_all(b"\"")?;
    serde_json::to_writer(CsvEscaped(writer), value)?;
    writer.write_all(b"\"")?;
    Ok(())
}
fn comma(writer: &mut impl Write) -> io::Result<()> {
    writer.write_all(b",")
}
fn domain(value: SitemapFindingDomain) -> &'static str {
    match value {
        SitemapFindingDomain::SavedPage => "saved_page",
        SitemapFindingDomain::SourceAttempt => "source_attempt",
    }
}
fn filter(value: SitemapFindingFilter) -> &'static str {
    value
        .granular_id()
        .strip_prefix("filter.sitemaps-")
        .expect("fixed Core filter prefix")
}
fn selected(value: SitemapFindingAssessmentSelection) -> &'static str {
    match value {
        SitemapFindingAssessmentSelection::Match => "match",
        SitemapFindingAssessmentSelection::NotMatched => "not_matched",
        SitemapFindingAssessmentSelection::Unknown => "unknown",
        SitemapFindingAssessmentSelection::All => "all",
    }
}
fn verdict(value: SitemapFindingVerdict) -> &'static str {
    match value {
        SitemapFindingVerdict::Match => "match",
        SitemapFindingVerdict::NotMatched => "not_matched",
        SitemapFindingVerdict::Unknown => "unknown",
    }
}

pub(crate) fn write_findings_csv_session(
    session: &SitemapVerifiedFindings<'_>,
    writer: &mut impl Write,
) -> anyhow::Result<u64> {
    write_findings_csv_session_observed(session, writer, || {})
}
pub(crate) fn write_findings_csv_session_observed(
    session: &SitemapVerifiedFindings<'_>,
    writer: &mut impl Write,
    mut on_row: impl FnMut(),
) -> anyhow::Result<u64> {
    if session.context().availability == SitemapFindingsAvailability::Unavailable {
        return Err(SitemapFindingsError::Unavailable.into());
    }
    let selection = session.selection();
    if selection_hash(selection).map_err(anyhow::Error::new)? != session.context().selection_sha256
    {
        return Err(anyhow::anyhow!("Finding CSV selection context differs"));
    }
    let mut out = CsvBounded {
        writer,
        bytes: 0,
        exceeded: false,
    };
    let result = (|| {
        for (index, name) in [
            "record_type",
            "schema_version",
            "domain",
            "filter",
            "assessment",
            "original_identity",
            "position_json",
            "reasons_json",
            "typed_finding_json",
            "context_json",
        ]
        .into_iter()
        .enumerate()
        {
            if index > 0 {
                comma(&mut out)?;
            }
            text(&mut out, name)?;
        }
        out.write_all(b"\r\n")?;
        for (index, value) in [
            "context",
            "1",
            domain(selection.filter.domain()),
            filter(selection.filter),
            selected(selection.assessment),
            "",
            "",
            "",
            "",
        ]
        .into_iter()
        .enumerate()
        {
            if index > 0 {
                comma(&mut out)?;
            }
            text(&mut out, value)?;
        }
        #[derive(Serialize)]
        struct Metadata<'a> {
            engine: &'static str,
            selection: &'a SitemapFindingSelection,
            context: &'a SitemapFindingsContext,
        }
        comma(&mut out)?;
        json_field(
            &mut out,
            &Metadata {
                engine: ENGINE,
                selection,
                context: session.context(),
            },
        )?;
        out.write_all(b"\r\n")?;
        let mut rows = 0u64;
        session.visit_all(|row| {
            let identity = match &row.evidence {
                SitemapFindingEvidence::SavedPage { page } => {
                    page.finding.observation.requested_url.as_str()
                }
                SitemapFindingEvidence::SourceAttempt { source } => source.original_task_url(),
            };
            for (index, value) in [
                "finding",
                "1",
                domain(selection.filter.domain()),
                filter(selection.filter),
                verdict(row.assessment.verdict),
                identity,
            ]
            .into_iter()
            .enumerate()
            {
                if index > 0 {
                    comma(&mut out)?;
                }
                if index == 5 {
                    text(&mut out, &open_seo_spider_core::csv_text(value))?;
                } else {
                    text(&mut out, value)?;
                }
            }
            comma(&mut out)?;
            json_field(&mut out, &row.position)?;
            comma(&mut out)?;
            json_field(&mut out, &row.assessment.reasons)?;
            comma(&mut out)?;
            json_field(&mut out, row)?;
            comma(&mut out)?;
            text(&mut out, "")?;
            out.write_all(b"\r\n")?;
            rows = rows
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("Finding CSV row count overflow"))?;
            on_row();
            Ok(())
        })?;
        out.flush()?;
        Ok(rows)
    })();
    if out.exceeded {
        return Err(CsvByteBudget.into());
    }
    result
}

#[cfg(test)]
#[path = "sitemap_findings_tests.rs"]
mod tests;
