# Open SEO Spider

Independent local SEO crawler in Rust. The objective is broad functional parity
with desktop SEO crawlers without a commercial URL cap. This repository is
currently private and under active development.

**Full Screaming Frog feature parity is not achieved or claimed.** The acceptance
ledger tracks 144 grouped requirements across 16 product areas and now 416 named
filters observed in the installed original CLI. Granular acceptance contracts
and actual comparisons on an independently authored local website are retained
in [reference evidence](docs/parity/reference-comparison.md) and the
[rule backlog](docs/parity/granular-acceptance.md). Matching exported cells are
not a measure of complete feature coverage. See [feature catalog](docs/feature-catalog.md),
[parity ledger](docs/feature-parity.json), and
[observed installed architecture](docs/research/installed-architecture.md).

## Local run

Requires Rust with a native compiler toolchain. Windows is supported with MSVC
Build Tools; Docker is optional.

```sh
cargo run -p open-seo-spider -- serve
```

On Windows, double-click **Open SEO Spider.cmd**. The launcher uses the compiled
release binary and starts the optional browser worker when its dependencies are
installed. Alternatively, run `scripts/start.ps1`; `scripts/stop.ps1` stops the
recorded application and browser worker processes.
If the optional browser worker cannot start, the launcher warns and still starts
the HTTP application; JavaScript rendering remains unavailable until the worker
is running.

Open `http://127.0.0.1:8787`. Crawl state stays in the ignored `data/` directory.
Completed snapshots, cancelled crawls and interrupted SQLite frontiers survive
a restart. Imported snapshots are readable but do not contain a resumable frontier.

```sh
cargo run -p open-seo-spider -- crawl https://example.com --output crawl.json --csv pages.csv
cargo run -p open-seo-spider -- crawl --sitemap https://example.com/sitemap.xml
```

List mode accepts explicit URL arguments and local TXT/CSV/XML/XLSX files, with
original rows, duplicates and invalid values retained separately from network work.
Use the upload preview in the UI, or the byte-only `import-list` command. XLSX
requires explicit sheet/column options; formulas default to rejection.

```sh
open-seo-spider import-list urls.txt --output input.json --csv input.csv
open-seo-spider import-list urls.xlsx --inspect
open-seo-spider crawl --list-file urls.txt --config crawl-config.json --output crawl.json --input-csv inputs.csv
```

`--config` loads native JSON settings; unset flags keep their configured values.
`--list-file` selects list mode, and `--list-options` supplies a native JSON import
selection for CSV/XLSX. Importing makes no website request; crawl start is explicit.
See [list import contracts and resource limits](docs/list-imports.md).
`max_pages` is
optional; the default has no product-imposed page limit. Crawls obey robots.txt
by default and pace requests by origin. Optional include/exclude regular
expressions filter discovered URLs. Named CSS, XPath1.0 and Rust-regex
extractions retain text, attributes, inner/outer HTML and supported XPath scalar
values with their source, result type, observed count,
capture bound and errors. XPath uses the repaired HTML DOM and explicit foreign
namespace bindings; id(), lang(), variables and extension functions are not
supported. Regex does not support lookaround or backreferences.
The default HTTP path does not run JavaScript.

HTML and HTTP Link-header canonical/hreflang declarations remain distinguishable,
including PDF/non-HTML response relationships, base-URL resolution and alternate
anchor contexts. Page-level indexing directives are evaluated for a configured
bot separately from the request User-Agent. Configurable onpage/URL/link
thresholds can be disabled individually. Title/description pixel widths use an
openly licensed Roboto profile; complex shaping and missing glyphs remain unknown.
The fixed 150% measurement profile is calibrated against the retained Windows
reference fixture; other display settings and universal font equivalence remain
unproven. See [pixel calibration](docs/research/text-width-calibration.md).

Native PDF observations include Info metadata, bounded page text and HTTP(S)
annotation-link discovery. Encryption, malformed content and extraction limits
remain explicit rather than turning into empty successful text. Fetched image
dimensions are separate from declared HTML image attributes. Dedicated PDF,
image-resource and intermediate-response views and CSV endpoints preserve that
evidence. See [resource contract](docs/resource-analysis.md).

The [offline SERP editor](http://127.0.0.1:8787/serp) imports CSV metadata,
preserves duplicate URL rows and ordering, supports editing and local drafts,
and exports approximate measurements without requesting the entered websites.

Captured DOM observations retain heading order, image alt text, meta keywords,
link locations, explicit CSS position rules, responsive image candidates and
nonstandard navigation attributes. Declared image candidates do not imply the
browser selected or fetched them. Optional fragment validation checks only
captured, complete HTML targets; unavailable targets remain unknown. These
observations are available in the URL inspector, dedicated tables and CSV.
See [DOM contracts and unknown states](docs/dom-observations.md).

The [offline robots.txt tester](http://127.0.0.1:8787/robots) evaluates supplied
rules and URLs with the crawler's own matcher. It makes no website requests;
invalid rows and unsupported directives remain explicit. See
[matcher scope and limits](docs/robots-simulation.md).

## JavaScript rendering

Install the optional worker dependencies once and use an installed Chrome/Edge:

```sh
cd renderer
npm ci --ignore-scripts
npm start
```

Enable JavaScript rendering in the application's settings. The worker runs at
`http://127.0.0.1:8790/render`. Raw HTTP evidence and rendered DOM observations
remain separate. Optional PNG screenshots and axe accessibility findings appear
in the URL inspector and project JSON. See [worker contract](renderer/README.md).

Optional custom async JavaScript evaluates named function bodies after the render
wait. Per-script results/errors and redacted cookie attributes have dedicated
UI tables and CSV exports. Cookies remain isolated per URL; this does not provide
a shared authenticated browser session.

CSV exports prefix potentially active spreadsheet formulas with an apostrophe.
Typed numeric fields remain numeric; exact captured strings remain in project
JSON.

## MCP and local API

The app supports graceful pause: stop admitting URLs, finish active native/browser
work, save a checkpoint, then resume the remaining frontier. Page and sitemap
discovery queues are stored separately. SQLite-only recovery and immutable report
revisions are described in [crawl control](docs/crawl-control.md). An exhausted
operator limit can be explicitly changed when resuming; it is not a commercial
URL allowance. The MCP tool `spider_pause` exposes the same lifecycle.

Selected recrawl creates a new child job from a stable source snapshot. Select
exact recorded requested URLs in the page table, or use
`open-seo-spider recrawl parent.json https://example.com/page --output child.json`.
Only selected targets are admitted; source seeds, sitemaps and new outward links
are not traversed. The parent and unselected observations are retained, pending
selected old rows are excluded, and JSON/CSV disclose mixed-age coverage. Its
optional `max_attempts` budget is separate from the source page-count limit.
See [selected recrawl](docs/selected-recrawl.md) for identity, recovery and export
contracts. `spider_recrawl` exposes the same derived-job operation over MCP.

`open-seo-spider preview-policy policy.json --output decisions.json` and
`POST /api/policy/preview` evaluate explicit scope/query/rewrite rules without
network requests. [Policy preview](docs/crawl-policy.md) is a separate pure engine;
its rules are not yet applied by the live crawler.

The running application exposes stateless Streamable HTTP MCP at
`http://127.0.0.1:8787/mcp`. For stdio, configure a client to execute:

```sh
open-seo-spider mcp --server http://127.0.0.1:8787
```

Thirteen tools cover crawl start/status/list/pause/cancel/resume/selected recrawl, bounded pages, issues,
analysis, snapshot retrieval and comparison. The browser application must be
running. JSON API routes and acceptance limits are documented in
[architecture](docs/architecture.md). MCP implementation follows the
[official transport specification](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports).

## Docker

```sh
docker compose up --build -d
```

The service publishes only to loopback and stores crawls in a named volume.
This image packages the Rust HTTP application. A Chromium worker needs its own
runtime in the same local network namespace. Linux CI verifies the image build,
unprivileged runtime, offline font measurements and a persistent crawl snapshot
after a container restart. Docker installation and runtime remain **NOT PROVEN**
on this Windows machine; the native Windows application is the tested local path.

## Validation

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
node scripts/api-e2e.mjs
cd scripts && npm ci --ignore-scripts && node mcp-e2e.mjs
cd ../renderer && npm ci --ignore-scripts && npm test
```

The implementation is original. No vendor binaries, proprietary source/assets,
license bypass, credentials or private crawl data belong in this repository.
External service integrations will require the user's legitimate credentials
and remain subject to those services' availability and quotas.

License: MIT for the original project code. Third-party dependencies retain
their own licenses.
