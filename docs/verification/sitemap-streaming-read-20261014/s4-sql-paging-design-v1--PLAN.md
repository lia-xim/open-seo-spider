# S4: verified SQLite paging and CSV without full raw-row materialization

DESIGN ONLY / NOT_RUN. No implementation or Root/Git/runtime mutation. No Cargo, compiler, tests, SQLite fixture, HTTP, source generator, native or browser execution. The actual source files read are pinned in source-pins.json. Root's large-source worktree was mutable; hashes describe the bounded read, not a new product freeze.

## Decision and acceptance boundary

Add a new transaction-scoped verified reader and an owned compact read result. Do not turn a page into a fake SitemapCatalog with rows=Some(window), or mark rows=None as a complete catalog. Keep the existing materializing portable validator/import API as an oracle and compatibility path. Changing sitemap_ledger::page alone does NOT fix the actual HTTP path: it is not called by server/sitemaps.rs; the server loads SitemapReadSnapshot, and App inventory/CSV filters its complete catalog.

The raw-row validator/page/export can use O(D + W + R) Rust space, where D is compact document counters, W is the bounded selected window, and R is one bounded raw/policy/measurement row. Complete current rich-API context does NOT yet have that bound: saved config/imports (C), tasks/roots/edges (T), selected plan/parent (P), and report source envelope are separate existing allocations. The honest intermediate bound is O(C + D + W + R) for an ordinary context, with existing selected/coverage paths potentially adding O(P + T). Removing them is part of the necessary end state below, not permission to omit their checks.

## Actual paths and invariants that must survive

| Source | Current materialization / contract |
|---|---|
| sitemap_ledger.rs:1407-1522 | Preflight row size/count; then UNION ALL global-sequence cursor builds all rows; canonical JSON/SHA/SQL-column equality; full validate_catalog, inherited anchor and current-policy checks. |
| :747-1041 | IDs across all four tables, unique source tasks, prior document/entry references, spans, synthetic missing-Loc qualification, exact admission identity/child alias/policy revision, exact entry-to-Loc multiset, per-document ordinal/count order and robots declarations. Uses entry/Loc maps and vectors in addition to the full row Vec. |
| :77-89, :531-554, :1043-1094 | Exact existing tagged rolling hash, sequence/counters, summary equality, revision bounds, metadata flags, and full canonical serialized catalog byte count. SUM(data bytes) alone is insufficient. |
| :1294-1405 | Policy-bound rows must still match actual committed policy data; original config/import root coordinates remain verified. Inherited IDs intentionally bind the parent rather than current policy rows. |
| :1533-1589 | Exact parent BLOB SHA, typed parent catalog SHA, projected inherited envelope including recomputed serialized size, and full child equality. |
| store.rs:2290-2512 | Actual full saved config/import/run-plan verification, selected rows and mandatory ParentSome/ChildNone presence guard, same-TX source/coverage state. None remains historical unknown. |
| source_store.rs:270-441, :445-515, :760-841 | Canonical measurements/links and SQL projections, sequence/profile/payload/measurement binding, actual raw-document seal, rolling hashes, exact byte accounting, link reserve and omission marker. Current load builds records+commits and repeatedly loads raw catalog. |
| App sitemap_membership.rs:349-409, :471-620, :703-830 | Job config/generation binding; state SHA includes summary/inheritance/coverage inventory/derived coverage/source context/engines/analysis context; strict role/filter/stale cursors; raw filter/page/lastmod; CSV exact existing columns, quoting and formula protection. |

Reusable now: policy_ledger::verify(:508-577) already iterates rows and recomputes its ledger digest. analysis::report_hash(:301-327) already hashes a borrowed SQLite TEXT value with no Rust report clone. SourceStore snapshot_context(:503-515) hashes validated summary only, so its v1 fingerprint can remain byte-identical after a streaming summary rebuild. Do not replace it with a new guessed hash.

## Proposed implementable private/public boundary

Private, non-Clone, non-serializable token, constructible only after full validation inside one actual transaction:

```rust
struct VerifiedSitemapRead<'tx> { tx: &'tx Transaction<'tx>, header: VerifiedHeader, /* private */ }
// Illustrative lifetime only: implementation must fit actual rusqlite connection lifetime.
fn with_verified_sitemap_read<T>(
    store: &CrawlStore,
    f: impl FnOnce(&VerifiedSitemapRead<'_>) -> Result<T>,
) -> Result<T>;
```

The Store owns the lock/connection/transaction throughout validation, context construction and selection. The callback cannot return a row iterator or token that outlives the transaction. Do not open a second connection for config, coverage, source links or target analysis.

Public owned DTOs should be new types, not truncated legacy catalogs:
- SitemapVerifiedHeader: catalog availability plus optional summary/inherited basis; actual config-byte SHA/generation; verified source-store summary/context; compact coverage and captured-inventory fingerprint; optional terminal-analysis context; explicit context/read engine.
- SitemapReadWindow: header, bounded raw rows, last matching sequence/has_more, optional independently bounded target/page analysis and visible-entry lastmod assessment.
- SitemapCsvReceipt: verified header/context, exact bytes/hash/row count. The full exporter accepts a Write sink; no all-CSV Vec in Core.
- Public read_sitemap_window(request) and write_sitemap_csv(request, sink) run the private session internally. Expected App config/generation and decoded cursor context are request constraints, not authority supplied as a fake verified header.

No public constructor or deserializer should mint the private verification token. All errors remain Result failures.

## Raw verification algorithm

1. Read summary/inherited metadata with existing bounds and exact absent-marker/table-empty/partial-schema rules. Preflight SQL row lengths/counts before owning JSON; keep row/catalog ceilings unchanged.
2. Iterate the exact current four-table UNION ALL in global sequence order. For every row, check data SHA, parse the one bounded typed row, stream canonical serialization against actual data bytes, compare all projected SQL columns and sequence. Recompute the existing roll using canonical actual bytes and all existing counters. Canonical comparison can use a comparison writer rather than a second full row String.
3. Retain only compact per-document facts/counters, not entire document bodies: observed source ID/sequence/kind, completeness and expected/next entry/location/robots counts. Other needed document values can be fetched by its indexed row_id when checking a row.
4. Check source task duplicates and IDs across table kinds without an all-row Rust set. Use indexed cross-table ID point checks or a single SQL duplicate/group check. Do not merely assume constraints exist: current validation detects duplicates even when a database was externally rebuilt with weaker DDL.
5. Preserve prior-reference semantics: referenced document/entry must exist in the same verified snapshot and have smaller global sequence. A single entry row lookup by row_id is enough per Loc; document summary is cached or one point lookup. Enforce per-document ordinal progression with counters, and compare final counters to parser/declaration counts.
6. Preserve entry loc_occurrence_ordinals as an exact multiset, including duplicates in a declared list; real Locs only contribute, synthetic missing rows do not. Use a second ordered relational cursor grouped by entry_id and global Loc ordinal, comparing to one entry's bounded sorted declared list. Do not issue an unindexed full location scan per entry. Current schema has no locations(entry_id,ordinal) index; an optional writable-only index can help. A legacy read-only DB must still work without DDL: use one ordered join/sort/merge pass, not O(E*L) per-entry scans. SQL working-set/RSS remains a separate measured bound.
7. Current policy checks become a one-row validator receiving the current row plus compact document lookup; reuse original-coordinate and committed-policy validation. Do not reparse the complete saved configuration for each raw root.
8. Rebuild exact whole-catalog serialized size with a serializer adapter for schema_version/summary/inherited/rows, streaming each row. Reuse serde ordering/escaping and the existing typed representations, not a guessed fixed envelope size. Compare actual bytes and rebuilt summary exactly; retain revision, flags, inheritance and unknown-history checks. No raw row can reach a successful page until the tail and all relation checks pass.

This is O(N) scanning per session, with constant-number indexed lookups and ordered secondary passes. It is not O(window) time. A header checksum cache alone is unsafe: unchanged summary metadata does not prove an unmodified corrupt tail outside the requested page.

## Stream source-sidecar verification in the same session

Preflight the two sidecar tables and fixed metadata exactly as current load. Iterate measurements in sequence, validating one <=16KiB envelope, canonical bytes, SQL columns, initial anchor and strict typed measurement/hash. Rebuild content hash and exact record bytes without collecting records.

Iterate links in document_sequence order; validate duplicate/document/measurement identity and each canonical link. Fetch the referenced one measurement and actual raw document by indexed IDs, compare measurement-envelope hash, actual canonical raw document payload hash, staged document seal and bind_record_document. A-only records remain valid and distinct from commits. Verify summary record/link counts, revision, omission marker, content/link hashes, charged/logical/reserved byte accounting and link reserve with an accumulator equivalent to refresh. Never drop the fixed unknown/budget marker when capacity is exhausted.

Compute the existing snapshot_context from that fully rebuilt/verified summary. This removes records/commits Vec and raw load recursion from the read path. It does not alter the writer receipt lifecycle, immutable measurements, raw rows or stored hash profiles.

## Exact selection and full CSV

After validation, execute a parameterized SQL window with the existing filter's exact semantics and BINARY string equality:
- Root source = observation_id (nullable); member = $.row.admission.membership_url.
- Document source = row_id; no member.
- Entry source = observation_id; no member.
- Location source = observation_id; member = $.row.admission.membership_url.
The JSON envelope is {kind,row}; using $.admission or $.observed at the top level is wrong. Original membership is not the admitted alias or final redirect URL.

Select sequence > after_sequence ordered globally, with limit+1 for has_more. Before a continuation, require the after_sequence row to exist AND match the same filter, not only be <= last_sequence. Reuse all current query validation and role-bound cursor checks. Bound window JSON bytes while collecting, then run the final exact response-size check; count + byte bounds remain independent. Lastmod runs on visible actual Entry rows only. Target joins receive the identical selected window in this transaction.

CSV needs no new large spool:
1. Full validation/context/token checks.
2. First filtered row traversal writes the existing CSV encoding to a Count+SHA writer, enforcing unchanged64MiB output cap. No response headers/output yet. This guarantees validation/cap errors are returned before success.
3. Second traversal in the SAME read transaction uses the same formatter and a bounded writer into fixed-size chunks (for example64KiB, channel depth2). Return expected Content-Length and actual preflight SHA in headers. Core can expose a synchronous Write API; App runs the owning transaction on one blocking worker and uses blocking_send for backpressure.
4. App must own and join/drain producer completion on success or client cancellation; drop transaction/connection on every path. No detached blocking producer. The channel carries only bounded owned chunks, no rusqlite references. Add an explicit bounded session/export deadline and cancellation/SQLite interrupt path, so a stalled client cannot retain a WAL read snapshot indefinitely. No unbounded queued chunks or retry loop.
A disk/read/channel failure AFTER HTTP headers cannot become a clean JSON error or an atomic full HTTP download. Terminate with a body error; expected length/hash lets the consumer reject incomplete data. Do not claim zero partial network bytes. CLI can use an atomic create-new destination; callers requiring full atomic HTTP artifact availability need a separately agreed bounded spool, not a silently introduced unbounded buffer.

The existing csv_record/column mapping/csv_text sanitizer should be factored and reused byte-for-byte; do not duplicate a slightly different CSV implementation.

## Rich context and selected parent are necessary remaining slices

Keep one shared context gate; do not call current load_sitemap_read_snapshot_verified from the new reader because it rematerializes raw/source rows. Refactor the gate to consume verified header/doc lookup and streaming sources.

Coverage currently materializes tasks/expected roots/child edges (coverage.rs:44-52, :59-137, :480-523, :660-705; initial_roots.rs:15-75). Pending tasks may greatly outnumber observed documents. Therefore O(D+W) is not true if that inventory is simply retained as a Vec. A streaming initial-root visitor, ordered task reader and SQL graph traversal/dedup (or explicitly bounded separate graph state) are required. Hash the same captured inventory serialization through a writer; do not substitute an empty task list. Preserve complete/known-empty/unknown reasons, cycles, invalid child edges, actual initialized/config/generation and inherited historical scope.

App's current state hash serializes full inventory. Either reproduce its exact serialization via a stream adapter or introduce an explicit new opaque cursor profile/engine that binds the verified inventory SHA plus the same coverage/source/analysis state. Recommended: version the new compact read/cursor context and reject old-profile continuations as stale with a restart message. Do not silently accept old tokens under changed hash meaning. Stored catalog/source hash engines remain unchanged.

analysis::project can reuse report_hash and its existing SQL page/inlink column readers (:702-936), taking only the selected row window and compact verified header. validate_report_source_measurements(:334-369) currently allocates the entire projected envelope; replace that comparison with a typed streaming envelope/records/links visitor checked against the already verified SQL source rows, preserving unknown-field/duplicate/type rejection and whole typed equality. A report-only rehash cannot mask conflict.

Selected read is NOT solved by a metadata parent hash:
- Store load_run_plan/verify_recrawl_rows(:2575-2650) and recrawl::validate_saved_plan reconstruct full parent bytes/CrawlReport today.
- The new selected context validator must still hash the actual bounded parent BLOB, reconstruct the complete saved-plan/config/selection contract, validate every selected/inherited row and compare parentSome/childNone presence for BOTH raw catalog and measurements.
- A borrowed SQLite BLOB plus a DeserializeSeed/visitor can validate parent fields/pages one at a time, compare selected targets via indexed tables, and compare parent catalog/source streams to the verified child projection. Preserve the exact parent catalog/source SHA and projected envelope sizes. Keep original parent policy scope; do not demand absent historical policy SQL in the child.
- Until that read-only selected validator has equivalent tests, using the existing full validator is an explicitly materializing compatibility path. It cannot be labeled the final no-report-allocation S4 path.

Config/import C is also independent of D. Existing fully typed config/import validation and compiled-policy setup can remain initially with its documented64MiB bound, but this adds O(C) Rust memory; strict complete-context O(D+W+R) requires a bounded import/root visitor or separately accepted C bound. SQLite itself may materialize/sort/parse large TEXT values. Rust-space asymptotics are not an RSS guarantee.

## Integration order / proposed handshake

A. Extract shared one-row validators, raw/source verified stream accumulators and private same-TX session; implement raw window + writer/export primitives, with old full validators as semantic oracles in authored tests. No API switch before equivalence review.
B. Refactor context/coverage/analysis envelope and selected-parent readers so the full reader avoids raw/source/report Vec, preserving all guards. Clearly document any temporary C/P/T compatibility cost.
C. Switch actual server inventory/CSV and App bound_catalog to compact window/header, retaining response semantics and explicit cursor migration. MCP spider_sitemaps already forwards the inventory query and can reuse the API. There is currently no CLI raw-sitemap-ledger command in main.rs; a CLI writer command is separate explicit scope, not an existing workflow to claim.
D. Prove real memory/read behavior and whole path under released compute. No S4 closure from merely replacing the unused ledger page helper or preserving32MiB full Vec under a streaming name.

Root supplied the separate future integration target after this read: E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-streaming-integration-v1 at HEAD380beb1ccacf73659d0ac8dd7fdb39c543b88883. The source pins below remain the actual S4 tree read; no implementation or target mutation was authorized here.

Recommended ownership/contract split for the first handshake:
- Raw Core owner: new sitemap_read_core.rs/models/tests, with narrowly shared pure helpers from sitemap_ledger.rs. Publish private verify_raw(tx)->Option<VerifiedRawRead<'tx>>, select_window(&verified,selection), and visit_filtered(&verified,selection,visitor) contract first. Global-ID duplicate UNION check and ordered entry-Loc relation pass are mandatory. No schema mutation is required for the first correct path; an optional index is a separate Root-owned writable upgrade.
- Source Core owner: new sitemap_source_read.rs/tests plus narrowly shared helpers from sitemap_source_store.rs. Consume VerifiedRawRead/document lookup; return a verified source summary/context after streamed record/link validation. Do not guess a public raw header or call full load_catalog/load sidecar internally.
- Root/context owner: store.rs transaction facade and public exports, then separate read-only context/coverage/selected-plan modules. Own the actual same-TX lifetime, no-fallback presence guards, compact read header and full rich-context convergence. Existing portable/writer paths remain unchanged until separately reviewed.
- App owner after contracts freeze: server/sitemaps.rs + sitemap_membership.rs for actual compact-window binding/role cursors, exact CSV formatter extraction, bounded backpressure transport and producer lifecycle. MCP query forwarding can reuse the route; preserve tool bounds and do not add an oversized CSV JSON response.
Raw Core-first is a reviewable increment only. Source-store, actual coverage/selected-parent/analysis context, App hookup and physical proof remain full S4 gates. All ownership must be assigned by Root before code changes.

## Directed regression gates for implementation

- Compare old full validator and new stream on real existing SQLite catalogs: every row kind, roots/imports/robots, namespaces/loc diagnostics, all policy decisions and inheritance. Exact summaries, roll hashes, full serialized byte count, CSV bytes/hash and every paged sequence/filter must agree.
- Corrupt tail outside requested page, cross-table duplicate ID/global sequence, preceding/missing doc/entry, per-document ordinal reorder, wrong SQL projection/hash, changed payload with stale summary, entry declared Loc multiset, synthetic missing Loc, child alias, policy coordinates/committed row; reject before any successful JSON/CSV headers.
- Raw/source absence/empty/partial schema, A-only change, wrong link/envelope/document seal, omitted fixed marker/reserve boundary; cursor becomes stale when required even if raw revision is identical.
- WAL writer while read session/page/CSV is held; each uses one snapshot. Next session sees new state. No per-page reopened context reads.
- Selected real Some and historical None parent, delete-all child markers/rows, rehashed parent/report tamper, changed config/selection/retained page; no weakened presence guard.
- Real max-row/window JSON limit and64MiB CSV exact/+1; all escaping/formula/raw-field bytes preserved. No partial successful output on preflight failure; late I/O body failure, channel backpressure/cancel/drop, close/drain all workers.
- Index-absent legacy read-only catalog remains immutable and uses ordered relation pass; avoid E*L scans. Instrument actual raw-row/record/lookup/window high-water counts. Physical RSS includes SQLite and must be measured separately.
