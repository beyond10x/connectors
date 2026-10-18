//! story:connections-reconnect-verb: `connections reconnect` replaces
//! `connections repair`, and the `instances` group replaces `adapters`. Each old
//! path stays a path alias for the deprecation window: it runs the same command
//! with the same result and says it is deprecated exactly once
//! (`contracts/cli/v1alpha1/semantics.md` §3 "Renamed commands").
use connectors_cli_contract::wire::{ArgumentSource, Command as Declaration, Plan, Shape};
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
    sync::OnceLock,
};

/// A directory holding no executable (`cli_surface_minor.rs`).
const NOT_INSTALLED: &str = if cfg!(windows) {
    "C:/not-installed"
} else {
    "/not-installed"
};
const REVISION: &str = "desc-1";
const OPERATION: &str = "item.read";
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
/// carries the pins of the legacy entry `forge`, with a cached description. No
/// executable exists and none is launched.
fn configured() -> tempfile::TempDir {
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

fn private_file(path: &Path, text: &str) -> String {
    fs::write(path, text).unwrap();
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    path.to_str().unwrap().to_owned()
}

fn plan() -> &'static Plan {
    static PLAN: OnceLock<Plan> = OnceLock::new();
    PLAN.get_or_init(connectors_cli_contract::plan)
}

/// Every renamed path, old and new, read from the plan's path aliases.
fn renamed() -> Vec<(Vec<String>, &'static Declaration)> {
    plan()
        .commands
        .iter()
        .flat_map(|declaration| {
            declaration
                .aliases
                .iter()
                .map(move |alias| (alias.clone(), declaration))
        })
        .collect()
}

/// The arguments after the path: the instance selected with `--connector
/// --instance`, and a value for each other required argument.
fn arguments(root: &Path, declaration: &Declaration) -> Vec<String> {
    let callable = &plan().callables[&declaration.callable];
    let Some(Shape::Struct { fields }) = callable.input.as_ref().map(|input| &input.shape) else {
        return Vec::new();
    };
    let credential = private_file(&root.join("credential"), "x");
    let mut args = Vec::new();
    for argument in &declaration.arguments {
        let field = argument.field.as_str();
        let value = match field {
            "adapter" => continue,
            "connector" => CONNECTOR.into(),
            "instance" => INSTANCE.into(),
            "credential_document" => {
                args.extend(["--credential-file".into(), credential.clone()]);
                continue;
            }
            "connection" => "conn-1".into(),
            _ if fields[field].optional() => continue,
            _ => "1".into(),
        };
        let ArgumentSource::Option { long } = &argument.source else {
            panic!("{field}: no option source");
        };
        args.extend([format!("--{long}"), value]);
    }
    args
}

/// The envelope without observation times.
fn steady(mut value: Value) -> Value {
    fn walk(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                for (key, field) in fields.iter_mut() {
                    if key.ends_with("_ms") && field.is_i64() {
                        *field = json!(0);
                    } else {
                        walk(field);
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(walk),
            _ => {}
        }
    }
    walk(&mut value);
    value
}

/// One parsed envelope: the success envelope on stdout with an empty standard
/// error, or exactly one failure envelope on standard error with an empty stdout.
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

/// Run `path` and its arguments on a freshly configured instance, so a command
/// that changes state (`instances stop`, `connections reconnect`) answers each
/// name from the same state; then stop any owner the run started.
fn run(output: &str, path: &[String], declaration: &Declaration, extra: &[&str]) -> Output {
    let root = configured();
    let rest = arguments(root.path(), declaration);
    let args: Vec<&str> = path
        .iter()
        .chain(&rest)
        .map(String::as_str)
        .chain(extra.iter().copied())
        .collect();
    let ran = command(root.path(), output, &args);
    let stopped = command(root.path(), "json", &["setup", "owner-stop"]);
    assert!(stopped.status.success(), "{stopped:?}");
    ran
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Human output without observation times: every run of digits is one `0`.
fn steady_text(text: &str) -> String {
    let mut out = String::new();
    let mut digits = false;
    for c in text.chars() {
        if c.is_ascii_digit() {
            if !digits {
                out.push('0');
            }
            digits = true;
        } else {
            out.push(c);
            digits = false;
        }
    }
    out
}

/// The renamed paths are exactly the five the story names, each an alias of
/// its new name and of nothing else.
#[test]
fn the_old_paths_are_aliases_of_the_new_ones() {
    let actual: BTreeSet<(String, String)> = renamed()
        .into_iter()
        .map(|(old, declaration)| (old.join(" "), declaration.path.join(" ")))
        .collect();
    let expected: BTreeSet<(String, String)> = [
        ("connections repair", "connections reconnect"),
        ("adapters list", "instances list"),
        ("adapters describe", "instances describe"),
        ("adapters status", "instances status"),
        ("adapters stop", "instances stop"),
    ]
    .into_iter()
    .map(|(old, new)| (old.to_owned(), new.to_owned()))
    .collect();
    assert_eq!(actual, expected);
}

/// The acceptance: each renamed path runs under its old and its new name with
/// the same arguments against one instance. Under `--output json` both answer
/// the same envelope with the same exit status, and the old name's carries one
/// `deprecations` entry naming the new path; under `--output human` both write
/// the same stdout, and the old name adds exactly one line on standard error
/// ahead of what the new one writes there.
#[test]
fn each_old_name_answers_as_its_new_name_and_says_it_is_deprecated_once() {
    let mut wrong = Vec::new();
    let mut outcomes = Vec::new();
    for (old, declaration) in renamed() {
        let new = &declaration.path;
        let name = new.join(" ");

        let (by_new, by_old) = (
            run("json", new, declaration, &[]),
            run("json", &old, declaration, &[]),
        );
        assert_eq!(by_new.status.code(), by_old.status.code(), "{name}");
        let first = envelope(&by_new);
        let mut second = envelope(&by_old);
        assert!(first.get("deprecations").is_none(), "{name}: {first}");
        let deprecations = second
            .as_object_mut()
            .unwrap()
            .remove("deprecations")
            .unwrap_or_else(|| panic!("{name}: no deprecations: {second}"));
        assert_eq!(
            deprecations,
            json!([{"command": old.join(" "), "replacement": [name.clone()]}]),
            "{name}"
        );
        if steady(first.clone()) != steady(second.clone()) {
            wrong.push(format!("{name}:\n  {first}\n  {second}"));
        }
        if first["ok"] == true {
            let shape = &plan().callables[&declaration.callable].result.shape;
            assert!(shape.accepts(&first["result"]), "{name}: {first}");
        }
        outcomes.push(format!(
            "{name}: {}",
            if first["ok"] == true {
                "ok".to_owned()
            } else {
                first["error"]["data"]["code"].to_string()
            }
        ));

        let (by_new, by_old) = (
            run("human", new, declaration, &[]),
            run("human", &old, declaration, &[]),
        );
        assert_eq!(by_new.status.code(), by_old.status.code(), "{name}");
        assert_eq!(
            steady_text(&text(&by_new.stdout)),
            steady_text(&text(&by_old.stdout)),
            "{name}"
        );
        let (new_err, old_err) = (text(&by_new.stderr), text(&by_old.stderr));
        let line = format!("deprecated: {}; use {name}\n", old.join(" "));
        assert!(
            !new_err.lines().any(|line| line.starts_with("deprecated:")),
            "{name}: {new_err}"
        );
        assert_eq!(
            old_err
                .lines()
                .filter(|l| l.starts_with("deprecated:"))
                .count(),
            1,
            "{name}: {old_err}"
        );
        assert_eq!(
            steady_text(&old_err),
            steady_text(&format!("{line}{new_err}")),
            "{name}"
        );
    }
    println!("{}", outcomes.join("\n"));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(
        outcomes.iter().filter(|o| o.ends_with(": ok")).count() >= 3,
        "{outcomes:?}"
    );
}

/// An old name with the deprecated `--adapter` names both: one entry each,
/// the path first, and one line each under `--output human`.
#[test]
fn an_old_name_with_the_adapter_selector_names_both_deprecations() {
    let root = configured();
    let output = command(
        root.path(),
        "json",
        &["adapters", "describe", "--adapter", ALIAS],
    );
    assert_eq!(
        envelope(&output)["deprecations"],
        json!([
            {"command":"adapters describe","replacement":["instances describe"]},
            {"option":"--adapter","replacement":["--connector","--instance"]}
        ])
    );
    let output = command(
        root.path(),
        "human",
        &["adapters", "describe", "--adapter", ALIAS],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        text(&output.stderr),
        "deprecated: adapters describe; use instances describe\n\
         deprecated: --adapter; select with --connector <connector> --instance <instance>\n"
    );
}

/// A parse refusal under an old name keeps its one envelope, which carries the
/// deprecation, as `--adapter`'s does.
#[test]
fn a_parse_refusal_under_an_old_name_carries_the_deprecation() {
    let root = configured();
    let output = command(root.path(), "json", &["adapters", "list", "--no-such-flag"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let refused = envelope(&output);
    assert_eq!(refused["error"]["code"], "cli_parse", "{refused}");
    assert_eq!(
        refused["deprecations"],
        json!([{"command":"adapters list","replacement":["instances list"]}])
    );
    let output = command(
        root.path(),
        "json",
        &["instances", "list", "--no-such-flag"],
    );
    assert!(envelope(&output).get("deprecations").is_none());
}
