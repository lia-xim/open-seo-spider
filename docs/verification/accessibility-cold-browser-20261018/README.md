# Accessibility review and cold specialist analysis

The redesigned dashboard now has a saved Accessibility table and inspector,
with a complete native CSV export. The local Windows browser acceptance passes on the
normal executable SHA-256
`f493874eff33d2dde33ba5d9634599fdf63914dbbfd57e64bd9162b6d9dc41de`.
This is an independent automated-results viewer, not manual accessibility
verification or proof of WCAG conformance. Reference-product parity remains
**NOT_PROVEN**.

- Actual pinned axe 4.14.0/Chromium captures 59 owned pages. The saved reporting
  profile exposes 58. Fixtures produce valid zero findings, genuine incomplete
  color-contrast observations, button-name/image-alt violations and an actual
  engine error. Missing, malformed and failed observations retain unknown
  counters. Declared imported negative controls are separate from native data.
- The browser compares every ordered cell in the first 50 and final 8 rows.
  Five actual-helper controls reject duplicated, missing, misplaced, reordered
  or wrong cells; two positive slices pass. Four separate frontend-closure VM
  regressions cover hostile imported metadata and late export errors.
- The inspector shows rule impact, tags, selectors, inert HTML and check
  details. Complete CSV includes all visible saved pages, every violation and
  needs-review node, exact raw observation JSON and its digest. Its node
  multiset and every flattened field are checked, independently of table paging
  or filtering. Native length/SHA-256/no-store/generation headers are verified.
- Four native tests cover unknown/zero distinctions, profile filtering,
  external pages,205-page complete output, active/stale refusal and atomic
  budgets. The physical 64 MiB CSV response succeeds at the exact limit and
  refuses one added byte before CSV success headers. Selected page JSON has
  a 4 MiB pre-copy budget; physical overflow and small injected exact/+1 budgets
  are tested. Exact physical 4 MiB success is not separately proved.
- Two normal native processes reopen the original accessibility observations
  and byte-identical CSV with zero website requests or repeated axe audits.
  Populated 1440px/320px layouts are exercised.

The new N-gram restart suite uses three genuine saved crawls, 105 unique
unigrams and exact 100+5 DOM paging. N=1/2/6, case, query, stopwords, missing DOM,
empty/incomplete/truncated captures and retained title provenance remain
distinct. Nineteen full CSVs and eight page JSON downloads are checked;54
read-only native response oracles bind browser metadata and content. Nine CSV
header mutations and four source URL/ordinal mutations are rejected by the
actual positive validators. Options remain document-local and are explicitly
reapplied after reload; persisted server options are not claimed.

The segment restart suite uses 109 genuine captured pages, seven warm/cold
projections, exact 100+5 reported and100+9 raw DOM paging, 28 complete JSON/CSV
downloads and 47 read-only native oracles. All six CSV cells are checked for
page, issue and metadata records; seven mutated-cell controls are rejected.
Browser-local ordered rules are restored using one browser context and the
same owned app origin. Native saved data is a separate persistence boundary.
Declared empty/missing/title controls do not claim native capture provenance.
The 320px segment table can have very tall wrapped rows. No-overflow and export
reachability checks do not establish compact mobile readability; that remains
open in the feature backlog.

Both specialist suites retain the original website as an HTTP 410 witness from
before the first stdin EOF through both natural process closes and browser
close. Two nonce-tagged positive witness probes are separate from zero product
refetches. This does not prove crash recovery, a fresh browser profile, every
operator, large-corpus performance or full feature parity.

All 35 UI suites pass together: nine explicit API-fixture suites and 26 native
suites. Their app closes naturally with exit 0; tracked sources and executable
stay unchanged during the run. After that terminal run, only two cold test
assertions were strengthened; both updated suites pass in a separate focused
run on the same binary. Product inputs stay unchanged. This is not
claimed as a second whole 35-suite run with the updated assertion files. Three
native test fixture initializers were subsequently restyled for strict Clippy;
the four affected native tests and Clippy are separate repeated gates.

Warm N-gram and segment tests also replace unavailable CDP response bodies with
unconditional read-only native response oracles, retaining exact DOM, paging,
error and complete-download assertions. Mutation requests are never replayed.
The historical 081bf33 public renderer failure is not retrospectively relabelled
as passing. Fresh commit CI remains a separate gate.

See [local results](local-result.json), [source and artifact bindings](source-gates.json)
and [the E2E guide](../../e2e-testing.md). Raw stores, screenshots and logs remain
ignored local artifacts. No installed application, user store, production
provider, billing limit or release is changed by these checks.
