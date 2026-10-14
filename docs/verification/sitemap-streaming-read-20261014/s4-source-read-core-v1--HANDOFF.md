# SourceStore incremental verified read — source-only handoff

Base: `E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-streaming-integration-v1`, Root declared HEAD `380beb1ccacf73659d0ac8dd7fdb39c543b88883`.

Only this separate E handoff was changed. No Root, Git, runtime, Store facade, App, public exports or peer-owned file was changed. No Cargo/compiler/test/SQLite fixture/HTTP/generator/browser/native run occurred. `rustfmt` is the only Rust tool used. All nine test groups are AUTHORED / NOT_COMPILED / NOT_RUN.

## Promotion guards and registration

- Replace `crates/core/src/sitemap_source_store.rs` only if its raw SHA256 is `f1ae66933221d7b84f065d820642c65418ff18fd0b789be09b35fa03a095fefa`.
- Add `crates/core/src/sitemap_source_store/verified_read.rs`; parent payload registers `pub(crate) mod verified_read`.
- Add `crates/core/src/sitemap_source_store/verified_read_tests.rs`.
- Root must append the following test-only hook to existing `crates/core/src/sitemap_source_store_tests.rs`, guarded by raw SHA256 `6fa3e14f62670e27eb8be8647244e1b3cc914def024e6d50d99eebde41f12099`:

```rust
#[path = "sitemap_source_store/verified_read_tests.rs"]
mod verified_read_tests;
```

The new tests are a child of the existing tests module so they use its actual private CrawlStore/SQLite/observed-decoder/XML/A-before-B fixtures. Existing writers are not exposed as public APIs. This hook and the peer RawReader must be integrated before a compile/test gate; neither was run here.

## Exact crate-private API

```rust
crate::sitemap_source_store::verified_read::verify_source<'conn>(
    connection: &'conn rusqlite::Connection,
    raw: Option<&crate::sitemap_ledger::verified_read::VerifiedRawRead<'conn>>,
) -> anyhow::Result<Option<VerifiedSourceRead<'conn>>>
```

`VerifiedSourceRead` has private fields and no Clone/serde constructor. Its methods are `summary()`, `context()`, `connection()` and `materializing_selected_compatibility()`. Raw API dependencies were explicitly coordinated with its owner: `summary`, `inherited`, `connection`, `document_by_id`. Raw immutable companion pin is supplied by that owner/Root, not guessed in this handoff.

The actual Connection must be inside a caller-owned transaction; any Raw token must borrow that exact Connection pointer. SourceSome with RawNone is rejected. Historical SourceNone remains None; no empty summary or completion claim is manufactured. Root's facade must still perform both existing selected row/frontier and legacy-presence gates, including when either token is None. This low-level source reader independently validates actual selected plan/parent source presence.

## Ordinary read path and compatibility

Ordinary reads preflight metadata/schema/counts/bytes/identity columns, reject duplicate record/link IDs in SQL even when uniqueness constraints were removed, scan every record in sequence, then use an ordered SQL link→record join and verified Raw document lookup. All original canonical JSON, SHA, SQL projections, contiguous source sequence, link order, envelope seal, stage seal, transport/decode/retention binding and unknown-history checks remain.

The shared `SummaryAccumulator` replaces the old `refresh` loop accumulation and serves both portable/writer refresh and SQL streaming validation. Original revision, comma/framing, fixed metadata allocation, unjoined-link reserve, charged/logical bytes, rolling hashes and fixed-point summary algorithm remain shared. The summary/context returned only follows complete-tail verification. `snapshot_context` still hashes the exact v1 `(engine,"present",summary)` tuple or `(engine,"historical-unknown")` tuple; portable validation remains in the public helper.

The shared link-record validator explicitly compares exact measurement IDs as well as envelope and expected document IDs; SQL join collation is not trusted as proof of byte identity. Canonical SQL row decoding is also shared with the old materializing loader. Existing portable error precedence may shift where common header checks were extracted; accepted portable states and context bytes are intended to remain identical.

For selected recrawls, this increment deliberately validates the actual operation marker, saved plan, exact bounded parent BLOB/SHA, actual config and source projection, then invokes the old materializing `load` compatibility validator. The new helper preflights run-plan bytes at 256MiB (matching Root's additive raw-read facade cap), parent BLOB at 256MiB, actual config at 64MiB and operation marker at 64 bytes. Selected SourceSome still invokes the old inherited-anchor helper with its existing 64MiB run-plan cap. Selected parentSome/currentNone is rejected; selected parentNone/currentNone remains historical unknown. The token method returns true for selected SourceSome. Selected SourceNone has no token, so Root must derive the compatibility flag from its already-verified SelectedPlan. No selected memory improvement is claimed.

Ordinary Rust retention is a compact summary plus one bounded canonical record/link/document and temporary canonical serialization; no complete source records/commits/raw-row Vec is built. Config/run-plan metadata has its existing independent cap. SQLite aggregates/grouping/order/join may allocate working storage or transient indices. No query-plan, timing, concurrency, RSS or end-to-end bounded-memory performance claim has been tested.

## Authored controls

1. Actual initialized empty → A-only → B path matches materializing portable oracle, v1 context tuple, revision, reserve and raw catalog stability.
2. Actual capture-unavailable and record-capacity omission markers retain record count/reserve and exact first-marker revision.
3. Valid resealed last unjoined record with unchanged summary, late SQL projection/sequence corruption and summary reserve/marker corruption all fail both readers.
4. Resealed portable-valid link/document-payload and record-stage mutations still fail actual raw-document binding; missing measurement ref is explicitly SQL-fixture corruption with FK disabled only during mutation.
5. Duplicate source measurement IDs after controlled UNIQUE removal fail even with matching schema shape and refreshed accounting.
6. Autocommit, another connection to the same database and missing Raw proof are rejected.
7. Historical no-schema None, uninitialized empty schema None, initialized empty Some and partial schema failure remain distinct.
8. Actual selected parentSome and parentNone controls preserve explicit compatibility and reject changed exact parent bytes.
9. Actual selected parentSome/currentNone delete-all control cannot be relabeled historical absence.

All controls are source-authored only. They have not constructed SQLite fixtures or executed any product code under the compute HOLD.
