//! Pure assessment of retained Sitemap lastmod facts. No HTTP, storage writes,
//! inferred fetch dates, system clock, UTC conversion, or source-cohort claim.
//!
//! Primary datatype contract: Sitemap 0.9 tLastmod, the xsd:date|xsd:dateTime
//! union using XML Schema 1.0 Second Edition (2004), sections 3.2.7 and 3.2.9.
//! The separately reported W3C NOTE-datetime profile includes reduced precision.
//! Neither assessment is validation of the entire XML document or its ordering.
use crate::{SitemapFieldOccurrence, StoredSitemapEntry};
use serde::{Deserialize, Serialize};

pub const SITEMAP_LASTMOD_ENGINE: &str = "sitemap-lastmod/xsd-1.0-second-edition+w3c-note-1998/v1";
pub const SITEMAP_LASTMOD_XSD_PROFILE: &str =
    "sitemap-0.9/tLastmod/xsd-1.0-second-edition/date|dateTime";
pub const SITEMAP_LASTMOD_W3C_PROFILE: &str = "W3C-NOTE-datetime-19980827/six-precision-levels";
const MAX_FIELDS: usize = 10_000;
const MAX_VALUE_BYTES: usize = 256 * 1024;
const MAX_RETAINED_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SitemapLastmodLimits {
    pub max_fields: usize,
    pub max_value_bytes: usize,
    /// All copied original strings plus worst-case evaluated/parsed lexemes.
    /// This bounds retained string data, not the serialized JSON envelope.
    pub max_retained_bytes: usize,
}
impl Default for SitemapLastmodLimits {
    fn default() -> Self {
        Self {
            max_fields: MAX_FIELDS,
            max_value_bytes: MAX_VALUE_BYTES,
            max_retained_bytes: MAX_RETAINED_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapLastmodFailureCode {
    InvalidLimits,
    FieldCountLimit,
    ValueBytesLimit,
    RetainedBytesLimit,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapLastmodFailure {
    pub code: SitemapLastmodFailureCode,
    pub field_ordinal: Option<usize>,
}
impl std::fmt::Display for SitemapLastmodFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Sitemap lastmod {:?}, field {:?}",
            self.code, self.field_ordinal
        )
    }
}
impl std::error::Error for SitemapLastmodFailure {}
fn failure(code: SitemapLastmodFailureCode, field_ordinal: Option<usize>) -> SitemapLastmodFailure {
    SitemapLastmodFailure {
        code,
        field_ordinal,
    }
}
impl SitemapLastmodLimits {
    pub fn validate(&self) -> Result<(), SitemapLastmodFailure> {
        if self.max_fields == 0
            || self.max_fields > MAX_FIELDS
            || self.max_value_bytes == 0
            || self.max_value_bytes > MAX_VALUE_BYTES
            || self.max_retained_bytes == 0
            || self.max_retained_bytes > MAX_RETAINED_BYTES
        {
            return Err(failure(SitemapLastmodFailureCode::InvalidLimits, None));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapLastmodPresence {
    AbsentOptional,
    UnobservedIncompleteEntry,
    ObservedOne,
    ObservedMultiple,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapLastmodPrecision {
    Year,
    Month,
    Date,
    Minute,
    Second,
    FractionalSecond,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapLastmodParsedValue {
    pub precision: SitemapLastmodPrecision,
    /// Exact signed year lexeme; no machine-integer limit on valid XSD years.
    pub year: String,
    pub month: Option<u8>,
    pub day: Option<u8>,
    pub hour: Option<u8>,
    pub minute: Option<u8>,
    pub second: Option<u8>,
    /// Exact fraction digits. No rounding or precision loss.
    pub fraction: Option<String>,
    /// None means the source supplies no timezone. Never fill with local/UTC.
    pub timezone_lexeme: Option<String>,
    pub timezone_offset_minutes: Option<i16>,
    /// Lexical 24:00:00 represents next-day midnight; originals stay unchanged.
    pub end_of_day: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapLastmodInvalidReason {
    Empty,
    LexicalGrammar,
    YearZero,
    InvalidCalendarDate,
    InvalidTime,
    InvalidTimezone,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SitemapLastmodUndeterminedReason {
    LeapSecondNeedsExternalEvidence,
    W3cYearZeroNotSpecified,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SitemapLastmodValueAssessment {
    Valid {
        value: SitemapLastmodParsedValue,
    },
    Invalid {
        reason: SitemapLastmodInvalidReason,
    },
    Undetermined {
        reason: SitemapLastmodUndeterminedReason,
        lexical_value: SitemapLastmodParsedValue,
    },
}

/// Pure lexical projection for explicit caller-supplied values. This carries
/// no entry/source identity, XML occurrence, byte span or freshness authority.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapLastmodLexicalAssessment {
    pub evaluated_value: String,
    pub xsd: SitemapLastmodValueAssessment,
    pub w3c_note: SitemapLastmodValueAssessment,
}

/// Assess an already decoded textual value without inventing a stored XML
/// occurrence. XML generation callers retain their original/provenance and
/// escape it independently; this function does not decode XML references.
/// Bounds fail before copied normalized/parsed strings, with no partial DTO.
pub fn assess_sitemap_lastmod_value(
    value: &str,
    limits: &SitemapLastmodLimits,
) -> Result<SitemapLastmodLexicalAssessment, SitemapLastmodFailure> {
    limits.validate()?;
    if value.len() > limits.max_value_bytes {
        return Err(failure(SitemapLastmodFailureCode::ValueBytesLimit, None));
    }
    // One normalized copy plus both profiles' total exact parsed lexemes.
    if value
        .len()
        .checked_mul(3)
        .is_none_or(|bytes| bytes > limits.max_retained_bytes)
    {
        return Err(failure(SitemapLastmodFailureCode::RetainedBytesLimit, None));
    }
    let evaluated_value = collapse_xml_whitespace(value);
    Ok(SitemapLastmodLexicalAssessment {
        xsd: assess_value(&evaluated_value, Profile::Xsd),
        w3c_note: assess_value(&evaluated_value, Profile::W3c),
        evaluated_value,
    })
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapLastmodFieldAssessment {
    pub observed: SitemapFieldOccurrence,
    /// XML Schema whiteSpace=collapse. Internal whitespace stays a separator,
    /// so two date fragments are never concatenated into a valid value.
    pub evaluated_value: String,
    pub xsd: SitemapLastmodValueAssessment,
    pub w3c_note: SitemapLastmodValueAssessment,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum SitemapLastmodFinding {
    MultipleLastmodElements {
        observed_count: usize,
    },
    InvalidSchemaValue {
        field_ordinal: usize,
        reason: SitemapLastmodInvalidReason,
    },
    SchemaProfileDisagreement {
        field_ordinal: usize,
    },
    SchemaValueUndetermined {
        field_ordinal: usize,
        reason: SitemapLastmodUndeterminedReason,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SitemapLastmodEntryAssessment {
    pub engine: String,
    pub xsd_profile: String,
    pub w3c_note_profile: String,
    pub entry_id: String,
    pub observation_id: String,
    pub entry_sequence: u64,
    pub entry_ordinal: usize,
    pub entry_xml_byte_start: u64,
    pub entry_xml_byte_end: u64,
    pub entry_complete: bool,
    pub presence: SitemapLastmodPresence,
    /// False for retained entry prefixes, even if a closed lastmod was assessed.
    pub all_entry_fields_observed: bool,
    pub findings: Vec<SitemapLastmodFinding>,
    pub fields: Vec<SitemapLastmodFieldAssessment>,
    pub freshness_assessed: bool,
    pub http_status_assessed: bool,
}

/// Assess an entry supplied by a verified catalog/read snapshot. Identity and
/// spans are copied, not independently authenticated by this pure projection.
/// Limits fail atomically before cloning originals; there is no partial result.
pub fn assess_sitemap_lastmod_entry(
    entry: &StoredSitemapEntry,
    limits: &SitemapLastmodLimits,
) -> Result<SitemapLastmodEntryAssessment, SitemapLastmodFailure> {
    limits.validate()?;
    let source = &entry.observed;
    if source.lastmod.len() > limits.max_fields {
        return Err(failure(SitemapLastmodFailureCode::FieldCountLimit, None));
    }
    let mut bytes = entry
        .entry_id
        .len()
        .checked_add(entry.observation_id.len())
        .ok_or_else(|| failure(SitemapLastmodFailureCode::RetainedBytesLimit, None))?;
    for field in &source.lastmod {
        if field.decoded_value.len() > limits.max_value_bytes {
            return Err(failure(
                SitemapLastmodFailureCode::ValueBytesLimit,
                Some(field.ordinal),
            ));
        }
        // One evaluated copy and up to two parsed lexeme copies per profile.
        for len in [
            field.raw_element_xml.len(),
            field.raw_inner_xml.len(),
            field.decoded_value.len(),
            field.decoded_value.len(),
            field.decoded_value.len(),
            field.decoded_value.len(),
        ] {
            bytes = bytes.checked_add(len).ok_or_else(|| {
                failure(
                    SitemapLastmodFailureCode::RetainedBytesLimit,
                    Some(field.ordinal),
                )
            })?;
        }
        if bytes > limits.max_retained_bytes {
            return Err(failure(
                SitemapLastmodFailureCode::RetainedBytesLimit,
                Some(field.ordinal),
            ));
        }
    }
    if bytes > limits.max_retained_bytes {
        return Err(failure(SitemapLastmodFailureCode::RetainedBytesLimit, None));
    }
    let mut findings = Vec::new();
    if source.lastmod.len() > 1 {
        findings.push(SitemapLastmodFinding::MultipleLastmodElements {
            observed_count: source.lastmod.len(),
        });
    }
    let fields = source
        .lastmod
        .iter()
        .map(|field| {
            let evaluated_value = collapse_xml_whitespace(&field.decoded_value);
            let xsd = assess_value(&evaluated_value, Profile::Xsd);
            let w3c_note = assess_value(&evaluated_value, Profile::W3c);
            match &xsd {
                SitemapLastmodValueAssessment::Invalid { reason } => {
                    findings.push(SitemapLastmodFinding::InvalidSchemaValue {
                        field_ordinal: field.ordinal,
                        reason: *reason,
                    });
                    if matches!(w3c_note, SitemapLastmodValueAssessment::Valid { .. }) {
                        findings.push(SitemapLastmodFinding::SchemaProfileDisagreement {
                            field_ordinal: field.ordinal,
                        });
                    }
                }
                SitemapLastmodValueAssessment::Undetermined { reason, .. } => {
                    findings.push(SitemapLastmodFinding::SchemaValueUndetermined {
                        field_ordinal: field.ordinal,
                        reason: *reason,
                    })
                }
                SitemapLastmodValueAssessment::Valid { .. } => {
                    if !matches!(w3c_note, SitemapLastmodValueAssessment::Valid { .. }) {
                        findings.push(SitemapLastmodFinding::SchemaProfileDisagreement {
                            field_ordinal: field.ordinal,
                        });
                    }
                }
            }
            SitemapLastmodFieldAssessment {
                observed: field.clone(),
                evaluated_value,
                xsd,
                w3c_note,
            }
        })
        .collect();
    let presence = match (source.lastmod.len(), source.complete) {
        (0, true) => SitemapLastmodPresence::AbsentOptional,
        (0, false) => SitemapLastmodPresence::UnobservedIncompleteEntry,
        (1, _) => SitemapLastmodPresence::ObservedOne,
        _ => SitemapLastmodPresence::ObservedMultiple,
    };
    Ok(SitemapLastmodEntryAssessment {
        engine: SITEMAP_LASTMOD_ENGINE.into(),
        xsd_profile: SITEMAP_LASTMOD_XSD_PROFILE.into(),
        w3c_note_profile: SITEMAP_LASTMOD_W3C_PROFILE.into(),
        entry_id: entry.entry_id.clone(),
        observation_id: entry.observation_id.clone(),
        entry_sequence: entry.sequence,
        entry_ordinal: source.ordinal,
        entry_xml_byte_start: source.xml_byte_start,
        entry_xml_byte_end: source.xml_byte_end,
        entry_complete: source.complete,
        presence,
        all_entry_fields_observed: source.complete,
        findings,
        fields,
        freshness_assessed: false,
        http_status_assessed: false,
    })
}

fn collapse_xml_whitespace(value: &str) -> String {
    value
        .split([' ', '\t', '\r', '\n'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Profile {
    Xsd,
    W3c,
}
fn invalid(reason: SitemapLastmodInvalidReason) -> SitemapLastmodValueAssessment {
    SitemapLastmodValueAssessment::Invalid { reason }
}
fn assess_value(value: &str, profile: Profile) -> SitemapLastmodValueAssessment {
    use SitemapLastmodInvalidReason as Reason;
    if value.is_empty() {
        return invalid(Reason::Empty);
    }
    let parsed = match parse_value(value, profile) {
        Ok(parsed) => parsed,
        Err(reason) => return invalid(reason),
    };
    if parsed
        .year
        .trim_start_matches('-')
        .bytes()
        .all(|b| b == b'0')
    {
        if profile == Profile::W3c {
            return SitemapLastmodValueAssessment::Undetermined {
                reason: SitemapLastmodUndeterminedReason::W3cYearZeroNotSpecified,
                lexical_value: parsed,
            };
        }
        return invalid(Reason::YearZero);
    }
    if parsed.second == Some(60) {
        return SitemapLastmodValueAssessment::Undetermined {
            reason: SitemapLastmodUndeterminedReason::LeapSecondNeedsExternalEvidence,
            lexical_value: parsed,
        };
    }
    SitemapLastmodValueAssessment::Valid { value: parsed }
}

fn parse_value(
    value: &str,
    profile: Profile,
) -> Result<SitemapLastmodParsedValue, SitemapLastmodInvalidReason> {
    use SitemapLastmodInvalidReason as Reason;
    let b = value.as_bytes();
    if !value.is_ascii() {
        return Err(Reason::LexicalGrammar);
    }
    let negative = b.first() == Some(&b'-');
    let start = usize::from(negative);
    let year_digits = b[start..].iter().take_while(|b| b.is_ascii_digit()).count();
    if year_digits < 4
        || (year_digits > 4 && b[start] == b'0')
        || (profile == Profile::W3c && (negative || year_digits != 4))
    {
        return Err(Reason::LexicalGrammar);
    }
    let mut pos = start + year_digits;
    let mut parsed = SitemapLastmodParsedValue {
        precision: SitemapLastmodPrecision::Year,
        year: value[..pos].into(),
        month: None,
        day: None,
        hour: None,
        minute: None,
        second: None,
        fraction: None,
        timezone_lexeme: None,
        timezone_offset_minutes: None,
        end_of_day: false,
    };
    if pos == b.len() {
        return if profile == Profile::W3c {
            Ok(parsed)
        } else {
            Err(Reason::LexicalGrammar)
        };
    }
    punctuation(b, &mut pos, b'-')?;
    let month = digits2(b, &mut pos)?;
    if !(1..=12).contains(&month) {
        return Err(Reason::InvalidCalendarDate);
    }
    parsed.month = Some(month);
    parsed.precision = SitemapLastmodPrecision::Month;
    if pos == b.len() {
        return if profile == Profile::W3c {
            Ok(parsed)
        } else {
            Err(Reason::LexicalGrammar)
        };
    }
    punctuation(b, &mut pos, b'-')?;
    let day = digits2(b, &mut pos)?;
    let remainder = b[start..start + year_digits]
        .iter()
        .fold(0u16, |r, digit| (r * 10 + u16::from(digit - b'0')) % 400);
    // XSD 1.0 has no year zero: -0001 is 1 BCE, the proleptic Gregorian
    // astronomical year 0 (a leap year), -0005 is year -4, and so on.
    // Work on a bounded remainder, never parse an expanded year into i64.
    let calendar_remainder = if negative {
        (401 - remainder) % 400
    } else {
        remainder
    };
    let leap =
        calendar_remainder == 0 || (calendar_remainder % 4 == 0 && calendar_remainder % 100 != 0);
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if day == 0 || day > days {
        return Err(Reason::InvalidCalendarDate);
    }
    parsed.day = Some(day);
    parsed.precision = SitemapLastmodPrecision::Date;
    if b.get(pos) == Some(&b'T') {
        pos += 1;
        let hour = digits2(b, &mut pos)?;
        punctuation(b, &mut pos, b':')?;
        let minute = digits2(b, &mut pos)?;
        parsed.hour = Some(hour);
        parsed.minute = Some(minute);
        parsed.precision = SitemapLastmodPrecision::Minute;
        let mut second = 0;
        if b.get(pos) == Some(&b':') {
            pos += 1;
            second = digits2(b, &mut pos)?;
            parsed.second = Some(second);
            parsed.precision = SitemapLastmodPrecision::Second;
            if b.get(pos) == Some(&b'.') {
                pos += 1;
                let begin = pos;
                while b.get(pos).is_some_and(u8::is_ascii_digit) {
                    pos += 1;
                }
                if pos == begin {
                    return Err(Reason::LexicalGrammar);
                }
                parsed.fraction = Some(value[begin..pos].into());
                parsed.precision = SitemapLastmodPrecision::FractionalSecond;
            }
        } else if profile == Profile::Xsd {
            return Err(Reason::LexicalGrammar);
        }
        let zero_fraction = parsed
            .fraction
            .as_ref()
            .is_none_or(|fraction| fraction.bytes().all(|digit| digit == b'0'));
        let max_hour = if profile == Profile::Xsd { 24 } else { 23 };
        let max_second = if profile == Profile::Xsd { 60 } else { 59 };
        if hour > max_hour
            || minute > 59
            || second > max_second
            || (hour == 24 && (minute != 0 || second != 0 || !zero_fraction))
        {
            return Err(Reason::InvalidTime);
        }
        parsed.end_of_day = hour == 24;
        if profile == Profile::W3c && pos == b.len() {
            return Err(Reason::InvalidTimezone);
        }
    } else if profile == Profile::W3c && pos != b.len() {
        return Err(Reason::LexicalGrammar);
    }
    if pos < b.len() {
        let tz_start = pos;
        let offset = match b[pos] {
            b'Z' => {
                pos += 1;
                0
            }
            sign @ (b'+' | b'-') => {
                pos += 1;
                let hour = digits2(b, &mut pos).map_err(|_| Reason::InvalidTimezone)?;
                punctuation(b, &mut pos, b':').map_err(|_| Reason::InvalidTimezone)?;
                let minute = digits2(b, &mut pos).map_err(|_| Reason::InvalidTimezone)?;
                let max_hour = if profile == Profile::Xsd { 14 } else { 23 };
                if hour > max_hour
                    || minute > 59
                    || (profile == Profile::Xsd && hour == 14 && minute != 0)
                {
                    return Err(Reason::InvalidTimezone);
                }
                let minutes = i16::from(hour) * 60 + i16::from(minute);
                if sign == b'-' { -minutes } else { minutes }
            }
            _ => return Err(Reason::InvalidTimezone),
        };
        if pos != b.len() {
            return Err(Reason::InvalidTimezone);
        }
        parsed.timezone_lexeme = Some(value[tz_start..pos].into());
        parsed.timezone_offset_minutes = Some(offset);
    }
    Ok(parsed)
}
fn digits2(bytes: &[u8], pos: &mut usize) -> Result<u8, SitemapLastmodInvalidReason> {
    match bytes.get(*pos..pos.saturating_add(2)) {
        Some([first, second]) if first.is_ascii_digit() && second.is_ascii_digit() => {
            *pos += 2;
            Ok((first - b'0') * 10 + second - b'0')
        }
        _ => Err(SitemapLastmodInvalidReason::LexicalGrammar),
    }
}
fn punctuation(
    bytes: &[u8],
    pos: &mut usize,
    expected: u8,
) -> Result<(), SitemapLastmodInvalidReason> {
    if bytes.get(*pos) != Some(&expected) {
        return Err(SitemapLastmodInvalidReason::LexicalGrammar);
    }
    *pos += 1;
    Ok(())
}

#[cfg(test)]
#[path = "sitemap_lastmod_tests.rs"]
mod tests;
