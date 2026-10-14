# Bounded independent static review

No confirmed finding in the pinned Core rich-read facade and typed Raw budget integration. This is a static source result, not compilation or runtime proof.

The actual facade acquires one Store connection and one transaction. Metadata preflights and the shared saved-config/import/policy guard run before typed raw/source proof construction. Both existing selected-parent higher readers run even when raw or source history is None. Selected presence and exact parent projection checks remain materializing compatibility; they are not claimed to be bounded ordinary streaming.

Analysis is prepared once from verified same-transaction inputs before constructing the full shared read-state hash. All Expected fields are checked against the actual header before continuation matching or raw window selection. The shared preimage includes the new reader engine, full raw summary, inherited basis, optional canonical inventory-body SHA, coverage, verified source context, and existing lastmod/analysis engines and analysis context. Unknown history remains Option::None; a context-only Some(empty rows) is not fabricated as a complete catalog.

The private RawWindowByteBudget marker covers raw row-byte and exact serde page-size refusals. The rich facade maps that marker to ResponseByteBudget while preserving other errors. Its own exact serde DTO count has a separate 16 MiB ceiling; the App 8 MiB whole-response cap remains separate. No arbitrary corruption is converted into a stale or budget error. The raw continuation check uses the actual row filter and sequence; stale Expected comparison precedes that check.

Actual exports, module registrations, sibling helper visibility, owned return fields, and lexical/NLL borrow scopes were inspected against the integrated companion APIs. Proof handles are dropped before transaction commit. This does not establish compiler acceptance or runtime lifetime/SQL behavior.

All nine Root test groups were inspected as authored source: six in store/sitemap_read_window_tests.rs and three appended at store-sitemap-source-tests.rs:1137, :1202, :1249. They use actual Store/parser/A/B/selected-parent producers, with labelled SQL mutation fixtures. They cover full-snapshot oracle agreement, stale-before-invalid continuation, pending-only state change, unknown/filter scope, tail corruption, read-only/input bounds, A-only/B state changes, selected missing presence, and legacy selected None/exact parent BLOB guard. The existing Raw 8 MiB test additionally asserts the concrete error marker. No test was compiled or run.

Limits: independent integration review only, with previously reviewed Raw/Source validators and frozen Coverage/Analysis companion contracts. No complete algorithm re-audit, TypePASS, measured memory/RSS bound, full feature parity, native runtime, or production readiness claim. Saved configuration and selected parent/run-plan compatibility retain documented input/materialization costs. Same-connection transaction discipline is required; the actual facade obeys that static contract.

Execution gates: Cargo/compiler/tests/SQLite fixtures/HTTP/browser/native all NOT_RUN under coordinated compute HOLD. Product, Root, Git and runtime files were not modified. Only this review directory was written.
