# Actual findings browser acceptance — authored, not executed

This immutable handoff is a separate future native runtime harness. It does not edit product source, replace the thirteen mocked UI groups, increase Rust test counts, release compute HOLD, build software, install anything, publish a website or touch services 8787/8790.

Source basis: frozen 297-input findings-surfaces source manifest `69492d0c0cc56bab5de86251045459a1707deb4c991d5871f9ad39015e897751` (current Root reports f43). Required future receipts are the actual `sitemap-findings-surfaces-suite-v1/proof.json` (1170+ passed, zero failed/ignored, all 388 required names and all four renderer gates) and selected `sitemap-findings-surfaces-{debug|optimized}-build-v2/proof.json`. Preparation/publication JSON is not accepted as a suite/build receipt. Actual logs, close events, SHA and exact commands are checked, as is the candidate executable in the build-v2 directory.

## Files and future invocation

`preflight.mjs` admits a Root release and actual source/suite/build/binary/Chrome identity before any mkdir, runtime import, SQLite open, listener or child. `browser-flow.mjs` exports `runNativeFindingsBrowser(env)`; it uses actual DOM controls, real HTTP and real downloads. `run-sitemap-findings-browser-native-v1.mjs` is the owned lifecycle/fixture/SQLite driver. These runtime artifacts stay outside the 297 compiler inputs.

Future invocation, only after Root explicitly releases compute:

```
node ABS_HANDOFF/run-sitemap-findings-browser-native-v1.mjs --build-mode debug --release ABS_ROOT_RELEASE.json
```

The environment must contain `OPEN_SEO_COORDINATED_EXECUTION_WINDOW=confirmed`. Required Root release properties:

- `kind`: `root_released_sitemap_findings_browser_native_v1`
- `build_mode`: `debug` or `optimized`, matching the CLI
- `source_manifest_sha256`: the fixed 69492d0c...751 manifest above
- `driver_sha256`: actual final driver SHA from this handoff manifest
- `preparation_manifest_sha256`: actual final handoff manifest SHA
- `build_proof_sha256`: SHA of the actual successful selected build-v2 proof
- `candidate_binary_sha256`: actual build-v2 executable SHA, equal to build proof
- `chrome_executable`: absolute real Chrome executable path
- `chrome_sha256`: actual SHA of that executable
- `output`: exactly `E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/sitemap-findings-browser-native-{mode}-v1`, which must not exist
- `expires_at`: a future parseable timestamp

No empty/placeholder SHA is accepted. There is no runnable example release containing invented build/binary hashes. Node >=22.16 is required for readonly built-in SQLite. The existing Playwright Core 1.64.0 package/entry are SHA-pinned; this does not attest its entire transitive dependency tree. No installation or dependency mutation occurs. Chrome is spawned directly with a new profile before CDP connect, so its actual ChildProcess is retained even if startup/connect fails.

## Actual future fixture and checks

The website serves exactly 405 distinct tiny HTML pages from explicit seeds plus two genuine XML URL sets. Explicit config: max_pages1000, max_sitemap_documents10, concurrency1, delay0, request timeout10s, page body1MiB; crawl_links/discover_sitemaps/obey_robots/check_external false. Both XML roots list p0 and p1. Page p0 has genuine noindex; p2 has a genuine anchor to p1. Links are parsed although crawl_links is false. No sitemap row, measurement, page, report, token or finding is inserted/restored/mocked by the harness.

The public App crawl must complete with405 pages, zero failed/inflight/pending and exactly407 fixture requests. Readonly node:sqlite independently verifies actual page/Document/Entry/Loc/measurement/exact-association counts405/2/4/4/2/2, pending0, actual saved config byteSHA and generation1. The real App findings reader then independently validates its existing Store/session guards; the fixture SQL check is not a replacement for those guards.

Twenty planned native groups are authored, zero executed:

1. Actual public crawl and readonly SQLite capture.
2–8. Each of seven filters: real API full traversal compared to actual browser one-row response/inspector.
9. Genuine listed/orphan/noindex/multiple-source positive and negative controls; both source-size filters are negative controls only.
10. One-row Next/Back returns the exact preceding stored row.
11. Exact original identity from a real report position beyond400 is shown in UI.
12. Real empty sparse candidate-limit1 page with Next/Back remains distinct from exhausted empty selection.
13–15. Match/not_matched/unknown selectors use actual responses; the unknown selector here is an evaluated empty result, not a positive Unknown-evidence fixture.
16. Actual full saved-selection403-data-row CSV download versus visible1row, independently parsed complete CRLF CSV and byte-identical actual API body, declared length/EOF/SHA/context/selection headers.
17. Actual65,537-byte selector validation400 disables export and creates no new download.
18–19. Desktop1440 and narrow320 horizontal overflow, footer access and keyboard inspector/focus return, with real screenshots.
20. Zero additional fixture website HTTP and unchanged durable saved files throughout API/UI/export reads.

Actual raw embedded `/`, app.js, styles.css, list-import.js and saved-comparison.js must SHA-match frozen files. Browser routing only continues own App reads/findings POSTs and aborts unintended mutations/external routes; it never fulfills or fabricates a response. No app-state variables are injected; control values are set through DOM and actual production submit handlers. Direct API oracles also use the genuine App.

The CSV oracle independently checks header/context/data schema, full typed row JSON, positions, match verdicts, full403row count, at least one original beyond400, exact bytes/length/SHA and genuine blob download. The UI itself does not verify the SHA header; this harness performs the independent comparison. Successful EOF/length is required before comparing bytes.

## Lifecycle, retention and bounds

App binds127.0.0.1:0; own website and Chrome DevTools use ephemeral ports; every port explicitly excludes8787/8790. Chrome gets a fresh profile; page routes block external destinations and website requests during the read interval are independently counted by the website. This is not an OS-wide Chrome background-network attestation.

Actual held App/Chrome ChildProcess objects are tracked from spawn through actual close and stdout/stderr-file close. Log cap16MiB each causes cooperative abort and drains discarded producer output. On finalization only the held child objects can receive kill; no later public/stale PID is used. Actual process/pipe close is required before PASS. This is not an enumeration of every OS descendant nor proof of graceful App Ctrl-C/registry shutdown. After successful UI/CSV all work is already complete; error cleanup may terminate the owned process. A 600-second cooperative deadline/IPC abort does not claim hard shutdown latency. Failure/unresolved close keeps the output directory and produces failure evidence when possible. There is no recursive deletion anywhere.

Shared guardian IPC is compatible with the separate native acceptance preparer's `findings_native_ownership_v1` spawn/close/directory/port/retained events and `findings_native_abort_v1`. Generic guardian currently supports native|ui-mock; browser-native registration remains an explicit future integration step. The standalone driver can run under an owner that waits its actual close, but an external forced kill can interrupt cleanup. Do not treat the standalone source as evidence that such a kill is safe.

Every durable data/report/WAL file is hashed before and after; SQLite SHM lock-coordination files are explicitly excluded. Snapshots cap256MiB/1024files and never alter saved files. Dataset is retained. Candidate executable, Chrome and all297 source input hashes are checked again at completion. HTTP JSON/CSV bodies are bounded8MiB/64MiB and require full EOF/declared length. Output files use fresh exclusive writes except actual Playwright saveAs to a fresh private destination.

## Explicit remaining gates

All actual runtime/native/browser/SQLite/HTTP/import execution remains NOT_RUN under HOLD. Native calibration may expose a real fixture or product defect and must be resolved against the frozen receipt rather than represented as passed. No claim of100% product/Screaming Frog parity follows from this source.

Positive50,001-URL and52,428,801-decoded-byte source controls, genuine incomplete/legacy/budget/A-only/Unknown-source matrix, physical64MiB CSV+1, cancellation/deadline/slow-client/413/422 UI transport matrix, selected inheritance/materialization/RSS/latency, other platforms and optimized installation remain separate gates. These twenty groups do not replace the existing mocked transport matrix or physical source work. No new Rust workspace expectation count is added for authored JavaScript.

## Authoring verification

Final `node --check` on all three .mjs files: exit0. No imports or harness execution. No fixture/SQLite/HTTP/browser/SDK/Cargo/test/build/runtime operations were run. Product, Git and installed binaries were not written. An intermediate newline-escape authoring syntax error was corrected before the final successful checks; it was never executed.