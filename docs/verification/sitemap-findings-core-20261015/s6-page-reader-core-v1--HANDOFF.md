# S6 private saved-page candidate reader handoff

IMMUTABLE AUTHORED SOURCE / NOT_COMPILED / NOT_RUN. Only owned staged files were written, read/hash/text checks and rustfmt parser/check were executed. No Cargo/compiler/test/SQLite/fixture/HTTP/PDF generator/native/browser/Git/Root product/runtime execution or mutation occurred. Root is sole integrator.

Apply exactly four guarded payloads from manifest.json onto Root's sitemap-findings-core-integration-v1 declared basisc8f539be979ac5436dcc962c5ce7677aab9f3b0d. Two existing files require exact before hashes; two new files must not exist. Frozen parent model d7d62bd0 / payload723200ce is unchanged and mandatory. No public export, model or Store change belongs to this handoff. Root owns its existing test parent and findings_store_tests.rs; neither was edited.

## Exact accepted private API

The analysis parent registers the new private findings module and crate-private re-export:

```rust
pub(crate) fn visit_saved_page_candidates(
    prepared: &VerifiedAnalysisRead<'_, '_>,
    raw: Option<&VerifiedRawRead<'_>>,
    coverage: &SitemapCoverage,
    control: &SitemapReadControl,
    after: Option<&SitemapFindingPosition>,
    visitor: impl FnMut(SitemapFindingPosition, SitemapFindingPageCandidate)
        -> anyhow::Result<std::ops::ControlFlow<()>>,
) -> anyhow::Result<bool>;
```

Root invokes this only after actual same-TX full context/Expected comparison from its prepared session. It returns true only when the candidate cursor is exhausted; false when the visitor breaks. It does not count matches, manufacture a page cursor or imply more matching candidates. Unavailable prepared target analysis returns typed SitemapFindingsError::Unavailable, never a valid empty cohort.

window.rs adds only a parent-private borrowed findings_basis accessor. Token fields remain private, with no public constructor/Clone/Serde/SQL exposure. Actual raw-token pointer equality, coverage equality and active Connection transaction are checked again before traversal. Both the prepared shared control and supplied actual control are checked; actual SQL interrupt errors retain the requested typed stop through run mapping.

SavedPage positions are zero-based, as frozen model requires. The reader validates the position role/range and queries actual report json_each membership before selection; ordinal0 is valid if present, actual last ordinal gives a valid empty exhausted suffix, outside/wrong-domain yields typed InvalidContinuation. No synthetic Snapshot or public RichWindow loop is used.

## Shared projection and new typed facts

One ordered actual report SQL candidate cursor remains alive across all batches. At most200 page projections and their extra facts are held in a batch; no whole candidate/raw/source Vec is collected in production. Each batch uses existing page_columns/page_from_row, exact original requested membership/inlink counts, DISTINCT original XML task/observation counts and bounded samples. It does not reprepare/hash the full read context or redundantly reload target pages/frontier for already parsed saved candidates.

The original SavedPage constructor is factored into build_saved_page_findings and called by BOTH old inventory projection and new traversal. Executed text comparisons confirm old PAGE_COLUMNS, page_from_row, page_columns, prepare_context_inner and read_inlinks are unchanged. SavedPage-builder tokens are unchanged apart from moving the old observations-map cleanup ahead of the shared call. No legacy DTO, assessment engine or state-hash profile is extended.

Only the new SELECT appends five tail columns, preserving old21-column offsets. New Candidate carries actual content_type from existing column18; robots_warning field presence (missing differs from explicit null) and actual optional String; PDF field presence, actual null versus object and optional typed ResourceAnalysisStatus. A missing/null status stays unobserved, never an invented default Error or Complete. Present invalid JSON kinds/status values fail instead of becoming absent or healthy. No MIME/warning/PDF status is guessed from text heuristics. Actual raw MIME text is retained for Root's predicate.

## Work, ordering and scope limits

All report/page/link/source/selected guards are still Root's actual common preparation. Internal cardinality200 is not an output-byte/RSS/hard-time bound: SQLite sort/JSON/aggregate workspace and existing coarse selected compatibility can be substantial. Membership/inlink aggregate scans still occur per batch. A visitor may break after its own candidate budget; the rest of that already projected batch is discarded and can be visited on the next actual position. Root's evaluated_candidates counts actual visitor evaluations, not hidden SQL CPU work or projected cardinality.

Root applies exact selector/search and the five tri-state predicates BEFORE selecting user limit+1 matches. A normal matched continuation must use the last RETURNED match, not this reader's loaded batch end or extra lookahead. Sparse candidate-budget stops need truthful incomplete/progress state; false is not has_more=false or matched_total0. Same-TX scopes and inherited age/Unknown qualifiers remain actual existing DTO values.

## Five actual authored regression groups

The new findings module registers its own new findings_tests.rs; no Root test-parent hook is needed. authored-tests.json lists exact names/lines. All five are NOT_COMPILED / NOT_RUN:

1. Actual500-page public Crawler Store, source sibling XML documents and exact original self/nonself anchors. Single requested identities beyond200 and400 are present, same-file duplicates versus distinct task counts are checked, full existing materialized analysis DTOs are an oracle, and break201/resume yields identical order with no skip/duplicate.
2. Actual HTTP-authored complete/two-page-partial/malformed/encrypted PDFs, real parser status preconditions, exact Candidate MIME/status values and observed null PDF on ordinary HTML.
3. Actual robots500 policy-unavailable versus successful Disallow: both native blocked states retain the genuinely distinct optional warning field/value, with no text inference.
4. Explicit historical representation fixtures remove warning/PDF fields or set null; a rehashed report seal lets new typed-column checks reach actual late450 invalid PDF status/warning type instead of merely failing an old SHA. These SQL controls are labelled fixture changes, not permitted native writer or positive row provenance.
5. Actual ordinal0/last/outside/wrong-domain, mid-traversal cancellation after201 callback evaluations with typed Cancelled, and real checkpoint SourceSummary mismatch producing typed Unavailable rather than an empty success.

Positive stores/rows are produced solely by actual public Crawler/parser/Store HTTP paths. SQL controls only alter real reports/checkpoint seals for historical/corruption cases. No positive Header/Window/Session/Candidate or expected membership DTO is synthesized. All request counts, exact sizes, parsed statuses, ordering, type/lifetime compatibility and runtime behavior remain future execution obligations.

No product test pass, accepted new binary, installed behavior, S6 completion, source physical-gate closure or manufacturer predicate parity is asserted.
