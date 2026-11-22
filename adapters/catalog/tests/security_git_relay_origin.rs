//! Security review 2026-11-22, finding 2: the git relay sends the instance's
//! token to its configured upstream, which was accepted independently of the
//! origin the instance's credential is provisioned for. Start now refuses an
//! upstream off the instance's API origin (or the Git origin reviewed for it,
//! `served::git` `GIT_ORIGINS`) before the bridge reads the mounted
//! credential. The served binary runs with a credential directory that does
//! not exist: a start that reached custody refuses naming the credential, one
//! refused earlier names the upstream. Nothing is contacted; every host is
//! under `.invalid`.
#![cfg(unix)]

use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

const ENDPOINT: &str = "https://gitlab.fixture.invalid";
const UPSTREAM_REFUSED: &str = "upstream is not the origin of the instance's API";
const CREDENTIAL_UNREAD: &str = "the mounted credential is not available";

/// The served binary's start refusal for a relay to `upstream`.
fn start(root: &Path, upstream: &str) -> String {
    let config = json!({
        "service": {"instance": "gitlab.fixture", "listen": "127.0.0.1:0"},
        "catalog": {"connector": "gitlab", "instance": "fixture", "endpoint": ENDPOINT},
        "credential": {
            "binding": "mounted_secret",
            "directory": root.join("no-credential"),
        },
        "git": {
            "upstream": upstream,
            "repositories": ["group/project"],
            "ref_prefixes": ["agent/"],
            "work_directory": root.join("work"),
        },
    });
    let path = root.join("served.json");
    write(&path, &config);
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--config")
        .arg(&path)
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .output()
        .unwrap();
    assert!(!output.status.success(), "{upstream}: start succeeded");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn write(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn security_an_unapproved_upstream_refuses_start_before_the_credential_is_read() {
    let root = tempfile::tempdir().unwrap();
    for upstream in [
        "https://elsewhere.fixture.invalid",
        "https://gitlab.fixture.invalid:8443",
        "https://gitlab.fixture.invalid.example.invalid",
        "https://github.com",
    ] {
        let refused = start(root.path(), upstream);
        assert!(
            refused.contains(UPSTREAM_REFUSED),
            "{upstream}: not refused by its origin: {refused}"
        );
        assert!(
            !refused.contains(CREDENTIAL_UNREAD),
            "{upstream}: the credential was read first: {refused}"
        );
    }
    assert!(
        !root.path().join("work").exists(),
        "the relay prepared its work directory for an unapproved upstream"
    );
}

#[test]
fn security_the_api_origin_is_approved_and_start_goes_on_to_the_credential() {
    let root = tempfile::tempdir().unwrap();
    for upstream in [ENDPOINT, "https://gitlab.fixture.invalid/scm"] {
        let refused = start(root.path(), upstream);
        assert!(
            !refused.contains(UPSTREAM_REFUSED),
            "{upstream}: the API origin was refused: {refused}"
        );
        assert!(
            refused.contains(CREDENTIAL_UNREAD),
            "{upstream}: start did not go on to the credential: {refused}"
        );
    }
}
