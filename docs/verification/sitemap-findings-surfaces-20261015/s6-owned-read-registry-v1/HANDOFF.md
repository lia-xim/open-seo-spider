# Owned findings read registry — guarded source handoff

AUTHORED / NOT_COMPILED / NOT_RUN under compute HOLD. Two new server-child files only; Root alone owns product integration, AppState/routes/shutdown and Git. Exact current dependency source hashes are pinned; no existing file or dependency was changed. Default120s is production behavior; short deadlines exist only through a cfg(test) constructor.

## Exact API and Root hooks

`pub(super) struct ReadRegistry` is Clone+Default.

`pub(super) async fn read<T:Send+'static>(&self, operation:impl FnOnce(SitemapReadControl)->anyhow::Result<T>+Send+'static)->Result<T,crate::sitemap_membership::Failure>`.

`pub(super) async fn shutdown_and_drain(&self)->anyhow::Result<()>`.

Test-only `pub(super) snapshot()->ReadStats` returns actual independent started/active/joined/failed/retained/closed fields. No raw CSV counters are reused.

Root registers `mod sitemap_findings_reads;` under server, stores a cloned/default registry in AppState, and invokes `.read(move|control| ... actual readonly Core operation using that control ...)` for owned findings reads. The same actual control must reach Core's verified read methods, which attach real SQLite InterruptHandles. The registry does not construct or impersonate a SQL snapshot/proof. Root must call shutdown_and_drain on all service exit paths BEFORE destroying the Tokio runtime. Finding Failure codes require shared findings::status, not old Failure.status alone.

## Ownership and completion invariants

Admission and shutdown close/cancel are serialized under one lock. Every admitted request increments active before spawning a real blocking worker. The registry stores an async supervisor JoinHandle plus the same actual control. The supervisor owns and awaits the real spawn_blocking JoinHandle; it never aborts or detaches it. A request-future Drop guard calls cancel immediately, independently of supervisor scheduling; the result receiver closing also signals the supervisor. The independent immutable120s async timer calls expire even if the worker does not poll checkpoints or changes its cooperative deadline. That invokes attached actual SQLite interrupt handles where the Core read has bound them.

Neither cancellation nor timeout returns while that worker remains alive. Worker success is accepted only after actual join and a final shared control checkpoint. Actual operation errors pass through findings::typed_failure; unrelated IO/corruption is not overwritten as Unknown. A blocking worker panic/join error becomes a read-failure result only after JoinHandle resolution. A post-join RAII guard decrements actual active and notifies drains even if result formatting/classification/destruction unexpectedly panics. This guard is deliberately constructed AFTER worker join, so it cannot claim completion of a live worker.

Completed supervisors are reaped only after is_finished, polling their actual handles to resolution. Unexpected completed supervisor join failures are retained as a fixed first failure and returned by drain, rather than silently ignored. Shutdown cancels every admitted control and leaves unfinished supervisor handles IN the registry. Cancelling one drain future cannot drop/take those handles; another concurrent drain still waits for actual active0 and an empty reaped handle map. No mutex guard crosses an await.

Notify is registered/enabled BEFORE checking completion predicates. If active has reached0 but a supervisor is in its final synchronous return interval, drain yields and checks the actual JoinHandle; it does not wait for a second notification that can never happen. Repeated/concurrent drains share closed admission and completion state. Poison recovery preserves retained task ownership and records an error; drain still waits before returning that error.

## Limits and honest caveats

An uncooperative blocking worker/noninterruptible OS IO may keep read and drain pending. The registry does not promise forced thread termination, hard cancellation latency, or a finite shutdown bound in that case. Returning early would falsify worker completion. Root must preserve the live runtime until drain; externally aborting/destroying the runtime or process is outside this orderly-shutdown contract. No new global admission/resource cap is imposed; retained bookkeeping is proportional to admitted active/recent requests and completed entries are reaped on later admission/drain. SQLite interrupt timing, native disconnect behavior and physical resource bounds remain acceptance gates.

## Eight meaningful authored test groups, all0executed

1. Actual worker scope Drop occurs before successful result; drain sees joined1/active0 and reaped supervisor.
2. Abort actual request future while blocking worker waits on a real channel: actual control is cancelled immediately; drain cannot finish before release/join.
3. Short supervisor deadline with the control's cooperative deadline removed: only asynchronous expire can stop it; no result before channel release, then typed timeout.
4. Two concurrent drains, cancel one of them, reject new admission without running its closure, keep active handle until actual release/join.
5. Real spawn_blocking panic joins and reports500 without active leakage.
6. Actual operation typed query failure and permission IO retain shared classification.
7. Actual completed supervisor handles are reaped before a later held worker is admitted.
8. Already-completed notifications and repeated concurrent drains do not lose wakeups.

Tests use actual blocking workers, async tasks/channels/short timers, not fabricated stats or Core proofs. They do not execute a SQLite query and are not physical SQLite interrupt proof. Source parsing/formatting and rustfmt--check exit0 only. No compiler/Cargo/clippy/tests/SQLite/HTTP/native/browser/runtime/fixtures executed, and no accepted test/feature count increases.