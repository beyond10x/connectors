//! story:connection-next-actions-no-cycle (issue 181). Every next action a
//! connection command answers is accepted by the state it leads to: from a
//! pending connection whose configuration moved and whose credential the new
//! provider refuses (an Atlassian site whose v2 probe answers 401 "scope does
//! not match"), following each answer's `next_action` ends in a connection or
//! in a refusal that names the failed probe, and no answer repeats.
//!
//! The production CLI and owner run against an adapter child that is this test
//! binary, whose validation answers `insufficient_scope`. The cases need a
//! disposable Secret Service (`dbus-daemon`, `gnome-keyring-daemon`) and are
//! ignored by default.
#![cfg(unix)]
use connectors_host::local::{
    config::{Config, Paths},
    owner::Client,
    registry, runtime,
};
const REASON: &str = "Unauthorized; scope does not match";

use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
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
        let keyring_stderr = (directory.join("runtime")).with_extension("stderr");
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
                .stderr(Stdio::from(std::fs::File::create(&keyring_stderr).unwrap()))
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
            assert!(
                Instant::now() < until,
                "fixture custody unavailable; keyring stderr: {}",
                keyring_stderr_tail(&keyring_stderr)
            );
            assert!(
                daemon.0.try_wait().unwrap().is_none(),
                "fixture keyring daemon exited; its stderr: {}",
                keyring_stderr_tail(&keyring_stderr)
            );
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
        bindings: None,
        credential_revision: None,
        executable: None,
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
        binding: None,
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

/// The adapter child: the production owner launches this test binary,
/// re-entered through libtest's exact filter. Every validation answers the
/// provider's refusal of a credential below the profile's scope.
#[test]
fn scope_refusing_provider_fixture() {
    let args: Vec<String> = std::env::args().collect();
    if !args.iter().any(|arg| arg == "--connectors-private-fd") {
        return;
    }
    let revision = args
        .iter()
        .find_map(|arg| arg.strip_prefix("fixture-revision="))
        .unwrap()
        .to_owned();
    let _ = runtime::serve(3, RefusingProvider { revision });
}

struct RefusingProvider {
    revision: String,
}

#[async_trait::async_trait]
impl runtime::Adapter for RefusingProvider {
    fn bootstrap(&self) -> runtime::Bootstrap {
        runtime::Bootstrap {
            bindings: None,
            configuration_revision: self.revision.clone(),
            ..bootstrap()
        }
    }
    async fn validate(
        &self,
        _: &str,
        _: connectors_sdk::Secret,
    ) -> runtime::Result<runtime::Baseline> {
        Err(runtime::Failure::InsufficientScope)
    }
    async fn validate_explained(
        &self,
        _: &str,
        _: connectors_sdk::Secret,
    ) -> std::result::Result<runtime::Baseline, runtime::Refusal> {
        Err(runtime::Refusal {
            failure: runtime::Failure::InsufficientScope,
            reason: Some(REASON.into()),
            retry_after_seconds: None,
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
}

/// One command of the walk and what it answered.
#[derive(Debug)]
struct Step {
    /// The connection's state and `pending_reason` as `connections describe`
    /// reported them before the command ran.
    before: (Value, Value),
    command: &'static str,
    exit: Option<i32>,
    code: String,
    stage: String,
    next_action: String,
    reason: String,
}

fn strings(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

/// Publishes a ready connection under `cfg-1`, moves the configuration to
/// `cfg-2` (whose provider refuses every credential), then follows each
/// answer's `next_action` from a `connections revalidate`, for at most
/// `limit` commands.
fn walk(limit: usize) -> Vec<Step> {
    walk_from("revalidate_connection", limit)
}

/// `walk`, starting from the command the next action `start` names.
fn walk_from(start: &str, limit: usize) -> Vec<Step> {
    let root = tempfile::tempdir().unwrap();
    let custody = Custody::new(root.path());
    let cli = Cli {
        paths: Paths {
            config: root.path().join("cli/config.toml"),
            state: root.path().join("cli/state"),
        },
    };
    assert!(cli.run(&["setup", "init"]).status.success());
    let provider = std::env::current_exe().unwrap().canonicalize().unwrap();
    let digest =
        hex(ring::digest::digest(&ring::digest::SHA256, &fs::read(&provider).unwrap()).as_ref());
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let config = |revision: &str| {
        let args = [
            "--exact".to_owned(),
            "scope_refusing_provider_fixture".to_owned(),
            "--".to_owned(),
            format!("fixture-revision={revision}"),
        ];
        format!(
            "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='{revision}'\nprotocol='v1alpha1'\n[adapters.forge.permissions]\nprofiles=['fixture.basic']\noperations=[]\n[adapters.forge.executable]\npath={}\nsha256='{digest}'\nargs={}\n",
            connectors_host::local::filesystem::uid(),
            q(custody.socket.to_str().unwrap()),
            q(provider.to_str().unwrap()),
            serde_json::to_string(&args).unwrap(),
        )
    };
    private(&cli.paths.config, config("cfg-1").as_bytes());
    let adapter = Config::load(&cli.paths.config).unwrap().adapters["forge"].clone();
    runtime::state::State::new(&cli.paths.state)
        .remember(&adapter.selection(), &bootstrap())
        .unwrap();
    let reference = publish(&cli.paths.state, &bootstrap(), &custody.socket);
    private(&cli.paths.config, config("cfg-2").as_bytes());
    let credential = root.path().join("private/credential.json");
    private(&credential, br#"{"token":"fixture-token-one"}"#);
    let credential = credential.to_str().unwrap().to_owned();
    let revision = || {
        registry::Registry::with_system_clock(&cli.paths.state)
            .describe(
                "forge-local",
                "catalog",
                "cfg-1",
                &reference,
                connectors_sdk::now_ms(),
                true,
            )
            .unwrap()
            .revision
    };
    let mut steps = Vec::new();
    let mut action = start.to_owned();
    while steps.len() < limit {
        let mut args = strings(&["connections", "", "--adapter", "forge"]);
        let command = match action.as_str() {
            "revalidate_connection" | "repair_connection" | "revoke_connection" => {
                let command = match action.as_str() {
                    "revalidate_connection" => "revalidate",
                    "repair_connection" => "reconnect",
                    _ => "revoke",
                };
                args[1] = command.into();
                args.extend(strings(&["--connection", &reference]));
                args.extend(strings(&["--expected-revision", &revision()]));
                if command == "reconnect" {
                    args.extend(strings(&["--credential-file", &credential]));
                }
                command
            }
            "create_connection" => {
                args[1] = "connect".into();
                args.extend(strings(&[
                    "--profile",
                    "fixture.basic",
                    "--credential-file",
                    &credential,
                ]));
                "connect"
            }
            _ => break,
        };
        let listed = cli.run(&["connections", "list", "--adapter", "forge"]);
        let summary = serde_json::from_slice::<Value>(&listed.stdout)
            .ok()
            .and_then(|v| {
                v["result"]["connections"]
                    .as_array()?
                    .iter()
                    .find(|c| c["connection"] == reference.as_str())
                    .cloned()
            })
            .unwrap_or(Value::Null);
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = cli.run(&refs);
        let data = serde_json::from_slice::<Value>(&output.stderr)
            .map(|v| v["error"]["data"].clone())
            .unwrap_or(Value::Null);
        let text = |key: &str| data[key].as_str().unwrap_or("").to_owned();
        eprintln!("{command}: {:?} {data}", output.status.code());
        action = text("next_action");
        steps.push(Step {
            before: (summary["state"].clone(), summary["pending_reason"].clone()),
            command,
            exit: output.status.code(),
            code: text("code"),
            stage: text("stage"),
            next_action: action.clone(),
            reason: text("service_reason"),
        });
        if output.status.success() {
            // A revoked connection leaves the instance to connect again;
            // anything else that succeeded is the connection.
            if command != "revoke" {
                break;
            }
            action = "create_connection".to_owned();
        }
    }
    steps
}

/// Following the answers from a pending connection whose provider refuses the
/// credential reaches a success or a refusal that offers no further command
/// within three commands, and no command's answer repeats.
#[test]
#[ignore = "requires dbus-daemon and gnome-keyring-daemon"]
fn following_next_actions_from_a_refused_upgrade_does_not_cycle() {
    let steps = walk(6);
    let seen: Vec<_> = steps
        .iter()
        .map(|s| {
            (
                s.command,
                s.code.clone(),
                s.stage.clone(),
                s.next_action.clone(),
            )
        })
        .collect();
    let distinct: BTreeSet<_> = seen.iter().collect();
    assert_eq!(distinct.len(), seen.len(), "an answer repeats: {steps:#?}");
    // The revalidation, the revocation it names, and the connect whose
    // refusal names the probe.
    assert!(steps.len() <= 3, "no end within three commands: {steps:#?}");
    let last = steps.last().unwrap();
    assert!(
        last.exit == Some(0)
            || (last.code == "not_granted"
                && last.next_action == "retry_explicitly"
                && last.reason == REASON),
        "the walk did not end in a connection or the probe's refusal: {steps:#?}"
    );
}

/// A connection pending because its configuration moved says so, and says
/// when the configured provider already refused its credential.
#[test]
#[ignore = "requires dbus-daemon and gnome-keyring-daemon"]
fn a_pending_connection_says_why() {
    let steps = walk(2);
    assert_eq!(
        steps[0].before,
        (json!("pending"), json!("configuration_changed")),
        "{steps:#?}"
    );
    assert_eq!(
        steps[1].before,
        (json!("pending"), json!("configuration_refused")),
        "{steps:#?}"
    );
}

/// Adversary: the story's Outcome and Acceptance read literally for one walk.
/// Every next action is one the state it leads to accepts (no command a step
/// named is refused as `lifecycle_conflict`), no answer repeats, and the walk
/// ends in a connection or the probe's refusal within three commands.
fn assert_story_holds(start: &str, steps: &[Step], most: usize) {
    let seen: Vec<_> = steps
        .iter()
        .map(|s| {
            (
                s.command,
                s.code.clone(),
                s.stage.clone(),
                s.next_action.clone(),
            )
        })
        .collect();
    let distinct: BTreeSet<_> = seen.iter().collect();
    assert_eq!(
        distinct.len(),
        seen.len(),
        "from {start}, an answer repeats: {steps:#?}"
    );
    for (i, step) in steps.iter().enumerate().skip(1) {
        assert_ne!(
            step.code,
            "lifecycle_conflict",
            "from {start}, step {i} ({}) ran the command step {} named and was refused by the state it led to: {steps:#?}",
            step.command,
            i - 1
        );
    }
    assert!(
        steps.len() <= most,
        "from {start}, no end within {most} commands (story Acceptance): {steps:#?}"
    );
    let last = steps.last().unwrap();
    assert!(
        last.exit == Some(0) || (last.code == "not_granted" && last.reason == REASON),
        "from {start}, the walk did not end in a connection or the probe's refusal naming the probe: {steps:#?}"
    );
}

#[test]
#[ignore = "requires dbus-daemon and gnome-keyring-daemon"]
fn adversary_the_walk_from_revalidate_satisfies_the_story() {
    assert_story_holds("revalidate", &walk(6), 3);
}

#[test]
#[ignore = "requires dbus-daemon and gnome-keyring-daemon"]
/// The decided reading of the Acceptance: three next actions followed after
/// the starting command, so at most four commands with the start.
fn adversary_the_walk_from_reconnect_satisfies_the_story() {
    assert_story_holds("reconnect", &walk_from("repair_connection", 6), 4);
}

#[test]
#[ignore = "requires dbus-daemon and gnome-keyring-daemon"]
/// The decided reading of the Acceptance: three next actions followed after
/// the starting command, so at most four commands with the start.
fn adversary_the_walk_from_connect_satisfies_the_story() {
    assert_story_holds("connect", &walk_from("create_connection", 6), 4);
}

/// The last lines the fixture's GNOME Keyring wrote to its standard error, for
/// a failure message: the daemon's own words on why it never answered.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn keyring_stderr_tail(path: &std::path::Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => {
            format!("(empty: {} was written nothing)", path.display())
        }
        Ok(text) => {
            let lines: Vec<&str> = text.lines().collect();
            lines[lines.len().saturating_sub(20)..].join("\n")
        }
        Err(error) => format!("(unreadable: {}: {error})", path.display()),
    }
}
