# End-to-end checks for the redesigned application

The supported story is: open the redesigned interface, configure a crawl, fetch
owned pages through the Rust backend, inspect the saved result, and export or
reopen that result. Tests must follow the real boundary they claim to cover.

## Run locally

Use the project's Rust toolchain and Node.js 24. Install renderer dependencies
with `npm ci --ignore-scripts` in `renderer/`. Install Chrome/Chromium separately
and optionally set `OPEN_SEO_BROWSER_PATH` to its executable.

```sh
cargo build --locked -p open-seo-spider
node scripts/ui-e2e.mjs
node scripts/renderer-rust-e2e.mjs
```

`SPIDER_BINARY` selects an explicitly built app. The runner never installs or
updates the user's application. It launches a fresh app and renderer on owned
dynamic loopback ports, uses a new test store, and closes its app through stdin
EOF. It refuses the installed application's ports 8787/8790. Runtime evidence
goes to an ignored `artifacts/ui-e2e-*` directory; `SPIDER_E2E_OUTPUT` may select
another **new** output directory. Each suite gets a separate artifact directory.
Tracked files are hashed before/after execution; a changed file fails the run.
The application child does not inherit `OPEN_SEO_PAGESPEED_API_KEY`. The API
enrichment suite exercises preview and refusal paths without sending targets
to Google; it does not establish successful provider measurements.

The separate positive enrichment check builds a **test executable**, then owns
its full native router, a local HTTP provider, a website-request trap and two
native processes using the same fresh store:

```sh
node --test web/verification/api-enrichment-state.test.cjs
node scripts/run-enrichment-browser-checks.mjs
```

It compiles the app test target through Cargo's JSON artifact output and binds
the exact executable, Rust/manifests/web/assets inputs and test drivers by SHA-256.
The immutable `cfg(test)` transport sends actual HTTP to the owned provider
after the ordinary start guards. The normal production provider URL and
credential handling are unchanged. No job or saved result is seeded. The child
processes cannot inherit a PageSpeed key; only isolated loopback ports are used.
`SPIDER_ENRICHMENT_ARTIFACT_DIR` may select a **new** artifact directory; reuse is
refused. Default output is `artifacts/enrichment-success-*`. This command runs
separately from the 32-suite UI runner and is not an additional registered suite.

For a focused investigation:

```sh
node scripts/ui-e2e.mjs --only tool-navigation,saved-spelling
```

The default run includes every registered suite. `--only` does not constitute a
complete E2E pass. The runner stops at the first failure and retains its logs.

The second command executes the four Rust fixtures explicitly ignored by the
ordinary workspace run because they need Chromium. It owns a dynamic renderer
port and closes the worker afterward; no installed renderer is required. CI's
renderer job runs the same command. Report ordinary ignored tests and these
explicit executions separately.

## Coverage and boundaries

| User path | Registered suite | Evidence type |
| --- | --- | --- |
| Dashboard, settings, tables, filters, downloads, responsive controls | dashboard | Real browser with explicit API fixtures |
| Pause/resume, selected recrawl, policy, content and resource edge states | pause-controls, recrawl-controls, policy-controls, content-controls, resource-controls, resource-ledger, resource-contract | Real browser with explicit API fixtures |
| Saved comparison errors and optional bundle controls | comparison-controls | Real browser with explicit API fixtures |
| All nine redesigned tool-menu links and return navigation, desktop/320px | tool-navigation | Real browser and Rust HTML routes; navigation only |
| JavaScript crawl, screenshots, accessibility and DOM capture | rendered-crawl, dom-capture | Browser → Rust → owned renderer → owned HTTP site |
| Resource and cookie observations | resources | Browser → Rust → owned fixture |
| Offline robots and SERP editing/exports | robots, serp | Browser → Rust; website-request witnesses |
| URL-list import and crawl | list-import | Browser → Rust → owned fixture |
| Drain/pause/resume and selected recrawl | pause-resume, selected-recrawl | Browser → Rust → owned HTTP barriers/persistent crawl |
| Policy preview and crawl decisions | policy | Browser → Rust → owned fixture |
| Saved raw/rendered text and content analysis | content | Browser → native saved content |
| Discovery, occurrence/transport data and persisted resources | resource-discovery, resource-persistence | Browser → Rust → owned fixture |
| Saved spelling, explicit transfer acknowledgement, dictionary/profile reload, CSV and provider failure | saved-spelling | Browser → Rust → deterministic local LanguageTool-compatible fixture; not real-engine quality |
| Saved comparison, original-report bundles and cold restart | saved-comparison, comparison-bundles | Owned native processes, browser, saved data and offline HTTP witnesses |
| Migration CSV mapping, redirect outcomes and retained broken-link sources | link-workflows | Browser → native saved report; 109 mappings, 107 link occurrences, pagination and complete verified exports |
| Directory hierarchy, raw/reported views and unknown/imported data | directory-tree | Browser → native saved report; exact rendered cards, 100+5 paging, full CSV and stale tokens |
| Segment builder, persisted rules, assignments and issue views | saved-segments | Browser → native saved text; exact membership/cells, 100+5 paging, JSON/CSV and stale rules/views |
| N-gram totals, occurrences, word cloud and text/metadata channels | saved-ngrams | Browser → native corpus; exact rendered rows, complete CSV, missing sources and real stale-generation refusal |
| Resource/CSS failure occurrences and unavailable history | resource-sources | Browser → native SQLite ledger; 106 occurrences, 100+6 paging, snapshot-bound CSV and page JSON |
| API enrichment preview, acknowledgement, invalidation and error messages | api-enrichment | Browser → ordinary native API; no successful provider calls or persisted remote metrics claimed |
| API enrichment start/poll, exact metrics and unknowns, 100+8 rows, refresh, cancellation, full export and cold reopen | separate enrichment coordinator | Browser → full native test router/service → owned HTTP provider → saved data; production Google transport and successful enrichment CLI remain unproven |
| Custom literal/regex search, separate raw/rendered/selected sources, exact counts, unknowns and limits | custom-search | Browser → Rust → actual Chromium → owned website; saved inspector, complete CSV/JSON, import and owned cold native restart |

The existing native sitemap membership/assessment, API, CLI, MCP and renderer
integration suites remain separate CI gates. Extracted-function VM tests are
also separate; they must not be described as browser E2E tests.

## Remaining important-path work

The 32 registered suites provide bounded functional cases for the specialist
tools above; a passing case is not complete feature acceptance. Cold native
restart across the remaining tools, all rule/operator combinations, broader
malformed imported archives and concurrent mutations need further coverage.
The controlled positive API-enrichment path is now proved by the
[2026-10-18 receipt](verification/api-enrichment-browser-20261018/README.md):
105 unique URLs and 108 supplied rows, exact first-100/tail-8 DOM contents,
five full CSV downloads, refresh as a new job preserving the original four
saved files and CSV, an aborted held provider request, and process-cold reopening
with no repeat provider or website requests. The refresh case specifically
reopens the saved job before a new preview and renewed acknowledgement; it does
not prove universal freshness enforcement on every start. Eight deferred-response regressions
separately cover UI ownership, stale callbacks and invalid snapshot controls.
These VM checks are not browser/native-provider evidence. Authorized production
Google metrics, successful `enrich-urls` CLI, other providers and broader tool
acceptance remain open.
Real LanguageTool language quality, other external providers, future features
and broad cross-browser parity remain separate requirements in
`FEATURE_TODO.md`.

Generated logs/screenshots/results are deliberately not committed over the
historical fixtures. GitHub Actions uploads the new run's evidence separately.

The final local gates for the enrichment slice pass: 1,749 ordinary Rust
tests with zero failures/five deliberate ignores, four separately executed
Chromium fixtures, all 32 UI suites (nine fixture/23 native; natural app exit 0),
strict Clippy and the normal build. The 28 pure bundle controls, 11 native bundle
groups and 12 official SDK checks pass on the normal executable. See the
enrichment receipt for exact source and executable bindings. Process-cold native
crawl/source reopen for segments/n-grams, separate same-origin restoration of
browser-local segment rules, and accessibility journeys preserving incomplete
and needs-review states still need their own acceptance. Server rule persistence
is not claimed.

The latest public run on `01ba979` is
[39120704481](https://github.com/lia-xim/open-seo-spider/actions/runs/39120704481).
It remains a historical FAIL: two Windows timeouts, an obsolete Linux inventory
expectation and an unavailable directory debugger response body despite
immediate capture. The directory test now uses an unconditional independent
native POST artifact oracle together with browser metadata and exact DOM checks.
Both previously failing Windows tests pass locally and CI concurrency was
aligned, but the exact historical five-second drain-timeout cause remains
NOT_PROVEN. The next public CI run is pending; local green does not prove that
public Windows is fixed.
