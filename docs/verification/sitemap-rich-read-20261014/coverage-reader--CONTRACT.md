# Compact coverage read: immutable source-only candidate

Baseline: `E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-rich-read-integration-v1`, HEAD `5091eb941c6740bfb2a6bf610388bc9ca6fb081b`. Root alone owns integration, Git and runtime. This handoff owns only the coverage parent, its new private reader/tests, and the original-root constructor's visitor extraction.

## API

`crate::sitemap_coverage::verified_read` exposes crate-private:

```rust
CoverageReadContext<'a> {
    saved_config_bytes: Option<&'a [u8]>,
    saved_config_sha256: Option<&'a str>,
    generation: Option<u64>,
    discovery_initialized: bool,
}
VerifiedCoverageRead {
    coverage: SitemapCoverage,
    inventory_sha256: Option<String>,
    materializing_selected_compatibility: bool,
}
project_read(
    &Connection,
    Option<&VerifiedRawRead<'_>>,
    CoverageReadContext<'_>,
) -> anyhow::Result<VerifiedCoverageRead>
```

The caller owns one active read transaction, obtains the actual verified Raw token in it, passes actual saved-config bytes/SHA, current generation and discovery flag from that same transaction, and makes no same-connection writes or transaction end/restart while proofs are in use. The function checks active transaction, actual Connection pointer equality, exact persisted config presence/bytes/hash/length and actual run-state generation/discovery marker. It never treats a header or caller boolean as raw verification. No available current-inventory result is produced before the provided Raw proof and all inventory tails have been checked. `None` Raw is unavailable; missing actual config/generation remains inventory unavailable, not an invented empty inventory or a claim about unexamined task tails.

Higher Store selected-plan, mandatory parent-presence and exact parent/source guards remain mandatory. The result's compatibility flag reports this reader's materializing inherited path; Root must also retain any independently necessary selected compatibility flag from its higher guards, including a selected parent with no raw catalog. Source verification is a separate actual same-Connection proof owned by the Source reader, not replaced by coverage.

## Same bytes and semantics

`inventory_sha256` is `Some(SHA256(serde_json::to_vec(&SitemapCoverageInventory)))`, exactly the existing inventory BODY hash and `coverage.basis.captured_inventory_sha256`. It is not a new hash profile. `None` remains `None`, not SHA256(`null`). The streamed serializer preserves all eight existing struct fields and their order, exact root DTOs, the complete sorted task inventory, and optional value/null semantics. Task order and equality explicitly use SQLite BINARY semantics to match Rust `(frontier_kind, task_url)` sorting, including imported NOCASE schemas. No task array or serialized complete inventory buffer is retained.

The existing original-root constructor delegates to `visit_initial_roots(..., FnMut(InitialSitemapRoot)->Result<()>) -> Result<bool>`. False retains unknown legacy provenance and the old early return without import reconstruction. The visitor constructs the exact same one-root DTO and policy coordinates; the portable constructor still collects it when a report requires a full inventory. Coverage uses the visitor both for streamed JSON and expected-coordinate comparison. Extra observed roots are checked by their already verified kind/basis/ordinal identity against the actual explicit/imported intervals.

The portable projector and this SQL projector share coverage initialization/basis, successful completion, source incompleteness reasons and admitted-edge reason classification. The SQL implementation preserves complete reason counts, document/root/child/page-diagnostic counts, required Robots sources, missing/pending/done source distinctions, source generation bounds, every initial declaration, recursive admitted task identities, cycles and duplicate suppression. Invalid page Locs remain diagnostic counts and do not gate complete source scanning. No target page status/indexability participates in source completion.

The ordinary graph stores distinct reached admitted task URLs plus its queue; source documents and child rows are looked up from verified SQL. It does not retain a full original Row Vec, source catalog, all-task Vec or raw edge list. Direct source adjacency uses `sitemap_locations.observation_id` rather than scanning the full Location table per entry. There are several linear verification/serialization/projection passes and per-task/per-source lookups; no measured latency claim is made.

## Explicit resource profile and remaining costs

- Saved config bytes and complete serialized inventory remain bounded at 64 MiB, matching the existing inventory envelope cap. Typed config deserialization and its input/import vectors still exist and are explicit costs.
- Actual task inventory remains at most 1,000,000 rows / 32 MiB URL text; every pending/done row is included and malformed/duplicate identities are errors. Tasks are visited one at a time.
- Distinct graph identities are explicitly bounded at 1,000,000 / 32 MiB URL text. The queue owns additional string copies and the ordered set has allocation overhead; logical text accounting is not an RSS bound. Raw catalog retention is unchanged at 32 MiB.
- Required Robots origins are at most 1,000,000 / 64 MiB unique origin text, followed by the exact 64 MiB combined inventory serialization cap. Both ordinary seeds and actual finalized initial-policy URL SQL use this bounded set. This derived profile does not claim acceptance beyond these measured caps.
- Policy-mode initial origins come only from actual finalized initial page admissions in the existing policy table. The unchanged authoritative `policy_ledger::verify` is retained, even when Robots discovery is disabled. Root's existing config guard may perform the same pass separately; this duplicate work, existing policy-reader allocations, SQLite caches/sorts and physical latency/RSS remain explicit open costs. No fake already-verified boolean is introduced.
- Inherited history reloads the old validated raw catalog and actual parent report bytes (at most 256 MiB), validates the parent inventory against the parent's original config, binds it to exact actual parent bytes and uses the old projector. Its full parent/report/catalog/inventory allocations are deliberate materializing compatibility and are not the ordinary compact path.

No default, source/parser retention cap, persisted row, catalog hash, Report/Checkpoint schema, public API, App transport, SourceStore or SQL schema is changed by this handoff. Complete S4, full CSV streaming, all selected paths, report-writing memory behavior and physical protocol/large-source/RSS proof remain open.

## Authored tests and actual commands

Fifteen new test functions are authored in the new child test module. The fixtures use actual CrawlStore preparation/discovery APIs and actual decoder/parser/robots observations; comparison oracles intentionally materialize the OLD actual inventory on the same transaction. They compare the whole `SitemapCoverage` and exact inventory hash, including empty/pending/legacy, real complete cycles/duplicates, denied/invalid/incomplete sources, an actual 1,201-row task tail, config whitespace and binding mismatches, WAL snapshot isolation, imported NOCASE task schema and duplicates, missing context/different proof connection, actual committed policy initial page admissions, read-only database/WAL bytes, inherited Some/None parent inventories, visitor equivalence/error propagation, empty robots/HTTP206 model observations, unavailable/corrupt tail and graph accounting controls. HTTP206 is explicitly a stored wrapper/model fixture, not a real network gate; selected fixture is the low-level compatibility envelope, not the higher full selected-plan proof.

Actual checks: `rustfmt --edition 2024 --config skip_children=true` and `rustfmt --check --edition 2024 --config skip_children=true` on all four payloads, exit 0. No Cargo, compiler, SQLite fixture, test, data generator, HTTP, native App, browser or SDK execution occurred. All such gates are **NOT_RUN** under the coordinated compute hold. Root must review and execute combined gates after release.
