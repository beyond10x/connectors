//! Adversary cases for story:mcp-stdio-runtime-branch, on the harness of
//! `local_mcp_server.rs` (copied; each description is padded to LONG bytes so a
//! `tools/list` reply spans several 4096-byte writes).
#![allow(dead_code)]
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors_custody::mounted::fixture::{Jail, Map};
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::{config, registry, runtime},
};
use connectors_sdk::AuthenticatedHttp;
use mounted::{ALIAS, CONFIG, Container, PROFILE, SECRET, STATE, digest, success};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpListener},
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

const ADAPTER_FIXTURE: &str = "adversary_local_mcp_server_adapter_fixture";
/// Padding of every operation description, so one listed tool is a reply
/// longer than the server's 4096-byte write and than a shrunken pipe.
const LONG: usize = 12_000;
/// Inside the jail: the loopback address of the test's provider.
const PROVIDER: &str = "/work/provider";
/// The server's companion configuration (`<config>.mcp-stdio.json`).
const COMPANION: &str = "/srv/connectors/config.toml.mcp-stdio.json";
/// The protected operation curation (`<config>.operation-curation.json`).
const CURATION: &str = "/srv/connectors/config.toml.operation-curation.json";
const INSTANCE: &str = "forge-local";
const ADAPTER: &str = "fixture";
const DESCRIPTION: &str = "desc-1";
const REVISION_META: &str = "io.beyond10x.connectors/revision";
/// Every operation the adapter describes; `ungranted` is outside the
/// configured permissions.
const OPERATIONS: [&str; 5] = ["read", "disabled", "hidden", "guarded", "ungranted"];
const GRANTED: [&str; 4] = ["read", "disabled", "hidden", "guarded"];
/// Exposed operations; `hidden` has no exposure and `disabled` is disabled.
const EXPOSED: [&str; 4] = ["read", "disabled", "guarded", "ungranted"];
/// The input whose answer the provider holds for 3 s.
const SLOW: &str = "999";
/// The input whose answer the provider holds for 8 s: longer than local
/// teardown (5 s), inside the declared 15 s provider budget.
const STALL: &str = "998";
/// An input number no f64 holds: it reaches the provider and comes back as
/// these digits only through lossless reads.
const WIDE: &str = "1844674407370955161701";

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

/// The adapter's description: five reads under one static profile, each
/// taking `{n: number}` and answering `{observed: number}`.
fn bootstrap() -> runtime::Bootstrap {
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: INSTANCE.into(),
        adapter: ADAPTER.into(),
        revision: DESCRIPTION.into(),
        operations: OPERATIONS
            .iter()
            .map(|id| connectors_core::Operation {
                id: (*id).into(),
                description: format!("fixture {id} {}", "x".repeat(LONG)),
                contract: "operations/v1alpha1".into(),
                profile: "resource".into(),
                input_schema: json!({"type":"object","properties":{"n":{"type":"number"}},"required":["n"],"additionalProperties":false}),
                output_schema: json!({"type":"object","properties":{"observed":{"type":"number"}},"required":["observed"],"additionalProperties":false}),
            })
            .collect(),
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
        requirements: OPERATIONS
            .iter()
            .map(|operation| runtime::Requirement {
                operation: (*operation).into(),
                custody: false,
                profile: PROFILE.into(),
                scopes: BTreeSet::new(),
                effect: runtime::Effect::Read,
            })
            .collect(),
    }
}

fn transport() -> &'static ScopedHttp {
    static HTTP: OnceLock<ScopedHttp> = OnceLock::new();
    HTTP.get_or_init(|| {
        let address = fs::read_to_string(PROVIDER).unwrap();
        ScopedHttp::new_with_ca_bytes(
            &HttpConfig {
                base_url: format!("http://{}/", address.trim()),
                credential: None,
                credential_header: "authorization".into(),
                bearer: true,
                allow_plaintext: true,
                ca_file: None,
            },
            None,
            None,
        )
        .unwrap()
    })
}

/// The adapter child: validation makes no provider request; a bounded read
/// is one `GET /observe/<n>` to the provider named in [`PROVIDER`], whose
/// JSON answer it returns. It has no unbounded invoke.
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
        let n = input["n"].to_string();
        let response = transport()
            .get(&["observe", &n], &[])
            .await
            .map_err(runtime::Failure::from_provider)?;
        if response.status != 200 {
            return Err(runtime::Failure::Unavailable);
        }
        connectors_core::json::decode(&response.body, 64).map_err(|_| runtime::Failure::Protocol)
    }
}

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn adversary_local_mcp_server_adapter_fixture() {
    if std::env::args().any(|arg| arg == "--connectors-private-fd") {
        let _ = runtime::serve(3, Reading);
    }
}

/// A loopback HTTP/1.1 provider answering `GET /observe/<n>` with
/// `{"observed":<n>}`, the digits as sent and any query dropped; [`SLOW`] is
/// answered after 3 s.
struct Provider {
    address: String,
    requests: Arc<AtomicUsize>,
    /// [`STALL`] answers whose connection the client still held open 8 s on.
    stalled_open: Arc<AtomicUsize>,
}

impl Provider {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(AtomicUsize::new(0));
        let counted = requests.clone();
        let stalled_open = Arc::new(AtomicUsize::new(0));
        let open = stalled_open.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let counted = counted.clone();
                let open = open.clone();
                std::thread::spawn(move || {
                    loop {
                        let mut head = Vec::new();
                        let mut byte = [0u8; 1];
                        while !head.ends_with(b"\r\n\r\n") {
                            if stream.read(&mut byte).unwrap_or(0) == 0 {
                                let _ = stream.shutdown(Shutdown::Both);
                                return;
                            }
                            head.push(byte[0]);
                        }
                        counted.fetch_add(1, Ordering::SeqCst);
                        let head = String::from_utf8_lossy(&head).into_owned();
                        let n = head
                            .strip_prefix("GET /observe/")
                            .and_then(|rest| rest.split([' ', '?']).next())
                            .unwrap_or("null")
                            .to_owned();
                        if n == SLOW {
                            std::thread::sleep(Duration::from_secs(3));
                        }
                        if n == STALL {
                            std::thread::sleep(Duration::from_secs(8));
                            stream.set_nonblocking(true).unwrap();
                            if stream
                                .peek(&mut [0u8; 1])
                                .is_err_and(|e| e.kind() == std::io::ErrorKind::WouldBlock)
                            {
                                open.fetch_add(1, Ordering::SeqCst);
                            }
                            stream.set_nonblocking(false).unwrap();
                        }
                        let body = format!("{{\"observed\":{n}}}");
                        let answer = format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
                            body.len()
                        );
                        if stream.write_all(answer.as_bytes()).is_err() {
                            return;
                        }
                    }
                });
            }
        });
        Self {
            address,
            requests,
            stalled_open,
        }
    }
    fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

/// Write `text` owner-only, as the server requires of its inputs.
fn private(path: &Path, text: String) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn clock_key(seed: u8) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode([seed; 32])
}

/// The configured adapter entry `forge`: [`Reading`] over
/// `connectors-private/3`, granted every operation but `ungranted`.
fn entry(container: &Container) -> config::Adapter {
    config::Adapter {
        instance_id: INSTANCE.into(),
        adapter_id: ADAPTER.into(),
        configuration_revision: "cfg-1".into(),
        credential_revision: None,
        protocol: "v1alpha1".into(),
        private_protocol: Some(runtime::PrivateProtocol::V3),
        startup: Default::default(),
        restart: Default::default(),
        executable: config::Executable {
            path: "/bin-test/adapter".into(),
            sha256: digest(&fs::read(container.path("/bin-test/adapter")).unwrap()),
            args: vec!["--exact".into(), ADAPTER_FIXTURE.into(), "--".into()],
        },
        permissions: config::Permissions {
            profiles: BTreeSet::from([PROFILE.to_owned()]),
            operations: GRANTED.iter().map(|op| (*op).to_owned()).collect(),
            custody: Default::default(),
        },
    }
}

/// The configuration: mounted custody, a two-source approval clock (the
/// session's rate assumption; never sampled here) and `adapter` as `forge`.
fn configure(container: &Container, adapter: &config::Adapter) {
    let uid = connectors_host::local::filesystem::uid();
    let text = format!(
        "format='connectors-local/4'\nowner_uid={uid}\n{custody}[approval_clock]\nformat='roughtime-quorum/1'\nquorum=2\nmax_rate_error_ppm=10000\n[[approval_clock.sources]]\naddress='127.0.0.1:9'\npublic_key='{key_a}'\nversion='roughtime-0x8000000c'\n[[approval_clock.sources]]\naddress='127.0.0.1:10'\npublic_key='{key_b}'\nversion='roughtime-0x8000000c'\n[adapters.{ALIAS}]\ninstance_id='{INSTANCE}'\nadapter_id='{ADAPTER}'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/3'\n[adapters.{ALIAS}.permissions]\nprofiles={profiles}\noperations={operations}\n[adapters.{ALIAS}.executable]\npath='/bin-test/adapter'\nsha256='{sha256}'\nargs={args}\n",
        custody = Container::mounted(),
        key_a = clock_key(7),
        key_b = clock_key(8),
        profiles = serde_json::to_string(&adapter.permissions.profiles).unwrap(),
        operations = serde_json::to_string(&adapter.permissions.operations).unwrap(),
        sha256 = adapter.executable.sha256,
        args = serde_json::to_string(&adapter.executable.args).unwrap(),
    );
    private(&container.path(CONFIG), text);
}

/// One container with a connection made through the mounted credential, the
/// operation curation pinned to the adapter, and every exposure written.
struct Served {
    container: Container,
    provider: Provider,
    connection: String,
}

impl Served {
    fn new() -> Self {
        let provider = Provider::start();
        let container = Container::new(ADAPTER_FIXTURE);
        fs::write(container.path(PROVIDER), &provider.address).unwrap();
        container.deliver(PROFILE, SECRET);
        let adapter = entry(&container);
        configure(&container, &adapter);
        let init = success(&container.run(&["setup", "init"]));
        assert_eq!(init["disposition"], "state_initialized", "{init}");
        let connected = success(&container.connect(PROFILE));
        let connection = connected["connection"]["summary"]["connection"]
            .as_str()
            .unwrap_or_else(|| panic!("{connected}"))
            .to_owned();
        let operations: Vec<Value> = OPERATIONS
            .iter()
            .map(|operation| {
                json!({"operation":operation,"metadata":{"effects":["network"],"semantic_effects":[],"risk":"low","idempotency":{"kind":"none"},
                    "approval":if *operation == "guarded" {"required"} else {"not_required"},"realization":"implemented",
                    "limits":{"request_bytes":65536,"result_bytes":4194304,"execution_ms":20000,"provider_ms":15000,"connect_ms":5000}}})
            })
            .collect();
        private(
            &container.path(CURATION),
            json!({"format":"connectors-operation-curation/1","adapters":[{"adapter_alias":ALIAS,
                "executable_selection":adapter.selection(),
                "bootstrap_sha256":connectors_core::digest(&serde_json::to_value(bootstrap()).unwrap()),
                "operations":operations}]})
            .to_string(),
        );
        let served = Self {
            container,
            provider,
            connection,
        };
        served.exposures(true);
        assert_eq!(served.provider.requests(), 0, "setting up sent a request");
        served
    }

    /// Write the companion: each of [`EXPOSED`] as a tool, enabled when
    /// `enabled`, except `disabled`, which never is.
    fn exposures(&self, enabled: bool) {
        let exposures: Vec<Value> = EXPOSED
            .iter()
            .map(|operation| {
                json!({"adapter_alias":ALIAS,"operation_ref":operation,"connection_ref":self.connection,
                    "families":["tools"],"enabled":*operation != "disabled" && enabled})
            })
            .collect();
        private(
            &self.container.path(COMPANION),
            json!({"format":"connectors-mcp-local/1","limits":{"frame_octets":1048576,"response_octets":33554432,
                "concurrent_requests":1,"request_milliseconds":120000},"exposures":exposures})
            .to_string(),
        );
    }

    /// The jailed server command, with an empty environment and only the
    /// two path globals.
    fn server(&self) -> Command {
        let mut command = self
            .container
            .jail
            .command("/bin-test/connectors", Map::Own);
        command
            .args(["--config", CONFIG, "--state-dir", STATE])
            .args(["server", "--transport", "stdio"])
            .env_clear();
        command
    }
}

/// The tool name of `operation` on this adapter.
fn name(operation: &str) -> String {
    connectors_mcp::names::encode_name([INSTANCE, ADAPTER, operation]).unwrap()
}

/// One JSON-RPC 2.0 response frame, or why the stdout line is not one. Only
/// an error may omit the `id`, for a request whose `id` could not be read
/// (adapters/mcp/contracts/server/v1alpha1/semantics.md § 3.1).
fn frame(line: &str) -> Result<Value, String> {
    let value: Value = serde_json::from_str(line)
        .map_err(|error| format!("protocol stdout polluted ({error}): {line}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| format!("not one JSON-RPC frame: {line}"))?;
    let answers =
        usize::from(object.contains_key("result")) + usize::from(object.contains_key("error"));
    let correlated = object.contains_key("id") || object.contains_key("error");
    if value["jsonrpc"] != "2.0" || !correlated || answers != 1 {
        return Err(format!("not one JSON-RPC response frame: {line}"));
    }
    Ok(value)
}

/// An MCP client of the jailed server: its stdin, every stdout line checked
/// by [`frame`], and its stderr apart.
struct Peer {
    child: Child,
    frames: mpsc::Receiver<Result<Value, String>>,
    lines: Arc<Mutex<Vec<String>>>,
    reader: Option<JoinHandle<()>>,
    errors: Option<JoinHandle<Vec<u8>>>,
    primary: bool,
}

impl Peer {
    /// Start a session: on `2026-07-28` (`primary`) each request carries its
    /// protocol revision; on `2025-11-25` the session is initialized first.
    fn start(served: &Served, primary: bool) -> Self {
        let mut command = served.server();
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut child = loop {
            match command.spawn() {
                Ok(child) => break child,
                Err(error)
                    if error.raw_os_error() == Some(libc::ETXTBSY) && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("{}", Jail::spawn_error(&error)),
            }
        };
        let stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let (sender, frames) = mpsc::channel();
        let lines = Arc::new(Mutex::new(Vec::new()));
        let seen = lines.clone();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                seen.lock().unwrap().push(line.clone());
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
        let mut peer = Self {
            child,
            frames,
            lines,
            reader: Some(reader),
            errors: Some(errors),
            primary,
        };
        if !primary {
            peer.send(json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-11-25",
                "capabilities":{},"clientInfo":{"name":"independent-peer","version":"1"}}}));
            let init = peer.recv();
            assert_eq!(init["result"]["protocolVersion"], "2025-11-25", "{init}");
            assert!(
                init["result"]["capabilities"]["tools"].is_object(),
                "{init}"
            );
            peer.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        }
        peer
    }
    fn send(&mut self, value: Value) {
        self.raw(format!("{value}\n").as_bytes());
    }
    fn raw(&mut self, bytes: &[u8]) {
        self.child.stdin.as_mut().unwrap().write_all(bytes).unwrap();
    }
    fn recv(&self) -> Value {
        match self.frames.recv_timeout(Duration::from_secs(15)) {
            Ok(Ok(value)) => value,
            Ok(Err(line)) => panic!("{line}"),
            Err(_) => panic!("the server did not reply"),
        }
    }
    fn request(&mut self, id: i32, method: &str, mut params: Value) -> Value {
        if self.primary {
            if !params["_meta"].is_object() {
                params["_meta"] = json!({});
            }
            params["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2026-07-28");
            params["_meta"]["io.modelcontextprotocol/clientCapabilities"] = json!({});
        }
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        let reply = self.recv();
        assert_eq!(reply["id"], id, "{reply}");
        reply
    }
    /// `tools/call` of `name` with `{"input":{"n":<n>}}` at `revision`.
    fn call(&mut self, id: i32, name: &str, n: &str, revision: &Value) -> Value {
        let input: Value = serde_json::from_str(&format!("{{\"n\":{n}}}")).unwrap();
        self.request(
            id,
            "tools/call",
            json!({"name":name,"arguments":{"input":input},"_meta":{REVISION_META:revision}}),
        )
    }
    /// Close stdin and wait for the server to exit as `success` says, with
    /// nothing more on stdout: every stdout line it wrote, and its stderr.
    fn exit(mut self, success: bool) -> (Vec<String>, Vec<u8>) {
        drop(self.child.stdin.take());
        let until = Instant::now() + Duration::from_secs(6);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert_eq!(status.success(), success, "{status}");
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
        let errors = self.errors.take().unwrap().join().unwrap();
        (self.lines.lock().unwrap().clone(), errors)
    }
    fn finish(self) -> (Vec<String>, Vec<u8>) {
        self.exit(true)
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The primary revision's per-request `_meta`.
fn meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}})
}

/// Spawn the jailed server with piped stdio, retrying a busy text file.
fn spawn(served: &Served) -> Child {
    let mut command = served.server();
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match command.spawn() {
            Ok(child) => return child,
            Err(error)
                if error.raw_os_error() == Some(libc::ETXTBSY) && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("{}", Jail::spawn_error(&error)),
        }
    }
}

/// What a client saw that shrank its stdout pipe to 4096 bytes, sent
/// `tools/list` (one tool, a reply of about 12 KB, so it is blocked
/// mid-frame), optionally cancelled it, sent a `ping`, and began reading only
/// after `stall`: whether the ping was answered, the exit status, the
/// stdout line lengths and the server's stderr.
struct Stalled {
    answered: bool,
    status: Option<std::process::ExitStatus>,
    lines: Vec<usize>,
    stderr: String,
}

fn stalled(cancel: bool, stall: Duration) -> Stalled {
    use std::os::fd::AsRawFd;
    let served = Served::new();
    let mut child = spawn(&served);
    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let errors = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    // SAFETY: F_SETPIPE_SZ on this test's own pipe read end.
    let size = unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_SETPIPE_SZ, 4096) };
    assert_eq!(size, 4096, "could not shrink the stdout pipe");
    let mut stdin = child.stdin.take().unwrap();
    let list = json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{"_meta":meta()}});
    stdin.write_all(format!("{list}\n").as_bytes()).unwrap();
    // Wait until the pipe holds exactly its 4096 bytes: the reply has begun
    // and is blocked mid-frame, unread.
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        let mut queued: libc::c_int = 0;
        // SAFETY: FIONREAD into a live c_int on this test's own pipe.
        assert_eq!(
            unsafe { libc::ioctl(stdout.as_raw_fd(), libc::FIONREAD, &mut queued) },
            0
        );
        if queued == 4096 {
            break;
        }
        assert!(
            Instant::now() < until,
            "the tools/list reply never filled the pipe ({queued} bytes)"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    if cancel {
        let cancel =
            json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":1}});
        stdin.write_all(format!("{cancel}\n").as_bytes()).unwrap();
    }
    let ping = json!({"jsonrpc":"2.0","id":2,"method":"ping","params":{"_meta":meta()}});
    stdin.write_all(format!("{ping}\n").as_bytes()).unwrap();
    std::thread::sleep(stall);
    let (sender, received) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { return };
            if sender.send(line).is_err() {
                return;
            }
        }
    });
    let until = Instant::now() + Duration::from_secs(10);
    let mut lines = Vec::new();
    let answered = loop {
        match received.recv_timeout(until.saturating_duration_since(Instant::now())) {
            Ok(line) => {
                let ping = frame(&line).is_ok_and(|v| v["id"] == 2 && v.get("result").is_some());
                lines.push(line.len());
                if ping {
                    break true;
                }
            }
            Err(_) => break false,
        }
    };
    let status = child.try_wait().unwrap();
    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    let stderr = errors.join().unwrap();
    Stalled {
        answered,
        status,
        lines,
        stderr,
    }
}

/// Control for the two cases below: a client that drains promptly gets the
/// whole `tools/list` reply and the ping.
#[test]
fn adversary_control_a_prompt_reader_gets_the_reply_and_the_ping() {
    let seen = stalled(false, Duration::from_millis(300));
    assert!(
        seen.answered,
        "exit {:?}, stdout line lengths {:?}, stderr {}",
        seen.status, seen.lines, seen.stderr
    );
}

/// adapters/mcp/contracts/server/v1alpha1/semantics.md § 3.4: a cancellation
/// naming a request whose response has already begun is "expected and is not
/// an error", and "the session stays `Ready`": withdrawing one request never
/// ends the caller's connection. Here the `tools/list` reply is partly
/// written when its cancellation arrives; a later `ping` must be answered.
#[test]
fn adversary_cancel_racing_a_partly_written_reply_keeps_the_session_ready() {
    let seen = stalled(true, Duration::from_millis(300));
    assert!(
        seen.answered,
        "a cancellation racing a partly written reply ended the session: exit {:?}, \
         stdout line lengths {:?}, stderr {}",
        seen.status, seen.lines, seen.stderr
    );
}

/// semantics.md § 3.5 bounds a response by "the earlier of the § 4.1 ceiling
/// and the live lease's own deadline", and § 3.3 renews that lease while the
/// session is `Ready`. A client that reads the rest of a `tools/list` reply
/// 2.5 s after it began, inside the configured 120 s request ceiling, must
/// get it and the ping.
#[test]
fn adversary_a_reply_drained_after_one_lease_still_arrives() {
    let seen = stalled(false, Duration::from_millis(2500));
    assert!(
        seen.answered,
        "a reply drained 2.5 s late ended the session: exit {:?}, stdout line lengths {:?}, \
         stderr {}",
        seen.status, seen.lines, seen.stderr
    );
}

/// adapters/mcp/contracts/server/v1alpha1/semantics.md § 3.3,
/// `renewed-lease-keeps-a-long-request-ready`: "a request that runs longer
/// than one lease is the normal case"; the supervisor renews and "the
/// response goes out under the renewed one". The admitted read declares
/// `execution_ms` 20000 and `provider_ms` 15000; the provider answers in 3 s.
#[test]
fn adversary_a_read_longer_than_one_lease_is_answered_and_the_session_stays_ready() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let list = peer.request(1, "tools/list", json!({}));
    let tool = list["result"]["tools"][0].clone();
    let revision = tool["_meta"][REVISION_META].clone();
    assert!(revision.is_string(), "{list}");
    peer.send(
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
        "name":tool["name"],"arguments":{"input":{"n":999}},
        "_meta":{REVISION_META:revision,"io.modelcontextprotocol/protocolVersion":"2026-07-28",
            "io.modelcontextprotocol/clientCapabilities":{}}}}),
    );
    let started = Instant::now();
    let reply = peer.frames.recv_timeout(Duration::from_secs(15));
    let elapsed = started.elapsed();
    let status = peer.child.try_wait().unwrap();
    match reply {
        Ok(Ok(reply)) => {
            assert_eq!(reply["id"], 2, "{reply}");
            assert_eq!(
                reply["result"]["structuredContent"]["status"], "success",
                "a 3 s read inside its 20 s budget was refused after {elapsed:?}: {reply}"
            );
        }
        Ok(Err(line)) => panic!("{line}"),
        Err(_) => panic!(
            "no reply to a read the provider answers in 3 s (declared execution_ms 20000, \
             provider_ms 15000): after {elapsed:?} the server is {status:?}, provider requests {}",
            served.provider.requests()
        ),
    }
    assert!(peer.request(3, "ping", json!({})).get("result").is_some());
    peer.finish();
}

/// 2025-11-25 lifecycle: between the `initialize` response and the
/// `notifications/initialized` notification a client may still send `ping`
/// ("The client SHOULD NOT send requests other than pings before the server
/// has responded to the initialize request", and ping is a liveness check
/// usable at any time). A ping in that window must be answered, not refused.
#[test]
fn adversary_legacy_ping_before_initialized_is_answered() {
    let served = Served::new();
    let mut child = spawn(&served);
    let stdout = child.stdout.take().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let init = json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-11-25",
        "capabilities":{},"clientInfo":{"name":"independent-peer","version":"1"}}});
    let ping = json!({"jsonrpc":"2.0","id":"p","method":"ping"});
    stdin
        .write_all(format!("{init}\n{ping}\n").as_bytes())
        .unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let first = frame(&lines.next().unwrap().unwrap()).unwrap();
    assert_eq!(first["id"], "init", "{first}");
    let second = frame(&lines.next().unwrap().unwrap()).unwrap();
    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(second["id"], "p", "{second}");
    assert!(
        second.get("result").is_some(),
        "a ping between initialize and initialized was refused: {second}"
    );
}

/// Round 2. The listed read tool and its revision, from `tools/list` id 1.
fn listed(peer: &mut Peer) -> (Value, Value) {
    let list = peer.request(1, "tools/list", json!({}));
    let tool = list["result"]["tools"][0]["name"].clone();
    let revision = list["result"]["tools"][0]["_meta"][REVISION_META].clone();
    assert!(tool.is_string() && revision.is_string(), "{list}");
    (tool, revision)
}

/// Send `tools/call` of `tool` with input `n`, primary revision, not waiting.
fn send_call(peer: &mut Peer, id: i32, tool: &Value, n: &str, revision: &Value) {
    let input: Value = serde_json::from_str(&format!("{{\"n\":{n}}}")).unwrap();
    peer.send(
        json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{
        "name":tool,"arguments":{"input":input},
        "_meta":{REVISION_META:revision,"io.modelcontextprotocol/protocolVersion":"2026-07-28",
            "io.modelcontextprotocol/clientCapabilities":{}}}}),
    );
}

/// semantics.md § 3.4: on `notifications/cancelled` the binding "stops
/// processing, frees what the request held, and sends no response for it".
/// After the in-flight call is cancelled, the next call must be served, not
/// refused for want of the one slot the cancelled call held.
#[test]
fn adversary_a_cancelled_call_frees_its_slot_for_the_next_call() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = listed(&mut peer);
    send_call(&mut peer, 2, &tool, SLOW, &revision);
    std::thread::sleep(Duration::from_millis(300));
    peer.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":2}}));
    std::thread::sleep(Duration::from_millis(50));
    let reply = peer.call(3, tool.as_str().unwrap(), "1", &revision);
    assert_eq!(
        reply["result"]["structuredContent"]["status"], "success",
        "the call after a cancelled one was refused: {reply}"
    );
    peer.finish();
}

/// sessions § 4.1 and docs/local-mcp-stdio.md: stdin closed with a read in
/// flight that the lease live at the close cannot carry (provider answers in
/// 3 s, lease at most 1.9 s): no reply is written after the cutoff and the
/// exit is unsuccessful, within the 5 s teardown.
#[test]
fn adversary_stdin_close_discards_a_reply_the_lease_cannot_carry() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = listed(&mut peer);
    send_call(&mut peer, 2, &tool, SLOW, &revision);
    std::thread::sleep(Duration::from_millis(100));
    let (lines, _) = peer.exit(false);
    assert_eq!(lines.len(), 1, "only the list reply: {lines:?}");
}

/// sessions § 4.1 "Teardown": by t0 + 5,000 ms local tasks are stopped and
/// joined or the session is reported `lost`. A read the provider holds for 8 s
/// must not keep the process past t0 + 5 s, and the exit must be unsuccessful.
#[test]
fn adversary_teardown_does_not_wait_for_a_read_past_five_seconds() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = listed(&mut peer);
    send_call(&mut peer, 2, &tool, STALL, &revision);
    std::thread::sleep(Duration::from_millis(100));
    let started = Instant::now();
    let (lines, _) = peer.exit(false);
    assert!(
        started.elapsed() < Duration::from_millis(5500),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(lines.len(), 1, "only the list reply: {lines:?}");
}

/// semantics.md § 3.4 with § 3.5: a call cancelled before stdin closes is
/// answered with nothing, even though replies to requests already read are
/// otherwise still written after the close; the exit is successful.
#[test]
fn adversary_a_call_cancelled_before_stdin_closes_is_never_answered() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = listed(&mut peer);
    send_call(&mut peer, 2, &tool, SLOW, &revision);
    std::thread::sleep(Duration::from_millis(100));
    peer.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":2}}));
    let (lines, _) = peer.exit(true);
    assert_eq!(lines.len(), 1, "only the list reply: {lines:?}");
}

/// The 2025-11-25 window between the `initialize` response and
/// `notifications/initialized` admits only `ping`: a `tools/list` or
/// `tools/call` sent there is refused and reaches no provider.
#[test]
fn adversary_legacy_window_admits_no_tool_request() {
    let served = Served::new();
    let mut child = spawn(&served);
    let stdout = child.stdout.take().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let init = json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-11-25",
        "capabilities":{},"clientInfo":{"name":"independent-peer","version":"1"}}});
    let list = json!({"jsonrpc":"2.0","id":"l","method":"tools/list","params":{}});
    let call = json!({"jsonrpc":"2.0","id":"c","method":"tools/call","params":{
        "name":name("read"),"arguments":{"input":{"n":1}}}});
    let ping =
        json!({"jsonrpc":"2.0","id":"p","method":"ping","params":{"_meta":{"progressToken":1}}});
    stdin
        .write_all(format!("{init}\n{list}\n{call}\n{ping}\n").as_bytes())
        .unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let mut replies = Vec::new();
    for _ in 0..4 {
        replies.push(frame(&lines.next().unwrap().unwrap()).unwrap());
    }
    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(replies[1]["id"], "l", "{:?}", replies);
    assert!(replies[1].get("error").is_some(), "{}", replies[1]);
    assert_eq!(replies[2]["id"], "c", "{:?}", replies);
    assert!(replies[2].get("error").is_some(), "{}", replies[2]);
    assert!(replies[3].get("result").is_some(), "{}", replies[3]);
    assert_eq!(served.provider.requests(), 0);
}

/// sessions § 4.1 "Teardown": "By `t0 + 5,000 ms`, stop/join local tasks,
/// release owned transport/device resources". Round 2 dropped the lease cap on
/// a call's read budget, so a read dispatched under a live lease now runs to
/// its declared budget. Once the session has ended (`lost`, exit 1, about
/// 5 s after stdin closed), the provider read it started must not still be
/// held open by the owner side 8 s after dispatch.
#[test]
#[ignore = "deferred: the owner has no per-request cancel; story:owner-cancels-a-bounded-read"]
fn adversary_a_lost_session_leaves_no_provider_read_open() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = listed(&mut peer);
    send_call(&mut peer, 2, &tool, STALL, &revision);
    let dispatched = Instant::now();
    std::thread::sleep(Duration::from_millis(100));
    let _ = peer.exit(false);
    let ended = dispatched.elapsed();
    std::thread::sleep(Duration::from_millis(9000).saturating_sub(dispatched.elapsed()));
    assert_eq!(
        served.provider.stalled_open.load(Ordering::SeqCst),
        0,
        "the session ended {ended:?} after dispatch, yet 8 s after dispatch its provider \
         read was still open, waiting for the answer"
    );
}
