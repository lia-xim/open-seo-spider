# Saved content in the local UI

The **Inhalte** view reads optional, persisted HTML text samples and analyzes an explicitly selected cohort. Saving capture settings, opening saved metadata or text, changing analysis parameters, calculating results and downloading their JSON make **zero website requests**. A new crawl or selected recrawl remains a separate action that can contact websites.

This view uses the independent capture and analysis contracts in [content-capture.md](content-capture.md) and [content-analysis.md](content-analysis.md). Reference observations in [reference-content-capture.md](reference-content-capture.md) remain separate from acceptance of this implementation. Full vendor parity is `NOT_PROVEN`.

## Future crawl settings

**Text-Erfassung** is off by default. With it off, the browser omits `content_capture` from the crawl request; historical reports without capture observations remain missing, rather than becoming empty text.

The optional JSON editor accepts only `sources`, `include_selectors`, `exclude_selectors` and `limits`. For example:

```json
{
  "sources": ["raw_http_html", "rendered_dom_html"],
  "include_selectors": ["body"],
  "exclude_selectors": ["script", "style", "template", "noscript", "nav", "footer"]
}
```

`raw_http_html` and `rendered_dom_html` are separate sources, without fallback between them. Selecting rendered capture does not silently enable the browser worker; rendering is enabled separately. An unavailable rendered source stays unavailable. Each source retains its own document URL, generation, revision, selection/config/text hashes and root `<html lang>` observation. The UI never substitutes effective page language, word counts, metadata, extraction values, PDF previews or raw text for another source.

The browser checks JSON shape, known fields, source choices and declared numeric/byte limits. Rust remains responsible for actual CSS compilation and authoritative pre-request validation. Saving settings does not re-extract old documents: the original complete HTML/DOM is not retained as an offline replay input. Text selection is the documented ancestor-membership/document-order profile, without computed visibility, layout or inline image-alt insertion.

## Bounded saved inventory and explicit selection

The UI requests at most **200 metadata rows** per page, using `after_ordinal` and the returned `next_after_ordinal`. Search, filtering, sorting and the filtered CSV apply to the currently loaded metadata page. There is no automatic load-all or automatic first-128 selection.

Checkboxes select the exact saved `requested_url`, not a final redirect URL or normalized replacement. Selection order is retained across pages of the same snapshot. At most **128** unique requested URLs are admitted, or a lower configured `max_documents`; an attempted 129th selection is explicitly rejected. Source selection is explicit. A source change retains the selected identities but invalidates results and rereads source metadata; a project or snapshot change clears selection.

Columns distinguish source status, nullable text bytes, language/reason, complete sample, capture generation/revision, hashes and observation basis. Statuses are `missing`, `complete`, `empty`, `incomplete`, `unavailable` and `error`. Missing capture, failed/unavailable source and observed `text: ""` remain different. Zero ordinal, zero bytes and generation zero are retained; `null` remains unknown.

The snapshot records `typed-crawl-report-serde-json-v1`, its SHA256, nullable generation and origin. `native_saved_report` means that the app matched a validated native checkpoint to that typed report projection; a path alone is insufficient. `declared_imported_report` remains a declared report, without authenticated network provenance. A native selected child can inherit samples from an imported parent: `parent_source_kind` and `parent_snapshot_sha256` remain visible, with `imported_snapshot` explicitly labelled as declared. Native child origin does not attest those inherited captures.

Selected recrawl basis comes from operation metadata: `refreshed_selected`, `inherited_parent`, `pending_selected` or `unknown_operation`; full captures use `captured_in_crawl`. Text completeness does not mean a recent fetch. Pending selected historical samples are not exposed as current text, and retained samples are not relabelled as newly captured.

## Plain-text source inspector

Opening a row requests exactly one saved source. The response must match the selected source, requested URL, snapshot SHA and nullable generation. The text uses `textContent`, so captured markup is inert. Available empty text has an explicit empty-string message; absent text shows its reason instead.

The UI recognizes the native binding engine plus the capture engine, selection, whitespace, parser and HTML-hash representation profiles. Unknown profiles withhold the interpreted text; the exact JSON remains downloadable. The JSON contains the full returned source wrapper, nullable capture result/failure and capture evidence. The DOM parser profile is an independent recovered HTML5 tree, not a browser layout or original byte-offset claim. Encoded raw body hashes and supplied decoded UTF-8 HTML hashes remain separate.

## Selected offline calculation

**Ausgewählte Texte analysieren** sends the complete ordered selection, source, analysis configuration and explicit filters. `indexable_only` and `complete_only` are opt-in. With both off, missing and incomplete sources remain included as nullable/partial inputs; the UI does not silently discard them. With filtering enabled, each excluded row and its reason remain visible. A valid all-filtered selection returns `empty_filtered_cohort`, `analysis: null` and its exclusions; no score or empty complete analysis is invented.

Analysis JSON accepts `case_behavior`, `stopwords`, `similarity`, `readability` and `limits` under the pure contract. The default editor value `{}` resolves native defaults. Bounded 1-, 2- and 3-gram tables retain exact counts and separately sampled one-based positions. Pair tables distinguish observed exact-shingle-set Jaccard, nullable near-duplicate classification and exact supplied-text identity; Jaccard 1 does not mean byte identity. Incomplete sample comparisons can have an observed score with unknown classification.

Readability shows the declared per-source EN/DE language, formula, count evidence, versioned syllable heuristic, nullable score and reason. Scores remain unclamped, including negative values and values over 100. Unsupported language, unsupported tokens, missing/empty/incomplete text and short samples remain unknown. No language detection, grade, comprehension or search-ranking prediction is implied.

The result table renders at most **200 rows** per display page. Its search/sort applies to the already bounded returned result, without new website requests. Unknown analysis/token/similarity profiles produce no derived table values; unknown readability profiles withhold the score. Exact adapter and pure result JSON remain downloadable.

## Snapshot and request guards

Inventory, source inspection and calculation have separate abortable tokens bound to project ID, browser project generation, selected source and input revision. Project changes, crawl controls, source/config/filter/cohort changes and explicit cancellation invalidate older operations. A successfully refreshed inventory with a different snapshot invalidates outstanding source/analysis responses and clears selection. Late success, error and `finally` callbacks cannot replace a newer result or unlock its pending controls.

For calculation, `expected_generation` is taken only from the saved inventory snapshot. A known zero is sent as zero; a nullable generation is omitted. It is never coerced from job status or inferred from `fetched_at`. Returned snapshot identity, ordered cohort, filter values, included/excluded identities and pure document/pair IDs must agree before display. Active jobs cannot be analyzed through this saved-view flow.

The browser cancellation control stops adopting that operation's response. It does not promise to interrupt a bounded server calculation that has already begun. No server-side crawler is started by analysis.

## API and downloads

| Action | Local API |
| --- | --- |
| One metadata page | `GET /api/crawls/{id}/content?after_ordinal=0&limit=200&source_kind=raw_http_html` |
| One exact saved source | `GET /api/crawls/{id}/content/document?url={encodedRequestedUrl}&source_kind=raw_http_html` |
| Explicit selected analysis | `POST /api/crawls/{id}/content/analyze` |

All envelopes declare `website_requests: 0` and `site_wide_coverage: false`. Inventory metadata declares `text_included: false`; only the document/analysis actions load selected text. The filtered inventory CSV includes snapshot origin/hash/generation, observation basis, source fields and parent provenance. Strings starting with formula prefixes, including leading whitespace, receive a leading apostrophe; typed numbers and booleans stay typed in representation. JSON preserves exact data without this CSV transformation.

Source and analysis JSON downloads preserve returned arrays, nulls and values through JSON serialization. The snapshot SHA describes the typed report representation, not byte-identical original imported JSON. The project download remains the separate full report action.

## Verification boundary

`web/verification/ui-content-smoke.cjs` passed **18** isolated browser/API fixture groups on 2026-10-12, including same-project snapshot changes with older and newer simultaneously pending source/analysis response bodies. The existing general UI fixture passed **47** checks on the frozen source. Both reported no browser runtime errors. These authored fixtures do not execute native capture or analysis.

`web/verification/ui-content-live.cjs` passed **12** actual browser → native crawler → saved capture → selected analysis groups on 2026-10-12T16:56:35.526Z, using an owned local HTTP fixture and the existing local Chrome worker. [Its result](../web/verification/ui-content-live-results.json) records the actual preview, fixture, crawl identities, exact checks and empty browser-error array. Website counters remained unchanged during analysis and saved-source changes. The gate also verifies invalid native CSS rejection before website HTTP/job creation, retained/refreshed capture hashes, imported nullable generation and 320px control reachability.

Related actual Policy8, SelectedRecrawl9, CrawlControl10 and Renderer14 gates passed on the same native Windows debug executable, for **53** actual browser groups total. Its SHA256 was `bcb59848694ef3b5181e428a783c7f2b1fa990c8baac3ded47a9befca29df115`. Task-work evidence is archived in `work/content-analysis/b8-ui-runtime-proof.json` with individual artifact/source hashes. The first content attempt failed because the test read `innerText` from a closed JSON-details panel; the successful rerun opened its summary before asserting the visible exclusions. No embedded product change or native rebuild occurred between those attempts.

These are controlled local runtime checks, separate from immutable release installation, CI and full vendor parity. The user's app on 8787 and worker on 8790 were not restarted by these gates.
