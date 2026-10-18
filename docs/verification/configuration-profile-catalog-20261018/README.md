# Named profile catalog and complete local regression

Result: `PASS_LOCAL_NAMED_PROFILE_CATALOG_AND_FORTY_SUITE_REGRESSION`.

This is bounded acceptance of the independent Rust implementation. Full manufacturer feature, format and behavior parity remains `NOT_PROVEN`; `storage.config` remains partial. The installed Source16 pilot was not updated.

## Verified source and execution

- Normal Windows binary SHA-256: `b6945c9d0786586a60b9646ed803e75f3ea117263a51135b6d3fd6dc139d7bd5`.
- 573 explicit prebuild source/harness inputs, including both literal test-document fixtures and `.gitattributes`, remained unchanged. This is not an exhaustive compiler or runtime dependency closure.
- 1817 ordinary Rust tests passed, zero failed, five deliberate opt-in ignores. All four named Chromium cases also executed successfully on an owned worker.
- Strict Clippy for all targets/features, 130 state tests, 40 semantic UI groups and the controlled local-provider browser/native/cold path passed.
- All 40 UI suites passed in one invocation: nine API fixtures and 31 native suites, with unchanged source/binary bytes and natural coordinator EOF0.

## Named native profiles

Actual browser actions create, replace, rename, delete and reload canonical portable profiles in the selected data directory. Explicit Load stages the dialog copy; the existing settings Save adopts it. Catalog mutations preserve the active copy, browser storage and volatile header bindings.

Two positive native processes use the same catalog across different origins and fresh browser contexts. Two additional negative-only processes prove corrupt and unsupported storage refusal. All four close naturally with EOF0. Five actual downloads and 11 SQLite snapshots establish exact profile/hash/metadata preservation, including an unexecuted u64 above JavaScript integer precision. Two tabs demonstrate stale PUT and DELETE conflicts; a held real GET cannot overwrite a newer selection. The actual warm two-page crawl preserves every loaded native option plus explicitly supplied volatile headers. The cold phase reloads and reapplies profiles and proves missing-header Start refusal; it does not perform a second successful crawl. A positive HTTP410 witness spans both positive process closes, with zero product refetches. All catalog controls and an actual export are checked at 320 px.

## Retained failure and current keyboard check

The earlier 39-suite aggregate and the old [e4f58c2 renderer CI job](https://github.com/lia-xim/open-seo-spider/actions/runs/39151352510/job/117745485458) failed at the original density Space assertion after 31 successful suites. Those results remain failed. Passive observation reproduced the missing native activation click while the preceding touch toggle was still settling. The canonical driver now waits for that actual trusted toggle completion before Space; original assertions and deadlines remain. Three prespecified matching-build runs and the full current 40-suite invocation passed. The exact Chromium cause and general browser stability are not proven.

## Open gates

Fresh exact-commit public CI, optimized build/install, other operating systems/browsers, production provider credentials and full vendor parity remain separate. The catalog accepts at most 128 live entries, 64 MiB per profile and 256 MiB live profile text; the actual 128-entry native boundary is tested, but byte-boundary negatives use smaller injected limits. Maximum-size actual browser paths, global RAM/CPU/physical-disk bounds, crash durability and simultaneous multi-process stress are not established. Portable CLI profile execution, migration/history and synchronization remain open.

Machine-readable [local result](local-result.json) and [source/gates](source-gates.json) retain exact hashes, commands, durations and scope. Raw runtime artifacts stay outside Git.
