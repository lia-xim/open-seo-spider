# Actual CSV streaming regression handoff

Status: AUTHORED / NOT_COMPILED / NOT_RUN. Compute HOLD was preserved. No compiler, Cargo, test, SQLite fixture, source website, HTTP route, native product, browser, RSS or large generator was executed. Only source reads, owned-file authoring, rustfmt parser/format checks and hashes were used. Root alone integrates product code and the parent hook. These files do not establish feature parity or finish S4.

## Integration and exact ownership

Copy only the TWO NEW files listed in manifest.json. Both before hashes are null; refuse to overwrite an existing path without review. Root appends the following child hook to the ACTUAL existing `crates/app/src/server/sitemaps_tests.rs` after applying the App handoff's exports-default literal. That parent SHA is `9265c0c67b90513d8ad4736d8012e4db0957a735c7dac14d6db8b8bbd376b0d4`:

```rust
#[path = "sitemaps_streaming_tests.rs"]
mod streaming_tests;
```

The new test child registers `sitemaps_streaming_fixture.rs` itself. No existing test assertion, fixture prefix, production file, dependency, Git/index or Root path was changed by this handoff. AppState construction uses the actual new exports registry. The actual handler route is `/api/crawls/{id}/sitemaps/export.csv`, not `/sitemaps.csv`.

Required product dependencies: immutable App manifest `3dac7530629186a60d9da074f12a765167dc5e470110d4c63469f6d9a90ed99e`, plus Root's reviewed postfix manifest `5f97d60596b1ba05549b81a124ada1f2680341696f4aaf569343a0bfbb744932`. The postfix removes an incorrect cfg(test) boundary from production-shared BoundedBytes, fixes preparation ownership, marks test-only stats appropriately and updates actual consumer-prefix progress for the idle timer. Final transport SHA is `a1ff346402e5bd1afc65930029c3cab1432aeb2ac43b5eeae8b344e606a81fea`; final membership SHA is `2151dc243d9b06a411a29c0cd883a51acae49de744139649a8fe982a8de764ed`. APIs/hooks were coordinated with App owner. No locally compiled compatibility is claimed.

## Actual fixtures and measurements

Positive stores come from actual public Crawler/ControlledCrawlResult/Store APIs. The existing small Fixture is reused. New Website produces an actual sitemapindex and two XML leaves; 260 entries exercise the first200 boundary. A separate 600-entry cohort with5000 literal quote characters per lastmod exercises large stored/CSV data under unchanged source/XML/field/catalog limits. The tests ASSERT actual oracle CSV length is greater than8MiB and less than64MiB before using that cohort for backpressure. These sizes remain runtime obligations, not measured facts.

The URL exclusion pattern retains original raw observations while preventing irrelevant member-page requests; each new Website setup must make exactly four actual website requests (seed, index, two leaves), and export must add zero. No Session/Header/window is synthesized. Byte comparisons call the preserved buffered formatter oracle over a complete, genuinely verified Store snapshot. CSV cells independently decode quoting/CRLF and check original typed row identity. Small malformed loc/formula and duplicate original identities remain original evidence.

Real Axum handlers run on loopback. Reqwest observes headers, exact length/SHA and late body errors. Raw TCP pauses reading the real socket; a requested64KiB client receive window avoids depending on receive autotuning, while product64KiB chunks and queue4 remain unchanged. Counters witness actual verified callback entry, completed actual passes, successful formatted rows, Core TX return and actual JoinHandle completion. No RSS or hard maximum cancellation-latency proof is inferred.

Only the positive active-drain subsection deliberately polls the actual handler's Axum Body directly. It uses a real PublicCrawler store and actual producer, drains any excess prefixes, then proves Core completed/joined before slow polling the remaining3..4 real queued64KiB chunks every750ms with idle1s. Total duration exceeds the idle period. This isolates real consumer progress from kernel buffering; separate tests retain genuine socket stalls/disconnects/timeouts. The test requires Root's consumer-progress postfix and never reduces CSV/chunk/queue caps.

SQL operations are explicitly negative corruption, concurrency, historical compatibility or restoration controls. A-only data comes from an actual public Crawler whose B transaction fails an actual trigger after A committed. Exact sidecar bytes are restored into the same-config source fixture; no hash/receipt/envelope is fabricated, and run_state is not rewritten. A6000-character real task identity produces an actual CaptureUnavailable marker via the existing bounded capture path. Its exact persisted marker is restored after a clearly labeled historical-absence representation; zero retained records never imply zero source URLs. A same-values pretty report rewrite changes only exact report bytes and checkpoint SHA; a separate rehashed source-record tamper must still fail SQL equality. These controlled restorations are not claimed as natural terminal writer history.

## Fourteen planned gates versus thirteen authored functions

Every row below is NOT_RUN. The word authored describes source, not a passing test.

| Design gate | Actual authored function / precise boundary |
|---|---|
|1 allstored/filter/oracle|`actual_csv_stream_matches_buffered_oracle_all_stored_exact_filters`: five kinds, source filter, source+location, exact original member, empty result; >200 stored Entry/Location rows and typed-row CSV oracle.|
|2 freshInventory / independentpaging|`actual_inventory_snapshot_token_opens_same_tx_two_pass_csv`: genuine full token from independently paged raw/saved views; true session/pass/row/join counters.|
|3 fulltail/integrity beforeReady|`actual_csv_stream_rejects_full_tail_source_report_and_inventory_corruption_before_ready`: excluded last-location SHA, source data/link, reportSHA, rehashed report/source record, invalid task done state; JSON500 and no verified callback/prefix.|
|4 context-only staleness|`actual_csv_stream_stales_a_marker_pending_and_terminal_generation_context`: pending-only, true A-before-B rollback restoration, actual omission marker, same-values pretty report SHA/length, genuine next terminal generation. An isolated selected-operation-only409 is not separately authored here.|
|5 selectedpresence/parent|`actual_csv_stream_selected_presence_and_exact_parent_guards`: actual selected child healthy export, parentSome/childNone sidecar deletion, exact parent BLOB whitespace tamper. Inverse parentNone/childSome wire variant remains OPEN.|
|6 WAL twopass|`actual_csv_stream_pins_original_tx_while_later_writer_changes_generation`: actual resumed public writer generation2 between real Ready and Pass2; old response generation1/oracle, later oldtoken409 and fresh export; App job write lock available afterReady.|
|7 exact64MiB/+1|NOT_AUTHORED / OPEN physical gate; concrete harness design below. No passing placeholder or tiny-cap substitute.|
|8 boundedbackpressure|`actual_http_csv_large_body_uses_bounded_chunks_and_backpressure`: real large TCP response, full fixed queue, actual producer-row/prefix plateau, then exact full oracle.|
|9 disconnect|`actual_http_csv_client_disconnect_cancels_and_joins_producer`: real stalled socket drop, actual join/cancel, unchanged Store files and subsequent successful export.|
|10 deadlines / activeprogress|`actual_http_csv_idle_and_absolute_deadline_close_body_and_drain`: real idle socket, absolute ready gate, small true-pass preparation timeout504, plus joined-worker positive active Body drain; timing assumptions must be checked natively.|
|11 tailafterjoin/TX|`actual_http_csv_tail_is_withheld_until_worker_join_and_tx_close`: actual pre/post Core-return barriers; fewer thanContent-Length bytes before join, final complete bytes only after joined1; fresh WAL checkpoint succeeds after actual TX return.|
|12 failures/drop|`actual_csv_stream_second_pass_failure_and_response_build_failure_drain`: real second-pass sink threshold, before-Core-return error, receipt mismatch, after-Core-return panic, actual builder failure/drop and pending HTTP request drop. Gates pin true late200 before fault release.|
|13 shutdown|`actual_csv_stream_shutdown_cancels_and_drains_active_exports`: real backpressured export, concurrent registry drains, exact joined/active state, shortened body and rejected new503 admission.|
|14 priority/roles|`actual_csv_stream_error_priority_preserves_existing_statuses_and_token_roles`: raw/page/malformed export role400, missing job404, filter/UUID stale409, unknown query400, actual App/DB conflict409, existingDB RawNone+previous full token exact catalog_unavailable409, missing DB409 before worker admission. The accepted simultaneous two409-code conflict is not asserted as an unchanged precedence.|

The13 names are also machine-readable in authored-test-names.json. Test source is not a13/14 coverage percentage. Group14 has not authored every optional legacy-engine/alias variation. Group5 inverse presence and group4 isolated selected-operation context remain directed follow-up cases even after the main functions pass.

## Actual native exact64MiB/+1 harness gate (still OPEN)

After HOLD is lifted and ordinary compilation succeeds, use a separately scheduled native App test/harness with a single stable loopback origin. Construct BOTH fixtures through actual Crawler/parser/Store calls and all existing32MiB serialized catalog, row, field, initial-input and source budgets. Do not seed valid rows/headers with SQL and do not change the product cap.

First establish reachability under ALL those limits. Candidate quote amplification may mix actual retained invalid explicit roots (raw CSV value plus typed JSON), XML location text and lastmod text. Treat this as a calibration hypothesis: raw input8MiB and retained/source/catalog bounds can prevent the desired total. An attractive formatter expansion ratio alone does not prove an accepted store exists.

In the native harness, measure each actually accepted store with the SAME production write_csv_header/write_csv_row and opaque session visit, using an independent uncapped counting/SHA sink only for expected-length measurement (no giant CSV Vec). Two adjustable real raw-text dimensions, such as ordinary ASCII and quote/escape characters in retained originals, must demonstrate relatively-prime actual formatter deltas after offsets/ordinals/row JSON/IDs/config changes are fully accounted for. Rebuild and fully validate every candidate; do not assume that delta1 remains delta1 across JSON escaping, XML offsets or ordinal digit boundaries.

Freeze a physically accepted pair whose independently counted totals are EXACTLY67_108_864 and67_108_865, with source/config/catalog hashes and deterministic producer inputs. Compare the exact-cap buffered oracle and full streaming response length/SHA, typed rows, headers, normal EOF and joined/TXdone. The+1 buffered oracle must give its real budget error, and the actual HTTP route must413 sitemap_csv_byte_budget beforeReady with zero CSV prefix/pass2. Preserve native wall time, actual errors and optional RSS/socket observations separately. If no accepted pair exists under the current producer budgets, leave the physical gate OPEN and report the limiting accepted path; do not invent a positive Store or call the sink unit test physical proof.

## Remaining execution sequence

Root reviews and integrates the two files/hook, preserves the existing suite and freezes source. After coordinated HOLD release: compile/type/lint gates; directed13 named groups; preserved historical formatter/reader/selection controls; exact physical harness only after a valid calibration pair; native socket/cancellation timing and resource evidence. Failures justify narrow repairs and retests. Nothing here authorizes a runner start, replaces those gates or marks the project complete.
