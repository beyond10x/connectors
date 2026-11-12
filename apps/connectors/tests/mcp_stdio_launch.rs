//! story:mcp-stdio-on-macos-and-windows. The launch contract of `connectors
//! server --transport stdio` that needs no custody binding, so it runs on every
//! platform the server serves on: the selected launch's own help, and a launch
//! that is invalid or whose owner is unavailable ends with a diagnostic on
//! standard error and no byte on protocol standard output.
//!
//! The tests that list and call through an owner use the Linux-only mounted
//! custody binding (`local_mcp_server.rs`).
use std::process::{Command, Stdio};

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
    command.env_clear().stdin(Stdio::null());
    command
}

#[test]
fn selected_launch_help_uses_the_native_grammar() {
    let output = command().args(["server", "--help"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("--transport"), "{help}");
    assert!(help.contains("--config"), "{help}");
    assert!(!help.contains("--output"), "{help}");
}

#[test]
fn owner_refusal_and_invalid_launch_emit_no_protocol_bytes() {
    let root = tempfile::tempdir().unwrap();
    for suffix in [
        vec!["--transport", "stdio"],
        vec![],
        vec!["--transport", "http"],
        vec!["--transport", "stdio", "--output", "json"],
        vec!["--transport", "stdio", "--credential-stdin"],
    ] {
        let output = command()
            .arg("--config")
            .arg(root.path().join("absent.toml"))
            .arg("--state-dir")
            .arg(root.path().join("absent-state"))
            .arg("server")
            .args(suffix)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            output.stdout.is_empty(),
            "CLI envelope leaked to protocol stdout: {output:?}"
        );
        assert!(!output.stderr.is_empty());
        assert!(!root.path().join("absent-state").exists());
    }
}
