# CSV cooperative reader Control — authored source handoff

Baseline: `sitemap-csv-streaming-integration-v1`, Root HEAD `d105fb8a0c527ffd99164e44ec644bd0e49f61a5`. This directory alone was written. Root repository, Git, services, old immutable sources and other agents' payloads were not changed.

## Contract

Root registers `mod sitemap_read_control;` and publicly reexports `SitemapReadControl` and `SitemapReadControlError`. No dependency or feature changes. `SitemapReadControl` is Clone+Default. Public methods exactly: associated `with_deadline(Instant)`, `cancel`, `expire`, `set_deadline(Option<Instant>)`, `checkpoint()->anyhow::Result<()>`. Error variants are `Cancelled`/`DeadlineExceeded`, Copy+Debug+StdError. Clones share the stop and deadline. The first observed stop is sticky; resetting a deadline cannot restart a stopped operation. Default is never cancelled/no deadline.

Private real-connection `bind_connection(&Connection)->Result<SitemapReadConnectionGuard>` owns the Control clone and monotonically assigned attachment ID. It holds no Connection borrow/lifetime/PhantomData. The mutex-protected attachment map holds actual rusqlite InterruptHandles obtained from that Connection. Guard Drop removes only its own ID. Post-attachment failure drops the newly made guard. This is no snapshot proof: Root still owns the actual Connection/read Transaction and must prohibit same-connection writes/restart while reader tokens live.

All four controlled private entry points retain their unchanged return types:

- `sitemap_ledger::verified_read::verify_raw_with_control<'conn>(&'conn Connection,&SitemapReadControl)->Result<Option<VerifiedRawRead<'conn>>>`
- `sitemap_source_store::verified_read::verify_source_with_control<'conn>(&'conn Connection,Option<&VerifiedRawRead<'conn>>,&SitemapReadControl)->Result<Option<VerifiedSourceRead<'conn>>>`
- `sitemap_coverage::verified_read::project_read_with_control(&Connection,Option<&VerifiedRawRead<'_>>,CoverageReadContext<'_>,&SitemapReadControl)->Result<VerifiedCoverageRead>`
- `sitemap_membership_analysis::prepare_window_analysis_with_control<'a,'conn>(&'conn Connection,&AnalysisReadInput<'a,'conn>,&SitemapReadControl)->Result<VerifiedAnalysisRead<'a,'conn>>`

Raw token privately retains a Control clone for later visit/document/window operations. Prepared analysis privately retains the shared clone for `project_prepared_window`. Source token getters remain inert; controlled Report verification receives an explicit Control. Existing default wrappers and exact bytes/context/ordering/guards remain, including the test-only four-argument `read_target_pages` helper used by an existing regression.

Private Control helpers `run`, `map_result`, `sha256` are available crate-wide. `map_result` only restores a current typed stop across actual SQLITE_INTERRUPT, an existing typed Control error, or the fixed internal Control message erased by serde custom conversion; unrelated corruption/SQL/I/O errors remain errors of their original kind. Report array SQL and Coverage task SQL map actual errors BEFORE serde erases their type. A stopped successful operation returns a typed stop rather than a success prefix. SQL interruption without a stopped Control remains a rusqlite error.

## Actual checkpoints and limits

Real row visits include raw verification/filtered pages/entry-location relation verification; source record and commit loops; coverage roots/tasks/graph queue/edges; target, page, membership counts and inlink loops. Exact catalog/page serializers, exact coverage inventory hashing, report-source lexical whitespace/span/skipping passes and source array equality visitors checkpoint. Actual saved-config and report SHA computation hashes 64KiB chunks with checkpoints. Control adds no raw/XML/report clones and does not change caps, hashes, raw facts, policy, selected ancestry or source validation.

Interrupt handling was checked against the actual locked local rusqlite0.39.0 source (`get_interrupt_handle`, `InterruptHandle::interrupt` and close-pointer clearing), and libsqlite3-sys0.37.0 ErrorCode::OperationInterrupted. No new feature is required. Handles may safely outlive a closed Connection because rusqlite clears its shared pointer. Clone-shared Control state lock recovers only its own poisoned mutex.

This remains cooperative, NOT a hard latency/RSS/query-plan guarantee. No timer is spawned. Root/App must use its owned watcher to call expire/cancel while SQL is blocked. SQLite interrupt is not persistent before a query starts; the reader checkpoints immediately before operation/loops and after results, but that pre-step race has no hard latency bound. Existing selected/import/config/policy materializations and serde's bounded unrelated `IgnoredAny` value parsing keep explicit coarse pre/post boundaries. Existing SQL sort/JSON working storage remains SQLite-owned. Source/report lexical passes check at64KiB offsets, while bounded field reads keep their original guards. No S4 completion claim.

## Directed authored regressions

Ten new test functions in `sitemap_read_control_tests.rs`, hooked only by the new Control module:

1. Default/shared clones/sticky first observed stop.
2. Shared movable/clearable deadline and typed expiry.
3. Poison recovery for own state mutex.
4. Actual owned two-connection attachments; own-ID cleanup; mutable transaction/close allowed with guard alive.
5. Actual recursive SQLite query interrupted through attached handle; assert real SQLITE_INTERRUPT, then SQL→serde custom→outer typed mapping; unrelated and externally supplied interrupt errors preserved.
6. Exact64KiB-chunk hash and typed stop/custom-error restoration.
7. Actual CrawlStore initialization with explicit empty roots; all old/default vs controlled raw/source/coverage/analysis results equal on a read-only connection and no writes.
8. Actual initialized root row: visitor cancels after its first callback, no successful prefix/window follows.
9. Both pre-stopped reasons on all four APIs plus controlled Report before any missing-schema SQL.
10. Actual prepared empty-Store analysis/raw proof retains shared later expiry.

All functions are AUTHORED, NOT COMPILED, NOT RUN. The SQLite interrupt fixture deliberately enters the actual SQL before checking stop, solely to prove the real handle in a test. It repeatedly notifies to cover interrupt-before-step, owns an emergency handle and joins its worker; no test execution occurred.

## Validation performed

Only source reads/transforms, guarded hashes and standalone rustfmt with `--edition 2024 --config skip_children=true`, followed by `--check`, were permitted/performed. No Cargo, compiler, tests, fixture execution, SQL connection, native run, website request, browser, SDK or large generator. Rustfmt's parser/format success is not type/borrow/Clippy proof. Root must integrate lib registration and Store/App callers separately, then run held gates after release.
