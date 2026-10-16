# Contributing

Open SEO Spider is an independent alpha project. Start with the [feature ledger](docs/feature-parity.json) and [to-do list](docs/FEATURE_TODO.md); a partial capability is not a promise of complete parity.

## Development

Use Rust 1.95 with a native compiler/linker, and Node.js 24 for optional renderer work. Install renderer dependencies with `npm ci --ignore-scripts` from `renderer/`.

Run checks appropriate to your change:

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

For renderer changes, run `npm test` in `renderer/`. Real browser and LanguageTool checks need their separately configured local runtimes; a skipped or unconfigured check is not a pass. Report what you actually ran.

## Issues and pull requests

- For bugs, include the source revision, OS/toolchain, expected and actual behavior, and a small sanitized reproduction.
- For features, describe the user workflow, input/output contract, limits and an observable acceptance criterion.
- Keep changes focused. Add meaningful regression tests for behavior changes, including missing/incomplete data and relevant boundary cases.
- Prefer synthetic local fixtures. Keep tests deterministic and avoid unsolicited requests to public websites.
- Explain source checks, executed tests and remaining runtime/platform gaps separately in your pull request.
- Update relevant docs and acceptance criteria without turning a bounded fixture pass into a broad parity claim.

Never commit credentials, cookies, private crawl data, local databases, downloaded browser/JRE/engine packages, vendor binaries or proprietary source/assets. Existing third-party notices must remain intact. Report suspected security problems through [SECURITY.md](SECURITY.md), not a public reproduction.

Public CI availability and a release's verification status must be checked for that revision. Do not bypass a failing check or claim a pending job passed.
