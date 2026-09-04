//! **Adversary pass 5 on `story:cli-first-level-groups`: the rewritten compatibility shim.**
//!
//! `moved` in `crates/connectors-cli/src/lib.rs` was rewritten to let clap read the argv: a
//! prefilter over the first `LEGACY_WINDOW` words, the real parse whose `InvalidSubcommand` and
//! `UnknownArgument` refusals mean "a word that moved", and a globals-only reader with
//! `allow_external_subcommands(true)` that walks the argv one word at a time. Each case below is one
//! argv shape the three stages read differently from the release before this one, measured against
//! the pre-release `connectors 0.5.11` binary and against this tree's.
//!
//! Every case asks `connectors_cli::moved` directly — the function `run_from` applies — and, where
//! the answer can be observed without starting a server or touching a credential, drives the built
//! binary too.

use std::ffi::OsString;
use std::process::{Command, Output};

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(arguments)
        .output()
        .expect("run the connectors binary")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn argv(words: &[&str]) -> Vec<OsString> {
    std::iter::once("connectors")
        .chain(words.iter().copied())
        .map(OsString::from)
        .collect()
}

fn shown(argv: &[OsString]) -> String {
    argv.iter()
        .map(|word| word.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// **`--` ends the options; it does not begin a path that moved.**
///
/// The release before this one refused `connectors -- doctor` with clap's own words — *"unexpected
/// argument 'doctor' found; tip: subcommand 'doctor' exists; to use it, remove the '--' before
/// it"*, exit 2 — and so does this tree for a word of its own: `connectors -- inspect --help` is
/// refused the same way. But the globals-only reader in `read_one_word` consumes `--` as the
/// escape, and with no positional declared, the word after it is admitted as the external
/// subcommand. So `-- doctor` is walked as `doctor`, the escape is dropped from the rebuilt argv,
/// and `connectors -- doctor` runs the doctor; `connectors -- serve --config x` starts the local
/// server; `connectors -- help doctor` prints help. An argv the product refused now does something,
/// and what it does is decided by discarding the one token that said "read the rest literally".
#[test]
fn the_double_dash_escape_is_not_a_word_that_moved() {
    let control = run(&["--", "inspect", "--help"]);
    assert_eq!(
        control.status.code(),
        Some(2),
        "`connectors -- inspect --help` is a word of this release behind the escape and clap \
         accepted it, so `--` is not an escape in this tree and this case measures the wrong \
         thing:\n{}",
        stderr(&control)
    );

    let mut rewritten = Vec::new();
    for words in [
        &["--", "doctor"][..],
        &["-o", "json", "--", "doctor"],
        &["--", "serve", "--config", "x"],
        &["--", "auth", "status"],
        &["--", "help", "doctor"],
    ] {
        let mut arguments = argv(words);
        if let Some((old, new)) = connectors_cli::moved(&mut arguments) {
            rewritten.push(format!(
                "`connectors {}` was rewritten to `{}` as `{}` → `{}`",
                words.join(" "),
                shown(&arguments),
                old.join(" "),
                new.join(" ")
            ));
        }
    }
    assert!(
        rewritten.is_empty(),
        "a word behind `--` was refused by the release before this one and is read as a path that \
         moved by this one, with the escape dropped from the argv handed on:\n  {}",
        rewritten.join("\n  ")
    );

    let output = run(&["--", "doctor", "--help"]);
    assert!(
        !stderr(&output).contains("note:") && output.status.code() == Some(2),
        "`connectors -- doctor --help` exited {:?} and wrote:\n{}",
        output.status.code(),
        stderr(&output)
    );
}

/// **`connectors serve` with nothing but global options is the `serve` group, in every spelling
/// and either position, and no note is written.**
///
/// The first version of this case asserted the opposite: that `connectors serve -o json`,
/// `connectors -o json serve` and `connectors serve --output=json` served, as the pre-release
/// `connectors 0.5.11` binary did when measured with `HOME` pointed at scratch, or else named
/// `connectors serve local` on stderr — and it held the *"Every old path works for one more
/// release … the global `-o`/`--output` may stand in front of the words or between them"* line of
/// `CHANGELOG.md` to be the document to keep, against the `MOVED` doc comment's *"fires only when
/// what follows it is something the group refuses"*. That is the resolution that was not taken.
/// The decision went the other way: bare `connectors serve` is the group, the way bare `setup`,
/// `inspect` and `session` are, and `serve` carrying only global options is the same invocation —
/// a `serve -o json` that started a server would make `serve` and `serve -o json` two different
/// commands under one name, which is the defect an earlier pass found and the rewrite removed.
/// The CHANGELOG entry was corrected to state the break instead. So this case, alone among the
/// four, was rewritten to assert what is now correct rather than what it first measured.
///
/// Nine argvs: bare `serve`, and the four spellings of the format flag in front of it and behind
/// it. `moved` leaves every one alone; the binary answers every one with the group's usage line on
/// stderr and exit 2; none carries the one-line note, because none of them is a path that moved.
#[test]
fn serve_with_only_global_options_is_the_group_in_every_spelling_and_position() {
    let spellings: [&[&str]; 4] = [
        &["-o", "json"],
        &["--output", "json"],
        &["--output=json"],
        &["-ojson"],
    ];
    let mut argvs: Vec<Vec<&str>> = vec![vec!["serve"]];
    for flag in spellings {
        let mut in_front: Vec<&str> = flag.to_vec();
        in_front.push("serve");
        let mut behind: Vec<&str> = vec!["serve"];
        behind.extend_from_slice(flag);
        argvs.push(in_front);
        argvs.push(behind);
    }

    let mut rewritten = Vec::new();
    let mut not_the_group = Vec::new();
    for words in &argvs {
        let mut arguments = argv(words);
        let outcome = connectors_cli::moved(&mut arguments);
        if outcome.is_some() || arguments != argv(words) {
            rewritten.push(format!(
                "`connectors {}` → {:?}, argv handed on: `{}`",
                words.join(" "),
                outcome.map(|(old, new)| (old.join(" "), new.join(" "))),
                shown(&arguments)
            ));
        }
        let output = run(words);
        let said = stderr(&output);
        let is_the_group = output.status.code() == Some(2)
            && said.contains("Usage: connectors serve [OPTIONS] <COMMAND>")
            && !said.contains("note:");
        if !is_the_group {
            not_the_group.push(format!(
                "`connectors {}` exited {:?} and wrote:\n      {}",
                words.join(" "),
                output.status.code(),
                said.trim().replace('\n', "\n      ")
            ));
        }
    }
    assert!(
        rewritten.is_empty(),
        "`serve` with only global options is the group, not a path that moved, and the shim \
         touched these:\n  {}",
        rewritten.join("\n  ")
    );
    assert!(
        not_the_group.is_empty(),
        "each of these is the `serve` group — exit 2, the group's usage line, no note — and these \
         were answered otherwise:\n  {}",
        not_the_group.join("\n  ")
    );
}

/// **An argument that neither the `serve` group nor `serve local` declares does not mean the old
/// leaf was typed.**
///
/// The `MOVED` doc comment: the `serve` row *"fires only when what follows it is something the
/// group refuses — `--config`, `--state-root`"*. Stage 2 implements "the group refuses it" as any
/// `UnknownArgument`, which is also what the group says to `--bogus`, `-V`, `--version` and a
/// mistyped `--hlep`, none of which the old leaf took either. So `connectors serve --hlep` now
/// writes a note that `connectors serve` is `connectors serve local` — a path the person did not
/// type — and clap's usage line names `connectors serve local --help` for an argument the leaf
/// refuses exactly as the group does. `connectors setup --hlep` is the control: the same typo at a
/// group whose name is not a row, answered by clap alone. The two have to differ only in the
/// group's name.
#[test]
fn an_argument_neither_the_group_nor_the_leaf_declares_is_not_the_old_leaf() {
    let mut rewritten = Vec::new();
    for words in [
        &["serve", "--bogus"][..],
        &["serve", "-V"],
        &["serve", "--version"],
        &["serve", "--hlep"],
    ] {
        let mut arguments = argv(words);
        if let Some((old, new)) = connectors_cli::moved(&mut arguments) {
            rewritten.push(format!(
                "`connectors {}` was rewritten to `{}` as `{}` → `{}`",
                words.join(" "),
                shown(&arguments),
                old.join(" "),
                new.join(" ")
            ));
        }
    }
    assert!(
        rewritten.is_empty(),
        "these arguments are refused by the group and by `serve local` alike, so nothing in them \
         names the old leaf, and the shim rewrote them anyway:\n  {}",
        rewritten.join("\n  ")
    );

    let control = stderr(&run(&["setup", "--hlep"]));
    assert!(
        control.contains("Usage: connectors setup"),
        "`connectors setup --hlep` did not print the group's usage, so the control is wrong:\n\
         {control}"
    );
    let expected = control.replace("setup", "serve");
    let serve = stderr(&run(&["serve", "--hlep"]));
    assert_eq!(
        serve, expected,
        "`connectors serve --hlep` is answered differently from `connectors setup --hlep`, and \
         the only difference typed was the group's name"
    );
}

/// **A positional value spelled `help` is a value, not a help request.**
///
/// `login` takes `<BASE>` and `connect` takes `<PROVIDER>`, so at the release before this one
/// `connectors login help` was a login to a base named `help` (it failed at discovery, exit 1) and
/// `connectors connect help` was a connect to a provider named `help` (it failed reading the
/// configuration, exit 1). Stage 3 takes `help` wherever it stands after the words already walked,
/// so both are now rebuilt as `help session login` and `help setup connect` and print the leaf's
/// help, exit 0. The rule that made `connectors auth help` work — `auth` was a group whose `help`
/// subcommand existed — is applied to leaves whose `help` was a value.
#[test]
fn a_positional_value_spelled_help_is_a_value_not_a_help_request() {
    let mut stolen = Vec::new();
    for (words, expected) in [
        (
            &["login", "help"][..],
            &["connectors", "session", "login", "help"][..],
        ),
        (
            &["connect", "help"],
            &["connectors", "setup", "connect", "help"],
        ),
    ] {
        let mut arguments = argv(words);
        connectors_cli::moved(&mut arguments);
        let expected: Vec<OsString> = expected.iter().map(OsString::from).collect();
        if arguments != expected {
            stolen.push(format!(
                "`connectors {}` is handed on as `{}`, not `{}`",
                words.join(" "),
                shown(&arguments),
                shown(&expected)
            ));
        }
    }
    assert!(
        stolen.is_empty(),
        "`help` after a leaf that declares a positional was that positional's value at the release \
         before this one, and the shim reads it as a help request:\n  {}",
        stolen.join("\n  ")
    );
}
