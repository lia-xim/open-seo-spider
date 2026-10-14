# Bounded independent S4 SourceReader review

Result: no confirmed P1/P2 finding in the examined immutable bytes. Static review only; compiler, Cargo, tests, SQLite fixture execution, HTTP, native App, browser and large-generator runs were NOT_RUN under the active compute HOLD. This is not a compiler, runtime, memory, S4-completion or manufacturer-parity PASS.

Basis: source handoff manifest 06ea077f06f4233c47575b0cfeafcf7fa732b6952307b2958c775e43e81c7073. Exact paths, byte sizes and SHA-256 pins are in SOURCE-PINS.json. All five manifest entries match their declared bytes. The three promoted Root product files match the immutable handoff; the actual Root test hook is f745be9102866d24273f0d192e49e971814dbf5d0dd4768785b9b22456d580a4. Peer Raw manifest 14221c867291188534c0f5d7e9630ebc1250cbeccd3653266bdc02214441095c was read only for the called type/method signatures, not independently re-audited here.

## Examined behavior

- New child verified_read.rs, all 262 lines: private proof fields and API (13-31); actual saved-plan/parent-bytes projection and FullCrawl/Selected dispatch (38-89); Raw initial/inherited/unknown binding (91-110); explicit SQL duplicates (115-127); transaction and actual connection-identity preconditions (132-145); Selected compatibility and Option-presence equality before any None return (146-166); canonical summary/partial-schema guards (168-195); mandatory full record and link tails, joined envelope identity, actual Raw document sequence/payload/stage and semantic document binding (197-254); context creation only after validation (255-261).
- Parent sitemap_source_store.rs: extracted summary accumulator and old refresh compared (273-369); summary/record/link validation extraction and portable validation (370-500); unchanged portable actual-catalog binding (504-560); unchanged v1 context preimage routed through a private summary helper (562-580); shared bounded SQL preflight, canonical row decoders and old materializing load (785-923). The new linked-record primitive explicitly checks measurement_id equality as well as the old expected-document/envelope hash checks.
- The accumulator preserves the old ordered measurement/link hashes, fixed 4096-byte summary reservation, 1024-byte future-link reservation per unjoined record, frame/comma accounting, checked counters, omission revision and fixed-point canonical summary-size accounting. Both materialized and streaming paths use the same implementation. All record/link/summary tails are checked before returning the token/context.
- Thirty-one baseline parent functions were compared as complete normalized source and were unchanged, including schema/init helpers, actual_config, full_active, source_identity, bind_record_document, all A/B writers, inherited projection and anchor validation, preflight and canonical record/link serialization. No changed writer/admission path was identified in this handoff.
- Ordinary Source verification keeps summary plus a bounded current record/link/document in Rust; it does not construct a full SourceSnapshot or raw-catalog vector. It still performs mandatory complete SQL scans/joins/duplicate queries. SQLite sorting/cache/temp-storage behavior and measured RSS are unproven. The Selected path deliberately invokes the previous materializing loader and actual parent projection. Config/plan costs and selected compatibility are not a general bounded-memory claim.
- The source proof is crate-private, non-Clone/non-Serde with private fields. Peer Raw connection/document_by_id signatures match the called APIs on a static read. Caller-owned same-TX lifetime and no same-connection writes/restart while a proof exists remain the contract; Rust type/borrow/visibility acceptance was not compiled.

## Tests read, not executed

All nine authored child test functions and their helpers were read (verified_read_tests.rs:1-368), together with the actual parent hook. Tests use actual existing SQLite/Store/source fixtures and compare the old load oracle with the new reader; this review did not execute or regenerate them.

1. Initialized-empty, actual A-only and actual linked-B transitions; context literal v1 hash witness and connection identity (8-76).
2. Actual capture-unknown and storage-budget omission receipts/reservations (79-123).
3. Last unjoined record, SQL projection/sequence, reserve and omission-revision corruption controls (126-163).
4. Canonically resealed but actual-Raw-inconsistent document/link/stage or missing-measurement controls (166-216).
5. Duplicate measurement identity despite an imported table with its UNIQUE constraint removed (219-257).
6. Missing transaction, different actual connection and absent Raw proof (260-271).
7. Initialized empty versus historical absence and partial schema (274-293).
8. Selected parent Some/None using actual prepared selected plan/child Store, plus exact parent BLOB tampering (295-358).
9. Parent Some with all child sidecar state removed: old low-level None remains insufficient and new reader rejects (361-368).

The portable parent report fixture in the Selected tests is source-test data, not a public HTTP/native crawl proof. The top-level Store's selected frontier/raw-presence/plan gates remain necessary; this low-level Source proof does not replace them. Absence remains unknown. No public App migration or complete coverage/analysis claim follows from this review.

## Unrun gates

Compiler/type/borrow/visibility checking; all nine Source test groups; companion Raw tests; same-read-TX facade/CLI integration tests; platform/read-only/WAL behavior; practical allocation and SQLite workspace measurements. No commands or fixtures in these categories ran during this review, and no Root/product/Git/runtime files were written.
