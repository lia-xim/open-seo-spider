# Independent post-crawl analysis

`crates/app/src/analysis.rs` exposes `analyze_report(&CrawlReport) -> serde_json::Value`. It reads an existing snapshot and performs no network requests. The application owns the HTTP endpoint and report integration.

The result contains `coverage`, `graph`, `canonical_cycles`, `structured_data`, `issues` and `limitations`. Issues use the existing core schema: `code`, `severity`, `url`, `message` and `related_urls`. A code already emitted for the same URL in `report.issues` is omitted from the additional analysis issues.

## Implemented calculations

- Graph nodes represent observed internal URL records; graph edges represent observed anchor links between those records. Resource references do not become anchor edges. Repeated anchors contribute occurrence counts while unique source/target pairs contribute degrees.
- PageRank uses damping 0.85, uniform teleportation and dangling-node redistribution, a maximum of 200 iterations and L1 tolerance 1e-12. Nofollow, ugc, sponsored and page nofollow do not pass score. Duplicate anchors do not amplify score. The largest rank becomes a normalized score of 100. Convergence metadata remains visible.
- Stored crawl-depth distribution and shortest observed anchor depth are separate. Zero observed inlinks produces an orphan **candidate** only for usable non-seed HTML. A partial crawl never becomes a complete-site claim.
- Canonical checks distinguish unobserved targets, robots blocking, failed requests, directly redirected declarations, non-200 status, noindex, conflicting targets, further canonicalization and cycles. A final URL observed through a redirect can supply document facts without falsely declaring that final URL itself a redirect.
- Hreflang checks include self-reference, conflicting targets per normalized language label, return links and the same target-state checks. Missing return links are asserted only for complete, usable target HTML.
- HTTPS checks examine HSTS/CSP/Referrer-Policy presence, `nosniff`, and X-Frame-Options or CSP frame-ancestors presence. These observations do not prove complete security-policy quality. Unencrypted resources and HTTP anchor navigation are distinct findings.
- JSON-LD analysis reports parsed and invalid JSON block counts and recursively observed type facts, excluding type-coercion definitions inside `@context`. It performs **no Schema.org vocabulary or Google rich-result validation**.

## Focused verification

Eight deterministic unit tests are defined in the module:

1. Duplicate anchor counts, unique degrees, resource exclusion, shortest depth, normalized scores and input-order invariance.
2. Empty and dangling graph behavior, conserved rank and nofollow exclusion.
3. Canonical cycles versus unobserved targets.
4. Initial redirect status, noindex evidence and final-alias handling.
5. Hreflang self/conflicts/noindex/canonicalization/returns and incomplete-target handling.
6. Case-insensitive security headers, CSP framing alternative and navigation/resource distinction.
7. Nested JSON-LD types versus context definitions and explicit unperformed validation.
8. Suppression of existing report issues, limited/pending coverage and seed exclusion from orphan candidates.

Executed natively on Windows on 11 October 2026 after the application declared `mod analysis;`:

```text
Command: C:\Users\matth\.cargo\bin\cargo.exe test -p open-seo-spider analysis::tests -- --nocapture
Finished test profile; initial dependency/build time: 36.82s
Result: 8 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
```

These focused tests prove the stated fixture behavior and successful application compilation at that point. HTTP integration, browser behavior and large-site performance are separate gates. Feature-parity statuses remain unchanged until integrated acceptance evidence exists.

## Scope still open

The report contains neither a full body corpus nor fetched content for every unobserved target. Near-duplicate/semantic body analysis is consequently not inferred. Graph scores characterize this captured graph rather than Google PageRank or any proprietary product metric. Redirect records contain final-document extraction associated with the originally requested URL; they remain observable graph records rather than silently merged identities. Complete reference-product parity and large-crawl performance are not proven by these focused tests.
