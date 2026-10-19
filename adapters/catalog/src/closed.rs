//! A write's closed JSON body (`connectors_catalog.selection.ClosedBody`): the
//! keys `body_keys` admits, each a plain top-level name or an RFC 6901 pointer
//! into nested objects, and the members that type, require, fix, enumerate,
//! require one of and prefix them. Built and checked once when the selection
//! loads; then held on every write before anything is sent, and declared.
use crate::{BodyType, Selection, declared_type, guarded_body_paths, refuse};
use connectors_core::Result;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// A closed body key's path: its segments from the body's root.
type Path = Vec<String>;

/// The path a written key names: a plain name is one segment; a pointer led by
/// `/` is its segments, none empty and none holding `~` or `.`. `None` for
/// anything else: an empty or dotted plain name, or a malformed pointer.
fn path(key: &str) -> Option<Path> {
    match key.strip_prefix('/') {
        None => (!key.is_empty() && !key.contains('.')).then(|| vec![key.to_owned()]),
        Some(rest) => {
            let segments: Path = rest.split('/').map(str::to_owned).collect();
            segments
                .iter()
                .all(|segment| !segment.is_empty() && !segment.contains(['~', '.']))
                .then_some(segments)
        }
    }
}

/// Whether `inner` lies strictly inside `outer`.
fn inside(inner: &[String], outer: &[String]) -> bool {
    inner.len() > outer.len() && inner.starts_with(outer)
}

/// The value at a path in a body, if every object above it carries it.
fn at<'a>(body: &'a Value, path: &[String]) -> Option<&'a Value> {
    path.iter()
        .try_fold(body, |value, segment| value.get(segment))
}

/// The value at a path, creating the objects above it; `None` when a value
/// above it is not an object.
fn at_mut<'a>(body: &'a mut Value, path: &[String]) -> Option<&'a mut Value> {
    let mut value = body;
    for segment in path {
        if value.is_null() {
            *value = Value::Object(Map::new());
        }
        value = value
            .as_object_mut()?
            .entry(segment.clone())
            .or_insert(Value::Null);
    }
    Some(value)
}

/// The JSON Schema of a value typed `value_type`: exactly that JSON type.
pub(crate) fn type_schema(value_type: BodyType) -> Value {
    match value_type {
        BodyType::String => json!({"type": "string"}),
        BodyType::Integer => json!({"type": "integer"}),
        BodyType::Boolean => json!({"type": "boolean"}),
        BodyType::StringList => json!({"type": "array", "items": {"type": "string"}}),
    }
}

/// Whether a body value is a JSON value of exactly the declared type.
pub(crate) fn fits(value_type: BodyType, value: &Value) -> bool {
    match value_type {
        BodyType::String => value.is_string(),
        BodyType::Integer => value.is_i64() || value.is_u64(),
        BodyType::Boolean => value.is_boolean(),
        BodyType::StringList => value
            .as_array()
            .is_some_and(|items| items.iter().all(Value::is_string)),
    }
}

/// One selection's closed body, every name resolved to its path.
#[derive(Clone, Debug, Default)]
pub(crate) struct Closed {
    /// Each admitted key, in `body_keys` order, with the name it is written as.
    keys: Vec<(Path, String)>,
    types: BTreeMap<Path, BodyType>,
    /// `body_required`, in its order.
    required: Vec<Path>,
    nonempty: Vec<Path>,
    fixed: Vec<(Path, Value)>,
    enums: BTreeMap<Path, Vec<String>>,
    any_of: Vec<Path>,
    prefixes: Vec<(Path, String)>,
    /// The admitted keys a guard reads, at or above the path it reads: required.
    guarded: BTreeSet<Path>,
    /// The admitted keys a guard reads as themselves: compared as scalars.
    scalars: BTreeSet<Path>,
}

impl Closed {
    /// The closed body a selection declares, checked against what the members
    /// may name; `request_body` is whether its operation takes a body.
    pub(crate) fn new(selection: &Selection, request_body: bool) -> Result<Self> {
        let id = &selection.id;
        let refused = |reason: &str| Err(refuse(format!("selection `{id}` {reason}")));
        let mut closed = Self::default();
        if !selection.body_keys.is_empty() {
            let mut seen = BTreeSet::new();
            let paths: Option<Vec<Path>> = selection.body_keys.iter().map(|k| path(k)).collect();
            let valid = paths.as_ref().is_some_and(|paths| {
                paths.iter().all(|p| seen.insert(p.clone()))
                    && paths
                        .iter()
                        .all(|p| !paths.iter().any(|other| inside(p, other)))
            });
            if selection.effect != crate::Effect::Write || !request_body || !valid {
                return refused(
                    "declares body_keys that do not close a write's body with distinct plain names or pointers, none inside another",
                );
            }
            closed.keys = paths
                .unwrap_or_default()
                .into_iter()
                .zip(selection.body_keys.iter().cloned())
                .collect();
        }
        let admitted = |name: &str| -> Option<Path> {
            let path = path(name)?;
            closed.keys.iter().any(|(k, _)| *k == path).then_some(path)
        };
        // A guard reading a path the closed body cannot carry could never be
        // satisfied: refused here, not at every write. A path under an
        // admitted key reads into its value, which must then be untyped.
        let mut guarded = BTreeSet::new();
        let mut scalars = BTreeSet::new();
        if !closed.keys.is_empty()
            && let Some(guard) = &selection.guard
        {
            for dotted in guarded_body_paths(guard) {
                let read: Path = dotted.split('.').map(str::to_owned).collect();
                let Some((key, _)) = closed.keys.iter().find(|(key, _)| read.starts_with(key))
                else {
                    return Err(refuse(format!(
                        "guard of `{id}` reads `body.{dotted}`, which its body_keys do not admit"
                    )));
                };
                if read == *key {
                    scalars.insert(key.clone());
                }
                guarded.insert(key.clone());
            }
        }
        closed.guarded = guarded;
        closed.scalars = scalars;
        for (name, value_type) in &selection.body_types {
            let Some(path) = admitted(name) else {
                return refused("declares body_types for a key its body_keys do not admit");
            };
            if value_type == &BodyType::StringList && closed.scalars.contains(&path) {
                return Err(refuse(format!(
                    "guard of `{id}` reads body key `{name}` as a scalar, but body_types types it as a string_list"
                )));
            }
            if closed.types.insert(path, *value_type).is_some() {
                return Err(refuse(format!(
                    "selection `{id}` repeats body_types key `{name}`"
                )));
            }
        }
        let mut required = BTreeSet::new();
        for name in &selection.body_required {
            match admitted(name) {
                Some(path) if required.insert(path.clone()) => closed.required.push(path),
                _ => {
                    return refused(
                        "declares body_required that do not name distinct keys its body_keys admit",
                    );
                }
            }
        }
        // A guard path inside a typed key could never resolve.
        if let Some(guard) = &selection.guard {
            for dotted in guarded_body_paths(guard) {
                let read: Path = dotted.split('.').map(str::to_owned).collect();
                if closed.types.keys().any(|typed| inside(&read, typed)) {
                    return Err(refuse(format!(
                        "guard of `{id}` reads `body.{dotted}` under a key its body_types type as a scalar"
                    )));
                }
            }
        }
        let fixed_names: BTreeSet<Path> = selection
            .body_fixed
            .keys()
            .filter_map(|k| path(k))
            .collect();
        let string = |path: &Path| closed.types.get(path) == Some(&BodyType::String);
        // A non-empty key is a caller's string: admitted, typed as a
        // string, not fixed, and named once.
        let mut nonempty = BTreeSet::new();
        for name in &selection.body_nonempty {
            match admitted(name) {
                Some(path)
                    if string(&path)
                        && !fixed_names.contains(&path)
                        && nonempty.insert(path.clone()) =>
                {
                    closed.nonempty.push(path)
                }
                _ => {
                    return refused(
                        "declares body_nonempty that do not name distinct string keys its body_keys admit and the caller sends",
                    );
                }
            }
        }
        // A fixed key is a closed body key the caller never sends: its value
        // is a scalar of the key's type, and nothing may also require it
        // from the caller or compare it as caller input.
        for (name, value) in &selection.body_fixed {
            let scalar_value =
                value.is_string() || value.is_boolean() || value.is_i64() || value.is_u64();
            let path = admitted(name);
            let reason = match &path {
                None => Some("which its body_keys do not admit"),
                Some(_) if !scalar_value => {
                    Some("to a value that is not a string, an integer or a boolean")
                }
                Some(path)
                    if closed
                        .types
                        .get(path)
                        .is_some_and(|value_type| !fits(*value_type, value)) =>
                {
                    Some("to a value that is not of its declared type")
                }
                Some(path) if closed.required.contains(path) => {
                    Some("which its body_required also names")
                }
                Some(path) if closed.guarded.contains(path) => {
                    Some("which its guard reads as an input")
                }
                Some(_) => None,
            };
            if let Some(reason) = reason {
                return Err(refuse(format!(
                    "selection `{id}` fixes body key `{name}`, {reason}"
                )));
            }
            if let Some(path) = &path
                && closed.fixed.iter().any(|(fixed, _)| fixed == path)
            {
                return Err(refuse(format!(
                    "selection `{id}` repeats body_fixed key `{name}`"
                )));
            }
            closed.fixed.push((path.unwrap_or_default(), value.clone()));
        }
        for (name, values) in &selection.body_enums {
            let mut seen = BTreeSet::new();
            match admitted(name) {
                Some(path)
                    if string(&path)
                        && !fixed_names.contains(&path)
                        && !values.is_empty()
                        && values.iter().all(|v| !v.is_empty() && seen.insert(v)) =>
                {
                    if closed.enums.insert(path, values.clone()).is_some() {
                        return Err(refuse(format!(
                            "selection `{id}` repeats body_enums key `{name}`"
                        )));
                    }
                }
                _ => {
                    return refused(
                        "declares body_enums other than distinct non-empty values of a string key its body_keys admit and the caller sends",
                    );
                }
            }
        }
        if !selection.body_any_of.is_empty() {
            let mut seen = BTreeSet::new();
            let paths: Option<Vec<Path>> = selection
                .body_any_of
                .iter()
                .map(|name| {
                    admitted(name).filter(|p| !fixed_names.contains(p) && seen.insert(p.clone()))
                })
                .collect();
            let parent = |path: &Path| path[..path.len() - 1].to_vec();
            match paths {
                Some(paths)
                    if paths.len() >= 2 && paths.iter().all(|p| parent(p) == parent(&paths[0])) =>
                {
                    closed.any_of = paths
                }
                _ => {
                    return refused(
                        "declares body_any_of other than at least two distinct keys under one parent that its body_keys admit and the caller sends",
                    );
                }
            }
        }
        for (name, prefix) in &selection.body_prefixes {
            match admitted(name) {
                Some(path)
                    if string(&path) && !fixed_names.contains(&path) && !prefix.is_empty() =>
                {
                    if closed.prefixes.iter().any(|(seen, _)| *seen == path) {
                        return Err(refuse(format!(
                            "selection `{id}` repeats body_prefixes key `{name}`"
                        )));
                    }
                    closed.prefixes.push((path, prefix.clone()))
                }
                _ => {
                    return refused(
                        "declares body_prefixes other than a non-empty prefix of a string key its body_keys admit and the caller sends",
                    );
                }
            }
        }
        Ok(closed)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Whether the caller's `body` may be omitted: every admitted key is
    /// fixed, so a caller has nothing to send in it.
    pub(crate) fn fixed_whole(&self) -> bool {
        !self.fixed.is_empty()
            && self
                .keys
                .iter()
                .all(|(key, _)| self.fixed.iter().any(|(fixed, _)| fixed == key))
    }

    fn name(&self, path: &[String]) -> &str {
        self.keys
            .iter()
            .find(|(key, _)| key == path)
            .map_or("", |(_, name)| name.as_str())
    }

    fn fixed(&self, path: &[String]) -> bool {
        self.fixed.iter().any(|(fixed, _)| fixed == path)
    }

    /// Whether some admitted key lies inside `path`, which is then an object.
    fn above(&self, path: &[String]) -> bool {
        self.keys.iter().any(|(key, _)| inside(key, path))
    }

    /// The shape of a caller's body: an object carrying only the segments
    /// that lead to admitted keys, each object above a key an object, and no
    /// fixed key.
    fn shape(&self, value: &Value, at: &mut Path) -> Result<()> {
        let Some(members) = value.as_object() else {
            return Err(if at.is_empty() {
                refuse("body is missing or not an object, as its selection closes it")
            } else {
                refuse(format!(
                    "body value `/{}` is not an object, as its selection closes it",
                    at.join("/")
                ))
            });
        };
        for (segment, member) in members {
            at.push(segment.clone());
            if self.keys.iter().any(|(key, _)| key == at) {
                if self.fixed(at) {
                    return Err(refuse(format!(
                        "body key `{}` is fixed by its selection and is not caller input",
                        self.name(at)
                    )));
                }
            } else if self.above(at) {
                self.shape(member, at)?;
            } else {
                return Err(refuse(
                    "body is missing or not an object, as its selection closes it",
                ));
            }
            at.pop();
        }
        Ok(())
    }

    /// Hold a caller's body to the closed shape and every rule of its keys,
    /// before anything is sent.
    pub(crate) fn check(&self, body: &Value) -> Result<()> {
        self.shape(body, &mut Vec::new())?;
        if let Some(path) = self.required.iter().find(|path| at(body, path).is_none()) {
            return Err(refuse(format!(
                "body lacks the required key `{}`",
                self.name(path)
            )));
        }
        if let Some((path, _)) = self.types.iter().find(|(path, value_type)| {
            at(body, path).is_some_and(|value| !fits(**value_type, value))
        }) {
            return Err(refuse(format!(
                "body key `{}` is not of its declared type",
                self.name(path)
            )));
        }
        if let Some(path) = self.nonempty.iter().find(|path| {
            at(body, path)
                .and_then(Value::as_str)
                .is_some_and(str::is_empty)
        }) {
            return Err(refuse(format!(
                "body key `{}` is empty, which its selection does not admit",
                self.name(path)
            )));
        }
        if let Some((path, _)) = self.enums.iter().find(|(path, values)| {
            at(body, path)
                .and_then(Value::as_str)
                .is_some_and(|value| !values.iter().any(|allowed| allowed == value))
        }) {
            return Err(refuse(format!(
                "body key `{}` is not one of its allowed values",
                self.name(path)
            )));
        }
        if !self.any_of.is_empty() && self.any_of.iter().all(|path| at(body, path).is_none()) {
            let names: Vec<&str> = self.any_of.iter().map(|path| self.name(path)).collect();
            return Err(refuse(format!(
                "body carries none of `{}`, one of which its selection requires",
                names.join("`, `")
            )));
        }
        Ok(())
    }

    /// The body sent: the caller's, each fixed value at its key, the objects
    /// above it created where the caller's body lacks them, and each prefix
    /// before the caller's text.
    pub(crate) fn complete(&self, mut body: Value) -> Value {
        for (path, value) in &self.fixed {
            if let Some(slot) = at_mut(&mut body, path) {
                *slot = value.clone();
            }
        }
        for (path, prefix) in &self.prefixes {
            if let Some(Value::String(text)) = at_mut_existing(&mut body, path) {
                text.insert_str(0, prefix);
            }
        }
        body
    }

    /// The declared schema of the caller's body: the closed objects, without
    /// the fixed keys.
    pub(crate) fn schema(&self) -> Value {
        self.object(&[])
    }

    fn object(&self, at: &[String]) -> Value {
        let mut properties = Map::new();
        for (key, _) in &self.keys {
            if self.fixed(key) || !inside(key, at) {
                continue;
            }
            let segment = &key[at.len()];
            if properties.contains_key(segment) {
                continue;
            }
            let child: Path = key[..=at.len()].to_vec();
            let schema = if child == *key {
                self.leaf(key)
            } else {
                self.object(&child)
            };
            properties.insert(segment.clone(), schema);
        }
        let required: BTreeSet<&str> = self
            .required
            .iter()
            .chain(&self.guarded)
            .filter(|path| inside(path, at))
            .map(|path| path[at.len()].as_str())
            .collect();
        let mut object = json!({
            "type": "object",
            "properties": properties,
            "additionalProperties": false,
        });
        if !required.is_empty() {
            object["required"] = json!(required);
        }
        if self
            .any_of
            .first()
            .is_some_and(|first| first[..first.len() - 1] == *at)
        {
            object["anyOf"] = self
                .any_of
                .iter()
                .map(|path| json!({"required": [path[at.len()]]}))
                .collect();
        }
        object
    }

    /// A key typed by `body_types` is declared as exactly that JSON type;
    /// otherwise a key a guard reads directly is declared as the scalar the
    /// guard compares, and any other key takes any JSON value.
    fn leaf(&self, key: &Path) -> Value {
        if let Some(value_type) = self.types.get(key) {
            let mut schema = type_schema(*value_type);
            if self.nonempty.contains(key) {
                schema["minLength"] = json!(1);
            }
            if let Some(values) = self.enums.get(key) {
                schema["enum"] = json!(values);
            }
            schema
        } else if self.scalars.contains(key) {
            declared_type(None)
        } else {
            json!({})
        }
    }
}

/// The value at a path when every object above it exists; never creates one.
fn at_mut_existing<'a>(body: &'a mut Value, path: &[String]) -> Option<&'a mut Value> {
    path.iter()
        .try_fold(body, |value, segment| value.get_mut(segment))
}
