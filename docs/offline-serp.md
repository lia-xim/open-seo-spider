# Offline metadata editor

Open `/serp` in the local application. The editor accepts UTF-8 CSV with URL,
Title and Description columns, including supported English/German aliases.
It preserves row ordering, duplicate URLs, quotes, Unicode and embedded newlines.
URLs are labels: preview/import/export make zero requests to entered websites.

The browser keeps an editable draft in localStorage and offers an exact JSON
download. CSV output includes source-row positions, Unicode scalar counts and
approximate local pixel widths. Unsupported glyphs/shaping have unknown widths;
an empty text has a known zero width. The fixed font/scale profile is documented
in [text calibration](research/text-width-calibration.md). This preview cannot
predict a search engine's font, text selection, layout or truncation.

The API accepts up to 10,000 rows, 65,536 UTF-8 bytes per field and eight MiB of
text. Malformed or ambiguous columns and exceeded limits are explicit errors.
CSV protects formula-like string cells with a leading apostrophe; JSON drafts
preserve original input values exactly. The table displays 100 rows per page.

`POST /api/serp/preview` and `/api/serp/export.csv` take `{ "rows": [...] }`,
where each row contains `url`, `title` and `description`. `/api/serp/import`
takes `{ "csv": "..." }`. `web/verification/ui-serp-live.cjs` exercises the
real browser and Rust endpoints and includes an owned HTTP fixture that must
receive zero requests. No proprietary SERP/project format compatibility is
claimed.
