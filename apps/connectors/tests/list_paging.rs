//! `adapters list` and `operations list` page with a self-contained cursor.
//! A list longer than `--limit` answers one page and a `next_cursor`; following
//! it answers the rest and no cursor. A cursor whose selection changed, one
//! older than 300 seconds and one that does not decode answer `stale_cursor`.
//! `--limit` is checked before the cached description is read.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
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

const OPERATIONS: [&str; 3] = ["item.read", "item.list", "item.write"];

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
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    value["result"].clone()
}

fn refusal(output: &Output) -> Value {
    assert!(
        !output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    error["error"]["data"].clone()
}

fn adapter_entry(alias: &str, revision: &str) -> String {
    format!(
        "\n[adapters.{alias}]\ninstance_id='{alias}-local'\nadapter_id='catalog'\nconfiguration_revision='{revision}'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.{alias}.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[adapters.{alias}.permissions]\nprofiles=['token']\noperations=['{}','{}','{}']\n",
        "a".repeat(64),
        OPERATIONS[0],
        OPERATIONS[1],
        OPERATIONS[2]
    )
}

fn initialized(entries: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = command(&root, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config_path = root.path().join("config/config.toml");
    let mut config = fs::read_to_string(&config_path).unwrap();
    for (alias, revision) in entries {
        config += &adapter_entry(alias, revision);
    }
    fs::write(&config_path, config).unwrap();
    root
}

fn bootstrap(instance: &str) -> runtime::Bootstrap {
    let operations = OPERATIONS
        .iter()
        .map(|id| connectors_core::Operation {
            id: (*id).into(),
            description: "fixture operation".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
        })
        .collect();
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: instance.into(),
        adapter: "catalog".into(),
        revision: "desc-1".into(),
        operations,
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        instance: instance.into(),
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
        requirements: OPERATIONS
            .iter()
            .map(|id| runtime::Requirement {
                operation: (*id).into(),
                profile: "token".into(),
                scopes: BTreeSet::new(),
                effect: runtime::Effect::Read,
            })
            .collect(),
    }
}

/// One adapter `forge` with a cached description of three granted operations.
fn described() -> tempfile::TempDir {
    let root = initialized(&[("forge", "cfg-1")]);
    let paths = Paths {
        config: root.path().join("config/config.toml"),
        state: root.path().join("state"),
    };
    let adapter = Config::load(&paths.config).unwrap().adapters["forge"].clone();
    runtime::state::State::new(&paths.state)
        .remember(&adapter.selection(), &bootstrap("forge-local"))
        .unwrap();
    root
}

/// The cursor with its `issued_at` moved 301 seconds into the past, everything
/// else unchanged.
fn aged(cursor: &str) -> String {
    let mut body: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(cursor).unwrap()).unwrap();
    let issued = body["issued_at"].as_u64().unwrap();
    body["issued_at"] = json!(issued - 301);
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&body).unwrap())
}

fn aliases(page: &Value) -> Vec<String> {
    page["adapters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["adapter"].as_str().unwrap().to_owned())
        .collect()
}

fn operation_ids(page: &Value) -> Vec<String> {
    page["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["id"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn adapters_list_pages_with_a_cursor_and_ends_without_one() {
    let root = initialized(&[("alpha", "cfg-1"), ("beta", "cfg-1"), ("gamma", "cfg-1")]);
    let first = success(&command(&root, &["adapters", "list", "--limit", "2"]));
    assert_eq!(aliases(&first), ["alpha", "beta"]);
    let cursor = first["next_cursor"]
        .as_str()
        .expect("a next_cursor")
        .to_owned();
    let rest = success(&command(
        &root,
        &["adapters", "list", "--limit", "2", "--cursor", &cursor],
    ));
    assert_eq!(aliases(&rest), ["gamma"]);
    assert!(rest["next_cursor"].is_null(), "{rest}");
    let whole = success(&command(&root, &["adapters", "list", "--limit", "3"]));
    assert_eq!(aliases(&whole), ["alpha", "beta", "gamma"]);
    assert!(whole["next_cursor"].is_null(), "{whole}");
}

#[test]
fn adapters_list_refuses_a_stale_expired_or_undecodable_cursor() {
    let root = initialized(&[("alpha", "cfg-1"), ("beta", "cfg-1"), ("gamma", "cfg-1")]);
    let first = success(&command(&root, &["adapters", "list", "--limit", "2"]));
    let cursor = first["next_cursor"].as_str().unwrap().to_owned();

    let expired = refusal(&command(
        &root,
        &[
            "adapters",
            "list",
            "--limit",
            "2",
            "--cursor",
            &aged(&cursor),
        ],
    ));
    assert_eq!(expired["code"], "stale_cursor", "{expired}");

    for garbage in [
        "unknown",
        "!!!",
        &URL_SAFE_NO_PAD.encode(b"{\"version\":1}"),
    ] {
        let refused = refusal(&command(
            &root,
            &["adapters", "list", "--limit", "2", "--cursor", garbage],
        ));
        assert_eq!(refused["code"], "stale_cursor", "{garbage}: {refused}");
    }

    // A cursor issued for another page limit selects another page.
    let other_limit = refusal(&command(
        &root,
        &["adapters", "list", "--limit", "1", "--cursor", &cursor],
    ));
    assert_eq!(other_limit["code"], "stale_cursor", "{other_limit}");

    // The configuration changed under the cursor.
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap() + &adapter_entry("delta", "cfg-1");
    fs::write(&config_path, config).unwrap();
    let changed = refusal(&command(
        &root,
        &["adapters", "list", "--limit", "2", "--cursor", &cursor],
    ));
    assert_eq!(changed["code"], "stale_cursor", "{changed}");
    assert_eq!(changed["stage"], "observation", "{changed}");
}

#[test]
fn operations_list_pages_with_a_cursor_and_refuses_a_stale_one() {
    let root = described();
    let first = success(&command(
        &root,
        &["operations", "list", "--adapter", "forge", "--limit", "2"],
    ));
    assert_eq!(operation_ids(&first).len(), 2, "{first}");
    let cursor = first["next_cursor"]
        .as_str()
        .expect("a next_cursor")
        .to_owned();
    let rest = success(&command(
        &root,
        &[
            "operations",
            "list",
            "--adapter",
            "forge",
            "--limit",
            "2",
            "--cursor",
            &cursor,
        ],
    ));
    assert_eq!(operation_ids(&rest).len(), 1, "{rest}");
    assert!(rest["next_cursor"].is_null(), "{rest}");
    let mut all = operation_ids(&first);
    all.extend(operation_ids(&rest));
    all.sort();
    let mut expected = OPERATIONS.map(String::from).to_vec();
    expected.sort();
    assert_eq!(all, expected);

    for cursor in [aged(&cursor), "unknown".to_owned()] {
        let refused = refusal(&command(
            &root,
            &[
                "operations",
                "list",
                "--adapter",
                "forge",
                "--limit",
                "2",
                "--cursor",
                &cursor,
            ],
        ));
        assert_eq!(refused["code"], "stale_cursor", "{refused}");
    }

    // Revoking a grant changes the permitted operations under the cursor.
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap().replace(
        &format!(
            "operations=['{}','{}','{}']",
            OPERATIONS[0], OPERATIONS[1], OPERATIONS[2]
        ),
        &format!("operations=['{}','{}']", OPERATIONS[0], OPERATIONS[1]),
    );
    fs::write(&config_path, config).unwrap();
    let changed = refusal(&command(
        &root,
        &[
            "operations",
            "list",
            "--adapter",
            "forge",
            "--limit",
            "2",
            "--cursor",
            &cursor,
        ],
    ));
    assert_eq!(changed["code"], "stale_cursor", "{changed}");
}

#[test]
fn operations_list_checks_the_limit_before_reading_the_cached_description() {
    let root = initialized(&[("forge", "cfg-1")]);
    for limit in ["0", "501"] {
        let refused = refusal(&command(
            &root,
            &["operations", "list", "--adapter", "forge", "--limit", limit],
        ));
        assert_eq!(refused["code"], "invalid_input", "{limit}: {refused}");
    }
    let unlimited = refusal(&command(
        &root,
        &["operations", "list", "--adapter", "forge"],
    ));
    assert_eq!(unlimited["code"], "description_unavailable", "{unlimited}");
}
