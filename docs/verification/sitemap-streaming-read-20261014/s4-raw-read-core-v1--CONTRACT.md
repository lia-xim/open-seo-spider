# S4 raw read increment: source-only handoff

Baseline: `E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-streaming-integration-v1`, Root HEAD `380beb1ccacf73659d0ac8dd7fdb39c543b88883`.

This revision contains exactly three product payloads: the existing ledger module with a child registration and shared per-row policy guard, the new private verified reader, and its new tests. Root alone integrates these files. No SourceStore, Store facade, App, schema, public export, default or retention limit is changed here.

## Crate-private API

`sitemap_ledger::verified_read` exposes:

- `verify_raw(&Connection) -> anyhow::Result<Option<VerifiedRawRead<'_>>>`.
- `VerifiedRawRead::summary() -> &SitemapLedgerSummary`.
- `VerifiedRawRead::inherited() -> Option<&SitemapInheritedBasis>`.
- `VerifiedRawRead::connection() -> &Connection`, the actual borrowed connection, for the independent Source reader's same-connection check.
- `VerifiedRawRead::document_by_id(&str) -> Result<Option<SitemapSourceDocument>>`, preserving actual sequence and all raw document facts.
- `VerifiedRawRead::visit_filtered(&SitemapLedgerFilter, impl FnMut(&SitemapLedgerRow) -> Result<()>) -> Result<()>`.
- `VerifiedRawRead::window(&SitemapLedgerFilter, after: Option<u64>, limit: usize) -> Result<SitemapLedgerPage>`.
- `SitemapLedgerFilter { kind: SitemapLedgerRowKind, source_observation_id: Option<String>, membership_url: Option<String> }`, with `validate() -> Result<()>`.
- `SitemapLedgerRowKind::{All, Root, Document, Entry, Location}`.

The caller owns an active read transaction, keeps it active, and does not write through the same connection while a token exists. Verification refuses autocommit; subsequent visitor/window calls also refuse an ended transaction. The token is neither constructible outside this module nor Clone. A transaction is the snapshot boundary; this API does not claim that a borrowed `Connection` statically prohibits writes or proves transaction identity after external end/restart misuse.

Filters preserve App's existing exact original-membership semantics: lowercase SHA-256 source identity; nonempty, at most 65,536 UTF-8 bytes, control-free exact global-normalizer URL spelling. No final-URL/alias substitution or additional protocol Loc lexical rules are introduced. A source filter matches document IDs and other rows' source IDs. A membership filter matches only root and Location admission original identities. `after` must name a retained row matching the same filter. Limits are 1..=1,000 rows and an actual 8 MiB complete serialized page cap; failure returns no partial page. A smaller valid page can recover. Public callers must bind continuation state separately.

## Verification and bounds

Before issuing any ordinary token, the reader checks the entire raw catalog: table presence, metadata and budgets, cross-table global IDs without assuming imported UNIQUE constraints, source task duplicates, canonical typed JSON and each SQL projection, payload SHA, exact global sequence and rolling SHA, document identities, source-before-child relations, actual per-document entry/Location/root ordinal order and counts, entry-to-Location multiset equality, admission/raw/span/alias rules, and the same committed-policy/original-root guards as the old loader. The policy guard extraction is shared with the old materializing reader. Entry declared Location arrays are sorted for multiset comparison as before; their input order is not tightened.

The exact existing `SitemapCatalog` serde envelope size is measured with a row iterator and the old counting writer, including metadata, punctuation and the byte-count field itself. It is not estimated from SQL payload sums. All tail rows are verified even when the caller later requests only the first row. A visitor propagates caller errors rather than reporting a complete scan.

The ordinary Rust working set is compact document counters/IDs plus a single document cache, a bounded row/related entry and a requested bounded page. The entry/Location relation check is one ordered join, not a full Location scan for each entry. Point lookups remain per Location and per distinct source switch. There is no full raw `Vec` or report allocation in this ordinary path. SQLite UNION/GROUP BY/ORDER BY work, its page/cache/temp-store behavior, serde temporary row copies and callback-owned allocations are separate from this Rust working-set description; no measured RSS or latency result is claimed.

Inherited anchors deliberately call the existing full `load_catalog` compatibility path, including its exact parent envelope and raw-catalog binding. It materializes the inherited catalog temporarily and is not the bounded S4 end state. Low-level `None` has the existing unavailable/uninitialized meaning; mandatory selected-parent presence/exact-plan guards belong to the higher Store reader and must run before exposing public results. SourceStore context, coverage, selected-plan/report validation, full CSV transport and App adaptation are separate slices, not proved by this raw increment.

## Authored evidence only

Fourteen regression functions are present. They cover initialized empty versus unavailable, active transaction, old full-loader oracle comparison, filters and matching continuations, visitor failure, corrupt tail, SQL projection tampering, imported schema cross-table duplicate IDs, global sequence and per-source ordinal reordering, unsorted valid versus wrong Location multisets, exact envelope size, missing entry/committed policy row, inherited exact BLOB compatibility, read-only bytes and WAL snapshot isolation, missing table without repair, an oversized SQL index projection, and the unchanged real 8 MiB page boundary with smaller-page recovery.

Only `rustfmt --edition 2024 --config skip_children=true` and its `--check` form were executed on the three authored files. No compiler, Cargo, SQLite, fixture generator, tests, HTTP, native App, browser, SDK or runtime process was executed. In particular the large-page fixture is authored source, not generated evidence. All compile/test/native gates are **NOT_RUN** under the compute hold. Root must review, integrate and execute the appropriate gates after release.
