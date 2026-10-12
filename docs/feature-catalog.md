# Open SEO Spider capability catalog

Catalog prepared: 11 October 2026; implementation/evidence review: 12 October 2026. Reference: public Screaming Frog SEO Spider documentation, whose download panels advertised version **24.3** at review time. The vendor describes more than 300 SEO issue checks; this catalog groups the documented product areas into **144 implementation requirements**, rather than claiming to enumerate every individual issue filter. [Product overview](https://www.screamingfrog.co.uk/seo-spider/) · [Official guide](https://www.screamingfrog.co.uk/seo-spider/user-guide/).

This is an independent product specification. The initial grouped catalog was prepared from public documentation. Subsequent private static inspection and ordinary original CLI runs inform the granular contracts and comparisons. No vendor executable, implementation code, font, artwork, credential or license mechanism is copied into this repository. Public feature behavior provides a reference; the acceptance tests below are newly authored tests for Open SEO Spider. The user prefers Rust and local operation. Docker, WSL and native packaging are separate delivery choices.

**Independent implementation status is tracked below. Full parity is NOT PROVEN.** The machine-readable [feature-parity.json](feature-parity.json) is the tracking baseline. Implementation and passing tests must be linked before a feature changes status. A passing Open SEO Spider fixture establishes its behavior; a separately authorized, reproducible reference comparison is needed for a parity claim.

“No commercial URL ceiling” does not remove machine resource limits, site crawl policy, or third-party API quotas and fees. Provider integrations need independent accounts and authorization. Open SEO Spider must not reuse Screaming Frog credentials or proprietary integration identity.

## Delivery sequence

| Phase | Scope | Feature count |
| --- | --- | ---: |
| P1 | HTTP crawler foundation and essential extraction | 45 |
| P2 | Persistence, graph analysis, relationships, sitemaps and analyst interface | 45 |
| P3 | Browser rendering, extraction and advanced validation | 24 |
| P4 | Authorized providers, optional AI, automation and MCP | 30 |

The phases are delivery areas, not claims of completed milestones. Current evidence proves native/API/browser slices; each broader acceptance still has its own status. Large-scale, production readiness and exact issue-rule parity require separate evidence.

## Publicly documented building blocks and recent scope

The rendering documentation describes Chromium; the accessibility tutorial describes the open-source axe engine. These are public integration facts, not a reconstruction of the vendor's complete architecture. [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) · [Accessibility auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-perform-a-web-accessibility-audit/).

Version 24 added MCP, automatic crawl comparison, richer email change summaries, export attachments, nonstandard link evidence and local usage statistics. Current guide sections also include direct AI integrations, embeddings, semantic search, HTML validation and APIs-only operation. These areas make “all functionality” considerably broader than an HTTP link scanner. [Version 24 release](https://www.screamingfrog.co.uk/blog/seo-spider-24/) · [AI prompts](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-crawl-with-ai-prompts/) · [Semantic analysis](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-identify-semantically-similar-pages-outliers/).

## Capability requirements

The source link in each row supports the reference product area. Acceptance criteria are independent Open SEO Spider requirements and do not imply reference execution. Keys are stable and match the JSON manifest. `implemented` means the stated independent acceptance is supported; `partial` has a verified slice with outstanding acceptance; `planned` has no sufficient tracked implementation slice. Full feature and workflow parity remains `NOT_PROVEN`; bounded reference observations are reported separately. Exact evidence and remaining acceptance are recorded per key in the JSON manifest.

### Crawl modes and control (P1)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `modes.spider` | `implemented` | Website spider | Crawl a synthetic linked graph; fetch each normalized URL once and preserve every link occurrence. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `modes.list` | `implemented` | List import | Import TXT, CSV, XLSX and XML; keep original row order and duplicate inputs in exports while deduplicating network work. | [List mode](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-list-mode/) |
| `modes.serp` | `implemented` | Offline SERP editing | Import URL/title/description rows, calculate length and approximate width, and export without making network requests. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `modes.apis` | `planned` | API-only enrichment | Enrich supplied URLs using a mock API; record zero website requests. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `modes.compare` | `partial` | Snapshot comparison | Compare two saved fixture snapshots and classify new, missing and changed URLs. | [Crawl comparison](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-compare-crawls/) |
| `modes.control` | `implemented` | Pause, stop, resume | Pause with requests in flight, persist frontier, restart, and complete with no missing or duplicated results. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `modes.recrawl` | `implemented` | Selective re-crawl | Re-fetch a chosen failed URL after the fixture changes without clearing unrelated saved results. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |

### Discovery, scope and URL policy (P1)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `crawl.unlimited` | `implemented` | No commercial URL ceiling | Complete a deterministic 501-URL fixture; configurable operator limits remain independent of licenses. | [SEO Spider overview](https://www.screamingfrog.co.uk/seo-spider/) |
| `crawl.scope` | `partial` | Host, subdomain and folder scope | Verify exact host, all-subdomains and start-folder policies against an HTTP fixture with cross-scope links. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.resources` | `partial` | Per-type discovery and storage | Independently store and fetch HTML, CSS, JavaScript, images, media, PDF and legacy SWF references. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.relationships` | `partial` | Relationship discovery | Enable discovery separately for canonicals, hreflang, next/prev, AMP, iframe, mobile alternate and meta refresh. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.normalization` | `partial` | URL identity and encoding | Cover relative URLs, base href, Unicode host/path, percent encoding, fragments and duplicate query strings without merging distinct resources. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `crawl.rewrite` | `implemented` | URL rewriting rules | Apply saved query removal, regex replacement and rewrite rules; keep original and requested addresses for evidence. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.include_exclude` | `partial` | Include/exclude expressions | Exercise allow and deny regexes against links and resources with an explained exclusion reason. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.cdns` | `planned` | CDN classification | Classify configured CDN hosts as internal while preventing unrestricted external traversal. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.nofollow` | `partial` | Link directive policy | Exercise nofollow, ugc, sponsored and page nofollow; store links regardless of whether discovery is enabled. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.limits` | `partial` | Operator crawl controls | Test total, depth, URLs-per-depth, folder, query, subdomain, redirect, URL length, link-count and response-size bounds. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.speed` | `partial` | Concurrency and request speed | Use a timestamped fixture to prove request-rate bounds and bounded concurrency under latency. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.timeouts_retries` | `partial` | Timeout and retry policy | Distinguish timeout, DNS/network and HTTP errors; retry selected 5xx responses with bounded attempts. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.headers_proxy` | `partial` | Agents, headers and proxies | Verify custom user agent/header and proxy behavior using captured fixture requests; redact secrets from export. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.tls` | `planned` | Trusted certificates and HSTS | Crawl a fixture with a user-trusted certificate and observe HSTS upgrade provenance without disabling validation globally. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |

### Robots and indexability (P1)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `robots.fetch` | `partial` | robots.txt retrieval and caching | Cache per origin and test distinct policy on scheme/host/port; record blocked URLs without fabricated status codes. | [Robots testing](https://www.screamingfrog.co.uk/seo-spider/tutorials/robots-txt-tester/) |
| `robots.matching` | `partial` | Agent matching and directives | Run fixtures for multiple user-agent groups, allow/disallow wildcards, end anchors and sitemap declarations. | [Robots testing](https://www.screamingfrog.co.uk/seo-spider/tutorials/robots-txt-tester/) |
| `robots.test` | `implemented` | Custom policy simulation | Evaluate supplied rules against a URL list without editing the remote site's robots.txt. | [Robots testing](https://www.screamingfrog.co.uk/seo-spider/tutorials/robots-txt-tester/) |
| `robots.override` | `partial` | Explicit robots override | Keep respect as the default; require a saved explicit override and show its state in crawl metadata. | [Robots testing](https://www.screamingfrog.co.uk/seo-spider/tutorials/robots-txt-tester/) |
| `robots.meta` | `implemented` | Meta and HTTP robots | Extract multiple meta robots and X-Robots-Tag fields; distinguish general and agent-specific directives. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `robots.indexability` | `partial` | Indexability explanations | Report concrete reasons for redirect, robots blocking, noindex, non-HTML and canonicalization with supporting evidence. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `robots.traversal` | `planned` | Directive-dependent discovery | Verify saved noindex/canonical/pagination policies influence expansion without suppressing observed evidence. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |

### Transport, status and redirects (P1)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `http.response` | `partial` | Response evidence | Record status, final content type, measured bytes, transferred bytes, timings and complete header evidence. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `http.failures` | `partial` | Status and network failure groups | Seed 2xx, 3xx, 4xx, 5xx, timeout and malformed links; expose distinct machine-readable results. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `http.redirects` | `partial` | Redirect chains and loops | Persist every hop including terminal URL; find chains, circular loops, off-scope destinations and non-200 endings. | [Redirect auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/audit-redirects/) |
| `http.refresh` | `partial` | Refresh and script redirects | Detect HTTP refresh, meta refresh and rendered JavaScript navigation; label each mechanism separately. | [Redirect auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/audit-redirects/) |
| `http.migration` | `planned` | Migration destination validation | Check a CSV of old/new URL expectations and identify mismatched final destinations. | [Redirect auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/audit-redirects/) |
| `http.broken_sources` | `partial` | Broken-link source export | Export every source occurrence for a broken destination with anchor, rel and location. | [Link auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/) |

### Page metadata and content (P1)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `metadata.titles` | `partial` | Title extraction and audits | Test absent, empty, multiple, duplicate and configurable character/width thresholds with a known HTML fixture. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `metadata.descriptions` | `partial` | Description extraction and audits | Test absent, duplicate, multiple and length/width limits without assuming a fixed search-engine display width. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `metadata.headings` | `implemented` | H1/H2 and meta keyword evidence | Record heading order and duplicate/missing/multiple conditions plus meta keyword values as observational fields. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `metadata.serp_preview` | `implemented` | SERP preview | Show editable title/description preview; identify it as an approximation and export edited values. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `metadata.area` | `planned` | Configurable content area | Exclude shared navigation/footer and include a chosen selector; prove selected text feeds analysis. | [Duplicate content](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-check-for-duplicate-content/) |
| `metadata.wordcount` | `partial` | Text count and ratio | Use fixture paragraphs and hidden/script content to prove documented tokenization and text/code calculations. | [Readability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-test-readability/) |
| `metadata.readability` | `partial` | Readability scoring | Return score and formula/language version; classify unsupported language explicitly. | [Readability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-test-readability/) |
| `metadata.exact` | `partial` | Exact duplicate detection | Group identical content but separate a one-character change; record the digest method. | [Duplicate content](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-check-for-duplicate-content/) |
| `metadata.near` | `partial` | Near duplicate detection | Score a controlled changed paragraph above/below a configured threshold; preserve matched URL and comparison evidence. | [Duplicate content](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-check-for-duplicate-content/) |
| `metadata.spelling` | `planned` | Spelling and grammar | Test multilingual dictionaries, ignored words/rules and exact issue locations against saved text. | [Spelling and grammar](https://www.screamingfrog.co.uk/seo-spider/tutorials/spelling-grammar-checker/) |
| `metadata.ngrams` | `partial` | N-gram frequency | Count overlapping one-, two- and three-word sequences with documented stopword and case policies. | [N-grams](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-n-grams/) |

### Link graph and architecture (P2)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `links.occurrences` | `implemented` | Inlinks and outlinks | For repeated A-to-B anchors, expose total occurrences and unique sources separately with anchor text and rel. | [Link auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/) |
| `links.positions` | `implemented` | Link position and path | Classify nav/header/footer/content links and custom positions; retain selector/path per occurrence. | [Link auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/) |
| `links.depth` | `partial` | Shortest crawl depth | On a graph with multiple routes, compute minimum click depth deterministically after complete discovery. | [Link auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/) |
| `links.score` | `partial` | Internal authority scoring | Run a documented independent PageRank-style method on a known graph; save algorithm and parameter version. | [Link auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/) |
| `links.orphans` | `partial` | Orphan URL discovery | Merge sitemap/analytics/search data; distinguish absent internal links from incomplete crawl coverage. | [Orphan auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/find-orphan-pages/) |
| `links.uncrawlable` | `partial` | Nonstandard link evidence | Record onclick, javascript URLs and href on non-anchor elements without assuming a valid navigable target. | [Version 24 release (19 May 2026)](https://www.screamingfrog.co.uk/blog/seo-spider-24/) |
| `links.fragments` | `implemented` | Bookmark validation | Verify decoded fragment destinations against IDs/names; distinguish a missing fragment from a failed page. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `links.crawl_graph` | `partial` | Crawl tree and graph views | Render shortest-path hierarchy, focus/expand and status/indexability coloring from fixture data. | [Architecture visualization](https://www.screamingfrog.co.uk/seo-spider/tutorials/site-architecture-crawl-visualisations/) |
| `links.directory_graph` | `planned` | Directory architecture views | Build directory hierarchy including synthetic grouping paths and label paths that are not fetched URLs. | [Architecture visualization](https://www.screamingfrog.co.uk/seo-spider/tutorials/site-architecture-crawl-visualisations/) |
| `links.graphs_3d` | `planned` | 3D graph views | Display a bounded large graph and expose truncation/focus rather than silently omitting nodes. | [Architecture visualization](https://www.screamingfrog.co.uk/seo-spider/tutorials/site-architecture-crawl-visualisations/) |
| `links.segments` | `planned` | Configurable segmentation | Apply ordered field/regex segments and filter counts, issues, charts and exports by segment. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |

### Canonical, multilingual and alternative pages (P2)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `relationships.canonical` | `implemented` | Canonical extraction | Extract HTML and HTTP Link canonical entries including relative, multiple and self-referencing values. | [Canonical auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-canonicals/) |
| `relationships.canonical_checks` | `partial` | Canonical validation | Flag malformed, conflicting, redirecting, blocked and non-indexable targets and canonical chains. | [Canonical auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-canonicals/) |
| `relationships.pagination` | `partial` | Pagination relationship validation | Extract HTML/HTTP next and prev; test broken links, absent returns and inconsistent chains. | [Pagination auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-pagination/) |
| `relationships.hreflang` | `partial` | Hreflang extraction | Read HTML, HTTP Link and XML sitemap alternates with language/region and x-default. | [Hreflang auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-hreflang/) |
| `relationships.hreflang_checks` | `partial` | Hreflang reciprocity and validity | Test unsupported codes, missing returns/self-reference, duplicates and non-indexable targets. | [Hreflang auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-hreflang/) |
| `relationships.amp` | `partial` | AMP extraction and validation | Detect amphtml/canonical reciprocity and report versioned AMP validation findings. | [AMP auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-validate-amp/) |
| `relationships.mobile` | `partial` | Mobile alternate relationships | Resolve alternate media relationships and surface missing or non-indexable target evidence. | [Mobile usability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-mobile-usability/) |

### Images and resource evidence (P2)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `images.sources` | `partial` | Image candidates and occurrences | Record src/srcset candidates and every source page independently; preserve per-occurrence alt. | [Image auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-find-missing-image-alt-text/) |
| `images.alt` | `partial` | Missing alt distinctions | Separate missing attribute from empty/whitespace alt using three deliberately different image elements. | [Image auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-find-missing-image-alt-text/) |
| `images.size` | `partial` | Image dimensions and size audits | Record intrinsic/rendered dimensions, transfer size and configurable thresholds without mislabeling unavailable data. | [Image auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-find-missing-image-alt-text/) |
| `images.cookies` | `implemented` | Cookie audit | Collect response and script cookies with domain/path/security attributes and distinguish session contexts. | [Cookies](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-perform-a-cookie-audit/) |
| `images.pdf` | `partial` | PDF metadata and text | Extract title, author, dates, pages and text from an owned fixture PDF; show unsupported encrypted files explicitly. | [PDF auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-pdfs/) |
| `images.pdf_links` | `partial` | PDF links and anchor evidence | Resolve link annotations, record source document/page and extract text where reliable. | [PDF auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-pdfs/) |

### Browser rendering and custom scripts (P3)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `rendering.text` | `implemented` | Raw HTML mode | Fetch and parse a script-dependent fixture while proving no JavaScript execution. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.chromium` | `implemented` | Chromium rendering mode | Discover links created after script execution; save browser/version/config used for the crawl. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.wait` | `partial` | Render completion policy | Control viewport, wait/timeout and resource loading; distinguish timed-out render from a successful raw fetch. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.source_diff` | `partial` | Raw/rendered parity analysis | Surface elements and links added/removed by rendering while retaining both representations. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.screenshot` | `partial` | Rendered screenshots | Capture a deterministic owned page at selected viewport and link the image to URL/snapshot. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.console` | `partial` | Console and blocked-resource evidence | Save browser JS errors and failed/robots-blocked resource loads with URLs and error types. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.flatten` | `planned` | Shadow DOM and iframe flattening | Extract owned fixtures with shadow roots and same-origin iframe content; preserve source context. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.archive` | `planned` | Website archiving | Save configured page/resource artifacts and verify a manifest can locate each stored file. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `rendering.custom_extract` | `implemented` | CSS/XPath/regex extraction | Return text, attributes, HTML, inner HTML and function values from a known fixture with multi-value results. | [Custom extraction](https://www.screamingfrog.co.uk/seo-spider/tutorials/web-scraping/) |
| `rendering.visual_extract` | `planned` | Visual selector authoring | Choose an element, preview raw/rendered extraction, save selector and verify it on a second fixture. | [Custom extraction](https://www.screamingfrog.co.uk/seo-spider/tutorials/web-scraping/) |
| `rendering.custom_search` | `planned` | Literal and regex search | Search source, rendered content and selected text with occurrence counts and match evidence. | [Custom search](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-custom-search/) |
| `rendering.javascript` | `implemented` | Custom JavaScript extraction | Run a user-authored script against a fixture with timeout/error handling and typed returned columns. | [JavaScript debugging](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-debug-custom-javascript-snippets/) |
| `rendering.interact` | `planned` | Scripted interactions | Execute authored hover/click/scroll actions in declared order and discover resulting links. | [User interaction scripts](https://www.screamingfrog.co.uk/seo-spider/tutorials/simulating-user-interactions-with-custom-javascript/) |
| `rendering.snippet_library` | `planned` | User snippet library | Import/export user-owned snippets without storing provider secrets in shared library files. | [JavaScript debugging](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-debug-custom-javascript-snippets/) |

### Structured data, usability and validation (P3)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `quality.structured_formats` | `partial` | JSON-LD, Microdata and RDFa | Extract nested graphs from each format; keep parse errors and raw node references. | [Structured data](https://www.screamingfrog.co.uk/seo-spider/tutorials/structured-data-testing-validation/) |
| `quality.schema` | `planned` | Schema vocabulary validation | Check types/properties against an identified schema version; test unknown and deprecated terms. | [Structured data](https://www.screamingfrog.co.uk/seo-spider/tutorials/structured-data-testing-validation/) |
| `quality.richresults` | `planned` | Rich result requirements | Validate required/recommended fields using versioned public rules; distinguish errors from warnings. | [Structured data](https://www.screamingfrog.co.uk/seo-spider/tutorials/structured-data-testing-validation/) |
| `quality.accessibility` | `implemented` | axe accessibility findings | Run version-pinned axe on a deliberately failing rendered fixture; retain rule, impact, node and WCAG tags. | [Accessibility](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-perform-a-web-accessibility-audit/) |
| `quality.accessibility_limit` | `partial` | Manual accessibility boundaries | Display automated-only coverage and incomplete/needs-review findings in report exports. | [Accessibility](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-perform-a-web-accessibility-audit/) |
| `quality.mobile` | `partial` | Mobile usability checks | Test viewport, sizing, tap targets and incompatible content under defined mobile rendering. | [Mobile usability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-mobile-usability/) |
| `quality.html_head` | `planned` | Invalid head elements | Seed invalid head children that affect subsequent metadata; point to observed raw markup. | [HTML head validation](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-debug-invalid-html-elements-in-the-head/) |
| `quality.html_validation` | `planned` | HTML conformance checks | Run a versioned validator against owned malformed HTML and retain line/column/rule where available. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `quality.security` | `partial` | Transport and header audits | Test mixed content, insecure forms, MIME mismatch and configurable security-header presence with concrete evidence. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `quality.carbon` | `planned` | Green hosting and carbon estimate | Expose dataset/method/version and unknown states; label output as an estimate. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |

### Sitemaps (P2)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `sitemap.discovery` | `partial` | Robots and explicit sitemaps | Load a sitemap index, gzipped child sitemap and explicit input; handle loops/malformed XML safely. | [Sitemap auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-xml-sitemaps/) |
| `sitemap.coverage` | `partial` | Sitemap coverage and orphan checks | Compare fetched pages and sitemap locations; distinguish not listed, sitemap-only and incomplete crawl. | [Sitemap auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-xml-sitemaps/) |
| `sitemap.validity` | `partial` | Sitemap URL validation | Find redirect/non-200/non-indexable URLs and invalid lastmod values in fixture XML. | [Sitemap auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-xml-sitemaps/) |
| `sitemap.generate` | `partial` | XML sitemap generation | Export valid XML with optional lastmod, image entries and hreflang; parse output with an independent parser. | [Sitemap generation](https://www.screamingfrog.co.uk/seo-spider/tutorials/xml-sitemap-generator/) |
| `sitemap.split` | `planned` | Split and segmented sitemaps | Split at configured URL/byte bounds, create index and prove segment URL membership. | [Sitemap generation](https://www.screamingfrog.co.uk/seo-spider/tutorials/xml-sitemap-generator/) |

### Authorized API integrations (P4)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `integrations.ga4` | `planned` | Google Analytics 4 | Using an authorized property, join page metrics with declared URL matching and preserve absent data as absent. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `integrations.gsc` | `planned` | Search Console analytics | Fetch selected dates/dimensions and join clicks/impressions/CTR/position; report API errors and row coverage. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `integrations.inspection` | `planned` | Search Console URL inspection | Honor daily/property quotas; retain inspection timestamp and distinguish inspected from queued URLs. | [URL Inspection API](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-automate-the-url-inspection-api/) |
| `integrations.psi` | `planned` | PageSpeed Insights and Lighthouse | Preserve mobile/desktop, field/lab distinction, metric units and provider error details. | [Core Web Vitals](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-core-web-vitals/) |
| `integrations.cwv` | `planned` | Core Web Vitals aggregation | Compute pass/fail from available field metrics; never substitute lab results for unavailable field data. | [Core Web Vitals](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-core-web-vitals/) |
| `integrations.majestic` | `planned` | Majestic metrics | Use separately authorized credentials; mock fresh/historic datasets, quota exhaustion and URL/domain scopes. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `integrations.ahrefs` | `planned` | Ahrefs metrics | Use independent authorized app/API access; mock expiry, quota and metric joins without reusing vendor tokens. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `integrations.moz` | `planned` | Moz metrics | Mock authorized URL/domain metric queries with unavailable/quota states. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `integrations.oauth` | `planned` | Account connection management | Add, disconnect and revoke an authorized provider account; verify tokens stay out of logs and version control. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `integrations.provider_urls` | `planned` | API-discovered crawl candidates | Optionally enqueue provider-only URLs with provenance and scope checking; preserve orphan evidence. | [Orphan auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/find-orphan-pages/) |

### Optional AI and semantic analysis (P4)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `ai.providers` | `planned` | Cloud and local model adapters | Use mock OpenAI-compatible/Gemini/Anthropic and a local Ollama endpoint; record model/provider errors separately. | [AI prompts](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-crawl-with-ai-prompts/) |
| `ai.prompts` | `planned` | Custom prompts and test preview | Run an authored prompt on selected page text/HTML/extraction; preview exact input and persist typed output. | [AI prompts](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-crawl-with-ai-prompts/) |
| `ai.prompt_library` | `planned` | Prompt import/export | Round-trip user-authored prompts with model/settings and no secrets. | [AI prompts](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-crawl-with-ai-prompts/) |
| `ai.embeddings` | `planned` | Embedding generation | Embed selected text, cache with model/input hash and reject incompatible dimensionality. | [Semantic similarity](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-identify-semantically-similar-pages-outliers/) |
| `ai.similarity` | `planned` | Semantic similarity | Rank a known vector fixture using a documented cosine method and configured thresholds. | [Semantic similarity](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-identify-semantically-similar-pages-outliers/) |
| `ai.outliers` | `planned` | Semantic relevance and outliers | Report low-centroid-similarity fixture vectors with exact scores and declared corpus coverage. | [Semantic similarity](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-identify-semantically-similar-pages-outliers/) |
| `ai.semantic_search` | `planned` | Semantic search and clusters | Query saved vectors and visualize clusters while retaining model/dimensionality provenance. | [Semantic similarity](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-identify-semantically-similar-pages-outliers/) |
| `ai.redirect_mapping` | `planned` | Semantic redirect suggestions | Rank candidate target pages from a deterministic vector fixture and export suggestions with score; never apply redirects automatically. | [Semantic redirect mapping](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-vector-embeddings-for-redirect-mapping/) |
| `ai.usage` | `planned` | Provider cost and rate controls | Enforce explicit rate/spend bounds for an API fixture, record retries and distinguish local from paid providers. | [AI prompts](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-crawl-with-ai-prompts/) |

### Persistence, projects and portability (P2)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `storage.database` | `implemented` | Durable local database | Crash after committed requests; recover completed results and pending queue with consistent graph state. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `storage.memory` | `partial` | Optional memory storage | Run the same fixture in memory and disk modes; compare results while documenting mode differences. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `storage.snapshots` | `partial` | Save/open/import/export snapshots | Round-trip an Open SEO Spider snapshot including config, graph, issues and optional artifacts; verify schema version. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `storage.projects` | `partial` | Projects and crawl history | Group snapshots by project, label them, reopen them and record their creation/config identity. | [Team workflows](https://www.screamingfrog.co.uk/seo-spider/tutorials/seo-spider-working-in-teams/) |
| `storage.retention` | `planned` | Retention and locking | Use test timestamps to expire unlocked fixture snapshots while preserving locked snapshots and active jobs. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `storage.config` | `partial` | Reusable crawl configuration | Export/import typed config and validate missing credentials/paths without silently changing settings. | [Team workflows](https://www.screamingfrog.co.uk/seo-spider/tutorials/seo-spider-working-in-teams/) |
| `storage.portable` | `partial` | Shared crawl portability | Copy an Open SEO Spider archive to a second local directory and open all declared portable artifacts. | [Team workflows](https://www.screamingfrog.co.uk/seo-spider/tutorials/seo-spider-working-in-teams/) |
| `storage.compare_changes` | `partial` | Field and content changes | Detect title, description, headings, graph, directives, structured data and content changes even if issue counts are unchanged. | [Crawl comparison](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-compare-crawls/) |
| `storage.compare_mapping` | `planned` | Cross-host comparison mapping | Map staging/production or migrated URL prefixes before joining snapshots and expose collisions. | [Crawl comparison](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-compare-crawls/) |
| `storage.compare_export` | `partial` | Comparison export | Export both snapshots and deltas with new/missing/changed evidence and stable URL keys. | [Crawl comparison](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-compare-crawls/) |

### Automation, exports and agent access (P4)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `automation.tables` | `partial` | Table and bulk exports | Export filtered observations and all-occurrence evidence as CSV/XLSX; escape formulas and preserve Unicode/newlines. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `automation.reports` | `partial` | Issue and specialist reports | Export versioned overview plus redirect/canonical/hreflang/sitemap/security/content report fixtures. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `automation.assets` | `partial` | Artifact exports | Export source, rendered HTML, screenshots, headers, cookies and PDF artifacts with a manifest. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `automation.looker` | `planned` | Google Sheets/Looker output | Use an authorized export destination; retain append/replace setting and stable column schema across two runs. | [Automated report exports](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-automate-crawl-reports-in-data-studio/) |
| `automation.cli` | `implemented` | Headless command line | Run a crawl/config/import/analysis/export sequence from CLI and verify deterministic exit codes. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `automation.scheduler` | `planned` | Recurring scheduling | Use a fake clock to execute saved crawl tasks, record history/errors and define overlap policy. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `automation.auto_compare` | `planned` | Automatic last-two comparison | After two completed project snapshots, create a comparison automatically using saved comparison settings. | [Version 24 release (19 May 2026)](https://www.screamingfrog.co.uk/blog/seo-spider-24/) |
| `automation.notifications` | `planned` | Completion and change notifications | Send only to an explicitly configured local test SMTP sink; verify summary, change table and attachment error handling. | [Notifications](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-set-up-crawl-email-notifications-export-attachments/) |
| `automation.mcp` | `implemented` | MCP agent interface | Perform MCP initialize/list-tools/crawl/read/export using a local client; keep script execution an explicit capability. | [Version 24 release (19 May 2026)](https://www.screamingfrog.co.uk/blog/seo-spider-24/) |
| `automation.mcp_transport` | `implemented` | STDIO and streamable HTTP | Exercise both transports on loopback and prove JSON protocol framing and job identifiers remain stable. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `automation.stats` | `partial` | Local usage statistics | Aggregate locally saved crawl counts/duration without sending telemetry by default. | [Version 24 release (19 May 2026)](https://www.screamingfrog.co.uk/blog/seo-spider-24/) |

### Analyst interface (P2)

| Key | Status | Capability | Independent acceptance test | Source |
| --- | --- | --- | --- | --- |
| `ui.realtime` | `partial` | Live crawl progress and tables | During a delayed fixture crawl, update progress/results and permit safe pause/config inspection. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `ui.filter` | `partial` | Sort, search and saved columns | Combine numeric/text filters and sort; keep column preferences when reopening a project. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `ui.overview` | `partial` | Overview and issue taxonomy | Show severity/type/count and drill-down to every affected URL; distinguish observation from advice. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `ui.details` | `partial` | URL evidence inspector | Select a URL and inspect links, resources, raw/rendered source, headers, cookies and structured nodes. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `ui.charts` | `partial` | Site structure and timing charts | Aggregate folder/depth/status/timing over a fixture dataset with consistent filtered totals. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `ui.localization` | `planned` | Language and UI preferences | Switch supported interface locale/theme/font settings while preserving stored crawl text exactly. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |

## Evidence-based implementation status

Review: 12 October 2026. **25 implemented, 71 partial, 48 planned; 144 grouped requirements.** This is independent acceptance, never a vendor-coverage percentage. The separate [416-filter ledger](parity/granular-acceptance.md) records exact parameters, trigger/control fixtures and edge contracts with all reference parity statuses still NOT_PROVEN.

The preserved [earlier advanced batch](verification/advanced-batch.json) records its original 91 Rust tests and earlier export/browser checks. The preserved [resource and offline batch](verification/resource-offline-batch.json) records **126 default Rust tests** (28 app, 71 core, 23 crawl, four resource fixtures), one explicit real-browser gate, Clippy/build checks, 16 worker tests, 33 UI fixtures, 14 actual renderer-flow checks, 13 actual resource-flow checks, six offline SERP browser checks, and 13 editor-race regressions. The API gate covers all eleven crawl export endpoints, recovery and offline import/export; official MCP SDK tests pass in both transports with 11 tools. The actual offline SERP fixture receives **zero website requests**.

The previous [DOM and robots batch](verification/dom-robots-batch.json) records **163 default Rust tests**, one explicit Chrome gate, 40 UI fixtures, 19 actual DOM browser checks, eight offline robots browser checks, 13 crawl exports and a startup regression preserving completed/imported snapshot bytes and unknown future fields. Seven existing user crawls and eleven original JSON files remain byte-identical after the installed release restart.

The retained [original fixture comparison](parity/reference-comparison.md) records **887 matching cells, 29 differences and three unavailable measurements**, across 35 reference rows and 34 independent document rows. Native response inventories separately retain intermediate redirect observations; they do not manufacture document rows. The [native text-width comparison](parity/native-text-width-comparison.json) matches **58 present title/description measurements** for the fixed public-font 150-percent profile, with two absent-element placeholders excluded. Other display profiles and complex shaping remain outside that proof. [Membership evidence](parity/reference-membership/membership.md) covers 78 selected filters; empty or unconfirmed probes remain open. Matching cells and measurements do not count as complete features or prove all filter boundaries.

The offline SERP editor, meta/HTTP robots extraction and supported HTML extraction modes now meet their independent acceptance. PDF and image analysis remain partial: PDF unsupported fonts, Form XObjects, rendering/OCR and annotation anchor text, plus rendered image dimensions, wire-transfer bytes and configurable dimension/size thresholds, still require proof. Imported provenance and explicit native URL rewriting now have bounded integration proof. Resource-wide/browser policy, full relationship formats, archives, provider/AI adapters, authentication, scheduling and large-scale performance retain explicit remaining acceptance. See the JSON ledger for each contract.

| Phase | Implemented | Partial | Planned | Total |
| --- | ---: | ---: | ---: | ---: |
| P1 | 11 | 27 | 7 | 45 |
| P2 | 6 | 32 | 7 | 45 |
| P3 | 5 | 8 | 11 | 24 |
| P4 | 3 | 4 | 23 | 30 |

## Reference workflow evidence

Small HTTP/graph/robots/relationship comparisons are now recorded with known differences. Full workflow parity, browser parity, provider comparison and production-scale proof remain NOT_PROVEN. Linux container and four-job CI are verified at the exact prior commit in [the CI proof](verification/resource-offline-ci.json); Windows Docker/WSL installation remains unproven.

## Required proof before a complete-parity claim

1. Expand grouped requirements into individual versioned issue rules, columns, export schemas and configuration semantics; attach fixture inputs and expected outputs.
2. Prove native and browser crawls separately. Browser-only checks, authentication, storage recovery, API calls and exports each need end-to-end evidence.
3. Compare only authorized reference operation on owned test fixtures. Save version, configuration, raw observations, differences and analysis. UI wording or visual similarity does not prove behavior.
4. Measure bounded memory/frontier growth, disk usage, correctness under concurrency and recovery after interruption at progressively larger URL counts.
5. Preserve explicit unknown/disabled/error states. Missing GSC/GA4/PSI data is not zero traffic, zero demand or a passing audit.
6. Maintain independent schemas, algorithms and original presentation. No promise is made here to import vendor-specific `.seospider` or `.dbseospider` formats.
7. Keep a tracked issue per capability or coherent subfeature and link code, tests and proof artifacts in the manifest. Do not bulk-mark planned capabilities complete because a basic crawler runs.

## Source inventory

All references below are official primary documentation reviewed online on 11 October 2026. Documentation can change; freeze requirements by project version when implementation starts.

- `overview`: [SEO Spider overview](https://www.screamingfrog.co.uk/seo-spider/)
- `guide`: [User guide index](https://www.screamingfrog.co.uk/seo-spider/user-guide/)
- `config`: [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/)
- `general`: [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/)
- `tabs`: [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/)
- `release24`: [Version 24 release (19 May 2026)](https://www.screamingfrog.co.uk/blog/seo-spider-24/)
- `list`: [List mode](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-list-mode/)
- `render`: [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/)
- `compare`: [Crawl comparison](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-compare-crawls/)
- `visuals`: [Architecture visualization](https://www.screamingfrog.co.uk/seo-spider/tutorials/site-architecture-crawl-visualisations/)
- `extract`: [Custom extraction](https://www.screamingfrog.co.uk/seo-spider/tutorials/web-scraping/)
- `structured`: [Structured data](https://www.screamingfrog.co.uk/seo-spider/tutorials/structured-data-testing-validation/)
- `accessibility`: [Accessibility](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-perform-a-web-accessibility-audit/)
- `duplicates`: [Duplicate content](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-check-for-duplicate-content/)
- `semantic`: [Semantic similarity](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-identify-semantically-similar-pages-outliers/)
- `ai`: [AI prompts](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-crawl-with-ai-prompts/)
- `images`: [Image auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-find-missing-image-alt-text/)
- `canonical`: [Canonical auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-canonicals/)
- `hreflang`: [Hreflang auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-hreflang/)
- `pagination`: [Pagination auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-pagination/)
- `sitemaps`: [Sitemap auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-xml-sitemaps/)
- `sitemapgen`: [Sitemap generation](https://www.screamingfrog.co.uk/seo-spider/tutorials/xml-sitemap-generator/)
- `spelling`: [Spelling and grammar](https://www.screamingfrog.co.uk/seo-spider/tutorials/spelling-grammar-checker/)
- `cwv`: [Core Web Vitals](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-core-web-vitals/)
- `mobile`: [Mobile usability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-mobile-usability/)
- `ngrams`: [N-grams](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-n-grams/)
- `cookies`: [Cookies](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-perform-a-cookie-audit/)
- `pdf`: [PDF auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-pdfs/)
- `auth`: [Authentication](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawling-password-protected-websites/)
- `robots`: [Robots testing](https://www.screamingfrog.co.uk/seo-spider/tutorials/robots-txt-tester/)
- `notifications`: [Notifications](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-set-up-crawl-email-notifications-export-attachments/)
- `teams`: [Team workflows](https://www.screamingfrog.co.uk/seo-spider/tutorials/seo-spider-working-in-teams/)
- `links`: [Link auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/)
- `inspection`: [URL Inspection API](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-automate-the-url-inspection-api/)
- `readability`: [Readability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-test-readability/)
- `head`: [HTML head validation](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-debug-invalid-html-elements-in-the-head/)
- `redirectmap`: [Semantic redirect mapping](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-vector-embeddings-for-redirect-mapping/)
- `orphans`: [Orphan auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/find-orphan-pages/)
- `redirects`: [Redirect auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/audit-redirects/)
- `amp`: [AMP auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-validate-amp/)
- `interactions`: [User interaction scripts](https://www.screamingfrog.co.uk/seo-spider/tutorials/simulating-user-interactions-with-custom-javascript/)
- `jsdebug`: [JavaScript debugging](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-debug-custom-javascript-snippets/)
- `search`: [Custom search](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-custom-search/)
- `looker`: [Automated report exports](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-automate-crawl-reports-in-data-studio/)


The earlier [list import batch](verification/list-import-batch.json) records **189 default Rust tests plus one explicit Chrome gate**, 17 worker tests, 47 UI fixtures, 12 import-state regressions and 14 actual import browser checks. TXT, CSV, XML and XLSX parsing retains ordered duplicate/error occurrences; import and inspection make zero website requests. An explicit crawl deduplicates actual fetch work and joins original inputs to captured observations in the fourteenth export, `inputs.csv`. The CLI config/import/crawl/analysis/export sequence and success/error exit codes are tested. Seven existing user projects and eleven JSON files are preserved byte-for-byte. The [previous exact-commit CI](verification/dom-robots-ci.json) is green on Windows/Linux/Chromium/Linux Docker; the [list-import exact-commit CI](verification/list-import-ci.json) is also green.


The earlier [crawl control batch](verification/crawl-control-batch.json) records **215 default Rust tests plus two explicit Chrome gates**, 47 baseline UI fixtures, 13 control-race fixtures, 10 actual control browser checks and 12 MCP tools over both SDK transports. Actual held-request pause drains admitted documents, retains page and sitemap discovery frontiers, survives read-only SQLite-only recovery and preserves exact historical bytes. Explicit resume may change only operational limits and advances generation/revision. The independent URL-policy preview executes 47 authored candidates and equal API/CLI decisions without HTTP; at that earlier batch, applying it to live crawls remained pending. The B7 evidence below records subsequent native integration. The [exact-commit control CI](verification/crawl-control-ci.json) is green on Windows/Linux/Chromium/Linux Docker. Full feature/filter/workflow parity remains **NOT_PROVEN**.



<!-- B7-NATIVE-CONTENT-START -->

The [B6 selected-recrawl acceptance](verification/selected-recrawl-e2e.json) proves the existing chosen-failed-URL contract: after the fixture changes, recorded404 becomes a captured200 while four unrelated saved rows and exact parent bytes remain unchanged. Six native fixtures plus a separate actual-Chrome selection gate, 11 API/CLI checks, 16 browser-state checks and nine actual browser checks support immutable parent history, exact requested identities, duplicate occurrences, independent attempt budgets, pending exclusion, freshness and selected-only pause/recovery. [Exact B6 CI](verification/selected-recrawl-ci.json) is green for commit0df95ca/run38255480208; it does not attest later B7 source.

The [B7 native policy API/CLI gate](verification/crawl-policy-e2e.json) records two broad directed workflows and complete506-row occurrence snapshots. Thirteen native HTTP fixtures plus four policy-ledger units prove actual-target robots, original credential origins, raw/fetch alias separation, folder/check-only boundaries, terminal redirect evidence, import-reject provenance, atomic storage/recovery and validated saved permission reuse. The UI has 15 isolated policy checks and eight actual native policy checks. Explicit saved native query removal/rewrite meets its own acceptance; all-subdomain HTTP integration and resource-wide include/exclude coverage remain partial. Renderer+live-policy is explicitly rejected before HTTP. Imported zero-occurrence parent history is unknown/incomplete, not proven empty history.

The [offline content API/CLI gate](verification/content-analysis-e2e.json) and 20 core unit tests prove versioned Unicode/case/stopword 1/2/3-grams, exact bounded shingle-set Jaccard and declared EN/DE heuristic Flesch scores with explicit corpus, missing/incomplete/unsupported states and zero website requests. These three capabilities remain partial because native selected-text capture, source revision/freshness, corpus/UI/export integration is not part of B7. Content-area capture remains planned. [Original content observations](reference-content-analysis.md) cover22 owned HTML cases; near checking was disabled, original token/sentence/readability values remain distinct from the own heuristic, and an empty filter is never a passing comparison.

[Final B7 aggregate proof](verification/native-policy-content-batch.json) records 269 default workspace tests (36 app, 233 core, three ignored), a separate selected-Chrome gate, final focused policy units4/HTTP13 and workspace Clippy/fmt. Its final debug binary has eight actual policy UI checks; the 95 earlier browser checks bind a separately stated earlier binary. Exact B7 CI, immutable release and installed replacement remain pending; the existing installed B6 runtime is separately verified. Full reference parity still needs separate verification.

All416 filter parity and32 complete-workflow reference claims remain NOT_PROVEN. Related capability statuses in the granular ledger are refreshed context, not automatic filter/workflow acceptance.
<!-- B7-NATIVE-CONTENT-END -->
