//! Adversarial edge cases for the removed explicit service commands
//! (story:connectors-do-short-invocation): no route reaches them, and `invoke`
//! is the generated command whatever precedes or follows it.

use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
use std::process::{Command, Output};

const SENTINEL: &str = "adversary-argv-sentinel-91c4";
const JSON_PARSE: &str = "{\"error\":{\"code\":\"cli_parse\",\"data\":{}},\"ok\":false}\n";
const HUMAN_PARSE: &str = "cli_parse\n";
const BLOCK: &str = "Explicit service commands";

fn cli<I: IntoIterator<Item = OsString>>(args: I) -> Output {
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
/// help, and carries no service block there either.
#[test]
fn root_help_after_a_global_output_carries_no_service_block() {
    for args in [
        &["--output", "json", "--help"][..],
        &["--output=human", "--help"][..],
        &["--output", "human", "-h"][..],
    ] {
        let (code, stdout, stderr) = parts(&cli(os(args)));
        assert_eq!((code, stderr.as_str()), (Some(0), ""), "{args:?}");
        assert!(
            !stdout.contains(BLOCK) && stdout.contains("  invoke "),
            "story:connectors-do-short-invocation: `connectors {}` is root help, lists `invoke` and no explicit service commands",
            args.join(" ")
        );
    }
}

/// Everything after `--` is positional; an `--output json` there selects
/// nothing. For `invoke` the words are its target and operation, so whatever
/// refuses them answers in human output.
#[test]
fn output_after_the_separator_does_not_select_json() {
    for command in ["describe", "serve"] {
        let args = [command, "--", "--output", "json"];
        assert_eq!(
            parts(&cli(os(&args))),
            (Some(2), String::new(), HUMAN_PARSE.to_owned()),
            "story:connectors-do-short-invocation: {args:?}"
        );
    }
    for args in [
        &["invoke", "--", "--output", "json"][..],
        &["invoke", "--", "x/", "--output=json"],
    ] {
        let (code, stdout, stderr) = parts(&cli(os(args)));
        assert_eq!(
            (code, stdout.as_str(), stderr.starts_with("failure: {")),
            (Some(2), "", true),
            "story:connectors-do-short-invocation: {args:?} answers in human output: {stderr}"
        );
    }
}

// Unix only: the argument is a non-UTF-8 byte string, and a 100,000-byte
// argument exceeds the 32,767-character Windows command line.
#[cfg(unix)]
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
            long.clone().into(),
        ],
        vec!["invoke".into(), bad.clone(), "item.read".into()],
        vec!["invoke".into(), "--endpoint".into(), bad.clone()],
    ];
    for case in cases {
        assert_eq!(
            parts(&cli(case.clone())),
            (Some(2), String::new(), HUMAN_PARSE.to_owned()),
            "story:connectors-do-short-invocation: {case:?}"
        );
    }
    // A long target parses, and is refused as the target without its echo.
    let (code, stdout, stderr) = parts(&cli(vec![
        OsString::from("invoke"),
        long.into(),
        "item.read".into(),
        "--input-json".into(),
        "{}".into(),
    ]));
    assert_eq!(
        (code, stdout.as_str()),
        (Some(2), ""),
        "story:connectors-do-short-invocation"
    );
    assert!(
        stderr.contains("\"field\":\"target\"") && !stderr.contains(SENTINEL),
        "story:connectors-do-short-invocation: {stderr}"
    );
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
            "story:connectors-do-short-invocation: {args:?}"
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
            "story:connectors-do-short-invocation: {args:?}"
        );
        assert!(!stderr.contains(SENTINEL));
    }
}
