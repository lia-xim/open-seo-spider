# Local reference comparison

Run: 2026-10-12T10:48:27.390Z. Full product parity: **NOT PROVEN**.

35 reference rows / 34 independent rows; 887 matching fields, 29 differing fields, 3 unavailable fields.

Fixture SHA-256: `928545b1b70ae07da917fddd0fd1fd137878e7819326741dbbd6c23cb4607072`. Native binary SHA-256: `266bcaf4e87c56d83347a288e7c2b7e7c8331a4796f2aaba9452471e5b774783`.

The original normal CLI reported a missing licence and successfully exported eleven CSV tabs for this controlled fixture. The original CLI rewrites its ordinary profile on shutdown; no licence/config/auth flag or setting change was supplied.

Comparisons are field-level observations. They do not prove issue-filter membership, paid features or complete behavior equivalence.

Metric-profile correction: the first run compared all native references, including three IMG occurrences, to reference anchor outlinks. This run counts anchors for the comparable field and retains total references/kinds separately. Initial-response and final-document metadata differences remain visible.

| Path | Field | Result | Reference | Independent |
| --- | --- | --- | --- | --- |
| /blocked | indexability_reason | DIFFERENT | "von robots.txt gesperrt" | "blocked by robots.txt, requested url does not return http 200, not a supported indexable document type" |
| /blocked | title_pixels | UNAVAILABLE | 0 | null |
| /canonical-404 | indexability_reason | DIFFERENT | "canonicalized" | "canonical points to another url" |
| /canonical-conflict | indexability_reason | DIFFERENT | "canonicalized" | "canonical points to another url" |
| /canonical-other | indexability_reason | DIFFERENT | "canonicalized" | "canonical points to another url" |
| /header-noindex | indexability_reason | DIFFERENT | "noindex" | "noindex directive" |
| /image.png | indexability | DIFFERENT | "indexable" | "non-indexable" |
| /image.png | indexability_reason | DIFFERENT | "" | "not a supported indexable document type" |
| /image.png | title_pixels | UNAVAILABLE | 0 | null |
| /missing | indexability_reason | DIFFERENT | "client-fehler" | "requested url does not return http 200" |
| /missing | title | DIFFERENT | "" | "Real missing page" |
| /missing | title_occurrences | DIFFERENT | 0 | 1 |
| /missing | title_length | DIFFERENT | 0 | 17 |
| /missing | title_pixels | DIFFERENT | 0 | 161 |
| /missing-title | title_pixels | UNAVAILABLE | 0 | null |
| /noindex | indexability_reason | DIFFERENT | "noindex" | "noindex directive" |
| /redirect-one | indexability_reason | DIFFERENT | "umgeleitet" | "requested url does not return http 200" |
| /redirect-one | title | DIFFERENT | "" | "An independently authored local SEO reference fixture /baseline" |
| /redirect-one | title_occurrences | DIFFERENT | 0 | 1 |
| /redirect-one | title_length | DIFFERENT | 0 | 63 |
| /redirect-one | title_pixels | DIFFERENT | 0 | 565 |
| /redirect-one | description | DIFFERENT | "" | "A useful independently authored description for the local fixture /baseline" |
| /redirect-one | description_occurrences | DIFFERENT | 0 | 1 |
| /redirect-one | description_length | DIFFERENT | 0 | 75 |
| /redirect-one | h1 | DIFFERENT | "" | "/baseline" |
| /redirect-one | word_count | DIFFERENT | 0 | 81 |
| /redirect-two | coverage | DIFFERENT | true | false |
| /server-error | indexability_reason | DIFFERENT | "server-fehler" | "requested url does not return http 200" |
| /server-error | title | DIFFERENT | "" | "Controlled 503" |
| /server-error | title_occurrences | DIFFERENT | 0 | 1 |
| /server-error | title_length | DIFFERENT | 0 | 14 |
| /server-error | title_pixels | DIFFERENT | 0 | 131 |

Machine-readable explanations are in `reference-comparison.json`; exact independently authored cases are in `fixture-cases.json`.

Run `node scripts/reference-parity.mjs` to compare both engines on the same server. Set `SPIDER_BINARY` for a freshly built executable. `--reuse-reference` compares against retained reference CSVs without executing the original again.
