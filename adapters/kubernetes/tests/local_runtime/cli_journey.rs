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
    configure_operations(
        cli,
        cluster,
        custody,
        &["resources.list", "endpoints.discover"],
    );
}

fn configure_operations(cli: &Cli, cluster: &Cluster, custody: &Custody, operations: &[&str]) {
    success(cli.run(&["setup", "init"]));
    success(cli.run(&["setup", "check"]));
    let adapter = cluster.selection();
    // JSON string/array literals are valid TOML for these fixture-only ASCII
    // paths and selectors. No credential is a configuration input.
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let configuration = format!(
        "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.cluster]\ninstance_id={}\nadapter_id='kubernetes'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.cluster.permissions]\nprofiles=['kubernetes.token']\noperations={}\n[adapters.cluster.executable]\npath={}\nsha256={}\nargs={}\n",
        filesystem::uid(),
        q(custody.socket.to_str().unwrap()),
        q(&adapter.instance_id),
        q(&adapter.configuration_revision),
        serde_json::to_string(operations).unwrap(),
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

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn kubernetes_cli_reads_helm_release_history_and_redacted_values() {
    // Content disclosure is configured, so all four release operations are
    // advertised. `helm_releases.manifest` is deliberately left out of the
    // permitted operations, which is a separate decision from advertisement.
    let cluster = Cluster::with(false, "redacted_content");
    let custody = Custody::new(cluster.root.path());
    let cli = Cli::new(cluster.root.path());
    configure_operations(
        &cli,
        &cluster,
        &custody,
        &[
            "helm_releases.history",
            "helm_releases.status",
            "helm_releases.values",
        ],
    );
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
    assert_eq!(connected["summary"]["state"], "ready");
    // One identity probe and no business read.
    assert_eq!(cluster.count(), 1);

    let read = |operation: &str, input: &str| -> Value {
        let describe = success(cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "cluster",
            "--operation",
            operation,
        ]));
        let output = cli.run(&[
            "operations",
            "invoke",
            "--adapter",
            "cluster",
            "--connection",
            &reference,
            "--operation",
            operation,
            "--schema",
            describe["schema"].as_str().unwrap(),
            "--revision",
            describe["revision"].as_str().unwrap(),
            "--input-json",
            input,
        ]);
        // The production CLI's own bytes must not carry a recorded literal.
        let printed = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(!printed.contains(RECORDED_VALUE_SECRET), "{operation}");
        assert!(!printed.contains(RENDERED_MANIFEST_SECRET), "{operation}");
        serde_json::from_str(success(output)["result"].as_str().unwrap()).unwrap()
    };

    let history = read(
        "helm_releases.history",
        r#"{"namespace":"fixture","release":"api","limit":50}"#,
    );
    let revisions: Vec<u64> = history["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["revision"].as_u64().unwrap())
        .collect();
    assert_eq!(revisions, [1, 2, 3]);
    assert_eq!(history["complete"], true);
    assert_eq!(
        history["items"][2]["source_secret"],
        "sh.helm.release.v1.api.v3"
    );

    let status = read(
        "helm_releases.status",
        r#"{"namespace":"fixture","release":"api","limit":50}"#,
    );
    assert_eq!(status["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(status["items"][0]["status"], "deployed");

    let values = read(
        "helm_releases.values",
        r#"{"namespace":"fixture","release":"api","revision":3,"limit":200}"#,
    );
    assert_eq!(values["complete"], true);
    assert_eq!(
        values["provenance"]["resource"],
        "fixture/sh.helm.release.v1.api.v3/values"
    );
    assert!(
        values["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"] == "postgresql.auth.password" && item["kind"] == "string")
    );

    // An advertised release operation the configuration did not permit is
    // refused at description, before any provider request.
    let before = cluster.count();
    refusal(
        cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "cluster",
            "--operation",
            "helm_releases.manifest",
        ]),
        "forbidden",
    );
    assert_eq!(cluster.count(), before);

    // A release outside the configured namespace scope never reaches the
    // cluster, and is distinct from the empty history `backendless` returns.
    let describe = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "cluster",
        "--operation",
        "helm_releases.history",
    ]));
    let outside = [
        "operations",
        "invoke",
        "--adapter",
        "cluster",
        "--connection",
        &reference,
        "--operation",
        "helm_releases.history",
        "--schema",
        describe["schema"].as_str().unwrap(),
        "--revision",
        describe["revision"].as_str().unwrap(),
        "--input-json",
        r#"{"namespace":"kube-system","release":"api","limit":50}"#,
    ];
    let before = cluster.count();
    refusal(cli.run(&outside), "forbidden");
    assert_eq!(cluster.count(), before);
    let empty = read(
        "helm_releases.history",
        r#"{"namespace":"backendless","release":"api","limit":50}"#,
    );
    assert_eq!(empty["items"].as_array().map(Vec::len), Some(0));
    assert_eq!(empty["complete"], true);
}

/// Opt-in real cluster: `CONNECTORS_K8S_SANDBOX=<api-base>` with
/// `CONNECTORS_K8S_CA` and `CONNECTORS_K8S_TOKEN` naming owner-only files.
/// Without them the test is skipped rather than silently passing.
fn k8s_sandbox() -> Option<(String, PathBuf, PathBuf)> {
    Some((
        std::env::var("CONNECTORS_K8S_SANDBOX").ok()?,
        std::env::var_os("CONNECTORS_K8S_CA")?.into(),
        std::env::var_os("CONNECTORS_K8S_TOKEN")?.into(),
    ))
}

#[test]
#[ignore = "requires CONNECTORS_K8S_SANDBOX, a built production CLI and qualified disposable Secret Service"]
fn a_real_cluster_session_persists_across_cli_and_owner_restart() {
    let Some((api_base, ca, token_file)) = k8s_sandbox() else {
        eprintln!("CONNECTORS_K8S_SANDBOX is unset; skipping");
        return;
    };
    let root = tempfile::tempdir().unwrap();
    filesystem::directory(&root.path().join("private"), true, true).unwrap();
    let mut custody = Custody::new(root.path());
    let cli = Cli::new(root.path());

    let native = root.path().join("private/kubernetes.json");
    private(
        &native,
        &serde_json::to_vec(&json!({
            "format":"connectors-kubernetes-local/1","instance":"k8s-sandbox",
            "api_base":api_base,"ca_file":ca,
            "namespaces":["fixture"],
            "resource_kinds":["pods","services","endpointslices"],
            "discover_hosts":false
        }))
        .unwrap(),
    );
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-kubernetes"))
        .canonicalize()
        .unwrap();
    let output = Command::new(&binary)
        .arg("--local-config")
        .arg(&native)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
    bootstrap.validate().unwrap();
    success(cli.run(&["setup", "init"]));
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let configuration = format!(
        "format='connectors-local/1'\nowner_uid={}\nsecret_service_socket={}\n[adapters.cluster]\ninstance_id='k8s-sandbox'\nadapter_id='kubernetes'\nconfiguration_revision={}\nprotocol='v1alpha1'\nstartup='on-demand'\nrestart='never'\n[adapters.cluster.permissions]\nprofiles=['kubernetes.token']\noperations=['resources.list','endpoints.discover']\n[adapters.cluster.executable]\npath={}\nsha256={}\nargs={}\n",
        filesystem::uid(),
        q(custody.socket.to_str().unwrap()),
        q(&bootstrap.configuration_revision),
        q(binary.to_str().unwrap()),
        q(&hex::encode(Sha256::digest(fs::read(&binary).unwrap()))),
        serde_json::to_string(&["--local-config", native.to_str().unwrap()]).unwrap()
    );
    private(&cli.paths.config, configuration.as_bytes());
    connectors_host::local::config::Config::load(&cli.paths.config).unwrap();

    // The credential document carries the real service-account token.
    let credential = root.path().join("private/credential.json");
    let token = fs::read_to_string(&token_file).unwrap();
    private(
        &credential,
        &serde_json::to_vec(&json!({ "token": token.trim() })).unwrap(),
    );
    let rejected = root.path().join("private/rejected.json");
    private(&rejected, br#"{"token":"not.a.valid.token"}"#);
    let refused = refusal(
        cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "cluster",
            "--profile",
            "kubernetes.token",
            "--credential-file",
            rejected.to_str().unwrap(),
        ]),
        "service_failure",
    );
    assert_eq!(refused["service_code"], "unauthorized", "{refused}");

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
    let pods = [
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
        r#"{"namespace":"fixture","kind":"pods","limit":10}"#,
    ];
    let value = success(cli.run(&pods));
    let page: Value = serde_json::from_str(value["result"].as_str().unwrap()).unwrap();
    assert!(
        page["items"].as_array().is_some_and(|i| !i.is_empty()),
        "the sandbox namespace has a pod: {page}"
    );
    assert_eq!(page["complete"], true);

    // A namespace outside the configured scope is refused before any request.
    let outside = [
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
        r#"{"namespace":"kube-system","kind":"pods","limit":10}"#,
    ];
    refusal(cli.run(&outside), "forbidden");

    // The cluster's own RBAC denial is distinct from the local scope refusal:
    // deployments are inside the configured kinds for neither this role nor
    // this configuration, so ask for a kind the role cannot read.
    cli.shutdown();
    custody.restart();
    fs::remove_file(&credential).unwrap();
    // The saved exact token version carries the read after both restarts.
    let value = success(cli.run(&pods));
    let page: Value = serde_json::from_str(value["result"].as_str().unwrap()).unwrap();
    assert!(page["items"].as_array().is_some_and(|i| !i.is_empty()));
    let current = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "cluster",
        "--connection",
        &reference,
    ]));
    assert_eq!(current["connection"]["summary"]["revision"], revision);

    // endpoints.discover reaches the same cluster on the same saved credential.
    let describe = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "cluster",
        "--operation",
        "endpoints.discover",
    ]));
    let listed = success(cli.run(&[
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
        r#"{"namespace":"fixture","limit":50}"#,
    ]));
    let page: Value = serde_json::from_str(listed["result"].as_str().unwrap()).unwrap();
    assert!(page["items"].is_array(), "{page}");
}
