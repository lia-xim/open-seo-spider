# Local reference comparison

Run: 2026-10-12T07:44:00.559Z. Full product parity: **NOT PROVEN**.

35 reference rows / 34 independent rows; 863 matching fields, 53 differing fields, 3 unavailable fields.

Fixture SHA-256: `928545b1b70ae07da917fddd0fd1fd137878e7819326741dbbd6c23cb4607072`. Native binary SHA-256: `986a7f8534692e37fb78dda322b6c4fcbea60bd712aaa04a3824c7b68b860a3f`.

The original normal CLI reported a missing licence and successfully exported eleven CSV tabs for this controlled fixture. The original CLI rewrites its ordinary profile on shutdown; no licence/config/auth flag or setting change was supplied.

Comparisons are field-level observations. They do not prove issue-filter membership, paid features or complete behavior equivalence.

Metric-profile correction: the first run compared all native references, including three IMG occurrences, to reference anchor outlinks. This run counts anchors for the comparable field and retains total references/kinds separately. Initial-response and final-document metadata differences remain visible.

| Path | Field | Result | Reference | Independent |
| --- | --- | --- | --- | --- |
| / | title_pixels | DIFFERENT | 491 | 495 |
| /Upper_Case | title_pixels | DIFFERENT | 598 | 602 |
| /baseline | title_pixels | DIFFERENT | 565 | 570 |
| /baseline?a=1&b=2 | title_pixels | DIFFERENT | 565 | 570 |
| /blocked | indexability_reason | DIFFERENT | "von robots.txt gesperrt" | "blocked by robots.txt, requested url does not return http 200, not a supported indexable document type" |
| /blocked | title_pixels | UNAVAILABLE | 0 | null |
| /body-metadata | title_pixels | DIFFERENT | 141 | 142 |
| /canonical-404 | indexability_reason | DIFFERENT | "canonicalized" | "canonical points to another url" |
| /canonical-404 | title_pixels | DIFFERENT | 617 | 619 |
| /canonical-conflict | indexability_reason | DIFFERENT | "canonicalized" | "canonical points to another url" |
| /canonical-conflict | title_pixels | DIFFERENT | 649 | 652 |
| /canonical-other | indexability_reason | DIFFERENT | "canonicalized" | "canonical points to another url" |
| /canonical-other | title_pixels | DIFFERENT | 629 | 633 |
| /canonical-self | title_pixels | DIFFERENT | 615 | 619 |
| /duplicate-a | title_pixels | DIFFERENT | 365 | 368 |
| /duplicate-b | title_pixels | DIFFERENT | 365 | 368 |
| /empty-description | title_pixels | DIFFERENT | 651 | 656 |
| /header-noindex | indexability_reason | DIFFERENT | "noindex" | "noindex directive" |
| /header-noindex | title_pixels | DIFFERENT | 629 | 633 |
| /image.png | indexability | DIFFERENT | "indexable" | "non-indexable" |
| /image.png | indexability_reason | DIFFERENT | "" | "not a supported indexable document type" |
| /image.png | title_pixels | UNAVAILABLE | 0 | null |
| /images | title_pixels | DIFFERENT | 556 | 561 |
| /jsonld-invalid | title_pixels | DIFFERENT | 607 | 612 |
| /jsonld-valid | title_pixels | DIFFERENT | 591 | 596 |
| /lang-de | title_pixels | DIFFERENT | 557 | 561 |
| /lang-en | title_pixels | DIFFERENT | 557 | 561 |
| /links | title_pixels | DIFFERENT | 532 | 536 |
| /missing | indexability_reason | DIFFERENT | "client-fehler" | "requested url does not return http 200" |
| /missing | title | DIFFERENT | "" | "Real missing page" |
| /missing | title_occurrences | DIFFERENT | 0 | 1 |
| /missing | title_length | DIFFERENT | 0 | 17 |
| /missing | title_pixels | DIFFERENT | 0 | 163 |
| /missing-description | title_pixels | DIFFERENT | 665 | 670 |
| /missing-title | title_pixels | UNAVAILABLE | 0 | null |
| /multiple-description | title_pixels | DIFFERENT | 667 | 673 |
| /multiple-title | title_pixels | DIFFERENT | 601 | 609 |
| /noindex | indexability_reason | DIFFERENT | "noindex" | "noindex directive" |
| /noindex | title_pixels | DIFFERENT | 562 | 565 |
| /redirect-one | indexability_reason | DIFFERENT | "umgeleitet" | "requested url does not return http 200" |
| /redirect-one | title | DIFFERENT | "" | "An independently authored local SEO reference fixture /baseline" |
| /redirect-one | title_occurrences | DIFFERENT | 0 | 1 |
| /redirect-one | title_length | DIFFERENT | 0 | 63 |
| /redirect-one | title_pixels | DIFFERENT | 0 | 570 |
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
| /server-error | title_pixels | DIFFERENT | 0 | 130 |
| /soft-error | title_pixels | DIFFERENT | 573 | 579 |

Machine-readable explanations are in `reference-comparison.json`; exact independently authored cases are in `fixture-cases.json`.

Run `node scripts/reference-parity.mjs` to compare both engines on the same server. Set `SPIDER_BINARY` for a freshly built executable. `--reuse-reference` compares against retained reference CSVs without executing the original again.
