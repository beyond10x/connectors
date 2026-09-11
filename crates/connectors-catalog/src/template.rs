//! An inventoried operation as a bounded request template. A template describes a
//! request and never performs one: no host, no scheme, no credential and no
//! transport live here, and binding runs no code the document supplied. What the
//! caller's values cannot be made to say safely is refused by name rather than
//! encoded away into a request that means something else.

use crate::inventory::{Location, Operation, Parameter};
use connectors_core::{Error, ErrorCode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Why a binding was refused, each case carrying the parameter, media type or
/// path it concerns so a report can name it without re-deriving it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A required parameter the caller supplied no value for.
    ValueAbsent(String),
    /// A value for a key this operation does not declare. Passing it on would
    /// send a parameter the document never described.
    ParameterUndeclared(String),
    /// A value under a key every parameter it could reach is already bound by.
    /// Carrying it nowhere would be the silent drop this pass refuses.
    KeyUnread(String),
    /// A parameter no key reaches, because another parameter is literally named
    /// its qualified spelling. Refused when the template is built, so no refusal
    /// can later name a key a caller cannot use.
    ParameterUnreachable(String),
    /// A `{placeholder}` in the path with no declared parameter behind it.
    PlaceholderUndeclared(String),
    /// A parameter declared `in: path` that the path templates nowhere. Its value
    /// would reach no part of the request.
    PathParameterUnplaced(String),
    /// A path this pass cannot read as a template at all — an unterminated `{`.
    /// It names the path, because no one parameter is at fault.
    PathMalformed(String),
    /// A path value that is empty: it would collapse the segment and change the
    /// path the operation identifies.
    PathValueEmpty(String),
    /// A path value that is a dot segment. `.` and `..` are unreserved, so they
    /// survive encoding, and RFC 3986 section 5.2.4 resolves them away before the
    /// request leaves — moving the path rather than filling it.
    PathValueDotSegment(String),
    /// A header name or value carrying a byte a header line cannot hold. A bare
    /// CR or LF ends the line and starts a header nobody declared.
    HeaderUnsafe(String),
    /// A request media type the operation does not offer.
    MediaTypeUnoffered(String),
    /// A request media type that is not a `type/subtype` a header can carry. It
    /// is written into a header, so it is held to that line's grammar.
    MediaTypeUnsafe(String),
    /// A cookie parameter. This pass carries none, and says so rather than
    /// binding the operation as though the cookie were not declared.
    CookieUnsupported(String),
}

impl Refusal {
    /// The parameter name, media type or path the refusal is about.
    pub fn subject(&self) -> &str {
        match self {
            Self::ValueAbsent(subject)
            | Self::ParameterUndeclared(subject)
            | Self::KeyUnread(subject)
            | Self::ParameterUnreachable(subject)
            | Self::PlaceholderUndeclared(subject)
            | Self::PathParameterUnplaced(subject)
            | Self::PathMalformed(subject)
            | Self::PathValueEmpty(subject)
            | Self::PathValueDotSegment(subject)
            | Self::HeaderUnsafe(subject)
            | Self::MediaTypeUnoffered(subject)
            | Self::MediaTypeUnsafe(subject)
            | Self::CookieUnsupported(subject) => subject,
        }
    }

    pub fn reason(&self) -> String {
        match self {
            Self::ValueAbsent(name) => format!("required parameter `{name}` has no value"),
            Self::ParameterUndeclared(name) => {
                format!("`{name}` is not a parameter this operation declares")
            }
            Self::KeyUnread(key) => {
                format!("no parameter is left to read the value supplied under `{key}`")
            }
            Self::ParameterUnreachable(key) => {
                format!("no key reaches the parameter `{key}` names")
            }
            Self::PlaceholderUndeclared(name) => {
                format!("path placeholder `{name}` has no declared parameter")
            }
            Self::PathParameterUnplaced(name) => {
                format!("path parameter `{name}` has no placeholder in the path")
            }
            Self::PathMalformed(path) => {
                format!("path `{path}` has an unterminated placeholder")
            }
            Self::PathValueEmpty(name) => format!("path parameter `{name}` has an empty value"),
            Self::PathValueDotSegment(name) => {
                format!("path parameter `{name}` has a value that is a dot segment")
            }
            Self::HeaderUnsafe(name) => {
                format!("header `{name}` carries a byte a header line cannot hold")
            }
            Self::MediaTypeUnoffered(media_type) => {
                format!("this operation does not offer request media type `{media_type}`")
            }
            Self::MediaTypeUnsafe(media_type) => {
                format!("request media type `{media_type}` is not one this pass writes to a header")
            }
            Self::CookieUnsupported(name) => {
                format!("cookie parameter `{name}` is unsupported by this template pass")
            }
        }
    }
}

impl From<Refusal> for Error {
    fn from(value: Refusal) -> Self {
        // Matched case by case rather than with a catch-all, so a refusal added
        // later cannot inherit a classification nobody chose for it.
        let code = match value {
            Refusal::CookieUnsupported(_) => ErrorCode::Unsupported,
            Refusal::ValueAbsent(_)
            | Refusal::ParameterUndeclared(_)
            | Refusal::KeyUnread(_)
            | Refusal::ParameterUnreachable(_)
            | Refusal::PlaceholderUndeclared(_)
            | Refusal::PathParameterUnplaced(_)
            | Refusal::PathMalformed(_)
            | Refusal::PathValueEmpty(_)
            | Refusal::PathValueDotSegment(_)
            | Refusal::HeaderUnsafe(_)
            | Refusal::MediaTypeUnoffered(_)
            | Refusal::MediaTypeUnsafe(_) => ErrorCode::InvalidInput,
        };
        Error::new(code, value.reason())
    }
}

/// The characters a path segment keeps literal. Everything else is escaped, so a
/// value holding `/` cannot add a segment the operation never declared.
const PATH_KEEP: &[u8] = b"";
/// A query value may keep the characters a query legally carries; `&`, `=` and
/// `#` are not among them, so a value cannot add a pair or cut the query short.
const QUERY_KEEP: &[u8] = b"/:@";

const HEX: &[u8; 16] = b"0123456789ABCDEF";

fn encode(value: &str, keep: &[u8]) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        let unreserved = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~');
        if unreserved || keep.contains(&byte) {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push(HEX[usize::from(byte >> 4)] as char);
            out.push(HEX[usize::from(byte & 0x0f)] as char);
        }
    }
    out
}

/// A header field value carries neither a control byte nor a DEL: each of them
/// ends or mangles the line, so a request would carry a field nobody declared.
fn field_value_safe(text: &str) -> bool {
    !text.bytes().any(|byte| byte < 0x20 || byte == 0x7f)
}

/// RFC 9110 `tchar`. A field name is a `token`, so a name holding `:`, a space, a
/// comma or an `@` is not a header name at all — it is written into the line the
/// same way and read back as something else, or as nothing.
fn token(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
}

/// RFC 9110 `media-type`: `type/subtype`, each a token, with parameters this pass
/// does not parse beyond the field-value rule. It is written into a header, so
/// what is not a media type cannot be written as one.
fn media_type_writable(text: &str) -> bool {
    let (essence, parameters) = match text.split_once(';') {
        Some((essence, parameters)) => (essence, parameters),
        None => (text, ""),
    };
    match essence.split_once('/') {
        Some((kind, subtype)) => token(kind) && token(subtype) && field_value_safe(parameters),
        None => false,
    }
}

/// How a parameter is addressed in the supplied map when its name alone is
/// ambiguous. A parameter's identity is its name *and* its location, and this is
/// the only spelling that can say both.
fn qualified(parameter: &Parameter) -> String {
    format!("{}:{}", label(parameter.location), parameter.name)
}

/// The reverse of [`qualified`], for a key no parameter is literally named.
fn split_qualified(key: &str) -> Option<(Location, &str)> {
    for location in [
        Location::Path,
        Location::Query,
        Location::Header,
        Location::Cookie,
    ] {
        if let Some(name) = key
            .strip_prefix(label(location))
            .and_then(|rest| rest.strip_prefix(':'))
        {
            return Some((location, name));
        }
    }
    None
}

fn label(location: Location) -> &'static str {
    match location {
        Location::Path => "path",
        Location::Query => "query",
        Location::Header => "header",
        Location::Cookie => "cookie",
    }
}

/// Which parameters a supplied key can reach, best first.
///
/// A parameter's own name always wins: `location:name` is a spelling in the same
/// namespace as the names themselves, so a parameter literally named
/// `query:trace` would otherwise share a key with the query parameter `trace` and
/// one value would populate both. Only when no parameter answers to the key
/// literally is it read as a qualified spelling, and then it names exactly one.
fn candidates(parameters: &[Parameter], key: &str) -> Vec<usize> {
    let mut literal: Vec<usize> = parameters
        .iter()
        .enumerate()
        .filter(|(_, p)| p.name == key)
        .map(|(index, _)| index)
        .collect();
    if !literal.is_empty() {
        literal.sort_by_key(|index| rank(parameters[*index].location));
        return literal;
    }
    match split_qualified(key) {
        Some((location, name)) => parameters
            .iter()
            .enumerate()
            .filter(|(_, p)| p.location == location && p.name == name)
            .map(|(index, _)| index)
            .collect(),
        None => Vec::new(),
    }
}

/// The key a caller must supply to reach exactly this parameter: its bare name,
/// or its qualified spelling when a twin of the same name outranks it and would
/// take the bare key first. A refusal naming any other key would be one no
/// re-supply could satisfy.
fn key_for(parameters: &[Parameter], parameter: &Parameter) -> String {
    let outranked = parameters
        .iter()
        .any(|p| p.name == parameter.name && rank(p.location) < rank(parameter.location));
    if outranked {
        qualified(parameter)
    } else {
        parameter.name.clone()
    }
}

/// Which location a bare name reaches when a document declares one name in
/// several: the path first, because a path parameter is structural and a request
/// without it identifies nothing.
fn rank(location: Location) -> u8 {
    match location {
        Location::Path => 0,
        Location::Query => 1,
        Location::Header => 2,
        Location::Cookie => 3,
    }
}

/// A piece of the operation's path: text the document wrote, or a parameter whose
/// value is substituted at bind time.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Segment {
    Literal(String),
    Value(String),
}

fn segments(path: &str) -> std::result::Result<Vec<Segment>, Refusal> {
    let mut out = Vec::new();
    let mut rest = path;
    while let Some(open) = rest.find('{') {
        if open > 0 {
            out.push(Segment::Literal(rest[..open].to_owned()));
        }
        let tail = &rest[open + 1..];
        // An unterminated `{` is a malformed path, not an undeclared parameter:
        // the text after it names nothing, so the refusal names the path.
        let close = tail
            .find('}')
            .ok_or_else(|| Refusal::PathMalformed(path.to_owned()))?;
        out.push(Segment::Value(tail[..close].to_owned()));
        rest = &tail[close + 1..];
    }
    if !rest.is_empty() {
        out.push(Segment::Literal(rest.to_owned()));
    }
    Ok(out)
}

/// The effective parameter set: one entry per (name, location). A later
/// declaration of the same pair replaces the earlier one in place, which is how
/// OpenAPI 3.1 section 4.8.9.1 reads and what keeps the declared order stable.
fn effective(parameters: &[Parameter]) -> Vec<Parameter> {
    let mut out: Vec<Parameter> = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        match out
            .iter_mut()
            .find(|p| p.name == parameter.name && p.location == parameter.location)
        {
            Some(overridden) => *overridden = parameter.clone(),
            None => out.push(parameter.clone()),
        }
    }
    out
}

/// What one binding resolved to. Joining this to a host, a scheme and a credential
/// is the runtime's work, not the template's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub path: String,
    /// Name and value, both encoded for the query set, in the order the operation
    /// declares its parameters.
    pub query: Vec<(String, String)>,
    /// Header names and values as declared and supplied: a header is not part of
    /// the URL and is not percent-encoded, so instead nothing that would break the
    /// header line is permitted through at all.
    pub headers: Vec<(String, String)>,
    /// Absent when the operation offers no request media type at all.
    pub media_type: Option<String>,
}

impl Binding {
    /// The query as it would follow a `?`, empty when there are no pairs.
    pub fn query_string(&self) -> String {
        self.query
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("&")
    }
}

/// One operation, checked once, ready to bind values against repeatedly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Template {
    segments: Vec<Segment>,
    parameters: Vec<Parameter>,
    request_media_types: Vec<String>,
}

impl Template {
    /// Check what the operation alone decides: that it declares no cookie, that
    /// its header names and offered media types can be written into a header, and
    /// that its path and its path parameters account for each other exactly. A
    /// template that exists has passed all of them.
    pub fn from_operation(operation: &Operation) -> std::result::Result<Self, Refusal> {
        let parameters = effective(&operation.parameters);
        for parameter in &parameters {
            match parameter.location {
                Location::Cookie => {
                    return Err(Refusal::CookieUnsupported(parameter.name.clone()));
                }
                Location::Header if !token(&parameter.name) => {
                    return Err(Refusal::HeaderUnsafe(parameter.name.clone()));
                }
                _ => {}
            }
        }
        for media_type in &operation.request_media_types {
            if !media_type_writable(media_type) {
                return Err(Refusal::MediaTypeUnsafe(media_type.clone()));
            }
        }
        // Every parameter must have a key that reaches it. Otherwise a value for
        // it could never be supplied, and a refusal naming its key would be one
        // the caller cannot act on.
        for parameter in &parameters {
            let key = key_for(&parameters, parameter);
            let reached = candidates(&parameters, &key)
                .first()
                .map(|index| &parameters[*index]);
            if reached != Some(parameter) {
                return Err(Refusal::ParameterUnreachable(key));
            }
        }

        let segments = segments(&operation.path)?;
        for segment in &segments {
            let Segment::Value(name) = segment else {
                continue;
            };
            let declared = parameters
                .iter()
                .any(|p| p.location == Location::Path && &p.name == name);
            if !declared {
                return Err(Refusal::PlaceholderUndeclared(name.clone()));
            }
        }
        // The converse: a path parameter the path templates nowhere would take a
        // value from the caller and put it nowhere, which is the silent drop this
        // pass refuses for a cookie.
        for parameter in parameters.iter().filter(|p| p.location == Location::Path) {
            let placed = segments
                .iter()
                .any(|s| matches!(s, Segment::Value(name) if name == &parameter.name));
            if !placed {
                return Err(Refusal::PathParameterUnplaced(parameter.name.clone()));
            }
        }

        Ok(Self {
            segments,
            parameters,
            request_media_types: operation.request_media_types.clone(),
        })
    }

    /// Bind a map of parameter key to value, and the request media type the caller
    /// asks for — `None` selects the first the operation offers.
    ///
    /// A key is a parameter's own name, or `location:name` for a parameter no key
    /// is literally named. Every supplied value reaches exactly one parameter: a
    /// qualified key claims its own, and a bare name falls to the first location
    /// by [`rank`] that no qualified key has claimed. A key that would reach none
    /// is refused rather than dropped.
    pub fn bind(
        &self,
        values: &BTreeMap<String, String>,
        media_type: Option<&str>,
    ) -> std::result::Result<Binding, Refusal> {
        let assigned = self.assign(values)?;

        let mut path = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Literal(text) => path.push_str(text),
                Segment::Value(name) => path.push_str(&self.resolve(name, &assigned)?),
            }
        }

        let mut query = Vec::new();
        let mut headers = Vec::new();
        // Declared order, not the supplied map's: the output order is the
        // document's and does not move when a caller reorders its own values.
        for (index, parameter) in self.parameters.iter().enumerate() {
            let carried = match assigned[index] {
                Some(value) => value,
                None if parameter.required => {
                    return Err(Refusal::ValueAbsent(key_for(&self.parameters, parameter)));
                }
                None => continue,
            };
            match parameter.location {
                Location::Query => query.push((
                    encode(&parameter.name, QUERY_KEEP),
                    encode(carried, QUERY_KEEP),
                )),
                Location::Header => {
                    if !field_value_safe(carried) {
                        return Err(Refusal::HeaderUnsafe(parameter.name.clone()));
                    }
                    headers.push((parameter.name.clone(), carried.clone()));
                }
                // Path parameters are already in the path; cookies never reach here.
                Location::Path | Location::Cookie => {}
            }
        }

        Ok(Binding {
            path,
            query,
            headers,
            media_type: self.media_type(media_type)?,
        })
    }

    /// Which value each declared parameter is bound to, by position. Qualified
    /// keys are read first because each names exactly one parameter; a bare name
    /// then takes the best location still free, so no two keys share a parameter
    /// and no key is read by none.
    fn assign<'a>(
        &self,
        values: &'a BTreeMap<String, String>,
    ) -> std::result::Result<Vec<Option<&'a String>>, Refusal> {
        let mut assigned: Vec<Option<&String>> = vec![None; self.parameters.len()];
        for key in values.keys() {
            if candidates(&self.parameters, key).is_empty() {
                return Err(Refusal::ParameterUndeclared(key.clone()));
            }
        }
        let literal = |key: &String| self.parameters.iter().any(|p| &p.name == key);
        for (key, value) in values.iter().filter(|(key, _)| !literal(key)) {
            let index = candidates(&self.parameters, key)[0];
            assigned[index] = Some(value);
        }
        for (key, value) in values.iter().filter(|(key, _)| literal(key)) {
            let index = candidates(&self.parameters, key)
                .into_iter()
                .find(|index| assigned[*index].is_none())
                .ok_or_else(|| Refusal::KeyUnread(key.clone()))?;
            assigned[index] = Some(value);
        }
        Ok(assigned)
    }

    /// One path placeholder's replacement: present, non-empty, and unable to move
    /// the path once a client resolves it.
    fn resolve(
        &self,
        name: &str,
        assigned: &[Option<&String>],
    ) -> std::result::Result<String, Refusal> {
        let (index, parameter) = self
            .parameters
            .iter()
            .enumerate()
            .find(|(_, p)| p.location == Location::Path && p.name == name)
            .ok_or_else(|| Refusal::PlaceholderUndeclared(name.to_owned()))?;
        let value = assigned[index]
            .ok_or_else(|| Refusal::ValueAbsent(key_for(&self.parameters, parameter)))?;
        if value.is_empty() {
            return Err(Refusal::PathValueEmpty(name.to_owned()));
        }
        let encoded = encode(value, PATH_KEEP);
        if encoded == "." || encoded == ".." {
            return Err(Refusal::PathValueDotSegment(name.to_owned()));
        }
        Ok(encoded)
    }

    fn media_type(&self, asked: Option<&str>) -> std::result::Result<Option<String>, Refusal> {
        match asked {
            Some(wanted) => {
                if self.request_media_types.iter().any(|m| m == wanted) {
                    Ok(Some(wanted.to_owned()))
                } else {
                    Err(Refusal::MediaTypeUnoffered(wanted.to_owned()))
                }
            }
            None => Ok(self.request_media_types.first().cloned()),
        }
    }
}
