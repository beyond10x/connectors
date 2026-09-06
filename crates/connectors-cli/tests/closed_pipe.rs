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

// Close the output peer before the process starts, so small reports cannot race the test's
// close. A stream socket has the same observable BrokenPipe error as the anonymous pipes above.
fn already_closed_stdout() -> OwnedFd {
    let (reader, mut writer) = std::os::unix::net::UnixStream::pair().unwrap();
    // Concurrent subprocess creation can temporarily inherit a descriptor until exec closes it.
    // Shutdown closes the peer's read direction even while such a duplicate still exists.
    reader.shutdown(std::net::Shutdown::Both).unwrap();
    drop(reader);
    assert_eq!(
        writer.write(b"probe").unwrap_err().kind(),
        std::io::ErrorKind::BrokenPipe
    );
    writer.into()
}

#[test]
fn an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes() {
    let mut unexpectedly_successful = Vec::new();
    for format in FORMATS {
        let fixture = Fixture::new();
        let doctor = || {
            let mut command = fixture.command(format);
            command
                .args(["inspect", "doctor", "--config"])
                .arg(fixture.root.join("missing.toml"))
                .arg("--state-root")
                .arg(&fixture.root);
            command
        };
        let control = doctor().output().unwrap();
        assert!(!control.status.success(), "{format}: {control:?}");
        assert!(
            String::from_utf8_lossy(&control.stdout).contains("configuration"),
            "{format}: {control:?}"
        );
        let closed = doctor().stdout(already_closed_stdout()).output().unwrap();
        println!(
            "doctor {format}: open reader {}, closed reader {}",
            control.status, closed.status
        );
        if closed.status.success() {
            unexpectedly_successful.push(format);
        }
    }
    assert!(
        unexpectedly_successful.is_empty(),
        "a closed report reader erased the unhealthy status in {unexpectedly_successful:?}"
    );
}

fn admin_status(format: &str, close_reader: bool) -> Output {
    admin_output(format, close_reader.then(already_closed_stdout), false)
}

fn admin_output(format: &str, sink: Option<OwnedFd>, write_credential: bool) -> Output {
    let fixture = Fixture::new();
    let token_file = fixture.root.join("synthetic-access-token");
    fs::write(&token_file, "fixture-access-token").unwrap();
    fs::set_permissions(&token_file, fs::Permissions::from_mode(0o600)).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!(
        "http://{}/api/connectors/v1",
        listener.local_addr().unwrap()
    );
    let server = thread::spawn(move || {
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
                result => panic!("admin fixture not reached: {result:?}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut request = String::new();
        let mut reader = std::io::BufReader::new(&stream);
        loop {
            let mut line = String::new();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0);
            request.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        let response = if write_credential {
            assert!(request.starts_with(
                "PUT /api/connectors/v1/admin/integrations/fixture/credentials/credential HTTP/1.1\r\n"
            ));
            let length: usize = request
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().unwrap())
                })
                .unwrap();
            let mut bytes = vec![0; length];
            reader.read_exact(&mut bytes).unwrap();
            let body: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["value"], "fixture-secret");
            assert_eq!(body["replace"], false);
            json!({
                "request_id": body["request_id"], "integration_ref": "fixture",
                "credential": "credential", "state": "present", "replaced": false
            })
        } else {
            assert!(request.starts_with("GET /api/connectors/v1/admin/integrations HTTP/1.1\r\n"));
            json!({
                "ready": true,
                "integrations": [{
                    "integration_ref": "fixture.integration", "active": true,
                    "configuration": [], "credentials": [], "ready": true
                }]
            })
        };
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer fixture-access-token\r\n"));
        let body = response.to_string();
        write!(stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nPragma: no-cache\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ).unwrap();
    });
    let mut command = fixture.command(format);
    if write_credential {
        let secret_file = fixture.root.join("synthetic-secret");
        fs::write(&secret_file, "fixture-secret").unwrap();
        fs::set_permissions(&secret_file, fs::Permissions::from_mode(0o600)).unwrap();
        command
            .args([
                "admin",
                "credentials",
                "set",
                "fixture",
                "credential",
                "--secret-file",
            ])
            .arg(secret_file)
            .args(["--endpoint", &endpoint, "--access-token-file"]);
    } else {
        command.args([
            "admin",
            "integrations",
            "status",
            "--endpoint",
            &endpoint,
            "--access-token-file",
        ]);
    }
    command
        .arg(token_file)
        .env("NO_PROXY", "127.0.0.1")
        .env("no_proxy", "127.0.0.1");
    if let Some(sink) = sink {
        command.stdout(sink);
    }
    let output = command.output().unwrap();
    server.join().unwrap();
    output
}

#[test]
fn successful_admin_results_accept_a_closed_reader_in_every_format() {
    let mut failed = Vec::new();
    for format in FORMATS {
        let control = admin_status(format, false);
        assert!(
            control.status.success(),
            "admin control {format}: {control:?}"
        );
        assert_success(&control);
        assert!(String::from_utf8_lossy(&control.stdout).contains("fixture.integration"));
        let closed = admin_status(format, true);
        println!(
            "admin {format}: open reader {}, closed reader {}; stderr: {}",
            control.status,
            closed.status,
            String::from_utf8_lossy(&closed.stderr)
        );
        if !closed.status.success() || !closed.stderr.is_empty() {
            failed.push(format);
        }
    }
    assert!(
        failed.is_empty(),
        "successful admin output rejected a closed reader in {failed:?}"
    );
}

#[test]
fn stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader() {
    for format in FORMATS {
        let fixture = Fixture::new();
        // A directory cannot supply caller JSON. The CLI must retain this input failure even
        // if the downstream reader also closes before its diagnostic is emitted.
        let directory = fs::File::open(&fixture.root).unwrap();
        let output = fixture
            .command(format)
            .args([
                "operation",
                "invoke",
                "--operation",
                "fixture.read",
                "--connection",
                "fixture.connection",
                "--description-ref",
                "fixture.description",
                "--input",
                "-",
                "--config",
            ])
            .arg(fixture.root.join("connectors.toml"))
            .arg("--state-root")
            .arg(&fixture.root)
            .stdin(directory)
            .stdout(already_closed_stdout())
            .output()
            .unwrap();
        assert!(!output.status.success(), "{format}: {output:?}");
        println!("stdin failure {format}: {}", output.status);
    }
}

#[test]
fn completion_scripts_keep_other_output_write_failures_unsuccessful() {
    let fixture = Fixture::new();
    let sink = UnixDatagram::unbound().unwrap();
    assert_ne!(
        sink.send(b"probe").unwrap_err().kind(),
        std::io::ErrorKind::BrokenPipe
    );
    let output = fixture
        .command("text")
        .args(["setup", "completions", "bash"])
        .stdout(OwnedFd::from(sink))
        .output()
        .unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("could not write the completion script")
    );
}

#[test]
fn a_healthy_doctor_accepts_a_closed_report_reader() {
    for format in FORMATS {
        let fixture = Fixture::new();
        let doctor = || {
            let mut command = fixture.command(format);
            command
                .args(["inspect", "doctor", "--config"])
                .arg(fixture.root.join("connectors.toml"))
                .arg("--state-root")
                .arg(&fixture.root);
            command
        };
        assert_success(&doctor().output().unwrap());
        assert_success(&doctor().stdout(already_closed_stdout()).output().unwrap());
    }
}

#[test]
fn each_unhealthy_report_class_keeps_its_failure_when_output_closes() {
    let mut unexpectedly_successful = Vec::new();
    for problem in [
        "malformed-config",
        "state-mode",
        "state-file",
        "socket-budget",
    ] {
        for format in FORMATS {
            let fixture = Fixture::new();
            let config = fixture.root.join("connectors.toml");
            let mut state = fixture.root.clone();
            match problem {
                "malformed-config" => fs::write(&config, "this is not TOML").unwrap(),
                "state-mode" => {
                    fs::set_permissions(&state, fs::Permissions::from_mode(0o755)).unwrap()
                }
                "state-file" => {
                    state = fixture.root.join("not-a-directory");
                    fs::write(&state, "fixture").unwrap();
                }
                "socket-budget" => state = fixture.root.join("x".repeat(100)),
                _ => unreachable!(),
            }
            let doctor = || {
                let mut command = fixture.command(format);
                command
                    .args(["inspect", "doctor", "--config"])
                    .arg(&config)
                    .arg("--state-root")
                    .arg(&state);
                command
            };
            let control = doctor().output().unwrap();
            assert!(!control.status.success(), "{problem} {format}: {control:?}");
            let closed = doctor().stdout(already_closed_stdout()).output().unwrap();
            if closed.status.success() {
                unexpectedly_successful.push((problem, format));
            }
        }
    }
    assert!(
        unexpectedly_successful.is_empty(),
        "{unexpectedly_successful:?}"
    );
}

#[test]
fn successful_admin_credential_write_accepts_a_closed_reader() {
    let mut failed = Vec::new();
    for format in FORMATS {
        let control = admin_output(format, None, true);
        assert_success(&control);
        assert!(String::from_utf8_lossy(&control.stdout).contains("present"));
        let closed = admin_output(format, Some(already_closed_stdout()), true);
        if !closed.status.success() || !closed.stderr.is_empty() {
            failed.push((format, closed.status));
        }
    }
    assert!(failed.is_empty(), "{failed:?}");
}

#[test]
fn every_admin_leaf_preserves_non_broken_pipe_output_failures() {
    for write_credential in [false, true] {
        for format in FORMATS {
            let sink = UnixDatagram::unbound().unwrap();
            assert_ne!(
                sink.send(b"probe").unwrap_err().kind(),
                std::io::ErrorKind::BrokenPipe
            );
            let output = admin_output(format, Some(sink.into()), write_credential);
            assert!(!output.status.success(), "{format}: {output:?}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains("the result could not be written")
                    || error.contains("failed printing to stdout"),
                "{error}"
            );
        }
    }
}

#[test]
fn admin_authentication_failures_remain_unsuccessful_with_a_closed_reader() {
    for format in FORMATS {
        let fixture = Fixture::new();
        let output = fixture
            .command(format)
            .args(["admin", "integrations", "status"])
            .stdout(already_closed_stdout())
            .output()
            .unwrap();
        assert!(!output.status.success(), "{format}: {output:?}");
    }
}
