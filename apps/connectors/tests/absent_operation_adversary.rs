//! Adversary cases for story:absent-operation-reports-not-found. Each asserts
//! that describe, invoke and approval prepare give one answer for one id.
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

const READ: &str = "item.read";
const WRITE: &str = "item.write";
const REVISION: &str = "desc-1";

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

/// `(exit, code)`; `code` is `ok` on success.
fn answer(output: &Output) -> (i32, String) {
    let exit = output.status.code().unwrap();
    if exit == 0 {
        return (0, "ok".into());
    }
    let error: Value = serde_json::from_slice(&output.stderr).unwrap_or_else(|_| {
        panic!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    });
    (
        exit,
        error["error"]["data"]["code"].as_str().unwrap().to_owned(),
    )
}

fn bootstrap() -> runtime::Bootstrap {
    let ids = [READ, WRITE];
    let operations = ids
        .iter()
        .map(|id| connectors_core::Operation {
            id: (*id).into(),
            description: "fixture operation".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object","additionalProperties":false,
                "properties":{"name":{"type":"string"}}}),
            output_schema: json!({"type":"object","additionalProperties":false}),
        })
        .collect();
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        revision: REVISION.into(),
        operations,
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
            minimum_scopes: BTreeSet::from(["write".into()]),
            evidence_lifetime_ms: 60_000,
            fields: vec![runtime::EntryField {
                name: "token".into(),
                label: "Token".into(),
                max_bytes: 1024,
            }],
        }],
        requirements: ids
            .iter()
            .map(|id| runtime::Requirement {
                operation: (*id).into(),
                profile: "token".into(),
                scopes: BTreeSet::new(),
                effect: if *id == READ {
                    runtime::Effect::Read
                } else {
                    runtime::Effect::Write
                },
            })
            .collect(),
    }
}

/// Both operations are granted by operation id; the profile grant is `profiles`.
fn configured(profiles: &str) -> (tempfile::TempDir, String, String) {
    let root = tempfile::tempdir().unwrap();
    let init = command(&root, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.forge.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[adapters.forge.permissions]\nprofiles=[{profiles}]\noperations=['{READ}','{WRITE}']\n",
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
    let read = connectors_host::local::owner::schema(&bootstrap, READ).unwrap();
    let write = connectors_host::local::owner::schema(&bootstrap, WRITE).unwrap();
    (root, read, write)
}

fn describe(root: &tempfile::TempDir, operation: &str) -> Output {
    command(
        root,
        &[
            "operations",
            "describe",
            "--adapter",
            "forge",
            "--operation",
            operation,
        ],
    )
}

fn invoke(root: &tempfile::TempDir, operation: &str, schema: &str, revision: &str) -> Output {
    command(
        root,
        &[
            "operations",
            "invoke",
            "--adapter",
            "forge",
            "--connection",
            "conn-1",
            "--operation",
            operation,
            "--schema",
            schema,
            "--revision",
            revision,
            "--input-json",
            r#"{"name":"item"}"#,
        ],
    )
}

fn prepare(root: &tempfile::TempDir, operation: &str, schema: &str, revision: &str) -> Output {
    let input = root.path().join("input.json");
    fs::write(&input, r#"{"name":"item"}"#).unwrap();
    fs::set_permissions(&input, fs::Permissions::from_mode(0o600)).unwrap();
    command(
        root,
        &[
            "approvals",
            "prepare",
            "--adapter",
            "forge",
            "--connection",
            "conn-1",
            "--operation",
            operation,
            "--schema",
            schema,
            "--revision",
            revision,
            "--input-file",
            input.to_str().unwrap(),
        ],
    )
}

/// The operation id is granted but the profile its requirement names is not.
/// `admit_operation` calls that "not granted" and invoke/prepare answer
/// `forbidden`; describe must give the same answer for the same id.
#[test]
fn describe_agrees_with_invoke_when_the_operation_profile_is_not_granted() {
    let (root, read, write) = configured("'other'");
    let invoked = answer(&invoke(&root, READ, &read, REVISION));
    let prepared = answer(&prepare(&root, WRITE, &write, REVISION));
    assert_eq!(invoked, (1, "forbidden".into()), "invoke");
    assert_eq!(prepared, (1, "forbidden".into()), "prepare");
    assert_eq!(answer(&describe(&root, READ)), invoked, "describe {READ}");
    assert_eq!(
        answer(&describe(&root, WRITE)),
        prepared,
        "describe {WRITE}"
    );
}

/// Existence is decided before the grant and before the revision even when the
/// profile is the missing grant: an absent id is `not_found` everywhere.
#[test]
fn absent_id_is_not_found_everywhere_when_the_profile_is_not_granted() {
    let (root, _, _) = configured("'other'");
    let expected = (1, "not_found".to_owned());
    assert_eq!(answer(&describe(&root, "nosuch.op")), expected, "describe");
    assert_eq!(
        answer(&invoke(
            &root,
            "nosuch.op",
            "stale-schema",
            "stale-revision"
        )),
        expected,
        "invoke"
    );
    assert_eq!(
        answer(&prepare(
            &root,
            "nosuch.op",
            "stale-schema",
            "stale-revision"
        )),
        expected,
        "prepare"
    );
}

/// One id that is not a well-formed identifier gets one answer from describe,
/// invoke and prepare.
#[test]
fn an_ill_formed_operation_id_gets_one_answer_from_every_verb() {
    let (root, _, _) = configured("'token'");
    for id in ["No Such", "nosuch/op"] {
        let described = answer(&describe(&root, id));
        let invoked = answer(&invoke(&root, id, "stale-schema", "stale-revision"));
        let prepared = answer(&prepare(&root, id, "stale-schema", "stale-revision"));
        assert_eq!(described, invoked, "describe vs invoke for {id:?}");
        assert_eq!(invoked, prepared, "invoke vs prepare for {id:?}");
    }
}
