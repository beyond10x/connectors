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
//! Three rules keep that honest. **No field is dropped**: a field that will not fit a cell is
//! folded onto it rather than omitted, and `compact` spells a nested field out in full, because a
//! format that quietly loses a field is worse than one that is hard to read. **The rank is not
//! carried by colour**: `+`, `!`, `x` and `?` survive a redirect, a pager and `NO_COLOR`, which is
//! where an operator actually reads this. And **a row is one line and fits a terminal**: control
//! characters are folded, and a table too wide for `TABLE_BUDGET` cuts its leading cells — with
//! the cut marked, never silent — so that the last column starts on screen and every column before
//! it is aligned there. That last rule is the one that trades text away, which is why it marks
//! itself and why the cut never reaches `compact`, `json` or `yaml`.
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
    /// One record per line — every line a whole record, so `wc -l` counts records.
    ///
    /// A field the report carries *beside* its records rides on every record line rather than
    /// taking a line of its own. A summary line spelled in the same `key=value` vocabulary is
    /// indistinguishable from a record to `wc -l` and to an `awk -F\t` loop, and a line a script
    /// has to learn to skip is worse than one it can read: `providers` answered 65 to `wc -l`
    /// before this format carried its summary and must go on answering 65.
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
    // Nothing to say is not the same as a blank line. `compact` answers an empty listing with no
    // records at all, and a newline for it is a line `wc -l` counts and an `awk` loop reads.
    if rendered.is_empty() {
        return Ok(());
    }
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
    let Some((name, items)) = unwrap_single_array(value) else {
        return compact_line(value);
    };

    // Only an object reaches here with fields beside the records, and `unwrap_single_array`
    // admitted it because exactly one of its fields is an array — so every non-array field is
    // beside the records rather than one of them.
    let mut beside = Vec::new();
    if let Value::Object(fields) = value {
        for (key, field) in fields.iter().filter(|(_, field)| !field.is_array()) {
            flatten(key, field, &mut beside);
        }
    }

    // Nothing to ride on. The report-level fields on a line of their own would be a line spelled
    // exactly like a record and counted as one by `wc -l`, which is the defect this format's own
    // documentation names — so an empty listing is an empty stream, and the summary of an empty
    // listing is read in `text`, `json` or `yaml`. This is the one place `compact` answers less
    // than the value holds, and it is the format's contract that decides it.
    if items.is_empty() {
        return String::new();
    }

    items
        .iter()
        .map(|item| {
            // A record that is not an object is a single value, and the array's own key is the
            // only name it has. Writing it bare put a token on the line that no `key=value` reader
            // could address: `connect slack` lost the name `events` off its own event list.
            let record = match (item, name) {
                (Value::Object(_), _) | (_, None) => compact_line(item),
                (single, Some(name)) => {
                    let mut pairs = Vec::new();
                    flatten(name, single, &mut pairs);
                    pairs.join("\t")
                }
            };
            let mut pairs: Vec<&str> = Vec::new();
            if !record.is_empty() {
                pairs.push(&record);
            }
            pairs.extend(beside.iter().map(String::as_str));
            pairs.join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The records a list response carries, and the name the report gave them.
///
/// The name matters to a reader of the line: a record that is not an object has no key of its own,
/// so the array's key is the only name its value will ever have.
fn unwrap_single_array(value: &Value) -> Option<(Option<&str>, &Vec<Value>)> {
    match value {
        // A bare array was never named by anything.
        Value::Array(items) => Some((None, items)),
        Value::Object(fields) => {
            let mut arrays = fields
                .iter()
                .filter_map(|(key, field)| Some((key.as_str(), field.as_array()?)));
            let only = arrays.next()?;
            arrays.next().is_none().then_some((Some(only.0), only.1))
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

/// The terminal a table is laid out for.
///
/// Not a guess at the real one: reading it would need a dependency this package does not have and
/// an answer that does not exist when stdout is a pipe. 120 columns is the wide-but-ordinary
/// terminal, and a table laid out for it is readable on a narrower one for every column that fits.
const TABLE_BUDGET: usize = 120;

/// The marker column: one glyph and one space, on every row of every table.
const MARKER_WIDTH: usize = 2;

/// The narrowest a column may be squeezed to before the layout gives up on it.
///
/// Three columns is an elision mark and two characters of content. Below that a column says
/// nothing at all, and a column saying nothing is worse than a table that overruns.
const MIN_COLUMN: usize = 3;

/// What a cut cell carries in place of the text it lost.
const ELISION: char = '~';

/// What a column name keeps while the cells beside it are still being paid.
///
/// Three characters and a mark. Measured against this catalogue: at three columns `reads` and
/// `required_config_fields` both render `re…` and the table has two columns with one name, which
/// is the header's own version of the collision this allocation exists to prevent.
const NAME_FLOOR: usize = 4;

/// One aligned row per record, under a header naming the columns.
///
/// Every row opens with a marker column: where a record carries its own rank the glyph is the
/// rank, and where it does not the column is blank. Blank rather than absent, because a column
/// that appears and disappears is a column a reader cannot trust — a `label` beginning `x ` would
/// otherwise land exactly where `doctor` puts a failure.
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
    // A column name is a JSON key and a JSON key is data: nothing upstream stops one carrying a
    // newline, and it reaches the header without passing through a cell.
    let mut header: Vec<String> = columns.iter().map(|column| one_line(column)).collect();
    // What the cells alone need, kept apart from what the name needs. The budget is spent on
    // content first, and this is the number that says how much content there is.
    let content: Vec<usize> = (0..columns.len())
        .map(|index| {
            cells
                .iter()
                .map(|row| display_width(&row[index]))
                .max()
                .unwrap_or_default()
        })
        .collect();
    let mut widths: Vec<usize> = content
        .iter()
        .zip(&header)
        .map(|(cells, name)| *cells.max(&display_width(name)))
        .collect();
    let mut content = content;

    // A table is only aligned in practice if the column that runs long is the last one: a free-text
    // `detail` in the middle pushes every column after it past the width of a terminal and the row
    // wraps, which is the thing one row per record exists to prevent. Moved only when a single
    // column is strictly the widest, so an ordinary table keeps the order its records carry.
    if let Some(widest) = strictly_widest(&widths) {
        if widest + 1 < widths.len() {
            let name = header.remove(widest);
            header.push(name);
            let width = widths.remove(widest);
            widths.push(width);
            let needed = content.remove(widest);
            content.push(needed);
            for row in &mut cells {
                let moved = row.remove(widest);
                row.push(moved);
            }
        }
    }

    fit_to_budget(&header, &content, &mut widths, pad.len());
    // Only the leading columns are capped, and only after the budget is settled. The last column
    // is deliberately unbounded: it holds the free text — `doctor`'s `detail` is the sentence that
    // says what to do about the row — and a line that wraps inside its last cell still leaves
    // every column before it aligned on screen. Capping it would cut the one field worth reading.
    let leading = widths.len().saturating_sub(1);
    for index in 0..leading {
        header[index] = elide(&header[index], widths[index]);
        for row in &mut cells {
            row[index] = elide(&row[index], widths[index]);
        }
    }

    write_row(&pad, "  ", &header, &widths, buffer);
    for (item, row) in items.iter().zip(&cells) {
        let marker = format!("{} ", row_severity(item).map_or(' ', Severity::marker));
        write_row(&pad, &marker, row, &widths, buffer);
    }
}

/// Squeeze the leading columns until the last one starts inside the budget.
///
/// Moving the widest column last cannot fix a wide table on its own: `connectors providers` has
/// eleven columns before the moved one and they came to 196 terminal columns between them, so
/// every row wrapped and nothing after the first column was aligned.
///
/// **The budget is spent on content, and a name longer than its content is what gives way first.**
/// Paying the name first is what the previous shape did, and it cost the story its point: a
/// 22-character `required_config_fields` held 22 columns for one-character cells while `provider`
/// was cut to eight, so 18 of 65 catalogued ids were truncated and four pairs of distinct
/// authorities rendered as the same string. An identifier that collides on screen is worse than a
/// name that is short, so a name is cut down to its content, and to [`NAME_FLOOR`] at the least —
/// three characters and a mark, which is what keeps `reads` and `required_config_fields` apart.
/// Only when even that does not fit does the second phase take content down too, to
/// [`MIN_COLUMN`], and no column is ever squeezed below the one column its elision mark occupies.
fn fit_to_budget(header: &[String], content: &[usize], widths: &mut [usize], pad: usize) {
    let Some(leading) = widths.len().checked_sub(1).filter(|count| *count > 0) else {
        return;
    };
    let paid: Vec<usize> = header
        .iter()
        .zip(content)
        .map(|(name, needed)| (*needed).max(display_width(name).min(NAME_FLOOR)).max(1))
        .collect();
    for floors in [
        paid.clone(),
        // Never below one column: that is what the elision mark itself occupies, and a column
        // narrower than its own mark spends a terminal column the layout did not allocate and
        // pushes every column after it out of line.
        paid.iter()
            .map(|floor| (*floor).clamp(1, MIN_COLUMN))
            .collect(),
    ] {
        loop {
            let start = pad + MARKER_WIDTH + widths[..leading].iter().map(|w| w + 2).sum::<usize>();
            // Strictly inside: a terminal `TABLE_BUDGET` columns wide holds cells `0..TABLE_BUDGET`,
            // so a column *beginning* at the budget has not one character on the line.
            if start < TABLE_BUDGET {
                return;
            }
            // The widest column that can still give, earliest one first on a tie, one column at a
            // time — so the columns end up as even as the budget allows rather than one of them
            // being cut to nothing.
            let Some(target) = (0..leading)
                .filter(|index| widths[*index] > floors[*index])
                .max_by_key(|index| (widths[*index], std::cmp::Reverse(*index)))
            else {
                break;
            };
            widths[target] -= 1;
        }
    }
}

fn write_row(pad: &str, marker: &str, cells: &[String], widths: &[usize], buffer: &mut String) {
    let mut line = String::from(pad);
    line.push_str(marker);
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            line.push_str("  ");
        }
        line.push_str(cell);
        // The last column is never padded: no cell is ever empty, so nothing here can leave a line
        // ending in whitespace.
        if index + 1 < cells.len() {
            let used = display_width(cell);
            line.push_str(&" ".repeat(widths[index].saturating_sub(used)));
        }
    }
    let _ = writeln!(buffer, "{line}");
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

/// How many terminal columns a string occupies, which is not how many `char`s it holds.
///
/// A CJK or fullwidth character is two columns wide, so counting `char`s displaces every later
/// column on that row by one per wide character — reachable through any value chosen elsewhere,
/// such as a Grafana target's label or the free-form `instance` of `auth status`.
///
/// The East Asian Wide and Fullwidth ranges, and no more: this package has no width table and is
/// not going to grow one. A combining mark still counts as a column it does not occupy, and an
/// emoji sequence counts as one column per code point; both are wrong, both are rarer than the
/// case this fixes, and neither is chased here. A control character would be counted as a column
/// it does not occupy either, which is why everything measured has been through [`one_line`] —
/// cells by way of `scalar`, and column names, which are JSON keys and never touched a cell,
/// where the header is built.
fn display_width(text: &str) -> usize {
    text.chars()
        .map(|character| {
            let code = character as u32;
            let wide = (0x1100..=0x115F).contains(&code)
                || (0x2E80..=0xA4CF).contains(&code)
                || (0xAC00..=0xD7A3).contains(&code)
                || (0xF900..=0xFAFF).contains(&code)
                || (0xFE30..=0xFE6F).contains(&code)
                || (0xFF00..=0xFF60).contains(&code)
                || (0xFFE0..=0xFFE6).contains(&code);
            usize::from(wide) + 1
        })
        .sum()
}

/// Cut a cell to `width` terminal columns, saying so.
///
/// The mark is what makes this honest: a silently truncated value reads as the whole value, and a
/// reader who cannot see that text was cut has no reason to go and look at `-o json`, which still
/// carries all of it.
///
/// `~` rather than a typographic ellipsis, because the mark is **one byte as well as one column**.
/// Everything that reads a line off a terminal counts bytes — `cut -c`, `awk`, `wc -c`, and a test
/// asking where a column begins — and a three-byte mark makes the layout's own arithmetic disagree
/// with theirs by two bytes per cut cell. Whatever skew the data brings is the data's; the layout
/// adds none of its own.
fn elide(text: &str, width: usize) -> String {
    if display_width(text) <= width {
        return text.to_owned();
    }
    let mut cut = String::new();
    let mut used = 0;
    for character in text.chars() {
        let next = used + display_width(character.encode_utf8(&mut [0; 4]));
        // One column is kept back for the mark itself.
        if next + 1 > width {
            break;
        }
        cut.push(character);
        used = next;
    }
    cut.push(ELISION);
    cut
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
        // An empty string is a value the record carries, and a cell showing nothing shows the same
        // as a row that is not there — a record whose every field is empty rendered as a blank
        // line. Named, like the empty list and the empty object beside it.
        Value::String(text) if text.is_empty() => "\"\"".to_owned(),
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
        // A readiness field is the whole answer, and `false` is a warning rather than a failure:
        // 13 of 65 catalogued providers declare no verify probe, and every one of them can still
        // be connected. Marking them "this cannot work" says something untrue about most of the
        // catalogue; `warn` and `ok` are still two different glyphs, so readiness stays visible.
        Value::Bool(ready) => {
            if *ready {
                Severity::Ok
            } else {
                Severity::Warn
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
/// The second half of the vocabulary is the protocol's, and it is ranked against this module's own
/// definition of `fail` — *this cannot work* — rather than against how the word sounds. A
/// `revoked` Connection and a `stopped` Channel cannot carry a call as they stand, and an
/// `expired` Connect Session can no longer be completed; `created`, `authorized`, `starting` and
/// `reconnecting` are all a step on the way to working, which is a warning. Held to the enums
/// themselves by `every_protocol_state_this_renderer_can_be_handed_has_a_rank`, whose match is
/// exhaustive, so a variant added upstream cannot compile until it is answered here.
///
/// `absent`, `unavailable` and `not-callable` are warnings rather than failures on `auth.rs`'s own
/// reasoning: a credential that was never stored is the ordinary state before `connect`, and a
/// store that cannot be read is "we cannot say" rather than "it is not there". `not-callable` is
/// set by exactly that state (`auth.rs`, `if satisfied.is_empty()`), so ranking it `fail` marked
/// every configured provider on a fresh install as broken. `no-authority` and `unknown-provider`
/// stay failures: those two are a configuration that cannot be made to work by connecting it.
fn severity_of_word(word: &str) -> Severity {
    match word {
        "ok" | "healthy" | "ready" | "pass" | "passed" | "callable" | "stored" | "listening"
        | "connected" | "completed" => Severity::Ok,
        // `stored-without-user-half` arrived with `[catalog.usernames]` and was caught by the
        // scan below rather than by review: the secret is in the keyring and the non-secret user
        // half is not, so every call refuses until an operator writes one config line. That is the
        // same shape as `not-callable` and `absent` — fixable, and not the glyph for cannot work.
        "warn"
        | "warning"
        | "degraded"
        | "absent"
        | "missing"
        | "unavailable"
        | "pending"
        | "not-callable"
        | "stored-without-user-half"
        | "created"
        | "authorized"
        | "starting"
        | "reconnecting" => Severity::Warn,
        "fail" | "failed" | "error" | "refused" | "unhealthy" | "not-ready" | "no-authority"
        | "unknown-provider" | "revoked" | "stopped" | "expired" => Severity::Fail,
        _ => Severity::Unknown,
    }
}

/// A scalar without JSON's quotes, which a person reading a terminal did not ask for.
fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => one_line(text),
        Value::Null => "null".to_owned(),
        other => other.to_string(),
    }
}

/// One line, whatever the value had in it.
///
/// **The layout of both human formats rests on a value being one line, and nothing upstream
/// guarantees that.** `toml::de::Error` renders a six-line caret diagram, `doctor` folds it into a
/// check's `detail`, and a TOML typo is the single most likely reason anyone runs `doctor` at all:
/// one record became six lines, five of them with nothing in the marker column. The same newline
/// breaks `compact`'s one-record-per-line contract, and this is the one place both formats pass
/// through.
///
/// Every control character is folded, not only the newline: `ESC` reaches here from any value
/// chosen on the far side of a wire, and a terminal reads an escape sequence rather than printing
/// it. A backslash is deliberately *not* escaped — a Windows path would pay for it on every line —
/// so a folded `\n` is not distinguishable from a value that really held those two characters.
/// `-o json` is the format that answers that question exactly.
fn one_line(text: &str) -> String {
    if !text.chars().any(char::is_control) {
        return text.to_owned();
    }
    let mut folded = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\n' => folded.push_str("\\n"),
            '\r' => folded.push_str("\\r"),
            '\t' => folded.push_str("\\t"),
            control if control.is_control() => {
                let _ = write!(folded, "\\u{{{:02x}}}", control as u32);
            }
            ordinary => folded.push(ordinary),
        }
    }
    folded
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

include!("output_tests.rs");
