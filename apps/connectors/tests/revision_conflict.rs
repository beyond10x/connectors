//! story:revision-conflict-wire-code. A metadata write refused because the
//! store's recorded revision moved under it (`Failure::ConcurrentRevision`)
//! definitely did not commit, so it is not an unreadable store: it has its own
//! wire code, `revision_conflict`, and a retry of the same command is safe
//! (`next_action = retry_explicitly`). An unreadable store keeps
//! `metadata_unavailable` with `next_action = retry_status`.
//!
//! The lifecycle lock serializes every metadata writer in production, so no
//! black-box sequence of CLI processes loses that race deterministically. The
//! CLI case therefore drives the production CLI against a stand-in owner of the
//! CLI's own build whose answer carries the owner's code for the conflict, and
//! the owner's own conversions are asserted directly.
use connectors_host::local::{self, owner, registry};
#[path = "support/socket.rs"]
mod sockets;
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    process::{Command, Output},
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
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.forge.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
    root
}

fn failure(output: &Output) -> Value {
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"]["data"].clone()
}

fn read_frame(stream: &mut UnixStream) -> Value {
    let mut sizes = [0u8; 12];
    stream.read_exact(&mut sizes).unwrap();
    let size = |at: usize| u32::from_be_bytes(sizes[at..at + 4].try_into().unwrap()) as usize;
    let mut rest = vec![0; size(0) + size(4) + size(8)];
    stream.read_exact(&mut rest).unwrap();
    serde_json::from_slice(&rest[..size(0)]).unwrap()
}

fn write_frame(stream: &mut UnixStream, control: &Value) {
    let control = serde_json::to_vec(control).unwrap();
    let mut frame = Vec::new();
    for length in [control.len(), 0, 0] {
        frame.extend_from_slice(&(length as u32).to_be_bytes());
    }
    frame.extend_from_slice(&control);
    stream.write_all(&frame).unwrap();
}

fn sha256_hex(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// One connection served as an owner of the CLI's own build, answering its one
/// work request with the owner's definite failure for a revision conflict.
fn stand_in_owner(root: &tempfile::TempDir) -> std::thread::JoinHandle<Vec<String>> {
    let state = root.path().join("state");
    let listener = sockets::bind(&state, "owner.sock");
    fs::set_permissions(state.join("owner.sock"), fs::Permissions::from_mode(0o600)).unwrap();
    let build = sha256_hex(&fs::read(env!("CARGO_BIN_EXE_connectors")).unwrap());
    let error =
        serde_json::to_value(owner::Error::from(registry::Failure::ConcurrentRevision)).unwrap();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let hello = read_frame(&mut stream);
        assert_eq!(hello["kind"], "hello", "{hello}");
        write_frame(
            &mut stream,
            &json!({
                "kind": "hello",
                "version": hello["version"],
                "challenge": hello["challenge"],
                "host_incarnation": uuid::Uuid::new_v4().to_string(),
                "authority": hello["authority"],
                "build": build,
            }),
        );
        let request = read_frame(&mut stream);
        write_frame(&mut stream, &json!({"kind": "failed", "error": error}));
        vec![request["kind"].as_str().unwrap_or_default().to_owned()]
    })
}

#[test]
fn the_owner_reports_a_revision_conflict_under_its_own_code() {
    for error in [
        owner::Error::from(registry::Failure::ConcurrentRevision),
        owner::Error::from(local::Failure::ConcurrentRevision),
    ] {
        assert_eq!(
            serde_json::to_value(error.code).unwrap(),
            json!("revision_conflict")
        );
    }
    for error in [
        owner::Error::from(registry::Failure::MetadataUnavailable),
        owner::Error::from(local::Failure::MetadataUnavailable),
    ] {
        assert_eq!(error.code, owner::Code::MetadataUnavailable);
    }
}

#[test]
fn a_revision_conflict_reaches_the_cli_as_revision_conflict() {
    let root = configured();
    let owner = stand_in_owner(&root);
    let output = command(
        &root,
        &[
            "adapters",
            "stop",
            "--adapter",
            "forge",
            "--expected-revision",
            "cfg-1",
            "--host-incarnation",
            &uuid::Uuid::new_v4().to_string(),
            "--child-incarnation",
            &uuid::Uuid::new_v4().to_string(),
        ],
    );
    assert_eq!(owner.join().unwrap(), ["stop"]);
    let data = failure(&output);
    assert_eq!(data["kind"], "operational", "{data}");
    assert_eq!(data["code"], "revision_conflict", "{data}");
    assert_eq!(data["stage"], "publication", "{data}");
    assert_eq!(data["next_action"], "retry_explicitly", "{data}");
}

#[test]
fn an_unreadable_store_still_answers_metadata_unavailable() {
    let root = configured();
    fs::remove_file(root.path().join("state/metadata.sqlite3")).unwrap();
    let data = failure(&command(
        &root,
        &["connections", "list", "--adapter", "forge"],
    ));
    assert_eq!(data["code"], "metadata_unavailable", "{data}");
    assert_eq!(data["stage"], "observation", "{data}");
    assert_eq!(data["next_action"], "retry_status", "{data}");
    assert!(!root.path().join("state/metadata.sqlite3").exists());
}
