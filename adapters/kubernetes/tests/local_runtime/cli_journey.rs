//! Production CLI, real private owner/adapter processes and disposable custody.
//! The Kubernetes counterpart of the GitLab journey: it exercises the same
//! provider-neutral setup, connection and operation surface against a local TLS
//! fixture cluster and a task-owned Secret Service.
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
        success(self.run(&["adapters", "status", "--adapter", "cluster"]))["observation"].clone()
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
    assert!(!String::from_utf8_lossy(&output.stdout).contains("fixture-kubernetes-token"));
    value["result"].clone()
}
#[track_caller]
fn refusal(output: Output, code: &str) -> Value {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("fixture-kubernetes-token"));
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["error"]["data"]["code"], code, "{value}");
    value["error"]["data"].clone()
}

fn configure(cli: &Cli, cluster: &Cluster, custody: &Custody) {
    success(cli.run(&["setup", "init"]));
    success(cli.run(&["setup", "check"]));
    let adapter = cluster.selection();
    // JSON string/array literals are valid TOML for these fixture-only ASCII
    // paths and selectors. No credential is a configuration input.
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let configuration = format!(
        "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.cluster]\ninstance_id={}\nadapter_id='kubernetes'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.cluster.permissions]\nprofiles=['kubernetes.token']\noperations=['resources.list','endpoints.discover']\n[adapters.cluster.executable]\npath={}\nsha256={}\nargs={}\n",
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
fn persistent_kubernetes_cli_owner_and_keyring_restart() {
    // Host discovery is configured, so `hosts.discover` is advertised. It is
    // deliberately left out of the permitted operations below.
    let cluster = Cluster::new(true);
    let mut custody = Custody::new(cluster.root.path());
    let cli = Cli::new(cluster.root.path());
    configure(&cli, &cluster, &custody);
    assert_eq!(cli.status()["state"], "owner_unavailable");
    // An undeclared profile is refused before any owner or adapter starts.
    refusal(
        cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "cluster",
            "--profile",
            "missing",
            "--credential-file",
            "/never-read",
        ]),
        "forbidden",
    );
    assert!(!cli.paths.state.join("owner.sock").exists());
    assert_eq!(cluster.count(), 0);

    let credential = cluster.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "cluster",
        "--profile",
        "kubernetes.token",
        "--credential-file",
        credential.to_str().unwrap(),
    ]))["connection"]
        .clone();
    let reference = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let revision = connected["summary"]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(connected["summary"]["state"], "ready");
    // One SelfSubjectReview identity probe, and no business read.
    assert_eq!(cluster.count(), 1);
    assert_eq!(
        cluster.routes(),
        ["/apis/authentication.k8s.io/v1/selfsubjectreviews"]
    );

    let describe = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "cluster",
        "--operation",
        "resources.list",
    ]));
    let schema = describe["schema"].as_str().unwrap().to_owned();
    let descriptor = describe["revision"].as_str().unwrap().to_owned();
    let invoke = [
        "operations",
        "invoke",
        "--adapter",
        "cluster",
        "--connection",
        &reference,
        "--operation",
        "resources.list",
        "--schema",
        &schema,
        "--revision",
        &descriptor,
        "--input-json",
        r#"{"namespace":"fixture","kind":"pods","limit":1}"#,
    ];
    let value = success(cli.run(&invoke));
    assert_eq!(
        serde_json::from_str::<Value>(value["result"].as_str().unwrap()).unwrap()["items"][0]["metadata"]
            ["name"],
        "pod-0"
    );

    let old_host = cli.status()["host_incarnation"].clone();
    cli.shutdown();
    custody.restart();
    // The saved exact credential version is the only remaining copy.
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
    // One identity probe, five reads; no credential re-entry.
    assert_eq!(cluster.count(), 6);
    let current = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "cluster",
        "--connection",
        &reference,
    ]));
    assert_eq!(current["connection"]["summary"]["revision"], revision);

    // An advertised operation the configuration did not permit is refused by
    // admission at description, before any provider request. The cached
    // description is not a permission and does not disclose the operation.
    let before = cluster.count();
    refusal(
        cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "cluster",
            "--operation",
            "hosts.discover",
        ]),
        "forbidden",
    );
    assert_eq!(cluster.count(), before);

    // Terminal local revocation survives a restart and does not re-enter custody.
    success(
        cli.run(&[
            "connections",
            "revoke",
            "--adapter",
            "cluster",
            "--connection",
            &reference,
            "--expected-revision",
            current["connection"]["summary"]["revision"]
                .as_str()
                .unwrap(),
        ]),
    );
    cli.shutdown();
    let revoked = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "cluster",
        "--connection",
        &reference,
    ]));
    assert_eq!(revoked["connection"]["summary"]["state"], "revoked");
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn kubernetes_cli_refuses_a_changed_cluster_identity_and_preserves_the_saved_credential() {
    let cluster = Cluster::new(false);
    let custody = Custody::new(cluster.root.path());
    let cli = Cli::new(cluster.root.path());
    configure(&cli, &cluster, &custody);
    let credential = cluster.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "cluster",
        "--profile",
        "kubernetes.token",
        "--credential-file",
        credential.to_str().unwrap(),
    ]))["connection"]
        .clone();
    let reference = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let revision = connected["summary"]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    // A different Kubernetes principal is a replacement, not a renewal.
    private(&credential, &token(false).0);
    refusal(
        cli.run(&[
            "connections",
            "repair",
            "--adapter",
            "cluster",
            "--connection",
            &reference,
            "--expected-revision",
            &revision,
            "--credential-file",
            credential.to_str().unwrap(),
        ]),
        "identity_mismatch",
    );
    let observed = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "cluster",
        "--connection",
        &reference,
    ]));
    assert_eq!(observed["connection"]["summary"]["state"], "ready");
    assert_eq!(observed["connection"]["summary"]["revision"], revision);
    // The still-valid saved credential remains usable after the failed repair.
    let describe = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "cluster",
        "--operation",
        "endpoints.discover",
    ]));
    let result = success(cli.run(&[
        "operations",
        "invoke",
        "--adapter",
        "cluster",
        "--connection",
        &reference,
        "--operation",
        "endpoints.discover",
        "--schema",
        describe["schema"].as_str().unwrap(),
        "--revision",
        describe["revision"].as_str().unwrap(),
        "--input-json",
        r#"{"namespace":"fixture","limit":10}"#,
    ]));
    assert_eq!(
        serde_json::from_str::<Value>(result["result"].as_str().unwrap()).unwrap()["items"][0]["address"],
        "10.0.0.7"
    );
}
