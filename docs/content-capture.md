# Opt-in HTML text capture

This pure module selects text from an explicitly supplied UTF-8 HTML string. It performs no HTTP, file, browser, provider or configuration mutation. It does not enable capture on existing crawls: the integrating crawler must keep its optional capture configuration absent by default. Supplied HTML is required for a text observation; metadata, counts, titles, PDF previews and an existing body hash cannot stand in for body text.

## Public contract

`ContentCaptureConfig::validate()` and `::compile()` reject invalid CSS and resource settings without needing HTML, so an integrating crawler can validate before HTTP. `CompiledContentCaptureConfig` exposes `config()` and `config_sha256()` and provides `capture(&ContentCaptureInput)`. The convenience `capture_content(&ContentCaptureInput)` compiles and captures. Input must carry the exact effective compiled configuration; a mismatch is an error. Source selection is part of that configuration and its hash.

```json
{
  "source_kind": "raw_http_html",
  "html": "<html lang='de'><body><nav>Navigation</nav><main>Hallo <b>Welt</b>!</main></body></html>",
  "source_complete": true,
  "decode_status": "exact",
  "source_truncation_reason": null,
  "unavailable_reason": null,
  "config": {
    "sources": ["raw_http_html"],
    "include_selectors": ["body"],
    "exclude_selectors": ["script", "style", "template", "noscript", "nav", "footer"]
  }
}
```

The selected text in that example is `Hallo Welt !`, with root-declared language `de`. Configuration limits omitted from JSON use the defaults below. Input/config/limits and result shapes deny unknown fields. Sources must contain one or two distinct values (`raw_http_html`, `rendered_dom_html`). The default is raw only. A rendered input under raw-only configuration is rejected; selected but failed rendered observation uses absent HTML and a concrete unavailable reason. Raw and rendered calls have separately labelled results and HTML/text hashes. The pure module never substitutes raw content for a rendered observation or executes JavaScript.

The result contains `engine`, `selection_profile`, `whitespace_profile`, `parser_profile`, `unicode_version`, caller-declared source kind/completeness/decoding, `config` and `config_sha256`, `html_hash_representation`, nullable `html_utf8_sha256`/`html_utf8_bytes`, nullable `text`/`text_sha256`, `status`, ordered `reasons`, nullable `evidence`, and `website_requests: 0`. Root URL, final native/rendered document URL, original wire bytes, timestamps, browser identity, source revision and persisted attempt/freshness belong to the integrating crawler's separate provenance binding; this module does not fabricate those fields.

## Selection and whitespace profiles

Engine `open-seo-spider/html-content-capture-v1` parses with the locked scraper0.27.0/html5ever document-recovery implementation. Its selection profile is `css-ancestor-membership-document-order-v1`. At least one nonblank include selector is required; excludes may be empty. A text node is eligible when some element ancestor, including its containing element, matches an include and no element ancestor matches an exclude. Exclusions win even over a deeper included descendant. Include list order does not reorder output. Duplicate/overlapping selector matches do not duplicate text. Comment/attribute values, image alt, input values and generated CSS content are not text nodes and are not appended.

The engine opens each reachable node once and maintains inherited include/exclude state on a path stack. Template content's extra fragment level is traversed with the template ancestor state, so default template exclusion prevents leakage. Clearing exclusions can deliberately include parser-represented template/script/style content. This is an independently specified selection profile, not a universal browser DOM/template or vendor-equivalence claim. Malformed HTML is analyzed as the parser's recovered document, without claiming the author's source was valid. Scraper supplies document parsing and CSS selection through html5ever/selectors. [Primary crate documentation](https://docs.rs/scraper/latest/scraper/), [upstream implementation](https://github.com/rust-scraper/scraper).

Whitespace profile `unicode-whitespace-and-nonempty-node-boundaries-ascii-space-v1` splits each eligible text node on Rust Unicode whitespace, emits each nonempty run unchanged, and inserts one ASCII space between consecutive runs, including adjacent nonempty text nodes. Leading/trailing whitespace disappears. It performs no case conversion, Unicode normalization or visual-word inference: `foo<em>bar</em>baz` becomes `foo bar baz`; composed and decomposed accents remain distinct. Entities are decoded by the HTML parser. U+200B is not removed by this whitespace rule. The result records the Rust Unicode version used. [Rust character/Unicode whitespace documentation](https://doc.rust-lang.org/std/primitive.char.html#method.is_whitespace).

Hidden attributes and inline `display:none` do not cause automatic exclusion. The module does not evaluate CSS, computed visibility, browser layout, shadow roots, remote iframe contents, readability-library article selection, detected language or PDF text. An explicit user selector may exclude nodes based on their stored attributes. Native HTML and a rendered snapshot use the same documented parser-selection profile; accepting a rendered snapshot does not add computed-visibility semantics.

## Declared HTML language

`html_language` reads only the parsed root HTML element's `lang` attribute from this exact supplied source. It never inherits language from a body/template tag or another raw/rendered observation. Accepted values preserve case and exact attribute text: 2–8 ASCII letters in the first subtag, followed by zero or more hyphen-separated 1–8 ASCII alphanumeric subtags, at most128UTF-8 bytes. This bounded syntax profile does not validate the complete BCP47 registry and does not detect the text's language. Surrounding whitespace, private-use-only/grandfathered forms outside this profile, malformed/overlong values remain unknown with `html_language_reason` (`html_unavailable`, `missing_attribute`, `empty_attribute`, `unsupported_syntax`, `byte_limit`). Unsupported language does not make otherwise complete captured text incomplete; downstream readability can remain unknown separately. A header-declared language is not silently substituted for this HTML-root observation.

## Completeness and hashes

| Status | Text | Meaning |
| --- | --- | --- |
| `complete` | Nonempty string | Declared complete source, exact decoding, full reachable traversal and no text truncation. |
| `empty` | Empty string | The same completeness conditions, with a legitimately empty selection. |
| `incomplete` | String, possibly empty | Available source sample with source/decoding/traversal/capture uncertainty. |
| `unavailable` | `null` | HTML was not supplied; no text/HTML hash or traversal evidence is invented. |

Absent HTML cannot declare `source_complete: true`. A complete source cannot declare a truncation reason. A caller unavailable reason is valid only with absent HTML. Caller reasons must be nonblank, control-free and at most512UTF-8 bytes. These are caller declarations, not attested artifact authenticity.

Reasons have typed code and nullable detail. Codes are `html_not_supplied`, `source_incomplete`, `decode_lossy`, `decode_unknown`, `dom_node_limit`, `dom_depth_limit`, `selector_check_limit`, `text_byte_limit`. Source uncertainty appears before traversal/text limits. Lossy or unknown decoding prevents both `complete` and `empty`, even when the supplied sample is usable. Source body/render caps remain explicitly incomplete; absent/non-HTML/failed renderer input remains unavailable under its selected source label.

`html_utf8_sha256` hashes the exact supplied string's UTF-8 bytes under `supplied-html-string-utf8-v1`. It is not an original HTTP-wire-body hash when decoding/transcoding occurred. `config_sha256` hashes deterministic typed JSON with resolved defaults and preserved list order. Engine/profile labels are separate versioned facts. `text_sha256` hashes exact emitted UTF-8 text, including a partial sample. Empty available text has the standard empty SHA256; absent text has no hash.

Evidence records parser-allocated node count (including detached recovery allocations), actually visited nodes/elements/text, eligible/nonempty nodes, template fragment count, maximum visited depth, checked selector calls, per-selector match counts, and `traversal_complete`. Counts/hashes of the observed normalized stream describe only the reached prefix if traversal stops. When only the emitted text cap stops storage, traversal continues within its budgets: `observed_normalized_utf8_bytes` and its separate SHA can describe the whole selected stream, while the emitted sample/hash remain partial. They never imply source decoding/completeness was independently proven.

The text cap keeps an exact stream prefix at a valid UTF-8 scalar boundary. It never skips a large scalar to append later small ones, never leaves a separator whose next scalar could not fit, and never splits a scalar. Exact cap occupancy is complete if no further selected character exists; trailing whitespace alone does not manufacture truncation. No ellipsis is appended.

## Resource boundaries

All limits are positive and may be lowered, with the following defaults also serving as hard ceilings. Exceeding input/config/output budgets is an explicit error. Exceeding traversal/check/text budgets produces an available incomplete sample with a typed reason.

| Limit | Default and ceiling |
| --- | ---: |
| `max_html_utf8_bytes` | 8MiB |
| `max_text_utf8_bytes` | 512KiB |
| `max_visited_dom_nodes` | 250000 |
| `max_dom_depth` | 1024 (document root at zero; fragments count) |
| `max_selectors` | 64 across include/exclude lists |
| `max_selector_utf8_bytes` | 4096 per selector |
| `max_selector_checks` | 2000000 calls |
| `max_config_json_bytes` | 64KiB actual typed serialization |
| `max_output_json_bytes` | 4MiB actual typed serialization |

Input byte size is checked before hashing/parsing. The parser constructs the supplied bounded document before traversal; visited-node/depth limits do not promise a preallocation limit or fixed process RSS. CSS matching uses scraper's selector engine and can itself walk relationships; the checked-call budget does not assert a strict linear cost for arbitrary CSS. The module's own inheritance bookkeeping uses the path stack instead of repeatedly scanning every text node's ancestors. The scalar output loop stops storing after its cap while incremental observed SHA/counting continues within traversal budgets. Per-crawl storage/retention quotas and admitted-task persistence remain integration work.

## Verification boundary and integration acceptance

Twenty meaningful unit tests are authored in the module, including ten independently authored JSON selection fixtures. They cover document order, nested/duplicate includes, excluded ancestors, template fragments, parser recovery, attributes/visibility facts, Unicode entities/spacing, empty versus missing, decode/source uncertainty, separate raw/rendered identity, root language, UTF-8 exact caps, traversal/check budgets, a400-level DOM, deterministic hashes/roundtrip, malformed strict shapes/declarations, and actual byte budgets. Own-file rustfmt and fixture JSON parsing passed. Cargo/module exposure, executable integration, HTTP persistence/recovery/recrawl and browser/API/CLI proofs await the coordinated B8 integration gate; no executed unit or vendor-parity claim is made here yet.

Before integrated capture claims: verify default disabled legacy serialization; config failure before HTTP; raw/rendered status/language/source-revision isolation; page+capture+ledger/frontier transaction and generation binding; retained parent versus refreshed/pending selective-recrawl captures; input/output/storage quota evidence; hash-bound export/import validation; explicit corpus/eligibility/freshness selection; saved complete/incomplete/missing text passed into offline content analysis with zero website requests. Full vendor parity remains `NOT_PROVEN`.
