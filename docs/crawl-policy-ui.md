# Native URL-policy interface

The policy UI uses the independent `open-seo-spider/crawl-policy-v1` rule configuration and the saved `open-seo-spider/native-crawl-policy-v1` ledger. These are separate profiles: an offline decision is not a network response or an admitted task. This document describes the current interface contract. Isolated browser fixtures are passing; the fresh embedded native B7 end-to-end gate is pending at this document's creation.

## Configuration and offline probe

Open **Crawl-Einstellungen → URL-Filter, Extraktionen & Prüfgrenzen → Native URL-Policy & Offline-Probe**. Policy use for the next crawl requires the explicit checkbox. When disabled, the crawl request omits `crawl_policy` and `live_policy_limits`, including their UI-only editor fields. Existing legacy settings remain available.

The JSON editor exposes the complete independent v1 configuration: exact origin roots, folders/exact paths, optional strict subdomains, outside-folder/external deny or check-once decisions, query-key removal/order, ordered regex replacements, include/exclude identity and initial-input exemptions. The inline example is an operator draft; the UI never derives a scope, changes an original seed or extends header permission from it. See [the pure policy contract](crawl-policy.md) for syntax and exact defaults.

With enabled native policy, JavaScript rendering and nonempty legacy include/exclude controls are explicit errors. Their old values are not silently removed. The browser validates JSON types, known fields/profiles, origins, folders, enums and resource bounds. Native Rust still compiles and validates its own regex/replacement/profile semantics before HTTP; the browser's checks do not replace that preflight.

`live_policy_limits` is optional. An empty editor object omits it and uses native defaults. Supported fields:

| Field | Native default | Constraint |
| --- | --- | --- |
| `max_row_bytes` | 262144 | 1024–4194304 bytes |
| `max_batch_bytes` | 33554432 | At least one permitted row; at most 256 MiB |
| `max_batch_occurrences` | 100000 | 1–1000000 occurrences per source batch |
| `max_ledger_bytes` | `null` | Positive bytes or `null`; no default total ledger or URL-count allowance |
| `max_page_bytes` | 1048576 | At least one permitted row; at most 16 MiB |

The pure configuration's `limits.max_inputs` limits one offline preview batch. It is not a total native crawl URL limit.

The **Offline-Probe** posts only `{config,inputs}` to `/api/policy/preview`. Inputs are ordered operator-supplied raw lines, an optional HTTP(S) base URL and an explicit source kind. Duplicates, malformed values and fragments remain individual decisions. Initial `seed`/`list_page` examples preserve transformation and exempt filters by default; an `anchor` example exercises ordinary discovery transformation. Redirect/saved-fetch/sitemap-document probe sources skip transformation according to the v1 contract.

Preview output includes effective config, rule profile, policy SHA, raw/discovered/fetch identities, decisions, reasons, warnings and per-row policy steps. `website_requests:0` is an offline-operation fact. It does not establish a captured source, robots permission, successful response or actual scheduling. Preview `coalesced_into` denotes another permitted fetch URL in that preview, not a persisted task occurrence ID. The pure decision's `host_internal` and `folder_internal` classify its **transformed fetch candidate**, not the observed original address.

Editing the configuration or probe, closing settings or switching the current project invalidates/aborts the preview. Even a response delivered after abort cannot enable an old result or clear a newer pending request. Preview and settings save start no crawl. **Crawl starten** remains a separate action. A job's saved policy is immutable during pause/resume and selected recrawl.

## Saved decisions and transformations

**Policy-Entscheidungen** shows actual saved `StoredPolicyOccurrence` rows from:

```text
GET /api/crawls/{id}/policy?after_sequence={sequence}&limit=200
```

The interface uses at most 200 rows per API page. It does not automatically load the complete ledger. Next/previous navigation uses saved sequences and a cursor stack; local search, sorting and decision/basis filters affect only the loaded page. The label states the cursor and loaded/filter-visible counts. Project changes invalidate pending requests; old success, error or completion cannot replace a newer project's ledger or release its busy state.

Rows keep distinct columns for:

- Persisted sequence, generation and `basis` (`current` or `inherited`).
- Occurrence ID, source kind/task role, original raw value and its observation basis/availability.
- Observed URL, transformed fetch candidate and original fragment.
- Policy decision and the separate actual admission reason, scheduled request URL and coalesced task ID.
- Actual fetch-candidate origin/path classification, source ordinal/revision and policy SHA.

An allowed decision does not by itself prove a task was scheduled. List mode, depth, observation-only roles and task coalescing remain separate admission outcomes. Denied rows are never synthetic `PageRecord` captures and have no inferred HTTP status, title, fetch time or selected-recrawl document checkbox.

**URL-Transformationen** filters these same saved facts to cases where both the observed normalized identity and fetch candidate are present and differ. It does not treat that operator transformation as an HTTP redirect. Existing link graph, canonical/hreflang and fragment identities remain the observed addresses; replacement-body metadata and fragment success are not transferred to an uncaptured original address.

The keyboard-accessible occurrence inspector displays exact saved JSON as inert text. An explicit document action exists only when that occurrence's scheduled URL matches an actual current `PageRecord.url`. It opens that captured request, not an invented original target. Page details separately show its saved admission, policy SHA, initial/effective page-traversal permission, XML-discovery permission, inherited marker, candidate completeness/source revision and fresh redirect decisions. `can_discover_xml` is distinct from page `can_expand`; sitemap check-only does not alone establish recursive XML discovery permission.

## Completeness, history and exports

The summary presents persisted revision, occurrence count, committed bytes, incomplete-source count, rolling SHA and complete/incomplete/unknown state. Completeness concerns the saved ledger, not exhaustive coverage of every URL on a website. The server's `scope` presents configured origin/folder boundaries; it does not establish universal capture of every possible source kind. Unknown engines or endpoint/storage errors are unavailable states, not empty complete results. `ledger_available:false` explicitly means saved SQLite occurrence rows are unavailable (for example during initialization or for imported summary-only history); the UI shows the server reason, disables full exports, and does not treat an imported `complete:true` summary as an available empty ledger.

Optional `checkpoint.inherited_history` reports expected/available parent occurrence counts, completeness and parent-ledger SHA separately. A selected child can preserve original admissions while some imported parent ledger history is unavailable. Its `basis:inherited` rows remain old evidence; `basis:current` means a current policy observation, not proof of a successful document request. The interface never derives policy-row freshness from `fetched_at`.

The global CSV action exports only the filtered rows of the loaded ledger page, with source identity/basis/hash. Dedicated **Vollständiges Policy-Ledger CSV/JSON** actions directly download root-native exports from `/policy.csv` and `/policy.json` through the browser, rather than reconstructing a complete ledger or buffering its entire response into the app's JavaScript heap. Native exports use one SQLite read snapshot even while another generation appends. JSON includes checkpoint and `export` row-count/revision/all-snapshot-rows/incomplete evidence; response headers include `x-policy-snapshot-revision`, `x-policy-snapshot-occurrences`, `x-policy-sha256`, `x-policy-ledger-sha256` and `x-policy-ledger-complete`. Exporting all rows in a snapshot does not imply that its ledger is complete. All strings remain escaped/inert in the UI. CSV prefixes formula-like text after leading whitespace/tab/CR/BOM with an apostrophe; JSON preserves exact values.

Legacy jobs without policy recording remain explicitly unrecorded. No startup migration or retroactive policy attestation is implied by opening them.

## Verification scope

`web/verification/ui-policy-smoke.cjs` currently passes 15 isolated Chromium fixture groups: typed/disabled config, conflicting settings, offline zero-crawl behavior, deliberately late preview/ledger responses after abort, paging200/200/5, page-local filter/sort/CSV, current/inherited and unavailable-history facts, explicit unavailable SQLite rows despite an imported `complete:true` summary, unknown/legacy/error handling, inert text, direct browser export separation and 320px reachability. Same-project pause and resume deliberately hold two response bodies from different view generations; an old response cannot release or replace the newer loader. This fixture is not native engine/persistence proof.

`web/verification/ui-policy-live.cjs` passed all 8 actual local groups on the final frozen Debug app at `http://127.0.0.1:18789` on 2026-10-12 at `14:20:09.791Z` (binary SHA256 `92F8D68294451852A7DCDB47DCD1BE6B5F595E7184DECF79B1F0BB9B224E1B78`). The flow checks offline zero requests, explicit configuration, 211 alias occurrences coalesced to one request, actual-target robots, denied redirects/check-only expansion boundaries, original fragment/link identities, bounded ledger paging, native CSV/JSON, narrow UI and actual imported summary-only history without SQLite rows or new website requests. Browser runtime errors were empty. Exact evidence is retained in `web/verification/ui-policy-live-results.json`. The document action is asserted only after the browser has loaded the actual completed crawl and six capture records; API completion can precede its next UI poll.

The separate earlier 95-check full UI regression suite used binary SHA256 `B9879198CDDE5AFA66B155C64DD15A51908975741A83E6439DA0659132F350EA`, including its first Policy8 pass at `14:04:42.192Z`. Only Policy8 was repeated against the final `92F8…` binary; the other 87 checks are not claimed as freshly executed on it. Root's work evidence archives each run with its own runtime/result hashes.

Native export evidence binds the browser's actual download URL and bytes to a separate byte-identical GET of the completed immutable snapshot, whose response headers are checked against checkpoint/hash metadata. Chromium attachment navigation emits a download event but no Playwright `page.response`; the test does not attribute those separately observed headers to a browser response event. This bounded local integration proof does not establish every source kind, resumed export concurrency, total website coverage or vendor feature parity.
