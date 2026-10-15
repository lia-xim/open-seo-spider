# Explicit durable sitemap membership capacity — authored, not run

This isolated proposal fixes the specific storage mismatch exposed by the mandatory genuine compact 50,000-URL sitemap case. Fifty thousand entry/Loc pairs require at least 51,700,000 JSON bytes before catalog envelopes, so a fixed 32 MiB ledger cannot retain all those rows even when XML decoding/counting succeeds. This is a storage failure, not a passed completeness or unknown verdict.

## Concrete behavior

`CrawlConfig.sitemap_membership_limits` is a separate, explicit optional object:

```json
{
  "sitemap_source_processing": {
    "max_post_http_entity_bytes": 67108864,
    "max_decoded_xml_bytes": 67108864,
    "max_xml_events": 2000000
  },
  "sitemap_membership_limits": {
    "max_catalog_json_bytes": 134217728,
    "max_row_json_bytes": 262144,
    "max_rows": 1000000
  }
}
```

The profiles are independent. Source 64 MiB alone still has the legacy 32 MiB membership budget. Membership 128 MiB alone preserves the legacy source/page-body clamp. Default/absent/null serializes the new field omitted; old configuration JSON bytes/hashes retain their existing representation. `{}` explicitly selects existing defaults. The optional object rejects sequences, scalars and unknown nested keys; field types remain integers. Direct Serde decoding rejects duplicate nested keys, but HTTP `Json<Value>` has already coalesced duplicate literals: no raw-HTTP duplicate rejection is claimed.

The existing 32 MiB default becomes `DEFAULT_SITEMAP_CATALOG_BYTES`. The hard maximum becomes 128 MiB, requiring an explicit configured budget. Existing per-row maximum 1 MiB, default row 256 KiB and maximum one million rows are unchanged. Invalid zero/overflow/cross-field values are rejected before Crawler I/O; no clamping or input repair occurs.

Fresh Crawler initialization uses the validated limits. Existing catalog initialization/resume continues using the exact durable limits, config hash, root IDs and historical qualification. Changing the new field on resume is rejected by the existing full config comparison; four existing operational overrides remain allowed. Selected recrawl clones the complete parent config (`recrawl.rs:240–241`), and its existing validator permits only four operational knobs (`404–414`); parent Some128 to child None is unsupported. A positive above-default SQL inherited-catalog control retains the same config/rows/digest, plus a None portable-config negative. Pure `project_inherited_catalog` behavior is unchanged.

An above-default catalog cannot grant itself greater capacity solely by editing a self-consistent summary: initialization, materialized SQL load and streamed raw verification require an exact declared limits block projected from saved config. The declaration is measured as actual SQL UTF-8 BLOB bytes before copying its string into Rust; only 1..=4096 bytes are fetched, and the fetch repeats the bound. This small-object boundary adds no overall config/wire limit. Tests include a valid >64 KiB outer config and an oversized Unicode nested declaration. Portable report validation checks the declaration even when coverage inventory is absent. Lower custom and legacy capacities keep their prior authority semantics. All existing row/sequence/rolling hash/source/policy/current-read/cursor checks and source append savepoint remain intact; the proposal does not change their bodies.

API create input and CLI `--config` already deserialize the whole CrawlConfig, while MCP forwards that whole config to the same API adapter. The new field is therefore preserved without a lossy additional mapper. The MCP tools/list schema explicitly exposes object/null, all three integer bounds/defaults and nested `additionalProperties:false`. Capabilities expose the default/maximum and source-work independence. The UI exposes a separate opt-in checkbox and three bounded fields, validates row<=catalog, persists/reopens/restores their exact strings, strips UI-only keys and sends the typed config object. Disabled settings omit the object. Existing source-processing controls remain independent.

## Meaningful authored tests

Nine new Rust test functions are listed verbatim in `manifest.json`. They cover absent/null/default round trips, wrong shapes/unknown fields/types/hard bounds, source independence, actual Crawler/Store save/bootstrap/reopen and operational resume, actual create-config adapter and actual MCP tools/list schema dispatch, and genuine parser plus on-disk SQLite 50,000-pair admission.

The 50,000 test parses real XML with all retained entries/locations; its default 32 MiB control must reject atomically with no committed source rows and pending task unchanged. Its explicit 128 MiB control must commit all 100,001 document/entry/location rows, exceed the former cap, preserve completeness and digest, support tail paging/stale-cursor rejection, and cold reopen with identical catalog. Additional real SQLite controls preserve row count/row-byte atomic failure and reject above-default self-granted summary capacity in both raw/materialized/portable readers. These tests use existing caller-declared transport facts; they are not HTTP/native capture proof.

`browser-membership-flow.mjs` is a separate authored acceptance composition function, outside product inputs. Root can compose it into a newly source-bound actual-browser gate. It checks disabled defaults, save/reopen/reload, actual typed API-start payload, actual saved report config, invalid cross-field/hard maximum behavior, and disabling again with three tiny owned fixture crawls. It grants itself no browser/server, runs nothing on import, and has not been executed. Both this function and the changed `web/app.js` passed syntax-only `node --check`.

## Resource and coverage boundaries

128 MiB is a bounded catalog-data capacity, not a total heap/RSS guarantee. The existing materialized append/compatibility catalog path can hold original/extended rows and derived serialization/validation state concurrently. No total-RSS improvement is claimed. The native genuine 50k positive, source threshold matrix, CLI/SDK/browser runtime, full suite, formatting and Clippy must execute against a fresh combined Root source/build before acceptance.

The compact 50k positive does not prove every protocol-valid 50k/50 MiB sitemap can be retained: retained text remains 32 MiB, field bounds remain 8,192 bytes, and the explicitly chosen catalog budget can still be exhausted. Full source measurement, protocol limit findings, retained membership and source-cohort completion remain distinct. API JSON 8 MiB, CSV 64 MiB, source measurement store 32 MiB, source XML/body/parser limits and all default limits are unchanged. Full original XML response durability remains an independent gap; hashes and harness wire witnesses are not durable product body retention.

## Promotion contract

Only Root may promote. `manifest.json` lists each real `source/<path>` after hash/bytes and exact Source V3 before hash, or `ABSENT` for a new file. `derivation.json` holds reversible literal operations for all modified baseline files. Root should verify all before bytes, independently review the proposal, promote only these files, format them together with the separate SQL repair, produce explicit final formatted approval hashes, and only then freeze a fresh combined V4 snapshot. No current 301 Root source inputs, executable, Git state, user data or processes were changed by this author. No Rust formatter/compiler/tests/native fixtures were run here.

Revision v2 preserves v1 ee7f... unchanged. It corrects the independent review finding that v1 fetched the projected object string before checking its byte length. The inherited SQL control uses the existing minimal selected_anchor helper (parent catalog only); it proves child declaration plus immutable catalog/row/digest inheritance, not a complete public parent CrawlReport/config round trip. The public parent-config clone/four-operational-knob route remains source-traced and separately requires runtime acceptance.

Revision v3 additionally applies UI operations to the original mixed-newline buffers with minimal positional edits. Every original byte outside the intended replacement spans is preserved; ui-byte-edits.json records a byte-exact inverse. Product semantic text is unchanged from v2. The separate authored browser helper now requires the initialized catalog for every fresh crawl before checking its effective32/128MiB capacity; a missing catalog cannot pass conditionally.
