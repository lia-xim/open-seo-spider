# Requested Windows store paths

Local acceptance passes on the normal executable `0a2f40e9e9cf4bbfd9ba0baf00120a1127fb2cb2b71b353e2c573d24cd483f46`.
Installed Source16, user crawl stores and installed ports remain unchanged.

[local-result.json](local-result.json) records actual results and raw evidence hashes.
[source-gates.json](source-gates.json) binds the source before compilation and after runtime acceptance.

## Observed failure and change

The published b37/25fc baseline captures two pages at 214 UTF-16 units.
At 255 units it leaves a zero-byte database and fails before website requests;
at 300 units it fails without creating the main database. All three owned
baseline processes actually close with EOF0. A zero-byte file alone does not
identify a failed SQLite call. Separate direct legacy controls observe the
255-unit connection opening successfully and its WAL/core-schema batch failing;
at 300 units connection opening fails and initialization is not attempted.

Writable and read-only CrawlStore connections now share a Windows adapter.
Ordinary filesystem paths use explicit Windows extended syntax and the
connection-local SQLite win32-longpath VFS. No OS setting, global VFS or
fallback location changes. Existing flags and URI/memory/temp/device/legacy
name semantics remain intact. Already-verbatim paths preserve their bytes.
The source archive continues to derive its authority from the original caller
path. Opening failures retain the failed stage, SQLite message/code and typed
underlying cause in the first message displayed by the app.

## Actual gates

| Gate | Result |
| --- | --- |
| Focused native tests | Five Windows path tests plus three common special-name/error tests pass |
| Ordinary Rust workspace | 1,780 passed, zero failed, five deliberate ignores |
| Explicit Chromium | All four named ignored Rust browser fixtures actually execute |
| Positive enrichment | Separate cfg(test) binary, owned HTTP provider and two actual cold processes; production providers remain separate |
| Formatting/build/Clippy | Formatting, normal build and all-target/all-feature Clippy with denied warnings pass |
| Existing UI regression | All 37 selected suites pass in one invocation with unchanged tracked inputs and normal binary |
| New Windows native/browser gate | All five cases pass in 17512 ms |

The current UI invocation excludes segments-repeated-metadata. Its separate
[previous-commit acceptance](../large-segment-issues-20261018/README.md) remains
historical; it is not a current-binary rerun or an all-38 current pass.

The new gate uses exact SQLite paths of 214, 255, 300 and 1,150 UTF-16 units
and a 650-unit Unicode path of 1,270 UTF-8 bytes. Every case genuinely captures
two pages and a sitemap. Complete native JSON and all 42 CSV columns, actual
browser downloads, original XML archive bytes and checkpoint state survive
two real native process openings of the same store and origin.

Ten native processes close naturally with EOF0. All 42 read-only CLI processes
close: 32 successes and ten expected missing/invalid-input refusals. There are
20 verified browser downloads, 20 archive-body HTTP downloads and ten positive
nonce-protected HTTP410 controls. The website remains offline from before the
first close through both closes, with zero product refetches. Independent
artifact review checks physical identities, checkpoints and all 420 CSV cells.

Native tests also exercise real WAL/SHM files and cold read-only stores at
1,200 units, Unicode and already-verbatim paths. Read-only failures create no
new database/parent/output; malformed original bytes remain unchanged.

## Source and proof boundaries

The pre-build manifest pins 493 source/test inputs. All compiled product
inputs remain unchanged. Independent review led to one authorized test-only
driver change after compilation: real table selectors, the Node legacy-name
oracle, explicit checkpoint facts and bounded failure cleanup. Runtime pins
all 493 final inputs; the broad Rust/browser coordinator pins the same 492
inputs excluding only that separately reviewed standalone driver. Both failed
focused attempts are retained: the authored COUNT type error and the lexical
UNC-prefix error found by the first actual test run.

Node filesystem lookups of legacy trailing-dot/space aliases return ENOENT
on this host while SQLite accepts those aliases. Their Node file ID is not
claimed. Exact saved SQL rows/context, original database hash and absence of
additional files remain asserted; ordinary/verbatim/relative/junction aliases
retain direct file-ID comparisons. The owned junction control proves only
SQLite/raw-row identity. Archive traversal retains its nofollow policy.

Live UNC shares, ambiguous long legacy aliases, universal 32k namespace
coverage, power-loss durability, release/installation and universal resource
guarantees remain unproven. The full ledger remains 144 features, 416 filters
and 32 reference workflows, with 38 implemented/69 partial/37 planned.
Full manufacturer parity remains NOT_PROVEN. Fresh exact-commit public CI is
a separate pending gate; the Windows matrix now runs this native/browser gate
and uploads its evidence.
