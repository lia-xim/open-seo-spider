# Custom search contract

Custom search is an opt-in capability of the independent crawler. It searches
explicitly selected UTF-8 sources during a crawl and persists observations for
later inspection and export. This document describes the implementation;
executed acceptance evidence is recorded separately in the feature ledger.
Reference-product parity remains `NOT_PROVEN`.

## Configuration and sources

The redesigned settings panel accepts a versioned JSON profile. The API and
existing MCP `spider_start_crawl` tool accept the same `config.custom_search`
object:

```json
{
  "version": 1,
  "rules": [{
    "name": "Brand mention",
    "method": "literal",
    "pattern": "Example",
    "sources": ["raw_html", "rendered_html"]
  }],
  "limits": {"max_saved_matches": 25}
}
```

Supported sources are `raw_html`, `rendered_html`, `raw_selected_text` and
`rendered_selected_text`. Rendered sources require the configured renderer.
Selected-text sources require an explicit content-capture profile; search never
enables capture or substitutes a different source. Raw and rendered observations
coexist. Source hashes describe the decoded UTF-8 input, not HTTP wire bytes.

Literal matching is case-sensitive and non-overlapping. Regex matching uses the
Rust `regex` engine, including its Unicode and zero-width semantics. Optional
distinct flags are `i`, `m`, `s`, `x` and `U`; literal rules cannot use flags.
Look-around and backreferences are unsupported. Rules and limits are compiled
and checked before website requests. Unknown configuration keys, positional
arrays, duplicate names/sources and unsupported versions are rejected.

## Counts, evidence and bounds

Every newly captured native page has an observation for each configured
rule/source pair. Historical absence remains supported. `count` is an exact total
only when the source and search are complete. Unavailable and incomplete sources
have a null count. `matches_observed` is a lower bound for the input actually
searched. A complete observed source with no matches has count zero.

Saved matches contain UTF-8 byte offsets, bounded matched text and excerpts.
Their truncation does not change an otherwise exact count, but makes
`evidence_complete` false. Results bind the profile, rule pattern, input hash,
source revision, document URL and observed generation. Portable validation checks
internal consistency; imported JSON is a declared observation, not proof of its
origin.

An incomplete regex context is not searched: truncating the input could fabricate
end anchors, word boundaries or greedy alternatives. Such a result is incomplete
with null count and `regex_partial_context_unknown`. Literal prefix search may
retain a lower bound. A conservative regex work allowance charges remaining
input bytes before each search, including the final EOF probe. It limits work
without changing regex context; it is not a CPU deadline.

| Limit | Default | Maximum |
| --- | ---: | ---: |
| Rules / unique name bytes / pattern bytes | — | 32 / 128 / 8192 |
| Complete serialized configuration | — | 64 KiB |
| Input bytes per source/rule | 1 MiB | 8 MiB |
| Matches scanned | 100,000 | 1,000,000 |
| Regex scan work bytes | 1 GiB | 16 GiB |
| Saved matches | 100 | 256; zero enables counts-only storage |
| Match text and excerpt bytes, each | 256 | 4096 |
| Combined serialized results per page | 1 MiB | 4 MiB |
| Complete native CSV | — | 64 MiB |

Oversized source metadata becomes explicitly unavailable. A result budget never
silently turns unknown into zero. The native CSV encoder also checks imported
page evidence before copying it, and returns HTTP 413 on overflow rather than
a partial successful download.

## Saved workflows

The sidebar results table and inspector read saved data. The inspector expands
match evidence and JSON lazily using text nodes. Full native CSV is available at
`/api/crawls/{id}/custom-search.csv` with exact byte length, SHA-256 and
`Cache-Control: no-store`. It includes all observations on saved-profile-visible
pages, independently of current table filtering and pagination; there is no raw
CSV mode. Filtered view export is labelled separately. Raw project JSON retains
all captured pages, the complete configuration and observations. Missing historical
observations stay unavailable with blank CSV counts.

Selected recrawl refreshes selected pages and preserves unrelated saved results.
Present parent observations must match their configured profile before child
creation; absence remains a supported legacy state. SQLite recovery and project
import retain the same serialized observations. Saved analysis/export performs
no website refetch.

## Verification boundaries

Focused engine, native crawl/storage, API/export and MCP tests accompany the
implementation. `ui-custom-search-live.cjs`, registered in `scripts/ui-e2e.mjs`,
tests the actual browser, Rust API and Chromium renderer, exact table/CSV/JSON
values, source separation, invalid rules, Unicode, limits, 320px layout and two
owned native processes for cold recovery. Tests use owned loopback fixtures and
fresh stores, leaving the installed app untouched. They do not establish all
vendor features, every regex combination, other browser engines or origin
attestation for imported data.
