# Independent application architecture

```mermaid
flowchart LR
    UI[Local browser interface] --> API[Rust Axum API]
    MCP[MCP stdio or HTTP client] --> Bridge[Original MCP bridge]
    Bridge --> API
    API --> Core[Tokio crawl scheduler]
    Core --> Robots[Robots and per-origin pacing]
    Robots --> HTTP[Reqwest HTTP requests]
    HTTP --> DOM[HTML metadata and CSS extraction]
    HTTP --> Worker[Optional local Chromium worker]
    Worker --> DOM
    Core <--> DB[(SQLite pages and durable frontier)]
    DOM --> Audit[Original audit rules]
    Audit --> Graph[Graph and relationship analysis]
    Graph --> Export[CSV / XML / project JSON]
    Export --> API
```

The crawler fetches HTTP URLs concurrently, normalizes URL fragments without
merging arbitrary query variants, and limits request starts per origin. Scope,
robots, include/exclude filters, depth and body limits are checked explicitly.
Default page count is unlimited. Physical memory/disk and user-selected limits
still apply; the current UI/server retains page observations in memory as well
as SQLite. Million-URL throughput and memory scale have not been benchmarked.

SQLite commits a completed page and its discovered links atomically. In-flight
URLs remain pending until their result is committed. A job manifest makes
unfinished crawls visible as interrupted after application restart. Reports and
manifests use temporary-file replacement. Finished reports merge the independent
relationship/security audits before exports and UI retrieval.

An optional browser worker runs each page in an isolated context. The native
response status/headers remain unchanged; DOM metadata, links and custom
extractions use the rendered document, with raw metadata retained separately.
Browser failure is recorded explicitly. Robots-disallowed pages do not invoke
the browser. Browser navigation/subresource traffic has independent timing and
does not claim identical behavior to the HTTP crawler or reference product.

## API surface

| Route | Purpose |
|---|---|
| `GET /healthz`, `/api/capabilities` | Process readiness and implemented/optional capability facts |
| `GET/POST /api/crawls` | Saved jobs / start crawl with `CrawlConfig` |
| `GET /api/crawls/{id}` | Status, warnings, configured scope and counters |
| `GET /api/crawls/{id}/pages?offset=0&limit=200` | Bounded page observations, optional `q` and `internal` |
| `GET /api/crawls/{id}/issues`, `/analysis` | Audit findings and graph/JSON-LD facts |
| `POST /api/crawls/{id}/cancel`, `/resume` | Durable lifecycle control |
| `GET /api/crawls/{id}/export.csv`, `/issues.csv`, `/links.csv`, `/extractions.csv` | Observed page, issue, link and CSS-value exports |
| `GET /api/crawls/{id}/sitemap.xml`, `/project.json` | Sitemap candidates / own project snapshot |
| `POST /api/crawls/import` | Import an Open SEO Spider snapshot |
| `GET /api/compare?before={id}&after={id}` | Changes in observed fields between snapshots |
| `POST /mcp` | Stateless Streamable HTTP MCP; GET returns 405 |

The service is intended for loopback access. Host and browser Origin checks
protect local mutations. Custom crawl headers go only to explicitly configured
seed origins; robots and external requests do not inherit them. The browser
worker separately checks every redirect hop before adding custom headers.
There is no persistent browser login/profile import or provider credential UI.

## Evidence boundaries

Deterministic HTTP/API tests prove more than 500 URL observations, cancellation,
resume after process termination, exports, import and comparison. Unit/integration
tests and browser fixture results are retained beside their reproducible scripts.
Graph scores are original PageRank-style calculations. JSON-LD facts are parse
and type observations, not a complete rich-results validator. Axe findings are
automated page checks. Compatibility with proprietary project files, every
reference audit rule, all providers, cross-platform UI behavior, Docker runtime
and million-URL performance remain separate acceptance work.
