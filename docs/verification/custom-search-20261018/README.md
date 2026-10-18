# Custom search and redesigned workflows: local acceptance

The independent custom-search acceptance is fulfilled on the local Windows
executable SHA-256 `c8aaa15a96698c045ce65440d9bb278730e998a2e8f2d431985aa5cbb34f28c9`.
Full Screaming Frog feature/regex equivalence remains **NOT_PROVEN**. The next
public CI run, other browser engines and release/install validation are separate.

- 1,746 ordinary workspace Rust tests pass, zero failures. Four Chromium tests
  are explicitly ignored there and all four pass separately through
  `node scripts/renderer-rust-e2e.mjs` on an owned ephemeral worker: 1,750
  successful executions in total. The original 512 KiB, unoptimized saved raw,
  rendered and selected-recovery gate passes without changing its stack limit.
- Strict Clippy passes for all targets/features with warnings denied.
- All 32 UI suites pass together: nine explicit API-fixture suites and 23 native
  suites. Main app shutdown is0; binary and tracked sources remain unchanged.
- The custom-search suite passes 10 workflows, including actual Chromium,
  four separate raw/rendered/selected sources, exact UTF-8 offsets, saved lazy
  inspector, unsupported regex preflight, unknown/zero/budget states, profile
  hiding, genuine external URLs, JSON import and two cold native processes.
  Seven real browser downloads match independent exact native HTTP artifacts;
  no website analysis refetches or page errors occur. Both cold processes close0.
- A consistent modern selected-recrawl source passes 12 native API/CLI/recovery
  scenarios. Changed discovery configuration with unchanged bound coverage is a
  separate refusal control:400 before child files or HTTP.
- 40 extracted UI fixture groups and32 state regressions pass separately; they
  are not browser tests.

The previous public run39117165582 failed Windows stack recovery, Linux's
inconsistent recrawl fixture and the directory test's late response-body read.
Those three concrete causes are corrected and locally exercised here. Its
container job passed; this does not attest the upcoming changed commit.

See [the search contract](../../custom-search-plan.md),
[the E2E guide](../../e2e-testing.md), [local results](local-result.json) and
[changed source hashes](source-gates.json). Large raw logs/screenshots stay in
ignored local artifacts and are hash-bound in the receipt. The installed user
application and crawl store were not used. Broader specialist combinations,
authorized provider success and the remaining feature catalog stay open.
