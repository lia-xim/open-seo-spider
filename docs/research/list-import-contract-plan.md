# Offline URL-list import: implementation contract proposal

Prepared 2026-10-12. This is a proposed independent contract and acceptance plan, not an implemented feature or a vendor-parity result. No Rust module, dependency manifest or application handler was changed for this preparation. The robots browser gate precedes implementation of this slice.

## Scope and primary observations

The original application's documentation lists TXT, CSV, XML, XLS and XLSX inputs, scans HTTP/HTTPS-prefixed values, and describes a separate ordered export that retains original URLs, duplicates and fix-ups. Those observations justify retaining original input identity; they do not establish the original parser's behavior for ambiguous sheets, malformed rows or formula caches. Our first importer will cover explicitly selected TXT/CSV/sitemap XML/XLSX inputs. XLS and heuristic scanning of every spreadsheet cell remain separate acceptance work. Importing bytes never starts a crawl or changes crawl settings. [Official List Mode documentation](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/#list-mode).

The sitemap protocol distinguishes page locations in `urlset/url/loc` from sitemap-document locations in `sitemapindex/sitemap/loc`. An imported index must therefore expose sitemap candidates, rather than pretend that index entries are page seeds. No index entry, schema location, entity or alternate URL is fetched by the importer. [Sitemap protocol](https://www.sitemaps.org/protocol.html).

## Proposed pure public interface

The parent owns API request decoding, route/static integration and later crawl hand-off. This module accepts in-memory bytes, not a filesystem path or a URL:

```rust
pub fn import_url_list(
    bytes: &[u8],
    options: &ListImportOptions,
) -> anyhow::Result<ListImportResult>;

pub fn inspect_xlsx_list(
    bytes: &[u8],
    limits: &ListImportLimits,
) -> anyhow::Result<ListImportInspection>;
```

Inspection lists bounded sheet metadata in workbook order, including visible/hidden/very-hidden status and worksheet versus non-worksheet type. It does not guess a URL column, read an entire dense worksheet or invoke macros. Only an explicitly selected worksheet may be imported.

`ListImportOptions` contains a tagged `ListImportFormat` and `ListImportLimits`. Tagged variants make contradictory format options invalid rather than silently ignored:

| Format variant | Explicit selection |
| --- | --- |
| `Txt` | One URL candidate per physical line; no header inference |
| `Csv` | Delimiter enum comma/semicolon/tab, column selector and positive `start_row` |
| `Xml` | Sitemap `urlset` or `sitemapindex`, detected from a validated root and namespace |
| `Xlsx` | Sheet name or one-based workbook index, column selector, positive `start_row`, formula policy |

The column selector is either a positive one-based index or an exact header name plus positive header row. A missing or duplicated matching header is an explicit selection error. Header values are not trimmed or case-folded implicitly. `start_row` means the first data row to include; it must follow the selected header row. All source row and column coordinates are one-based externally.

Example proposed options JSON:

```json
{
  "format": {
    "kind": "xlsx",
    "sheet": { "kind": "name", "value": "URLs" },
    "column": { "kind": "header", "name": "URL", "row": 1 },
    "start_row": 2,
    "formula_policy": "reject"
  }
}
```

The default formula policy is `Reject`. An explicit `CachedStringOnly` policy accepts only an already stored string cache and records that it came from a formula; it never evaluates or recalculates a formula. A missing, numeric, Boolean, date or error cache remains an invalid row. Shared formula anchors and derived placements retain their distinct metadata; no fabricated expanded formula text is provided.

## Output identity and errors

`ListImportResult` should include an engine/version label, input SHA256 and byte count, exact effective options, ordered rows, sheets/selection metadata where applicable, `website_requests: 0`, warnings and explicit limitations. Hashes cover submitted bytes, including original encoding/BOM/line endings, not a normalized reconstruction.

Every `ListImportRow` should carry:

| Field | Meaning |
| --- | --- |
| `source_index` | One-based ordinal in emitted output order, including invalid and duplicate rows |
| `source_row` | TXT physical line, CSV logical record, XML entry ordinal or XLSX worksheet row |
| `source_line` | CSV/XML starting physical line if measured; otherwise null, never inferred |
| `sheet` / `cell` | Exact sheet name and A1 cell coordinate for XLSX; otherwise null |
| `raw_value` | Exact decoded selected value, before URL normalization; XML entity decoding and XLSX shared-string decoding are identified as decoding, not raw markup |
| `cell_type` | String, empty/missing, integer, float, Boolean, date/time, error, formula-cache type or plain text/XML source |
| `formula_metadata` | Nullable normal/shared-anchor/shared-derived facts, bounded formula text only where actually supplied |
| `candidate_kind` | `PageUrl` or `SitemapDocument`; an index entry remains `SitemapDocument` even if malformed |
| `normalized_url` | Nullable HTTP/HTTPS URL using the existing crawler normalization function |
| `fragment` | Nullable parsed fragment evidence, retained separately when crawl normalization removes it |
| `error` | Nullable explicit per-row error; no invalid row is silently removed |

Duplicate values and duplicate normalized URLs remain separate rows. The importer must not deduplicate, reorder query parameters, apply discovery rewrites or turn invalid values into empty successful seeds. It need not assign a duplicate flag to retain duplicates correctly.

Only absolute HTTP/HTTPS URLs with a host are accepted. Raw control characters, userinfo/credential-bearing URLs, relative paths, FTP and other schemes produce row errors. Original values remain local evidence; they are never interpolated into HTML or fetched. Formula-safe CSV exports use the existing shared escaping function. Whole-document encoding/syntax/selection/budget failures return an error, with no claim that a partial import succeeded.

## Parsing choices

TXT and CSV initially require strict UTF-8 with an optional leading UTF-8 BOM. Other encodings fail explicitly. TXT preserves interior blank lines as invalid rows and removes only the final line terminator, not a substantive final blank row. CR, LF and CRLF cases need separate tests.

CSV uses independent strict bounded record framing so interior blank records survive, with explicit delimiter, no implicit headers or trimming, and flexible field counts so a missing selected field is a row error rather than discarding the document. Quoted commas, escaped quotes and quoted newlines remain a single logical record. The existing `csv` 1.4 dependency supplies escaped output and independent export decoding in tests; ordinary `records()` input alone would not prove preservation of blank records. Raw selected field identity is the decoded CSV value, not the enclosing quote syntax. [CSV ReaderBuilder API](https://docs.rs/csv/1.4.0/csv/struct.ReaderBuilder.html).

XML uses the existing direct `quick-xml` 0.42 dependency and namespace-aware events. Only immediate sitemap-namespace `loc` children of the expected entry parent are candidates; image/video extension locations and alternate-link attributes must not become page seeds. UTF-8, prefixed/default namespace equivalence, entity escapes and CDATA are covered explicitly. DTD/entity declarations, wrong roots, malformed XML and ambiguous multiple `loc` children fail explicitly. Non-location metadata can be ignored without pretending to validate a complete sitemap schema. [Namespace-aware reader API](https://docs.rs/quick-xml/0.42.0/quick_xml/reader/struct.NsReader.html).

XLSX imports logical worksheet rows in increasing coordinate order. Preflight rejects duplicate cell coordinates and non-monotonic row/cell order rather than silently sorting a malformed worksheet into a different input history. Missing selected cells and interior missing rows between `start_row` and the last actually observed worksheet row are represented as empty row errors, subject to the row budget. Declared worksheet dimensions alone do not manufacture millions of trailing rows. Merged cells are not forward-filled; hidden rows/cells are ordinary observed input unless a future explicit policy is added. Hyperlink targets do not substitute for cell text. Display number formats do not convert numeric/date cells into URLs.

## Dependency and allocation facts

The candidate dependency is `calamine = { version = "=0.36.1", default-features = false }`, MIT licensed. Its manifest declares Rust 1.88, `zip` 8.6 with deflate, and `quick-xml` 0.41 with encoding. The workspace's existing direct XML 0.42 dependency should be retained; a second transitive XML version is acceptable, not a reason to downgrade the crawler. Optional pictures/chrono are unnecessary. [Pinned upstream manifest](https://raw.githubusercontent.com/tafia/calamine/v0.36.1/Cargo.toml).

Use `Xlsx::worksheet_cells_reader(name)` and one method consistently: `next_cell_with_formula_metadata()`. It returns position, cached/literal value and normal/shared/derived formula metadata in XML stream order, avoiding dense `Range` construction and shared-derived formula text expansion. Dimensions are declared metadata, not a safe allocation size. [Streaming cell API](https://docs.rs/calamine/0.36.1/calamine/struct.XlsxCellReader.html), [upstream formula records](https://raw.githubusercontent.com/tafia/calamine/v0.36.1/src/xlsx/cells_reader.rs).

Constructing `Xlsx` eagerly reads package relationships, shared strings, styles and workbook metadata. In particular, its shared-string reader reserves capacity from `sst@uniqueCount`. A tiny ZIP/XML payload with an enormous declared count is therefore not controlled by a compressed-byte cap or subsequent cell iterator alone. Preflight must bound that attribute before constructing the workbook. This is a dependency-allocation observation, not evidence of the original application's behavior. [Pinned upstream XLSX constructor and shared-string parser](https://raw.githubusercontent.com/tafia/calamine/v0.36.1/src/xlsx/mod.rs).

## Proposed positive input budgets

Every configured bound must be a positive integer no greater than a fixed documented ceiling. Initial defaults can equal the ceilings below; callers can lower them. All accounting uses checked arithmetic. Exceeding a bound rejects the whole import before handing off seeds; no truncation is called success.

| Limit | Proposed ceiling | Enforced scope |
| --- | ---: | --- |
| `max_input_bytes` | 8 MiB | Submitted bytes, before any decoding/decompression |
| `max_rows` | 50,000 | Emitted rows including invalid, duplicate and synthesized interior empty rows |
| `max_field_bytes` | 8,192 | Decoded selected value, header, formula and relevant XML attribute/text field |
| `max_zip_entries` | 1,024 | ZIP directory count before workbook construction |
| `max_zip_entry_bytes` | 8 MiB | Actual inflated bytes for each entry |
| `max_zip_total_bytes` | 32 MiB | Actual inflated bytes summed across all entries |
| `max_xml_events` | 500,000 | Total parser events across XML preflight, shared strings/styles/relationships and selected worksheet |
| `max_xml_depth` | 64 | Nested XML elements |
| `max_shared_strings` | 100,000 | Both declared uniqueCount and observed `si` records |
| `max_sheet_cells` | 200,000 | Observed cells in the selected worksheet, including unselected columns |
| `max_csv_fields` | 200,000 | Decoded CSV fields, including unselected columns and headers |
| `max_output_bytes` | 8 MiB | Retained decoded selected/header values and formula text; shared strings cannot multiply output allocation without a bound |
| `max_sheets` | 128 | Workbook sheet inventory |

These are resource guards, not licence caps. They should be calibrated with measured memory/time once implemented; the plan does not claim them as universal large-workbook throughput proof.

ZIP preflight runs entirely in memory. It checks standard central-directory counts before allocations driven by those counts, and explicitly rejects ZIP64 and multi-disk containers in this initial bounded contract. It also rejects encryption/password protection, unsupported compression, symlinks, duplicate normalized paths and invalid package paths, and never extracts an archive to disk. For each entry it reads at most the remaining positive budget plus one byte and checks actual inflation and CRC completion, not just declared sizes. ZIP entry metadata exposes encrypted state and declared sizes, but those values alone are not decompression proof. [ZIP file API](https://docs.rs/zip/8.6.0/zip/read/struct.ZipFile.html).

XML preflight bounds actual events, nesting and fields before calamine's eager parsers. It also validates allocation-driving declared shared-string counts, cell coordinates and selected-sheet row span. Internal relationships must resolve to existing bounded package entries. External hyperlinks or external-book relationships are recorded as unsupported/ignored facts; external package/workbook/sheet targets are errors. No external relationship, macro or formula is executed. Strict `.xlsx` content types are enforced; XLSM/XLSB/XLS/ODS acceptance is not implied by calamine supporting other formats.

## Required executable acceptance cases

1. TXT: original order, duplicates, Unicode host/path, query order, fragments, whitespace normalization evidence, interior blank rows, bad scheme/userinfo/control/relative value, BOM and CR/LF/CRLF endings.
2. CSV: explicit index/header, duplicate/missing header, quoted comma and quote, multiline selected value retained but invalid as URL, ragged/missing selected field, empty records, duplicate URLs and formula-like strings without execution. Source record and starting line are checked independently.
3. XML: default/prefixed namespaces, escaped query ampersand, duplicate pages, blank/missing/multiple location, malformed URL, DTD/unknown entity, wrong root/namespace, malformed/truncated document, image extension location excluded. Sitemap index rows are `SitemapDocument`, with no page-seed list fabricated.
4. XLSX: two visible plus hidden sheets, exact name/index selection, selected header/index, inline/shared/rich strings, missing selected cell/interior row, duplicate URLs, non-URL cell types, errors, normal/shared/derived formulas, missing cache and explicit cached-string policy. Original A1 coordinates and row order are asserted. Hyperlink display text and external target never substitute for URL text or issue a request.
5. Bounds: every limit zero/above-ceiling rejected, compressed input cap, actual ZIP inflation cap, oversized central-directory/ZIP64 count, huge uniqueCount in small XML, oversized formula/shared string, hostile cell coordinate/dimension, event/depth/cell/sheet caps, duplicate/case-colliding paths and duplicate/out-of-order cells. Failure cannot silently return a truncated valid list.
6. End-to-end: a real browser uploads all four independently authored fixtures to the Rust API; duplicate, invalid and sitemap-index identities survive exact JSON and escaped CSV exports; a counted owned fixture website and external-link sentinel receive zero requests. Client byte/selection validation cannot bypass server bounds. Deferred file reads/import responses must not overwrite a newer selection or edit.

The original application's corresponding edge-case behavior remains `NOT_PROVEN` until an ordinary bounded reference run or UI observation supplies direct evidence. No acceptance case in this plan is a claim of complete vendor parity.
