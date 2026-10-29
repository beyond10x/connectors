//! story:connectors-do-short-invocation: `operations invoke` is removed, not
//! deprecated. Its command lines, with `--definition` and `--help` too, are
//! the grouped parser's fixed `cli_parse` envelope, exit 2, and `operations
//! --help` lists no `invoke`; `connectors invoke` is the one way to invoke.

use std::process::{Command, Output, Stdio};

const JSON_PARSE: &str = "{\"error\":{\"code\":\"cli_parse\",\"data\":{}},\"ok\":false}\n";
const HUMAN_PARSE: &str = "cli_parse\n";

fn cli(args: &[&str]) -> Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(args)
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_STATE_HOME")
        .stdin(Stdio::null())
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

/// story:connectors-do-short-invocation: every `operations invoke` command
/// line is a parse refusal in both output modes, whatever it selects with.
#[test]
fn operations_invoke_is_a_parse_refusal() {
    let lines: [&[&str]; 6] = [
        &["operations", "invoke"],
        &[
            "operations",
            "invoke",
            "--adapter",
            "forge",
            "--connection",
            "c1",
            "--operation",
            "item.read",
            "--schema",
            "s1",
            "--revision",
            "r1",
            "--input-json",
            "{}",
        ],
        &[
            "operations",
            "invoke",
            "--connector",
            "fixture",
            "--instance",
            "main",
            "--operation",
            "item.read",
            "--input-json",
            "{}",
        ],
        &[
            "operations",
            "invoke",
            "--definition",
            "/fixture/defs/connectors/forge.yaml",
            "--operation",
            "project.get",
            "--input-json",
            "{}",
        ],
        &["operations", "invoke", "--help"],
        &["operations", "invoke", "-h"],
    ];
    for line in lines {
        for (output, expected) in [
            (&["--output", "human"][..], HUMAN_PARSE),
            (&["--output", "json"][..], JSON_PARSE),
        ] {
            assert_eq!(
                parts(&cli(&[output, line].concat())),
                (Some(2), String::new(), expected.to_owned()),
                "story:connectors-do-short-invocation: `connectors {}` is removed",
                line.join(" ")
            );
        }
    }
}

/// story:connectors-do-short-invocation: `operations --help` lists `list` and
/// `describe` and no `invoke`; root help lists `invoke`.
#[test]
fn operations_help_lists_no_invoke() {
    let (code, stdout, stderr) = parts(&cli(&["operations", "--help"]));
    assert_eq!((code, stderr.as_str()), (Some(0), ""), "{stdout}");
    let commands: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("  ") && !line.trim_start().starts_with('-'))
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    assert!(
        commands.contains(&"list") && commands.contains(&"describe"),
        "story:connectors-do-short-invocation: {stdout}"
    );
    assert!(
        !commands.contains(&"invoke"),
        "story:connectors-do-short-invocation: `operations --help` lists invoke: {stdout}"
    );
    let (code, root, _) = parts(&cli(&["--help"]));
    assert_eq!(code, Some(0));
    assert!(
        root.lines()
            .any(|line| line.split_whitespace().next() == Some("invoke")),
        "story:connectors-do-short-invocation: {root}"
    );
}
