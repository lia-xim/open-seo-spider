# Named local configuration profiles

Status: locally verified in the [catalog and full-regression receipt](verification/configuration-profile-catalog-20261018/README.md). Fresh exact-commit public CI and broader vendor parity are separate open gates.
The portable [version 1 format](configuration-profiles.md) remains authoritative;
catalog metadata is separate from that document.

The settings dialog lets the operator explicitly list profiles stored in the selected native
application data directory. Loading one stages the working copy and clears
volatile header bindings. The existing settings Save action adopts that copy
for the next crawl. Creating, replacing, renaming or deleting a catalog entry
does not apply it, start a crawl or change an already active copy.

The application does not select a catalog profile automatically at startup.
Native persistence is independent of browser-local draft storage, so it must
survive a fresh browser origin and a second native process.

## Native storage and revisions

The fixed database is `configuration-profiles/catalog.sqlite` below the chosen
application data directory. UUIDs identify rows; display names never form paths.
The existing native SQLite opening boundary handles Windows paths.

One transaction stores the display name, canonical portable profile text, its
SHA-256 and byte length, revision, creation time and update time. Replace,
rename and delete require both the expected revision and profile hash. A stale
writer receives a conflict and cannot overwrite newer data. Revisions travel
as decimal strings rather than JavaScript numbers.

An absent catalog lists as empty without creating its directory or database.
Corruption, an unsupported schema or inaccessible existing data is an explicit
error. It must never appear as a valid empty catalog or reset an active draft.
Invalid profile input must fail native validation before filesystem writes.
Header values, provider credentials and filesystem references are not added to
the portable profile format.

The declared limits are 128 live entries, 256 UTF-8 bytes per nonblank display
name, 64 MiB per canonical profile and 256 MiB of live canonical profile text.
Names preserve their exact Unicode text and have no control characters. Exact
names are unique. These are logical data limits, not physical SQLite journal,
temporary-disk, process-memory or performance guarantees.

## API contract

The collection is `/api/config-profile-catalog`:

- `GET`: metadata and limits only.
- `POST`: `{name, profile_text}` creates an entry.
- `GET /{id}`: entry metadata and the existing native profile preview envelope.
- `PUT /{id}`: `{expected_revision, expected_sha256, name, profile_text}` replaces.
- `POST /{id}/rename`: `{expected_revision, expected_sha256, name}` renames.
- `DELETE /{id}`: `{expected_revision, expected_sha256}` deletes.

Metadata contains `id`, `name`, string `revision`, `profile_sha256`,
`profile_bytes`, `created_at` and `updated_at`. Writes require JSON and retain
the strict duplicate-key and unknown-field rejection of profile preparation.
Validation, missing entries, revision/name conflicts and unavailable storage
are distinct outcomes. No operation silently retries a conflicted mutation.

## Acceptance contract

The real browser path must create two different profiles, load without applying,
replace, rename and delete them, then explicitly apply and crawl a selected
copy. Compare the full persisted native configuration, exact large integers,
required header names, real attachments and original profile bytes.

Two tabs must demonstrate stale replace/delete refusal without losing the
intended working or active copy. Delayed actual responses must not override a
later selection, edit or closed dialog. Hostile-looking Unicode/path names
must display as text and never become paths. Invalid input, corrupt storage
and unsupported schemas must not fall back to an empty/default profile.

After an actual clean native close, a new native process and fresh browser
origin must see the same catalog without relying on browser-local storage.
Prove the selected copy through a genuine owned crawl, then retain a positive
offline witness through both process closes. At 320 px, all catalog actions
and real downloads must remain reachable.

Catalog execution through the CLI, cross-machine synchronization, shared team
profiles, version history, general format migration and manufacturer parity
remain separate backlog items. Overall coverage is still recorded in
[FEATURE_TODO.md](FEATURE_TODO.md).
