# Native per-job column preferences

Status on 21 October 2026: **implemented and locally accepted for this bounded column workflow**. Fresh selected Node579, semantic40, RustR2 App550 and NativeR2 pass. The separate affected-nine R3 invocation also passes. The first exact successor `9816fe9` failed its comparison fixture and Unix symlink-test cleanup; the [original CI failure](verification/native-project-columns-20261021/ci-9816fe9-failure-20261021/README.md) is retained. The [local test-only correction](verification/native-project-columns-20261021/ci-repair-local-20261021/README.md) passes comparison-controls R2 and fresh Windows RustR4 App550/format/build/strict Clippy. Complete corrected50-suite/public-CI acceptance remains pending; the Unix correction still needs its new Linux CI execution. The [source-bound receipt](verification/native-project-columns-20261021/README.md) records the exact gates and preserved failures.

This is one component of the [open native workspace plan](native-project-workspace-plan.md): explicit storage of visible columns and their order for main-table sections of an existing native crawl job. Here `project_id` is that job's UUID. Named project grouping, native named-view catalogs, portable workspace packages and other settings remain separate work.

## User actions

**Load project columns** reads the native document and applies only the current section's column keys. **Save columns for this section** merges the current projection into the last explicitly loaded whole document, preserving its other sections. **Clear all saved project columns** has a separate confirmation and clears every saved section of this job while leaving the current table active.

These operations are explicit. Opening the application, selecting a job or changing sections does not automatically fetch, apply or save native column preferences. Existing browser-local columns and origin-local V1/V2 catalogs retain their behavior and bytes. Loading native columns preserves search, the built-in filter, sort, accepted V1 AND rules and V2 expression trees.

The complete current schema must match before Load applies anything. Unknown or missing keys are not silently projected away. Mandatory identity columns remain visible in their fixed prefix order. A stale Save is refused with a conflict; there is no automatic retry or overwrite. Saved report JSON, crawl SQLite state, report hashes and native generations are outside the preference mutation.

## Native contract

| Operation | Route | Body |
| --- | --- | --- |
| Read | `GET /api/crawls/{id}/table-preferences` | none |
| Save | `PUT /api/crawls/{id}/table-preferences` | `{expected, entries}` |
| Clear all | `POST /api/crawls/{id}/table-preferences/reset` | `{expected}` |

Successful responses contain `{project_id, fingerprint, document, error}`. The fingerprint is exactly one of `{state:"missing"}`, `{state:"present",revision,sha256}` or `{state:"invalid",sha256}`. Missing reads create nothing. Invalid bounded bytes return their raw hash and a concrete error with `document:null`; ordinary Save refuses that baseline. Clear all writes a **present empty document with a fresh revision**, preserving the distinction between absence and reset and preventing reset ABA.

The document is `{engine:"open-seo-spider/table-column-preferences-v1",version:1,project_id,revision,entries}`. Each entry is `{section,schema:[{key,type,mandatory}],visible_keys}`; types are `text`, `number` or `auto`. Duplicate/unknown fields, duplicate sections or keys, missing mandatory columns and an invalid mandatory-prefix order are refused.

Saved bytes are limited to512KiB, with4096 additional bytes for request/response metadata. Limits are64 sections,128 columns per section,256 UTF-8 bytes per key and128 per section name. Oversized saved bytes are refused without claiming a full fingerprint. Files live under `<data_dir>/table-preferences/<job-UUID>.json` with a stable sibling lock. Writes use a dedicated two-slot gate, cooperating-process locking, reread/CAS and complete temporary-file publication. This does not promise recovery from arbitrary power loss or writers that ignore advisory locking.

The UI guards job, section, UI/native generation, segment context and query revision. A fixed15-second request-and-body deadline releases busy state. An uncertain write requires another explicit read; abort is not a rollback guarantee. Conflicts, invalid storage and transport/file failures are shown as column-specific errors.

## Actual acceptance and remaining work

The [NativeR2 receipt](verification/native-project-columns-20261021/receipt.json) records14 native groups, six complete CSV/XLSX pairs and12 download events. Each format contains258 data rows,2316 data cells and52 header cells. Two genuine jobs contain63 and3 pages; title/status views span more than one visible page. Expected values, order, typed numbers/booleans and every provenance cell are checked independently of the product projection/compiler/writer. The63-page job's warm/cold title and status files retain identical bytes.

The executed workflow preserves AND/OR query state, exact report bytes and nonempty browser-local column/V1/V2 catalog values. Five negative-control kinds cover two-context CAS conflict/reset ABA, corrupt storage, a regular-file directory blocker, partial HTTP PUT followed by valid recovery, and a stale read against a visibly different working projection. Six control-geometry checks cover1440/320px. Two native processes close naturally withEOF0; the cold origin changes and starts with empty browser storage. Two real410 witnesses accompany zero offline website requests; warm preference operations also forbid refetching after completed captures.

Fresh ordinary product-path controls pass579 selected Node tests, including the44 new controller/adapter cases, plus40 separate semantic groups. RustR2 passes format, normal build, strict workspace Clippy and550 App tests with0 failures and1 deliberate ignored opt-in. The normal binary is bound to426 unchanged selected inputs. This is not a fresh ordinary Core/workspace or renderer-opt-in run, nor a transitive dependency/toolchain closure.

The [parent evidence](verification/native-project-columns-20261021/README.md) retains original RustR1 and NativeR1 failures. The [affected-nine supplement](verification/native-project-columns-20261021/affected9-20261021/README.md) separately retains its R1/R2 failures; its separate R3 invocation passes all nine selected suites with unchanged product inputs. Successful individual or selected runs do not establish a combined50-suite result or new public CI. Original store/binary/capture corpus files are not copied into the public receipt; retained oracle JSON does not replace that original input evidence.

The full workspace remains open: native V1/V2 catalogs, portable packages with exact original-report bytes, atomic report-plus-workspace import under a new UUID, cross-directory portability, named grouping, selected-recrawl adapters and broader settings. Other schemas, platform/race/crash/resource limits and the complete `ui.filter-details-export` workflow need their own acceptance. The144-capability/416-filter/32-workflow ledger and39/68/37 capability states are unchanged; manufacturer parity remains `NOT_PROVEN`.
