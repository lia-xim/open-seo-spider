# Granular acceptance backlog

Created 2026-10-12T07:47:33.321Z; reference24.3. Full vendor parity **NOT PROVEN**.

416 installed named filters each have one stable tracker record; 32 additional workflow contracts cover behavior outside filters. Existing 144-capability statuses are untouched.

A filter record separates observed argument availability, proposed positive/control inputs, data/parameters, edge cases, related implementation evidence and unmeasured vendor membership. Existing implemented/partial/planned capability slices are retained as context; they are not inherited as per-filter statuses.

**The 857 matching cells in the first measured comparison are exported cell comparisons, not features and not a product coverage percentage.**

| Priority | Named filter records | Meaning |
| --- | ---: | --- |
| R1 | 161 | Status, robots, metadata, URL and relationship correctness |
| R2 | 215 | Resource/content/browser/validator and comparison detail |
| R3 | 40 | Authorized upstream metrics and verdicts |

| Tab | Named views | Contract detail |
| --- | ---: | --- |
| AMP | 17 | Individual proposed trigger/control/edge contracts in JSON |
| Accessibility | 102 | Authored DOM pairs + actual pinned axe signal/control evidence; original mapping unproven |
| Analytics | 5 | Individual proposed trigger/control/edge contracts in JSON |
| Canonicals | 12 | Individual proposed trigger/control/edge contracts in JSON |
| Change Detection | 14 | Individual proposed trigger/control/edge contracts in JSON |
| Content | 11 | Individual proposed trigger/control/edge contracts in JSON |
| Directives | 17 | Individual proposed trigger/control/edge contracts in JSON |
| External | 11 | Individual proposed trigger/control/edge contracts in JSON |
| H1 | 6 | Individual proposed trigger/control/edge contracts in JSON |
| H2 | 5 | Individual proposed trigger/control/edge contracts in JSON |
| Hreflang | 14 | Individual proposed trigger/control/edge contracts in JSON |
| Images | 7 | Individual proposed trigger/control/edge contracts in JSON |
| Internal | 11 | Individual proposed trigger/control/edge contracts in JSON |
| JavaScript | 18 | Individual proposed trigger/control/edge contracts in JSON |
| Links | 12 | Individual proposed trigger/control/edge contracts in JSON |
| Meta Description | 8 | Individual proposed trigger/control/edge contracts in JSON |
| Meta Keywords | 3 | Individual proposed trigger/control/edge contracts in JSON |
| Mobile | 6 | Individual proposed trigger/control/edge contracts in JSON |
| Page Titles | 9 | Individual proposed trigger/control/edge contracts in JSON |
| PageSpeed | 20 | Individual proposed trigger/control/edge contracts in JSON |
| Pagination | 10 | Individual proposed trigger/control/edge contracts in JSON |
| Response Codes | 34 | Individual proposed trigger/control/edge contracts in JSON |
| Search Console | 11 | Individual proposed trigger/control/edge contracts in JSON |
| Security | 13 | Individual proposed trigger/control/edge contracts in JSON |
| Sitemaps | 7 | Individual proposed trigger/control/edge contracts in JSON |
| Structured Data | 11 | Individual proposed trigger/control/edge contracts in JSON |
| UNDEF | 1 | Individual proposed trigger/control/edge contracts in JSON |
| URL | 11 | Individual proposed trigger/control/edge contracts in JSON |
| Validation | 10 | Individual proposed trigger/control/edge contracts in JSON |

Independent accessibility fixture results: PASS_PAIR=79, INCOMPLETE_REVIEW_PAIR=12, POSITIVE_NOT_CONFIRMED=2, CONTROL_STILL_VIOLATES=0. Manual-review signals remain separate from automated violations. See `accessibility-fixtures.json` and `accessibility-fixture-results.json`.

Original named-filter membership: named_filters=78, csv_files=87, positive_control_pairs=64, refined_control_pairs=4, partial_observations=10. See `reference-membership/membership.md`; unknown eligibility and contradictory proposals remain recorded, without changing implementation or parity statuses.

## Candidate data/display profiles

- **native-evidence**: Store requested-response and final/rendered-document data independently; preserve all link/resource kinds and all reasons. Do not discard richer observations to imitate a table.
- **reference-table-candidate**: Initial redirect/error rows expose their requested status; final metadata shown in dedicated final-document fields. Add every intermediate redirect URL to compatibility table. Count anchors only in Links-view outlinks; expose IMG/resource metrics separately.
- **resource-indexability-candidate**: Document indexability and resource indexing eligibility are separate booleans/reasons. PNG/JPEG/PDF can be indexable resources; content type alone is not a noindex directive.
- **reason-taxonomy-candidate**: Map localized reference reason names to semantic codes while retaining all underlying reasons and precedence. Cosmetic label match is separate from decision match.
- **measurement-calibration**: Pixel width, content tokenization, rank, MinHash and score boundaries require recorded algorithm/config/version. Approximate measurements must never masquerade as calibrated reference values.

## Additional workflow acceptance

- **crawl.robots-agent-groups (R1)** — robots: wildcard+specific agents, Allow/Disallow ties, encoded path, redirects and missing/503 robots. Expected: Selected-agent request log proves denied targets never fetched; fail-policy explicitly recorded.
- **crawl.url-identity (R1)** — Relative/base href, fragments, IDN, default ports, percent encoding, query order/duplicates and trailing slash. Expected: Only specified normalizations deduplicate; path/query case or order changes are not silently merged.
- **crawl.scope-credentials (R1)** — Two localhost origins with custom Authorization header; same-origin redirects versus foreign redirect/resources. Expected: No configured credentials reach foreign origin; known request provenance retained.
- **crawl.include-exclude (R1)** — Explicit seed exempt, overlapping include/exclude expressions, sitemap-discovered URL, invalid regex. Expected: Apply validated policy before enqueue; no network request for excluded discovery.
- **crawl.request-retry (R1)** — 503 twice then200;429 Retry-After; closed socket; streaming body stalls. Expected: Bounded configured retry/backoff, error category and request counts; no fake zero status.
- **crawl.user-resource-limits (R1)** — URL cap/depth/path sampling, max bytes, cancel while workers active, per-origin throttling. Expected: User limits are provenance; lower or absent limit preserves no commercial cap in independent engine.
- **data.request-and-final-document (R1)** — A301→B302→C200 with C title/canonical/noindex and A Location/header. Expected: Keep initial response and final document observations separate. Reference-table display can hide final metadata on A without deleting C observation.
- **data.redirect-intermediate-rows (R1)** — A→B→C; B only encountered as redirect hop and later as direct seed. Expected: Hop ledger always retained; compatibility table adds B row with its initial302 once; direct fetch provenance remains distinguishable.
- **data.anchor-resource-metrics (R1)** — Page contains three img occurrences, two repeated anchors, one canonical and one stylesheet. Expected: All resources retained by kind; reference Links view counts anchors only. Separate occurrence and unique-target/source metrics.
- **data.resource-indexability (R1)** — 200 PNG, PDF, XML, HTML plus header-noindex variants. Expected: Separate crawler fetchability, Google-style resource indexing eligibility, HTML document indexability and policy profile; non-HTML is not intrinsically non-indexable.
- **analysis.independent-link-score (R2)** — Directed graph with dangling node, self-loop, repeated edges, nofollow, page nofollow, partial boundary. Expected: Deterministic PageRank facts/config and scope; reference LinkScore requires algorithm calibration and cannot be claimed from normalized rank alone.
- **analysis.orphan-source-cohort (R2)** — Sitemap-only, GA-only/GSC-only, self-link-only, partial-crawl and fetched anchor-discovered candidates. Expected: Observed cohort and graph scope bound claim; unavailable integration cannot prove no orphan.
- **analysis.content-area (R2)** — Header/nav/footer identical; main text differs; CSS selector include/exclude and content types HTML/PDF. Expected: Store selected text/selection config and tokenization; keep full-HTML digest separate from text similarity.
- **rendering.wait-interactions (R2)** — Timer after load, fetch-inserted DOM, click/scroll action, infinite requests, post-action navigation. Expected: Bounded actions/wait with recorded browser/config; distinguish wait elapsed, script failure and completed observation.
- **rendering.iframes-shadow-dom (R2)** — Same/foreign-origin iframe and open/closed shadow roots with anchors/metadata. Expected: Source provenance retained, explicit accessibility/browsing boundaries; unsupported closed root is unknown.
- **extraction.css-xpath-regex (R2)** — Text/innerHTML/outerHTML/attribute; zero/multiple values; XPath text/attr/nodes; regex capture groups. Expected: Validate before crawl; columns remain stable; unavailable/invalid extraction distinct from zero match.
- **extraction.custom-search (R2)** — Contains/not-contains, literal/regex, raw/rendered HTML/text, case sensitivity, overlapping matches. Expected: Named filter results with scope/config retained; binary/nonHTML and disabled extraction explicit.
- **extraction.custom-javascript (R2)** — Action then extraction promise; throw/reject, selector missing, primitive/object return and internal-only policy. Expected: Return structured per-snippet result/error; no implicit API messages or cross-origin credentials.
- **storage.stop-resume-crash (R1)** — Pause/cancel, terminate actual process midfrontier, restart/resume, disk-write failure. Expected: Completed URL state/frontier/config persisted; user-visible resumable state and no refetch of completed seed.
- **storage.project-config-auth (R2)** — Export/import project with named config, credentials redacted, relative paths and schema upgrade. Expected: Versioned owned format, portable nonsecret state, no proprietary .seospider compatibility claim without separate proof.
- **storage.compare-mapping (R2)** — Before live host to staging host/path rewrite; added/removed; redirect identity; fields reordered. Expected: Explicit mapping profile and normalized differences; mapping collision is error, not silent merge.
- **auth.basic-digest-form (R3)** — Owned fixture401 Basic/Digest and session form requiring cookie/CSRF; failed auth and logout. Expected: Authorized credentials only exact allowed origin; no login/2FA/security prompt automation or license change.
- **provider.oauth-token-lifecycle (R3)** — Provider mock with expiry/refresh/revoke, permission denied, quota and pagination. Expected: Provider parsing proof distinct from authorized live proof; no token in repo/export/log.
- **provider.psi-cwv (R3)** — Device-specific PSI/Lighthouse audit sets and missing CrUX field cohort. Expected: Pinned lab versus field provenance; missing cohort is unavailable, not zero.
- **provider.backlinks (R3)** — Owned/mock URL lookup with redirect/canonical mismatch and quota errors. Expected: Service-specific metrics/units/account/query timestamp retained; account fees/quotas not bypassed.
- **ai.prompts-embeddings (R3)** — Prompt input containing untrusted page instructions; embedding vectors/missing model/quota/usage. Expected: Provider/key authorization, returned model/vector dimensions/version and usage; no silent external call from crawl default.
- **ui.filter-details-export (R1)** — Select one URL/issue, AND/OR search across columns, stable sort/pagination, CSV/XLSX export with commas/newlines/formulas. Expected: View count matches exact serialized predicate; source/target lower rows reachable; export security and quoting tested.
- **ui.graphs-segments (R2)** — Crawl/directory/content cluster layouts with selected segment and large partial graph. Expected: Topology versus layout separated; SVG/image/HTML export and segment membership explicit; drawing alone is not graph-algorithm proof.
- **automation.cli-scheduler-mcp (R2)** — Headless CLI nonzero failure, overlapping schedules/timezones, MCP stdio/HTTP lifecycle and tool errors. Expected: Same config/data behavior across transports; schedule persists/restarts; actual SDK tests bound protocol claims.
- **automation.reports-assets (R2)** — Multiple export tabs, screenshot/source archive, empty report skip, folder timestamping, notification disabled. Expected: Owned deterministic artifacts and separate explicit authorization for any external message/cloud write.
- **modes.list-sitemap-serp-api (R2)** — URL list dedupe, sitemap index import, SERP preview with selected device, integration-only URL cohort. Expected: Explicit traversal rules/mode provenance; preview is not measured Google search appearance.
- **platform.native-wsl-container (R2)** — Native Windows release, Ubuntu CI, browser installation and Docker start/stop/data volume. Expected: Actual install/run/protocol evidence per platform; absent Docker/WSL run stays NOT_PROVEN.

## Source and proof boundaries

- Current installed help is retained as names/metadata only in `reference-cli-inventory.json`:1149 arguments across34 tabs,416 named filters,700 configurable slots,33 All aggregates.
- `fixture-cases.json` contains original independent fixture HTML/transport cases. `reference-comparison.json` retains actual values and individual mismatches for both engines.
- The original normal CLI succeeds on the small local fixture with missing licence. Paid feature/config entitlement, proprietary project-file compatibility and complete reference filter behavior are not established.
- Existing static JAR metadata is architectural context; it is not execution evidence or recovered proprietary algorithms.

Official primary sources for reviewed specifics:
- [filter.canonicals-invalid-attribute-in-annotation](https://www.screamingfrog.co.uk/seo-spider/issues/canonicals/invalid-attribute-in-annotation/)
- [filter.canonicals-outside-head](https://www.screamingfrog.co.uk/seo-spider/issues/canonicals/canonicals-outside-head/)
- [filter.page-titles-outside-head](https://www.screamingfrog.co.uk/seo-spider/issues/page-titles/page-title-outside-head/)
- [filter.meta-description-outside-head](https://www.screamingfrog.co.uk/seo-spider/issues/meta-description/meta-description-outside-head/)
- [filter.directives-outside-head](https://www.screamingfrog.co.uk/seo-spider/issues/directives/directive-outside-head/)
- [filter.validation-invalid-html-elements-in-head](https://www.screamingfrog.co.uk/seo-spider/issues/validation/invalid-html-elements-in-head/)
- [filter.validation-head-not-first-in-html-element](https://www.screamingfrog.co.uk/seo-spider/issues/validation/head-not-first-in-html-element/)
- [filter.images-incorrectly-sized-images](https://www.screamingfrog.co.uk/seo-spider/issues/images/incorrectly-sized-images/)
- [filter.images-missing-alt-text](https://www.screamingfrog.co.uk/seo-spider/issues/images/missing-alt-text/)
- [filter.content-soft-404-pages](https://www.screamingfrog.co.uk/seo-spider/issues/content/soft-404-pages/)
- [filter.content-near-duplicates](https://www.screamingfrog.co.uk/seo-spider/issues/content/near-duplicates/)
- [filter.content-exact-duplicates](https://www.screamingfrog.co.uk/seo-spider/issues/content/exact-duplicates/)
- [filter.links-pages-with-high-crawl-depth](https://www.screamingfrog.co.uk/seo-spider/issues/links/pages-with-high-crawl-depth/)
- [filter.links-pages-with-uncrawlable-internal-outlinks](https://www.screamingfrog.co.uk/seo-spider/issues/links/pages-with-uncrawlable-internal-outlinks/)
- [filter.hreflang-missing-return-links](https://www.screamingfrog.co.uk/seo-spider/issues/hreflang/missing-return-links/)
- [filter.structured-data-validation-errors](https://www.screamingfrog.co.uk/seo-spider/issues/structured-data/validation-errors/)
- [filter.security-unsafe-cross-origin-links](https://www.screamingfrog.co.uk/seo-spider/issues/security/unsafe-cross-origin-links/)
- [filter.sitemaps-xml-sitemap-over-50mb](https://www.screamingfrog.co.uk/seo-spider/issues/sitemaps/xml-sitemap-over-50mb/)
- [filter.search-console-rich-result-invalid](https://www.screamingfrog.co.uk/seo-spider/issues/search-console/rich-result-is-invalid/)

General references: [configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/), [tabs](https://www.screamingfrog.co.uk/seo-spider/user-guide/tabs/), [normal CLI](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/).

Regenerate with `node scripts/reference-backlog.mjs`; validate that exactly416 unique named arguments remain mapped. Do not count unresolved contract design as implementation work completed.

## Bounded heading-order calibration

The corrected native implementation matches 48 retained H1/H2 membership observations on 24 authored raw HTML cases: [native comparison](native-heading-comparison.json). The two scoped contracts retain their former trigger proposals and original contradictions, alongside the refined first-H1/first-H2 behavior. This does not establish rendered, noindex, malformed, incomplete or whole-filter parity. All parity statuses remain NOT_PROVEN.
