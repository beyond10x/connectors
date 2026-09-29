use serde_json::Value;
use std::{
    fs,
    process::{Command, Output},
};

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
    assert_eq!(value["ok"], true);
    value["result"].clone()
}

/// The first ```toml block after the sentence that tells the reader to add an
/// entry to the configuration created by setup, with its placeholders filled.
fn documented_entry(doc: &str) -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/").to_owned() + doc;
    let text = fs::read_to_string(&path).unwrap();
    let start = text
        .find("Add an adapter entry to the configuration created by setup")
        .unwrap_or_else(|| panic!("{doc}: no add-entry step"));
    let rest = &text[start..];
    let open = rest.find("```toml\n").unwrap() + "```toml\n".len();
    let close = rest[open..].find("```").unwrap();
    rest[open..open + close]
        .replace("REPLACE_FROM_BOOTSTRAP", "cfg-1")
        .replace("REPLACE_WITH_EXECUTABLE_SHA256", &"a".repeat(64))
}

fn follow_documented_steps(doc: &str) {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap() + "\n" + &documented_entry(doc);
    fs::write(&config_path, config).unwrap();
    let output = command(&root, &["setup", "check"]);
    assert!(
        output.status.success(),
        "{doc}: setup init + the documented entry is refused by setup check: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn documented_postgres_entry_passes_setup_check_after_init() {
    follow_documented_steps("local-postgres-cli.md");
}

#[test]
fn documented_kubernetes_entry_passes_setup_check_after_init() {
    follow_documented_steps("local-kubernetes-cli.md");
}

#[test]
fn documented_catalog_entry_passes_setup_check_after_init() {
    follow_documented_steps("local-catalog-provider.md");
}

fn legacy_root(with_entry: bool) -> (tempfile::TempDir, std::path::PathBuf, Vec<u8>) {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let mut text = fs::read_to_string(&config_path)
        .unwrap()
        .replace("connectors-local/2", "connectors-local/1");
    assert!(text.contains("format = \"connectors-local/1\""));
    if with_entry {
        text += &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\n[adapters.forge.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    }
    fs::write(&config_path, &text).unwrap();
    (root, config_path, text.into_bytes())
}

#[test]
fn existing_v1_files_load_and_are_never_rewritten() {
    for with_entry in [false, true] {
        let (root, config_path, before) = legacy_root(with_entry);
        let check = success(&command(&root, &["setup", "check"]));
        assert!(
            check["prerequisites"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["name"] == "configuration" && p["state"] == "ready"),
            "{check}"
        );
        let list = success(&command(&root, &["adapters", "list"]));
        assert_eq!(
            list["adapters"].as_array().unwrap().len(),
            with_entry as usize
        );
        if with_entry {
            success(&command(
                &root,
                &["adapters", "status", "--adapter", "forge"],
            ));
            success(&command(
                &root,
                &["connections", "list", "--adapter", "forge"],
            ));
            let _ = command(
                &root,
                &[
                    "connections",
                    "connect",
                    "--adapter",
                    "forge",
                    "--profile",
                    "gitlab.pat",
                    "--credential-stdin",
                ],
            );
        }
        let again = command(&root, &["setup", "init"]);
        assert_eq!(again.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&again.stderr).contains("configuration_exists"));
        assert_eq!(
            fs::read(&config_path).unwrap(),
            before,
            "with_entry={with_entry}"
        );
    }
}

#[test]
fn initialized_empty_configuration_is_byte_stable_apart_from_owner() {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let text = fs::read_to_string(root.path().join("config/config.toml")).unwrap();
    let uid = text
        .lines()
        .find(|l| l.starts_with("owner_uid"))
        .unwrap()
        .to_owned();
    assert_eq!(
        text.replace(&uid, "owner_uid = UID"),
        "# Linux Secret Service custody is required; credentials never belong here.\nformat = \"connectors-local/2\"\nowner_uid = UID\n\n[adapters]\n"
    );
}
