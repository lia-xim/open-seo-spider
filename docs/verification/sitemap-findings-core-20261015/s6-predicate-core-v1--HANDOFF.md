# S6 pure finding predicates: guarded source handoff

Two NEW files only. Root owns integration; require both target paths absent. No parent, repository, Git, database, installed artifact or user runtime was modified. Authoritative parent model is frozen model723200ced8fafb9652a89f6a8f19fa416dffbee3a62ddd780093c5380e36d1ef from s6-findings-model-v1 manifestd7d62bd04bb4fb5fcd9befec5623b93b8c4bdc6b90f0cdf697546ae02307079c. Its existing `pub(crate) mod predicate;` resolves this child. Tests are declared by the child itself; no parent test hook is required.

Exact private APIs:

- `assess_saved_page(filter: SitemapFindingFilter, candidate: &SitemapFindingPageCandidate, coverage: &SitemapCoverage) -> anyhow::Result<SitemapFindingAssessment>`
- `assess_source(filter: SitemapFindingFilter, evidence: &SitemapSourceFindingEvidence) -> anyhow::Result<SitemapFindingAssessment>`

The seven granular_id values match all seven IDs in the frozen acceptance audit TEST-CONTRACT.json exactly. No parent/model delta is needed. Strict wrong-domain calls return the shared InvalidSelection error. This code does not implement stored traversal, source enumeration, same-TX guards, pagination/cursors, export or adapters. Those remain Root/peer acceptance gates.

## Semantic boundary

- Original membership enum plus matching current/historical coverage permission supplies narrow negative membership. Aliases cannot supply positive membership. Known observed positives survive unrelated source incompleteness.
- Orphan requires listing and zero captured nonself anchors, retaining CapturedGraphOnly; observed nonself anchors or known not-listed is a known false conjunct. The caller must supply actual valid terminal saved-graph candidates, not fabricate page rows when analysis is unavailable.
- Multiple sources uses the existing DISTINCT original XML task count, never occurrence/observation/final-alias counts. More than one is positive; at most one is negative only with existing closed-cohort permission.
- Non-indexable is a three-valued conjunction. Known not-listed, or complete directly indexable target, makes the conjunction negative independently of uncertainty in the other operand. Native redirect/non200 and qualified real robots disallow are positive target witnesses before later body/render/PDF failures. A blocked robots result with a warning or unobserved warning field is Unknown. Rendered navigation/non200 never substitutes for original native HTTP status.
- Complete or Disabled typed PDF analysis can qualify a complete local metadata conclusion. Partial/Error/Encrypted, missing typed status/object, or unobserved PDF field remains Unknown absent independent native obstruction. Failed/truncated/required-unobserved rendering and missing target observations remain Unknown. Saved reasons are opaque: no directive, canonical, failure or PDF meaning is inferred from human text. Bot scoping/conflict resolution/raw-render provenance stay with the existing saved audit and actual producer tests.
- Source reader supplies `assess_sitemap_source_protocol_limits` from its validated actual record. This predicate checks measurement identity agreement and maps the existing verdict unchanged. It does not duplicate numeric eligibility/threshold rules or infer SQL retention from capture retention/expected ID. The reader owner confirmed it uses the same portable measurement validation limits and attaches only actual exact B links. A-only records stay independent.
- LegacyDocument and UnmeasuredTask retain distinct Unknown reasons, never numeric zero/within-limit. Actual source reader owns XML-only enumeration and suppressing duplicate document/task evidence; a global omission marker must never be forged into a measured attempt.

## Authored tests and proof limits

18 named pure-model groups are enumerated in TESTS.json. The source numeric inputs are synthetic caller-supplied facts, sealed with the actual measurement constructor and passed through the existing projector. There is no HTTP/XML/PDF/browser/Store authenticity or physical size claim. Tests cover exact seven IDs/domains, membership/negative-cohort truth tables, orphan/multiple identities, conjunction short circuit, native obstruction priority, robots qualification, complete metadata versus missing reason, unavailable target states, PDF status matrix, rendering provenance, exact threshold boundaries for urlsets/indexes, structural-versus-eligible counts, failed-decoder lower bounds, received-versus-decoded extent, legacy/pending/done Unknown and mismatched protocol identity.

Actual rustfmt format/parser and --check exit0. NO Rust compiler, Cargo, tests, SQLite, native process, HTTP, browser, renderer or large generator executed under the Compute HOLD. These are 18 authored / 0 executed tests. Actual stored full-cohort selection (>200), cursor/context/tail-corruption tests, adapter parity, physical threshold producers and renderer/PDF/bot-scoped producer controls remain NOT_RUN. Manufacturer predicate equivalence remains NOT_PROVEN.
