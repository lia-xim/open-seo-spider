# XLSX native preparation, 2026-10-21

This folder is ignored preparation. No tracked file was edited and no native process, Chrome session, Cargo build or Git mutation was run by this agent. The package reader was checked against three existing pure-proof workbooks; the complete browser/native candidate has only been syntax checked.

Proposed tracked targets after root review:

- `web/verification/ui-table-xlsx-live.cjs`
- `web/verification/xlsx-package-oracle.cjs`

The candidate can be copied without changing imports: both its ignored and proposed tracked locations are two directory levels below the repository root. It uses the existing explicit normal binary/SHA and Chromium environment contract; `SPIDER_TABLE_XLSX_OUTPUT` selects a fresh owned output directory. Keep the existing 240000ms gate deadline for the first coordinated run. Do not raise it just to hide a failed assertion or synchronization error.

`plan.json` records exact selectors, raw-vs-CSV representations, ownership, independent source oracles, planned counts and the broader backlog. The initial API-started captures are genuine owned crawls, preserving the R2 corpus structure; the third one-page32768-title crawl is negative only. A saved PageRecord or CSV export alone is never used as a replacement for the actual downloaded workbook. Every positive workbook is read through an independent ZIP/CRC reader and a fresh `about:blank` Chromium `DOMParser` realm, with the expected complete typed matrix independently mapped from original captured records and fixture membership.

Root integration assumptions are `#export-xlsx`, visible `#table-export-status[data-phase=prepare]`, terminal `data-code`, cancel control `#table-export-cancel`, exact current filename and embedded `table-xlsx.js`/`table-export-ui.js`. The controller must genuinely yield after publishing prepare and before reading the projection. The post-product-handler race listener observes that phase and dispatches ordinary DOM controls. It does not replace RAF, Promise, timers, Blob, URL or App functions.

The candidate has30 paired XLSX/CSV files,60 browser download events,8 prepare races,1440/320 geometry and downloads, two native EOF0 processes, two explicit410 offline witnesses, zero website refetches, original API/disk report immutability and full row/cell/type/provenance checks. Those are planned assertions, not claimed results. Full AND and overlapping OR populations retain118 original rows; titles and statuses/responses retain152/153 rows, including empty, unknown, zero, NFD and equal-URL original response occurrences.

Default large row/cell/payload/package boundaries remain portable pure proof. Only a genuine qualifying corpus could establish their native behavior. The actual32768-title refusal covers a genuine native cell limit; the deliberately corrupt workbook copy covers the independent reader only. Python/openpyxl is not needed in CI. Its separate local reader observations and actual Excel GUI behavior must not be folded into a Chromium XML acceptance claim.
