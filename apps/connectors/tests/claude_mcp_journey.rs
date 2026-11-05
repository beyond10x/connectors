//! story:claude-connects-to-the-mcp-server. A session recorded from a real
//! Claude Code client (a headless `claude -p` run; the client and its version
//! are in the header line of `fixtures/claude-code/session.jsonl`) is replayed
//! against the production `connectors server --transport stdio`, and every
//! answer is compared with the one the client received
//! (`fixtures/claude-code/session.expected.jsonl`).
//!
//! The server, its owner and an adapter child run in the jail of
//! `support/mounted.rs` against the read-only `mounted_secret` binding. The
//! child stands in for the catalog provider's Jira instance `jira.work`: it
//! describes the instance through the catalog engine
//! (`connectors_catalog_provider::Engine`) from the shipped Jira bundle and
//! selection, which is the description the catalog provider gives under
//! `connectors-private/3`, so the tool names and schemas are the ones the
//! client saw. It reads through the engine and the host's HTTP transport, in
//! plaintext, from a provider this test serves on the loopback, which answers
//! `GET /rest/api/3/myself` and `GET /rest/api/3/search/jql`, the latter with
//! `fixtures/claude-code/jira-search.json`. (The catalog provider itself
//! speaks HTTPS only and is not built for this package's tests.)
//!
//! What the recording shows: the client probes with `server/discover` on
//! `2026-07-28`, sends an `initialize` on `2025-11-25`, which a session already
//! on `2026-07-28` refuses as an invalid request, discovers again, lists the
//! tools and calls the Jira search tool twice, with `walk` and two different
//! `fields`. `claude_mcp_add_arguments_start_the_server` holds the `2025-11-25`
//! handshake on its own: the recorded `initialize` as a session's first frame.
//! Linux only: the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors_catalog_provider::{Effect, Engine, Selection, feed};
use connectors_custody::mounted::fixture::{Jail, Map};
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    local::{config, registry, runtime},
};
use connectors_sdk::{AuthenticatedHttp, Credential, Secret};
use mounted::{ALIAS, CONFIG, Container, STATE, digest, success};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpListener},
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, OnceLock, mpsc},
    thread::JoinHandle,
    time::{Duration, Instant},
};

const ADAPTER_FIXTURE: &str = "claude_mcp_journey_adapter_fixture";
/// Inside the jail: the loopback address of the test's provider.
const PROVIDER: &str = "/work/provider";
const COMPANION: &str = "/srv/connectors/config.toml.mcp-stdio.json";
const CURATION: &str = "/srv/connectors/config.toml.operation-curation.json";
const INSTANCE: &str = "jira.work";
const ADAPTER: &str = "catalog";
const PROFILE: &str = "atlassian.basic";
/// A fictional Atlassian account and API token, the profile's document.
const CREDENTIAL: &[u8] =
    br#"{"account":"fixture-operator@example.com","token":"fixture-api-token-0001"}"#;
/// `Basic base64("fixture-operator@example.com:fixture-api-token-0001")`,
/// computed outside the test with coreutils `base64`.
const AUTHORIZATION: &str =
    "Basic Zml4dHVyZS1vcGVyYXRvckBleGFtcGxlLmNvbTpmaXh0dXJlLWFwaS10b2tlbi0wMDAx";
const REVISION_META: &str = "io.beyond10x.connectors/revision";
const OPERATION_META: &str = "io.beyond10x.connectors/operation";

const SESSION: &str = include_str!("fixtures/claude-code/session.jsonl");
const EXPECTED: &str = include_str!("fixtures/claude-code/session.expected.jsonl");
const TRANSCRIPT: &str = include_str!("fixtures/claude-code/transcript.jsonl");
const SEARCH: &str = include_str!("fixtures/claude-code/jira-search.json");
const BUNDLE: &[u8] =
    include_bytes!("../../../adapters/catalog/generated/bundles/jira.bundle.json");
const SELECTION: &str = include_str!("../../../adapters/catalog/providers/jira/operations.json");
const MCP_PAGE: &str = include_str!("../../../website/docs/connectors/mcp.md");

/// The catalog engine over the shipped Jira bundle and selection, its feed
/// included, as the catalog provider builds it for an instance whose API base
/// path is `/rest/api/3`.
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let bundle: connectors_catalog::bundle::Bundle =
            connectors_core::read_json(BUNDLE).unwrap();
        let selection: Value = serde_json::from_str(SELECTION).unwrap();
        let selections: Vec<Selection> =
            serde_json::from_value(selection["operations"].clone()).unwrap();
        let feed: Option<feed::Declaration> =
            serde_json::from_value(selection["feed"].clone()).unwrap();
        Engine::with_feed(&bundle, "/rest/api/3", &selections, feed.as_ref()).unwrap()
    })
}

/// The operations of the Jira selection whose declared effect is `effect`.
fn selected(effect: &str) -> Vec<String> {
    let selection: Value = serde_json::from_str(SELECTION).unwrap();
    selection["operations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|operation| operation["effect"] == effect)
        .map(|operation| operation["id"].as_str().unwrap().to_owned())
        .collect()
}

fn profile() -> runtime::Profile {
    runtime::Profile {
        id: PROFILE.into(),
        revision: "profile-1".into(),
        purpose: registry::Purpose::DelegatedUser,
        subject: registry::Subject::User,
        scheme: "http_basic".into(),
        capability: "http-basic".into(),
        minimum_scopes: BTreeSet::new(),
        evidence_lifetime_ms: 60_000,
        fields: vec![
            runtime::EntryField {
                name: "account".into(),
                label: "Account email".into(),
                max_bytes: 1024,
            },
            runtime::EntryField {
                name: "token".into(),
                label: "API token".into(),
                max_bytes: 8192,
            },
        ],
        acquisition: None,
    }
}

/// The description the catalog provider gives `jira.work` under
/// `connectors-private/3`: the selection's reads, the feed's two among them,
/// and no write, under the one basic profile.
fn bootstrap() -> runtime::Bootstrap {
    let engine = engine();
    let reads = engine.declarations(&[Effect::Read]);
    let descriptor = connectors_core::Descriptor {
        version: connectors_core::WIRE_VERSION.into(),
        instance: INSTANCE.into(),
        adapter: ADAPTER.into(),
        revision: "desc-1".into(),
        operations: reads.clone(),
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        bindings: None,
        credential_revision: None,
        executable: None,
        instance: INSTANCE.into(),
        adapter: ADAPTER.into(),
        protocol: connectors_core::WIRE_VERSION.into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: "https://demo.example.com/rest/api/3/".into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![profile()],
        requirements: reads
            .iter()
            .map(|operation| runtime::Requirement {
                operation: operation.id.clone(),
                custody: engine.declares_custody(&operation.id),
                profile: PROFILE.into(),
                scopes: BTreeSet::new(),
                effect: runtime::Effect::Read,
            })
            .collect(),
    }
}

/// The described reads of profile `generic-http`, the ones the stdio
/// projection can advertise; the feed's reads have a profile of their own.
fn generic_reads() -> Vec<String> {
    engine()
        .declarations(&[Effect::Read])
        .into_iter()
        .filter(|operation| operation.profile == "generic-http")
        .map(|operation| operation.id)
        .collect()
}

/// The `Authorization` value a basic document gives.
struct Basic(Secret);

#[async_trait::async_trait]
impl Credential for Basic {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(self.0.0.clone()))
    }
}

/// The plaintext transport to the provider named in [`PROVIDER`], with the
/// basic header of `document` (`{"account","token"}`).
fn http(document: &Secret) -> runtime::Result<ScopedHttp> {
    use base64::Engine as _;
    static HTTP: OnceLock<ScopedHttp> = OnceLock::new();
    let entry: Value =
        serde_json::from_slice(&document.0).map_err(|_| runtime::Failure::InvalidCredential)?;
    let (Some(account), Some(token)) = (entry["account"].as_str(), entry["token"].as_str()) else {
        return Err(runtime::Failure::InvalidCredential);
    };
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(format!("{account}:{token}").as_bytes());
    let header = Secret(format!("Basic {encoded}").into_bytes());
    let base = HTTP.get_or_init(|| {
        let address = fs::read_to_string(PROVIDER).unwrap();
        ScopedHttp::new_with_ca_bytes(
            &HttpConfig {
                base_url: format!("http://{}/rest/api/3/", address.trim()),
                credential: None,
                credential_header: "Authorization".into(),
                bearer: false,
                allow_plaintext: true,
                ca_file: None,
            },
            None,
            None,
        )
        .unwrap()
    });
    Ok(base.with_credential(Arc::new(Basic(header))))
}

/// The adapter child: the catalog provider's description of `jira.work`;
/// validation is the declared identity read (`GET myself`, subject
/// `/accountId`); a bounded read is the engine's one GET.
struct JiraStandIn;

#[async_trait::async_trait]
impl runtime::Adapter for JiraStandIn {
    fn bootstrap(&self) -> runtime::Bootstrap {
        bootstrap()
    }
    fn bootstrap_v3(&self) -> runtime::Result<runtime::Bootstrap> {
        Ok(bootstrap())
    }
    async fn validate(&self, _: &str, document: Secret) -> runtime::Result<runtime::Baseline> {
        let response = http(&document)?
            .get(&["myself"], &[])
            .await
            .map_err(runtime::Failure::from_provider)?;
        match response.status {
            200 => {}
            401 => return Err(runtime::Failure::InvalidCredential),
            403 => return Err(runtime::Failure::Forbidden),
            429 | 500..=599 => return Err(runtime::Failure::Unavailable),
            _ => return Err(runtime::Failure::Protocol),
        }
        let body: Value =
            serde_json::from_slice(&response.body).map_err(|_| runtime::Failure::Protocol)?;
        let subject = body["accountId"]
            .as_str()
            .ok_or(runtime::Failure::Protocol)?;
        let now = connectors_sdk::now_ms();
        Ok(runtime::Baseline {
            binding: None,
            identity: registry::ExternalIdentity {
                kind: "atlassian.account".into(),
                subject: subject.into(),
            },
            granted_scopes: None,
            credential_expires_at_ms: None,
            collected_at_ms: now,
            valid_until_ms: now + 50_000,
        })
    }
    async fn invoke(&self, _: &str, _: &str, _: Secret, _: Value) -> runtime::Result<Value> {
        Err(runtime::Failure::Unsupported)
    }
    async fn invoke_bounded(
        &self,
        operation: &str,
        _: &str,
        document: Secret,
        input: Value,
        budget: runtime::ReadBudget,
    ) -> runtime::Result<Value> {
        budget.until()?;
        engine()
            .read(&http(&document)?, INSTANCE, operation, input)
            .await
            .map_err(runtime::Failure::from_provider)
    }
}

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn claude_mcp_journey_adapter_fixture() {
    if std::env::args().any(|arg| arg == "--connectors-private-fd") {
        let _ = runtime::serve(3, JiraStandIn);
    }
}

/// A loopback HTTP/1.1 Jira site: `GET /rest/api/3/myself` answers a
/// fictional account, `GET /rest/api/3/search/jql` [`SEARCH`] whatever the
/// query, anything else 404, and a request without [`AUTHORIZATION`] 401.
/// Every request line is kept with whether its header was the expected one.
struct Provider {
    address: String,
    requests: Arc<Mutex<Vec<(String, bool)>>>,
}

impl Provider {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                let seen = seen.clone();
                std::thread::spawn(move || {
                    let mut writer = stream.try_clone().unwrap();
                    let mut reader = BufReader::new(stream);
                    loop {
                        let mut line = String::new();
                        if reader.read_line(&mut line).unwrap_or(0) == 0 {
                            let _ = writer.shutdown(Shutdown::Both);
                            return;
                        }
                        let mut authorized = false;
                        loop {
                            let mut header = String::new();
                            if reader.read_line(&mut header).unwrap_or(0) == 0 {
                                return;
                            }
                            let header = header.trim_end();
                            if header.is_empty() {
                                break;
                            }
                            if let Some((name, value)) = header.split_once(':') {
                                authorized |= name.eq_ignore_ascii_case("authorization")
                                    && value.trim() == AUTHORIZATION;
                            }
                        }
                        let target = line.split_whitespace().nth(1).unwrap_or("").to_owned();
                        let route = target.split('?').next().unwrap_or("").to_owned();
                        seen.lock()
                            .unwrap()
                            .push((line.trim_end().to_owned(), authorized));
                        let (status, body) = match (authorized, route.as_str()) {
                            (false, _) => ("401 Unauthorized", r#"{"errorMessages":["fixture refusal"]}"#.to_owned()),
                            (true, "/rest/api/3/myself") => ("200 OK", r#"{"accountId":"fixture-account-0001","accountType":"atlassian","displayName":"Fixture Operator","active":true}"#.to_owned()),
                            (true, "/rest/api/3/search/jql") => ("200 OK", SEARCH.trim_end().to_owned()),
                            (true, _) => ("404 Not Found", r#"{"errorMessages":["no fixture"]}"#.to_owned()),
                        };
                        let answer = format!(
                            "HTTP/1.1 {status}\r\ncontent-type: application/json;charset=UTF-8\r\ncontent-length: {}\r\n\r\n{body}",
                            body.len()
                        );
                        if writer.write_all(answer.as_bytes()).is_err() {
                            return;
                        }
                    }
                });
            }
        });
        Self { address, requests }
    }
    /// Every request line the provider answered, and whether it carried the
    /// fixture's `Authorization`.
    fn requests(&self) -> Vec<(String, bool)> {
        self.requests.lock().unwrap().clone()
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

/// The configured entry `forge`: [`JiraStandIn`] as the instance `jira.work`
/// over `connectors-private/3`, granted every described read.
fn entry(container: &Container) -> config::Adapter {
    config::Adapter {
        instance_id: INSTANCE.into(),
        adapter_id: ADAPTER.into(),
        configuration_revision: "cfg-1".into(),
        credential_revision: None,
        protocol: connectors_core::WIRE_VERSION.into(),
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
            operations: engine()
                .declarations(&[Effect::Read])
                .into_iter()
                .map(|operation| operation.id)
                .collect(),
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

/// One jail with a connection made through the mounted credential, the
/// operation curation of the generic reads pinned to the entry, and an
/// enabled tools exposure of each operation in `exposed`.
struct Served {
    container: Container,
    provider: Provider,
}

impl Served {
    fn new(exposed: &[String]) -> Self {
        let provider = Provider::start();
        let container = Container::new(ADAPTER_FIXTURE);
        fs::write(container.path(PROVIDER), &provider.address).unwrap();
        container.deliver(PROFILE, CREDENTIAL);
        let adapter = entry(&container);
        configure(&container, &adapter);
        let init = success(&container.run(&["setup", "init"]));
        assert_eq!(init["disposition"], "state_initialized", "{init}");
        let connected = success(&container.connect(PROFILE));
        let connection = connected["connection"]["summary"]["connection"]
            .as_str()
            .unwrap_or_else(|| panic!("{connected}"))
            .to_owned();
        assert_eq!(
            connected["connection"]["external_identity"]["subject"], "fixture-account-0001",
            "{connected}"
        );
        // The stdio projection's declarations for a generic read: network
        // only, no approval, natural repeat semantics and the generic budget.
        let operations: Vec<Value> = generic_reads()
            .iter()
            .map(|operation| {
                json!({"operation":operation,"metadata":{"effects":["network"],"semantic_effects":[],"risk":"low",
                    "idempotency":{"kind":"natural"},"approval":"not_required","realization":"generic",
                    "limits":{"request_bytes":262144,"result_bytes":4194304,"execution_ms":40000,"provider_ms":30000,"connect_ms":5000}}})
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
        let exposures: Vec<Value> = exposed
            .iter()
            .map(|operation| {
                json!({"adapter_alias":ALIAS,"operation_ref":operation,"connection_ref":connection,
                    "families":["tools"],"enabled":true})
            })
            .collect();
        private(
            &container.path(COMPANION),
            json!({"format":"connectors-mcp-local/1","limits":{"frame_octets":1048576,"response_octets":33554432,
                "concurrent_requests":1,"request_milliseconds":120000},"exposures":exposures})
            .to_string(),
        );
        Self {
            container,
            provider,
        }
    }

    /// The search requests the provider answered.
    fn searches(&self) -> Vec<(String, bool)> {
        self.provider
            .requests()
            .into_iter()
            .filter(|(line, _)| line.starts_with("GET /rest/api/3/search/jql"))
            .collect()
    }

    /// The jailed CLI with exactly `args`, an empty environment, and the
    /// protocol on its standard streams.
    fn server(&self, args: &[&str]) -> Command {
        let mut command = self
            .container
            .jail
            .command("/bin-test/connectors", Map::Own);
        command.args(args).env_clear();
        command
    }
}

/// The documented launch: `--config CONFIG --state-dir STATE_DIR server
/// --transport stdio`.
const LAUNCH: [&str; 7] = [
    "--config",
    CONFIG,
    "--state-dir",
    STATE,
    "server",
    "--transport",
    "stdio",
];

/// One JSON-RPC 2.0 response frame, or why the stdout line is not one.
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

/// The client side of one server process: stdin, every stdout line checked by
/// [`frame`], and stderr apart.
struct Client {
    child: Child,
    frames: mpsc::Receiver<Result<Value, String>>,
    reader: Option<JoinHandle<()>>,
    errors: Option<JoinHandle<Vec<u8>>>,
}

impl Client {
    fn start(mut command: Command) -> Self {
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
    /// Write one frame exactly as given, newline-terminated.
    fn send(&mut self, line: &str) {
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(line.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
    }
    fn answer(&self) -> Value {
        match self.frames.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(value)) => value,
            Ok(Err(line)) => panic!("{line}"),
            Err(_) => panic!("the server did not answer"),
        }
    }
    /// Close stdin; the server exits successfully with nothing more on stdout.
    fn finish(mut self) {
        drop(self.child.stdin.take());
        let until = Instant::now() + Duration::from_secs(10);
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

impl Drop for Client {
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

/// The session's header and its frames, as recorded.
fn session() -> (Value, Vec<&'static str>) {
    let mut lines = SESSION.lines();
    let header: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    assert!(header.get("jsonrpc").is_none(), "{header}");
    let frames: Vec<&str> = lines.collect();
    assert_eq!(header["frames"], frames.len(), "{header}");
    (header, frames)
}

/// The recorded frame whose method is `method`, the `nth` of them.
fn recorded(method: &str, nth: usize) -> &'static str {
    session()
        .1
        .into_iter()
        .filter(|line| serde_json::from_str::<Value>(line).unwrap()["method"] == method)
        .nth(nth)
        .unwrap_or_else(|| panic!("the session has no {method} #{nth}"))
}

fn uuid(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        text.len() == 36
            && text.char_indices().all(|(index, c)| {
                if [8, 13, 18, 23].contains(&index) {
                    c == '-'
                } else {
                    c.is_ascii_hexdigit()
                }
            })
    })
}

fn sha256_hex(value: &Value) -> bool {
    value
        .as_str()
        .is_some_and(|text| text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Replace the value at `pointer`, when present, with `marker`, after `shape`
/// admits it.
fn mark(value: &mut Value, pointer: &str, shape: fn(&Value) -> bool, marker: &str) {
    if let Some(field) = value.pointer_mut(pointer) {
        assert!(shape(field), "{pointer}: {field}");
        *field = json!(marker);
    }
}

/// An answer with only what differs between runs replaced by a marker, each
/// field by name:
///
/// - `result.serverInfo.version` and
///   `result._meta["io.modelcontextprotocol/serverInfo"].version`: the
///   package version of the build that answers;
/// - `result._meta["io.beyond10x.connectors/revision"]` and the same member of
///   every tool's `_meta`: the projection revision, which digests the
///   configured executable, so each build has its own;
/// - `result.structuredContent.audit_ref` and `.request_id`, and the same two
///   members of the envelope `result.content[0].text` carries: the identities
///   of one execution.
///
/// The JSON-RPC `id` is not replaced: the replay sends the recorded ones.
fn normalized(mut answer: Value) -> Value {
    let version = |v: &Value| v.as_str().is_some_and(|text| !text.is_empty());
    mark(
        &mut answer,
        "/result/serverInfo/version",
        version,
        "<version>",
    );
    mark(
        &mut answer,
        "/result/_meta/io.modelcontextprotocol~1serverInfo/version",
        version,
        "<version>",
    );
    mark(
        &mut answer,
        "/result/_meta/io.beyond10x.connectors~1revision",
        sha256_hex,
        "<revision>",
    );
    if let Some(tools) = answer
        .pointer_mut("/result/tools")
        .and_then(Value::as_array_mut)
    {
        for tool in tools {
            mark(
                tool,
                "/_meta/io.beyond10x.connectors~1revision",
                sha256_hex,
                "<revision>",
            );
        }
    }
    for pointer in [
        "/result/structuredContent/audit_ref",
        "/result/structuredContent/request_id",
    ] {
        mark(&mut answer, pointer, uuid, "<execution>");
    }
    if let Some(text) = answer.pointer_mut("/result/content/0/text") {
        let mut envelope: Value = serde_json::from_str(text.as_str().unwrap()).unwrap();
        for pointer in ["/audit_ref", "/request_id"] {
            mark(&mut envelope, pointer, uuid, "<execution>");
        }
        *text = envelope;
    }
    answer
}

/// The answers to the requests of `frames` whose method is `method`.
fn answered<'a>(frames: &[&str], answers: &'a [Value], method: &str) -> Vec<&'a Value> {
    frames
        .iter()
        .filter(|line| {
            serde_json::from_str::<Value>(line)
                .unwrap()
                .get("id")
                .is_some()
        })
        .zip(answers)
        .filter(|(line, _)| serde_json::from_str::<Value>(line).unwrap()["method"] == method)
        .map(|(_, answer)| answer)
        .collect()
}

/// The tool name of `operation` on `jira.work`.
fn name(operation: &str) -> String {
    connectors_mcp::names::encode_name([INSTANCE, ADAPTER, operation]).unwrap()
}

/// `tools/list` holds exactly the generic reads, each read-only, and no
/// operation the selection declares as a write.
fn lists_the_reads_and_no_write(list: &Value) {
    let tools = list["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("{list}"));
    let listed: BTreeSet<String> = tools
        .iter()
        .map(|tool| {
            tool["_meta"][OPERATION_META]["operation"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    let reads: BTreeSet<String> = generic_reads().into_iter().collect();
    assert_eq!(listed, reads, "{list}");
    assert_eq!(listed.len(), 11, "the eleven Jira reads: {list}");
    for tool in tools {
        let operation = tool["_meta"][OPERATION_META]["operation"].as_str().unwrap();
        assert_eq!(tool["name"], name(operation), "{tool}");
        assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");
    }
    let writes = selected("write");
    assert!(!writes.is_empty());
    for write in writes {
        assert!(!listed.contains(&write), "{write} is listed: {list}");
    }
}

/// Replaying `session.jsonl` gives one answer per request, each equal to its
/// line of `session.expected.jsonl` once [`normalized`]; every stdout line is
/// one JSON-RPC response frame.
#[test]
fn claude_code_session_is_answered() {
    let (header, frames) = session();
    let expected: Vec<Value> = EXPECTED
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let served = Served::new(&generic_reads());
    let mut client = Client::start(served.server(&LAUNCH));
    let mut answers = Vec::new();
    for line in &frames {
        let request: Value = serde_json::from_str(line).unwrap();
        assert_eq!(
            request["params"]["clientInfo"]["version"].as_str().or(
                request["params"]["_meta"]["io.modelcontextprotocol/clientInfo"]["version"]
                    .as_str()
            ),
            header["client"]["version"].as_str(),
            "the header names the client that sent {line}"
        );
        client.send(line);
        if request.get("id").is_some() {
            let answer = client.answer();
            assert_eq!(answer["id"], request["id"], "{answer}");
            answers.push(answer);
        }
    }
    client.finish();
    assert_eq!(answers.len(), expected.len(), "one answer per request");
    for (answer, expected) in answers.iter().zip(&expected) {
        if let Some(version) =
            answer.pointer("/result/_meta/io.modelcontextprotocol~1serverInfo/version")
        {
            assert_eq!(version, env!("CARGO_PKG_VERSION"));
        }
        if let Some(envelope) = answer.pointer("/result/structuredContent") {
            let text = answer["result"]["content"][0]["text"].as_str().unwrap();
            assert_eq!(&serde_json::from_str::<Value>(text).unwrap(), envelope);
        }
        assert_eq!(normalized(answer.clone()), normalized(expected.clone()));
    }

    let by_method = |method: &str| answered(&frames, &answers, method);
    for discovered in by_method("server/discover") {
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
    // A session already on 2026-07-28 refuses the 2025-11-25 handshake; the
    // client carried on with the primary revision.
    for initialized in by_method("initialize") {
        assert_eq!(initialized["error"]["code"], -32600, "{initialized}");
    }
    let lists = by_method("tools/list");
    assert_eq!(lists.len(), 1);
    lists_the_reads_and_no_write(lists[0]);
    let revision = &lists[0]["result"]["_meta"][REVISION_META];
    for tool in lists[0]["result"]["tools"].as_array().unwrap() {
        assert_eq!(&tool["_meta"][REVISION_META], revision, "{tool}");
    }
    let calls = by_method("tools/call");
    assert_eq!(calls.len(), 2);
    for call in &calls {
        assert_eq!(call["result"]["isError"], false, "{call}");
        assert_eq!(
            call["result"]["structuredContent"]["status"], "success",
            "{call}"
        );
        assert_eq!(
            call["result"]["structuredContent"]["audit_status"], "complete",
            "{call}"
        );
    }
    let keys: Vec<&Value> = calls[1]["result"]["structuredContent"]["result"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| &item["key"])
        .collect();
    assert_eq!(keys, [&json!("DEMO-1"), &json!("DEMO-2"), &json!("DEMO-3")]);

    // Each call walked one page: one search request each, under the mounted
    // credential, with the JQL and page size the client chose.
    let searches = served.searches();
    assert_eq!(searches.len(), 2, "{searches:?}");
    for (line, authorized) in &searches {
        assert!(authorized, "{line}");
        assert!(line.contains("maxResults=100"), "{line}");
        assert!(line.contains("currentUser"), "{line}");
    }

    // The transcript's tool uses are the session's calls, by the client's
    // name for the server's tool.
    let called: Vec<String> = frames
        .iter()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|request| request["method"] == "tools/call")
        .map(|request| {
            format!(
                "mcp__connectors__{}",
                request["params"]["name"].as_str().unwrap()
            )
        })
        .collect();
    let used: Vec<String> = TRANSCRIPT
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|record| record["type"] == "assistant")
        .flat_map(|record| {
            record["message"]["content"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter(|content| content["type"] == "tool_use")
        .map(|content| content["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(used, called);
    assert_eq!(
        called[0],
        format!("mcp__connectors__{}", name("issues.search"))
    );
}

/// The registration line of `website/docs/connectors/mcp.md`, `claude mcp add
/// connectors -- connectors ...`: the arguments after the command name.
fn registered() -> Vec<String> {
    let line = MCP_PAGE
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("claude mcp add connectors -- "))
        .expect("mcp.md shows the claude mcp add registration line");
    let mut words = line
        .strip_prefix("claude mcp add connectors -- ")
        .unwrap()
        .split_whitespace();
    assert_eq!(words.next(), Some("connectors"), "{line}");
    words.map(str::to_owned).collect()
}

/// The page's registration arguments, with `CONFIG` and `STATE_DIR` naming
/// the jail's files, start the server; the recorded `initialize` as the
/// session's first frame is answered on 2025-11-25 with the tools capability,
/// and after `notifications/initialized` the recorded `tools/list` lists the
/// same tools the recorded session did.
#[test]
fn claude_mcp_add_arguments_start_the_server() {
    let arguments: Vec<String> = registered()
        .into_iter()
        .map(|word| match word.as_str() {
            "CONFIG" => CONFIG.to_owned(),
            "STATE_DIR" => STATE.to_owned(),
            _ => word,
        })
        .collect();
    assert_eq!(arguments, LAUNCH, "the documented launch");
    let served = Served::new(&generic_reads());
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let mut client = Client::start(served.server(&arguments));
    client.send(recorded("initialize", 0));
    let initialized = client.answer();
    assert_eq!(
        initialized["result"]["protocolVersion"], "2025-11-25",
        "{initialized}"
    );
    assert!(
        initialized["result"]["capabilities"]["tools"].is_object(),
        "{initialized}"
    );
    assert_eq!(initialized["result"]["serverInfo"]["name"], "connectors");
    assert_eq!(
        initialized["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );
    // Not in the recording: the client never finished this handshake.
    client.send(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
    client.send(recorded("tools/list", 0));
    let list = client.answer();
    client.finish();
    lists_the_reads_and_no_write(&list);
    let recorded_list = EXPECTED
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .find(|answer| answer["result"]["tools"].is_array())
        .unwrap();
    assert_eq!(
        normalized(list)["result"]["tools"],
        normalized(recorded_list)["result"]["tools"]
    );
    assert!(served.searches().is_empty(), "listing sent a search");
}

/// With every write the Jira selection declares exposed and enabled beside
/// its reads, the recorded `tools/list` still lists the eleven reads and no
/// write: under `connectors-private/3` the instance describes no write.
#[test]
fn no_write_tool_is_listed() {
    let writes = selected("write");
    let exposed: Vec<String> = generic_reads().into_iter().chain(writes).collect();
    let served = Served::new(&exposed);
    let mut client = Client::start(served.server(&LAUNCH));
    client.send(recorded("server/discover", 0));
    let discovered = client.answer();
    assert!(
        discovered["result"]["capabilities"]["tools"].is_object(),
        "{discovered}"
    );
    client.send(recorded("tools/list", 0));
    let list = client.answer();
    client.finish();
    lists_the_reads_and_no_write(&list);
    assert!(served.searches().is_empty(), "listing sent a search");
}
