//! Adversary cases for story:cli-connector-instance-selectors, driven from the
//! unit's own contract text in `contracts/cli/v1alpha1/semantics.md`
//! §3 "Selecting an instance":
//!
//! > Every result type that carries `adapter` carries `connector` and
//! > `instance` beside it, all three required.
//!
//! The contract exempts one shape, the service descriptor `adapters describe`
//! returns (commit eca9692bd); every other result object is held here.
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
const OPERATION: &str = "item.read";
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
            effect: runtime::Effect::Read,
        }],
    }
}

/// One `connectors-local/5` instance, `fixture/main`, legacy alias `forge`,
/// with a cached description, as `connector_instance_selectors.rs` builds it.
fn configured() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = command(root.path(), &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config_path = root.path().join("config/config.toml");
    let initialized = fs::read_to_string(&config_path).unwrap();
    let owner = initialized
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

/// Every object in `value` that carries `adapter` but not both `connector`
/// and `instance`, by JSON pointer. The service descriptor (the object
/// carrying `configuration_schema`) is the one shape the contract exempts
/// (semantics.md §3, commit eca9692bd): its `adapter` is the implementation
/// and its `instance` the instance id, and renaming them belongs to
/// story:cli-text-without-adapter.
fn unnamed(value: &Value, pointer: String, found: &mut Vec<String>) {
    match value {
        Value::Object(fields) => {
            if fields.contains_key("adapter")
                && !fields.contains_key("configuration_schema")
                && !(fields.contains_key("connector") && fields.contains_key("instance"))
            {
                found.push(format!("{pointer}: {}", Value::Object(fields.clone())));
            }
            for (key, field) in fields {
                unnamed(field, format!("{pointer}/{key}"), found);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                unnamed(item, format!("{pointer}/{index}"), found);
            }
        }
        _ => {}
    }
}

/// semantics.md §3: "Every result type that carries `adapter` carries
/// `connector` and `instance` beside it". `adapters describe` with a cached
/// description answers a `descriptor` object carrying `adapter` (the
/// implementation id, `catalog`) and `instance` (the instance id,
/// `forge-local`) and no `connector`, inside the same result whose summary
/// names `adapter` `forge` and `instance` `main`: one envelope, two meanings
/// for each of the two names the contract just defined.
#[test]
fn every_result_object_carrying_adapter_names_its_connector_and_instance() {
    let root = configured();
    let output = command(
        root.path(),
        &[
            "adapters",
            "describe",
            "--connector",
            CONNECTOR,
            "--instance",
            INSTANCE,
        ],
    );
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    let result = &envelope["result"];
    assert_eq!(result["source"], "cached", "{result}");
    let mut found = Vec::new();
    unnamed(result, String::new(), &mut found);
    assert!(
        found.is_empty(),
        "result objects carrying `adapter` without `connector` and `instance`:\n{}",
        found.join("\n")
    );
}
