# Profile browser-response capture — scoped local acceptance

Actual Chromium/native checks pass on **582 frozen inputs**, using unchanged normal Windows binary `c780fa04c09a7f8a20e1ba5f61368fb0df2d729b85785fbcb34dbe95cae0ed59`. This change affects verification code and CI registration. Product Rust and embedded redesign assets are unchanged.

- **180 state/helper tests** pass, including Promise/Response identity and explicit failure/resource controls.
- **12 reported Chromium tests** (11 subcontrols) pass with exactly15 HTTP transport requests,6 document requests and zero CDP body/text/json calls. Original HTTP bytes remain consumable after clone-only limits/timeouts; opaque-page transitions produce zero pageerrors.
- **Three actual native suites pass in one invocation:**10 portable-profile groups,4 named-catalog groups and3 viewport/screenshot groups. There are45 exact captured inspect responses,11 independent SQL snapshots and8 natural native EOF0 exits, plus the coordinator's natural exit0. All three suites have zero pageerrors/external requests/offline product requests and two positive410 controls each.

The allowlist is exact same-origin POST `/api/config-profiles/inspect` and GET catalog list/UUID item. Capture binds document epoch, awaited pre-action sequence, method/full URL/raw request SHA, status and normalized headers. GET request data must be absent. Duplicate or stale association refuses; there is no replay, retry or CDP fallback. The original synchronous fetch Promise and Response are returned unchanged. Existing catalog mutations and crawl starts keep one original response capture.

Fixture limits are2MiB per request/response,16MiB charged aggregate bytes,64 records per document,32KiB transfer chunks and a10s clone-only read deadline. They are test-fixture limits, not product maximum-size acceptance or global physical-memory guarantees. Captured buffers are released after transfer.

The Node-SQLite fixture now uses an extended Windows filename for its two database opens. Independent controls at220/252 characters and the subsequent long-output catalog run verify this test-only correction. Native store paths and SQL read-only flags are unchanged.

Prior failures remain failures: R1 initialized the observer on about:blank; R2 exposed the Node-SQLite ordinary-path limit; R3 failed actual CDP body reading during cold catalog GET. Their logs and hashes are retained below. The previous public513 renderer failure is also retained; its Chromium root cause and a product defect are not established.

**Fresh exact public CI is pending.** All41 local UI suites and1,831 Rust tests passed on prior b7d2e75; this follow-up does not claim those complete runs again. It does not update the installed pilot or establish all-browser/load/max-size behavior, a stable release, or manufacturer parity. The full144/416/32 ledger remains39 implemented,68 partial,37 planned.

Evidence: [local result](local-result.json), [source gates](source-gates.json), [actual profile flow](profile-live.json), [actual catalog flow](catalog-live.json), [actual screenshot flow](screenshot-live.json), [browser controls](capture-controls.log), [state/helper log](state-and-capture.log), [prior failures](prior-failures.json), [independent path controls](node-sqlite-path-controls.json).
