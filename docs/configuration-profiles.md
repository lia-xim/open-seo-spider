# Configuration profiles

Open Settings and expand the configuration-profile section to import a JSON
file, inspect/edit its native declaration, export the current working copy or
load the complete configuration of the selected crawl. Saving commits one
sanitized browser-local draft. Cancelling keeps the previously saved draft.
Import, inspection and export do not start a crawl.

A damaged or unverified stored draft keeps Start blocked and displays a
persistent warning. Opening or closing Settings does not bypass that guard.
Recovery requires an explicitly selected valid profile and a successful Save;
failed validation or storage writes retain the previous stored bytes.

The portable format is versioned:

```json
{
  "version": 1,
  "config": {
    "seed_urls": ["https://example.com/"],
    "crawl_links": false,
    "max_body_bytes": 1572865,
    "custom_headers": {}
  },
  "required_headers": ["Authorization"]
}
```

The configuration uses native `CrawlConfig` fields. It is transported as JSON
text, including patch values, so integers through the native unsigned 64-bit
range and exact byte budgets do not pass through JavaScript floating-point
conversion. Absent and explicitly null optional fields remain distinct in
the declaration. The effective JSON view shows native defaults and
normalization. Existing form controls patch only changed fields.

An unchanged toolbar retains all declared seeds, sitemaps and list-import
provenance. Editing the toolbar target, mode or list explicitly replaces that
run's source declarations. It does not infer a new `crawl_links` setting from
the toolbar mode in profile mode.

Every custom header becomes an exact-name unresolved requirement when a
native configuration is converted or loaded from a crawl. Bindings are entered
explicitly, may intentionally be empty and remain outside browser-local
profile storage and profile downloads. Missing, partial or undeclared bindings
refuse Start. Changing profiles or targets clears bindings. A genuine crawl
still stores its complete native runtime configuration, including supplied
header values, in the local crawl store and ordinary project reports. Profile
sanitization does not redact those existing report formats.

The pure boundary rejects duplicate and unknown fields, unsupported versions,
positional struct arrays, invalid native combinations, credential-bearing
HTTP(S) URLs and private-key material. It does not load referenced filesystem
paths, inherit environment credentials, contact providers or construct HTTP
clients. Explicit Start alone resolves bindings and enters ordinary native
crawl validation, admission and persistence. Existing `/api/crawls` aliases
retain their previous behavior.

Each profile/declaration/effective JSON document is limited to 64 MiB, including
after default expansion. Parsing has depth64 and one-million-node bounds. Two
preparation slots per process cover request buffering and tracked blocking
work; JSON-text request wrappers and escaped replies have separate bounded
envelopes. These are explicit format limits, not general memory or performance
guarantees. A profile without targets may be saved; Start requires targets.

The API engine is `open-seo-spider/crawl-config-profile-v1`:

- `GET /api/config-profiles/default`
- `POST /api/config-profiles/from-config` with `config_text`
- `GET /api/crawls/{id}/config-profile`
- `POST /api/config-profiles/inspect` with `profile_text` and optional `patches`
- `POST /api/config-profiles/export` with the same declaration and patches
- `POST /api/crawls/from-profile` with explicit optional `header_values`

POST requests require `application/json`. A patch uses a config-root key path
and either one JSON-text `value_json` or `remove: true`. Removal requires an
existing key. Paths address objects, not array indices. Patches are applied
transactionally and re-enter the same native parser. Export returns an actual
JSON attachment with a length and SHA-256 header; preview replies keep their
three native documents as strings.

This portable envelope is a browser/API profile format. The existing CLI
`--config` input remains a raw native configuration JSON object. Recognizable
profile envelopes are explicitly refused before creating a client, output or
store, with directions to browser Settings and `/api/crawls/from-profile`.
The combination of top-level `config` with `version` or `required_headers` is
reserved for this refusal. Other raw parsing still uses the original bytes,
including native option values and duplicate-field errors. Executing portable
profiles through the CLI, named profile catalogs, synchronization and general
vendor-format compatibility remain outside this format's acceptance.

See [E2E testing](e2e-testing.md) for the registered browser workflow and
[the feature ledger](FEATURE_TODO.md) for remaining scope and proof gates.
