# App source-measurement context binding

Guarded source handoff only. Rustfmt/parser PASS; Rust compilation, all five tests, native/runtime, SDK and browser execution are NOT_RUN under the coordinated compute hold. This handoff does not establish S3 completion or feature parity.

The sole production change adds `SitemapReadSnapshot.sitemap_source_measurement_context` to the existing `catalog_state_sha256` tuple in `bound_catalog`. Core supplies this string after loading and validating the durable sidecar and original catalog in the caller's existing SQLite read transaction. App does not reopen the database, traverse the sidecar again, or synthesize an empty measurement catalog. The context includes the Core engine/profile, full verified summary, revision, content and association hashes, typed omission marker, and inherited parent basis; historical absence has its own explicit Core context.

Existing raw JSON cursors, saved-page cursors and full raw CSV snapshot tokens all use that state hash. A sidecar-only change therefore invalidates the old tokens with `stale_sitemap_cursor` / HTTP status 409 while the original catalog rows and CSV schema/body remain unchanged. No endpoint, response DTO, UI, schema, CSV column, cap, writer visibility or admission rule is added by this App slice.

## Guarded payload

- Replace the two existing App files only when their exact `before_sha256` matches the manifest. Their only textual deltas are the six-line context/comment insertion and a three-line child-test-module hook with its separating newline.
- Create `crates/app/src/sitemap_source_context_tests.rs` only if absent. The child module deliberately reuses the existing private actual HTTP/Crawler fixtures; it is registered from `sitemap_membership_tests.rs`.
- Product copies are under `product/`; Root is the sole integrator. No Core file, Root checkout, Git state, installed binary or running application was modified by this task.

## Integration dependencies

The frozen companion Core v1 and Store integration v1 plus its required v2 successor are pinned in `SOURCE-PINS.json`. The Store v2 delta hardens selected-parent presence/exact-parent checks; its public API is unchanged. Production needs the actual public `SitemapReadSnapshot.sitemap_source_measurement_context: String` field generated after the verified same-transaction read. The authored tests additionally require the public optional `sitemap_source_measurements` fields on the read snapshot, report and checkpoint; public `SitemapSourceStoreOmission`; and Root's agreed public alias `sitemap_source_measurement_snapshot_context` for `snapshot_context`. No private writer is exposed or called.

Root owns module/export registration, all other DTO-literal repairs, and the terminal saved-report consistency gate. The App tests depend on the observed Root gate returning unavailable analysis with reason `saved_source_measurement_context_mismatch` after a valid sidecar-only restoration changes a terminal checkpoint's source summary. The original catalog/raw read stays available; stale page cursors are rejected before any current page analysis can be claimed. Selected recrawl must also preserve parent `None` as historical unknown and verify exact parent scope; the tests exercise this integration obligation rather than treating an unverified draft as accepted.

## Directed authored controls

The new module contains five test functions, all NOT_RUN. No HTTP/SQLite fixture has been materialized by executing these tests. When executed, HTTP means actual loopback source/page requests through public `Crawler`; App cursor/export checks call the actual inventory/CSV projection functions and assert their HTTP status mapping. These are not App-router, MCP, browser, native-binary, crash or process-restart execution proofs.

1. Actual A capture survives a real SQL trigger abort in the B association transaction. The canonical A-only SQL rows and metadata are copied to an explicitly labelled terminal restoration fixture whose original non-source SQL rows remain byte-for-byte equal. Raw, saved-page and CSV tokens stale; refreshed raw CSV bytes stay equal.
2. Two real captures with identical saved config/task/generation/body must produce byte-identical A rows. Restoring the unlinked state changes association state without a new captured record or any non-source SQL-row change. All three token roles stale.
3. An actual long source task exceeds the strict bounded measurement JSON representation while retaining a valid source task identity. The resulting actual typed `CaptureUnavailable` marker, with an omission revision and no retained record, is restored from real bytes. It is not a fabricated empty/complete source. All three token roles stale.
4. An explicitly labelled historical compatibility fixture removes only the optional new source state from a real report/checkpoint and rebinds the exact rewritten report SHA. Subsequent reads remain `None`, use the actual public unknown-context helper, retain available legacy target analysis, and do not change stored file bytes.
5. Real selected recrawls inherit actual original records and links, bind the exact parent snapshot bytes and the separate source-store/legacy-catalog parent hashes, and make no new source fetch. A historical parent remains `None`. A deliberately wrong typed parent hash is rejected by the real loader; restoring the original bytes restores the original context. Parent files stay unchanged.

The SQL restoration and historical compatibility steps are controlled test mutations, not claims that production permits an A write to an already completed run. The test code obtains positive source snapshots/seals/accounting values only through actual public Crawler writes when executed. The tests do not construct a positive measurement/catalog DTO or recompute its integrity seals.
