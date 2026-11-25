//! Adversary cases for story:host-suite-subprocess-bounds (wave 2 unit C).
use crate::local::config;
use crate::local::runtime::{Child, Failure, PrivateProtocol};
use std::{path::Path, process::Command, time::Duration};

fn selection(root: &Path, mode: &str) -> config::Adapter {
    let (path, hash) = crate::local::test_executable::path_and_sha256();
    config::Adapter {
        standing_grants: None,
        credential_revision: None,
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

/// `approval_policy::tests::child` starts a child "with production's" wait
/// when it is given none: it removes the variable. Here the parent is a
/// `policy_child` this test hands 600 s, so the variable is in the parent's
/// own environment; it reports its own wait and starts such a child, which
/// reports its wait and takes no lock.
///
/// On Unix the removal is the second of two things that hold this: the
/// variable names the process that handed it, so a child that inherits one
/// handed to its parent does not take it
/// (`a_wait_handed_to_a_parent_is_not_its_childs_but_that_of_a_process_replacing_the_parent`).
/// On Windows a handed value binds whichever process handed it, and only the
/// removal keeps the child on production's wait.
#[test]
fn blocked_policy_child_keeps_production_wait_when_the_variable_is_inherited() {
    let inherited = Duration::from_secs(600);
    let root = tempfile::Builder::new()
        .prefix("policy-wait-adv-")
        .tempdir()
        .unwrap();
    let mut parent = Command::new(std::env::current_exe().unwrap());
    parent
        .args([
            "--exact",
            "local::approval_policy::tests::policy_child",
            "--ignored",
            "--quiet",
        ])
        .env("CONNECTORS_POLICY_TEST_PATH", root.path())
        .env("CONNECTORS_POLICY_TEST_ACTION", "production-child");
    crate::local::metadata::child_lock_wait(&mut parent, inherited);
    assert!(parent.status().unwrap().success());
    let reported = |name: &str| {
        Duration::from_millis(
            std::fs::read_to_string(root.path().join(name))
                .unwrap()
                .parse()
                .unwrap(),
        )
    };
    assert_eq!(
        reported("parent-lock-wait"),
        inherited,
        "the parent did not run under the variable"
    );
    assert_eq!(
        reported("child-lock-wait"),
        crate::local::metadata::WAIT_BOUND,
        "the child took a wait it merely inherited"
    );
}

/// Starts `policy_child` with the action `lock-wait` through a shell this
/// test hands 600 s, with no removal of the variable anywhere, and returns
/// the wait that child reports. `script` decides whether the shell stays the
/// child's parent or is replaced by it. The shell is `/bin/sh` and the
/// scripts use nothing beyond POSIX `sh`.
#[cfg(unix)]
fn wait_reported_through_a_shell(script: &str) -> Duration {
    let root = tempfile::Builder::new()
        .prefix("policy-wait-adv-")
        .tempdir()
        .unwrap();
    let mut shell = Command::new("/bin/sh");
    shell
        .args(["-c", script])
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "local::approval_policy::tests::policy_child",
            "--ignored",
            "--quiet",
        ])
        .env("CONNECTORS_POLICY_TEST_PATH", root.path())
        .env("CONNECTORS_POLICY_TEST_ACTION", "lock-wait");
    crate::local::metadata::child_lock_wait(&mut shell, Duration::from_secs(600));
    assert!(shell.status().unwrap().success());
    Duration::from_millis(
        std::fs::read_to_string(root.path().join("child-lock-wait"))
            .unwrap()
            .parse()
            .unwrap(),
    )
}

/// The binding of the variable to a process id, alone and across a process
/// boundary (adversary read of
/// story:four-host-tests-no-longer-wait-out-the-30-s-lock-wait). The first
/// child is the state the inner child of
/// `blocked_policy_child_keeps_production_wait_when_the_variable_is_inherited`
/// would be in if `child_production_lock_wait` removed nothing: it inherits
/// a variable its grandparent handed to its parent. Unix only: Windows has no
/// parent process id to bind to.
#[cfg(unix)]
#[test]
fn a_wait_handed_to_a_parent_is_not_its_childs_but_that_of_a_process_replacing_the_parent() {
    // The shell runs the child and outlives it: the child's parent is the
    // shell, the variable names this test. The child is not the script's last
    // command, so no shell runs it in its own place.
    assert_eq!(
        wait_reported_through_a_shell(r#""$0" "$@"; status=$?; exit "$status""#),
        crate::local::metadata::WAIT_BOUND,
        "a child took a wait that was handed to its parent"
    );
    // The shell is replaced by the child: the child's parent is this test,
    // which the variable names. The same report shows a wait that is taken.
    assert_eq!(
        wait_reported_through_a_shell(r#"exec "$0" "$@""#),
        Duration::from_secs(600),
        "a child did not take the wait its own parent handed it"
    );
}
