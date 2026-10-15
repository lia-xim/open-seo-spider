use super::*;
fn profile(channel: NgramHtmlTextChannel) -> NgramHtmlTextProfile {
    NgramHtmlTextProfile {
        version: 1,
        channel,
    }
}
fn doc<'a>(url: &'a str, elements: Option<Vec<&'a str>>) -> NgramChannelDocument<'a> {
    NgramChannelDocument {
        source: NgramCorpusDocument {
            requested_url: url,
            source_revision: Some("original-capture-revision"),
            capture_config_sha256: None,
            text_sha256: None,
            text: None,
            complete: elements.is_some(),
            basis: "declared_pure_element_fixture",
            observed_generation: Some(7),
        },
        source_kind: ContentCaptureSourceKind::RawHttpHtml,
        source_document_url: Some(url),
        capture_html_sha256: Some(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
        capture_status: None,
        metadata_observation: None,
        unavailable_reason: elements.is_none().then_some("missing_legacy_metadata"),
        elements,
    }
}
fn metadata_doc<'a>(url: &'a str, elements: Option<Vec<&'a str>>) -> NgramChannelDocument<'a> {
    let mut d = doc(url, elements);
    d.source.source_revision = None;
    d.capture_html_sha256 = None;
    if d.source.complete {
        d.metadata_observation = Some(NgramRetainedMetadataIdentity {
            fetched_at: "2026-10-15T00:00:00Z",
            rendered_at: None,
            engine: "independent_html5_dom_v1",
            source: "raw_html",
        });
    }
    d
}
#[test]
fn saved_element_ngrams_never_cross_elements_and_count_one_document_frequency_per_page() {
    let r = aggregate_saved_channel_ngrams(
        &[
            doc("a", Some(vec!["Red red", "red red", "", "blue"])),
            doc("b", Some(vec!["red red"])),
        ],
        &Default::default(),
        profile(NgramHtmlTextChannel::H2),
    )
    .unwrap();
    let red = r.rows.iter().find(|r| r.tokens == ["red"]).unwrap();
    assert_eq!(
        (red.occurrences, red.document_frequency, r.memberships),
        (6, 2, 5)
    );
    let pair = r.rows.iter().find(|r| r.tokens == ["red", "red"]).unwrap();
    assert_eq!((pair.occurrences, pair.document_frequency), (3, 2));
    assert_eq!(pair.sources[0].first_positions, [1, 3]);
    assert!(
        !r.rows
            .iter()
            .any(|r| r.tokens == ["red", "red", "red"] || r.tokens == ["red", "blue"])
    );
    let spans = r.sources[0]
        .channel
        .as_ref()
        .unwrap()
        .elements
        .as_ref()
        .unwrap();
    assert_eq!(spans.len(), 4);
    assert_eq!(
        (
            spans[2].retained_element_ordinal,
            spans[2].tokens,
            spans[2].token_start
        ),
        (3, 0, 5)
    );
    assert_eq!(r.candidate_windows, [7, 3, 0]);
    assert_eq!(r.total_text_bytes, 25);
    assert_eq!(
        r.sources[0].source_revision.as_deref(),
        Some("original-capture-revision")
    );
}
#[test]
fn saved_element_ngrams_extended_windows_and_stopwords_do_not_bridge_element_or_exclusion_boundaries()
 {
    let options = NgramCorpusOptions {
        max_n: 6,
        stopwords: vec!["und".into()],
        ..Default::default()
    };
    let r = aggregate_saved_channel_ngrams(
        &[doc(
            "a",
            Some(vec!["A B C", "D E F", "A B C D E F", "A B und C D E F"]),
        )],
        &options,
        profile(NgramHtmlTextChannel::H6),
    )
    .unwrap();
    let six = r
        .rows
        .iter()
        .find(|r| r.tokens == ["a", "b", "c", "d", "e", "f"])
        .unwrap();
    assert_eq!(six.occurrences, 1);
    assert_eq!(six.sources[0].first_positions, [7]);
    assert_eq!(r.extended_windows[2].candidate_windows, 3);
    assert_eq!(r.extended_windows[2].excluded_stopword_windows, 2);
    assert_eq!(r.extended_windows[2].included_windows, 1);
    assert_eq!(
        r.rows
            .iter()
            .find(|r| r.tokens == ["c", "d"])
            .unwrap()
            .occurrences,
        2
    );
}
#[test]
fn saved_element_ngrams_observed_empty_set_empty_element_and_legacy_unknown_are_distinct() {
    let r = aggregate_saved_channel_ngrams(
        &[
            doc("none", None),
            doc("empty-set", Some(vec![])),
            doc("empty-element", Some(vec![""])),
        ],
        &Default::default(),
        profile(NgramHtmlTextChannel::Title),
    )
    .unwrap();
    assert!(r.rows.is_empty());
    assert!(!r.sources[0].counted);
    assert_eq!(r.source_states["complete_empty"], 2);
    assert!(!r.all_supplied_sources_complete);
    assert!(r.sources[0].channel.as_ref().unwrap().elements.is_none());
    assert!(
        r.sources[1]
            .channel
            .as_ref()
            .unwrap()
            .elements
            .as_ref()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        r.sources[2]
            .channel
            .as_ref()
            .unwrap()
            .elements
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    assert_ne!(r.sources[1].text_sha256, r.sources[2].text_sha256);
}
#[test]
fn saved_element_ngrams_version_source_channel_and_element_order_have_distinct_revisions() {
    let docs = [doc("a", Some(vec!["Äpfel", "Bäume"]))];
    let options = Default::default();
    let a = aggregate_saved_channel_ngrams(&docs, &options, profile(NgramHtmlTextChannel::Title))
        .unwrap();
    let b = aggregate_saved_channel_ngrams(
        &docs,
        &options,
        profile(NgramHtmlTextChannel::MetaDescription),
    )
    .unwrap();
    let c = aggregate_saved_channel_ngrams(
        &[doc("a", Some(vec!["Bäume", "Äpfel"]))],
        &options,
        profile(NgramHtmlTextChannel::Title),
    )
    .unwrap();
    assert_ne!(
        a.sources[0].channel.as_ref().unwrap().channel_revision,
        b.sources[0].channel.as_ref().unwrap().channel_revision
    );
    assert_ne!(a.sources[0].text_sha256, c.sources[0].text_sha256);
    let mut dom = doc("b", Some(vec!["Äpfel"]));
    dom.source_kind = ContentCaptureSourceKind::RenderedDomHtml;
    assert!(
        aggregate_saved_channel_ngrams(
            &[doc("a", Some(vec!["Äpfel"])), dom],
            &options,
            profile(NgramHtmlTextChannel::Title)
        )
        .is_err()
    );
    for invalid in [
        r#"{"version":2,"channel":"title"}"#,
        r#"{"channel":"h1"}"#,
        r#"{"version":1,"channel":"body"}"#,
        r#"{"version":1,"channel":"title","fallback":true}"#,
        r#"[1,"title"]"#,
    ] {
        assert!(serde_json::from_str::<NgramHtmlTextProfile>(invalid).is_err());
    }
}
#[test]
fn saved_element_ngrams_text_token_identity_and_element_caps_fail_before_publishing_prefixes() {
    let docs = [doc("a", Some(vec!["one two", "three four"]))];
    for field in 0..5 {
        let mut o = NgramCorpusOptions::default();
        match field {
            0 => o.limits.max_text_bytes = 16,
            1 => o.limits.max_total_tokens = 3,
            2 => o.limits.max_unique_ngrams = 1,
            3 => o.limits.max_memberships = 1,
            _ => o.limits.max_key_bytes = 1,
        };
        assert!(
            aggregate_saved_channel_ngrams(&docs, &o, profile(NgramHtmlTextChannel::H1)).is_err()
        );
    }
    let too_many = vec![""; MAX_NGRAM_ELEMENTS_PER_PAGE + 1];
    assert!(
        aggregate_saved_channel_ngrams(
            &[doc("a", Some(too_many))],
            &Default::default(),
            profile(NgramHtmlTextChannel::H1)
        )
        .is_err()
    );
    assert!(
        aggregate_saved_channel_ngrams(
            &[doc("a", Some(vec!["one"])), doc("a", Some(vec!["two"]))],
            &Default::default(),
            profile(NgramHtmlTextChannel::H1)
        )
        .is_err()
    );
    let mut invalid = doc("a", Some(vec!["one"]));
    invalid.capture_html_sha256 = None;
    assert!(
        aggregate_saved_channel_ngrams(
            &[invalid],
            &Default::default(),
            profile(NgramHtmlTextChannel::H1)
        )
        .is_err()
    );
    let mut unknown = doc("a", None);
    unknown.elements = Some(vec!["untrusted prefix"]);
    assert!(
        aggregate_saved_channel_ngrams(
            &[unknown],
            &Default::default(),
            profile(NgramHtmlTextChannel::H1)
        )
        .is_err()
    );
}
#[test]
fn saved_body_corpus_json_keeps_exact_legacy_shape_without_channel_fields() {
    let docs = [NgramCorpusDocument {
        requested_url: "a",
        source_revision: None,
        capture_config_sha256: None,
        text_sha256: None,
        text: Some("red red"),
        complete: true,
        basis: "declared_compatibility",
        observed_generation: None,
    }];
    let r = aggregate_saved_ngrams(&docs, &Default::default()).unwrap();
    let json = serde_json::to_value(&r).unwrap();
    assert!(json.get("channel_profile").is_none());
    assert!(json["sources"][0].get("channel").is_none());
    assert_eq!(
        serde_json::to_string(&r.sources[0]).unwrap(),
        r#"{"ordinal":0,"requested_url":"a","source_revision":null,"capture_config_sha256":null,"text_sha256":null,"basis":"declared_compatibility","observed_generation":null,"state":"complete","counted":true,"text_bytes":7,"tokens":2}"#
    );
    assert_eq!(r.engine, NGRAM_CORPUS_ENGINE);
    assert_eq!(r.rows[0].tokens, ["red"]);
    assert_eq!(r.rows[0].occurrences, 2);
}
#[test]
fn saved_element_ngrams_utf8_exact_budget_and_position_sampling_keep_complete_counts() {
    let docs = [doc("a", Some(vec!["ä ä", "ä"]))];
    let mut options = NgramCorpusOptions::default();
    options.limits.position_samples = 1;
    options.limits.max_text_bytes = 6;
    assert!(
        aggregate_saved_channel_ngrams(&docs, &options, profile(NgramHtmlTextChannel::H1)).is_err()
    );
    for cap in [7, 8] {
        options.limits.max_text_bytes = cap;
        let r = aggregate_saved_channel_ngrams(&docs, &options, profile(NgramHtmlTextChannel::H1))
            .unwrap();
        assert_eq!(r.total_text_bytes, 7);
        assert_eq!(r.total_tokens, 3);
        let word = r.rows.iter().find(|r| r.tokens == ["ä"]).unwrap();
        assert_eq!((word.occurrences, word.document_frequency), (3, 1));
        assert_eq!(word.sources[0].first_positions, [1]);
        assert!(word.sources[0].positions_sampled);
        assert_eq!(
            r.rows
                .iter()
                .find(|r| r.tokens == ["ä", "ä"])
                .unwrap()
                .occurrences,
            1
        );
        let spans = r.sources[0]
            .channel
            .as_ref()
            .unwrap()
            .elements
            .as_ref()
            .unwrap();
        assert_eq!((spans[1].token_start, spans[1].tokens), (3, 1));
    }
}
#[test]
fn saved_retained_metadata_counts_without_capture_and_preserves_element_and_unknown_identities() {
    let p = NgramRetainedMetadataProfile {
        version: 1,
        channel: NgramHtmlTextChannel::Title,
    };
    let r = aggregate_saved_metadata_ngrams(
        &[
            metadata_doc("a", Some(vec!["red", "red", "blue"])),
            metadata_doc("empty", Some(vec![])),
            metadata_doc("legacy", None),
        ],
        &Default::default(),
        p,
    )
    .unwrap();
    assert_eq!(r.engine, NGRAM_RETAINED_METADATA_ENGINE);
    assert!(r.channel_profile.is_none());
    assert_eq!(r.retained_metadata_profile, Some(p));
    let red = r.rows.iter().find(|r| r.tokens == ["red"]).unwrap();
    assert_eq!((red.occurrences, red.document_frequency), (2, 1));
    assert!(!r.rows.iter().any(|r| r.n > 1));
    assert_eq!(r.sources[1].state, "complete_empty");
    assert!(!r.sources[2].counted);
    let c = r.sources[0].channel.as_ref().unwrap();
    assert!(c.profile.is_none());
    assert_eq!(c.retained_metadata_profile, Some(p));
    assert!(c.capture_html_sha256.is_none());
    assert!(r.sources[0].source_revision.is_none());
    assert!(c.metadata_source.as_ref().unwrap().revision.len() == 64);
    assert_eq!(
        c.metadata_source.as_ref().unwrap().authority_profile,
        "retained_html_metadata_v1"
    );
    let mut invalid = metadata_doc("a", Some(vec!["red"]));
    invalid.metadata_observation = None;
    assert!(aggregate_saved_metadata_ngrams(&[invalid], &Default::default(), p).is_err());
}
#[test]
fn saved_retained_metadata_revision_separates_capture_attachment_and_binds_observation_context() {
    let p = NgramRetainedMetadataProfile {
        version: 1,
        channel: NgramHtmlTextChannel::H1,
    };
    let options = Default::default();
    let base =
        aggregate_saved_metadata_ngrams(&[metadata_doc("a", Some(vec!["same text"]))], &options, p)
            .unwrap();
    let c = base.sources[0].channel.as_ref().unwrap();
    let original = c.metadata_source.as_ref().unwrap().revision.clone();
    let mut attached = metadata_doc("a", Some(vec!["same text"]));
    attached.source.source_revision = Some("original-optional-capture");
    attached.capture_html_sha256 =
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    attached.capture_status = Some(crate::PageContentCaptureStatus::Incomplete);
    let r = aggregate_saved_metadata_ngrams(&[attached], &options, p).unwrap();
    let next = r.sources[0].channel.as_ref().unwrap();
    assert_eq!(next.metadata_source.as_ref().unwrap().revision, original);
    assert_ne!(next.channel_revision, c.channel_revision);
    assert_eq!(
        next.capture_html_sha256.as_deref(),
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
    );
    assert_eq!(
        next.capture_status,
        Some(crate::PageContentCaptureStatus::Incomplete)
    );
    for mode in 0..4 {
        let mut d = metadata_doc("a", Some(vec!["same text"]));
        match mode {
            0 => d.metadata_observation.as_mut().unwrap().fetched_at = "2026-10-16T00:00:00Z",
            1 => d.source_document_url = Some("different-document"),
            2 => d.source.observed_generation = Some(8),
            _ => d.elements = Some(vec!["same", "text"]),
        };
        let changed = aggregate_saved_metadata_ngrams(&[d], &options, p).unwrap();
        assert_ne!(
            changed.sources[0]
                .channel
                .as_ref()
                .unwrap()
                .metadata_source
                .as_ref()
                .unwrap()
                .revision,
            original
        );
    }
    let mut wrong = metadata_doc("a", Some(vec!["same text"]));
    wrong.metadata_observation.as_mut().unwrap().source = "rendered_html";
    assert!(aggregate_saved_metadata_ngrams(&[wrong], &options, p).is_err());
    assert!(
        serde_json::from_str::<NgramRetainedMetadataProfile>(r#"{"version":2,"channel":"title"}"#)
            .is_err()
    );
}
