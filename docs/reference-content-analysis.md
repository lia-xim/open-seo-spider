# Bounded original content observations

On 2026-10-12 the installed Screaming Frog24.3 Windows CLI crawled **22 independently authored localhost HTML cases**, made23 requests to that fixture and produced eight requested CSV exports with exit0. [The normalized artifact](reference-content-observations.json) retains our complete authored HTML/text cases, CSV hashes, path memberships, nullable measurements, controls and provenance. Original/profile/log byte snapshots remain private task-work evidence. No vendor code, font, bundled translations, profile values or credentials are included. Full product and individual whole-filter parity remain **NOT_PROVEN**; this run has no executed native filter comparison.

The ordinary installed profile was left in place. The CLI reported missing licence and still produced these exports. Observed nonsecret runtime settings were near-duplicate checkingfalse, threshold90, indexable-only duplicatestrue and readabilitytrue. The ordinary profile write changed `analytics.last_sent`; before/after hashes and the changed key are recorded. The runner used no config/auth/licence flags, did not control the original GUI session and did not roll unknown settings back. Only own-fixture requests are measured; background application network is outside the assertion.

| Export | Rows | Bounded evidence |
| --- | ---: | --- |
| Content:All | 22 | Every intended case is in the measured cohort. |
| Exact Duplicates | 2 | Identical a/b are members; changed HTML, same text with changed markup and header-noindex controls are absent. |
| Near Duplicates | 0 | Checking was explicitly disabled; negative algorithm acceptance is unavailable. |
| Low Content Pages | 6 | Tiny/empty/175-word cases are members;200/210-word controls are absent. Exact threshold/precedence not calibrated. |
| Readability Difficult | 0 | No positive measured-range fixture; remains partial. |
| Readability Very Difficult | 2 | Hard-English is a member; easy-English/tiny controls are absent. |
| Lorem Ipsum Placeholder | 1 | Lorem case is a member; easy-English control is absent. |
| Soft404 Pages | 1 | Own200 error-text case is a member; easy-English control is absent. |

All22 exported HTML digests equal direct MD5 of our exact served HTML bytes. The noindex-header variant shares the identical a/b digest but is excluded from the Exact-Duplicates filter. A markup-only change preserves authored visible text while changing that digest. This establishes a bounded distinction between HTML identity, indexability eligibility and selected-text similarity.

The hidden/style-hidden case has390 words versus the210-word baseline. Script/style additions leave210, and nav/footer additions leave210. The whitespace/punctuation case reports5 words. These observations belong to a normal raw-HTML profile; they do not establish arbitrary parser recovery or rendered visibility behavior.

Readability exports retain scores/counts rather than forcing a matching heuristic. Tiny text has6words/1sentence/score100; empty text has words0 and blank sentence/score. The hard text scores0; declared German/French easy examples also receive100. A paragraph with35 literal terminal-punctuation sentences exports45, and one long literal sentence exports19. Sentence/syllable rules and language eligibility cannot be inferred from those cases. The independent supplied-text engine keeps its versioned Unicode tokenizer, declared English/German heuristic, unsupported-language unknowns and unclamped/unrounded outputs. It does not claim original metric parity.

The official duplicate tutorial documents text MinHash, licensed enabling/content-area steps, excluding exact duplicates from near-duplicate reporting and rounded display values. This run did not enable checking and does not establish a runtime licence rejection. [Official duplicate tutorial](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-check-for-duplicate-content/). Official readability documentation describes English analysis and Difficult30–49 / Very-Difficult0–29 groups; exact boundary/rounding cases need further runtime controls. [Official readability tutorial](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-test-readability/).

A next content comparison should preserve selected raw/rendered text, selector/completeness/config provenance and explicit eligibility. Measure punctuation/whitespace and score-boundary cases before defining any reference-compatible display profile. Empty disabled/post-analysis/licence-dependent exports never count as PASS. Ordinary CLI operations follow the [official CLI documentation](https://www.screamingfrog.co.uk/seo-spider/user-guide/general/#command-line-interface).
