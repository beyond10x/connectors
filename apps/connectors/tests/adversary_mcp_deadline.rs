//! Adversary cases for story:mcp-tool-deadline-keeps-the-server (CR-001), on
//! the harness of `adversary_local_mcp_server.rs` (copied, trimmed: no slow
//! provider answers, no padded descriptions).
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
    os::fd::AsRawFd,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{OnceLock, mpsc},
    time::{Duration, Instant},
};

const ADAPTER_FIXTURE: &str = "adversary_mcp_deadline_adapter_fixture";
const PROVIDER: &str = "/work/provider";
const COMPANION: &str = "/srv/connectors/config.toml.mcp-stdio.json";
const CURATION: &str = "/srv/connectors/config.toml.operation-curation.json";
const INSTANCE: &str = "forge-local";
const ADAPTER: &str = "fixture";
const DESCRIPTION: &str = "desc-1";
const OPERATIONS: [&str; 1] = ["read"];
/// The request budget every case here configures: short, so a held store
/// reaches it quickly.
const BUDGET_MS: u64 = 2000;
/// Pings sent at once: their replies are far more than the 4096-byte pipe.
const PINGS: i64 = 200;

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

#[test]
fn adversary_mcp_deadline_adapter_fixture() {
    if std::env::args().any(|arg| arg == "--connectors-private-fd") {
        let _ = runtime::serve(3, Reading);
    }
}

/// A loopback provider answering `GET /observe/<n>` with `{"observed":<n>}`.
fn provider() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
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
    address
}

fn private(path: &Path, text: String) {
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn clock_key(seed: u8) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode([seed; 32])
}

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
            operations: OPERATIONS.iter().map(|op| (*op).to_owned()).collect(),
            custody: Default::default(),
        },
    }
}

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

/// One container with a connection, the curation pinned, and the companion
/// written with `request_milliseconds` [`BUDGET_MS`].
struct Served {
    container: Container,
}

impl Served {
    fn new() -> Self {
        let address = provider();
        let container = Container::new(ADAPTER_FIXTURE);
        fs::write(container.path(PROVIDER), &address).unwrap();
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
                    "approval":"not_required","realization":"implemented",
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
        let exposures: Vec<Value> = OPERATIONS
            .iter()
            .map(|operation| {
                json!({"adapter_alias":ALIAS,"operation_ref":operation,"connection_ref":connection,
                    "families":["tools"],"enabled":true})
            })
            .collect();
        private(
            &container.path(COMPANION),
            json!({"format":"connectors-mcp-local/1","limits":{"frame_octets":1048576,"response_octets":33554432,
                "concurrent_requests":1,"request_milliseconds":BUDGET_MS},"exposures":exposures})
            .to_string(),
        );
        Self { container }
    }

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

/// The primary revision's per-request `_meta`.
fn meta() -> Value {
    json!({"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}})
}

fn request(id: i64, method: &str) -> String {
    format!(
        "{}\n",
        json!({"jsonrpc":"2.0","id":id,"method":method,"params":{"_meta":meta()}})
    )
}

/// The state store's lifecycle lock, held until the file is dropped.
fn hold_store(served: &Served) -> fs::File {
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

/// A server whose stdout pipe is shrunk to 4096 bytes and not read until
/// [`Slow::drain`].
struct Slow {
    child: Child,
    stdin: ChildStdin,
    stdout: Option<ChildStdout>,
    stderr: Option<std::thread::JoinHandle<String>>,
    lines: Option<mpsc::Receiver<String>>,
}

impl Slow {
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
        // SAFETY: F_SETPIPE_SZ on this test's own pipe read end.
        let size = unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_SETPIPE_SZ, 4096) };
        assert_eq!(size, 4096, "could not shrink the stdout pipe");
        let mut stderr = child.stderr.take().unwrap();
        let stderr = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        });
        let stdin = child.stdin.take().unwrap();
        let mut slow = Self {
            child,
            stdin,
            stdout: Some(stdout),
            stderr: Some(stderr),
            lines: None,
        };
        slow.ready();
        slow
    }
    /// Wait until the server answers a ping, so that every later request's
    /// budget starts when the test sends it, not when a slow start ends.
    fn ready(&mut self) {
        self.send(&request(1, "ping"));
        let stdout = self.stdout.as_mut().unwrap();
        let mut line = Vec::new();
        let mut byte = [0u8; 1];
        while byte[0] != b'\n' {
            assert_eq!(stdout.read(&mut byte).unwrap(), 1, "the server ended");
            line.push(byte[0]);
        }
        let reply: Value = serde_json::from_slice(&line).unwrap();
        assert!(reply["id"] == 1 && reply.get("result").is_some(), "{reply}");
    }
    fn send(&mut self, text: &str) {
        self.stdin.write_all(text.as_bytes()).unwrap();
    }
    /// Begin reading stdout, promptly from now on.
    fn drain(&mut self) {
        let stdout = self.stdout.take().unwrap();
        let mut queued: libc::c_int = 0;
        // SAFETY: FIONREAD into a live c_int on this test's own pipe.
        unsafe { libc::ioctl(stdout.as_raw_fd(), libc::FIONREAD, &mut queued) };
        eprintln!(
            "drain: {queued} bytes in the pipe, server ended {:?}",
            self.child.try_wait().unwrap()
        );
        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if sender.send(line).is_err() {
                    return;
                }
            }
        });
        self.lines = Some(lines);
    }
    /// Every frame read until `wanted` says it has all it needs, or 10 s.
    fn collect(&self, mut wanted: impl FnMut(&[Value]) -> bool) -> Vec<Value> {
        let lines = self.lines.as_ref().unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        let mut frames = Vec::new();
        while !wanted(&frames) {
            match lines.recv_timeout(until.saturating_duration_since(Instant::now())) {
                Ok(line) => frames.push(
                    serde_json::from_str(&line)
                        .unwrap_or_else(|e| panic!("stdout is not a frame ({e}): {line}")),
                ),
                Err(_) => break,
            }
        }
        frames
    }
    /// The exit status if the server has ended, and its stderr then.
    fn ended(&mut self) -> Option<(std::process::ExitStatus, String)> {
        let status = self.child.try_wait().unwrap()?;
        Some((status, self.stderr.take().unwrap().join().unwrap()))
    }
}

impl Drop for Slow {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn answered(frames: &[Value], id: i64) -> Option<&Value> {
    frames.iter().find(|f| f["id"] == id)
}

/// docs/local-mcp-stdio.md (this unit): "A request's timeout does not end the
/// session." The client sends 200 pings (about 12 KB of replies through a
/// 4096-byte pipe) and begins reading 600 ms after their 2 s request budget.
/// The doc as written promises every ping a reply and a live session; the
/// serve loop's write bound (stdio.rs:179-181) ends the process instead.
#[test]
fn adversary_a_reply_undrained_past_its_deadline_does_not_end_the_session() {
    let served = Served::new();
    let mut server = Slow::start(&served);
    let pings: String = (100..100 + PINGS).map(|id| request(id, "ping")).collect();
    server.send(&pings);
    std::thread::sleep(Duration::from_millis(BUDGET_MS + 600));
    server.drain();
    let frames = server.collect(|frames| frames.len() as i64 >= PINGS);
    let replied = frames.iter().filter(|f| f.get("result").is_some()).count();
    if let Some((status, stderr)) = server.ended() {
        panic!(
            "a request's deadline passed while its reply waited in the pipe and the session \
             ended: {status}, stderr {stderr:?}, {replied} of {PINGS} pings answered"
        );
    }
    assert_eq!(replied as i64, PINGS, "{frames:?}");
}

/// Mutant guard for `LATE_WRITE_GRACE` (stdio.rs:63): the late `timeout`
/// refusal is queued behind ping replies that fill the 4096-byte pipe, and the
/// client begins reading 600 ms after the list's deadline. The refusal must
/// still be written and the session must continue; a grace shorter than that
/// (or none) ends the session here, which the unit's own case cannot see
/// because its refusal is written in the same loop turn it is queued in.
#[test]
fn adversary_a_late_timeout_queued_behind_a_full_pipe_is_still_written() {
    let served = Served::new();
    let mut server = Slow::start(&served);
    let lock = hold_store(&served);
    let sent = Instant::now();
    server.send(&request(2, "tools/list"));
    std::thread::sleep(Duration::from_millis(BUDGET_MS - 600));
    let pings: String = (100..100 + PINGS).map(|id| request(id, "ping")).collect();
    server.send(&pings);
    std::thread::sleep((Duration::from_millis(BUDGET_MS + 600)).saturating_sub(sent.elapsed()));
    server.drain();
    let frames = server.collect(|frames| answered(frames, 2).is_some());
    if let Some((status, stderr)) = server.ended() {
        panic!(
            "the session ended with the late timeout queued behind a full pipe: {status}, \
             stderr {stderr:?}, frames {}",
            frames.len()
        );
    }
    let list = answered(&frames, 2).unwrap_or_else(|| panic!("no reply to the held list"));
    assert_eq!(list["error"]["data"]["error"]["code"], "timeout", "{list}");
    drop(lock);
    server.send(&request(3, "ping"));
    let frames = server.collect(|frames| answered(frames, 3).is_some());
    assert!(
        answered(&frames, 3).is_some_and(|f| f.get("result").is_some()),
        "no ping answered after the late timeout"
    );
}
