//! Configured line and trace profiles (`contracts/logs/v1alpha1/semantics.md` §12),
//! modelled as `connectors_loki.reads.LineProfile` and `TraceProfile` in
//! `spec/ess/domains/reads.yaml`.
//!
//! An operator writes a profile in the connection's configuration; the caller only
//! names it. This module admits a profile (refusing everything the model refuses and
//! the ESS-LIMIT it records), composes its LogQL from the structured selector, and
//! reads a selected line into the profile's typed, masked fields. The one caller
//! value that reaches LogQL, a call identifier or a matched field value, is rendered
//! only as a LogQL string literal.
use connectors_core::{Error, ErrorCode, Result};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Profiles of one kind per configuration (the model's `count <= 16`).
pub const PROFILES: usize = 16;
/// The longest field text a record carries; a longer value does not read.
pub const FIELD_BYTES: usize = 4096;
/// What a `redact` mask answers in place of the value.
pub const REDACTED: &str = "<redacted>";

fn invalid() -> Error {
    Error::invalid("invalid profile configuration")
}

fn not_configured() -> Error {
    Error::new(ErrorCode::NotFound, "the profile is not configured")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Equal,
    NotEqual,
    Matches,
    NotMatches,
}

impl Operator {
    fn logql(self) -> &'static str {
        match self {
            Operator::Equal => "=",
            Operator::NotEqual => "!=",
            Operator::Matches => "=~",
            Operator::NotMatches => "!~",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Matcher {
    label: String,
    operator: Operator,
    value: String,
}

impl Matcher {
    fn logql(&self) -> String {
        format!(
            "{}{}{}",
            self.label,
            self.operator.logql(),
            string_literal(&self.value)
        )
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PatternDocument {
    pattern: String,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum FormatDocument {
    Logfmt,
    Pattern(PatternDocument),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FieldType {
    String,
    Integer,
    PhoneNumber,
    UnixSeconds,
    UnixMillis,
    UnixNanos,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mask {
    None,
    Redact,
    LastFour,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldDocument {
    name: String,
    key: String,
    #[serde(rename = "type")]
    kind: FieldType,
    mask: Option<Mask>,
    optional: Option<bool>,
    filterable: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LineDocument {
    name: String,
    selector: Vec<Matcher>,
    format: FormatDocument,
    fields: Vec<FieldDocument>,
    call_identifier: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LabelDocument {
    label: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldSourceDocument {
    key: String,
    format: FormatDocument,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum CallDocument {
    Label(LabelDocument),
    Field(FieldSourceDocument),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceDocument {
    name: String,
    selector: Vec<Matcher>,
    call_identifier: CallDocument,
    source_label: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Equality {
    label: String,
    value: String,
}

/// One piece of a pattern: literal text, or a capture (`None` for `<_>`).
#[derive(Clone, Debug)]
enum Segment {
    Literal(String),
    Capture(Option<String>),
}

#[derive(Clone, Debug)]
enum Format {
    Logfmt,
    /// The pattern as configured, and its segments.
    Pattern(String, Vec<Segment>),
}

impl Format {
    fn admit(document: FormatDocument) -> Result<Self> {
        match document {
            FormatDocument::Logfmt => Ok(Format::Logfmt),
            FormatDocument::Pattern(p) => {
                let count = p.pattern.chars().count();
                if !(3..=1024).contains(&count) {
                    return Err(invalid());
                }
                let segments = pattern(&p.pattern)?;
                Ok(Format::Pattern(p.pattern, segments))
            }
        }
    }

    /// Whether `key` is a value this format can produce.
    fn produces(&self, key: &str) -> bool {
        match self {
            Format::Logfmt => true,
            Format::Pattern(_, segments) => segments
                .iter()
                .any(|s| matches!(s, Segment::Capture(Some(name)) if name == key)),
        }
    }

    /// The named values of one line, or `None` when the line does not read.
    fn read(&self, line: &str) -> Option<BTreeMap<String, String>> {
        match self {
            Format::Logfmt => logfmt(line),
            Format::Pattern(_, segments) => matched(segments, line),
        }
    }

    /// The LogQL parser stage that extracts this format's keys as labels, so an
    /// exact label filter can follow it.
    fn stage(&self) -> String {
        match self {
            Format::Logfmt => "| logfmt".to_owned(),
            Format::Pattern(source, _) => format!("| pattern {}", string_literal(source)),
        }
    }
}

/// `| <key>="<value>"` after a parser stage: Loki keeps a line only when the
/// parsed key is exactly the value. Never a substring line filter (§12.2).
fn exact(key: &str, value: &str) -> String {
    format!(" | {key}={}", string_literal(value))
}

/// A key a label filter may name after a parser stage: a label name, since Loki
/// rewrites any other key, and not a `__` name, which Loki keeps for itself.
fn matched_key(text: &str) -> bool {
    label_name(text) && !text.starts_with("__")
}

#[derive(Clone, Debug)]
struct Field {
    name: String,
    key: String,
    kind: FieldType,
    mask: Mask,
    optional: bool,
    filterable: bool,
}

/// An admitted line profile.
#[derive(Clone, Debug)]
pub struct LineProfile {
    pub name: String,
    selector: Vec<Matcher>,
    format: Format,
    fields: Vec<Field>,
    call_identifier: usize,
}

#[derive(Clone, Debug)]
enum CallSource {
    Label(String),
    Field { key: String, format: Format },
}

/// An admitted trace profile.
#[derive(Clone, Debug)]
pub struct TraceProfile {
    pub name: String,
    selector: Vec<Matcher>,
    call: CallSource,
    source_label: String,
}

/// Every admitted profile of one configuration.
#[derive(Clone, Debug, Default)]
pub struct Profiles {
    pub line: Vec<LineProfile>,
    pub trace: Vec<TraceProfile>,
}

/// A Loki label name: `[A-Za-z_][A-Za-z0-9_]*`, 1–128 bytes.
fn label_name(text: &str) -> bool {
    let mut bytes = text.bytes();
    text.len() <= 128
        && bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn profile_name(text: &str) -> bool {
    (1..=64).contains(&text.len())
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
}

fn field_name(text: &str) -> bool {
    (1..=64).contains(&text.len())
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn key(text: &str) -> bool {
    (1..=128).contains(&text.chars().count())
}

fn selector(matchers: Vec<Matcher>, scope: &[Equality]) -> Result<Vec<Matcher>> {
    if !(1..=16).contains(&matchers.len())
        || matchers
            .iter()
            .any(|m| !label_name(&m.label) || m.value.chars().count() > 1024)
        // Loki refuses a selector that every stream matches; one nonempty equality
        // is what the model requires.
        || !matchers
            .iter()
            .any(|m| m.operator == Operator::Equal && !m.value.is_empty())
    {
        return Err(invalid());
    }
    // Every configured equality is one of the selector's exact equalities, so the
    // profile reads no stream the scope does not admit (§12.3). Extra matchers only
    // narrow: a selector's matchers all hold of every stream it selects.
    for equality in scope {
        if !matchers.iter().any(|m| {
            m.operator == Operator::Equal && m.label == equality.label && m.value == equality.value
        }) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "a profile does not carry the configured scope",
            ));
        }
    }
    Ok(matchers)
}

/// Parse a pattern: literal text and `<name>` captures, `<_>` discarding. A `<` that
/// does not open a capture is literal text.
fn pattern(text: &str) -> Result<Vec<Segment>> {
    let mut segments: Vec<Segment> = Vec::new();
    let mut literal = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '<'
            && let Some(close) = rest.find('>')
            && (rest[1..close] == *"_" || label_name(&rest[1..close]))
        {
            let name = &rest[1..close];
            if !literal.is_empty() {
                segments.push(Segment::Literal(std::mem::take(&mut literal)));
            }
            if matches!(segments.last(), Some(Segment::Capture(_))) {
                // Two adjacent captures have no boundary between them.
                return Err(invalid());
            }
            segments.push(Segment::Capture((name != "_").then(|| name.to_owned())));
            rest = &rest[close + 1..];
            continue;
        }
        literal.push(c);
        rest = &rest[c.len_utf8()..];
    }
    if !literal.is_empty() {
        segments.push(Segment::Literal(literal));
    }
    let mut names = BTreeSet::new();
    for segment in &segments {
        if let Segment::Capture(Some(name)) = segment
            && !names.insert(name.clone())
        {
            return Err(invalid());
        }
    }
    if names.is_empty() {
        return Err(invalid());
    }
    Ok(segments)
}

fn line_profile(document: LineDocument, scope: &[Equality]) -> Result<LineProfile> {
    if !profile_name(&document.name) || !(1..=64).contains(&document.fields.len()) {
        return Err(invalid());
    }
    let selector = selector(document.selector, scope)?;
    let format = Format::admit(document.format)?;
    let mut names = BTreeSet::new();
    let mut fields = Vec::with_capacity(document.fields.len());
    for field in document.fields {
        let text = matches!(field.kind, FieldType::String | FieldType::PhoneNumber);
        let mask = match (field.mask, field.kind) {
            (Some(mask), _) => mask,
            (None, FieldType::PhoneNumber) => Mask::LastFour,
            (None, _) => Mask::None,
        };
        if !field_name(&field.name)
            || !key(&field.key)
            || !names.insert(field.name.clone())
            || !format.produces(&field.key)
            // Only text is masked: a masked integer or timestamp is no longer one.
            || (mask != Mask::None && !text)
            // A matched key is named by an exact label filter after the parser, and
            // a stream label of its name would shadow the parsed value there.
            || (field.filterable == Some(true)
                && (!matched_key(&field.key) || selector.iter().any(|m| m.label == field.key)))
        {
            return Err(invalid());
        }
        fields.push(Field {
            name: field.name,
            key: field.key,
            kind: field.kind,
            mask,
            optional: field.optional.unwrap_or(false),
            filterable: field.filterable.unwrap_or(false),
        });
    }
    // The call identifier is the join key: present, unmasked text.
    let call_identifier = fields
        .iter()
        .position(|f| {
            f.name == document.call_identifier
                && f.kind == FieldType::String
                && f.mask == Mask::None
                && !f.optional
        })
        .ok_or_else(invalid)?;
    Ok(LineProfile {
        name: document.name,
        selector,
        format,
        fields,
        call_identifier,
    })
}

fn trace_profile(document: TraceDocument, scope: &[Equality]) -> Result<TraceProfile> {
    if !profile_name(&document.name) || !label_name(&document.source_label) {
        return Err(invalid());
    }
    let selector = selector(document.selector, scope)?;
    let call = match document.call_identifier {
        CallDocument::Label(l) => {
            // The composed equality is the only matcher on the call label, and the
            // source is another label.
            if !label_name(&l.label)
                || l.label == document.source_label
                || selector.iter().any(|m| m.label == l.label)
            {
                return Err(invalid());
            }
            CallSource::Label(l.label)
        }
        CallDocument::Field(f) => {
            let format = Format::admit(f.format)?;
            // The key is named by the exact label filter after the parser; a stream
            // label of its name, the source label or a selected one, would shadow it.
            if !matched_key(&f.key)
                || !format.produces(&f.key)
                || f.key == document.source_label
                || selector.iter().any(|m| m.label == f.key)
            {
                return Err(invalid());
            }
            CallSource::Field { key: f.key, format }
        }
    };
    Ok(TraceProfile {
        name: document.name,
        selector,
        call,
        source_label: document.source_label,
    })
}

fn distinct<'a>(names: impl Iterator<Item = &'a String>) -> bool {
    let mut seen = BTreeSet::new();
    names.into_iter().all(|n| seen.insert(n))
}

/// Admit the configured profiles under the configured scope's equalities. A refusal is
/// InvalidInput (a malformed profile) or Forbidden (a profile outside the scope); the
/// composition reports either as an invalid configuration.
pub fn admit(line: &[Value], trace: &[Value], scope: &[Value]) -> Result<Profiles> {
    if line.len() > PROFILES || trace.len() > PROFILES {
        return Err(invalid());
    }
    let scope: Vec<Equality> = scope
        .iter()
        .map(|e| serde_json::from_value(e.clone()).map_err(|_| invalid()))
        .collect::<Result<_>>()?;
    let line = line
        .iter()
        .map(|p| {
            line_profile(
                serde_json::from_value(p.clone()).map_err(|_| invalid())?,
                &scope,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let trace = trace
        .iter()
        .map(|p| {
            trace_profile(
                serde_json::from_value(p.clone()).map_err(|_| invalid())?,
                &scope,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    if !distinct(line.iter().map(|p| &p.name)) || !distinct(trace.iter().map(|p| &p.name)) {
        return Err(invalid());
    }
    Ok(Profiles { line, trace })
}

/// The profiles an effective configuration carries, under its `query_scope`.
pub fn from_configuration(configuration: &Value) -> Result<Profiles> {
    let list = |name: &str| -> Result<Vec<Value>> {
        match configuration.get(name) {
            None => Ok(Vec::new()),
            Some(Value::Array(items)) if !items.is_empty() => Ok(items.clone()),
            Some(_) => Err(invalid()),
        }
    };
    let scope = configuration
        .pointer("/query_scope/required_equalities")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    admit(&list("line_profiles")?, &list("trace_profiles")?, scope)
}

impl Profiles {
    pub fn line(&self, name: &str) -> Result<&LineProfile> {
        self.line
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(not_configured)
    }
    pub fn trace(&self, name: &str) -> Result<&TraceProfile> {
        self.trace
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(not_configured)
    }
}

fn braces(matchers: impl Iterator<Item = String>) -> String {
    format!("{{{}}}", matchers.collect::<Vec<_>>().join(","))
}

/// A caller's exact match on one filterable field.
pub struct Match {
    field: usize,
    value: String,
}

/// One line read as a profile: its call identifier and typed, masked fields.
pub struct Record {
    pub call_id: String,
    pub fields: Map<String, Value>,
}

/// What one selected line is to a line profile read.
pub enum Read {
    /// The line reads, holding every matched value exactly.
    Record(Record),
    /// The line does not read as the profile, and either there is no match or it
    /// holds every matched value exactly: counted under `skipped.unreadable`.
    Unreadable,
    /// The line does not hold every matched value exactly, or there is a match
    /// and its format does not read at all. Dropped and counted nowhere, so no
    /// count in the answer reflects a value other than the caller's exact one.
    Unmatched,
}

impl LineProfile {
    /// The stream selector, as LogQL.
    pub fn selector(&self) -> String {
        braces(self.selector.iter().map(Matcher::logql))
    }

    /// Admit the caller's matches: each names a field this profile declares
    /// filterable, with a nonempty value.
    pub fn matches(&self, requested: &BTreeMap<String, String>) -> Result<Vec<Match>> {
        requested
            .iter()
            .map(|(name, value)| {
                let field = self
                    .fields
                    .iter()
                    .position(|f| &f.name == name && f.filterable)
                    .ok_or_else(|| Error::invalid("match names no filterable field"))?;
                if value.is_empty() {
                    return Err(Error::invalid("a match value is empty"));
                }
                Ok(Match {
                    field,
                    value: value.clone(),
                })
            })
            .collect()
    }

    /// The selector; with matches, the format's parser stage and one exact label
    /// filter per match, so Loki answers only lines whose parsed field is exactly
    /// the value (§12.2). No match is ever a substring line filter.
    pub fn query(&self, matches: &[Match]) -> String {
        let mut query = self.selector();
        if !matches.is_empty() {
            query.push(' ');
            query.push_str(&self.format.stage());
            for m in matches {
                query.push_str(&exact(&self.fields[m.field].key, &m.value));
            }
        }
        query
    }

    /// Read one selected line. The matches are checked again, on the adapter's own
    /// reading, before anything about the line is counted.
    pub fn read(&self, line: &str, matches: &[Match]) -> Read {
        let Some(values) = self.format.read(line) else {
            return if matches.is_empty() {
                Read::Unreadable
            } else {
                Read::Unmatched
            };
        };
        if matches
            .iter()
            .any(|m| values.get(&self.fields[m.field].key) != Some(&m.value))
        {
            return Read::Unmatched;
        }
        let mut fields = Map::new();
        for field in &self.fields {
            let value = match values.get(&field.key) {
                None if field.optional => Value::Null,
                None => return Read::Unreadable,
                Some(text) => match typed(field, text) {
                    Some(value) => value,
                    None => return Read::Unreadable,
                },
            };
            fields.insert(field.name.clone(), value);
        }
        let call_id = values[&self.fields[self.call_identifier].key].clone();
        Read::Record(Record { call_id, fields })
    }
}

impl TraceProfile {
    /// The selector for one call: the profile's matchers, the source label required
    /// present and, for a label-carried identifier, its equality.
    pub fn selector_for(&self, call_id: &str) -> String {
        let mut matchers: Vec<String> = self.selector.iter().map(Matcher::logql).collect();
        matchers.push(format!("{}=~\".+\"", self.source_label));
        if let CallSource::Label(label) = &self.call {
            matchers.push(format!("{label}={}", string_literal(call_id)));
        }
        braces(matchers.into_iter())
    }

    /// The whole query for one call: for a line-carried identifier, the format's
    /// parser stage and an exact label filter on its key (§12.3), never a substring.
    pub fn query(&self, call_id: &str) -> String {
        let mut query = self.selector_for(call_id);
        if let CallSource::Field { key, format } = &self.call {
            query.push(' ');
            query.push_str(&format.stage());
            query.push_str(&exact(key, call_id));
        }
        query
    }

    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    /// The label carrying the call identifier, when a label carries it.
    pub fn call_label(&self) -> Option<&str> {
        match &self.call {
            CallSource::Label(label) => Some(label),
            CallSource::Field { .. } => None,
        }
    }

    /// Whether one selected line is this call's: always for a label-carried
    /// identifier (the stream's label is checked by the caller), and for a field
    /// only when the line reads and that field is exactly the identifier.
    pub fn carries(&self, line: &str, call_id: &str) -> bool {
        match &self.call {
            CallSource::Label(_) => true,
            CallSource::Field { key, format } => format
                .read(line)
                .is_some_and(|values| values.get(key).map(String::as_str) == Some(call_id)),
        }
    }
}

/// A field's text as its declared type, masked as declared; `None` does not read.
fn typed(field: &Field, text: &str) -> Option<Value> {
    if text.len() > FIELD_BYTES {
        return None;
    }
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    let nanoseconds = |whole: &str, scale: u64, fraction: &str| -> Option<Value> {
        let whole: u64 = whole.parse().ok()?;
        let fraction: u64 = if fraction.is_empty() {
            0
        } else {
            format!("{fraction:0<9}").parse().ok()?
        };
        whole
            .checked_mul(scale)?
            .checked_add(fraction)
            .filter(|n| *n <= i64::MAX as u64)
            .map(|n| Value::String(n.to_string()))
    };
    match field.kind {
        FieldType::String | FieldType::PhoneNumber => Some(Value::String(mask(field.mask, text))),
        FieldType::Integer => {
            let unsigned = text.strip_prefix('-').unwrap_or(text);
            if !digits(unsigned) {
                return None;
            }
            text.parse::<i64>().ok().map(|n| json!(n))
        }
        FieldType::UnixSeconds => {
            let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
            if !digits(whole) || (text.contains('.') && !digits(fraction)) || fraction.len() > 9 {
                return None;
            }
            nanoseconds(whole, 1_000_000_000, fraction)
        }
        FieldType::UnixMillis if digits(text) => nanoseconds(text, 1_000_000, ""),
        FieldType::UnixNanos if digits(text) => nanoseconds(text, 1, ""),
        FieldType::UnixMillis | FieldType::UnixNanos => None,
    }
}

fn mask(mask: Mask, text: &str) -> String {
    match mask {
        Mask::None => text.to_owned(),
        Mask::Redact => REDACTED.to_owned(),
        Mask::LastFour => {
            let count = text.chars().count();
            let keep = if count > 4 { 4 } else { 0 };
            text.chars()
                .enumerate()
                .map(|(i, c)| if i + keep < count { '*' } else { c })
                .collect()
        }
    }
}

/// `value` as one LogQL string literal: a double-quoted string whose quote,
/// backslash and control characters are escaped, so no caller text can end the
/// literal or add a matcher, a filter or a stage.
pub fn string_literal(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Read a logfmt line: `key=value`, `key="quoted value"` or a bare `key` (empty
/// value), separated by spaces or tabs. An unterminated quote, an unknown escape, a
/// quote inside a bare value, an empty key or a key written twice does not read.
fn logfmt(line: &str) -> Option<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    let mut chars = line.chars().peekable();
    loop {
        while chars.peek().is_some_and(|c| *c == ' ' || *c == '\t') {
            chars.next();
        }
        if chars.peek().is_none() {
            return Some(values);
        }
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c == ' ' || c == '\t' {
                break;
            }
            if c == '"' || c.is_control() {
                return None;
            }
            key.push(c);
            chars.next();
        }
        if key.is_empty() {
            return None;
        }
        let mut value = String::new();
        if chars.peek() == Some(&'=') {
            chars.next();
            if chars.peek() == Some(&'"') {
                chars.next();
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => match chars.next()? {
                            '"' => value.push('"'),
                            '\\' => value.push('\\'),
                            'n' => value.push('\n'),
                            'r' => value.push('\r'),
                            't' => value.push('\t'),
                            'u' => {
                                let hex: String = (0..4).filter_map(|_| chars.next()).collect();
                                if hex.len() != 4 {
                                    return None;
                                }
                                value.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                            }
                            _ => return None,
                        },
                        c => value.push(c),
                    }
                }
                if chars.peek().is_some_and(|c| *c != ' ' && *c != '\t') {
                    return None;
                }
            } else {
                while let Some(&c) = chars.peek() {
                    if c == ' ' || c == '\t' {
                        break;
                    }
                    if c == '"' {
                        return None;
                    }
                    value.push(c);
                    chars.next();
                }
            }
        }
        if values.insert(key, value).is_some() {
            return None;
        }
    }
}

/// Match a line against a pattern from its start: a literal must follow exactly, a
/// capture runs to the next literal's first occurrence or, last, to the line's end.
fn matched(segments: &[Segment], line: &str) -> Option<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    let mut position = 0;
    for (index, segment) in segments.iter().enumerate() {
        match segment {
            Segment::Literal(text) => {
                if !line[position..].starts_with(text.as_str()) {
                    return None;
                }
                position += text.len();
            }
            Segment::Capture(name) => {
                let end = match segments.get(index + 1) {
                    Some(Segment::Literal(next)) => {
                        position + line[position..].find(next.as_str())?
                    }
                    _ => line.len(),
                };
                if let Some(name) = name {
                    values.insert(name.clone(), line[position..end].to_owned());
                }
                position = end;
            }
        }
    }
    Some(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logfmt_reads_bare_quoted_and_escaped_values_and_refuses_ambiguity() {
        let read = logfmt(r#"a=1 b="two words" c="q\"\\\nA" d"#).unwrap();
        assert_eq!(read["a"], "1");
        assert_eq!(read["b"], "two words");
        assert_eq!(read["c"], "q\"\\\nA");
        assert_eq!(read["d"], "");
        for bad in [
            r#"a="open"#,
            "a=1 a=2",
            r#"a=x"y"#,
            "=1",
            r#"a="x\q""#,
            r#"a="x"b"#,
        ] {
            assert!(logfmt(bad).is_none(), "{bad}");
        }
        // Bare, quoted and escaped spellings of one value read back as it.
        for (line, value) in [
            ("x=plain", "plain"),
            (r#"x="a b""#, "a b"),
            (r#"x="q\"""#, "q\""),
            (r#"x="back\\slash""#, "back\\slash"),
            (r#"x="nl\n""#, "nl\n"),
            (r#"x="nul\u0000""#, "nul\u{0}"),
            (r#"x="""#, ""),
            ("x=", ""),
            ("x=k=v", "k=v"),
            (r#"x="k=v""#, "k=v"),
        ] {
            assert_eq!(logfmt(line).unwrap()["x"], value, "{line}");
        }
    }

    #[test]
    fn a_pattern_matches_from_the_start_and_captures_to_the_next_literal() {
        let segments = pattern("<_> [<level>] <message>").unwrap();
        let read = matched(&segments, "t0 [warn] a [b] c").unwrap();
        assert_eq!(read["level"], "warn");
        assert_eq!(read["message"], "a [b] c");
        assert!(!read.contains_key("_"));
        let segments = pattern("id=<id> <sip:x>").unwrap();
        assert!(matched(&segments, "id=7 <sip:x>").is_some());
        assert!(matched(&segments, "xid=7 <sip:x>").is_none());
        assert!(pattern("<a><b>").is_err());
        assert!(pattern("<a> <a>").is_err());
        assert!(pattern("no captures <_>").is_err());
    }

    #[test]
    fn masks_hide_all_but_what_they_declare() {
        assert_eq!(mask(Mask::LastFour, "+15550100001"), "********0001");
        assert_eq!(mask(Mask::LastFour, "1234"), "****");
        assert_eq!(mask(Mask::Redact, "+15550100001"), REDACTED);
        assert_eq!(mask(Mask::None, "x"), "x");
    }
}
