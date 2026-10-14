# Segmented Sitemap generation

`POST /api/crawls/{id}/sitemap-bundle` consumes a captured report already available to the local application. It generates a local download; it performs no website requests, writes no crawl/report files, and publishes nothing.

The existing `GET /api/crawls/{id}/sitemap.xml` remains compatible. CLI and MCP integration are described in [sitemap-generation-workflows.md](sitemap-generation-workflows.md). The combined native/API/CLI/SDK/browser acceptance, optimized release and installation are still pending for this source revision.

```json
{
  "public_base_url": "https://example.org/",
  "max_urls_per_file": 50000,
  "max_urlset_uncompressed_bytes": 52428800,
  "max_index_uncompressed_bytes": 52428800,
  "max_total_output_bytes": 67108864,
  "include_images": false,
  "allow_external_images": false,
  "include_hreflang": false,
  "lastmod": {"source": "omit"},
  "changefreq": null,
  "priority": null
}
```

Both XML byte limits are independent, positive, and at most 52,428,800. The URLset count is 1–50,000; the index lists at most 50,000 segments. These protocol limits apply before compression. A location must contain fewer than 2,048 characters. [Sitemaps protocol](https://www.sitemaps.org/protocol.html)

The complete ZIP, including headers, must fit `max_total_output_bytes`, which is bounded by 268,435,456. Complete generation fails rather than returning partial XML or losing an oversized entry. Other explicit safety bounds are 1,000,000 report pages, 256 MiB streamed typed report serialization, 32 MiB supplied lastmod text, 4,096 bytes per supplied date/priority, and 1,024 bytes per supplied date reference.

The base must be a canonical HTTP(S) directory ending in `/`, without credentials, query, fragment, path traversal, or encoded separators. Generated files have fixed flat names. Selected page URLs must have the exact same scheme/host/port and a descendant path. A base `/sitemaps/` excludes captured pages under `/products/`; `/` covers the origin.

Every generator URL role checks percent-escape syntax before URL-parser normalization: `%` must be followed by two ASCII hexadecimal digits. Malformed option identities fail pure preflight; malformed captured page, image or hreflang URLs are omitted and diagnosed. Valid encoded octets and their recorded hex case remain unchanged, including `%20`, `%25` and encoded UTF-8 sequences. This lexical check neither decodes octets nor double-escapes them; XML entity escaping such as `&amp;` remains a separate step. The global crawler normalizer is unchanged. [RFC 3986, section 2.1](https://www.rfc-editor.org/rfc/rfc3986#section-2.1)

Candidates require recorded initial and final HTTP 200, no redirect/navigation alias, internal indexable document flags, and no recorded transport failure, truncation, or robots block. Images/stylesheets themselves are not emitted as document URLs. Render failures, missing configured HTML rendering, rendered non-200 observations, and rendered navigation to a different HTTP fetch identity are excluded and diagnosed. A canonical rendered URL may differ only by `#fragment` without becoming another fetch target; its raw stored value remains unchanged and bound by the report hash. Rendered paths, query values/order, scheme, host and port remain part of identity. Malformed, credential-bearing, control-containing or missing render URLs are excluded as incomplete observations; they are not silently repaired. The exporter consumes recorded indexability; it does not conduct a new audit or establish fresh availability.

Duplicate recorded requested URLs are excluded as ambiguous. Every emitted URL is bound to its exact captured report page ordinal and segment. Excluded candidates and extension omissions are diagnosed. If none are eligible, HTTP 422 returns structured `diagnostic_counts` and no artifact.

Image output uses captured image references and the image namespace. Each page supports at most 1,000 unique image references; excess fails without truncation. External images require explicit opt-in and remain labelled as lacking verification evidence. Deprecated image tags are not generated. [Google image sitemap documentation](https://developers.google.com/search/docs/crawling-indexing/sitemaps/image-sitemaps)

Hreflang output uses captured effective page declarations, the XHTML namespace, original language lexemes and source ordinals. It checks HTTP(S) identity, language/script/region grammar, self references, duplicate/conflicting declarations. It does not attest assigned ISO registry codes, reciprocal declarations, or destination availability. Conflicting or incomplete sets are omitted with diagnostics. [Google localized-version sitemap documentation](https://developers.google.com/search/docs/specialty/international/localized-versions#sitemap)

`lastmod` supports `{"source":"omit"}`, `{"source":"captured_sitemap"}`, or `{"source":"provided","values":[{"page_url":"https://example.org/a","value":"2024-02-29","reference":"editorial change record #17"}]}`. The default omits dates. Provided values require canonical identities, explicit references and valid Sitemap XSD date/dateTime values. Captured values require actual validated report Sitemap rows, a complete source and entry, exactly one location and date, and an unambiguous original identity. Date values are lexically assessed by the shared pure assessor. Supplied source references and original values, or captured document/entry/field coordinates, are retained in the manifest. These are source declarations rather than independently verified content modification times. HTTP `Last-Modified`, fetch times, current time, timezone conversion and freshness inference are not used.

`priority` accepts the bounded XML decimal grammar between 0 and 1, including leading zeros, `.5`, signed positive values, and negative zero. Exponents, nonzero negatives and values above 1 are rejected. The caller's lexical value is retained. `changefreq` accepts the protocol enum.

The successful response is `application/zip` with `Content-Length`, a fixed UUID-based attachment name and headers `x-sitemap-generation-engine`, `x-sitemap-report-sha256`, `x-sitemap-bundle-sha256`, `x-sitemap-generated-urls`, and `x-sitemap-excluded-pages`. The engine is `open-seo-spider/segmented-sitemap-generation-v1`. The report hash binds canonical typed Rust/serde JSON, not original imported file bytes.

The ZIP contains `sitemap-00001.xml` and further segments, `sitemap-index.xml`, and `manifest.json`. The manifest includes exact memberships, per-file uncompressed byte lengths and SHA-256, options/hash, captured report hash, diagnostics, explicit static report scope, and whether the report is a mixed-age selected recrawl. ZIP timestamps are fixed to 1980-01-01 for deterministic packaging; they are not content modification dates. XML index entries deliberately contain no invented modification times.

Errors have `{code,message,diagnostic_counts?}`. Invalid options/dates are 400, unavailable/invalid source is 409, input/output/entry/index bounds are 413, no eligible URLs is 422, internal ZIP failure is 500, and missing jobs remain 404. No failure produces an application download or partially saved bundle.
