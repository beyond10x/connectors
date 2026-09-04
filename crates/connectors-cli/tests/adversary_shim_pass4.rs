//! **Adversary pass 4 on `story:cli-first-level-groups`: the `auth` row of the compatibility
//! table, and the copy of that table this suite still keeps by hand.**
//!
//! Pass 3 found `connectors auth` missing from `MOVED` and the unit added
//! `(&["auth"], &["inspect", "auth"])`. That row rewrites the *word* `auth` onto `inspect auth`,
//! which is a **leaf**. Everything the old first-level **group** `auth` accepted after its own name
//! — its `help` subcommand, and the global format flag between the group and `status` — therefore
//! becomes a positional argument of a leaf that takes none, and clap refuses it.
//!
//! The story's compatibility promise is "Old paths keep working for one release"
//! (`.engineering/planning/story/cli-first-level-groups.md`, `## Compatibility`), restated by
//! `crates/connectors-cli/tests/first_level_groups.rs` as "a command typed the way it was typed
//! before this release still runs". Each case below is one command that was typed that way, driven
//! through the built binary, because the rewrite happens in `run_from` before clap is handed
//! anything.
//!
//! Every case is `--help`-terminated or a `help` path, so nothing here reads a configuration,
//! opens a socket or touches a credential.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(arguments)
        .output()
        .expect("run the connectors binary")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the repository root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repository_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

/// Every `(&[old…], &[new…])` row of a `const MOVED:` table, read out of the named source.
///
/// One parser for both tables, so a difference this reports is a difference between the tables and
/// not between two ways of reading them.
fn moved_table(relative: &str) -> Vec<(Vec<String>, Vec<String>)> {
    let source = read(relative);
    let table = source
        .split_once("const MOVED:")
        .unwrap_or_else(|| panic!("{relative} declares a `const MOVED:` table"))
        .1
        .split_once("\n];")
        .unwrap_or_else(|| panic!("{relative}'s table closes"))
        .0;
    let words = |text: &str| {
        text.split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect::<Vec<String>>()
    };
    table
        .lines()
        .filter_map(|line| line.trim().strip_prefix("(&["))
        .filter_map(|entry| entry.split_once("], &["))
        .map(|(old, new)| (words(old), words(new.split("],").next().unwrap_or(new))))
        .filter(|(old, new)| !old.is_empty() && !new.is_empty())
        .collect()
}

/// **The `help` subcommand the `auth` group advertised still answers.**
///
/// At the release this one replaces, `connectors auth --help` printed a `Commands:` block naming
/// two words — `status` and `help` — so `connectors auth help` and `connectors auth help status`
/// are paths the product itself offered, in exactly the sense that made
/// `crates/connectors-cli/tests/adversary_shim_pass3.rs::the_serve_group_advertises_a_help_subcommand`
/// a control for `connectors serve help`.
///
/// The control here is the same shape at a group that did not move: `connectors inspect --help`
/// lists `help`, and `connectors inspect help auth` answers. So the shape is one this release
/// carries; it is refused only where the shim rewrote a group name onto a leaf.
#[test]
fn the_auth_group_still_answers_the_help_subcommand_it_advertised() {
    let listed = stdout(&run(&["inspect", "--help"]));
    assert!(
        listed.contains("\n  help"),
        "`connectors inspect --help` does not offer a `help` subcommand under the group, so this \
         case is measuring the wrong thing:\n{listed}"
    );
    let control = run(&["inspect", "help", "auth"]);
    assert!(
        control.status.success(),
        "`connectors inspect help auth` is `<group> help <leaf>` at a group that did not move and \
         it failed, so this case is measuring the wrong thing:\n{}",
        stderr(&control)
    );

    let mut broken = Vec::new();
    for path in [&["auth", "help"][..], &["auth", "help", "status"]] {
        let output = run(path);
        if !output.status.success() {
            broken.push(format!(
                "`connectors {}` exited {:?}:\n      {}",
                path.join(" "),
                output.status.code(),
                stderr(&output).trim().replace('\n', "\n      "),
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "`auth` was a first-level group whose `--help` listed `status` and `help`; the row \
         `(&[\"auth\"], &[\"inspect\", \"auth\"])` rewrites the group's name onto a leaf, so the \
         `help` it advertised becomes a positional argument clap refuses:\n  {}",
        broken.join("\n  ")
    );
}

/// **A two-word path that moved still works with the global format flag between its words.**
///
/// `--output` / `-o` is `global = true` on the root (`crates/connectors-cli/src/lib.rs`), so at the
/// release this one replaces it was accepted between a group and its command:
/// `connectors auth --output json status` printed the credential-store report as JSON.
///
/// `moved` steps over globals only in the run of words *before* the path, so a global inside a
/// two-word entry breaks the adjacency the table match needs. The longest-match rule then does not
/// refuse — it silently falls back to the one-word `auth` row, prints a note naming a path the
/// person did not type, and hands clap an argv it rejects.
///
/// The control is the same flag one word earlier, which the shim does step over.
#[test]
fn a_two_word_path_that_moved_works_with_the_global_flag_between_its_words() {
    for flag in [
        &["-o", "json"][..],
        &["--output", "json"],
        &["--output=json"],
        &["-ojson"],
    ] {
        let mut control: Vec<&str> = flag.to_vec();
        control.extend_from_slice(&["auth", "status", "--help"]);
        let control = run(&control);
        assert!(
            control.status.success(),
            "`connectors {}` is the same flag one word earlier and it failed, so this case is \
             measuring the wrong thing:\n{}",
            flag.join(" "),
            stderr(&control)
        );
    }

    let mut broken = Vec::new();
    for flag in [
        &["-o", "json"][..],
        &["--output", "json"],
        &["--output=json"],
        &["-ojson"],
    ] {
        let mut argument: Vec<&str> = vec!["auth"];
        argument.extend_from_slice(flag);
        argument.extend_from_slice(&["status", "--help"]);
        let output = run(&argument);
        if !output.status.success() {
            broken.push(format!(
                "`connectors {}` exited {:?}:\n      {}",
                argument.join(" "),
                output.status.code(),
                stderr(&output).trim().replace('\n', "\n      "),
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "a global option between the two words of a `MOVED` entry defeats the match, and the \
         one-word `auth` row absorbs the invocation instead of it being refused:\n  {}",
        broken.join("\n  ")
    );
}

/// **The compatibility table this suite copied by hand is the table the binary ships.**
///
/// `crates/connectors-cli/tests/adversary_shim_pass3.rs` declared its own `const MOVED` and its doc
/// comment called it "the compatibility table, as `crates/connectors-cli/src/lib.rs` declares it".
/// Three of that file's cases are driven from it, so a row the shipped table had and the copy did
/// not was a row those cases never drove — which is the same defect the unit fixed one file over
/// when it made `crates/connectors-cli/tests/first_level_groups.rs` *read* `MOVED` rather than
/// restate it, for the reason its own comment gives: "a restatement drifts, and this one did".
///
/// As first written, this case compared the copy with the shipped table row by row, and its own
/// message said to stop keeping a copy. The remedy it asked for makes its precondition false — a
/// row-by-row comparison needs a copy to compare — so the assertion is now the invariant behind it:
/// pass 3 keeps no table of its own. Its `MOVED` has to be `connectors_cli::MOVED`, the shipped
/// one, no row may be spelled out in that file, and what the crate exports has to be what this file
/// reads out of `lib.rs`, so that the two ways of reaching the table cannot disagree either.
#[test]
fn the_table_this_suite_copies_by_hand_is_the_table_the_binary_ships() {
    let shipped = moved_table("crates/connectors-cli/src/lib.rs");
    assert!(
        shipped.len() >= 12 && shipped.iter().any(|(old, _)| old == &["auth"]),
        "the shipped table was read as {shipped:?}; it moved, so read it again before believing \
         any result from this case"
    );
    let exported: Vec<(Vec<String>, Vec<String>)> = connectors_cli::MOVED
        .iter()
        .map(|(old, new)| {
            (
                old.iter().map(|word| (*word).to_owned()).collect(),
                new.iter().map(|word| (*word).to_owned()).collect(),
            )
        })
        .collect();
    assert_eq!(
        exported, shipped,
        "`connectors_cli::MOVED` is not the table this file reads out of \
         crates/connectors-cli/src/lib.rs, so one of the two readings is wrong"
    );

    let pass3 = read("crates/connectors-cli/tests/adversary_shim_pass3.rs");
    let restated: Vec<String> = pass3
        .lines()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with("(&[\""))
        .map(|(number, line)| format!("{}: {}", number + 1, line.trim()))
        .collect();
    assert!(
        restated.is_empty(),
        "crates/connectors-cli/tests/adversary_shim_pass3.rs spells out rows of `MOVED` again, \
         and a copy is what drifted:\n  {}",
        restated.join("\n  ")
    );
    assert!(
        pass3.contains("const MOVED: &[(&[&str], &[&str])] = connectors_cli::MOVED;"),
        "crates/connectors-cli/tests/adversary_shim_pass3.rs does not take its table from \
         `connectors_cli::MOVED`, so its three table-driven cases drive something other than what \
         the binary ships"
    );
}
