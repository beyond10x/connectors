//! **Adversary pass 3 on `story:cli-first-level-groups`: the compatibility shim, constructed against.**
//!
//! `crates/connectors-cli/tests/first_level_groups.rs` drives every entry of `MOVED` as a bare
//! `connectors <old> --help`. That is one shape of invocation out of several a person actually
//! typed before this release, and the shim in `crates/connectors-cli/src/lib.rs`, when this pass
//! was written, matched only literal words at `argv[1]`, with a guard that asked an *unbuilt*
//! `clap::Command` what its subcommands were. Each finding below names that shim in the present
//! tense; the cases are kept as written, and each now passes against the shim that replaced it.
//!
//! Each test below is one invocation shape the release before this one accepted, or one path this
//! release carries, driven through the built binary the way the shim is reached.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The compatibility table, the one the binary ships: `connectors_cli::MOVED`. This file restated
/// it once, eleven rows against the twelve shipped, and the row it lacked — `auth` alone — was the
/// row its cases therefore never drove. Pass 4 found that; nothing here is copied any more.
const MOVED: &[(&[&str], &[&str])] = connectors_cli::MOVED;

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

/// **A path that moved, typed with the global format flag in front of it, still works.**
///
/// `--output` / `-o` is declared `global = true` on the top-level `Cli`'s `output` field
/// (`crates/connectors-cli/src/lib.rs:40`), so it was accepted before the subcommand at the
/// release this one replaces, and `connectors -o compact doctor` is written into a shipped source
/// of this repository (`crates/connectors-console/src/output.rs:176`) as the invocation a reader
/// should picture. `moved()` reads `arguments.iter().skip(1).take(2)` and compares literal words,
/// so an argv whose second element is a flag matches no entry and is handed to clap unchanged.
///
/// The promise the story makes is "a command typed the way it was typed before this release still
/// runs, producing the same output as the new path plus one line on stderr naming where it went".
/// This is that promise, with the flag the tool documents in front of the word.
///
/// The `serve` row is driven with the leaf's own `--config` in front of the `--help`: a bare
/// `serve --help` is the `serve` *group's* help, exactly as `setup --help` is the `setup` group's,
/// and the row fires only on what the group refuses. As first written this case drove every row as
/// `<old> --help` alone, which for `serve` asserted that the group's help had to be the leaf's —
/// the shadowing `first_level_groups.rs::the_serve_group_answers_bare_and_with_help_like_the_other_groups`
/// exists to refuse.
#[test]
fn a_moved_path_typed_with_the_global_output_flag_still_works() {
    let mut broken = Vec::new();
    for (old, new) in MOVED {
        let telling: &[&str] = if connectors_cli::command().find_subcommand(old[0]).is_some() {
            &["--config", "x"]
        } else {
            &[]
        };
        let mut legacy: Vec<&str> = vec!["--output", "json"];
        legacy.extend_from_slice(old);
        legacy.extend_from_slice(telling);
        legacy.push("--help");
        let mut current: Vec<&str> = vec!["--output", "json"];
        current.extend_from_slice(new);
        current.extend_from_slice(telling);
        current.push("--help");

        let legacy_output = run(&legacy);
        let current_output = run(&current);

        if !legacy_output.status.success() || stdout(&legacy_output) != stdout(&current_output) {
            broken.push(format!(
                "`connectors --output json {}` exited {:?} and does not render `connectors \
                 --output json {}`; its stderr was:\n      {}",
                old.join(" "),
                legacy_output.status.code(),
                new.join(" "),
                stderr(&legacy_output).trim().replace('\n', "\n      "),
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "the compatibility shim does not fire when the global format flag precedes the word that \
         moved, so these invocations worked before this release and do not now:\n  {}",
        broken.join("\n  ")
    );
}

/// **`connectors help <a path that moved>` still names something.**
///
/// `connectors --help` lists `help` as a first-level word — it is clap's own subcommand and the
/// documented way to ask about a command without typing its flags — so `connectors help doctor`
/// is a path the product taught and the release before this one answered. `moved()` matches the
/// table against `argv[1]`, which is `help` here, and no entry begins with it.
#[test]
fn connectors_help_still_answers_for_a_path_that_moved() {
    let listed = stdout(&run(&["--help"]));
    assert!(
        listed.contains("\n  help"),
        "`connectors --help` does not offer a `help` subcommand, so this test is measuring the \
         wrong thing:\n{listed}"
    );

    let mut broken = Vec::new();
    for (old, _) in MOVED {
        let mut arguments: Vec<&str> = vec!["help"];
        arguments.extend_from_slice(old);
        let output = run(&arguments);
        if !output.status.success() {
            broken.push(format!(
                "`connectors help {}` exited {:?}: {}",
                old.join(" "),
                output.status.code(),
                stderr(&output).lines().next().unwrap_or("").trim(),
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "`connectors help <word>` is the path `connectors --help` itself advertises, and the shim \
         does not reach it because the word that moved is not argv[1]:\n  {}",
        broken.join("\n  ")
    );
}

/// **The group word whose only command moved still answers.**
///
/// `auth` was a group with one variant, `status`; `connectors auth` and `connectors auth --help`
/// listed it. `MOVED` carries the two-word path `auth status` alone, and `moved()` requires
/// `old.len() <= leading.len()` with every word equal, so a one-word `auth` matches nothing: the
/// person who typed the group is told `auth` is unrecognized, with no mention of `inspect auth`.
#[test]
fn the_group_word_whose_only_command_moved_still_points_somewhere() {
    let mut broken = Vec::new();
    for arguments in [&["auth"][..], &["auth", "--help"]] {
        let output = run(arguments);
        let said = format!("{}{}", stdout(&output), stderr(&output));
        if !said.contains("inspect auth") {
            broken.push(format!(
                "`connectors {}` exited {:?} and says nothing about `connectors inspect auth`:\n \
                 {}",
                arguments.join(" "),
                output.status.code(),
                said.trim().replace('\n', "\n     "),
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "`auth` was a first-level group before this release and its one command moved; neither \
         invocation of the group names where it went:\n  {}",
        broken.join("\n  ")
    );
}

/// **The `help` subcommand `connectors help serve` advertises exists.**
///
/// Recorded on its own so the test below is known to be measuring a path a person is offered
/// rather than one this file invented. clap builds a `help` subcommand under every command that
/// carries subcommands, and the group's own help output lists it.
#[test]
fn the_serve_group_advertises_a_help_subcommand() {
    let listed = stdout(&run(&["help", "serve"]));
    assert!(
        listed.contains("\n  help"),
        "`connectors help serve` does not list a `help` subcommand under the group:\n{listed}"
    );
}

/// **A path of this release under `serve` is not rewritten by the shim.**
///
/// `first_level_groups.rs::a_path_of_the_new_tree_is_left_alone` checks `serve local`, `serve
/// hosted` and `serve mcp`. The guard it exercises asks `command()` — `Cli::command()`, which the
/// derive returns *unbuilt* — whether the second word is a subcommand of the first. clap adds the
/// `help` subcommand in `_build_self`, so `find_subcommand("help")` answers `None` on that value
/// and the guard does not see the fourth command the group actually carries. `connectors serve
/// help` is therefore rewritten to `connectors serve local help`, which clap refuses.
///
/// `serve` is the only group this can reach, because it is the only group word that is also an
/// entry of `MOVED`.
#[test]
fn a_help_path_of_the_new_tree_under_serve_is_left_alone() {
    let mut broken = Vec::new();
    for path in [
        &["serve", "help"][..],
        &["serve", "help", "local"],
        &["serve", "help", "hosted"],
        &["serve", "help", "mcp"],
    ] {
        let output = run(path);
        if !output.status.success() || !stderr(&output).is_empty() {
            broken.push(format!(
                "`connectors {}` exited {:?} with stderr:\n      {}",
                path.join(" "),
                output.status.code(),
                stderr(&output).trim().replace('\n', "\n      "),
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "these are paths of this release's own tree and the compatibility shim fired on them:\n  \
         {}",
        broken.join("\n  ")
    );
}

/// **A moved path written with a global flag between the binary and the word is caught too.**
///
/// `crates/connectors-cli/tests/moved_paths_are_not_taught.rs` searches shipped sources for the
/// literal `connectors <old words>`, so `connectors -o compact doctor` is invisible to it. That
/// string is not merely advice with a removal date on it, which is the class that fence names: it
/// is an invocation this release refuses outright, because the shim does not fire behind a flag.
///
/// The scope rules are the fence's own — `#[cfg(test)]` cut off, `//!` and plain `//` skipped,
/// `///` in — so a site this refuses is a site that fence claims to cover.
#[test]
fn nothing_this_product_prints_names_a_moved_path_behind_a_global_flag() {
    fn visit(path: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|name| name.to_str()) != Some("target") {
                    visit(&path, found);
                }
            } else if path.extension().and_then(|name| name.to_str()) == Some("rs")
                && path
                    .components()
                    .any(|component| component.as_os_str() == "src")
                && !path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with("_tests.rs"))
            {
                found.push(path);
            }
        }
    }

    let root = repository_root();
    let mut files = Vec::new();
    visit(&root.join("crates"), &mut files);
    files.sort();
    assert!(
        files.len() > 50,
        "only {} shipped sources were found under crates/; the layout moved",
        files.len()
    );

    let mut teaching = Vec::new();
    for file in files {
        let source = std::fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
        let shipped = source
            .split("\n#[cfg(test)]")
            .next()
            .unwrap_or("")
            .to_owned();
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        for (number, line) in shipped.lines().enumerate() {
            let head = line.trim_start();
            if head.starts_with("//!") || (head.starts_with("//") && !head.starts_with("///")) {
                continue;
            }
            for (old, new) in MOVED {
                for flag in ["-o", "--output"] {
                    for format in ["text", "json", "yaml", "compact"] {
                        let named = format!("connectors {flag} {format} {}", old.join(" "));
                        if line.contains(&named) {
                            teaching.push(format!(
                                "{relative}:{}: {}\n      `{named}` is refused by this release; \
                                 the shim reads argv[1] and finds `{flag}`. It is `connectors \
                                 {flag} {format} {}` now.",
                                number + 1,
                                line.trim(),
                                new.join(" "),
                            ));
                        }
                    }
                }
            }
        }
    }

    assert!(
        teaching.is_empty(),
        "these name a path `MOVED` rewrites, behind the global format flag, which is a shape \
         crates/connectors-cli/tests/moved_paths_are_not_taught.rs does not match:\n  {}",
        teaching.join("\n  ")
    );
}
