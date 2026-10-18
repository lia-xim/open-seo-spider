# Specialist workflows: local browser acceptance

The [local result](local-result.json) records a single passing run of all31
registered suites on Windows/MSVC with Chromium155: nine explicit API-fixture
suites and22 native suites. The application exited naturally with code0;
tracked sources and the executable stayed unchanged. The changed UI/test inputs
are hashed on top of `db5948a`; this pre-commit result is not a clean-commit
compiler receipt. Logs/screenshots remain in the named ignored artifact folder.

Six additional suites exercise the redesigned specialist tools:

- Migration and retained broken-link sources: exact outcomes for redirects,
  missing/invalid mappings, duplicate/conflicting rows,100+9 and100+7 paging,
  and complete verified CSV/JSON exports.
- Directory tree: exact rendered cards and counts, raw/reported profiles,
  100+5 paging, stale snapshots, imported unknown/empty/conflicting observations
  and whole-tree CSV.
- Segments: real builder/save/reload, exact assignments and rendered cells,
  missing-title unknowns, issue views,100+5 paging and bound full exports.
- N-grams: exact visible totals/occurrences, word-cloud drilldown, text/metadata
  sources, n/case/stopword options, incomplete/missing sources, full CSV/JSON
  and stale-generation refusal after an explicit native resume.
- Resource sources:106 failure occurrences over shared native transports,
  100+6 paging, exact page JSON and whole-selection CSV; store-only JavaScript
  remains unknown and an import without native history stays unavailable.
- API enrichment: native preview, acknowledgement, invalidation and refusals
  for invalid input, missing credentials and missing jobs. The frontend now
  displays the backend's string error as well as structured error messages.
  No target is submitted to Google. Successful remote metrics, persisted
  provider outcomes and provider cold reopen are **not proved** here.

Independent read-only reviews found gaps in the first tests. The final versions
compare rendered rows/cards to native responses and check every exported row,
not just selected substrings. The runner also fails its exit status when the
binary digest or final completeness flag fails. Five counterfactual controls of
the actual final assertion block passed; these controls are not a real binary
mutation or a separate browser run.

The public [db5948a run](https://github.com/lia-xim/open-seo-spider/actions/runs/39115539109)
passed renderer and Docker. Ubuntu passed workspace Rust tests and strict
Clippy, then failed the legacy sitemap-ledger fixture. Windows was cancelled
by matrix fail-fast and has no completed acceptance from that run.

The failed legacy fixture deleted its catalog while retaining the newer source
measurements and coverage inventory. The backend correctly rejected this
inconsistent snapshot. The revised fixture removes all dependent modern fields
and separately verifies that each orphan field still causes HTTP400 before
child files or website requests. The original corrupt-catalog control remains.
A direct local run passed all11 scenario groups,11 actual read-only SQLite
witnesses and final drained HTTP counts. This direct run precedes the clean
commit gate; its caller-declared base SHA does not attest source-to-binary
provenance. A fresh strict wrapper and public CI are separate checks.

The [coverage matrix](../../e2e-testing.md) and feature ledger retain the remaining
work: cold native reopen of the new tools, more operators/archives/concurrency,
successful provider integration and full feature acceptance. No catalog feature
was promoted solely because these bounded browser cases passed.
