//! Compare the actual saved report's source envelope with verified same-TX SQL.
//! No report/source Vec is built. SQLite's report/JSON projection storage and
//! serde scratch remain outside a measured-RSS claim. No DDL or writes.
use super::{verified_read::VerifiedSourceRead, *};
use crate::sitemap_read_control::SitemapReadControl;
use serde::de::{self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use std::fmt;

const MAX_REPORT: usize = 268_435_456;
const MAX_ENVELOPE: i64 = 67_108_864;
const MAX_REPORT_KEY_BYTES: usize = 1024;
// One decoded UTF-8 byte can require at most six JSON content bytes (\u00XX).
// Bound serde scratch BEFORE MapKey decodes an escaped identifier/suffix.
const MAX_REPORT_KEY_ENCODED_BYTES: usize = 6 * MAX_REPORT_KEY_BYTES;

/// `None` is historical unknown, not a synthetic empty measured envelope.
pub(crate) fn verify_saved_report(
    connection: &Connection,
    expected: Option<&VerifiedSourceRead<'_>>,
) -> Result<()> {
    verify_saved_report_with_control(connection, expected, &SitemapReadControl::default())
}
pub(crate) fn verify_saved_report_with_control(
    connection: &Connection,
    expected: Option<&VerifiedSourceRead<'_>>,
    control: &SitemapReadControl,
) -> Result<()> {
    let _binding = control.bind_connection(connection)?;
    control.run(|| verify_saved_report_inner(connection, expected, control))
}
fn verify_saved_report_inner(
    connection: &Connection,
    expected: Option<&VerifiedSourceRead<'_>>,
    control: &SitemapReadControl,
) -> Result<()> {
    ensure!(
        !connection.is_autocommit(),
        "Source report verification requires a read transaction"
    );
    if let Some(proof) = expected {
        ensure!(
            std::ptr::eq(connection, proof.connection()),
            "Source report proof belongs to another connection"
        );
    }
    let (kind, bytes): (Option<String>, Option<i64>) = connection.query_row(
        "SELECT json_type(value,'$.sitemap_source_measurements'),CASE WHEN json_type(value,'$.sitemap_source_measurements')='object' THEN length(CAST(json_extract(value,'$.sitemap_source_measurements') AS BLOB)) END FROM metadata WHERE key='report'",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if let Some(proof) = expected {
        ensure!(
            kind.as_deref() == Some("object")
                && bytes.is_some_and(|n| (1..=MAX_ENVELOPE).contains(&n)),
            "Sitemap analysis saved source measurement envelope differs or exceeds read budget"
        );
        let mut statement = connection.prepare("SELECT CAST(json_extract(value,'$.sitemap_source_measurements') AS BLOB) FROM metadata WHERE key='report'")?;
        let mut rows = statement.query([])?;
        let row = rows.next()?.context("Saved report missing")?;
        let slice = row.get_ref(0)?.as_bytes()?;
        ensure!(
            Some(i64::try_from(slice.len())?) == bytes,
            "Saved source envelope byte length differs"
        );
        preflight(slice, proof, control)?;
    } else {
        ensure!(
            kind.is_none() || kind.as_deref() == Some("null"),
            "Sitemap analysis saved source measurements have no verified SQL basis"
        );
    }
    let mut statement = connection.prepare("SELECT value FROM metadata WHERE key='report'")?;
    let mut rows = statement.query([])?;
    let row = rows.next()?.context("Saved report missing")?;
    let bytes = row.get_ref(0)?.as_bytes()?;
    ensure!(
        (1..=MAX_REPORT).contains(&bytes.len()),
        "Sitemap analysis report input budget exceeded"
    );
    preflight_report_keys(bytes, control)?;
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    ReportSeed { expected, control }
        .deserialize(&mut decoder)
        .context("Sitemap analysis saved source measurement projection is invalid")?;
    decoder.end()?;
    Ok(())
}

// Field visitors classify borrowed identifiers. Unknown source keys fail with a
// constant error; no attacker-controlled field name is cloned into the error.
enum ReportField {
    Source,
    Other,
}
impl<'de> Deserialize<'de> for ReportField {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct Field;
        impl Visitor<'_> for Field {
            type Value = ReportField;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("report field")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                if v.len() > MAX_REPORT_KEY_BYTES {
                    return Err(E::custom("Oversized saved report field name"));
                }
                Ok(if v == "sitemap_source_measurements" {
                    ReportField::Source
                } else {
                    ReportField::Other
                })
            }
        }
        d.deserialize_identifier(Field)
    }
}
/// Borrow-only resource pass over actual outer report keys. It neither creates
/// strings nor recursively materializes values. Full syntax, duplicate/source
/// semantics and the complete end remain checked by the serde seed below.
/// A 6,144-byte unescaped key can pass this raw cap; the decoded 1,024-byte
/// visitor then rejects it with bounded scratch. Maximally escaped valid keys
/// remain accepted at the exact raw bound.
fn preflight_report_keys(bytes: &[u8], control: &SitemapReadControl) -> Result<()> {
    control.checkpoint()?;
    let mut pos = 0;
    byte(bytes, &mut pos, b'{', control)?;
    loop {
        ws(bytes, &mut pos, control)?;
        if bytes.get(pos) == Some(&b'}') {
            pos += 1;
            break;
        }
        byte(bytes, &mut pos, b'"', control)?;
        let content_start = pos;
        let mut escaped = false;
        loop {
            let value = *bytes.get(pos).context("Unterminated report outer key")?;
            if value == b'"' && !escaped {
                pos += 1;
                break;
            }
            pos += 1;
            ensure!(
                pos - content_start <= MAX_REPORT_KEY_ENCODED_BYTES,
                "Saved report outer key encoded byte budget exceeded"
            );
            if escaped {
                escaped = false;
            } else if value == b'\\' {
                escaped = true;
            }
        }
        byte(bytes, &mut pos, b':', control)?;
        skip_report_value(bytes, &mut pos, control)?;
        ws(bytes, &mut pos, control)?;
        if bytes.get(pos) == Some(&b'}') {
            pos += 1;
            break;
        }
        byte(bytes, &mut pos, b',', control)?;
    }
    ws(bytes, &mut pos, control)?;
    ensure!(pos == bytes.len(), "Trailing report input");
    Ok(())
}

/// Skip one outer value with scalar counters only. Nested keys/string contents
/// are not outer identifiers and do not receive the outer-key cap. This is a
/// lexical resource guard, not a replacement JSON parser; serde is required.
fn skip_report_value(bytes: &[u8], pos: &mut usize, control: &SitemapReadControl) -> Result<()> {
    control.checkpoint()?;
    ws(bytes, pos, control)?;
    let start = *pos;
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    while let Some(&value) = bytes.get(*pos) {
        if *pos & 65_535 == 0 {
            control.checkpoint()?;
        }
        if !quoted && depth == 0 && matches!(value, b',' | b'}') {
            break;
        }
        *pos += 1;
        if quoted {
            if escaped {
                escaped = false;
            } else if value == b'\\' {
                escaped = true;
            } else if value == b'"' {
                quoted = false;
            }
        } else {
            match value {
                b'"' => quoted = true,
                b'{' | b'[' => depth = depth.checked_add(1).context("Report nesting overflow")?,
                b'}' | b']' => depth = depth.checked_sub(1).context("Malformed report value")?,
                _ => {}
            }
        }
    }
    ensure!(
        *pos > start && !quoted && depth == 0,
        "Malformed report value"
    );
    Ok(())
}

enum SourceField {
    Version,
    Summary,
    Records,
    Commits,
}
impl<'de> Deserialize<'de> for SourceField {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct Field;
        impl Visitor<'_> for Field {
            type Value = SourceField;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("source snapshot field")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                match v {
                    "schema_version" => Ok(SourceField::Version),
                    "summary" => Ok(SourceField::Summary),
                    "records" => Ok(SourceField::Records),
                    "commits" => Ok(SourceField::Commits),
                    _ => Err(E::custom("Unknown source snapshot field")),
                }
            }
        }
        d.deserialize_identifier(Field)
    }
}
struct ReportSeed<'a, 'conn> {
    expected: Option<&'a VerifiedSourceRead<'conn>>,
    control: &'a SitemapReadControl,
}
impl<'de> DeserializeSeed<'de> for ReportSeed<'_, '_> {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> std::result::Result<(), D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de> Visitor<'de> for ReportSeed<'_, '_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("saved report object")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<(), A::Error> {
        let mut seen = false;
        while let Some(key) = map.next_key::<ReportField>()? {
            self.control
                .checkpoint()
                .map_err(<A::Error as de::Error>::custom)?;
            match key {
                ReportField::Source => {
                    if seen {
                        return Err(de::Error::custom(
                            "Duplicate saved source measurement field",
                        ));
                    }
                    seen = true;
                    map.next_value_seed(SourceSeed {
                        expected: self.expected,
                        control: self.control,
                    })?;
                }
                ReportField::Other => {
                    map.next_value::<IgnoredAny>()?;
                    self.control
                        .checkpoint()
                        .map_err(<A::Error as de::Error>::custom)?;
                }
            }
        }
        if self.expected.is_some() && !seen {
            return Err(de::Error::custom("Missing saved source measurements"));
        }
        Ok(())
    }
}
struct SourceSeed<'a, 'conn> {
    expected: Option<&'a VerifiedSourceRead<'conn>>,
    control: &'a SitemapReadControl,
}
impl<'de> DeserializeSeed<'de> for SourceSeed<'_, '_> {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> std::result::Result<(), D::Error> {
        d.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for SourceSeed<'_, '_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("exact source snapshot or historical null")
    }
    fn visit_unit<E: de::Error>(self) -> std::result::Result<(), E> {
        if self.expected.is_some() {
            Err(E::custom("Missing saved source measurements"))
        } else {
            Ok(())
        }
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<(), A::Error> {
        let expected = self.expected.ok_or_else(|| {
            <A::Error as de::Error>::custom("Source envelope has no verified SQL basis")
        })?;
        let mut seen = [false; 4];
        while let Some(key) = map.next_key::<SourceField>()? {
            self.control
                .checkpoint()
                .map_err(<A::Error as de::Error>::custom)?;
            let index = match key {
                SourceField::Version => 0,
                SourceField::Summary => 1,
                SourceField::Records => 2,
                SourceField::Commits => 3,
            };
            if std::mem::replace(&mut seen[index], true) {
                return Err(de::Error::custom("Duplicate source snapshot field"));
            }
            match key {
                SourceField::Version => {
                    if map.next_value::<u32>()? != 1 {
                        return Err(de::Error::custom("Unknown source snapshot version"));
                    }
                }
                SourceField::Summary => {
                    let saved = map.next_value::<SitemapSourceStoreSummary>()?;
                    if &saved != expected.summary() {
                        return Err(de::Error::custom(
                            "Saved source summary differs from verified SQL",
                        ));
                    }
                }
                SourceField::Records => map.next_value_seed(ArraySeed {
                    expected,
                    control: self.control,
                    records: true,
                })?,
                SourceField::Commits => map.next_value_seed(ArraySeed {
                    expected,
                    control: self.control,
                    records: false,
                })?,
            }
        }
        if !seen.into_iter().all(|v| v) {
            return Err(de::Error::custom("Incomplete source snapshot envelope"));
        }
        Ok(())
    }
}
struct ArraySeed<'a, 'conn> {
    expected: &'a VerifiedSourceRead<'conn>,
    control: &'a SitemapReadControl,
    records: bool,
}
impl<'de> DeserializeSeed<'de> for ArraySeed<'_, '_> {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> std::result::Result<(), D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de> Visitor<'de> for ArraySeed<'_, '_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ordered SQL-equivalent source array")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<(), A::Error> {
        let sql = if self.records {
            "SELECT sequence,measurement_id,source_kind,task_url,generation,source_revision,expected_document_id,capture_config_sha256,initial_catalog_config_sha256,data,data_sha256 FROM sitemap_source_measurements ORDER BY sequence"
        } else {
            "SELECT document_id,measurement_id,document_sequence,data,data_sha256 FROM sitemap_source_measurement_commits ORDER BY document_sequence"
        };
        // Restore typed SQLite stops BEFORE serde custom-error conversion
        // would erase the rusqlite error's type/source chain.
        let mut statement = self
            .control
            .map_result(self.expected.connection().prepare(sql).map_err(Into::into))
            .map_err(<A::Error as de::Error>::custom)?;
        let mut rows = self
            .control
            .map_result(statement.query([]).map_err(Into::into))
            .map_err(<A::Error as de::Error>::custom)?;
        while let Some(row) = self
            .control
            .map_result(rows.next().map_err(Into::into))
            .map_err(<A::Error as de::Error>::custom)?
        {
            self.control
                .checkpoint()
                .map_err(<A::Error as de::Error>::custom)?;
            if self.records {
                let expected = read_record_row(row, 0, &self.expected.summary().limits)
                    .map_err(<A::Error as de::Error>::custom)?;
                let saved = seq
                    .next_element::<SitemapSourceStoredMeasurement>()?
                    .ok_or_else(|| {
                        <A::Error as de::Error>::custom("Saved source record missing")
                    })?;
                if saved != expected {
                    return Err(de::Error::custom(
                        "Saved source record differs from verified SQL",
                    ));
                }
            } else {
                let expected = read_commit_row(row, 0).map_err(<A::Error as de::Error>::custom)?;
                let saved = seq
                    .next_element::<SitemapSourceCommitAssociation>()?
                    .ok_or_else(|| {
                        <A::Error as de::Error>::custom("Saved source commit missing")
                    })?;
                if saved != expected {
                    return Err(de::Error::custom(
                        "Saved source commit differs from verified SQL",
                    ));
                }
            }
        }
        if seq.next_element::<IgnoredAny>()?.is_some() {
            return Err(de::Error::custom("Saved source array has extra rows"));
        }
        Ok(())
    }
}

// JSON1 has validated the envelope; this borrowed lexical pass limits each
// typed value BEFORE serde can allocate its fields. Count every byte in strings
// (including whitespace/escapes), ignoring only JSON whitespace outside strings.
// Six bytes per canonical byte permits equivalent Unicode escapes and pretty
// printing without silently raising the durable SQL limits.
fn span(bytes: &[u8], pos: &mut usize, cap: usize, control: &SitemapReadControl) -> Result<()> {
    control.checkpoint()?;
    ws(bytes, pos, control)?;
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    let mut string_start = 0usize;
    let mut charged = 0usize;
    while let Some(&byte) = bytes.get(*pos) {
        if *pos & 65_535 == 0 {
            control.checkpoint()?;
        }
        if !quoted && depth == 0 && matches!(byte, b',' | b']' | b'}') {
            break;
        }
        if quoted || !byte.is_ascii_whitespace() {
            charged = charged
                .checked_add(1)
                .context("Source report span overflow")?;
            ensure!(
                charged <= cap,
                "Saved source typed value exceeds read budget"
            );
        }
        *pos += 1;
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
                let mut next = *pos;
                ws(bytes, &mut next, control)?;
                ensure!(
                    bytes.get(next) != Some(&b':') || *pos - string_start <= 1024,
                    "Oversized source field name"
                );
            }
        } else {
            match byte {
                b'"' => {
                    quoted = true;
                    string_start = *pos - 1;
                }
                b'{' | b'[' => {
                    depth += 1;
                    ensure!(
                        depth <= 128,
                        "Saved source JSON nesting exceeds read budget"
                    );
                }
                b'}' | b']' => {
                    ensure!(depth > 0, "Malformed source value");
                    depth -= 1;
                }
                _ => {}
            }
        }
    }
    ensure!(
        !quoted && depth == 0 && charged > 0,
        "Malformed source value"
    );
    Ok(())
}
fn ws(bytes: &[u8], pos: &mut usize, control: &SitemapReadControl) -> Result<()> {
    control.checkpoint()?;
    while bytes.get(*pos).is_some_and(u8::is_ascii_whitespace) {
        *pos += 1;
        if *pos & 65_535 == 0 {
            control.checkpoint()?;
        }
    }
    Ok(())
}
fn byte(bytes: &[u8], pos: &mut usize, expected: u8, control: &SitemapReadControl) -> Result<()> {
    ws(bytes, pos, control)?;
    ensure!(
        bytes.get(*pos) == Some(&expected),
        "Malformed source envelope"
    );
    *pos += 1;
    Ok(())
}
fn preflight(
    bytes: &[u8],
    proof: &VerifiedSourceRead<'_>,
    control: &SitemapReadControl,
) -> Result<()> {
    control.checkpoint()?;
    let mut pos = 0;
    byte(bytes, &mut pos, b'{', control)?;
    loop {
        ws(bytes, &mut pos, control)?;
        if bytes.get(pos) == Some(&b'}') {
            pos += 1;
            break;
        }
        let start = pos;
        // A JSON field name ends before its colon. Its syntax is checked by
        // serde below; no unknown/duplicate key is silently accepted.
        ensure!(bytes.get(pos) == Some(&b'"'), "Malformed source field");
        pos += 1;
        let mut escaped = false;
        while let Some(&b) = bytes.get(pos) {
            pos += 1;
            ensure!(pos - start <= 1024, "Oversized source field name");
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                break;
            }
        }
        let key: String = serde_json::from_slice(&bytes[start..pos])?;
        byte(bytes, &mut pos, b':', control)?;
        match key.as_str() {
            "records" | "commits" => {
                byte(bytes, &mut pos, b'[', control)?;
                let records = key == "records";
                let max = if records {
                    proof.summary().record_count
                } else {
                    proof.summary().commit_count
                };
                let cap = if records {
                    proof.summary().limits.max_record_json_bytes
                } else {
                    1024
                };
                let mut count = 0u64;
                loop {
                    ws(bytes, &mut pos, control)?;
                    if bytes.get(pos) == Some(&b']') {
                        pos += 1;
                        break;
                    }
                    count += 1;
                    ensure!(count <= max, "Saved source array has extra rows");
                    span(bytes, &mut pos, cap * 6 + 1024, control)?;
                    ws(bytes, &mut pos, control)?;
                    if bytes.get(pos) == Some(&b']') {
                        pos += 1;
                        break;
                    }
                    byte(bytes, &mut pos, b',', control)?;
                }
            }
            "summary" => span(bytes, &mut pos, 4096 * 6 + 1024, control)?,
            "schema_version" => span(bytes, &mut pos, 64, control)?,
            _ => bail!("Unknown source snapshot field"),
        }
        ws(bytes, &mut pos, control)?;
        if bytes.get(pos) == Some(&b'}') {
            pos += 1;
            break;
        }
        byte(bytes, &mut pos, b',', control)?;
    }
    ws(bytes, &mut pos, control)?;
    ensure!(pos == bytes.len(), "Trailing source envelope input");
    Ok(())
}
