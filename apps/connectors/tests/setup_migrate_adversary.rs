//! Adversary cases for story:setup-migrate-adapter-entries: `setup migrate
//! --apply` must never publish a configuration that the next command refuses.
#![cfg(unix)]
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

/// The bound every configuration read admits (`Config::read`, 1 MiB).
const LIMIT: usize = 1024 * 1024;

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

fn envelope(output: &Output) -> Value {
    let stream = if output.status.success() {
        &output.stdout
    } else {
        &output.stderr
    };
    serde_json::from_slice(stream).unwrap_or_else(|_| panic!("{output:?}"))
}

fn private(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

/// A legacy entry written compactly, as an operator may write it.
fn entry(alias: &str, adapter: &str, args: &[String], operations: &[String]) -> String {
    let quoted = |items: &[String]| {
        items
            .iter()
            .map(|item| format!("'{item}'"))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "[adapters.{alias}]\ninstance_id='{alias}-local'\nadapter_id='{adapter}'\nconfiguration_revision='cfg-{alias}'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.{alias}.executable]\npath='/not-installed/{adapter}'\nsha256='{}'\nargs=[{}]\n[adapters.{alias}.permissions]\nprofiles=['fixture.token']\noperations=[{}]\n",
        "a".repeat(64),
        quoted(args),
        quoted(operations)
    )
}

/// A configuration the CLI admits (at most 1 MiB) holding one catalog entry
/// that migrates and MCP entries filled with operations up to the bound. The
/// migration rewrites the whole file in the serializer's spacing and adds the
/// instance table and the defaulted keys, so the file it publishes is larger
/// than the one it read; past 1 MiB, every later command refuses it.
#[test]
fn an_applied_migration_publishes_only_a_configuration_the_next_command_reads() {
    let root = tempfile::tempdir().unwrap();
    assert!(command(root.path(), &["setup", "init"]).status.success());
    let native = root.path().join("native/mail.json");
    private(
        &native,
        &json!({"provider":"google-gmail","api_base":"https://gmail.googleapis.com"}).to_string(),
    );
    let head = format!(
        "format='connectors-local/4'\nowner_uid={}\n{}",
        connectors_host::local::filesystem::uid(),
        entry(
            "mail",
            "catalog",
            &["--local-config".into(), native.to_str().unwrap().into()],
            &["item.read".into()],
        )
    );
    // Fill MCP entries with 120-byte operation ids until one more would
    // cross the bound.
    let mut text = head.clone();
    'fill: for index in 0..63 {
        let alias = format!("pad-{index:02}");
        let mut operations: Vec<String> = Vec::new();
        for op in 0..256 {
            operations.push(format!("{alias}.{op:03}.{}", "x".repeat(110)));
            let candidate = format!("{text}{}", entry(&alias, "mcp", &[], &operations));
            if candidate.len() > LIMIT - 64 {
                operations.pop();
                if !operations.is_empty() {
                    text.push_str(&entry(&alias, "mcp", &[], &operations));
                }
                break 'fill;
            }
        }
        text.push_str(&entry(&alias, "mcp", &[], &operations));
    }
    assert!(text.len() <= LIMIT, "{}", text.len());
    private(&root.path().join("config/config.toml"), &text);

    // Coordinator decision: a rewrite past the read bound is refused in dry-run too.
    let planned = command(root.path(), &["setup", "migrate", "--dry-run"]);
    let planned = envelope(&planned);
    assert_eq!(
        planned["error"]["data"]["configuration_rule"], "config_too_large",
        "{planned}"
    );
    eprintln!("read {} bytes (bound {LIMIT})", text.len());

    let applied = command(root.path(), &["setup", "migrate", "--apply"]);
    let applied_envelope = envelope(&applied);
    let written = fs::read_to_string(root.path().join("config/config.toml")).unwrap();
    // Whatever migrate decided, the file it left must be one the CLI reads.
    let next = command(root.path(), &["instances", "list"]);
    assert!(
        next.status.success(),
        "migrate answered {} and left a {}-byte configuration that the next command refuses: {}",
        applied_envelope["result"]["disposition"],
        written.len(),
        String::from_utf8_lossy(&next.stderr)
    );
}

/// A GitLab entry at a relative root states a `request_prefix` that its
/// declaration produces (it has an `api_path`): it migrates, unlike a Jira
/// gateway entry. Catches the mutant that treats every prefix as a gateway.
#[test]
fn a_gitlab_relative_root_prefix_is_no_gateway() {
    let root = tempfile::tempdir().unwrap();
    assert!(command(root.path(), &["setup", "init"]).status.success());
    let native = |alias: &str, value: Value| {
        let path = root.path().join(format!("native/{alias}.json"));
        private(&path, &value.to_string());
        vec![
            "--local-config".to_owned(),
            path.to_str().unwrap().to_owned(),
        ]
    };
    let gitlab = native(
        "code",
        json!({"provider":"gitlab","api_base":"https://example.com/gitlab/api/v4","request_prefix":"/gitlab"}),
    );
    let gateway = native(
        "wiki",
        json!({"provider":"confluence","api_base":"https://api.atlassian.com/ex/confluence/c/wiki","request_prefix":"/ex/confluence/c"}),
    );
    let text = format!(
        "format='connectors-local/4'\nowner_uid={}\n{}{}",
        connectors_host::local::filesystem::uid(),
        entry("code", "catalog", &gitlab, &["item.read".into()]),
        entry("wiki", "catalog", &gateway, &["item.read".into()]),
    );
    private(&root.path().join("config/config.toml"), &text);
    let result = envelope(&command(root.path(), &["setup", "migrate", "--dry-run"]));
    let entries = result["result"]["entries"].as_array().unwrap();
    let find = |alias: &str| entries.iter().find(|e| e["adapter"] == alias).unwrap();
    assert_eq!(find("code")["disposition"], "migrated", "{result}");
    assert_eq!(find("code")["connector"], "gitlab", "{result}");
    assert_eq!(find("wiki")["reason"], "gateway_prefix", "{result}");
}

/// A configuration reached through a symlink is refused before anything is
/// written: the link stays a link and its target keeps its bytes.
#[test]
fn a_symlinked_configuration_is_refused_and_left_alone() {
    let root = tempfile::tempdir().unwrap();
    assert!(command(root.path(), &["setup", "init"]).status.success());
    let config = root.path().join("config/config.toml");
    let target = root.path().join("config/real.toml");
    fs::rename(&config, &target).unwrap();
    std::os::unix::fs::symlink(&target, &config).unwrap();
    let before = fs::read(&target).unwrap();
    let output = command(root.path(), &["setup", "migrate", "--apply"]);
    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stderr).lines().count(), 1);
    assert!(
        fs::symlink_metadata(&config)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(&target).unwrap(), before);
    let names: Vec<_> = fs::read_dir(root.path().join("config"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert!(
        names.iter().all(|n| !n.contains("before-migrate")),
        "{names:?}"
    );
}
