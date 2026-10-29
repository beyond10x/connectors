//! story:connectors-do-short-invocation: the explicit service commands
//! `describe`, `invoke --endpoint` and `serve` are removed. Each of their
//! command lines is the grouped parser's fixed `cli_parse` envelope, never an
//! argv echo, in either output mode and with `--output` before or after the
//! command word; `invoke` is the generated command.

use std::process::{Command, Output};

const SENTINEL: &str = "fictional-argv-sentinel-7f3a";
// Key order is the serializer's; byte equality with the grouped parser is asserted below.
const JSON_PARSE: &str = "{\"error\":{\"code\":\"cli_parse\",\"data\":{}},\"ok\":false}\n";
const HUMAN_PARSE: &str = "cli_parse\n";

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

fn malformed() -> Vec<Vec<String>> {
    let flag = format!("--{SENTINEL}");
    let equals = format!("--endpoint={SENTINEL}");
    let mut cases = Vec::new();
    for command in ["describe", "invoke", "serve"] {
        cases.push(vec![command.to_owned(), SENTINEL.to_owned()]);
        cases.push(vec![command.to_owned(), flag.clone()]);
        cases.push(vec![command.to_owned(), flag.clone(), SENTINEL.to_owned()]);
        cases.push(vec![
            command.to_owned(),
            equals.clone(),
            SENTINEL.to_owned(),
        ]);
    }
    cases.push(vec!["describe".to_owned()]);
    cases.push(vec!["serve".to_owned(), "--config".to_owned()]);
    cases
}

#[test]
fn removed_command_lines_are_the_human_parse_envelope_without_argv() {
    for case in malformed() {
        let args: Vec<&str> = case.iter().map(String::as_str).collect();
        assert_eq!(
            parts(&cli(&args)),
            (Some(2), String::new(), HUMAN_PARSE.to_owned()),
            "story:connectors-do-short-invocation: `connectors {}` must refuse with the fixed human parse envelope",
            case.join(" "),
        );
    }
}

#[test]
fn removed_command_lines_are_the_json_envelope_before_or_after_the_command_word() {
    for case in malformed() {
        let (command, rest) = case.split_first().unwrap();
        let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
        let selections: [Vec<&str>; 3] = [
            [&["--output", "json", command.as_str()][..], &rest].concat(),
            [&["--output=json", command.as_str()][..], &rest].concat(),
            [&[command.as_str(), "--output", "json"][..], &rest].concat(),
        ];
        for args in selections {
            assert_eq!(
                parts(&cli(&args)),
                (Some(2), String::new(), JSON_PARSE.to_owned()),
                "story:connectors-do-short-invocation: `connectors {}` must refuse with the fixed JSON parse envelope",
                args.join(" "),
            );
        }
    }
}

#[test]
fn a_removed_word_is_answered_as_any_unknown_word() {
    for output in ["human", "json"] {
        let grouped = cli(&["--output", output, SENTINEL]);
        for removed in ["describe", "serve"] {
            let legacy = cli(&["--output", output, removed, SENTINEL]);
            assert_eq!(
                parts(&legacy),
                parts(&grouped),
                "story:connectors-do-short-invocation: `{removed}` is an unknown word, output {output}"
            );
        }
    }
}

/// story:connectors-do-short-invocation: what the removed routes ran is a
/// parse refusal now, with `--output` before the command word too; their
/// `--help` is one as well.
#[test]
fn the_removed_routes_are_parse_refusals_with_output_before_the_word() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing");
    let missing = missing.to_str().unwrap();
    for (output, expected) in [
        (["--output", "json"], JSON_PARSE),
        (["--output", "human"], HUMAN_PARSE),
    ] {
        let served = cli(&[&output[..], &["serve", "--config", missing]].concat());
        let described = cli(&[
            &output[..],
            &[
                "describe",
                "--endpoint",
                "http://127.0.0.1:9/",
                "--token-file",
                missing,
            ],
        ]
        .concat());
        let invoked = cli(&[
            &output[..],
            &[
                "invoke",
                "--endpoint",
                "http://127.0.0.1:9/",
                "--token-file",
                missing,
                "--operation",
                "read",
                "--input",
                missing,
            ],
        ]
        .concat());
        for refused in [served, described, invoked] {
            assert_eq!(
                parts(&refused),
                (Some(2), String::new(), expected.to_owned()),
                "story:connectors-do-short-invocation: {output:?}"
            );
        }
        for command in ["describe", "serve"] {
            assert_eq!(
                parts(&cli(&[output[0], output[1], command, "--help"])),
                (Some(2), String::new(), expected.to_owned()),
                "story:connectors-do-short-invocation: `{command} --help`"
            );
        }
        let help = cli(&[output[0], output[1], "invoke", "--help"]);
        let (code, stdout, stderr) = parts(&help);
        assert_eq!((code, stderr.as_str()), (Some(0), ""));
        assert!(
            stdout.contains("Usage:") && !stdout.contains("--endpoint"),
            "story:connectors-do-short-invocation: {stdout}"
        );
    }
    assert_eq!(
        parts(&cli(&["--output=json", "describe", "--help"])),
        (Some(2), String::new(), JSON_PARSE.to_owned()),
        "story:connectors-do-short-invocation"
    );
}

#[test]
fn root_help_is_the_generated_tree_and_names_the_native_server() {
    // help.txt is the generated tree's long help; root `--help` is the short form of that tree
    // as `connectors::help` presents it.
    assert_eq!(
        connectors_cli_contract::command()
            .render_long_help()
            .to_string(),
        include_str!("../../connectors-cli-contract/help.txt")
    );
    let short = connectors::help::command().render_help().to_string();
    let native = "Native integration commands (use COMMAND --help for options):\n  server    Serve admitted read-only MCP tools over stdio\n";
    assert_eq!(
        parts(&cli(&["--help"])),
        (Some(0), format!("{short}\n{native}"), String::new()),
        "story:connectors-do-short-invocation: root help is the generated tree and the native server"
    );
    assert!(
        !short.contains("Explicit service commands"),
        "story:connectors-do-short-invocation"
    );
    let semantics = include_str!("../../../contracts/cli/v1alpha1/semantics.md");
    assert!(
        !semantics.contains("Explicit service commands (use COMMAND --help for options)"),
        "story:connectors-do-short-invocation: semantics.md states no explicit service commands block"
    );
}
