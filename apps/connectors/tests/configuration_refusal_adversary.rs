// Adversary cases for story:configuration-refusal-names-the-entry.
// Every CLI command that refuses a format/private-protocol mismatch as
// `invalid_configuration` must name the format and the entry, whichever load
// site inside the CLI process reached the configuration first.
use serde_json::Value;
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

fn command(root: &tempfile::TempDir, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.path().join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.path().join("state"))
        .args(args)
        .output()
        .unwrap()
}

fn mismatched_v1(root: &tempfile::TempDir) {
    let init = command(root, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path)
        .unwrap()
        .replace("connectors-local/2", "connectors-local/1")
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.forge.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
}

#[test]
fn every_cli_command_refusing_a_mismatched_entry_names_format_and_entry() {
    let root = tempfile::tempdir().unwrap();
    mismatched_v1(&root);
    let credential = root.path().join("credential.json");
    fs::write(&credential, "{}").unwrap();
    fs::set_permissions(&credential, fs::Permissions::from_mode(0o600)).unwrap();
    let credential = credential.to_str().unwrap().to_owned();
    let invoke_tail = [
        "--operation",
        "read",
        "--schema",
        "s",
        "--revision",
        "r",
        "--input-json",
        "{}",
    ];
    let commands: Vec<Vec<&str>> = vec![
        vec!["setup", "check"],
        vec!["adapters", "list"],
        vec!["adapters", "describe", "--adapter", "forge"],
        vec!["adapters", "status", "--adapter", "forge"],
        vec!["connections", "list", "--adapter", "forge"],
        vec!["operations", "list", "--adapter", "forge"],
        vec![
            "operations",
            "describe",
            "--adapter",
            "forge",
            "--operation",
            "read",
        ],
        [
            vec!["operations", "invoke", "--adapter", "forge"],
            invoke_tail.to_vec(),
        ]
        .concat(),
        vec!["approvals", "policy-status", "--adapter", "forge"],
        vec!["approvals", "key-status", "--adapter", "forge"],
        [
            vec![
                "approvals",
                "prepare",
                "--adapter",
                "forge",
                "--connection",
                "c1",
            ],
            invoke_tail.to_vec(),
        ]
        .concat(),
        vec![
            "connections",
            "connect",
            "--adapter",
            "forge",
            "--profile",
            "p",
            "--credential-file",
            credential.as_str(),
        ],
    ];
    let mut unnamed = Vec::new();
    for args in &commands {
        let output = command(&root, args);
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let data = serde_json::from_slice::<Value>(&output.stderr)
            .map(|v| v["error"]["data"].clone())
            .unwrap_or(Value::Null);
        if data["code"] != "invalid_configuration"
            || data["configuration_format"] != "connectors-local/1"
            || data["instance_id"] != "forge-local"
        {
            unnamed.push(format!(
                "{}: exit {:?} {}",
                args.join(" "),
                output.status.code(),
                stderr.trim()
            ));
        }
    }
    assert!(
        unnamed.is_empty(),
        "commands whose refusal does not name the mismatched entry:\n{}",
        unnamed.join("\n")
    );
}
