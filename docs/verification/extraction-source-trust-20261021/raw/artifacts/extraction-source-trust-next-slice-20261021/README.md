# Extraction source trust: read-only next-slice proposal

Research date: 2026-10-21. This is an ignored preparation artifact. No tracked
source edits, native server/browser/worker, Cargo, or Git mutation was performed
by its author. Current existing native evidence remains owned by the root agent.

## Concrete user problem

A user configures a CSS/XPath/regex extraction, crawls a response that is cut at
the configured body limit or decoded with replacement characters, chooses
`Eigene Extraktionen` and `Ohne Werte`, then downloads CSV/XLSX. The extraction
row exposes `matched_count = 0`, `truncated = false`, no error, and `raw_html`.
Its page may separately carry a truncation/read error, but the detached main and
native extraction CSV rows omit those source facts. A legacy zero and an
authoritatively evaluated complete-source zero also have no per-result evidence
distinction. This is a source-trust gap, not a workbook-encoder gap.

The current fields are not inherently incorrect: `matched_count` is explicitly
the number of extractable values in the evaluated input before `max_results`;
`truncated` means retained-result truncation only. Preserve that contract.
`docs/advanced-extraction.md` already warns that a truncated HTTP response cannot
prove absence. The missing feature is carrying that warning as machine-readable
source evidence into the actual result/filter/export workflow.

## Exact current path

- `models.rs:929`: `CustomExtraction` has values, source, engine, result type,
  optional matched_count, result truncation, and error; no source observation.
- `crawler.rs:3691-3716`: a body limit sets `page.truncated`; body read errors set
  `page.error` but leave any captured prefix available.
- `crawler.rs:3849-3861`: decoding records exact/lossy/unknown in a local variable.
- `crawler.rs:3941-3957`: HTML extraction is applied even when the page is
  truncated, body-read-failed, non-200, or decoded uncertainly. This is useful
  partial evidence, but the result does not carry its scope.
- `extraction_engine.rs:469`: every successful evaluation starts with known
  input matched_count 0; append/node-set code measures all input values while
  retaining at most max_results. Error results retain unknown matched_count.
- `renderer.rs:101,150-177,249`: raw metadata is saved before rendering. Success
  replaces current extractions from DOM; failure keeps raw results. Extraction
  runs before the rendered HTML eligibility checks later used by Custom Search.
- `app.js:864,912,945,976,1383`: table flattens stored results, has no source-state
  column, `empty` means `!error && values.length === 0`, and `truncated` filter
  only checks the result-budget flag. Inspector similarly has only that flag.
- `export.rs:667-745`: native CSV emits a retained empty/error extraction row but
  no page-body/decode/source-completeness facts. Main CSV/XLSX uses the same
  table projection. `server.rs:2284` reads the saved report and delegates native
  CSV; it does not reconstruct source evidence.

## Important semantic constraints

1. Partial input counts are not a generic lower bound for the final document.
   Regex `</p>$` can match a truncated prefix but not the full response; CSS
   `body > p:last-child` can select a different node after the remainder arrives;
   XPath `count(...)`, `last()`, negative predicates and string values can change.
   Keep the original partial-input values, but explicitly qualify their scope.
2. Result truncation is independent of source completeness. Complete source with
   3 extracted values and max_results 2 has known input count 3 and source complete
   despite retained values truncated. Partial source with no retained values is
   not a confirmed complete-source zero even when result.truncated is false.
3. `source` names alone (`raw_html`/`rendered_html`) do not prove completion.
4. Raw source completion is determined before optional rendering. A renderer
   failure does not invalidate a complete exact native raw response. Do not copy
   the current `completeDOM` predicate with its blanket `!renderer_error` check.
5. A 404 HTML entity can be completely observed even though it is not indexable.
   Decide entity/source completeness independently from successful HTTP/SEO
   eligibility. A 206/range entity, 204/205/304 no-content, read failure, and
   incomplete wire observation require explicit semantics; do not accidentally
   promote status >= 200 to a full-resource claim. Native API/body facts must
   supply the authority, not a label inferred later in App.
6. Decoded source SHA-256 and byte length describe the exact input string,
   not wire bytes, original HTML serialization, website truth or import origin.

## Proposed complete additive contract (root decision before implementation)

Add an optional per-result `source_observation` object, default None and omitted
when absent, so old reports and public callers preserve legacy serialized bytes.
No report reader may reconstruct the object from current config or default
booleans. Version/engine identifies the evidence contract.

The object needs independently representable fields for:

- source status `complete | incomplete | unavailable`, with bounded reasons;
- evaluation status `evaluated | error | not_run` (or equivalent independent
  state); evaluation error remains the existing error field;
- decode status `exact | lossy | unknown` (rendered DOM string can be exact UTF-8
  while original wire decode is a separate browser concern);
- exact evaluated decoded input UTF-8 length and SHA-256 when input exists;
- original requested/document URL, observed generation and a source revision or
  time binding sufficient to distinguish raw and DOM observations;
- source kind/identity must match existing result.source; invalid imported
  objects are unknown/error rather than an authoritative zero.

`None` means legacy/uncaptured trust, not complete. Preserve `values`,
`matched_count`, `truncated`, errors and existing configuration semantics.
Do not change public `extract_configured(html, config, source)` or `extract_custom`
into trusted native-capture constructors just because the caller supplies a
string. They lack native capture context: either they leave observation absent
or a separate pure helper records an explicitly caller-supplied input scope.

Derived exact whole-source count is known only for source complete + successful
complete evaluation. For partial source it stays null; the old matched_count is
the count on the evaluated input, not a whole-resource lower bound. Preserve
literal empty string, scalar numeric `0`, scalar `false`, and empty node set as
four different outcomes. An XPath scalar returns one extracted value even when
its value is zero/false/empty.

Expose the source state/reason and count scope in table + inspector + CSV/XLSX,
with appended native CSV columns to retain existing column positions. Add a
confirmed-zero filter that requires complete source; keep legacy `Ohne Werte`
only if clearly labelled as retained-value absence, otherwise migrate it with
explicit compatibility documentation. Do not call old/prefix counts exact.
Result-limit filter should be labelled as retained-result truncation; source
incomplete and unknown need their own filters. Main export should carry trust
evidence for the exported row and must not silently drop missing configured
observations. A no-config source can refuse export with a clear reason; a known
complete source narrowed to zero rows can still export legitimate headers.

For failed/skipped/non-HTML native captures, the implementation must explicitly
choose and document whether every configured extraction receives an unavailable
result or a separate page source-state sentinel. An absent result cannot become
an evaluated zero. Avoid inventing PageRecords, values or generations.

Current selected-recrawl validation checks content/search evidence but not custom
extraction metadata. Validate any newly present source_observation on imports,
selected-parent admission and native storage read, while preserving absent
legacy objects. Do not claim input hashes authenticate an imported website.

## Regression first, then native acceptance

Before fixing the source, add failing actual-App and native core regressions:

1. Body-limited HTML has a prefix ending `</p>` and a suffix containing another
   sibling. CSS last-child, XPath count/last and anchored regex demonstrate why
   prefix results are neither final positives nor generic lower bounds.
2. Complete missing selector is complete zero; truncated missing selector has
   input count 0 but unknown full-source total. Complete source/max_results=2
   with 3 values is a separate retained-evidence case with known count 3.
3. Explicit UTF-8 invalid bytes, unsupported charset, and literal U+FFFD are
   distinct. Literal U+FFFD can be exact; invalid bytes are lossy; unsupported
   charset is unknown. Reuse content-capture fixture patterns, not a mock success.
4. Partial body read after real prefix delivery preserves incomplete source
   reason and never becomes a confirmed zero. Empty known full HTML is positive.
5. Genuine browser raw price10 -> rendered price20 carries two independent source
   observations. Fail a real worker navigation/runtime attempt and prove complete
   raw fallback stays complete/raw while renderer_error remains visible.
6. A loaded legacy report with only the new observation member omitted keeps
   original values/count/flags and remains unknown. Preserve source/request/
   imported bytes and label deliberate omission as a negative historical input.
7. Wrong source identity/status/count/hash bindings in declared negative imports
   must not pass complete-source/zero filters. Keep input budgets bounded; one
   malformed imported object must not crash the table/inspector/export.
8. Prove native CSV old column prefix and existing field values unchanged;
   appended trust columns match stored evidence. Main CSV formula protection and
   XLSX scalar types/null/empty remain deliberately different representations.

Native E2E must include settings -> real native crawl -> actual results/filter ->
inspector -> browser CSV/XLSX download -> saved JSON -> cold reopen. Use >50
extraction rows, stable rule/source/original ordinal ordering, full export beyond
visible pagination, typed query AND/grouped OR/current projection, true knownzero
and incomplete/legacy distinctions, source identity, result-budget truncation,
one selected refresh plus inherited rows, and profile-visible/raw segment scope
if current APIs support it. Keep native 240s default; split coherent suites when
needed rather than increasing deadlines or weakening assertions.

Expected membership, values/counts and evidence come from actual captured report
bytes + fixture protocol transcripts/config + original ordinals, independently
of product projection/compiler/writer and not merely from CSV. Read actual XLSX
through the independent bounded ZIP/XML oracle, including typed numbers/bools,
literal empty strings, nulls and all provenance cells. Exact SHA/length/native
download source and unchanged original report bytes are required.

Use fresh owned store and same binary across natural EOF0/cold reopen. Stop the
fixture origin for saved-read windows, witness real 410, assert zero hits during
analysis/export/import/reopen, and explicitly reopen origin only for authorized
selected refresh. Source failures use real bad transport/bytes/worker failures;
positive captures must never be fulfilled with mocked HTTP success.

## Wider matrix retained

Custom Search: captured configured results already carry complete/incomplete/
unavailable exact-count/evidence contracts. Disabled/legacy main table becomes
empty despite native specialist CSV sentinel; keep as a separate UI/export task.

Cookies: configured flag + rendered success can support an empty capture in new
native worker results, but absent/default [] legacy vectors are ambiguous. No
per-page capture-state object currently proves optional cookie capture happened.
The current UI/native exports flatten vectors and cannot export unknown pages.
Cookie values must stay absent; preserve only allowlisted attributes/value_length.

JavaScript: configured scripts produce one success/error observation each in a
successful worker; individual errors do not stop later scripts. A hard script
timeout invalidates the whole rendering result, leaving no captured result rows.
No scripts/no renderer/renderer failure/legacy missing arrays all flatten empty.
Any capture-state followup must preserve the intentional total-render failure
semantics and script isolation, not synthesize successful null values.

Accessibility: already emits per-page missing/renderer_error/error/malformed/
observed_zero/needs_review/violations states and null counters. This is a useful
reference; additional actual XLSX journeys are coverage work, not a claim that
source classification is missing. Existing native suite covers58 rows and cold
browser/storage proof for saved axe data, separate from new XLSX depth.

All29 main schemas, specialist/cursor/bulk exports, policy/content/resource/
sitemap feature breadth, body/header partial cases, DOM/resource occurrence
identity, preferences and real GUI/provider/large-file boundaries remain open at
their existing criteria. This extraction slice closes none of them by proxy and
does not change the broader144-feature/416-filter parity denominator.
