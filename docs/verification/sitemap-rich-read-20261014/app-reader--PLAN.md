# App rich-window migration — agreed design

Base Root: `E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-rich-read-integration-v1`, Root-declared HEAD `5091eb9`. Product edits began only after Root confirmed the exact API contract; the implementation follows it. No compiler, tests, SQLite/HTTP fixtures, generator, native/browser/runtime or Git execution.

Ownership after handshake: only guarded payloads for `crates/app/src/sitemap_membership.rs`, `crates/app/src/server/sitemaps.rs`, and new directed App test modules. No Core or existing test fixture modifications here. Root alone integrates. Existing membership before SHA256 `4e1143d1b460181352bd2f6bef5bf32862f0f5fb1562f7452264d106d52c50ec`; server before `be64fad950ec2fda024ad6f188e6edcd5830813c34b9c5255179fe8bd84d1923`.

## Minimum actual Core result

The existing inventory response needs actual same-TX config byte SHA/generation; optional raw summary/inherited basis; complete derived coverage; optional canonical captured-inventory body SHA; verified source context string; membership analysis (including its complete context and independently selected saved pages); bounded owned raw rows with next matching raw sequence and has_more. The agreed `SitemapReadWindow` is flat and supplies these fields, reader_engine, catalog_state_sha256 and the selected-compatibility marker. No truncated SitemapCatalog or fake full Source snapshot is created.

Coverage owner confirms `inventory_sha256: Option<String>`: Some hashes canonical `serde_json::to_vec(&inventory)` body and equals derived coverage's captured-inventory SHA; None remains None. The materializing CSV reader can map its full optional inventory through the same body serializer/hash without changing the coverage hash profile.

Agreed Core call is Root-owned `sitemap_read_window_with_membership_analysis(&request, expected.as_ref()) -> SitemapReadWindow`, using `SitemapReadWindowExpectedContext` for every existing Snapshot field except App job UUID. Core verifies expected context before selecting raw continuation. Public shared `sitemap_read_state_sha256(&SitemapReadState)` supplies the exact rich engine/profile hash to compact JSON and buffered CSV. Root's actual physical DTO/API file was inspected after confirmation.

## Preserve actual response, filtering and cursors

The actual inventory HTTP handler switches to the new Core call, holding the existing App job read lock through SQL capture. A compact App projector consumes only bounded rows and verified state. The complete materializing inventory path remains a cfg(test) oracle; CSV retains its separate materializing compatibility path. JSON schema fields, availability reasons, raw/page role bounds (200 App rows/pages), original-identity exact filters, lastmod projection on visible Entry rows only, read-only/zero-website behavior, 8MiB complete response cap and 409 job-snapshot conflict remain.

Tokens keep the existing canonical envelope/checksum/role parser. The new catalog-state preimage explicitly includes a rich-read engine/profile and replaces full inventory bytes with its exact Option<body SHA>. Old raw/page/export tokens still parse and must fail as stale409 when compared to the new state; malformed/wrong-role tokens remain400. Raw and saved-page cursors remain independent; page size alone does not change captured state.

CSV remains the existing materializing/full-buffered transport for this increment. Its `bound_catalog` must compute the SAME new snapshot state from its fully validated full snapshot (map optional inventory to canonical body SHA). Otherwise every newly issued inventory snapshot token would fail ordinary Inventory→CSV export. Old pre-migration tokens should be stale409 for both paths; no silent CSV fallback accepts a different state profile.

The stale-before-invalid-continuation edge is settled: App parses roles/job/filter and consistent raw/page snapshots before Store; Core compares full expected context before selection, then App still checks the returned actual Snapshot. Typed Core errors StaleContext, InvalidContinuation and ResponseByteBudget map to the existing 409 stale, 400 invalid-cursor and 413 response-budget codes. Other corruption is not relabeled. Analysis consumes already-selected rows and its captured context remains independent of page positions.

## Directed controls authored, all NOT_RUN

Eight groups use actual existing CrawlStore/loopback producers and actual App handlers: compact/full-oracle complete JSON equality across all row roles; fresh JSON export into buffered CSV with exact original typed rows; independent raw/page pagination and role checks; typed Core error contract; visible lastmod/full context/CSV equality; old-profile raw/page/export tokens→stale; expected context before invalid after with job/filter/mixed-snapshot prechecks; unknown/known-empty/incomplete and real JSON budget/smaller recovery. The last two topics are separate membership groups, for five membership and three server groups total. No Window/header DTO literal stands in for an actual Store read.

Prefer new test files as children of existing fixture modules using guarded Root-only hooks. Existing membership test fixture before SHA256 `c8ed1dcac533475620308f5f698791fdebf91b1f3e98fe2f3bf30c590b377845`; server fixture before `bf0a5057a9e2046535b8952f3bd9210bea4ad8040aa91e6615d1123cd092bd50`. Exact hook locations/names will be listed in the final handoff. Tests stay AUTHORED / NOT_COMPILED / NOT_RUN under HOLD.
