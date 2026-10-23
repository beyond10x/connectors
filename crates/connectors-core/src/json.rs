//! Lossless JSON values with explicit depth and duplicate-key checks.
//! The enclosing frame reader owns the byte ceiling. Literal objects are decoded
//! separately from scalar numbers so serde's internal number tokens cannot turn
//! an operation-local object into a number. No legacy core codec is changed.
use serde::{
    Deserialize,
    de::{MapAccess, Visitor},
};
use serde_json::{Map, Value, value::RawValue};
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimit,
    Malformed,
    DuplicateKey,
    Depth,
}

/// Decode one complete document. A depth counts containers, not scalar leaves;
/// 1 permits a root object/array of scalar values. The maximum allowed limit is
/// 128, keeping this recursive reconstruction within a fixed implementation bound.
/// Numbers retain decimal precision; this is not a byte-canonicalization function.
pub fn decode(bytes: &[u8], maximum_depth: usize) -> Result<Value, Error> {
    if !(1..=128).contains(&maximum_depth) {
        return Err(Error::InvalidLimit);
    }
    let raw: &RawValue = serde_json::from_slice(bytes).map_err(|_| Error::Malformed)?;
    value(raw, maximum_depth)
}

fn value(raw: &RawValue, remaining: usize) -> Result<Value, Error> {
    let text = raw.get();
    match text.as_bytes().first() {
        Some(b'{') => {
            let remaining = remaining.checked_sub(1).ok_or(Error::Depth)?;
            let Fields(fields) = serde_json::from_str(text).map_err(|_| Error::Malformed)?;
            let mut result = Map::new();
            for (key, raw) in fields {
                if result.contains_key(&key) {
                    return Err(Error::DuplicateKey);
                }
                result.insert(key, value(raw, remaining)?);
            }
            Ok(Value::Object(result))
        }
        Some(b'[') => {
            let remaining = remaining.checked_sub(1).ok_or(Error::Depth)?;
            let children: Vec<&RawValue> =
                serde_json::from_str(text).map_err(|_| Error::Malformed)?;
            children
                .into_iter()
                .map(|raw| value(raw, remaining))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        _ => serde_json::from_str(text).map_err(|_| Error::Malformed),
    }
}

struct Fields<'a>(Vec<(String, &'a RawValue)>);
impl<'de> Deserialize<'de> for Fields<'de> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FieldsVisitor;
        impl<'de> Visitor<'de> for FieldsVisitor {
            type Value = Fields<'de>;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a literal JSON object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut fields = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    fields.push((key, map.next_value::<&RawValue>()?));
                }
                Ok(Fields(fields))
            }
        }
        deserializer.deserialize_map(FieldsVisitor)
    }
}
