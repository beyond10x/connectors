use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
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

#[test]
fn production_parser_initializes_and_inspects_across_processes() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        success(&command(&root, &["setup", "init"]))["disposition"],
        "created"
    );
    assert_eq!(
        success(&command(&root, &["adapters", "list"]))["adapters"],
        serde_json::json!([])
    );
    let before = fs::read(root.path().join("config/config.toml")).unwrap();
    let exists = command(&root, &["setup", "init"]);
    assert_eq!(exists.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&exists.stderr).contains("configuration_exists"));
    assert_eq!(
        fs::read(root.path().join("config/config.toml")).unwrap(),
        before
    );
    // No configured adapter or credential is created by setup.
    let check = success(&command(&root, &["setup", "check"]));
    assert!(
        check["prerequisites"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "persistent_custody_qualification" && p["state"] == "failed")
    );
}

#[test]
fn inventory_and_unavailable_status_never_launch_configured_executable() {
    let root = tempfile::tempdir().unwrap();
    success(&command(&root, &["setup", "init"]));
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='gitlab'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\n[adapters.forge.executable]\npath='/not-installed/connectors-gitlab'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
    let list = success(&command(&root, &["adapters", "list"]));
    assert_eq!(list["adapters"][0]["adapter"], "forge");
    let status = success(&command(
        &root,
        &["adapters", "status", "--adapter", "forge"],
    ));
    assert_eq!(status["observation"]["state"], "owner_unavailable");
    let describe = success(&command(
        &root,
        &["adapters", "describe", "--adapter", "forge"],
    ));
    assert_eq!(describe["source"], "configuration");
    assert!(describe.get("descriptor").is_none());
    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o644)).unwrap();
    let output = command(&root, &["adapters", "list"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("config.toml"));
}

#[test]
fn protected_sources_are_not_consumed_without_a_coordinator() {
    let root = tempfile::tempdir().unwrap();
    let output = command(
        &root,
        &[
            "connections",
            "connect",
            "--adapter",
            "forge",
            "--profile",
            "token",
            "--credential-file",
            "/private/sentinel-secret-path",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!text.contains("sentinel"));
    let refusal: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(refusal["error"]["code"], "cli_source");
}
