//! beyond10x/connectors#105: `operations describe` and `adapters describe`
//! answer each operation's input and output schema as a JSON object, not as a
//! string holding JSON text. The schema digest stays the operation's identity.
//! No adapter executable exists and none is ever launched.
use connectors_host::local::{
    config::{Config, Paths},
    registry, runtime,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    process::{Command, Output},
};

const READ: &str = "item.read";

fn input_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"name":{"type":"string"}},"required":["name"]})
}
fn output_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"id":{"type":"integer"}}})
}

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

fn success(output: &Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice::<Value>(&output.stdout).unwrap()["result"].clone()
}

fn bootstrap() -> runtime::Bootstrap {
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        revision: "desc-1".into(),
        operations: vec![connectors_core::Operation {
            id: READ.into(),
            description: "fixture operation".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: input_schema(),
            output_schema: output_schema(),
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
            operation: READ.into(),
            profile: "token".into(),
            scopes: BTreeSet::new(),
            effect: runtime::Effect::Read,
        }],
    }
}

/// One configured adapter with a cached description of one granted read.
fn configured() -> (tempfile::TempDir, String) {
    let root = tempfile::tempdir().unwrap();
    let init = command(&root, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.forge.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[adapters.forge.permissions]\nprofiles=['token']\noperations=['{READ}']\n",
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
    let schema = connectors_host::local::owner::schema(&bootstrap, READ).unwrap();
    (root, schema)
}

#[test]
fn operations_describe_answers_schemas_as_json_objects() {
    let (root, schema) = configured();
    let result = success(&command(
        &root,
        &[
            "operations",
            "describe",
            "--adapter",
            "forge",
            "--operation",
            READ,
        ],
    ));
    assert_eq!(result["operation"]["input_schema"], input_schema());
    assert_eq!(result["operation"]["output_schema"], output_schema());
    assert_eq!(result["schema"], schema);
}

#[test]
fn adapters_describe_answers_operation_schemas_as_json_objects() {
    let (root, _) = configured();
    let result = success(&command(&root, &["adapters", "describe", "--adapter", "forge"]));
    let operations = result["descriptor"]["operations"].as_array().unwrap();
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0]["input_schema"], input_schema());
    assert_eq!(operations[0]["output_schema"], output_schema());
}
