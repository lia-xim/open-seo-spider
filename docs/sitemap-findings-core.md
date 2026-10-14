# Full-stored sitemap finding kernel

Status: authored source, **NOT_COMPILED / NOT_RUN / NOT_INSTALLED**. This is an
independent local predicate contract. Manufacturer equivalence remains
**NOT_PROVEN**. The installed application still uses the previously accepted
Windows release; this document does not grant acceptance to the new source.

The work implements the Core portion of S6. API, CLI, MCP, browser presentation
and the distinct full finding export remain required follow-up work. Existing
raw inventory routes and raw CSV are separate interfaces.

## Selection domains

| Finding | Actual stored domain |
| --- | --- |
| URLs in sitemap | Original requested saved page identity |
| URLs not in sitemap | Qualified current or historical negative membership |
| Orphan URLs | Listed original and zero captured nonself anchor occurrences |
| Non-indexable URLs in sitemap | Listed original and qualified target evidence |
| URLs in multiple sitemaps | Distinct original XML source task URLs |
| XML sitemap with over 50K URLs | Real measured XML source attempt |
| XML sitemap over 50MB | Real measured XML source attempt |

Each candidate has `match`, `not_matched` or `unknown` and typed reasons. An
unknown result is not a negative control. Alias/final/rendered URLs do not replace
the original membership identity. An observed orphan candidate concerns the
captured report graph; it does not establish an orphan across an entire website.

Known original HTTP redirects, non-200 responses and actual robots Disallow can
establish independent obstructions. A failed robots fetch, incomplete target,
unqualified false indexability flag, incomplete PDF analysis or browser-only
navigation does not supply the same evidence. The new saved-page candidate
includes actual typed PDF/robots qualification fields without changing the
legacy saved-page finding DTO.

Source findings reuse the existing measured protocol assessment. A committed
document is attached only through its exact stored association. An A-only capture
remains visible without inventing a document commit. Legacy XML documents and
actual pending/done tasks without measurement have explicit unknown evidence.
Robots tasks are not XML attempts. Missing history and a global capture omission
qualify scope; they do not fabricate individual measured attempts.

## Core read contract

`CrawlStore::with_verified_sitemap_findings` creates a private-field borrowing
session within the existing actual verified SQLite read transaction. Configuration,
raw ledger, source capture/commit links, selected parent, coverage and saved report
are verified before a caller can visit results. In the available saved-page
domain, a separate full-report pass validates all typed target/PDF/robots facts
before the public context/callback, including excluded rows before an `after`
position or beyond the first result batch. The session exposes context and
repeatable complete-stored visits, not a SQL connection or a proof constructor.
An owned result returns only after the read transaction has closed. A callback
must not reenter the same Store while its connection lock is held.

The original rich header/profile is retained. A separate findings context binds
the versioned engine, canonical typed query, actual verified header and source
scope. Actual current task bytes/state are hashed even when a historical raw
catalog is unavailable. A stale expected context fails before cursor lookup and
before the user callback. Invalid query/domain/hash syntax is rejected before
database access.

`CrawlStore::sitemap_findings_page` applies predicates and exact original-identity
selectors before visible pagination. Limits are 1–200 returned rows and 1–5,000
evaluated candidates per work window. These bounds do not limit the session's
full visitor. The collector checks its work budget before applying the finding
predicate. Verification, protocol projection and a page batch of at most 200
can precede that check; the counter is not a SQL, wall-time or RSS budget.
Sparse work-window stops return actual examined progress and an
explicit continuation, including when zero matches have been returned. A known
selected lookahead resumes after the last returned match. Neither stop invents
an exact total or silently claims exhaustion.

The result distinguishes unavailable analysis (`rows: null`) from an evaluated
empty selection. A source result with unknown history retains that qualification
even after all stored candidates are exhausted. Exhaustion describes stored
candidates, not remote completeness.

Canonical query hashing and JSON size checks write into counting/hash sinks,
without collecting complete serialized queries/results. The complete Core page
has an inclusive 8MiB serialized limit. Legacy task input retains its actual
aggregate 32MiB inventory bound; a valid large row that cannot fit the output
budget produces an explicit error instead of silent omission. Existing selected
compatibility materialization and SQLite query working storage remain separate
costs. No RSS or hard cancellation-latency bound has been measured.

Unmeasured task continuation ordinals refer to the ordered eligible unmeasured
XML task set. They are not remote attempt numbers or positions in an unrelated
robots/full frontier list. The complete versioned task-state fingerprint and
query binding invalidate these positions when their actual context changes.

## Acceptance still required

- Compile the exact frozen candidate and execute the complete workspace with the
  owned renderer, required test names, format and all-target Clippy.
- Execute actual saved-page/source pagination, sparse windows, A-only/commit,
  legacy/unknown, selected-parent, integrity, cancellation and read-only cases.
- Connect API, CLI, MCP and UI to this shared contract and add the separate
  complete finding export, including strict domain validation and late errors.
- Verify actual native binary/API/SDK/CLI/browser behavior, narrow/wide keyboard
  use, stale request handling, output limits, recovery and physical source limits.
- Keep the existing raw inventory/CSV compatibility checks and the larger source
  matrix, measured performance, Linux/container and manufacturer parity gates open.

Authored tests and source reviews are preparation evidence only. They are not
execution receipts and do not increase accepted feature counts.
