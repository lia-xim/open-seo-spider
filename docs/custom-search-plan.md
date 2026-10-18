# Next product capability: custom search

`rendering.custom_search` remains planned in the feature ledger. Implement a
dedicated search capability rather than treating named extraction as acceptance.
No implementation or reference-product parity is claimed by this plan.

The user configures bounded literal or Rust-regex rules and explicitly selects
raw HTTP HTML, rendered DOM HTML or captured selected text as the search source.
Persist the rule identity, source kind/revision, count, match offsets and bounded
excerpts. Raw and rendered observations must coexist. Missing sources remain
unknown; a complete observed source with no matches can report zero.

Acceptance requires:

1. Exact literal/regex counts and Unicode-safe offsets/excerpts on owned inputs.
2. Invalid rules rejected before website requests; caps for pattern compilation,
   input work, retained matches and serialized evidence. Incomplete input or
   evidence must not masquerade as complete counts.
3. Optional configuration and result fields preserve disabled/legacy behavior.
   Selected-text rules use the explicit capture profile without silently
   capturing or falling back to a different source.
4. Native crawl and actual renderer fixtures keep raw and DOM evidence separate.
5. Browser configuration → crawl → saved inspector/results → CSV/JSON → cold
   reopen retains the same bound evidence. Add corresponding API/MCP contracts.

Likely implementation boundaries are `crates/core/src/models.rs`, a dedicated
`custom_search.rs` exported by `lib.rs`, the raw processing path in `crawler.rs`,
the rendered HTML path in `renderer.rs`, and existing content-capture bindings.
The bounded regex compilation in `extraction_engine.rs` can inform shared
compilation without changing extraction semantics. UI, analysis aggregation,
exports and MCP schema changes must accompany the core engine.
