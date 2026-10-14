# Source protocol measurement: S3 implementation contract

Status: **IN PROGRESS / COMPILE AND RUNTIME NOT RUN**. This work continues S3 of
[the filter-depth plan](sitemap-filter-depth-plan.md). It does not complete S4
large-source processing, S5 physical acceptance or S6 public filters.

The implementation is developed in `codex/sitemap-source-protocol-20261014`,
separately from the pending generation/target candidate and its frozen218-input
source. The installed accepted Windows runtime remains unchanged. The original
design audit is pinned by SHA256
`172c2f35706827e373c1e6440d8a75b5189d059fdbcbc3e0ee55c3029839f6d5`.

Source milestone: the pure model/projector, observed decoder and shared strict
counter are merged with28/11/7 authored test functions. The combined workspace
format check passes. Compilation and all46 new tests remain NOT RUN; the future
workspace minimum is892, including the earlier generation/target candidate.
Independent static review found three DTO contradictions; v2 corrects truncated
gzip, known Identity/no-output and the event-budget sentinel. Static closure is
separate from executed acceptance. Fetch collection and durable/public projection
have not been implemented in this milestone.

The count and decoded-byte ceilings follow the published
[Sitemaps protocol](https://www.sitemaps.org/protocol.html). Protocol validity,
captured measurements and retained membership completeness are separate axes.

## Required behavior

1. Persisted measurements describe an actual captured attempt and its effective
   limits. They do not establish current website state or complete membership.
2. Every byte/count quantity states its domain and whether it is exact, a lower
   bound or unavailable. Known zero and unavailable remain different values.
3. Count immediate entry starts, completed entries, actual document-namespace
   `loc` occurrences and eligible completed single-`loc` entries separately.
   Missing-`loc` diagnostics are never invented URL occurrences. Two valid
   entries with the same URL count twice for the entry limit.
4. A source URL-count excess requires more than50,000 eligible completed entries.
   A50,001st entry start alone is insufficient. A complete structural count at
   or below50,000 can prove the count ceiling; diagnostics still qualify syntax
   and URL validity. Index children remain separate from page entries.
5. A decoded-byte excess requires more than52,428,800 bytes actually returned by
   the applicable decoder. A lower bound may establish excess even when a later
   checksum fails. It cannot establish exact full size or successful integrity.
   A bounded prefix at/below the ceiling cannot establish a full-source ceiling.
6. Decoder failure preserves the count/hash of successfully returned bytes.
   Bytes written into a buffer by a failing `Read` call are not returned evidence.
   True EOF, all-member gzip completion, checksum failure, truncation, nested
   gzip and unavailable decoding remain explicit. Header sizes do not replace
   actual measurements.
7. The count-only path shares the strict XML recognizer and namespace/entity
   grammar with the existing parser. It retains bounded active state rather
   than every source entry. Existing retained-mode rows, errors, field caps,
   hashes and admission behavior remain unchanged.
8. Binding preserves actual legacy task/request/final identities. New protocol
   eligibility rules for a `loc` must not repair or reject an otherwise admitted
   task identity. Content-addressed measurement IDs and expected document IDs
   are consistency/provenance checks, not authentication of external data.

## Ordered completion gates

- [ ] Pure typed model, strict bounded input validation and independent count/
      decoded-byte threshold projection; exact/lower-bound/unknown controls.
- [ ] Failure-preserving entity decode observations; existing decode API and
      all historical compatibility tests preserved.
- [ ] Count-only strict scanner;49,999/50,000/50,001 actual entry fixtures,
      independent golden counters and malformed/namespace/retention controls.
- [ ] Current fetch-layer received-versus-retained byte facts with explicit
      HTTP decoding and incomplete/non200/denied scope.
- [ ] Bounded versioned measurement storage, guarded by the same source task,
      generation and config. Observation persistence survives a later full
      membership transaction rollback without marking the source done or
      admitting any target.
- [ ] Same-transaction reads, absent historical schema, report/inherited basis,
      measurement-only revision changes and cursor invalidation.
- [ ] Combined format/Clippy/workspace and source-bound native acceptance after
      the coordinated machine window is released. Authored tests are not passes.
- [ ] Per-criterion ledger and issue updates using actual accepted evidence.

S4 remains necessary for genuine protocol-sized decoded sources and practical
durable membership paging. Current5/8MiB input limits and32MiB retained-catalog
limits are resource policies. This slice does not silently raise them. Actual
52,428,799/52,428,800/52,428,801-byte plain/gzip/index acceptance remains open,
as does complete Screaming Frog parity.
