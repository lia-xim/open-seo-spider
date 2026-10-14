# Declared sitemap sources and lastmod assessment

The existing sitemap read API now has two separate derived projections. Raw catalog rows, original declaration identities, stored hashes and the ten-column CSV stay unchanged.

## Source cohort

`coverage.derived` reports `complete`, `known_empty`, or `unknown`. Its scope is either current declared sources, inherited historical declared sources, or unavailable. It checks the actual initial declarations, required Robots origins, every captured discovery task, complete HTTP/decode/parser observations and recursively admitted index edges. Cycles and duplicate references do not cause repeated traversal, but their original occurrences remain visible.

The inventory and validated catalog are captured in one SQLite read transaction. A task's Done flag does not replace a source observation. Missing, pending, denied, malformed, truncated or non-200 source documents prevent completion. A pending Robots task prevents a premature known-empty result. Initial policy admissions and ordinary sitemap-only origins determine required Robots sources.

Completed source capture permits only a narrow absence statement among valid original normalized Page Loc values from that captured declared cohort. Invalid or missing Page Loc values remain separate counted observations. The result does not establish full website coverage, a complete crawl, fresh inherited evidence or an arbitrary invalid raw URL's absence.

Selected recrawls can retain a captured parent inventory bound to exact immutable parent bytes. Its scope is historical. Older parents without the inventory remain unknown. Incompatible inherited inventories are rejected instead of being repaired into completeness.

## Lastmod values

`lastmod_assessments` contains assessments of Entry rows in the returned page. Unavailable history returns null; an available page without Entry rows returns an empty array. Each assessment retains the raw field occurrence, ordinal, XML byte span and exact precision. XML Schema whitespace collapse supplies the evaluated value without changing the source.

The named profiles are the Sitemap 0.9 `tLastmod` union of XML Schema 1.0 Second Edition `date|dateTime`, and the six precision levels in W3C NOTE-datetime. A value can be valid for one profile and invalid for the other. Duplicate elements, invalid calendar dates, incomplete entry prefixes and undetermined leap seconds are explicit. No timezone, freshness or target HTTP status is inferred.

The interface provides loaded-page filters for invalid values, profile disagreement, undetermined values, multiple elements, optional absence and incomplete entries. A separate inspector shows the assessment; the original row inspector preserves its original JSON.

## Snapshot and limits

The API snapshot state hash binds the catalog summary, inherited basis, actual captured inventory, derived source projection and assessment engine. A context-only change invalidates continuation and export tokens even when row revision and rolling hash remain unchanged.

The complete JSON response remains bounded at 8 MiB. A page exceeding the limit fails atomically; a smaller page can be requested. The full raw CSV remains bounded at 64 MiB and contains no extra derived columns. Reading these projections performs zero website requests.

## Remaining work

- Join original memberships to actual page observations and inlink evidence, retaining missing, pending, historical and rewritten identities.
- Produce named redirect, non-200 and non-indexable sitemap target findings.
- Complete live generation race and physical exact-fit/one-byte-over budget acceptance.
- Extend sitemap generation with lastmod, image/hreflang entries and segmented index output.
- Run Linux/container and GitHub gates when the account becomes available; local acceptance is a separate gate.

Primary definitions: [Sitemaps protocol](https://www.sitemaps.org/protocol.html), [Sitemap schema](https://www.sitemaps.org/schemas/sitemap/0.9/sitemap.xsd), [XML Schema 1.0 Second Edition](https://www.w3.org/TR/2004/REC-xmlschema-2-20041028/), [W3C NOTE-datetime](https://www.w3.org/TR/NOTE-datetime).

These are independent implementation contracts. Full Screaming Frog behavior parity remains NOT_PROVEN.
