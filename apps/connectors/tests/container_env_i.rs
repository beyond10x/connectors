//! story:mounted-secret-custody, the documented container setup
//! (docs/local-container.md) as a run: the production CLI, owner and an
//! adapter child under an empty environment (`env -i`: no `HOME`, no `PATH`)
//! with only `--config` and `--state-dir`, as a user that is not root, the
//! configuration baked in owner-only, and the credential delivered in a
//! Kubernetes Secret volume's projected layout on a read-only mount.
//!
//! An unprivileged jail maps only the test's own user, so the item here is
//! owner-only (`0400`, owned by the run user). The `fsGroup` delivery, `0440`
//! owned by root with the pod's group, is the custody crate's
//! `group_read_needs_root_and_a_held_group`; a real Secret volume needs a
//! cluster.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use mounted::*;

const ADAPTER_FIXTURE: &str = "container_adapter_fixture";

/// The adapter child the owner starts. The ordinary run does nothing.
#[test]
fn container_adapter_fixture() {
    serve_when_child();
}

#[test]
fn env_i_mounted_custody_round_trip() {
    // SAFETY: geteuid has no preconditions.
    assert_ne!(unsafe { libc::geteuid() }, 0, "the run user is not root");
    let container = Container::new(ADAPTER_FIXTURE);
    container.deliver(PROFILE, SECRET);
    container.configure("connectors-local/4", &Container::mounted());
    // Ephemeral state: each run initializes it beside the baked configuration.
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    let check = success(&container.run(&["setup", "check"]));
    assert_eq!(check["custody_backend"], "mounted_secret", "{check}");
    assert!(
        check["prerequisites"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["state"] == "ready"),
        "{check}"
    );
    let connected = success(&container.connect(PROFILE));
    let connection = connected["connection"]["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let listed = success(&container.run(&["connections", "list", "--adapter", ALIAS]));
    assert_eq!(
        listed["connections"][0]["connection"],
        connection.as_str(),
        "{listed}"
    );
    assert_eq!(listed["connections"][0]["state"], "ready", "{listed}");
    let read = success(&container.invoke(&connection));
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
    assert_eq!(container.seen("invocations"), [digest(SECRET)]);
}
