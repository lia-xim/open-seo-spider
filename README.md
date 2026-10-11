# Open SEO Spider

Independent local SEO crawler in Rust. The objective is broad functional parity
with desktop SEO crawlers without a commercial URL cap. This repository is
currently private and under active development.

**Full Screaming Frog feature parity is not achieved or claimed.** The acceptance
ledger tracks 144 grouped requirements across 16 product areas; these must still
be expanded into individual audit rules and verified against representative
fixtures. See [feature catalog](docs/feature-catalog.md),
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

List mode uses `--list` and one or more explicit URL arguments. `max_pages` is
optional; the default has no product-imposed page limit. Crawls obey robots.txt
by default and pace requests by origin. Optional include/exclude regular
expressions filter discovered URLs. Named CSS extractions retain every match
and can export one value per CSV row. The default HTTP path does not run JavaScript.

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

## MCP and local API

The running application exposes stateless Streamable HTTP MCP at
`http://127.0.0.1:8787/mcp`. For stdio, configure a client to execute:

```sh
open-seo-spider mcp --server http://127.0.0.1:8787
```

Eleven tools cover crawl start/status/list/cancel/resume, bounded pages, issues,
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
runtime in the same local network namespace. Docker runtime validation remains
**NOT PROVEN** on this machine; the native Windows application is the tested path.

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
