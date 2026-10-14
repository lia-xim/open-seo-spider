# Guarded compact analysis handoff

AUTHORED_NOT_COMPILED_NOT_RUN. Basis Root rich worktree HEAD declared 5091eb941c6740bfb2a6bf610388bc9ca6fb081b. Only this handoff directory was written. Root owns integration, Store/App bindings, cursor comparison, selected guards and promotion. No Cargo/compiler/fixture/test/HTTP/native/browser or large-generator run occurred under compute HOLD. Rustfmt parser/format checks on five changed/new implementation/test files exited0; the historical test file was preserved as exact raw bytes plus its approved child hook.

## Exact phased API

Reexported crate-private from sitemap_membership_analysis.rs:

```rust
pub(crate) struct AnalysisReadInput<'a, 'conn> {
    pub(crate) raw: Option<&'a VerifiedRawRead<'conn>>,
    pub(crate) source: Option<&'a VerifiedSourceRead<'conn>>,
    pub(crate) generation: Option<u64>,
    pub(crate) crawl_config_sha256: Option<&'a str>,
    pub(crate) coverage: &'a SitemapCoverage,
}
pub(crate) fn prepare_window_analysis<'a, 'conn>(
    connection: &'conn Connection,
    input: &AnalysisReadInput<'a, 'conn>,
) -> Result<VerifiedAnalysisRead<'a, 'conn>>;
impl VerifiedAnalysisRead<'_, '_> {
    pub(crate) fn context(&self) -> &SitemapAnalysisContext;
}
pub(crate) fn project_prepared_window(
    prepared: &VerifiedAnalysisRead<'_, '_>,
    raw: Option<&VerifiedRawRead<'_>>,
    coverage: &SitemapCoverage,
    visible_rows: &[SitemapLedgerRow],
    request: &SitemapMembershipAnalysisRequest,
) -> Result<SitemapMembershipAnalysis>;
```

VerifiedAnalysisRead fields are private; no Clone/Deserialize. The prepare phase validates the terminal report/checkpoint/source context once. Root must compare its actual state/cursor before selecting raw continuation rows, then pass those actual bounded rows and the exact same Raw proof and equal coverage into projection. The projection checks active transaction, pointer identity, coverage equality, row count/order/request matches and raw presence. It does not redo the report/source verification and does not revalidate an after-selector before the Root state comparison. Caller must retain the same read transaction and perform no writes or transaction restart through that connection while proofs live. An ended TX is rejected; restart/write detection is not an unforgeable SQLite snapshot-id claim.

Root must retain the existing full selected-plan/parent/frontier/raw/source presence guards. Raw/Source selected tokens still incur explicitly materializing compatibility; this slice does not remove that gate. Compact coverage comes from finish_coverage's actual same-TX projection, passed as &SitemapCoverage. Coverage basis initial config/revision/rolling hash is checked against Raw; Root owns inventory SHA and rich state preimage.

## Behavior and ownership

Six promoted payloads, three guarded existing files and three new files:

- sitemap_membership_analysis.rs: existing project now uses shared prepare_context and project_rows. It retains a materialized-membership oracle and full historical source-snapshot validator on the old read path. Target observation states, exact original versus admitted alias identity, render evidence, pending states, saved-page independent ordinal paging and inlink SQL are shared with compact projection. The PAGE_COLUMNS/page_from_row/query/inlink tail is exactly equal after newline normalization. An explicit input-generation versus actual run guard now qualifies a mismatched generation as saved_checkpoint_context_mismatch.
- sitemap_membership_analysis/window.rs: new phased proof and compact SQL whole-cohort membership. For <=200 saved-page requested URLs, constant-number batched SQL scans compute full occurrence/alias counts, distinct requested XML task counts, observation counts, and <=5 task/observation samples in BINARY order. Raw document_by_id supplies sample provenance. It does not retain all locations/documents, all source sets or an E-times-P Rust scan. Saved-page membership is independent of visible rows/source/member filters. The legacy path keeps the original full-row sets as an oracle only.
- sitemap_membership_analysis/window_tests.rs: ten new actual PublicCrawler/temporary SQLite groups, beneath existing private fixture module.
- sitemap_membership_analysis_tests.rs: exact original raw-byte prefix plus ONLY the approved path/module child hook. No historical assertion changed.
- sitemap_source_store.rs: ONLY `pub(crate) mod verified_report;` added; no writer/admission change.
- sitemap_source_store/verified_report.rs: borrowed actual report serde visitor/seed, strict source envelope and ordered one-at-a-time SQL comparison. Source verified_read.rs is unchanged and is only a pinned context copy, not a payload.

No complete SourceSnapshot/report/PageRecord vector, raw catalog or fake SitemapReadSnapshot is constructed on the new ordinary analysis path. Bounded visible rows and wanted target/page/inlink results are retained. SQLite report/JSON projection/sort/group/window/cache working storage and serde scratch are not an RSS or performance proof. The old materialized path remains intentionally materialized.

## Saved report source envelope

The actual report remains capped256MiB. Its JSON1 source-envelope projection retains the existing64MiB read cap. Source presence must be actual SQL Some/object or historical None/absent-or-null. The complete actual report map is streamed; duplicate source keys, malformed suffixes and a non-object report fail. Unrelated report values use IgnoredAny; report field names longer than1024 bytes get a constant error without cloning/echoing an unknown field name. Small unrelated outer report fields remain tolerated as in the prior report reader.

The source object requires exactly schema_version/summary/records/commits and rejects duplicate/unknown keys and wrong types. Typed summary must equal the verified summary. Every ordered saved record and commit must equal the corresponding actual canonical SQL row, with all SQL identities/row hashes checked by shared decoders. Missing/extra/reordered/tampered rows fail even when a report/checkpoint SHA is recomputed. No report-only summary substitutes for full equality.

A borrowed lexical preflight runs before typed allocations, over the actual bounded JSON1 envelope projection. It counts all bytes inside strings, including whitespace and escapes; only whitespace outside strings is ignored. Each record gets 6*actual durable record cap+1024 encoded bytes, each commit6*1024+1024, summary6*4096+1024; the factor admits equivalent Unicode escapes and pretty formatting rather than changing any durable limit. Source field names are bounded1024 with constant errors, and nesting128. Final serde decoding validates syntax, field types/duplicates/required fields and the whole report end. JSON1 and serde internal storage remain expressly unmeasured.

## Ten authored regression groups, all NOT_RUN

1. Actual sibling XML sources, duplicate locs, all raw kinds, exact source/member filters and a one-row visible window versus whole saved-page membership; old oracle equality.
2. One actual prepared context reused across independently paged saved URLs with incomplete source negatives qualified; exact ordinal coverage and old oracle equality.
3. Actual policy rewrite alias versus original and real observed nonself/self inlinks.
4. Actual report records/commits/stage/summary tampered and checkpoint SHA recomputed; SQL equality still rejects.
5. Duplicate source/snapshot keys, unknown/type/trailing/oversized typed values and oversized unknown report key controls.
6. Valid pretty/escaped equivalent source plus a large unrelated ignored value accepted.
7. Wrong actual connection/source/raw proof, changed coverage and ended TX rejected.
8. Actual selected recrawl preserves historical membership and mixed current/inherited age, with old oracle equality.
9. Historical SourceNone absent/null allowed; actual original report source bytes cannot invent missing SQL provenance after rehash.
10. Actual checkpoint source-summary mismatch stays unavailable with no findings.

Existing render/navigation/missing-evidence regression group and all prior analysis groups remain unchanged and required. These are authored controls, not executed evidence. Actual query semantics, Rust type/borrow/visibility, source lexical cases, all old/new tests, Root rich facade/App cursor/CSV convergence, selected compatibility, platform/RSS/performance and installed runtime remain open gates.
