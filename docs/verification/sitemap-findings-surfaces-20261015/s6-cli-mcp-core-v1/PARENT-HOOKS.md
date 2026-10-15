# Parent hook contract (source-only, Root owns application)

Read baseline main.rs SHA dc86c227a229e00001c194f74d3e1c15101cda9f4381a0adc9dfc9644841016d; mcp.rs SHA 887e342dff02a9c7d71673576726e0f1d7c208c1b01a12a67cc312f83cb9c36e. These bytes already include Root-owned parallel registrations if present; do not replace whole files from an old baseline.

main.rs: add `mod sitemap_findings_cli;` alongside Root-owned `mod sitemap_findings;`. Command enum adds:

```rust
/// Read a local findings page and/or export the complete fixed selection; no website requests.
SitemapFindings {
    store: PathBuf,
    #[arg(long)]
    request: PathBuf,
    #[arg(long, required_unless_present = "csv")]
    output: Option<PathBuf>,
    #[arg(long)]
    csv: Option<PathBuf>,
},
```

Dispatch:
```rust
Command::SitemapFindings { store, request, output, csv } => {
    let receipt = sitemap_findings_cli::run(store, request, output, csv).await?;
    println!("{}", serde_json::to_string(&receipt)?);
}
```

mcp.rs add `mod sitemap_findings;`. Immediately AFTER request.is_object rejection and BEFORE cloning id/jsonrpc version or args/global schema errors, replace the old `let id = request.get("id")?.clone();` with:
```rust
let id_ref = request.get("id")?;
if request["method"] == "tools/call"
    && request["params"]["name"] == sitemap_findings::NAME
    && !sitemap_findings::valid_rpc_id(id_ref)
{
    return Some(error(Value::Null, -32600, "Finding RPC id must be an integer or at most 1024 UTF-8 bytes"));
}
let id = id_ref.clone();
```
This placement bounds wrong-jsonrpc and argument-error replies as well as successful new-tool calls. Old tools retain their existing policy. No later duplicate ID guard is needed.
After the existing global schema/missing checks, before old special branches:
```rust
if name == sitemap_findings::NAME {
    let result = sitemap_findings::call(self, args).await;
    return Some(sitemap_findings::complete_response(id, result).await);
}
```

Inventory adds `tools.push(sitemap_findings::definition());`. Existing `stdio_stream<R,W>` function keeps generic bounds and delegates body once to `sitemap_findings::stdio_stream(bridge,input,output).await`; remove `mut` from output argument. Production imports `tokio::io::{AsyncRead,AsyncWrite}` remain. Preserve old stdio tests byte-for-byte by moving formerly production AsyncBufReadExt/AsyncReadExt/AsyncWriteExt/BufReader imports to a cfg(test) use; the old tests inherit these from super. MAX_STDIO_REQUEST_BYTES remains production-used by the child.

New MCP tool args use optional `include_export_descriptor: bool`. Descriptor is POST relative findings/export.csv + exact fixed selection and required export token, never CSV payload. Current SDK cancellation semantics pinned by local official @modelcontextprotocol/core CancelledNotificationSchema and client sender/handler: notifications/cancelled params.requestId. Active findings alone pumps input, preserving one queued frame plus one partial bounded frame. Additional requests get a small -32000 busy error; further notifications are dropped while slot full. Matching cancel always observed, active result -32800, actual request future dropped; EOF drops active and ends. Legacy framing/serial calls remain delegated through actual entrypoint and all existing stdio tests remain required.

Shared dependency: adapter manifest 5a0a2e7c22b345b689339deb0bd18a7ecb4292c127bf0c9e69b0f389f99101bb; module e6728e4e9e1558732d51c5c10e1fb24834ab97abb7d33d25574c6c98e448b31d. Core session page/selection manifest 8ca736921c66be73a0624294df3bdee9b30e0cddc13a2688c4eec66e1766d5e5.

No builds/tests/native runs under HOLD. Child files are still being authored; final manifest will pin immutable payload bytes.

Final wire shape: `structuredContent` contains the seven actual shared Envelope fields at its root (CorePage remains exactly once in `findings`), plus `export_descriptor` only when explicitly requested; an unavailable envelope gets descriptor:null. It is NOT wrapped in another `findings` envelope. Concise text never duplicates page data. Entire JSON-RPC compact response has cap8MiB-1, reserving the actual stdio newline so full frame<=8MiB. RPC strings are<=1024 UTF-8 bytes and integer IDs remain exact serde numbers. HTTP errors preserve status and original bounded API error under structuredContent.api_error. Partial JSON is never published as a success.

Root count hook: replace the one `assert_eq!(inventory().len(), 25);` with26. Nine separately guarded counter payloads are in COUNTER-BINDINGS.json: two existing Rust children and seven current active harnesses, including correction of old active resource24 and two stale new-run descriptions24. Do not modify historical accepted docs/verification/installed receipts or frozen harness copies.

Accepted paired-output publication limit: if second hard-link publication races and fails, preserve the COMPLETE first output, preserve concurrent second winner, return explicit error naming retained first path and partial-pair state, and return no success receipt. No unsafe check/delete rollback and no atomic-both/crash transaction promise.
