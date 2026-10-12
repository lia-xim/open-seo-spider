# Reproducible analysis of supplied text

`crates/core/src/content_analysis.rs` provides the pure public function `analyze_content(&ContentAnalysisInput) -> anyhow::Result<ContentAnalysisResult>`. It performs no HTTP, provider access or file reads. **This module analyzes explicitly supplied text; crawl-body collection and app/UI integration are separate gates.** Current `PageRecord` has word counts, content hashes, metadata, extraction values and bounded PDF previews, but no retained complete page-body text. None of those are silently promoted to full text. No Screaming Frog implementation or parity claim is involved.

The engine is `open-seo-spider/content-analysis-v1`. Public input/config structs reject unknown JSON fields. Input requires corpus identity/scope/completeness and ordered document ids, caller-declared sources, nullable text/language and explicit text completeness. Missing text cannot be declared complete; a complete row cannot claim a truncation reason. These declarations are provenance supplied by the caller, not attested capture authenticity or website-wide coverage.

```json
{
  "corpus_id": "manual-example",
  "corpus_scope": "two_explicit_manual_texts",
  "corpus_complete": true,
  "config": {
    "case_behavior": "unicode_lowercase",
    "stopwords": [],
    "similarity": {"shingle_size": 3, "threshold": 0.9, "min_tokens": 10},
    "readability": {"min_words": 30}
  },
  "documents": [
    {"id":"a","source":"manual_text","text":"The cat sits on the mat.","language":"en","complete":true},
    {"id":"b","source":"manual_text","text":null,"language":null,"complete":false,"truncation_reason":"body_not_retained"}
  ]
}
```

This short example deliberately produces unknown similarity/readability under the default minimum-word/token settings. It is a schema illustration, not a test execution artifact.

## Token and N-gram profile

`unicode-alphanumeric-mark-runs-no-normalization-v1` starts tokens at Rust Unicode alphabetic/numeric scalars and continues them through those scalars or Unicode General_Category Mark scalars. Marks are accepted after a token start. Other characters separate tokens. Punctuation, apostrophes and hyphens therefore break tokens; emoji do not become words. This is a versioned character-run profile, not UAX word-boundary segmentation or dictionary segmentation of Chinese/Japanese/Thai.

`case_behavior` is `unicode_lowercase` by default or explicit `preserve`. Lowercase uses Rust's Unicode lowercase mapping; the result records `char::UNICODE_VERSION`. Mark categories come from the regex dependency's Unicode tables, controlled by the build's locked dependency version. Reproducibility requires the same engine/profile and dependency/compiler Unicode tables. Rust documents that Unicode methods can change with its Unicode tables. [Primary Rust character documentation](https://doc.rust-lang.org/std/primitive.char.html), [primary regex Unicode documentation](https://docs.rs/regex/1.13.1/regex/#unicode).

There is no NFC/NFKC normalization, stemming or full case folding: composed `café` and decomposed `cafe` plus combining accent remain distinct; `Straße` and `STRASSE` do not collapse into the same lowercase token. The normalized token-stream hash uses length-prefixed UTF-8 tokens, so token-boundary concatenation is unambiguous. Raw text has its own SHA-256.

All overlapping 1-, 2- and 3-token windows are counted. Windows run over the full supplied token stream, including across sentence punctuation. Output rows are deterministically sorted by their token vectors and retain exact occurrence counts with bounded one-based position samples. `positions_sampled` explicitly indicates omitted position examples; it never means the occurrence count was truncated.

Stopwords are an explicit custom list, without built-in language lists. Each must be exactly one unpadded token; duplicates after configured case handling are rejected. A window containing a stopword is excluded. Words on either side are not bridged into a previously absent adjacency. For each N the result reports candidate windows, stopword-excluded windows and included windows. Readability always uses the original token stream and is unaffected by stopword/case settings.

## Exact near-duplicate observations

`exact-token-shingle-set-jaccard-v1` compares unique token-shingle sets, with configurable size 1–3, minimum token count and inclusive threshold. Its observed score is:

```text
intersection size / union size
near_duplicate = observed score >= threshold
```

The set-resemblance definition follows the published shingling approach. This implementation uses exact token vectors and set intersection, without fingerprints, sampling or MinHash approximation. [Broder's primary paper, author-uploaded copy](https://www.researchgate.net/publication/262333747_On_the_Resemblance_and_Containment_of_Documents).

Every unordered pair of supplied rows is emitted in input order, including pairs whose score is unknown. A scored pair retains both ids, set sizes, nullable intersection/union, threshold and bounded lexicographically ordered shared-shingle examples. The shared-sample truncation flag describes only examples. Frequency does not affect this set-based score. `identical_supplied_text` separately compares exact supplied strings; a score of 1 does not establish byte-identical text, semantic equivalence or plagiarism.

Missing text, too few tokens and empty post-filter features produce nullable score/intersection/union/classification and a reason. Incomplete supplied samples can receive an observed sample score with its actual counts, while `near_duplicate` stays unknown. Complete pairs remain meaningful within a partial corpus; they do not establish absence of other duplicates outside it. Corpus evidence records declared scope/completeness, missing/incomplete rows, possible/emitted/scored pairs and complete processing of the supplied inputs.

## Explicitly limited readability

`declared-en-de-vowel-groups-terminal-e-v1` accepts only declared `en`/`de` or their syntactically bounded hyphenated subtags. It does not detect language. A supported declaration does not prove the text is actually in that language. English words must use ASCII letters; German additionally accepts `äöüß`. Any unsupported token, including numbers, other scripts or decomposed accents, makes the readability score unknown and retains unsupported counts/examples. Missing/empty/incomplete samples and samples below the configured word minimum also stay unknown.

Sentence counting closes a pending word-containing span at `.`, `!` or `?`; repeated terminal punctuation adds no empty sentence. A trailing unterminated word-containing span counts as one sentence. Abbreviations, quotation syntax and decimal punctuation have no linguistic parser. Tokenized contractions and hyphenated compounds may count as several words.

Syllables are estimated by contiguous vowel groups. English uses `aeiouy`, subtracting one terminal silent `e` when more than one group exists except consonant-plus-`le`; the result is at least one. German uses `aeiouyäöü` without the English silent-e subtraction. This is an own heuristic: exceptions, loan words, hiatus, acronym pronunciation and compound boundaries are not resolved. For example, adjacent vowels in German `bauen` are undercounted by this profile. It is not a phonetic dictionary or a replication of another tool's syllable counter.

The unrounded, unclamped outputs use:

- English: `206.835 − 1.015 × words/sentences − 84.6 × syllables/words`. Formula and programmable heuristic context: [primary 1986 research paper](https://doi.org/10.1145/10563.10583); original publication identity: [Flesch 1948](https://pubmed.ncbi.nlm.nih.gov/18867058/).
- German Amstad adaptation: `180 − words/sentences − 58.5 × syllables/words`. Formula used in [primary DELITE 2024 readability research, section 4.1](https://aclanthology.org/2024.delite-1.pdf#page=30).

Scores may be below 0 or above 100. Counts, formula, declared language, heuristic profile, nullable score and unknown reason are reported together. No school-grade, comprehension, accessibility-conformance or search-ranking prediction follows from this number. The default minimum is 30 words as an explicit own sample-size guard, not a claim of a universal scientific cutoff.

## Bounds and integration

All limits are positive and may be lowered from hard ceilings: 128 documents; 8 MiB actual serialized input; 512 KiB per text; 50,000 tokens per document; 4,096 bytes per token before/after case handling; 100,000 unique N-gram rows per document across N=1/2/3; 16 MiB cumulative N-gram key bytes; 8,192 pairs; 20 million charged pair features; 16 position examples and 16 shared-shingle examples; 16 MiB actual serialized output. At most 1,024 custom stopwords are accepted. Pair work charges the sum of both unique-set sizes for each scored pair; it is a reproducible work budget, not an exact measurement of CPU comparisons or process RSS.

Overflow and exhausted budgets return an error, with no silently partial result or fabricated score. Counts/unique rows are bounded before insertion. Serialization uses a bounded writer that accounts for escaping and metadata. The all-pairs algorithm is intentionally bounded; million-document analysis is not claimed. Raw input/hash data, generated keys and results remain memory-only; downstream storage/retention needs a separately reviewed integration.

The 20 in-module tests cover overlapping positions, Unicode/case distinctions, leading marks and joiners, expanding lowercase byte bounds for both document tokens and stopwords, stopword adjacency, identical and controlled-changed paragraphs, all three shingle sizes, inclusive zero/one thresholds, byte identity versus set equality, missing/empty/short/filtered features, incomplete samples and partial corpora, supported/unsupported readability, explicitly negative scores, deterministic output/roundtrip, malformed config/shapes, actual serialized escaping/resource bounds/overflow, inclusive pair-work limits and explicit example sampling. **The coordinated core gate `cargo test --locked -p open-seo-spider-core --lib content_analysis::tests` passed all 20 tests on 2026-10-12; core all-targets check also passed, without source fixes.** API/CLI/UI and final workspace/Clippy gates remain separate. No catalog or vendor-parity upgrade follows automatically from a focused core test.

The separate [bounded original content observations](reference-content-analysis.md) preserve a controlled normal CLI run and its known differences/prerequisites. These observations do not redefine the independent tokenization, unrounded formulas or exact Jaccard profile documented here.
