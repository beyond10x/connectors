//! story:grafana-irm-history-reads, security review: the IRM API URL is a second origin
//! the admitted token is sent to. The host binds a stored credential to the binding's
//! provider authority and refuses to carry it across a changed one
//! (`docs/local-connection-registry.md`, "Configuration upgrades": "Changing the provider
//! authority ... still needs a new instance id"). A configuration that moves the IRM API
//! URL to another origin must therefore change the provider authority, not only the
//! configuration revision, or an upgrading revalidation sends the stored token there.
//! Only the bootstrap is printed; no provider is contacted.
#![cfg(unix)]
use connectors_host::local::{filesystem, runtime::Bootstrap};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

fn bootstrap(directory: &Path, name: &str, irm_api_url: &str) -> Bootstrap {
    let config = directory.join(name);
    fs::write(
        &config,
        serde_json::to_vec(&json!({
            "format": "connectors-grafana-local/1",
            "instance": "fixture-grafana",
            "base_url": "https://stack.fixture.invalid/",
            "irm_api_url": irm_api_url,
        }))
        .unwrap(),
    )
    .unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-grafana"))
        .arg("--local-config")
        .arg(&config)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
    bootstrap.validate().unwrap();
    bootstrap
}

#[test]
fn moving_the_irm_api_url_to_another_origin_changes_the_provider_authority() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("private");
    filesystem::directory(&directory, true, true).unwrap();
    let first = bootstrap(
        &directory,
        "first.json",
        "https://oncall-first.fixture.invalid/",
    );
    let moved = bootstrap(
        &directory,
        "moved.json",
        "https://oncall-moved.fixture.invalid/",
    );
    // The revision moves (the in-tree tests already hold this) ...
    assert_ne!(first.configuration_revision, moved.configuration_revision);
    // ... and so must the authority the stored token is bound to.
    assert_ne!(
        first.provider_authority, moved.provider_authority,
        "the IRM token's second destination is outside the provider authority"
    );
}
