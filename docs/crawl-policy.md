# Crawl policy preview and native execution

`crates/core/src/crawl_policy.rs` is an independently authored pure policy engine. It compiles rules and evaluates ordered URL occurrences without HTTP, filesystem access or crawl mutation. The optional native integration persists each decision in SQLite before scheduling work. A preview result is neither robots permission nor evidence of a server response or complete Screaming Frog equivalence.

The app transport passes a `CrawlPolicyPreviewInput` directly to `preview_crawl_policy`. Its shape is:

```json
{
  "config": {
    "scope": {"roots": [{"origin": "https://example.test", "folders": ["/shop/"]}]},
    "query": {"remove_keys": ["utm_source"]},
    "rewrites": [{"id": "old-route", "pattern": "/old/", "replacement": {"kind": "literal", "text": "/new/"}}]
  },
  "inputs": [
    {"occurrence_id": "1", "source_kind": "anchor", "raw_value": "/shop/old/a?utm_source=one&keep=%20#part", "base_url": "https://example.test/shop/index"},
    {"occurrence_id": "2", "source_kind": "anchor", "raw_value": "/shop/old/a?utm_source=two&keep=%20#part", "base_url": "https://example.test/shop/index"}
  ]
}
```

All objects and tagged enum variants reject unknown JSON fields. Configuration defaults are explicit in the returned complete `config`; an empty scope is invalid. The engine identifier is `open-seo-spider/crawl-policy-v1` and the replacement profile is `rust-first-match-literal-or-braced-captures-v1`. `CompiledCrawlPolicy::new`, `config`, `policy_sha256` and `evaluate` can be used independently. Rules compile once. Changing any effective rule, ordering or limit changes the SHA-256 of the serialized effective config.

## Origin, folder and initial-input rules

Origins compare actual scheme, normalized host and effective port. Supplying `https://example.test` authorizes neither HTTP nor another port. `subdomains: true` permits strict descendants of that explicit host, without inferring sibling hosts or a registrable root. A `www.example.test` root cannot implicitly admit `cdn.example.test`. Additional origins require additional explicit roots.

Folder prefixes are encoded, absolute, normalized paths beginning and ending with `/`. `/shop/` includes itself and descendants, but excludes `/shop`, `/shopping/`, `/Shop/` and `/shop%2Fitem`. `exact_paths` can permit the distinct unslashed landing path. The compiler rejects ambiguous origin/path/query/fragment/userinfo shapes, duplicate paths and dot-segment or malformed percent-encoding configuration. It does not guess a folder from a file-like starting address.

`scope.outside_folder` and `scope.external` default to `deny`; `check_once` gives a `check_only` decision that forbids onward graph traversal. Host/folder classification is independent of fetch permission and remains visible on denied candidates.

`initial_inputs` defaults to `{"rewrite":"preserve","filters":"exempt"}` for `seed` and `list_page`. Opt-in `apply` values transform/filter those explicitly. Safe URL and explicit scope requirements still apply. `redirect`, `saved_fetch` and `sitemap_document` sources skip discovery query/rewrite operations. Redirect/saved targets still receive scope/filter decisions. A sitemap document is kept as its own kind and never becomes a page traversal seed. No source kind grants credentials or changes a crawler's original seed-origin allowlist.

Supported source kinds are `seed`, `list_page`, `sitemap_document`, `sitemap_location`, `anchor`, `resource`, `relationship`, `pdf_annotation`, `redirect` and `saved_fetch`. Optional typed `source_location` is `{ "document_source": "raw_html", "dom_path": "html:nth-of-type(1)/body:nth-of-type(1)/a:nth-of-type(1)", "source_index": 1 }`; it preserves supplied provenance and does not attest that the document was captured. Optional `source_url` and `base_url` must be safe absolute HTTP URLs.

## Query and rewrite profile

Query order, repeated keys, bare keys, empty values, empty markers, percent-escape case and `+` remain unchanged by default. Explicit removal operates on original `&`-separated tokens, without a query-pairs decode/re-encode round trip. Keys match exactly and case-sensitively; no tracking/session parameter names are built in.

`query.key_match` defaults to `percent_decoded_utf8`: well-formed percent bytes are decoded only for name comparison, and `+` remains literal. Invalid percent/UTF-8 keys remain unchanged with a warning. `encoded_bytes` performs no name decoding. `query.order` defaults to `preserve`; `stable_encoded_key` sorts by raw key bytes while keeping complete token bytes and the relative order of equal keys. Empty tokens are retained; deleting all real tokens does not delete an actually retained empty query marker. Sorting is an operator-directed semantic change, not a statement that the server treats two orders as equivalent.

Each rewrite rule performs **one capture search and one replacement**, in saved rule order. No fixed-point loop or all-match iterator is used. `literal` inserts its `text` exactly, including `$1`. `captures` accepts a `template` with `${0}`, `${1}` or `${name}` and `$$` for a literal dollar; invalid/unknown/unclosed references fail compilation. An optional unmatched valid group expands to empty text. This is an explicit own profile; Java replacement syntax and all-substring behavior are not supported.

After each replacement, the resulting URL must be bounded absolute HTTP(S), have a host, contain no userinfo/raw control characters and create no fragment. A later changed step that revisits a previous normalized identity is denied with `rewrite_cycle`; an unchanged match is allowed. A rewrite does not expand scope. It never changes the original raw, resolved or normalized-discovered identity.

Rust regex single searches have `O(m*n)` worst-case behavior; all-match iterators can have `O(m*n²)`. V1 intentionally uses the single-search profile. Regex compilation rejects look-around, pattern backreferences and excessive compiled sizes. [Primary Rust regex documentation](https://docs.rs/regex/1.13.1/regex/).

## Filters, facts and bounds

`filter_identity` defaults to `fetch`; `discovered` uses the pre-transformation URL and `both` requires includes on both identities and denies an exclude on either. Each include list uses any-match semantics; exclusions take precedence. Regex anchors select whole-URL matching. Matched filter ids are stable `include-1`/`exclude-1` indexes; rewrite ids live in ordered `policy_steps`. Initial filter exemptions are explicit. No include generates unobserved URLs, and excluded candidates have no guessed children.

Every decision retains occurrence/source/raw facts, nullable `resolved_discovered_url`, `normalized_discovered_url`, `fetch_candidate_url` and original fragment; ordered query/rewrite/filter/scope steps; `decision`, nullable `reason_code`/`error`, matching ids, host/folder classification, `traverse`, effective engine/hash and warnings. `policy_complete: false` indicates an interrupted policy evaluation such as a trace/token budget failure. Invalid input, unsafe rewrite and scope/filter rejection are explicit denies. A denied row is not a synthetic crawled page and has no invented HTTP status or fetch time.

Preview preserves duplicates and order, while `coalesced_into` identifies an already represented permitted fetch URL. It reports `website_requests: 0`; this is a pure engine fact, not a browser network test. A result also contains the complete effective configuration, engine/profile/hash and limitations. It does not overwrite `LinkRecord` graph/relationship targets or existing crawl snapshots.

All configurable limits are positive and capped: 10,000 preview inputs; 8 MiB serialized input/config; 16,384 bytes per URL; 32 origin roots; 128 folder/exact paths; 128 removal keys; 1,024 query tokens; 64 rewrite rules; 128 include/exclude patterns; 8,192 bytes per pattern and 65,536 pattern bytes total; 4,096 replacement bytes; 32 capture groups; 256 trace steps; 16 MiB serialized output. Regex compiled-size and DFA-size ceilings are each 1 MiB per pattern. These are resource guards, not a licensed crawl URL allowance. Actual serialized input/output counting includes escaping, row objects and numeric metadata through a bounded writer. The engine does not claim total process RSS equals its regex compilation limits.

## Acceptance boundary

Own fixtures under `crates/core/tests/fixtures/crawl-policy/` define 47 independent candidate contracts and nine future integration scenarios. Eleven focused unit tests replay those contracts and exercise explicit capture behavior, aliases, order, malformed shapes, exact origin/suffix/IP boundaries, source-kind exemption, changed identity cycles and configured resource limits. The locked native gate passed all 11 tests on 2026-10-12; its command, scope and source hash are recorded in the fixture manifest. Authored expectations alone are not proof of execution or vendor parity.

The [native integration](native-crawl-policy.md) now uses this engine for scheduling, actual-target robots checks, header isolation, redirect stops, atomic decision/frontier storage and saved-identity recovery. It retains observed graph targets and fragments separately from fetch aliases. The [UI contract](crawl-policy-ui.md) separates offline preview from explicit crawl start, unavailable inherited rows and bounded native evidence. Browser rendering combined with the new policy remains unsupported. Saved effective fetch identities are not rewritten a second time on restart or selective recrawl.

Public vendor capability reference: [Screaming Frog configuration](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/). Its public wording is ambiguous for some list-rewrite and include-match cases; no exact vendor behavior follows from this independent profile. URL and robots standards: [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986.html), [RFC 9309](https://www.rfc-editor.org/rfc/rfc9309.html). No vendor code or assets are used.
