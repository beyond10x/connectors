//! How a read's pages are walked and when the walk ends: the `walk` member of a read in a
//! `connectors-catalog-operations/1` selection (`connectors_catalog.walk.Walk` in
//! `spec/ess/domains/walk.yaml`, stated in `docs/local-catalog-provider.md`). The engine checks
//! it when the selection loads and carries it in the operation's declared input schema under
//! [`KEYWORD`], an annotation no validator asserts, so the admitted description (and
//! `invoke --describe`) carries it beside the schema and revision an invoke reads.
//!
//! The evaluator ([`Walker::step`]) decides one page at a time: `continues` holding goes on
//! whatever the ends say, else the first end that holds ends the walk by its name, else the walk
//! goes on to the position at `next`. A page that names no position where one is needed, or one
//! seen before in the walk, breaks it ([`Broken`]). Running the walk, one admitted read per page,
//! is the caller's.
use crate::feed::{Condition, url_parameter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// The input schema annotation under which a declared operation carries its walk.
pub const KEYWORD: &str = "x-connectors-walk";
/// The most ends one walk declares.
pub const ENDS: usize = 8;
/// The longest end name.
const NAME_BYTES: usize = 64;

/// One read's walk (`connectors_catalog.walk.Walk`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Walk {
    /// The query parameter that carries the position from the second page on.
    pub parameter: String,
    /// RFC 6901 pointer, in a page's answer, to the next position.
    pub next: String,
    /// With it, `next` is a link and the position is this one query parameter of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_parameter: Option<String>,
    /// RFC 6901 pointer, in a page's answer, to its records.
    pub records: String,
    /// Query values every page sends.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub query: BTreeMap<String, Value>,
    /// Query parameters the first page sends and later pages leave out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub first_only: Vec<String>,
    /// While it holds the walk goes on, whatever `ends` say, and needs a next position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continues: Option<Condition>,
    /// The first that holds ends the walk, by its name.
    pub ends: Vec<End>,
}

/// One named end condition (`connectors_catalog.walk.End`): exactly one of the four tests.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct End {
    pub name: String,
    /// Holds when the value at `pointer` equals `equals`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<Condition>,
    /// Holds when the value at the pointer is absent, null or the empty string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absent: Option<String>,
    /// Holds when the value at the pointer is present and neither null nor the empty string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub present: Option<String>,
    /// Holds when the value at the pointer is absent, null or an empty array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empty: Option<String>,
}

/// What one page decides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// The walk goes on: the next page sends this position as `parameter`.
    Next(Value),
    /// The walk ends, by the name of the end that held.
    End(String),
}

/// A page the walk cannot go on from, nor end on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Broken {
    /// `continues` held and the page names no next position.
    ContinuesWithoutPosition,
    /// No end held and the page names no next position.
    NoPosition,
    /// The value at `next` is neither a string nor an integer.
    InvalidPosition,
    /// The next position was already sent in this walk.
    RepeatedPosition(Value),
    /// The value at `records` is neither absent, null nor an array.
    RecordsNotAList,
}

impl Broken {
    /// The stable name a walk's summary reports.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ContinuesWithoutPosition => "continues_without_position",
            Self::NoPosition => "no_position",
            Self::InvalidPosition => "invalid_position",
            Self::RepeatedPosition(_) => "repeated_position",
            Self::RecordsNotAList => "records_not_a_list",
        }
    }
}

/// An RFC 6901 pointer into a document: led by `/`, each `~` followed by `0` or `1`. The whole
/// document (the empty pointer) is never a walk's position, records or test.
fn pointer(text: &str) -> bool {
    text.starts_with('/') && {
        let mut escaped = false;
        text.chars().all(|c| {
            if escaped {
                escaped = false;
                matches!(c, '0' | '1')
            } else {
                escaped = c == '~';
                true
            }
        }) && !escaped
    }
}

/// An end name: a lower-case ASCII letter, then lower-case letters, digits and `_`.
fn name(text: &str) -> bool {
    text.len() <= NAME_BYTES
        && text.starts_with(|c: char| c.is_ascii_lowercase())
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Absent, null or the empty string.
fn missing(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::String(text)) => text.is_empty(),
        Some(_) => false,
    }
}

impl End {
    fn holds(&self, body: &Value) -> bool {
        if let Some(condition) = &self.when {
            condition.holds(body)
        } else if let Some(at) = &self.absent {
            missing(body.pointer(at))
        } else if let Some(at) = &self.present {
            !missing(body.pointer(at))
        } else if let Some(at) = &self.empty {
            match body.pointer(at) {
                None | Some(Value::Null) => true,
                Some(Value::Array(items)) => items.is_empty(),
                Some(_) => false,
            }
        } else {
            false
        }
    }
}

impl Walk {
    /// The walk an operation's declared input schema carries under [`KEYWORD`], checked;
    /// `None` when it carries none. A carried value that is no walk is refused by name.
    pub fn declared(input_schema: &Value) -> Result<Option<Self>, String> {
        let Some(carried) = input_schema.get(KEYWORD) else {
            return Ok(None);
        };
        let walk: Self = serde_json::from_value(carried.clone())
            .map_err(|error| format!("walk is not a walk: {error}"))?;
        walk.check()?;
        Ok(Some(walk))
    }

    /// The checks that need no operation: each pointer parses, the link parameter is a name,
    /// `query` holds scalars and not the position, `first_only` names neither, and `ends` holds
    /// one to [`ENDS`] ends, each named once and taking exactly one test.
    pub fn check(&self) -> Result<(), String> {
        let refused = |what: String| Err(format!("walk {what}"));
        if self.parameter.is_empty() {
            return refused("`parameter` is empty".into());
        }
        for (member, at) in [("next", &self.next), ("records", &self.records)] {
            if !pointer(at) {
                return refused(format!("`{member}` is not a JSON pointer"));
            }
        }
        if let Some(name) = &self.url_parameter
            && (name.is_empty() || name.len() > 128 || !name.bytes().all(|b| b.is_ascii_graphic()))
        {
            return refused("`url_parameter` is not 1 to 128 visible ASCII characters".into());
        }
        if let Some(condition) = &self.continues
            && !pointer(&condition.pointer)
        {
            return refused("`continues` is not a JSON pointer".into());
        }
        for (key, value) in &self.query {
            if key == &self.parameter {
                return refused(format!("`query` `{key}` is the walk's own `parameter`"));
            }
            if !(value.is_string() || value.is_boolean() || value.is_i64() || value.is_u64()) {
                return refused(format!(
                    "`query` `{key}` is not a string, an integer or a boolean"
                ));
            }
        }
        let mut first = BTreeSet::new();
        for key in &self.first_only {
            if key == &self.parameter || self.query.contains_key(key) || !first.insert(key) {
                return refused(format!(
                    "`first_only` `{key}` is the walk's own `parameter` or fixed in its `query`"
                ));
            }
        }
        if self.ends.is_empty() {
            return refused("`ends` holds no end".into());
        }
        if self.ends.len() > ENDS {
            return refused(format!("`ends` holds more than {ENDS} ends"));
        }
        let mut names = BTreeSet::new();
        for end in &self.ends {
            if !name(&end.name) {
                return refused(format!("end `{}` is not a name", end.name));
            }
            if !names.insert(end.name.as_str()) {
                return refused(format!("names end `{}` twice", end.name));
            }
            let tests = [
                ("when", end.when.as_ref().map(|c| &c.pointer)),
                ("absent", end.absent.as_ref()),
                ("present", end.present.as_ref()),
                ("empty", end.empty.as_ref()),
            ];
            let mut declared = tests.iter().filter(|(_, at)| at.is_some());
            let (Some((test, Some(at))), None) = (declared.next(), declared.next()) else {
                return refused(format!(
                    "end `{}` takes exactly one of `when`, `absent`, `present` and `empty`",
                    end.name
                ));
            };
            if !pointer(at) {
                return refused(format!("end `{}` `{test}` is not a JSON pointer", end.name));
            }
        }
        Ok(())
    }

    /// The page's records: absent or null is none; anything but an array breaks the walk.
    pub fn records<'a>(&self, body: &'a Value) -> Result<&'a [Value], Broken> {
        match body.pointer(&self.records) {
            None | Some(Value::Null) => Ok(&[]),
            Some(Value::Array(items)) => Ok(items),
            Some(_) => Err(Broken::RecordsNotAList),
        }
    }

    /// The position at `next`, or with `url_parameter` that parameter of the link at `next`;
    /// `None` when there is none.
    fn position(&self, body: &Value) -> Result<Option<Value>, Broken> {
        let found = body.pointer(&self.next);
        if missing(found) {
            return Ok(None);
        }
        match (found, &self.url_parameter) {
            (Some(Value::String(link)), Some(name)) => {
                Ok(url_parameter(link, name).map(Value::String))
            }
            (Some(_), Some(_)) => Err(Broken::InvalidPosition),
            (Some(value), None) if value.is_string() || value.is_i64() || value.is_u64() => {
                Ok(Some(value.clone()))
            }
            _ => Err(Broken::InvalidPosition),
        }
    }

    /// One page decided against the positions this walk already sent.
    pub fn step(&self, body: &Value, sent: &BTreeSet<String>) -> Result<Step, Broken> {
        self.records(body)?;
        let continues = self
            .continues
            .as_ref()
            .is_some_and(|condition| condition.holds(body));
        if !continues && let Some(end) = self.ends.iter().find(|end| end.holds(body)) {
            return Ok(Step::End(end.name.clone()));
        }
        let Some(position) = self.position(body)? else {
            return Err(if continues {
                Broken::ContinuesWithoutPosition
            } else {
                Broken::NoPosition
            });
        };
        if sent.contains(&position.to_string()) {
            return Err(Broken::RepeatedPosition(position));
        }
        Ok(Step::Next(position))
    }

    /// The input of one page: the caller's, with every `query` value, and from the second page
    /// on (`position` given) the position as `parameter` and no `first_only` parameter.
    pub fn page_input(&self, caller: &Value, position: Option<&Value>) -> Value {
        let mut input = caller.clone();
        if let Value::Object(members) = &mut input {
            for (key, value) in &self.query {
                members.insert(key.clone(), value.clone());
            }
            if let Some(position) = position {
                for key in &self.first_only {
                    members.remove(key);
                }
                members.insert(self.parameter.clone(), position.clone());
            }
        }
        input
    }
}

/// One walk in progress: the positions it sent, so a repeated one breaks it.
pub struct Walker<'a> {
    walk: &'a Walk,
    sent: BTreeSet<String>,
}

impl<'a> Walker<'a> {
    pub fn new(walk: &'a Walk) -> Self {
        Self {
            walk,
            sent: BTreeSet::new(),
        }
    }

    /// One page decided; a next position is recorded as sent.
    pub fn step(&mut self, body: &Value) -> Result<Step, Broken> {
        let step = self.walk.step(body, &self.sent)?;
        if let Step::Next(position) = &step {
            self.sent.insert(position.to_string());
        }
        Ok(step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pointer_is_led_by_a_slash_and_escapes_only_zero_and_one() {
        for good in [
            "/a",
            "/meta/after_cursor",
            "/a~0b",
            "/a~1b",
            "/",
            "/_links/next",
        ] {
            assert!(pointer(good), "{good}");
        }
        for bad in ["", "a", "meta/x", "/a~", "/a~2", "/~x"] {
            assert!(!pointer(bad), "{bad}");
        }
    }

    #[test]
    fn a_name_is_lower_snake_case() {
        for good in ["end_of_stream", "no_cursor", "a1"] {
            assert!(name(good), "{good}");
        }
        for bad in ["", "1a", "Has", "has more", "has-more", &"a".repeat(65)] {
            assert!(!name(bad), "{bad}");
        }
    }
}
