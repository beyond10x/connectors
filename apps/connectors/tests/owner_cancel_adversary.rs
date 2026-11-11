//! Adversary cases for story:owner-cancels-a-bounded-read, on the harness of
//! `adversary_local_mcp_server.rs` (copied). The adapter fixture here also
//! sleeps `/work/start_delay_ms` before its handshake, so a launch can be held
//! open while a call is cancelled.
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
use mounted::{ALIAS, CONFIG, Container, PROFILE, SECRET, SOURCE, STAMP, STATE, digest, success};
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

const ADAPTER_FIXTURE: &str = "owner_cancel_adversary_adapter_fixture";
/// Inside the jail: milliseconds the adapter child sleeps before serving.
const START_DELAY: &str = "/work/start_delay_ms";
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
/// The input whose answer the provider holds for 25 s: longer than the 20 s
/// execution budget of a read.
const HOLD: &str = "997";
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
fn owner_cancel_adversary_adapter_fixture() {
    if std::env::args().any(|arg| arg == "--connectors-private-fd") {
        if let Some(ms) = fs::read_to_string(START_DELAY)
            .ok()
            .and_then(|text| text.trim().parse::<u64>().ok())
        {
            std::thread::sleep(Duration::from_millis(ms));
        }
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
                        if n == HOLD {
                            std::thread::sleep(Duration::from_secs(25));
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

/// `notifications/cancelled` naming `id`.
fn cancel(peer: &mut Peer, id: i32) {
    peer.send(
        json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":id}}),
    );
}

/// Wait up to 10 s until the provider has seen `count` requests.
fn reached(served: &Served, count: usize) {
    reached_within(served, count, Duration::from_secs(10));
}

/// [`reached`] waiting up to `within`.
fn reached_within(served: &Served, count: usize, within: Duration) {
    let until = Instant::now() + within;
    while served.provider.requests() < count {
        assert!(
            Instant::now() < until,
            "the provider saw {} requests, never {count}",
            served.provider.requests()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The instance's observed state, as `instances status` answers it.
fn instance_state(served: &Served) -> Value {
    let status = success(
        &served
            .container
            .run(&["instances", "status", "--adapter", ALIAS]),
    );
    status["observation"]["state"].clone()
}

/// The pids of this container's adapter children: this binary run as the
/// fixture with a private channel, rooted in the container's jail.
fn adapters(served: &Served) -> Vec<i32> {
    let root = served.container.path("/");
    let mut pids = Vec::new();
    for entry in fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
        else {
            continue;
        };
        let Ok(command) = fs::read(entry.path().join("cmdline")) else {
            continue;
        };
        let words: Vec<&[u8]> = command.split(|byte| *byte == 0).collect();
        if !words.contains(&ADAPTER_FIXTURE.as_bytes())
            || !words.contains(&&b"--connectors-private-fd"[..])
        {
            continue;
        }
        let jailed = fs::read_link(entry.path().join("root")).is_ok_and(|at| {
            let at = at.to_string_lossy();
            at.strip_suffix(" (deleted)").unwrap_or(&at) == root.to_string_lossy()
        });
        if jailed {
            pids.push(pid);
        }
    }
    pids
}

/// Whether `pid` still has a process table entry, a zombie included.
fn present(pid: i32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

/// An inotify watch for opens of one file: every open of its inode, through
/// any mount of it, is counted.
struct Opens(std::os::fd::OwnedFd);

impl Opens {
    fn watch(path: &Path) -> Self {
        use std::os::{fd::FromRawFd, unix::ffi::OsStrExt};
        // SAFETY: plain syscall; the descriptor is owned below.
        let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        assert!(fd >= 0, "inotify_init1");
        // SAFETY: fd was just returned by inotify_init1 and is owned only here.
        let owned = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) };
        let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: a live inotify descriptor and a NUL-terminated path.
        let wd = unsafe { libc::inotify_add_watch(fd, name.as_ptr(), libc::IN_OPEN) };
        assert!(wd >= 0, "inotify_add_watch {}", path.display());
        Self(owned)
    }
    /// The opens seen since the last call.
    fn take(&self) -> usize {
        use std::os::fd::AsRawFd;
        let mut opens = 0;
        let mut buffer = [0u8; 4096];
        loop {
            // SAFETY: reads into this live buffer of its own length.
            let read =
                unsafe { libc::read(self.0.as_raw_fd(), buffer.as_mut_ptr().cast(), buffer.len()) };
            if read <= 0 {
                return opens;
            }
            let read = read as usize;
            let mut offset = 0;
            while offset + 16 <= read {
                let mask = u32::from_ne_bytes(buffer[offset + 4..offset + 8].try_into().unwrap());
                let len = u32::from_ne_bytes(buffer[offset + 12..offset + 16].try_into().unwrap())
                    as usize;
                if mask & libc::IN_OPEN != 0 {
                    opens += 1;
                }
                offset += 16 + len;
            }
        }
    }
}

/// owner.md (added by this unit): a cancelled read's job "running ends its
/// adapter process, so its provider call and credential read end with it";
/// the story's outcome: "No provider or credential read is open after the
/// server process has exited". A read cancelled while its instance's adapter
/// is still launching (here held 5 s by the fixture) must not go on to read
/// the stored credential once the launch ends.
#[test]
fn a_read_cancelled_during_its_launch_reads_no_credential_after_the_cancel() {
    let served = Served::new();
    let item = served
        .container
        .path(SOURCE)
        .join(STAMP)
        .join(ALIAS)
        .join(PROFILE);
    let opens = Opens::watch(&item);
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = listed(&mut peer);
    // Control: an ordinary read opens the stored credential.
    let _ = opens.take();
    let reply = peer.call(2, tool.as_str().unwrap(), "1", &revision);
    assert_eq!(
        reply["result"]["structuredContent"]["status"], "success",
        "{reply}"
    );
    assert!(
        opens.take() >= 1,
        "the watch saw no credential open of a read"
    );
    // Hold every later launch for 5 s, then end the running adapter process
    // through a cancel of a read in flight, so the next read launches anew.
    fs::write(served.container.path(START_DELAY), "5000").unwrap();
    send_call(&mut peer, 3, &tool, HOLD, &revision);
    reached(&served, 2);
    std::thread::sleep(Duration::from_millis(100));
    cancel(&mut peer, 3);
    std::thread::sleep(Duration::from_millis(1000));
    // The next read waits on its launch; it is cancelled, and the session ends.
    send_call(&mut peer, 4, &tool, "1", &revision);
    std::thread::sleep(Duration::from_millis(1500));
    let _ = opens.take();
    cancel(&mut peer, 4);
    let cancelled = Instant::now();
    peer.finish();
    // Past the held launch (5 s) and well inside the read's 20 s budget.
    std::thread::sleep(Duration::from_secs(7).saturating_sub(cancelled.elapsed()));
    assert_eq!(
        served.provider.requests(),
        2,
        "the cancelled read reached the provider"
    );
    assert_eq!(
        opens.take(),
        0,
        "the stored credential was opened after its read was cancelled and its server exited"
    );
}

/// contracts/cli/v1alpha1/semantics.md: an instance is `failed` on "observed
/// unexpected termination". A cancel ending the adapter process on purpose
/// (owner.md, story:owner-cancels-a-bounded-read) is no such termination: the
/// instance is not reported `failed` after it.
#[test]
fn a_cancelled_read_does_not_leave_its_instance_failed() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = listed(&mut peer);
    send_call(&mut peer, 2, &tool, HOLD, &revision);
    reached(&served, 1);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        instance_state(&served),
        "ready",
        "control: a read in flight"
    );
    cancel(&mut peer, 2);
    std::thread::sleep(Duration::from_millis(1500));
    let state = instance_state(&served);
    assert_ne!(
        state, "failed",
        "an instance whose read was cancelled is reported failed"
    );
    peer.finish();
}

/// owner.md: after a cancel "the next job of the instance starts at once
/// rather than at the read's budget". Measured against a control, as the
/// machine is loaded: a call that
/// must relaunch the adapter (its process killed from outside) measures a
/// cold launch; the call after a cancelled read, which relaunches it too, may
/// take that long and 1.5 s more, not the cancelled read's remaining budget.
#[test]
fn the_call_after_a_cancelled_read_waits_no_longer_than_a_relaunch() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = listed(&mut peer);
    let warm = peer.call(2, tool.as_str().unwrap(), "1", &revision);
    assert_eq!(
        warm["result"]["structuredContent"]["status"], "success",
        "{warm}"
    );
    let running = adapters(&served);
    assert_eq!(running.len(), 1, "control: one adapter child, {running:?}");
    // SAFETY: a signal to this test's own adapter child, found just above.
    assert_eq!(unsafe { libc::kill(running[0], libc::SIGKILL) }, 0);
    let until = Instant::now() + Duration::from_secs(5);
    while adapters(&served).contains(&running[0]) {
        assert!(Instant::now() < until, "the killed adapter never ended");
        std::thread::sleep(Duration::from_millis(10));
    }
    let started = Instant::now();
    let relaunched = peer.call(3, tool.as_str().unwrap(), "2", &revision);
    let cold = started.elapsed();
    assert_eq!(
        relaunched["result"]["structuredContent"]["status"], "success",
        "{relaunched}"
    );
    send_call(&mut peer, 4, &tool, HOLD, &revision);
    reached(&served, 3);
    std::thread::sleep(Duration::from_millis(100));
    cancel(&mut peer, 4);
    let cancelled = Instant::now();
    let reply = peer.call(5, tool.as_str().unwrap(), "3", &revision);
    let waited = cancelled.elapsed();
    assert_eq!(
        reply["result"]["structuredContent"]["status"], "success",
        "{reply}"
    );
    assert!(
        waited <= cold + Duration::from_millis(1500),
        "the call after a cancelled read waited {waited:?}; a relaunching call took {cold:?}"
    );
    peer.finish();
}

/// owner.md: "a job still queued is dropped unrun", and "no frame on one
/// stream reaches another stream's request". Two sessions read the same
/// instance; the second's read queues behind the first's and is cancelled.
/// The first still gets its success, and the second's read never reaches the
/// provider.
#[test]
fn a_queued_read_another_session_cancels_runs_unsent_and_the_running_read_succeeds() {
    let served = Served::new();
    let mut first = Peer::start(&served, true);
    let mut second = Peer::start(&served, true);
    let (tool, revision) = listed(&mut first);
    let _ = listed(&mut second);
    send_call(&mut first, 2, &tool, SLOW, &revision);
    reached(&served, 1);
    send_call(&mut second, 2, &tool, "1", &revision);
    std::thread::sleep(Duration::from_millis(700));
    cancel(&mut second, 2);
    let reply = first.recv();
    assert_eq!(reply["id"], 2, "{reply}");
    assert_eq!(
        reply["result"]["structuredContent"]["status"], "success",
        "the running read of another session was cancelled: {reply}"
    );
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(
        served.provider.requests(),
        1,
        "the cancelled queued read was dispatched"
    );
    let again = second.call(3, tool.as_str().unwrap(), "2", &revision);
    assert_eq!(
        again["result"]["structuredContent"]["status"], "success",
        "{again}"
    );
    assert_eq!(served.provider.requests(), 2);
    second.finish();
    first.finish();
}

/// owner.md: a running cancelled read "ends its adapter process". The exact
/// process serving it is gone within a second, reaped rather than a zombie.
#[test]
fn a_cancelled_read_leaves_no_adapter_process_or_zombie() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = listed(&mut peer);
    send_call(&mut peer, 2, &tool, HOLD, &revision);
    reached(&served, 1);
    std::thread::sleep(Duration::from_millis(100));
    let before = adapters(&served);
    assert_eq!(before.len(), 1, "control: one adapter child, {before:?}");
    cancel(&mut peer, 2);
    let until = Instant::now() + Duration::from_secs(1);
    while present(before[0]) && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    let state = fs::read_to_string(format!("/proc/{}/stat", before[0])).unwrap_or_default();
    assert!(
        !present(before[0]),
        "the cancelled read's adapter process {} is still there: {state}",
        before[0]
    );
    peer.finish();
}
