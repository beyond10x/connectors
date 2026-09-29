//! Integrity conformance checks for the owner build handshake
//! (`contracts/cli/v1alpha1/owner.md`): no work request reaches an owner of
//! another build, and an owner of the same build is never refused as another.
use connectors_host::local::{
    config::Paths,
    owner::{Client, Code, WriteClient, mutation::Deadline},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    os::unix::{
        fs::{OpenOptionsExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    process::{Command, Output},
    time::{Duration, Instant},
};

fn command(root: &tempfile::TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.path().join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.path().join("state"))
        .args(args)
        .output()
        .unwrap()
}

fn configured() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = command(&root, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\n[adapters.forge.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
    root
}

fn paths(root: &tempfile::TempDir) -> Paths {
    Paths::resolve(
        Some(&root.path().join("config/config.toml")),
        Some(&root.path().join("state")),
    )
    .unwrap()
}

struct OwnerProcess(std::process::Child);
impl Drop for OwnerProcess {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

/// Starts a real `__connectors-owner` from the CLI binary. The test process
/// itself is a different executable, so every `Client` it opens is, by
/// digest, another build than this owner.
fn start_owner(root: &tempfile::TempDir) -> OwnerProcess {
    let state = root.path().join("state");
    let lock = fs::OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(state.join("owner.lock"))
        .unwrap();
    let (startup, owner_end) = UnixStream::pair().unwrap();
    let owner = Command::new("/bin/sh")
        .arg("-c")
        .arg(r#"exec "$0" __connectors-owner "$1" "$2" 3<&0 4<&1 </dev/null >/dev/null"#)
        .arg(env!("CARGO_BIN_EXE_connectors"))
        .arg(root.path().join("config/config.toml"))
        .arg(&state)
        .stdin(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            owner_end,
        )))
        .stdout(std::process::Stdio::from(lock))
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    drop(startup);
    let until = Instant::now() + Duration::from_secs(20);
    while !state.join("owner.sock").exists() {
        assert!(Instant::now() < until, "owner never listened");
        std::thread::sleep(Duration::from_millis(20));
    }
    OwnerProcess(owner)
}

fn code<T>(result: connectors_host::local::owner::Result<T>) -> Option<Code> {
    result.err().map(|error| error.code)
}

#[test]
fn every_work_request_is_refused_before_it_reaches_an_owner_of_another_build() {
    let root = configured();
    let mut owner = start_owner(&root);
    let paths = paths(&root);
    let connect = || Client::connect(&paths, false).expect("greeting is allowed across builds");
    let mismatch = Some(Code::OwnerBuildMismatch);

    assert_eq!(code(connect().status("forge")), mismatch, "status");
    let host = connect().host_incarnation.clone();
    assert_eq!(
        code(connect().stop("forge", "cfg-1", &host, &uuid::Uuid::new_v4().to_string())),
        mismatch,
        "stop"
    );
    assert_eq!(
        code(connect().begin("forge", None, None, None)),
        mismatch,
        "begin"
    );
    let call = json!({"connection":"c","operation":"o","schema":"s","revision":"r"});
    assert_eq!(
        code(connect().invoke("forge", &call, b"{}", 1_000)),
        mismatch,
        "invoke"
    );
    assert_eq!(
        code(connect().revalidate("forge", "c", "r", 1_000)),
        mismatch,
        "revalidate"
    );
    assert_eq!(
        code(WriteClient::connect(
            &paths,
            false,
            Deadline::start().unwrap()
        )),
        mismatch,
        "write"
    );

    assert!(owner.0.try_wait().unwrap().is_none(), "owner left running");
    let client = connect();
    let host = client.host_incarnation.clone();
    client.shutdown(&host).unwrap();
    assert!(owner.0.wait().unwrap().success());
}

fn sha256_hex(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_frame(stream: &mut UnixStream) -> Option<Value> {
    let mut sizes = [0u8; 12];
    stream.read_exact(&mut sizes).ok()?;
    let size =
        |offset: usize| u32::from_be_bytes(sizes[offset..offset + 4].try_into().unwrap()) as usize;
    let mut rest = vec![0; size(0) + size(4) + size(8)];
    stream.read_exact(&mut rest).ok()?;
    serde_json::from_slice(&rest[..size(0)]).ok()
}

fn write_frame(stream: &mut UnixStream, control: &Value, document: &[u8]) {
    let control = serde_json::to_vec(control).unwrap();
    let mut frame = Vec::new();
    for length in [control.len(), 0, document.len()] {
        frame.extend_from_slice(&(length as u32).to_be_bytes());
    }
    frame.extend_from_slice(&control);
    frame.extend_from_slice(document);
    stream.write_all(&frame).unwrap();
}

/// An owner of the CLI's own build that follows `transport.rs` exactly, except
/// that its first accepted connection is closed without a reply — the
/// behaviour of `handle()` when 32 clients are already being served.
#[test]
fn an_owner_of_the_same_build_that_sheds_one_connection_is_not_reported_as_another_build() {
    let root = configured();
    let socket = root.path().join("state/owner.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
    let build = sha256_hex(&fs::read(env!("CARGO_BIN_EXE_connectors")).unwrap());
    let server = std::thread::spawn(move || {
        let mut greetings = Vec::new();
        // Capacity: accepted and dropped without reading.
        drop(listener.accept().unwrap());
        #[allow(clippy::never_loop)]
        for _ in 0..3 {
            listener.set_nonblocking(false).unwrap();
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let Some(hello) = read_frame(&mut stream) else {
                break;
            };
            greetings.push(hello["build"].is_string());
            let mut reply = json!({
                "kind": "hello",
                "version": hello["version"],
                "challenge": hello["challenge"],
                "host_incarnation": uuid::Uuid::new_v4().to_string(),
                "authority": hello["authority"],
            });
            if hello["build"].is_string() {
                reply["build"] = json!(build);
            }
            write_frame(&mut stream, &reply, &[]);
            if read_frame(&mut stream).is_some() {
                write_frame(
                    &mut stream,
                    &json!({"kind":"failed","error":{"code":"unavailable"}}),
                    &[],
                );
            }
            break;
        }
        greetings
    });

    let output = command(&root, &["adapters", "status", "--adapter", "forge"]);
    let greetings = server.join().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("owner_build_mismatch"),
        "an owner running the CLI's own build was refused as another build \
         (greetings carried build: {greetings:?}): {stderr}"
    );
}
