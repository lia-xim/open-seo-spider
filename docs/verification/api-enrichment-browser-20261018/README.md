# API enrichment: controlled native browser acceptance

The focused enrichment path passes on the freshly compiled **test executable**
SHA-256 `5c907bf2eb21c1b582f3aa9180692ec40f265421bf970a1bc8629690ffdb31e4`.
This is a full native router/service/parser/storage run with an immutable
`cfg(test)` transport to an owned local HTTP provider. Successful Google
PageSpeed measurements, enrichment CLI success and complete provider/feature
parity remain unproven. `modes.apis` remains `partial` and manufacturer
parity remains **NOT_PROVEN**.

The final profile-debug0 source-bound run is
`artifacts/enrichment-gates-1792322715916/provider/`. Its coordinator receipt binds
356 Rust/manifests/web/assets files and three test-driver files, verifies that
they remain unchanged, and obtains the exact native test executable from Cargo
JSON output. The normal start guards, routing, result parser and persistence
are reused; no API success is mocked in the browser and no job/report is seeded.
The normal Google endpoint is unchanged. Inherited PageSpeed keys are removed,
and the installed ports 8787/8790 and user store are not used.

- Eight deferred-response VM regressions pass against the actual frontend
  script. They separately check terminal cancellation, stale status/cancel
  callbacks, busy-state ownership and invalidation after results/CSV refusal.
  These are UI ordering tests, not native/provider evidence.
- Five real browser workflow groups pass: start/poll/results; explicit refresh;
  stale snapshots/cross-job continuation; active cancellation; process-cold
  reopening. The first job has 105 unique URLs, two extra duplicate occurrences
  and one rejected row: 108 supplied rows. Every displayed cell is checked on
  the first 100 rows and the final eight, including zero versus missing metrics.
- The provider handles 213 actual HTTP attempts: 106 initial, 106 explicit
  refresh and one held cancellation request. Fixtures exercise a 429 retry,
  authentication and Lighthouse runtime failures, missing metrics and exact
  stored metrics. There are zero website requests and zero page errors.
- Five complete browser CSV downloads match independent responses from the
  exact native endpoint and request, including content length, SHA-256 and
  `no-store`. All ordered CSV cells match the saved rows, including duplicates
  and rejected input. Three original-job downloads have identical bytes.
- The tested refresh flow reopens the original saved job, observes unchecked
  acknowledgement, creates a new preview, checks acknowledgement again and
  starts a new job with different fixture metrics. The original `input.bin`,
  `plan.json`, `report.json`, `completion.json`, rows, generation and full CSV
  remain unchanged. This proves that flow, not universal freshness enforcement:
  the UI retains preview/acknowledgement after a start, and an input change clears
  the preview. The start API requires `acknowledge_remote_fetch: true` per POST;
  it does not require a fresh consent token.
- The active cancellation disconnects the held provider request. The remaining
  URL is saved as cancelled with zero attempts and unknown metrics.
- Two owned native processes close naturally through stdin EOF with exit 0.
  The second process uses the same store and exact executable, with a deny-only
  provider endpoint. A new browser reopens the exact original configuration,
  generation, 100+8 rows and CSV with zero repeat provider calls and an unchecked
  acknowledgement. The stored job is never automatically resumed.
- Populated 1440px and 320px layouts have no document overflow; the export
  control remains reachable. Ignored screenshot paths are
  `artifacts/enrichment-gates-1792322715916/provider/enrichment-success-1440.png` and
  `artifacts/enrichment-gates-1792322715916/provider/enrichment-success-320.png`.

Reproduce with Chromium installed and renderer dependencies available:

```sh
node --test web/verification/api-enrichment-state.test.cjs
node scripts/run-enrichment-browser-checks.mjs
```

See [local results](local-result.json), [source and artifact hashes](source-gates.json)
and [the E2E guide](../../e2e-testing.md). Generated raw logs, native stores and
screenshots remain ignored local artifacts; this directory contains a bounded
public summary and hash bindings.

The final local aggregate gates also pass: 1,749 ordinary workspace Rust tests,
zero failures and five deliberately ignored tests. Four ignored Chromium
fixtures execute separately and pass; the fifth ignored test is the coordinator's
native host. All 32 UI suites pass together (nine API-fixture suites and 23 native
suites), with natural app exit 0 and unchanged tracked sources/binary. Strict
all-target/all-feature Clippy, format and the normal build pass. The normal
executable SHA-256 is `10596b1c74e35410eeaa0b1b3539fbb869b41341ff92408c9899b0a77a63628f`.
Separately, 28 pure bundle controls pass; the normal binary passes 11 native
bundle groups and 12 official SDK checks. The test-only provider
selector is absent from that normal binary and present in the test executable
as a positive control. These ordinary-binary gates do not turn the controlled
provider measurements into real Google observations.

The public CI run
[39120704481 on 01ba979](https://github.com/lia-xim/open-seo-spider/actions/runs/39120704481)
remains a historical FAIL: two Windows timeouts, an obsolete Linux inventory
expectation and a directory debugger response body that was unavailable even
with immediate capture. The actual native directory artifact and DOM were
correct; the browser test now unconditionally compares the exact independent
native POST artifact with browser response metadata and displayed content.
Both previously failing Windows tests pass within the local 1,749-test run,
and the CI concurrency budget was aligned. The exact cause of the historical
five-second drain timeout remains **NOT_PROVEN**. Fresh public CI is pending;
local passes do not establish that public Windows is fixed.

Actual Google success, successful enrichment CLI, other providers, process-cold
reopen of native saved crawl/source data for segments and n-grams, separate
same-origin restoration of browser-local segment rules, accessibility UI journeys retaining
incomplete/needs-review observations, additional specialist cases and full
reference parity remain open. Server persistence of segment rules is not
claimed. No release or user installation occurred.
