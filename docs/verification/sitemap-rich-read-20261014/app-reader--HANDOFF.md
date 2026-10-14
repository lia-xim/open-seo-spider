# App rich inventory window — guarded source-only handoff

Root base: `E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-rich-read-integration-v1`, Root-declared HEAD `5091eb9`. Root confirmed the exact Core contract before any product payload edit. All changes are confined to this independent E handoff; Root alone promotes and registers hooks. No Core, repository, Git, runtime or peer source was modified.

## Payloads and Root hooks

1. Replace `crates/app/src/sitemap_membership.rs` only at before SHA256 `4e1143d1b460181352bd2f6bef5bf32862f0f5fb1562f7452264d106d52c50ec`.
2. Replace `crates/app/src/server/sitemaps.rs` only at before SHA256 `be64fad950ec2fda024ad6f188e6edcd5830813c34b9c5255179fe8bd84d1923`.
3. Add `crates/app/src/sitemap_membership_rich_read_tests.rs`.
4. Add `crates/app/src/server/sitemaps_rich_read_tests.rs`.

Append this test-only hook in existing `crates/app/src/sitemap_membership_tests.rs`, guarded by before SHA256 `c8ed1dcac533475620308f5f698791fdebf91b1f3e98fe2f3bf30c590b377845`:

```rust
#[path = "sitemap_membership_rich_read_tests.rs"]
mod rich_read_window_tests;
```

Append this hook in existing `crates/app/src/server/sitemaps_tests.rs`, guarded by before SHA256 `bf0a5057a9e2046535b8952f3bd9210bea4ad8040aa91e6615d1123cd092bd50`:

```rust
#[path = "sitemaps_rich_read_tests.rs"]
mod rich_read_window_tests;
```

Both new modules are children of the actual existing fixture modules and reuse their private actual CrawlStore/crawler/loopback/drain/file-invariance helpers. Existing fixture files were not changed here. No production writer or Core proof constructor was exposed for tests.

## Actual production path

The existing inventory HTTP handler calls `CrawlStore::sitemap_read_window_with_membership_analysis(&analysis_request, expected.as_ref())`, then `membership::inventory_window`. It does not call the materializing `sitemap_read_snapshot_with_membership_analysis` path. A generic read helper retains the existing App job generation/config read lock through the actual SQL capture and returns only owned data.

The Core request/result/error/shared-hash symbols were explicitly agreed with Root and then inspected in its physical `crates/core/src/store/sitemap_read_window.rs`. Root owns their final promotion/public exports and companion verification. App consumes the exact flat `SitemapReadWindow` fields; it verifies reader_engine and recomputes the shared state from the actual returned summary/inherited/coverage inventory SHA/coverage/source/analysis context before accepting catalog_state_sha256. No full or truncated SitemapCatalog or SourceStore snapshot is created in this path.

The materializing Inventory projector remains only under `cfg(test)` as the old-selection oracle. Its response assembly is shared with the compact path, preserving raw JSON fields/availability strings, exact original-identity filters, 200-row/page bounds, lastmod on visible Entry rows only, independent raw/page tokens, and the complete 8MiB App response budget. The production compact branch retains only the already selected rows. Unknown rows remain None; known empty remains Some([]).

## Cursor migration, precise errors and CSV

ReadQuery parses the existing canonical token/checksum/role envelope and prechecks job UUID, filter SHA and consistent raw/page snapshots before Store. It passes all remaining Snapshot fields to the exact Core ExpectedContext. Core checks them against actual state before continuation selection. App still compares actual returned Snapshot/filter and page availability/count after capture. Invalid roles remain400; typed Core StaleContext→stale_sitemap_cursor409, InvalidContinuation→invalid_sitemap_cursor400, ResponseByteBudget→sitemap_response_byte_budget413. Other errors keep the previous corruption/read/budget classification.

One bounded priority difference was explicitly accepted by Root: if a continuation token is stale AND the actual database generation/config also differs from the current App job, Core may return stale_sitemap_cursor before App can emit the former sitemap_job_snapshot_conflict. HTTP status remains409. First-page/no-token checks and post-read App-job mismatch checks retain their former code. No second SQL read is introduced to distinguish that simultaneous-failure case. Missing App jobs still return404 before the job/filter/snapshot precheck; malformed query/role errors remain400 before job lookup.

Both compact JSON and materializing CSV use public `sitemap_read_state_sha256(&SitemapReadState)`. Its explicit new Core reader engine and Option<canonical captured-inventory body SHA> replace the old full-inventory preimage. Token JSON shape and roles are unchanged: valid old-profile raw/page/export tokens still parse and become stale409. Fresh Inventory snapshot tokens work with buffered CSV because `bound_catalog` maps its actual full optional coverage inventory through canonical body serialization/SHA and uses the same Core helper/context fields.

CSV deliberately keeps the old complete-snapshot reader and 64MiB buffered output. Its column mapping, quoting, formula protection, original typed row serialization and transport are unchanged. This handoff does not provide streaming CSV or claim its memory benefit. The existing MCP `spider_sitemaps` route forwards the same inventory HTTP endpoint and already fixes row/page limits at200, cursor4096 and membership65536; no MCP schema/default bound was changed.

## Eight authored controls, zero executed

- Actual HTTP handler versus full Store/oracle complete JSON across all five raw roles, plus fresh Inventory→CSV export and exact original typed row cells.
- Actual HTTP raw/page independent pagination, stable original row under saved-page pagination, stable first saved page under raw pagination, role errors and changed-filter stale409.
- Typed Core error wrappers map to exact old HTTP status/code and website_requests0; ordinary corruption stays500/read_failure.
- Actual visible lastmod, full analysis context and raw CSV byte equivalence against the materializing oracle.
- Exact old-profile tuple fixture generates canonical parse-valid raw/page/export tokens from actual captured Snapshot fields; Core/App/CSV reject stale.
- Valid current versus stale expected state precedes an out-of-range raw position; wrong job/filter and mismatched raw/page snapshots are prechecked.
- Actual historical unknown, initialized empty, incomplete source and missing SQLite retain full old optional-evidence JSON behavior.
- Real unchanged 8MiB derived lastmod response cap rejects a200-entry page and100 recovers; no reduced threshold or generated fixture was run.

Five async membership and two async handler functions plus one server unit function are authored. Their actual Store windows and old full snapshots come from real fixture methods in source; no Window/header DTO literal substitutes for verification. The SQL/HTTP fixtures, crawlers, tests and large-field source have not executed under HOLD.

Only rustfmt and rustfmt --check were run on the four payloads, both exit0. Static byte/text controls and hashes are recorded separately; these are not typechecking. Compiler, Cargo, tests, SQLite, HTTP, generators, native App, browser and SDK remain NOT_RUN. Integration, independent review, actual compile/tests, live handler/browser/MCP proof, RSS and complete S4/parity closure remain Root's gates. Selected-parent materializing compatibility costs remain explicit in Core; CSV remains buffered.
