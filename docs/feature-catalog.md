# Open SEO Spider capability catalog

Catalog date: 11 October 2026. Reference: public Screaming Frog SEO Spider documentation, whose download panels advertised version **24.3** at review time. The vendor describes more than 300 SEO issue checks; this catalog groups the documented product areas into **144 implementation requirements**, rather than claiming to enumerate every individual issue filter. [Product overview](https://www.screamingfrog.co.uk/seo-spider/) · [Official guide](https://www.screamingfrog.co.uk/seo-spider/user-guide/).

This is an independent product specification. No proprietary executable, code, artwork, credential or license mechanism was inspected or copied to prepare it. Public feature behavior provides a reference; the acceptance tests below are newly authored tests for Open SEO Spider. The user prefers Rust and local operation. Docker, WSL and native packaging are separate delivery choices.

**Every entry is planned. Full parity is NOT PROVEN.** The machine-readable [feature-parity.json](feature-parity.json) is the tracking baseline. Implementation and passing tests must be linked before a feature changes status. A passing Open SEO Spider fixture establishes its behavior; a separately authorized, reproducible reference comparison is needed for a parity claim.

“No commercial URL ceiling” does not remove machine resource limits, site crawl policy, or third-party API quotas and fees. Provider integrations need independent accounts and authorization. Open SEO Spider must not reuse Screaming Frog credentials or proprietary integration identity.

## Delivery sequence

| Phase | Scope | Feature count |
| --- | --- | ---: |
| P1 | HTTP crawler foundation and essential extraction | 45 |
| P2 | Persistence, graph analysis, relationships, sitemaps and analyst interface | 45 |
| P3 | Browser rendering, extraction and advanced validation | 24 |
| P4 | Authorized providers, optional AI, automation and MCP | 30 |

The phases are proposed sequencing, not completed milestones. Large-scale, production readiness and exact issue-rule parity require separate evidence.

## Publicly documented building blocks and recent scope

The rendering documentation describes Chromium; the accessibility tutorial describes the open-source axe engine. These are public integration facts, not a reconstruction of the vendor's complete architecture. [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) · [Accessibility auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-perform-a-web-accessibility-audit/).

Version 24 added MCP, automatic crawl comparison, richer email change summaries, export attachments, nonstandard link evidence and local usage statistics. Current guide sections also include direct AI integrations, embeddings, semantic search, HTML validation and APIs-only operation. These areas make “all functionality” considerably broader than an HTTP link scanner. [Version 24 release](https://www.screamingfrog.co.uk/blog/seo-spider-24/) · [AI prompts](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-crawl-with-ai-prompts/) · [Semantic analysis](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-identify-semantically-similar-pages-outliers/).

## Capability requirements

The source link in each row supports the product area. Acceptance criteria describe the proposed independent implementation and do not imply the reference product was tested against that fixture. Keys are stable and match the JSON manifest. All rows have status `planned`.

### Crawl modes and control (P1)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `modes.spider` | Website spider | Crawl a synthetic linked graph; fetch each normalized URL once and preserve every link occurrence. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `modes.list` | List import | Import TXT, CSV, XLSX and XML; keep original row order and duplicate inputs in exports while deduplicating network work. | [List mode](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-list-mode/) |
| `modes.serp` | Offline SERP editing | Import URL/title/description rows, calculate length and approximate width, and export without making network requests. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `modes.apis` | API-only enrichment | Enrich supplied URLs using a mock API; record zero website requests. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `modes.compare` | Snapshot comparison | Compare two saved fixture snapshots and classify new, missing and changed URLs. | [Crawl comparison](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-compare-crawls/) |
| `modes.control` | Pause, stop, resume | Pause with requests in flight, persist frontier, restart, and complete with no missing or duplicated results. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `modes.recrawl` | Selective re-crawl | Re-fetch a chosen failed URL after the fixture changes without clearing unrelated saved results. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |

### Discovery, scope and URL policy (P1)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `crawl.unlimited` | No commercial URL ceiling | Complete a deterministic 501-URL fixture; configurable operator limits remain independent of licenses. | [SEO Spider overview](https://www.screamingfrog.co.uk/seo-spider/) |
| `crawl.scope` | Host, subdomain and folder scope | Verify exact host, all-subdomains and start-folder policies against an HTTP fixture with cross-scope links. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.resources` | Per-type discovery and storage | Independently store and fetch HTML, CSS, JavaScript, images, media, PDF and legacy SWF references. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.relationships` | Relationship discovery | Enable discovery separately for canonicals, hreflang, next/prev, AMP, iframe, mobile alternate and meta refresh. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.normalization` | URL identity and encoding | Cover relative URLs, base href, Unicode host/path, percent encoding, fragments and duplicate query strings without merging distinct resources. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `crawl.rewrite` | URL rewriting rules | Apply saved query removal, regex replacement and rewrite rules; keep original and requested addresses for evidence. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.include_exclude` | Include/exclude expressions | Exercise allow and deny regexes against links and resources with an explained exclusion reason. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.cdns` | CDN classification | Classify configured CDN hosts as internal while preventing unrestricted external traversal. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.nofollow` | Link directive policy | Exercise nofollow, ugc, sponsored and page nofollow; store links regardless of whether discovery is enabled. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.limits` | Operator crawl controls | Test total, depth, URLs-per-depth, folder, query, subdomain, redirect, URL length, link-count and response-size bounds. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.speed` | Concurrency and request speed | Use a timestamped fixture to prove request-rate bounds and bounded concurrency under latency. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.timeouts_retries` | Timeout and retry policy | Distinguish timeout, DNS/network and HTTP errors; retry selected 5xx responses with bounded attempts. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.headers_proxy` | Agents, headers and proxies | Verify custom user agent/header and proxy behavior using captured fixture requests; redact secrets from export. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `crawl.tls` | Trusted certificates and HSTS | Crawl a fixture with a user-trusted certificate and observe HSTS upgrade provenance without disabling validation globally. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |

### Robots and indexability (P1)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `robots.fetch` | robots.txt retrieval and caching | Cache per origin and test distinct policy on scheme/host/port; record blocked URLs without fabricated status codes. | [Robots testing](https://www.screamingfrog.co.uk/seo-spider/tutorials/robots-txt-tester/) |
| `robots.matching` | Agent matching and directives | Run fixtures for multiple user-agent groups, allow/disallow wildcards, end anchors and sitemap declarations. | [Robots testing](https://www.screamingfrog.co.uk/seo-spider/tutorials/robots-txt-tester/) |
| `robots.test` | Custom policy simulation | Evaluate supplied rules against a URL list without editing the remote site's robots.txt. | [Robots testing](https://www.screamingfrog.co.uk/seo-spider/tutorials/robots-txt-tester/) |
| `robots.override` | Explicit robots override | Keep respect as the default; require a saved explicit override and show its state in crawl metadata. | [Robots testing](https://www.screamingfrog.co.uk/seo-spider/tutorials/robots-txt-tester/) |
| `robots.meta` | Meta and HTTP robots | Extract multiple meta robots and X-Robots-Tag fields; distinguish general and agent-specific directives. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `robots.indexability` | Indexability explanations | Report concrete reasons for redirect, robots blocking, noindex, non-HTML and canonicalization with supporting evidence. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `robots.traversal` | Directive-dependent discovery | Verify saved noindex/canonical/pagination policies influence expansion without suppressing observed evidence. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |

### Transport, status and redirects (P1)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `http.response` | Response evidence | Record status, final content type, measured bytes, transferred bytes, timings and complete header evidence. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `http.failures` | Status and network failure groups | Seed 2xx, 3xx, 4xx, 5xx, timeout and malformed links; expose distinct machine-readable results. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `http.redirects` | Redirect chains and loops | Persist every hop including terminal URL; find chains, circular loops, off-scope destinations and non-200 endings. | [Redirect auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/audit-redirects/) |
| `http.refresh` | Refresh and script redirects | Detect HTTP refresh, meta refresh and rendered JavaScript navigation; label each mechanism separately. | [Redirect auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/audit-redirects/) |
| `http.migration` | Migration destination validation | Check a CSV of old/new URL expectations and identify mismatched final destinations. | [Redirect auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/audit-redirects/) |
| `http.broken_sources` | Broken-link source export | Export every source occurrence for a broken destination with anchor, rel and location. | [Link auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/) |

### Page metadata and content (P1)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `metadata.titles` | Title extraction and audits | Test absent, empty, multiple, duplicate and configurable character/width thresholds with a known HTML fixture. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `metadata.descriptions` | Description extraction and audits | Test absent, duplicate, multiple and length/width limits without assuming a fixed search-engine display width. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `metadata.headings` | H1/H2 and meta keyword evidence | Record heading order and duplicate/missing/multiple conditions plus meta keyword values as observational fields. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `metadata.serp_preview` | SERP preview | Show editable title/description preview; identify it as an approximation and export edited values. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `metadata.area` | Configurable content area | Exclude shared navigation/footer and include a chosen selector; prove selected text feeds analysis. | [Duplicate content](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-check-for-duplicate-content/) |
| `metadata.wordcount` | Text count and ratio | Use fixture paragraphs and hidden/script content to prove documented tokenization and text/code calculations. | [Readability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-test-readability/) |
| `metadata.readability` | Readability scoring | Return score and formula/language version; classify unsupported language explicitly. | [Readability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-test-readability/) |
| `metadata.exact` | Exact duplicate detection | Group identical content but separate a one-character change; record the digest method. | [Duplicate content](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-check-for-duplicate-content/) |
| `metadata.near` | Near duplicate detection | Score a controlled changed paragraph above/below a configured threshold; preserve matched URL and comparison evidence. | [Duplicate content](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-check-for-duplicate-content/) |
| `metadata.spelling` | Spelling and grammar | Test multilingual dictionaries, ignored words/rules and exact issue locations against saved text. | [Spelling and grammar](https://www.screamingfrog.co.uk/seo-spider/tutorials/spelling-grammar-checker/) |
| `metadata.ngrams` | N-gram frequency | Count overlapping one-, two- and three-word sequences with documented stopword and case policies. | [N-grams](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-n-grams/) |

### Link graph and architecture (P2)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `links.occurrences` | Inlinks and outlinks | For repeated A-to-B anchors, expose total occurrences and unique sources separately with anchor text and rel. | [Link auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/) |
| `links.positions` | Link position and path | Classify nav/header/footer/content links and custom positions; retain selector/path per occurrence. | [Link auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/) |
| `links.depth` | Shortest crawl depth | On a graph with multiple routes, compute minimum click depth deterministically after complete discovery. | [Link auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/) |
| `links.score` | Internal authority scoring | Run a documented independent PageRank-style method on a known graph; save algorithm and parameter version. | [Link auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/internal-linking-audit-with-the-seo-spider/) |
| `links.orphans` | Orphan URL discovery | Merge sitemap/analytics/search data; distinguish absent internal links from incomplete crawl coverage. | [Orphan auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/find-orphan-pages/) |
| `links.uncrawlable` | Nonstandard link evidence | Record onclick, javascript URLs and href on non-anchor elements without assuming a valid navigable target. | [Version 24 release (19 May 2026)](https://www.screamingfrog.co.uk/blog/seo-spider-24/) |
| `links.fragments` | Bookmark validation | Verify decoded fragment destinations against IDs/names; distinguish a missing fragment from a failed page. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `links.crawl_graph` | Crawl tree and graph views | Render shortest-path hierarchy, focus/expand and status/indexability coloring from fixture data. | [Architecture visualization](https://www.screamingfrog.co.uk/seo-spider/tutorials/site-architecture-crawl-visualisations/) |
| `links.directory_graph` | Directory architecture views | Build directory hierarchy including synthetic grouping paths and label paths that are not fetched URLs. | [Architecture visualization](https://www.screamingfrog.co.uk/seo-spider/tutorials/site-architecture-crawl-visualisations/) |
| `links.graphs_3d` | 3D graph views | Display a bounded large graph and expose truncation/focus rather than silently omitting nodes. | [Architecture visualization](https://www.screamingfrog.co.uk/seo-spider/tutorials/site-architecture-crawl-visualisations/) |
| `links.segments` | Configurable segmentation | Apply ordered field/regex segments and filter counts, issues, charts and exports by segment. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |

### Canonical, multilingual and alternative pages (P2)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `relationships.canonical` | Canonical extraction | Extract HTML and HTTP Link canonical entries including relative, multiple and self-referencing values. | [Canonical auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-canonicals/) |
| `relationships.canonical_checks` | Canonical validation | Flag malformed, conflicting, redirecting, blocked and non-indexable targets and canonical chains. | [Canonical auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-canonicals/) |
| `relationships.pagination` | Pagination relationship validation | Extract HTML/HTTP next and prev; test broken links, absent returns and inconsistent chains. | [Pagination auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-pagination/) |
| `relationships.hreflang` | Hreflang extraction | Read HTML, HTTP Link and XML sitemap alternates with language/region and x-default. | [Hreflang auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-hreflang/) |
| `relationships.hreflang_checks` | Hreflang reciprocity and validity | Test unsupported codes, missing returns/self-reference, duplicates and non-indexable targets. | [Hreflang auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-hreflang/) |
| `relationships.amp` | AMP extraction and validation | Detect amphtml/canonical reciprocity and report versioned AMP validation findings. | [AMP auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-validate-amp/) |
| `relationships.mobile` | Mobile alternate relationships | Resolve alternate media relationships and surface missing or non-indexable target evidence. | [Mobile usability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-mobile-usability/) |

### Images and resource evidence (P2)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `images.sources` | Image candidates and occurrences | Record src/srcset candidates and every source page independently; preserve per-occurrence alt. | [Image auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-find-missing-image-alt-text/) |
| `images.alt` | Missing alt distinctions | Separate missing attribute from empty/whitespace alt using three deliberately different image elements. | [Image auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-find-missing-image-alt-text/) |
| `images.size` | Image dimensions and size audits | Record intrinsic/rendered dimensions, transfer size and configurable thresholds without mislabeling unavailable data. | [Image auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-find-missing-image-alt-text/) |
| `images.cookies` | Cookie audit | Collect response and script cookies with domain/path/security attributes and distinguish session contexts. | [Cookies](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-perform-a-cookie-audit/) |
| `images.pdf` | PDF metadata and text | Extract title, author, dates, pages and text from an owned fixture PDF; show unsupported encrypted files explicitly. | [PDF auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-pdfs/) |
| `images.pdf_links` | PDF links and anchor evidence | Resolve link annotations, record source document/page and extract text where reliable. | [PDF auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-pdfs/) |

### Browser rendering and custom scripts (P3)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `rendering.text` | Raw HTML mode | Fetch and parse a script-dependent fixture while proving no JavaScript execution. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.chromium` | Chromium rendering mode | Discover links created after script execution; save browser/version/config used for the crawl. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.wait` | Render completion policy | Control viewport, wait/timeout and resource loading; distinguish timed-out render from a successful raw fetch. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.source_diff` | Raw/rendered parity analysis | Surface elements and links added/removed by rendering while retaining both representations. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.screenshot` | Rendered screenshots | Capture a deterministic owned page at selected viewport and link the image to URL/snapshot. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.console` | Console and blocked-resource evidence | Save browser JS errors and failed/robots-blocked resource loads with URLs and error types. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.flatten` | Shadow DOM and iframe flattening | Extract owned fixtures with shadow roots and same-origin iframe content; preserve source context. | [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/) |
| `rendering.archive` | Website archiving | Save configured page/resource artifacts and verify a manifest can locate each stored file. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `rendering.custom_extract` | CSS/XPath/regex extraction | Return text, attributes, HTML, inner HTML and function values from a known fixture with multi-value results. | [Custom extraction](https://www.screamingfrog.co.uk/seo-spider/tutorials/web-scraping/) |
| `rendering.visual_extract` | Visual selector authoring | Choose an element, preview raw/rendered extraction, save selector and verify it on a second fixture. | [Custom extraction](https://www.screamingfrog.co.uk/seo-spider/tutorials/web-scraping/) |
| `rendering.custom_search` | Literal and regex search | Search source, rendered content and selected text with occurrence counts and match evidence. | [Custom search](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-custom-search/) |
| `rendering.javascript` | Custom JavaScript extraction | Run a user-authored script against a fixture with timeout/error handling and typed returned columns. | [JavaScript debugging](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-debug-custom-javascript-snippets/) |
| `rendering.interact` | Scripted interactions | Execute authored hover/click/scroll actions in declared order and discover resulting links. | [User interaction scripts](https://www.screamingfrog.co.uk/seo-spider/tutorials/simulating-user-interactions-with-custom-javascript/) |
| `rendering.snippet_library` | User snippet library | Import/export user-owned snippets without storing provider secrets in shared library files. | [JavaScript debugging](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-debug-custom-javascript-snippets/) |

### Structured data, usability and validation (P3)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `quality.structured_formats` | JSON-LD, Microdata and RDFa | Extract nested graphs from each format; keep parse errors and raw node references. | [Structured data](https://www.screamingfrog.co.uk/seo-spider/tutorials/structured-data-testing-validation/) |
| `quality.schema` | Schema vocabulary validation | Check types/properties against an identified schema version; test unknown and deprecated terms. | [Structured data](https://www.screamingfrog.co.uk/seo-spider/tutorials/structured-data-testing-validation/) |
| `quality.richresults` | Rich result requirements | Validate required/recommended fields using versioned public rules; distinguish errors from warnings. | [Structured data](https://www.screamingfrog.co.uk/seo-spider/tutorials/structured-data-testing-validation/) |
| `quality.accessibility` | axe accessibility findings | Run version-pinned axe on a deliberately failing rendered fixture; retain rule, impact, node and WCAG tags. | [Accessibility](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-perform-a-web-accessibility-audit/) |
| `quality.accessibility_limit` | Manual accessibility boundaries | Display automated-only coverage and incomplete/needs-review findings in report exports. | [Accessibility](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-perform-a-web-accessibility-audit/) |
| `quality.mobile` | Mobile usability checks | Test viewport, sizing, tap targets and incompatible content under defined mobile rendering. | [Mobile usability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-mobile-usability/) |
| `quality.html_head` | Invalid head elements | Seed invalid head children that affect subsequent metadata; point to observed raw markup. | [HTML head validation](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-debug-invalid-html-elements-in-the-head/) |
| `quality.html_validation` | HTML conformance checks | Run a versioned validator against owned malformed HTML and retain line/column/rule where available. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `quality.security` | Transport and header audits | Test mixed content, insecure forms, MIME mismatch and configurable security-header presence with concrete evidence. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `quality.carbon` | Green hosting and carbon estimate | Expose dataset/method/version and unknown states; label output as an estimate. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |

### Sitemaps (P2)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `sitemap.discovery` | Robots and explicit sitemaps | Load a sitemap index, gzipped child sitemap and explicit input; handle loops/malformed XML safely. | [Sitemap auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-xml-sitemaps/) |
| `sitemap.coverage` | Sitemap coverage and orphan checks | Compare fetched pages and sitemap locations; distinguish not listed, sitemap-only and incomplete crawl. | [Sitemap auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-xml-sitemaps/) |
| `sitemap.validity` | Sitemap URL validation | Find redirect/non-200/non-indexable URLs and invalid lastmod values in fixture XML. | [Sitemap auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-xml-sitemaps/) |
| `sitemap.generate` | XML sitemap generation | Export valid XML with optional lastmod, image entries and hreflang; parse output with an independent parser. | [Sitemap generation](https://www.screamingfrog.co.uk/seo-spider/tutorials/xml-sitemap-generator/) |
| `sitemap.split` | Split and segmented sitemaps | Split at configured URL/byte bounds, create index and prove segment URL membership. | [Sitemap generation](https://www.screamingfrog.co.uk/seo-spider/tutorials/xml-sitemap-generator/) |

### Authorized API integrations (P4)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `integrations.ga4` | Google Analytics 4 | Using an authorized property, join page metrics with declared URL matching and preserve absent data as absent. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `integrations.gsc` | Search Console analytics | Fetch selected dates/dimensions and join clicks/impressions/CTR/position; report API errors and row coverage. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `integrations.inspection` | Search Console URL inspection | Honor daily/property quotas; retain inspection timestamp and distinguish inspected from queued URLs. | [URL Inspection API](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-automate-the-url-inspection-api/) |
| `integrations.psi` | PageSpeed Insights and Lighthouse | Preserve mobile/desktop, field/lab distinction, metric units and provider error details. | [Core Web Vitals](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-core-web-vitals/) |
| `integrations.cwv` | Core Web Vitals aggregation | Compute pass/fail from available field metrics; never substitute lab results for unavailable field data. | [Core Web Vitals](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-audit-core-web-vitals/) |
| `integrations.majestic` | Majestic metrics | Use separately authorized credentials; mock fresh/historic datasets, quota exhaustion and URL/domain scopes. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `integrations.ahrefs` | Ahrefs metrics | Use independent authorized app/API access; mock expiry, quota and metric joins without reusing vendor tokens. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `integrations.moz` | Moz metrics | Mock authorized URL/domain metric queries with unavailable/quota states. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `integrations.oauth` | Account connection management | Add, disconnect and revoke an authorized provider account; verify tokens stay out of logs and version control. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `integrations.provider_urls` | API-discovered crawl candidates | Optionally enqueue provider-only URLs with provenance and scope checking; preserve orphan evidence. | [Orphan auditing](https://www.screamingfrog.co.uk/seo-spider/tutorials/find-orphan-pages/) |

### Optional AI and semantic analysis (P4)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `ai.providers` | Cloud and local model adapters | Use mock OpenAI-compatible/Gemini/Anthropic and a local Ollama endpoint; record model/provider errors separately. | [AI prompts](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-crawl-with-ai-prompts/) |
| `ai.prompts` | Custom prompts and test preview | Run an authored prompt on selected page text/HTML/extraction; preview exact input and persist typed output. | [AI prompts](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-crawl-with-ai-prompts/) |
| `ai.prompt_library` | Prompt import/export | Round-trip user-authored prompts with model/settings and no secrets. | [AI prompts](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-crawl-with-ai-prompts/) |
| `ai.embeddings` | Embedding generation | Embed selected text, cache with model/input hash and reject incompatible dimensionality. | [Semantic similarity](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-identify-semantically-similar-pages-outliers/) |
| `ai.similarity` | Semantic similarity | Rank a known vector fixture using a documented cosine method and configured thresholds. | [Semantic similarity](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-identify-semantically-similar-pages-outliers/) |
| `ai.outliers` | Semantic relevance and outliers | Report low-centroid-similarity fixture vectors with exact scores and declared corpus coverage. | [Semantic similarity](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-identify-semantically-similar-pages-outliers/) |
| `ai.semantic_search` | Semantic search and clusters | Query saved vectors and visualize clusters while retaining model/dimensionality provenance. | [Semantic similarity](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-identify-semantically-similar-pages-outliers/) |
| `ai.redirect_mapping` | Semantic redirect suggestions | Rank candidate target pages from a deterministic vector fixture and export suggestions with score; never apply redirects automatically. | [Semantic redirect mapping](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-use-vector-embeddings-for-redirect-mapping/) |
| `ai.usage` | Provider cost and rate controls | Enforce explicit rate/spend bounds for an API fixture, record retries and distinguish local from paid providers. | [AI prompts](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-crawl-with-ai-prompts/) |

### Persistence, projects and portability (P2)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `storage.database` | Durable local database | Crash after committed requests; recover completed results and pending queue with consistent graph state. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `storage.memory` | Optional memory storage | Run the same fixture in memory and disk modes; compare results while documenting mode differences. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `storage.snapshots` | Save/open/import/export snapshots | Round-trip an Open SEO Spider snapshot including config, graph, issues and optional artifacts; verify schema version. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `storage.projects` | Projects and crawl history | Group snapshots by project, label them, reopen them and record their creation/config identity. | [Team workflows](https://www.screamingfrog.co.uk/seo-spider/tutorials/seo-spider-working-in-teams/) |
| `storage.retention` | Retention and locking | Use test timestamps to expire unlocked fixture snapshots while preserving locked snapshots and active jobs. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `storage.config` | Reusable crawl configuration | Export/import typed config and validate missing credentials/paths without silently changing settings. | [Team workflows](https://www.screamingfrog.co.uk/seo-spider/tutorials/seo-spider-working-in-teams/) |
| `storage.portable` | Shared crawl portability | Copy an Open SEO Spider archive to a second local directory and open all declared portable artifacts. | [Team workflows](https://www.screamingfrog.co.uk/seo-spider/tutorials/seo-spider-working-in-teams/) |
| `storage.compare_changes` | Field and content changes | Detect title, description, headings, graph, directives, structured data and content changes even if issue counts are unchanged. | [Crawl comparison](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-compare-crawls/) |
| `storage.compare_mapping` | Cross-host comparison mapping | Map staging/production or migrated URL prefixes before joining snapshots and expose collisions. | [Crawl comparison](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-compare-crawls/) |
| `storage.compare_export` | Comparison export | Export both snapshots and deltas with new/missing/changed evidence and stable URL keys. | [Crawl comparison](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-compare-crawls/) |

### Automation, exports and agent access (P4)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `automation.tables` | Table and bulk exports | Export filtered observations and all-occurrence evidence as CSV/XLSX; escape formulas and preserve Unicode/newlines. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `automation.reports` | Issue and specialist reports | Export versioned overview plus redirect/canonical/hreflang/sitemap/security/content report fixtures. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `automation.assets` | Artifact exports | Export source, rendered HTML, screenshots, headers, cookies and PDF artifacts with a manifest. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `automation.looker` | Google Sheets/Looker output | Use an authorized export destination; retain append/replace setting and stable column schema across two runs. | [Automated report exports](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-automate-crawl-reports-in-data-studio/) |
| `automation.cli` | Headless command line | Run a crawl/config/import/analysis/export sequence from CLI and verify deterministic exit codes. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `automation.scheduler` | Recurring scheduling | Use a fake clock to execute saved crawl tasks, record history/errors and define overlap policy. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `automation.auto_compare` | Automatic last-two comparison | After two completed project snapshots, create a comparison automatically using saved comparison settings. | [Version 24 release (19 May 2026)](https://www.screamingfrog.co.uk/blog/seo-spider-24/) |
| `automation.notifications` | Completion and change notifications | Send only to an explicitly configured local test SMTP sink; verify summary, change table and attachment error handling. | [Notifications](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-set-up-crawl-email-notifications-export-attachments/) |
| `automation.mcp` | MCP agent interface | Perform MCP initialize/list-tools/crawl/read/export using a local client; keep script execution an explicit capability. | [Version 24 release (19 May 2026)](https://www.screamingfrog.co.uk/blog/seo-spider-24/) |
| `automation.mcp_transport` | STDIO and streamable HTTP | Exercise both transports on loopback and prove JSON protocol framing and job identifiers remain stable. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |
| `automation.stats` | Local usage statistics | Aggregate locally saved crawl counts/duration without sending telemetry by default. | [Version 24 release (19 May 2026)](https://www.screamingfrog.co.uk/blog/seo-spider-24/) |

### Analyst interface (P2)

| Key | Capability | Proposed acceptance test | Source |
| --- | --- | --- | --- |
| `ui.realtime` | Live crawl progress and tables | During a delayed fixture crawl, update progress/results and permit safe pause/config inspection. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `ui.filter` | Sort, search and saved columns | Combine numeric/text filters and sort; keep column preferences when reopening a project. | [General operation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/) |
| `ui.overview` | Overview and issue taxonomy | Show severity/type/count and drill-down to every affected URL; distinguish observation from advice. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `ui.details` | URL evidence inspector | Select a URL and inspect links, resources, raw/rendered source, headers, cookies and structured nodes. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `ui.charts` | Site structure and timing charts | Aggregate folder/depth/status/timing over a fixture dataset with consistent filtered totals. | [Data tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/) |
| `ui.localization` | Language and UI preferences | Switch supported interface locale/theme/font settings while preserving stored crawl text exactly. | [Configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/) |

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

