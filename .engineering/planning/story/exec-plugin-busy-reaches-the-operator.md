---
format: aep.planning-md/3
id: story:exec-plugin-busy-reaches-the-operator
kind: story
status: draft
title: An exec plugin that cannot run tells the operator whether it is busy or missing
relations:
- serves: vision:independent-contract-adapters
- informed_by: review-result:wave-20261019-post0540-u4-adversary-1
scope:
- confidence: cited
  path: adapters/kubernetes/src/local.rs
- confidence: cited
  path: crates/connectors-host/src/local/runtime.rs
revision: 2
---
## Outcome

When an exec plugin cannot be run, the answer the host gives says whether the plugin is missing or busy (`ExecutableFileBusy`), so an operator can tell the two apart.

## Why now

Split from story:ci-timing-flakes-20261018 by wave 20261019-post0540 (review-result:wave-20261019-post0540-u4-adversary-1, finding 1, origin undecided, decided by the coordinator as out of the unit's surface). U4 made the Kubernetes adapter's `PluginUnavailable` carry `Option<io::ErrorKind>` (commit b9731c6ef), but `exec_failure` (`adapters/kubernetes/src/local.rs:417`) maps every `PluginUnavailable` to `Failure::InvalidConfiguration`, and the host's `Failure` (`crates/connectors-host/src/local/runtime.rs:42`) has no field to carry the kind. The acceptance line folded into ci-timing-flakes from kubernetes-exec-fixture-busy ("an operator can tell a missing plugin from a busy one") is therefore not met.

## Acceptance

- The test under "Red test" passes: through `connectors_host::local::runtime::Child`, `validate` with a busy plugin and with a missing plugin give different answers to the host.
- The difference reaches the CLI's failure output without carrying OS text or a path.

## Scope

- **Files:** `adapters/kubernetes/src/local.rs` (`exec_failure` ~:417) — cited (adversary report)
- **Files:** `crates/connectors-host/src/local/runtime.rs` (`Failure` ~:42) — cited (adversary report)
- **Confidence:** medium; how the kind travels (a new `Failure` variant or a detail field) is undecided

## Red test

`adapters/kubernetes/tests/exec_plugin_unavailable_operator_view.rs`, as written by the adversary (red on b9731c6ef: both answers `Err(InvalidConfiguration)`):

```rust
//! story:ci-timing-flakes-20261018, acceptance folded from
//! story:kubernetes-exec-fixture-busy: "`PluginUnavailable` from an exec
//! plugin carries the spawn error kind, so an operator can tell a missing
//! plugin from a busy one". The operator does not hold a `Refusal`; the host
//! receives what the adapter child answers. This drives the adapter child the
//! way the host does and compares the two answers.
#![cfg(target_os = "linux")]
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    runtime::Child,
};
use connectors_sdk::Secret;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::BufRead,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const USER: &str = "eks-fixture-user";

fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

/// The plugin is written by a `cp` child, as the unit's fixtures are.
fn executable(path: &Path, contents: &[u8]) {
    let staged = path.with_extension("staged");
    fs::write(&staged, contents).unwrap();
    let status = Command::new("cp").arg(&staged).arg(path).status().unwrap();
    assert!(status.success());
    fs::remove_file(&staged).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

fn selection(config: &Path) -> Adapter {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-kubernetes"))
        .arg("--local-config")
        .arg(config)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bootstrap: connectors_host::local::runtime::Bootstrap =
        serde_json::from_slice(&output.stdout).unwrap();
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-kubernetes"))
        .canonicalize()
        .unwrap();
    Adapter {
        credential_revision: None,
        private_protocol: None,
        permissions: Default::default(),
        instance_id: "fixture-kubernetes".into(),
        adapter_id: "kubernetes".into(),
        configuration_revision: bootstrap.configuration_revision,
        protocol: "v1alpha1".into(),
        startup: Startup::OnDemand,
        restart: Restart::Never,
        executable: Executable {
            sha256: hex::encode(Sha256::digest(fs::read(&binary).unwrap())),
            path: binary,
            args: vec!["--local-config".into(), config.to_str().unwrap().into()],
        },
    }
}

fn validate(config: &Path) -> String {
    let mut child = Child::spawn(&selection(config)).unwrap();
    let answer = child.validate(
        "kubernetes.exec_plugin",
        &Secret(serde_json::to_vec(&json!({ "user": USER })).unwrap()),
        connectors_sdk::now_ms() + 30_000,
    );
    format!("{:?}", answer.map(|_| ()))
}

#[test]
fn an_operator_tells_a_busy_exec_plugin_from_a_missing_one() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("private");
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let plugin = directory.join("fixture-plugin");
    let script = b"#!/bin/sh\nexit 1\n";
    executable(&plugin, script);
    let kubeconfig = directory.join("kubeconfig");
    private(
        &kubeconfig,
        format!(
            "apiVersion: v1\nkind: Config\ncurrent-context: eks\n\
             contexts:\n- name: eks\n  context:\n    cluster: fixture-cluster\n    user: {USER}\n\
             users:\n- name: {USER}\n  user:\n    exec:\n      apiVersion: client.authentication.k8s.io/v1\n      \
             command: {}\n",
            plugin.display()
        )
        .as_bytes(),
    );
    let config = directory.join("kubernetes.json");
    private(
        &config,
        &serde_json::to_vec(&json!({
            "format": "connectors-kubernetes-local/1",
            "instance": "fixture-kubernetes",
            "api_base": "https://localhost:1/",
            "namespaces": ["fixture"],
            "resource_kinds": ["pods"],
            "kubeconfig": kubeconfig,
            "exec_plugin": {
                "path": plugin,
                "sha256": hex::encode(Sha256::digest(script)),
                "user": USER,
                "env": [],
                "timeout_ms": 5000
            }
        }))
        .unwrap(),
    );

    // Busy: another process holds the plugin open for writing (ETXTBSY).
    let mut writer = Command::new("/bin/sh")
        .args(["-c", "exec 3>>\"$1\" && echo held && read -r line", "writer"])
        .arg(&plugin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut held = String::new();
    std::io::BufReader::new(writer.stdout.as_mut().unwrap())
        .read_line(&mut held)
        .unwrap();
    assert_eq!(held, "held\n");
    let busy = validate(&config);
    writer.kill().unwrap();
    writer.wait().unwrap();
    // Control: released, the same configuration reaches the plugin, which
    // exits 1 (`plugin_failed`, `unavailable`). So the busy answer above is
    // the plugin's refusal, not the kubeconfig or the binding.
    assert_eq!(validate(&config), "Err(Unavailable)");

    // Missing: the pinned path names nothing.
    fs::remove_file(&plugin).unwrap();
    let missing = validate(&config);

    assert_ne!(
        busy, missing,
        "the host receives the same answer for a busy plugin and a missing one"
    );
}
```
