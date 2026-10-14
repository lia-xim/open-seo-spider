# S6 actual source candidates — immutable source handoff

This is authored source only. Root owns product/Git integration. No Rust compiler, Cargo, test, SQLite fixture, HTTP, native process, browser or SDK was run. rustfmt parsing/formatting and final --check with skip_children=true passed. The initial rustfmt invocation without skip_children failed because the isolated handoff intentionally does not include the existing coverage child tests; the successful corrected invocation did not execute those tests.

## Contract

`prepare_source_findings<'a,'conn>(&'conn Connection, Option<&'a VerifiedRawRead<'conn>>, Option<&'a VerifiedSourceRead<'conn>>, Option<u64>, Option<&str>, &SitemapReadControl) -> Result<PreparedSourceFindings<'a,'conn>>`.

`scope() -> &SitemapSourceFindingsScope` and `visit_after(Option<&SitemapFindingPosition>, FnMut(Position,SourceEvidence)->Result<ControlFlow<()>>) -> Result<bool>` preserve the agreed API. True means exhausted; false means the actual visitor broke. The token borrows actual verified capabilities and connection, owns a shared Control clone, and cannot be constructed through public DTOs. Root's prepared read pipeline is responsible for obtaining actual tokens and preserving the existing higher selected-parent guards. The caller must not mutate the Connection while verified capabilities are alive.

## Evidence and order

1. All actual XML measurement records, in positive measurement sequence, including A-only. The Source token's new typed visitor decodes existing canonical SQL projections with existing helpers and LEFT JOINs B by exact BINARY measurement_id. No join on expected source/document identity is used. Numeric protocol projection is the existing validated pure assessor.
2. Actual XML legacy documents in global raw sequence which have no exact durable B link for their actual document_id. This stage does not manufacture a measurement or numeric within-limit verdict.
3. Actual XML frontier tasks with neither an actual measurement for the exact current generation/config/task binding nor a provably current legacy document. Only this filtered candidate set receives ordinals 1+. SQL distinct joins perform the set subtraction; there is no all-task Rust Vec or per-task full Source scan.

A legacy document lacks a per-capture saved-config hash. It suppresses a current task only if document generation equals actual current generation AND actual current config SHA equals immutable raw initial-config SHA. Otherwise the task stays explicit Unknown. Thus a stored done task can coexist with a historical document/capture without being presented as an additional remote HTTP attempt. Same URL alone never suppresses a new-generation/new-config pending task. A fixed global omission marker remains scope-only; it does not become a synthetic source attempt or claim an omitted-attempt count.

The continuation is validated against an actual candidate in its exact stage before any following callback is invoked. Associated documents cannot serve as LegacySourceDocument anchors; arbitrary sequence/ordinal/domain/zero values are rejected.

## Actual context and bounds

Preparation verifies !autocommit and actual connection pointer identity, actual saved-config byte SHA directly from bounded SQL BLOB bytes (no reserialization/copy), and actual run_state generation. It then validates and hashes every task, including robots/done/pending and the late tail, even RawNone. The shared coverage validator is only made crate-visible; its normalization/duplicate/order/preflight semantics remain unchanged: 1,000,000 tasks and aggregate32MiB original task UTF8, no new per-task8KiB cap.

Scope task SHA has explicit profile `open-seo-spider/source-findings-current-task-inventory-v1`, hashing the compact serde tuple `(profile, actual_generation_Option, actual_current_config_sha_Option, all_typed_tasks_in_binary_kind_url_order)` directly into SHA256. It is deliberately NOT the broader coverage-inventory SHA. `current_frontier_task_count` counts all actual task rows, including robots. Table absence yields unavailable/None, never measured zero; an existing empty table is known-empty inventory. Configuration/generation Options remain explicit in the fingerprint and must match actual SQL presence.

Ordinary extra Rust retention is one bounded record/link/document/task plus a previous task identity used by the existing validator. Existing Raw/Source verification and selected materializing compatibility costs remain; SQL DISTINCT/joins/order may allocate SQLite working storage. Physical RSS, query plans, cancellation latency, compiled lifetimes, full-workspace integration and real native acceptance are NOT PROVEN. This does not close S4 or full Screaming Frog parity.

## Authored directed tests (11; all NOT_RUN)

Tests use actual CrawlStore prepare/begin/initialize/A/B APIs, real observed entity decode/XML parser/measurement collector, fresh readonly connections and real SQLite transactions. They distinguish stored transport fixtures from native HTTP evidence. Covered: distinct gzip captures sharing SourceID with exact B, legacy no-measurement, current config/gen guards, RawNone full task validation and long URL, binary order/real anchor rejection, scope-only fixed marker, historical None vs initialized empty, missing task table, WAL snapshot consistency, actual connection/config mismatch, typed cancel/deadline.

Only four product payloads are supplied: two new finding files, one narrow Source token visitor addition, and two visibility-only coverage task helper changes. Existing root model, Store facade, API, lib.rs, Raw implementation and selected validators are unchanged.