# Sitemap filter depth: remaining acceptance

Status: **OPEN / NOT RUN**. This is an acceptance plan, not another runtime PASS. It supplements the seven Sitemaps filter contracts in [granular-acceptance.json](granular-acceptance.json) and the separate [physical storage/export gates](sitemap-budget-plan.md). Own capability checkmarks do not prove these individual filters or vendor parity.

The current native XML parser has independent defaults of8MiB entity/XML and50,000 retained entries, and the fetch path applies the smaller configured body/default entity limit. Its validated limit type currently permits lowering those defaults. The full retained ledger has separate row/catalog bounds. These are resource/retention policies; hitting one does not establish compliance with a protocol threshold that was never observed. Exact source pins are in [sitemap-filter-depth-source-pins.json](sitemap-filter-depth-source-pins.json).

The published protocol permits up to50,000 URLs and52,428,800 uncompressed bytes per sitemap, with corresponding index entry/byte bounds. Gzip size is assessed after decompression. [Sitemaps protocol](https://www.sitemaps.org/protocol.html)

## Individual filters

| Contract ID | Positive and control | Required evidence |
| --- | --- | --- |
| `filter.sitemaps-urls-in-sitemap` | Original requested page listed; fetched unlisted page as control | Exact original PageLoc, all occurrences/source provenance and separate admitted/final identities. Duplicate same-file entries remain separate. |
| `filter.sitemaps-urls-not-in-sitemap` | Fetched page absent from a complete declared source cohort; listed control | Current versus historical cohort, all roots/recursive sources and unavailable/incomplete controls. No data from an undeclared or denied source is not a negative proof. |
| `filter.sitemaps-orphan-urls` | Listed sitemap-only page with zero observed nonself anchors; linked and self-only controls | Explicit captured graph/cohort, exact anchor counts and bounded source samples. Partial crawl and unavailable GA/GSC/provider cohorts cannot establish a website-wide orphan. |
| `filter.sitemaps-non-indexable-urls-in-sitemap` | Actual301→200,404,noindex,canonical-other,robots blocked,failed/truncated and rendered navigation/status cases; direct indexable200 control | Original membership binds its own requested observation. Rewrites and final targets remain separate. Missing response/render facts stay unknown/incomplete. |
| `filter.sitemaps-urls-in-multiple-sitemaps` | Same original PageLoc in two sibling document URLs; twice in one file as control | Total occurrences and distinct original requested source document count are separate. Repeat observations of one task URL do not manufacture a second sitemap. Redirect-final source aliases do not replace original source identity. |
| `filter.sitemaps-xml-sitemap-with-over-50k-urls` | Actual49,999/50,000/50,001 URL entries; repeat for index child entries | Real correctly namespaced immediate entries with actual Locs; independent fixture/parser counts. Missing Loc, duplicate Loc in one entry, extension Loc, comments/CDATA and malformed suffix have explicit controls. Entry/row/retention-limit failure alone is not a measured50,001-URL result. |
| `filter.sitemaps-xml-sitemap-over-50mb` | Actual52,428,799/52,428,800/52,428,801 decoded XML bytes, plain and gzip; repeat for index | Hash/count actual delivered/decompressed bytes. HTTP Content-Length, gzip ISIZE, compressed byte count and declared charset are not substitutes. Earlier8MiB/resource stops remain unknown at50MiB. |

Every filter needs configured source/document traversal settings, disabled/unavailable cases, API/export and actual browser membership checks. A loaded-page UI filter must identify its page scope; it must not claim to filter the complete saved catalog without a full stored query.

## Ordered tasks

1. [ ] **S1 — Finish the current target/inlink/source-document projection.** Preserve raw rows/CSV and lastmod observations. Complete real Crawler/SQLite tests, bounded independent pagination, stale/role-mismatched cursor controls, API/SDK and actual browser proof. Current Work drafts are not completion evidence.
2. [ ] **S2 — Finish generation separately.** Parse actual API/CLI/MCP/browser ZIP outputs independently, check complete file/index/membership hashes and exact byte/count segmentation. A successful generated50,001-URL split does not prove auditing a fetched50,001-entry source.
3. [ ] **S3 — Implement source protocol-size/count assessment.** Persist measured facts and their completeness independently of retained membership rows. Define complete/compliant, observed-exceeded and unknown explicitly. A lower bound may prove excess; it cannot prove a complete document count, CRC or valid XML. Preserve actual request/transport/decode context and keep index entries separate from page entries.
4. [ ] **S4 — Make large-source processing practical and explicit.** Support real protocol-sized sources through a documented, configurable bounded path and durable paging/streaming. Avoid requiring an unbounded full report/catalog allocation. Do not silently raise production retention limits, suppress failures or add test-only overrides. When capture cannot complete, display the reason and keep source coverage unknown.
5. [ ] **S5 — Run the physical count/byte matrix.** Use owned fixtures and actual current production paths, valid Locs, same-file duplicates, sibling files, prefixed/default/foreign namespaces, plain/gzip/HTTP-gzip distinctions, truncated/CRC-bad/DTD suffixes and index controls. Configured max_documents/max_urls and admission rollback stay visible. Record all attempted cases, including failures; close every producer and drain fixture connections before final request assertions.
6. [ ] **S6 — Complete filter surfaces.** Add full stored selection/export where required, preserve provenance and unknowns in API/CLI/MCP/browser, prove pagination/filter interactions and narrow/wide UI. Do not silently fetch other pages or use the first loaded200 rows as whole-crawl results.
7. [ ] **S7 — Publish criterion-by-criterion acceptance.** Update each granular filter with its actual positive/control/parameter/edge proof, source/binary/harness hashes and scope. Keep parser controls, native fixtures, optimized release, installation, remote platforms and vendor comparison separate. Update the living TODO and GitHub issue without inflating capability counts from repeated executions.

## Proof boundaries

The accepted optimized Windows source/binary and physical JSON/CSV gates remain immutable. Current generator and target-analysis drafts require a fresh combined source closure and runtime acceptance. Large build/test execution is temporarily coordinated with another local project; that hold is not a waiver or an acceptance result.

No manufacturer algorithm equivalence, million-URL scalability, every integration source, every granular filter or current Linux/container/remote run is established by this document. Missing evidence remains **NOT_PROVEN**.
