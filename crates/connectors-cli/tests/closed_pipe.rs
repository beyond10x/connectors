//! A consumer may stop reading a successful result; other failures remain failures.

use std::fs;
use std::io::{BufRead as _, Read as _, Write as _};
use std::os::fd::OwnedFd;
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::net::{UnixDatagram, UnixListener};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

const FORMATS: [&str; 4] = ["json", "yaml", "text", "compact"];
// Larger than a 64 KiB pipe, but still inside the 256 KiB operation response bound. A scalar
// keeps the text table's deliberate column clipping from shrinking the synthetic response.
const PAYLOAD_BYTES: usize = 128 * 1024;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "pipe-{:x}-{:x}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let config = root.join("connectors.toml");
        fs::write(
            &config,
            format!(
                "[owner]\ntenant_id = 'fixture'\nagent_id = 'fixture'\nagent_revision = 1\n\
                 authority_snapshot_id = 'fixture'\nauthority_snapshot_sha256 = '{}'\n\n\
                 [[catalog]]\nprovider = 'slack'\ngrant_ref = 'fixture'\ninitiation = 'platform'\n",
                "a".repeat(64)
            ),
        )
        .unwrap();
        fs::set_permissions(config, fs::Permissions::from_mode(0o600)).unwrap();
        Self { root }
    }

    fn command(&self, format: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
        command
            .args(["--output", format])
            .env("HOME", &self.root)
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("PATH", self.root.join("no-programs"))
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    fn invoke(&self, format: &str) -> Command {
        let mut command = self.command(format);
        command
            .args([
                "operation",
                "invoke",
                "--operation",
                "fixture.read",
                "--connection",
                "fixture.connection",
                "--description-ref",
                "fixture.description",
                "--input-json",
                "{}",
                "--config",
            ])
            .arg(self.root.join("connectors.toml"))
            .arg("--state-root")
            .arg(&self.root);
        command
    }

    fn respond(&self, close_transport: bool) -> JoinHandle<()> {
        let listener = UnixListener::bind(self.root.join("connectors.sock")).unwrap();
        listener.set_nonblocking(true).unwrap();
        thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        thread::sleep(Duration::from_millis(10));
                    }
                    result => panic!("local fixture not reached: {result:?}"),
                }
            };
            if close_transport {
                return;
            }
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut line = String::new();
            std::io::BufReader::new(&stream)
                .read_line(&mut line)
                .unwrap();
            let request: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(request["request"]["method"], "invoke");
            let response = json!({
                "protocol": request["protocol"], "request_id": request["request_id"],
                "status": "ok", "response": {"result": "invoke", "value": {
                    "operation_ref": "fixture.read", "output": "x".repeat(PAYLOAD_BYTES),
                    "connector_audit_ref": "fixture.audit"
                }}
            });
            assert!(response.to_string().len() < protocol::operation::MAX_RESULT_BYTES);
            writeln!(stream, "{response}").unwrap();
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {}; stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{output:?}");
}

fn consumer_closes(format: &str, reads_first: bool) {
    let fixture = Fixture::new();
    let server = fixture.respond(false);
    let mut child = fixture.invoke(format).spawn().unwrap();
    let mut reader = child.stdout.take().unwrap();
    if reads_first {
        reader.read_exact(&mut [0]).unwrap();
    }
    drop(reader);
    let output = child.wait_with_output().unwrap();
    server.join().unwrap();
    assert_success(&output);
}

#[test]
fn json_consumer_closes_early() {
    consumer_closes("json", true);
    consumer_closes("json", false);
}

#[test]
fn yaml_consumer_closes_early() {
    consumer_closes("yaml", true);
    consumer_closes("yaml", false);
}

#[test]
fn text_consumer_closes_early() {
    consumer_closes("text", true);
    consumer_closes("text", false);
}

#[test]
fn compact_consumer_closes_early() {
    consumer_closes("compact", true);
    consumer_closes("compact", false);
}

#[test]
fn every_format_really_emits_more_than_a_64_kib_pipe_buffer() {
    for format in FORMATS {
        let fixture = Fixture::new();
        let server = fixture.respond(false);
        let output = fixture.invoke(format).output().unwrap();
        server.join().unwrap();
        assert_success(&output);
        assert!(
            output.stdout.len() > 64 * 1024,
            "{format}: only {} bytes emitted",
            output.stdout.len()
        );
        println!("{format}: {} output bytes", output.stdout.len());
    }
}

#[test]
fn a_real_non_broken_pipe_output_failure_stays_unsuccessful() {
    for format in FORMATS {
        let fixture = Fixture::new();
        // This is a real stdout write failure, distinct from a consumer's closed pipe. Unlike
        // an invalid descriptor (which Rust stdout deliberately ignores), it reaches the caller.
        let sink = UnixDatagram::unbound().unwrap();
        let refusal = sink.send(b"probe").unwrap_err();
        assert_ne!(refusal.kind(), std::io::ErrorKind::BrokenPipe);
        let output = fixture
            .command(format)
            .args(["inspect", "providers"])
            .stdout(OwnedFd::from(sink))
            .output()
            .unwrap();
        assert!(!output.status.success(), "{format}: {output:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("the result could not be written")
                || error.contains("failed printing to stdout"),
            "{format}: {error}"
        );
        assert!(!error.contains("Broken pipe"), "{format}: {error}");
    }
}

#[test]
fn a_closed_transport_stays_unsuccessful() {
    for format in FORMATS {
        let fixture = Fixture::new();
        let server = fixture.respond(true);
        let output = fixture.invoke(format).output().unwrap();
        server.join().unwrap();
        assert!(!output.status.success(), "{format}: {output:?}");
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(diagnostic.contains("connector-unreachable"), "{diagnostic}");
    }
}

#[test]
fn completion_scripts_still_accept_a_closed_reader() {
    let fixture = Fixture::new();
    let mut child = fixture
        .command("text")
        .args(["setup", "completions", "bash"])
        .spawn()
        .unwrap();
    drop(child.stdout.take().unwrap());
    assert_success(&child.wait_with_output().unwrap());
}
