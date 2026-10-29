//! story:connectors-do-short-invocation, adversary pass 1 on wave D218 unit U1:
//! `connectors invoke <connector>[/<instance>] <operation>` driven against the
//! approved decisions (D1-D8), the story's acceptance and the contract text the
//! unit wrote (`contracts/cli/v1alpha1/semantics.md`, "Invoking an
//! operation").
//!
//! The cases without a jail use a `connectors-local/5` configuration whose
//! executables are not installed, with cached descriptions. The jailed cases
//! run the production CLI, owner and an adapter child against the read-only
//! `mounted_secret` binding (`support/mounted.rs`); Linux only. The definition
//! case needs the built catalog provider named by
//! `CONNECTORS_TEST_CATALOG_PROVIDER`, as `definition_connect_question.rs`
//! does, and is ignored without it.
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

/// One `fixture` instance, `main`, described.
fn one_instance() -> tempfile::TempDir {
    configured(
        &format!(
            "[connectors.fixture]\nenabled=true\n{}",
            instance("fixture", "main", "forge", "forge-local")
        ),
        &[("forge", "forge-local")],
    )
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

/// A legacy `[adapters.main]` catalog entry that `setup migrate` derives
/// connector `jira` for, beside `[connectors.jira.instances.main]`. The file
/// is admitted: the legacy entry's own names are `catalog/main`, so no two
/// entries share names (`Config::check_selectors`). `invoke` names the legacy
/// entry `jira/main` as well (D7), so `jira/main` names two entries.
fn legacy_beside_instance_of_the_same_name() -> tempfile::TempDir {
    configured(
        &format!(
            "{}[connectors.jira]\nenabled=true\n{}",
            legacy("main", "jira", "jira-legacy"),
            instance("jira", "main", "jira-main", "jira-main")
        ),
        &[("main", "jira-legacy"), ("jira-main", "jira-main")],
    )
}

/// story:connectors-do-short-invocation, D2: `instance_ambiguous` is the
/// answer to a target that leaves the instance out. A target that names the
/// instance, `jira/main`, is never `instance_ambiguous`, and no refusal lists
/// one instance twice.
#[test]
fn a_named_instance_is_never_instance_ambiguous() {
    let root = legacy_beside_instance_of_the_same_name();
    let output = run(root.path(), &["invoke", "jira/main", READ, "--describe"]);
    let answer = envelope(&output);
    let data = &answer["error"]["data"];
    assert_ne!(
        data["code"], "instance_ambiguous",
        "story:connectors-do-short-invocation: a target naming its instance was refused as \
         ambiguous: {answer}"
    );
}

/// story:connectors-do-short-invocation, D2: `Failure.instances` lists every
/// instance of the connector as `<connector>/<instance>`, sorted, so the
/// caller can name one. An entry listed twice cannot be named.
#[test]
fn the_ambiguity_list_names_each_instance_once() {
    let root = legacy_beside_instance_of_the_same_name();
    let output = run(root.path(), &["invoke", "jira", READ, "--describe"]);
    let answer = envelope(&output);
    let listed: Vec<String> = answer["error"]["data"]["instances"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| item.as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default();
    let once: BTreeSet<&String> = listed.iter().collect();
    assert_eq!(
        once.len(),
        listed.len(),
        "story:connectors-do-short-invocation: an instance is listed twice: {answer}"
    );
}

/// story:connectors-do-short-invocation, D4: an operation whose description
/// leaves its effect unknown cannot be shown to be a read, so neither it nor
/// any operation of that description is invoked: each is refused before any
/// provider call.
#[test]
fn an_operation_of_unknown_effect_is_refused_before_any_provider_call() {
    let root = configured_with(
        &format!(
            "[connectors.fixture]\nenabled=true\n{}",
            instance("fixture", "main", "forge", "forge-local")
        ),
        &[("forge", "forge-local")],
        true,
    );
    // `c1` is not held and the executable is not installed: a call that went
    // on would answer `not_found` or `unavailable`.
    for extra in [
        &[PROBE][..],
        &[PROBE, "--idempotency-key", "k1"],
        &[WRITE],
        &[READ],
    ] {
        let args = [
            &[
                "invoke",
                "fixture",
                "--connection",
                "c1",
                "--input-json",
                "{}",
            ][..],
            extra,
        ]
        .concat();
        let output = run(root.path(), &args);
        let answer = envelope(&output);
        let code = answer["error"]["data"]["code"].as_str().unwrap_or("");
        assert!(
            matches!(code, "approval_required" | "unsupported"),
            "story:connectors-do-short-invocation: {args:?}: {answer}"
        );
    }
}

/// story:connectors-do-short-invocation, contract "Invoking an operation": with
/// no input flag a standard input that is not a terminal is read, and one that
/// holds no JSON value (closed at once, or empty) is `cli_dynamic_input`, not a
/// wait and not an `invalid_input` without `field`.
#[test]
fn a_closed_or_empty_stdin_is_one_undecodable_document() {
    let root = one_instance();
    let args = ["invoke", "fixture/main", READ, "--connection", "c1"];
    let refused = "{\"error\":{\"code\":\"cli_dynamic_input\",\"data\":{}},\"ok\":false}\n";
    let closed = run(root.path(), &args);
    let mut child = command(root.path(), &args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdin.take());
    let empty = child.wait_with_output().unwrap();
    for (what, output) in [("closed", closed), ("empty", empty)] {
        assert_eq!(
            (
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).into_owned()
            ),
            (Some(2), refused.to_owned()),
            "story:connectors-do-short-invocation: {what}"
        );
    }
}

/// story:connectors-do-short-invocation, D8 and the contract: a target part of
/// more than 128 bytes or outside `[A-Za-z0-9._-]` is `invalid_input` naming
/// `target`; an operation of 129 bytes names `operation`; a target that is not
/// UTF-8 is a parse refusal, never an `invalid_input` without `field`.
#[test]
fn over_long_and_non_ascii_names_are_refused_naming_their_argument() {
    let root = one_instance();
    let long = "a".repeat(129);
    let cases: Vec<(Vec<String>, &str)> = vec![
        (vec![long.clone(), READ.into()], "target"),
        (vec![format!("fixture/{long}"), READ.into()], "target"),
        (vec!["fixtüre".into(), READ.into()], "target"),
        (vec!["fixture/maın".into(), READ.into()], "target"),
        (vec!["fixture/main".into(), long.clone()], "operation"),
    ];
    for (positionals, field) in cases {
        let mut args = vec!["invoke".to_owned()];
        args.extend(positionals);
        args.extend(["--input-json".into(), "{}".into()]);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let answer = envelope(&run(root.path(), &args));
        let data = &answer["error"]["data"];
        assert_eq!(
            (&data["code"], &data["field"]),
            (&json!("invalid_input"), &json!(field)),
            "story:connectors-do-short-invocation: {:?}: {answer}",
            &args[1..3]
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let output = command(root.path(), &["invoke"])
            .arg(std::ffi::OsStr::from_bytes(b"fixture/m\xffin"))
            .args([READ, "--input-json", "{}"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let answer = envelope(&output);
        assert_eq!(output.status.code(), Some(2), "{answer}");
        assert!(
            answer["error"]["data"]["code"] != "invalid_input"
                || answer["error"]["data"]["field"] == "target",
            "story:connectors-do-short-invocation: a non-UTF-8 target: {answer}"
        );
    }
}

/// story:connectors-do-short-invocation, D6/D8: `--describe` given anything but
/// a Boolean is a parse refusal; it never answers a description, and it is
/// never an `invalid_input` without `field`.
#[test]
fn describe_given_a_value_that_is_no_boolean_is_refused() {
    let root = one_instance();
    for args in [
        &["invoke", "fixture/main", READ, "--describe=yes"][..],
        &["invoke", "fixture/main", READ, "--describe", "yes"],
        &["invoke", "fixture/main", READ, "--describe="],
    ] {
        let output = run(root.path(), args);
        let answer = envelope(&output);
        let data = &answer["error"]["data"];
        assert!(
            output.status.code() == Some(2)
                && (data["code"] != "invalid_input" || !data["field"].is_null()),
            "story:connectors-do-short-invocation: {args:?}: {answer}"
        );
    }
}

/// story:connectors-do-short-invocation, D1: the preparation restates every
/// value as `--<long>=<value>` and the target after `--`. A value holding `=`,
/// or spelled like a flag (`--describe`, `--input-json`), stays that value: the
/// write below is refused at admission, never described, never parsed
/// differently, and its key is never the refused argument.
#[test]
fn values_spelled_like_flags_or_holding_equals_stay_values() {
    let root = one_instance();
    let absent = root.path().join("no=proof.json");
    let absent = absent.to_str().unwrap();
    for key in ["--describe", "--input-json", "a=b", "--connection=x"] {
        let approval = format!("--approval-file={absent}");
        let idempotency = format!("--idempotency-key={key}");
        let output = run(
            root.path(),
            &[
                "invoke",
                "fixture/main",
                WRITE,
                "--connection",
                "c1",
                &approval,
                &idempotency,
                "--input-json={\"name\":\"--describe=true\"}",
            ],
        );
        let answer = envelope(&output);
        let data = &answer["error"]["data"];
        // Admitted as given: refused at admission (the proof is absent, `c1`
        // is not held), never parsed as another command line or described.
        assert!(
            !output.status.success()
                && answer["error"]["code"] != "cli_parse"
                && data["field"] != "idempotency_key"
                && data["code"] != "stale_description",
            "story:connectors-do-short-invocation: key {key:?}: {answer}"
        );
    }
}

/// story:connectors-do-short-invocation, D9: the explicit service commands are
/// gone from root help, from `invoke`'s help and from the completion script.
#[test]
fn removed_commands_appear_in_no_help_or_completion() {
    let home = tempfile::tempdir().unwrap();
    let run_plain = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_connectors"))
            .args(args)
            .env("HOME", home.path())
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("XDG_STATE_HOME")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let root = run_plain(&["--help"]);
    let root_help = String::from_utf8_lossy(&root.stdout);
    // `server`, the MCP command, stays; `serve` and `describe` are gone.
    for word in ["describe", "serve", "Explicit"] {
        assert!(
            !root_help
                .lines()
                .any(|line| line.split_whitespace().next() == Some(word)),
            "story:connectors-do-short-invocation: root help lists {word:?}: {root_help}"
        );
    }
    let invoke = run_plain(&["invoke", "--help"]);
    let completion = run_plain(&["completions", "bash"]);
    for (what, output) in [
        ("invoke --help", &invoke),
        ("completions bash", &completion),
    ] {
        assert!(output.status.success(), "{what}: {output:?}");
        let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
        for removed in ["--endpoint", "--token-file", "--allow-plaintext"] {
            assert!(
                !text.contains(removed),
                "story:connectors-do-short-invocation: {what} offers {removed}"
            );
        }
    }
    let invoke_help = String::from_utf8_lossy(&invoke.stdout).to_lowercase();
    for word in ["adapter", "catalog", "--connector", "--instance"] {
        assert!(
            !invoke_help.contains(word),
            "story:connectors-do-short-invocation: invoke --help says {word:?}: {invoke_help}"
        );
    }
    for args in [
        &["help", "serve"][..],
        &["help", "describe"],
        &["serve", "--help"],
        &["describe", "--help"],
    ] {
        let output = run_plain(args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "story:connectors-do-short-invocation: {args:?}: {output:?}"
        );
    }
}

#[cfg(target_os = "linux")]
mod jailed {
    use super::{Value, json};
    use crate::mounted::*;
    use std::{fs, os::unix::fs::PermissionsExt, process::Output, time::Duration};

    pub const ADAPTER_FIXTURE: &str = "jailed::adversary_d218_adapter_fixture";

    /// The adapter child the owner starts: this binary, filtered to this test.
    #[test]
    fn adversary_d218_adapter_fixture() {
        serve_when_child();
    }

    /// The fixture adapter as instance `fixture/main` (alias `forge`) of a
    /// `connectors-local/5` file, custody the mounted directory.
    fn configure(container: &Container) {
        let uid = connectors_host::local::filesystem::uid();
        let adapter = digest(&fs::read(container.path("/bin-test/adapter")).unwrap());
        let args = serde_json::to_string(&["--exact", ADAPTER_FIXTURE, "--"]).unwrap();
        let table = "connectors.fixture.instances.main";
        let text = format!(
            "format='connectors-local/5'\nowner_uid={uid}\n{}\n[connectors.fixture]\nenabled=true\n[{table}]\nalias='{ALIAS}'\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[{table}.permissions]\nprofiles=['{PROFILE}','{OAUTH_PROFILE}']\noperations=['{READ}']\n[{table}.executable]\npath='/bin-test/adapter'\nsha256='{adapter}'\nargs={args}\n",
            Container::mounted()
        );
        let path = container.path(CONFIG);
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    /// A jail whose instance holds one connection whose validation evidence
    /// stays current for `evidence_ms`: the connection and its revision.
    fn connected(evidence_ms: u64) -> (Container, String, String) {
        let container = Container::new(ADAPTER_FIXTURE);
        container.deliver(PROFILE, SECRET);
        configure(&container);
        success(&container.run(&["setup", "init"]));
        container.set_knob("evidence_ms", evidence_ms);
        let connected = success(&container.connect(PROFILE));
        let summary = &connected["connection"]["summary"];
        assert_eq!(summary["state"], "ready", "{connected}");
        (
            container,
            summary["connection"].as_str().unwrap().to_owned(),
            summary["revision"].as_str().unwrap().to_owned(),
        )
    }

    fn answer(output: &Output) -> Value {
        let stream = if output.status.success() {
            &output.stdout
        } else {
            &output.stderr
        };
        serde_json::from_slice(stream).unwrap_or_else(|_| panic!("{output:?}"))
    }

    /// story:connectors-do-short-invocation, D3 and the acceptance: "An
    /// instance with no `ready` connection [...] is refused by
    /// `connection_unresolved`, listing each connection and its state." The
    /// instance's one connection reads `pending` once its validation evidence
    /// lapsed; `invoke` without `--connection` refuses it and sends nothing to
    /// the provider, neither the read nor a recollection of its evidence.
    #[test]
    fn a_connection_that_is_not_ready_is_not_selected() {
        let (container, connection, _) = connected(1_000);
        std::thread::sleep(Duration::from_millis(1_500));
        let listed = success(&container.run(&[
            "connections",
            "list",
            "--connector",
            "fixture",
            "--instance",
            "main",
        ]));
        assert_eq!(
            listed["connections"][0]["state"], "pending",
            "precondition: the lapsed connection reads pending: {listed}"
        );
        let validations = container.seen("validations").len();
        let output = container.run(&["invoke", "fixture", READ, "--input-json", "{}"]);
        let answer = answer(&output);
        assert_eq!(
            (
                output.status.code(),
                answer["error"]["data"].clone(),
                container.seen("invocations").len(),
                container.seen("validations").len() - validations,
            ),
            (
                Some(1),
                json!({"kind":"operational","code":"connection_unresolved","stage":"admission",
                    "next_action":"repair_connection",
                    "connections":[{"connection":connection,"state":"pending"}]}),
                0,
                0,
            ),
            "story:connectors-do-short-invocation: a connection that is not ready was used: {answer}"
        );
    }

    /// story:connectors-do-short-invocation, D3: a revoked connection is never
    /// selected; the instance is `connection_unresolved` and nothing reaches the
    /// provider.
    #[test]
    fn a_revoked_connection_is_not_selected() {
        let (container, connection, revision) = connected(50_000);
        success(&container.run(&[
            "connections",
            "revoke",
            "--connector",
            "fixture",
            "--instance",
            "main",
            "--connection",
            &connection,
            "--expected-revision",
            &revision,
        ]));
        let validations = container.seen("validations").len();
        let output = container.run(&["invoke", "fixture", READ, "--input-json", "{}"]);
        let answer = answer(&output);
        assert_eq!(
            (
                output.status.code(),
                &answer["error"]["data"]["code"],
                container.seen("invocations").len(),
                container.seen("validations").len() - validations,
            ),
            (Some(1), &json!("connection_unresolved"), 0, 0),
            "story:connectors-do-short-invocation: {answer}"
        );
    }
}

/// story:connectors-do-short-invocation, D6 and the contract: "`--describe`
/// [...] reads no document, needs no connection and invokes nothing". A
/// definition file's instance that holds no connection is described without a
/// connect question and without connecting: the command has no terminal here,
/// so a question would be a refusal instead of the description.
#[cfg(unix)]
#[test]
#[ignore = "requires CONNECTORS_TEST_CATALOG_PROVIDER (a built catalog provider)"]
fn describe_of_a_definition_needs_no_connection() {
    use sha2::{Digest, Sha256};
    use std::os::unix::process::CommandExt;
    const OPENAPI: &str = r#"{"openapi":"3.0.3","info":{"title":"Fixture forge","version":"1"},
"servers":[{"url":"https://forge.example.com/api/v4"}],
"paths":{
"/user":{"get":{"operationId":"getApiV4User","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"}}}}}}}}},
"/personal_access_tokens/self":{"get":{"operationId":"getApiV4PersonalAccessTokensSelf","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}}}},
"/projects/{id}":{"get":{"operationId":"getApiV4ProjectsId","parameters":[{"name":"id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"}}}}}}}}}
}}"#;
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xffff_ffffu32;
        for &byte in data {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
    fn gzip(data: &[u8]) -> Vec<u8> {
        let len = data.len() as u16;
        let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff, 0x01];
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(data);
        out.extend_from_slice(&crc32(data).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out
    }
    let built = std::path::PathBuf::from(
        std::env::var_os("CONNECTORS_TEST_CATALOG_PROVIDER")
            .expect("CONNECTORS_TEST_CATALOG_PROVIDER names no built catalog provider"),
    );
    let root = tempfile::tempdir().unwrap();
    let init = run(root.path(), &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config = root.path().join("config/config.toml");
    let initialized = fs::read_to_string(&config).unwrap();
    let line = |key: &str| {
        initialized
            .lines()
            .find(|line| line.trim_start().starts_with(key))
            .unwrap()
            .to_owned()
    };
    let socket = root.path().join("no-secret-service.sock");
    fs::write(
        &config,
        format!(
            "{}\n{}\nsecret_service_socket={}\n",
            line("format"),
            line("owner_uid"),
            serde_json::to_string(socket.to_str().unwrap()).unwrap()
        ),
    )
    .unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    let defs = root.path().join("defs");
    fs::create_dir_all(defs.join("connectors")).unwrap();
    fs::create_dir_all(defs.join("upstream/forge")).unwrap();
    fs::write(
        defs.join("upstream/forge/openapi.json.gz"),
        gzip(OPENAPI.as_bytes()),
    )
    .unwrap();
    let bin = root.path().join("bin");
    fs::create_dir(&bin).unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
    let installed = bin.join("connectors-catalog-provider");
    fs::copy(&built, &installed).unwrap();
    fs::set_permissions(&installed, fs::Permissions::from_mode(0o700)).unwrap();
    let definition = defs.join("connectors/forge.yaml");
    fs::write(
        &definition,
        format!(
            "schema: urn:beyond10x:connectors-definition:1\nid: forge\nsource:\n  openapi: upstream/forge/openapi.json.gz\n  sha256: {}\nbase_url: https://{{hostname}}/api/v4\nconfig: {{hostname: {{type: string, default: forge.example.com}}}}\nauth:\n  forge.pat:\n    scheme: header\n    header: PRIVATE-TOKEN\n    label: Fixture personal access token\n    identity: {{path: user, kind: forge.user, subject: /id}}\n    scopes: {{path: personal_access_tokens/self, pointer: /scopes, minimum: [api]}}\noperations:\n  project.get:\n    op: getApiV4ProjectsId\n    description: Read one project\n",
            hex::encode(Sha256::digest(OPENAPI.as_bytes()))
        ),
    )
    .unwrap();
    let mut command = command(
        root.path(),
        &[
            "invoke",
            "forge",
            "project.get",
            "--definition",
            definition.to_str().unwrap(),
            "--describe",
        ],
    );
    command
        .env_remove("DBUS_SESSION_BUS_ADDRESS")
        .env("CONNECTORS_INSTALLATION", &bin)
        .stdin(Stdio::null());
    // SAFETY: the child alone leaves the test's session: it has no
    // controlling terminal to ask a question on.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let output = command.output().unwrap();
    let answer = envelope(&output);
    assert_eq!(
        (
            output.status.code(),
            &answer["result"]["operation"],
            &answer["result"]["effect"]
        ),
        (Some(0), &json!("project.get"), &json!("read")),
        "story:connectors-do-short-invocation: --describe of a definition without a connection: {answer}"
    );
}
