# Optional MCP file tools

This is a new source implementation. Actual focused Windows tests now pass:
15 module groups and 14 CLI/Bridge/STDIO/HTTP groups. The HTTP group uses three
owned server modes, including the enabled second loopback address. The
[initial failures](verification/mcp-file-tools-source-20261015/focused-module-first/index.json)
and [repaired module](verification/mcp-file-tools-source-20261015/focused-module-repaired/index.json) /
[transport attempts](verification/mcp-file-tools-source-20261015/focused-wiring-repaired/index.json)
retain their exact source bindings and logs. These results do not establish
official SDK, Unix, combined full-workspace or installed-runtime acceptance.
The installed application acquires these capabilities only after a new candidate
passes its gates and is installed.

## Configuration

File access is disabled by default. An operator can grant one existing directory
when starting a transport:

```sh
open-seo-spider serve --listen 127.0.0.1:8787 --mcp-files-dir ./agent-files
open-seo-spider mcp --server http://127.0.0.1:8787 --mcp-files-dir ./agent-files
```

Create the directory before starting the process. The HTTP application and a
separate STDIO process each own their configuration; enabling one does not enable
the other. HTTP file tools require a loopback listener. Invalid directory or
listener configuration fails startup before the application opens its listener
or creates its crawl-data directory.

The default source registry has 26 tools. Enabling file access appends these five:

| Tool | Operation |
| --- | --- |
| `spider_list_allowed_directory` | Discover the granted root marker `.` and supported file profile, without exposing its host path. |
| `spider_list_directory` | Read a bounded, sorted directory inventory with continuation. |
| `spider_create_directory` | Create directories within the granted root. |
| `spider_read_text_file` | Read UTF-8 text in byte-addressed pages bound to a content snapshot. |
| `spider_write_text_file` | Write UTF-8 text by replacing the target with a sibling temporary file. |

These counts describe source registration, not a runtime acceptance result.
File operations use the held directory capability directly and do not fetch the
crawler's HTTP API or any website.

## Paths and file profile

Pass relative paths with `/` separators. `.` identifies the granted directory
for listing. Absolute paths and `..` traversal are rejected; Windows also rejects
drive-qualified paths, UNC paths and backslash separators. On Unix a backslash
is a literal filename character. The profile accepts at most 64 KiB of UTF-8 path
data and 256 components, subject to the operating system's own name and path limits. Windows
device names, alternate data streams and trailing-dot or trailing-space aliases
are rejected on Windows.

The internal `.__open_seo_spider_mcp_tmp_` filename prefix is reserved. A listing
can describe names that this path profile cannot address; their
`addressable_name` value is false. A non-UTF-8 name makes the listing fail
explicitly, rather than returning an ambiguous lossy spelling.

The implementation opens each directory component without following links and
checks an opened file before reading. Text reads and writes support regular
files. Symbolic links, junction traversal and special files are outside this
profile. Existing directory handles remain the authority during an operation;
string-prefix checks alone do not grant access. On Windows a held directory
handle can prevent renaming or deleting that directory.

## Resource and output contracts

Reads capture at most 32 MiB, validate UTF-8, and return a SHA-256 content hash
plus a snapshot token bound to the file path and running file-tool instance.
A page defaults to 64 KiB and can request at most 256 KiB. Page offsets
and boundaries must preserve UTF-8 characters. Continuation uses the returned
offset and snapshot; a changed file is an error rather than a continuation of
different content.

Directory listings scan at most 10,000 entries and 2 MiB of entry names, sort
the observed inventory and return at most 200 entries per page (100 by default).
Continuation is bound to the running file-tool instance, directory and its
observed names-and-types inventory; it does not track file-content changes.
Exceeding a scan or response budget is an explicit error; a shortened inventory is not
reported as complete. Directory enumeration does not provide a transactional
filesystem snapshot against concurrent external writers.

Recursive directory creation reports its created components. If a later
component fails, directories already created can remain; the operation does not
promise transactional rollback.

Writes accept at most 1 MiB of UTF-8 text. A new sibling temporary file is
written and synced before rename replaces the destination. Replacing an existing
regular file is permitted. The implementation does not edit an existing inode
in place, so a hard link does not turn a write into modification of another
linked pathname. File sync and rename do not establish power-loss durability
of the parent directory on every operating system.

Identity checks around temporary-file cleanup and rename are observations, not
filesystem compare-and-swap operations. The supported MCP operations cannot
address reserved temporary names. An external process that can concurrently
replace entries in the granted directory can still race those namespace
checks; the file tools do not promise an immutable namespace against that
process. A rare staging-metadata failure reports the retained relative
temporary name when safe identity-based cleanup cannot be established.

At most four blocking file jobs run concurrently. A dropped request retains its
actual blocking-task join handle; shutdown closes admission and awaits the
registered jobs. This provides ownership and drain accounting. It does not
promise a hard completion deadline for a stalled disk or network filesystem.

Tool results contain a short text summary and one structured result. The
complete JSON-RPC response, including the ID, wrapper and STDIO newline, is
bounded to 8 MiB. File-tool calls reject non-integer IDs and string IDs above
1,024 UTF-8 bytes before performing a file operation.

## Remaining MCP scope

This implements a bounded local file profile. Optional Node/npm processing,
complete report/bulk-export/field/filter discovery, full selected-format export
and the remaining stored-content workflows are tracked separately in the
[feature ledger](feature-parity.json) and [feature TODO](FEATURE_TODO.md).
`automation.mcp` remains partial. Full Screaming Frog parity is **NOT PROVEN**.
