//! story:connectors-do-short-invocation: `connectors invoke <connector>[/<instance>]
//! <operation>` invokes one operation of one configured instance. The instance
//! is the connector's only one when the target names no instance, the
//! connection is the instance's one `ready` connection, the schema and revision
//! are the instance's current admitted description, and the business document
//! is read from standard input when no input flag is given and standard input
//! is not a terminal (`contracts/cli/v1alpha1/semantics.md`, "Invoking an
//! operation"). The explicit service route (`describe`, `invoke --endpoint`,
//! `serve`) is gone: each is a parse refusal.
//!
//! The cases without a jail use a `connectors-local/5` configuration whose
//! executables are not installed, with cached descriptions: nothing they run
//! can reach a provider. The jailed cases run the production CLI, owner and
//! an adapter child against the read-only `mounted_secret` binding
//! (`support/mounted.rs`); Linux only, as that binding is.
#[cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
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
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

/// A directory holding no executable (`cli_surface_minor.rs`).
const NOT_INSTALLED: &str = if cfg!(windows) {
    "C:/not-installed"
} else {
    "/not-installed"
};
const CONNECTOR: &str = "fixture";
const READ: &str = "item.read";
const WRITE: &str = "item.write";
const REVISION: &str = "desc-1";

/// One configured instance of [`CONNECTOR`].
struct Instance {
    name: &'static str,
    alias: &'static str,
    id: &'static str,
}
const MAIN: Instance = Instance {
    name: "main",
    alias: "forge",
    id: "forge-local",
};
const BACKUP: Instance = Instance {
    name: "backup",
    alias: "forge-backup",
    id: "forge-backup",
};

fn input_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"name":{"type":"string"}}})
}

/// The cached description of instance `id`: one read and one write.
fn bootstrap(id: &str) -> runtime::Bootstrap {
    let operation = |op: &str, description: &str| connectors_core::Operation {
        id: op.into(),
        description: description.into(),
        contract: "operations/v1alpha1".into(),
        profile: "resource".into(),
        input_schema: input_schema(),
        output_schema: json!({"type":"object","additionalProperties":false}),
    };
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: id.into(),
        adapter: "catalog".into(),
        revision: REVISION.into(),
        operations: vec![
            operation(READ, "Read one fixture item"),
            operation(WRITE, "Write one fixture item"),
        ],
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

fn private(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

/// An initialised root whose configuration is `connectors-local/5` with
/// `extra` top-level text and each of `instances` under [`CONNECTOR`], every
/// one with its description cached.
fn configured(instances: &[&Instance], extra: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = run(root.path(), &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config = root.path().join("config/config.toml");
    let initialized = fs::read_to_string(&config).unwrap();
    let owner = initialized
        .lines()
        .find(|line| line.starts_with("owner_uid"))
        .unwrap();
    let mut text = format!(
        "format = \"connectors-local/5\"\n{owner}\n{extra}[connectors.{CONNECTOR}]\nenabled=true\n"
    );
    for instance in instances {
        let table = format!("connectors.{CONNECTOR}.instances.{}", instance.name);
        text.push_str(&format!(
            "[{table}]\nalias='{}'\ninstance_id='{}'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[{table}.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[{table}.permissions]\nprofiles=['token']\noperations=['{READ}','{WRITE}']\n",
            instance.alias,
            instance.id,
            "a".repeat(64)
        ));
    }
    private(&config, &text);
    remember(root.path(), instances);
    root
}

/// Cache each instance's description, as a launch for a description would.
fn remember(root: &Path, instances: &[&Instance]) {
    let paths = Paths {
        config: root.join("config/config.toml"),
        state: root.join("state"),
    };
    let config = Config::load(&paths.config).unwrap();
    for instance in instances {
        runtime::state::State::new(&paths.state)
            .remember(
                &config.adapters[instance.alias].selection(),
                &bootstrap(instance.id),
            )
            .unwrap();
    }
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

/// The command with standard input closed: nothing to read, and no terminal.
fn run(root: &Path, args: &[&str]) -> Output {
    command(root, args).stdin(Stdio::null()).output().unwrap()
}

/// The command with `stdin` written to its standard input.
fn run_with_stdin(root: &Path, args: &[&str], stdin: &[u8]) -> Output {
    let mut child = command(root, args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = child.stdin.take().unwrap().write_all(stdin);
    child.wait_with_output().unwrap()
}

/// The refusal's exit status and `error.data`, from the one envelope on
/// standard error.
#[track_caller]
fn refusal(output: &Output) -> (Option<i32>, Value) {
    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let envelope: Value = serde_json::from_slice(&output.stderr)
        .unwrap_or_else(|_| panic!("one JSON envelope: {output:?}"));
    assert_eq!(envelope["ok"], false, "{envelope}");
    (output.status.code(), envelope["error"]["data"].clone())
}

/// The successful result.
#[track_caller]
fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["ok"], true, "{envelope}");
    envelope["result"].clone()
}

/// Whether any object in `value` has a member named `adapter`.
fn names_adapter(value: &Value) -> bool {
    match value {
        Value::Object(fields) => {
            fields.contains_key("adapter") || fields.values().any(names_adapter)
        }
        Value::Array(items) => items.iter().any(names_adapter),
        _ => false,
    }
}

/// story:connectors-do-short-invocation: two instances of the connector and a
/// target naming the connector alone is `instance_ambiguous` at `arguments`,
/// exit 2, listing every instance as `<connector>/<instance>`, sorted.
#[test]
fn two_instances_and_the_connector_alone_are_instance_ambiguous() {
    let root = configured(&[&MAIN, &BACKUP], "");
    let output = run(
        root.path(),
        &["invoke", CONNECTOR, READ, "--input-json", "{}"],
    );
    let (code, data) = refusal(&output);
    assert_eq!(
        (code, data),
        (
            Some(2),
            json!({"kind":"usage","code":"instance_ambiguous","stage":"arguments","next_action":"none",
                "instances":["fixture/backup","fixture/main"]})
        ),
        "story:connectors-do-short-invocation: both instances are listed"
    );
}

/// story:connectors-do-short-invocation: a connector with no configured
/// instance is `not_found` at admission.
#[test]
fn a_connector_without_an_instance_is_not_found() {
    let root = configured(&[&MAIN], "");
    for target in ["nosuch", "nosuch/main", "fixture/nosuch"] {
        let (code, data) = refusal(&run(
            root.path(),
            &["invoke", target, READ, "--input-json", "{}"],
        ));
        assert_eq!(
            (code, &data["code"], &data["stage"], &data["next_action"]),
            (
                Some(1),
                &json!("not_found"),
                &json!("admission"),
                &json!("check_configuration")
            ),
            "story:connectors-do-short-invocation: {target}: {data}"
        );
    }
}

/// story:connectors-do-short-invocation: an instance that holds no connection
/// is `connection_unresolved` at admission naming a connect, with an empty
/// `connections` list.
#[test]
fn an_instance_without_a_ready_connection_is_connection_unresolved() {
    let root = configured(&[&MAIN], "");
    for target in [CONNECTOR, "fixture/main"] {
        let (code, data) = refusal(&run(
            root.path(),
            &["invoke", target, READ, "--input-json", "{}"],
        ));
        assert_eq!(
            (code, data),
            (
                Some(1),
                json!({"kind":"operational","code":"connection_unresolved","stage":"admission",
                    "next_action":"create_connection","connections":[]})
            ),
            "story:connectors-do-short-invocation: {target}"
        );
    }
}

/// story:connectors-do-short-invocation: a write without `--approval-file` is
/// `approval_required` naming the three approval commands, before any
/// provider call: the instance's executable is not installed, so any call
/// would have answered something else.
#[test]
fn a_write_without_an_approval_file_is_approval_required_naming_the_steps() {
    let root = configured(&[&MAIN], "");
    let (code, data) = refusal(&run(
        root.path(),
        &[
            "invoke",
            CONNECTOR,
            WRITE,
            "--connection",
            "c1",
            "--input-json",
            "{}",
        ],
    ));
    assert_eq!(
        (code, data),
        (
            Some(1),
            json!({"kind":"operational","code":"approval_required","stage":"admission","next_action":"none",
                "approval_steps":["approvals prepare","approvals clock-check","approvals issue"]})
        ),
        "story:connectors-do-short-invocation"
    );
}

/// story:connectors-do-short-invocation: `--describe` prints the operation's
/// description, input schema, effect, schema and revision, and invokes
/// nothing: no connection is held and no executable is installed.
#[test]
fn describe_prints_the_operation_and_calls_no_provider() {
    let root = configured(&[&MAIN], "");
    for (operation, description, effect) in [
        (READ, "Read one fixture item", "read"),
        (WRITE, "Write one fixture item", "write"),
    ] {
        let result = success(&run(
            root.path(),
            &["invoke", CONNECTOR, operation, "--describe"],
        ));
        assert_eq!(
            result,
            json!({"connector":CONNECTOR,"instance":"main","operation":operation,
                "description":description,"effect":effect,
                "schema":connectors_host::local::owner::schema(&bootstrap(MAIN.id), operation).unwrap(),
                "revision":REVISION,"input_schema":input_schema()}),
            "story:connectors-do-short-invocation: {operation}"
        );
    }
}

/// story:connectors-do-short-invocation: a `--revision` or `--schema` pin that
/// differs from the instance's current description is refused as
/// `stale_description`, never re-resolved to the current one.
#[test]
fn a_stale_pin_is_refused_not_re_resolved() {
    let root = configured(&[&MAIN], "");
    let schema = connectors_host::local::owner::schema(&bootstrap(MAIN.id), READ).unwrap();
    for pin in [
        &["--revision", "desc-0"][..],
        &["--schema", "0000000000000000"],
        &["--revision", "desc-0", "--schema", &schema],
    ] {
        let args = [
            &[
                "invoke",
                CONNECTOR,
                READ,
                "--connection",
                "c1",
                "--input-json",
                "{}",
            ][..],
            pin,
        ]
        .concat();
        let (code, data) = refusal(&run(root.path(), &args));
        assert_eq!(
            (code, &data["code"], &data["stage"]),
            (Some(1), &json!("stale_description"), &json!("observation")),
            "story:connectors-do-short-invocation: {pin:?}: {data}"
        );
    }
}

/// story:connectors-do-short-invocation: `invoke` takes no `--endpoint` and no
/// selector option: each is a parse refusal.
#[test]
fn endpoint_and_selector_options_are_parse_refusals() {
    let root = configured(&[&MAIN], "");
    for args in [
        &["invoke", "--endpoint", "x"][..],
        &["invoke", CONNECTOR, READ, "--endpoint", "x"],
        &["invoke", "--connector", "x"],
        &["invoke", CONNECTOR, READ, "--connector", "x"],
        &["invoke", CONNECTOR, READ, "--instance", "main"],
        &["invoke", CONNECTOR, READ, "--adapter", "forge"],
    ] {
        let output = run(root.path(), args);
        assert_eq!(
            (
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            ),
            (
                Some(2),
                "{\"error\":{\"code\":\"cli_parse\",\"data\":{}},\"ok\":false}\n".into()
            ),
            "story:connectors-do-short-invocation: {args:?}"
        );
        assert!(output.stdout.is_empty());
    }
}

/// story:connectors-do-short-invocation: the explicit service commands are
/// removed. Their command lines are parse refusals, in both output modes.
#[test]
fn the_explicit_service_commands_are_parse_refusals() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        &[
            "invoke",
            "--endpoint",
            "https://127.0.0.1:9/",
            "--token-file",
            "/fixture/token",
            "--operation",
            "read",
            "--input",
            "/fixture/input.json",
        ][..],
        &[
            "describe",
            "--endpoint",
            "https://127.0.0.1:9/",
            "--token-file",
            "/fixture/token",
        ],
        &["serve", "--config", "/fixture/federation.toml"],
    ] {
        for (output, expected) in [
            (&[][..], "cli_parse\n"),
            (
                &["--output", "json"][..],
                "{\"error\":{\"code\":\"cli_parse\",\"data\":{}},\"ok\":false}\n",
            ),
        ] {
            let ran = Command::new(env!("CARGO_BIN_EXE_connectors"))
                .args(output)
                .args(args)
                .env("HOME", home.path())
                .env_remove("XDG_CONFIG_HOME")
                .env_remove("XDG_STATE_HOME")
                .stdin(Stdio::null())
                .output()
                .unwrap();
            assert_eq!(
                (
                    ran.status.code(),
                    String::from_utf8_lossy(&ran.stdout).into_owned(),
                    String::from_utf8_lossy(&ran.stderr).into_owned()
                ),
                (Some(2), String::new(), expected.to_owned()),
                "story:connectors-do-short-invocation: {output:?} {args:?}"
            );
        }
    }
}

/// story:connectors-do-short-invocation: every `invalid_input` `invoke`
/// answers names the argument it refused in `field`, exit 2.
#[test]
fn every_invalid_input_of_invoke_names_its_field() {
    let root = configured(&[&MAIN], "");
    let definitions = root.path().join("defs/connectors");
    fs::create_dir_all(&definitions).unwrap();
    let definition = definitions.join("forge.yaml");
    fs::write(
        &definition,
        "schema: urn:beyond10x:connectors-definition:1\nsource:\n  openapi: upstream/forge/openapi.json.gz\n  sha256: 01fcc470fbbd1a2c691d5a17c35a00b086ca5e62fdd08747f92cb350e0fc6fa1\nbase_url: https://{hostname}/api/v4\nconfig: {hostname: {type: string, default: forge.example.com}}\nauth:\n  forge.pat:\n    scheme: header\n    header: PRIVATE-TOKEN\n    label: Fixture personal access token\n    identity: {path: user, kind: forge.user, subject: /id}\n    scopes: {path: personal_access_tokens/self, pointer: /scopes, minimum: [api]}\noperations:\n  project.get:\n    op: getApiV4ProjectsId\n    description: Read one project\n",
    )
    .unwrap();
    let definition = definition.to_str().unwrap();
    let absent = root.path().join("absent.json");
    let absent = absent.to_str().unwrap();
    let pinned = ["--connection", "c1"];
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (vec!["invoke", "fixture/main/extra", READ], "target"),
        (vec!["invoke", "/main", READ], "target"),
        (vec!["invoke", "fixture/", READ], "target"),
        (vec!["invoke", "fix ture", READ], "target"),
        (vec!["invoke", "", READ], "target"),
        (vec!["invoke", CONNECTOR, "item read"], "operation"),
        (
            vec!["invoke", CONNECTOR, READ, "--connection", "c 1"],
            "connection",
        ),
        (
            [&["invoke", CONNECTOR, READ, "--schema", "s 1"][..], &pinned].concat(),
            "schema",
        ),
        (
            [
                &["invoke", CONNECTOR, READ, "--revision", "r 1"][..],
                &pinned,
            ]
            .concat(),
            "revision",
        ),
        (
            [
                &["invoke", CONNECTOR, READ, "--input-file", absent][..],
                &pinned,
            ]
            .concat(),
            "input",
        ),
        (
            [
                &["invoke", CONNECTOR, READ, "--input-json", "{\"name\":1}"][..],
                &pinned,
            ]
            .concat(),
            "input",
        ),
        (
            [
                &["invoke", CONNECTOR, READ, "--idempotency-key", "k1"][..],
                &pinned,
            ]
            .concat(),
            "idempotency_key",
        ),
        (
            [
                &["invoke", CONNECTOR, READ, "--approval-file", absent][..],
                &pinned,
            ]
            .concat(),
            "approval_file",
        ),
        (
            [
                &[
                    "invoke",
                    CONNECTOR,
                    WRITE,
                    "--approval-file",
                    absent,
                    "--idempotency-key",
                    "",
                ][..],
                &pinned,
            ]
            .concat(),
            "idempotency_key",
        ),
        (
            vec!["invoke", CONNECTOR, READ, "--definitions-root", "/fixture"],
            "definitions_root",
        ),
        (
            vec!["invoke", CONNECTOR, READ, "--credential-file", "/fixture"],
            "credential_file",
        ),
        (
            vec!["invoke", "forge", "project.get", "--definition", absent],
            "definition",
        ),
        (
            vec![
                "invoke",
                CONNECTOR,
                "project.get",
                "--definition",
                definition,
            ],
            "target",
        ),
    ];
    let mut seen = Vec::new();
    let mut expected = Vec::new();
    for (mut args, field) in cases {
        if !args.iter().any(|arg| arg.starts_with("--input-")) {
            args.extend(["--input-json", "{}"]);
        }
        let (code, data) = refusal(&run(root.path(), &args));
        seen.push((
            args.join(" "),
            code,
            data["code"].clone(),
            data["stage"].clone(),
            data["field"].clone(),
        ));
        expected.push((
            args.join(" "),
            Some(2),
            json!("invalid_input"),
            json!("arguments"),
            json!(field),
        ));
    }
    assert_eq!(seen, expected, "story:connectors-do-short-invocation");
}

/// story:connectors-do-short-invocation: with no input flag and standard input
/// not a terminal, the document is read from standard input: one the
/// operation's input schema refuses is `invalid_input` naming `input`, and
/// one it admits goes on to the connection.
#[test]
fn without_an_input_flag_the_document_is_read_from_stdin() {
    let root = configured(&[&MAIN], "");
    let args = ["invoke", CONNECTOR, READ, "--connection", "c1"];
    let (code, data) = refusal(&run_with_stdin(root.path(), &args, b"{\"name\":1}"));
    assert_eq!(
        (code, &data["code"], &data["field"]),
        (Some(2), &json!("invalid_input"), &json!("input")),
        "story:connectors-do-short-invocation: {data}"
    );
    let (code, data) = refusal(&run_with_stdin(root.path(), &args, b"{\"name\":\"item\"}"));
    assert_eq!(
        (code, &data["code"]),
        (Some(1), &json!("not_found")),
        "story:connectors-do-short-invocation: the admitted document reached the connection: {data}"
    );
}

/// story:connectors-do-short-invocation: a document with a repeated key, at
/// the top or nested, from any source, is the generated runtime's
/// `cli_dynamic_input` (`contracts/cli/v1alpha1/semantics.md`, the parse
/// refusal table), before the connection is admitted and before any provider
/// call: the connection `c1` is not held, so reaching it would answer
/// `not_found`. The explicit service route refused it before contacting the
/// service (`tests/input.rs`).
#[test]
fn repeated_input_keys_are_refused_before_any_provider_call() {
    let root = configured(&[&MAIN], "");
    let file = root.path().join("input.json");
    let refused = "{\"error\":{\"code\":\"cli_dynamic_input\",\"data\":{}},\"ok\":false}\n";
    let args = ["invoke", CONNECTOR, READ, "--connection", "c1"];
    for document in [r#"{"name":"a","name":"b"}"#, r#"{"nested":{"x":1,"x":2}}"#] {
        fs::write(&file, document).unwrap();
        for output in [
            run(
                root.path(),
                &[&args[..], &["--input-json", document]].concat(),
            ),
            run(
                root.path(),
                &[&args[..], &["--input-file", file.to_str().unwrap()]].concat(),
            ),
            run_with_stdin(root.path(), &args, document.as_bytes()),
        ] {
            assert_eq!(
                (
                    output.status.code(),
                    String::from_utf8_lossy(&output.stdout).into_owned(),
                    String::from_utf8_lossy(&output.stderr).into_owned()
                ),
                (Some(2), String::new(), refused.to_owned()),
                "story:connectors-do-short-invocation: {document}"
            );
        }
    }
}

/// story:connectors-do-short-invocation: with no input flag and standard input
/// a terminal, `invoke` refuses at once with `invalid_input` naming `input`,
/// without reading it.
#[cfg(target_os = "linux")]
#[test]
fn a_terminal_stdin_without_an_input_flag_is_refused_without_reading() {
    use std::os::fd::{FromRawFd, OwnedFd};
    let root = configured(&[&MAIN], "");
    let (mut master, mut slave) = (-1, -1);
    // SAFETY: two out-pointers to live integers; no name, settings or size.
    let opened = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    assert_eq!(opened, 0, "openpty");
    // SAFETY: openpty returned two open descriptors this test now owns.
    let (master, slave) = unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
    let mut child = command(
        root.path(),
        &["invoke", CONNECTOR, READ, "--connection", "c1"],
    )
    .stdin(Stdio::from(slave))
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            panic!("story:connectors-do-short-invocation: invoke waited on the terminal");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(master);
    let output = child.wait_with_output().unwrap();
    let (code, data) = refusal(&output);
    assert_eq!(
        (code, data),
        (
            Some(2),
            json!({"kind":"usage","code":"invalid_input","stage":"arguments","next_action":"none","field":"input"})
        ),
        "story:connectors-do-short-invocation"
    );
}

/// story:connectors-do-short-invocation: a legacy `[adapters.<alias>]` catalog
/// entry is named by the connector `setup migrate` derives for it, never
/// `catalog`.
#[test]
fn a_legacy_catalog_entry_is_named_by_its_derived_connector() {
    let root = tempfile::tempdir().unwrap();
    let init = run(root.path(), &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config = root.path().join("config/config.toml");
    let initialized = fs::read_to_string(&config).unwrap();
    let owner = initialized
        .lines()
        .find(|line| line.starts_with("owner_uid"))
        .unwrap();
    private(
        &config,
        &format!(
            "format = \"connectors-local/4\"\n{owner}\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.forge.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=['--connector','jira']\n[adapters.forge.permissions]\nprofiles=['token']\noperations=['{READ}','{WRITE}']\n",
            "a".repeat(64)
        ),
    );
    remember(root.path(), &[&MAIN]);
    for target in ["jira", "jira/forge"] {
        let result = success(&run(root.path(), &["invoke", target, READ, "--describe"]));
        assert_eq!(
            (&result["connector"], &result["instance"]),
            (&json!("jira"), &json!("forge")),
            "story:connectors-do-short-invocation: {target}: {result}"
        );
    }
    for target in ["catalog", "catalog/forge"] {
        let (code, data) = refusal(&run(root.path(), &["invoke", target, READ, "--describe"]));
        assert_eq!(
            (code, &data["code"]),
            (Some(1), &json!("not_found")),
            "story:connectors-do-short-invocation: {target}: {data}"
        );
    }
}

/// story:connectors-do-short-invocation: no `invoke` answer, success or
/// refusal, carries an `adapter` member.
#[test]
fn no_invoke_answer_names_adapter() {
    let root = configured(&[&MAIN, &BACKUP], "");
    for args in [
        &["invoke", CONNECTOR, READ, "--input-json", "{}"][..],
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
        &[
            "invoke",
            "fixture/main",
            READ,
            "--connection",
            "c1",
            "--revision",
            "x",
            "--input-json",
            "{}",
        ],
        &["invoke", "fixture/main", READ, "--describe"],
    ] {
        let output = run(root.path(), args);
        let stream = if output.status.success() {
            &output.stdout
        } else {
            &output.stderr
        };
        let envelope: Value = serde_json::from_slice(stream).unwrap();
        assert!(
            !names_adapter(&envelope),
            "story:connectors-do-short-invocation: {args:?}: {envelope}"
        );
    }
}

#[cfg(target_os = "linux")]
mod jailed {
    use super::{Value, json, names_adapter};
    use crate::mounted::*;
    use connectors_custody::mounted::fixture::Map;
    use std::{
        fs,
        io::Write,
        os::unix::fs::PermissionsExt,
        process::{Output, Stdio},
    };

    /// The adapter child the owner starts filters to this test's exact name.
    pub const ADAPTER_FIXTURE: &str = "jailed::invoke_short_adapter_fixture";

    /// The adapter child the owner starts: this binary, filtered to this test.
    /// The ordinary run does nothing.
    #[test]
    fn invoke_short_adapter_fixture() {
        serve_when_child();
    }

    /// The fixture adapter configured as instance `fixture/main` (alias
    /// `forge`) of a `connectors-local/5` file at configuration revision
    /// `revision`, custody the mounted directory.
    fn configure(container: &Container, revision: &str) {
        let uid = connectors_host::local::filesystem::uid();
        let adapter = digest(&fs::read(container.path("/bin-test/adapter")).unwrap());
        let args = serde_json::to_string(&["--exact", ADAPTER_FIXTURE, "--"]).unwrap();
        let table = "connectors.fixture.instances.main";
        let text = format!(
            "format='connectors-local/5'\nowner_uid={uid}\n{}\n[connectors.fixture]\nenabled=true\n[{table}]\nalias='{ALIAS}'\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='{revision}'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[{table}.permissions]\nprofiles=['{PROFILE}','{OAUTH_PROFILE}']\noperations=['{READ}']\n[{table}.executable]\npath='/bin-test/adapter'\nsha256='{adapter}'\nargs={args}\n",
            Container::mounted()
        );
        let path = container.path(CONFIG);
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    /// A jail whose instance holds one connection, made by a connect.
    fn connected() -> (Container, String) {
        let container = Container::new(ADAPTER_FIXTURE);
        container.deliver(PROFILE, SECRET);
        configure(&container, "cfg-1");
        let init = success(&container.run(&["setup", "init"]));
        assert_eq!(init["disposition"], "state_initialized", "{init}");
        let connected = success(&container.connect(PROFILE));
        let connection = connected["connection"]["summary"]["connection"]
            .as_str()
            .unwrap()
            .to_owned();
        (container, connection)
    }

    /// The CLI in the jail with `stdin` written to its standard input.
    fn run_with_stdin(container: &Container, args: &[&str], stdin: &[u8]) -> Output {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut child = loop {
            let spawned = container
                .jail
                .command("/bin-test/connectors", Map::Own)
                .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
                .args(args)
                .env_clear()
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
            match spawned {
                Ok(child) => break child,
                // The installed binary may still be open for writing.
                Err(error)
                    if error.raw_os_error() == Some(libc::ETXTBSY)
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(error) => panic!("spawn in the jail: {error}"),
            }
        };
        let _ = child.stdin.take().unwrap().write_all(stdin);
        child.wait_with_output().unwrap()
    }

    /// story:connectors-do-short-invocation: one instance, the document on
    /// standard input and no `--connection`, `--schema` or `--revision`:
    /// `invoke <connector> <operation>` invokes it once, and the result names
    /// the connector and instance and no `adapter`.
    #[test]
    fn the_connector_alone_and_stdin_invoke_the_one_instance() {
        let (container, _) = connected();
        let output = run_with_stdin(&container, &["invoke", "fixture", READ], b"{}");
        let result = success(&output);
        assert_eq!(
            result,
            json!({"connector":"fixture","instance":"main","operation":READ,"revision":REVISION,
                "result":{"credential":digest(SECRET)}}),
            "story:connectors-do-short-invocation"
        );
        let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(!names_adapter(&envelope), "{envelope}");
        assert_eq!(container.seen("invocations"), [digest(SECRET)]);
    }

    /// story:connectors-do-short-invocation: the one connection the instance
    /// holds is selected without `--connection`; a pinned connection it does
    /// not hold is refused, not replaced by the held one.
    #[test]
    fn the_one_held_connection_is_resolved_without_naming_it() {
        let (container, connection) = connected();
        let resolved =
            success(&container.run(&["invoke", "fixture/main", READ, "--input-json", "{}"]));
        let pinned = success(&container.run(&[
            "invoke",
            "fixture/main",
            READ,
            "--connection",
            &connection,
            "--input-json",
            "{}",
        ]));
        assert_eq!(resolved, pinned, "story:connectors-do-short-invocation");
        let (code, data) = refusal(&container.run(&[
            "invoke",
            "fixture/main",
            READ,
            "--connection",
            "0fd9de39-aed4-4673-8330-12f20cd0b962",
            "--input-json",
            "{}",
        ]));
        assert_eq!(
            (code, &data["code"]),
            (Some(1), &json!("not_found")),
            "story:connectors-do-short-invocation: {data}"
        );
        assert_eq!(container.seen("invocations").len(), 2);
    }

    /// story:connectors-do-short-invocation: an instance whose one connection
    /// is not `ready` is `connection_unresolved` naming a repair and listing
    /// the connection with its state.
    #[test]
    fn a_held_connection_that_is_not_ready_is_connection_unresolved() {
        let (container, connection) = connected();
        // A changed configuration revision leaves the connection pending.
        configure(&container, "cfg-2");
        let (code, data) =
            refusal(&container.run(&["invoke", "fixture", READ, "--input-json", "{}"]));
        assert_eq!(
            (code, data),
            (
                Some(1),
                json!({"kind":"operational","code":"connection_unresolved","stage":"admission",
                    "next_action":"repair_connection",
                    "connections":[{"connection":connection,"state":"pending"}]})
            ),
            "story:connectors-do-short-invocation"
        );
        assert!(container.seen("invocations").is_empty());
    }
}
