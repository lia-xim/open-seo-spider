# Native resource discovery in the local UI

This optional configuration belongs to the independent native resource-discovery implementation in [resource-discovery.md](resource-discovery.md). It is off by default. With it off, the browser omits `resource_discovery` and `resource_discovery_limits` from the crawl request, even if their disabled editors contain malformed JSON. Historical jobs retain their previous behavior.

Saving settings makes no website requests. The settings apply to a future crawl; they do not re-extract a saved source or fetch a declared resource. Native resource discovery and JavaScript rendering cannot be enabled together. The browser reports this conflict without silently clearing either value; Rust remains responsible for authoritative validation before HTTP.

## Explicit typed settings

The **Native Ressourcenermittlung** settings accept a JSON object with only these fields:

| Field | Members |
| --- | --- |
| `resources` | `css`, `javascript`, `images`, `media`, `pdf`, `swf`, `other`, each with optional boolean `store` and `fetch` |
| `navigation` | `internal_anchors`, `external_anchors`, `iframes`, `refresh`, `canonical`, `hreflang`, `pagination`, `amp`, `mobile_alternate`, `pdf_annotations`, each with optional boolean `store`, `fetch` and `expand` |
| `css` | Boolean `stylesheets`, `style_elements`, `style_attributes` |
| `resource_scope` | `external_origins`, an array of exact HTTP(S) origins |
| `follow_rel_nofollow` | Boolean |
| `follow_document_nofollow` | Boolean |

Unknown fields, null rules, arrays in place of objects and non-boolean switches are rejected. A supplied navigation rule with `expand: true` requires an explicit `fetch: true`. The browser preserves the entered object without adding defaults or replacing its strings.

For example, this captures and fetches image references while recording canonical references without fetching them:

```json
{
  "resources": {"images": {"store": true, "fetch": true}},
  "navigation": {"canonical": {"store": true, "fetch": false, "expand": false}},
  "resource_scope": {"external_origins": ["https://cdn.example.test"]}
}
```

The `{}` configuration requests the independent v1 defaults. Resources default to store/fetch enabled. Completely absent internal-anchor, iframe and refresh families default to store/fetch/expand enabled; other absent navigation families default to store enabled and fetch/expand disabled. A **provided partial navigation object** instead uses the generic rule defaults: store enabled, fetch and expand disabled. For example, `"internal_anchors": {"store": false}` does not inherit that family's fetch/expand defaults. CSS source switches default to enabled; both nofollow switches default to false. These defaults are implementation choices, without a claim of manufacturer parity.

`crawl_links: false` disables navigation discovery in explicit native mode. Resource and CSS dependency switches remain independent, including list mode. A selected recrawl suppresses all outgoing discovery. Each occurrence retains its own decision: a disabled switch does not erase the source observation, and another independently allowed occurrence may grant a fetch for the same target. A source reference alone does not establish target success, browser selection, CSS cascade, script execution or rendered visibility.

## Resource origins and operational budgets

`external_origins` accepts at most **32** different normalized HTTP(S) origins. A trailing slash is allowed. Host case and default ports are normalized for duplicate detection, so `https://CDN.example.test` and `https://cdn.example.test:443/` are duplicates. Wildcards, encoded wildcard hosts, content paths, credentials, query markers and fragments are rejected by the browser. Rust verifies its own URL contract. This resource scope never expands navigation, robots or policy permissions, or the original credential/header origins.

The separate budget JSON accepts these fields. All numeric values are positive integers; zero does not disable a budget.

| Field | Native default | Maximum accepted by the UI |
| --- | ---: | ---: |
| `max_source_candidates` | 100,000 | 1,000,000 |
| `max_occurrence_utf8_bytes` | 65,536 | 65,536 |
| `max_source_batch_bytes` | 33,554,432 | 134,217,728 |
| `max_css_utf8_bytes` | 16,777,216 | 67,108,864 |
| `max_css_tokens` | 1,000,000 | 8,000,000 |
| `max_css_nesting` | 64 | 128 |
| `max_diagnostics` | 1,000 | 10,000 |
| `max_ledger_bytes` | `null` | Positive JavaScript safe integer, or `null` |

`max_ledger_bytes: null` means no operator total-ledger budget; individual source, row, parser and page budgets still apply. An empty `{}` budget editor omits the field and uses native defaults. The browser does not invent relationships between different budgets. Rust is authoritative for platform integer range and runtime enforcement.

## Explicit current semantic rules

The optional `resource_discovery.contract` uses `open-seo-spider/resource-contract-v2`. An absent or null contract keeps the existing rules exactly; the top-level `{}` configuration remains V1. An explicitly supplied `"contract": {}` opts into native V2 defaults. The browser preserves it without inserting an engine or flags. Disabling resource discovery, reopening settings, restoring a saved draft, importing a report or resuming a job never inserts this contract automatically.

The visible **Aktuelle Regeln verwenden** action changes only the next crawl's draft. Its internal/external overrides apply to the exact Rel qualifiers `nofollow`, `ugc`, `sponsored` **or** the applicable document Nofollow directive. The original seed scope determines internal/external classification; asset origins do not redefine it. The action clears both old `follow_rel_nofollow` and `follow_document_nofollow` flags. If their effective old values are equal, that value initializes both new overrides. If they differ, the dialog explains that exact mapping is impossible and initializes both overrides to false. The user can then change each independently. V2 plus either old flag true is rejected before the crawl POST; values are preserved for correction.

Saved provenance is separate from this editable draft. The API's `configured_semantic_contract` and `semantic_contract_binding` identify declared settings versus a full-config/generation-bound native snapshot. An available V2 snapshot must carry the matching `snapshot.resource_contract_engine`; V1 omits it. Missing historical profile fields stay unknown. A declared imported profile does not reconstruct native history. An unsupported explicit profile has no supported semantic interpretation, while its exact values remain inert JSON.

The observed Grant role comes only from `grant.reference_kind`. It is never filled from the occurrence's declared role, response MIME, task union or current editor. Optional qualification facts retain their own profile, original-scope result, Rel qualifiers, HTML/Header source, document Nofollow and recorded follow decision. An absent fact stays unknown; an unsupported fact profile suppresses interpreted values while preserving raw JSON. This qualification decision is independent of other scope, Robots, policy or fetch checks.

The transport inspector also shows optional per-hop redirect facts: source URL, literal Location, target URL, original Grant roles, authorizing occurrence IDs, scope/policy/Robots facts, admission reason and `request_started`. This last value records request initiation, without proving delivery or success. Null checks differ from false, and absent redirect facts differ from an explicitly observed empty array. The table displays at most 200 supplied decisions and states that bound. Intermediate directives or permissions never become final-document facts through UI inference.

The dedicated browser/API fixture `ui-resource-contract-smoke.cjs` now passes **11 groups** at 2026-10-12T22:43:40.339Z, script SHA256 `0fd07abf51582332b37caf0c7b101ee39bb92671d085699b347e56fb6bc0e983`, result SHA256 `604c12142005202a4eaee0a0cafc1ac677923d2da25a15f89fc70497d1edfa20`. It retains migration truth combinations, exact serialization, profiles/grants/qualifiers/redirect unknowns, malformed facts, held responses, bounded inspectors and 320px controls. Its former 10-group PASS and the exact subsequent source d0f3edd CI failure are preserved in task-level `work/resource-discovery/b10-ui-ci-repair-2026-10-12`.

The CI failure compared all API requests (110 versus108), including legitimate background GETs; the old logger did not retain those two paths. A controlled pending analysis response now emits responses/fragments GETs during a correctly rejected invalid Save, proving the old total-count oracle's defect. The replacement guards26 actions: only recognized bounded GET reads are allowed, all API mutation ingress and website attempts stay zero, and loaded-page CSV additionally forbids ledger/export/cursor reads. Arbitrary POST and website negative controls must fail the guard; their events are recorded separately. A global foreign-network fence runs before navigation and aborts both unexpected product attempts and the deliberate probe before delivery. The final product runtime-error and unexpected/blocked-network arrays are empty. Independent read-only review closed both oracle concerns. This is an isolated authored API fixture and local replay; CI rerun is still pending and native runtime evidence below remains unchanged.

The native script `ui-resource-contract-live.cjs` passed **10 actual groups** at 2026-10-12T22:12:52.024Z on the authorized isolated preview `http://127.0.0.1:18789`, Debug SHA256 `a5881b7c8cffff68a1e47e8a22d9d02e6ca2f1c1c698cebc74036b06dbf92d0f`, PID28724. It exercised successfully completed UI-started V1/V2 jobs against three owned origins, exact internal/external HTML and HTTP-Link qualifier facts, document Nofollow rules, allowed and denied asset redirects, stored profile binding, declarative imports, read-only exports with exact denied CSV/JSON cells, preserved V1 report bytes and 320px reachability. Runtime errors and unexpected requests were empty. Source, served-source, result, log, runtime and saved native snapshot artifacts are recorded separately in task-level `work/resource-discovery/b10-ui-runtime-proof.json`. No product change was needed. A first ten-group PASS is retained separately; the final rerun strengthens the harness's successful-completion and exact CSV assertions without counting the first pass again. Earlier B9 runtime checks remain separate; this ten-group gate does not establish the entire planned resource matrix, manufacturer parity, CI, release or installation.

## Bounded saved ledger and transport views

The **Ressourcenermittlung** sidebar now reads the fixed native API, without making website requests:

- `GET /api/crawls/{id}/resources?after_sequence=0&limit=200` reads ordered `observations` with a returned `next_after_sequence`.
- `GET /api/crawls/{id}/resources/transports?limit=200` reads ordered `transports`; subsequent requests send the returned `next_after_url`. The first request omits this cursor, rather than sending an invalid empty URL.

Each request loads at most **200** rows, potentially fewer when the API byte budget applies. Next/back use the exact returned cursor; there is no automatic load-all. Search, sort and filter apply only to the loaded page. A new page is an independently read SQLite snapshot, not a claim that several active-crawl pages share one transaction. The native full exports provide that separate snapshot boundary.

The envelope states `website_requests: 0`, `entity_bodies_included: false` and `snapshot_scope: "sqlite_read_snapshot"`. An available native snapshot contains `snapshot.crawl_config_sha256` and positive `snapshot.generation`, bound by the server in the same SQL read transaction. The UI requires those to agree with the expected full-config hash and generation in the envelope, and with the currently selected job's generation when known. The resource-specific checkpoint hash is a separate binding. An unavailable ledger has `snapshot: null`; a declared checkpoint does not reconstruct native rows.

Occurrence columns preserve sequence, generation, current/inherited basis, requested source identity, source document URL, raw/decoded/resolved values, reference role, classification hint, original MIME hint, source ordinal, DOM/UTF-8 position, source revision and evaluation revision. `source_complete` applies to the actual HTML DOM or individual CSS block/attribute/stylesheet identified by `source_unit_revision`; it is not the overall discovery-complete flag. Missing fields display unknown, while zero byte positions and empty strings stay observed values.

The exact JSON inspector retains each decision reason, independent immutable grant, coalesced occurrence ID and attributes. A later union of task permissions never rewrites an earlier check-only grant. A permission is not a success status; occurrence rows do not borrow HTTP facts from a matching target URL.

The transport view separately shows metadata-only `ResourceTransportObservation` rows: requested identity, final stored URL, observed generation, original/final status when known, transport error/Robots flag and outer `retained_response`. Optional response facts expose body-retention, decode/hash/completeness observations and `unretained_fields`. The projection itself is bodyless and can have missing or explicitly unavailable content-capture text, PDF text or custom extraction values. Its inner `resource_response.retained_response: false` is distinct from an outer `retained_response: true` for an answer retained in the ordinary report. No text or entity body is reconstructed from metadata.

The summary deliberately labels `completed_transports` **Abgeschlossene URL-Abrufe (vorliegendes Protokoll)** and `retained_responses` **Gespeicherte Ergebniszeilen**. The former counts committed URL/Page-frontier units, without representing every redirect hop or separate Robots/sitemap HTTP request. A selected child can retain inherited report rows while its available transport history covers a different cohort. `pending_transports`, `pending_expansions`, `incomplete_sources` and `complete` remain separate. Missing inherited-history rows remain incomplete even when a declared parent's count is zero.

Project, view and crawl-generation changes invalidate held requests and inspectors. Old responses and old `finally` blocks cannot replace or unlock a newer request. Failed reads and unsupported bindings suppress displayed rows and exports; they are not complete empty results. Untrusted values are inert escaped table cells and plain-text JSON.

## Explicit export scope

**Gefilterte geladene Seite CSV** exports only visible rows of the loaded page. It appends snapshot scope, full-config hash, SQL generation, checkpoint revision/hash, discovery-complete and inherited-history availability, plus `filtered_loaded_page_only`. Website-controlled strings use the same apostrophe prefix for spreadsheet formula protection as other UI CSV exports; typed numeric/boolean values are preserved. JSON inspectors keep exact data.

**Alle gespeicherten Vorkommen CSV/JSON** requests `/resources.csv` or `/resources.json` directly as a native download. The browser does not collect cursor pages or substitute filtered rows. The server pins one read transaction, streams every saved occurrence and rejects missing/disabled SQLite history with HTTP409. A complete saved-row export does not establish complete discovery. Imported checkpoint-only data disables these export controls.

## Acceptance boundary

Settings and ledger display fixtures are `web/verification/ui-resource-discovery-smoke.cjs` and `ui-resource-ledger-smoke.cjs`. Final snapshot-bound fixture results are **10/10 PASS** at 2026-10-12T18:07:47.776Z and **16/16 PASS** at 2026-10-12T18:07:29.774Z. Both record zero browser runtime errors and unexpected website requests. The ledger fixture covers 67 bounded reads and two direct native-path fixture downloads, without export Fetch/XHR or cursor traversal. It checks full-config/generation bindings, unavailable native snapshots, byte-short pages, source-unit unknowns, immutable grants, bodyless transport facts and held project/view/resume responses. These are authored local API responses, without native parsing or persistence proof.

The general47 and saved-content18 fixtures passed separately after integration and remain separate regression checks. Their source and result paths are recorded rather than relabeling prior native runtime evidence as a B9 test.

The actual owned-site script `web/verification/ui-resource-discovery-live.cjs` passed **12/12 native UI checks** at 2026-10-12T18:52:22.645Z against the isolated `http://127.0.0.1:18789` repaired Debug binary, SHA256 `142c0a1af3d420de5ccb1e182bdd04d701f8087a5dc72b3758464aa5e3d65724`. It verifies owned HTML/CSS/CDN fetch roles, independent same-target grants, source-unit incompleteness, bodyless transport projections, source-capture availability, returned cursors, native/loaded-page exports, selected-child no-outward behavior, immutable parent bytes, declared imports and 320px reachability. Browser runtime errors and unexpected requests were empty. The former `1fc72a...` runtime's 65 checks and first attempt's harness-only selector failure remain archived separately. The new runtime includes the coordinating agent's saved-capture cold-load correction; the embedded UI and all six live scripts stayed unchanged during this rerun.

Related actual Content12, Policy8, Selected9, Control10 and Renderer14 checks also passed on that same repaired binary through 2026-10-12T18:53:48.748Z: **65 actual checks** in total, without mixing the isolated display fixtures or former runtime's checks into the count. Source, result and log hashes, executable path/PID, prior-runtime archive and separate failure records are preserved in `work/resource-discovery/b9-ui-runtime-proof.json` at task workspace level. The independent cold-start reproduction, native build/unit tests, commit CI, release and installation remain separate evidence owned by the coordinating agent. Full manufacturer and browser-selection parity remain `NOT_PROVEN`.
