//! Adversary cases for story:cli-surface-minor-findings-0-18-0. Each drives the
//! binary from a sentence the unit added to `contracts/cli/v1alpha1/semantics.md`
//! or `scenarios.md` C05.
use connectors_host::local::{
    config::{Config, Paths},
    registry, runtime,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    process::{Command, Output, Stdio},
};

const REVISION: &str = "desc-1";
const OPERATION: &str = "item.read";

fn base(root: &tempfile::TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
    command
        .args(["--output", "json", "--config"])
        .arg(root.path().join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.path().join("state"));
    command
}

fn command(root: &tempfile::TempDir, args: &[&str]) -> Output {
    base(root).args(args).stdin(Stdio::null()).output().unwrap()
}

fn with_stdin(root: &tempfile::TempDir, args: &[&str], stdin: &[u8]) -> Output {
    let mut child = base(root)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // A refusal may close stdin early; a broken pipe is not the subject here.
    let _ = child.stdin.take().unwrap().write_all(stdin);
    child.wait_with_output().unwrap()
}

/// `exit code/stdout-empty/<ess-cli code>` or `.../failure/<Failure.code>`,
/// with ess-cli data checked to be empty.
fn answer(output: &Output) -> String {
    let exit = output.status.code().unwrap();
    let envelope: Value = serde_json::from_slice(&output.stderr).unwrap_or_default();
    let error = &envelope["error"];
    let code = if error["code"] == "failure" {
        format!("failure/{}", error["data"]["code"].as_str().unwrap_or("?"))
    } else if error["data"] == json!({}) {
        error["code"].as_str().unwrap_or("?").to_owned()
    } else {
        format!("{}+data:{}", error["code"], error["data"])
    };
    format!("{exit}/{}/{code}", output.stdout.is_empty())
}

fn initialized() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = command(&root, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    root
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
            input_schema: json!({"type":"object","additionalProperties":false,
                "properties":{"name":{}}}),
            output_schema: json!({"type":"object","additionalProperties":false}),
        }],
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
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
            profile: "token".into(),
            scopes: BTreeSet::new(),
            effect: runtime::Effect::Read,
        }],
    }
}

/// One configured adapter with a cached, granted description whose `name`
/// field accepts any JSON value, so a deeply nested document is refused only
/// by the depth bound. No adapter executable exists and none is launched.
fn configured() -> (tempfile::TempDir, String) {
    let root = initialized();
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.forge.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[adapters.forge.permissions]\nprofiles=['token']\noperations=['{OPERATION}']\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
    let paths = Paths {
        config: config_path.clone(),
        state: root.path().join("state"),
    };
    let adapter = Config::load(&paths.config).unwrap().adapters["forge"].clone();
    let bootstrap = bootstrap();
    runtime::state::State::new(&paths.state)
        .remember(&adapter.selection(), &bootstrap)
        .unwrap();
    let schema = connectors_host::local::owner::schema(&bootstrap, OPERATION).unwrap();
    (root, schema)
}

fn selected<'a>(verb: &'a [&'a str], schema: &'a str) -> Vec<&'a str> {
    [
        verb,
        &[
            "--adapter",
            "forge",
            "--connection",
            "conn-1",
            "--operation",
            OPERATION,
            "--schema",
            schema,
            "--revision",
            REVISION,
        ][..],
    ]
    .concat()
}

fn nested(depth: usize) -> String {
    format!("{{\"name\":{}0{}}}", "[".repeat(depth), "]".repeat(depth))
}

#[test]
fn c05_wrong_input_is_classified_the_same_from_inline_and_stdin() {
    let (root, schema) = configured();
    let cases = [
        (
            "malformed",
            "{\"name\":".to_owned(),
            "2/true/cli_dynamic_input",
        ),
        ("trailing", "{} {}".to_owned(), "2/true/cli_dynamic_input"),
        // Since ESS 0.52 the generated decoder refuses a duplicate object key itself.
        (
            "duplicate",
            r#"{"name":"a","name":"b"}"#.to_owned(),
            "2/true/cli_dynamic_input",
        ),
        (
            "wrong field",
            r#"{"other":"a"}"#.to_owned(),
            "2/true/failure/invalid_input",
        ),
        ("depth", nested(70), "2/true/failure/invalid_input"),
    ];
    let mut wrong = Vec::new();
    for (case, document, expected) in &cases {
        let mut inline = selected(&["operations", "invoke"], &schema);
        inline.extend(["--input-json", document.as_str()]);
        let got = answer(&command(&root, &inline));
        if got != *expected {
            wrong.push(format!("inline {case}: {got}"));
        }
        let mut stdin = selected(&["operations", "invoke"], &schema);
        stdin.push("--input-stdin");
        let got = answer(&with_stdin(&root, &stdin, document.as_bytes()));
        if got != *expected {
            wrong.push(format!("stdin {case}: {got}"));
        }
    }
    let oversized = format!("{{\"name\":\"{}\"}}", "a".repeat(1024 * 1024 + 1));
    let mut stdin = selected(&["operations", "invoke"], &schema);
    stdin.push("--input-stdin");
    let got = answer(&with_stdin(&root, &stdin, oversized.as_bytes()));
    if got != "2/true/failure/invalid_input" {
        wrong.push(format!("stdin bytes: {got}"));
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// semantics.md error table: "stale, expired or foreign list cursor
/// (stage = observation, next_action = retry_explicitly) | stale_cursor | 1",
/// on every list command that takes a cursor.
#[test]
fn every_list_cursor_this_tree_cannot_honour_is_stale_cursor() {
    let (root, _) = configured();
    let mut wrong = Vec::new();
    for args in [
        &["adapters", "list", "--cursor", "c1"][..],
        &[
            "connections",
            "list",
            "--adapter",
            "forge",
            "--cursor",
            "c1",
        ][..],
        &["operations", "list", "--adapter", "forge", "--cursor", "c1"][..],
    ] {
        let output = command(&root, args);
        let envelope: Value = serde_json::from_slice(&output.stderr).unwrap_or_default();
        let data = &envelope["error"]["data"];
        if output.status.code() != Some(1)
            || data["code"] != "stale_cursor"
            || data["stage"] != "observation"
            || data["next_action"] != "retry_explicitly"
        {
            wrong.push(format!("{}: {envelope}", args[..2].join(" ")));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
