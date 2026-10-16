// Simulated provider fixtures validate adapter contracts, not LanguageTool rule quality.
use super::*;
use serde_json::{Value, json};
fn config() -> Config {
    Config {
        enabled: true,
        endpoint: Some("http://127.0.0.1:8081/v2/check".into()),
        acknowledge_local_text_transfer: true,
        ..Default::default()
    }
}
fn reply(text: &str, offset: usize, length: usize) -> Value {
    json!({"software":{"name":"LanguageTool","version":"simulated-v1","buildDate":"fixture","apiVersion":1,"additive":true},
        "language":{"name":"English (US)","code":"en-US","detectedLanguage":{"name":"Unknown fixture","code":"en"}},
        "warnings":{"incompleteResults":false},"matches":[{"message":"Simulated misspelling","shortMessage":"","offset":offset,"length":length,
        "context":{"text":text,"offset":offset,"length":length},"sentence":text,
        "replacements":[{"value":"sentence"}],"rule":{"id":"SIM_SPELL","description":"Synthetic rule","issueType":"misspelling","category":{"id":"TYPOS","name":"Possible typo"}}}],"new_provider_field":{"safe":true}})
}
fn parse(v: &Value, text: &str, c: &Config, d: &Dictionary) -> Result<Checked, String> {
    parse_reply(
        &serde_json::to_vec(v).unwrap(),
        text,
        &choose_language(c, None).unwrap(),
        c,
        d,
    )
}
#[test]
fn spelling_default_off_and_endpoint_requires_explicit_local_acknowledgement() {
    let c = Config::default();
    assert!(!c.enabled);
    assert!(c.validate().is_ok());
    let mut c = config();
    c.acknowledge_local_text_transfer = false;
    assert!(c.validate().is_err());
    c = config();
    c.endpoint = Some("https://api.languagetool.org/v2/check".into());
    assert!(c.validate().is_err());
    for value in [
        "http://localhost:8081/v2/check",
        "http://127.0.0.1:0/v2/check",
        "http://127.0.0.1:8081/v2/check?x=1",
        "http://user@127.0.0.1:8081/v2/check",
        "http://192.0.2.1:8081/v2/check",
    ] {
        assert!(validate_endpoint(value).is_err(), "{value}");
    }
    assert!(validate_endpoint("http://[::1]:8081/v2/check").is_ok());
}
#[test]
fn spelling_language_manual_html_region_and_fallback_are_separate() {
    let mut c = config();
    assert_eq!(choose_language(&c, Some("de")).unwrap().requested, "de-DE");
    assert_eq!(
        choose_language(&c, Some("de")).unwrap().basis,
        "saved_html_lang"
    );
    assert_eq!(
        choose_language(&c, Some("not a language")).unwrap().basis,
        "configured_fallback"
    );
    c.manual_language = Some("pt-BR".into());
    let d = choose_language(&c, Some("de")).unwrap();
    assert_eq!(d.requested, "pt-BR");
    assert_eq!(d.basis, "manual");
    assert!(!d.automatic_detection);
    c.manual_language = Some("auto".into());
    assert!(c.validate().is_err());
}
#[test]
fn spelling_utf16_emoji_combining_and_newlines_keep_exact_locations() {
    let text = "😀 e\u{301}\r\nsentense";
    assert_eq!(utf16_slice(text, 7, 8).unwrap(), "sentense");
    assert_eq!(utf16_slice(text, 3, 2).unwrap(), "e\u{301}");
    let checked = parse(&reply(text, 7, 8), text, &config(), &Dictionary::default()).unwrap();
    assert_eq!(checked.issues[0].offset_utf16, 7);
    assert_eq!(checked.issues[0].matched_text, "sentense");
    assert_eq!(checked.accepted_count, Some(1));
}
#[test]
fn spelling_rejects_surrogate_split_overflow_and_wrong_context() {
    let text = "😀 bad";
    for (a, n) in [(1, 1), (0, 1), (99, 1), (usize::MAX, 4)] {
        assert!(utf16_slice(text, a, n).is_err());
    }
    let mut v = reply(text, 3, 3);
    v["matches"][0]["context"]["text"] = json!("😀 xyz");
    assert!(parse(&v, text, &config(), &Dictionary::default()).is_err());
}
#[test]
fn spelling_incomplete_results_cannot_be_zero_errors() {
    let mut v = reply("bad", 0, 3);
    v["warnings"]["incompleteResults"] = json!(true);
    v["matches"] = json!([]);
    let checked = parse(&v, "bad", &config(), &Dictionary::default()).unwrap();
    assert!(!checked.complete);
    assert_eq!(checked.accepted_count, None);
    v.as_object_mut().unwrap().remove("warnings");
    assert!(parse(&v, "bad", &config(), &Dictionary::default()).is_err());
}
#[test]
fn spelling_provider_identity_language_version_and_required_fields_are_checked() {
    let mut v = reply("bad", 0, 3);
    v["software"]["name"] = json!("Other");
    assert!(parse(&v, "bad", &config(), &Dictionary::default()).is_err());
    v = reply("bad", 0, 3);
    v["language"]["code"] = json!("de-DE");
    assert!(parse(&v, "bad", &config(), &Dictionary::default()).is_err());
    let mut c = config();
    c.expected_engine_version = Some("different".into());
    assert!(parse(&reply("bad", 0, 3), "bad", &c, &Dictionary::default()).is_err());
    v = reply("bad", 0, 3);
    v["matches"][0].as_object_mut().unwrap().remove("offset");
    assert!(parse(&v, "bad", &config(), &Dictionary::default()).is_err());
}
#[test]
fn spelling_additive_provider_fields_are_safe_but_own_schema_is_strict() {
    assert!(
        parse(
            &reply("bad", 0, 3),
            "bad",
            &config(),
            &Dictionary::default()
        )
        .is_ok()
    );
    let mut c = serde_json::to_value(Config::default()).unwrap();
    c["cloud_fallback"] = json!(true);
    assert!(serde_json::from_value::<Config>(c).is_err());
    assert!(serde_json::from_value::<Config>(json!({})).is_err());
    assert!(
        serde_json::from_value::<Dictionary>(json!({"version":1,"words":[],"unknown":1})).is_err()
    );
}
#[test]
fn spelling_dictionary_is_case_unicode_and_language_scoped() {
    let mut d = Dictionary::default();
    d.words.push(Word {
        language: "de".into(),
        word: "bad".into(),
    });
    let v = reply("bad", 0, 3);
    assert_eq!(
        parse(&v, "bad", &config(), &d).unwrap().accepted_count,
        Some(1)
    );
    d.words[0].language = "en".into();
    assert_eq!(
        parse(&v, "bad", &config(), &d).unwrap().issues[0].disposition,
        "dictionary"
    );
    d.words[0].word = "Bad".into();
    assert_eq!(
        parse(&v, "bad", &config(), &d).unwrap().accepted_count,
        Some(1)
    );
    d.words[0].word = "e\u{301}".into();
    assert_eq!(
        parse(&reply("é", 0, 1), "é", &config(), &d)
            .unwrap()
            .accepted_count,
        Some(1)
    );
}
#[test]
fn spelling_per_crawl_ignored_word_rule_and_switches_retain_observed_diagnostics() {
    let v = reply("bad", 0, 3);
    let mut c = config();
    c.ignored_words.push(Word {
        language: "en-US".into(),
        word: "bad".into(),
    });
    assert_eq!(
        parse(&v, "bad", &c, &Dictionary::default()).unwrap().issues[0].disposition,
        "ignored_word"
    );
    c.ignored_rules.push("SIM_SPELL".into());
    assert_eq!(
        parse(&v, "bad", &c, &Dictionary::default()).unwrap().issues[0].disposition,
        "ignored_rule"
    );
    c = config();
    c.spelling = false;
    assert_eq!(
        parse(&v, "bad", &c, &Dictionary::default()).unwrap().issues[0].disposition,
        "disabled_kind"
    );
    c = config();
    c.grammar = false;
    let mut g = v.clone();
    g["matches"][0]["rule"]["issueType"] = json!("grammar");
    assert_eq!(
        parse(&g, "bad", &c, &Dictionary::default()).unwrap().issues[0].disposition,
        "disabled_kind"
    );
}
#[test]
fn spelling_finite_text_response_issues_replacements_and_dictionary_bounds() {
    let text = "x".repeat(MAX_TEXT_BYTES + 1);
    assert!(parse(&reply("x", 0, 1), &text, &config(), &Dictionary::default()).is_err());
    let mut v = reply("x", 0, 1);
    v["matches"][0]["replacements"] = json!(vec![json!({"value":"x"}); 513]);
    assert!(parse(&v, "x", &config(), &Dictionary::default()).is_err());
    v = reply("x", 0, 1);
    v["matches"] = json!(vec![v["matches"][0].clone(); MAX_ISSUES + 1]);
    assert!(parse(&v, "x", &config(), &Dictionary::default()).is_err());
    assert!(
        parse_reply(
            &vec![b' '; MAX_RESPONSE_BYTES + 1],
            "x",
            &choose_language(&config(), None).unwrap(),
            &config(),
            &Dictionary::default()
        )
        .is_err()
    );
    let d = Dictionary {
        version: 1,
        words: vec![
            Word {
                language: "en".into(),
                word: "bad".into()
            };
            2
        ],
    };
    assert!(d.validate().is_err());
}
// Saved actual LanguageTool6.9-SNAPSHOT response from the Root-owned de-DE probe.
// These offline regressions validate the adapter; they do not run the engine or prove current rule quality.
#[test]
fn spelling_saved_actual_german_31_suggestions_preserves_all_ordered_values_and_span() {
    const RAW: &str = r####"{"software":{"name":"LanguageTool","version":"6.9-SNAPSHOT","buildDate":"2026-10-16 18:49:04 +0200","apiVersion":1,"premium":false,"premiumHint":"You might be missing errors only the Premium version can find. Contact us at support<at>languagetoolplus.com.","status":""},"warnings":{"incompleteResults":false},"language":{"name":"German (Germany)","code":"de-DE","detectedLanguage":{"name":"German (Germany)","code":"de-DE","confidence":0.9999957,"source":"+fallback"}},"matches":[{"message":"Möglicher Tippfehler gefunden.","shortMessage":"Rechtschreibfehler","replacements":[{"value":"Fehler"},{"value":"Vieler"},{"value":"Feuer"},{"value":"Täler"},{"value":"Feiern"},{"value":"Feder"},{"value":"Feier"},{"value":"Feiert"},{"value":"Felder"},{"value":"Pfeiler"},{"value":"Velar"},{"value":"Vier"},{"value":"Tälern"},{"value":"Väter"},{"value":"Feldern"},{"value":"Aller"},{"value":"Feuern"},{"value":"Feger"},{"value":"Viele"},{"value":"Feuert"},{"value":"Pfälzer"},{"value":"Pfeilern"},{"value":"Vielen"},{"value":"Federn"},{"value":"Velen"},{"value":"Federt"},{"value":"Meter"},{"value":"Verler"},{"value":"Feder-"},{"value":"Eher"},{"value":"Fehlern"}],"offset":12,"length":5,"context":{"text":"Das ist ein Feler.","offset":12,"length":5},"sentence":"Das ist ein Feler.","type":{"typeName":"UnknownWord"},"rule":{"id":"GERMAN_SPELLER_RULE","description":"Möglicher Rechtschreibfehler","issueType":"misspelling","category":{"id":"TYPOS","name":"Mögliche Tippfehler"}},"ignoreForIncompleteSentence":false,"contextForSureMatch":0}],"sentenceRanges":[[0,18]],"extendedSentenceRanges":[{"from":0,"to":18,"detectedLanguages":[{"language":"de","rate":1.0}]}]}"####;
    const EXPECTED: [&str; 31] = [
        "Fehler", "Vieler", "Feuer", "Täler", "Feiern", "Feder", "Feier", "Feiert", "Felder",
        "Pfeiler", "Velar", "Vier", "Tälern", "Väter", "Feldern", "Aller", "Feuern", "Feger",
        "Viele", "Feuert", "Pfälzer", "Pfeilern", "Vielen", "Federn", "Velen", "Federt", "Meter",
        "Verler", "Feder-", "Eher", "Fehlern",
    ];
    assert_eq!(RAW.len(), 1663);
    assert_eq!(
        digest(RAW.as_bytes()),
        "397139b8389c9e7c541c12ba6cbbfca20434f9d481c7e0d69315c3b00a63510e"
    );
    let mut c = config();
    c.manual_language = Some("de-DE".into());
    c.expected_engine_version = Some("6.9-SNAPSHOT".into());
    let language = choose_language(&c, None).unwrap();
    let checked = parse_reply(
        RAW.as_bytes(),
        "Das ist ein Feler.",
        &language,
        &c,
        &Dictionary::default(),
    )
    .unwrap();
    assert_eq!(checked.response_language, "de-DE");
    assert_eq!(checked.response_bytes, 1663);
    assert_eq!(checked.response_sha256, digest(RAW.as_bytes()));
    assert_eq!(checked.software.version, "6.9-SNAPSHOT");
    assert!(checked.complete);
    assert_eq!(checked.accepted_count, Some(1));
    assert_eq!(checked.issues.len(), 1);
    let issue = &checked.issues[0];
    assert_eq!(issue.rule_id, "GERMAN_SPELLER_RULE");
    assert_eq!(issue.issue_type, "misspelling");
    assert_eq!(issue.offset_utf16, 12);
    assert_eq!(issue.length_utf16, 5);
    assert_eq!(issue.matched_text, "Feler");
    assert_eq!(issue.context, "Das ist ein Feler.");
    assert_eq!(issue.context_offset_utf16, 12);
    assert_eq!(issue.disposition, "accepted");
    assert_eq!(
        issue.replacements,
        EXPECTED.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>()
    );
}

#[test]
fn spelling_replacements_accepts_all_512_rejects_513_and_keeps_string_bounds() {
    let expected = (0..512)
        .map(|i| format!("suggestion-{i:03}"))
        .collect::<Vec<_>>();
    let mut v = reply("x", 0, 1);
    v["matches"][0]["replacements"] = json!(
        expected
            .iter()
            .map(|value| json!({"value":value}))
            .collect::<Vec<_>>()
    );
    let checked = parse(&v, "x", &config(), &Dictionary::default()).unwrap();
    assert_eq!(checked.issues[0].replacements, expected);
    assert_eq!(checked.accepted_count, Some(1));
    v["matches"][0]["replacements"]
        .as_array_mut()
        .unwrap()
        .push(json!({"value":"suggestion-512"}));
    assert_eq!(
        parse(&v, "x", &config(), &Dictionary::default()).unwrap_err(),
        "Provider replacements exceed bound"
    );
    v["matches"][0]["replacements"] = json!([{"value":"x".repeat(1024)}]);
    assert_eq!(
        parse(&v, "x", &config(), &Dictionary::default())
            .unwrap()
            .issues[0]
            .replacements[0]
            .len(),
        1024
    );
    v["matches"][0]["replacements"] = json!([{"value":"x".repeat(1025)}]);
    assert_eq!(
        parse(&v, "x", &config(), &Dictionary::default()).unwrap_err(),
        "Provider replacements exceed bound"
    );
    v["matches"][0]["replacements"] = json!([{"value":"x\0"}]);
    assert_eq!(
        parse(&v, "x", &config(), &Dictionary::default()).unwrap_err(),
        "Provider replacements exceed bound"
    );
}

#[test]
fn spelling_valid_json_exact_4mib_response_accepts_but_next_byte_refuses() {
    let c = config();
    let language = choose_language(&c, None).unwrap();
    let mut raw = serde_json::to_vec(&reply("x", 0, 1)).unwrap();
    raw.resize(MAX_RESPONSE_BYTES, b' ');
    let checked = parse_reply(&raw, "x", &language, &c, &Dictionary::default()).unwrap();
    assert_eq!(checked.response_bytes, 4 * 1024 * 1024);
    assert_eq!(checked.issues.len(), 1);
    assert_eq!(checked.issues[0].replacements, vec!["sentence".to_owned()]);
    raw.push(b' ');
    assert_eq!(
        parse_reply(&raw, "x", &language, &c, &Dictionary::default()).unwrap_err(),
        "Provider response byte bound"
    );
}
