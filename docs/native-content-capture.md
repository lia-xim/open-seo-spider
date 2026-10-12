# Native source-bound content capture

`CrawlConfig.content_capture` is optional and defaults to absent. Its absence is omitted from serialized config JSON, preserving legacy immutable-config hash inputs. Enable it when creating a new crawl; adding or changing the selection on an existing saved crawl is rejected. Captures retain selected normalized text, source hashes and selection evidence, not the full HTML document. Changing CSS after a crawl therefore requires another capture or separately supplied HTML.

Example:

```json
{"content_capture":{"sources":["raw_http_html","rendered_dom_html"],"include_selectors":["main"],"exclude_selectors":["script","style","nav","footer"]}}
```

Sources are explicit and kept independently in `PageRecord.content_captures`. Requesting rendered text without a configured renderer produces an unavailable rendered observation. A failed or unverified browser response never substitutes raw HTTP text. Native live crawl policy and the browser renderer remain an unsupported combination and fail before website requests.

Each wrapper uses `open-seo-spider/native-content-binding-v1` and records the exact requested URL, actual source document URL, fetch/render timestamps, configured selection SHA, source revision and optional observed SQLite generation. The revision binds the source/result/config/provenance tuple. This checks internal consistency; it is not authentication of arbitrary imported JSON or externally rewritten databases. In-memory runs deliberately leave `observed_generation` unknown. Imported snapshots cannot establish a native run generation by declaring a number.

Generation evidence assumes the controlled writer and its atomic commits. Ordinary recovery requires a positive capture generation within saved run history; selected current rows also match the exact target commit generation. Coherently rewriting payloads and all their binding hashes does not establish or disprove external authenticity. A generation number alone must never label an imported or inherited observation as freshly fetched.

Raw HTML SHA and byte length describe the actual character-decoded UTF-8 string supplied to the parser. The legacy-shaped fields `encoded_body_sha256`, `encoded_body_bytes` and `encoded_body_complete` describe **captured decompressed HTTP entity bytes before character decoding**, not gzip bytes, packets or original wire bytes. The actual decoder name accounts for a BOM override. Invalid byte sequences produce lossy evidence; unsupported declared charset labels use the existing UTF-8 fallback with unknown decode evidence. A literal valid U+FFFD is not automatically a decode error. Browser HTML is the worker's DOM serialization; it has no native entity-byte or character-decoder facts.

Language comes only from that source's root HTML `lang` attribute under the independent bounded syntax profile. Raw `de` and rendered `en` remain separate. Missing, empty or unsupported language attributes remain unknown and never borrow the other source's language.

The independent selection engine retains its parser/whitespace/selection profiles. Complete nonempty and complete empty results require a successful known HTML response, exact decode, complete selection and no contradictory reason. Non-200/truncated/lossy/unknown-decode sources preserve observed partial text as incomplete. Robots denial, absent/native non-HTML data, unknown browser MIME and failed browser responses are unavailable. A missing browser timestamp makes supplied rendered HTML incomplete. Raw successful zero-byte HTML yields an observed empty selection rather than absent HTML.

Config syntax and positive resource limits are validated before HTTP. DOM traversal/text budgets return explicit incomplete evidence. Oversized HTML inputs or serialized result budgets produce bounded per-source errors and warnings while the ordinary page observation still commits. The parser is bounded by supplied input bytes; the DOM visitation budget is not a parser memory reservation or hard process RSS limit. There is no fixed URL-count limit.

SQLite commits the source captures as part of the page payload in the same transaction as frontier completion, selected-target state and any native policy occurrences. A failed transaction publishes none of those source facts. Pause drains already admitted work; crash recovery retries uncommitted requests and binds new captures to the new generation. Read-only inspection and explicit resume reject inconsistent captures before advancing run state. Retained unselected records preserve their parent payload; selected pending records have no current page, and a fresh failed selection cannot inherit old complete text. These operations remain mixed-age selected coverage.

Verification uses original local HTTP fixtures for charset/BOM/gzip representations, partial/error/unknown states, strict pre-request rejection, separate mocked browser sources, SQLite-only pause/crash recovery, capture tampering, immutable legacy settings, selected replacement and transactional rollback with policy. The ignored test `live_browser_worker_captures_actual_raw_and_rendered_sources_and_selected_freshness` requires the real loopback Chrome worker on port 8790. Its result must be reported separately from default tests. These checks establish this capture contract, not universal parity with a proprietary SEO spider.
