//! Configured read profiles and `profile.read`.
//!
//! A read profile is the operator's: a single parameterised read statement, the names of its
//! parameters in placeholder order, its output columns with declared types and masking, the
//! column that carries the call identifier and the column the keyset cursor follows. The
//! repository holds only this shape and its validation; `connectors_sql.profiles` in the
//! adapter's ESS model fixes both, and the executable admits a profile exactly when the model
//! does.
//!
//! A page runs the statement through the `query.read` path (the same read-only session,
//! deadlines and byte bounds) inside a derived table ordered by the ordering column, after
//! the cursor when there is one, asking for one row past the limit. A server, or a proxy in
//! front of it, may return fewer rows than any LIMIT asks for, so a short page proves
//! nothing: every page that received a row carries a cursor after the last row it returns,
//! and only a page that received none completes the read.
use base64::Engine as _;
use connectors_core::{Error, ErrorCode, Operation, Result};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// The operation that serves read profiles.
pub const OPERATION: &str = "profile.read";
/// At most this many profiles per connection.
pub const MAX_PROFILES: usize = 32;
/// A profile's statement takes at most this many parameters: the cursor is bound after them,
/// within the 64 a read binds.
pub const MAX_PARAMETERS: usize = 63;
/// The longest ordering value a cursor carries, in bytes.
const MAX_CURSOR_VALUE: usize = 8192;

/// The declared type of an output column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    Text,
    Integer,
    Timestamp,
    PhoneNumber,
}
impl ColumnType {
    pub const ALL: [ColumnType; 4] = [
        ColumnType::Text,
        ColumnType::Integer,
        ColumnType::Timestamp,
        ColumnType::PhoneNumber,
    ];
    /// The model's name for the type (`connectors_sql.profiles.ColumnType`).
    pub fn name(self) -> &'static str {
        match self {
            ColumnType::Text => "text",
            ColumnType::Integer => "integer",
            ColumnType::Timestamp => "timestamp",
            ColumnType::PhoneNumber => "phone_number",
        }
    }
    /// Whether a non-null native text cell is a value of this type.
    fn admits(self, value: &str) -> bool {
        match self {
            ColumnType::Text | ColumnType::PhoneNumber => true,
            ColumnType::Integer => {
                let digits = value.strip_prefix('-').unwrap_or(value);
                !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
            }
            ColumnType::Timestamp => {
                // `YYYY-MM-DD`, `T` or a space, `HH:MM:SS`, then anything (a fraction, an
                // offset, `Z`): both engines' native texts start this way.
                let b = value.as_bytes();
                let digit = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
                [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18]
                    .into_iter()
                    .all(digit)
                    && b.get(4) == Some(&b'-')
                    && b.get(7) == Some(&b'-')
                    && matches!(b.get(10), Some(b'T' | b' '))
                    && b.get(13) == Some(&b':')
                    && b.get(16) == Some(&b':')
            }
        }
    }
    /// The published schema of one non-null value.
    fn value_schema(self, masked: bool) -> Value {
        if masked {
            return json!({"type":"string","pattern":"^(\\*+[\\s\\S]{0,4})?$",
                "description": format!("{}, masked: each character but the last four is `*`", self.name())});
        }
        match self {
            ColumnType::Text => json!({"type":"string","description":"text"}),
            ColumnType::PhoneNumber => {
                json!({"type":"string","description":"phone_number, unmasked by the profile"})
            }
            ColumnType::Integer => {
                json!({"type":"string","pattern":"^-?[0-9]+$","description":"integer, as decimal text"})
            }
            ColumnType::Timestamp => json!({"type":"string",
                "pattern":"^[0-9]{4}-[0-9]{2}-[0-9]{2}[T ][0-9]{2}:[0-9]{2}:[0-9]{2}",
                "description":"timestamp, as the engine's native text"}),
        }
    }
}

/// `masked` is a boolean when present; `null` is refused, as the model refuses it.
fn present_bool<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<bool>, D::Error> {
    bool::deserialize(deserializer).map(Some)
}

/// One declared output column, in the statement's column order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ColumnType,
    #[serde(
        default,
        deserialize_with = "present_bool",
        skip_serializing_if = "Option::is_none"
    )]
    pub masked: Option<bool>,
}
impl ProfileColumn {
    /// `masked` when the profile says, otherwise exactly the `phone_number` columns
    /// (`connectors_sql.profiles.ColumnMasking`).
    pub fn effective_masked(&self) -> bool {
        self.masked.unwrap_or(self.kind == ColumnType::PhoneNumber)
    }
}

/// One read profile as the native file declares it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadProfile {
    pub id: String,
    pub statement: String,
    pub parameters: Vec<String>,
    pub columns: Vec<ProfileColumn>,
    pub call_identifier: String,
    pub ordering: String,
}

fn bounded_name(value: &str, alphabet: fn(u8) -> bool) -> bool {
    (1..=64).contains(&value.len()) && value.bytes().all(alphabet)
}
fn field_name(value: &str) -> bool {
    bounded_name(value, |b| {
        b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'
    })
}
fn profile_id(value: &str) -> bool {
    bounded_name(value, |b| {
        b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-')
    })
}

impl ReadProfile {
    /// Every rule of `connectors_sql.profiles.ReadProfile` and of the profile rules of
    /// `connectors_sql.connection.LocalConfiguration`.
    pub fn validate(&self) -> Result<()> {
        let refuse = |why: &str| Err(Error::invalid(format!("invalid read profile: {why}")));
        if !profile_id(&self.id) {
            return refuse("id");
        }
        if self.statement.is_empty() || self.statement.len() > 8192 {
            return refuse("statement");
        }
        if self.parameters.len() > MAX_PARAMETERS
            || !self.parameters.iter().all(|p| field_name(p))
            || self.parameters.iter().collect::<BTreeSet<_>>().len() != self.parameters.len()
        {
            return refuse("parameters");
        }
        if !(1..=crate::MAX_COLUMNS).contains(&self.columns.len())
            || !self.columns.iter().all(|c| field_name(&c.name))
            || self
                .columns
                .iter()
                .map(|c| &c.name)
                .collect::<BTreeSet<_>>()
                .len()
                != self.columns.len()
        {
            return refuse("columns");
        }
        for key in [&self.call_identifier, &self.ordering] {
            match self.columns.iter().find(|c| &c.name == key) {
                Some(column)
                    if column.kind != ColumnType::PhoneNumber && !column.effective_masked() => {}
                _ => return refuse("call identifier or ordering column"),
            }
        }
        Ok(())
    }
    fn ordering_index(&self) -> usize {
        self.columns
            .iter()
            .position(|c| c.name == self.ordering)
            .expect("a validated profile names its ordering column")
    }
    fn key(&self, column: &ProfileColumn) -> bool {
        column.name == self.call_identifier || column.name == self.ordering
    }

    /// The published input of this profile: its id, a value (text or null) for each
    /// declared parameter and nothing else, an optional cursor and the limit.
    fn input_schema(&self) -> Value {
        let parameter = json!({"anyOf":[{"type":"string","maxLength":8192},{"type":"null"}]});
        let mut required = vec!["profile", "limit"];
        if !self.parameters.is_empty() {
            required.push("parameters");
        }
        json!({
            "type": "object",
            "properties": {
                "profile": {"const": self.id},
                "parameters": {
                    "type": "object",
                    "properties": self.parameters.iter()
                        .map(|p| (p.clone(), parameter.clone()))
                        .collect::<Map<_, _>>(),
                    "required": self.parameters,
                    "additionalProperties": false
                },
                "cursor": {"anyOf":[{"type":"string","minLength":1,"maxLength":16384},{"type":"null"}]},
                "limit": {"type":"integer","minimum":1,"maximum":1000}
            },
            "required": required,
            "additionalProperties": false
        })
    }

    fn columns_document(&self) -> Value {
        Value::Array(
            self.columns
                .iter()
                .map(|c| json!({"name": c.name, "type": c.kind.name(), "masked": c.effective_masked()}))
                .collect(),
        )
    }

    /// The published output of this profile: its own columns under their declared names and
    /// types, the call identifier and ordering columns never null.
    fn output_schema(&self, provenance: &Value) -> Value {
        let properties = self
            .columns
            .iter()
            .map(|c| {
                let value = c.kind.value_schema(c.effective_masked());
                let schema = if self.key(c) {
                    value
                } else {
                    json!({"anyOf": [value, {"type": "null"}]})
                };
                (c.name.clone(), schema)
            })
            .collect::<Map<_, _>>();
        json!({
            "type": "object",
            "properties": {
                "profile": {"const": self.id},
                "call_identifier": {"const": self.call_identifier},
                "ordering": {"const": self.ordering},
                "columns": {"const": self.columns_document()},
                "items": {"type": "array", "items": {
                    "type": "object",
                    "properties": properties,
                    "required": self.columns.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
                    "additionalProperties": false
                }},
                "next_cursor": {"anyOf":[{"type":"string","minLength":1,"maxLength":16384},{"type":"null"}]},
                "complete": {"type": "boolean"},
                "provenance": provenance
            },
            "required": ["profile","call_identifier","ordering","columns","items","next_cursor","complete","provenance"],
            "additionalProperties": false
        })
    }
}

/// Validate a connection's profiles: each one, at most 32, ids distinct.
pub fn validate_profiles(profiles: &[ReadProfile]) -> Result<()> {
    if profiles.len() > MAX_PROFILES
        || profiles
            .iter()
            .map(|p| &p.id)
            .collect::<BTreeSet<_>>()
            .len()
            != profiles.len()
    {
        return Err(Error::invalid("invalid read profiles"));
    }
    profiles.iter().try_for_each(ReadProfile::validate)
}

/// The instance's `profile.read`: the generated declaration with each configured profile's
/// exact input and output published as one alternative of each schema.
pub(crate) fn specialize(template: &Operation, profiles: &[ReadProfile]) -> Operation {
    let provenance = &template.output_schema["properties"]["provenance"];
    let mut operation = template.clone();
    operation.input_schema =
        json!({"oneOf": profiles.iter().map(ReadProfile::input_schema).collect::<Vec<_>>()});
    operation.output_schema =
        json!({"oneOf": profiles.iter().map(|p| p.output_schema(provenance)).collect::<Vec<_>>()});
    operation
}

/// A masked value: its last four characters kept, each earlier one written as `*`; a value
/// of four characters or fewer is all `*`.
pub fn mask(value: &str) -> String {
    let count = value.chars().count();
    if count <= 4 {
        return "*".repeat(count);
    }
    let mut masked = "*".repeat(count - 4);
    masked.extend(value.chars().skip(count - 4));
    masked
}

/// The caller's input; the instance's input schema has already admitted it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Input {
    pub profile: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, Option<String>>,
    #[serde(default)]
    pub cursor: Option<String>,
    pub limit: u16,
}

/// What a cursor carries: the profile and parameter values it continues, and the ordering
/// value of the last row returned. It is not signed: the ordering column is never masked,
/// so the value is one the caller already read, and a forged value only moves where a read
/// the caller may make anyway starts. A cursor for another profile or other values is stale.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    profile: String,
    context: String,
    after: String,
}
fn context(parameters: &BTreeMap<String, Option<String>>) -> String {
    connectors_core::digest(&json!(parameters))
}
fn stale() -> Error {
    Error::new(
        ErrorCode::StaleCursor,
        "cursor does not belong to this profile and these parameters",
    )
}

/// What a page binds and runs: the statement, the declared values in placeholder order, and
/// the keyset over the ordering column.
pub(crate) struct Prepared {
    pub statement: String,
    pub parameters: Vec<Option<String>>,
    pub keyset: crate::Keyset,
}

/// Admit one call before any session: the profile, exactly its declared parameters and a
/// cursor issued for this profile and these values.
pub(crate) fn prepare<'a>(
    profiles: &'a [ReadProfile],
    input: &Input,
) -> Result<(&'a ReadProfile, Prepared)> {
    let profile = profiles
        .iter()
        .find(|p| p.id == input.profile)
        .ok_or_else(|| Error::invalid("unknown read profile"))?;
    if input.parameters.len() != profile.parameters.len()
        || !profile
            .parameters
            .iter()
            .all(|p| input.parameters.contains_key(p))
    {
        return Err(Error::invalid(
            "parameters must be exactly the profile's declared ones",
        ));
    }
    let after = match &input.cursor {
        None => None,
        Some(token) => {
            let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(token)
                .map_err(|_| stale())?;
            let cursor: Cursor = serde_json::from_slice(&bytes).map_err(|_| stale())?;
            if cursor.profile != profile.id || cursor.context != context(&input.parameters) {
                return Err(stale());
            }
            Some(cursor.after)
        }
    };
    let index = profile.ordering_index();
    Ok((
        profile,
        Prepared {
            statement: profile.statement.clone(),
            parameters: profile
                .parameters
                .iter()
                .map(|p| input.parameters[p].clone())
                .collect(),
            keyset: crate::Keyset {
                column: index,
                kind: profile.columns[index].kind,
                after,
                columns: profile.columns.len(),
            },
        },
    ))
}

/// Write one page from the rows the read returned (at most `limit`): every row typed and
/// masked as declared, and a cursor after the last one when there is one.
pub(crate) fn page(
    profile: &ReadProfile,
    input: &Input,
    rows: &[Value],
    provenance: Value,
) -> Result<Value> {
    let unsupported = |why: &'static str| Error::new(ErrorCode::Unsupported, why);
    let mut items = Vec::with_capacity(rows.len());
    let mut last_key = None;
    for row in rows {
        let cells = row.as_array().ok_or_else(Error::internal)?;
        if cells.len() != profile.columns.len() {
            return Err(unsupported(
                "profile statement does not return its declared columns",
            ));
        }
        let mut item = Map::new();
        for (column, cell) in profile.columns.iter().zip(cells) {
            let value = match cell {
                Value::Null if profile.key(column) => {
                    return Err(unsupported("a call identifier or ordering value was null"));
                }
                Value::Null => Value::Null,
                Value::String(text) if !column.kind.admits(text) => {
                    return Err(unsupported(
                        "a value does not match its declared column type",
                    ));
                }
                Value::String(text) if column.effective_masked() => Value::String(mask(text)),
                Value::String(text) => Value::String(text.clone()),
                _ => return Err(Error::internal()),
            };
            if column.name == profile.ordering {
                last_key = value.as_str().map(str::to_owned);
            }
            item.insert(column.name.clone(), value);
        }
        items.push(Value::Object(item));
    }
    let next_cursor = match last_key {
        None => None,
        Some(after) if after.len() > MAX_CURSOR_VALUE => {
            return Err(unsupported(
                "an ordering value is too long to continue after",
            ));
        }
        Some(after) => {
            let cursor = serde_json::to_vec(&Cursor {
                profile: profile.id.clone(),
                context: context(&input.parameters),
                after,
            })
            .map_err(|_| Error::internal())?;
            Some(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(cursor))
        }
    };
    let page = json!({
        "profile": profile.id,
        "call_identifier": profile.call_identifier,
        "ordering": profile.ordering,
        "columns": profile.columns_document(),
        "complete": next_cursor.is_none(),
        "next_cursor": next_cursor,
        "items": items,
        "provenance": provenance,
    });
    if serde_json::to_vec(&page)
        .map_err(|_| Error::internal())?
        .len()
        > connectors_core::RESPONSE_LIMIT
    {
        return Err(Error::new(
            ErrorCode::Capacity,
            "profile page exceeds byte limit; ask for a smaller limit",
        ));
    }
    Ok(page)
}
