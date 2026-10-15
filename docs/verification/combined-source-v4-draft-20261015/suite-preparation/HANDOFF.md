# V4 source freeze and complete workspace acceptance

These scripts are preparation only. They have not frozen V4 source, opened a fixture, started a compiler, run SQLite, launched a browser/native app or installed an executable. They are derived from Root's actual V3 freezer and suite driver. V3 results cannot establish V4 acceptance.

Root owns promotion, formatting, commit/push, the final byte approval and all execution. Copy the final, independently reviewed script bytes to the exact task `work/freeze-combined-source-v4.mjs` and `work/run-combined-suite-v4.mjs` paths. Both reject execution from the handoff directory. The V4 source and result locations must not exist before their respective commands.

The freezer requires five explicit option/value pairs, with no defaults:

- `--source-commit`: the actual clean, committed and privately pushed source commit.
- `--ledger-manifest`: the absolute actual `sitemap-membership-capacity-v3/manifest.json` path.
- `--ledger-sha`: exactly `9da86f091f5bba9ba70eca5415542cfaf9fb43947c9f5bb1259c192b2d170a82`.
- `--root-approval`: the absolute actual Root final-byte approval JSON path.
- `--root-approval-sha`: that approval's actual SHA-256.

The fixed parent is `combined-source-v3/manifest.json`, SHA `01bfa8efcb6745c5521be5e533ae63f2f15679318b095a29eb3ab925fef95bdf`: all 301 inputs, all 421 required names and the 1,203-function baseline minimum are retained. The SQL handoff and its independent review are pinned to their actual existing SHA-256 values. Only the actual final capacity-v3 source revision above is admitted; capacity-v1 and v2 are excluded. The ledger's independently reviewed receipt must be an actual pinned file; no placeholder or future acceptance is admitted.

The actual final capacity review is `sitemap-membership-capacity-review-v1/receipt.json`, SHA `d9504ec7b44aa5d102cef7c6b270b0445412aa813f67978113edb6c53981f18b`, result `NO_FINDINGS_BOUNDED_SOURCE_REVIEW`. The freezer and suite pin this exact review. The reviewed authored union has 18 paths, including three new source files, and 13 new test functions. This yields source expectations of 304 inputs, 434 required names and a 1,216-function minimum; these are authored expectations, not executed V4 acceptance. The freezer recomputes the values from the actual final formatter-only Root bytes.

Root's approval must have these fields:

```text
kind: root_approved_combined_source_v4
status: REVIEWED_FORMATTED_COMMITTED_AND_PRIVATELY_PUSHED
parent_source_manifest_sha256: the fixed V3 SHA
source_commit: the same actual --source-commit value
source_branch: codex/mcp-file-tools-20261015
source_worktree: the exact mcp-file-tools-v1 path
proposals: exactly two ordered entries, sql then ledger
  each entry: role, path, sha256, review_path, review_sha256
files: exactly the union of the reviewed source paths
  each entry: path, role, before_sha256, proposal_sha256, after_sha256, bytes, format_only
new_authored_tests: actual new function declarations grouped by their real source path
  each entry: path, names
```

For `proposals`, use the actual absolute path strings and hashes from the handoff manifests, with the SQL constants taken from the freezer. For new files `before_sha256` is `ABSENT`. `proposal_sha256` is the reviewed authored after SHA; `after_sha256` and `bytes` are the actual final formatted and committed Root source. Set `format_only: true` when formatting changed the authored file hash; Root must review that this really is formatting only. No other change is admitted. The freezer checks actual Git source bytes against both the approval and the explicitly published commit, rejects unexpected changed/new compile inputs and checks the live repository remains private.

Group `new_authored_tests` by source path sorted with `a.path.localeCompare(b.path, 'en')`; keep function order as declared in each source file. The freezer independently derives new test declarations from both the authored proposals and final Root files, rejects removed old or new functions and requires exact agreement. It appends all actual new functions to the 421 inherited required names and raises the minimum from 1,203 by that exact number. It also retains the four genuine owned Chromium gates. The nine capacity functions are physically in three new source files; two are included into existing modules with `include!`. The source manifest records their authored full Cargo module names, and the suite checks every new exact module name in the actually closed workspace raw log in addition to the unchanged leaf-name/count parser.

After the freezer closes successfully, obtain the actual printed V4 manifest SHA. Run the suite only with `OPEN_SEO_COORDINATED_EXECUTION_WINDOW=confirmed` and `--manifest-sha` set to that exact actual SHA. The suite runs fresh `cargo fmt --all --check`, the complete locked workspace tests with real owned renderer/fixture, and all-target locked Clippy with `-D warnings`. It retains opt-level 1, debug assertions and overflow checks enabled, debug information disabled. All actual child close events, raw logs, required names/counts, zero failures/ignored tests, final source/reference hashes and actual owned-server closes remain mandatory.

The optimized build, source-bound native/SDK/browser/physical acceptance, platform gates and data-preserving installation are separate subsequent work. The unchanged physical CSV 64 MiB / App JSON 8 MiB exact-plus-one harness must not be replaced by smaller caps or weaker data. The prior externally aborted physical attempt is a preserved failure, not a natural timeout or a proven first-page hang. There is no measured performance/RSS claim or manufacturer parity claim here.
