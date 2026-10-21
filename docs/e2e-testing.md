# End-to-end checks for the redesigned application

The first public successor run for `2c573d9` failed its dashboard fixture: a legacy extraction without source evidence was incorrectly expected to be confirmed empty. The original failure and its 48 unreached suites are preserved in the [dashboard correction evidence](verification/extraction-source-trust-20261021/ci-dashboard-correction-20261021/README.md). The corrected dashboard suite passes separately with both unchanged legacy unknowns and two explicitly declared mock complete-source controls. Its CSV/filter/inspector checks do not claim native capture evidence or a full49 result; product inputs and the normal binary remain unchanged. The exact corrected successor still needs its complete CI run.

The [bounded extraction source trust acceptance](verification/extraction-source-trust-20261021/README.md) now passes one selected native suite with 19 CSV/XLSX pairs, 712 rows and four separate native API CSV files with 265 rows. It includes actual source hashes, both saved-view modes, import controls and cold reopen. Ordinary Rust1843/ignore5, Node535, semantic40, normal build/format/Clippy and four separately executed Core renderer tests pass. Core tests execute their own test binaries; App native_enrichment_browser_fixture remains separate. A [different selected two-suite invocation](verification/extraction-source-trust-20261021/existing-views-20261021/README.md) passes existing named-view and expression workflows. No combined full49 run or exact new public CI is inferred. All prior failures remain recorded.

The [extraction source trust contract](extraction-source-trust.md) adds
`node scripts/ui-e2e.mjs --only extraction-source-trust`, an owned native path
for complete/incomplete/unavailable inputs, raw/DOM observations, CSV/XLSX and
cold reopen. Registration is not a passing run; current gates are in
[FEATURE_TODO.md](FEATURE_TODO.md). There are now 49 registered suites; earlier
46-/48-suite receipts bind their own revisions. The new pure regression runs
actual App functions through
`node --test web/verification/extraction-source-trust.test.cjs`.

The [source-availability/provenance follow-up](verification/table-xlsx-provenance-20261021/README.md) fixes missing-source false success, Boolean export representation and confirmed-empty native segment export. Fresh510 selected Node tests,40 semantic groups, format/build/strict Clippy and one complete focused20-suite invocation pass (9 fixture/11 native). Independent Python reconstruction checks43 distinct original pairs,3062 rows and65321 physical cells plus188 read-only segment POST observations. Direct R3 provenance35 is PASS inside an original failed aggregate; R4 availability8 is a separate selected PASS. NativeR1/R2/R3 failures remain unchanged. Fresh ordinary Rust tests/renderer opt-ins and a full local48 run were not performed for that follow-up. The exact `905a248` subsequently passes [all four public jobs and one unfiltered48-suite invocation](verification/table-xlsx-provenance-20261021/public-ci-905a248-20261021/README.md); this is separate from the newer49-suite extraction implementation. The609 selected inputs bind this slice; metadata/transitive dependency closure and installed pilot update are separate.

The supported story is: open the redesigned interface, configure a crawl, fetch
owned pages through the Rust backend, inspect the saved result, and export or
reopen that result. Tests must follow the real boundary they claim to cover.

## Main-table XLSX exports

The [local XLSX receipt](verification/table-xlsx-export-20261021/README.md) binds two actual saved native crawls and a separate genuine 32,768-unit title-cell refusal. Thirty XLSX and thirty CSV downloads preserve 3,020 data rows: 35,652 data cells plus 342 header cells. The title, status and response tables cover raw null versus empty/zero, literal formula/XML/Xstring text, overlapping AND/OR populations, current ordered columns, sorting and complete multi-page export. Eight real-DOM prepare races, twelve geometry observations at 1440/320 px, natural warm/cold EOF0, two positive HTTP-410 controls and zero offline product refetches are measured. An independent Python ZIP/XML/openpyxl3.1.5 audit reconstructs all 35,994 physical cells, types, order and provenance from the immutable original native reports. Its standards Xstring adapter is explicit; no Excel application roundtrip is claimed.

Fresh Rust gates cover format, normal build, 533 App passes with zero failures and one ignored native_enrichment_browser_fixture opt-in, and strict Clippy. The slice also passes 459 selected Node tests and 40 pure semantic groups. Ordinary Core tests and renderer opt-ins were not freshly run here. Focused R3 is one complete invocation of exactly 18 unique suites: nine API fixtures and nine native suites, including the new XLSX suite. Original R1 remains FAIL after sixteen successes at the old 15-versus-17 asset precondition. Original R2 remains FAIL after sixteen successes at the old 30-versus-34 aggregate served-asset expectation. R3 corrects those asset guards; functional, data and download assertions are retained. All three attempts and their raw logs remain separate.

The exact R3 manifest contains 605 explicit selected source/harness inputs, not a transitive toolchain/dependency or whole metadata closure. Compiled product inputs and the normal binary match the fresh Rust build gates; the suite registration, portability asset guards and whitespace-policy changes are qualified separately. The exact published `22fc522` subsequently passed [all four public jobs and one complete unfiltered46-suite invocation](verification/table-xlsx-export-20261021/public-ci-22fc522-20261021/README.md). A full local46 run remains NOT_RUN. This source-specific acceptance does not cover the later Boolean/source/provenance successor, whose current48-suite public CI is separate. Native acceptance across the other schemas in the 29 pure table projections, segment/selected-recrawl provenance, specialist/cursor/bulk export, large streaming workbooks, Excel/LibreOffice GUI, the full `ui.filter-details-export` workflow and vendor parity remain open. The ledger retains 144 capabilities, 416 filters, 32 workflows and 39/68/37 acceptance states.

## Portable view definition files

The [portable definition contract](table-view-portability.md) has [bounded R2 native acceptance](verification/table-view-portability-20261019/README.md): one new suite, four groups, 22 actual definition JSON files, 27 actual CSV files with 2,626 data rows and 26,260 data cells, 61 complete DOM populations, eight scripted real-file DOM race controls and 36 geometry observations at 1440/320 px. Two actual Chromium quota refusals and two real insecure-origin missing-Web-Locks refusals preserve the separate v1/v2 catalogs, working query and saved reports. Two native owners end naturally with EOF0; two positive HTTP-410 controls accompany zero product refetches. An independent offline reviewer reconstructs every CSV cell from original native reports.

R1 remains a failed 16-suite invocation: 15 other suites passed and the new portability driver failed its active-mode Clear precondition. R2 changes only that driver and freshly passes only the new suite. The 15 other successes and unchanged 397 Node tests, 40 semantic groups and 22 actual Chromium controls are explicitly reused from R1; there is no combined 16-suite PASS. Fresh R2 checks cover normal build, format, strict Clippy and 533 App tests with native_enrichment_browser_fixture ignored. The four separately executed Rust/Chromium renderer cases are Core integration tests; they do not run that App opt-in. The 1,298 ordinary Core passes with four ignores are historical R4 evidence reused for 241 identical selected inputs and five equal captured environment values, not a fresh workspace run or complete transitive toolchain proof.

The exact published portability successor `66dfe5c` passes all four public CI jobs, including all 45 registered UI suites in one unfiltered invocation ([terminal receipt](verification/table-view-portability-20261019/public-ci-66dfe5c-20261021/receipt.json)). The additional complete local 45-suite run remains NOT_RUN. Native project preferences, XLSX paths outside the separate bounded acceptance, every table/detail/cursor/export combination, arbitrary delayed reads, larger corpora, resource/crash limits, other browsers and complete manufacturer parity remain open; the full ui.filter-details-export contract remains NOT_PROVEN.

## Named complete table views

The [nested query contract](table-query-expressions.md) has [three successful prescribed native R6 repetitions](verification/table-query-readiness-20261019-r6/README.md): each includes two153-page captures,45 complete DOM observations,9 actual CSV files/927 rows and7 selected-owner data-readiness witnesses. The actual R4 full44 invocation remains FAIL with43 successful suites and a final driver readiness failure. R6 waits for the genuine final owner-refresh response and committed UI data; all original assertions remain. The exact earlier published successor `8d9d7cc` subsequently passed all 44 registered UI suites in one [public Renderer invocation](verification/table-view-portability-20261019/prior-ci8d/table-query-ci-8d-20261019-renderer-receipt.json). All four public jobs subsequently completed successfully; see the separate [terminal receipt](verification/table-view-portability-20261019/prior-ci8d-terminal/receipt.json). That result covers the earlier source. The exact portable successor `66dfe5c` now separately passes all four public jobs and its complete 45-suite invocation, as linked above.

The separate [R5 screenshot follow-up](verification/screenshot-response-harness-20261019-r5/README.md) freshly passed330 pure tests,40 fixture groups,22 actual Chromium controls and the native screenshot suite (11 original clone JSON responses and5 PNG files). R6 changes only the expression native driver and explicitly reuses these unchanged controls. R4's1831 ordinary Rust passes/5 opt-in ignores and4 explicitly executed Chromium fixtures are also reused for identical product/compiler/fixture inputs. The exact parent public CI is terminal FAIL in Renderer; its other three jobs passed.

The [named-view contract](table-views.md) has [bounded native acceptance](verification/table-named-views-20261019-r3/README.md). The registered `table-named-views` suite prepares two actual 107-page captures through the ordinary native API; it does not prove the dashboard Start button. Explicitly saved definitions include the section filter, search, active AND rules, sort and visible-column order. Selection never applies, saves do not apply, and Apply installs the complete query on the selected compatible crawl at page 0. The driver asserts page reset before its pagination helper can change pages. Active-ID Replace/Rename/Delete update only their badge state; edits to other IDs do not change the applied binding.

Five native groups cover 29 complete title-table observations, nine real CSV downloads and all 719 data/provenance rows, 26 controls measured at 1,440/320 px, actual Chromium storage quota, real Web Lock contention and separate same-tab byte-CAS/cross-tab storage-event controls. The latter requires explicit Reload and never silently applies another tab's changes. Corrupt bytes remain until a separately confirmed reset of only the named catalog; working query and both legacy column-preference keys remain. Immediate same-event-turn search/owner changes and detached controls refuse stale saves. The NoLocks case uses an actual insecure owned read-only passthrough to the loopback service; native alias-host HTTP 403 is retained as a separate negative control.

Two normal native processes end naturally after stdin EOF with exit 0. Fresh same-origin storage restores definitions without auto-Apply, unchanged saved reports reopen without product website refetches, two positive 410 witnesses remain active, and cold/320 px CSV bytes match their warm counterparts. R1's disabled-Clear driver failure and R2's insecure alias setup failure remain FAIL with original hashes; only the driver/setup changed before R3. Unsupported/oversized recovery, schema/query-semantics incompatibility, no-crawl controls and additional stale-route combinations are pure controls, not all native cases.

Fresh acceptance on the same 593 frozen inputs passed **282 state/helper tests**, **40 semantic groups**, **17 reported Chromium transport tests (two parents and 15 subcontrols)**, **1,831 ordinary workspace tests / zero failures / five opt-in ignores**, format, strict Clippy, four separately executed Rust/Chromium fixtures and **all 43 UI suites in one complete invocation** (nine API-fixture and 34 native suites). The initial R3 whole invocation passed 11 suites and then dom-capture exited as a Windows Node process with code 3221226505 (0xc0000409), no stderr and no timeout. The native coordinator closed naturally with exit 0 and sources/binary stayed unchanged. An unchanged standalone diagnostic then passed, followed by this fresh full 43-suite retry. The earlier aggregate remains FAIL and the original crash cause remains NOT_PROVEN. The fresh R3 workspace/Chromium gates are preserved on identical sources; the UI retry did not repeat those gates. The parent1e public CI is now terminal FAIL in Renderer after40 passing suites; Windows,Ubuntu and container passed. The new successor requires separate complete CI. Manufacturer parity and the full `ui.filter-details-export` workflow remain `NOT_PROVEN`; the installed Source16 pilot is unchanged.

## Main table filters and saved column views

The [table contract](table-explorer.md) has [actual bounded native acceptance](verification/table-explorer-20261018-r4/README.md). Six groups, 36 complete views and five actual CSV files with 521 data rows were checked. Two same-origin native processes and their coordinator ended naturally after stdin EOF with exit 0. Two positive HTTP-410 controls accompanied zero product refetches. Native API-prepared captures of 107 pages and one page preserve unknown/empty/zero states, Unicode matching, stable sort, response occurrence inspectors and exact saved report bytes. Explicit per-crawl/section browser preferences include genuine quota, corruption and stale-control tests at 320 and 1,440 pixels. Fresh R4 gates passed **all 42 registered UI suites in one complete invocation**, **201 state tests**, 40 semantic groups, 17 reported Chromium tests with two parents and 15 subcontrols, and four targeted native suites. Four explicit Rust/Chromium fixtures also passed separately on R4. The **R3 workspace result of 1,831 passed, zero failed and five ignored tests**, format and strict Clippy are explicitly reused against identical product/native sources and binary. No new R4 workspace execution is claimed. R1/R2 failures and the R3 aggregate failure after 35 passing suites remain recorded. This does not establish portable preference storage or finish the OR, XLSX, specialist, large-corpus and reference scope in `ui.filter-details-export`.

## Current profile response-capture acceptance

The [verification-only follow-up](verification/profile-browser-capture-20261018/README.md) records **180 state/helper tests**, **12 actual Chromium tests** and **three affected native suites in one complete invocation** (10 profile,4 catalog and3 screenshot groups). Actual browser clone bytes are bound to the exact request/document; no HTTP replay or fallback is used. Original fetch Promise/Response identity, pure-read allowlists, opaque-document transitions, clone-only budgets/timeouts and Windows Node-SQLite fixture paths are covered. Existing mutation response capture and all product assertions/deadlines remain. Three prior local failures are retained. The exact public CI run for 25b, [39164257731](https://github.com/lia-xim/open-seo-spider/actions/runs/39164257731), is terminal failure. Windows, Ubuntu and container succeeded. Renderer failed at the first catalog Create A POST with unavailable CDP response bytes. Read-only SQLite association does not prove its exact wire status or root cause. The explicit owned mutation-response capture policy is now applied only in test harnesses. Fresh R4 controls passed 201 state cases, 40 semantic groups, 17 reported Chromium tests with two parents and 15 subcontrols, four targeted native workflows and all 42 UI suites in one complete invocation. Default capture remains limited to pure reads. Each opted-in owned operation captures the original response clone without replay; existing assertions and budgets remain. Only six test files changed. Product code and binary are unchanged. The R3 whole UI attempt remains failed after 35 passing suites because its custom-search oracle expected the old identity-tie order. The following 41-suite result remains historical. The exact published R4 commit 2281086 subsequently passed all four public CI jobs in [run 39172919251](verification/table-explorer-public-ci-20261019/README.md). That terminal result applies to the earlier published source; new named-view sources need their own public CI.

## Current screenshot and complete regression acceptance

The [current screenshot receipt](verification/rendered-screenshots-20261018/README.md) records **1,831 ordinary Rust tests**, five deliberate opt-in ignores, strict Clippy, four explicit Chromium cases, **160 state tests**,40 semantic UI groups and **all41 registered UI suites in one complete invocation**. The selected-viewport workflow includes actual320 ×240/1440 ×900 pixel checks, five byte-identical PNG downloads, guarded lazy preview and a two-process offline cold reopen. The full144-capability/416-filter/32-workflow ledger remains intact; manufacturer parity is `NOT_PROVEN`.

## Prior named-profile acceptance

The [named-profile receipt](verification/configuration-profile-catalog-20261018/README.md) records **1,817 ordinary Rust tests**, five deliberate opt-in ignores, strict Clippy, four explicitly executed Chromium fixtures, controlled-provider cold replay, 130 state tests and 40 semantic UI groups.

All **40 registered UI suites passed in one complete invocation** on the same frozen product and driver bytes: nine API-fixture suites and 31 native suites. The named native catalog has four actual browser groups, two positive native processes and two separately prepared negative-only processes, five real downloads and a fresh-origin cold reload. Existing portable-profile acceptance also passed its ten browser groups.

The prior [39-suite aggregate failure](verification/configuration-profiles-20261018/README.md) remains historical. Its Space step was reproduced with correctly focused, unblocked key events but no native activation click. The canonical test now waits for the preceding trusted native toggle to finish before sending Space. Three prespecified matching-build runs and the whole 40-suite run passed without raising deadlines or removing assertions. The exact Chromium root cause and universal stability are not proven.

The actual CLI check is `node scripts/crawl-config-cli-e2e.mjs`, with `SPIDER_BINARY` explicitly set to the built executable. It proves raw native `--config` options and portable-envelope refusal before HTTP/output/store effects. Portable profile CLI execution remains unsupported. See [the format contract](configuration-profiles.md) and [the complete backlog](FEATURE_TODO.md).

## Run locally

Use the project's Rust toolchain and Node.js 24. Install renderer dependencies
with `npm ci --ignore-scripts` in `renderer/`. Install Chrome/Chromium separately
and set `OPEN_SEO_BROWSER_PATH` to its executable for the complete suite. The
named-catalog driver requires this explicit browser selection.

```sh
cargo build --locked -p open-seo-spider
node scripts/ui-e2e.mjs
node scripts/renderer-rust-e2e.mjs
```

`SPIDER_BINARY` selects an explicitly built app. The coordinator computes and
passes its SHA-256 to each child; a direct catalog-driver invocation must also
supply `SPIDER_BINARY_SHA256`. The runner never installs or
updates the user's application. It launches a fresh app and renderer on owned
dynamic loopback ports, uses a new test store, and closes its app through stdin
EOF. It refuses the installed application's ports 8787/8790. Runtime evidence
goes to an ignored `artifacts/ui-e2e-*` directory; `SPIDER_E2E_OUTPUT` may select
another **new** output directory. Each suite gets a separate artifact directory.
Use a fresh output path in deeply nested Windows checkouts. The separate
[Windows store-path receipt](verification/windows-store-paths-20261018/README.md)
records the fixed ordinary SQLite path cases through 1,150 UTF-16 units and
Unicode paths; network UNC and broader namespace portability remain unproven.
Tracked files are hashed before/after execution; a changed file fails the run.
The application child does not inherit `OPEN_SEO_PAGESPEED_API_KEY`. The API
enrichment suite exercises preview and refusal paths without sending targets
to Google; it does not establish successful provider measurements.

The separate positive enrichment check builds a **test executable**, then owns
its full native router, a local HTTP provider, a website-request trap and two
native processes using the same fresh store:

```sh
node --test web/verification/api-enrichment-state.test.cjs
node scripts/run-enrichment-browser-checks.mjs
```

It compiles the app test target through Cargo's JSON artifact output and binds
the exact executable, Rust/manifests/web/assets inputs and test drivers by SHA-256.
The immutable `cfg(test)` transport sends actual HTTP to the owned provider
after the ordinary start guards. The normal production provider URL and
credential handling are unchanged. No job or saved result is seeded. The child
processes cannot inherit a PageSpeed key; only isolated loopback ports are used.
`SPIDER_ENRICHMENT_ARTIFACT_DIR` may select a **new** artifact directory; reuse is
refused. Default output is `artifacts/enrichment-success-*`. This command runs
separately from the UI runner and is not an additional registered suite.

For a focused investigation:

```sh
node scripts/ui-e2e.mjs --only tool-navigation,saved-spelling
```

The default run includes every registered suite. `--only` does not constitute a
complete E2E pass. The runner stops at the first failure and retains its logs.

The second command executes the four Rust fixtures explicitly ignored by the
ordinary workspace run because they need Chromium. It owns a dynamic renderer
port and closes the worker afterward; no installed renderer is required. CI's
renderer job runs the same command. Report ordinary ignored tests and these
explicit executions separately.

## Coverage and boundaries

| User path | Registered suite | Evidence type |
| --- | --- | --- |
| Dashboard, settings, tables, filters, downloads, responsive controls | dashboard | Real browser with explicit API fixtures |
| Pause/resume, selected recrawl, policy, content and resource edge states | pause-controls, recrawl-controls, policy-controls, content-controls, resource-controls, resource-ledger, resource-contract | Real browser with explicit API fixtures |
| Saved comparison errors and optional bundle controls | comparison-controls | Real browser with explicit API fixtures |
| All nine redesigned tool-menu links and return navigation, desktop/320px | tool-navigation | Real browser and Rust HTML routes; navigation only |
| JavaScript crawl, screenshots, accessibility and DOM capture | rendered-crawl, dom-capture | Browser → Rust → owned renderer → owned HTTP site |
| Saved accessibility review, unknown/zero/needs-review states, inspector and full CSV | accessibility-review | Two normal native processes → pinned axe/Chromium on owned HTML; 59 captures, 58 visible pages, declared import controls separate |
| Resource and cookie observations | resources | Browser → Rust → owned fixture |
| Offline robots and SERP editing/exports | robots, serp | Browser → Rust; website-request witnesses |
| URL-list import and crawl | list-import | Browser → Rust → owned fixture |
| Named complete main-table definitions, explicit CRUD/reset, exact conflict refusals and same-origin cold reopen | table-named-views | Two ordinary native API-prepared captures; all selected title-table/CSV/provenance cells, genuine Web Locks/quota/CAS/storage events, negative insecure NoLocks passthrough, explicit first-page Apply and active badges; bounded receipt above |
| Nested main-table AND/OR, explicit mode conversion and separate v2 grouped-view catalog | table-expressions | Three prescribed native R6 invocations PASS: each2x153 pages,45 views,9 CSV/927 rows,7 owner-readiness witnesses, conflict guards and same-origin cold reopen; fresh full44/public CI pending |
| Portable v1/v2 definition files, complete exports, conflict preview and guarded import | table-view-portability | Implementation in progress; actual downloads, file-picker/lock/CAS races, untouched queries/reports and cold reopen await fresh acceptance |
| Full native configuration profiles, exact integer/byte preservation, volatile headers and cold reopen | configuration-profiles | Registered owned-native browser/file-picker/download driver; current source acceptance is recorded separately |
| Named native profile catalog, CRUD, two-tab CAS, corruption refusal and fresh-origin cold reload | configuration-profile-catalog | Actual browser → native API → SQLite → download, two positive and two negative-only EOF0 processes; bounded local receipt above |
| Drain/pause/resume and selected recrawl | pause-resume, selected-recrawl | Browser → Rust → owned HTTP barriers/persistent crawl |
| Policy preview and crawl decisions | policy | Browser → Rust → owned fixture |
| Saved raw/rendered text and content analysis | content | Browser → native saved content |
| Discovery, occurrence/transport data and persisted resources | resource-discovery, resource-persistence | Browser → Rust → owned fixture |
| Saved spelling, explicit transfer acknowledgement, dictionary/profile reload, CSV and provider failure | saved-spelling | Browser → Rust → deterministic local LanguageTool-compatible fixture; not real-engine quality |
| Saved comparison, original-report bundles and cold restart | saved-comparison, comparison-bundles | Owned native processes, browser, saved data and offline HTTP witnesses |
| Migration CSV mapping, redirect outcomes and retained broken-link sources | link-workflows | Browser → native saved report; 109 mappings, 107 link occurrences, pagination and complete verified exports |
| Directory hierarchy, raw/reported views and unknown/imported data | directory-tree | Browser → native saved report; exact rendered cards, 100+5 paging, full CSV and stale tokens |
| Segment builder, browser-local rules, assignments and issue views | saved-segments | Browser → native saved text; exact membership/cells, 100+5 paging, JSON/CSV and stale rules/views |
| Applied segments across the main dashboard, charts, full native/section exports and cold reopen | dashboard-segments | Two normal native processes and 1,010 genuinely captured pages; all 1,005 selected ordered titles, original ordinal controls, snapshot-bound original records, global/selected scopes and complete exports |
| Compact mobile segment cells with full expandable values | segments-density | Normal native process and 105 genuinely captured pages; 320px closed-row bounds, keyboard/touch/selection, full cells and identical expanded/collapsed exports |
| Repeated metadata with over a million complete issue relationships | segments-repeated-metadata | 1,010 genuinely captured pages; all 7,035 selected issue rows and ordered relationships, full Issues CSV/JSON, actual standalone API/CLI/MCP and two-process cold reopen |
| N-gram totals, occurrences, word cloud and text/metadata channels | saved-ngrams | Browser → native corpus; exact rendered rows, complete CSV, missing sources and real stale-generation refusal |
| N-gram and segment analysis after native restart | ngrams-cold-restart, segments-cold-restart | Two owned native processes per suite; exact saved sources and complete exports, active HTTP410 witness through both closes; browser state boundaries described below |
| Resource/CSS failure occurrences and unavailable history | resource-sources | Browser → native SQLite ledger; 106 occurrences, 100+6 paging, snapshot-bound CSV and page JSON |
| API enrichment preview, acknowledgement, invalidation and error messages | api-enrichment | Browser → ordinary native API; no successful provider calls or persisted remote metrics claimed |
| API enrichment start/poll, exact metrics and unknowns, 100+8 rows, refresh, cancellation, full export and cold reopen | separate enrichment coordinator | Browser → full native test router/service → owned HTTP provider → saved data; production Google transport and successful enrichment CLI remain unproven |
| Custom literal/regex search, separate raw/rendered/selected sources, exact counts, unknowns and limits | custom-search | Browser → Rust → actual Chromium → owned website; saved inspector, complete CSV/JSON, import and owned cold native restart |

The existing native sitemap membership/assessment, API, CLI, MCP and renderer
integration suites remain separate CI gates. Extracted-function VM tests are
also separate; they must not be described as browser E2E tests.

## Verified slices and remaining important paths

The earlier [dashboard-segment receipt](verification/dashboard-segments-20261018/README.md)
records all 37 registered suites passing in three separate phases (2+1+34) on
the same final normal executable and 485 unchanged source/driver inputs. This
is not a single shared-store 37-suite invocation. The large dashboard suite
had a measured-workflow budget of 900 seconds; its other 36 suites retained 240
seconds. Its positive fixture contains 1,010 real pages with unique descriptions.
The separate repeated-description case was refused before paging by the old
32 MiB projection-copy budget. That historical failure is preserved.

The earlier [large-issue receipt](verification/large-segment-issues-20261018/README.md)
records successful executions of all 38 registered suites on the same normal
executable and 486 unchanged source/driver inputs. These are separate phases:
14 successful cases before a Node-process crash in Robots, 22 complete cases,
one plain Robots acceptance after bounded diagnosis, and the new large case.
The original failed batch is not relabelled successful. The new case completes
in 19m36s with a case-specific 1,800-second budget; the existing dashboard case
retains 900 seconds and the other 36 retain 240 seconds.

Indexed original-issue selection now allows the repeated-description cohort to
page fully and export complete 45,193,762-byte JSON and 44,429,381-byte issue CSV.
Warm/cold files are byte-identical. Actual standalone API and CLI full exports
still refuse at 32 MiB, and the MCP bridge retains its 1 MiB response bound.
Smaller MCP previews retain every original related URL. Six intermediate
checkpoints are explicitly `IN_PROGRESS`; only the final receipt establishes
whole-case acceptance. General performance/RSS, long Windows stores and the
isolated Node crash's root cause remain unproven. Fresh public CI is separate.

The 40 registered suites provide bounded functional cases for the specialist
tools above; a passing case is not complete feature acceptance. Cold native
restart across other tools, all rule/operator combinations, broader
malformed imported archives and concurrent mutations need further coverage.
The controlled positive API-enrichment path is now proved by the
[2026-10-18 receipt](verification/api-enrichment-browser-20261018/README.md):
105 unique URLs and 108 supplied rows, exact first-100/tail-8 DOM contents,
five full CSV downloads, refresh as a new job preserving the original four
saved files and CSV, an aborted held provider request, and process-cold reopening
with no repeat provider or website requests. The refresh case specifically
reopens the saved job before a new preview and renewed acknowledgement; it does
not prove universal freshness enforcement on every start. Eight deferred-response regressions
separately cover UI ownership, stale callbacks and invalid snapshot controls.
These VM checks are not browser/native-provider evidence. Authorized production
Google metrics, successful `enrich-urls` CLI, other providers and broader tool
acceptance remain open.
Real LanguageTool language quality, other external providers, future features
and broad cross-browser parity remain separate requirements in
`FEATURE_TODO.md`.

Generated logs/screenshots/results are deliberately not committed over the
historical fixtures. GitHub Actions uploads the new run's evidence separately.

The final local gates for the enrichment slice pass: 1,749 ordinary Rust
tests with zero failures/five deliberate ignores, four separately executed
Chromium fixtures, all 32 UI suites (nine fixture/23 native; natural app exit 0),
strict Clippy and the normal build. The 28 pure bundle controls, 11 native bundle
groups and 12 official SDK checks pass on the normal executable. See the
enrichment receipt for exact source and executable bindings. Those 32-suite
results are historical; the three new suites have separate acceptance.

The segment restart test retains one browser context and the same owned app
origin across two actual native processes. Browser-local ordered rules are
restored separately from native saved crawl/source data. It does not establish
server persistence of rules, a fresh browser profile, crash recovery or every
operator. The n-gram tool has no persisted analysis options: the test reapplies
them after document reload and checks original saved captures and metadata.
Both suites keep the original website active as an HTTP410 witness from before
the first shutdown through the second shutdown and browser close. Nonce-tagged
positive probes are separate from the zero product requests.

The accessibility journey runs the pinned local axe engine on owned HTML,
including a genuine incomplete color-contrast result and an actual engine
failure. Missing, failed and malformed observations retain unknown counters;
empty valid findings mean observed zero. Complete CSV has per-page summaries,
every rule/node/check and exact saved observation JSON; a filtered or paged
table never reduces that export. A 4 MiB selected-page JSON budget and 64 MiB full
CSV budget refuse oversized output atomically. Native tests separately exercise
physical64MiB exact/+1-byte responses; exact4MiB success is not separately
proved. Imported malformed/false-scope controls are labelled separately from
native observations. Automated-only results do not establish manual review or
WCAG conformity.

The historical public run on `01ba979` is
[39120704481](https://github.com/lia-xim/open-seo-spider/actions/runs/39120704481).
It remains a historical FAIL: two Windows timeouts, an obsolete Linux inventory
expectation and an unavailable directory debugger response body despite
immediate capture. The directory test now uses an unconditional independent
native POST artifact oracle together with browser metadata and exact DOM checks.
Both previously failing Windows tests pass locally and CI concurrency was
aligned, but the exact historical five-second drain-timeout cause remains
NOT_PROVEN. The next public CI run is pending; local green does not prove that
public Windows is fixed. Run
[39123847418 on081bf33](https://github.com/lia-xim/open-seo-spider/actions/runs/39123847418)
subsequently passed Linux, container, the controlled positive provider gate and
directory E2E, then failed at an unavailable n-gram CDP response body. Its
Windows result remains separately tracked. Warm n-gram and segment tests now
use unconditional read-only native response oracles with exact browser metadata,
DOM and download comparisons; mutation requests are never replayed. Fresh CI
is required for the current accessibility and cold-analysis changes.

The current accessibility/cold-analysis local receipt records 1,753 ordinary
Rust passes, zero failures/five deliberate ignores, strict Clippy and the normal
build. All 35 UI suites passed together; the two cold assertion drivers were
then strengthened and passed again separately on the same executable. See
[the phase-bound receipt](verification/accessibility-cold-browser-20261018/README.md).
These local gates do not establish public CI, release/installation or full
manufacturer parity.

## Requested Windows store paths

`scripts/windows-store-paths-e2e.cjs` is a separate Windows native/browser gate.
It requires an explicitly pinned normal executable (`SPIDER_BINARY` and
`SPIDER_BINARY_SHA256`), an installed matching Chromium
(`OPEN_SEO_BROWSER_PATH`) and a fresh explicit `SPIDER_WINDOWS_PATH_OUTPUT`.
Choose that output short enough to construct the 214 UTF-16 baseline; the
driver refuses unsuitable locations instead of relocating test stores.

Five actual captures use SQLite paths of 214, 255, 300 and 1,150 UTF-16 units,
plus a 650-unit Unicode path independently exceeding 1,040 UTF-8 bytes. Each
case captures two pages and a sitemap, validates every native CSV column and
complete JSON, downloads the original archived XML and actual browser exports,
then closes and reopens the same store in a second native process. The owned
website returns HTTP410 before the first close through both closes, with
nonce-protected witness probes and zero permitted product refetches.

Standalone read-only CLI controls compare complete saved rows through ordinary,
verbatim, relative/mixed/dot and owned directory-junction spellings. Trailing
dot/space aliases are tested separately at the short baseline. Missing and
invalid read-only inputs must fail without creating outputs, database siblings
or archive directories. Original file bytes and identities are checked.

The junction control covers SQLite/raw rows only. Original archive traversal
continues to reject symlinks/junctions; this gate makes no UNC-share, ambiguous
long-alias, system-setting, crash-durability or full manufacturer-parity claim.
The Windows matrix runs this gate and uploads its raw evidence. Local execution
results and source/build binding are recorded separately from public CI.
