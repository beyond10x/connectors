//! Adversarial edge cases for the explicit service commands' parse routing.

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::process::{Command, Output};

const SENTINEL: &str = "adversary-argv-sentinel-91c4";
const JSON_PARSE: &str = "{\"error\":{\"code\":\"cli_parse\",\"data\":{}},\"ok\":false}\n";
const HUMAN_PARSE: &str = "cli_parse\n";
const BLOCK: &str = "Explicit service commands (use COMMAND --help for options):\n";

fn cli<I: IntoIterator<Item = OsString>>(args: I) -> Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(args)
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_STATE_HOME")
        .output()
        .unwrap()
}

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn parts(output: &Output) -> (Option<i32>, String, String) {
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Root `--help` with the process global `--output` selected is still root
/// help; semantics.md §2 says root help carries the service block.
#[test]
fn root_help_after_a_global_output_carries_the_service_block() {
    for args in [
        &["--output", "json", "--help"][..],
        &["--output=human", "--help"][..],
        &["--output", "human", "-h"][..],
    ] {
        let (code, stdout, stderr) = parts(&cli(os(args)));
        assert_eq!((code, stderr.as_str()), (Some(0), ""), "{args:?}");
        assert!(
            stdout.contains(BLOCK),
            "`connectors {}` is root help and must list the explicit service commands",
            args.join(" ")
        );
    }
}

/// Everything after `--` is positional; an `--output json` there selects nothing.
/// Catches removal of the `--` stop in `parse_refusal`.
#[test]
fn output_after_the_separator_does_not_select_json() {
    for command in ["describe", "invoke", "serve"] {
        let args = [command, "--", "--output", "json"];
        assert_eq!(
            parts(&cli(os(&args))),
            (Some(2), String::new(), HUMAN_PARSE.to_owned()),
            "{args:?}"
        );
    }
}

#[test]
fn non_utf8_and_long_argv_are_the_fixed_envelope() {
    let mut bad = SENTINEL.as_bytes().to_vec();
    bad.push(0xff);
    let bad = OsString::from_vec(bad);
    let long = format!("{}{SENTINEL}", "a".repeat(100_000));
    let cases: Vec<Vec<OsString>> = vec![
        vec!["describe".into(), bad.clone()],
        vec!["describe".into(), "--endpoint".into(), bad.clone()],
        vec!["--output".into(), bad.clone(), "describe".into()],
        vec!["describe".into(), long.clone().into()],
        vec![
            "serve".into(),
            format!("--config={long}").into(),
            long.into(),
        ],
    ];
    for case in cases {
        assert_eq!(
            parts(&cli(case.clone())),
            (Some(2), String::new(), HUMAN_PARSE.to_owned()),
            "{case:?}"
        );
    }
}

#[test]
fn routing_edges_answer_like_the_grouped_parser() {
    for args in [
        &["--output", "json"][..],
        &["--output", "json", "--output", "json", "describe", SENTINEL][..],
        &["--output", "human", "--output", "json", "serve", "--config"][..],
        &["--output=json", "help", "describe"][..],
        &["--output", "json", "--", "describe"][..],
        &["describe", "--output", "json", "--output", "json"][..],
    ] {
        assert_eq!(
            parts(&cli(os(args))),
            (Some(2), String::new(), JSON_PARSE.to_owned()),
            "{args:?}"
        );
    }
    for args in [
        &["--output", SENTINEL, "describe"][..],
        &[&format!("--output={SENTINEL}"), "invoke"][..],
        &[
            "describe",
            &format!("--endpoint={SENTINEL}"),
            &format!("--{SENTINEL}"),
        ][..],
        &["descrbe", SENTINEL][..],
    ] {
        let (code, stdout, stderr) = parts(&cli(os(args)));
        assert_eq!(
            (code, stdout.as_str(), stderr.as_str()),
            (Some(2), "", HUMAN_PARSE),
            "{args:?}"
        );
        assert!(!stderr.contains(SENTINEL));
    }
}
