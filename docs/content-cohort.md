# Analysis of saved source captures

The native opt-in capture records selected text from actual decoded raw HTML and,
when explicitly requested, the renderer's DOM serialization. Each source keeps
its own language, status, selection profile, byte/hash evidence and observation
generation. No capture is enabled by default. See [capture profile](content-capture.md)
and [the feature to-do list](FEATURE_TODO.md).

```json
{
  "seed_urls": ["https://example.test/"],
  "content_capture": {
    "sources": ["raw_http_html", "rendered_dom_html"],
    "include_selectors": ["main"],
    "exclude_selectors": ["nav", "footer", "script", "style", "template", "noscript"]
  },
  "renderer": {"endpoint": "http://127.0.0.1:8790/render"}
}
```

Raw capture works without a renderer. A requested DOM source with no successful
render stays unavailable. Raw and DOM text never substitute for one another.
The captured raw bytes have undergone reqwest transport decoding; they are not
compressed wire packets. Decoded HTML hashes refer to the actual supplied UTF-8
string. Lossy/truncated/non200 samples remain incomplete; empty successful
selection is distinguishable from missing source text. Capture-budget failures
produce a bounded analytical error without erasing an otherwise successful HTTP
response. Pages and captures use the same persistence transaction.

## Saved-content API

All routes require a stable report, reject an active generation, and make zero
website or renderer requests. The projection profile is
`open-seo-spider/saved-content-cohort-v1`.

| Route | Behavior |
| --- | --- |
| `GET /api/crawls/{id}/content?after_ordinal=0&limit=200&source_kind=raw_http_html` | Metadata-only page, maximum200 saved rows, explicit next ordinal. |
| `GET /api/crawls/{id}/content/document?url=<encoded exact requested URL>&source_kind=raw_http_html` | One source capture including its bounded text; unknown exact identities are refused. |
| `POST /api/crawls/{id}/content/analyze` | Explicit ordered unique URL cohort, optional filters and existing content-analysis configuration. |

```json
{
  "urls": ["https://example.test/one", "https://example.test/two"],
  "source_kind": "raw_http_html",
  "indexable_only": false,
  "complete_only": false,
  "expected_generation": 1,
  "config": {
    "similarity": {"shingle_size": 3, "threshold": 0.9, "min_tokens": 10}
  }
}
```

No arbitrary address is fetched. Duplicate/unknown URLs, unknown request fields,
invalid profiles, changed expected generation and resource-budget excess are
errors. The selected cohort is limited to128 documents and8MiB engine input by
default; lowering an explicit budget is supported. There is no silent first128
selection. Excluded identities and reasons stay in the result. An entirely
filtered cohort returns explicit `empty_filtered_cohort`, with `analysis:null`.
Invalid configuration remains invalid even when every row is filtered out.

Results bind to a SHA256 of the typed report under
`typed-crawl-report-serde-json-v1`. This differs from the original JSON file's
byte hash and does not authenticate arbitrary imported content. API results name
their saved/imported origin and job generation; the CLI names imported-file
origin and an unknown generation. A validator proves internal source/hash
consistency. Full source HTML is not retained by this slice, so it cannot replay
a changed CSS selection from a saved document or attest arbitrary imported text
as a network observation.

Recognized selected-recrawl targets use explicit operation state. Pending
targets never supply old parent text. Committed selected text must bind to the
target's committed generation. Unselected samples remain `inherited_parent`.
Unknown operation profiles get `unknown_operation`; matching text, URLs or
timestamps do not establish freshness. Complete inherited text is still old.

The finite explicit cohort is fully represented before optional filters, while
`site_wide_coverage:false` remains explicit. Complete pairs can be classified
under the independent shingle profile. Incomplete pairs retain nullable observed
similarity and unknown near-duplicate classification. Readability uses only the
captured source's root language and the declared EN/DE heuristic. Effective
post-render `PageRecord.language` is never a raw-text fallback. See
[engine boundaries](content-analysis.md) and
[original fixture observations](reference-content-analysis.md).

## Offline CLI and MCP

```sh
open-seo-spider analyze-crawl-content crawl.json --selection selection.json --output analysis.json
```

Omit `expected_generation` for a file-only CLI request: this command cannot
verify a live job generation. Input files and existing output destinations are
preserved. The engine result matches API analysis of the same typed source
snapshot and selection; context explicitly preserves origin/generation limits.

MCP adds `spider_content`, `spider_content_document` and
`spider_analyze_crawl_content` over both existing transports. These operations
are read-only local analysis. Native HTTP/CLI/API proofs, actual Chromium proofs
and browser state/race proofs are separate gates. Full reference feature,
filter and workflow parity remains **NOT_PROVEN**.
