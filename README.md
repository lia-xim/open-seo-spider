# Open SEO Spider

An independent local SEO crawler written in Rust. **Alpha software:** useful workflows exist, but complete Screaming Frog feature parity and production readiness are not claimed. This project is not affiliated with Screaming Frog.

Public source preview: [v0.1.0-alpha.1](https://github.com/lia-xim/open-seo-spider/releases/tag/v0.1.0-alpha.1). The [GitHub Actions results](https://github.com/lia-xim/open-seo-spider/actions) are tracked separately from the local checks below.

There is no fixed commercial URL allowance. Your configured crawl budgets, storage, memory, network and hardware still limit a crawl.

## What is available

- Native HTTP crawling, robots rules, redirects, internal links, canonical/hreflang observations and configurable SEO checks.
- URL-list imports, CSS/XPath/regex extraction, PDF/image observations and JSON/CSV exports.
- Main result tables with [combined numeric/text filters, stable sorting and saved browser column layouts](docs/table-explorer.md), plus complete filtered CSV exports.
- [Named complete table views](docs/table-views.md) with explicit browser-local save/apply/rename/replace/delete and separately confirmed catalog reset; definitions can be reused on compatible saved crawls.
- [Portable view definition files](docs/table-view-portability.md): separate v1/v2 selected/all JSON exports, complete import preview and explicit skip/replace/copy decisions; import does not automatically apply a view.
- SQLite-backed crawl state, pause/resume, selected recrawl and saved snapshot comparison.
- Sitemap discovery, parsing, findings, saved-source reporting and sitemap generation.
- Opt-in saved raw/rendered HTML text, content cohorts, segments and n-gram reports, without refetching websites for saved analysis.
- Native TLS/custom-CA and opt-in HSTS controls. Combining these active controls with browser rendering is currently refused.
- An optional Chromium worker for JavaScript rendering, screenshots, accessibility observations and custom JavaScript.
- Opt-in [literal and regex search](docs/custom-search-plan.md) over raw/rendered HTML and explicitly captured text, with saved source-specific counts, bounded evidence and CSV export.
- A local API and **32 MCP tools**, plus **five optional file tools** restricted to an explicitly configured directory.

See the [feature catalog](docs/feature-catalog.md), [crawl control](docs/crawl-control.md), [saved comparison](docs/saved-comparison.md), [sitemap generation](docs/sitemap-generation.md) and [MCP file-tool contract](docs/mcp-file-tools.md).

Saved-HTML spelling/grammar analysis is a bounded slice. It requires explicit acknowledgement and an independently installed local LanguageTool service. The service is off by default; the application does not bundle an engine/JRE, launch it automatically, or fall back to a cloud provider. Failed or incomplete checks remain unknown. This does not establish complete spelling/grammar support, PDF or title/meta checking, or broad language-quality parity.

## Run from source

Development baseline: **Rust 1.95, edition 2024**, with a native compiler/linker. Windows development uses MSVC Build Tools. Docker is optional.

```sh
cargo run -p open-seo-spider -- serve --listen 127.0.0.1:8787
```

Open <http://127.0.0.1:8787>. The default local store is `data/`; keep it out of version control. For a CLI crawl:

```sh
cargo run -p open-seo-spider -- crawl https://example.com --output crawl.json --csv pages.csv
```

Use `cargo run -p open-seo-spider -- --help` for command options. Check scope and request limits before starting a large crawl.

### Optional browser worker

Use Node.js 24 and an existing Chrome, Edge or Chromium installation:

```sh
cd renderer
npm ci --ignore-scripts
npm start
```

The worker defaults to `127.0.0.1:8790`; enable rendering in the app. It does not download a browser. See the [renderer contract](renderer/README.md) for configuration, isolation and limits. Browser jobs do not share an authenticated session.

### MCP

The running app exposes Streamable HTTP MCP at <http://127.0.0.1:8787/mcp>. For stdio, run:

```sh
cargo run -p open-seo-spider -- mcp --server http://127.0.0.1:8787
```

The local HTTP app must already be running. File tools require the explicit `--mcp-files-dir` option; grant only a directory you intend the client to access.

## Current limits and verification

The [feature ledger](docs/feature-parity.json) tracks 144 grouped requirements: **39 independently accepted, 68 partial and 37 open**. These are project acceptance states, not a percentage of another product's functionality. The [ongoing to-do list](docs/FEATURE_TODO.md) records the remaining work.

[Portable-view acceptance](docs/verification/table-view-portability-20261019/README.md) freshly passes the new R2 native suite: four groups, 22 JSON files, 27 CSV files with 2,626 data rows and an independent 26,260-cell check. R1 remains FAIL with 15 other suites passed; R2 reuses those successes and unchanged 397 Node, 40 semantic and 22 Chromium controls, without a combined 16-suite PASS. App533 with native_enrichment_browser_fixture ignored, four separately executed Core renderer integration tests, format and Clippy are fresh R2 checks; the renderer tests do not run that App opt-in. The 1,298 ordinary Core passes with four ignores are explicitly reused historical R4 evidence, not a fresh workspace run. The exact published successor `66dfe5c` now passes all four public jobs and the 45 registered UI suites in one unfiltered invocation ([terminal receipt](docs/verification/table-view-portability-20261019/public-ci-66dfe5c-20261021/receipt.json)). An additional local full45 run remains NOT_RUN. Native preferences and the full ui.filter-details-export workflow remain open; XLSX has separate bounded acceptance below.

[Main-table XLSX acceptance](docs/verification/table-xlsx-export-20261021/README.md) checks 30 actual XLSX and 30 CSV files, 3,020 data rows and all 35,994 physical cells through an independent original-report Python/XML/openpyxl audit. Fresh checks cover 533 App tests with one deliberate browser opt-in ignored, format/build/Clippy, 459 selected Node tests and 40 semantic groups. One complete focused R3 invocation passes 18 suites (nine API fixtures and nine native suites); both earlier 16-pass failures remain recorded for obsolete asset guards, with all functional/data oracles retained. The 605 selected inputs bind this slice, including separately qualified harness and whitespace-policy changes; compiled product inputs and binary match the fresh Rust gates. Ordinary Core tests and renderer opt-ins were not freshly run here. The complete local 46-suite run is NOT_RUN and exact successor public CI is PENDING. Other tables/provenance/specialist/cursor/bulk paths, large streaming files, Excel GUI and manufacturer parity remain open.

See [named local profiles](docs/configuration-profile-catalog.md) for explicit native save/load/rename/delete and revision conflicts. See [configuration profiles](docs/configuration-profiles.md) for the versioned browser/API format, exact native settings, explicit header requirements and its distinction from CLI configuration files. The [redesign E2E guide](docs/e2e-testing.md) separates browser/native workflows from fixture-only checks.

The saved-spelling slice has focused evidence for 34 default Rust tests, 40 extracted UI VM groups across related workflows, and two real app-consumer LanguageTool tests using English/German fixtures. Those bounded checks do not prove every UI path, language or external integration. A VM fixture is not a real browser workflow.

Nested AND/OR table queries and separately saved grouped views now have [three successful native E2E repetitions](docs/verification/table-query-readiness-20261019-r6/README.md), each with two153-page captures,45 complete table observations and9 actual CSV files/927 rows. The [screenshot capture follow-up](docs/verification/screenshot-response-harness-20261019-r5/README.md) passed330 pure tests,40 semantic fixture groups,22 actual Chromium controls and the native screenshot workflow. R6 reuses these unchanged inputs and the1831 Rust passes/four opt-in Chromium fixtures from R4.

The [original full44 attempt](docs/verification/table-query-expressions-20261019-r4/README.md) remains FAIL:43 suites passed; the final driver checked results before the selected crawl data had loaded. The corrected driver preserves all data and first-page assertions. The exact earlier published successor `8d9d7cc` subsequently passed all 44 registered UI suites in one [public Renderer invocation](docs/verification/table-view-portability-20261019/prior-ci8d/table-query-ci-8d-20261019-renderer-receipt.json). All four public jobs subsequently completed successfully; see the separate [terminal receipt](docs/verification/table-view-portability-20261019/prior-ci8d-terminal/receipt.json). That result covers the earlier source; successor `66dfe5c` now has its own separate complete 45-suite public acceptance linked above. The [parent public CI](docs/verification/screenshot-response-harness-20261019-r5/parent-ci/terminal-capture.json) failed in Renderer after40 passing suites; Windows,Ubuntu and container passed. Earlier [named-view43-suite](docs/verification/table-named-views-20261019-r3/README.md) and [table-filter](docs/verification/table-explorer-20261018-r4/README.md) results retain their original scope. The installed Source16 pilot is unchanged.

Complete authenticated browser sessions, OCR, scheduling and comprehensive external-provider workflows remain incomplete or open. Provider features require your own authorized credentials and remain subject to provider quotas. Proprietary crawl-file compatibility and performance equivalence are not claimed.

A [Docker Compose recipe](compose.yaml) is included, but a recipe is not current container-runtime proof. Raw HTTP and rendered evidence remain separate; missing observations must not be read as successful zero findings.

The API is intended for local use and has no built-in user authentication. Read [SECURITY.md](SECURITY.md) before changing listen addresses or accepting untrusted crawl requests.

## Contributing and license

Read [CONTRIBUTING.md](CONTRIBUTING.md) and the [changelog](CHANGELOG.md). Original project code is [MIT licensed](LICENSE). Bundled fonts and dependencies retain their licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
