# Full configuration profiles

Local profile acceptance passes on the normal Windows executable
`f4b8ede57fc8dbdcb7a9c20a8c71ccf17a474c089c91bdbc52ba97e4e41eab44`.
All 39 registered UI suites have successful executions across three invocations.
The attempted whole-39 invocation still fails at a density keyboard wait.
**Stable-release readiness and aggregate regression readiness remain false.**
These qualified results support an OSS alpha source publication, not a stable
release or a successful single 39-suite invocation. Fresh exact-commit CI remains
pending.

[local-result.json](local-result.json) records the completed observations,
evidence hashes, earlier failures and outstanding gates. Raw local artifacts
remain ignored; this public summary contains no header values or absolute
machine paths. The parent source was `6398077e266083ea4ceb5ed8a4bc29c0dbdcb6ba`;
this is working-source acceptance, not a claim about a future commit.
Combined qualified source binding: [source-gates.json](source-gates.json),
including 15 passed coordinator gates and the retained failed aggregate.

## Behavior covered

The existing settings dialog can import a strict versioned profile, load the
complete selected crawl configuration, inspect effective defaults and download
a sanitized profile. Native JSON text remains authoritative. Untouched optional
absence, native-only fields, exact non-MiB byte values and the integer
`9007199254740993` survive the relevant save/reload/export paths. The latter is
a roundtrip test, not a crawl executed with that delay.

Header names become explicit, case-sensitive requirements. Values stay out of
portable profiles and browser draft storage. Explicitly resolved values **do**
become part of the locally saved native run configuration and raw project.
Runtime secret encryption or raw-project redaction is not implemented here.

Malformed or unsupported stored drafts retain their original bytes and block
both the Start button and programmatic starts. Opening Settings during a held
startup validation cannot release that guard. Importing a valid replacement
alone is insufficient: native validation and an explicit successful atomic Save
must complete. Delayed file reads and superseded native responses cannot
overwrite a newer draft or release its busy state.

Changing a toolbar target clears its previous header bindings. Reentering them
and saving Settings preserves that explicit target. Start replaces only the
declared run inputs; unchanged profile sources keep their full seed and import
provenance. The actual replacement job retains `crawl_links: false` and leaves
the original saved project unchanged.

## Executed local gates

| Gate | Actual result |
| --- | --- |
| Current R5 normal build | Pass; 564 frozen inputs unchanged |
| New profile browser fixture | Pass; 10 check groups, 26 negative controls and six verified downloads |
| Genuine native capture | Two jobs, with two original pages and one replacement page |
| Cold reopen | Two naturally closed EOF0 processes, same fresh store and same owned origin; both project byte sequences preserved |
| Offline witness | Two positive HTTP410 controls; zero product refetches through both closes |
| Earlier responsive probe | At 320 and 1440 pixels, import, selected-load, export, Save and Close are reachable; document width equals viewport width; current served UI assets are identical |
| Earlier focused Rust gates | 11 Core profile, 21 TLS and five App tests pass on unchanged profile/TLS component sources |
| Current CLI boundary | Four unit controls and two actual normal CLI children pass: one raw-config capture and one explicit portable-envelope refusal |
| Current UI state/semantic gates | 104 state tests and 40 semantic fixture groups pass under R5; these are not native/browser execution |
| Current ordinary Rust workspace | 1,802 pass, zero fail, five deliberate ignores across 17 result blocks |
| Strict Clippy / explicit Chromium | All-target/all-feature Clippy with denied warnings passes; all four named Rust Chromium fixtures actually execute and pass |
| Positive enrichment | Separate cfg(test) host, owned HTTP provider and two actual native processes pass; production-provider success remains separate |
| Separate sitemap gates | Membership passes 11 groups and assessment passes 13 groups |
| Registered UI suites | 31 successful cases before the aggregate failure, seven remaining cases, then one unchanged canonical density success; all 39 distinct names covered |

The main fixture also verifies import, inspect and export reachability at 320
pixels and performs an exact profile download there. The separate responsive
probe uses the earlier R4 binary with identical served UI assets, includes Save
and Close and ends with actual native exit zero. It does
not exercise every settings control at every viewport.

The owned site receives only the three expected capture GETs. There are no
external browser requests, renderer-worker calls or provider calls in this
profile fixture. The original site returns HTTP410 before the first native
close and remains that witness until the second close. Browser storage uses a
retained context on the exact same origin; this is not cross-origin migration.

## Aggregate regression outcome and publication scope

All named local coordinator handles are terminal. The attempted whole-39 UI
invocation records 31 successful suites, including the complete million-URL-
relationship case in 1,145,866 ms. It then fails at the density driver's Space-
collapse `waitForFunction` after its unchanged 30-second event timeout. The
outer suite deadline is not reached. The overall invocation remains incomplete
and failed; its main app closes zero with no source or normal-binary drift.

The seven remaining suites pass in a separate invocation. The unchanged
canonical density suite then passes alone; its coordinator completes in
10,265 ms. All 39 distinct registered names consequently have successful
executions across these three phases. This is **not** a successful single
39-suite run or proof that the intermittent failure is fixed. Its cause remains
`NOT_PROVEN`. An ignored instrumented diagnostic copy also passes, but is not
acceptance evidence. No assertion, deadline or product behavior was changed to
obtain the canonical isolated pass.

The registry contains nine mocked-API fixture suites and 30 live or owned-native
cases. Those fixture suites remain labelled; they are not additional native
feature acceptance or proof of every important path.

The machine-readable result retains the failed aggregate, sets
`local_feature_acceptance` to `PASS`, and keeps `stable_release_ready` and
`aggregate_regression_ready` false. OSS alpha source publication must preserve
these limits. A fresh exact-commit public CI result is still pending.

R4 previously passed 1,798 ordinary Rust tests with zero failures and five
deliberate ignores. That result precedes the new CLI parser guard and is kept
as a baseline, not a current R5 workspace pass. R5 adds an explicit portable
profile refusal to the CLI, compatibility tests and narrowly updated static
asset expectations. It rebuilds the normal executable and actually reruns the
profile browser, raw/refused CLI, 104 state tests and 40 semantic groups. The
current manifest pins 564 inputs. The earlier R3/R4 comparison still records
its exact six-rule CSS-only change; it does not describe the later R4/R5 delta.

## Source and post-run evidence boundaries

The manifest contains 564 explicitly frozen prebuild acceptance source/harness
inputs. The frozen working bytes and exact staged Git blobs for those inputs match.
It is **not** an exhaustive compiler dependency closure. A supplemental
post-run audit checks 81 literal include references and identifies two further
test-only documents outside that manifest. Their current raw bytes match the
parent commit and appear completely embedded in four current test executables
at the exact paths recorded by the R5 workspace run. No separate pre-execution
hashes were captured for those four test executables. This is supplemental
post-run evidence, not a retroactive prebuild freeze or historical executable
identity proof.

After the terminal acceptance runs, a file-specific Git `-text` rule preserves
the already tested 71-byte binary fixture when staging. Working fixture bytes
and product sources stay unchanged; no broad renormalization is performed.
The separate Git byte-preservation receipt records the old parent blob and
tested working bytes rather than claiming they were originally identical.

The actual raw-config CLI child exits zero and fetches its one owned page.
Passing a recognizable portable profile to `crawl --config` exits one with
explicit guidance to browser Settings or `/api/crawls/from-profile`. The refusal
makes no additional HTTP request, creates no requested output/store directory
and changes no prior product files. Both children actually close. This is a
tested refusal rather than CLI support for executing portable profiles or
resolving their header requirements. Legacy raw configuration and its original
typed errors remain a separate contract.

## Retained failures and limits

The first Core attempt fails compilation on a missing test import. The second
executes eight passing tests and two failures: a sitemap fixture lacks its
required namespace, and a strict tagged-unit regression exposes ignored extra
fields. Both remain recorded alongside the later focused pass.

The first browser attempt rejects an authored renderer fixture with HTTP400.
The next reaches nine check groups but fails the 320-pixel layout: the long
unwrapped selected-load label forces a 340.3-pixel grid track inside a
254-pixel container. A scoped CSS correction precedes the final pass. The first
layout probe records geometry and EOF0 but fails its own jobs-response-shape
assertion; its whole result remains a failure.

The first 39-suite invocation passes eight fixture cases, then fails the strict
comparison-fixture static asset allowlist on `/config-profiles.js`. That whole
invocation remains incomplete and failed. The later R5 invocation includes the
explicit allowlist correction and reaches the separate density failure
described above. Neither invocation is rewritten as a pass.

Parent public run
[39141990077 on6398077](https://github.com/lia-xim/open-seo-spider/actions/runs/39141990077)
is terminal failure. Ubuntu, renderer and container jobs pass; Windows fails
at optimized sitemap verification because it compares the release binary with
an inherited debug SHA. Current CI source repins the release SHA after its
release build. That correction is not publicly verified until new exact-commit
CI executes; the parent run is not an overall pass.

There is one browser-origin draft, with no named-profile collection or profile
migration. The existing CLI `--config` format remains distinct from the portable
v1 envelope and now explicitly refuses recognizable profile envelopes. Native
boundary tests and small browser-file tests are separate;
this receipt does not claim a 64-MiB browser-picker success. Optional configuration
roundtrips do not prove execution of every associated crawler feature. Full
manufacturer parity remains `NOT_PROVEN`; `storage.config` remains partial.
The ledger counts remain 144 features, 416 filters and 32 workflows, with
38 implemented, 69 partial and 37 planned features.
