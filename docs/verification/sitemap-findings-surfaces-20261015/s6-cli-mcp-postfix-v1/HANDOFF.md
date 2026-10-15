# S6 CLI/MCP reviewed postfix — source only

Basis immutable s6-cli-mcp-core-v1 manifest ebaebf161dbdc7defbd3e96ff7c24af9cadf6535d801d28714961558783492cf. Original v1 remains unchanged. This separate handoff corrects three independently confirmed findings. No Root/Git/runtime writes, compiler, Cargo, tests, HTTP/stdio/SQLite fixtures, SDK/native binary, actual61s wait or browser execution. Root is sole integrator.

## Minimal product corrections

1. In mcp/sitemap_findings/stdio.rs, active id is borrowed from request.get("id"), passed to valid_rpc_id BEFORE cloning, then cloned only after successful bound/type validation. Root's earliest parent dispatch borrowed-id guard remains required and unchanged. Actual input framing/queue/cancel/EOF/error text and wire API remain unchanged. The prevention of an avoidable oversized clone is established by the actual statement order in pinned source; no allocation/RSS test was executed.

2. In mcp/sitemap_findings.rs, ONLY the actual dedicated findings POST adds RequestBuilder.timeout(Duration::from_secs(150)). The shared Bridge constructor retains inherited60s for legacy tools. The150s value gives bounded outer margin over Root's120s App read deadline; it is not a hard SQL interruption, filesystem IO or worker-join latency guarantee. All request/response/id/CSV-descriptor/wire caps and public API are unchanged. Locked local official reqwest0.12.28 source RequestBuilder.timeout docs/setter285-296 states per-request override; actual Client total_timeout selection2629-2635 reads request extensions. Cargo.lock and those exact source files are pinned, read only.

3. In the NEW native SDK harness only, CLI ChildProcess ownership moves to outer scope. Child.close Promise resolves only upon actual close, while spawn error is separately retained (error is not a close witness). finally stops/awaits actual CLI close before App/data teardown, tracks already-closed state and signals only held live ChildProcess objects; no arbitrary PID lookup. Bounded graceful shutdown can force the same still-live owned child, but forced shutdown makes the run fail. No data-directory cleanup and no positive receipt if any owned close remains unobserved. Final positive receipt still follows actual shutdown/cleanup. Script beforeSHA604963fce6798cf03309738b9235a438ae325809e63a66a6b5af04f466a07dfe. Lifecycle correction is static only; no hung-child process fixture was run.

## Directed authored regressions

Original18 Rust test functions/assertions remain byte-for-byte in the existing tests.rs prefix. Only a final child module hook is appended; new postfix_tests.rs adds2 groups, total20 including v1:

- findings_postfix_stdio_exact_utf8_rpc_id_and_plus_one_preserve_following_frame: real PublicCrawler/Store-backed HTTP Bridge over actual stdio,1024 UTF-8 bytes accepted,1025 rejected as small null-id error before another API request, following ping preserved. This validates transport behavior and byte boundary; it does not measure clone allocations.
- findings_postfix_actual_sixty_one_second_http_response_succeeds_while_legacy_times_out: two actual loopback requests run concurrently against61s delayed responses; dedicated findings call must return real Core/adapter envelope, legacy spider_status must fail with actual reqwest timeout under inherited60s.90s outer test budget; no paused clock, reduced cap, mocked client or constant-equality substitute. It is designed to distinguish old60s dedicated behavior; it has NOT RUN. Configured exact150s is source-pinned; runtime behavior past60s and exact expiry/latency remain unproven until execution.

No extra parent main/mcp hooks are needed beyond original v1 integration. Root copies the guarded updated tests.rs payload plus NEW sibling postfix_tests.rs; new module is test-only and inherits actual private fixture helpers. No dependency changes.

## Integration/proof boundary

Five payloads: two product replacements, existing new-test file with append-only child hook, one new test child, one guarded native script replacement. Before hashes are immutable v1 after hashes; new path must be absent. Current Root parent mcp.rs is additionally pinned to show60s legacy constructor unchanged. All original active inventory counts, paired CLI output guarantees, published source lineages,18 v1 tests and previous scripts remain unchanged.

Executed only source/hash reads and own writes, rustfmt format/parser on3changed/new files, rustfmt--check on4Rust payloads, Node--check on the one native script; all exit0. No test/fixture/native/runtime PASS. The new61s test must be budgeted as a real long directed gate when HOLD is explicitly lifted. This handoff does not finish S6/whole-feature/manufacturer equivalence or promote installed runtime.
