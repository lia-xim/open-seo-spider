# Dashboard segments and bounded exports

Local acceptance passes. Fresh public CI remains a separate gate. The installed
Source16 application and user crawl store were not updated.

The final normal executable has SHA-256
`794ffe668b1724240ab0ba43cc6df1f0de8893011fb7c92d18f2e057cc844ad3`.
[local-result.json](local-result.json) records the actual phases and raw-evidence
hashes; [source-gates.json](source-gates.json) binds 485 unchanged source/driver
inputs before and after the final browser phases.

## Resulting behavior

The main dashboard can explicitly apply browser-local saved rules to All, one
segment, Unassigned or Unknown, in automatic/reported/raw mode. It loads every
200-row native page and issue cursor and the bound analysis before installing
the selection. Draft edits, reload, rule storage events, crawl/generation changes
and stale callbacks invalidate the prior view and exports.

Selected pages retain complete original PageRecords and their raw ordinals;
duplicate URLs do not substitute a different observation. Issues use the union
of selected exact URL owners; genuine orphan issues remain separately counted.
Page-owned tables, counts and charts use the cohort. Queue/discovery/activity and
specialist input/policy/resource/sitemap/content inventories retain explicit
original-crawl scope. Graph metrics and relationship targets use the full original
capture before presentation filtering. An outside target does not become unknown
or external because its source cohort excludes it.

Native routes under `/api/crawls/{id}/segments/` are:

- `dashboard`: original page/issue rows, complete aggregates, cursor and context.
- `dashboard-analysis`: required preview token, full-capture analysis with selected
  owners, original target facts, responses and fragments.
- `dashboard.json`: complete named cohort envelope, distinct from original raw JSON.
- `dashboard/{kind}.csv`: pages, issues, links, DOM, extractions, JavaScript, cookies,
  PDF, images, custom-search, accessibility, responses and fragments.

All exports bind report, rules, selection, reporting view and App generation.
Length, SHA-256, no-store, token and generation headers are verified before browser
downloads. Native CSV writers append 13 cohort-context columns and write rows
directly into a bounded sink. Fragment resolution retains full original targets
and filters source ordinals before copying payloads. Redirect-response production
also stops when the sink refuses. Original raw report bytes remain unchanged.

## Executed gates

| Gate | Actual result and boundary |
| --- | --- |
| Ordinary workspace Rust | 1,766 passed, zero failures, five deliberate ignores; before the subsequent JavaScript-only selection guard, with Rust implementation unchanged |
| Explicit Chromium Rust fixtures | Four named fixtures actually executed separately; positive API-provider coordinator was not rerun in this slice |
| Native dashboard | Seven tests, original ordinals/profile/stale contexts/full-target graph facts, all 13 CSV kinds, pre-derivation controls and physical 64 MiB exactly/+1-byte refusal |
| Frontend state / semantic VM | 68 state checks; 40 unchanged historical-harness groups against explicitly reviewed current source bytes; these are not browser E2Es |
| Final strict Clippy | All targets/features, warnings denied, after the selection guard |
| Final browser E2E | All 37 registered suites, explicitly in three phases: 2 focused + 1 dashboard + 34 remaining; nine API fixtures and 28 native suites |

Every final browser phase used the identical executable and unchanged bound
sources, with natural app shutdown 0. This is not a claim of one shared-store
37-suite invocation. No successful long suite was repeated unnecessarily.

The dashboard suite captured 1,010 real local pages and verified every selected
title across all 1,005 product rows, every native cursor, 13 applied projections
and 23 complete downloads. It includes unknown/unassigned/overlapping/empty groups,
reported/raw profiles, an original ordinal beyond 1,000, stale refusals, cross-tab
rule edits, an explicitly declared duplicate-URL import and two actual native
processes. Cold exports are byte-identical; the active HTTP410 witness records
zero product refetches through both EOF0 closes and browser shutdown, with two
successful control probes. Declared import controls are not fresh-crawl provenance.

The density suite captured 105 real pages. All first-100 and final-five closed
rows measured 83.125px at 320px. Full values remain selectable and accessible by
keyboard, native marker and touch; expanded/collapsed full exports match. The
test found and reproduced a real defect: mouse selection collapsed a native
summary. A narrow pointer-selection guard fixes it while retaining native
keyboard and marker activation. The final driver measures and hit-tests the
marker rather than guessing its screen coordinates.

## Limits and earlier failures

Input limits remain 50,000 pages, 100,000 issues and 64 MiB of typed report.
Segment projection and paged/analysis responses have a 32 MiB budget; complete
JSON/CSV has a 64 MiB budget. Conservative work preflight covers canonical cycles,
duplicate-keyword groups and complete fragment-target payload amplification.
These are logical/work/output bounds, not a universal RSS or CPU-time guarantee.

A real remaining case was exposed: 1,010 pages with repeated descriptions created
a 45,247,815-byte saved report with 1,015,056 related-URL entries. The existing
32 MiB segment projection correctly refused it; this was not a 45 MB response.
The positive fixture now has unique descriptions. The refusal is retained as
evidence of an open large-corpus limitation, not broad 1,000-page usability proof.

Earlier exploratory failures remain hashed in the receipt: this budget refusal,
30-second event and four-minute workflow deadlines during valid paging, a wrong
one-issue expectation for an import that actually retains two exact findings,
the real pointer-selection defect and the guessed marker coordinate. Final waits
are bounded at 120 seconds for the analysis event and 900 seconds for this large
dashboard workflow only; the other 36 workflows retain 240 seconds.

Public run [39127069406](https://github.com/lia-xim/open-seo-spider/actions/runs/39127069406)
on predecessor `5d512cf` failed. Ubuntu/Windows rejected stale historical fixture
hashes; the renderer failed a resource-sources CDP response-body read after its
accessibility/cold suites passed. The checker now preserves historical contracts
and explicitly pins reviewed current targets. The resource driver uses the exact
observed pure-read request as an independent native oracle, without replaying
mutations. Both repairs pass locally; the historical public run remains failed.

The catalog remains **38 own acceptances / 69 partial / 37 planned**, covering
144 grouped capabilities plus 416 named-filter and 32 workflow contracts. All
manufacturer parity remains **NOT_PROVEN**. Further operators/combinations,
large-corpus performance, additional specialist integrations, server-side rule
storage, fresh browser profiles and crash recovery need separate acceptance.
