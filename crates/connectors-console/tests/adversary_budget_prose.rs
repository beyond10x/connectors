//! **Adversary pass: the two prose claims `story:cli-surface-fences` added about the table
//! budget, measured.**
//!
//! Added by an adversary pass. It changes no implementation file and weakens no existing case.
//!
//! The unit rewrote `output.rs`'s module header and the `CHANGELOG.md` entry beside it, and both
//! now carry three measured numbers: that 66 of the 71 lines `connectors providers` prints are
//! wider than `TABLE_BUDGET`, and that the longest is 237 terminal columns.
//! `output_tests.rs::the_budget_is_documented_as_what_it_is_and_a_real_row_is_wider_than_it`
//! asserts only that *some* line is wider, so the three numbers are stated in two documents and
//! compared with nothing — which is the same shape as the workspace count
//! `crates/catalog-build/tests/main/dependency_fence.rs` was extended to hold in this same diff.
//!
//! Every case drives the public surface, `connectors_console::emit`, through the re-execution
//! device `crates/connectors-console/tests/adversary_readability_pass2.rs` already uses and
//! documents: `emit` writes to file descriptor 1, which libtest does not intercept.

use connectors_console::{emit, Format};
use serde_json::{json, Value};

const VALUE_ENV: &str = "ADVERSARY3_RENDER_VALUE";
const OPEN: &str = "<<<adversary3-render-begin>>>";
const CLOSE: &str = "<<<adversary3-render-end>>>";

/// The terminal width `output.rs` lays a table out for, as `TABLE_BUDGET` declares it.
const TABLE_BUDGET: usize = 120;

/// The other half of [`rendered`]. Inert unless the parent set `ADVERSARY3_RENDER_VALUE`.
#[test]
fn pass3_render_helper_child() {
    let Ok(raw) = std::env::var(VALUE_ENV) else {
        return;
    };
    let value: Value = serde_json::from_str(&raw).expect("the value the parent handed over");
    emit(Format::Text, &json!(OPEN)).expect("the opening sentinel");
    emit(Format::Text, &value).expect("the value under test");
    emit(Format::Text, &json!(CLOSE)).expect("the closing sentinel");
}

/// Exactly the bytes `emit` puts on stdout for `value` in `Format::Text`.
fn rendered(value: &Value) -> String {
    let exe = std::env::current_exe().expect("this test binary");
    let output = std::process::Command::new(&exe)
        .args([
            "--exact",
            "pass3_render_helper_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(VALUE_ENV, serde_json::to_string(value).expect("json"))
        .output()
        .expect("the helper child runs");
    assert!(
        output.status.success(),
        "the helper child failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf-8 on stdout");
    let start = stdout.find(OPEN).expect("the opening sentinel") + OPEN.len();
    let end = stdout.find(CLOSE).expect("the closing sentinel");
    stdout[start..end].trim_matches('\n').to_owned()
}

/// `output.rs`'s own `display_width`, which is what the budget is spent in.
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

fn repository_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the repository root")
        .to_path_buf()
}

/// **The three numbers `output.rs` and `CHANGELOG.md` state about `connectors providers` are the
/// measured ones.**
///
/// Both documents say "66 of the 71 lines" and "the longest is 237". Neither number is read from
/// anything: the case the unit added asserts only that the count of over-wide lines is above zero,
/// so a catalogue that gains or loses a provider leaves two documents stating a width nobody
/// prints. `the_gate_and_the_release_workflow_state_the_gates_own_workspace_count`, added in this
/// same diff, is the same rule applied to a different pair of documents.
#[test]
fn the_widths_the_documents_state_are_the_widths_the_renderer_prints() {
    let rendered = rendered(&connectors_console::providers::run(""));
    let widths: Vec<usize> = rendered.lines().map(display_width).collect();
    let total = widths.len();
    let over = widths.iter().filter(|width| **width > TABLE_BUDGET).count();
    let longest = widths.iter().max().copied().unwrap_or(0);

    let claim = format!("{over} of the {total} lines");
    let longest_claim = longest.to_string();
    let mut wrong = Vec::new();
    for document in ["crates/connectors-console/src/output.rs", "CHANGELOG.md"] {
        let text = std::fs::read_to_string(repository_root().join(document))
            .unwrap_or_else(|error| panic!("read {document}: {error}"));
        let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if !one_line.contains(&claim) {
            wrong.push(format!("{document} does not state `{claim}`"));
        }
        if !one_line.contains(&longest_claim) {
            wrong.push(format!(
                "{document} does not state the longest line, {longest_claim}"
            ));
        }
    }

    assert!(
        wrong.is_empty(),
        "`connectors providers` prints {total} lines, {over} of them wider than {TABLE_BUDGET}, \
         the longest at {longest}. The documents say otherwise:\n  {}",
        wrong.join("\n  ")
    );
}

/// **The sentence the pass-two adversary suite quotes out of `output.rs` is still in `output.rs`,
/// at the line it cites.**
///
/// `crates/connectors-console/tests/adversary_readability_pass2.rs` explains
/// `the_last_column_of_providers_begins_one_column_past_the_terminal_it_is_laid_out_for` by
/// quoting the module header — "so that the last column starts on screen" — and citing
/// `output.rs:30`. That was the line at the base commit. This unit rewrote the header and inserted
/// nine lines into it without touching the file that cites it, so the citation now lands on a
/// different sentence and the quoted one is nowhere in the file.
///
/// This is the class `every_citation_this_unit_wrote_resolves` and
/// `every_citation_of_the_specification_resolves` were added for. Neither reads this crate.
#[test]
fn the_quoted_module_header_sentence_is_at_the_line_the_pass_two_suite_cites() {
    let root = repository_root();
    let citing = std::fs::read_to_string(
        root.join("crates/connectors-console/tests/adversary_readability_pass2.rs"),
    )
    .expect("the pass-two adversary suite");
    let quoted = "so that the last column starts on screen";
    assert!(
        citing.contains(quoted),
        "the pass-two suite no longer quotes `{quoted}`; this case was written for that quotation"
    );

    let header = std::fs::read_to_string(root.join("crates/connectors-console/src/output.rs"))
        .expect("output.rs");
    let lines: Vec<&str> = header.lines().collect();
    assert!(
        header.contains(quoted),
        "crates/connectors-console/tests/adversary_readability_pass2.rs quotes \
         `{quoted}` out of the module header of crates/connectors-console/src/output.rs and \
         cites `output.rs:30`. No line of that file carries the sentence any more, and line 30 is \
         now `{}`.",
        lines.get(29).copied().unwrap_or("<past end>").trim()
    );
    assert!(
        lines
            .get(29)
            .is_some_and(|line| line.contains("the last column starts on screen")),
        "output.rs:30 is `{}`, and that is the line the pass-two adversary suite cites for the \
         sentence it quotes",
        lines.get(29).copied().unwrap_or("<past end>").trim()
    );
}
