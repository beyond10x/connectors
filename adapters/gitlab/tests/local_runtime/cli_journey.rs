//! Production CLI, real private owner/adapter processes and disposable custody.
use super::*;
use connectors_host::local::{config::Paths, keyring, owner};
use std::{
    io::Write,
    process::{Child as Process, Output, Stdio},
    time::Instant,
};

struct OwnedProcess(Process);
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct Custody {
    daemon: Option<OwnedProcess>,
    _bus: OwnedProcess,
    directory: PathBuf,
    socket: PathBuf,
    epoch: u32,
}
impl Custody {
    fn new(parent: &std::path::Path) -> Self {
        let directory = parent.join("custody");
        filesystem::directory(&directory, true, true).unwrap();
        filesystem::directory(&directory.join("home"), true, true).unwrap();
        filesystem::directory(&directory.join("data/keyrings"), true, true).unwrap();
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
        while std::os::unix::net::UnixStream::connect(&socket).is_err() {
            assert!(Instant::now() < until, "fixture bus did not start");
            std::thread::sleep(Duration::from_millis(20));
        }
        let mut fixture = Self {
            daemon: None,
            _bus: bus,
            directory,
            socket,
            epoch: 0,
        };
        fixture.start();
        fixture
    }
    fn start(&mut self) {
        assert!(self.daemon.is_none());
        self.epoch += 1;
        let runtime = self.directory.join(format!("r{}", self.epoch));
        filesystem::directory(&runtime, true, true).unwrap();
        let mut daemon = OwnedProcess(
            Command::new("/usr/bin/gnome-keyring-daemon")
                .args([
                    "--foreground",
                    "--components=secrets",
                    "--unlock",
                    "--control-directory",
                ])
                .arg(&runtime)
                .env_clear()
                .env("PATH", "/usr/bin")
                .env("HOME", self.directory.join("home"))
                .env("XDG_DATA_HOME", self.directory.join("data"))
                .env("XDG_RUNTIME_DIR", &runtime)
                .env(
                    "DBUS_SESSION_BUS_ADDRESS",
                    format!("unix:path={}", self.socket.display()),
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
            .write_all(b"fictional-cli-fixture-keyring-password")
            .unwrap();
        self.daemon = Some(daemon);
        let until = Instant::now() + Duration::from_secs(10);
        while !keyring::custody::available_at(Some(&self.socket)) {
            assert!(
                Instant::now() < until,
                "qualified private fixture custody unavailable"
            );
            assert!(
                self.daemon
                    .as_mut()
                    .unwrap()
                    .0
                    .try_wait()
                    .unwrap()
                    .is_none()
            );
            std::thread::sleep(Duration::from_millis(30));
        }
    }
    fn restart(&mut self) {
        drop(self.daemon.take());
        self.start();
    }
}
struct Cli {
    binary: PathBuf,
    paths: Paths,
}
impl Cli {
    fn new(root: &std::path::Path) -> Self {
        let binary = std::env::var_os("CONNECTORS_TEST_CLI")
            .expect("built production CLI required")
            .into();
        Self {
            binary,
            paths: Paths {
                config: root.join("cli/config.toml"),
                state: root.join("cli/state"),
            },
        }
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .env_clear()
            .args(["--output", "json", "--config"])
            .arg(&self.paths.config)
            .arg("--state-dir")
            .arg(&self.paths.state)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }
    fn status(&self) -> Value {
        success(self.run(&["adapters", "status", "--adapter", "forge"]))["observation"].clone()
    }
    fn shutdown(&self) {
        if let Ok(client) = owner::Client::connect(&self.paths, false) {
            let host = client.host_incarnation.clone();
            client.shutdown(&host).unwrap();
            let until = Instant::now() + Duration::from_secs(5);
            while self.paths.state.join("owner.sock").exists() {
                assert!(Instant::now() < until, "owner did not finish cleanup");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}
impl Drop for Cli {
    fn drop(&mut self) {
        self.shutdown();
    }
}
#[track_caller]
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("fixture-pat"));
    value["result"].clone()
}
#[track_caller]
fn refusal(output: Output, code: &str) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("fixture-pat"));
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["error"]["data"]["code"], code, "{value}");
    value["error"]["data"].clone()
}

#[test]
#[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
fn gitlab_cli_failed_repair_and_busy_stop_preserve_authority() {
    let provider = Provider::new();
    let mut custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    fs::set_permissions(&credential, fs::Permissions::from_mode(0o644)).unwrap();
    let refused = refusal(
        cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "forge",
            "--profile",
            "gitlab.pat",
            "--credential-file",
            credential.to_str().unwrap(),
        ]),
        "protected_entry_unavailable",
    );
    assert_eq!(refused["kind"], "usage");
    assert_eq!(provider.count(), 0);
    fs::set_permissions(&credential, fs::Permissions::from_mode(0o600)).unwrap();
    let mut command = cli.command(&[
        "connections",
        "connect",
        "--adapter",
        "forge",
        "--profile",
        "gitlab.pat",
        "--credential-stdin",
    ]);
    let mut child = command.stdin(Stdio::piped()).spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&token(true).0)
        .unwrap();
    let connected = success(child.wait_with_output().unwrap())["connection"].clone();
    let reference = connected["summary"]["connection"].as_str().unwrap();
    let revision = connected["summary"]["revision"].as_str().unwrap();
    private(&credential, &token(false).0);
    refusal(
        cli.run(&[
            "connections",
            "repair",
            "--adapter",
            "forge",
            "--connection",
            reference,
            "--expected-revision",
            revision,
            "--credential-file",
            credential.to_str().unwrap(),
        ]),
        "identity_mismatch",
    );
    let observed = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "forge",
        "--connection",
        reference,
    ]));
    assert_eq!(observed["connection"]["summary"]["state"], "ready");
    assert_eq!(observed["connection"]["summary"]["revision"], revision);
    let describe = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "forge",
        "--operation",
        "project.get",
    ]));
    let schema = describe["schema"].as_str().unwrap();
    let descriptor = describe["revision"].as_str().unwrap();
    let invoke = [
        "operations",
        "invoke",
        "--adapter",
        "forge",
        "--connection",
        reference,
        "--operation",
        "project.get",
        "--schema",
        schema,
        "--revision",
        descriptor,
        "--input-json",
        r#"{"project":"org/project"}"#,
    ];
    success(cli.run(&invoke));
    let before = provider.count();
    let mut invalid = invoke;
    *invalid.last_mut().unwrap() = r#"{"project":"org/project","project":"org/project"}"#;
    refusal(cli.run(&invalid), "invalid_input");
    let mut stale = invoke;
    stale[9] = "old-schema";
    refusal(cli.run(&stale), "stale_description");
    let configuration = fs::read_to_string(&cli.paths.config).unwrap();
    private(
        &cli.paths.config,
        configuration
            .replace(
                "operations=['project.get','issues.list','file.get']",
                "operations=[]",
            )
            .as_bytes(),
    );
    refusal(cli.run(&invoke), "forbidden");
    private(&cli.paths.config, configuration.as_bytes());
    assert_eq!(provider.count(), before);
    let capture = owner::Client::connect(&cli.paths, false)
        .unwrap()
        .begin("forge", Some("gitlab.pat".into()), None, None)
        .unwrap();
    let coordinates = cli.status();
    provider
        .pause
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let busy = cli.command(&invoke).spawn().unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while provider.count() == before {
        assert!(Instant::now() < until, "fixture read was not dispatched");
        std::thread::sleep(Duration::from_millis(10));
    }
    // Observation and stop must not wait behind the provider's blocked channel.
    assert_eq!(cli.status()["state"], "ready");
    let start = Instant::now();
    success(cli.run(&[
        "adapters",
        "stop",
        "--adapter",
        "forge",
        "--expected-revision",
        coordinates["configuration_revision"].as_str().unwrap(),
        "--host-incarnation",
        coordinates["host_incarnation"].as_str().unwrap(),
        "--child-incarnation",
        coordinates["child_incarnation"].as_str().unwrap(),
    ]));
    assert!(start.elapsed() < Duration::from_secs(6));
    refusal(busy.wait_with_output().unwrap(), "unavailable");
    assert_eq!(
        capture.complete(&token(true)).unwrap_err().code,
        owner::Code::LifecycleConflict
    );
    assert_eq!(cli.status()["state"], "suppressed");
    assert_eq!(provider.count(), before + 1);
    cli.shutdown();
    drop(custody.daemon.take());
    refusal(cli.run(&invoke), "custody_unavailable");
    assert!(!cli.paths.state.join("owner.sock").exists());
    let database = fs::read(cli.paths.state.join("metadata.sqlite3")).unwrap();
    assert!(
        !database
            .windows(b"fixture-pat".len())
            .any(|w| w == b"fixture-pat")
    );
}
fn configure(cli: &Cli, provider: &Provider, custody: &Custody) {
    success(cli.run(&["setup", "init"]));
    let adapter = provider.selection();
    // JSON string/array literals are valid TOML for these fixture-only ASCII
    // paths and selectors. No credential is a configuration input.
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let configuration = format!(
        "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.forge]\ninstance_id={}\nadapter_id='gitlab'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.forge.permissions]\nprofiles=['gitlab.pat']\noperations=['project.get','issues.list','file.get']\n[adapters.forge.executable]\npath={}\nsha256={}\nargs={}\n",
        filesystem::uid(),
        q(custody.socket.to_str().unwrap()),
        q(&adapter.instance_id),
        q(&adapter.configuration_revision),
        q(adapter.executable.path.to_str().unwrap()),
        q(&adapter.executable.sha256),
        serde_json::to_string(&adapter.executable.args).unwrap()
    );
    private(&cli.paths.config, configuration.as_bytes());
    connectors_host::local::config::Config::load(&cli.paths.config).unwrap();
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn gitlab_cli_explicit_revalidation_after_real_expiry_without_reentry() {
    let provider = Provider::new();
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "forge",
        "--profile",
        "gitlab.pat",
        "--credential-file",
        credential.to_str().unwrap(),
    ]))["connection"]
        .clone();
    let reference = connected["summary"]["connection"].as_str().unwrap();
    let revision = connected["summary"]["revision"].as_str().unwrap();
    let revalidate = [
        "connections",
        "revalidate",
        "--adapter",
        "forge",
        "--connection",
        reference,
        "--expected-revision",
        revision,
    ];
    let status = [
        "connections",
        "status",
        "--adapter",
        "forge",
        "--connection",
        reference,
    ];
    let descriptor = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "forge",
        "--operation",
        "project.get",
    ]));
    let invoke = [
        "operations",
        "invoke",
        "--adapter",
        "forge",
        "--connection",
        reference,
        "--operation",
        "project.get",
        "--schema",
        descriptor["schema"].as_str().unwrap(),
        "--revision",
        descriptor["revision"].as_str().unwrap(),
        "--input-json",
        r#"{"project":"org/project"}"#,
    ];
    fs::remove_file(&credential).unwrap();
    cli.shutdown();
    let until = connected["valid_until_ms"].as_u64().unwrap();
    while connectors_sdk::now_ms() <= until {
        std::thread::sleep(Duration::from_millis(100));
    }
    let before = provider.count();
    assert_eq!(
        success(cli.run(&status))["connection"]["summary"]["state"],
        "pending"
    );
    refusal(cli.run(&invoke), "not_granted");
    assert_eq!(provider.count(), before);
    assert!(!cli.paths.state.join("owner.sock").exists());
    let refreshed = success(cli.run(&revalidate))["connection"].clone();
    assert_eq!(refreshed["summary"], connected["summary"]);
    assert!(refreshed["valid_until_ms"].as_u64().unwrap() > until);
    assert_eq!(provider.count(), before + 2);
    let result = success(cli.run(&invoke));
    let result: Value = serde_json::from_str(result["result"].as_str().unwrap()).unwrap();
    assert_eq!(result["item"]["name"], "fixture-project");
    // Upstream unavailability is not positive invalidity of the retained token.
    provider
        .response_status
        .store(503, std::sync::atomic::Ordering::SeqCst);
    assert!(!cli.run(&revalidate).status.success());
    assert_eq!(
        success(cli.run(&status))["connection"]["summary"]["state"],
        "ready"
    );
    provider
        .response_status
        .store(0, std::sync::atomic::Ordering::SeqCst);
    success(cli.run(&invoke));
    provider
        .response_status
        .store(401, std::sync::atomic::Ordering::SeqCst);
    refusal(cli.run(&revalidate), "service_failure");
    assert_eq!(
        success(cli.run(&status))["connection"]["summary"]["state"],
        "reauthorization_required"
    );
    cli.shutdown();
    let before = provider.count();
    refusal(cli.run(&revalidate), "not_granted");
    assert_eq!(provider.count(), before);
    assert!(!cli.paths.state.join("owner.sock").exists());
}

#[test]
#[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
fn persistent_gitlab_cli_owner_and_keyring_restart() {
    let provider = Provider::new();
    let mut custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    assert_eq!(cli.status()["state"], "owner_unavailable");
    refusal(
        cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "forge",
            "--profile",
            "missing",
            "--credential-file",
            "/never-read",
        ]),
        "forbidden",
    );
    assert!(!cli.paths.state.join("owner.sock").exists());
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let connect = || {
        cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "forge",
            "--profile",
            "gitlab.pat",
            "--credential-file",
            credential.to_str().unwrap(),
        ])
    };
    let connected = success(connect())["connection"].clone();
    let reference = connected["summary"]["connection"].as_str().unwrap();
    let revision = connected["summary"]["revision"].as_str().unwrap();
    assert_eq!(connected["summary"]["state"], "ready");
    assert_eq!(provider.count(), 2);
    let describe = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "forge",
        "--operation",
        "project.get",
    ]));
    let schema = describe["schema"].as_str().unwrap();
    let descriptor = describe["revision"].as_str().unwrap();
    let invoke = [
        "operations",
        "invoke",
        "--adapter",
        "forge",
        "--connection",
        reference,
        "--operation",
        "project.get",
        "--schema",
        schema,
        "--revision",
        descriptor,
        "--input-json",
        r#"{"project":"org/project"}"#,
    ];
    let value = success(cli.run(&invoke));
    assert_eq!(
        serde_json::from_str::<Value>(value["result"].as_str().unwrap()).unwrap()["item"]["name"],
        "fixture-project"
    );
    let old_host = cli.status()["host_incarnation"].clone();
    cli.shutdown();
    custody.restart();
    fs::remove_file(&credential).unwrap();
    // Every command is a new production CLI process. These simultaneous reads
    // must share one newly started owner and reuse existing custody material.
    let children: Vec<_> = (0..4)
        .map(|_| cli.command(&invoke).spawn().unwrap())
        .collect();
    for child in children {
        success(child.wait_with_output().unwrap());
    }
    let status = cli.status();
    assert_ne!(status["host_incarnation"], old_host);
    assert_eq!(status["state"], "ready");
    assert_eq!(provider.count(), 7); // Two identity checks, five reads; no re-entry.
    let current = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "forge",
        "--connection",
        reference,
    ]));
    assert_eq!(current["connection"]["summary"]["revision"], revision);
    let stop = |host: &str, child: &str| {
        cli.run(&[
            "adapters",
            "stop",
            "--adapter",
            "forge",
            "--expected-revision",
            status["configuration_revision"].as_str().unwrap(),
            "--host-incarnation",
            host,
            "--child-incarnation",
            child,
        ])
    };
    refusal(
        stop(status["host_incarnation"].as_str().unwrap(), "stale-child"),
        "incarnation_mismatch",
    );
    assert_eq!(
        cli.status()["child_incarnation"],
        status["child_incarnation"]
    );
    success(stop(
        status["host_incarnation"].as_str().unwrap(),
        status["child_incarnation"].as_str().unwrap(),
    ));
    assert_eq!(cli.status()["state"], "suppressed");
    cli.shutdown();
    assert_eq!(cli.status()["state"], "owner_unavailable");
    success(cli.run(&invoke));
    assert_ne!(
        cli.status()["child_incarnation"],
        status["child_incarnation"]
    );
    success(cli.run(&[
        "connections",
        "revoke",
        "--adapter",
        "forge",
        "--connection",
        reference,
        "--expected-revision",
        revision,
    ]));
    cli.shutdown();
    let before = provider.count();
    refusal(cli.run(&invoke), "revoked");
    assert_eq!(provider.count(), before);
    assert!(!cli.paths.state.join("owner.sock").exists());
}
