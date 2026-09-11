//! A locally authored action, expanded into the operation the inventory emits.
//! This path exists so a person can name one HTTP action without an OpenAPI
//! document; it is not a second engine. The file is read into an
//! [`Operation`](crate::inventory::Operation) and handed to
//! [`Template::from_operation`], which stays the only place a template is built,
//! so every refusal the imported path enforces applies here unchanged and a
//! template refusal is reported as itself. No credential, no host, no execution:
//! the document names an auth profile, it does not carry one.

use crate::inventory::{Location, METHODS, Operation, Parameter};
use crate::template::{self, Template};
use connectors_core::{Error, ErrorCode, Result};
use std::path::Path;
use toml::Value;
use toml::value::Table;

/// The largest authored document this reader accepts. An authored file names a
/// handful of actions by hand; anything past this is a generated document, which
/// belongs in the ingest path with its provenance rather than here.
pub const DOCUMENT_LIMIT: usize = 64 * 1024;

/// Why an authored document was refused. Each case carries the field, value or
/// path it concerns so a report can name it without re-deriving it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No file at this path. Carries the file's own name.
    FileAbsent(String),
    /// A file this reader may not open.
    FileForbidden(String),
    /// A path that exists and is not a file this reader can read at all.
    Unreadable(String),
    /// Larger than [`DOCUMENT_LIMIT`].
    TooLarge,
    /// Not TOML this build parses. Carries the parser's own complaint.
    Malformed(String),
    /// A field the document must declare and does not, by its dotted name.
    FieldAbsent(String),
    /// A field this reader does not read. Refused rather than ignored, because a
    /// misspelled field would otherwise silently leave its default in place.
    FieldUnknown(String),
    /// A field present but not the type this reader reads.
    FieldMalformed(String),
    /// A method outside [`METHODS`], named as the document wrote it.
    MethodUnknown(String),
    /// A path that does not begin with `/`. A relative path would be joined to a
    /// runtime's base in a way the document never described.
    PathRelative(String),
    /// A parameter location the inventory model does not carry.
    LocationUnsupported(String),
    /// An `action` array that declares no action.
    NoAction,
    /// Two actions in one document under one name. Keeping the last would make
    /// the file's meaning depend on its order.
    ActionDuplicate(String),
    /// One parameter declared twice under one name and location.
    ParameterDuplicate(String),
    /// One request media type offered twice.
    MediaTypeDuplicate(String),
    /// What the template pass refused about the expanded operation, held as it
    /// was raised so the authored path restates nothing.
    Template(template::Refusal),
}

impl Refusal {
    /// The field, value or parameter the refusal is about.
    pub fn subject(&self) -> &str {
        match self {
            Self::FileAbsent(subject)
            | Self::FileForbidden(subject)
            | Self::Unreadable(subject)
            | Self::Malformed(subject)
            | Self::FieldAbsent(subject)
            | Self::FieldUnknown(subject)
            | Self::FieldMalformed(subject)
            | Self::MethodUnknown(subject)
            | Self::PathRelative(subject)
            | Self::LocationUnsupported(subject)
            | Self::ActionDuplicate(subject)
            | Self::ParameterDuplicate(subject)
            | Self::MediaTypeDuplicate(subject) => subject,
            Self::TooLarge => "document",
            Self::NoAction => "action",
            Self::Template(inner) => inner.subject(),
        }
    }

    pub fn reason(&self) -> String {
        match self {
            Self::FileAbsent(name) => format!("no authored document `{name}` is there to read"),
            Self::FileForbidden(name) => {
                format!("authored document `{name}` may not be read by this reader")
            }
            Self::Unreadable(name) => format!("authored document `{name}` cannot be read"),
            Self::TooLarge => "authored document exceeds the authored limit".to_owned(),
            Self::Malformed(complaint) => {
                format!("authored document is not TOML this build parses: {complaint}")
            }
            Self::FieldAbsent(field) => format!("`{field}` is a field this document must declare"),
            Self::FieldUnknown(field) => format!("`{field}` is not a field this document reads"),
            Self::FieldMalformed(field) => {
                format!("`{field}` is not the type this document reads")
            }
            Self::MethodUnknown(method) => format!("`{method}` is not a method this build reads"),
            Self::PathRelative(path) => format!("path `{path}` does not begin with `/`"),
            Self::LocationUnsupported(location) => {
                format!("parameter location `{location}` is not one the model carries")
            }
            Self::NoAction => "the document's `action` array declares no action".to_owned(),
            Self::ActionDuplicate(name) => {
                format!("action `{name}` is declared twice in one document")
            }
            Self::ParameterDuplicate(key) => {
                format!("parameter `{key}` is declared twice in one action")
            }
            Self::MediaTypeDuplicate(media_type) => {
                format!("request media type `{media_type}` is offered twice")
            }
            Self::Template(inner) => inner.reason(),
        }
    }
}

impl From<Refusal> for Error {
    fn from(value: Refusal) -> Self {
        // A template refusal is converted by the template pass itself. Restating
        // it here would give the authored path a second classification of the
        // same fact, which is exactly what this module exists not to do.
        if let Refusal::Template(inner) = value {
            return Error::from(inner);
        }
        // Matched case by case rather than with a catch-all, so a refusal added
        // later cannot inherit a classification nobody chose for it.
        let code = match value {
            Refusal::LocationUnsupported(_) => ErrorCode::Unsupported,
            Refusal::FileAbsent(_) => ErrorCode::NotFound,
            Refusal::FileForbidden(_) => ErrorCode::Forbidden,
            Refusal::TooLarge => ErrorCode::Capacity,
            Refusal::Unreadable(_)
            | Refusal::NoAction
            | Refusal::ParameterDuplicate(_)
            | Refusal::MediaTypeDuplicate(_)
            | Refusal::Malformed(_)
            | Refusal::FieldAbsent(_)
            | Refusal::FieldUnknown(_)
            | Refusal::FieldMalformed(_)
            | Refusal::MethodUnknown(_)
            | Refusal::PathRelative(_)
            | Refusal::ActionDuplicate(_) => ErrorCode::InvalidInput,
            Refusal::Template(_) => unreachable!("returned above"),
        };
        Error::new(code, value.reason())
    }
}

/// One authored action: the operation it expands to, the template built from
/// that operation, and the auth profile it is performed under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub name: String,
    pub operation: Operation,
    pub template: Template,
    /// The profile this action authenticates under — its own, or the document's
    /// when it names none. A reference by id; no credential is read here.
    pub auth_profile: Option<String>,
}

/// What one authored file declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub provider: String,
    pub actions: Vec<Action>,
}

impl Document {
    pub fn find(&self, name: &str) -> Option<&Action> {
        self.actions.iter().find(|action| action.name == name)
    }
}

type Refused<T> = std::result::Result<T, Refusal>;

/// A field's name as a reader of the file sees it: `action.parameter.location`,
/// not `location` alone, which several tables carry.
fn qualify(scope: &str, field: &str) -> String {
    if scope.is_empty() {
        field.to_owned()
    } else {
        format!("{scope}.{field}")
    }
}

/// Every key this table may carry. A key outside the set is refused rather than
/// ignored: a misspelled `require` would otherwise leave `required` false and
/// bind an operation the author did not describe.
fn only(table: &Table, scope: &str, fields: &[&str]) -> Refused<()> {
    for key in table.keys() {
        if !fields.contains(&key.as_str()) {
            return Err(Refusal::FieldUnknown(qualify(scope, key)));
        }
    }
    Ok(())
}

fn text(table: &Table, scope: &str, field: &str) -> Refused<Option<String>> {
    match table.get(field) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(Refusal::FieldMalformed(qualify(scope, field))),
    }
}

fn required_text(table: &Table, scope: &str, field: &str) -> Refused<String> {
    text(table, scope, field)?.ok_or_else(|| Refusal::FieldAbsent(qualify(scope, field)))
}

fn flag(table: &Table, scope: &str, field: &str) -> Refused<bool> {
    match table.get(field) {
        None => Ok(false),
        Some(Value::Boolean(value)) => Ok(*value),
        Some(_) => Err(Refusal::FieldMalformed(qualify(scope, field))),
    }
}

fn texts(table: &Table, scope: &str, field: &str) -> Refused<Vec<String>> {
    let Some(value) = table.get(field) else {
        return Ok(Vec::new());
    };
    let Some(list) = value.as_array() else {
        return Err(Refusal::FieldMalformed(qualify(scope, field)));
    };
    list.iter()
        .map(|member| match member {
            Value::String(text) => Ok(text.clone()),
            _ => Err(Refusal::FieldMalformed(qualify(scope, field))),
        })
        .collect()
}

fn tables<'a>(table: &'a Table, scope: &str, field: &str) -> Refused<Vec<&'a Table>> {
    let Some(value) = table.get(field) else {
        return Ok(Vec::new());
    };
    let Some(list) = value.as_array() else {
        return Err(Refusal::FieldMalformed(qualify(scope, field)));
    };
    list.iter()
        .map(|member| {
            member
                .as_table()
                .ok_or_else(|| Refusal::FieldMalformed(qualify(scope, field)))
        })
        .collect()
}

fn parameter(table: &Table) -> Refused<Parameter> {
    const SCOPE: &str = "action.parameter";
    only(table, SCOPE, &["name", "location", "required"])?;
    let name = required_text(table, SCOPE, "name")?;
    let declared = required_text(table, SCOPE, "location")?;
    // The inventory's own table, not a copy of it: a document spells a location
    // the way an OpenAPI document spells it, and two tables that must agree are
    // one table that cannot disagree.
    let location =
        Location::parse(&declared).ok_or_else(|| Refusal::LocationUnsupported(declared.clone()))?;
    Ok(Parameter {
        name,
        location,
        required: flag(table, SCOPE, "required")?,
    })
}

/// The request media types an action offers, as the set an equivalent import
/// offers: an imported operation reads them out of a JSON object, which is a
/// sorted map of unique keys, and `Template::bind` takes the first offered as
/// the default. An authored list in another order would choose another default
/// and stop binding as the equivalent operation binds.
fn media_types(table: &Table) -> Refused<Vec<String>> {
    let mut declared = texts(table, "action", "request_media_types")?;
    declared.sort();
    if let Some(repeated) = declared.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(Refusal::MediaTypeDuplicate(repeated[0].clone()));
    }
    Ok(declared)
}

fn action(table: &Table, document_profile: Option<&String>) -> Refused<Action> {
    const SCOPE: &str = "action";
    only(
        table,
        SCOPE,
        &[
            "name",
            "method",
            "path",
            "parameter",
            "request_media_types",
            "auth_profile",
        ],
    )?;
    let name = required_text(table, SCOPE, "name")?;
    let declared = required_text(table, SCOPE, "method")?;
    // A person writes `GET` as readily as `get`; the inventory emits the lower
    // case spelling, so an authored action is normalised to it rather than
    // producing an operation the imported path would never produce.
    let method = declared.to_ascii_lowercase();
    if !METHODS.contains(&method.as_str()) {
        return Err(Refusal::MethodUnknown(declared));
    }
    let path = required_text(table, SCOPE, "path")?;
    if !path.starts_with('/') {
        return Err(Refusal::PathRelative(path));
    }
    let mut parameters: Vec<Parameter> = Vec::new();
    for declared in tables(table, SCOPE, "parameter")? {
        let parameter = parameter(declared)?;
        // Refused, not merged. An imported document merges a repeat because it
        // has two layers and the operation's own declaration overrides the path
        // item's; one authored action has no second layer for an override to
        // mean anything, so a repeat is a mistake, and taking the last would
        // drop a `required` the author wrote and leave this action's operation
        // and its template disagreeing about whether a value is needed.
        if parameters
            .iter()
            .any(|kept| kept.name == parameter.name && kept.location == parameter.location)
        {
            // Named the way the template pass names a parameter whose identity
            // is its location as well as its name.
            return Err(Refusal::ParameterDuplicate(format!(
                "{}:{}",
                parameter.location.label(),
                parameter.name
            )));
        }
        parameters.push(parameter);
    }
    let operation = Operation {
        method,
        path,
        // The action's name is the operation's identity, which is what an
        // `operationId` is in an imported document.
        operation_id: Some(name.clone()),
        parameters,
        request_media_types: media_types(table)?,
        // An authored action describes a request, not the responses it may get.
        responses: Vec::new(),
    };
    let template = Template::from_operation(&operation).map_err(Refusal::Template)?;
    Ok(Action {
        name,
        operation,
        template,
        auth_profile: text(table, SCOPE, "auth_profile")?.or_else(|| document_profile.cloned()),
    })
}

/// Read exactly these bytes as an authored document.
pub fn read(bytes: &[u8]) -> Refused<Document> {
    if bytes.len() > DOCUMENT_LIMIT {
        return Err(Refusal::TooLarge);
    }
    let text_form =
        std::str::from_utf8(bytes).map_err(|error| Refusal::Malformed(error.to_string()))?;
    let root: Table = toml::from_str(text_form)
        .map_err(|error| Refusal::Malformed(error.message().to_owned()))?;
    only(&root, "", &["provider", "auth_profile", "action"])?;
    let provider = required_text(&root, "", "provider")?;
    let document_profile = text(&root, "", "auth_profile")?;
    let declared = tables(&root, "", "action")?;
    if declared.is_empty() {
        return Err(match root.contains_key("action") {
            // A document that wrote `action = []` declared the field; telling its
            // author to declare it names a correction they cannot make.
            true => Refusal::NoAction,
            false => Refusal::FieldAbsent("action".into()),
        });
    }
    let mut actions: Vec<Action> = Vec::with_capacity(declared.len());
    for table in declared {
        let action = action(table, document_profile.as_ref())?;
        if actions.iter().any(|kept| kept.name == action.name) {
            return Err(Refusal::ActionDuplicate(action.name));
        }
        actions.push(action);
    }
    Ok(Document { provider, actions })
}

/// Read and expand a local authored file. The name a refusal carries is the
/// file's own name, not the path it was read through, so a refusal does not
/// carry a home directory.
pub fn read_file(path: &Path) -> Result<Document> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::from(Refusal::Unreadable(String::new())))?;
    let bytes = std::fs::read(path).map_err(|error| Error::from(unreadable(name, error.kind())))?;
    read(&bytes).map_err(Error::from)
}

/// Why a path could not be read, kept apart by kind. One refusal for all of them
/// would tell a caller that a directory, a file they may not open and a file that
/// is not there are the same fact, and only one of the three is fixed by writing
/// the document.
fn unreadable(name: &str, kind: std::io::ErrorKind) -> Refusal {
    match kind {
        std::io::ErrorKind::NotFound => Refusal::FileAbsent(name.to_owned()),
        std::io::ErrorKind::PermissionDenied => Refusal::FileForbidden(name.to_owned()),
        _ => Refusal::Unreadable(name.to_owned()),
    }
}
