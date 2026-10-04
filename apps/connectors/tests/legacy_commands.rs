//! The explicit service commands `describe`, `invoke` and `serve` parse like the
//! grouped CLI contract: parse errors are the fixed `cli_parse` envelope, never an
//! argv echo, and `--output` is accepted before the command word.

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
fn legacy_parse_errors_are_the_human_envelope_without_argv() {
    for case in malformed() {
        let args: Vec<&str> = case.iter().map(String::as_str).collect();
        assert_eq!(
            parts(&cli(&args)),
            (Some(2), String::new(), HUMAN_PARSE.to_owned()),
            "`connectors {}` must refuse with the fixed human parse envelope",
            case.join(" "),
        );
    }
}

#[test]
fn legacy_parse_errors_are_the_json_envelope_before_or_after_the_command_word() {
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
                "`connectors {}` must refuse with the fixed JSON parse envelope",
                args.join(" "),
            );
        }
    }
}

#[test]
fn legacy_parse_envelope_is_the_grouped_parse_envelope() {
    for output in ["human", "json"] {
        let grouped = cli(&["--output", output, SENTINEL]);
        let legacy = cli(&["--output", output, "describe", SENTINEL]);
        assert_eq!(parts(&legacy), parts(&grouped), "output {output}");
    }
}

#[test]
fn output_before_the_legacy_command_word_is_accepted() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing");
    let missing = missing.to_str().unwrap();
    for output in [["--output", "json"], ["--output", "human"]] {
        // Parsed and routed: the refusal is the command's own, not the parser's.
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
        for routed in [served, described] {
            let (code, stdout, stderr) = parts(&routed);
            assert_eq!((code, stdout.as_str()), (Some(1), ""), "{stderr}");
            assert!(!stderr.contains("cli_parse"), "{stderr}");
        }
        for command in ["describe", "invoke", "serve"] {
            let help = cli(&[output[0], output[1], command, "--help"]);
            let (code, stdout, stderr) = parts(&help);
            assert_eq!((code, stderr.as_str()), (Some(0), ""));
            assert!(stdout.contains("Usage:"), "{stdout}");
        }
    }
    let equals = cli(&["--output=json", "describe", "--help"]);
    assert_eq!(equals.status.code(), Some(0));
}

#[test]
fn root_help_keeps_generated_and_service_help_and_names_the_native_server() {
    let semantics = include_str!("../../../contracts/cli/v1alpha1/semantics.md");
    let block = semantics
        .split("```text\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .filter(|block| block.starts_with("Explicit service commands"))
        .expect("semantics.md states the explicit service commands block");
    // help.txt is the generated tree's long help; root `--help` is its short form.
    let mut generated = connectors_cli_contract::command();
    assert_eq!(
        generated.render_long_help().to_string(),
        include_str!("../../connectors-cli-contract/help.txt")
    );
    let short = generated.render_help().to_string();
    let native = "Native integration commands (use COMMAND --help for options):\n  server    Serve admitted read-only MCP tools over stdio\n";
    assert_eq!(
        parts(&cli(&["--help"])),
        (
            Some(0),
            format!("{short}\n{native}\n{block}"),
            String::new()
        )
    );
    assert!(!short.contains("Explicit service commands"));
}
