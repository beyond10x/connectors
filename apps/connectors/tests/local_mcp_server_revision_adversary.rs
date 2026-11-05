//! Adversary cases for projection.md § Qualified names and inverse selection:
//! a `tools/call` with no `_meta["io.beyond10x.connectors/revision"]`, or no
//! `_meta`, binds to the revision the server advertises when it is called; a
//! present revision is a valid identifier or malformed, and a valid old one
//! is `stale_description`.
//!
//! The harness is `local_mcp_server.rs`'s, copied up to its `admitted`
//! helper: the production `connectors server --transport stdio` in a jail
//! with the `mounted_secret` binding, an adapter child over
//! `connectors-private/3` and a loopback provider whose request count is the
//! "no provider request" observation. Linux only: the binding is.
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

const ADAPTER_FIXTURE: &str = "local_mcp_server_revision_adversary_adapter_fixture";
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
/// The input whose answer the provider holds for 300 ms, well inside one
/// data lease.
const BRIEF: &str = "333";

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
                description: format!("fixture {id}"),
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
fn local_mcp_server_revision_adversary_adapter_fixture() {
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
}

impl Provider {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(AtomicUsize::new(0));
        let counted = requests.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let counted = counted.clone();
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
                        } else if n == BRIEF {
                            std::thread::sleep(Duration::from_millis(300));
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
        Self { address, requests }
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

/// `tools/list` holds exactly the admitted read, with no provider request:
/// the tool and its revision.
fn admitted(served: &Served, peer: &mut Peer, id: i32) -> (Value, Value) {
    let list = peer.request(id, "tools/list", json!({}));
    let tools = list["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("{list}"));
    assert_eq!(tools.len(), 1, "only the admitted read is listed: {list}");
    let tool = tools[0].clone();
    assert_eq!(tool["name"], name("read"), "{tool}");
    assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");
    // story:mcp-tool-call-shapes-its-result: the
    // advertised `result` is any JSON, and the operation's own output schema
    // is under the tool's operation `_meta`.
    assert_eq!(
        tool["outputSchema"]["properties"]["result"],
        json!({}),
        "{tool}"
    );
    assert_eq!(
        tool["_meta"]["io.beyond10x.connectors/operation"]["output_schema"]["properties"]["observed"]
            ["type"],
        "number",
        "{tool}"
    );
    assert_eq!(served.provider.requests(), 0, "listing sent a request");
    let revision = tool["_meta"][REVISION_META].clone();
    assert!(revision.is_string(), "{tool}");
    (tool, revision)
}

/// `tools/call` params for `name` with input `{"n":<n>}` and `meta` as its
/// `_meta`, or no `_meta` member at all for `None`.
fn params(name: &str, n: u64, meta: Option<Value>) -> Value {
    let mut params = json!({"name":name,"arguments":{"input":{"n":n}}});
    if let Some(meta) = meta {
        params["_meta"] = meta;
    }
    params
}

/// The shared refusal code of an rpc-error reply.
fn code(reply: &Value) -> &Value {
    &reply["error"]["data"]["error"]["code"]
}

/// The admitted read answered `n` as a successful envelope.
fn answered(reply: &Value, n: u64) {
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    let envelope = &reply["result"]["structuredContent"];
    assert_eq!(envelope["status"], "success", "{reply}");
    assert_eq!(envelope["result"]["observed"], n, "{reply}");
}

/// One `tools/call` frame on `2026-07-28` whose `_meta` is `meta`, written
/// verbatim so it can hold JSON escapes and duplicate members.
fn raw_call(id: i32, name: &str, n: u64, meta: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"tools/call\",\"params\":{{\"name\":\"{name}\",\"arguments\":{{\"input\":{{\"n\":{n}}}}},{meta}}}}}\n"
    )
}

/// The `2026-07-28` protocol members of a request's `_meta`.
const PROTOCOL_META: &str = "\"io.modelcontextprotocol/protocolVersion\":\"2026-07-28\",\"io.modelcontextprotocol/clientCapabilities\":{}";

/// Write the companion with each of [`EXPOSED`] but those in `absent` as a
/// tool, enabled unless it is `disabled` or in `off`.
fn companion(served: &Served, off: &[&str], absent: &[&str]) {
    let exposures: Vec<Value> = EXPOSED
        .iter()
        .filter(|operation| !absent.contains(operation))
        .map(|operation| {
            json!({"adapter_alias":ALIAS,"operation_ref":operation,"connection_ref":served.connection,
                "families":["tools"],"enabled":*operation != "disabled" && !off.contains(operation)})
        })
        .collect();
    private(
        &served.container.path(COMPANION),
        json!({"format":"connectors-mcp-local/1","limits":{"frame_octets":1048576,"response_octets":33554432,
            "concurrent_requests":1,"request_milliseconds":120000},"exposures":exposures})
        .to_string(),
    );
}

/// On `2025-11-25`, where no protocol member needs `_meta`, a `_meta` that is
/// present but not an object is not an absent revision: it is invalid params
/// with no provider request. `null` in particular is not "no `_meta`".
#[test]
fn a_meta_that_is_not_an_object_is_invalid_params_with_no_request() {
    let served = Served::new();
    let mut peer = Peer::start(&served, false);
    let (tool, revision) = admitted(&served, &mut peer, 1);
    let name = tool["name"].as_str().unwrap().to_owned();
    for (id, meta) in [
        (2, json!(null)),
        (3, json!([])),
        (4, json!("x")),
        (5, json!(7)),
        (6, json!(false)),
        (7, json!([{REVISION_META: revision}])),
        (8, json!(revision)),
    ] {
        let reply = peer.request(id, "tools/call", params(&name, 17, Some(meta.clone())));
        assert_eq!(reply["error"]["code"], -32602, "{meta}: {reply}");
    }
    assert_eq!(
        served.provider.requests(),
        0,
        "a call with a malformed _meta sent a request"
    );
    peer.finish();
}

/// "Absent" is the exact member name missing. A member that differs in case,
/// by a trailing space or by its prefix names no revision, so the call binds
/// to the advertised one even though that member holds a stale value. The
/// exact name spelled with JSON escapes is the member: its stale value is
/// stale, its current value runs.
#[test]
fn only_the_exact_member_name_is_a_revision() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = admitted(&served, &mut peer, 1);
    let name = tool["name"].as_str().unwrap().to_owned();
    for (id, key) in [
        (2, "io.beyond10x.connectors/Revision"),
        (3, "io.beyond10x.CONNECTORS/REVISION"),
        (4, "io.beyond10x.connectors/revision "),
        (5, "revision"),
    ] {
        let reply = peer.request(
            id,
            "tools/call",
            params(&name, id as u64, Some(json!({key: "stale-0"}))),
        );
        answered(&reply, id as u64);
    }
    assert_eq!(served.provider.requests(), 4, "four calls, four requests");
    let escaped = |id: i32, n: u64, value: &Value| {
        raw_call(
            id,
            &name,
            n,
            &format!(
                "\"_meta\":{{\"io.beyond10x.connectors\\/revisio\\u006e\":{value},{PROTOCOL_META}}}"
            ),
        )
    };
    peer.raw(escaped(6, 6, &json!("stale-0")).as_bytes());
    let reply = peer.recv();
    assert_eq!(reply["id"], 6, "{reply}");
    assert_eq!(code(&reply), "stale_description", "{reply}");
    assert_eq!(
        served.provider.requests(),
        4,
        "the escaped stale call sent a request"
    );
    peer.raw(escaped(7, 7, &revision).as_bytes());
    let reply = peer.recv();
    assert_eq!(reply["id"], 7, "{reply}");
    answered(&reply, 7);
    assert_eq!(served.provider.requests(), 5, "one call, one request");
    peer.finish();
}

/// A frame that names the revision twice, under one spelling or two, or has
/// two `_meta` members, cannot have one copy dropped so that a stale
/// revision becomes an absent one: it is a parse error with no request.
#[test]
fn a_duplicated_revision_or_meta_is_a_parse_error_with_no_request() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = admitted(&served, &mut peer, 1);
    let name = tool["name"].as_str().unwrap().to_owned();
    for meta in [
        format!(
            "\"_meta\":{{\"{REVISION_META}\":\"stale-0\",\"{REVISION_META}\":{revision},{PROTOCOL_META}}}"
        ),
        format!(
            "\"_meta\":{{\"{REVISION_META}\":{revision},\"{REVISION_META}\":\"stale-0\",{PROTOCOL_META}}}"
        ),
        format!(
            "\"_meta\":{{\"{REVISION_META}\":\"stale-0\",\"io.beyond10x.connectors\\/revision\":{revision},{PROTOCOL_META}}}"
        ),
        format!(
            "\"_meta\":{{\"{REVISION_META}\":\"stale-0\",{PROTOCOL_META}}},\"_meta\":{{{PROTOCOL_META}}}"
        ),
        format!(
            "\"_meta\":{{{PROTOCOL_META}}},\"_meta\":{{\"{REVISION_META}\":\"stale-0\",{PROTOCOL_META}}}"
        ),
    ] {
        peer.raw(raw_call(2, &name, 17, &meta).as_bytes());
        let reply = peer.recv();
        assert_eq!(reply["error"]["code"], -32700, "{meta}: {reply}");
    }
    assert_eq!(
        served.provider.requests(),
        0,
        "a duplicated revision sent a request"
    );
    let ping = peer.request(3, "ping", json!({}));
    assert!(ping.get("result").is_some(), "{ping}");
    peer.finish();
}

/// An absent revision binds to what the server advertises when the call
/// arrives, not to what the client listed: after the companion changes the
/// listed revision is stale and the absent call runs; once the tool's
/// exposure is gone the absent call is `not_found` and the listed or the
/// later revision is `stale_description`, each with no request.
#[test]
fn an_absent_revision_binds_to_the_current_advertisement_not_the_listed_one() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, listed) = admitted(&served, &mut peer, 1);
    let name = tool["name"].as_str().unwrap().to_owned();
    companion(&served, &["guarded"], &[]);
    let (_, current) = admitted(&served, &mut peer, 2);
    assert_ne!(listed, current, "a changed companion kept its revision");
    let reply = peer.call(3, &name, "17", &listed);
    assert_eq!(code(&reply), "stale_description", "{reply}");
    assert_eq!(
        served.provider.requests(),
        0,
        "the stale call sent a request"
    );
    let reply = peer.request(4, "tools/call", params(&name, 18, None));
    answered(&reply, 18);
    assert_eq!(served.provider.requests(), 1, "one call, one request");
    companion(&served, &[], &["read"]);
    let list = peer.request(5, "tools/list", json!({}));
    assert_eq!(list["result"]["tools"], json!([]), "{list}");
    let reply = peer.request(6, "tools/call", params(&name, 19, None));
    assert_eq!(code(&reply), "not_found", "{reply}");
    for (id, revision) in [(7, &listed), (8, &current)] {
        let reply = peer.call(id, &name, "19", revision);
        assert_eq!(code(&reply), "stale_description", "{reply}");
    }
    assert_eq!(
        served.provider.requests(),
        1,
        "a call on the withdrawn tool sent a request"
    );
    peer.finish();
}

/// A client that calls before it ever lists binds to the advertised
/// revision on either protocol revision: one call, one request.
#[test]
fn a_call_before_any_listing_binds_on_both_protocol_revisions() {
    let served = Served::new();
    for (primary, n) in [(true, 31), (false, 32)] {
        let mut peer = Peer::start(&served, primary);
        let reply = peer.request(1, "tools/call", params(&name("read"), n, None));
        answered(&reply, n);
        peer.finish();
    }
    assert_eq!(served.provider.requests(), 2, "two calls, two requests");
}

/// The absent path skips no check the current path runs: for every refused
/// tool, a call naming no revision is refused exactly as one naming the
/// current revision, malformed arguments and names stay `-32602`, and a
/// stale revision still beats the hidden tool's `not_found`. No request.
#[test]
fn an_absent_revision_meets_every_refusal_a_current_one_meets() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, current) = admitted(&served, &mut peer, 1);
    let foreign = connectors_mcp::names::encode_name(["other-instance", ADAPTER, "read"]).unwrap();
    let mut id = 2;
    for (tool_name, expected) in [
        (name("hidden"), "not_found"),
        (name("disabled"), "forbidden"),
        (name("guarded"), "not_found"),
        (name("ungranted"), "not_granted"),
        (foreign, "forbidden"),
    ] {
        let absent = peer.request(id, "tools/call", params(&tool_name, 17, None));
        let named = peer.call(id + 1, &tool_name, "17", &current);
        id += 2;
        assert_eq!(code(&absent), expected, "{tool_name}: {absent}");
        assert_eq!(code(&named), expected, "{tool_name}: {named}");
    }
    for bad in [
        json!({"name":tool["name"],"arguments":{"n":17}}),
        json!({"name":tool["name"],"arguments":{"input":{"n":17},"extra":1}}),
        json!({"name":"not a tool name","arguments":{"input":{"n":17}}}),
        json!({"arguments":{"input":{"n":17}}}),
    ] {
        let absent = peer.request(id, "tools/call", bad.clone());
        let mut with = bad.clone();
        with["_meta"] = json!({REVISION_META: current});
        let named = peer.request(id + 1, "tools/call", with);
        id += 2;
        assert_eq!(absent["error"]["code"], -32602, "{bad}: {absent}");
        assert_eq!(named["error"]["code"], -32602, "{bad}: {named}");
    }
    let stale = peer.call(id, &name("hidden"), "17", &json!("stale-0"));
    assert_eq!(code(&stale), "stale_description", "{stale}");
    assert_eq!(
        served.provider.requests(),
        0,
        "a refused call sent a request"
    );
    peer.finish();
}
