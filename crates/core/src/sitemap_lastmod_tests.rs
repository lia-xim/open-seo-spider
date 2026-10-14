use super::*;
use crate::{SitemapParserContext, SitemapParserLimits, observe_sitemap_xml};

fn xsd(value: &str) -> SitemapLastmodValueAssessment {
    assess_value(value, Profile::Xsd)
}
fn w3c(value: &str) -> SitemapLastmodValueAssessment {
    assess_value(value, Profile::W3c)
}
fn valid(value: SitemapLastmodValueAssessment) -> SitemapLastmodParsedValue {
    match value {
        SitemapLastmodValueAssessment::Valid { value } => value,
        other => panic!("Expected valid value, got {other:?}"),
    }
}
fn invalid_xsd(value: &str, reason: SitemapLastmodInvalidReason) {
    assert_eq!(
        xsd(value),
        SitemapLastmodValueAssessment::Invalid { reason },
        "{value:?}"
    );
}
fn stored_entry(xml: &str) -> StoredSitemapEntry {
    let parsed = observe_sitemap_xml(
        xml.as_bytes(),
        SitemapParserContext::default(),
        &SitemapParserLimits::default(),
    )
    .unwrap();
    assert_eq!(parsed.entries.len(), 1);
    StoredSitemapEntry {
        sequence: 73,
        observation_id: "source".into(),
        entry_id: "entry".into(),
        observed: parsed.entries.into_iter().next().unwrap(),
    }
}
fn one_value(value: &str) -> StoredSitemapEntry {
    stored_entry(&format!(
        "<urlset><url><loc>https://example.org/</loc><lastmod>{value}</lastmod></url></urlset>"
    ))
}
fn assess(entry: &StoredSitemapEntry) -> SitemapLastmodEntryAssessment {
    assess_sitemap_lastmod_entry(entry, &SitemapLastmodLimits::default()).unwrap()
}

#[test]
fn official_date_and_datetime_examples_preserve_source_precision() {
    for lexical in [
        "2005-05-10",
        "2005-05-10T17:33:30+08:00",
        "1994-11-05T08:15:30-05:00",
        "1994-11-05T13:15:30Z",
    ] {
        assert!(
            matches!(xsd(lexical), SitemapLastmodValueAssessment::Valid { .. }),
            "{lexical}"
        );
        assert!(
            matches!(w3c(lexical), SitemapLastmodValueAssessment::Valid { .. }),
            "{lexical}"
        );
    }
    let date = valid(xsd("2005-05-10"));
    assert_eq!(date.precision, SitemapLastmodPrecision::Date);
    assert_eq!(
        (
            date.hour,
            date.minute,
            date.second,
            date.fraction,
            date.timezone_lexeme
        ),
        (None, None, None, None, None)
    );
    let time = valid(xsd("2005-05-10T17:33:30+08:00"));
    assert_eq!(
        (time.hour, time.minute, time.second),
        (Some(17), Some(33), Some(30))
    );
    assert_eq!(time.timezone_offset_minutes, Some(480));
}

#[test]
fn xsd_date_and_datetime_timezones_are_optional_and_dates_accept_zones() {
    for value in [
        "2026-10-14",
        "2026-10-14Z",
        "2026-10-14+14:00",
        "2026-10-14-14:00",
        "2026-10-14T12:05:06",
        "2026-10-14T12:05:06.003",
    ] {
        valid(xsd(value));
    }
    assert_eq!(
        valid(xsd("2026-10-14T12:05:06")).timezone_offset_minutes,
        None
    );
    assert!(matches!(
        w3c("2026-10-14T12:05:06"),
        SitemapLastmodValueAssessment::Invalid {
            reason: SitemapLastmodInvalidReason::InvalidTimezone
        }
    ));
    assert!(matches!(
        w3c("2026-10-14Z"),
        SitemapLastmodValueAssessment::Invalid { .. }
    ));
}

#[test]
fn w3c_all_six_precision_levels_are_separately_assessed() {
    for (value, precision) in [
        ("1997", SitemapLastmodPrecision::Year),
        ("1997-07", SitemapLastmodPrecision::Month),
        ("1997-07-16", SitemapLastmodPrecision::Date),
        ("1997-07-16T19:20+01:00", SitemapLastmodPrecision::Minute),
        ("1997-07-16T19:20:30+01:00", SitemapLastmodPrecision::Second),
        (
            "1997-07-16T19:20:30.45+01:00",
            SitemapLastmodPrecision::FractionalSecond,
        ),
    ] {
        assert_eq!(valid(w3c(value)).precision, precision);
        if matches!(
            precision,
            SitemapLastmodPrecision::Year
                | SitemapLastmodPrecision::Month
                | SitemapLastmodPrecision::Minute
        ) {
            invalid_xsd(value, SitemapLastmodInvalidReason::LexicalGrammar);
            let assessment = assess(&one_value(value));
            assert!(
                assessment
                    .findings
                    .contains(&SitemapLastmodFinding::SchemaProfileDisagreement {
                        field_ordinal: 1
                    })
            );
        }
    }
    for value in [
        "1997Z",
        "1997-07Z",
        "1997-07-16T19:20",
        "1997-07-16T19:20.5Z",
    ] {
        assert!(
            matches!(w3c(value), SitemapLastmodValueAssessment::Invalid { .. }),
            "{value}"
        );
    }
}

#[test]
fn timezone_bounds_and_exact_signed_zero_lexemes_are_retained() {
    for (zone, minutes) in [
        ("Z", 0),
        ("+00:00", 0),
        ("-00:00", 0),
        ("+14:00", 840),
        ("-14:00", -840),
        ("+13:59", 839),
        ("-13:59", -839),
    ] {
        let parsed = valid(xsd(&format!("2000-01-01T00:00:00{zone}")));
        assert_eq!(parsed.timezone_lexeme.as_deref(), Some(zone));
        assert_eq!(parsed.timezone_offset_minutes, Some(minutes));
    }
    for zone in [
        "+14:01", "-14:01", "+15:00", "-15:00", "+00:60", "-00:60", "+24:00", "z", "+1:00",
        "+0100", "+01", "Z ", "ZZ",
    ] {
        invalid_xsd(
            &format!("2000-01-01T00:00:00{zone}"),
            SitemapLastmodInvalidReason::InvalidTimezone,
        );
    }
    // The NOTE supplies its own hh=00..23 grammar, distinct from XSD's 14:00 limit.
    let separate = valid(w3c("2000-01-01T00:00:00+23:59"));
    assert_eq!(separate.timezone_offset_minutes, Some(1439));
    invalid_xsd(
        "2000-01-01T00:00:00+23:59",
        SitemapLastmodInvalidReason::InvalidTimezone,
    );
}

#[test]
fn calendar_dates_cover_a_complete_gregorian_cycle_against_chrono() {
    for year in 1600..2000 {
        for month in 1..=12 {
            for day in 0..=32 {
                let value = format!("{year:04}-{month:02}-{day:02}");
                let expected = chrono::NaiveDate::from_ymd_opt(year, month, day).is_some();
                assert_eq!(
                    matches!(xsd(&value), SitemapLastmodValueAssessment::Valid { .. }),
                    expected,
                    "{value}"
                );
            }
        }
    }
}

#[test]
fn invalid_calendar_values_do_not_become_valid_timestamps() {
    for date in [
        "2026-02-29",
        "1900-02-29",
        "2000-02-30",
        "2026-04-31",
        "2026-00-01",
        "2026-13-01",
        "2026-01-00",
        "2026-01-32",
    ] {
        invalid_xsd(date, SitemapLastmodInvalidReason::InvalidCalendarDate);
        invalid_xsd(
            &format!("{date}T00:00:00Z"),
            SitemapLastmodInvalidReason::InvalidCalendarDate,
        );
    }
    valid(xsd("2000-02-29"));
    valid(xsd("2400-02-29"));
}

#[test]
fn xsd_bce_years_use_no_year_zero_calendar_and_keep_exact_years() {
    for value in ["-0001-02-29", "-0005-02-29", "-0401-02-29", "-10001-02-29"] {
        let parsed = valid(xsd(value));
        assert!(parsed.year.starts_with('-'));
        assert!(matches!(
            w3c(value),
            SitemapLastmodValueAssessment::Invalid { .. }
        ));
    }
    for value in ["-0004-02-29", "-0101-02-29", "-0400-02-29"] {
        invalid_xsd(value, SitemapLastmodInvalidReason::InvalidCalendarDate);
    }
    for year in -400..0 {
        let value = format!("-{:04}-02-29", -year);
        let expected = chrono::NaiveDate::from_ymd_opt(year + 1, 2, 29).is_some();
        assert_eq!(
            matches!(xsd(&value), SitemapLastmodValueAssessment::Valid { .. }),
            expected,
            "{value}"
        );
    }
}

#[test]
fn expanded_years_and_fractions_do_not_overflow_or_round() {
    let year = format!("1{}", "0".repeat(5000));
    let fraction = format!("{}7", "0".repeat(5000));
    let text = format!("{year}-02-29T23:59:59.{fraction}-14:00");
    let parsed = valid(xsd(&text));
    assert_eq!(parsed.year, year);
    assert_eq!(parsed.fraction.as_deref(), Some(fraction.as_str()));
    assert_eq!(parsed.timezone_offset_minutes, Some(-840));
    valid(xsd(&format!("-{year}1-02-29")));
    invalid_xsd("02000-01-01", SitemapLastmodInvalidReason::LexicalGrammar);
    invalid_xsd("-02000-01-01", SitemapLastmodInvalidReason::LexicalGrammar);
    invalid_xsd("+2000-01-01", SitemapLastmodInvalidReason::LexicalGrammar);
    invalid_xsd("200-01-01", SitemapLastmodInvalidReason::LexicalGrammar);
}

#[test]
fn xsd_year_zero_is_invalid_and_note_year_zero_remains_undetermined() {
    for value in ["0000-01-01", "-0000-01-01", "0000-01-01T00:00:00Z"] {
        invalid_xsd(value, SitemapLastmodInvalidReason::YearZero);
    }
    for value in ["0000", "0000-01", "0000-02-29", "0000-01-01T00:00:00Z"] {
        assert!(matches!(
            w3c(value),
            SitemapLastmodValueAssessment::Undetermined {
                reason: SitemapLastmodUndeterminedReason::W3cYearZeroNotSpecified,
                ..
            }
        ));
    }
}

#[test]
fn end_of_day_is_xsd_only_and_all_subsecond_digits_must_be_zero() {
    for value in [
        "2000-12-31T24:00:00",
        "2000-12-31T24:00:00Z",
        "2000-12-31T24:00:00.0000-14:00",
        "-0001-12-31T24:00:00+14:00",
    ] {
        let parsed = valid(xsd(value));
        assert!(parsed.end_of_day);
        assert_eq!(parsed.hour, Some(24));
        assert!(matches!(
            w3c(value),
            SitemapLastmodValueAssessment::Invalid { .. }
        ));
    }
    for time in [
        "24:00:01",
        "24:01:00",
        "24:00:00.0001",
        "25:00:00",
        "23:60:00",
        "23:59:61",
    ] {
        invalid_xsd(
            &format!("2000-01-01T{time}Z"),
            SitemapLastmodInvalidReason::InvalidTime,
        );
    }
    invalid_xsd(
        "2000-01-01T24:00Z",
        SitemapLastmodInvalidReason::LexicalGrammar,
    );
}

#[test]
fn leap_second_is_not_declared_valid_or_invalid_without_external_evidence() {
    for value in [
        "2016-12-31T23:59:60Z",
        "2017-01-01T00:59:60+01:00",
        "2999-01-01T00:00:60",
        "2016-12-31T23:59:60.123Z",
    ] {
        match xsd(value) {
            SitemapLastmodValueAssessment::Undetermined {
                reason,
                lexical_value,
            } => {
                assert_eq!(
                    reason,
                    SitemapLastmodUndeterminedReason::LeapSecondNeedsExternalEvidence
                );
                assert_eq!(lexical_value.second, Some(60));
            }
            other => panic!("{value} {other:?}"),
        }
        assert!(matches!(
            w3c(value),
            SitemapLastmodValueAssessment::Invalid { .. }
        ));
        let assessment = assess(&one_value(value));
        assert!(
            assessment
                .findings
                .contains(&SitemapLastmodFinding::SchemaValueUndetermined {
                    field_ordinal: 1,
                    reason: SitemapLastmodUndeterminedReason::LeapSecondNeedsExternalEvidence
                })
        );
        assert!(
            !assessment
                .findings
                .iter()
                .any(|f| matches!(f, SitemapLastmodFinding::InvalidSchemaValue { .. }))
        );
    }
}

#[test]
fn grammar_rejects_locale_unicode_lowercase_and_unexpected_suffixes() {
    for value in [
        "2026/10/14",
        "14.10.2026",
        "20261014",
        "2026-1-01",
        "2026-01-1",
        "2026-01-01t00:00:00Z",
        "2026-01-01 00:00:00Z",
        "2026-01-01T0:00:00Z",
        "2026-01-01T00:00:0Z",
        "2026-01-01T00:00:00.Z",
        "2026-01-01T00:00:00,1Z",
        "２０２６-01-01",
        "2026-01-01\u{a0}",
        "2026-01-01\0",
    ] {
        assert!(
            matches!(xsd(value), SitemapLastmodValueAssessment::Invalid { .. }),
            "{value:?}"
        );
    }
    invalid_xsd("", SitemapLastmodInvalidReason::Empty);
}

#[test]
fn real_xml_decoding_preserves_original_entity_cdata_comments_and_byte_spans() {
    let xml = "<urlset><url><loc>https://example.org/ä</loc><lastmod>\t&#50;000-01-01\r\n</lastmod><lastmod><![CDATA[2000-01-01T12:34:56.000000001Z]]></lastmod><lastmod>2000-<!--marker-->01-01</lastmod></url></urlset>";
    let entry = stored_entry(xml);
    let before = entry.clone();
    let assessment = assess(&entry);
    assert_eq!(entry, before);
    assert_eq!(assessment.fields.len(), 3);
    for (field, observed) in assessment.fields.iter().zip(&entry.observed.lastmod) {
        assert_eq!(&field.observed, observed);
        assert_eq!(
            &xml[observed.xml_byte_start as usize..observed.xml_byte_end as usize],
            observed.raw_element_xml
        );
        assert!(matches!(
            field.xsd,
            SitemapLastmodValueAssessment::Valid { .. }
        ));
    }
    assert_eq!(assessment.fields[0].evaluated_value, "2000-01-01");
    assert!(
        assessment.fields[0]
            .observed
            .raw_inner_xml
            .contains("&#50;")
    );
    assert!(
        assessment.fields[1]
            .observed
            .raw_inner_xml
            .contains("<![CDATA[")
    );
    assert!(
        assessment.fields[2]
            .observed
            .raw_inner_xml
            .contains("<!--marker-->")
    );
    assert_eq!(
        valid(assessment.fields[1].xsd.clone()).fraction.as_deref(),
        Some("000000001")
    );
}

#[test]
fn xml_whitespace_collapses_without_joining_date_fragments() {
    let good = assess(&one_value(" \t2000-01-01\r\n "));
    assert_eq!(good.fields[0].evaluated_value, "2000-01-01");
    valid(good.fields[0].xsd.clone());
    for value in ["2000-\t01-01", "2000-01-\n01", "2000-01-01&#160;"] {
        let result = assess(&one_value(value));
        assert!(matches!(
            result.fields[0].xsd,
            SitemapLastmodValueAssessment::Invalid { .. }
        ));
    }
}

#[test]
fn optional_absence_empty_and_duplicate_elements_remain_distinct() {
    let absent = assess(&stored_entry(
        "<urlset><url><loc>https://example.org/</loc></url></urlset>",
    ));
    assert_eq!(absent.presence, SitemapLastmodPresence::AbsentOptional);
    assert!(absent.findings.is_empty());
    let empty = assess(&stored_entry(
        "<urlset><url><loc>https://example.org/</loc><lastmod/></url></urlset>",
    ));
    assert_eq!(empty.presence, SitemapLastmodPresence::ObservedOne);
    assert_eq!(
        empty.fields[0].xsd,
        SitemapLastmodValueAssessment::Invalid {
            reason: SitemapLastmodInvalidReason::Empty
        }
    );
    let duplicated = assess(&stored_entry(
        "<urlset><url><loc>https://example.org/</loc><lastmod>2000-01-01</lastmod><lastmod>bad</lastmod></url></urlset>",
    ));
    assert_eq!(
        duplicated.presence,
        SitemapLastmodPresence::ObservedMultiple
    );
    assert_eq!(
        duplicated.findings[0],
        SitemapLastmodFinding::MultipleLastmodElements { observed_count: 2 }
    );
    assert!(
        duplicated
            .findings
            .contains(&SitemapLastmodFinding::InvalidSchemaValue {
                field_ordinal: 2,
                reason: SitemapLastmodInvalidReason::LexicalGrammar
            })
    );
    assert_eq!(duplicated.fields[0].observed.ordinal, 1);
    assert_eq!(duplicated.fields[1].observed.ordinal, 2);
}

#[test]
fn incomplete_entry_cannot_claim_absence_or_all_fields_but_retains_closed_facts() {
    let absent = assess(&stored_entry(
        "<urlset><url><loc>https://example.org/</loc>",
    ));
    assert_eq!(
        absent.presence,
        SitemapLastmodPresence::UnobservedIncompleteEntry
    );
    assert!(!absent.entry_complete);
    assert!(!absent.all_entry_fields_observed);
    let present = assess(&stored_entry(
        "<urlset><url><lastmod>bad</lastmod><lastmod>",
    ));
    assert_eq!(present.presence, SitemapLastmodPresence::ObservedOne);
    assert!(!present.all_entry_fields_observed);
    assert_eq!(present.fields.len(), 1);
    assert_eq!(present.fields[0].observed.decoded_value, "bad");
    assert!(
        present
            .findings
            .contains(&SitemapLastmodFinding::InvalidSchemaValue {
                field_ordinal: 1,
                reason: SitemapLastmodInvalidReason::LexicalGrammar
            })
    );
}

#[test]
fn identity_json_roundtrip_and_no_inferred_freshness_or_http_assessment() {
    let entry = one_value("9999-12-31T23:59:59.0000000001+14:00");
    let result = assess(&entry);
    assert_eq!(result.entry_id, entry.entry_id);
    assert_eq!(result.observation_id, entry.observation_id);
    assert_eq!(result.entry_sequence, 73);
    assert_eq!(result.entry_ordinal, entry.observed.ordinal);
    assert_eq!(
        (result.entry_xml_byte_start, result.entry_xml_byte_end),
        (entry.observed.xml_byte_start, entry.observed.xml_byte_end)
    );
    assert!(!result.freshness_assessed);
    assert!(!result.http_status_assessed);
    assert_eq!(
        serde_json::from_slice::<SitemapLastmodEntryAssessment>(
            &serde_json::to_vec(&result).unwrap()
        )
        .unwrap(),
        result
    );
}

#[test]
fn configuration_field_and_value_limits_fail_atomically_with_actual_ordinals() {
    let entry = one_value("2000-01-01");
    let original = entry.clone();
    for limits in [
        SitemapLastmodLimits {
            max_fields: 0,
            ..Default::default()
        },
        SitemapLastmodLimits {
            max_fields: MAX_FIELDS + 1,
            ..Default::default()
        },
        SitemapLastmodLimits {
            max_value_bytes: 0,
            ..Default::default()
        },
        SitemapLastmodLimits {
            max_value_bytes: MAX_VALUE_BYTES + 1,
            ..Default::default()
        },
        SitemapLastmodLimits {
            max_retained_bytes: 0,
            ..Default::default()
        },
        SitemapLastmodLimits {
            max_retained_bytes: MAX_RETAINED_BYTES + 1,
            ..Default::default()
        },
    ] {
        assert_eq!(
            assess_sitemap_lastmod_entry(&entry, &limits).unwrap_err(),
            failure(SitemapLastmodFailureCode::InvalidLimits, None)
        );
    }
    let tight = SitemapLastmodLimits {
        max_value_bytes: 10,
        ..Default::default()
    };
    assess_sitemap_lastmod_entry(&entry, &tight).unwrap();
    let tighter = SitemapLastmodLimits {
        max_value_bytes: 9,
        ..Default::default()
    };
    assert_eq!(
        assess_sitemap_lastmod_entry(&entry, &tighter).unwrap_err(),
        failure(SitemapLastmodFailureCode::ValueBytesLimit, Some(1))
    );
    let two = stored_entry(
        "<urlset><url><lastmod>2000-01-01</lastmod><lastmod>2001-01-01</lastmod></url></urlset>",
    );
    assert_eq!(
        assess_sitemap_lastmod_entry(
            &two,
            &SitemapLastmodLimits {
                max_fields: 1,
                ..Default::default()
            }
        )
        .unwrap_err(),
        failure(SitemapLastmodFailureCode::FieldCountLimit, None)
    );
    assert_eq!(entry, original);
}

#[test]
fn exact_retained_string_limit_includes_ids_raw_xml_and_both_profiles() {
    let empty = stored_entry("<urlset><url/></urlset>");
    // IDs are exactly 11 bytes, even when the optional field is absent.
    assess_sitemap_lastmod_entry(
        &empty,
        &SitemapLastmodLimits {
            max_retained_bytes: 11,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        assess_sitemap_lastmod_entry(
            &empty,
            &SitemapLastmodLimits {
                max_retained_bytes: 10,
                ..Default::default()
            }
        )
        .unwrap_err(),
        failure(SitemapLastmodFailureCode::RetainedBytesLimit, None)
    );
    let entry = one_value("2000-01-01");
    // 11 IDs + 29 raw element + 10 raw inner + 4 * 10 original/evaluated/profile lexemes.
    assess_sitemap_lastmod_entry(
        &entry,
        &SitemapLastmodLimits {
            max_retained_bytes: 90,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        assess_sitemap_lastmod_entry(
            &entry,
            &SitemapLastmodLimits {
                max_retained_bytes: 89,
                ..Default::default()
            }
        )
        .unwrap_err(),
        failure(SitemapLastmodFailureCode::RetainedBytesLimit, Some(1))
    );
}

#[test]
fn arbitrary_unicode_and_every_ascii_byte_cannot_panic_the_pure_assessor() {
    for byte in 0u8..=127 {
        for prefix in ["", "2000-01-01", "2000-01-01T00:00:00", "-", "2000-01-01+"] {
            let value = format!("{prefix}{}", char::from(byte));
            let _ = xsd(&value);
            let _ = w3c(&value);
        }
    }
    for value in [
        "🕷",
        "\u{feff}2000-01-01",
        "é-01-01",
        "2000-01-01T00:00:00é",
        "-é",
        "e\u{301}",
    ] {
        assert!(matches!(
            xsd(value),
            SitemapLastmodValueAssessment::Invalid { .. }
        ));
    }
}

#[test]
fn explicit_lexical_assessment_has_no_fabricated_stored_authority_or_xml_decoding() {
    let text = " \t1997-07-16T19:20+01:00\r\n";
    let assessed = assess_sitemap_lastmod_value(text, &SitemapLastmodLimits::default()).unwrap();
    assert_eq!(assessed.evaluated_value, "1997-07-16T19:20+01:00");
    assert!(matches!(
        assessed.xsd,
        SitemapLastmodValueAssessment::Invalid { .. }
    ));
    assert_eq!(
        valid(assessed.w3c_note.clone()).precision,
        SitemapLastmodPrecision::Minute
    );
    let object = serde_json::to_value(&assessed).unwrap();
    assert_eq!(
        object
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["evaluated_value", "w3c_note", "xsd"]
    );
    let entity_text =
        assess_sitemap_lastmod_value("&#50;000-01-01", &SitemapLastmodLimits::default()).unwrap();
    assert_eq!(entity_text.evaluated_value, "&#50;000-01-01");
    assert!(matches!(
        entity_text.xsd,
        SitemapLastmodValueAssessment::Invalid { .. }
    ));
    let ambiguous =
        assess_sitemap_lastmod_value("2016-12-31T23:59:60Z", &SitemapLastmodLimits::default())
            .unwrap();
    assert!(matches!(
        ambiguous.xsd,
        SitemapLastmodValueAssessment::Undetermined { .. }
    ));
    let bce =
        assess_sitemap_lastmod_value("-0001-02-29", &SitemapLastmodLimits::default()).unwrap();
    assert!(matches!(
        bce.xsd,
        SitemapLastmodValueAssessment::Valid { .. }
    ));
}

#[test]
fn explicit_lexical_value_limits_fail_before_partial_normalization() {
    let value = "2000-01-01";
    let exact = SitemapLastmodLimits {
        max_value_bytes: 10,
        max_retained_bytes: 30,
        ..Default::default()
    };
    let before = value.to_owned();
    let assessed = assess_sitemap_lastmod_value(value, &exact).unwrap();
    assert_eq!(valid(assessed.xsd).precision, SitemapLastmodPrecision::Date);
    assert_eq!(value, before);
    let narrow_value = SitemapLastmodLimits {
        max_value_bytes: 9,
        ..exact.clone()
    };
    assert_eq!(
        assess_sitemap_lastmod_value(value, &narrow_value).unwrap_err(),
        failure(SitemapLastmodFailureCode::ValueBytesLimit, None)
    );
    let narrow_retained = SitemapLastmodLimits {
        max_retained_bytes: 29,
        ..exact
    };
    assert_eq!(
        assess_sitemap_lastmod_value(value, &narrow_retained).unwrap_err(),
        failure(SitemapLastmodFailureCode::RetainedBytesLimit, None)
    );
}
