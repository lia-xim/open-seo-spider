# Selected recrawl

This is an original independent implementation. A selected recrawl creates a new child snapshot. The immutable parent JSON remains available; unselected typed page payloads, their timestamps and their order are inherited unchanged. Only exact recorded requested URLs can be selected. Final/rendered aliases do not silently select a different direct row.

The child runs with the parent's effective configuration and original seed/sitemap-origin credential scope. Selection does not add credentials, scope roots or seeds. It suppresses seed injection, sitemap discovery and outward link admission. An admitted document still performs its ordinary robots check, redirects, extraction, configured browser rendering and bounded PDF/image analysis.

Pending selected historical pages are excluded from the child's current `pages` and analysis. Their old observations remain in the immutable parent. Each freshly committed target replaces the old observation with a newly constructed record, including robots-blocked, HTTP failure, truncation or renderer failure. Parent issues/warnings are not appended to current child issues. Core audits are rebuilt over the unique merged observation set; app relationship/graph audits must derive from that fresh report.

The report's optional `operation` records the explicit `mixed_age_selected_recrawl` coverage, exact parent-byte SHA-256, compact parent-page hashes, immutable plan/config hashes, selection occurrences, retained/refreshed/pending counts and target evidence. Counts come from the operation ledger; an old `fetched_at` is not a freshness inference. Ordinary and legacy JSON reports have `operation: null` or no operation field.

## Core API

```rust,ignore
let prepared = prepare_selected_recrawl(parent, &exact_snapshot_bytes, &selected_urls, Some(2))?;
let store = CrawlStore::open(new_child_database)?;
store.initialize_selected_recrawl(&prepared)?; // refuses any nonempty destination
let crawler = Crawler::new(prepared.config)?.with_store(new_child_database)?;
let result = crawler.run_controlled(events, CrawlControl::new()).await?;
```

`SelectedRecrawlParent` contains `job_id`, optional positive `checkpoint_revision`, and `source_kind`: `captured_checkpoint`, `imported_snapshot`, `current_projection` or `stored_json`. These describe the caller's actual source. They are not independent authentication of imported observations. Preparation hashes the bytes it actually receives, retaining whitespace and unknown report fields in the immutable source artifact.

For an in-memory child, use `with_selected_recrawl(prepared)`. A store-backed runner detects and validates the persisted `RunPlan::SelectedRecrawl` automatically, so SQLite-only recovery does not depend on an app manifest. `with_run_plan` can assert an expected saved plan; it cannot silently replace one.

The optional `max_attempts` limits unique target admissions **per invocation**, independently of inherited-page count and the parent's `config.max_pages`. `None` or initial `Some(0)` means unlimited. `with_recrawl_resume_override(RecrawlResumeOverride { max_attempts })` uses `None` to retain the last effective saved budget, `Some(0)` to remove it, or a positive number for a new per-invocation budget. Resuming never reopens already committed targets.

Read-only `CrawlStore::inspect`, `run_plan`, `recrawl_operation` and `parent_snapshot_bytes` do not start HTTP. Inspection validates the immutable source/plan/config, retained payload hashes, selected frontier depth/source, admission/commit generations, target counters and committed payload hashes. New plan contracts reject unknown semantic fields. Ordinary legacy report/PageRecord/config parsing remains tolerant.

`CrawlEvent::Operation` sends complete initial/terminal metadata. `RecrawlProgress` sends an authoritative one-target delta and cumulative counts after each admission/commit, avoiding quadratic copying of the complete target array. The app can maintain URL-to-target indices and expose a full current operation in status responses.

The CLI `recrawl parent.json <requested-url>... --output child.json --csv child.csv --store child.sqlite` requires new and distinct output/store destinations. Existing files, symlinks and alternate hardlinks are rejected before requests; final report writes also use exclusive creation. A source snapshot is never an output destination. CLI process failure can leave the newly owned SQLite frontier for explicit API recovery; JSON/CSV output is not an atomic two-file transaction.

Page CSV appends coverage, observation basis, selected state, immutable parent hash and captured timestamp after the original columns. Selected freshness follows the operation ledger rather than timestamp comparisons. Unknown future operation engines report `unknown_operation`; imported provenance is declared evidence, not an independent attestation.

## Pause, cancellation and recovery

Pause stops new admission, drains active native/browser/parser work and commits a durable checkpoint. Discoveries from selected documents do not enter a frontier. Cancellation/process interruption leaves uncommitted targets pending. A later explicit resume may repeat a request that reached its server before commit. Admission counters are evidence of admitted tasks, not exactly-once remote-request proof.

The child initialization and each selected page/target/frontier/counter update are transactional. Stable report checkpoints retain the existing generation guards. Unselected rows and exact parent bytes are never rewritten by resume. Preserved parent depth/source is historical discovery provenance, not a newly measured website-wide depth.

Resource bounds are explicit: parent JSON at most 256 MiB, at most 50,000 selection occurrences, at most 8 MiB total selection text and at most 16 KiB per selected URL. These bound one operation input; no 500-URL commercial crawl cap is introduced. The crawler's ordinary body/renderer/parser/resource bounds still apply.

Semantic config overrides, browser-only rerender from absent source HTML, full-site fresh coverage, reference-product parity, exactly-once network requests and power-loss durability are not claimed. Stored hashes detect tested individual inconsistency/corruption within this independent controlled writer; they are not an external signature against a completely and coherently rewritten database.
