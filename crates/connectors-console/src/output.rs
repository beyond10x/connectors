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
//!
//! # Why a list of records is a table
//!
//! `text` is what a person gets by default, and a person reading a list is scanning it. A generic
//! walker cannot be scanned: it spent four lines on one `doctor` check — a bare `-`, then a line
//! per field — so six checks filled a screen and the one `warn` among them read exactly like the
//! five `ok` rows above it. So a list of records is rendered as one aligned row each, under a
//! header, with the record's own rank in a marker column at the left.
//!
//! Two rules keep that honest. **Nothing is dropped**: a field that will not fit a cell is folded
//! onto it rather than omitted, and `compact` spells a nested field out in full, because a format
//! that quietly loses a field is worse than one that is hard to read. And **the rank is not
//! carried by colour**: `+`, `!`, `x` and `?` survive a redirect, a pager and `NO_COLOR`, which is
//! where an operator actually reads this.
//!
//! `json` and `yaml` are untouched by all of it. Their reader is a parser that has already
//! committed to these bytes, and readability is not its problem.

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

/// Render without writing, so a module can assert what its own result looks like to a reader.
///
/// Crate-visible rather than private: `doctor` ranks its checks and the rank is only worth
/// something if it survives this function, which is a claim `doctor`'s own tests have to be able
/// to make. Still not public — `emit` is what a caller outside this package uses.
pub(crate) fn render(format: Format, value: &Value) -> Result<String, OutputError> {
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

/// One record per line, carrying everything the value carries.
///
/// The interesting case is the shape every list response in this component has: one object whose
/// single array field holds the records, and scalars beside it that summarise them. Rendering that
/// as one line — the object — would defeat the format, so the array is unwrapped first and each
/// element becomes its own line.
///
/// The fields *beside* the records become the first line rather than being discarded. Dropping
/// them is what put `healthy` out of reach of `connectors -o compact doctor | grep healthy`, which
/// printed nothing at all for a report whose whole purpose is to answer that one question.
fn compact(value: &Value) -> String {
    let mut lines = Vec::new();
    match unwrap_single_array(value) {
        Some(items) => {
            // Only an object reaches here with siblings, and `unwrap_single_array` admitted it
            // because exactly one of its fields is an array — so every non-array field is beside
            // the records rather than one of them.
            if let Value::Object(fields) = value {
                let mut pairs = Vec::new();
                for (key, field) in fields.iter().filter(|(_, field)| !field.is_array()) {
                    flatten(key, field, &mut pairs);
                }
                if !pairs.is_empty() {
                    lines.push(pairs.join("\t"));
                }
            }
            lines.extend(items.iter().map(compact_line));
        }
        None => lines.push(compact_line(value)),
    }
    lines.join("\n")
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

fn compact_line(value: &Value) -> String {
    let Value::Object(fields) = value else {
        return scalar(value);
    };
    let mut pairs = Vec::new();
    for (key, field) in fields {
        flatten(key, field, &mut pairs);
    }
    pairs.join("\t")
}

/// One value as `key=value` pairs, with a nested key spelled out in full.
///
/// `compact` is what a script greps, and a field the line silently omitted is a field the script
/// cannot see is missing — so a nested one keeps its whole address, `credentials.0.state=stored`,
/// rather than being skipped for not being a scalar. An empty list is named for the same reason:
/// *no credentials* and *no such field* are different answers.
fn flatten(prefix: &str, value: &Value, pairs: &mut Vec<String>) {
    match value {
        Value::Object(fields) if fields.is_empty() => pairs.push(format!("{prefix}=(empty)")),
        Value::Object(fields) => {
            for (key, field) in fields {
                flatten(&format!("{prefix}.{key}"), field, pairs);
            }
        }
        Value::Array(items) if items.is_empty() => pairs.push(format!("{prefix}=(none)")),
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                flatten(&format!("{prefix}.{index}"), item, pairs);
            }
        }
        scalar_value => pairs.push(format!("{prefix}={}", scalar(scalar_value))),
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
                    Value::Array(items) if is_record_list(items) => {
                        let _ = writeln!(buffer, "{pad}{key}:");
                        table(items, depth + 1, buffer);
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
        Value::Array(items) if is_record_list(items) => table(items, depth, buffer),
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

/// A list worth aligning: every element is an object with something in it.
///
/// A list of scalars stays a list of scalars — a table of one unnamed column is worse than the
/// `- value` lines it would replace.
fn is_record_list(items: &[Value]) -> bool {
    !items.is_empty()
        && items
            .iter()
            .all(|item| matches!(item, Value::Object(fields) if !fields.is_empty()))
}

/// One aligned row per record, under a header naming the columns.
///
/// Where a record carries its own rank, the row opens with a marker column: `doctor` ranks its
/// three states and the generic walker discarded that rank at the last inch, so one `warn` looked
/// exactly like the five `ok` rows above it.
fn table(items: &[Value], depth: usize, buffer: &mut String) {
    let pad = "  ".repeat(depth);

    // Every key any record carries, in the order the records carry them. A key one record omits is
    // still a column: a record list is not always uniform, and a missing cell is an answer.
    let mut columns: Vec<&str> = Vec::new();
    for item in items {
        if let Value::Object(fields) = item {
            for key in fields.keys() {
                if !columns.contains(&key.as_str()) {
                    columns.push(key);
                }
            }
        }
    }

    let mut cells: Vec<Vec<String>> = items
        .iter()
        .map(|item| columns.iter().map(|column| cell(item, column)).collect())
        .collect();
    let mut widths: Vec<usize> = columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            cells
                .iter()
                .map(|row| row[index].chars().count())
                .chain(std::iter::once(column.chars().count()))
                .max()
                .unwrap_or_default()
        })
        .collect();

    // A table is only aligned in practice if the column that runs long is the last one: a free-text
    // `detail` in the middle pushes every column after it past the width of a terminal and the row
    // wraps, which is the thing one row per record exists to prevent. Moved only when a single
    // column is strictly the widest, so an ordinary table keeps the order its records carry.
    if let Some(widest) = strictly_widest(&widths) {
        if widest + 1 < columns.len() {
            let column = columns.remove(widest);
            columns.push(column);
            let width = widths.remove(widest);
            widths.push(width);
            for row in &mut cells {
                let moved = row.remove(widest);
                row.push(moved);
            }
        }
    }

    let ranked = items.iter().any(|item| row_severity(item).is_some());
    let header: Vec<String> = columns.iter().map(|column| (*column).to_owned()).collect();
    write_row(
        &pad,
        if ranked { "  " } else { "" },
        &header,
        &widths,
        buffer,
    );
    for (item, row) in items.iter().zip(&cells) {
        let marker = if ranked {
            // A ranked table whose row carries no rank keeps the column and leaves it blank, so
            // every row still starts in the same place.
            format!("{} ", row_severity(item).map_or(' ', Severity::marker))
        } else {
            String::new()
        };
        write_row(&pad, &marker, row, &widths, buffer);
    }
}

fn write_row(pad: &str, marker: &str, cells: &[String], widths: &[usize], buffer: &mut String) {
    let mut line = String::from(pad);
    line.push_str(marker);
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            line.push_str("  ");
        }
        // The last column is never padded: trailing spaces are invisible to a reader and a
        // nuisance to everything else.
        if index + 1 == cells.len() {
            line.push_str(cell);
        } else {
            let _ = write!(line, "{cell:<width$}", width = widths[index]);
        }
    }
    let _ = writeln!(buffer, "{}", line.trim_end());
}

/// The one column wider than every other, if there is one.
fn strictly_widest(widths: &[usize]) -> Option<usize> {
    let widest = *widths.iter().max()?;
    let mut widest_columns = widths
        .iter()
        .enumerate()
        .filter(|(_, width)| **width == widest);
    let (index, _) = widest_columns.next()?;
    widest_columns.next().is_none().then_some(index)
}

/// One cell.
///
/// A key this record does not carry is `-`, which reads differently from an empty value: a reader
/// can tell *this record has no such field* from *the table lost it*.
fn cell(item: &Value, column: &str) -> String {
    item.get(column).map_or_else(|| "-".to_owned(), inline)
}

/// A nested value folded onto one line, whole.
///
/// A row is one line and a field that would not fit on it is still a field the value carries, so
/// it is folded rather than dropped — `-o text` loses nothing that `-o json` would have shown.
fn inline(value: &Value) -> String {
    match value {
        Value::Array(items) if items.is_empty() => "(none)".to_owned(),
        Value::Array(items) => items.iter().map(inline).collect::<Vec<_>>().join(", "),
        Value::Object(fields) if fields.is_empty() => "(empty)".to_owned(),
        Value::Object(fields) => {
            let pairs = fields
                .iter()
                .map(|(key, field)| format!("{key}={}", inline(field)))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{{{pairs}}}")
        }
        scalar_value => scalar(scalar_value),
    }
}

/// The rank of one row, as a reader sees it before reading the row.
///
/// The three states are `doctor`'s own — `fail` is "this cannot work", `warn` is "this works and
/// you should know", `ok` is silence worth confirming. `Unknown` is the honest answer for a word
/// this package does not own: the renderer is shared with results that come off the wire, and
/// ranking an unrecognised word as `ok` would be a lie where `?` is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Ok,
    Warn,
    Fail,
    Unknown,
}

impl Severity {
    /// The character the marker column carries.
    ///
    /// ASCII, and never colour: an escape sequence is nothing in a file, nothing under `NO_COLOR`
    /// and nothing to `grep`, and a severity a pipe erases is a severity nobody can act on. Colour
    /// could be added on top of this one day; it could never replace it.
    const fn marker(self) -> char {
        match self {
            Self::Ok => '+',
            Self::Warn => '!',
            Self::Fail => 'x',
            Self::Unknown => '?',
        }
    }
}

/// The fields a record carries its own rank in, most specific first.
const SEVERITY_KEYS: [&str; 6] = ["status", "severity", "state", "health", "healthy", "ready"];

fn row_severity(item: &Value) -> Option<Severity> {
    let Value::Object(fields) = item else {
        return None;
    };
    SEVERITY_KEYS
        .iter()
        .find_map(|key| fields.get(*key))
        .map(severity_of)
}

fn severity_of(value: &Value) -> Severity {
    match value {
        // A readiness field is the whole answer: `ready: false` is a row an operator has to do
        // something about, which is what the marker is for.
        Value::Bool(ready) => {
            if *ready {
                Severity::Ok
            } else {
                Severity::Fail
            }
        }
        Value::String(word) => severity_of_word(word),
        _ => Severity::Unknown,
    }
}

/// Rank one word.
///
/// The vocabulary is this package's own, and it is not kept complete by hand: the renderer's tests
/// read every literal written at a severity key in `src/` and fail on one that lands here as
/// `Unknown`. The words are `doctor`'s three (`doctor.rs`), `auth status`'s four states and its
/// three credential states (`auth.rs`), and the ordinary spellings of the same three ranks.
///
/// `absent` and `unavailable` are warnings rather than failures on `auth.rs`'s own reasoning: a
/// credential that was never stored is the ordinary state before `connect`, and a store that
/// cannot be read is "we cannot say" rather than "it is not there".
fn severity_of_word(word: &str) -> Severity {
    match word {
        "ok" | "healthy" | "ready" | "pass" | "passed" | "callable" | "stored" | "listening"
        | "connected" => Severity::Ok,
        "warn" | "warning" | "degraded" | "absent" | "missing" | "unavailable" | "pending" => {
            Severity::Warn
        }
        "fail" | "failed" | "error" | "refused" | "unhealthy" | "not-ready" | "not-callable"
        | "no-authority" | "unknown-provider" => Severity::Fail,
        _ => Severity::Unknown,
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

    /// The shape `doctor` emits: a summary scalar beside one record per check.
    fn doctor_report() -> Value {
        json!({
            "healthy": false,
            "checks": [
                {"check": "configuration", "status": "ok", "detail": "declares slack"},
                {"check": "daemon", "status": "warn", "detail": "not running"},
                {"check": "state-root", "status": "fail", "detail": "belongs to uid 0"},
            ],
        })
    }

    /// The shape `auth status` emits: records whose own fields are lists, one of them a list of
    /// objects. Nothing here may be dropped by any format.
    fn auth_status() -> Value {
        json!({
            "store": "keyring",
            "providers": [
                {
                    "provider": "slack",
                    "instance": "timo-ai",
                    "status": "callable",
                    "credentials": [
                        {"credential": "slack.bot_token", "subject": "app", "state": "stored"},
                        {"credential": "slack.user_token", "subject": "user", "state": "absent"},
                    ],
                    "satisfied_mechanisms": ["slack.bot_token"],
                    "verify": "slack-users-info",
                },
                {
                    "provider": "gitlab",
                    "instance": "default",
                    "status": "not-callable",
                    "credentials": [],
                    "satisfied_mechanisms": [],
                    "verify": null,
                },
            ],
        })
    }

    fn row_holding<'a>(rendered: &'a str, needle: &str) -> &'a str {
        let mut rows = rendered.lines().filter(|line| line.contains(needle));
        let row = rows
            .next()
            .unwrap_or_else(|| panic!("no row carries `{needle}`:\n{rendered}"));
        assert!(
            rows.next().is_none(),
            "`{needle}` is spread over more than one line:\n{rendered}"
        );
        row
    }

    #[test]
    fn text_spends_one_aligned_row_on_each_record() {
        // Six checks used to cost 26 lines: a bare `-` and one line per field. One row per record
        // is the whole story — a label, a header, three rows, and the summary field beside them.
        let rendered = render(Format::Text, &doctor_report()).unwrap();
        assert_eq!(rendered.lines().count(), 6, "\n{rendered}");

        let header = rendered
            .lines()
            .nth(1)
            .expect("a header naming the columns");
        for column in ["check", "status", "detail"] {
            assert!(
                header.contains(column),
                "the header drops `{column}`:\n{rendered}"
            );
        }

        // Aligned means every cell of one column starts at the same offset, header included.
        let ok = row_holding(&rendered, "configuration");
        let warn = row_holding(&rendered, "daemon");
        let fail = row_holding(&rendered, "state-root");
        assert_eq!(header.find("status"), ok.find("ok"), "\n{rendered}");
        assert_eq!(header.find("status"), warn.find("warn"), "\n{rendered}");
        assert_eq!(header.find("status"), fail.find("fail"), "\n{rendered}");
        assert_eq!(
            header.find("detail"),
            ok.find("declares slack"),
            "\n{rendered}"
        );
        assert_eq!(
            header.find("detail"),
            warn.find("not running"),
            "\n{rendered}"
        );
        assert_eq!(
            header.find("detail"),
            fail.find("belongs to uid 0"),
            "\n{rendered}"
        );
    }

    #[test]
    fn a_row_shows_its_severity_before_anybody_reads_it() {
        // `doctor.rs` ranks its three states and the renderer used to discard the rank at the last
        // inch, so one `warn` looked exactly like the five `ok` rows above it.
        let rendered = render(Format::Text, &doctor_report()).unwrap();
        let marker = |needle: &str| {
            row_holding(&rendered, needle)
                .trim_start()
                .chars()
                .next()
                .expect("a marker")
        };
        assert_eq!(marker("configuration"), '+', "\n{rendered}");
        assert_eq!(marker("daemon"), '!', "\n{rendered}");
        assert_eq!(marker("state-root"), 'x', "\n{rendered}");
    }

    #[test]
    fn severity_survives_a_pipe_because_it_is_not_carried_by_colour() {
        // A colour escape is invisible in a file, in `less`, and under NO_COLOR. The marker is
        // plain ASCII, so redirection cannot lose it.
        let rendered = render(Format::Text, &doctor_report()).unwrap();
        assert!(rendered.is_ascii(), "\n{rendered}");
        assert!(!rendered.contains('\u{1b}'), "\n{rendered}");
    }

    #[test]
    fn a_table_reads_left_to_right_with_the_column_that_runs_long_last() {
        let rendered = render(
            Format::Text,
            &json!({"rows": [{"name": "a", "status": "ok"}, {"name": "bb", "status": "fail"}]}),
        )
        .unwrap();
        assert_eq!(
            rendered,
            concat!(
                "rows:\n",
                "    name  status\n",
                "  + a     ok\n",
                "  x bb    fail\n",
            )
        );
    }

    #[test]
    fn text_keeps_every_field_a_record_carries_including_a_nested_list() {
        let rendered = render(Format::Text, &auth_status()).unwrap();
        // One row per provider, and the store the report was read from beside them.
        assert_eq!(rendered.lines().count(), 5, "\n{rendered}");
        assert!(rendered.contains("store: keyring"), "\n{rendered}");
        for field in [
            "slack.bot_token",
            "stored",
            "slack.user_token",
            "absent",
            "slack-users-info",
            "timo-ai",
        ] {
            assert!(
                rendered.contains(field),
                "`{field}` is missing from the text rendering:\n{rendered}"
            );
        }
    }

    #[test]
    fn compact_keeps_the_scalar_a_list_response_carries_beside_its_records() {
        // `connectors -o compact doctor | grep -c healthy` printed 0: the single array was
        // unwrapped and every scalar beside it was discarded.
        let rendered = render(Format::Compact, &doctor_report()).unwrap();
        assert!(
            rendered.lines().any(|line| line.contains("healthy=false")),
            "\n{rendered}"
        );
        assert_eq!(rendered.lines().count(), 4, "\n{rendered}");
    }

    #[test]
    fn compact_keeps_a_field_a_record_carries_below_its_top_level() {
        let rendered = render(Format::Compact, &auth_status()).unwrap();
        assert!(rendered.contains("store=keyring"), "\n{rendered}");
        for pair in [
            "credentials.0.credential=slack.bot_token",
            "credentials.0.state=stored",
            "credentials.1.state=absent",
            "satisfied_mechanisms.0=slack.bot_token",
        ] {
            assert!(
                rendered.contains(pair),
                "`{pair}` is missing from the compact rendering:\n{rendered}"
            );
        }
        // An empty list is still a field the value carries, so it is named rather than omitted.
        assert!(rendered.contains("credentials=(none)"), "\n{rendered}");
    }

    #[test]
    fn the_structured_formats_render_the_bytes_they_rendered_before() {
        // Pinned against what this module emitted before the text work: `-o json` and `-o yaml` are
        // contracts for a caller that parses stdout, and readability is not their reader's problem.
        let report = json!({
            "healthy": true,
            "checks": [{"check": "daemon", "status": "warn", "detail": "not running"}],
        });
        assert_eq!(
            render(Format::Json, &report).unwrap(),
            concat!(
                "{\n",
                "  \"checks\": [\n",
                "    {\n",
                "      \"check\": \"daemon\",\n",
                "      \"detail\": \"not running\",\n",
                "      \"status\": \"warn\"\n",
                "    }\n",
                "  ],\n",
                "  \"healthy\": true\n",
                "}",
            )
        );
        assert_eq!(
            render(Format::Yaml, &report).unwrap(),
            concat!(
                "checks:\n",
                "- check: daemon\n",
                "  detail: not running\n",
                "  status: warn\n",
                "healthy: true\n",
            )
        );
    }

    /// The marker a one-record table shows for a word written under a severity key.
    fn marker_for(word: &str) -> Option<char> {
        let rendered = render(
            Format::Text,
            &json!({"rows": [{"subject": "the-record", "status": word}]}),
        )
        .expect("a text rendering");
        row_holding(&rendered, "the-record")
            .trim_start()
            .chars()
            .next()
    }

    #[test]
    fn a_word_the_renderer_cannot_rank_is_marked_unknown_rather_than_good() {
        // The renderer is shared with results that come off the wire, whose vocabulary this
        // package does not own. Ranking an unrecognised word as `ok` would be a lie; `?` is not.
        let word = "sasquatch";
        assert_eq!(marker_for(word), Some('?'));
    }

    #[test]
    fn every_status_word_this_package_emits_is_one_the_renderer_can_rank() {
        // The class, checked rather than listed: a marker column is only worth reading if every
        // word that can appear under a severity key has a rank. A hand-kept list would need an
        // adversary to extend it, so the package's own sources are the list — every string literal
        // written at a `"status":`, `"state":` or `"severity":` key must rank as something other
        // than unknown. Bounded on purpose: a word assigned through a variable
        // (`auth.rs`'s credential `state`) is invisible here, and `doctor.rs` covers its own three.
        let sources = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut found = 0_usize;
        for entry in std::fs::read_dir(&sources).expect("the package's own sources") {
            let path = entry.expect("a source entry").path();
            if path.extension().and_then(std::ffi::OsStr::to_str) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("a readable source file");
            for (offset, line) in source.lines().enumerate() {
                for word in severity_words_written_on(line) {
                    let marker = marker_for(&word);
                    assert!(
                        matches!(marker, Some('+' | '!' | 'x')),
                        "{}:{} writes `{word}` under a severity key and the renderer ranks it \
                         `{marker:?}` rather than ok, warn or fail",
                        path.display(),
                        offset + 1
                    );
                    found += 1;
                }
            }
        }
        assert!(
            found >= 6,
            "the scan ranked {found} words; it has stopped reading the sources"
        );
    }

    /// Every string literal written as the value of a severity key on one line of Rust source.
    ///
    /// Stops at the next key so `"status": "x", "detail": "y"` yields `x` alone, and keeps reading
    /// past a conditional so `if empty { "a" } else { "b" }` yields both.
    fn severity_words_written_on(line: &str) -> Vec<String> {
        let mut words = Vec::new();
        // A comment emits nothing, and this module's own documentation quotes the shape it reads.
        if line.trim_start().starts_with("//") {
            return words;
        }
        for key in ["\"status\":", "\"state\":", "\"severity\":"] {
            let Some(start) = line.find(key) else {
                continue;
            };
            let mut rest = &line[start + key.len()..];
            while let Some(open) = rest.find('"') {
                let Some(close) = rest[open + 1..].find('"') else {
                    break;
                };
                let word = &rest[open + 1..open + 1 + close];
                rest = &rest[open + close + 2..];
                // A literal immediately followed by `:` is the next key, not a value.
                if rest.trim_start().starts_with(':') {
                    break;
                }
                words.push(word.to_owned());
            }
        }
        words
    }
}
