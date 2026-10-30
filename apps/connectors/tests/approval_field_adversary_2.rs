//! Second adversary pass on story:approval-issue-slack-bot-post (issue #180),
//! driven from the unit's contract text (`contracts/cli/v1alpha1/semantics.md`,
//! "The refused argument"):
//!
//! > An `invalid_input` that `approvals policy-set`, `approvals prepare` or
//! > `approvals issue` decides names the argument it refused in
//! > `Failure.field`, as the CLI input model names it: [...] `input` (the
//! > business document, from whichever of `--input-json`, `--input-file` or
//! > `--input-stdin`) [...]; a refusal the presentation raises before the
//! > handler runs carries its `ess-cli/1` code and no `field`.
//!
//! The business document of `prepare` and `issue` given by `--input-file` or
//! `--input-stdin` is read by the connectors host (`session::acquire`,
//! `protected::document_until`), and its refusal is rendered by
//! `session::finish` through `owner_failure` as the owner's own
//! `failure`/`invalid_input` (stage `arguments`), not as an `ess-cli/1` code.
//! It is the host's refusal of the `input` argument, so the contract gives it
//! `field: "input"`.
use serde_json::Value;
use std::{
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

/// `approval_issuance::TARGET_LIMIT`, the bound `session::acquire` reads to.
const TARGET_LIMIT: usize = 256 * 1024;

fn command(root: &Path, args: &[&str], stdin: Option<&[u8]>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(bytes) = stdin {
        // The reader may stop at the bound; a broken pipe past it is expected.
        let _ = child.stdin.take().unwrap().write_all(bytes);
    }
    child.wait_with_output().unwrap()
}

#[track_caller]
fn failure(output: &Output) -> Value {
    assert!(!output.status.success(), "{output:?}");
    let stream = if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    let envelope: Value = serde_json::from_slice(stream).unwrap();
    envelope["error"]["data"].clone()
}

fn initialized() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = command(root.path(), &["setup", "init"], None);
    assert!(init.status.success(), "{init:?}");
    root
}

/// `approvals prepare` and `approvals issue` refusing a business document
/// given by `--input-file` (absent, a directory, one byte over the bound) or
/// by `--input-stdin` (one byte over the bound) answer the owner's
/// `invalid_input` at stage `arguments`, and name no argument.
#[test]
fn an_approval_business_document_refused_from_a_file_or_stdin_names_input() {
    let root = initialized();
    let oversized = root.path().join("oversized.json");
    let mut document = vec![b' '; TARGET_LIMIT];
    document.extend_from_slice(b"{}");
    std::fs::write(&oversized, &document).unwrap();
    let directory = root.path().join("a-directory");
    std::fs::create_dir(&directory).unwrap();
    let absent = root.path().join("absent.json");
    let target = [
        "--adapter",
        "absent",
        "--connection",
        "c1",
        "--operation",
        "item.write",
        "--schema",
        "s1",
        "--revision",
        "r1",
    ];
    let issue_only = [
        "--approve-subject",
        "abc",
        "--proof-output",
        "/absent/proof.json",
    ];
    let mut seen = Vec::new();
    let mut expected = Vec::new();
    for (verb, extra) in [("prepare", &[][..]), ("issue", &issue_only[..])] {
        for (label, file) in [
            ("absent", &absent),
            ("directory", &directory),
            ("oversized", &oversized),
        ] {
            let args = [
                &["approvals", verb][..],
                &target,
                &["--input-file", file.to_str().unwrap()],
                extra,
            ]
            .concat();
            let data = failure(&command(root.path(), &args, None));
            seen.push((verb, label, data["code"].clone(), data["field"].clone()));
            expected.push((
                verb,
                label,
                Value::from("invalid_input"),
                Value::from("input"),
            ));
        }
        let args = [&["approvals", verb][..], &target, &["--input-stdin"], extra].concat();
        let data = failure(&command(root.path(), &args, Some(&document)));
        seen.push((
            verb,
            "stdin oversized",
            data["code"].clone(),
            data["field"].clone(),
        ));
        expected.push((
            verb,
            "stdin oversized",
            Value::from("invalid_input"),
            Value::from("input"),
        ));
    }
    assert_eq!(seen, expected);
}

/// The correction's `select::refusal_of` names `adapter` on a refused
/// instance selection of exactly `policy-set`, `prepare` and `issue`, and no
/// other command carries `field`. Only `issue` was asserted, so dropping
/// `approval-policy-set` or `approval-prepare` from its list, or widening it
/// to every callable, stayed green.
#[test]
fn a_refused_instance_selection_names_adapter_on_the_three_approval_commands_only() {
    let root = initialized();
    let target = [
        "--connection",
        "c1",
        "--operation",
        "item.write",
        "--schema",
        "s1",
        "--revision",
        "r1",
        "--input-json",
        "{}",
    ];
    let commands: [(&str, Vec<&str>, Option<&str>); 5] = [
        (
            "policy-set",
            vec![
                "approvals",
                "policy-set",
                "--input-file",
                "/absent/policy.json",
            ],
            Some("adapter"),
        ),
        (
            "prepare",
            [&["approvals", "prepare"][..], &target].concat(),
            Some("adapter"),
        ),
        (
            "issue",
            [
                &["approvals", "issue"][..],
                &target,
                &[
                    "--approve-subject",
                    "abc",
                    "--proof-output",
                    "/absent/p.json",
                ],
            ]
            .concat(),
            Some("adapter"),
        ),
        ("policy-status", vec!["approvals", "policy-status"], None),
        ("describe", vec!["connections", "describe"], None),
    ];
    let mut seen = Vec::new();
    let mut expected = Vec::new();
    for (label, args, field) in commands {
        for selection in [&["--connector", "slack"][..], &["--instance", "main"][..]] {
            let args = [&args[..], selection].concat();
            let data = failure(&command(root.path(), &args, None));
            seen.push((label, data["code"].clone(), data.get("field").cloned()));
            expected.push((label, Value::from("invalid_input"), field.map(Value::from)));
        }
    }
    // `invoke` selects by its target, so the half selections are a target
    // naming only a connector or only an instance, and the field is `target`.
    for selection in ["slack/", "/main"] {
        let args = ["invoke", selection, "item.write", "--input-json", "{}"];
        let data = failure(&command(root.path(), &args, None));
        seen.push(("invoke", data["code"].clone(), data.get("field").cloned()));
        expected.push((
            "invoke",
            Value::from("invalid_input"),
            Some(Value::from("target")),
        ));
    }
    assert_eq!(
        seen, expected,
        "story:connectors-do-short-invocation: `adapter` is named by the three approval commands only; `invoke` names its `target`"
    );
}
