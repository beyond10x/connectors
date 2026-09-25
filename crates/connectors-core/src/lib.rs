//! Provider-independent operation and service wire contracts.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const WIRE_VERSION: &str = "v1alpha1";
pub const REQUEST_LIMIT: usize = 64 * 1024;
pub const RESPONSE_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidInput,
    Unsupported,
    Unauthorized,
    Forbidden,
    NotFound,
    StaleDescription,
    StaleCursor,
    RateLimited,
    Unavailable,
    Capacity,
    Timeout,
    UpstreamProtocol,
    Internal,
}

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
#[error("{code:?}: {message}")]
#[serde(deny_unknown_fields)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub retry_after_seconds: Option<u64>,
}

impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retry_after_seconds: None,
        }
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }
    pub fn unavailable() -> Self {
        Self::new(ErrorCode::Unavailable, "dependency unavailable")
    }
    pub fn internal() -> Self {
        Self::new(ErrorCode::Internal, "internal operation failure")
    }
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub id: String,
    pub description: String,
    pub contract: String,
    pub profile: String,
    pub input_schema: Value,
    pub output_schema: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub version: String,
    pub instance: String,
    pub adapter: String,
    pub revision: String,
    pub operations: Vec<Operation>,
    pub configuration_schema: Value,
}

impl Descriptor {
    pub fn operation(&self, id: &str) -> Result<&Operation> {
        self.operations
            .iter()
            .find(|o| o.id == id)
            .ok_or_else(|| Error::new(ErrorCode::NotFound, "operation is not provided"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    pub version: String,
    pub request_id: String,
    pub operation: String,
    pub revision: String,
    pub input: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    Success { result: Value },
    Error { error: Error },
}

#[derive(Clone, Debug, Serialize)]
pub struct Response {
    pub version: String,
    pub request_id: String,
    #[serde(flatten)]
    pub outcome: Outcome,
}

impl<'de> Deserialize<'de> for Response {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
        enum Wire {
            Success {
                version: String,
                request_id: String,
                result: Value,
            },
            Error {
                version: String,
                request_id: String,
                error: Error,
            },
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Success {
                version,
                request_id,
                result,
            } => Self {
                version,
                request_id,
                outcome: Outcome::Success { result },
            },
            Wire::Error {
                version,
                request_id,
                error,
            } => Self {
                version,
                request_id,
                outcome: Outcome::Error { error },
            },
        })
    }
}

/// Keys serde_json's own `Value` reader reinterprets as a number or raw JSON
/// under its `arbitrary_precision` and `raw_value` features.
const NUMBER_TOKEN: &str = "$serde_json::private::Number";
const PRIVATE_TOKENS: [&str; 2] = [NUMBER_TOKEN, "$serde_json::private::RawValue"];

/// Read a JSON number's digits the way the default serde_json build does: an
/// integer outside `i64`/`u64` or any fraction/exponent is the nearest `f64`.
fn parsed_number<E: serde::de::Error>(digits: &str) -> std::result::Result<Value, E> {
    digits
        .parse::<f64>()
        .ok()
        .and_then(serde_json::Number::from_f64)
        .map(Value::Number)
        .ok_or_else(|| E::custom("number out of range"))
}

/// The value under a private token: `Some` only when handed over owned.
struct TokenValue;
impl<'de> serde::de::DeserializeSeed<'de> for TokenValue {
    type Value = Option<String>;
    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Option<String>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("serde_json number digits")
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Self::Value, E> {
                Ok(Some(v))
            }
            fn visit_str<E: serde::de::Error>(
                self,
                _: &str,
            ) -> std::result::Result<Self::Value, E> {
                Ok(None)
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

/// Reject ambiguous duplicate keys before projecting JSON into typed envelopes.
/// Numbers read identically with and without serde_json `arbitrary_precision`,
/// and a document key spelled like a serde_json private token is refused.
pub fn read_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    struct Strict(Value);
    impl<'de> Deserialize<'de> for Strict {
        fn deserialize<D: serde::Deserializer<'de>>(
            deserializer: D,
        ) -> std::result::Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Strict;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("unambiguous JSON")
                }
                fn visit_bool<E: serde::de::Error>(
                    self,
                    v: bool,
                ) -> std::result::Result<Strict, E> {
                    Ok(Strict(Value::Bool(v)))
                }
                fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Strict, E> {
                    // Only `-0` reaches here as a signed zero: the default
                    // serde_json build reads it as the float -0.0, while
                    // `arbitrary_precision` reads it as the integer 0.
                    if v == 0 {
                        return self.visit_f64(-0.0);
                    }
                    Ok(Strict(v.into()))
                }
                fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Strict, E> {
                    Ok(Strict(v.into()))
                }
                fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Strict, E> {
                    serde_json::Number::from_f64(v)
                        .map(|n| Strict(Value::Number(n)))
                        .ok_or_else(|| E::custom("nonfinite JSON number"))
                }
                fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Strict, E> {
                    Ok(Strict(Value::String(v.into())))
                }
                fn visit_string<E: serde::de::Error>(
                    self,
                    v: String,
                ) -> std::result::Result<Strict, E> {
                    Ok(Strict(Value::String(v)))
                }
                fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Strict, E> {
                    Ok(Strict(Value::Null))
                }
                fn visit_seq<A: serde::de::SeqAccess<'de>>(
                    self,
                    mut seq: A,
                ) -> std::result::Result<Strict, A::Error> {
                    let mut values = Vec::new();
                    while let Some(Strict(v)) = seq.next_element()? {
                        values.push(v);
                    }
                    Ok(Strict(Value::Array(values)))
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> std::result::Result<Strict, A::Error> {
                    let mut values = serde_json::Map::new();
                    while let Some(k) = map.next_key::<String>()? {
                        if PRIVATE_TOKENS.contains(&k.as_str()) {
                            // With `arbitrary_precision`, serde_json hands a
                            // non-integer number over as a one-entry map under
                            // its number token, the digits as an owned string.
                            // `from_slice` never hands a document string over
                            // owned, so an owned value here is that number and
                            // anything else is a document spelling the token.
                            let digits = map.next_value_seed(TokenValue)?;
                            if values.is_empty()
                                && k == NUMBER_TOKEN
                                && let Some(digits) = digits
                                && map.next_key::<serde::de::IgnoredAny>()?.is_none()
                            {
                                return parsed_number(&digits).map(Strict);
                            }
                            return Err(serde::de::Error::custom("reserved serde_json key"));
                        }
                        let Strict(v) = map.next_value()?;
                        if values.insert(k, v).is_some() {
                            return Err(serde::de::Error::custom("duplicate JSON key"));
                        }
                    }
                    Ok(Strict(Value::Object(values)))
                }
            }
            deserializer.deserialize_any(Visitor)
        }
    }
    let Strict(value) =
        serde_json::from_slice(bytes).map_err(|_| Error::invalid("invalid or ambiguous JSON"))?;
    serde_json::from_value(value)
        .map_err(|_| Error::invalid("JSON does not match the required type"))
}

/// Explicit recursive canonicalization: independent of serde_json map features.
pub fn canonical(value: &Value) -> Vec<u8> {
    fn ordered(value: &Value) -> Value {
        match value {
            Value::Object(values) => {
                let sorted: BTreeMap<_, _> = values
                    .iter()
                    .map(|(k, v)| (k.clone(), ordered(v)))
                    .collect();
                Value::Object(sorted.into_iter().collect())
            }
            Value::Array(values) => Value::Array(values.iter().map(ordered).collect()),
            // `arbitrary_precision` keeps a parsed number's source spelling
            // (`1.50`, `1e2`); write every non-integer as the default build
            // does, the nearest `f64` in its shortest form.
            Value::Number(number) if !(number.is_u64() || number.is_i64()) => number
                .as_f64()
                .and_then(serde_json::Number::from_f64)
                .map_or_else(|| value.clone(), Value::Number),
            other => other.clone(),
        }
    }
    serde_json::to_vec(&ordered(value)).expect("JSON Value is serializable")
}

pub fn digest(value: &Value) -> String {
    hex::encode(Sha256::digest(canonical(value)))
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
}
