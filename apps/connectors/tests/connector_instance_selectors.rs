//! story:cli-connector-instance-selectors: every command that takes `--adapter`
//! also takes `--connector` and `--instance`, both select the same instance, and
//! `--adapter` still works for the deprecation window and says so exactly once
//! (`contracts/cli/v1alpha1/semantics.md` §3 "Selecting an instance").
use connectors_cli_contract::wire::{ArgumentSource, Shape};
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
    process::{Command, Output},
};

/// A directory holding no executable (`cli_surface_minor.rs`).
const NOT_INSTALLED: &str = if cfg!(windows) {
    "C:/not-installed"
} else {
    "/not-installed"
};
const REVISION: &str = "desc-1";
const OPERATION: &str = "item.read";
const UUID: &str = "0fd9de39-aed4-4673-8330-12f20cd0b962";
/// The configured instance: connector `fixture`, instance `main`, legacy alias `forge`.
const CONNECTOR: &str = "fixture";
const INSTANCE: &str = "main";
const ALIAS: &str = "forge";

fn command(root: &Path, output: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", output, "--config"])
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
            input_schema: json!({"type":"object","additionalProperties":false,
                "properties":{"name":{"type":"string"}}}),
            output_schema: json!({"type":"object","additionalProperties":false}),
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
            effect: runtime::Effect::Read,
        }],
    }
}

/// A `connectors-local/5` configuration whose one instance, `fixture/main`,
/// carries the pins of the legacy entry `forge`, plus `extra` top-level keys,
/// with a cached, granted description. No executable exists and none is
/// launched. Returns the root and the operation's schema identity.
fn configured(extra: &str) -> (tempfile::TempDir, String) {
    let root = tempfile::tempdir().unwrap();
    let init = command(root.path(), "json", &["setup", "init"]);
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
            "format = \"connectors-local/5\"\n{owner}\n{extra}[connectors.{CONNECTOR}]\nenabled=true\n[{table}]\nalias='{ALIAS}'\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[{table}.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[{table}.permissions]\nprofiles=['token']\noperations=['{OPERATION}']\n",
            "a".repeat(64)
        ),
    )
    .unwrap();
    let paths = Paths {
        config: config_path,
        state: root.path().join("state"),
    };
    let adapter = Config::load(&paths.config).unwrap().adapters[ALIAS].clone();
    let bootstrap = bootstrap();
    runtime::state::State::new(&paths.state)
        .remember(&adapter.selection(), &bootstrap)
        .unwrap();
    let schema = connectors_host::local::owner::schema(&bootstrap, OPERATION).unwrap();
    (root, schema)
}

fn private_file(path: &Path, text: &str) -> String {
    fs::write(path, text).unwrap();
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    path.to_str().unwrap().to_owned()
}

/// Every command path of `apps/connectors/spec/cli.yaml` that takes `--adapter`,
/// with a value for each other required argument.
fn selecting_commands(root: &Path, schema: &str) -> Vec<(String, Vec<String>)> {
    let plan = connectors_cli_contract::plan();
    let policy = private_file(&root.join("policy.json"), r#"{"operations":[]}"#);
    let credential = private_file(&root.join("credential"), "x");
    let mut commands = Vec::new();
    for declaration in &plan.commands {
        let callable = &plan.callables[&declaration.callable];
        let Some(Shape::Struct { fields }) = callable.input.as_ref().map(|input| &input.shape)
        else {
            continue;
        };
        if !declaration.arguments.iter().any(|argument| {
            matches!(&argument.source, ArgumentSource::Option { long } if long == "adapter")
        }) {
            continue;
        }
        let mut args: Vec<String> = declaration.path.clone();
        for argument in &declaration.arguments {
            let field = argument.field.as_str();
            if matches!(field, "adapter" | "connector" | "instance") {
                continue;
            }
            let value = match field {
                "credential_document" => {
                    args.extend(["--credential-file".into(), credential.clone()]);
                    continue;
                }
                "input" => {
                    args.extend(["--input-json".into(), r#"{"name":"x"}"#.into()]);
                    continue;
                }
                "input_file" => policy.clone(),
                // `connections status` takes it optionally.
                "connection" => "conn-1".into(),
                _ if fields[field].optional() => continue,
                "operation" => OPERATION.into(),
                "schema" => schema.into(),
                "revision" => REVISION.into(),
                "profile" => "token".into(),
                "consumer" => "probe".into(),
                "approve_subject" => "x".into(),
                "proof_output" => root.join("proof").to_str().unwrap().into(),
                "expected_revision" if declaration.callable.starts_with("approval-keys-") => {
                    UUID.into()
                }
                "expected_key" | "candidate" | "key" => UUID.into(),
                _ => "1".into(),
            };
            let ArgumentSource::Option { long } = &argument.source else {
                panic!("{field}: no option source");
            };
            args.extend([format!("--{long}"), value]);
        }
        commands.push((declaration.callable.clone(), args));
    }
    commands
}

/// The envelope without what is not the selection's: observation times, and
/// the identifiers and key a command mints at random (`approvals key-init`
/// creates an issuer and a key whose ids and public key differ every time).
fn steady(mut value: Value) -> Value {
    fn walk(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                for (key, field) in fields.iter_mut() {
                    if key.ends_with("_ms") && field.is_i64() {
                        *field = json!(0);
                    } else if key == "public_key" {
                        *field = json!("<minted>");
                    } else {
                        walk(field);
                    }
                }
            }
            Value::String(text) => {
                if let Some((head, tail)) = text.rsplit_once('/').or(Some(("", text.as_str())))
                    && uuid::Uuid::parse_str(tail).is_ok()
                {
                    *text = format!("{head}/<minted>");
                }
            }
            Value::Array(items) => items.iter_mut().for_each(walk),
            _ => {}
        }
    }
    walk(&mut value);
    value
}

/// Whether every struct of the shape that carries `adapter` requires
/// `connector` and `instance` beside it. A service descriptor's `adapter` is
/// the implementation it describes, not a selector, and is left out.
fn names_selection(shape: &Shape) -> bool {
    match shape.required() {
        Shape::Struct { fields } => {
            (!fields.contains_key("adapter")
                || fields.contains_key("configuration_schema")
                || ["adapter", "connector", "instance"]
                    .iter()
                    .all(|name| fields.get(*name) == Some(&Shape::String)))
                && fields.values().all(names_selection)
        }
        Shape::List { of } => names_selection(of),
        Shape::Map { value } => names_selection(value),
        _ => true,
    }
}

/// One parsed envelope from an output: the success envelope on stdout with an
/// empty standard error, or exactly one failure envelope on standard error
/// with an empty stdout.
fn envelope(output: &Output) -> Value {
    let (stdout, stderr) = (
        String::from_utf8(output.stdout.clone()).unwrap(),
        String::from_utf8(output.stderr.clone()).unwrap(),
    );
    let (text, other) = if output.status.success() {
        (stdout, stderr)
    } else {
        (stderr, stdout)
    };
    assert!(other.is_empty(), "{output:?}");
    assert_eq!(text.lines().count(), 1, "{text}");
    serde_json::from_str(&text).unwrap()
}

/// The acceptance: each command path that takes `--adapter` runs twice against
/// one instance, with `--connector --instance` and with `--adapter <its legacy
/// alias>`. Every result validates against the generated result type, which
/// requires `adapter`, `connector` and `instance` wherever it carries a
/// selector, and the second run differs only by one `deprecations` entry.
#[test]
fn both_selectors_reach_one_instance_and_only_the_alias_is_deprecated() {
    let plan = connectors_cli_contract::plan();
    let (root, schema) = configured("");
    let count = selecting_commands(root.path(), &schema).len();
    // `operations invoke` is removed; `invoke` selects by its target
    // (story:connectors-do-short-invocation).
    assert_eq!(count, 25);
    let (mut succeeded, mut wrong) = (Vec::new(), Vec::new());
    for index in 0..count {
        // Each run starts from the same configured state, so a command that
        // writes (a key, a policy) answers the second run as it did the first.
        let run = |selector: &[&str]| {
            let (root, schema) = configured("");
            let (callable, args) = selecting_commands(root.path(), &schema).remove(index);
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let output = command(root.path(), "json", &[&args[..], selector].concat());
            // `connections connect` starts this state directory's owner.
            let stopped = command(root.path(), "json", &["setup", "owner-stop"]);
            assert!(stopped.status.success(), "{stopped:?}");
            (callable, output)
        };
        let (callable, by_instance) = run(&["--connector", CONNECTOR, "--instance", INSTANCE]);
        let (_, by_alias) = run(&["--adapter", ALIAS]);
        let callable = &callable;
        let result_shape = &plan.callables[callable].result.shape;
        assert!(names_selection(result_shape), "{callable}");
        assert_eq!(
            by_instance.status.code(),
            by_alias.status.code(),
            "{callable}"
        );
        let first = envelope(&by_instance);
        let mut second = envelope(&by_alias);
        assert!(first.get("deprecations").is_none(), "{callable}: {first}");
        let deprecations = second
            .as_object_mut()
            .unwrap()
            .remove("deprecations")
            .unwrap_or_else(|| panic!("{callable}: no deprecations"));
        assert_eq!(
            deprecations,
            json!([{"option":"--adapter","replacement":["--connector","--instance"]}]),
            "{callable}"
        );
        if steady(first.clone()) != steady(second.clone()) {
            wrong.push(format!("{callable}:\n  {first}\n  {second}"));
        }
        if first["ok"] == true {
            for result in [&first["result"], &second["result"]] {
                assert!(result_shape.accepts(result), "{callable}: {result}");
            }
            succeeded.push(callable.clone());
        }
        println!(
            "{callable}: {}",
            if first["ok"] == true {
                "ok".to_owned()
            } else {
                format!("{}", first["error"]["data"]["code"])
            }
        );
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(succeeded.len() >= 6, "{succeeded:?}");
}

/// Every result that names the selected alias names its connector and instance
/// too, and the alias is the instance's legacy alias.
#[test]
fn a_result_names_the_connector_instance_and_legacy_alias() {
    let (root, _) = configured("");
    let output = command(
        root.path(),
        "json",
        &[
            "instances",
            "describe",
            "--connector",
            CONNECTOR,
            "--instance",
            INSTANCE,
        ],
    );
    let summary = envelope(&output)["result"]["configuration"]["summary"].clone();
    assert_eq!(
        (
            &summary["adapter"],
            &summary["connector"],
            &summary["instance"]
        ),
        (&json!(ALIAS), &json!(CONNECTOR), &json!(INSTANCE)),
        "{summary}"
    );
    let output = command(root.path(), "json", &["instances", "list"]);
    let listed = envelope(&output)["result"]["adapters"].clone();
    assert_eq!(listed[0]["connector"], CONNECTOR, "{listed}");
    assert_eq!(listed[0]["instance"], INSTANCE, "{listed}");
}

/// Exactly one of `--adapter` or `--connector` with `--instance` selects; an
/// instance name is unique only within its connector, so `--instance` alone
/// selects nothing. Neither, with no default configured, selects nothing.
#[test]
fn a_selector_that_names_no_single_instance_is_refused() {
    let (root, _) = configured("");
    let refused = |args: &[&str]| {
        let output = command(
            root.path(),
            "json",
            &[&["approvals", "policy-status"][..], args].concat(),
        );
        let envelope = envelope(&output);
        (
            output.status.code(),
            envelope["error"]["data"]["code"].clone(),
            envelope["error"]["data"]["stage"].clone(),
        )
    };
    let usage = (Some(2), json!("invalid_input"), json!("arguments"));
    assert_eq!(refused(&["--instance", INSTANCE]), usage);
    assert_eq!(refused(&["--connector", CONNECTOR]), usage);
    assert_eq!(
        refused(&[
            "--adapter",
            ALIAS,
            "--connector",
            CONNECTOR,
            "--instance",
            INSTANCE
        ]),
        usage
    );
    assert_eq!(refused(&[]), usage);
    let missing = (Some(1), json!("not_found"), json!("admission"));
    assert_eq!(
        refused(&["--connector", CONNECTOR, "--instance", "other"]),
        missing
    );
    assert_eq!(
        refused(&["--connector", "other", "--instance", INSTANCE]),
        missing
    );
}

/// `default_instance = "<connector>/<instance>"` selects when no selector is
/// given, as `default_adapter` does.
#[test]
fn default_instance_selects_when_no_selector_is_given() {
    for default in [
        format!("default_instance = \"{CONNECTOR}/{INSTANCE}\"\n"),
        format!("default_adapter = \"{ALIAS}\"\n"),
    ] {
        let (root, _) = configured(&default);
        let output = command(root.path(), "json", &["approvals", "policy-status"]);
        let result = envelope(&output)["result"].clone();
        assert_eq!(
            (
                &result["adapter"],
                &result["connector"],
                &result["instance"]
            ),
            (&json!(ALIAS), &json!(CONNECTOR), &json!(INSTANCE)),
            "{default}: {result}"
        );
    }
}

/// Under `--output human`, `--adapter` adds exactly one line on standard error
/// and changes nothing else; a failure keeps its one envelope under
/// `--output json`, so a consumer reading all of standard error as one JSON
/// envelope still reads its code.
#[test]
fn the_deprecation_is_one_human_line_or_one_json_member() {
    let (root, _) = configured("");
    let args = ["approvals", "policy-status"];
    let by_instance = command(
        root.path(),
        "human",
        &[
            &args[..],
            &["--connector", CONNECTOR, "--instance", INSTANCE],
        ]
        .concat(),
    );
    let by_alias = command(
        root.path(),
        "human",
        &[&args[..], &["--adapter", ALIAS]].concat(),
    );
    assert!(by_instance.status.success(), "{by_instance:?}");
    assert!(by_instance.stderr.is_empty(), "{by_instance:?}");
    assert_eq!(by_alias.status.code(), Some(0), "{by_alias:?}");
    assert_eq!(by_alias.stdout, by_instance.stdout);
    let stderr = String::from_utf8(by_alias.stderr).unwrap();
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.starts_with("deprecated: --adapter"), "{stderr}");
    assert!(
        stderr.contains("--connector") && stderr.contains("--instance"),
        "{stderr}"
    );

    let failed = command(
        root.path(),
        "json",
        &[
            "operations",
            "describe",
            "--adapter",
            ALIAS,
            "--operation",
            "absent",
        ],
    );
    assert_eq!(failed.status.code(), Some(1), "{failed:?}");
    let envelope = envelope(&failed);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "failure");
    assert_eq!(envelope["deprecations"][0]["option"], "--adapter");
    let parse = command(
        root.path(),
        "json",
        &["approvals", "policy-status", "--adapter"],
    );
    let refused = envelope_of_parse(&parse);
    assert_eq!(refused["error"]["code"], "cli_parse");
    assert_eq!(refused["deprecations"][0]["option"], "--adapter");
}

fn envelope_of_parse(output: &Output) -> Value {
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    envelope(output)
}
