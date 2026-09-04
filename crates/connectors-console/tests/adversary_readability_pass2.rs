//! Second adversarial pass against `story:informative-command-readability`, aimed at the ground the
//! correction (`06e1e22`) newly opened: `fit_to_budget`, `elide`, and `compact` carrying the
//! report-level fields on every record line.
//!
//! Every case drives the **public** surface — `connectors_console::emit`, the function the binary
//! calls — so what is measured is what an operator's terminal receives, not what a crate-private
//! helper returns. `emit` writes straight to file descriptor 1, which libtest does not intercept,
//! so a case that wants the bytes re-executes this test binary with `--exact
//! pass2_render_helper_child` and reads the child's stdout between two sentinels. That is the same
//! device `adversary_readability.rs` uses, and for the same reason.

use connectors_console::{emit, Format};
use serde_json::{json, Map, Value};

const VALUE_ENV: &str = "ADVERSARY2_RENDER_VALUE";
const FORMAT_ENV: &str = "ADVERSARY2_RENDER_FORMAT";
const OPEN: &str = "<<<adversary2-render-begin>>>";
const CLOSE: &str = "<<<adversary2-render-end>>>";

/// The terminal width `output.rs` lays a table out for (`TABLE_BUDGET`, `output.rs:308`).
const TABLE_BUDGET: usize = 120;

/// The other half of [`rendered`]. Inert unless the parent set `ADVERSARY2_RENDER_VALUE`.
#[test]
fn pass2_render_helper_child() {
    let Ok(raw) = std::env::var(VALUE_ENV) else {
        return;
    };
    let value: Value = serde_json::from_str(&raw).expect("the value the parent handed over");
    let format = if std::env::var(FORMAT_ENV).as_deref() == Ok("compact") {
        Format::Compact
    } else {
        Format::Text
    };
    emit(Format::Text, &json!(OPEN)).expect("the opening sentinel");
    emit(format, &value).expect("the value under test");
    emit(Format::Text, &json!(CLOSE)).expect("the closing sentinel");
}

/// Exactly the bytes `emit` puts on stdout for `value` in `format`.
fn rendered(format: &str, value: &Value) -> String {
    let exe = std::env::current_exe().expect("this test binary");
    let output = std::process::Command::new(&exe)
        .args([
            "--exact",
            "pass2_render_helper_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(VALUE_ENV, serde_json::to_string(value).expect("json"))
        .env(FORMAT_ENV, format)
        .output()
        .expect("the helper child runs");
    assert!(
        output.status.success(),
        "the helper child failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8 on stdout");
    let start = stdout
        .find(OPEN)
        .unwrap_or_else(|| panic!("no opening sentinel in:\n{stdout}"))
        + OPEN.len();
    let end = stdout
        .find(CLOSE)
        .unwrap_or_else(|| panic!("no closing sentinel in:\n{stdout}"));
    stdout[start..end]
        .strip_prefix('\n')
        .expect("a newline after the sentinel")
        .to_owned()
}

/// How many terminal columns a string occupies. The same East Asian Wide and Fullwidth ranges
/// `output.rs:472` uses, so a measurement here is the measurement the layout made.
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

#[test]
fn compact_answers_an_empty_listing_with_a_line_that_is_not_a_record() {
    // `Format::Compact`'s own documentation, written by this unit's correction (`output.rs:49`):
    //
    //   "One record per line — every line a whole record, so `wc -l` counts records. A field the
    //    report carries *beside* its records rides on every record line rather than taking a line
    //    of its own. A summary line spelled in the same `key=value` vocabulary is indistinguishable
    //    from a record to `wc -l` and to an `awk -F\t` loop, and a line a script has to learn to
    //    skip is worse than one it can read."
    //
    // That is exactly what an empty listing emits. `output.rs:180` returns `beside.join("\t")` when
    // there is no record to ride on, so the stream is one line — `summary.listed=0` — for zero
    // records. Reached by `connectors -o compact providers --query <no match>`
    // (`connectors-cli/src/lib.rs:524`, `--query` is a documented flag at `lib.rs:91`), and by
    // `connectors -o compact auth status` on a configuration that lists no provider
    // (`auth.rs:125`).
    let none = connectors_console::providers::run("no-such-provider");
    let records = none["providers"].as_array().expect("the records").len();
    assert_eq!(records, 0, "the fixture is meant to be an empty listing");

    let rendered = rendered("compact", &none);
    let lines: Vec<&str> = rendered.lines().collect();
    let strays: Vec<&&str> = lines
        .iter()
        .filter(|line| !line.contains("provider="))
        .collect();
    assert!(
        strays.is_empty(),
        "{} of {} lines are not records, so an `awk -F'\\t'` loop reads a summary as a record: \
         {strays:?}",
        strays.len(),
        lines.len()
    );
    assert_eq!(
        lines.len(),
        records,
        "`wc -l` counts {} lines for {records} records",
        lines.len()
    );
}

#[test]
fn the_last_column_of_providers_begins_one_column_past_the_terminal_it_is_laid_out_for() {
    // `fit_to_budget` (`output.rs:405`) stops squeezing at `if start <= TABLE_BUDGET`
    // (`output.rs:416`), so the last column is allowed to *begin* at terminal column
    // `TABLE_BUDGET`. A terminal `TABLE_BUDGET` columns wide holds cells 0..=119, so a column
    // beginning at 120 has not one character on the line: the whole of it is on the wrapped
    // continuation, which is the thing the module documentation says the budget prevents —
    // "so that the last column starts on screen" (`output.rs:30`).
    //
    // `connectors providers` reaches the bound exactly, and not by chance: its eleven leading
    // column names come to 94 columns, and `4 + 94 + 2*11` is 120 on the nose. Reached by
    // `connectors providers`, default format, on the 120-column terminal `TABLE_BUDGET`
    // documents itself as ("the wide-but-ordinary terminal", `output.rs:307`).
    let all = connectors_console::providers::run("");
    let rendered = rendered("text", &all);
    let header = rendered
        .lines()
        .find(|line| line.contains("provider") && line.contains("credentials"))
        .expect("a header naming the columns");
    let byte = header
        .rfind("credentials")
        .expect("where the last column begins");
    let start = display_width(&header[..byte]);
    assert!(
        start < TABLE_BUDGET,
        "the last column begins at terminal column {start} of a {TABLE_BUDGET}-column terminal, \
         so none of it is on the line and every row's last cell is read on the wrap:\n{header}"
    );
}

#[test]
fn compact_drops_the_name_of_the_array_a_report_carries() {
    // The acceptance: "no output format drops a field the value carries." When the single array a
    // report carries holds scalars rather than records, `compact_line` (`output.rs:211`) renders
    // each element with `scalar` and the array's own key is never written, so the line opens with a
    // bare token that no `key=value` reader can name. `grep events=` finds nothing and `wc -l`
    // counts event types as records.
    //
    // Reached by `connectors -o compact connect slack`, whose result is built at
    // `connect.rs:143` with `"events": channel.events` as its only array, and by
    // `connectors -o compact connect kubernetes` when several contexts are detected
    // (`connect.rs:196`, `"contexts"`).
    let value = json!({
        "provider": "slack",
        "connected": true,
        "connection": "beyond10x",
        "connection_ref": "connection:slack:T1",
        "events": ["message", "reaction_added"],
    });
    let rendered = rendered("compact", &value);
    assert!(
        rendered.contains("events"),
        "the only array the report carries lost its name, so its two elements arrive as bare \
         tokens a `key=value` reader cannot address:\n{rendered}"
    );
}

#[test]
fn a_column_the_budget_squeezes_to_nothing_pushes_every_later_column_out_of_line() {
    // The degenerate end of `fit_to_budget`: a column whose *name* is empty has a phase-one floor
    // of zero (`output.rs:409`), so the budget may squeeze its width to zero. `elide`
    // (`output.rs:493`) cannot honour a width of zero — it keeps one column back for the mark and
    // then pushes the mark unconditionally (`output.rs:508`) — so every cell in that column
    // occupies one terminal column the layout did not allocate, while the header cell, being
    // already empty, occupies none. The header and every row below it are then one column out of
    // step, which is the alignment the acceptance asks for.
    //
    // Thirteen columns, twelve of them named, is what it takes to drive the phase-one floors past
    // the budget; the widths are all equal so nothing is reordered.
    let mut record = Map::new();
    record.insert(String::new(), json!("a".repeat(30)));
    for index in 0..12 {
        record.insert(format!("column_{index:02}"), json!("y".repeat(30)));
    }
    let value = json!({ "rows": [Value::Object(record)] });
    let rendered = rendered("text", &value);
    let header = rendered
        .lines()
        .find(|line| line.contains("column_00") || line.contains("column_"))
        .expect("a header naming the columns");
    let row = rendered
        .lines()
        .find(|line| line.contains('y'))
        .expect("the one record");

    // Measured on the *last* column, which is the one that carries the accumulated error of every
    // column before it — and, being uncapped, is the one cell and one name the layout never cuts,
    // so a needle addresses the same column in both lines however hard the budget squeezed the
    // rest. (The original needles, `column_` and `y`, address different columns as soon as the
    // budget cuts a name short of its underscore, which is a property of the fixture rather than
    // of the alignment under test.)
    let header_last = display_width(&header[..header.rfind("column_11").expect("a name")]);
    let row_last = display_width(&row[..row.rfind(&"y".repeat(30)).expect("a cell")]);
    assert_eq!(
        header_last, row_last,
        "the last column starts at terminal column {header_last} in the header and \
         {row_last} in the row, because a column squeezed to zero width still spends one column \
         on its elision mark:\n{rendered}"
    );
}
