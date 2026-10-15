use super::*;
fn doc<'a>(url: &'a str, text: Option<&'a str>, complete: bool) -> NgramCorpusDocument<'a> {
    NgramCorpusDocument {
        requested_url: url,
        source_revision: Some("declared-revision"),
        capture_config_sha256: None,
        text_sha256: None,
        text,
        complete,
        basis: "declared_core_test",
        observed_generation: None,
    }
}
#[test]
fn ngram_corpus_aggregates_more_than128_sources_exact_frequency_memberships_and_overlapping_positions()
 {
    let urls = (0..205)
        .map(|n| format!("https://example.test/{n}"))
        .collect::<Vec<_>>();
    let docs = urls
        .iter()
        .map(|u| doc(u, Some("Red red red"), true))
        .collect::<Vec<_>>();
    let r = aggregate_saved_ngrams(&docs, &Default::default()).unwrap();
    let row = r.rows.iter().find(|r| r.tokens == ["red", "red"]).unwrap();
    assert_eq!(row.occurrences, 410);
    assert_eq!(row.document_frequency, 205);
    assert_eq!(row.sources.len(), 205);
    assert_eq!(row.sources[204].source_ordinal, 204);
    assert_eq!(row.sources[0].first_positions, vec![1, 2]);
    assert_eq!(r.included_windows, [615, 410, 205]);
    assert_eq!(r.sources.len(), 205);
    assert!(!r.whole_website_coverage_attested);
}
#[test]
fn ngram_corpus_existing_unicode_stopword_profile_never_bridges_sources_or_removed_words() {
    let options = NgramCorpusOptions {
        stopwords: vec!["und".into()],
        ..Default::default()
    };
    let r = aggregate_saved_ngrams(
        &[
            doc("a", Some("Äpfel und Bäume"), true),
            doc("b", Some("ÄPFEL Bäume"), true),
        ],
        &options,
    )
    .unwrap();
    assert!(
        r.rows
            .iter()
            .any(|r| r.tokens == ["äpfel", "bäume"] && r.occurrences == 1)
    );
    assert!(!r.rows.iter().any(|r| r.tokens == ["bäume", "äpfel"]));
    assert_eq!(r.excluded_stopword_windows, [1, 2, 1]);
    let preserve = NgramCorpusOptions {
        case_behavior: ContentCaseBehavior::Preserve,
        ..Default::default()
    };
    let r = aggregate_saved_ngrams(&[doc("a", Some("A a"), true)], &preserve).unwrap();
    assert_eq!(r.rows.iter().filter(|r| r.n == 1).count(), 2);
}
#[test]
fn ngram_corpus_incomplete_missing_empty_duplicate_identity_and_tiny_budgets_are_explicit() {
    let docs = [
        doc("missing", None, false),
        doc("partial", Some("must not count"), false),
        doc("empty", Some(""), true),
    ];
    let r = aggregate_saved_ngrams(&docs, &Default::default()).unwrap();
    assert!(r.rows.is_empty());
    assert_eq!(r.source_states["missing_or_unavailable"], 1);
    assert_eq!(r.source_states["incomplete_excluded"], 1);
    assert_eq!(r.source_states["complete_empty"], 1);
    assert!(!r.all_supplied_sources_complete);
    assert!(!r.sources[1].counted);
    assert!(
        aggregate_saved_ngrams(
            &[
                doc("same", Some("one"), true),
                doc("same", Some("two"), true)
            ],
            &Default::default()
        )
        .is_err()
    );
    for field in 0..4 {
        let mut options = NgramCorpusOptions::default();
        match field {
            0 => options.limits.max_text_bytes = 1,
            1 => options.limits.max_unique_ngrams = 1,
            2 => options.limits.max_memberships = 1,
            _ => options.limits.max_total_tokens = 1,
        };
        assert!(
            aggregate_saved_ngrams(&[doc("a", Some("one two three"), true)], &options).is_err()
        );
    }
}
#[test]
fn ngram_corpus_position_sample_flag_preserves_full_count_and_actual_engine_positions() {
    let options = NgramCorpusOptions {
        limits: NgramCorpusLimits {
            position_samples: 2,
            ..Default::default()
        },
        ..Default::default()
    };
    let r = aggregate_saved_ngrams(&[doc("a", Some("x x x x x"), true)], &options).unwrap();
    let row = r.rows.iter().find(|r| r.tokens == ["x"]).unwrap();
    assert_eq!(row.occurrences, 5);
    assert_eq!(row.sources[0].first_positions, vec![1, 2]);
    assert!(row.sources[0].positions_sampled);
    assert_eq!(row.document_frequency, 1);
}

#[test]
fn ngram_corpus_extended_overlaps_exact_sources_positions_and_legacy_three_counters() {
    let docs = [
        doc("a", Some("x x x x x x x x x x"), true),
        doc("b", Some("x x x x x x"), true),
        doc("missing", None, false),
        doc("incomplete", Some("x x x x x x"), false),
    ];
    let mut options = NgramCorpusOptions {
        max_n: 6,
        ..Default::default()
    };
    options.limits.position_samples = 2;
    let report = aggregate_saved_ngrams(&docs, &options).unwrap();
    for (n, total, first) in [(4, 10, 7), (5, 8, 6), (6, 6, 5)] {
        let row = report.rows.iter().find(|r| r.n == n).unwrap();
        assert_eq!(row.occurrences, total);
        assert_eq!(row.document_frequency, 2);
        assert_eq!(row.sources.len(), 2);
        assert_eq!(row.sources[0].occurrences, first);
        assert_eq!(row.sources[0].first_positions, vec![1, 2]);
        assert!(row.sources[0].positions_sampled);
        assert_eq!(row.sources[1].occurrences, 7 - n);
        assert_eq!(row.sources[1].positions_sampled, 7 - n > 2);
        let counter = &report.extended_windows[n - 4];
        assert_eq!(counter.n, n);
        assert_eq!(counter.candidate_windows, total);
        assert_eq!(counter.excluded_stopword_windows, 0);
        assert_eq!(counter.included_windows, total);
    }
    let legacy = aggregate_saved_ngrams(
        &docs,
        &NgramCorpusOptions {
            limits: options.limits.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(legacy.extended_windows.is_empty());
    assert!(legacy.rows.iter().all(|r| r.n <= 3));
    assert_eq!(report.candidate_windows, legacy.candidate_windows);
    assert_eq!(report.included_windows, legacy.included_windows);
    assert_eq!(
        report.excluded_stopword_windows,
        legacy.excluded_stopword_windows
    );
    assert_eq!(report.total_tokens, 16);
    assert_eq!(report.total_tokens, legacy.total_tokens);
    for old in &legacy.rows {
        let current = report.rows.iter().find(|r| r.tokens == old.tokens).unwrap();
        assert_eq!(
            serde_json::to_vec(current).unwrap(),
            serde_json::to_vec(old).unwrap()
        );
    }
    assert!(!report.sources[2].counted);
    assert!(!report.sources[3].counted);
    assert_eq!(report.website_requests, 0);
    options.max_n = 4;
    let four = aggregate_saved_ngrams(&docs, &options).unwrap();
    assert_eq!(four.extended_windows.len(), 1);
    assert!(four.rows.iter().all(|r| r.n <= 4));
}

#[test]
fn ngram_corpus_extended_unicode_no_normalization_stopword_gaps_and_document_boundaries() {
    let options = NgramCorpusOptions {
        max_n: 6,
        stopwords: vec!["UND".into()],
        ..Default::default()
    };
    let report=aggregate_saved_ngrams(&[doc("a",Some("\u{301}ÄPFEL, e\u{301}. １２３! 東京? İ; end UND after one two three four five six"),true),doc("b",Some("after one two three four five six"),true),doc("c",Some("one two three"),true),doc("d",Some("four five six"),true)],&options).unwrap();
    let six = vec!["äpfel", "e\u{301}", "１２３", "東京", "i\u{307}", "end"];
    let row = report.rows.iter().find(|r| r.tokens == six).unwrap();
    assert_eq!(row.occurrences, 1);
    assert_eq!(row.sources[0].first_positions, vec![1]);
    let after = report
        .rows
        .iter()
        .find(|r| r.tokens == ["after", "one", "two", "three", "four", "five"])
        .unwrap();
    assert_eq!(after.occurrences, 2);
    assert_eq!(after.document_frequency, 2);
    assert_eq!(after.sources[0].first_positions, vec![8]);
    assert_eq!(after.sources[1].first_positions, vec![1]);
    assert!(
        report
            .rows
            .iter()
            .filter(|r| r.n >= 4)
            .all(|r| !r.tokens.iter().any(|t| t == "und"))
    );
    assert!(
        !report
            .rows
            .iter()
            .any(|r| r.tokens == ["東京", "i\u{307}", "end", "after"])
    );
    assert!(
        !report
            .rows
            .iter()
            .any(|r| r.tokens == ["six", "after", "one", "two"])
    );
    let separate = aggregate_saved_ngrams(
        &[
            doc("c", Some("one two three"), true),
            doc("d", Some("four five six"), true),
        ],
        &options,
    )
    .unwrap();
    assert!(separate.rows.iter().all(|r| r.n <= 3));
    assert!(
        separate
            .extended_windows
            .iter()
            .all(|w| w.candidate_windows == 0)
    );
    let preserve = NgramCorpusOptions {
        max_n: 6,
        case_behavior: ContentCaseBehavior::Preserve,
        ..Default::default()
    };
    let report =
        aggregate_saved_ngrams(&[doc("p", Some("A a Ä ä É e\u{301}"), true)], &preserve).unwrap();
    assert!(
        report
            .rows
            .iter()
            .any(|r| r.tokens == ["A", "a", "Ä", "ä", "É", "e\u{301}"])
    );
    let blocked =
        aggregate_saved_ngrams(&[doc("s", Some("a b c UND d e f"), true)], &options).unwrap();
    assert_eq!(
        blocked
            .extended_windows
            .iter()
            .map(|w| w.candidate_windows)
            .collect::<Vec<_>>(),
        vec![4, 3, 2]
    );
    assert!(
        blocked
            .extended_windows
            .iter()
            .all(|w| w.candidate_windows == w.excluded_stopword_windows && w.included_windows == 0)
    );
}

#[test]
fn ngram_corpus_extended_exact_aggregate_budgets_refuse_without_truncating_or_changing_default() {
    let docs = [doc("a", Some("a b c d e f"), true)];
    let mut options = NgramCorpusOptions {
        max_n: 6,
        ..Default::default()
    };
    options.limits.max_unique_ngrams = 21;
    options.limits.max_memberships = 21;
    options.limits.max_key_bytes = 56;
    options.limits.max_total_tokens = 6;
    options.limits.max_text_bytes = 11;
    let report = aggregate_saved_ngrams(&docs, &options).unwrap();
    assert_eq!(report.rows.len(), 21);
    assert_eq!(report.memberships, 21);
    assert_eq!(
        report
            .rows
            .iter()
            .map(|r| r.tokens.iter().map(String::len).sum::<usize>())
            .sum::<usize>(),
        56
    );
    for field in 0..5 {
        let mut short = options.clone();
        match field {
            0 => short.limits.max_unique_ngrams = 20,
            1 => short.limits.max_memberships = 20,
            2 => short.limits.max_key_bytes = 55,
            3 => short.limits.max_total_tokens = 5,
            _ => short.limits.max_text_bytes = 10,
        };
        assert!(aggregate_saved_ngrams(&docs, &short).is_err());
    }
    options.max_n = 3;
    options.limits.max_unique_ngrams = 15;
    options.limits.max_memberships = 15;
    options.limits.max_key_bytes = 28;
    let legacy = aggregate_saved_ngrams(&docs, &options).unwrap();
    assert_eq!(legacy.rows.len(), 15);
    assert_eq!(legacy.memberships, 15);
    assert!(legacy.extended_windows.is_empty());
    for invalid in [0, 1, 2, 7, usize::MAX] {
        options.max_n = invalid;
        assert!(aggregate_saved_ngrams(&[], &options).is_err());
    }
    let mut invalid = NgramCorpusOptions {
        max_n: 6,
        ..Default::default()
    };
    invalid.stopwords = vec!["A".into(), "a".into()];
    assert!(aggregate_saved_ngrams(&[], &invalid).is_err());
    invalid.stopwords = vec![" two ".into()];
    assert!(aggregate_saved_ngrams(&[], &invalid).is_err());
}

#[test]
fn ngram_corpus_extended_keeps_per_document_token_byte_text_and_old_shingle_bounds() {
    let mut options = NgramCorpusOptions {
        max_n: 6,
        stopwords: vec!["x".into()],
        ..Default::default()
    };
    let exact = vec!["x"; 50_000].join(" ");
    let report = aggregate_saved_ngrams(&[doc("tokens", Some(&exact), true)], &options).unwrap();
    assert_eq!(report.total_tokens, 50_000);
    assert!(report.rows.is_empty());
    assert_eq!(report.extended_windows[2].excluded_stopword_windows, 49_995);
    let plus = format!("{exact} x");
    assert!(aggregate_saved_ngrams(&[doc("tokens+1", Some(&plus), true)], &options).is_err());
    options.stopwords.clear();
    let token = "z".repeat(4096);
    let text = format!("{token} a b c d e");
    assert!(aggregate_saved_ngrams(&[doc("token-bytes", Some(&text), true)], &options).is_ok());
    let token = "z".repeat(4097);
    assert!(aggregate_saved_ngrams(&[doc("token-bytes+1", Some(&token), true)], &options).is_err());
    let lower_expansion = "İ".repeat(1366);
    assert!(lower_expansion.len() < 4096);
    assert!(lower_expansion.to_lowercase().len() > 4096);
    assert!(
        aggregate_saved_ngrams(
            &[doc("normalized-bytes", Some(&lower_expansion), true)],
            &options
        )
        .is_err()
    );
    let text = " ".repeat(512 * 1024);
    assert!(aggregate_saved_ngrams(&[doc("text", Some(&text), true)], &options).is_ok());
    let plus = format!("{text} ");
    assert!(aggregate_saved_ngrams(&[doc("text+1", Some(&plus), true)], &options).is_err());
    let old = ContentAnalysisConfig {
        similarity: crate::ContentSimilarityOptions {
            shingle_size: 4,
            min_tokens: 4,
            ..Default::default()
        },
        ..Default::default()
    };
    let input = ContentAnalysisInput {
        corpus_id: "old-shingle".into(),
        corpus_scope: "existing engine remains1..3".into(),
        corpus_complete: true,
        config: old,
        documents: vec![ContentTextInput {
            id: "a".into(),
            source: "declared".into(),
            text: Some("a b c d e f".into()),
            language: None,
            complete: true,
            truncation_reason: None,
        }],
    };
    assert!(analyze_content(&input).is_err());
}
