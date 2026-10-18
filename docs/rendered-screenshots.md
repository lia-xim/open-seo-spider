# Rendered viewports and saved screenshots

Status: the independent selected-viewport and saved-PNG acceptance is fulfilled.
See [the completed local receipt](verification/rendered-screenshots-20261018/README.md).
Full manufacturer parity remains `NOT_PROVEN`.

## Capture configuration

`renderer.viewport` is optional. When absent, the worker keeps its existing
1280 × 800 default. An explicit value requires both integer dimensions:

```json
{"width": 320, "height": 240}
```

Each dimension must be between 100 and 4096 CSS pixels. Unknown properties,
arrays, incomplete objects, strings, fractional values and out-of-range values
are rejected. The viewport affects responsive rendering even when screenshot
capture is disabled. A selected viewport must match the worker's reported
dimensions before the rendered observation is accepted.

Old renderer configurations without this property retain their serialized
bytes. Editing one viewport dimension must preserve other renderer options,
including native-only response limits. Explicitly disabling the custom viewport
removes this optional property; it does not disable rendering.

## Saved image identity

The metadata endpoint is `/api/crawls/{id}/screenshot?url=...`. An original
zero-based `ordinal` disambiguates repeated requested URLs. Ambiguous URLs must
be refused rather than silently selecting their first occurrence.

The PNG endpoint is `/api/crawls/{id}/screenshot.png`. It requires the original
ordinal and URL, expected image SHA-256, expected snapshot SHA-256 and, for a
native saved generation, its expected generation. A snapshot token is the
SHA-256 of this compact JSON array:

```text
["open-seo-spider/screenshot-snapshot-v1", job_id,
 generation_as_decimal_string_or_null, typed_report_sha256]
```

The typed report digest uses the existing compact `CrawlReport` serialization.
Binding the token to the job prevents a token from an otherwise identical
imported report from selecting a different job. These are consistency tokens;
they do not grant authorization.

Image reads use an immutable cached report under its job read lock, with the
current generation and report digest. This is not a fresh SQLite attestation.
Neither endpoint fetches the original website or repeats browser rendering.
Missing, disabled, failed and corrupt images must be explicit outcomes.

The original PNG is exported as a no-store attachment, with byte length,
SHA-256 and snapshot identity headers. Preparation is bounded to two shared
profile/image operations. A PNG has a 64 MiB logical output limit, dimensions
of at most 4096 pixels per axis and at most 10,000 chunks. Base64, chunk bounds,
ordering, CRCs and dimensions are checked before export. This structural check
does not inflate or decode every pixel. Global RAM, CPU and transport-memory
bounds are not established by the per-artifact limit.

## Required end-to-end evidence

Use actual settings controls and the real Chromium worker to capture a
deterministic owned page at 320 × 240 and 1440 × 900. Decode the resulting PNGs
and compare dimensions and known pixel regions. Actual browser downloads must
match the original saved screenshot bytes and URL/snapshot identity.

Exercise missing images, stale and foreign contexts, late metadata/download
responses, preservation of native-only profile options and reachable controls
at a 320-pixel interface width. After a clean native close, a fresh process and
browser context must reopen the same reports and image bytes while the fixture
website remains an HTTP410 witness. Full vendor parity remains separate.
