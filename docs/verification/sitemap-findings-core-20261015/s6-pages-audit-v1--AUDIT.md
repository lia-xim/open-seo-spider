# S6 saved-page audit and minimal traversal handshake

Status: READ_ONLY_SOURCE_AUDIT / NO_PRODUCT_IMPLEMENTATION / ALL_EXECUTION_NOT_RUN. Root-declared basis c8f539b, current265 source a319. The read source was sitemap-csv-streaming-integration-v1; Root's next sitemap-findings-core-integration-v1 starts from the same declared basis. No Git/SQL/test/compiler/HTTP/native/browser was invoked. This is the first FIVE granular page filters, not a relabeling of the existing UI state options or completion of either source-limit filter.

## Actual reusable symbols and guards

- store/sitemap_read_context.rs81-196 `with_prepared_sitemap_read` owns one actual Connection lock, transaction and interrupt guard; saved-config/import/policy verification, both selected raw/source parent-presence gates109/111, `verify_raw_with_control`114, `verify_source_with_control`115, `project_read_with_control`121, analysis preparation144 and exact shared state149 are performed once. `PreparedSitemapRead`34-39 exposes only private borrowed proof references. The callback finishes before proof drop/TX commit189; owned result returns afterwards. Expected validation occurs here, but exact `matches_header` is enforced by the caller facade before selection (read_window172); a new facade must retain that explicit comparison, not assume the common helper does it.
- sitemap_membership_analysis/window.rs15 `VerifiedAnalysisRead` has private actual Connection/raw/coverage/PreparedContext. `prepare_window_analysis_with_control`33 verifies actual TX and raw/source same-Connection pointer identities46-60 and coverage/raw basis62-72 before calling shared `prepare_context_with_control`. No public Header/partial Snapshot can replace this token.
- sitemap_membership_analysis.rs494-655 `prepare_context_inner` binds run/checkpoint generation/outcome/config/raw/source summaries, report length/SHA, terminal state, page/frontier/discovery counts, actual full SourceReport equality, envelope/identity/link shape and bounds. Preserve all failures as failures; an invalid report is not Unknown fallback. `report_hash`310 streams the borrowed SQLite value without a second Rust full report clone. Constants: report256MiB,1M pages,5M links,64KiB field,1MiB metadata,5 samples.
- `page_columns`974 plus `page_from_row`820 are the single target/renderer/age classifier. The saved-page identity is exact original requested_url and report_page_ordinal, not final/admitted/rendered URL. Selected generation provenance895-904 remains intact. `read_inlinks`1046 counts whole saved report anchors, self/nonself/internal/distinct requested sources and deterministic bounded samples. Never substitute a global graph or canonical/final identity.
- window.rs162 `MembershipBasis::Verified` -> `sql_counts`269 performs whole verified retained-source joins for <=200 page identities, original PageLoc role, validation/ordinal checks, DISTINCT BINARY original task URLs and observations, separate alias counts and samples from actual XML Documents. Retain same-file duplicates versus distinct-task semantics; repeated observations/final aliases are not extra sitemaps.
- Current `project_prepared_window`96 verifies the actual proof/coverage and calls shared projection. `project_rows_inner`705 fetches a candidate page BEFORE UI predicates,773 aggregates its memberships,786-811 builds final findings. `read_page_window`978 SQL-limits page_limit+1 before external filtering. It is not an all-stored finding filter by itself.

## Existing local UI versus five granular predicates

web/app.js1625 currently filters only loaded saved_pages: multiple-documents count>1; listed-no-links existing boolean; not-listed current|historical membership; unknown Membership::Unknown; otherwise observation.state equals a selected TargetState. HTML128 exposes seven concrete TargetState options. Preserve that loaded-page UI until the additive S6 surface is implemented; these options do not define the five granular filter contract.

The agreed plan is docs/parity/sitemap-filter-surfaces-plan.md40-44:

| Granular ID | Positive / control / Unknown boundary from current models |
|---|---|
| urls-in-sitemap | `membership == ListedOriginal` (equivalently actual original valid occurrence count>0). Known complete current/historical NotListed is NotMatched; Membership::Unknown is Unknown. Alias/final/render navigation never substitutes. |
| urls-not-in-sitemap | Match ONLY `NotListedCurrentDeclaredSources` or `NotListedHistoricalDeclaredSources`, retaining scope. ListedOriginal is NotMatched. Unknown membership stays Unknown even when observed count is0. |
| orphan-urls | Reuse `listed_without_observed_nonself_inlinks`230/800: positive listing plus0 observed nonself anchors in this bound saved report. Self-only is a positive control, an actual nonself anchor rules out this qualified candidate. Keep report observation_scope and website_wide_orphan_inference=false. Partial terminal reports may supply this explicitly qualified captured-graph fact; they never establish website-wide orphanhood. Missing terminal/graph evidence is unavailable/Unknown, not an empty graph. Exact tri-state conjunction controls should be fixed in Root's model table. |
| non-indexable-urls-in-sitemap | Require original listing and known target obstruction, per plan: BlockedByRobots, Redirected, Non200, or complete NonIndexable with actual reasons. Direct Indexable is control. FetchFailed, IncompleteObservation, NoResponse/missing render evidence are Unknown. RenderedNavigationChanged and RenderedNon200 must retain their separate evidence and receive an explicit Root model verdict. `!observation.indexable` alone is too broad; `state==NonIndexable` alone misses known redirect/non200/robots obstructions. |
| urls-in-multiple-sitemaps | Match observed `original_source_document_count>1`, already DISTINCT original requested XML task URLs. Counts<=1 are only NotMatched with known complete declared cohort; otherwise Unknown negative with observed lower-bound count. Repeated loc/observation or shared final URL cannot manufacture count2. |

`SitemapTargetState`137-151 is ordered by classifier939-966: robots, native redirect, native non200, fetch failure, truncation/renderer error, rendered navigation, rendered non200, missing configured render facts, no response, non-indexable, indexable. Do not recalculate using a boolean and lose this precedence. Pending/NoStoredRequestedObservation/InvalidOrMissingOriginalLocation are used for location findings741-748, not natural members of a saved-page candidate traversal.

## Unknown and unavailable are separate axes

Actual `SitemapAnalysisContext.available=false` carries one of: terminal_saved_report_unavailable516, validated_source_catalog_unavailable535, live_crawl_target_analysis_unavailable552, saved_checkpoint_context_mismatch579, saved_source_measurement_context_mismatch588. The base result has saved_pages=None, not Some([]). An unavailable whole analysis cannot be bypassed by a fake per-page context or raw positive rows. The source-attempt domain may have different availability; it is outside this saved-page traversal.

When analysis is available, individual saved-page membership can be Unknown because both narrow negative-permitted flags are false. Coverage supplies scope, completion and typed `SitemapCoverageReason`348-385 counts: unavailable/history/inventory/initial-root/discovery/task gaps, pending sources, incomplete/failed/denied/unbound source/declaration/parser/transport rows etc. Preserve the actual reason_counts and inherited basis; never turn these flags into site-wide completeness. Positive observed membership or observed distinct count2 remains evidence even with incomplete sources. Inlinks are available captured-report counts, not a new graph-complete flag. Existing models have no website-wide complete graph assertion.

## Minimal private implementation choices

The smallest functional adapter is a private loop around `project_prepared_window` using the SAME VerifiedAnalysisRead, raw proof and coverage inside ONE common callback, empty visible_rows, row_limit0, page_limit200, and last actual candidate ordinal. Apply tri-state predicate and assessment selection before collecting user limit+1 matches. This reuses all current semantics without a public RichWindow loop or fabricated Snapshot. It must not re-enter the public Store reader/reprepare per internal chunk.

Its explicit cost trap: each `read_page_window` restarts json_each(report.pages), and each `read_inlinks`/`sql_counts` performs whole-cohort SQL scans for its <=200 keys. With sparse1M candidates and a256MiB report, repeated candidate JSON parsing and joins can be expensive even though Rust retains only200 findings. Do not call that a hard CPU/RSS bound.

Recommended narrow factor for real all-stored traversal:

```rust
// PRIVATE proposal; names/signature are a handshake, not existing implemented API.
pub(crate) fn visit_prepared_saved_pages(
    prepared: &VerifiedAnalysisRead<'_, '_>,
    after_page_ordinal: Option<u64>,
    visitor: impl FnMut(&SitemapSavedPageFinding)
        -> anyhow::Result<std::ops::ControlFlow<()>>,
) -> anyhow::Result<SavedPageTraversalOutcome>;
```

Place it in the existing private analysis/window boundary so it can use the token's actual Connection/raw/coverage/PreparedContext directly. Outcome explicitly distinguishes unavailable, exhausted and caller-stopped traversal; it must not encode unavailable as count0/exhausted. Check actual TX/control and after ordinal against actual prepared saved_page_count before opening selection. Return only owned progress/context after the callback ends; never expose SQL/proof constructors.

Use ONE ordered SQL candidate cursor with page_columns/page_from_row over actual report ordinals after the supplied position; keep <=200 projected candidates at a time. Concurrent readonly aggregate statements on this same Connection can process that batch while the outer cursor lives. Extract the current saved-page construction786-811 into a shared helper used by BOTH existing project_rows and the new traversal. The new path calls existing read_inlinks and MembershipBasis::Verified.counts per chunk, then that shared helper. It needs no redundant read_target_pages or frontier scan for these already parsed saved pages; those belong to visible-location projection. Preserve samples/order/age/aliases and original DTO exactly. No full candidate Vec or raw/source materialization is introduced for ordinary reads; selected compatibility costs remain explicitly coarse.

This is bounded Rust cardinality, not a measured RSS limit. Existing SQL workspace/report/field/sample limits can still be substantial. Root's new work and output-byte bounds must be explicit; do not silently lower or raise old limits during extraction.

## Pagination, sparse progress and directed gates

- Filter/assessment/exact identity/search are applied to every candidate BEFORE the user-facing limit. Internal batches200 are not public pages. Keep scanning sparse nonmatches past200 and400 until limit+1 matches or honest exhaustion.
- Cursor binds the actual same-TX shared context, finding engine/domain, canonical filter/assessment/search/identity and stable original report ordinal. Fresh raw/page/CSV roles are not findings roles. Expected full state must match before traversal; preserve old/new role400, stale409 and corruption500 distinctions.
- A normal matched page cursor uses the LAST RETURNED match ordinal; do not advance it to the extra lookahead match or end of its projected batch. That would silently skip matches on the next page. Optional scan progress is distinct from matched-page continuation.
- If a scan-work/deadline/input/export budget stops before enough matches/exhaustion: either typed budget error with no successful partial result, or an explicit incomplete scan with an actual last-examined candidate cursor and resumable status. Never has_more=false, matched_total0 or complete merely because a200/400 candidate budget ran out. Keep examined/projected/returned counts separately meaningful.
- Observe control checkpoints before/between candidate conversions and aggregate rows, retain interrupt mapping through the common outer control, and propagate failures from later chunks. Source/report/selected guards run once before traversal; never replace them with a report-only or first loaded page proof.
- Required future tests: >400 actual saved pages with sole match after200 and400 for all five applicable predicates; limit1/10/200 traversal against independently expected full identity/order; all Unknown/current/historical controls; same-file duplicates versus two task documents; self/nonself and alias/final/original identities; redirect404/noindex/canonical/blocked/failure/truncation/render cases; exact changed report/S3 A/B/marker/selected/context stale; tampered late source/selected/report failure; sparse budget progress without false exhaustion; repeated chunk boundaries without duplicate/skip and same-TX WAL writer; no website requests/read writes. ALL NOT_RUN.

No SourceAttempt implementation, public DTO/facade, tests or product code was authored by this audit. Root owns the final model predicate table and API names before implementation.
