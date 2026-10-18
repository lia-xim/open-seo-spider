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
```

`SPIDER_BINARY` selects an explicitly built app. The runner never installs or
updates the user's application. It launches a fresh app and renderer on owned
dynamic loopback ports, uses a new test store, and closes its app through stdin
EOF. It refuses the installed application's ports 8787/8790. Runtime evidence
goes to an ignored `artifacts/ui-e2e-*` directory; `SPIDER_E2E_OUTPUT` may select
another **new** output directory. Each suite gets a separate artifact directory.
Tracked files are hashed before/after execution; a changed file fails the run.

For a focused investigation:

```sh
node scripts/ui-e2e.mjs --only tool-navigation,saved-spelling
```

The default run includes every registered suite. `--only` does not constitute a
complete E2E pass. The runner stops at the first failure and retains its logs.

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

The existing native sitemap membership/assessment, API, CLI, MCP and renderer
integration suites remain separate CI gates. Extracted-function VM tests are
also separate; they must not be described as browser E2E tests.

## Remaining important-path work

Tool-menu navigation is not functional acceptance of migration, directory-tree,
saved segments, n-gram, broken-source or provider-enrichment workflows. Those
specialist flows need their own actual UI/API/store/export cases in this runner,
including stale revisions, pagination and cold reopen. Existing Rust tests and
older bounded native reports are useful evidence but do not replace current
redesign E2E execution. Real LanguageTool language quality, external authorized
providers, all future features and broad cross-browser parity remain separate
requirements in `FEATURE_TODO.md`.

Generated logs/screenshots/results are deliberately not committed over the
historical fixtures. GitHub Actions uploads the new run's evidence separately.
