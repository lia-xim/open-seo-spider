# Open SEO Spider

An independent local SEO crawler written in Rust. **Alpha software:** useful workflows exist, but complete Screaming Frog feature parity and production readiness are not claimed. This project is not affiliated with Screaming Frog.

Public source preview: [v0.1.0-alpha.1](https://github.com/lia-xim/open-seo-spider/releases/tag/v0.1.0-alpha.1). The [GitHub Actions results](https://github.com/lia-xim/open-seo-spider/actions) are tracked separately from the local checks below.

There is no fixed commercial URL allowance. Your configured crawl budgets, storage, memory, network and hardware still limit a crawl.

## What is available

- Native HTTP crawling, robots rules, redirects, internal links, canonical/hreflang observations and configurable SEO checks.
- URL-list imports, CSS/XPath/regex extraction, PDF/image observations and JSON/CSV exports.
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

The [feature ledger](docs/feature-parity.json) tracks 144 grouped requirements: **38 independently accepted, 69 partial and 37 open**. These are project acceptance states, not a percentage of another product's functionality. The [ongoing to-do list](docs/FEATURE_TODO.md) records the remaining work.

See [configuration profiles](docs/configuration-profiles.md) for the versioned browser/API format, exact native settings, explicit header requirements and its distinction from CLI configuration files. The [redesign E2E guide](docs/e2e-testing.md) separates browser/native workflows from fixture-only checks.

The saved-spelling slice has focused evidence for 34 default Rust tests, 40 extracted UI VM groups across related workflows, and two real app-consumer LanguageTool tests using English/German fixtures. Those bounded checks do not prove every UI path, language or external integration. A VM fixture is not a real browser workflow.

The latest local Windows run passed **1,746 ordinary workspace tests plus all four explicitly enabled Chromium fixtures**, with zero failures. All **32 UI suites** passed together (nine API-fixture suites and 23 native suites); strict Clippy passed for all targets/features. See the [custom-search and redesign verification](docs/verification/custom-search-20261018/README.md) for exact hashes, cold restart and scope. The fresh public CI run remains pending; local evidence does not establish universal coverage. Earlier [Source28 proof](docs/verification/source28-open-source-alpha-20261016/README.md) is historical. The installed Windows pilot remains Source16 and was not updated by this work.

Complete authenticated browser sessions, OCR, scheduling and comprehensive external-provider workflows remain incomplete or open. Provider features require your own authorized credentials and remain subject to provider quotas. Proprietary crawl-file compatibility and performance equivalence are not claimed.

A [Docker Compose recipe](compose.yaml) is included, but a recipe is not current container-runtime proof. Raw HTTP and rendered evidence remain separate; missing observations must not be read as successful zero findings.

The API is intended for local use and has no built-in user authentication. Read [SECURITY.md](SECURITY.md) before changing listen addresses or accepting untrusted crawl requests.

## Contributing and license

Read [CONTRIBUTING.md](CONTRIBUTING.md) and the [changelog](CHANGELOG.md). Original project code is [MIT licensed](LICENSE). Bundled fonts and dependencies retain their licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
