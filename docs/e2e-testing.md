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
| Custom literal/regex search, separate raw/rendered/selected sources, exact counts, unknowns and limits | custom-search | Browser → Rust → actual Chromium → owned website; saved inspector, complete CSV/JSON, import and owned cold native restart |

The existing native sitemap membership/assessment, API, CLI, MCP and renderer
integration suites remain separate CI gates. Extracted-function VM tests are
also separate; they must not be described as browser E2E tests.

## Remaining important-path work

The 32 registered suites provide bounded functional cases for the specialist
tools above; a passing case is not complete feature acceptance. Cold native
restart across the remaining tools, all rule/operator combinations, broader
malformed imported archives and concurrent mutations need further coverage.
API enrichment still needs start/poll/persist/reopen success with a synthetic
provider boundary, followed by a separately authorized real-provider check.
Real LanguageTool language quality, other external providers, future features
and broad cross-browser parity remain separate requirements in
`FEATURE_TODO.md`.

Generated logs/screenshots/results are deliberately not committed over the
historical fixtures. GitHub Actions uploads the new run's evidence separately.
