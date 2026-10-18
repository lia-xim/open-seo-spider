//! Decode wide saved records in place on the heap, without growing thread stacks.
//! Serde's generated in-place visitors retain field defaults, duplicate-key
//! rejection and unknown-field behavior. No intermediate JSON Value is built.
use serde::{
    Deserialize, Deserializer,
    de::{DeserializeOwned, DeserializeSeed, SeqAccess, Visitor},
};
use std::{fmt, marker::PhantomData};

struct HeapSeed<T>(PhantomData<T>);
impl<'de, T: Deserialize<'de> + Default> DeserializeSeed<'de> for HeapSeed<T> {
    type Value = Box<T>;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let mut value = Box::<T>::default();
        T::deserialize_in_place(deserializer, &mut *value)?;
        Ok(value)
    }
}

pub(crate) fn value<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    HeapSeed::<T>(PhantomData)
        .deserialize(deserializer)
        .map(|value| *value)
}

pub(crate) fn pages<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<crate::PageRecord>, D::Error> {
    struct Pages;
    impl<'de> Visitor<'de> for Pages {
        type Value = Vec<crate::PageRecord>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a sequence of saved page records")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            let mut pages = Vec::new();
            // Vec::deserialize_in_place falls back to ordinary T::deserialize
            // for new entries. Use a seed for every new page instead.
            while let Some(page) =
                sequence.next_element_seed(HeapSeed::<crate::PageRecord>(PhantomData))?
            {
                pages.push(*page);
            }
            Ok(pages)
        }
    }
    deserializer.deserialize_seq(Pages)
}

pub(crate) fn from_str<T: DeserializeOwned + Default>(input: &str) -> serde_json::Result<T> {
    let mut decoder = serde_json::Deserializer::from_str(input);
    let value = HeapSeed::<T>(PhantomData).deserialize(&mut decoder)?;
    decoder.end()?;
    Ok(*value)
}

pub(crate) fn from_slice<T: DeserializeOwned + Default>(input: &[u8]) -> serde_json::Result<T> {
    let mut decoder = serde_json::Deserializer::from_slice(input);
    let value = HeapSeed::<T>(PhantomData).deserialize(&mut decoder)?;
    decoder.end()?;
    Ok(*value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn heap_saved_page_decoder_preserves_derived_json_fields_defaults_and_errors() {
        let full = serde_json::to_value(crate::PageRecord {
            url: "https://owned.test/雪".into(),
            titles: vec!["Owned \"quote\"".into()],
            ..Default::default()
        })
        .unwrap();
        for value in [
            json!({}),
            // Derived defaulted structs also accept positional sequences. Keep
            // their accepted shapes rather than imposing an object-only reader.
            json!([]),
            json!(["https://owned.test/positional"]),
            json!(["https://owned.test/positional", "https://owned.test/final"]),
            json!({"url":"https://owned.test/a","future_field":{"nested":[1,2,3]}}),
            full,
        ] {
            let bytes = serde_json::to_vec(&value).unwrap();
            let ordinary: crate::PageRecord = serde_json::from_slice(&bytes).unwrap();
            let heap: crate::PageRecord = from_slice(&bytes).unwrap();
            assert_eq!(
                serde_json::to_value(heap).unwrap(),
                serde_json::to_value(ordinary).unwrap()
            );
        }
        for input in [
            r#"{"url":1}"#,
            r#"[null]"#,
            r#"["owned",1]"#,
            r#"["owned"] true"#,
            r#"{"url":"first","url":"second"}"#,
            r#"{"url":"owned"} true"#,
        ] {
            assert!(serde_json::from_str::<crate::PageRecord>(input).is_err());
            assert!(from_str::<crate::PageRecord>(input).is_err());
        }
    }

    #[test]
    fn heap_report_sequence_preserves_full_fields_and_duplicate_rejection() {
        let mut report = crate::CrawlReport::default();
        report.pages.push(crate::PageRecord {
            url: "https://owned.test/a".into(),
            raw_metadata: Some(Default::default()),
            ..Default::default()
        });
        let bytes = serde_json::to_vec(&report).unwrap();
        let recovered: crate::CrawlReport = from_slice(&bytes).unwrap();
        assert_eq!(
            serde_json::to_value(recovered).unwrap(),
            serde_json::to_value(report).unwrap()
        );
        let mut invalid: Value = serde_json::from_slice(&bytes).unwrap();
        invalid["pages"][0]["size_bytes"] = json!("wrong type");
        assert!(from_slice::<crate::CrawlReport>(&serde_json::to_vec(&invalid).unwrap()).is_err());
        let valid: Value = serde_json::from_slice(&bytes).unwrap();
        let mut null_pages = valid.clone();
        null_pages["pages"] = Value::Null;
        let mut malformed = vec![null_pages];
        for required in ["config", "pages", "issues", "summary", "warnings"] {
            let mut missing = valid.clone();
            missing.as_object_mut().unwrap().remove(required);
            malformed.push(missing);
        }
        for value in malformed {
            let bytes = serde_json::to_vec(&value).unwrap();
            assert!(serde_json::from_slice::<crate::CrawlReport>(&bytes).is_err());
            assert!(from_slice::<crate::CrawlReport>(&bytes).is_err());
        }
        for page in [
            json!([]),
            json!(["https://owned.test/array"]),
            json!(["https://owned.test/array", "https://owned.test/final"]),
        ] {
            let mut positional = valid.clone();
            positional["pages"] = json!([page]);
            let bytes = serde_json::to_vec(&positional).unwrap();
            let ordinary: crate::CrawlReport = serde_json::from_slice(&bytes).unwrap();
            let heap: crate::CrawlReport = from_slice(&bytes).unwrap();
            assert_eq!(
                serde_json::to_value(heap).unwrap(),
                serde_json::to_value(ordinary).unwrap()
            );
        }
        let duplicate = format!(
            r#"{{"config":{{}},"pages":[{{"url":"first","url":"second"}}],"issues":[],"summary":{},"warnings":[]}}"#,
            serde_json::to_string(&crate::CrawlSummary::default()).unwrap()
        );
        assert!(from_str::<crate::CrawlReport>(&duplicate).is_err());
    }
}
