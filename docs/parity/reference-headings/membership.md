# Bounded original heading-order membership

Observed 2026-10-12T09:28:04.233Z; original24.3 ordinary CLI, exit 0, 26 local paths. Full vendor parity **NOT_PROVEN**.

| Case | Tag order | H1 All | H1 non-sequential | H2 All | H2 non-sequential | Source prediction mismatch |
| --- | --- | --- | --- | --- | --- | --- |
| / | 1 → 2 | true | false | true | false | none |
| /h1-h2 | 1 → 2 | true | false | true | false | none |
| /h2-h1 | 2 → 1 | true | true | true | false | h2 |
| /h1-h3-h2 | 1 → 3 → 2 | true | false | true | true | none |
| /h1-h2-h3-h2 | 1 → 2 → 3 → 2 | true | false | true | false | none |
| /h3-h2-no-h1 | 3 → 2 | true | false | true | false | h2 |
| /repeated-h2 | 1 → 2 → 2 | true | false | true | false | none |
| /h2-only | 2 | true | false | true | false | h2 |
| /h3-h1-h2 | 3 → 1 → 2 | true | true | true | false | none |
| /empty-h1-before-h2 | 1 → 2 | true | false | true | false | none |
| /empty-h2-before-h1 | 2 → 1 → 2 | true | true | true | false | h2 |
| /hidden-h2-before-h1 | 2 → 1 → 2 | true | true | true | false | h2 |
| /css-hidden-h2-before-h1 | 2 → 1 → 2 | true | true | true | false | h2 |
| /hidden-h1-before-h2 | 1 → 2 | true | false | true | false | none |
| /image-alt-h1-before-h2 | 1 → 2 | true | false | true | false | none |
| /image-alt-h2-before-h1 | 2 → 1 → 2 | true | true | true | false | h2 |
| /image-alt-only-h1-h2 | 1 → 2 | true | false | true | false | none |
| /empty-h2-after-h1 | 1 → 2 | true | false | true | false | none |
| /no-headings | no headings | true | false | true | false | none |
| /h3-only | 3 | true | false | true | false | none |
| /aria-heading-before-h1 | 1 → 2 | true | false | true | false | none |
| /h1-h3-h1-h2 | 1 → 3 → 1 → 2 | true | false | true | true | h2 |
| /h1-h1-h2 | 1 → 1 → 2 | true | false | true | false | none |
| /h1-h2-h1-h2 | 1 → 2 → 1 → 2 | true | false | true | false | none |

The independent column is a snapshot of the current newly authored `audit_dom_metadata` predicate, not an executed Rust parity result. Raw owned HTML is in fixture-cases.json; exact export paths, hashes and rows are in latest-run.json. Hidden attributes, inline CSS and image alt are raw facts; rendered visibility is not inferred.

Profile bytes: before d8fc4dbcbaceaaadfcd638ae8578e4ab207ff98a5f3fbb4a79c28cb249217409, after 1284aa1fbf678cef66d303e7e37bdab255ba042f9d5a9f9bea4d0b0bf1c9c368. Noncomment content unchanged: true. Changed property keys: none. Private byte snapshots remain only in task work; no profile writes or rollback by the runner.

Sources: [H1 non-sequential](https://www.screamingfrog.co.uk/seo-spider/issues/h1/h1-non-sequential/), [H2 non-sequential](https://www.screamingfrog.co.uk/seo-spider/issues/h2/h2-non-sequential/).

Rerun with `node scripts/reference-headings.mjs`; this creates a new ordinary reference observation without changing existing settings.
