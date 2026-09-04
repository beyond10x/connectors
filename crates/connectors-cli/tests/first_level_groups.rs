//! **The top level reads as categories, and every path that moved still works.**
//!
//! `story:cli-first-level-groups`. `crates/connectors-cli/tests/cli_surface.rs` holds the parser
//! against `ess/system/components.yaml` — what the surface *is* declared to be. This file holds the
//! two things that story asks for as behaviour a person can observe: the first level is eight words
//! rather than sixteen, and a command typed the way it was typed before this release still runs,
//! producing the same output as the new path plus one line on stderr naming where it went.
//!
//! The comparison is driven through the built binary rather than through the parser, because the
//! rewrite happens in `run_from` before clap is handed anything and a parser-level check would not
//! see it at all.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The whole first level, in the order `connectors --help` prints it.
const TOP_LEVEL: &[&str] = &[
    "setup",
    "inspect",
    "session",
    "serve",
    "connection",
    "event",
    "operation",
    "admin",
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the repository root")
        .to_path_buf()
}

/// **Every path that moved, and where it went**, read out of `MOVED` in
/// `crates/connectors-cli/src/lib.rs`.
///
/// The table this file checks is the compatibility promise the story makes: old paths keep working
/// for one release, and each says its new name. It is *read* rather than restated because a
/// restatement drifts, and this one did: an adversary pass found `connectors auth` — a first-level
/// group at the release before this one — missing from the shipped table, and the copy that stood
/// here could not have shown that, because it was the same copy.
fn moved() -> Vec<(Vec<String>, Vec<String>)> {
    let source =
        std::fs::read_to_string(repository_root().join("crates/connectors-cli/src/lib.rs"))
            .expect("read crates/connectors-cli/src/lib.rs");
    let table = source
        .split_once("const MOVED:")
        .expect("crates/connectors-cli/src/lib.rs declares the compatibility table")
        .1
        .split_once("\n];")
        .expect("the table closes")
        .0;
    let words = |text: &str| {
        text.split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect::<Vec<String>>()
    };
    let pairs: Vec<(Vec<String>, Vec<String>)> = table
        .lines()
        .filter_map(|line| line.trim().strip_prefix("(&["))
        .filter_map(|entry| entry.split_once("], &["))
        .map(|(old, new)| (words(old), words(new.split("],").next().unwrap_or(new))))
        .filter(|(old, new)| !old.is_empty() && !new.is_empty())
        .collect();
    assert!(
        pairs.len() >= 12 && pairs.iter().any(|(old, _)| old == &["auth"]),
        "the compatibility table was read as {pairs:?}; it moved, so read it again before \
         believing any result from this file"
    );
    pairs
}

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

/// **`connectors --help` lists eight commands.**
///
/// The defect the story names is that it listed sixteen in one block, so the output did not tell an
/// operator which of them they wanted. Both the set and its size are asserted: a ninth word added
/// later is as much a regression as a missing one.
#[test]
fn the_first_level_is_eight_words() {
    let parser = connectors_cli::command();
    let carried: Vec<&str> = parser
        .get_subcommands()
        .map(clap::Command::get_name)
        .collect();
    assert_eq!(
        carried,
        TOP_LEVEL,
        "`connectors --help` lists {} commands: {carried:?}",
        carried.len()
    );

    let help = stdout(&run(&["--help"]));
    for word in TOP_LEVEL {
        assert!(
            help.contains(word),
            "`connectors --help` does not name `{word}`:\n{help}"
        );
    }
    for gone in ["serve-hosted", "logout", "doctor", "providers"] {
        assert!(
            !help.contains(gone),
            "`connectors --help` still lists `{gone}` at the first level:\n{help}"
        );
    }
}

/// What has to follow a row's old words for the old *leaf* to be meant rather than the group that
/// now carries its name. Empty for every row but `serve`.
///
/// Bare `connectors serve` is the `serve` group, exactly as bare `connectors setup` is the `setup`
/// group, and `connectors serve --help` is the group's help; the row `serve` → `serve local` fires
/// only on what the group refuses and the old leaf took. So the old leaf is driven with the first
/// long option the new leaf declares — `--config` — which the group has no answer to. Read off the
/// parser rather than named, so a renamed option needs no edit here.
fn what_tells_the_old_leaf_from_the_group(old: &[String], new: &[String]) -> Vec<&'static str> {
    let tree = connectors_cli::command();
    if tree.find_subcommand(&old[0]).is_none() {
        return Vec::new();
    }
    let mut leaf = &tree;
    for word in new {
        leaf = leaf.find_subcommand(word).unwrap_or_else(|| {
            panic!("`connectors {}` is not a path of the parser", new.join(" "))
        });
    }
    let (flag, takes_value) = leaf
        .get_arguments()
        .filter(|argument| !argument.is_global_set())
        .find_map(|argument| {
            argument
                .get_long()
                .map(|long| (format!("--{long}"), argument.get_action().takes_values()))
        })
        .unwrap_or_else(|| {
            panic!(
                "`connectors {}` declares no long option, so nothing tells it from the group `{}`",
                new.join(" "),
                old[0]
            )
        });
    let flag: &'static str = Box::leak(flag.into_boxed_str());
    if takes_value {
        vec![flag, "x"]
    } else {
        vec![flag]
    }
}

/// **A path that moved still works, and one line on stderr says where it went.**
///
/// `--help` is what each pair is driven with: it exercises the rewrite for every entry of the table
/// without starting a server, opening a browser or touching a credential, and the rendered help is
/// the parser's own account of the path it ended up at — so identical output is identical
/// destination. The one row whose old word is also a group of this release carries the leaf's own
/// option in front of the `--help`, for the reason [`what_tells_the_old_leaf_from_the_group`] gives.
#[test]
fn every_moved_path_still_works_and_names_where_it_went() {
    for (old, new) in moved() {
        let telling = what_tells_the_old_leaf_from_the_group(&old, &new);
        let mut legacy: Vec<&str> = old.iter().map(String::as_str).collect();
        legacy.extend(&telling);
        legacy.push("--help");
        let mut current: Vec<&str> = new.iter().map(String::as_str).collect();
        current.extend(&telling);
        current.push("--help");

        let legacy = run(&legacy);
        let current = run(&current);

        assert_eq!(
            stdout(&legacy),
            stdout(&current),
            "`connectors {}` and `connectors {}` render different help",
            old.join(" "),
            new.join(" ")
        );
        assert_eq!(
            legacy.status.code(),
            current.status.code(),
            "`connectors {}` and `connectors {}` exit differently",
            old.join(" "),
            new.join(" ")
        );

        let note = stderr(&legacy);
        assert_eq!(
            note.lines().count(),
            1,
            "`connectors {}` writes {} lines to stderr and the promise is exactly one naming the \
             new path:\n{note}",
            old.join(" "),
            note.lines().count()
        );
        assert!(
            note.contains(&format!("connectors {}", new.join(" "))),
            "`connectors {}` says nothing about `connectors {}`:\n{note}",
            old.join(" "),
            new.join(" ")
        );
    }
}

/// **`connectors doctor` produces the same report as `connectors inspect doctor`.**
///
/// The acceptance statement's own example, run rather than parsed. Both are given the same explicit
/// configuration and state root so the report is a function of its arguments and nothing else.
///
/// "The same output, plus one stderr line" is asserted as exactly that: this installation is not
/// healthy, so both paths also print a refusal on stderr, and the legacy invocation's stderr has to
/// be that same refusal with one line in front of it. Asserting only that the note is present would
/// pass on a legacy path that swallowed the refusal.
#[test]
fn doctor_reports_the_same_installation_at_both_paths() {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("doctor-both-paths");
    let config = root.join("connectors.toml");
    let state = root.join("state");
    let paths = [
        "--config",
        config.to_str().expect("a utf-8 temporary path"),
        "--state-root",
        state.to_str().expect("a utf-8 temporary path"),
    ];

    let legacy = run(&[&["doctor"][..], &paths].concat());
    let current = run(&[&["inspect", "doctor"][..], &paths].concat());

    assert_eq!(
        stdout(&legacy),
        stdout(&current),
        "`connectors doctor` and `connectors inspect doctor` report differently"
    );
    assert_eq!(legacy.status.code(), current.status.code());

    let carried = stderr(&legacy);
    let (note, rest) = carried.split_once('\n').unwrap_or_else(|| {
        panic!("`connectors doctor` writes a note and then the report:\n{carried}")
    });
    assert!(
        note.contains("connectors inspect doctor"),
        "`connectors doctor` does not name its new path:\n{note}"
    );
    assert_eq!(
        rest,
        stderr(&current),
        "`connectors doctor` writes more than one line the new path does not"
    );
}

/// **A path the new tree already carries is not rewritten.**
///
/// `serve` is both a word that moved and the group it moved into, so a table applied to the first
/// word alone turns `connectors serve hosted` into `connectors serve local hosted`. Every new path
/// under a group whose name used to be a command is checked, and silence on stderr is the evidence
/// that the shim did not fire.
#[test]
fn a_path_of_the_new_tree_is_left_alone() {
    for path in [
        &["serve", "local"][..],
        &["serve", "hosted"],
        &["serve", "mcp"],
        &["setup", "init"],
        &["inspect", "auth"],
        &["session", "login"],
    ] {
        let mut arguments: Vec<&str> = path.to_vec();
        arguments.push("--help");
        let output = run(&arguments);
        assert!(
            stderr(&output).is_empty(),
            "`connectors {}` is a path of this release and the compatibility shim fired on it:\n{}",
            path.join(" "),
            stderr(&output)
        );
        assert!(
            output.status.success(),
            "`connectors {} --help` failed:\n{}",
            path.join(" "),
            stderr(&output)
        );
    }
}

/// **Bare `serve`, `serve --help` and `serve -h` reach the group, like the other three groups.**
///
/// `serve` is the one group whose name is also a row of `MOVED`, and the row shadowed the group:
/// `connectors serve --help` printed `Usage: connectors serve local [OPTIONS]` behind a
/// deprecation note, and bare `connectors serve` published a readiness document and blocked as a
/// server, while bare `connectors setup`, `inspect` and `session` each printed their `Commands:`
/// listing. Only `connectors help serve` reached the group. `a_path_of_the_new_tree_is_left_alone`
/// drove `serve local|hosted|mcp` and never bare `serve`, so nothing caught it.
///
/// `setup` is the control: whatever it does for each of the three invocations, `serve` has to do
/// the same — same exit code, a `Commands:` listing naming every leaf, no note and no leaf's usage.
#[test]
fn the_serve_group_answers_bare_and_with_help_like_the_other_groups() {
    for arguments in [&[][..], &["--help"], &["-h"]] {
        let control = run(&[&["setup"][..], arguments].concat());
        let serve = run(&[&["serve"][..], arguments].concat());
        let said = format!("{}{}", stdout(&serve), stderr(&serve));
        assert_eq!(
            serve.status.code(),
            control.status.code(),
            "`connectors serve {}` exits {:?} and `connectors setup {}` exits {:?}:\n{said}",
            arguments.join(" "),
            serve.status.code(),
            arguments.join(" "),
            control.status.code()
        );
        assert!(
            said.contains("Commands:")
                && ["local", "hosted", "mcp"]
                    .iter()
                    .all(|leaf| said.contains(leaf)),
            "`connectors serve {}` does not list the group's commands:\n{said}",
            arguments.join(" ")
        );
        assert!(
            !said.contains("note:") && !said.contains("connectors serve local"),
            "`connectors serve {}` is the group, and the compatibility shim fired on it:\n{said}",
            arguments.join(" ")
        );
    }
}
