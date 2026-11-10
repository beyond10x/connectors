//! story:provider-programs-run-on-macos. On macOS the CLI starts the owner from
//! a private launch copy of the executable (`connectors-image.*` in `TMPDIR`)
//! and leaves it to the owner, which removes it when it stops serving
//! (`serve_with_read_policy` holds `own_copy()`). Without that hold every owner
//! start would leak a full copy of `connectors`. This starts an owner through
//! the production CLI with a fresh `TMPDIR`, stops it, waits for the owner to
//! exit (`setup owner-stop` returns once the owner's lifetime lock is free) and
//! asserts no `connectors-image.*` directory is left, after seeing one while
//! the owner served.
#![cfg(target_os = "macos")]
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn command(root: &Path, temporary: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .env("TMPDIR", temporary)
        .args(["--output", "json", "--config"])
        .arg(root.join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(args)
        .output()
        .unwrap()
}

fn copies(temporary: &Path) -> Vec<String> {
    fs::read_dir(temporary)
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with("connectors-image."))
        .collect()
}

#[test]
fn an_owner_that_stops_removes_its_launch_copy() {
    let root = tempfile::tempdir().unwrap();
    // Short, so the owner's socket path stays under the sockaddr limit.
    let temporary = root.path().join("t");
    fs::create_dir(&temporary).unwrap();
    let init = command(root.path(), &temporary, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config_path = root.path().join("config/config.toml");
    let initialized = fs::read_to_string(&config_path).unwrap();
    let owner = initialized
        .lines()
        .find(|line| line.starts_with("owner_uid"))
        .unwrap();
    let table = "connectors.catalog.instances.forge";
    let config = format!(
        "format = \"connectors-local/5\"\n{owner}\n[connectors.catalog]\nenabled=true\n[{table}]\nalias='forge'\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[{table}.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[{table}.permissions]\nprofiles=['token']\noperations=['x']\n",
        "a".repeat(64)
    );
    fs::write(&config_path, config).unwrap();
    let credential = root.path().join("credential.json");
    fs::write(&credential, "{}").unwrap();
    // Starts this state directory's owner; the connect itself is refused.
    command(
        root.path(),
        &temporary,
        &[
            "connections",
            "connect",
            "--adapter",
            "forge",
            "--profile",
            "token",
            "--credential-file",
            credential.to_str().unwrap(),
        ],
    );
    assert!(
        root.path().join("state/owner.sock").exists(),
        "no owner was started"
    );
    assert!(
        !copies(&temporary).is_empty(),
        "the serving owner has no launch copy"
    );
    let stop = command(root.path(), &temporary, &["setup", "owner-stop"]);
    let stopped: Value = serde_json::from_slice(&stop.stdout).unwrap();
    assert_eq!(stopped["result"]["disposition"], "stopped", "{stopped}");
    assert_eq!(
        copies(&temporary),
        Vec::<String>::new(),
        "the owner left its launch copy"
    );
}
