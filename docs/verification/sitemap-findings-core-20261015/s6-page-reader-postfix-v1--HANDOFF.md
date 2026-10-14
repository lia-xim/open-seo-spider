# Saved-page domain typed-facts preflight postfix

IMMUTABLE AUTHORED / NOT_COMPILED / NOT_RUN. V1 manifestabe095c0 and all its files remain unchanged. This is a separate sequential three-payload guarded postfix; it depends on v1's unchanged window accessor and frozen modeld7d62bd0/723200ce. Root alone applies it and adds the actual Store call.

## Confirmed v1 issue

The existing PreparedAnalysis global validation covers report envelope/url/link shape, full checkpoint/source/report identity, but does not type-check robots_warning/PDF/content-type/target fields for every saved page. V1 extra_facts/page_from_row only visited the current projected batch. A visitor break at1 could hide malformed robots_warning at450; after399 could bypass a malformed typed PDF/target field at0. A valid full report SHA does not make those actual typed fields healthy. This is a concrete additive S6 domain guard gap; old raw/inventory behavior and its hash profile are not changed.

## Exact additive API and required Root placement

```rust
pub(crate) fn validate_saved_page_candidate_facts(
    prepared: &VerifiedAnalysisRead<'_, '_>,
    control: &SitemapReadControl,
) -> anyhow::Result<()>;
```

The analysis parent re-exports it crate-privately beside visit_saved_page_candidates. Root MUST call it only in the available SavedPage-domain branch of its actual with_prepared callback, before constructing/exposing the new findings context or invoking a findings user callback. SourceAttempt and legacy readers are unaffected. The helper never accepts an after/selector/work limit, fake Header or constructor.

The helper borrows the actual private prepared Connection/context, checks actual TX/availability/generation/count, binds the shared interrupt control and uses one ALL-report ordered SQL cursor. It runs the EXACT existing page_from_row typed target classifier plus the same new extra_facts for each row, verifies contiguous zero-based ordinals and total prepared count, and discards each row immediately. No membership/inlink aggregation, whole Vec, second Store connection, reprepare or full SHA pass is introduced. Both controls retain typed stop mapping. Existing coarse report/SQLite workspace costs remain; no RSS/hard latency promise is made.

The original visit_saved_page_candidates and remainder of its reader are exact-text unchanged. The existing analysis parent changes only its private re-export. No PAGE_COLUMNS/legacy DTO/hash/target classification/predicate/model/source changes are included.

## Directed source regression

The actual readonly preparation fixture now invokes this helper before its visitor, matching Root's mandated placement. One new async group adds an actual500-page Crawler Store and controlled rehashed report-corruption cases: malformed warning at450 with would-be break1, malformed PDF at0 with after399, malformed old target bool at0 with after399. Each must fail with callback_count0. Restoring genuine null/false values permits exactly one callback at actual ordinal400. SQL operations are labelled corruption fixtures, not positive writer provenance. Existing five names/assertions remain, now six total authored functions. All six remain NOT_COMPILED / NOT_RUN; the postfix adds ONE source-authored group, not six new passes.

Actual checks performed: rustfmt parser/check exit0 for the three payloads; exact unchanged visitor and parent-only-reexport text comparisons; before/after hashes and unchanged v1 dependency. No compiler/Cargo/test/SQL/HTTP/PDF/native/browser/Git/Root product/runtime operation was run. Final combined type/runtime and Root placement remain unproven until their separate gates.
