//! The explicit service commands `describe`, `invoke` and `serve` are reachable
//! through every route the root help advertises: `help COMMAND` prints the usage
//! `COMMAND --help` prints, and the Bash completion completes them as root words
//! with their own flags, not those of a generated command of the same name.

use std::collections::BTreeSet;
use std::process::{Command, Output};

const SERVICE_COMMANDS: [&str; 3] = ["describe", "invoke", "serve"];

fn cli(args: &[&str]) -> Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(args)
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_STATE_HOME")
        .output()
        .unwrap()
}

fn parts(output: &Output) -> (Option<i32>, String, String) {
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn help_command_prints_the_service_command_usage() {
    for command in SERVICE_COMMANDS {
        let expected = parts(&cli(&[command, "--help"]));
        assert_eq!(expected.0, Some(0), "`{command} --help` exits 0");
        assert!(
            expected.1.contains("Usage:") && expected.1.contains(command),
            "`{command} --help` prints the command usage: {:?}",
            expected.1
        );
        assert_eq!(
            parts(&cli(&["help", command])),
            expected,
            "`connectors help {command}` prints what `connectors {command} --help` prints",
        );
    }
}

#[test]
fn help_route_does_not_widen_the_parser() {
    for args in [
        &["help", "describe", "extra"][..],
        &["help", "serve", "--config"][..],
        &["--config", "x", "help", "invoke"][..],
    ] {
        assert_eq!(
            parts(&cli(args)),
            (Some(2), String::new(), "cli_parse\n".to_owned()),
            "`connectors {}` stays a parse refusal",
            args.join(" "),
        );
    }
}

fn script() -> String {
    let (code, stdout, stderr) = parts(&cli(&["completions", "bash"]));
    assert_eq!((code, stderr.as_str()), (Some(0), ""), "completions bash");
    stdout
}

/// Sources the emitted script in bash, finds the function `complete -p` registers
/// for `connectors`, and drives it with `words` (the last one is being completed).
fn complete(words: &[&str]) -> BTreeSet<String> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("connectors.bash");
    std::fs::write(&path, script()).unwrap();
    let driver = r#"
set -u
source "$1"; shift
spec=$(complete -p connectors) || exit 3
func=${spec#*-F }; func=${func%% *}
COMP_WORDS=("$@")
COMP_CWORD=$(( ${#COMP_WORDS[@]} - 1 ))
COMP_LINE="${COMP_WORDS[*]}"
COMP_POINT=${#COMP_LINE}
prev=""
if (( COMP_CWORD > 0 )); then prev=${COMP_WORDS[COMP_CWORD-1]}; fi
COMPREPLY=()
"$func" connectors "${COMP_WORDS[COMP_CWORD]}" "$prev"
printf '%s\n' "${COMPREPLY[@]}"
"#;
    let output = Command::new("bash")
        .arg("--norc")
        .arg("--noprofile")
        .arg("-c")
        .arg(driver)
        .arg("driver")
        .arg(&path)
        .args(words)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "completion driver failed for {words:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn set(words: &[&str]) -> BTreeSet<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

#[test]
fn completion_offers_service_commands_at_the_root() {
    let root = complete(&["connectors", ""]);
    for word in SERVICE_COMMANDS
        .iter()
        .chain(&["adapters", "operations", "help"])
    {
        assert!(
            root.contains(*word),
            "root completion offers {word}: {root:?}"
        );
    }
    let after_output = complete(&["connectors", "--output", "json", ""]);
    for word in SERVICE_COMMANDS {
        assert!(
            after_output.contains(word),
            "completion after --output offers {word}: {after_output:?}"
        );
    }
    let help = complete(&["connectors", "help", ""]);
    for word in SERVICE_COMMANDS.iter().chain(&["adapters", "operations"]) {
        assert!(
            help.contains(*word),
            "help completion offers {word}: {help:?}"
        );
    }
    // `help COMMAND` after a global is a parse refusal, so it is not offered there.
    let help_after_output = complete(&["connectors", "--output", "json", "help", ""]);
    for word in SERVICE_COMMANDS {
        assert!(
            !help_after_output.contains(word),
            "help after --output does not offer {word}: {help_after_output:?}"
        );
    }
}

#[test]
fn completion_offers_exactly_the_service_command_flags() {
    let cases: [(&[&str], &[&str]); 4] = [
        (
            &["connectors", "describe", "--"],
            &[
                "--endpoint",
                "--token-file",
                "--allow-plaintext",
                "--output",
                "--help",
            ],
        ),
        (
            &["connectors", "invoke", "--"],
            &[
                "--endpoint",
                "--token-file",
                "--allow-plaintext",
                "--operation",
                "--input",
                "--output",
                "--help",
            ],
        ),
        (
            &["connectors", "serve", "--"],
            &["--config", "--output", "--help"],
        ),
        (
            &["connectors", "--output", "json", "describe", "--"],
            &[
                "--endpoint",
                "--token-file",
                "--allow-plaintext",
                "--output",
                "--help",
            ],
        ),
    ];
    for (words, flags) in cases {
        assert_eq!(complete(words), set(flags), "completion of {words:?}");
    }
}

#[test]
fn generated_completion_is_unchanged() {
    let script = script();
    let generated = include_str!("../../connectors-cli-contract/completions/connectors.bash");
    let body = generated
        .find("\nif [[ \"${BASH_VERSINFO[0]}\" -eq 4")
        .map_or(generated, |end| &generated[..end]);
    assert!(
        script.starts_with(body),
        "the emitted script keeps the generated completion function verbatim"
    );
    // A generated `describe` keeps its own flags, and grouped words their completion.
    let operations = complete(&["connectors", "operations", "describe", "--"]);
    assert!(operations.contains("--state-dir"), "{operations:?}");
    assert!(!operations.contains("--endpoint"), "{operations:?}");
    let adapters = complete(&["connectors", "adapters", ""]);
    assert!(
        adapters.contains("describe") && !adapters.contains("invoke"),
        "{adapters:?}"
    );
}
