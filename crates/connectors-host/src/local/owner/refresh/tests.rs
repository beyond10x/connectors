//! story:owner-rotating-refresh against a fake token server on `127.0.0.1`
//! behind TLS with a per-test self-signed root. Every token here is a
//! fictional fixture value.
use super::*;
use crate::local::{
    metadata::{Metadata, RefreshState},
    registry::{Binding, ExternalIdentity, State},
};
use rustls::pki_types::PrivatePkcs8KeyDer;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
    },
};

const CLIENT: &str = "fixture-client";
const ACCESS: &str = "fixture-access-token-one";
const REFRESHED_ACCESS: &str = "fixture-access-token-two";
const REFRESH: &str = "fixture-refresh-token-one";
const ROTATED: &str = "fixture-refresh-token-two";
const INSTANCE: &str = "fixture-instance";

const RESERVE: &str = "connectors.refresh.ReserveRefresh";
const AUTHORIZE: &str = "connectors.refresh.AuthorizeExchange";
const STORE: &str = "connectors.refresh.StoreResponse";
const QUARANTINE: &str = "connectors.refresh.QuarantineRefresh";
const RELEASE: &str = "connectors.refresh.ReleaseRateLimited";
const RECOVER: &str = "connectors.refresh.RecoverStoredResponse";
const PUBLISH: &str = "connectors.refresh.PublishRefresh";
const DISCARD: &str = "connectors.refresh.DiscardCandidate";

/// One scripted answer of the fake token server.
struct Reply {
    status: u16,
    body: Value,
    retry_after: Option<&'static str>,
}

fn token(refresh: Option<&str>, scope: Option<&str>) -> Reply {
    let mut body = json!({
        "access_token": REFRESHED_ACCESS,
        "token_type": "Bearer",
        "expires_in": 3600,
    });
    if let Some(refresh) = refresh {
        body["refresh_token"] = refresh.into();
    }
    if let Some(scope) = scope {
        body["scope"] = scope.into();
    }
    Reply {
        status: 200,
        body,
        retry_after: None,
    }
}

fn error(status: u16, code: &str) -> Reply {
    Reply {
        status,
        body: json!({"error": code}),
        retry_after: None,
    }
}

/// The server reads the request, then closes the connection unanswered.
fn dropped() -> Reply {
    Reply {
        status: 0,
        body: Value::Null,
        retry_after: None,
    }
}

struct Shared {
    replies: Mutex<VecDeque<Reply>>,
    requests: Mutex<Vec<Vec<u8>>>,
    delay: Duration,
    stop: AtomicBool,
}

/// The fake token server. Dropping it stops its thread.
struct Server {
    port: u16,
    root: String,
    shared: Arc<Shared>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    fn start(replies: Vec<Reply>) -> Self {
        Self::delayed(replies, Duration::ZERO)
    }

    /// Answers each request `delay` after it arrived.
    fn delayed(replies: Vec<Reply>, delay: Duration) -> Self {
        let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![certified.cert.der().clone()],
            PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der()).into(),
        )
        .unwrap();
        let tls = Arc::new(tls);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let shared = Arc::new(Shared {
            replies: Mutex::new(replies.into()),
            requests: Mutex::new(Vec::new()),
            delay,
            stop: AtomicBool::new(false),
        });
        let served = shared.clone();
        let thread = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if served.stop.load(Ordering::SeqCst) {
                    return;
                }
                if let Ok(stream) = stream {
                    serve(stream, &tls, &served);
                }
            }
        });
        Self {
            port,
            root: certified.cert.pem(),
            shared,
            thread: Some(thread),
        }
    }

    fn url(&self) -> String {
        format!("https://localhost:{}/token", self.port)
    }

    /// The form fields of every request the server read.
    fn requests(&self) -> Vec<BTreeMap<String, String>> {
        self.shared
            .requests
            .lock()
            .unwrap()
            .iter()
            .map(|body| url::form_urlencoded::parse(body).into_owned().collect())
            .collect()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(stream: TcpStream, tls: &Arc<rustls::ServerConfig>, shared: &Shared) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let Ok(connection) = rustls::ServerConnection::new(tls.clone()) else {
        return;
    };
    let mut stream = rustls::StreamOwned::new(connection, stream);
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() > 65_536 || !matches!(stream.read(&mut byte), Ok(1)) {
            return;
        }
        head.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let length: usize = head
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
                .and_then(|(_, value)| value.trim().parse().ok())
        })
        .unwrap_or(0);
    let mut body = vec![0_u8; length.min(65_536)];
    if stream.read_exact(&mut body).is_err() {
        return;
    }
    shared.requests.lock().unwrap().push(body);
    std::thread::sleep(shared.delay);
    let reply = shared
        .replies
        .lock()
        .unwrap()
        .pop_front()
        .unwrap_or_else(|| error(500, "server_error"));
    if reply.status == 0 {
        return;
    }
    let body = serde_json::to_vec(&reply.body).unwrap();
    let mut out = format!(
        "HTTP/1.1 {} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        body.len()
    );
    if let Some(delay) = reply.retry_after {
        out.push_str(&format!("Retry-After: {delay}\r\n"));
    }
    out.push_str("\r\n");
    let mut bytes = out.into_bytes();
    bytes.extend_from_slice(&body);
    let _ = stream.write_all(&bytes);
    let _ = stream.flush();
    stream.conn.send_close_notify();
    let _ = stream.flush();
}

/// Custody in memory, by version id.
#[derive(Default)]
struct Memory {
    versions: Mutex<BTreeMap<uuid::Uuid, Vec<u8>>>,
}

impl Memory {
    fn entry(&self, version: &str) -> Value {
        let id = uuid::Uuid::parse_str(version).unwrap();
        serde_json::from_slice(&self.versions.lock().unwrap()[&id]).unwrap()
    }
}

impl Custody for Arc<Memory> {
    fn read(&self, version: custody::Version) -> Result<Secret, custody::Failure> {
        self.versions
            .lock()
            .unwrap()
            .get(&version.id())
            .map(|bytes| Secret(bytes.clone()))
            .ok_or(custody::Failure::Missing)
    }
    fn writable(&self, _: custody::Version) -> bool {
        true
    }
    fn write_new(
        &self,
        version: custody::Version,
        material: &Secret,
        guard: &mut dyn FnMut() -> Result<(), custody::Failure>,
    ) -> Result<custody::WrittenVersion, custody::Failure> {
        let mut versions = self.versions.lock().unwrap();
        if versions.contains_key(&version.id()) {
            return Err(custody::Failure::Conflict);
        }
        guard()?;
        versions.insert(version.id(), material.0.clone());
        Ok(custody::WrittenVersion::fixture(version))
    }
}

/// A published `oauth2_grant` connection whose entry holds [`REFRESH`].
struct Connected {
    _root: tempfile::TempDir,
    path: PathBuf,
    binding: Binding,
    connection: String,
    custody: Arc<Memory>,
}

/// The connection's active generation, custody version and private fence.
#[derive(Debug, PartialEq, Eq)]
struct Active {
    generation: String,
    material: String,
    fence: String,
}

impl Connected {
    fn new(server: &Server, rotation: &str, scopes: &[&str]) -> Self {
        Self::with(server, rotation, scopes, 3_600_000, true)
    }

    /// A connection whose profile names `token_url` as its token endpoint.
    fn at(token_url: &str, rotation: &str, scopes: &[&str]) -> Self {
        Self::build(token_url, rotation, scopes, 3_600_000, true)
    }

    /// A connection whose access token expires `expires_in_ms` from now; with
    /// `grant` false its profile declares no authorization server.
    fn with(
        server: &Server,
        rotation: &str,
        scopes: &[&str],
        expires_in_ms: u64,
        grant: bool,
    ) -> Self {
        Self::build(&server.url(), rotation, scopes, expires_in_ms, grant)
    }

    fn build(
        token_url: &str,
        rotation: &str,
        scopes: &[&str],
        expires_in_ms: u64,
        grant: bool,
    ) -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        crate::local::filesystem::directory(&path, true, true).unwrap();
        drop(Metadata::initialize(&path).unwrap());
        let mut binding = registry::fixture_binding(INSTANCE);
        binding.profile.oauth2 = grant.then(|| {
            serde_json::from_value(json!({
                "authorize_url": "https://localhost/fixture/authorize",
                "token_url": token_url,
                "device_authorization_url": null,
                "client_id": CLIENT,
                "client_kind": "public",
                "request_encoding": "form",
                "refresh_encoding": "form",
                "redirect_kind": "loopback_any_port",
                "redirect_uri": null,
                "authorize_parameters": [],
                "requested_scopes": ["read"],
                "required_scopes": ["read"],
                "rotation": rotation,
                "refresh_lead_seconds": 60,
                "grant_types": ["authorization_code"],
                "sources": ["https://localhost/fixture-authorization-server"],
                "state_in_exchange": false,
                "scope_in_refresh": false,
            }))
            .unwrap()
        });
        let registry = Registry::new(&path);
        let at = now();
        let entry = serde_json::to_vec(&json!({
            "client_id": CLIENT,
            "access_token": ACCESS,
            "refresh_token": REFRESH,
            "expires_at_ms": at + expires_in_ms,
            "scopes": scopes,
        }))
        .unwrap();
        let acquisition = registry.begin(&binding, at).unwrap();
        let claim = registry.consume(acquisition, at).unwrap();
        let baseline = ValidatedBaseline {
            identity: ExternalIdentity {
                kind: "fixture-account".into(),
                subject: "one".into(),
            },
            granted_scopes: Some(scopes.iter().map(|scope| (*scope).to_owned()).collect()),
            credential_expires_at_ms: Some(at + expires_in_ms),
            collected_at_ms: at,
            valid_until_ms: at + expires_in_ms.min(60_000),
            binding: None,
        };
        let prepared = registry.prepare(&claim, baseline, entry.len(), at).unwrap();
        let custody = Arc::new(Memory::default());
        custody
            .versions
            .lock()
            .unwrap()
            .insert(prepared.version().id(), entry);
        let receipt = custody::WrittenVersion::fixture(prepared.version());
        let stored = registry.acknowledge(prepared, receipt, at).unwrap();
        let connection = registry.publish(stored, at).unwrap();
        Self {
            _root: root,
            path,
            binding,
            connection,
            custody,
        }
    }

    fn refresher(&self, owner: &str, server: &Server) -> Refresher {
        self.trusting(owner, &server.root)
    }

    /// A refresher whose only trusted root is the PEM `root`.
    fn trusting(&self, owner: &str, root: &str) -> Refresher {
        Refresher::fixture(
            Registry::with_system_clock(&self.path),
            owner,
            root.as_bytes(),
            Box::new(self.custody.clone()),
        )
    }

    fn active(&self) -> Active {
        let metadata = Metadata::inspect(&self.path).unwrap();
        metadata
            .connection
            .query_row(
                "SELECT active_generation,active_material,publication_fence FROM registry_connections WHERE connection_ref=?1",
                [&self.connection],
                |row| {
                    Ok(Active {
                        generation: row.get(0)?,
                        material: row.get(1)?,
                        fence: row.get(2)?,
                    })
                },
            )
            .unwrap()
    }

    fn retired(&self, material: &str) -> bool {
        let metadata = Metadata::inspect(&self.path).unwrap();
        metadata
            .connection
            .query_row(
                "SELECT retirement_fence IS NOT NULL FROM registry_materials WHERE version_id=?1",
                [material],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn generations(&self) -> i64 {
        let metadata = Metadata::inspect(&self.path).unwrap();
        metadata
            .connection
            .query_row(
                "SELECT count(*) FROM registry_generations WHERE connection_ref=?1",
                [&self.connection],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn described(&self) -> registry::ObservedConnection {
        Registry::new(&self.path)
            .describe(
                INSTANCE,
                &self.binding.adapter_id,
                &self.binding.configuration_revision,
                &self.connection,
                now(),
                true,
            )
            .unwrap()
    }

    /// The one attempt the ledger holds, and its recorded commands.
    fn attempt(&self) -> (crate::local::metadata::RefreshAttempt, Vec<String>) {
        let mut attempts = self.attempts();
        assert_eq!(attempts.len(), 1, "one refresh attempt");
        attempts.remove(0)
    }

    /// Every attempt the ledger holds, each with its recorded commands.
    fn attempts(&self) -> Vec<(crate::local::metadata::RefreshAttempt, Vec<String>)> {
        let attempts = Metadata::inspect(&self.path)
            .unwrap()
            .refresh_attempts()
            .unwrap();
        let mut recorded = Metadata::recorded_refresh_commands(&self.path).unwrap();
        attempts
            .into_iter()
            .map(|attempt| {
                let commands = recorded.remove(&attempt.id).unwrap();
                (attempt, commands)
            })
            .collect()
    }
}

fn scopes(names: &[&str]) -> Option<BTreeSet<String>> {
    Some(names.iter().map(|name| (*name).to_owned()).collect())
}

fn refreshed_request(server: &Server) {
    let requests = server.requests();
    assert_eq!(requests.len(), 1, "the server saw one request");
    assert_eq!(requests[0]["grant_type"], "refresh_token");
    assert_eq!(requests[0]["refresh_token"], REFRESH);
    assert_eq!(requests[0]["client_id"], CLIENT);
}

#[test]
fn rotating_refresh_publishes_a_new_generation() {
    let server = Server::start(vec![token(Some(ROTATED), Some("read"))]);
    let connected = Connected::new(&server, "rotating", &["read"]);
    let before = connected.active();
    let outcome = connected
        .refresher("owner-one", &server)
        .refresh_connection(&connected.connection)
        .unwrap();
    assert_eq!(outcome, Outcome::Refreshed);
    let after = connected.active();
    assert_ne!(after.generation, before.generation);
    assert_ne!(after.material, before.material);
    assert_ne!(after.fence, before.fence);
    assert!(connected.retired(&before.material), "old version retired");
    assert!(!connected.retired(&after.material));
    refreshed_request(&server);
    let entry = connected.custody.entry(&after.material);
    assert_eq!(entry["access_token"], REFRESHED_ACCESS);
    assert_eq!(entry["refresh_token"], ROTATED);
    assert_eq!(entry["client_id"], CLIENT);
    assert_eq!(entry["scopes"], json!(["read"]));
    let described = connected.described();
    assert_eq!(described.state, State::Ready);
    assert_eq!(described.granted_scopes, scopes(&["read"]));
    let (attempt, commands) = connected.attempt();
    assert_eq!(attempt.state, RefreshState::Published);
    assert_eq!(attempt.source_generation, before.generation);
    assert_eq!(attempt.candidate_generation, Some(after.generation));
    assert_eq!(commands, [RESERVE, AUTHORIZE, STORE, PUBLISH]);
}

#[test]
fn concurrent_callers_share_one_exchange() {
    let server = Server::delayed(
        vec![token(Some(ROTATED), Some("read"))],
        Duration::from_secs(2),
    );
    let connected = Connected::new(&server, "rotating", &["read"]);
    let before = connected.active();
    let refresher = connected.refresher("owner-one", &server);
    let start = Barrier::new(8);
    let outcomes = std::thread::scope(|scope| {
        let callers = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    start.wait();
                    refresher.refresh_connection(&connected.connection)
                })
            })
            .collect::<Vec<_>>();
        callers
            .into_iter()
            .map(|caller| caller.join().unwrap().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(outcomes, [Outcome::Refreshed; 8]);
    refreshed_request(&server);
    assert_eq!(connected.generations(), 2, "one published generation");
    let after = connected.active();
    assert_ne!(after.generation, before.generation);
    let (attempt, commands) = connected.attempt();
    assert_eq!(attempt.candidate_generation, Some(after.generation));
    assert_eq!(commands, [RESERVE, AUTHORIZE, STORE, PUBLISH]);
}

#[test]
fn lost_response_is_uncertain_and_never_resent() {
    let server = Server::start(vec![token(Some(ROTATED), Some("read"))]);
    let connected = Connected::new(&server, "rotating", &["read"]);
    let before = connected.active();
    let killed = connected
        .refresher("owner-one", &server)
        .crashing(Crash::BeforeStore);
    killed.refresh_connection(&connected.connection).unwrap();
    drop(killed);
    assert_eq!(connected.attempt().0.state, RefreshState::Authorized);
    let restarted = connected.refresher("owner-two", &server);
    assert_eq!(restarted.recover().unwrap(), 1);
    let (attempt, commands) = connected.attempt();
    assert_eq!(attempt.state, RefreshState::Uncertain);
    assert_eq!(commands, [RESERVE, AUTHORIZE, QUARANTINE]);
    assert_eq!(connected.described().state, State::Stale);
    assert_eq!(connected.active().generation, before.generation);
    assert_eq!(
        restarted.refresh_connection(&connected.connection).unwrap(),
        Outcome::ReauthorizationRequired
    );
    refreshed_request(&server);
}

#[test]
fn stored_response_is_published_after_restart() {
    let server = Server::start(vec![token(Some(ROTATED), Some("read"))]);
    let connected = Connected::new(&server, "rotating", &["read"]);
    let before = connected.active();
    let killed = connected
        .refresher("owner-one", &server)
        .crashing(Crash::AfterStore);
    killed.refresh_connection(&connected.connection).unwrap();
    drop(killed);
    let (stored, _) = connected.attempt();
    assert_eq!(stored.state, RefreshState::ResponseStored);
    assert_eq!(connected.active(), before, "nothing published yet");
    let restarted = connected.refresher("owner-two", &server);
    assert_eq!(restarted.recover().unwrap(), 1);
    let after = connected.active();
    assert_eq!(
        Some(&after.generation),
        stored.candidate_generation.as_ref()
    );
    assert_ne!(after.material, before.material);
    assert!(connected.retired(&before.material), "old version retired");
    assert_eq!(connected.described().state, State::Ready);
    assert_eq!(
        connected.custody.entry(&after.material)["refresh_token"],
        ROTATED
    );
    let (attempt, commands) = connected.attempt();
    assert_eq!(attempt.state, RefreshState::Published);
    assert_eq!(commands, [RESERVE, AUTHORIZE, STORE, RECOVER, PUBLISH]);
    refreshed_request(&server);
}

#[test]
fn invalid_grant_marks_the_connection_stale() {
    let server = Server::start(vec![error(400, "invalid_grant")]);
    let connected = Connected::new(&server, "rotating", &["read"]);
    let before = connected.active();
    let outcome = connected
        .refresher("owner-one", &server)
        .refresh_connection(&connected.connection)
        .unwrap();
    assert_eq!(outcome, Outcome::ReauthorizationRequired);
    assert_eq!(connected.described().state, State::Stale);
    assert_eq!(connected.active().generation, before.generation);
    assert_eq!(connected.active().material, before.material);
    let (attempt, commands) = connected.attempt();
    assert_eq!(attempt.state, RefreshState::Uncertain);
    assert_eq!(commands, [RESERVE, AUTHORIZE, QUARANTINE]);
    refreshed_request(&server);
}

#[test]
fn rate_limited_changes_nothing() {
    let mut limited = error(429, "slow_down");
    limited.retry_after = Some("30");
    let server = Server::start(vec![limited, token(Some(ROTATED), Some("read"))]);
    let connected = Connected::new(&server, "rotating", &["read"]);
    let before = connected.active();
    let described = connected.described();
    let outcome = connected
        .refresher("owner-one", &server)
        .refresh_connection(&connected.connection)
        .unwrap();
    assert_eq!(
        outcome,
        Outcome::RateLimited {
            retry_after_ms: Some(30_000)
        }
    );
    assert_eq!(connected.active(), before, "the connection is unchanged");
    assert!(!connected.retired(&before.material));
    let after = connected.described();
    assert_eq!(after.state, State::Ready);
    assert_eq!(after.revision, described.revision);
    assert_eq!(after.granted_scopes, described.granted_scopes);
    assert_eq!(
        after.credential_expires_at_ms,
        described.credential_expires_at_ms
    );
    assert_eq!(
        connected.custody.entry(&before.material)["refresh_token"],
        REFRESH
    );
    refreshed_request(&server);

    // The 429 released the source: the next refresh sends one request and
    // publishes, as if the limited exchange had never been authorized.
    let outcome = connected
        .refresher("owner-one", &server)
        .refresh_connection(&connected.connection)
        .unwrap();
    assert_eq!(outcome, Outcome::Refreshed);
    let requests = server.requests();
    assert_eq!(requests.len(), 2, "the second refresh sent one request");
    assert_eq!(requests[1]["grant_type"], "refresh_token");
    assert_eq!(requests[1]["refresh_token"], REFRESH);
    let after = connected.active();
    assert_ne!(after.generation, before.generation);
    assert!(connected.retired(&before.material));
    assert_eq!(
        connected.custody.entry(&after.material)["refresh_token"],
        ROTATED
    );
    assert_eq!(connected.described().state, State::Ready);
    let mut attempts = connected.attempts();
    attempts.sort_by_key(|(attempt, _)| attempt.state != RefreshState::Released);
    let [
        (released, released_commands),
        (published, published_commands),
    ] = <[_; 2]>::try_from(attempts).unwrap();
    assert_eq!(released.state, RefreshState::Released);
    assert_eq!(released.source_generation, before.generation);
    assert_eq!(released.candidate_generation, None);
    assert_eq!(released_commands, [RESERVE, AUTHORIZE, RELEASE]);
    assert_eq!(published.state, RefreshState::Published);
    assert_eq!(published.source_generation, before.generation);
    assert_eq!(published_commands, [RESERVE, AUTHORIZE, STORE, PUBLISH]);
}

#[test]
fn refresh_cannot_widen_scopes() {
    let server = Server::start(vec![token(Some(ROTATED), Some("read write"))]);
    let connected = Connected::new(&server, "rotating", &["read"]);
    let before = connected.active();
    let outcome = connected
        .refresher("owner-one", &server)
        .refresh_connection(&connected.connection)
        .unwrap();
    assert_eq!(outcome, Outcome::InsufficientScope);
    let after = connected.active();
    assert_eq!(after.generation, before.generation);
    assert_eq!(after.material, before.material);
    let described = connected.described();
    assert_eq!(described.granted_scopes, scopes(&["read"]));
    assert_eq!(described.state, State::Stale);
    assert_eq!(connected.generations(), 1, "no candidate generation");
    let (_, commands) = connected.attempt();
    assert_eq!(commands, [RESERVE, AUTHORIZE, QUARANTINE]);
    refreshed_request(&server);
}

#[test]
fn refresh_without_scope_keeps_granted_scopes() {
    let server = Server::start(vec![token(Some(ROTATED), None)]);
    let connected = Connected::new(&server, "rotating", &["profile", "read"]);
    let outcome = connected
        .refresher("owner-one", &server)
        .refresh_connection(&connected.connection)
        .unwrap();
    assert_eq!(outcome, Outcome::Refreshed);
    let after = connected.active();
    assert_eq!(
        connected.described().granted_scopes,
        scopes(&["profile", "read"])
    );
    assert_eq!(
        connected.custody.entry(&after.material)["scopes"],
        json!(["profile", "read"])
    );
    let (_, commands) = connected.attempt();
    assert_eq!(commands, [RESERVE, AUTHORIZE, STORE, PUBLISH]);
    refreshed_request(&server);
}

#[test]
fn non_rotating_refresh_keeps_the_stored_refresh_token() {
    let server = Server::start(vec![token(None, Some("read"))]);
    let connected = Connected::new(&server, "non_rotating", &["read"]);
    let before = connected.active();
    let outcome = connected
        .refresher("owner-one", &server)
        .refresh_connection(&connected.connection)
        .unwrap();
    assert_eq!(outcome, Outcome::Refreshed);
    let after = connected.active();
    assert_ne!(after.generation, before.generation);
    let entry = connected.custody.entry(&after.material);
    assert_eq!(entry["refresh_token"], REFRESH);
    assert_eq!(entry["access_token"], REFRESHED_ACCESS);
    assert!(connected.retired(&before.material));
    let (_, commands) = connected.attempt();
    assert_eq!(commands, [RESERVE, AUTHORIZE, STORE, PUBLISH]);
    refreshed_request(&server);
}

#[test]
fn non_rotating_response_with_a_new_refresh_token_is_uncertain() {
    let server = Server::start(vec![token(Some(ROTATED), Some("read"))]);
    let connected = Connected::new(&server, "non_rotating", &["read"]);
    let before = connected.active();
    let outcome = connected
        .refresher("owner-one", &server)
        .refresh_connection(&connected.connection)
        .unwrap();
    assert_eq!(outcome, Outcome::Uncertain);
    assert_eq!(connected.active().generation, before.generation);
    assert_eq!(connected.active().material, before.material);
    assert_eq!(connected.described().state, State::Stale);
    assert_eq!(connected.generations(), 1, "nothing of the response stored");
    let (attempt, commands) = connected.attempt();
    assert_eq!(attempt.state, RefreshState::Uncertain);
    assert_eq!(commands, [RESERVE, AUTHORIZE, QUARANTINE]);
    refreshed_request(&server);
}

// ------------------------------------------------------------ adversary U4-1

/// Waits, bounded, until the fake server has read `count` requests.
fn await_requests(server: &Server, count: usize) {
    let until = Instant::now() + Duration::from_secs(10);
    while server.requests().len() < count {
        assert!(
            Instant::now() < until,
            "the server never saw request {count}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// §4.1: "At exchange authorization the old generation's refresh authority
/// is consumed; old-generation dispatch is withheld atomically with that
/// authorization while refresh is unresolved." A read's evidence
/// recollection (`Registry::capture_recollection`, run by the supervisor for
/// any connection whose evidence lapsed or nears its end) that lands while
/// the one exchange is on the wire must not cost the connection its rotated
/// refresh token: either the recollection is withheld, or the refresh still
/// publishes. Here it is neither: the recollection advances the private
/// publication fence, `prepare_refresh` refuses the response as a conflict,
/// the rotated token is dropped and the connection goes Stale.
#[test]
fn a_read_recollection_during_the_exchange_does_not_burn_the_rotated_token() {
    let server = Server::delayed(
        vec![token(Some(ROTATED), Some("read"))],
        Duration::from_secs(3),
    );
    let connected = Connected::new(&server, "rotating", &["read"]);
    let before = connected.active();
    let refresher = connected.refresher("owner-one", &server);
    let (outcome, recollection) = std::thread::scope(|scope| {
        let refresh = scope.spawn(|| refresher.refresh_connection(&connected.connection));
        // The exchange is authorized and on the wire.
        await_requests(&server, 1);
        let registry = Registry::with_system_clock(&connected.path);
        let recollection = (|| -> registry::Result<()> {
            let at = now();
            let captured = registry.capture_recollection(
                &connected.binding,
                &connected.connection,
                at,
                at + 20_000,
            )?;
            let dispatched = registry.dispatch_revalidation(captured, now())?;
            let collected = now();
            registry.finish_revalidation(
                dispatched,
                Ok(ValidatedBaseline {
                    identity: ExternalIdentity {
                        kind: "fixture-account".into(),
                        subject: "one".into(),
                    },
                    granted_scopes: scopes(&["read"]),
                    credential_expires_at_ms: None,
                    collected_at_ms: collected,
                    valid_until_ms: collected + 30_000,
                    binding: None,
                }),
                now(),
            )
        })();
        (refresh.join().unwrap().unwrap(), recollection)
    });
    let described = connected.described();
    assert_eq!(
        (outcome, described.state),
        (Outcome::Refreshed, State::Ready),
        "the one exchange was sent and answered with a rotated token, yet the \
         refresh did not publish it (recollection during the exchange: {:?}); \
         the source token is consumed and the connection needs reauthorization",
        recollection.err()
    );
    let after = connected.active();
    assert_ne!(after.generation, before.generation);
    assert_eq!(
        connected.custody.entry(&after.material)["refresh_token"],
        ROTATED
    );
    refreshed_request(&server);
}

/// [`Connected::new`] under a profile whose evidence lasts `lifetime_ms`
/// (60 s is the default every shipped adapter digests, CHANGELOG.md:2053).
fn connected_for(server: &Server, lifetime_ms: u64) -> Connected {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    crate::local::filesystem::directory(&path, true, true).unwrap();
    drop(Metadata::initialize(&path).unwrap());
    let mut binding = registry::fixture_binding(INSTANCE);
    binding.profile.evidence_lifetime_ms = lifetime_ms;
    binding.profile.oauth2 = Some(
        serde_json::from_value(json!({
            "authorize_url": "https://localhost/fixture/authorize",
            "token_url": server.url(),
            "device_authorization_url": null,
            "client_id": CLIENT,
            "client_kind": "public",
            "request_encoding": "form",
            "refresh_encoding": "form",
            "redirect_kind": "loopback_any_port",
            "redirect_uri": null,
            "authorize_parameters": [],
            "requested_scopes": ["read"],
            "required_scopes": ["read"],
            "rotation": "rotating",
            "refresh_lead_seconds": 60,
            "grant_types": ["authorization_code"],
            "sources": ["https://localhost/fixture-authorization-server"],
            "state_in_exchange": false,
        }))
        .unwrap(),
    );
    let registry = Registry::new(&path);
    let at = now();
    let entry = serde_json::to_vec(&json!({
        "client_id": CLIENT,
        "access_token": ACCESS,
        "refresh_token": REFRESH,
        "expires_at_ms": at + 3_600_000,
        "scopes": ["read"],
    }))
    .unwrap();
    let acquisition = registry.begin(&binding, at).unwrap();
    let claim = registry.consume(acquisition, at).unwrap();
    let baseline = ValidatedBaseline {
        identity: ExternalIdentity {
            kind: "fixture-account".into(),
            subject: "one".into(),
        },
        granted_scopes: scopes(&["read"]),
        credential_expires_at_ms: Some(at + 3_600_000),
        collected_at_ms: at,
        valid_until_ms: at + lifetime_ms,
        binding: None,
    };
    let prepared = registry.prepare(&claim, baseline, entry.len(), at).unwrap();
    let custody = Arc::new(Memory::default());
    custody
        .versions
        .lock()
        .unwrap()
        .insert(prepared.version().id(), entry);
    let receipt = custody::WrittenVersion::fixture(prepared.version());
    let stored = registry.acknowledge(prepared, receipt, at).unwrap();
    let connection = registry.publish(stored, at).unwrap();
    Connected {
        _root: root,
        path,
        binding,
        connection,
        custody,
    }
}

/// Acceptance, `stored_response_is_published_after_restart`: "killed after
/// `StoreResponse`; after restart the candidate is published and no request
/// is sent", and §4.1 "Recover committed response ... Recover exactly the
/// recorded candidate, rechecking custody and evidence". The stored response
/// is durable and its access token valid for an hour, but recovery dates its
/// evidence from the store acknowledgement (`Refresher::resolve`), so an
/// owner that restarts later than the profile's evidence lifetime after the
/// crash (60 s by default; 10 s here) refuses its own durable candidate,
/// discards the rotated refresh token and marks the connection Stale.
#[test]
fn a_stored_response_is_published_by_an_owner_restarting_after_the_evidence_lifetime() {
    const LIFETIME_MS: u64 = 10_000;
    let server = Server::start(vec![token(Some(ROTATED), Some("read"))]);
    let connected = connected_for(&server, LIFETIME_MS);
    let before = connected.active();
    let killed = connected
        .refresher("owner-one", &server)
        .crashing(Crash::AfterStore);
    killed.refresh_connection(&connected.connection).unwrap();
    drop(killed);
    let (stored, _) = connected.attempt();
    assert_eq!(stored.state, RefreshState::ResponseStored);
    // The owner comes back after the evidence lifetime, well inside the
    // stored access token's hour.
    std::thread::sleep(Duration::from_millis(LIFETIME_MS + 1_000));
    let restarted = connected.refresher("owner-two", &server);
    assert_eq!(restarted.recover().unwrap(), 1);
    let (attempt, commands) = connected.attempt();
    let state = connected.described().state;
    assert_eq!(
        (attempt.state, state),
        (RefreshState::Published, State::Ready),
        "recovery refused the durable stored response ({commands:?}); the \
         rotated refresh token is discarded and the connection needs \
         reauthorization although no exchange was lost"
    );
    let after = connected.active();
    assert_eq!(
        Some(&after.generation),
        stored.candidate_generation.as_ref()
    );
    assert_ne!(after.material, before.material);
    refreshed_request(&server);
}

// ------------------------------------------------------------ fix round 1

/// A root no fixture server presents.
fn unrelated_root() -> String {
    rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
        .unwrap()
        .cert
        .pem()
}

/// The one attempt was released and the connection is as it was: Ready,
/// the same generation, version and fence, nothing retired.
fn released_unchanged(connected: &Connected, before: &Active) {
    assert_eq!(connected.active(), *before, "the connection is unchanged");
    assert!(!connected.retired(&before.material));
    assert_eq!(connected.described().state, State::Ready);
    let (attempt, commands) = connected.attempt();
    assert_eq!(attempt.state, RefreshState::Released);
    assert_eq!(attempt.candidate_generation, None);
    assert_eq!(commands, [RESERVE, AUTHORIZE, RELEASE]);
}

/// A token endpoint nothing listens on refuses the connection before any
/// byte of the request leaves: the exchange is released as a 429's is.
#[test]
fn a_refused_connection_releases_the_exchange() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let connected = Connected::at(
        &format!("https://localhost:{port}/token"),
        "rotating",
        &["read"],
    );
    let before = connected.active();
    let outcome = connected
        .trusting("owner-one", &unrelated_root())
        .refresh_connection(&connected.connection)
        .unwrap();
    assert_eq!(outcome, Outcome::Unavailable);
    released_unchanged(&connected, &before);
}

/// A server whose certificate the owner does not trust fails the TLS
/// handshake before any byte of the request leaves: the exchange is
/// released, and the next refresh under the right root sends the one
/// exchange and publishes.
#[test]
fn an_untrusted_token_server_releases_the_exchange() {
    let server = Server::start(vec![token(Some(ROTATED), Some("read"))]);
    let connected = Connected::new(&server, "rotating", &["read"]);
    let before = connected.active();
    let outcome = connected
        .trusting("owner-one", &unrelated_root())
        .refresh_connection(&connected.connection)
        .unwrap();
    assert_eq!(outcome, Outcome::Unavailable);
    assert!(
        server.requests().is_empty(),
        "no request reached the server"
    );
    released_unchanged(&connected, &before);

    let outcome = connected
        .refresher("owner-one", &server)
        .refresh_connection(&connected.connection)
        .unwrap();
    assert_eq!(outcome, Outcome::Refreshed);
    refreshed_request(&server);
    let after = connected.active();
    assert_ne!(after.generation, before.generation);
    assert_eq!(
        connected.custody.entry(&after.material)["refresh_token"],
        ROTATED
    );
}

/// A connection the server closes after it read the request may have
/// consumed the refresh token: uncertain, quarantined, the connection Stale.
#[test]
fn a_connection_dropped_after_the_request_is_uncertain() {
    let server = Server::start(vec![dropped()]);
    let connected = Connected::new(&server, "rotating", &["read"]);
    let before = connected.active();
    let outcome = connected
        .refresher("owner-one", &server)
        .refresh_connection(&connected.connection)
        .unwrap();
    assert_eq!(outcome, Outcome::Uncertain);
    assert_eq!(connected.described().state, State::Stale);
    assert_eq!(connected.active().generation, before.generation);
    let (attempt, commands) = connected.attempt();
    assert_eq!(attempt.state, RefreshState::Uncertain);
    assert_eq!(commands, [RESERVE, AUTHORIZE, QUARANTINE]);
    refreshed_request(&server);
}

/// At recovery the stored response's access token decides: one that
/// expired while no owner ran is discarded and the connection goes Stale.
/// Nothing is sent.
#[test]
fn an_expired_stored_response_is_discarded_at_recovery() {
    let mut short = token(Some(ROTATED), Some("read"));
    short.body["expires_in"] = json!(2);
    let server = Server::start(vec![short]);
    let connected = Connected::new(&server, "rotating", &["read"]);
    let before = connected.active();
    let killed = connected
        .refresher("owner-one", &server)
        .crashing(Crash::AfterStore);
    killed.refresh_connection(&connected.connection).unwrap();
    drop(killed);
    assert_eq!(connected.attempt().0.state, RefreshState::ResponseStored);
    std::thread::sleep(Duration::from_millis(2_500));
    let restarted = connected.refresher("owner-two", &server);
    assert_eq!(restarted.recover().unwrap(), 1);
    let (attempt, commands) = connected.attempt();
    assert_eq!(attempt.state, RefreshState::Discarded);
    assert_eq!(commands, [RESERVE, AUTHORIZE, STORE, RECOVER, DISCARD]);
    assert_eq!(connected.described().state, State::Stale);
    assert_eq!(connected.active().generation, before.generation);
    refreshed_request(&server);
}

/// A connection whose custody holds as many versions as it may retain
/// refuses the refresh before reserving it: no request, nothing recorded,
/// the connection unchanged.
#[test]
fn a_full_version_history_refuses_before_any_request() {
    registry::RETAINED_BOUND.with(|bound| bound.set(2));
    let server = Server::start(vec![
        token(Some(ROTATED), Some("read")),
        token(Some("fixture-refresh-token-three"), Some("read")),
    ]);
    let connected = Connected::new(&server, "rotating", &["read"]);
    let refresher = connected.refresher("owner-one", &server);
    assert_eq!(
        refresher.refresh_connection(&connected.connection).unwrap(),
        Outcome::Refreshed
    );
    // The active version and the retired source, not yet deleted.
    let full = connected.active();
    assert_eq!(
        refresher.refresh_connection(&connected.connection).unwrap(),
        Outcome::CustodyUnavailable
    );
    assert_eq!(connected.active(), full, "the connection is unchanged");
    assert_eq!(connected.described().state, State::Ready);
    refreshed_request(&server);
    let (attempt, commands) = connected.attempt();
    assert_eq!(attempt.state, RefreshState::Published);
    assert_eq!(commands, [RESERVE, AUTHORIZE, STORE, PUBLISH]);
}

mod issue;
#[cfg(target_os = "linux")]
mod launch;
