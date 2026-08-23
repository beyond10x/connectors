//! How one command's result reaches a terminal, a pipe, or a parser.
//!
//! # Why a failure is not always on stderr
//!
//! A person reads `text`; a script reads `json`. The two want opposite things from a failure. On a
//! terminal, an error belongs on stderr so it does not corrupt the thing being read. In `json` or
//! `yaml` the caller has already committed to parsing stdout, and a failure written to stderr
//! leaves that parser reading empty input — so it reports "unexpected end of JSON" for what was
//! really "the token was rejected". The refusal that names itself is the whole point of this
//! component's error vocabulary, and it would be lost at the last inch.
//!
//! So the rule is: **structured formats put the error envelope on stdout, in the requested format.**
//! `fluxplane-plugin` reached the same conclusion under `--result-only`, and `dex` documents it as
//! the contract for `-o json`. The exit code is non-zero either way; nothing here changes that.

use std::fmt::Write as _;
use std::io::{self, Write as _};

use serde_json::{Map, Value};

/// The rendering a caller asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
#[value(rename_all = "lower")]
pub enum Format {
    /// Indented key-and-value text for a person.
    #[default]
    Text,
    /// One record per line, for `grep` and `wc -l`.
    Compact,
    /// Pretty-printed JSON.
    Json,
    /// YAML.
    Yaml,
}

impl Format {
    /// Whether a failure belongs on stdout, inside the envelope, rather than on stderr.
    ///
    /// See this module's documentation: a caller parsing stdout must receive the refusal there or
    /// it sees an empty stream instead of a named reason.
    #[must_use]
    pub const fn errors_on_stdout(self) -> bool {
        matches!(self, Self::Json | Self::Yaml)
    }
}

/// Render one successful result.
///
/// # Errors
///
/// Serialization failure for the selected format, or a closed stdout.
pub fn emit(format: Format, value: &Value) -> Result<(), OutputError> {
    let rendered = render(format, value)?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(rendered.as_bytes())?;
    if !rendered.ends_with('\n') {
        stdout.write_all(b"\n")?;
    }
    stdout.flush()?;
    Ok(())
}

/// Render one failure, choosing the stream the caller can actually read.
///
/// `code` names a class of fault and never carries a credential; `message` is the human sentence.
pub fn emit_error(format: Format, code: &str, message: &str) {
    if format.errors_on_stdout() {
        let envelope = Value::Object(Map::from_iter([(
            "error".to_owned(),
            Value::Object(Map::from_iter([
                ("code".to_owned(), Value::String(code.to_owned())),
                ("message".to_owned(), Value::String(message.to_owned())),
            ])),
        )]));
        // A failure to render the failure still has to say something, and it must not be silent.
        match render(format, &envelope) {
            Ok(rendered) => println!("{}", rendered.trim_end()),
            Err(_) => eprintln!("{code}: {message}"),
        }
        return;
    }
    eprintln!("{code}: {message}");
}

/// Strip the result-discriminant wrapper the three protocols tag their results with.
///
/// Every result serializes as `{"result": "<variant>", "value": {…}}` — an internally tagged enum,
/// which is right on the wire because a reader has to know which variant arrived. A person at a
/// terminal already knows: they typed the subcommand. Leaving it in costs two levels of indentation
/// in `text` and, worse, defeats `compact` entirely — the top level would be one object with no
/// array in it, so every list rendered as the single line `result=candidate_search`.
///
/// Conservative on purpose: only an object with *exactly* these two keys is unwrapped, so a payload
/// that happens to carry a `value` field of its own is left alone.
#[must_use]
pub fn payload(value: Value) -> Value {
    let Value::Object(fields) = &value else {
        return value;
    };
    if fields.len() == 2 && fields.contains_key("result") && fields.contains_key("value") {
        if let Value::Object(mut fields) = value {
            return fields.remove("value").unwrap_or(Value::Null);
        }
    }
    value
}

fn render(format: Format, value: &Value) -> Result<String, OutputError> {
    match format {
        Format::Json => Ok(serde_json::to_string_pretty(value)?),
        Format::Yaml => serde_norway::to_string(value).map_err(|_| OutputError::Yaml),
        Format::Compact => Ok(compact(value)),
        Format::Text => {
            let mut buffer = String::new();
            text(value, 0, &mut buffer);
            Ok(buffer)
        }
    }
}

/// One record per line.
///
/// The interesting case is the shape every list response in this component has: one object whose
/// single array field carries the records. Rendering that as one line — the object — would defeat
/// the format, so the array is unwrapped first and each element becomes its own line.
fn compact(value: &Value) -> String {
    match unwrap_single_array(value) {
        Some(items) => items
            .iter()
            .map(compact_scalar_line)
            .collect::<Vec<_>>()
            .join("\n"),
        None => compact_scalar_line(value),
    }
}

fn unwrap_single_array(value: &Value) -> Option<&Vec<Value>> {
    match value {
        Value::Array(items) => Some(items),
        Value::Object(fields) => {
            let mut arrays = fields.values().filter_map(Value::as_array);
            let only = arrays.next()?;
            arrays.next().is_none().then_some(only)
        }
        _ => None,
    }
}

fn compact_scalar_line(value: &Value) -> String {
    match value {
        Value::Object(fields) => {
            let mut line = String::new();
            for (key, field) in fields {
                if matches!(field, Value::Array(_) | Value::Object(_)) {
                    continue;
                }
                if !line.is_empty() {
                    line.push('\t');
                }
                let _ = write!(line, "{key}={}", scalar(field));
            }
            line
        }
        other => scalar(other),
    }
}

fn text(value: &Value, depth: usize, buffer: &mut String) {
    let pad = "  ".repeat(depth);
    match value {
        Value::Object(fields) => {
            for (key, field) in fields {
                match field {
                    Value::Object(inner) if inner.is_empty() => {
                        let _ = writeln!(buffer, "{pad}{key}:");
                    }
                    Value::Array(items) if items.is_empty() => {
                        let _ = writeln!(buffer, "{pad}{key}: (none)");
                    }
                    Value::Object(_) | Value::Array(_) => {
                        let _ = writeln!(buffer, "{pad}{key}:");
                        text(field, depth + 1, buffer);
                    }
                    scalar_field => {
                        let _ = writeln!(buffer, "{pad}{key}: {}", scalar(scalar_field));
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                match item {
                    Value::Object(_) | Value::Array(_) => {
                        let _ = writeln!(buffer, "{pad}-");
                        text(item, depth + 1, buffer);
                    }
                    scalar_item => {
                        let _ = writeln!(buffer, "{pad}- {}", scalar(scalar_item));
                    }
                }
            }
        }
        scalar_value => {
            let _ = writeln!(buffer, "{pad}{}", scalar(scalar_value));
        }
    }
}

/// A scalar without JSON's quotes, which a person reading a terminal did not ask for.
fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => "null".to_owned(),
        other => other.to_string(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("the result could not be written: {0}")]
    Io(#[from] io::Error),
    #[error("the result could not be rendered as JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the result could not be rendered as YAML")]
    Yaml,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_structured_format_carries_its_failure_on_stdout() {
        // The distinction this asserts is the module's reason for existing: a parser reading stdout
        // must find the refusal there, not an empty stream.
        assert!(Format::Json.errors_on_stdout());
        assert!(Format::Yaml.errors_on_stdout());
        assert!(!Format::Text.errors_on_stdout());
        assert!(!Format::Compact.errors_on_stdout());
    }

    #[test]
    fn compact_unwraps_the_one_array_a_list_response_carries() {
        let response = json!({"connections": [
            {"connection_ref": "connection:slack:a", "label": "Slack"},
            {"connection_ref": "connection:slack:b", "label": "Other"},
        ]});
        let rendered = render(Format::Compact, &response).unwrap();
        assert_eq!(rendered.lines().count(), 2);
        assert!(rendered.starts_with("connection_ref=connection:slack:a\tlabel=Slack"));
    }

    #[test]
    fn compact_leaves_a_single_record_as_one_line() {
        let rendered = render(
            Format::Compact,
            &json!({"ready": true, "socket": "/x.sock"}),
        )
        .unwrap();
        assert_eq!(rendered, "ready=true\tsocket=/x.sock");
    }

    #[test]
    fn an_object_with_two_arrays_is_not_unwrapped() {
        // Guessing which array is "the records" would be wrong as often as right.
        let response = json!({"connections": [{"a": 1}], "observations": [{"b": 2}]});
        assert!(unwrap_single_array(&response).is_none());
    }

    #[test]
    fn text_does_not_quote_a_string_a_person_is_reading() {
        let rendered = render(Format::Text, &json!({"label": "Development cluster"})).unwrap();
        assert_eq!(rendered, "label: Development cluster\n");
    }

    #[test]
    fn text_says_none_rather_than_printing_an_empty_bracket() {
        let rendered = render(Format::Text, &json!({"events": []})).unwrap();
        assert_eq!(rendered, "events: (none)\n");
    }

    #[test]
    fn the_result_discriminant_is_stripped_so_compact_can_see_the_records() {
        let tagged =
            json!({"result": "candidate_search", "value": {"candidates": [{"a": 1}, {"a": 2}]}});
        let stripped = payload(tagged);
        assert_eq!(stripped, json!({"candidates": [{"a": 1}, {"a": 2}]}));
        assert_eq!(
            render(Format::Compact, &stripped).unwrap().lines().count(),
            2
        );
    }

    #[test]
    fn a_payload_carrying_its_own_value_field_is_left_alone() {
        // Unwrapping on the presence of `value` alone would eat a real field. Both keys, and only
        // those two keys, is the signature of the wrapper.
        let genuine = json!({"value": 7, "unit": "seconds"});
        assert_eq!(payload(genuine.clone()), genuine);
        let three_keys = json!({"result": "x", "value": 1, "extra": 2});
        assert_eq!(payload(three_keys.clone()), three_keys);
    }

    #[test]
    fn yaml_renders_through_the_maintained_crate() {
        let rendered = render(Format::Yaml, &json!({"ready": true})).unwrap();
        assert_eq!(rendered.trim(), "ready: true");
    }
}
