# End-to-end checks for the redesigned application

The supported story is: open the redesigned interface, configure a crawl, fetch
owned pages through the Rust backend, inspect the saved result, and export or
reopen that result. Tests must follow the real boundary they claim to cover.

## Current named-profile and regression acceptance

The [current receipt](verification/configuration-profile-catalog-20261018/README.md) records **1,817 ordinary Rust tests**, five deliberate opt-in ignores, strict Clippy, four explicitly executed Chromium fixtures, controlled-provider cold replay, 130 state tests and 40 semantic UI groups.

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
