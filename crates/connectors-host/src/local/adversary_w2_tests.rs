//! Adversary cases for story:host-suite-subprocess-bounds (wave 2 unit C).
use crate::local::config;
use crate::local::runtime::{Child, Failure, PrivateProtocol};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

fn selection(root: &Path, mode: &str) -> config::Adapter {
    let path = std::env::current_exe().unwrap();
    let hash = hex::encode(Sha256::digest(std::fs::read(&path).unwrap()));
    config::Adapter {
        instance_id: "fixture".into(),
        adapter_id: "fixture".into(),
        configuration_revision: "fixture-config".into(),
        protocol: "v1alpha1".into(),
        private_protocol: Some(PrivateProtocol::V2),
        startup: config::Startup::OnDemand,
        restart: config::Restart::Never,
        permissions: Default::default(),
        executable: config::Executable {
            path,
            sha256: hash,
            args: vec![
                "--exact".into(),
                "local::runtime::process::write_tests::fixture".into(),
                "--".into(),
                format!("write-mode={mode}"),
                format!("write-root={}", root.display()),
            ],
        },
    }
}

fn count(root: &Path, name: &str) -> usize {
    std::fs::read_to_string(root.join(name))
        .unwrap_or_default()
        .lines()
        .count()
}

/// `expiring()` discards any preparation that fails after its deadline, with
/// any error, and never looks at the child it discards. This is the attempt it
/// discards, asserted: the preparation times out, the child is terminated, and
/// nothing is sent.
#[test]
fn preparation_whose_deadline_passes_in_preflight_times_out_and_terminates_the_child() {
    let root = tempfile::Builder::new()
        .prefix("private-v2-adv-")
        .tempdir()
        .unwrap();
    let mut child = Child::spawn(&selection(root.path(), "stalled-preflight")).unwrap();
    let result = child
        .prepare_write(
            "write",
            "fixture-private-descriptor",
            "partition",
            &connectors_sdk::Secret(b"fictional-only".to_vec()),
            br#"{"value":true}"#,
            connectors_sdk::now_ms() + 2_000,
        )
        .err();
    // private-adapter.md: a peer that closes at its deadline is observed as
    // unavailability, a socket timeout as a timeout; both are the deadline.
    assert!(
        matches!(result, Some(Failure::Timeout | Failure::Unavailable)),
        "{result:?}"
    );
    assert!(
        !child.running().unwrap(),
        "child left live after a prepare timeout"
    );
    assert_eq!(count(root.path(), "prepare"), 1);
    assert_eq!(count(root.path(), "send"), 0);
}

/// `approval_policy::tests::child` says the `blocked` child "keeps
/// production's wait". It only does when the variable is absent from the
/// parent's own environment: the child inherits it, and every thread of a test
/// process reads it.
#[test]
fn blocked_policy_child_keeps_production_wait_when_the_variable_is_inherited() {
    let started = Instant::now();
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "local::approval_policy::tests::held_use_blocks_cross_process_revocation_until_release",
            "--quiet",
        ])
        .env("CONNECTORS_TEST_LOCK_WAIT_MS", "1:600000")
        .status()
        .unwrap();
    let elapsed = started.elapsed();
    assert!(status.success());
    assert!(
        elapsed < Duration::from_secs(120),
        "blocked child waited {elapsed:?}, not production's 2 s"
    );
}
