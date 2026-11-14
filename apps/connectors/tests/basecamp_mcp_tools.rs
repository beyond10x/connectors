//! story:catalog-basecamp. The production `connectors server --transport
//! stdio`, with a Basecamp instance, lists the thirteen shipped Basecamp reads
//! as tools, and a listed tool reads a page and answers the next page's URL
//! from Basecamp's `Link` header as `next`.
//!
//! The server, its owner and an adapter child run in the jail of
//! `support/mounted.rs` against the read-only `mounted_secret` binding, as in
//! `claude_mcp_journey.rs`. The child stands in for the catalog provider's
//! instance `basecamp.work`: it describes the instance through the catalog
//! engine (`connectors_catalog_provider::Engine`) from the shipped Basecamp
//! bundle and selection, built as the provider builds it for the endpoint
//! `https://<host>/12345` (document base `/`, the account path in front of
//! every request, and `rel="next"` held under that endpoint), so the tool
//! names and schemas are the ones the provider gives. It reads through the
//! engine and the host's HTTP transport, in plaintext, from a Basecamp this
//! test serves on the loopback; the endpoint the engine holds a `rel="next"`
//! to is that server's address under `https://`, which is the URL the server's
//! `Link` header names. (The catalog provider itself speaks HTTPS only and is
//! not built for this package's tests; `adapters/catalog/tests/basecamp.rs`
//! runs it over HTTPS.)
//! Linux only: the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use connectors_catalog_provider::{Effect, Engine, Selection};
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

const ADAPTER_FIXTURE: &str = "basecamp_mcp_tools_adapter_fixture";
/// Inside the jail: the loopback address of the test's Basecamp.
const PROVIDER: &str = "/work/provider";
const COMPANION: &str = "/srv/connectors/config.toml.mcp-stdio.json";
const CURATION: &str = "/srv/connectors/config.toml.operation-curation.json";
const INSTANCE: &str = "basecamp.work";
const ADAPTER: &str = "catalog";
const PROFILE: &str = "basecamp.pat";
const ACCOUNT: &str = "12345";
/// A fictional personal access token.
const TOKEN: &str = concat!("fixture-basecamp-", "personal-access-token");
/// The profile's document: the token alone.
const CREDENTIAL: &str = concat!(
    r#"{"token":""#,
    "fixture-basecamp-",
    "personal-access-token",
    r#""}"#
);
const OPERATION_META: &str = "io.beyond10x.connectors/operation";
const BUNDLE: &[u8] =
    include_bytes!("../../../adapters/catalog/generated/bundles/basecamp.bundle.json");
const SELECTION: &str =
    include_str!("../../../adapters/catalog/providers/basecamp/operations.json");
/// The thirteen reads `docs/catalog-basecamp.md` lists.
const READS: [&str; 13] = [
    "projects.list",
    "project.get",
    "project.people.list",
    "message_board.get",
    "messages.list",
    "message.get",
    "comments.list",
    "comment.get",
    "todoset.get",
    "todolists.list",
    "todolist.get",
    "todos.list",
    "todo.get",
];

/// The endpoint the engine holds a `rel="next"` under: the test Basecamp's
/// address and the account, as an instance endpoint names them.
fn endpoint() -> String {
    let address = fs::read_to_string(PROVIDER).unwrap();
    format!("https://{}/{ACCOUNT}/", address.trim())
}

/// The catalog engine over the shipped Basecamp bundle and selection, as the
/// catalog provider builds it for an instance whose endpoint is
/// [`endpoint`]: the account path is the request prefix, so the bundle's
/// paths sit under the document base `/`.
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let bundle: connectors_catalog::bundle::Bundle =
            connectors_core::read_json(BUNDLE).unwrap();
        let selection: Value = serde_json::from_str(SELECTION).unwrap();
        let selections: Vec<Selection> =
            serde_json::from_value(selection["operations"].clone()).unwrap();
        let engine = Engine::new(&bundle, "/", &selections).unwrap();
        // Listing needs no provider; only a read reads the address.
        match fs::read_to_string(PROVIDER) {
            Ok(_) => engine.with_link_origin(&endpoint()).unwrap(),
            Err(_) => engine,
        }
    })
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
            label: "Basecamp personal access token".into(),
            max_bytes: 8192,
        }],
        acquisition: None,
    }
}

/// The description the catalog provider gives `basecamp.work` under
/// `connectors-private/3`: the selection's reads and no write, under the one
/// bearer profile.
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
        provider_authority: "https://3.basecampapi.com/12345/".into(),
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

/// Every described operation: the stdio projection advertises a `generic`
/// read of profile `generic-http`.
fn reads() -> Vec<String> {
    engine()
        .declarations(&[Effect::Read, Effect::Write])
        .into_iter()
        .map(|operation| {
            assert_eq!(operation.profile, "generic-http", "{}", operation.id);
            operation.id
        })
        .collect()
}

/// The `Authorization` value of the token.
struct Bearer(Secret);

#[async_trait::async_trait]
impl Credential for Bearer {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(self.0.0.clone()))
    }
}

/// The plaintext transport to the account on the provider named in
/// [`PROVIDER`], with the bearer header of `document` (`{"token"}`).
fn http(document: &Secret) -> runtime::Result<ScopedHttp> {
    static HTTP: OnceLock<ScopedHttp> = OnceLock::new();
    let entry: Value =
        serde_json::from_slice(&document.0).map_err(|_| runtime::Failure::InvalidCredential)?;
    let Some(token) = entry["token"].as_str() else {
        return Err(runtime::Failure::InvalidCredential);
    };
    let header = Secret(format!("Bearer {token}").into_bytes());
    let base = HTTP.get_or_init(|| {
        let address = fs::read_to_string(PROVIDER).unwrap();
        ScopedHttp::new_with_ca_bytes(
            &HttpConfig {
                base_url: format!("http://{}/{ACCOUNT}/", address.trim()),
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
    Ok(base.with_credential(Arc::new(Bearer(header))))
}

/// The adapter child: the catalog provider's description of `basecamp.work`;
/// validation is the declared identity read (`GET my/profile.json`, subject
/// `/id`); a bounded read is the engine's one GET.
struct BasecampStandIn;

#[async_trait::async_trait]
impl runtime::Adapter for BasecampStandIn {
    fn bootstrap(&self) -> runtime::Bootstrap {
        bootstrap()
    }
    fn bootstrap_v3(&self) -> runtime::Result<runtime::Bootstrap> {
        Ok(bootstrap())
    }
    async fn validate(&self, _: &str, document: Secret) -> runtime::Result<runtime::Baseline> {
        let response = http(&document)?
            .get(&["my", "profile.json"], &[])
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
        let subject = body["id"].as_u64().ok_or(runtime::Failure::Protocol)?;
        let now = connectors_sdk::now_ms();
        Ok(runtime::Baseline {
            binding: None,
            identity: registry::ExternalIdentity {
                kind: "basecamp.person".into(),
                subject: subject.to_string(),
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
fn basecamp_mcp_tools_adapter_fixture() {
    if std::env::args().any(|arg| arg == "--connectors-private-fd") {
        let _ = runtime::serve(3, BasecampStandIn);
    }
}

/// A loopback HTTP/1.1 Basecamp account `12345`: `GET /12345/my/profile.json`
/// answers a fictional person, `GET /12345/projects.json` a page of one
/// project, the first page with a `Link` to the second, anything else 404,
/// and a request without the bearer token 401. Every request line is kept
/// with whether its header was the token.
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
        let host = address.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                let seen = seen.clone();
                let host = host.clone();
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
                                    && value.trim() == format!("Bearer {TOKEN}");
                            }
                        }
                        let target = line.split_whitespace().nth(1).unwrap_or("").to_owned();
                        seen.lock()
                            .unwrap()
                            .push((line.trim_end().to_owned(), authorized));
                        let project = |id: u64| {
                            json!([{"id": id, "name": format!("Fixture project {id}"), "dock": []}])
                                .to_string()
                        };
                        let (status, link, body) = match (authorized, target.as_str()) {
                            (false, _) => (
                                "401 Unauthorized",
                                None,
                                r#"{"error":"fixture refusal"}"#.to_owned(),
                            ),
                            (true, "/12345/my/profile.json") => (
                                "200 OK",
                                None,
                                r#"{"id":1049715913,"name":"Fixture Person"}"#.to_owned(),
                            ),
                            (true, "/12345/projects.json") => (
                                "200 OK",
                                Some(format!(
                                    "<https://{host}/12345/projects.json?page=2>; rel=\"next\""
                                )),
                                project(1),
                            ),
                            (true, "/12345/projects.json?page=2") => ("200 OK", None, project(2)),
                            (true, _) => (
                                "404 Not Found",
                                None,
                                r#"{"error":"no fixture"}"#.to_owned(),
                            ),
                        };
                        let link = link
                            .map(|link| format!("link: {link}\r\n"))
                            .unwrap_or_default();
                        let answer = format!(
                            "HTTP/1.1 {status}\r\ncontent-type: application/json; charset=utf-8\r\n{link}content-length: {}\r\n\r\n{body}",
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
    /// token.
    fn requests(&self) -> Vec<(String, bool)> {
        self.requests.lock().unwrap().clone()
    }
}

/// Write `text` owner-only, as the server requires of its inputs.
fn private(path: &Path, text: String) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

/// The configured entry `forge`: [`BasecampStandIn`] as the instance
/// `basecamp.work` over `connectors-private/3`, granted every described read.
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
            operations: reads().into_iter().collect(),
            custody: Default::default(),
        },
    }
}

fn clock_key(seed: u8) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode([seed; 32])
}

/// The configuration: mounted custody, a two-source approval clock (never
/// sampled here; the server's launch requires one) and `adapter` as `forge`.
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

/// One jail with a connection made through the mounted token, the operation
/// curation of every read pinned to the entry, and an enabled tools exposure
/// of each.
struct Served {
    container: Container,
    provider: Provider,
}

impl Served {
    fn new() -> Self {
        let provider = Provider::start();
        let container = Container::new(ADAPTER_FIXTURE);
        fs::write(container.path(PROVIDER), &provider.address).unwrap();
        container.deliver(PROFILE, CREDENTIAL.as_bytes());
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
            connected["connection"]["external_identity"]["subject"], "1049715913",
            "{connected}"
        );
        // The stdio projection's declarations for a generic read: network
        // only, no approval, natural repeat semantics and the generic budget.
        let operations: Vec<Value> = reads()
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
        let exposures: Vec<Value> = reads()
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
    /// Send `request` and wait for its answer.
    fn ask(&mut self, request: Value) -> Value {
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(request.to_string().as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        if request.get("id").is_none() {
            return Value::Null;
        }
        let answer = match self.frames.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(value)) => value,
            Ok(Err(line)) => panic!("{line}"),
            Err(_) => panic!("the server did not answer"),
        };
        assert_eq!(answer["id"], request["id"], "{answer}");
        answer
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

/// The tool name of `operation` on `basecamp.work`.
fn name(operation: &str) -> String {
    connectors_mcp::names::encode_name([INSTANCE, ADAPTER, operation]).unwrap()
}

/// The structured result of a successful `tools/call`.
fn called(answer: &Value) -> &Value {
    assert_eq!(answer["result"]["isError"], false, "{answer}");
    let envelope = &answer["result"]["structuredContent"];
    assert_eq!(envelope["status"], "success", "{answer}");
    &envelope["result"]
}

/// After the handshake, `tools/list` holds exactly the thirteen reads, each
/// by its encoded name, read-only and without `accountId` in its input; the
/// listed `projects.list` reads the first page under the account with the
/// token and answers the second page's URL as `next`, and the second page,
/// read by its `page`, answers none.
#[test]
fn the_stdio_server_lists_the_thirteen_reads_and_one_reads_a_page_with_its_next() {
    let served = Served::new();
    let mut client = Client::start(served.server(&LAUNCH));
    let initialized = client.ask(json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{
        "protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"fixture-client","version":"1"}}}));
    assert_eq!(
        initialized["result"]["protocolVersion"], "2025-11-25",
        "{initialized}"
    );
    client.ask(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
    let list = client.ask(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}));
    let tools = list["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("{list}"));
    let listed: BTreeSet<&str> = tools
        .iter()
        .map(|tool| tool["_meta"][OPERATION_META]["operation"].as_str().unwrap())
        .collect();
    assert_eq!(listed, BTreeSet::from(READS), "{list}");
    assert_eq!(tools.len(), 13, "{list}");
    for tool in tools {
        let operation = tool["_meta"][OPERATION_META]["operation"].as_str().unwrap();
        assert_eq!(tool["name"], name(operation), "{tool}");
        assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");
        assert!(
            !tool["inputSchema"].to_string().contains("accountId"),
            "{tool}"
        );
    }
    assert!(
        served.provider.requests().len() == 1,
        "listing sent a request: {:?}",
        served.provider.requests()
    );

    let first = client.ask(
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{
        "name":name("projects.list"),"arguments":{"input":{}}}}),
    );
    let page = called(&first);
    assert_eq!(page["status"], 200, "{first}");
    assert_eq!(page["body"][0]["id"], 1, "{first}");
    let address = &served.provider.address;
    assert_eq!(
        page["next"],
        format!("https://{address}/12345/projects.json?page=2"),
        "{first}"
    );
    let second = client.ask(
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
        "name":name("projects.list"),"arguments":{"input":{"page":2}}}}),
    );
    let page = called(&second);
    assert_eq!(page["body"][0]["id"], 2, "{second}");
    assert!(page.get("next").is_none_or(Value::is_null), "{second}");
    client.finish();

    // Connecting read the person; each call read one page, under the account
    // and with the token.
    let requests = served.provider.requests();
    let lines: Vec<&str> = requests.iter().map(|(line, _)| line.as_str()).collect();
    assert_eq!(
        lines,
        [
            "GET /12345/my/profile.json HTTP/1.1",
            "GET /12345/projects.json HTTP/1.1",
            "GET /12345/projects.json?page=2 HTTP/1.1",
        ],
        "{requests:?}"
    );
    assert!(
        requests.iter().all(|(_, authorized)| *authorized),
        "{requests:?}"
    );
}
