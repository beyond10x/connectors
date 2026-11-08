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
//! `fields`. `constructed_initialize_first_session_lists_the_recorded_tools`
//! holds the `2025-11-25` handshake on its own, in a sequence constructed from
//! the recorded `initialize` frame that is not the recorded session.
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
/// One recorded `claude mcp add --scope project` run: its argv, exit status,
/// output (the project directory as `PROJECT_DIR`) and the `.mcp.json` it wrote.
const MCP_ADD: &str = include_str!("fixtures/claude-code/mcp-add.json");

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
///
/// Line 1 of `session.jsonl` is no frame: it is a header added in this
/// repository when the recording was copied in. It holds the recording format
/// `claude-code-session/1`, the client's name and version, the recording date,
/// the number of frames after it and a note on how they were captured; no
/// path, email or id. Every later line is a frame the client sent.
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

/// `tools/list` holds exactly the eleven generic reads, each by its encoded
/// name and marked read-only.
fn lists_the_eleven_reads(list: &Value) {
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
    lists_the_eleven_reads(lists[0]);
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

    // The transcript holds the session's calls. Each tool use, in order, is a
    // recorded `tools/call`: the client's name for the server's tool, the
    // call's arguments as its input (also as the wire input), and its id as
    // the call's `claudecode/toolUseId`. Each tool result answers that use
    // with the recorded answer's text content and structured content.
    let requests: Vec<Value> = frames
        .iter()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|request| request["method"] == "tools/call")
        .collect();
    let recorded_calls = answered(&frames, &expected, "tools/call");
    let records: Vec<Value> = TRANSCRIPT
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let parts = |record_type: &str, part_type: &str| -> Vec<(Value, Value)> {
        records
            .iter()
            .filter(|record| record["type"] == record_type)
            .flat_map(|record| {
                record["message"]["content"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|part| part["type"] == part_type)
                    .map(|part| (record.clone(), part))
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    let uses = parts("assistant", "tool_use");
    let results = parts("user", "tool_result");
    assert_eq!(uses.len(), requests.len(), "one tool use per call");
    assert_eq!(results.len(), requests.len(), "one tool result per call");
    for (((record, used), (outcome, result)), (request, answer)) in uses
        .iter()
        .zip(&results)
        .zip(requests.iter().zip(&recorded_calls))
    {
        let params = &request["params"];
        let id = used["id"].as_str().unwrap();
        assert_eq!(
            used["name"],
            format!("mcp__connectors__{}", params["name"].as_str().unwrap())
        );
        assert_eq!(used["input"], params["arguments"], "{used}");
        assert_eq!(record["wire_tool_inputs"][id], params["arguments"]);
        assert_eq!(params["_meta"]["claudecode/toolUseId"], id, "{request}");
        assert_eq!(result["tool_use_id"], id, "{result}");
        assert_eq!(
            result["content"], answer["result"]["content"][0]["text"],
            "{result}"
        );
        assert_eq!(outcome["tool_use_result"]["content"], result["content"]);
        assert_eq!(
            outcome["tool_use_result"]["structuredContent"],
            answer["result"]["structuredContent"]
        );
    }
    assert_eq!(requests[0]["params"]["name"], name("issues.search"));
    // The session's answer names every issue of the last call by key,
    // summary and status.
    let reply = records
        .iter()
        .find(|record| record["type"] == "result")
        .and_then(|record| record["result"].as_str())
        .unwrap();
    let items = recorded_calls[1]["result"]["structuredContent"]["result"]["items"]
        .as_array()
        .unwrap();
    assert_eq!(items.len(), 3);
    for item in items {
        for field in [
            &item["key"],
            &item["fields"]["summary"],
            &item["fields"]["status"]["name"],
        ] {
            assert!(reply.contains(field.as_str().unwrap()), "{field}: {reply}");
        }
    }
}

/// The `connectors` entry of each `--mcp-config` document
/// `website/docs/connectors/mcp.md` shows (each JSON block with `mcpServers`),
/// in page order; each document names that one server.
fn mcp_config_entries() -> Vec<Value> {
    let mut blocks = Vec::new();
    let mut block: Option<String> = None;
    for line in MCP_PAGE.lines() {
        let fence = line.trim();
        match block.take() {
            None if fence == "```json" => block = Some(String::new()),
            None => {}
            Some(text) if fence == "```" => blocks.push(text),
            Some(mut text) => {
                text.push_str(line);
                text.push('\n');
                block = Some(text);
            }
        }
    }
    blocks
        .iter()
        .filter_map(|text| serde_json::from_str::<Value>(text).ok())
        .filter(|value| value.get("mcpServers").is_some())
        .map(|config| {
            let servers = config["mcpServers"].as_object().unwrap();
            assert_eq!(
                servers.keys().collect::<Vec<_>>(),
                ["connectors"],
                "{config}"
            );
            servers["connectors"].clone()
        })
        .collect()
}

/// The members of an `--mcp-config` entry.
fn members(entry: &Value) -> BTreeSet<&str> {
    entry
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

/// The arguments of an `--mcp-config` entry, `CONFIG` and `STATE_DIR`
/// replaced by `config` and `state`.
fn arguments<'a>(entry: &'a Value, config: &'a str, state: &'a str) -> Vec<&'a str> {
    entry["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|word| match word.as_str().unwrap() {
            "CONFIG" => config,
            "STATE_DIR" => state,
            word => word,
        })
        .collect()
}

/// mcp.md shows two `--mcp-config` entries for the stdio server
/// `connectors`. The first is the one the recorded session ran: its command
/// is the capturing wrapper and its environment names the capture directory
/// (placeholders `CAPTURE_WRAPPER` and `CAPTURE_DIR`), and its arguments are
/// the user form's with the sandbox's `/srv/connectors` files in place of
/// `CONFIG` and `STATE_DIR`. The second is the user form: command
/// `connectors`, and with the jail's files in place of the placeholders, the
/// launch the replay starts. The `claude -p` command the page shows reads
/// the entry from `FILE` and allows only the search tool the recorded
/// session called.
#[test]
fn mcp_config_entry_is_the_replayed_launch() {
    let entries = mcp_config_entries();
    assert_eq!(
        entries.len(),
        2,
        "mcp.md shows the entry that ran and the user form"
    );
    let (ran, user) = (&entries[0], &entries[1]);
    assert_eq!(
        members(ran),
        BTreeSet::from(["args", "command", "env", "type"]),
        "{ran}"
    );
    assert_eq!(ran["type"], "stdio", "{ran}");
    assert_eq!(ran["command"], "CAPTURE_WRAPPER", "{ran}");
    assert_eq!(ran["env"], json!({"RIG_CAPTURE":"CAPTURE_DIR"}), "{ran}");
    assert_eq!(
        members(user),
        BTreeSet::from(["args", "command", "type"]),
        "{user}"
    );
    assert_eq!(user["type"], "stdio", "{user}");
    assert_eq!(user["command"], "connectors", "{user}");
    assert_eq!(
        arguments(ran, "CONFIG", "STATE_DIR"),
        arguments(user, "/srv/connectors/config.toml", "/srv/connectors/state"),
        "the run's arguments are the user form's on the sandbox's files"
    );
    assert_eq!(
        arguments(user, CONFIG, STATE),
        LAUNCH,
        "the replayed launch"
    );
    let command = MCP_PAGE
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("claude -p "))
        .expect("mcp.md shows the claude -p command that ran");
    for option in [
        "--mcp-config FILE".to_owned(),
        "--strict-mcp-config".to_owned(),
        r#"--tools """#.to_owned(),
        format!(
            r#"--allowedTools "mcp__connectors__{}""#,
            name("issues.search")
        ),
    ] {
        assert!(command.contains(&option), "{option}: {command}");
    }
}

/// The recorded `claude mcp add` run (`fixtures/claude-code/mcp-add.json`).
fn mcp_add() -> Value {
    serde_json::from_str(MCP_ADD).unwrap()
}

/// The `connectors` entry of the `.mcp.json` the recorded `claude mcp add`
/// wrote, the only server in it.
fn mcp_add_entry() -> Value {
    let recording = mcp_add();
    let servers = recording["written"][".mcp.json"]["mcpServers"]
        .as_object()
        .unwrap();
    assert_eq!(
        servers.keys().collect::<Vec<_>>(),
        ["connectors"],
        "{recording}"
    );
    servers["connectors"].clone()
}

/// The lines of each fenced block of mcp.md whose opening fence is `fence`,
/// in page order.
fn page_blocks(fence: &str) -> Vec<Vec<&'static str>> {
    let mut blocks = Vec::new();
    let mut block: Option<Vec<&'static str>> = None;
    for line in MCP_PAGE.lines() {
        match block.take() {
            None if line.trim() == fence => block = Some(Vec::new()),
            None => {}
            Some(lines) if line.trim() == "```" => blocks.push(lines),
            Some(mut lines) => {
                lines.push(line);
                block = Some(lines);
            }
        }
    }
    blocks
}

/// The recorded `claude mcp add --scope project` exited 0 with nothing on
/// stderr and wrote one server, `connectors`, to `.mcp.json`: type `stdio`,
/// command `connectors`, no environment, and as arguments the launch the
/// replay starts, on the `/srv/connectors` files. Its argv after `--` is that
/// command and those arguments.
#[test]
fn recorded_mcp_add_entry_is_the_replayed_launch() {
    let recording = mcp_add();
    assert_eq!(recording["exit"], 0, "{recording}");
    assert_eq!(recording["stderr"], json!([]), "{recording}");
    let entry = mcp_add_entry();
    assert_eq!(
        members(&entry),
        BTreeSet::from(["args", "command", "env", "type"]),
        "{entry}"
    );
    assert_eq!(entry["type"], "stdio", "{entry}");
    assert_eq!(entry["command"], "connectors", "{entry}");
    assert_eq!(entry["env"], json!({}), "{entry}");
    assert_eq!(
        arguments(&entry, "CONFIG", "STATE_DIR"),
        LAUNCH,
        "the replayed launch"
    );
    let argv: Vec<&str> = recording["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|word| word.as_str().unwrap())
        .collect();
    let separator = argv.iter().position(|word| *word == "--").unwrap();
    assert_eq!(argv[separator + 1], "connectors", "{argv:?}");
    assert_eq!(argv[separator + 2..], LAUNCH, "{argv:?}");
}

/// The entry the recorded `claude mcp add` wrote is the user form mcp.md
/// shows, its `CONFIG` and `STATE_DIR` filled with the `/srv/connectors` files
/// as `mcp_config_entry_is_the_replayed_launch` fills them, plus an empty
/// `env`.
#[test]
fn recorded_mcp_add_entry_is_the_documented_user_form() {
    let entries = mcp_config_entries();
    let user = entries.last().expect("mcp.md shows the user form");
    assert_eq!(user["command"], "connectors", "{user}");
    let mut filled = user.clone();
    filled["args"] = json!(arguments(
        user,
        "/srv/connectors/config.toml",
        "/srv/connectors/state"
    ));
    filled["env"] = json!({});
    assert_eq!(mcp_add_entry(), filled);
}

/// mcp.md shows the `claude mcp add` command that was recorded, word for word
/// with `CONFIG` and `STATE_DIR` for the `/srv/connectors` files, the two lines
/// it printed, and the `.mcp.json` it wrote.
#[test]
fn mcp_page_shows_the_recorded_mcp_add() {
    let recording = mcp_add();
    let command = MCP_PAGE
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("claude mcp add "))
        .expect("mcp.md shows the claude mcp add command");
    let words: Vec<&str> = command
        .split_whitespace()
        .map(|word| match word {
            "CONFIG" => CONFIG,
            "STATE_DIR" => STATE,
            word => word,
        })
        .collect();
    assert_eq!(json!(words), recording["argv"], "{command}");
    let output = page_blocks("```text");
    assert!(
        output
            .iter()
            .any(|lines| json!(lines) == recording["stdout"]),
        "mcp.md shows the two lines claude mcp add printed: {}",
        recording["stdout"]
    );
    let written = page_blocks(r#"```json title=".mcp.json""#);
    assert_eq!(written.len(), 1, "mcp.md shows the .mcp.json it wrote");
    let shown: Value = serde_json::from_str(&written[0].join("\n")).unwrap();
    assert_eq!(shown, recording["written"][".mcp.json"]);
}

/// Not the recorded session: a sequence constructed from its recorded
/// `initialize` frame, sent first, as a client on `2025-11-25` would. It is
/// answered on 2025-11-25 with the tools capability, and after a
/// `notifications/initialized` the recorded `tools/list` lists the same tools
/// the recorded session did.
#[test]
fn constructed_initialize_first_session_lists_the_recorded_tools() {
    let served = Served::new(&generic_reads());
    let mut client = Client::start(served.server(&LAUNCH));
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
    lists_the_eleven_reads(&list);
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

/// The operations [`lists_exactly_the_recorded_read_tools`] exposes: the
/// eleven generic reads the stand-in describes, nothing else.
fn listed_exposures() -> Vec<String> {
    generic_reads()
}

/// The recorded `server/discover` and `tools/list` alone: the server lists
/// exactly the eleven read tools the recorded `tools/list` answer holds, by
/// name and in its order, and each tool equals the recorded one once
/// [`normalized`]. Listing sends no search.
#[test]
fn lists_exactly_the_recorded_read_tools() {
    let served = Served::new(&listed_exposures());
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
    let recorded_list = EXPECTED
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .find(|answer| answer["result"]["tools"].is_array())
        .unwrap();
    let names = |list: &Value| -> Vec<String> {
        list["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(names(&list), names(&recorded_list));
    assert_eq!(names(&list).len(), 11, "{list}");
    lists_the_eleven_reads(&list);
    assert_eq!(
        normalized(list)["result"]["tools"],
        normalized(recorded_list)["result"]["tools"]
    );
    assert!(served.searches().is_empty(), "listing sent a search");
}

/// Adversary probe (expected green): a server that changed what a client
/// depends on would answer differently from `session.expected.jsonl`, and
/// [`normalized`] keeps each such difference visible. Each mutation is made
/// on an in-memory copy of the expected answers, never on the fixture.
#[test]
fn adversary_normalization_keeps_what_a_client_depends_on() {
    let expected: Vec<Value> = EXPECTED
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let search = expected[3]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .position(|tool| tool["_meta"][OPERATION_META]["operation"] == "issues.search")
        .unwrap();
    type Mutation = (&'static str, usize, fn(&mut Value, usize));
    let mutations: [Mutation; 8] = [
        ("tool name", 3, |v, i| {
            v["result"]["tools"][i]["name"] = json!("issues_search")
        }),
        ("input schema", 3, |v, i| {
            v["result"]["tools"][i]["inputSchema"]["required"] = json!([])
        }),
        ("readOnlyHint", 3, |v, i| {
            v["result"]["tools"][i]["annotations"]["readOnlyHint"] = json!(false)
        }),
        ("structuredContent", 5, |v, _| {
            v["result"]["structuredContent"]["result"]["items"][0]["key"] = json!("DEMO-9")
        }),
        ("text content", 5, |v, _| {
            let text = v["result"]["content"][0]["text"].as_str().unwrap();
            v["result"]["content"][0]["text"] = json!(text.replace("DEMO-1", "DEMO-9"))
        }),
        ("isError", 5, |v, _| v["result"]["isError"] = json!(true)),
        ("supportedVersions", 0, |v, _| {
            v["result"]["supportedVersions"] = json!(["2026-07-28"])
        }),
        ("late initialize -32600", 1, |v, _| {
            v["error"]["code"] = json!(-32601)
        }),
    ];
    for (label, line, mutate) in mutations {
        let mut mutated = expected[line].clone();
        mutate(&mut mutated, search);
        assert_ne!(
            normalized(mutated),
            normalized(expected[line].clone()),
            "normalization hides a change of the {label}"
        );
    }
}

/// Adversary case: the transcript's ids are placeholders, and so is every
/// thinking `signature`; no API-issued blob is kept. The replay reads the
/// transcript's tool uses and results, never a signature.
#[test]
fn adversary_transcript_keeps_no_api_issued_signature() {
    for line in TRANSCRIPT.lines() {
        let record: Value = serde_json::from_str(line).unwrap();
        for content in record["message"]["content"]
            .as_array()
            .cloned()
            .unwrap_or_default()
        {
            if let Some(signature) = content.get("signature") {
                let text = signature.as_str().unwrap_or_default();
                assert!(
                    text.contains("fixture"),
                    "a raw API-issued thinking signature of {} characters is kept",
                    text.len()
                );
            }
        }
    }
}

/// Adversary case, restated from a probe of the write case it replaced:
/// [`lists_exactly_the_recorded_read_tools`] exposes only operations the
/// stand-in describes, each a read, and no write of the Jira selection, so
/// it holds the listed reads and claims nothing about writes. The write
/// guard is held elsewhere.
#[test]
fn adversary_list_case_exposes_no_write() {
    let Ok(descriptor) = bootstrap().descriptor() else {
        panic!("the stand-in's descriptor")
    };
    let described: BTreeSet<String> = descriptor
        .operations
        .iter()
        .map(|operation| operation.id.clone())
        .collect();
    let reads: BTreeSet<String> = engine()
        .declarations(&[Effect::Read])
        .into_iter()
        .map(|operation| operation.id)
        .collect();
    assert_eq!(described, reads, "the stand-in describes reads only");
    let writes = selected("write");
    assert!(!writes.is_empty(), "the Jira selection declares writes");
    for exposed in listed_exposures() {
        assert!(described.contains(&exposed), "{exposed} is not described");
        assert!(!writes.contains(&exposed), "{exposed} is a write");
    }
}
