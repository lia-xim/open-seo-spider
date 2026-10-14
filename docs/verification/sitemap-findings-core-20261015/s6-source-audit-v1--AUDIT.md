# S6 source evidence audit and minimal private API proposal

DESIGN / READ_ONLY_SOURCE_AUDIT. Root-declared HEADc8f539b, actual Root `E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-csv-streaming-integration-v1`; frozen265 manifest `a31936510381cca40901eb547f15090a77c833f617f84f51f67b057fe7a5714e`. Product pins are checked against that frozen source. No product/Git/runtime writes, compiler, Cargo, tests, SQLite connection, HTTP, browser or native execution. No feature/API is implemented by this note. Root owns the model/API handshake.

## Actual current proof boundary

`crates/core/src/sitemap_source_store/verified_read.rs:15–34` has a private-field VerifiedSourceRead with only summary(), context(), connection(), materializing_selected_compatibility(). There is NO measurement/link/attempt visitor. `verify_source_with_control:140` requires a real read transaction, actual Raw pointer equality, whole header/schema/canonical payload/count/hash/byte validation, all measurement records, all exact B links and actual raw Document seals before returning. Ordinary rows are streamed at221–290, preserving one bounded record/link/document. Selected uses exact plan/parent projection plus old bounded materializing load at40–99,171–194. Some/None historical presence is distinct and partial schema is an error.

`store/sitemap_read_context.rs:115` already obtains the correct same-TX source token, but PreparedSitemapRead at34–39 exposes only header/raw/analysis/coverage; the callback cannot reach it. Header contains SourceSummary+context only and is a DTO, not a measurement proof. The smallest Root integration hook is to add an optional borrowed actual Source token to that private PreparedSitemapRead and pass source.as_ref(); retain its current lifetime until the existing drop(source) at187. Do not reload a32MiB SourceSnapshot, expose a public SQL connection or manufacture a partial catalog.

Root's prepared context must retain both existing higher selected-parent presence gates before any new projection, including SourceNone. Low-level absence is not proof that an initialized selected parent had no measurements. Existing SourceReport/checkpoint equality protects SavedPage analysis; it must not make valid A-only Source evidence require a terminal report. Current context preparation already returns an explicit unavailable analysis context for a live/mismatching report while independently verifying Source SQL.

## Minimal typed traversal proposal for handshake

Add only a private method in the existing SourceReader child; use the real current types/names:

```rust
impl VerifiedSourceRead<'_> {
    pub(crate) fn visit_measurements(
        &self,
        control: &SitemapReadControl,
        visitor: impl for<'row> FnMut(
            &'row SitemapSourceStoredMeasurement,
            Option<&'row SitemapSourceCommitAssociation>,
        ) -> anyhow::Result<()>,
    ) -> anyhow::Result<()>;
}
```

One ordered measurement LEFT JOIN exact commit (`ORDER BY m.sequence`, all identity joins COLLATE BINARY), using the existing private read_record_row/read_commit_row validators and source limits. Association None means A-only; do not drop the row or infer why B did not commit. Token/connection read-TX checks, actual shared control binding/checkpoints and typed SQL interrupt mapping remain. No borrowed row escapes and no Vec of all records/links is built. Existing all-tail token validation remains mandatory before pagination/predicate. This is a narrow reader extension, no SQL/schema/writer change.

Root's composite finding projector may then use this private input, not a supplied header:

```rust
pub(crate) struct SourceEvidenceReadInput<'a, 'conn> {
    pub(crate) connection: &'conn rusqlite::Connection,
    pub(crate) raw: Option<&'a VerifiedRawRead<'conn>>,
    pub(crate) source: Option<&'a VerifiedSourceRead<'conn>>,
}
// Names for Root handshake, not implemented/public DTO promises:
// SourceEvidenceRef = Measured{record, exact_commit?}
//                  | UnmeasuredXmlDocument{document}
//                  | PendingXmlTask{original_task_url}
//                  | DoneXmlTaskWithoutDocument{original_task_url}.
// Global marker/history/absence belong to SourceEvidenceScope, never a fake row.
pub(crate) fn visit_all_source_evidence(
    input: SourceEvidenceReadInput<'_, '_>,
    control: &SitemapReadControl,
    visitor: impl FnMut(SourceEvidenceRef<'_>) -> anyhow::Result<()>,
) -> anyhow::Result<SourceEvidenceScope>;
```

The composite signature is a minimal proposal to coordinate all actual universes. Root can place it in its forthcoming findings module. It checks !autocommit, exact Raw/Source connection identity and real SourceNone marker/table absence (do not trust a caller-supplied None to suppress an existing Source token). Higher selected guards remain Root's responsibility. The returned owned Scope is available only after full traversal/validation; partial callback buffers must not be published as success if a later task/row fails. Root can instead retain a private prepared source-evidence token if its common pipeline needs a reusable preflight; no mutable verified:true/header shortcut.

The composite uses three actual bounded cohorts:

1. Every retained XML measurement in sequence order, linked or A-only. Existing DTO validation explicitly permits only SitemapXml (`sitemap_source_measurement.rs:350–356`; SQL schema source_kind='sitemap'). A-only has actual protocol evidence and may Match an excess filter even if membership B rolled back.
2. Every retained raw XML Document with NO EXACT commit association. Use one SQL left join/NOT EXISTS on document_id, not expected-ID equality. Raw proofs/limits and document_by_id are available (`sitemap_ledger/verified_read.rs:104,111`), or factor a narrow existing typed document decoder to avoid redundant lookups. Robots documents are excluded from these XML protocol rows. Missing association is not a measurement of zero and is not automatically a specific budget/capture failure.
3. Actual XML discovery tasks: done0 remains pending even when another measurement/document exists for that task; done1 without a raw XML Document remains an explicit unobserved legacy task. A pending task is NOT an actual HTTP attempt; use that distinct discriminator/label. Use binary original kind/url identities, never redirect-final aliases. No all-task Rust Vec is needed: one ordered SQL cursor/NOT EXISTS. Do not query absent Source tables in historical None; that branch must use actual validated table/marker absence.

Task validation MUST reuse/lift the current coverage visitor semantics at `sitemap_coverage/verified_read.rs:103–155` (all kinds known, done0/1, exact legacy normalizer identity, binary duplicate/order check,1M rows/32MiB aggregate text). SourceTask identity is opaque legacy transport identity: do not add generator/Loc2048/percent rules or repair strings. Coverage currently returns early for RawNone at744 onward, so RawNone does NOT automatically provide a validated task inventory. A new task read must run its real full-tail validation and budgets before publishing a successful result, including that case. Retain global policy/config verification; no fake already-verified boolean.

A single unusually long valid legacy task can approach the existing aggregate32MiB task profile; no current per-task8KiB promise exists in coverage. Borrow it where possible, fail a bounded response atomically if unrepresentable, and do not truncate/omit it silently. The8KiB measurement identity cap is NOT the legacy task URL cap. Protocol Loc limits are separate again.

## Evidence precision and unknown rules

Actual public types: SitemapSourceMeasurement{measurement_id,payload}; SitemapSourceMeasurementPayload{binding,transport,decode,scan,membership_retention,bounds}; SitemapMeasuredValue{value,precision Exact/AtLeast/Unavailable}; SitemapSourceScanStatus CompleteProfile/Failed/NotObserved; SitemapSourceDecodeIntegrity Complete/Failed/Incomplete/NotObserved; SitemapMembershipRetention Complete/Incomplete/Unobserved. All strict typed measurement fields/profiles/IDs are validated. Preserve original requested task, final transport alias, capture config SHA/generation, initial catalog anchor and selected historical scope separately.

Use existing `assess_sitemap_source_protocol_limits:871` on each ACTUAL verified record, not on a newly sealed legacy fallback. Return its actual SitemapSourceProtocolAssessment and map ObservedExceeded→Match, WithinLimit→NotMatched, Unknown→Unknown. Missing measurement gets no fabricated protocol assessment object (assessment None plus explicit Unknown reason). Never call Measurement::new to fill absent facts, infer count from retained membership rows, set unavailable to exact0, or infer XML size from HTTP bytes/Content-Length/gzip ISIZE.

Count threshold50000: recognized Sitemap09 namespace+UrlSet/SitemapIndex are required; >50000 eligible completed single-Loc entries can prove excess even on an incomplete prefix. Starts/completed slots/closed Loc occurrences remain separate.50,001 starts without eligible closed Locs do not prove50,001URLs. WithinLimit requires actual full decode and complete structural scan with completed entry upper bound<=50000; it is not full XSD validity. Preserve page-URLset versus index child-entry units and invalid/missing/multiple Loc diagnostics. Legacy unqualified/foreign namespace is explicit Unknown for entry threshold.

Byte threshold52,428,800: actual decoded returned bytes lower bound above it can prove excess even with later failure; a prefix at/below it remains Unknown. Full decode/body-complete HTTP200 can prove WithinLimit even if later XML syntax/namespace is invalid; keep those axes and do not call the source valid XML. Neither received HTTP bytes nor legacy retained bytes substitute for decoder facts. There are genuine positive/within outcomes; no permanent-Unknown workaround.

A record's membership_retention describes the captured source parser batch, not durable catalog commit. Only exact B association proves admission, and its stored Document completeness remains another fact. Multiple file-gzip captures can share one expected legacy document ID; only their exact measurement/link tuple associates B. A-only alongside another linked capture must not be attached by shared expected_source_observation_id or task URL.

`SourceStoreSummary.omission` is the FIRST FIXED global marker only (`source_store.rs:1180–1205`); CaptureUnavailable/StorageBudgetExceeded identify neither source URL nor omitted count and are not a per-attempt log. Later missing rows cannot inherit that cause. Keep it in Scope with omission_revision, legacy_history_unknown, optional inherited basis and actual observed counters. No fake 'omitted attempt1' row, no total attempt count from record_count. Some(empty) known source store, historical None, pending tasks, done-without-document and unassociated legacy documents remain distinct. Even all retained rows exhausted cannot prove the globally omitted source universe empty/complete.

Scope should expose retained measurement count, exact commit count, A-only count, unmeasured retained XML document count and observed task-state counts separately; these are observed local cohort counters. Global unmeasured attempt total stays unavailable where history/marker exists. Pending Robots may declare future XML tasks, so XMLtask0 never independently proves a complete declared-source cohort. Reuse existing coverage scope rather than invent another completeness flag.

## Context, ordering and costs

Current Source context SHA hashes the ENTIRE verified summary (`source_store.rs:569`), preserving None as historical-unknown. A-only changes content/revision without Raw revision; B-only changes links; marker-only changes summary/revision. Current shared header also binds actual config/generation, inventory/coverage, terminal analysis context and inherited anchors. Reuse it; Source findings must remain available when SavedPage analysis is unavailable. Do not require a terminal-report-positive boolean for Source rows.

Measurement.sequence, raw Document.sequence and pending task binary URL order are distinct domains. A new source-finding cursor must include an explicit domain/phase+original key bound to canonical filter, actual shared context and engine; never reuse raw after_sequence or silently coalesce task/measurement identities. Huge task identities may require a bounded opaque digest anchor that is resolved against the same actual validated inventory, not a copied unbounded URL token. Root's new DTO/cursor handshake decides the exact format.

Ordinary traversal can stay one bounded record/link/document/task at a time plus a bounded response window and SQL sorting/join work. No SourceSnapshot32MiB Vec, whole raw catalog or all-task Vec is required. One joined SQL cohort is preferable to querying all measurement records for each document/task. Imported schemas cannot assume original UNIQUE/index/collation constraints; retain existing duplicate validators and explicit BINARY joins. No physical RSS/query-plan/latency result is asserted. Selected parent validation still materializes its bounded historical report/catalog; unchanged separate S4 gap. No schema migration or HTTP request is required for this reader/projection slice.

## Existing fixture/helpers to reuse after handshake and HOLD

- `crates/core/src/sitemap_source_store_tests.rs:17 fixture`,54 normal,64 source (actual observed decoder/count scanner+legacy parser),194 persist(A),204 commit(B),220 snapshot. Exact gzip same-ID case365; crash resume685; global omission/unknown492/722; read-only history550; selected parent587. Core writer/SQL test fixtures, not a full public Crawler claim.
- `crates/core/src/sitemap_source_store/verified_read_tests.rs:8 compare_in_tx`,52 empty/A-only/B,79 both fixed marker types,126 late unjoined tail,166 exact raw seals,219 weakened imported uniqueness,274 None/empty/partial schemas,295 selected_child,337/361 selected Some/None exact guards. This child reuses the real parent Fixture, not fake tokens.
- `crates/core/src/store-sitemap-source-tests.rs:19 fixture`,25 fixture_with_measurement_limits,96 batch,152 measurement,175 a,198 b,263 snapshot,270 report. Actual ordinary rollback293, policy rollback327, WAL451, identity/gzip493, capture-unavailable527, legacy read-only567, selected608/931/1002, A-only shared Raw/rich state1049/1137. These are real Store integration fixtures.
- `crates/core/src/store/sitemap_stored_source_tests.rs:7 captured_header`,17 repeated stored session A-only/B/stale callback boundary; child uses those real Store fixtures. Current common callback/proof/context regression source.
- `crates/core/src/crawler/sitemap_capture_tests.rs:64 BinaryFixture`,148 start,223 setup,255 context,271 collect,283 measured,292 assess. Actual plain/file-gzip/HTTP-gzip303; received-prefix/206/robots403; CRC/truncation495; real50,001 counts570; namespace/Loc/DTD632; actual config700; corrupt snapshot906; long task948. These are actual fetch wrapper/loopback controls; all execution still held.
- `crates/app/src/sitemap_source_context_tests.rs:52 ActualSidecar`,159 make_legacy_source_history_fixture,205 crawl_config,230 actual_pending_capture_from_same_config,289 actual_tokens; actual A-only/association/marker restoration390/421/451, historical502, selected542. Positive capture comes from public Crawler; explicit restored SQL is a labelled context test, not natural production history.
- `crates/core/src/sitemap_source_measurement_tests.rs` is the pure threshold/consistency oracle; model-only fixtures do not replace native fetch/Store evidence or the physical S5 matrix.

Directed follow-up tests (PLANNED ONLY): positive source after200 with filter-before-page; A-only over-threshold despite B rollback; two gzip captures/shared expected ID with only one exact link; linked versus unmeasured XML and Robots exclusion; historical None vs known empty; global marker without fabricated per-task cause/count; late pending task and done-without-doc; source record/commit/marker-only cursor staleness and WAL same-TX snapshot; selected Some/None parent tamper; exact/AtLeast/Unavailable count+byte axes and invalid namespace/start-only controls; read-only/noHTTP and large task response-budget controls. No test count is incremented by this audit.
