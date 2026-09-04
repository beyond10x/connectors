//! Adversarial cases against `story:informative-command-readability`.
//!
//! Every case here drives the **public** surface — `connectors_console::output::emit`, the same
//! function the binary calls — rather than the crate-private renderer, so what is measured is what
//! an operator's terminal receives.
//!
//! `emit` writes straight to file descriptor 1, which libtest's capture does not intercept, so a
//! case that wants the bytes re-executes this very test binary with `--exact render_helper_child`
//! and reads the child's stdout between two sentinels. The child is the production path: real
//! `emit`, real formatting, real newline handling.

use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;

use connectors_console::{emit, Format};
use serde_json::{json, Value};

const VALUE_ENV: &str = "ADVERSARY_RENDER_VALUE";
const FORMAT_ENV: &str = "ADVERSARY_RENDER_FORMAT";
const OPEN: &str = "<<<adversary-render-begin>>>";
const CLOSE: &str = "<<<adversary-render-end>>>";

/// The other half of [`rendered`]. Inert unless the parent set `ADVERSARY_RENDER_VALUE`.
#[test]
fn render_helper_child() {
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
            "render_helper_child",
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

/// How many terminal columns a string occupies, which is not how many `char`s it holds.
///
/// The East Asian Wide and Fullwidth ranges, which is all this needs: every other character in
/// these fixtures is one column.
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

/// The four glyphs the marker column is allowed to carry.
const MARKERS: [char; 4] = ['+', '!', 'x', '?'];

/// A `doctor` report read off a configuration with an ordinary TOML typo in it.
///
/// `check_config` folds the parse failure into the check's `detail`, and `toml::de::Error`
/// renders its caret diagram over six lines. Nothing in the pipeline flattens it.
fn doctor_report_over_a_malformed_configuration() -> (Value, tempfile::TempDir) {
    let directory = tempfile::tempdir().expect("a temporary state root");
    let config = directory.path().join("connectors.toml");
    let mut file = std::fs::File::create(&config).expect("a configuration file");
    // A missing pair of quotes. The most ordinary way to break a TOML file by hand.
    file.write_all(b"[owner]\ntenant_id = beyond10x\n")
        .expect("the malformed configuration is written");
    file.sync_all().expect("flushed");
    drop(file);
    std::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o600))
        .expect("owner-only, which is what the trusted reader requires");
    let report = connectors_console::doctor::run(&config, directory.path());
    (report.to_value(), directory)
}

#[test]
fn doctor_spreads_one_check_over_several_unmarked_lines_when_the_configuration_is_malformed() {
    // Acceptance: "`connectors doctor` ... renders in `text` as one aligned row per record with the
    // severity ... of a row distinguishable without reading its text."
    //
    // A TOML syntax error is the single most likely reason somebody runs `doctor` at all, and the
    // parse failure it reports is six lines tall. The table has no fold for that: the newlines go
    // into the cell verbatim, so one record becomes six lines, five of them with nothing in the
    // marker column and no column boundary anywhere.
    let (report, _directory) = doctor_report_over_a_malformed_configuration();
    let checks = report["checks"].as_array().expect("the checks").len();
    let rendered = rendered("text", &report);

    let header = rendered
        .lines()
        .position(|line| line.contains("check") && line.contains("detail"))
        .expect("a header naming the columns");
    let summary = rendered
        .lines()
        .position(|line| line.starts_with("healthy:"))
        .expect("the summary line beside the table");
    let body: Vec<&str> = rendered.lines().collect::<Vec<_>>()[header + 1..summary].to_vec();

    assert_eq!(
        body.len(),
        checks,
        "{checks} checks were rendered over {} lines, so a record is not one row:\n{rendered}",
        body.len()
    );
    for (offset, line) in body.iter().enumerate() {
        let marker = line.chars().nth(2);
        assert!(
            marker.is_some_and(|glyph| MARKERS.contains(&glyph)),
            "row {offset} carries {marker:?} where the severity marker belongs, so its severity \
             cannot be read without reading its text:\n{rendered}"
        );
    }
}

#[test]
fn providers_starts_its_last_column_past_the_width_of_any_terminal() {
    // `output.rs` states the reason the widest column is moved last: "a free-text `detail` in the
    // middle pushes every column after it past the width of a terminal and the row wraps, which is
    // the thing one row per record exists to prevent."
    //
    // For the command that motivated the change that reordering cannot help, because the eleven
    // columns *before* the moved one are already wider than a terminal on their own. Measured
    // against the offset the last column starts at, so the assertion holds however short the last
    // cell happens to be: a row that begins its last field at column 196 has already wrapped.
    let all = connectors_console::providers::run("");
    let rendered = rendered("text", &all);
    let header = rendered
        .lines()
        .find(|line| line.contains("provider") && line.contains("credentials"))
        .expect("a header naming the columns");
    let last_column_starts = header
        .rfind("credentials")
        .expect("the column the reordering moved last");
    assert!(
        last_column_starts <= 120,
        "`connectors providers` begins its last column at terminal column {last_column_starts} \
         (widest row {} columns), so every row wraps and no column after the first is aligned on \
         screen:\n{header}",
        rendered
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or_default()
    );
}

#[test]
fn compact_no_longer_puts_one_record_on_every_line() {
    // `Format::Compact` still documents itself as "One record per line, for `grep` and `wc -l`"
    // (output.rs, unchanged by this unit). The unit added a summary line at the top of the same
    // stream, spelled in the same `key=value` vocabulary as a record and distinguished from one by
    // nothing a reader or a script can test. `connectors -o compact providers | wc -l` answered 65
    // before this change and answers 66 after it, and the first line an `awk -F'\t'` loop now sees
    // is `summary.listed=65`.
    let all = connectors_console::providers::run("");
    let records = all["providers"].as_array().expect("the records").len();
    let rendered = rendered("compact", &all);
    let lines: Vec<&str> = rendered.lines().collect();
    let strays: Vec<&&str> = lines
        .iter()
        .filter(|line| !line.contains("provider="))
        .collect();
    assert!(
        strays.is_empty(),
        "{} of {} lines are not records: {strays:?}",
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
fn a_wide_character_cell_leaves_the_column_after_it_ragged() {
    // Column widths are counted in `char`s (`row[index].chars().count()`), and a terminal lays out
    // columns. A CJK or fullwidth character occupies two of them, so every cell after the widest
    // one in that row is displaced by one column per wide character — the alignment the acceptance
    // asks for, lost.
    //
    // Reached by `connectors connect grafana`, whose `targets` rows carry a `label` chosen on the
    // Grafana side (src/connect.rs:161), and by `connectors auth status`, whose `instance` column
    // is free-form user text: `CatalogIntegrationConfig::instance` is an `Option<String>` and
    // `validate` constrains only `provider` and `grant_ref`.
    let value = json!({
        "provider": "grafana",
        "connected": true,
        "connection_ref": "connection:grafana:main",
        "targets": [
            {
                "label": "本番Prometheus",
                "integration_ref": "integration:prometheus",
                "connection_ref": "connection:prom:a",
            },
            {
                "label": "staging",
                "integration_ref": "integration:prometheus",
                "connection_ref": "connection:prom:b",
            },
        ],
    });
    let rendered = rendered("text", &value);
    let offsets: Vec<usize> = rendered
        .lines()
        .filter_map(|line| {
            line.find("integration:prometheus")
                .map(|byte| display_width(&line[..byte]))
        })
        .collect();
    assert_eq!(offsets.len(), 2, "two target rows:\n{rendered}");
    assert_eq!(
        offsets[0], offsets[1],
        "the column after `label` starts at terminal column {} on the row with a wide-character \
         label and at {} on the row without one:\n{rendered}",
        offsets[0], offsets[1]
    );
}

#[test]
fn an_unranked_table_lets_a_cell_sit_where_the_severity_marker_sits() {
    // The marker column exists only when some record in the table carries a rank. `connect
    // grafana`'s `targets` carry none, so their first cell starts exactly where `doctor`'s and
    // `providers`' markers do, at column 2 of the row. A label beginning `x ` — chosen on the
    // Grafana side, not here — is then indistinguishable from a row this renderer marked as failed,
    // in the one position the acceptance reserves for severity.
    let value = json!({
        "targets": [
            {"label": "x marks the spot", "connection_ref": "connection:prom:a"},
            {"label": "ok", "connection_ref": "connection:prom:b"},
        ],
    });
    let rendered = rendered("text", &value);
    let row = rendered
        .lines()
        .find(|line| line.contains("marks the spot"))
        .expect("the row");
    let occupant = row.chars().nth(2);
    assert!(
        !occupant.is_some_and(|glyph| MARKERS.contains(&glyph) && row.chars().nth(3) == Some(' ')),
        "an unranked row opens with {occupant:?} in the marker column, which the reader has been \
         taught means a failed row:\n{rendered}"
    );
}

#[test]
fn a_record_whose_cells_are_all_empty_is_rendered_as_a_blank_line() {
    // "no output format drops a field the value carries". `write_row` trims the finished line, so a
    // record whose every cell is the empty string leaves a line with nothing on it — not the keys,
    // not the record, not even the `-` the table uses for a field a record does not have. The
    // generic walker this replaced printed `a:` and `b:` for the same value.
    let value = json!({"rows": [{"a": "", "b": ""}]});
    let rendered = rendered("text", &value);
    let blank = rendered
        .lines()
        .filter(|line| line.trim().is_empty())
        .count();
    assert_eq!(
        blank, 0,
        "a record was rendered as {blank} blank line(s), so nothing on screen says it exists:\n\
         {rendered:?}"
    );
}
