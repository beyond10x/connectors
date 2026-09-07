use std::fs;
use std::io::{BufRead as _, Write as _};
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

struct Fixture {
    root: PathBuf,
    config: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "auth-cli-{:x}-{:x}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let config = root.join("config.toml");
        fs::write(&config,format!("[owner]\ntenant_id='fixture'\nagent_id='fixture'\nagent_revision=1\nauthority_snapshot_id='fixture'\nauthority_snapshot_sha256='{}'\n[[catalog]]\nprovider='slack'\ngrant_ref='fixture'\ninitiation='platform'\n","a".repeat(64))).unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
        Self { root, config }
    }
    fn command(&self, format: &str, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
        command
            .args(["--output", format])
            .args(args)
            .arg("--config")
            .arg(&self.config)
            .arg("--state-root")
            .arg(&self.root)
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn auth_stage2_bound_grammar_pairs_targets_and_preserves_existing_provider_mode() {
    for args in [
        vec![
            "connectors",
            "setup",
            "connect",
            "--operation",
            "fixture.write",
            "--connection",
            "connection:fixture",
            "--input-json",
            "{}",
        ],
        vec![
            "connectors",
            "setup",
            "connect",
            "gitlab",
            "--auth-profile",
            "gitlab.oauth_token",
        ],
        vec!["connectors", "setup", "connect", "slack"],
    ] {
        connectors_cli::command()
            .try_get_matches_from(args)
            .expect("the selected exact mode parses");
    }
    for args in [
        vec![
            "connectors",
            "setup",
            "connect",
            "--operation",
            "fixture.write",
        ],
        vec![
            "connectors",
            "setup",
            "connect",
            "--connection",
            "connection:fixture",
        ],
        vec![
            "connectors",
            "setup",
            "connect",
            "gitlab",
            "--operation",
            "fixture.write",
            "--connection",
            "connection:fixture",
            "--input-json",
            "{}",
        ],
        vec![
            "connectors",
            "setup",
            "connect",
            "--operation",
            "fixture.write",
            "--connection",
            "connection:fixture",
            "--input-json",
            "{}",
            "--auth-profile",
            "gitlab.oauth_token",
        ],
    ] {
        assert!(connectors_cli::command()
            .try_get_matches_from(args)
            .is_err());
    }
}

#[test]
fn auth_stage2_bound_overrides_refuse_before_waiting_for_stdin() {
    let fixture = Fixture::new();
    for override_args in [
        vec!["--label", "other"],
        vec!["--context", "other"],
        vec!["--as", "other"],
        vec!["--auth-profile", "other"],
        vec!["--set", "origin=https://example.test"],
        vec!["--allow", "writes"],
        vec!["--operator-network"],
        vec!["--credential-file", "unread"],
        vec!["--instance", "other"],
    ] {
        let mut command = fixture.command(
            "json",
            &[
                "setup",
                "connect",
                "--operation",
                "fixture.write",
                "--connection",
                "connection:fixture",
                "--input",
                "-",
            ],
        );
        command.args(override_args).stdin(Stdio::piped());
        let mut child = command.spawn().unwrap();
        let input = child.stdin.take().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        if child.try_wait().unwrap().is_none() {
            child.kill().unwrap();
            panic!("conflicting targeting reached caller input acquisition");
        }
        drop(input);
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("cannot be used with"), "{stderr}");
    }
}

#[test]
fn auth_stage2_bound_setup_requires_daemon_before_input_or_private_file() {
    let fixture = Fixture::new();
    let destination = fixture.root.join("instructions");
    let output = fixture
        .command(
            "json",
            &[
                "setup",
                "connect",
                "--operation",
                "fixture.write",
                "--connection",
                "connection:fixture",
                "--input-file",
                "missing-unread-input",
            ],
        )
        .arg("--instruction-file")
        .arg(&destination)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "daemon-required");
    assert!(!destination.exists());
    assert!(!fixture.root.join("connectors.sock").exists());
}

#[test]
fn auth_stage2_real_cli_auth_refusals_keep_every_format_private_without_resend() {
    use protocol::operation::{v3, versions};
    for format in ["text", "compact", "json", "yaml"] {
        let fixture = Fixture::new();
        let socket = fixture.root.join("connectors.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let finished = done.clone();
        let serving = std::thread::spawn(move || {
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(pair) => break pair,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if finished.load(Ordering::SeqCst) {
                            return false;
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut line = String::new();
            std::io::BufReader::new(&mut stream)
                .read_line(&mut line)
                .unwrap();
            let (version, request) = versions::decode_request(line.as_bytes()).unwrap();
            assert_eq!(version, versions::Version::V0Alpha3);
            let response = serde_json::json!({"protocol":v3::CONTRACT,"request_id":request.request_id,"status":"error","error":{"code":"authentication_required","message":"https://private.example/SYNTHETIC_PRIVATE_INSTRUCTION","retriable":false,"authentication":{"operation_ref":"fixture.write","connection_ref":"connection:fixture","integration_ref":"https://private.example/SYNTHETIC_PRIVATE_INSTRUCTION","auth_profile":"fixture.user","need":"reauthorize_existing","attempt":"not_attempted","next_action":"start_trusted_remediation"}}});
            versions::decode_response(response.to_string().as_bytes()).unwrap();
            writeln!(stream, "{response}").unwrap();
            drop(stream);
            listener.set_nonblocking(true).unwrap();
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            true
        });
        let output = fixture
            .command(
                format,
                &[
                    "operation",
                    "invoke",
                    "--operation",
                    "fixture.write",
                    "--connection",
                    "connection:fixture",
                    "--description-ref",
                    "description:fixture",
                    "--input-json",
                    "{}",
                ],
            )
            .output()
            .unwrap();
        done.store(true, Ordering::SeqCst);
        assert!(
            serving.join().unwrap(),
            "CLI exited before expected exchange: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(!output.status.success());
        let public = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        for forbidden in [
            "SYNTHETIC_PRIVATE_INSTRUCTION",
            "https://private.example",
            "integration_ref",
        ] {
            assert!(!public.contains(forbidden), "{format}");
        }
        for fact in [
            "authentication_required",
            "not_attempted",
            "reauthorize_existing",
            "start_trusted_remediation",
        ] {
            assert!(public.contains(fact), "{format}: missing {fact}");
        }
    }
}

#[test]
fn auth_stage2_operation_versions_select_the_real_exchange_without_resend() {
    use std::sync::{atomic::AtomicBool, Arc};
    let mut failures = Vec::new();
    for version in ["v2", "v3"] {
        let fixture = Fixture::new();
        let socket = fixture.root.join("connectors.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let done = Arc::new(AtomicBool::new(false));
        let finished = done.clone();
        let serving = std::thread::spawn(move || {
            let mut versions = Vec::new();
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(5)))
                            .unwrap();
                        let mut line = String::new();
                        std::io::BufReader::new(&mut stream)
                            .read_line(&mut line)
                            .unwrap();
                        let request: serde_json::Value = serde_json::from_str(&line).unwrap();
                        versions.push(request["protocol"].clone());
                        let response = serde_json::json!({"protocol":request["protocol"],"request_id":request["request_id"],"status":"error","error":{"code":"rate_limited","message":"fixture rate refusal","retriable":true,"retry_after_seconds":7}});
                        protocol::operation::versions::decode_response(
                            response.to_string().as_bytes(),
                        )
                        .unwrap();
                        writeln!(stream, "{response}").unwrap();
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if finished.load(Ordering::SeqCst) {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("{error}"),
                }
            }
            versions
        });
        let output = fixture
            .command(
                "json",
                &[
                    "operation",
                    "--protocol-version",
                    version,
                    "invoke",
                    "--operation",
                    "fixture.write",
                    "--connection",
                    "connection:fixture",
                    "--description-ref",
                    "description:fixture",
                    "--input-json",
                    "{}",
                ],
            )
            .output()
            .unwrap();
        done.store(true, Ordering::SeqCst);
        let seen = serving.join().unwrap();
        let expected = format!("b10x.connector-operation.v0alpha{}", &version[1..]);
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
        if seen != vec![serde_json::json!(expected)]
            || output.status.success()
            || value["error"]["code"] != "rate_limited"
            || value["error"]["retry_after_seconds"] != 7
        {
            failures.push(format!(
                "{version}: exchanges={seen:?}; output={value}; stderr={}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn auth_stage2_unknown_operation_version_refuses_before_input_or_socket() {
    let fixture = Fixture::new();
    let listener =
        std::os::unix::net::UnixListener::bind(fixture.root.join("connectors.sock")).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut child = fixture
        .command(
            "json",
            &[
                "operation",
                "--protocol-version",
                "v99",
                "invoke",
                "--operation",
                "fixture.write",
                "--connection",
                "connection:fixture",
                "--description-ref",
                "description:fixture",
                "--input",
                "-",
            ],
        )
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let input = child.stdin.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let acquired_input = child.try_wait().unwrap().is_none();
    if acquired_input {
        child.kill().unwrap();
    }
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert!(
        !acquired_input,
        "unknown version reached caller input acquisition"
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("invalid value")
            && stderr.contains("v99")
            && stderr.contains("v2")
            && stderr.contains("v3"),
        "{stderr}"
    );
    assert!(connectors_cli::command()
        .try_get_matches_from([
            "connectors",
            "connection",
            "--protocol-version",
            "v3",
            "list"
        ])
        .is_err());
    assert!(connectors_cli::command()
        .try_get_matches_from([
            "connectors",
            "setup",
            "connect",
            "slack",
            "--protocol-version",
            "v3"
        ])
        .is_err());
}

#[test]
fn auth_stage2_v3_refusal_keeps_failure_and_privacy_with_open_or_closed_output() {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::sync::{atomic::AtomicBool, Arc};

    for format in ["json", "yaml", "text", "compact"] {
        for closed in [false, true] {
            let fixture = Fixture::new();
            let socket = fixture.root.join("connectors.sock");
            let listener = UnixListener::bind(&socket).unwrap();
            fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
            listener.set_nonblocking(true).unwrap();
            let done = Arc::new(AtomicBool::new(false));
            let finished = done.clone();
            let server = std::thread::spawn(move || {
                let mut calls = 0;
                loop {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream
                                .set_read_timeout(Some(Duration::from_secs(5)))
                                .unwrap();
                            let mut line = String::new();
                            std::io::BufReader::new(&mut stream)
                                .read_line(&mut line)
                                .unwrap();
                            let (version, request) =
                                protocol::operation::versions::decode_request(line.as_bytes())
                                    .unwrap();
                            assert_eq!(version, protocol::operation::versions::Version::V0Alpha3);
                            assert!(matches!(
                                request.request,
                                protocol::operation::OperationRequest::Search(_)
                            ));
                            calls += 1;
                            let response = serde_json::json!({"protocol":protocol::operation::v3::CONTRACT,"request_id":request.request_id,"status":"error","error":{"code":"not_found","message":"SYNTHETIC_PRIVATE_INSTRUCTION","retriable":false}});
                            protocol::operation::versions::decode_response(
                                response.to_string().as_bytes(),
                            )
                            .unwrap();
                            writeln!(stream, "{response}").unwrap();
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if finished.load(Ordering::SeqCst) {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        Err(error) => panic!("{error}"),
                    }
                }
                calls
            });
            let mut command = fixture.command(format, &["operation", "search"]);
            if closed {
                let (writer, reader) = UnixStream::pair().unwrap();
                drop(reader);
                let writer: OwnedFd = writer.into();
                command.stdout(writer);
            }
            let output = command.output().unwrap();
            done.store(true, Ordering::SeqCst);
            assert_eq!(server.join().unwrap(), 1, "{format}, closed={closed}");
            assert!(!output.status.success(), "{format}, closed={closed}");
            let public = format!(
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                !public.contains("SYNTHETIC_PRIVATE_INSTRUCTION"),
                "{format}, closed={closed}"
            );
            if !closed {
                assert!(public.contains("not_found"), "{format}: {public}");
                assert!(
                    public.contains("the operation was not found"),
                    "{format}: {public}"
                );
            }
        }
    }
}

#[test]
fn auth_adversary_cli_unsafe_daemon_objects_refuse_before_open_stdin_or_private_destination() {
    let mut observations = Vec::new();
    for case in ["absent", "regular", "symlink"] {
        let fixture = Fixture::new();
        let socket = fixture.root.join("connectors.sock");
        let original = fixture.root.join("retained");
        fs::write(&original, "retain-fixture").unwrap();
        match case {
            "regular" => fs::write(&socket, "retain-fixture").unwrap(),
            "symlink" => std::os::unix::fs::symlink(&original, &socket).unwrap(),
            _ => {}
        }
        let destination = fixture.root.join("private-instructions");
        let mut child = fixture
            .command(
                "json",
                &[
                    "setup",
                    "connect",
                    "--operation",
                    "fixture.write",
                    "--connection",
                    "connection:fixture",
                    "--input",
                    "-",
                ],
            )
            .arg("--instruction-file")
            .arg(&destination)
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let blocked = child.try_wait().unwrap().is_none();
        if blocked {
            child.kill().unwrap();
        }
        drop(input);
        let output = child.wait_with_output().unwrap();
        observations.push((case, blocked, output.status.code()));
        assert!(
            !destination.exists(),
            "unsafe/absent daemon must not reserve human output"
        );
        assert_eq!(fs::read_to_string(&original).unwrap(), "retain-fixture");
        assert!(!output.status.success());
    }
    eprintln!("real CLI with stdin kept open; timed-out fixture children were killed and joined: {observations:?}");
    assert!(
        observations.iter().all(|(_, blocked, _)| !blocked),
        "an existing non-socket or symlink must be refused before blocking on caller stdin"
    );
}

#[test]
fn auth_adversary2_cli_socket_permissions_refuse_before_open_stdin_in_every_format() {
    for format in ["json", "yaml", "text", "compact"] {
        for (root_mode, socket_mode) in [(0o700, 0o640), (0o700, 0o606), (0o750, 0o600)] {
            let fixture = Fixture::new();
            let socket = fixture.root.join("connectors.sock");
            let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
            listener.set_nonblocking(true).unwrap();
            fs::set_permissions(&socket, fs::Permissions::from_mode(socket_mode)).unwrap();
            fs::set_permissions(&fixture.root, fs::Permissions::from_mode(root_mode)).unwrap();
            let destination = fixture.root.join("SYNTHETIC_PRIVATE_INSTRUCTION");
            let mut command = fixture.command(
                format,
                &[
                    "setup",
                    "connect",
                    "--operation",
                    "fixture.write",
                    "--connection",
                    "connection:fixture",
                    "--input",
                    "-",
                ],
            );
            command
                .arg("--instruction-file")
                .arg(&destination)
                .stdin(Stdio::piped());
            let mut child = command.spawn().unwrap();
            let input = child.stdin.take().unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            let blocked = child.try_wait().unwrap().is_none();
            if blocked {
                child.kill().unwrap();
            }
            drop(input);
            let output = child.wait_with_output().unwrap();
            eprintln!("CLI permissions {format}/{root_mode:o}/{socket_mode:o}: blocked={blocked}, exit={:?}", output.status.code());
            assert!(!blocked, "permission refusal waited for caller stdin");
            assert!(!output.status.success());
            let text = String::from_utf8_lossy(&output.stdout).to_string()
                + &String::from_utf8_lossy(&output.stderr);
            let expected_code = if root_mode == 0o700 {
                "configuration"
            } else {
                "runtime"
            };
            assert!(text.contains(expected_code), "actual refusal: {text}");
            assert!(!text.contains("SYNTHETIC_PRIVATE_INSTRUCTION"));
            assert!(!destination.exists());
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        }
    }
}
