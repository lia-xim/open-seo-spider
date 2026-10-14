# Bounded independent App rich-reader review

No confirmed new App P1/P2 finding in the examined payloads. This is source review only. Compiler/type/borrow/visibility acceptance, Cargo, tests, real SQLite/HTTP fixtures, browser/native App and physical latency/RSS were NOT_RUN under active HOLD. Root/product/handoff/Git/runtime bytes were not modified.

App manifest b02737c28cc72bbe081f1030a03a74dce1a7e129b8fa89b7dc40f5aa90e1d3f8 and all four payloads matched promoted Root bytes. sitemap_membership.rs pin712ce974c3dafbb0f8b4312f80d0ddc101a5e6f07867f55747a1a57d72c59b83; server/sitemaps.rs pin4884307301eb0c5c4bd32284979edd2db6d1f7ee0527c37819a9ff118e998189. Exact other pins/hook bytes in SOURCE-PINS.json.

## Production chain and lock

Actual registered GET /api/crawls/{id}/sitemaps invokes server/sitemaps.rs:71-94, parses the strict query and separate raw/page roles, then generic read_context29-70 passes expected state into actual CrawlStore::sitemap_read_window_with_membership_analysis. It does not call the legacy full inventory projection. The Job read guard remains live through its generation/config capture, file existence check and awaited blocking SQL read; it is explicitly dropped after capture. Source commits are pinned by Core's transaction rather than falsely serialized by the App guard. Actual values are owned into the blocking closure; static Send/lifetime observations are not a compiler PASS.

Missing job404 remains before expected job/filter conflict checks; canonical query/role parsing still occurs before lookup. SQL/Source corruption is propagated, no fallback to a supplied report or partial fake catalog is introduced. The narrow simultaneous stale continuation plus App-job/SQL generation/config mismatch may prefer stale_sitemap_cursor over sitemap_job_snapshot_conflict; both409. Root explicitly accepted this documented priority change. It was not generalized into an approval for unrelated error changes.

## Cursor, independent paging and bounded response

sitemap_membership.rs:209-275 creates one typed Core request and expected context from canonical tokens. Raw/page roles are schema1/nonzero sequence and schema2/bounded ordinal respectively; both snapshot/job/filter identities must agree before Store opening. Core compares its whole actual state before continuation selection. inventory_window553-657 checks actual reader profile, generation/current-config, complete shared state digest and source/header evidence before building InventoryRows::Window. No truncated SitemapCatalog or SourceSnapshot is created.

project_inventory674-857 keeps the old actual response shape and exact original filter semantics. Saved-page context/ordinal checks remain independent from raw continuation; Core selected row order/after/filter/next/has_more is checked. Visible Entry rows alone receive lastmod assessments. None unavailable history remains distinct from available Some([]). Whole actual JSON response is measured with the existing bounded serializer and8MiB cap, returning no successful prefix on cap failure. Core's additional16MiB internal DTO envelope is not an App8MiB cap increase.

## CSV and legacy profile

The existing full inventory function and Materialized enum branch are cfg(test) only. Production CSV intentionally still calls the complete materializing reader and buffered64MiB CSV transport. Its bound_catalog390-450 uses the exact same sitemap_read_state_sha256 tuple as compact JSON: explicit rich engine, actual summary/inherited, exact Option inventory-body SHA, coverage, Source context, lastmod/analysis engine and actual analysis context. Therefore a fresh JSON full-export token has the same current state basis as CSV; old-profile tokens parse canonically but become stale rather than being reinterpreted as invalid shapes.

Normalized full-source comparison confirms token+decode functions and the entire page_token/bounded serializer/CSV formatter/export tail are unchanged from baseline4e1143d1b460181352bd2f6bef5bf32862f0f5fb1562f7452264d106d52c50ec. Existing CSV formula sanitization/quoting is preserved. No CSV streaming-completion claim is made.

server read_failure12-28 retains typed stale409, invalid400 and response-budget413 even through anyhow context wrappers; corruption remains500. Failure website_requests remains0. This review did not execute network request-count assertions.

## Tests and integration read

All eight authored groups were read: actual lastmod rows/context versus full oracle and identical CSV; legacy raw/page/export token stale behavior; state-before-invalid-u64 sequence and job/filter/snapshot prechecks; unknown/empty/incomplete evidence; actual8MiB full-response refusal and smaller-page recovery; actual production inventory handler/full oracle and fresh token CSV; independent raw/page cursors and role/404/filter cases; typed Core errors through wrappers and corruption500. Their fixtures use the existing actual LastmodStored/PublicCrawler/handler setup, not a mock compact DTO. Root's two actual child hooks and route declarations were read. Parent-module imports and called Core API declarations were checked statically.

Known separate dependency: finish_coverage confirmed a SourceReport v1 escaped outer-key serde scratch resource P2, outside this App adapter. Root requested an immutable guarded lexical-preflight postfix and independent follow-up review. This App receipt does not clear that issue or approve runtime promotion before its correction and required gates.

Unrun gates remain combined compiler/clippy/old+new tests, real handler/SQLite/WAL/read-only/concurrent execution, fixed SourceReport preflight, App/browser/CLI/MCP/protocol matrices, CSV streaming, selected materialization and memory/latency proof. No full S4 or feature-parity PASS follows.
