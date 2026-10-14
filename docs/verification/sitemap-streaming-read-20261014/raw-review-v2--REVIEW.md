# Independent RawReader BINARY postfix review

This is only the Root-supplied fix delta after the independent v1 receipt `13d9acdbe4f91234ce972d887750fb97b5eaa740ce395ea51f1a1fcfb14ba571`. Frozen Raw-Core v1 and its open-finding receipt are preserved unchanged.

Actual Root files read:

- `E:/CodexWork/open-seo-spider-01a12ac3-20261013/sitemap-streaming-integration-v1/crates/core/src/sitemap_ledger/verified_read.rs`, SHA256 `173c4eb928e8bd4d2a1881d404d00f105416b5b943554357a2b126ba614afd5c`.
- Sibling `verified_read_tests.rs`, SHA256 `c2e3d03a3ab4ccb82253bd6b3cd3c4d9fa32e8dd12a4a54927ebce3d9cc1830b`.
- Parent `sitemap_ledger.rs` still SHA256 `8ef116f6ae538f78dea5f5267db5c1fd1bceac1fa0e507ea9a9833440d14848e`.

Verdict: **v1's P2 task-collation finding is statically addressed by this delta; no new confirmed finding within the delta**. This is not compiler/type/test/runtime PASS.

The entire new production reader text equals frozen v1 after exactly two literal replacements: global row-ID grouping becomes `GROUP BY row_id COLLATE BINARY`, and observed-task grouping becomes `GROUP BY source_kind COLLATE BINARY,task_url COLLATE BINARY`. Explicit BINARY removes the inherited NOCASE/RTRIM task comparison and preserves the old Rust string identity for `/A.xml` versus `/a.xml`; grouping still detects exact duplicates. No filter, relation, canonical decoder, sequence, policy, serialization or token API code changed.

The old test producer now delegates to `source_rows_for(xml,TASK)`. Independent normalized-text comparison confirms the new parameterized helper equals the old helper after only the task argument/signature substitution. Its requested/final parser context, document source ID/task, transport requested/final URL and child aliases all use the same supplied task. The unchanged default caller still receives the old TASK. The existing `memory` fixture and all 14 old regression functions form an exactly unchanged suffix.

The 15th authored function `imported_nocase_task_column_preserves_distinct_case_sensitive_source_urls` constructs two actual decoder/parser/ledger sources for `/A.xml` and `/a.xml`, first saves an old-loader oracle catalog, then explicitly reconstructs only the documents table with `task_url TEXT COLLATE NOCASE` and without the task UNIQUE constraint. It restores the same rows, turns foreign keys back on, checks complete old-loader equality, compares new summary and checks the complete new raw window against the old row vector. This targets the actual imported-schema false-rejection condition without weakening the production schema or lowering a threshold. It remains AUTHORED / NOT_COMPILED / NOT_RUN.

The read/hash/text controls were actually performed and are pinned in `source-pins-and-controls.json`; they are source comparisons only. The first PowerShell control command had a parse-only syntax error before running any statements; the corrected file-read/hash/text comparison command completed successfully. No Cargo, compiler, SQLite fixture, product test, HTTP, source generator, native App, browser or SDK ran. No Root/source/Git/runtime file was changed by this reviewer. The v1 ordinary/selected/caller-transaction/SQLite-working-set and untested positive-policy/root limitations remain unchanged.
