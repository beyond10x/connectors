//! Adversary cases for story:approval-issue-slack-bot-post (issue #180), driven
//! from the unit's own contract text, `contracts/cli/v1alpha1/semantics.md`
//! "The refused argument":
//!
//! > An `invalid_input` that `approvals policy-set`, `approvals prepare` or
//! > `approvals issue` decides names the argument it refused in
//! > `Failure.field`, as the CLI input model names it: [...] and `adapter` for
//! > the instance selection. [...] a refusal the presentation raises before
//! > the handler runs carries its `ess-cli/1` code and no `field`.
//!
//! and the failure table row: "on an approval command, `field` names the
//! argument".
use connectors_host::local::{
    config::{Config, Paths},
    registry, runtime,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    process::{Command, Output},
};

const NOT_INSTALLED: &str = if cfg!(windows) {
    "C:/not-installed"
} else {
    "/not-installed"
};
const REVISION: &str = "desc-1";
const OPERATION: &str = "item.write";
const CONNECTOR: &str = "fixture";
const INSTANCE: &str = "main";
const ALIAS: &str = "forge";

fn command(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(args)
        .output()
        .unwrap()
}

/// The refusal's failure data, from whichever stream carries the envelope.
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
    let init = command(root.path(), &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    root
}

/// The instance selection is the one argument every approval command reads,
/// and the contract names `adapter` for it. `approvals issue` with half of
/// `--connector`/`--instance`, with `--adapter` beside `--connector`, or with
/// no selector and no default instance answers `invalid_input` from the
/// connectors handler (`select::resolve`, before `approvals::execute`), not
/// from the `ess-cli/1` presentation, and names no argument.
#[test]
fn approvals_issue_refusing_its_instance_selection_names_adapter() {
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
        "--approve-subject",
        "abc",
        "--proof-output",
        "/absent/proof.json",
    ];
    let mut seen = Vec::new();
    let mut expected = Vec::new();
    for selection in [
        &["--connector", "slack"][..],
        &["--adapter", "slack", "--connector", "slack"][..],
        &[][..],
    ] {
        let args = [&["approvals", "issue"][..], selection, &target].concat();
        let data = failure(&command(root.path(), &args));
        seen.push((
            selection.join(" "),
            data["code"].clone(),
            data["field"].clone(),
        ));
        expected.push((
            selection.join(" "),
            json!("invalid_input"),
            json!("adapter"),
        ));
    }
    assert_eq!(seen, expected);
}

/// The CLI half of the rule, held without a Secret Service. The unit's host
/// tests assert `owner::Error::field`; only the ignored Slack journey reads
/// `Failure.field` from the CLI, so dropping the `data["field"]` line in
/// `owner_failure` or the argument in `approvals.rs` stays green on the gate.
/// Each refusal here is decided by the handler before any instance is read.
#[test]
fn approval_commands_name_the_refused_argument_on_the_cli() {
    let root = initialized();
    let target = |connection: &'static str, input: &'static str| {
        [
            "--adapter",
            "absent",
            "--connection",
            connection,
            "--operation",
            "item.write",
            "--schema",
            "s1",
            "--revision",
            "r1",
            "--input-json",
            input,
        ]
    };
    let issue = [
        &["approvals", "issue"][..],
        &target("not an id", "{}"),
        &[
            "--approve-subject",
            "abc",
            "--proof-output",
            "/absent/proof.json",
        ],
    ]
    .concat();
    let prepare = [&["approvals", "prepare"][..], &target("c1", "{")].concat();
    let policy_set = [
        "approvals",
        "policy-set",
        "--adapter",
        "absent",
        "--input-file",
        "relative-policy.json",
    ];
    let mut seen = Vec::new();
    for args in [&issue[..], &prepare[..], &policy_set[..]] {
        let data = failure(&command(root.path(), args));
        seen.push((data["code"].clone(), data["field"].clone()));
    }
    assert_eq!(
        seen,
        [
            (json!("invalid_input"), json!("connection")),
            (json!("invalid_input"), json!("input")),
            (json!("invalid_input"), json!("input_file")),
        ]
    );
}

fn bootstrap() -> runtime::Bootstrap {
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        revision: REVISION.into(),
        operations: vec![connectors_core::Operation {
            id: OPERATION.into(),
            description: "fixture operation".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
        }],
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        bindings: None,
        credential_revision: None,
        executable: None,
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        protocol: "v1alpha1".into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: "https://fixture.invalid".into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![runtime::Profile {
            id: "token".into(),
            revision: "profile-1".into(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            minimum_scopes: BTreeSet::new(),
            evidence_lifetime_ms: 60_000,
            fields: vec![runtime::EntryField {
                name: "token".into(),
                label: "Token".into(),
                max_bytes: 1024,
            }],
            acquisition: None,
        }],
        requirements: vec![runtime::Requirement {
            operation: OPERATION.into(),
            custody: false,
            profile: "token".into(),
            scopes: BTreeSet::new(),
            effect: runtime::Effect::Write,
        }],
    }
}

/// One `connectors-local/5` private/2 instance, `fixture/main`, alias `forge`,
/// whose cached description declares one write operation.
fn configured() -> tempfile::TempDir {
    let root = initialized();
    let config_path = root.path().join("config/config.toml");
    let text = fs::read_to_string(&config_path).unwrap();
    let owner = text
        .lines()
        .find(|line| line.starts_with("owner_uid"))
        .unwrap();
    let table = format!("connectors.{CONNECTOR}.instances.{INSTANCE}");
    fs::write(
        &config_path,
        format!(
            "format = \"connectors-local/5\"\n{owner}\n[connectors.{CONNECTOR}]\nenabled=true\n[{table}]\nalias='{ALIAS}'\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[{table}.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[{table}.permissions]\nprofiles=['token']\noperations=['{OPERATION}']\n",
            "a".repeat(64)
        ),
    )
    .unwrap();
    let paths = Paths {
        config: config_path,
        state: root.path().join("state"),
    };
    let adapter = Config::load(&paths.config).unwrap().adapters[ALIAS].clone();
    runtime::state::State::new(&paths.state)
        .remember(&adapter.selection(), &bootstrap())
        .unwrap();
    root
}

/// `invoke` of a write reconstructs the approval subject through
/// the same `approval_issuance::resolve` the approval commands use, so its
/// `invalid_input` could carry the approval commands' `field` values. Since
/// story:connectors-do-short-invocation every `invalid_input` of `invoke` names
/// the `invoke` argument it refused, never an approval command's. A
/// connection id the handler refuses is the case here.
#[test]
fn a_write_invoke_refusal_carries_no_approval_field() {
    let root = configured();
    let schema = connectors_host::local::owner::schema(&bootstrap(), OPERATION).unwrap();
    let target = format!("{CONNECTOR}/{INSTANCE}");
    let output = command(
        root.path(),
        &[
            "invoke",
            &target,
            OPERATION,
            "--connection",
            "not an id",
            "--schema",
            &schema,
            "--revision",
            REVISION,
            "--input-json",
            "{}",
        ],
    );
    let data = failure(&output);
    assert_eq!(data["code"], "invalid_input", "{data}");
    assert_eq!(
        data["field"], "connection",
        "story:connectors-do-short-invocation: invoke names the argument it refused: {data}"
    );
}
