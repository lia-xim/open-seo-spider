# S4 configured source work profile: fetch, capture and durable A guard

Immutable guarded source handoff only. Four new directed test functions are authored, with zero executed fixtures/tests. Rustfmt and its check pass; no Rust type-check, Cargo, source generator, HTTP, SQLite fixture, native, SDK or browser run was performed under the compute hold.

The production slice is limited to `crawler.rs`, `crawler/sitemap_capture.rs` and `sitemap_source_store.rs`. It depends on Root's actual optional `CrawlConfig.sitemap_source_processing` field and actual `sitemap_source_parser_limits()` / `sitemap_source_body_limit()` helpers, plus the frozen companion profile module. Root remains sole integrator; no Root/Core/Git/runtime file was modified by this task.

## Behavior

- Constructor validates both source helpers before client/store I/O. Invalid explicit work bounds fail; page response limits must still be positive.
- Native sitemap body retention uses the actual configured source body helper. `None` retains the previous min(page cap, default8MiB) behavior. Explicit `Some` uses its source cap independently of the page cap.
- The original supplied-slice `capture` and `capture_with_progress` helpers keep default semantics and XML ownership. Actual configured fetches select the correct profile, including non200, transport failure and unstarted envelopes. `SourceCapture.limits`, observed decoder, count-only pass and retained parser share the actual parser limits.
- Durable `ExpectedCapture` carries the body cap and full parser limits from actual current saved config bytes. `record_a` checks every recorded work/retention bound against that profile, including events. It does not substitute the immutable initial-catalog config hash for the current capture hash.
- Received chunk progress remains post-HTTP-decoding and independent of retained prefix bytes; decoder-returned XML bytes/EOF/CRC remain independent. Protocol thresholds, original source IDs/failures, parser retention budgets, sidecar budgets, B admission and existing helper XML buffers are not raised/rewritten here.

No streaming/memory optimization is included. The existing body plus decoded XML vectors and separate complete count/legacy passes remain. A successful64MiB opt-in is not a below50MiB RSS claim. Genuine52_428_800-byte protocol boundary/excess, crash/restart and physical/native acceptance remain separate open gates.

## Required Root hook

After guarding the existing original test file, append to `crates/core/src/crawler/sitemap_capture_tests.rs`:

```rust
#[path = "sitemap_large_capture_tests.rs"]
mod large_source_tests;
```

The new file is intentionally a child of that existing test module so it reuses its actual HTTP listener/connection drain, gzip producer, crawler/store setup and native collector helpers. Do not register it as a sibling of `sitemap_capture_tests`; that would hide its private fixture helpers. This hook is a Root-owned integration instruction, not an edited old-test payload.

## Four authored test groups

1. Real9MiB XML responses with an absent historical field retain5MiB by default and8MiB under a larger page cap; explicit `Some(Default)` remains8MiB while independent of a1024-byte page cap. All stopped counts remain unavailable/unknown, and actual returned chunk progress exceeds retained bytes.
2. Zero and ceiling-plus-one values for each explicit profile work field fail the public Crawler constructor without a loopback request.
3. Real9MiB Plain, file-gzip, HTTP-gzip and sitemap-index responses retain and decode the actual XML, report its actual hash/count, record exact64MiB/2M-event opt-in bounds and persist an A record. An actual page fetch remains capped at1024 bytes. Non200 and robots-denied/unstarted envelopes retain the actual Some profile and can persist a correctly bound A record without invented decode/count facts. This is a real>8MiB fixture design, not a52MiB protocol matrix or a lowered protocol threshold.
4. A legitimate actual delay override changes current saved-config SHA while preserving the initial catalog anchor. Strict, internally valid negative measurements derived from the actual capture carry wrong body/XML/event bounds; the actual Store must reject each without changing the sidecar, then accept the actual correctly bound capture.

These tests have not been executed or materialized. Positive measurements are obtained from actual public Crawler/HTTP and actual Store APIs only when executed; constructed DTO variants are explicitly negative policy-provenance controls.
