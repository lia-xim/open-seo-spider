# Saved-report sitemap generation workflows

These routes use the same `open-seo-spider/segmented-sitemap-generation-v1` Core
generator and preserve the ZIP manifest's static saved-observation scope. They do
not fetch pages, publish sitemaps, or establish current indexability/freshness.
Selected-recrawl reports can contain observations from different times.

## CLI

```
open-seo-spider generate-sitemaps crawl.json --options sitemap-options.json --output new-bundle.zip
```

Options JSON uses the existing sitemap bundle API contract, including mandatory
useful `public_base_url`, independent URLset/index byte caps, optional extensions,
and explicit `omit`, `captured_sitemap`, or `provided` lastmod sources.

Options input is bounded to 64 MiB and report JSON to 256 MiB, both checked before
and during reading. Exact Core structural/date/reference validation runs before
report loading. Report parsing/generation run on a blocking worker. Existing Core
input/output budgets still apply (default complete ZIP 64 MiB; maximum 256 MiB).
This is saved report JSON input, not a live SQLite crawl/export operation.

The destination directory must already exist. Existing output files, directories
and links are refused, including aliases of source inputs. Complete generation
finishes before any temporary output is created. Bytes are written and synced to
an exclusively created random temporary file in the destination directory, then
published using a new hard-link name. Hard-link support is required. There is no
overwriting rename fallback. Concurrent attempts to the same name have one winner
and cannot expose partial bytes through the destination name. A process crash can
leave its random temporary file; filesystem-level durability across power loss is
not asserted. Cleanup failure after successful publication is an explicit warning
in the successful receipt, rather than a misleading claim that output failed.

Stdout emits one JSON receipt with actual output path, bytes, ZIP/report SHA-256,
engine, generated/excluded counts, and optional cleanup warning. Sources remain
unchanged. Errors use the CLI's existing nonzero error path. There is no `--force`
or implicit output path.

## MCP

`spider_generate_sitemaps` has exactly `{id, options}` arguments. It is read-only,
non-destructive, idempotent, and `openWorldHint=false`. It calls only the local
API `POST /api/crawls/{id}/sitemap-bundle`, never a website or artifact store.
The provider base must be canonical HTTP loopback at its root directory, without
credentials, query, fragment, controls, or normalization of supplied paths. The
usual missing trailing slash on a root base is accepted. Redirects and proxies
remain disabled.

All argument types, unknown nested keys, Core option semantics, provided date
lexemes, source references, canonical identities and duplicate identities are
validated before API I/O. Omitted `max_total_output_bytes` defaults to 8,388,608
for this transport. Explicit values above 8,388,608 are refused before I/O. This
differs deliberately from the API/CLI's larger default and is in tool metadata.
Serialized effective options also have an 8 MiB body bound.

Successful provider responses require `application/zip`, actual Content-Length,
the exact generation-engine header, the expected fixed UUID attachment filename,
valid SHA-256 headers and bounded generated/excluded counts. Both declared and
streamed lengths are bounded; actual bytes and SHA-256 must match. A ZIP local
file signature is checked, but MCP does not independently validate every XML
document or ZIP entry. API errors are retained as bounded explicit tool errors;
oversized bodies return `too_large`. Error response bodies are capped at 64 KiB.

The result's `structuredContent` contains `mime_type`, `filename`, `bytes`,
`sha256`, `report_sha256`, `engine`, `generated_urls`, `excluded_pages`,
`encoding="base64"`, `data_base64`, `artifact_stored=false`, and saved-report
`scope`. The attachment filename is a suggested download name, not a stored local
artifact. There is no invented download URL. `content` contains only a short
receipt; binary base64 appears once. The complete JSON-RPC envelope is checked
against 16 MiB, including the request ID. Oversized envelopes return a protocol
error retaining the original request ID whenever that bounded error fits 16 MiB.
Only a direct in-process ID too large even for the error falls back to a null ID.

Inventory becomes 25 tools when this draft and its generation API dependency are
promoted together. Existing script inventory assertions must change at that time,
not before promotion.
