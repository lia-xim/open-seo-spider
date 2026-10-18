# Claude redesign: local end-to-end acceptance

The local Windows/MSVC run on 2026-10-18 passed all 25 registered suites in one
execution. Nine use explicit API fixtures; 16 exercise native application flows.
The [machine-readable result](local-result.json) records individual suite exits,
the tested binary digest and hashes of the changed source inputs on top of
Claude's `53bb730` redesign. These hashes describe the tested working tree,
before this documentation commit; they are not a clean-commit build receipt.

The owned application closed naturally with exit code zero. Tracked files and
the executable remained unchanged during execution. Runtime logs and screenshots
remain in the ignored local run directory named in the result. CI uploads its
own fresh evidence; this local result does not claim a CI pass.

New browser coverage includes 18 tool-menu round trips at 1440/320px and six
saved-spelling workflows through Rust and an owned deterministic
LanguageTool-compatible HTTP fixture. The latter makes three real protocol
requests, checks dictionary/profile reload and historical CSV binding, handles
provider failure, and performs zero website refetches. It does not assess a real
LanguageTool engine's language quality or a cold spelling-service restart.

The tests exposed and corrected the normal unoptimized Windows startup stack
overflow and a populated spelling-page overflow at 320px. Test infrastructure
also needed a fixed legacy input path and an explicit owned renderer endpoint.
Earlier failing artifacts remain local; no assertions were removed to pass.

Additional current checks passed: formatting, strict workspace Clippy for all
targets/features on Rust 1.95, 40 extracted UI VM groups, 32 asynchronous-state
tests, and focused Rust certificate/connection-overflow regressions. This is
not a fresh full-workspace Rust test run. Linux and public CI are separate gates.

See [the coverage matrix and remaining specialist workflows](../../e2e-testing.md).
Migration, directory tree, segments, n-grams, broken-source and provider tools
still need functional browser acceptance beyond menu navigation. The full
144-capability backlog and reference-product parity remain open.
