//! story:owner-errors-are-readable. A CLI write whose owner failed is
//! `outcome_unknown`, and the owner's own error went to a discarded stderr. The
//! owner now writes its diagnostics to a bounded log only it can read, in the
//! state directory, and the failure's data names that log and the owner's code.
//!
//! The production CLI, owner and an adapter child against the read-only
//! `mounted_secret` binding in a jail (`support/mounted.rs`); a test build's
//! owner answers its first publication `outcome_unknown`
//! (`CONNECTORS_TEST_PUBLICATION_FAILS_ONCE`) after it read the credential.
//! Linux only: the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors_custody::mounted::fixture::{Jail, Map};
use mounted::*;
use serde_json::json;
use std::{os::unix::fs::PermissionsExt, process::Stdio};

const ADAPTER_FIXTURE: &str = "owner_errors_are_readable_adapter_fixture";
/// The most bytes the log may hold.
const CAP: u64 = 256 * 1024;

/// The adapter child the owner starts: this binary, filtered to this test.
#[test]
fn owner_errors_are_readable_adapter_fixture() {
    serve_when_child();
}

fn container() -> Container {
    let container = Container::new(ADAPTER_FIXTURE);
    container.deliver(PROFILE, SECRET);
    container.configure("connectors-local/4", &Container::mounted());
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    container
}

fn failing_connect(container: &Container) -> std::process::Output {
    std::fs::write(container.path("/work/fail_once"), b"").unwrap();
    Jail::output(
        container
            .jail
            .command("/bin-test/connectors", Map::Own)
            .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
            .args(["connections", "connect", "--adapter", ALIAS])
            .args(["--profile", PROFILE])
            .env_clear()
            .env("CONNECTORS_TEST_PUBLICATION_FAILS_ONCE", "/work/fail_once")
            .stdin(Stdio::null()),
    )
}

#[test]
fn an_owner_failure_behind_outcome_unknown_names_a_log_that_holds_its_code() {
    let container = container();
    let output = failing_connect(&container);
    assert!(output.stdout.is_empty(), "{output:?}");
    let (code, data) = refusal(&output);
    assert_eq!(
        (code, &data["code"], &data["stage"]),
        (Some(1), &json!("outcome_unknown"), &json!("publication")),
        "{data}"
    );
    assert_eq!(data["owner_error"], "outcome_unknown", "{data}");
    let named = data["owner_log"].as_str().expect("the data names the log");
    assert_eq!(named, format!("{STATE}/owner.log"), "{data}");
    let log = container.path(named);
    let text = std::fs::read_to_string(&log).expect("the named log exists");
    assert!(text.contains("code=outcome_unknown"), "{text}");
    let mode =
        |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&log), 0o600);
    assert_eq!(mode(log.parent().unwrap()), 0o700);
    assert!(std::fs::metadata(&log).unwrap().len() <= CAP);
}

#[test]
fn no_byte_of_the_log_holds_the_credential_a_failing_owner_read() {
    let container = container();
    let output = failing_connect(&container);
    let (_, data) = refusal(&output);
    let log = std::fs::read(container.path(data["owner_log"].as_str().unwrap())).unwrap();
    assert!(!log.is_empty());
    for needle in [SECRET.to_vec(), digest(SECRET).into_bytes()] {
        assert!(
            !log.windows(needle.len()).any(|window| window == needle),
            "the log holds the credential"
        );
    }
}

#[test]
fn a_connect_that_succeeds_writes_nothing_to_stderr() {
    let container = container();
    let output = container.connect(PROFILE);
    success(&output);
    assert!(output.stderr.is_empty(), "{output:?}");
}
