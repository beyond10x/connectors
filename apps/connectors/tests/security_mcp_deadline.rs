//! Security review conformance cases for story:mcp-tool-deadline-keeps-the-server, on the
//! harness of `adversary_local_mcp_server.rs` (copied; each description is padded to LONG
//! bytes so a `tools/list` reply spans several 4096-byte writes).
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

const ADAPTER_FIXTURE: &str = "security_mcp_deadline_adapter_fixture";
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
fn security_mcp_deadline_adapter_fixture() {
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
        self.exposures_within(enabled, 120_000);
    }

    /// [`Served::exposures`] with the request budget `request_milliseconds`.
    fn exposures_within(&self, enabled: bool, request_milliseconds: u64) {
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
                "concurrent_requests":1,"request_milliseconds":request_milliseconds},"exposures":exposures})
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

/// The state store's lifecycle lock, taken by the test (waiting at most 5 s)
/// and released when the returned file is dropped.
fn hold_store(served: &Served) -> fs::File {
    use std::os::fd::AsRawFd;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(served.container.path(STATE).join("metadata.lock"))
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    // SAFETY: lock owns a live descriptor; dropping it releases the lock.
    while unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        assert!(
            Instant::now() < until,
            "the store lock stayed taken for 5 s"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    lock
}

/// The refusal code a reply carries, or `result` for a success.
fn outcome(reply: &Value) -> String {
    if reply.get("result").is_some() {
        "result".into()
    } else {
        reply["error"]["data"]["error"]["code"]
            .as_str()
            .unwrap_or("none")
            .to_owned()
    }
}

/// story:mcp-tool-deadline-keeps-the-server, conformance: several requests at
/// once while the owner cannot answer. The list holding the request slot
/// passes its deadline; the others are refused at once. Every request gets
/// exactly one reply on its own id (a string id included), the late one is
/// `timeout`, a second deadline in a row also leaves the session up, and the
/// session ends cleanly with no unsolicited frame.
#[test]
fn security_requests_past_their_deadline_each_get_one_reply_on_their_own_id() {
    let served = Served::new();
    served.exposures_within(true, 2000);
    let mut peer = Peer::start(&served, true);
    assert!(peer.request(1, "ping", json!({})).get("result").is_some());
    let lock = hold_store(&served);
    let batch: String = [
        json!({"jsonrpc":"2.0","id":"late-1","method":"tools/list","params":{"_meta":meta()}}),
        json!({"jsonrpc":"2.0","id":11,"method":"tools/list","params":{"_meta":meta()}}),
        json!({"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"_meta":meta(),
            "name":name("read"),"arguments":{"input":{"n":1}}}}),
        json!({"jsonrpc":"2.0","id":13,"method":"ping","params":{"_meta":meta()}}),
    ]
    .iter()
    .map(|frame| format!("{frame}\n"))
    .collect();
    peer.raw(batch.as_bytes());
    let mut seen = Vec::new();
    for _ in 0..4 {
        let reply = peer.recv();
        seen.push((reply["id"].clone(), outcome(&reply)));
    }
    assert!(
        peer.frames
            .recv_timeout(Duration::from_millis(300))
            .is_err(),
        "a fifth reply to four requests: {seen:?}"
    );
    seen.sort_by_key(|(id, _)| id.to_string());
    assert_eq!(
        seen,
        vec![
            (json!("late-1"), "timeout".to_owned()),
            (json!(11), "capacity".to_owned()),
            (json!(12), "capacity".to_owned()),
            (json!(13), "result".to_owned()),
        ],
        "each request answered once, on its own id"
    );
    let again = peer.request(21, "tools/list", json!({}));
    assert_eq!(outcome(&again), "timeout", "{again}");
    assert!(
        peer.child.try_wait().unwrap().is_none(),
        "the session ended at a second deadline"
    );
    drop(lock);
    let next = peer.request(31, "tools/list", json!({}));
    assert_eq!(outcome(&next), "result", "{next}");
    peer.finish();
}

/// The request budget of the paused cases: the least that still lists the tools.
const BUDGET: Duration = Duration::from_millis(20_000);

/// What a client saw that shrank its stdout pipe to 4096 bytes, sent a
/// `tools/list` while the store was held, released the store after `release`,
/// and began reading only `read_at` after sending, past the request budget
/// `budget`. A budget under the operations' 20 s `execution_ms` lists no tool
/// (projection.rs `limits`), so the cases below use 20 s.
struct Paused {
    /// Whether the list's reply had begun (the pipe full, the reply blocked
    /// mid-frame) before the request's deadline: the reply was on time.
    began_on_time: bool,
    reply: Option<Value>,
    pinged: bool,
    status: Option<std::process::ExitStatus>,
    lines: Vec<String>,
    stderr: String,
}

fn paused(budget: Duration, release: Duration, read_at: Duration) -> Paused {
    use std::os::fd::AsRawFd;
    let served = Served::new();
    served.exposures_within(true, budget.as_millis() as u64);
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
    let fd = stdout.as_raw_fd();
    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let ping = json!({"jsonrpc":"2.0","id":1,"method":"ping","params":{"_meta":meta()}});
    stdin.write_all(format!("{ping}\n").as_bytes()).unwrap();
    let mut first = String::new();
    reader.read_line(&mut first).unwrap();
    assert!(
        frame(first.trim_end()).is_ok_and(|v| v["id"] == 1 && v.get("result").is_some()),
        "the first ping was not answered: {first}"
    );
    let lock = hold_store(&served);
    let list = json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":meta()}});
    let sent = Instant::now();
    stdin.write_all(format!("{list}\n").as_bytes()).unwrap();
    std::thread::sleep(release);
    drop(lock);
    let mut began_on_time = false;
    while sent.elapsed() < budget - Duration::from_millis(100) {
        let mut queued: libc::c_int = 0;
        // SAFETY: FIONREAD into a live c_int on this test's own pipe.
        assert_eq!(unsafe { libc::ioctl(fd, libc::FIONREAD, &mut queued) }, 0);
        if queued == 4096 {
            began_on_time = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    std::thread::sleep((sent + read_at).saturating_duration_since(Instant::now()));
    let (sender, received) = mpsc::channel();
    std::thread::spawn(move || {
        for line in reader.lines() {
            let Ok(line) = line else { return };
            if sender.send(line).is_err() {
                return;
            }
        }
    });
    let mut lines = Vec::new();
    let mut wait = |id: i32, within: Duration| -> Option<Value> {
        let until = Instant::now() + within;
        loop {
            let line = received
                .recv_timeout(until.saturating_duration_since(Instant::now()))
                .ok()?;
            lines.push(format!(
                "{} bytes after {:?}: {}",
                line.len(),
                sent.elapsed(),
                line.chars().take(400).collect::<String>()
            ));
            if let Ok(value) = frame(&line)
                && value["id"] == id
            {
                return Some(value);
            }
        }
    };
    let reply = wait(2, Duration::from_secs(10));
    let ping = json!({"jsonrpc":"2.0","id":3,"method":"ping","params":{"_meta":meta()}});
    let pinged = stdin.write_all(format!("{ping}\n").as_bytes()).is_ok()
        && wait(3, Duration::from_secs(10)).is_some();
    let status = child.try_wait().unwrap();
    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    Paused {
        began_on_time,
        reply,
        pinged,
        status,
        lines,
        stderr: errors.join().unwrap(),
    }
}

/// Control: the owner is held past the 20 s deadline, so the list's reply is
/// the late `timeout` refusal. The client that starts reading 0.5 s after the
/// deadline gets it, and the session answers the next ping.
#[test]
fn security_control_a_late_reply_to_a_paused_reader_keeps_the_session() {
    let seen = paused(
        BUDGET,
        BUDGET + Duration::from_millis(600),
        BUDGET + Duration::from_millis(500),
    );
    let reply = seen.reply.as_ref().map(outcome);
    assert!(
        reply.as_deref() == Some("timeout") && seen.pinged,
        "reply {reply:?}, pinged {}, exit {:?}, stdout {:?}, stderr {}",
        seen.pinged,
        seen.status,
        seen.lines,
        seen.stderr
    );
}

/// The same client, the same pause, but the store is released at once, so
/// the owner answers inside the deadline and the reply (several 12 KB tools)
/// is blocked mid-frame in the 4096-byte pipe when the deadline passes. The
/// request is past its deadline with its reply unwritten, as in the control;
/// the only difference is that the owner was faster. The brief's invariant: a
/// request past its deadline never ends the session and never leaves the
/// client without a reply. (A store held to 1.5 s before the deadline did not
/// build this: the owner still answered `timeout`.) Should the reply not have
/// begun before the deadline, the case proves nothing; it says so and passes.
#[test]
fn security_an_on_time_reply_written_across_its_deadline_keeps_the_session() {
    let seen = paused(BUDGET, Duration::ZERO, BUDGET + Duration::from_millis(500));
    if !seen.began_on_time {
        eprintln!(
            "scenario not built: the reply did not begin before the deadline: {:?}",
            seen.lines
        );
        return;
    }
    let reply = seen.reply.as_ref().map(outcome);
    eprintln!(
        "on time: reply {reply:?}, pinged {}, exit {:?}",
        seen.pinged, seen.status
    );
    assert!(
        reply.is_some() && seen.pinged,
        "a reply that began inside its deadline ended the session when its write \
         crossed the deadline: reply {reply:?}, pinged {}, exit {:?}, stdout {:?}, stderr {}",
        seen.pinged,
        seen.status,
        seen.lines,
        seen.stderr
    );
}
