# Offline list imports

`crates/core/src/list_import.rs` parses submitted bytes without fetching URLs, opening files, changing crawl settings, following spreadsheet links or evaluating formulas. It preserves occurrence order, duplicate values and invalid rows. Importing a list and starting a crawl are separate actions. Its independent parser contract is not a claim of complete Screaming Frog list-mode parity.

The public functions are `import_url_list(&[u8], &ListImportOptions)`, `inspect_xlsx_list(&[u8], &ListImportLimits)`, `validate_import_result(&ListImportResult)` and `export_url_list_csv(&ListImportResult)`. They return `anyhow::Result`. The engine identifier is `open-seo-spider/offline-list-v1/calamine-0.36.1`.

## Input and selection

The HTTP transport accepts `POST /api/lists/inspect` with `{ "content_base64": "...", "limits": {} }` and `POST /api/lists/import` with `{ "content_base64": "...", "options": { ... } }`. Inspection returns `ListImportInspection` directly and needs no sheet or column selection. Import returns `ListImportResult` directly. Transport integration and browser proof are separate gates from the parser tests below.

Examples of `options`, with omitted limits using the ceilings below:

```json
{"format":{"kind":"txt"}}
{"format":{"kind":"csv","delimiter":"comma","column":{"kind":"header","name":"URL","row":1},"start_row":2}}
{"format":{"kind":"xml"}}
{"format":{"kind":"xlsx","sheet":{"kind":"name","value":"URLs"},"column":{"kind":"index","value":2},"start_row":2,"formula_policy":"reject"}}
```

`delimiter` is `comma`, `semicolon` or `tab`. Column selectors are `{"kind":"index","value":2}` or `{"kind":"header","name":"URL","row":1}`. Sheet selectors are an exact name or `{"kind":"index","value":1}`. All row, column and sheet indexes are one-based. Header matching is exact and requires one matching header; ambiguous or missing selection fails the document. A header row must precede `start_row`, which is currently bounded by `max_rows`. Sheet inspection reports actual workbook order, `WorkSheet` type and `Visible`, `Hidden` or `VeryHidden` metadata. It does not guess a URL column.

| Format | Occurrence and parsing rules |
| --- | --- |
| TXT | Strict UTF-8 with optional UTF-8 BOM. CR, LF and CRLF delimit physical rows. Interior blank rows remain unknown occurrences; a final line terminator adds no artificial row. |
| CSV | Same UTF-8 rule. Explicit delimiter and column selection. Quoted delimiters, escaped quotes and multiline fields are supported. Blank records and ragged rows remain occurrences. Malformed quoting fails the document. `source_row` counts logical records; `source_line` records the physical starting line. |
| XML | Strict UTF-8 and the sitemap 0.9 namespace. Only direct `urlset/url/loc` or `sitemapindex/sitemap/loc` values are candidates. An entry with no `loc` remains unknown; multiple direct `loc` values fail the document. Image and alternate extension locations do not become page seeds. DTDs, unknown entities and malformed XML fail. |
| XLSX | Standard bounded ZIP package, then streaming selected worksheet cells. Shared, rich and inline strings are decoded. Missing selected cells, including sparse gaps and explicit empty final rows, remain occurrences. Declared worksheet dimensions do not create a dense allocation or invent trailing rows. Stored numeric, boolean and date lexemes remain typed unknown values. |

A sitemap index produces `candidate_kind: "sitemap_document"`, while a URL set and non-XML formats produce `"page_url"`. An index entry is never silently converted to a page seed. Neither inspecting nor importing downloads referenced sitemap documents.

Raw values retain their decoded input identity. URL interpretation trims surrounding whitespace, rejects raw control characters and accepts only absolute HTTP/HTTPS URLs with a host and no username/password. Normalization uses the crawler's existing URL function: fragments are saved separately, query order and encoded path facts are retained, and normal URL-library host normalization applies. Invalid, empty, relative, credential or unsupported-scheme values have `normalized_url: null`, `fragment: null` and an explicit `error`.

## Formula and provenance contract

Formula policy defaults to `reject`. The explicit `cached_string_only` option accepts a formula cell only when it contains a usable stored string URL cache. It never evaluates, expands, recalculates or verifies freshness of that cache. Normal and shared formula metadata are retained; shared-derived metadata retains the shared index rather than fabricating the expanded expression. Unsupported formula metadata, missing caches and non-string caches stay unknown. Formula headers are unsupported. Hyperlink labels are never replaced by external relationship targets.

Each result stores the complete effective options, exact submitted byte count and SHA-256, selected sheet/column where applicable, warnings, limitations and `website_requests: 0`. Each row stores `source_index`, `source_row`, nullable `source_line`, nullable sheet/A1 cell, `raw_value`, `cell_type`, nullable `formula_metadata`, candidate kind, nullable normalized URL/fragment and error. XML and XLSX physical line numbers are `null` because they were not measured.

Results transmitted by a client are **client-provided provenance, not attested artifact authenticity**. Once original bytes are discarded, a declared SHA-256 cannot prove the source document, header match, formula cache freshness or original XML root. `validate_import_result` checks the supported engine, zero website-request declaration, positive bounded input bytes, SHA shape, options, row count/order, positive coordinates, format-specific sheet/cell/type shape, formula-policy consistency and accumulated metadata limits. Successful normalized URLs and fragments must derive from the same raw value; error rows cannot also claim a URL. It counts the actual serialized JSON through a bounded writer, including escaping, row objects and numeric metadata, without allocating an output-sized buffer. Unknown JSON fields are rejected. The crawler must run this validator before deriving page seeds or sitemap documents from supplied results.

CSV export also validates the result first. It preserves one record per occurrence and uses the existing shared formula-neutralization helper for textual fields. Its columns are Source Index, Source Row, Source Line, Sheet, Cell, Raw Value, Cell Type, Candidate Kind, Normalized URL, Fragment, Error and Formula Metadata. Formula metadata is encoded as JSON. The JSON result preserves raw text; CSV may prepend an apostrophe to a dangerous spreadsheet formula prefix.

## Resource boundaries

Every limit is positive, may be lowered and cannot exceed these ceilings. An exceeded budget fails the document; there is no silent truncation. These are resource guards and do not impose a paid URL allowance.

| Limit | Ceiling |
| --- | ---: |
| `max_input_bytes` | 8,388,608 |
| `max_rows` | 50,000 |
| `max_field_bytes` | 8,192 decoded bytes |
| `max_output_bytes` | 8,388,608 serialized result bytes |
| `max_csv_fields` | 200,000 stored CSV fields |
| `max_zip_entries` | 1,024 |
| `max_zip_entry_bytes` | 8,388,608 inflated bytes |
| `max_zip_total_bytes` | 33,554,432 actual inflated bytes |
| `max_xml_events` | 500,000 |
| `max_xml_depth` | 64 |
| `max_shared_strings` | 100,000 |
| `max_sheet_cells` | 200,000 |
| `max_sheets` | 128 |

ZIP central-directory counts and bounds are checked before constructing `ZipArchive`. Paths are checked for ambiguous, duplicate and case-colliding parts. Entry reads are bounded using actual inflation, independently of declared uncompressed lengths. XML events, depth, fields, shared-string counts, sheets and strictly ordered row/cell coordinates are checked before constructing Calamine. This matters because Calamine reads shared strings eagerly and can reserve capacity from `uniqueCount`. The selected sheet is consumed through `next_cell_with_formula_metadata`; dense `Range` APIs are not used.

ZIP64, multi-disk, encrypted, macro-enabled, non-UTF-8 XML packages and compression methods other than stored/deflate are explicitly unsupported. XLS, XLSB, ODS, all-cell heuristic HTTP scanning, formula evaluation, external-link substitution and recursive sitemap downloads are not implemented. Data in unknown workbook extensions is not promoted to URLs. These boundaries do not establish vendor-parser equivalence.

## Verification and sources

The isolated native module passed 19 focused unit tests and Clippy with warnings denied. Tests cover duplicates and source coordinates; blank/malformed values; CSV quoting and ambiguous headers; namespace-aware URL/index sitemap handling; ZIP directory corruption; huge declared shared-string counts; sparse rows; formula/cache types; hidden sheets; missing/duplicate XLSX headers; external relationships; duplicate/wrong cells; typed stored lexemes; configured limits; CSV escaping; and tampered transmitted results. A separate forged-size ZIP test understates both local and central uncompressed size but retains a large compressed string and proves the actual-inflation limit rejects it. Public fixtures, their generator, exact serialized examples and SHA-256 manifest are in `crates/core/tests/fixtures/list-import/`.

The integrated module also passed `cargo test --locked -p open-seo-spider-core --lib list_import::tests`: 19 passed, none failed or ignored, and 102 unrelated tests were filtered out. The fixture manifest records this bounded native gate and hashes. The separate [integrated batch](verification/list-import-batch.json), [11 actual API/HTTP/CLI checks](verification/list-import-e2e.json) and [14 actual browser checks](../web/verification/ui-list-import-live-results.json) now prove those owned workflows. A successful pure parser run does not itself prove them, and these bounded gates do not establish vendor parity.

## Crawl, source rows and command line

The browser lets the user inspect a workbook, choose its sheet/column/header and formula policy, then preview and accept the ordered rows before explicitly starting a crawl. Manual text uses the same native TXT parser. Editing the text or changing selection invalidates the previous preview; 12 deferred-state regressions cover stale responses. Browser textareas normalize line endings, while uploaded files retain the exact submitted bytes and hash.

Accepted results are stored as `CrawlConfig.input_imports`. The crawler validates them before deriving page seeds and sitemap-document inputs. It preserves all declared occurrences and deduplicates actual normalized page fetches. `GET /api/crawls/{id}/inputs.csv` exports one record per original occurrence, including errors and duplicates, joined to matching captured page observations. Unfetched or invalid inputs have no invented HTTP status. Client provenance is labeled `declared_client_input`. Old reports without these facts remain unknown in the source table.

`open-seo-spider import-list FILE --output import.json` parses offline; `--inspect` reports XLSX sheets and `--options options.json` supplies explicit selections. XLSX import requires an explicit sheet/column selection. `crawl --config config.json --list-file urls.txt --output crawl.json --input-csv inputs.csv` preserves saved settings unless an explicit CLI flag overrides them, performs the crawl and analysis, and writes the exports. Success exits 0, malformed input/configuration exits 1, and invalid command options exit 2. These paths are exercised with zero requests and no output for rejected commands.

Primary references: [Calamine 0.36.1 package](https://docs.rs/calamine/0.36.1/calamine/), [streaming XLSX cells and formula metadata](https://docs.rs/calamine/0.36.1/calamine/struct.XlsxCellReader.html), [Calamine eager XLSX parsing source](https://github.com/tafia/calamine/blob/v0.36.1/src/xlsx/mod.rs), [ZIP 8.6 entry reader](https://docs.rs/zip/8.6.0/zip/read/struct.ZipFile.html), [quick-xml namespace reader](https://docs.rs/quick-xml/0.42.0/quick_xml/reader/struct.NsReader.html), [Sitemap protocol](https://www.sitemaps.org/protocol.html), and [Screaming Frog list-mode documentation](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/#list-mode). Calamine is MIT licensed; package dependencies are pinned where newly added. The fixtures and parser are independently authored and contain no manufacturer files or assets.
