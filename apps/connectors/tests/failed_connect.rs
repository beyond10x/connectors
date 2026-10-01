//! story:failed-connect-reports-its-cause. An owner that answers a connect or a
//! revalidation with its own definite failure has recorded that outcome, so the
//! caller receives that code, never `outcome_unknown`. Only a reply that never
//! arrived leaves the outcome unknown (scenario "C03 publication reply lost").
//!
//! The first four cases drive the production owner client against a stand-in
//! owner socket and run in the ordinary lane. Two CLI cases need a disposable
//! Secret Service (`dbus-daemon`, `gnome-keyring-daemon`) and are ignored by
//! default: the observed defect end to end, the production CLI, owner and
//! catalog provider child against an unreachable provider (it also needs a
//! built catalog provider named by `CONNECTORS_TEST_CATALOG_PROVIDER`); and a
//! revalidation of a published connection that a stand-in owner definitely
//! fails.
use connectors_host::local::{
    config::{Config, Paths},
    owner::{self, Client, Code},
    registry, runtime,
};
#[path = "support/socket.rs"]
mod sockets;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

const ACQUISITION: &str = "acquisition-fixture-1";

fn frame(control: &Value) -> Vec<u8> {
    let bytes = serde_json::to_vec(control).unwrap();
    let mut frame = Vec::new();
    frame.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    frame.extend_from_slice(&0u32.to_be_bytes());
    frame.extend_from_slice(&0u32.to_be_bytes());
    frame.extend_from_slice(&bytes);
    frame
}

fn read_control(stream: &mut UnixStream) -> Value {
    let mut sizes = [0u8; 12];
    stream.read_exact(&mut sizes).unwrap();
    let size = |at: usize| u32::from_be_bytes(sizes[at..at + 4].try_into().unwrap()) as usize;
    let mut control = vec![0; size(0)];
    stream.read_exact(&mut control).unwrap();
    let mut rest = vec![0; size(4) + size(8)];
    stream.read_exact(&mut rest).unwrap();
    serde_json::from_slice(&control).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The digest the client sends: this very test process's executable image.
fn own_build() -> String {
    let image = fs::read("/proc/self/exe").unwrap();
    hex(ring::digest::digest(&ring::digest::SHA256, &image).as_ref())
}

fn profile() -> runtime::Profile {
    runtime::Profile {
        id: "fixture.basic".into(),
        revision: "profile-1".into(),
        purpose: registry::Purpose::DelegatedUser,
        subject: registry::Subject::User,
        scheme: "http_basic".into(),
        capability: "http-basic".into(),
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

/// What the stand-in owner does with the one work request after the greeting.
#[derive(Clone, Copy)]
enum Answer {
    /// The owner's own definite `Failed` reply carrying this code.
    Failed(Code),
    /// The stream closes after the request arrived, without any reply.
    Lost,
}

/// An initialized state directory with a stand-in owner listening on its
/// socket. The stand-in greets the client as a same-build owner, then answers
/// `Begin` with a capture (connect only) and the final request with `answer`.
fn stand_in(
    answer: Answer,
) -> (
    tempfile::TempDir,
    Paths,
    std::thread::JoinHandle<Vec<String>>,
) {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths::resolve(
        Some(&root.path().join("config/config.toml")),
        Some(&root.path().join("state")),
    )
    .unwrap();
    Config::initialize(&paths).unwrap();
    let owner = serve(&paths, own_build(), answer);
    (root, paths, owner)
}

/// Listens on `paths`' owner socket as an owner whose executable digest is
/// `build`, for exactly one client connection.
fn serve(paths: &Paths, build: String, answer: Answer) -> std::thread::JoinHandle<Vec<String>> {
    let socket = paths.state.join("owner.sock");
    let listener = sockets::bind(&paths.state, "owner.sock");
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let hello = read_control(&mut stream);
        assert_eq!(hello["kind"], "hello", "{hello}");
        stream
            .write_all(&frame(&json!({
                "kind": "hello",
                "version": hello["version"],
                "challenge": hello["challenge"],
                "host_incarnation": uuid::Uuid::new_v4().to_string(),
                "authority": hello["authority"],
                "build": build,
            })))
            .unwrap();
        let mut seen = Vec::new();
        loop {
            let request = read_control(&mut stream);
            let kind = request["kind"].as_str().unwrap().to_owned();
            seen.push(kind.clone());
            if kind == "begin" {
                stream
                    .write_all(&frame(&json!({
                        "kind": "capture",
                        "acquisition": ACQUISITION,
                        "expires_at_ms": connectors_sdk::now_ms() + 60_000,
                        "profile": profile(),
                    })))
                    .unwrap();
                continue;
            }
            if let Answer::Failed(code) = answer {
                let mut error = owner::Error::from(code);
                if kind == "complete" {
                    error.acquisition = Some(ACQUISITION.into());
                }
                stream
                    .write_all(&frame(&json!({"kind": "failed", "error": error})))
                    .unwrap();
            }
            return seen;
        }
    })
}

fn connect(answer: Answer) -> owner::Error {
    let (_root, paths, owner) = stand_in(answer);
    let capture = Client::connect(&paths, false)
        .unwrap()
        .begin("forge", Some("fixture.basic".into()), None, None)
        .unwrap();
    let error = capture
        .complete(&connectors_sdk::Secret(br#"{"token":"fixture"}"#.to_vec()))
        .unwrap_err();
    assert_eq!(owner.join().unwrap(), ["begin", "complete"]);
    error
}

fn revalidate(answer: Answer) -> owner::Error {
    let (_root, paths, owner) = stand_in(answer);
    let error = Client::connect(&paths, false)
        .unwrap()
        .revalidate(
            "forge",
            "connection-1",
            "revision-1",
            connectors_sdk::now_ms() + 10_000,
        )
        .unwrap_err();
    assert_eq!(owner.join().unwrap(), ["revalidate"]);
    error
}

#[test]
fn an_owners_definite_connect_failure_keeps_its_code_and_acquisition() {
    for code in [Code::Unavailable, Code::Timeout] {
        let error = connect(Answer::Failed(code));
        assert_eq!(error.code, code, "the owner answered {code:?}");
        assert_eq!(error.acquisition.as_deref(), Some(ACQUISITION));
    }
}

#[test]
fn a_connect_reply_that_never_arrives_is_outcome_unknown() {
    let error = connect(Answer::Lost);
    assert_eq!(error.code, Code::OutcomeUnknown);
    assert_eq!(error.acquisition.as_deref(), Some(ACQUISITION));
}

#[test]
fn an_owners_definite_revalidation_failure_keeps_its_code() {
    for code in [Code::Unavailable, Code::Timeout] {
        let error = revalidate(Answer::Failed(code));
        assert_eq!(error.code, code, "the owner answered {code:?}");
    }
}

#[test]
fn a_revalidation_reply_that_never_arrives_is_outcome_unknown() {
    assert_eq!(revalidate(Answer::Lost).code, Code::OutcomeUnknown);
}

/// A child process killed and reaped when dropped.
struct OwnedProcess(std::process::Child);

impl Drop for OwnedProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A disposable session bus and unlocked GNOME Keyring under `parent`.
struct Custody {
    _daemon: OwnedProcess,
    _bus: OwnedProcess,
    socket: PathBuf,
}

impl Custody {
    fn new(parent: &Path) -> Self {
        use connectors_host::local::{filesystem, keyring};
        let directory = parent.join("custody");
        for path in [
            directory.clone(),
            directory.join("home"),
            directory.join("data/keyrings"),
            directory.join("runtime"),
        ] {
            filesystem::directory(&path, true, true).unwrap();
        }
        let socket = directory.join("bus");
        let bus = OwnedProcess(
            Command::new("/usr/bin/dbus-daemon")
                .args(["--session", "--nofork", "--nopidfile"])
                .arg(format!("--address=unix:path={}", socket.display()))
                .env_clear()
                .env("PATH", "/usr/bin")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let until = Instant::now() + Duration::from_secs(5);
        while UnixStream::connect(&socket).is_err() {
            assert!(Instant::now() < until, "fixture bus did not start");
            std::thread::sleep(Duration::from_millis(20));
        }
        let mut daemon = OwnedProcess(
            Command::new("/usr/bin/gnome-keyring-daemon")
                .args([
                    "--foreground",
                    "--components=secrets",
                    "--unlock",
                    "--control-directory",
                ])
                .arg(directory.join("runtime"))
                .env_clear()
                .env("PATH", "/usr/bin")
                .env("HOME", directory.join("home"))
                .env("XDG_DATA_HOME", directory.join("data"))
                .env("XDG_RUNTIME_DIR", directory.join("runtime"))
                .env(
                    "DBUS_SESSION_BUS_ADDRESS",
                    format!("unix:path={}", socket.display()),
                )
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        daemon
            .0
            .stdin
            .take()
            .unwrap()
            .write_all(b"fictional-failed-connect-keyring-password")
            .unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while !keyring::custody::available_at(Some(&socket)) {
            assert!(Instant::now() < until, "fixture custody unavailable");
            assert!(daemon.0.try_wait().unwrap().is_none());
            std::thread::sleep(Duration::from_millis(30));
        }
        Self {
            _daemon: daemon,
            _bus: bus,
            socket,
        }
    }
}

struct Cli {
    paths: Paths,
}

impl Cli {
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_connectors"))
            .env_clear()
            .args(["--output", "json", "--config"])
            .arg(&self.paths.config)
            .arg("--state-dir")
            .arg(&self.paths.state)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }
}

impl Drop for Cli {
    fn drop(&mut self) {
        if let Ok(client) = Client::connect(&self.paths, false) {
            let host = client.host_incarnation.clone();
            let _ = client.shutdown(&host);
        }
    }
}

fn private(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
#[ignore = "requires dbus-daemon, gnome-keyring-daemon and CONNECTORS_TEST_CATALOG_PROVIDER"]
fn a_connect_to_an_unreachable_provider_reports_the_owners_cause_at_dispatch() {
    let provider = PathBuf::from(
        std::env::var_os("CONNECTORS_TEST_CATALOG_PROVIDER")
            .expect("a built connectors-catalog-provider is required"),
    )
    .canonicalize()
    .unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/catalog");
    let root = tempfile::tempdir().unwrap();
    let custody = Custody::new(root.path());
    let provider_config = root.path().join("provider/jira.json");
    private(
        &provider_config,
        &serde_json::to_vec(&json!({
            "format": "connectors-catalog-local/2",
            "instance": "jira-unreachable",
            "provider": "jira",
            "bundle_directory": repository.join("generated/bundles").canonicalize().unwrap(),
            "api_base": "https://127.0.0.1:9/rest/api/3",
            "auth": {
                "profile": "atlassian.basic",
                "scheme": "basic",
                "header": "Authorization",
                "bearer": false,
                "account_label": "Account email",
                "label": "API token",
                "identity": {"path": "myself", "kind": "atlassian.account", "subject_pointer": "/accountId"}
            },
            "operations_file": repository.join("providers/jira/operations.json").canonicalize().unwrap(),
        }))
        .unwrap(),
    );
    let printed = Command::new(&provider)
        .arg("--local-config")
        .arg(&provider_config)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(printed.status.success(), "{printed:?}");
    let bootstrap: runtime::Bootstrap = serde_json::from_slice(&printed.stdout).unwrap();
    let cli = Cli {
        paths: Paths {
            config: root.path().join("cli/config.toml"),
            state: root.path().join("cli/state"),
        },
    };
    assert!(cli.run(&["setup", "init"]).status.success());
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let image = fs::read(&provider).unwrap();
    private(
        &cli.paths.config,
        format!(
            "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.jira]\ninstance_id='jira-unreachable'\nadapter_id='catalog'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.jira.permissions]\nprofiles=['atlassian.basic']\noperations=[]\n[adapters.jira.executable]\npath={}\nsha256={}\nargs=['--local-config',{}]\n",
            connectors_host::local::filesystem::uid(),
            q(custody.socket.to_str().unwrap()),
            q(&bootstrap.configuration_revision),
            q(provider.to_str().unwrap()),
            q(&hex(ring::digest::digest(&ring::digest::SHA256, &image).as_ref())),
            q(provider_config.to_str().unwrap()),
        )
        .as_bytes(),
    );
    let credential = root.path().join("private/credential.json");
    private(
        &credential,
        br#"{"account":"fixture-account@example.test","token":"fixture-api-token-one"}"#,
    );

    let refused = cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "jira",
        "--profile",
        "atlassian.basic",
        "--credential-file",
        credential.to_str().unwrap(),
    ]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert!(refused.stdout.is_empty());
    let data = serde_json::from_slice::<Value>(&refused.stderr).unwrap()["error"]["data"].clone();
    assert_ne!(data["code"], "outcome_unknown", "{data}");
    assert!(
        matches!(data["code"].as_str(), Some("unavailable" | "timeout")),
        "{data}"
    );
    assert_eq!(
        (&data["stage"], &data["next_action"]),
        (&json!("dispatch"), &json!("retry_explicitly")),
        "{data}"
    );
    let acquisition = data["acquisition"].as_str().expect("names its acquisition");

    let status = cli.run(&[
        "connections",
        "status",
        "--adapter",
        "jira",
        "--acquisition",
        acquisition,
    ]);
    assert!(status.status.success(), "{status:?}");
    let observed =
        serde_json::from_slice::<Value>(&status.stdout).unwrap()["result"]["acquisition"].clone();
    assert_eq!(observed["state"], "failed", "{observed}");
    assert_eq!(observed["next_action"], data["next_action"], "{observed}");
    assert!(observed.get("connection").is_none(), "{observed}");
}

/// A cached description with the one profile the stand-in owner captures.
fn bootstrap() -> runtime::Bootstrap {
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        revision: "desc-1".into(),
        operations: Vec::new(),
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        instance: "forge-local".into(),
        adapter: "catalog".into(),
        protocol: "v1alpha1".into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: "https://fixture.invalid".into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![profile()],
        requirements: Vec::new(),
    }
}

/// Publishes one ready connection through the registry's own lifecycle, with
/// its material in the disposable Secret Service. Returns its reference.
fn publish(state: &Path, bootstrap: &runtime::Bootstrap, socket: &Path) -> String {
    use connectors_host::local::keyring::custody;
    let registry = registry::Registry::with_system_clock(state);
    let binding = bootstrap.binding("fixture.basic").unwrap();
    let acquisition = registry.begin(&binding, connectors_sdk::now_ms()).unwrap();
    let claim = registry
        .consume(acquisition, connectors_sdk::now_ms())
        .unwrap();
    let material = connectors_sdk::Secret(br#"{"token":"fixture"}"#.to_vec());
    let now = connectors_sdk::now_ms();
    let baseline = registry::ValidatedBaseline {
        identity: registry::ExternalIdentity {
            kind: "fixture.user".into(),
            subject: "fixture-subject-1".into(),
        },
        granted_scopes: None,
        credential_expires_at_ms: None,
        collected_at_ms: now,
        valid_until_ms: now + 50_000,
    };
    let prepared = registry
        .prepare(&claim, baseline, material.0.len(), now)
        .unwrap();
    let store = custody::Store::open_at(prepared.version().scope(), Some(socket)).unwrap();
    registry
        .store_and_publish(prepared, &material, &store, connectors_sdk::now_ms)
        .unwrap()
}

#[test]
#[ignore = "requires dbus-daemon and gnome-keyring-daemon"]
fn a_revalidation_the_owner_definitely_fails_reports_dispatch_at_the_cli() {
    let root = tempfile::tempdir().unwrap();
    let custody = Custody::new(root.path());
    let cli = Cli {
        paths: Paths {
            config: root.path().join("cli/config.toml"),
            state: root.path().join("cli/state"),
        },
    };
    assert!(cli.run(&["setup", "init"]).status.success());
    private(
        &cli.paths.config,
        format!(
            "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\n[adapters.forge.permissions]\nprofiles=['fixture.basic']\noperations=[]\n[adapters.forge.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n",
            connectors_host::local::filesystem::uid(),
            serde_json::to_string(custody.socket.to_str().unwrap()).unwrap(),
            "a".repeat(64),
        )
        .as_bytes(),
    );
    let adapter = Config::load(&cli.paths.config).unwrap().adapters["forge"].clone();
    let bootstrap = bootstrap();
    runtime::state::State::new(&cli.paths.state)
        .remember(&adapter.selection(), &bootstrap)
        .unwrap();
    let reference = publish(&cli.paths.state, &bootstrap, &custody.socket);
    let revision = registry::Registry::with_system_clock(&cli.paths.state)
        .describe(
            "forge-local",
            "catalog",
            "cfg-1",
            &reference,
            connectors_sdk::now_ms(),
            true,
        )
        .unwrap()
        .revision;
    // The CLI greets with its own executable's digest.
    let image = fs::read(env!("CARGO_BIN_EXE_connectors")).unwrap();
    let build = hex(ring::digest::digest(&ring::digest::SHA256, &image).as_ref());
    for code in [Code::Unavailable, Code::Timeout] {
        let owner = serve(&cli.paths, build.clone(), Answer::Failed(code));
        let refused = cli.run(&[
            "connections",
            "revalidate",
            "--adapter",
            "forge",
            "--connection",
            &reference,
            "--expected-revision",
            &revision,
        ]);
        assert_eq!(owner.join().unwrap(), ["revalidate"]);
        fs::remove_file(cli.paths.state.join("owner.sock")).unwrap();
        assert_eq!(refused.status.code(), Some(1), "{refused:?}");
        assert!(refused.stdout.is_empty());
        let data =
            serde_json::from_slice::<Value>(&refused.stderr).unwrap()["error"]["data"].clone();
        assert_eq!(data["code"], serde_json::to_value(code).unwrap(), "{data}");
        assert_eq!(
            (&data["stage"], &data["next_action"]),
            (&json!("dispatch"), &json!("retry_explicitly")),
            "{data}"
        );
    }
}
