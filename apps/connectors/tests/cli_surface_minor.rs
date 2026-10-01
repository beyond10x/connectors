//! Minor CLI surface findings from the 0.18.0 black-box test
//! (story:cli-surface-minor-findings-0-18-0): the `metadata` prerequisite's
//! next action, the stage of an unknown adapter alias, and the `ess-cli/1`
//! presentation statements of `contracts/cli/v1alpha1/semantics.md` that the
//! binary can show (parser codes, raw completions, C05 wrong input).
use connectors_host::local::{
    config::{Config, Paths},
    registry, runtime,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Output},
};

const REVISION: &str = "desc-1";
const OPERATION: &str = "item.read";
const UUID: &str = "0fd9de39-aed4-4673-8330-12f20cd0b962";

fn command(root: &tempfile::TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.path().join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.path().join("state"))
        .args(args)
        .output()
        .unwrap()
}

fn initialized() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = command(&root, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    root
}

fn stderr_envelope(output: &Output, exit: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "{output:?}");
    serde_json::from_slice(&output.stderr).unwrap()
}

/// D6a: a `ready` metadata prerequisite has nothing to do next.
#[test]
fn setup_check_reports_no_next_action_for_ready_metadata() {
    let root = initialized();
    let output = command(&root, &["setup", "check"]);
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let metadata = value["result"]["prerequisites"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "metadata")
        .unwrap()
        .clone();
    assert_eq!(metadata["state"], "ready", "{metadata}");
    assert_eq!(metadata["next_action"], "none", "{metadata}");
}

/// D7: every command that selects an adapter answers an unknown alias with
/// `not_found` at `stage = admission` and `next_action = check_configuration`
/// (semantics.md: a refusal made before any provider request).
#[test]
fn every_adapter_command_answers_an_unknown_alias_at_admission() {
    let root = initialized();
    let input = root.path().join("input.json");
    fs::write(&input, r#"{"operations":[]}"#).unwrap();
    fs::set_permissions(&input, fs::Permissions::from_mode(0o600)).unwrap();
    let credential = root.path().join("credential");
    fs::write(&credential, "x").unwrap();
    fs::set_permissions(&credential, fs::Permissions::from_mode(0o600)).unwrap();
    let proof = root.path().join("proof");
    let (input, credential, proof) = (
        input.to_str().unwrap(),
        credential.to_str().unwrap(),
        proof.to_str().unwrap(),
    );
    let selected = [
        "--connection",
        "c",
        "--operation",
        "o",
        "--schema",
        "s",
        "--revision",
        "r",
        "--input-json",
        "{}",
    ];
    let commands: Vec<Vec<&str>> = vec![
        vec!["approvals", "policy-status"],
        vec![
            "approvals",
            "policy-set",
            "--expected-revision",
            "1",
            "--input-file",
            input,
        ],
        [&["approvals", "prepare"][..], &selected].concat(),
        [
            &["approvals", "issue"][..],
            &selected,
            &["--approve-subject", "x", "--proof-output", proof],
        ]
        .concat(),
        vec!["approvals", "clock-check"],
        vec!["approvals", "key-status"],
        vec!["approvals", "key-init", "--expected-revision", UUID],
        vec![
            "approvals",
            "key-rotate",
            "--expected-revision",
            UUID,
            "--expected-key",
            UUID,
        ],
        vec![
            "approvals",
            "key-recover",
            "--expected-revision",
            UUID,
            "--candidate",
            UUID,
        ],
        vec![
            "approvals",
            "key-revoke",
            "--expected-revision",
            UUID,
            "--expected-key",
            UUID,
        ],
        vec![
            "approvals",
            "key-retire",
            "--expected-revision",
            UUID,
            "--key",
            UUID,
        ],
        vec!["adapters", "describe"],
        vec!["adapters", "status"],
        vec![
            "adapters",
            "stop",
            "--expected-revision",
            "1",
            "--host-incarnation",
            "1",
            "--child-incarnation",
            "1",
        ],
        vec!["connections", "list"],
        vec!["connections", "describe", "--connection", "c"],
        vec![
            "connections",
            "connect",
            "--profile",
            "p",
            "--credential-file",
            credential,
        ],
        vec![
            "connections",
            "repair",
            "--connection",
            "c",
            "--expected-revision",
            "1",
            "--credential-file",
            credential,
        ],
        vec!["connections", "status", "--connection", "c"],
        vec![
            "connections",
            "revalidate",
            "--connection",
            "c",
            "--expected-revision",
            "1",
        ],
        vec![
            "connections",
            "revoke",
            "--connection",
            "c",
            "--expected-revision",
            "1",
        ],
        vec!["operations", "list"],
        vec!["operations", "describe", "--operation", "o"],
        [&["operations", "invoke"][..], &selected].concat(),
    ];
    let mut wrong = Vec::new();
    for args in &commands {
        let output = command(&root, &[&args[..], &["--adapter", "nosuch"]].concat());
        let envelope: Value = serde_json::from_slice(&output.stderr).unwrap_or_default();
        let data = &envelope["error"]["data"];
        if output.status.code() != Some(1)
            || !output.stdout.is_empty()
            || envelope["error"]["code"] != "failure"
            || data["code"] != "not_found"
            || data["stage"] != "admission"
            || data["next_action"] != "check_configuration"
        {
            wrong.push(format!("{}: {envelope}", args[..2].join(" ")));
        }
    }
    assert_eq!(commands.len(), 24);
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Parser failures carry the `ess-cli/1` code `cli_parse` with empty data and
/// exit 2, never an application `invalid_input` Failure.
#[test]
fn an_unknown_flag_or_command_is_cli_parse() {
    let root = initialized();
    for args in [
        &["adapters", "list", "--no-such-flag"][..],
        &["no-such-command"][..],
    ] {
        let envelope = stderr_envelope(&command(&root, args), 2);
        assert_eq!(
            envelope,
            json!({"ok":false,"error":{"code":"cli_parse","data":{}}}),
            "{args:?}"
        );
    }
}

/// A wrong CLI field type is `cli_input` with empty data and exit 2.
#[test]
fn a_wrong_argument_type_is_cli_input() {
    let root = initialized();
    let envelope = stderr_envelope(&command(&root, &["adapters", "list", "--limit", "many"]), 2);
    assert_eq!(
        envelope,
        json!({"ok":false,"error":{"code":"cli_input","data":{}}})
    );
}

/// `completions` emits the raw completion script whatever `--output` says.
#[test]
fn completions_emit_the_raw_script_under_json_output() {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["completions", "bash", "--output", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(serde_json::from_str::<Value>(&stdout).is_err(), "{stdout}");
    assert!(stdout.starts_with("_connectors()"), "{stdout}");
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
                "properties":{"name":{"type":"string"}}}),
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

/// One configured adapter with a cached, granted description. No adapter
/// executable exists and none is ever launched.
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

/// C05 wrong input: every business-JSON defect is refused with exit 2, empty
/// stdout and no dispatch. The ones the CLI's own decoder refuses carry the
/// `ess-cli/1` code `cli_dynamic_input` with empty data; a decoded document the
/// owner's schema check refuses is the application Failure `invalid_input`.
#[test]
fn c05_wrong_business_input_codes() {
    let (root, schema) = configured();
    let file = root.path().join("business.json");
    let deep = format!("{{\"name\":{}0{}}}", "[".repeat(70), "]".repeat(70));
    let oversized = format!("{{\"name\":\"{}\"}}", "a".repeat(1024 * 1024 + 1));
    let mut seen = Vec::new();
    for (case, document) in [
        ("malformed", "{\"name\":"),
        ("trailing", "{} {}"),
        ("duplicate", r#"{"name":"a","name":"b"}"#),
        ("wrong field", r#"{"other":"a"}"#),
        ("wrong type", r#"{"name":1}"#),
        ("depth", deep.as_str()),
        ("bytes", oversized.as_str()),
    ] {
        fs::write(&file, document).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
        let output = command(
            &root,
            &[
                "operations",
                "invoke",
                "--adapter",
                "forge",
                "--connection",
                "conn-1",
                "--operation",
                OPERATION,
                "--schema",
                &schema,
                "--revision",
                REVISION,
                "--input-file",
                file.to_str().unwrap(),
            ],
        );
        let envelope = stderr_envelope(&output, 2);
        let error = &envelope["error"];
        let code = if error["code"] == "failure" {
            format!("failure/{}", error["data"]["code"].as_str().unwrap())
        } else {
            assert_eq!(error["data"], json!({}), "{case}");
            error["code"].as_str().unwrap().to_owned()
        };
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("name"),
            "{case}"
        );
        seen.push((case, code));
    }
    let decoder = "cli_dynamic_input".to_owned();
    let owner = "failure/invalid_input".to_owned();
    assert_eq!(
        seen,
        [
            ("malformed", decoder.clone()),
            ("trailing", decoder),
            ("duplicate", owner.clone()),
            ("wrong field", owner.clone()),
            ("wrong type", owner.clone()),
            ("depth", owner.clone()),
            ("bytes", owner),
        ]
    );
}

/// Business-input sources: two sources for one field is `cli_parse`; a selected
/// file the owner's reader refuses is the owner's `invalid_input` Failure, which
/// takes the place of the presentation's `cli_source`.
#[test]
fn c05_business_input_sources() {
    let (root, schema) = configured();
    let base = [
        "operations",
        "invoke",
        "--adapter",
        "forge",
        "--connection",
        "conn-1",
        "--operation",
        OPERATION,
        "--schema",
        &schema,
        "--revision",
        REVISION,
    ];
    let collision = command(
        &root,
        &[&base[..], &["--input-json", "{}", "--input-stdin"]].concat(),
    );
    assert_eq!(
        stderr_envelope(&collision, 2),
        json!({"ok":false,"error":{"code":"cli_parse","data":{}}})
    );
    let missing = root.path().join("absent.json");
    let unreadable = command(
        &root,
        &[&base[..], &["--input-file", missing.to_str().unwrap()]].concat(),
    );
    let envelope = stderr_envelope(&unreadable, 2);
    assert_eq!(envelope["error"]["code"], "failure", "{envelope}");
    assert_eq!(
        envelope["error"]["data"]["code"], "invalid_input",
        "{envelope}"
    );
    assert_eq!(envelope["error"]["data"]["kind"], "usage", "{envelope}");
}

fn code(output: &Output) -> String {
    let envelope: Value = serde_json::from_slice(&output.stderr).unwrap_or_default();
    let error = &envelope["error"];
    let code = if error["code"] == "failure" {
        format!("failure/{}", error["data"]["code"].as_str().unwrap_or("?"))
    } else {
        assert_eq!(error["data"], json!({}), "{envelope}");
        error["code"].as_str().unwrap_or("?").to_owned()
    };
    format!("{}/{}", output.status.code().unwrap(), code)
}

fn invoke_args<'a>(verb: &[&'a str], alias: &'a str, schema: &'a str) -> Vec<&'a str> {
    [
        verb,
        &[
            "--adapter",
            alias,
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

/// `approvals prepare` and `approvals issue` hand their document to the owner
/// undecoded: an undecodable or trailing document is the owner's
/// `invalid_input`, not `cli_dynamic_input`.
#[test]
fn approvals_refuse_an_undecodable_document_with_owner_invalid_input() {
    let (root, schema) = configured();
    let proof = root.path().join("proof");
    let mut seen = Vec::new();
    for verb in ["prepare", "issue"] {
        for document in ["{\"name\":", "{} {}"] {
            let mut args = invoke_args(&["approvals", verb], "forge", &schema);
            args.extend(["--input-json", document]);
            if verb == "issue" {
                args.extend(["--approve-subject", "x", "--proof-output"]);
                args.push(proof.to_str().unwrap());
            }
            seen.push((verb, document, code(&command(&root, &args))));
        }
    }
    let owner = "2/failure/invalid_input".to_owned();
    assert_eq!(
        seen,
        [
            ("prepare", "{\"name\":", owner.clone()),
            ("prepare", "{} {}", owner.clone()),
            ("issue", "{\"name\":", owner.clone()),
            ("issue", "{} {}", owner),
        ]
    );
}

/// C05 depth: the decoder's recursion limit (128 nesting levels counting the
/// outermost value) answers `cli_dynamic_input` before the owner's depth
/// bound; below it a depth excess is the owner's `invalid_input`.
#[test]
fn c05_depth_past_the_decoder_limit_is_cli_dynamic_input() {
    let (root, schema) = configured();
    let file = root.path().join("business.json");
    let mut seen = Vec::new();
    // Levels counting the outer object are arrays + 1.
    for arrays in [65, 126, 127, 200] {
        let document = format!("{{\"name\":{}0{}}}", "[".repeat(arrays), "]".repeat(arrays));
        fs::write(&file, document).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
        let mut args = invoke_args(&["operations", "invoke"], "forge", &schema);
        args.extend(["--input-file", file.to_str().unwrap()]);
        seen.push((arrays + 1, code(&command(&root, &args))));
    }
    let owner = "2/failure/invalid_input".to_owned();
    let decoder = "2/cli_dynamic_input".to_owned();
    assert_eq!(
        seen,
        [
            (66, owner.clone()),
            (127, owner),
            (128, decoder.clone()),
            (201, decoder),
        ]
    );
}

/// Precedence: the four document-taking commands read their document before
/// admitting the alias; every other command answers `not_found` at admission.
#[test]
fn document_commands_refuse_the_document_before_an_unknown_alias() {
    let root = initialized();
    let absent = root.path().join("absent.json");
    let absent = absent.to_str().unwrap();
    let proof = root.path().join("proof");
    let proof = proof.to_str().unwrap();
    let with = |verb: &[&'static str], source: &[&'static str]| -> Vec<String> {
        let mut args: Vec<String> = invoke_args(verb, "nosuch", "s")
            .into_iter()
            .map(String::from)
            .collect();
        args.extend(source.iter().map(|s| s.to_string()));
        if verb[1] == "issue" {
            args.extend(["--approve-subject", "x", "--proof-output", proof].map(String::from));
        }
        args
    };
    let run = |args: Vec<String>| {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        code(&command(&root, &args))
    };
    let mut seen = Vec::new();
    for verb in [
        &["operations", "invoke"][..],
        &["approvals", "prepare"][..],
        &["approvals", "issue"][..],
    ] {
        let mut args = with(verb, &[]);
        args.extend(["--input-file".to_owned(), absent.to_owned()]);
        seen.push((verb[1], "absent", run(args)));
        seen.push((
            verb[1],
            "malformed",
            run(with(verb, &["--input-json", "{\"name\":"])),
        ));
        seen.push((
            verb[1],
            "readable",
            run(with(verb, &["--input-json", "{}"])),
        ));
    }
    seen.push((
        "policy-set",
        "absent",
        run([
            "approvals",
            "policy-set",
            "--adapter",
            "nosuch",
            "--expected-revision",
            "1",
            "--input-file",
            absent,
        ]
        .map(String::from)
        .to_vec()),
    ));
    let owner = "2/failure/invalid_input".to_owned();
    let absent_alias = "1/failure/not_found".to_owned();
    assert_eq!(
        seen,
        [
            ("invoke", "absent", owner.clone()),
            ("invoke", "malformed", "2/cli_dynamic_input".to_owned()),
            ("invoke", "readable", absent_alias.clone()),
            ("prepare", "absent", owner.clone()),
            ("prepare", "malformed", owner.clone()),
            ("prepare", "readable", absent_alias.clone()),
            ("issue", "absent", owner.clone()),
            ("issue", "malformed", owner.clone()),
            ("issue", "readable", absent_alias),
            ("policy-set", "absent", owner),
        ]
    );
}

/// Selectors are bounded at 128 bytes: a 129-byte alias is
/// `invalid_configuration`, a 128-byte one is admitted.
#[test]
fn a_configured_alias_is_bounded_at_128_bytes() {
    let mut seen = Vec::new();
    for length in [128, 129] {
        let root = initialized();
        let config_path = root.path().join("config/config.toml");
        let alias = "a".repeat(length);
        let config = fs::read_to_string(&config_path).unwrap()
            + &format!(
                "\n[adapters.{alias}]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.{alias}.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[adapters.{alias}.permissions]\nprofiles=['token']\noperations=['{OPERATION}']\n",
                "a".repeat(64)
            );
        fs::write(&config_path, config).unwrap();
        let output = command(&root, &["adapters", "list"]);
        seen.push((
            length,
            if output.status.success() {
                "0".to_owned()
            } else {
                code(&output)
            },
        ));
    }
    assert_eq!(
        seen,
        [
            (128, "0".to_owned()),
            (129, "2/failure/invalid_configuration".to_owned()),
        ]
    );
}
