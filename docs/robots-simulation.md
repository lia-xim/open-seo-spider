# Offline robots simulation

The simulator evaluates pasted robots.txt text without requesting any supplied URL or sitemap. It calls the same `RobotsPolicy.allowed` and installed `robotstxt` Google DefaultMatcher as the crawler. It does not start a crawl or change saved crawl settings, published robots rules or the original application's profile.

The standalone interface is `/robots`. Its local API contract is `POST /api/robots/simulate`; `POST /api/robots/export.csv` accepts the same input and returns a spreadsheet-safe CSV. Requests are JSON:

```json
{
  "robots_text": "User-agent: *\nDisallow: /private\nAllow: /private/public\n",
  "user_agent": "OpenSEOSpider",
  "urls": [
    "https://example.test/private",
    "https://example.test/private/public",
    "https://example.test/private",
    "not a URL"
  ]
}
```

`user_agent` is the matcher input used for crawler access, independent of the meta/X-Robots-Tag indexability agent. HTTP user-agent strings can be passed unchanged; UI presets provide ordinary crawler tokens. Empty, non-ASCII, control-containing or greater-than-256-byte agents are rejected before evaluation. The request rejects unknown JSON fields rather than interpreting them as fetch instructions.

URL rows retain their original order, duplicates and exact submitted values. Each result has `source_row`, `input_url`, `normalized_url`, `path_query`, `fragment`, nullable `allowed`, and nullable `error`. Invalid, non-HTTP, raw-control-containing and userinfo URLs have `allowed: null` and an explicit row error; they are not silently removed or reported as blocked. One bad URL does not discard other valid decisions.

Normalization uses the existing crawler URL function: URL syntax/host normalization and relative dot-segment processing, without sorting query parameters or changing path case. Only absolute HTTP/HTTPS URLs with a host are accepted. The original fragment remains in input evidence, with its parsed encoded form in `fragment`; it is excluded from the normalized crawl URL and matcher path. The actual matcher path/query is returned, rather than reconstructed as an explanation of which rule won. Existing escapes and duplicate query parameters remain observational evidence.

The top-level result identifies the engine, exact UTF-8 rule-byte count and SHA256 of the complete submitted rule text. It also contains the agent, ordered rows, `website_requests: 0`, `ignored_directives`, and explicit limitations. This hash includes comments and line endings; it is not a semantic equivalence hash. No winning-rule or line-number trace is manufactured because the shared matcher does not expose a verified one.

## Input bounds

An optional `limits` object can lower any of the following defaults. Each bound must be a positive integer no greater than its listed ceiling. Oversized inputs are rejected as a whole before matching; the simulator never silently truncates rules or rows.

| Limit | Default and ceiling | Purpose |
| --- | ---: | --- |
| `max_robots_bytes` | 524,288 | Full pasted UTF-8 rule bytes; aligns with the crawler's 512 KiB capture ceiling |
| `max_urls` | 10,000 | URL rows, including duplicate and invalid rows |
| `max_url_bytes` | 8,192 | Each original UTF-8 URL field |
| `max_input_bytes` | 8,388,608 | Sum of rule text, agent and URL field bytes |
| `max_evaluation_bytes` | 67,108,864 | Rule bytes multiplied by row count, with an empty rule text counted as one byte for this work bound |

The work bound is necessary because the installed matcher parses rules for each row. Large rule files can therefore require smaller batches than the maximum row count. A valid list has at least one row; an empty rule text is valid and permits every valid URL. Technical resource bounds are independent of licenses and impose no commercial URL quota on crawling.

The browser checks the same numeric, UTF-8-byte and work bounds before submitting. One URL per line preserves interior blank rows as unknown input, while a single final line terminator does not create an extra row. Input changes invalidate displayed results and prevent late simulation or CSV responses from restoring an older result or downloading stale data.

## Rule semantics and boundaries

Tests cover agent-specific and wildcard groups, merging same-agent groups, case-sensitive paths, longest-rule precedence, equal-length Allow precedence, `*`, `$`, query matching, Unicode paths, percent-encoded reserved paths, comments and empty rules. These cases are guided by [RFC 9309 section 2.2](https://www.rfc-editor.org/rfc/rfc9309.html#section-2.2). The default 512 KiB bound meets the RFC's minimum parser capacity; an operator lowering this offline input bound receives rejection rather than partial-rule decisions.

The installed parser classifies standard-key aliases and unknown fields. `ignored_directives` inventories nondecision fields the parser actually emits; it is not a complete syntax validator. `Crawl-delay`, `Request-rate`, `Host`, `Noindex` and `Sitemap` do not change this access decision. A sitemap is not fetched, and these fields do not terminate the access-rule group merely because they are listed as ignored. Google also distinguishes supported access rules from unsupported fields in its [robots.txt specification](https://developers.google.com/search/docs/crawling-indexing/robots/robots_txt).

The matcher expects escaped URLs and performs no additional URL percent-decoding. The simulator intentionally uses the crawler's current normalization rather than a separate simulator-only path algorithm. Complete RFC equivalence for encoded-unreserved octets, alternative encodings and every tolerant syntax is **NOT PROVEN**. This is the current shared-engine behavior, not a claim of complete RFC conformance or reference-product parity.

HTTP robots retrieval status, redirects, cache age, network failures, crawl scope, site authentication, meta robots and X-Robots-Tag indexability are outside this simulation. Pasted rules apply to every valid input host; their actual publication or origin ownership is not checked. Robots access is not an authorization or security decision, as the [RFC security considerations](https://www.rfc-editor.org/rfc/rfc9309.html#section-3) explain. The simulator does not change these conditions or access a protected site.

CSV uses shared formula escaping for textual fields and leaves unknown decisions blank. JSON export retains the exact input rules/rows plus their result, allowing unknown to remain `null`. UI output uses text nodes for input URLs, errors, engine facts and ignored field names. Supplied URLs are not turned into navigation links.

## Verification

Seven focused Rust tests in `crates/core/src/robots_simulation.rs` passed in the serialized core library gate. Seven focused script regressions passed with the actual `web/robots.js` and deferred local API responses:

```text
cargo test -p open-seo-spider-core robots_simulation::tests
node --test web/verification/robots-input-state.test.cjs
```

The script tests prove input bounds, exact ordered rows, unknown result handling, safe text rendering, agent selection and stale-response/export protection. Their fake DOM does not establish browser layout or native matcher integration.

The actual browser-to-Rust gate is `node web/verification/ui-robots-live.cjs`, using `SPIDER_URL` when a different local app port is required. On 2026-10-12T09:46:36.894Z it passed all eight checks against the fresh app at `http://127.0.0.1:18789`: agent selection; wildcard/end-anchor/query and longest-Allow decisions; duplicate, blank and malformed URL rows; native formula-safe CSV and exact JSON; client/server input bounds; a real deferred-response race; and desktop/mobile layout at 1440, 390 and 320 pixels. Its owned counted website and supplied sitemap received **zero requests**, with no JavaScript runtime errors. The measured result is retained in `web/verification/ui-robots-live-results.json`, with exact exported JSON/CSV beside it. This proves the bounded local workflow; complete RFC equivalence and original application parity remain outside that gate.

Public exports are `RobotsSimulationInput`, `RobotsSimulationLimits`, `RobotsSimulationRow`, `RobotsSimulation`, `ROBOTS_SIMULATION_ENGINE`, `simulate_robots`, and `export_robots_simulation_csv`. The pure functions contain no HTTP client or asynchronous network work.
