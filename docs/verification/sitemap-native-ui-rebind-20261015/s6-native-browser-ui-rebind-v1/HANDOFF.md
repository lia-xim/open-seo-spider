# Browser acceptance rebind to the source-processing UI

This is a source-only, immutable preparation. No release was issued or consumed. No harness, compiler, SQLite fixture, HTTP client or browser was run. Four `node --check` invocations returned 0; those are syntax checks, not native acceptance.

## Fixed candidate and proof admission

The source candidate is UI manifest `e9bbc545…`, published as UI-only commit `f4ed4b9…` by exact receipt `4316704…`. Its 297 compiler inputs consist of 295 byte-identical inputs from base `69492d0…`/`f43efc1…` plus exactly `web/app.js` and `web/index.html`. It adds zero Rust test functions. The inherited 47 new Rust functions, 38 base bindings, 376 source-authored expectations, 388 required names plus four renderer gates, and minimum 1170 workspace tests remain inherited quantities. None is an execution result.

`preflight.mjs` reads the historical base publication and UI publication, plus exact metadata receipt `ab9c374…` (historical source-only metadata commit `70f2f45…`). It requires that receipt's selected UI suite `b4c0ee4…` and builder `1c72e8b…`, rehashes both actual script files, and reuses the builder's exact `verifyUiLineage` source. No current Git HEAD is required: documentation commits may advance while all compiler bytes remain fixed. Current feature-coverage counts are deliberately not admission inputs.

The only accepted future suite proof is `results/sitemap-source-ui-suite-v1/proof.json`; the only accepted build proof is `results/sitemap-source-ui-{debug|optimized}-build-v2/proof.json`. Both must carry the exact UI `source_lineage`, e9 identity and successful actual-run evidence. Old 694 suite/build proofs cannot satisfy this gate. Actual format/workspace/clippy logs and workspace summaries/names are independently checked, including an empty successful format log. Build proof, binary, browser, Playwright entrypoints and preparation payloads must match their bound hashes. All 297 frozen/current compiler input bytes are checked before runtime imports or output writes, and rechecked before final proof.

Historical receipts do not transfer native acceptance to e9. The original 13 mocked UI groups remain separate. The old native baseline's unknown/stale/race/large-threshold gaps remain open; this rebind does not close them.

## Future execution contract; still blocked by HOLD

The driver retains its strict argument form `--build-mode debug|optimized --release ABSOLUTE_JSON_FILE`. Before any output creation, runtime import, fixture listener, SQLite access or process spawn, it requires both the coordinated execution environment guard and a fresh separately approved release. This document supplies no operative release.

The release schema is the inherited schema with the following distinct bindings:

- `kind`: `root_released_sitemap_findings_browser_ui_native_v1`
- `source_manifest_sha256`: exact e9 manifest
- `driver_sha256`, `preparation_manifest_sha256`, `build_proof_sha256`, `candidate_binary_sha256`: actual selected bytes
- `build_mode`, `expires_at`: explicit selected mode and future expiration
- `chrome_executable`, `chrome_sha256`: explicit absolute browser identity
- `output`: exact fresh `E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/sitemap-findings-browser-ui-native-{mode}-v1`

The output must not already exist. App, fixture and directly owned Chrome use isolated ephemeral ports; 8787 and 8790 are rejected. Data directories and diagnostics are retained on failure. No arbitrary PID is killed. Actual App/Chrome ChildProcess closes and stdout/stderr file drains are observed, then an immutable monotonic completion snapshot must be strictly below 600 seconds. The existing absolute clock, timer ordering and full CSV oracle are preserved.

## Two explicit phases

First, the original actual 405-page public crawl and two XML sources populate the real Store. The original browser flow stays byte-identical to corrected `894010bc…`: seven API/UI filters, independent complete 405 ordinal/identity traversal, two exact XML identities, the 403-match independent API traversal and full ten-column CSV equality, metadata/selection/whole-context comparisons, download EOF/SHA, responsive and inspector controls. Before the new profile module runs, the original zero-new-website-request assertion and every durable-file before/after hash comparison have already succeeded. This phase's proof remains `actual_readonly_interval`.

Second, a new isolated real browser context runs five new native groups through normal DOM interaction. It has no `route.fulfill`, fake API rows, localStorage injection, product-state injection, SQL writes or preset DTO response. Its network route only continues real reads of the owned App and real `POST /api/crawls`; external/other mutations fail.

1. `actual_source_profile_off_defaults_save_reopen_reload`: disabled/hidden fields, 8 MiB / 8 MiB / 500k defaults, real save/reopen/reload persistence; page cap is independently 2 MiB.
2. `actual_source_profile_enabled_defaults_exact_html_bounds`: enabling reveals fields, exact required/min/max/step attributes, all three minimum and maximum positive boundary controls.
3. `actual_source_profile_invalid_range_empty_fraction_blocks_save`: zero, upper+1, empty and fractional values for each field block actual form submission without changing saved settings or starting a crawl.
4. `actual_source_profile_off_real_start_native_omission_and_saved_config`: a genuine UI start for `/p404`, actual HTTP request omits the native object and all four UI-only keys, actual completed crawl and readonly SQLite saved configuration confirm omission.
5. `actual_source_profile_on_exact_three_values_save_reopen_reload_real_start`: real save/reopen/reload of 16 MiB incoming / 32 MiB decoded / 1,250,000 events, actual UI start and readonly saved config confirm the exact three native fields; page cap stays 2 MiB.

The two profile crawls are intentional mutations in the isolated fixture after the readonly interval. Ordinary UI defaults may discover robots/default sitemap paths, so the driver records these own fixture requests without assuming exactly one website request per job. Each crawl must actually complete with one fetched page. No large XML payload is created or fetched. These profile probes demonstrate wiring/persistence only when executed; effective large-source processing and threshold positives remain separate OPEN gates.

## Authorship and derivation

There are 20 inherited native groups plus five new authored groups, total 25, all 0 executed. Four payloads are released together. `DERIVATION.json` contains 20 exact substitutions from immutable eddb payloads; source checks replayed forward and inverse to byte equality for all three inherited files. `browser-flow.mjs` is unchanged. The fourth module is new authored source. `SOURCE-PINS.json` records all read-only dependency pins; `verification.json` records the exact boundary of checks. No Root, compiler-source, Task/work, runtime, installation or Git state was written by this handoff.
