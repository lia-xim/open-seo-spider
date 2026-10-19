# Open SEO Spider

An independent local SEO crawler written in Rust. **Alpha software:** useful workflows exist, but complete Screaming Frog feature parity and production readiness are not claimed. This project is not affiliated with Screaming Frog.

Public source preview: [v0.1.0-alpha.1](https://github.com/lia-xim/open-seo-spider/releases/tag/v0.1.0-alpha.1). The [GitHub Actions results](https://github.com/lia-xim/open-seo-spider/actions) are tracked separately from the local checks below.

There is no fixed commercial URL allowance. Your configured crawl budgets, storage, memory, network and hardware still limit a crawl.

## What is available

- Native HTTP crawling, robots rules, redirects, internal links, canonical/hreflang observations and configurable SEO checks.
- URL-list imports, CSS/XPath/regex extraction, PDF/image observations and JSON/CSV exports.
- Main result tables with [combined numeric/text filters, stable sorting and saved browser column layouts](docs/table-explorer.md), plus complete filtered CSV exports.
- [Named complete table views](docs/table-views.md) with explicit browser-local save/apply/rename/replace/delete and separately confirmed catalog reset; definitions can be reused on compatible saved crawls.
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

See [named local profiles](docs/configuration-profile-catalog.md) for explicit native save/load/rename/delete and revision conflicts. See [configuration profiles](docs/configuration-profiles.md) for the versioned browser/API format, exact native settings, explicit header requirements and its distinction from CLI configuration files. The [redesign E2E guide](docs/e2e-testing.md) separates browser/native workflows from fixture-only checks.

The saved-spelling slice has focused evidence for 34 default Rust tests, 40 extracted UI VM groups across related workflows, and two real app-consumer LanguageTool tests using English/German fixtures. Those bounded checks do not prove every UI path, language or external integration. A VM fixture is not a real browser workflow.

The latest local Windows acceptance passed **282 state/helper tests**, **40 semantic UI groups**, **17 reported Chromium transport tests (two parent cases and 15 subcontrols)**, **1,831 ordinary workspace tests / zero failures / five opt-in ignores**, format, strict Clippy, four separately enabled Rust/Chromium fixtures and **all 43 UI suites in one complete invocation** (nine API-fixture suites and 34 native-service suites). These are fresh gates on the frozen named-view source and normal executable, not reuse of the earlier product build. The [named-view and full-regression receipt](docs/verification/table-named-views-20261019-r3/README.md) binds 593 source/harness inputs and the exact binary. Its bounded title-table corpus includes two separately captured 107-page crawls, 29 complete table observations and nine actual CSV downloads with 719 data rows, genuine lock/quota/conflict controls, 320 px and a same-origin two-process cold reopen. R1/R2 failures remain recorded. The first R3 whole invocation passed 11 suites and then the dom-capture Node process exited with 3221226505 (0xc0000409), without stderr or timeout. Its native coordinator still closed naturally and source/binary bytes remained unchanged. The unchanged standalone diagnostic passed, and a fresh complete retry passed all 43 suites; neither later success proves the original crash cause. Fresh R3 Rust/Chromium results precede those UI attempts and remain bound to identical sources; they were not rerun for the UI retry. The earlier [table-filter receipt](docs/verification/table-explorer-20261018-r4/README.md) and [profile receipt](docs/verification/configuration-profile-catalog-20261018/README.md) remain historical. The exact public CI for the earlier commit 2281086 passed all four jobs; see the [terminal CI record](docs/verification/table-explorer-public-ci-20261019/README.md). Public CI for this successor source is still pending. The installed Windows pilot remains Source16 and was not updated by this work.

Complete authenticated browser sessions, OCR, scheduling and comprehensive external-provider workflows remain incomplete or open. Provider features require your own authorized credentials and remain subject to provider quotas. Proprietary crawl-file compatibility and performance equivalence are not claimed.

A [Docker Compose recipe](compose.yaml) is included, but a recipe is not current container-runtime proof. Raw HTTP and rendered evidence remain separate; missing observations must not be read as successful zero findings.

The API is intended for local use and has no built-in user authentication. Read [SECURITY.md](SECURITY.md) before changing listen addresses or accepting untrusted crawl requests.

## Contributing and license

Read [CONTRIBUTING.md](CONTRIBUTING.md) and the [changelog](CHANGELOG.md). Original project code is [MIT licensed](LICENSE). Bundled fonts and dependencies retain their licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
