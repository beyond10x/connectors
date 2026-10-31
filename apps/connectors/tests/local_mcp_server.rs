//! story:mcp-stdio-runtime-branch. The production `connectors server
//! --transport stdio` lists and invokes admitted read-only tools through the
//! protected local owner (docs/local-mcp-stdio.md).
//!
//! The server, its owner and an adapter child run against the read-only
//! `mounted_secret` binding in a jail (`support/mounted.rs`). The adapter
//! child serves `connectors-private/3` and reads through the host's own HTTP
//! transport from a provider this test serves on the loopback, so "no provider
//! request" is that provider's request count staying 0. Every stdout line a
//! scenario reads must be one JSON-RPC frame. Linux only: the binding is.
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

const ADAPTER_FIXTURE: &str = "local_mcp_server_adapter_fixture";
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
fn local_mcp_server_adapter_fixture() {
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
    // story:mcp-tool-call-shapes-its-result (decision D240 c = A): the
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

/// The admitted read listed and called: the provider's answer comes back
/// whole as a successful, audited envelope after exactly one request.
fn lists_and_calls(primary: bool) {
    let served = Served::new();
    let mut peer = Peer::start(&served, primary);
    if primary {
        let discovered = peer.request(1, "server/discover", json!({}));
        assert_eq!(
            discovered["result"]["supportedVersions"],
            json!(["2026-07-28", "2025-11-25"]),
            "{discovered}"
        );
        assert!(
            discovered["result"]["capabilities"]["tools"].is_object(),
            "{discovered}"
        );
    }
    let (tool, revision) = admitted(&served, &mut peer, 2);
    let reply = peer.call(3, tool["name"].as_str().unwrap(), WIDE, &revision);
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    let envelope = &reply["result"]["structuredContent"];
    assert_eq!(envelope["status"], "success", "{reply}");
    let wide: Value = serde_json::from_str(WIDE).unwrap();
    assert_eq!(envelope["result"]["observed"], wide, "{reply}");
    assert_eq!(envelope["audit_status"], "complete", "{reply}");
    assert!(envelope["audit_ref"].is_string(), "{reply}");
    assert!(envelope["request_id"].is_string(), "{reply}");
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("{reply}"));
    assert_eq!(serde_json::from_str::<Value>(text).unwrap(), *envelope);
    assert!(
        text.contains(&format!("\"observed\":{WIDE}")),
        "the provider's number did not come back digit for digit: {text}"
    );
    assert_eq!(served.provider.requests(), 1, "one call, one request");
    peer.finish();
}

#[test]
fn stdio_lists_and_calls_an_admitted_read_on_2026_07_28() {
    lists_and_calls(true);
}

#[test]
fn stdio_lists_and_calls_an_admitted_read_on_2025_11_25() {
    lists_and_calls(false);
}

/// `operation` is not listed, and its call is refused with `code` before
/// any provider request.
fn refused(operation: &str, code: &str) {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (_, revision) = admitted(&served, &mut peer, 1);
    let reply = peer.call(2, &name(operation), "17", &revision);
    assert_eq!(reply["error"]["data"]["error"]["code"], code, "{reply}");
    assert_eq!(
        served.provider.requests(),
        0,
        "the refused {operation} call sent a request"
    );
    peer.finish();
}

#[test]
fn a_disabled_operation_is_not_listed_and_its_call_makes_no_request() {
    refused("disabled", "forbidden");
}

#[test]
fn a_hidden_operation_is_not_listed_and_its_call_makes_no_request() {
    refused("hidden", "not_found");
}

#[test]
fn an_approval_required_operation_is_refused_with_no_request() {
    refused("guarded", "not_found");
}

#[test]
fn an_ungranted_operation_is_refused_with_no_request() {
    refused("ungranted", "not_granted");
}

/// Every exchange of the scenarios above, on both revisions, leaves only
/// JSON-RPC frames on stdout, one per line; a server that cannot start
/// writes its diagnostic to stderr and nothing to stdout.
#[test]
fn stdout_carries_only_json_rpc_frames() {
    let served = Served::new();
    let mut lines = Vec::new();
    for primary in [true, false] {
        let mut peer = Peer::start(&served, primary);
        let (tool, revision) = {
            let list = peer.request(1, "tools/list", json!({}));
            let tool = list["result"]["tools"][0].clone();
            let revision = tool["_meta"][REVISION_META].clone();
            (tool, revision)
        };
        let reply = peer.call(2, tool["name"].as_str().unwrap(), "5", &revision);
        assert_eq!(reply["result"]["isError"], false, "{reply}");
        for (id, operation) in [
            (3, "disabled"),
            (4, "hidden"),
            (5, "guarded"),
            (6, "ungranted"),
        ] {
            let reply = peer.call(id, &name(operation), "17", &revision);
            assert!(reply["error"].is_object(), "{reply}");
        }
        peer.raw(b"{broken\n");
        assert_eq!(peer.recv()["error"]["code"], -32700);
        peer.raw(b"[]\n");
        assert_eq!(peer.recv()["error"]["code"], -32600);
        assert!(peer.request(7, "ping", json!({})).get("result").is_some());
        let (written, _) = peer.finish();
        assert_eq!(written.len(), if primary { 9 } else { 10 }, "{written:?}");
        lines.extend(written);
    }
    for line in &lines {
        assert!(!line.is_empty(), "an empty stdout line");
        frame(line).unwrap_or_else(|error| panic!("{error}"));
    }
    assert_eq!(served.provider.requests(), 2, "two admitted calls");
    fs::remove_file(served.container.path(COMPANION)).unwrap();
    let output = Jail::output(served.server().stdin(Stdio::null()));
    assert!(!output.status.success(), "{output:?}");
    assert!(
        output.stdout.is_empty(),
        "a diagnostic reached protocol stdout: {output:?}"
    );
    assert!(!output.stderr.is_empty(), "{output:?}");
}

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
    command.env_clear().stdin(Stdio::null());
    command
}

#[test]
fn selected_launch_help_uses_the_native_grammar() {
    let output = command().args(["server", "--help"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("--transport"), "{help}");
    assert!(help.contains("--config"), "{help}");
    assert!(!help.contains("--output"), "{help}");
}

#[test]
fn owner_refusal_and_invalid_launch_emit_no_protocol_bytes() {
    let root = tempfile::tempdir().unwrap();
    for suffix in [
        vec!["--transport", "stdio"],
        vec![],
        vec!["--transport", "http"],
        vec!["--transport", "stdio", "--output", "json"],
        vec!["--transport", "stdio", "--credential-stdin"],
    ] {
        let output = command()
            .arg("--config")
            .arg(root.path().join("absent.toml"))
            .arg("--state-dir")
            .arg(root.path().join("absent-state"))
            .arg("server")
            .args(suffix)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            output.stdout.is_empty(),
            "CLI envelope leaked to protocol stdout: {output:?}"
        );
        assert!(!output.stderr.is_empty());
        assert!(!root.path().join("absent-state").exists());
    }
}

/// Withdrawal, framing and explicit cancellation: a withdrawn exposure is
/// unlisted and its call stale; malformed and oversized frames are answered
/// and the session goes on; a cancelled call is never answered nor replayed;
/// EOF discards an unfinished frame and ends the session cleanly.
#[test]
fn production_stdio_withdrawal_framing_and_cancellation() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    peer.send(json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}));
    assert_eq!(peer.recv()["error"]["code"], -32602);
    let (tool, revision) = admitted(&served, &mut peer, 2);
    let name = tool["name"].as_str().unwrap().to_owned();
    served.exposures(false);
    let list = peer.request(3, "tools/list", json!({}));
    assert_eq!(list["result"]["tools"], json!([]), "{list}");
    let reply = peer.call(4, &name, "1", &revision);
    assert_eq!(
        reply["error"]["data"]["error"]["code"], "stale_description",
        "{reply}"
    );
    assert_eq!(served.provider.requests(), 0);
    peer.raw(b"{broken\n");
    assert_eq!(peer.recv()["error"]["code"], -32700);
    peer.raw(b"[]\n");
    assert_eq!(peer.recv()["error"]["code"], -32600);
    peer.raw(&vec![b' '; 1_048_577]);
    peer.raw(b"\n");
    assert_eq!(peer.recv()["error"]["code"], -32700);
    let ping = peer.request(5, "ping", json!({}));
    assert_eq!(ping["result"]["resultType"], "complete", "{ping}");
    served.exposures(true);
    let (_, revision) = admitted(&served, &mut peer, 6);
    peer.send(json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":name,"arguments":{"input":{"n":999}},
        "_meta":{REVISION_META:revision,"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}));
    let until = Instant::now() + Duration::from_secs(5);
    while served.provider.requests() == 0 {
        assert!(
            Instant::now() < until,
            "the call never reached the provider"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    peer.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":7}}));
    let ping = peer.request(8, "ping", json!({}));
    assert!(ping.get("result").is_some(), "{ping}");
    std::thread::sleep(Duration::from_millis(2100));
    let ping = peer.request(9, "ping", json!({}));
    assert!(
        ping.get("result").is_some(),
        "cancelled call answered: {ping}"
    );
    assert_eq!(
        served.provider.requests(),
        1,
        "cancellation replayed the provider request"
    );
    // EOF discards an unfinished frame.
    peer.raw(b"{\"jsonrpc\":\"2.0\"");
    peer.finish();
}

/// A session stopped past its 1900 ms data lease cannot renew it: resumed, it
/// ends unsuccessfully without another frame or provider request.
#[test]
fn production_stdio_stopped_session_cannot_renew_after_expiry() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    assert!(peer.request(1, "ping", json!({})).get("result").is_some());
    let pid = i32::try_from(peer.child.id()).unwrap();
    // SAFETY: these signals target only this test's own unreaped child.
    assert_eq!(unsafe { libc::kill(pid, libc::SIGSTOP) }, 0);
    std::thread::sleep(Duration::from_millis(2200));
    // SAFETY: as above.
    assert_eq!(unsafe { libc::kill(pid, libc::SIGCONT) }, 0);
    peer.exit(false);
    assert_eq!(served.provider.requests(), 0);
}

/// `tools/call` id 2 of the admitted read with input `n`, primary revision,
/// not waiting for its reply.
fn send_call(peer: &mut Peer, tool: &Value, n: u64, revision: &Value) {
    peer.send(
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
        "name":tool["name"],"arguments":{"input":{"n":n}},
        "_meta":{REVISION_META:revision,"io.modelcontextprotocol/protocolVersion":"2026-07-28",
            "io.modelcontextprotocol/clientCapabilities":{}}}}),
    );
}

/// Closing stdin is the graceful shutdown of server semantics § 3.5, not a
/// withdrawal: a call already at the owner when stdin closes is still answered
/// under the live lease, after which the server exits successfully with
/// nothing more.
#[test]
fn stdin_close_still_answers_requests_already_read() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = admitted(&served, &mut peer, 1);
    send_call(&mut peer, &tool, 333, &revision);
    let until = Instant::now() + Duration::from_secs(5);
    while served.provider.requests() == 0 {
        assert!(
            Instant::now() < until,
            "the call never reached the provider"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(peer.child.stdin.take());
    let reply = match peer.frames.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(reply)) => reply,
        Ok(Err(line)) => panic!("{line}"),
        Err(_) => panic!(
            "no reply to a call read before stdin closed (provider holds {BRIEF} for 300 ms): \
             the server is {:?}, provider requests {}",
            peer.child.try_wait().unwrap(),
            served.provider.requests()
        ),
    };
    assert_eq!(reply["id"], 2, "{reply}");
    assert_eq!(
        reply["result"]["structuredContent"]["status"], "success",
        "{reply}"
    );
    assert_eq!(
        reply["result"]["structuredContent"]["result"]["observed"], 333,
        "{reply}"
    );
    peer.finish();
    assert_eq!(served.provider.requests(), 1, "one call, one request");
}

/// sessions § 4.1: closing stdin is the `remote_hangup` terminal, at which
/// local admission stops. A call read just before the close, whose owner read
/// has not started, is answered `unavailable` and never reaches the owner's
/// provider; the server still exits successfully. The server is stopped while
/// the call is written and stdin closed, so it finds both on resuming, however
/// the host schedules its threads.
#[test]
fn stdin_close_refuses_a_call_not_yet_at_the_owner() {
    let served = Served::new();
    let mut peer = Peer::start(&served, true);
    let (tool, revision) = admitted(&served, &mut peer, 1);
    let pid = i32::try_from(peer.child.id()).unwrap();
    // SAFETY: these calls target only this test's own unreaped child; the
    // stop is reported without reaping it.
    assert_eq!(unsafe { libc::kill(pid, libc::SIGSTOP) }, 0);
    let mut status = 0;
    assert_eq!(
        unsafe { libc::waitpid(pid, &mut status, libc::WUNTRACED) },
        pid
    );
    assert!(
        libc::WIFSTOPPED(status),
        "the server did not stop: {status}"
    );
    send_call(&mut peer, &tool, 333, &revision);
    drop(peer.child.stdin.take());
    // SAFETY: as above.
    assert_eq!(unsafe { libc::kill(pid, libc::SIGCONT) }, 0);
    let reply = match peer.frames.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(reply)) => reply,
        Ok(Err(line)) => panic!("{line}"),
        Err(_) => panic!(
            "no reply to a call read before stdin closed: the server is {:?}, provider requests {}",
            peer.child.try_wait().unwrap(),
            served.provider.requests()
        ),
    };
    assert_eq!(reply["id"], 2, "{reply}");
    assert_eq!(
        reply["error"]["data"]["error"]["code"], "unavailable",
        "a call admitted after stdin closed: {reply}"
    );
    peer.finish();
    assert_eq!(
        served.provider.requests(),
        0,
        "a call reached the provider after stdin closed"
    );
}
