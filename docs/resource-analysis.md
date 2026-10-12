# Native PDF and image observations

`PageRecord.pdf` and `image_metadata` are additive snapshot fields. Legacy
snapshots without them remain unknown; imports do not manufacture a new parse.
Separate views and `pdf.csv`/`images.csv` endpoints display captured observations.

PDF inspection uses public [lopdf 0.45](https://docs.rs/lopdf/0.45.0/lopdf/)
APIs and originally written bounded traversal/text processing. `CrawlConfig.pdf_analysis`
defaults to enabled and exposes page, object, decompressed-content, text and
annotation limits. Parsing concurrency is two. Limits and parser errors remain
visible in `status`, `parse_error`, `errors`, `limits_reached` and per-page text
observations. These are bounded parser operations, not a global memory/CPU
isolation guarantee.

The default bounds are 1,000 pages, 100,000 objects, 8 MiB decompressed content,
4 MiB total decoded text, 1 MiB decoded text per page and 10,000 URI annotations.
The `pdf_analysis` configuration validates positive values before website requests.
States `complete`, `partial`, `error`, `encrypted` and `disabled` remain distinct.
Disabled inspection is not a successful empty PDF, and a declared page-tree
count is not substituted for a verified count.

The record preserves version, observed and declared page counts, encryption,
Info metadata, text previews/hashes and URI annotation links with page numbers.
Only normalized HTTP(S) URI targets enter normal crawl discovery and scope/robots
rules. Unsupported actions remain observational rather than executing. A PDF
URI base is resolved as evidence; files, scripts and arbitrary destinations are
never executed by PDF inspection.

Text means decoded supported page text operators. Unsupported font mappings,
Form XObjects, corrupt streams and exhausted bounds produce incomplete text;
aggregate text counts remain unknown instead of a false zero-word result.
Encryption is explicit; there is no password guessing, OCR, PDF rendering,
layout reconstruction or complete PDF conformance validation. A complete parse
does not prove that every visible character in an arbitrary PDF was extracted.

Raster images use [imagesize 0.15](https://docs.rs/imagesize/0.15.0/imagesize/)
encoded-header inspection. Stored width/height are encoded raster dimensions,
not declared `img` attributes, CSS display sizes or guaranteed EXIF-oriented
display dimensions. The parser does not decode every image pixel or validate
all animation/integrity constraints.

SVG parsing uses bounded XML events/depth and explicit absolute lengths in
CSS pixels. Percentages, automatic sizing, viewport defaults, a viewBox alone,
and stylesheet-dependent sizes do not establish intrinsic width/height. A
viewBox remains a distinct field. External references are never fetched by this
metadata parser. Captured-body truncation and malformed input remain explicit.

Resource CSV exports preserve errors and unknown numeric cells; known zero
values remain zero. Metadata JSON is exact; spreadsheet-oriented string cells
use the shared formula protection. HTML image occurrence tables still preserve
each source page's own alt text and declared dimensions.

Reproducible gates: `crates/core/tests/resource_fixtures.rs` exercises real binary
HTTP, discovery and SQLite roundtrips; `web/verification/ui-resource-live.cjs`
exercises actual UI/API/native parse, filters, exports, legacy unknowns and
disabled configuration. Results are recorded separately after executing those
gates; these scripts do not assert reference-product parity.
