# Future single Migration Root release

This file is a source-only contract, not a grant. No Source8 runtime hash or lease is issued here.
The one reviewed harness remains in its immutable E preparation, with manifest and browser helper beside it. Do not copy the harness alone to Task/work because its HERE-relative guards require that bundle.

The future Root-owned release is a fresh, exclusively created file at Task/work/v8-migration-native-release.json. It must include the following fields:

- kind root_released_migration_workflow_native_v8; status RELEASED; issued_by Root.
- scope genuine_saved_crawl_migration_api_cli_browser_cold_reopen.
- execution_lock_mode outer_owns_shared_native_lock; deadline_ms 600000. The latter describes the unchanged inner cooperative deadline, not an outer timer or whole-application deadline.
- Actual not_before and expires_at, finite UTC times separated by at most two hours.
- A fresh Root-issued lowercase hexadecimal lock_token of 32 to 128 characters. Root must generate the token when it issues the future grant. The outer does not generate a different UUID.
- Actual source_manifest_sha256, suite_proof_sha256, build_proof_sha256 and candidate_binary_sha256. All four are explicit Root CLI arguments and must exactly match the grant.
- build_driver_sha256 e48e2cadc361a100eecbbed5465bf4436d28f7fa19068682550932f320e1c547 and source8_admission_status ACTUAL_REVIEWED_BINDER_AND_PROOFS_AVAILABLE.
- preparation_manifest_sha256 52f828b753b2316b34cd9f0f5e3ca79dc246028e730e2529e27af95ff6210a95; harness_sha256 c0af21f5fc5f66b3d82b3902f64a41ced69a56b7cf31f14dd08f5e333690f647.
- independent_review with the actual absolute path and SHA of E:/CodexWork/open-seo-spider-01a12ac3-20261013/http-migration-native-independent-review-v3/receipt.json (3ffbd29e36d3d68eed2ed4e78e9e660f2a53f75f19880f0241602c981f67b79a).
- binder with exact path/SHA E:/CodexWork/open-seo-spider-01a12ac3-20261013/combined-v8-runner-preparation-v4/combined-bindings-v8.mjs (c58f1984c6b6593d384915157afdcdae206acd1bb8af72279a9ee82702e1b7e7), manifest {path,sha256} for E:/CodexWork/open-seo-spider-01a12ac3-20261013/combined-v8-runner-preparation-v4/manifest.json (d4adece778ec4a2b3ef713be7319840457bbf5119b48809fe07493af65ca1c80), and independent_review {path,sha256} for E:/CodexWork/open-seo-spider-01a12ac3-20261013/combined-v8-runner-independent-review-v4/receipt.json (3b6240aca4bec56afb058d83b8552ef1c36b28ba465169a31052c5df0e28ae78).
- migration_outer_preparation {path,sha256} directly referencing this final proposal manifest; outer_driver_sha256 referencing the exact promoted outer.
- outer_independent_review {path,sha256}, kind http_migration_native_outer_independent_review, NO_FINDINGS_BOUNDED_SOURCE_REVIEW/findings[], direct proposal_manifest and outer {path,sha256}. This independent review does not exist in this authored preparation.
- Actual node_sha256; ordinary chrome {path,sha256} using C:/Program Files/Google/Chrome/Application/chrome.exe; actual playwright_root under the Task renderer/node_modules/playwright-core and playwright_package_sha256.
- output exactly E:/CodexWork/open-seo-spider-01a12ac3-20261013/results/migration-workflow-native-v8, which must be absent.

The future outer is promoted to Task/work/run-migration-native-outer-v8.mjs. Its preflight remains in this E proposal. The future CLI has eight pairs, with no default or fabricated runtime hash:

    --release ABS --release-sha ACTUAL64 --preparation-sha ACTUAL64 --source-manifest-sha ACTUAL64 --suite-proof-sha ACTUAL64 --build-proof-sha ACTUAL64 --candidate-sha ACTUAL64 --lock-token ACTUAL_ROOT_HEX

The only child command remains the existing harness CLI, exactly four pairs:

    --release ABS --release-sha256 ACTUAL64 --source-manifest-sha ACTUAL64 --lock-token ACTUAL_ROOT_HEX

The outer alone exclusively creates results/combined-native-outer-v4/active.lock with gate migration-workflow-v8, its actual process PID, the exact grant/CLI token and actual Source8 SHA. The harness only reads/rechecks it. The new outer output is results/combined-native-outer-v4/migration-workflow-v8. Both output directories must be absent; there is no retry or overwrite path.

The outer proof records lock_token, lock path, lock-byte SHA, actual immutable grant SHA and exact child arguments. The unchanged inner proof binds the same token through its actual release {path,sha256}; the hashed grant contains the token and the harness checks its CLI token against both that grant and the outer lock. No new inner proof field or harness runtime edit is claimed.

Outer success means only PASS_OUTER_CHILD_EXIT_NOT_INNER_ACCEPTANCE. Root must separately require the inner PASS_ACTUAL_SOURCE8_MIGRATION_WORKFLOW, all eight exact cases, five actual natural child closes, complete pipes/logs, actual fixture/socket closure, unchanged website counter and final deep Source8 binding. An outer success never replaces those inner oracles.

No foreign PID kill, outer timeout, automatic retry, Source7 PASS reuse, install, source change, real grant or lease creation is included in this source preparation.

Revision2 uses only the independently repaired RunnerV4 and direct review3b624. The blocked V1/399b and RunnerV3/032bb remain immutable history. Actual Source8 runtime proofs are still required; this is not a grant.
