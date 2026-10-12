# Content selection: retained reference facts and independent capture contract

Reviewed 12 October 2026. This document distinguishes public requirements, retained ordinary original CLI observations and the independently authored B8 capture profile. It changes no catalog status. The subsequent [B8 final aggregate](verification/content-capture-batch.json) verifies the bounded native capture/persistence/cohort contracts below; complete vendor parity remains `NOT_PROVEN`. The exact B8 source667bc97 passes [four CI jobs](verification/content-capture-ci.json) and an [immutable installed release](verification/content-capture-installed.json); subsequent B9 changes require their own proofs.

## Primary documented behavior

The vendor documents a configurable body-content area with default nav/footer exclusion, used for content checks rather than link discovery. It also documents later content-area and duplicate-threshold refinement without recrawling. [Official configuration: Content area](https://www.screamingfrog.co.uk/seo-spider/user-guide/configuration/#content-area), [duplicate-content tutorial](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-check-for-duplicate-content/).

Its readability tutorial describes English analysis of the configured body-content area. Raw response and executed DOM are distinct observations in the JavaScript tutorial. These requirements do not specify our Unicode tokenization, syllable heuristic, selector precedence or whitespace bytes. [Readability](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-test-readability/), [JavaScript crawling](https://www.screamingfrog.co.uk/seo-spider/tutorials/crawl-javascript-seo/).

## Existing owned-fixture observations

The earlier normal installed 24.3 CLI run observed 22 independently authored localhost HTML cases under the existing unlicensed profile. No runner configuration/licence changes or GUI control occurred. Only the ordinary analytics timestamp changed in the private profile snapshot. Full normalized inputs, exact supplied HTML hashes, output measurements, source CSV hashes and limitations are retained in [reference-content-observations.json](reference-content-observations.json); [reference-content-analysis.md](reference-content-analysis.md) explains that run. The present review reused those records and made no additional original CLI request.

| Own fixture paths | Observed original word count | Bounded implication |
| --- | ---: | --- |
| `/identical-a`, `/identical-b` | 210 each | Equal independently supplied HTML has equal reported content digest; this does not identify every token rule. |
| `/hidden` | 390 | Added `hidden`/inline-hidden source text affects the native text-only count in this case; computed browser visibility is not established. |
| `/script` | 210 | Added script/style text does not change the baseline count for this case. |
| `/boilerplate-a`, `/boilerplate-b` | 210 each | Different nav/footer boilerplate leaves the same baseline count in these cases. |
| `/punctuation-whitespace` | 5 | The recorded string has this result; no universal whitespace/token rule is inferred. |
| `/tiny` | 6 | Available short content is not empty; its reported score100 is an original observation, not our own heuristic acceptance. |
| `/empty` | 0 | No readability/sentence value is available for this row; null is not zero score. |

The original near-duplicate checker was disabled. Its empty near export provides no positive similarity, content-preview or configurable-selector proof. Equal reported word counts do not demonstrate identical selected text, DOM visibility or exact score formulas. The MD5 equality observed for all 22 exact supplied HTML strings is a separate HTML-representation observation, not a selected-text SHA contract. No licensed feature has been enabled for this comparison.

## Independent B8 profile

The authored module is `content_capture.rs`; its precise public API, resource limits and status/hash meanings are in [content-capture.md](content-capture.md). The engine profile is `open-seo-spider/html-content-capture-v1`. Default configuration selects `raw_http_html`, includes body, and excludes script/style/template/noscript/nav/footer. The integrating crawler must keep optional capture absent by default. A configured `rendered_dom_html` result is a separately supplied DOM snapshot; missing/failed rendering cannot borrow raw text or raw language.

The independent selector rule is ancestor membership in parsed document order: nested/duplicate include matches capture each text node once, an excluded ancestor wins, and template fragment traversal inherits the template state. Each nonempty Unicode-whitespace run and each nonempty node boundary is joined with an ASCII space; this intentionally makes `foo<em>bar</em>baz` into `foo bar baz`. Attributes/image alt, CSS-generated text and computed visibility are not inferred. The standard512KiB emitted cap is scalar-safe, with full observed-prefix/traversal and source/decode uncertainty separately stated. These are our specified behaviors, not observations of the vendor's implementation.

The selected-text SHA binds exact emitted UTF-8 text; `html_utf8_sha256` binds the supplied UTF-8 HTML representation, not untranslated HTTP wire bytes. Each source obtains `html_language` only from its own root HTML `lang` under the bounded declared-language syntax profile. Capture completeness means complete available observation under its source/selection profile; it does not itself imply200HTTP, indexability or website-wide coverage. Corpus eligibility, requested/final identity, response status, capture revision and retained/refreshed/pending basis require explicit integrating provenance.

Twenty meaningful pure unit tests, four binding units and sixteen native wrapper fixtures passed in the [B8 aggregate](verification/content-capture-batch.json), with a separate actual Chromium capture test. The [native API/CLI gate](verification/content-capture-e2e.json) records eight directed groups and the [actual Content UI](../web/verification/ui-content-live-results.json) records twelve checks. These proofs remain distinct from the ten authored selection fixtures and the earlier supplied-text content-analysis tests. Full-site analysis and changed CSS selection from saved full HTML/DOM remain outside these gates.

## Offline refinements versus new selection

With exact saved selected text, changing n-gram case/stopwords, Jaccard threshold or supported readability options can be recomputed without fetching a website. The emitted text and its SHA identify the saved sample; missing/partial text stays unknown or incomplete under the chosen cohort.

A new CSS include/exclude choice cannot be correctly applied to selected text or an HTML hash. It needs retained source HTML/DOM under an explicit artifact/retention budget, or a new capture. B8 selected-text persistence alone does not implement the documented post-crawl content-area workflow. The UI/API must expose this limit and never label an old selected-text sample as freshly recaptured under new selectors. Shadow DOM/iframe flattening, PDF text, visible-text browser evaluation and image-alt selection also need their own source/profile contracts.

## Native acceptance still required

1. Disabled capture creates no payload/placeholder complete flag and preserves legacy config/snapshot bytes.
2. Invalid CSS/source/positive-limit settings fail before any target request.
3. Raw and actual JS-changed DOM retain distinct texts, root languages, effective document URLs/statuses, source revisions and actual browser provenance; unavailable renderer has no raw fallback.
4. Excluded/nested/template/hidden/attribute fixtures produce exact bytes without changing link discovery; parser recovery remains explicitly its own profile.
5. Complete empty, absent/non-HTML/blocked final body, lossy/unknown decoding, native/worker/body caps and scalar text limits remain distinct and feed nullable downstream results.
6. Page, capture, admitted-task frontier and policy occurrence commit in the same generation or roll back together; pause/crash recovery cannot mix old text with new metadata.
7. Selected recrawl keeps exact parent captures, omits pending selected historical samples from the fresh corpus, and binds each new capture to its committed attempt while retaining unrelated observations unchanged.
8. Corpus requests choose exact source revisions and an explicit eligibility/freshness policy. Duplicate requested identities and source/final aliases do not silently collapse distinct captures. Missing and excluded records remain accounted for.
9. Saved-text analysis, repeated parameter changes and rejected imports make zero website requests. Corpus/page/output/storage bounds produce explicit failure/coverage rather than a silent complete-site subset.
10. API/CLI/JSON/CSV/UI agree on hash/profile/status/provenance, client imports remain declarative rather than attested capture, and hostile text stays inert/formula-safe.

These gates are an independent implementation contract. Complete feature/filter/workflow reference parity remains `NOT_PROVEN`.
