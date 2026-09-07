//! The binary reports compiled capabilities without needing an installation's state.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "inspect-upgrade-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn command(&self, format: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
        command
            .args(["inspect", "upgrade", "--output", format])
            .env("HOME", self.0.join("home"))
            .env("XDG_CONFIG_HOME", self.0.join("config"))
            .env("XDG_STATE_HOME", self.0.join("state"))
            .env("PATH", self.0.join("no-programs"))
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    fn run(&self, format: &str) -> Vec<u8> {
        let output = self.command(format).output().unwrap();
        assert_success(&output);
        assert!(output.stderr.is_empty(), "{output:?}");
        output.stdout
    }

    fn report(&self) -> Value {
        serde_json::from_slice(&self.run("json")).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {}; stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn upgrade_reports_compiled_capabilities_without_state() {
    let fixture = Fixture::new();
    let report = fixture.report();
    assert_eq!(report["cli_version"], env!("CARGO_PKG_VERSION"));
    assert!(report["catalog"]["schema_version"].as_u64().unwrap() > 0);
    let digest = report["catalog"]["digest"].as_str().unwrap();
    assert_eq!(digest.len(), 64);
    assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let versions = report["credential_store"]["read_versions"]
        .as_array()
        .unwrap();
    assert_eq!(versions.len(), 2);
    assert_ne!(versions[0], versions[1]);
    assert_eq!(
        report["credential_store"]["write_versions"],
        report["credential_store"]["read_versions"]
    );
    assert_eq!(
        report["credential_store"]["initial_write_version"],
        versions[0]
    );
    assert_eq!(
        report["credential_store"]["prepared_write_version"],
        versions[1]
    );
    assert!(report["credential_store"]["transition"]
        .as_str()
        .unwrap()
        .contains("prepared"));
    assert!(report["session_metadata_version"].as_u64().unwrap() > 0);
    assert_eq!(report["installation"]["command"], "task install");
    assert!(report["installation"]["context"]
        .as_str()
        .unwrap()
        .contains("source checkout"));
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 0);
}

#[test]
fn upgrade_does_not_start_the_async_runtime() {
    let fixture = Fixture::new();
    for format in ["text", "compact", "json", "yaml"] {
        // Tokio refuses a zero-sized worker pool before dispatch. A compiled report does
        // not need that pool or the I/O driver's socket pair, so it must still succeed.
        let output = fixture
            .command(format)
            .env("TOKIO_WORKER_THREADS", "0")
            .output()
            .unwrap();
        assert_success(&output);
        assert!(output.stderr.is_empty(), "{output:?}");
    }
}

#[test]
fn all_output_modes_preserve_the_reported_facts() {
    let fixture = Fixture::new();
    let report = fixture.report();
    let yaml: Value = serde_norway::from_slice(&fixture.run("yaml")).unwrap();
    assert_eq!(yaml, report);
    for format in ["text", "compact"] {
        let rendered = String::from_utf8(fixture.run(format)).unwrap();
        for fact in [
            env!("CARGO_PKG_VERSION"),
            report["catalog"]["digest"].as_str().unwrap(),
            report["credential_store"]["transition"].as_str().unwrap(),
            "task install",
        ] {
            assert!(rendered.contains(fact), "{format} lost {fact}: {rendered}");
        }
    }
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 0);
}

#[test]
fn unusable_configuration_and_state_paths_do_not_change_the_report() {
    let fixture = Fixture::new();
    let expected = fixture.report();
    for name in ["home", "config", "state"] {
        fs::write(fixture.0.join(name), "SENTINEL-UNREAD-INSTALLATION").unwrap();
    }
    assert_eq!(fixture.report(), expected);
    for name in ["home", "config", "state"] {
        assert_eq!(
            fs::read_to_string(fixture.0.join(name)).unwrap(),
            "SENTINEL-UNREAD-INSTALLATION"
        );
    }
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 3);
}

#[test]
fn a_closed_report_consumer_is_successful() {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;

    let fixture = Fixture::new();
    for format in ["text", "compact", "json", "yaml"] {
        let (writer, reader) = UnixStream::pair().unwrap();
        drop(reader);
        let output = fixture
            .command(format)
            .stdout(OwnedFd::from(writer))
            .output()
            .unwrap();
        assert_success(&output);
        assert!(output.stderr.is_empty(), "{output:?}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn report_write_failures_remain_failures() {
    let fixture = Fixture::new();
    for format in ["text", "compact", "json", "yaml"] {
        let output = fixture
            .command(format)
            .stdout(
                fs::OpenOptions::new()
                    .write(true)
                    .open("/dev/full")
                    .unwrap(),
            )
            .output()
            .unwrap();
        assert!(!output.status.success(), "{format} hid a write failure");
    }
}
