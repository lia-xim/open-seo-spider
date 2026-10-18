//! Strictness belongs to the portable boundary, not the legacy crawl DTO.
use super::{CONFIG_PROFILE_MAX_DEPTH, CONFIG_PROFILE_MAX_NODES};
use serde::{
    Deserialize,
    de::{
        self, DeserializeOwned, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor,
    },
};
use serde_json::{Map, Number, Value};
use std::fmt;

pub fn parse_strict_json<T: DeserializeOwned>(bytes: &[u8], max_bytes: usize) -> anyhow::Result<T> {
    let value = parse_value(bytes, max_bytes)?;
    Ok(T::deserialize(StrictValue(value))?)
}

pub(super) fn parse_value(bytes: &[u8], max_bytes: usize) -> anyhow::Result<Value> {
    anyhow::ensure!(
        !bytes.is_empty() && bytes.len() <= max_bytes,
        "Configuration JSON exceeds its byte limit or is empty"
    );
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let mut remaining = CONFIG_PROFILE_MAX_NODES;
    let value = Node {
        depth: 0,
        remaining: &mut remaining,
    }
    .deserialize(&mut decoder)?;
    decoder.end()?;
    Ok(value)
}

pub(super) fn validate_crawl_tagged_units(config: &Value) -> anyhow::Result<()> {
    // Serde's internally tagged enum decoder buffers Content before decoding
    // its variant. InternallyTaggedUnitVisitor then ignores leftover fields,
    // even with deny_unknown_fields, bypassing StrictValue::ignored_any below.
    // Audit new internally tagged unit variants reachable from CrawlConfig here.
    // These are the three current cases; keep unrelated free JSON maps intact.
    fn only_tag(value: Option<&Value>, units: &[&str]) -> anyhow::Result<()> {
        if let Some(fields) = value.and_then(Value::as_object)
            && fields
                .get("kind")
                .and_then(Value::as_str)
                .is_some_and(|kind| units.contains(&kind))
        {
            anyhow::ensure!(
                fields.len() == 1,
                "Unknown property in a unit configuration variant"
            );
        }
        Ok(())
    }

    if let Some(imports) = config.get("input_imports").and_then(Value::as_array) {
        for import in imports {
            only_tag(import.pointer("/options/format"), &["txt", "xml"])?;
            if let Some(rows) = import.get("rows").and_then(Value::as_array) {
                for row in rows {
                    only_tag(row.get("formula_metadata"), &["unsupported"])?;
                }
            }
        }
    }
    Ok(())
}

struct Node<'a> {
    depth: usize,
    remaining: &'a mut usize,
}
impl<'de> DeserializeSeed<'de> for Node<'_> {
    type Value = Value;
    fn deserialize<D: de::Deserializer<'de>>(self, decoder: D) -> Result<Value, D::Error> {
        if self.depth > CONFIG_PROFILE_MAX_DEPTH {
            return Err(de::Error::custom("Configuration JSON exceeds depth64"));
        }
        if *self.remaining == 0 {
            return Err(de::Error::custom("Configuration JSON exceeds node limit"));
        }
        *self.remaining -= 1;
        decoder.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Node<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded configuration JSON")
    }
    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_none<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| de::Error::custom("Non-finite configuration number"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.into()))
    }
    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element_seed(Node {
            depth: self.depth + 1,
            remaining: &mut *self.remaining,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("Duplicate configuration JSON property"));
            }
            if *self.remaining == 0 {
                return Err(de::Error::custom("Configuration JSON exceeds node limit"));
            }
            *self.remaining -= 1;
            let value = map.next_value_seed(Node {
                depth: self.depth + 1,
                remaining: &mut *self.remaining,
            })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

/// Existing typed Serde visitors supply the field names. Recursing through
/// strict values rejects ignored fields and positional struct arrays without
/// maintaining a second handwritten CrawlConfig schema.
pub(super) struct StrictValue(pub Value);
impl<'de> IntoDeserializer<'de, serde_json::Error> for StrictValue {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self {
        self
    }
}
impl<'de> de::Deserializer<'de> for StrictValue {
    type Error = serde_json::Error;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::Null => visitor.visit_unit(),
            Value::Bool(value) => visitor.visit_bool(value),
            Value::String(value) => visitor.visit_string(value),
            Value::Number(value) => {
                de::Deserializer::deserialize_any(Value::Number(value), visitor)
            }
            Value::Array(values) => {
                visitor.visit_seq(de::value::SeqDeserializer::<_, Self::Error>::new(
                    values.into_iter().map(StrictValue),
                ))
            }
            Value::Object(values) => {
                visitor.visit_map(de::value::MapDeserializer::<_, Self::Error>::new(
                    values
                        .into_iter()
                        .map(|(key, value)| (key, StrictValue(value))),
                ))
            }
        }
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.0.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if !self.0.is_object() {
            return Err(de::Error::custom(
                "Configuration structures require JSON objects",
            ));
        }
        self.deserialize_any(visitor)
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if !self.0.is_object() {
            return Err(de::Error::custom("Configuration maps require JSON objects"));
        }
        self.deserialize_any(visitor)
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(de::Error::custom("Unknown configuration property"))
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.0 {
            Value::String(name) => {
                visitor.visit_enum(de::value::StringDeserializer::<Self::Error>::new(name))
            }
            Value::Object(values) if values.len() == 1 => {
                let (name, value) = values.into_iter().next().unwrap();
                visitor.visit_enum(EnumValue { name, value })
            }
            _ => Err(de::Error::custom("Invalid configuration enum")),
        }
    }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf unit unit_struct seq tuple tuple_struct identifier }
}
struct EnumValue {
    name: String,
    value: Value,
}
impl<'de> de::EnumAccess<'de> for EnumValue {
    type Error = serde_json::Error;
    type Variant = Self;
    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self), Self::Error> {
        let name = seed.deserialize(de::value::StringDeserializer::<Self::Error>::new(
            self.name.clone(),
        ))?;
        Ok((name, self))
    }
}
impl<'de> de::VariantAccess<'de> for EnumValue {
    type Error = serde_json::Error;
    fn unit_variant(self) -> Result<(), Self::Error> {
        <()>::deserialize(StrictValue(self.value))
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        seed.deserialize(StrictValue(self.value))
    }
    fn tuple_variant<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error> {
        de::Deserializer::deserialize_seq(StrictValue(self.value), visitor)
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        de::Deserializer::deserialize_struct(StrictValue(self.value), "variant", fields, visitor)
    }
}
