//! story:connectors-do-short-invocation, adversary: the edges of `connectors
//! invoke <connector>[/<instance>] <operation>` the main suite
//! (`invoke_short.rs`) does not reach. `default_instance` never stands in for
//! an instance the target leaves out; the ambiguity list names derived
//! connectors, never `catalog`, in order; `--describe` reads nothing; no
//! answer in either output mode says `adapter`; a refused target is not
//! echoed; an operation after `--` is data, not a flag.
use connectors_host::local::{
    config::{Config, Paths},
    registry, runtime,
};
use serde_json::{Value, json};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

const NOT_INSTALLED: &str = if cfg!(windows) {
    "C:/not-installed"
} else {
    "/not-installed"
};
const READ: &str = "item.read";
const WRITE: &str = "item.write";

fn bootstrap(id: &str) -> runtime::Bootstrap {
    let operation = |op: &str| connectors_core::Operation {
        id: op.into(),
        description: "fixture operation".into(),
        contract: "operations/v1alpha1".into(),
        profile: "resource".into(),
        input_schema: json!({"type":"object"}),
        output_schema: json!({"type":"object"}),
    };
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: id.into(),
        adapter: "catalog".into(),
        revision: "desc-1".into(),
        operations: vec![operation(READ), operation(WRITE)],
        configuration_schema: json!({"type":"object"}),
    };
    let requirement = |op: &str, effect| runtime::Requirement {
        operation: op.into(),
        custody: false,
        profile: "token".into(),
        scopes: BTreeSet::new(),
        effect,
    };
    runtime::Bootstrap {
        bindings: None,
        credential_revision: None,
        executable: None,
        instance: id.into(),
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
        requirements: vec![
            requirement(READ, runtime::Effect::Read),
            requirement(WRITE, runtime::Effect::Write),
        ],
    }
}

fn command(root: &Path, output: &str, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
    command
        .args(["--output", output, "--config"])
        .arg(root.join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(args);
    command
}

fn run(root: &Path, output: &str, args: &[&str]) -> Output {
    command(root, output, args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// One pinned instance table: `(connector, instance, alias, instance id)`.
fn instance(connector: &str, name: &str, alias: &str, id: &str) -> String {
    let table = format!("connectors.{connector}.instances.{name}");
    format!(
        "[{table}]\nalias='{alias}'\ninstance_id='{id}'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[{table}.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[{table}.permissions]\nprofiles=['token']\noperations=['{READ}','{WRITE}']\n",
        "a".repeat(64)
    )
}

/// An initialised root whose configuration is `body` under the initialised
/// owner, with every `(alias, instance id)` of `cached` given its description.
fn configured(format: &str, body: &str, cached: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = run(root.path(), "json", &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config = root.path().join("config/config.toml");
    let initialized = fs::read_to_string(&config).unwrap();
    let owner = initialized
        .lines()
        .find(|line| line.starts_with("owner_uid"))
        .unwrap();
    fs::write(&config, format!("format = \"{format}\"\n{owner}\n{body}")).unwrap();
    #[cfg(unix)]
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    let paths = Paths {
        config,
        state: root.path().join("state"),
    };
    let loaded = Config::load(&paths.config).unwrap();
    for (alias, id) in cached {
        runtime::state::State::new(&paths.state)
            .remember(&loaded.adapters[*alias].selection(), &bootstrap(id))
            .unwrap();
    }
    root
}

/// Two instances of `fixture`, `main` and `backup`, both described.
fn two_instances(extra: &str) -> tempfile::TempDir {
    configured(
        "connectors-local/5",
        &format!(
            "{extra}[connectors.fixture]\nenabled=true\n{}{}",
            instance("fixture", "main", "forge", "forge-local"),
            instance("fixture", "backup", "forge-backup", "forge-backup"),
        ),
        &[("forge", "forge-local"), ("forge-backup", "forge-backup")],
    )
}

#[track_caller]
fn data(output: &Output) -> (Option<i32>, Value) {
    assert!(!output.status.success(), "{output:?}");
    let envelope: Value = serde_json::from_slice(&output.stderr)
        .unwrap_or_else(|_| panic!("one JSON envelope: {output:?}"));
    (output.status.code(), envelope["error"]["data"].clone())
}

/// story:connectors-do-short-invocation: `default_instance` is not consulted.
/// A connector with two instances and a default naming one of them is still
/// `instance_ambiguous`, and an explicit instance is served even when it is
/// not the default.
#[test]
fn default_instance_never_stands_in_for_the_left_out_instance() {
    let root = two_instances("default_instance = \"fixture/main\"\n");
    let (code, refused) = data(&run(
        root.path(),
        "json",
        &["invoke", "fixture", READ, "--input-json", "{}"],
    ));
    assert_eq!(
        (code, &refused["code"], &refused["instances"]),
        (
            Some(2),
            &json!("instance_ambiguous"),
            &json!(["fixture/backup", "fixture/main"])
        ),
        "story:connectors-do-short-invocation: the default instance was consulted: {refused}"
    );
    let described = run(
        root.path(),
        "json",
        &["invoke", "fixture/backup", READ, "--describe"],
    );
    assert!(described.status.success(), "{described:?}");
    let result: Value = serde_json::from_slice(&described.stdout).unwrap();
    assert_eq!(
        result["result"]["instance"], "backup",
        "story:connectors-do-short-invocation: {result}"
    );
}

/// story:connectors-do-short-invocation: a legacy catalog entry joins the
/// instances of the connector `setup migrate` derives for it. The ambiguity
/// list names it under that connector, sorted, and never under `catalog`.
#[test]
fn the_ambiguity_list_names_the_derived_connector_in_order() {
    let legacy = format!(
        "[adapters.old]\ninstance_id='jira-old'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.old.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=['--connector','jira']\n[adapters.old.permissions]\nprofiles=['token']\noperations=['{READ}']\n",
        "a".repeat(64)
    );
    let root = configured(
        "connectors-local/5",
        &format!(
            "{legacy}[connectors.jira]\nenabled=true\n{}",
            instance("jira", "main", "jira-main", "jira-main")
        ),
        &[],
    );
    for output in ["json", "human"] {
        let ran = run(
            root.path(),
            output,
            &["invoke", "jira", READ, "--input-json", "{}"],
        );
        assert_eq!(ran.status.code(), Some(2), "{ran:?}");
        let text = String::from_utf8_lossy(&ran.stderr);
        assert!(
            text.contains("instance_ambiguous") && !text.contains("catalog"),
            "story:connectors-do-short-invocation: {output}: {text}"
        );
        let main = text.find("jira/main");
        let old = text.find("jira/old");
        assert!(
            main.is_some() && old.is_some() && main < old,
            "story:connectors-do-short-invocation: {output}: both instances, sorted: {text}"
        );
    }
    let (code, refused) = data(&run(
        root.path(),
        "json",
        &["invoke", "catalog", READ, "--input-json", "{}"],
    ));
    assert_eq!(
        (code, &refused["code"]),
        (Some(1), &json!("not_found")),
        "story:connectors-do-short-invocation: `catalog` names no connector: {refused}"
    );
}

/// story:connectors-do-short-invocation: `--describe` answers without reading
/// standard input, even one left open and never written to.
#[test]
fn describe_never_reads_an_open_stdin() {
    let root = two_instances("");
    let mut child = command(
        root.path(),
        "json",
        &["invoke", "fixture/main", WRITE, "--describe"],
    )
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    let held = child.stdin.take();
    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            panic!("story:connectors-do-short-invocation: --describe waited on stdin");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(held);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        result["result"]["effect"], "write",
        "story:connectors-do-short-invocation: {result}"
    );
}

/// story:connectors-do-short-invocation: no answer `invoke` gives, in either
/// output mode, success or refusal, says `adapter`.
#[test]
fn no_answer_says_adapter_in_either_output_mode() {
    let root = two_instances("");
    let cases: [&[&str]; 7] = [
        &["invoke", "fixture", READ, "--input-json", "{}"],
        &["invoke", "nosuch", READ, "--input-json", "{}"],
        &["invoke", "fixture/main", READ, "--input-json", "{}"],
        &[
            "invoke",
            "fixture/main",
            WRITE,
            "--connection",
            "c1",
            "--input-json",
            "{}",
        ],
        &["invoke", "fixture/main", READ, "--describe"],
        &["invoke", "fixture/main", "nosuch.op", "--describe"],
        &["invoke", "fixture/main/x", READ, "--input-json", "{}"],
    ];
    for args in cases {
        for output in ["json", "human"] {
            let ran = run(root.path(), output, args);
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&ran.stdout),
                String::from_utf8_lossy(&ran.stderr)
            );
            assert!(
                !text.is_empty() && !text.to_lowercase().contains("adapter"),
                "story:connectors-do-short-invocation: {output} {args:?}: {text}"
            );
        }
    }
}

/// story:connectors-do-short-invocation: a refused target is named by
/// `field`, never echoed: a value a consumer passed through from elsewhere
/// does not reach the answer.
#[test]
fn a_refused_target_is_not_echoed() {
    const SENTINEL: &str = "sentinel-7f3a91";
    let root = two_instances("");
    for target in [
        format!("{SENTINEL}/main"),
        format!("fixture/{SENTINEL}"),
        format!("fixture/{SENTINEL}/x"),
        format!("{SENTINEL} x"),
    ] {
        for output in ["json", "human"] {
            let ran = run(
                root.path(),
                output,
                &["invoke", &target, READ, "--input-json", "{}"],
            );
            assert!(!ran.status.success(), "{ran:?}");
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&ran.stdout),
                String::from_utf8_lossy(&ran.stderr)
            );
            assert!(
                !text.contains(SENTINEL),
                "story:connectors-do-short-invocation: {output} {target}: {text}"
            );
        }
    }
}

/// story:connectors-do-short-invocation: after `--` an operation spelled like
/// a flag is the operation, looked up and not found, never taken as the input
/// flag it resembles.
#[test]
fn an_operation_after_the_separator_is_data() {
    let root = two_instances("");
    let (code, refused) = data(&run(
        root.path(),
        "json",
        &[
            "invoke",
            "--connection",
            "c1",
            "fixture/main",
            "--",
            "--input-json",
        ],
    ));
    assert_eq!(
        (code, &refused["code"]),
        (Some(1), &json!("not_found")),
        "story:connectors-do-short-invocation: {refused}"
    );
}
