//! C05 refusal classes: an operation id the adapter does not expose answers
//! `not_found`; one it exposes but the configuration does not grant answers
//! `forbidden`; a granted one selected under a stale description answers
//! `stale_description`. Admission order is existence, then grant, then
//! revision, on every path that names an operation.
use connectors_host::local::{
    config::{Config, Paths},
    registry, runtime,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Output},
};

const ABSENT: &str = "nosuch.op";
const GRANTED_READ: &str = "item.read";
const GRANTED_WRITE: &str = "item.write";
const UNGRANTED_READ: &str = "item.secret";
const UNGRANTED_WRITE: &str = "item.locked";
const REVISION: &str = "desc-1";

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

fn refusal(output: &Output) -> String {
    assert_eq!(
        output.status.code(),
        Some(1),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    error["error"]["data"]["code"].as_str().unwrap().to_owned()
}

fn bootstrap() -> runtime::Bootstrap {
    let ids = [GRANTED_READ, GRANTED_WRITE, UNGRANTED_READ, UNGRANTED_WRITE];
    let operations = ids
        .iter()
        .map(|id| connectors_core::Operation {
            id: (*id).into(),
            description: "fixture operation".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object","additionalProperties":false,
                "properties":{"name":{"type":"string"}}}),
            output_schema: json!({"type":"object","additionalProperties":false}),
        })
        .collect();
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        revision: REVISION.into(),
        operations,
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        protocol: "v1alpha1".into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: "https://fixture.invalid".into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![runtime::Profile {
            id: "token".into(),
            revision: "profile-1".into(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            minimum_scopes: BTreeSet::from(["write".into()]),
            evidence_lifetime_ms: 60_000,
            fields: vec![runtime::EntryField {
                name: "token".into(),
                label: "Token".into(),
                max_bytes: 1024,
            }],
        }],
        requirements: ids
            .iter()
            .map(|id| runtime::Requirement {
                operation: (*id).into(),
                profile: "token".into(),
                scopes: BTreeSet::new(),
                effect: if matches!(*id, GRANTED_READ | UNGRANTED_READ) {
                    runtime::Effect::Read
                } else {
                    runtime::Effect::Write
                },
            })
            .collect(),
    }
}

/// One configured adapter with a cached description of four operations, two of
/// them granted. No adapter executable exists and none is ever launched.
fn configured() -> (tempfile::TempDir, String) {
    let root = tempfile::tempdir().unwrap();
    let init = command(&root, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap()
        + &format!(
            "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.forge.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[adapters.forge.permissions]\nprofiles=['token']\noperations=['{GRANTED_READ}','{GRANTED_WRITE}']\n",
            "a".repeat(64)
        );
    fs::write(&config_path, config).unwrap();
    let paths = Paths {
        config: config_path.clone(),
        state: root.path().join("state"),
    };
    let adapter = Config::load(&paths.config).unwrap().adapters["forge"].clone();
    let bootstrap = bootstrap();
    runtime::state::State::new(&paths.state)
        .remember(&adapter.selection(), &bootstrap)
        .unwrap();
    let schema = connectors_host::local::owner::schema(&bootstrap, GRANTED_READ).unwrap();
    (root, schema)
}

fn describe(root: &tempfile::TempDir, operation: &str) -> Output {
    command(
        root,
        &[
            "operations",
            "describe",
            "--adapter",
            "forge",
            "--operation",
            operation,
        ],
    )
}

fn invoke(root: &tempfile::TempDir, operation: &str, schema: &str, revision: &str) -> Output {
    command(
        root,
        &[
            "operations",
            "invoke",
            "--adapter",
            "forge",
            "--connection",
            "conn-1",
            "--operation",
            operation,
            "--schema",
            schema,
            "--revision",
            revision,
            "--input-json",
            r#"{"name":"item"}"#,
        ],
    )
}

fn prepare(root: &tempfile::TempDir, operation: &str, schema: &str, revision: &str) -> Output {
    let input = root.path().join("input.json");
    fs::write(&input, r#"{"name":"item"}"#).unwrap();
    fs::set_permissions(&input, fs::Permissions::from_mode(0o600)).unwrap();
    command(
        root,
        &[
            "approvals",
            "prepare",
            "--adapter",
            "forge",
            "--connection",
            "conn-1",
            "--operation",
            operation,
            "--schema",
            schema,
            "--revision",
            revision,
            "--input-file",
            input.to_str().unwrap(),
        ],
    )
}

#[test]
fn describe_answers_not_found_for_an_operation_the_adapter_does_not_expose() {
    let (root, _) = configured();
    assert_eq!(refusal(&describe(&root, ABSENT)), "not_found");
    assert_eq!(refusal(&describe(&root, UNGRANTED_READ)), "forbidden");
    let granted = describe(&root, GRANTED_READ);
    assert!(granted.status.success(), "{granted:?}");
}

#[test]
fn invoke_answers_not_found_before_the_grant_and_the_revision() {
    let (root, schema) = configured();
    assert_eq!(
        refusal(&invoke(&root, ABSENT, "stale-schema", "stale-revision")),
        "not_found"
    );
    assert_eq!(
        refusal(&invoke(&root, ABSENT, &schema, REVISION)),
        "not_found"
    );
    assert_eq!(
        refusal(&invoke(
            &root,
            UNGRANTED_READ,
            "stale-schema",
            "stale-revision"
        )),
        "forbidden"
    );
    assert_eq!(
        refusal(&invoke(&root, GRANTED_READ, &schema, "stale-revision")),
        "stale_description"
    );
    assert_eq!(
        refusal(&invoke(&root, GRANTED_READ, "stale-schema", REVISION)),
        "stale_description"
    );
}

#[test]
fn approvals_prepare_answers_not_found_before_the_grant_and_the_revision() {
    let (root, _) = configured();
    assert_eq!(
        refusal(&prepare(&root, ABSENT, "stale-schema", "stale-revision")),
        "not_found"
    );
    assert_eq!(
        refusal(&prepare(
            &root,
            UNGRANTED_WRITE,
            "stale-schema",
            "stale-revision"
        )),
        "forbidden"
    );
    assert_eq!(
        refusal(&prepare(
            &root,
            GRANTED_WRITE,
            "stale-schema",
            "stale-revision"
        )),
        "stale_description"
    );
}

#[test]
fn operations_list_keeps_hiding_ungranted_operations() {
    let (root, _) = configured();
    let output = command(&root, &["operations", "list", "--adapter", "forge"]);
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let ids: Vec<&str> = value["result"]["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [GRANTED_READ, GRANTED_WRITE]);
}

// The owner's own `invoke` request repeats admission; drive it directly on the
// owner socket, since the CLI's preflight refuses before reaching it.

struct OwnerProcess(std::process::Child);
impl Drop for OwnerProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_owner(root: &tempfile::TempDir) -> OwnerProcess {
    use std::os::{
        fd::AsRawFd,
        unix::{fs::OpenOptionsExt, net::UnixStream},
    };
    let state = root.path().join("state");
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(state.join("owner.lock"))
        .unwrap();
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;
    // SAFETY: a live descriptor owned by `lock`.
    assert_eq!(unsafe { flock(lock.as_raw_fd(), LOCK_EX | LOCK_NB) }, 0);
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
    let until = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !state.join("owner.sock").exists() {
        assert!(std::time::Instant::now() < until, "owner never listened");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    OwnerProcess(owner)
}

/// Owner-channel frame: three big-endian u32 section sizes, then the sections.
fn frame(control: &Value, document: &[u8]) -> Vec<u8> {
    let bytes = serde_json::to_vec(control).unwrap();
    let mut frame = Vec::new();
    frame.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    frame.extend_from_slice(&0u32.to_be_bytes());
    frame.extend_from_slice(&(document.len() as u32).to_be_bytes());
    frame.extend_from_slice(&bytes);
    frame.extend_from_slice(document);
    frame
}

fn read_control(stream: &mut std::os::unix::net::UnixStream) -> Value {
    use std::io::Read;
    let mut sizes = [0u8; 12];
    stream.read_exact(&mut sizes).unwrap();
    let size = |at: usize| u32::from_be_bytes(sizes[at..at + 4].try_into().unwrap()) as usize;
    let mut control = vec![0; size(0)];
    stream.read_exact(&mut control).unwrap();
    let mut rest = vec![0; size(4) + size(8)];
    stream.read_exact(&mut rest).unwrap();
    serde_json::from_slice(&control).unwrap()
}

fn owner_invoke(
    root: &tempfile::TempDir,
    authority: &str,
    operation: &str,
    schema: &str,
    revision: &str,
) -> Value {
    use std::io::Write;
    let mut stream =
        std::os::unix::net::UnixStream::connect(root.path().join("state/owner.sock")).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(20)))
        .unwrap();
    let hello = json!({
        "kind": "hello", "version": "connectors-owner/1",
        "challenge": "00000000-0000-4000-8000-000000000003",
        "configuration": root.path().join("config/config.toml"), "authority": authority
    });
    stream.write_all(&frame(&hello, &[])).unwrap();
    let greeting = read_control(&mut stream);
    assert_eq!(greeting["kind"], "hello", "{greeting}");
    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        + 60_000;
    let request = json!({
        "kind": "invoke", "adapter": "forge", "connection": "conn-1",
        "operation": operation, "schema": schema, "revision": revision,
        "deadline_ms": deadline
    });
    stream
        .write_all(&frame(&request, br#"{"name":"item"}"#))
        .unwrap();
    read_control(&mut stream)
}

#[test]
fn the_owner_invoke_request_answers_not_found_before_the_grant() {
    let (root, schema) = configured();
    let _owner = start_owner(&root);
    // The authority is not public API; read it with the system sqlite3 (read-only).
    let authority = Command::new("sqlite3")
        .arg("-readonly")
        .arg(root.path().join("state/metadata.sqlite3"))
        .arg("SELECT authority_id FROM local_authority WHERE singleton=1")
        .output()
        .unwrap();
    let authority = String::from_utf8(authority.stdout)
        .unwrap()
        .trim()
        .to_owned();
    let code = |operation: &str, schema: &str, revision: &str| {
        let reply = owner_invoke(&root, &authority, operation, schema, revision);
        reply["error"]["code"].clone()
    };
    assert_eq!(
        code(ABSENT, "stale-schema", "stale-revision"),
        "not_found",
        "absent"
    );
    assert_eq!(
        code(UNGRANTED_READ, "stale-schema", "stale-revision"),
        "forbidden",
        "ungranted"
    );
    assert_eq!(
        code(GRANTED_READ, &schema, "stale-revision"),
        "stale_description",
        "stale"
    );
}
