//! Adversary pass on story:legacy-alias-sharing-an-instance-name (commit b90f3b349):
//! the selection paths the unit did not test, and a configuration with no shared
//! name, against contracts/cli/v1alpha1/semantics.md "Invoking an operation".
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
};

const NOT_INSTALLED: &str = if cfg!(windows) {
    "C:/not-installed"
} else {
    "/not-installed"
};
const READ: &str = "item.read";
const WRITE: &str = "item.write";
/// An operation whose description classifies its effect as neither.
const PROBE: &str = "item.probe";

fn input_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"name":{"type":"string"}}})
}

/// The cached description of instance `id`: a read and a write, and with
/// `probe` an operation of unknown effect.
fn described(id: &str, probe: bool) -> runtime::Bootstrap {
    let operation = |op: &str| connectors_core::Operation {
        id: op.into(),
        description: format!("fixture {op}"),
        contract: "operations/v1alpha1".into(),
        profile: "resource".into(),
        input_schema: input_schema(),
        output_schema: json!({"type":"object","additionalProperties":false}),
    };
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: id.into(),
        adapter: "catalog".into(),
        revision: "desc-1".into(),
        operations: [READ, WRITE, PROBE][..if probe { 3 } else { 2 }]
            .iter()
            .map(|op| operation(op))
            .collect(),
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
            requirement(PROBE, runtime::Effect::Unknown),
        ][..if probe { 3 } else { 2 }]
            .to_vec(),
    }
}

/// The pinned entry body of one instance or legacy entry under `table`.
fn entry(table: &str, id: &str, args: &str) -> String {
    format!(
        "instance_id='{id}'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[{table}.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs={args}\n[{table}.permissions]\nprofiles=['token']\noperations=['{READ}','{WRITE}','{PROBE}']\n",
        "a".repeat(64)
    )
}

/// `[connectors.<connector>.instances.<name>]` with alias `alias`.
fn instance(connector: &str, name: &str, alias: &str, id: &str) -> String {
    let table = format!("connectors.{connector}.instances.{name}");
    format!("[{table}]\nalias='{alias}'\n{}", entry(&table, id, "[]"))
}

/// A legacy `[adapters.<alias>]` catalog entry `setup migrate` derives
/// `connector` for.
fn legacy(alias: &str, connector: &str, id: &str) -> String {
    let table = format!("adapters.{alias}");
    format!(
        "[{table}]\n{}",
        entry(&table, id, &format!("['--connector','{connector}']"))
    )
}

fn command(root: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
    command
        .args(["--output", "json", "--config"])
        .arg(root.join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(args);
    command
}

fn run(root: &Path, args: &[&str]) -> Output {
    command(root, args).stdin(Stdio::null()).output().unwrap()
}

/// An initialised root whose configuration is `body`, with every `(alias,
/// instance id)` of `cached` given its description.
fn configured(body: &str, cached: &[(&str, &str)]) -> tempfile::TempDir {
    configured_with(body, cached, false)
}

/// [`configured`], each description holding the unknown-effect operation when
/// `probe`.
fn configured_with(body: &str, cached: &[(&str, &str)], probe: bool) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = run(root.path(), &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config = root.path().join("config/config.toml");
    let initialized = fs::read_to_string(&config).unwrap();
    let owner = initialized
        .lines()
        .find(|line| line.starts_with("owner_uid"))
        .unwrap();
    fs::write(
        &config,
        format!("format = \"connectors-local/5\"\n{owner}\n{body}"),
    )
    .unwrap();
    #[cfg(unix)]
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    let paths = Paths {
        config,
        state: root.path().join("state"),
    };
    let loaded = Config::load(&paths.config).unwrap();
    for (alias, id) in cached {
        runtime::state::State::new(&paths.state)
            .remember(&loaded.adapters[*alias].selection(), &described(id, probe))
            .unwrap();
    }
    root
}

/// The one envelope the command answered, on stdout or stderr.
#[track_caller]
fn envelope(output: &Output) -> Value {
    let stream = if output.status.success() {
        &output.stdout
    } else {
        &output.stderr
    };
    serde_json::from_slice(stream).unwrap_or_else(|_| panic!("one JSON envelope: {output:?}"))
}

/// `[connectors.jira.instances.main]` (alias `jira-main`) beside a legacy
/// `[adapters.main]` catalog entry `setup migrate` derives `jira` for: the
/// name `jira/main` is held by two entries. `extra` is written first.
fn shared(extra: &str) -> tempfile::TempDir {
    configured(
        &format!(
            "{extra}{}[connectors.jira]\nenabled=true\n{}{}",
            legacy("main", "jira", "jira-legacy"),
            instance("jira", "main", "jira-main", "jira-main"),
            instance("jira", "other", "jira-other", "jira-other"),
        ),
        &[
            ("main", "jira-legacy"),
            ("jira-main", "jira-main"),
            ("jira-other", "jira-other"),
        ],
    )
}

/// The `error.data.code` an answer carries, `""` for a success.
fn code(answer: &Value) -> String {
    answer["error"]["data"]["code"]
        .as_str()
        .unwrap_or("")
        .to_owned()
}

/// story:legacy-alias-sharing-an-instance-name, contract "Invoking an
/// operation": "`--connector` with `--instance` and `default_instance` select by
/// the same rule and give the same refusal". Every command that takes
/// `--connector` and `--instance` (apps/connectors/spec/cli.yaml) is driven
/// with the shared name; each must answer `instance_ambiguous`, exit 2, before
/// any credential is read or any request sent.
#[test]
fn every_command_taking_connector_and_instance_refuses_a_shared_name() {
    let root = shared("");
    let file = root.path().join("doc.json");
    fs::write(&file, "{}").unwrap();
    let file = file.to_str().unwrap().to_owned();
    let commands: Vec<Vec<&str>> = vec![
        vec!["approvals", "policy-status"],
        vec!["approvals", "policy-set", "--input-file", &file],
        vec![
            "approvals",
            "prepare",
            "--connection",
            "c1",
            "--operation",
            WRITE,
            "--schema",
            "s1",
            "--revision",
            "desc-1",
            "--input-file",
            &file,
        ],
        vec![
            "approvals",
            "issue",
            "--connection",
            "c1",
            "--operation",
            WRITE,
            "--schema",
            "s1",
            "--revision",
            "desc-1",
            "--approve-subject",
            "x",
            "--proof-output",
            "proof.json",
            "--input-file",
            &file,
        ],
        vec!["instances", "describe"],
        vec!["instances", "status"],
        vec![
            "instances",
            "stop",
            "--expected-revision",
            "r1",
            "--host-incarnation",
            "h1",
            "--child-incarnation",
            "c1",
        ],
        vec!["connections", "list"],
        vec!["connections", "describe", "--connection", "c1"],
        vec![
            "connections",
            "connect",
            "--profile",
            "token",
            "--credential-file",
            &file,
        ],
        vec!["connections", "connect", "--profile", "token"],
        vec![
            "connections",
            "reconnect",
            "--connection",
            "c1",
            "--expected-revision",
            "r1",
            "--credential-file",
            &file,
        ],
        vec!["connections", "status", "--connection", "c1"],
        vec![
            "connections",
            "revalidate",
            "--connection",
            "c1",
            "--expected-revision",
            "r1",
        ],
        vec!["connections", "token", "--connection", "c1"],
        vec![
            "connections",
            "revoke",
            "--connection",
            "c1",
            "--expected-revision",
            "r1",
        ],
        vec!["operations", "list"],
        vec!["operations", "describe", "--operation", READ],
    ];
    let mut wrong = Vec::new();
    for command in &commands {
        let args = [&command[..], &["--connector", "jira", "--instance", "main"]].concat();
        let output = run(root.path(), &args);
        let answer = envelope(&output);
        if code(&answer) != "instance_ambiguous" || output.status.code() != Some(2) {
            wrong.push(format!(
                "{:?} exit {:?}: {answer}",
                command,
                output.status.code()
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "story:legacy-alias-sharing-an-instance-name: a shared name was not refused as \
         instance_ambiguous:\n{}",
        wrong.join("\n")
    );
}

/// The positional target of `connections connect` "selects as `--connector`
/// and `--instance` do": a shared name is refused before the credential.
#[test]
fn a_connect_target_naming_a_shared_name_is_refused_before_the_credential() {
    let root = shared("");
    let file = root.path().join("credential.json");
    fs::write(&file, "{\"token\":\"secret\"}").unwrap();
    let output = run(
        root.path(),
        &[
            "connections",
            "connect",
            "jira/main",
            "--profile",
            "token",
            "--credential-file",
            file.to_str().unwrap(),
        ],
    );
    let answer = envelope(&output);
    assert_eq!(code(&answer), "instance_ambiguous", "{answer}");
    assert_eq!(output.status.code(), Some(2), "{answer}");
}

/// A write through the shared name is refused at selection, as a read is,
/// whatever the approval file holds.
#[test]
fn a_write_through_a_shared_name_is_refused_at_selection() {
    let root = shared("");
    let approval = root.path().join("approval.json");
    fs::write(&approval, "{}").unwrap();
    let output = run(
        root.path(),
        &[
            "invoke",
            "jira/main",
            WRITE,
            "--connection",
            "c1",
            "--approval-file",
            approval.to_str().unwrap(),
            "--idempotency-key",
            "k1",
            "--input-json",
            "{}",
        ],
    );
    let answer = envelope(&output);
    assert_eq!(code(&answer), "instance_ambiguous", "{answer}");
}

/// The other instance of the connector, and each entry by its alias, are
/// selected as before: the shared name refuses only itself.
#[test]
fn the_entries_beside_a_shared_name_are_selected_as_before() {
    let root = shared("");
    for args in [
        &["--connector", "jira", "--instance", "other"][..],
        &["--adapter", "main"],
        &["--adapter", "jira-main"],
    ] {
        let output = run(
            root.path(),
            &[&["approvals", "policy-status"][..], args].concat(),
        );
        let answer = envelope(&output);
        assert!(output.status.success(), "{args:?}: {answer}");
    }
    let output = run(root.path(), &["invoke", "jira/other", READ, "--describe"]);
    assert!(output.status.success(), "{:?}", envelope(&output));
    let answer = envelope(&run(root.path(), &["invoke", "jira", READ, "--describe"]));
    assert_eq!(
        answer["error"]["data"]["instances"],
        json!([
            "jira/main (alias jira-main)",
            "jira/main (alias main)",
            "jira/other"
        ]),
        "{answer}"
    );
}

/// No shared name: a `connectors-local/5` file whose `default_instance` names
/// a legacy catalog entry by its own names, `catalog/<alias>`, is admitted
/// (`Config::check_selectors` selects it through `Config::select`), and before
/// b90f3b349 a command without a selector selected that entry
/// (`Config::select(Selector::Default)`). It must behave exactly as before: the
/// admitted default selects its entry, or the file is refused at load as a
/// `default_instance` naming no instance.
#[test]
fn an_admitted_default_instance_still_selects_its_entry() {
    let root = configured(
        &format!(
            "default_instance='catalog/main'\n{}",
            legacy("main", "jira", "jira-legacy")
        ),
        &[("main", "jira-legacy")],
    );
    let output = run(root.path(), &["approvals", "policy-status"]);
    let answer = envelope(&output);
    assert!(
        output.status.success() || code(&answer) == "invalid_configuration",
        "an admitted default_instance selects nothing and is not refused at load: {answer}"
    );
}

/// No shared name, the default instance a `[connectors]` instance: selected
/// as before, and a connector with several instances still needs one named.
#[test]
fn a_default_instance_without_a_shared_name_selects_as_before() {
    let root = configured(
        &format!(
            "default_instance='jira/main'\n[connectors.jira]\nenabled=true\n{}{}",
            instance("jira", "main", "jira-main", "jira-main"),
            instance("jira", "other", "jira-other", "jira-other"),
        ),
        &[("jira-main", "jira-main"), ("jira-other", "jira-other")],
    );
    let output = run(root.path(), &["approvals", "policy-status"]);
    assert!(output.status.success(), "{:?}", envelope(&output));
    let answer = envelope(&run(root.path(), &["invoke", "jira", READ, "--describe"]));
    assert_eq!(code(&answer), "instance_ambiguous", "{answer}");
    assert_eq!(
        answer["error"]["data"]["instances"],
        json!(["jira/main", "jira/other"]),
        "{answer}"
    );
}

/// Adversary pass 2. Contract "Invoking an operation", `instance_ambiguous`, as
/// b90f3b349 extends it to `--connector`/`--instance` on every command: "Nothing
/// was read beyond the configuration, no credential is read and no request is
/// sent." Each command here names a document file that does not exist. A
/// command that refuses the shared name before reading anything answers
/// `instance_ambiguous`; one that opens the document first answers the
/// document's own refusal. `invoke jira` (several instances, no instance) is
/// the same claim as it stood before the unit, for comparison.
#[test]
fn a_shared_name_reads_no_document_before_its_refusal() {
    let root = shared("");
    let missing = root.path().join("absent.json");
    let m: &str = missing.to_str().unwrap();
    let write = |target: &'static str| -> Vec<String> {
        [
            "invoke",
            target,
            WRITE,
            "--connection",
            "c1",
            "--approval-file",
            m,
            "--idempotency-key",
            "k1",
            "--input-json",
            "{}",
        ]
        .iter()
        .map(|arg| (*arg).to_owned())
        .collect()
    };
    let named = |command: &[&str]| -> Vec<String> {
        [command, &["--connector", "jira", "--instance", "main"]]
            .concat()
            .iter()
            .map(|arg| (*arg).to_owned())
            .collect()
    };
    let commands: Vec<Vec<String>> = vec![
        write("jira/main"),
        write("jira"),
        named(&["approvals", "policy-set", "--input-file", m]),
        named(&[
            "approvals",
            "prepare",
            "--connection",
            "c1",
            "--operation",
            WRITE,
            "--schema",
            "s1",
            "--revision",
            "desc-1",
            "--input-file",
            m,
        ]),
        named(&[
            "approvals",
            "issue",
            "--connection",
            "c1",
            "--operation",
            WRITE,
            "--schema",
            "s1",
            "--revision",
            "desc-1",
            "--approve-subject",
            "x",
            "--proof-output",
            "proof.json",
            "--input-file",
            m,
        ]),
        named(&[
            "connections",
            "connect",
            "--profile",
            "token",
            "--credential-file",
            m,
        ]),
    ];
    let mut wrong = Vec::new();
    for command in &commands {
        let args: Vec<&str> = command.iter().map(String::as_str).collect();
        let output = run(root.path(), &args);
        let answer = envelope(&output);
        if code(&answer) != "instance_ambiguous" {
            wrong.push(format!(
                "{:?} exit {:?}: {}",
                &args[..2],
                output.status.code(),
                answer["error"]["data"]
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "a shared name was refused only after a document was opened:\n{}",
        wrong.join("\n")
    );
}

/// Adversary pass 2. docs/local-catalog-provider.md: a legacy entry "keeps
/// working: each is selected by the connector `setup migrate` derives for it
/// and its alias"; contract "Giving an instance its endpoint": `--endpoint` on
/// a name a legacy entry holds is refused, the entry's endpoint being its own
/// pins. So `connections connect jira/main` on a pinned legacy `main` that
/// derives `jira` (Atlassian, no default endpoint) selects that entry and
/// asks for no endpoint, as `--adapter main` does; with `--endpoint` refused
/// for it there is no other answer an operator can give.
#[test]
fn a_legacy_entry_is_connected_by_its_name_without_an_endpoint() {
    let root = configured(
        &legacy("main", "jira", "jira-legacy"),
        &[("main", "jira-legacy")],
    );
    let file = root.path().join("credential.json");
    fs::write(&file, "{\"token\":\"secret\"}").unwrap();
    let file = file.to_str().unwrap();
    let connect = |selector: &[&str]| {
        envelope(&run(
            root.path(),
            &[
                &["connections", "connect"][..],
                selector,
                &["--profile", "token", "--credential-file", file],
            ]
            .concat(),
        ))
    };
    let by_alias = connect(&["--adapter", "main"]);
    assert_ne!(by_alias["error"]["data"]["field"], "endpoint", "{by_alias}");
    for selector in [
        &["jira/main"][..],
        &["--connector", "jira", "--instance", "main"],
    ] {
        let answer = connect(selector);
        assert_ne!(
            answer["error"]["data"]["field"], "endpoint",
            "{selector:?} asks for an endpoint the legacy entry pins; --adapter answered \
             {by_alias}; answered {answer}"
        );
    }
}
