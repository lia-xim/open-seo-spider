# Complete large segment issue views

Local acceptance passes on normal development executable
`b37fedf446d887ddd803a2fd5a21bc636b7d3e1274c6afa93ef4f9ce8e6bd3a8`.
The installed Source16 application and user crawl store remain unchanged.
Fresh public CI for this change is a separate gate.

[local-result.json](local-result.json) records the executed gates and hashes of
locally retained raw evidence. [source-gates.json](source-gates.json) binds 486
source/driver inputs before and after the final browser phases, plus supplemental
harnesses. Product inputs match the pre-compilation freeze. The authorized test
driver/checkpoint and workflow-timeout edits after compilation are explicit;
they do not imply a rebuild of product code or a clean-HEAD compilation.

## Resulting behavior

The shared segment evaluator selects original issue ordinals. App previews and
full exports borrow those original issues rather than copying every related URL
before paging. Original order, duplicates, issue ownership, complete relationships,
profiles, target facts and snapshot/generation checks remain intact. The public
owned Core adapter retains its copy budget. Existing issue deduplication borrows
owner/code/relationship data and preserves exact-set and empty-wildcard semantics.

The dashboard offers all 13 native CSV kinds. Native **Issues** CSV contains every
original related URL and 13 cohort-context columns, for 18 columns in total. The
generic table CSV remains a summary with related-URL counts. Choosing an export
kind preserves the already confirmed cohort.

The new genuine crawl captures 1,010 owned pages with one shared description and
distinct titles/document text. Its selected 1,005 pages contain 7,035 issues and
1,012,035 related-URL occurrences. This is not a declared import or an arbitrary
identical-HTML corpus.

The actual browser validates every selected title and all issue-summary cells
across 11 and 71 UI pages respectively. It verifies all 200-row native cursors,
full JSON and every native Issues CSV cell, including ordered original URL
strings. A late original ordinal exposes all 1,007 related links in the inspector.
Four negative controls exercise the actual validators against missing,
substituted, reordered and altered-context evidence.

Two native server processes reopen the same store and origin. After the first
capture, the fixture returns HTTP410 through both natural EOF0 shutdowns and
browser closure. There are zero product refetches and two successful nonce
control probes. Full warm/cold JSON is byte-identical at **45,193,762 bytes**;
full Issues CSV is byte-identical at **44,429,381 bytes**. Original saved report
bytes remain unchanged. A separately labelled duplicate-URL import proves the
selected original ordinal rather than attributing that control to a real crawl.

## Executed gates

| Gate | Actual result |
| --- | --- |
| Ordinary Rust workspace | 1,772 passed, zero failed, five deliberate ignores |
| Explicit Chromium fixtures | All four named ignored browser fixtures actually executed |
| Positive enrichment coordinator | Actual test executable, owned HTTP provider, refresh/cancellation/export and two-process cold reopen; production providers remain separate |
| Focused Core / App / analysis | 22 / 26 / 32 passed; these overlap the workspace total |
| Frontend state / semantic VM | 71 / 40 passed; these are not browser E2Es |
| Format / strict Clippy / normal build | Passed with unchanged product inputs |
| Registered UI cases | All 38 have successful executions: nine API fixtures and 29 native/live cases, in the separate phases below |
| New repeated-metadata case | Whole-case PASS in 1,176,159ms; two native servers, actual CLI/MCP consumers, full exports, cold restart, 320px/1440px reachability and zero runtime/external-request errors |

The 38 cases comprise 14 successful cases in a batch that subsequently failed
in Robots, a complete 22-case batch, one plain isolated Robots acceptance, and
the new large case. This is not one successful shared-store 38-case invocation.
The failed batch remains failed. A diagnostic Robots run also passed, but is
not counted as an additional registered suite.

The standalone API checks all 42 page/issue cursors and explicitly refuses its
complete JSON/CSV at the unchanged 32 MiB bound. Actual normal CLI commands read
the genuine saved report: tail JSON/CSV retain all relationships, while two
literal-All exports exit 1 without output or temporary files. The two successful
CLI commands exit 0. The actual SDK2.3.1 MCP stdio bridge returns five exact native
previews and refuses a 7,888,848-byte 200-duplicate-issue response at its unchanged
1 MiB bound. Its smaller eight-issue duplicate preview retains all 1,007 URLs
per issue. The bridge closes through stdin EOF with code 0. Three actual 409
responses reject stale token, generation and cross-job export contexts.

## Limits and retained failures

Input/report, per-page, standalone, full-dashboard and MCP limits remain distinct.
Paged responses and standalone exports retain 32 MiB; full dashboard exports
retain 64 MiB; MCP retains 1 MiB. The legacy owned projection still has its
32 MiB copy budget. No data-size limit was raised, and no URL evidence
was truncated. These are logical/work/output bounds, not universal RSS or CPU
guarantees. The development test's extra native-oracle requests also prevent
treating its duration as a normal application benchmark.

The previous 45 MB repeated-description refusal is preserved. Current attempts
also retain: an early SQLite initialization failure with a 255-character Windows
database path, an analysis-event timeout after all 1,005 pages and 4,600 issues,
and a 900-second aggregate timeout after warm/API/MCP progress. A shorter owned
path is a test workaround; Windows long-store support remains open. The event
window is 420 seconds. Measured warm verification alone takes about eight minutes,
so only this new aggregate case has a 1,800-second bound. The earlier dashboard
case retains 900 seconds and the other 36 cases retain 240 seconds. Six durable
intermediate receipts say `IN_PROGRESS`; they never substitute for final PASS.

The first Robots child terminated natively with `0xC0000409` between its fifth
and sixth blocks, without a JavaScript error or timeout. One bounded diagnostic
run and one subsequent plain run both passed all eight blocks on unchanged
source and runtime pins. The initial failure is retained; its root cause and
general stability remain unproven.

Public predecessor run [39132410328](https://github.com/lia-xim/open-seo-spider/actions/runs/39132410328)
is terminal **FAIL**: Windows, Ubuntu and container jobs passed; the renderer
completed all 37 prior UI cases before a separate sitemap harness blocked its own
new `dashboard-segments.js`. Both sitemap allowlists now include exactly that
asset and its served hash. Their current normal-binary local runs pass 11 and
13 groups, preserving safety assertions. Those older sitemap harnesses use their
existing SIGTERM lifecycle; they are not relabelled EOF0. Fresh public execution
of these fixes and the new large case remains separate.

The catalog remains **38 own acceptances / 69 partial / 37 planned** across
144 grouped capabilities, 416 named-filter contracts and 32 reference workflows.
All manufacturer parity is **NOT_PROVEN**. Other operators/combinations,
integrations, fresh browser profiles, server-side rule persistence, crash
recovery, long Windows stores and large-corpus performance need further work.
