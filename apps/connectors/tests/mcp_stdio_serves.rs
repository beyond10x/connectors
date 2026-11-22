//! story:mcp-stdio-on-macos-and-windows. The production `connectors server
//! --transport stdio` serves a session on every platform it supports:
//! `initialize`, `tools/list` and one `tools/call` that is answered, through
//! the real owner and an adapter child.
//!
//! The scenarios of `local_mcp_server.rs` run against the read-only
//! `mounted_secret` binding in a jail and so compile on Linux only. This one
//! uses the platform's own custody, with nothing platform-specific in the
//! test: the macOS login keychain, the Windows Credential Manager and, on
//! Linux, a disposable GNOME Secret Service the configuration selects by its
//! socket. It stores a credential in that store, so it is ignored by default
//! and run explicitly (`-- --ignored`), as the other cases that need a real
//! store are; the macOS and Windows host jobs run it by name and check its
//! count.
//!
//! The adapter child is this test binary, filtered to [`FIXTURE`]; it answers
//! a read from the credential it is given and makes no provider request, so
//! there is no network.
use connectors_host::local::{config, registry, runtime};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    sync::mpsc,
    thread::JoinHandle,
    time::{Duration, Instant},
};

const FIXTURE: &str = "mcp_stdio_serves_adapter_fixture";
const ALIAS: &str = "forge";
const INSTANCE: &str = "forge-local";
const ADAPTER: &str = "fixture";
const PROFILE: &str = "fixture.token";
const DESCRIPTION: &str = "desc-1";
const REVISION_META: &str = "io.beyond10x.connectors/revision";
const SECRET: &[u8] = b"fictional-platform-token-0001";

fn profile() -> runtime::Profile {
    runtime::Profile {
        id: PROFILE.into(),
        revision: "profile-1".into(),
        purpose: registry::Purpose::DelegatedUser,
        subject: registry::Subject::User,
        scheme: "http_bearer".into(),
        capability: "http-bearer".into(),
        minimum_scopes: BTreeSet::new(),
        evidence_lifetime_ms: 60_000,
        fields: vec![runtime::EntryField {
            name: "token".into(),
            label: "Token".into(),
            max_bytes: 1024,
        }],
        acquisition: None,
    }
}

/// The adapter's description: one read taking `{n: number}` and answering
/// `{observed: number}`.
fn bootstrap() -> runtime::Bootstrap {
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: INSTANCE.into(),
        adapter: ADAPTER.into(),
        revision: DESCRIPTION.into(),
        operations: vec![connectors_core::Operation {
            id: "read".into(),
            description: "fixture read".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object","properties":{"n":{"type":"number"}},"required":["n"],"additionalProperties":false}),
            output_schema: json!({"type":"object","properties":{"observed":{"type":"number"}},"required":["observed"],"additionalProperties":false}),
        }],
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        bindings: None,
        credential_revision: None,
        executable: None,
        instance: INSTANCE.into(),
        adapter: ADAPTER.into(),
        protocol: "v1alpha1".into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: "https://fixture.invalid".into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![profile()],
        requirements: vec![runtime::Requirement {
            operation: "read".into(),
            custody: false,
            profile: PROFILE.into(),
            scopes: BTreeSet::new(),
            effect: runtime::Effect::Read,
        }],
    }
}

struct Reading;

#[async_trait::async_trait]
impl runtime::Adapter for Reading {
    fn bootstrap(&self) -> runtime::Bootstrap {
        bootstrap()
    }
    fn bootstrap_v3(&self) -> runtime::Result<runtime::Bootstrap> {
        Ok(bootstrap())
    }
    async fn validate(
        &self,
        _: &str,
        _: connectors_sdk::Secret,
    ) -> runtime::Result<runtime::Baseline> {
        let now = connectors_sdk::now_ms();
        Ok(runtime::Baseline {
            binding: None,
            identity: registry::ExternalIdentity {
                kind: "fixture.user".into(),
                subject: "fixture-subject-1".into(),
            },
            granted_scopes: None,
            credential_expires_at_ms: None,
            collected_at_ms: now,
            valid_until_ms: now + 50_000,
        })
    }
    async fn invoke(
        &self,
        _: &str,
        _: &str,
        _: connectors_sdk::Secret,
        _: Value,
    ) -> runtime::Result<Value> {
        Err(runtime::Failure::Unsupported)
    }
    async fn invoke_bounded(
        &self,
        operation: &str,
        _: &str,
        secret: connectors_sdk::Secret,
        input: Value,
        budget: runtime::ReadBudget,
    ) -> runtime::Result<Value> {
        budget.until()?;
        if operation != "read" || secret.0 != SECRET {
            return Err(runtime::Failure::Forbidden);
        }
        Ok(json!({"observed": input["n"]}))
    }
}

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn mcp_stdio_serves_adapter_fixture() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(at) = args.iter().position(|arg| arg == "--connectors-private-fd") {
        let fd: i32 = args[at + 1].parse().unwrap();
        let _ = runtime::serve(fd, Reading);
    }
}

fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Write `text` readable by the owner alone, as the server requires of its inputs.
fn private(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}

fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    value["result"].clone()
}

fn clock_key(seed: u8) -> String {
    use base64::Engine;
    use ring::signature::KeyPair;
    let pair = ring::signature::Ed25519KeyPair::from_seed_unchecked(&[seed; 32]).unwrap();
    base64::engine::general_purpose::STANDARD.encode(pair.public_key().as_ref())
}

/// A configured instance with one connection, its curation and its exposure,
/// in a directory of its own.
struct Served {
    root: tempfile::TempDir,
    #[cfg(target_os = "linux")]
    _keyring: connectors_custody::secret_service::fixture::Fixture,
}

impl Served {
    fn config(&self) -> PathBuf {
        self.root.path().join("config/config.toml")
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_connectors"))
            .args(["--output", "json", "--config"])
            .arg(self.config())
            .arg("--state-dir")
            .arg(self.root.path().join("state"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    fn new() -> Self {
        #[cfg(target_os = "linux")]
        let keyring = connectors_custody::secret_service::fixture::Fixture::new();
        let served = Self {
            root: tempfile::tempdir().unwrap(),
            #[cfg(target_os = "linux")]
            _keyring: keyring,
        };
        success(&served.run(&["setup", "init"]));
        let executable = std::env::current_exe().unwrap();
        let adapter = config::Adapter {
            instance_id: INSTANCE.into(),
            adapter_id: ADAPTER.into(),
            configuration_revision: "cfg-1".into(),
            credential_revision: None,
            protocol: "v1alpha1".into(),
            private_protocol: Some(runtime::PrivateProtocol::V3),
            startup: Default::default(),
            restart: Default::default(),
            executable: config::Executable {
                sha256: digest(&fs::read(&executable).unwrap()),
                path: executable,
                args: vec!["--exact".into(), FIXTURE.into(), "--".into()],
            },
            permissions: config::Permissions {
                profiles: BTreeSet::from([PROFILE.to_owned()]),
                operations: BTreeSet::from(["read".to_owned()]),
                custody: Default::default(),
            },
        };
        let original = fs::read_to_string(served.config()).unwrap();
        let mut text = original.replace("connectors-local/5", "connectors-local/4");
        #[cfg(target_os = "linux")]
        {
            let socket = served._keyring.socket.display();
            text = text.replacen('\n', &format!("\nsecret_service_socket='{socket}'\n"), 1);
        }
        text += &format!(
            "\n[approval_clock]\nformat='roughtime-quorum/1'\nquorum=2\nmax_rate_error_ppm=10000\n[[approval_clock.sources]]\naddress='127.0.0.1:9'\npublic_key='{}'\nversion='roughtime-0x8000000c'\n[[approval_clock.sources]]\naddress='127.0.0.1:10'\npublic_key='{}'\nversion='roughtime-0x8000000c'\n[adapters.{ALIAS}]\ninstance_id='{INSTANCE}'\nadapter_id='{ADAPTER}'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/3'\n[adapters.{ALIAS}.permissions]\nprofiles={}\noperations={}\n[adapters.{ALIAS}.executable]\npath='{}'\nsha256='{}'\nargs={}\n",
            clock_key(7),
            clock_key(8),
            serde_json::to_string(&adapter.permissions.profiles).unwrap(),
            serde_json::to_string(&adapter.permissions.operations).unwrap(),
            adapter.executable.path.display(),
            adapter.executable.sha256,
            serde_json::to_string(&adapter.executable.args).unwrap(),
        );
        private(&served.config(), &text);
        let credential = served.root.path().join("credential");
        private(&credential, std::str::from_utf8(SECRET).unwrap());
        let connected = success(&served.run(&[
            "connections",
            "connect",
            "--adapter",
            ALIAS,
            "--profile",
            PROFILE,
            "--credential-file",
            credential.to_str().unwrap(),
        ]));
        let connection = connected["connection"]["summary"]["connection"]
            .as_str()
            .unwrap_or_else(|| panic!("{connected}"))
            .to_owned();
        let companion = |suffix: &str| {
            let mut path = served.config().into_os_string();
            path.push(suffix);
            PathBuf::from(path)
        };
        private(
            &companion(".operation-curation.json"),
            &json!({"format":"connectors-operation-curation/1","adapters":[{"adapter_alias":ALIAS,
                "executable_selection":adapter.selection(),
                "bootstrap_sha256":connectors_core::digest(&serde_json::to_value(bootstrap()).unwrap()),
                "operations":[{"operation":"read","metadata":{"effects":["network"],"semantic_effects":[],"risk":"low","idempotency":{"kind":"none"},
                    "approval":"not_required","realization":"implemented",
                    "limits":{"request_bytes":65536,"result_bytes":4194304,"execution_ms":20000,"provider_ms":15000,"connect_ms":5000}}}]}]})
            .to_string(),
        );
        private(
            &companion(".mcp-stdio.json"),
            &json!({"format":"connectors-mcp-local/1","limits":{"frame_octets":1048576,"response_octets":33554432,
                "concurrent_requests":1,"request_milliseconds":120000},
                "exposures":[{"adapter_alias":ALIAS,"operation_ref":"read","connection_ref":connection,
                    "families":["tools"],"enabled":true}]})
            .to_string(),
        );
        served
    }

    fn server(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
        command
            .arg("--config")
            .arg(self.config())
            .arg("--state-dir")
            .arg(self.root.path().join("state"))
            .args(["server", "--transport", "stdio"]);
        command
    }
}

impl Drop for Served {
    /// The owner the first command started outlives the CLI by design.
    fn drop(&mut self) {
        let _ = self.run(&["setup", "owner", "stop"]);
    }
}

/// One JSON-RPC 2.0 response frame, or why the stdout line is not one.
fn frame(line: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(line)
        .map_err(|error| format!("protocol stdout polluted ({error}): {line}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| format!("not one JSON-RPC frame: {line}"))?;
    let answers =
        usize::from(object.contains_key("result")) + usize::from(object.contains_key("error"));
    if value["jsonrpc"] != "2.0" || !object.contains_key("id") || answers != 1 {
        return Err(format!("not one JSON-RPC response frame: {line}"));
    }
    Ok(value)
}

/// An MCP client of the server: its stdin, every stdout line checked by
/// [`frame`], and its stderr apart.
struct Peer {
    child: Child,
    frames: mpsc::Receiver<Result<Value, String>>,
    reader: Option<JoinHandle<()>>,
    errors: Option<JoinHandle<Vec<u8>>>,
}

impl Peer {
    fn start(served: &Served) -> Self {
        let mut child = served
            .server()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let (sender, frames) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if sender.send(frame(&line)).is_err() {
                    return;
                }
            }
        });
        let errors = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stderr.read_to_end(&mut bytes);
            bytes
        });
        Self {
            child,
            frames,
            reader: Some(reader),
            errors: Some(errors),
        }
    }
    fn send(&mut self, value: Value) {
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(format!("{value}\n").as_bytes()).unwrap();
        stdin.flush().unwrap();
    }
    fn recv(&self) -> Value {
        match self.frames.recv_timeout(Duration::from_secs(60)) {
            Ok(Ok(value)) => value,
            Ok(Err(line)) => panic!("{line}"),
            Err(_) => panic!("the server did not reply"),
        }
    }
    fn request(&mut self, id: i32, method: &str, params: Value) -> Value {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        let reply = self.recv();
        assert_eq!(reply["id"], id, "{reply}");
        reply
    }
    /// Close stdin and wait for the server to exit cleanly with nothing more
    /// on stdout.
    fn finish(mut self) {
        drop(self.child.stdin.take());
        let until = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "{status}");
                break;
            }
            assert!(Instant::now() < until, "the stdio server did not exit");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            self.frames
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "unsolicited output"
        );
        self.reader.take().unwrap().join().unwrap();
        self.errors.take().unwrap().join().unwrap();
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        // A failed test shows why the server ended: its stderr, which a
        // passing test has already taken.
        if let Some(errors) = self.errors.take().filter(|_| std::thread::panicking()) {
            eprintln!(
                "server stderr: {}",
                String::from_utf8_lossy(&errors.join().unwrap_or_default())
            );
        }
    }
}

#[test]
#[ignore = "stores a credential in the platform's custody; run explicitly"]
fn the_stdio_server_serves_a_session_and_answers_a_call() {
    let served = Served::new();
    let mut peer = Peer::start(&served);
    let init = peer.request(
        1,
        "initialize",
        json!({"protocolVersion":"2025-11-25","capabilities":{},
            "clientInfo":{"name":"independent-peer","version":"1"}}),
    );
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25", "{init}");
    assert!(
        init["result"]["capabilities"]["tools"].is_object(),
        "{init}"
    );
    peer.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    let list = peer.request(2, "tools/list", json!({}));
    let tools = list["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("{list}"));
    assert_eq!(tools.len(), 1, "{list}");
    let tool = &tools[0];
    assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");
    let revision = tool["_meta"][REVISION_META].clone();
    assert!(revision.is_string(), "{tool}");
    let reply = peer.request(
        3,
        "tools/call",
        json!({"name":tool["name"],"arguments":{"input":{"n":42}},"_meta":{REVISION_META:revision}}),
    );
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    let envelope = &reply["result"]["structuredContent"];
    assert_eq!(envelope["status"], "success", "{reply}");
    assert_eq!(envelope["result"]["observed"], 42, "{reply}");
    assert_eq!(envelope["audit_status"], "complete", "{reply}");
    peer.finish();
}
