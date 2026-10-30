//! story:connectors-do-short-invocation: the explicit service commands
//! `describe` and `serve` are removed and `invoke` is the generated command.
//! No route the root help advertises reaches a removed command: `help
//! COMMAND` and `COMMAND --help` are parse refusals for it, and the Bash
//! completion is the generated script, completing `invoke` with its own flags
//! and offering no removed word.

use std::collections::BTreeSet;
use std::process::{Command, Output};

const REMOVED_COMMANDS: [&str; 2] = ["describe", "serve"];
/// The explicit `invoke`'s own flags, none of which the generated one takes.
const ENDPOINT_FLAGS: [&str; 5] = [
    "--endpoint",
    "--token-file",
    "--allow-plaintext",
    "--operation",
    "--input",
];

fn cli(args: &[&str]) -> Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(args)
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_STATE_HOME")
        .stdin(std::process::Stdio::null())
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
fn help_reaches_invoke_and_no_removed_command() {
    for command in REMOVED_COMMANDS {
        for args in [&[command, "--help"][..], &["help", command]] {
            assert_eq!(
                parts(&cli(args)),
                (Some(2), String::new(), "cli_parse\n".to_owned()),
                "story:connectors-do-short-invocation: `connectors {}` is a parse refusal",
                args.join(" "),
            );
        }
    }
    let expected = parts(&cli(&["invoke", "--help"]));
    assert_eq!(expected.0, Some(0), "`invoke --help` exits 0");
    assert!(
        expected.1.contains("Usage:")
            && expected.1.contains("CONNECTOR[/INSTANCE]")
            && ENDPOINT_FLAGS[..3]
                .iter()
                .all(|flag| !expected.1.contains(flag)),
        "story:connectors-do-short-invocation: `invoke --help` is the generated command's: {:?}",
        expected.1
    );
    assert_eq!(
        parts(&cli(&["help", "invoke"])),
        expected,
        "story:connectors-do-short-invocation: `connectors help invoke` prints what `connectors invoke --help` prints",
    );
}

#[test]
fn help_route_does_not_widen_the_parser() {
    for args in [
        &["help", "describe", "extra"][..],
        &["help", "serve", "--config"][..],
        &["--config", "x", "help", "serve"][..],
        &["help", "invoke", "extra"][..],
    ] {
        assert_eq!(
            parts(&cli(args)),
            (Some(2), String::new(), "cli_parse\n".to_owned()),
            "story:connectors-do-short-invocation: `connectors {}` stays a parse refusal",
            args.join(" "),
        );
    }
}

fn script() -> String {
    let (code, stdout, stderr) = parts(&cli(&["completions", "bash"]));
    assert_eq!((code, stderr.as_str()), (Some(0), ""), "completions bash");
    stdout
}

/// The Bash that runs the completion. On Windows `Command::new("bash")` searches
/// the system directory before `PATH` and finds `System32\bash.exe`, the WSL
/// launcher, so the first `bash.exe` on `PATH` outside the Windows directory
/// (Git Bash on the runner) is taken instead.
fn bash() -> std::path::PathBuf {
    if !cfg!(windows) {
        return "bash".into();
    }
    let windows = std::env::var_os("SystemRoot")
        .map(|root| root.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| r"c:\windows".to_owned());
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .filter(|dir| !dir.to_string_lossy().to_lowercase().starts_with(&windows))
        .map(|dir| dir.join("bash.exe"))
        .find(|candidate| candidate.is_file())
        .expect("a bash.exe on PATH outside the Windows directory")
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
    // Git Bash reads a drive path written with forward slashes.
    let script_path = if cfg!(windows) {
        path.to_string_lossy().replace('\\', "/")
    } else {
        path.to_string_lossy().into_owned()
    };
    let output = Command::new(bash())
        .arg("--norc")
        .arg("--noprofile")
        .arg("-c")
        .arg(driver)
        .arg("driver")
        .arg(script_path)
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

#[test]
fn completion_offers_invoke_and_no_removed_command() {
    let root = complete(&["connectors", ""]);
    for word in ["invoke", "instances", "adapters", "operations", "help"] {
        assert!(
            root.contains(word),
            "root completion offers {word}: {root:?}"
        );
    }
    let after_output = complete(&["connectors", "--output", "json", ""]);
    let help = complete(&["connectors", "help", ""]);
    for (offered, name) in [
        (&root, "root"),
        (&after_output, "after --output"),
        (&help, "help"),
    ] {
        assert!(
            offered.contains("invoke"),
            "story:connectors-do-short-invocation: {name} offers invoke: {offered:?}"
        );
        for word in REMOVED_COMMANDS {
            assert!(
                !offered.contains(word),
                "story:connectors-do-short-invocation: {name} does not offer {word}: {offered:?}"
            );
        }
    }
}

#[test]
fn completion_of_invoke_offers_its_own_flags_only() {
    let invoke = connectors_cli_contract::command()
        .find_subcommand("invoke")
        .cloned()
        .expect("the generated tree has invoke");
    let own: BTreeSet<String> = invoke
        .get_arguments()
        .filter_map(|arg| arg.get_long().map(|long| format!("--{long}")))
        .collect();
    for words in [
        &["connectors", "invoke", "--"][..],
        &["connectors", "--output", "json", "invoke", "--"],
    ] {
        let offered = complete(words);
        for flag in &own {
            assert!(
                offered.contains(flag),
                "story:connectors-do-short-invocation: completion of {words:?} offers {flag}: {offered:?}"
            );
        }
        for flag in ENDPOINT_FLAGS {
            assert!(
                !offered.contains(flag),
                "story:connectors-do-short-invocation: completion of {words:?} offers no {flag}: {offered:?}"
            );
        }
    }
    for command in REMOVED_COMMANDS {
        let offered = complete(&["connectors", command, "--"]);
        assert!(
            ["--endpoint", "--token-file", "--allow-plaintext"]
                .iter()
                .all(|flag| !offered.contains(*flag)),
            "story:connectors-do-short-invocation: `{command}` completes no service flag: {offered:?}"
        );
    }
}

#[test]
fn the_completion_script_is_the_generated_one() {
    let script = script();
    let generated = include_str!("../../connectors-cli-contract/completions/connectors.bash");
    assert_eq!(
        script, generated,
        "story:connectors-do-short-invocation: the emitted script is the generated completion, with nothing appended"
    );
    // A generated `describe` keeps its own flags, and grouped words their completion.
    let operations = complete(&["connectors", "operations", "describe", "--"]);
    assert!(operations.contains("--state-dir"), "{operations:?}");
    assert!(!operations.contains("--endpoint"), "{operations:?}");
    let instances = complete(&["connectors", "instances", ""]);
    assert!(
        instances.contains("describe") && !instances.contains("invoke"),
        "{instances:?}"
    );
}
