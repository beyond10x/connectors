//! story:mcp-write-operation-call-by-name-is-refused. The production
//! `connectors server --transport stdio` advertises read-only tools only, and
//! a client that calls an operation by name without it being listed meets the
//! same refusal: `not_found`, before any provider request, whether the call
//! names the current projection revision or none.
//!
//! Two operations are called by name while their approval is not required:
//!
//! - `write`, with a `Write` requirement and an `external_write` declaration.
//!   `connectors-private/3`, the only binding the server projects, refuses a
//!   `Write` requirement at readiness (`Bootstrap::validate_for`), so a
//!   description holding one exists only on `connectors-private/2`; this case
//!   runs there.
//! - `notify`, a read on `connectors-private/3` whose declaration carries a
//!   non-empty `semantic_effects`.
//!
//! The harness is `local_mcp_server.rs`'s, cut down to what these calls need:
//! the server in a jail with the `mounted_secret` binding, an adapter child
//! that sends a request to a loopback provider for every operation it is asked
//! to run, bounded or not, and that provider's request count as the "no
//! provider request" observation. Linux only: the binding is.
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
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

const ADAPTER_FIXTURE: &str = "local_mcp_server_write_refusal_adapter_fixture";
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

/// One operation of a description: its id, its native profile, its
/// requirement's effect, its declared effects and semantic effects.
struct Declared {
    id: &'static str,
    profile: &'static str,
    effect: runtime::Effect,
    effects: &'static [&'static str],
    semantic_effects: &'static [&'static str],
}

const READ: Declared = Declared {
    id: "read",
    profile: "resource",
    effect: runtime::Effect::Read,
    effects: &["network"],
    semantic_effects: &[],
};
const WRITE: Declared = Declared {
    id: "write",
    profile: "mutation",
    effect: runtime::Effect::Write,
    effects: &["network", "external_write"],
    semantic_effects: &[],
};
const NOTIFY: Declared = Declared {
    id: "notify",
    profile: "resource",
    effect: runtime::Effect::Read,
    effects: &["network"],
    semantic_effects: &["human_visible"],
};

/// The description the adapter serves on each binding: `read` and `write`
/// on `connectors-private/2`, `read` and `notify` on `connectors-private/3`.
#[derive(Clone, Copy)]
enum Binding {
    V2,
    V3,
}

impl Binding {
    fn operations(self) -> [&'static Declared; 2] {
        match self {
            Self::V2 => [&READ, &WRITE],
            Self::V3 => [&READ, &NOTIFY],
        }
    }
    fn protocol(self) -> runtime::PrivateProtocol {
        match self {
            Self::V2 => runtime::PrivateProtocol::V2,
            Self::V3 => runtime::PrivateProtocol::V3,
        }
    }
    fn wire(self) -> &'static str {
        match self {
            Self::V2 => "connectors-private/2",
            Self::V3 => "connectors-private/3",
        }
    }
}

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

/// The description on `binding`: each operation takes `{n: number}` and
/// answers `{observed: number}` under one static profile.
fn bootstrap(binding: Binding) -> runtime::Bootstrap {
    let operations = binding.operations();
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: INSTANCE.into(),
        adapter: ADAPTER.into(),
        revision: DESCRIPTION.into(),
        operations: operations
            .iter()
            .map(|operation| connectors_core::Operation {
                id: operation.id.into(),
                description: format!("fixture {}", operation.id),
                contract: "operations/v1alpha1".into(),
                profile: operation.profile.into(),
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
        requirements: operations
            .iter()
            .map(|operation| runtime::Requirement {
                operation: operation.id.into(),
                custody: false,
                profile: PROFILE.into(),
                scopes: BTreeSet::new(),
                effect: operation.effect,
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

/// One `GET /observe/<n>` to the provider, whatever the operation: a call
/// that reaches the adapter is a provider request.
async fn observe(secret: connectors_sdk::Secret, input: Value) -> runtime::Result<Value> {
    if secret.0 != SECRET {
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

/// The adapter child: validation makes no provider request; every invoke,
/// bounded or not, of any operation is one provider request.
struct Observing;

#[async_trait::async_trait]
impl runtime::Adapter for Observing {
    fn bootstrap(&self) -> runtime::Bootstrap {
        bootstrap(Binding::V3)
    }
    fn bootstrap_v2(&self) -> runtime::Result<runtime::Bootstrap> {
        Ok(bootstrap(Binding::V2))
    }
    fn bootstrap_v3(&self) -> runtime::Result<runtime::Bootstrap> {
        Ok(bootstrap(Binding::V3))
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
        secret: connectors_sdk::Secret,
        input: Value,
    ) -> runtime::Result<Value> {
        observe(secret, input).await
    }
    async fn invoke_bounded(
        &self,
        _: &str,
        _: &str,
        secret: connectors_sdk::Secret,
        input: Value,
        budget: runtime::ReadBudget,
    ) -> runtime::Result<Value> {
        budget.until()?;
        observe(secret, input).await
    }
}

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn local_mcp_server_write_refusal_adapter_fixture() {
    if std::env::args().any(|arg| arg == "--connectors-private-fd") {
        let _ = runtime::serve(3, Observing);
    }
}

/// A loopback HTTP/1.1 provider answering `GET /observe/<n>` with
/// `{"observed":<n>}`, counting every request head it reads.
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

/// The configured adapter entry `forge` on `binding`, granted every
/// operation of its description.
fn entry(container: &Container, binding: Binding) -> config::Adapter {
    config::Adapter {
        instance_id: INSTANCE.into(),
        adapter_id: ADAPTER.into(),
        configuration_revision: "cfg-1".into(),
        credential_revision: None,
        protocol: "v1alpha1".into(),
        private_protocol: Some(binding.protocol()),
        startup: Default::default(),
        restart: Default::default(),
        executable: config::Executable {
            path: "/bin-test/adapter".into(),
            sha256: digest(&fs::read(container.path("/bin-test/adapter")).unwrap()),
            args: vec!["--exact".into(), ADAPTER_FIXTURE.into(), "--".into()],
        },
        permissions: config::Permissions {
            profiles: BTreeSet::from([PROFILE.to_owned()]),
            operations: binding
                .operations()
                .iter()
                .map(|operation| operation.id.to_owned())
                .collect(),
            custody: Default::default(),
        },
    }
}

/// The configuration: mounted custody, a two-source approval clock (never
/// sampled here) and `adapter` as `forge` on `binding`.
fn configure(container: &Container, adapter: &config::Adapter, binding: Binding) {
    let uid = connectors_host::local::filesystem::uid();
    let text = format!(
        "format='connectors-local/4'\nowner_uid={uid}\n{custody}[approval_clock]\nformat='roughtime-quorum/1'\nquorum=2\nmax_rate_error_ppm=10000\n[[approval_clock.sources]]\naddress='127.0.0.1:9'\npublic_key='{key_a}'\nversion='roughtime-0x8000000c'\n[[approval_clock.sources]]\naddress='127.0.0.1:10'\npublic_key='{key_b}'\nversion='roughtime-0x8000000c'\n[adapters.{ALIAS}]\ninstance_id='{INSTANCE}'\nadapter_id='{ADAPTER}'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='{wire}'\n[adapters.{ALIAS}.permissions]\nprofiles={profiles}\noperations={operations}\n[adapters.{ALIAS}.executable]\npath='/bin-test/adapter'\nsha256='{sha256}'\nargs={args}\n",
        custody = Container::mounted(),
        key_a = clock_key(7),
        key_b = clock_key(8),
        wire = binding.wire(),
        profiles = serde_json::to_string(&adapter.permissions.profiles).unwrap(),
        operations = serde_json::to_string(&adapter.permissions.operations).unwrap(),
        sha256 = adapter.executable.sha256,
        args = serde_json::to_string(&adapter.executable.args).unwrap(),
    );
    private(&container.path(CONFIG), text);
}

/// One container on `binding` with a connection made through the mounted
/// credential, each operation curated with approval not required, and each
/// exposed as an enabled tool.
struct Served {
    container: Container,
    provider: Provider,
}

impl Served {
    fn new(binding: Binding) -> Self {
        let provider = Provider::start();
        let container = Container::new(ADAPTER_FIXTURE);
        fs::write(container.path(PROVIDER), &provider.address).unwrap();
        container.deliver(PROFILE, SECRET);
        let adapter = entry(&container, binding);
        configure(&container, &adapter, binding);
        let init = success(&container.run(&["setup", "init"]));
        assert_eq!(init["disposition"], "state_initialized", "{init}");
        let connected = success(&container.connect(PROFILE));
        let connection = connected["connection"]["summary"]["connection"]
            .as_str()
            .unwrap_or_else(|| panic!("{connected}"))
            .to_owned();
        let operations: Vec<Value> = binding
            .operations()
            .iter()
            .map(|operation| {
                json!({"operation":operation.id,"metadata":{"effects":operation.effects,
                    "semantic_effects":operation.semantic_effects,"risk":"low","idempotency":{"kind":"none"},
                    "approval":"not_required","realization":"implemented",
                    "limits":{"request_bytes":65536,"result_bytes":4194304,"execution_ms":20000,"provider_ms":15000,"connect_ms":5000}}})
            })
            .collect();
        private(
            &container.path(CURATION),
            json!({"format":"connectors-operation-curation/1","adapters":[{"adapter_alias":ALIAS,
                "executable_selection":adapter.selection(),
                "bootstrap_sha256":connectors_core::digest(&serde_json::to_value(bootstrap(binding)).unwrap()),
                "operations":operations}]})
            .to_string(),
        );
        let exposures: Vec<Value> = binding
            .operations()
            .iter()
            .map(|operation| {
                json!({"adapter_alias":ALIAS,"operation_ref":operation.id,"connection_ref":connection,
                    "families":["tools"],"enabled":true})
            })
            .collect();
        private(
            &container.path(COMPANION),
            json!({"format":"connectors-mcp-local/1","limits":{"frame_octets":1048576,"response_octets":33554432,
                "concurrent_requests":1,"request_milliseconds":120000},"exposures":exposures})
            .to_string(),
        );
        assert_eq!(provider.requests(), 0, "setting up sent a request");
        Self {
            container,
            provider,
        }
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

/// An MCP client of the jailed server on protocol `2026-07-28`: its stdin,
/// each stdout line parsed as one JSON value, and its stderr apart.
struct Peer {
    child: Child,
    frames: mpsc::Receiver<Result<Value, String>>,
    reader: Option<JoinHandle<()>>,
    errors: Option<JoinHandle<Vec<u8>>>,
}

impl Peer {
    fn start(served: &Served) -> Self {
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
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                let frame = serde_json::from_str(&line)
                    .map_err(|error| format!("protocol stdout polluted ({error}): {line}"));
                if sender.send(frame).is_err() {
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
    fn recv(&self) -> Value {
        match self.frames.recv_timeout(Duration::from_secs(15)) {
            Ok(Ok(value)) => value,
            Ok(Err(line)) => panic!("{line}"),
            Err(_) => panic!("the server did not reply"),
        }
    }
    /// One request carrying the protocol revision; `params["_meta"]` is
    /// kept and only the protocol fields are added to it.
    fn request(&mut self, id: i32, method: &str, mut params: Value) -> Value {
        if !params["_meta"].is_object() {
            params["_meta"] = json!({});
        }
        params["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2026-07-28");
        params["_meta"]["io.modelcontextprotocol/clientCapabilities"] = json!({});
        let frame = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(format!("{frame}\n").as_bytes()).unwrap();
        let reply = self.recv();
        assert_eq!(reply["jsonrpc"], "2.0", "{reply}");
        assert_eq!(reply["id"], id, "{reply}");
        reply
    }
    /// `tools/call` of `operation` by name with `{"input":{"n":17}}`, at
    /// `revision` when there is one and naming none otherwise.
    fn call(&mut self, id: i32, operation: &str, revision: Option<&Value>) -> Value {
        let mut params = json!({"name":name(operation),"arguments":{"input":{"n":17}}});
        if let Some(revision) = revision {
            params["_meta"] = json!({REVISION_META: revision});
        }
        self.request(id, "tools/call", params)
    }
    /// Close stdin and wait for a clean exit with nothing more on stdout.
    fn finish(mut self) {
        drop(self.child.stdin.take());
        let until = Instant::now() + Duration::from_secs(6);
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
        let _ = self.errors.take().unwrap().join();
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(errors) = self.errors.take().filter(|_| std::thread::panicking()) {
            eprintln!(
                "server stderr: {}",
                String::from_utf8_lossy(&errors.join().unwrap_or_default())
            );
        }
    }
}

/// The names `tools/list` advertises, and the projection revision it names.
fn listed(peer: &mut Peer, id: i32) -> (Vec<String>, Value) {
    let list = peer.request(id, "tools/list", json!({}));
    let tools = list["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("{list}"))
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or_default().to_owned())
        .collect();
    let revision = list["result"]["_meta"][REVISION_META].clone();
    assert!(revision.is_string(), "{list}");
    (tools, revision)
}

/// `operation`, called by name at the current revision and then with none,
/// is refused `not_found` each time with no provider request, and is not
/// listed.
fn refused_by_name(served: &Served, peer: &mut Peer, operation: &str) {
    let (tools, revision) = listed(peer, 1);
    for (id, revision) in [(2, Some(&revision)), (3, None)] {
        let reply = peer.call(id, operation, revision);
        assert_eq!(
            reply["error"]["data"]["error"]["code"], "not_found",
            "{operation} at revision {revision:?}: {reply}"
        );
        assert_eq!(
            served.provider.requests(),
            0,
            "the refused {operation} call at revision {revision:?} sent a provider request"
        );
    }
    assert!(
        !tools.contains(&name(operation)),
        "{operation} is listed: {tools:?}"
    );
}

#[test]
fn a_write_requirement_called_by_name_is_not_found_with_no_request() {
    let served = Served::new(Binding::V2);
    let mut peer = Peer::start(&served);
    refused_by_name(&served, &mut peer, WRITE.id);
    peer.finish();
}

/// The same server runs the admitted `read` after refusing `notify`: the
/// refusal is the guard's, not an unreachable provider's.
#[test]
fn a_semantic_effect_called_by_name_is_not_found_with_no_request() {
    let served = Served::new(Binding::V3);
    let mut peer = Peer::start(&served);
    refused_by_name(&served, &mut peer, NOTIFY.id);
    let (tools, revision) = listed(&mut peer, 4);
    assert_eq!(tools, [name(READ.id)], "only the admitted read is listed");
    let reply = peer.call(5, READ.id, Some(&revision));
    assert_eq!(
        reply["result"]["structuredContent"]["status"], "success",
        "{reply}"
    );
    assert_eq!(served.provider.requests(), 1, "the read sent one request");
    peer.finish();
}
